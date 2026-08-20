//! Plugin runtime regression tests.

use super::*;
use crate::{
    protocol::{PAD_INTERNAL_NOTE, SynthesisEvent, SynthesisEventKind},
    synthesizer::Parameters,
};

#[test]
fn au_block_parameter_polling_does_not_overwrite_rendered_pad_vowel() {
    let mut state = PluginState::default();
    state.processor.prepare(44_100.0, 512);
    state.last_host_parameters = Some([0.5, 0.5, 0.8, 0.5]);
    let events = [
        SynthesisEvent {
            kind: SynthesisEventKind::PadPitch,
            value: 0.7,
            local_pad: true,
            ..SynthesisEvent::default()
        },
        SynthesisEvent {
            kind: SynthesisEventKind::PadVowel,
            value: 0.82,
            local_pad: true,
            ..SynthesisEvent::default()
        },
        SynthesisEvent {
            kind: SynthesisEventKind::NoteOn,
            note: PAD_INTERNAL_NOTE,
            value: 64.0 / 127.0,
            local_pad: true,
            ..SynthesisEvent::default()
        },
    ];
    let mut rendered_energy = 0.0;
    for block in 0..12 {
        assert!(
            !state.processor.apply_changed_host_parameters(
                &mut state.last_host_parameters,
                [0.5, 0.5, 0.8, 0.5],
            )
        );
        let mut left = vec![0.0; 512];
        let mut right = vec![0.0; 512];
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
fn editor_bridge_carries_editor_gesture_through_the_host_owner() {
    use crate::{
        plugin::raw_editor::{PointerPhase, pad_gesture},
        protocol::{PAD_HOST_NOTE, PAD_INTERNAL_NOTE},
    };

    let params = PluginParams::new();
    params
        .editor
        .push(pad_gesture(0.7, 0.2, PointerPhase::Down));
    let gesture = params.editor.pop();
    assert!(gesture.is_some(), "asset-editor gesture was not queued");
    let Some(gesture) = gesture else {
        return;
    };
    assert_eq!(
        gesture.transition,
        crate::protocol::GestureTransition::NoteOn(PAD_HOST_NOTE)
    );

    let mut processor = ProcessorModel::default();
    processor.prepare(44_100.0, 64);
    processor.apply_pad_gesture(gesture);
    assert_eq!(processor.voice_state().current_note, PAD_INTERNAL_NOTE);
    assert!(processor.voice_state().gate);
    assert!((processor.visual_state().vowel - 0.8).abs() <= f32::EPSILON);
}

#[test]
fn changed_knob_still_updates_only_its_parameter() {
    let mut previous = Some([0.5, 0.5, 0.8, 0.5]);
    let mut processor = ProcessorModel::default();
    processor.set_parameters(Parameters {
        vowel: 0.82,
        xy_routing: 0.7,
        ..Parameters::default()
    });
    assert!(processor.apply_changed_host_parameters(&mut previous, [0.6, 0.5, 0.8, 0.5]));
    let merged = processor.parameters();
    assert!((merged.vowel - 0.6).abs() <= f32::EPSILON);
    assert!((merged.xy_routing - 0.7).abs() <= f32::EPSILON);
}
