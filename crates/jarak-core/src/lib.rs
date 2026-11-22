//! Jarak's document model and editing engine (docs/JARAK_PHILOSOPHY.md §5, ADR 0102).
//!
//! The core knows nothing about windows, input devices, the clipboard or a GUI toolkit; the
//! headless CLI (`jarak.exe`) and the window (`jarak-gui.exe`) are built on it.
//!
//! - Step 85: the per-user locations ([`paths`]) and the command-line open targets ([`target`]).
//! - Step 86: the text model — positions and graphemes over a rope ([`text`]), carets and
//!   selections ([`selection`]), transactions with undo/redo ([`history`]), the [`document`] that
//!   edits only through them, and the command registry ([`command`]).
//! - Step 87: files ([`file`]) — encodings, line endings, the lossless check, atomic saves, and
//!   the [`file::FileDocument`] that ties a document to its path and format.
//! - Step 89: the Markdown render model ([`markdown`]): blocks and inlines with their source
//!   lines, drawn natively by the window.
//! - Step 90: pasted images as portable assets next to the document ([`assets`]).
//! - Step 92: Vim-style modal editing over the same document operations ([`vim`]).
//! - Step 93: the session and the recent files ([`session`]).
//! - Step 94: find and replace in a document ([`search`]); markers on the document.

pub mod assets;
pub mod assist;
pub mod command;
pub mod document;
pub mod file;
pub mod history;
pub mod infiles;
pub mod lineops;
pub mod macros;
pub mod markdown;
pub mod paths;
pub mod recovery;
pub mod search;
pub mod selection;
pub mod session;
pub mod settings;
pub mod structure;

/// The text type of a document (Step 99: the window hands a copy to a worker thread).
pub use ropey::Rope;
pub mod syntax;
pub mod target;
pub mod text;
pub mod vim;

/// Jarak's version (the whole `jarak` group ships under one version, ADR 0097).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
