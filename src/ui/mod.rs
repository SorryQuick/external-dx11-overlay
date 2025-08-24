use std::sync::{Mutex, OnceLock};

use mmf::MMFData;
use rendering::{OverlayState, detoured_present};
use windows::{
    Win32::{Foundation::HANDLE, Graphics::Dxgi::IDXGISwapChain},
    core::HRESULT,
};

pub static MMF_DATA: OnceLock<Mutex<MMFData>> = OnceLock::new();
pub static OVERLAY_STATE: OnceLock<Mutex<Option<OverlayState>>> = OnceLock::new();

mod mmf;
mod rendering;

//Handle to MMFs. Currently named "HEADER" because the previous version used a body as well
static SHARED_HANDLE_HEADER: OnceLock<Mutex<HANDLE>> = OnceLock::new();
static HEADER_NAME: &str = "BlishHUD_Header";

//See C# for description
const HEADER_SIZE: usize = 28;

pub fn get_detoured_present() -> impl Fn(IDXGISwapChain, u32, u32) -> HRESULT {
    detoured_present
}
