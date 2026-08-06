/**
 * Offline caller insight for a number with no saved contact: region, carrier and
 * line type, derived from the digits alone with libphonenumber. Nothing leaves the
 * phone — unlike a crowdsourced name database, this needs no lookup service and
 * discloses nothing about the user or the caller.
 */
package com.tandem.gateway.contacts

import android.content.Context
import android.telephony.TelephonyManager
import com.google.i18n.phonenumbers.NumberParseException
import com.google.i18n.phonenumbers.PhoneNumberUtil
import com.google.i18n.phonenumbers.PhoneNumberToCarrierMapper
import com.google.i18n.phonenumbers.geocoding.PhoneNumberOfflineGeocoder
import dagger.hilt.android.qualifiers.ApplicationContext
import java.util.Locale
import java.util.concurrent.ConcurrentHashMap
import javax.inject.Inject
import javax.inject.Singleton

/** What can be said about a number without asking anyone. */
data class NumberInsight(
    val region: String,
    val carrier: String,
    val lineType: String,
) {
    /** One line for the UI, skipping whatever could not be determined. */
    val summary: String
        get() = listOf(lineType, carrier, region).filter { it.isNotBlank() }.joinToString(" · ")

    val isEmpty: Boolean get() = summary.isEmpty()
}

@Singleton
class NumberInsightResolver @Inject constructor(
    @ApplicationContext private val context: Context,
) {
    private val util by lazy { PhoneNumberUtil.getInstance() }
    private val geocoder by lazy { PhoneNumberOfflineGeocoder.getInstance() }
    private val carriers by lazy { PhoneNumberToCarrierMapper.getInstance() }
    private val memo = ConcurrentHashMap<String, NumberInsight>()

    fun insightFor(number: String): NumberInsight? {
        if (number.isBlank()) return null
        memo[number]?.let { return it.takeUnless { cached -> cached.isEmpty } }

        val insight = runCatching { compute(number) }.getOrNull() ?: EMPTY
        memo[number] = insight
        return insight.takeUnless { it.isEmpty }
    }

    private fun compute(number: String): NumberInsight {
        val parsed = try {
            util.parse(number, defaultRegion())
        } catch (_: NumberParseException) {
            return EMPTY
        }
        if (!util.isValidNumber(parsed)) return EMPTY

        val locale = Locale.getDefault()
        return NumberInsight(
            region = geocoder.getDescriptionForNumber(parsed, locale).orEmpty(),
            carrier = carriers.getNameForNumber(parsed, locale).orEmpty(),
            lineType = when (util.getNumberType(parsed)) {
                PhoneNumberUtil.PhoneNumberType.MOBILE -> "Mobile"
                PhoneNumberUtil.PhoneNumberType.FIXED_LINE -> "Landline"
                PhoneNumberUtil.PhoneNumberType.FIXED_LINE_OR_MOBILE -> "Phone"
                PhoneNumberUtil.PhoneNumberType.TOLL_FREE -> "Toll free"
                PhoneNumberUtil.PhoneNumberType.PREMIUM_RATE -> "Premium rate"
                PhoneNumberUtil.PhoneNumberType.VOIP -> "Internet call"
                else -> ""
            },
        )
    }

    /**
     * A national-format number can only be parsed against a region. The SIM's
     * network is the best guess; the locale is the fallback for a phone with no
     * service.
     */
    private fun defaultRegion(): String {
        val fromSim = runCatching {
            context.getSystemService(TelephonyManager::class.java)?.networkCountryIso
        }.getOrNull()

        return (fromSim?.takeIf { it.isNotBlank() } ?: Locale.getDefault().country)
            .uppercase(Locale.ROOT)
    }

    private companion object {
        val EMPTY = NumberInsight("", "", "")
    }
}
