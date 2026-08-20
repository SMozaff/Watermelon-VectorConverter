// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.ui.screens

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.navigation.NavController
import com.watermelon.converter.Routes
import com.watermelon.converter.ui.sharedGraphViewModel
import com.watermelon.converter.ui.theme.*
import com.watermelon.converter.viewmodel.BatchUiState
import com.watermelon.converter.viewmodel.BatchViewModel
import com.watermelon.converter.viewmodel.ConversionViewModel
import com.watermelon.converter.viewmodel.ConvertUiState
import com.watermelon.converter.viewmodel.ReverseBatchViewModel
import com.watermelon.converter.viewmodel.ReverseConversionViewModel
import com.watermelon.converter.viewmodel.ReverseConvertUiState
import com.watermelon.converter.viewmodel.SettingsViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import com.watermelon.converter.util.OutputDestination
import androidx.compose.ui.platform.LocalContext

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ExportScreen(
    nav: NavController,
    convVm: ConversionViewModel = nav.sharedGraphViewModel(),
    batchVm: BatchViewModel = nav.sharedGraphViewModel(),
    revConvVm: ReverseConversionViewModel = nav.sharedGraphViewModel(),
    revBatchVm: ReverseBatchViewModel = nav.sharedGraphViewModel(),
    settingsVm: SettingsViewModel = viewModel(),
) {
    val convState  by convVm.state.collectAsState()
    val batchState by batchVm.state.collectAsState()
    val revConvState  by revConvVm.state.collectAsState()
    val revBatchState by revBatchVm.state.collectAsState()
    val settings   by settingsVm.settings.collectAsState()
    val ctx = LocalContext.current

    val hasSingle    = convState  is ConvertUiState.Done
    val hasBatch     = batchState is BatchUiState.Done
    val hasRevSingle = revConvState  is ReverseConvertUiState.Done
    val hasRevBatch  = revBatchState is BatchUiState.Done
    val hasAny = hasSingle || hasBatch || hasRevSingle || hasRevBatch
    val hasDestination = settings.outputDestinationUri != null

    var exportDone by remember { mutableStateOf(false) }

    // Folder picker — only needed when no Settings destination is configured.
    val treePicker = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocumentTree()
    ) { treeUri ->
        if (treeUri != null) {
            if (hasSingle) convVm.export(treeUri)
            if (hasBatch)  batchVm.export(treeUri)
            if (hasRevSingle) revConvVm.export(treeUri)
            if (hasRevBatch)  revBatchVm.export(treeUri)
            exportDone = true
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Export", fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.onBackground) },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.background,
                ),
            )
        },
        containerColor = MaterialTheme.colorScheme.background,
    ) { pad ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(pad)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 24.dp, vertical = 32.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            if (!hasAny) {
                Text("Nothing to export.", color = MaterialTheme.colorScheme.onSurfaceVariant, textAlign = androidx.compose.ui.text.style.TextAlign.Center)
            } else if (exportDone) {
                Text("✓  Exported successfully.", color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.Bold, fontSize = 18.sp)
                Spacer(Modifier.height(8.dp))
                if (hasDestination) {
                    Text(
                        "Saved to: ${OutputDestination.displayLabel(ctx, settings.outputDestinationUri)}",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        fontSize = 13.sp,
                    )
                }
            } else {
                Text(
                    when {
                        hasBatch    -> "Batch result ready to export as ZIP."
                        hasSingle   -> "VectorDrawable ready to export."
                        hasRevBatch -> "Batch result ready to export as ZIP."
                        hasRevSingle -> "SVG ready to export."
                        else        -> ""
                    },
                    style = MaterialTheme.typography.titleLarge,
                    color = MaterialTheme.colorScheme.onBackground,
                )

                Spacer(Modifier.height(8.dp))

                if (hasDestination) {
                    // Settings destination configured — export directly, no picker.
                    Text(
                        "Will save to:\n${OutputDestination.displayLabel(ctx, settings.outputDestinationUri)}",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        fontSize = 13.sp,
                        textAlign = androidx.compose.ui.text.style.TextAlign.Center,
                    )
                    Spacer(Modifier.height(8.dp))
                    Button(
                        onClick = {
                            if (hasSingle) convVm.export()
                            if (hasBatch)  batchVm.exportToSettings()
                            if (hasRevSingle) revConvVm.export()
                            if (hasRevBatch)  revBatchVm.exportToSettings()
                            exportDone = true
                        },
                        modifier = Modifier.fillMaxWidth().height(56.dp),
                        shape = RoundedCornerShape(50),
                        colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary,
                            contentColor = MaterialTheme.colorScheme.onPrimary),
                    ) {
                        Text("Export", fontSize = 16.sp, fontWeight = FontWeight.Bold)
                    }
                    TextButton(onClick = { treePicker.launch(null) }) {
                        Text("Choose a different folder…", color = MaterialTheme.colorScheme.onSurfaceVariant, fontSize = 13.sp)
                    }
                } else {
                    // No Settings destination — use the folder picker.
                    Button(
                        onClick = { treePicker.launch(null) },
                        modifier = Modifier.fillMaxWidth().height(56.dp),
                        shape = RoundedCornerShape(50),
                        colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary,
                            contentColor = MaterialTheme.colorScheme.onPrimary),
                    ) {
                        Text("Choose destination folder", fontSize = 16.sp, fontWeight = FontWeight.Bold)
                    }
                    Text(
                        "Tip: set a default destination in Settings to skip this step.",
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        fontSize = 12.sp,
                        textAlign = androidx.compose.ui.text.style.TextAlign.Center,
                    )
                }
            }

            Spacer(Modifier.height(8.dp))
            OutlinedButton(
                onClick = { nav.navigate(Routes.PAGER) { popUpTo(Routes.PAGER) { inclusive = false } } },
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(50),
            ) {
                Text("Done")
            }
        }
    }
}
