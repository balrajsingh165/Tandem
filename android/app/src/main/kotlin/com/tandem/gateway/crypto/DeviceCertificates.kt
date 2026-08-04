/**
 * Creates and persists the long-lived self-signed X.509 certificate wrapping the
 * identity key. Certificates are TLS carriers only; trust is pinned SPKI hashes,
 * never chains (ADR-0006).
 */
package com.tandem.gateway.crypto

import java.util.Calendar
import java.util.Date
import javax.inject.Inject
import javax.inject.Singleton
import javax.security.auth.x500.X500Principal

@Singleton
class DeviceCertificates @Inject constructor() {

    fun subject(): X500Principal = X500Principal("CN=Tandem Gateway")

    fun notBefore(): Date = Date()

    /**
     * Long validity is deliberate: expiry protects nothing when trust is a pinned
     * key, and a mid-deployment expiry would break pairing for no security gain
     * (docs/08 key-rotation section).
     */
    fun notAfter(): Date = Calendar.getInstance().apply {
        time = notBefore()
        add(Calendar.DAY_OF_YEAR, VALIDITY_DAYS)
    }.time

    companion object {
        const val VALIDITY_DAYS: Int = 3650
    }
}
