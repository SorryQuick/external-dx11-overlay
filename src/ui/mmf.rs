use std::{
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    ptr::read_unaligned,
    slice::from_raw_parts,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

use windows::{
    Win32::{
        Foundation::{BOOL, HANDLE, WAIT_OBJECT_0},
        System::{
            Memory::{
                FILE_MAP_ALL_ACCESS, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile, OpenFileMappingW,
                UnmapViewOfFile,
            },
            Threading::{
                self, CreateEventW, OpenEventW, ResetEvent, SetEvent, WaitForSingleObject,
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
    pub pixels: Vec<u8>, // RGBA values in pairs of 4 bytes.
}

///This thread listens for messages from Blish. It updates the FRAME_BUFFER accordingly.
pub fn start_frame_watcher_thread() {
    FRAME_BUFFER
        .set(Mutex::new(SharedFrame {
            width: 0,
            height: 0,
            pixels: Vec::new(),
        }))
        .unwrap();

    std::thread::spawn(move || {
        let mut header: Option<MEMORY_MAPPED_VIEW_ADDRESS> = None;
        let (frame_ready, frame_consumed) = init_events();
        loop {
            if header.is_none() {
                header = open_header_mmf().ok();
            }
            if let Some(header_ptr) = header {
                try_read_shared_memory(header_ptr, &frame_ready, &frame_consumed);
                wait_for_frame_ready(header_ptr.Value as *mut u8, &frame_ready);
            } else {
                //If Blish is not running, don't bother running this thread too often.
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        }
    });
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
fn wait_for_frame_ready(header_ptr: *mut u8, frame_ready: &HANDLE) {
    let result = unsafe { WaitForSingleObject(*frame_ready, Threading::INFINITE) };
    if result != WAIT_OBJECT_0 {
        panic!("WaitForSingleObject failed.");
    }
}

fn open_header_mmf() -> Result<MEMORY_MAPPED_VIEW_ADDRESS, ()> {
    let handle = get_header_handle()?;
    let header_ptr = unsafe { MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, HEADER_SIZE) };
    if header_ptr.Value.is_null() {
        println!("Could not read header info.");
        Err(())
    } else {
        Ok(header_ptr)
    }
}

fn open_body_mmf(size: usize) -> Result<MEMORY_MAPPED_VIEW_ADDRESS, ()> {
    let handle = get_body_handle()?;
    let body_ptr = unsafe { MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, size) };
    if body_ptr.Value.is_null() {
        println!("Could not read shared body. Size: {}", size);
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
) {
    unsafe {
        let header_ptr = header.Value as *mut u8;

        //Header data
        let header = from_raw_parts(header_ptr as *const u8, HEADER_SIZE);

        //Dimensions of the frame
        let width = u32::from_le_bytes(header[0..4].try_into().unwrap());
        let height = u32::from_le_bytes(header[4..8].try_into().unwrap());

        if width == 0 || height == 0 || width > 10000 || height > 10000 {
            println!("Width/Height issue: {}x{}", width, height);
            return;
        }

        //Read the actual frame
        let total_size = (width * height * 4) as usize;
        let full_ptr = open_body_mmf(total_size).unwrap();
        let buffer_ptr = full_ptr.Value as *const u8;

        //Copy frame
        let mtx = FRAME_BUFFER.get().unwrap();
        let mut frame = mtx.lock().unwrap();

        if frame.width != width || frame.height != height {
            frame.width = width;
            frame.height = height;

            //malloc
            frame.pixels = Vec::with_capacity(total_size);
            frame.pixels.set_len(total_size);
        }

        std::ptr::copy_nonoverlapping(buffer_ptr, frame.pixels.as_mut_ptr(), total_size);

        UnmapViewOfFile(full_ptr).ok();

        SetEvent(*frame_consumed).expect("Could not set the frame_consumed event.");
        ResetEvent(*frame_ready).expect("Could not reset the frame_ready event.");
    }
}
