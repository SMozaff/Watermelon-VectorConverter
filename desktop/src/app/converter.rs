// Placeholder — Phase 2 of the desktop rewrite. Not yet implemented.
use iced::{Element, Subscription, Task};

pub struct Converter;

impl Converter {
    pub fn new() -> (Self, Task<Message>) {
        (Converter, Task::none())
    }
    pub fn update(&mut self, _message: Message) -> Task<Message> {
        Task::none()
    }
    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::center(iced::widget::text("Converter UI — coming in phase 2")).into()
    }
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }
}

#[derive(Debug, Clone)]
pub enum Message {}
