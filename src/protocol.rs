//! Numeric identifiers and conversions for host events.

/// Largest MIDI data value.
pub const SEVEN_BIT_MAX: i32 = 127;
/// Largest MIDI pitch-bend value.
pub const FOURTEEN_BIT_MAX: i32 = 16_383;
/// Lowest host note accepted by the adapter.
pub const HOST_NOTE_MINIMUM: i32 = 16;
/// Highest host note accepted by the adapter.
pub const HOST_NOTE_MAXIMUM: i32 = 84;
/// Offset from host notes to internal notes.
pub const HOST_NOTE_OFFSET: i32 = 12;
/// Lowest internal note accepted by the engine.
pub const MINIMUM_NOTE: i32 = HOST_NOTE_MINIMUM - HOST_NOTE_OFFSET;
/// Highest internal note accepted by the engine.
pub const MAXIMUM_NOTE: i32 = HOST_NOTE_MAXIMUM - HOST_NOTE_OFFSET;
/// Internal note used for editor-pad lifecycle events.
pub const PAD_INTERNAL_NOTE: i32 = 28;
/// Host-note representation of the editor pad's fixed voice.
pub const PAD_HOST_NOTE: i32 = PAD_INTERNAL_NOTE + HOST_NOTE_OFFSET;
/// Fixed velocity used when the editor pad starts its voice.
pub const PAD_NOTE_ON_VELOCITY: f32 = 64.0 / 127.0;
/// MIDI controller for vibrato.
pub const CC_VIBRATO: i32 = 1;
/// MIDI controller for portamento time.
pub const CC_PORT_TIME: i32 = 5;
/// MIDI controller for volume.
pub const CC_VOLUME: i32 = 7;
/// MIDI controller for pad pitch routing.
pub const CC_XY_ROUTING: i32 = 11;
/// MIDI controller for delay mix.
pub const CC_DELAY_MIX: i32 = 12;
/// MIDI controller for voice character.
pub const CC_VOICE: i32 = 13;
/// Scale applied to MIDI CC7 volume.
pub const CC7_VOLUME_SCALE: f32 = 0.127;

/// Position on the asset editor's pad, from zero to one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadPosition {
    /// Horizontal pad position.
    pub x: f32,
    /// Vertical pad position.
    pub y: f32,
}

/// Note change produced by a pad gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GestureTransition {
    /// The gesture does not change note ownership.
    None,
    /// The gesture starts the contained logical note.
    NoteOn(i32),
    /// The gesture ends the contained logical note.
    NoteOff(i32),
}

/// Pad values sent from the asset editor to the host layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GestureResult {
    /// Pointer position from zero to one.
    pub position: PadPosition,
    /// Inverted vertical position used for vowel selection.
    pub vowel: f32,
    /// Note lifecycle change caused by this gesture.
    pub transition: GestureTransition,
}

/// Returns whether a host note is within the supported range.
#[must_use]
pub const fn is_host_note(note: i32) -> bool {
    note >= HOST_NOTE_MINIMUM && note <= HOST_NOTE_MAXIMUM
}

/// Converts a host note to the internal note numbering.
#[must_use]
pub const fn to_internal_note(note: i32) -> i32 {
    note - HOST_NOTE_OFFSET
}

/// Converts a host note when it is within the supported range.
#[must_use]
pub const fn internal_note(note: i32) -> Option<i32> {
    if is_host_note(note) {
        Some(to_internal_note(note))
    } else {
        None
    }
}

/// MIDI messages accepted from a host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MidiMessage {
    /// Starts a note with a seven-bit velocity.
    NoteOn {
        /// Host MIDI note number.
        note: i32,
        /// Seven-bit note-on velocity.
        velocity: i32,
    },
    /// Releases a note.
    NoteOff {
        /// Host MIDI note number.
        note: i32,
        /// Seven-bit note-off velocity.
        velocity: i32,
    },
    /// Sets a fourteen-bit pitch-bend value.
    PitchBend {
        /// Fourteen-bit pitch-bend value.
        value: i32,
    },
    /// Sets a seven-bit controller value.
    ControlChange {
        /// MIDI continuous-controller identifier.
        controller: i32,
        /// Seven-bit controller value.
        value: i32,
    },
}

/// Command applied by the synthesizer at a sample position in a block.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SynthesisEvent {
    /// Synthesis operation.
    pub kind: SynthesisEventKind,
    /// Audio-block sample offset.
    pub sample_offset: i32,
    /// Internal note number.
    pub note: i32,
    /// Event value from zero to one.
    pub value: f32,
    /// Controller number for control changes.
    pub controller: i32,
    /// Whether the event originated from the editor pad.
    pub local_pad: bool,
}

/// Operations accepted by the synthesizer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum SynthesisEventKind {
    /// Releases a held note.
    #[default]
    NoteOff,
    /// Starts a note.
    NoteOn,
    /// Changes pitch-bend state.
    PitchBend,
    /// Changes a continuous controller.
    ControlChange,
    /// Changes the editor pad pitch route.
    PadPitch,
    /// Changes the editor pad vowel position.
    PadVowel,
}

impl Default for SynthesisEvent {
    fn default() -> Self {
        Self {
            kind: SynthesisEventKind::NoteOff,
            sample_offset: 0,
            note: 0,
            value: 0.0,
            controller: 0,
            local_pad: false,
        }
    }
}

/// Turns a MIDI message into a synthesis command.
#[must_use]
pub fn to_synthesis_event(message: MidiMessage, sample_offset: i32) -> Option<SynthesisEvent> {
    let base_event = SynthesisEvent {
        kind: SynthesisEventKind::ControlChange,
        sample_offset,
        note: 0,
        value: 0.0,
        controller: 0,
        local_pad: false,
    };
    match message {
        MidiMessage::NoteOn { note, velocity } | MidiMessage::NoteOff { note, velocity } => {
            let note = internal_note(note)?;
            Some(SynthesisEvent {
                kind: if matches!(message, MidiMessage::NoteOn { .. }) {
                    SynthesisEventKind::NoteOn
                } else {
                    SynthesisEventKind::NoteOff
                },
                note,
                value: f32::from(i16::try_from(velocity.clamp(0, SEVEN_BIT_MAX)).unwrap_or(0))
                    / 127.0,
                ..base_event
            })
        }
        MidiMessage::PitchBend { value } => Some(SynthesisEvent {
            kind: SynthesisEventKind::PitchBend,
            value: f32::from(i16::try_from(value.clamp(0, FOURTEEN_BIT_MAX)).unwrap_or(0))
                / 16383.0,
            ..base_event
        }),
        MidiMessage::ControlChange { controller, value } => Some(SynthesisEvent {
            kind: SynthesisEventKind::ControlChange,
            controller,
            value: f32::from(i16::try_from(value.clamp(0, SEVEN_BIT_MAX)).unwrap_or(0)) / 127.0,
            ..base_event
        }),
    }
}
