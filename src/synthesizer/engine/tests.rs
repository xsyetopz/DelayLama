//! Focused tests for private engine state transitions.

use super::{PAD_INTERNAL_NOTE, SynthEngine, SynthesisEvent, SynthesisEventKind};

#[test]
fn controls_before_local_note_on_start_at_pointer_pitch_without_fixed_carrier() {
    let mut engine = SynthEngine::default();
    engine.apply(SynthesisEvent {
        kind: SynthesisEventKind::PadPitch,
        value: 1.0,
        local_pad: true,
        ..SynthesisEvent::default()
    });
    engine.apply(SynthesisEvent {
        kind: SynthesisEventKind::PadVowel,
        value: 0.5,
        local_pad: true,
        ..SynthesisEvent::default()
    });
    engine.apply(SynthesisEvent {
        kind: SynthesisEventKind::NoteOn,
        note: PAD_INTERNAL_NOTE,
        value: 64.0 / 127.0,
        local_pad: true,
        ..SynthesisEvent::default()
    });

    assert_eq!(engine.notes.voice.current_note, PAD_INTERNAL_NOTE);
    assert!((engine.controls.pitch.current - 48.0).abs() <= f64::EPSILON);
    assert!((engine.controls.pitch.target - 48.0).abs() <= f64::EPSILON);
    assert!((engine.controls.route.current - 48.0).abs() <= f32::EPSILON);
    assert_eq!(engine.controls.route.steps_remaining, 0);
}

#[test]
fn vowel_bend_does_not_retune_grain_overlap_pitch() {
    fn samples_since_last_grain(bend: f32) -> usize {
        let mut engine = SynthEngine::default();
        engine.prepare(44_100.0, 1_024, 2);
        engine.apply(SynthesisEvent {
            kind: SynthesisEventKind::NoteOn,
            note: 48,
            value: 1.0,
            ..SynthesisEvent::default()
        });
        engine.controls.bend.current = bend;
        engine.grain.dirty = false;
        let mut left = vec![0.0; 1_024];
        let mut right = vec![0.0; 1_024];
        engine.process(&mut [&mut left, &mut right], 1_024, &[]);
        engine.grain.overlap.samples_since_grain
    }

    assert_eq!(samples_since_last_grain(0.0), samples_since_last_grain(1.0));
}
