/**
 * Routes decoded control-plane requests to their use-cases and maps results onto
 * Ack/typed responses. Pure dispatch: authentication happened at TLS accept,
 * policy lives in use-cases.
 */
package com.tandem.gateway.transport

import com.tandem.gateway.domain.model.AudioRouteTarget
import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.port.MediaRouteError
import com.tandem.gateway.domain.port.PairedDeviceRepository
import com.tandem.gateway.domain.port.TelecomError
import com.tandem.gateway.domain.usecase.AnswerCall
import com.tandem.gateway.domain.usecase.CallAlreadyHandled
import com.tandem.gateway.domain.usecase.EmergencyNumberBlocked
import com.tandem.gateway.domain.usecase.EndCall
import com.tandem.gateway.domain.usecase.HoldCall
import com.tandem.gateway.domain.usecase.MergeCalls
import com.tandem.gateway.domain.usecase.PlaceCall
import com.tandem.gateway.domain.usecase.RejectCall
import com.tandem.gateway.domain.usecase.RequestAudioRoute
import com.tandem.gateway.domain.usecase.SendDtmf
import com.tandem.gateway.domain.usecase.SetMute
import com.tandem.gateway.domain.usecase.SyncCallLog
import com.tandem.gateway.domain.usecase.UnholdCall
import com.tandem.gateway.proto.v1.Ack
import com.tandem.gateway.proto.v1.CallLogSyncResponse
import com.tandem.gateway.proto.v1.Envelope
import com.tandem.gateway.proto.v1.ErrorCode
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ControlPlaneRouter @Inject constructor(
    private val placeCall: PlaceCall,
    private val answerCall: AnswerCall,
    private val rejectCall: RejectCall,
    private val endCall: EndCall,
    private val setMute: SetMute,
    private val holdCall: HoldCall,
    private val unholdCall: UnholdCall,
    private val mergeCalls: MergeCalls,
    private val sendDtmf: SendDtmf,
    private val requestAudioRoute: RequestAudioRoute,
    private val syncCallLog: SyncCallLog,
    private val pairedDeviceRepository: PairedDeviceRepository,
    private val codec: EnvelopeCodec,
) {
    suspend fun handle(
        envelope: Envelope,
        session: DesktopSession,
        snapshot: CallSnapshot?,
        nowMs: Long,
    ): Envelope {
        val result: Result<Envelope> = when (envelope.payloadCase) {
            Envelope.PayloadCase.DIAL_REQUEST -> {
                if (!session.allowDial(nowMs)) {
                    return ack(envelope, ErrorCode.ERROR_CODE_RATE_LIMITED)
                }
                val request = envelope.dialRequest
                placeCall(request.number, request.simSlot, fromDesktop = true)
                    .map { ackEnvelope(envelope, ErrorCode.ERROR_CODE_OK) }
            }

            Envelope.PayloadCase.ANSWER_REQUEST ->
                answerCall(envelope.answerRequest.callId, session.deviceId)
                    .map { ackEnvelope(envelope, ErrorCode.ERROR_CODE_OK) }

            Envelope.PayloadCase.REJECT_REQUEST ->
                rejectCall(envelope.rejectRequest.callId)
                    .map { ackEnvelope(envelope, ErrorCode.ERROR_CODE_OK) }

            Envelope.PayloadCase.END_REQUEST ->
                endCall(envelope.endRequest.callId, snapshot)
                    .map { ackEnvelope(envelope, ErrorCode.ERROR_CODE_OK) }

            Envelope.PayloadCase.MUTE_REQUEST ->
                setMute(envelope.muteRequest.muted)
                    .map { ackEnvelope(envelope, ErrorCode.ERROR_CODE_OK) }

            Envelope.PayloadCase.HOLD_REQUEST ->
                holdCall(envelope.holdRequest.callId, snapshot)
                    .map { ackEnvelope(envelope, ErrorCode.ERROR_CODE_OK) }

            Envelope.PayloadCase.UNHOLD_REQUEST ->
                unholdCall(envelope.unholdRequest.callId, snapshot)
                    .map { ackEnvelope(envelope, ErrorCode.ERROR_CODE_OK) }

            Envelope.PayloadCase.MERGE_REQUEST ->
                mergeCalls(
                    envelope.mergeRequest.callId,
                    envelope.mergeRequest.otherCallId,
                    snapshot,
                ).map { ackEnvelope(envelope, ErrorCode.ERROR_CODE_OK) }

            Envelope.PayloadCase.SEND_DTMF_REQUEST ->
                sendDtmf(
                    envelope.sendDtmfRequest.callId,
                    envelope.sendDtmfRequest.digits,
                    snapshot,
                ).map { ackEnvelope(envelope, ErrorCode.ERROR_CODE_OK) }

            Envelope.PayloadCase.AUDIO_ROUTE_REQUEST ->
                requestAudioRoute(
                    AudioRouteTarget(
                        route = codec.fromProto(envelope.audioRouteRequest.route),
                        btDeviceAddress = envelope.audioRouteRequest.btDeviceAddress,
                    ),
                    snapshot,
                ).map { ackEnvelope(envelope, ErrorCode.ERROR_CODE_OK) }

            Envelope.PayloadCase.CALL_LOG_SYNC_REQUEST ->
                syncCallLog(
                    envelope.callLogSyncRequest.sinceMs,
                    envelope.callLogSyncRequest.maxEntries,
                ).map { page ->
                    Envelope.newBuilder()
                        .setProtocolVersion(EnvelopeCodec.PROTOCOL_VERSION)
                        .setInReplyTo(envelope.messageId)
                        .setCallLogSyncResponse(
                            CallLogSyncResponse.newBuilder()
                                .setStatus(codec.status(ErrorCode.ERROR_CODE_OK))
                                .addAllEntries(page.entries.map(codec::toProto))
                                .setLogVersion(page.logVersion)
                                .setHasMore(page.hasMore)
                                .build(),
                        )
                        .build()
                }

            // The desktop forgot this phone, so the phone forgets it back: leaving
            // the pin in place would keep admitting a computer the user removed.
            // Revoked before the reply, so a reconnect during teardown still fails
            // the pinned-key lookup.
            Envelope.PayloadCase.UNPAIR_REQUEST ->
                pairedDeviceRepository.revoke(session.deviceId)
                    .map { ackEnvelope(envelope, ErrorCode.ERROR_CODE_OK) }

            else -> return ack(envelope, ErrorCode.ERROR_CODE_INTERNAL)
        }

        return result.getOrElse { cause -> ack(envelope, toErrorCode(cause), cause.message.orEmpty()) }
    }

    /**
     * The single place domain failures become wire codes. The emergency refusal
     * must map to its own code so every desktop shows the handset guidance
     * (ADR-0008).
     */
    private fun toErrorCode(cause: Throwable): ErrorCode = when (cause) {
        is EmergencyNumberBlocked -> ErrorCode.ERROR_CODE_EMERGENCY_NUMBER_BLOCKED
        is CallAlreadyHandled -> ErrorCode.ERROR_CODE_ALREADY_HANDLED
        is TelecomError.CallNotFound -> ErrorCode.ERROR_CODE_CALL_NOT_FOUND
        is TelecomError.InvalidCallState, TelecomError.EmergencyCallActive ->
            ErrorCode.ERROR_CODE_INVALID_CALL_STATE
        is TelecomError.DialerRoleMissing, TelecomError.PermissionDenied,
        is TelecomError.PlacementFailed -> ErrorCode.ERROR_CODE_TELECOM_FAILURE
        is MediaRouteError -> ErrorCode.ERROR_CODE_AUDIO_ROUTE_UNAVAILABLE
        else -> ErrorCode.ERROR_CODE_INTERNAL
    }

    private fun ack(request: Envelope, code: ErrorCode, message: String = ""): Envelope =
        ackEnvelope(request, code, message)

    private fun ackEnvelope(request: Envelope, code: ErrorCode, message: String = ""): Envelope =
        Envelope.newBuilder()
            .setProtocolVersion(EnvelopeCodec.PROTOCOL_VERSION)
            .setInReplyTo(request.messageId)
            .setAck(Ack.newBuilder().setStatus(codec.status(code, message)).build())
            .build()
}
