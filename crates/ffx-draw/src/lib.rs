//! The in-process renderer core: pipeline-state layer and a thin backend
//! trait, D3D11 first.
//!
//! - [`desc`]: object descriptions field by field (buffers, textures,
//!   views, input layouts, blend, depth-stencil, rasterizer and sampler
//!   states), with the exact D3D11 bytes for the states.
//! - [`state`]: what a draw has bound ([`DrawState`]) and the call
//!   ([`DrawCall`]).
//! - [`backend`]: the [`Backend`] trait and its handles.
//! - [`cache`]: one backend object per shader (by DXBC), layout, state
//!   description and constant buffer size.
//! - [`pass`]: a draw list for one pass of a world ([`pass::PassDraw`]),
//!   issued between `begin` and `end` (the map path's entry).
//! - [`replay`]: a captured frame (`capture.ffxgfx`) issued through a
//!   backend, and the replayed draws back in the capture's terms.
//! - `d3d11` (Windows): the Direct3D 11 backend on a device and context it
//!   is given (the game's).
//!
//! Design notes: `docs/draw.md`.

pub mod backend;
pub mod cache;
#[cfg(windows)]
pub mod d3d11;
pub mod desc;
pub mod pass;
pub mod replay;
pub mod state;

pub use backend::{Backend, Error, Readback, Result};
pub use state::{DrawCall, DrawState};

#[cfg(test)]
mod tests;
