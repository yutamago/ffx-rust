//! The remaster's glue outside rendering, ported function by function from
//! FFX.exe (M4 port round 1, work list `h4-glue-other`): the PS2 sound
//! API's FMOD entry points that `ffx_sound` had not reached, the
//! `SyncManager` calls, the post-effect and movie singletons, and the
//! small menu, Iggy and text helpers.
//!
//! - [`sound`]: `iSfx*`, `iVoice*`, `iSync*` and FMOD callbacks;
//! - [`effect`]: the post-effect singletons and their small methods;
//! - [`misc`]: forwarders, menu, Iggy and text helpers.
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

pub mod effect;
pub mod misc;
pub mod sound;
