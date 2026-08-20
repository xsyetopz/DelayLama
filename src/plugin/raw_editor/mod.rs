mod animation;
mod artwork;
mod draws;
mod geometry;
mod interaction;
mod lifecycle;
mod renderer;

pub(super) use animation::animation_frame;
pub(super) use lifecycle::RawEditor;

#[cfg(test)]
pub(super) use interaction::{PointerPhase, pad_gesture};

pub(super) const SIZE: (u32, u32) = geometry::SOURCE_SIZE;

#[cfg(test)]
mod tests;
