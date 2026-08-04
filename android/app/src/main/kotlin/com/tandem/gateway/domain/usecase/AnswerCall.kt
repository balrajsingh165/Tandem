/**
 * Use-case: answer a ringing call. Atomically arbitrates first-answer-wins across
 * desktop sessions against current telecom state, then delegates to
 * TelecomBridge.answer.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.port.LanServer
import com.tandem.gateway.domain.port.TelecomBridge
import javax.inject.Inject

class AnswerCall @Inject constructor(
    private val telecomBridge: TelecomBridge,
    private val lanServer: LanServer,
) {
    /**
     * Losing the race is not an error state: the call is being answered, just by
     * someone else, so callers report AlreadyHandled and follow the event stream.
     */
    suspend operator fun invoke(callId: String, requestingDeviceId: String): Result<Unit> {
        val claimed = lanServer.claimCall(callId, requestingDeviceId)
        if (!claimed) return Result.failure(CallAlreadyHandled(callId))
        return telecomBridge.answer(callId)
    }
}

/** Another desktop won the answer race; maps to ERROR_CODE_ALREADY_HANDLED. */
class CallAlreadyHandled(val callId: String) :
    Exception("call $callId was already handled by another device")
