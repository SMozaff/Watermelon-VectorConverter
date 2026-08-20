// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

package com.watermelon.converter.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.watermelon.converter.data.files.FileKind
import com.watermelon.converter.data.files.FileNode
import com.watermelon.converter.data.files.RealFileRepository
import com.watermelon.converter.data.files.StorageRoot
import com.watermelon.converter.data.files.TypeFilter
import com.watermelon.converter.data.prefs.SettingsRepository
import com.watermelon.converter.data.model.BatchReport
import com.watermelon.converter.data.model.FileOutcome
import com.watermelon.converter.jni.RealSvgConverter
import com.watermelon.converter.jni.SvgConverter
import com.watermelon.converter.logging.AppLogger
import com.watermelon.converter.util.OutputDestination
import com.watermelon.converter.util.StoragePermission
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.ByteArrayInputStream
import java.io.File
import java.util.zip.ZipInputStream

sealed interface RowNode {
    data class Root(val storageRoot: StorageRoot) : RowNode
    data class Entry(val node: FileNode) : RowNode
}

data class TreeRow(val row: RowNode, val depth: Int, val expanded: Boolean)

sealed interface PreviewState {
    data object Empty : PreviewState
    data object Loading : PreviewState
    @Suppress("ArrayInDataClass") data class SvgImage(val name: String, val png: ByteArray) : PreviewState
    data class Failed(val name: String, val message: String) : PreviewState
}

enum class FileBatchDirection(
    val label: String,
    val expectedKind: FileKind,
    val outputPrefix: String,
) {
    SvgToVector("SVG → VectorDrawable XML", FileKind.Svg, "watermelon_vectors"),
    VectorToSvg("VectorDrawable XML → SVG", FileKind.Xml, "watermelon_svgs"),
}

data class FileBrowserBatchPreflight(
    val direction: FileBatchDirection,
    val eligibleFiles: List<File>,
    val rejected: List<BatchInputRejection>,
    val inputBytes: Long,
    val outputName: String,
    val destinationLabel: String,
) {
    val selectedCount: Int get() = eligibleFiles.size + rejected.size
}

sealed interface FileBrowserBatchState {
    data object Idle : FileBrowserBatchState
    data class Preflight(val details: FileBrowserBatchPreflight) : FileBrowserBatchState
    data class Working(val progress: BatchProgress?) : FileBrowserBatchState
    data class Done(val report: BatchReport, val outputName: String, val savedTo: String) : FileBrowserBatchState
    data class Cancelled(val message: String) : FileBrowserBatchState
    data class Failed(val message: String) : FileBrowserBatchState
}

class FileManagerViewModel(
    app: Application,
    private val native: SvgConverter,
) : AndroidViewModel(app) {
    constructor(app: Application) : this(app, RealSvgConverter)

    private val repo = RealFileRepository()
    private val settingsRepo = SettingsRepository(app.applicationContext)
    private suspend fun outputDestUri(): String? = settingsRepo.settings.first().outputDestinationUri

    private val _hasPermission = MutableStateFlow(StoragePermission.isGranted())
    val hasPermission: StateFlow<Boolean> = _hasPermission.asStateFlow()
    private val _currentDir = MutableStateFlow<File?>(null)
    val currentDir: StateFlow<File?> = _currentDir.asStateFlow()
    private val _storageRoots = MutableStateFlow<List<StorageRoot>>(emptyList())
    val storageRoots: StateFlow<List<StorageRoot>> = _storageRoots.asStateFlow()
    private val _filter = MutableStateFlow(TypeFilter())
    val filter: StateFlow<TypeFilter> = _filter.asStateFlow()
    private val _rows = MutableStateFlow<List<TreeRow>>(emptyList())
    val rows: StateFlow<List<TreeRow>> = _rows.asStateFlow()
    private val _preview = MutableStateFlow<PreviewState>(PreviewState.Empty)
    val preview: StateFlow<PreviewState> = _preview.asStateFlow()
    private val _previewedFile = MutableStateFlow<FileNode?>(null)
    val previewedFile: StateFlow<FileNode?> = _previewedFile.asStateFlow()
    private val _properties = MutableStateFlow<com.watermelon.converter.data.model.VectorProperties?>(null)
    val properties: StateFlow<com.watermelon.converter.data.model.VectorProperties?> = _properties.asStateFlow()
    private val _query = MutableStateFlow("")
    val query: StateFlow<String> = _query.asStateFlow()
    private val _searchResults = MutableStateFlow<List<FileNode>>(emptyList())
    val searchResults: StateFlow<List<FileNode>> = _searchResults.asStateFlow()
    private val _selectionMode = MutableStateFlow(false)
    val selectionMode: StateFlow<Boolean> = _selectionMode.asStateFlow()
    private val _selected = MutableStateFlow<Set<String>>(emptySet())
    val selected: StateFlow<Set<String>> = _selected.asStateFlow()
    private val _opStatus = MutableStateFlow<String?>(null)
    val opStatus: StateFlow<String?> = _opStatus.asStateFlow()
    private val _loading = MutableStateFlow(false)
    val loading: StateFlow<Boolean> = _loading.asStateFlow()
    private val _fileBatch = MutableStateFlow<FileBrowserBatchState>(FileBrowserBatchState.Idle)
    val fileBatch: StateFlow<FileBrowserBatchState> = _fileBatch.asStateFlow()

    private val expanded = LinkedHashSet<String>()
    private val childrenCache = HashMap<String, List<FileNode>>()
    private val containsCache = HashMap<String, Boolean>()
    private var lastPreflight: FileBrowserBatchPreflight? = null
    private var cancelRequested = false

    init { refreshRoots() }

    private fun refreshRoots() {
        _hasPermission.value = StoragePermission.isGranted()
        if (!_hasPermission.value) return
        _storageRoots.value = repo.storageRoots(getApplication())
        if (_currentDir.value == null) rebuild(showLoading = false)
    }

    fun recheckPermission() = refreshRoots()
    fun tapRoot(root: StorageRoot) {
        childrenCache.clear(); containsCache.clear(); expanded.clear()
        _currentDir.value = root.root
        rebuild(showLoading = true)
    }
    fun goToStorageRoot() {
        _currentDir.value = null; expanded.clear(); childrenCache.clear(); containsCache.clear()
        rebuild(showLoading = false)
    }
    fun openPermissionSettings() {
        val ctx = getApplication<Application>()
        ctx.startActivity(StoragePermission.settingsIntent(ctx).addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK))
    }
    fun setFilter(showSvg: Boolean, showXml: Boolean) {
        _filter.value = TypeFilter(showSvg, showXml); containsCache.clear(); rebuild(showLoading = false)
    }
    fun toggleDir(node: FileNode) {
        if (expanded.contains(node.absolutePath)) expanded.remove(node.absolutePath) else expanded.add(node.absolutePath)
        rebuild(showLoading = false)
    }

    fun setQuery(query: String) {
        _query.value = query
        if (query.isBlank()) { _searchResults.value = emptyList(); return }
        val root = _currentDir.value ?: run { _searchResults.value = emptyList(); return }
        viewModelScope.launch {
            _loading.value = true
            _searchResults.value = withContext(Dispatchers.IO) { repo.search(root, query).filter { _filter.value.accepts(it) } }
            _loading.value = false
        }
    }

    fun preview(node: FileNode) {
        _previewedFile.value = node
        _preview.value = PreviewState.Loading
        _properties.value = null
        viewModelScope.launch {
            try {
                val bytes = withContext(Dispatchers.IO) { node.file.readBytes() }
                when (node.kind) {
                    FileKind.Svg, FileKind.Xml -> {
                        val render = withContext(Dispatchers.IO) { async {
                            if (node.kind == FileKind.Svg) native.renderSvgPreview(bytes, 256)
                            else native.renderVdPreview(String(bytes), 256)
                        } }
                        val analyze = withContext(Dispatchers.IO) { async {
                            runCatching {
                                com.watermelon.converter.data.model.VectorProperties.fromJson(node.name, native.analyzeVector(bytes))
                            }.onFailure { AppLogger.logError("FileManager", "analysis failed for ${node.name}", it) }.getOrNull()
                        } }
                        _preview.value = PreviewState.SvgImage(node.name, render.await())
                        _properties.value = analyze.await()
                    }
                    else -> _preview.value = PreviewState.Empty
                }
            } catch (e: Exception) {
                AppLogger.logError("FileManager", "preview failed for ${node.name}", e)
                _preview.value = PreviewState.Failed(node.name, e.message ?: "Preview failed")
            }
        }
    }

    fun closePreview() { _preview.value = PreviewState.Empty; _previewedFile.value = null; _properties.value = null }

    // Single source of truth for both file operations and batch conversion.
    fun startBatchSelection() { _selectionMode.value = true; closePreview() }
    fun isSelected(node: FileNode): Boolean = node.absolutePath in _selected.value
    fun startSelection(node: FileNode) {
        if (!node.isDirectory && !batchOwnsSelection()) {
            _selectionMode.value = true
            _selected.value = _selected.value + node.absolutePath
        }
    }
    fun toggleSelect(node: FileNode) {
        if (node.isDirectory || batchOwnsSelection()) return
        val key = node.absolutePath
        _selected.value = _selected.value.toMutableSet().apply { if (!add(key)) remove(key) }
        if (_selected.value.isEmpty()) _selectionMode.value = false
    }
    fun exitSelection() { if (!batchOwnsSelection()) { _selectionMode.value = false; _selected.value = emptySet() } }
    fun selectAllSvg() = selectAllOfType(FileKind.Svg)
    fun selectAllXml() = selectAllOfType(FileKind.Xml)
    private fun selectAllOfType(kind: FileKind) {
        val dir = _currentDir.value ?: return
        viewModelScope.launch {
            val filter = if (kind == FileKind.Svg) TypeFilter(true, false) else TypeFilter(false, true)
            val files = withContext(Dispatchers.IO) { repo.matchingFilesIn(dir, filter) }
            if (files.isNotEmpty()) { _selectionMode.value = true; _selected.value = _selected.value + files.map { it.absolutePath } }
        }
    }
    private fun selectedNodes(): List<FileNode> =
        (childrenCache.values.flatten() + _searchResults.value).distinctBy { it.absolutePath }
            .filter { it.absolutePath in _selected.value && it.file.exists() }
    private fun batchOwnsSelection() = _fileBatch.value is FileBrowserBatchState.Preflight || _fileBatch.value is FileBrowserBatchState.Working
    fun dismissOpStatus() { _opStatus.value = null }

    fun prepareSelectedBatch(direction: FileBatchDirection) {
        val files = selectedNodes().map { it.file }
        if (files.isEmpty()) { _opStatus.value = "Select files before starting a batch"; return }
        viewModelScope.launch {
            val preflight = withContext(Dispatchers.IO) {
                val eligible = files.filter { file ->
                    when (direction) {
                        FileBatchDirection.SvgToVector -> file.name.endsWith(".svg", true)
                        FileBatchDirection.VectorToSvg -> file.name.endsWith(".xml", true)
                    }
                }
                val rejected = files.filterNot { it in eligible }.map { file ->
                    BatchInputRejection(file.name, "Not eligible for ${direction.label}")
                }
                val destUri = outputDestUri()
                FileBrowserBatchPreflight(
                    direction = direction,
                    eligibleFiles = eligible,
                    rejected = rejected,
                    inputBytes = eligible.sumOf { it.length() },
                    outputName = "${direction.outputPrefix}_${System.currentTimeMillis()}.zip",
                    destinationLabel = OutputDestination.displayLabel(getApplication(), destUri),
                )
            }
            lastPreflight = preflight
            _fileBatch.value = FileBrowserBatchState.Preflight(preflight)
        }
    }

    fun confirmSelectedBatch() {
        val preflight = (_fileBatch.value as? FileBrowserBatchState.Preflight)?.details ?: return
        if (preflight.eligibleFiles.isEmpty()) { _fileBatch.value = FileBrowserBatchState.Failed("No eligible files were selected"); return }
        cancelRequested = false
        _fileBatch.value = FileBrowserBatchState.Working(null)
        viewModelScope.launch {
            val started = System.currentTimeMillis()
            try {
                val output = withContext(Dispatchers.IO) {
                    val zip = zipSelectedFiles(preflight.eligibleFiles)
                    val callback = object : com.watermelon.converter.jni.ProgressCallback {
                        override fun onProgress(done: Int, total: Int, currentName: String) {
                            _fileBatch.value = FileBrowserBatchState.Working(BatchProgress(done, total, currentName))
                        }
                    }
                    if (preflight.direction == FileBatchDirection.SvgToVector) native.convertZip(zip, callback)
                    else native.convertVdZip(zip, callback)
                }
                if (cancelRequested) {
                    _fileBatch.value = FileBrowserBatchState.Cancelled("Batch cancelled. No success result was recorded.")
                    return@launch
                }
                val report = withContext(Dispatchers.IO) { buildBatchReport(output, preflight.inputBytes, System.currentTimeMillis() - started) }
                val savedTo = withContext(Dispatchers.IO) {
                    val destUri = outputDestUri()
                    OutputDestination.write(getApplication(), output, preflight.outputName, destUri)
                    "${OutputDestination.displayLabel(getApplication(), destUri)}/${preflight.outputName}"
                }
                _fileBatch.value = FileBrowserBatchState.Done(report, preflight.outputName, savedTo)
                _selectionMode.value = false
                _selected.value = emptySet()
            } catch (e: Exception) {
                _fileBatch.value = if (cancelRequested) FileBrowserBatchState.Cancelled("Batch cancelled. No success result was recorded.")
                else FileBrowserBatchState.Failed(e.message ?: "Batch conversion failed")
            }
        }
    }

    fun cancelSelectedBatch() { cancelRequested = true; native.cancel() }
    fun retrySelectedBatch() {
        val retry = lastPreflight ?: return
        _fileBatch.value = FileBrowserBatchState.Preflight(retry.copy(outputName = "${retry.direction.outputPrefix}_${System.currentTimeMillis()}.zip"))
    }
    fun dismissFileBatch() { _fileBatch.value = FileBrowserBatchState.Idle; lastPreflight = null; cancelRequested = false }

    private fun buildBatchReport(outputZip: ByteArray, inputBytes: Long, duration: Long): BatchReport {
        val outcomes = arrayListOf<FileOutcome>()
        var outputBytes = 0L
        ZipInputStream(ByteArrayInputStream(outputZip)).use { zis ->
            while (true) {
                val entry = zis.nextEntry ?: break
                val content = zis.readBytes()
                outputBytes += content.size
                if (entry.name.endsWith(".error.txt", true)) {
                    val error = String(content).trim()
                    outcomes += FileOutcome(entry.name.removeSuffix(".error.txt"), false, error.substringAfter('[', "").substringBefore(']').toIntOrNull(), error.substringAfter("] ", error))
                } else outcomes += FileOutcome(entry.name, true)
            }
        }
        val failed = outcomes.count { !it.ok }
        return BatchReport(outcomes.size, outcomes.size - failed, failed, inputBytes, outputBytes, duration, outcomes)
    }

    fun deleteSelected() {
        if (batchOwnsSelection()) return
        val files = selectedNodes().map { it.file }; if (files.isEmpty()) return
        viewModelScope.launch { val count = withContext(Dispatchers.IO) { repo.delete(files) }; invalidateAndRebuild(); exitSelection(); _opStatus.value = "Deleted $count file${if (count == 1) "" else "s"}" }
    }
    fun renameSelected(baseName: String) {
        if (batchOwnsSelection() || baseName.isBlank()) return
        val files = selectedNodes().map { it.file }; if (files.isEmpty()) return
        viewModelScope.launch {
            withContext(Dispatchers.IO) { files.forEachIndexed { index, file ->
                val ext = file.name.substringAfterLast('.', ""); val stem = if (index == 0) baseName else "${baseName}_$index"
                repo.rename(file, if (ext.isBlank()) stem else "$stem.$ext")
            } }
            invalidateAndRebuild(); exitSelection(); _opStatus.value = "Renamed ${files.size} file${if (files.size == 1) "" else "s"}"
        }
    }
    fun copyOrMoveSelectedTo(destDir: File, move: Boolean) {
        if (batchOwnsSelection()) return
        val files = selectedNodes().map { it.file }; if (files.isEmpty()) return
        viewModelScope.launch {
            val count = withContext(Dispatchers.IO) { files.count { file -> (if (move) repo.moveInto(file, destDir) else repo.copyInto(file, destDir)) != null } }
            if (move) invalidateAndRebuild(); exitSelection(); _opStatus.value = "${if (move) "Moved" else "Copied"} $count file${if (count == 1) "" else "s"}"
        }
    }

    private fun invalidateAndRebuild() { childrenCache.clear(); containsCache.clear(); rebuild(showLoading = true) }
    private fun rebuild(showLoading: Boolean = true) {
        viewModelScope.launch {
            if (showLoading) _loading.value = true
            _rows.value = withContext(Dispatchers.IO) { flatten(true) }
            if (showLoading) _loading.value = false
            if (_currentDir.value != null) _rows.value = withContext(Dispatchers.IO) { flatten(false) }
        }
    }
    private fun flatten(fast: Boolean): List<TreeRow> {
        val dir = _currentDir.value ?: return _storageRoots.value.map { TreeRow(RowNode.Root(it), 0, false) }
        val out = arrayListOf<TreeRow>(); val filter = _filter.value
        fun walk(folder: File, depth: Int) {
            val key = folder.absolutePath
            val children = if (fast) childrenCache.getOrPut(key) { repo.listChildrenFast(folder) } else {
                val existing = childrenCache[key] ?: repo.listChildrenFast(folder)
                existing.map { if (it.isDirectory) it else repo.withMetadata(it) }.also { childrenCache[key] = it }
            }
            children.forEach { node ->
                if (node.isDirectory) {
                    if (!containsCache.getOrPut(node.absolutePath) { repo.containsMatching(node.file, filter) }) return@forEach
                    val isExpanded = node.absolutePath in expanded
                    out += TreeRow(RowNode.Entry(node), depth, isExpanded)
                    if (isExpanded) walk(node.file, depth + 1)
                } else if (filter.accepts(node)) out += TreeRow(RowNode.Entry(node), depth, false)
            }
        }
        walk(dir, 0); return out
    }
}
