/**
 * Posts the incoming-call notification (USE_FULL_SCREEN_INTENT +
 * POST_NOTIFICATIONS) with answer/decline actions and launches InCallActivity
 * when ringing. The only surface allowed to use a full-screen intent.
 */
package com.tandem.gateway.ui.incall

import android.app.Notification
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.telecom.Call
import com.tandem.gateway.R
import com.tandem.gateway.service.GatewayNotifications
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class IncomingCallNotifier @Inject constructor(
    @ApplicationContext private val context: Context,
    private val notifications: GatewayNotifications,
) {
    private val notificationManager: NotificationManager?
        get() = context.getSystemService(NotificationManager::class.java)

    fun notifyRinging(call: Call) {
        notifications.ensureChannels()
        val manager = notificationManager ?: return

        val caller = call.details?.handle?.schemeSpecificPart
            ?: context.getString(R.string.call_unknown_number)

        val fullScreenIntent = PendingIntent.getActivity(
            context,
            0,
            Intent(context, InCallActivity::class.java)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )

        val notification = Notification.Builder(context, GatewayNotifications.CHANNEL_CALLS)
            .setContentTitle(context.getString(R.string.call_incoming))
            .setContentText(caller)
            .setSmallIcon(android.R.drawable.stat_sys_phone_call)
            .setCategory(Notification.CATEGORY_CALL)
            .setOngoing(true)
            .setFullScreenIntent(fullScreenIntent, true)
            .build()

        manager.notify(GatewayNotifications.NOTIFICATION_ID_INCOMING, notification)
    }

    fun cancelRinging() {
        notificationManager?.cancel(GatewayNotifications.NOTIFICATION_ID_INCOMING)
    }

    /** Telecom asked for silence without cancelling; keep the notification. */
    fun silence() {
        notificationManager?.cancel(GatewayNotifications.NOTIFICATION_ID_INCOMING)
    }
}
