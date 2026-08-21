// Watermelon Vector Converter — Desktop
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
// Proprietary and source-available. Reuse prohibited without written permission.
// See LICENSE for terms.
//
// Windows file association (the "install alongside the main app, so double-
// clicking an .svg/.xml opens the viewer" behavior). macOS (LaunchServices)
// and Linux (xdg-mime) equivalents are out of scope for this module — this
// is a Windows-only file (see the #[cfg(windows)] gate on its declaration
// in main.rs, and the matching [target.'cfg(windows)'.dependencies] entry
// for winreg in Cargo.toml) — each OS's association mechanism is
// independent enough that doing them one at a time is easier to review and
// test correctly than bundling all three in one pass.
//
// Registration strategy, and why it's *not* a blind HKCR\.svg / HKCR\.xml
// takeover:
//   - .svg has no strong existing claim on a typical Windows box (no
//     built-in shell handler), so it's safe to also set it as the
//     extension's default handler.
//   - .xml is a shared, generic extension used by many unrelated file
//     types (RSS feeds, config files, Office/VS project files, etc.).
//     Overwriting HKCR\.xml's default ProgID would silently change what
//     opens for every other app's XML files too — a real regression, not
//     a fix. Instead, Watermelon registers a ProgID and adds itself to
//     .xml's "Open with" list (OpenWithProgids) WITHOUT touching .xml's
//     existing default, so the user keeps whichever app currently owns
//     plain XML by default and can explicitly choose Watermelon per file
//     or set it as default themselves via Windows' own Settings UI.
//
// Everything here writes under HKEY_CURRENT_USER (not HKEY_LOCAL_MACHINE /
// HKEY_CLASSES_ROOT), which needs no admin elevation and only affects the
// installing user — matching the installer's existing per-user Start Menu
// shortcut / PATH entries rather than requiring elevated MSI custom actions.

use std::io;
use std::path::Path;
use winreg::enums::*;
use winreg::RegKey;

/// ProgID Watermelon registers itself under. Namespaced with the app name
/// to avoid colliding with any other app's ProgID.
const SVG_PROG_ID: &str = "Watermelon.SvgViewer.1";
const VD_PROG_ID: &str = "Watermelon.VectorDrawableViewer.1";

/// Resolves the currently running executable's path and its installed
/// icon files (expected alongside it — see the WiX installer, which places
/// watermelon_svg.ico / watermelon_xml.ico in the same `bin` directory as
/// wvgc-desktop.exe), then registers both associations.
///
/// This is the entry point `main.rs` calls on every ordinary (non-viewer)
/// startup.
pub fn register_current_exe() -> io::Result<()> {
    let exe_path = std::env::current_exe()?;
    let install_dir = exe_path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "exe has no parent directory"))?;
    let svg_icon = install_dir.join("watermelon_svg.ico");
    let xml_icon = install_dir.join("watermelon_xml.ico");
    register(&exe_path, &svg_icon, &xml_icon)
}

/// Registers Watermelon as a viewer for .svg (as the default handler) and
/// .xml (added to "Open with", default left untouched — see module doc).
/// `exe_path` is the absolute path to wvgc-desktop.exe; `svg_icon`/`xml_icon`
/// are absolute paths to the per-type .ico files installed alongside it.
///
/// Idempotent: safe to call on every launch (or from the installer) — it
/// only ever writes these specific keys/values, never deletes or reads
/// pre-existing associations for other ProgIDs.
pub fn register(exe_path: &Path, svg_icon: &Path, xml_icon: &Path) -> io::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let classes = hkcu.create_subkey("Software\\Classes")?.0;

    register_prog_id(&classes, SVG_PROG_ID, "Watermelon SVG Viewer", exe_path, svg_icon)?;
    register_prog_id(
        &classes,
        VD_PROG_ID,
        "Watermelon VectorDrawable Viewer",
        exe_path,
        xml_icon,
    )?;

    // .svg: no strong existing owner on a typical system, so also claim
    // the extension's own default handler, not just the ProgID.
    set_extension_default(&classes, ".svg", SVG_PROG_ID)?;
    add_open_with(&classes, ".svg", SVG_PROG_ID)?;

    // .xml: shared extension — add to "Open with" only, deliberately not
    // touching whatever already owns .xml's default (see module doc).
    add_open_with(&classes, ".xml", VD_PROG_ID)?;

    notify_shell_associations_changed();
    Ok(())
}

/// Creates `Software\Classes\<prog_id>` with a display name, a `DefaultIcon`
/// pointing at the given .ico, and a `shell\open\command` that launches
/// wvgc-desktop.exe with the clicked file's path as its one argument —
/// exactly the argv shape main.rs already branches viewer-mode on.
fn register_prog_id(
    classes: &RegKey,
    prog_id: &str,
    display_name: &str,
    exe_path: &Path,
    icon_path: &Path,
) -> io::Result<()> {
    let (key, _disposition) = classes.create_subkey(prog_id)?;
    key.set_value("", &display_name)?;

    let (icon_key, _) = key.create_subkey("DefaultIcon")?;
    // Icon path with no index suffix defaults to the .ico's first image;
    // these .ico files each embed a single logical icon at multiple
    // resolutions (see icons/watermelon_svg.ico, icons/watermelon_xml.ico),
    // so there's no second icon to disambiguate with a ",1" index.
    icon_key.set_value("", &icon_path.display().to_string())?;

    let (command_key, _) = key.create_subkey("shell\\open\\command")?;
    // Quoted twice deliberately: the outer quotes are the registry value's
    // own escaping for a path containing spaces (e.g. "Program Files"),
    // the inner %1 is the shell's placeholder for the clicked file's path,
    // which Explorer substitutes already-quoted if the path has spaces.
    let command = format!("\"{}\" \"%1\"", exe_path.display());
    command_key.set_value("", &command)?;

    Ok(())
}

/// Sets `HKCU\Software\Classes\<ext>`'s unnamed (default) value to the given
/// ProgID — this makes double-click open with that ProgID's command.
fn set_extension_default(classes: &RegKey, ext: &str, prog_id: &str) -> io::Result<()> {
    let (key, _) = classes.create_subkey(ext)?;
    key.set_value("", &prog_id)?;
    Ok(())
}

/// Adds `prog_id` as one of the extension's "Open with" choices, via the
/// `OpenWithProgids` subkey. This is purely additive — Explorer's "Open
/// with" list gains Watermelon as an option, but nothing about the
/// extension's *default* handler changes if one is already set (which is
/// exactly why this call, unlike `set_extension_default`, is used for
/// .xml — see module doc). The value's own content doesn't matter, only
/// its presence — an empty REG_NONE (zero-length binary) value is the
/// documented convention Windows itself uses for these entries.
fn add_open_with(classes: &RegKey, ext: &str, prog_id: &str) -> io::Result<()> {
    let (ext_key, _) = classes.create_subkey(ext)?;
    let (open_with_key, _) = ext_key.create_subkey("OpenWithProgids")?;
    open_with_key.set_raw_value(
        prog_id,
        &winreg::RegValue {
            bytes: Vec::new(),
            vtype: REG_NONE,
        },
    )?;
    Ok(())
}

/// Tells Explorer to pick up the association changes without requiring a
/// logoff/logon. Uses SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, ...)
/// via a direct extern "C" FFI declaration rather than pulling in the
/// windows-sys/winapi shell32 bindings crate just for this one call — the
/// same reasoning Cargo.toml's comment gives for the macOS LaunchServices
/// call in this same module.
fn notify_shell_associations_changed() {
    #[link(name = "shell32")]
    extern "system" {
        fn SHChangeNotify(
            event_id: i32,
            flags: u32,
            item1: *const std::ffi::c_void,
            item2: *const std::ffi::c_void,
        );
    }
    const SHCNE_ASSOCCHANGED: i32 = 0x0800_0000;
    const SHCNF_IDLIST: u32 = 0x0000;
    unsafe {
        SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, std::ptr::null(), std::ptr::null());
    }
}
