//! Stateful DSP engine for the Delay Lama monophonic synthesizer.
#![deny(unsafe_code)]
/// Stateful grain, modulation, and delay rendering.
mod engine;
/// Engine state exported to host and editor consumers.
mod state;
/// Lookup-table construction for synthesis preparation.
pub mod tables;

pub use delaylama_protocol::{SynthesisEvent, SynthesisEventKind};
pub use engine::SynthEngine;
pub use state::*;
pub use tables::{excitation, formant_curve, formant_tables, frequency_table, sine_table, window};
