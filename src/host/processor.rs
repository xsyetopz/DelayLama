//! Host lifecycle, state, and event translation for the processor.

use crate::{
    protocol::{
        GestureResult, GestureTransition, PAD_NOTE_ON_VELOCITY, SynthesisEvent, SynthesisEventKind,
        internal_note,
    },
    synthesizer::{Parameters, SynthEngine, VoiceState},
};

const FACTORY_PROGRAMS: [(&str, f32, f32, f32); 5] = [
    ("Rabten", 0.5, 0.8, 0.5),
    ("Dorje", 0.4, 0.3, 0.0),
    ("Ngawang", 0.8, 0.6, 0.25),
    ("Jamyang", 0.5, 0.0, 0.75),
    ("Tinley", 1.0, 0.9, 1.0),
];

/// Values used to choose the asset editor's animation frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HostVisualState {
    /// Current internal note number.
    pub note: i32,
    /// Whether the engine currently has a held voice.
    pub gate: bool,
    /// Current vowel value between zero and one.
    pub vowel: f32,
    /// Artwork selector between zero and one.
    pub atlas_selector: f32,
}

/// Owns the synthesis engine, saved state, and host event handling.
#[derive(Debug, Default)]
pub struct ProcessorModel {
    engine: SynthEngine,
}

impl ProcessorModel {
    /// Prepares audio buffers and applies the current parameters.
    pub fn prepare(&mut self, sample_rate: f64, max_block_size: usize) {
        self.engine.prepare(sample_rate, max_block_size, 2);
    }

    /// Clears notes, buffers, and render state.
    pub fn release(&mut self) {
        self.engine.reset();
    }

    /// Processes one block of audio and its events.
    pub fn process(
        &mut self,
        outputs: &mut [&mut [f32]],
        samples: usize,
        events: &[SynthesisEvent],
    ) {
        self.engine.process(outputs, samples, events);
    }

    /// Applies an event before rendering the next sample.
    pub fn apply_event(&mut self, event: SynthesisEvent) {
        self.engine.process(&mut [], 0, &[event]);
    }

    /// Renders one stereo sample without allocating on the audio thread.
    pub fn render_stereo_frame(&mut self) -> [f32; 2] {
        let mut left = [0.0_f32];
        let mut right = [0.0_f32];
        self.engine.process(&mut [&mut left, &mut right], 1, &[]);
        [
            left.first().copied().unwrap_or_default(),
            right.first().copied().unwrap_or_default(),
        ]
    }

    /// Returns the parameters currently applied to the engine.
    pub const fn parameters(&self) -> Parameters {
        self.engine.parameters()
    }

    /// Stores and applies synthesis settings.
    pub fn set_parameters(&mut self, parameters: Parameters) {
        self.engine.set_parameters(parameters);
    }

    /// Applies host parameter values that changed since the last block.
    pub fn apply_changed_host_parameters(
        &mut self,
        previous: &mut Option<[f32; 4]>,
        next: [f32; 4],
    ) -> bool {
        let [next_vowel, next_port_time, next_delay_mix, next_voice] = next;
        let old = previous.replace(next);
        if old.is_some_and(|[old_vowel, old_port_time, old_delay_mix, old_voice]| {
            old_vowel.partial_cmp(&next_vowel) == Some(std::cmp::Ordering::Equal)
                && old_port_time.partial_cmp(&next_port_time) == Some(std::cmp::Ordering::Equal)
                && old_delay_mix.partial_cmp(&next_delay_mix) == Some(std::cmp::Ordering::Equal)
                && old_voice.partial_cmp(&next_voice) == Some(std::cmp::Ordering::Equal)
        }) {
            return false;
        }

        let mut merged = self.parameters();
        if old.is_none_or(|[old_vowel, ..]| old_vowel.to_bits() != next_vowel.to_bits()) {
            merged.vowel = next_vowel;
        }
        if old.is_none_or(|[_, old_port_time, ..]| {
            old_port_time.to_bits() != next_port_time.to_bits()
        }) {
            merged.port_time = next_port_time;
        }
        if old.is_none_or(|[_, _, old_delay_mix, _]| {
            old_delay_mix.to_bits() != next_delay_mix.to_bits()
        }) {
            merged.delay_mix = next_delay_mix;
        }
        if old.is_none_or(|[_, _, _, old_voice]| old_voice.to_bits() != next_voice.to_bits()) {
            merged.voice = next_voice;
        }
        self.set_parameters(merged);
        true
    }

    /// Returns the built-in program names and settings.
    pub const fn factory_programs() -> &'static [(&'static str, f32, f32, f32); 5] {
        &FACTORY_PROGRAMS
    }

    /// Saves the current settings in the versioned state format.
    pub fn save_state(&self) -> [u8; 32] {
        let parameters = self.engine.parameters();
        let values = [
            parameters.vowel,
            parameters.port_time,
            parameters.delay_mix,
            parameters.voice,
            parameters.vibrato,
            parameters.volume,
            parameters.xy_routing,
        ];
        let mut state = [0_u8; 32];
        if let Some(header) = state.get_mut(..4) {
            header.copy_from_slice(b"DLM1");
        }
        for (index, value) in values.into_iter().enumerate() {
            let start = 4 + index * 4;
            if let Some(destination) = state.get_mut(start..start + 4) {
                destination.copy_from_slice(&value.to_le_bytes());
            }
        }
        state
    }

    /// Loads settings from a versioned state buffer.
    pub fn load_state(&mut self, state: &[u8]) -> bool {
        if state.len() != 32 || state.get(..4) != Some(b"DLM1".as_slice()) {
            return false;
        }
        let mut values = [0.0_f32; 7];
        for (index, value) in values.iter_mut().enumerate() {
            let start = 4 + index * 4;
            let Some(bytes) = state.get(start..start + 4) else {
                return false;
            };
            let Ok(bytes) = <[u8; 4]>::try_from(bytes) else {
                return false;
            };
            *value = f32::from_le_bytes(bytes);
        }
        let [
            vowel,
            port_time,
            delay_mix,
            voice,
            vibrato,
            volume,
            xy_routing,
        ] = values;
        self.set_parameters(Parameters {
            vowel,
            port_time,
            delay_mix,
            voice,
            vibrato,
            volume,
            xy_routing,
        });
        true
    }

    /// Loads a built-in program by index.
    pub fn load_factory_program(&mut self, index: usize) -> bool {
        let Some((_, port_time, delay_mix, voice)) = FACTORY_PROGRAMS.get(index).copied() else {
            return false;
        };
        self.set_parameters(Parameters {
            port_time,
            delay_mix,
            voice,
            ..self.engine.parameters()
        });
        true
    }

    /// Returns the current engine voice state.
    pub const fn voice_state(&self) -> VoiceState {
        self.engine.voice_state()
    }

    /// Applies one pad gesture to the synthesis engine.
    pub fn apply_pad_gesture(&mut self, result: GestureResult) {
        match result.transition {
            GestureTransition::NoteOn(note) => {
                self.apply_pad_position(result);
                if let Some(note) = internal_note(note) {
                    self.apply_local_pad_event(
                        SynthesisEventKind::NoteOn,
                        note,
                        PAD_NOTE_ON_VELOCITY,
                    );
                }
            }
            GestureTransition::None => self.apply_pad_position(result),
            GestureTransition::NoteOff(note) => {
                if let Some(note) = internal_note(note) {
                    self.apply_local_pad_event(SynthesisEventKind::NoteOff, note, 0.0);
                }
            }
        }
    }

    fn apply_pad_position(&mut self, result: GestureResult) {
        self.apply_local_pad_event(SynthesisEventKind::PadPitch, 0, result.position.x);
        self.apply_local_pad_event(SynthesisEventKind::PadVowel, 0, result.vowel);
    }

    fn apply_local_pad_event(&mut self, kind: SynthesisEventKind, note: i32, value: f32) {
        self.apply_event(SynthesisEvent {
            kind,
            note,
            value,
            local_pad: true,
            ..SynthesisEvent::default()
        });
    }

    /// Returns the values used to draw the current voice.
    pub const fn visual_state(&self) -> HostVisualState {
        let voice = self.engine.voice_state();
        let parameters = self.engine.parameters();
        let pad = self.engine.pad_state();
        HostVisualState {
            note: voice.current_note,
            gate: voice.gate,
            vowel: if voice.gate && pad.active {
                pad.vowel
            } else {
                parameters.vowel
            },
            atlas_selector: self.engine.atlas_selector(),
        }
    }
}
