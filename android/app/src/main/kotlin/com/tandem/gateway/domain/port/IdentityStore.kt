/**
 * Port over identity-key custody: create-if-absent and expose this device's
 * DeviceIdentity, and sign TLS handshake material. Key material stays inside the
 * implementation (Android Keystore); callers only ever see public artifacts.
 */
package com.tandem.gateway.domain.port

import com.tandem.gateway.domain.model.DeviceIdentity

interface IdentityStore {
    /** Creates the identity on first call and returns it thereafter. */
    suspend fun identity(): Result<DeviceIdentity>

    /** True when the key is held in StrongBox rather than the TEE. */
    suspend fun isHardwareBacked(): Boolean

    /**
     * Derives the pairing short code from the TLS exporter and both peers'
     * fingerprints, byte-identically to the desktop (docs/07).
     */
    suspend fun deriveShortCode(
        tlsExporter: ByteArray,
        peerSpkiSha256: String,
    ): Result<String>
}

/** Typed failures at the crypto boundary; never carries key material. */
sealed class CryptoError(message: String) : Exception(message) {
    data object KeyGenerationFailed : CryptoError("could not generate the identity key")

    data object KeystoreUnavailable : CryptoError("the Android Keystore is unavailable")

    data object CertificateFailed : CryptoError("could not build the device certificate")

    data object PinMismatch : CryptoError("the peer key did not match its pin")
}
