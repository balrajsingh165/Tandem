/**
 * Domain model of the phone's call-audio route (earpiece, speaker, wired,
 * Bluetooth with device address). Mirrors android.telecom.CallAudioState routes
 * without framework types.
 */
package com.tandem.gateway.domain.model

enum class AudioRoute {
    EARPIECE,
    SPEAKER,
    WIRED_HEADSET,
    BLUETOOTH,
    ;

    /** What a desktop shows for this route when no device name applies. */
    fun label(): String = when (this) {
        EARPIECE -> "Phone earpiece"
        SPEAKER -> "Phone speaker"
        WIRED_HEADSET -> "Wired headset"
        BLUETOOTH -> "Bluetooth"
    }
}

/**
 * A requested or reported route. The address is meaningful only for BLUETOOTH,
 * where it names which bonded device should carry the audio.
 */
data class AudioRouteTarget(
    val route: AudioRoute,
    val btDeviceAddress: String = "",
) {
    val requiresBluetoothDevice: Boolean
        get() = route == AudioRoute.BLUETOOTH
}
