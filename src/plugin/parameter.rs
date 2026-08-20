//! Parameter names, IDs, and defaults.

/// Parameter exposed to the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginParameter {
    /// Vowel/formant position.
    Vowel,
    /// Portamento time.
    Portamento,
    /// Delay wet/dry mix.
    Delay,
    /// Voice character.
    Voice,
}

impl PluginParameter {
    /// All host parameters in ID order.
    pub const ALL: [Self; 4] = [Self::Vowel, Self::Portamento, Self::Delay, Self::Voice];

    const fn ordinal(self) -> u8 {
        match self {
            Self::Vowel => 0,
            Self::Portamento => 1,
            Self::Delay => 2,
            Self::Voice => 3,
        }
    }

    /// Returns the host parameter ID.
    pub fn id(self) -> u32 {
        u32::from(self.ordinal())
    }

    /// Returns the position in the parameter array.
    pub fn index(self) -> usize {
        usize::from(self.ordinal())
    }

    /// Finds a parameter from its host ID.
    pub const fn from_id(id: u32) -> Option<Self> {
        match id {
            0 => Some(Self::Vowel),
            1 => Some(Self::Portamento),
            2 => Some(Self::Delay),
            3 => Some(Self::Voice),
            _ => None,
        }
    }

    /// Returns the field name used in presets and parameter data.
    pub const fn field_name(self) -> &'static str {
        match self {
            Self::Vowel => "vowel",
            Self::Portamento => "port_time",
            Self::Delay => "delay_mix",
            Self::Voice => "voice",
        }
    }

    /// Returns the name shown by the host.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Vowel => "Vowel",
            Self::Portamento => "Portamento",
            Self::Delay => "Delay",
            Self::Voice => "Voice",
        }
    }

    /// Returns the default value between zero and one.
    pub const fn default(self) -> f64 {
        match self {
            Self::Vowel | Self::Portamento | Self::Voice => 0.5,
            Self::Delay => 0.8,
        }
    }
}
