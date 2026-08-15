//! Public protocol-contract tests.

use delaylama_protocol::{CC_DELAY_MIX, MidiMessage, internal_note, to_synthesis_event};

#[test]
fn maps_midi_messages_at_host_boundary() {
    assert_eq!(
        to_synthesis_event(
            MidiMessage::NoteOn {
                note: 60,
                velocity: 127
            },
            3
        )
        .unwrap()
        .note,
        48
    );
    assert!(
        to_synthesis_event(
            MidiMessage::NoteOn {
                note: 10,
                velocity: 127
            },
            0
        )
        .is_none()
    );
    assert_eq!(
        to_synthesis_event(
            MidiMessage::ControlChange {
                controller: CC_DELAY_MIX,
                value: 64
            },
            2
        )
        .unwrap()
        .controller,
        CC_DELAY_MIX
    );
}

#[test]
fn maps_canonical_host_note_range() {
    assert_eq!(internal_note(16), Some(4));
    assert_eq!(internal_note(84), Some(72));
    assert_eq!(internal_note(15), None);
    assert_eq!(internal_note(85), None);
}
