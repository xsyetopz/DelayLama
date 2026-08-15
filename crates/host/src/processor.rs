use delaylama_core::{Event, EventType, Parameters, SynthEngine, VisualState, VoiceState};
use delaylama_editor::{GestureResult, PadGesture};

const FACTORY_PROGRAMS: [(&str, f32, f32, f32); 5] = [
    ("Rabten", 0.5, 0.8, 0.5),
    ("Dorje", 0.4, 0.3, 0.0),
    ("Ngawang", 0.8, 0.6, 0.25),
    ("Jamyang", 0.5, 0.0, 0.75),
    ("Tinley", 1.0, 0.9, 1.0),
];

#[derive(Debug)]
pub struct ProcessorModel {
    engine: SynthEngine,
    parameters: Parameters,
}
impl Default for ProcessorModel {
    fn default() -> Self {
        Self {
            engine: SynthEngine::default(),
            parameters: Parameters::default(),
        }
    }
}
impl ProcessorModel {
    pub fn prepare(&mut self, sample_rate: f64, max_block_size: usize) {
        self.engine.prepare(sample_rate, max_block_size, 2);
        self.engine.set_parameters(self.parameters)
    }
    pub fn release(&mut self) {
        self.engine.reset()
    }
    pub fn process(&mut self, outputs: &mut [&mut [f32]], samples: usize, events: &[Event]) {
        self.engine.process(outputs, samples, events)
    }
    pub fn parameters(&self) -> Parameters {
        self.engine.parameters()
    }
    pub fn set_parameters(&mut self, p: Parameters) {
        self.parameters = p;
        self.engine.set_parameters(p)
    }

    pub fn factory_programs() -> &'static [(&'static str, f32, f32, f32); 5] {
        &FACTORY_PROGRAMS
    }

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
        state[..4].copy_from_slice(b"DLM1");
        for (index, value) in values.into_iter().enumerate() {
            let start = 4 + index * 4;
            state[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
        state
    }

    pub fn load_state(&mut self, state: &[u8]) -> bool {
        if state.len() != 32 || state[..4] != *b"DLM1" {
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
        self.set_parameters(Parameters {
            vowel: values[0],
            port_time: values[1],
            delay_mix: values[2],
            voice: values[3],
            vibrato: values[4],
            volume: values[5],
            xy_routing: values[6],
        });
        true
    }

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
    pub fn voice_state(&self) -> VoiceState {
        self.engine.voice_state()
    }

    pub fn pad_events(result: GestureResult, gesture: PadGesture) -> [Event; 3] {
        let note_on = !matches!(gesture, PadGesture::Up);
        [
            Event {
                kind: if note_on {
                    EventType::NoteOn
                } else {
                    EventType::NoteOff
                },
                note: 28,
                value: if note_on { 1.0 } else { 0.0 },
                local_pad: true,
                ..Event::default()
            },
            Event {
                kind: EventType::PadPitch,
                value: result.x,
                local_pad: true,
                ..Event::default()
            },
            Event {
                kind: EventType::PadVowel,
                value: result.vowel,
                local_pad: true,
                ..Event::default()
            },
        ]
    }

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
