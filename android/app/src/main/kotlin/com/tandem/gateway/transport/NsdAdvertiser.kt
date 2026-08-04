/**
 * Advertises _tandem._tcp via NsdManager with TXT records for protocol version,
 * device id, and display name. Re-registers on network change; advertisement
 * carries no secrets.
 */
package com.tandem.gateway.transport

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class NsdAdvertiser @Inject constructor(
    @ApplicationContext private val context: Context,
) {
    private val nsdManager: NsdManager?
        get() = context.getSystemService(NsdManager::class.java)

    private var listener: NsdManager.RegistrationListener? = null

    /**
     * The advertisement is public to anyone on the LAN, so it carries only what
     * a desktop needs to recognize a phone it already paired with. Identity is
     * proven by the TLS pin, never by these records (docs/08).
     */
    fun register(port: Int, deviceId: String, displayName: String) {
        unregister()
        val manager = nsdManager ?: return

        val serviceInfo = NsdServiceInfo().apply {
            serviceName = SERVICE_NAME
            serviceType = SERVICE_TYPE
            this.port = port
            setAttribute(TXT_VERSION, EnvelopeCodec.PROTOCOL_VERSION.toString())
            setAttribute(TXT_DEVICE_ID, deviceId)
            setAttribute(TXT_NAME, displayName)
        }

        val registration = object : NsdManager.RegistrationListener {
            override fun onServiceRegistered(info: NsdServiceInfo) = Unit
            override fun onRegistrationFailed(info: NsdServiceInfo, errorCode: Int) = Unit
            override fun onServiceUnregistered(info: NsdServiceInfo) = Unit
            override fun onUnregistrationFailed(info: NsdServiceInfo, errorCode: Int) = Unit
        }

        listener = registration
        manager.registerService(serviceInfo, NsdManager.PROTOCOL_DNS_SD, registration)
    }

    fun unregister() {
        val manager = nsdManager ?: return
        listener?.let { runCatching { manager.unregisterService(it) } }
        listener = null
    }

    companion object {
        const val SERVICE_TYPE: String = "_tandem._tcp."
        const val SERVICE_NAME: String = "Tandem Gateway"
        const val TXT_VERSION: String = "v"
        const val TXT_DEVICE_ID: String = "id"
        const val TXT_NAME: String = "name"
    }
}
