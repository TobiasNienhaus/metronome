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

// Somehow have this only work, if Option is not none
// Also only serialize the inner values, so first number than file
enum BpmSetting {
    Value(Option<u16>),
    File(String)
}

#[derive(Debug)]
pub struct SongListing {
    title: String,
    bpm: Option<u16>,
    title_input: iced::text_input::State,
    bpm_input: iced::text_input::State,
    button: iced::button::State,
}

impl SongListing {
    pub fn new(title: &str, bpm: u16) -> SongListing {
        SongListing {
            title: String::from(title),
            bpm: Some(bpm),
            title_input: iced::text_input::State::new(),
            bpm_input: iced::text_input::State::new(),
            button: iced::button::State::new()
        }
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

    pub fn bpm_str(&self, default: &str) -> String {
        self.bpm.map_or(String::from(default), |opt| { format!("{}", opt) })
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

    pub fn editable_element(&mut self, height: Length) -> Row<Message> {
        Row::new()
            .push(
                TextInput::new(
                    &mut self.title_input,
                    "1",
                    &self.title,
                    move |v| { Message::None }
                ).width(Length::FillPortion(45)),
            )
            .push(
                TextInput::new(
                    &mut self.bpm_input,
                    "BPM",
                    &*self.bpm.map_or(String::new(), |opt| { format!("{}", opt) }),
                    move |v| { Message::None }
                ).width(Length::FillPortion(45)),
            )
            .push(
                Button::new(&mut self.button, Text::new("Del"))
                    .width(Length::FillPortion(10))
                    .on_press(Message::None),
            )
            .spacing(10)
            .height(height)
    }

    pub fn element(&self, height: Length) -> Row<Message> {
        Row::new()
            .push(Text::new(&self.title).width(Length::FillPortion(50)),
            )
            .push(Text::new(format!("{} BPM", self.bpm.unwrap())).width(Length::FillPortion(50)),
            )
            .spacing(10)
            .height(height)
    }
}
