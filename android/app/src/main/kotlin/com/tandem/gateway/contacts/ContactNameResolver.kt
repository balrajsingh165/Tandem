/**
 * Resolves a saved contact name for a dialing number via ContactsContract
 * PhoneLookup, with a small memo so an in-call screen recomposing does not requery
 * the provider. Telecom only reports carrier CNAP, which is usually empty, so
 * without this a saved contact shows as a bare number.
 */
package com.tandem.gateway.contacts

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.ContactsContract
import androidx.core.content.ContextCompat
import java.util.concurrent.ConcurrentHashMap
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ContactNameResolver @Inject constructor(
    @dagger.hilt.android.qualifiers.ApplicationContext private val context: Context,
) {
    private val memo = ConcurrentHashMap<String, String>()

    /**
     * The saved name for [number], or null when there is no match. A miss is
     * memoized as an empty string so repeated lookups for an unknown caller stay
     * free.
     */
    fun nameFor(number: String): String? {
        if (number.isBlank()) return null
        memo[number]?.let { return it.ifEmpty { null } }

        if (ContextCompat.checkSelfPermission(context, Manifest.permission.READ_CONTACTS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            return null
        }

        val resolved = runCatching {
            val uri = Uri.withAppendedPath(
                ContactsContract.PhoneLookup.CONTENT_FILTER_URI,
                Uri.encode(number),
            )
            context.contentResolver.query(
                uri,
                arrayOf(ContactsContract.PhoneLookup.DISPLAY_NAME),
                null,
                null,
                null,
            )?.use { cursor ->
                if (cursor.moveToFirst()) cursor.getString(0).orEmpty() else ""
            }.orEmpty()
        }.getOrDefault("")

        memo[number] = resolved
        return resolved.ifEmpty { null }
    }

    /** Dropped when the address book changes, so a rename is picked up. */
    fun invalidate() = memo.clear()
}
