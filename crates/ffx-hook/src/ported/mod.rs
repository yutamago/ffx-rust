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

mod auto;
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
mod vfx_hi;
mod vfx_ppp;
mod vfx_shape;

pub(crate) use battle::ms_screen_process;

/// Every replacement, for the hook table.
#[must_use]
pub fn hooks() -> Vec<Hook> {
    let mut all = Vec::new();
    all.extend(auto::hooks());
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
    all.extend(vfx_hi::hooks());
    all.extend(vfx_ppp::hooks());
    all.extend(vfx_shape::hooks());
    all
}

/// `W` zeroed words on this stack, for a port whose original keeps locals
/// in its frame and passes their addresses ([`Self::at`]) to its bridge or
/// callees. It lives until the end of the statement or block that holds it,
/// like the original's frame.
///
/// Scratch memory for a shim must be per call (this) or per thread
/// (`thread_local!`), never a process-wide `static`: the game runs ports on
/// several threads at once (job-scheduler workers, docs/hooking.md,
/// "Threads"), and a shared buffer gets overwritten mid-call
/// (`tools/check-ported.py` rejects one).
pub(crate) struct StackFrame<const W: usize>([u32; W]);

impl<const W: usize> StackFrame<W> {
    pub(crate) const fn new() -> Self {
        Self([0; W])
    }

    /// The frame's address (32-bit process).
    pub(crate) fn at(&mut self) -> u32 {
        u32::try_from(self.0.as_mut_ptr() as usize).unwrap_or(0)
    }
}

impl<const W: usize> Drop for StackFrame<W> {
    fn drop(&mut self) {
        // the bridge and the originals wrote it through its address
        std::hint::black_box(&mut self.0);
    }
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

/// Whether a shim may take the caller's argument slots (`ported!`): the
/// extra last argument must be a stack word, so not thiscall or fastcall.
pub(crate) const fn slots_abi(abi: &str) -> bool {
    matches!(abi.as_bytes(), b"C" | b"cdecl" | b"stdcall")
}

/// Whether the callee pops its stack arguments (everything but cdecl).
pub(crate) const fn callee_pops(abi: &str) -> bool {
    !matches!(abi.as_bytes(), b"C" | b"cdecl")
}

/// Calls an original cdecl function by absolute address with 0 to 12 word
/// arguments, returning `eax`. cdecl leaves the stack to the caller, so
/// the call is the same for any count; more than 8 is a bug in the bridge
/// that asked.
pub(crate) fn call_cdecl(f: u32, a: &[u32]) -> u32 {
    cdecl12("call_cdecl", f, a)
}

/// [`call_cdecl`] for an original that returns its result in `ST0`
/// (`float` or `double` in C, often `void` in Ghidra: look for
/// `(float10)Name(` at the callers): the value at the x87's precision
/// (53 bits under the game's control word), which the declared `f64`
/// return pops off the x87 stack. Only for a callee that always leaves
/// `ST0` set: popping an empty x87 stack gives a NaN and unbalances it.
pub(crate) fn call_cdecl_f64(f: u32, a: &[u32]) -> f64 {
    cdecl12("call_cdecl_f64", f, a)
}

/// [`call_cdecl_f64`] rounded to `f32` (`fstp dword`), for an original
/// whose callers store its `ST0` as a `float` (`MsCameraGetYaw`,
/// `AtelPopStackFloat`).
pub(crate) fn call_cdecl_f32(f: u32, a: &[u32]) -> f32 {
    cdecl12("call_cdecl_f32", f, a)
}

/// [`call_cdecl`] for an original that returns 64 bits in `edx:eax`
/// (`graphicGetTimeInMicroseconds`).
pub(crate) fn call_cdecl_u64(f: u32, a: &[u32]) -> u64 {
    cdecl12("call_cdecl_u64", f, a)
}

/// The call behind [`call_cdecl`] and its float forms: 12 words, the
/// missing ones zero, `R` from `eax` (`u32`) or `ST0` (`f32`, `f64`).
#[allow(clippy::many_single_char_names)] // the arguments, in order
fn cdecl12<R>(helper: &str, f: u32, a: &[u32]) -> R {
    type F12<R> =
        unsafe extern "C" fn(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32) -> R;
    // No panic here: a panic in a function the game calls ends the process.
    if a.len() > 12 {
        crate::log::log(&format!(
            "{helper}: {} arguments for {f:#010x}, only 12 passed",
            a.len()
        ));
    }
    let mut w = [0u32; 12];
    let n = a.len().min(12);
    w[..n].copy_from_slice(&a[..n]);
    // SAFETY: a cdecl function of at most 12 word arguments returning `R`
    // where the ABI puts it (`eax`, or `ST0` for a float); the extra zero
    // words it doesn't read sit on the caller's side of the stack.
    unsafe {
        let g: F12<R> = std::mem::transmute::<usize, F12<R>>(f as usize);
        g(
            w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7], w[8], w[9], w[10], w[11],
        )
    }
}

/// The most stack words [`call_cdecl_words`] passes.
pub(crate) const WIDE_WORDS: usize = 32;

/// Calls an original cdecl function by absolute address with up to
/// [`WIDE_WORDS`] stack words, returning `eax`: for callees that take a
/// struct by value (`yonExtMapRegister(id, 0x4c-byte struct)` from
/// `opu_unit_etc`, 20 words), which [`call_cdecl`]'s 12 can't hold. `a` is
/// the stack as the original pushes it, lowest address first: a by-value
/// struct is its words in memory order, rounded up to whole words. cdecl
/// leaves the stack to the caller, so the zero words past `a` are only
/// unread stack. More than [`WIDE_WORDS`] would cut a struct short: that is
/// logged and the original is not called (0).
pub(crate) fn call_cdecl_words(f: u32, a: &[u32]) -> u32 {
    #[rustfmt::skip]
    type Wide = unsafe extern "C" fn(
        u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32,
        u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32,
    ) -> u32;
    // No panic here: a panic in a function the game calls ends the process.
    if a.len() > WIDE_WORDS {
        crate::log::log(&format!(
            "call_cdecl_words: {} words for {f:#010x}, at most {WIDE_WORDS}; not called",
            a.len()
        ));
        return 0;
    }
    let mut w = [0u32; WIDE_WORDS];
    w[..a.len()].copy_from_slice(a);
    // SAFETY: a cdecl function of this build taking at most these stack
    // words and returning in `eax`; the zero words it doesn't read sit on
    // the caller's side of the stack, which the caller pops.
    unsafe {
        let g = std::mem::transmute::<usize, Wide>(f as usize);
        g(
            w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7], w[8], w[9], w[10], w[11], w[12], w[13],
            w[14], w[15], w[16], w[17], w[18], w[19], w[20], w[21], w[22], w[23], w[24], w[25],
            w[26], w[27], w[28], w[29], w[30], w[31],
        )
    }
}

/// Calls an original thiscall (or fastcall taking only `ecx`) function by
/// absolute address: `this` in `ecx`, 0 to 8 word arguments on the
/// stack, returning `eax`. The callee pops its stack arguments, so `a`
/// must hold exactly as many words as it takes; more than 8 is a bridge
/// bug, logged, and the original is not called (0).
pub(crate) fn call_thiscall(f: u32, this: u32, a: &[u32]) -> u32 {
    macro_rules! call {
        ($($w:ident),*) => {{
            // SAFETY: a thiscall function of this build that takes these
            // stack words (the bridge's count) and pops them itself.
            unsafe {
                let g = std::mem::transmute::<
                    usize,
                    unsafe extern "thiscall" fn(u32 $(, call!(@word $w))*) -> u32,
                >(f as usize);
                g(this $(, $w)*)
            }
        }};
        (@word $w:ident) => { u32 };
    }
    match *a {
        [] => call!(),
        [a0] => call!(a0),
        [a0, a1] => call!(a0, a1),
        [a0, a1, a2] => call!(a0, a1, a2),
        [a0, a1, a2, a3] => call!(a0, a1, a2, a3),
        [a0, a1, a2, a3, a4] => call!(a0, a1, a2, a3, a4),
        [a0, a1, a2, a3, a4, a5] => call!(a0, a1, a2, a3, a4, a5),
        [a0, a1, a2, a3, a4, a5, a6] => call!(a0, a1, a2, a3, a4, a5, a6),
        [a0, a1, a2, a3, a4, a5, a6, a7] => call!(a0, a1, a2, a3, a4, a5, a6, a7),
        _ => {
            // No panic here: a panic in a function the game calls ends the
            // process. A wrong count would unbalance the stack: don't call.
            crate::log::log(&format!(
                "call_thiscall: {} arguments for {f:#010x}, at most 8; not called",
                a.len()
            ));
            0
        }
    }
}

/// `ported! { /// doc  Name @ 0xADDR => extern "C" fn shim(args) -> ret { body } ... }`:
/// the shims, a register-saving thunk for each ([`Saved`]), and the
/// module's `hooks()`.
///
/// `fn shim(args; slots: u32)` also hands the shim the address of the
/// caller's stack arguments (`slots`, not an argument of the original): the
/// thunk passes the shim a copy of the arguments, so an original that
/// stores into its own argument slots (MSVC reuses them as scratch, and
/// `op_random` leaves its result there) needs its shim to write them
/// through `slots` for callers that read the word after the call
/// (`tools/check-argslots.py`). cdecl and stdcall only.
macro_rules! ported {
    ($( $(#[$doc:meta])* $name:ident @ $addr:literal => extern $abi:literal fn $shim:ident($($arg:ident: $ty:ty),* $(; $slots:ident: $sty:ty)?) $(-> $ret:ty)? $body:block )*) => {
        $(
            $(#[$doc])*
            unsafe extern $abi fn $shim($($arg: $ty,)* $($slots: $sty)?) $(-> $ret)? $body
        )*

        /// This module's replacements.
        #[must_use]
        pub fn hooks() -> Vec<crate::Hook> {
            vec![$(crate::Hook {
                name: stringify!($name),
                address: $addr,
                prologue: &[],
                replacement: $crate::ported::thunk!($abi, $shim, ($($ty),*), ($($ret)?) $(, $slots)?),
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
    // the same, with the address of the caller's arguments as the shim's
    // last (extra) stack argument: pushed first, at `esp + 4` on entry
    ($abi:literal, $shim:ident, ($($ty:ty),*), ($($ret:ty)?), $slots:ident) => {{
        type Ret = $crate::ported::thunk!(@ret $($ret)?);
        const WORDS: usize = $crate::ported::stack_words($abi, &[$(::core::mem::size_of::<$ty>()),*]);
        const EAX: usize = <Ret as $crate::ported::Saved>::EAX as usize;
        const EDX: usize = <Ret as $crate::ported::Saved>::EDX as usize;
        const SAVED: usize = 1 + EAX + EDX;
        const POPS: usize = $crate::ported::callee_pops($abi) as usize;
        const _: () = assert!(
            $crate::ported::slots_abi($abi),
            "argument slots only for cdecl and stdcall shims"
        );
        #[unsafe(naked)]
        unsafe extern "C" fn thunk() {
            ::core::arch::naked_asm!(
                ".if {eax}", "push eax", ".endif",
                "push ecx",
                ".if {edx}", "push edx", ".endif",
                // esp as it was before this push, moved up to the first argument
                "push esp",
                "add dword ptr [esp], {to_args}",
                ".rept {words}", "push dword ptr [esp + {from}]", ".endr",
                "call {shim}",
                ".if {pops} == 0", "add esp, {bytes} + 4", ".endif",
                ".if {edx}", "pop edx", ".endif",
                "pop ecx",
                ".if {eax}", "pop eax", ".endif",
                ".if {pops}", "ret {bytes}", ".else", "ret", ".endif",
                eax = const EAX,
                edx = const EDX,
                words = const WORDS,
                to_args = const 4 * SAVED + 4,
                from = const 4 * (SAVED + WORDS + 1),
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
    ($abi:literal, $shim:ident, ($($ty:ty),*), ($($ret:ty)?) $(, $slots:ident)?) => {
        $shim as *const () as usize
    };
}
pub(crate) use thunk;
