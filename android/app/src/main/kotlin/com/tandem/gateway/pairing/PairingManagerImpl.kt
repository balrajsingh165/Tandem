/**
 * PairingManager implementation: opens a 120 s single-use pairing window,
 * validates tokens, drives the user confirmation sheet, and finalizes via
 * PairDesktop. Enforces one pairing candidate at a time.
 */
package com.tandem.gateway.pairing

import com.tandem.gateway.crypto.TlsServerFactory
import com.tandem.gateway.domain.model.DesktopPlatform
import com.tandem.gateway.domain.model.PairedDesktop
import com.tandem.gateway.domain.port.IdentityStore
import com.tandem.gateway.domain.port.PairingError
import com.tandem.gateway.domain.port.PairingInvitation
import com.tandem.gateway.domain.port.PairingManager
import com.tandem.gateway.domain.port.PairingWindowState
import com.tandem.gateway.domain.port.ScannedOffer
import com.tandem.gateway.domain.port.SettingsRepository
import java.security.SecureRandom
import java.util.UUID
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

/** A desktop that presented a valid token and is awaiting the user's verdict. */
data class PairingCandidate(
    val desktopName: String,
    val desktopPlatform: String,
    val certDer: ByteArray,
    val spkiSha256: String,
    val protocolMin: Int,
    val protocolMax: Int,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is PairingCandidate) return false
        return spkiSha256 == other.spkiSha256 && certDer.contentEquals(other.certDer)
    }

    override fun hashCode(): Int = 31 * spkiSha256.hashCode() + certDer.contentHashCode()
}

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
    private var candidate: PairingCandidate? = null
    private var verdict: CompletableDeferred<PairedDesktop?>? = null

    /** Set only for a scanned offer: the one key allowed to claim the token. */
    private var expectedFingerprint: String? = null

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.Default)
    private var expiryJob: Job? = null

    override suspend fun openWindow(ttlSeconds: Int): Result<PairingInvitation> = mutex.withLock {
        if (session != null) return Result.failure(PairingError.WindowBusy)

        val identity = identityStore.identity().getOrElse { return Result.failure(it) }
        val expiresAtMs = System.currentTimeMillis() + ttlSeconds * 1_000L

        val invitation = PairingInvitation(
            // The desktop dials this address, so an empty host would make the
            // payload unusable no matter how the user transfers it.
            host = LocalAddress.current().orEmpty(),
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

    override suspend fun openScannedWindow(
        offer: ScannedOffer,
        ttlSeconds: Int,
    ): Result<Unit> = mutex.withLock {
        // Only a desktop actually waiting on the user's verdict may hold the
        // window; anything else is a leftover the new scan should replace, or a
        // second scan could never succeed.
        if (candidate != null) return Result.failure(PairingError.WindowBusy)
        if (session != null) teardown(null)

        val expiresAtMs = System.currentTimeMillis() + ttlSeconds * 1_000L
        session = PairingSession(
            token = offer.token,
            expiresAtMs = expiresAtMs,
            requireShortCode = false,
        )
        expectedFingerprint = offer.fingerprint

        tlsServerFactory.setPairingWindowOpen(true)
        _state.value = PairingWindowState.AwaitingDesktop(offer.desktopName)
        armExpiry(expiresAtMs)
        Result.success(Unit)
    }

    /**
     * Closes an unclaimed window once its token dies. Without this the screen
     * would wait on a desktop that can no longer be admitted, with no way back
     * to the scanner.
     */
    private fun armExpiry(expiresAtMs: Long) {
        expiryJob?.cancel()
        expiryJob = scope.launch {
            delay((expiresAtMs - System.currentTimeMillis()).coerceAtLeast(0L))
            mutex.withLock {
                if (candidate == null && session != null) teardown(PairingError.TokenExpired)
            }
        }
    }

    override suspend fun closeWindow() = mutex.withLock { teardown(null) }

    /**
     * Called from the LAN accept path when a provisional peer presents a token.
     * Consumes the token whether or not it matched, so a leaked code cannot be
     * retried.
     */
    suspend fun presentCandidate(
        token: String,
        presented: PairingCandidate,
        shortCode: String?,
    ): Result<Unit> = mutex.withLock {
        val active = session ?: return Result.failure(PairingError.WindowBusy)
        if (candidate != null) return Result.failure(PairingError.WindowBusy)

        // A scanned offer names the exact key the user pointed the camera at, so
        // any other key claiming the token is a different machine (docs/07).
        expectedFingerprint?.let { expected ->
            if (presented.spkiSha256 != expected) {
                teardown(PairingError.FingerprintMismatch)
                return Result.failure(PairingError.FingerprintMismatch)
            }
        }

        active.presentToken(token, System.currentTimeMillis())
            .onFailure { cause ->
                teardown(cause as? PairingError ?: PairingError.TokenMismatch)
                return Result.failure(cause)
            }

        candidate = presented
        verdict = CompletableDeferred()
        _state.value = PairingWindowState.AwaitingConfirmation(
            desktopName = presented.desktopName,
            desktopPlatform = presented.desktopPlatform,
            fingerprint = presented.spkiSha256,
            shortCode = shortCode,
        )
        Result.success(Unit)
    }

    /** Suspends until the user taps allow or deny, or the window is torn down. */
    suspend fun awaitVerdict(): Result<PairedDesktop> {
        val pending = mutex.withLock { verdict } ?: return Result.failure(PairingError.WindowBusy)
        val accepted = pending.await() ?: return Result.failure(PairingError.RejectedByUser)
        return Result.success(accepted)
    }

    override suspend fun submitDecision(accept: Boolean): Result<PairedDesktop?> = mutex.withLock {
        val active = session ?: return Result.failure(PairingError.WindowBusy)
        val presented = candidate ?: return Result.failure(PairingError.WindowBusy)

        if (!accept) {
            active.reject()
            verdict?.complete(null)
            teardown(PairingError.RejectedByUser)
            return Result.success(null)
        }

        active.accept().onFailure { cause ->
            verdict?.complete(null)
            teardown(cause as? PairingError ?: PairingError.RejectedByUser)
            return Result.failure(cause)
        }

        val now = System.currentTimeMillis()
        val desktop = PairedDesktop(
            deviceId = UUID.randomUUID().toString(),
            name = presented.desktopName,
            platform = DesktopPlatform.fromWire(presented.desktopPlatform),
            spkiSha256 = presented.spkiSha256,
            certDer = presented.certDer,
            btMacAddress = null,
            createdAtMs = now,
            lastSeenAtMs = now,
            revoked = false,
        )

        verdict?.complete(desktop)
        session = null
        candidate = null
        verdict = null
        expectedFingerprint = null
        tlsServerFactory.setPairingWindowOpen(false)
        _state.value = PairingWindowState.Completed(desktop)
        Result.success(desktop)
    }

    fun qrPayload(invitation: PairingInvitation): String = qrPayloadCodec.encode(invitation)

    /** Closes the window and releases any waiter, so no caller hangs forever. */
    private fun teardown(error: PairingError?) {
        expiryJob?.cancel()
        expiryJob = null
        session?.expire()
        session = null
        candidate = null
        verdict?.complete(null)
        verdict = null
        expectedFingerprint = null
        tlsServerFactory.setPairingWindowOpen(false)
        _state.value = error?.let { PairingWindowState.Failed(it) } ?: PairingWindowState.Closed
    }

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
