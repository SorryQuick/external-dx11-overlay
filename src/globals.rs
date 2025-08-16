use std::sync::{OnceLock, atomic::AtomicBool};

use windows::Win32::{Foundation::HANDLE, UI::WindowsAndMessaging::WNDPROC};

pub static mut ORIGINAL_WNDPROC: Option<WNDPROC> = None;

//Mutex used to check if blish is still alive, if it crashed, or if it simply not sending frames
//(eg if it hasn't changed)
pub static LIVE_MUTEX: OnceLock<Option<HANDLE>> = OnceLock::new();

//This socket is used to send input data to any overlay that
//cares to listen to this port.
pub const UDPADDR: &str = "127.0.0.1:49152";

//Anything related to debugging should be added here, then toggled with a keybind.
pub struct DebugFeatures {
    pub rendering_enabled: AtomicBool,
    pub processing_enabled: AtomicBool,
}

pub static DEBUG_FEATURES: DebugFeatures = DebugFeatures {
    rendering_enabled: AtomicBool::new(true),
    processing_enabled: AtomicBool::new(true),
};
