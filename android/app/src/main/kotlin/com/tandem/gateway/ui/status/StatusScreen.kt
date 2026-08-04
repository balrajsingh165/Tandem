/**
 * Compose screen showing gateway health: dialer-role status, LAN listener state,
 * connected desktops, BT audio state, and the emergency-policy notice. Renders
 * StatusViewModel state; no logic.
 */
package com.tandem.gateway.ui.status

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.tandem.gateway.R

@Composable
fun StatusScreen(
    onOpenPairing: () -> Unit,
    onOpenSettings: () -> Unit,
    onOpenDialpad: () -> Unit,
    viewModel: StatusViewModel = hiltViewModel(),
) {
    val state by viewModel.uiState.collectAsStateWithLifecycle()
    val context = LocalContext.current

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            text = stringResource(R.string.status_title),
            style = MaterialTheme.typography.headlineSmall,
        )

        if (!state.isDefaultDialer) {
            Card {
                Column(Modifier.padding(12.dp), Arrangement.spacedBy(8.dp)) {
                    Text(stringResource(R.string.status_dialer_role_missing))
                    Button(onClick = {
                        viewModel.roleRequestIntent()?.let(context::startActivity)
                    }) {
                        Text(stringResource(R.string.status_dialer_role_action))
                    }
                }
            }
        } else {
            Text(stringResource(R.string.status_dialer_role_held))
        }

        Text(
            if (state.listening) {
                stringResource(R.string.status_listener_running)
            } else {
                stringResource(R.string.status_listener_stopped)
            },
        )
        Text(stringResource(R.string.status_connected_desktops, state.connectedDesktops))
        Text(stringResource(R.string.status_audio_route, state.audioRoute.name))

        if (state.hasActiveEmergency) {
            Text(
                text = stringResource(R.string.emergency_read_only),
                color = MaterialTheme.colorScheme.error,
            )
        }

        Card {
            Text(
                text = stringResource(R.string.emergency_policy_notice),
                modifier = Modifier.padding(12.dp),
                style = MaterialTheme.typography.bodySmall,
            )
        }

        Button(onClick = onOpenDialpad) { Text(stringResource(R.string.dialpad_title)) }
        Button(onClick = onOpenPairing) { Text(stringResource(R.string.pairing_title)) }
        Button(onClick = onOpenSettings) { Text(stringResource(R.string.settings_title)) }
    }
}
