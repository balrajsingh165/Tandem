/**
 * PairingManager implementation: opens a 120 s single-use pairing window,
 * validates tokens, drives the user confirmation sheet, and finalizes via
 * PairDesktop. Enforces one pairing candidate at a time.
 */
package com.tandem.gateway.pairing

import com.tandem.gateway.crypto.TlsServerFactory
import com.tandem.gateway.domain.model.PairedDesktop
import com.tandem.gateway.domain.port.IdentityStore
import com.tandem.gateway.domain.port.PairingError
import com.tandem.gateway.domain.port.PairingInvitation
import com.tandem.gateway.domain.port.PairingManager
import com.tandem.gateway.domain.port.PairingWindowState
import com.tandem.gateway.domain.port.SettingsRepository
import java.security.SecureRandom
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

@Singleton
class PairingManagerImpl @Inject constructor(
    private val identityStore: IdentityStore,
    private val settingsRepository: SettingsRepository,
    private val qrPayloadCodec: QrPayloadCodec,
    private val tlsServerFactory: TlsServerFactory,
) : PairingManager {

    private val _state = MutableStateFlow<PairingWindowState>(PairingWindowState.Closed)
    override val state: StateFlow<PairingWindowState> = _state.asStateFlow()

    private val mutex = Mutex()
    private var session: PairingSession? = null
    private var candidate: PairedDesktop? = null

    override suspend fun openWindow(ttlSeconds: Int): Result<PairingInvitation> = mutex.withLock {
        if (session != null) return Result.failure(PairingError.WindowBusy)

        val identity = identityStore.identity().getOrElse { return Result.failure(it) }
        val expiresAtMs = System.currentTimeMillis() + ttlSeconds * 1_000L

        val invitation = PairingInvitation(
            host = "",
            port = settingsRepository.listenPort.first(),
            fingerprint = identity.spkiSha256,
            token = generateToken(),
            phoneName = identity.displayName,
            expiresAtMs = expiresAtMs,
        )

        session = PairingSession(
            token = invitation.token,
            expiresAtMs = expiresAtMs,
            requireShortCode = false,
        )

        // Admits exactly one unknown peer into the provisional path while the
        // window is open (docs/07).
        tlsServerFactory.setPairingWindowOpen(true)
        _state.value = PairingWindowState.Open(invitation)
        Result.success(invitation)
    }

    override suspend fun closeWindow() = mutex.withLock {
        session?.expire()
        session = null
        candidate = null
        tlsServerFactory.setPairingWindowOpen(false)
        _state.value = PairingWindowState.Closed
    }

    override suspend fun submitDecision(accept: Boolean): Result<PairedDesktop?> = mutex.withLock {
        val active = session ?: return Result.failure(PairingError.WindowBusy)

        val result = if (accept) {
            active.accept().map { candidate }
        } else {
            active.reject().map { null }
        }

        session = null
        tlsServerFactory.setPairingWindowOpen(false)
        _state.value = result.fold(
            onSuccess = { desktop ->
                if (desktop != null) {
                    PairingWindowState.Completed(desktop)
                } else {
                    PairingWindowState.Closed
                }
            },
            onFailure = { cause ->
                PairingWindowState.Failed(cause as? PairingError ?: PairingError.RejectedByUser)
            },
        )
        candidate = null
        result
    }

    fun qrPayload(invitation: PairingInvitation): String = qrPayloadCodec.encode(invitation)

    private fun generateToken(): String {
        val bytes = ByteArray(TOKEN_BYTES)
        SecureRandom().nextBytes(bytes)
        return android.util.Base64.encodeToString(
            bytes,
            android.util.Base64.URL_SAFE or android.util.Base64.NO_PADDING or
                android.util.Base64.NO_WRAP,
        )
    }

    private companion object {
        const val TOKEN_BYTES = 16
    }
}
