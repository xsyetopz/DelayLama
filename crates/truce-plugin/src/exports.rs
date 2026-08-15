//! Framework-owned export declarations for supported plugin formats.

use truce::prelude::*;

truce::plugin! { logic: crate::runtime::PluginLogic, params: crate::runtime::PluginParams }
