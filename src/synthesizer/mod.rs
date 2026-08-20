//! Stateful synthesis engine and its public data contracts.

mod engine;
mod state;
pub mod tables;

pub use engine::SynthEngine;
pub use state::{
    DEFAULT_SAMPLE_RATE, MAX_SAMPLE_RATE, MIN_SAMPLE_RATE, PadState, Parameters, VoiceState,
};
pub use tables::{excitation, formant_curve, formant_tables, frequency_table, sine_table, window};
