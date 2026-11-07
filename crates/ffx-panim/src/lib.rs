//! `ffx-panim`: `PhyreEngine`'s `PAnimation` code as FFX.exe reaches it,
//! ported function by function (M4 port round 1, list `p2-anim`,
//! `~/ffx-work/m4/port/plan.md`) with the M3 recipe (`docs/porting.md`).
//!
//! - [`descriptor`]: the class-descriptor callbacks the type system calls
//!   through the `PClassDescriptor*` vtables (construct and destruct an
//!   instance, fix objects after loading) of the animation classes and
//!   their `PArray`s, and the classes' registrations;
//! - [`array`]: `PArray` setters and the dynamic array members'
//!   forwarding callbacks;
//! - [`controller`]: the animation controllers' constructors and
//!   destructors;
//! - [`network`]: `PAnimationNetworkInstance`'s parts;
//! - [`p2_bind`]: the classes' `Bind()` tables (run by h1's and p3's
//!   interpreters);
//! - [`p2_declare`]: the `PArray<T>` / `PSharray<T>` descriptors'
//!   declarations.
//!
//! Everything this list does not hold (`Phyre::PhyreFree`, the type
//! registry, other classes' methods) is called at its address through
//! [`ffx_graphic::host::Host`].

pub mod array;
pub mod controller;
pub mod descriptor;
pub mod l9_anim;
pub mod m5p_anim_arrays;
pub mod m5p_anim_left_a;
pub mod m5p_anim_left_b;
pub mod m5p_anim_left_c;
pub mod m5p_anim_left_d;
pub mod m5p_anim_left_e;
pub mod m5p_anim_left_f;
pub mod m5p_anim_left_g;
pub mod m5p_anim_left_h;
pub mod m5p_anim_left_i;
pub mod m5p_anim_left_j;
pub mod m5p_anim_left_k;
pub mod m5p_anim_left_l;
pub mod m5p_anim_left_m;
pub mod m5p_anim_misc;
pub mod m5p_anim_small;
pub mod network;
pub mod p2_bind;
pub mod p2_declare;
pub mod p2_more;

/// `mov al, 1` over an `eax` that held `eax`: what the callbacks that end
/// that way leave for their caller.
#[must_use]
pub const fn al_one(eax: u32) -> u32 {
    eax & !0xff | 1
}
