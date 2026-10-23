//! `op_shape_draw(shape, cmd)` (`0x007ed0c0`, 31 KB): the shape draw of
//! the op layer, a translation of PS2 code that runs on the emulated EE
//! (`g_Ee_Gpr`, `0x00c8f508`) and VU0 (`0x00c8f788`..) registers in game
//! memory.
//!
//! The original is a head, six vertex loops and a tail
//! (`~/ffx-work/vfx/op_shape_draw-plan.md`). The head picks the loop by
//! `shape +0` bit 3, `cmd +2` bits `0xc00` and the texture `cmd +0x1c`
//! ([`bridge::Variant`]); every loop ends in the tail, which returns the
//! packet cursor or `cmd +0x14`.
//!
//! The pieces hand each other state through four words of the original's
//! frame (`ebp - 0x1bbc` .. `ebp - 0x1bb0`, [`bridge::Frame`]); the packet
//! goes to `cmd +0x14` (rounded up to 16 bytes) or, with `cmd +2` bit 2, to
//! a buffer in the frame itself (`ebp - 0x1804`). Every other frame word
//! is a temporary the loop writes before it reads it in the same vertex
//! (`fild`/`fstp` staging, copies of VU0 registers), kept in Rust locals
//! here.
//!
//! Frame: `0x1f3c` bytes (`_alloca_probe`), so a shim needs a
//! `StackFrame` of `0x7cf` words. Its one reference is the
//! `g_MagicImports` slot `0x00c650b0`, so magic overlays call it; on the
//! job workers (128 KB) that fits, as it does for the original.
//!
//! Ported so far: the head, the tail and the plain loop (S6,
//! [`bridge::plain`]); [`bridge::op_shape_draw`] returns `None` for the
//! loops still to port. Not hooked yet (`docs/porting.md`: the whole
//! function first).

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::x87::{Reg, ftol};
    use ffx_bridge::{Global, Mem};

    /// The original's frame size below `ebp` (`mov eax, 0x1f3c; call
    /// _alloca_probe`).
    pub const FRAME_SIZE: u32 = 0x1f3c;
    /// `cmd +2` bit 2: the packet goes to the frame buffer at `ebp -
    /// 0x1804` instead of `cmd +0x14`.
    const TO_FRAME: u8 = 4;

    /// `g_pppColorConShift`: added once to the vertex reads, then cleared.
    const SHIFT: Global<u32> = Global::new(0x00c8_b500);
    /// The EE's `r0` (zero) register, low and high words; the original's
    /// `beq rX, r0` tests compare 64 bits with them.
    const R0: Global<u32> = Global::new(0x00c8_f508);
    const R0_HI: Global<u32> = Global::new(0x00c8_f50c);
    /// `v0` (r2), four words.
    const V0: u32 = 0x00c8_f528;
    /// `v1` (r3), four words.
    const V1: u32 = 0x00c8_f538;
    /// `t1` (r9), low word: `r0` in the plain loop; a nonzero low half
    /// makes the tail return `cmd +0x14`.
    const T1: Global<u32> = Global::new(0x00c8_f598);
    /// `t4` (r12), four words: `y`, `z`, `w` of each point.
    const T4: u32 = 0x00c8_f5c8;
    /// `t6` (r14), four words: `y`, `z`, `w` of the texture pair.
    const T6: u32 = 0x00c8_f5e8;
    /// `t7` (r15), low half: the vertex count (`i16`), set by the head.
    const COUNT: Global<u16> = Global::new(0x00c8_f5f8);
    /// `s1` (r17) and `s2` (r18), low words: the draw flags.
    const S1: Global<u32> = Global::new(0x00c8_f618);
    const S2: Global<u32> = Global::new(0x00c8_f628);

    /// VU0's `Q`.
    const Q: Global<u32> = Global::new(0x00c8_f788);
    /// VU0's accumulator (four `f32`).
    const ACC: u32 = 0x00c8_f790;
    /// The two VU0 registers the loop clamps and sums: point or texture
    /// coordinate minus the origin.
    const VA: u32 = 0x00c8_f7a0;
    const VB: u32 = 0x00c8_f7d0;
    /// `op_vec3_max`'s and `op_vec3_min`'s results.
    const VMAX: u32 = 0x00c8_f7e0;
    const VMIN: u32 = 0x00c8_f7f0;
    /// The 48 bytes the head copies from `ORIGIN_SRC`; the first three words
    /// are subtracted from each projected point.
    const ORIGIN: u32 = 0x00c8_f850;
    const ORIGIN_SRC: u32 = 0x012e_95d8;
    /// The two points of a vertex, projected in place.
    const P0: u32 = 0x00c8_f8d0;
    const P1: u32 = 0x00c8_f900;
    /// The matrix the points go through (four rows of four `f32`).
    const MATRIX: u32 = 0x00c8_f950;
    /// `vf00.w` (`g_Vu0_Vf00 +0xc`), the numerator of `Q`.
    const VF00_W: Global<u32> = Global::new(0x00c0_a010);

    /// `1/16`, the point scale (`fld qword 0x00b921e0`).
    const POINT_SCALE: f64 = 0.0625;
    /// `1/4096`, the texture scale (`fld qword 0x00b50190`).
    const TEX_SCALE: f64 = 0.000_244_140_625;
    /// `16.0`, the fixed-point scale (`g_Vu0FixedScale`, `0x00b313a0`).
    const FIXED_SCALE: f64 = 16.0;

    /// The bytes per vertex record.
    const VERTEX: u32 = 0x18;

    /// The frame words the head, the loops and the tail share.
    #[derive(Debug, Clone, Copy)]
    pub struct Frame {
        /// The original's `ebp`.
        pub ebp: u32,
    }

    impl Frame {
        fn at(self, off: u32) -> u32 {
            self.ebp.wrapping_sub(off)
        }
        /// `ebp - 0x1bbc`: the packet cursor (the tail's result).
        pub fn cursor(self, mem: &impl Mem) -> u32 {
            mem.read(self.at(0x1bbc))
        }
        fn set_cursor(self, mem: &mut impl Mem, v: u32) {
            mem.write(self.at(0x1bbc), v);
        }
        /// `ebp - 0x1bb8`: `cmd`.
        pub fn cmd(self, mem: &impl Mem) -> u32 {
            mem.read(self.at(0x1bb8))
        }
        /// `ebp - 0x1bb4`: `shape` after the head; the loops reuse it as a
        /// temporary.
        pub fn shape(self, mem: &impl Mem) -> u32 {
            mem.read(self.at(0x1bb4))
        }
        /// `ebp - 0x1bb0`: the vertex record pointer.
        pub fn vertex(self, mem: &impl Mem) -> u32 {
            mem.read(self.at(0x1bb0))
        }
        fn set_vertex(self, mem: &mut impl Mem, v: u32) {
            mem.write(self.at(0x1bb0), v);
        }
        /// `ebp - 0x1804`: the packet buffer used with `cmd +2` bit 2.
        #[must_use]
        pub fn buffer(self) -> u32 {
            self.at(0x1804)
        }
    }

    /// The six vertex loops, by the head's selection.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Variant {
        /// S1 (`0x007ed1bc`): `shape` bit 3, `cmd` bits `0xc00`.
        Vu0Clip,
        /// S2 (`0x007eed39`): bit 3, a texture.
        Vu0Textured,
        /// S3 (`0x007f0a18`): bit 3 alone.
        Vu0Plain,
        /// S4 (`0x007f1a60`): bits `0xc00`.
        Clip,
        /// S5 (`0x007f2d94`): a texture.
        Textured,
        /// S6 (`0x007f3f03`): neither ([`plain`]).
        Plain,
    }

    fn rd(mem: &impl Mem, sym: u32) -> u32 {
        mem.read(mem.rebase(sym))
    }

    fn wr(mem: &mut impl Mem, sym: u32, v: u32) {
        let at = mem.rebase(sym);
        mem.write(at, v);
    }

    /// `mov [reg], ax`: a register's low half alone, the rest kept (the
    /// translation's `lhu`).
    fn wr16(mem: &mut impl Mem, sym: u32, v: u16) {
        let at = mem.rebase(sym);
        mem.write(at, v);
    }

    fn rd16(mem: &impl Mem, at: u32) -> u16 {
        mem.read(at)
    }

    fn f(bits: u32) -> Reg {
        Reg::load(f32::from_bits(bits))
    }

    /// `fild dword; fstp dword; fld dword`: an integer word as `f32`.
    fn int_f32(bits: u32) -> Reg {
        f(Reg::constant(f64::from(bits.cast_signed()))
            .store()
            .to_bits())
    }

    fn st(r: Reg) -> u32 {
        r.store().to_bits()
    }

    /// The `beq rX, r0` the translation tests: `lo` and `hi` against `r0`.
    fn is_r0(mem: &impl Mem, lo: u32, hi: u32) -> bool {
        lo == R0.get(mem) && hi == R0_HI.get(mem)
    }

    /// The head (`0x007ed0c0`..`0x007ed1bb` and the count stores at each
    /// loop's start): the frame words, the origin block, the vertex count
    /// (`i16` at `shape + g_pppColorConShift + 2`, the shift cleared) and
    /// the loop to run.
    pub fn head(mem: &mut impl Mem, shape: u32, cmd: u32, frame: Frame) -> Variant {
        mem.write(frame.at(0x1bb4), shape);
        mem.write(frame.at(0x1bb8), cmd);
        let mut cursor = if mem.read::<u8>(cmd.wrapping_add(2)) & TO_FRAME == 0 {
            mem.read::<u32>(cmd.wrapping_add(0x14)).wrapping_add(0xf) & !0xf
        } else {
            frame.buffer()
        };
        let src = mem.rebase(ORIGIN_SRC);
        let origin: [u32; 12] = mem.read(src);
        let at = mem.rebase(ORIGIN);
        mem.write(at, origin);
        let flags = mem.read::<u16>(cmd.wrapping_add(2));
        cursor = cursor.wrapping_add(0x40);
        frame.set_vertex(mem, shape.wrapping_add(0x20));
        frame.set_cursor(mem, cursor);
        if flags & 0x18 != 0 {
            frame.set_cursor(mem, cursor.wrapping_add(0x20));
        }
        let vu0 = mem.read::<u8>(shape) & 8 != 0;
        let clip = flags & 0xc00 != 0;
        let count_at = SHIFT.get(mem).wrapping_add(shape).wrapping_add(2);
        let count = rd16(mem, count_at);
        let textured = || mem.read::<u32>(cmd.wrapping_add(0x1c)) != 0;
        let variant = match (vu0, clip) {
            (true, true) => Variant::Vu0Clip,
            (true, false) if textured() => Variant::Vu0Textured,
            (true, false) => Variant::Vu0Plain,
            (false, true) => Variant::Clip,
            (false, false) if textured() => Variant::Textured,
            (false, false) => Variant::Plain,
        };
        COUNT.set(mem, count);
        SHIFT.set(mem, 0);
        variant
    }

    /// The tail (`0x007f499c`): `cmd +0x14` when `t1`'s low half is set and
    /// `cmd +2` has bit 2 (the packet went to the frame buffer), else the
    /// cursor.
    pub fn tail(mem: &impl Mem, frame: Frame) -> u32 {
        let cmd = frame.cmd(mem);
        if T1.get(mem) & 0xffff != 0
            && mem.read::<u16>(cmd.wrapping_add(2)) & u16::from(TO_FRAME) != 0
        {
            mem.read(cmd.wrapping_add(0x14))
        } else {
            frame.cursor(mem)
        }
    }

    /// `op_shape_draw(shape, cmd)` with its frame at `ebp`: the head, the
    /// selected loop, the tail's result. `None` for a loop not ported yet.
    pub fn op_shape_draw(mem: &mut impl Mem, shape: u32, cmd: u32, ebp: u32) -> Option<u32> {
        let frame = Frame { ebp };
        match head(mem, shape, cmd, frame) {
            Variant::Plain => plain(mem, frame, cmd, shape.wrapping_add(0x20)),
            _ => return None,
        }
        Some(tail(mem, frame))
    }

    /// One point through `MATRIX` with the accumulator (`vmulax`,
    /// `vmadday`, `vmaddaz`, `vmaddw`), then `Q = vf00.w / w`. Each step
    /// rounds to `f32` where the original stores. `rows_in_memory`: the
    /// x and y rows' matrix elements are the memory operands (the second
    /// point's code) instead of loaded first; it only tells a signalling
    /// NaN element from a quiet one.
    fn transform<M: Mem>(mem: &mut M, point: u32, rows_in_memory: bool) {
        let element = |mem: &M, row: u32, c: u32| rd(mem, MATRIX + 0x10 * row + 4 * c);
        let acc = |c: u32| ACC + 4 * c;
        let lane = |c: u32| point + 4 * c;
        for row in 0..3u32 {
            let s = rd(mem, lane(row));
            for c in 0..4u32 {
                let e = element(mem, row, c);
                let product = if c == 3 || rows_in_memory && row < 2 {
                    f(s).mul_m32(f32::from_bits(e))
                } else {
                    f(e) * f(s)
                };
                let v = if row == 0 {
                    product
                } else {
                    product + f(rd(mem, acc(c)))
                };
                wr(mem, acc(c), st(v));
            }
        }
        let w = rd(mem, lane(3));
        for c in 0..4u32 {
            let e = element(mem, 3, c);
            let product = match c {
                0 => f(e).mul_m32(f32::from_bits(w)),
                3 => f(w).mul_m32(f32::from_bits(e)),
                _ => f(e) * f(w),
            };
            let v = product + f(rd(mem, acc(c)));
            wr(mem, lane(c), st(v));
        }
        let q = f(VF00_W.get(mem)).div_m32(f32::from_bits(rd(mem, lane(3))));
        Q.set(mem, st(q));
    }

    /// `x, y, z *= Q` (`vmulq.xyz`; `z` is the memory operand).
    fn scale_by_q(mem: &mut impl Mem, point: u32) {
        let q = f(Q.get(mem));
        for c in 0..3u32 {
            let at = point + 4 * c;
            let x = rd(mem, at);
            let v = if c == 2 {
                q.mul_m32(f32::from_bits(x))
            } else {
                f(x) * q
            };
            wr(mem, at, st(v));
        }
    }

    /// `dst.xyz = point.xyz - origin` (`vsub.xyz`), then `restoreVf00Register`.
    fn minus_origin(mem: &mut impl Mem, dst: u32, point: u32) {
        for c in 0..3u32 {
            let v = f(rd(mem, point + 4 * c)).sub_m32(f32::from_bits(rd(mem, ORIGIN + 4 * c)));
            wr(mem, dst + 4 * c, st(v));
        }
        ffx_ps2::ee::bridge::restore_vf00(mem);
    }

    /// `vftoi4`: the four words times 16, truncated (`_ftol2_sse`).
    fn to_fixed(mem: &mut impl Mem, point: u32) {
        let v: [u32; 4] = mem.read(mem.rebase(point));
        for (c, x) in (0u32..).zip(v) {
            let n = ftol(mem, (f(x) * Reg::constant(FIXED_SCALE)).value());
            wr(mem, point + 4 * c, n);
        }
    }

    /// A point into `dst`: `x` is `v0`'s low word, `y`, `z`, `w` are `t4`'s
    /// (`t4`'s first word and `v0`'s third, `hi`, are set as the
    /// translation does). `x` and `y` become `f32` / 16, `z` and `w` stay
    /// as they are.
    #[allow(clippy::many_single_char_names)] // the lanes
    fn load_point(mem: &mut impl Mem, dst: u32, hi: u32) {
        let x = rd(mem, V0);
        let [_, y, z, w]: [u32; 4] = mem.read(mem.rebase(T4));
        wr(mem, dst + 8, z);
        wr(mem, V0 + 8, hi);
        wr(mem, T4, x);
        wr(mem, dst + 0xc, w);
        let k = Reg::constant(POINT_SCALE);
        wr(mem, dst, st(int_f32(x) * k));
        wr(mem, dst + 4, st(int_f32(y) * k));
    }

    /// The texture pair's coordinate into `dst` from `v0`'s low word and
    /// `t6`'s `y`, `z`, `w`: `x`, `y` as `f32` / 4096, `z` and `w` as they
    /// are; `t6`'s first word = `x`.
    #[allow(clippy::many_single_char_names)] // the lanes
    fn load_tex(mem: &mut impl Mem, dst: u32) {
        let x = rd(mem, V0);
        let [_, y, z, w]: [u32; 4] = mem.read(mem.rebase(T6));
        wr(mem, T6, x);
        mem.write(mem.rebase(dst + 8), [z, w]);
        let k = Reg::constant(TEX_SCALE);
        wr(mem, dst, st(int_f32(x) * k));
        wr(mem, dst + 4, st(int_f32(y) * k));
    }

    /// S6, the plain loop (`0x007f3f03`..`0x007f499b`), entered with `cmd`
    /// (`esi`) and the first vertex record (`edi`, `shape +0x20`). Per
    /// record (`0x18` bytes, `i16` count in `t7`, at least one pass):
    /// - the first point (`+0`, `+2`) through the matrix;
    /// - unless `u16 +0xc` is set (`bne v0, r0`): the second point (`+0xc`,
    ///   `+0xe`) too, both scaled by `Q`, each minus the origin
    ///   (`VA`, `VB`), clamped (`op_vec3_max`/`min` of `VB`'s words on
    ///   `VA`);
    /// - with no `s2 & 0x28514a` flag: both points to fixed point; with no
    ///   `s2 & 0x810` flag the texture pair (`+0x10`/`+0x12`, `+0x14`/
    ///   `+0x16`) into `VA` and `VB`, `VB.xy += VA.xy`.
    ///
    /// Nothing is written to the packet: only the EE and VU0 registers
    /// change. `v0`'s second word carries from one record to the next
    /// (`ebx`). The translation clears `g_pppColorConShift` after each
    /// record read and adds it to some; the loop's start clears it and
    /// nothing in the loop writes it again, so those change nothing and are
    /// left out.
    pub fn plain(mem: &mut impl Mem, frame: Frame, cmd: u32, first: u32) {
        let r0 = R0.get(mem);
        T1.set(mem, r0);
        let mut hi = rd(mem, V0 + 4);
        wr(mem, V0, mem.read(cmd.wrapping_add(4)));
        SHIFT.set(mem, 0);
        let mut p = first;
        loop {
            hi = plain_vertex(mem, p, hi);
            p = p.wrapping_add(VERTEX);
            let count = COUNT.get(mem).wrapping_sub(1);
            COUNT.set(mem, count);
            frame.set_vertex(mem, p);
            if count.cast_signed() <= 0 {
                break;
            }
        }
    }

    /// One record of [`plain`]; returns `v0`'s second word for the next.
    fn plain_vertex<M: Mem>(mem: &mut M, p: u32, hi: u32) -> u32 {
        let half = |mem: &M, off: u32| rd16(mem, p.wrapping_add(off));
        // x, y into v0 / v1 (lhu): low halves only
        wr16(mem, V0, half(mem, 0));
        wr16(mem, V1, half(mem, 2));
        let v1_hi = rd(mem, V1 + 4);
        wr(mem, V0 + 0xc, v1_hi);
        load_point(mem, P0, hi);
        transform(mem, P0, false);
        // and s1, 0x11 into v0 (64 bits): v0's second word is now 0
        let flags = S1.get(mem) & 0x11;
        wr(mem, V0, flags);
        wr(mem, V0 + 4, 0);
        let hi = 0;
        wr16(mem, V0, half(mem, 0xc));
        let second = rd(mem, V0);
        if !is_r0(mem, second, hi) {
            return hi;
        }
        // (the translation loads `+0xe` into v1 and copies v1 into v0 here:
        // the flag test below overwrites both before anything reads them)
        load_point(mem, P1, hi);
        scale_by_q(mem, P0);
        transform(mem, P1, true);
        scale_by_q(mem, P1);
        minus_origin(mem, VA, P0);
        minus_origin(mem, VB, P1);
        let b: [u32; 4] = mem.read(mem.rebase(VB));
        crate::op::bridge::op_vec3_clamp(mem, mem.rebase(VMAX), mem.rebase(VA), b, true);
        let b: [u32; 4] = mem.read(mem.rebase(VB));
        crate::op::bridge::op_vec3_clamp(mem, mem.rebase(VMIN), mem.rebase(VA), b, false);
        // and v1, s2, 0x28514a; v0 = 0x810: beq v1, r0. r0's high word
        // was 0 at the test above and nothing writes it: only the low words
        // are compared from here on
        let s2 = S2.get(mem);
        let r0 = R0.get(mem);
        let m = s2 & 0x0028_514a;
        mem.write(mem.rebase(V1), [m, 0]);
        wr(mem, V0, 0x810); // (its second word is 0 already)
        if m != r0 {
            return 0;
        }
        to_fixed(mem, P0);
        to_fixed(mem, P1);
        let m = s2 & 0x810;
        mem.write(mem.rebase(V1), [m, 0]);
        if m != r0 {
            return 0;
        }
        // the texture pair: (+0x10, +0x12) into VA, (+0x14, +0x16) into VB;
        // v0 then holds the second v1 in its second word, the first in its
        // third, v1's second word (0 since the flag test) in its fourth
        wr16(mem, V0, half(mem, 0x10));
        wr16(mem, V1, half(mem, 0x12));
        let first = rd(mem, V1);
        wr(mem, V0 + 0xc, 0);
        load_tex(mem, VA);
        wr16(mem, V0, half(mem, 0x14));
        wr16(mem, V1, half(mem, 0x16));
        let second = rd(mem, V1);
        wr(mem, V0 + 4, second);
        wr(mem, V0 + 8, first);
        load_tex(mem, VB);
        let vb_y = rd(mem, VB + 4);
        let sum_x = f(rd(mem, VA)).add_m32(f32::from_bits(rd(mem, VB)));
        wr(mem, VB, st(sum_x));
        let sum_y = f(rd(mem, VA + 4)).add_m32(f32::from_bits(vb_y));
        wr(mem, VB + 4, st(sum_y));
        second
    }
}
