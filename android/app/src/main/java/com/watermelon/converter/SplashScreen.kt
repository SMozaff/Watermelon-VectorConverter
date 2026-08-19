// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

package com.watermelon.converter

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay

private val SplashBackground = Color(0xFF050C09)
private val SplashRind = Color(0xFF40C556)
private val SplashGlow = Color(0xFF8AEB4E)
private val SplashMuted = Color(0xFF71DA7A)

/**
 * A short branded launch treatment. The staged progress is deliberately
 * simulated so the first frame is polished while the app initializes.
 */
@Composable
fun WatermelonSplash(onFinished: () -> Unit) {
    val targetProgress = remember { mutableFloatStateOf(0f) }
    val progress by animateFloatAsState(
        targetValue = targetProgress.floatValue,
        animationSpec = tween(durationMillis = 90),
        label = "splash_progress",
    )

    LaunchedEffect(Unit) {
        repeat(25) { step ->
            targetProgress.floatValue = (step + 1) * 4f
            delay(90)
        }
        onFinished()
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(SplashBackground)
            .padding(horizontal = 36.dp, vertical = 32.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(
            text = "↑",
            color = SplashGlow,
            fontSize = 64.sp,
            fontWeight = FontWeight.Black,
            lineHeight = 64.sp,
        )
        Text(
            text = "▪  ▪  ▪",
            color = SplashMuted,
            fontSize = 14.sp,
            letterSpacing = 4.sp,
        )
        Spacer(Modifier.height(8.dp))
        Image(
            painter = painterResource(R.drawable.watermelon_splash),
            contentDescription = "Watermelon conversion illustration",
            contentScale = ContentScale.Fit,
            modifier = Modifier.size(208.dp),
        )
        Spacer(Modifier.height(16.dp))
        Text(
            text = "WATERMELON",
            color = Color.White,
            fontSize = 30.sp,
            fontWeight = FontWeight.Black,
            letterSpacing = 3.sp,
            textAlign = TextAlign.Center,
        )
        Text(
            text = "VECTOR CONVERTER",
            color = SplashRind,
            fontSize = 15.sp,
            fontWeight = FontWeight.Bold,
            letterSpacing = 3.sp,
            textAlign = TextAlign.Center,
        )
        Spacer(Modifier.height(46.dp))
        Text(
            text = if (progress >= 96f) "READY" else "LAUNCHING…",
            color = SplashMuted,
            fontSize = 13.sp,
            fontWeight = FontWeight.Bold,
            letterSpacing = 4.sp,
        )
        Spacer(Modifier.height(12.dp))
        LinearProgressIndicator(
            progress = { progress / 100f },
            modifier = Modifier
                .width(280.dp)
                .height(10.dp),
            color = SplashGlow,
            trackColor = Color(0xFF0C2316),
        )
        Spacer(Modifier.height(8.dp))
        Text(
            text = "${progress.toInt()}%",
            color = SplashRind,
            fontSize = 12.sp,
            fontWeight = FontWeight.SemiBold,
        )
    }
}
