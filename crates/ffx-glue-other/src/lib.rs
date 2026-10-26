//! The remaster's glue outside rendering, ported function by function from
//! FFX.exe (M4 port round 1, work list `h4-glue-other`): the PS2 sound
//! API's FMOD entry points that `ffx_sound` had not reached, the
//! `SyncManager` calls, the post-effect and movie singletons, and the
//! small menu, Iggy and text helpers.
//!
//! - [`sound`]: `iSfx*`, `iVoice*`, `iSync*`, FMOD's memory/file/event
//!   callbacks, `SfxEvent` and `FmodMusic`/`FmodVoice` helpers;
//! - [`effect`]: the post-effect singletons and their small methods,
//!   shader-parameter copies, the character cache's release timer;
//! - [`movie`]: `PhyFMVPlayerManager` and the PS2 movie layer's tests;
//! - [`menu`]: the escape and save menus, `IggyMenu`/`IggyMenuManager`,
//!   `IconPanel`, `TextUtil`, scene cursors, the Iggy heap;
//! - [`misc`]: forwarders, PS2 packet/DMA/GS helpers;
//! - [`copies`]: exact copies of other modules' ports (`tools/dupes.py`);
//! - [`r3`]: round 3: the wide cluster/font calls, singletons, array
//!   helpers, dlmalloc's `create_mspace_with_base`, menu textures;
//! - [`h4_c1`]: the continuation round: the accumulation buffer's GS
//!   quads, menu lookups, FMOD event collectors, texture-animation
//!   records, the escape menu's string arrays;
//! - [`h4_c2`]: its second batch: the cluster manager's releases, the FMOD
//!   bank destructors and walks, the id-array tree inserts;
//! - [`h4_c3`]: its third batch: `FmodVoice` and `FmodMusic` teardown, the
//!   sync manager's change, the event groups, Iggy callbacks, `rfind`;
//! - [`h4_c4`]: its fourth batch: the extended blur's render, the voice's
//!   event check, a six-value Flash call, menu texture and vertex helpers;
//! - [`h4_d1`]: continuation round 2: the particle fill-buffer jobs, the
//!   Iggy manager's render and menu insert, `GDraw`'s render-thread setup,
//!   files, the menu textures' kind dispatch, the distortion target,
//!   `SfxEvent_isPlaying`, the Atel shape's VFX;
//! - [`h4_d2`]: its second batch: the escape menu's callbacks and
//!   language call, the Iggy callback table, the button-remap tables, the
//!   menu buffers and UI-instance unload, the touch reset, the shader
//!   texture rebinding walks;
//! - [`h4_d3`]: its third batch: the menu font, Sa-menu, monster-book and
//!   language texture loaders, the distortion grid's indices, the loading
//!   screen's textures, the FMOD command queue's push and synchronous
//!   command, two constructors, the button icon cells;
//! - [`h4_d4`]: its fourth batch: an eight-value Flash call, a texture
//!   animation frame by name, the soft particles' specular maps, and the
//!   FMOD data loaders (sync buffers, voice lists and projects, music
//!   projects, the shout stream);
//! - [`h4_d5`]: its fifth batch: the message windows' draw and an Iggy
//!   menu's mouse handler;
//! - [`h4_e1`]: continuation round 3: the menus' x87 cursors, brushes and
//!   backgrounds, the weapon ability lists, the battle target cursors and
//!   the enemies' limit gauges;
//! - [`h4_e2`]: its second batch: the FMOD volume code (the music's direct
//!   and whole volume, its fades, the voice's loop fade, the sound
//!   events' pan and stop, the sync manager's data read);
//! - [`h4_e3`]: its third batch: the FMOD music's stop, stop-all, restart
//!   and per-frame fade, the sound events' start and fades;
//! - [`h4_e4`]: its fourth batch: UI records for `graphicDrawUIElement`
//!   (the active character's cursor, a sequence rectangle, a mode-2
//!   gradient box), the distortion grid's weights, the button icons' UV,
//!   size and position remaps;
//! - [`h4_e5`]: its fifth batch: the FMOD music's start, read, cross-play,
//!   data send and background-music switch;
//! - [`h4_f1`]: continuation round 4, batch 1: the FMOD voice's start and
//!   restart, the sound loop, the sound effects' whole volume and project
//!   unload, a sound event's volume fade, the menu manager's instance
//!   setups and buffer chain swap;
//! - [`h4_f2`]: its second batch: the render thread's shader-pass switch,
//!   the menu manager's material update and buffer reset, the button-icon
//!   vertex info lookup, a sound event's constructor and release.
//!
//! Everything these call that is not ported here (the FMOD queue, the
//! managers' big methods, `PhyreEngine`, imports) is called at its address
//! through [`ffx_graphic::host::Host`], as other ports call original code.
//! Every bridge returns what the original leaves in `eax` where a caller
//! can read it.

// 32-bit game addresses and words.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

pub mod copies;
pub mod effect;
pub mod frames;
pub mod h4_c1;
pub mod h4_c2;
pub mod h4_c3;
pub mod h4_c4;
pub mod h4_d1;
pub mod h4_d2;
pub mod h4_d3;
pub mod h4_d4;
pub mod h4_d5;
pub mod h4_e1;
pub mod h4_e2;
pub mod h4_e3;
pub mod h4_e4;
pub mod h4_e5;
pub mod h4_f1;
pub mod h4_f2;
pub mod menu;
pub mod misc;
pub mod movie;
pub mod r3;
pub mod sound;
