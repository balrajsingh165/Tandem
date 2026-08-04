/**
 * EmergencyNumberSource implementation over TelephonyManager.isEmergencyNumber
 * and getEmergencyNumberList, with a conservative static fallback (112/911) when
 * telephony is unavailable. Refreshes on SIM/carrier config change.
 */
package com.tandem.gateway.dialer

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.telephony.TelephonyManager
import androidx.core.content.ContextCompat
import com.tandem.gateway.domain.port.EmergencyNumberSource
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class EmergencyNumberSourceImpl @Inject constructor(
    @ApplicationContext private val context: Context,
) : EmergencyNumberSource {

    private val telephonyManager: TelephonyManager?
        get() = context.getSystemService(TelephonyManager::class.java)

    /**
     * Fails closed: when telephony cannot answer, a number on the conservative
     * fallback list is still treated as an emergency number. Wrongly refusing a
     * desktop dial is recoverable; wrongly allowing one is not (ADR-0008).
     */
    override suspend fun isEmergencyNumber(dialString: String): Boolean {
        val normalized = dialString.filter { it.isDigit() || it == '*' || it == '#' }
        if (normalized.isEmpty()) return false

        val manager = telephonyManager ?: return isFallbackEmergency(normalized)
        if (!hasPhoneStatePermission()) return isFallbackEmergency(normalized)

        return runCatching { manager.isEmergencyNumber(normalized) }
            .getOrElse { isFallbackEmergency(normalized) }
    }

    override suspend fun currentEmergencyNumbers(): List<String> {
        val manager = telephonyManager ?: return EmergencyNumberSource.CONSERVATIVE_FALLBACK
        if (!hasPhoneStatePermission()) return EmergencyNumberSource.CONSERVATIVE_FALLBACK

        val fromTelephony = runCatching {
            manager.emergencyNumberList.values
                .flatten()
                .map { it.number }
                .filter { it.isNotBlank() }
                .distinct()
        }.getOrDefault(emptyList())

        // Union, never replacement: the platform list can omit codes that are
        // valid on other networks the phone may roam onto.
        return (fromTelephony + EmergencyNumberSource.CONSERVATIVE_FALLBACK).distinct()
    }

    private fun isFallbackEmergency(normalized: String): Boolean =
        normalized in EmergencyNumberSource.CONSERVATIVE_FALLBACK

    private fun hasPhoneStatePermission(): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.READ_PHONE_STATE) ==
            PackageManager.PERMISSION_GRANTED
}
