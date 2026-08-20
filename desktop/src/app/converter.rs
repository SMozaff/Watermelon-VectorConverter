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

const SPLASH_ART: &[u8] = include_bytes!("../../assets/watermelon_launch_art.png");
const SPLASH_TICK: Duration = Duration::from_millis(16);
const SPLASH_DURATION: Duration = Duration::from_millis(1500);

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
    Splash,
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
    splash_elapsed: Duration,
    appearance: Appearance,
    direction: Direction,
    state: ConversionState,
    last_input: Option<InputFile>,
    notice: Option<String>,
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
                screen: Screen::Splash,
                splash_elapsed: Duration::ZERO,
                appearance: Appearance::Dark,
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
                self.splash_elapsed += SPLASH_TICK;
                if self.splash_elapsed >= SPLASH_DURATION {
                    self.screen = Screen::Converter;
                }
            }
            Message::SplashTick => {}
            Message::OpenAbout => {
                self.screen = Screen::About;
            }
            Message::CloseAbout => {
                self.screen = Screen::Converter;
            }
            Message::ToggleAppearance => {
                self.appearance = self.appearance.toggled();
            }
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
            Screen::About => self.about_view(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match self.screen {
            Screen::Splash => iced::time::every(SPLASH_TICK).map(|_| Message::SplashTick),
            Screen::Converter => iced::event::listen().map(Message::IcedEvent),
            Screen::About => Subscription::none(),
        }
    }

    fn splash_view(&self) -> Element<'_, Message> {
        let progress = (self.splash_elapsed.as_millis() as f32
            / SPLASH_DURATION.as_millis() as f32)
            .clamp(0.0, 1.0);
        // Cubic ease-out creates a quiet pop-in without animating a fabricated
        // conversion percentage; the artwork already contains its launch bar.
        let eased = 1.0 - (1.0 - progress).powi(3);
        let art_size = 310.0 + (48.0 * eased);
        let glow = 0.08 + (0.18 * eased);

        let art = container(
            image(image::Handle::from_bytes(SPLASH_ART))
                .width(Length::Fixed(art_size))
                .height(Length::Fixed(art_size))
                .content_fit(ContentFit::Contain),
        )
        .padding(14)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba8(42, 255, 86, glow))),
            border: Border::default().rounded(32.0),
            ..container::Style::default()
        });

        container(center(art))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style::default().background(Color::BLACK))
            .into()
    }

    fn workspace_view(&self) -> Element<'_, Message> {
        let p = self.palette();
        let svg_selected = self.direction == Direction::SvgToVectorDrawable;
        let xml_selected = self.direction == Direction::VectorDrawableToSvg;

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

        let header = row![
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
            button(text(self.appearance.label()).size(13))
                .on_press(Message::ToggleAppearance)
                .padding([8, 12])
                .style(move |_, _| secondary_button_style(p)),
        ]
        .align_y(iced::Alignment::Center)
        .padding([4, 0]);

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
        let p = self.palette();
        let content = column![
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
            progress_bar(0.0..=1.0, 0.72)
                .length(Length::Fixed(360.0))
                .girth(Length::Fixed(8.0))
                .style(move |_| progress_bar::Style {
                    background: Background::Color(p.surface_muted),
                    bar: Background::Color(p.primary_glow),
                    border: Border::default().rounded(8.0).width(1.0).color(p.border),
                }),
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
        &self,
        name: &'a str,
        direction: Direction,
        source_preview: Option<&'a image::Handle>,
        output_preview: Option<&'a image::Handle>,
        output_text: &'a str,
    ) -> Element<'a, Message> {
        let p = self.palette();
        let summary = row![
            text("✓").size(24).color(p.primary),
            column![
                text("CONVERSION COMPLETE").size(15).color(p.primary),
                text(name).size(18).color(p.on_surface),
                text(format!(
                    "{} · {} bytes",
                    direction.label(),
                    output_text.len()
                ))
                .size(13)
                .color(p.muted),
            ]
            .spacing(3),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center);

        let previews = row![
            preview_panel(direction.source_label(), source_preview, p),
            preview_panel(direction.output_label(), output_preview, p),
        ]
        .spacing(16)
        .width(Length::Fill);

        let output = column![
            row![
                text("OUTPUT").size(14).color(p.primary),
                space::horizontal(),
                button(text("Copy").size(14))
                    .on_press(Message::CopyOutput)
                    .padding([7, 12])
                    .style(move |_, _| secondary_button_style(p)),
            ]
            .align_y(iced::Alignment::Center),
            scrollable(text(output_text).size(13).color(p.on_surface)).height(Length::Fixed(190.0)),
        ]
        .spacing(10);

        let actions = row![
            button(text("Save As…").size(15))
                .on_press(Message::SaveRequested)
                .padding([11, 18])
                .style(move |_, _| primary_button_style(p)),
            button(text("New conversion").size(15))
                .on_press(Message::Reset)
                .padding([11, 18])
                .style(move |_, _| secondary_button_style(p)),
        ]
        .spacing(12);

        let content = column![summary, previews, output, actions]
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
                image(image::Handle::from_bytes(SPLASH_ART))
                    .width(Length::Fixed(106.0))
                    .height(Length::Fixed(106.0)),
            )
            .padding(14)
            .style(move |_| panel_style(p.surface, p.border)),
            text("Watermelon").size(34).color(p.on_background),
            text("Vector Graphics Converter").size(17).color(p.muted),
        ]
        .spacing(8)
        .align_x(iced::Alignment::Center);

        let ifem_signature = container(
            row![
                text("◇").size(38).color(p.primary),
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
            text("TECHNOLOGY STACK").size(12).color(p.muted),
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
            text("◆").size(14).color(p.watermelon_red),
            text("DEVELOPED BY").size(12).color(p.primary),
            text("Soheil Mozaffari").size(22).color(p.on_background),
            text("Software Engineer · Systems Architect")
                .size(14)
                .color(p.muted),
        ]
        .spacing(5)
        .align_x(iced::Alignment::Center);

        let doctrine = container(
            row![
                text("◇").size(30).color(p.primary),
                column![
                    text("Architected using IFEM Doctrine")
                        .size(15)
                        .color(p.primary),
                    text("Learn more about Interface-First Engineering Methodology")
                        .size(13)
                        .color(p.muted),
                    text("ifem.dev").size(13).color(p.primary),
                ]
                .spacing(3),
            ]
            .spacing(14)
            .align_y(iced::Alignment::Center),
        )
        .padding(16)
        .style(move |_| panel_style(p.surface, p.border));

        let content = column![
            header,
            hero,
            ifem_signature,
            badges,
            stack,
            developer,
            doctrine
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
    SplashTick,
    OpenAbout,
    CloseAbout,
    ToggleAppearance,
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
