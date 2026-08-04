/**
 * Use-case: merge TelecomBridge call events, CallMediaProvider route changes, and
 * mute state into the versioned CallSnapshot stream (epoch_id, state_seq) that
 * feeds every desktop session and the handset UI alike.
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.port.CallMediaProvider
import com.tandem.gateway.domain.port.TelecomBridge
import java.util.UUID
import java.util.concurrent.atomic.AtomicLong
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map

/**
 * The epoch is minted once per gateway process; every desktop that sees a new
 * epoch discards its mirror wholesale, because sequence numbers from a previous
 * process mean nothing (ADR-0007).
 */
@Singleton
class ObserveCallState @Inject constructor(
    private val telecomBridge: TelecomBridge,
    private val callMediaProvider: CallMediaProvider,
) {
    private val epochId: String = UUID.randomUUID().toString()
    private val stateSeq = AtomicLong(0)

    fun currentEpochId(): String = epochId

    fun currentStateSeq(): Long = stateSeq.get()

    operator fun invoke(): Flow<CallSnapshot> =
        combine(
            telecomBridge.calls,
            callMediaProvider.currentRoute,
            telecomBridge.microphoneMuted,
        ) { calls, route, muted ->
            Triple(calls, route, muted)
        }
            .distinctUntilChanged()
            .map { (calls, route, muted) ->
                CallSnapshot(
                    epochId = epochId,
                    stateSeq = stateSeq.incrementAndGet(),
                    calls = calls,
                    audioRoute = route.route,
                    microphoneMuted = muted,
                    btRouteAddress = route.btDeviceAddress,
                )
            }
}
