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

    override suspend fun page(offset: Int, maxEntries: Int): Result<ContactPage> =
        withContext(ioDispatcher) {
            if (!hasPermission()) {
                return@withContext Result.failure(StoreError.PermissionDenied)
            }

            val limit = maxEntries.coerceIn(1, ContactRepository.MAX_PAGE_SIZE)
            val entries = mutableListOf<ContactNumber>()

            runCatching {
                // One extra row answers hasMore without a second query.
                context.contentResolver.query(
                    ContactsContract.CommonDataKinds.Phone.CONTENT_URI,
                    PROJECTION,
                    null,
                    null,
                    "${ContactsContract.CommonDataKinds.Phone.DISPLAY_NAME_PRIMARY} ASC " +
                        "LIMIT ${limit + 1} OFFSET ${offset.coerceAtLeast(0)}",
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
            }.recoverCatching { throw StoreError.QueryFailed(it.message.orEmpty()) }
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
