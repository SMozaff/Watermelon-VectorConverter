fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new()
                .commands(&["pick_file_preview", "take_pending_file_preview"]),
        ),
    )
    .expect("failed to configure Tauri command permissions");
}
