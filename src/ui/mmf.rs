use std::{
    ffi::OsStr, os::windows::ffi::OsStrExt, ptr::read_unaligned, slice::from_raw_parts, sync::Mutex,
};

use windows::{
    Win32::{
        Foundation::{BOOL, HANDLE},
        System::Memory::{
            FILE_MAP_ALL_ACCESS, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile, OpenFileMappingW,
            UnmapViewOfFile,
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
    FRAME_BUFFER.set(Mutex::new(None)).unwrap();

    std::thread::spawn(move || {
        let mut header: Option<MEMORY_MAPPED_VIEW_ADDRESS> = None;
        loop {
            if header.is_none() {
                header = open_header_mmf().ok();
            }
            if let Some(header_ptr) = header {
                if let Some(data) = try_read_shared_memory(header_ptr) {
                    if let Some(buf) = FRAME_BUFFER.get() {
                        let mut buf = buf.lock().unwrap();
                        *buf = Some(data)
                    }
                }
                wait_for_frame_ready(header_ptr.Value as *mut u8);
            } else {
                //If Blish is not running, don't bother running this thread too often.
                std::thread::sleep(std::time::Duration::from_millis(1000));
            }
        }
    });
}

///Waits for a frame to be ready
//TODO: Use events for better performance, CreateEvent...
fn wait_for_frame_ready(header_ptr: *mut u8) {
    let mut attempts = 0;
    loop {
        let ready = unsafe { read_unaligned(header_ptr.add(8) as *const u32).to_le() };

        if ready == 1 {
            break;
        }

        // Spin a bit, then yield/sleep
        if attempts < 50 {
            std::hint::spin_loop();
        } else if attempts < 200 {
            std::thread::yield_now();
        }

        attempts += 1;
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
fn try_read_shared_memory(header: MEMORY_MAPPED_VIEW_ADDRESS) -> Option<SharedFrame> {
    unsafe {
        let header_ptr = header.Value as *mut u8;

        //Header data, look at C# doc for format.
        let header = from_raw_parts(header_ptr as *const u8, HEADER_SIZE);

        //println!("Got a frame! {} Data: {:02X?}", header.len(), header);

        //Dimensions of the frame
        let width = u32::from_le_bytes(header[0..4].try_into().unwrap());
        let height = u32::from_le_bytes(header[4..8].try_into().unwrap());

        if width == 0 || height == 0 || width > 10000 || height > 10000 {
            println!("Width/Height issue: {}x{}", width, height);
            return None;
        }

        //Read the actual frame
        let max_frame_size = 3840 * 2160 * 4; //TODO: Dynamic size
        let max_buffer_size = 4 + max_frame_size; //Include 4 bytes for dirty_count
        let total_size = max_buffer_size;

        let full_ptr = open_body_mmf(total_size).ok()?;
        let buffer_ptr = full_ptr.Value as *const u8;

        // Parse dirty rect count
        let dirty_count =
            u32::from_le_bytes(from_raw_parts(buffer_ptr, 4).try_into().unwrap()) as usize;

        //Clone here because we don't want to hold the lock for too long.
        //Pretty important since we need this lock everytime the mouse moves and a frame is drawn.
        let mut full_pixels = if let Some(mtx) = FRAME_BUFFER.get() {
            let guard = mtx.lock().unwrap();
            if let Some(ref frame) = *guard {
                if frame.width == width && frame.height == height {
                    frame.pixels.clone()
                } else {
                    vec![0u8; (width * height * 4) as usize]
                }
            } else {
                vec![0u8; (width * height * 4) as usize]
            }
        } else {
            vec![0u8; (width * height * 4) as usize]
        };

        //Loop over the dirty rectangles
        let mut offset = 4;
        for _ in 0..dirty_count {
            if offset > max_buffer_size {
                println!("Offset calculation issue in shared memory");
                UnmapViewOfFile(full_ptr).ok();
                return None;
            }
            let rect_x = u32::from_le_bytes(
                from_raw_parts(buffer_ptr.add(offset), 4)
                    .try_into()
                    .unwrap(),
            ) as usize;
            offset += 4;
            let rect_y = u32::from_le_bytes(
                from_raw_parts(buffer_ptr.add(offset), 4)
                    .try_into()
                    .unwrap(),
            ) as usize;
            offset += 4;
            let rect_w = u32::from_le_bytes(
                from_raw_parts(buffer_ptr.add(offset), 4)
                    .try_into()
                    .unwrap(),
            ) as usize;
            offset += 4;
            let rect_h = u32::from_le_bytes(
                from_raw_parts(buffer_ptr.add(offset), 4)
                    .try_into()
                    .unwrap(),
            ) as usize;
            offset += 4;
            /*if dirty_count == 1 {
                println!(
                    "rect_x: {}, rect_y: {}, w: {}, h: {}",
                    rect_x, rect_y, rect_w, rect_h
                );
            }*/
            let pixels_len = rect_w * rect_h * 4;
            if offset + pixels_len > max_buffer_size {
                println!(
                    "Offset calculation issue in shared memory (2) {} + {} > {}",
                    offset, pixels_len, max_buffer_size
                );
                UnmapViewOfFile(full_ptr).ok();
                return None;
            }
            if rect_x + rect_w > width as usize || rect_y + rect_h > height as usize {
                //Frame inconsistency, drop it.
                println!("Bad frame??? This should never trigger?");
                UnmapViewOfFile(full_ptr).ok();
                return None;
            }
            for row in 0..rect_h {
                let src_start = offset + row * rect_w * 4;
                let dst_start = ((rect_y + row) * width as usize + rect_x) * 4;

                full_pixels[dst_start..dst_start + rect_w * 4]
                    .copy_from_slice(from_raw_parts(buffer_ptr.add(src_start), rect_w * 4));
            }
            offset += pixels_len;
        }

        UnmapViewOfFile(full_ptr).ok();

        //Update frame_consumed = 1 and frame_ready = 0
        std::ptr::write_unaligned(header_ptr.add(12) as *mut i32, 1);
        std::ptr::write_unaligned(header_ptr.add(8) as *mut i32, 0);

        Some(SharedFrame {
            width,
            height,
            pixels: full_pixels,
        })
    }
}
