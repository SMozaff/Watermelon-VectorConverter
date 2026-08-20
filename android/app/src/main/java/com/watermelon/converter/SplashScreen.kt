// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

package com.watermelon.converter

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

private const val SPLASH_DURATION_MS = 1500L

/**
 * Presents the supplied Watermelon launch artwork as a brief product-identity
 * moment. The image itself already contains the product lockup and launch bar,
 * so the native animation intentionally adds only an entrance and ambient glow.
 */
@Composable
fun WatermelonSplash(onFinished: () -> Unit) {
    val revealed = remember { mutableStateOf(false) }
    val alpha by animateFloatAsState(
        targetValue = if (revealed.value) 1f else 0f,
        animationSpec = tween(durationMillis = 420, easing = FastOutSlowInEasing),
        label = "launch_art_alpha",
    )
    val scale by animateFloatAsState(
        targetValue = if (revealed.value) 1f else 0.91f,
        animationSpec = tween(durationMillis = 720, easing = FastOutSlowInEasing),
        label = "launch_art_scale",
    )
    val ambient = rememberInfiniteTransition(label = "launch_ambient")
    val glowAlpha by ambient.animateFloat(
        initialValue = 0.12f,
        targetValue = 0.28f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 900, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "launch_glow_alpha",
    )

    LaunchedEffect(Unit) {
        revealed.value = true
        delay(SPLASH_DURATION_MS)
        onFinished()
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black),
        contentAlignment = Alignment.Center,
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(
                    Brush.radialGradient(
                        colors = listOf(
                            Color(0x4A49FF63).copy(alpha = glowAlpha),
                            Color.Transparent,
                        ),
                        radius = 920f,
                    ),
                ),
        )
        Image(
            painter = painterResource(R.drawable.watermelon_launch_art),
            contentDescription = "Watermelon Vector Converter launching",
            modifier = Modifier
                .fillMaxWidth(0.9f)
                .aspectRatio(1f)
                .padding(8.dp)
                .graphicsLayer(
                    alpha = alpha,
                    scaleX = scale,
                    scaleY = scale,
                ),
            contentScale = ContentScale.Fit,
        )
    }
}
