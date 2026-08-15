//! Pointer gesture results produced by the editor surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Pointer operation applied to the editor pad.
pub enum PadGesture {
    /// Begins a pad gesture.
    Down,
    /// Updates a held pad gesture.
    Drag,
    /// Ends a pad gesture.
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Normalized pad values and lifecycle changes produced by a gesture.
pub struct GestureResult {
    /// Horizontal pad position.
    pub x: f32,
    /// Vertical pad position.
    pub y: f32,
    /// Inverted vertical position used for vowel selection.
    pub vowel: f32,
    /// Logical note represented by the pad.
    pub note: i32,
    /// Note emitted when the gesture starts, or `-1`.
    pub note_on_note: i32,
    /// Note emitted when the gesture ends, or `-1`.
    pub note_off_note: i32,
    /// Whether the gesture starts a note.
    pub note_on: bool,
    /// Whether the gesture ends a note.
    pub note_off: bool,
}
