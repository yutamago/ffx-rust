//! `ffx-panim`: `PhyreEngine`'s `PAnimation` code as FFX.exe reaches it,
//! ported function by function (M4 port round 1, list `p2-anim`,
//! `~/ffx-work/m4/port/plan.md`) with the M3 recipe (`docs/porting.md`).
//!
//! - [`descriptor`]: the class-descriptor callbacks the type system calls
//!   through the `PClassDescriptor*` vtables (construct and destruct an
//!   instance, fix objects after loading) of the animation classes and
//!   their `PArray`s.
//!
//! Everything this list does not hold (`Phyre::PhyreFree`, the type
//! registry, other classes' methods) is called at its address through
//! [`ffx_graphic::host::Host`].

pub mod descriptor;

/// `mov al, 1` over an `eax` that held `eax`: what the callbacks that end
/// that way leave for their caller.
#[must_use]
pub const fn al_one(eax: u32) -> u32 {
    eax & !0xff | 1
}
