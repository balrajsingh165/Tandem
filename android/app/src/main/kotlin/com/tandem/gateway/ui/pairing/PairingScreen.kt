/**
 * Compose screen for pairing: requests the camera, scans the code shown on the
 * desktop, reports progress while that desktop connects and confirms, and renders
 * the accept/reject sheet for the legacy phone-shows-a-code path.
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
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.tandem.gateway.R
import com.tandem.gateway.domain.port.PairingWindowState
import com.tandem.gateway.ui.components.DetailRow
import com.tandem.gateway.ui.components.SectionCard
import com.tandem.gateway.ui.components.TandemScreen

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

    TandemScreen(
        title = stringResource(R.string.pairing_title),
        eyebrow = "One-time setup",
        onBack = onBack,
    ) {
        when (val current = state) {
            is PairingWindowState.Closed, is PairingWindowState.Failed -> {
                if (current is PairingWindowState.Failed) {
                    Notice(current.error.message.orEmpty(), isError = true)
                }

                if (scanning && cameraGranted) {
                    Text(
                        text = stringResource(R.string.pairing_scan_instruction),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .aspectRatio(1f)
                            .clip(MaterialTheme.shapes.large),
                    ) {
                        QrScannerView(
                            onDecoded = { viewModel.onCodeScanned(it) },
                            modifier = Modifier.fillMaxSize(),
                        )
                    }
                    OutlinedButton(
                        onClick = { viewModel.stopScanning() },
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Text("Cancel")
                    }
                } else {
                    SectionCard {
                        Text(
                            text = "Scan the code on your computer",
                            style = MaterialTheme.typography.titleMedium,
                            fontWeight = FontWeight.SemiBold,
                        )
                        Text(
                            text = stringResource(R.string.pairing_scan_prompt),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Button(
                            onClick = {
                                if (cameraGranted) {
                                    viewModel.startScanning()
                                } else {
                                    requestCamera.launch(Manifest.permission.CAMERA)
                                }
                            },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text(stringResource(R.string.pairing_scan_action))
                        }
                    }
                }

                scanError?.let { Notice(it, isError = true) }
            }

            is PairingWindowState.AwaitingDesktopApproval -> Waiting(
                stringResource(R.string.pairing_confirm_on_computer, current.desktopName),
            )

            is PairingWindowState.AwaitingDesktop -> {
                Waiting(stringResource(R.string.pairing_awaiting_desktop, current.desktopName))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
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
                Notice(stringResource(R.string.pairing_manual_instruction), isError = false)
                OutlinedButton(
                    onClick = { viewModel.closeWindow() },
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Text("Cancel")
                }
            }

            is PairingWindowState.AwaitingConfirmation -> {
                SectionCard(accented = true) {
                    Text(
                        text = stringResource(R.string.pairing_confirm_title),
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.SemiBold,
                    )
                    Text(
                        text = stringResource(
                            R.string.pairing_confirm_body,
                            current.desktopName,
                            current.desktopPlatform,
                        ),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    current.shortCode?.let { code ->
                        DetailRow(
                            label = stringResource(R.string.pairing_short_code_prompt),
                            value = code,
                            emphasis = true,
                        )
                    }
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
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
            }

            is PairingWindowState.Completed -> SectionCard(accented = true) {
                Text(
                    text = "Paired with ${current.desktop.name}",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    text = "That computer can now place and control calls on this phone.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun Waiting(message: String) {
    SectionCard {
        Row(
            horizontalArrangement = Arrangement.spacedBy(14.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            CircularProgressIndicator(strokeWidth = 2.dp, modifier = Modifier.padding(2.dp))
            Text(text = message, style = MaterialTheme.typography.bodyMedium)
        }
    }
}

@Composable
private fun Notice(message: String, isError: Boolean) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = MaterialTheme.shapes.medium,
        color = if (isError) {
            MaterialTheme.colorScheme.errorContainer
        } else {
            MaterialTheme.colorScheme.surfaceVariant
        },
    ) {
        Text(
            text = message,
            modifier = Modifier.padding(14.dp),
            style = MaterialTheme.typography.bodyMedium,
            color = if (isError) {
                MaterialTheme.colorScheme.onErrorContainer
            } else {
                MaterialTheme.colorScheme.onSurfaceVariant
            },
        )
    }
}
