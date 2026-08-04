/**
 * Builds the pairing QR payload (host, port, SPKI fingerprint, one-time token,
 * name) and renders it for display. Format is pinned in
 * docs/07-pairing-and-auth.md; token TTL 120 s.
 */
package com.tandem.gateway.pairing

import com.tandem.gateway.domain.port.PairingInvitation
import javax.inject.Inject
import javax.inject.Singleton
import org.json.JSONObject

@Singleton
class QrPayloadCodec @Inject constructor() {

    /** Compact keys are part of the wire contract; do not rename them. */
    fun encode(invitation: PairingInvitation): String =
        JSONObject()
            .put(KEY_VERSION, PAYLOAD_VERSION)
            .put(KEY_HOST, invitation.host)
            .put(KEY_PORT, invitation.port)
            .put(KEY_FINGERPRINT, invitation.fingerprint)
            .put(KEY_TOKEN, invitation.token)
            .put(KEY_NAME, invitation.phoneName)
            .toString()

    companion object {
        const val PAYLOAD_VERSION: Int = 1
        const val KEY_VERSION: String = "v"
        const val KEY_HOST: String = "host"
        const val KEY_PORT: String = "port"
        const val KEY_FINGERPRINT: String = "fp"
        const val KEY_TOKEN: String = "tok"
        const val KEY_NAME: String = "name"
    }
}
