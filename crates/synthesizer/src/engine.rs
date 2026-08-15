//! Original Delay Lama granular-formant synthesis engine.

use crate::state::*;
use crate::tables;
use delaylama_protocol::{
    CC_DELAY_MIX, CC_PORT_TIME, CC_VIBRATO, CC_VOICE, CC_VOLUME, CC_XY_ROUTING, CC7_VOLUME_SCALE,
    PAD_INTERNAL_NOTE, SynthesisEvent, SynthesisEventKind,
};

const GRAIN_SECONDS: f64 = 0.02;
type Event = SynthesisEvent;
type EventType = SynthesisEventKind;

/// Stateful original Delay Lama granular-formant synthesizer.
#[derive(Clone, Debug)]
pub struct SynthEngine {
    sample_rate: f64,
    channels: usize,
    params: Parameters,
    voice: VoiceState,
    pad: PadState,
    bend_current: f32,
    bend_target: f32,
    bend_increment: f32,
    bend_steps_remaining: i32,
    bend_update_counter: usize,
    bend_update_interval: usize,
    current_pitch: f64,
    target_pitch: f64,
    legato_glide: bool,
    route_current: f32,
    route_target: f32,
    route_increment: f32,
    route_steps_remaining: i32,
    phase: f64,
    vibrato_phase: f64,
    vibrato_rate_hz: f32,
    vibrato_refresh_counter: usize,
    vibrato_refresh_interval: usize,
    random_state: u32,
    delay_left: Vec<f32>,
    delay_right: Vec<f32>,
    delay_write: usize,
    delay_left_tap: usize,
    delay_right_tap: usize,
    note_stack: [i32; 128],
    local_pad_note_active: bool,
    external_notes_active: [bool; 128],
    grain: Vec<f32>,
    dry_ring: Vec<f32>,
    dry_cursor: usize,
    samples_since_grain: usize,
    grain_dirty: bool,
    formants: [Vec<f32>; 3],
    window: Vec<f32>,
    excitation: Vec<f32>,
    atlas_selector: f32,
    atlas_dirty: bool,
    atlas_tick_samples: usize,
    atlas_idle_elapsed: usize,
    atlas_tick_counter: usize,
    atlas_idle_index: usize,
}

impl Default for SynthEngine {
    fn default() -> Self {
        Self {
            sample_rate: DEFAULT_SAMPLE_RATE,
            channels: 2,
            params: Parameters::default(),
            voice: VoiceState {
                current_note: -1,
                gate: false,
            },
            pad: PadState::default(),
            bend_current: 8192.0 / 16384.0,
            bend_target: 8192.0 / 16384.0,
            bend_increment: 0.0,
            bend_steps_remaining: 0,
            bend_update_counter: 0,
            bend_update_interval: 441,
            current_pitch: 36.0,
            target_pitch: 36.0,
            legato_glide: false,
            route_current: 36.0,
            route_target: 36.0,
            route_increment: 0.0,
            route_steps_remaining: 0,
            phase: 0.0,
            vibrato_phase: 0.0,
            vibrato_rate_hz: 4.0,
            vibrato_refresh_counter: 0,
            vibrato_refresh_interval: 4586,
            random_state: 1,
            delay_left: vec![0.0; 20_000],
            delay_right: vec![0.0; 20_000],
            delay_write: 0,
            delay_left_tap: 0,
            delay_right_tap: 0,
            note_stack: [-1; 128],
            local_pad_note_active: false,
            external_notes_active: [false; 128],
            grain: Vec::new(),
            dry_ring: vec![0.0; 20_000],
            dry_cursor: 0,
            samples_since_grain: 0,
            grain_dirty: true,
            formants: tables::formant_tables(),
            window: Vec::new(),
            excitation: Vec::new(),
            atlas_selector: 0.0,
            atlas_dirty: true,
            atlas_tick_samples: 1,
            atlas_idle_elapsed: 0,
            atlas_tick_counter: 0,
            atlas_idle_index: 0,
        }
    }
}

impl SynthEngine {
    /// Prepares buffers and timing state for the host sample rate and channel count.
    pub fn prepare(&mut self, sample_rate: f64, _max: usize, ch: usize) {
        self.sample_rate = if sample_rate.is_finite() {
            sample_rate.clamp(MIN_SAMPLE_RATE, MAX_SAMPLE_RATE)
        } else {
            DEFAULT_SAMPLE_RATE
        };
        self.channels = ch.max(1);
        self.delay_left = vec![0.0; 20_000];
        self.delay_right = vec![0.0; 20_000];
        let grain_samples = (self.sample_rate * GRAIN_SECONDS) as usize;
        self.grain = vec![0.0; grain_samples.max(1)];
        self.dry_ring = vec![0.0; 20_000];
        self.window = tables::window(self.sample_rate, self.grain.len());
        self.excitation = tables::excitation(self.sample_rate, self.grain.len());
        self.atlas_tick_samples = (self.sample_rate * 0.208).max(1.0) as usize;
        self.reset()
    }

    /// Resets voices, modulation, grain history, delay state, and artwork state.
    pub fn reset(&mut self) {
        self.voice = VoiceState {
            current_note: -1,
            gate: false,
        };
        self.pad = PadState::default();
        self.bend_current = 8192.0 / 16384.0;
        self.bend_target = self.bend_current;
        self.bend_increment = 0.0;
        self.bend_steps_remaining = 0;
        self.bend_update_counter = 0;
        self.bend_update_interval = (self.sample_rate * 0.01) as usize;
        self.current_pitch = 36.0;
        self.target_pitch = 36.0;
        self.legato_glide = false;
        self.route_current = 36.0;
        self.route_target = 36.0;
        self.route_increment = 0.0;
        self.route_steps_remaining = 0;
        self.phase = 0.0;
        self.vibrato_phase = 0.0;
        self.vibrato_rate_hz = 4.0;
        self.vibrato_refresh_counter = 0;
        self.vibrato_refresh_interval = (self.sample_rate * 0.104) as usize;
        self.random_state = 1;
        self.dry_ring.fill(0.0);
        self.dry_cursor = 0;
        self.samples_since_grain = 0;
        self.grain_dirty = true;
        self.delay_left.fill(0.0);
        self.delay_right.fill(0.0);
        self.delay_write = 0;
        self.delay_left_tap = (self.delay_left.len()
            - (self.sample_rate * 0.309592) as usize % self.delay_left.len())
            % self.delay_left.len();
        self.delay_right_tap = (self.delay_right.len()
            - (self.sample_rate * 0.398435) as usize % self.delay_right.len())
            % self.delay_right.len();
        self.note_stack = [-1; 128];
        self.local_pad_note_active = false;
        self.external_notes_active = [false; 128];
        self.atlas_selector = 0.0;
        self.atlas_dirty = true;
        self.atlas_tick_counter = 0;
        self.atlas_idle_elapsed = 0;
        self.atlas_idle_index = 0;
    }

    /// Applies normalized host parameters to the original synthesis model.
    pub fn set_parameters(&mut self, p: Parameters) {
        let vowel_changed = clamp(p.vowel, 0.5) != self.params.vowel;
        let voice_changed = clamp(p.voice, 0.5) != self.params.voice;
        self.atlas_dirty = self.atlas_dirty || vowel_changed;
        self.grain_dirty = self.grain_dirty || vowel_changed || voice_changed;
        self.params = Parameters {
            vowel: clamp(p.vowel, 0.5),
            port_time: clamp(p.port_time, 0.5),
            delay_mix: clamp(p.delay_mix, 0.8),
            voice: clamp(p.voice, 0.5),
            vibrato: clamp(p.vibrato, 0.0),
            volume: clamp(p.volume, 0.1),
            xy_routing: clamp(p.xy_routing, 0.0),
        };
        if vowel_changed && self.voice.gate {
            self.atlas_selector = 0.2 + 0.8 * self.params.vowel;
            self.atlas_dirty = false;
        }
    }

    /// Returns the currently active synthesis parameters.
    pub fn parameters(&self) -> Parameters {
        self.params
    }

    /// Returns the monophonic voice state.
    pub fn voice_state(&self) -> VoiceState {
        self.voice
    }

    /// Returns the current editor-pad state.
    pub fn pad_state(&self) -> PadState {
        self.pad
    }

    /// Returns the artwork-atlas selector driven by the original engine state.
    pub fn atlas_selector(&self) -> f32 {
        self.atlas_selector
    }

    /// Processes one block using the original grain, overlap-add, and delay algorithm.
    pub fn process(&mut self, outputs: &mut [&mut [f32]], n: usize, events: &[SynthesisEvent]) {
        if n == 0 {
            for event in events.iter().filter(|event| event.sample_offset <= 0) {
                self.apply(*event);
            }
            return;
        }
        for i in 0..n {
            for e in events
                .iter()
                .filter(|e| e.sample_offset == i as i32 || (i == 0 && e.sample_offset < 0))
            {
                self.apply(*e)
            }
            self.advance_controls();
            self.advance_atlas_state();
            let vibrato = self.advance_vibrato();
            let freq = if self.voice.gate {
                440.0 * 2f64.powf((self.current_pitch - 69.0 + vibrato as f64) / 12.0)
            } else {
                0.0
            };
            self.phase = (self.phase + freq / self.sample_rate) % 1.0;
            if self.voice.gate {
                let period = (self.sample_rate / freq.max(1.0)).max(1.0) as usize;
                let force_grain = self.grain_dirty;
                if force_grain {
                    self.rebuild_grain();
                }
                if force_grain || self.samples_since_grain >= period {
                    for (offset, sample) in self.grain.iter().copied().enumerate() {
                        let destination = (self.dry_cursor + offset) % self.dry_ring.len();
                        self.dry_ring[destination] += sample;
                    }
                    self.samples_since_grain = 0;
                }
            }
            let dry = self.dry_ring[self.dry_cursor];
            self.dry_ring[self.dry_cursor] = 0.0;
            self.dry_cursor = (self.dry_cursor + 1) % self.dry_ring.len();
            self.samples_since_grain += 1;
            let lf = self.delay_left[self.delay_left_tap];
            let rf = self.delay_right[self.delay_right_tap];
            let mix = self.params.delay_mix;
            self.delay_left[self.delay_write] = (dry + lf * 0.5) * mix;
            self.delay_right[self.delay_write] = (dry + rf * 0.5) * mix;
            let output_gain = (2.0 - self.current_pitch as f32 / 72.0) * self.params.volume;
            let left = (dry + self.delay_left[self.delay_left_tap]) * output_gain;
            let right = (dry + self.delay_right[self.delay_right_tap]) * output_gain;
            self.delay_write = (self.delay_write + 1) % self.delay_left.len();
            self.delay_left_tap = (self.delay_left_tap + 1) % self.delay_left.len();
            self.delay_right_tap = (self.delay_right_tap + 1) % self.delay_right.len();
            for (ci, ch) in outputs.iter_mut().enumerate() {
                if i < ch.len() {
                    ch[i] = if ci % 2 == 0 { left } else { right }
                }
            }
        }
    }

    fn rebuild_grain(&mut self) {
        let n = self.grain.len();
        if n == 0 {
            return;
        }
        let vi = (self.params.vowel.clamp(0.0, 1.0) * 1279.0) as usize;
        let scale = (0.75 + 0.5 * self.params.voice) as f64;
        let mut ph = [0.0f64; 3];
        let mut dec = [0.0f64; 3];
        for i in 0..n {
            let mut v = 0.0;
            for j in 0..3 {
                let hz = self.formants[j][vi] as f64 * scale;
                v += (ph[j] * std::f64::consts::TAU).sin()
                    * (-157.0796327 * dec[j] / self.sample_rate).exp();
                ph[j] = (ph[j] + hz / self.sample_rate) % 1.0;
                dec[j] += [0.65, 0.95, 1.25][j];
            }
            self.grain[i] = ((v + 0.5 * self.excitation[i] as f64) * self.window[i] as f64) as f32;
        }
        self.grain_dirty = false;
    }

    fn note_on(&mut self, note: i32) {
        if !(4..=72).contains(&note) || self.note_stack.contains(&note) {
            return;
        }
        if self.note_stack[127] < 0 {
            for index in (1..128).rev() {
                self.note_stack[index] = self.note_stack[index - 1];
            }
        }
        self.note_stack[0] = note;
        self.update_voice();
    }

    fn note_off(&mut self, note: i32) {
        if let Some(index) = self.note_stack.iter().position(|&held| held == note) {
            for next in index..127 {
                self.note_stack[next] = self.note_stack[next + 1];
            }
            self.note_stack[127] = -1;
        }
        self.update_voice();
    }

    fn update_voice(&mut self) {
        let note = self.note_stack[0];
        let was_gated = self.voice.gate;
        self.voice = VoiceState {
            current_note: note,
            gate: note >= 0,
        };
        if note < 0 {
            self.legato_glide = false;
            // Grains are queued ahead in the overlap-add ring. Once the last note is
            // released they no longer belong to a live voice; leaving them queued
            // makes the previous pitch sound alongside the next pad gesture.
            self.dry_ring.fill(0.0);
            self.samples_since_grain = 0;
            self.atlas_selector = 5.0 / 30.0;
            self.atlas_idle_elapsed = 0;
            self.atlas_tick_counter = 0;
            self.atlas_idle_index = 0;
            self.atlas_dirty = true;
            return;
        }
        self.target_pitch = note as f64;
        if !was_gated {
            self.current_pitch = self.target_pitch;
            self.legato_glide = false;
        } else {
            self.legato_glide = true;
        }
    }

    fn advance_atlas_state(&mut self) {
        if self.voice.gate {
            self.atlas_idle_elapsed = 1;
            if self.atlas_dirty {
                self.atlas_selector = 0.2 + 0.8 * self.params.vowel;
                self.atlas_dirty = false;
            }
            self.atlas_tick_counter += 1;
            return;
        }
        let t = self.atlas_tick_samples;
        if self.atlas_idle_elapsed == t * 7 || self.atlas_idle_elapsed == t * 15 {
            self.atlas_selector = 2.0 / 30.0;
        }
        if self.atlas_idle_elapsed == (t as f64 * 8.5) as usize || self.atlas_idle_elapsed == t * 17
        {
            self.atlas_selector = 5.0 / 30.0;
        }
        if self.atlas_tick_counter >= t && self.atlas_idle_elapsed >= t * 23 {
            const IDLE: [usize; 24] = [
                5, 3, 4, 3, 2, 1, 0, 1, 5, 3, 4, 3, 5, 1, 0, 1, 2, 3, 4, 3, 5, 1, 0, 1,
            ];
            self.atlas_tick_counter = 0;
            if self.atlas_idle_index >= IDLE.len() {
                self.atlas_idle_index = 0;
            }
            self.atlas_selector = IDLE[self.atlas_idle_index] as f32 / 30.0;
            self.atlas_idle_index += 1;
            self.atlas_idle_elapsed = t * 23;
        }
        self.atlas_tick_counter += 1;
        self.atlas_idle_elapsed += 1;
    }

    fn advance_vibrato(&mut self) -> f32 {
        if self.vibrato_refresh_counter >= self.vibrato_refresh_interval {
            self.vibrato_refresh_counter = 0;
            self.random_state = self
                .random_state
                .wrapping_mul(1664525)
                .wrapping_add(1013904223);
            self.vibrato_rate_hz = 5.0 + 2.0 * (self.random_state as f32 / 4294967296.0);
        }
        let sample = ((self.vibrato_phase * std::f64::consts::TAU).sin() as f32)
            * (self.params.vibrato + 0.2);
        self.vibrato_phase = (self.vibrato_phase
            + (1.0 + 0.2 * self.params.vibrato) as f64 * self.vibrato_rate_hz as f64
                / self.sample_rate)
            % 1.0;
        self.vibrato_refresh_counter += 1;
        sample
    }

    fn advance_controls(&mut self) {
        if self.legato_glide {
            let d = self.target_pitch - self.current_pitch;
            if d.abs() <= 0.2 {
                self.current_pitch = self.target_pitch;
            } else {
                self.current_pitch +=
                    d.signum() * 12.0 / ((self.params.port_time as f64 + 0.01) * self.sample_rate);
            }
        } else {
            self.current_pitch = self.target_pitch;
        }
        if self.bend_update_counter >= self.bend_update_interval {
            self.bend_update_counter = 0;
            if self.bend_steps_remaining > 0 {
                self.bend_current += self.bend_increment;
                self.bend_steps_remaining -= 1;
                self.params.vowel = self.bend_current.clamp(0.0, 1.0);
                self.atlas_dirty = true;
                self.grain_dirty = true;
            }
            if self.route_steps_remaining > 0 {
                self.route_current += self.route_increment;
                self.route_steps_remaining -= 1;
                self.params.xy_routing = ((self.route_current - 36.0) / 12.0).clamp(0.0, 1.0);
                self.target_pitch = self.route_current as f64;
                if !self.legato_glide {
                    self.current_pitch = self.target_pitch;
                }
            }
        } else {
            self.bend_update_counter += 1;
        }
    }

    fn apply(&mut self, e: Event) {
        match e.kind {
            EventType::NoteOn => {
                if e.local_pad && e.note == PAD_INTERNAL_NOTE {
                    self.pad.active = e.value > 0.0;
                    let external_voice_held = self.external_notes_active.iter().any(|held| *held);
                    if e.value > 0.0 && !external_voice_held {
                        if !self.local_pad_note_active {
                            self.local_pad_note_active = true;
                            self.note_on(e.note);
                            // The pad note is lifecycle metadata, not an audible fixed carrier.
                            // Start the one voice at the pad route before the first grain is queued.
                            self.route_current = 36.0 + 12.0 * self.pad.pitch_modulation;
                            self.route_target = self.route_current;
                            self.route_increment = 0.0;
                            self.route_steps_remaining = 0;
                            self.current_pitch = self.route_current as f64;
                            self.target_pitch = self.current_pitch;
                        }
                    } else if e.value <= 0.0 && self.local_pad_note_active {
                        self.local_pad_note_active = false;
                        self.note_off(e.note);
                    }
                } else if e.value > 0.0 {
                    if self.local_pad_note_active {
                        self.local_pad_note_active = false;
                        self.note_off(PAD_INTERNAL_NOTE);
                    }
                    let mut newly_held = true;
                    if let Ok(index) = usize::try_from(e.note)
                        && index < self.external_notes_active.len()
                    {
                        newly_held = !self.external_notes_active[index];
                        self.external_notes_active[index] = true;
                    }
                    if newly_held {
                        self.note_on(e.note);
                    }
                } else {
                    if let Ok(index) = usize::try_from(e.note)
                        && index < self.external_notes_active.len()
                    {
                        self.external_notes_active[index] = false;
                    }
                    self.note_off(e.note);
                }
            }
            EventType::NoteOff => {
                if e.local_pad && e.note == PAD_INTERNAL_NOTE {
                    self.pad.active = false;
                    if self.local_pad_note_active {
                        self.local_pad_note_active = false;
                        self.note_off(e.note);
                    }
                } else {
                    if let Ok(index) = usize::try_from(e.note)
                        && index < self.external_notes_active.len()
                    {
                        self.external_notes_active[index] = false;
                    }
                    self.note_off(e.note);
                }
            }
            EventType::PitchBend => {
                let target = if e.value > 1.0 {
                    e.value.clamp(0.0, 16383.0) / 16384.0
                } else {
                    e.value.clamp(0.0, 1.0)
                };
                self.bend_target = target;
                self.bend_increment = (target - self.bend_current) / 10.0;
                self.bend_steps_remaining = 10;
            }
            EventType::PadPitch => {
                self.pad.pitch_modulation = clamp(e.value, 0.5);
                self.pad.active = true;
                self.route_target = 36.0 + 12.0 * self.pad.pitch_modulation;
                self.route_increment = (self.route_target - self.route_current) / 10.0;
                self.route_steps_remaining = 10;
            }
            EventType::PadVowel => {
                self.pad.vowel = clamp(e.value, 0.5);
                self.pad.active = true;
                self.bend_target = self.pad.vowel;
                self.bend_increment = (self.bend_target - self.bend_current) / 10.0;
                self.bend_steps_remaining = 10;
            }
            EventType::ControlChange => {
                let v = if e.value > 1.0 {
                    e.value / 127.0
                } else {
                    e.value.clamp(0.0, 1.0)
                };
                match e.controller {
                    CC_VIBRATO => self.params.vibrato = v,
                    CC_PORT_TIME => self.params.port_time = v,
                    CC_VOLUME => self.params.volume = v * CC7_VOLUME_SCALE,
                    CC_XY_ROUTING => {
                        self.route_target = 36.0 + 12.0 * v;
                        self.route_increment = (self.route_target - self.route_current) / 10.0;
                        self.route_steps_remaining = 10;
                    }
                    CC_DELAY_MIX => self.params.delay_mix = v,
                    CC_VOICE => {
                        self.params.voice = v;
                        self.grain_dirty = true;
                    }
                    _ => {}
                }
            }
        }
    }
}

fn clamp(v: f32, f: f32) -> f32 {
    if v.is_finite() { v.clamp(0.0, 1.0) } else { f }
}

#[cfg(test)]
mod local_pad_event_path_tests {
    use super::*;

    #[test]
    fn controls_before_local_note_on_start_at_pointer_pitch_without_fixed_carrier() {
        let mut engine = SynthEngine::default();
        engine.apply(Event {
            kind: EventType::PadPitch,
            value: 1.0,
            local_pad: true,
            ..Event::default()
        });
        engine.apply(Event {
            kind: EventType::PadVowel,
            value: 0.5,
            local_pad: true,
            ..Event::default()
        });
        engine.apply(Event {
            kind: EventType::NoteOn,
            note: PAD_INTERNAL_NOTE,
            value: 64.0 / 127.0,
            local_pad: true,
            ..Event::default()
        });

        assert_eq!(engine.voice.current_note, PAD_INTERNAL_NOTE);
        assert_eq!(engine.current_pitch, 48.0);
        assert_eq!(engine.target_pitch, 48.0);
        assert_eq!(engine.route_current, 48.0);
        assert_eq!(engine.route_steps_remaining, 0);
    }

    #[test]
    fn vowel_bend_does_not_retune_grain_overlap_pitch() {
        fn samples_since_last_grain(bend: f32) -> usize {
            let mut engine = SynthEngine::default();
            engine.prepare(44_100.0, 1_024, 2);
            engine.apply(Event {
                kind: EventType::NoteOn,
                note: 48,
                value: 1.0,
                ..Event::default()
            });
            engine.bend_current = bend;
            engine.grain_dirty = false;
            let mut left = [0.0; 1_024];
            let mut right = [0.0; 1_024];
            engine.process(&mut [&mut left, &mut right], 1_024, &[]);
            engine.samples_since_grain
        }

        assert_eq!(samples_since_last_grain(0.0), samples_since_last_grain(1.0));
    }
}
