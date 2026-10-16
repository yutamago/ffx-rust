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
//! - [`x87`]: what the game's x87 code does to the floats it moves.

pub mod chr;
pub mod handle;
pub mod layout;
pub mod x87;
