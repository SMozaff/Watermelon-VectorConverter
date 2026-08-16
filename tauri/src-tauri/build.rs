fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "convert_svg",
            "convert_vd",
            "render_svg_preview",
            "render_vd_preview",
            "convert_zip",
            "convert_vd_zip",
            "zip_loose_files",
            "open_url",
            "set_file_association",
            "get_file_association",
            "detect_animation",
            "render_avd_frames",
            "pick_input_files",
            "save_output_file",
        ]),
    ))
    .expect("failed to configure Tauri command permissions");
}
