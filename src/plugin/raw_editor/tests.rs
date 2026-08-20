//! Asset-editor geometry, artwork, gesture, and animation contracts.

use crate::{
    host::HostVisualState,
    protocol::{GestureTransition, MAXIMUM_NOTE, MINIMUM_NOTE, PAD_HOST_NOTE, PAD_INTERNAL_NOTE},
};

use super::{
    animation::animation_frame,
    geometry::{
        HitTarget, SourceRect, ViewTransform, hit_target, linear_value, pad_position, rotary_value,
    },
    interaction::{PointerPhase, pad_gesture},
};

fn assert_rect(actual: SourceRect, expected: SourceRect) {
    assert!((actual.x - expected.x).abs() <= f32::EPSILON);
    assert!((actual.y - expected.y).abs() <= f32::EPSILON);
    assert!((actual.width - expected.width).abs() <= f32::EPSILON);
    assert!((actual.height - expected.height).abs() <= f32::EPSILON);
}

#[test]
fn raw_editor_pad_gesture_contract() {
    let result = pad_gesture(-1.0, 2.0, PointerPhase::Down);
    assert!(result.position.x.abs() <= f32::EPSILON);
    assert!((result.position.y - 1.0).abs() <= f32::EPSILON);
    assert!(result.vowel.abs() <= f32::EPSILON);
    assert_eq!(result.transition, GestureTransition::NoteOn(PAD_HOST_NOTE));

    let result = pad_gesture(0.05, 0.05, PointerPhase::Up);
    assert_eq!(result.transition, GestureTransition::NoteOff(PAD_HOST_NOTE));
}

#[test]
fn visual_state_contract() {
    assert_eq!(
        animation_frame(HostVisualState {
            note: PAD_INTERNAL_NOTE,
            gate: true,
            vowel: 1.04,
            atlas_selector: 0.0,
        }),
        0
    );
    assert_eq!(
        animation_frame(HostVisualState {
            note: MINIMUM_NOTE - 1,
            gate: true,
            vowel: 1.0,
            atlas_selector: 0.0,
        }),
        5
    );
}

#[test]
fn idle_state_uses_atlas_animation_frames() {
    let frame = |atlas_selector| {
        animation_frame(HostVisualState {
            note: MINIMUM_NOTE - 1,
            gate: false,
            vowel: 0.5,
            atlas_selector,
        })
    };
    assert_eq!(frame(0.0), 0);
    assert_eq!(frame(2.0 / 30.0), 2);
    assert_eq!(frame(5.0 / 30.0), 5);
    assert_eq!(frame(1.0), 5);
}

#[test]
fn frame_selection_boundaries() {
    let frame = |note, vowel| {
        animation_frame(HostVisualState {
            note,
            gate: true,
            vowel,
            atlas_selector: -1.0,
        })
    };
    assert_eq!(frame(MINIMUM_NOTE, 0.0), 6);
    assert_eq!(frame(MAXIMUM_NOTE, 1.0), 29);
    assert_eq!(frame(MAXIMUM_NOTE + 1, 0.05), 5);
}

#[test]
fn source_coordinates_match_cpp_editor_contract() {
    assert_rect(
        SourceRect::PAD,
        SourceRect {
            x: 96.0,
            y: 362.0,
            width: 166.0,
            height: 84.0,
        },
    );
    assert_rect(
        SourceRect::PORTAMENTO,
        SourceRect {
            x: 21.0,
            y: 448.0,
            width: 50.0,
            height: 50.0,
        },
    );
    assert_rect(
        SourceRect::DELAY,
        SourceRect {
            x: 104.0,
            y: 479.0,
            width: 152.0,
            height: 25.0,
        },
    );
    assert_rect(
        SourceRect::VOICE,
        SourceRect {
            x: 293.0,
            y: 447.0,
            width: 50.0,
            height: 50.0,
        },
    );
    assert_rect(
        SourceRect::HELP,
        SourceRect {
            x: 284.0,
            y: 300.0,
            width: 43.0,
            height: 35.0,
        },
    );
    let transform = ViewTransform::fit(720.0, 1200.0);
    let view = transform.source_to_view((96.0, 362.0));
    let source = transform.view_to_source(view);
    assert!((source.0 - 96.0).abs() < 0.001);
    assert!((source.1 - 362.0).abs() < 0.001);
}

#[test]
fn hit_testing_uses_inverse_source_transform() {
    let transform = ViewTransform::fit(1000.0, 510.0);
    let source = transform.view_to_source(transform.source_to_view((179.0, 404.0)));
    assert!(SourceRect::PAD.contains(source));
    assert!(!SourceRect::VOICE.contains(source));
}

#[test]
fn asset_editor_geometry_owns_hit_testing_and_parameter_edits() {
    assert_eq!(hit_target((96.0, 362.0)), Some(HitTarget::Pad));
    let pad_position = pad_position((179.0, 404.0));
    assert!((pad_position.0 - 0.5).abs() <= f32::EPSILON);
    assert!((pad_position.1 - 0.5).abs() <= f32::EPSILON);
    assert_eq!(hit_target((180.0, 487.0)), Some(HitTarget::Delay));
    assert!((linear_value(180.0, SourceRect::DELAY) - 0.5).abs() < 0.001);
    assert!((rotary_value(0.5, (25.0, 0.0)) - 0.6).abs() < 0.001);
}

#[test]
fn rotary_drag_sensitivity_uses_logical_pixels_like_juce() {
    let value = rotary_value(0.5, (0.0, -25.0));
    assert!((value - 0.6).abs() < 0.001);
}

#[test]
fn rotary_hit_regions_match_visible_controls_and_drag_axes_match_contract() {
    assert_eq!(hit_target((46.0, 473.0)), Some(HitTarget::Portamento));
    assert_eq!(hit_target((21.0, 448.0)), None);
    assert_eq!(hit_target((70.9, 497.9)), None);
    assert_eq!(hit_target((20.9, 448.0)), None);
    assert_eq!(hit_target((318.0, 472.0)), Some(HitTarget::Voice));
    assert_eq!(hit_target((293.0, 447.0)), None);
    assert_eq!(hit_target((342.9, 496.9)), None);
    assert_eq!(hit_target((343.1, 447.0)), None);

    let horizontal = rotary_value(0.5, (25.0, 0.0));
    let vertical_up = rotary_value(0.5, (0.0, -25.0));
    let vertical_down = rotary_value(0.5, (0.0, 25.0));
    assert!((horizontal - 0.6).abs() < 0.001);
    assert!((vertical_up - horizontal).abs() < 0.001);
    assert!((vertical_down - 0.4).abs() < 0.001);
}
