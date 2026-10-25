//! The field scene: what `PhyreEngine` draws and from which data, and the
//! Rust scene builder for the map world.
//!
//! Round 1 of M4 (`docs/formats/scene.md`): a census of one captured
//! frame's scene draws ([`census`]) and the parity of the static objects
//! they use with what the archive's clusters make ([`assets`]), matched by
//! content key ([`key`]).
//!
//! Round 2 (`docs/scene-builder.md`): [`build::build`] makes the map
//! world's draws from the clusters ([`map`], [`effect`], read through
//! [`phyre`]) and the frame's [`frame::SceneInputs`] (from a capture for
//! now: [`capture_inputs`]), culled ([`cull`]) and sorted ([`sort`]) as
//! the original does; [`emit`] turns them into a capture for
//! `ffx-drawdiff`.

pub mod assets;
pub mod build;
pub mod capture;
pub mod capture_inputs;
pub mod census;
pub mod chars;
pub mod chars_check;
pub mod cull;
pub mod effect;
pub mod emit;
pub mod frame;
pub mod inputs;
pub mod key;
pub mod map;
pub mod phyre;
pub mod sort;
pub mod upload;
