// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.viewmodel

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.watermelon.converter.data.model.BatchReport
import com.watermelon.converter.data.model.FileOutcome
import com.watermelon.converter.data.repository.FileRepository
import com.watermelon.converter.jni.ConversionException
import com.watermelon.converter.logging.AppLogger
import com.watermelon.converter.jni.ProgressCallback
import com.watermelon.converter.jni.RealSvgConverter
import com.watermelon.converter.jni.SvgConverter
import com.watermelon.converter.jni.userMessage
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.ByteArrayInputStream
import java.util.zip.ZipInputStream

/**
 * Reverse direction of BatchViewModel: zip of VectorDrawable .xml -> zip of
 * .svg. Mirrors BatchViewModel's state machine exactly, using convertVdZip
 * instead of convertZip — kept separate, consistent with the FFI layer's
 * convert_vd_zip split, so the shipped batch path is never touched.
 */
class ReverseBatchViewModel(
    app: Application,
    private val native: SvgConverter,
) : AndroidViewModel(app) {

    constructor(app: Application) : this(app, RealSvgConverter)

    private val repo = FileRepository(app.applicationContext)
    private val settingsRepo = com.watermelon.converter.data.prefs.SettingsRepository(app.applicationContext)
    private val _state = MutableStateFlow<BatchUiState>(BatchUiState.Idle)
    val state: StateFlow<BatchUiState> = _state.asStateFlow()

    private var lastZip: ByteArray? = null

    private suspend fun outputDestUri(): String? =
        settingsRepo.settings.first().outputDestinationUri

    fun convertZip(zipUri: Uri) {
        runConvert { repo.readBytes(zipUri) }
    }

    /**
     * Convert a custom batch of loose XML files (selected across folders in
     * the file manager). Zipped in Kotlin, fed to the native convertVdZip path.
     */
    fun convertFromUris(uris: List<Uri>) {
        if (uris.isEmpty()) return
        runConvert { repo.zipBytes(uris) }
    }

    private fun runConvert(makeZipBytes: () -> ByteArray) {
        _state.value = BatchUiState.Working(null)
        viewModelScope.launch {
            val started = System.currentTimeMillis()
            try {
                val destUri = outputDestUri()
                val (inputBytes, out) = withContext(Dispatchers.IO) {
                    val bytes = makeZipBytes()
                    val cb = object : ProgressCallback {
                        override fun onProgress(done: Int, total: Int, currentName: String) {
                            _state.value = BatchUiState.Working(BatchProgress(done, total, currentName))
                        }
                    }
                    bytes.size.toLong() to native.convertVdZip(bytes, cb)
                }
                lastZip = out
                withContext(Dispatchers.IO) {
                    val fileName = "batch_svg_${System.currentTimeMillis()}.zip"
                    com.watermelon.converter.util.OutputDestination.write(
                        getApplication(), out, fileName, destUri,
                    )
                }
                val report = withContext(Dispatchers.IO) {
                    buildReport(out, inputBytes, System.currentTimeMillis() - started)
                }
                _state.value = BatchUiState.Done(out, report)
            } catch (e: ConversionException) {
                _state.value = BatchUiState.Error(e.userMessage(getApplication()))
            } catch (e: Exception) {
                _state.value = BatchUiState.Error(e.message ?: "Unknown error")
            }
        }
    }

    /**
     * Build the end-of-run report by scanning the output ZIP. Mirrors
     * BatchViewModel.buildReport exactly — the native reverse batch processor
     * writes the same "<n>.error.txt" sidecar convention.
     */
    private fun buildReport(outputZip: ByteArray, inputBytes: Long, durationMs: Long): BatchReport {
        val outcomes = ArrayList<FileOutcome>()
        var outputBytes = 0L
        ZipInputStream(ByteArrayInputStream(outputZip)).use { zis ->
            while (true) {
                val entry = zis.nextEntry ?: break
                val name = entry.name
                val content = zis.readBytes()
                outputBytes += content.size.toLong()
                if (name.endsWith(".error.txt", ignoreCase = true)) {
                    val original = name.removeSuffix(".error.txt")
                    val text = String(content).trim()
                    val code = text.substringAfter('[', "").substringBefore(']').toIntOrNull()
                    val msg = text.substringAfter("] ", text)
                    outcomes.add(FileOutcome(original, ok = false, errorCode = code, errorMessage = msg))
                    com.watermelon.converter.data.model.HistoryStore.add(original, "", ok = false, error = text)
                } else {
                    outcomes.add(FileOutcome(name, ok = true))
                    com.watermelon.converter.data.model.HistoryStore.add(name, String(content), ok = true)
                }
            }
        }
        val failed = outcomes.count { !it.ok }
        return BatchReport(
            total = outcomes.size,
            succeeded = outcomes.size - failed,
            failed = failed,
            inputBytes = inputBytes,
            outputBytes = outputBytes,
            durationMillis = durationMs,
            outcomes = outcomes,
        )
    }

    fun cancel() { native.cancel() }

    fun export(treeUri: Uri, fileName: String = "watermelon_svgs.zip") {
        val zip = lastZip ?: return
        viewModelScope.launch(Dispatchers.IO) {
            runCatching { repo.writeToTree(treeUri, fileName, "application/zip", zip) }
        }
    }

    fun exportToSettings(fileName: String = "watermelon_svgs.zip") {
        val zip = lastZip ?: return
        viewModelScope.launch(Dispatchers.IO) {
            runCatching {
                val destUri = settingsRepo.settings.first().outputDestinationUri
                if (destUri != null) {
                    com.watermelon.converter.util.OutputDestination.write(
                        getApplication(), zip, fileName, destUri, mime = "application/zip",
                    )
                }
            }
        }
    }

    fun reset() { _state.value = BatchUiState.Idle }

    private val _reportSaveState = kotlinx.coroutines.flow.MutableStateFlow<String?>(null)
    val reportSaveState: StateFlow<String?> = _reportSaveState.asStateFlow()

    fun saveReport() {
        val report = (_state.value as? BatchUiState.Done)?.report ?: return
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val destUri = outputDestUri()
                val fileName = "report_svg_${System.currentTimeMillis()}.txt"
                val bytes = formatReport(report).toByteArray()
                com.watermelon.converter.util.OutputDestination.write(
                    getApplication(), bytes, fileName, destUri, mime = "text/plain",
                )
                _reportSaveState.value = "Report saved: ${fileName}"
            } catch (e: Exception) {
                AppLogger.logError("ReverseBatchViewModel", "saveReport failed", e)
                _reportSaveState.value = "Could not save report"
            }
        }
    }

    fun dismissReport() {
        _state.value = BatchUiState.Idle
        _reportSaveState.value = null
    }

    private fun formatReport(r: BatchReport): String = buildString {
        appendLine("=== Watermelon Vector Converter — conversion report (XML \u2192 SVG) ===")
        appendLine("Total files: ${r.total}")
        appendLine("Succeeded:   ${r.succeeded}")
        appendLine("Failed:      ${r.failed}")
        appendLine("Input size:  ${r.inputBytes} bytes")
        appendLine("Output size: ${r.outputBytes} bytes")
        appendLine("Duration:    ${r.durationMillis} ms")
        appendLine()
        if (r.rejected.isNotEmpty()) {
            appendLine("Rejected files:")
            r.rejected.forEach { f ->
                val code = f.errorCode?.let { "[$it] " } ?: ""
                appendLine("  - ${f.name}: $code${f.errorMessage ?: "unknown"}")
            }
        } else {
            appendLine("All files converted successfully.")
        }
    }
}
