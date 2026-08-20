//! Granular formant synthesis engine.

mod events;
mod render;
mod voice;

use num_traits::ToPrimitive;

use super::state::{
    DEFAULT_SAMPLE_RATE, MAX_SAMPLE_RATE, MIN_SAMPLE_RATE, PadState, Parameters, VoiceState,
};
use super::tables;
use crate::protocol::{
    CC_DELAY_MIX, CC_PORT_TIME, CC_VIBRATO, CC_VOICE, CC_VOLUME, CC_XY_ROUTING, CC7_VOLUME_SCALE,
    PAD_INTERNAL_NOTE, SynthesisEvent, SynthesisEventKind,
};

const GRAIN_SECONDS: f64 = 0.02;
/// Granular formant synthesizer that keeps its render state between blocks.
#[derive(Debug)]
pub struct SynthEngine {
    configuration: EngineConfiguration,
    parameters: Parameters,
    notes: NoteState,
    controls: ControlState,
    oscillator: OscillatorState,
    delay: DelayState,
    grain: GrainState,
    atlas: AtlasState,
}

#[derive(Debug)]
struct EngineConfiguration {
    sample_rate: f64,
}

#[derive(Debug)]
struct NoteState {
    voice: VoiceState,
    pad: PadState,
    stack: [i32; 128],
    local_pad_active: bool,
    external_active: [bool; 128],
}

#[derive(Debug)]
struct ControlState {
    bend: SmoothingRamp,
    pitch: PitchState,
    route: SmoothingRamp,
    update: UpdateClock,
}

#[derive(Debug)]
struct SmoothingRamp {
    current: f32,
    target: f32,
    increment: f32,
    steps_remaining: i32,
}

#[derive(Debug)]
struct PitchState {
    current: f64,
    target: f64,
    legato: bool,
}

#[derive(Debug)]
struct UpdateClock {
    counter: usize,
    interval: usize,
}

#[derive(Debug)]
struct OscillatorState {
    phase: f64,
    vibrato: VibratoState,
    random_state: u32,
}

#[derive(Debug)]
struct VibratoState {
    phase: f64,
    rate_hz: f32,
    refresh: UpdateClock,
}

#[derive(Debug)]
struct DelayState {
    buffers: StereoDelayBuffers,
    cursors: DelayCursors,
}

#[derive(Debug)]
struct StereoDelayBuffers {
    left: Vec<f32>,
    right: Vec<f32>,
}

#[derive(Debug)]
struct DelayCursors {
    write: usize,
    left_tap: usize,
    right_tap: usize,
}

#[derive(Debug)]
struct GrainState {
    samples: Vec<f32>,
    source: GrainSource,
    overlap: OverlapState,
    dirty: bool,
}

#[derive(Debug)]
struct GrainSource {
    formants: [Vec<f32>; 3],
    window: Vec<f32>,
    excitation: Vec<f32>,
}

#[derive(Debug)]
struct OverlapState {
    ring: Vec<f32>,
    cursor: usize,
    samples_since_grain: usize,
}

#[derive(Debug)]
struct AtlasState {
    selector: f32,
    dirty: bool,
    tick_samples: usize,
    timing: AtlasTiming,
    idle_index: usize,
}

#[derive(Debug)]
struct AtlasTiming {
    idle_elapsed: usize,
    tick_counter: usize,
}

impl Default for SynthEngine {
    fn default() -> Self {
        Self {
            configuration: EngineConfiguration {
                sample_rate: DEFAULT_SAMPLE_RATE,
            },
            parameters: Parameters::default(),
            notes: NoteState {
                voice: VoiceState {
                    current_note: -1,
                    gate: false,
                },
                pad: PadState::default(),
                stack: [-1; 128],
                local_pad_active: false,
                external_active: [false; 128],
            },
            controls: ControlState {
                bend: SmoothingRamp {
                    current: 8_192.0 / 16_384.0,
                    target: 8_192.0 / 16_384.0,
                    increment: 0.0,
                    steps_remaining: 0,
                },
                pitch: PitchState {
                    current: 36.0,
                    target: 36.0,
                    legato: false,
                },
                route: SmoothingRamp {
                    current: 36.0,
                    target: 36.0,
                    increment: 0.0,
                    steps_remaining: 0,
                },
                update: UpdateClock {
                    counter: 0,
                    interval: 441,
                },
            },
            oscillator: OscillatorState {
                phase: 0.0,
                vibrato: VibratoState {
                    phase: 0.0,
                    rate_hz: 4.0,
                    refresh: UpdateClock {
                        counter: 0,
                        interval: 4586,
                    },
                },
                random_state: 1,
            },
            delay: DelayState {
                buffers: StereoDelayBuffers {
                    left: vec![0.0; 20_000],
                    right: vec![0.0; 20_000],
                },
                cursors: DelayCursors {
                    write: 0,
                    left_tap: 0,
                    right_tap: 0,
                },
            },
            grain: GrainState {
                samples: Vec::new(),
                source: GrainSource {
                    formants: tables::formant_tables(),
                    window: Vec::new(),
                    excitation: Vec::new(),
                },
                overlap: OverlapState {
                    ring: vec![0.0; 20_000],
                    cursor: 0,
                    samples_since_grain: 0,
                },
                dirty: true,
            },
            atlas: AtlasState {
                selector: 0.0,
                dirty: true,
                tick_samples: 1,
                timing: AtlasTiming {
                    idle_elapsed: 0,
                    tick_counter: 0,
                },
                idle_index: 0,
            },
        }
    }
}

impl SynthEngine {
    /// Prepares buffers and timing for a sample rate and channel count.
    pub fn prepare(&mut self, sample_rate: f64, _max_block_size: usize, _channel_count: usize) {
        self.configuration.sample_rate = if sample_rate.is_finite() {
            sample_rate.clamp(MIN_SAMPLE_RATE, MAX_SAMPLE_RATE)
        } else {
            DEFAULT_SAMPLE_RATE
        };

        self.delay.buffers.left = vec![0.0; 20_000];
        self.delay.buffers.right = vec![0.0; 20_000];

        let grain_samples = (self.configuration.sample_rate * GRAIN_SECONDS)
            .to_usize()
            .unwrap_or(1);
        self.grain.samples = vec![0.0; grain_samples.max(1)];
        self.grain.overlap.ring = vec![0.0; 20_000];
        self.grain.source.window =
            tables::window(self.configuration.sample_rate, self.grain.samples.len());
        self.grain.source.excitation =
            tables::excitation(self.configuration.sample_rate, self.grain.samples.len());

        self.atlas.tick_samples = (self.configuration.sample_rate * 0.208)
            .max(1.0)
            .to_usize()
            .unwrap_or(1);

        self.reset();
    }

    /// Clears voices, modulation, grain history, delay, and artwork state.
    pub fn reset(&mut self) {
        self.notes.voice = VoiceState {
            current_note: -1,
            gate: false,
        };
        self.notes.pad = PadState::default();

        self.controls.bend.current = 8_192.0 / 16_384.0;
        self.controls.bend.target = self.controls.bend.current;
        self.controls.bend.increment = 0.0;
        self.controls.bend.steps_remaining = 0;
        self.controls.update.counter = 0;
        self.controls.update.interval = (self.configuration.sample_rate * 0.01)
            .to_usize()
            .unwrap_or(1);

        self.controls.pitch.current = 36.0;
        self.controls.pitch.target = 36.0;
        self.controls.pitch.legato = false;
        self.controls.route.current = 36.0;
        self.controls.route.target = 36.0;
        self.controls.route.increment = 0.0;
        self.controls.route.steps_remaining = 0;

        self.oscillator.phase = 0.0;
        self.oscillator.vibrato.phase = 0.0;
        self.oscillator.vibrato.rate_hz = 4.0;
        self.oscillator.vibrato.refresh.counter = 0;
        self.oscillator.vibrato.refresh.interval = (self.configuration.sample_rate * 0.104)
            .to_usize()
            .unwrap_or(1);
        self.oscillator.random_state = 1;

        self.grain.overlap.ring.fill(0.0);
        self.grain.overlap.cursor = 0;
        self.grain.overlap.samples_since_grain = 0;
        self.grain.dirty = true;

        self.delay.buffers.left.fill(0.0);
        self.delay.buffers.right.fill(0.0);
        self.delay.cursors.write = 0;
        self.delay.cursors.left_tap = (self.delay.buffers.left.len()
            - (self.configuration.sample_rate * 0.309_592)
                .to_usize()
                .unwrap_or(0)
                % self.delay.buffers.left.len())
            % self.delay.buffers.left.len();
        self.delay.cursors.right_tap = (self.delay.buffers.right.len()
            - (self.configuration.sample_rate * 0.398_435)
                .to_usize()
                .unwrap_or(0)
                % self.delay.buffers.right.len())
            % self.delay.buffers.right.len();

        self.notes.stack = [-1; 128];
        self.notes.local_pad_active = false;
        self.notes.external_active = [false; 128];

        self.atlas.selector = 0.0;
        self.atlas.dirty = true;
        self.atlas.timing.tick_counter = 0;
        self.atlas.timing.idle_elapsed = 0;
        self.atlas.idle_index = 0;
    }

    /// Applies synthesis settings from the host.
    pub fn set_parameters(&mut self, parameters: Parameters) {
        let vowel_changed = clamp(parameters.vowel, 0.5)
            .partial_cmp(&self.parameters.vowel)
            .is_none_or(|ordering| !ordering.is_eq());
        let voice_changed = clamp(parameters.voice, 0.5)
            .partial_cmp(&self.parameters.voice)
            .is_none_or(|ordering| !ordering.is_eq());
        self.atlas.dirty = self.atlas.dirty || vowel_changed;
        self.grain.dirty = self.grain.dirty || vowel_changed || voice_changed;
        self.parameters = Parameters {
            vowel: clamp(parameters.vowel, 0.5),
            port_time: clamp(parameters.port_time, 0.5),
            delay_mix: clamp(parameters.delay_mix, 0.8),
            voice: clamp(parameters.voice, 0.5),
            vibrato: clamp(parameters.vibrato, 0.0),
            volume: clamp(parameters.volume, 0.1),
            xy_routing: clamp(parameters.xy_routing, 0.0),
        };
        if vowel_changed && self.notes.voice.gate {
            self.atlas.selector = 0.8_f32.mul_add(self.parameters.vowel, 0.2);
            self.atlas.dirty = false;
        }
    }

    /// Returns the currently active synthesis parameters.
    pub const fn parameters(&self) -> Parameters {
        self.parameters
    }

    /// Returns the monophonic voice state.
    pub const fn voice_state(&self) -> VoiceState {
        self.notes.voice
    }

    /// Returns the current editor-pad state.
    pub const fn pad_state(&self) -> PadState {
        self.notes.pad
    }

    /// Returns the current artwork frame selector.
    pub const fn atlas_selector(&self) -> f32 {
        self.atlas.selector
    }
}

const fn clamp(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}
