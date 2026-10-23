//! Characters: models, motions, attachments (the PS2 `Ch` module).
//!
//! The `chara` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a `bridge` function
//! over game memory (`docs/porting.md`).
//!
//! - [`layout`]: the character object and its model block, as far as the
//!   ported functions show them (`docs/formats/chara.md`).
//! - [`handle`]: finding characters (`Ch_CheckHandle`, `Ch_HandleGetNext`,
//!   `Ch_BlkGetHandleFast`, ...).
//! - [`chr`]: the accessors (`Ch_Get*`, `Ch_Set*`, `Ch_En*`).
//! - [`parts`]: cloth, binding, elements, shade, lights, mouth and neck,
//!   optional positions, motion state.
//! - [`texture`]: textures and cluts.
//! - [`id`]: the map id word.
//! - [`sys`]: the module's own settings (drawing, shadow, matrices,
//!   effect packets).
//! - [`attach`]: attachment points, binding to them, optional positions.
//! - [`cloth`]: the cloth exceptions and resetting cloth.
//! - [`draw`]: skinning the models and their world matrices for drawing.
//! - [`draw_main`]: the frame's character drawing (`Ch_DrawMain`).
//! - [`mapping`]: fixing up moved character data, filling model blocks.
//! - [`motion`]: motion lerps, the motion frame loop, the sequence table.
//! - [`matrix`]: the root and model matrices.
//! - [`texanim`]: texture animation and texture register fix-ups.
//! - [`msvc`]: the C runtime's `sprintf` output for the formats used.
//! - [`fixed`]: fixed-point and 12-bit angle helpers.
//! - [`x87`]: what the game's x87 code does to the floats it computes.

// pointers, counts and offsets named as short as the original's registers
#![allow(clippy::many_single_char_names)]

pub mod attach;
pub mod chr;
pub mod cloth;
pub mod draw;
pub mod draw_main;
pub mod fixed;
pub mod handle;
pub mod id;
pub mod layout;
pub mod mapping;
pub mod matrix;
pub mod motion;
pub mod msvc;
pub mod parts;
pub mod sys;
pub mod texanim;
pub mod texture;
pub mod x87;
