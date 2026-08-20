// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.ui.screens

import android.graphics.BitmapFactory
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavController
import com.watermelon.converter.Routes
import com.watermelon.converter.ui.components.VectorPropertiesPanel
import com.watermelon.converter.ui.sharedGraphViewModel
import com.watermelon.converter.ui.theme.*
import com.watermelon.converter.util.ShareUtils
import com.watermelon.converter.viewmodel.ConversionViewModel
import com.watermelon.converter.viewmodel.ConvertUiState
import com.watermelon.converter.viewmodel.ReverseConversionViewModel
import com.watermelon.converter.viewmodel.ReverseConvertUiState
import com.watermelon.converter.viewmodel.SettingsViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PreviewScreen(
    nav: NavController,
    vm: ConversionViewModel = nav.sharedGraphViewModel(),
    revVm: ReverseConversionViewModel = nav.sharedGraphViewModel(),
    settingsVm: SettingsViewModel = viewModel(),
) {
    val state by vm.state.collectAsState()
    val revState by revVm.state.collectAsState()
    val settings by settingsVm.settings.collectAsState()
    // StateFlow delegation cannot be smart-cast by Kotlin. These stable local
    // values keep the state renderer exhaustive and safe across recomposition.
    val forwardState = state
    val reverseState = revState
    val done = forwardState as? ConvertUiState.Done
    val revDone = reverseState as? ReverseConvertUiState.Done
    val forwardActive = forwardState !is ConvertUiState.Idle
    val ctx = LocalContext.current

    val title = when {
        forwardActive && forwardState is ConvertUiState.Working -> forwardState.sourceName
        forwardActive && forwardState is ConvertUiState.Error -> forwardState.sourceName
        done != null -> done.sourceName
        reverseState is ReverseConvertUiState.Working -> reverseState.sourceName
        reverseState is ReverseConvertUiState.Error -> reverseState.sourceName
        revDone != null -> revDone.sourceName
        else -> "Preview"
    }

    Scaffold(
        topBar = {
            TopAppBar(
                navigationIcon = {
                    TextButton(onClick = { nav.popBackStack() }) {
                        Text("Back", color = MaterialTheme.colorScheme.primary)
                    }
                },
                title = {
                    Text(
                        title,
                        fontWeight = FontWeight.SemiBold,
                        color = MaterialTheme.colorScheme.onBackground,
                        maxLines = 1,
                    )
                },
                actions = {
                    if (done != null) {
                        TextButton(onClick = { ShareUtils.copyToClipboard(ctx, "VectorDrawable", done.vdXml) }) {
                            Text("Copy", color = MaterialTheme.colorScheme.primary)
                        }
                        TextButton(onClick = { ShareUtils.shareText(ctx, done.sourceName, done.vdXml) }) {
                            Text("Share", color = MaterialTheme.colorScheme.primary)
                        }
                    } else if (revDone != null) {
                        TextButton(onClick = { ShareUtils.copyToClipboard(ctx, "SVG", revDone.svgXml) }) {
                            Text("Copy", color = MaterialTheme.colorScheme.primary)
                        }
                        TextButton(onClick = { ShareUtils.shareText(ctx, revDone.sourceName, revDone.svgXml) }) {
                            Text("Share", color = MaterialTheme.colorScheme.primary)
                        }
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.background,
                ),
            )
        },
        bottomBar = {
            if (done != null || revDone != null) {
                Row(
                    Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 10.dp),
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    OutlinedButton(
                        onClick = {
                            vm.reset(); revVm.reset()
                            nav.navigate(Routes.PAGER) { popUpTo(Routes.PAGER) { inclusive = false } }
                        },
                        shape = RoundedCornerShape(14.dp),
                        modifier = Modifier.weight(1f),
                    ) { Text("New") }
                    Button(
                        onClick = { nav.navigate(Routes.EXPORT) },
                        shape = RoundedCornerShape(14.dp),
                        modifier = Modifier.weight(1f),
                    ) { Text("Export", fontWeight = FontWeight.Bold) }
                }
            }
        },
        containerColor = MaterialTheme.colorScheme.background,
    ) { pad ->
        when {
            forwardActive && forwardState is ConvertUiState.Working -> {
                ConversionWorking(
                    modifier = Modifier.padding(pad),
                    sourceName = forwardState.sourceName,
                    direction = "SVG → VectorDrawable",
                    onBack = { nav.popBackStack() },
                )
            }
            forwardActive && forwardState is ConvertUiState.Error -> {
                ConversionError(
                    modifier = Modifier.padding(pad),
                    sourceName = forwardState.sourceName,
                    message = forwardState.message,
                    retryAvailable = forwardState.retryAvailable,
                    onRetry = vm::retry,
                    onChooseAnother = {
                        vm.reset()
                        nav.navigate(Routes.PAGER) { popUpTo(Routes.PAGER) { inclusive = false } }
                    },
                )
            }
            !forwardActive && reverseState is ReverseConvertUiState.Working -> {
                ConversionWorking(
                    modifier = Modifier.padding(pad),
                    sourceName = reverseState.sourceName,
                    direction = "VectorDrawable → SVG",
                    onBack = { nav.popBackStack() },
                )
            }
            !forwardActive && reverseState is ReverseConvertUiState.Error -> {
                ConversionError(
                    modifier = Modifier.padding(pad),
                    sourceName = reverseState.sourceName,
                    message = reverseState.message,
                    retryAvailable = reverseState.retryAvailable,
                    onRetry = revVm::retry,
                    onChooseAnother = {
                        revVm.reset()
                        nav.navigate(Routes.PAGER) { popUpTo(Routes.PAGER) { inclusive = false } }
                    },
                )
            }
            done != null -> SuccessContent(
                modifier = Modifier.padding(pad),
                sourceName = done.sourceName,
                outputXml = done.vdXml,
                outputLabel = "VD XML",
                sourcePreviewLabel = "Original SVG",
                sourcePreview = done.svgPreviewPng,
                outputPreviewLabel = "Generated VD",
                outputPreview = done.vdPreviewPng,
                analysisJson = done.analysisJson,
                showFileProperties = settings.showFileProperties,
                isForward = true,
            )
            revDone != null -> SuccessContent(
                modifier = Modifier.padding(pad),
                sourceName = revDone.sourceName,
                outputXml = revDone.svgXml,
                outputLabel = "SVG",
                sourcePreviewLabel = "Original VD",
                sourcePreview = revDone.vdPreviewPng,
                outputPreviewLabel = "Generated SVG",
                outputPreview = revDone.svgPreviewPng,
                analysisJson = revDone.analysisJson,
                showFileProperties = settings.showFileProperties,
                isForward = false,
            )
            else -> PreviewIdle(
                modifier = Modifier.padding(pad),
                onChooseFile = {
                    nav.navigate(Routes.PAGER) { popUpTo(Routes.PAGER) { inclusive = false } }
                },
            )
        }
    }
}

@Composable
private fun PreviewIdle(modifier: Modifier = Modifier, onChooseFile: () -> Unit) {
    Box(modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
        ElevatedCard(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(20.dp),
            colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surface),
        ) {
            Column(
                Modifier.padding(24.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("Ready when you are", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
                Text(
                    "Choose an SVG or VectorDrawable XML file to start a conversion.",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
                Button(onClick = onChooseFile, shape = RoundedCornerShape(14.dp)) {
                    Text("Choose a file")
                }
            }
        }
    }
}

@Composable
private fun ConversionWorking(
    modifier: Modifier = Modifier,
    sourceName: String,
    direction: String,
    onBack: () -> Unit,
) {
    Box(modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
        ElevatedCard(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(20.dp),
            colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surface),
        ) {
            Column(
                Modifier.padding(24.dp).semantics {
                    contentDescription = "Converting $sourceName. Please wait."
                },
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(14.dp),
            ) {
                CircularProgressIndicator(color = MaterialTheme.colorScheme.primary)
                Text("Converting", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
                Text(sourceName, color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.SemiBold)
                Text(
                    "$direction is in progress. Previews are being prepared after conversion.",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
                TextButton(onClick = onBack) { Text("Continue browsing") }
            }
        }
    }
}

@Composable
private fun ConversionError(
    modifier: Modifier = Modifier,
    sourceName: String,
    message: String,
    retryAvailable: Boolean,
    onRetry: () -> Unit,
    onChooseAnother: () -> Unit,
) {
    Box(modifier.fillMaxSize().padding(24.dp), contentAlignment = Alignment.Center) {
        ElevatedCard(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(20.dp),
            colors = CardDefaults.elevatedCardColors(
                containerColor = MaterialTheme.colorScheme.errorContainer,
                contentColor = MaterialTheme.colorScheme.onErrorContainer,
            ),
        ) {
            Column(
                Modifier.padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Text("Conversion could not finish", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.Bold)
                Text(sourceName, fontWeight = FontWeight.SemiBold)
                Text(message)
                Text(
                    "No output was saved. Retry this file or choose another one.",
                    style = MaterialTheme.typography.bodyMedium,
                )
                Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    if (retryAvailable) {
                        Button(onClick = onRetry, shape = RoundedCornerShape(14.dp)) { Text("Retry") }
                    }
                    OutlinedButton(onClick = onChooseAnother, shape = RoundedCornerShape(14.dp)) {
                        Text("Choose another")
                    }
                }
            }
        }
    }
}

@Composable
private fun SuccessContent(
    modifier: Modifier = Modifier,
    sourceName: String,
    outputXml: String,
    outputLabel: String,
    sourcePreviewLabel: String,
    sourcePreview: ByteArray?,
    outputPreviewLabel: String,
    outputPreview: ByteArray?,
    analysisJson: String?,
    showFileProperties: Boolean,
    isForward: Boolean,
) {
    Column(
        modifier = modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp, vertical = 16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        ConversionReport(sourceName, outputXml, outputLabel = outputLabel)

        Text(
            "Both previews are approximate (rendered via resvg, not Android's pipeline).",
            style = MaterialTheme.typography.labelLarge,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Row(
            Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            PreviewTile(sourcePreviewLabel, sourcePreview, Modifier.weight(1f))
            PreviewTile(outputPreviewLabel, outputPreview, Modifier.weight(1f))
        }

        if (showFileProperties && analysisJson != null) {
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            val props = remember(analysisJson) {
                runCatching {
                    if (isForward) {
                        com.watermelon.converter.data.model.VectorProperties.fromJson(
                            name = sourceName,
                            json = analysisJson,
                        )
                    } else {
                        com.watermelon.converter.data.model.VectorProperties.fromJson(
                            name = sourceName,
                            json = analysisJson,
                        )
                    }
                }.getOrNull()
            }
            if (props != null) VectorPropertiesPanel(props)
        }

        var expanded by remember { mutableStateOf(false) }
        ElevatedCard(
            Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(16.dp),
            colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surface),
        ) {
            Column(Modifier.padding(14.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        outputLabel,
                        style = MaterialTheme.typography.titleMedium,
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(onClick = { expanded = !expanded }) {
                        Text(if (expanded) "Collapse" else "Inspect")
                    }
                }
                if (expanded) {
                    Spacer(Modifier.height(8.dp))
                    Text(
                        outputXml,
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace,
                    )
                }
            }
        }
    }
}

@Composable
private fun ConversionReport(sourceName: String, outputXml: String, outputLabel: String) {
    val lineCount = outputXml.lines().size
    val sizeKb = "%.1f KB".format(outputXml.toByteArray().size / 1024.0)

    ElevatedCard(
        Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.elevatedCardColors(
            containerColor = MaterialTheme.colorScheme.primaryContainer,
            contentColor = MaterialTheme.colorScheme.onPrimaryContainer,
        ),
    ) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Text("Conversion successful", fontWeight = FontWeight.Bold, fontSize = 16.sp)
            ReportRow("Source file", sourceName)
            ReportRow("Output size", sizeKb)
            ReportRow("$outputLabel lines", lineCount.toString())
        }
    }
}

@Composable
private fun ReportRow(label: String, value: String) {
    Row(Modifier.fillMaxWidth()) {
        Text(
            label,
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.width(100.dp),
        )
        Text(value, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurface)
    }
}

@Composable
private fun PreviewTile(label: String, png: ByteArray?, modifier: Modifier = Modifier) {
    OutlinedCard(
        modifier,
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.outlinedCardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(
            Modifier.fillMaxWidth().padding(10.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(label, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.height(8.dp))
            if (png != null) {
                val bmp = remember(png) { BitmapFactory.decodeByteArray(png, 0, png.size) }
                if (bmp != null) {
                    Box(
                        Modifier.size(120.dp).background(MaterialTheme.colorScheme.surfaceVariant),
                        contentAlignment = Alignment.Center,
                    ) {
                        Image(bmp.asImageBitmap(), contentDescription = label, modifier = Modifier.size(120.dp))
                    }
                } else {
                    Text("Preview unavailable", textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            } else {
                Text("Preview unavailable", textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}
