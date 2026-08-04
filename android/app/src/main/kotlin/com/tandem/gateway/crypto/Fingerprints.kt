/**
 * Pure helpers: SPKI-SHA256 fingerprints, base64url rendering, and the 6-digit
 * pairing short code derived via HKDF from both SPKI hashes and the TLS exporter
 * (docs/07). No I/O.
 */
package com.tandem.gateway.crypto

import java.security.MessageDigest
import java.security.cert.CertificateFactory
import java.security.cert.X509Certificate
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec
import kotlin.experimental.and

object Fingerprints {

    /** Wire contract with the desktop; changing either breaks short codes. */
    private val SHORT_CODE_SALT = "tandem-pairing-short-code-v1".toByteArray()
    const val EXPORTER_LABEL: String = "EXPORTER-tandem-pairing-v1"
    const val EXPORTER_LENGTH: Int = 32

    fun spkiSha256(spkiDer: ByteArray): ByteArray =
        MessageDigest.getInstance("SHA-256").digest(spkiDer)

    fun spkiSha256FromCertificate(certDer: ByteArray): ByteArray {
        val factory = CertificateFactory.getInstance("X.509")
        val certificate = factory.generateCertificate(certDer.inputStream()) as X509Certificate
        return spkiSha256(certificate.publicKey.encoded)
    }

    fun toBase64Url(digest: ByteArray): String =
        android.util.Base64.encodeToString(
            digest,
            android.util.Base64.URL_SAFE or android.util.Base64.NO_PADDING or
                android.util.Base64.NO_WRAP,
        )

    fun fromBase64Url(encoded: String): ByteArray =
        android.util.Base64.decode(
            encoded,
            android.util.Base64.URL_SAFE or android.util.Base64.NO_PADDING or
                android.util.Base64.NO_WRAP,
        )

    /** Constant-time comparison so pins cannot be probed by timing. */
    fun matches(a: ByteArray, b: ByteArray): Boolean {
        if (a.size != b.size) return false
        var diff = 0
        for (i in a.indices) diff = diff or (a[i].toInt() xor b[i].toInt())
        return diff == 0
    }

    /**
     * HKDF-SHA256 over the TLS exporter, with info = phone SPKI hash followed by
     * desktop SPKI hash. Byte-identical to the desktop implementation, so both
     * screens show the same six digits.
     */
    fun deriveShortCode(
        tlsExporter: ByteArray,
        phoneSpkiSha256: ByteArray,
        desktopSpkiSha256: ByteArray,
    ): String {
        val prk = hmacSha256(SHORT_CODE_SALT, tlsExporter)
        val info = phoneSpkiSha256 + desktopSpkiSha256
        val okm = hmacSha256(prk, info + byteArrayOf(1))

        var value = 0
        for (i in 0 until 4) {
            value = (value shl 8) or (okm[i] and 0xFF.toByte()).toInt().and(0xFF)
        }
        value = value and 0x7FFFFFFF
        return (value % 1_000_000).toString().padStart(6, '0')
    }

    private fun hmacSha256(key: ByteArray, data: ByteArray): ByteArray {
        val mac = Mac.getInstance("HmacSHA256")
        mac.init(SecretKeySpec(key, "HmacSHA256"))
        return mac.doFinal(data)
    }
}
