/**
 * Builds the persistent gateway status notification and its channel set
 * (POST_NOTIFICATIONS) showing connected desktops and audio-route state.
 * Incoming-call notifications live in IncomingCallNotifier, not here.
 */
package com.tandem.gateway.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.app.PendingIntent
import com.tandem.gateway.R
import com.tandem.gateway.domain.port.SessionInfo
import com.tandem.gateway.ui.MainActivity
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class GatewayNotifications @Inject constructor(
    @ApplicationContext private val context: Context,
) {
    private val notificationManager: NotificationManager?
        get() = context.getSystemService(NotificationManager::class.java)

    fun ensureChannels() {
        val manager = notificationManager ?: return

        manager.createNotificationChannel(
            NotificationChannel(
                CHANNEL_GATEWAY,
                context.getString(R.string.notification_channel_gateway),
                NotificationManager.IMPORTANCE_LOW,
            ),
        )

        // Calls ride a high-importance channel so the incoming-call full-screen
        // intent is honored.
        manager.createNotificationChannel(
            NotificationChannel(
                CHANNEL_CALLS,
                context.getString(R.string.notification_channel_calls),
                NotificationManager.IMPORTANCE_HIGH,
            ),
        )
    }

    /**
     * Names the computer in control rather than announcing that the app is running:
     * "Tandem is running" tells the user nothing they cannot see from the icon, while
     * which machine can dial from their SIM is worth a permanent notification. A
     * connected session also gets a Disconnect action, since the phone must always be
     * able to take back control without opening anything.
     */
    fun buildGatewayNotification(sessions: List<SessionInfo>): Notification {
        val connected = sessions.firstOrNull()

        val builder = Notification.Builder(context, CHANNEL_GATEWAY)
            .setSmallIcon(android.R.drawable.stat_sys_phone_call)
            .setOngoing(true)
            .setContentIntent(
                PendingIntent.getActivity(
                    context,
                    0,
                    Intent(context, MainActivity::class.java),
                    PendingIntent.FLAG_IMMUTABLE,
                ),
            )

        if (connected == null) {
            builder
                .setContentTitle(context.getString(R.string.notification_waiting_title))
                .setContentText(context.getString(R.string.notification_waiting_body))
            return builder.build()
        }

        val extra = sessions.size - 1
        builder
            .setContentTitle(
                if (extra > 0) {
                    context.getString(
                        R.string.notification_connected_title_multi,
                        connected.displayName,
                        extra,
                    )
                } else {
                    context.getString(R.string.notification_connected_title, connected.displayName)
                },
            )
            .setContentText(context.getString(R.string.notification_connected_body))
            .addAction(
                Notification.Action.Builder(
                    null,
                    context.getString(R.string.notification_disconnect),
                    PendingIntent.getService(
                        context,
                        1,
                        Intent(context, GatewayForegroundService::class.java)
                            .setAction(GatewayForegroundService.ACTION_DISCONNECT)
                            .putExtra(GatewayForegroundService.EXTRA_DEVICE_ID, connected.deviceId),
                        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
                    ),
                ).build(),
            )

        return builder.build()
    }

    companion object {
        const val CHANNEL_GATEWAY: String = "tandem.gateway"
        const val CHANNEL_CALLS: String = "tandem.calls"
        const val NOTIFICATION_ID_GATEWAY: Int = 1
        const val NOTIFICATION_ID_INCOMING: Int = 2
    }
}
