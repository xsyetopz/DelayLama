//! Host-facing processor model that connects editor gestures to synthesis.
#![deny(unsafe_code)]
/// Host lifecycle, persistence, and editor-to-synth coordination.
mod processor;
pub use processor::ProcessorModel;
