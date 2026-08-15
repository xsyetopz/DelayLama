use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crossbeam_queue::ArrayQueue;

use delaylama_core::{Event as CoreEvent, EventType, Parameters};
use delaylama_host::ProcessorModel;
use delaylama_protocol::{CoreEventKind, MidiMessage, to_core_event};

use truce::core::state::StateLoadError;
use truce::prelude::PluginLogic as PluginLogicTrait;
use truce::prelude::*;

fn trace_line(message: &str) {
    log::info!(target: "openlama.pad_trace", "{message}");
}

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
            let _discarded = self.commands.pop();
            let _result = self.commands.push(command);
        }
    }
}

#[expect(
    missing_debug_implementations,
    reason = "truce parameter type does not implement Debug"
)]
#[derive(Params)]
pub struct PluginParams {
    #[param(name = "Vowel", range = "linear(0, 1)", default = 0.5)]
    pub vowel: FloatParam,
    #[param(name = "Portamento", range = "linear(0, 1)", default = 0.5)]
    pub port_time: FloatParam,
    #[param(name = "Delay", range = "linear(0, 1)", default = 0.8)]
    pub delay_mix: FloatParam,
    #[param(name = "Voice", range = "linear(0, 1)", default = 0.5)]
    pub voice: FloatParam,
    #[skip]
    pub editor: Arc<EditorBridge>,
}

#[derive(Debug)]
pub struct PluginLogic;

#[derive(Debug, Default)]
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
    if old.is_none_or(|values| values[0] != next[0]) {
        merged.vowel = next[0];
    }
    if old.is_none_or(|values| values[1] != next[1]) {
        merged.port_time = next[1];
    }
    if old.is_none_or(|values| values[2] != next[2]) {
        merged.delay_mix = next[2];
    }
    if old.is_none_or(|values| values[3] != next[3]) {
        merged.voice = next[3];
    }
    Some(merged)
}

impl PluginLogicTrait for PluginLogic {
    type Params = PluginParams;
    type DspState = PluginState;

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
        let mut mapped: Vec<CoreEvent> = events
            .iter()
            .filter_map(|event| {
                trace_line(&format!(
                    "host event: {:?} offset={}",
                    event.body, event.sample_offset
                ));
                let message = match event.body {
                    EventBody::NoteOn { note, velocity, .. } => MidiMessage::NoteOn {
                        note: i32::from(note),
                        velocity: i32::from(velocity),
                    },
                    EventBody::NoteOff { note, .. } => MidiMessage::NoteOff {
                        note: i32::from(note),
                        velocity: 0,
                    },
                    EventBody::PitchBend { value, .. } => MidiMessage::PitchBend {
                        value: i32::from(value),
                    },
                    _ => return None,
                };
                to_core_event(message, event.sample_offset as i32).map(|event| CoreEvent {
                    kind: match event.kind {
                        CoreEventKind::NoteOn => EventType::NoteOn,
                        CoreEventKind::NoteOff => EventType::NoteOff,
                        CoreEventKind::PitchBend => EventType::PitchBend,
                        CoreEventKind::ControlChange => EventType::ControlChange,
                    },
                    sample_offset: event.sample_offset,
                    note: event.note,
                    value: event.value,
                    controller: event.controller,
                    local_pad: event.local_pad,
                })
            })
            .collect();
        while let Some(command) = params.editor.commands.pop() {
            trace_line(&format!("editor command: {:?}", command));
            match command {
                PadCommand::Down(x, y) => {
                    // Establish controls before opening the gate so the first rendered grain
                    // starts at the pointer pitch instead of the pad's lifecycle note.
                    mapped.push(CoreEvent {
                        kind: EventType::PadPitch,
                        sample_offset: 0,
                        note: -1,
                        value: x,
                        controller: 0,
                        local_pad: true,
                    });
                    mapped.push(CoreEvent {
                        kind: EventType::PadVowel,
                        sample_offset: 0,
                        note: -1,
                        value: 1.0 - y,
                        controller: 0,
                        local_pad: true,
                    });
                    mapped.push(CoreEvent {
                        kind: EventType::NoteOn,
                        sample_offset: 0,
                        note: 28,
                        value: 64.0 / 127.0,
                        controller: 0,
                        local_pad: true,
                    });
                }
                PadCommand::Drag(x, y) => {
                    mapped.push(CoreEvent {
                        kind: EventType::PadPitch,
                        sample_offset: 0,
                        note: -1,
                        value: x,
                        controller: 0,
                        local_pad: true,
                    });
                    mapped.push(CoreEvent {
                        kind: EventType::PadVowel,
                        sample_offset: 0,
                        note: -1,
                        value: 1.0 - y,
                        controller: 0,
                        local_pad: true,
                    });
                }
                PadCommand::Up => mapped.push(CoreEvent {
                    kind: EventType::NoteOff,
                    sample_offset: 0,
                    note: 28,
                    value: 0.0,
                    controller: 0,
                    local_pad: true,
                }),
            }
        }
        let host_parameters = [
            params.vowel.value() as f32,
            params.port_time.value() as f32,
            params.delay_mix.value() as f32,
            params.voice.value() as f32,
        ];
        if let Some(parameters) = merge_changed_host_parameters(
            state.processor.parameters(),
            &mut state.last_host_parameters,
            host_parameters,
        ) {
            state.processor.set_parameters(parameters);
        }
        let n = buffer.num_samples();
        let mut left = vec![0.0; n];
        let mut right = vec![0.0; n];
        let mut outputs: Vec<&mut [f32]> = vec![&mut left, &mut right];
        state.processor.process(&mut outputs, n, &mapped);
        let visual = state.processor.visual_state();
        let selector = if visual.atlas_selector.is_finite() && visual.atlas_selector >= 0.0 {
            visual.atlas_selector.clamp(0.0, 1.0)
        } else if visual.gate {
            0.2 + 0.8 * visual.vowel.clamp(0.0, 1.0)
        } else {
            5.0 / 30.0
        };
        let frame = (selector * 29.0 + 0.5).min(29.0) as usize;
        params
            .editor
            .animation_frame
            .store(frame, Ordering::Relaxed);
        for channel in 0..buffer.num_output_channels() {
            buffer.output(channel)[..n].copy_from_slice(if channel % 2 == 0 {
                &left
            } else {
                &right
            });
        }
        ProcessStatus::Normal
    }
    fn editor(params: Arc<PluginParams>) -> Box<dyn Editor> {
        Box::new(crate::raw_editor::RawEditor::new(params))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn au_block_parameter_polling_does_not_overwrite_rendered_pad_vowel() {
        let mut state = PluginState::default();
        state.processor.prepare(44_100.0, 512);
        state.last_host_parameters = Some([0.5, 0.5, 0.8, 0.5]);
        let events = [
            CoreEvent {
                kind: EventType::PadPitch,
                value: 0.7,
                local_pad: true,
                ..CoreEvent::default()
            },
            CoreEvent {
                kind: EventType::PadVowel,
                value: 0.82,
                local_pad: true,
                ..CoreEvent::default()
            },
            CoreEvent {
                kind: EventType::NoteOn,
                note: 28,
                value: 64.0 / 127.0,
                local_pad: true,
                ..CoreEvent::default()
            },
        ];
        let mut rendered_energy = 0.0;
        for block in 0..12 {
            let current = state.processor.parameters();
            if let Some(parameters) = merge_changed_host_parameters(
                current,
                &mut state.last_host_parameters,
                [0.5, 0.5, 0.8, 0.5],
            ) {
                state.processor.set_parameters(parameters);
            }
            let mut left = [0.0; 512];
            let mut right = [0.0; 512];
            state.processor.process(
                &mut [&mut left, &mut right],
                512,
                if block == 0 { &events } else { &[] },
            );
            rendered_energy += left.iter().map(|sample| sample.abs()).sum::<f32>();
        }

        assert!(rendered_energy > 0.0);
        assert!((state.processor.parameters().vowel - 0.82).abs() < 0.001);
    }

    #[test]
    fn changed_knob_still_updates_only_its_parameter() {
        let mut previous = Some([0.5, 0.5, 0.8, 0.5]);
        let current = Parameters {
            vowel: 0.82,
            xy_routing: 0.7,
            ..Parameters::default()
        };
        let merged = merge_changed_host_parameters(current, &mut previous, [0.6, 0.5, 0.8, 0.5])
            .expect("changed vowel knob must be applied");

        assert_eq!(merged.vowel, 0.6);
        assert_eq!(merged.xy_routing, 0.7);
    }
}
