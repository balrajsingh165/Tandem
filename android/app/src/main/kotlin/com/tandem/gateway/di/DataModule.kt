/**
 * Hilt module for persistence: provides TandemDatabase, DAOs, DataStore, and
 * binds CallLogRepository, PairedDeviceRepository, and SettingsRepository to
 * their impls. Bindings only; no logic.
 */
package com.tandem.gateway.di

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.preferencesDataStoreFile
import androidx.room.Room
import com.tandem.gateway.calllog.CallLogRepositoryImpl
import com.tandem.gateway.data.PairedDeviceRepositoryImpl
import com.tandem.gateway.data.SettingsRepositoryImpl
import com.tandem.gateway.data.db.PairedDesktopDao
import com.tandem.gateway.data.db.TandemDatabase
import com.tandem.gateway.domain.port.CallLogRepository
import com.tandem.gateway.domain.port.PairedDeviceRepository
import com.tandem.gateway.domain.port.SettingsRepository
import dagger.Binds
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
object DataProvidersModule {

    @Provides
    @Singleton
    fun provideDatabase(@ApplicationContext context: Context): TandemDatabase =
        Room.databaseBuilder(context, TandemDatabase::class.java, TandemDatabase.NAME).build()

    @Provides
    @Singleton
    fun providePairedDesktopDao(database: TandemDatabase): PairedDesktopDao =
        database.pairedDesktopDao()

    @Provides
    @Singleton
    fun provideDataStore(@ApplicationContext context: Context): DataStore<Preferences> =
        PreferenceDataStoreFactory.create {
            context.preferencesDataStoreFile(SETTINGS_STORE_NAME)
        }

    private const val SETTINGS_STORE_NAME = "tandem-settings"
}

@Module
@InstallIn(SingletonComponent::class)
abstract class DataModule {

    @Binds
    @Singleton
    abstract fun bindCallLogRepository(impl: CallLogRepositoryImpl): CallLogRepository

    @Binds
    @Singleton
    abstract fun bindPairedDeviceRepository(
        impl: PairedDeviceRepositoryImpl,
    ): PairedDeviceRepository

    @Binds
    @Singleton
    abstract fun bindSettingsRepository(impl: SettingsRepositoryImpl): SettingsRepository
}
