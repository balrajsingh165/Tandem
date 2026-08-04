/**
 * CallLogRepository implementation querying android.provider.CallLog.Calls with
 * paged, timestamp-bounded projections (READ_CALL_LOG). Read-only by design;
 * never writes or deletes OS call-log rows.
 */
package com.tandem.gateway.calllog

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.provider.CallLog
import androidx.core.content.ContextCompat
import com.tandem.gateway.domain.model.CallLogEntry
import com.tandem.gateway.domain.model.CallLogPage
import com.tandem.gateway.domain.model.CallLogType
import com.tandem.gateway.domain.port.CallLogRepository
import com.tandem.gateway.domain.port.StoreError
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.withContext

@Singleton
class CallLogRepositoryImpl @Inject constructor(
    @ApplicationContext private val context: Context,
    private val callLogObserver: CallLogObserver,
    private val ioDispatcher: CoroutineDispatcher,
) : CallLogRepository {

    override val logVersion: Flow<Long> = callLogObserver.logVersion

    override suspend fun page(sinceMs: Long, maxEntries: Int): Result<CallLogPage> =
        withContext(ioDispatcher) {
            if (!hasReadCallLogPermission()) {
                return@withContext Result.failure(StoreError.PermissionDenied)
            }

            // One extra row answers hasMore without a second query.
            val limit = maxEntries.coerceIn(1, CallLogRepository.MAX_PAGE_SIZE)
            val entries = mutableListOf<CallLogEntry>()

            runCatching {
                context.contentResolver.query(
                    CallLog.Calls.CONTENT_URI,
                    PROJECTION,
                    "${CallLog.Calls.DATE} >= ?",
                    arrayOf(sinceMs.toString()),
                    "${CallLog.Calls.DATE} DESC LIMIT ${limit + 1}",
                )?.use { cursor ->
                    val idIndex = cursor.getColumnIndexOrThrow(CallLog.Calls._ID)
                    val numberIndex = cursor.getColumnIndexOrThrow(CallLog.Calls.NUMBER)
                    val nameIndex = cursor.getColumnIndexOrThrow(CallLog.Calls.CACHED_NAME)
                    val typeIndex = cursor.getColumnIndexOrThrow(CallLog.Calls.TYPE)
                    val dateIndex = cursor.getColumnIndexOrThrow(CallLog.Calls.DATE)
                    val durationIndex = cursor.getColumnIndexOrThrow(CallLog.Calls.DURATION)

                    while (cursor.moveToNext() && entries.size < limit) {
                        entries += CallLogEntry(
                            entryId = cursor.getLong(idIndex).toString(),
                            number = cursor.getString(numberIndex).orEmpty(),
                            displayName = cursor.getString(nameIndex).orEmpty(),
                            type = mapType(cursor.getInt(typeIndex)),
                            startedAtMs = cursor.getLong(dateIndex),
                            durationSeconds = cursor.getInt(durationIndex),
                            simSlot = -1,
                        )
                    }
                    return@runCatching cursor.count > limit
                }
                false
            }.fold(
                onSuccess = { hasMore ->
                    Result.success(
                        CallLogPage(
                            entries = entries,
                            logVersion = callLogObserver.currentVersion(),
                            hasMore = hasMore,
                        ),
                    )
                },
                onFailure = { cause ->
                    Result.failure(StoreError.QueryFailed(cause.message ?: "call log query failed"))
                },
            )
        }

    override suspend fun currentVersion(): Long = callLogObserver.currentVersion()

    private fun mapType(type: Int): CallLogType = when (type) {
        CallLog.Calls.INCOMING_TYPE -> CallLogType.INCOMING
        CallLog.Calls.OUTGOING_TYPE -> CallLogType.OUTGOING
        CallLog.Calls.MISSED_TYPE -> CallLogType.MISSED
        CallLog.Calls.REJECTED_TYPE -> CallLogType.REJECTED
        else -> CallLogType.INCOMING
    }

    private fun hasReadCallLogPermission(): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.READ_CALL_LOG) ==
            PackageManager.PERMISSION_GRANTED

    private companion object {
        val PROJECTION = arrayOf(
            CallLog.Calls._ID,
            CallLog.Calls.NUMBER,
            CallLog.Calls.CACHED_NAME,
            CallLog.Calls.TYPE,
            CallLog.Calls.DATE,
            CallLog.Calls.DURATION,
        )
    }
}
