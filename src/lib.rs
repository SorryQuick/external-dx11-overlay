use address_finder::AddressFinder;
use controls::initialize_controls;
use hooks::present_hook;
use std::mem;
use utils::{get_base_addr_and_size, get_mainwindow_hwnd};
use windows::Win32::{
    Foundation::HINSTANCE,
    System::{
        Console::{AllocConsole, FreeConsole},
        LibraryLoader::FreeLibraryAndExitThread,
        SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH},
    },
};

pub mod address_finder;
pub mod controls;
pub mod globals;
pub mod hooks;
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
    #[cfg(debug_assertions)]
    unsafe {
        AllocConsole().unwrap()
    };

    std::thread::spawn(move || {
        let (base, size) = get_base_addr_and_size();
        let mainwindow_hwnd = get_mainwindow_hwnd().expect("Could not get the game's window.");

        if base == 0 || size == 0 {
            println!(
                "Could not get the module base/size. Base: {} Size: {}",
                base, size
            );
            unsafe { FreeLibraryAndExitThread(HINSTANCE { 0: handle.0 }, 0) };
        }

        let address_finder = AddressFinder {
            base_addr: base,
            module_size: size,
        };

        let present_addr = address_finder.find_addr_present();

        if present_addr == 0 {
            println!("Could not find the address of DirectX11 Present.");
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

        ui::startup_ui_rendering();
        initialize_controls(mainwindow_hwnd);
    });
}

fn detatch() {
    unsafe {
        present_hook.disable().unwrap();

        #[cfg(debug_assertions)]
        FreeConsole().unwrap();
    }
}
