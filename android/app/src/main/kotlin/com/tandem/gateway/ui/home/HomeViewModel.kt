/**
 * ViewModel behind the phone's Recents and Contacts tabs: loads a page of the OS
 * call log and the address book, and places a call from either. Read-only over
 * both providers; dialing goes through PlaceCall so the emergency policy and
 * dialer-role checks still apply.
 */
package com.tandem.gateway.ui.home

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.tandem.gateway.contacts.NumberInsightResolver
import com.tandem.gateway.domain.model.CallLogEntry
import com.tandem.gateway.domain.model.ContactNumber
import com.tandem.gateway.domain.port.CallLogRepository
import com.tandem.gateway.domain.port.ContactRepository
import com.tandem.gateway.domain.port.ContactSort
import com.tandem.gateway.domain.port.ContactSource
import com.tandem.gateway.domain.usecase.PlaceCall
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

private const val RECENTS_PAGE = 100
private const val CONTACTS_PAGE = 400

data class HomeUiState(
    val recents: List<CallLogEntry> = emptyList(),
    val contacts: List<ContactNumber> = emptyList(),
    /** Accounts this phone actually has, for the source picker. */
    val sources: List<ContactSource> = emptyList(),
    val selectedSources: Set<String> = emptySet(),
    val sort: ContactSort = ContactSort.NAME,
    val loading: Boolean = true,
    /** Set when a provider refused; the tab says so rather than looking empty. */
    val notice: String? = null,
)

@HiltViewModel
class HomeViewModel @Inject constructor(
    private val callLogRepository: CallLogRepository,
    private val contactRepository: ContactRepository,
    private val numberInsightResolver: NumberInsightResolver,
    private val placeCall: PlaceCall,
) : ViewModel() {

    private val _uiState = MutableStateFlow(HomeUiState())
    val uiState: StateFlow<HomeUiState> = _uiState.asStateFlow()

    private val _failure = MutableStateFlow<String?>(null)
    val failure: StateFlow<String?> = _failure.asStateFlow()

    init {
        refresh()
    }

    fun setSort(sort: ContactSort) {
        _uiState.value = _uiState.value.copy(sort = sort)
        refresh()
    }

    /** Toggling the last selected source falls back to every account. */
    fun toggleSource(accountType: String) {
        val current = _uiState.value.selectedSources
        val next = if (accountType in current) current - accountType else current + accountType
        _uiState.value = _uiState.value.copy(selectedSources = next)
        refresh()
    }

    fun refresh() = viewModelScope.launch {
        val previous = _uiState.value
        _uiState.value = previous.copy(loading = true, notice = null)

        val log = callLogRepository.page(0, RECENTS_PAGE, 0)
        val book = contactRepository.page(
            offset = 0,
            maxEntries = CONTACTS_PAGE,
            sources = previous.selectedSources,
            sort = previous.sort,
        )
        val sources = contactRepository.sources().getOrNull().orEmpty()

        _uiState.value = HomeUiState(
            recents = log.getOrNull()?.entries.orEmpty(),
            contacts = book.getOrNull()?.entries.orEmpty(),
            sources = sources,
            selectedSources = previous.selectedSources,
            sort = previous.sort,
            loading = false,
            notice = listOfNotNull(
                "Call history needs permission".takeIf { log.isFailure },
                "Contacts need permission".takeIf { book.isFailure },
            ).joinToString(" · ").ifEmpty { null },
        )
    }

    /** Region, carrier and line type for a number with no saved name. */
    fun insightFor(number: String): String =
        numberInsightResolver.insightFor(number)?.summary.orEmpty()

    fun call(number: String) = viewModelScope.launch {
        placeCall(number, simSlot = -1, fromDesktop = false)
            .onFailure { _failure.value = it.message ?: "Could not place the call" }
    }

    fun clearFailure() {
        _failure.value = null
    }
}
