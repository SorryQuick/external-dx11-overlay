use std::{
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    ptr::{null_mut, write_unaligned},
    slice::from_raw_parts,
    sync::Mutex,
    time::Duration,
};

use crate::globals::{self};
use windows::{
    Win32::{
        Foundation::{BOOL, CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT},
        Graphics::Dxgi::IDXGISwapChain,
        System::{
            Memory::{
                FILE_MAP_ALL_ACCESS, MEM_COMMIT, MEMORY_BASIC_INFORMATION,
                MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile, OpenFileMappingW, PAGE_NOACCESS,
                UnmapViewOfFile, VirtualQuery,
            },
            Threading::{self, CreateEventW, MUTEX_ALL_ACCESS, OpenMutexW, WaitForSingleObject},
        },
    },
    core::PCWSTR,
};

use super::{HEADER_NAME, HEADER_SIZE, MMF_DATA, SHARED_HANDLE_HEADER, rendering::OverlayState};

#[derive(Debug)]
pub struct MMFData {
    header: Option<MEMORY_MAPPED_VIEW_ADDRESS>,
    file_mapping: Option<HANDLE>,
    pub width: u32,
    pub height: u32,
    pub index: u32,
    pub addr1: u64,
    pub addr2: u64,
}
unsafe impl Send for MMFData {}
impl MMFData {
    pub fn get_current_addr(&self) -> u64 {
        if self.index == 0 {
            self.addr1
        } else {
            self.addr2
        }
    }
}

pub fn read_mmf_data() -> Result<(), ()> {
    if MMF_DATA.get().is_none() {
        MMF_DATA
            .set(Mutex::new(MMFData {
                header: None,
                file_mapping: None,
                width: 0,
                height: 0,
                index: 0,
                addr1: 0,
                addr2: 0,
            }))
            .ok();
    }
    let mut mmfdata = MMF_DATA.get().unwrap().lock().unwrap();
    if mmfdata.header.is_none() {
        if !is_blish_alive() {
            //This is fine, if blish is not open we just get out asap and rendering will not
            //proceed. For some reason, the MMF can still be "valid" even if blish was killed.
            return Err(());
        }

        if let Ok(header) = open_header_mmf(&mut mmfdata) {
            mmfdata.header = Some(header);
        } else {
            return Err(());
        }
    }
    if let Some(ptr) = mmfdata.header {
        let ptr = ptr.Value as *mut u8;
        let data = unsafe { from_raw_parts(ptr, HEADER_SIZE) };

        mmfdata.width = u32::from_le_bytes(data[0..4].try_into().unwrap());
        mmfdata.height = u32::from_le_bytes(data[4..8].try_into().unwrap());
        mmfdata.index = u32::from_le_bytes(data[8..12].try_into().unwrap());
        mmfdata.addr1 = u64::from_le_bytes(data[12..20].try_into().unwrap());
        mmfdata.addr2 = u64::from_le_bytes(data[20..28].try_into().unwrap());
    }
    Ok(())
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

//Simply pings the mutex in the blish fork, to check if it's still up and hasn't crashed.
pub fn is_blish_alive() -> bool {
    let name: Vec<u16> = "Global\\blish_isalive_mutex"
        .encode_utf16()
        .chain(Some(0))
        .collect();

    unsafe {
        match OpenMutexW(
            Threading::SYNCHRONIZATION_ACCESS_RIGHTS(0x00100000),
            false,
            PCWSTR(name.as_ptr()),
        ) {
            Ok(handle) => {
                CloseHandle(handle).ok();
                true
            }
            Err(e) => {
                let err = e.code().0 as u32;
                match err {
                    2 | 123 => false,
                    5 => true,
                    _ => false,
                }
            }
        }
    }
}

pub fn cleanup_shutdown() {
    if let Some(mmfdata) = MMF_DATA.get() {
        let mut mmfdata = mmfdata.lock().unwrap();
        if let (Some(view), Some(handle)) = (mmfdata.header, mmfdata.file_mapping) {
            unsafe {
                std::ptr::write_bytes(view.Value, 0, HEADER_SIZE);
            }
        }
        if let Some(view) = mmfdata.header.take() {
            unsafe {
                UnmapViewOfFile(view);
            }
        }
        if let Some(hmap) = mmfdata.file_mapping.take() {
            unsafe {
                CloseHandle(hmap);
            }
        }
        mmfdata.height = 0;
        mmfdata.width = 0;
        mmfdata.index = 0;
        mmfdata.addr1 = 0;
        mmfdata.addr2 = 0;
    }
}

fn open_header_mmf(mmfdata: &mut MMFData) -> Result<MEMORY_MAPPED_VIEW_ADDRESS, ()> {
    unsafe {
        let wide_name: Vec<u16> = OsStr::new(HEADER_NAME)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        mmfdata.file_mapping =
            OpenFileMappingW(FILE_MAP_ALL_ACCESS.0, BOOL(0), PCWSTR(wide_name.as_ptr())).ok();
        if let Some(map) = mmfdata.file_mapping {
            let view = MapViewOfFile(map, FILE_MAP_ALL_ACCESS, 0, 0, HEADER_SIZE);
            if view.Value != null_mut() {
                return Ok(view);
            }
        }
        Err(())
    }
}
