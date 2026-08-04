/**
 * Domain model of a paired desktop: device id, display name, platform, pinned
 * SPKI hash, certificate bytes, optional Bluetooth MAC, timestamps, and
 * revocation flag. The phone is the authority for this set (ADR-0007).
 */
package com.tandem.gateway.domain.model

data class PairedDesktop(
    val deviceId: String,
    val name: String,
    val platform: DesktopPlatform,
    val spkiSha256: String,
    val certDer: ByteArray,
    val btMacAddress: String?,
    val createdAtMs: Long,
    val lastSeenAtMs: Long,
    val revoked: Boolean,
) {
    val canReceiveAudio: Boolean
        get() = !revoked && !btMacAddress.isNullOrEmpty()

    // certDer is a ByteArray, so identity must be compared structurally rather
    // than by reference.
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is PairedDesktop) return false
        return deviceId == other.deviceId &&
            name == other.name &&
            platform == other.platform &&
            spkiSha256 == other.spkiSha256 &&
            certDer.contentEquals(other.certDer) &&
            btMacAddress == other.btMacAddress &&
            createdAtMs == other.createdAtMs &&
            lastSeenAtMs == other.lastSeenAtMs &&
            revoked == other.revoked
    }

    override fun hashCode(): Int {
        var result = deviceId.hashCode()
        result = 31 * result + name.hashCode()
        result = 31 * result + platform.hashCode()
        result = 31 * result + spkiSha256.hashCode()
        result = 31 * result + certDer.contentHashCode()
        result = 31 * result + (btMacAddress?.hashCode() ?: 0)
        result = 31 * result + createdAtMs.hashCode()
        result = 31 * result + lastSeenAtMs.hashCode()
        result = 31 * result + revoked.hashCode()
        return result
    }
}

enum class DesktopPlatform {
    LINUX,
    WINDOWS,
    MACOS,
    UNKNOWN,
    ;

    companion object {
        fun fromWire(value: String): DesktopPlatform = when (value.lowercase()) {
            "linux" -> LINUX
            "windows" -> WINDOWS
            "macos" -> MACOS
            else -> UNKNOWN
        }
    }
}
