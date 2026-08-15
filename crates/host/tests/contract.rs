use delaylama_core::Parameters;
use delaylama_editor::{GestureResult, PadGesture};
use delaylama_host::ProcessorModel;

#[test]
fn parameter_round_trip() {
    let mut p = ProcessorModel::default();
    let v = Parameters {
        vowel: 0.02,
        port_time: 0.03,
        delay_mix: 0.04,
        voice: 0.05,
        vibrato: 0.06,
        volume: 0.07,
        xy_routing: 0.08,
    };
    p.set_parameters(v);
    assert_eq!(p.parameters(), v);
}

#[test]
fn lifecycle_and_processing() {
    let mut p = ProcessorModel::default();
    p.prepare(48000.0, 32);
    let mut left = vec![0.0; 8];
    let mut right = vec![0.0; 8];
    let mut out = vec![left.as_mut_slice(), right.as_mut_slice()];
    p.process(&mut out, 8, &[]);
    p.release();
}

#[test]
fn visual_state_follows_processor_and_pad_ownership() {
    let mut processor = ProcessorModel::default();
    processor.set_parameters(Parameters {
        vowel: 0.25,
        ..Parameters::default()
    });
    let state = processor.visual_state();
    assert_eq!(state.note, -1);
    assert!(!state.gate);
    assert_eq!(state.vowel, 0.25);
    assert_eq!(state.atlas_selector, 0.0);
}

#[test]
fn factory_programs_match_original_contract() {
    assert_eq!(ProcessorModel::factory_programs().len(), 5);
    assert_eq!(ProcessorModel::factory_programs()[0].0, "Rabten");
    let mut processor = ProcessorModel::default();
    assert!(processor.load_factory_program(4));
    assert_eq!(processor.parameters().voice, 1.0);
    assert!(!processor.load_factory_program(5));
}

#[test]
fn state_round_trip_is_versioned_and_deterministic() {
    let mut source = ProcessorModel::default();
    source.set_parameters(Parameters {
        vowel: 0.2,
        volume: 0.7,
        ..Parameters::default()
    });
    let state = source.save_state();
    let mut restored = ProcessorModel::default();
    assert!(restored.load_state(&state));
    assert_eq!(restored.parameters(), source.parameters());
    assert!(!restored.load_state(b"invalid"));
}

#[test]
fn pad_gesture_becomes_local_core_events() {
    let events = ProcessorModel::pad_events(
        GestureResult {
            x: 0.25,
            y: 0.75,
            vowel: 0.25,
            note: 40,
            note_on_note: 40,
            note_off_note: -1,
            note_on: true,
            note_off: false,
        },
        PadGesture::Down,
    );
    assert!(events[0].local_pad);
    assert_eq!(events[0].kind, delaylama_core::EventType::NoteOn);
    assert_eq!(events[1].kind, delaylama_core::EventType::PadPitch);
    assert_eq!(events[2].kind, delaylama_core::EventType::PadVowel);
}

#[test]
fn pad_drag_keeps_single_local_voice_gated() {
    let result = GestureResult {
        x: 0.8,
        y: 0.2,
        vowel: 0.8,
        note: 40,
        note_on_note: -1,
        note_off_note: -1,
        note_on: false,
        note_off: false,
    };
    let events = ProcessorModel::pad_events(result, PadGesture::Drag);
    assert_eq!(events[0].kind, delaylama_core::EventType::NoteOn);
    assert_eq!(events[0].value, 1.0);
    assert!(events[0].local_pad);
}
