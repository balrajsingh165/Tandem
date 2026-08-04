/**
 * Hilt module binding telephony-side ports: TelecomBridge to TelecomBridgeImpl,
 * CallMediaProvider to HfpCallMediaProvider, EmergencyNumberSource to
 * EmergencyNumberSourceImpl. Bindings only; no logic.
 */
package com.tandem.gateway.di

import com.tandem.gateway.bluetooth.HfpCallMediaProvider
import com.tandem.gateway.dialer.EmergencyNumberSourceImpl
import com.tandem.gateway.domain.port.CallMediaProvider
import com.tandem.gateway.domain.port.EmergencyNumberSource
import com.tandem.gateway.domain.port.TelecomBridge
import com.tandem.gateway.telecom.TelecomBridgeImpl
import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
abstract class TelecomModule {

    @Binds
    @Singleton
    abstract fun bindTelecomBridge(impl: TelecomBridgeImpl): TelecomBridge

    /**
     * Bound from Phase 1 even on Tier A builds: route mirroring feeds
     * ObserveCallState and the status screen, so the binding must always exist
     * (docs/16 Phase 1).
     */
    @Binds
    @Singleton
    abstract fun bindCallMediaProvider(impl: HfpCallMediaProvider): CallMediaProvider

    @Binds
    @Singleton
    abstract fun bindEmergencyNumberSource(
        impl: EmergencyNumberSourceImpl,
    ): EmergencyNumberSource
}
