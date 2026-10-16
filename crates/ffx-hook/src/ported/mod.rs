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

/// The scratch registers a shim's thunk saves for the caller, by the
/// shim's return type: what it returns in stays the shim's, the rest keeps
/// the caller's value.
///
/// The originals were compiled with whole-program optimisation, which lets
/// a caller keep `eax`, `ecx` or `edx` live across a call to a function it
/// knows leaves them alone (`tools/check-clobber.py`: `op_spline_xyz_table`
/// returns the `eax` of a call before `op_spr_free`, which only decrements
/// a byte). A Rust function clobbers all three, as the ABI allows, so every
/// replacement goes through a thunk that keeps them.
pub(crate) trait Saved {
    /// Whether `eax` is saved (nothing returned in it).
    const EAX: bool;
    /// Whether `edx` is saved (no 64-bit result).
    const EDX: bool;
}

macro_rules! saved {
    ($($t:ty => $eax:literal, $edx:literal;)*) => {
        $(impl Saved for $t {
            const EAX: bool = $eax;
            const EDX: bool = $edx;
        })*
    };
}
saved! {
    () => true, true;
    f32 => true, true;
    f64 => true, true;
    bool => false, true;
    u8 => false, true;
    i8 => false, true;
    u16 => false, true;
    i16 => false, true;
    u32 => false, true;
    i32 => false, true;
    u64 => false, false;
    i64 => false, false;
}

/// The stack words a shim's caller pushed: every argument for cdecl and
/// stdcall, all but `this` for thiscall (`ecx`), all but the first two for
/// fastcall (`ecx`, `edx`).
pub(crate) const fn stack_words(abi: &str, sizes: &[usize]) -> usize {
    let skip = match abi.as_bytes() {
        b"thiscall" => 1,
        b"fastcall" => 2,
        _ => 0,
    };
    let mut words = 0;
    let mut i = skip;
    while i < sizes.len() {
        words += sizes[i].div_ceil(4);
        i += 1;
    }
    words
}

/// Whether the callee pops its stack arguments (everything but cdecl).
pub(crate) const fn callee_pops(abi: &str) -> bool {
    !matches!(abi.as_bytes(), b"C" | b"cdecl")
}

/// Calls an original cdecl function by absolute address with 0 to 12 word
/// arguments, returning `eax`. cdecl leaves the stack to the caller, so
/// the call is the same for any count; more than 8 is a bug in the bridge
/// that asked.
#[allow(clippy::many_single_char_names)] // the arguments, in order
pub(crate) fn call_cdecl(f: u32, a: &[u32]) -> u32 {
    type F12 =
        unsafe extern "C" fn(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32) -> u32;
    // No panic here: a panic in a function the game calls ends the process.
    if a.len() > 12 {
        crate::log::log(&format!(
            "call_cdecl: {} arguments, only 12 passed",
            a.len()
        ));
    }
    let mut w = [0u32; 12];
    let n = a.len().min(12);
    w[..n].copy_from_slice(&a[..n]);
    // SAFETY: a cdecl function of at most 12 word arguments; the extra
    // zero words it doesn't read sit on the caller's side of the stack.
    unsafe {
        let g: F12 = std::mem::transmute::<usize, F12>(f as usize);
        g(
            w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7], w[8], w[9], w[10], w[11],
        )
    }
}

/// `ported! { /// doc  Name @ 0xADDR => extern "C" fn shim(args) -> ret { body } ... }`:
/// the shims, a register-saving thunk for each ([`Saved`]), and the
/// module's `hooks()`.
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
                replacement: $crate::ported::thunk!($abi, $shim, ($($ty),*), ($($ret)?)),
                original: None,
                steal: 0,
            }),*]
        }
    };
}
use ported;

/// The address of `$shim`'s thunk: saves the scratch registers its return
/// type leaves alone, copies the caller's stack arguments, calls the shim
/// and returns as the original did (`ret n` for the callee-pops ABIs).
#[cfg(target_arch = "x86")]
macro_rules! thunk {
    ($abi:literal, $shim:ident, ($($ty:ty),*), ($($ret:ty)?)) => {{
        type Ret = $crate::ported::thunk!(@ret $($ret)?);
        const WORDS: usize = $crate::ported::stack_words($abi, &[$(::core::mem::size_of::<$ty>()),*]);
        const EAX: usize = <Ret as $crate::ported::Saved>::EAX as usize;
        const EDX: usize = <Ret as $crate::ported::Saved>::EDX as usize;
        const SAVED: usize = 1 + EAX + EDX;
        const POPS: usize = $crate::ported::callee_pops($abi) as usize;
        #[unsafe(naked)]
        unsafe extern "C" fn thunk() {
            ::core::arch::naked_asm!(
                ".if {eax}", "push eax", ".endif",
                "push ecx",
                ".if {edx}", "push edx", ".endif",
                ".rept {words}", "push dword ptr [esp + {from}]", ".endr",
                "call {shim}",
                ".if {pops} == 0", ".if {words}", "add esp, {bytes}", ".endif", ".endif",
                ".if {edx}", "pop edx", ".endif",
                "pop ecx",
                ".if {eax}", "pop eax", ".endif",
                ".if {pops}", "ret {bytes}", ".else", "ret", ".endif",
                eax = const EAX,
                edx = const EDX,
                words = const WORDS,
                from = const 4 * (SAVED + WORDS),
                bytes = const 4 * WORDS,
                pops = const POPS,
                shim = sym $shim,
            )
        }
        thunk as *const () as usize
    }};
    (@ret) => { () };
    (@ret $ret:ty) => { $ret };
}
/// Off the game's architecture (host builds, clippy) the shim itself.
#[cfg(not(target_arch = "x86"))]
macro_rules! thunk {
    ($abi:literal, $shim:ident, ($($ty:ty),*), ($($ret:ty)?)) => {
        $shim as *const () as usize
    };
}
pub(crate) use thunk;
