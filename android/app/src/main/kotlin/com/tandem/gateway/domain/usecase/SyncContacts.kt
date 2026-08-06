/**
 * Use-case: serve ContactsSyncRequest pages from ContactRepository and expose the
 * current directory version. Read-only; the address book belongs to the phone.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.port.ContactPage
import com.tandem.gateway.domain.port.ContactRepository
import javax.inject.Inject

class SyncContacts @Inject constructor(
    private val contactRepository: ContactRepository,
) {
    /** The phone caps [maxEntries] regardless of what a desktop asks for. */
    suspend operator fun invoke(offset: Int, maxEntries: Int): Result<ContactPage> =
        contactRepository.page(
            offset.coerceAtLeast(0),
            maxEntries.coerceIn(1, ContactRepository.MAX_PAGE_SIZE),
        )

    suspend fun directoryVersion(): Long = contactRepository.directoryVersion()
}
