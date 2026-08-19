// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import com.watermelon.converter.data.prefs.ThemeMode
import com.watermelon.converter.logging.AppLogger
import com.watermelon.converter.ui.MainPager
import com.watermelon.converter.ui.screens.*
import com.watermelon.converter.ui.theme.WatermelonTheme
import com.watermelon.converter.viewmodel.SettingsViewModel

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        AppLogger.log("MainActivity", "onCreate start")
        setContent {
            val settingsVm: SettingsViewModel = viewModel()
            val settings by settingsVm.settings.collectAsState()
            val dark = when (settings.themeMode) {
                ThemeMode.LIGHT -> false
                ThemeMode.DARK -> true
                ThemeMode.SYSTEM -> isSystemInDarkTheme()
            }
            WatermelonTheme(darkTheme = dark) {
                val showSplash = rememberSaveable { mutableStateOf(true) }
                Surface(modifier = Modifier) {
                    if (showSplash.value) {
                        WatermelonSplash(onFinished = { showSplash.value = false })
                    } else {
                        AppNavHost(settingsVm)
                    }
                }
            }
        }
        AppLogger.log("MainActivity", "onCreate done — UI set")
    }

    override fun onResume() {
        super.onResume()
        AppLogger.log("MainActivity", "onResume")
    }

    override fun onPause() {
        super.onPause()
        AppLogger.log("MainActivity", "onPause")
    }
}

object Routes {
    const val PAGER = "pager"          // Home + Files + Settings (swipeable)
    const val PREVIEW = "preview"
    const val BATCH = "batch"
    const val BATCH_REVERSE = "batch_reverse"
    const val EXPORT = "export"
    const val HISTORY = "history"
    const val ABOUT = "about"
}

@androidx.compose.runtime.Composable
fun AppNavHost(settingsVm: SettingsViewModel) {
    val nav = rememberNavController()
    NavHost(navController = nav, startDestination = Routes.PAGER) {
        composable(Routes.PAGER) { MainPager(nav, settingsVm) }
        composable(Routes.PREVIEW) { PreviewScreen(nav) }
        composable(Routes.BATCH) { BatchScreen(nav, reverse = false) }
        composable(Routes.BATCH_REVERSE) { BatchScreen(nav, reverse = true) }
        composable(Routes.EXPORT) { ExportScreen(nav) }
        composable(Routes.HISTORY) { HistoryScreen(nav) }
        composable(Routes.ABOUT) { AboutScreen(nav) }
    }
}
