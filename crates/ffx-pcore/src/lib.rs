//! `PhyreEngine`'s core, ported function by function from FFX.exe (M4
//! port round 1, list `p5-core`: the functions the game reaches of the
//! math, the thread primitives and pool, and the containers), as the M3
//! pipeline ports game code (`docs/porting.md`, "Library code").
//!
//! - [`matrix`] has `PMatrix4` copies, builders and products (x87, 53 bits);
//! - [`sync`] has `PEventFlagWin32`, `PSemaphoreWin32`, the critical sections,
//!   thread initialisation, the thread pool's small members;
//! - [`array`] has `PArray` assignment, comparison and bit fields;
//! - [`freelist`] has `PFreeList`;
//! - [`string`] has `PString`, `PStringBuilder` and the aligned allocator;
//! - [`p5_more`] has the thread pools' and ring buffer's singletons,
//!   `PShadowCaster`, `PInputAction`'s destructor, `setLightMatrix`, the
//!   transpose and the registrations' tables (continuation round);
//! - [`small`] has the class descriptors' little virtual members (construct
//!   in place, destroy, forward).
//!
//! Win32 calls stay calls through `FFX.exe`'s import slots; everything
//! outside the list is called at its address through
//! [`ffx_graphic::host::Host`].
//!
//! Results in `al` only: several originals end with `mov al, 1` (or
//! `xor al, al`) over whatever `eax` held. Where the rest of `eax` is
//! known (an argument, a constant) the bridge returns all 32 bits; where
//! it is a callee's or the caller's, it returns the byte (the shim's
//! `u8`), and the callers test `al` only (`tools/check-clobber.py`).

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

pub mod array;
pub mod freelist;
pub mod matrix;
pub mod p5_more;
pub mod small;
pub mod string;
pub mod sync;

use ffx_graphic::host::{Conv, Host};

/// `Phyre::PhyreFree` (cdecl `(p)`).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Phyre::PhyreMalloc` with an alignment (cdecl `(size, align)`).
pub const PHYRE_MALLOC_ALIGNED: u32 = 0x0042_fb80;
/// `memcpy`'s import thunk (cdecl `(dst, src, n)`).
pub const MEMCPY: u32 = 0x0094_9256;

/// Calls the Win32 function in `FFX.exe`'s import slot `slot` (stdcall).
pub fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
    let f: u32 = h.read(h.rebase(slot));
    h.call(f, Conv::Stdcall, args)
}

/// `PhyreFree(p)`.
pub fn phyre_free(h: &mut impl Host, p: u32) {
    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
}

/// `PhyreMalloc(size, align)`.
pub fn phyre_malloc(h: &mut impl Host, size: u32, align: u32) -> u32 {
    h.call_sym(PHYRE_MALLOC_ALIGNED, Conv::Cdecl, &[size, align])
}

/// `memcpy(dst, src, n)` (the CRT's).
pub fn memcpy(h: &mut impl Host, dst: u32, src: u32, n: u32) {
    h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, n]);
}

/// The length of the C string at `s` (the originals' inline `strlen`).
pub fn strlen(h: &impl Host, s: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}
