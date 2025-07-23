use std::{
    net::UdpSocket,
    sync::{Mutex, OnceLock},
};

use windows::Win32::UI::WindowsAndMessaging::WNDPROC;

pub static mut ORIGINAL_WNDPROC: Option<WNDPROC> = None;

//This socket is used to send input data to any overlay that
//cares to listen to this port.
pub static UDPSOCKET: OnceLock<Mutex<UdpSocket>> = OnceLock::new();
pub const UDPPORT: &str = "49152";
pub fn get_udp_socket_lock() -> &'static Mutex<UdpSocket> {
    UDPSOCKET.get_or_init(|| {
        Mutex::new(UdpSocket::bind("0.0.0.0:0").expect("Error creating udp socket."))
    })
}
