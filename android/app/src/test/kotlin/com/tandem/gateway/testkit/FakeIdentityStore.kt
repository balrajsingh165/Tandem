/**
 * IdentityStore fake with a fixed test keypair and fingerprint so pairing and TLS
 * tests are deterministic.
 */
package com.tandem.gateway.testkit

import com.tandem.gateway.domain.model.DeviceIdentity
import com.tandem.gateway.domain.port.CryptoError
import com.tandem.gateway.domain.port.IdentityStore

class FakeIdentityStore(
    private val identity: DeviceIdentity = DEFAULT_IDENTITY,
) : IdentityStore {

    var failNextIdentity: Boolean = false
    var hardwareBacked: Boolean = true

    /** Fixed so short codes are reproducible across runs. */
    var shortCode: String = "123456"

    override suspend fun identity(): Result<DeviceIdentity> {
        if (failNextIdentity) {
            failNextIdentity = false
            return Result.failure(CryptoError.KeystoreUnavailable)
        }
        return Result.success(identity)
    }

    override suspend fun isHardwareBacked(): Boolean = hardwareBacked

    override suspend fun deriveShortCode(
        tlsExporter: ByteArray,
        peerSpkiSha256: String,
    ): Result<String> = Result.success(shortCode)

    companion object {
        val DEFAULT_IDENTITY = DeviceIdentity(
            deviceId = "phone-test-0000",
            displayName = "Test Phone",
            spkiSha256 = "dGVzdC1waG9uZS1zcGtp",
            certDer = byteArrayOf(9, 8, 7),
        )
    }
}
