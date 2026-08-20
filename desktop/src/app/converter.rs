// Watermelon Vector Converter — Desktop conversion workspace.
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

use iced::widget::{
    button, center, column, container, image, progress_bar, row, scrollable, space, text,
};
use iced::{
    clipboard, window, Background, Border, Color, ContentFit, Element, Length, Subscription, Task,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

const SPLASH_ART: &[u8] = include_bytes!("../../../icons/png/512.png");
const SPLASH_TICK: Duration = Duration::from_millis(90);
const SPLASH_STEP: f32 = 20.0;

const BACKGROUND: Color = Color::from_rgb8(5, 12, 9);
const SURFACE: Color = Color::from_rgb8(12, 29, 20);
const SURFACE_MUTED: Color = Color::from_rgb8(18, 43, 30);
const WATERMELON_RED: Color = Color::from_rgb8(205, 48, 65);
const RIND_GREEN: Color = Color::from_rgb8(64, 197, 86);
const GLOW_GREEN: Color = Color::from_rgb8(138, 235, 78);
const MUTED_GREEN: Color = Color::from_rgb8(156, 215, 161);
const MUTED_TEXT: Color = Color::from_rgb8(178, 195, 183);
const ERROR_RED: Color = Color::from_rgb8(255, 142, 152);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Splash,
    Converter,
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
}

pub struct Converter {
    screen: Screen,
    splash_progress: f32,
    direction: Direction,
    state: ConversionState,
    last_input: Option<InputFile>,
    notice: Option<String>,
}

impl Converter {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                screen: Screen::Splash,
                splash_progress: 0.0,
                direction: Direction::SvgToVectorDrawable,
                state: ConversionState::Empty,
                last_input: None,
                notice: None,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SplashTick if self.screen == Screen::Splash => {
                self.splash_progress = (self.splash_progress + SPLASH_STEP).min(100.0);
                if self.splash_progress >= 100.0 {
                    self.screen = Screen::Converter;
                }
            }
            Message::SplashTick => {}
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
                self.state = ConversionState::Done {
                    name: converted.name,
                    direction: converted.direction,
                    source_preview: converted.source_preview.map(image::Handle::from_bytes),
                    output_preview: converted.output_preview.map(image::Handle::from_bytes),
                    output_text: converted.output_text,
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
            Message::IcedEvent(_) => {}
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.screen {
            Screen::Splash => self.splash_view(),
            Screen::Converter => self.workspace_view(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match self.screen {
            Screen::Splash => iced::time::every(SPLASH_TICK).map(|_| Message::SplashTick),
            Screen::Converter => iced::event::listen().map(Message::IcedEvent),
        }
    }

    fn splash_view(&self) -> Element<'_, Message> {
        let progress = progress_bar(0.0..=100.0, self.splash_progress)
            .length(Length::Fixed(240.0))
            .girth(Length::Fixed(8.0))
            .style(|_| progress_bar::Style {
                background: Background::Color(SURFACE),
                bar: Background::Color(GLOW_GREEN),
                border: Border::default().rounded(8.0).width(1.0).color(RIND_GREEN),
            });

        let content = column![
            image(image::Handle::from_bytes(SPLASH_ART))
                .width(Length::Fixed(128.0))
                .height(Length::Fixed(128.0)),
            text("WATERMELON").size(30).color(Color::WHITE),
            text("VECTOR CONVERTER").size(14).color(RIND_GREEN),
            space::vertical().height(Length::Fixed(18.0)),
            progress,
        ]
        .spacing(10)
        .align_x(iced::Alignment::Center)
        .width(Length::Shrink);

        container(center(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style::default().background(BACKGROUND))
            .into()
    }

    fn workspace_view(&self) -> Element<'_, Message> {
        let svg_selected = self.direction == Direction::SvgToVectorDrawable;
        let xml_selected = self.direction == Direction::VectorDrawableToSvg;

        let direction_switch = row![
            button(text("SVG → XML").size(14))
                .on_press(Message::DirectionSelected(Direction::SvgToVectorDrawable))
                .padding([8, 14])
                .style(move |_, _| direction_button_style(svg_selected)),
            button(text("XML → SVG").size(14))
                .on_press(Message::DirectionSelected(Direction::VectorDrawableToSvg))
                .padding([8, 14])
                .style(move |_, _| direction_button_style(xml_selected)),
        ]
        .spacing(4);

        let header = row![
            column![
                text("WATERMELON").size(20).color(GLOW_GREEN),
                text("VECTOR CONVERTER").size(12).color(MUTED_TEXT),
            ]
            .spacing(1),
            space::horizontal(),
            direction_switch,
        ]
        .align_y(iced::Alignment::Center)
        .padding([4, 0]);

        let mut content = column![header, self.state_view()]
            .spacing(24)
            .width(Length::Fill)
            .max_width(1040);

        if let Some(notice) = &self.notice {
            content = content.push(
                container(text(notice).size(13).color(MUTED_GREEN))
                    .padding([10, 14])
                    .style(|_| panel_style(SURFACE_MUTED, RIND_GREEN)),
            );
        }

        let page = scrollable(container(content).padding(28).center_x(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill);

        container(page)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style::default().background(BACKGROUND))
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
            } => self.done_view(
                name,
                *direction,
                source_preview.as_ref(),
                output_preview.as_ref(),
                output_text,
            ),
            ConversionState::Error { name, message } => self.error_view(name.as_deref(), message),
        }
    }

    fn empty_view(&self) -> Element<'_, Message> {
        let content = column![
            text("DROP A VECTOR FILE").size(16).color(GLOW_GREEN),
            text(self.direction.input_hint()).size(15).color(MUTED_TEXT),
            text("SVG and Android VectorDrawable XML are detected from their contents.")
                .size(13)
                .color(MUTED_TEXT),
            space::vertical().height(Length::Fixed(12.0)),
            button(text("Choose file").size(15))
                .on_press(Message::PickFileRequested)
                .padding([12, 22])
                .style(|_, _| primary_button_style()),
        ]
        .spacing(10)
        .align_x(iced::Alignment::Center)
        .max_width(580);

        container(center(content))
            .width(Length::Fill)
            .height(Length::Fixed(420.0))
            .padding(28)
            .style(|_| panel_style(SURFACE, RIND_GREEN))
            .into()
    }

    fn ready_view<'a>(&self, input: &'a InputFile) -> Element<'a, Message> {
        let file_summary = row![
            text("✓").size(22).color(GLOW_GREEN),
            column![
                text(&input.name).size(18).color(Color::WHITE),
                text(format!(
                    "{} · {} KB",
                    input.kind.label(),
                    input.bytes.len() / 1024 + 1
                ))
                .size(13)
                .color(MUTED_TEXT),
            ]
            .spacing(3),
            space::horizontal(),
            button(text("Change").size(14))
                .on_press(Message::PickFileRequested)
                .padding([8, 12])
                .style(|_, _| secondary_button_style()),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);

        let content = column![
            text("READY TO CONVERT").size(15).color(GLOW_GREEN),
            container(file_summary)
                .padding(16)
                .style(|_| panel_style(SURFACE_MUTED, Color::from_rgb8(49, 96, 65))),
            text(format!(
                "{} will produce a clean {} output.",
                input.kind.label(),
                self.direction.output_label()
            ))
            .size(14)
            .color(MUTED_TEXT),
            space::vertical().height(Length::Fixed(8.0)),
            button(text(format!("Convert {}", self.direction.label())).size(16))
                .on_press(Message::ConvertRequested)
                .padding([13, 22])
                .style(|_, _| primary_button_style()),
        ]
        .spacing(16)
        .align_x(iced::Alignment::Start)
        .max_width(680);

        container(content)
            .width(Length::Fill)
            .padding(28)
            .style(|_| panel_style(SURFACE, RIND_GREEN))
            .into()
    }

    fn working_view<'a>(&self, name: &'a str) -> Element<'a, Message> {
        let content = column![
            text("CONVERTING").size(16).color(GLOW_GREEN),
            text(name).size(20).color(Color::WHITE),
            text("Validating the vector and preparing previews. Keep this window open.")
                .size(14)
                .color(MUTED_TEXT),
            progress_bar(0.0..=1.0, 0.72)
                .length(Length::Fixed(360.0))
                .girth(Length::Fixed(8.0))
                .style(|_| progress_bar::Style {
                    background: Background::Color(SURFACE_MUTED),
                    bar: Background::Color(GLOW_GREEN),
                    border: Border::default().rounded(8.0),
                }),
        ]
        .spacing(12)
        .align_x(iced::Alignment::Center)
        .max_width(620);

        container(center(content))
            .width(Length::Fill)
            .height(Length::Fixed(360.0))
            .padding(28)
            .style(|_| panel_style(SURFACE, RIND_GREEN))
            .into()
    }

    fn done_view<'a>(
        &self,
        name: &'a str,
        direction: Direction,
        source_preview: Option<&'a image::Handle>,
        output_preview: Option<&'a image::Handle>,
        output_text: &'a str,
    ) -> Element<'a, Message> {
        let summary = row![
            text("✓").size(24).color(GLOW_GREEN),
            column![
                text("CONVERSION COMPLETE").size(15).color(GLOW_GREEN),
                text(name).size(18).color(Color::WHITE),
                text(format!(
                    "{} · {} bytes",
                    direction.label(),
                    output_text.len()
                ))
                .size(13)
                .color(MUTED_TEXT),
            ]
            .spacing(3),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);

        let previews = row![
            preview_panel(direction.source_label(), source_preview),
            preview_panel(direction.output_label(), output_preview),
        ]
        .spacing(16)
        .width(Length::Fill);

        let output = column![
            row![
                text("OUTPUT").size(14).color(GLOW_GREEN),
                space::horizontal(),
                button(text("Copy").size(14))
                    .on_press(Message::CopyOutput)
                    .padding([7, 12])
                    .style(|_, _| secondary_button_style()),
            ]
            .align_y(iced::Alignment::Center),
            scrollable(
                text(output_text)
                    .size(13)
                    .color(Color::from_rgb8(228, 238, 231))
            )
            .height(Length::Fixed(190.0)),
        ]
        .spacing(10);

        let actions = row![
            button(text("Save As…").size(15))
                .on_press(Message::SaveRequested)
                .padding([11, 18])
                .style(|_, _| primary_button_style()),
            button(text("New conversion").size(15))
                .on_press(Message::Reset)
                .padding([11, 18])
                .style(|_, _| secondary_button_style()),
        ]
        .spacing(12);

        let content = column![summary, previews, output, actions]
            .spacing(20)
            .width(Length::Fill);

        container(content)
            .width(Length::Fill)
            .padding(24)
            .style(|_| panel_style(SURFACE, RIND_GREEN))
            .into()
    }

    fn error_view<'a>(&self, name: Option<&'a str>, message: &'a str) -> Element<'a, Message> {
        let mut actions = row![button(text("Choose another file").size(15))
            .on_press(Message::PickFileRequested)
            .padding([11, 18])
            .style(|_, _| secondary_button_style()),]
        .spacing(12);

        if self.last_input.is_some() {
            actions = actions.push(
                button(text("Try again").size(15))
                    .on_press(Message::RetryRequested)
                    .padding([11, 18])
                    .style(|_, _| primary_button_style()),
            );
        }

        let mut content =
            column![text("CONVERSION NEEDS ATTENTION").size(15).color(ERROR_RED),].spacing(10);

        if let Some(name) = name {
            content = content.push(text(name).size(18).color(Color::WHITE));
        }

        content = content
            .push(text(message).size(14).color(MUTED_TEXT))
            .push(
                text(
                    "No output was written. You can retry the same file or choose a different one.",
                )
                .size(13)
                .color(MUTED_TEXT),
            )
            .push(actions);

        container(content)
            .width(Length::Fill)
            .padding(28)
            .style(|_| panel_style(SURFACE, WATERMELON_RED))
            .into()
    }
}

fn preview_panel<'a>(label: &'a str, handle: Option<&'a image::Handle>) -> Element<'a, Message> {
    let content: Element<'a, Message> = match handle {
        Some(handle) => image(handle.clone())
            .width(Length::Fill)
            .height(Length::Fixed(220.0))
            .content_fit(ContentFit::Contain)
            .into(),
        None => center(text("Preview unavailable").size(13).color(MUTED_TEXT))
            .height(Length::Fixed(220.0))
            .into(),
    };

    container(column![text(label).size(12).color(MUTED_TEXT), content].spacing(10))
        .width(Length::FillPortion(1))
        .padding(14)
        .style(|_| panel_style(SURFACE_MUTED, Color::from_rgb8(49, 96, 65)))
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

fn primary_button_style() -> button::Style {
    button::Style {
        background: Some(Background::Color(RIND_GREEN)),
        text_color: BACKGROUND,
        border: Border::default().rounded(10.0),
        ..button::Style::default()
    }
}

fn secondary_button_style() -> button::Style {
    button::Style {
        background: Some(Background::Color(SURFACE_MUTED)),
        text_color: Color::WHITE,
        border: Border::default()
            .rounded(10.0)
            .width(1.0)
            .color(Color::from_rgb8(62, 119, 78)),
        ..button::Style::default()
    }
}

fn direction_button_style(selected: bool) -> button::Style {
    if selected {
        button::Style {
            background: Some(Background::Color(RIND_GREEN)),
            text_color: BACKGROUND,
            border: Border::default().rounded(9.0),
            ..button::Style::default()
        }
    } else {
        button::Style {
            background: Some(Background::Color(SURFACE_MUTED)),
            text_color: MUTED_TEXT,
            border: Border::default().rounded(9.0),
            ..button::Style::default()
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SplashTick,
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

    Ok(InputFile {
        name: file_display_name(&path),
        bytes,
        kind,
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

    Ok(ConvertedOutput {
        name: input.name,
        direction,
        source_preview,
        output_preview,
        output_text,
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
