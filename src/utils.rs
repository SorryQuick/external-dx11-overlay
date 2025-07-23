use std::{
    ffi::OsStr,
    mem::{self, size_of},
    os::windows::ffi::OsStrExt,
};
use windows::{
    Win32::{
        Foundation::{BOOL, HWND, LPARAM},
        System::{
            LibraryLoader::GetModuleHandleA,
            ProcessStatus::{GetModuleInformation, MODULEINFO},
            Threading::{GetCurrentProcess, GetCurrentProcessId},
        },
        UI::{
            Input::KeyboardAndMouse::IsWindowEnabled,
            WindowsAndMessaging::{
                EnumWindows, FindWindowW, GW_OWNER, GWL_EXSTYLE, GetParent, GetWindow,
                GetWindowLongPtrW, GetWindowTextW, GetWindowThreadProcessId, IsWindow,
                IsWindowVisible, WS_EX_TOOLWINDOW,
            },
        },
    },
    core::PCWSTR,
};

///Returns the base address of the process memory.
///This returns the range (from, to) used to compute offsets
///and modify the program's memory.
pub fn get_base_addr_and_size() -> (usize, usize) {
    unsafe {
        let handle = GetModuleHandleA(None);
        if handle.is_err() {
            return (0, 0);
        }
        let handle = handle.unwrap();
        let mut modinfo: MODULEINFO = mem::zeroed();

        let ret = GetModuleInformation(
            GetCurrentProcess(),
            handle,
            &mut modinfo as *mut MODULEINFO,
            size_of::<MODULEINFO>() as u32,
        );
        if ret.is_err() {
            return (0, 0);
        }
        (modinfo.lpBaseOfDll as usize, modinfo.SizeOfImage as usize)
    }
}
///Gets the HWND of the window this DLL is attached to.
///There very well may be a better way to do this.
pub fn get_mainwindow_hwnd() -> Option<HWND> {
    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            let target_pid = GetCurrentProcessId();

            if pid == target_pid {
                let is_visible = IsWindowVisible(hwnd).as_bool();
                let is_enabled = IsWindowEnabled(hwnd).as_bool();
                let owner = GetWindow(hwnd, GW_OWNER);
                let parent = GetParent(hwnd);

                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;

                if !is_visible || !is_enabled || (ex_style & WS_EX_TOOLWINDOW.0) != 0 {
                    return true.into();
                }

                if owner.0 == 0 && parent.0 == 0 {
                    *(lparam.0 as *mut HWND) = hwnd;
                    return false.into();
                }
            }
        }
        true.into()
    }

    unsafe {
        let mut hwnd: HWND = HWND(0);
        EnumWindows(
            Some(enum_windows_proc),
            LPARAM(&mut hwnd as *mut _ as isize),
        )
        .ok();

        if hwnd.0 != 0 { Some(hwnd) } else { None }
    }
}

fn to_pcwstr(s: &str) -> PCWSTR {
    //Create null-terminated u16 array as is the wide character standard.
    let wide: Vec<u16> = OsStr::new(s).encode_wide().chain(Some(0)).collect();
    let ptr = wide.as_ptr();

    std::mem::forget(wide);

    PCWSTR(ptr)
}

///Simple utility to get a HWND from a window title.
pub fn find_hwnd_by_title(title: &str) -> Option<HWND> {
    let hwnd = unsafe { FindWindowW(PCWSTR::null(), to_pcwstr(title)) };
    if hwnd.0 != 0 { Some(hwnd) } else { None }
}

///For debugging purposes. Lists all windows and their titles.
///Helps to know what to pass to find_hwnd_by_title()
pub fn dump_all_window_titles() {
    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, _lparam: LPARAM) -> BOOL {
        unsafe {
            if !IsWindow(hwnd).as_bool() || !IsWindowVisible(hwnd).as_bool() {
                return true.into();
            }
            let mut buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut buf);
            if len > 0 {
                let title = String::from_utf16_lossy(&buf[..len as usize]);
                println!("HWND: {:?} - Title: {}", hwnd, title);
            }
            true.into()
        }
    }

    unsafe {
        EnumWindows(Some(enum_windows_proc), LPARAM(0)).ok();
    }
}

///Takes a pointer to some area in memory and derefences it into T
pub fn read<T: Clone>(p: usize) -> Option<T> {
    if p == 0 {
        None
    } else {
        unsafe { Some((*(p as *const T)).clone()) }
    }
}
