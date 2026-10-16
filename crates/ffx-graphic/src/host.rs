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
}

#[cfg(target_arch = "x86")]
impl Host for ffx_bridge::RawMem {
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
