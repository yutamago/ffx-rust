//! Functions rewritten in Rust behind `ffx-bridge` (`docs/rewrite-plan.md`):
//! each line names the original, its address and signature, and the Rust
//! that replaces it. The logic lives in the module crates (portable, checked
//! by `ffx-oracle` against the original on synthetic inputs and snapshots);
//! a shim here only adapts the calling convention and hands over the game's
//! memory as [`RawMem`].
//!
//! A replacement jumps from the original's first five bytes, so it needs a
//! function of at least five bytes; the build check (`check_build`) already
//! pins every address to this exact `FFX.exe`.

//!
//! One file per module crate (`battle`, `kernel`, ...), each with its own
//! `ported!` list, so work on different modules does not touch the same
//! file.

use ffx_bridge::RawMem;

use crate::Hook;

mod battle;
mod chara;
mod kernel;
mod main;
mod map;
mod menu;
mod movie;
mod pc_ui;
mod ps2_emu;
mod render;
mod script;
mod sound;
mod system;
mod text;
mod vfx;

/// Every replacement, for the hook table.
#[must_use]
pub fn hooks() -> Vec<Hook> {
    let mut all = Vec::new();
    all.extend(battle::hooks());
    all.extend(chara::hooks());
    all.extend(kernel::hooks());
    all.extend(main::hooks());
    all.extend(map::hooks());
    all.extend(menu::hooks());
    all.extend(movie::hooks());
    all.extend(pc_ui::hooks());
    all.extend(ps2_emu::hooks());
    all.extend(render::hooks());
    all.extend(script::hooks());
    all.extend(sound::hooks());
    all.extend(system::hooks());
    all.extend(text::hooks());
    all.extend(vfx::hooks());
    all
}

/// The game's memory, for the bridges.
fn mem() -> RawMem {
    #[allow(clippy::cast_possible_truncation)] // 32-bit process
    RawMem {
        image: crate::GAME_BASE.load(std::sync::atomic::Ordering::Acquire) as u32,
    }
}

/// Calls an original function by its `ffx.sym` address (cdecl, no
/// arguments), for bridges that still need one.
fn call0(sym: u32) -> u32 {
    // SAFETY: the address is a cdecl function of this build taking nothing.
    unsafe {
        let f: unsafe extern "C" fn() -> u32 = std::mem::transmute(crate::rebase(sym));
        f()
    }
}

/// `ported! { /// doc  Name @ 0xADDR => extern "C" fn shim(args) -> ret { body } ... }`:
/// the shims and the module's `hooks()`.
macro_rules! ported {
    ($( $(#[$doc:meta])* $name:ident @ $addr:literal => extern $abi:literal fn $shim:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? $body:block )*) => {
        $(
            $(#[$doc])*
            unsafe extern $abi fn $shim($($arg: $ty),*) $(-> $ret)? $body
        )*

        /// This module's replacements.
        #[must_use]
        pub fn hooks() -> Vec<crate::Hook> {
            vec![$(crate::Hook {
                name: stringify!($name),
                address: $addr,
                prologue: &[],
                replacement: $shim as *const () as usize,
                original: None,
                steal: 0,
            }),*]
        }
    };
}
use ported;
