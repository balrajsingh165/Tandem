/**
 * Domain model of this phone's own identity: device id, display name,
 * SPKI-SHA256 fingerprint, and certificate bytes. Private key material never
 * leaves IdentityStore.
 */
package com.tandem.gateway.domain.model

data class DeviceIdentity(
    val deviceId: String,
    val displayName: String,
    val spkiSha256: String,
    val certDer: ByteArray,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is DeviceIdentity) return false
        return deviceId == other.deviceId &&
            displayName == other.displayName &&
            spkiSha256 == other.spkiSha256 &&
            certDer.contentEquals(other.certDer)
    }

    override fun hashCode(): Int {
        var result = deviceId.hashCode()
        result = 31 * result + displayName.hashCode()
        result = 31 * result + spkiSha256.hashCode()
        result = 31 * result + certDer.contentHashCode()
        return result
    }
}
