//! The op layer's per-vertex light set-ups for the model draws at
//! `0x00a6d380..0x00a6dbe0` (`op_vertex_normal_point44_ffx` and its
//! variants; `docs/formats/vfx.md`, "High range").

#![allow(clippy::many_single_char_names)]

/// The functions over game memory.
pub mod bridge {
    use ffx_bridge::{Mem, x87};

    /// Calls the game function at `sym` (rebased) through `call`.
    fn go<M: Mem>(
        mem: &mut M,
        sym: u32,
        args: &[u32],
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let f = mem.rebase(sym);
        call(mem, f, args)
    }

    /// The emulated EE's `v0` (`g_Ee_Gpr + 0x20`), which the fog vectors
    /// pass through.
    const EE_V0: u32 = 0x00c8_f528;
    /// `g_OpFog + 0x10` (`0x011a0020`): the fog's scale vector; the second
    /// vector the draws take sits at `+0x40` (`0x011a0050`).
    const OP_FOG_SCALE: u32 = 0x011a_0020;
    /// `f64` 1/128 (`0x00b170b8`).
    const K_128TH: u32 = 0x00b1_70b8;
    /// `f64` 255 (`0x00b170c0`).
    const K_255: u32 = 0x00b1_70c0;
    /// `f32` 0.5 (`0x00b92194`).
    const K_HALF: u32 = 0x00b9_2194;
    /// `g_MagicBuffersA` (`0x022fb3e0`): five aligned buffers.
    const MAGIC_BUFFERS_A: u32 = 0x022f_b3e0;

    const SCE_VU0_INVERS_MATRIX4X4: u32 = 0x006e_d460;
    const FFX_VU0_MUL_MATRIX: u32 = 0x006e_d5c0;
    const FFX_VU0_APPLY_MATRIX: u32 = 0x006e_d2e0;
    const OP_FFXLIGHT_TO_OMDLIGHT: u32 = 0x00a6_90c0;
    const OP_FFXLIGHT_TO_OMDLIGHT_0279: u32 = 0x00a6_91c0;
    const OP_MATRIX_LIGHT4_VECTOR: u32 = 0x00a6_cd80;
    const CH_GET_CHR_RENDER_INSTANCE: u32 = 0x0082_6b70;
    const CH_GET_NAME: u32 = 0x0082_6b90;
    const CH_DRAW_CALC_SHADE: u32 = 0x0082_eaf0;
    const MAGIC_FILE_GET_MAGIC_ID: u32 = 0x009d_a3b0;
    const GRAPHIC_PS2_LIGHTING_SETUP_EFFECT: u32 = 0x0064_2750;
    const USER_ALIGNED_MALLOC: u32 = 0x0068_6ff0;

    fn f64_at(mem: &impl Mem, sym: u32) -> f64 {
        mem.read(mem.rebase(sym))
    }

    /// `-x` through `fld`/`fchs`/`fstp` (a signalling NaN comes out quiet).
    fn neg(bits: u32) -> u32 {
        x87::store(-x87::load(bits))
    }

    /// The two fog vectors into `work +0x280` and `+0x290`, each through
    /// the emulated `v0` (`0x00c8f528`; the first leaves its `x` word
    /// alone) as the original copies them.
    fn fog_vectors(mem: &mut impl Mem, work: u32) {
        let fog = mem.rebase(OP_FOG_SCALE);
        let v0 = mem.rebase(EE_V0);
        for (from, to, write_x) in [(0u32, 0x280u32, false), (0x30, 0x290, true)] {
            let src = fog.wrapping_add(from);
            for k in 1..4u32 {
                let w: u32 = mem.read(src.wrapping_add(4 * k));
                mem.write(v0.wrapping_add(4 * k), w);
            }
            let x: u32 = mem.read(src);
            if write_x {
                mem.write(v0, x);
            }
            let dst = work.wrapping_add(to);
            mem.write(dst, x);
            for k in 1..4u32 {
                let w: u32 = mem.read(v0.wrapping_add(4 * k));
                mem.write(dst.wrapping_add(4 * k), w);
            }
        }
    }

    /// Copies the model's matrix `[model +0x28]` to `mat` and its light
    /// matrix `[model +0x2c]` to `light`, the light's rows 0..2 with `x`
    /// and `y` negated.
    fn copy_matrices(mem: &mut impl Mem, model: u32, mat: u32, light: u32) {
        let m: u32 = mem.read(model.wrapping_add(0x28));
        let words: [u32; 16] = mem.read(m);
        mem.write(mat, words);
        let l: u32 = mem.read(model.wrapping_add(0x2c));
        let words: [u32; 16] = mem.read(l);
        mem.write(light, words);
        for at in [0u32, 4, 0x10, 0x14, 0x20, 0x24, 0x30, 0x34] {
            let x = neg(mem.read(light.wrapping_add(at)));
            mem.write(light.wrapping_add(at), x);
        }
    }

    /// `mat` = `light` x (`inverse([model +0x2c])` x `mat`), as two
    /// `FFXVu0MulMatrix` after `sceVu0InversMatrix4x4` into `inv`.
    fn local_matrix<M: Mem>(
        mem: &mut M,
        model: u32,
        mat: u32,
        light: u32,
        inv: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let l: u32 = mem.read(model.wrapping_add(0x2c));
        go(mem, SCE_VU0_INVERS_MATRIX4X4, &[inv, l], call);
        go(mem, FFX_VU0_MUL_MATRIX, &[mat, inv, mat], call);
        go(mem, FFX_VU0_MUL_MATRIX, &[mat, light, mat], call);
    }

    /// The colour scale `work +0x370..+0x37c` times 1/128 (`f32` each)
    /// into `out`.
    fn colour_scale(mem: &mut impl Mem, work: u32, out: u32) {
        let k = f64_at(mem, K_128TH);
        for i in 0..4u32 {
            let x = x87::load(mem.read(work.wrapping_add(0x370 + 4 * i)));
            mem.write(out.wrapping_add(4 * i), x87::store(x87::mul(x, k)));
        }
    }

    /// A byte as `fild`/`fstp m32` gives it.
    fn byte_f32(b: u8) -> u32 {
        f32::from(b).to_bits()
    }

    /// `op_vertex_normal_point44_ffx(obj, model, work, chr)` (`0x00a6d380`):
    /// the lights of a character effect model. With `w = [work +0x390]`
    /// and the locals below `frame` (the original's 16-aligned frame
    /// pointer): the model matrix `[model +0x28]` and the light matrix
    /// `[model +0x2c]` (rows 0..2 `x`, `y` negated) are copied to `frame -
    /// 0x98` and `frame - 0x58`; the first becomes light x inverse(`[model
    /// +0x2c]`) x it; `op_ffxlight_to_omdlight(w + 0x80, w + 0xc0, w +
    /// 0x100)`, `op_matrix_light4_vector(w + 0xc0, w + 0xc0, light)`; the
    /// fog vectors into `work +0x280`/`+0x290`. With a character: the
    /// shade (`Ch_DrawCalcShade` into `frame - 0xf0`), the colour scale
    /// `work +0x370` / 128 (`frame - 0x14`) and
    /// `graphicPS2LightingSetupEffect(instance, name starts with 'w', mat,
    /// [model +0x24], f32 byte +0x4d, w + 0xc0, w + 0x80, f32 byte +0x4c,
    /// shade, scale, 0)`, whose `eax` it returns (else 0). Callees through
    /// `call`.
    pub fn op_vertex_normal_point44_ffx<M: Mem>(
        mem: &mut M,
        model: u32,
        work: u32,
        chr: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (mat, light, inv) = (
            frame.wrapping_sub(0x98),
            frame.wrapping_sub(0x58),
            frame.wrapping_sub(0xd8),
        );
        let w: u32 = mem.read(work.wrapping_add(0x390));
        copy_matrices(mem, model, mat, light);
        local_matrix(mem, model, mat, light, inv, call);
        let (colours, dirs) = (w.wrapping_add(0x80), w.wrapping_add(0xc0));
        go(
            mem,
            OP_FFXLIGHT_TO_OMDLIGHT,
            &[colours, dirs, w.wrapping_add(0x100)],
            call,
        );
        go(mem, OP_MATRIX_LIGHT4_VECTOR, &[dirs, dirs, light], call);
        fog_vectors(mem, work);
        if chr == 0 {
            return 0;
        }
        let instance = go(mem, CH_GET_CHR_RENDER_INSTANCE, &[chr], call);
        let name = go(mem, CH_GET_NAME, &[chr], call);
        let scale = frame.wrapping_sub(0x14);
        colour_scale(mem, work, scale);
        let b4d = byte_f32(mem.read(model.wrapping_add(0x4d)));
        let b4c = byte_f32(mem.read(model.wrapping_add(0x4c)));
        let is_w = u32::from(mem.read::<u8>(name) == b'w');
        let shade = frame.wrapping_sub(0xf0);
        go(
            mem,
            CH_DRAW_CALC_SHADE,
            &[chr, shade, shade.wrapping_add(4), shade.wrapping_add(8)],
            call,
        );
        let tex: u32 = mem.read(model.wrapping_add(0x24));
        go(
            mem,
            GRAPHIC_PS2_LIGHTING_SETUP_EFFECT,
            &[
                instance, is_w, mat, tex, b4d, dirs, colours, b4c, shade, scale, 0,
            ],
            call,
        )
    }

    /// `op_vertex_normal_point44_ffx_2(obj, model, work, chr)`
    /// (`0x00a6d670`): as [`op_vertex_normal_point44_ffx`] for a character
    /// `chr` (nothing at all without one, returning 0), with the lights in
    /// locals: the shade first (`frame - 0xf0`), the matrices as there,
    /// `op_ffxlight_to_omdlight_0279(frame - 0x180, frame - 0x140)` and
    /// `op_matrix_light4_vector(frame - 0x140, frame - 0x140, light)`
    /// before the local matrix; no fog vectors; the colour scale is (1, 1,
    /// 0.5, `w`) for magic 0x245 (`MagicFile_GetMagicId`); the call is
    /// `graphicPS2LightingSetupEffect(instance, 0, mat, [model +0x24], f32
    /// byte +0x4d, frame - 0x140, frame - 0x180, f32 byte +0x4c, shade,
    /// scale, 1)`.
    pub fn op_vertex_normal_point44_ffx_2<M: Mem>(
        mem: &mut M,
        model: u32,
        work: u32,
        chr: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        if chr == 0 {
            return 0;
        }
        let (mat, light, inv) = (
            frame.wrapping_sub(0x98),
            frame.wrapping_sub(0x58),
            frame.wrapping_sub(0xd8),
        );
        let (colours, dirs) = (frame.wrapping_sub(0x180), frame.wrapping_sub(0x140));
        let shade = frame.wrapping_sub(0xf0);
        go(
            mem,
            CH_DRAW_CALC_SHADE,
            &[chr, shade, shade.wrapping_add(4), shade.wrapping_add(8)],
            call,
        );
        copy_matrices(mem, model, mat, light);
        go(mem, OP_FFXLIGHT_TO_OMDLIGHT_0279, &[colours, dirs], call);
        go(mem, OP_MATRIX_LIGHT4_VECTOR, &[dirs, dirs, light], call);
        local_matrix(mem, model, mat, light, inv, call);
        let instance = go(mem, CH_GET_CHR_RENDER_INSTANCE, &[chr], call);
        let b4d = byte_f32(mem.read(model.wrapping_add(0x4d)));
        let b4c = byte_f32(mem.read(model.wrapping_add(0x4c)));
        let scale = frame.wrapping_sub(0x14);
        colour_scale(mem, work, scale);
        if go(mem, MAGIC_FILE_GET_MAGIC_ID, &[], call) == 0x245 {
            mem.write(scale.wrapping_add(4), 1.0f32);
            mem.write(scale, 1.0f32);
            let half: u32 = mem.read(mem.rebase(K_HALF));
            mem.write(scale.wrapping_add(8), x87::store(x87::load(half)));
        }
        let tex: u32 = mem.read(model.wrapping_add(0x24));
        go(
            mem,
            GRAPHIC_PS2_LIGHTING_SETUP_EFFECT,
            &[
                instance, 0, mat, tex, b4d, dirs, colours, b4c, shade, scale, 1,
            ],
            call,
        )
    }

    /// `op_0437_vertex_normal_point44_ffx(packet, model, work, base, mul1,
    /// mul0, slot)` (`0x00a6d890`): the `opus_0437` model's vertex
    /// colours. The buffer `g_MagicBuffersA[slot]` (`0x022fb3e0`) is made
    /// first when null (`user_aligned_malloc(i16 packet +0x14 << 4,
    /// 0x10)`). With the locals below `frame` (the original's `ebp`):
    /// `op_ffxlight_to_omdlight(frame - 0xa4, frame - 0x64, frame -
    /// 0x10c)`, `op_matrix_light4_vector(frame - 0x64, frame - 0x64,
    /// [model +0x2c])`, the fog vectors into `work +0x280`/`+0x290`. Then
    /// for each of the `i16 packet +0x12` groups of the `i16` stream at
    /// `packet + [packet +0x1c] + 0x10` (a count, then that many vertex
    /// numbers `v`; `i = (v << 2) as i16`): the normal `-[model +0x3c]
    /// [i..i + 3]` (w 1) through the light directions, negatives to +0
    /// (w 1.0 after), then through the colours, and for `k` in 0..3 the
    /// colour's negatives to +0, `c = f32(c x f32(byte +0x4c / 128))`
    /// kept, and `buffer[i + k] = f32((c x mul0[k] x mul1[k] + base[k]) /
    /// 255)`. Returns `eax` as left: `packet` without groups, else the
    /// group count sign-extended.
    ///
    /// The original also normalises rows 0..2 of `[model +0x2c]` into a
    /// local nothing reads; that dead work is left out.
    #[allow(clippy::too_many_arguments)]
    pub fn op_0437_vertex_normal_point44_ffx<M: Mem>(
        mem: &mut M,
        packet: u32,
        model: u32,
        work: u32,
        vecs: [u32; 3],
        slot: u32,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let [base, mul1, mul0] = vecs;
        let shade = x87::store(x87::mul(
            f64::from(mem.read::<u8>(model.wrapping_add(0x4c))),
            f64_at(mem, K_128TH),
        ));
        let slot_at = mem
            .rebase(MAGIC_BUFFERS_A)
            .wrapping_add(slot.wrapping_mul(4));
        if mem.read::<u32>(slot_at) == 0 {
            let n = i32::from(mem.read::<i16>(packet.wrapping_add(0x14)));
            let b = go(
                mem,
                USER_ALIGNED_MALLOC,
                &[n.wrapping_shl(4).cast_unsigned(), 0x10],
                call,
            );
            mem.write(slot_at, b);
        }
        let buffer: u32 = mem.read(slot_at);
        let (colours, dirs) = (frame.wrapping_sub(0xa4), frame.wrapping_sub(0x64));
        go(
            mem,
            OP_FFXLIGHT_TO_OMDLIGHT,
            &[colours, dirs, frame.wrapping_sub(0x10c)],
            call,
        );
        let l: u32 = mem.read(model.wrapping_add(0x2c));
        go(mem, OP_MATRIX_LIGHT4_VECTOR, &[dirs, dirs, l], call);
        fog_vectors(mem, work);
        let groups = |mem: &M| i32::from(mem.read::<i16>(packet.wrapping_add(0x12)));
        if groups(mem) <= 0 {
            return packet;
        }
        let (normal, out) = (frame.wrapping_sub(0x24), frame.wrapping_sub(0x14));
        let k255 = f64_at(mem, K_255);
        let off: u32 = mem.read(packet.wrapping_add(0x1c));
        let mut s = off.wrapping_add(0x10).wrapping_add(packet);
        let mut g = 0;
        loop {
            let n = i32::from(mem.read::<i16>(s));
            s = s.wrapping_add(2);
            for _ in 0..n.max(0) {
                let i = mem.read::<u16>(s).wrapping_shl(2);
                s = s.wrapping_add(2);
                let normals: u32 = mem.read(model.wrapping_add(0x3c));
                for k in 0..3u16 {
                    let at = i32::from(i.wrapping_add(k).cast_signed());
                    let at = normals.wrapping_add(at.wrapping_mul(4).cast_unsigned());
                    let x = neg(mem.read(at));
                    mem.write(normal.wrapping_add(4 * u32::from(k)), x);
                }
                mem.write(normal.wrapping_add(0xc), 1.0f32);
                go(mem, FFX_VU0_APPLY_MATRIX, &[out, dirs, normal], call);
                for k in 0..4u32 {
                    let c: f32 = mem.read(out.wrapping_add(4 * k));
                    if c < 0.0 {
                        mem.write(out.wrapping_add(4 * k), 0.0f32);
                    }
                }
                mem.write(out.wrapping_add(0xc), 1.0f32);
                go(mem, FFX_VU0_APPLY_MATRIX, &[out, colours, out], call);
                let dst =
                    buffer.wrapping_add(i32::from(i.cast_signed()).wrapping_mul(4).cast_unsigned());
                for k in 0..3u32 {
                    let at = out.wrapping_add(4 * k);
                    let c: f32 = mem.read(at);
                    if c < 0.0 {
                        mem.write(at, 0.0f32);
                    }
                    let c = x87::store(x87::mul(x87::load(mem.read(at)), x87::load(shade)));
                    mem.write(at, c);
                    let mut x = x87::load(c);
                    x = x87::mul(x, x87::load(mem.read(mul0.wrapping_add(4 * k))));
                    x = x87::mul(x, x87::load(mem.read(mul1.wrapping_add(4 * k))));
                    x = x87::add(x, x87::load(mem.read(base.wrapping_add(4 * k))));
                    x = x87::div(x, k255);
                    mem.write(dst.wrapping_add(4 * k), x87::store(x));
                }
            }
            g += 1;
            let count = groups(mem);
            if g >= count {
                return count.cast_unsigned();
            }
        }
    }
}
