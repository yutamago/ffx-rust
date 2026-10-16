//! The system layer: archives, the PS2 file and CD layers, saves, language, settings.
//!
//! The `system` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a `bridge` function
//! over game memory (`docs/porting.md`), one module per area.

pub mod cd;
pub mod config;
pub mod instance;
pub mod math;
pub mod msg;
pub mod save;
pub mod stub;
pub mod x87;
