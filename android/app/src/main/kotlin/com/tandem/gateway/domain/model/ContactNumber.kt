/**
 * One dialable number belonging to one contact. A contact with several numbers is
 * several of these sharing a contactId, which is what lets a desktop group them
 * without asking again.
 */
package com.tandem.gateway.domain.model

data class ContactNumber(
    val contactId: String,
    val displayName: String,
    val number: String,
    val label: String,
    val starred: Boolean,
)
