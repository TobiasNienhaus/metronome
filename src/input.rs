use super::Message;
use crate::input::windows::WindowsInput;
use futures_channel::mpsc::UnboundedSender;
use log::error;
use once_cell::sync::OnceCell;

use super::util::SyncWrapper;

#[cfg(target_os = "windows")]
mod windows;

pub mod keycode;

type WrappedSender = SyncWrapper<UnboundedSender<Message>>;

static SENDER: OnceCell<WrappedSender> = OnceCell::new();

pub trait OsInput {
    fn new() -> Self;
    fn on_shutdown(&self);
}

pub fn init() -> impl OsInput {
    #[cfg(target_os = "windows")]
    {
        WindowsInput::new()
    }
}

pub fn add_sender(sender: UnboundedSender<Message>) {
    if let Err(_) = SENDER.set(WrappedSender::new(&sender)) {
        error!("Could not set sender!");
    }
}
