use futures_channel::mpsc::UnboundedSender;
use log::{error, trace};
use once_cell::sync::OnceCell;
use rust_win32error::Win32Error;
use std::process::exit;
use winapi::shared::windef::HHOOK;
use winapi::um::winuser::{
    CallNextHookEx, SetWindowsHookExA, UnhookWindowsHookEx, LPKBDLLHOOKSTRUCT, WH_KEYBOARD_LL,
    WM_KEYDOWN,
};

use super::keycode::KeyCode;
use super::Message;
use super::OsInput;

pub(super) struct WindowsInput {
    hook_id: HHOOK,
}

impl OsInput for WindowsInput {
    fn new() -> WindowsInput {
        let hook_id = unsafe {
            SetWindowsHookExA(WH_KEYBOARD_LL, Some(hook_callback), std::ptr::null_mut(), 0)
        };

        if hook_id == std::ptr::null_mut() {
            error!("Could not create Hook -> {}", Win32Error::new());
            exit(2);
        } else {
            trace!("Created hook -> {:?}", hook_id);
        }

        WindowsInput { hook_id }
    }

    fn on_shutdown(&self) {
        unsafe {
            UnhookWindowsHookEx(self.hook_id);
        }
    }
}

unsafe extern "system" fn hook_callback(code: i32, w_param: usize, l_param: isize) -> isize {
    if w_param == WM_KEYDOWN as usize {
        let s = *(l_param as LPKBDLLHOOKSTRUCT);
        if let Some(sender) = super::SENDER.get() {
            if let Err(e) = sender
                .get()
                .unbounded_send(Message::KeyEvent(KeyCode::from(s.vkCode)))
            {
                eprintln!("Failed to send Message! ({:?})", e);
            }
        }
    }
    return CallNextHookEx(std::ptr::null_mut(), code, w_param, l_param);
}
