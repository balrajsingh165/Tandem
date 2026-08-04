/**
 * PairedDeviceRepository implementation bridging PairedDesktopDao rows and domain
 * models. Owns the entity/domain mapping; enforces that revocation is a flag-set,
 * never a hard delete, so audit history survives.
 */
package com.tandem.gateway.data

import com.tandem.gateway.data.db.PairedDesktopDao
import com.tandem.gateway.data.db.PairedDesktopEntity
import com.tandem.gateway.domain.model.DesktopPlatform
import com.tandem.gateway.domain.model.PairedDesktop
import com.tandem.gateway.domain.port.PairedDeviceRepository
import com.tandem.gateway.domain.port.StoreError
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

@Singleton
class PairedDeviceRepositoryImpl @Inject constructor(
    private val dao: PairedDesktopDao,
) : PairedDeviceRepository {

    override val devices: Flow<List<PairedDesktop>> =
        dao.observeAll().map { rows -> rows.map(::toDomain) }

    override suspend fun upsert(desktop: PairedDesktop): Result<Unit> = runCatching {
        dao.upsert(toEntity(desktop))
    }.recoverCatching { cause ->
        throw StoreError.WriteFailed(cause.message ?: "could not persist paired desktop")
    }

    override suspend fun byDeviceId(deviceId: String): PairedDesktop? =
        dao.byDeviceId(deviceId)?.let(::toDomain)

    override suspend fun byPinnedKey(spkiSha256: String): PairedDesktop? =
        dao.byPinnedKey(spkiSha256)?.let(::toDomain)

    override suspend fun revoke(deviceId: String): Result<Unit> = runCatching {
        val updated = dao.revoke(deviceId)
        if (updated == 0) throw StoreError.NotFound(deviceId)
    }

    override suspend fun recordSeen(deviceId: String, atMs: Long) {
        runCatching { dao.recordSeen(deviceId, atMs) }
    }

    override suspend fun setBluetoothAddress(deviceId: String, address: String): Result<Unit> =
        runCatching {
            val updated = dao.setBluetoothAddress(deviceId, address)
            if (updated == 0) throw StoreError.NotFound(deviceId)
        }

    private fun toDomain(entity: PairedDesktopEntity) = PairedDesktop(
        deviceId = entity.deviceId,
        name = entity.name,
        platform = DesktopPlatform.fromWire(entity.platform),
        spkiSha256 = entity.spkiSha256,
        certDer = entity.certDer,
        btMacAddress = entity.btMac,
        createdAtMs = entity.createdAtMs,
        lastSeenAtMs = entity.lastSeenAtMs,
        revoked = entity.revoked,
    )

    private fun toEntity(desktop: PairedDesktop) = PairedDesktopEntity(
        deviceId = desktop.deviceId,
        name = desktop.name,
        platform = desktop.platform.name.lowercase(),
        spkiSha256 = desktop.spkiSha256,
        certDer = desktop.certDer,
        btMac = desktop.btMacAddress,
        createdAtMs = desktop.createdAtMs,
        lastSeenAtMs = desktop.lastSeenAtMs,
        revoked = desktop.revoked,
    )
}
