/**
 * Registry of live DesktopSessions: broadcast fan-out of call/log events,
 * revocation enforcement, and the atomic claim primitive AnswerCall uses for
 * first-answer-wins arbitration.
 */
package com.tandem.gateway.transport

import com.tandem.gateway.domain.port.SessionInfo
import java.util.concurrent.ConcurrentHashMap
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

@Singleton
class SessionRegistry @Inject constructor() {

    private val sessions = ConcurrentHashMap<String, DesktopSession>()
    private val claims = ConcurrentHashMap<String, String>()
    private val claimMutex = Mutex()

    private val _connected = MutableStateFlow<List<SessionInfo>>(emptyList())
    val connected: StateFlow<List<SessionInfo>> = _connected.asStateFlow()

    fun register(session: DesktopSession) {
        sessions[session.deviceId] = session
        publish()
    }

    fun unregister(deviceId: String) {
        sessions.remove(deviceId)
        publish()
    }

    fun session(deviceId: String): DesktopSession? = sessions[deviceId]

    suspend fun broadcast(frame: ByteArray) {
        sessions.values.forEach { runCatching { it.send(frame) } }
    }

    suspend fun close(deviceId: String, reason: String) {
        sessions.remove(deviceId)?.let { runCatching { it.closeWithRevocation(reason) } }
        publish()
    }

    /**
     * The arbitration point for multi-desktop answering: the first caller to
     * claim a call id wins, and every later caller is told it was already
     * handled. Holding the mutex makes the check-and-set atomic across sessions.
     */
    suspend fun claim(callId: String, deviceId: String): Boolean = claimMutex.withLock {
        val existing = claims[callId]
        if (existing != null) return@withLock existing == deviceId
        claims[callId] = deviceId
        true
    }

    fun releaseClaim(callId: String) {
        claims.remove(callId)
    }

    private fun publish() {
        _connected.value = sessions.values.map { it.info() }
    }
}
