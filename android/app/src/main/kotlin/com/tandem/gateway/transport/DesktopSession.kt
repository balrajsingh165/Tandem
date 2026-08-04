/**
 * Per-connection session actor: performs SessionHello/SessionWelcome and Resume,
 * tracks (epoch_id, state_seq) delivery, applies per-session rate limits, and
 * serializes outbound events. One coroutine per session; no shared mutable state.
 */
package com.tandem.gateway.transport

import com.tandem.gateway.domain.port.SessionInfo
import com.tandem.gateway.proto.v1.Envelope
import io.ktor.websocket.Frame
import io.ktor.websocket.WebSocketSession
import io.ktor.websocket.send
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

class DesktopSession(
    val deviceId: String,
    private val displayName: String,
    private val btAdapterAddress: String,
    private val connectedAtMs: Long,
    private val socket: WebSocketSession,
    private val codec: EnvelopeCodec,
) {
    // One send at a time: interleaved writes would corrupt the frame stream.
    private val sendMutex = Mutex()
    private val dialTimestamps = ArrayDeque<Long>()

    fun info(): SessionInfo = SessionInfo(
        deviceId = deviceId,
        displayName = displayName,
        connectedAtMs = connectedAtMs,
        btAdapterAddress = btAdapterAddress,
    )

    suspend fun send(frame: ByteArray) = sendMutex.withLock {
        socket.send(Frame.Binary(fin = true, data = frame))
    }

    suspend fun send(envelope: Envelope) = send(codec.encode(envelope))

    /**
     * Toll-fraud mitigation from docs/08: a compromised desktop cannot turn a
     * paired phone into an auto-dialer.
     */
    fun allowDial(nowMs: Long): Boolean {
        val windowStart = nowMs - DIAL_WINDOW_MS
        while (dialTimestamps.isNotEmpty() && dialTimestamps.first() < windowStart) {
            dialTimestamps.removeFirst()
        }
        if (dialTimestamps.size >= MAX_DIALS_PER_WINDOW) return false
        dialTimestamps.addLast(nowMs)
        return true
    }

    suspend fun closeWithRevocation(reason: String) {
        runCatching {
            send(
                Envelope.newBuilder()
                    .setProtocolVersion(EnvelopeCodec.PROTOCOL_VERSION)
                    .setRevokedEvent(
                        com.tandem.gateway.proto.v1.RevokedEvent.newBuilder()
                            .setReason(reason)
                            .build(),
                    )
                    .build(),
            )
        }
        runCatching { socket.close() }
    }

    companion object {
        const val MAX_DIALS_PER_WINDOW: Int = 5
        const val DIAL_WINDOW_MS: Long = 60_000
    }
}
