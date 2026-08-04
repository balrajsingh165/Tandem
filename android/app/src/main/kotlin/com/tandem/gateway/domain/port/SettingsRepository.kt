/**
 * Port over user settings (autostart, listening port, device display name)
 * exposed as Flows with suspend setters. Backed by DataStore in
 * SettingsRepositoryImpl.
 */
package com.tandem.gateway.domain.port

import kotlinx.coroutines.flow.Flow

interface SettingsRepository {
    val autostartEnabled: Flow<Boolean>

    val listenPort: Flow<Int>

    val deviceDisplayName: Flow<String>

    /** Persisted so the version survives process death and signals gaps. */
    val callLogVersion: Flow<Long>

    suspend fun setAutostartEnabled(enabled: Boolean)

    suspend fun setListenPort(port: Int)

    suspend fun setDeviceDisplayName(name: String)

    suspend fun setCallLogVersion(version: Long)

    companion object {
        const val DEFAULT_PORT: Int = 46521
    }
}
