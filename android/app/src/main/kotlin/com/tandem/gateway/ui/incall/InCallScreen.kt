/**
 * Compose in-call UI on the handset: answer/reject/end, mute, hold, merge, DTMF
 * pad, audio route picker. The default-dialer contract requires this to be fully
 * usable without any desktop.
 */
package com.tandem.gateway.ui.incall

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.tandem.gateway.R
import com.tandem.gateway.domain.model.AudioRoute
import com.tandem.gateway.domain.model.CallState

@Composable
fun InCallScreen(viewModel: InCallViewModel = hiltViewModel()) {
    val state by viewModel.uiState.collectAsStateWithLifecycle()
    val call = state.primaryCall ?: return

    Column(
        modifier = Modifier
            .fillMaxSize()
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            text = call.remoteDisplayName.ifEmpty {
                call.remoteNumber.ifEmpty { stringResource(R.string.call_unknown_number) }
            },
            style = MaterialTheme.typography.headlineMedium,
        )
        Text(call.state.name, style = MaterialTheme.typography.bodyMedium)

        if (state.isEmergency) {
            Text(
                text = stringResource(R.string.emergency_read_only),
                color = MaterialTheme.colorScheme.error,
            )
        }

        if (call.state == CallState.RINGING) {
            Row(Modifier.fillMaxWidth(), Arrangement.spacedBy(8.dp)) {
                Button(onClick = { viewModel.answer(call.callId) }, modifier = Modifier.weight(1f)) {
                    Text(stringResource(R.string.call_answer))
                }
                OutlinedButton(
                    onClick = { viewModel.reject(call.callId) },
                    modifier = Modifier.weight(1f),
                ) {
                    Text(stringResource(R.string.call_decline))
                }
            }
        } else {
            Row(Modifier.fillMaxWidth(), Arrangement.spacedBy(8.dp)) {
                OutlinedButton(
                    onClick = { viewModel.setMuted(!state.muted) },
                    modifier = Modifier.weight(1f),
                ) {
                    Text(
                        stringResource(
                            if (state.muted) R.string.call_unmute else R.string.call_mute,
                        ),
                    )
                }
                OutlinedButton(
                    onClick = {
                        if (call.state == CallState.HOLDING) {
                            viewModel.unhold(call.callId)
                        } else {
                            viewModel.hold(call.callId)
                        }
                    },
                    enabled = call.canHold && !state.isEmergency,
                    modifier = Modifier.weight(1f),
                ) {
                    Text(
                        stringResource(
                            if (call.state == CallState.HOLDING) {
                                R.string.call_resume
                            } else {
                                R.string.call_hold
                            },
                        ),
                    )
                }
            }

            if (state.canMerge) {
                OutlinedButton(
                    onClick = { viewModel.merge(call.callId) },
                    enabled = call.canMerge && !state.isEmergency,
                ) {
                    Text(stringResource(R.string.call_merge))
                }
            }

            Row(Modifier.fillMaxWidth(), Arrangement.spacedBy(8.dp)) {
                OutlinedButton(
                    onClick = { viewModel.setAudioRoute(AudioRoute.SPEAKER) },
                    enabled = !state.isEmergency,
                    modifier = Modifier.weight(1f),
                ) {
                    Text(AudioRoute.SPEAKER.name)
                }
                OutlinedButton(
                    onClick = { viewModel.setAudioRoute(AudioRoute.EARPIECE) },
                    enabled = !state.isEmergency,
                    modifier = Modifier.weight(1f),
                ) {
                    Text(AudioRoute.EARPIECE.name)
                }
            }

            Button(
                onClick = { viewModel.end(call.callId) },
                enabled = !state.isEmergency,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(stringResource(R.string.call_end))
            }
        }
    }
}
