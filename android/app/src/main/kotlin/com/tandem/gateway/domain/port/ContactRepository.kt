/**
 * Port over the phone's address book: one name-ordered page of dialable numbers
 * plus a directory version that changes whenever contacts do. Read-only — Tandem
 * never writes to the address book. Implemented by ContactRepositoryImpl.
 */
package com.tandem.gateway.domain.port

import com.tandem.gateway.domain.model.ContactNumber

interface ContactRepository {
    /** The phone caps [maxEntries] regardless of what a desktop asks for. */
    suspend fun page(offset: Int, maxEntries: Int): Result<ContactPage>

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
