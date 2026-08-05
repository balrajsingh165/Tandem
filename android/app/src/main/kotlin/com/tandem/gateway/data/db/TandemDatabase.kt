/**
 * Room database (tandem.db, v1) hosting the paired-desktop table. Schema DDL and
 * migration policy documented in docs/09-data-models.md.
 */
package com.tandem.gateway.data.db

import androidx.room.Database
import androidx.room.RoomDatabase

// Schema export stays off until a migration exists to validate against; Room
// warns otherwise on every build.
@Database(
    entities = [PairedDesktopEntity::class],
    version = TandemDatabase.VERSION,
    exportSchema = false,
)
abstract class TandemDatabase : RoomDatabase() {

    abstract fun pairedDesktopDao(): PairedDesktopDao

    companion object {
        const val VERSION: Int = 1
        const val NAME: String = "tandem.db"
    }
}
