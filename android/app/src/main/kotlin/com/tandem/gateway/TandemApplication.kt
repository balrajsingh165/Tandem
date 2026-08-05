/**
 * Application root for the Tandem Gateway. Hosts the Hilt component graph and
 * schedules GatewayForegroundService startup when a paired desktop exists. No
 * business logic; wiring only.
 */
package com.tandem.gateway

import android.app.Application
import android.content.Intent
import com.tandem.gateway.dialer.DefaultDialerManager
import com.tandem.gateway.domain.port.PairedDeviceRepository
import com.tandem.gateway.service.GatewayForegroundService
import dagger.hilt.android.HiltAndroidApp
import javax.inject.Inject
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

@HiltAndroidApp
class TandemApplication : Application() {

    @Inject lateinit var pairedDeviceRepository: PairedDeviceRepository

    @Inject lateinit var defaultDialerManager: DefaultDialerManager

    // Unqualified on purpose: a qualifier on a Kotlin `lateinit` field lands on
    // the property rather than the field, which Dagger does not read.
    @Inject lateinit var applicationScope: CoroutineScope

    override fun onCreate() {
        super.onCreate()
        startGatewayIfUsable()
    }

    /**
     * The gateway runs whenever Tandem is the phone app, not only once a desktop
     * is paired: the LAN listener is what a desktop connects to in order to pair
     * in the first place, so gating it on an existing pairing would make first
     * pairing impossible.
     */
    private fun startGatewayIfUsable() {
        applicationScope.launch {
            val hasPairedDesktop = pairedDeviceRepository.devices.first().any { !it.revoked }
            if (defaultDialerManager.isDefaultDialer() || hasPairedDesktop) {
                startForegroundService(
                    Intent(this@TandemApplication, GatewayForegroundService::class.java),
                )
            }
        }
    }
}
