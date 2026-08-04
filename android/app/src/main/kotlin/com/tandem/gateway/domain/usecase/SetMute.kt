/**
 * Use-case: set the phone microphone mute state via TelecomBridge. Idempotent by
 * design: callers send the absolute target state, not a toggle.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.port.TelecomBridge
import javax.inject.Inject

class SetMute @Inject constructor(
    private val telecomBridge: TelecomBridge,
) {
    suspend operator fun invoke(muted: Boolean): Result<Unit> = telecomBridge.setMuted(muted)
}
