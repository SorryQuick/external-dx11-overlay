use std::sync::{Mutex, OnceLock};

use mmf::{SharedFrame, get_blank_shared_frame, start_frame_watcher_thread};
use rendering::{OverlayState, detoured_present};
use windows::{
    Win32::{Foundation::HANDLE, Graphics::Dxgi::IDXGISwapChain},
    core::HRESULT,
};

//Contains an entire frame. This frame will be the one rendered in the hooked present.
pub static FRAME_BUFFER: OnceLock<Mutex<SharedFrame>> = OnceLock::new();

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

//See C# for description
const HEADER_SIZE: usize = 12;

///Simple utility to verify if a given coordinate is over the overlay. Used for mouse input mostly.
pub fn is_overlay_pixel(x: u32, y: u32) -> bool {
    let frame_buf = FRAME_BUFFER.get_or_init(|| Mutex::new(get_blank_shared_frame()));
    //Safe but slow way with a LOCK
    let frame = frame_buf.lock().unwrap();
    if x >= frame.width || y >= frame.height {
        return false;
    }

    let index = ((y * frame.width + x) * 4 + 3) as usize;
    return unsafe { *frame.pixels.get_unchecked(index) } > 0;
}

pub fn get_detoured_present() -> impl Fn(IDXGISwapChain, u32, u32) -> HRESULT {
    detoured_present
}

pub fn startup_ui_rendering() {
    start_frame_watcher_thread();
}
