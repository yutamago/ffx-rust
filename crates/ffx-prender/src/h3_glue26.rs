//! The render-side glue, twenty-sixth batch (M4 port round 7, list
//! `h3-glue-render`): post-process passes in the shape of
//! [`crate::h3_glue21`]'s full-screen texture pass. The exception frame
//! and cookie are left out, as in [`crate::glue10`]; a render surface in
//! the frame is destroyed on every path, as the original's unwinding does.

pub use crate::h3_glue21::{
    BEGIN_SCENE, BIND_TEXTURE, COLOR_BUFFER, END_SCENE, FIND_PARAMETER, FULL_SCREEN_PASS,
    GET_APPLICATION, RENDER_MESH_INSTANCE, SET_COLOR_TARGET, SET_DEPTH_STENCIL_TARGET,
    SET_NON_16_BY_9_VIEWPORT, SET_VIEWPORT, SURFACE_CTOR, SURFACE_DTOR,
};

/// `"UvScaleBias"`, `ShaderParamBuffer_SetValue(buffer, parameter,
/// value)` (thiscall, `ret 8`) and the narrow scale 2/3 (`f32`).
pub const UV_SCALE_BIAS: u32 = 0x00b4_25e4;
pub const SET_VALUE: u32 = 0x0063_8780;
pub const NARROW_SCALE: u32 = 0x00b2_c5c4;

/// `"ColorScale"` and `memcpy(dst, src, n)` (cdecl).
pub const COLOR_SCALE: u32 = 0x00b4_7a48;
pub const MEMCPY: u32 = crate::glue5::MEMCPY;

/// The bright pass: `"g_brightPara"`, `"g_BrightTex"`, the `f64`s 50 and
/// 100 its parameter is scaled by, `setClearColor(this, r, g, b, a)`
/// (`ret 0x10`), the bright pass entry and
/// `PostProcessManager_copyBuffer(this, renderer, texture, target, wide,
/// _, mode2)` (`ret 0x18`).
pub const BRIGHT_PARA: u32 = 0x00b4_7934;
pub const BRIGHT_TEX: u32 = 0x00b4_7944;
pub const FIFTY: u32 = 0x00b1_f2c0;
pub const HUNDRED: u32 = 0x00b9_22e0;
pub const SET_CLEAR_COLOR: u32 = crate::glue6::RENDERER_SET_CLEAR_COLOR;
pub const BRIGHT_PASS: u32 = 0x00cc_ca3c;
pub const COPY_BUFFER: u32 = 0x0069_2c40;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `PArray`-style element pointer: the data `+4` with a count over 1,
    /// the inline element `+4` with 1, none with 0.
    fn array_data(h: &impl Host, array: u32) -> u32 {
        match h.read::<u32>(array) & 0x7fff_ffff {
            0 => 0,
            1 => array + 4,
            _ => h.read(array + 4),
        }
    }

    /// A render target's texture: `*target` unless its byte `+4` is set.
    fn target_texture(h: &impl Host, target: u32) -> u32 {
        if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        }
    }

    /// A float constant as `fld m32` / `fst m32` copy it.
    fn copied(bits: u32) -> u32 {
        x87::store(x87::load(bits))
    }

    /// The full-screen mesh's (`+0x1208`) first material: its shader and
    /// its parameter buffer.
    fn full_screen_material(h: &impl Host, this: u32) -> (u32, u32) {
        let mesh: u32 = h.read(this + 0x1208);
        let materials: u32 = h.read(mesh + 0x10);
        let material: u32 = h.read(array_data(h, materials));
        let shader: u32 = h.read(material);
        let material: u32 = h.read(array_data(h, materials));
        (shader, h.read(material + 4))
    }

    /// `texture` as the material's `"ColorBuffer"` (the record `{0xff, 0,
    /// this + 0x123dc, texture}` at `ebp - 0x20`): the binder's error.
    fn bind_color_buffer(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        shader: u32,
        buffer: u32,
        texture: u32,
    ) -> u32 {
        let name = h.rebase(COLOR_BUFFER);
        let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if parameter == 0 {
            return 0;
        }
        let record = ebp - 0x20;
        h.write(ebp - 0x14, texture);
        h.write(ebp - 0x18, this + 0x1_23dc);
        h.write(record, 0xffu32);
        h.write(ebp - 0x1c, 0u32);
        h.call_sym(BIND_TEXTURE, Conv::Thiscall(buffer), &[parameter, record])
    }

    /// `{s, s, 0, 0}` as the material's `"UvScaleBias"` (the record at
    /// `ebp - 0x20`; `s` 2/3 when `wide`'s low byte and `+0x123d9` are
    /// set, else 1).
    fn set_uv_scale(h: &mut impl Host, ebp: u32, this: u32, shader: u32, buffer: u32, wide: u32) {
        let name = h.rebase(UV_SCALE_BIAS);
        let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if parameter == 0 {
            return;
        }
        let record = ebp - 0x20;
        let one = copied(1.0f32.to_bits());
        h.write(record, one);
        h.write(ebp - 0x1c, one);
        h.write(ebp - 0x18, 0u32);
        h.write(ebp - 0x14, 0u32);
        if wide as u8 != 0 && h.read::<u8>(this + 0x1_23d9) != 0 {
            let at = h.rebase(NARROW_SCALE);
            let scale = copied(h.read(at));
            h.write(record, scale);
            h.write(ebp - 0x1c, scale);
        }
        h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[parameter, record]);
    }

    /// The targets: a surface over `target` (at `ebp - 0x30`, destroyed
    /// after) or the back buffer, no second colour target, no depth
    /// target; then the viewport of the target's texture (or the
    /// application's size). The first error.
    fn set_targets(h: &mut impl Host, ebp: u32, renderer: u32, target: u32) -> u32 {
        if target == 0 {
            let error = h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[0, 0]);
            if error != 0 {
                return error;
            }
        } else {
            let surface = ebp - 0x30;
            h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[target, 0]);
            let error = h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[surface, 0]);
            h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
            if error != 0 {
                return error;
            }
        }
        let error = h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[0, 1]);
        if error != 0 {
            return error;
        }
        let error = h.call_sym(SET_DEPTH_STENCIL_TARGET, Conv::Thiscall(renderer), &[0]);
        if error != 0 {
            return error;
        }
        let (w, ht) = if target == 0 {
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            let ht: u32 = h.read(app + 0x270);
            let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            (h.read(app + 0x26c), ht)
        } else {
            let a = target_texture(h, target);
            let b = target_texture(h, target);
            let ht: u32 = h.read(a.wrapping_add(0x20));
            (h.read(b.wrapping_add(0x1c)), ht)
        };
        h.call_sym(SET_VIEWPORT, Conv::Thiscall(renderer), &[0, 0, w, ht]);
        0
    }

    /// The full-screen mesh drawn with `pass`, then `endScene(1)`'s
    /// result.
    fn draw_full_screen(h: &mut impl Host, this: u32, renderer: u32, pass: u32) -> u32 {
        let context: u32 = h.read(renderer + 0x13c);
        h.write(renderer + 0x48, pass);
        let mesh: u32 = h.read(this + 0x1208);
        let data: u32 = h.read(mesh + 0x10);
        h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(renderer),
            &[mesh, data, 0, context],
        );
        h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1])
    }

    /// `PostProcessManager_copyBuffer(this, renderer, texture, target,
    /// wide, _, mode2)` (`ret 0x18`; frame: the surface at `ebp - 0x30`,
    /// the record at `ebp - 0x20`, `this` at `ebp - 0x10`): the
    /// full-screen mesh's first material gets `texture` as its
    /// `"ColorBuffer"` and the `"UvScaleBias"`; then the targets, a scene
    /// begun with 2 (`mode2`) or 1, the 16:9 fit and the mesh drawn with
    /// the full-screen pass. `eax`: the first error, or `endScene`'s.
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn post_copy_buffer(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        renderer: u32,
        texture: u32,
        target: u32,
        wide: u32,
        mode2: u32,
    ) -> u32 {
        h.write(ebp - 0x10, this);
        let (shader, buffer) = full_screen_material(h, this);
        let error = bind_color_buffer(h, ebp, this, shader, buffer, texture);
        if error != 0 {
            return error;
        }
        set_uv_scale(h, ebp, this, shader, buffer, wide);
        let error = set_targets(h, ebp, renderer, target);
        if error != 0 {
            return error;
        }
        let mode = if mode2 as u8 == 0 { 1 } else { 2 };
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[mode]);
        if error != 0 {
            return error;
        }
        let this: u32 = h.read(ebp - 0x10);
        h.call_sym(
            SET_NON_16_BY_9_VIEWPORT,
            Conv::Thiscall(this),
            &[renderer, target],
        );
        let pass = h.rebase(FULL_SCREEN_PASS);
        draw_full_screen(h, this, renderer, pass)
    }

    /// `PostProcessManager_copyDepthBuffer(this, renderer, texture,
    /// target, pass, colour, wide, _, clear_a, clear_b; slots)` (`ret
    /// 0x24`; the frame as [`post_copy_buffer`]'s): as `copyBuffer`, with
    /// the `"ColorScale"` parameter between: none given returns 1, one out
    /// of its buffer (offset under 4, or past the buffer's size) returns 3,
    /// else `colour` is copied in (the offset goes over `texture`'s slot);
    /// the scene begins with `clear_a | clear_b << 1` (their low bytes as
    /// flags) and the mesh is drawn with `pass`, without the 16:9 fit.
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn post_copy_depth_buffer(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        this: u32,
        renderer: u32,
        texture: u32,
        target: u32,
        pass: u32,
        colour: u32,
        wide: u32,
        clear_a: u32,
        clear_b: u32,
    ) -> u32 {
        h.write(ebp - 0x10, this);
        let (shader, buffer) = full_screen_material(h, this);
        let error = bind_color_buffer(h, ebp, this, shader, buffer, texture);
        if error != 0 {
            return error;
        }
        let name = h.rebase(COLOR_SCALE);
        let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if parameter != 0 {
            if colour == 0 {
                return 1;
            }
            let offset = u32::from(h.read::<u16>(parameter + 8));
            let size = u32::from(h.read::<u16>(parameter + 0xa) & 0x1fff);
            h.write(slots + 4, offset);
            if offset + size > h.read::<u32>(buffer) || offset < 4 {
                return 3;
            }
            h.call_sym(
                MEMCPY,
                Conv::Cdecl,
                &[buffer.wrapping_add(offset), colour, size],
            );
        }
        set_uv_scale(h, ebp, this, shader, buffer, wide);
        let error = set_targets(h, ebp, renderer, target);
        if error != 0 {
            return error;
        }
        let mode = u32::from(clear_a as u8 != 0) | u32::from(clear_b as u8 != 0) << 1;
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[mode]);
        if error != 0 {
            return error;
        }
        let this: u32 = h.read(ebp - 0x10);
        draw_full_screen(h, this, renderer, pass)
    }

    /// `PostProcessManager_renderBright(this, renderer, target)` (`ret 8`;
    /// frame: the surface at `ebp - 0x34`, the texture record at `ebp -
    /// 0x24`, the offset at `ebp - 0x14`, the brightness at `ebp - 0x10`):
    /// with both enable bytes (`+0x123d7`, `+0x123d6`), the bright mesh's
    /// (`+0x12320`) first material gets `(+0x12328 - 50) / 100` (rounded
    /// to `f32`) as `"g_brightPara"` (none when out of its buffer) and the
    /// target's texture as `"g_BrightTex"`; then a surface over `+0xa4` is
    /// the target, no depth target, its texture's viewport, a black clear,
    /// a scene begun with 0, the mesh drawn with the bright pass and the
    /// scene ended; on success the result goes back through
    /// `copyBuffer(this, renderer, texture, target, 1, 0, 0)`. The surface
    /// is destroyed after. Callers read no result.
    pub fn post_render_bright(h: &mut impl Host, ebp: u32, this: u32, renderer: u32, target: u32) {
        if h.read::<u8>(this + 0x1_23d7) == 0 || h.read::<u8>(this + 0x1_23d6) == 0 {
            return;
        }
        let mesh: u32 = h.read(this + 0x1_2320);
        let materials: u32 = h.read(mesh + 0x10);
        let material: u32 = h.read(array_data(h, materials));
        let shader: u32 = h.read(material);
        let material: u32 = h.read(array_data(h, materials));
        let buffer: u32 = h.read(material + 4);
        let name = h.rebase(BRIGHT_PARA);
        let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if parameter != 0 {
            let fifty = h.rebase(FIFTY);
            let fifty = f64::from_bits(h.read::<u64>(fifty));
            let hundred = h.rebase(HUNDRED);
            let hundred = f64::from_bits(h.read::<u64>(hundred));
            let level = x87::load(h.read(this + 0x1_2328));
            let level = x87::div(x87::sub(level, fifty), hundred);
            let offset = u32::from(h.read::<u16>(parameter + 8));
            let size = u32::from(h.read::<u16>(parameter + 0xa) & 0x1fff);
            h.write(ebp - 0x14, offset);
            h.write(ebp - 0x10, x87::store(level));
            if offset + size > h.read::<u32>(buffer) || offset < 4 {
                return;
            }
            h.call_sym(
                MEMCPY,
                Conv::Cdecl,
                &[buffer.wrapping_add(offset), ebp - 0x10, size],
            );
        }
        let name = h.rebase(BRIGHT_TEX);
        let parameter = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if parameter != 0 {
            let record = ebp - 0x24;
            h.write(record, 0xffu32);
            let texture = target_texture(h, target);
            h.write(ebp - 0x18, texture);
            h.write(ebp - 0x1c, 0u32);
            h.write(ebp - 0x20, 0u32);
            let error = h.call_sym(BIND_TEXTURE, Conv::Thiscall(buffer), &[parameter, record]);
            if error != 0 {
                return;
            }
        }
        let surface = ebp - 0x34;
        let own: u32 = h.read(this + 0xa4);
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[own, 0]);
        if bright_pass(h, this, renderer, surface) == 0 {
            let own: u32 = h.read(this + 0xa4);
            let texture = target_texture(h, own);
            h.call_sym(
                COPY_BUFFER,
                Conv::Thiscall(this),
                &[renderer, texture, target, 1, 0, 0],
            );
        }
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
    }

    /// The bright pass's draw into `surface`: the first error.
    fn bright_pass(h: &mut impl Host, this: u32, renderer: u32, surface: u32) -> u32 {
        let error = h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[surface, 0]);
        if error != 0 {
            return error;
        }
        let error = h.call_sym(SET_DEPTH_STENCIL_TARGET, Conv::Thiscall(renderer), &[0]);
        if error != 0 {
            return error;
        }
        let own: u32 = h.read(this + 0xa4);
        let a = target_texture(h, own);
        let b = target_texture(h, own);
        let ht: u32 = h.read(a.wrapping_add(0x20));
        let w: u32 = h.read(b.wrapping_add(0x1c));
        h.call_sym(SET_VIEWPORT, Conv::Thiscall(renderer), &[0, 0, w, ht]);
        let zero = x87::store(0.0);
        let error = h.call_sym(SET_CLEAR_COLOR, Conv::Thiscall(renderer), &[zero; 4]);
        if error != 0 {
            return error;
        }
        let error = h.call_sym(BEGIN_SCENE, Conv::Thiscall(renderer), &[0]);
        if error != 0 {
            return error;
        }
        let context: u32 = h.read(renderer + 0x13c);
        let pass = h.rebase(BRIGHT_PASS);
        h.write(renderer + 0x48, pass);
        let mesh: u32 = h.read(this + 0x1_2320);
        let data: u32 = h.read(mesh + 0x10);
        let error = h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(renderer),
            &[mesh, data, 0, context],
        );
        if error != 0 {
            return error;
        }
        h.call_sym(END_SCENE, Conv::Thiscall(renderer), &[1])
    }
}
