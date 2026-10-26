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
//!   helpers, dlmalloc's `create_mspace_with_base`, menu textures.
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
pub mod menu;
pub mod misc;
pub mod movie;
pub mod r3;
pub mod sound;
