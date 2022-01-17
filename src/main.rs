use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::process::exit;
use std::sync::{Arc, Mutex};

mod keyboard;

use winapi::um::winuser::{WH_KEYBOARD_LL, SetWindowsHookExA, UnhookWindowsHookEx, CallNextHookEx, WM_KEYDOWN, LPKBDLLHOOKSTRUCT};

use rust_win32error::*;

use iced::{button, executor, Align, Application, Button, Column, Command, Container, Element, Length, ProgressBar, Settings, Subscription, Text, Clipboard, Color, HorizontalAlignment, VerticalAlignment};
use iced::window::Mode;
use crate::keyboard::KeyCode;

use std::sync::mpsc::{channel, Receiver, Sender};
use std::task::{Context, Poll};
use futures_channel::mpsc::{unbounded, UnboundedSender, UnboundedReceiver, TryRecvError};
use iced_futures::{BoxStream, futures};
use rand::{Rng, RngCore};

use once_cell::sync::OnceCell;
use crate::futures::StreamExt;

static SENDER: OnceCell<SyncWrapper> = OnceCell::new();

#[derive(Debug)]
struct SyncWrapper {
    sender: UnboundedSender<Message>
}

unsafe impl Sync for SyncWrapper {}

impl SyncWrapper {
    fn get(&self) -> UnboundedSender<Message> {
        self.sender.clone()
    }

    fn new(sender: UnboundedSender<Message>) -> SyncWrapper {
        SyncWrapper {
            sender
        }
    }
}

#[derive(Debug)]
enum Example {
    Counter(isize)
}

#[derive(Debug, Clone)]
enum Message {
    IncreaseCounter
}

impl Application for Example {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Example::Counter(0),
            Command::none()
        )
    }

    fn title(&self) -> String {
        if let Self::Counter(num) = self {
            format!("Counter - {}", num)
        } else {
            String::from("ERROR")
        }
    }

    fn update(&mut self, message: Self::Message, _: &mut Clipboard) -> Command<Self::Message> {
        let val = if let Self::Counter(num) = self {
            *num
        } else {
            0
        };
        match message {
            Message::IncreaseCounter => {
                *self = Self::Counter(val + 1);
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let number: Element<_> = match self {
            Example::Counter(num) => {
                Text::new(num.to_string())
                    .horizontal_alignment(HorizontalAlignment::Center)
                    .vertical_alignment(VerticalAlignment::Center)
                    .size(100)
                    .into()
            }
        };

        Container::new(number)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

fn main() {
    let (tx, rx) = unbounded();

    SENDER.set(SyncWrapper::new(tx.clone())).unwrap();

    let hook_id = unsafe { SetWindowsHookExA(WH_KEYBOARD_LL, Some(hook_callback), std::ptr::null_mut(), 0) };

    if hook_id == std::ptr::null_mut() {
        eprintln!("Could not create Hook -> {}", Win32Error::new());
        exit(2);
    } else {
        println!("{:?}", hook_id);
    }

    Example::run(Settings::default());

    unsafe { UnhookWindowsHookEx(hook_id); }
}

unsafe extern "system" fn hook_callback(code: i32, wParam: usize, lParam: isize) -> isize {
    if code < 0 {
        return CallNextHookEx(std::ptr::null_mut(), code, wParam, lParam);
    } else {

        if wParam == WM_KEYDOWN as usize {
            let s = unsafe { *(lParam as LPKBDLLHOOKSTRUCT) };
            println!("{:?}", KeyCode::from(s.vkCode));
            if KeyCode::from(s.vkCode) == KeyCode::F12 {
                println!("F12 pressed");
                if let Some(sender) = SENDER.get() {
                    sender.get().unbounded_send(Message::IncreaseCounter);
                }
            }
        }
        0
    }
}

struct InputReceiver {
    rx: UnboundedReceiver<Message>
}

impl InputReceiver {
    fn new(rx: UnboundedReceiver<Message>) -> InputReceiver {
        InputReceiver {
            rx
        }
    }
}

impl iced::futures::Stream for InputReceiver {
    type Item = Message;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Ok(a) = self.rx.try_next() {
            Poll::from(a)
        } else {
            Poll::from(None)
        }
    }
}

impl<H: std::hash::Hasher, I> iced_native::subscription::Recipe<H, I> for InputReceiver {
    type Output = Message;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;

        std::any::TypeId::of::<Self>().hash(state);
        let num = rand::thread_rng().next_u64();
        num.hash(state);
    }

    fn stream(mut self: Box<Self>, _: BoxStream<I>) -> BoxStream<Self::Output> {
        // let mut rx = self.rx.take().unwrap();
        Box::pin(self)
    }
}
