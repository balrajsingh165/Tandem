/**
 * Routes external ACTION_DIAL and tel: intents into the handset dialpad UI with
 * the number prefilled, fulfilling the default-dialer contract. Never
 * auto-places calls from intents.
 */
package com.tandem.gateway.dialer

import android.content.Intent
import android.net.Uri
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class DialIntentRouter @Inject constructor() {

    /**
     * Extracts the number to prefill. Returning it rather than dialing is the
     * point: an app or web page can ask to dial, but only the user may place the
     * call.
     */
    fun prefillNumber(intent: Intent?): String? {
        if (intent == null) return null
        if (intent.action !in HANDLED_ACTIONS) return null
        val data: Uri = intent.data ?: return null
        if (!data.scheme.equals("tel", ignoreCase = true)) return null
        return data.schemeSpecificPart?.takeIf { it.isNotBlank() }
    }

    private companion object {
        val HANDLED_ACTIONS = setOf(Intent.ACTION_DIAL, Intent.ACTION_VIEW)
    }
}
