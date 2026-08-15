use delaylama_core::VisualState;
use delaylama_editor::{EditorModel, PadGesture};

#[test]
fn default_and_gesture_contract() {
    let mut m = EditorModel::default();
    assert_eq!(m.animation_frame(), 5);
    let r = m.handle_gesture(-1.0, 2.0, PadGesture::Down);
    assert_eq!((r.x, r.y, r.vowel), (0.0, 1.0, 0.0));
    assert!(r.note_on && !r.note_off);
    assert_eq!(r.note_on_note, 40);
    assert_eq!(r.note_off_note, -1);
    assert_eq!(m.animation_frame(), 6);
    let r = m.handle_gesture(0.05, 0.05, PadGesture::Up);
    assert!(r.note_off && !r.note_on);
    assert_eq!(r.note_on_note, -1);
    assert_eq!(r.note_off_note, 40);
    assert_eq!(m.animation_frame(), 5);
}

#[test]
fn visual_state_contract() {
    let mut m = EditorModel::default();
    m.apply_external_state(VisualState {
        note: 40,
        gate: true,
        vowel: 1.04,
        atlas_selector: 0.0,
    });
    assert_eq!(m.animation_frame(), 0);
    m.apply_external_state(VisualState {
        note: 3,
        gate: true,
        vowel: 1.0,
        atlas_selector: 0.0,
    });
    assert_eq!(m.animation_frame(), 5);
}

#[test]
fn frame_selection_boundaries() {
    assert_eq!(EditorModel::select_visual_state(4, true, 0.0), 6);
    assert_eq!(EditorModel::select_visual_state(72, true, 1.0), 29);
    assert_eq!(EditorModel::select_visual_state(73, true, 0.05), 5);
}

#[test]
fn embeds_original_artwork() {
    assert!(delaylama_editor::Artwork::ORIGINAL.is_complete());
}

#[test]
fn source_coordinates_match_cpp_editor_contract() {
    use delaylama_editor::{SourceRect, ViewTransform};
    assert_eq!(
        SourceRect::PAD,
        SourceRect {
            x: 96.0,
            y: 362.0,
            width: 166.0,
            height: 84.0
        }
    );
    assert_eq!(
        SourceRect::PORTAMENTO,
        SourceRect {
            x: 21.0,
            y: 448.0,
            width: 50.0,
            height: 50.0
        }
    );
    assert_eq!(
        SourceRect::DELAY,
        SourceRect {
            x: 104.0,
            y: 479.0,
            width: 152.0,
            height: 25.0
        }
    );
    assert_eq!(
        SourceRect::VOICE,
        SourceRect {
            x: 293.0,
            y: 447.0,
            width: 50.0,
            height: 50.0
        }
    );
    assert_eq!(
        SourceRect::HELP,
        SourceRect {
            x: 284.0,
            y: 300.0,
            width: 43.0,
            height: 35.0
        }
    );
    let transform = ViewTransform::fit(720.0, 1200.0);
    let view = transform.source_to_view((96.0, 362.0));
    let source = transform.view_to_source(view);
    assert!((source.0 - 96.0).abs() < 0.001);
    assert!((source.1 - 362.0).abs() < 0.001);
}

#[test]
fn hit_testing_uses_inverse_source_transform() {
    use delaylama_editor::{SourceRect, ViewTransform};
    let transform = ViewTransform::fit(1000.0, 510.0);
    let source = transform.view_to_source(transform.source_to_view((179.0, 404.0)));
    assert!(SourceRect::PAD.contains(source));
    assert!(!SourceRect::VOICE.contains(source));
}

#[test]
fn model_owns_hit_testing_and_parameter_edits() {
    use delaylama_editor::{EditorModel, HitTarget, SourceRect};
    assert_eq!(EditorModel::hit_test((96.0, 362.0)), Some(HitTarget::Pad));
    assert_eq!(EditorModel::pad_position((179.0, 404.0)), (0.5, 0.5));
    assert_eq!(
        EditorModel::hit_test((180.0, 487.0)),
        Some(HitTarget::Delay)
    );
    assert!((EditorModel::linear_value(180.0, SourceRect::DELAY) - 0.5).abs() < 0.001);
    assert!((EditorModel::rotary_value(0.5, (25.0, 0.0)) - 0.6).abs() < 0.001);
}

#[test]
fn rotary_drag_sensitivity_uses_logical_pixels_like_juce() {
    let value = EditorModel::rotary_value(0.5, (0.0, -25.0));
    assert!((value - 0.6).abs() < 0.001);
}

#[test]
fn rotary_hit_regions_match_visible_controls_and_drag_axes_match_contract() {
    use delaylama_editor::{EditorModel, HitTarget};

    assert_eq!(
        EditorModel::hit_test((46.0, 473.0)),
        Some(HitTarget::Portamento)
    );
    assert_eq!(EditorModel::hit_test((21.0, 448.0)), None);
    assert_eq!(EditorModel::hit_test((70.9, 497.9)), None);
    assert_eq!(EditorModel::hit_test((20.9, 448.0)), None);
    assert_eq!(
        EditorModel::hit_test((318.0, 472.0)),
        Some(HitTarget::Voice)
    );
    assert_eq!(EditorModel::hit_test((293.0, 447.0)), None);
    assert_eq!(EditorModel::hit_test((342.9, 496.9)), None);
    assert_eq!(EditorModel::hit_test((343.1, 447.0)), None);

    let horizontal = EditorModel::rotary_value(0.5, (25.0, 0.0));
    let vertical_up = EditorModel::rotary_value(0.5, (0.0, -25.0));
    let vertical_down = EditorModel::rotary_value(0.5, (0.0, 25.0));
    assert!((horizontal - 0.6).abs() < 0.001);
    assert!((vertical_up - horizontal).abs() < 0.001);
    assert!((vertical_down - 0.4).abs() < 0.001);
}
