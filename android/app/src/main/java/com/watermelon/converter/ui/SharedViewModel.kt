// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.

package com.watermelon.converter.ui

import androidx.compose.runtime.Composable
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewmodel.compose.viewModel
import androidx.navigation.NavController
import androidx.compose.runtime.remember
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.currentBackStackEntryAsState

/** Returns a ViewModel scoped to the nav graph's start destination, so the
 *  same instance is shared across Import -> Preview -> Export (and Batch -> Export).
 *  Without this, viewModel() would hand each screen its OWN instance and the
 *  converted data would be lost between screens. */
@Composable
inline fun <reified T : ViewModel> NavController.sharedGraphViewModel(): T {
    val currentBackStackEntry = currentBackStackEntryAsState().value
    val entry = remember(currentBackStackEntry) {
        getBackStackEntry(graph.findStartDestination().id)
    }
    return viewModel(viewModelStoreOwner = entry)
}
