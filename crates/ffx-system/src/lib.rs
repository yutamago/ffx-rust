//! The system layer: archives, the PS2 file and CD layers, saves, language, settings.
//!
//! The `system` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a `bridge` function
//! over game memory (`docs/porting.md`), one module per area.

pub mod accessor;
pub mod btl;
pub mod cd;
pub mod config;
pub mod container;
pub mod daps;
pub mod dtor;
pub mod files;
pub mod fixup;
pub mod heap;
pub mod instance;
pub mod math;
pub mod menu;
pub mod msg;
pub mod names;
pub mod pad;
pub mod pkt;
pub mod render;
pub mod save;
pub mod scene;
pub mod sort;
pub mod stl;
pub mod stub;
pub mod sync;
pub mod vbf;
pub mod window;
