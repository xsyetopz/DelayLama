//! Shared adapter contract. No format-specific symbols belong here.

/// Compile-time description of a truce export target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Adapter {
    pub name: &'static str,
    pub enabled: bool,
}

impl Adapter {
    pub const fn new(name: &'static str, enabled: bool) -> Self {
        Self { name, enabled }
    }

    pub const fn formats() -> [Self; 5] {
        [
            formats::CLAP,
            formats::VST3,
            formats::AUv2,
            formats::AUv3,
            formats::LV2,
        ]
    }
}

/// Format descriptors owned by the adapter capability.
mod formats {
    use super::Adapter;

    pub const CLAP: Adapter = Adapter::new("clap", cfg!(feature = "clap"));
    pub const VST3: Adapter = Adapter::new("vst3", cfg!(feature = "vst3"));
    pub const AUv2: Adapter = Adapter::new("auv2", cfg!(feature = "auv2"));
    pub const AUv3: Adapter = Adapter::new("auv3", cfg!(feature = "auv3"));
    pub const LV2: Adapter = Adapter::new("lv2", cfg!(feature = "lv2"));
}
