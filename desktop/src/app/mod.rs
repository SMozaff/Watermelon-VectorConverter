// Watermelon Vector Converter — Desktop
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

mod converter;
mod viewer;

use std::path::PathBuf;

pub fn run_converter() -> iced::Result {
    iced::application(
        converter::Converter::new,
        converter::Converter::update,
        converter::Converter::view,
    )
    .title("Watermelon Vector Converter")
    .subscription(converter::Converter::subscription)
    .theme(iced::Theme::Dark)
    .window_size((460.0, 640.0))
    .centered()
    .resizable(false)
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
    .theme(iced::Theme::Dark)
    .run()
}
