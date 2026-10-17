//! The effects' random numbers: VU0's `R` register as the PC port
//! emulates it (`g_Vu0_R`, `0x00c0a000`).
//!
//! `R` holds the bits of a float in `[1, 2)`. A step (the PS2's `vrnext`)
//! shifts the low 22 fraction bits up by one and feeds bit 4 of
//! `R ^ (R >> 18)` into the bottom. Until something seeds it
//! (`g_Vu0_RSeeded` is 0), a draw uses 1.0 instead and marks it seeded.
//!
//! `op_random(x)` and `op_random_p(x)` scale a draw to `[-x, x)` and
//! `[0, x)`, and leave `R` at the draw minus 1.8872414 (the next seed),
//! with the x87 arithmetic of the original ([`crate::x87`]).
//!
//! The particle programs `pppRandFloat`, `pppRandUpFloat`,
//! `pppRandDownFloat` and their vector forms (`pppRandFV`, `...UpFV`,
//! `...DownFV`) draw differently, in the PC port's translation of the
//! PS2's VU0 code ([`bridge::draw`]): `t0 = r0 - 1` is copied into a VF
//! register, `R ^= t0 & 0x7fffff`, the CRT is seeded with
//! `srand(R & 0x7fffff)`, `vrnext` draws, and the draw is that plus
//! `t0` as a float. Every emulated register write is reproduced. At an
//! object's frame 0 the program draws once, or twice when the key's flag
//! byte is set, and keeps the value `w` in its work area:
//! - plain: `2 d1` or `d2 + d1`, applied as `target += w k - k`;
//! - up: `d1` or `(d2 + d1) / 2`, applied as `target += k w`;
//! - down: `-d1` or `(-d1 - d2) / 2`, applied as `target += k w`.
//!
//! At later frames whose key applies it adds again with the kept `w`. The
//! target is a field of the object (key `+4`) or `g_pppRandTarget` for -1;
//! `k` is the key's float at `+8` (or three at `+0x10`).

use crate::x87;

/// The constant the draws subtract for the next seed (`f64` at
/// `0x00b58250`; it has 21 significant bits, so `draw - C` is exact).
const NEXT: f64 = 1.887_241_363_525_390_6;

/// One step of `R` (bits in, bits out).
#[must_use]
pub const fn step(r: u32) -> u32 {
    ((((r >> 18) ^ r) >> 4) & 1) | (((r & 0x003f_ffff) | 0x1fc0_0000) << 1)
}

/// A draw in `[1, 2)`: `R` stepped, or 1.0 while unseeded.
#[must_use]
pub fn draw(r: u32, seeded: bool) -> f32 {
    if seeded { f32::from_bits(step(r)) } else { 1.0 }
}

/// `op_random(x)`: from the draw `d`, (`2d - 3) * x` and the next `R`.
#[must_use]
pub fn signed(d: f32, x: f32) -> (f32, f32) {
    let two = d + d; // exact
    (x87::mul(x87::sub_f64(two, 3.0), x), x87::sub_f64(two, NEXT))
}

/// `op_random_p(x)`: from the draw `d`, `(d - 1) * x` and the next `R`.
#[must_use]
pub fn positive(d: f32, x: f32) -> (f32, f32) {
    let unit = x87::sub_f64(d, 1.0);
    (x87::mul(unit, x), x87::sub_f64(unit, NEXT))
}

/// The generator where the game keeps it.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::x87;

    /// `g_Vu0_R`: VU0's `R`, as `f32` bits.
    pub const R: Global<u32> = Global::new(0x00c0_a000);
    /// `g_Vu0_RSeeded`: nonzero once `R` has been seeded.
    pub const SEEDED: Global<u32> = Global::new(0x00c8_f728);

    fn run(mem: &mut impl Mem, x: f32, f: fn(f32, f32) -> (f32, f32)) -> f32 {
        let d = super::draw(R.get(mem), SEEDED.get(mem) != 0);
        SEEDED.set(mem, 1);
        let (v, next) = f(d, x);
        R.set(mem, next.to_bits());
        v
    }

    /// `op_random(x)` (`0x007e6700`): a random value in `[-x, x)`.
    pub fn op_random(mem: &mut impl Mem, x: f32) -> f32 {
        run(mem, x, super::signed)
    }

    /// `op_random_p(x)` (`0x007e6780`): a random value in `[0, x)`.
    pub fn op_random_p(mem: &mut impl Mem, x: f32) -> f32 {
        run(mem, x, super::positive)
    }

    /// Copies the emulated VU0 register word `from` to `to` (both
    /// addresses before rebasing).
    fn vmove(mem: &mut impl Mem, to: u32, from: u32) {
        let v: u32 = mem.read(Global::<u32>::new(from).ptr(mem).addr);
        mem.write(Global::<u32>::new(to).ptr(mem).addr, v);
    }

    fn vset(mem: &mut impl Mem, to: u32, v: u32) {
        mem.write(Global::<u32>::new(to).ptr(mem).addr, v);
    }

    fn vget(mem: &impl Mem, from: u32) -> u32 {
        mem.read(Global::<u32>::new(from).ptr(mem).addr)
    }

    /// The draw of the integer generators: `R` stepped into memory when
    /// seeded (1.0 otherwise), as `f32`.
    fn idraw(mem: &mut impl Mem) -> f32 {
        if SEEDED.get(mem) == 0 {
            1.0
        } else {
            let r = super::step(R.get(mem));
            R.set(mem, r);
            f32::from_bits(r)
        }
    }

    #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
    fn ifloat(v: u32) -> u32 {
        (v.cast_signed() as f32).to_bits()
    }

    /// `op_irandom_core_007e35c0(n, b, c, d)` (`0x007e35c0`): a random
    /// integer in `[0, n)`: `(draw - 1) * n` truncated (`_ftol2_sse`),
    /// through the emulated VU0 registers as the original moves them (the
    /// four arguments to `0x00c8f970`, the draw to `0x00c8f984`..`98c`,
    /// the result and `b`..`d` to `0x00c8f5a8`..`5b4`); `R` becomes the
    /// draw minus 1.8872414; seeded after.
    pub fn op_irandom_core(mem: &mut impl Mem, args: [u32; 4]) -> u32 {
        for (i, a) in (0u32..).zip(args) {
            vset(mem, 0x00c8_f970 + 4 * i, a);
        }
        vset(mem, 0x00c8_f538, 1.0f32.to_bits());
        vset(mem, 0x00c8_f558, 0x3ff1_9120);
        let v = idraw(mem);
        vmove(mem, 0x00c8_f964, 0x00c8_f53c);
        vset(mem, 0x00c8_f984, v.to_bits());
        vset(mem, 0x00c8_f988, v.to_bits());
        vmove(mem, 0x00c8_f968, 0x00c8_f540);
        vset(mem, 0x00c8_f98c, v.to_bits());
        vmove(mem, 0x00c8_f96c, 0x00c8_f544);
        vmove(mem, 0x00c8_f944, 0x00c8_f55c);
        vmove(mem, 0x00c8_f948, 0x00c8_f560);
        vset(mem, 0x00c8_f970, ifloat(args[0]));
        let one = f32::from_bits(vget(mem, 0x00c8_f538));
        vset(mem, 0x00c8_f960, 1.0f32.to_bits());
        vset(mem, 0x00c8_f940, 0x3ff1_9120);
        vset(mem, 0x00c8_f980, x87::sub(v, one).to_bits());
        vmove(mem, 0x00c8_f94c, 0x00c8_f564);
        ffx_ps2::ee::bridge::restore_vf00(mem);
        let next = x87::sub(
            f32::from_bits(vget(mem, 0x00c8_f98c)),
            f32::from_bits(vget(mem, 0x00c8_f940)),
        );
        vset(mem, 0x00c8_f92c, next.to_bits());
        let t = x87::mul(
            f32::from_bits(vget(mem, 0x00c8_f980)),
            f32::from_bits(vget(mem, 0x00c8_f970)),
        );
        vset(mem, 0x00c8_f970, t.to_bits());
        let n = ffx_bridge::x87::ftol(mem, x87::Reg::load_f32(t).value());
        R.set(mem, x87::load(vget(mem, 0x00c8_f92c)));
        vmove(mem, 0x00c8_f5ac, 0x00c8_f974);
        vmove(mem, 0x00c8_f5b0, 0x00c8_f978);
        vset(mem, 0x00c8_f970, n);
        vset(mem, 0x00c8_f5a8, n);
        vmove(mem, 0x00c8_f5b4, 0x00c8_f97c);
        SEEDED.set(mem, 1);
        n
    }

    /// `g_pppColorConShift` (`0x00c8b500`): `op_random_per` stores its
    /// result at this byte offset from its first argument's stack slot,
    /// then clears it.
    pub const COLOR_CON_SHIFT: Global<u32> = Global::new(0x00c8_b500);

    /// `op_random_per(n, b, c, d)` (`0x007e33e0`): a random integer in
    /// `[-n, n)`: `(2 draw - 3) * n` truncated, through the emulated VU0
    /// registers as the original moves them (`b`, `c`, `d` converted to
    /// floats in `0x00c8f974`..`97c`); `R` becomes the draw minus
    /// 1.8872414. The result goes to the first argument's stack slot at
    /// `COLOR_CON_SHIFT` (then cleared), and the slot is returned: the
    /// result when the shift is 0, else the scaled float's bits with any
    /// overlapping bytes of the result. (A shift outside the slot writes
    /// elsewhere on the original's stack; that is not reproduced.)
    pub fn op_random_per(mem: &mut impl Mem, args: [u32; 4]) -> u32 {
        for (i, a) in (0u32..).zip(args) {
            vset(mem, 0x00c8_f970 + 4 * i, a);
        }
        vset(mem, 0x00c8_f538, 2.0f32.to_bits());
        vset(mem, 0x00c8_f548, 3.0f32.to_bits());
        vset(mem, 0x00c8_f558, 0x3ff1_9120);
        let v = idraw(mem);
        vmove(mem, 0x00c8_f964, 0x00c8_f53c);
        vset(mem, 0x00c8_f984, v.to_bits());
        vmove(mem, 0x00c8_f968, 0x00c8_f540);
        vset(mem, 0x00c8_f988, v.to_bits());
        vset(mem, 0x00c8_f98c, v.to_bits());
        vmove(mem, 0x00c8_f96c, 0x00c8_f544);
        vmove(mem, 0x00c8_f954, 0x00c8_f54c);
        vset(mem, 0x00c8_f970, ifloat(args[0]));
        vmove(mem, 0x00c8_f958, 0x00c8_f550);
        vmove(mem, 0x00c8_f95c, 0x00c8_f554);
        vset(mem, 0x00c8_f974, ifloat(args[1]));
        vmove(mem, 0x00c8_f944, 0x00c8_f55c);
        vmove(mem, 0x00c8_f948, 0x00c8_f560);
        vset(mem, 0x00c8_f978, ifloat(args[2]));
        vset(mem, 0x00c8_f960, 2.0f32.to_bits());
        vset(mem, 0x00c8_f950, 3.0f32.to_bits());
        vset(mem, 0x00c8_f940, 0x3ff1_9120);
        vset(mem, 0x00c8_f97c, ifloat(args[3]));
        vmove(mem, 0x00c8_f94c, 0x00c8_f564);
        let two = x87::mul(f32::from_bits(vget(mem, 0x00c8_f538)), v);
        vset(mem, 0x00c8_f980, two.to_bits());
        let next = x87::sub(v, f32::from_bits(vget(mem, 0x00c8_f558)));
        vset(mem, 0x00c8_f92c, next.to_bits());
        let u = x87::sub(two, f32::from_bits(vget(mem, 0x00c8_f548)));
        vset(mem, 0x00c8_f980, u.to_bits());
        ffx_ps2::ee::bridge::restore_vf00(mem);
        let t = x87::mul(f32::from_bits(vget(mem, 0x00c8_f970)), u);
        let n = ffx_bridge::x87::ftol(mem, x87::Reg::load_f32(t).value());
        R.set(mem, x87::load(vget(mem, 0x00c8_f92c)));
        let shift = COLOR_CON_SHIFT.get(mem).cast_signed();
        vmove(mem, 0x00c8_f5ac, 0x00c8_f974);
        vmove(mem, 0x00c8_f5b0, 0x00c8_f978);
        let mut slot = t.to_bits().to_le_bytes();
        for (i, b) in (0i32..).zip(n.to_le_bytes()) {
            if let Ok(j @ 0..4) = usize::try_from(shift.wrapping_add(i)) {
                slot[j] = b;
            }
        }
        vset(mem, 0x00c8_f970, n);
        vset(mem, 0x00c8_f5a8, n);
        vmove(mem, 0x00c8_f5b4, 0x00c8_f97c);
        COLOR_CON_SHIFT.set(mem, 0);
        SEEDED.set(mem, 1);
        u32::from_le_bytes(slot)
    }

    /// `g_Ee_Gpr`'s `r0`, low word.
    const R0: Global<u32> = Global::new(0x00c8_f508);
    /// `g_Ee_Gpr`'s `t0` (four words).
    const T0: Global<u32> = Global::new(0x00c8_f588);
    /// The emulated VF register the draws leave their value in (x).
    pub const VF_DRAW: Global<u32> = Global::new(0x00c8_f7a0);
    /// The emulated VF register that takes `t0` (four words).
    pub const VF_T0: Global<u32> = Global::new(0x00c8_f7b0);
    /// `g_pppRandTarget`: what the rand programs add to for target -1.
    pub const TARGET: Global<u32> = Global::new(0x00d5_2398);

    fn at(mem: &impl Mem, g: Global<u32>, word: u32) -> u32 {
        g.ptr(mem).addr + 4 * word
    }

    /// `pppVu0RNext(out, _)` (`0x0072f420`): `vrnext`: `R` stepped (when
    /// seeded; it does not mark it seeded) into `out`, else 1.0.
    pub fn ppp_vu0_rnext(mem: &mut impl Mem, out: u32) {
        let v = if SEEDED.get(mem) != 0 {
            let r = super::step(R.get(mem));
            R.set(mem, r);
            r
        } else {
            1.0f32.to_bits()
        };
        mem.write(out, crate::x87::load(v));
    }

    /// One draw of the rand programs, `srand` being the CRT's.
    pub fn draw<M: Mem>(mem: &mut M, srand: &mut impl FnMut(&mut M, u32)) -> f32 {
        let t0 = R0.get(mem).wrapping_sub(1);
        let hi = (t0.cast_signed() >> 31).cast_unsigned();
        T0.set(mem, t0);
        VF_T0.set(mem, t0);
        let z = mem.read::<u32>(at(mem, T0, 2));
        mem.write(at(mem, VF_T0, 2), z);
        let w = mem.read::<u32>(at(mem, T0, 3));
        mem.write(at(mem, VF_T0, 3), w);
        let r = R.get(mem) ^ (crate::x87::load(t0) & 0x007f_ffff);
        R.set(mem, r);
        mem.write(at(mem, T0, 1), hi);
        mem.write(at(mem, VF_T0, 1), hi);
        srand(mem, r & 0x007f_ffff);
        #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
        let tf = VF_T0.get(mem).cast_signed() as f32;
        VF_T0.set(mem, tf.to_bits());
        let out = VF_DRAW.ptr(mem).addr;
        ppp_vu0_rnext(mem, out);
        let d = crate::x87::add(f32::from_bits(VF_DRAW.get(mem)), tf);
        VF_DRAW.set(mem, d.to_bits());
        d
    }

    /// How a rand program spreads its draws.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Spread {
        /// `pppRandFloat`, `pppRandFV`.
        Plain,
        /// `pppRandUpFloat`, `pppRandUpFV`.
        Up,
        /// `pppRandDownFloat`, `pppRandDownFV`.
        Down,
    }

    /// What a rand program applies its value to: the key's factors and the
    /// targets.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Kind {
        /// One `f32` factor at `+8`, an `f32` target (`pppRandFloat`).
        Float,
        /// Three `f32` factors at `+0x10`, `f32` targets (`pppRandFV`).
        FloatVec,
        /// One `u8` factor at `+8`, a byte target (`pppRandChar`).
        Char,
        /// One `u16` factor at `+8`, a half-word target (`pppRandShort`).
        Short,
        /// One `u32` factor at `+8` (as an `f32`), a word target
        /// (`pppRandInt`).
        Int,
        /// Three `i32` factors at `+0x10` (as `f32`), word targets
        /// (`pppRandIV`).
        IntVec,
        /// Four `i8` factors at `+8`, byte targets (`pppRandCV`).
        CharVec,
        /// Four `i16` factors at `+8`, half-word targets (`pppRandHCV`).
        ShortVec,
    }

    impl Kind {
        /// Where the key's "draw twice" flag byte is.
        #[must_use]
        pub const fn flag(self) -> u32 {
            match self {
                Self::Float | Self::Int | Self::CharVec => 0xc,
                Self::FloatVec | Self::IntVec => 0x20,
                Self::Char => 9,
                Self::Short => 0xa,
                Self::ShortVec => 0x10,
            }
        }
    }

    /// The rand programs (`pppRand{,Up,Down}{Float,FV,Char,Short,Int,IV,CV,HCV}`).
    pub fn ppp_rand<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        spread: Spread,
        kind: Kind,
        srand: &mut impl FnMut(&mut M, u32),
    ) {
        if crate::ppp::bridge::STOP.get(mem) != 0 {
            return;
        }
        let frame = mem.read::<u32>(obj.wrapping_add(0xc));
        // the integer vector forms draw whenever their key applies
        let per_key = matches!(kind, Kind::IntVec | Kind::CharVec | Kind::ShortVec);
        if per_key && mem.read::<u32>(key) != frame {
            return;
        }
        let work = |mem: &M| {
            let offsets = mem.read::<u32>(ctrl.wrapping_add(0xc));
            obj.wrapping_add(0xa0)
                .wrapping_add(mem.read::<u32>(offsets))
        };
        let w_at = if per_key || frame == 0 {
            let two = mem.read::<u8>(key + kind.flag()) != 0;
            let w = spread_draw(mem, spread, two, srand);
            let w_at = work(mem);
            mem.write(w_at, w.to_bits());
            w_at
        } else {
            if mem.read::<u32>(key) != frame {
                return;
            }
            work(mem)
        };
        apply(mem, obj, key, spread, kind, w_at, 0, false);
    }

    /// One spread value: a draw (negated for `Down`), or with `two` the
    /// two draws combined.
    fn spread_draw<M: Mem>(
        mem: &mut M,
        spread: Spread,
        two: bool,
        srand: &mut impl FnMut(&mut M, u32),
    ) -> f32 {
        use crate::x87::{ADD, MUL, Reg, SUB};
        let d1 = draw(mem, srand);
        let d1 = if spread == Spread::Down { -d1 } else { d1 };
        if two {
            let d2 = draw(mem, srand);
            match spread {
                Spread::Plain => x87::add(d2, d1),
                Spread::Up => Reg::load_f32(d2).mem(ADD, d1).constant(MUL, 0.5).store(),
                Spread::Down => Reg::load_f32(d1).mem(SUB, d2).constant(MUL, 0.5).store(),
            }
        } else if spread == Spread::Plain {
            let r = Reg::load_f32(d1);
            r.reg(ADD, r).store()
        } else {
            d1
        }
    }

    /// The seeded-per-axis rand programs (`pppSRand{,Up,Down}{FV,CV,HCV}`,
    /// `kind` `FloatVec`, `CharVec` or `ShortVec`): as [`ppp_rand`], but
    /// each of the three (four) targets gets a draw of its own, kept at
    /// `+4 i` in the work area. `FV` draws at frame 0, the others whenever
    /// their key applies; the `HCV` forms add the sign-extended low byte of
    /// each `ftol` (a `cbw` the plain rand programs do not have).
    pub fn ppp_srand<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        ctrl: u32,
        spread: Spread,
        kind: Kind,
        srand: &mut impl FnMut(&mut M, u32),
    ) {
        if crate::ppp::bridge::STOP.get(mem) != 0 {
            return;
        }
        let frame = mem.read::<u32>(obj.wrapping_add(0xc));
        let draws = if kind == Kind::FloatVec {
            if frame != 0 && mem.read::<u32>(key) != frame {
                return;
            }
            frame == 0
        } else {
            if mem.read::<u32>(key) != frame {
                return;
            }
            true
        };
        let offsets = mem.read::<u32>(ctrl.wrapping_add(0xc));
        let w_at = obj
            .wrapping_add(0xa0)
            .wrapping_add(mem.read::<u32>(offsets));
        if draws {
            let n = if kind == Kind::FloatVec { 3 } else { 4 };
            for i in 0..n {
                let two = mem.read::<u8>(key + kind.flag()) != 0;
                let w = spread_draw(mem, spread, two, srand);
                mem.write(w_at.wrapping_add(4 * i), w.to_bits());
            }
        }
        apply(mem, obj, key, spread, kind, w_at, 4, kind == Kind::ShortVec);
    }

    /// Adds a rand program's value (at `w_at`, the next one `stride`
    /// further for each target) to its targets.
    #[allow(clippy::too_many_arguments)]
    fn apply<M: Mem>(
        mem: &mut M,
        obj: u32,
        key: u32,
        spread: Spread,
        kind: Kind,
        w_at: u32,
        stride: u32,
        cbw: bool,
    ) {
        use crate::x87::{ADD, MUL, Reg, SUB};
        let to = mem.read::<u32>(key + 4);
        let t_at = if to == u32::MAX {
            TARGET.ptr(mem).addr
        } else {
            obj.wrapping_add(0xa0).wrapping_add(to)
        };
        let float = |mem: &M, a: u32| f32::from_bits(mem.read(a));
        match kind {
            Kind::Float => {
                let k = float(mem, key + 8);
                let w = float(mem, w_at);
                let term = if spread == Spread::Plain {
                    Reg::load_f32(w).mem(MUL, k).mem(SUB, k)
                } else {
                    Reg::load_f32(k).mem(MUL, w)
                };
                let sum = term.mem(ADD, float(mem, t_at)).store();
                mem.write(t_at, sum.to_bits());
            }
            Kind::FloatVec => {
                for i in 0..3 {
                    let k = float(mem, key + 0x10 + 4 * i);
                    let w = float(mem, w_at.wrapping_add(stride * i));
                    let term = if spread == Spread::Plain {
                        let kk = Reg::load_f32(x87::load_f32(k));
                        Reg::load_f32(w).reg(MUL, kk).reg(SUB, kk).store()
                    } else {
                        x87::mul(k, w)
                    };
                    let target = t_at.wrapping_add(4 * i);
                    let sum = x87::add(term, float(mem, target));
                    mem.write(target, sum.to_bits());
                }
            }
            _ => apply_int(mem, spread, kind, key, w_at, t_at, stride, cbw),
        }
    }

    /// The integer rand programs' application: `ftol` of `w k - k` (plain)
    /// or `k w`, its low bits added to each target.
    #[allow(clippy::too_many_arguments)]
    fn apply_int<M: Mem>(
        mem: &mut M,
        spread: Spread,
        kind: Kind,
        key: u32,
        w_at: u32,
        t_at: u32,
        stride: u32,
        cbw: bool,
    ) {
        use crate::x87::{MUL, Reg, SUB};
        let (count, width) = match kind {
            Kind::Char => (1, 1),
            Kind::Short => (1, 2),
            Kind::Int => (1, 4),
            Kind::IntVec => (3, 4),
            Kind::CharVec => (4, 1),
            _ => (4, 2),
        };
        for i in 0..count {
            #[allow(clippy::cast_precision_loss)] // fild, then fstp m32
            let factor = match kind {
                Kind::Char => Reg::Num(f64::from(mem.read::<u8>(key + 8))),
                Kind::Short => Reg::Num(f64::from(mem.read::<u16>(key + 8))),
                Kind::Int => Reg::load_f32(mem.read::<u32>(key + 8) as f32),
                Kind::IntVec => Reg::load_f32(mem.read::<i32>(key + 0x10 + 4 * i) as f32),
                Kind::CharVec => Reg::Num(f64::from(mem.read::<i8>(key + 8 + i))),
                _ => Reg::Num(f64::from(mem.read::<i16>(key + 8 + 2 * i))),
            };
            let value = f32::from_bits(mem.read(w_at.wrapping_add(stride * i)));
            let product = if spread == Spread::Plain {
                Reg::load_f32(value).reg(MUL, factor).reg(SUB, factor)
            } else {
                factor.mem(MUL, value)
            };
            let int = ffx_bridge::x87::ftol(mem, product.value());
            let target = t_at.wrapping_add(width * i);
            #[allow(clippy::cast_possible_truncation)] // the low byte or half
            match width {
                1 => {
                    let sum = mem.read::<u8>(target).wrapping_add(int as u8);
                    mem.write(target, sum);
                }
                2 => {
                    let add = if cbw {
                        i16::from(int as i8).cast_unsigned()
                    } else {
                        int as u16
                    };
                    let sum = mem.read::<u16>(target).wrapping_add(add);
                    mem.write(target, sum);
                }
                _ => {
                    let sum = mem.read::<u32>(target).wrapping_add(int);
                    mem.write(target, sum);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_stay_in_one_to_two() {
        let mut r = 0x3f80_0000;
        for _ in 0..1000 {
            r = step(r);
            let f = f32::from_bits(r);
            assert!((1.0..2.0).contains(&f), "{f}");
        }
        assert_eq!(NEXT.to_bits(), 0x3ffe_3224_0000_0000);
    }
}
