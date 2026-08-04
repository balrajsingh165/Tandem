/**
 * Pure mapping from android.telecom.Call state, details, and capabilities to the
 * domain Call model, including DisconnectCause translation. Stateless;
 * unit-tested exhaustively.
 */
package com.tandem.gateway.telecom

import android.telecom.Call as TelecomCall
import android.telecom.DisconnectCause as TelecomDisconnectCause
import com.tandem.gateway.domain.model.CallDirection
import com.tandem.gateway.domain.model.CallState
import com.tandem.gateway.domain.model.DisconnectCause

object CallStateMapper {

    fun mapState(telecomState: Int): CallState = when (telecomState) {
        TelecomCall.STATE_NEW, TelecomCall.STATE_CONNECTING -> CallState.CONNECTING
        TelecomCall.STATE_DIALING -> CallState.DIALING
        TelecomCall.STATE_RINGING -> CallState.RINGING
        TelecomCall.STATE_ACTIVE -> CallState.ACTIVE
        TelecomCall.STATE_HOLDING -> CallState.HOLDING
        TelecomCall.STATE_DISCONNECTING -> CallState.DISCONNECTING
        TelecomCall.STATE_DISCONNECTED -> CallState.DISCONNECTED
        // SELECT_PHONE_ACCOUNT and PULLING_CALL are pre-connection states the
        // desktop renders the same as connecting.
        else -> CallState.CONNECTING
    }

    fun mapDirection(callDirection: Int): CallDirection = when (callDirection) {
        TelecomCall.Details.DIRECTION_OUTGOING -> CallDirection.OUTGOING
        else -> CallDirection.INCOMING
    }

    fun mapDisconnectCause(cause: TelecomDisconnectCause?): DisconnectCause = when (cause?.code) {
        null -> DisconnectCause.UNSPECIFIED
        TelecomDisconnectCause.LOCAL -> DisconnectCause.LOCAL_HANGUP
        TelecomDisconnectCause.REMOTE -> DisconnectCause.REMOTE_HANGUP
        TelecomDisconnectCause.BUSY -> DisconnectCause.BUSY
        TelecomDisconnectCause.MISSED -> DisconnectCause.MISSED
        TelecomDisconnectCause.REJECTED -> DisconnectCause.REJECTED
        TelecomDisconnectCause.CANCELED -> DisconnectCause.CANCELED
        TelecomDisconnectCause.ERROR -> DisconnectCause.ERROR
        else -> DisconnectCause.UNSPECIFIED
    }

    fun canHold(capabilities: Int): Boolean =
        capabilities and TelecomCall.Details.CAPABILITY_HOLD != 0

    fun canMerge(capabilities: Int): Boolean =
        capabilities and TelecomCall.Details.CAPABILITY_MERGE_CONFERENCE != 0

    /**
     * Telecom marks emergency calls through the connection properties; this flag
     * drives the read-only policy in ADR-0008, so an unknown value must never be
     * treated as "not an emergency".
     */
    fun isEmergency(properties: Int): Boolean =
        properties and TelecomCall.Details.PROPERTY_EMERGENCY_CALLBACK_MODE != 0 ||
            properties and TelecomCall.Details.PROPERTY_NETWORK_IDENTIFIED_EMERGENCY_CALL != 0

    fun isConference(properties: Int): Boolean =
        properties and TelecomCall.Details.PROPERTY_CONFERENCE != 0
}
