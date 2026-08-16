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
use iced::{window, Element, Length, Subscription, Task};
use std::path::{Path, PathBuf};
use std::time::Duration;

use svg_converter_core::animation::{detect_animation, AnimationKind, FileKind};

#[derive(Debug, Clone)]
#[expect(
    dead_code,
    reason = "file-arrival routing is retained for the single-instance integration"
)]
pub enum Message {
    OpenFilePicked(Option<PathBuf>),
    OpenFileRequested,
    FileLoaded(Result<LoadedFile, String>),
    /// A file was dropped onto the window, or another already-running
    /// instance was told (via single_instance) to open a new file.
    FileArrived(PathBuf),
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

#[expect(
    dead_code,
    reason = "empty viewer state is retained for startup and reset behavior"
)]
enum ViewState {
    Empty,
    Loading,
    Static {
        name: String,
        handle: image::Handle,
    },
    Avd {
        name: String,
        handles: Vec<image::Handle>,
        frame_durations_ms: Vec<u32>,
        current: usize,
        elapsed_in_frame: Duration,
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
            Message::FileArrived(path) => {
                self.state = ViewState::Loading;
                return Task::perform(load_file(path), Message::FileLoaded);
            }
            Message::FileLoaded(Ok(loaded)) => {
                self.state = match loaded {
                    LoadedFile::Static { name, png } => ViewState::Static {
                        name,
                        handle: image::Handle::from_bytes(png),
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
            Message::IcedEvent(_) => {}
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = match &self.state {
            ViewState::Empty => center(
                column![
                    text("Drop an SVG or VectorDrawable XML file here").size(15),
                    button("Open…").on_press(Message::OpenFileRequested),
                ]
                .spacing(12)
                .align_x(iced::Alignment::Center),
            )
            .into(),
            ViewState::Loading => center(text("Loading…")).into(),
            ViewState::Static { handle, .. } => center(
                image(handle.clone()).content_fit(iced::ContentFit::Contain),
            )
            .into(),
            ViewState::Avd { handles, current, .. } => {
                let handle = handles.get(*current).cloned().unwrap_or_else(|| handles[0].clone());
                center(image(handle).content_fit(iced::ContentFit::Contain)).into()
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

        column![
            row![
                text(name).size(13),
                space::horizontal(),
                button("Open…").on_press(Message::OpenFileRequested),
            ]
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
