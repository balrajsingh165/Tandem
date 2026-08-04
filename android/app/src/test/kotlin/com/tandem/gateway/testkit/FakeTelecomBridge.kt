/**
 * In-memory TelecomBridge fake: tests script call arrivals and state transitions
 * and assert on received commands. Backs use-case and router unit tests without
 * Android Telecom.
 */
package com.tandem.gateway.testkit

import com.tandem.gateway.domain.model.AudioRoute
import com.tandem.gateway.domain.model.Call
import com.tandem.gateway.domain.model.CallDirection
import com.tandem.gateway.domain.model.CallState
import com.tandem.gateway.domain.model.DisconnectCause
import com.tandem.gateway.domain.port.TelecomBridge
import com.tandem.gateway.domain.port.TelecomError
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class FakeTelecomBridge : TelecomBridge {

    private val _calls = MutableStateFlow<List<Call>>(emptyList())
    override val calls: StateFlow<List<Call>> = _calls

    private val _audioRoute = MutableStateFlow(AudioRoute.EARPIECE)
    override val audioRoute: StateFlow<AudioRoute> = _audioRoute

    private val _microphoneMuted = MutableStateFlow(false)
    override val microphoneMuted: StateFlow<Boolean> = _microphoneMuted

    /** Every command the subject issued, in order, for assertion. */
    val commands = mutableListOf<String>()

    /** Set to make the next command fail, exercising error paths. */
    var nextFailure: Throwable? = null

    fun addCall(
        callId: String,
        state: CallState = CallState.RINGING,
        isEmergency: Boolean = false,
        canHold: Boolean = true,
        canMerge: Boolean = false,
    ) {
        _calls.value = _calls.value + Call(
            callId = callId,
            state = state,
            direction = CallDirection.INCOMING,
            remoteNumber = "+14155550123",
            remoteDisplayName = "Alex",
            startedAtMs = 1_700_000_000_000,
            isConference = false,
            canHold = canHold,
            canMerge = canMerge,
            isEmergency = isEmergency,
            disconnectCause = DisconnectCause.UNSPECIFIED,
            simSlot = 0,
        )
    }

    fun transition(callId: String, state: CallState) {
        _calls.value = _calls.value.map { if (it.callId == callId) it.copy(state = state) else it }
    }

    fun setRoute(route: AudioRoute) {
        _audioRoute.value = route
    }

    override suspend fun dial(number: String, simSlot: Int): Result<String> =
        record("dial:$number") { number }

    override suspend fun answer(callId: String): Result<Unit> = record("answer:$callId") { }

    override suspend fun reject(callId: String): Result<Unit> = record("reject:$callId") { }

    override suspend fun disconnect(callId: String): Result<Unit> = record("end:$callId") { }

    override suspend fun hold(callId: String): Result<Unit> = record("hold:$callId") { }

    override suspend fun unhold(callId: String): Result<Unit> = record("unhold:$callId") { }

    override suspend fun merge(callId: String, otherCallId: String): Result<Unit> =
        record("merge:$callId+$otherCallId") { }

    override suspend fun setMuted(muted: Boolean): Result<Unit> = record("mute:$muted") {
        _microphoneMuted.value = muted
    }

    override suspend fun sendDtmf(callId: String, digits: String): Result<Unit> =
        record("dtmf:$callId:$digits") { }

    private fun <T> record(command: String, action: () -> T): Result<T> {
        commands += command
        nextFailure?.let {
            nextFailure = null
            return Result.failure(it)
        }
        val known = _calls.value.any { command.substringAfter(':').startsWith(it.callId) }
        if (command.contains(':') && command.startsWith("answer") && !known) {
            return Result.failure(TelecomError.CallNotFound(command.substringAfter(':')))
        }
        return Result.success(action())
    }
}
