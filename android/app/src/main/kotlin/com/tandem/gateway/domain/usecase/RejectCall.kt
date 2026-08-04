/**
 * Use-case: reject a ringing call via TelecomBridge. Idempotence: rejecting a
 * non-ringing call yields InvalidCallState, never a crash.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.port.TelecomBridge
import javax.inject.Inject

class RejectCall @Inject constructor(
    private val telecomBridge: TelecomBridge,
) {
    suspend operator fun invoke(callId: String): Result<Unit> = telecomBridge.reject(callId)
}
