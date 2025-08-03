use windows::Win32::UI::WindowsAndMessaging::WNDPROC;

pub static mut ORIGINAL_WNDPROC: Option<WNDPROC> = None;

//This socket is used to send input data to any overlay that
//cares to listen to this port.
pub const UDPADDR: &str = "127.0.0.1:49152";
