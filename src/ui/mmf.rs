use std::{
    ffi::OsStr, os::windows::ffi::OsStrExt, ptr::write_unaligned, slice::from_raw_parts,
    sync::Mutex,
};

use crate::globals::{self};
use windows::{
    Win32::{
        Foundation::{BOOL, HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::{
            Memory::{
                FILE_MAP_ALL_ACCESS, MEM_COMMIT, MEMORY_BASIC_INFORMATION,
                MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile, OpenFileMappingW, PAGE_NOACCESS,
                VirtualQuery,
            },
            Threading::{self, CreateEventW, OpenMutexW, WaitForSingleObject},
        },
    },
    core::PCWSTR,
};

use super::{HEADER_NAME, HEADER_SIZE, MMF_DATA, SHARED_HANDLE_HEADER, rendering::OverlayState};

#[derive(Debug)]
pub struct MMFData {
    pub width: u32,
    pub height: u32,
    pub index: u32,
    pub addr1: u64,
    pub addr2: u64,
}
impl MMFData {
    pub fn get_current_addr(&self) -> u64 {
        if self.index == 0 {
            self.addr1
        } else {
            self.addr2
        }
    }
}

pub fn read_mmf_data(state: &mut OverlayState) {
    let mut header: Option<MEMORY_MAPPED_VIEW_ADDRESS> = None;
    if header.is_none() {
        header = open_header_mmf().ok();
    }

    if MMF_DATA.get().is_none() {
        MMF_DATA
            .set(Mutex::new(MMFData {
                width: 0,
                height: 0,
                index: 0,
                addr1: 0,
                addr2: 0,
            }))
            .ok();
    }

    if let Some(ptr) = header {
        let mut mmfdata = MMF_DATA.get().unwrap().lock().unwrap();

        let ptr = ptr.Value as *mut u8;
        let data = unsafe { from_raw_parts(ptr, HEADER_SIZE) };

        mmfdata.width = u32::from_le_bytes(data[0..4].try_into().unwrap());
        mmfdata.height = u32::from_le_bytes(data[4..8].try_into().unwrap());
        mmfdata.index = u32::from_le_bytes(data[8..12].try_into().unwrap());
        mmfdata.addr1 = u64::from_le_bytes(data[12..20].try_into().unwrap());
        mmfdata.addr2 = u64::from_le_bytes(data[20..28].try_into().unwrap());

        state.resize(mmfdata.width, mmfdata.height);
    }
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
