/**
 * Launcher activity hosting the Compose navigation graph (status, pairing,
 * settings, dialpad). Receives DialIntentRouter forwards; holds no state beyond
 * navigation.
 */
package com.tandem.gateway.ui

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import com.tandem.gateway.dialer.DialIntentRouter
import com.tandem.gateway.ui.home.HomeScreen
import com.tandem.gateway.ui.pairing.PairingScreen
import com.tandem.gateway.ui.settings.SettingsScreen
import com.tandem.gateway.ui.status.StatusScreen
import com.tandem.gateway.ui.theme.TandemTheme
import dagger.hilt.android.AndroidEntryPoint
import javax.inject.Inject

@AndroidEntryPoint
class MainActivity : ComponentActivity() {

    @Inject lateinit var dialIntentRouter: DialIntentRouter

    private var prefilledNumber: String? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        prefilledNumber = dialIntentRouter.prefillNumber(intent)

        setContent {
            TandemTheme {
                // A dialer's home is the dialer. Tandem's own features live behind
                // the menu, so the app reads as a phone app first.
                var destination by remember { mutableStateOf(Destination.DIALPAD) }

                when (destination) {
                    Destination.STATUS -> StatusScreen(
                        onOpenPairing = { destination = Destination.PAIRING },
                        onOpenSettings = { destination = Destination.SETTINGS },
                        onOpenDialpad = { destination = Destination.DIALPAD },
                    )

                    Destination.PAIRING -> PairingScreen(
                        onBack = { destination = Destination.STATUS },
                    )

                    Destination.SETTINGS -> SettingsScreen(
                        onBack = { destination = Destination.STATUS },
                    )

                    Destination.DIALPAD -> HomeScreen(
                        initialNumber = prefilledNumber.orEmpty(),
                        onOpenConnect = { destination = Destination.STATUS },
                    )
                }
            }
        }
    }

    /** A tel: intent arriving while the app is open must still prefill. */
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        prefilledNumber = dialIntentRouter.prefillNumber(intent)
    }

    private enum class Destination { STATUS, PAIRING, SETTINGS, DIALPAD }
}
