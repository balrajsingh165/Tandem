/**
 * In-memory CallMediaProvider fake: records route requests and lets tests
 * simulate route changes and SCO drops, including the fall-back-to-earpiece path.
 */
package com.tandem.gateway.testkit

import com.tandem.gateway.domain.model.AudioRoute
import com.tandem.gateway.domain.model.AudioRouteTarget
import com.tandem.gateway.domain.port.BluetoothTarget
import com.tandem.gateway.domain.port.CallMediaProvider
import com.tandem.gateway.domain.port.MediaRouteError
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class FakeCallMediaProvider : CallMediaProvider {

    private val _currentRoute = MutableStateFlow(AudioRouteTarget(AudioRoute.EARPIECE))
    override val currentRoute: StateFlow<AudioRouteTarget> = _currentRoute

    val requestedRoutes = mutableListOf<AudioRouteTarget>()

    var bondedTargets: List<BluetoothTarget> = emptyList()
    var desktopAudioSupported: Boolean = true
    var nextFailure: Throwable? = null

    override fun supportsDesktopAudio(): Boolean = desktopAudioSupported

    override suspend fun requestRoute(target: AudioRouteTarget): Result<Unit> {
        requestedRoutes += target
        nextFailure?.let {
            nextFailure = null
            return Result.failure(it)
        }
        if (target.route == AudioRoute.BLUETOOTH &&
            bondedTargets.none { it.address.equals(target.btDeviceAddress, true) }
        ) {
            return Result.failure(MediaRouteError.DeviceNotBonded(target.btDeviceAddress))
        }
        _currentRoute.value = target
        return Result.success(Unit)
    }

    override suspend fun availableBluetoothTargets(): List<BluetoothTarget> = bondedTargets

    /**
     * Simulates SCO dropping mid-call. The route falls back to the earpiece and
     * the call itself is untouched (docs/05).
     */
    fun simulateScoDrop() {
        _currentRoute.value = AudioRouteTarget(AudioRoute.EARPIECE)
    }
}
