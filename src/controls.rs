use std::{
    net::UdpSocket,
    slice::from_raw_parts,
    sync::{
        OnceLock,
        mpsc::{Sender, channel},
    },
};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, SetFocus},
        WindowsAndMessaging::{
            CallWindowProcW, DefWindowProcW, GWLP_WNDPROC, SetForegroundWindow, SetWindowLongPtrW,
            WM_ACTIVATE, WM_ACTIVATEAPP, WM_KEYDOWN, WM_KILLFOCUS, WM_MOUSEMOVE, WM_SETFOCUS,
        },
    },
};

use crate::{
    globals::{self, ORIGINAL_WNDPROC},
    keybinds::{KEYBINDS, get_current_keybind},
};

pub fn initialize_controls(hwnd: HWND) {
    unsafe {
        let old_wndproc = SetWindowLongPtrW(hwnd, GWLP_WNDPROC, wnd_proc as _);
        ORIGINAL_WNDPROC = Some(std::mem::transmute(old_wndproc));
    }
}

fn get_x_lparam(lparam: LPARAM) -> i32 {
    let lparam_u32 = lparam.0 as u32;
    let x = (lparam_u32 & 0xFFFF) as i16;
    x as i32
}

fn get_y_lparam(lparam: LPARAM) -> i32 {
    let lparam_u32 = lparam.0 as u32;
    let y = ((lparam_u32 >> 16) & 0xFFFF) as i16;
    y as i32
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct MouseInputPacket {
    id: u8,
    x: i32,
    y: i32,
}

//Unsafe way to send packets over to a thread.
//It's 100% safe as long as:
//- Thread is initialized before the first call
//- Sender is only used in wnd_proc
#[derive(Debug)]
struct StaticSender {
    sender: *const Sender<MouseInputPacket>,
}
unsafe impl Sync for StaticSender {}
unsafe impl Send for StaticSender {}
static MOUSE_SENDER: OnceLock<StaticSender> = OnceLock::new();

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    'local_handling: {
        match msg {
            //Mouse
            WM_MOUSEMOVE => {
                let x = get_x_lparam(lparam);
                let y = get_y_lparam(lparam);

                //let is_overlay_pixel = ui::is_overlay_pixel(x as u32, y as u32);

                //Mouse up/down are seemingly handled globally.
                //So we only need to pass MOUSEMOVE.
                let id = match msg {
                    /*WM_LBUTTONDOWN => 0,
                    WM_LBUTTONUP => 1,*/
                    WM_MOUSEMOVE => 2,
                    /*WM_RBUTTONDOWN => 3,
                    WM_RBUTTONUP => 4,*/
                    _ => {
                        break 'local_handling;
                    }
                };

                //Send packet to listening thread.
                let packet = MouseInputPacket { id, x, y };
                let sender = unsafe { &*MOUSE_SENDER.get().unwrap().sender };
                sender.send(packet).ok();
                /*if is_overlay_pixel && msg == WM_LBUTTONDOWN && msg == WM_RBUTTONDOWN {
                    return LRESULT(0);
                }*/
            }
            WM_KEYDOWN => {
                if let Some(map) = KEYBINDS.get() {
                    let combo = get_current_keybind(wparam.0 as u32);
                    if let Some(action) = map.get(&combo) {
                        action();
                        return LRESULT(0);
                    }
                }
            }
            WM_SETFOCUS => grab_focus(hwnd),
            WM_KILLFOCUS => release_focus(),
            WM_ACTIVATEAPP | WM_ACTIVATE => {
                if wparam.0 != 0 {
                    grab_focus(hwnd);
                } else {
                    release_focus();
                }
            }
            _ => {}
        }
    }
    unsafe {
        if let Some(original) = ORIGINAL_WNDPROC {
            CallWindowProcW(original, hwnd, msg, wparam, lparam)
        } else {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
}

fn grab_focus(hwnd: HWND) {
    unsafe {
        SetForegroundWindow(hwnd).ok().ok();
        SetFocus(hwnd);
        SetCapture(hwnd);
    }
}
fn release_focus() {
    unsafe {
        ReleaseCapture().ok();
    }
}

pub fn start_mouse_input_thread() {
    let (tx, rx) = channel::<MouseInputPacket>();

    MOUSE_SENDER
        .set(StaticSender {
            sender: Box::into_raw(Box::new(tx)),
        })
        .unwrap();

    std::thread::spawn(move || {
        let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind socket");
        for packet in rx {
            let data = unsafe {
                from_raw_parts(
                    &packet as *const MouseInputPacket as *const u8,
                    size_of::<MouseInputPacket>(),
                )
            };
            socket.send_to(data, globals::UDPADDR).ok();
        }
    });
}
