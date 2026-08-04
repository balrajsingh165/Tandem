/**
 * Hilt module binding LAN-side ports: LanServer to LanServerImpl, PairingManager
 * to PairingManagerImpl, IdentityStore to IdentityStoreImpl. Bindings only; no
 * logic.
 */
package com.tandem.gateway.di

import com.tandem.gateway.crypto.IdentityStoreImpl
import com.tandem.gateway.domain.port.IdentityStore
import com.tandem.gateway.domain.port.LanServer
import com.tandem.gateway.domain.port.PairingManager
import com.tandem.gateway.pairing.PairingManagerImpl
import com.tandem.gateway.transport.LanServerImpl
import dagger.Binds
import dagger.Module
import dagger.hilt.InstallIn
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
abstract class TransportModule {

    @Binds
    @Singleton
    abstract fun bindLanServer(impl: LanServerImpl): LanServer

    @Binds
    @Singleton
    abstract fun bindPairingManager(impl: PairingManagerImpl): PairingManager

    @Binds
    @Singleton
    abstract fun bindIdentityStore(impl: IdentityStoreImpl): IdentityStore
}
