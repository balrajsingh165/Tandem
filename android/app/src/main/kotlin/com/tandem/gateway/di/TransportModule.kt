/**
 * Hilt module binding LAN-side ports: LanServer to LanServerImpl, PairingManager
 * to PairingManagerImpl, IdentityStore to IdentityStoreImpl. Bindings only; no
 * logic.
 */
package com.tandem.gateway.di

import com.tandem.gateway.crypto.IdentityStoreImpl
import com.tandem.gateway.domain.port.CallClaimArbiter
import com.tandem.gateway.domain.port.IdentityStore
import com.tandem.gateway.domain.port.LanServer
import com.tandem.gateway.domain.port.PairingManager
import com.tandem.gateway.pairing.PairingManagerImpl
import com.tandem.gateway.transport.LanServerImpl
import com.tandem.gateway.transport.SessionRegistry
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

    /**
     * Bound to the registry rather than the server: the server routes to the
     * use-case that arbitrates, so binding it here would close a cycle.
     */
    @Binds
    @Singleton
    abstract fun bindCallClaimArbiter(impl: SessionRegistry): CallClaimArbiter

    @Binds
    @Singleton
    abstract fun bindPairingManager(impl: PairingManagerImpl): PairingManager

    @Binds
    @Singleton
    abstract fun bindIdentityStore(impl: IdentityStoreImpl): IdentityStore
}
