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
//! The loops: plain (S6, [`bridge::plain`]), VU0 plain (S3,
//! [`bridge::vu0_plain`]), textured (S5, [`bridge::textured`]; S2,
//! [`bridge::vu0_textured`]) and fog (S4, [`bridge::fog`]; S1,
//! [`bridge::vu0_fog`]).

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::x87::Reg;
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
    const R0_AT: u32 = 0x00c8_f508;
    const R0: Global<u32> = Global::new(R0_AT);
    const R0_HI: Global<u32> = Global::new(0x00c8_f50c);
    /// `v0` (r2), four words.
    const V0: u32 = 0x00c8_f528;
    /// `v1` (r3), four words.
    const V1: u32 = 0x00c8_f538;
    /// `t1` (r9), low word: `r0` in the plain loop; a nonzero low half
    /// makes the tail return `cmd +0x14`.
    const T1: Global<u32> = Global::new(0x00c8_f598);
    /// `t2` (r10), `t3` (r11) and `t5` (r13), four words each: set by S5's
    /// start, read by nothing here.
    const T2: u32 = 0x00c8_f5a8;
    const T3: u32 = 0x00c8_f5b8;
    const T5: u32 = 0x00c8_f5d8;
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
    /// A record's points, projected in place (S6 uses the first and the
    /// last).
    const P0: u32 = 0x00c8_f8d0;
    const P1: u32 = 0x00c8_f8e0;
    const P2: u32 = 0x00c8_f8f0;
    const P3: u32 = 0x00c8_f900;
    /// The matrix the points go through (four rows of four `f32`).
    const MATRIX: u32 = 0x00c8_f950;
    /// S5's second matrix, the texture's (`cmd +0x1c`, or the first when
    /// that equals `r0`).
    const MATRIX2: u32 = 0x00c8_f810;
    /// S5's colour: `cmd +4`'s bytes unpacked to words, times `1/128`.
    const COLOUR: u32 = 0x00c8_f880;
    /// S5's and S2's texture points: each point again, through `MATRIX2`
    /// (S2's last through `MATRIX`).
    const P4: u32 = 0x00c8_f910;
    const P5: u32 = 0x00c8_f920;
    const P6: u32 = 0x00c8_f930;
    const P7: u32 = 0x00c8_f940;
    /// S4's and S1's fog vector (`x` the depth scale, `y` the lowest
    /// value, `w` the highest): `g_OpFog +0x10` (`0x011a0020`), or `+0x30`
    /// when `cmd +2` bit `0x800` equals `r0` (is clear).
    const FOG: u32 = 0x00c8_f800;
    /// S4's and S1's second fog vector: `g_OpFog +0x20` (`0x011a0030`), or
    /// `+0x40` when `cmd +2` bit `0x400` equals `r0`. Nothing here reads it.
    const FOG2: u32 = 0x00c8_f8c0;
    /// S4's and S1's copy of the first point after its projection, before
    /// `Q` scales it.
    const P0_COPY: u32 = 0x00c8_f8b0;
    /// `g_OpFog +0x10`, the first of the four fog vectors.
    const OP_FOG_VECTORS: u32 = 0x011a_0020;
    /// `vf00.w` (`g_Vu0_Vf00 +0xc`), the numerator of `Q`.
    const VF00_W: Global<u32> = Global::new(0x00c0_a010);

    /// `1/16`, the point scale (`fld qword 0x00b921e0`).
    const POINT_SCALE: f64 = 0.0625;
    /// `1/4096`, the texture scale (`fld qword 0x00b50190`).
    const TEX_SCALE: f64 = 0.000_244_140_625;

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
        /// S1 (`0x007ed1bc`): `shape` bit 3, `cmd` bits `0xc00` ([`vu0_fog`]).
        Vu0Fog,
        /// S2 (`0x007eed39`): bit 3, a texture.
        Vu0Textured,
        /// S3 (`0x007f0a18`): bit 3 alone.
        Vu0Plain,
        /// S4 (`0x007f1a60`): bits `0xc00` ([`fog`]).
        Fog,
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
            (true, true) => Variant::Vu0Fog,
            (true, false) if textured() => Variant::Vu0Textured,
            (true, false) => Variant::Vu0Plain,
            (false, true) => Variant::Fog,
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
    /// selected loop, the tail's result.
    pub fn op_shape_draw(mem: &mut impl Mem, shape: u32, cmd: u32, ebp: u32) -> u32 {
        let frame = Frame { ebp };
        match head(mem, shape, cmd, frame) {
            Variant::Plain => plain(mem, frame, cmd, shape.wrapping_add(0x20)),
            Variant::Vu0Plain => vu0_plain(mem, cmd, shape.wrapping_add(0x20)),
            Variant::Textured => textured(mem, frame, cmd, shape.wrapping_add(0x20)),
            Variant::Vu0Textured => vu0_textured(mem, frame, cmd, shape.wrapping_add(0x20)),
            Variant::Fog => fog(mem, frame, cmd, shape.wrapping_add(0x20)),
            Variant::Vu0Fog => vu0_fog(mem, frame, cmd, shape.wrapping_add(0x20)),
        }
        tail(mem, frame)
    }

    /// One point through `matrix` with the accumulator (`vmulax`,
    /// `vmadday`, `vmaddaz`, `vmaddw`), then `Q = vf00.w / w`. Each step
    /// rounds to `f32` where the original stores. `memory_rows[r]`: row
    /// `r`'s matrix elements are the memory operands instead of loaded
    /// first (lane `w` always is); `memory_w`: so are the last row's `y`
    /// and `z`. The forms differ between the points' code and only tell a
    /// signalling NaN element from a quiet one.
    fn transform(
        mem: &mut impl Mem,
        matrix: u32,
        point: u32,
        memory_rows: [bool; 3],
        memory_w: bool,
    ) {
        for (row, memory) in (0..3u32).zip(memory_rows) {
            madda(mem, matrix, point, row, memory);
        }
        maddw(mem, matrix, point, form(memory_w));
        set_q(mem, point);
    }

    /// Row `row` (0..3) of `matrix` times the point's lane `row` into the
    /// accumulator (row 0 replaces it).
    fn madda(mem: &mut impl Mem, matrix: u32, point: u32, row: u32, memory: bool) {
        let s = rd(mem, point + 4 * row);
        for c in 0..4u32 {
            let e = rd(mem, matrix + 0x10 * row + 4 * c);
            let product = if c == 3 || memory {
                f(s).mul_m32(f32::from_bits(e))
            } else {
                f(e) * f(s)
            };
            let v = if row == 0 {
                product
            } else {
                product + f(rd(mem, ACC + 4 * c))
            };
            wr(mem, ACC + 4 * c, st(v));
        }
    }

    /// The operand forms of a last row (`vmaddw`): they differ only for a
    /// signalling NaN `w` or matrix element.
    #[derive(Clone, Copy)]
    enum W {
        /// `x` with `w` the memory operand, `y` and `z` with the element.
        Memory,
        /// `x` with `w` the memory operand, `y` and `z` both loaded.
        Loaded,
        /// `pppVu0MaddW_copy` (`0x0074bde0`): `x`, `y`, `z` both loaded.
        Call,
    }

    /// [`W::Memory`] or [`W::Loaded`].
    fn form(memory_w: bool) -> W {
        if memory_w { W::Memory } else { W::Loaded }
    }

    /// The last row times `w` plus the accumulator into the point (`w`
    /// lane: the element the memory operand).
    fn maddw(mem: &mut impl Mem, matrix: u32, point: u32, form: W) {
        let w = rd(mem, point + 0xc);
        for c in 0..4u32 {
            let e = rd(mem, matrix + 0x30 + 4 * c);
            let product = match (c, form) {
                (3, _) | (1..=2, W::Memory) => f(w).mul_m32(f32::from_bits(e)),
                (0, W::Memory | W::Loaded) => f(e).mul_m32(f32::from_bits(w)),
                _ => f(e) * f(w),
            };
            let v = product + f(rd(mem, ACC + 4 * c));
            wr(mem, point + 4 * c, st(v));
        }
    }

    /// `Q = vf00.w / w` (`vdiv`).
    fn set_q(mem: &mut impl Mem, point: u32) {
        let q = f(VF00_W.get(mem)).div_m32(f32::from_bits(rd(mem, point + 0xc)));
        Q.set(mem, st(q));
    }

    /// The first point's forms: every row element loaded.
    const LOADED: [bool; 3] = [false; 3];

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

    /// `vftoi4` in place: the four words times 16, truncated
    /// (`rcVec4fToFixed`, which S3 calls and S6 has inline).
    fn to_fixed(mem: &mut impl Mem, point: u32) {
        let at = mem.rebase(point);
        ffx_field::model::bridge::rc_vec4f_to_fixed(mem, at, at);
    }

    /// A point into `dst`: `x` is `v0`'s low word, `y`, `z`, `w` are `t4`'s
    /// (`t4`'s first word and `v0`'s third, `hi`, are set as the
    /// translation does). `x` and `y` become `f32` / 16, `z` and `w` stay
    /// as they are.
    fn load_point(mem: &mut impl Mem, dst: u32, hi: u32) {
        wr(mem, V0 + 8, hi);
        load_again(mem, dst);
    }

    /// [`load_point`] without the `v0` store: S5's and S2's texture
    /// points, loaded from the same registers right after their point.
    #[allow(clippy::many_single_char_names)] // the lanes
    fn load_again(mem: &mut impl Mem, dst: u32) {
        let x = rd(mem, V0);
        let [_, y, z, w]: [u32; 4] = mem.read(mem.rebase(T4));
        wr(mem, dst + 8, z);
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
        let mut hi = loop_start(mem, cmd);
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

    /// Both loops' start: `t1` = `r0`'s low word, `v0`'s low word =
    /// `cmd +4`, the shift cleared; returns `v0`'s second word (`ebx`).
    fn loop_start(mem: &mut impl Mem, cmd: u32) -> u32 {
        let r0 = R0.get(mem);
        T1.set(mem, r0);
        let hi = rd(mem, V0 + 4);
        wr(mem, V0, mem.read(cmd.wrapping_add(4)));
        SHIFT.set(mem, 0);
        hi
    }

    /// A record's first point (`+0`, `+2`) into `P0` through the matrix,
    /// then `s1 & 0x11` into `v0` (64 bits, the second word 0).
    fn first_point(mem: &mut impl Mem, p: u32, hi: u32) {
        // x, y into v0 / v1 (lhu): low halves only
        wr16(mem, V0, rd16(mem, p));
        wr16(mem, V1, rd16(mem, p.wrapping_add(2)));
        let v1_hi = rd(mem, V1 + 4);
        wr(mem, V0 + 0xc, v1_hi);
        load_point(mem, P0, hi);
        transform(mem, MATRIX, P0, LOADED, false);
        set_flags(mem);
    }

    /// `and v0, s1, 0x11`: `v0` = the flags (64 bits); returns them.
    fn set_flags(mem: &mut impl Mem) -> u32 {
        let flags = S1.get(mem) & 0x11;
        wr(mem, V0, flags);
        wr(mem, V0 + 4, 0);
        flags
    }

    /// One record of [`plain`]; returns `v0`'s second word for the next.
    fn plain_vertex<M: Mem>(mem: &mut M, p: u32, hi: u32) -> u32 {
        let half = |mem: &M, off: u32| rd16(mem, p.wrapping_add(off));
        first_point(mem, p, hi);
        let hi = 0;
        wr16(mem, V0, half(mem, 0xc));
        let second = rd(mem, V0);
        if !is_r0(mem, second, hi) {
            return hi;
        }
        // (the translation loads `+0xe` into v1 and copies v1 into v0 here:
        // the flag test below overwrites both before anything reads them)
        load_point(mem, P3, hi);
        scale_by_q(mem, P0);
        transform(mem, MATRIX, P3, [true, true, false], false);
        scale_by_q(mem, P3);
        minus_origin(mem, VA, P0);
        minus_origin(mem, VB, P3);
        let b: [u32; 4] = mem.read(mem.rebase(VB));
        crate::op::bridge::op_vec3_clamp(mem, mem.rebase(VMAX), mem.rebase(VA), b, true);
        let b: [u32; 4] = mem.read(mem.rebase(VB));
        crate::op::bridge::op_vec3_clamp(mem, mem.rebase(VMIN), mem.rebase(VA), b, false);
        if !fixed_point(mem, &[P0, P3]) {
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

    /// Both loops' flag tests after the points: `and v1, s2, 0x28514a`,
    /// `v0 = 0x810`, `beq v1, r0`: with none of those flags the points to
    /// fixed point (`rcVec4fToFixed`, inline in S6), then `and v1, s2, v0`
    /// (`0x810`), `beq v1, r0`. Whether both tests passed. `r0`'s high
    /// word was 0 at the loops' earlier tests and nothing writes it, and the
    /// high words of `v0` and `v1` are 0 here, so only low words count.
    fn fixed_point(mem: &mut impl Mem, points: &[u32]) -> bool {
        let r0 = R0.get(mem);
        let m = S2.get(mem) & 0x0028_514a;
        mem.write(mem.rebase(V1), [m, 0]);
        wr(mem, V0, 0x810);
        if m != r0 {
            return false;
        }
        for &point in points {
            to_fixed(mem, point);
        }
        let m = S2.get(mem) & 0x810;
        wr(mem, V1, m);
        m == r0
    }

    /// The vertices' bounding box corners and `VB.xy`: `op_vec3_max` and
    /// `op_vec3_min` of `VA` with `VB`'s words, then of the results with
    /// `VC`'s and `VD`'s.
    fn clamp_quad(mem: &mut impl Mem) {
        use crate::op::bridge::op_vec3_clamp;
        let (max, min) = (mem.rebase(VMAX), mem.rebase(VMIN));
        let va = mem.rebase(VA);
        for (k, src) in [VA + 0x10, VA + 0x20, VB].into_iter().enumerate() {
            let words: [u32; 4] = mem.read(mem.rebase(src));
            let (v_max, v_min) = if k == 0 { (va, va) } else { (max, min) };
            op_vec3_clamp(mem, max, v_max, words, true);
            let words: [u32; 4] = mem.read(mem.rebase(src));
            op_vec3_clamp(mem, min, v_min, words, false);
        }
    }

    /// S3, the VU0 plain loop (`0x007f0a18`..`0x007f1a5f`), entered as
    /// [`plain`]. Per record (`0x18` bytes): up to four points (`+0`/`+2`,
    /// `+4`/`+6`, `+8`/`+0xa`, `+0xc`/`+0xe`), each next one only while
    /// the half before it equals `r0` (`bne v0, r0`, a 0 flag), each
    /// through the matrix and the one before it scaled by `Q`; then, with
    /// `s1 & 0x11` clear, the last scaled too, the four minus the origin
    /// (`VA`..`VD`, `0x00c8f7a0`.. step 16) and their box (`clamp_quad`);
    /// with no `s2 & 0x28514a` flag all four to fixed point
    /// (`rcVec4fToFixed`); with no `s2 & 0x810` flag the texture pairs
    /// (`+0x10`/`+0x12`, `+0x14`/`+0x16`, `pextlw` with `t6`'s `yzw`,
    /// times 1/4096) into `VA` and `VD` and the quad's four corners
    /// (`VA`, `VA` with `x` + `VD.x`, with `y` + `VD.y`, both) in
    /// `VA`..`VD`. The vertex pointer stays in a register: the frame's
    /// vertex word keeps the head's value.
    ///
    /// The records' `y` halves only reach `v1`: every point takes `y`, `z`
    /// and `w` from `t4`, and the later points' `x` is the flag half, equal
    /// to `r0`'s low word to get there. So points 1..3 are always projected
    /// from the same values and `VB`, `VC`, `VD` coincide.
    pub fn vu0_plain(mem: &mut impl Mem, cmd: u32, first: u32) {
        let mut hi = loop_start(mem, cmd);
        let mut p = first;
        loop {
            hi = vu0_plain_vertex(mem, p, hi);
            p = p.wrapping_add(VERTEX);
            let count = COUNT.get(mem).wrapping_sub(1);
            COUNT.set(mem, count);
            if count.cast_signed() <= 0 {
                break;
            }
        }
    }

    /// One record of [`vu0_plain`]; returns `v0`'s second word for the next.
    fn vu0_plain_vertex<M: Mem>(mem: &mut M, p: u32, hi: u32) -> u32 {
        let half = |mem: &M, off: u32| rd16(mem, p.wrapping_add(off));
        first_point(mem, p, hi);
        // the middle points: y (+6, +0xa) into v1's low half, unread but
        // left there when a later test ends the record
        for (flag, point, before) in [(4, P1, P0), (8, P2, P1)] {
            wr16(mem, V0, half(mem, flag));
            if !is_r0(mem, rd(mem, V0), 0) {
                return 0;
            }
            wr16(mem, V1, half(mem, flag + 2));
            load_point(mem, point, 0);
            scale_by_q(mem, before);
            transform(mem, MATRIX, point, [true, true, false], true);
            // (the translation sets v0 to the flags again here: the next
            // flag half replaces its low half, the rest is equal)
        }
        // the last point: its y and z rows through pppVu0MaddaY_copy /
        // pppVu0MaddaZ_copy (the loaded form); the copy of v1 into v0's
        // second word is overwritten by the flags
        wr16(mem, V0, half(mem, 0xc));
        if !is_r0(mem, rd(mem, V0), 0) {
            return 0;
        }
        wr16(mem, V1, half(mem, 0xe));
        load_point(mem, P3, 0);
        scale_by_q(mem, P2);
        transform(mem, MATRIX, P3, [true, false, false], true);
        let flags = set_flags(mem);
        if !is_r0(mem, flags, 0) {
            return 0;
        }
        scale_by_q(mem, P3);
        quad(mem, p)
    }

    /// The end of an S3 or S2 record (the same code in both): the four
    /// points minus the origin (`VA`..`VD`, `0x00c8f7a0`.. step 16) and
    /// their box (`clamp_quad`); with no `s2 & 0x28514a` flag all four to
    /// fixed point (`rcVec4fToFixed`); with no `s2 & 0x810` flag the
    /// texture pairs (`+0x10`/`+0x12`, `+0x14`/`+0x16`, `pextlw` with
    /// `t6`'s `yzw`, times 1/4096) into `VA` and `VD` and the quad's four
    /// corners. Returns `v0`'s second word.
    fn quad<M: Mem>(mem: &mut M, p: u32) -> u32 {
        let half = |mem: &M, off: u32| rd16(mem, p.wrapping_add(off));
        for (dst, point) in [(VA, P0), (VA + 0x10, P1), (VA + 0x20, P2), (VB, P3)] {
            minus_origin(mem, dst, point);
        }
        clamp_quad(mem);
        if !fixed_point(mem, &[P0, P1, P2, P3]) {
            return 0;
        }
        for (off, dst) in [(0x10, VA), (0x14, VB)] {
            wr16(mem, V0, half(mem, off));
            wr16(mem, V1, half(mem, off + 2));
            // CopyFields_007ecb20(v0, v1, v0): pextlw v0, v1, v0
            let [a0, a1, _, _]: [u32; 4] = mem.read(mem.rebase(V0));
            let [b0, b1, _, _]: [u32; 4] = mem.read(mem.rebase(V1));
            mem.write(mem.rebase(V0), [a0, b0, a1, b1]);
            let [_, y, z, w]: [u32; 4] = mem.read(mem.rebase(T6));
            wr(mem, T6, a0);
            let at = mem.rebase(dst);
            mem.write(at, [a0, y, z, w]);
            crate::op::bridge::op_ivec2_scale(mem, at, 0x00b5_0190);
        }
        let next = rd(mem, V0 + 4);
        // the quad: VA, VA + (VD.x, 0), VA + (0, VD.y), VA + VD.xy
        // (`fst` copies: a signalling NaN comes out quiet)
        let copy = |bits: u32| st(f(bits));
        let [ax, ay, az, aw]: [u32; 4] = mem.read(mem.rebase(VA));
        let (dx, dy) = (rd(mem, VB), rd(mem, VB + 4));
        let x = st(f(ax).add_m32(f32::from_bits(dx)));
        let y = st(f(ay).add_m32(f32::from_bits(dy)));
        mem.write(mem.rebase(VA + 0x10), [x, copy(ay), copy(az), copy(aw)]);
        mem.write(mem.rebase(VA + 0x20), [copy(ax), y, copy(az), copy(aw)]);
        mem.write(mem.rebase(VB), [x, y]);
        next
    }

    /// S5, the textured loop (`0x007f2dae`..`0x007f3f02`), entered as
    /// [`plain`]. Its start ([`textured_start`]) sets the colour, both
    /// matrices and `t2`..`t6`; then per record (`0x18` bytes):
    /// - the first point (`+0`, `+2`; `y`, `z`, `w` from `t4`) into `P0`
    ///   and `P4`, `P0` through `MATRIX` (and `Q` from it), the first row
    ///   of `MATRIX2` times `P4` into the accumulator;
    /// - with `s1 & 0x11` equal to `r0`: `P4` through `MATRIX2`'s other
    ///   rows, `P0.xyz *= Q`, `Q` from `P4`; the second point (`+0xc`,
    ///   `+0xe`) into `P3`, `P4.z *= Q`, `P3` through `MATRIX` without its
    ///   `z` lane (`Q` from it); `P3.xy *= Q`, `P0.z` and `P3.z` = `P4.z`;
    ///   both minus the origin into `VA`, `VB` and clamped as in [`plain`];
    /// - the fixed-point and texture steps of [`plain`], the texture pair
    ///   summed `VB.xy = VB.xy + VA.xy`.
    ///
    /// The packet is not written. The original keeps `1/16` and, from the
    /// first point's `Q` to the second's, `vf00.w` on the x87 stack (the
    /// `s1` test pops `vf00.w` when it fails); `vf00` is not written in
    /// between, so it is read again here. Every register the loop carries
    /// from one record to the next (`ebx`, `ecx`, `edx`, `esi`, `ebp -
    /// 0x1bac`) equals an EE word at the loop's start (`v0`'s and `v1`'s
    /// second words, `t4`'s `y`, `z`, `w`), which are read from there.
    pub fn textured(mem: &mut impl Mem, frame: Frame, cmd: u32, first: u32) {
        textured_start(mem, frame, cmd);
        let mut p = first;
        loop {
            textured_vertex(mem, p);
            p = p.wrapping_add(VERTEX);
            let count = COUNT.get(mem).wrapping_sub(1);
            COUNT.set(mem, count);
            frame.set_vertex(mem, p);
            if count.cast_signed() <= 0 {
                break;
            }
        }
    }

    /// `int` to `f32` (`fild dword; fstp dword`).
    fn int_bits(bits: u32) -> f32 {
        Reg::constant(f64::from(bits.cast_signed())).store()
    }

    /// S5's start (`0x007f2dae`..`0x007f3286`).
    #[allow(clippy::similar_names)] // the matrices
    fn textured_start(mem: &mut impl Mem, frame: Frame, cmd: u32) {
        let r0: [u32; 4] = mem.read(mem.rebase(R0_AT));
        T1.set(mem, r0[0]);
        let c4: u32 = mem.read(cmd.wrapping_add(4));
        // v1 = (1/128, 0, ..), VA a copy (v1's first word is replaced
        // below)
        wr(mem, V1 + 4, 0);
        let [_, _, z, w]: [u32; 4] = mem.read(mem.rebase(V1));
        mem.write(mem.rebase(VA), [0x3c00_0000, 0, z, w]);
        // pextlb v0, r0, v0 (inline), then pextlh v0, r0, v0: the colour's
        // bytes to words when r0 is 0. pextlh reads only v0's low eight
        // bytes and writes all 16, so only pextlb's low half (`cmd +4`'s
        // bytes with r0's) counts.
        let (rt, rs) = (c4.to_le_bytes(), r0[0].to_le_bytes());
        let bytes: [u8; 8] = std::array::from_fn(|i| [rt, rs][i % 2][i / 2]);
        mem.write(mem.rebase(V0), bytes);
        SHIFT.set(mem, 0);
        let (v0, r0_at) = (mem.rebase(V0), mem.rebase(R0_AT));
        crate::mat::bridge::ppp_ee_pextlh(mem, v0, r0_at, v0);
        let words: [u32; 4] = mem.read(v0);
        let k = f(rd(mem, VA));
        for (c, word) in (0u32..).zip(words) {
            wr(mem, COLOUR + 4 * c, st(k.mul_m32(int_bits(word))));
        }
        // the matrices (`g_pppColorConShift` is 0 here)
        let m1: u32 = mem.read(cmd.wrapping_add(0x10));
        wr(mem, V0, m1);
        let rows: [u32; 16] = mem.read(m1);
        mem.write(mem.rebase(MATRIX), rows);
        let tex: u32 = mem.read(cmd.wrapping_add(0x1c));
        wr(mem, V1, tex);
        let m2 = if is_r0(mem, tex, rd(mem, V1 + 4)) {
            wr(mem, V1, m1);
            m1
        } else {
            tex
        };
        let rows: [u32; 16] = mem.read(m2);
        mem.write(mem.rebase(MATRIX2), rows);
        // t6 = (0, 2 r0.z, 1, 2 r0.w), t5 = 2 r0, t4 = (r0.x, r0.z, 0, 1)
        // (t4's first word is replaced by every record's first point)
        let twice = r0.map(|x| x.wrapping_add(x));
        mem.write(mem.rebase(T6), [0, twice[2], 0x3f80_0000, twice[3]]);
        mem.write(mem.rebase(T5), twice);
        mem.write(mem.rebase(T4 + 4), [r0[2], 0, 0x3f80_0000]);
        // t3 = shape's words +0x10 (64 bits) plus v1 (cmd +8 into the
        // texture pointer's low half), sign-extended; v0 = (v1, v0 << 5)
        let shape = frame.shape(mem);
        let t3: u64 = mem.read(shape.wrapping_add(0x10));
        wr16(mem, V0, mem.read(cmd.wrapping_add(0xa)));
        wr16(mem, V1, mem.read(cmd.wrapping_add(8)));
        // (`or` with v1's second word, 0 since the start)
        let shifted = rd(mem, V0) << 5;
        let v1 = rd(mem, V1);
        wr(mem, V0, v1);
        let t3 = t3.wrapping_add_signed(i64::from(v1.cast_signed()));
        mem.write(mem.rebase(T3), t3);
        wr(mem, V0 + 4, shifted);
        mem.write(mem.rebase(T2), [0x8000u32, 0]);
    }

    /// One record of [`textured`].
    fn textured_vertex(mem: &mut impl Mem, p: u32) {
        pair_vertex(mem, p, false);
    }

    /// One record of [`textured`] (`fog` clear) or [`fog`] (`fog` set):
    /// the two loops differ only in the fog step and in the operand form
    /// of three last rows (`memory_w`, the matrix element the memory
    /// operand in S5, loaded in S4).
    #[allow(clippy::many_single_char_names)] // the lanes
    fn pair_vertex<M: Mem>(mem: &mut M, p: u32, fog: bool) {
        let memory_w = !fog;
        if !textured_first(mem, p, memory_w) {
            return;
        }
        madda(mem, MATRIX2, P4, 1, false);
        madda(mem, MATRIX2, P4, 2, false);
        maddw(mem, MATRIX2, P4, form(memory_w));
        if fog {
            fog_step(mem);
        } else {
            scale_by_q(mem, P0);
            set_q(mem, P4);
        }
        // the second point's x (+0xc); the translation also loads +0xe into
        // v1 and copies v1 into v0's second and fourth words: the flag
        // test below replaces v1's first word and v0's second, and v0's
        // fourth already holds v1's second word
        wr16(mem, V0, rd16(mem, p.wrapping_add(0xc)));
        load_point(mem, P3, 0);
        let q = f32::from_bits(Q.get(mem));
        let z = st(f(rd(mem, P4 + 8)).mul_m32(q));
        wr(mem, P4 + 8, z);
        madda(mem, MATRIX, P3, 0, true);
        madda(mem, MATRIX, P3, 1, true);
        madda(mem, MATRIX, P3, 2, false);
        // (the original leaves out the z lane, replaced below)
        maddw(mem, MATRIX, P3, form(memory_w));
        set_q(mem, P3);
        // P3.xy *= Q; P0.z and P3.z = P4.z (`fst` copies); with fog, P0.w
        // and P3.w = the fog value
        let z = st(f(z));
        wr(mem, P0 + 8, z);
        let q = f(Q.get(mem));
        let x = st(f(rd(mem, P3)) * q);
        wr(mem, P3, x);
        let y = st(q.mul_m32(f32::from_bits(rd(mem, P3 + 4))));
        wr(mem, P3 + 4, y);
        wr(mem, P3 + 8, z);
        if fog {
            let w = st(f(rd(mem, VB + 0xc)));
            wr(mem, P0 + 0xc, w);
            wr(mem, P3 + 0xc, w);
        }
        pair_end(mem, p);
    }

    /// S4, the fog loop (`0x007f1a8b`..`0x007f2d93`), entered as [`plain`]:
    /// [`textured`] with [`fog_start`] after its start and
    /// [`fog_step`] after each record's first point; the points' `w`
    /// become the fog value. The x87 stack carries `vf00.w` (as in S5) and
    /// the first point's `w` from its projection into the fog step; both
    /// are read again here (nothing writes them in between).
    pub fn fog(mem: &mut impl Mem, frame: Frame, cmd: u32, first: u32) {
        textured_start(mem, frame, cmd);
        fog_start(mem, cmd);
        let mut p = first;
        loop {
            pair_vertex(mem, p, true);
            p = p.wrapping_add(VERTEX);
            let count = COUNT.get(mem).wrapping_sub(1);
            COUNT.set(mem, count);
            frame.set_vertex(mem, p);
            if count.cast_signed() <= 0 {
                break;
            }
        }
    }

    /// The fog loops' start after [`textured_start`] (S4 `0x007f1f4a`..
    /// `0x007f204a`, S1 `0x007ed691`..`0x007ed78f`): `cmd +2` into `v0`'s
    /// low half, then [`FOG2`] and [`FOG`] from `g_OpFog` by its bits
    /// `0x400` and `0x800` (each compared with `r0`, 64 bits), `v1` = the
    /// `0x800` bit (64 bits).
    fn fog_start(mem: &mut impl Mem, cmd: u32) {
        wr16(mem, V0, mem.read(cmd.wrapping_add(2)));
        let flags = rd(mem, V0);
        fog_vector(mem, FOG2, if is_r0(mem, flags & 0x400, 0) { 3 } else { 1 });
        mem.write(mem.rebase(V1), [flags & 0x800, 0]);
        fog_vector(mem, FOG, if is_r0(mem, flags & 0x800, 0) { 2 } else { 0 });
    }

    /// `g_OpFog`'s vector `k` (from `+0x10`) into `dst`.
    fn fog_vector(mem: &mut impl Mem, dst: u32, k: u32) {
        let v: [u32; 4] = mem.read(mem.rebase(OP_FOG_VECTORS + 0x10 * k));
        mem.write(mem.rebase(dst), v);
    }

    /// The fog step after a record's first point (S4 `0x007f2513`..
    /// `0x007f25ec`, S1 `0x007edc4c`..`0x007edd25`): `P0` into
    /// [`P0_COPY`], `P0.xyz *= Q`, `Q` from `P4`, and the fog value into
    /// `VB.w`: `P0.w * FOG.x + FOG.w`, then the smaller of it and `FOG.w`,
    /// then the larger of that and `FOG.y` (`fcom`: a NaN or a tie picks
    /// the fog vector's word).
    fn fog_step(mem: &mut impl Mem) {
        // (the copy is of values the projection stored: `fst` keeps them)
        let p0: [u32; 4] = mem.read(mem.rebase(P0));
        mem.write(mem.rebase(P0_COPY), p0);
        let [scale, low, _, high]: [u32; 4] = mem.read(mem.rebase(FOG));
        let depth = st(f(p0[3]).mul_m32(f32::from_bits(scale)));
        let q = f(Q.get(mem));
        for (c, &x) in (0u32..3).zip(&p0) {
            wr(mem, P0 + 4 * c, st(f(x) * q));
        }
        set_q(mem, P4);
        let high = f(high);
        let v = f(st(high.add_m32(f32::from_bits(depth))));
        // (the original stores this and loads it again: both choices are
        // `f32` values already)
        let v = if v.value() < high.value() { v } else { high };
        let low = f(low);
        let v = if low.value() < v.value() { v } else { low };
        wr(mem, VB + 0xc, st(v));
    }

    /// The first point of an S5, S2, S4 or S1 record (`+0`, `+2`; `y`, `z`,
    /// `w` from `t4`) into `P0` and `P4`, `P0` through `MATRIX` (and `Q`
    /// from it; `memory_w`: the last row's operand form), the first row of
    /// `MATRIX2` times `P4` into the accumulator, the flags into `v0`.
    /// Whether they equal `r0` (the record goes on).
    fn textured_first(mem: &mut impl Mem, p: u32, memory_w: bool) -> bool {
        let hi = rd(mem, V0 + 4);
        wr16(mem, V0, rd16(mem, p));
        wr16(mem, V1, rd16(mem, p.wrapping_add(2)));
        wr(mem, V0 + 0xc, rd(mem, V1 + 4));
        load_point(mem, P0, hi);
        load_again(mem, P4);
        transform(mem, MATRIX, P0, LOADED, memory_w);
        madda(mem, MATRIX2, P4, 0, false);
        let flags = set_flags(mem);
        is_r0(mem, flags, 0)
    }

    /// The end of an S5 or S4 record: `P0` and `P3` minus the origin into
    /// `VA`, `VB` and clamped as in [`plain`], then the fixed-point and
    /// texture steps of [`plain`], the texture pair summed (`VB.xy +=
    /// VA.xy`; S5 adds `VB.y + VA.y`, S4 `VA.y + VB.y`: both are integers /
    /// 4096, never NaN, so the order changes nothing).
    fn pair_end<M: Mem>(mem: &mut M, p: u32) {
        let half = |mem: &M, off: u32| rd16(mem, p.wrapping_add(off));
        minus_origin(mem, VA, P0);
        minus_origin(mem, VB, P3);
        let b: [u32; 4] = mem.read(mem.rebase(VB));
        crate::op::bridge::op_vec3_clamp(mem, mem.rebase(VMAX), mem.rebase(VA), b, true);
        let b: [u32; 4] = mem.read(mem.rebase(VB));
        crate::op::bridge::op_vec3_clamp(mem, mem.rebase(VMIN), mem.rebase(VA), b, false);
        // (the translation zeroes v0's second word here: the flags did)
        if !fixed_point(mem, &[P0, P3]) {
            return;
        }
        // the texture pair: (+0x10, +0x12) into VA, (+0x14, +0x16) into VB;
        // v0 then holds the second v1, the first and v1's second word (0
        // since the flag test) in its last three words (the translation
        // sets them after the first pair too: the second replaces them)
        wr16(mem, V0, half(mem, 0x10));
        wr16(mem, V1, half(mem, 0x12));
        let first = rd(mem, V1);
        load_tex(mem, VA);
        wr16(mem, V0, half(mem, 0x14));
        wr16(mem, V1, half(mem, 0x16));
        let second = rd(mem, V1);
        mem.write(mem.rebase(V0 + 4), [second, first, 0]);
        load_tex(mem, VB);
        let sum_x = f(rd(mem, VA)).add_m32(f32::from_bits(rd(mem, VB)));
        wr(mem, VB, st(sum_x));
        let sum_y = f(rd(mem, VB + 4)).add_m32(f32::from_bits(rd(mem, VA + 4)));
        wr(mem, VB + 4, st(sum_y));
    }

    /// S2, the VU0 textured loop (`0x007eed53`..`0x007f0a17`): [`textured`]'s
    /// start, then per record (`0x18` bytes) up to four points (`+0`/`+2`,
    /// `+4`/`+6`, `+8`/`+0xa`, `+0xc`/`+0xe`), each twice: through
    /// `MATRIX` into `P0`..`P3` and through `MATRIX2` into `P4`..`P7` (the
    /// last through `MATRIX` too). The first point stops the record when
    /// `s1 & 0x11` differs from `r0` (after `MATRIX2`'s first row). Then
    /// each point is scaled by the `Q` of its own projection, each `z` by
    /// the `Q` of its texture point, which replaces it, and the record ends
    /// as S3's ([`quad`]).
    ///
    /// The original tests the flags again before the second, third and
    /// fourth points (`0x007efa92`, `0x007eff94`, `0x007f0425`), on the
    /// same registers and the same `s1` and `r0`, so those tests never
    /// fail: they are left out. Unlike S3 it never tests the records'
    /// halves `+4`, `+8`, `+0xc`.
    pub fn vu0_textured(mem: &mut impl Mem, frame: Frame, cmd: u32, first: u32) {
        textured_start(mem, frame, cmd);
        let mut p = first;
        loop {
            vu0_textured_vertex(mem, p);
            p = p.wrapping_add(VERTEX);
            let count = COUNT.get(mem).wrapping_sub(1);
            COUNT.set(mem, count);
            frame.set_vertex(mem, p);
            if count.cast_signed() <= 0 {
                break;
            }
        }
    }

    /// `z = texture point's z * Q` (`fld; fmul dword [Q]; fstp`).
    fn z_by_q(mem: &mut impl Mem, point: u32, tex: u32) {
        let q = f32::from_bits(Q.get(mem));
        let z = st(f(rd(mem, tex + 8)).mul_m32(q));
        wr(mem, point + 8, z);
    }

    /// One record of [`vu0_textured`].
    fn vu0_textured_vertex<M: Mem>(mem: &mut M, p: u32) {
        let half = |mem: &M, off: u32| rd16(mem, p.wrapping_add(off));
        if !textured_first(mem, p, false) {
            return;
        }
        madda(mem, MATRIX2, P4, 1, false);
        madda(mem, MATRIX2, P4, 2, false);
        maddw(mem, MATRIX2, P4, W::Loaded);
        scale_by_q(mem, P0);
        set_q(mem, P4);
        // the middle points. The translation also loads each point's y
        // half into v1, copies v1's second word into v0's fourth (it is
        // there already) and sets v0 to the flags again: from here every
        // record reaches the flag test in `quad`, which replaces v0's and
        // v1's first words, and v0's second is still 0. v0's third word
        // (0) is set by the last point.
        for (off, point, tex) in [(4, P1, P5), (8, P2, P6)] {
            wr16(mem, V0, half(mem, off));
            load_again(mem, point);
            load_again(mem, tex);
            z_by_q(mem, point - 0x10, tex - 0x10);
            transform(mem, MATRIX, point, [true, true, false], false);
            madda(mem, MATRIX2, tex, 0, true);
            // (the third point's second and third rows through
            // pppVu0MaddaY_copy / pppVu0MaddaZ_copy, the loaded form)
            madda(mem, MATRIX2, tex, 1, off == 4);
            madda(mem, MATRIX2, tex, 2, false);
            maddw(mem, MATRIX2, tex, W::Loaded);
            scale_by_q(mem, point);
            set_q(mem, tex);
        }
        // the last point and its texture point, both through MATRIX (rows
        // 1 and 2 through the copies; the texture point's last row through
        // pppVu0MaddW_copy). The same input through the same matrix: P7
        // equals P3, so the original's `Q = vf00.w / P7.w` gives the `Q`
        // already there and is left out. (v1, v0 and the flags as for the
        // middle points.)
        wr16(mem, V0, half(mem, 0xc));
        load_point(mem, P3, 0);
        load_again(mem, P7);
        z_by_q(mem, P2, P6);
        transform(mem, MATRIX, P3, [true, false, false], false);
        madda(mem, MATRIX, P7, 0, true);
        madda(mem, MATRIX, P7, 1, false);
        madda(mem, MATRIX, P7, 2, false);
        maddw(mem, MATRIX, P7, W::Call);
        let q = f(Q.get(mem));
        let x = st(f(rd(mem, P3)) * q);
        wr(mem, P3, x);
        let y = st(q.mul_m32(f32::from_bits(rd(mem, P3 + 4))));
        wr(mem, P3 + 4, y);
        z_by_q(mem, P3, P7);
        quad(mem, p);
    }

    /// S1, the VU0 fog loop (`0x007ed1d2`..`0x007eed38`): [`fog`]'s start,
    /// then per record (`0x18` bytes) [`vu0_textured`]'s four points with
    /// [`fog_step`] after the first, each later point's `w` the fog value
    /// once the next point is loaded, and S3's end ([`quad`]). The later
    /// points' second and third rows go through `pppVu0MaddaY_copy` /
    /// `pppVu0MaddaZ_copy`, and every last row but the second point's
    /// through `pppVu0MaddW_copy` (the loaded forms). The flag tests before
    /// the later points repeat the first on unchanged operands (as in S2)
    /// and are left out.
    pub fn vu0_fog(mem: &mut impl Mem, frame: Frame, cmd: u32, first: u32) {
        textured_start(mem, frame, cmd);
        fog_start(mem, cmd);
        let mut p = first;
        loop {
            vu0_fog_vertex(mem, p);
            p = p.wrapping_add(VERTEX);
            let count = COUNT.get(mem).wrapping_sub(1);
            COUNT.set(mem, count);
            frame.set_vertex(mem, p);
            if count.cast_signed() <= 0 {
                break;
            }
        }
    }

    /// A point through `matrix`: row 0 with the elements the memory
    /// operands, rows 1 and 2 through the helper calls (the loaded form),
    /// the last row through `pppVu0MaddW_copy`. `Q` is left alone.
    fn project_calls(mem: &mut impl Mem, matrix: u32, point: u32) {
        madda(mem, matrix, point, 0, true);
        madda(mem, matrix, point, 1, false);
        madda(mem, matrix, point, 2, false);
        maddw(mem, matrix, point, W::Call);
    }

    /// `point.w` = the fog value (`fld; fstp`).
    fn fog_w(mem: &mut impl Mem, point: u32) {
        let w = st(f(rd(mem, VB + 0xc)));
        wr(mem, point + 0xc, w);
    }

    /// One record of [`vu0_fog`].
    fn vu0_fog_vertex<M: Mem>(mem: &mut M, p: u32) {
        let half = |mem: &M, off: u32| rd16(mem, p.wrapping_add(off));
        if !textured_first(mem, p, true) {
            return;
        }
        madda(mem, MATRIX2, P4, 1, false);
        madda(mem, MATRIX2, P4, 2, false);
        maddw(mem, MATRIX2, P4, W::Memory);
        fog_step(mem);
        // the later points, each loaded with v0's third word 0 (the
        // translation also loads the y half into v1 and copies v1 into
        // v0's second and fourth words: the flags replace the second, the
        // fourth holds v1's second word already, and `quad` replaces v1's
        // first). The point before gets z = its texture point's z * Q and
        // w = the fog value.
        let points = [
            (4, P1, P5, P0, P4),
            (8, P2, P6, P1, P5),
            (0xc, P3, P7, P2, P6),
        ];
        for (off, point, tex, before, before_tex) in points {
            wr16(mem, V0, half(mem, off));
            load_point(mem, point, 0);
            load_again(mem, tex);
            z_by_q(mem, before, before_tex);
            fog_w(mem, before);
            if off == 4 {
                // (its last row inline, `w` the memory operand)
                transform(mem, MATRIX, point, [true, false, false], true);
            } else {
                project_calls(mem, MATRIX, point);
                set_q(mem, point);
            }
            if off == 0xc {
                break;
            }
            project_calls(mem, MATRIX2, tex);
            scale_by_q(mem, point);
            set_q(mem, tex);
        }
        // the last texture point through MATRIX, as in S2
        project_calls(mem, MATRIX, P7);
        let q = f(Q.get(mem));
        let x = st(f(rd(mem, P3)) * q);
        wr(mem, P3, x);
        let y = st(q.mul_m32(f32::from_bits(rd(mem, P3 + 4))));
        wr(mem, P3 + 4, y);
        set_q(mem, P7);
        z_by_q(mem, P3, P7);
        fog_w(mem, P3);
        quad(mem, p);
    }
}
