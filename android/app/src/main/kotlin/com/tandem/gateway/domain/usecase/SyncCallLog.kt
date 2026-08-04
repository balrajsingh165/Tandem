/**
 * Use-case: serve CallLogSyncRequest pages from CallLogRepository and expose the
 * current log_version. Read-only; retention/refresh policy in
 * docs/09-data-models.md.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.model.CallLogPage
import com.tandem.gateway.domain.port.CallLogRepository
import javax.inject.Inject

class SyncCallLog @Inject constructor(
    private val callLogRepository: CallLogRepository,
) {
    /** The server caps [maxEntries] regardless of what a desktop asks for. */
    suspend operator fun invoke(sinceMs: Long, maxEntries: Int): Result<CallLogPage> {
        val capped = maxEntries.coerceIn(1, CallLogRepository.MAX_PAGE_SIZE)
        return callLogRepository.page(sinceMs.coerceAtLeast(0), capped)
    }

    suspend fun currentVersion(): Long = callLogRepository.currentVersion()
}
