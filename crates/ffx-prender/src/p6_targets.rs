//! List p6-render-c, continuation round 3 (batch 15): `PRenderer`'s
//! colour and depth-stencil targets, a light's attenuation factors, the
//! legacy glow's GPU targets and shaders, and a dynamic-geometry packet's
//! layout.
//!
//! Conventions are read from the disassembly; `eax` is returned wherever
//! the original defines it.

use ffx_bridge::x87::Reg;
use ffx_graphic::host::{Conv, Host};

/// `0x0059e510(target)` (thiscall, no words): the render target's check
/// (`PRenderTargetNVN::validate`; returns 0 here).
pub const VALIDATE: u32 = 0x0059_e510;
/// `PRenderTargetBase::getViewportSize(target; &w, &h)` (thiscall, `ret
/// 8`).
pub const VIEWPORT_SIZE: u32 = 0x004c_82b0;
/// `GetRenderStateCache()` (cdecl): the function-local static
/// `0x00c94f00`, built on first use; its back buffer's size at `+0x200`,
/// `+0x204`.
pub const RENDER_STATE_CACHE: u32 = 0x004f_d8b0;
/// `0x005b6a70(renderer; bytes, commands)` (thiscall, `ret 8`): `al` 1
/// when both rings have room.
pub const HAS_ROOM: u32 = 0x005b_6a70;
/// `PRenderRingBufferSegmented::allocate(ring; size)` (thiscall, `ret 4`).
pub const RING_ALLOCATE: u32 = 0x0061_3e10;
/// The colour and depth-stencil target commands' opcodes.
pub const SET_COLOR_OPCODE: u32 = 0xb;
pub const SET_DEPTH_OPCODE: u32 = 0xc;

/// The renderer's fields the target setters write.
pub mod renderer {
    /// The viewport: x, y (`+8`, `+0xc`), width and height (`+0x10`,
    /// `+0x14`).
    pub const VIEWPORT: u32 = 8;
    /// The four colour targets, 16 bytes each.
    pub const COLOR_TARGETS: u32 = 0xe8;
    /// The depth-stencil target, 16 bytes.
    pub const DEPTH_TARGET: u32 = 0x128;
    /// The bound colour targets' mask (`u8`).
    pub const COLOR_MASK: u32 = 0x138;
    /// Whether a depth-stencil target is bound (`u8`).
    pub const HAS_DEPTH: u32 = 0x139;
}

/// `_CIcos`' import thunk: `ST0` in and out.
pub const CI_COS: u32 = 0x0094_98c2;
/// `f64` 1e-6 (`fsub`/`fadd qword`): the cone angles' margin.
pub const ANGLE_EPSILON: u32 = 0x00b3_68a8;
/// `f64` -1.0.
pub const MINUS_ONE: u32 = 0x00b9_2350;
/// The two light types `0x005b5ef0` tells apart (`PLightType` vtables at
/// the light's `+0x18`): the spot light's (both factor pairs) and the
/// point light's (distance factors only).
pub const SPOT_TYPE: u32 = 0x00c1_0850;
pub const POINT_TYPE: u32 = 0x00c1_0814;

/// The legacy glow's GPU targets: per level two half-size targets made as
/// `p6_post::bridge::half_target` makes them, named by these strings.
pub const GLOW_X_NAME: u32 = 0x00b7_6b08;
pub const GLOW_Y_NAME: u32 = 0x00b7_6b24;

/// `PLegacyGlowBase::initializeShaders`' callees: the glow GPU effect
/// `0x00a06120(cluster)` (cdecl), `PEffectVariant::createMaterial(effect;
/// cluster, 0, 0)`, the parameter lookup `0x0056cbf0(effect; name)`, the
/// state array `PCluster::create(cluster; 1, type, 0)`, its word store
/// `0x0048c540(array; i, v)`, `ShaderParamBuffer::SetValue(buffer; param,
/// src)` and the texture registration `0x0058cf80(this)` (all thiscall).
pub const GLOW_GPU_EFFECT: u32 = 0x00a0_6120;
pub const CREATE_MATERIAL: u32 = 0x004e_6a40;
pub const FIND_PARAMETER: u32 = 0x0056_cbf0;
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
pub const STATE_ARRAY_TYPE: u32 = 0x00c9_98e8;
pub const SET_WORD: u32 = 0x0048_c540;
pub const SET_VALUE: u32 = 0x0063_8780;
pub const REGISTER: u32 = 0x0058_cf80;
pub const FREE: u32 = 0x0042_fb20;
/// The six parameter names, in the order looked up (fields `+0x60` to
/// `+0x74`): `GlowBuffer`, `GlowBufferState`, `GlowGain`,
/// `GaussianOutputScale`, `GaussianBlurBufferSize`, `UvScaleBias`.
pub const GLOW_PARAMETERS: [u32; 6] = [
    0x00b7_6a8c,
    0x00b7_6aec,
    0x00b7_6afc,
    0x00b7_6a98,
    0x00b4_94d8,
    0x00b4_25e4,
];

/// `initializeShaders`' frame: the four floats handed to `SetValue`
/// (`ebp - 0x1c`).
pub const SHADERS_FRAME: u32 = 0x10;
/// `setDepthStencilTarget`'s frame: the rings' wrapped cursors, the
/// command ring's at `ebp - 0x1c`, the data ring's at `ebp - 0x18`.
pub const DEPTH_FRAME: u32 = 8;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ANGLE_EPSILON, CI_COS, CLUSTER_CREATE, CREATE_MATERIAL, Conv, FIND_PARAMETER, FREE,
        GLOW_GPU_EFFECT, GLOW_PARAMETERS, GLOW_X_NAME, GLOW_Y_NAME, HAS_ROOM, Host, MINUS_ONE,
        POINT_TYPE, REGISTER, RENDER_STATE_CACHE, RING_ALLOCATE, Reg, SET_COLOR_OPCODE,
        SET_DEPTH_OPCODE, SET_VALUE, SET_WORD, SPOT_TYPE, STATE_ARRAY_TYPE, VALIDATE,
        VIEWPORT_SIZE, renderer,
    };
    use crate::p6_encode::bridge::full;
    use crate::p6_post::bridge::half_target;

    /// The viewport from the render state cache's back buffer size: `x`,
    /// `y` 0. `skip_null`: `setColorTarget` checks the cache's address
    /// (`+0x1f0` not 0) first.
    fn viewport_from_cache(h: &mut impl Host, r: u32, skip_null: bool) {
        let cache = h.call_sym(RENDER_STATE_CACHE, Conv::Cdecl, &[]);
        if skip_null && cache.wrapping_add(0x1f0) == 0 {
            return;
        }
        let w: u32 = h.read(cache.wrapping_add(0x200));
        let ht: u32 = h.read(cache.wrapping_add(0x204));
        h.write(r + renderer::VIEWPORT, 0u32);
        h.write(r + renderer::VIEWPORT + 4, 0u32);
        h.write(r + renderer::VIEWPORT + 8, w);
        h.write(r + renderer::VIEWPORT + 0xc, ht);
    }

    /// The command and its 16-byte payload block (only with a target):
    /// the frame's first command when there is none, its end; `opcode`;
    /// the payload's address at `+8` (0 without a target). 0, or 0xd when
    /// a ring has no room.
    fn finish(h: &mut impl Host, r: u32, target: u32, size: u32, opcode: u32) -> Option<u32> {
        let data = if target == 0 {
            0
        } else {
            let d = h.call_sym(RING_ALLOCATE, Conv::Thiscall(r + 0x1c), &[size]);
            if d == 0 {
                return None;
            }
            d
        };
        let cmd = h.call_sym(RING_ALLOCATE, Conv::Thiscall(r + 0x30), &[0x48]);
        if cmd == 0 {
            return None;
        }
        if h.read::<u32>(r + 0x4c) == 0 {
            h.write(r + 0x4c, cmd);
        }
        h.write(r + 0x50, cmd.wrapping_add(0x48));
        h.write(cmd, opcode);
        if target != 0 {
            for k in 0..4 {
                let w: u32 = h.read(target + 4 * k);
                h.write(data + 4 * k, w);
            }
        }
        h.write(cmd + 8, data);
        Some(cmd)
    }

    /// `PRenderer::setColorTarget(renderer; target, slot)` (`0x005bb130`,
    /// thiscall, `ret 8`): 0x16 in a scene, 3 for a slot above 3. A
    /// target (16 bytes; its texture first) is checked, the viewport set
    /// from its size (or from the back buffer when it has no texture),
    /// copied into the slot and marked in the mask; no target clears the
    /// slot's bit, and with no colour or depth target left the viewport
    /// is the back buffer's. Then the command 0xb `{payload, slot}` (the
    /// frame submitted first when the rings are full). 0, a callee's
    /// error, or 0xd.
    pub fn set_color_target(h: &mut impl Host, r: u32, target: u32, slot: u32) -> u32 {
        let state: u32 = h.read(r + 4);
        if state & 1 != 0 {
            return 0x16;
        }
        if slot >= 4 {
            return 3;
        }
        let bit = 1u8 << (slot & 7);
        let mask_at = r + renderer::COLOR_MASK;
        let from_cache = if target == 0 {
            let mask = h.read::<u8>(mask_at) & !bit;
            h.write(mask_at, mask);
            mask == 0 && h.read::<u8>(r + renderer::HAS_DEPTH) == 0
        } else {
            let tex: u32 = h.read(target);
            if tex != 0 {
                let e = h.call_sym(VALIDATE, Conv::Thiscall(tex), &[]);
                if e != 0 {
                    return e;
                }
            }
            let tex: u32 = h.read(target);
            if tex != 0 {
                let e = h.call_sym(
                    VIEWPORT_SIZE,
                    Conv::Thiscall(tex),
                    &[r + renderer::VIEWPORT + 8, r + renderer::VIEWPORT + 0xc],
                );
                if e != 0 {
                    return e;
                }
                h.write(r + renderer::VIEWPORT, 0u32);
                h.write(r + renderer::VIEWPORT + 4, 0u32);
            }
            let at = r + renderer::COLOR_TARGETS + 16 * slot;
            for k in 0..4 {
                let w: u32 = h.read(target + 4 * k);
                h.write(at + 4 * k, w);
            }
            let mask = h.read::<u8>(mask_at) | bit;
            h.write(mask_at, mask);
            tex == 0
        };
        if from_cache {
            viewport_from_cache(h, r, true);
        }
        let size = if target == 0 { 0 } else { 0x10 };
        let room = h.call_sym(HAS_ROOM, Conv::Thiscall(r), &[size, 1]);
        if room as u8 == 0 {
            h.call_virtual(r, 2, &[]);
        }
        let Some(cmd) = finish(h, r, target, size, SET_COLOR_OPCODE) else {
            return 0xd;
        };
        h.write(cmd + 0xc, slot);
        0
    }

    /// `PRenderer::setDepthStencilTarget(renderer; target)` (`0x005bb5b0`,
    /// thiscall, `ret 4`): 0x16 in a scene. A target is checked, copied
    /// to `+0x128` and marked bound, and the viewport set from its size
    /// (the back buffer's without a texture); no target unbinds it, and
    /// with no colour target either the viewport is the back buffer's.
    /// Then the command 0xc `{payload}`, the rings checked inline (their
    /// wrapped cursors in `frame`, the original's `ebp - 0x1c`). 0, a
    /// callee's error, or 0xd.
    pub fn set_depth_stencil_target(h: &mut impl Host, r: u32, frame: u32, target: u32) -> u32 {
        let state: u32 = h.read(r + 4);
        if state & 1 != 0 {
            return 0x16;
        }
        let from_cache = if target == 0 {
            let mask: u8 = h.read(r + renderer::COLOR_MASK);
            h.write(r + renderer::HAS_DEPTH, 0u8);
            mask == 0
        } else {
            let tex: u32 = h.read(target);
            if tex != 0 {
                let e = h.call_sym(VALIDATE, Conv::Thiscall(tex), &[]);
                if e != 0 {
                    return e;
                }
            }
            for k in 0..4 {
                let w: u32 = h.read(target + 4 * k);
                h.write(r + renderer::DEPTH_TARGET + 4 * k, w);
            }
            h.write(r + renderer::HAS_DEPTH, 1u8);
            let tex: u32 = h.read(target);
            if tex != 0 {
                let e = h.call_sym(
                    VIEWPORT_SIZE,
                    Conv::Thiscall(tex),
                    &[r + renderer::VIEWPORT + 8, r + renderer::VIEWPORT + 0xc],
                );
                if e != 0 {
                    return e;
                }
                h.write(r + renderer::VIEWPORT + 4, 0u32);
                h.write(r + renderer::VIEWPORT, 0u32);
                false
            } else {
                true
            }
        };
        if from_cache {
            viewport_from_cache(h, r, false);
        }
        let size = if target == 0 { 0 } else { 0x10 };
        if full(h, r + 0x1c, size, frame + 4) || full(h, r + 0x30, 0x48, frame) {
            h.call_virtual(r, 2, &[]);
        }
        if finish(h, r, target, size, SET_DEPTH_OPCODE).is_none() {
            return 0xd;
        }
        0
    }

    fn f(bits: u32) -> Reg {
        Reg::load(f32::from_bits(bits))
    }

    fn bits(r: Reg) -> u32 {
        r.store().to_bits()
    }

    /// The original's `f64` constant at `sym`.
    fn constant(h: &impl Host, sym: u32) -> Reg {
        Reg::constant(h.read::<f64>(h.rebase(sym)))
    }

    /// `0x005b5ef0(&a, &b, &c, &d, light)` (cdecl): a light's attenuation
    /// factors. With `n = +0x28`, `f = +0x2c` (the distances) and
    /// `r = 1 / (f² - n²)`: `a = -r`, `b = n² r + 1`; with the cone's cosines
    /// `ci = cos(+0x1c - 1e-6)`, `co = cos(+0x20 + 1e-6)`: `c = -1 / (co -
    /// ci)`, `d = co / (co - ci)`. A point light keeps `c = 0`, `d = 1`;
    /// any other type than spot or point gets `0, 1, 0, 1`. Both cosines
    /// are taken always.
    ///
    /// Returns `eax` (`&d`) and the light argument slot's last value (the
    /// original keeps its temporaries there).
    pub fn light_factors(h: &mut impl Host, out: [u32; 4], light: u32) -> (u32, u32) {
        let far = f(h.read(light + 0x2c));
        let near = f(h.read(light + 0x28));
        let near2 = near * near;
        let span = bits(far * far - near2);
        let r = bits(Reg::ONE / f(span));
        let minus_r = bits(-f(r));
        let n2 = bits(near2);
        let b = bits(f(r) * f(n2) + Reg::ONE);
        let eps = constant(h, ANGLE_EPSILON);
        let cos = h.rebase(CI_COS);
        let inner = bits(f(h.read(light + 0x1c)) - eps);
        let ci = h.call_intrinsic(cos, f(inner).value()).to_bits();
        let outer = bits(f(h.read(light + 0x20)) + eps);
        let co = h.call_intrinsic(cos, f(outer).value()).to_bits();
        let diff = f(co) - f(ci);
        let c = bits(constant(h, MINUS_ONE) / diff);
        let d = bits(f(co) / diff);
        let ty: u32 = h.read(light + 0x18);
        let (mut a_out, mut b_out, mut c_out, mut d_out) = (minus_r, b, c, d);
        if ty != h.rebase(SPOT_TYPE) {
            if ty != h.rebase(POINT_TYPE) {
                a_out = 0;
                b_out = 1f32.to_bits();
            }
            c_out = 0;
            d_out = 1f32.to_bits();
        }
        for (p, v) in out.into_iter().zip([a_out, b_out, c_out, d_out]) {
            h.write(p, v);
        }
        (out[3], a_out)
    }

    /// `PLegacyGlowGPUBase::initialize(this; cluster, w, h)` (`0x00a17bd0`,
    /// thiscall, `ret 0xc`): three levels from a quarter of the size,
    /// halved each level, each two targets (`+0x78 + 4k` named "glowX",
    /// `+0x84 + 4k` "glowY"; the name stored even without a target, as
    /// the original does). 0.
    pub fn glow_gpu_initialize(h: &mut impl Host, this: u32, cluster: u32, w: u32, ht: u32) -> u32 {
        let (mut w, mut ht) = (w >> 2, ht >> 2);
        for k in 0..3 {
            for (field, name) in [(0x78, GLOW_X_NAME), (0x84, GLOW_Y_NAME)] {
                let rt = half_target(h, cluster, w, ht);
                h.write(this + field + 4 * k, rt);
                let name = h.rebase(name);
                h.write(rt.wrapping_add(0xc), name);
            }
            w >>= 1;
            ht >>= 1;
        }
        0
    }

    /// `PLegacyGlowBase::initializeShaders(this; cluster, effect)`
    /// (`0x00a1ab00`, thiscall, `ret 8`): the glow GPU effect (`+0x48`;
    /// 0x15 without), its material (`+0x4c`), the six parameters
    /// (`+0x60..+0x74`; 0x13 when one of the first five is missing), a
    /// one-word state array holding the material, `SetValue(uv scale bias,
    /// {1, 1, 0, 0})` through the frame (`ebp - 0x1c`), then the two
    /// textures' registration (`this`, `this + 0x24`). 0 or the first
    /// error.
    pub fn glow_initialize_shaders(
        h: &mut impl Host,
        this: u32,
        frame: u32,
        cluster: u32,
        effect: u32,
    ) -> u32 {
        let fx = h.call_sym(GLOW_GPU_EFFECT, Conv::Cdecl, &[cluster]);
        h.write(this + 0x48, fx);
        if fx == 0 {
            return 0x15;
        }
        let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(effect), &[cluster, 0, 0]);
        h.write(this + 0x4c, material);
        let mut params = [0u32; 6];
        for (k, &name) in GLOW_PARAMETERS.iter().enumerate() {
            let at = h.rebase(name);
            params[k] = h.call_sym(FIND_PARAMETER, Conv::Thiscall(effect), &[at]);
            h.write(this + 0x60 + 4 * k as u32, params[k]);
        }
        if params[..5].contains(&0) {
            return 0x13;
        }
        let ty = h.rebase(STATE_ARRAY_TYPE);
        let array = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, ty, 0]);
        let count: u32 = h.read(array);
        if count != 1 {
            let cap = count & 0x7fff_ffff;
            let data = match cap {
                0 => 0,
                1 => array + 4,
                _ => h.read(array + 4),
            };
            h.write(array + 4, 0u32);
            let count: u32 = h.read(array);
            if data != array + 4 && count >> 31 == 0 {
                h.call_sym(FREE, Conv::Cdecl, &[data]);
            }
            h.write(array, 1u32);
        }
        let material: u32 = h.read(this + 0x4c);
        h.call_sym(SET_WORD, Conv::Thiscall(array), &[0, material]);
        let fx: u32 = h.read(this + 0x48);
        h.write(fx + 0x10, array);
        let uv: u32 = h.read(this + 0x74);
        if uv != 0 {
            let material: u32 = h.read(this + 0x4c);
            let one = 1f32.to_bits();
            for (k, v) in [one, one, 0, 0].into_iter().enumerate() {
                h.write(frame + 4 * k as u32, v);
            }
            let buffer: u32 = h.read(material + 4);
            let e = h.call_sym(SET_VALUE, Conv::Thiscall(buffer), &[uv, frame]);
            if e != 0 {
                return e;
            }
        }
        let e = h.call_sym(REGISTER, Conv::Thiscall(this), &[]);
        if e != 0 {
            return e;
        }
        h.call_sym(REGISTER, Conv::Thiscall(this + 0x24), &[])
    }

    /// `0x005c6c30(network; out, ctx)` (thiscall, `ret 8`): a dynamic
    /// geometry packet's layout. The header: the network's `+0x28` and its
    /// first `u16` (`+0x10`, `+0x1c`), the context's segment `+0x10` and
    /// the network's `+0x2c` (`+0x18`, `+0x20`; 0 without a segment),
    /// `+0x30 = out + 4`. Then each input (`+4` count, 20-byte records at
    /// `+8`, each pointing at its type) gets its offset in the `u16`
    /// table at `+0x34` and its block from the type's callbacks: `+0x1c`
    /// `(dst + 0x10, ctx, dst)` behind a 16-byte header, else `+0x20`
    /// `(dst, ctx)` (cdecl; sizes rounded up to 16); the context's `+0xc`
    /// is the input's index. The table is padded with `0xffff` to 16
    /// bytes, the end offset stored at `+0x22` and the network's `+0x18`
    /// words zeroed after it. 0.
    pub fn packet_layout(h: &mut impl Host, net: u32, out: u32, ctx: u32) -> u32 {
        let shared: u32 = h.read(net + 0x28);
        h.write(out + 0x10, shared);
        let first: u16 = if shared == 0 { 0 } else { h.read(shared) };
        h.write(out + 0x1c, first);
        h.write(out + 0x14, 0u32);
        h.write(out + 0x1e, 0u16);
        let segment: u32 = h.read(ctx + 8);
        if segment == 0 {
            h.write(out + 0x18, 0u32);
            h.write(out + 0x20, 0u16);
        } else {
            let v: u32 = h.read(segment + 0x10);
            h.write(out + 0x18, v);
            let v: u16 = h.read(net + 0x2c);
            h.write(out + 0x20, v);
        }
        h.write(out + 0x30, out + 4);
        let count = h.read::<u32>(net + 4) & 0x7fff_ffff;
        let words = h.read::<u32>(net + 0x18) & 0x7fff_ffff;
        let table = out + 0x34;
        let mut offset = count.wrapping_mul(2).wrapping_add(0x33) & !0xf;
        for i in 0..count {
            h.write(table + 2 * i, offset as u16);
            h.write(ctx + 0xc, i);
            let records: u32 = h.read(net + 8);
            let ty: u32 = h.read(records + 0x14 * i);
            let dst = out.wrapping_add(0x10).wrapping_add(offset);
            let with_header: u32 = h.read(ty + 0x1c);
            let size = if with_header != 0 {
                let n = h.call(with_header, Conv::Cdecl, &[dst + 0x10, ctx, dst]);
                (n.wrapping_add(0xf) & !0xf).wrapping_add(0x10)
            } else {
                let plain: u32 = h.read(ty + 0x20);
                let n = if plain == 0 {
                    0
                } else {
                    h.call(plain, Conv::Cdecl, &[dst, ctx])
                };
                n.wrapping_add(0xf) & !0xf
            };
            offset = offset.wrapping_add(size);
        }
        let mut pad = table.wrapping_add(count.wrapping_mul(2));
        while pad & 0xf != 0 {
            h.write(pad, 0xffffu16);
            pad = pad.wrapping_add(2);
        }
        h.write(out + 0x22, offset as u16);
        let tail = out.wrapping_add(0x10).wrapping_add(offset);
        for k in 0..words {
            h.write(tail.wrapping_add(4 * k), 0u32);
        }
        0
    }
}
