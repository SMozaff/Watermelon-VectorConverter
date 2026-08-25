// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.watermelon.converter.data.prefs.AppSettings
import com.watermelon.converter.data.prefs.SettingsRepository
import com.watermelon.converter.data.prefs.ThemeMode
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

class SettingsViewModel(app: Application) : AndroidViewModel(app) {
    private val repo = SettingsRepository(app.applicationContext)

    val settings: StateFlow<AppSettings> = repo.settings.stateIn(
        viewModelScope, SharingStarted.WhileSubscribed(5000), AppSettings()
    )

    fun setPreviewPx(px: Int) = viewModelScope.launch { repo.setPreviewPx(px) }
    fun setThemeMode(mode: ThemeMode) = viewModelScope.launch { repo.setThemeMode(mode) }
    fun setSlideAnimation(enabled: Boolean) = viewModelScope.launch { repo.setSlideAnimation(enabled) }
    fun setOutputDestination(uri: android.net.Uri) = viewModelScope.launch { repo.setOutputDestination(uri) }
    fun clearOutputDestination() = viewModelScope.launch { repo.clearOutputDestination() }
}