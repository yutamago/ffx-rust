//! `opp_anmm`'s sprite-animation commands.
//!
//! `opp_anmm` (`0x008124b0`) steps a list of animated sprites. Each sprite
//! (`work`, 0x50 bytes or more) runs a command list: a command is an `i16`
//! whose low byte picks the handler in the table at `0x00c49318` and whose
//! high bits are flags (the interpreter keeps it in the host's `u16
//! +0x21a`). A handler gets `(host, set, cmd, work)` and returns the next
//! command's address; the interpreter runs handlers until one sets the
//! host's wait byte `+0x210`.
//!
//! The flag nibble (`+0x21a` bits 12..15) says which of x, y, z (and w)
//! the vector commands read: one `i16` each, in that order. Bits 8..11
//! (`(+0x21a >> 7) & 0xe`, an even index `i`) choose the target: the `i16`
//! offset at `0x00c49300 + 2 i` into `work`, scaled by the words at
//! `0x00c49398 + 16 i` (x, y, z) and 1/4096.
//!
//! The random commands call `op_irandom_core`/`op_random_per` with one
//! argument; those read four, so the other three are the handler's own
//! stack words (saved registers, a return address). They only reach the
//! emulated VU0 registers, and are a parameter here (`junk`).

/// The handlers where the game keeps them.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::rand::bridge as rand;
    use crate::x87::{ADD, MUL, Reg};

    /// The `i16` work offsets of the vector targets (`0x00c49300`).
    const TARGETS: u32 = 0x00c4_9300;
    /// The `i16` work offsets of the short targets (`0x00c4930c`).
    const SHORTS: u32 = 0x00c4_930c;
    /// The targets' `i32` x, y, z scales and a fourth word, 16 bytes each
    /// (`0x00c49398`).
    const SCALES: u32 = 0x00c4_9398;
    /// The `f64` constant 1/4096 (`0x00b50190`).
    const FIXED: f64 = 1.0 / 4096.0;

    fn word(mem: &impl Mem, at: u32) -> u32 {
        Global::<u32>::new(at).ptr(mem).addr
    }

    fn flags(mem: &impl Mem, host: u32) -> u16 {
        mem.read(host.wrapping_add(0x21a))
    }

    /// The vector target of the flags: its address in `work` and its x, y,
    /// z scales.
    fn target(mem: &impl Mem, f: u16, work: u32) -> (u32, [i32; 3]) {
        let i = u32::from((f >> 7) & 0xe);
        let off: i16 = mem.read(word(mem, TARGETS) + 2 * i);
        let at = work.wrapping_add(i32::from(off).cast_unsigned());
        let s: [i32; 3] = mem.read(word(mem, SCALES) + 16 * i);
        (at, s)
    }

    /// `fild` of the scaled `i16` then `fstp m32`.
    #[allow(clippy::cast_precision_loss)] // the fstp m32 rounding
    fn scaled(mem: &impl Mem, at: u32, scale: i32) -> f32 {
        (i32::from(mem.read::<i16>(at)).wrapping_mul(scale)) as f32
    }

    /// `0x008106e0` (`opp_anmm_wait_random`): the wait `+0x210` = a random
    /// number below the command's high byte (`+0x21b`,
    /// `op_irandom_core`). Returns `cmd + 2`.
    pub fn wait_random(mem: &mut impl Mem, host: u32, cmd: u32, junk: [u32; 3]) -> u32 {
        let n = u32::from(mem.read::<u8>(host.wrapping_add(0x21b)));
        let r = rand::op_irandom_core(mem, [n, junk[0], junk[1], junk[2]]);
        #[allow(clippy::cast_possible_truncation)] // mov [esi+0x210], al
        mem.write(host.wrapping_add(0x210), r as u8);
        cmd.wrapping_add(2)
    }

    /// `0x00810720` (`opp_anmm_sprite_set`): the sprite takes texture
    /// `i16 +2` of its part's oef (part `u16 work +0x4c` of `set`'s 0x100-byte
    /// parts at `[set +0x20]`, oef `[set +0x60 + 4 * part byte +0x1e]`, the
    /// entry at `oef + [oef +0x10] + 4 i`) at `+4`; `+8` = 0xff010000 with
    /// the part's byte `+0xb3` in its low byte; `+0xc` = 0x100; `+0x1c` =
    /// 0x80808080; `+0x2c` and `+0x3c` = 0. Returns `cmd + 4`.
    pub fn sprite_set(mem: &mut impl Mem, set: u32, cmd: u32, work: u32) -> u32 {
        let parts: u32 = mem.read(set.wrapping_add(0x20));
        let no = u32::from(mem.read::<u16>(work.wrapping_add(0x4c)));
        let part = (no << 8).wrapping_add(parts);
        let g = u32::from(mem.read::<u8>(part.wrapping_add(0x1e)));
        let oef: u32 = mem.read(set.wrapping_add(0x60).wrapping_add(4 * g));
        let i = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        let base: u32 = mem.read(oef.wrapping_add(0x10));
        let tex: u32 = mem.read(base.wrapping_add(i.wrapping_mul(4)).wrapping_add(oef));
        mem.write(work.wrapping_add(4), tex);
        mem.write(work.wrapping_add(8), 0xff01_0000u32);
        let b: u8 = mem.read(part.wrapping_add(0xb3));
        mem.write(work.wrapping_add(8), b);
        mem.write(work.wrapping_add(0xc), 0x100u32);
        mem.write(work.wrapping_add(0x1c), 0x8080_8080u32);
        mem.write(work.wrapping_add(0x2c), 0u32);
        mem.write(work.wrapping_add(0x3c), 0u32);
        cmd.wrapping_add(4)
    }

    /// `0x00810790` (`opp_anmm_flag_on`): with no flag bits (`+0x21a &
    /// 0xf000` clear) the sprite's byte `+8` gets bit 7. Returns `cmd + 2`.
    pub fn flag_on(mem: &mut impl Mem, host: u32, cmd: u32, work: u32) -> u32 {
        if flags(mem, host) & 0xf000 == 0 {
            let b: u8 = mem.read(work.wrapping_add(8));
            mem.write(work.wrapping_add(8), b | 0x80);
        }
        cmd.wrapping_add(2)
    }

    /// `0x008107c0` (`opp_anmm_vec_set`): each flagged x, y, z of the
    /// target = the command's `i16` times its scale (wrapping, rounded to
    /// `f32`) / 4096. Returns the address after the values read.
    pub fn vec_set(mem: &mut impl Mem, host: u32, cmd: u32, work: u32) -> u32 {
        let fl = flags(mem, host);
        let (at, scale) = target(mem, fl, work);
        let mut p = cmd.wrapping_add(2);
        for (k, bit) in [0x1000u16, 0x2000, 0x4000].into_iter().enumerate() {
            if fl & bit != 0 {
                let v = Reg::load_f32(scaled(mem, p, scale[k]))
                    .constant(MUL, FIXED)
                    .store();
                #[allow(clippy::cast_possible_truncation)] // 0..3
                mem.write(at.wrapping_add(4 * k as u32), v);
                p = p.wrapping_add(2);
            }
        }
        p
    }

    /// `0x00810a90` (`opp_anmm_vec_add`): as [`vec_set`], added to the
    /// target's values.
    pub fn vec_add(mem: &mut impl Mem, host: u32, cmd: u32, work: u32) -> u32 {
        let fl = flags(mem, host);
        let (at, scale) = target(mem, fl, work);
        let mut p = cmd.wrapping_add(2);
        for (k, bit) in [0x1000u16, 0x2000, 0x4000].into_iter().enumerate() {
            if fl & bit != 0 {
                #[allow(clippy::cast_possible_truncation)] // 0..3
                let t = at.wrapping_add(4 * k as u32);
                let v = Reg::load_f32(scaled(mem, p, scale[k]))
                    .constant(MUL, FIXED)
                    .mem(ADD, mem.read(t))
                    .store();
                mem.write(t, v);
                p = p.wrapping_add(2);
            }
        }
        p
    }

    /// The `f64` constant `NEXT` of [`crate::rand`] (`0x00b58250`).
    const NEXT: f64 = 1.887_241_363_525_390_6;

    /// One step of VU0's `R` from a register value: `fstp m32` of it, then
    /// `vrnext` on the bits.
    fn next(r: Reg) -> f32 {
        f32::from_bits(crate::rand::step(r.store().to_bits()))
    }

    /// `0x00810890` (`opp_anmm_vec_add_random`): as [`vec_add`], each value
    /// times its own `2 d - 3` for three draws `d` of VU0's `R` (the first
    /// 1.0 while unseeded; each next one from the draw minus 1.8872414,
    /// `R` left at the third minus that). The flagged values take the
    /// draws in order. Returns the address after the values read.
    pub fn vec_add_random(mem: &mut impl Mem, host: u32, cmd: u32, work: u32) -> u32 {
        let fl = flags(mem, host);
        let (at, scale) = target(mem, fl, work);
        let d0 = crate::rand::draw(rand::R.get(mem), rand::SEEDED.get(mem) != 0);
        rand::SEEDED.set(mem, 1);
        let d1 = next(Reg::load_f32(d0).constant(crate::x87::SUB, NEXT));
        let d2 = next(Reg::load_f32(d1).constant(crate::x87::SUB, NEXT));
        let seed = Reg::load_f32(d2).constant(crate::x87::SUB, NEXT).store();
        rand::R.set(mem, seed.to_bits());
        let draws = [d0, d1, d2];
        let mut p = cmd.wrapping_add(2);
        let mut used = 0;
        for (k, bit) in [0x1000u16, 0x2000, 0x4000].into_iter().enumerate() {
            if fl & bit != 0 {
                let w = Reg::load_f32(draws[used])
                    .constant(MUL, 2.0)
                    .constant(crate::x87::SUB, 3.0)
                    .store();
                #[allow(clippy::cast_possible_truncation)] // 0..3
                let t = at.wrapping_add(4 * k as u32);
                let v = Reg::load_f32(scaled(mem, p, scale[k]))
                    .constant(MUL, FIXED)
                    .reg(MUL, Reg::load_f32(w))
                    .mem(ADD, mem.read(t))
                    .store();
                mem.write(t, v);
                p = p.wrapping_add(2);
                used += 1;
            }
        }
        p
    }

    /// `0x00810ba0` (`opp_anmm_short_add_random`): the `i16` of `work` at
    /// the offset `0x00c4930c[(i16 +0x21a >> 12) & 7]` += a random number
    /// in `[-n, n)` for the command's `i16 +2` (`op_random_per`). Returns
    /// `cmd + 4`.
    pub fn short_add_random(
        mem: &mut impl Mem,
        host: u32,
        cmd: u32,
        work: u32,
        junk: [u32; 3],
    ) -> u32 {
        let i = ((i32::from(flags(mem, host).cast_signed()) >> 12) & 7).cast_unsigned();
        let off: i16 = mem.read(word(mem, SHORTS) + 2 * i);
        let at = work.wrapping_add(i32::from(off).cast_unsigned());
        let n = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        let r = rand::op_random_per(mem, [n, junk[0], junk[1], junk[2]]);
        let v: u16 = mem.read(at);
        #[allow(clippy::cast_possible_truncation)] // movzx eax, ax
        mem.write(at, v.wrapping_add(r as u16));
        cmd.wrapping_add(4)
    }

    /// `0x00810be0` (`opp_anmm_pair_set`): the command's two `u16`s (`+2`,
    /// `+4`) to `work +0x2c` with flag 0x1000, else to `work +0x1c`.
    /// Returns `cmd + 6`.
    pub fn pair_set(mem: &mut impl Mem, host: u32, cmd: u32, work: u32) -> u32 {
        let pair: [u16; 2] = mem.read(cmd.wrapping_add(2));
        let to = if flags(mem, host) & 0x1000 != 0 {
            0x2c
        } else {
            0x1c
        };
        mem.write(work.wrapping_add(to), pair);
        cmd.wrapping_add(6)
    }

    /// `0x00810c20` (`opp_anmm_mode_set`): `work +0x4e` = the flags' top
    /// nibble (`i16 +0x21a >> 12`, low byte); for mode 2 the three floats
    /// `+0x10` are copied to `+0x40` (through the FPU). Returns `cmd + 2`.
    pub fn mode_set(mem: &mut impl Mem, host: u32, cmd: u32, work: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the low byte
        let mode = (flags(mem, host).cast_signed() >> 12) as u8;
        mem.write(work.wrapping_add(0x4e), mode);
        if mode == 2 {
            for k in 0..3 {
                let v: u32 = mem.read(work.wrapping_add(0x10 + 4 * k));
                mem.write(work.wrapping_add(0x40 + 4 * k), crate::x87::load(v));
            }
        }
        cmd.wrapping_add(2)
    }

    /// `0x00810c60` (`opp_anmm_rgba_set`): each flagged byte of `work
    /// +0x1c..+0x1f` (flags 0x1000..0x8000) from the low byte of the next
    /// command word. Returns the address after the words read.
    pub fn rgba_set(mem: &mut impl Mem, host: u32, cmd: u32, work: u32) -> u32 {
        let mut p = cmd.wrapping_add(2);
        for (k, bit) in [0x1000u16, 0x2000, 0x4000, 0x8000].into_iter().enumerate() {
            if flags(mem, host) & bit != 0 {
                let b: u8 = mem.read(p);
                #[allow(clippy::cast_possible_truncation)] // 0..4
                mem.write(work.wrapping_add(0x1c + k as u32), b);
                p = p.wrapping_add(2);
            }
        }
        p
    }

    /// `0x00810cf0` (`opp_anmm_loop`): the loop count `work +0x4f` down by
    /// one; while not 0 back by the command's `i16 +2` (from `cmd`), else
    /// `cmd + 4`.
    pub fn loop_back(mem: &mut impl Mem, cmd: u32, work: u32) -> u32 {
        let c = mem.read::<u8>(work.wrapping_add(0x4f)).wrapping_sub(1);
        mem.write(work.wrapping_add(0x4f), c);
        if c == 0 {
            return cmd.wrapping_add(4);
        }
        let d = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        d.wrapping_add(cmd)
    }

    /// `0x00810d10` (`opp_anmm_branch_random`): when a random number in
    /// `[-0x100, 0x100)` (`op_random_per`, compared as `i16`) is below the
    /// command's `i16 +2`, on by its `u16 +4` (from `cmd`), else `cmd + 6`.
    pub fn branch_random(mem: &mut impl Mem, cmd: u32, junk: [u32; 3]) -> u32 {
        let limit: i16 = mem.read(cmd.wrapping_add(2));
        let r = rand::op_random_per(mem, [0x100, junk[0], junk[1], junk[2]]);
        #[allow(clippy::cast_possible_truncation)] // cmp ax, si
        let r = r as u16;
        if r.cast_signed() < limit {
            u32::from(mem.read::<u16>(cmd.wrapping_add(4))).wrapping_add(cmd)
        } else {
            cmd.wrapping_add(6)
        }
    }

    /// `0x00810d50` (`opp_anmm_fade`): at `work` + the command's `i16 +2`,
    /// the step byte `+0x2c` = the command's `+5`, and the host's wait
    /// `+0x210` = `|target - current|` (`u8 +4` and `u8 +0x1c`) over the
    /// step's magnitude (1 for 0; `idiv`, low byte). Returns `cmd + 6`.
    pub fn fade(mem: &mut impl Mem, host: u32, cmd: u32, work: u32) -> u32 {
        let off = i32::from(mem.read::<i16>(cmd.wrapping_add(2))).cast_unsigned();
        let at = work.wrapping_add(off);
        let target: u8 = mem.read(cmd.wrapping_add(4));
        let step: u8 = mem.read(cmd.wrapping_add(5));
        mem.write(at.wrapping_add(0x2c), step);
        let step = if step == 0 {
            1
        } else {
            step.cast_signed().unsigned_abs()
        };
        let cur: u8 = mem.read(at.wrapping_add(0x1c));
        let d = (i32::from(target) - i32::from(cur)).unsigned_abs();
        #[allow(clippy::cast_possible_truncation)] // mov [ecx+0x210], al
        mem.write(host.wrapping_add(0x210), (d / u32::from(step)) as u8);
        cmd.wrapping_add(6)
    }

    /// `0x00810da0` (`opp_anmm_part_float`): `work` + the command's byte
    /// `+2` = the float of the sprite's part (`u16 work +0x4c`, parts at
    /// `[set +0x20]`) at the command's byte `+3` (through the FPU). Returns
    /// `cmd + 4`.
    pub fn part_float(mem: &mut impl Mem, set: u32, cmd: u32, work: u32) -> u32 {
        let to = u32::from(mem.read::<u8>(cmd.wrapping_add(2)));
        let from = u32::from(mem.read::<u8>(cmd.wrapping_add(3)));
        let no = u32::from(mem.read::<u16>(work.wrapping_add(0x4c)));
        let parts: u32 = mem.read(set.wrapping_add(0x20));
        let v: u32 = mem.read(((no << 8) + from).wrapping_add(parts));
        mem.write(to.wrapping_add(work), crate::x87::load(v));
        cmd.wrapping_add(4)
    }
}
