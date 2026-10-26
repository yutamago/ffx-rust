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

/// The most words [`Host::call`] (`st0` false) or [`Host::call_st0`] /
/// [`Host::call_st0_f64`] (`st0` true) passes in the game
/// ([`ffx_bridge::RawMem`]) with `conv`: twelve for a plain `cdecl` call
/// and sixteen stack words after `this` for a plain `thiscall`; eight
/// otherwise (a fastcall's two register words included). The twelve cdecl
/// words cover the glue's widest calls: `registAsyncLoadTask`'s nine
/// (`PhyreScene_loadCharacterAsync`), the point-light setup's ten
/// (`graphicPS2LightingSetupEffect`). The sixteen thiscall words cover
/// `PRenderThread::handleNonRenderInstanceCommand`'s fifteen (`0x0059bac0`).
///
/// The game's host does not refuse a longer list: a cdecl call drops the
/// words past the limit (the callee reads the caller's stack there), and
/// the callee-pops conventions pop more than were pushed. Test hosts in
/// `ffx-oracle` fail a wider call (`ffx_oracle::check_width`), as
/// `ffx_bridge::Host`'s rule says.
#[must_use]
pub const fn max_words(conv: Conv, st0: bool) -> usize {
    match (conv, st0) {
        (Conv::Thiscall(_), false) => 16,
        (Conv::Cdecl, false) => 12,
        _ => 8,
    }
}

/// Game memory plus calls into original code. In the game a plain call
/// ([`ffx_bridge::RawMem`]); in `ffx-oracle`, the emulator.
///
/// Call width: see [`max_words`] and `ffx_bridge::Host`.
pub trait Host: Mem {
    /// Calls the function at `addr` (an address in this process, already
    /// rebased) with up to [`max_words`] 32-bit arguments (twelve for
    /// `cdecl`, sixteen stack words for a `thiscall`, else eight); returns
    /// `eax`.
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

    /// Calls a function at `addr` that returns a `double` in `ST0`, as
    /// [`Host::call`]; returns it as the caller's `fstp m64` stores it
    /// (`Phyre_PTimerWin32_GetTime`, whose callers subtract and scale it
    /// before rounding). Callers read nothing else of its registers.
    ///
    /// The default calls it and returns NaN, as [`Host::call_st0`]'s does.
    fn call_st0_f64(&mut self, addr: u32, conv: Conv, args: &[u32]) -> f64 {
        self.call(addr, conv, args);
        f64::NAN
    }

    /// [`Host::call_st0_f64`] of the original at `sym`.
    fn call_sym_st0_f64(&mut self, sym: u32, conv: Conv, args: &[u32]) -> f64 {
        let addr = self.rebase(sym);
        self.call_st0_f64(addr, conv, args)
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
        float_call_any::<f32>(addr, conv, args)
    }

    fn call_st0_f64(&mut self, addr: u32, conv: Conv, args: &[u32]) -> f64 {
        float_call_any::<f64>(addr, conv, args)
    }

    #[allow(clippy::many_single_char_names)] // the arguments
    fn call(&mut self, addr: u32, conv: Conv, args: &[u32]) -> u32 {
        type C12 =
            unsafe extern "C" fn(u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32, u32) -> u32;
        // the bridges pass at most sixteen to a thiscall, twelve to a
        // cdecl, eight to the others (no panics in what the game calls)
        let mut a16 = [0u32; 16];
        for (w, &v) in a16.iter_mut().zip(args) {
            *w = v;
        }
        let [p, q, r, s, t, u, v, w, x, y, z, b, ..] = a16;
        let a = [p, q, r, s, t, u, v, w];
        let f = addr as usize;
        // SAFETY: `addr` is a function of this process with this
        // convention and these arguments (the bridges call what the
        // original calls). cdecl lets the caller pass more than the callee
        // reads (the zero words past `args` sit on the caller's side of
        // the stack); the callee-pops conventions need the exact count.
        unsafe {
            match (conv, args.len()) {
                (Conv::Cdecl, _) => {
                    std::mem::transmute::<usize, C12>(f)(p, q, r, s, t, u, v, w, x, y, z, b)
                }
                (Conv::Thiscall(this), n) => thiscall(f, this, n, a16),
                (Conv::Stdcall, n) => stdcall(f, n, a),
                (Conv::Fastcall, n) => fastcall(f, n, a),
            }
        }
    }
}

/// A `thiscall` with `n` stack arguments (at most 16).
///
/// # Safety
/// `f` takes `this` and exactly `n` 32-bit arguments.
#[cfg(target_arch = "x86")]
#[allow(clippy::many_single_char_names, clippy::too_many_lines)] // the arguments
unsafe fn thiscall(f: usize, this: u32, n: usize, a: [u32; 16]) -> u32 {
    use std::mem::transmute as t;
    type U = u32;
    let [p, q, r, s, u, v, w, x, y, z, b, c, d, e, g, h] = a;
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
            8 => t::<usize, unsafe extern "thiscall" fn(U, U, U, U, U, U, U, U, U) -> U>(f)(
                this, p, q, r, s, u, v, w, x,
            ),
            9 => t::<usize, unsafe extern "thiscall" fn(U, U, U, U, U, U, U, U, U, U) -> U>(f)(
                this, p, q, r, s, u, v, w, x, y,
            ),
            10 => t::<usize, unsafe extern "thiscall" fn(U, U, U, U, U, U, U, U, U, U, U) -> U>(f)(
                this, p, q, r, s, u, v, w, x, y, z,
            ),
            11 => t::<usize, unsafe extern "thiscall" fn(U, U, U, U, U, U, U, U, U, U, U, U) -> U>(
                f,
            )(this, p, q, r, s, u, v, w, x, y, z, b),
            12 => {
                t::<usize, unsafe extern "thiscall" fn(U, U, U, U, U, U, U, U, U, U, U, U, U) -> U>(
                    f,
                )(this, p, q, r, s, u, v, w, x, y, z, b, c)
            }
            13 => t::<
                usize,
                unsafe extern "thiscall" fn(U, U, U, U, U, U, U, U, U, U, U, U, U, U) -> U,
            >(f)(this, p, q, r, s, u, v, w, x, y, z, b, c, d),
            14 => t::<
                usize,
                unsafe extern "thiscall" fn(U, U, U, U, U, U, U, U, U, U, U, U, U, U, U) -> U,
            >(f)(this, p, q, r, s, u, v, w, x, y, z, b, c, d, e),
            15 => t::<
                usize,
                unsafe extern "thiscall" fn(U, U, U, U, U, U, U, U, U, U, U, U, U, U, U, U) -> U,
            >(f)(this, p, q, r, s, u, v, w, x, y, z, b, c, d, e, g),
            _ => t::<
                usize,
                unsafe extern "thiscall" fn(U, U, U, U, U, U, U, U, U, U, U, U, U, U, U, U, U) -> U,
            >(f)(this, p, q, r, s, u, v, w, x, y, z, b, c, d, e, g, h),
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

/// [`Host::call_st0`] and [`Host::call_st0_f64`] in the game: `ST0` read
/// as `R` (`f32`: `fstp m32`; `f64`: `fstp m64`) by the float-returning
/// signature (no inline assembly).
#[cfg(target_arch = "x86")]
fn float_call_any<R>(addr: u32, conv: Conv, args: &[u32]) -> R {
    let mut a = [0u32; 8];
    for (w, &v) in a.iter_mut().zip(args) {
        *w = v;
    }
    let n = args.len().min(8);
    let f = addr as usize;
    // SAFETY: as for `Host::call`; the callee leaves its result in ST0,
    // which the float-returning signature reads.
    unsafe {
        match conv {
            Conv::Cdecl => float_cdecl::<R>(f, a),
            Conv::Thiscall(this) => float_thiscall::<R>(f, this, n, a),
            Conv::Stdcall => float_stdcall::<R>(f, n, a),
            Conv::Fastcall => float_fastcall::<R>(f, n, a),
        }
    }
}

/// The float-returning calls: one arm per stack-argument count, from the
/// signature's ABI and the `this`/register words it takes first; `ST0`
/// read as `$r`.
#[cfg(target_arch = "x86")]
macro_rules! float_call {
    ($f:ident, $abi:literal, $r:ty, $n:expr, ($($pre:expr),*), $a:ident) => {{
        use std::mem::transmute as t;
        let [p, q, r, s, u, v, w, x] = $a;
        match $n {
            0 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre)),*) -> $r>($f)($($pre),*),
            1 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32) -> $r>($f)($($pre,)* p),
            2 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32) -> $r>($f)($($pre,)* p, q),
            3 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32, u32) -> $r>($f)($($pre,)* p, q, r),
            4 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32, u32, u32) -> $r>($f)($($pre,)* p, q, r, s),
            5 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32, u32, u32, u32) -> $r>($f)($($pre,)* p, q, r, s, u),
            6 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32, u32, u32, u32, u32) -> $r>($f)($($pre,)* p, q, r, s, u, v),
            7 => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32, u32, u32, u32, u32, u32) -> $r>($f)($($pre,)* p, q, r, s, u, v, w),
            _ => t::<usize, unsafe extern $abi fn($(float_call!(@ty $pre),)* u32, u32, u32, u32, u32, u32, u32, u32) -> $r>($f)($($pre,)* p, q, r, s, u, v, w, x),
        }
    }};
    (@ty $e:expr) => { u32 };
}

/// A cdecl call returning `ST0` (the callee reads the words it takes).
///
/// # Safety
/// `f` is a cdecl function of at most 8 word arguments returning a float
/// (`R` is `f32` or `f64`).
#[cfg(target_arch = "x86")]
#[allow(clippy::many_single_char_names)] // the arguments
unsafe fn float_cdecl<R>(f: usize, a: [u32; 8]) -> R {
    let [p, q, r, s, u, v, w, x] = a;
    // SAFETY: as the caller promises.
    unsafe {
        std::mem::transmute::<
            usize,
            unsafe extern "C" fn(u32, u32, u32, u32, u32, u32, u32, u32) -> R,
        >(f)(p, q, r, s, u, v, w, x)
    }
}

/// A `thiscall` with `n` stack arguments returning `ST0`.
///
/// # Safety
/// `f` takes `this` and exactly `n` 32-bit arguments and returns a float.
#[cfg(target_arch = "x86")]
#[allow(clippy::many_single_char_names)] // the arguments
unsafe fn float_thiscall<R>(f: usize, this: u32, n: usize, a: [u32; 8]) -> R {
    // SAFETY: as the caller promises.
    unsafe { float_call!(f, "thiscall", R, n, (this), a) }
}

/// A `stdcall` with `n` arguments returning `ST0`.
///
/// # Safety
/// `f` takes exactly `n` 32-bit arguments and returns a float.
#[cfg(target_arch = "x86")]
#[allow(clippy::many_single_char_names)] // the arguments
unsafe fn float_stdcall<R>(f: usize, n: usize, a: [u32; 8]) -> R {
    // SAFETY: as the caller promises.
    unsafe { float_call!(f, "stdcall", R, n, (), a) }
}

/// A `fastcall` with `n` arguments returning `ST0` (the first two in
/// registers; fewer pass zeros there).
///
/// # Safety
/// `f` takes exactly `n` 32-bit arguments and returns a float.
#[cfg(target_arch = "x86")]
#[allow(clippy::many_single_char_names)] // the arguments
unsafe fn float_fastcall<R>(f: usize, n: usize, a: [u32; 8]) -> R {
    let [p, q, rest @ ..] = a;
    let b = [rest[0], rest[1], rest[2], rest[3], rest[4], rest[5], 0, 0];
    // SAFETY: as the caller promises.
    unsafe { float_call!(f, "fastcall", R, n.saturating_sub(2), (p, q), b) }
}
