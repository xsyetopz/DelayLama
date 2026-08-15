//! Host lifecycle, state, and event translation for the processor.
use delaylama_editor::{GestureResult, PadGesture};
use delaylama_synthesizer::{
    Parameters, SynthEngine, SynthesisEvent, SynthesisEventKind, VisualState, VoiceState,
};

const FACTORY_PROGRAMS: [(&str, f32, f32, f32); 5] = [
    ("Rabten", 0.5, 0.8, 0.5),
    ("Dorje", 0.4, 0.3, 0.0),
    ("Ngawang", 0.8, 0.6, 0.25),
    ("Jamyang", 0.5, 0.0, 0.75),
    ("Tinley", 1.0, 0.9, 1.0),
];

#[derive(Debug, Default)]
/// Host-facing lifecycle and state wrapper around the synthesis engine.
pub struct ProcessorModel {
    engine: SynthEngine,
    parameters: Parameters,
}
impl ProcessorModel {
    /// Prepares audio buffers and applies the current parameters.
    pub fn prepare(&mut self, sample_rate: f64, max_block_size: usize) {
        self.engine.prepare(sample_rate, max_block_size, 2);
        self.engine.set_parameters(self.parameters);
    }
    /// Releases the current render state.
    pub fn release(&mut self) {
        self.engine.reset();
    }
    /// Processes one host audio block and its normalized events.
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
    pub fn parameters(&self) -> Parameters {
        self.engine.parameters()
    }
    /// Stores and applies normalized synthesis parameters.
    pub fn set_parameters(&mut self, p: Parameters) {
        self.parameters = p;
        self.engine.set_parameters(p);
    }

    /// Returns the built-in program parameter records.
    pub const fn factory_programs() -> &'static [(&'static str, f32, f32, f32); 5] {
        &FACTORY_PROGRAMS
    }

    /// Serializes parameters into the versioned host state format.
    pub fn save_state(&self) -> [u8; 32] {
        let p = self.parameters;
        let values = [
            p.vowel,
            p.port_time,
            p.delay_mix,
            p.voice,
            p.vibrato,
            p.volume,
            p.xy_routing,
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

    /// Loads a versioned state buffer, returning whether it was accepted.
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
            ..self.parameters
        });
        true
    }
    /// Returns the current engine voice state.
    pub fn voice_state(&self) -> VoiceState {
        self.engine.voice_state()
    }

    /// Converts a pad gesture into core events.
    pub fn pad_events(result: GestureResult, gesture: PadGesture) -> [SynthesisEvent; 3] {
        let note_on = !matches!(gesture, PadGesture::Up);
        [
            SynthesisEvent {
                kind: if note_on {
                    SynthesisEventKind::NoteOn
                } else {
                    SynthesisEventKind::NoteOff
                },
                note: 28,
                value: if note_on { 1.0 } else { 0.0 },
                local_pad: true,
                ..SynthesisEvent::default()
            },
            SynthesisEvent {
                kind: SynthesisEventKind::PadPitch,
                value: result.x,
                local_pad: true,
                ..SynthesisEvent::default()
            },
            SynthesisEvent {
                kind: SynthesisEventKind::PadVowel,
                value: result.vowel,
                local_pad: true,
                ..SynthesisEvent::default()
            },
        ]
    }

    /// Returns the visual state consumed by the editor.
    pub fn visual_state(&self) -> VisualState {
        let voice = self.engine.voice_state();
        let parameters = self.engine.parameters();
        let pad = self.engine.pad_state();
        VisualState {
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
