/**
 * ViewModel deriving StatusScreen state from ObserveCallState, LanServer status,
 * and repositories. UI state only; commands delegate to use-cases.
 */
package com.tandem.gateway.ui.status

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.tandem.gateway.dialer.DefaultDialerManager
import com.tandem.gateway.domain.model.AudioRoute
import com.tandem.gateway.domain.port.LanServer
import com.tandem.gateway.domain.usecase.ObserveCallState
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.stateIn

data class StatusUiState(
    val isDefaultDialer: Boolean = false,
    val listening: Boolean = false,
    val port: Int = 0,
    val connectedDesktops: Int = 0,
    val audioRoute: AudioRoute = AudioRoute.EARPIECE,
    val activeCalls: Int = 0,
    val hasActiveEmergency: Boolean = false,
)

@HiltViewModel
class StatusViewModel @Inject constructor(
    private val defaultDialerManager: DefaultDialerManager,
    lanServer: LanServer,
    observeCallState: ObserveCallState,
) : ViewModel() {

    private val dialerRoleHeld = MutableStateFlow(defaultDialerManager.isDefaultDialer())

    val uiState: StateFlow<StatusUiState> = combine(
        dialerRoleHeld,
        lanServer.status,
        lanServer.connectedSessions,
        observeCallState(),
    ) { roleHeld, serverStatus, sessions, snapshot ->
        StatusUiState(
            isDefaultDialer = roleHeld,
            listening = serverStatus.listening,
            port = serverStatus.port,
            connectedDesktops = sessions.size,
            audioRoute = snapshot.audioRoute,
            activeCalls = snapshot.calls.count { !it.isTerminal },
            hasActiveEmergency = snapshot.hasActiveEmergency(),
        )
    }.stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), StatusUiState())

    /** Re-checked on resume because the role can change outside the app. */
    fun refreshDialerRole() {
        dialerRoleHeld.value = defaultDialerManager.isDefaultDialer()
    }

    fun roleRequestIntent() = defaultDialerManager.buildRoleRequestIntent()
}
