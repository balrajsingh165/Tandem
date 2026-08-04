/**
 * Port over call-audio routing: request/observe the active audio route,
 * including routing to a specific Bluetooth device. Implemented today by
 * HfpCallMediaProvider [Tier A/B]; a Tier C vendor backend would implement the
 * same port (ADR-0010).
 */
package com.tandem.gateway.domain.port

import com.tandem.gateway.domain.model.AudioRouteTarget
import kotlinx.coroutines.flow.Flow

interface CallMediaProvider {
    /** The route the phone is actually using, as reported by the OS. */
    val currentRoute: Flow<AudioRouteTarget>

    /** False when no media backend can carry audio to a desktop [Tier B-lite fallback]. */
    fun supportsDesktopAudio(): Boolean

    suspend fun requestRoute(target: AudioRouteTarget): Result<Unit>

    /** Bonded devices eligible as an audio target, for UX and validation. */
    suspend fun availableBluetoothTargets(): List<BluetoothTarget>
}

data class BluetoothTarget(
    val address: String,
    val name: String,
    val connected: Boolean,
)

/**
 * Typed failures at the media boundary. Losing audio is never fatal to a call:
 * the phone falls back to its earpiece and the call continues (docs/05).
 */
sealed class MediaRouteError(message: String) : Exception(message) {
    data object NoActiveCall : MediaRouteError("no call is active to route")

    data class DeviceNotBonded(val address: String) :
        MediaRouteError("device $address is not bonded to this phone")

    data object BluetoothPermissionMissing :
        MediaRouteError("BLUETOOTH_CONNECT is required to select a Bluetooth target")

    data object RouteUnavailable : MediaRouteError("the requested route is not available")

    data object EmergencyCallActive :
        MediaRouteError("audio routing is refused while an emergency call is active")
}
