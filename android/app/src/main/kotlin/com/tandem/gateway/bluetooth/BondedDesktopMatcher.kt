/**
 * Resolves a paired desktop's stored BT MAC to a live BluetoothDevice among
 * current bonds, so routing targets the right HF. Reports unbonded desktops so
 * UX can prompt Bluetooth pairing.
 */
package com.tandem.gateway.bluetooth

import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothManager
import android.content.Context
import com.tandem.gateway.domain.model.PairedDesktop
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class BondedDesktopMatcher @Inject constructor(
    @ApplicationContext private val context: Context,
) {
    private val adapter
        get() = context.getSystemService(BluetoothManager::class.java)?.adapter

    /**
     * LAN pairing and Bluetooth bonding are separate steps: a desktop can be
     * fully trusted for control while still not bonded for audio (docs/07).
     */
    @Suppress("MissingPermission")
    fun resolve(desktop: PairedDesktop): BluetoothDevice? {
        val address = desktop.btMacAddress?.takeIf { it.isNotEmpty() } ?: return null
        return runCatching {
            adapter?.bondedDevices?.firstOrNull { it.address.equals(address, ignoreCase = true) }
        }.getOrNull()
    }

    fun isBonded(desktop: PairedDesktop): Boolean = resolve(desktop) != null

    @Suppress("MissingPermission")
    fun bondedAddresses(): List<String> = runCatching {
        adapter?.bondedDevices?.map { it.address }.orEmpty()
    }.getOrDefault(emptyList())
}
