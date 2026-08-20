use num_traits::ToPrimitive;

use crate::{
    host::HostVisualState,
    protocol::{MAXIMUM_NOTE, MINIMUM_NOTE},
};

pub(in crate::plugin) fn animation_frame(state: HostVisualState) -> usize {
    let gate = state.gate && (MINIMUM_NOTE..=MAXIMUM_NOTE).contains(&state.note);
    let selector = if !gate {
        5.0 / 30.0
    } else if state.atlas_selector.is_finite() && state.atlas_selector >= 0.0 {
        state.atlas_selector.clamp(0.0, 1.0)
    } else {
        0.8_f32.mul_add(state.vowel.clamp(0.0, 1.0), 0.2)
    };
    selector
        .mul_add(29.0, 0.5)
        .min(29.0)
        .to_usize()
        .unwrap_or(0)
}
