// Watermelon Vector Converter — Desktop
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
//
// Single binary, two modes:
//   - No file argument (or an unrecognized flag) -> full converter UI (and,
//     on Windows, (re-)registers the .svg/.xml file associations — see
//     file_assoc/).
//   - One argument that is an existing .svg/.xml file path -> lightweight
//     viewer mode, opened by double-clicking a file via the OS file
//     association (see file_assoc/).
//
// Previously this was two separate crates (tauri/, viewer/) glued together
// by Tauri's externalBin sidecar mechanism. Folding both into one binary
// removes that whole mechanism — no more sidecar path resolution, no
// separate installer-bundling step for a second executable, and (the
// actual point of this rewrite) no WebView2/WebKitGTK dependency anywhere.

mod app;
#[cfg(windows)]
mod file_assoc;
mod single_instance;

use std::path::PathBuf;

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().collect();
    let file_arg: Option<PathBuf> = args
        .iter()
        .skip(1)
        .find(|a| !a.starts_with('-'))
        .map(PathBuf::from)
        .filter(|p| p.exists());

    match file_arg {
        Some(path) => {
            // Viewer mode. If another instance of the viewer is already
            // running, hand it the path and exit instead of opening a
            // second window — the single-instance behavior the old
            // tauri-plugin-single-instance provided.
            if single_instance::notify_existing_instance(&path) {
                return Ok(());
            }
            app::run_viewer(path)
        }
        None => {
            // Converter-mode startup (launched from the Start Menu, not
            // via a file association) is also where file-association
            // registration runs — idempotent, so re-running it on every
            // ordinary launch is harmless, and this avoids needing a
            // separate MSI custom action or first-run flag. Best-effort:
            // a registration failure (e.g. no write access in some
            // locked-down environment) is logged, not fatal — the
            // converter itself doesn't depend on the association existing.
            #[cfg(windows)]
            if let Err(e) = file_assoc::register_current_exe() {
                eprintln!("Watermelon: file association registration failed: {e}");
            }
            app::run_converter()
        }
    }
}
