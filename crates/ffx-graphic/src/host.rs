//! Calls from a bridge into code that stays original: `PhyreEngine` and the
//! C++ objects the remaster builds on it (virtual methods, managers), which
//! are library code and never ported, and callees of other modules not
//! ported yet.
//!
//! The render layer is C++: most of its callees are `thiscall` (`this` in
//! `ecx`) or `fastcall`, so [`Host::call`] takes the convention, unlike
//! `ffx_bridge::Host` (cdecl only).

use ffx_bridge::{Mem, route};

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

    /// Calls a function at `addr` that returns a 64-bit integer in
    /// `edx:eax` (`graphicGetTimeInMicroseconds`), as [`Host::call`] with
    /// at most eight words; returns `edx:eax`.
    ///
    /// The default returns `eax` with `edx` read as 0: hosts whose bridges
    /// never call such a function need not read `edx`; the game's and the
    /// oracle tests' hosts that do override it.
    fn call_u64(&mut self, addr: u32, conv: Conv, args: &[u32]) -> u64 {
        u64::from(self.call(addr, conv, args))
    }

    /// [`Host::call_u64`] of the original at `sym`.
    fn call_sym_u64(&mut self, sym: u32, conv: Conv, args: &[u32]) -> u64 {
        let addr = self.rebase(sym);
        self.call_u64(addr, conv, args)
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

    /// Calls a two-operand CRT intrinsic (`_CIpow` through the game's
    /// import thunk at `addr`) with `x` in `ST1` and `y` in `ST0` (each
    /// loaded as `fld qword`); returns its `ST0` result as the caller's
    /// `fstp m32` stores it.
    ///
    /// The default returns NaN, as [`Host::call_intrinsic`]'s does.
    fn call_intrinsic2(&mut self, addr: u32, x: f64, y: f64) -> f32 {
        let _ = (addr, x, y);
        f32::NAN
    }

    /// Calls the `cdecl` function at `addr` with up to [`WIDE_WORDS`]
    /// stack words, returning `eax`: for callees that take more than
    /// [`max_words`]' twelve, such as `AsyncLoadManager_loadCluster`'s
    /// nineteen (its callback's seven 64-bit arguments by value, from
    /// `PhyreScene_fieldMapLoad`). `args` is the stack as the original
    /// pushes it, lowest address first.
    ///
    /// The game's host passes 32 words (the zero words past `args` sit on
    /// the caller's side of the stack, which the caller pops) and refuses
    /// a longer list: nothing is called and it returns 0. The default
    /// forwards to [`Host::call`], so a test host whose bridges pass more
    /// than twelve words must override it (`ffx_oracle::check_width`
    /// against [`WIDE_WORDS`]).
    fn call_wide(&mut self, addr: u32, args: &[u32]) -> u32 {
        self.call(addr, Conv::Cdecl, args)
    }

    /// [`Host::call_wide`] of the original at `sym`.
    fn call_sym_wide(&mut self, sym: u32, args: &[u32]) -> u32 {
        let addr = self.rebase(sym);
        self.call_wide(addr, args)
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

/// The most stack words [`Host::call_wide`] passes.
pub const WIDE_WORDS: usize = 32;

#[cfg(target_arch = "x86")]
thread_local! {
    /// The game thread's scratch (`Host::scratch`), when no router serves
    /// one (the hooked game).
    static SCRATCH: std::cell::UnsafeCell<[u32; SCRATCH_LEN as usize / 4]> =
        const { std::cell::UnsafeCell::new([0; SCRATCH_LEN as usize / 4]) };
}

/// In the game every call goes through `ffx_bridge::route`: a native call
/// in the hooked game, the function table in `ffx-host`. The word counts
/// are the ones this host always passed (`max_words`; a longer list is cut
/// at the limit, as the fixed signatures cut it).
impl Host for ffx_bridge::RawMem {
    fn scratch(&mut self) -> u32 {
        if let Some(at) = route::scratch(route::Scratch::Graphic) {
            return at;
        }
        // the address only: the bridge reads and writes it through `Mem`,
        // the originals through their pointers, as they would their stack
        #[cfg(target_arch = "x86")]
        #[allow(clippy::cast_possible_truncation)] // 32-bit process
        return SCRATCH.with(|s| s.get() as usize as u32);
        #[cfg(not(target_arch = "x86"))]
        0
    }

    fn call_intrinsic(&mut self, addr: u32, x: f64) -> f32 {
        // the intrinsic takes its argument in ST0 and leaves its result
        // there, stored as `fstp m32`
        route::x87(addr, &[x])
    }

    fn call_intrinsic2(&mut self, addr: u32, x: f64, y: f64) -> f32 {
        // the operands in ST1 and ST0 (the intrinsic pops both and leaves
        // its result in ST0)
        route::x87(addr, &[x, y])
    }

    fn call_st0(&mut self, addr: u32, conv: Conv, args: &[u32]) -> f32 {
        float_call_any::<f32>(addr, conv, args)
    }

    fn call_st0_f64(&mut self, addr: u32, conv: Conv, args: &[u32]) -> f64 {
        float_call_any::<f64>(addr, conv, args)
    }

    fn call_u64(&mut self, addr: u32, conv: Conv, args: &[u32]) -> u64 {
        // a `u64` result is `edx:eax` in every 32-bit convention
        float_call_any::<u64>(addr, conv, args)
    }

    fn call_wide(&mut self, addr: u32, args: &[u32]) -> u32 {
        // no panics in what the game calls: a longer list is refused
        if args.len() > WIDE_WORDS {
            return 0;
        }
        let mut w = [0u32; WIDE_WORDS];
        w[..args.len()].copy_from_slice(args);
        // `addr` is a cdecl function taking at most these stack words (the
        // bridges call what the original calls); the zero words it doesn't
        // read sit on the caller's side of the stack, which the caller pops
        route::cdecl(addr, &w)
    }

    fn call(&mut self, addr: u32, conv: Conv, args: &[u32]) -> u32 {
        // the bridges pass at most sixteen to a thiscall, twelve to a
        // cdecl, eight to the others (no panics in what the game calls)
        let mut a = [0u32; 16];
        for (w, &v) in a.iter_mut().zip(args) {
            *w = v;
        }
        let n = args.len();
        // cdecl lets the caller pass more than the callee reads (the zero
        // words past `args` sit on the caller's side of the stack); the
        // callee-pops conventions need the exact count
        match conv {
            Conv::Cdecl => route::cdecl(addr, &a[..12]),
            Conv::Thiscall(this) => route::thiscall(addr, this, &a[..n.min(16)]),
            Conv::Stdcall => route::stdcall(addr, &a[..n.min(8)]),
            // two or fewer: both registers, zero when missing
            Conv::Fastcall => route::fastcall(addr, &a[..n.clamp(2, 8)]),
        }
    }
}

/// [`Host::call_st0`], [`Host::call_st0_f64`] and [`Host::call_u64`] in the
/// game: `R` read from `ST0` (`f32`: `fstp m32`; `f64`: `fstp m64`) or
/// `edx:eax` (`u64`). Eight words for a cdecl, the exact count (at most
/// eight) for the others.
fn float_call_any<R: route::Out>(addr: u32, conv: Conv, args: &[u32]) -> R {
    let mut a = [0u32; 8];
    for (w, &v) in a.iter_mut().zip(args) {
        *w = v;
    }
    let n = args.len().min(8);
    match conv {
        Conv::Cdecl => route::cdecl(addr, &a),
        Conv::Thiscall(this) => route::thiscall(addr, this, &a[..n]),
        Conv::Stdcall => route::stdcall(addr, &a[..n]),
        // the first two in registers; fewer pass zeros there
        Conv::Fastcall => route::fastcall(addr, &a[..n.max(2)]),
    }
}
