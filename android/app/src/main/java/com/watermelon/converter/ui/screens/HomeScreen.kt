// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.ui.screens

import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.navigation.NavController
import com.watermelon.converter.R
import com.watermelon.converter.Routes
import com.watermelon.converter.ui.components.TransformationMotif
import com.watermelon.converter.ui.sharedGraphViewModel
import com.watermelon.converter.viewmodel.ConversionViewModel
import com.watermelon.converter.viewmodel.ReverseConversionViewModel
import com.watermelon.converter.util.SafAccess

/** A task-first landing surface for single and batch vector conversion. */
@Composable
fun HomeScreen(
    nav: NavController,
    convVm: ConversionViewModel = nav.sharedGraphViewModel(),
    revConvVm: ReverseConversionViewModel = nav.sharedGraphViewModel(),
) {
    val context = LocalContext.current
    val openPreview = remember(nav) {
        {
            nav.navigate(Routes.PREVIEW) {
                launchSingleTop = true
            }
        }
    }
    val svgPicker = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri != null) {
            SafAccess.persistReadGrant(context, uri)
            revConvVm.reset()
            convVm.convert(uri)
            openPreview()
        }
    }
    val xmlPicker = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri != null) {
            SafAccess.persistReadGrant(context, uri)
            convVm.reset()
            revConvVm.convert(uri)
            openPreview()
        }
    }

    Column(
        Modifier
            .fillMaxSize()
            .padding(horizontal = 20.dp, vertical = 20.dp),
        verticalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Image(
                painter = painterResource(id = R.drawable.we_stand_with_watermelon),
                contentDescription = "Watermelon Vector Converter",
                modifier = Modifier.size(72.dp),
                contentScale = ContentScale.Fit,
            )
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(
                    "Watermelon",
                    style = MaterialTheme.typography.headlineSmall,
                    fontWeight = FontWeight.Bold,
                )
                Text(
                    "Vector Converter",
                    style = MaterialTheme.typography.titleMedium,
                    color = MaterialTheme.colorScheme.primary,
                )
                Text(
                    "Offline SVG and Android VectorDrawable conversion",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }

        Text(
            "Choose a conversion",
            style = MaterialTheme.typography.titleMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(top = 8.dp),
        )

        ConversionOption(
            title = "SVG to VectorDrawable",
            subtitle = "Prepare an Android XML vector from an SVG file.",
            primaryLabel = "Choose SVG",
            isForward = true,
            onSingle = { svgPicker.launch(arrayOf("image/svg+xml", "text/xml", "*/*")) },
            onBatch = { nav.navigate(Routes.BATCH) },
        )

        ConversionOption(
            title = "VectorDrawable to SVG",
            subtitle = "Turn Android VectorDrawable XML back into SVG.",
            primaryLabel = "Choose XML",
            isForward = false,
            onSingle = { xmlPicker.launch(arrayOf("text/xml", "application/xml", "*/*")) },
            onBatch = { nav.navigate(Routes.BATCH_REVERSE) },
        )

        Spacer(Modifier.weight(1f))
        TextButton(
            onClick = { nav.navigate(Routes.ABOUT) },
            modifier = Modifier.align(Alignment.CenterHorizontally),
        ) {
            Text("About this app")
        }
    }
}

@Composable
private fun ConversionOption(
    title: String,
    subtitle: String,
    primaryLabel: String,
    isForward: Boolean,
    onSingle: () -> Unit,
    onBatch: () -> Unit,
) {
    ElevatedCard(
        Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(20.dp),
        colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surface),
    ) {
        Column(
            Modifier.padding(18.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Text(title, style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.Bold)
            TransformationMotif(isForward = isForward)
            Text(
                subtitle,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Spacer(Modifier.height(4.dp))
            // Single-file conversion is the primary, fastest workflow —
            // full-width button, not sharing a row with batch. Batch is
            // deliberately a secondary text/link-style action below it, per
            // the redesign prompt's explicit instruction: "Do NOT render
            // two equally weighted buttons... Use one clear primary
            // button; one secondary text/link-style batch action."
            Button(
                onClick = onSingle,
                shape = RoundedCornerShape(14.dp),
                modifier = Modifier.fillMaxWidth(),
            ) { Text(primaryLabel, textAlign = TextAlign.Center) }
            TextButton(
                onClick = onBatch,
                modifier = Modifier.fillMaxWidth(),
            ) { Text("Convert multiple files →") }
        }
    }
}
