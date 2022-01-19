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
    Length, Row, Scrollable, Settings, Text, TextInput, VerticalAlignment,
};
use iced_futures::{futures, BoxStream};
use iced_native::subscription::Subscription;

use iced_native::event::Status;
use iced_native::{Column, Slider};
use log::error;
use std::process::exit;
use iced::futures::future::err;

use once_cell::sync::OnceCell;
use rand::RngCore;

mod input;
mod keyboard;
mod util;
mod ui;
mod id;

use input::OsInput;

#[derive(Debug)]
struct Example {
    kb_worker: KbWorker,
    slider: iced::slider::State,
    scrollable_state: iced::scrollable::State,
    slider_value: f32,
    songs: HashMap<u64, ui::SongListing>
}

#[derive(Debug, Clone)]
pub enum Message {
    Ready(UnboundedSender<Message>),
    KeyEvent(input::keycode::KeyCode),
    VolumeChanged(f32),
    SongListingChanged(u64, ui::SongListingEvent),
    SongListingDeleted(u64)
}

impl Application for Example {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Example {
                kb_worker: KbWorker::new(),
                slider: iced::slider::State::new(),
                scrollable_state: iced::scrollable::State::new(),
                slider_value: 0.0,
                songs: HashMap::new()
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
            }
            Message::SongListingChanged(id, event) => {
                if let Some(song) = self.songs.get_mut(&id) {
                    song.apply_event(event);
                }
            }
            Message::SongListingDeleted(id) => {
                self.songs.remove(&id);
            }
            _ => {}
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
        let tempo: Element<_> = Row::new()
            .push(
                Column::new()
                    .width(Length::FillPortion(70))
                    .push(
                        Text::new("AAA")
                            .height(Length::FillPortion(70))
                            .size(100)
                            .vertical_alignment(VerticalAlignment::Center),
                    )
                    .push(
                        Text::new("AAA")
                            .height(Length::FillPortion(30))
                            .size(30)
                            .vertical_alignment(VerticalAlignment::Center),
                    ),
            )
            .push(
                Column::new()
                    .width(Length::FillPortion(30))
                    .push(Text::new("").height(Length::FillPortion(70)))
                    .push(
                        Text::new("AAA")
                            .height(Length::FillPortion(30))
                            .size(30)
                            .horizontal_alignment(HorizontalAlignment::Right)
                            .vertical_alignment(VerticalAlignment::Center),
                    ),
            )
            .height(Length::FillPortion(30))
            .padding(10)
            .into();

        let mut scrollable = Scrollable::new(&mut self.scrollable_state)
            .width(Length::Fill)
            .spacing(10);

        for (_, song) in &mut self.songs {
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
                0.0..=100.0,
                self.slider_value,
                Message::VolumeChanged,
            ))
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
                self.songs.insert(song.id(), song);
            }
            _ => {}
        }
    }
}

fn main() {
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Error,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    );

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
