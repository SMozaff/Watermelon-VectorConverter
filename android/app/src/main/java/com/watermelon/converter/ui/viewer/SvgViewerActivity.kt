// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Lightweight viewer Activity — opened via ACTION_VIEW on SVG or
// VectorDrawable XML files. Detects which by root tag (<svg> vs <vector>),
// not by file extension or declared MIME type, so a misnamed file still
// previews correctly.
//
// Contract C-5 routing: after the static SVG-vs-VD detection above, a
// second check (nativeDetectAnimation) decides how the file is actually
// displayed:
//   AnimationKind.NONE      -> existing static bitmap preview (unchanged)
//   AnimationKind.AVD       -> render_avd_frames -> AnimationDrawable (C-5.3)
//   AnimationKind.SVG_SMIL/
//   AnimationKind.SVG_CSS   -> WebView, offline-locked (C-5.5)

package com.watermelon.converter.ui.viewer

import android.animation.AnimationDrawable
import android.graphics.Bitmap
import android.graphics.BitmapDrawable
import android.graphics.BitmapFactory
import android.net.Uri
import android.os.Bundle
import android.webkit.WebView
import android.widget.ImageView
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import com.watermelon.converter.jni.AnimationKind
import com.watermelon.converter.jni.AvdFramesResult
import com.watermelon.converter.jni.SvgConverterNative
import java.util.Base64

class SvgViewerActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val uri = intent?.data
        setContent {
            MaterialTheme { SvgViewerScreen(uri = uri, onClose = { finish() }) }
        }
    }
}

/** What the viewer actually ended up loading, once detection has run. */
private sealed interface ViewerContent {
    data class Static(val bitmap: Bitmap) : ViewerContent
    data class Avd(val frames: AvdFramesResult) : ViewerContent
    data class Animated(val htmlDataUri: String) : ViewerContent
}

@Composable
private fun SvgViewerScreen(uri: Uri?, onClose: () -> Unit) {
    val context = LocalContext.current
    var content by remember { mutableStateOf<ViewerContent?>(null) }
    var error by remember { mutableStateOf<String?>(null) }
    var loading by remember { mutableStateOf(true) }
    var fileName by remember { mutableStateOf("") }

    var scale by remember { mutableFloatStateOf(1f) }
    var offset by remember { mutableStateOf(Offset.Zero) }

    LaunchedEffect(uri) {
        if (uri == null) { loading = false; error = "No file provided."; return@LaunchedEffect }
        try {
            val bytes = context.contentResolver.openInputStream(uri)?.readBytes()
                ?: throw Exception("Cannot read file")
            fileName = uri.lastPathSegment?.substringAfterLast('/') ?: ""

            // Detect VectorDrawable XML vs. plain SVG by root tag rather than
            // file extension or the intent's declared MIME type — a renamed
            // or misdeclared file still previews correctly either way. This
            // mirrors the desktop viewer's render_file_preview detection.
            val text = String(bytes, Charsets.UTF_8)
            val firstTag = text
                .lineSequence()
                .map { it.trim() }
                .firstOrNull { it.isNotEmpty() && !it.startsWith("<?xml") }
                ?: ""
            val isVectorDrawable = firstTag.startsWith("<vector") || firstTag.startsWith("<animated-vector")

            // Contract C-5.1: is this file animated, and how?
            val animKind = AnimationKind.fromOrdinal(
                SvgConverterNative.nativeDetectAnimation(bytes, isVectorDrawable)
            )

            content = when (animKind) {
                AnimationKind.AVD -> {
                    // Contract C-5.2/C-5.3: render frames natively, play via
                    // AnimationDrawable — NOT AnimatedVectorDrawable, which
                    // requires compiled resources this runtime file doesn't have.
                    val json = SvgConverterNative.nativeRenderAvdFrames(bytes, 30, 90, 512)
                    ViewerContent.Avd(AvdFramesResult.decode(json))
                }
                AnimationKind.SVG_SMIL, AnimationKind.SVG_CSS -> {
                    // Contract C-5.5: hand the raw SVG to a locked-down WebView.
                    // Built as a data: URI (no filesystem/content:// exposure
                    // to the WebView, and no network path exists regardless).
                    val encoded = Base64.getEncoder().encodeToString(bytes)
                    ViewerContent.Animated("data:image/svg+xml;base64,$encoded")
                }
                AnimationKind.NONE -> {
                    val pngBytes = if (isVectorDrawable) {
                        SvgConverterNative.nativeRenderVdPreview(text, 1024)
                    } else {
                        SvgConverterNative.nativeRenderSvgPreview(bytes, 1024)
                    }
                    val bmp = BitmapFactory.decodeByteArray(pngBytes, 0, pngBytes.size)
                        ?: throw Exception("Could not decode preview")
                    ViewerContent.Static(bmp)
                }
            }
        } catch (e: Exception) {
            error = e.message
        } finally {
            loading = false
        }
    }

    Box(modifier = Modifier.fillMaxSize().background(Color(0xFF15181C))) {
        when {
            loading -> CircularProgressIndicator(
                modifier = Modifier.align(Alignment.Center),
                color = Color(0xFF2A9D8F)
            )
            error != null -> Text(
                text = "⚠ $error",
                color = Color(0xFFE63946),
                modifier = Modifier.align(Alignment.Center).padding(32.dp),
                fontSize = 14.sp
            )
            content is ViewerContent.Static -> Image(
                bitmap = (content as ViewerContent.Static).bitmap.asImageBitmap(),
                contentDescription = fileName,
                contentScale = ContentScale.Fit,
                modifier = Modifier
                    .fillMaxSize()
                    .pointerInput(Unit) {
                        detectTransformGestures { _, pan, zoom, _ ->
                            scale = (scale * zoom).coerceIn(0.5f, 8f)
                            offset += pan
                        }
                    }
                    .graphicsLayer(
                        scaleX = scale, scaleY = scale,
                        translationX = offset.x, translationY = offset.y
                    )
            )
            content is ViewerContent.Avd -> AvdPlaybackView(
                frames = (content as ViewerContent.Avd).frames,
                modifier = Modifier.fillMaxSize(),
            )
            content is ViewerContent.Animated -> AnimatedSvgWebView(
                dataUri = (content as ViewerContent.Animated).htmlDataUri,
                modifier = Modifier.fillMaxSize(),
            )
        }

        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(Color(0xCC1C2128))
                .padding(horizontal = 12.dp, vertical = 8.dp)
                .align(Alignment.TopStart),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceBetween
        ) {
            Text(
                text = fileName,
                color = Color(0xFF9AA5B1),
                fontSize = 13.sp,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
                modifier = Modifier.weight(1f)
            )
            IconButton(onClick = onClose, modifier = Modifier.size(36.dp)) {
                Text("✕", color = Color.White, fontSize = 16.sp)
            }
        }
    }
}

/**
 * Contract C-5.3 playback: decode each frame PNG into a Bitmap, assemble an
 * AnimationDrawable with per-frame durations from frame_durations_ms, host
 * it in a classic ImageView via AndroidView (Compose has no first-class
 * AnimationDrawable equivalent), and start it once inflated.
 */
@Composable
private fun AvdPlaybackView(frames: AvdFramesResult, modifier: Modifier = Modifier) {
    AndroidView(
        modifier = modifier,
        factory = { ctx ->
            ImageView(ctx).apply {
                scaleType = ImageView.ScaleType.FIT_CENTER
                val drawable = AnimationDrawable().apply {
                    frames.frames.forEachIndexed { i, png ->
                        val bmp = BitmapFactory.decodeByteArray(png, 0, png.size) ?: return@forEachIndexed
                        val durationMs = frames.frameDurationsMs.getOrElse(i) { 33 }
                        addFrame(BitmapDrawable(ctx.resources, bmp), durationMs)
                    }
                    // AnimationDrawable only has "loop forever" (isOneShot=false)
                    // or "play once" (isOneShot=true) — LoopMode.Reverse isn't
                    // representable natively here. Approximated as Repeat;
                    // exact ping-pong playback is a future refinement, not a
                    // regression from the static-preview baseline this
                    // replaces.
                    isOneShot = frames.loopMode == com.watermelon.converter.jni.LoopMode.ONCE
                }
                setImageDrawable(drawable)
                drawable.start()
            }
        },
    )
}

/**
 * Contract C-5.5 (Android half): SMIL/CSS-animated SVGs are handed to a
 * WebView rather than the native resvg renderer, since neither SMIL timing
 * nor CSS @keyframes are implemented in Rust (and re-implementing a CSS
 * engine there would be wasted effort — every WebView already has one).
 *
 * Offline safety is enforced exactly per the frozen contract:
 *   - setBlockNetworkLoads(true): no network request can ever leave this
 *     WebView, regardless of what the SVG references.
 *   - addJavascriptInterface is never called: no JS bridge is exposed, so
 *     even with JS enabled (kept on — some engines need it for CSS
 *     animation timing) page script has no path back into the app.
 *   - The content is loaded as a data: URI built from the file's own
 *     bytes, never a file:// or content:// URI — no filesystem path is
 *     ever reachable from inside the WebView's origin.
 */
@Composable
private fun AnimatedSvgWebView(dataUri: String, modifier: Modifier = Modifier) {
    AndroidView(
        modifier = modifier,
        factory = { ctx ->
            WebView(ctx).apply {
                settings.javaScriptEnabled = true
                settings.blockNetworkLoads = true
                settings.allowFileAccess = false
                settings.allowContentAccess = false
                setBackgroundColor(android.graphics.Color.TRANSPARENT)
                loadUrl(dataUri)
            }
        },
    )
}
