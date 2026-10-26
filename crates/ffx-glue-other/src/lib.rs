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
//!   `SfxEvent_isPlaying`, the Atel shape's VFX.
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
pub mod menu;
pub mod misc;
pub mod movie;
pub mod r3;
pub mod sound;
