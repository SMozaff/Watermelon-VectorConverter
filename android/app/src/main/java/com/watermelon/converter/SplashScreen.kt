// Watermelon Vector Converter
// Copyright (c) 2026 Soheil Mozaffari. All rights reserved.

package com.watermelon.converter

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.delay

private const val SPLASH_DURATION_MS = 1500L
private val SplashBackground = Color(0xFF050C09)
private val SplashGreen = Color(0xFF64D399)
private val SplashTeal = Color(0xFF147A70)
private val SplashRed = Color(0xFFC62839)
private val SplashMint = Color(0xFFF7FAF8)

/**
 * A native reconstruction of the launch artwork's visual language. The
 * watermelon slice, conversion arrow, ambient light, lockup, and loading bar
 * are all drawn and animated by Compose instead of presenting a bitmap.
 */
@Composable
fun WatermelonSplash(onFinished: () -> Unit) {
    val revealed = remember { mutableStateOf(false) }
    val entrance by animateFloatAsState(
        targetValue = if (revealed.value) 1f else 0f,
        animationSpec = tween(durationMillis = 940, easing = FastOutSlowInEasing),
        label = "splash_entrance",
    )
    val ambience = rememberInfiniteTransition(label = "splash_ambience")
    val glowPulse by ambience.animateFloat(
        initialValue = 0.32f,
        targetValue = 0.82f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 900, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "splash_glow_pulse",
    )

    LaunchedEffect(Unit) {
        revealed.value = true
        delay(SPLASH_DURATION_MS)
        onFinished()
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(SplashBackground),
        contentAlignment = Alignment.Center,
    ) {
        Canvas(modifier = Modifier.fillMaxSize()) {
            val center = Offset(size.width * 0.50f, size.height * 0.42f)
            drawCircle(
                brush = Brush.radialGradient(
                    colors = listOf(
                        SplashGreen.copy(alpha = 0.20f * glowPulse),
                        SplashTeal.copy(alpha = 0.06f * glowPulse),
                        Color.Transparent,
                    ),
                    center = center,
                    radius = size.minDimension * 0.74f,
                ),
                radius = size.minDimension * 0.74f,
                center = center,
            )
            drawCircle(
                color = SplashRed.copy(alpha = 0.08f * (1f - glowPulse)),
                radius = size.minDimension * 0.36f,
                center = Offset(size.width * 0.28f, size.height * 0.57f),
            )
        }

        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 32.dp)
                .graphicsLayer(alpha = entrance),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            Canvas(
                modifier = Modifier
                    .fillMaxWidth(0.78f)
                    .aspectRatio(1.12f)
                    .graphicsLayer(
                        scaleX = 0.72f + (0.28f * entrance),
                        scaleY = 0.72f + (0.28f * entrance),
                        translationY = (1f - entrance) * 30f,
                    )
                    .semantics { contentDescription = "Animated watermelon conversion mark" },
            ) {
                val width = size.width
                val height = size.height
                val fruitCenter = Offset(width * 0.37f, height * 0.54f)
                val fruitSize = Size(width * 0.60f, height * 0.60f)
                val rindInset = width * 0.025f
                val innerInset = width * 0.072f

                drawArc(
                    color = SplashGreen.copy(alpha = 0.22f * glowPulse),
                    startAngle = 190f,
                    sweepAngle = 160f,
                    useCenter = true,
                    topLeft = Offset(fruitCenter.x - width * 0.38f, fruitCenter.y - height * 0.35f),
                    size = Size(width * 0.76f, height * 0.76f),
                )
                drawArc(
                    color = SplashTeal,
                    startAngle = 198f,
                    sweepAngle = 144f,
                    useCenter = true,
                    topLeft = Offset(fruitCenter.x - fruitSize.width / 2f, fruitCenter.y - fruitSize.height / 2f),
                    size = fruitSize,
                )
                drawArc(
                    color = SplashMint,
                    startAngle = 200f,
                    sweepAngle = 140f,
                    useCenter = true,
                    topLeft = Offset(
                        fruitCenter.x - fruitSize.width / 2f + rindInset,
                        fruitCenter.y - fruitSize.height / 2f + rindInset,
                    ),
                    size = Size(fruitSize.width - (rindInset * 2f), fruitSize.height - (rindInset * 2f)),
                )
                drawArc(
                    color = SplashRed,
                    startAngle = 202f,
                    sweepAngle = 136f,
                    useCenter = true,
                    topLeft = Offset(
                        fruitCenter.x - fruitSize.width / 2f + innerInset,
                        fruitCenter.y - fruitSize.height / 2f + innerInset,
                    ),
                    size = Size(fruitSize.width - (innerInset * 2f), fruitSize.height - (innerInset * 2f)),
                )

                listOf(
                    Offset(width * 0.25f, height * 0.49f),
                    Offset(width * 0.36f, height * 0.40f),
                    Offset(width * 0.48f, height * 0.49f),
                    Offset(width * 0.33f, height * 0.60f),
                    Offset(width * 0.43f, height * 0.58f),
                ).forEach { seed ->
                    drawOval(
                        color = SplashBackground.copy(alpha = 0.86f),
                        topLeft = Offset(seed.x - width * 0.012f, seed.y - height * 0.022f),
                        size = Size(width * 0.024f, height * 0.044f),
                    )
                }

                val arrowStart = Offset(width * 0.57f, height * 0.43f)
                val arrowEnd = Offset(width * 0.89f, height * 0.43f)
                drawLine(
                    color = SplashGreen,
                    start = arrowStart,
                    end = arrowEnd,
                    strokeWidth = width * 0.035f,
                    cap = StrokeCap.Round,
                )
                val arrowHead = Path().apply {
                    moveTo(width * 0.87f, height * 0.31f)
                    lineTo(width * 0.98f, height * 0.43f)
                    lineTo(width * 0.87f, height * 0.55f)
                    lineTo(width * 0.89f, height * 0.47f)
                    lineTo(width * 0.69f, height * 0.47f)
                    lineTo(width * 0.69f, height * 0.39f)
                    lineTo(width * 0.89f, height * 0.39f)
                    close()
                }
                drawPath(arrowHead, SplashMint.copy(alpha = 0.96f))
                drawCircle(
                    color = SplashGreen.copy(alpha = 0.22f * glowPulse),
                    center = Offset(width * 0.76f, height * 0.43f),
                    radius = width * 0.14f,
                )
            }

            Spacer(Modifier.height(14.dp))
            Text(
                text = "WATERMELON",
                style = MaterialTheme.typography.headlineMedium,
                fontWeight = FontWeight.Black,
                color = SplashMint,
            )
            Text(
                text = "VECTOR GRAPHICS CONVERTER",
                style = MaterialTheme.typography.labelMedium,
                color = SplashGreen,
                fontWeight = FontWeight.Bold,
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(28.dp))
            Surface(
                modifier = Modifier
                    .width(176.dp)
                    .height(5.dp)
                    .clip(RoundedCornerShape(99.dp)),
                color = SplashMint.copy(alpha = 0.14f),
            ) {
                Canvas(modifier = Modifier.fillMaxSize()) {
                    drawRoundRect(
                        brush = Brush.horizontalGradient(
                            listOf(SplashTeal, SplashGreen, SplashMint),
                            endX = size.width * entrance,
                        ),
                        size = Size(size.width * entrance, size.height),
                        cornerRadius = androidx.compose.ui.geometry.CornerRadius(size.height, size.height),
                    )
                    if (entrance > 0.02f) {
                        drawCircle(
                            color = SplashMint.copy(alpha = 0.72f),
                            radius = size.height * 0.45f,
                            center = Offset(size.width * entrance, size.height / 2f),
                            style = Stroke(width = 1f),
                        )
                    }
                }
            }
        }
    }
}
