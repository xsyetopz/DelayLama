#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadGesture {
    Down,
    Drag,
    Up,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureResult {
    pub x: f32,
    pub y: f32,
    pub vowel: f32,
    pub note: i32,
    pub note_on_note: i32,
    pub note_off_note: i32,
    pub note_on: bool,
    pub note_off: bool,
}
