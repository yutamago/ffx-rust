//! Post-processing census (M4, `docs/formats/postfx.md`): every draw a
//! captured frame makes outside the scene targets, sorted into the
//! post-process passes `PhyreScene_render` (`0x00668930`) runs after the
//! scene (glow, distortion, brightness, the composite to the back buffer)
//! and the effects and UI draws between them, with the source of every
//! constant the post-process shaders read checked against the data.
//!
//! - [`shaders`]: shader families by bytecode key, from the archive's
//!   shader clusters.
//! - [`camera`]: the map's `GlobalCamera`, where the glow and the clear
//!   colour come from.
//! - [`census`]: the classification, the order check and the constant
//!   checks.

pub mod camera;
pub mod census;
pub mod shaders;
