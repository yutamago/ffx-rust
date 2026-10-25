//! `PhyreEngine` code the game reaches outside rendering, ported function
//! by function from FFX.exe (M4 port round 1, list `p4-other-b`): the M3
//! pipeline of `docs/porting.md` with the pilot's "Library code" rules.
//!
//! - [`physics`]: the `PPhysics` class descriptors' instance callbacks;
//! - [`object`]: scalar deleting destructors, dynamic-mesh instance
//!   constructors, class registrations.
//!
//! Everything outside the list (the allocator `Phyre_PhyreFree`, the
//! physics objects' own destructors, virtual methods) is called at its
//! address through [`ffx_graphic::host::Host`], in the original's
//! convention.

// 32-bit game addresses and words.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

pub mod methods;
pub mod object;
pub mod physics;
pub mod small;

/// `Phyre_PhyreFree` (cdecl `(ptr)`).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
