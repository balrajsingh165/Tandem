/**
 * Use-case: place an outgoing call. Runs GuardEmergencyNumber, then delegates to
 * TelecomBridge.dial; returns a typed result the transport layer maps onto Ack
 * statuses.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.port.TelecomBridge
import com.tandem.gateway.domain.port.TelecomError
import javax.inject.Inject

class PlaceCall @Inject constructor(
    private val telecomBridge: TelecomBridge,
    private val guardEmergencyNumber: GuardEmergencyNumber,
) {
    /**
     * [fromDesktop] gates the emergency guard: handset-originated dials are the
     * sanctioned emergency path and pass through untouched (ADR-0008).
     */
    suspend operator fun invoke(
        number: String,
        simSlot: Int,
        fromDesktop: Boolean,
    ): Result<String> {
        if (fromDesktop) {
            when (val verdict = guardEmergencyNumber.guardDial(number)) {
                is GuardEmergencyNumber.Verdict.EmergencyBlocked ->
                    return Result.failure(EmergencyNumberBlocked(verdict.number))

                GuardEmergencyNumber.Verdict.EmergencyCallActive ->
                    return Result.failure(TelecomError.EmergencyCallActive)

                GuardEmergencyNumber.Verdict.Allowed -> Unit
            }
        }
        return telecomBridge.dial(number, simSlot)
    }
}

/**
 * Refusal of a desktop-originated emergency dial. Maps to
 * ERROR_CODE_EMERGENCY_NUMBER_BLOCKED at the transport edge.
 */
class EmergencyNumberBlocked(val number: String) :
    Exception("$number is an emergency number; it must be dialed on the handset")
