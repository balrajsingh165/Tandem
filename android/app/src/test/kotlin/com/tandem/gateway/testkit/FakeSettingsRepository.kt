/**
 * In-memory SettingsRepository fake with mutable Flows for settings-dependent
 * behavior tests.
 */
package com.tandem.gateway.testkit

import com.tandem.gateway.domain.port.SettingsRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class FakeSettingsRepository : SettingsRepository {

    private val _autostartEnabled = MutableStateFlow(false)
    override val autostartEnabled: StateFlow<Boolean> = _autostartEnabled

    private val _listenPort = MutableStateFlow(SettingsRepository.DEFAULT_PORT)
    override val listenPort: StateFlow<Int> = _listenPort

    private val _deviceDisplayName = MutableStateFlow("Test Phone")
    override val deviceDisplayName: StateFlow<String> = _deviceDisplayName

    private val _callLogVersion = MutableStateFlow(0L)
    override val callLogVersion: StateFlow<Long> = _callLogVersion

    override suspend fun setAutostartEnabled(enabled: Boolean) {
        _autostartEnabled.value = enabled
    }

    override suspend fun setListenPort(port: Int) {
        _listenPort.value = port
    }

    override suspend fun setDeviceDisplayName(name: String) {
        _deviceDisplayName.value = name
    }

    override suspend fun setCallLogVersion(version: Long) {
        _callLogVersion.value = version
    }
}
