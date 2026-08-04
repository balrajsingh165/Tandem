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

sealed interface PairingWindowState {
    data object Closed : PairingWindowState

    data class Open(val invitation: PairingInvitation) : PairingWindowState

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

    data object CertificateBindingFailed :
        PairingError("the presented certificate did not match the TLS session")

    data object RejectedByUser : PairingError("pairing was declined on the phone")

    data object WindowBusy : PairingError("another computer is already pairing")

    data class VersionNegotiationFailed(val min: Int, val max: Int) :
        PairingError("no mutually supported protocol version in $min..$max")
}
