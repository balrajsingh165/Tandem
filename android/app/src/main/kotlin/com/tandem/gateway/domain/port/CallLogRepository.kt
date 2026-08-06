/**
 * Port over the OS call log: paged reads since a timestamp plus a Flow of change
 * notifications with a monotonic log version. Strictly read-only (no writes to
 * the OS log).
 */
package com.tandem.gateway.domain.port

import com.tandem.gateway.domain.model.CallLogPage
import kotlinx.coroutines.flow.Flow

interface CallLogRepository {
    /** Emits the new version whenever the OS call log changes. */
    val logVersion: Flow<Long>

    /**
     * Returns entries with startedAtMs >= [sinceMs], newest first, capped at
     * [MAX_PAGE_SIZE] regardless of what the caller asks for.
     */
    /**
     * One newest-first page. [beforeMs] is an exclusive upper bound so a desktop
     * can walk the whole log; 0 asks for the newest rows.
     */
    suspend fun page(sinceMs: Long, maxEntries: Int, beforeMs: Long = 0): Result<CallLogPage>

    suspend fun currentVersion(): Long

    companion object {
        const val MAX_PAGE_SIZE: Int = 200
    }
}

/** Typed failures at the storage boundary, shared by the repository ports. */
sealed class StoreError(message: String) : Exception(message) {
    data object PermissionDenied : StoreError("READ_CALL_LOG was denied")

    data class QueryFailed(val reason: String) : StoreError(reason)

    data class NotFound(val key: String) : StoreError("no record for $key")

    data class WriteFailed(val reason: String) : StoreError(reason)
}
