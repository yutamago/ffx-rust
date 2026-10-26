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

/// A post effect's `initializeCommonShaders(this; cluster, effect)`
/// (thiscall, `ret 8`): the fields and names it uses.
#[derive(Debug, Clone, Copy)]
pub struct CommonShaders {
    /// The effect object (0x15 without it) and the material's field.
    pub effect: u32,
    pub material: u32,
    /// The parameters looked up, in order: (name, field).
    pub params: &'static [(u32, u32)],
    /// The two textures registered (`0x0058cf80`), in order.
    pub register: [u32; 2],
    /// The two samplers: (name, sampler field). The first's block is at
    /// the frame's `+0x10` (`ebp - 0x10`), the second's at `+0` (`ebp -
    /// 0x20`).
    pub samplers: [(u32, u32); 2],
    /// Whether both samplers are looked up before either is written
    /// (else each right before its write).
    pub lookups_first: bool,
}

/// `PFXAABase::initializeCommonShaders` (`0x00a18870`).
pub const FXAA_COMMON: CommonShaders = CommonShaders {
    effect: 0x60,
    material: 0x64,
    params: &[(0x00b4_25d8, 0x68), (0x00b7_6b58, 0x6c)],
    register: [0x14, 0x38],
    samplers: [(LINEAR_CLAMP, 0x38), (POINT_CLAMP, 0x14)],
    lookups_first: false,
};

/// `PDepthOfFieldBase::initializeCommonShaders` (`0x00a185a0`).
pub const DOF_COMMON: CommonShaders = CommonShaders {
    effect: 0xf38,
    material: 0xf3c,
    params: &[
        (0x00b4_25d8, 0xf88),
        (0x00b7_6674, 0xf8c),
        (0x00b7_668c, 0xf90),
        (0x00b7_61b8, 0xf94),
        (0x00b7_66a0, 0xf98),
        (0x00b7_66ac, 0xf9c),
        (0x00b7_66c0, 0xfa0),
        (0x00b7_66d4, 0xfa4),
        (0x00b7_66e4, 0xfa8),
        (0x00b7_66fc, 0xfac),
    ],
    register: [0xf40, 0xf64],
    samplers: [(POINT_CLAMP, 0xf40), (LINEAR_CLAMP, 0xf64)],
    lookups_first: true,
};

/// The sampler parameters' names, `LinearClampSampler` and
/// `PointClampSampler`; the CRT's `memcpy` (cdecl).
pub const LINEAR_CLAMP: u32 = 0x00b3_018c;
pub const POINT_CLAMP: u32 = 0x00b7_62d0;
pub const MEMCPY: u32 = 0x0094_9256;
/// `initializeCommonShaders`' frame: the two sampler blocks
/// (`ebp - 0x20`).
pub const COMMON_FRAME: u32 = 0x20;

/// `0x005ba380`'s callees: the records gatherer `0x005b5370(base, list,
/// out)` and the texture pointers `0x005b53f0(base, params, out)` (cdecl,
/// each returning the cursor past what it wrote), the material's state
/// element `0x004348c0(array; i)` (thiscall), `InterlockedExchangeAdd`'s
/// import slot (stdcall) and the command ring's `allocate`.
pub const GATHER_RECORDS: u32 = 0x005b_5370;
pub const TEXTURE_POINTERS: u32 = 0x005b_53f0;
pub const STATE_ELEMENT: u32 = 0x0043_48c0;
pub const INTERLOCKED_ADD: u32 = 0x00b0_c128;
/// The stride `0x005ba380` adds to its instance pointer per command.
pub const INSTANCE_STRIDE: u32 = 0x254;

/// `0x005ba380`'s arguments after `this` (the renderer).
#[derive(Debug, Clone, Copy)]
pub struct MeshCommands {
    pub context: u32,
    pub instances: u32,
    pub extra: u32,
    pub material: u32,
    pub tag: u32,
    pub refs: u32,
    pub out: u32,
    pub callback: u32,
}

/// `populateContextParameterCaptureBufferWithStreams`' warning format
/// "Failed to find data block/vertex stream to satisfy RDT %s for stream
/// %d\n" and `PhyrePrintf` (cdecl).
pub const STREAM_WARNING: u32 = 0x00b3_6800;
pub const PRINTF: u32 = 0x0043_52f0;

/// `populateContextParameterCaptureBufferWithDynamicStreams`' callees:
/// `PDynamicMesh::getDataBlockCount(mesh; segment)`,
/// `PDynamicMesh::groupSegment(mesh; table, segment, base, blocks)` (which
/// fills the block pointers) and the block record's constructor
/// `0x0055dd30(this)` (thiscall).
pub const DATA_BLOCK_COUNT: u32 = 0x0049_8770;
pub const GROUP_SEGMENT: u32 = 0x0049_8a30;
pub const BLOCK_RECORD_CTOR: u32 = 0x0055_dd30;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ANGLE_EPSILON, CI_COS, CLUSTER_CREATE, CREATE_MATERIAL, Conv, FIND_PARAMETER, FREE,
        GLOW_GPU_EFFECT, GLOW_PARAMETERS, GLOW_X_NAME, GLOW_Y_NAME, HAS_ROOM, Host, MINUS_ONE,
        POINT_TYPE, REGISTER, RENDER_STATE_CACHE, RING_ALLOCATE, Reg, SET_COLOR_OPCODE,
        SET_DEPTH_OPCODE, SET_VALUE, SET_WORD, SPOT_TYPE, STATE_ARRAY_TYPE, VALIDATE,
        VIEWPORT_SIZE, renderer,
    };
    use super::{BLOCK_RECORD_CTOR, DATA_BLOCK_COUNT, GROUP_SEGMENT, PRINTF, STREAM_WARNING};
    use super::{
        CommonShaders, GATHER_RECORDS, INSTANCE_STRIDE, INTERLOCKED_ADD, MEMCPY, MeshCommands,
        STATE_ELEMENT, TEXTURE_POINTERS,
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

    /// A state array (`PCluster::create(cluster; 1, 0x00c998e8, 0)`)
    /// reset to one word (an owned block freed), as the shader set-ups
    /// do. Returns it.
    fn one_word_array(h: &mut impl Host, cluster: u32) -> u32 {
        let ty = h.rebase(STATE_ARRAY_TYPE);
        let array = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, ty, 0]);
        let count: u32 = h.read(array);
        if count != 1 {
            let data = array_data(h, array, count);
            h.write(array + 4, 0u32);
            let count: u32 = h.read(array);
            if data != array + 4 && count >> 31 == 0 {
                h.call_sym(FREE, Conv::Cdecl, &[data]);
            }
            h.write(array, 1u32);
        }
        array
    }

    /// A `PSharray`'s data: inline for one element, 0 for none.
    fn array_data(h: &impl Host, array: u32, count: u32) -> u32 {
        match count & 0x7fff_ffff {
            0 => 0,
            1 => array + 4,
            _ => h.read(array + 4),
        }
    }

    /// `PFXAABase`/`PDepthOfFieldBase::initializeCommonShaders(this;
    /// cluster, effect)` ([`CommonShaders`]): 0x15 without the effect
    /// object; the material (its parameter buffer copied from the parent's
    /// `[this + 8] + 4` when their layouts match), the parameters, a
    /// one-word state array holding the material, the two textures'
    /// registration (its error returned), then each sampler's block
    /// `{0x47, 0, sampler, 0}` copied into the material's buffer at the
    /// parameter's offset (3 when it does not fit or lies in the header).
    /// 0.
    ///
    /// `frame`: the original's `ebp - 0x20`. The second result: the
    /// effect argument slot's last value when the original reuses it.
    pub fn common_shaders(
        h: &mut impl Host,
        this: u32,
        frame: u32,
        cluster: u32,
        effect: u32,
        c: &CommonShaders,
    ) -> (u32, Option<u32>) {
        if h.read::<u32>(this + c.effect) == 0 {
            return (0x15, None);
        }
        let material = h.call_sym(CREATE_MATERIAL, Conv::Thiscall(effect), &[cluster, 0, 0]);
        h.write(this + c.material, material);
        let parent: u32 = h.read(this + 8);
        if parent != 0 {
            let dst: u32 = h.read(material + 4);
            let src: u32 = h.read(parent + 4);
            let a: u32 = h.read(dst + 8);
            let b: u32 = h.read(src + 8);
            if (a ^ b).trailing_zeros() >= 31 {
                let n: u32 = h.read(src);
                h.call_sym(MEMCPY, Conv::Cdecl, &[dst, src, n]);
            }
        }
        for &(name, field) in c.params {
            let at = h.rebase(name);
            let p = h.call_sym(FIND_PARAMETER, Conv::Thiscall(effect), &[at]);
            h.write(this + field, p);
        }
        let array = one_word_array(h, cluster);
        let count: u32 = h.read(array);
        if count & 0x7fff_ffff != 0 {
            let data = array_data(h, array, count);
            let material: u32 = h.read(this + c.material);
            h.write(data, material);
        }
        let fx: u32 = h.read(this + c.effect);
        h.write(fx + 0x10, array);
        for off in c.register {
            let e = h.call_sym(REGISTER, Conv::Thiscall(this + off), &[]);
            if e != 0 {
                return (e, None);
            }
        }
        let mut slot = None;
        let mut found = [0u32; 2];
        if c.lookups_first {
            for (k, &(name, _)) in c.samplers.iter().enumerate() {
                let at = h.rebase(name);
                found[k] = h.call_sym(FIND_PARAMETER, Conv::Thiscall(effect), &[at]);
            }
            slot = Some(found[0]);
        }
        for (k, &(name, sampler)) in c.samplers.iter().enumerate() {
            if !c.lookups_first {
                let at = h.rebase(name);
                found[k] = h.call_sym(FIND_PARAMETER, Conv::Thiscall(effect), &[at]);
            }
            let p = found[k];
            if p == 0 {
                continue;
            }
            let material: u32 = h.read(this + c.material);
            let buffer: u32 = h.read(material + 4);
            let offset = u32::from(h.read::<u16>(p + 8));
            let size = u32::from(h.read::<u16>(p + 0xa) & 0x1fff);
            let block = if k == 0 { frame + 0x10 } else { frame };
            for (i, v) in [0x47, 0, this + sampler, 0].into_iter().enumerate() {
                h.write(block + 4 * i as u32, v);
            }
            if k == 0 {
                slot = Some(offset);
            }
            let room: u32 = h.read(buffer);
            if offset + size > room || offset < 4 {
                return (3, slot);
            }
            h.call_sym(MEMCPY, Conv::Cdecl, &[buffer + offset, block, size]);
        }
        (0, slot)
    }

    /// `0x005ba380(renderer; context, instances, extra, material, tag,
    /// refs, out, callback)` (thiscall, `ret 0x20`): the resources the
    /// commands use, gathered at `out + 4` (`refs`' first block, its
    /// records and textures, the material's state element unless `refs`
    /// has one, `extra`'s two blocks) with their count at `out`, each
    /// reference count raised by the instance count (`InterlockedExchangeAdd`);
    /// then one 0x48-byte command per instance (opcode 1, or 2 with the
    /// callback at `+0x44`), the instance pointer stepping by 0x254. 0.
    ///
    /// The second result: the `instances` argument slot's last value (the
    /// original keeps its instance pointer there).
    #[allow(clippy::too_many_lines)]
    pub fn mesh_commands(h: &mut impl Host, r: u32, a: &MeshCommands) -> (u32, u32) {
        let extra_data: u32 = if a.extra == 0 { 0 } else { h.read(a.extra + 4) };
        let mut instance: u32 = h.read(a.instances + 8);
        let n = h.read::<u32>(a.instances + 4) & 0x7fff_ffff;
        let list = a.out + 4;
        let mut cur = list;
        let first: u32 = h.read(a.refs);
        if first != 0 {
            h.write(list, first + 4);
            cur = list + 4;
        }
        let records: u32 = h.read(a.refs + 0xc);
        if records != 0 {
            cur = h.call_sym(
                GATHER_RECORDS,
                Conv::Cdecl,
                &[records, a.instances + 0x14, cur],
            );
        }
        let textures: u32 = h.read(a.refs + 4);
        if textures != 0 {
            cur = h.call_sym(
                TEXTURE_POINTERS,
                Conv::Cdecl,
                &[textures, textures + 8, cur],
            );
        }
        let records: u32 = h.read(a.refs + 0xc);
        if records != 0 {
            cur = h.call_sym(
                TEXTURE_POINTERS,
                Conv::Cdecl,
                &[records, a.instances + 0xc, cur],
            );
        }
        let mut state: u32 = h.read(a.refs + 0x18);
        let mut key = 0u32;
        if state == 0 {
            let states = a.material + 0x30;
            if h.read::<u32>(states + 8) != 0 {
                key = h.read(states + 0x10);
                state = states;
                let e = h.call_sym(STATE_ELEMENT, Conv::Thiscall(states), &[key]);
                h.write(cur, e);
                cur += 4;
            }
        }
        if extra_data != 0 {
            h.write(cur, a.extra + 8);
            h.write(cur + 4, extra_data + 0x18);
            cur += 8;
        }
        let count = (cur.wrapping_sub(list) as i32 >> 2) as u32;
        h.write(a.out, count);
        let add: u32 = h.read(h.rebase(INTERLOCKED_ADD));
        let mut i = 0u32;
        while i < h.read::<u32>(a.out) {
            let target: u32 = h.read(list + 4 * i);
            h.call(add, Conv::Stdcall, &[target, n]);
            i += 1;
        }
        for _ in 0..n {
            let cmd = h.call_sym(RING_ALLOCATE, Conv::Thiscall(r + 0x30), &[0x48]);
            if cmd != 0 {
                if h.read::<u32>(r + 0x4c) == 0 {
                    h.write(r + 0x4c, cmd);
                }
                h.write(r + 0x50, cmd.wrapping_add(0x48));
            }
            if a.callback == 0 {
                h.write(cmd, 1u32);
            } else {
                h.write(cmd, 2u32);
                h.write(cmd.wrapping_add(0x44), a.callback);
            }
            let words = [
                (0x8, a.extra),
                (0xc, extra_data),
                (0x10, h.read(a.refs + 0x14)),
                (0x14, h.read(a.refs + 0x10)),
                (0x18, a.context),
                (0x1c, instance),
            ];
            for (off, v) in words {
                h.write(cmd.wrapping_add(off), v);
            }
            h.write(cmd.wrapping_add(0x24), a.tag as u16);
            h.write(cmd.wrapping_add(0x20), a.material);
            for k in 0..4 {
                let v: u32 = h.read(a.refs + 4 * k);
                h.write(cmd.wrapping_add(0x28 + 4 * k), v);
            }
            h.write(cmd.wrapping_add(0x38), a.out);
            let v = if state != 0 {
                h.read(state + 8)
            } else if h.read::<u32>(a.material + 0x28) & 0x7fff_ffff != 0 {
                let p: u32 = h.read(a.material + 0x2c);
                h.read(p + 4)
            } else {
                0
            };
            instance = instance.wrapping_add(INSTANCE_STRIDE);
            h.write(cmd.wrapping_add(0x3c), state);
            h.write(cmd.wrapping_add(0x26), key as u16);
            h.write(cmd.wrapping_add(0x40), v);
        }
        (0, instance)
    }

    /// A stream's binding found in `src`'s blocks (`+0x28` count, 0x40-byte
    /// blocks at `+0x2c`, each with `+8` count, 12-byte items at `+0xc`):
    /// the first item whose `+4` is `key` (and whose byte `+9` is `sem`,
    /// when given). Returns (block, item).
    fn find_item(h: &impl Host, src: u32, key: u32, sem: Option<u8>) -> Option<(u32, u32)> {
        let n = h.read::<u32>(src + 0x28) & 0x7fff_ffff;
        let mut block: u32 = h.read(src + 0x2c);
        for _ in 0..n {
            let m = h.read::<u32>(block + 8) & 0x7fff_ffff;
            let mut item: u32 = h.read(block + 0xc);
            for _ in 0..m {
                if h.read::<u32>(item + 4) == key && sem.is_none_or(|s| h.read::<u8>(item + 9) == s)
                {
                    return Some((block, item));
                }
                item = item.wrapping_add(0xc);
            }
            block = block.wrapping_add(0x40);
        }
        None
    }

    /// A binding's three words at `buf + u16 [stream + 8]`: the block, the
    /// item and the block's `+0x10`.
    fn bind_stream(h: &mut impl Host, buf: u32, stream: u32, block: u32, item: u32) {
        let dst = buf.wrapping_add(u32::from(h.read::<u16>(stream + 8)));
        h.write(dst.wrapping_add(4), item);
        h.write(dst, block);
        let w: u32 = h.read(block + 0x10);
        h.write(dst.wrapping_add(8), w);
    }

    /// A string pointer with its low tag bit cleared.
    fn untagged(h: &impl Host, at: u32) -> u32 {
        let p: u32 = h.read(at);
        p.wrapping_sub(p & 1)
    }

    /// The CRT's `strcmp` as the original inlines it: whether the strings
    /// are equal.
    fn same_string(h: &impl Host, mut a: u32, mut b: u32) -> bool {
        loop {
            let x: u8 = h.read(a);
            if x != h.read::<u8>(b) {
                return false;
            }
            if x == 0 {
                return true;
            }
            a = a.wrapping_add(1);
            b = b.wrapping_add(1);
        }
    }

    /// `PRendererBase::populateContextParameterCaptureBufferWithStreams(buf,
    /// ctx, &out, mask)` (`0x005b94b0`, stdcall, `ret 0x10`): binds the
    /// context's vertex streams (16-byte records from `[ctx + 0x6c] +
    /// 0x18`, one mask bit each; bits already in `mask` are bound) to data
    /// blocks. First each of the geometry's named inputs (`[ctx + 0x10] +
    /// 0x28`, entry `[ctx + 0x20]`) takes the stream of its key, index and
    /// name (else the first of its key and type byte) and the block item
    /// of its key and semantic byte in `[ctx + 0x1c]`; then every stream
    /// left takes the first item of its key in `[ctx + 0x18]` (when its
    /// key's flag byte is set and there is one) or `[ctx + 0x1c]`, a flagged
    /// one also storing the block's `+4` at `out`. 0, or 0x13 (with a
    /// warning for a named stream) at the first stream with no item.
    ///
    /// `slots`: the caller's argument words, which the original reuses (the
    /// mask so far in the second, the bit and the flag in the fourth).
    pub fn populate_with_streams(
        h: &mut impl Host,
        slots: u32,
        buf: u32,
        ctx: u32,
        out: u32,
        mask: u32,
    ) -> u32 {
        let table: u32 = h.read(ctx + 0x6c);
        let streams: u32 = h.read(table + 0x18);
        let count = h.read::<u32>(table + 0x14) & 0x7fff_ffff;
        let end = streams.wrapping_add(count << 4);
        let primary: u32 = h.read(ctx + 0x1c);
        let secondary: u32 = h.read(ctx + 0x18);
        let index: u32 = h.read(ctx + 0x20);
        let geo: u32 = h.read(h.read::<u32>(ctx + 0x10) + 0x28);
        let at = geo.wrapping_add(index.wrapping_mul(36));
        let inputs = h.read::<u32>(at + 0x1c) & 0x7fff_ffff;
        let mut entry = match inputs {
            0 => 0,
            1 => at + 0x20,
            _ => h.read(at + 0x20),
        };
        let mut bound = mask;
        h.write(slots + 4, bound);
        for _ in 0..inputs {
            let e: u32 = h.read(entry);
            let name = untagged(h, e + 4);
            let key: u32 = h.read(e);
            let kind: u8 = h.read(e + 0xa);
            let (mut fallback, mut fallback_bit) = (0u32, 0u32);
            let mut bit = 1u32;
            h.write(slots + 0xc, bit);
            let mut cur = streams;
            while cur < end {
                if bound & bit == 0 {
                    let same_key = h.read::<u32>(cur) == key;
                    if fallback == 0 && same_key && h.read::<u8>(cur + 0xf) == kind {
                        fallback = cur;
                        fallback_bit = bit;
                    }
                    let e: u32 = h.read(entry);
                    let same_index = h.read::<u16>(cur + 0xc) == h.read::<u16>(e + 8);
                    if same_index && same_key {
                        let s = untagged(h, cur + 4);
                        if s != 0 && same_string(h, s, name) {
                            break;
                        }
                    }
                }
                cur = cur.wrapping_add(0x10);
                bit = bit.wrapping_add(bit);
                h.write(slots + 0xc, bit);
            }
            if cur == end {
                if fallback == 0 {
                    entry += 4;
                    continue;
                }
                cur = fallback;
                bit = fallback_bit;
                h.write(slots + 0xc, bit);
            }
            let e: u32 = h.read(entry);
            let sem: u8 = h.read(e + 0xb);
            if let Some((block, item)) = find_item(h, primary, key, Some(sem)) {
                bind_stream(h, buf, cur, block, item);
                bound |= bit;
                h.write(slots + 4, bound);
            }
            entry += 4;
        }
        let mut bit = 1u32;
        let mut cur = streams;
        while cur < end {
            if bound & bit == 0 {
                let d: u32 = h.read(cur);
                let flag: u8 = if d == 0 { 0 } else { h.read(d + 8) };
                h.write(slots + 0xf, flag);
                let src = if flag & 1 != 0 && secondary != 0 {
                    secondary
                } else {
                    primary
                };
                let Some((block, item)) = find_item(h, src, d, None) else {
                    if d != 0 {
                        let format = h.rebase(STREAM_WARNING);
                        let text: u32 = h.read(d);
                        let n = (cur.wrapping_sub(streams) as i32 >> 4) as u32;
                        h.call_sym(PRINTF, Conv::Cdecl, &[1, format, text, n]);
                    }
                    return 0x13;
                };
                if flag != 0 {
                    let w: u32 = h.read(block + 4);
                    h.write(out, w);
                }
                bind_stream(h, buf, cur, block, item);
            }
            cur = cur.wrapping_add(0x10);
            bit = bit.wrapping_add(bit);
        }
        0
    }

    /// `PRendererBase::populateContextParameterCaptureBufferWithDynamicStreams
    /// (buf, ctx, &flags, mesh, &base, &record)` (`0x005b9290`, stdcall,
    /// `ret 0x18`) -> the mask of the streams bound. With the dynamic
    /// mesh's (`[mesh + 4]`) segment `[ctx + 0x20]` having data blocks and
    /// a `+8` word: `base = buf + [buf]`, the blocks grouped
    /// (`groupSegment`, into `alloc(n)`: the original's `_alloca` of `n`
    /// words), each stream of `[ctx + 0x6c]` bound (block, item, 0) to the
    /// first block item of its key, a flagged key setting `flags` to -1.
    /// Then, when the segment's `+0xc` word `w` is set, a block record
    /// constructed after the blocks (`base + (64 n + 15) & !15`) at `record`:
    /// `+0x34 = (w > 0 ? 4 : 2) |w|`, `+0x38 = 0`, byte `+0xe = 4`,
    /// `+8 = |w|`, byte `+0xc = w > 0 ? 8 : 12`.
    pub fn populate_with_dynamic_streams(
        h: &mut impl Host,
        args: [u32; 6],
        alloc: impl FnOnce(u32) -> u32,
    ) -> u32 {
        let [buf, ctx, flags, mesh, base_out, record_out] = args;
        let segment: u32 = h.read(ctx + 0x20);
        let table: u32 = h.read(ctx + 0x6c);
        let dm: u32 = h.read(mesh + 4);
        let n = h.call_sym(DATA_BLOCK_COUNT, Conv::Thiscall(dm), &[segment]);
        let mut bound = 0u32;
        let segs: u32 = h.read(dm + 0x14);
        let at = segs.wrapping_add(segment << 4);
        if n != 0 && h.read::<u32>(at + 8) != 0 {
            let count = h.read::<u32>(table + 0x14) & 0x7fff_ffff;
            let base = h.read::<u32>(buf).wrapping_add(buf);
            h.write(base_out, base);
            let blocks = alloc(n);
            h.call_sym(
                GROUP_SEGMENT,
                Conv::Thiscall(dm),
                &[table, segment, base, blocks],
            );
            let mut bit = 1u32;
            for k in 0..count {
                let streams: u32 = h.read(table + 0x18);
                let stream = streams.wrapping_add(16 * k);
                let key: u32 = h.read(stream);
                'blocks: for j in 0..n {
                    let block: u32 = h.read(blocks.wrapping_add(4 * j));
                    if block == 0 {
                        continue;
                    }
                    let m = h.read::<u32>(block + 8) & 0x7fff_ffff;
                    let mut item: u32 = h.read(block + 0xc);
                    for _ in 0..m {
                        if h.read::<u32>(item + 4) == key {
                            let dst = buf.wrapping_add(u32::from(h.read::<u16>(stream + 8)));
                            h.write(dst.wrapping_add(4), item);
                            h.write(dst, block);
                            h.write(dst.wrapping_add(8), 0u32);
                            if h.read::<u8>(key + 8) != 0 {
                                h.write(flags, u32::MAX);
                            }
                            bound |= bit;
                            break 'blocks;
                        }
                        item = item.wrapping_add(0xc);
                    }
                }
                bit = bit.wrapping_add(bit);
            }
        }
        let segs: u32 = h.read(dm + 0x14);
        let w: u32 = h.read(segs.wrapping_add(segment << 4) + 0xc);
        if w == 0 {
            return bound;
        }
        let first = h.read::<u32>(buf).wrapping_add(buf);
        let record = ((n << 6).wrapping_add(0xf) & !0xf).wrapping_add(first);
        if record != 0 {
            h.call_sym(BLOCK_RECORD_CTOR, Conv::Thiscall(record), &[]);
        }
        h.write(record_out, record);
        let wi = w.cast_signed();
        let abs = wi.unsigned_abs();
        let unit = if wi > 0 { 4 } else { 2 };
        h.write(record.wrapping_add(0x34), abs.wrapping_mul(unit));
        h.write(record.wrapping_add(0x38), 0u32);
        h.write(record.wrapping_add(0xe), 4u8);
        let rec: u32 = h.read(record_out);
        h.write(rec.wrapping_add(8), abs);
        let rec: u32 = h.read(record_out);
        h.write(rec.wrapping_add(0xc), if wi > 0 { 8u8 } else { 12 });
        bound
    }
}
