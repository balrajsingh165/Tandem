/**
 * Everything the address book knows about a caller: saved name, contact photo, and
 * whether the number is also a WhatsApp contact. All of it comes from
 * ContactsContract — WhatsApp's own sync adapter puts its contacts and photos
 * there, which is the only sanctioned way to see them (docs/18).
 */
package com.tandem.gateway.contacts

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.ContactsContract
import androidx.core.content.ContextCompat
import dagger.hilt.android.qualifiers.ApplicationContext
import java.util.concurrent.ConcurrentHashMap
import javax.inject.Inject
import javax.inject.Singleton

/** The WhatsApp sync adapter's account type; its raw contacts carry this. */
private const val WHATSAPP_ACCOUNT = "com.whatsapp"

data class CallerIdentity(
    val displayName: String,
    val photoUri: String,
    /** True when this number also exists as a WhatsApp contact on this phone. */
    val onWhatsApp: Boolean,
) {
    val isEmpty: Boolean
        get() = displayName.isEmpty() && photoUri.isEmpty() && !onWhatsApp
}

@Singleton
class CallerIdentityResolver @Inject constructor(
    @ApplicationContext private val context: Context,
) {
    private val memo = ConcurrentHashMap<String, CallerIdentity>()

    fun identityFor(number: String): CallerIdentity? {
        if (number.isBlank()) return null
        memo[number]?.let { return it.takeUnless { cached -> cached.isEmpty } }

        if (ContextCompat.checkSelfPermission(context, Manifest.permission.READ_CONTACTS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            return null
        }

        val resolved = runCatching { lookup(number) }.getOrDefault(EMPTY)
        memo[number] = resolved
        return resolved.takeUnless { it.isEmpty }
    }

    private fun lookup(number: String): CallerIdentity {
        val uri = Uri.withAppendedPath(
            ContactsContract.PhoneLookup.CONTENT_FILTER_URI,
            Uri.encode(number),
        )

        val (contactId, name, photo) = context.contentResolver.query(
            uri,
            arrayOf(
                ContactsContract.PhoneLookup.CONTACT_ID,
                ContactsContract.PhoneLookup.DISPLAY_NAME,
                ContactsContract.PhoneLookup.PHOTO_URI,
            ),
            null,
            null,
            null,
        )?.use { cursor ->
            if (cursor.moveToFirst()) {
                Triple(cursor.getLong(0), cursor.getString(1).orEmpty(), cursor.getString(2).orEmpty())
            } else {
                Triple(0L, "", "")
            }
        } ?: Triple(0L, "", "")

        if (contactId == 0L) return EMPTY

        return CallerIdentity(
            displayName = name,
            photoUri = photo,
            onWhatsApp = hasWhatsAppRawContact(contactId),
        )
    }

    /**
     * A WhatsApp raw contact under this person means they are reachable there. It
     * says nothing about their WhatsApp profile — that is not readable by any app
     * (docs/18) — only that the number is registered.
     */
    private fun hasWhatsAppRawContact(contactId: Long): Boolean =
        context.contentResolver.query(
            ContactsContract.RawContacts.CONTENT_URI,
            arrayOf(ContactsContract.RawContacts._ID),
            "${ContactsContract.RawContacts.CONTACT_ID} = ? AND " +
                "${ContactsContract.RawContacts.ACCOUNT_TYPE} = ?",
            arrayOf(contactId.toString(), WHATSAPP_ACCOUNT),
            null,
        )?.use { it.count > 0 } ?: false

    fun invalidate() = memo.clear()

    private companion object {
        val EMPTY = CallerIdentity("", "", false)
    }
}
