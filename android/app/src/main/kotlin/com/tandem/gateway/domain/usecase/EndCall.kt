/**
 * Use-case: end an active, held, or dialing call via TelecomBridge.disconnect.
 * Emergency calls in progress are excluded (GuardEmergencyNumber policy; see
 * docs/08).
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.port.TelecomBridge
import com.tandem.gateway.domain.port.TelecomError
import javax.inject.Inject

class EndCall @Inject constructor(
    private val telecomBridge: TelecomBridge,
    private val guardEmergencyNumber: GuardEmergencyNumber,
) {
    suspend operator fun invoke(callId: String, snapshot: CallSnapshot?): Result<Unit> {
        if (snapshot?.call(callId)?.isEmergency == true) {
            return Result.failure(TelecomError.EmergencyCallActive)
        }
        if (!guardEmergencyNumber.guardRemoteControl(snapshot).isAllowed) {
            return Result.failure(TelecomError.EmergencyCallActive)
        }
        return telecomBridge.disconnect(callId)
    }
}
