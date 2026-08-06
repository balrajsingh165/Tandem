/**
 * Encodes/decodes tandem.v1 Envelope frames and maps between generated proto
 * types and domain models. The only Android file that imports generated proto
 * classes (ADR-0009).
 */
package com.tandem.gateway.transport

import com.tandem.gateway.domain.model.AudioRoute
import com.tandem.gateway.domain.model.ContactNumber
import com.tandem.gateway.domain.model.Call
import com.tandem.gateway.domain.model.CallDirection
import com.tandem.gateway.domain.model.CallLogEntry
import com.tandem.gateway.domain.model.CallLogType
import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.model.CallState
import com.tandem.gateway.domain.model.DisconnectCause
import com.tandem.gateway.proto.v1.AudioRoute as ProtoAudioRoute
import com.tandem.gateway.proto.v1.ContactEntry as ProtoContactEntry
import com.tandem.gateway.proto.v1.CallDirection as ProtoCallDirection
import com.tandem.gateway.proto.v1.CallInfo
import com.tandem.gateway.proto.v1.CallLogEntry as ProtoCallLogEntry
import com.tandem.gateway.proto.v1.CallLogType as ProtoCallLogType
import com.tandem.gateway.proto.v1.CallSnapshot as ProtoCallSnapshot
import com.tandem.gateway.proto.v1.CallState as ProtoCallState
import com.tandem.gateway.proto.v1.DisconnectCause as ProtoDisconnectCause
import com.tandem.gateway.proto.v1.Envelope
import com.tandem.gateway.proto.v1.ErrorCode
import com.tandem.gateway.proto.v1.Status
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class EnvelopeCodec @Inject constructor() {

    fun decode(frame: ByteArray): Envelope {
        require(frame.size <= MAX_ENVELOPE_BYTES) {
            "frame of ${frame.size} bytes exceeds $MAX_ENVELOPE_BYTES"
        }
        return Envelope.parseFrom(frame)
    }

    fun encode(envelope: Envelope): ByteArray {
        val bytes = envelope.toByteArray()
        require(bytes.size <= MAX_ENVELOPE_BYTES) {
            "frame of ${bytes.size} bytes exceeds $MAX_ENVELOPE_BYTES"
        }
        return bytes
    }

    fun status(code: ErrorCode, message: String = ""): Status =
        Status.newBuilder().setCode(code).setMessage(message).build()

    fun toProto(snapshot: CallSnapshot): ProtoCallSnapshot =
        ProtoCallSnapshot.newBuilder()
            .setEpochId(snapshot.epochId)
            .setStateSeq(snapshot.stateSeq)
            .addAllCalls(snapshot.calls.map(::toProto))
            .setAudioRoute(toProto(snapshot.audioRoute))
            .setMicrophoneMuted(snapshot.microphoneMuted)
            .setBtRouteAddress(snapshot.btRouteAddress)
            .build()

    fun toProto(call: Call): CallInfo =
        CallInfo.newBuilder()
            .setCallId(call.callId)
            .setState(toProto(call.state))
            .setDirection(toProto(call.direction))
            .setRemoteNumber(call.remoteNumber)
            .setRemoteDisplayName(call.remoteDisplayName)
            .setStartedAtMs(call.startedAtMs)
            .setIsConference(call.isConference)
            .setCanHold(call.canHold)
            .setCanMerge(call.canMerge)
            .setIsEmergency(call.isEmergency)
            .setDisconnectCause(toProto(call.disconnectCause))
            .setSimSlot(call.simSlot)
            .build()

    fun toProto(contact: ContactNumber): ProtoContactEntry =
        ProtoContactEntry.newBuilder()
            .setContactId(contact.contactId)
            .setDisplayName(contact.displayName)
            .setNumber(contact.number)
            .setLabel(contact.label)
            .setStarred(contact.starred)
            .build()

    fun toProto(entry: CallLogEntry): ProtoCallLogEntry =
        ProtoCallLogEntry.newBuilder()
            .setEntryId(entry.entryId)
            .setNumber(entry.number)
            .setDisplayName(entry.displayName)
            .setType(toProto(entry.type))
            .setStartedAtMs(entry.startedAtMs)
            .setDurationSeconds(entry.durationSeconds)
            .setSimSlot(entry.simSlot)
            .build()

    private fun toProto(state: CallState): ProtoCallState = when (state) {
        CallState.CONNECTING -> ProtoCallState.CALL_STATE_CONNECTING
        CallState.DIALING -> ProtoCallState.CALL_STATE_DIALING
        CallState.RINGING -> ProtoCallState.CALL_STATE_RINGING
        CallState.ACTIVE -> ProtoCallState.CALL_STATE_ACTIVE
        CallState.HOLDING -> ProtoCallState.CALL_STATE_HOLDING
        CallState.DISCONNECTING -> ProtoCallState.CALL_STATE_DISCONNECTING
        CallState.DISCONNECTED -> ProtoCallState.CALL_STATE_DISCONNECTED
    }

    private fun toProto(direction: CallDirection): ProtoCallDirection = when (direction) {
        CallDirection.INCOMING -> ProtoCallDirection.CALL_DIRECTION_INCOMING
        CallDirection.OUTGOING -> ProtoCallDirection.CALL_DIRECTION_OUTGOING
    }

    fun toProto(route: AudioRoute): ProtoAudioRoute = when (route) {
        AudioRoute.EARPIECE -> ProtoAudioRoute.AUDIO_ROUTE_EARPIECE
        AudioRoute.SPEAKER -> ProtoAudioRoute.AUDIO_ROUTE_SPEAKER
        AudioRoute.WIRED_HEADSET -> ProtoAudioRoute.AUDIO_ROUTE_WIRED_HEADSET
        AudioRoute.BLUETOOTH -> ProtoAudioRoute.AUDIO_ROUTE_BLUETOOTH
    }

    fun fromProto(route: ProtoAudioRoute): AudioRoute = when (route) {
        ProtoAudioRoute.AUDIO_ROUTE_SPEAKER -> AudioRoute.SPEAKER
        ProtoAudioRoute.AUDIO_ROUTE_WIRED_HEADSET -> AudioRoute.WIRED_HEADSET
        ProtoAudioRoute.AUDIO_ROUTE_BLUETOOTH -> AudioRoute.BLUETOOTH
        else -> AudioRoute.EARPIECE
    }

    private fun toProto(cause: DisconnectCause): ProtoDisconnectCause = when (cause) {
        DisconnectCause.UNSPECIFIED -> ProtoDisconnectCause.DISCONNECT_CAUSE_UNSPECIFIED
        DisconnectCause.LOCAL_HANGUP -> ProtoDisconnectCause.DISCONNECT_CAUSE_LOCAL_HANGUP
        DisconnectCause.REMOTE_HANGUP -> ProtoDisconnectCause.DISCONNECT_CAUSE_REMOTE_HANGUP
        DisconnectCause.BUSY -> ProtoDisconnectCause.DISCONNECT_CAUSE_BUSY
        DisconnectCause.MISSED -> ProtoDisconnectCause.DISCONNECT_CAUSE_MISSED
        DisconnectCause.REJECTED -> ProtoDisconnectCause.DISCONNECT_CAUSE_REJECTED
        DisconnectCause.CANCELED -> ProtoDisconnectCause.DISCONNECT_CAUSE_CANCELED
        DisconnectCause.ERROR -> ProtoDisconnectCause.DISCONNECT_CAUSE_ERROR
    }

    private fun toProto(type: CallLogType): ProtoCallLogType = when (type) {
        CallLogType.INCOMING -> ProtoCallLogType.CALL_LOG_TYPE_INCOMING
        CallLogType.OUTGOING -> ProtoCallLogType.CALL_LOG_TYPE_OUTGOING
        CallLogType.MISSED -> ProtoCallLogType.CALL_LOG_TYPE_MISSED
        CallLogType.REJECTED -> ProtoCallLogType.CALL_LOG_TYPE_REJECTED
    }

    companion object {
        const val MAX_ENVELOPE_BYTES: Int = 256 * 1024
        const val PROTOCOL_VERSION: Int = 1
    }
}
