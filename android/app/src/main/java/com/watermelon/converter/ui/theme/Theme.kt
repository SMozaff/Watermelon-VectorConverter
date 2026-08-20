// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val LightColors = lightColorScheme(
    primary = FreshTeal,
    onPrimary = PureWhite,
    primaryContainer = TealContainer,
    onPrimaryContainer = OnTealContainer,
    secondary = WatermelonRed,
    onSecondary = PureWhite,
    secondaryContainer = WatermelonContainer,
    onSecondaryContainer = OnWatermelonContainer,
    tertiary = DeepNavy,
    background = OffWhite,
    onBackground = Charcoal,
    surface = PureWhite,
    onSurface = Charcoal,
    surfaceVariant = LightSurfaceVariant,
    onSurfaceVariant = SlateGray,
    outline = SlateGray,
    outlineVariant = LightSurfaceVariant,
    error = WatermelonRed,
    onError = PureWhite,
    errorContainer = WatermelonContainer,
    onErrorContainer = OnWatermelonContainer,
)

private val DarkColors = darkColorScheme(
    primary = DarkTeal,
    onPrimary = DeepCharcoal,
    primaryContainer = FreshTeal,
    onPrimaryContainer = TealContainer,
    secondary = DarkWatermelon,
    onSecondary = OnWatermelonContainer,
    secondaryContainer = WatermelonRed,
    onSecondaryContainer = WatermelonContainer,
    tertiary = DarkTeal,
    background = DeepCharcoal,
    onBackground = OffWhite,
    surface = DarkSurface,
    onSurface = OffWhite,
    surfaceVariant = DarkSurfaceVariant,
    onSurfaceVariant = Color(0xFFC2CDC6),
    outline = Color(0xFF8C9991),
    outlineVariant = DarkSurfaceVariant,
    error = DarkWatermelon,
    onError = OnWatermelonContainer,
    errorContainer = WatermelonRed,
    onErrorContainer = WatermelonContainer,
)

@Composable
fun WatermelonTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    MaterialTheme(
        colorScheme = if (darkTheme) DarkColors else LightColors,
        typography = WatermelonTypography,
        content = content,
    )
}
