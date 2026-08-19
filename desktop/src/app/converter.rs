// Watermelon Vector Converter — Desktop splash and converter shell.
// Copyright (c) 2026 Suhail Muzaffari. All rights reserved.

use iced::widget::{center, column, container, image, progress_bar, text};
use iced::{Background, Border, Color, Element, Length, Subscription, Task};
use std::time::Duration;

const SPLASH_ART: &[u8] = include_bytes!("../../../icons/png/512.png");
const SPLASH_TICK: Duration = Duration::from_millis(90);
const SPLASH_STEP: f32 = 4.0;

const BACKGROUND: Color = Color::from_rgb8(5, 12, 9);
const WATERMELON_RED: Color = Color::from_rgb8(230, 57, 70);
const RIND_GREEN: Color = Color::from_rgb8(64, 197, 86);
const GLOW_GREEN: Color = Color::from_rgb8(138, 235, 78);
const MUTED_GREEN: Color = Color::from_rgb8(113, 218, 122);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Splash,
    Converter,
}

pub struct Converter {
    screen: Screen,
    splash_progress: f32,
}

impl Converter {
    pub fn new() -> (Self, Task<Message>) {
        (
            Converter {
                screen: Screen::Splash,
                splash_progress: 0.0,
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
        }

        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.screen {
            Screen::Splash => self.splash_view(),
            Screen::Converter => center(text("Converter UI — coming in phase 2")).into(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.screen == Screen::Splash {
            iced::time::every(SPLASH_TICK).map(|_| Message::SplashTick)
        } else {
            Subscription::none()
        }
    }

    fn splash_view(&self) -> Element<'_, Message> {
        let status = if self.splash_progress >= 96.0 {
            "READY"
        } else {
            "LAUNCHING…"
        };

        let progress = progress_bar(0.0..=100.0, self.splash_progress)
            .length(Length::Fixed(280.0))
            .girth(Length::Fixed(11.0))
            .style(|_| progress_bar::Style {
                background: Background::Color(Color::from_rgb8(12, 35, 22)),
                bar: Background::Color(GLOW_GREEN),
                border: Border::default()
                    .rounded(8.0)
                    .width(1.0)
                    .color(RIND_GREEN),
            });

        let content = column![
            text("↑").size(74).color(GLOW_GREEN),
            text("▪  ▪  ▪").size(16).color(MUTED_GREEN),
            image(image::Handle::from_bytes(SPLASH_ART))
                .width(Length::Fixed(188.0))
                .height(Length::Fixed(188.0)),
            text("WATERMELON").size(32).color(Color::WHITE),
            text("VECTOR CONVERTER").size(17).color(RIND_GREEN),
            text(status).size(14).color(MUTED_GREEN),
            progress,
            text(format!("{}%", self.splash_progress.round() as u8))
                .size(12)
                .color(WATERMELON_RED),
        ]
        .spacing(10)
        .align_x(iced::Alignment::Center)
        .width(Length::Shrink);

        container(center(content))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(34)
            .style(|_| container::Style::default().background(BACKGROUND))
            .into()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    SplashTick,
}
