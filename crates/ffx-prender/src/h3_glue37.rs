//! The render-side glue, thirty-seventh batch (M4 port round 8, list
//! `h3-glue-render`): the post-process manager's frame. The exception
//! frame and cookie are left out, as in [`crate::glue10`].

/// `PArray_Data_004d45a0(array)`, `ShaderParams_FindByName(shader;
/// name)` (`ret 4`), the texture parameter `0x0066e4f0(buffer; param,
/// record)` and `ShaderParamBuffer_SetValue(buffer; param, src)` (`ret
/// 8`), the three parameter names and 2/3 as a float.
pub const ARRAY_DATA: u32 = 0x004d_45a0;
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
pub const SET_TEXTURE: u32 = 0x0066_e4f0;
pub const SET_VALUE: u32 = 0x0063_8780;
pub const COLOR_BUFFER: u32 = 0x00b4_25d8;
pub const COLOR_CORRECTION_BUFFER: u32 = 0x00b4_7a20;
pub const UV_SCALE_BIAS: u32 = 0x00b4_25e4;
pub const TWO_THIRDS: u32 = 0x00b2_c5c4;

/// The render surface's constructor and destructor, the renderer's
/// `setColorTarget` (`ret 8`), `setDepthStencilTarget`, `beginScene`,
/// `endScene` (`ret 4`), `setViewport` and `setClearColor` (`ret 0x10`),
/// `renderMeshInstanceForPass(this; instance)` (`ret 4`) and the pass
/// word stored in the renderer's `+0x48`.
pub const SURFACE_CTOR: u32 = crate::h3_glue18::SURFACE_CTOR;
pub const SURFACE_DTOR: u32 = crate::h3_glue18::SURFACE_DTOR;
pub const SET_COLOR_TARGET: u32 = crate::h3_glue18::SET_COLOR_TARGET;
pub const SET_DEPTH_STENCIL_TARGET: u32 = 0x005b_b5b0;
pub const BEGIN_SCENE: u32 = crate::h3_glue18::BEGIN_SCENE;
pub const END_SCENE: u32 = crate::h3_glue18::END_SCENE;
pub const SET_VIEWPORT: u32 = 0x005b_c060;
pub const SET_CLEAR_COLOR: u32 = 0x005b_ada0;
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a70;
pub const PASS_WORD: u32 = 0x00cc_c8ac;

/// `PostProcessManager_prepareGlowMap(this; renderer, map, source)`
/// (`ret 0xc`), the tone pass `0x00a1ca10(this; renderer, a, source,
/// target, b)` (`ret 0x14`), `PostProcessManager_copyBuffer(this;
/// renderer, source, b, 1, 1, 0)` (`ret 0x18`) and
/// `PLegacyGlowGPUBase_render(this; renderer, b, glow, 0, 0)` (`ret
/// 0x14`).
pub const PREPARE_GLOW_MAP: u32 = 0x0069_4c60;
pub const TONE_PASS: u32 = 0x00a1_ca10;
pub const COPY_BUFFER: u32 = 0x0069_2c40;
pub const GLOW_RENDER: u32 = 0x00a1_de20;

pub mod bridge {
    //! The originals over game memory.

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

    /// A float as `fld dword` / `fstp dword` copy it.
    fn copied(h: &impl Host, at: u32) -> u32 {
        x87::store(x87::load(h.read(at)))
    }

    /// `PostProcessManager_render(this, renderer, a, b; slots)` (thiscall,
    /// `ret 0xc`; frame: a render surface at `ebp - 0x3c`, a parameter
    /// record at `- 0x2c`, the targets' textures at `- 0x1c` .. `- 0x10`).
    /// Without settings (`+0`): 1; not ready (`+0x123b4`): 0x17. The
    /// colour correction (`+0x123b5`, `+0x123b0` and its table `+0x1200`)
    /// renders the mesh `+0x1204` into target `+0x9c` with the colour
    /// buffer, the table and the UV scale; the glow map (`+0x123b8`,
    /// `+0x123b3`) is prepared; the tone pass (`+0x123b7`, `+0x123b2`)
    /// runs, else the corrected image is copied to `b`; the glow
    /// (unless `+0x123d1` and `+0x1260`) is rendered with the settings'
    /// clear colour restored. Returns the first error, or 0. The
    /// renderer's slot takes the shader, then the slots the colour.
    pub fn post_process_render(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        this: u32,
        renderer: u32,
        a: u32,
        b: u32,
    ) -> u32 {
        if h.read::<u32>(this) == 0 {
            return 1;
        }
        if h.read::<u8>(this + 0x123b4) == 0 {
            return 0x17;
        }
        let r = Conv::Thiscall(renderer);
        h.write(ebp - 0x10, 0u32);
        h.write(ebp - 0x14, 0u32);
        let colour = texture(h, h.read(this + 0x8c));
        h.write(ebp - 0x18, colour);
        let tone_target = texture(h, h.read(this + 0x90));
        h.write(ebp - 0x1c, tone_target);
        let correct = h.read::<u8>(this + 0x123b5) != 0;
        if correct {
            let corrected = texture(h, h.read(this + 0x9c));
            h.write(ebp - 0x10, corrected);
        }
        if h.read::<u8>(this + 0x123b8) != 0 {
            let glow = texture(h, h.read(this + 0xa4));
            h.write(ebp - 0x14, glow);
        }
        let mut source = colour;
        if correct && h.read::<u8>(this + 0x123b0) != 0 && h.read::<u32>(this + 0x1200) != 0 {
            let error = color_correction(h, ebp, slots, this, renderer, colour);
            if error != 0 {
                return error;
            }
            source = h.read(ebp - 0x10);
        }
        if h.read::<u8>(this + 0x123b8) != 0 && h.read::<u8>(this + 0x123b3) != 0 {
            let map: u32 = h.read(this + 0xa4);
            h.call_sym(
                PREPARE_GLOW_MAP,
                Conv::Thiscall(this),
                &[renderer, map, source],
            );
        }
        let mut copy = true;
        if h.read::<u8>(this + 0x123b7) != 0 {
            if h.read::<u8>(this + 0x123b2) != 0 {
                let settings: u32 = h.read(this);
                for k in 0..3 {
                    let v = copied(h, settings + 0x90 + 4 * k);
                    h.write(this + 0x154 + 4 * k, v);
                }
                let target: u32 = h.read(ebp - 0x1c);
                h.call_sym(
                    TONE_PASS,
                    Conv::Thiscall(this + 0x140),
                    &[renderer, a, source, target, b],
                );
            }
            copy = !(h.read::<u8>(this + 0x123b7) != 0 && h.read::<u8>(this + 0x123b2) != 0);
        }
        if copy
            && h.read::<u8>(this + 0x123b5) != 0
            && h.read::<u8>(this + 0x123b0) != 0
            && h.read::<u32>(this + 0x1200) != 0
        {
            h.call_sym(
                COPY_BUFFER,
                Conv::Thiscall(this),
                &[renderer, source, b, 1, 1, 0],
            );
        }
        if h.read::<u8>(this + 0x123b8) == 0
            || h.read::<u8>(this + 0x123b3) == 0
            || (h.read::<u8>(this + 0x123d1) != 0 && h.read::<u8>(this + 0x1260) != 0)
        {
            return 0;
        }
        let settings: u32 = h.read(this);
        for k in 0..2 {
            let v = copied(h, settings + 0x88 + 4 * k);
            h.write(this + 0x1154 + 4 * k, v);
        }
        let error = h.call_sym(SET_CLEAR_COLOR, r, &[0; 4]);
        if error != 0 {
            return error;
        }
        let glow: u32 = h.read(ebp - 0x14);
        h.call_sym(
            GLOW_RENDER,
            Conv::Thiscall(this + 0x1100),
            &[renderer, b, glow, 0, 0],
        );
        // the settings' clear colour, through the argument slots
        let settings: u32 = h.read(this);
        for (slot, at) in [(8, 0x38), (0, 0x34), (4, 0x30)] {
            let v = copied(h, settings + at);
            h.write(slots + slot, v);
        }
        let colour: [u32; 4] = [
            copied(h, slots + 4),
            copied(h, slots),
            copied(h, slots + 8),
            1f32.to_bits(),
        ];
        h.call_sym(SET_CLEAR_COLOR, r, &colour)
    }

    /// A texture parameter record `{0xff, 0, 0, texture}` at `ebp -
    /// 0x2c` for `param`, set on `buffer`.
    fn set_texture(h: &mut impl Host, ebp: u32, buffer: u32, param: u32, texture: u32) -> u32 {
        let record = ebp - 0x2c;
        h.write(record + 0xc, texture);
        h.write(record, 0xffu32);
        h.write(record + 8, 0u32);
        h.write(record + 4, 0u32);
        h.call_sym(SET_TEXTURE, Conv::Thiscall(buffer), &[param, record])
    }

    /// The colour correction pass (see [`post_process_render`]); its
    /// first error, or 0.
    fn color_correction(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        this: u32,
        renderer: u32,
        colour: u32,
    ) -> u32 {
        let mesh: u32 = h.read(this + 0x1204);
        let materials: u32 = h.read(mesh + 0x10);
        let first = h.call_sym(ARRAY_DATA, Conv::Thiscall(materials), &[]);
        let first: u32 = h.read(first);
        let shader: u32 = h.read(first);
        h.write(slots, shader);
        let first = h.call_sym(ARRAY_DATA, Conv::Thiscall(materials), &[]);
        let first: u32 = h.read(first);
        let buffer: u32 = h.read(first + 4);
        let name = h.rebase(COLOR_BUFFER);
        let param = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if param != 0 {
            let error = set_texture(h, ebp, buffer, param, colour);
            if error != 0 {
                return error;
            }
        }
        let shader: u32 = h.read(slots);
        let name = h.rebase(COLOR_CORRECTION_BUFFER);
        let param = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if param != 0 {
            let table: u32 = h.read(this + 0x1200);
            let error = set_texture(h, ebp, buffer, param, table);
            if error != 0 {
                return error;
            }
        }
        let shader: u32 = h.read(slots);
        let name = h.rebase(UV_SCALE_BIAS);
        let param = h.call_sym(FIND_PARAMETER, Conv::Thiscall(shader), &[name]);
        if param != 0 {
            let record = ebp - 0x2c;
            let scale = if h.read::<u8>(this + 0x123d9) == 0 {
                1f32.to_bits()
            } else {
                copied(h, h.rebase(TWO_THIRDS))
            };
            h.write(record, scale);
            h.write(record + 4, scale);
            h.write(record + 8, 0f32);
            h.write(record + 0xc, 0f32);
            let error = h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[param, record]);
            if error != 0 {
                return error;
            }
        }
        let surface = ebp - 0x3c;
        let target: u32 = h.read(this + 0x9c);
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[target, 0]);
        let error = render_pass(h, this, renderer, surface);
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
        error
    }

    /// The colour correction's draw into the surface: target, viewport,
    /// clear, the mesh; the first error, or 0.
    fn render_pass(h: &mut impl Host, this: u32, renderer: u32, surface: u32) -> u32 {
        let r = Conv::Thiscall(renderer);
        let error = h.call_sym(SET_COLOR_TARGET, r, &[surface, 0]);
        if error != 0 {
            return error;
        }
        let error = h.call_sym(SET_DEPTH_STENCIL_TARGET, r, &[0]);
        if error != 0 {
            return error;
        }
        let target: u32 = h.read(this + 0x9c);
        let tall = texture(h, target);
        let wide = texture(h, target);
        let width: u32 = h.read(wide + 0x1c);
        let height: u32 = h.read(tall + 0x20);
        h.call_sym(SET_VIEWPORT, r, &[0, 0, width, height]);
        let error = h.call_sym(SET_CLEAR_COLOR, r, &[0; 4]);
        if error != 0 {
            return error;
        }
        let error = h.call_sym(BEGIN_SCENE, r, &[1]);
        if error != 0 {
            return error;
        }
        let pass = h.rebase(PASS_WORD);
        h.write(renderer + 0x48, pass);
        let mesh: u32 = h.read(this + 0x1204);
        let error = h.call_sym(RENDER_MESH_INSTANCE, r, &[mesh]);
        if error != 0 {
            return error;
        }
        h.call_sym(END_SCENE, r, &[1])
    }
}
