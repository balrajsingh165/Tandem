/**
 * Use-case: unhold a call via TelecomBridge. Unholding an active call is an OK
 * no-op.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.model.CallState
import com.tandem.gateway.domain.port.TelecomBridge
import javax.inject.Inject

class UnholdCall @Inject constructor(
    private val telecomBridge: TelecomBridge,
) {
    suspend operator fun invoke(callId: String, snapshot: CallSnapshot?): Result<Unit> {
        if (snapshot?.call(callId)?.state == CallState.ACTIVE) return Result.success(Unit)
        return telecomBridge.unhold(callId)
    }
}
