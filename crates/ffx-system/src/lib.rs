//! The system layer: archives, the PS2 file and CD layers, saves, language, settings.
//!
//! The `system` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a `bridge` function
//! over game memory (`docs/porting.md`), one module per area.

pub mod accessor;
pub mod btl;
pub mod cd;
pub mod chr_plate;
pub mod cmdwin;
pub mod config;
pub mod container;
pub mod daps;
pub mod dtor;
pub mod files;
pub mod fixup;
pub mod heap;
pub mod instance;
pub mod intl;
pub mod m5a_files;
pub mod m5a_fixup;
pub mod m5a_glue;
pub mod m5a_platform;
pub mod m5a_savedata;
pub mod m5a_savedata2;
pub mod m5a_settings;
pub mod m5a_std;
pub mod m5a_vfs;
pub mod m5b_async;
pub mod m5b_hdd;
pub mod m5b_misc;
pub mod m5b_movie;
pub mod m5b_pc;
pub mod m5b_stackwalker;
pub mod m5b_steam;
pub mod m5b_stl;
pub mod m5b_sync;
pub mod magic;
pub mod math;
pub mod menu;
pub mod msg;
pub mod names;
pub mod pad;
pub mod pkt;
pub mod plate;
pub mod render;
pub mod save;
pub mod scene;
pub mod seq_prim;
pub mod sort;
pub mod stl;
pub mod stub;
pub mod sync;
pub mod tolib;
pub mod vbf;
pub mod window;
