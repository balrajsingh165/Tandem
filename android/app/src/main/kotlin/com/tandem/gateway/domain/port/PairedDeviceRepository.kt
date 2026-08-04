/**
 * Port over the paired-desktop store: CRUD for PairedDesktop rows, revocation
 * flagging, and lookup by pinned SPKI hash during TLS handshakes.
 */
package com.tandem.gateway.domain.port

import com.tandem.gateway.domain.model.PairedDesktop
import kotlinx.coroutines.flow.Flow

interface PairedDeviceRepository {
    val devices: Flow<List<PairedDesktop>>

    suspend fun upsert(desktop: PairedDesktop): Result<Unit>

    suspend fun byDeviceId(deviceId: String): PairedDesktop?

    /**
     * Called on every TLS handshake, so it must be fast and must never return a
     * revoked device.
     */
    suspend fun byPinnedKey(spkiSha256: String): PairedDesktop?

    /** Revocation is a flag, never a delete, so the audit trail survives. */
    suspend fun revoke(deviceId: String): Result<Unit>

    suspend fun recordSeen(deviceId: String, atMs: Long)

    suspend fun setBluetoothAddress(deviceId: String, address: String): Result<Unit>
}
