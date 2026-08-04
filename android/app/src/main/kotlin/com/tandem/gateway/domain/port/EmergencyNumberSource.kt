/**
 * Port answering "is this an emergency number right now" from current SIM/region
 * data, and exposing the current emergency-number list for sync to desktops.
 * Consulted by GuardEmergencyNumber before every dial (ADR-0008).
 */
package com.tandem.gateway.domain.port

interface EmergencyNumberSource {
    /**
     * Authoritative check against the current SIM and region. Implementations
     * must fail closed: if telephony cannot answer, treat a number matching the
     * conservative fallback list as an emergency number rather than allowing it.
     */
    suspend fun isEmergencyNumber(dialString: String): Boolean

    /** The list synced to desktops so they can pre-check locally. */
    suspend fun currentEmergencyNumbers(): List<String>

    companion object {
        /** Used only when telephony is unavailable; never narrows the real list. */
        val CONSERVATIVE_FALLBACK: List<String> =
            listOf("112", "911", "999", "000", "110", "118", "119")
    }
}
