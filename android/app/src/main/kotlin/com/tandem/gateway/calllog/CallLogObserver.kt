/**
 * ContentObserver on the CallLog provider: bumps the persisted monotonic
 * log_version and emits change notifications that become CallLogChangedEvent
 * fan-outs.
 */
package com.tandem.gateway.calllog

import android.content.Context
import android.database.ContentObserver
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.provider.CallLog
import com.tandem.gateway.domain.port.SettingsRepository
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

@Singleton
class CallLogObserver @Inject constructor(
    @ApplicationContext private val context: Context,
    private val settingsRepository: SettingsRepository,
    private val scope: CoroutineScope,
) {
    private val _logVersion = MutableStateFlow(0L)
    val logVersion: StateFlow<Long> = _logVersion.asStateFlow()

    private var registered = false

    private val observer = object : ContentObserver(Handler(Looper.getMainLooper())) {
        override fun onChange(selfChange: Boolean, uri: Uri?) {
            bump()
        }
    }

    fun start() {
        if (registered) return
        scope.launch {
            _logVersion.value = settingsRepository.callLogVersion.first()
        }
        context.contentResolver.registerContentObserver(
            CallLog.Calls.CONTENT_URI,
            true,
            observer,
        )
        registered = true
    }

    fun stop() {
        if (!registered) return
        context.contentResolver.unregisterContentObserver(observer)
        registered = false
    }

    fun currentVersion(): Long = _logVersion.value

    /**
     * The version is persisted, so it keeps increasing across process death — a
     * desktop that reconnects can always tell whether it missed a change.
     */
    private fun bump() {
        val next = _logVersion.value + 1
        _logVersion.value = next
        scope.launch { settingsRepository.setCallLogVersion(next) }
    }
}
