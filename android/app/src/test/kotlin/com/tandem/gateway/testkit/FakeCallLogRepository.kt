/**
 * In-memory CallLogRepository fake seeded with fixture entries; supports paging
 * bounds and log-version bumps for sync tests.
 */
package com.tandem.gateway.testkit

import com.tandem.gateway.domain.model.CallLogEntry
import com.tandem.gateway.domain.model.CallLogPage
import com.tandem.gateway.domain.model.CallLogType
import com.tandem.gateway.domain.port.CallLogRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class FakeCallLogRepository : CallLogRepository {

    private val _logVersion = MutableStateFlow(1L)
    override val logVersion: StateFlow<Long> = _logVersion

    var entries: List<CallLogEntry> = emptyList()

    fun seed(count: Int) {
        entries = (0 until count).map { index ->
            CallLogEntry(
                entryId = "entry-$index",
                number = "+1415555%04d".format(index),
                displayName = "Contact $index",
                type = CallLogType.INCOMING,
                startedAtMs = 1_700_000_000_000 - index * 60_000L,
                durationSeconds = 30,
                simSlot = 0,
            )
        }
    }

    fun bumpVersion() {
        _logVersion.value += 1
    }

    override suspend fun page(
        sinceMs: Long,
        maxEntries: Int,
        beforeMs: Long,
    ): Result<CallLogPage> {
        val matching = entries
            .filter { it.startedAtMs >= sinceMs && (beforeMs <= 0 || it.startedAtMs < beforeMs) }
            .sortedByDescending { it.startedAtMs }
        val page = matching.take(maxEntries)
        return Result.success(
            CallLogPage(
                entries = page,
                logVersion = _logVersion.value,
                hasMore = matching.size > page.size,
            ),
        )
    }

    override suspend fun currentVersion(): Long = _logVersion.value
}
