/*!
# Executable Manager Module

Handles all executable management functionality for the Nexus addon, including:
- Persistent storage of executable paths
- Launching and stopping processes
- Process tracking and cleanup
- File dialog integration for selecting executables

## Usage Example

```rust
use crate::nexus_addon::manager::ExeManager;
use std::path::PathBuf;

let addon_dir = PathBuf::from("path/to/addon");
let mut manager = ExeManager::new(addon_dir)?;

// Add an executable
manager.add_exe("C:\\Windows\\System32\\notepad.exe".to_string())?;

// Launch an executable
manager.launch_exe("C:\\Windows\\System32\\notepad.exe")?;

// Stop all running executables
manager.stop_all()?;
```

## Error Handling

All fallible operations return `Result<T, NexusError>`. Errors are logged using the `log` crate.

*/

use std::{
    fs::{read_to_string, write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

use crate::nexus_addon::{NexusError, Result};

/**
 * Manages a single executable file and its running process.
 *
 * Stores one executable path, tracks its running process, and provides methods for launching, stopping,
 * and cleaning up the executable. All operations return a `Result<T, NexusError>` for robust error handling.
 */
#[derive(Debug)]
pub struct ExeManager {
    exe_path: Option<String>,
    running_process: Option<Child>,
    addon_dir: PathBuf,
    launch_on_startup: bool,
}

impl ExeManager {
    /**
     * Creates a new ExeManager instance and loads the existing exe list from disk.
     *
     * # Arguments
     * * `addon_dir` - Path to the addon directory containing exes.txt
     *
     * # Errors
     * Returns `NexusError::FileOperation` if loading the exe list fails.
     */
    pub fn new(addon_dir: PathBuf) -> Result<Self> {
        let mut manager = Self {
            exe_path: None,
            running_process: None,
            addon_dir,
            launch_on_startup: false,
        };
        manager.load_exe_path()?;
        Ok(manager)
    }

    /**
     * Loads the executable list from the exes.txt file in the addon directory.
     *
     * # Errors
     * Returns `NexusError::FileOperation` if reading the file fails.
     */
    fn load_exe_path(&mut self) -> Result<()> {
        let mut exes_file = self.addon_dir.clone();
        exes_file.push("exes.txt");

        match read_to_string(&exes_file) {
            Ok(contents) => {
                let lines: Vec<&str> = contents.lines().collect();
                if lines.len() >= 1 {
                    let path = lines[0].trim();
                    if !path.is_empty() {
                        self.exe_path = Some(path.to_string());
                    }
                }
                if lines.len() >= 2 {
                    self.launch_on_startup = lines[1].trim().parse::<bool>().unwrap_or(false);
                }
                log::info!("Loaded executable and launch setting from exe file");
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                log::info!("No existing exe file found, starting with empty path");
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to read exe file from {exes_file:?}: {e}");
                log::error!("{error_msg}");
                Err(NexusError::FileOperation(error_msg))
            }
        }
    }

    /**
     * Saves the current executable list to the exes.txt file.
     *
     * # Errors
     * Returns `NexusError::FileOperation` if writing to the file fails.
     */
    fn save_exe_path(&self) -> Result<()> {
        let mut exes_file = self.addon_dir.clone();
        exes_file.push("exes.txt");

        let exe_content = self.exe_path.clone().unwrap_or_default();
        let content = format!("{}\n{}", exe_content, self.launch_on_startup);
        write(&exes_file, content).map_err(|e| {
            let error_msg = format!("Failed to save exe path to {exes_file:?}: {e}");
            log::error!("{error_msg}");
            NexusError::FileOperation(error_msg)
        })?;

        log::debug!("Saved executable and launch setting to exe file");
        Ok(())
    }

    /**
     * Adds a new executable path to the list and persists it.
     *
     * # Arguments
     * * `path` - Path to the executable file
     *
     * # Errors
     * Returns `NexusError::FileOperation` if the path is empty or saving fails.
     */
    pub fn set_exe(&mut self, path: String) -> Result<()> {
        if path.trim().is_empty() {
            return Err(NexusError::FileOperation(
                "Cannot set empty executable path".to_string(),
            ));
        }
        self.exe_path = Some(path.clone());
        self.save_exe_path()?;
        log::info!("Set executable: {path}");
        Ok(())
    }

    /**
     * Removes an executable from the list by index and stops its process if running.
     *
     * # Arguments
     * * `index` - Index of the executable in the list
     *
     * # Errors
     * Returns `NexusError::FileOperation` if the index is invalid or saving fails.
     */
    pub fn clear_exe(&mut self) -> Result<()> {
        if self.exe_path.is_some() {
            self.stop_exe()?;
            let path = self.exe_path.take();

            // remove from the text file also
            self.save_exe_path()?;

            if let Some(ref p) = path {
                log::info!("Cleared executable: {p}");
            }
        }
        Ok(())
    }

    /**
     * Launches an executable by path.
     *
     * # Arguments
     * * `path` - Path to the executable file
     *
     * # Errors
     * Returns `NexusError::ProcessLaunch` if the process is already running or spawning fails.
     */
    pub fn launch_exe(&mut self) -> Result<()> {
        use std::os::windows::process::CommandExt;

        let path = match &self.exe_path {
            Some(p) => p,
            None => {
                return Err(NexusError::ProcessLaunch(
                    "No executable path set".to_string(),
                ));
            }
        };

        if self.running_process.is_some() {
            return Err(NexusError::ProcessLaunch(format!(
                "Process is already running: {path}"
            )));
        }

        match Command::new(path)
            .creation_flags(0x08000000)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => {
                log::info!("Launched executable: {path}");
                self.running_process = Some(child);
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Failed to launch {path}: {e}");
                log::error!("{error_msg}");
                Err(NexusError::ProcessLaunch(error_msg))
            }
        }
    }

    /**
     * Stops a running executable by path.
     *
     * If no process is running, this is a no-op and returns success.
     *
     * # Arguments
     * * `path` - Path to the executable file
     *
     * # Errors
     * Returns `NexusError::ProcessStop` if killing the process fails.
     */
    pub fn stop_exe(&mut self) -> Result<()> {
        if let Some(mut child) = self.running_process.take() {
            match child.kill() {
                Ok(_) => {
                    log::info!("Stopped executable");

                    // Give the process a moment to fully terminate before returning
                    // This helps prevent race conditions with cleanup threads
                    std::thread::sleep(std::time::Duration::from_millis(100));

                    Ok(())
                }
                Err(e) => {
                    let error_msg = format!("Failed to stop executable: {e}");
                    log::error!("{error_msg}");
                    Err(NexusError::ProcessStop(error_msg))
                }
            }
        } else {
            log::info!("No process running to stop");
            Ok(())
        }
    }

    /**
     * Cleans up finished processes from the running processes map.
     * Should be called periodically to avoid resource leaks.
     */
    pub fn cleanup_finished_process(&mut self) {
        if let Some(child) = &mut self.running_process {
            if let Ok(Some(_)) = child.try_wait() {
                self.running_process = None;
                log::info!("Process finished");
            }
        }
    }

    /**
     * Checks if an executable is currently running.
     *
     * # Arguments
     * * `path` - Path to the executable file
     *
     * # Returns
     * `true` if the process is running, `false` otherwise.
     */
    pub fn is_running(&self) -> bool {
        self.running_process.is_some()
    }

    /**
     * Stops all running executables.
     *
     * # Errors
     * Returns `NexusError::ProcessStop` if any process fails to stop.
     */
    // No stop_all needed for single exe

    /**
     * Gets a reference to the exe paths list.
     *
     * # Returns
     * Reference to the vector of executable paths.
     */
    pub fn exe_path(&self) -> Option<&String> {
        self.exe_path.as_ref()
    }

    /**
     * Gets the number of running processes.
     *
     * # Returns
     * Number of currently running processes.
     */
    pub fn is_process_running(&self) -> bool {
        self.running_process.is_some()
    }

    pub fn launch_on_startup(&mut self) -> &mut bool {
        &mut self.launch_on_startup
    }

    /**
     * Saves the current settings to the exes.txt file.
     *
     * # Errors
     * Returns `NexusError::FileOperation` if writing to the file fails.
     */
    pub fn save_settings(&self) -> Result<()> {
        self.save_exe_path()
    }
}

/// Opens a file dialog to select an executable file
pub fn open_file_dialog() -> Option<String> {
    rfd::FileDialog::new()
        .add_filter("Executable Files", &["exe"])
        .add_filter("All Files", &["*"])
        .set_title("Select Executable")
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

/// Global static reference to the exe manager
pub static EXE_MANAGER: std::sync::OnceLock<Arc<Mutex<ExeManager>>> = std::sync::OnceLock::new();