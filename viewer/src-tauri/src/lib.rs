// Watermelon Vector Viewer
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
//
// Single-instance file viewer for SVG and VectorDrawable XML files.
// Registered as the default handler for .svg / .xml on install.
// Opening a new file while already running re-uses the same window.

use std::sync::Mutex;
use tauri::{Emitter, Manager};

mod commands;

/// Holds the path of the file most recently requested to open,
/// so the frontend can pull it after the webview is ready.
pub struct PendingFile(pub Mutex<Option<String>>);

/// Extract the first plausible file path from CLI args (skip the binary name
/// and any flags).
fn extract_file_path(args: &[String]) -> Option<String> {
    args.iter().skip(1).find(|a| !a.starts_with('-')).cloned()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            // A second launch happened — argv[1] is the new file path.
            // Push it to the existing window instead of opening a new one.
            if let Some(path) = extract_file_path(&argv) {
                if let Some(state) = app.try_state::<PendingFile>() {
                    if let Ok(mut pending) = state.0.lock() {
                        *pending = Some(path);
                    }
                }
                let _ = app.emit("viewer://file-ready", ());
            }
            // Bring the existing window to front.
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.set_focus();
                let _ = w.unminimize();
            }
        }))
        .manage(PendingFile(Mutex::new(extract_file_path(
            &std::env::args().collect::<Vec<_>>(),
        ))))
        .invoke_handler(tauri::generate_handler![
            commands::pick_file_preview,
            commands::take_pending_file_preview,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Watermelon Vector Viewer");
}
