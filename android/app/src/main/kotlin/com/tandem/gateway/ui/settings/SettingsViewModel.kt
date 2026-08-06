/**
 * ViewModel binding SettingsScreen to SettingsRepository and RevokeDesktop. UI
 * state only.
 */
package com.tandem.gateway.ui.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.tandem.gateway.domain.model.PairedDesktop
import com.tandem.gateway.domain.port.CallLogRepository
import com.tandem.gateway.domain.port.ContactRepository
import com.tandem.gateway.domain.port.ContactSource
import com.tandem.gateway.domain.port.PairedDeviceRepository
import com.tandem.gateway.domain.port.SettingsRepository
import com.tandem.gateway.domain.usecase.RevokeDesktop
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

data class SettingsUiState(
    val pairedDesktops: List<PairedDesktop> = emptyList(),
    /** What a paired computer is allowed to read from this phone. */
    val contactSources: List<ContactSource> = emptyList(),
    val callLogEntries: Int = 0,
    val contactsShared: Int = 0,
    val autostartEnabled: Boolean = false,
    val listenPort: Int = SettingsRepository.DEFAULT_PORT,
    val deviceDisplayName: String = "",
)

@HiltViewModel
class SettingsViewModel @Inject constructor(
    private val settingsRepository: SettingsRepository,
    private val revokeDesktop: RevokeDesktop,
    private val contactRepository: ContactRepository,
    private val callLogRepository: CallLogRepository,
    pairedDeviceRepository: PairedDeviceRepository,
) : ViewModel() {

    private val syncFacts = MutableStateFlow(SyncFacts())

    /** What is actually being shared, read once rather than guessed at. */
    private data class SyncFacts(
        val sources: List<ContactSource> = emptyList(),
        val contacts: Int = 0,
        val calls: Int = 0,
    )

    init {
        viewModelScope.launch {
            val sources = contactRepository.sources().getOrNull().orEmpty()
            syncFacts.value = SyncFacts(
                sources = sources,
                contacts = sources.sumOf { it.count },
                calls = callLogRepository.page(0, 1, 0).getOrNull()?.let { 1 } ?: 0,
            )
        }
    }

    val uiState: StateFlow<SettingsUiState> = combine(
        pairedDeviceRepository.devices.map { list -> list.filter { !it.revoked } },
        settingsRepository.autostartEnabled,
        settingsRepository.listenPort,
        settingsRepository.deviceDisplayName,
        syncFacts,
    ) { values ->
        @Suppress("UNCHECKED_CAST")
        val desktops = values[0] as List<PairedDesktop>
        val facts = values[4] as SyncFacts
        SettingsUiState(
            pairedDesktops = desktops,
            contactSources = facts.sources,
            callLogEntries = facts.calls,
            contactsShared = facts.contacts,
            autostartEnabled = values[1] as Boolean,
            listenPort = values[2] as Int,
            deviceDisplayName = values[3] as String,
        )
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), SettingsUiState())

    fun setAutostart(enabled: Boolean) = viewModelScope.launch {
        settingsRepository.setAutostartEnabled(enabled)
    }

    fun setDeviceName(name: String) = viewModelScope.launch {
        settingsRepository.setDeviceDisplayName(name)
    }

    /** Takes effect immediately: the desktop's live session is closed too. */
    fun revoke(deviceId: String) = viewModelScope.launch {
        revokeDesktop(deviceId, reason = "removed on the phone")
    }
}
