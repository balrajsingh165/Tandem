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

    fun buildGatewayNotification(connectedDesktops: Int): Notification {
        val content = if (connectedDesktops > 0) {
            context.getString(R.string.status_connected_desktops, connectedDesktops)
        } else {
            context.getString(R.string.notification_gateway_body)
        }

        return Notification.Builder(context, CHANNEL_GATEWAY)
            .setContentTitle(context.getString(R.string.notification_gateway_title))
            .setContentText(content)
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
            .build()
    }

    companion object {
        const val CHANNEL_GATEWAY: String = "tandem.gateway"
        const val CHANNEL_CALLS: String = "tandem.calls"
        const val NOTIFICATION_ID_GATEWAY: Int = 1
        const val NOTIFICATION_ID_INCOMING: Int = 2
    }
}
