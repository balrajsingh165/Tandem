/**
 * LanServer fake that connects in-process desktop sessions, letting protocol
 * round-trip tests run the real router/use-case path with no sockets or TLS.
 */
package com.tandem.gateway.testkit

import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.port.CallClaimArbiter
import com.tandem.gateway.domain.port.LanServer
import com.tandem.gateway.domain.port.ServerStatus
import com.tandem.gateway.domain.port.SessionInfo
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class InMemoryLanServer : LanServer, CallClaimArbiter {

    private val _status = MutableStateFlow(ServerStatus(false, 0, "Test Phone"))
    override val status: StateFlow<ServerStatus> = _status

    private val _connectedSessions = MutableStateFlow<List<SessionInfo>>(emptyList())
    override val connectedSessions: StateFlow<List<SessionInfo>> = _connectedSessions

    val broadcastSnapshots = mutableListOf<CallSnapshot>()
    val broadcastLogVersions = mutableListOf<Long>()
    val revocations = mutableListOf<Pair<String, String>>()

    private val claims = mutableMapOf<String, String>()

    fun connect(deviceId: String) {
        _connectedSessions.value = _connectedSessions.value + SessionInfo(
            deviceId = deviceId,
            displayName = "Desktop $deviceId",
            connectedAtMs = 1_700_000_000_000,
            btAdapterAddress = "",
        )
    }

    override suspend fun start(port: Int): Result<Unit> {
        _status.value = ServerStatus(listening = true, port = port, advertisedName = "Test Phone")
        return Result.success(Unit)
    }

    override suspend fun stop() {
        _status.value = _status.value.copy(listening = false)
    }

    override suspend fun broadcastSnapshot(snapshot: CallSnapshot) {
        broadcastSnapshots += snapshot
    }

    override suspend fun broadcastCallLogChanged(logVersion: Long) {
        broadcastLogVersions += logVersion
    }

    var audioDeviceBroadcasts: Int = 0
        private set

    override suspend fun broadcastAudioDevices() {
        audioDeviceBroadcasts += 1
    }

    override suspend fun revokeSession(deviceId: String, reason: String) {
        revocations += deviceId to reason
        _connectedSessions.value = _connectedSessions.value.filter { it.deviceId != deviceId }
    }

    /** Same first-wins rule as the real registry, so arbitration tests are real. */
    override suspend fun claimCall(callId: String, deviceId: String): Boolean {
        val existing = claims[callId]
        if (existing != null) return existing == deviceId
        claims[callId] = deviceId
        return true
    }

    override fun releaseClaim(callId: String) {
        claims.remove(callId)
    }
}
