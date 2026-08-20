//! Audio-thread plugin state and editor-command translation.

use std::sync::Arc;

use truce::core::state::StateLoadError;
use truce::prelude::{
    AudioBuffer, AudioConfig, BusLayout, ChannelConfig, Editor, EventBody, EventList, InitContext,
    PluginLogic as PluginLogicTrait, ProcessContext, ProcessStatus,
};

use crate::{
    host::ProcessorModel,
    protocol::{MidiMessage, to_synthesis_event},
};

use super::{
    parameter::PluginParameter,
    params::PluginParams,
    raw_editor::{RawEditor, animation_frame},
};

/// Plugin processing behavior.
#[derive(Debug)]
pub struct PluginLogic;

/// Runtime state owned by one active plugin instance.
#[derive(Debug, Default)]
pub struct PluginState {
    processor: ProcessorModel,
    last_host_parameters: Option<[f32; 4]>,
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
        while let Some(command) = params.editor.pop() {
            state.processor.apply_pad_gesture(command);
        }

        let host_parameters = PluginParameter::ALL.map(|parameter| params.value(parameter));
        state
            .processor
            .apply_changed_host_parameters(&mut state.last_host_parameters, host_parameters);

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

        let frame = animation_frame(state.processor.visual_state());
        params.editor.publish_animation_frame(frame);
        ProcessStatus::Normal
    }

    fn editor(params: Arc<PluginParams>) -> Box<dyn Editor> {
        Box::new(RawEditor::new(params))
    }
}

#[cfg(test)]
mod tests;
