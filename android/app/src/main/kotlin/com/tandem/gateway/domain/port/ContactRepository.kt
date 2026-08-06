/**
 * Port over the phone's address book: one name-ordered page of dialable numbers
 * plus a directory version that changes whenever contacts do. Read-only — Tandem
 * never writes to the address book. Implemented by ContactRepositoryImpl.
 */
package com.tandem.gateway.domain.port

import com.tandem.gateway.domain.model.ContactNumber

/** How the directory is ordered. Sorting belongs to the query: sorting a page
 *  client-side would reorder only the rows in hand, which reads as a bug. */
enum class ContactSort {
    NAME,
    RECENT,
    STARRED_FIRST,
}

/** One account the address book draws from, as the phone actually reports it. */
data class ContactSource(
    val accountType: String,
    val label: String,
    val count: Int,
)

interface ContactRepository {
    /**
     * The phone caps [maxEntries] regardless of what a desktop asks for. An empty
     * [sources] means every account, which is what a dialer shows by default.
     */
    suspend fun page(
        offset: Int,
        maxEntries: Int,
        sources: Set<String> = emptySet(),
        sort: ContactSort = ContactSort.NAME,
    ): Result<ContactPage>

    /**
     * The accounts present on this phone. Never a hardcoded list: SIM account
     * types differ by OEM, so offering a choice the phone does not have would be
     * a dead option (docs/18).
     */
    suspend fun sources(): Result<List<ContactSource>>

    /**
     * Changes whenever the address book does, so a desktop can tell a stale
     * projection from a current one without re-reading every row.
     */
    suspend fun directoryVersion(): Long

    companion object {
        const val MAX_PAGE_SIZE: Int = 500
    }
}

data class ContactPage(
    val entries: List<ContactNumber>,
    val hasMore: Boolean,
    val directoryVersion: Long,
)
