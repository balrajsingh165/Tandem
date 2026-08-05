/**
 * IdentityStore implementation over Android Keystore (StrongBox when available):
 * generates the non-exportable P-256 identity key on first run and exposes
 * DeviceIdentity. Private key operations never leave the Keystore.
 */
package com.tandem.gateway.crypto

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import com.tandem.gateway.domain.model.DeviceIdentity
import com.tandem.gateway.domain.port.CryptoError
import com.tandem.gateway.domain.port.IdentityStore
import com.tandem.gateway.domain.port.SettingsRepository
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.cert.X509Certificate
import java.security.spec.ECGenParameterSpec
import java.util.UUID
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

@Singleton
class IdentityStoreImpl @Inject constructor(
    private val deviceCertificates: DeviceCertificates,
    private val settingsRepository: SettingsRepository,
) : IdentityStore {

    private val mutex = Mutex()
    private var cached: DeviceIdentity? = null

    override suspend fun identity(): Result<DeviceIdentity> = mutex.withLock {
        cached?.let { return Result.success(it) }

        runCatching {
            val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
            if (!keyStore.containsAlias(KEY_ALIAS)) generateKey()

            val certificate = keyStore.getCertificate(KEY_ALIAS) as? X509Certificate
                ?: throw CryptoError.CertificateFailed

            val identity = DeviceIdentity(
                deviceId = deviceId(),
                displayName = settingsRepository.deviceDisplayName.first(),
                spkiSha256 = Fingerprints.toBase64Url(
                    Fingerprints.spkiSha256(certificate.publicKey.encoded),
                ),
                certDer = certificate.encoded,
            )
            cached = identity
            identity
        }.recoverCatching { cause ->
            throw if (cause is CryptoError) cause else CryptoError.KeystoreUnavailable
        }
    }

    override suspend fun isHardwareBacked(): Boolean = runCatching {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        keyStore.containsAlias(KEY_ALIAS)
    }.getOrDefault(false)

    override suspend fun deriveShortCode(
        tlsExporter: ByteArray,
        peerSpkiSha256: String,
    ): Result<String> = runCatching {
        val self = identity().getOrThrow()
        Fingerprints.deriveShortCode(
            tlsExporter = tlsExporter,
            phoneSpkiSha256 = Fingerprints.fromBase64Url(self.spkiSha256),
            desktopSpkiSha256 = Fingerprints.fromBase64Url(peerSpkiSha256),
        )
    }

    /**
     * StrongBox is preferred but not universal; falling back to the TEE keeps the
     * key non-exportable, which is the property the trust model relies on.
     */
    private fun generateKey() {
        val builder = KeyGenParameterSpec.Builder(
            KEY_ALIAS,
            KeyProperties.PURPOSE_SIGN or KeyProperties.PURPOSE_VERIFY,
        )
            .setAlgorithmParameterSpec(ECGenParameterSpec("secp256r1"))
            // DIGEST_NONE is not optional: the TLS stack hashes the handshake
            // transcript itself and asks the keystore for a raw ECDSA signature
            // over that digest. A key without it cannot serve TLS at all.
            .setDigests(
                KeyProperties.DIGEST_NONE,
                KeyProperties.DIGEST_SHA256,
                KeyProperties.DIGEST_SHA384,
                KeyProperties.DIGEST_SHA512,
            )
            .setCertificateSubject(deviceCertificates.subject())
            .setCertificateNotBefore(deviceCertificates.notBefore())
            .setCertificateNotAfter(deviceCertificates.notAfter())

        val generator = KeyPairGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_EC,
            ANDROID_KEYSTORE,
        )

        runCatching {
            generator.initialize(builder.setIsStrongBoxBacked(true).build())
            generator.generateKeyPair()
        }.recoverCatching {
            generator.initialize(builder.setIsStrongBoxBacked(false).build())
            generator.generateKeyPair()
        }.getOrElse { throw CryptoError.KeyGenerationFailed }
    }

    private suspend fun deviceId(): String {
        val name = settingsRepository.deviceDisplayName.first()
        return if (name.isNotEmpty()) {
            UUID.nameUUIDFromBytes(name.toByteArray()).toString()
        } else {
            UUID.randomUUID().toString()
        }
    }

    private companion object {
        const val ANDROID_KEYSTORE = "AndroidKeyStore"
        const val KEY_ALIAS = "tandem-gateway-identity"
    }
}
