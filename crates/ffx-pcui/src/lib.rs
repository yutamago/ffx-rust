//! The PC user interface: title, config, save list, the Esc menu (`Menu*`, `FFEscMenu`, Iggy).
//!
//! The `pc-ui` module of `docs/rewrite-plan.md`: each original function
//! becomes a core (portable, on its own types) and a `bridge` function
//! over game memory (`docs/porting.md`), one module per family
//! (`docs/formats/pc-ui.md`):
//!
//! - [`iggy`]: Iggy data values and the menus' handler slots;
//! - [`camera`]: the camera tracks of the movies over 3D scenes;
//! - [`input`]: the per-frame input pass;
//! - [`abmap`]: the sphere grid screen (`g_Abmap`) helpers the PC port
//!   placed beside its UI code;
//! - [`dyngeo`]: two shader setters of `ClassDynamicGeometry`;
//! - [`cursor`]: the sphere grid cursor's move and node searches;
//! - [`link`]: the sphere grid's link outlines (straight and curved);
//! - [`mapname`]: the map name overlay;
//! - [`menus`]: the Iggy menu manager's viewport, the Esc menu's handler
//!   and pop-ups;
//! - [`panel`]: the sphere grid's list panels (cursor, scrolling, choice);
//! - [`item`]: item menu rows and key strings the PC port placed here;
//! - [`sea`]: the sea's UV scroll speed nudge;
//! - [`spdctrl`]: where the speed-up booster stays off;
//! - [`touch`]: the pointer (mouse and touch) zones of the PC screens;
//! - [`video`]: the movie player's Vorbis audio to 16-bit samples;
//! - [`worldmap`]: the menu's world map window.
//! - [`m5u_a`]: M5 stage 1 batches (the controller queues, the menu
//!   texture unloads, the Iggy manager teardown, debug GUI parsers).

pub mod abmap;
pub mod camera;
pub mod cursor;
pub mod dyngeo;
pub mod iggy;
pub mod input;
pub mod item;
pub mod link;
pub mod m5u_a;
pub mod m5u_b;
pub mod m5u_c;
pub mod m5u_d;
pub mod m5u_e;
pub mod m5u_f;
pub mod m5u_g;
pub mod m5u_h;
pub mod m5u_i;
pub mod m5u_j;
pub mod m5u_k;
pub mod m5u_l;
pub mod mapname;
pub mod menus;
pub mod panel;
pub mod sea;
pub mod spdctrl;
pub mod touch;
pub mod video;
pub mod worldmap;
