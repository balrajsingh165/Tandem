/**
 * ViewModel driving PairingScreen from PairingManager events: scanning a
 * desktop's pairing code, the legacy show-a-code window, candidate arrival,
 * short-code display, and verdict submission.
 */
package com.tandem.gateway.ui.pairing

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.tandem.gateway.domain.port.PairingError
import com.tandem.gateway.domain.port.PairingWindowState
import com.tandem.gateway.domain.usecase.PairDesktop
import com.tandem.gateway.pairing.DesktopOfferCodec
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
    private val desktopOfferCodec: DesktopOfferCodec,
) : ViewModel() {

    val state: StateFlow<PairingWindowState> = pairingManager.state
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), PairingWindowState.Closed)

    private val _scanning = MutableStateFlow(false)
    val scanning: StateFlow<Boolean> = _scanning.asStateFlow()

    private val _scanError = MutableStateFlow<String?>(null)
    val scanError: StateFlow<String?> = _scanError.asStateFlow()

    private val _qrPayload = MutableStateFlow<String?>(null)
    val qrPayload: StateFlow<String?> = _qrPayload.asStateFlow()

    fun startScanning() {
        _scanError.value = null
        _scanning.value = true
    }

    fun stopScanning() {
        _scanning.value = false
    }

    /** Abandons a window nobody claimed and reopens the camera in one step. */
    fun rescan() = viewModelScope.launch {
        pairDesktop.closeWindow()
        _scanError.value = null
        _scanning.value = true
    }

    /**
     * Called once per camera session; a code that is not a Tandem offer leaves
     * the scanner running so the user can aim at the right thing.
     */
    fun onCodeScanned(raw: String) = viewModelScope.launch {
        if (!_scanning.value) return@launch

        val offer = desktopOfferCodec.decode(raw).getOrElse { cause ->
            _scanError.value = cause.message ?: PairingError.InvalidOffer.message
            return@launch
        }

        _scanning.value = false
        pairDesktop.openScannedWindow(offer).onFailure { cause ->
            _scanError.value = cause.message
        }
    }

    fun openWindow() = viewModelScope.launch {
        pairDesktop.openWindow().onSuccess { invitation ->
            _qrPayload.value = qrPayloadCodec.encode(invitation)
        }
    }

    fun closeWindow() = viewModelScope.launch {
        pairDesktop.closeWindow()
        _qrPayload.value = null
        _scanning.value = false
        _scanError.value = null
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
