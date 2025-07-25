use std::sync::{Mutex, OnceLock};

use mmf::{SharedFrame, start_frame_watcher_thread};
use rendering::{OverlayState, detoured_present};
use windows::{
    Win32::{Foundation::HANDLE, Graphics::Dxgi::IDXGISwapChain},
    core::HRESULT,
};

//Contains an entire frame. This frame will be the one rendered in the hooked present.
static FRAME_BUFFER: OnceLock<Mutex<Option<SharedFrame>>> = OnceLock::new();

pub static OVERLAY_STATE: OnceLock<Mutex<Option<OverlayState>>> = OnceLock::new();

mod mmf;
mod rendering;

//Handle to MMFs
//The header is permanently mapped, while the body is mapped only when nessesary.
//TODO: Dynamic body
static SHARED_HANDLE_HEADER: OnceLock<Mutex<HANDLE>> = OnceLock::new();
static SHARED_HANDLE_BODY: OnceLock<Mutex<HANDLE>> = OnceLock::new();
static HEADER_NAME: &str = "BlishHUD_Header";
static BODY_NAME: &str = "BlishHUD_Body";

const HEADER_SIZE: usize = 8;

///Simple utility to verify if a given coordinate is over the overlay. Used for mouse input mostly.
///Pretty fast, but only if the FRAME_BUFFER is not locked. Only really checks if alpha > 0
pub fn is_overlay_pixel(x: u32, y: u32) -> bool {
    if let Some(frame_buf) = FRAME_BUFFER.get() {
        let guard = frame_buf.lock().unwrap();
        if let Some(ref frame) = *guard {
            if x >= frame.width || y >= frame.height {
                return false;
            }
            return frame
                .pixels
                .get(((y * frame.width + x) * 4 + 3) as usize)
                .map_or(false, |&alpha| alpha > 0);
        }
    }
    false
}

pub fn get_detoured_present() -> impl Fn(IDXGISwapChain, u32, u32) -> HRESULT {
    detoured_present
}

pub fn startup_ui_rendering() {
    start_frame_watcher_thread();
}
