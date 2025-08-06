use std::{
    ffi::OsStr, os::windows::ffi::OsStrExt, ptr::write_unaligned, slice::from_raw_parts,
    sync::Mutex,
};

use crate::globals;
use windows::{
    Win32::{
        Foundation::{BOOL, CloseHandle, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::{
            Memory::{
                FILE_MAP_ALL_ACCESS, MEM_COMMIT, MEMORY_BASIC_INFORMATION,
                MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile, OpenFileMappingW, PAGE_NOACCESS,
                UnmapViewOfFile, VirtualQuery,
            },
            Threading::{
                self, CreateEventW, OpenMutexW, ResetEvent, SetEvent, WaitForSingleObject,
            },
        },
    },
    core::PCWSTR,
};

use super::{
    BODY_NAME, FRAME_BUFFER, HEADER_NAME, HEADER_SIZE, SHARED_HANDLE_BODY, SHARED_HANDLE_HEADER,
};

#[derive(Debug)]
pub struct SharedFrame {
    pub width: u32,
    pub height: u32,
    pub hold: bool,
    pub pixels: Vec<u8>, // RGBA values in pairs of 4 bytes.
}

pub fn get_blank_shared_frame() -> SharedFrame {
    let sf = SharedFrame {
        width: 0,
        height: 0,
        hold: false,
        pixels: Vec::new(),
    };
    sf
}

///This thread listens for messages from Blish. It updates the FRAME_BUFFER accordingly.
pub fn start_frame_watcher_thread() {
    FRAME_BUFFER
        .set(Mutex::new(get_blank_shared_frame()))
        .unwrap();

    std::thread::spawn(move || {
        let mut header: Option<MEMORY_MAPPED_VIEW_ADDRESS> = None;
        let mut body: Option<MEMORY_MAPPED_VIEW_ADDRESS> = None;
        let mut body_handle: Option<HANDLE> = None;
        let (frame_ready, frame_consumed) = init_events();
        loop {
            if header.is_none() {
                header = open_header_mmf().ok();
            }
            if let Some(header_ptr) = header {
                if is_header_valid(header_ptr) {
                    if !wait_for_frame_ready(&frame_ready) {
                        //Blish is closed. Cleanup.
                        log::info!("BlishHUD was (potentially) closed. Cleanup.");
                        if let Some(header) = header.take() {
                            make_header_invalid(header);
                            unsafe {
                                UnmapViewOfFile(header).ok();
                            }
                        }
                        if let Some(body) = body.take() {
                            unsafe {
                                UnmapViewOfFile(body).ok();
                            }
                        }

                        if let Some(handle) = SHARED_HANDLE_BODY.get() {
                            if let Ok(mut lock) = handle.lock() {
                                if lock.0 != 0 {
                                    *lock = HANDLE(0);
                                }
                            }
                        }
                        if let Some(handle) = SHARED_HANDLE_HEADER.get() {
                            if let Ok(mut lock) = handle.lock() {
                                if lock.0 != 0 {
                                    *lock = HANDLE(0);
                                }
                            }
                        }
                        if let Some(frame_buf) = FRAME_BUFFER.get() {
                            let mut lock = frame_buf.lock().unwrap();
                            if lock.width != 0 || lock.height != 0 || !lock.pixels.is_empty() {
                                lock.width = 0;
                                lock.height = 0;
                                lock.hold = false;
                                lock.pixels.clear();
                            }
                        }
                        if let Some(h) = body_handle {
                            if !h.is_invalid() && h.0 != 0 {
                                unsafe {
                                    CloseHandle(h).ok();
                                }
                            }
                        }
                        body_handle = None;
                        continue;
                    }

                    try_read_shared_memory(
                        header_ptr,
                        &frame_ready,
                        &frame_consumed,
                        &mut body,
                        &mut body_handle,
                    );
                } else {
                    make_header_invalid(header_ptr);
                    header = None;
                    //If Blish is not running, don't bother running this thread too often.
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
            } else {
                //If Blish is not running, don't bother running this thread too often.
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        }
    });
}

/// Quick utility to ensure the header is valid.
fn is_header_valid(header: MEMORY_MAPPED_VIEW_ADDRESS) -> bool {
    unsafe {
        let ptr = header.Value as *const u8;
        if ptr.is_null() || !is_pointer_valid(ptr, HEADER_SIZE) {
            return false;
        }
        let data = from_raw_parts(ptr, HEADER_SIZE);
        let width = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let height = u32::from_le_bytes(data[4..8].try_into().unwrap());

        width > 0 && height > 0
    }
}

unsafe fn is_pointer_valid(ptr: *const u8, len: usize) -> bool {
    unsafe {
        let mut mbi = std::mem::zeroed::<MEMORY_BASIC_INFORMATION>();
        let result = VirtualQuery(
            Some(ptr as *const _),
            &mut mbi,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        );

        result != 0
            && mbi.State == MEM_COMMIT
            && mbi.Protect != PAGE_NOACCESS
            && (ptr as usize) >= mbi.BaseAddress as usize
            && (ptr as usize + len) <= (mbi.BaseAddress as usize + mbi.RegionSize)
    }
}

/// Quick utility to make the header invalid should Blish have closed down early.
fn make_header_invalid(header: MEMORY_MAPPED_VIEW_ADDRESS) {
    unsafe {
        let ptr = header.Value as *mut u8;
        write_unaligned(ptr as *mut u32, 0);
        write_unaligned(ptr.add(4) as *mut u32, 0);
    }
}

fn init_events() -> (HANDLE, HANDLE) {
    let frame_ready_name: Vec<u16> = "BlishHUD_FrameReady\0".encode_utf16().collect();
    let frame_consumed_name: Vec<u16> = "BlishHUD_FrameConsumed\0".encode_utf16().collect();
    let frame_ready = unsafe {
        CreateEventW(None, true, false, PCWSTR(frame_ready_name.as_ptr()))
            .expect("Could not open frame ready event.")
    };
    let frame_consumed = unsafe {
        CreateEventW(None, true, true, PCWSTR(frame_consumed_name.as_ptr()))
            .expect("Could not open frame consumed event.")
    };

    if frame_ready.0 == 0 || frame_consumed.0 == 0 {
        panic!("Could not initialize events.");
    }
    (frame_ready, frame_consumed)
}

///Waits for a frame to be ready
///Returns false if it assumes blish was closed, true otherwise
fn wait_for_frame_ready(frame_ready: &HANDLE) -> bool {
    let mut is_overlay_hidden = false;
    loop {
        let result = unsafe { WaitForSingleObject(*frame_ready, 100) };
        if result == WAIT_TIMEOUT {
            if is_overlay_hidden {
                continue;
            } else if !is_blish_alive() {
                return false;
            } else {
                //If Blish is still alive but we did not get a frame within the timeout,
                //We need to check if it went dead because eg. the UI is hidden
                //or eg. the frame did not change.
                if let Some(frame) = FRAME_BUFFER.get() {
                    if let Ok(mut lock) = frame.lock() {
                        if lock.hold {
                            is_overlay_hidden = false;
                            continue;
                        } else {
                            is_overlay_hidden = true;
                            lock.width = 0;
                            lock.height = 0;
                            lock.hold = false;
                            lock.pixels.clear();
                            continue;
                        }
                    }
                }
                log::error!("Failed to acquired the FRAME_BUFFER.");
                continue;
            }
        } else if result == WAIT_OBJECT_0 {
            return true;
        } else {
            return false;
        }
    }
}

//Simply pings the mutex in the blish fork, to check if it's still up and hasn't crashed.
fn is_blish_alive() -> bool {
    let handle = globals::LIVE_MUTEX.get_or_init(|| unsafe {
        let name: Vec<u16> = "Global\\blish_isalive_mutex"
            .encode_utf16()
            .chain(Some(0))
            .collect();

        OpenMutexW(
            Threading::SYNCHRONIZATION_ACCESS_RIGHTS(0x00100000),
            false,
            PCWSTR(name.as_ptr()),
        )
        .ok()
    });

    if let Some(h) = handle {
        matches!(
            unsafe { WaitForSingleObject(*h, 0) },
            WAIT_OBJECT_0 | WAIT_TIMEOUT
        )
    } else {
        false
    }
}

fn open_header_mmf() -> Result<MEMORY_MAPPED_VIEW_ADDRESS, ()> {
    let handle = get_header_handle()?;
    let header_ptr = unsafe { MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, HEADER_SIZE) };
    if header_ptr.Value.is_null() {
        log::error!("Could not read header info.");
        Err(())
    } else {
        Ok(header_ptr)
    }
}

fn open_body_mmf(
    size: usize,
    body_handle: &mut Option<HANDLE>,
) -> Result<MEMORY_MAPPED_VIEW_ADDRESS, ()> {
    if body_handle.is_none() {
        if let Ok(new_handle) = get_body_handle() {
            *body_handle = Some(new_handle);
        } else {
            return Err(());
        }
    }
    let body_ptr = unsafe { MapViewOfFile(body_handle.unwrap(), FILE_MAP_ALL_ACCESS, 0, 0, size) };
    if body_ptr.Value.is_null() {
        log::error!("Could not read shared body. Size: {}", size);
        Err(())
    } else {
        Ok(body_ptr)
    }
}

fn get_header_handle() -> Result<HANDLE, ()> {
    let lock = SHARED_HANDLE_HEADER.get_or_init(|| Mutex::new(HANDLE(0)));
    let mut guard = lock.lock().unwrap();

    if guard.0 != 0 {
        return Ok(*guard);
    }

    if let Ok(h) = unsafe {
        let wide_name: Vec<u16> = OsStr::new(HEADER_NAME)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        OpenFileMappingW(FILE_MAP_ALL_ACCESS.0, BOOL(0), PCWSTR(wide_name.as_ptr()))
    } {
        if h.0 != 0 {
            *guard = h;
        } else {
            return Err(());
        }
    } else {
        return Err(());
    }
    Ok(*guard)
}
fn get_body_handle() -> Result<HANDLE, ()> {
    let lock = SHARED_HANDLE_BODY.get_or_init(|| Mutex::new(HANDLE(0)));
    let mut guard = lock.lock().unwrap();
    if guard.0 != 0 {
        return Ok(*guard);
    }
    if let Ok(h) = unsafe {
        let wide_name: Vec<u16> = OsStr::new(BODY_NAME)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        OpenFileMappingW(FILE_MAP_ALL_ACCESS.0, BOOL(0), PCWSTR(wide_name.as_ptr()))
    } {
        if h.0 != 0 {
            *guard = h;
        } else {
            return Err(());
        }
    } else {
        return Err(());
    }
    Ok(*guard)
}

///This is the big function that receives the frame from Blish.
fn try_read_shared_memory(
    header: MEMORY_MAPPED_VIEW_ADDRESS,
    frame_ready: &HANDLE,
    frame_consumed: &HANDLE,
    body: &mut Option<MEMORY_MAPPED_VIEW_ADDRESS>,
    body_handle: &mut Option<HANDLE>,
) {
    unsafe {
        let header_ptr = header.Value as *mut u8;

        //Header data
        let header = from_raw_parts(header_ptr as *const u8, HEADER_SIZE);

        //Dimensions of the frame
        let width = u32::from_le_bytes(header[0..4].try_into().unwrap());
        let height = u32::from_le_bytes(header[4..8].try_into().unwrap());

        //Whether to hold the frame if no other is sent after that. (Eg if the frame hasn't changed)
        let hold_u32 = u32::from_le_bytes(header[8..12].try_into().unwrap());
        let hold = hold_u32 != 0;

        if width == 0 || height == 0 || width > 10000 || height > 10000 {
            log::error!("Width/Height issue: {}x{}", width, height);
            return;
        }

        //Read the actual frame
        let total_size = (width * height * 4) as usize;

        let mtx = FRAME_BUFFER.get().unwrap();
        let mut frame = mtx.lock().unwrap();

        //Resize occured
        if frame.width != width || frame.height != height || body.is_none() {
            frame.width = width;
            frame.height = height;

            //malloc
            frame.pixels = Vec::with_capacity(total_size);
            frame.pixels.set_len(total_size);

            //New Body
            if let Some(old_body) = body {
                if UnmapViewOfFile(*old_body).is_err() {
                    return;
                }
            }

            if let Ok(new_body) = open_body_mmf(total_size, body_handle) {
                *body = Some(new_body);
            } else {
                return;
            }
        }

        if let Some(body) = body {
            //Copy frame
            std::ptr::copy_nonoverlapping(
                body.Value as *const u8,
                frame.pixels.as_mut_ptr(),
                total_size,
            );
            frame.hold = hold;
        }

        SetEvent(*frame_consumed).expect("Could not set the frame_consumed event.");
        ResetEvent(*frame_ready).expect("Could not reset the frame_ready event.");
    }
}
