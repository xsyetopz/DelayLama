//! Export declarations for the supported plugin formats.

use truce::prelude::*;

truce::plugin! {
    logic: crate::plugin::runtime::PluginLogic,
    params: crate::plugin::params::PluginParams,
}
