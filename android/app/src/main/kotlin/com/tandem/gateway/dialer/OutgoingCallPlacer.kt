/**
 * Places outgoing calls via TelecomManager.placeCall (requires CALL_PHONE +
 * ROLE_DIALER). Invoked only by TelecomBridgeImpl after the emergency guard has
 * passed.
 */
package com.tandem.gateway.dialer

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Bundle
import android.telecom.PhoneAccountHandle
import android.telecom.TelecomManager
import androidx.core.content.ContextCompat
import com.tandem.gateway.domain.port.TelecomError
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class OutgoingCallPlacer @Inject constructor(
    @ApplicationContext private val context: Context,
    private val defaultDialerManager: DefaultDialerManager,
) {
    private val telecomManager: TelecomManager?
        get() = context.getSystemService(TelecomManager::class.java)

    /**
     * [simSlot] of -1 uses the default outgoing account; otherwise the slot is
     * resolved to its PhoneAccountHandle.
     */
    fun place(number: String, simSlot: Int): Result<String> {
        if (!defaultDialerManager.isDefaultDialer()) {
            return Result.failure(TelecomError.DialerRoleMissing)
        }
        if (!hasCallPhonePermission()) {
            return Result.failure(TelecomError.PermissionDenied)
        }
        val manager = telecomManager
            ?: return Result.failure(TelecomError.Internal("TelecomManager unavailable"))

        val uri = Uri.fromParts("tel", number, null)
        val extras = Bundle().apply {
            accountForSlot(manager, simSlot)?.let {
                putParcelable(TelecomManager.EXTRA_PHONE_ACCOUNT_HANDLE, it)
            }
        }

        return runCatching {
            manager.placeCall(uri, extras)
            number
        }.recoverCatching { cause ->
            throw TelecomError.PlacementFailed(cause.message ?: "placeCall was refused")
        }
    }

    private fun hasCallPhonePermission(): Boolean =
        ContextCompat.checkSelfPermission(context, Manifest.permission.CALL_PHONE) ==
            PackageManager.PERMISSION_GRANTED

    private fun accountForSlot(manager: TelecomManager, simSlot: Int): PhoneAccountHandle? {
        if (simSlot < 0) return null
        return runCatching {
            manager.callCapablePhoneAccounts.getOrNull(simSlot)
        }.getOrNull()
    }
}
