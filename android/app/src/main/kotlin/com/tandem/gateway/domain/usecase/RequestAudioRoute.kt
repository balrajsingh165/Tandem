/**
 * Use-case: request an absolute audio route via CallMediaProvider, validating
 * that Bluetooth targets are bonded and that no emergency call is active. The LAN
 * triggers routing; HFP carries the audio (docs/05).
 */
package com.tandem.gateway.domain.usecase

import com.tandem.gateway.domain.model.AudioRoute
import com.tandem.gateway.domain.model.AudioRouteTarget
import com.tandem.gateway.domain.model.CallSnapshot
import com.tandem.gateway.domain.port.CallMediaProvider
import com.tandem.gateway.domain.port.MediaRouteError
import javax.inject.Inject

class RequestAudioRoute @Inject constructor(
    private val callMediaProvider: CallMediaProvider,
    private val guardEmergencyNumber: GuardEmergencyNumber,
) {
    suspend operator fun invoke(
        target: AudioRouteTarget,
        snapshot: CallSnapshot?,
    ): Result<Unit> {
        if (!guardEmergencyNumber.guardRemoteControl(snapshot).isAllowed) {
            return Result.failure(MediaRouteError.EmergencyCallActive)
        }

        if (target.route == AudioRoute.BLUETOOTH) {
            if (target.btDeviceAddress.isEmpty()) {
                return Result.failure(MediaRouteError.RouteUnavailable)
            }
            val bonded = callMediaProvider.availableBluetoothTargets()
                .any { it.address.equals(target.btDeviceAddress, ignoreCase = true) }
            if (!bonded) {
                return Result.failure(MediaRouteError.DeviceNotBonded(target.btDeviceAddress))
            }
        }

        return callMediaProvider.requestRoute(target)
    }
}
