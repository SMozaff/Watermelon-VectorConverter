// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.ui.screens

import android.graphics.BitmapFactory
import androidx.compose.animation.Crossfade
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.window.Dialog
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavController
import com.watermelon.converter.data.files.FileKind
import com.watermelon.converter.data.files.FileNode
import com.watermelon.converter.data.files.StorageRoot
import com.watermelon.converter.data.files.TypeFilter
import com.watermelon.converter.ui.components.FileIconWithLabel
import com.watermelon.converter.ui.components.FolderIcon
import com.watermelon.converter.ui.components.VectorPropertiesPanel
import com.watermelon.converter.ui.components.WatermelonLoader
import com.watermelon.converter.ui.theme.*
import com.watermelon.converter.viewmodel.FileBrowserBatchState
import com.watermelon.converter.viewmodel.FileManagerViewModel
import com.watermelon.converter.viewmodel.PreviewState
import com.watermelon.converter.viewmodel.RowNode
import com.watermelon.converter.viewmodel.TreeRow
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

private val dateFmt = SimpleDateFormat("MMM d, yyyy • h:mm a", Locale.getDefault())
private fun fmtDate(ms: Long) = dateFmt.format(Date(ms))
private fun fmtSize(b: Long) = when {
    b < 1024 -> "$b B"
    b < 1024 * 1024 -> "%.0f KB".format(b / 1024.0)
    else -> "%.1f MB".format(b / (1024.0 * 1024.0))
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FilesScreen(
    nav: NavController,
    vm: FileManagerViewModel = viewModel(),
    settingsVm: com.watermelon.converter.viewmodel.SettingsViewModel = viewModel(),
) {
    val settings by settingsVm.settings.collectAsState()
    val rows by vm.rows.collectAsState()
    val filter by vm.filter.collectAsState()
    val preview by vm.preview.collectAsState()
    val previewedFile by vm.previewedFile.collectAsState()
    val query by vm.query.collectAsState()
    val searchResults by vm.searchResults.collectAsState()
    val fileBatch by vm.fileBatch.collectAsState()
    val selectionMode by vm.selectionMode.collectAsState()
    val selected by vm.selected.collectAsState()
    val opStatus by vm.opStatus.collectAsState()
    val loading by vm.loading.collectAsState()
    val properties by vm.properties.collectAsState()

    var fullScreen by remember { mutableStateOf(false) }
    var showRenameDialog by remember { mutableStateOf(false) }
    var showDeleteConfirm by remember { mutableStateOf(false) }
    var pendingMove by remember { mutableStateOf(false) }

    var showFolderChooser by remember { mutableStateOf(false) }

    val hasPermission by vm.hasPermission.collectAsState()

    val lifecycleOwner = LocalLifecycleOwner.current
    DisposableEffect(lifecycleOwner) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_RESUME) vm.recheckPermission()
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }

    val currentDir by vm.currentDir.collectAsState()

    if (!hasPermission) {
        PermissionRationale(
            onGrant = { vm.openPermissionSettings() },
            onUseSystemPicker = { nav.navigate(com.watermelon.converter.Routes.PAGER) },
        )
        return
    }

    if (showFolderChooser) {
        FolderChooserDialog(
            storageRoots = vm.storageRoots.collectAsState().value,
            onDismiss = { showFolderChooser = false },
            onSelected = { dir ->
                showFolderChooser = false
                vm.copyOrMoveSelectedTo(dir, pendingMove)
            },
        )
    }

    Box(Modifier.fillMaxSize().background(androidx.compose.material3.MaterialTheme.colorScheme.background)) {
        Column(Modifier.fillMaxSize()) {
            // ── Header ──
            if (selectionMode) {
                SelectionTopBar(
                    count = selected.size,
                    onExit = { vm.exitSelection() },
                    onSelectAllSvg = { vm.selectAllSvg() },
                    onSelectAllXml = { vm.selectAllXml() },
                )
            } else {
                FilesTopBar(
                    query = query,
                    onQueryChange = { vm.setQuery(it) },
                    filter = filter,
                    onFilterChange = { svg, xml -> vm.setFilter(svg, xml) },
                    showBack = currentDir != null,
                    onBack = { vm.goToStorageRoot() },
                    onStartBatchSelection = { vm.startBatchSelection() },
                )
            }

            // ── Content ──
            val showingSearch = query.isNotBlank()
            val displayRows = if (showingSearch) {
                searchResults.map { TreeRow(RowNode.Entry(it), 0, false) }
            } else rows

            when {
                loading -> {
                    Box(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                        WatermelonLoader(size = 56.dp)
                    }
                }
                displayRows.isEmpty() -> {
                    Box(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
                        Text(
                            when {
                                showingSearch -> "No matches found."
                                currentDir == null -> "No storage found."
                                else -> "No SVG or XML files here."
                            },
                            color = SlateGray,
                            textAlign = TextAlign.Center,
                        )
                    }
                }
                else -> {
                    LazyColumn(Modifier.weight(1f)) {
                        items(
                            displayRows,
                            key = { row ->
                                when (val r = row.row) {
                                    is RowNode.Root -> "root:${r.storageRoot.root.absolutePath}"
                                    is RowNode.Entry -> r.node.absolutePath
                                }
                            },
                        ) { row ->
                            when (val r = row.row) {
                                is RowNode.Root -> {
                                    StorageRootRow(
                                        storageRoot = r.storageRoot,
                                        onTap = { vm.tapRoot(r.storageRoot) },
                                    )
                                }
                                is RowNode.Entry -> {
                                    val node = r.node
                                    FileRow(
                                        row = row,
                                        node = node,
                                        isSelected = selected.contains(node.absolutePath),
                                        selectionMode = selectionMode,
                                        onTap = {
                                            if (node.isDirectory) vm.toggleDir(node)
                                            else if (selectionMode) vm.toggleSelect(node)
                                            else vm.preview(node)
                                        },
                                        onLongPress = { if (!node.isDirectory) vm.startSelection(node) },
                                        onMenuAction = { action ->
                                            when (action) {
                                                "rename" -> showRenameDialog = true.also { vm.startSelection(node) }
                                                "delete" -> showDeleteConfirm = true.also { vm.startSelection(node) }
                                                "copy" -> { vm.startSelection(node); pendingMove = false; showFolderChooser = true }
                                                "move" -> { vm.startSelection(node); pendingMove = true; showFolderChooser = true }
                                            }
                                        },
                                    )
                                }
                            }
                        }
                    }
                }
            }

            // ── Preview pane ──
            if (preview !is PreviewState.Empty && !selectionMode) {
                HorizontalDivider(color = Color(0xFFE2E8F0))
                PreviewPane(
                    state = preview,
                    onExpand = { fullScreen = true },
                    onClose = { vm.closePreview() },
                    properties = properties,
                    showProperties = settings.showFileProperties,
                )
            }
        }

        if (selectionMode && selected.isNotEmpty() && fileBatch is com.watermelon.converter.viewmodel.FileBrowserBatchState.Idle) {
            Box(Modifier.fillMaxSize().padding(16.dp), contentAlignment = Alignment.BottomEnd) {
                FileOpsFab(
                    onConvertSvg = { vm.prepareSelectedBatch(com.watermelon.converter.viewmodel.FileBatchDirection.SvgToVector) },
                    onConvertXml = { vm.prepareSelectedBatch(com.watermelon.converter.viewmodel.FileBatchDirection.VectorToSvg) },
                    onMove = { pendingMove = true; showFolderChooser = true },
                    onCopy = { pendingMove = false; showFolderChooser = true },
                    onRename = { showRenameDialog = true },
                    onDelete = { showDeleteConfirm = true },
                )
            }
        }
    }

    // ── Fullscreen preview ──
    if (fullScreen && preview !is PreviewState.Empty) {
        FullScreenPreview(state = preview, onClose = { fullScreen = false })
    }

    FileBrowserBatchDialog(state = fileBatch, vm = vm)

    if (showRenameDialog) {
        RenameDialog(
            count = selected.size,
            onConfirm = { base -> vm.renameSelected(base); showRenameDialog = false },
            onDismiss = { showRenameDialog = false },
        )
    }

    if (showDeleteConfirm) {
        AlertDialog(
            onDismissRequest = { showDeleteConfirm = false },
            title = { Text("Delete ${selected.size} file${if (selected.size == 1) "" else "s"}?") },
            text = { Text("This permanently deletes the selected file(s) from storage. This can't be undone.") },
            confirmButton = {
                TextButton(onClick = { vm.deleteSelected(); showDeleteConfirm = false }) {
                    Text("Delete", color = WatermelonRed)
                }
            },
            dismissButton = { TextButton(onClick = { showDeleteConfirm = false }) { Text("Cancel") } },
        )
    }

    opStatus?.let { status ->
        LaunchedEffect(status) {
            kotlinx.coroutines.delay(2500)
            vm.dismissOpStatus()
        }
        Box(Modifier.fillMaxSize().padding(16.dp), contentAlignment = Alignment.BottomCenter) {
            Snackbar { Text(status) }
        }
    }
}

@Composable
private fun FileBrowserBatchDialog(state: FileBrowserBatchState, vm: FileManagerViewModel) {
    when (state) {
        FileBrowserBatchState.Idle -> Unit
        is FileBrowserBatchState.Preflight -> AlertDialog(
            onDismissRequest = { vm.dismissFileBatch() },
            title = { Text("Review batch") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(state.details.direction.label, color = MaterialTheme.colorScheme.primary, fontWeight = FontWeight.SemiBold)
                    Text("${state.details.eligibleFiles.size} eligible of ${state.details.selectedCount} selected")
                    Text("Input size: ${fmtSize(state.details.inputBytes)}")
                    Text("Output: ${state.details.outputName}", style = MaterialTheme.typography.bodySmall)
                    Text("Save to: ${state.details.destinationLabel}", style = MaterialTheme.typography.bodySmall)
                    if (state.details.rejected.isNotEmpty()) {
                        Text("${state.details.rejected.size} file${if (state.details.rejected.size == 1) " is" else "s are"} excluded", color = MaterialTheme.colorScheme.error)
                        state.details.rejected.take(4).forEach { item -> Text("• ${item.name}: ${item.reason}", style = MaterialTheme.typography.bodySmall) }
                    }
                }
            },
            confirmButton = { TextButton(onClick = { vm.confirmSelectedBatch() }, enabled = state.details.eligibleFiles.isNotEmpty()) { Text("Convert") } },
            dismissButton = { TextButton(onClick = { vm.dismissFileBatch() }) { Text("Back to selection") } },
        )
        is FileBrowserBatchState.Working -> AlertDialog(
            onDismissRequest = {},
            title = { Text("Converting batch") },
            text = {
                val progress = state.progress
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    if (progress == null) {
                        CircularProgressIndicator()
                        Text("Preparing selected files…")
                    } else {
                        LinearProgressIndicator(progress = { progress.totalFraction }, modifier = Modifier.fillMaxWidth())
                        Text("${progress.done} of ${progress.total}: ${progress.currentName}")
                    }
                }
            },
            confirmButton = { TextButton(onClick = { vm.cancelSelectedBatch() }) { Text("Cancel") } },
        )
        is FileBrowserBatchState.Done -> AlertDialog(
            onDismissRequest = { vm.dismissFileBatch() },
            title = { Text("Batch complete") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("${state.report.succeeded} succeeded · ${state.report.failed} failed", color = MaterialTheme.colorScheme.primary)
                    Text("Saved: ${state.savedTo}", style = MaterialTheme.typography.bodySmall)
                    if (state.report.failed > 0) state.report.rejected.forEach { failure ->
                        Text("• ${failure.name}: ${failure.errorMessage ?: "Unknown reason"}", color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
                    }
                }
            },
            confirmButton = { TextButton(onClick = { vm.dismissFileBatch() }) { Text("Start another batch") } },
        )
        is FileBrowserBatchState.Cancelled -> FileBrowserBatchRecoveryDialog("Batch cancelled", state.message, vm)
        is FileBrowserBatchState.Failed -> FileBrowserBatchRecoveryDialog("Batch needs attention", state.message, vm)
    }
}

@Composable
private fun FileBrowserBatchRecoveryDialog(title: String, message: String, vm: FileManagerViewModel) {
    AlertDialog(
        onDismissRequest = { vm.dismissFileBatch() },
        title = { Text(title) },
        text = { Text(message, color = MaterialTheme.colorScheme.error) },
        confirmButton = { TextButton(onClick = { vm.retrySelectedBatch() }) { Text("Review and retry") } },
        dismissButton = { TextButton(onClick = { vm.dismissFileBatch() }) { Text("Choose another batch") } },
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Top bars
// ─────────────────────────────────────────────────────────────────────────────

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun FilesTopBar(
    query: String,
    onQueryChange: (String) -> Unit,
    filter: TypeFilter,
    onFilterChange: (Boolean, Boolean) -> Unit,
    showBack: Boolean,
    onBack: () -> Unit,
    onStartBatchSelection: () -> Unit,
) {
    Column(
        Modifier
            .fillMaxWidth()
            .background(androidx.compose.material3.MaterialTheme.colorScheme.background)
            .padding(horizontal = 16.dp, vertical = 12.dp),
    ) {
        Box(Modifier.fillMaxWidth()) {
            if (showBack) {
                TextButton(onClick = onBack, modifier = Modifier.align(Alignment.CenterStart)) {
                    Text("‹ Storage", color = FreshTeal, fontWeight = FontWeight.SemiBold)
                }
            }
            Text(
                "Files",
                style = MaterialTheme.typography.titleLarge.copy(
                    fontWeight = FontWeight.Bold,
                    fontSize = 22.sp,
                    color = MaterialTheme.colorScheme.onBackground,
                ),
                modifier = Modifier.fillMaxWidth(),
                textAlign = TextAlign.Center,
            )
            TextButton(onClick = onStartBatchSelection, modifier = Modifier.align(Alignment.CenterEnd)) {
                Text("Batch select", color = FreshTeal, fontWeight = FontWeight.SemiBold)
            }
        }
        Spacer(Modifier.height(12.dp))

        // Search pill
        OutlinedTextField(
            value = query,
            onValueChange = onQueryChange,
            placeholder = { Text("Search files and folders", color = SlateGray) },
            leadingIcon = { Text("⌕", fontSize = 18.sp, color = SlateGray) },
            singleLine = true,
            shape = RoundedCornerShape(50),
            colors = OutlinedTextFieldDefaults.colors(
                focusedBorderColor = FreshTeal,
                unfocusedBorderColor = Color(0xFFE2E8F0),
                focusedContainerColor = PureWhite,
                unfocusedContainerColor = PureWhite,
            ),
            modifier = Modifier.fillMaxWidth().height(52.dp),
        )

        Spacer(Modifier.height(12.dp))

        // Filter switches — SVG and XML
        Row(horizontalArrangement = Arrangement.spacedBy(20.dp)) {
            FilterSwitch(
                label = "SVG",
                checked = filter.showSvg,
                onCheckedChange = { onFilterChange(it, filter.showXml) },
            )
            FilterSwitch(
                label = "XML",
                checked = filter.showXml,
                onCheckedChange = { onFilterChange(filter.showSvg, it) },
            )
        }
        Spacer(Modifier.height(4.dp))
    }
    HorizontalDivider(color = Color(0xFFE2E8F0))
}

@Composable
private fun FilterSwitch(label: String, checked: Boolean, onCheckedChange: (Boolean) -> Unit) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier.clickable { onCheckedChange(!checked) },
    ) {
        Text(
            label,
            color = if (checked) MaterialTheme.colorScheme.onBackground else SlateGray,
            fontWeight = if (checked) FontWeight.SemiBold else FontWeight.Normal,
            fontSize = 14.sp,
        )
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            colors = SwitchDefaults.colors(
                checkedThumbColor = PureWhite,
                checkedTrackColor = FreshTeal,
                uncheckedThumbColor = PureWhite,
                uncheckedTrackColor = SlateGray.copy(alpha = 0.4f),
                uncheckedBorderColor = Color.Transparent,
                checkedBorderColor = Color.Transparent,
            ),
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun SelectionTopBar(
    count: Int,
    onExit: () -> Unit,
    onSelectAllSvg: () -> Unit,
    onSelectAllXml: () -> Unit,
) {
    var menuOpen by remember { mutableStateOf(false) }
    TopAppBar(
        title = { Text("$count selected", fontWeight = FontWeight.SemiBold) },
        navigationIcon = {
            TextButton(onClick = onExit) { Text("Cancel", color = WatermelonRed) }
        },
        actions = {
            Box {
                TextButton(onClick = { menuOpen = true }) { Text("Select all", color = FreshTeal) }
                DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                    DropdownMenuItem(text = { Text("All SVG in folder") }, onClick = { menuOpen = false; onSelectAllSvg() })
                    DropdownMenuItem(text = { Text("All XML in folder") }, onClick = { menuOpen = false; onSelectAllXml() })
                }
            }
        },
        colors = TopAppBarDefaults.topAppBarColors(containerColor = androidx.compose.material3.MaterialTheme.colorScheme.background),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// File row
// ─────────────────────────────────────────────────────────────────────────────

@OptIn(ExperimentalFoundationApi::class)
@Composable
private fun FileRow(
    row: TreeRow,
    node: FileNode,
    isSelected: Boolean,
    selectionMode: Boolean,
    onTap: () -> Unit,
    onLongPress: () -> Unit,
    onMenuAction: (String) -> Unit,
) {
    var menuOpen by remember { mutableStateOf(false) }
    val indentDp = (row.depth * 16).dp

    Row(
        Modifier
            .fillMaxWidth()
            .combinedClickable(onClick = onTap, onLongClick = onLongPress)
            .background(
                if (isSelected) FreshTeal.copy(alpha = 0.10f) else Color.Transparent
            )
            .padding(start = 16.dp + indentDp, end = 12.dp, top = 12.dp, bottom = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        // Selection checkbox
        if (selectionMode && !node.isDirectory) {
            Checkbox(
                checked = isSelected,
                onCheckedChange = { onTap() },
                colors = CheckboxDefaults.colors(checkedColor = FreshTeal),
                modifier = Modifier.size(20.dp),
            )
            Spacer(Modifier.width(10.dp))
        }

        // Icon
        when (node.kind) {
            FileKind.Directory -> FolderIcon(size = 18.dp)
            FileKind.Svg -> FileIconWithLabel("SVG", FreshTeal, size = 18.dp)
            FileKind.Xml -> FileIconWithLabel("XML", DeepNavy, size = 18.dp)
            FileKind.Other -> FileIconWithLabel("—", SlateGray, size = 18.dp)
        }

        Spacer(Modifier.width(14.dp))

        // Name + meta
        Column(Modifier.weight(1f)) {
            Text(
                node.name,
                fontWeight = FontWeight.SemiBold,
                fontSize = 15.sp,
                color = MaterialTheme.colorScheme.onBackground,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                buildString {
                    append(fmtDate(node.lastModified))
                    if (!node.isDirectory && node.sizeBytes > 0) {
                        append(" • ")
                        append(fmtSize(node.sizeBytes))
                    }
                },
                fontSize = 12.sp,
                color = SlateGray,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }

        // Three-dot menu
        if (!selectionMode) {
            Spacer(Modifier.width(4.dp))
            Box {
                TextButton(
                    onClick = { menuOpen = true },
                    contentPadding = PaddingValues(0.dp),
                    modifier = Modifier.size(32.dp),
                ) {
                    Text("⋮", fontSize = 18.sp, color = SlateGray)
                }
                DropdownMenu(expanded = menuOpen, onDismissRequest = { menuOpen = false }) {
                    if (!node.isDirectory) {
                        DropdownMenuItem(text = { Text("Rename") }, onClick = { menuOpen = false; onMenuAction("rename") })
                        DropdownMenuItem(text = { Text("Copy") }, onClick = { menuOpen = false; onMenuAction("copy") })
                        DropdownMenuItem(text = { Text("Move") }, onClick = { menuOpen = false; onMenuAction("move") })
                        DropdownMenuItem(text = { Text("Delete", color = WatermelonRed) }, onClick = { menuOpen = false; onMenuAction("delete") })
                    }
                }
            }
        }
    }
    HorizontalDivider(color = Color(0xFFE2E8F0), thickness = 0.5.dp)
}

@Composable
private fun StorageRootRow(storageRoot: StorageRoot, onTap: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .clickable(onClick = onTap)
            .padding(horizontal = 16.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            if (storageRoot.isPrimary) "\uD83D\uDCF1" else "\uD83D\uDCBE", // phone = internal, SD card = external
            fontSize = 20.sp,
        )
        Spacer(Modifier.width(14.dp))
        Column(Modifier.weight(1f)) {
            Text(
                storageRoot.label,
                fontWeight = FontWeight.SemiBold,
                fontSize = 15.sp,
                color = MaterialTheme.colorScheme.onBackground,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
            Text(
                if (storageRoot.isPrimary) "Internal storage" else "External storage",
                fontSize = 12.sp,
                color = SlateGray,
            )
        }
    }
    HorizontalDivider(color = Color(0xFFE2E8F0), thickness = 0.5.dp)
}

// ─────────────────────────────────────────────────────────────────────────────
// Bottom bars
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// FAB
// ─────────────────────────────────────────────────────────────────────────────

@Composable
private fun FileOpsFab(
    onConvertSvg: () -> Unit,
    onConvertXml: () -> Unit,
    onMove: () -> Unit,
    onCopy: () -> Unit,
    onRename: () -> Unit,
    onDelete: () -> Unit,
) {
    var open by remember { mutableStateOf(false) }
    Box {
        FloatingActionButton(
            onClick = { open = true },
            containerColor = WatermelonRed,
            contentColor = PureWhite,
            shape = CircleShape,
        ) { Text("+", fontSize = 24.sp, fontWeight = FontWeight.Light) }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            DropdownMenuItem(text = { Text("Convert SVG → XML") }, onClick = { open = false; onConvertSvg() })
            DropdownMenuItem(text = { Text("Convert XML → SVG") }, onClick = { open = false; onConvertXml() })
            DropdownMenuItem(text = { Text("Move…") }, onClick = { open = false; onMove() })
            DropdownMenuItem(text = { Text("Copy…") }, onClick = { open = false; onCopy() })
            DropdownMenuItem(text = { Text("Rename…") }, onClick = { open = false; onRename() })
            DropdownMenuItem(text = { Text("Delete…") }, onClick = { open = false; onDelete() })
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Preview pane
// ─────────────────────────────────────────────────────────────────────────────

@Composable
private fun PreviewPane(
    state: PreviewState,
    onExpand: () -> Unit,
    onClose: () -> Unit,
    properties: com.watermelon.converter.data.model.VectorProperties? = null,
    showProperties: Boolean = true,
) {
    Column(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 120.dp, max = 520.dp)
            .background(androidx.compose.material3.MaterialTheme.colorScheme.background)
            .verticalScroll(rememberScrollState())
            .padding(12.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                when (state) {
                    is PreviewState.SvgImage -> state.name
                    is PreviewState.Failed -> state.name
                    else -> "Preview"
                },
                fontWeight = FontWeight.SemiBold,
                fontSize = 15.sp,
                color = MaterialTheme.colorScheme.onBackground,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f),
            )
            TextButton(onClick = onExpand) { Text("Expand", fontSize = 13.sp, color = FreshTeal) }
            TextButton(onClick = onClose) { Text("Close", fontSize = 13.sp, color = SlateGray) }
        }
        Spacer(Modifier.height(8.dp))
        Box(
            Modifier.fillMaxWidth().heightIn(min = 80.dp, max = 200.dp),
            contentAlignment = Alignment.Center,
        ) {
            Crossfade(targetState = state, label = "preview") { s ->
                when (s) {
                    is PreviewState.Loading -> WatermelonLoader(size = 44.dp)
                    is PreviewState.SvgImage -> {
                        val bmp = remember(s.png) { BitmapFactory.decodeByteArray(s.png, 0, s.png.size) }
                        if (bmp != null) {
                            Box(Modifier.background(Color.White).padding(8.dp), contentAlignment = Alignment.Center) {
                                Image(bmp.asImageBitmap(), contentDescription = s.name)
                            }
                        }
                    }
                    is PreviewState.Failed -> Text(s.message, color = WatermelonRed, fontSize = 13.sp)
                    PreviewState.Empty -> {}
                }
            }
        }
        if (showProperties && properties != null) {
            Spacer(Modifier.height(12.dp))
            HorizontalDivider(color = Color(0xFFE2E8F0))
            Spacer(Modifier.height(8.dp))
            VectorPropertiesPanel(properties)
        }
    }
}

@Composable
private fun FullScreenPreview(state: PreviewState, onClose: () -> Unit) {
    // Full-screen overlay with white canvas, image fills the screen, close button at top.
    // Uses fillMaxSize so the image actually expands rather than opening at the same size.
    androidx.compose.ui.window.Dialog(
        onDismissRequest = onClose,
        properties = androidx.compose.ui.window.DialogProperties(
            usePlatformDefaultWidth = false,
            dismissOnBackPress = true,
            dismissOnClickOutside = false,
        ),
    ) {
        Box(
            Modifier
                .fillMaxSize()
                .background(Color.White),
        ) {
            when (val s = state) {
                is PreviewState.SvgImage -> {
                    val bmp = remember(s.png) {
                        BitmapFactory.decodeByteArray(s.png, 0, s.png.size)
                    }
                    if (bmp != null) {
                        // Image fills available space, maintaining aspect ratio
                        Image(
                            bitmap = bmp.asImageBitmap(),
                            contentDescription = s.name,
                            modifier = Modifier
                                .fillMaxSize()
                                .padding(24.dp),
                            contentScale = ContentScale.Fit,
                        )
                    }
                }
                is PreviewState.Failed -> {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Text(s.message, color = WatermelonRed, textAlign = TextAlign.Center)
                    }
                }
                else -> {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        WatermelonLoader(size = 56.dp)
                    }
                }
            }

            // Close button pinned top-right
            Box(
                Modifier
                    .align(Alignment.TopEnd)
                    .padding(12.dp)
                    .clip(CircleShape)
                    .background(DeepNavy.copy(alpha = 0.75f))
                    .clickable { onClose() }
                    .padding(horizontal = 16.dp, vertical = 8.dp),
            ) {
                Text("✕  Close", color = PureWhite, fontSize = 13.sp, fontWeight = FontWeight.SemiBold)
            }

            // File name pinned at bottom
            if (state is PreviewState.SvgImage) {
                Box(
                    Modifier
                        .align(Alignment.BottomCenter)
                        .fillMaxWidth()
                        .background(DeepNavy.copy(alpha = 0.55f))
                        .padding(12.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    Text(state.name, color = PureWhite, fontSize = 13.sp)
                }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Dialogs
// ─────────────────────────────────────────────────────────────────────────────

@Composable
private fun RenameDialog(count: Int, onConfirm: (String) -> Unit, onDismiss: () -> Unit) {
    var name by remember { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (count > 1) "Rename $count files" else "Rename file") },
        text = {
            Column {
                if (count > 1) {
                    Text(
                        "First file keeps this name; the rest are numbered. Extensions are preserved.",
                        style = MaterialTheme.typography.labelLarge,
                        color = SlateGray,
                    )
                    Spacer(Modifier.height(8.dp))
                }
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    placeholder = { Text("New name (no extension)") },
                    singleLine = true,
                    shape = RoundedCornerShape(12.dp),
                )
            }
        },
        confirmButton = {
            Button(
                onClick = { if (name.isNotBlank()) onConfirm(name.trim()) },
                shape = RoundedCornerShape(50),
                colors = ButtonDefaults.buttonColors(containerColor = FreshTeal),
            ) { Text("Rename") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel", color = SlateGray) } },
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Permission rationale (MANAGE_EXTERNAL_STORAGE)
// ─────────────────────────────────────────────────────────────────────────────

@Composable
private fun PermissionRationale(onGrant: () -> Unit, onUseSystemPicker: () -> Unit) {
    Box(
        Modifier
            .fillMaxSize()
            .background(androidx.compose.material3.MaterialTheme.colorScheme.background)
            .padding(32.dp),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally) {
            Text("\uD83D\uDCC1", fontSize = 48.sp)
            Spacer(Modifier.height(16.dp))
            Text(
                "Storage access needed",
                fontWeight = FontWeight.Bold,
                fontSize = 20.sp,
                color = MaterialTheme.colorScheme.onBackground,
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(8.dp))
            Text(
                "Full-device browsing is optional. Grant access only if you want Watermelon " +
                    "to search internal storage and SD cards in one browser. For ordinary " +
                    "conversion, use the system file, ZIP, or folder picker instead.",
                fontSize = 14.sp,
                color = SlateGray,
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(24.dp))
            Button(
                onClick = onUseSystemPicker,
                shape = RoundedCornerShape(50),
                colors = ButtonDefaults.buttonColors(containerColor = FreshTeal),
            ) { Text("Use system picker") }
            TextButton(onClick = onGrant) { Text("Enable full-device browser") }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Folder chooser (destination picker for copy/move)
// ─────────────────────────────────────────────────────────────────────────────

@Composable
private fun FolderChooserDialog(
    storageRoots: List<StorageRoot>,
    onDismiss: () -> Unit,
    onSelected: (File) -> Unit,
) {
    var current by remember { mutableStateOf<File?>(null) }
    val entries = remember(current) {
        current?.listFiles { f -> f.isDirectory && !f.name.startsWith(".") }
            ?.sortedBy { it.name.lowercase() } ?: emptyList()
    }

    Dialog(onDismissRequest = onDismiss) {
        Surface(shape = RoundedCornerShape(20.dp), color = MaterialTheme.colorScheme.surface) {
            Column(Modifier.padding(20.dp).heightIn(max = 480.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    if (current != null) {
                        TextButton(onClick = {
                            val parent = current?.parentFile
                            current = if (storageRoots.any { it.root.absolutePath == current?.absolutePath }) null else parent
                        }) {
                            Text("‹", color = FreshTeal, fontSize = 18.sp)
                        }
                    }
                    Text(
                        current?.name ?: "Choose destination",
                        fontWeight = FontWeight.SemiBold,
                        fontSize = 16.sp,
                        color = MaterialTheme.colorScheme.onSurface,
                        modifier = Modifier.weight(1f),
                    )
                }
                Spacer(Modifier.height(8.dp))
                LazyColumn(Modifier.weight(1f, fill = false)) {
                    if (current == null) {
                        items(storageRoots) { root ->
                            Row(
                                Modifier
                                    .fillMaxWidth()
                                    .clickable { current = root.root }
                                    .padding(vertical = 12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Text(if (root.isPrimary) "\uD83D\uDCF1" else "\uD83D\uDCBE", fontSize = 18.sp)
                                Spacer(Modifier.width(10.dp))
                                Text(root.label, color = MaterialTheme.colorScheme.onSurface)
                            }
                        }
                    } else {
                        items(entries) { dir ->
                            Row(
                                Modifier
                                    .fillMaxWidth()
                                    .clickable { current = dir }
                                    .padding(vertical = 12.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                FolderIcon(size = 18.dp)
                                Spacer(Modifier.width(10.dp))
                                Text(dir.name, color = MaterialTheme.colorScheme.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            }
                        }
                    }
                }
                Spacer(Modifier.height(12.dp))
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                    TextButton(onClick = onDismiss) { Text("Cancel", color = SlateGray) }
                    Spacer(Modifier.width(8.dp))
                    Button(
                        onClick = { current?.let(onSelected) },
                        enabled = current != null,
                        shape = RoundedCornerShape(50),
                        colors = ButtonDefaults.buttonColors(containerColor = FreshTeal),
                    ) { Text("Select this folder") }
                }
            }
        }
    }
}
