/**
 * SettingsRepository implementation over Preferences DataStore: autostart, port
 * override, device display name. Exposes Flows; all writes are suspend and
 * transactional.
 */
package com.tandem.gateway.data

import android.os.Build
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.longPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import com.tandem.gateway.domain.port.SettingsRepository
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

@Singleton
class SettingsRepositoryImpl @Inject constructor(
    private val dataStore: DataStore<Preferences>,
) : SettingsRepository {

    override val autostartEnabled: Flow<Boolean> =
        dataStore.data.map { it[KEY_AUTOSTART] ?: false }

    override val listenPort: Flow<Int> =
        dataStore.data.map { it[KEY_PORT] ?: SettingsRepository.DEFAULT_PORT }

    override val deviceDisplayName: Flow<String> =
        dataStore.data.map { it[KEY_DEVICE_NAME] ?: defaultDeviceName() }

    override val callLogVersion: Flow<Long> =
        dataStore.data.map { it[KEY_CALL_LOG_VERSION] ?: 0L }

    override suspend fun setAutostartEnabled(enabled: Boolean) {
        dataStore.edit { it[KEY_AUTOSTART] = enabled }
    }

    override suspend fun setListenPort(port: Int) {
        dataStore.edit { it[KEY_PORT] = port.coerceIn(1024, 65535) }
    }

    override suspend fun setDeviceDisplayName(name: String) {
        dataStore.edit { it[KEY_DEVICE_NAME] = name.take(MAX_NAME_LENGTH) }
    }

    override suspend fun setCallLogVersion(version: Long) {
        dataStore.edit { it[KEY_CALL_LOG_VERSION] = version }
    }

    private fun defaultDeviceName(): String =
        listOf(Build.MANUFACTURER, Build.MODEL)
            .filter { it.isNotBlank() }
            .joinToString(" ")
            .ifBlank { "Android phone" }

    private companion object {
        val KEY_AUTOSTART = booleanPreferencesKey("autostart_enabled")
        val KEY_PORT = intPreferencesKey("lan_port_override")
        val KEY_DEVICE_NAME = stringPreferencesKey("device_display_name")
        val KEY_CALL_LOG_VERSION = longPreferencesKey("call_log_version")
        const val MAX_NAME_LENGTH = 64
    }
}
