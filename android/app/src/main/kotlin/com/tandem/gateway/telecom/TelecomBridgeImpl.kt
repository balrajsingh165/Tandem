/**
 * TelecomBridge implementation: tracks Call objects registered by
 * TandemInCallService, mints stable call ids, executes control commands on the
 * right Call, and emits domain call events. The only class that touches
 * android.telecom.Call directly.
 */
package com.tandem.gateway.telecom

import android.telecom.Call as TelecomCall
import android.telecom.CallAudioState
import android.telecom.InCallService
import com.tandem.gateway.dialer.OutgoingCallPlacer
import com.tandem.gateway.domain.model.AudioRoute
import com.tandem.gateway.domain.model.Call
import com.tandem.gateway.domain.port.TelecomBridge
import com.tandem.gateway.domain.port.TelecomError
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

@Singleton
class TelecomBridgeImpl @Inject constructor(
    private val outgoingCallPlacer: OutgoingCallPlacer,
) : TelecomBridge {

    private val tracked = ConcurrentHashMap<String, TelecomCall>()
    private val idsByCall = ConcurrentHashMap<TelecomCall, String>()
    private val nextId = AtomicLong(1)
    private var inCallService: InCallService? = null

    private val _calls = MutableStateFlow<List<Call>>(emptyList())
    override val calls: StateFlow<List<Call>> = _calls.asStateFlow()

    private val _audioRoute = MutableStateFlow(AudioRoute.EARPIECE)
    override val audioRoute: StateFlow<AudioRoute> = _audioRoute.asStateFlow()

    private val _microphoneMuted = MutableStateFlow(false)
    override val microphoneMuted: StateFlow<Boolean> = _microphoneMuted.asStateFlow()

    /** The MAC of the Bluetooth device currently carrying audio, if any. */
    private val _btRouteAddress = MutableStateFlow("")
    val btRouteAddress: StateFlow<String> = _btRouteAddress.asStateFlow()

    fun onCallAdded(call: TelecomCall, service: InCallService) {
        inCallService = service
        val id = idsByCall.getOrPut(call) { "call-${nextId.getAndIncrement()}" }
        tracked[id] = call
        publish()
    }

    fun onCallChanged(@Suppress("UNUSED_PARAMETER") call: TelecomCall) {
        publish()
    }

    fun onCallRemoved(call: TelecomCall) {
        val id = idsByCall.remove(call)
        if (id != null) tracked.remove(id)
        publish()
    }

    fun onAudioStateChanged(audioState: CallAudioState) {
        _microphoneMuted.value = audioState.isMuted
        _audioRoute.value = when (audioState.route) {
            CallAudioState.ROUTE_SPEAKER -> AudioRoute.SPEAKER
            CallAudioState.ROUTE_WIRED_HEADSET -> AudioRoute.WIRED_HEADSET
            CallAudioState.ROUTE_BLUETOOTH -> AudioRoute.BLUETOOTH
            else -> AudioRoute.EARPIECE
        }
        _btRouteAddress.value = audioState.activeBluetoothDevice?.address.orEmpty()
    }

    /** Exposes the live service so the media provider can set routes. */
    fun currentService(): InCallService? = inCallService

    override suspend fun dial(number: String, simSlot: Int): Result<String> =
        outgoingCallPlacer.place(number, simSlot)

    override suspend fun answer(callId: String): Result<Unit> = withCall(callId, "answer") {
        it.answer(android.telecom.VideoProfile.STATE_AUDIO_ONLY)
    }

    override suspend fun reject(callId: String): Result<Unit> = withCall(callId, "reject") {
        it.reject(false, null)
    }

    override suspend fun disconnect(callId: String): Result<Unit> = withCall(callId, "end") {
        it.disconnect()
    }

    override suspend fun hold(callId: String): Result<Unit> = withCall(callId, "hold") {
        it.hold()
    }

    override suspend fun unhold(callId: String): Result<Unit> = withCall(callId, "unhold") {
        it.unhold()
    }

    override suspend fun merge(callId: String, otherCallId: String): Result<Unit> =
        withCall(callId, "merge") { call ->
            val other = tracked[otherCallId]
                ?: throw TelecomError.CallNotFound(otherCallId)
            call.conference(other)
        }

    override suspend fun setMuted(muted: Boolean): Result<Unit> {
        val service = inCallService ?: return Result.failure(TelecomError.DialerRoleMissing)
        return runCatching { service.setMuted(muted) }
    }

    override suspend fun sendDtmf(callId: String, digits: String): Result<Unit> =
        withCall(callId, "dtmf") { call ->
            digits.forEach { digit ->
                call.playDtmfTone(digit)
                call.stopDtmfTone()
            }
        }

    /**
     * A typed failure raised by the action keeps its own meaning; only an
     * unexpected framework throw becomes InvalidCallState, so a missing call is
     * never reported as a bad state.
     */
    private inline fun withCall(
        callId: String,
        command: String,
        action: (TelecomCall) -> Unit,
    ): Result<Unit> {
        val call = tracked[callId] ?: return Result.failure(TelecomError.CallNotFound(callId))
        return runCatching { action(call) }.recoverCatching { cause ->
            throw if (cause is TelecomError) cause else TelecomError.InvalidCallState(callId, command)
        }
    }

    /**
     * Emergency numbers for the current SIM, cached because call mapping is
     * synchronous. Telecom's network-identified property is authoritative when
     * present but is not always set, so the number is checked too — a call
     * wrongly treated as ordinary would be remotely controllable (ADR-0008).
     */
    @Volatile
    private var emergencyNumbers: List<String> = emptyList()

    fun setEmergencyNumbers(numbers: List<String>) {
        emergencyNumbers = numbers.map(::normalizeDialString)
        publish()
    }

    private fun isEmergencyNumber(number: String?): Boolean {
        val normalized = normalizeDialString(number.orEmpty())
        return normalized.isNotEmpty() && normalized in emergencyNumbers
    }

    private fun normalizeDialString(value: String): String =
        value.filter { it.isDigit() || it == '*' || it == '#' }

    private fun publish() {
        _calls.value = tracked.entries.map { (id, call) -> toDomain(id, call) }
    }

    private fun toDomain(id: String, call: TelecomCall): Call {
        val details = call.details
        return Call(
            callId = id,
            state = CallStateMapper.mapState(call.state),
            direction = CallStateMapper.mapDirection(details.callDirection),
            remoteNumber = details.handle?.schemeSpecificPart.orEmpty(),
            remoteDisplayName = details.callerDisplayName.orEmpty(),
            startedAtMs = details.connectTimeMillis,
            isConference = CallStateMapper.isConference(details.callProperties),
            canHold = CallStateMapper.canHold(details.callCapabilities),
            canMerge = CallStateMapper.canMerge(details.callCapabilities),
            isEmergency = CallStateMapper.isEmergency(details.callProperties) ||
                isEmergencyNumber(details.handle?.schemeSpecificPart),
            disconnectCause = CallStateMapper.mapDisconnectCause(details.disconnectCause),
            simSlot = -1,
        )
    }
}
