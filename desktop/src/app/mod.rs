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
    .theme(|_: &_| iced::Theme::Dark)
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
    .theme(|_: &_| iced::Theme::Dark)
    .run()
}

/// What kind of animated content a file turned out to be, mirrored here
/// from svg_converter_core::animation::AnimationKind so the UI layer
/// doesn't need to depend on the core crate's internal module path in
/// every match arm — kept in exact 1:1 correspondence with the real enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimKind {
    None,
    Avd,
    SvgSmil,
    SvgCss,
}

impl From<svg_converter_core::animation::AnimationKind> for AnimKind {
    fn from(k: svg_converter_core::animation::AnimationKind) -> Self {
        use svg_converter_core::animation::AnimationKind as K;
        match k {
            K::None => AnimKind::None,
            K::Avd => AnimKind::Avd,
            K::SvgSmil => AnimKind::SvgSmil,
            K::SvgCss => AnimKind::SvgCss,
        }
    }
}
