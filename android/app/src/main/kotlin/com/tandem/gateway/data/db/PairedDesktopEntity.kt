/**
 * Room entity mirroring domain PairedDesktop one-to-one (docs/09 schema). Mapping
 * to domain lives in PairedDeviceRepositoryImpl, keeping Room out of the domain
 * layer.
 */
package com.tandem.gateway.data.db

import androidx.room.ColumnInfo
import androidx.room.Entity
import androidx.room.Index
import androidx.room.PrimaryKey

@Entity(
    tableName = "paired_desktop",
    indices = [Index(value = ["spki_sha256"], unique = true)],
)
data class PairedDesktopEntity(
    @PrimaryKey
    @ColumnInfo(name = "device_id") val deviceId: String,
    @ColumnInfo(name = "name") val name: String,
    @ColumnInfo(name = "platform") val platform: String,
    @ColumnInfo(name = "spki_sha256") val spkiSha256: String,
    @ColumnInfo(name = "cert_der", typeAffinity = ColumnInfo.BLOB) val certDer: ByteArray,
    @ColumnInfo(name = "bt_mac") val btMac: String?,
    @ColumnInfo(name = "created_at_ms") val createdAtMs: Long,
    @ColumnInfo(name = "last_seen_at_ms") val lastSeenAtMs: Long,
    @ColumnInfo(name = "revoked") val revoked: Boolean,
) {
    override fun equals(other: Any?): Boolean {
        if (this === other) return true
        if (other !is PairedDesktopEntity) return false
        return deviceId == other.deviceId &&
            name == other.name &&
            platform == other.platform &&
            spkiSha256 == other.spkiSha256 &&
            certDer.contentEquals(other.certDer) &&
            btMac == other.btMac &&
            createdAtMs == other.createdAtMs &&
            lastSeenAtMs == other.lastSeenAtMs &&
            revoked == other.revoked
    }

    override fun hashCode(): Int {
        var result = deviceId.hashCode()
        result = 31 * result + spkiSha256.hashCode()
        result = 31 * result + certDer.contentHashCode()
        result = 31 * result + revoked.hashCode()
        return result
    }
}
