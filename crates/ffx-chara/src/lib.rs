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
//! - [`fixed`]: fixed-point and 12-bit angle helpers.
//! - [`x87`]: what the game's x87 code does to the floats it computes.

pub mod chr;
pub mod fixed;
pub mod handle;
pub mod id;
pub mod layout;
pub mod parts;
pub mod sys;
pub mod texture;
pub mod x87;
