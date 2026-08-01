// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.viewmodel

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.watermelon.converter.data.files.FileKind
import com.watermelon.converter.data.files.FileNode
import com.watermelon.converter.data.files.MarkedFilesStore
import com.watermelon.converter.data.files.RealFileRepository
import com.watermelon.converter.data.files.StorageRoot
import com.watermelon.converter.data.files.TypeFilter
import com.watermelon.converter.data.prefs.SettingsRepository
import com.watermelon.converter.jni.RealSvgConverter
import com.watermelon.converter.jni.SvgConverter
import com.watermelon.converter.logging.AppLogger
import com.watermelon.converter.util.StoragePermission
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.ByteArrayOutputStream
import java.io.File
import java.util.zip.ZipEntry
import java.util.zip.ZipOutputStream

/** A row of the tree: either a top-level storage root, or a file/folder entry. */
sealed interface RowNode {
    data class Root(val storageRoot: StorageRoot) : RowNode
    data class Entry(val node: FileNode) : RowNode
}

/** A row in the flattened, indented tree view. */
data class TreeRow(
    val row: RowNode,
    val depth: Int,
    val expanded: Boolean,
)

/** Preview content for the docked pane. */
sealed interface PreviewState {
    data object Empty : PreviewState
    data object Loading : PreviewState
    @Suppress("ArrayInDataClass")
    data class SvgImage(val name: String, val png: ByteArray) : PreviewState
    data class Failed(val name: String, val message: String) : PreviewState
}

/** Outcome of a "convert marked files" run, shown as a small confirmation. */
data class ConvertMarkedResult(
    val succeeded: Int,
    val failed: Int,
    val savedTo: String,
)

class FileManagerViewModel(
    app: Application,
    private val native: SvgConverter,
) : AndroidViewModel(app) {

    constructor(app: Application) : this(app, RealSvgConverter)

    private val repo = RealFileRepository()
    private val settingsRepo = SettingsRepository(app.applicationContext)

    private suspend fun outputDestUri(): String? =
        settingsRepo.settings.first().outputDestinationUri

    // --- permission + roots --------------------------------------------------

    private val _hasPermission = MutableStateFlow(StoragePermission.isGranted())
    val hasPermission: StateFlow<Boolean> = _hasPermission.asStateFlow()

    /** null = at the synthetic root (storage roots shown). non-null = browsing a folder. */
    private val _currentDir = MutableStateFlow<File?>(null)
    val currentDir: StateFlow<File?> = _currentDir.asStateFlow()

    private val _storageRoots = MutableStateFlow<List<StorageRoot>>(emptyList())
    val storageRoots: StateFlow<List<StorageRoot>> = _storageRoots.asStateFlow()

    private val _filter = MutableStateFlow(TypeFilter())
    val filter: StateFlow<TypeFilter> = _filter.asStateFlow()

    private val _rows = MutableStateFlow<List<TreeRow>>(emptyList())
    val rows: StateFlow<List<TreeRow>> = _rows.asStateFlow()

    val marked: StateFlow<Set<String>> = MarkedFilesStore.marked

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

    private val _converting = MutableStateFlow(false)
    val converting: StateFlow<Boolean> = _converting.asStateFlow()

    private val _convertResult = MutableStateFlow<ConvertMarkedResult?>(null)
    val convertResult: StateFlow<ConvertMarkedResult?> = _convertResult.asStateFlow()

    private val _selectionMode = MutableStateFlow(false)
    val selectionMode: StateFlow<Boolean> = _selectionMode.asStateFlow()

    private val _selected = MutableStateFlow<Set<String>>(emptySet()) // absolute paths
    val selected: StateFlow<Set<String>> = _selected.asStateFlow()

    private val _opStatus = MutableStateFlow<String?>(null)
    val opStatus: StateFlow<String?> = _opStatus.asStateFlow()

    /** True only while the fast (names) pass hasn't produced rows yet. */
    private val _loading = MutableStateFlow(false)
    val loading: StateFlow<Boolean> = _loading.asStateFlow()

    private val expanded = LinkedHashSet<String>()          // absolute paths
    private val childrenCache = HashMap<String, List<FileNode>>()
    private val containsCache = HashMap<String, Boolean>()

    init {
        refreshRoots()
    }

    private fun refreshRoots() {
        _hasPermission.value = StoragePermission.isGranted()
        if (!_hasPermission.value) return
        _storageRoots.value = repo.storageRoots(getApplication())
        if (_currentDir.value == null) rebuild(showLoading = false)
    }

    /** Call on resume — permission may have just been granted via Settings. */
    fun recheckPermission() = refreshRoots()

    fun tapRoot(root: StorageRoot) {
        childrenCache.clear()
        containsCache.clear()
        expanded.clear()
        _currentDir.value = root.root
        rebuild(showLoading = true)
    }

    fun goToStorageRoot() {
        _currentDir.value = null
        expanded.clear()
        childrenCache.clear()
        containsCache.clear()
        rebuild(showLoading = false)
    }

    fun openPermissionSettings() {
        val ctx = getApplication<Application>()
        ctx.startActivity(
            StoragePermission.settingsIntent(ctx).addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK)
        )
    }

    fun setFilter(showSvg: Boolean, showXml: Boolean) {
        _filter.value = TypeFilter(showSvg, showXml)
        containsCache.clear()
        rebuild(showLoading = false)
    }

    fun toggleDir(node: FileNode) {
        val key = node.absolutePath
        if (expanded.contains(key)) expanded.remove(key) else expanded.add(key)
        rebuild(showLoading = false)
    }

    fun isMarked(node: FileNode): Boolean = MarkedFilesStore.isMarked(node.file)
    fun clearMarks() = MarkedFilesStore.clear()

    fun setQuery(q: String) {
        _query.value = q
        if (q.isBlank()) {
            _searchResults.value = emptyList()
            return
        }
        val root = _currentDir.value ?: run { _searchResults.value = emptyList(); return }
        viewModelScope.launch {
            _loading.value = true
            val results = withContext(Dispatchers.IO) {
                repo.search(root, q).filter { _filter.value.accepts(it) }
            }
            _searchResults.value = results
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
                        val renderDeferred = withContext(Dispatchers.IO) {
                            async {
                                when (node.kind) {
                                    FileKind.Svg -> native.renderSvgPreview(bytes, 256)
                                    else         -> native.renderVdPreview(String(bytes), 256)
                                }
                            }
                        }
                        val analyzeDeferred = withContext(Dispatchers.IO) {
                            async {
                                try {
                                    val json = native.analyzeVector(bytes)
                                    com.watermelon.converter.data.model.VectorProperties.fromJson(node.name, json)
                                } catch (e: Exception) {
                                    AppLogger.logError("FileManager", "analysis failed for ${node.name}", e)
                                    null
                                }
                            }
                        }
                        _preview.value    = PreviewState.SvgImage(node.name, renderDeferred.await())
                        _properties.value = analyzeDeferred.await()
                    }
                    else -> _preview.value = PreviewState.Empty
                }
            } catch (e: Exception) {
                AppLogger.logError("FileManager", "preview failed for ${node.name}", e)
                _preview.value = PreviewState.Failed(node.name, e.message ?: "preview failed")
            }
        }
    }

    fun closePreview() {
        _preview.value = PreviewState.Empty
        _previewedFile.value = null
        _properties.value = null
    }

    fun toggleMarkPreviewed() {
        val node = _previewedFile.value ?: return
        if (node.kind == FileKind.Svg) MarkedFilesStore.toggle(node.file)
    }

    fun convertMarked() {
        val ctx = getApplication<Application>()
        val files = MarkedFilesStore.files().filter {
            it.isFile && it.name.endsWith(".svg", ignoreCase = true)
        }
        if (files.isEmpty()) return
        _converting.value = true
        viewModelScope.launch {
            try {
                val outFile = withContext(Dispatchers.IO) {
                    val zipBytes = zipFiles(files)
                    val resultZip = native.convertZip(zipBytes, object : com.watermelon.converter.jni.ProgressCallback {
                        override fun onProgress(done: Int, total: Int, currentName: String) {}
                    })
                    var succeeded = 0
                    var failed = 0
                    java.util.zip.ZipInputStream(resultZip.inputStream()).use { zis ->
                        while (true) {
                            val e = zis.nextEntry ?: break
                            val content = zis.readBytes()
                            if (e.name.endsWith(".error.txt")) {
                                failed++
                                val src = e.name.removeSuffix(".error.txt")
                                com.watermelon.converter.data.model.HistoryStore.add(
                                    src, "", ok = false, error = String(content).trim()
                                )
                            } else {
                                succeeded++
                                com.watermelon.converter.data.model.HistoryStore.add(
                                    e.name, String(content), ok = true
                                )
                            }
                        }
                    }
                    val outName = "batch_${System.currentTimeMillis()}.zip"
                    val destUri = outputDestUri()
                    com.watermelon.converter.util.OutputDestination.write(
                        getApplication(), resultZip, outName, destUri,
                    )
                    val savedTo = com.watermelon.converter.util.OutputDestination
                        .displayLabel(getApplication(), destUri) + "/" + outName
                    ConvertMarkedResult(succeeded, failed, savedTo)
                }
                _convertResult.value = outFile
                MarkedFilesStore.clear()
            } catch (e: Exception) {
                AppLogger.logError("FileManager", "convertMarked failed", e)
            } finally {
                _converting.value = false
            }
        }
    }

    fun dismissConvertResult() { _convertResult.value = null }

    private fun zipFiles(files: List<File>): ByteArray {
        val out = ByteArrayOutputStream()
        val used = HashSet<String>()
        ZipOutputStream(out).use { zos ->
            for (f in files) {
                var name = f.name
                if (!used.add(name)) {
                    val base = name.substringBeforeLast('.', name)
                    val ext = name.substringAfterLast('.', "")
                    var i = 1
                    do {
                        name = if (ext.isEmpty()) "${base}_$i" else "${base}_$i.$ext"
                        i++
                    } while (!used.add(name))
                }
                val bytes = f.readBytes()
                zos.putNextEntry(ZipEntry(name))
                zos.write(bytes)
                zos.closeEntry()
            }
        }
        return out.toByteArray()
    }

    // --- selection management --------------------------------------

    fun isSelected(node: FileNode): Boolean = _selected.value.contains(node.absolutePath)

    fun startSelection(node: FileNode) {
        if (node.isDirectory) return
        _selectionMode.value = true
        _selected.value = _selected.value + node.absolutePath
    }

    fun toggleSelect(node: FileNode) {
        if (node.isDirectory) return
        val key = node.absolutePath
        _selected.value = _selected.value.toMutableSet().apply {
            if (contains(key)) remove(key) else add(key)
        }
        if (_selected.value.isEmpty()) _selectionMode.value = false
    }

    fun exitSelection() {
        _selectionMode.value = false
        _selected.value = emptySet()
    }

    fun selectAllSvg() = selectAllOfType(svg = true)
    fun selectAllXml() = selectAllOfType(svg = false)

    private fun selectAllOfType(svg: Boolean) {
        val dir = _currentDir.value ?: return
        viewModelScope.launch {
            val files = withContext(Dispatchers.IO) {
                val f = if (svg) TypeFilter(showSvg = true, showXml = false)
                        else TypeFilter(showSvg = false, showXml = true)
                repo.matchingFilesIn(dir, f)
            }
            if (files.isNotEmpty()) {
                _selectionMode.value = true
                _selected.value = _selected.value + files.map { it.absolutePath }
            }
        }
    }

    private fun selectedNodes(): List<FileNode> {
        val allCached = childrenCache.values.flatten()
        return allCached.filter { _selected.value.contains(it.absolutePath) && it.file.exists() }
    }

    fun dismissOpStatus() { _opStatus.value = null }

    // --- file operations -------------------------------------------

    fun zipAndShipSelected() {
        val files = selectedNodes().map { it.file }.filter { it.name.endsWith(".svg", true) }
        if (files.isEmpty()) {
            _opStatus.value = "No SVG files selected to convert"
            return
        }
        exitSelection()
        _converting.value = true
        viewModelScope.launch {
            try {
                val result = withContext(Dispatchers.IO) {
                    val zipBytes = zipFiles(files)
                    val resultZip = native.convertZip(zipBytes, object : com.watermelon.converter.jni.ProgressCallback {
                        override fun onProgress(done: Int, total: Int, currentName: String) {}
                    })
                    var succeeded = 0; var failed = 0
                    java.util.zip.ZipInputStream(resultZip.inputStream()).use { zis ->
                        while (true) {
                            val e = zis.nextEntry ?: break
                            val content = zis.readBytes()
                            if (e.name.endsWith(".error.txt")) {
                                failed++
                                val src = e.name.removeSuffix(".error.txt")
                                com.watermelon.converter.data.model.HistoryStore.add(
                                    src, "", ok = false, error = String(content).trim()
                                )
                            } else {
                                succeeded++
                                com.watermelon.converter.data.model.HistoryStore.add(
                                    e.name, String(content), ok = true
                                )
                            }
                        }
                    }
                    val outName = "batch_${System.currentTimeMillis()}.zip"
                    val destUri = outputDestUri()
                    com.watermelon.converter.util.OutputDestination.write(
                        getApplication(), resultZip, outName, destUri,
                    )
                    val savedTo = com.watermelon.converter.util.OutputDestination
                        .displayLabel(getApplication(), destUri) + "/" + outName
                    ConvertMarkedResult(succeeded, failed, savedTo)
                }
                _convertResult.value = result
            } catch (e: Exception) {
                AppLogger.logError("FileManager", "zipAndShipSelected failed", e)
                _opStatus.value = "Conversion failed"
            } finally {
                _converting.value = false
            }
        }
    }

    fun deleteSelected() {
        val files = selectedNodes().map { it.file }
        if (files.isEmpty()) return
        viewModelScope.launch {
            val n = withContext(Dispatchers.IO) { repo.delete(files) }
            invalidateAndRebuild()
            exitSelection()
            _opStatus.value = "Deleted $n file${if (n == 1) "" else "s"}"
        }
    }

    fun renameSelected(baseName: String) {
        val files = selectedNodes().map { it.file }
        if (files.isEmpty() || baseName.isBlank()) return
        viewModelScope.launch {
            withContext(Dispatchers.IO) {
                files.forEachIndexed { i, f ->
                    val ext = f.name.substringAfterLast('.', "")
                    val stem = if (i == 0) baseName else "${baseName}_$i"
                    val newName = if (ext.isEmpty()) stem else "$stem.$ext"
                    repo.rename(f, newName)
                }
            }
            invalidateAndRebuild()
            exitSelection()
            _opStatus.value = "Renamed ${files.size} file${if (files.size == 1) "" else "s"}"
        }
    }

    fun copyOrMoveSelectedTo(destDir: File, move: Boolean) {
        val files = selectedNodes().map { it.file }
        if (files.isEmpty()) return
        viewModelScope.launch {
            val n = withContext(Dispatchers.IO) {
                var count = 0
                for (f in files) {
                    val result = if (move) repo.moveInto(f, destDir) else repo.copyInto(f, destDir)
                    if (result != null) count++
                }
                count
            }
            if (move) invalidateAndRebuild()
            exitSelection()
            _opStatus.value = "${if (move) "Moved" else "Copied"} $n file${if (n == 1) "" else "s"}"
        }
    }

    private fun invalidateAndRebuild() {
        childrenCache.clear()
        containsCache.clear()
        rebuild(showLoading = true)
    }

    // --- tree building (two-phase: fast names, then deferred metadata) ------

    private fun rebuild(showLoading: Boolean = true) {
        viewModelScope.launch {
            if (showLoading) _loading.value = true
            // Phase 1: fast — names + extensions only, renders immediately.
            val fastList = withContext(Dispatchers.IO) { flatten(fast = true) }
            _rows.value = fastList
            if (showLoading) _loading.value = false

            // Phase 2: deferred — fill in size/lastModified for file rows in the
            // background, then swap in the enriched rows without a spinner.
            if (_currentDir.value != null) {
                val enriched = withContext(Dispatchers.IO) { flatten(fast = false) }
                _rows.value = enriched
            }
        }
    }

    private fun flatten(fast: Boolean): List<TreeRow> {
        val dir = _currentDir.value
        if (dir == null) {
            return _storageRoots.value.map { TreeRow(RowNode.Root(it), depth = 0, expanded = false) }
        }
        val out = ArrayList<TreeRow>()
        val filter = _filter.value
        fun walk(d: File, depth: Int) {
            val cacheKey = d.absolutePath
            val kids = if (fast) {
                childrenCache.getOrPut(cacheKey) { repo.listChildrenFast(d) }
            } else {
                // Enrich cached fast nodes in place with real metadata.
                val existing = childrenCache[cacheKey] ?: repo.listChildrenFast(d)
                val withMeta = existing.map { if (it.isDirectory) it else repo.withMetadata(it) }
                childrenCache[cacheKey] = withMeta
                withMeta
            }
            for (node in kids) {
                if (node.isDirectory) {
                    val matches = containsCache.getOrPut(node.absolutePath) {
                        repo.containsMatching(node.file, filter)
                    }
                    if (!matches) continue
                    val isExp = expanded.contains(node.absolutePath)
                    out.add(TreeRow(RowNode.Entry(node), depth, isExp))
                    if (isExp) walk(node.file, depth + 1)
                } else {
                    if (filter.accepts(node)) out.add(TreeRow(RowNode.Entry(node), depth, false))
                }
            }
        }
        walk(dir, 0)
        return out
    }
}
