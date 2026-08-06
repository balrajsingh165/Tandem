/**
 * ViewModel projecting ObserveCallState snapshots into in-call UI state and
 * dispatching control actions through the same use-cases the LAN path uses — one
 * command path for both surfaces.
 */
package com.tandem.gateway.ui.incall

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.tandem.gateway.contacts.CallerIdentityResolver
import com.tandem.gateway.contacts.NumberInsightResolver
import com.tandem.gateway.domain.model.AudioRoute
import com.tandem.gateway.domain.model.AudioRouteTarget
import com.tandem.gateway.domain.model.Call
import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.usecase.AnswerCall
import com.tandem.gateway.domain.usecase.EndCall
import com.tandem.gateway.domain.usecase.HoldCall
import com.tandem.gateway.domain.usecase.MergeCalls
import com.tandem.gateway.domain.usecase.ObserveCallState
import com.tandem.gateway.domain.usecase.RejectCall
import com.tandem.gateway.domain.usecase.RequestAudioRoute
import com.tandem.gateway.domain.usecase.SendDtmf
import com.tandem.gateway.domain.usecase.SetMute
import com.tandem.gateway.domain.usecase.UnholdCall
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

data class InCallUiState(
    val primaryCall: Call? = null,
    /** Offline metadata for a caller with no saved name; blank when known. */
    val callerInsight: String = "",
    /** Contact photo for the caller, when the address book has one. */
    val callerPhotoUri: String = "",
    /** True when the number is also registered on WhatsApp. */
    val callerOnWhatsApp: Boolean = false,
    val muted: Boolean = false,
    val audioRoute: AudioRoute = AudioRoute.EARPIECE,
    val canMerge: Boolean = false,
) {
    val isEmergency: Boolean get() = primaryCall?.isEmergency == true
}

@HiltViewModel
class InCallViewModel @Inject constructor(
    observeCallState: ObserveCallState,
    private val answerCall: AnswerCall,
    private val rejectCall: RejectCall,
    private val endCall: EndCall,
    private val setMute: SetMute,
    private val holdCall: HoldCall,
    private val unholdCall: UnholdCall,
    private val mergeCalls: MergeCalls,
    private val sendDtmf: SendDtmf,
    private val requestAudioRoute: RequestAudioRoute,
    private val numberInsightResolver: NumberInsightResolver,
    private val callerIdentityResolver: CallerIdentityResolver,
) : ViewModel() {

    private val snapshots: StateFlow<CallSnapshot?> = observeCallState()
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), null)

    val uiState: StateFlow<InCallUiState> = snapshots
        .map { snapshot ->
            val live = snapshot?.calls.orEmpty().filter { !it.isTerminal }
            val primary = live.firstOrNull { it.isRinging } ?: live.firstOrNull()
            val identity = primary?.let { callerIdentityResolver.identityFor(it.remoteNumber) }

            InCallUiState(
                primaryCall = primary,
                // Only worth showing when there is no name to show instead: it
                // answers "who is this?" for an unknown caller, offline.
                callerInsight = primary
                    ?.takeIf { it.remoteDisplayName.isBlank() }
                    ?.let { numberInsightResolver.insightFor(it.remoteNumber)?.summary }
                    .orEmpty(),
                callerPhotoUri = identity?.photoUri.orEmpty(),
                callerOnWhatsApp = identity?.onWhatsApp == true,
                muted = snapshot?.microphoneMuted ?: false,
                audioRoute = snapshot?.audioRoute ?: AudioRoute.EARPIECE,
                canMerge = live.size > 1,
            )
        }
        .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), InCallUiState())

    /**
     * The handset is a local surface, so it has no device id to arbitrate with;
     * it claims under a reserved identifier so the first-answer-wins rule still
     * holds against competing desktops.
     */
    fun answer(callId: String) = viewModelScope.launch {
        answerCall(callId, HANDSET_DEVICE_ID)
    }

    fun reject(callId: String) = viewModelScope.launch { rejectCall(callId) }

    fun end(callId: String) = viewModelScope.launch { endCall(callId, snapshots.value) }

    fun setMuted(muted: Boolean) = viewModelScope.launch { setMute(muted) }

    fun hold(callId: String) = viewModelScope.launch { holdCall(callId, snapshots.value) }

    fun unhold(callId: String) = viewModelScope.launch { unholdCall(callId, snapshots.value) }

    fun merge(callId: String) = viewModelScope.launch {
        mergeCalls(callId, "", snapshots.value)
    }

    fun dtmf(callId: String, digit: String) = viewModelScope.launch {
        sendDtmf(callId, digit, snapshots.value)
    }

    fun setAudioRoute(route: AudioRoute, address: String = "") = viewModelScope.launch {
        requestAudioRoute(AudioRouteTarget(route, address), snapshots.value)
    }

    private companion object {
        const val HANDSET_DEVICE_ID = "handset"
    }
}
