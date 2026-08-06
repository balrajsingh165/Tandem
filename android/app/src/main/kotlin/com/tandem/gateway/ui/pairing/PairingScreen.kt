/**
 * Compose screen for pairing: requests the camera, scans the code shown on the
 * desktop, reports progress while that desktop connects, and renders the
 * accept/reject confirmation sheet with its name and fingerprint.
 */
package com.tandem.gateway.ui.pairing

import android.Manifest
import android.content.pm.PackageManager
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
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
    val scanning by viewModel.scanning.collectAsStateWithLifecycle()
    val scanError by viewModel.scanError.collectAsStateWithLifecycle()

    val context = LocalContext.current
    var cameraGranted by remember {
        mutableStateOf(
            ContextCompat.checkSelfPermission(context, Manifest.permission.CAMERA) ==
                PackageManager.PERMISSION_GRANTED,
        )
    }
    val requestCamera = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        cameraGranted = granted
        if (granted) viewModel.startScanning()
    }

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
            is PairingWindowState.Closed, is PairingWindowState.Failed -> {
                if (current is PairingWindowState.Failed) {
                    Text(
                        text = current.error.message.orEmpty(),
                        color = MaterialTheme.colorScheme.error,
                    )
                }

                if (scanning && cameraGranted) {
                    Text(stringResource(R.string.pairing_scan_instruction))
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .aspectRatio(1f)
                            .clip(RoundedCornerShape(16.dp)),
                    ) {
                        QrScannerView(
                            onDecoded = { viewModel.onCodeScanned(it) },
                            modifier = Modifier.fillMaxSize(),
                        )
                    }
                    OutlinedButton(onClick = { viewModel.stopScanning() }) { Text("Cancel") }
                } else {
                    Text(stringResource(R.string.pairing_scan_prompt))
                    Button(
                        onClick = {
                            if (cameraGranted) {
                                viewModel.startScanning()
                            } else {
                                requestCamera.launch(Manifest.permission.CAMERA)
                            }
                        },
                    ) {
                        Text(stringResource(R.string.pairing_scan_action))
                    }
                }

                scanError?.let { message ->
                    Text(text = message, color = MaterialTheme.colorScheme.error)
                }
            }

            is PairingWindowState.AwaitingDesktopApproval -> {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    CircularProgressIndicator()
                    Text(stringResource(R.string.pairing_confirm_on_computer, current.desktopName))
                }
            }

            is PairingWindowState.AwaitingDesktop -> {
                Row(
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    CircularProgressIndicator()
                    Text(stringResource(R.string.pairing_awaiting_desktop, current.desktopName))
                }
                Row(Modifier.fillMaxWidth(), Arrangement.spacedBy(8.dp)) {
                    Button(onClick = { viewModel.rescan() }, modifier = Modifier.weight(1f)) {
                        Text(stringResource(R.string.pairing_scan_again))
                    }
                    OutlinedButton(
                        onClick = { viewModel.closeWindow() },
                        modifier = Modifier.weight(1f),
                    ) {
                        Text("Cancel")
                    }
                }
            }

            is PairingWindowState.Open -> {
                Text(stringResource(R.string.pairing_manual_instruction))
                Card {
                    Text(
                        text = stringResource(R.string.pairing_manual_instruction),
                        modifier = Modifier.padding(12.dp),
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
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
        }
    }
}
