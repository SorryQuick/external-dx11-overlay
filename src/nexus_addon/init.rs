/*!
# Nexus Addon Initialization Module

Handles initialization, resource loading, and cleanup for the Nexus addon.
Provides the main entry points for the addon lifecycle and orchestrates setup of UI, keybinds, quick access, and textures.

## Usage

Call the following functions from your main entry points:

```rust
use crate::nexus_addon::{nexus_load, nexus_unload};

#[cfg(feature = "nexus")]
nexus_load();

#[cfg(feature = "nexus")]
nexus_unload();
```

All initialization steps are performed with error handling and logging. Resources are registered and cleaned up automatically.

## Lifecycle

- `nexus_load`: Initializes all Nexus-specific functionality and resources
- `nexus_unload`: Cleans up resources and stops all running processes

*/

#[cfg(feature = "nexus")]
use nexus::{
    keybind::register_keybind_with_string,
    keybind_handler,
    paths::get_addon_dir,
    quick_access::add_quick_access,
    texture::{RawTextureReceiveCallback, load_texture_from_memory},
    texture_receive,
};

#[cfg(feature = "nexus")]
use windows::Win32::{Foundation::HINSTANCE, System::LibraryLoader::GetModuleHandleW};

#[cfg(feature = "nexus")]
use crate::nexus_addon::{NexusError, Result, manager::ExeManager, ui};

/// Returns the HMODULE and casts it into HINSTANCE
/// On modern systems, HMODULE is pretty much the same as HINSTANCE, and can be safely cast
#[cfg(feature = "nexus")]
fn get_hinstance() -> HINSTANCE {
    unsafe { GetModuleHandleW(None).unwrap().into() }
}

/// Nexus addon load function - handles initialization of all nexus-specific functionality
#[cfg(feature = "nexus")]
pub fn nexus_load() {
    log::info!("Loading Blish HUD overlay loader addon");

    if let Err(e) = initialize_nexus_addon() {
        log::error!("Failed to initialize nexus addon: {e}");
        return;
    }

    log::info!("Blish HUD overlay loader addon loaded successfully");
}

/// Internal initialization function with proper error handling
#[cfg(feature = "nexus")]
fn initialize_nexus_addon() -> Result<()> {
    // Initialize the nexus menus and options
    // Create the addon dir if it doesn't exist
    use std::fs;

    let addon_dir = get_addon_dir("LOADER_public").ok_or_else(|| {
        NexusError::ManagerInitialization("Failed to get addon directory".to_string())
    })?;

    fs::create_dir_all(&addon_dir).map_err(|e| {
        NexusError::ManagerInitialization(format!("Failed to create addon directory: {e}"))
    })?;

    // Initialize the exe manager
    let exe_manager = std::sync::Arc::new(std::sync::Mutex::new(ExeManager::new(addon_dir)?));

    crate::nexus_addon::manager::EXE_MANAGER
        .set(exe_manager.clone())
        .map_err(|_| {
            NexusError::ManagerInitialization("Failed to set global exe manager".to_string())
        })?;

    // Launch exe on startup if enabled
    {
        let mut manager = exe_manager.lock().map_err(|e| {
            NexusError::ManagerInitialization(format!("Failed to lock exe manager: {e}"))
        })?;
        if *manager.launch_on_startup() {
            if let Err(e) = manager.launch_exe() {
                log::error!("Failed to launch exe on startup: {e}");
            }
        }
    }

    // Load textures for the addon
    load_addon_textures()?;

    // Setup quick access menu
    setup_quick_access()?;

    // Setup keybinds
    setup_keybinds()?;

    // Setup UI rendering
    ui::setup_main_window_rendering();

    // Start the main DLL functionality
    let hinstance = get_hinstance();
    log::info!("Loading via Nexus - HMODULE/HINSTANCE: {}", hinstance.0);
    crate::attach(hinstance);

    Ok(())
}

/// Loads the addon textures from embedded resources
#[cfg(feature = "nexus")]
fn load_addon_textures() -> Result<()> {
    let icon = include_bytes!("./images/64p_nexus_blish_loader.png");
    let icon_hover = include_bytes!("./images/64p_nexus_blish_loader.png");

    let receive_texture: RawTextureReceiveCallback = texture_receive!(|id, _texture| {
        log::info!("texture {id} loaded");
    });

    // Note: load_texture_from_memory doesn't return a Result, so we assume success
    // In a real implementation, we might want to add validation
    load_texture_from_memory("BLISH_OVERLAY_LOADER_ICON", icon, Some(receive_texture));
    load_texture_from_memory(
        "BLISH_OVERLAY_LOADER_ICON_HOVER",
        icon_hover,
        Some(receive_texture),
    );

    log::info!("Addon textures loaded successfully");
    Ok(())
}

/// Sets up the quick access menu entry
#[cfg(feature = "nexus")]
fn setup_quick_access() -> Result<()> {
    // Note: add_quick_access doesn't return a Result, so we assume success
    // In a real implementation, we might want to add validation
    add_quick_access(
        "BLISH_OVERLAY_LOADER_SHORTCUT",
        "BLISH_OVERLAY_LOADER_ICON",
        "BLISH_OVERLAY_LOADER_ICON_HOVER",
        "BLISH_OVERLAY_LOADER_KEYBIND",
        "Blish HUD overlay loader",
    )
    .revert_on_unload();

    log::info!("Quick access menu setup successfully");
    Ok(())
}

/// Sets up the keybind handlers
#[cfg(feature = "nexus")]
fn setup_keybinds() -> Result<()> {
    let main_window_keybind_handler = keybind_handler!(|id, is_release| {
        log::info!(
            "keybind {id} {}",
            if is_release { "released" } else { "pressed" }
        );
        if !is_release {
            ui::toggle_window();
        }
    });

    // Note: register_keybind_with_string doesn't return a Result, so we assume success
    // In a real implementation, we might want to add validation
    register_keybind_with_string(
        "BLISH_OVERLAY_LOADER_KEYBIND",
        main_window_keybind_handler,
        "ALT+SHIFT+1",
    )
    .revert_on_unload();

    log::info!("Keybinds setup successfully");
    Ok(())
}

/// Nexus addon unload function - handles cleanup of all nexus-specific functionality
#[cfg(feature = "nexus")]
pub fn nexus_unload() {
    log::info!("Unloading Blish HUD overlay loader addon");

    if let Err(e) = cleanup_nexus_addon() {
        log::error!("Error during nexus addon cleanup: {e}");
    }

    log::info!("Blish HUD overlay loader addon unloaded");
}

/// Internal cleanup function with proper error handling
#[cfg(feature = "nexus")]
fn cleanup_nexus_addon() -> Result<()> {
    // Stop all running executables before unloading
    if let Some(exe_manager_arc) = crate::nexus_addon::manager::EXE_MANAGER.get() {
        let mut exe_manager = exe_manager_arc.lock().map_err(|e| {
            NexusError::ManagerInitialization(format!(
                "Failed to lock exe manager during cleanup: {e}"
            ))
        })?;
        exe_manager.stop_exe()?;
    }

    // Perform main cleanup
    crate::detatch();

    log::info!("Nexus addon cleanup completed successfully");
    Ok(())
}