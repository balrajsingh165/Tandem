/**
 * Compose in-call UI on the handset: caller identity, live duration,
 * answer/reject/end, mute, hold, merge, DTMF pad, and the audio route picker. The
 * default-dialer contract requires this to be fully usable without any desktop,
 * so every control the phone can offer is here rather than desktop-only.
 */
package com.tandem.gateway.ui.incall

import android.graphics.BitmapFactory
import android.net.Uri
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Call
import androidx.compose.material.icons.filled.Bluetooth
import androidx.compose.material.icons.filled.Chat
import androidx.compose.material.icons.filled.Headset
import androidx.compose.material.icons.filled.PhoneInTalk
import androidx.compose.material.icons.filled.CallEnd
import androidx.compose.material.icons.filled.Dialpad
import androidx.compose.material.icons.filled.MergeType
import androidx.compose.material.icons.filled.Mic
import androidx.compose.material.icons.filled.MicOff
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.VolumeDown
import androidx.compose.material.icons.filled.VolumeUp
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.tandem.gateway.R
import com.tandem.gateway.domain.model.AudioRoute
import com.tandem.gateway.domain.model.CallState
import kotlinx.coroutines.delay

/** How long "Call ended" stays up before the screen closes itself. */
private const val CALL_ENDED_LINGER_MS = 1200L

@Composable
fun InCallScreen(
    onCallsEnded: () -> Unit = {},
    viewModel: InCallViewModel = hiltViewModel(),
) {
    val state by viewModel.uiState.collectAsStateWithLifecycle()
    val call = state.primaryCall

    // A short beat so the outcome is legible, then the screen gets out of the way.
    LaunchedEffect(call == null) {
        if (call == null) {
            delay(CALL_ENDED_LINGER_MS)
            onCallsEnded()
        }
    }

    Surface(
        modifier = Modifier.fillMaxSize(),
        color = MaterialTheme.colorScheme.background,
    ) {
        if (call == null) {
            // Telecom tears the activity down; until it does, say so rather than
            // showing a blank screen.
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text(
                    text = stringResource(R.string.call_ended),
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            return@Surface
        }

        val ringing = call.state == CallState.RINGING
        val name = call.remoteDisplayName.ifEmpty {
            call.remoteNumber.ifEmpty { stringResource(R.string.call_unknown_number) }
        }
        var showKeypad by remember { mutableStateOf(false) }
        var showRoutes by remember { mutableStateOf(false) }

        Column(
            modifier = Modifier
                .fillMaxSize()
                .windowInsetsPadding(WindowInsets.safeDrawing)
                .padding(horizontal = 24.dp, vertical = 20.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Spacer(Modifier.weight(0.6f))

            Avatar(name = name, photoUri = state.callerPhotoUri)
            Spacer(Modifier.size(18.dp))

            Text(
                text = name,
                style = MaterialTheme.typography.headlineMedium,
                textAlign = TextAlign.Center,
            )
            if (call.remoteDisplayName.isNotEmpty() && call.remoteNumber.isNotEmpty()) {
                Spacer(Modifier.size(4.dp))
                Text(
                    text = call.remoteNumber,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            } else if (state.callerInsight.isNotEmpty()) {
                // Nothing saved for this number, so say what can be known from the
                // digits alone rather than leaving the caller anonymous.
                Spacer(Modifier.size(4.dp))
                Text(
                    text = state.callerInsight,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
            }

            if (state.callerOnWhatsApp) {
                Spacer(Modifier.size(6.dp))
                Row(
                    horizontalArrangement = Arrangement.spacedBy(5.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        Icons.Filled.Chat,
                        contentDescription = null,
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(13.dp),
                    )
                    Text(
                        text = stringResource(R.string.caller_on_whatsapp),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
            }

            Spacer(Modifier.size(10.dp))
            CallStatus(call.state, call.startedAtMs)

            if (state.isEmergency) {
                Spacer(Modifier.size(14.dp))
                Surface(
                    shape = MaterialTheme.shapes.medium,
                    color = MaterialTheme.colorScheme.errorContainer,
                ) {
                    Text(
                        text = stringResource(R.string.emergency_read_only),
                        modifier = Modifier.padding(14.dp),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onErrorContainer,
                        textAlign = TextAlign.Center,
                    )
                }
            }

            Spacer(Modifier.weight(1f))

            AnimatedVisibility(visible = showKeypad && !ringing) {
                DtmfPad(onDigit = { viewModel.dtmf(call.callId, it) })
            }

            AnimatedVisibility(visible = showRoutes && !ringing) {
                RoutePicker(
                    targets = state.audioTargets,
                    activeRoute = state.audioRoute,
                    activeBtAddress = state.activeBtAddress,
                    enabled = !state.isEmergency,
                    onPick = { target ->
                        viewModel.setAudioRoute(target.route, target.btAddress)
                    },
                )
            }

            if (ringing) {
                IncomingActions(
                    onAnswer = { viewModel.answer(call.callId) },
                    onDecline = { viewModel.reject(call.callId) },
                )
            } else {
                ActiveControls(
                    muted = state.muted,
                    onSpeaker = state.audioRoute == AudioRoute.SPEAKER,
                    holding = call.state == CallState.HOLDING,
                    canHold = call.canHold && !state.isEmergency,
                    canMerge = state.canMerge && call.canMerge && !state.isEmergency,
                    keypadOpen = showKeypad || showRoutes,
                    enabled = !state.isEmergency,
                    onMute = { viewModel.setMuted(!state.muted) },
                    onHold = {
                        if (call.state == CallState.HOLDING) {
                            viewModel.unhold(call.callId)
                        } else {
                            viewModel.hold(call.callId)
                        }
                    },
                    // More than one destination means the user gets a choice, not a
                    // guess about which one they meant.
                    onSpeakerToggle = {
                        if (state.audioTargets.size > 2) {
                            showRoutes = !showRoutes
                        } else {
                            viewModel.setAudioRoute(
                                if (state.audioRoute == AudioRoute.SPEAKER) {
                                    AudioRoute.EARPIECE
                                } else {
                                    AudioRoute.SPEAKER
                                },
                            )
                        }
                    },
                    onKeypad = { showKeypad = !showKeypad },
                    onMerge = { viewModel.merge(call.callId) },
                )

                Spacer(Modifier.size(18.dp))
                EndButton(enabled = !state.isEmergency) { viewModel.end(call.callId) }
            }

            Spacer(Modifier.size(8.dp))
        }
    }
}

/**
 * The contact photo when the address book has one, the initial otherwise. Decoded
 * here rather than through an image library: it is one small bitmap per call.
 */
@Composable
private fun Avatar(name: String, photoUri: String) {
    val context = LocalContext.current
    val photo by produceState<ImageBitmap?>(initialValue = null, photoUri) {
        value = if (photoUri.isEmpty()) {
            null
        } else {
            runCatching {
                context.contentResolver.openInputStream(Uri.parse(photoUri))?.use { stream ->
                    BitmapFactory.decodeStream(stream)?.asImageBitmap()
                }
            }.getOrNull()
        }
    }

    Box(
        modifier = Modifier
            .size(104.dp)
            .clip(CircleShape)
            .background(MaterialTheme.colorScheme.primaryContainer),
        contentAlignment = Alignment.Center,
    ) {
        val bitmap = photo
        if (bitmap != null) {
            Image(
                bitmap = bitmap,
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize(),
            )
        } else {
            Text(
                text = name.firstOrNull()?.uppercase() ?: "?",
                fontSize = 40.sp,
                fontWeight = FontWeight.SemiBold,
                color = MaterialTheme.colorScheme.primary,
            )
        }
    }
}

/** Ticks once a second while connected; a static "ACTIVE" tells the user nothing. */
@Composable
private fun CallStatus(state: CallState, startedAtMs: Long) {
    var elapsed by remember { mutableIntStateOf(0) }

    LaunchedEffect(state, startedAtMs) {
        if (state != CallState.ACTIVE || startedAtMs <= 0L) {
            elapsed = 0
            return@LaunchedEffect
        }
        while (true) {
            elapsed = ((System.currentTimeMillis() - startedAtMs) / 1000).coerceAtLeast(0).toInt()
            delay(1000)
        }
    }

    val label = when (state) {
        CallState.RINGING -> stringResource(R.string.call_incoming)
        CallState.DIALING, CallState.CONNECTING -> stringResource(R.string.call_dialing)
        CallState.HOLDING -> stringResource(R.string.call_on_hold)
        CallState.ACTIVE -> "%d:%02d".format(elapsed / 60, elapsed % 60)
        CallState.DISCONNECTING, CallState.DISCONNECTED -> stringResource(R.string.call_ended)
    }

    Text(
        text = label,
        style = MaterialTheme.typography.titleMedium,
        color = if (state == CallState.ACTIVE) {
            MaterialTheme.colorScheme.primary
        } else {
            MaterialTheme.colorScheme.onSurfaceVariant
        },
    )
}

@Composable
private fun IncomingActions(onAnswer: () -> Unit, onDecline: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Button(
            onClick = onDecline,
            modifier = Modifier
                .weight(1f)
                .height(60.dp),
            shape = CircleShape,
            colors = ButtonDefaults.buttonColors(
                containerColor = MaterialTheme.colorScheme.errorContainer,
                contentColor = MaterialTheme.colorScheme.onErrorContainer,
            ),
        ) {
            Icon(Icons.Filled.CallEnd, contentDescription = null, Modifier.size(20.dp))
            Spacer(Modifier.width(8.dp))
            Text(stringResource(R.string.call_decline), fontWeight = FontWeight.SemiBold)
        }
        Button(
            onClick = onAnswer,
            modifier = Modifier
                .weight(1f)
                .height(60.dp),
            shape = CircleShape,
        ) {
            Icon(Icons.Filled.Call, contentDescription = null, Modifier.size(20.dp))
            Spacer(Modifier.width(8.dp))
            Text(stringResource(R.string.call_answer), fontWeight = FontWeight.SemiBold)
        }
    }
}

@Composable
private fun ActiveControls(
    muted: Boolean,
    onSpeaker: Boolean,
    holding: Boolean,
    canHold: Boolean,
    canMerge: Boolean,
    keypadOpen: Boolean,
    enabled: Boolean,
    onMute: () -> Unit,
    onHold: () -> Unit,
    onSpeakerToggle: () -> Unit,
    onKeypad: () -> Unit,
    onMerge: () -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            ControlKey(
                label = stringResource(if (muted) R.string.call_unmute else R.string.call_mute),
                icon = if (muted) Icons.Filled.MicOff else Icons.Filled.Mic,
                active = muted,
                enabled = enabled,
                onClick = onMute,
                modifier = Modifier.weight(1f),
            )
            ControlKey(
                label = stringResource(R.string.call_keypad),
                icon = Icons.Filled.Dialpad,
                active = keypadOpen,
                enabled = enabled,
                onClick = onKeypad,
                modifier = Modifier.weight(1f),
            )
            ControlKey(
                label = stringResource(R.string.call_speaker),
                icon = if (onSpeaker) Icons.Filled.VolumeUp else Icons.Filled.VolumeDown,
                active = onSpeaker,
                enabled = enabled,
                onClick = onSpeakerToggle,
                modifier = Modifier.weight(1f),
            )
        }
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            ControlKey(
                label = stringResource(if (holding) R.string.call_resume else R.string.call_hold),
                icon = if (holding) Icons.Filled.PlayArrow else Icons.Filled.Pause,
                active = holding,
                enabled = canHold,
                onClick = onHold,
                modifier = Modifier.weight(1f),
            )
            if (canMerge) {
                ControlKey(
                    label = stringResource(R.string.call_merge),
                    icon = Icons.Filled.MergeType,
                    active = false,
                    enabled = true,
                    onClick = onMerge,
                    modifier = Modifier.weight(1f),
                )
            } else {
                Spacer(Modifier.weight(1f))
            }
            Spacer(Modifier.weight(1f))
        }
    }
}

/** A latching control: the on state has to be visible, not inferred. */
@Composable
private fun ControlKey(
    label: String,
    icon: ImageVector,
    active: Boolean,
    enabled: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        modifier = modifier,
        shape = MaterialTheme.shapes.medium,
        color = if (active) {
            MaterialTheme.colorScheme.primary
        } else {
            MaterialTheme.colorScheme.surfaceVariant
        },
        border = androidx.compose.foundation.BorderStroke(
            1.dp,
            if (active) {
                MaterialTheme.colorScheme.primary
            } else {
                MaterialTheme.colorScheme.outlineVariant
            },
        ),
        onClick = onClick,
        enabled = enabled,
    ) {
        val tint = when {
            !enabled -> MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f)
            active -> MaterialTheme.colorScheme.onPrimary
            else -> MaterialTheme.colorScheme.onSurface
        }

        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(vertical = 14.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(5.dp),
        ) {
            // The glyph carries the meaning; the word underneath keeps it
            // unambiguous and keeps the control reachable by screen readers.
            Icon(
                imageVector = icon,
                contentDescription = label,
                tint = tint,
                modifier = Modifier.size(22.dp),
            )
            Text(
                text = label,
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.Medium,
                color = tint,
            )
        }
    }
}

@Composable
private fun EndButton(enabled: Boolean, onClick: () -> Unit) {
    Button(
        onClick = onClick,
        enabled = enabled,
        modifier = Modifier
            .fillMaxWidth()
            .height(60.dp),
        shape = CircleShape,
        colors = ButtonDefaults.buttonColors(
            containerColor = MaterialTheme.colorScheme.error,
            contentColor = MaterialTheme.colorScheme.onError,
        ),
    ) {
        Icon(Icons.Filled.CallEnd, contentDescription = null, Modifier.size(20.dp))
        Spacer(Modifier.width(8.dp))
        Text(
            text = stringResource(R.string.call_end),
            fontWeight = FontWeight.SemiBold,
        )
    }
}

/**
 * Every audio destination the phone can use, including one row per connected
 * Bluetooth device so "Bluetooth" is never an ambiguous single option.
 */
@Composable
private fun RoutePicker(
    targets: List<AudioTarget>,
    activeRoute: AudioRoute,
    activeBtAddress: String,
    enabled: Boolean,
    onPick: (AudioTarget) -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(bottom = 18.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Text(
            text = stringResource(R.string.call_audio_output),
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        targets.forEach { target ->
            val active = target.route == activeRoute &&
                (target.route != AudioRoute.BLUETOOTH || target.btAddress == activeBtAddress)

            Surface(
                modifier = Modifier.fillMaxWidth(),
                shape = MaterialTheme.shapes.medium,
                color = if (active) {
                    MaterialTheme.colorScheme.primaryContainer
                } else {
                    MaterialTheme.colorScheme.surfaceVariant
                },
                enabled = enabled,
                onClick = { onPick(target) },
            ) {
                Row(
                    modifier = Modifier.padding(horizontal = 14.dp, vertical = 12.dp),
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(
                        imageVector = when (target.route) {
                            AudioRoute.SPEAKER -> Icons.Filled.VolumeUp
                            AudioRoute.WIRED_HEADSET -> Icons.Filled.Headset
                            AudioRoute.BLUETOOTH -> Icons.Filled.Bluetooth
                            AudioRoute.EARPIECE -> Icons.Filled.PhoneInTalk
                        },
                        contentDescription = null,
                        tint = if (active) {
                            MaterialTheme.colorScheme.primary
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        },
                        modifier = Modifier.size(18.dp),
                    )
                    Text(
                        text = target.label,
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (active) {
                            MaterialTheme.colorScheme.primary
                        } else {
                            MaterialTheme.colorScheme.onSurface
                        },
                    )
                }
            }
        }
    }
}

/** The pad the docstring has always promised; DTMF is unreachable without it. */
@Composable
private fun DtmfPad(onDigit: (String) -> Unit) {
    val rows = listOf(
        listOf("1", "2", "3"),
        listOf("4", "5", "6"),
        listOf("7", "8", "9"),
        listOf("*", "0", "#"),
    )

    Column(
        modifier = Modifier.padding(bottom = 18.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        for (row in rows) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                for (digit in row) {
                    Surface(
                        modifier = Modifier
                            .weight(1f)
                            .aspectRatio(1.6f),
                        shape = MaterialTheme.shapes.medium,
                        color = MaterialTheme.colorScheme.surfaceVariant,
                        onClick = { onDigit(digit) },
                    ) {
                        Box(contentAlignment = Alignment.Center) {
                            Text(
                                text = digit,
                                style = MaterialTheme.typography.titleMedium,
                                fontSize = 20.sp,
                            )
                        }
                    }
                }
            }
        }
    }
}
