use std::{
    net::UdpSocket,
    slice::from_raw_parts,
    sync::{
        OnceLock,
        atomic::{AtomicU8, Ordering},
        mpsc::{Sender, channel},
    },
    time::{Duration, Instant},
};

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Input::KeyboardAndMouse::{
            GetKeyState, ReleaseCapture, SetCapture, SetFocus, VK_MENU, VK_NUMLOCK,
        },
        WindowsAndMessaging::{
            CallWindowProcW, DefWindowProcW, GWLP_WNDPROC, SetForegroundWindow, SetWindowLongPtrW,
            WM_ACTIVATE, WM_ACTIVATEAPP, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS, WM_MOUSEMOVE,
            WM_SETFOCUS, WM_SYSKEYDOWN, WM_SYSKEYUP,
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
pub fn restore_wnd_proc(hwnd: HWND) {
    unsafe {
        if let Some(Some(orig)) = ORIGINAL_WNDPROC {
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, orig as _);
            ORIGINAL_WNDPROC = None;
        } else {
            log::error!("Could not get the value for ORIGINAL_WNDPROC to restore it.");
        }
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

//Keep track of the numlock state and ALT_UP
//This is basically just a workaround for focus issues where windows
//sends "fake" numlock states all the time.
//Alt is needed because of some weird windows shenanigans
static NUMLOCK_STATE: AtomicU8 = AtomicU8::new(99);
static mut LAST_ALT_UP: Option<Instant> = None;

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
            WM_SYSKEYUP | WM_SYSKEYDOWN => {
                if wparam.0 == 0x90 {
                    return LRESULT(0);
                }
            }
            WM_KEYDOWN | WM_KEYUP => {
                if wparam.0 as u16 == VK_MENU.0 {
                    unsafe {
                        LAST_ALT_UP = Some(Instant::now());
                    }
                }
                //Numlock fix
                if wparam.0 == 0x90 {
                    if !synchronize_numlock() {
                        return LRESULT(0);
                    }
                }

                if msg == WM_KEYDOWN {
                    if let Some(map) = KEYBINDS.get() {
                        let combo = get_current_keybind(wparam.0 as u32);
                        if let Some(action) = map.get(&combo) {
                            action();
                            return LRESULT(0);
                        }
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

//Returns true if the numlock press should count, false if we should swallow it
fn synchronize_numlock() -> bool {
    //Swallow numlock if alt was pressed soon before
    if let Some(instant) = unsafe { LAST_ALT_UP } {
        if instant.elapsed() < Duration::from_millis(100) {
            return false;
        }
    }
    let numlock_state = unsafe { GetKeyState(VK_NUMLOCK.0 as i32) & 1 } as u8;
    if numlock_state != NUMLOCK_STATE.load(Ordering::Relaxed) {
        NUMLOCK_STATE.store(numlock_state, Ordering::Relaxed);
        return true;
    }
    return false;
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
    synchronize_numlock();
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
