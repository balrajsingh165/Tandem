/**
 * Port over the pairing lifecycle: open/close a pairing window, expose the QR
 * payload, surface confirmation prompts, and finalize or reject a pairing
 * candidate. Implemented by PairingManagerImpl.
 */
package com.tandem.gateway.domain.port

import com.tandem.gateway.domain.model.PairedDesktop
import kotlinx.coroutines.flow.Flow

interface PairingManager {
    val state: Flow<PairingWindowState>

    /** Opens a single-use window; the token expires after [ttlSeconds]. */
    suspend fun openWindow(ttlSeconds: Int = DEFAULT_TTL_SECONDS): Result<PairingInvitation>

    /**
     * Opens a window for an offer scanned off a desktop's screen. The token comes
     * from the desktop, and only the key fingerprint printed in the code may
     * claim it — a desktop that answers with any other key is refused.
     */
    suspend fun openScannedWindow(
        offer: ScannedOffer,
        ttlSeconds: Int = DEFAULT_TTL_SECONDS,
    ): Result<Unit>

    suspend fun closeWindow()

    /** The user's verdict on the waiting candidate. */
    suspend fun submitDecision(accept: Boolean): Result<PairedDesktop?>

    companion object {
        const val DEFAULT_TTL_SECONDS: Int = 120
    }
}

/** What the phone displays for the desktop to scan or transcribe. */
data class PairingInvitation(
    val host: String,
    val port: Int,
    val fingerprint: String,
    val token: String,
    val phoneName: String,
    val expiresAtMs: Long,
)

/** What this phone read out of a desktop's on-screen pairing code. */
data class ScannedOffer(
    val fingerprint: String,
    val token: String,
    val desktopName: String,
)

sealed interface PairingWindowState {
    data object Closed : PairingWindowState

    data class Open(val invitation: PairingInvitation) : PairingWindowState

    /** Scanned a desktop's code; waiting for that desktop to connect. */
    data class AwaitingDesktop(val desktopName: String) : PairingWindowState

    /** A desktop presented a valid token; the user must now confirm. */
    data class AwaitingConfirmation(
        val desktopName: String,
        val desktopPlatform: String,
        val fingerprint: String,
        val shortCode: String?,
    ) : PairingWindowState

    data class Completed(val desktop: PairedDesktop) : PairingWindowState

    data class Failed(val error: PairingError) : PairingWindowState
}

/** Typed failures at the pairing boundary. */
sealed class PairingError(message: String) : Exception(message) {
    data object TokenExpired : PairingError("the pairing code has expired")

    data object TokenAlreadyUsed : PairingError("the pairing code has already been used")

    data object TokenMismatch : PairingError("the pairing code did not match")

    data object FingerprintMismatch :
        PairingError("that computer's key does not match the code you scanned")

    data object InvalidOffer : PairingError("that is not a Tandem pairing code")

    data object CertificateBindingFailed :
        PairingError("the presented certificate did not match the TLS session")

    data object RejectedByUser : PairingError("pairing was declined on the phone")

    data object WindowBusy : PairingError("another computer is already pairing")

    data class VersionNegotiationFailed(val min: Int, val max: Int) :
        PairingError("no mutually supported protocol version in $min..$max")
}
