/**
 * Port over the LAN control server: start/stop listening, observe inbound
 * authenticated requests, and fan events out to connected desktop sessions.
 * Implemented by LanServerImpl; faked by InMemoryLanServer in tests.
 */
package com.tandem.gateway.domain.port

import com.tandem.gateway.domain.model.CallSnapshot
import kotlinx.coroutines.flow.Flow

interface LanServer {
    val status: Flow<ServerStatus>

    val connectedSessions: Flow<List<SessionInfo>>

    suspend fun start(port: Int): Result<Unit>

    suspend fun stop()

    /** Broadcasts a versioned snapshot to every live session. */
    suspend fun broadcastSnapshot(snapshot: CallSnapshot)

    /** Nudges desktops that the OS call log changed. */
    suspend fun broadcastCallLogChanged(logVersion: Long)

    /** Ends a desktop's session immediately after telling it why (docs/07). */
    suspend fun revokeSession(deviceId: String, reason: String)

    /**
     * Atomically claims a ringing call for one desktop. The first claim wins;
     * later ones get false and must render the resulting state, not an error.
     */
    suspend fun claimCall(callId: String, deviceId: String): Boolean
}

data class ServerStatus(
    val listening: Boolean,
    val port: Int,
    val advertisedName: String,
)

data class SessionInfo(
    val deviceId: String,
    val displayName: String,
    val connectedAtMs: Long,
    val btAdapterAddress: String,
)

/** Typed failures at the LAN boundary. */
sealed class TransportError(message: String) : Exception(message) {
    data class BindFailed(val port: Int) : TransportError("could not bind port $port")

    data object NotAuthenticated : TransportError("session is not authenticated")

    data class ProtocolViolation(val detail: String) : TransportError(detail)

    data class FrameTooLarge(val size: Int) : TransportError("frame of $size bytes is too large")

    data class VersionUnsupported(val requested: Int) :
        TransportError("protocol version $requested is not supported")

    data object RateLimited : TransportError("too many requests from this session")
}
