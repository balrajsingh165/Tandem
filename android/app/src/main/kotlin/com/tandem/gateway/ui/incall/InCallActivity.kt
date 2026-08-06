/**
 * Activity shown over the lock screen for active calls (launched by
 * TandemInCallService and IncomingCallNotifier full-screen intent). Hosts
 * InCallScreen; window flags only, no call logic.
 */
package com.tandem.gateway.ui.incall

import android.app.KeyguardManager
import android.os.Build
import android.os.Bundle
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import com.tandem.gateway.ui.theme.TandemTheme
import dagger.hilt.android.AndroidEntryPoint

@AndroidEntryPoint
class InCallActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        showOverLockScreen()

        setContent {
            TandemTheme {
                // The call screen owns no lifecycle; when the last call goes away
                // this activity has to leave with it rather than sitting on a
                // "Call ended" message the user must dismiss.
                InCallScreen(onCallsEnded = { finishAndRemoveTask() })
            }
        }
    }

    /**
     * An incoming call must be answerable without unlocking, which is why this
     * activity is the only surface allowed a full-screen intent (docs/12).
     */
    private fun showOverLockScreen() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) {
            setShowWhenLocked(true)
            setTurnScreenOn(true)
            getSystemService(KeyguardManager::class.java)?.requestDismissKeyguard(this, null)
        } else {
            @Suppress("DEPRECATION")
            window.addFlags(
                WindowManager.LayoutParams.FLAG_SHOW_WHEN_LOCKED or
                    WindowManager.LayoutParams.FLAG_TURN_SCREEN_ON,
            )
        }
    }
}
