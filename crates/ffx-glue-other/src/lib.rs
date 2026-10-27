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
//!   vertex info lookup, a sound event's constructor and release;
//! - [`h4_f3`]: its third batch: the sound effects' per-frame update,
//!   event creation, battle stream, project load and constructor;
//! - [`h4_f4`]: its fourth batch: the FMOD event system's setup, a sound
//!   bank's constructor and its event tree's load;
//! - [`h4_f5`]: its fifth batch: the FMOD voice's event read and the sync
//!   manager's setup;
//! - [`h4_f6`]: its sixth batch: the menu message file read;
//! - [`h4_f7`]: its seventh batch: the sound event group's list load;
//! - [`h4_g1`]: continuation round 5, batch 1: the battle and menu UI's
//!   small x87 drawers (damage digits, battle icons, the limit plates and
//!   gauge, the ratio number, the simple help, the wait window's cursor),
//!   three UI records and `movie_disp2`;
//! - [`h4_g2`]: its second batch: the distortion grid's positions and
//!   UVs, the extended blur's, `TextUtil`'s and the tweak tool's
//!   constructors, an Iggy menu's viewport, a `GDraw` rectangle's UVs;
//! - [`h4_i1`]: continuation round 7, batch 1: the Win32 video playback's
//!   close, the VRAM resource save, the debug file read, the window setup,
//!   the VFX line buffer, a particle model's vertex copy, `GDraw`'s pool
//!   handle and free-all;
//! - [`h4_i2`]: its second batch: the movie player's play and render, the
//!   draw environment, the split image load, `IggyMenu`'s constructor, the
//!   semaphore queue's start, an escape-menu dialog callback;
//! - [`h4_i3`]: its third batch: the blur effect's constructor, the motion
//!   blur's render, the save/load menu's open, the boot text's update,
//!   `GDraw`'s resource limits and setup;
//! - [`h4_i4`]: its fourth batch: the PS2 GS layer's frame start, packet
//!   transfer and texture loads;
//! - [`h4_i5`]: its fifth batch: a particle model's projected positions,
//!   `GDraw`'s depth-stencil state cache and depth buffer, the distortion
//!   grid's setup;
//! - [`h4_i6`]: its sixth batch: the broken screen effect's frame capture
//!   and the movie player's texture update;
//! - [`h4_i7`]: its seventh batch: the cluster manager's preallocated data
//!   list;
//! - [`h4_i8`]: its eighth batch: the distortion effect's render;
//! - [`h4_j1`]: continuation round 8, batch 1: the PS2 GS layer's first
//!   effect packet, the settings pages' key-name tables, a UI button's
//!   remap, the character mesh instances' name suffixes;
//! - [`h4_j2`]: its second batch: the escape menu's controller and
//!   keyboard binding pages, the audio and parameter-change pages;
//! - [`h4_j3`]: its third batch: the menu texture clusters' load, the
//!   message button remap, the special-feature page;
//! - [`h4_j4`]: its fourth batch: the escape menu's main setting page;
//! - [`h4_j5`]: its fifth batch: the text utility's asset hook-up and the
//!   character shader's render data;
//! - [`h4_j6`]: its sixth batch: the guild map's dynamic geometry update;
//! - [`h4_k1`]: continuation round 9, batch 1: the VFX line render data,
//!   the blur effect's render and update, the extended blur's pass;
//! - [`h4_k2`]: its second batch: the text utility's line append, a
//!   button icon's position remap, the loading screen's update;
//! - [`h4_k3`]: its third batch: the depth-bias hook, the post-process
//!   manager's constructor, the broken-screen effect's render;
//! - [`h4_k4`]: its fourth batch: the event shapes' VFX render data
//!   buffers;
//! - [`h4_l1`]: continuation round 10, batch 1: the shapes' VFX render
//!   data buffers, the menu vertex records, the character cache's delayed
//!   release, the voice data change;
//! - [`h4_l2`]: its second batch: the guild map's VFX render data;
//! - [`h4_l3`], [`h4_l4`], [`h4_l5`]: its third batch: the movie player
//!   manager's video index map, a menu list item's shape sprite, the
//!   escape menu's update.
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

pub mod c2_iggy;
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
pub mod h4_f3;
pub mod h4_f4;
pub mod h4_f5;
pub mod h4_f6;
pub mod h4_f7;
pub mod h4_g1;
pub mod h4_g2;
pub mod h4_h1;
pub mod h4_h2;
pub mod h4_h3;
pub mod h4_h4;
pub mod h4_h5;
pub mod h4_h6;
pub mod h4_h7;
pub mod h4_h8;
pub mod h4_i1;
pub mod h4_i2;
pub mod h4_i3;
pub mod h4_i4;
pub mod h4_i5;
pub mod h4_i6;
pub mod h4_i7;
pub mod h4_i8;
pub mod h4_j1;
pub mod h4_j2;
pub mod h4_j3;
pub mod h4_j4;
pub mod h4_j5;
pub mod h4_j6;
pub mod h4_k1;
pub mod h4_k2;
pub mod h4_k3;
pub mod h4_k4;
pub mod h4_l1;
pub mod h4_l2;
pub mod h4_l3;
pub mod h4_l4;
pub mod h4_l5;
pub mod h4_m1;
pub mod h4_m2;
pub mod h4_m3;
pub mod h4_m4;
pub mod h4_m5;
pub mod h4_m6;
pub mod h4_m7;
pub mod h4_m8;
pub mod h4_m9;
pub mod h4b_a;
pub mod h4b_b;
pub mod h4b_c;
pub mod h4b_d;
pub mod h4b_e;
pub mod h4b_f;
pub mod h4b_g;
pub mod h4b_h;
pub mod h4b_i;
pub mod h4b_j;
pub mod h4b_k;
pub mod h4b_l;
pub mod h4b_m;
pub mod h4b_n;
pub mod h4y_dtt;
pub mod h4y_shp;
pub mod menu;
pub mod misc;
pub mod movie;
pub mod r3;
pub mod sound;
