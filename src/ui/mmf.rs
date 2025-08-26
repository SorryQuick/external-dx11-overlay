use std::{
    ffi::OsStr,
    os::windows::ffi::OsStrExt,
    ptr::null_mut,
    slice::from_raw_parts,
    sync::{Arc, RwLock},
    time::Duration,
};

use windows::{
    Win32::{
        Foundation::{BOOL, CloseHandle, HANDLE},
        System::{
            Memory::{
                FILE_MAP_ALL_ACCESS, MEMORY_MAPPED_VIEW_ADDRESS, MapViewOfFile, OpenFileMappingW,
                UnmapViewOfFile,
            },
            Threading::{self, OpenMutexW},
        },
    },
    core::PCWSTR,
};

use super::{HEADER_NAME, HEADER_SIZE, MMF_DATA, OVERLAY_STATE};

#[derive(Debug)]
pub struct MMFData {
    header: Option<MEMORY_MAPPED_VIEW_ADDRESS>,
    file_mapping: Option<HANDLE>,
    pub width: u32,
    pub height: u32,
    pub index: u32,
    pub addr1: u64,
    pub addr2: u64,
    pub is_blish_alive: bool,
}
unsafe impl Send for MMFData {}
unsafe impl Sync for MMFData {}

///This thread runs forever, updating the MMF data so as to not block present()
///With this current method, it takes 0-500 nanoseconds to get the lock in present().
///The performance impact is therefore unnoticable. However, it's important that the
///write lock is ONLY KEPT ALIVE AS LITTLE AS POSSIBLE. In other words, it should only be
///locked when directly reading or writing from MMFData, no other functions should be called
///while the lock is held. If more speed is required, use double buffering.
pub fn start_mmf_thread() {
    std::thread::spawn(|| {
        if MMF_DATA.get().is_none() {
            MMF_DATA
                .set(Arc::new(RwLock::new(MMFData {
                    header: None,
                    file_mapping: None,
                    width: 0,
                    height: 0,
                    index: 0,
                    addr1: 0,
                    addr2: 0,
                    is_blish_alive: false,
                })))
                .ok();
        }

        loop {
            //Get data locally so we can drop the lock
            let mut mmfdata = MMF_DATA.get().unwrap().write().unwrap();
            let mut blish_alive = mmfdata.is_blish_alive;
            let mut header = mmfdata.header;
            let mut mapping = mmfdata.file_mapping;
            drop(mmfdata);

            //Handle blish opening/closing
            if !is_blish_alive() {
                //It just got closed/crashed
                if blish_alive {
                    cleanup_shutdown();
                    continue;
                }
                blish_alive = false;
            } else {
                blish_alive = true;
            }
            if header.is_none() {
                if let Ok((_header, _mapping)) = open_header_mmf() {
                    header = Some(_header);
                    mapping = Some(_mapping);
                }
            }

            if let Some(ptr) = header {
                //Read into local variables, we don't want to lock mmfdata yet
                //since MMF reads are "slow" compared to assigning to a struct.
                let ptr = ptr.Value as *mut u8;
                let data = unsafe { from_raw_parts(ptr, HEADER_SIZE) };
                let width = u32::from_le_bytes(data[0..4].try_into().unwrap());
                let height = u32::from_le_bytes(data[4..8].try_into().unwrap());
                let index = u32::from_le_bytes(data[8..12].try_into().unwrap());
                let addr1 = u64::from_le_bytes(data[12..20].try_into().unwrap());
                let addr2 = u64::from_le_bytes(data[20..28].try_into().unwrap());

                //Lock real quick while copying the data (should be very fast)
                mmfdata = MMF_DATA.get().unwrap().write().unwrap();
                mmfdata.header = header;
                mmfdata.file_mapping = mapping;
                mmfdata.is_blish_alive = blish_alive;
                mmfdata.width = width;
                mmfdata.height = height;
                mmfdata.index = index;
                mmfdata.addr1 = addr1;
                mmfdata.addr2 = addr2;
                drop(mmfdata);
            }

            std::thread::sleep(Duration::from_millis(20));
        }
    });
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
        let mut mmfdata = mmfdata.write().unwrap();
        if let (Some(view), Some(_)) = (mmfdata.header, mmfdata.file_mapping) {
            unsafe {
                std::ptr::write_bytes(view.Value, 0, HEADER_SIZE);
            }
        }
        if let Some(view) = mmfdata.header.take() {
            unsafe {
                UnmapViewOfFile(view).ok();
            }
        }
        if let Some(hmap) = mmfdata.file_mapping.take() {
            unsafe {
                CloseHandle(hmap).ok();
            }
        }
        mmfdata.height = 0;
        mmfdata.is_blish_alive = false;
        mmfdata.width = 0;
        mmfdata.index = 0;
        mmfdata.addr1 = 0;
        mmfdata.addr2 = 0;
    }
    if let Some(state) = OVERLAY_STATE.get() {
        let mut lock = state.lock().unwrap();
        let state = lock.as_mut();
        if let Some(state) = state {
            state.shutdown();
        }
    }
}

fn open_header_mmf() -> Result<(MEMORY_MAPPED_VIEW_ADDRESS, HANDLE), ()> {
    unsafe {
        let wide_name: Vec<u16> = OsStr::new(HEADER_NAME)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let file_mapping =
            OpenFileMappingW(FILE_MAP_ALL_ACCESS.0, BOOL(0), PCWSTR(wide_name.as_ptr())).ok();
        if let Some(map) = file_mapping {
            let view = MapViewOfFile(map, FILE_MAP_ALL_ACCESS, 0, 0, HEADER_SIZE);
            if view.Value != null_mut() {
                return Ok((view, map));
            }
        }
        Err(())
    }
}
