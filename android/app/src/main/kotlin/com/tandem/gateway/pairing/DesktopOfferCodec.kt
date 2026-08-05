/**
 * Parses the pairing offer a desktop renders on screen and this phone scans with
 * its camera: version, the desktop's SPKI fingerprint to expect, the one-time
 * token to accept, and the name to show in the confirmation sheet. Mirror of
 * tandem_pairing::DesktopOffer; the compact keys are wire contract.
 */
package com.tandem.gateway.pairing

import com.tandem.gateway.domain.port.ScannedOffer
import javax.inject.Inject
import javax.inject.Singleton
import org.json.JSONObject

@Singleton
class DesktopOfferCodec @Inject constructor() {

    fun decode(raw: String): Result<ScannedOffer> = runCatching {
        val json = JSONObject(raw.trim())
        val version = json.optInt(KEY_VERSION, 0)
        require(version == OFFER_VERSION) { "unsupported pairing code version $version" }

        val fingerprint = json.optString(KEY_FINGERPRINT)
        val token = json.optString(KEY_TOKEN)
        require(fingerprint.isNotEmpty() && token.isNotEmpty()) { "incomplete pairing code" }

        ScannedOffer(
            fingerprint = fingerprint,
            token = token,
            desktopName = json.optString(KEY_NAME).ifEmpty { "Computer" },
        )
    }

    companion object {
        const val OFFER_VERSION: Int = 1
        const val KEY_VERSION: String = "v"
        const val KEY_FINGERPRINT: String = "fp"
        const val KEY_TOKEN: String = "tok"
        const val KEY_NAME: String = "name"
    }
}
