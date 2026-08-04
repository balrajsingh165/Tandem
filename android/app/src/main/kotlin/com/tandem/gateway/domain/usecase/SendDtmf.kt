/**
 * Use-case: send a DTMF digit sequence into an active call via TelecomBridge,
 * which plays digits sequentially with standard Telecom tone timing.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.model.CallState
import com.tandem.gateway.domain.port.TelecomBridge
import com.tandem.gateway.domain.port.TelecomError
import javax.inject.Inject

class SendDtmf @Inject constructor(
    private val telecomBridge: TelecomBridge,
) {
    suspend operator fun invoke(
        callId: String,
        digits: String,
        snapshot: CallSnapshot?,
    ): Result<Unit> {
        if (digits.isEmpty() || digits.any { it !in VALID_DIGITS }) {
            return Result.failure(TelecomError.InvalidCallState(callId, "dtmf"))
        }
        if (snapshot != null && snapshot.call(callId)?.state != CallState.ACTIVE) {
            return Result.failure(TelecomError.InvalidCallState(callId, "dtmf"))
        }
        return telecomBridge.sendDtmf(callId, digits)
    }

    private companion object {
        const val VALID_DIGITS = "0123456789*#ABCD"
    }
}
