/**
 * The phone's home: Recents, Contacts and Keypad behind a bottom bar, with the
 * Connect menu in the top bar. This is the surface that has to stand on its own as
 * the default dialer, so nothing here depends on a paired computer.
 */
package com.tandem.gateway.ui.home

import android.content.Intent
import android.net.Uri
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CallMade
import androidx.compose.material.icons.filled.CallMissed
import androidx.compose.material.icons.filled.CallReceived
import androidx.compose.material.icons.filled.Call
import androidx.compose.material.icons.filled.Chat
import androidx.compose.material.icons.filled.Computer
import androidx.compose.material.icons.filled.Message
import androidx.compose.material.icons.filled.Dialpad
import androidx.compose.material.icons.filled.History
import androidx.compose.material.icons.filled.Menu
import androidx.compose.material.icons.filled.People
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.tandem.gateway.R
import com.tandem.gateway.domain.model.CallLogEntry
import com.tandem.gateway.domain.model.CallLogType
import com.tandem.gateway.domain.model.ContactNumber
import com.tandem.gateway.domain.port.ContactSort
import com.tandem.gateway.domain.port.ContactSource
import com.tandem.gateway.ui.dialpad.DialpadScreen
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

private enum class Tab { RECENTS, CONTACTS, KEYPAD }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(
    initialNumber: String,
    onOpenConnect: () -> Unit,
    viewModel: HomeViewModel = hiltViewModel(),
) {
    val state by viewModel.uiState.collectAsStateWithLifecycle()
    // A tel: intent means the user wants to dial, so the keypad leads.
    var tab by remember { mutableStateOf(if (initialNumber.isEmpty()) Tab.RECENTS else Tab.KEYPAD) }
    var menuOpen by remember { mutableStateOf(false) }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.app_phone_title)) },
                actions = {
                    Box {
                        IconButton(onClick = { menuOpen = true }) {
                            Icon(Icons.Filled.Menu, contentDescription = "Menu")
                        }
                        DropdownMenu(
                            expanded = menuOpen,
                            onDismissRequest = { menuOpen = false },
                        ) {
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
                },
            )
        },
        bottomBar = {
            NavigationBar {
                NavigationBarItem(
                    selected = tab == Tab.RECENTS,
                    onClick = { tab = Tab.RECENTS },
                    icon = { Icon(Icons.Filled.History, contentDescription = null) },
                    label = { Text(stringResource(R.string.tab_recents)) },
                )
                NavigationBarItem(
                    selected = tab == Tab.CONTACTS,
                    onClick = { tab = Tab.CONTACTS },
                    icon = { Icon(Icons.Filled.People, contentDescription = null) },
                    label = { Text(stringResource(R.string.tab_contacts)) },
                )
                NavigationBarItem(
                    selected = tab == Tab.KEYPAD,
                    onClick = { tab = Tab.KEYPAD },
                    icon = { Icon(Icons.Filled.Dialpad, contentDescription = null) },
                    label = { Text(stringResource(R.string.tab_keypad)) },
                )
            }
        },
    ) { padding ->
        Box(Modifier.padding(padding)) {
            when (tab) {
                Tab.RECENTS -> RecentsList(
                    entries = state.recents,
                    loading = state.loading,
                    notice = state.notice,
                    insightFor = viewModel::insightFor,
                    onCall = viewModel::call,
                )

                Tab.CONTACTS -> Column {
                    ContactControls(
                        sort = state.sort,
                        sources = state.sources,
                        selected = state.selectedSources,
                        onSort = viewModel::setSort,
                        onToggleSource = viewModel::toggleSource,
                    )
                    ContactsList(
                        entries = state.contacts,
                        loading = state.loading,
                        notice = state.notice,
                        onCall = viewModel::call,
                    )
                }

                // The keypad screen owns its own chrome, so it is given the plain
                // variant rather than a second top bar.
                Tab.KEYPAD -> DialpadScreen(
                    initialNumber = initialNumber,
                    onOpenConnect = onOpenConnect,
                    showChrome = false,
                )
            }
        }
    }
}

@Composable
private fun RecentsList(
    entries: List<CallLogEntry>,
    loading: Boolean,
    notice: String?,
    insightFor: (String) -> String,
    onCall: (String) -> Unit,
) {
    if (entries.isEmpty()) {
        Empty(
            if (loading) stringResource(R.string.home_loading) else notice
                ?: stringResource(R.string.home_no_recents),
        )
        return
    }

    LazyColumn(Modifier.fillMaxSize()) {
        items(entries, key = { it.entryId }) { entry ->
            val subtitle = entry.displayName.takeIf { it.isNotBlank() }
                ?.let { entry.number }
                ?: insightFor(entry.number)
            var expanded by remember(entry.entryId) { mutableStateOf(false) }

            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { expanded = !expanded }
                    .padding(horizontal = 16.dp, vertical = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(
                    imageVector = entry.type.icon(),
                    contentDescription = null,
                    tint = if (entry.type == CallLogType.MISSED) {
                        MaterialTheme.colorScheme.error
                    } else {
                        MaterialTheme.colorScheme.onSurfaceVariant
                    },
                    modifier = Modifier.size(18.dp),
                )
                Column(
                    modifier = Modifier
                        .weight(1f)
                        .padding(start = 14.dp),
                ) {
                    Text(
                        text = entry.displayName.ifBlank { entry.number },
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.Medium,
                    )
                    Text(
                        text = listOf(stamp(entry.startedAtMs), subtitle)
                            .filter { it.isNotBlank() }
                            .joinToString(" · "),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                IconButton(onClick = { onCall(entry.number) }) {
                    Icon(
                        Icons.Filled.Call,
                        contentDescription = stringResource(R.string.dialpad_call),
                        tint = MaterialTheme.colorScheme.primary,
                    )
                }
            }

            if (expanded) {
                RowActions(number = entry.number)
            }
        }
    }
}

/**
 * Hands the number to the app that owns the conversation rather than reading it.
 * Tandem never touches message content: SMS goes out as an intent, WhatsApp via its
 * public wa.me link, and a missing app simply means the action is not offered.
 */
@Composable
private fun RowActions(number: String) {
    val context = LocalContext.current
    val digits = remember(number) { number.filter { it.isDigit() || it == '+' } }

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(start = 48.dp, end = 16.dp, bottom = 10.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        ActionChip(
            label = stringResource(R.string.action_message),
            icon = Icons.Filled.Message,
        ) {
            context.launchOrIgnore(
                Intent(Intent.ACTION_SENDTO, Uri.parse("smsto:$digits")),
            )
        }
        ActionChip(
            label = stringResource(R.string.action_whatsapp),
            icon = Icons.Filled.Chat,
        ) {
            // wa.me is WhatsApp's documented deep link and needs no permission.
            context.launchOrIgnore(
                Intent(Intent.ACTION_VIEW, Uri.parse("https://wa.me/${digits.removePrefix("+")}")),
            )
        }
    }
}

@Composable
private fun ActionChip(label: String, icon: ImageVector, onClick: () -> Unit) {
    Surface(
        shape = MaterialTheme.shapes.small,
        color = MaterialTheme.colorScheme.surfaceVariant,
        onClick = onClick,
    ) {
        Row(
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 7.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(icon, contentDescription = null, Modifier.size(15.dp))
            Text(text = label, style = MaterialTheme.typography.labelSmall)
        }
    }
}

/** A phone without the target app must not crash the dialer. */
private fun android.content.Context.launchOrIgnore(intent: Intent) {
    runCatching { startActivity(intent) }
}

/**
 * Sort and source pickers. Both re-run the provider query rather than reordering
 * the page in hand, so the whole directory answers to them.
 */
@Composable
private fun ContactControls(
    sort: ContactSort,
    sources: List<ContactSource>,
    selected: Set<String>,
    onSort: (ContactSort) -> Unit,
    onToggleSource: (String) -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 12.dp, vertical = 8.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            SortChip("Name", sort == ContactSort.NAME) { onSort(ContactSort.NAME) }
            SortChip("Recent", sort == ContactSort.RECENT) { onSort(ContactSort.RECENT) }
            SortChip("Starred", sort == ContactSort.STARRED_FIRST) {
                onSort(ContactSort.STARRED_FIRST)
            }
        }

        // Only shown when the phone has more than one account; a single-source
        // phone has nothing to choose between.
        if (sources.size > 1) {
            Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                sources.forEach { source ->
                    SortChip(
                        label = "${source.label} (${source.count})",
                        active = selected.isEmpty() || source.accountType in selected,
                    ) { onToggleSource(source.accountType) }
                }
            }
        }
    }
}

@Composable
private fun SortChip(label: String, active: Boolean, onClick: () -> Unit) {
    Surface(
        shape = MaterialTheme.shapes.small,
        color = if (active) {
            MaterialTheme.colorScheme.primaryContainer
        } else {
            MaterialTheme.colorScheme.surfaceVariant
        },
        onClick = onClick,
    ) {
        Text(
            text = label,
            modifier = Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
            style = MaterialTheme.typography.labelSmall,
            color = if (active) {
                MaterialTheme.colorScheme.primary
            } else {
                MaterialTheme.colorScheme.onSurfaceVariant
            },
        )
    }
}

@Composable
private fun ContactsList(
    entries: List<ContactNumber>,
    loading: Boolean,
    notice: String?,
    onCall: (String) -> Unit,
) {
    if (entries.isEmpty()) {
        Empty(
            if (loading) stringResource(R.string.home_loading) else notice
                ?: stringResource(R.string.home_no_contacts),
        )
        return
    }

    LazyColumn(Modifier.fillMaxSize()) {
        items(entries, key = { it.contactId + it.number }) { contact ->
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 16.dp, vertical = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Surface(
                    shape = CircleShape,
                    color = MaterialTheme.colorScheme.primaryContainer,
                    modifier = Modifier.size(38.dp),
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Text(
                            text = contact.displayName.firstOrNull()?.uppercase() ?: "#",
                            color = MaterialTheme.colorScheme.primary,
                            fontWeight = FontWeight.SemiBold,
                        )
                    }
                }
                Column(
                    modifier = Modifier
                        .weight(1f)
                        .padding(start = 14.dp),
                ) {
                    Text(
                        text = contact.displayName.ifBlank { contact.number },
                        style = MaterialTheme.typography.bodyMedium,
                        fontWeight = FontWeight.Medium,
                    )
                    Text(
                        text = listOf(contact.number, contact.label)
                            .filter { it.isNotBlank() }
                            .joinToString(" · "),
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                IconButton(onClick = { onCall(contact.number) }) {
                    Icon(
                        Icons.Filled.CallMade,
                        contentDescription = stringResource(R.string.dialpad_call),
                        tint = MaterialTheme.colorScheme.primary,
                    )
                }
            }
        }
    }
}

@Composable
private fun Empty(message: String) {
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Text(
            text = message,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(32.dp),
        )
    }
}

private fun CallLogType.icon(): ImageVector = when (this) {
    CallLogType.OUTGOING -> Icons.Filled.CallMade
    CallLogType.INCOMING -> Icons.Filled.CallReceived
    CallLogType.MISSED, CallLogType.REJECTED -> Icons.Filled.CallMissed
}

/** Today shows a time; anything older shows a date, as every dialer does. */
private fun stamp(startedAtMs: Long): String {
    if (startedAtMs <= 0L) return ""
    val now = System.currentTimeMillis()
    val sameDay = SimpleDateFormat("yyyyMMdd", Locale.getDefault()).let {
        it.format(Date(now)) == it.format(Date(startedAtMs))
    }
    val pattern = if (sameDay) "HH:mm" else "d MMM"
    return SimpleDateFormat(pattern, Locale.getDefault()).format(Date(startedAtMs))
}
