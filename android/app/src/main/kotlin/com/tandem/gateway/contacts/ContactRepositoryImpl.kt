/**
 * ContactRepository over ContactsContract: reads name-ordered dialable numbers a
 * page at a time and derives a directory version from the provider's own change
 * counters. Missing READ_CONTACTS is reported as a typed failure rather than an
 * empty address book, so the desktop can explain the gap.
 */
package com.tandem.gateway.contacts

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.provider.ContactsContract
import androidx.core.content.ContextCompat
import com.tandem.gateway.domain.model.ContactNumber
import com.tandem.gateway.domain.port.ContactPage
import com.tandem.gateway.domain.port.ContactRepository
import com.tandem.gateway.domain.port.ContactSort
import com.tandem.gateway.domain.port.ContactSource
import com.tandem.gateway.domain.port.StoreError
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.withContext

@Singleton
class ContactRepositoryImpl @Inject constructor(
    @ApplicationContext private val context: Context,
    private val ioDispatcher: CoroutineDispatcher,
) : ContactRepository {

    override suspend fun page(
        offset: Int,
        maxEntries: Int,
        sources: Set<String>,
        sort: ContactSort,
    ): Result<ContactPage> =
        withContext(ioDispatcher) {
            if (!hasPermission()) {
                return@withContext Result.failure(StoreError.PermissionDenied)
            }

            val limit = maxEntries.coerceIn(1, ContactRepository.MAX_PAGE_SIZE)
            val entries = mutableListOf<ContactNumber>()

            runCatching {
                // One extra row answers hasMore without a second query.
                // LIMIT/OFFSET go in the URI rather than the sort order: smuggling
                // them into ORDER BY is rejected by some providers.
                val uri = ContactsContract.CommonDataKinds.Phone.CONTENT_URI
                    .buildUpon()
                    .appendQueryParameter(
                        ContactsContract.LIMIT_PARAM_KEY,
                        (limit + 1).toString(),
                    )
                    .appendQueryParameter(
                        "offset",
                        offset.coerceAtLeast(0).toString(),
                    )
                    .build()

                // An empty source set means every account; a non-empty one becomes
                // an IN clause so the provider does the filtering.
                val selection = sources
                    .takeIf { it.isNotEmpty() }
                    ?.let { types ->
                        val placeholders = types.joinToString(",") { "?" }
                        "${ContactsContract.RawContacts.ACCOUNT_TYPE} IN ($placeholders)"
                    }

                context.contentResolver.query(
                    uri,
                    PROJECTION,
                    selection,
                    sources.takeIf { it.isNotEmpty() }?.toTypedArray(),
                    orderFor(sort),
                )?.use { cursor ->
                    val idIndex =
                        cursor.getColumnIndexOrThrow(ContactsContract.CommonDataKinds.Phone.CONTACT_ID)
                    val nameIndex = cursor.getColumnIndexOrThrow(
                        ContactsContract.CommonDataKinds.Phone.DISPLAY_NAME_PRIMARY,
                    )
                    val numberIndex =
                        cursor.getColumnIndexOrThrow(ContactsContract.CommonDataKinds.Phone.NUMBER)
                    val typeIndex =
                        cursor.getColumnIndexOrThrow(ContactsContract.CommonDataKinds.Phone.TYPE)
                    val labelIndex =
                        cursor.getColumnIndexOrThrow(ContactsContract.CommonDataKinds.Phone.LABEL)
                    val starredIndex =
                        cursor.getColumnIndexOrThrow(ContactsContract.CommonDataKinds.Phone.STARRED)

                    while (cursor.moveToNext() && entries.size < limit) {
                        val number = cursor.getString(numberIndex).orEmpty()
                        if (number.isBlank()) continue

                        entries += ContactNumber(
                            contactId = cursor.getLong(idIndex).toString(),
                            displayName = cursor.getString(nameIndex).orEmpty(),
                            number = number,
                            label = labelFor(
                                cursor.getInt(typeIndex),
                                cursor.getString(labelIndex),
                            ),
                            starred = cursor.getInt(starredIndex) == 1,
                        )
                    }

                    // A row beyond the page means there is more to fetch.
                    ContactPage(
                        entries = entries,
                        hasMore = !cursor.isAfterLast && cursor.moveToNext(),
                        directoryVersion = directoryVersion(),
                    )
                } ?: ContactPage(entries, hasMore = false, directoryVersion = directoryVersion())
            }.recoverCatching { cause ->
                // Logged at error level because this ROM drops app INFO/WARN, and a
                // silent empty address book is indistinguishable from a refusal.
                android.util.Log.e(TAG, "contacts query failed", cause)
                throw StoreError.QueryFailed(cause.message.orEmpty())
            }
        }

    /**
     * The provider exposes no change token, so the version is derived from the
     * row count and the newest contact update stamp: either moving means the
     * desktop's projection is stale.
     */
    override suspend fun directoryVersion(): Long = withContext(ioDispatcher) {
        if (!hasPermission()) return@withContext 0L

        runCatching {
            context.contentResolver.query(
                ContactsContract.Contacts.CONTENT_URI,
                arrayOf(ContactsContract.Contacts.CONTACT_LAST_UPDATED_TIMESTAMP),
                null,
                null,
                "${ContactsContract.Contacts.CONTACT_LAST_UPDATED_TIMESTAMP} DESC LIMIT 1",
            )?.use { cursor ->
                val newest = if (cursor.moveToFirst()) cursor.getLong(0) else 0L
                newest * 31 + cursor.count
            } ?: 0L
        }.getOrDefault(0L)
    }

    override suspend fun sources(): Result<List<ContactSource>> = withContext(ioDispatcher) {
        if (!hasPermission()) return@withContext Result.failure(StoreError.PermissionDenied)

        runCatching {
            val counts = linkedMapOf<String, Int>()
            context.contentResolver.query(
                ContactsContract.RawContacts.CONTENT_URI,
                arrayOf(ContactsContract.RawContacts.ACCOUNT_TYPE),
                "${ContactsContract.RawContacts.DELETED} = 0",
                null,
                null,
            )?.use { cursor ->
                while (cursor.moveToNext()) {
                    val type = cursor.getString(0).orEmpty()
                    counts[type] = (counts[type] ?: 0) + 1
                }
            }

            counts.map { (type, count) ->
                ContactSource(accountType = type, label = labelForAccount(type), count = count)
            }
        }
    }

    /** Account types are machine strings; the user reads where contacts came from. */
    private fun labelForAccount(type: String): String = when {
        type.isEmpty() -> "This phone"
        type.contains("google", ignoreCase = true) -> "Google"
        type.contains("sim", ignoreCase = true) -> "SIM"
        type.contains("whatsapp", ignoreCase = true) -> "WhatsApp"
        type.contains("telegram", ignoreCase = true) -> "Telegram"
        else -> type.substringAfterLast('.').replaceFirstChar(Char::titlecase)
    }

    private fun orderFor(sort: ContactSort): String = when (sort) {
        ContactSort.NAME ->
            "${ContactsContract.CommonDataKinds.Phone.DISPLAY_NAME_PRIMARY} ASC"
        ContactSort.RECENT ->
            "${ContactsContract.Contacts.LAST_TIME_CONTACTED} DESC, " +
                "${ContactsContract.CommonDataKinds.Phone.DISPLAY_NAME_PRIMARY} ASC"
        ContactSort.STARRED_FIRST ->
            "${ContactsContract.CommonDataKinds.Phone.STARRED} DESC, " +
                "${ContactsContract.CommonDataKinds.Phone.DISPLAY_NAME_PRIMARY} ASC"
    }

    private fun hasPermission(): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.READ_CONTACTS) ==
            PackageManager.PERMISSION_GRANTED

    private fun labelFor(type: Int, custom: String?): String = when (type) {
        ContactsContract.CommonDataKinds.Phone.TYPE_MOBILE -> "Mobile"
        ContactsContract.CommonDataKinds.Phone.TYPE_HOME -> "Home"
        ContactsContract.CommonDataKinds.Phone.TYPE_WORK -> "Work"
        ContactsContract.CommonDataKinds.Phone.TYPE_MAIN -> "Main"
        ContactsContract.CommonDataKinds.Phone.TYPE_CUSTOM -> custom.orEmpty()
        else -> ""
    }

    private companion object {
        const val TAG = "TandemContacts"

        val PROJECTION = arrayOf(
            ContactsContract.CommonDataKinds.Phone.CONTACT_ID,
            ContactsContract.CommonDataKinds.Phone.DISPLAY_NAME_PRIMARY,
            ContactsContract.CommonDataKinds.Phone.NUMBER,
            ContactsContract.CommonDataKinds.Phone.TYPE,
            ContactsContract.CommonDataKinds.Phone.LABEL,
            ContactsContract.CommonDataKinds.Phone.STARRED,
        )
    }
}
