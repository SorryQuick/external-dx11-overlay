use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    sync::{OnceLock, atomic::Ordering},
};

use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};

use crate::{
    debug::{
        DEBUG_FEATURES,
        debug_overlay::{clear_debug_overlay, refresh_overlay_buffer},
        dump_debug_data, restart_blish,
    },
    ui::FRAME_BUFFER,
};

//Handle keybinds and custom keybinds
#[derive(Eq, Hash, PartialEq, Debug)]
pub struct KeyBind {
    key: u32,
    ctrl: bool,
    alt: bool,
    shift: bool,
}
pub static KEYBINDS: OnceLock<HashMap<KeyBind, fn()>> = OnceLock::new();

pub fn init_keybinds() {
    let path = "addons/LOADER_public/keybinds.conf";
    let map = if std::path::Path::new(path).exists() {
        load_keybinds(path)
    } else {
        dump_default_keybinds(path);
        load_keybinds(path)
    };

    KEYBINDS.set(map).unwrap();
}

fn dump_default_keybinds(path: &str) {
    let file = File::create(path).expect("Failed to create keybinds file");
    let mut writer = BufWriter::new(file);

    let defaults = vec![
        ("Ctrl+Alt+P", "dump_debug_data"),
        ("Ctrl+Alt+O", "restart_blish"),
        ("Ctrl+Alt+B", "toggle_rendering"),
        ("Ctrl+Alt+N", "toggle_processing"),
        ("Ctrl+Alt+D", "toggle_debug_overlay"),
    ];

    for (combo, action) in defaults {
        writeln!(writer, "{} {}", combo, action).ok();
    }
}

//Parses a line / keybind from the keybinds file.
fn parse_keybind_line(line: &str) -> Option<(KeyBind, fn())> {
    let mut parts = line.split_whitespace();
    let combo = parts.next()?;
    let action_name = parts.next()?;

    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let key_char: u32;

    let combo = combo.split('+').collect::<Vec<_>>();
    key_char = combo.last()?.chars().next()? as u32;
    for part in &combo[..combo.len() - 1] {
        match part.to_lowercase().as_str() {
            "ctrl" => ctrl = true,
            "alt" => alt = true,
            "shift" => shift = true,
            _ => {}
        }
    }

    Some((
        KeyBind {
            key: key_char,
            ctrl,
            alt,
            shift,
        },
        action_from_name(action_name),
    ))
}

//Loads keybinds from the config file
fn load_keybinds(path: &str) -> HashMap<KeyBind, fn()> {
    let file = File::open(path).expect("Failed to open keybinds file");
    let reader = BufReader::new(file);
    let mut map = HashMap::new();
    reader.lines().for_each(|l| {
        if let Ok(line) = l {
            if let Some((keybind, action)) = parse_keybind_line(&line) {
                map.insert(keybind, action);
            }
        }
    });
    map
}

fn action_from_name(name: &str) -> fn() {
    match name {
        "dump_debug_data" => dump_debug_data as fn(),
        "restart_blish" => restart_blish as fn(),
        "toggle_rendering" => toggle_rendering_action as fn(),
        "toggle_processing" => toggle_processing_action as fn(),
        "toggle_debug_overlay" => toggle_debug_overlay as fn(),
        _ => panic!("Unknown action: {}", name),
    }
}

//Gets a KeyBind struct from wparam
pub fn get_current_keybind(wparam: u32) -> KeyBind {
    let ctrl_pressed = (unsafe { GetKeyState(VK_CONTROL.0 as i32) } as u16 & 0x8000) != 0;
    let alt_pressed = (unsafe { GetKeyState(VK_MENU.0 as i32) } as u16 & 0x8000) != 0;
    let shift_pressed = (unsafe { GetKeyState(VK_SHIFT.0 as i32) } as u16 & 0x8000) != 0;

    KeyBind {
        key: wparam,
        ctrl: ctrl_pressed,
        alt: alt_pressed,
        shift: shift_pressed,
    }
}

fn toggle_rendering_action() {
    log::info!("Rendering toggled.");
    DEBUG_FEATURES.rendering_enabled.store(
        !DEBUG_FEATURES.rendering_enabled.load(Ordering::Relaxed),
        Ordering::Relaxed,
    );
}
fn toggle_processing_action() {
    log::info!("Processing toggled.");
    DEBUG_FEATURES.processing_enabled.store(
        !DEBUG_FEATURES.processing_enabled.load(Ordering::Relaxed),
        Ordering::Relaxed,
    );
}
fn toggle_debug_overlay() {
    let old = DEBUG_FEATURES.debug_overlay_enabled.load(Ordering::Relaxed);
    DEBUG_FEATURES
        .debug_overlay_enabled
        .store(!old, Ordering::Relaxed);
    //Need to clear the overlay
    if old == true {
        if let Some(buf) = FRAME_BUFFER.get() {
            let mut frame = buf.lock().unwrap();
            let width = frame.width;
            clear_debug_overlay(&mut frame.pixels, width);
        }
    } else {
        refresh_overlay_buffer();
    }
    log::info!("Debug overlay toggled.");
}
