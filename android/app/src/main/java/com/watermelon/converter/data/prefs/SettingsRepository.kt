// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.data.prefs

import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map

val Context.dataStore: DataStore<Preferences> by preferencesDataStore(name = "wvgc_settings")

enum class ThemeMode { SYSTEM, LIGHT, DARK }

data class AppSettings(
    val previewPx: Int = 256,
    val themeMode: ThemeMode = ThemeMode.SYSTEM,
    val slideAnimation: Boolean = true,
    /** SAF tree URI string for custom output destination, null = use default WvgcPaths. */
    val outputDestinationUri: String? = null,
)

class SettingsRepository(private val context: Context) {
    private object Keys {
        val PREVIEW_PX = intPreferencesKey("preview_px")
        val THEME_MODE = stringPreferencesKey("theme_mode")
        val SLIDE_ANIMATION = booleanPreferencesKey("slide_animation")
        val OUTPUT_DEST_URI = stringPreferencesKey("output_destination_uri")
    }

    val settings: Flow<AppSettings> = context.dataStore.data.map { p ->
        AppSettings(
            previewPx = (p[Keys.PREVIEW_PX] ?: 256).coerceIn(16, 2048),
            themeMode = runCatching { ThemeMode.valueOf(p[Keys.THEME_MODE] ?: "SYSTEM") }
                .getOrDefault(ThemeMode.SYSTEM),
            slideAnimation = p[Keys.SLIDE_ANIMATION] ?: true,
            outputDestinationUri = p[Keys.OUTPUT_DEST_URI],
        )
    }

    suspend fun setPreviewPx(px: Int) {
        context.dataStore.edit { it[Keys.PREVIEW_PX] = px.coerceIn(16, 2048) }
    }

    suspend fun setThemeMode(mode: ThemeMode) {
        context.dataStore.edit { it[Keys.THEME_MODE] = mode.name }
    }

    suspend fun setSlideAnimation(enabled: Boolean) {
        context.dataStore.edit { it[Keys.SLIDE_ANIMATION] = enabled }
    }

    suspend fun setOutputDestination(treeUri: android.net.Uri) {
        // Persist SAF permission so it survives restarts.
        try {
            context.contentResolver.takePersistableUriPermission(
                treeUri,
                android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION or
                    android.content.Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
            )
        } catch (_: SecurityException) {}
        context.dataStore.edit { it[Keys.OUTPUT_DEST_URI] = treeUri.toString() }
    }

    suspend fun clearOutputDestination() {
        context.dataStore.edit { it.remove(Keys.OUTPUT_DEST_URI) }
    }
}