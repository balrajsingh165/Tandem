/**
 * Compose screen for settings: paired desktop list with revoke actions, autostart
 * toggle, port override, device name. Revocation confirmation copy warns it is
 * immediate.
 */
package com.tandem.gateway.ui.settings

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.tandem.gateway.R
import com.tandem.gateway.domain.model.PairedDesktop

@Composable
private fun SyncRow(label: String, value: String) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Text(text = value, style = MaterialTheme.typography.bodyMedium)
    }
}

@Composable
fun SettingsScreen(
    onBack: () -> Unit,
    viewModel: SettingsViewModel = hiltViewModel(),
) {
    val state by viewModel.uiState.collectAsStateWithLifecycle()
    var pendingRevoke by remember { mutableStateOf<PairedDesktop?>(null) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        TextButton(onClick = onBack) { Text("Back") }

        Text(
            text = stringResource(R.string.settings_title),
            style = MaterialTheme.typography.headlineSmall,
        )

        // What a paired computer can read, stated plainly. Pairing grants call
        // control; this is the data that rides along with it, so the user should be
        // able to see it in one place rather than infer it.
        Card {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(12.dp),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Text(
                    text = stringResource(R.string.settings_sync_title),
                    style = MaterialTheme.typography.titleMedium,
                )
                Text(
                    text = stringResource(R.string.settings_sync_body),
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )

                SyncRow(
                    label = stringResource(R.string.settings_sync_calls),
                    value = if (state.callLogEntries > 0) {
                        stringResource(R.string.settings_sync_on)
                    } else {
                        stringResource(R.string.settings_sync_needs_permission)
                    },
                )
                SyncRow(
                    label = stringResource(R.string.settings_sync_contacts),
                    value = if (state.contactsShared > 0) {
                        stringResource(R.string.settings_sync_count, state.contactsShared)
                    } else {
                        stringResource(R.string.settings_sync_needs_permission)
                    },
                )
                state.contactSources.forEach { source ->
                    SyncRow(
                        label = "   ${source.label}",
                        value = source.count.toString(),
                    )
                }
                SyncRow(
                    label = stringResource(R.string.settings_sync_messages),
                    value = stringResource(R.string.settings_sync_never),
                )
            }
        }

        Text(
            text = stringResource(R.string.settings_paired_devices),
            style = MaterialTheme.typography.titleMedium,
        )

        LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp)) {
            items(state.pairedDesktops, key = { it.deviceId }) { desktop ->
                Card {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(12.dp),
                        horizontalArrangement = Arrangement.SpaceBetween,
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column {
                            Text(desktop.name)
                            Text(
                                text = desktop.platform.name.lowercase(),
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                        OutlinedButton(onClick = { pendingRevoke = desktop }) {
                            Text(stringResource(R.string.settings_revoke))
                        }
                    }
                }
            }
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(stringResource(R.string.settings_autostart))
            Switch(
                checked = state.autostartEnabled,
                onCheckedChange = { viewModel.setAutostart(it) },
            )
        }

        Text(stringResource(R.string.settings_port) + ": ${state.listenPort}")
        Text(stringResource(R.string.settings_device_name) + ": ${state.deviceDisplayName}")

        Card {
            Text(
                text = stringResource(R.string.emergency_policy_notice),
                modifier = Modifier.padding(12.dp),
                style = MaterialTheme.typography.bodySmall,
            )
        }
    }

    pendingRevoke?.let { desktop ->
        AlertDialog(
            onDismissRequest = { pendingRevoke = null },
            title = { Text(stringResource(R.string.settings_revoke)) },
            text = { Text(stringResource(R.string.settings_revoke_confirm, desktop.name)) },
            confirmButton = {
                TextButton(onClick = {
                    viewModel.revoke(desktop.deviceId)
                    pendingRevoke = null
                }) {
                    Text(stringResource(R.string.settings_revoke))
                }
            },
            dismissButton = {
                TextButton(onClick = { pendingRevoke = null }) { Text("Cancel") }
            },
        )
    }
}
