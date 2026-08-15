use crate::gesture::{GestureResult, PadGesture};
use delaylama_core::VisualState;

pub const SOURCE_WIDTH: f32 = 360.0;
pub const SOURCE_HEIGHT: f32 = 510.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SourceRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl SourceRect {
    pub const PAD: Self = Self {
        x: 96.0,
        y: 362.0,
        width: 166.0,
        height: 84.0,
    };
    pub const PORTAMENTO: Self = Self {
        x: 21.0,
        y: 448.0,
        width: 50.0,
        height: 50.0,
    };
    pub const DELAY: Self = Self {
        x: 104.0,
        y: 479.0,
        width: 152.0,
        height: 25.0,
    };
    pub const DELAY_HITBOX: Self = Self {
        x: 94.0,
        y: 469.0,
        width: 172.0,
        height: 45.0,
    };
    pub const VOICE: Self = Self {
        x: 293.0,
        y: 447.0,
        width: 50.0,
        height: 50.0,
    };
    pub const HELP: Self = Self {
        x: 284.0,
        y: 300.0,
        width: 43.0,
        height: 35.0,
    };

    pub fn contains(self, point: (f32, f32)) -> bool {
        point.0 >= self.x
            && point.0 <= self.x + self.width
            && point.1 >= self.y
            && point.1 <= self.y + self.height
    }

    fn contains_ellipse(self, point: (f32, f32)) -> bool {
        let radius_x = self.width * 0.5;
        let radius_y = self.height * 0.5;
        let normalized_x = (point.0 - (self.x + radius_x)) / radius_x;
        let normalized_y = (point.1 - (self.y + radius_y)) / radius_y;
        normalized_x * normalized_x + normalized_y * normalized_y <= 1.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewTransform {
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}

impl ViewTransform {
    pub fn fit(view_width: f32, view_height: f32) -> Self {
        let scale = (view_width / SOURCE_WIDTH)
            .min(view_height / SOURCE_HEIGHT)
            .max(f32::EPSILON);
        Self {
            scale,
            offset_x: (view_width - SOURCE_WIDTH * scale) * 0.5,
            offset_y: (view_height - SOURCE_HEIGHT * scale) * 0.5,
        }
    }

    pub fn source_to_view(self, point: (f32, f32)) -> (f32, f32) {
        (
            self.offset_x + point.0 * self.scale,
            self.offset_y + point.1 * self.scale,
        )
    }

    pub fn view_to_source(self, point: (f32, f32)) -> (f32, f32) {
        (
            (point.0 - self.offset_x) / self.scale,
            (point.1 - self.offset_y) / self.scale,
        )
    }

    pub fn scale(self) -> f32 {
        self.scale
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HitTarget {
    Pad,
    Portamento,
    Delay,
    Voice,
    Help,
}

#[derive(Clone, Debug)]
pub struct EditorModel {
    gate: bool,
    vowel: f32,
    frame: usize,
}

impl Default for EditorModel {
    fn default() -> Self {
        Self {
            gate: false,
            vowel: 0.5,
            frame: 5,
        }
    }
}

impl EditorModel {
    pub fn hit_test(source_point: (f32, f32)) -> Option<HitTarget> {
        if SourceRect::PAD.contains(source_point) {
            Some(HitTarget::Pad)
        } else if SourceRect::PORTAMENTO.contains_ellipse(source_point) {
            Some(HitTarget::Portamento)
        } else if SourceRect::DELAY_HITBOX.contains(source_point) {
            Some(HitTarget::Delay)
        } else if SourceRect::VOICE.contains_ellipse(source_point) {
            Some(HitTarget::Voice)
        } else if SourceRect::HELP.contains(source_point) {
            Some(HitTarget::Help)
        } else {
            None
        }
    }

    pub fn pad_position(source_point: (f32, f32)) -> (f32, f32) {
        (
            ((source_point.0 - SourceRect::PAD.x) / SourceRect::PAD.width).clamp(0.0, 1.0),
            ((source_point.1 - SourceRect::PAD.y) / SourceRect::PAD.height).clamp(0.0, 1.0),
        )
    }

    pub fn linear_value(source_x: f32, bounds: SourceRect) -> f32 {
        ((source_x - bounds.x) / bounds.width).clamp(0.0, 1.0)
    }

    pub fn rotary_value(origin: f32, view_delta: (f32, f32)) -> f32 {
        (origin + (view_delta.0 - view_delta.1) / 250.0).clamp(0.0, 1.0)
    }
    pub fn handle_gesture(&mut self, x: f32, y: f32, g: PadGesture) -> GestureResult {
        let x = x.clamp(0.0, 1.0);
        let y = y.clamp(0.0, 1.0);
        self.vowel = 1.0 - y;
        let (on, off) = match g {
            PadGesture::Down => (true, false),
            PadGesture::Up => (false, true),
            PadGesture::Drag => (false, false),
        };
        if on {
            self.gate = true;
        }
        if off {
            self.gate = false;
        }
        self.frame = Self::select_visual_state(28, self.gate, self.vowel);
        GestureResult {
            x,
            y,
            vowel: self.vowel,
            note: 40,
            note_on_note: if on { 40 } else { -1 },
            note_off_note: if off { 40 } else { -1 },
            note_on: on,
            note_off: off,
        }
    }

    pub fn apply_external_state(&mut self, s: VisualState) {
        self.gate = s.gate && (4..=72).contains(&s.note);
        self.vowel = s.vowel.clamp(0.0, 1.0);
        let selector = if !self.gate {
            5.0 / 30.0
        } else if s.atlas_selector.is_finite() && s.atlas_selector >= 0.0 {
            s.atlas_selector.clamp(0.0, 1.0)
        } else {
            0.2 + 0.8 * self.vowel
        };
        self.frame = (selector * 29.0 + 0.5).min(29.0) as usize;
    }

    pub fn select_visual_state(note: i32, gate: bool, vowel: f32) -> usize {
        if gate && (4..=72).contains(&note) {
            ((0.2 + 0.8 * vowel.clamp(0.0, 1.0)) * 29.0).round() as usize
        } else {
            5
        }
    }

    pub fn animation_frame(&self) -> usize {
        self.frame
    }
}
