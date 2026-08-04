/**
 * Use-case: hold a call via TelecomBridge, honoring Call.can_hold capability.
 * Holding an already-held call is an OK no-op.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.model.CallState
import com.tandem.gateway.domain.port.TelecomBridge
import com.tandem.gateway.domain.port.TelecomError
import javax.inject.Inject

class HoldCall @Inject constructor(
    private val telecomBridge: TelecomBridge,
) {
    suspend operator fun invoke(callId: String, snapshot: CallSnapshot?): Result<Unit> {
        val call = snapshot?.call(callId)
        if (call?.state == CallState.HOLDING) return Result.success(Unit)
        if (call != null && !call.canHold) {
            return Result.failure(TelecomError.InvalidCallState(callId, "hold"))
        }
        return telecomBridge.hold(callId)
    }
}
