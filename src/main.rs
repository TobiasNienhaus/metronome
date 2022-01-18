use winapi::um::winuser::{
    CallNextHookEx, SetWindowsHookExA, UnhookWindowsHookEx, LPKBDLLHOOKSTRUCT, WH_KEYBOARD_LL,
    WM_KEYDOWN,
};
use rust_win32error::*;

// TODO https://www.hackster.io/HiAmadeus/analog-inputs-on-windows-10-raspberry-pi-using-adc-493ab9

use iced::{executor, Application, Clipboard, Command, Container, Element, HorizontalAlignment, Length, Settings, Text, VerticalAlignment, Row, Scrollable, TextInput, Button};
use iced_futures::{futures, BoxStream};
use iced_native::subscription::Subscription;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};

use std::process::exit;
use iced_native::{Column, Slider};
use iced_native::event::Status;

use once_cell::sync::OnceCell;
use rand::RngCore;

mod keyboard;
use keyboard::KeyCode;

static SENDER: OnceCell<SyncWrapper> = OnceCell::new();

#[derive(Debug)]
struct SyncWrapper {
    sender: UnboundedSender<Message>,
}

unsafe impl Sync for SyncWrapper {}

impl SyncWrapper {
    fn get(&self) -> UnboundedSender<Message> {
        self.sender.clone()
    }

    fn new(sender: UnboundedSender<Message>) -> SyncWrapper {
        SyncWrapper { sender }
    }
}

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
    button: iced::button::State
}

#[derive(Debug, Clone)]
enum Message {
    Ready(UnboundedSender<Message>),
    KeyEvent(keyboard::KeyCode),
    VolumeChanged(f32),
    Text1Changed(String),
    Text2Changed(String),
    DeletePressed
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
                button: iced::button::State::new()
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
                if let Err(_) = SENDER.set(SyncWrapper::new(sender)) {
                    eprintln!("Could not set sender!");
                    exit(4)
                }
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
            iced_native::subscription::events_with(|event, status| {
                match status {
                    Status::Ignored => {
                        if let iced_native::Event::Keyboard(iced_native::keyboard::Event::KeyPressed { key_code, modifiers }) = event {
                            Some(key_code)
                        } else {
                            None
                        }
                    }
                    Status::Captured => None
                }
            }).map(|e| Message::KeyEvent(keyboard::KeyCode::from(e)))
        ])
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let tempo: Element<_> = Row::new()
            .push(Column::new()
                .width(Length::FillPortion(70))
                .push(Text::new("AAA")
                    .height(Length::FillPortion(70))
                    .size(100)
                    .vertical_alignment(VerticalAlignment::Center)
                )
                .push(Text::new("AAA")
                    .height(Length::FillPortion(30))
                    .size(30)
                    .vertical_alignment(VerticalAlignment::Center)
                )
            )
            .push(Column::new()
                .width(Length::FillPortion(30))
                .push(Text::new("").height(Length::FillPortion(70)))
                .push(Text::new("AAA")
                    .height(Length::FillPortion(30))
                    .size(30)
                    .horizontal_alignment(HorizontalAlignment::Right)
                    .vertical_alignment(VerticalAlignment::Center)
                )
            )
            .height(Length::FillPortion(30))
            .padding(10)
            .into();

        let mut scrollable = Scrollable::new(&mut self.scrollable_state)
            .width(Length::Fill)
            .push(Row::new()
                .height(Length::Units(30))
                .push(TextInput::new(&mut self.input_state1, "1", &self.text1, Message::Text1Changed)
                    .width(Length::FillPortion(45))
                )
                .push(TextInput::new(&mut self.input_state2, "2", &self.text2, Message::Text2Changed)
                    .width(Length::FillPortion(45))
                )
                .push(Button::new(&mut self.button, Text::new("Del"))
                    .width(Length::FillPortion(10))
                    .on_press(Message::DeletePressed)
                )
                .spacing(10))
            .spacing(10);

        for i in 0..100 {
            scrollable = scrollable.push(Row::new().push(
                Text::new(format!("Text {}", i)))
            )
            .height(Length::Units(30));
        }

        let grid: Element<_>  = Row::new()
            .height(Length::FillPortion(60))
            .push(
                scrollable
            )
            .padding(10).into();

        let volume: Element<_> = Row::new()
            .padding(10)
            .push(Slider::new(&mut self.slider, 0.0..=100.0, self.slider_value, Message::VolumeChanged))
            .height(Length::FillPortion(10))
            .into();

        let combined: Element<_> = Column::new()
            .push(tempo)
            .push(grid)
            .push(volume)
            .into();

        Container::new(combined)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

impl Example {
    fn handle_keystroke(&mut self, code: keyboard::KeyCode) {
        use keyboard::KeyCode::*;
        match code {
            A => self.counter += 1,
            _ => {}
        }
    }
}

fn main() {
    let hook_id =
        unsafe { SetWindowsHookExA(WH_KEYBOARD_LL, Some(hook_callback), std::ptr::null_mut(), 0) };

    if hook_id == std::ptr::null_mut() {
        eprintln!("Could not create Hook -> {}", Win32Error::new());
        exit(2);
    }

    let mut settings = Settings::default();
    settings.window = Default::default();
    settings.window.min_size = Some((600, 400));

    if let Err(e) = Example::run(settings) {
        eprintln!("Application failed! ({:?})", e);
    }

    unsafe {
        UnhookWindowsHookEx(hook_id);
    }
}

unsafe extern "system" fn hook_callback(code: i32, w_param: usize, l_param: isize) -> isize {
    if w_param == WM_KEYDOWN as usize {
        let s = *(l_param as LPKBDLLHOOKSTRUCT);
        if let Some(sender) = SENDER.get() {
            if let Err(e) = sender.get().unbounded_send(Message::KeyEvent(KeyCode::from(s.vkCode))) {
                eprintln!("Failed to send Message! ({:?})", e);
            }
        }
    }
    return CallNextHookEx(std::ptr::null_mut(), code, w_param, l_param);
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
