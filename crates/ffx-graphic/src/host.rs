//! Calls from a bridge into code that stays original: `PhyreEngine` and the
//! C++ objects the remaster builds on it (virtual methods, managers), which
//! are library code and never ported, and callees of other modules not
//! ported yet.
//!
//! The render layer is C++: most of its callees are `thiscall` (`this` in
//! `ecx`) or `fastcall`, so [`Host::call`] takes the convention, unlike
//! `ffx_bridge::Host` (cdecl only).

use ffx_bridge::Mem;

/// How a called original takes its arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conv {
    /// On the stack, the caller pops them.
    Cdecl,
    /// On the stack, the callee pops them.
    Stdcall,
    /// `this` in `ecx`, the rest on the stack, the callee pops them.
    Thiscall(u32),
    /// The first two in `ecx` and `edx`, the rest on the stack, the callee
    /// pops them.
    Fastcall,
}

/// Game memory plus calls into original code. In the game a plain call
/// ([`ffx_bridge::RawMem`]); in `ffx-oracle`, the emulator.
pub trait Host: Mem {
    /// Calls the function at `addr` (an address in this process, already
    /// rebased) with up to eight 32-bit stack arguments; returns `eax`.
    fn call(&mut self, addr: u32, conv: Conv, args: &[u32]) -> u32;

    /// Calls a function at `addr` that returns a float in `ST0`, as
    /// [`Host::call`]; returns it as the caller's `fstp m32` stores it.
    /// Callers read nothing else of its registers.
    ///
    /// The default calls it and returns NaN: hosts whose bridges never
    /// call a float function (other modules' test hosts) need not read
    /// `ST0`; the game's and the render oracle's hosts override it.
    fn call_st0(&mut self, addr: u32, conv: Conv, args: &[u32]) -> f32 {
        self.call(addr, conv, args);
        f32::NAN
    }

    /// Calls a CRT intrinsic (`_CIsin`, `_CIcos` through the game's import
    /// thunk at `addr`) with `x` in `ST0`; returns its `ST0` result as the
    /// caller's `fstp m32` stores it.
    ///
    /// The default returns NaN, as [`Host::call_st0`]'s does.
    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        let _ = (addr, x);
        f32::NAN
    }

    /// [`Host::call_st0`] of the original at `sym`.
    fn call_sym_st0(&mut self, sym: u32, conv: Conv, args: &[u32]) -> f32 {
        let addr = self.rebase(sym);
        self.call_st0(addr, conv, args)
    }

    /// Calls the original at `sym` (its `ffx.sym` address).
    fn call_sym(&mut self, sym: u32, conv: Conv, args: &[u32]) -> u32 {
        let addr = self.rebase(sym);
        self.call(addr, conv, args)
    }

    /// Calls virtual method `slot` (the vtable's word index) of `obj`
    /// (`thiscall`).
    fn call_virtual(&mut self, obj: u32, slot: u32, args: &[u32]) -> u32 {
        let vtable: u32 = self.read(obj);
        let f: u32 = self.read(vtable + 4 * slot);
        self.call(f, Conv::Thiscall(obj), args)
    }

    /// [`SCRATCH_LEN`] bytes of memory the original callees can reach,
    /// for what the original kept in its stack frame and passed by
    /// address (matrices handed to `PhyreEngine`). Private to the calling
    /// thread; a bridge must not keep it across calls.
    fn scratch(&mut self) -> u32;
}

/// The size of [`Host::scratch`].
pub const SCRATCH_LEN: u32 = 0x200;

#[cfg(target_arch = "x86")]
thread_local! {
    /// The game thread's scratch (`Host::scratch`).
    static SCRATCH: std::cell::UnsafeCell<[u32; SCRATCH_LEN as usize / 4]> =
        const { std::cell::UnsafeCell::new([0; SCRATCH_LEN as usize / 4]) };
}

#[cfg(target_arch = "x86")]
impl Host for ffx_bridge::RawMem {
    fn scratch(&mut self) -> u32 {
        // the address only: the bridge reads and writes it through `Mem`,
        // the originals through their pointers, as they would their stack
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        SCRATCH.with(|s| s.get() as usize as u32)
    }

    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        let mut out = 0f32;
        // SAFETY: `addr` is the import thunk of a CRT intrinsic, which takes
        // its argument in ST0 and leaves its result alone there (eax, ecx,
        // edx, the x87 and SSE registers clobbered). The x87 stack is empty
        // here (Rust keeps no values in it). Only edi, which the callee
        // keeps, is read after the call.
        unsafe {
            std::arch::asm!(
                "fld qword ptr [{x}]",
                "call {f}",
                "fstp dword ptr [edi]",
                x = in(reg) &raw const x,
                f = in(reg) addr,
                in("edi") &raw mut out,
                clobber_abi("C"),
            );
        }
        out
    }

    fn call_st0(&mut self, addr: u32, conv: Conv, args: &[u32]) -> f32 {
        let mut a = [0u32; 8];
        for (w, &v) in a.iter_mut().zip(args) {
            *w = v;
        }
        let n = args.len().min(8);
        let f = addr as usize;
        // SAFETY: as for `call`; the callee leaves its result in ST0,
        // which the float-returning signature reads (no inline assembly).
        unsafe {
            match conv {
                Conv::Cdecl => float_cdecl(f, a),
                Conv::Thiscall(this) => float_thiscall(f, this, n, a),
                Conv::Stdcall => float_stdcall(f, n, a),
                Conv::Fastcall => float_fastcall(f, n, a),
            }
        }
    }

    #[allow(clippy::many_single_char_names)] // the arguments
    fn call(&mut self, addr: u32, conv: Conv, args: &[u32]) -> u32 {
        type C8 = unsafe extern "C" fn(u32, u32, u32, u32, u32, u32, u32, u32) -> u32;
        // the bridges pass at most eight (no panics in what the game calls)
        let mut a = [0u32; 8];
        for (w, &v) in a.iter_mut().zip(args) {
            *w = v;
        }
        let [p, q, r, s, t, u, v, w] = a;
        let f = addr as usize;
        // SAFETY: `addr` is a function of this process with this
        // convention and these arguments (the bridges call what the
        // original calls). cdecl lets the caller pass more than the callee
        // reads; the callee-pops conventions need the exact count.
        unsafe {
            match (conv, args.len()) {
                (Conv::Cdecl, _) => std::mem::transmute::<usize, C8>(f)(p, q, r, s, t, u, v, w),
                (Conv::Thiscall(this), n) => thiscall(f, this, n, a),
                (Conv::Stdcall, n) => stdcall(f, n, a),
                (Conv::Fastcall, n) => fastcall(f, n, a),
            }
        }
    }
}

/// A `thiscall` with `n` stack arguments.
///
/// # Safety
/// `f` takes `this` and exactly `n` 32-bit arguments.
#[cfg(target_arch = "x86")]
#[allow(clippy::many_single_char_names)] // the arguments
unsafe fn thiscall(f: usize, this: u32, n: usize, a: [u32; 8]) -> u32 {
    use std::mem::transmute as t;
    let [p, q, r, s, u, v, w, x] = a;
    // SAFETY: as the caller promises.
    unsafe {
        match n {
            0 => t::<usize, unsafe extern "thiscall" fn(u32) -> u32>(f)(this),
            1 => t::<usize, unsafe extern "thiscall" fn(u32, u32) -> u32>(f)(this, p),
            2 => t::<usize, unsafe extern "thiscall" fn(u32, u32, u32) -> u32>(f)(this, p, q),
            3 => {
                t::<usize, unsafe extern "thiscall" fn(u32, u32, u32, u32) -> u32>(f)(this, p, q, r)
            }
            4 => t::<usize, unsafe extern "thiscall" fn(u32, u32, u32, u32, u32) -> u32>(f)(
                this, p, q, r, s,
            ),
            5 => t::<usize, unsafe extern "thiscall" fn(u32, u32, u32, u32, u32, u32) -> u32>(f)(
                this, p, q, r, s, u,
            ),
            6 => t::<usize, unsafe extern "thiscall" fn(u32, u32, u32, u32, u32, u32, u32) -> u32>(
                f,
            )(this, p, q, r, s, u, v),
            7 => t::<
                usize,
                unsafe extern "thiscall" fn(u32, u32, u32, u32, u32, u32, u32, u32) -> u32,
            >(f)(this, p, q, r, s, u, v, w),
            _ => t::<
                usize,
                unsafe extern "thiscall" fn(u32, u32, u32, u32, u32, u32, u32, u32, u32) -> u32,
            >(f)(this, p, q, r, s, u, v, w, x),
        }
    }
}

/// A `stdcall` with `n` arguments.
///
/// # Safety
/// `f` takes exactly `n` 32-bit arguments.
#[cfg(target_arch = "x86")]
#[allow(clippy::many_single_char_names)] // the arguments
unsafe fn stdcall(f: usize, n: usize, a: [u32; 8]) -> u32 {
    use std::mem::transmute as t;
    let [p, q, r, s, u, v, w, x] = a;
    // SAFETY: as the caller promises.
    unsafe {
        match n {
            0 => t::<usize, unsafe extern "stdcall" fn() -> u32>(f)(),
            1 => t::<usize, unsafe extern "stdcall" fn(u32) -> u32>(f)(p),
            2 => t::<usize, unsafe extern "stdcall" fn(u32, u32) -> u32>(f)(p, q),
            3 => t::<usize, unsafe extern "stdcall" fn(u32, u32, u32) -> u32>(f)(p, q, r),
            4 => t::<usize, unsafe extern "stdcall" fn(u32, u32, u32, u32) -> u32>(f)(p, q, r, s),
            5 => t::<usize, unsafe extern "stdcall" fn(u32, u32, u32, u32, u32) -> u32>(f)(
                p, q, r, s, u,
            ),
            6 => t::<usize, unsafe extern "stdcall" fn(u32, u32, u32, u32, u32, u32) -> u32>(f)(
                p, q, r, s, u, v,
            ),
            7 => t::<usize, unsafe extern "stdcall" fn(u32, u32, u32, u32, u32, u32, u32) -> u32>(
                f,
            )(p, q, r, s, u, v, w),
            _ => t::<
                usize,
                unsafe extern "stdcall" fn(u32, u32, u32, u32, u32, u32, u32, u32) -> u32,
            >(f)(p, q, r, s, u, v, w, x),
        }
    }
}

/// A `fastcall` with `n` arguments (the first two in registers).
///
/// # Safety
/// `f` takes exactly `n` 32-bit arguments (`n` of 0 to 2 pass zeros in
/// the unused registers).
#[cfg(target_arch = "x86")]
#[allow(clippy::many_single_char_names)] // the arguments
unsafe fn fastcall(f: usize, n: usize, a: [u32; 8]) -> u32 {
    use std::mem::transmute as t;
    let [p, q, r, s, u, v, w, x] = a;
    // SAFETY: as the caller promises.
    unsafe {
        match n {
            0..=2 => t::<usize, unsafe extern "fastcall" fn(u32, u32) -> u32>(f)(p, q),
            3 => t::<usize, unsafe extern "fastcall" fn(u32, u32, u32) -> u32>(f)(p, q, r),
            4 => t::<usize, unsafe extern "fastcall" fn(u32, u32, u32, u32) -> u32>(f)(p, q, r, s),
            5 => t::<usize, unsafe extern "fastcall" fn(u32, u32, u32, u32, u32) -> u32>(f)(
                p, q, r, s, u,
            ),
            6 => t::<usize, unsafe extern "fastcall" fn(u32, u32, u32, u32, u32, u32) -> u32>(f)(
                p, q, r, s, u, v,
            ),
            7 => t::<usize, unsafe extern "fastcall" fn(u32, u32, u32, u32, u32, u32, u32) -> u32>(
                f,
            )(p, q, r, s, u, v, w),
            _ => t::<
                usize,
                unsafe extern "fastcall" fn(u32, u32, u32, u32, u32, u32, u32, u32) -> u32,
            >(f)(p, q, r, s, u, v, w, x),
        }
    }
}

/// The float-returning calls: one arm per stack-argument count, from the
/// signature's ABI and the `this`/register words it takes first.
#[cfg(target_arch = "x86")]
macro_rules! float_call {
    ($f:ident, $abi:literal, $n:expr, ($($pre:expr),*), $a:ident) => {{
        use std::mem::transmute as t;
        let [p, q, r, s, u, v, w, x] = $a;
        match $n {
            0 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre)),*) -> f32>($f)($($pre),*),
            1 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32) -> f32>($f)($($pre,)* p),
            2 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32) -> f32>($f)($($pre,)* p, q),
            3 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32, u32) -> f32>($f)($($pre,)* p, q, r),
            4 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32, u32, u32) -> f32>($f)($($pre,)* p, q, r, s),
            5 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32, u32, u32, u32) -> f32>($f)($($pre,)* p, q, r, s, u),
            6 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32, u32, u32, u32, u32) -> f32>($f)($($pre,)* p, q, r, s, u, v),
            7 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32, u32, u32, u32, u32, u32) -> f32>($f)($($pre,)* p, q, r, s, u, v, w),
            _ => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32, u32, u32, u32, u32, u32, u32) -> f32>($f)($($pre,)* p, q, r, s, u, v, w, x),
        }
    }};
    (@ty $e:expr) => { u32 };
}

/// A cdecl call returning `ST0` (the callee reads the words it takes).
///
/// # Safety
/// `f` is a cdecl function of at most 8 word arguments returning a float.
#[cfg(target_arch = "x86")]
#[allow(clippy::many_single_char_names)] // the arguments
unsafe fn float_cdecl(f: usize, a: [u32; 8]) -> f32 {
    let [p, q, r, s, u, v, w, x] = a;
    // SAFETY: as the caller promises.
    unsafe {
        std::mem::transmute::<
            usize,
            unsafe extern "C" fn(u32, u32, u32, u32, u32, u32, u32, u32) -> f32,
        >(f)(p, q, r, s, u, v, w, x)
    }
}

/// A `thiscall` with `n` stack arguments returning `ST0`.
///
/// # Safety
/// `f` takes `this` and exactly `n` 32-bit arguments and returns a float.
#[cfg(target_arch = "x86")]
#[allow(clippy::many_single_char_names)] // the arguments
unsafe fn float_thiscall(f: usize, this: u32, n: usize, a: [u32; 8]) -> f32 {
    // SAFETY: as the caller promises.
    unsafe { float_call!(f, "thiscall", n, (this), a) }
}

/// A `stdcall` with `n` arguments returning `ST0`.
///
/// # Safety
/// `f` takes exactly `n` 32-bit arguments and returns a float.
#[cfg(target_arch = "x86")]
#[allow(clippy::many_single_char_names)] // the arguments
unsafe fn float_stdcall(f: usize, n: usize, a: [u32; 8]) -> f32 {
    // SAFETY: as the caller promises.
    unsafe { float_call!(f, "stdcall", n, (), a) }
}

/// A `fastcall` with `n` arguments returning `ST0` (the first two in
/// registers; fewer pass zeros there).
///
/// # Safety
/// `f` takes exactly `n` 32-bit arguments and returns a float.
#[cfg(target_arch = "x86")]
#[allow(clippy::many_single_char_names)] // the arguments
unsafe fn float_fastcall(f: usize, n: usize, a: [u32; 8]) -> f32 {
    let [p, q, rest @ ..] = a;
    let b = [rest[0], rest[1], rest[2], rest[3], rest[4], rest[5], 0, 0];
    // SAFETY: as the caller promises.
    unsafe { float_call!(f, "fastcall", n.saturating_sub(2), (p, q), b) }
}
