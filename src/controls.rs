use std::slice::from_raw_parts;

use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Controls::{WM_MOUSEHOVER, WM_MOUSELEAVE},
        Input::KeyboardAndMouse::{GetKeyState, ReleaseCapture, SetCapture, SetFocus, VK_CONTROL},
        WindowsAndMessaging::{
            CallWindowProcW, DefWindowProcW, GWLP_WNDPROC, SetForegroundWindow, SetWindowLongPtrW,
            WM_ACTIVATE, WM_ACTIVATEAPP, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP,
            WM_MOUSEMOVE, WM_NCHITTEST, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SETCURSOR, WM_SETFOCUS,
        },
    },
};

use crate::{
    debug::{dump_debug_data, restart_blish},
    globals::{self, ORIGINAL_WNDPROC, get_udp_socket_lock},
    ui::{self},
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

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    //TODO: Queue and thread. Locks, especially the one used in is_overlay_pixel, can slow down the
    //UI thread quite a bit
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

                let packet = MouseInputPacket { id, x, y };
                let data = unsafe {
                    from_raw_parts(
                        &packet as *const MouseInputPacket as *const u8,
                        size_of::<MouseInputPacket>(),
                    )
                };

                let sock_lock = get_udp_socket_lock();
                if let Ok(socket) = sock_lock.lock() {
                    socket
                        .send_to(&data, "127.0.0.1:".to_owned() + globals::UDPPORT)
                        .ok();
                }
                /*if is_overlay_pixel && msg == WM_LBUTTONDOWN && msg == WM_RBUTTONDOWN {
                    return LRESULT(0);
                }*/
            }
            /*WM_KEYDOWN | WM_KEYUP | WM_CHAR | WM_SYSKEYDOWN | WM_SYSKEYUP |  => {

            }*/
            WM_KEYDOWN => {
                if wparam.0 as u32 == 'P' as u32 {
                    if (unsafe { GetKeyState(VK_CONTROL.0 as i32) } as u16 & 0x8000) != 0 {
                        dump_debug_data();
                        return LRESULT(0);
                    }
                }
                if wparam.0 as u32 == 'O' as u32 {
                    if (unsafe { GetKeyState(VK_CONTROL.0 as i32) } as u16 & 0x8000) != 0 {
                        restart_blish();
                        return LRESULT(0);
                    }
                }
            }
            WM_SETFOCUS => grab_focus(hwnd),
            WM_KILLFOCUS => release_focus(hwnd),
            WM_ACTIVATEAPP | WM_ACTIVATE => {
                if wparam.0 != 0 {
                    grab_focus(hwnd);
                } else {
                    release_focus(hwnd);
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
        SetForegroundWindow(hwnd);
        SetFocus(hwnd);
        SetCapture(hwnd);
    }
}
fn release_focus(hwnd: HWND) {
    unsafe {
        ReleaseCapture();
    }
}
