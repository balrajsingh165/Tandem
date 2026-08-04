/**
 * Application root for the Tandem Gateway. Hosts the Hilt component graph and
 * schedules GatewayForegroundService startup when a paired desktop exists. No
 * business logic; wiring only.
 */
package com.tandem.gateway

import android.app.Application
import android.content.Intent
import com.tandem.gateway.di.ApplicationScope
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

    @Inject
    @ApplicationScope
    lateinit var applicationScope: CoroutineScope

    override fun onCreate() {
        super.onCreate()
        startGatewayIfPaired()
    }

    /**
     * Starting the service before anything is paired would show a permanent
     * notification for a gateway nobody can reach.
     */
    private fun startGatewayIfPaired() {
        applicationScope.launch {
            val hasPairedDesktop = pairedDeviceRepository.devices.first().any { !it.revoked }
            if (hasPairedDesktop) {
                startForegroundService(Intent(this@TandemApplication, GatewayForegroundService::class.java))
            }
        }
    }
}
