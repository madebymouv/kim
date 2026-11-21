//! Jarak's document model and editing engine (docs/JARAK_PHILOSOPHY.md §5, ADR 0102).
//!
//! The core knows nothing about windows, input devices, the clipboard or a GUI toolkit; the
//! headless CLI (`jarak.exe`) and the window (`jarak-gui.exe`) are built on it. Step 85 gives it
//! the per-user locations and the command-line open targets; the text model follows in Step 86.

pub mod paths;
pub mod target;

/// Jarak's version (the whole `jarak` group ships under one version, ADR 0097).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
