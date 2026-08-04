/**
 * Domain model of a live call: Call plus the CallState, CallDirection, and
 * DisconnectCause enums, mirroring Android Telecom semantics without framework
 * types. Mapped to/from tandem.v1 protos in the transport layer only.
 */
package com.tandem.gateway.domain.model

data class Call(
    val callId: String,
    val state: CallState,
    val direction: CallDirection,
    val remoteNumber: String,
    val remoteDisplayName: String,
    val startedAtMs: Long,
    val isConference: Boolean,
    val canHold: Boolean,
    val canMerge: Boolean,
    val isEmergency: Boolean,
    val disconnectCause: DisconnectCause,
    val simSlot: Int,
) {
    val isTerminal: Boolean get() = state == CallState.DISCONNECTED

    val isRinging: Boolean get() = state == CallState.RINGING
}

enum class CallState {
    CONNECTING,
    DIALING,
    RINGING,
    ACTIVE,
    HOLDING,
    DISCONNECTING,
    DISCONNECTED,
}

enum class CallDirection {
    INCOMING,
    OUTGOING,
}

enum class DisconnectCause {
    UNSPECIFIED,
    LOCAL_HANGUP,
    REMOTE_HANGUP,
    BUSY,
    MISSED,
    REJECTED,
    CANCELED,
    ERROR,
}

/**
 * Authoritative call-plane state, versioned so desktops can detect gaps and
 * reconcile against the phone (ADR-0007).
 */
data class CallSnapshot(
    val epochId: String,
    val stateSeq: Long,
    val calls: List<Call>,
    val audioRoute: AudioRoute,
    val microphoneMuted: Boolean,
    val btRouteAddress: String,
) {
    fun call(callId: String): Call? = calls.firstOrNull { it.callId == callId }

    fun hasActiveEmergency(): Boolean = calls.any { it.isEmergency && !it.isTerminal }
}
