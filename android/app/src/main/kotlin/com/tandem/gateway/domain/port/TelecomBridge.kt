/**
 * Port over Android Telecom: observe the authoritative call list as a Flow, and
 * execute answer/reject/end/hold/unhold/merge/mute/DTMF/dial commands.
 * Implemented by TelecomBridgeImpl; faked in tests. Contract in
 * docs/11-api-reference.md.
 */
package com.tandem.gateway.domain.port

import com.tandem.gateway.domain.model.Call
import com.tandem.gateway.domain.model.AudioRoute
import kotlinx.coroutines.flow.Flow

interface TelecomBridge {
    val calls: Flow<List<Call>>

    val audioRoute: Flow<AudioRoute>

    val microphoneMuted: Flow<Boolean>

    suspend fun dial(number: String, simSlot: Int): Result<String>

    suspend fun answer(callId: String): Result<Unit>

    suspend fun reject(callId: String): Result<Unit>

    suspend fun disconnect(callId: String): Result<Unit>

    suspend fun hold(callId: String): Result<Unit>

    suspend fun unhold(callId: String): Result<Unit>

    suspend fun merge(callId: String, otherCallId: String): Result<Unit>

    suspend fun setMuted(muted: Boolean): Result<Unit>

    suspend fun sendDtmf(callId: String, digits: String): Result<Unit>
}

/** Typed failures at the telecom boundary; never stringly-typed (docs/14). */
sealed class TelecomError(message: String) : Exception(message) {
    data object DialerRoleMissing : TelecomError("Tandem is not the default phone app")

    data object PermissionDenied : TelecomError("a required telephony permission was denied")

    data class CallNotFound(val callId: String) : TelecomError("no call with id $callId")

    data class InvalidCallState(val callId: String, val command: String) :
        TelecomError("call $callId cannot $command in its current state")

    data class PlacementFailed(val reason: String) : TelecomError("could not place call: $reason")

    data object EmergencyCallActive :
        TelecomError("remote control is refused while an emergency call is active")

    data class Internal(val reason: String) : TelecomError(reason)
}
