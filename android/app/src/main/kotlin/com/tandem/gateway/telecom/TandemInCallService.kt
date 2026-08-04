/**
 * android.telecom.InCallService implementation: receives Call objects and
 * audio-state callbacks while Tandem is the default dialer, forwards them to
 * TelecomBridgeImpl, and launches the handset in-call UI. No business logic in
 * callbacks (docs/14 layering rule).
 */
package com.tandem.gateway.telecom

import android.content.Intent
import android.telecom.Call
import android.telecom.CallAudioState
import android.telecom.InCallService
import com.tandem.gateway.ui.incall.InCallActivity
import com.tandem.gateway.ui.incall.IncomingCallNotifier
import dagger.hilt.android.AndroidEntryPoint
import javax.inject.Inject

@AndroidEntryPoint
class TandemInCallService : InCallService() {

    @Inject
    lateinit var telecomBridge: TelecomBridgeImpl

    @Inject
    lateinit var incomingCallNotifier: IncomingCallNotifier

    private val callCallback = object : Call.Callback() {
        override fun onStateChanged(call: Call, state: Int) {
            telecomBridge.onCallChanged(call)
        }

        override fun onDetailsChanged(call: Call, details: Call.Details) {
            telecomBridge.onCallChanged(call)
        }

        override fun onCannotHandleCall(call: Call) {
            telecomBridge.onCallChanged(call)
        }
    }

    override fun onCallAdded(call: Call) {
        call.registerCallback(callCallback)
        telecomBridge.onCallAdded(call, this)

        if (call.state == Call.STATE_RINGING) {
            incomingCallNotifier.notifyRinging(call)
        } else {
            startActivity(
                Intent(this, InCallActivity::class.java)
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            )
        }
    }

    override fun onCallRemoved(call: Call) {
        call.unregisterCallback(callCallback)
        telecomBridge.onCallRemoved(call)
        incomingCallNotifier.cancelRinging()
    }

    override fun onCallAudioStateChanged(audioState: CallAudioState) {
        telecomBridge.onAudioStateChanged(audioState)
    }

    override fun onSilenceRinger() {
        incomingCallNotifier.silence()
    }
}
