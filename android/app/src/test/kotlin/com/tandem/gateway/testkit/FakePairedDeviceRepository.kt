/**
 * In-memory PairedDeviceRepository fake for pairing, revocation, and TLS-pin
 * lookup tests.
 */
package com.tandem.gateway.testkit

import com.tandem.gateway.domain.model.DesktopPlatform
import com.tandem.gateway.domain.model.PairedDesktop
import com.tandem.gateway.domain.port.PairedDeviceRepository
import com.tandem.gateway.domain.port.StoreError
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class FakePairedDeviceRepository : PairedDeviceRepository {

    private val _devices = MutableStateFlow<List<PairedDesktop>>(emptyList())
    override val devices: StateFlow<List<PairedDesktop>> = _devices

    fun seed(
        deviceId: String,
        spkiSha256: String = "pin-$deviceId",
        revoked: Boolean = false,
        btMac: String? = null,
    ): PairedDesktop {
        val desktop = PairedDesktop(
            deviceId = deviceId,
            name = "Desktop $deviceId",
            platform = DesktopPlatform.LINUX,
            spkiSha256 = spkiSha256,
            certDer = byteArrayOf(1, 2, 3),
            btMacAddress = btMac,
            createdAtMs = 1_700_000_000_000,
            lastSeenAtMs = 1_700_000_000_000,
            revoked = revoked,
        )
        _devices.value = _devices.value.filter { it.deviceId != deviceId } + desktop
        return desktop
    }

    override suspend fun upsert(desktop: PairedDesktop): Result<Unit> {
        _devices.value = _devices.value.filter { it.deviceId != desktop.deviceId } + desktop
        return Result.success(Unit)
    }

    override suspend fun byDeviceId(deviceId: String): PairedDesktop? =
        _devices.value.firstOrNull { it.deviceId == deviceId }

    /** Mirrors the real query: a revoked device is never returned. */
    override suspend fun byPinnedKey(spkiSha256: String): PairedDesktop? =
        _devices.value.firstOrNull { it.spkiSha256 == spkiSha256 && !it.revoked }

    override suspend fun revoke(deviceId: String): Result<Unit> {
        val existing = byDeviceId(deviceId) ?: return Result.failure(StoreError.NotFound(deviceId))
        return upsert(existing.copy(revoked = true))
    }

    override suspend fun recordSeen(deviceId: String, atMs: Long) {
        byDeviceId(deviceId)?.let { upsert(it.copy(lastSeenAtMs = atMs)) }
    }

    override suspend fun setBluetoothAddress(deviceId: String, address: String): Result<Unit> {
        val existing = byDeviceId(deviceId) ?: return Result.failure(StoreError.NotFound(deviceId))
        return upsert(existing.copy(btMacAddress = address))
    }
}
