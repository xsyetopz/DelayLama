//! Normalized event application and note-source arbitration.

use num_traits::ToPrimitive;

use super::{
    CC_DELAY_MIX, CC_PORT_TIME, CC_VIBRATO, CC_VOICE, CC_VOLUME, CC_XY_ROUTING, CC7_VOLUME_SCALE,
    PAD_INTERNAL_NOTE, SynthEngine, SynthesisEvent, SynthesisEventKind, clamp,
};

impl SynthEngine {
    pub(super) fn apply(&mut self, event: SynthesisEvent) {
        match event.kind {
            SynthesisEventKind::NoteOn => self.apply_note_on(event),
            SynthesisEventKind::NoteOff => self.apply_note_off(event),
            SynthesisEventKind::PitchBend => self.apply_pitch_bend(event.value),
            SynthesisEventKind::PadPitch => self.apply_pad_pitch(event.value),
            SynthesisEventKind::PadVowel => self.apply_pad_vowel(event.value),
            SynthesisEventKind::ControlChange => {
                self.apply_control_change(event.controller, event.value);
            }
        }
    }

    fn apply_note_on(&mut self, event: SynthesisEvent) {
        if event.local_pad && event.note == PAD_INTERNAL_NOTE {
            self.apply_pad_note_on(event.value);
            return;
        }
        if event.value > 0.0 {
            self.release_local_pad();
            let newly_held = self.mark_external_note(event.note, true);
            if newly_held {
                self.note_on(event.note);
            }
        } else {
            self.mark_external_note(event.note, false);
            self.note_off(event.note);
        }
    }

    fn apply_pad_note_on(&mut self, value: f32) {
        self.notes.pad.active = value > 0.0;
        let external_voice_held = self.notes.external_active.iter().any(|held| *held);
        if value > 0.0 && !external_voice_held {
            if !self.notes.local_pad_active {
                self.notes.local_pad_active = true;
                self.note_on(PAD_INTERNAL_NOTE);
                // The pad note is lifecycle metadata, not an audible fixed carrier.
                // Start the one voice at the pad route before the first grain is queued.
                self.controls.route.current =
                    12.0_f32.mul_add(self.notes.pad.pitch_modulation, 36.0);
                self.controls.route.target = self.controls.route.current;
                self.controls.route.increment = 0.0;
                self.controls.route.steps_remaining = 0;
                self.controls.pitch.current = self.controls.route.current.to_f64().unwrap_or(0.0);
                self.controls.pitch.target = self.controls.pitch.current;
            }
        } else if value <= 0.0 && self.notes.local_pad_active {
            self.notes.local_pad_active = false;
            self.note_off(PAD_INTERNAL_NOTE);
        }
    }

    fn release_local_pad(&mut self) {
        if self.notes.local_pad_active {
            self.notes.local_pad_active = false;
            self.note_off(PAD_INTERNAL_NOTE);
        }
    }

    fn mark_external_note(&mut self, note: i32, active: bool) -> bool {
        let Some(index) = note.to_usize() else {
            return true;
        };
        let Some(held) = self.notes.external_active.get_mut(index) else {
            return true;
        };
        let newly_held = !*held;
        *held = active;
        newly_held
    }

    fn apply_note_off(&mut self, event: SynthesisEvent) {
        if event.local_pad && event.note == PAD_INTERNAL_NOTE {
            self.notes.pad.active = false;
            if self.notes.local_pad_active {
                self.notes.local_pad_active = false;
                self.note_off(event.note);
            }
            return;
        }
        self.mark_external_note(event.note, false);
        self.note_off(event.note);
    }

    fn apply_pitch_bend(&mut self, value: f32) {
        let target = if value > 1.0 {
            value.clamp(0.0, 16_383.0) / 16_384.0
        } else {
            value.clamp(0.0, 1.0)
        };
        self.controls.bend.target = target;
        self.controls.bend.increment = (target - self.controls.bend.current) / 10.0;
        self.controls.bend.steps_remaining = 10;
    }

    fn apply_pad_pitch(&mut self, value: f32) {
        self.notes.pad.pitch_modulation = clamp(value, 0.5);
        self.notes.pad.active = true;
        self.controls.route.target = 12.0_f32.mul_add(self.notes.pad.pitch_modulation, 36.0);
        self.controls.route.increment =
            (self.controls.route.target - self.controls.route.current) / 10.0;
        self.controls.route.steps_remaining = 10;
    }

    fn apply_pad_vowel(&mut self, value: f32) {
        self.notes.pad.vowel = clamp(value, 0.5);
        self.notes.pad.active = true;
        self.controls.bend.target = self.notes.pad.vowel;
        self.controls.bend.increment =
            (self.controls.bend.target - self.controls.bend.current) / 10.0;
        self.controls.bend.steps_remaining = 10;
    }

    fn apply_control_change(&mut self, controller: i32, value: f32) {
        let value = if value > 1.0 {
            value / 127.0
        } else {
            value.clamp(0.0, 1.0)
        };
        match controller {
            CC_VIBRATO => self.parameters.vibrato = value,
            CC_PORT_TIME => self.parameters.port_time = value,
            CC_VOLUME => self.parameters.volume = value * CC7_VOLUME_SCALE,
            CC_XY_ROUTING => {
                self.controls.route.target = 12.0_f32.mul_add(value, 36.0);
                self.controls.route.increment =
                    (self.controls.route.target - self.controls.route.current) / 10.0;
                self.controls.route.steps_remaining = 10;
            }
            CC_DELAY_MIX => self.parameters.delay_mix = value,
            CC_VOICE => {
                self.parameters.voice = value;
                self.grain.dirty = true;
            }
            _ => {}
        }
    }
}
