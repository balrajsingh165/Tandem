/**
 * Room DAO for paired_desktop rows: upsert, revoke-flag update, lookup by SPKI
 * hash for TLS accept, and an observable list for the settings UI.
 */
package com.tandem.gateway.data.db

import androidx.room.Dao
import androidx.room.Query
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow

@Dao
interface PairedDesktopDao {

    @Query("SELECT * FROM paired_desktop ORDER BY created_at_ms DESC")
    fun observeAll(): Flow<List<PairedDesktopEntity>>

    @Upsert
    suspend fun upsert(entity: PairedDesktopEntity)

    @Query("SELECT * FROM paired_desktop WHERE device_id = :deviceId LIMIT 1")
    suspend fun byDeviceId(deviceId: String): PairedDesktopEntity?

    /** Hot path: consulted on every TLS handshake, so it must stay indexed. */
    @Query("SELECT * FROM paired_desktop WHERE spki_sha256 = :spkiSha256 AND revoked = 0 LIMIT 1")
    suspend fun byPinnedKey(spkiSha256: String): PairedDesktopEntity?

    /** Revocation flags rather than deletes, so the audit trail survives. */
    @Query("UPDATE paired_desktop SET revoked = 1 WHERE device_id = :deviceId")
    suspend fun revoke(deviceId: String): Int

    @Query("UPDATE paired_desktop SET last_seen_at_ms = :atMs WHERE device_id = :deviceId")
    suspend fun recordSeen(deviceId: String, atMs: Long)

    @Query("UPDATE paired_desktop SET bt_mac = :address WHERE device_id = :deviceId")
    suspend fun setBluetoothAddress(deviceId: String, address: String): Int
}
