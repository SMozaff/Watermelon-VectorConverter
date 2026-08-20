// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.ui.components

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.watermelon.converter.data.model.BatchReport

/**
 * Inline post-conversion report. Shown on whatever screen triggered the
 * conversion (single or batch) instead of as a separate route. Summarizes
 * what happened and offers a Save-report button.
 */
@Composable
fun ReportPanel(
    report: BatchReport,
    onSaveReport: () -> Unit,
    onDismiss: () -> Unit,
    saveState: String? = null,
    modifier: Modifier = Modifier,
) {
    ElevatedCard(modifier.fillMaxWidth().padding(12.dp)) {
        Column(Modifier.padding(16.dp).verticalScroll(rememberScrollState())) {
            Text("Conversion report", style = MaterialTheme.typography.titleLarge)
            Spacer(Modifier.height(8.dp))

            val rate = if (report.total > 0) report.succeeded.toFloat() / report.total else 0f
            SeedBar(progress = rate, label = "Success rate")
            Spacer(Modifier.height(12.dp))

            StatRow("Total files", report.total.toString())
            StatRow("Succeeded", report.succeeded.toString())
            StatRow("Failed", report.failed.toString())
            StatRow("Input size", humanSize(report.inputBytes))
            StatRow("Output size", humanSize(report.outputBytes))
            StatRow("Time", formatDuration(report.durationMillis))

            if (report.rejected.isNotEmpty()) {
                Spacer(Modifier.height(12.dp))
                Text(
                    "Rejected (${report.rejected.size})",
                    style = MaterialTheme.typography.titleLarge,
                    color = MaterialTheme.colorScheme.error,
                )
                report.rejected.forEach { f ->
                    Spacer(Modifier.height(4.dp))
                    Text(f.name, style = MaterialTheme.typography.bodyLarge)
                    Text(
                        buildString {
                            f.errorCode?.let { append("[$it] ") }
                            append(f.errorMessage ?: "unknown reason")
                        },
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            } else {
                Spacer(Modifier.height(12.dp))
                Text("All files converted successfully.", color = MaterialTheme.colorScheme.primary)
            }

            if (saveState != null) {
                Spacer(Modifier.height(8.dp))
                Text(saveState, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
            }

            Spacer(Modifier.height(16.dp))
            Row {
                OutlinedButton(onClick = onSaveReport, modifier = Modifier.weight(1f)) {
                    Text("Save report")
                }
                Spacer(Modifier.width(12.dp))
                Button(onClick = onDismiss, modifier = Modifier.weight(1f)) {
                    Text("Done")
                }
            }
        }
    }
}

@Composable
private fun StatRow(label: String, value: String) {
    Row(Modifier.fillMaxWidth().padding(vertical = 2.dp)) {
        Text(label, modifier = Modifier.weight(1f), color = MaterialTheme.colorScheme.primary)
        Text(value, fontWeight = FontWeight.SemiBold)
    }
}

private fun humanSize(bytes: Long): String {
    if (bytes < 1024) return "$bytes B"
    val kb = bytes / 1024.0
    if (kb < 1024) return String.format("%.1f KB", kb)
    return String.format("%.2f MB", kb / 1024.0)
}

private fun formatDuration(ms: Long): String {
    if (ms < 1000) return "$ms ms"
    return String.format("%.1f s", ms / 1000.0)
}