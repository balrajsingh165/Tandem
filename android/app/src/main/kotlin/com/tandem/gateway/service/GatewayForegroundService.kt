/**
 * Foreground service (types phoneCall|connectedDevice) keeping the LAN server,
 * NSD advertisement, and telecom observation alive; legal for phoneCall type
 * because Tandem holds ROLE_DIALER. Doze/battery behavior documented in docs/02.
 */
package com.tandem.gateway.service

import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.IBinder
import com.tandem.gateway.bluetooth.HfpAgMonitor
import com.tandem.gateway.calllog.CallLogObserver
import com.tandem.gateway.domain.port.EmergencyNumberSource
import com.tandem.gateway.domain.port.IdentityStore
import com.tandem.gateway.domain.port.LanServer
import com.tandem.gateway.domain.port.SettingsRepository
import com.tandem.gateway.domain.usecase.ObserveCallState
import com.tandem.gateway.telecom.TelecomBridgeImpl
import com.tandem.gateway.transport.NsdAdvertiser
import dagger.hilt.android.AndroidEntryPoint
import javax.inject.Inject
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.launchIn
import kotlinx.coroutines.flow.onEach
import kotlinx.coroutines.launch

@AndroidEntryPoint
class GatewayForegroundService : Service() {

    @Inject lateinit var lanServer: LanServer
    @Inject lateinit var nsdAdvertiser: NsdAdvertiser
    @Inject lateinit var observeCallState: ObserveCallState
    @Inject lateinit var callLogObserver: CallLogObserver
    @Inject lateinit var hfpAgMonitor: HfpAgMonitor
    @Inject lateinit var identityStore: IdentityStore
    @Inject lateinit var settingsRepository: SettingsRepository
    @Inject lateinit var notifications: GatewayNotifications
    @Inject lateinit var emergencyNumberSource: EmergencyNumberSource
    @Inject lateinit var telecomBridge: TelecomBridgeImpl

    private val scope = CoroutineScope(SupervisorJob())
    private var running = false

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onCreate() {
        super.onCreate()
        notifications.ensureChannels()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (!running) {
            running = true
            promoteToForeground()
            startSubsystems()
        }
        // The gateway is only useful while running, so the OS should restart it.
        return START_STICKY
    }

    override fun onDestroy() {
        scope.cancel()
        callLogObserver.stop()
        hfpAgMonitor.stop()
        nsdAdvertiser.unregister()
        scope.launch { lanServer.stop() }
        running = false
        super.onDestroy()
    }

    private fun promoteToForeground() {
        startForeground(
            GatewayNotifications.NOTIFICATION_ID_GATEWAY,
            notifications.buildGatewayNotification(connectedDesktops = 0),
            ServiceInfo.FOREGROUND_SERVICE_TYPE_PHONE_CALL or
                ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE,
        )
    }

    private fun startSubsystems(): Job = scope.launch {
        val identity = identityStore.identity().getOrNull() ?: return@launch
        val port = settingsRepository.listenPort.first()

        // Loaded before any call can be observed, so an emergency call is flagged
        // read-only from the first snapshot rather than after a delay (ADR-0008).
        telecomBridge.setEmergencyNumbers(emergencyNumberSource.currentEmergencyNumbers())

        lanServer.start(port)
        nsdAdvertiser.register(port, identity.deviceId, identity.displayName)
        callLogObserver.start()
        hfpAgMonitor.start()

        // Phone truth flows outward: every snapshot fans out to every desktop.
        observeCallState()
            .onEach { snapshot -> lanServer.broadcastSnapshot(snapshot) }
            .launchIn(scope)

        callLogObserver.logVersion
            .onEach { version -> lanServer.broadcastCallLogChanged(version) }
            .launchIn(scope)

        lanServer.connectedSessions
            .onEach { sessions ->
                notifications.ensureChannels()
                startForeground(
                    GatewayNotifications.NOTIFICATION_ID_GATEWAY,
                    notifications.buildGatewayNotification(sessions.size),
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_PHONE_CALL or
                        ServiceInfo.FOREGROUND_SERVICE_TYPE_CONNECTED_DEVICE,
                )
            }
            .launchIn(scope)
    }
}
