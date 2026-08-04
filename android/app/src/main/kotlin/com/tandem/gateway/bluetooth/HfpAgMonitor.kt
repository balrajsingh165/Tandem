/**
 * Observes BluetoothHeadset profile state via BluetoothProfile proxy
 * (BLUETOOTH_CONNECT): which bonded devices have an HFP link, SCO audio state
 * changes. The AG itself is Android's Bluetooth stack — Tandem observes and
 * steers, never reimplements it.
 */
package com.tandem.gateway.bluetooth

import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothHeadset
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.content.Context
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

@Singleton
class HfpAgMonitor @Inject constructor(
    @ApplicationContext private val context: Context,
) {
    private val adapter: BluetoothAdapter?
        get() = context.getSystemService(BluetoothManager::class.java)?.adapter

    private var headsetProxy: BluetoothHeadset? = null

    private val _connectedHeadsets = MutableStateFlow<List<BluetoothDevice>>(emptyList())
    val connectedHeadsets: StateFlow<List<BluetoothDevice>> = _connectedHeadsets.asStateFlow()

    private val _scoActive = MutableStateFlow(false)
    val scoActive: StateFlow<Boolean> = _scoActive.asStateFlow()

    private val serviceListener = object : BluetoothProfile.ServiceListener {
        override fun onServiceConnected(profile: Int, proxy: BluetoothProfile) {
            if (profile != BluetoothProfile.HEADSET) return
            headsetProxy = proxy as BluetoothHeadset
            refresh()
        }

        override fun onServiceDisconnected(profile: Int) {
            if (profile != BluetoothProfile.HEADSET) return
            headsetProxy = null
            _connectedHeadsets.value = emptyList()
            _scoActive.value = false
        }
    }

    fun start() {
        adapter?.getProfileProxy(context, serviceListener, BluetoothProfile.HEADSET)
    }

    fun stop() {
        val proxy = headsetProxy ?: return
        adapter?.closeProfileProxy(BluetoothProfile.HEADSET, proxy)
        headsetProxy = null
    }

    /**
     * Requires BLUETOOTH_CONNECT; without it the list stays empty and routing UX
     * degrades to handset-only rather than failing (docs/12).
     */
    @Suppress("MissingPermission")
    fun refresh() {
        val proxy = headsetProxy ?: return
        _connectedHeadsets.value = runCatching { proxy.connectedDevices }.getOrDefault(emptyList())
        _scoActive.value = _connectedHeadsets.value.any {
            runCatching { proxy.isAudioConnected(it) }.getOrDefault(false)
        }
    }
}
