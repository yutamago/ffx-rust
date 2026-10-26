//! Post anti-aliasing and the D3D11 renderer's resolve (M4 coverage 2,
//! list `c2-new`): what the `settings-fxaa`, `settings-smaa` and
//! `settings-msaa` replays reach and the default settings do not.
//!
//! - `PostProcessManager_renderFXAA` and the FXAA pass it reaches through
//!   the forwarder `0x00a20350` (the pass object at the manager's
//!   `+0x1190`);
//! - the shader-parameter store `0x00698900` that
//!   `PostProcessManager_renderSMAA` uses for its integer parameters;
//! - `0x00593e80`, the render-target array's element word that
//!   `PRendererD3D11_resolveTargets` reads with MSAA on.
//!
//! The FXAA pass sets up a C++ exception frame (`fs:[0]`) and a security
//! cookie; nothing it calls throws, so the port leaves both out, as
//! [`crate::glue10`]'s loaders do.

use ffx_bridge::x87;

/// `g_CurrentMapNo`.
pub const CURRENT_MAP: u32 = 0x012f_bc40;
/// The FXAA forwarder and pass.
pub const FXAA_FORWARD: u32 = 0x00a2_0350;
pub const FXAA_PASS: u32 = 0x00a2_0360;
/// `TrophyManager_Install(this; arg)` (returns 0 on PC; the parameter
/// buffers call it before binding a texture).
pub const TROPHY_INSTALL: u32 = 0x0059_e520;
/// `PRenderSurface` constructor `(this; surface, 0)` and destructor
/// `(this)`.
pub const SURFACE_CTOR: u32 = 0x005b_35c0;
pub const SURFACE_DTOR: u32 = 0x005b_3640;
/// `PRenderer` methods on the renderer: `setColorTarget(target, slot)`,
/// `setDepthStencilTarget(target)`, `beginScene(flags)`,
/// `renderMeshInstance(mesh, data, 0, context)`, `endScene(flags)`.
pub const SET_COLOR_TARGET: u32 = 0x005b_b130;
pub const SET_DEPTH_STENCIL: u32 = 0x005b_b5b0;
pub const BEGIN_SCENE: u32 = 0x005b_5e30;
pub const RENDER_MESH_INSTANCE: u32 = 0x005b_9a90;
pub const END_SCENE: u32 = 0x005b_6950;
/// `PParameterBuffer_setTextureChecked(this; desc, record)`.
pub const SET_TEXTURE_CHECKED: u32 = 0x0056_e670;
/// The two FXAA pass contexts the renderer's `+0x48` gets: with the
/// intermediate target (luma pass) and the final one.
pub const FXAA_CONTEXT_FINAL: u32 = 0x0194_49c4;
pub const FXAA_CONTEXT_LUMA: u32 = 0x0194_49d4;
/// `2^32` as `f32` (`0x4f800000`): added to an unsigned word that `fild`
/// read as negative.
pub const TWO_32: u32 = 0x00b1_1f14;
/// `PSharray` element `i` (`this; i`), 0x1c-byte elements.
pub const SHARRAY_ELEMENT28: u32 = 0x0058_5c90;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn r32(h: &impl Host, at: u32) -> u32 {
        h.read(at)
    }

    /// A `PRenderSurface`-like handle's texture: its first word unless its
    /// byte `+4` is set (then 0).
    fn surface_texture(h: &impl Host, s: u32) -> u32 {
        if h.read::<u8>(s + 4) == 0 {
            r32(h, s)
        } else {
            0
        }
    }

    /// `memcpy(dst, src, n)` (the CRT's, which copies overlapping ranges
    /// as `memmove`).
    fn memcpy(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let mut buf = vec![0u8; n as usize];
        h.read_bytes(src, &mut buf);
        h.write_bytes(dst, &buf);
    }

    /// The parameter store of the shader buffers: `n` bytes at `src` into
    /// `buffer` at the parameter's offset (`desc` `+8`, `+0xa` & 0x1fff the
    /// size): 3 when it ends past the buffer's size word or starts inside
    /// it, else 0.
    fn store_parameter(h: &mut impl Host, buffer: u32, desc: u32, src: u32) -> u32 {
        let at = u32::from(h.read::<u16>(desc + 8));
        let size = u32::from(h.read::<u16>(desc + 0xa) & 0x1fff);
        if at + size > r32(h, buffer) || at < 4 {
            return 3;
        }
        memcpy(h, buffer.wrapping_add(at), src, size);
        0
    }

    /// `FUN_00698900(buffer; desc, src)` (`ret 8`): 1 for a null `src`,
    /// else the parameter store (0, or 3 out of range).
    pub fn set_value_checked(h: &mut impl Host, buffer: u32, desc: u32, src: u32) -> u32 {
        if src == 0 {
            return 1;
        }
        store_parameter(h, buffer, desc, src)
    }

    /// `FUN_00593e80(this; i)` (`ret 4`): the first word of element `i`
    /// of the 0x1c-byte `PSharray` at `+0x24` when `i` is below its count,
    /// else 0.
    pub fn element_word(h: &mut impl Host, this: u32, i: u32) -> u32 {
        if i < r32(h, this + 0x24) & 0x7fff_ffff {
            let e = h.call_sym(SHARRAY_ELEMENT28, Conv::Thiscall(this + 0x24), &[i]);
            r32(h, e)
        } else {
            0
        }
    }

    /// `PostProcessManager_renderFXAA(this; renderer, target)` (`ret 8`)
    /// -> 0: with the FXAA shader loaded (`+0x123b9`) and the map not 0x17
    /// or 0x15c, the pass (`0x00a20350`, on `+0x1190`) from the colour
    /// buffer (`+0x8c`'s texture) into `target`, with the luma pre-pass.
    pub fn render_fxaa(h: &mut impl Host, this: u32, renderer: u32, target: u32) -> u32 {
        let colour = surface_texture(h, r32(h, this + 0x8c));
        if h.read::<u8>(this + 0x123b9) != 0 {
            let map = r32(h, h.rebase(CURRENT_MAP));
            if map != 0x17 && map != 0x15c {
                h.call_sym(
                    FXAA_FORWARD,
                    Conv::Thiscall(this + 0x1190),
                    &[renderer, target, colour, 1],
                );
            }
        }
        0
    }

    /// `FUN_00a20350(this; renderer, target, source, luma)` (`ret 0x10`):
    /// jumps to the pass (`0x00a20360`); returns its `eax`.
    pub fn fxaa_forward(
        h: &mut impl Host,
        this: u32,
        renderer: u32,
        target: u32,
        source: u32,
        luma: u32,
    ) -> u32 {
        h.call_sym(
            FXAA_PASS,
            Conv::Thiscall(this),
            &[renderer, target, source, luma],
        )
    }

    /// `1 / v` of an unsigned word as the x87 computes it: `fild`, `2^32`
    /// added when negative, rounded to `f32`, then divided at 53 bits.
    fn reciprocal(h: &impl Host, v: u32) -> u32 {
        let mut x = f64::from(v.cast_signed());
        if v.cast_signed() < 0 {
            x = x87::add(x, x87::load(r32(h, h.rebase(TWO_32))));
        }
        let x = x87::load(x87::store(x));
        x87::store(x87::div(1.0, x))
    }

    /// `FUN_00a20360(this; renderer, target, source, luma)` (`ret 0x10`,
    /// frame at `ebp`): the FXAA pass. First the shader's parameters: the
    /// input texture (`+0x68`, a texture-type descriptor: else 5; record
    /// `{0x42, 0, sampler +0x38, source}` at `ebp - 0x20`, after
    /// `TrophyManager_Install(sampler; source)`) and the inverse size of
    /// `source` (`+0x1c`/`+0x20`; `+0x6c`, `{1/w, 1/h, 0, 0}` at `ebp -
    /// 0x30`), into the buffer `[+0x64]+4` (3 when out of range). With
    /// `luma`, the luma pass draws the mesh `+0x60` into the work target
    /// `+0x5c` (context `0x019449d4`), whose texture then becomes the
    /// input (`setTextureChecked`, record at `ebp - 0x40`). Then the
    /// final pass into `target` (none: the back buffer) with context
    /// `0x019449c4`, and the colour target is reset. The first non-zero
    /// result of a step is returned.
    ///
    /// The parameter records are copied from the frame with the
    /// descriptor's size, as the original does; the shader's two
    /// parameters are 16 bytes (a larger size would copy the original's
    /// exception registration and cookie, which the port does not have).
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)] // as the original
    pub fn fxaa_pass(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        renderer: u32,
        target: u32,
        source: u32,
        luma: u32,
    ) -> u32 {
        let texture = r32(h, this + 0x68);
        if texture != 0 {
            let buffer = r32(h, r32(h, this + 0x64) + 4);
            h.write(ebp - 0x10, buffer);
            let kind = h.read::<u8>(texture + 2);
            h.write(ebp - 0x20, 0x42u32);
            h.write(ebp - 0x1c, 0u32);
            h.write(ebp - 0x14, source);
            h.write(ebp - 0x18, this + 0x38);
            if ![0x42, 0xd9, 0xda, 0xdb, 0xdc, 0x12].contains(&kind) {
                return 5;
            }
            let sampler = this.wrapping_add(0x38);
            if sampler != 0 && source != 0 {
                let r = h.call_sym(TROPHY_INSTALL, Conv::Thiscall(sampler), &[source]);
                if r != 0 {
                    return r;
                }
            }
            let r = store_parameter(h, buffer, texture, ebp - 0x20);
            if r != 0 {
                return r;
            }
        }
        let size = r32(h, this + 0x6c);
        if size != 0 {
            let w = reciprocal(h, r32(h, source + 0x1c));
            let ht = reciprocal(h, r32(h, source + 0x20));
            h.write(ebp - 0x30, w);
            h.write(ebp - 0x2c, ht);
            h.write(ebp - 0x28, 0u32);
            h.write(ebp - 0x24, 0u32);
            let buffer = r32(h, r32(h, this + 0x64) + 4);
            let r = store_parameter(h, buffer, size, ebp - 0x30);
            if r != 0 {
                return r;
            }
        }
        let this_r = Conv::Thiscall(renderer);
        if luma & 0xff != 0 {
            let surface = ebp - 0x50;
            let work = r32(h, this + 0x5c);
            h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[work, 0]);
            let r = luma_pass(h, ebp, this, renderer, surface);
            h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
            if r != 0 {
                return r;
            }
        }
        if target == 0 {
            let r = h.call_sym(SET_COLOR_TARGET, this_r, &[0, 0]);
            if r != 0 {
                return r;
            }
        } else {
            let surface = ebp - 0x60;
            h.call_sym(SURFACE_CTOR, Conv::Thiscall(surface), &[target, 0]);
            let r = h.call_sym(SET_COLOR_TARGET, this_r, &[surface, 0]);
            h.call_sym(SURFACE_DTOR, Conv::Thiscall(surface), &[]);
            if r != 0 {
                return r;
            }
        }
        let context = h.rebase(FXAA_CONTEXT_FINAL);
        let r = draw(h, this, renderer, context);
        if r != 0 {
            return r;
        }
        h.call_sym(SET_COLOR_TARGET, this_r, &[0, 0])
    }

    /// `setDepthStencilTarget(0)`, `beginScene(0)`, the context to the
    /// renderer's `+0x48`, the pass mesh (`+0x60`) and `endScene(1)`: the
    /// first non-zero result.
    fn draw(h: &mut impl Host, this: u32, renderer: u32, context: u32) -> u32 {
        let this_r = Conv::Thiscall(renderer);
        let r = h.call_sym(SET_DEPTH_STENCIL, this_r, &[0]);
        if r != 0 {
            return r;
        }
        let r = h.call_sym(BEGIN_SCENE, this_r, &[0]);
        if r != 0 {
            return r;
        }
        let pass_context = r32(h, renderer + 0x13c);
        h.write(renderer + 0x48, context);
        let mesh = r32(h, this + 0x60);
        let data = r32(h, mesh + 0x10);
        let r = h.call_sym(RENDER_MESH_INSTANCE, this_r, &[mesh, data, 0, pass_context]);
        if r != 0 {
            return r;
        }
        h.call_sym(END_SCENE, this_r, &[1])
    }

    /// The luma pass into the work target (`surface`, constructed), then
    /// its texture as the shader's input.
    fn luma_pass(h: &mut impl Host, ebp: u32, this: u32, renderer: u32, surface: u32) -> u32 {
        let r = h.call_sym(SET_COLOR_TARGET, Conv::Thiscall(renderer), &[surface, 0]);
        if r != 0 {
            return r;
        }
        let context = h.rebase(FXAA_CONTEXT_LUMA);
        let r = draw(h, this, renderer, context);
        if r != 0 {
            return r;
        }
        let texture = r32(h, this + 0x68);
        if texture == 0 {
            return 0;
        }
        let record = ebp - 0x40;
        h.write(record + 4, 0u32);
        h.write(record, 0x42u32);
        let work = surface_texture(h, r32(h, this + 0x5c));
        h.write(record + 0xc, work);
        h.write(record + 8, this + 0x38);
        let buffer = r32(h, r32(h, this + 0x64) + 4);
        h.call_sym(
            SET_TEXTURE_CHECKED,
            Conv::Thiscall(buffer),
            &[texture, record],
        )
    }
}
