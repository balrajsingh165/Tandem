/**
 * ViewModel driving PairingScreen from PairingManager events: window open/expiry,
 * candidate arrival, short-code display, verdict submission.
 */
package com.tandem.gateway.ui.pairing

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.tandem.gateway.domain.port.PairingWindowState
import com.tandem.gateway.domain.usecase.PairDesktop
import com.tandem.gateway.pairing.PairingManagerImpl
import com.tandem.gateway.pairing.QrPayloadCodec
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

@HiltViewModel
class PairingViewModel @Inject constructor(
    private val pairDesktop: PairDesktop,
    private val pairingManager: PairingManagerImpl,
    private val qrPayloadCodec: QrPayloadCodec,
) : ViewModel() {

    val state: StateFlow<PairingWindowState> = pairingManager.state
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), PairingWindowState.Closed)

    private val _qrPayload = MutableStateFlow<String?>(null)
    val qrPayload: StateFlow<String?> = _qrPayload.asStateFlow()

    fun openWindow() = viewModelScope.launch {
        pairDesktop.openWindow().onSuccess { invitation ->
            _qrPayload.value = qrPayloadCodec.encode(invitation)
        }
    }

    fun closeWindow() = viewModelScope.launch {
        pairDesktop.closeWindow()
        _qrPayload.value = null
    }

    fun accept() = viewModelScope.launch {
        pairDesktop.submitDecision(accept = true)
        _qrPayload.value = null
    }

    fun reject() = viewModelScope.launch {
        pairDesktop.submitDecision(accept = false)
        _qrPayload.value = null
    }
}
