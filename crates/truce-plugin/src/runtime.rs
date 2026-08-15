//! Audio-thread plugin state and editor-command translation.
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crossbeam_queue::ArrayQueue;

use delaylama_host::ProcessorModel;
use delaylama_protocol::{MidiMessage, to_synthesis_event};
use delaylama_synthesizer::Parameters;
use num_traits::ToPrimitive;

use truce::core::state::StateLoadError;
use truce::params::{
    FloatParamReadF64, ParamFlags, ParamInfo, ParamRange, ParamUnit, ParamValueKind, SmoothingStyle,
};
use truce::prelude::{
    AudioBuffer, AudioConfig, BusLayout, ChannelConfig, Editor, EventBody, EventList, FloatParam,
    InitContext, Params, PluginLogic as PluginLogicTrait, ProcessContext, ProcessStatus,
};

use crate::raw_editor::RawEditor;

#[derive(Clone, Copy, Debug)]
pub(crate) enum PadCommand {
    Down(f32, f32),
    Drag(f32, f32),
    Up,
}

#[derive(Debug)]
pub struct EditorBridge {
    commands: ArrayQueue<PadCommand>,
    animation_frame: AtomicUsize,
}

impl Default for EditorBridge {
    fn default() -> Self {
        Self {
            commands: ArrayQueue::new(64),
            animation_frame: AtomicUsize::new(5),
        }
    }
}

impl EditorBridge {
    pub(crate) fn animation_frame(&self) -> usize {
        self.animation_frame.load(Ordering::Relaxed)
    }

    pub(crate) fn push(&self, command: PadCommand) {
        if let Err(command) = self.commands.push(command) {
            let _ = self.commands.pop();
            let _ = self.commands.push(command);
        }
    }
}

/// Parameters exposed by the plugin host.
pub struct PluginParams {
    /// Vowel/formant position parameter.
    pub vowel: FloatParam,
    /// Portamento time parameter.
    pub port_time: FloatParam,
    /// Delay mix parameter.
    pub delay_mix: FloatParam,
    /// Voice character parameter.
    pub voice: FloatParam,
    /// Lock-free editor-to-audio command bridge.
    pub editor: Arc<EditorBridge>,
}

const PARAMETER_INFOS: [ParamInfo; 4] = [
    parameter_info(0, "Vowel", 0.5),
    parameter_info(1, "Portamento", 0.5),
    parameter_info(2, "Delay", 0.8),
    parameter_info(3, "Voice", 0.5),
];

const fn parameter_info(id: u32, name: &'static str, default_plain: f64) -> ParamInfo {
    ParamInfo {
        id,
        name,
        short_name: name,
        group: "",
        range: ParamRange::Linear { min: 0.0, max: 1.0 },
        default_plain,
        flags: ParamFlags::AUTOMATABLE.union(ParamFlags::CHUNKED),
        unit: ParamUnit::None,
        kind: ParamValueKind::Float,
        midi_map: None,
        midi_channel: None,
    }
}

impl PluginParams {
    /// Creates the four host-visible automation parameters.
    pub fn new() -> Self {
        Self {
            vowel: FloatParam::new(PARAMETER_INFOS[0], SmoothingStyle::None),
            port_time: FloatParam::new(PARAMETER_INFOS[1], SmoothingStyle::None),
            delay_mix: FloatParam::new(PARAMETER_INFOS[2], SmoothingStyle::None),
            voice: FloatParam::new(PARAMETER_INFOS[3], SmoothingStyle::None),
            editor: Arc::new(EditorBridge::default()),
        }
    }

    const fn parameter(&self, id: u32) -> Option<&FloatParam> {
        match id {
            0 => Some(&self.vowel),
            1 => Some(&self.port_time),
            2 => Some(&self.delay_mix),
            3 => Some(&self.voice),
            _ => None,
        }
    }
}

impl Default for PluginParams {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PluginParams {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PluginParams")
            .field("parameter_count", &PARAMETER_INFOS.len())
            .finish()
    }
}

impl truce::params::__private::Sealed for PluginParams {}

impl Params for PluginParams {
    fn param_infos(&self) -> Vec<ParamInfo> {
        Self::param_infos_static()
    }

    fn param_infos_static() -> Vec<ParamInfo> {
        PARAMETER_INFOS.to_vec()
    }

    fn count(&self) -> usize {
        PARAMETER_INFOS.len()
    }

    fn get_normalized(&self, id: u32) -> Option<f64> {
        self.parameter(id).map(|parameter| {
            parameter
                .info
                .range
                .normalize(FloatParamReadF64::value(parameter))
        })
    }

    fn set_normalized(&self, id: u32, value: f64) {
        if let Some(parameter) = self.parameter(id) {
            parameter.set_value(parameter.info.range.denormalize(value));
        }
    }

    fn get_plain(&self, id: u32) -> Option<f64> {
        self.parameter(id).map(FloatParamReadF64::value)
    }

    fn set_plain(&self, id: u32, value: f64) {
        if let Some(parameter) = self.parameter(id) {
            parameter.set_value(value);
        }
    }

    fn format_value(&self, id: u32, value: f64) -> Option<String> {
        self.parameter(id)
            .map(|parameter| truce::params::format_param_value(&parameter.info, value))
    }

    fn parse_value(&self, id: u32, text: &str) -> Option<f64> {
        self.parameter(id).and_then(|_| text.parse().ok())
    }

    fn snap_smoothers(&self) {
        for parameter in [&self.vowel, &self.port_time, &self.delay_mix, &self.voice] {
            parameter.smoother.snap(FloatParamReadF64::value(parameter));
        }
    }

    fn set_sample_rate(&self, sample_rate: f64) {
        for parameter in [&self.vowel, &self.port_time, &self.delay_mix, &self.voice] {
            parameter.smoother.set_sample_rate(sample_rate);
        }
    }

    fn collect_values(&self) -> (Vec<u32>, Vec<f64>) {
        let ids = PARAMETER_INFOS.map(|info| info.id).to_vec();
        let values = [
            FloatParamReadF64::value(&self.vowel),
            FloatParamReadF64::value(&self.port_time),
            FloatParamReadF64::value(&self.delay_mix),
            FloatParamReadF64::value(&self.voice),
        ]
        .to_vec();
        (ids, values)
    }

    fn restore_values(&self, values: &[(u32, f64)]) {
        for (id, value) in values {
            self.set_plain(*id, *value);
        }
    }
}

#[derive(Debug)]
/// Truce plugin behavior implementation.
pub struct PluginLogic;

#[derive(Debug, Default)]
/// Runtime state owned by one active plugin instance.
pub struct PluginState {
    processor: ProcessorModel,
    last_host_parameters: Option<[f32; 4]>,
}

fn merge_changed_host_parameters(
    current: Parameters,
    previous: &mut Option<[f32; 4]>,
    next: [f32; 4],
) -> Option<Parameters> {
    let old = previous.replace(next);
    if old == Some(next) {
        return None;
    }

    let mut merged = current;
    if old.is_none_or(|values| values[0].to_bits() != next[0].to_bits()) {
        merged.vowel = next[0];
    }
    if old.is_none_or(|values| values[1].to_bits() != next[1].to_bits()) {
        merged.port_time = next[1];
    }
    if old.is_none_or(|values| values[2].to_bits() != next[2].to_bits()) {
        merged.delay_mix = next[2];
    }
    if old.is_none_or(|values| values[3].to_bits() != next[3].to_bits()) {
        merged.voice = next[3];
    }
    Some(merged)
}

impl PluginLogicTrait for PluginLogic {
    type Params = PluginParams;
    type DspState = PluginState;

    /// Declares the synth's existing stereo output without an effect input bus.
    fn bus_layouts() -> Vec<BusLayout> {
        vec![BusLayout::new().with_output("Main", ChannelConfig::Stereo)]
    }

    fn init(_params: &PluginParams, _cx: &InitContext) -> Self::DspState {
        PluginState::default()
    }
    fn reset(state: &mut PluginState, _params: &PluginParams, config: &AudioConfig) {
        state
            .processor
            .prepare(config.sample_rate, config.max_block_size);
        state.last_host_parameters = None;
    }
    fn save_state(state: &PluginState) -> Vec<u8> {
        state.processor.save_state().to_vec()
    }
    fn load_state(state: &mut PluginState, data: &[u8]) -> Result<(), StateLoadError> {
        if state.processor.load_state(data) {
            Ok(())
        } else {
            Err(StateLoadError::Malformed("DLM1 state"))
        }
    }
    fn process(
        state: &mut PluginState,
        params: &PluginParams,
        buffer: &mut AudioBuffer,
        events: &EventList,
        _cx: &mut ProcessContext,
    ) -> ProcessStatus {
        while let Some(command) = params.editor.commands.pop() {
            apply_pad_command(&mut state.processor, command);
        }
        let host_parameters = [
            truce::params::FloatParamReadF32::value(&params.vowel),
            truce::params::FloatParamReadF32::value(&params.port_time),
            truce::params::FloatParamReadF32::value(&params.delay_mix),
            truce::params::FloatParamReadF32::value(&params.voice),
        ];
        if let Some(parameters) = merge_changed_host_parameters(
            state.processor.parameters(),
            &mut state.last_host_parameters,
            host_parameters,
        ) {
            state.processor.set_parameters(parameters);
        }
        let mut host_events = events.iter().peekable();
        let mut sample_offset = 0;
        buffer.for_each_stereo_frame(|_, output| {
            while host_events
                .peek()
                .is_some_and(|event| event.sample_offset <= sample_offset)
            {
                let Some(event) = host_events.next() else {
                    break;
                };
                let message = match event.body {
                    EventBody::NoteOn { note, velocity, .. } => MidiMessage::NoteOn {
                        note: i32::from(note),
                        velocity: i32::from(velocity),
                    },
                    EventBody::NoteOff { note, velocity, .. } => MidiMessage::NoteOff {
                        note: i32::from(note),
                        velocity: i32::from(velocity),
                    },
                    EventBody::PitchBend { value, .. } => MidiMessage::PitchBend {
                        value: i32::from(value),
                    },
                    EventBody::ControlChange { cc, value, .. } => MidiMessage::ControlChange {
                        controller: i32::from(cc),
                        value: i32::from(value),
                    },
                    EventBody::Aftertouch { .. }
                    | EventBody::ChannelPressure { .. }
                    | EventBody::ProgramChange { .. }
                    | EventBody::NoteOn2 { .. }
                    | EventBody::NoteOff2 { .. }
                    | EventBody::PolyPressure2 { .. }
                    | EventBody::PerNoteCC { .. }
                    | EventBody::PerNotePitchBend { .. }
                    | EventBody::PerNoteManagement { .. }
                    | EventBody::ControlChange2 { .. }
                    | EventBody::ChannelPressure2 { .. }
                    | EventBody::PitchBend2 { .. }
                    | EventBody::ProgramChange2 { .. }
                    | EventBody::RegisteredController { .. }
                    | EventBody::AssignableController { .. }
                    | EventBody::ParamChange { .. }
                    | EventBody::ParamMod { .. }
                    | EventBody::Transport(_)
                    | EventBody::SysEx { .. } => continue,
                };
                if let Some(synthesis_event) = to_synthesis_event(message, 0) {
                    state.processor.apply_event(synthesis_event);
                }
            }
            *output = state.processor.render_stereo_frame();
            sample_offset += 1;
        });
        let visual = state.processor.visual_state();
        let selector = if visual.atlas_selector.is_finite() && visual.atlas_selector >= 0.0 {
            visual.atlas_selector.clamp(0.0, 1.0)
        } else if visual.gate {
            0.8_f32.mul_add(visual.vowel.clamp(0.0, 1.0), 0.2)
        } else {
            5.0 / 30.0
        };
        let frame = selector
            .mul_add(29.0, 0.5)
            .min(29.0)
            .to_usize()
            .unwrap_or(0);
        params
            .editor
            .animation_frame
            .store(frame, Ordering::Relaxed);
        ProcessStatus::Normal
    }
    fn editor(params: Arc<PluginParams>) -> Box<dyn Editor> {
        Box::new(RawEditor::new(params))
    }
}

fn apply_pad_command(processor: &mut ProcessorModel, command: PadCommand) {
    match command {
        PadCommand::Down(x, y) => {
            processor.apply_event(delaylama_synthesizer::SynthesisEvent {
                kind: delaylama_synthesizer::SynthesisEventKind::PadPitch,
                value: x,
                local_pad: true,
                ..delaylama_synthesizer::SynthesisEvent::default()
            });
            processor.apply_event(delaylama_synthesizer::SynthesisEvent {
                kind: delaylama_synthesizer::SynthesisEventKind::PadVowel,
                value: 1.0 - y,
                local_pad: true,
                ..delaylama_synthesizer::SynthesisEvent::default()
            });
            processor.apply_event(delaylama_synthesizer::SynthesisEvent {
                kind: delaylama_synthesizer::SynthesisEventKind::NoteOn,
                note: 28,
                value: 64.0 / 127.0,
                local_pad: true,
                ..delaylama_synthesizer::SynthesisEvent::default()
            });
        }
        PadCommand::Drag(x, y) => {
            processor.apply_event(delaylama_synthesizer::SynthesisEvent {
                kind: delaylama_synthesizer::SynthesisEventKind::PadPitch,
                value: x,
                local_pad: true,
                ..delaylama_synthesizer::SynthesisEvent::default()
            });
            processor.apply_event(delaylama_synthesizer::SynthesisEvent {
                kind: delaylama_synthesizer::SynthesisEventKind::PadVowel,
                value: 1.0 - y,
                local_pad: true,
                ..delaylama_synthesizer::SynthesisEvent::default()
            });
        }
        PadCommand::Up => processor.apply_event(delaylama_synthesizer::SynthesisEvent {
            kind: delaylama_synthesizer::SynthesisEventKind::NoteOff,
            note: 28,
            local_pad: true,
            ..delaylama_synthesizer::SynthesisEvent::default()
        }),
    }
}

#[cfg(test)]
/// Runtime behavior tests kept beside the real-time adapter.
#[path = "plugin_logic/tests.rs"]
mod tests;
