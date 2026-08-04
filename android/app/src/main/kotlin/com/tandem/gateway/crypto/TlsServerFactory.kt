/**
 * Builds the TLS 1.3-only server context for LanServerImpl: presents the device
 * cert, requires client certs, and verifies peers against PairedDeviceRepository
 * pins — accepting unknown peers only into the provisional pairing path.
 */
package com.tandem.gateway.crypto

import com.tandem.gateway.domain.port.PairedDeviceRepository
import java.security.KeyStore
import java.security.cert.X509Certificate
import javax.inject.Inject
import javax.inject.Singleton
import javax.net.ssl.SSLContext
import javax.net.ssl.TrustManager
import javax.net.ssl.X509TrustManager
import kotlinx.coroutines.runBlocking

@Singleton
class TlsServerFactory @Inject constructor(
    private val pairedDeviceRepository: PairedDeviceRepository,
) {
    /** Set while a pairing window is open, admitting one unknown peer. */
    @Volatile
    private var pairingWindowOpen: Boolean = false

    fun setPairingWindowOpen(open: Boolean) {
        pairingWindowOpen = open
    }

    fun createContext(keyStore: KeyStore, keyPassword: CharArray): SSLContext {
        val keyManagerFactory = javax.net.ssl.KeyManagerFactory
            .getInstance(javax.net.ssl.KeyManagerFactory.getDefaultAlgorithm())
            .apply { init(keyStore, keyPassword) }

        return SSLContext.getInstance("TLSv1.3").apply {
            init(keyManagerFactory.keyManagers, arrayOf<TrustManager>(pinningTrustManager()), null)
        }
    }

    /**
     * There is no CA in the trust model: a peer is trusted exactly when its SPKI
     * hash matches a stored, non-revoked pin. Unknown peers are admitted only
     * while a pairing window is open, and only into the provisional path.
     */
    private fun pinningTrustManager(): X509TrustManager = object : X509TrustManager {
        override fun checkClientTrusted(chain: Array<out X509Certificate>?, authType: String?) {
            val leaf = chain?.firstOrNull() ?: throw CertificateRejected("no client certificate")
            val fingerprint = Fingerprints.toBase64Url(
                Fingerprints.spkiSha256(leaf.publicKey.encoded),
            )
            val known = runBlocking { pairedDeviceRepository.byPinnedKey(fingerprint) }

            if (known != null && !known.revoked) return
            if (pairingWindowOpen) return
            throw CertificateRejected("client key is not paired with this phone")
        }

        override fun checkServerTrusted(chain: Array<out X509Certificate>?, authType: String?) {
            throw CertificateRejected("this device does not act as a TLS client")
        }

        override fun getAcceptedIssuers(): Array<X509Certificate> = emptyArray()
    }
}

class CertificateRejected(message: String) : java.security.cert.CertificateException(message)
