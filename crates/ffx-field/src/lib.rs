//! Field maps: backgrounds, walk meshes, the guide map and map effects (the PS2 `rc*`, `Yn`, `Gu`, `Ke`, `Sqs`, `Nkz`, `yi` modules).
//!
//! The `map` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a `bridge` function
//! over game memory (`docs/porting.md`). The layouts they share are in
//! `docs/formats/field.md`.
//!
//! - [`rcdt`]: data chunks and the text parser.
//! - [`packet`]: PS2 DMA packets (`rcPacketData`).
//! - [`abmap`]: sphere grid helpers (`Abmap`).
//! - [`geom`]: bounding boxes, vectors, matrices (`rcBBox`, `rcVec3f`,
//!   `rcMat4f`).
//! - [`model`]: scene objects (`rcbgModel` and its primitive packets,
//!   layers, transfers, palettes, animations, particle simulations).
//! - [`guide`]: the guide map, the minimap (`rcbgGuide`, `Guide_*`, `Yn_*`).
//! - [`scene`]: the map's scene, its layers (map parts) and objects.
//! - [`task`]: the field effects' tasks and objects (`rcef`).
//! - [`rcui`]: the field UI helpers.
//! - [`ke`]: `Ke` resource heaps and lens flare objects.
//! - [`yi`]: the field renderer's state and display objects.
//! - [`vu0`]: the VU0-style vector helpers (`KeVu0*`).
//! - [`nkz`]: overdrive tables (`Nkz`) and sound status (`Sqs`).
//! - [`camera`]: the map camera's projection and view matrices.
//! - [`freecam`]: the free (debug) camera (`FreeCamera`).
//! - [`x87`]: what the original's x87 code does to floats.

#![allow(clippy::many_single_char_names)] // registers, pointers and vector parts, as the originals

pub mod abmap;
pub mod abmap_draw;
pub mod camera;
pub mod freecam;
pub mod geom;
pub mod guide;
pub mod ke;
pub mod model;
pub mod nkz;
pub mod packet;
pub mod rcdt;
pub mod rcui;
pub mod scene;
pub mod screff;
pub mod task;
pub mod vu0;
pub mod x87;
pub mod yi;
