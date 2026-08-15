//! Truce plugin integration and raw editor implementation.
#![deny(unsafe_code)]
/// Framework-owned plugin export declarations.
mod exports;
/// Asset-backed native editor implementation.
mod raw_editor;
/// Real-time Truce lifecycle adapter and editor layout.
mod runtime;

pub use runtime::{PluginLogic, PluginParams};
