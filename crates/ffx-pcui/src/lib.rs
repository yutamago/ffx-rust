//! The PC user interface: title, config, save list, the Esc menu (`Menu*`, `FFEscMenu`, Iggy).
//!
//! The `pc-ui` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a `bridge` function
//! over game memory (`docs/porting.md`), one module per family
//! (`docs/formats/pc-ui.md`):
//!
//! - [`iggy`]: Iggy data values and the menus' handler slots;
//! - [`abmap`]: the sphere grid screen (`g_Abmap`) helpers the PC port
//!   placed beside its UI code;
//! - [`cursor`]: the sphere grid cursor's move and node searches;
//! - [`link`]: the sphere grid's link outlines (straight and curved);
//! - [`panel`]: the sphere grid's list panels (cursor, scrolling, choice);
//! - [`item`]: item menu rows and key strings the PC port placed here;
//! - [`sea`]: the sea's UV scroll speed nudge;
//! - [`spdctrl`]: where the speed-up booster stays off;
//! - [`touch`]: the pointer (mouse and touch) zones of the PC screens.

pub mod abmap;
pub mod cursor;
pub mod iggy;
pub mod item;
pub mod link;
pub mod panel;
pub mod sea;
pub mod spdctrl;
pub mod touch;
