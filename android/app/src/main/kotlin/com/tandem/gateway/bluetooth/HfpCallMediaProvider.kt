/**
 * CallMediaProvider implementation: executes AudioRouteRequest by calling
 * InCallService.setAudioRoute / requestBluetoothAudio toward the desktop's bonded
 * HF device and reports route reality from CallAudioState callbacks. Falls back
 * to earpiece automatically if SCO drops — the call itself is never touched
 * (docs/05).
 */
package com.tandem.gateway.bluetooth

import android.telecom.CallAudioState
import com.tandem.gateway.domain.model.AudioRoute
import com.tandem.gateway.domain.model.AudioRouteTarget
import com.tandem.gateway.domain.port.BluetoothTarget
import com.tandem.gateway.domain.port.CallMediaProvider
import com.tandem.gateway.domain.port.MediaRouteError
import com.tandem.gateway.telecom.TelecomBridgeImpl
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.map

@Singleton
class HfpCallMediaProvider @Inject constructor(
    private val telecomBridge: TelecomBridgeImpl,
    private val hfpAgMonitor: HfpAgMonitor,
) : CallMediaProvider {

    override val currentRoute: Flow<AudioRouteTarget> =
        combine(telecomBridge.audioRoute, telecomBridge.btRouteAddress) { route, address ->
            AudioRouteTarget(route = route, btDeviceAddress = address)
        }

    /**
     * True whenever the phone can route to some Bluetooth device. Whether a
     * desktop can *receive* that audio is a desktop-side capability (Tier B); a
     * commodity headset works here too (Tier B-lite).
     */
    override fun supportsDesktopAudio(): Boolean = hfpAgMonitor.connectedHeadsets.value.isNotEmpty()

    @Suppress("MissingPermission")
    override suspend fun requestRoute(target: AudioRouteTarget): Result<Unit> {
        val service = telecomBridge.currentService()
            ?: return Result.failure(MediaRouteError.NoActiveCall)

        return runCatching {
            when (target.route) {
                AudioRoute.BLUETOOTH -> {
                    val device = hfpAgMonitor.connectedHeadsets.value
                        .firstOrNull { it.address.equals(target.btDeviceAddress, true) }
                        ?: throw MediaRouteError.DeviceNotBonded(target.btDeviceAddress)
                    service.requestBluetoothAudio(device)
                }

                AudioRoute.SPEAKER -> service.setAudioRoute(CallAudioState.ROUTE_SPEAKER)
                AudioRoute.WIRED_HEADSET -> service.setAudioRoute(CallAudioState.ROUTE_WIRED_HEADSET)
                AudioRoute.EARPIECE -> service.setAudioRoute(CallAudioState.ROUTE_EARPIECE)
            }
        }
    }

    @Suppress("MissingPermission")
    override suspend fun availableBluetoothTargets(): List<BluetoothTarget> =
        hfpAgMonitor.connectedHeadsets.value.map { device ->
            BluetoothTarget(
                address = device.address,
                name = runCatching { device.name }.getOrDefault(device.address),
                connected = true,
            )
        }

    /** The route as the OS reports it, which is the only authority (docs/05). */
    val observedRoute: Flow<AudioRoute> = currentRoute.map { it.route }
}
