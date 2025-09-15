use address_finder::AddressFinder;
use chrono::Local;
use controls::{initialize_controls, start_mouse_input_thread};
use debug::{debug_overlay::add_to_debug_log_overlay, statistics::start_statistics_server};
use fern::Dispatch;
use hooks::present_hook;
use keybinds::init_keybinds;
use std::{
    fs::{OpenOptions, create_dir_all},
    mem,
    path::PathBuf,
};
use ui::mmf::start_mmf_thread;
use utils::{get_base_addr_and_size, get_mainwindow_hwnd};
use windows::Win32::{
    Foundation::HINSTANCE,
    System::{
        LibraryLoader::FreeLibraryAndExitThread
    },
};
#[cfg(not(feature = "nexus"))]
use windows::Win32::System::SystemServices::{DLL_PROCESS_DETACH, DLL_PROCESS_ATTACH};

#[cfg(feature = "nexus")]
use nexus::{self, AddonFlags};

#[cfg(feature = "nexus")]
pub mod nexus_addon;

pub mod address_finder;
pub mod controls;
pub mod debug;
pub mod globals;
pub mod hooks;
pub mod keybinds;
pub mod ui;
pub mod utils;

static mut HANDLE_NO: u64 = 0;

/*
 *
 * This assumes that the DLL is loaded in a general way, such as LoadLibraryW. If other loading
 * methods need to be supported, simply call attach() and detatch() where appropriate.
 *
 * TODO: detatch() is poorly tested. It also definitely lacks some unloading stuff, like wnd_proc
 *
 * */
#[cfg(not(feature = "nexus"))]
#[unsafe(no_mangle)]
#[allow(unused_variables)]
extern "system" fn DllMain(dll_module: HINSTANCE, call_reason: u32, _: *mut ()) -> bool {
    match call_reason {
        DLL_PROCESS_ATTACH => attach(dll_module),
        DLL_PROCESS_DETACH => detatch(),

        _ => (),
    }
    true
}
///THE MAIN FUNCTION. It initializes everything needed.
///Ideally, all hooks are created here.
fn attach(handle: HINSTANCE) {
    std::thread::spawn(move || {
        log::info!("Attaching to process");
        enable_logging();

        //Do this early - only needed for external overlay functionality
        start_mmf_thread();

        let (base, size) = get_base_addr_and_size();

        let mainwindow_hwnd = get_mainwindow_hwnd().expect("Could not get the game's window.");

        if base == 0 || size == 0 {
            log::error!(
                "Could not get the module base/size. Base: {} Size: {}",
                base,
                size
            );
            unsafe { FreeLibraryAndExitThread(HINSTANCE { 0: handle.0 }, 0) };
        }

        let address_finder = AddressFinder {
            base_addr: base,
            module_size: size,
        };

            let present_addr = address_finder.find_addr_present();

            if present_addr == 0 {
                log::error!("Could not find the address of DirectX11 Present.");
                unsafe { FreeLibraryAndExitThread(HINSTANCE { 0: handle.0 }, 0) };
            }

            unsafe {
                present_hook
                    .initialize(
                        mem::transmute(present_addr as *const ()),
                        ui::get_detoured_present(),
                    )
                    .unwrap()
                    .enable()
                    .unwrap();
            }

        unsafe { HANDLE_NO = handle.0 as u64 };

            start_statistics_server();
            init_keybinds();

            //MUST BE CALLED IN THIS ORDER
            start_mouse_input_thread();
            initialize_controls(mainwindow_hwnd);

    });
}

fn detatch() {
    log::info!("Detatching from process");
    unsafe {
        present_hook.disable().unwrap();
    }
}
fn enable_logging() {
    let file = {

        let logs_dir = PathBuf::from("addons/LOADER_public/logs");

        create_dir_all(&logs_dir).expect("Failed to create logs directory");

        let filename = format!("overlay-{}.log", Local::now().format("%Y-%m-%d_%H-%M-%S"));
        let filepath = logs_dir.join(filename);

        OpenOptions::new()
            .create(true)
            .append(true)
            .open(filepath)
            .expect("Failed to open log file")
    };

    //Init Fern
    Dispatch::new()
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .chain(file)
        .format(|out, message, record| {
            let now = Local::now();
            let format = if record.level() == log::Level::Error {
                format_args!(
                    "[{}] [external-dx11-overlay] [{}] [{}:{}] {}",
                    now.format("%Y-%m-%d %H:%M:%S"),
                    record.level(),
                    record.file().unwrap_or("<unknown>"),
                    record.line().unwrap_or(0),
                    message
                )
            } else {
                format_args!(
                    "[{}] [external-dx11-overlay] [{}] {}",
                    now.format("%Y-%m-%d %H:%M:%S"),
                    record.level(),
                    message
                )
            };
            add_to_debug_log_overlay(format.to_string());
            out.finish(format);
        })
        .apply()
        .ok();

    //Panic hook
    std::panic::set_hook(Box::new(|panic_info| {
        let payload = panic_info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| {
                panic_info
                    .payload()
                    .downcast_ref::<String>()
                    .map(|s| s.as_str())
            })
            .unwrap_or("Unknown panic");

        let location = panic_info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown location".to_string());
        log::error!("PANIC at {}: {}", location, payload);
    }));

    log::info!(
        "---------------------------------------- New Session ----------------------------------------------"
    );
}

// ======= Nexus export - only compiled when building for nexus =============
#[cfg(feature = "nexus")]
nexus::export! {
    name: "Blish HUD overlay loader",
    signature: -0x7A8B9C2D,
    load: nexus_addon::nexus_load,
    unload: nexus_addon::nexus_unload,
    flags: AddonFlags::None,
    provider: nexus::UpdateProvider::GitHub,
    update_link: "https://github.com/SorryQuick/external-dx11-overlay",
    log_filter: "trace"
}