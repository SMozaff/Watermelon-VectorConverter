// Watermelon Vector Converter — Desktop Viewer (native iced, no WebView)
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.
//
// Replaces the former Tauri+WebView2 viewer entirely. Static SVG and
// VectorDrawable XML render via the existing resvg-based Rust pipeline,
// shown as a plain image widget. Animated VectorDrawables play back their
// pre-rendered frame sequence on a timer. SMIL/CSS-animated SVGs are
// explicitly NOT supported here — that class of animation genuinely needs
// a browser engine (CSS timing functions, keyframe interpolation), and the
// whole point of this rewrite is removing the WebView2 dependency that was
// causing blank-window failures. Detected and reported as unsupported
// rather than attempted.

use iced::widget::{button, center, column, container, image, row, space, text};
use iced::{mouse, window, Element, Length, Subscription, Task};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Zoom bounds and per-notch step for mouse-wheel zoom. A single wheel
/// "click" is almost always reported as ScrollDelta::Lines{y: ±1.0} by the
/// underlying winit backend, so a fixed additive step per whole line (rather
/// than a multiplicative one) keeps a single notch feeling like a single,
/// predictable increment regardless of the current zoom level.
const ZOOM_MIN: f32 = 0.25;
const ZOOM_MAX: f32 = 8.0;
const ZOOM_STEP_PER_LINE: f32 = 0.1;
/// Trackpads and some mice report ScrollDelta::Pixels instead of Lines; this
/// converts a pixel delta into an equivalent step so both input types feel
/// similarly paced (rough parity with a ~120-units-per-notch wheel).
const ZOOM_STEP_PER_PIXEL: f32 = ZOOM_STEP_PER_LINE / 120.0;

use svg_converter_core::animation::{detect_animation, AnimationKind, FileKind};

#[derive(Debug, Clone)]
pub enum Message {
    OpenFilePicked(Option<PathBuf>),
    OpenFileRequested,
    FileLoaded(Result<LoadedFile, String>),
    Tick,
    IcedEvent(iced::Event),
}

/// What loading a file actually produced — mirrors the three real C-5
/// outcomes (static, AVD, unsupported-animated) without the fourth,
/// WebView-only path this rewrite deliberately drops.
#[derive(Debug, Clone)]
pub enum LoadedFile {
    Static {
        name: String,
        png: Vec<u8>,
    },
    Avd {
        name: String,
        frames: Vec<Vec<u8>>,
        frame_durations_ms: Vec<u32>,
    },
    UnsupportedAnimatedSvg {
        name: String,
    },
}

enum ViewState {
    Loading,
    Static {
        name: String,
        handle: image::Handle,
        zoom: f32,
    },
    Avd {
        name: String,
        handles: Vec<image::Handle>,
        frame_durations_ms: Vec<u32>,
        current: usize,
        elapsed_in_frame: Duration,
        zoom: f32,
    },
    Unsupported {
        name: String,
    },
    Error {
        message: String,
    },
}

pub struct Viewer {
    state: ViewState,
}

impl Viewer {
    pub fn new(initial_path: PathBuf) -> (Self, Task<Message>) {
        let viewer = Viewer {
            state: ViewState::Loading,
        };
        (
            viewer,
            Task::perform(load_file(initial_path), Message::FileLoaded),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenFileRequested => {
                return Task::perform(pick_file(), Message::OpenFilePicked);
            }
            Message::OpenFilePicked(Some(path)) => {
                self.state = ViewState::Loading;
                return Task::perform(load_file(path), Message::FileLoaded);
            }
            Message::OpenFilePicked(None) => {}
            Message::FileLoaded(Ok(loaded)) => {
                self.state = match loaded {
                    // A freshly opened file always starts at 1.0 (fit-to-
                    // window via ContentFit::Contain) rather than carrying
                    // over the previous file's zoom level.
                    LoadedFile::Static { name, png } => ViewState::Static {
                        name,
                        handle: image::Handle::from_bytes(png),
                        zoom: 1.0,
                    },
                    LoadedFile::Avd {
                        name,
                        frames,
                        frame_durations_ms,
                    } => {
                        let handles = frames.into_iter().map(image::Handle::from_bytes).collect();
                        ViewState::Avd {
                            name,
                            handles,
                            frame_durations_ms,
                            current: 0,
                            elapsed_in_frame: Duration::ZERO,
                            zoom: 1.0,
                        }
                    }
                    LoadedFile::UnsupportedAnimatedSvg { name } => ViewState::Unsupported { name },
                };
            }
            Message::FileLoaded(Err(message)) => {
                self.state = ViewState::Error { message };
            }
            Message::Tick => {
                // Advance AVD playback by one tick (see subscription's
                // interval). Frame-accurate timing isn't critical for a
                // preview tool, so a fixed small tick advancing elapsed
                // time and rolling over to the next frame when its own
                // duration is exceeded is simple and good enough — this
                // mirrors the same "frame-swap on a timer" approach the
                // Tauri/Svelte version used, just driven by iced's own
                // subscription instead of setTimeout.
                if let ViewState::Avd {
                    handles,
                    frame_durations_ms,
                    current,
                    elapsed_in_frame,
                    ..
                } = &mut self.state
                {
                    const TICK: Duration = Duration::from_millis(16);
                    *elapsed_in_frame += TICK;
                    let this_frame_duration = frame_durations_ms
                        .get(*current)
                        .copied()
                        .map(|ms| Duration::from_millis(ms as u64))
                        .unwrap_or(Duration::from_millis(33));
                    if *elapsed_in_frame >= this_frame_duration {
                        *elapsed_in_frame = Duration::ZERO;
                        *current = (*current + 1) % handles.len().max(1);
                    }
                }
            }
            Message::IcedEvent(iced::Event::Window(window::Event::FileDropped(path))) => {
                self.state = ViewState::Loading;
                return Task::perform(load_file(path), Message::FileLoaded);
            }
            Message::IcedEvent(iced::Event::Mouse(mouse::Event::WheelScrolled { delta })) => {
                // Only Static/Avd actually show an image to zoom; any other
                // state (Loading/Unsupported/Error) has no image and simply
                // ignores wheel input rather than erroring.
                let zoom_ref = match &mut self.state {
                    ViewState::Static { zoom, .. } => Some(zoom),
                    ViewState::Avd { zoom, .. } => Some(zoom),
                    _ => None,
                };
                if let Some(zoom) = zoom_ref {
                    let step = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => y * ZOOM_STEP_PER_LINE,
                        mouse::ScrollDelta::Pixels { y, .. } => y * ZOOM_STEP_PER_PIXEL,
                    };
                    *zoom = (*zoom + step).clamp(ZOOM_MIN, ZOOM_MAX);
                }
            }
            Message::IcedEvent(_) => {}
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = match &self.state {
            ViewState::Loading => center(text("Loading…")).into(),
            ViewState::Static { handle, zoom, .. } => center(
                image(handle.clone())
                    .content_fit(iced::ContentFit::Contain)
                    .scale(*zoom),
            )
            .into(),
            ViewState::Avd { handles, current, zoom, .. } => {
                let handle = handles.get(*current).cloned().unwrap_or_else(|| handles[0].clone());
                center(
                    image(handle)
                        .content_fit(iced::ContentFit::Contain)
                        .scale(*zoom),
                )
                .into()
            }
            ViewState::Unsupported { .. } => center(
                column![
                    text("⚠ Animated SVG preview isn't supported").size(15),
                    text("This file uses SMIL or CSS animation, which needs a browser engine to play correctly. Static preview isn't available for this file.")
                        .size(12),
                ]
                .spacing(8)
                .align_x(iced::Alignment::Center)
                .max_width(420),
            )
            .into(),
            ViewState::Error { message } => center(
                column![
                    text("⚠ Couldn't load this file").size(15),
                    text(message.clone()).size(12),
                ]
                .spacing(8)
                .align_x(iced::Alignment::Center)
                .max_width(420),
            )
            .into(),
        };

        let name = match &self.state {
            ViewState::Static { name, .. }
            | ViewState::Avd { name, .. }
            | ViewState::Unsupported { name, .. } => name.as_str(),
            _ => "",
        };
        let zoom_label = match &self.state {
            ViewState::Static { zoom, .. } | ViewState::Avd { zoom, .. } => {
                Some(format!("{:.0}%", zoom * 100.0))
            }
            _ => None,
        };

        column![
            row![
                text(name).size(13),
                space::horizontal(),
                text(zoom_label.unwrap_or_default()).size(12),
                button("Open…").on_press(Message::OpenFileRequested),
            ]
            .spacing(10)
            .padding(10)
            .align_y(iced::Alignment::Center),
            container(content).width(Length::Fill).height(Length::Fill),
        ]
        .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subs = vec![iced::event::listen().map(Message::IcedEvent)];
        if matches!(self.state, ViewState::Avd { .. }) {
            subs.push(iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick));
        }
        Subscription::batch(subs)
    }
}

async fn pick_file() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("SVG / VectorDrawable", &["svg", "xml"])
        .pick_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}

/// Reads the file, detects SVG vs. VectorDrawable by root tag (not
/// extension), detects animation (Contract C-5.1), and produces whichever
/// of the three supported outcomes applies. Runs on iced's executor via
/// Task::perform — the conversion calls themselves are synchronous/CPU-
/// bound, but wrapping them in an async fn lets the UI thread stay
/// responsive while a large file's conversion runs.
async fn load_file(path: PathBuf) -> Result<LoadedFile, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let name = file_display_name(&path);
    let content = String::from_utf8_lossy(&bytes).into_owned();

    let is_vector_drawable = content
        .trim_start()
        .lines()
        .find(|l| !l.trim().is_empty() && !l.trim_start().starts_with("<?xml"))
        .map(|l| {
            let l = l.trim_start();
            l.starts_with("<vector") || l.starts_with("<animated-vector")
        })
        .unwrap_or(false);

    let file_kind = if is_vector_drawable {
        FileKind::Avd
    } else {
        FileKind::Svg
    };
    let anim_kind = detect_animation(&bytes, file_kind);

    match anim_kind {
        AnimationKind::Avd => {
            let frames = svg_converter_core::render_avd_frames(&bytes, 30, 90, 512)
                .map_err(|e| e.to_string())?;
            Ok(LoadedFile::Avd {
                name,
                frames: frames.frames,
                frame_durations_ms: frames.frame_durations_ms,
            })
        }
        AnimationKind::SvgSmil | AnimationKind::SvgCss => {
            Ok(LoadedFile::UnsupportedAnimatedSvg { name })
        }
        AnimationKind::None => {
            let png = if is_vector_drawable {
                svg_converter_core::image_export::render_vd_preview(&content, 1024)
                    .map_err(|e| e.to_string())?
            } else {
                svg_converter_core::image_export::render_svg_preview(&bytes, 1024)
                    .map_err(|e| e.to_string())?
            };
            Ok(LoadedFile::Static { name, png })
        }
    }
}

fn file_display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}
