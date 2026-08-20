//! Voice ownership, modulation, and editor-atlas progression.

use num_traits::ToPrimitive;

use super::{SynthEngine, VoiceState};

impl SynthEngine {
    pub(super) fn note_on(&mut self, note: i32) {
        if !(4..=72).contains(&note) || self.notes.stack.contains(&note) {
            return;
        }
        if self.notes.stack.get(127).copied().unwrap_or(-1) < 0 {
            for index in (1..128).rev() {
                let previous = self.notes.stack.get(index - 1).copied().unwrap_or(-1);
                if let Some(slot) = self.notes.stack.get_mut(index) {
                    *slot = previous;
                }
            }
        }
        if let Some(slot) = self.notes.stack.get_mut(0) {
            *slot = note;
        }
        self.update_voice();
    }

    pub(super) fn note_off(&mut self, note: i32) {
        if let Some(index) = self.notes.stack.iter().position(|&held| held == note) {
            for next in index..127 {
                let following = self.notes.stack.get(next + 1).copied().unwrap_or(-1);
                if let Some(slot) = self.notes.stack.get_mut(next) {
                    *slot = following;
                }
            }
            if let Some(slot) = self.notes.stack.get_mut(127) {
                *slot = -1;
            }
        }
        self.update_voice();
    }

    pub(super) fn update_voice(&mut self) {
        let note = self.notes.stack.first().copied().unwrap_or(-1);
        let was_gated = self.notes.voice.gate;
        self.notes.voice = VoiceState {
            current_note: note,
            gate: note >= 0,
        };
        if note < 0 {
            self.controls.pitch.legato = false;
            // Grains are queued ahead in the overlap-add ring. Once the last note is
            // released they no longer belong to a live voice; leaving them queued
            // makes the previous pitch sound alongside the next pad gesture.
            self.grain.overlap.ring.fill(0.0);
            self.grain.overlap.samples_since_grain = 0;
            self.atlas.selector = 5.0 / 30.0;
            self.atlas.timing.idle_elapsed = 0;
            self.atlas.timing.tick_counter = 0;
            self.atlas.idle_index = 0;
            self.atlas.dirty = true;
            return;
        }
        self.controls.pitch.target = note.to_f64().unwrap_or(0.0);
        if was_gated {
            self.controls.pitch.legato = true;
        } else {
            self.controls.pitch.current = self.controls.pitch.target;
            self.controls.pitch.legato = false;
        }
    }

    pub(super) fn advance_atlas_state(&mut self) {
        if self.notes.voice.gate {
            self.atlas.timing.idle_elapsed = 1;
            if self.atlas.dirty {
                self.atlas.selector = 0.8_f32.mul_add(self.parameters.vowel, 0.2);
                self.atlas.dirty = false;
            }
            self.atlas.timing.tick_counter += 1;
            return;
        }
        let tick_samples = self.atlas.tick_samples;
        if self.atlas.timing.idle_elapsed == tick_samples * 7
            || self.atlas.timing.idle_elapsed == tick_samples * 15
        {
            self.atlas.selector = 2.0 / 30.0;
        }
        if self.atlas.timing.idle_elapsed
            == (tick_samples.to_f64().unwrap_or(0.0) * 8.5)
                .to_usize()
                .unwrap_or(0)
            || self.atlas.timing.idle_elapsed == tick_samples * 17
        {
            self.atlas.selector = 5.0 / 30.0;
        }
        if self.atlas.timing.tick_counter >= tick_samples
            && self.atlas.timing.idle_elapsed >= tick_samples * 23
        {
            const IDLE: [usize; 24] = [
                5, 3, 4, 3, 2, 1, 0, 1, 5, 3, 4, 3, 5, 1, 0, 1, 2, 3, 4, 3, 5, 1, 0, 1,
            ];
            self.atlas.timing.tick_counter = 0;
            if self.atlas.idle_index >= IDLE.len() {
                self.atlas.idle_index = 0;
            }
            self.atlas.selector = IDLE
                .get(self.atlas.idle_index)
                .copied()
                .unwrap_or(0)
                .to_f32()
                .unwrap_or(0.0)
                / 30.0;
            self.atlas.idle_index += 1;
            self.atlas.timing.idle_elapsed = tick_samples * 23;
        }
        self.atlas.timing.tick_counter += 1;
        self.atlas.timing.idle_elapsed += 1;
    }

    pub(super) fn advance_vibrato(&mut self) -> f32 {
        if self.oscillator.vibrato.refresh.counter >= self.oscillator.vibrato.refresh.interval {
            self.oscillator.vibrato.refresh.counter = 0;
            self.oscillator.random_state = self
                .oscillator
                .random_state
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            self.oscillator.vibrato.rate_hz = 2.0_f32.mul_add(
                self.oscillator.random_state.to_f32().unwrap_or(0.0) / 4_294_967_296.0,
                5.0,
            );
        }
        let sample = (self.oscillator.vibrato.phase * std::f64::consts::TAU)
            .sin()
            .to_f32()
            .unwrap_or(0.0)
            * (self.parameters.vibrato + 0.2);
        self.oscillator.vibrato.phase = (self.oscillator.vibrato.phase
            + 0.2_f32
                .mul_add(self.parameters.vibrato, 1.0)
                .to_f64()
                .unwrap_or(0.0)
                * self.oscillator.vibrato.rate_hz.to_f64().unwrap_or(0.0)
                / self.configuration.sample_rate)
            % 1.0;
        self.oscillator.vibrato.refresh.counter += 1;
        sample
    }

    pub(super) fn advance_controls(&mut self) {
        if self.controls.pitch.legato {
            let pitch_delta = self.controls.pitch.target - self.controls.pitch.current;
            if pitch_delta.abs() <= 0.2 {
                self.controls.pitch.current = self.controls.pitch.target;
            } else {
                self.controls.pitch.current += pitch_delta.signum() * 12.0
                    / ((self.parameters.port_time.to_f64().unwrap_or(0.0) + 0.01)
                        * self.configuration.sample_rate);
            }
        } else {
            self.controls.pitch.current = self.controls.pitch.target;
        }
        if self.controls.update.counter >= self.controls.update.interval {
            self.controls.update.counter = 0;
            if self.controls.bend.steps_remaining > 0 {
                self.controls.bend.current += self.controls.bend.increment;
                self.controls.bend.steps_remaining -= 1;
                self.parameters.vowel = self.controls.bend.current.clamp(0.0, 1.0);
                self.atlas.dirty = true;
                self.grain.dirty = true;
            }
            if self.controls.route.steps_remaining > 0 {
                self.controls.route.current += self.controls.route.increment;
                self.controls.route.steps_remaining -= 1;
                self.parameters.xy_routing =
                    ((self.controls.route.current - 36.0) / 12.0).clamp(0.0, 1.0);
                self.controls.pitch.target = self.controls.route.current.to_f64().unwrap_or(0.0);
                if !self.controls.pitch.legato {
                    self.controls.pitch.current = self.controls.pitch.target;
                }
            }
        } else {
            self.controls.update.counter += 1;
        }
    }
}
