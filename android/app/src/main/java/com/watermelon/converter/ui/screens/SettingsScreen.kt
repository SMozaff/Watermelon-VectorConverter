// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.selection.selectable
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.platform.LocalContext
import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import com.watermelon.converter.logging.AppLogger
import com.watermelon.converter.util.OutputDestination
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavController
import com.watermelon.converter.data.prefs.ThemeMode
import com.watermelon.converter.viewmodel.SettingsViewModel
import kotlin.math.roundToInt

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(nav: NavController, vm: SettingsViewModel = viewModel()) {
    val settings by vm.settings.collectAsState()
    val ctx = LocalContext.current

    val destPicker = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocumentTree()
    ) { uri -> if (uri != null) vm.setOutputDestination(uri) }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Settings") },
            )
        }
    ) { pad ->
        Column(
            Modifier.fillMaxSize().padding(pad).verticalScroll(rememberScrollState()).padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(24.dp),
        ) {
            Text("Preview resolution", style = MaterialTheme.typography.titleLarge)
            Text("${settings.previewPx} px")
            Slider(
                value = settings.previewPx.toFloat(),
                onValueChange = { vm.setPreviewPx(it.roundToInt()) },
                valueRange = 64f..1024f,
                steps = 0,
            )

            HorizontalDivider()

            Text("Theme", style = MaterialTheme.typography.titleLarge)
            ThemeMode.entries.forEach { mode ->
                Row(
                    Modifier.fillMaxWidth().selectable(
                        selected = settings.themeMode == mode,
                        onClick = { vm.setThemeMode(mode) },
                    ).padding(vertical = 4.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    RadioButton(selected = settings.themeMode == mode, onClick = { vm.setThemeMode(mode) })
                    Spacer(Modifier.width(8.dp))
                    Text(mode.name.lowercase().replaceFirstChar { it.uppercase() })
                }
            }

            HorizontalDivider()

            Text("Output destination", style = MaterialTheme.typography.titleLarge)
            Text(
                "All converted files are saved here. Tap to change.",
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.height(4.dp))
            Text(
                OutputDestination.displayLabel(ctx, settings.outputDestinationUri),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.primary,
            )
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                Button(onClick = { destPicker.launch(null) }) { Text("Change folder") }
                if (settings.outputDestinationUri != null) {
                    OutlinedButton(onClick = { vm.clearOutputDestination() }) {
                        Text("Reset to default", color = MaterialTheme.colorScheme.error)
                    }
                }
            }

            HorizontalDivider()

            Text("Navigation", style = MaterialTheme.typography.titleLarge)
            Row(
                Modifier.fillMaxWidth().padding(vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text("Slide animation", modifier = Modifier.weight(1f))
                Switch(
                    checked = settings.slideAnimation,
                    onCheckedChange = { vm.setSlideAnimation(it) },
                )
            }

            HorizontalDivider()

            Text("File preview", style = MaterialTheme.typography.titleLarge)
            Row(
                Modifier.fillMaxWidth().padding(vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text("Show file properties")
                    Text(
                        "Displays name, size, dimensions and structure below the preview image.",
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Switch(
                    checked = settings.showFileProperties,
                    onCheckedChange = { vm.setShowFileProperties(it) },
                )
            }

            HorizontalDivider()

            Text("Diagnostics", style = MaterialTheme.typography.titleLarge)
            val ctx = LocalContext.current
            Button(
                onClick = {
                    val saved = AppLogger.dump("manual save from Settings")
                    Toast.makeText(
                        ctx,
                        if (saved != null) "Saved to Downloads/WVGC/$saved" else "Could not save logs",
                        Toast.LENGTH_LONG,
                    ).show()
                },
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Save logs to Downloads") }

            HorizontalDivider()

            // History row
            Row(
                Modifier
                    .fillMaxWidth()
                    .clickable { nav.navigate(com.watermelon.converter.Routes.HISTORY) }
                    .padding(vertical = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text("History", fontWeight = androidx.compose.ui.text.font.FontWeight.SemiBold, color = MaterialTheme.colorScheme.onSurface)
                    Text(
                        "Past conversions, successes and errors",
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Text("›", fontSize = 20.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }

            HorizontalDivider()

            // About row
            Row(
                Modifier
                    .fillMaxWidth()
                    .clickable { nav.navigate(com.watermelon.converter.Routes.ABOUT) }
                    .padding(vertical = 16.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(Modifier.weight(1f)) {
                    Text("About", fontWeight = androidx.compose.ui.text.font.FontWeight.SemiBold, color = MaterialTheme.colorScheme.onSurface)
                    Text(
                        "Developer info, version, license",
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Text("›", fontSize = 20.sp, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}