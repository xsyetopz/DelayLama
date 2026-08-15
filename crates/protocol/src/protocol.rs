pub const SEVEN_BIT_MAX: i32 = 127;
pub const FOURTEEN_BIT_MAX: i32 = 16_383;
pub const HOST_NOTE_MINIMUM: i32 = 16;
pub const HOST_NOTE_MAXIMUM: i32 = 84;
pub const HOST_NOTE_OFFSET: i32 = 12;
pub const MINIMUM_NOTE: i32 = HOST_NOTE_MINIMUM - HOST_NOTE_OFFSET;
pub const MAXIMUM_NOTE: i32 = HOST_NOTE_MAXIMUM - HOST_NOTE_OFFSET;
pub const PAD_INTERNAL_NOTE: i32 = 28;
pub const CC_VIBRATO: i32 = 1;
pub const CC_PORT_TIME: i32 = 5;
pub const CC_VOLUME: i32 = 7;
pub const CC_XY_ROUTING: i32 = 11;
pub const CC_DELAY_MIX: i32 = 12;
pub const CC_VOICE: i32 = 13;
pub const CC7_VOLUME_SCALE: f32 = 0.127;

#[must_use]
pub const fn is_host_note(note: i32) -> bool {
    note >= HOST_NOTE_MINIMUM && note <= HOST_NOTE_MAXIMUM
}

#[must_use]
pub const fn to_internal_note(note: i32) -> i32 {
    note - HOST_NOTE_OFFSET
}

#[must_use]
pub const fn internal_note(note: i32) -> Option<i32> {
    if is_host_note(note) {
        Some(to_internal_note(note))
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MidiMessage {
    NoteOn { note: i32, velocity: i32 },
    NoteOff { note: i32, velocity: i32 },
    PitchBend { value: i32 },
    ControlChange { controller: i32, value: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoreEvent {
    pub kind: CoreEventKind,
    pub sample_offset: i32,
    pub note: i32,
    pub value: f32,
    pub controller: i32,
    pub local_pad: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoreEventKind {
    NoteOn,
    NoteOff,
    PitchBend,
    ControlChange,
}

#[must_use]
pub fn to_core_event(message: MidiMessage, sample_offset: i32) -> Option<CoreEvent> {
    let base = CoreEvent {
        kind: CoreEventKind::ControlChange,
        sample_offset,
        note: 0,
        value: 0.0,
        controller: 0,
        local_pad: false,
    };
    match message {
        MidiMessage::NoteOn { note, velocity } | MidiMessage::NoteOff { note, velocity } => {
            let note = internal_note(note)?;
            Some(CoreEvent {
                kind: if matches!(message, MidiMessage::NoteOn { .. }) {
                    CoreEventKind::NoteOn
                } else {
                    CoreEventKind::NoteOff
                },
                note,
                value: f32::from(i16::try_from(velocity.clamp(0, SEVEN_BIT_MAX)).unwrap_or(0))
                    / 127.0,
                ..base
            })
        }
        MidiMessage::PitchBend { value } => Some(CoreEvent {
            kind: CoreEventKind::PitchBend,
            value: f32::from(i16::try_from(value.clamp(0, FOURTEEN_BIT_MAX)).unwrap_or(0))
                / 16383.0,
            ..base
        }),
        MidiMessage::ControlChange { controller, value } => Some(CoreEvent {
            kind: CoreEventKind::ControlChange,
            controller,
            value: f32::from(i16::try_from(value.clamp(0, SEVEN_BIT_MAX)).unwrap_or(0)) / 127.0,
            ..base
        }),
    }
}
