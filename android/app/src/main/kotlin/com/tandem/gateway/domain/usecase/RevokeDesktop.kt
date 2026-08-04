/**
 * Use-case: flag a desktop revoked in PairedDeviceRepository and instruct
 * LanServer to emit RevokedEvent and close its live sessions. Takes effect before
 * returning.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.port.LanServer
import com.tandem.gateway.domain.port.PairedDeviceRepository
import javax.inject.Inject

class RevokeDesktop @Inject constructor(
    private val pairedDeviceRepository: PairedDeviceRepository,
    private val lanServer: LanServer,
) {
    /**
     * The store is flagged before the session is closed, so a race that
     * reconnects during teardown still fails the pinned-key lookup.
     */
    suspend operator fun invoke(deviceId: String, reason: String): Result<Unit> {
        pairedDeviceRepository.revoke(deviceId).onFailure { return Result.failure(it) }
        lanServer.revokeSession(deviceId, reason)
        return Result.success(Unit)
    }
}
