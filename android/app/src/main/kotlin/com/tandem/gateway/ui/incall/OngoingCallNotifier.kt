/**
 * Posts the ongoing-call notification while a call is live, so leaving the app —
 * deliberately or not — still leaves a way back and a way to hang up. Uses
 * Notification.CallStyle where the platform has it (API 31+) so the system gives it
 * the same treatment as the stock dialer, and a plain notification with the same
 * actions below that.
 */
package com.tandem.gateway.ui.incall

import android.app.Notification
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Person
import android.content.Context
import android.content.Intent
import android.os.Build
import com.tandem.gateway.R
import com.tandem.gateway.domain.model.Call
import com.tandem.gateway.domain.model.CallState
import com.tandem.gateway.service.GatewayForegroundService
import com.tandem.gateway.service.GatewayNotifications
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class OngoingCallNotifier @Inject constructor(
    @ApplicationContext private val context: Context,
    private val notifications: GatewayNotifications,
) {
    private val notificationManager: NotificationManager?
        get() = context.getSystemService(NotificationManager::class.java)

    /** Posts or updates for [call]; a terminal call cancels instead. */
    fun notifyOngoing(call: Call) {
        if (call.state == CallState.DISCONNECTED || call.state == CallState.DISCONNECTING) {
            cancel()
            return
        }

        notifications.ensureChannels()
        val manager = notificationManager ?: return

        val who = call.remoteDisplayName.ifBlank {
            call.remoteNumber.ifBlank { context.getString(R.string.call_unknown_number) }
        }

        val builder = Notification.Builder(context, GatewayNotifications.CHANNEL_CALLS)
            .setSmallIcon(android.R.drawable.stat_sys_phone_call)
            .setCategory(Notification.CATEGORY_CALL)
            .setOngoing(true)
            // Tapping anywhere returns to the call rather than the dialer home.
            .setContentIntent(openCallIntent())

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            val person = Person.Builder().setName(who).setImportant(true).build()
            builder.setStyle(
                Notification.CallStyle.forOngoingCall(person, endCallIntent(call.callId)),
            )
        } else {
            builder
                .setContentTitle(who)
                .setContentText(context.getString(R.string.notification_call_ongoing))
                .addAction(
                    Notification.Action.Builder(
                        null,
                        context.getString(R.string.call_end),
                        endCallIntent(call.callId),
                    ).build(),
                )
        }

        // Timing the notification gives the shade a running duration for free.
        if (call.state == CallState.ACTIVE && call.startedAtMs > 0) {
            builder.setWhen(call.startedAtMs).setUsesChronometer(true)
        }

        manager.notify(GatewayNotifications.NOTIFICATION_ID_ONGOING, builder.build())
    }

    fun cancel() {
        notificationManager?.cancel(GatewayNotifications.NOTIFICATION_ID_ONGOING)
    }

    private fun openCallIntent(): PendingIntent = PendingIntent.getActivity(
        context,
        0,
        Intent(context, InCallActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )

    /**
     * Hanging up from the shade goes through the service, which owns the use-cases;
     * a notification action must not reach into Telecom directly.
     */
    private fun endCallIntent(callId: String): PendingIntent = PendingIntent.getService(
        context,
        2,
        Intent(context, GatewayForegroundService::class.java)
            .setAction(GatewayForegroundService.ACTION_END_CALL)
            .putExtra(GatewayForegroundService.EXTRA_CALL_ID, callId),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )
}
