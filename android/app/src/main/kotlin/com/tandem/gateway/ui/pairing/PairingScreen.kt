/**
 * Compose screen for pairing: renders the QR payload, the manual short-code
 * path, and the accept/reject confirmation sheet with the desktop's name and
 * fingerprint.
 */
package com.tandem.gateway.ui.pairing

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.tandem.gateway.R
import com.tandem.gateway.domain.port.PairingWindowState

@Composable
fun PairingScreen(
    onBack: () -> Unit,
    viewModel: PairingViewModel = hiltViewModel(),
) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val qrPayload by viewModel.qrPayload.collectAsStateWithLifecycle()

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        TextButton(onClick = onBack) { Text("Back") }

        Text(
            text = stringResource(R.string.pairing_title),
            style = MaterialTheme.typography.headlineSmall,
        )

        when (val current = state) {
            is PairingWindowState.Closed -> {
                Button(onClick = { viewModel.openWindow() }) {
                    Text(stringResource(R.string.pairing_title))
                }
            }

            is PairingWindowState.Open -> {
                Text(stringResource(R.string.pairing_scan_instruction))
                qrPayload?.let { payload ->
                    Card {
                        Text(
                            text = payload,
                            modifier = Modifier.padding(12.dp),
                            style = MaterialTheme.typography.bodySmall,
                        )
                    }
                }
                Text(stringResource(R.string.pairing_manual_instruction))
                OutlinedButton(onClick = { viewModel.closeWindow() }) { Text("Cancel") }
            }

            is PairingWindowState.AwaitingConfirmation -> {
                Text(
                    text = stringResource(R.string.pairing_confirm_title),
                    style = MaterialTheme.typography.titleMedium,
                )
                Text(
                    stringResource(
                        R.string.pairing_confirm_body,
                        current.desktopName,
                        current.desktopPlatform,
                    ),
                )

                current.shortCode?.let { code ->
                    Text(stringResource(R.string.pairing_short_code_prompt))
                    Text(text = code, style = MaterialTheme.typography.headlineLarge)
                }

                Row(Modifier.fillMaxWidth(), Arrangement.spacedBy(8.dp)) {
                    Button(onClick = { viewModel.accept() }, modifier = Modifier.weight(1f)) {
                        Text(stringResource(R.string.pairing_accept))
                    }
                    OutlinedButton(
                        onClick = { viewModel.reject() },
                        modifier = Modifier.weight(1f),
                    ) {
                        Text(stringResource(R.string.pairing_reject))
                    }
                }
            }

            is PairingWindowState.Completed -> Text("Paired with ${current.desktop.name}")

            is PairingWindowState.Failed -> Text(
                text = current.error.message.orEmpty(),
                color = MaterialTheme.colorScheme.error,
            )
        }
    }
}
