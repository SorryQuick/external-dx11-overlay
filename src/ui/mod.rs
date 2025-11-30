use std::sync::{Arc, Mutex, OnceLock, RwLock, atomic::AtomicBool};

use mmf::MMFData;
use rendering::{OverlayState, detoured_present};
use windows::{Win32::Graphics::Dxgi::IDXGISwapChain, core::HRESULT};

pub static MMF_DATA: OnceLock<Arc<RwLock<MMFData>>> = OnceLock::new();
pub static OVERLAY_STATE: OnceLock<Mutex<Option<OverlayState>>> = OnceLock::new();

pub mod mmf;
mod rendering;

//Handle to MMFs. Currently named "HEADER" because the previous version used a body as well
static HEADER_NAME: &str = "BlishHUD_Header";

//See C# for description
const HEADER_SIZE: usize = 28;

pub static UPDATE_SCHEDULED: AtomicBool = AtomicBool::new(false);

pub fn get_detoured_present() -> impl Fn(IDXGISwapChain, u32, u32) -> HRESULT {
    detoured_present
}
