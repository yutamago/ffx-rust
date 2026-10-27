//! The render-side glue at 0x00690000 and up (M4, list `h3-glue-render`,
//! agent h3b): `PostProcessManager_renderHDAOPass`, the HDAO pass (the
//! ambient occlusion, its blur iterations and the apply or copy pass).

pub use crate::h3b_glue2::{
    BEGIN_SCENE, COPY_BUFFER, END_SCENE, FIND_PARAMETER, MEMCPY, RENDER_MESH_FOR_PASS, SCENE,
    SET_CLEAR_COLOR, SET_COLOR_TARGET, SET_DEPTH_STENCIL_TARGET, SET_TEXTURE, SET_VALUE,
    SET_VALUE2, SET_VIEWPORT, SURFACE_CTOR, SURFACE_DTOR, TWO_32,
};

/// `PostProcessManager_resolveDepth(this; renderer, texture, target)`
/// (`ret 0xc`).
pub const RESOLVE_DEPTH: u32 = 0x0069_8110;
/// `ShaderParam_CopyToBuffer_00698960(buffer; param, value)` (`ret 8`).
pub const COPY_TO_BUFFER: u32 = 0x0069_8960;
/// -2.0 (f64).
pub const NEG_TWO: u32 = 0x00b2_c5d0;
/// The parameter names.
pub const NUM_RING: u32 = 0x00b4_7730;
pub const KERNEL_SCALE_CHR: u32 = 0x00b4_773c;
pub const REJECT_RADIUS_CHR: u32 = 0x00b4_7750;
/// `SetValue2` parameters and their fields: acceptRadiusChr,
/// intensityChr, blurDistanceChr, fadeoutScale, acceptAngleChr,
/// kernelScaleSce, rejectRadiusSce, acceptRadiusSce, intensitySce,
/// blurDistanceSce, acceptAngleSce.
pub const VALUES: [(u32, u32); 11] = [
    (0x00b4_7764, 0x1230),
    (0x00b4_7778, 0x1234),
    (0x00b4_7788, 0x1238),
    (0x00b4_779c, 0x1224),
    (0x00b4_77ac, 0x123c),
    (0x00b4_77c0, 0x1240),
    (0x00b4_77d4, 0x1244),
    (0x00b4_77e8, 0x1248),
    (0x00b4_77fc, 0x124c),
    (0x00b4_780c, 0x1250),
    (0x00b4_7820, 0x1254),
];
pub const WIDTH_SIZE: u32 = 0x00b4_7834;
pub const HEIGHT_SIZE: u32 = 0x00b4_7840;
pub const PROJ_INFO: u32 = 0x00b4_7850;
pub const EDGE_SHARPNESS: u32 = 0x00b4_7860;
pub const MAIN_TEX: u32 = 0x00b4_7878;
pub const NORMAL_TEX: u32 = 0x00b4_7884;
pub const DEPTH_TEX: u32 = 0x00b4_7890;
pub const TEXEL_SIZE: u32 = 0x00b4_789c;
pub const AXIS: u32 = 0x00b4_78b8;
pub const AO_TEX: u32 = 0x00b4_78c4;
/// The renderer's pass words: the HDAO, the blur, the apply and the copy.
pub const PASS_HDAO: u32 = 0x00cc_c98c;
pub const PASS_BLUR: u32 = 0x00cc_c99c;
pub const PASS_APPLY: u32 = 0x00cc_c9ac;
pub const PASS_COPY: u32 = 0x00cc_c9bc;

/// The frame (`ebp` offsets).
pub const MATRIX: u32 = 0x16c;
pub const SURFACE_BLUR_A: u32 = 0x12c;
pub const SURFACE_COPY: u32 = 0x11c;
pub const SURFACE_BLUR_B: u32 = 0x10c;
pub const SURFACE_APPLY: u32 = 0xfc;
pub const SURFACE_HDAO: u32 = 0xec;
pub const TEXEL: u32 = 0xdc;
pub const BLUR_B_TEX: u32 = 0xcc;
pub const PROJ: u32 = 0xbc;
pub const BLUR_A_TEX: u32 = 0xac;
pub const APPLY_AO_TEX: u32 = 0x9c;
pub const DEPTH_REC: u32 = 0x8c;
pub const COPY_AO_TEX: u32 = 0x7c;
pub const MAIN_REC: u32 = 0x6c;
pub const COPY_MAIN_TEX: u32 = 0x5c;
pub const NORMAL_REC: u32 = 0x4c;
pub const AXIS_B: u32 = 0x3c;
pub const AXIS_A: u32 = 0x34;
pub const NUM_RING_REC: u32 = 0x28;
pub const WIDTH: u32 = 0x1c;
pub const SHADER: u32 = 0x18;
pub const HEIGHT: u32 = 0x14;
pub const BUFFER: u32 = 0x10;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// A render target's texture: its `+0`, or 0 when its `+4` byte is
    /// set.
    fn texture(h: &impl Host, target: u32) -> u32 {
        if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        }
    }

    fn array_data(h: &impl Host, array: u32) -> u32 {
        match h.read::<u32>(array) & 0x7fff_ffff {
            0 => 0,
            1 => array + 4,
            _ => h.read(array + 4),
        }
    }

    /// An unsigned size as the original converts it: `fild`, plus 2^32
    /// when negative, stored as f32.
    fn size_f32(h: &impl Host, v: u32) -> u32 {
        let mut x = f64::from(v as i32);
        if (v as i32) < 0 {
            x = x87::add(x, x87::load(h.read(h.rebase(TWO_32))));
        }
        x87::store(x)
    }

    fn find(h: &mut impl Host, shader: u32, name: u32) -> u32 {
        let name = h.rebase(name);
        h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name])
    }

    /// A texture record `{0xff, 0, 0, texture}` at `ebp - at` set on
    /// `buffer` for `param`.
    fn set_texture(h: &mut impl Host, ebp: u32, at: u32, buffer: u32, param: u32, tex: u32) -> u32 {
        let record = ebp - at;
        h.write(record, 0xffu32);
        h.write(record + 0xc, tex);
        h.write(record + 8, 0u32);
        h.write(record + 4, 0u32);
        h.call_sym(SET_TEXTURE, Conv::Thiscall(buffer), &[param, record])
    }

    /// The named texture parameter, when the shader has it, set to the
    /// texture of the target at `*target_at`; its error.
    fn texture_param(
        h: &mut impl Host,
        ebp: u32,
        at: u32,
        shader: u32,
        buffer: u32,
        name: u32,
        target_at: u32,
    ) -> u32 {
        let param = find(h, shader, name);
        if param == 0 {
            return 0;
        }
        let tex = texture(h, h.read(target_at));
        set_texture(h, ebp, at, buffer, param, tex)
    }

    /// A parameter copied from `from` (`size` bytes at the parameter's
    /// offset into the buffer, which holds its size at `+0`): 3 when it
    /// doesn't fit or the offset is under 4.
    fn copy_param(h: &mut impl Host, buffer: u32, param: u32, from: u32) -> Result<(), u32> {
        let offset = u32::from(h.read::<u16>(param + 8));
        let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
        if offset + size > h.read::<u32>(buffer) || offset < 4 {
            return Err(3);
        }
        h.call_sym(MEMCPY, Conv::Cdecl, &[buffer + offset, from, size]);
        Ok(())
    }

    /// The axis record `{x, y}` at `ebp - at` for the "`g_ssao_axis`"
    /// parameter; its error.
    fn axis(h: &mut impl Host, ebp: u32, at: u32, shader: u32, buffer: u32, xy: [f32; 2]) -> u32 {
        let param = find(h, shader, AXIS);
        if param == 0 {
            return 0;
        }
        h.write(ebp - at, xy[0]);
        h.write(ebp - at + 4, xy[1]);
        h.call_sym(COPY_TO_BUFFER, Conv::Thiscall(buffer), &[param, ebp - at])
    }

    /// One pass of the HDAO mesh (`+0x121c`) into the target at
    /// `*target_at` through the surface at `ebp - at`: target, viewport,
    /// clear, `pass`; the first error, or 0. Each call's `eax` goes to
    /// `results` when given (the original keeps them in the renderer's
    /// argument slot). The surface is destroyed here.
    #[allow(clippy::too_many_arguments)] // the pass's parts
    fn pass(
        h: &mut impl Host,
        ebp: u32,
        at: u32,
        this: u32,
        renderer: u32,
        target_at: u32,
        pass: u32,
        results: Option<u32>,
    ) -> u32 {
        let surface = ebp - at;
        let target: u32 = h.read(target_at);
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[target, 0]);
        let error = pass_draw(h, surface, this, renderer, target_at, pass, results);
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
        error
    }

    /// `e`, kept in `results` when given.
    fn keep(h: &mut impl Host, results: Option<u32>, e: u32) -> u32 {
        if let Some(at) = results {
            h.write(at, e);
        }
        e
    }

    fn pass_draw(
        h: &mut impl Host,
        surface: u32,
        this: u32,
        renderer: u32,
        target_at: u32,
        pass: u32,
        results: Option<u32>,
    ) -> u32 {
        let r = Conv::Thiscall(renderer);
        let e = h.call_sym(SET_COLOR_TARGET, r, &[surface, 0]);
        if keep(h, results, e) != 0 {
            return e;
        }
        let e = h.call_sym(SET_DEPTH_STENCIL_TARGET, r, &[0]);
        if keep(h, results, e) != 0 {
            return e;
        }
        let target: u32 = h.read(target_at);
        let tall = texture(h, target);
        let wide = texture(h, target);
        let height: u32 = h.read(tall + 0x20);
        let width: u32 = h.read(wide + 0x1c);
        h.call_sym(SET_VIEWPORT, r, &[0, 0, width, height]);
        let e = h.call_sym(SET_CLEAR_COLOR, r, &[0; 4]);
        if keep(h, results, e) != 0 {
            return e;
        }
        let e = h.call_sym(BEGIN_SCENE, r, &[0]);
        if keep(h, results, e) != 0 {
            return e;
        }
        h.write(renderer + 0x48, h.rebase(pass));
        let mesh: u32 = h.read(this + 0x121c);
        let e = h.call_sym(RENDER_MESH_FOR_PASS, r, &[mesh]);
        if keep(h, results, e) != 0 {
            return e;
        }
        let e = h.call_sym(END_SCENE, r, &[1]);
        keep(h, results, e)
    }

    /// `PostProcessManager_renderHDAOPass(this; renderer, unused, target,
    /// normals)` (`0x00695ba0`, thiscall, `ret 0x10`; frame at `ebp`, see
    /// the module's offsets; the renderer's slot takes the HDAO pass's
    /// results, then the blur count, `normals`' slot the texel height).
    /// Only with `+0x123d0` and `+0x123d1` (set while the HDAO setting
    /// `+0x1264` is on, cleared for setting 0): resolves depth under MSAA
    /// (`+0x122b4`), sets the ring count (6, 0xc for 2, 0x14 for 3) and the
    /// HDAO parameters, renders the occlusion into `+0x9c`, blurs it
    /// `+0x125c` times through `+0xa4`, then applies it into `target`
    /// (`+0x1260`) or renders `+0xa4` and copies it to `target`. 0, 1 or 3
    /// (a parameter that doesn't fit), or the first error.
    pub fn render_hdao_pass(h: &mut impl Host, ebp: u32, slots: u32, this: u32) -> u32 {
        match hdao(h, ebp, slots, this) {
            Ok(()) => 0,
            Err(e) => e,
        }
    }

    fn check(e: u32) -> Result<(), u32> {
        if e == 0 { Ok(()) } else { Err(e) }
    }

    #[allow(clippy::too_many_lines)] // the original's sequence
    fn hdao(h: &mut impl Host, ebp: u32, slots: u32, this: u32) -> Result<(), u32> {
        let on: u8 = h.read(this + 0x123d0);
        if on == 0 {
            return Ok(());
        }
        if h.read::<u8>(this + 0x123d1) == 0 && h.read::<u32>(this + 0x1264) != 0 {
            h.write(this + 0x123d1, 1u8);
        }
        if h.read::<u8>(this + 0x123d1) == 0 {
            return Ok(());
        }
        let renderer: u32 = h.read(slots);
        if h.read::<u8>(this + 0x122b4) != 0 {
            let tex = texture(h, h.read(this + 0x90));
            let target: u32 = h.read(this + 0x98);
            h.call_sym(
                RESOLVE_DEPTH,
                Conv::Thiscall(this),
                &[renderer, tex, target],
            );
        }
        let mesh: u32 = h.read(this + 0x121c);
        let materials: u32 = h.read(mesh + 0x10);
        let first: u32 = h.read(array_data(h, materials));
        let shader: u32 = h.read(first);
        h.write(ebp - SHADER, shader);
        let first: u32 = h.read(array_data(h, materials));
        let buffer: u32 = h.read(first + 4);
        h.write(ebp - BUFFER, buffer);
        let rings = match h.read::<u32>(this + 0x1264) {
            0 => {
                h.write(this + 0x123d1, 0u8);
                6
            }
            2 => 0xc,
            3 => 0x14,
            _ => 6,
        };
        h.write(ebp - NUM_RING_REC, rings);

        let param = find(h, shader, NUM_RING);
        if param != 0 {
            // junk read: a size over 4 copies frame words past the count
            // (some never written); 0 there (brief 2026-10-24)
            copy_param(h, buffer, param, ebp - NUM_RING_REC)?;
        }
        for (name, field) in [(KERNEL_SCALE_CHR, 0x1228), (REJECT_RADIUS_CHR, 0x122c)] {
            let param = find(h, shader, name);
            if param != 0 {
                let from = this.wrapping_add(field);
                if from == 0 {
                    return Err(1);
                }
                copy_param(h, buffer, param, from)?;
            }
        }
        for (name, field) in VALUES {
            let param = find(h, shader, name);
            if param != 0 {
                check(h.call_sym(SET_VALUE2, Conv::Thiscall(buffer), &[param, this + field]))?;
            }
        }
        for (name, at, off) in [(WIDTH_SIZE, WIDTH, 0x1c), (HEIGHT_SIZE, HEIGHT, 0x20)] {
            let param = find(h, shader, name);
            if param != 0 {
                let target: u32 = h.read(this + 0x8c);
                let size: u32 = h.read(texture(h, target) + off);
                let v = size_f32(h, size);
                h.write(ebp - at, v);
                check(h.call_sym(SET_VALUE2, Conv::Thiscall(buffer), &[param, ebp - at]))?;
            }
        }
        let param = find(h, shader, PROJ_INFO);
        if param != 0 {
            proj_info(h, ebp, this, buffer, param)?;
        }
        let param = find(h, shader, EDGE_SHARPNESS);
        if param != 0 {
            check(h.call_sym(SET_VALUE2, Conv::Thiscall(buffer), &[param, this + 0x1258]))?;
        }
        check(texture_param(
            h,
            ebp,
            MAIN_REC,
            shader,
            buffer,
            MAIN_TEX,
            slots + 8,
        ))?;
        check(texture_param(
            h,
            ebp,
            NORMAL_REC,
            shader,
            buffer,
            NORMAL_TEX,
            slots + 0xc,
        ))?;
        let depth = if h.read::<u8>(this + 0x122b4) == 0 {
            0x90
        } else {
            0x98
        };
        check(texture_param(
            h,
            ebp,
            DEPTH_REC,
            shader,
            buffer,
            DEPTH_TEX,
            this + depth,
        ))?;

        check(pass(
            h,
            ebp,
            SURFACE_HDAO,
            this,
            renderer,
            this + 0x9c,
            PASS_HDAO,
            Some(slots),
        ))?;
        h.write(slots, 0u32);
        let mut i: i32 = 0;
        while i < h.read::<i32>(this + 0x125c) {
            blur(h, ebp, slots, this, renderer, shader, buffer)?;
            i += 1;
            h.write(slots, i);
        }

        if h.read::<u8>(this + 0x1260) != 0 {
            check(texture_param(
                h,
                ebp,
                APPLY_AO_TEX,
                shader,
                buffer,
                AO_TEX,
                this + 0x9c,
            ))?;
            check(pass(
                h,
                ebp,
                SURFACE_APPLY,
                this,
                renderer,
                slots + 8,
                PASS_APPLY,
                None,
            ))
        } else {
            check(texture_param(
                h,
                ebp,
                COPY_MAIN_TEX,
                shader,
                buffer,
                MAIN_TEX,
                slots + 8,
            ))?;
            check(texture_param(
                h,
                ebp,
                COPY_AO_TEX,
                shader,
                buffer,
                AO_TEX,
                this + 0x9c,
            ))?;
            let surface = ebp - SURFACE_COPY;
            let target: u32 = h.read(this + 0xa4);
            h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[target, 0]);
            let e = pass_draw(h, surface, this, renderer, this + 0xa4, PASS_COPY, None);
            if e == 0 {
                let from = texture(h, h.read(this + 0xa4));
                let to: u32 = h.read(slots + 8);
                h.call_sym(
                    COPY_BUFFER,
                    Conv::Thiscall(this),
                    &[renderer, from, to, 1, 0, 0],
                );
            }
            h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
            check(e)
        }
    }

    /// `g_ssao_projInfo` from the scene camera's projection (copied to the
    /// frame): `-2 / (w m00)`, `-2 / (h m11)`, `(1 - m02) / m00`,
    /// `(1 + m12) / m11`, the sizes of `+0x8c`'s texture as f32.
    fn proj_info(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        buffer: u32,
        param: u32,
    ) -> Result<(), u32> {
        let scene: u32 = h.read(h.rebase(SCENE));
        let camera: u32 = h.read(scene + 0xdc);
        let m = ebp - MATRIX;
        for k in 0..16 {
            let w: u32 = h.read(camera + 0x34 + 4 * k);
            h.write(m + 4 * k, w);
        }
        let target: u32 = h.read(this + 0x8c);
        let height: u32 = h.read(texture(h, target) + 0x20);
        let width: u32 = h.read(texture(h, target) + 0x1c);
        let m00 = x87::load(h.read(m));
        let m02 = x87::load(h.read(m + 8));
        let m11 = x87::load(h.read(m + 0x14));
        let m12 = x87::load(h.read(m + 0x18));
        let neg_two = f64::from_bits(h.read::<u64>(h.rebase(NEG_TWO)));
        let w = x87::load(size_f32(h, width));
        let ht = x87::load(size_f32(h, height));
        let record = ebp - PROJ;
        h.write(record, x87::store(x87::div(neg_two, x87::mul(w, m00))));
        h.write(record + 4, x87::store(x87::div(neg_two, x87::mul(ht, m11))));
        h.write(record + 8, x87::store(x87::div(x87::sub(1.0, m02), m00)));
        h.write(record + 0xc, x87::store(x87::div(x87::add(1.0, m12), m11)));
        check(h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[param, record]))
    }

    /// One blur iteration: the texel size, axis (1, 0) from `+0x9c` into
    /// `+0xa4`, axis (0, 1) back into `+0x9c`.
    #[allow(clippy::too_many_arguments)] // the original's locals
    fn blur(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        this: u32,
        renderer: u32,
        shader: u32,
        buffer: u32,
    ) -> Result<(), u32> {
        let param = find(h, shader, TEXEL_SIZE);
        if param != 0 {
            let target: u32 = h.read(this + 0x8c);
            let height: u32 = h.read(texture(h, target) + 0x20);
            let width: u32 = h.read(texture(h, target) + 0x1c);
            let w = size_f32(h, width);
            let ht = size_f32(h, height);
            let record = ebp - TEXEL;
            h.write(record, x87::store(x87::div(1.0, x87::load(w))));
            h.write(record + 4, x87::store(x87::div(1.0, x87::load(ht))));
            h.write(record + 8, w);
            h.write(record + 0xc, ht);
            // the original forms the sizes in the `normals` argument slot
            // (the height last)
            h.write(slots + 0xc, ht);
            check(h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[param, record]))?;
        }
        check(axis(h, ebp, AXIS_A, shader, buffer, [1.0, 0.0]))?;
        check(texture_param(
            h,
            ebp,
            BLUR_A_TEX,
            shader,
            buffer,
            MAIN_TEX,
            this + 0x9c,
        ))?;
        check(pass(
            h,
            ebp,
            SURFACE_BLUR_A,
            this,
            renderer,
            this + 0xa4,
            PASS_BLUR,
            None,
        ))?;
        check(axis(h, ebp, AXIS_B, shader, buffer, [0.0, 1.0]))?;
        check(texture_param(
            h,
            ebp,
            BLUR_B_TEX,
            shader,
            buffer,
            MAIN_TEX,
            this + 0xa4,
        ))?;
        check(pass(
            h,
            ebp,
            SURFACE_BLUR_B,
            this,
            renderer,
            this + 0x9c,
            PASS_BLUR,
            None,
        ))
    }
}
