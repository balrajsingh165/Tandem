/**
 * Resolves this phone's LAN IPv4 address for the pairing payload, so a desktop
 * has somewhere to dial. Prefers a Wi-Fi interface because Tandem is a
 * same-room, same-network product; loopback and cellular are never offered.
 */
package com.tandem.gateway.pairing

import java.net.Inet4Address
import java.net.NetworkInterface

object LocalAddress {

    /** Interface name prefixes that carry LAN traffic on Android devices. */
    private val LAN_PREFIXES = listOf("wlan", "eth", "ap")

    /**
     * Returns the best LAN address, or null when the phone is not on a local
     * network — in which case pairing cannot work and the UI must say so.
     */
    fun current(): String? {
        val candidates = runCatching {
            NetworkInterface.getNetworkInterfaces()
                .asSequence()
                .filter { it.isUp && !it.isLoopback }
                .flatMap { iface ->
                    iface.inetAddresses.asSequence()
                        .filterIsInstance<Inet4Address>()
                        .filter { !it.isLoopbackAddress && it.hostAddress != null }
                        .map { iface.name to it.hostAddress!! }
                }
                .toList()
        }.getOrDefault(emptyList())

        // A cellular address is routable but not reachable from the desktop, so
        // a LAN interface is preferred over whatever happens to come first.
        return candidates.firstOrNull { (name, _) ->
            LAN_PREFIXES.any { name.startsWith(it, ignoreCase = true) }
        }?.second ?: candidates.firstOrNull { (_, address) -> isPrivate(address) }?.second
    }

    /** RFC 1918 ranges, the only addresses a same-room desktop can reach. */
    private fun isPrivate(address: String): Boolean =
        address.startsWith("192.168.") ||
            address.startsWith("10.") ||
            Regex("^172\\.(1[6-9]|2\\d|3[01])\\.").containsMatchIn(address)
}
