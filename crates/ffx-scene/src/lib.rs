//! The field scene: what `PhyreEngine` draws and from which data, for the Rust scene builder.
//!
//! Round 1 of M4 (`docs/formats/scene.md`): a census of one captured
//! frame's scene draws ([`census`]) and the parity of the static objects
//! they use with what the archive's clusters make ([`assets`]), matched by
//! content key ([`key`]).

pub mod assets;
pub mod capture;
pub mod census;
pub mod inputs;
pub mod key;
