/**
 * ViewModel binding SettingsScreen to SettingsRepository and RevokeDesktop. UI
 * state only.
 */
package com.tandem.gateway.ui.settings

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.tandem.gateway.domain.model.PairedDesktop
import com.tandem.gateway.domain.port.PairedDeviceRepository
import com.tandem.gateway.domain.port.SettingsRepository
import com.tandem.gateway.domain.usecase.RevokeDesktop
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

data class SettingsUiState(
    val pairedDesktops: List<PairedDesktop> = emptyList(),
    val autostartEnabled: Boolean = false,
    val listenPort: Int = SettingsRepository.DEFAULT_PORT,
    val deviceDisplayName: String = "",
)

@HiltViewModel
class SettingsViewModel @Inject constructor(
    private val settingsRepository: SettingsRepository,
    private val revokeDesktop: RevokeDesktop,
    pairedDeviceRepository: PairedDeviceRepository,
) : ViewModel() {

    val uiState: StateFlow<SettingsUiState> = combine(
        pairedDeviceRepository.devices.map { list -> list.filter { !it.revoked } },
        settingsRepository.autostartEnabled,
        settingsRepository.listenPort,
        settingsRepository.deviceDisplayName,
    ) { desktops, autostart, port, name ->
        SettingsUiState(desktops, autostart, port, name)
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
