/**
 * Compose screen showing gateway health: a headline state, the setup steps still
 * outstanding, LAN listener and connected-desktop detail, BT audio state, and the
 * emergency-policy notice. Renders StatusViewModel state; no logic.
 */
package com.tandem.gateway.ui.status

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.tandem.gateway.R
import com.tandem.gateway.ui.components.DetailRow
import com.tandem.gateway.ui.components.SectionCard
import com.tandem.gateway.ui.components.StatusPill
import com.tandem.gateway.ui.components.TandemScreen

@Composable
fun StatusScreen(
    onOpenPairing: () -> Unit,
    onOpenSettings: () -> Unit,
    onOpenDialpad: () -> Unit,
    onBack: () -> Unit = onOpenDialpad,
    viewModel: StatusViewModel = hiltViewModel(),
) {
    val state by viewModel.uiState.collectAsStateWithLifecycle()
    val context = LocalContext.current

    // Ready means a computer could take control right now; anything less is a
    // setup step, and the screen leads with whichever it is.
    val ready = state.isDefaultDialer && state.listening
    val connected = state.connectedDesktops > 0

    TandemScreen(
        title = "Tandem",
        eyebrow = "Connect",
        onBack = onBack,
        trailing = {
            StatusPill(
                text = if (ready) "Ready" else "Setup",
                healthy = ready,
            )
        },
    ) {
        SectionCard(accented = ready) {
            Text(
                text = when {
                    connected -> stringResource(
                        R.string.status_connected_desktops,
                        state.connectedDesktops,
                    )
                    ready -> "Waiting for a computer"
                    else -> "Finish setup to begin"
                },
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.SemiBold,
            )
            Text(
                text = when {
                    connected -> "Your computer can place and control calls on this phone."
                    ready -> "Pair a computer to place and control calls from it."
                    else -> "Tandem needs to be your phone app before it can handle calls."
                },
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )

            if (!state.isDefaultDialer) {
                Button(
                    onClick = { viewModel.roleRequestIntent()?.let(context::startActivity) },
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text(stringResource(R.string.status_dialer_role_action))
                }
            }
        }

        SectionCard {
            Text(
                text = "This phone",
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            DetailRow(
                label = "Phone app",
                value = if (state.isDefaultDialer) "Tandem" else "Not Tandem",
                emphasis = state.isDefaultDialer,
            )
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            DetailRow(
                label = "Network",
                value = if (state.listening) "Listening · port ${state.port}" else "Not listening",
                emphasis = state.listening,
            )
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            DetailRow(
                label = "Computers",
                value = if (connected) "${state.connectedDesktops} connected" else "None",
                emphasis = connected,
            )
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            DetailRow(label = "Call audio", value = routeLabel(state.audioRoute.name))
        }

        if (state.hasActiveEmergency) {
            Surface(
                shape = MaterialTheme.shapes.medium,
                color = MaterialTheme.colorScheme.errorContainer,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(
                    text = stringResource(R.string.emergency_read_only),
                    modifier = Modifier.padding(14.dp),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onErrorContainer,
                )
            }
        }

        Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            FilledTonalButton(onClick = onOpenPairing, modifier = Modifier.fillMaxWidth()) {
                Text(stringResource(R.string.pairing_title))
            }
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                OutlinedButton(onClick = onOpenDialpad, modifier = Modifier.weight(1f)) {
                    Text(stringResource(R.string.dialpad_title))
                }
                OutlinedButton(onClick = onOpenSettings, modifier = Modifier.weight(1f)) {
                    Text(stringResource(R.string.settings_title))
                }
            }
        }

        Text(
            text = stringResource(R.string.emergency_policy_notice),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )

        Spacer(Modifier.height(4.dp))
    }
}

/** The enum name is a wire detail; the user reads the device. */
private fun routeLabel(route: String): String = when (route) {
    "SPEAKER" -> "Phone speaker"
    "WIRED_HEADSET" -> "Wired headset"
    "BLUETOOTH" -> "Bluetooth"
    else -> "Phone earpiece"
}
