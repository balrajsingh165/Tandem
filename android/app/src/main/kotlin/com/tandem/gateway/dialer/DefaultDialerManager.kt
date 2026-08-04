/**
 * Wraps RoleManager: reports whether Tandem holds ROLE_DIALER and builds the
 * role-request intent for onboarding. The app is inert as a gateway until the
 * role is granted (docs/12).
 */
package com.tandem.gateway.dialer

import android.app.role.RoleManager
import android.content.Context
import android.content.Intent
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class DefaultDialerManager @Inject constructor(
    @ApplicationContext private val context: Context,
) {
    private val roleManager: RoleManager?
        get() = context.getSystemService(RoleManager::class.java)

    fun isDefaultDialer(): Boolean =
        roleManager?.isRoleHeld(RoleManager.ROLE_DIALER) == true

    fun isRoleAvailable(): Boolean =
        roleManager?.isRoleAvailable(RoleManager.ROLE_DIALER) == true

    /**
     * Returns null when the role cannot be requested, so callers surface an
     * explanation rather than launching an intent that would fail.
     */
    fun buildRoleRequestIntent(): Intent? {
        val manager = roleManager ?: return null
        if (!manager.isRoleAvailable(RoleManager.ROLE_DIALER)) return null
        if (manager.isRoleHeld(RoleManager.ROLE_DIALER)) return null
        return manager.createRequestRoleIntent(RoleManager.ROLE_DIALER)
    }
}
