//! Audio rendering and grain construction.

use num_traits::ToPrimitive;

use super::{SynthEngine, SynthesisEvent};

const DECAY_STEPS: [f64; 3] = [0.65, 0.95, 1.25];

impl SynthEngine {
    /// Processes one block with grain, overlap-add, and delay stages.
    pub fn process(
        &mut self,
        outputs: &mut [&mut [f32]],
        sample_count: usize,
        events: &[SynthesisEvent],
    ) {
        if sample_count == 0 {
            self.apply_events_before_start(events);
            return;
        }
        for sample_index in 0..sample_count {
            self.apply_events_at_sample(events, sample_index);
            let (left, right) = self.render_sample();
            for (channel_index, channel) in outputs.iter_mut().enumerate() {
                if let Some(sample) = channel.get_mut(sample_index) {
                    *sample = if channel_index % 2 == 0 { left } else { right };
                }
            }
        }
    }

    fn apply_events_before_start(&mut self, events: &[SynthesisEvent]) {
        for event in events.iter().filter(|event| event.sample_offset <= 0) {
            self.apply(*event);
        }
    }

    fn apply_events_at_sample(&mut self, events: &[SynthesisEvent], sample_index: usize) {
        let sample_offset = sample_index.to_i32().unwrap_or(i32::MAX);
        for event in events.iter().filter(|event| {
            event.sample_offset == sample_offset || (sample_index == 0 && event.sample_offset < 0)
        }) {
            self.apply(*event);
        }
    }

    fn render_sample(&mut self) -> (f32, f32) {
        self.advance_controls();
        self.advance_atlas_state();
        let vibrato = self.advance_vibrato();
        let frequency = self.voice_frequency(vibrato);
        self.oscillator.phase =
            (self.oscillator.phase + frequency / self.configuration.sample_rate) % 1.0;
        self.queue_grain_if_due(frequency);

        let dry = self.take_overlap_sample();
        self.render_delay(dry)
    }

    fn voice_frequency(&self, vibrato: f32) -> f64 {
        if !self.notes.voice.gate {
            return 0.0;
        }
        let vibrato = vibrato.to_f64().unwrap_or(0.0);
        440.0 * ((self.controls.pitch.current - 69.0 + vibrato) / 12.0).exp2()
    }

    fn queue_grain_if_due(&mut self, frequency: f64) {
        if !self.notes.voice.gate {
            return;
        }
        let period = (self.configuration.sample_rate / frequency.max(1.0))
            .max(1.0)
            .to_usize()
            .unwrap_or(1);
        let force_grain = self.grain.dirty;
        if force_grain {
            self.rebuild_grain();
        }
        if force_grain || self.grain.overlap.samples_since_grain >= period {
            self.add_grain_to_overlap();
            self.grain.overlap.samples_since_grain = 0;
        }
    }

    fn add_grain_to_overlap(&mut self) {
        let ring_len = self.grain.overlap.ring.len();
        for (offset, sample) in self.grain.samples.iter().copied().enumerate() {
            let destination = (self.grain.overlap.cursor + offset) % ring_len;
            if let Some(slot) = self.grain.overlap.ring.get_mut(destination) {
                *slot += sample;
            }
        }
    }

    fn take_overlap_sample(&mut self) -> f32 {
        let cursor = self.grain.overlap.cursor;
        let dry = self.grain.overlap.ring.get(cursor).copied().unwrap_or(0.0);
        if let Some(slot) = self.grain.overlap.ring.get_mut(cursor) {
            *slot = 0.0;
        }
        self.grain.overlap.cursor = (cursor + 1) % self.grain.overlap.ring.len();
        self.grain.overlap.samples_since_grain += 1;
        dry
    }

    fn render_delay(&mut self, dry: f32) -> (f32, f32) {
        let left_tap = self.delay.cursors.left_tap;
        let right_tap = self.delay.cursors.right_tap;
        let write = self.delay.cursors.write;
        let left_feedback = self
            .delay
            .buffers
            .left
            .get(left_tap)
            .copied()
            .unwrap_or(0.0);
        let right_feedback = self
            .delay
            .buffers
            .right
            .get(right_tap)
            .copied()
            .unwrap_or(0.0);
        let mix = self.parameters.delay_mix;
        if let Some(sample) = self.delay.buffers.left.get_mut(write) {
            *sample = (dry + left_feedback * 0.5) * mix;
        }
        if let Some(sample) = self.delay.buffers.right.get_mut(write) {
            *sample = (dry + right_feedback * 0.5) * mix;
        }
        let output_gain = (2.0 - self.controls.pitch.current.to_f32().unwrap_or(0.0) / 72.0)
            * self.parameters.volume;
        let left = (dry
            + self
                .delay
                .buffers
                .left
                .get(left_tap)
                .copied()
                .unwrap_or(0.0))
            * output_gain;
        let right = (dry
            + self
                .delay
                .buffers
                .right
                .get(right_tap)
                .copied()
                .unwrap_or(0.0))
            * output_gain;
        let left_len = self.delay.buffers.left.len();
        let right_len = self.delay.buffers.right.len();
        self.delay.cursors.write = (write + 1) % left_len;
        self.delay.cursors.left_tap = (left_tap + 1) % left_len;
        self.delay.cursors.right_tap = (right_tap + 1) % right_len;
        (left, right)
    }

    pub(super) fn rebuild_grain(&mut self) {
        let grain_length = self.grain.samples.len();
        if grain_length == 0 {
            return;
        }
        let vowel_index = (self.parameters.vowel.clamp(0.0, 1.0) * 1_279.0)
            .to_usize()
            .unwrap_or(0);
        let scale = 0.5_f32
            .mul_add(self.parameters.voice, 0.75)
            .to_f64()
            .unwrap_or(0.0);
        let mut phases = [0.0_f64; 3];
        let mut decays = [0.0_f64; 3];
        for sample_index in 0..grain_length {
            let mut value = 0.0;
            for formant_index in 0..3 {
                let phase = phases.get(formant_index).copied().unwrap_or(0.0);
                let decay = decays.get(formant_index).copied().unwrap_or(0.0);
                let hz = self
                    .grain
                    .source
                    .formants
                    .get(formant_index)
                    .and_then(|table| table.get(vowel_index))
                    .copied()
                    .unwrap_or(0.0)
                    .to_f64()
                    .unwrap_or(0.0)
                    * scale;
                value = (phase * std::f64::consts::TAU).sin().mul_add(
                    (-157.079_632_7 * decay / self.configuration.sample_rate).exp(),
                    value,
                );
                if let Some(next_phase) = phases.get_mut(formant_index) {
                    *next_phase = (phase + hz / self.configuration.sample_rate) % 1.0;
                }
                if let Some(next_decay) = decays.get_mut(formant_index) {
                    *next_decay += DECAY_STEPS.get(formant_index).copied().unwrap_or(0.0);
                }
            }
            let excitation = self
                .grain
                .source
                .excitation
                .get(sample_index)
                .copied()
                .unwrap_or(0.0)
                .to_f64()
                .unwrap_or(0.0);
            let window = self
                .grain
                .source
                .window
                .get(sample_index)
                .copied()
                .unwrap_or(0.0)
                .to_f64()
                .unwrap_or(0.0);
            if let Some(sample) = self.grain.samples.get_mut(sample_index) {
                *sample = (0.5_f64.mul_add(excitation, value) * window)
                    .to_f32()
                    .unwrap_or(0.0);
            }
        }
        self.grain.dirty = false;
    }
}
