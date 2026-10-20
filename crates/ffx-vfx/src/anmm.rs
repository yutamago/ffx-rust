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

    /// The callees `opp_anmm` reaches through `call` (`fcall` for
    /// `Ch_GetDataScale`, which returns in `ST0`).
    const OPUS_PARENT_GET: u32 = 0x0080_1f00;
    const GET_RENDER_DATA: u32 = 0x0064_53d0;
    const VFX_DESTROY: u32 = 0x0064_5280;
    const CH_ELEM_GET_POS: u32 = 0x0083_3d10;
    const CH_GET_DATA_SCALE: u32 = 0x0082_ab60;
    const OPUS_CHARA_ROOT_M_GET: u32 = 0x0080_5400;
    const OPUS_GET_BONE_MATRIX: u32 = 0x0080_5430;
    const VU0_APPLY_MATRIX: u32 = 0x006e_d2e0;
    const SET_PTR32: u32 = 0x00a4_4700;
    const UPDATE_RENDER_DATA: u32 = 0x0071_b8a0;
    const VFX_UPDATE: u32 = 0x0064_58a0;
    const ALPHA_FUNCTION: u32 = 0x0071_2c30;
    const SET_ENLARGE: u32 = 0x0071_7610;
    const INIT_RENDER_DATA: u32 = 0x0071_1460;
    const RESET_ENLARGE: u32 = 0x0071_7600;
    /// The commands (`0x00c49318`).
    const COMMANDS: u32 = 0x00c4_9318;
    /// `g_EtChrHandles` and `g_EtChrDrawFlags`.
    const CHR_HANDLES: u32 = 0x012a_8100;
    const CHR_DRAW_FLAGS: u32 = 0x012a_84c0;
    /// The `f64` constants 1/16 (`0x00b921e0`) and 10 (`0x00b922b0`).
    const SIXTEENTH: u32 = 0x00b9_21e0;
    const TEN: u32 = 0x00b9_22b0;

    fn fl(mem: &impl Mem, at: u32) -> f32 {
        mem.read(at)
    }

    /// `opp_anmm(host, set)` (`0x008124b0`): steps the animated sprites of
    /// the host's unit (`[host +0x220]`; its head `[unit +0xa8]`: `u16 +0`
    /// sprites of 0x50 bytes from `+0x20`, `+4` into `host +0xc`).
    ///
    /// The host is set up first: `+0x210` = 0, its matrix `+0x40` = the
    /// unit's parent's (`opus_parent_get`, `op_m_copy`), `u16 +2` = 0x80,
    /// `+0x10` and `+0x1c` = `host +0x80`, `+8` = 0, byte `+0x18` = 0. For
    /// each sprite with a command pointer (`+0`; its part `u16 +0x4c` of
    /// the set's parts `[set +0x20]`):
    /// - when its wait byte `+9` runs out, its commands run (see the
    ///   module) until one sets the host's `+0x210`, which becomes the new
    ///   wait; a null next command ends the sprite (`+0` = 0, its render
    ///   data destroyed);
    /// - it moves: `+0x20..+0x28` += `+0x30..+0x38`, `+0x10..+0x18` +=
    ///   those (each rounded to `f32`), `u16 +0xc` += `u16 +0xe`, the
    ///   colour bytes `+0x1c..+0x1f` += `+0x2c..+0x2f` (copied to `host
    ///   +4..+7`), the angle `u16 +0x3c` += `u16 +0x3e`;
    /// - its position `host +0xb0` by the mode byte `+0x4e`: 0 the
    ///   position / 16; 2 a circle (radius `+0x10`, angle `ftol +0x18`,
    ///   height `+0x14`, into `host +0xc0`) around `+0x40`, / 16; 3 the
    ///   circle alone; 4 between the character's (part byte `+0x18`)
    ///   elements `ftol +0x40` and `ftol +0x44` (`Ch_ElemGetPos` into `host
    ///   +0xc0`/`+0xd0`) by `+0x48`, plus the position / 16; 5 `+0x10`
    ///   through the character's bone `ftol +0x40` (`opus_chara_root_m_get`,
    ///   `opus_get_bone_matrix`, `op_mul_m`), scaled by 10 /
    ///   `Ch_GetDataScale`; 1 and others leave it;
    /// - the host's `+0x80` matrix = scale `i16 +0xc` / 4096 turned by the
    ///   angle, `w` 1, and `FFXVu0ApplyMatrix(+0xb0, +0x40, +0xb0)`;
    /// - with a texture animation (`+4`): the frame time byte `+0xa` runs
    ///   down, then the frame `+0xb` steps (past the last: back to 0 when
    ///   the last frame or the sprite has bit 0x80, else the sprite ends),
    ///   and the new time is the frame's `i16 +0x12` / 512; unless the
    ///   part's draw flag is masked (`g_EtChrDrawFlags`), `host +0x14` =
    ///   the set's `+0x54` and the sprite's render data is updated, or
    ///   created (`InitVFXRenderDataShp`, enlarge factor 8 around it) with
    ///   `set_ptr32(anim)` in `frame`'s word.
    ///
    /// Callees through `call`/`fcall`. The set's `+0x54` is put back at the
    /// end. Returns `eax` as left: that word, or 0 with no sprites.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn opp_anmm<M: Mem>(
        mem: &mut M,
        host: u32,
        set: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        const DIVR: crate::x87::Op = |a, b| b / a;
        let h = |o: u32| host.wrapping_add(o);
        let unit: u32 = mem.read(h(0x220));
        let head: u32 = mem.read(unit.wrapping_add(0xa8));
        mem.write(h(0x210), 0u8);
        let f = word(mem, OPUS_PARENT_GET);
        let m = call(mem, f, &[host, unit]);
        crate::op::bridge::op_m_copy(mem, h(0x40), m);
        mem.write(h(2), 0x80u16);
        mem.write(h(0x10), h(0x80));
        mem.write(h(0x1c), h(0x80));
        mem.write(h(8), 0u32);
        let v: u32 = mem.read(head.wrapping_add(4));
        mem.write(h(0xc), v);
        mem.write(h(0x18), 0u8);
        let saved: u32 = mem.read(set.wrapping_add(0x54));
        if mem.read::<u16>(head) == 0 {
            mem.write(set.wrapping_add(0x54), saved);
            return 0;
        }
        let c16: f64 = mem.read(word(mem, SIXTEENTH));
        let mut w = head.wrapping_add(0x20);
        let mut i = 0i32;
        loop {
            'sprite: {
                let mut sp: u32 = mem.read(w);
                if sp == 0 {
                    break 'sprite;
                }
                let at = |o: u32| w.wrapping_add(o);
                let parts: u32 = mem.read(set.wrapping_add(0x20));
                let part = (u32::from(mem.read::<u16>(at(0x4c))) << 8).wrapping_add(parts);
                let wait = mem.read::<u8>(at(9)).wrapping_sub(1);
                mem.write(at(9), wait);
                if wait == 0 {
                    loop {
                        let c: u16 = mem.read(sp);
                        mem.write(h(0x21a), c);
                        let fp: u32 = mem.read(word(mem, COMMANDS) + 4 * u32::from(c & 0xff));
                        sp = call(mem, fp, &[host, set, sp, w]);
                        if mem.read::<u8>(h(0x210)) != 0 {
                            break;
                        }
                    }
                    let wt: u8 = mem.read(h(0x210));
                    mem.write(at(9), wt);
                    mem.write(w, sp);
                    if sp == 0 {
                        let g = word(mem, GET_RENDER_DATA);
                        if call(mem, g, &[0, w]) != 0 {
                            let d = word(mem, VFX_DESTROY);
                            call(mem, d, &[0, w]);
                        }
                    }
                    mem.write(h(0x210), 0u8);
                    if sp == 0 {
                        break 'sprite;
                    }
                }
                // the motion
                let mut steps = [0f32; 3];
                for (k, step) in (0u32..).zip(steps.iter_mut()) {
                    let v = crate::x87::add(fl(mem, at(0x30 + 4 * k)), fl(mem, at(0x20 + 4 * k)));
                    mem.write(at(0x20 + 4 * k), v);
                    *step = v;
                }
                let a: u16 = mem.read(at(0xc));
                let b: u16 = mem.read(at(0xe));
                mem.write(at(0xc), a.wrapping_add(b));
                for k in 0..4 {
                    let a: u8 = mem.read(at(0x1c + k));
                    let b: u8 = mem.read(at(0x2c + k));
                    mem.write(at(0x1c + k), a.wrapping_add(b));
                }
                let x = Reg::load_f32(steps[0]).mem(ADD, fl(mem, at(0x10))).store();
                mem.write(at(0x10), x);
                let y = Reg::load_f32(steps[1]).mem(ADD, fl(mem, at(0x14))).store();
                mem.write(at(0x14), y);
                let z = Reg::load_f32(steps[2]).mem(ADD, fl(mem, at(0x18))).store();
                mem.write(at(0x18), z);
                let rgba: [u8; 4] = mem.read(at(0x1c));
                mem.write(h(4), rgba);
                let a: u16 = mem.read(at(0x3c));
                let b: u16 = mem.read(at(0x3e));
                mem.write(at(0x3c), a.wrapping_add(b));
                let c = Reg::Num(c16);
                match mem.read::<u8>(at(0x4e)) {
                    0 => {
                        for k in 0..3 {
                            let v = Reg::load_f32(fl(mem, at(0x10 + 4 * k))).reg(MUL, c).store();
                            mem.write(h(0xb0 + 4 * k), v);
                        }
                    }
                    mode @ (2 | 3) => {
                        let a =
                            ffx_bridge::x87::ftol(mem, Reg::load_f32(fl(mem, at(0x18))).value());
                        let i1 = (a & 0xfff) + 0x400;
                        let t1 = fl(mem, word(mem, 0x00c4_3bf0).wrapping_add(4 * i1));
                        let r = fl(mem, at(0x10));
                        let v = Reg::load_f32(t1).mem(MUL, r).store();
                        mem.write(h(0xc0), v);
                        let t2 = fl(mem, word(mem, 0x00c4_4bf0).wrapping_add(4 * (i1 & 0xfff)));
                        let v = Reg::load_f32(t2).mem(MUL, fl(mem, at(0x10))).store();
                        mem.write(h(0xc8), v);
                        let y = crate::x87::load_f32(fl(mem, at(0x14)));
                        mem.write(h(0xc4), y);
                        for k in 0..3 {
                            let v = if mode == 2 {
                                Reg::load_f32(fl(mem, at(0x40 + 4 * k)))
                                    .mem(ADD, fl(mem, h(0xc0 + 4 * k)))
                                    .reg(MUL, c)
                            } else {
                                Reg::load_f32(fl(mem, h(0xc0 + 4 * k))).reg(MUL, c)
                            };
                            mem.write(h(0xb0 + 4 * k), v.store());
                        }
                    }
                    4 => {
                        let n = u32::from(mem.read::<u8>(part.wrapping_add(0x18)));
                        let chr: u32 = mem.read(word(mem, CHR_HANDLES) + 4 * n);
                        let gp = word(mem, CH_ELEM_GET_POS);
                        for (o, out) in [(0x40u32, 0xc0u32), (0x44, 0xd0)] {
                            let e =
                                ffx_bridge::x87::ftol(mem, Reg::load_f32(fl(mem, at(o))).value());
                            call(mem, gp, &[chr, e, 0, h(out)]);
                        }
                        let k = fl(mem, at(0x48));
                        let one = Reg::Num(1.0);
                        let inv = one.reg(crate::x87::SUB, Reg::load_f32(k));
                        // x: (d0 k + x / 16) + (1 - k) c0
                        let s = Reg::load_f32(fl(mem, h(0xd0)))
                            .mem(MUL, k)
                            .reg(ADD, Reg::load_f32(fl(mem, at(0x10))).reg(MUL, c));
                        let v = inv.mem(MUL, fl(mem, h(0xc0))).reg(ADD, s).store();
                        mem.write(h(0xb0), v);
                        // y: (d4 k + y / 16) + (1 - k) c4
                        let s = Reg::load_f32(fl(mem, h(0xd4)))
                            .mem(MUL, k)
                            .reg(ADD, Reg::load_f32(fl(mem, at(0x14))).reg(MUL, c));
                        let v = s.reg(ADD, inv.mem(MUL, fl(mem, h(0xc4)))).store();
                        mem.write(h(0xb4), v);
                        // z: (z / 16 + d8 k) + (1 - k) c8
                        let s = c
                            .mem(MUL, fl(mem, at(0x18)))
                            .reg(ADD, Reg::load_f32(fl(mem, h(0xd8))).mem(MUL, k));
                        let v = s
                            .reg(ADD, one.mem(crate::x87::SUB, k).mem(MUL, fl(mem, h(0xc8))))
                            .store();
                        mem.write(h(0xb8), v);
                    }
                    5 => {
                        for k in 0..3 {
                            let v: u32 = mem.read(at(0x10 + 4 * k));
                            mem.write(h(0xc0 + 4 * k), crate::x87::load(v));
                        }
                        let n = u32::from(mem.read::<u8>(part.wrapping_add(0x18)));
                        let chr: u32 = mem.read(word(mem, CHR_HANDLES) + 4 * n);
                        let gs = word(mem, CH_GET_DATA_SCALE);
                        let ds = ffx_bridge::x87::store(fcall(mem, gs, &[chr]));
                        let f = word(mem, OPUS_CHARA_ROOT_M_GET);
                        call(mem, f, &[h(0x100), chr]);
                        let e =
                            ffx_bridge::x87::ftol(mem, Reg::load_f32(fl(mem, at(0x40))).value());
                        let f = word(mem, OPUS_GET_BONE_MATRIX);
                        call(mem, f, &[h(0x140), chr, e]);
                        crate::op::bridge::op_mul_m(mem, h(0x100), h(0x100), h(0x140));
                        let ten: f64 = mem.read(word(mem, TEN));
                        let ds = f32::from_bits(ds);
                        let s = Reg::load_f32(ds).constant(DIVR, ten).store();
                        let v = h(0xc0);
                        crate::op::bridge::op_xyz_scale(mem, v, v, crate::x87::load(s.to_bits()));
                        crate::op::bridge::op_apply_m_w1(mem, v, h(0x100), v);
                        for k in 0..3 {
                            let x: u32 = mem.read(h(0xc0 + 4 * k));
                            mem.write(h(0xb0 + 4 * k), crate::x87::load(x));
                        }
                    }
                    _ => {}
                }
                // the scale and turn matrix at +0x80
                let ang = i32::from(mem.read::<i16>(at(0x3c))).cast_unsigned();
                let sin = word(mem, 0x00c4_4bf0);
                let cos = fl(mem, sin.wrapping_add(4 * (ang.wrapping_add(0x400) & 0xfff)));
                let sn = fl(mem, sin.wrapping_add(4 * (ang & 0xfff)));
                let sc = f64::from(mem.read::<i16>(at(0xc)));
                let fixed: f64 = mem.read(word(mem, 0x00b5_0190));
                let s = Reg::Num(sc).constant(MUL, fixed).store();
                mem.write(h(0x80), [0u32; 12]);
                let r1 = Reg::load_f32(s).mem(MUL, cos).store();
                mem.write(h(0x80), r1);
                let r2 = Reg::load_f32(s).mem(MUL, sn).store();
                mem.write(h(0x84), r2);
                mem.write(h(0x90), r2.to_bits() ^ 0x8000_0000);
                mem.write(h(0x94), r1);
                mem.write(h(0xa8), s);
                mem.write(h(0xbc), 1.0f32);
                let f = word(mem, VU0_APPLY_MATRIX);
                call(mem, f, &[h(0xb0), h(0x40), h(0xb0)]);
                // the texture animation
                let anim: u32 = mem.read(at(4));
                if anim == 0 {
                    break 'sprite;
                }
                let t = mem.read::<u8>(at(0xa)).wrapping_sub(1);
                mem.write(at(0xa), t);
                if t == 0 {
                    let mut fr = mem.read::<u8>(at(0xb)).wrapping_add(1);
                    mem.write(at(0xb), fr);
                    let n: i16 = mem.read(anim.wrapping_add(4));
                    if i32::from(fr) >= i32::from(n) {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // al
                        let last = u32::from((n as u8).wrapping_sub(1));
                        let lf: u8 = mem.read(anim.wrapping_add(0x14 + 8 * last));
                        let sf: u8 = mem.read(at(8));
                        if lf & 0x80 == 0 && sf & 0x80 == 0 {
                            mem.write(w, 0u32);
                            let g = word(mem, GET_RENDER_DATA);
                            if call(mem, g, &[0, w]) != 0 {
                                let d = word(mem, VFX_DESTROY);
                                call(mem, d, &[0, w]);
                            }
                            break 'sprite;
                        }
                        mem.write(at(0xb), 0u8);
                        fr = 0;
                    }
                    let d = i32::from(mem.read::<i16>(anim.wrapping_add(0x12 + 8 * u32::from(fr))));
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // al
                    mem.write(at(0xa), (d / 512) as u8);
                }
                let n = u32::from(mem.read::<u8>(part.wrapping_add(0x18)));
                let flags: u8 = mem.read(word(mem, CHR_DRAW_FLAGS) + n);
                if mem.read::<u8>(part.wrapping_add(0xb3)) & flags != 0 {
                    break 'sprite;
                }
                let fb = u32::from(mem.read::<u8>(at(0xb)));
                let fr = i32::from(mem.read::<i16>(anim.wrapping_add(0x10 + 8 * fb)))
                    .cast_unsigned()
                    .wrapping_add(anim);
                mem.write(h(0x14), saved);
                let f = word(mem, SET_PTR32);
                let p = call(mem, f, &[anim]);
                mem.write(frame, p);
                let g = word(mem, GET_RENDER_DATA);
                if call(mem, g, &[0, w]) != 0 {
                    let f = word(mem, UPDATE_RENDER_DATA);
                    let b = u32::from(mem.read::<u8>(at(0xb)));
                    call(mem, f, &[0, w, frame, host, b]);
                    let f = word(mem, VFX_UPDATE);
                    call(mem, f, &[0, w]);
                } else {
                    let o: u32 = mem.read(fr.wrapping_add(4));
                    let k = u32::from(mem.read::<u8>(o.wrapping_add(fr).wrapping_add(4)));
                    let f = word(mem, ALPHA_FUNCTION);
                    let alpha = call(mem, f, &[k]);
                    let f = word(mem, SET_ENLARGE);
                    call(mem, f, &[8]);
                    let b = u32::from(mem.read::<u8>(at(0xb)));
                    let f = word(mem, INIT_RENDER_DATA);
                    call(mem, f, &[0, w, frame, host, b, alpha]);
                    let f = word(mem, RESET_ENLARGE);
                    call(mem, f, &[]);
                }
            }
            i += 1;
            w = w.wrapping_add(0x50);
            if i >= i32::from(mem.read::<u16>(head)) {
                break;
            }
        }
        mem.write(set.wrapping_add(0x54), saved);
        saved
    }

    /// `0x00810dd0` (`opp_anmm_frame_random`, ours): the sprite's frame
    /// byte `+0xb` = the command's `u16 +2` plus a random number below its
    /// `u16 +4` (0xffff: up to the frame count, `u16 [work +4] +4`, minus
    /// the first; 0 takes no draw), the sum kept in a byte; `+0xa` = 1.
    /// The random number is `op_irandom_core`'s low 16 bits; `junk` as for
    /// [`wait_random`]. Returns `cmd + 6`.
    pub fn frame_random(mem: &mut impl Mem, cmd: u32, work: u32, junk: [u32; 3]) -> u32 {
        let first: u16 = mem.read(cmd.wrapping_add(2));
        let mut n: u16 = mem.read(cmd.wrapping_add(4));
        if n == 0xffff {
            let frames: u32 = mem.read(work.wrapping_add(4));
            n = mem.read::<u16>(frames.wrapping_add(4)).wrapping_sub(first);
        }
        if n != 0 {
            let arg = i32::from(n.cast_signed()).cast_unsigned();
            #[allow(clippy::cast_possible_truncation)] // movzx eax, ax
            let r = rand::op_irandom_core(mem, [arg, junk[0], junk[1], junk[2]]) as u16;
            n = r;
        }
        #[allow(clippy::cast_possible_truncation)] // add al, bl
        let b = (n as u8).wrapping_add(first as u8);
        mem.write(work.wrapping_add(0xb), b);
        mem.write(work.wrapping_add(0xa), 1u8);
        cmd.wrapping_add(6)
    }

    /// `0x008110d0` (`opp_anmm_width_scale`, ours): the sprite's float at
    /// byte offset `cmd +2` times the width of its part's character
    /// (`opus_chr_width(byte part +0x18, i8 host +0x21e)`, part `u16 work
    /// +0x4c` of `[set +0x20]`), rounded to `f32`. Returns `cmd + 4`.
    pub fn width_scale<M: Mem>(
        mem: &mut M,
        host: u32,
        set: u32,
        cmd: u32,
        work: u32,
        fcall: &mut impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        let off = u32::from(mem.read::<u8>(cmd.wrapping_add(2)));
        let no = u32::from(mem.read::<u16>(work.wrapping_add(0x4c)));
        let part = (no << 8).wrapping_add(mem.read(set.wrapping_add(0x20)));
        let battle = i32::from(mem.read::<i8>(host.wrapping_add(0x21e))).cast_unsigned();
        let chr = u32::from(mem.read::<u8>(part.wrapping_add(0x18)));
        let w = crate::opus::bridge::opus_chr_width(mem, chr, battle, fcall);
        let at = off.wrapping_add(work);
        let v = crate::x87::mul(mem.read(at), w);
        mem.write(at, v);
        cmd.wrapping_add(4)
    }

    /// `0x00810ff0` (`opp_anmm_scale_y`, ours): the sprite's float at byte
    /// offset `cmd +2` times the y of its character's scale
    /// (`MsGetChrScale(byte unit +0x18, host +0xc0)` through `call`, then
    /// `f32 host +0xc4`, negated when below 0: an x87 compare, so 0 and
    /// NaN stay), rounded to `f32`. Returns `cmd + 4`.
    pub fn scale_y<M: Mem>(
        mem: &mut M,
        host: u32,
        cmd: u32,
        work: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let off = u32::from(mem.read::<u8>(cmd.wrapping_add(2)));
        let unit: u32 = mem.read(host.wrapping_add(0x220));
        let chr = u32::from(mem.read::<u8>(unit.wrapping_add(0x18)));
        call(mem, word(mem, 0x0079_5720), &[chr, host.wrapping_add(0xc0)]);
        let y: f32 = mem.read(host.wrapping_add(0xc4));
        let y = if y < 0.0 { -y } else { y };
        let at = off.wrapping_add(work);
        let v = crate::x87::mul(y, mem.read(at));
        mem.write(at, v);
        cmd.wrapping_add(4)
    }

    /// `0x00811050` (`opp_anmm_weapon_count`, ours): the counter byte of
    /// the weapon of the sprite's part (`g_OpusWeaponCounter[w]`, `w` =
    /// `opus_get_weapon_index_by_no(byte part +0x18)`, -1 for none takes the
    /// byte before) up by one to at most 0x7f, or with flag bit 0x1000 down
    /// by one to at least 0 (both from the signed byte). Returns `cmd + 2`.
    pub fn weapon_count(mem: &mut impl Mem, host: u32, set: u32, cmd: u32, work: u32) -> u32 {
        let no = u32::from(mem.read::<u16>(work.wrapping_add(0x4c)));
        let part = (no << 8).wrapping_add(mem.read(set.wrapping_add(0x20)));
        let chr = u32::from(mem.read::<u8>(part.wrapping_add(0x18)));
        let w = crate::opus::bridge::opus_get_weapon_index_by_no(mem, chr);
        let at = word(mem, 0x0119_fefc).wrapping_add(w.cast_unsigned());
        let c = i32::from(mem.read::<i8>(at));
        let c = if flags(mem, host) & 0x1000 == 0 {
            (c + 1).min(0x7f)
        } else {
            (c - 1).max(0)
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=0x7f
        mem.write(at, c as u8);
        cmd.wrapping_add(2)
    }

    /// `opp_anmm_fin_en_bone5(host, set, cmd, work)` (`0x00810e20`): a
    /// random bone from the command's list (`t = cmd + i16 cmd +2`: `u16`
    /// count `n`, then `i16` bones): `op_irandom_p(n)` (`junk` as for
    /// [`wait_random`]), at most `n - 1` (as `i16`s); the sprite's `+0x40`
    /// = the bone, `+0x44` = its parent (`Ch_ElemGetParent` of the part's
    /// character handle, through `call`; negative as 0), as floats; the
    /// CRT's `rand()` (through `rand`, its result unused); `+0x48` =
    /// `op_random_p(1)`; byte `+0x4e` = 4. Returns `cmd + 4`.
    #[allow(clippy::too_many_arguments, clippy::many_single_char_names)]
    pub fn fin_en_bone5<M: Mem>(
        mem: &mut M,
        set: u32,
        cmd: u32,
        work: u32,
        junk: [u32; 3],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
        rand_fn: &mut impl FnMut(&mut M) -> u32,
    ) -> u32 {
        let no = u32::from(mem.read::<u16>(work.wrapping_add(0x4c)));
        let part = (no << 8).wrapping_add(mem.read(set.wrapping_add(0x20)));
        let t = i32::from(mem.read::<i16>(cmd.wrapping_add(2)))
            .cast_unsigned()
            .wrapping_add(cmd);
        let n: i16 = mem.read(t);
        let arg = i32::from(n).cast_unsigned();
        #[allow(clippy::cast_possible_truncation)] // movzx eax, ax
        let mut r = rand::op_irandom_core(mem, [arg, junk[0], junk[1], junk[2]]) as u16;
        if r.cast_signed() >= n {
            r = n.wrapping_sub(1).cast_unsigned();
        }
        let e = i32::from(r.cast_signed()).cast_unsigned();
        let bone = i32::from(mem.read::<i16>(t.wrapping_add(2 * e).wrapping_add(2)));
        let chr = u32::from(mem.read::<u8>(part.wrapping_add(0x18)));
        let h = crate::et::bridge::CHR_HANDLES.ptr(mem).at(chr).get(mem);
        #[allow(clippy::cast_possible_truncation)] // movzx eax, ax
        let parent = call(mem, word(mem, 0x0083_3cf0), &[h, bone.cast_unsigned()]) as u16;
        #[allow(clippy::cast_precision_loss)] // fild, then fstp m32: exact for i16 values
        mem.write(work.wrapping_add(0x40), bone as f32);
        let parent = parent.cast_signed().max(0);
        mem.write(work.wrapping_add(0x44), f32::from(parent));
        rand_fn(mem);
        let v = rand::op_random_p(mem, 1.0);
        mem.write(work.wrapping_add(0x48), v);
        mem.write(work.wrapping_add(0x4e), 4u8);
        cmd.wrapping_add(4)
    }
}
