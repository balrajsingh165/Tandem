/**
 * Use-case: validate a PairingRequest token, await user confirmation via
 * PairingManager, persist the accepted desktop through PairedDeviceRepository,
 * and produce the PairingDecision payload.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.model.PairedDesktop
import com.tandem.gateway.domain.port.PairedDeviceRepository
import com.tandem.gateway.domain.port.PairingManager
import com.tandem.gateway.domain.port.ScannedOffer
import javax.inject.Inject

class PairDesktop @Inject constructor(
    private val pairingManager: PairingManager,
    private val pairedDeviceRepository: PairedDeviceRepository,
) {
    suspend fun openWindow(ttlSeconds: Int = PairingManager.DEFAULT_TTL_SECONDS) =
        pairingManager.openWindow(ttlSeconds)

    suspend fun openScannedWindow(
        offer: ScannedOffer,
        ttlSeconds: Int = PairingManager.DEFAULT_TTL_SECONDS,
    ) = pairingManager.openScannedWindow(offer, ttlSeconds)

    suspend fun closeWindow() = pairingManager.closeWindow()

    /**
     * Persists only on acceptance; a rejected candidate leaves no trace beyond
     * the closed window.
     */
    suspend fun submitDecision(accept: Boolean): Result<PairedDesktop?> {
        val decision = pairingManager.submitDecision(accept)
        val desktop = decision.getOrNull()
        if (accept && desktop != null) {
            pairedDeviceRepository.upsert(desktop).onFailure { return Result.failure(it) }
        }
        return decision
    }
}
