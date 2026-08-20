//! Public data types exchanged by the synthesis stages.

/// Lowest sample rate accepted by the engine.
pub const MIN_SAMPLE_RATE: f64 = 8_000.0;
/// Highest sample rate accepted by the engine.
pub const MAX_SAMPLE_RATE: f64 = 384_000.0;
/// Sample rate used before the host prepares the engine.
pub const DEFAULT_SAMPLE_RATE: f64 = 44_100.0;

/// Synthesis settings supplied by the host.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// Vowel position in the range `0..=1`.
    pub vowel: f32,
    /// Portamento time in the range `0..=1`.
    pub port_time: f32,
    /// Delay mix in the range `0..=1`.
    pub delay_mix: f32,
    /// Voice character in the range `0..=1`.
    pub voice: f32,
    /// Vibrato amount in the range `0..=1`.
    pub vibrato: f32,
    /// Output volume in the range `0..=1`.
    pub volume: f32,
    /// Pad horizontal pitch routing in the range `0..=1`.
    pub xy_routing: f32,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            vowel: 0.5,
            port_time: 0.5,
            delay_mix: 0.8,
            voice: 0.5,
            vibrato: 0.0,
            volume: 0.1,
            xy_routing: 0.0,
        }
    }
}

/// Current monophonic voice state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VoiceState {
    /// Current internal note, or `-1` while idle.
    pub current_note: i32,
    /// Whether a note is held.
    pub gate: bool,
}

/// Current pad position, vowel value, and held state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadState {
    /// Horizontal pad pitch modulation in the range `0..=1`.
    pub pitch_modulation: f32,
    /// Pad vowel position in the range `0..=1`.
    pub vowel: f32,
    /// Whether the editor pad is held.
    pub active: bool,
}

impl Default for PadState {
    fn default() -> Self {
        Self {
            pitch_modulation: 0.5,
            vowel: 0.5,
            active: false,
        }
    }
}
