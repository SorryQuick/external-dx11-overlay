use crate::ui::OVERLAY_STATE;
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::AtomicBool;
use std::thread::sleep;
use std::time::Duration;
use windows::Win32::{
    Foundation::CloseHandle,
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
            TH32CS_SNAPPROCESS,
        },
        Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess},
    },
};

pub mod debug_overlay;
pub mod statistics;

//Anything related to debugging should be added here, then toggled with a keybind.
pub struct DebugFeatures {
    pub rendering_enabled: AtomicBool,
    pub processing_enabled: AtomicBool,
    pub debug_overlay_enabled: AtomicBool,
}

pub static DEBUG_FEATURES: DebugFeatures = DebugFeatures {
    rendering_enabled: AtomicBool::new(true),
    processing_enabled: AtomicBool::new(true),
    debug_overlay_enabled: AtomicBool::new(false),
};

//Prints a bunch of debug info.
pub fn dump_debug_data() {
    log::info!("------PRINTING DEBUG DATA------");

    {
        log::info!("Overlay State:");
        let state = OVERLAY_STATE.get().unwrap();
        let mut state_lock_opt = state.lock().unwrap();
        let state_lock = state_lock_opt.as_mut().unwrap();
        log::info!("  Width: {}", state_lock.width);
        log::info!("  Height: {}", state_lock.height);
        log::info!("Attempting to reset OVERLAY_STATE");
        *state_lock_opt = None;
    }

    log::info!("-------------------------------");
}

pub fn restart_blish() {
    log::info!("Restarting blish");
    kill_process_by_name("Blish HUD.exe");
    sleep(Duration::from_millis(1000));
    Command::new("addons/LOADER_public/Blish.HUD.1.2.0/Blish HUD.exe")
        .creation_flags(0x08000000)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok();
}

fn kill_process_by_name(target: &str) {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let exe_name = String::from_utf16_lossy(
                    &entry.szExeFile[..entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(0)],
                );

                if exe_name.eq_ignore_ascii_case(target) {
                    // Open process with terminate rights
                    let h_process = OpenProcess(PROCESS_TERMINATE, false, entry.th32ProcessID);
                    if let Ok(handle) = h_process {
                        TerminateProcess(handle, 1).ok();
                        CloseHandle(handle).ok();
                        println!("Terminated {}", exe_name);
                    } else {
                        println!("Failed to open {}", exe_name);
                    }
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        CloseHandle(snapshot).ok();
    }
}
