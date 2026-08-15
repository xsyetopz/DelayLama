//! Editor geometry and gesture interpretation for the Delay Lama interface.
#![deny(unsafe_code)]
/// Pointer gesture data emitted by the interaction surface.
mod gesture;
/// Source-coordinate interaction rules and visual-state projection.
mod interaction;
pub use delaylama_editor_assets::Artwork;
pub use gesture::{GestureResult, PadGesture};
pub use interaction::{
    EditorModel, HitTarget, SOURCE_HEIGHT, SOURCE_WIDTH, SourceRect, ViewTransform,
};
