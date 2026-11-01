//! `PhyreEngine`'s "other" code the game reaches (M4 port round 1, list
//! `p3-other-a`, `~/ffx-work/m4/port/plan.md`), ported function by
//! function from FFX.exe with the M3 recipe (`docs/porting.md`, "Library
//! code"):
//!
//! - [`class`]: the type system's lookups and class registration (the
//!   `Bind` sequences: type dispenser, namespace);
//! - [`object`]: small constructors, destructors and the class
//!   descriptors' create/destroy callbacks (`mov al, 1; ret 4`);
//! - [`physics`]: the physics classes' create callbacks;
//! - [`m5p_scripting_small`]: M5 phyre-scripting batch 1, the small functions;
//! - [`m5p_scripting_mid`]: M5 phyre-scripting batch 2, middle-sized Lua glue and fixups;
//! - [`m5p_scripting_lua`]: M5 phyre-scripting batch 3, object accessors, Vector and Matrix functions;
//! - [`m5p_scripting_sched`]: M5 phyre-scripting batch 4, the scheduler, scripted components, fixups;
//! - [`m5p_scripting_state`]: M5 phyre-scripting batch 5, script states and runs, member setters;
//! - [`m5p_scripting_more`]: M5 phyre-scripting batch 6, scheduler removal, Vector constructor, type queries;
//! - [`m5p_scripting_run`]: M5 phyre-scripting batch 7, the scheduler's runs;
//! - [`m5p_scripting_vec`]: M5 phyre-scripting batch 8, Vector dot and cross products.
//!
//! What this list does not hold (the class descriptor, the type
//! dispenser, the heap, the red-black tree) is called at its address
//! through [`ffx_graphic::host::Host`], as other ports call original code.

// 32-bit game addresses and words.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

use ffx_graphic::host::{Conv, Host};

pub mod class;
pub mod m5p_scripting_bp2;
pub mod m5p_scripting_df;
pub mod m5p_scripting_left2;
pub mod m5p_scripting_lua;
pub mod m5p_scripting_mid;
pub mod m5p_scripting_more;
pub mod m5p_scripting_run;
pub mod m5p_scripting_sched;
pub mod m5p_scripting_small;
pub mod m5p_scripting_state;
pub mod m5p_scripting_vec;
pub mod object;
pub mod p3_bind;
pub mod p3_init;
pub mod p3_misc;
pub mod p3_misc2;
pub mod p3_misc3;
pub mod p3_tree;
pub mod physics;

/// `Phyre_PhyreFree` (cdecl `(p)`).
pub const PHYRE_FREE: u32 = 0x0042_fb20;

/// Calls the original at `sym` as a `thiscall` on `this`.
pub fn thiscall(h: &mut impl Host, sym: u32, this: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call(f, Conv::Thiscall(this), args)
}

/// Calls the original at `sym` as a cdecl function.
pub fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
    let f = h.rebase(sym);
    h.call(f, Conv::Cdecl, args)
}

/// `Phyre_PhyreFree(p)`; its `eax`.
pub fn free(h: &mut impl Host, p: u32) -> u32 {
    cdecl(h, PHYRE_FREE, &[p])
}

/// `eax` after `mov al, v` over `eax`.
#[must_use]
pub const fn with_al(eax: u32, v: u8) -> u32 {
    (eax & !0xff) | v as u32
}

/// Byte-wise C string equality, as the originals' inlined `strcmp` loop
/// (two bytes per round) tests it.
pub fn str_eq(h: &impl Host, a: u32, b: u32) -> bool {
    let mut i = 0u32;
    loop {
        let x: u8 = h.read(a.wrapping_add(i));
        let y: u8 = h.read(b.wrapping_add(i));
        if x != y {
            return false;
        }
        if x == 0 {
            return true;
        }
        i = i.wrapping_add(1);
    }
}
