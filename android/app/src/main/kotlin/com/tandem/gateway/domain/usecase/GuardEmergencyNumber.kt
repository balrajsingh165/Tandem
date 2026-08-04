/**
 * Use-case: classify a dial string via EmergencyNumberSource and refuse
 * desktop-originated emergency calls with EmergencyNumberBlocked (ADR-0008).
 * Also flags active emergency calls so remote control and audio-route requests
 * are refused while one is live.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.port.EmergencyNumberSource
import javax.inject.Inject

class GuardEmergencyNumber @Inject constructor(
    private val emergencyNumbers: EmergencyNumberSource,
) {
    /**
     * Called for every desktop-originated dial. A refusal here never reaches
     * Telecom, so no emergency call can originate from a computer.
     */
    suspend fun guardDial(dialString: String): Verdict {
        val normalized = normalize(dialString)
        if (normalized.isEmpty()) return Verdict.Allowed
        return if (emergencyNumbers.isEmergencyNumber(normalized)) {
            Verdict.EmergencyBlocked(normalized)
        } else {
            Verdict.Allowed
        }
    }

    /**
     * Remote control of any kind is refused while an emergency call is live; the
     * call is surfaced read-only and belongs to the handset.
     */
    fun guardRemoteControl(snapshot: CallSnapshot?): Verdict =
        if (snapshot?.hasActiveEmergency() == true) {
            Verdict.EmergencyCallActive
        } else {
            Verdict.Allowed
        }

    private fun normalize(dialString: String): String =
        dialString.filter { it.isDigit() || it == '*' || it == '#' }

    sealed interface Verdict {
        data object Allowed : Verdict

        data class EmergencyBlocked(val number: String) : Verdict

        data object EmergencyCallActive : Verdict

        val isAllowed: Boolean get() = this is Allowed
    }
}
