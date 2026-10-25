//! `PhyreEngine`'s map-world render chain, ported function by function
//! from FFX.exe (the M4 pilot, `~/ffx-work/m4/pilot-report.md`): the
//! path from `PApplication::renderWorld` to the render commands, as the
//! M3 pipeline ports game code (`docs/porting.md`).
//!
//! - [`sort`]: the visibility entries' `std::sort` and its comparator;
//! - [`visibility`]: `PVisibilityCheck` and the culling job;
//! - [`world`]: `renderWorld`, `renderWorldCulledSorted`, `mergeResults`;
//! - [`array`]: `PArray`/`PSharray` storage helpers (h2-render-b);
//! - [`descriptor`]: the type system's descriptor and array-member slots
//!   of the render classes (M4 port round 1, h2-render-b);
//! - [`misc`]: small render-interface, material and shader methods (the
//!   same).
//!
//! Everything the slice does not port (the world cursors, the job system,
//! the ring allocators, the math helpers, `D3D11`) is called at its
//! address through [`ffx_graphic::host::Host`], as other ports call
//! original code.

// 32-bit game addresses and words: indices, offsets and signed words are
// converted where the original reinterprets them; single-letter names are
// the host, entries and pointers, as in the originals.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

pub mod array;
pub mod descriptor;
pub mod glue;
pub mod glue2;
pub mod glue3;
pub mod glue4;
pub mod glue5;
pub mod glue6;
pub mod mesh;
pub mod misc;
pub mod sort;
pub mod thread;
pub mod visibility;
pub mod world;
