// Watermelon Vector Converter
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. See LICENSE.

//! Tauri commands wrapping the Rust core (Contract C-3, desktop side).
//! Logic lives in free fns (do_*) so they remain testable without a webview.
//! #[tauri::command] wrappers are thin delegation only.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::atomic::AtomicBool;
use svg_converter_core::animation::{AnimationKind, FileKind};
use svg_converter_core::animation_engine::{AnimationFrames, LoopMode};
use svg_converter_core::batch_processor::{
    convert_vd_zip as core_convert_vd_zip, convert_zip as core_convert_zip, zip_files_into_archive,
    ProgressEvent,
};
use svg_converter_core::convert_svg as core_convert_svg;
use svg_converter_core::convert_vd as core_convert_vd;
use svg_converter_core::detect_animation as core_detect_animation;
use svg_converter_core::error::ConversionError;
use svg_converter_core::image_export::{
    render_svg_preview as core_render_svg, render_vd_preview as core_render_vd,
};
use svg_converter_core::limits::{
    MAX_SINGLE_INPUT_BYTES, MAX_ZIP_INPUT_BYTES, MAX_ZIP_OUTPUT_BYTES,
};
use svg_converter_core::render_avd_frames as core_render_avd_frames;

const MAX_NATIVE_OUTPUT_BYTES: usize = MAX_ZIP_OUTPUT_BYTES;
use tauri::Emitter;

// ── Error DTO ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ConversionErrorDto {
    pub code: u16,
    pub message: String,
}

impl From<ConversionError> for ConversionErrorDto {
    fn from(e: ConversionError) -> Self {
        ConversionErrorDto {
            code: e.code(),
            message: e.to_string(),
        }
    }
}

/// Progress payload emitted to the frontend as the "batch://progress" event.
#[derive(Clone, Serialize)]
pub struct BatchProgressDto {
    pub done: u32,
    pub total: u32,
    pub current_name: String,
}

impl From<ProgressEvent> for BatchProgressDto {
    fn from(e: ProgressEvent) -> Self {
        BatchProgressDto {
            done: e.done,
            total: e.total,
            current_name: e.current_name,
        }
    }
}

/// Frame sequence result for the Animation Preview Engine (Contract C-5.2).
/// Mirrors `AnimationFrames` field-for-field; `loop_mode` is serialized as
/// a string (not a numeric ordinal like the JNI side needs) since Tauri's
/// normal JSON IPC handles enums-as-strings cleanly via serde, and frames
/// go over the same normal JSON path as `Vec<Vec<u8>>` — AVD frame counts
/// and sizes are small relative to the ZIP case that justified `convert_zip`'s
/// raw-body optimization, so no special-casing is needed here.
#[derive(Debug, Serialize)]
pub struct AvdFramesDto {
    pub width: u32,
    pub height: u32,
    pub frame_durations_ms: Vec<u32>,
    pub frames: Vec<Vec<u8>>,
    pub loop_mode: String,
}

impl From<AnimationFrames> for AvdFramesDto {
    fn from(f: AnimationFrames) -> Self {
        AvdFramesDto {
            width: f.width,
            height: f.height,
            frame_durations_ms: f.frame_durations_ms,
            frames: f.frames,
            loop_mode: match f.loop_mode {
                LoopMode::Once => "Once".to_string(),
                LoopMode::Repeat => "Repeat".to_string(),
                LoopMode::Reverse => "Reverse".to_string(),
            },
        }
    }
}

// ── Testable logic (no Tauri dependency) ────────────────────────────────────

pub fn do_convert_svg(svg: Vec<u8>) -> Result<String, ConversionErrorDto> {
    core_convert_svg(&svg).map_err(Into::into)
}

/// Reverse direction: VectorDrawable XML bytes -> SVG string.
pub fn do_convert_vd(vd_xml: Vec<u8>) -> Result<String, ConversionErrorDto> {
    core_convert_vd(&vd_xml).map_err(Into::into)
}

pub fn do_render_svg_preview(svg: Vec<u8>, px: u32) -> Result<Vec<u8>, ConversionErrorDto> {
    core_render_svg(&svg, px).map_err(Into::into)
}

pub fn do_render_vd_preview(vd_xml: String, px: u32) -> Result<Vec<u8>, ConversionErrorDto> {
    core_render_vd(&vd_xml, px).map_err(Into::into)
}

/// Detect whether a file is animated, and how (Contract C-5.1). Returns the
/// variant name as a plain string — simplest possible contract for the
/// frontend; unlike the JNI side there's no need for a numeric ordinal
/// since Tauri's JSON IPC serializes this cleanly as-is.
pub fn do_detect_animation(file_bytes: Vec<u8>, is_avd: bool) -> String {
    let kind = if is_avd { FileKind::Avd } else { FileKind::Svg };
    match core_detect_animation(&file_bytes, kind) {
        AnimationKind::None => "None",
        AnimationKind::Avd => "Avd",
        AnimationKind::SvgSmil => "SvgSmil",
        AnimationKind::SvgCss => "SvgCss",
    }
    .to_string()
}

/// Render an AVD's animation frames (Contract C-5.2). Surfaces
/// `UnsupportedFeature` (and any other `ConversionError`) as a normal
/// `Err(ConversionErrorDto)` — this is the expected, catchable result while
/// the underlying C-5.2 engine is still landing, not a bug in this bridge.
pub fn do_render_avd_frames(
    avd_bytes: Vec<u8>,
    fps: u32,
    max_frames: u32,
    px: u32,
) -> Result<AvdFramesDto, ConversionErrorDto> {
    core_render_avd_frames(&avd_bytes, fps, max_frames, px)
        .map(Into::into)
        .map_err(Into::into)
}

/// Bytes returned only from a native-dialog file selection. The frontend never
/// supplies a path to this command, which prevents arbitrary webview content
/// from turning the Rust backend into a filesystem read primitive.
#[derive(Debug, Serialize)]
pub struct PickedInputFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

fn command_error(code: u16, message: impl Into<String>) -> ConversionErrorDto {
    ConversionErrorDto {
        code,
        message: message.into(),
    }
}

fn safe_file_name(name: &str) -> Result<&str, ConversionErrorDto> {
    let path = Path::new(name);
    let is_single_normal_component = path.components().count() == 1
        && path.file_name().and_then(|value| value.to_str()) == Some(name);
    if name.is_empty() || !is_single_normal_component {
        return Err(command_error(
            1008,
            "output name must be a single file name",
        ));
    }
    Ok(name)
}

fn input_limit_for(path: &Path) -> u64 {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        MAX_ZIP_INPUT_BYTES as u64
    } else {
        MAX_SINGLE_INPUT_BYTES as u64
    }
}

fn read_dialog_selected_file(path: &Path) -> Result<PickedInputFile, ConversionErrorDto> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        command_error(1005, format!("could not inspect selected file: {error}"))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(command_error(1005, "selected path is not a regular file"));
    }
    let input_limit = input_limit_for(path);
    if metadata.len() > input_limit {
        return Err(command_error(
            1008,
            format!(
                "selected file exceeds the {} MiB input limit",
                input_limit / 1024 / 1024
            ),
        ));
    }
    let bytes = fs::read(path)
        .map_err(|error| command_error(1005, format!("could not read selected file: {error}")))?;
    if bytes.len() as u64 > input_limit {
        return Err(command_error(
            1008,
            "selected file grew beyond the input limit while reading",
        ));
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| command_error(1005, "selected file name is not valid UTF-8"))?
        .to_owned();
    Ok(PickedInputFile { name, bytes })
}

/// Display a native file picker and return bounded byte buffers. The caller
/// selects only one accepted content family, so no path crosses IPC.
#[tauri::command]
pub fn pick_input_files(kind: String) -> Result<Vec<PickedInputFile>, ConversionErrorDto> {
    let paths = match kind.as_str() {
        "svg" => rfd::FileDialog::new()
            .add_filter("SVG or ZIP", &["svg", "zip"])
            .pick_files(),
        "xml" => rfd::FileDialog::new()
            .add_filter("VectorDrawable XML or ZIP", &["xml", "zip"])
            .pick_files(),
        _ => return Err(command_error(1000, "unsupported file selection kind")),
    };

    paths
        .unwrap_or_default()
        .iter()
        .map(|path| read_dialog_selected_file(path))
        .collect()
}

/// Display a native save dialog and write a bounded output buffer. The output
/// path remains in the Rust backend and is never supplied by the webview.
#[tauri::command]
pub fn save_output_file(
    suggested_name: String,
    bytes: Vec<u8>,
) -> Result<bool, ConversionErrorDto> {
    let suggested_name = safe_file_name(&suggested_name)?;
    if bytes.len() > MAX_NATIVE_OUTPUT_BYTES {
        return Err(command_error(
            1008,
            "generated output exceeds the 200 MiB export limit",
        ));
    }
    let Some(path) = rfd::FileDialog::new()
        .set_file_name(suggested_name)
        .save_file()
    else {
        return Ok(false);
    };
    fs::write(path, bytes)
        .map_err(|error| command_error(1005, format!("could not write output file: {error}")))?;
    Ok(true)
}

// ── Tauri command wrappers ───────────────────────────────────────────────────

/// Convert a single SVG file (bytes) → VectorDrawable XML string.
#[tauri::command]
pub fn convert_svg(svg: Vec<u8>) -> Result<String, ConversionErrorDto> {
    do_convert_svg(svg)
}

/// Reverse direction: convert a single VectorDrawable XML file (bytes) → SVG string.
#[tauri::command]
pub fn convert_vd(vd_xml: Vec<u8>) -> Result<String, ConversionErrorDto> {
    do_convert_vd(vd_xml)
}

/// Render an SVG preview PNG at the requested pixel size.
#[tauri::command]
pub fn render_svg_preview(svg: Vec<u8>, px: u32) -> Result<Vec<u8>, ConversionErrorDto> {
    do_render_svg_preview(svg, px)
}

/// Render a VectorDrawable XML preview PNG at the requested pixel size.
#[tauri::command]
pub fn render_vd_preview(vd_xml: String, px: u32) -> Result<Vec<u8>, ConversionErrorDto> {
    do_render_vd_preview(vd_xml, px)
}

/// Detect whether a file is animated, and how (Contract C-5.1).
/// Returns the AnimationKind variant name as a string: "None", "Avd",
/// "SvgSmil", or "SvgCss".
#[tauri::command]
pub fn detect_animation(file_bytes: Vec<u8>, is_avd: bool) -> String {
    do_detect_animation(file_bytes, is_avd)
}

/// Render an AVD's animation frames at the requested fps/frame cap/pixel
/// size (Contract C-5.2). Returns `Err(ConversionErrorDto)` — including
/// code 1002 (UnsupportedFeature) until the C-5.2 engine itself is fully
/// implemented — rather than ever panicking the backend.
#[tauri::command]
pub fn render_avd_frames(
    avd_bytes: Vec<u8>,
    fps: u32,
    max_frames: u32,
    px: u32,
) -> Result<AvdFramesDto, ConversionErrorDto> {
    do_render_avd_frames(avd_bytes, fps, max_frames, px)
}

/// Batch-convert a ZIP of SVG files → ZIP of VectorDrawable XML files.
/// Emits "batch://progress" events to the frontend as each file completes.
///
/// Takes the raw request body instead of a Vec<u8> parameter: JSON-encoding
/// a large ZIP as a `number[]` (Tauri's default IPC for Vec<u8>) is slow
/// enough to look like a hang for real-world batch sizes. The frontend must
/// send this as a raw ArrayBuffer/Uint8Array body, not a JSON `{ zip: [...] }`
/// argument.
#[tauri::command]
pub fn convert_zip(
    window: tauri::Window,
    request: tauri::ipc::Request<'_>,
) -> Result<Vec<u8>, ConversionErrorDto> {
    let zip = raw_body_bytes(&request)?;
    let cancel = AtomicBool::new(false);
    let result = core_convert_zip(
        &zip,
        &|e: ProgressEvent| {
            // Emit progress to the frontend. Ignore emit errors (window may close).
            let _ = window.emit("batch://progress", BatchProgressDto::from(e));
        },
        &cancel,
    );
    result.map_err(Into::into)
}

/// Reverse batch: ZIP of VectorDrawable .xml files → ZIP of .svg files.
/// Emits "batch://progress" events on the same channel as convert_zip — the
/// frontend already listens once and only one batch direction runs at a time.
/// See convert_zip's doc comment for why this takes a raw Request body.
#[tauri::command]
pub fn convert_vd_zip(
    window: tauri::Window,
    request: tauri::ipc::Request<'_>,
) -> Result<Vec<u8>, ConversionErrorDto> {
    let zip = raw_body_bytes(&request)?;
    let cancel = AtomicBool::new(false);
    let result = core_convert_vd_zip(
        &zip,
        &|e: ProgressEvent| {
            let _ = window.emit("batch://progress", BatchProgressDto::from(e));
        },
        &cancel,
    );
    result.map_err(Into::into)
}

/// Extract raw bytes from an IPC request body, rejecting non-binary bodies
/// with a normal ConversionErrorDto instead of panicking.
fn raw_body_bytes(request: &tauri::ipc::Request<'_>) -> Result<Vec<u8>, ConversionErrorDto> {
    match request.body() {
        tauri::ipc::InvokeBody::Raw(bytes) => Ok(bytes.clone()),
        _ => Err(ConversionErrorDto {
            code: 1000,
            message: "expected a raw binary request body".to_string(),
        }),
    }
}

/// One loose file selected/dropped alongside others, to be zipped together
/// before running through the existing batch path.
#[derive(Deserialize)]
pub struct LooseFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// Package multiple loose files (selected/dropped together, not already a
/// ZIP) into one in-memory ZIP, so the frontend's "multi-select" case can
/// reuse convert_zip/convert_vd_zip rather than a separate pipeline.
#[tauri::command]
pub fn zip_loose_files(files: Vec<LooseFile>) -> Result<Vec<u8>, ConversionErrorDto> {
    let pairs: Vec<(String, Vec<u8>)> = files.into_iter().map(|f| (f.name, f.bytes)).collect();
    zip_files_into_archive(&pairs).map_err(Into::into)
}

fn validate_external_url(url: &str) -> Result<(), ConversionErrorDto> {
    const PROJECT_URL: &str = "https://github.com/muzaff-beep/Watermelon-VectorConverter";
    const CONTACT_URL: &str = "mailto:so.muzaff@gmail.com";
    if url == PROJECT_URL || url == CONTACT_URL {
        Ok(())
    } else {
        Err(command_error(1000, "unapproved external URL"))
    }
}

/// Open one of the application-owned URLs in the system default browser.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), ConversionErrorDto> {
    validate_external_url(&url)?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd")
        .args(["/c", "start", "", &url])
        .spawn()
        .map_err(|e| ConversionErrorDto {
            code: 1099,
            message: e.to_string(),
        })?;

    #[cfg(target_os = "macos")]
    std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| ConversionErrorDto {
            code: 1099,
            message: e.to_string(),
        })?;

    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .map_err(|e| ConversionErrorDto {
            code: 1099,
            message: e.to_string(),
        })?;

    Ok(())
}

/// Register or unregister the bundled viewer as the HKCU handler for a
/// given extension ("svg" or "xml") on Windows, or as the default MIME
/// handler via xdg-mime + a self-authored .desktop file on Linux.
#[tauri::command]
pub fn set_file_association(ext: String, enabled: bool) -> Result<(), ConversionErrorDto> {
    if !matches!(ext.as_str(), "svg" | "xml") {
        return Err(command_error(
            1000,
            "unsupported file association extension",
        ));
    }
    #[cfg(windows)]
    {
        windows_assoc::set_association(&ext, enabled).map_err(|e| ConversionErrorDto {
            code: 1098,
            message: e.to_string(),
        })
    }
    #[cfg(target_os = "macos")]
    {
        macos_assoc::set_association(&ext, enabled).map_err(|message| ConversionErrorDto {
            code: 1098,
            message,
        })
    }
    #[cfg(target_os = "linux")]
    {
        linux_assoc::set_association(&ext, enabled).map_err(|e| ConversionErrorDto {
            code: 1098,
            message: e.to_string(),
        })
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        let _ = (ext, enabled);
        Err(ConversionErrorDto {
            code: 1098,
            message: "File association toggling is only supported on Windows, macOS, and Linux."
                .into(),
        })
    }
}

/// Query whether the viewer is currently the default handler for a given
/// extension. Always false on unsupported platforms.
#[tauri::command]
pub fn get_file_association(ext: String) -> bool {
    if !matches!(ext.as_str(), "svg" | "xml") {
        return false;
    }
    #[cfg(windows)]
    {
        windows_assoc::is_associated(&ext)
    }
    #[cfg(target_os = "macos")]
    {
        macos_assoc::is_associated(&ext)
    }
    #[cfg(target_os = "linux")]
    {
        linux_assoc::is_associated(&ext)
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        let _ = ext;
        false
    }
}

#[cfg(windows)]
mod windows_assoc {
    use winreg::enums::*;
    use winreg::RegKey;

    const PROG_ID: &str = "WatermelonVectorFile";

    /// Registers/unregisters `WatermelonVectorFile` as the handler for
    /// `.{ext}` under HKCU\Software\Classes, and lists it in the
    /// OpenWithProgids key so it shows up in the Explorer "Open with" menu.
    /// HKCU (not HKCR) is used deliberately: it requires no admin rights and
    /// is the per-user override Explorer checks first.
    pub fn set_association(ext: &str, enabled: bool) -> std::io::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let classes = hkcu
            .open_subkey_with_flags("Software\\Classes", KEY_ALL_ACCESS)
            .or_else(|_| hkcu.create_subkey("Software\\Classes").map(|(k, _)| k))?;

        let exe_path = std::env::current_exe()?
            .parent()
            .map(|p| p.join("wvgc-viewer.exe"))
            .unwrap_or_default();
        let exe_str = exe_path.to_string_lossy().to_string();

        if enabled {
            // ProgID definition (shared across extensions, written once/idempotent).
            let (prog_key, _) = classes.create_subkey(PROG_ID)?;
            prog_key.set_value("", &"Watermelon Vector File")?;
            let (icon_key, _) = prog_key.create_subkey("DefaultIcon")?;
            icon_key.set_value("", &format!("{},0", exe_str))?;
            let (cmd_key, _) = prog_key.create_subkey("shell\\open\\command")?;
            cmd_key.set_value("", &format!("\"{}\" \"%1\"", exe_str))?;

            // Extension -> OpenWithProgids entry (what Explorer's picker reads).
            let ext_path = format!(".{}", ext);
            let (ext_key, _) = classes.create_subkey(&ext_path)?;
            let (progids_key, _) = ext_key.create_subkey("OpenWithProgids")?;
            progids_key.set_value(PROG_ID, &"")?;
        } else {
            let ext_path = format!(".{}", ext);
            if let Ok(ext_key) = classes.open_subkey_with_flags(&ext_path, KEY_ALL_ACCESS) {
                if let Ok(progids_key) =
                    ext_key.open_subkey_with_flags("OpenWithProgids", KEY_ALL_ACCESS)
                {
                    let _ = progids_key.delete_value(PROG_ID);
                }
            }
        }

        // Notify Explorer so the "Open with" list refreshes without a restart.
        unsafe {
            #[link(name = "shell32")]
            extern "system" {
                fn SHChangeNotify(event_id: i32, flags: u32, item1: *const u8, item2: *const u8);
            }
            const SHCNE_ASSOCCHANGED: i32 = 0x08000000;
            const SHCNF_IDLIST: u32 = 0;
            SHChangeNotify(
                SHCNE_ASSOCCHANGED,
                SHCNF_IDLIST,
                std::ptr::null(),
                std::ptr::null(),
            );
        }

        Ok(())
    }

    pub fn is_associated(ext: &str) -> bool {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let ext_path = format!("Software\\Classes\\.{}\\OpenWithProgids", ext);
        hkcu.open_subkey(&ext_path)
            .and_then(|k| k.get_raw_value(PROG_ID))
            .is_ok()
    }
}

/// macOS association, via direct FFI to LaunchServices — NOT tested on a
/// real Mac yet (no Mac access at time of writing). Written carefully
/// against Apple's documented C signatures rather than a third-party
/// binding crate (core-services documents itself as "woefully
/// incomplete"), but this module needs real-machine verification before
/// being trusted the way the Windows/Linux equivalents already are.
///
/// Uses the deprecated-but-still-functional LSSetDefaultRoleHandlerForContentType
/// (available since macOS 10.4) rather than the newer NSWorkspace method
/// (macOS 12+ only), since this project's default minimum system version
/// is 10.13 — the newer API would silently be unavailable on 10.13–11.x.
#[cfg(target_os = "macos")]
mod macos_assoc {
    use core_foundation::base::TCFType;
    use core_foundation::string::CFString;
    use std::os::raw::c_void;

    // Bundle identifier of the sidecar viewer, as declared in
    // viewer/src-tauri/tauri.conf.json's "identifier" field. Must match
    // exactly, or LaunchServices will report success while silently
    // pointing at nothing (this is the single most likely thing to verify
    // /fix on first real-Mac test if association doesn't take effect).
    const VIEWER_BUNDLE_ID: &str = "com.watermelon.viewer";

    type OSStatus = i32;
    type LSRolesMask = u32;
    const KLS_ROLES_ALL: LSRolesMask = 0xFFFF_FFFF;

    #[allow(non_snake_case)]
    #[link(name = "CoreServices", kind = "framework")]
    extern "C" {
        fn LSSetDefaultRoleHandlerForContentType(
            inContentType: *const c_void,
            inRole: LSRolesMask,
            inHandlerBundleID: *const c_void,
        ) -> OSStatus;

        fn LSCopyDefaultRoleHandlerForContentType(
            inContentType: *const c_void,
            inRole: LSRolesMask,
        ) -> *const c_void; // owned CFStringRef, or NULL

        fn UTTypeCreatePreferredIdentifierForTag(
            inTagClass: *const c_void,
            inTag: *const c_void,
            inConformingToUTI: *const c_void,
        ) -> *const c_void; // owned CFStringRef, or NULL
    }

    /// "public.filename-extension" is the documented literal string value
    /// of the kUTTagClassFilenameExtension constant — declared as a plain
    /// string here rather than an extern static, since CF constant symbols
    /// are trickier to bind correctly than functions and the literal value
    /// is stable/documented by Apple.
    fn filename_extension_uti(ext: &str) -> Option<CFString> {
        let tag_class = CFString::new("public.filename-extension");
        let tag = CFString::new(ext);
        unsafe {
            let uti_ref = UTTypeCreatePreferredIdentifierForTag(
                tag_class.as_concrete_TypeRef() as *const c_void,
                tag.as_concrete_TypeRef() as *const c_void,
                std::ptr::null(),
            );
            if uti_ref.is_null() {
                return None;
            }
            // wrap_under_create_rule takes ownership of the +1 reference
            // UTTypeCreatePreferredIdentifierForTag returns, so CFRelease
            // is handled by CFString's own Drop — do not release it again.
            Some(CFString::wrap_under_create_rule(
                uti_ref as core_foundation::string::CFStringRef,
            ))
        }
    }

    pub fn set_association(ext: &str, enabled: bool) -> Result<(), String> {
        let uti = filename_extension_uti(ext)
            .ok_or_else(|| format!("could not resolve a UTI for extension '{ext}'"))?;

        // There's no documented "unset" — disabling means pointing the
        // role handler back at Finder's own default view of things. In
        // practice, re-running LSSetDefaultRoleHandlerForContentType with
        // Finder's bundle ID is the standard approach other tools use;
        // since Finder is always present, this is a safe, universal
        // fallback rather than leaving the extension in an undefined state.
        let target_bundle_id = if enabled {
            VIEWER_BUNDLE_ID
        } else {
            "com.apple.finder"
        };
        let bundle_id_cf = CFString::new(target_bundle_id);

        let status = unsafe {
            LSSetDefaultRoleHandlerForContentType(
                uti.as_concrete_TypeRef() as *const c_void,
                KLS_ROLES_ALL,
                bundle_id_cf.as_concrete_TypeRef() as *const c_void,
            )
        };

        if status == 0 {
            Ok(())
        } else {
            Err(format!(
                "LSSetDefaultRoleHandlerForContentType failed with OSStatus {status}"
            ))
        }
    }

    pub fn is_associated(ext: &str) -> bool {
        let uti = match filename_extension_uti(ext) {
            Some(u) => u,
            None => return false,
        };
        unsafe {
            let handler_ref = LSCopyDefaultRoleHandlerForContentType(
                uti.as_concrete_TypeRef() as *const c_void,
                KLS_ROLES_ALL,
            );
            if handler_ref.is_null() {
                return false;
            }
            let handler = CFString::wrap_under_create_rule(
                handler_ref as core_foundation::string::CFStringRef,
            );
            handler.to_string() == VIEWER_BUNDLE_ID
        }
    }
}
/// meant to auto-generate a .desktop file + MIME registration at .deb
/// install time, but this is a known-unreliable path upstream (tauri-apps/
/// tauri#9803: "file association... doesn't work at all [on Linux]").
/// Rather than depend on that, this registers a REAL, self-authored
/// .desktop file via the standard freedesktop.org mechanism (xdg-mime +
/// update-desktop-database), the same way any other Linux app would,
/// independent of whether Tauri's own bundler-level generation worked.
#[cfg(target_os = "linux")]
mod linux_assoc {
    use std::fs;
    use std::io::{self, Write};
    use std::path::PathBuf;
    use std::process::Command;

    const DESKTOP_FILE_NAME: &str = "watermelon-vector-viewer.desktop";

    fn mime_type_for(ext: &str) -> &'static str {
        match ext {
            "svg" => "image/svg+xml",
            // VectorDrawable XML has no registered public MIME type of its
            // own; text/xml is the correct generic fallback and is what
            // Tauri's own extension-inference would also produce.
            _ => "text/xml",
        }
    }

    fn applications_dir() -> io::Result<PathBuf> {
        let home = std::env::var("HOME")
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))?;
        let dir = PathBuf::from(home).join(".local/share/applications");
        fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    fn desktop_file_path() -> io::Result<PathBuf> {
        Ok(applications_dir()?.join(DESKTOP_FILE_NAME))
    }

    /// Path to the sidecar viewer binary, staged alongside the main
    /// converter executable by Tauri's externalBin bundling (no extension
    /// on Linux, unlike the Windows .exe suffix).
    fn viewer_exe_path() -> io::Result<PathBuf> {
        let current = std::env::current_exe()?;
        let dir = current.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not resolve executable directory",
            )
        })?;
        Ok(dir.join("wvgc-viewer"))
    }

    /// Write (or ensure) the .desktop file exists, declaring every
    /// supported MIME type up front — regardless of which single extension
    /// is being toggled — since a .desktop file's MimeType list is global
    /// to the file, not per-extension. Individual extensions are then
    /// associated/disassociated purely via `xdg-mime default`/`query`,
    /// which is the real source of truth for "is this the current
    /// default," matching how Linux desktop environments actually decide.
    fn ensure_desktop_file() -> io::Result<()> {
        let exe = viewer_exe_path()?;
        let exe_str = exe.to_string_lossy();
        let contents = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Watermelon Vector Viewer\n\
             Comment=Preview SVG and Android VectorDrawable files\n\
             Exec=\"{exe}\" %f\n\
             Terminal=false\n\
             NoDisplay=true\n\
             MimeType=image/svg+xml;text/xml;application/xml;\n\
             Categories=Graphics;Viewer;\n",
            exe = exe_str,
        );
        let path = desktop_file_path()?;
        let mut file = fs::File::create(&path)?;
        file.write_all(contents.as_bytes())?;
        Ok(())
    }

    /// Refresh the desktop database so file managers pick up the new/
    /// changed .desktop file without requiring a logout or reboot. Best
    /// effort: some minimal distros lack this tool, so a missing binary is
    /// not treated as a hard failure — the .desktop file and xdg-mime
    /// state are still correct even if the cache refresh is skipped.
    fn refresh_desktop_database() {
        if let Ok(dir) = applications_dir() {
            let _ = Command::new("update-desktop-database").arg(&dir).status();
        }
    }

    pub fn set_association(ext: &str, enabled: bool) -> io::Result<()> {
        ensure_desktop_file()?;
        refresh_desktop_database();

        let mime = mime_type_for(ext);
        if enabled {
            let status = Command::new("xdg-mime")
                .args(["default", DESKTOP_FILE_NAME, mime])
                .status()?;
            if !status.success() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("xdg-mime default failed for {mime} (exit {status})"),
                ));
            }
        } else {
            // xdg-mime has no direct "unset default" — the standard
            // approach is to remove our entry from the user's
            // mimeapps.list, which is what xdg-mime's "default" command
            // itself writes to. We edit that file directly rather than
            // shelling out further, since there is no single-purpose CLI
            // command for removal.
            remove_default_from_mimeapps(mime)?;
        }
        Ok(())
    }

    /// Returns every mimeapps.list path xdg-mime is known to write to,
    /// depending on distro/xdg-utils version — some write to
    /// ~/.local/share/applications/mimeapps.list, others to
    /// ~/.config/mimeapps.list. Checking only one has been observed to
    /// silently miss the actual entry on some systems, so both are
    /// checked/edited for the removal path.
    fn mimeapps_list_paths() -> io::Result<Vec<PathBuf>> {
        let home = std::env::var("HOME")
            .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HOME not set"))?;
        let home = PathBuf::from(home);
        Ok(vec![
            home.join(".local/share/applications/mimeapps.list"),
            home.join(".config/mimeapps.list"),
        ])
    }

    fn remove_default_from_mimeapps(mime: &str) -> io::Result<()> {
        for path in mimeapps_list_paths()? {
            if !path.exists() {
                continue;
            }
            let content = fs::read_to_string(&path)?;
            let filtered: String = content
                .lines()
                .filter(|line| {
                    let trimmed = line.trim();
                    !(trimmed.starts_with(&format!("{mime}="))
                        && trimmed.contains(DESKTOP_FILE_NAME))
                })
                .collect::<Vec<_>>()
                .join("\n");
            fs::write(&path, filtered)?;
        }
        Ok(())
    }

    /// Reads the user's actual current default handler via `xdg-mime
    /// query default`, rather than just checking whether our .desktop file
    /// exists — the existence of the file doesn't mean it's the ACTIVE
    /// default, matching the semantic of the Windows is_associated check.
    pub fn is_associated(ext: &str) -> bool {
        let mime = mime_type_for(ext);
        let output = Command::new("xdg-mime")
            .args(["query", "default", mime])
            .output();
        match output {
            Ok(o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout).trim() == DESKTOP_FILE_NAME
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod security_tests {
    use super::{safe_file_name, validate_external_url};

    #[test]
    fn output_name_must_not_contain_a_path() {
        assert!(safe_file_name("result.svg").is_ok());
        assert!(safe_file_name("../result.svg").is_err());
        assert!(safe_file_name("nested/result.svg").is_err());
        assert!(safe_file_name("").is_err());
    }

    #[test]
    fn external_url_is_strictly_allowlisted() {
        assert!(
            validate_external_url("https://github.com/muzaff-beep/Watermelon-VectorConverter")
                .is_ok()
        );
        assert!(validate_external_url("https://example.invalid").is_err());
        assert!(validate_external_url(
            "https://github.com/muzaff-beep/Watermelon-VectorConverter?x=1"
        )
        .is_err());
    }
}
