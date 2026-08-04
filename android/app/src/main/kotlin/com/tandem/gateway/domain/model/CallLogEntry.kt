/**
 * Domain model of one call-log row as mirrored to desktops: number, cached
 * display name, type, start time, duration, SIM slot. Read-only projection of
 * the OS call log.
 */
package com.tandem.gateway.domain.model

data class CallLogEntry(
    val entryId: String,
    val number: String,
    val displayName: String,
    val type: CallLogType,
    val startedAtMs: Long,
    val durationSeconds: Int,
    val simSlot: Int,
)

enum class CallLogType {
    INCOMING,
    OUTGOING,
    MISSED,
    REJECTED,
}

/** One page of history plus the version that produced it. */
data class CallLogPage(
    val entries: List<CallLogEntry>,
    val logVersion: Long,
    val hasMore: Boolean,
)
