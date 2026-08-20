// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.viewmodel

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.watermelon.converter.data.model.HistoryStore
import com.watermelon.converter.data.prefs.SettingsRepository
import com.watermelon.converter.data.repository.FileRepository
import com.watermelon.converter.jni.ConversionException
import com.watermelon.converter.jni.RealSvgConverter
import com.watermelon.converter.jni.SvgConverter
import com.watermelon.converter.jni.userMessage
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

sealed interface ConvertUiState {
    data object Idle : ConvertUiState
    data class Working(val sourceName: String) : ConvertUiState
    @Suppress("ArrayInDataClass")
    data class Done(
        val sourceName: String,
        val vdXml: String,
        val svgPreviewPng: ByteArray?,
        val vdPreviewPng: ByteArray?,
        val analysisJson: String? = null,   // structural analysis for the properties panel
    ) : ConvertUiState
    data class Error(
        val sourceName: String,
        val message: String,
        val retryAvailable: Boolean = true,
    ) : ConvertUiState
}

class ConversionViewModel(
    app: Application,
    private val native: SvgConverter,
) : AndroidViewModel(app) {

    /**
     * Constructor used by the Android ViewModel factory, which builds
     * ViewModels by reflection and requires a real (Application) constructor.
     * Kotlin default arguments do NOT produce one (they compile to a single
     * constructor with a synthetic mask arg), so we declare it explicitly and
     * delegate to the injectable constructor with the production bridge.
     */
    constructor(app: Application) : this(app, RealSvgConverter)


    private val repo = FileRepository(app.applicationContext)
    private val settingsRepo = SettingsRepository(app.applicationContext)

    private val _state = MutableStateFlow<ConvertUiState>(ConvertUiState.Idle)
    val state: StateFlow<ConvertUiState> = _state.asStateFlow()

    private var lastVdXml: String? = null
    private var lastSourceName: String? = null
    private var lastUri: Uri? = null

    fun convert(uri: Uri) {
        lastUri = uri
        lastVdXml = null
        lastSourceName = null
        _state.value = ConvertUiState.Working("Preparing SVG…")

        viewModelScope.launch {
            var name = "image.svg"
            try {
                name = withContext(Dispatchers.IO) { repo.displayName(uri) }
                _state.value = ConvertUiState.Working(name)
                val px = runCatching { settingsRepo.settings.first().previewPx }.getOrDefault(256)
                val result = withContext(Dispatchers.IO) {
                    val svgBytes = repo.readBytes(uri)
                    require(svgBytes.isNotEmpty()) { "The selected file is empty." }
                    val xml = native.convertSvg(svgBytes)
                    // Previews and structural analysis are optional enhancements; a
                    // successful conversion remains usable even if one fails.
                    val svgPngDef = async { runCatching { native.renderSvgPreview(svgBytes, px) }.getOrNull() }
                    val vdPngDef = async { runCatching { native.renderVdPreview(xml, px) }.getOrNull() }
                    val analysisDef = async { runCatching { native.analyzeVector(svgBytes) }.getOrNull() }
                    lastVdXml = xml
                    lastSourceName = name
                    ConvertUiState.Done(
                        sourceName = name,
                        vdXml = xml,
                        svgPreviewPng = svgPngDef.await(),
                        vdPreviewPng = vdPngDef.await(),
                        analysisJson = analysisDef.await(),
                    )
                }
                HistoryStore.add(result.sourceName, result.vdXml, ok = true)
                _state.value = result
            } catch (e: ConversionException) {
                val msg = e.userMessage(getApplication())
                HistoryStore.add(name, "", ok = false, error = msg)
                _state.value = ConvertUiState.Error(name, msg, retryAvailable = lastUri != null)
            } catch (e: Exception) {
                val msg = e.message ?: "Unknown error"
                HistoryStore.add(name, "", ok = false, error = msg)
                _state.value = ConvertUiState.Error(name, msg, retryAvailable = lastUri != null)
            }
        }
    }

    fun retry() {
        lastUri?.let(::convert)
    }

    /**
     * Export the converted VD XML to the user's chosen output destination.
     * If a Settings destination is configured, writes there directly (no
     * folder picker needed). Falls back to [treeUri] when called from the
     * Export screen (used when Settings destination is null).
     */
    fun export(treeUri: Uri? = null) {
        val xml  = lastVdXml ?: return
        val name = lastSourceName ?: "image.svg"
        viewModelScope.launch(Dispatchers.IO) {
            runCatching {
                val destUri = settingsRepo.settings.first().outputDestinationUri
                if (destUri != null) {
                    // Write directly to the Settings-configured destination via SAF.
                    com.watermelon.converter.util.OutputDestination.write(
                        getApplication(), xml.toByteArray(),
                        repo.xmlNameFor(name), destUri, mime = "application/xml",
                    )
                } else if (treeUri != null) {
                    // Fallback: use the URI from the ExportScreen folder picker.
                    repo.writeToTree(treeUri, repo.xmlNameFor(name), "application/xml", xml.toByteArray())
                }
            }
        }
    }

    fun currentXml(): String? = lastVdXml

    fun reset() {
        _state.value = ConvertUiState.Idle
        lastVdXml = null
        lastSourceName = null
        lastUri = null
    }
}
