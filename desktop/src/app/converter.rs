// Watermelon Vector Converter — Desktop conversion workspace.
// Copyright (c) 2026 Soheil Mozaffari. All rights reserved.

use iced::widget::{
    button, canvas, center, column, container, image, progress_bar, row, scrollable, space, text,
};
use iced::{
    clipboard, keyboard, mouse, window, Background, Border, Color, ContentFit, Element, Length,
    Point, Rectangle, Renderer, Subscription, Task, Theme,
};
use std::path::{Path, PathBuf};
use std::process::Command;

const ABOUT_LOGO: &[u8] = include_bytes!("../../assets/watermelon_iphone_logo.png");
const IFEM_DOCTRINE_URL: &str = "https://IFEM-doctrine.github.io/";
const PERSONAL_WEBSITE_URL: &str = "https://SMozaff.github.io/";
const IFEM_DOCTRINE_LABEL: &str = "IFEM-doctrine.github.io";
const PERSONAL_WEBSITE_LABEL: &str = "SMozaff.github.io";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Appearance {
    Light,
    Dark,
}

impl Appearance {
    fn toggled(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Light => "Dark mode",
            Self::Dark => "Light mode",
        }
    }

    fn theme(self) -> iced::Theme {
        match self {
            Self::Light => iced::Theme::Light,
            Self::Dark => iced::Theme::Dark,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Palette {
    background: Color,
    surface: Color,
    surface_muted: Color,
    primary: Color,
    primary_glow: Color,
    on_background: Color,
    on_surface: Color,
    muted: Color,
    border: Color,
    error: Color,
    watermelon_red: Color,
}

const DARK_PALETTE: Palette = Palette {
    background: Color::from_rgb8(5, 12, 9),
    surface: Color::from_rgb8(12, 29, 20),
    surface_muted: Color::from_rgb8(18, 43, 30),
    primary: Color::from_rgb8(100, 211, 153),
    primary_glow: Color::from_rgb8(138, 235, 78),
    on_background: Color::from_rgb8(247, 250, 248),
    on_surface: Color::from_rgb8(247, 250, 248),
    muted: Color::from_rgb8(194, 205, 198),
    border: Color::from_rgb8(53, 96, 72),
    error: Color::from_rgb8(255, 179, 177),
    watermelon_red: Color::from_rgb8(255, 119, 126),
};

const LIGHT_PALETTE: Palette = Palette {
    background: Color::from_rgb8(247, 250, 248),
    surface: Color::from_rgb8(255, 255, 255),
    surface_muted: Color::from_rgb8(232, 240, 235),
    primary: Color::from_rgb8(20, 122, 112),
    primary_glow: Color::from_rgb8(34, 144, 114),
    on_background: Color::from_rgb8(23, 34, 29),
    on_surface: Color::from_rgb8(23, 34, 29),
    muted: Color::from_rgb8(82, 97, 91),
    border: Color::from_rgb8(184, 207, 194),
    error: Color::from_rgb8(154, 27, 47),
    watermelon_red: Color::from_rgb8(198, 40, 57),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Converter,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    SvgToVectorDrawable,
    VectorDrawableToSvg,
}

impl Direction {
    fn label(self) -> &'static str {
        match self {
            Self::SvgToVectorDrawable => "SVG → XML",
            Self::VectorDrawableToSvg => "XML → SVG",
        }
    }

    fn input_hint(self) -> &'static str {
        match self {
            Self::SvgToVectorDrawable => "Drop an SVG file to create an Android VectorDrawable.",
            Self::VectorDrawableToSvg => "Drop a VectorDrawable XML file to create an SVG.",
        }
    }

    fn source_label(self) -> &'static str {
        match self {
            Self::SvgToVectorDrawable => "SOURCE SVG",
            Self::VectorDrawableToSvg => "SOURCE XML",
        }
    }

    fn output_label(self) -> &'static str {
        match self {
            Self::SvgToVectorDrawable => "GENERATED XML",
            Self::VectorDrawableToSvg => "GENERATED SVG",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::SvgToVectorDrawable => "xml",
            Self::VectorDrawableToSvg => "svg",
        }
    }

    fn source_format(self) -> VectorFormat {
        match self {
            Self::SvgToVectorDrawable => VectorFormat::Svg,
            Self::VectorDrawableToSvg => VectorFormat::VdXml,
        }
    }

    fn output_format(self) -> VectorFormat {
        match self {
            Self::SvgToVectorDrawable => VectorFormat::VdXml,
            Self::VectorDrawableToSvg => VectorFormat::Svg,
        }
    }
}

/// The shared Source -> Result format-badge vocabulary. Same three variants
/// as the Android counterpart (see Android's `VectorFormat.kt`) — this is
/// the "same conceptual grammar on both platforms" the redesign prompt
/// asks for; the two enums are independent Rust/Kotlin types (there's no
/// shared code between the platforms) but are kept deliberately in lock-
/// step: same variant names, same short labels, same semantic-token
/// mapping rationale documented in `format_badge()`'s own doc comment.
/// `BatchZip` is not wired into any UI yet — it exists now so a future
/// batch/zip badge follows the same component without a breaking change,
/// per the prompt's "optional future support" instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VectorFormat {
    Svg,
    VdXml,
    #[allow(dead_code)]
    BatchZip,
}

impl VectorFormat {
    fn short_label(self) -> &'static str {
        match self {
            Self::Svg => "SVG",
            Self::VdXml => "XML",
            Self::BatchZip => "ZIP",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputKind {
    Svg,
    VectorDrawable,
}

impl InputKind {
    fn label(self) -> &'static str {
        match self {
            Self::Svg => "SVG",
            Self::VectorDrawable => "VectorDrawable XML",
        }
    }

    fn natural_direction(self) -> Direction {
        match self {
            Self::Svg => Direction::SvgToVectorDrawable,
            Self::VectorDrawable => Direction::VectorDrawableToSvg,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct InputFile {
    name: String,
    bytes: Vec<u8>,
    kind: InputKind,
    source_preview: Option<image::Handle>,
}

#[derive(Debug)]
enum ConversionState {
    Empty,
    Ready(InputFile),
    Working {
        name: String,
    },
    Done {
        name: String,
        direction: Direction,
        source_preview: Option<image::Handle>,
        output_preview: Option<image::Handle>,
        output_text: String,
        output_analysis: Option<svg_converter_core::analysis::VectorAnalysis>,
    },
    Error {
        name: Option<String>,
        message: String,
    },
}

#[derive(Debug, Clone)]
pub(super) struct ConvertedOutput {
    name: String,
    direction: Direction,
    source_preview: Option<Vec<u8>>,
    output_preview: Option<Vec<u8>>,
    output_text: String,
    /// Structural analysis of the OUTPUT file, straight from
    /// `svg-converter-core::analysis` — the same authoritative walk the
    /// Android file-manager properties panel already uses. Never a
    /// fidelity score or a warning list: conversion here is strict
    /// all-or-nothing (an unsupported construct is a hard
    /// `ConversionError`, not a silent degrade — see svg_parser.rs's own
    /// doc comment), so there is nothing genuine to rate or warn about
    /// once conversion has actually succeeded. `None` only if the
    /// analysis pass itself failed to re-parse the output we just
    /// generated (should not happen in practice, but analysis is a
    /// best-effort summary, not something that should fail the whole
    /// conversion if it stumbles).
    output_analysis: Option<svg_converter_core::analysis::VectorAnalysis>,
}

pub struct Converter {
    screen: Screen,
    appearance: Appearance,
    direction: Direction,
    state: ConversionState,
    last_input: Option<InputFile>,
    notice: Option<String>,
    /// Current window width in logical pixels, used to switch between the
    /// side-by-side and stacked source/result layouts. Initialized to match
    /// mod.rs's initial `.window_size((1040.0, 760.0))` so the very first
    /// frame already renders at the right layout, before any
    /// `window::resize_events()` has fired; kept in sync afterwards via
    /// `Message::WindowResized`.
    window_width: f32,
    /// Backing state for the code viewer's `text_editor`. Rebuilt fresh
    /// whenever a NEW conversion completes (see ConversionFinished) rather
    /// than reused, since a fresh `Content::with_text` also resets cursor/
    /// scroll/selection position, which is the right behavior for "this is
    /// now a different file's output" rather than carrying over the
    /// previous file's cursor position into unrelated text. `text_editor`
    /// is used (not the plain `text` widget) specifically because plain
    /// `text` in iced has no mouse-drag selection or Select All at all —
    /// verified: iced_core/font.rs's Text widget API has no selection
    /// methods, and a live iced discourse thread explicitly requests this
    /// as a still-open feature for rendered text. `text_editor` is kept
    /// read-only by filtering `Action::Edit` out in `update()` rather than
    /// omitting `.on_action()` entirely — the latter disables ALL
    /// interaction (including the selection/scroll/click this viewer
    /// needs), per the widget's own documented behavior.
    code_content: iced::widget::text_editor::Content,
    /// Drives the indeterminate progress animation in `working_view` — a
    /// sweeping bar rather than a fixed value, since the real conversion
    /// (a single opaque `Task::perform(convert_input(...), ...)` with no
    /// partial-progress channel back to the UI) has no genuine percentage
    /// to report. Cycles 0.0->1.0 continuously while
    /// `ConversionState::Working` is active; the subscription that
    /// advances it is scoped to only run during that state (see
    /// `subscription()`), so it costs nothing the rest of the time.
    working_phase: f32,
}

impl Converter {
    pub fn theme(&self) -> iced::Theme {
        self.appearance.theme()
    }

    fn palette(&self) -> Palette {
        match self.appearance {
            Appearance::Light => LIGHT_PALETTE,
            Appearance::Dark => DARK_PALETTE,
        }
    }

    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                screen: Screen::Converter,
                appearance: Appearance::Dark,
                direction: Direction::SvgToVectorDrawable,
                state: ConversionState::Empty,
                last_input: None,
                notice: None,
                window_width: 1040.0,
                code_content: iced::widget::text_editor::Content::new(),
                working_phase: 0.0,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenAbout => {
                self.screen = Screen::About;
            }
            Message::CloseAbout => {
                self.screen = Screen::Converter;
            }
            Message::ToggleAppearance => {
                self.appearance = self.appearance.toggled();
            }
            Message::OpenPersonalWebsite => {
                return Task::perform(open_url(PERSONAL_WEBSITE_URL), |_| {
                    Message::LinkOpenFinished
                });
            }
            Message::OpenIfemDoctrine => {
                return Task::perform(open_url(IFEM_DOCTRINE_URL), |_| Message::LinkOpenFinished);
            }
            Message::LinkOpenFinished => {}
            Message::DirectionSelected(direction) => {
                self.direction = direction;
                self.notice = None;
            }
            Message::PickFileRequested => {
                return Task::perform(pick_file(), Message::FilePicked);
            }
            Message::FilePicked(Some(path)) => {
                self.notice = None;
                self.state = ConversionState::Working {
                    name: file_display_name(&path),
                };
                return Task::perform(load_input(path), Message::InputLoaded);
            }
            Message::FilePicked(None) => {}
            Message::InputLoaded(Ok(input)) => {
                let adjusted_direction = input.kind.natural_direction();
                if self.direction != adjusted_direction {
                    self.direction = adjusted_direction;
                    self.notice = Some(format!(
                        "Direction set to {} after checking the file contents.",
                        adjusted_direction.label()
                    ));
                }
                self.last_input = Some(input.clone());
                self.state = ConversionState::Ready(input);
            }
            Message::InputLoaded(Err(message)) => {
                self.state = ConversionState::Error {
                    name: None,
                    message,
                };
            }
            Message::ConvertRequested => {
                if let ConversionState::Ready(input) = &self.state {
                    let input = input.clone();
                    let direction = self.direction;
                    self.notice = None;
                    self.state = ConversionState::Working {
                        name: input.name.clone(),
                    };
                    return Task::perform(
                        convert_input(input, direction),
                        Message::ConversionFinished,
                    );
                }
            }
            Message::ConversionFinished(Ok(converted)) => {
                self.code_content =
                    iced::widget::text_editor::Content::with_text(&converted.output_text);
                self.state = ConversionState::Done {
                    name: converted.name,
                    direction: converted.direction,
                    source_preview: converted.source_preview.map(image::Handle::from_bytes),
                    output_preview: converted.output_preview.map(image::Handle::from_bytes),
                    output_text: converted.output_text,
                    output_analysis: converted.output_analysis,
                };
            }
            Message::ConversionFinished(Err(message)) => {
                let name = self.last_input.as_ref().map(|input| input.name.clone());
                self.state = ConversionState::Error { name, message };
            }
            Message::RetryRequested => {
                if let Some(input) = self.last_input.clone() {
                    let direction = self.direction;
                    self.notice = None;
                    self.state = ConversionState::Working {
                        name: input.name.clone(),
                    };
                    return Task::perform(
                        convert_input(input, direction),
                        Message::ConversionFinished,
                    );
                }
            }
            Message::CopyOutput => {
                if let ConversionState::Done { output_text, .. } = &self.state {
                    self.notice = Some("Output copied to the clipboard.".to_owned());
                    return clipboard::write(output_text.clone());
                }
            }
            Message::SaveRequested => {
                if let ConversionState::Done {
                    name,
                    direction,
                    output_text,
                    ..
                } = &self.state
                {
                    let default_name = output_name(name, *direction);
                    return Task::perform(
                        save_output(default_name, output_text.clone(), *direction),
                        Message::OutputSaved,
                    );
                }
            }
            Message::OutputSaved(Ok(Some(path))) => {
                self.notice = Some(format!("Saved {}", file_display_name(&path)));
            }
            Message::OutputSaved(Ok(None)) => {}
            Message::OutputSaved(Err(message)) => {
                self.notice = Some(format!("Could not save output: {message}"));
            }
            Message::Reset => {
                self.state = ConversionState::Empty;
                self.last_input = None;
                self.notice = None;
            }
            Message::IcedEvent(iced::Event::Window(window::Event::FileDropped(path)))
                if self.screen == Screen::Converter =>
            {
                self.notice = None;
                self.state = ConversionState::Working {
                    name: file_display_name(&path),
                };
                return Task::perform(load_input(path), Message::InputLoaded);
            }
            Message::IcedEvent(iced::Event::Window(window::Event::Resized(size))) => {
                self.window_width = size.width;
            }
            Message::IcedEvent(iced::Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                ..
            })) => {
                // Deliberately NOT binding Ctrl/Cmd+C to CopyOutput: when
                // the code viewer's text_editor has focus, that same combo
                // already triggers its own built-in Binding::Copy (copying
                // the current SELECTION, which is the more correct
                // behavior there) — adding a second, app-level global
                // binding for the same combo risks double-firing or
                // fighting with the editor's own copy. The toolbar's
                // "Copy" button (whole-output copy) stays mouse-only.
                use keyboard::key::Named;
                match key.as_ref() {
                    keyboard::Key::Character("o") if modifiers.command() => {
                        return Task::perform(pick_file(), Message::FilePicked);
                    }
                    keyboard::Key::Character("s")
                        if modifiers.command()
                            && matches!(self.state, ConversionState::Done { .. }) =>
                    {
                        return self.update(Message::SaveRequested);
                    }
                    keyboard::Key::Named(Named::Escape) if self.screen == Screen::About => {
                        return self.update(Message::CloseAbout);
                    }
                    _ => {}
                }
            }
            Message::CodeEditorAction(action) => {
                // Read-only: every interaction the editor supports EXCEPT
                // actually editing text is allowed through (selection,
                // click-to-position, drag-select, scroll) — see the
                // code_content field doc comment for why `.on_action()`
                // must still be called (omitting it disables selection
                // too, not just editing).
                if !matches!(action, iced::widget::text_editor::Action::Edit(_)) {
                    self.code_content.perform(action);
                }
            }
            Message::WorkingTick => {
                // Simple sawtooth 0.0 -> 1.0 -> 0.0 -> ...: advance by a
                // fixed step each 16ms tick and wrap. Deliberately not tied
                // to real elapsed conversion time (there is none to read),
                // this is purely a "something is happening" sweep, not a
                // percentage — see working_phase's field doc.
                self.working_phase = (self.working_phase + 0.012) % 1.0;
            }
            Message::IcedEvent(_) => {}
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.screen {
            Screen::Converter => self.workspace_view(),
            Screen::About => self.about_view(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match self.screen {
            Screen::Converter => {
                let events = iced::event::listen().map(Message::IcedEvent);
                if matches!(self.state, ConversionState::Working { .. }) {
                    Subscription::batch([
                        events,
                        iced::time::every(std::time::Duration::from_millis(16))
                            .map(|_| Message::WorkingTick),
                    ])
                } else {
                    events
                }
            }
            Screen::About => Subscription::none(),
        }
    }

    fn workspace_view(&self) -> Element<'_, Message> {
        let p = self.palette();

        let header = workspace_header(self.direction, self.appearance, p);

        let mut content = column![header, self.state_view()]
            .spacing(24)
            .width(Length::Fill)
            .max_width(1040);

        if let Some(notice) = &self.notice {
            content = content.push(
                container(text(notice).size(13).color(p.primary))
                    .padding([10, 14])
                    .style(move |_| panel_style(p.surface_muted, p.border)),
            );
        }

        let page = scrollable(container(content).padding(28).center_x(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill);

        container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style::default().background(p.background))
            .into()
    }

    fn state_view(&self) -> Element<'_, Message> {
        match &self.state {
            ConversionState::Empty => self.empty_view(),
            ConversionState::Ready(input) => self.ready_view(input),
            ConversionState::Working { name } => self.working_view(name),
            ConversionState::Done {
                name,
                direction,
                source_preview,
                output_preview,
                output_text,
                output_analysis,
            } => self.done_view(
                name,
                *direction,
                source_preview.as_ref(),
                output_preview.as_ref(),
                output_text,
                output_analysis.as_ref(),
            ),
            ConversionState::Error { name, message } => self.error_view(name.as_deref(), message),
        }
    }

    fn empty_view(&self) -> Element<'_, Message> {
        let p = self.palette();
        let content = column![
            transformation_motif(self.direction, p),
            text("DROP A VECTOR FILE").size(16).color(p.primary),
            text(self.direction.input_hint()).size(15).color(p.muted),
            text("SVG and Android VectorDrawable XML are detected from their contents.")
                .size(13)
                .color(p.muted),
            space::vertical().height(Length::Fixed(12.0)),
            button(text("Choose file").size(15))
                .on_press(Message::PickFileRequested)
                .padding([12, 22])
                .style(move |_, _| primary_button_style(p)),
        ]
        .spacing(10)
        .align_x(iced::Alignment::Center)
        .max_width(580);

        container(center(content))
            .width(Length::Fill)
            .height(Length::Fixed(420.0))
            .padding(28)
            .style(move |_| panel_style(p.surface, p.border))
            .into()
    }

    fn ready_view<'a>(&self, input: &'a InputFile) -> Element<'a, Message> {
        let p = self.palette();
        let file_summary = row![
            text("✓").size(22).color(p.primary),
            column![
                text(&input.name).size(18).color(p.on_surface),
                text(format!(
                    "{} · {} KB",
                    input.kind.label(),
                    input.bytes.len() / 1024 + 1
                ))
                .size(13)
                .color(p.muted),
            ]
            .spacing(3),
            space::horizontal(),
            button(text("Change").size(14))
                .on_press(Message::PickFileRequested)
                .padding([8, 12])
                .style(move |_, _| secondary_button_style(p)),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);

        let content = column![
            text("READY TO CONVERT").size(15).color(p.primary),
            container(file_summary)
                .padding(16)
                .style(move |_| panel_style(p.surface_muted, p.border)),
            preview_panel(input.kind.label(), input.source_preview.as_ref(), p),
            text(format!(
                "{} will produce a clean {} output.",
                input.kind.label(),
                self.direction.output_label()
            ))
            .size(14)
            .color(p.muted),
            space::vertical().height(Length::Fixed(8.0)),
            button(text(format!("Convert {}", self.direction.label())).size(16))
                .on_press(Message::ConvertRequested)
                .padding([13, 22])
                .style(move |_, _| primary_button_style(p)),
        ]
        .spacing(16)
        .align_x(iced::Alignment::Start)
        .max_width(680);

        container(content)
            .width(Length::Fill)
            .padding(28)
            .style(move |_| panel_style(p.surface, p.border))
            .into()
    }

    fn working_view<'a>(&self, name: &'a str) -> Element<'a, Message> {
        let p = self.palette();
        let content = column![
            text("CONVERTING").size(16).color(p.primary),
            text(name).size(20).color(p.on_surface),
            text("Validating the vector and preparing previews. Keep this window open.")
                .size(14)
                .color(p.muted),
            {
                // Indeterminate: no real percentage exists to show (see
                // working_phase's field doc), so this maps the 0..1
                // triangle-wave phase to a back-and-forth sweep rather than
                // a value that reads as "N% done" — the same
                // Duration/Instant-driven approach viewer.rs already uses
                // for AVD frame timing (iced::time::every, tokio feature,
                // already enabled in Cargo.toml).
                let sweep = 1.0 - (self.working_phase * 2.0 - 1.0).abs();
                progress_bar(0.0..=1.0, sweep)
                    .length(Length::Fixed(360.0))
                    .girth(Length::Fixed(8.0))
                    .style(move |_| progress_bar::Style {
                        background: Background::Color(p.surface_muted),
                        bar: Background::Color(p.primary_glow),
                        border: Border::default().rounded(8.0).width(1.0).color(p.border),
                    })
            },
        ]
        .spacing(12)
        .align_x(iced::Alignment::Center)
        .max_width(620);

        container(center(content))
            .width(Length::Fill)
            .height(Length::Fixed(360.0))
            .padding(28)
            .style(move |_| panel_style(p.surface, p.border))
            .into()
    }

    fn done_view<'a>(
        &'a self,
        name: &'a str,
        direction: Direction,
        source_preview: Option<&'a image::Handle>,
        output_preview: Option<&'a image::Handle>,
        output_text: &'a str,
        output_analysis: Option<&'a svg_converter_core::analysis::VectorAnalysis>,
    ) -> Element<'a, Message> {
        let p = self.palette();
        let summary = row![
            text("✓").size(24).color(p.primary),
            column![
                text("CONVERSION COMPLETE").size(15).color(p.primary),
                text(name).size(18).color(p.on_surface),
                row![
                    transformation_motif(direction, p),
                    text(format!("· {} bytes", output_text.len()))
                        .size(13)
                        .color(p.muted),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            ]
            .spacing(3),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);

        const TWO_PANE_BREAKPOINT: f32 = 760.0;
        let previews: Element<'a, Message> = if self.window_width >= TWO_PANE_BREAKPOINT {
            row![
                preview_panel(direction.source_label(), source_preview, p),
                preview_panel(direction.output_label(), output_preview, p),
            ]
            .spacing(16)
            .width(Length::Fill)
            .into()
        } else {
            column![
                preview_panel(direction.source_label(), source_preview, p),
                preview_panel(direction.output_label(), output_preview, p),
            ]
            .spacing(16)
            .width(Length::Fill)
            .into()
        };

        let analysis_element: Element<'a, Message> = match output_analysis {
            Some(a) => vector_analysis_panel(a, p),
            None => column![].into(),
        };

        let content = column![
            summary,
            previews,
            analysis_element,
            column![
                row![
                    column![
                        text("OUTPUT").size(14).color(p.primary),
                        text(format!(
                            "{} · {}",
                            output_name(name, direction),
                            direction.output_label()
                        ))
                        .size(12)
                        .color(p.muted),
                    ]
                    .spacing(2),
                    space::horizontal(),
                    button(text("Select All").size(14))
                        .on_press(Message::CodeEditorAction(
                            iced::widget::text_editor::Action::SelectAll
                        ))
                        .padding([7, 12])
                        .style(move |_, _| secondary_button_style(p)),
                    button(text("Copy").size(14))
                        .on_press(Message::CopyOutput)
                        .padding([7, 12])
                        .style(move |_, _| secondary_button_style(p)),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
                // Not wrapped in a scrollable(): text_editor has its own
                // built-in scrolling (confirmed via iced's 0.13.0
                // changelog referencing a fix for "scrolling in
                // text_editor"), and nesting it in an outer scrollable
                // risks the exact "nested scrollables capturing all scroll
                // events" class of bug iced's own changelog lists as a
                // past fix elsewhere — so the editor is given a fixed
                // height directly and left to handle its own scrolling.
                // .wrapping(Wrapping::None) is what actually produces
                // horizontal overflow instead of wrapping — this is the
                // one part of this widget I could not visually verify
                // (there's a known, filed iced issue where an equivalent
                // call is silently ignored on the plain `text` widget;
                // unconfirmed whether text_editor shares that bug) — check
                // this specifically on a real build.
                iced::widget::text_editor(&self.code_content)
                    .font(iced::Font::MONOSPACE)
                    .size(13)
                    .wrapping(iced::widget::text::Wrapping::None)
                    .height(Length::Fixed(190.0))
                    .on_action(Message::CodeEditorAction),
            ]
            .spacing(10),
            row![
                button(text("Save As…").size(15))
                    .on_press(Message::SaveRequested)
                    .padding([11, 18])
                    .style(move |_, _| primary_button_style(p)),
                button(text("New conversion").size(15))
                    .on_press(Message::Reset)
                    .padding([11, 18])
                    .style(move |_, _| secondary_button_style(p)),
            ]
            .spacing(12),
        ]
        .spacing(20)
        .width(Length::Fill);

        container(content)
            .width(Length::Fill)
            .padding(24)
            .style(move |_| panel_style(p.surface, p.border))
            .into()
    }

    fn about_view(&self) -> Element<'_, Message> {
        let p = self.palette();
        let header = row![
            button(text("← Back").size(14))
                .on_press(Message::CloseAbout)
                .padding([8, 12])
                .style(move |_, _| secondary_button_style(p)),
            space::horizontal(),
            text("About").size(22).color(p.on_background),
            space::horizontal(),
            button(text(self.appearance.label()).size(13))
                .on_press(Message::ToggleAppearance)
                .padding([8, 12])
                .style(move |_, _| secondary_button_style(p)),
        ]
        .align_y(iced::Alignment::Center);

        let hero = column![
            container(
                image(image::Handle::from_bytes(ABOUT_LOGO))
                    .width(Length::Fixed(106.0))
                    .height(Length::Fixed(106.0))
                    .content_fit(ContentFit::Contain),
            )
            .padding(12)
            .style(move |_| panel_style(p.surface, p.border)),
            text("Watermelon").size(34).color(p.on_background),
            text("Vector Graphics Converter").size(17).color(p.muted),
        ]
        .spacing(8)
        .align_x(iced::Alignment::Center);

        let ifem_signature = container(
            row![
                ifem_mark(),
                column![
                    text("Built with IFEM").size(16).color(p.primary),
                    text("Interface-First Engineering Methodology")
                        .size(13)
                        .color(p.muted),
                ]
                .spacing(3),
            ]
            .spacing(14)
            .align_y(iced::Alignment::Center),
        )
        .padding([14, 18])
        .width(Length::Fill)
        .style(move |_| panel_style(p.surface, p.border));

        let badges = column![
            row![
                technology_badge("Kotlin", p),
                technology_badge("Jetpack Compose", p),
                technology_badge("Material 3", p),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                technology_badge("Rust", p),
                technology_badge("JNI", p),
                technology_badge("resvg", p),
                technology_badge("SVG", p),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
            row![
                technology_badge("libSodium", p),
                technology_badge("vodozemac", p),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(8)
        .align_x(iced::Alignment::Center);

        let stack = column![
            section_divider("TECHNOLOGY STACK", p),
            technology_layer(
                "Application layer",
                "Kotlin · Jetpack Compose · Material 3",
                p
            ),
            technology_layer("Native processing layer", "Rust · JNI", p),
            technology_layer(
                "Graphics & security",
                "resvg · SVG processing · libSodium · vodozemac",
                p
            ),
        ]
        .spacing(10)
        .align_x(iced::Alignment::Center);

        let developer = column![
            row![
                horizontal_divider(p),
                container(space::horizontal())
                    .width(Length::Fixed(9.0))
                    .height(Length::Fixed(9.0))
                    .style(move |_| container::Style {
                        background: Some(Background::Color(p.watermelon_red)),
                        border: Border::default().rounded(99.0),
                        ..container::Style::default()
                    }),
                horizontal_divider(p),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
            text("DEVELOPED BY").size(12).color(p.primary),
            text("Soheil Mozaffari").size(22).color(p.on_background),
            text("Software Engineer · Systems Architect")
                .size(14)
                .color(p.muted),
        ]
        .spacing(6)
        .align_x(iced::Alignment::Center)
        .width(Length::Fill);

        let personal_site = button(
            row![
                link_glyph(p),
                column![
                    text("Personal website").size(15).color(p.primary),
                    text("Visit Soheil Mozaffari online")
                        .size(13)
                        .color(p.muted),
                    text(PERSONAL_WEBSITE_LABEL).size(13).color(p.primary),
                ]
                .spacing(3)
                .width(Length::Fill),
            ]
            .spacing(14)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::OpenPersonalWebsite)
        .padding(16)
        .width(Length::Fill)
        .style(move |_, _| link_card_style(p));

        let doctrine = button(
            row![
                ifem_mark(),
                column![
                    text("Architected using IFEM Doctrine")
                        .size(15)
                        .color(p.primary),
                    text("Learn more about Interface-First Engineering Methodology")
                        .size(13)
                        .color(p.muted),
                    text(IFEM_DOCTRINE_LABEL).size(13).color(p.primary),
                ]
                .spacing(3)
                .width(Length::Fill),
            ]
            .spacing(14)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::OpenIfemDoctrine)
        .padding(16)
        .width(Length::Fill)
        .style(move |_, _| link_card_style(p));

        let footer = column![
            text("© 2026 Soheil Mozaffari · All rights reserved.")
                .size(12)
                .color(p.muted),
            text("Proprietary and source-available.")
                .size(12)
                .color(p.muted),
        ]
        .spacing(2)
        .align_x(iced::Alignment::Center);

        let content = column![
            header,
            hero,
            ifem_signature,
            badges,
            stack,
            developer,
            personal_site,
            doctrine,
            footer,
        ]
        .spacing(22)
        .align_x(iced::Alignment::Center)
        .max_width(660)
        .width(Length::Fill);

        let page = scrollable(container(content).padding(32).center_x(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill);

        container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style::default().background(p.background))
            .into()
    }

    fn error_view<'a>(&self, name: Option<&'a str>, message: &'a str) -> Element<'a, Message> {
        let p = self.palette();
        let mut actions = row![button(text("Choose another file").size(15))
            .on_press(Message::PickFileRequested)
            .padding([11, 18])
            .style(move |_, _| secondary_button_style(p)),]
        .spacing(12);

        if self.last_input.is_some() {
            actions = actions.push(
                button(text("Try again").size(15))
                    .on_press(Message::RetryRequested)
                    .padding([11, 18])
                    .style(move |_, _| primary_button_style(p)),
            );
        }

        let mut content =
            column![text("CONVERSION NEEDS ATTENTION").size(15).color(p.error),].spacing(10);

        if let Some(name) = name {
            content = content.push(text(name).size(18).color(p.on_surface));
        }

        content = content
            .push(text(message).size(14).color(p.muted))
            .push(
                text(
                    "No output was written. You can retry the same file or choose a different one.",
                )
                .size(13)
                .color(p.muted),
            )
            .push(actions);

        container(content)
            .width(Length::Fill)
            .padding(28)
            .style(move |_| panel_style(p.surface, p.watermelon_red))
            .into()
    }
}

fn horizontal_divider<'a>(p: Palette) -> Element<'a, Message> {
    container(space::horizontal())
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(move |_| container::Style::default().background(p.border))
        .into()
}

fn section_divider<'a>(label: &'a str, p: Palette) -> Element<'a, Message> {
    row![
        horizontal_divider(p),
        text(label).size(12).color(p.muted),
        horizontal_divider(p),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center)
    .into()
}

fn link_glyph<'a>(p: Palette) -> Element<'a, Message> {
    container(text("↗").size(23).color(p.watermelon_red))
        .width(Length::Fixed(38.0))
        .height(Length::Fixed(38.0))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(p.watermelon_red.scale_alpha(0.16))),
            border: Border::default().rounded(99.0),
            ..container::Style::default()
        })
        .into()
}

fn ifem_mark<'a>() -> Element<'a, Message> {
    canvas::Canvas::new(IfemMark)
        .width(Length::Fixed(38.0))
        .height(Length::Fixed(46.0))
        .into()
}

fn link_card_style(p: Palette) -> button::Style {
    button::Style {
        background: Some(Background::Color(p.surface)),
        text_color: p.on_surface,
        border: Border::default().rounded(18.0).width(1.0).color(p.border),
        ..button::Style::default()
    }
}

struct IfemMark;

impl<Message> canvas::Program<Message> for IfemMark {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let point =
            |x: f32, y: f32| Point::new(bounds.width * x / 108.0, bounds.height * y / 132.0);
        let navy = Color::from_rgb8(18, 53, 101);
        let green = Color::from_rgb8(37, 141, 120);
        let blue = Color::from_rgb8(46, 103, 181);
        let gold = Color::from_rgb8(242, 165, 26);

        for points in [
            [
                (8.0, 10.0),
                (39.0, 10.0),
                (39.0, 15.0),
                (13.0, 15.0),
                (13.0, 40.0),
                (8.0, 40.0),
            ],
            [
                (69.0, 10.0),
                (100.0, 10.0),
                (100.0, 40.0),
                (95.0, 40.0),
                (95.0, 15.0),
                (69.0, 15.0),
            ],
            [
                (8.0, 88.0),
                (13.0, 88.0),
                (13.0, 113.0),
                (39.0, 113.0),
                (39.0, 118.0),
                (8.0, 118.0),
            ],
            [
                (95.0, 88.0),
                (100.0, 88.0),
                (100.0, 118.0),
                (69.0, 118.0),
                (69.0, 113.0),
                (95.0, 113.0),
            ],
        ] {
            let path = canvas::Path::new(|builder| {
                builder.move_to(point(points[0].0, points[0].1));
                for (x, y) in points.iter().skip(1) {
                    builder.line_to(point(*x, *y));
                }
                builder.close();
            });
            frame.fill(&path, navy);
        }
        for (top, color) in [(28.0, green), (56.0, blue), (84.0, gold)] {
            let diamond = canvas::Path::new(|builder| {
                builder.move_to(point(54.0, top));
                builder.line_to(point(88.0, top + 17.0));
                builder.line_to(point(54.0, top + 34.0));
                builder.line_to(point(20.0, top + 17.0));
                builder.close();
            });
            frame.fill(&diamond, color);
        }
        frame.stroke(
            &canvas::Path::line(point(54.0, 22.0), point(54.0, 28.0)),
            canvas::Stroke::default().with_width(2.5).with_color(navy),
        );
        frame.stroke(
            &canvas::Path::line(point(54.0, 118.0), point(54.0, 124.0)),
            canvas::Stroke::default().with_width(2.5).with_color(navy),
        );
        frame.fill(
            &canvas::Path::circle(point(54.0, 13.0), bounds.width * 4.5 / 108.0),
            green,
        );
        frame.fill(
            &canvas::Path::circle(point(54.0, 129.0), bounds.width * 4.5 / 108.0),
            gold,
        );
        for (y, color) in [(59.0, blue), (87.0, gold)] {
            frame.fill(
                &canvas::Path::circle(point(54.0, y), bounds.width * 5.0 / 108.0),
                Color::WHITE,
            );
            frame.fill(
                &canvas::Path::circle(point(54.0, y), bounds.width * 3.0 / 108.0),
                color,
            );
        }
        vec![frame.into_geometry()]
    }
}

fn technology_badge<'a>(label: &'a str, p: Palette) -> Element<'a, Message> {
    container(text(label).size(13).color(p.on_surface))
        .padding([7, 11])
        .style(move |_| panel_style(p.surface_muted, p.border))
        .into()
}

fn technology_layer<'a>(title: &'a str, detail: &'a str, p: Palette) -> Element<'a, Message> {
    container(
        column![
            text(title).size(15).color(p.primary),
            text(detail).size(13).color(p.muted),
        ]
        .spacing(3)
        .align_x(iced::Alignment::Center),
    )
    .padding([11, 16])
    .width(Length::Fill)
    .style(move |_| panel_style(p.surface, p.border))
    .into()
}

/// The top app bar: brand mark, the SVG↔VectorDrawable direction switch, and
/// the About/theme-toggle buttons. Extracted out of `workspace_view` (unlike
/// most other functions pulled out in this pass, this one is a real,
/// non-cosmetic split: it removes the need to recompute
/// `svg_selected`/`xml_selected` and rebuild this exact widget tree inline
/// were it ever needed elsewhere).
fn workspace_header<'a>(
    direction: Direction,
    appearance: Appearance,
    p: Palette,
) -> Element<'a, Message> {
    let svg_selected = direction == Direction::SvgToVectorDrawable;
    let xml_selected = direction == Direction::VectorDrawableToSvg;

    let direction_switch = row![
        button(text("SVG → XML").size(14))
            .on_press(Message::DirectionSelected(Direction::SvgToVectorDrawable))
            .padding([8, 14])
            .style(move |_, _| direction_button_style(svg_selected, p)),
        button(text("XML → SVG").size(14))
            .on_press(Message::DirectionSelected(Direction::VectorDrawableToSvg))
            .padding([8, 14])
            .style(move |_, _| direction_button_style(xml_selected, p)),
    ]
    .spacing(4);

    row![
        column![
            text("WATERMELON").size(20).color(p.primary),
            text("VECTOR CONVERTER").size(12).color(p.muted),
        ]
        .spacing(1),
        space::horizontal(),
        direction_switch,
        button(text("About").size(13))
            .on_press(Message::OpenAbout)
            .padding([8, 12])
            .style(move |_, _| secondary_button_style(p)),
        button(text(appearance.label()).size(13))
            .on_press(Message::ToggleAppearance)
            .padding([8, 12])
            .style(move |_, _| secondary_button_style(p)),
    ]
    .align_y(iced::Alignment::Center)
    .padding([4, 0])
    .into()
}

/// A compact, informational format label — SVG / XML / (future) ZIP. Not a
/// button: no `on_press`, purely a static pill of colored text, matching
/// the redesign prompt's explicit "compact informational labels, not
/// buttons" instruction. Background derives from `p.primary` via
/// `scale_alpha` (the same pattern already used in `link_glyph` — see its
/// doc comment from an earlier phase) rather than a new hardcoded color:
/// SVG/XML both represent "a real, converted vector format," which reads
/// as the same positive/primary semantic this app already uses for
/// success states (see done_view's "✓ CONVERSION COMPLETE", also colored
/// via p.primary) — so the badge intentionally borrows that same meaning
/// rather than introducing a third, competing color idea. BatchZip is
/// deliberately excluded from this green association (see its own arm
/// below): a batch/zip container isn't a converted vector format, so it
/// uses a neutral muted/border pairing instead, so it can't be mistaken
/// for a third real conversion format.
fn format_badge<'a>(format: VectorFormat, p: Palette) -> Element<'a, Message> {
    let (background, text_color) = match format {
        VectorFormat::Svg | VectorFormat::VdXml => (p.primary.scale_alpha(0.16), p.primary),
        VectorFormat::BatchZip => (p.surface_muted, p.muted),
    };

    container(
        text(format.short_label())
            .size(12)
            .color(text_color)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            }),
    )
    .padding([4, 10])
    .style(move |_| container::Style {
        background: Some(Background::Color(background)),
        border: Border::default().rounded(999.0),
        ..container::Style::default()
    })
    .into()
}

/// The shared Source -> Result transformation motif: `[SOURCE badge] →
/// [RESULT badge]`. Deliberately just the two badges plus a plain arrow
/// glyph — no morphing, no animation, no gradient, no particles, per the
/// redesign prompt's explicit "keep it restrained, static, and functional"
/// instruction. `direction` alone is enough to derive both badges (via
/// `Direction::source_format`/`output_format`), so every call site only
/// ever needs to pass the current conversion direction, not two separate
/// format values that could get swapped by mistake.
fn transformation_motif<'a>(direction: Direction, p: Palette) -> Element<'a, Message> {
    row![
        format_badge(direction.source_format(), p),
        text("→").size(16).color(p.muted),
        format_badge(direction.output_format(), p),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
    .into()
}

fn preview_panel<'a>(
    label: &'a str,
    handle: Option<&'a image::Handle>,
    p: Palette,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = match handle {
        Some(handle) => image(handle.clone())
            .width(Length::Fill)
            .height(Length::Fixed(220.0))
            .content_fit(ContentFit::Contain)
            .into(),
        None => center(text("Preview unavailable").size(13).color(p.muted))
            .height(Length::Fixed(220.0))
            .into(),
    };

    container(column![text(label).size(12).color(p.muted), content].spacing(10))
        .width(Length::FillPortion(1))
        .padding(14)
        .style(move |_| panel_style(p.surface_muted, p.border))
        .into()
}

/// Factual structural summary of the OUTPUT file — no fidelity score, no
/// qualitative rating, no warnings. `VectorAnalysis` describes what the
/// converter's own parser found by walking the output it just produced; it
/// says nothing about what may have been lost, because — per
/// svg_parser.rs's own doc comment — this converter's conversion is strict
/// all-or-nothing (an unsupported construct is a hard `ConversionError`),
/// so there is no partial-fidelity state to report. Naming this "Vector
/// Analysis", not "Fidelity" or "Compatibility", is deliberate: it should
/// never imply a check that didn't happen.
fn vector_analysis_panel<'a>(
    a: &svg_converter_core::analysis::VectorAnalysis,
    p: Palette,
) -> Element<'a, Message> {
    let mut facts = vec![format!(
        "{} path{}",
        a.path_count,
        if a.path_count == 1 { "" } else { "s" }
    )];
    if a.group_count > 0 {
        facts.push(format!(
            "{} group{}",
            a.group_count,
            if a.group_count == 1 { "" } else { "s" }
        ));
    }
    facts.push(format!("{:.0} × {:.0}", a.width, a.height));
    if a.uses_gradients {
        facts.push("gradients".to_owned());
    }
    if a.uses_strokes {
        facts.push("strokes".to_owned());
    }
    if let Some(tint) = &a.tint_color {
        facts.push(format!("single color ({tint})"));
    }
    if a.is_animated {
        facts.push("animated".to_owned());
    }

    container(
        column![
            text("VECTOR ANALYSIS").size(12).color(p.muted),
            text(facts.join("   ·   ")).size(14).color(p.on_surface),
        ]
        .spacing(6),
    )
    .width(Length::Fill)
    .padding(14)
    .style(move |_| panel_style(p.surface_muted, p.border))
    .into()
}

fn panel_style(background: Color, border_color: Color) -> container::Style {
    container::Style {
        background: Some(Background::Color(background)),
        border: Border::default()
            .rounded(16.0)
            .width(1.0)
            .color(border_color),
        ..container::Style::default()
    }
}

fn primary_button_style(p: Palette) -> button::Style {
    button::Style {
        background: Some(Background::Color(p.primary)),
        text_color: p.background,
        border: Border::default().rounded(10.0),
        ..button::Style::default()
    }
}

fn secondary_button_style(p: Palette) -> button::Style {
    button::Style {
        background: Some(Background::Color(p.surface_muted)),
        text_color: p.on_surface,
        border: Border::default().rounded(10.0).width(1.0).color(p.border),
        ..button::Style::default()
    }
}

fn direction_button_style(selected: bool, p: Palette) -> button::Style {
    if selected {
        primary_button_style(p)
    } else {
        button::Style {
            background: Some(Background::Color(p.surface_muted)),
            text_color: p.muted,
            border: Border::default().rounded(9.0).width(1.0).color(p.border),
            ..button::Style::default()
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenAbout,
    CloseAbout,
    ToggleAppearance,
    OpenPersonalWebsite,
    OpenIfemDoctrine,
    LinkOpenFinished,
    DirectionSelected(Direction),
    PickFileRequested,
    FilePicked(Option<PathBuf>),
    InputLoaded(Result<InputFile, String>),
    ConvertRequested,
    ConversionFinished(Result<ConvertedOutput, String>),
    RetryRequested,
    CopyOutput,
    SaveRequested,
    OutputSaved(Result<Option<PathBuf>, String>),
    Reset,
    IcedEvent(iced::Event),
    /// Any interaction with the code viewer's `text_editor` — click, drag,
    /// scroll, select, or (filtered out in update()) an edit attempt. Named
    /// generically rather than e.g. `CodeSelectAll` because `on_action`
    /// hands back every kind of `text_editor::Action` uniformly; the
    /// specific "Select All" button in the UI just dispatches
    /// `Action::SelectAll` through this same message.
    CodeEditorAction(iced::widget::text_editor::Action),
    /// Advances the indeterminate progress animation in `working_view` —
    /// only ever dispatched while `ConversionState::Working` is active
    /// (see `subscription()`).
    WorkingTick,
}

async fn open_url(url: &'static str) {
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let _ = Command::new("xdg-open").arg(url).spawn();
}

async fn pick_file() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .add_filter("SVG / VectorDrawable", &["svg", "xml"])
        .pick_file()
        .await
        .map(|handle| handle.path().to_path_buf())
}

async fn load_input(path: PathBuf) -> Result<InputFile, String> {
    let bytes = std::fs::read(&path).map_err(|error| format!("Could not read file: {error}"))?;
    if bytes.is_empty() {
        return Err("The selected file is empty.".to_owned());
    }

    let content = String::from_utf8_lossy(&bytes);
    let root = content.trim_start_matches('\u{feff}').trim_start();
    let kind = if root.starts_with("<svg") || root.starts_with("<?xml") && root.contains("<svg") {
        InputKind::Svg
    } else if root.starts_with("<vector")
        || root.starts_with("<animated-vector")
        || root.starts_with("<?xml") && root.contains("<vector")
    {
        InputKind::VectorDrawable
    } else {
        return Err("Choose an SVG or Android VectorDrawable XML file. The root element was not recognised.".to_owned());
    };

    let source_preview = match kind {
        InputKind::Svg => svg_converter_core::image_export::render_svg_preview(&bytes, 720).ok(),
        InputKind::VectorDrawable => {
            svg_converter_core::image_export::render_vd_preview(&content, 720).ok()
        }
    }
    .map(image::Handle::from_bytes);

    Ok(InputFile {
        name: file_display_name(&path),
        bytes,
        kind,
        source_preview,
    })
}

async fn convert_input(input: InputFile, direction: Direction) -> Result<ConvertedOutput, String> {
    if input.kind.natural_direction() != direction {
        return Err(format!(
            "{} input is incompatible with {}. Switch the direction and try again.",
            input.kind.label(),
            direction.label()
        ));
    }

    let (output_text, source_preview, output_preview) = match direction {
        Direction::SvgToVectorDrawable => {
            let output =
                svg_converter_core::convert_svg(&input.bytes).map_err(|error| error.to_string())?;
            let source =
                svg_converter_core::image_export::render_svg_preview(&input.bytes, 720).ok();
            let generated = svg_converter_core::image_export::render_vd_preview(&output, 720).ok();
            (output, source, generated)
        }
        Direction::VectorDrawableToSvg => {
            let output =
                svg_converter_core::convert_vd(&input.bytes).map_err(|error| error.to_string())?;
            let source = svg_converter_core::image_export::render_vd_preview(
                &String::from_utf8_lossy(&input.bytes),
                720,
            )
            .ok();
            let generated =
                svg_converter_core::image_export::render_svg_preview(output.as_bytes(), 720).ok();
            (output, source, generated)
        }
    };

    // Structural analysis of the OUTPUT — analyze_vector/analyze_vd_vector
    // are swapped relative to the render calls above, since the output of
    // SvgToVectorDrawable is VectorDrawable XML (needs the VD analyzer) and
    // vice versa; a best-effort summary, so a failure here doesn't fail the
    // conversion that already genuinely succeeded.
    let output_analysis = match direction {
        Direction::SvgToVectorDrawable => {
            svg_converter_core::analyze_vd_vector(output_text.as_bytes()).ok()
        }
        Direction::VectorDrawableToSvg => {
            svg_converter_core::analyze_vector(output_text.as_bytes()).ok()
        }
    };

    Ok(ConvertedOutput {
        name: input.name,
        direction,
        source_preview,
        output_preview,
        output_text,
        output_analysis,
    })
}

async fn save_output(
    default_name: String,
    output: String,
    direction: Direction,
) -> Result<Option<PathBuf>, String> {
    let filter = match direction {
        Direction::SvgToVectorDrawable => "Android VectorDrawable XML",
        Direction::VectorDrawableToSvg => "SVG",
    };

    let Some(handle) = rfd::AsyncFileDialog::new()
        .set_file_name(&default_name)
        .add_filter(filter, &[direction.extension()])
        .save_file()
        .await
    else {
        return Ok(None);
    };

    let path = handle.path().to_path_buf();
    std::fs::write(&path, output).map_err(|error| format!("{error}"))?;
    Ok(Some(path))
}

fn output_name(name: &str, direction: Direction) -> String {
    let stem = Path::new(name)
        .file_stem()
        .map(|stem| stem.to_string_lossy())
        .unwrap_or_else(|| name.into());
    format!("{stem}.{}", direction.extension())
}

fn file_display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}
