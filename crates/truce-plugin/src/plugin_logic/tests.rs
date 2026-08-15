//! Truce runtime regression tests.

use super::*;
use delaylama_protocol::{SynthesisEvent, SynthesisEventKind};

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
            note: 28,
            value: 64.0 / 127.0,
            local_pad: true,
            ..SynthesisEvent::default()
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
fn changed_knob_still_updates_only_its_parameter() {
    let mut previous = Some([0.5, 0.5, 0.8, 0.5]);
    let current = Parameters {
        vowel: 0.82,
        xy_routing: 0.7,
        ..Parameters::default()
    };
    let merged = merge_changed_host_parameters(current, &mut previous, [0.6, 0.5, 0.8, 0.5])
        .expect("changed vowel knob must be applied");

    assert!((merged.vowel - 0.6).abs() <= f32::EPSILON);
    assert!((merged.xy_routing - 0.7).abs() <= f32::EPSILON);
}
