// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

package com.watermelon.converter.viewmodel

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.watermelon.converter.data.model.BatchReport
import com.watermelon.converter.data.model.FileOutcome
import com.watermelon.converter.data.repository.FileRepository
import com.watermelon.converter.jni.ConversionException
import com.watermelon.converter.jni.ProgressCallback
import com.watermelon.converter.jni.RealSvgConverter
import com.watermelon.converter.jni.SvgConverter
import com.watermelon.converter.jni.userMessage
import com.watermelon.converter.logging.AppLogger
import com.watermelon.converter.util.OutputDestination
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.ByteArrayInputStream
import java.io.File
import java.util.zip.ZipInputStream

data class BatchProgress(
    val done: Int,
    val total: Int,
    val currentName: String,
) {
    val totalFraction: Float get() = if (total > 0) done.toFloat() / total.toFloat() else 0f
    val fileFraction: Float get() = if (total > 0) 1f else 0f
}

sealed interface BatchUiState {
    data object Idle : BatchUiState
    data class Preflight(val details: BatchPreflight) : BatchUiState
    data class Working(val progress: BatchProgress?) : BatchUiState
    data class Done(
        val outputZip: ByteArray,
        val report: BatchReport,
        val outputName: String,
        val savedTo: String,
    ) : BatchUiState
    data class Cancelled(val message: String) : BatchUiState
    data class Error(val message: String) : BatchUiState
}

class BatchViewModel(
    app: Application,
    private val native: SvgConverter,
) : AndroidViewModel(app) {
    constructor(app: Application) : this(app, RealSvgConverter)

    private val repo = FileRepository(app.applicationContext)
    private val settingsRepo = com.watermelon.converter.data.prefs.SettingsRepository(app.applicationContext)
    private val _state = MutableStateFlow<BatchUiState>(BatchUiState.Idle)
    val state: StateFlow<BatchUiState> = _state.asStateFlow()

    private val _reportSaveState = MutableStateFlow<String?>(null)
    val reportSaveState: StateFlow<String?> = _reportSaveState.asStateFlow()

    private var pendingInput: ByteArray? = null
    private var lastZip: ByteArray? = null
    private var cancelRequested = false

    private suspend fun outputDestUri(): String? = settingsRepo.settings.first().outputDestinationUri

    /** ZIP document picker entry point: inspect first, never start native work immediately. */
    fun prepareZip(zipUri: Uri) {
        prepareInput(inputLabel = zipUri.lastPathSegment ?: "Selected ZIP") { repo.readBytes(zipUri) }
    }

    /** File-browser entry point: selected loose SVG files use the identical preflight contract. */
    fun prepareLooseFiles(files: List<File>) {
        prepareInput(inputLabel = "${files.size} selected file${if (files.size == 1) "" else "s"}") {
            zipSelectedFiles(files)
        }
    }

    /** Kept for existing callers; conversion now always begins with preflight. */
    fun convertZip(zipUri: Uri) = prepareZip(zipUri)

    fun convertFromUris(uris: List<Uri>) {
        if (uris.isNotEmpty()) prepareInput("${uris.size} selected file${if (uris.size == 1) "" else "s"}") { repo.zipBytes(uris) }
    }

    private fun prepareInput(inputLabel: String, makeZipBytes: () -> ByteArray) {
        viewModelScope.launch {
            try {
                val (rawInput, destination, outputName) = withContext(Dispatchers.IO) {
                    val bytes = makeZipBytes()
                    val destUri = outputDestUri()
                    Triple(bytes, OutputDestination.displayLabel(getApplication(), destUri), "watermelon_vectors_${System.currentTimeMillis()}.zip")
                }
                val sanitized = withContext(Dispatchers.IO) { sanitizeBatchInput(rawInput, ".svg") }
                pendingInput = sanitized.eligibleZip
                _state.value = BatchUiState.Preflight(
                    batchPreflight(
                        sourceBytes = rawInput.size.toLong(),
                        eligibleCount = sanitized.eligibleCount,
                        rejected = sanitized.rejected,
                        directionLabel = "SVG → VectorDrawable XML",
                        inputLabel = inputLabel,
                        outputName = outputName,
                        destinationLabel = destination,
                    ),
                )
            } catch (e: Exception) {
                _state.value = BatchUiState.Error(e.message ?: "Could not inspect batch input")
            }
        }
    }

    /** Confirmation boundary: native conversion starts only from this P1 action. */
    fun confirmPreflight() {
        val preflight = (_state.value as? BatchUiState.Preflight)?.details ?: return
        val input = pendingInput ?: return
        if (preflight.eligibleCount == 0) {
            _state.value = BatchUiState.Error("No eligible SVG files were found in this batch")
            return
        }
        cancelRequested = false
        _state.value = BatchUiState.Working(null)
        viewModelScope.launch {
            val started = System.currentTimeMillis()
            try {
                val out = withContext(Dispatchers.IO) {
                    native.convertZip(input, object : ProgressCallback {
                        override fun onProgress(done: Int, total: Int, currentName: String) {
                            _state.value = BatchUiState.Working(BatchProgress(done, total, currentName))
                        }
                    })
                }
                if (cancelRequested) {
                    _state.value = BatchUiState.Cancelled("Batch cancelled. No success result was recorded.")
                    return@launch
                }
                val report = withContext(Dispatchers.IO) {
                    buildReport(out, input.size.toLong(), System.currentTimeMillis() - started)
                }
                val savedTo = withContext(Dispatchers.IO) {
                    val destUri = outputDestUri()
                    OutputDestination.write(getApplication(), out, preflight.outputName, destUri)
                    "${OutputDestination.displayLabel(getApplication(), destUri)}/${preflight.outputName}"
                }
                lastZip = out
                _state.value = BatchUiState.Done(out, report, preflight.outputName, savedTo)
            } catch (e: ConversionException) {
                _state.value = if (cancelRequested) BatchUiState.Cancelled("Batch cancelled. No success result was recorded.")
                else BatchUiState.Error(e.userMessage(getApplication()))
            } catch (e: Exception) {
                _state.value = if (cancelRequested) BatchUiState.Cancelled("Batch cancelled. No success result was recorded.")
                else BatchUiState.Error(e.message ?: "Unknown batch error")
            }
        }
    }

    fun cancel() {
        cancelRequested = true
        native.cancel()
    }

    fun retry() {
        if (_state.value is BatchUiState.Error || _state.value is BatchUiState.Cancelled) {
            rebuildPreflightFromPending()
        }
    }

    private fun rebuildPreflightFromPending() {
        val input = pendingInput ?: return
        viewModelScope.launch {
            val destUri = outputDestUri()
            val outputName = "watermelon_vectors_${System.currentTimeMillis()}.zip"
            _state.value = BatchUiState.Preflight(
                batchPreflight(
                    sourceBytes = input.size.toLong(),
                    eligibleCount = countZipEntries(input),
                    rejected = emptyList(),
                    directionLabel = "SVG → VectorDrawable XML",
                    inputLabel = "Retry previous batch",
                    outputName = outputName,
                    destinationLabel = OutputDestination.displayLabel(getApplication(), destUri),
                ),
            )
        }
    }

    fun export(treeUri: Uri, fileName: String = "watermelon_vectors.zip") {
        val zip = lastZip ?: return
        viewModelScope.launch(Dispatchers.IO) {
            runCatching { repo.writeToTree(treeUri, fileName, "application/zip", zip) }
        }
    }

    fun exportToSettings(fileName: String = "watermelon_vectors.zip") {
        val zip = lastZip ?: return
        viewModelScope.launch(Dispatchers.IO) {
            runCatching {
                val destUri = settingsRepo.settings.first().outputDestinationUri
                if (destUri != null) OutputDestination.write(getApplication(), zip, fileName, destUri, mime = "application/zip")
            }
        }
    }

    fun reset() {
        pendingInput = null
        cancelRequested = false
        _state.value = BatchUiState.Idle
        _reportSaveState.value = null
    }

    fun dismissReport() = reset()

    fun saveReport() {
        val report = (_state.value as? BatchUiState.Done)?.report ?: return
        viewModelScope.launch(Dispatchers.IO) {
            try {
                val destUri = outputDestUri()
                val fileName = "report_${System.currentTimeMillis()}.txt"
                OutputDestination.write(getApplication(), formatReport(report).toByteArray(), fileName, destUri, mime = "text/plain")
                _reportSaveState.value = "Report saved: $fileName"
            } catch (e: Exception) {
                AppLogger.logError("BatchViewModel", "saveReport failed", e)
                _reportSaveState.value = "Could not save report"
            }
        }
    }

    private fun buildReport(outputZip: ByteArray, inputBytes: Long, durationMs: Long): BatchReport {
        val outcomes = arrayListOf<FileOutcome>()
        var outputBytes = 0L
        ZipInputStream(ByteArrayInputStream(outputZip)).use { zis ->
            while (true) {
                val entry = zis.nextEntry ?: break
                val content = zis.readBytes()
                outputBytes += content.size
                if (entry.name.endsWith(".error.txt", ignoreCase = true)) {
                    val original = entry.name.removeSuffix(".error.txt")
                    val text = String(content).trim()
                    val code = text.substringAfter('[', "").substringBefore(']').toIntOrNull()
                    val message = text.substringAfter("] ", text)
                    outcomes += FileOutcome(original, ok = false, errorCode = code, errorMessage = message)
                    com.watermelon.converter.data.model.HistoryStore.add(original, "", ok = false, error = text)
                } else {
                    outcomes += FileOutcome(entry.name, ok = true)
                    com.watermelon.converter.data.model.HistoryStore.add(entry.name, String(content), ok = true)
                }
            }
        }
        val failed = outcomes.count { !it.ok }
        return BatchReport(outcomes.size, outcomes.size - failed, failed, inputBytes, outputBytes, durationMs, outcomes)
    }

    private fun formatReport(r: BatchReport): String = buildString {
        appendLine("=== Watermelon Vector Converter — conversion report ===")
        appendLine("Total files: ${r.total}")
        appendLine("Succeeded:   ${r.succeeded}")
        appendLine("Failed:      ${r.failed}")
        appendLine("Input size:  ${r.inputBytes} bytes")
        appendLine("Output size: ${r.outputBytes} bytes")
        appendLine("Duration:    ${r.durationMillis} ms")
        if (r.rejected.isNotEmpty()) {
            appendLine("\nRejected files:")
            r.rejected.forEach { item -> appendLine("  - ${item.name}: ${item.errorMessage ?: "unknown"}") }
        }
    }
}
