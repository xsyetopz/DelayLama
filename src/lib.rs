//! Audio synthesizer and plugin package.
//!
//! The root package owns explicit protocol, synthesis, host-coordination, and
//! plugin-framework boundaries. The asset editor is private to the plugin layer.

pub mod host;
mod plugin;
pub mod protocol;
pub mod synthesizer;

pub use plugin::{PluginLogic, PluginParameter, PluginParams};
