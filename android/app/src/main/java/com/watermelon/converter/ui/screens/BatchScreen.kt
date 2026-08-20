// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

package com.watermelon.converter.ui.screens

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.navigation.NavController
import com.watermelon.converter.Routes
import com.watermelon.converter.ui.components.SeedBar
import com.watermelon.converter.ui.sharedGraphViewModel
import com.watermelon.converter.viewmodel.BatchPreflight
import com.watermelon.converter.viewmodel.BatchProgress
import com.watermelon.converter.viewmodel.BatchUiState
import com.watermelon.converter.viewmodel.BatchViewModel
import com.watermelon.converter.viewmodel.ReverseBatchViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BatchScreen(
    nav: NavController,
    reverse: Boolean = false,
    vm: BatchViewModel = nav.sharedGraphViewModel(),
    revVm: ReverseBatchViewModel = nav.sharedGraphViewModel(),
) {
    val state by (if (reverse) revVm.state else vm.state).collectAsState()
    val reportSaveState by (if (reverse) revVm.reportSaveState else vm.reportSaveState).collectAsState()
    val onCancel: () -> Unit = { if (reverse) revVm.cancel() else vm.cancel() }
    val onReset: () -> Unit = { if (reverse) revVm.reset() else vm.reset() }
    val onRetry: () -> Unit = { if (reverse) revVm.retry() else vm.retry() }
    val onConfirm: () -> Unit = { if (reverse) revVm.confirmPreflight() else vm.confirmPreflight() }
    val onSaveReport: () -> Unit = { if (reverse) revVm.saveReport() else vm.saveReport() }
    val onPrepare: (android.net.Uri) -> Unit = { uri -> if (reverse) revVm.prepareZip(uri) else vm.prepareZip(uri) }
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if (uri != null) onPrepare(uri)
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(if (reverse) "Batch convert (XML → SVG)" else "Batch convert (SVG → XML)") },
                navigationIcon = { TextButton(onClick = { nav.popBackStack() }) { Text("Back") } },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            when (val current = state) {
                BatchUiState.Idle -> BatchPicker(reverse) { picker.launch(arrayOf("application/zip", "application/octet-stream")) }
                is BatchUiState.Preflight -> BatchPreflightCard(current.details, onConfirm, onReset)
                is BatchUiState.Working -> BatchProgressCard(current.progress, onCancel)
                is BatchUiState.Done -> BatchCompletionCard(current, reportSaveState, { nav.navigate(Routes.EXPORT) }, onSaveReport, onReset)
                is BatchUiState.Cancelled -> BatchRecoveryCard("Batch cancelled", current.message, onRetry, onReset)
                is BatchUiState.Error -> BatchRecoveryCard("Batch needs attention", current.message, onRetry, onReset)
            }
        }
    }
}

@Composable
private fun BatchPicker(reverse: Boolean, onPick: () -> Unit) {
    Text(if (reverse) "Choose a ZIP containing VectorDrawable XML files." else "Choose a ZIP containing SVG files.", style = MaterialTheme.typography.bodyLarge)
    Text("Before conversion, Watermelon will show eligible files, exclusions, output name, and save location.", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    Button(onClick = onPick, modifier = Modifier.fillMaxWidth()) { Text("Choose ZIP file") }
}

@Composable
private fun BatchPreflightCard(preflight: BatchPreflight, onConfirm: () -> Unit, onChooseAnother: () -> Unit) {
    Text("Ready to review", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text(preflight.directionLabel, color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.SemiBold)
    PreflightLine("Source", preflight.inputLabel)
    PreflightLine("Eligible", "${preflight.eligibleCount} of ${preflight.totalCount} files")
    PreflightLine("Input size", "${preflight.inputBytes} bytes")
    PreflightLine("Output archive", preflight.outputName)
    PreflightLine("Save location", preflight.destinationLabel)
    if (preflight.rejected.isNotEmpty()) {
        HorizontalDivider()
        Text("${preflight.rejected.size} file${if (preflight.rejected.size == 1) " is" else "s are"} excluded", color = MaterialTheme.colorScheme.error)
        preflight.rejected.take(5).forEach { rejected -> Text("• ${rejected.name}: ${rejected.reason}", style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant) }
        if (preflight.rejected.size > 5) Text("…and ${preflight.rejected.size - 5} more", style = MaterialTheme.typography.bodySmall)
    }
    Button(onClick = onConfirm, enabled = preflight.eligibleCount > 0, modifier = Modifier.fillMaxWidth()) { Text("Convert ${preflight.eligibleCount} eligible file${if (preflight.eligibleCount == 1) "" else "s"}") }
    OutlinedButton(onClick = onChooseAnother, modifier = Modifier.fillMaxWidth()) { Text("Choose another batch") }
}

@Composable
private fun PreflightLine(label: String, value: String) {
    Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text(label.uppercase(), style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.primary)
        Text(value, style = MaterialTheme.typography.bodyMedium)
    }
}

@Composable
private fun BatchProgressCard(progress: BatchProgress?, onCancel: () -> Unit) {
    Text("Converting batch", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    if (progress == null || progress.total == 0) {
        CircularProgressIndicator()
        Text("Reading batch input…", color = MaterialTheme.colorScheme.onSurfaceVariant)
    } else {
        SeedBar(progress = progress.fileFraction, label = "Current: ${progress.currentName}")
        Spacer(Modifier.height(12.dp))
        SeedBar(progress = progress.totalFraction, label = "Total batch")
        Text("${progress.done} of ${progress.total} files processed", style = MaterialTheme.typography.titleMedium)
    }
    OutlinedButton(onClick = onCancel) { Text("Cancel batch") }
}

@Composable
private fun BatchCompletionCard(
    state: BatchUiState.Done,
    reportSaveState: String?,
    onExport: () -> Unit,
    onSaveReport: () -> Unit,
    onAnother: () -> Unit,
) {
    var showFailures by remember(state.outputName) { mutableStateOf(false) }
    Text("Batch complete", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text("${state.report.succeeded} succeeded · ${state.report.failed} failed", color = MaterialTheme.colorScheme.primary)
    PreflightLine("Output archive", state.outputName)
    PreflightLine("Saved to", state.savedTo)
    Button(onClick = onExport, modifier = Modifier.fillMaxWidth()) { Text("Export ZIP") }
    if (state.report.failed > 0) {
        OutlinedButton(onClick = { showFailures = !showFailures }, modifier = Modifier.fillMaxWidth()) { Text(if (showFailures) "Hide failures" else "View ${state.report.failed} failure${if (state.report.failed == 1) "" else "s"}") }
        if (showFailures) state.report.rejected.forEach { failure -> Text("• ${failure.name}: ${failure.errorMessage ?: "Unknown reason"}", color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
    }
    OutlinedButton(onClick = onAnother, modifier = Modifier.fillMaxWidth()) { Text("Start another batch") }
    TextButton(onClick = onSaveReport) { Text("Save report") }
    if (reportSaveState != null) Text(reportSaveState, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
}

@Composable
private fun BatchRecoveryCard(title: String, message: String, onRetry: () -> Unit, onAnother: () -> Unit) {
    Text(title, style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.Bold)
    Text(message, color = MaterialTheme.colorScheme.error)
    Button(onClick = onRetry, modifier = Modifier.fillMaxWidth()) { Text("Review and retry") }
    OutlinedButton(onClick = onAnother, modifier = Modifier.fillMaxWidth()) { Text("Choose another batch") }
}
