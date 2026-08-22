//! Synthesizer behavior and regression tests.

use xymonk::{
    protocol::{PAD_INTERNAL_NOTE, SynthesisEvent, SynthesisEventKind},
    synthesizer::{Parameters, SynthEngine},
};

fn note_on(note: i32) -> SynthesisEvent {
    SynthesisEvent {
        kind: SynthesisEventKind::NoteOn,
        note,
        value: 1.0,
        ..SynthesisEvent::default()
    }
}

fn note_off(note: i32) -> SynthesisEvent {
    SynthesisEvent {
        kind: SynthesisEventKind::NoteOff,
        note,
        ..SynthesisEvent::default()
    }
}

fn local_pad_down(value: f32) -> SynthesisEvent {
    SynthesisEvent {
        kind: SynthesisEventKind::NoteOn,
        note: PAD_INTERNAL_NOTE,
        value,
        local_pad: true,
        ..SynthesisEvent::default()
    }
}

fn local_pad_up() -> SynthesisEvent {
    SynthesisEvent {
        kind: SynthesisEventKind::NoteOff,
        note: PAD_INTERNAL_NOTE,
        local_pad: true,
        ..SynthesisEvent::default()
    }
}

fn prepared_dry_engine(max_block_size: usize) -> SynthEngine {
    let mut engine = SynthEngine::default();
    engine.prepare(44_100.0, max_block_size, 2);
    engine.set_parameters(Parameters {
        delay_mix: 0.0,
        volume: 0.5,
        ..Parameters::default()
    });
    engine
}

fn assert_tail_is_silent(samples: &[f32]) {
    assert!(
        samples
            .get(20_000..)
            .is_some_and(|tail| tail.iter().all(|sample| sample.abs() <= f32::EPSILON)),
        "released local pad must not leave a continuous voice"
    );
}

fn release_local_pad_and_assert_silence(
    engine: &mut SynthEngine,
    left: &mut [f32],
    right: &mut [f32],
) {
    engine.process(&mut [&mut *left, &mut *right], 25_000, &[local_pad_up()]);
    assert!(!engine.voice_state().gate);
    assert_tail_is_silent(left);
}

#[test]
fn sanitises_parameters() {
    let mut e = SynthEngine::default();
    e.set_parameters(Parameters {
        vowel: f32::NAN,
        ..Parameters::default()
    });
    assert!((e.parameters().vowel - 0.5).abs() <= f32::EPSILON);
}

#[test]
fn note_on_renders_audio_and_updates_voice() {
    let mut e = SynthEngine::default();
    e.prepare(44100.0, 64, 2);
    let ev = [note_on(69)];
    let mut l = [0.0; 64];
    let mut r = [0.0; 64];
    e.process(&mut [&mut l, &mut r], 64, &ev);
    assert!(l.iter().any(|x| x.abs() > 0.0));
    assert_eq!(e.voice_state().current_note, 69);
}

#[test]
fn controls_pad() {
    let mut e = SynthEngine::default();
    e.process(
        &mut [],
        1,
        &[SynthesisEvent {
            kind: SynthesisEventKind::PadVowel,
            value: 0.08,
            ..SynthesisEvent::default()
        }],
    );
    assert!(e.pad_state().active);
    assert!((e.pad_state().vowel - 0.08).abs() <= f32::EPSILON);
}

#[test]
fn note_event_starts_at_sample_offset() {
    let mut engine = SynthEngine::default();
    engine.prepare(44_100.0, 64, 2);
    let event = SynthesisEvent {
        kind: SynthesisEventKind::NoteOn,
        sample_offset: 16,
        note: 69,
        value: 1.0,
        ..SynthesisEvent::default()
    };
    let mut left = [0.0; 32];
    let mut right = [0.0; 32];
    engine.process(&mut [&mut left, &mut right], 32, &[event]);
    assert!(
        left.get(..16)
            .is_some_and(|samples| samples.iter().all(|sample| sample.abs() <= f32::EPSILON))
    );
    assert!(
        left.get(16..)
            .is_some_and(|samples| samples.iter().any(|sample| sample.abs() > 0.0))
    );
}

#[test]
fn midi_controller_ids_match_core_protocol() {
    let mut engine = SynthEngine::default();
    engine.process(
        &mut [],
        0,
        &[SynthesisEvent {
            kind: SynthesisEventKind::ControlChange,
            controller: 12,
            value: 0.25,
            ..SynthesisEvent::default()
        }],
    );
    assert!((engine.parameters().delay_mix - 0.25).abs() <= f32::EPSILON);
}

#[test]
fn duplicate_note_on_is_idempotent_and_one_note_off_restores_previous_note() {
    let mut engine = SynthEngine::default();
    engine.process(&mut [], 0, &[note_on(60)]);
    engine.process(&mut [], 0, &[note_on(64)]);
    engine.process(&mut [], 0, &[note_on(64)]);
    engine.process(&mut [], 0, &[note_off(64)]);
    assert_eq!(engine.voice_state().current_note, 60);
}

#[test]
fn controller_7_scales_volume() {
    let mut engine = SynthEngine::default();
    engine.process(
        &mut [],
        0,
        &[SynthesisEvent {
            kind: SynthesisEventKind::ControlChange,
            controller: 7,
            value: 127.0,
            ..SynthesisEvent::default()
        }],
    );
    assert!((engine.parameters().volume - 0.127).abs() < 0.0001);
}

#[test]
fn local_pad_note_enters_audio_voice_and_renders() {
    let mut engine = SynthEngine::default();
    engine.prepare(44_100.0, 64, 2);
    let mut left = [0.0; 64];
    let mut right = [0.0; 64];
    engine.process(&mut [&mut left, &mut right], 64, &[local_pad_down(1.0)]);
    assert_eq!(engine.voice_state().current_note, PAD_INTERNAL_NOTE);
    assert!(engine.pad_state().active);
    assert!(left.iter().any(|sample| sample.abs() > 0.0));

    engine.process(&mut [&mut left, &mut right], 1, &[local_pad_up()]);
    assert!(!engine.voice_state().gate);
    assert!(!engine.pad_state().active);
}

fn render_note_with_voice(note: i32, voice: f32) -> Vec<f32> {
    let mut engine = SynthEngine::default();
    engine.prepare(44_100.0, 2_048, 2);
    engine.set_parameters(Parameters {
        volume: 0.5,
        voice,
        delay_mix: 0.0,
        ..Parameters::default()
    });
    let mut left = vec![0.0; 2_048];
    let mut right = vec![0.0; 2_048];
    engine.process(
        &mut [&mut left, &mut right],
        2_048,
        &[SynthesisEvent {
            kind: SynthesisEventKind::NoteOn,
            note,
            value: 1.0,
            ..SynthesisEvent::default()
        }],
    );
    left
}

#[test]
fn note_pitch_drives_pitch_synchronous_grain_overlap() {
    let low = render_note_with_voice(36, 0.5);
    let high = render_note_with_voice(60, 0.5);
    assert_ne!(low, high, "note pitch must affect audible overlap timing");
}

#[test]
fn every_voice_setting_rebuilds_the_formant_grain() {
    let low_voice = render_note_with_voice(48, 0.0);
    let high_voice = render_note_with_voice(48, 1.0);
    let difference: f32 = low_voice
        .iter()
        .zip(high_voice)
        .map(|(a, b)| (a - b).abs())
        .sum();
    assert!(
        difference > 0.01,
        "voice/head scaling must affect rendered formants"
    );
}

#[test]
fn local_pad_lifecycle_is_idempotent_and_one_release_stops_voice() {
    let mut engine = SynthEngine::default();
    let down = local_pad_down(1.0);
    let up = local_pad_up();
    engine.process(&mut [], 0, &[down, down]);
    assert!(engine.voice_state().gate);
    engine.process(&mut [], 0, &[up]);
    assert!(!engine.voice_state().gate);
    assert!(!engine.pad_state().active);
}

#[test]
fn reset_and_local_pad_release_have_no_default_or_stale_voice() {
    let mut engine = prepared_dry_engine(25_000);
    let mut left = vec![1.0; 25_000];
    let mut right = vec![1.0; 25_000];
    engine.process(&mut [&mut left, &mut right], 25_000, &[]);
    assert!(
        left.iter().all(|sample| sample.abs() <= f32::EPSILON),
        "initialization must not synthesize a default note"
    );

    let down = local_pad_down(1.0);
    engine.process(&mut [&mut left, &mut right], 1, &[down]);
    release_local_pad_and_assert_silence(&mut engine, &mut left, &mut right);
}

#[test]
fn external_midi_owns_voice_and_pad_cannot_add_note_28_phantom() {
    let mut engine = SynthEngine::default();
    let local_down = local_pad_down(1.0);
    let external_down = SynthesisEvent {
        kind: SynthesisEventKind::NoteOn,
        note: 60,
        value: 1.0,
        local_pad: false,
        ..SynthesisEvent::default()
    };
    let external_up = SynthesisEvent {
        kind: SynthesisEventKind::NoteOff,
        note: 60,
        local_pad: false,
        ..SynthesisEvent::default()
    };
    engine.process(&mut [], 0, &[local_down]);
    assert_eq!(engine.voice_state().current_note, PAD_INTERNAL_NOTE);
    engine.process(&mut [], 0, &[external_down, local_down]);
    assert_eq!(engine.voice_state().current_note, 60);
    engine.process(&mut [], 0, &[external_up]);
    assert!(
        !engine.voice_state().gate,
        "local note 28 must not remain beneath or return after external MIDI"
    );
}

fn render_mouse_pad_drag(x: f32) -> Vec<f32> {
    let mut engine = SynthEngine::default();
    engine.prepare(44_100.0, 4_096, 2);
    engine.set_parameters(Parameters {
        delay_mix: 0.0,
        volume: 0.5,
        ..Parameters::default()
    });
    let events = [
        local_pad_down(64.0 / 127.0),
        SynthesisEvent {
            kind: SynthesisEventKind::PadPitch,
            note: -1,
            value: x,
            local_pad: true,
            ..SynthesisEvent::default()
        },
        SynthesisEvent {
            kind: SynthesisEventKind::PadVowel,
            note: -1,
            value: 0.5,
            local_pad: true,
            ..SynthesisEvent::default()
        },
    ];
    let mut left = vec![0.0; 4_096];
    let mut right = vec![0.0; 4_096];
    engine.process(&mut [&mut left, &mut right], 4_096, &events);
    assert_eq!(engine.voice_state().current_note, PAD_INTERNAL_NOTE);
    assert!(engine.voice_state().gate);
    left
}

#[test]
fn mouse_pad_pitch_routes_the_single_voice_instead_of_leaving_a_fixed_low_carrier() {
    let low = render_mouse_pad_drag(0.0);
    let high = render_mouse_pad_drag(1.0);
    assert_ne!(
        low, high,
        "horizontal pad routing must retune the one active voice"
    );
}

#[test]
fn mouse_pad_down_drag_up_has_one_lifecycle_and_reaches_silence() {
    let mut engine = prepared_dry_engine(25_000);
    let down = local_pad_down(64.0 / 127.0);
    let drag = SynthesisEvent {
        kind: SynthesisEventKind::PadPitch,
        note: -1,
        value: 0.8,
        local_pad: true,
        ..SynthesisEvent::default()
    };
    let mut left = vec![0.0; 25_000];
    let mut right = vec![0.0; 25_000];
    engine.process(&mut [&mut left, &mut right], 1, &[down, drag]);
    assert!(engine.voice_state().gate);
    release_local_pad_and_assert_silence(&mut engine, &mut left, &mut right);
}

#[test]
fn releasing_last_note_discards_queued_dry_grains() {
    let mut engine = SynthEngine::default();
    engine.prepare(44_100.0, 64, 2);
    engine.set_parameters(Parameters {
        delay_mix: 0.0,
        volume: 0.5,
        ..Parameters::default()
    });
    let down = local_pad_down(1.0);
    let up = local_pad_up();
    let mut left = [0.0; 64];
    let mut right = [0.0; 64];
    engine.process(&mut [&mut left, &mut right], 64, &[down]);
    assert!(left.iter().any(|sample| sample.abs() > 0.0));

    left.fill(1.0);
    right.fill(1.0);
    engine.process(&mut [&mut left, &mut right], 64, &[up]);

    assert!(!engine.voice_state().gate);
    assert!(left.iter().all(|sample| sample.abs() <= f32::EPSILON));
    assert!(right.iter().all(|sample| sample.abs() <= f32::EPSILON));
}
