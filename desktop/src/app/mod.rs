// Watermelon Vector Converter — Desktop
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

mod converter;
mod viewer;

use std::path::PathBuf;

// The window/taskbar icon shown while the app is RUNNING. This is separate
// from (and previously missing alongside) build.rs's winres call, which
// only embeds an icon into wvgc-desktop.exe's own file resources — that
// covers what Explorer shows for the .exe before it's launched, not what
// the OS shows for the live window (taskbar, alt-tab, title bar) once it
// is. Without this, `window::Settings::icon` stays at its default of
// `None` and the running window gets a generic/blank icon regardless of
// platform. Same source art as the .exe icon (icons/watermelon.ico),
// decoded from a PNG since `window::icon::from_file_data` needs raster
// pixels, not an .ico container.
const WINDOW_ICON_PNG: &[u8] = include_bytes!("../../../icons/png/256.png");

fn window_icon() -> Option<iced::window::Icon> {
    iced::window::icon::from_file_data(WINDOW_ICON_PNG, None).ok()
}

pub fn run_converter() -> iced::Result {
    iced::application(
        converter::Converter::new,
        converter::Converter::update,
        converter::Converter::view,
    )
    .title("Watermelon Vector Converter")
    .subscription(converter::Converter::subscription)
    .theme(converter::Converter::theme)
    .window(iced::window::Settings {
        icon: window_icon(),
        // Floor for the responsive two-pane/stacked layout switch in
        // converter.rs (see Converter::window_width) — below this, neither
        // layout has room for its controls to stay usable, so the window
        // itself refuses to shrink further rather than letting content get
        // clipped or inaccessible.
        min_size: Some(iced::Size::new(560.0, 480.0)),
        ..Default::default()
    })
    .window_size((1040.0, 760.0))
    .centered()
    .resizable(true)
    .run()
}

pub fn run_viewer(path: PathBuf) -> iced::Result {
    // application()'s boot function takes no arguments, so the launch path
    // is captured via a move closure rather than passed as a boot fn
    // argument directly — see viewer::Viewer::new's own signature.
    iced::application(
        move || viewer::Viewer::new(path.clone()),
        viewer::Viewer::update,
        viewer::Viewer::view,
    )
    .title("Watermelon Vector Viewer")
    .subscription(viewer::Viewer::subscription)
    .theme(|_: &viewer::Viewer| -> Option<iced::Theme> { None })
    .window(iced::window::Settings {
        icon: window_icon(),
        ..Default::default()
    })
    .run()
}
