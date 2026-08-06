/**
 * Compose dialpad on the handset: number entry, delete, and place-call, prefilled
 * from a tel: intent when Tandem is opened as the dialer. Fully usable with no
 * desktop paired.
 */
package com.tandem.gateway.ui.dialpad

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
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Call
import androidx.compose.material.icons.filled.Computer
import androidx.compose.material.icons.filled.Menu
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.tandem.gateway.R

/** Letters under the digit: the muscle memory every phone dialer trains. */
private val SUBTITLES = mapOf(
    "2" to "ABC",
    "3" to "DEF",
    "4" to "GHI",
    "5" to "JKL",
    "6" to "MNO",
    "7" to "PQRS",
    "8" to "TUV",
    "9" to "WXYZ",
    "0" to "+",
)

private val KEYS = listOf(
    listOf("1", "2", "3"),
    listOf("4", "5", "6"),
    listOf("7", "8", "9"),
    listOf("*", "0", "#"),
)

@Composable
fun DialpadScreen(
    initialNumber: String,
    onOpenConnect: () -> Unit,
    /** False when a host scaffold already provides the bar and insets. */
    showChrome: Boolean = true,
    viewModel: DialpadViewModel = hiltViewModel(),
) {
    val dialString by viewModel.dialString.collectAsStateWithLifecycle()
    val failure by viewModel.failure.collectAsStateWithLifecycle()
    val suggestions by viewModel.suggestions.collectAsStateWithLifecycle()

    LaunchedEffect(initialNumber) { viewModel.setInitial(initialNumber) }

    Surface(
        modifier = Modifier.fillMaxSize(),
        color = MaterialTheme.colorScheme.background,
    ) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .then(
                    if (showChrome) Modifier.windowInsetsPadding(WindowInsets.safeDrawing)
                    else Modifier,
                )
                .padding(horizontal = 20.dp, vertical = 12.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            if (showChrome) {
            // A phone app's chrome: the product name, and one menu holding
            // everything that is not dialling.
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "Phone",
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.weight(1f),
                )
                var menuOpen by remember { mutableStateOf(false) }
                Box {
                    IconButton(onClick = { menuOpen = true }) {
                        Icon(Icons.Filled.Menu, contentDescription = "Menu")
                    }
                    DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                        DropdownMenuItem(
                            text = { Text(stringResource(R.string.menu_connect)) },
                            leadingIcon = {
                                Icon(Icons.Filled.Computer, contentDescription = null)
                            },
                            onClick = {
                                menuOpen = false
                                onOpenConnect()
                            },
                        )
                    }
                }
            }
            }

            Spacer(Modifier.size(10.dp))

            // The readout is the point of the screen, so it gets the room rather
            // than being one line among many.
            Text(
                text = dialString.ifEmpty { stringResource(R.string.dialpad_hint) },
                style = MaterialTheme.typography.headlineMedium,
                fontSize = if (dialString.length > 14) 24.sp else 32.sp,
                color = if (dialString.isEmpty()) {
                    MaterialTheme.colorScheme.onSurfaceVariant
                } else {
                    MaterialTheme.colorScheme.onSurface
                },
                textAlign = TextAlign.Center,
                maxLines = 2,
                modifier = Modifier.fillMaxWidth(),
            )

            failure?.let {
                Spacer(Modifier.size(10.dp))
                Text(
                    text = it,
                    color = MaterialTheme.colorScheme.error,
                    style = MaterialTheme.typography.bodyMedium,
                    textAlign = TextAlign.Center,
                )
            }

            // History takes the room between the readout and the keys and scrolls
            // within it, so the keypad never moves as the list grows or filters.
            if (suggestions.isEmpty()) {
                Spacer(Modifier.weight(1f))
            } else {
                LazyColumn(
                    modifier = Modifier
                        .weight(1f)
                        .fillMaxWidth()
                        .padding(top = 12.dp),
                    verticalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                    items(suggestions, key = { it.entryId }) { entry ->
                        SuggestionRow(
                            title = entry.displayName.ifBlank { entry.number },
                            subtitle = if (entry.displayName.isBlank()) "" else entry.number,
                            onPick = { viewModel.choose(entry.number) },
                            onCall = { viewModel.callNow(entry.number) },
                        )
                    }
                }
            }

            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                KEYS.forEach { row ->
                    Row(
                        Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        row.forEach { key ->
                            DialKey(
                                key = key,
                                sub = SUBTITLES[key].orEmpty(),
                                onClick = { viewModel.append(key) },
                                modifier = Modifier.weight(1f),
                            )
                        }
                    }
                }
            }

            Spacer(Modifier.size(16.dp))

            Row(
                Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Spacer(Modifier.weight(1f))
                Button(
                    onClick = { viewModel.call() },
                    enabled = dialString.isNotEmpty(),
                    modifier = Modifier
                        .weight(2f)
                        .height(58.dp),
                    shape = CircleShape,
                ) {
                    Text(
                        text = stringResource(R.string.dialpad_call),
                        fontWeight = FontWeight.SemiBold,
                    )
                }
                // Only shown when there is something to delete, so the row stays
                // balanced on an empty pad.
                Box(Modifier.weight(1f), contentAlignment = Alignment.Center) {
                    if (dialString.isNotEmpty()) {
                        TextButton(onClick = viewModel::backspace) {
                            Text(stringResource(R.string.dialpad_delete))
                        }
                    }
                }
            }

            Spacer(Modifier.size(6.dp))
        }
    }
}

/**
 * Tapping the row fills the field so the number can be checked before dialling;
 * the trailing button dials straight away.
 */
@Composable
private fun SuggestionRow(
    title: String,
    subtitle: String,
    onPick: () -> Unit,
    onCall: () -> Unit,
) {
    Surface(
        modifier = Modifier.fillMaxWidth(),
        shape = MaterialTheme.shapes.small,
        color = MaterialTheme.colorScheme.surface,
        onClick = onPick,
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 9.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Text(
                    text = title,
                    style = MaterialTheme.typography.bodyMedium,
                    fontWeight = FontWeight.Medium,
                )
                if (subtitle.isNotEmpty()) {
                    Text(
                        text = subtitle,
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
            IconButton(onClick = onCall) {
                Icon(
                    Icons.Filled.Call,
                    contentDescription = stringResource(R.string.dialpad_call),
                    tint = MaterialTheme.colorScheme.primary,
                )
            }
        }
    }
}

@Composable
private fun DialKey(
    key: String,
    sub: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        modifier = modifier.aspectRatio(1.35f),
        shape = MaterialTheme.shapes.medium,
        color = MaterialTheme.colorScheme.surfaceVariant,
        onClick = onClick,
    ) {
        Column(
            modifier = Modifier.fillMaxSize(),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            Text(text = key, fontSize = 24.sp, fontWeight = FontWeight.Medium)
            if (sub.isNotEmpty()) {
                Text(
                    text = sub,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}
