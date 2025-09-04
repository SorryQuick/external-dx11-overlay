/*!
# Nexus Addon Module

This module serves as the central entry point for all Nexus-specific functionality in the Guild Wars 2 overlay addon.
It organizes the codebase into logical submodules for executable management, UI rendering, and initialization/cleanup.

## Usage

Enable the `nexus` feature in your Cargo.toml to activate Nexus integration:

```toml
[features]
nexus = []
```

Import the main entry points for addon lifecycle management:

```rust
use crate::nexus_addon::{nexus_load, nexus_unload};
```

## Error Handling

All errors across Nexus modules use the [`NexusError`](src/nexus_addon/mod.rs:9) enum for consistent propagation and logging.
Use the provided `Result<T>` type alias for fallible operations.

## Modules

- [`manager`](src/nexus_addon/manager.rs): Executable management logic
- [`ui`](src/nexus_addon/ui.rs): UI rendering components
- [`init`](src/nexus_addon/init.rs): Initialization and cleanup routines

*/

pub mod init;
pub mod manager;
pub mod ui;

pub use init::{nexus_load, nexus_unload};

/// Consistent error types for the nexus addon
#[derive(Debug)]
pub enum NexusError {
    ManagerInitialization(String),
    ProcessLaunch(String),
    ProcessStop(String),
    FileOperation(String),
    ResourceLoading(String),
}

impl std::fmt::Display for NexusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NexusError::ManagerInitialization(msg) => {
                write!(f, "Manager initialization error: {msg}")
            }
            NexusError::ProcessLaunch(msg) => write!(f, "Process launch error: {msg}"),
            NexusError::ProcessStop(msg) => write!(f, "Process stop error: {msg}"),
            NexusError::FileOperation(msg) => write!(f, "File operation error: {msg}"),
            NexusError::ResourceLoading(msg) => write!(f, "Resource loading error: {msg}"),
        }
    }
}

impl std::error::Error for NexusError {}

/// Type alias for Results using NexusError
pub type Result<T> = std::result::Result<T, NexusError>;
