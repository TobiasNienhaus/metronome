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

use once_cell::sync::OnceCell;
use rand::RngCore;

mod input;
mod keyboard;
mod util;

use input::OsInput;

#[derive(Debug)]
struct Example {
    counter: isize,
    kb_worker: KbWorker,
    slider: iced::slider::State,
    scrollable_state: iced::scrollable::State,
    input_state1: iced::text_input::State,
    text1: String,
    input_state2: iced::text_input::State,
    text2: String,
    slider_value: f32,
    button: iced::button::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    Ready(UnboundedSender<Message>),
    KeyEvent(input::keycode::KeyCode),
    VolumeChanged(f32),
    Text1Changed(String),
    Text2Changed(String),
    DeletePressed,
}

impl Application for Example {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Example {
                counter: 0,
                kb_worker: KbWorker::new(),
                slider: iced::slider::State::new(),
                scrollable_state: iced::scrollable::State::new(),
                input_state1: iced::text_input::State::new(),
                text1: String::new(),
                input_state2: iced::text_input::State::new(),
                text2: String::new(),
                slider_value: 0.0,
                button: iced::button::State::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        format!("Counter - {}", self.counter)
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
            Message::Text1Changed(s) => {
                self.text1 = s;
            }
            Message::Text2Changed(s) => {
                self.text2 = s;
            }
            Message::DeletePressed => {
                println!("Hello World!");
            }
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
                            modifiers,
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
            .push(
                Row::new()
                    .height(Length::Units(30))
                    .push(
                        TextInput::new(
                            &mut self.input_state1,
                            "1",
                            &self.text1,
                            Message::Text1Changed,
                        )
                        .width(Length::FillPortion(45)),
                    )
                    .push(
                        TextInput::new(
                            &mut self.input_state2,
                            "2",
                            &self.text2,
                            Message::Text2Changed,
                        )
                        .width(Length::FillPortion(45)),
                    )
                    .push(
                        Button::new(&mut self.button, Text::new("Del"))
                            .width(Length::FillPortion(10))
                            .on_press(Message::DeletePressed),
                    )
                    .spacing(10),
            )
            .spacing(10);

        for i in 0..self.counter {
            scrollable = scrollable
                .push(Row::new().push(Text::new(format!("Text {}", i))))
                .height(Length::Units(30));
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
            UpArrow => self.counter += 1,
            DownArrow => self.counter -= 1,
            _ => {}
        }
    }
}

fn main() {
    simplelog::TermLogger::init(
        simplelog::LevelFilter::Trace,
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
