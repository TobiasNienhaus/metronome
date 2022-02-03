use std::collections::HashMap;

use rust_win32error::*;
use winapi::um::winuser::{
    CallNextHookEx, SetWindowsHookExA, UnhookWindowsHookEx, LPKBDLLHOOKSTRUCT, WH_KEYBOARD_LL,
    WM_KEYDOWN,
};

// TODO https://www.hackster.io/HiAmadeus/analog-inputs-on-windows-10-raspberry-pi-using-adc-493ab9

use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use iced::{
    executor, Application, Button, Clipboard, Command, Container, Element, HorizontalAlignment,
    Length, Row, Scrollable, Settings, Text, TextInput, VerticalAlignment, Space
};
use iced_futures::{futures, BoxStream};
use iced_native::subscription::Subscription;

use iced_native::event::Status;
use iced_native::{Align, Column, Slider};
use log::{debug, error};
use std::process::exit;
use iced::futures::future::err;

use once_cell::sync::OnceCell;
use rand::{Rng, RngCore};

use serde_yaml;

mod input;
mod keyboard;
mod util;
mod ui;
mod id;
mod audio;
mod song_listing;

use input::OsInput;
use audio::AudioMessage;
use crate::audio::AudioHandle;
use crate::song_listing::FileSongListing;

#[derive(Debug)]
struct Example {
    kb_worker: KbWorker,
    audio_handle: AudioHandle,
    slider: iced::slider::State,
    scrollable_state: iced::scrollable::State,
    slider_value: f32,
    songs: Vec<ui::SongListing>,
    current: usize,
    play_button: iced::button::State,
    pause_button: iced::button::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    Ready(UnboundedSender<Message>),
    KeyEvent(input::keycode::KeyCode),
    VolumeChanged(f32),
    AudioMessage(audio::AudioMessage),
    None
}

impl Application for Example {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut songs = Vec::new();

        for i in 0..30 {
            songs.push(
                ui::SongListing::new(
                    &format!("Song {}", i),
                    rand::thread_rng().gen_range(50..200)
                )
            )
        }

        (
            Example {
                kb_worker: KbWorker::new(),
                audio_handle: audio::setup(),
                slider: iced::slider::State::new(),
                scrollable_state: iced::scrollable::State::new(),
                slider_value: 0.0,
                songs,
                current: 0,
                play_button: iced::button::State::new(),
                pause_button: iced::button::State::new()
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        format!("Metronome")
    }

    fn update(&mut self, message: Self::Message, _: &mut Clipboard) -> Command<Self::Message> {
        match message {
            Message::Ready(sender) => {
                input::add_sender(sender);
            }
            Message::KeyEvent(code) => {
                self.handle_keystroke(code);
            }
            Message::VolumeChanged(vol) => {
                self.slider_value = vol;
                self.audio_handle.send(AudioMessage::SetVolume(vol as u16))
            }
            Message::AudioMessage(msg) => {
                self.audio_handle.send(msg)
            }
            Message::None => {}
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch([
            self.kb_worker.subscription(),
            iced_native::subscription::events_with(|event, status| match status {
                Status::Ignored => {
                    if let iced_native::Event::Keyboard(
                        iced_native::keyboard::Event::KeyPressed {
                            key_code,
                            modifiers: _,
                        },
                    ) = event
                    {
                        Some(key_code)
                    } else {
                        None
                    }
                }
                Status::Captured => None,
            })
            .map(|e| Message::KeyEvent(input::keycode::KeyCode::from(e))),
        ])
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let mut tempo = Row::new()
            .height(Length::Units(170))
            .padding(20);

        if let Some(song) = self.songs.get(self.current) {
            tempo = tempo.push(Column::new()
                .width(Length::FillPortion(70))
                .push(
                    Text::new(song.bpm_str(""))
                        .size(100),
                )
                .push(
                    Text::new(song.title())
                        .size(30),
                )
            );
        } else {
            tempo = tempo.push(Column::new().width(Length::FillPortion(70)));
        }

        if let Some(song) = self.songs.get(self.current + 1) {
            tempo = tempo.push(Column::new()
               .width(Length::FillPortion(30))
                .push(
                    Space::with_height(Length::Units(60))
                )
               .push(
                   Text::new(song.bpm_str(""))
                       .height(Length::FillPortion(70))
                       .horizontal_alignment(HorizontalAlignment::Right)
                       .size(60)
               )
               .push(
                   Text::new(song.title())
                       .height(Length::FillPortion(30))
                       .horizontal_alignment(HorizontalAlignment::Right)
                       .size(20)
               )
            );
        } else {
            tempo = tempo.push(Column::new().width(Length::FillPortion(30)));
        }

        let tempo: Element<_> = tempo.into();

        let mut scrollable = Scrollable::new(&mut self.scrollable_state)
            .width(Length::Fill)
            .spacing(10);

        for song in self.songs.iter().skip(self.current + 1) {
            scrollable = scrollable.push(song.element(Length::Units(30)));
        }

        let grid: Element<_> = Row::new()
            .height(Length::FillPortion(60))
            .push(scrollable)
            .padding(10)
            .into();

        let volume: Element<_> = Row::new()
            .padding(10)
            .push(Slider::new(
                &mut self.slider,
                0.0..=1000.0,
                self.slider_value,
                Message::VolumeChanged)
                .width(Length::FillPortion(80))
            )
            .push(Button::new(&mut self.play_button, Text::new("Play"))
                .on_press(Message::AudioMessage(audio::AudioMessage::Play))
                .width(Length::FillPortion(10))
            )
            .push(Button::new(&mut self.pause_button, Text::new("Pause"))
                .on_press(Message::AudioMessage(audio::AudioMessage::Pause))
                .width(Length::FillPortion(10))
            )
            .spacing(10)
            .height(Length::FillPortion(10))
            .into();

        let combined: Element<_> = Column::new().push(tempo).push(grid).push(volume).into();

        Container::new(combined)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

impl Example {
    fn handle_keystroke(&mut self, code: input::keycode::KeyCode) {
        use input::keycode::KeyCode::*;
        match code {
            UpArrow => {
                let song = ui::SongListing::new("Title", 128);
                self.songs.push(song);
            }
            LeftArrow => {
                self.current = self.current.saturating_sub(1);
                self.apply_current();
            }
            RightArrow => {
                if self.current < self.songs.len() - 1 {
                    self.current += 1;
                    self.apply_current();
                }
            }
            SpaceBar => {
                self.audio_handle.send(AudioMessage::Toggle)
            }
            _ => {}
        }
    }

    fn apply_current(&mut self) {
        self.audio_handle.send(AudioMessage::SetBpm(self.songs.get(self.current).unwrap().bpm().unwrap()))
    }
}

pub const fn bpm_to_ns(bpm: u128) -> u128 {
    (60000 * 1000000) / bpm
}

fn main() {
    fern::Dispatch::new()
        .filter(|metadata| {
            metadata.target().starts_with("metronome")
        })
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{}] [{} -> {}:{}] [{}]\n-> {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.module_path().or_else(|| { Some("unknown") }).unwrap(),
                record.file().or_else(|| { Some("unknown") }).unwrap(),
                record.line().map_or(String::from("unknown"), |v| { v.to_string() }),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .apply().unwrap();

    let mut songs = Vec::new();
    for _ in 0..100 {
        songs.push(song_listing::FileSongListing::random());
    }
    let yaml = serde_yaml::to_string(&songs).unwrap();
    println!("{}", yaml);
    let songs2: Vec<song_listing::FileSongListing> = serde_yaml::from_str(&yaml).unwrap();
    for song in songs2 {
        println!("{}", song.title());
    }
    return;

    let input_handler = input::init();

    let mut settings = Settings::default();
    settings.window = Default::default();
    settings.window.min_size = Some((600, 400));

    if let Err(e) = Example::run(settings) {
        error!("Application failed! ({:?})", e);
    }

    input_handler.on_shutdown();
}

enum State {
    Starting,
    Ready(UnboundedReceiver<Message>),
}

#[derive(Debug)]
struct KbWorker {
    internal_hash: u64,
}

impl KbWorker {
    pub fn new() -> KbWorker {
        KbWorker {
            internal_hash: rand::thread_rng().next_u64(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::Subscription::from_recipe(KbWorker {
            internal_hash: self.internal_hash,
        })
    }
}

impl<H, I> iced_native::subscription::Recipe<H, I> for KbWorker
where
    H: std::hash::Hasher,
{
    type Output = Message;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;

        std::any::TypeId::of::<Self>().hash(state);
        self.internal_hash.hash(state);
    }

    fn stream(self: Box<Self>, _: BoxStream<I>) -> BoxStream<Self::Output> {
        Box::pin(futures::stream::unfold(
            State::Starting,
            |state| async move {
                match state {
                    State::Starting => {
                        let (sender, receiver) = unbounded();

                        Some((Message::Ready(sender), State::Ready(receiver)))
                    }
                    State::Ready(mut receiver) => {
                        use iced_native::futures::StreamExt;

                        let input = receiver.select_next_some().await;

                        Some((input, State::Ready(receiver)))
                    }
                }
            },
        ))
    }
}
