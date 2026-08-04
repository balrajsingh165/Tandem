/**
 * BroadcastReceiver for BOOT_COMPLETED (RECEIVE_BOOT_COMPLETED): starts
 * GatewayForegroundService when the user has opted into autostart in settings.
 * Disabled by default.
 */
package com.tandem.gateway.service

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import com.tandem.gateway.domain.port.PairedDeviceRepository
import com.tandem.gateway.domain.port.SettingsRepository
import dagger.hilt.android.AndroidEntryPoint
import javax.inject.Inject
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

@AndroidEntryPoint
class BootCompletedReceiver : BroadcastReceiver() {

    @Inject lateinit var settingsRepository: SettingsRepository
    @Inject lateinit var pairedDeviceRepository: PairedDeviceRepository

    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != Intent.ACTION_BOOT_COMPLETED) return

        val pendingResult = goAsync()
        CoroutineScope(SupervisorJob()).launch {
            try {
                // Autostart is opt-in, and pointless with nothing paired: starting
                // a foreground service nobody can use is just a battery cost.
                val enabled = settingsRepository.autostartEnabled.first()
                val hasPairedDesktop = pairedDeviceRepository.devices.first()
                    .any { !it.revoked }

                if (enabled && hasPairedDesktop) {
                    context.startForegroundService(
                        Intent(context, GatewayForegroundService::class.java),
                    )
                }
            } finally {
                pendingResult.finish()
            }
        }
    }
}
