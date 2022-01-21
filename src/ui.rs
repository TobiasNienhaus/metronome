use iced::{
    Element, Row, TextInput, Length, Button, Text
};
use iced_native::Widget;
use log::error;
use rand::RngCore;

use super::id;
use super::Message;

#[derive(Debug, Clone)]
pub enum SongListingEvent {
    TitleChange(String),
    BpmChange(String)
}

#[derive(Debug)]
pub struct SongListing {
    id: u64,
    title: String,
    bpm: Option<u16>,
    title_input: iced::text_input::State,
    bpm_input: iced::text_input::State,
    button: iced::button::State,
}

impl SongListing {
    pub fn new(title: &str, bpm: u16) -> SongListing {
        SongListing {
            id: id::new(),
            title: String::from(title),
            bpm: Some(bpm),
            title_input: iced::text_input::State::new(),
            bpm_input: iced::text_input::State::new(),
            button: iced::button::State::new()
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, val: &str) {
        self.title = String::from(val);
    }

    pub fn bpm(&self) -> Option<u16> {
        self.bpm
    }

    pub fn set_bpm(&mut self, val: u16) {
        self.bpm = Some(val);
    }

    pub fn apply_event(&mut self, event: SongListingEvent) {
        match event {
            SongListingEvent::TitleChange(title) => self.title = title,
            SongListingEvent::BpmChange(bpm) => {
                if bpm.is_empty() {
                    self.bpm = None
                } else if let Ok(parsed_bpm) = bpm.parse() {
                    self.bpm = Some(parsed_bpm)
                }
            }
        }
    }

    pub fn element(&mut self, height: Length) -> Row<Message> {
        let id = self.id;
        Row::new()
            .push(
                TextInput::new(
                    &mut self.title_input,
                    "1",
                    &self.title,
                    move |v| {
                        error!("ID: {}", id);
                        Message::SongListingChanged(id, SongListingEvent::TitleChange(v))
                    }
                ).width(Length::FillPortion(45)),
            )
            .push(
                TextInput::new(
                    &mut self.bpm_input,
                    "BPM",
                    &*self.bpm.map_or(String::new(), |opt| { format!("{}", opt) }),
                    move |v| {
                        Message::SongListingChanged(id, SongListingEvent::BpmChange(v))
                    }
                ).width(Length::FillPortion(45)),
            )
            .push(
                Button::new(&mut self.button, Text::new("Del"))
                    .width(Length::FillPortion(10))
                    .on_press(Message::SongListingDeleted(self.id)),
            )
            .spacing(10)
            .height(height)
    }
}
