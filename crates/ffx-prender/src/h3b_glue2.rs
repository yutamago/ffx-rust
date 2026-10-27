//! The render-side glue at 0x00690000 and up (M4, list `h3-glue-render`,
//! agent h3b): `PostProcessManager_renderUSM`, the unsharp-mask pass.

pub use crate::h3_glue37::{
    BEGIN_SCENE, COPY_BUFFER, END_SCENE, FIND_PARAMETER, SET_CLEAR_COLOR, SET_COLOR_TARGET,
    SET_DEPTH_STENCIL_TARGET, SET_TEXTURE, SET_VALUE, SET_VIEWPORT, SURFACE_CTOR, SURFACE_DTOR,
};

/// `g_CurrentMapNo`: maps 0x17 and 0x15c skip the pass.
pub const CURRENT_MAP_NO: u32 = 0x012f_bc40;
/// `g_PhyreScene` and `PhyreScene_getLinearSampler(scene; which)`.
pub const SCENE: u32 = 0x00cc_b9d8;
pub const GET_LINEAR_SAMPLER: u32 = 0x0065_e050;
/// `memcpy` (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// `ShaderParamBuffer_SetValue2(buffer; param, value)` (`ret 8`).
pub const SET_VALUE2: u32 = 0x0063_86c0;
/// `PRendererBase_renderMeshInstance(renderer; mesh, materials, a, b)`
/// (`ret 0x10`) and its one-argument form (`ret 4`).
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
pub const RENDER_MESH_FOR_PASS: u32 = 0x005b_9a70;
/// The renderer's pass words for the blur and the apply passes.
pub const PASS_BLUR: u32 = 0x00cc_c9dc;
pub const PASS_APPLY: u32 = 0x00cc_c9ec;
/// 2^32 (f32), added to a negative `fild`.
pub const TWO_32: u32 = 0x00b1_1f14;
/// The parameter names: `"USMSamplerState"`, "offsets", `"blur_MainTex"`,
/// `"apply_MainTex"`, `"apply_MainTexBlurred"`, "intensity", "threshhold".
pub const USM_SAMPLER_STATE: u32 = 0x00b4_78cc;
pub const OFFSETS: u32 = 0x00b4_78dc;
pub const BLUR_MAIN_TEX: u32 = 0x00b4_78e4;
pub const APPLY_MAIN_TEX: u32 = 0x00b4_78f4;
pub const APPLY_MAIN_TEX_BLURRED: u32 = 0x00b4_7904;
pub const INTENSITY: u32 = 0x00b4_791c;
pub const THRESHHOLD: u32 = 0x00b4_7928;

/// The frame (`ebp` offsets): the three render surfaces, the parameter
/// records, and the locals callees never see but the sampler copy may.
pub const SURFACE_SOURCE: u32 = 0xb8;
pub const SURFACE_APPLY: u32 = 0xa8;
pub const SURFACE_BLUR: u32 = 0x98;
pub const BLUR_TEX_2: u32 = 0x88;
pub const BLUR_TEX_1: u32 = 0x78;
pub const SAMPLER: u32 = 0x68;
pub const BLURRED_TEX: u32 = 0x58;
pub const OFFSETS_V: u32 = 0x48;
pub const APPLY_TEX: u32 = 0x38;
pub const OFFSETS_U: u32 = 0x28;
pub const OFFSETS_PARAM: u32 = 0x18;
pub const BUFFER: u32 = 0x14;

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

    /// `PArray` data: inline for one element, the pointer for more, none
    /// for zero.
    fn array_data(h: &impl Host, array: u32) -> u32 {
        let n = h.read::<u32>(array) & 0x7fff_ffff;
        match n {
            0 => 0,
            1 => array + 4,
            _ => h.read(array + 4),
        }
    }

    /// `copyBuffer(this; renderer, from, to, 1, 0, 0)`; its `eax` is
    /// dropped.
    fn copy_buffer(h: &mut impl Host, this: u32, renderer: u32, from: u32, to: u32) {
        h.call_sym(
            COPY_BUFFER,
            Conv::Thiscall(this),
            &[renderer, from, to, 1, 0, 0],
        );
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

    /// `scale / size` as the original forms it: the size `fild`ed (plus
    /// 2^32 when negative) and stored as f32 first, the quotient stored as
    /// f32.
    fn texel(h: &impl Host, scale: u32, size: u32) -> u32 {
        let mut v = f64::from(size as i32);
        if (size as i32) < 0 {
            v = x87::add(v, x87::load(h.read(h.rebase(TWO_32))));
        }
        let size = x87::load(x87::store(v));
        x87::store(x87::div(x87::load(h.read(scale)), size))
    }

    /// One pass: `target` bound through the surface at `ebp - at`, the
    /// viewport its size, cleared to 0, `pass` set, the mesh drawn (`full`:
    /// with its materials and the renderer's `+0x13c`); the first error,
    /// or 0. The surface is constructed here; the caller destroys it.
    #[allow(clippy::too_many_arguments)] // the pass's parts
    fn pass(
        h: &mut impl Host,
        ebp: u32,
        at: u32,
        this: u32,
        renderer: u32,
        target_at: u32,
        pass: u32,
        full: bool,
    ) -> u32 {
        let surface = ebp - at;
        let target: u32 = h.read(this + target_at);
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[target, 0]);
        let r = Conv::Thiscall(renderer);
        let error = h.call_sym(SET_COLOR_TARGET, r, &[surface, 0]);
        if error != 0 {
            return error;
        }
        let error = h.call_sym(SET_DEPTH_STENCIL_TARGET, r, &[0]);
        if error != 0 {
            return error;
        }
        let target: u32 = h.read(this + target_at);
        let tall = texture(h, target);
        let wide = texture(h, target);
        let height: u32 = h.read(tall + 0x20);
        let width: u32 = h.read(wide + 0x1c);
        h.call_sym(SET_VIEWPORT, r, &[0, 0, width, height]);
        let error = h.call_sym(SET_CLEAR_COLOR, r, &[0; 4]);
        if error != 0 {
            return error;
        }
        let error = h.call_sym(BEGIN_SCENE, r, &[0]);
        if error != 0 {
            return error;
        }
        let error = if full {
            let extra: u32 = h.read(renderer + 0x13c);
            h.write(renderer + 0x48, h.rebase(pass));
            let mesh: u32 = h.read(this + 0x1229c);
            let materials: u32 = h.read(mesh + 0x10);
            h.call_sym(RENDER_MESH_INSTANCE, r, &[mesh, materials, 0, extra])
        } else {
            h.write(renderer + 0x48, h.rebase(pass));
            let mesh: u32 = h.read(this + 0x1229c);
            h.call_sym(RENDER_MESH_FOR_PASS, r, &[mesh])
        };
        if error != 0 {
            return error;
        }
        h.call_sym(END_SCENE, r, &[1])
    }

    fn find(h: &mut impl Host, shader: u32, name: u32) -> u32 {
        let name = h.rebase(name);
        h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name])
    }

    /// `PostProcessManager_renderUSM(this; renderer)` (`0x00697670`,
    /// thiscall, `ret 4`; frame at `ebp`, see the module's offsets; the
    /// renderer's slot takes the USM material's shader). Off (`+0x123d2`,
    /// `+0x123d3`) or on maps 0x17 and 0x15c: 0. Copies `+0x8c` to `+0x9c`,
    /// to `+0x1178`, to `+0x117c`; sets the sampler state (3 when its
    /// record doesn't fit the buffer), the offsets, the blur texture;
    /// blurs `+0x1178`'s mesh into `+0x1188` and `+0x117c`; sets the apply
    /// textures, intensity and threshold; then with `+0x122b0` not 0 copies
    /// `+0x117c` back to `+0x8c`, else renders the apply pass into `+0xac`
    /// and copies that. The first error, or 0.
    #[allow(clippy::too_many_lines)] // the original's sequence
    pub fn render_usm(h: &mut impl Host, ebp: u32, slots: u32, this: u32) -> u32 {
        if h.read::<u8>(this + 0x123d2) == 0 || h.read::<u8>(this + 0x123d3) == 0 {
            return 0;
        }
        let map: u32 = h.read(h.rebase(CURRENT_MAP_NO));
        if map == 0x17 || map == 0x15c {
            return 0;
        }
        let renderer: u32 = h.read(slots);
        for (from, to) in [(0x8c, 0x9c), (0x9c, 0x1178), (0x1178, 0x117c)] {
            let source = texture(h, h.read(this + from));
            let dest: u32 = h.read(this + to);
            copy_buffer(h, this, renderer, source, dest);
        }
        let mesh: u32 = h.read(this + 0x1229c);
        let materials: u32 = h.read(mesh + 0x10);
        let first: u32 = h.read(array_data(h, materials));
        let shader: u32 = h.read(first);
        h.write(slots, shader);
        let first: u32 = h.read(array_data(h, materials));
        let buffer: u32 = h.read(first + 4);
        h.write(ebp - BUFFER, buffer);

        let param = find(h, shader, USM_SAMPLER_STATE);
        if param != 0 {
            let scene: u32 = h.read(h.rebase(SCENE));
            let sampler = h.call_sym(GET_LINEAR_SAMPLER, Conv::Thiscall(scene), &[2]);
            h.write(ebp - SAMPLER + 8, sampler);
            let offset = u32::from(h.read::<u16>(param + 8));
            let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
            h.write(ebp - SAMPLER, 0x47u32);
            h.write(ebp - SAMPLER + 4, 0u32);
            h.write(ebp - SAMPLER + 0xc, 0u32);
            if offset + size > h.read::<u32>(buffer) || offset < 4 {
                return 3;
            }
            // junk read: a size over 0x10 copies frame words past the
            // record the original never wrote there; 0 (brief 2026-10-24)
            h.call_sym(MEMCPY, Conv::Cdecl, &[buffer + offset, ebp - SAMPLER, size]);
        }
        let offsets = find(h, shader, OFFSETS);
        h.write(ebp - OFFSETS_PARAM, offsets);
        if offsets != 0 {
            let target: u32 = h.read(this + 0x117c);
            let height: u32 = h.read(texture(h, target) + 0x20);
            let q = texel(h, this + 0x122a4, height);
            let record = ebp - OFFSETS_V;
            h.write(record, 0u32);
            h.write(record + 4, q);
            h.write(record + 8, 0u32);
            h.write(record + 0xc, 0u32);
            let error = h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[offsets, record]);
            if error != 0 {
                return error;
            }
        }
        let blur = find(h, shader, BLUR_MAIN_TEX);
        if blur != 0 {
            let tex = texture(h, h.read(this + 0x117c));
            let error = set_texture(h, ebp, BLUR_TEX_1, buffer, blur, tex);
            if error != 0 {
                return error;
            }
        }

        let error = usm_passes(h, ebp, this, renderer, shader, buffer, offsets, blur);
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(ebp - SURFACE_SOURCE), &[]);
        error
    }

    /// From the first pass on (its surface constructed here, destroyed by
    /// the caller); the first error, or 0.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the original's locals and sequence
    fn usm_passes(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        renderer: u32,
        shader: u32,
        buffer: u32,
        offsets: u32,
        blur: u32,
    ) -> u32 {
        let error = pass(
            h,
            ebp,
            SURFACE_SOURCE,
            this,
            renderer,
            0x1188,
            PASS_BLUR,
            true,
        );
        if error != 0 {
            return error;
        }
        if offsets != 0 {
            let target: u32 = h.read(this + 0x117c);
            let width: u32 = h.read(texture(h, target) + 0x1c);
            let size = {
                let mut v = f64::from(width as i32);
                if (width as i32) < 0 {
                    v = x87::add(v, x87::load(h.read(h.rebase(TWO_32))));
                }
                x87::store(v)
            };
            h.write(ebp - OFFSETS_PARAM, size);
            let q = texel(h, this + 0x122a4, width);
            let record = ebp - OFFSETS_U;
            h.write(record, q);
            h.write(record + 4, 0u32);
            h.write(record + 8, 0u32);
            h.write(record + 0xc, 0u32);
            let error = h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[offsets, record]);
            if error != 0 {
                return error;
            }
        }
        if blur != 0 {
            let tex = texture(h, h.read(this + 0x1188));
            let error = set_texture(h, ebp, BLUR_TEX_2, buffer, blur, tex);
            if error != 0 {
                return error;
            }
        }
        let error = pass(
            h,
            ebp,
            SURFACE_BLUR,
            this,
            renderer,
            0x117c,
            PASS_BLUR,
            false,
        );
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(ebp - SURFACE_BLUR), &[]);
        if error != 0 {
            return error;
        }
        for (name, from, at) in [
            (APPLY_MAIN_TEX, 0x8c, APPLY_TEX),
            (APPLY_MAIN_TEX_BLURRED, 0x117c, BLURRED_TEX),
        ] {
            let param = find(h, shader, name);
            if param != 0 {
                let tex = texture(h, h.read(this + from));
                let error = set_texture(h, ebp, at, buffer, param, tex);
                if error != 0 {
                    return error;
                }
            }
        }
        for (name, value) in [(INTENSITY, 0x122a8), (THRESHHOLD, 0x122ac)] {
            let param = find(h, shader, name);
            if param != 0 {
                let error = h.call_sym(SET_VALUE2, Conv::Thiscall(buffer), &[param, this + value]);
                if error != 0 {
                    return error;
                }
            }
        }
        // fld; fldz; fucompp: only an exact 0 (not a NaN) renders the
        // apply pass
        let keep = x87::load(h.read(this + 0x122b0));
        if keep != 0.0 {
            let source = texture(h, h.read(this + 0x117c));
            let dest: u32 = h.read(this + 0x8c);
            copy_buffer(h, this, renderer, source, dest);
            return 0;
        }
        let error = pass(
            h,
            ebp,
            SURFACE_APPLY,
            this,
            renderer,
            0xac,
            PASS_APPLY,
            false,
        );
        if error == 0 {
            let source = texture(h, h.read(this + 0xac));
            let dest: u32 = h.read(this + 0x8c);
            copy_buffer(h, this, renderer, source, dest);
        }
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(ebp - SURFACE_APPLY), &[]);
        error
    }
}
