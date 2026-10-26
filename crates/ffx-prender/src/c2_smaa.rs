//! `PostProcessManager_renderSMAA` (M4 coverage 2, list `c2-new`): the
//! three SMAA passes (edges, blend weights, neighbourhood blending) that
//! `PhyreScene_renderPostAA` runs for `PostAA` `PAA_SMAA1`-`4`; only the
//! `settings-smaa` replay reaches it.
//!
//! The original sets up a C++ exception frame (`fs:[0]`, its state at
//! `ebp - 4`) and a security cookie; nothing it calls throws, so the port
//! leaves both out, as [`crate::glue10`]'s loaders do. Its locals are
//! laid out as the original's: the parameter records and render surfaces
//! its callees get by address.

use ffx_bridge::x87;

/// `PostProcessManager_isEffectAllowed(map)` (stdcall, `al`) and
/// `g_CurrentMapNo`.
pub const IS_EFFECT_ALLOWED: u32 = 0x0069_8730;
pub const CURRENT_MAP: u32 = crate::c2_post::CURRENT_MAP;
/// `PArray_Data(this)`: the material's data (`[[r]]` the parameter list,
/// `[[r] + 4]` the parameter buffer).
pub const ARRAY_DATA: u32 = 0x004d_45a0;
/// `ShaderParams_FindByName(this; name)`, the buffer's setters
/// `setValueDesc(this; param, value)` (samplers), `0x0066e4f0(this; param,
/// record)` (textures), `ShaderParamBuffer_SetValue` / `SetValue2(this;
/// param, src)` (a float4, a float) and `0x00698900(this; param, src)`
/// (an int).
pub const FIND_BY_NAME: u32 = 0x0056_cbf0;
pub const SET_VALUE_DESC: u32 = 0x0056_e600;
pub const SET_TEXTURE: u32 = 0x0066_e4f0;
pub const SET_VALUE: u32 = 0x0063_8780;
pub const SET_VALUE2: u32 = 0x0063_86c0;
pub const SET_INT: u32 = 0x0069_8900;
/// The parameter names (`.rdata`).
pub const LINEAR_SAMPLER: u32 = 0x00b4_7950;
pub const POINT_SAMPLER: u32 = 0x00b4_7960;
pub const AREA_TEX: u32 = 0x00b4_7970;
pub const SEARCH_TEX: u32 = 0x00b4_7978;
pub const RT_METRICS: u32 = 0x00b4_7984;
pub const THRESHOLD: u32 = 0x00b4_7994;
pub const MAX_SEARCH_STEPS: u32 = 0x00b4_79a4;
pub const MAX_SEARCH_STEPS_DIAG: u32 = 0x00b4_79bc;
pub const CORNER_ROUNDING: u32 = 0x00b4_79d8;
pub const COLOR_TEX: u32 = 0x00b4_79f0;
pub const DEPTH_TEX: u32 = 0x00b4_79fc;
pub const EDGES_TEX: u32 = 0x00b4_7a08;
pub const BLEND_TEX: u32 = 0x00b4_7a14;
/// The thresholds of qualities 1 (0.15), 2-3 (0.1) and 4 (0.05), `f32`.
pub const THRESHOLD_LOW: u32 = 0x00b4_7c60;
pub const THRESHOLD_MID: u32 = 0x00b1_0f38;
pub const THRESHOLD_ULTRA: u32 = 0x00b4_7c5c;
/// `PRenderer` methods: `setViewport(x, y, w, h)`, `setClearColor(r, g,
/// b, a)`, `setClearStencil(v)`, `renderMeshInstanceForPass(mesh)`.
pub const SET_VIEWPORT: u32 = 0x005b_c060;
pub const SET_CLEAR_COLOR: u32 = 0x005b_ada0;
pub const SET_CLEAR_STENCIL: u32 = 0x005b_afc0;
pub const RENDER_FOR_PASS: u32 = 0x005b_9a70;
/// The three passes' contexts (the renderer's `+0x48`) and the depth
/// copy's.
pub const CONTEXT_EDGES: u32 = 0x00cc_c9fc;
pub const CONTEXT_BLEND: u32 = 0x00cc_ca1c;
pub const CONTEXT_RESOLVE: u32 = 0x00cc_ca2c;
pub const CONTEXT_DEPTH_COPY: u32 = 0x00cc_c8ec;
/// `PostProcessManager_copyBuffer(this; renderer, texture, target, 1, 0,
/// 0)` and `copyDepthBuffer(this; renderer, texture, target, context,
/// &colour, 1, 0, 0, 0)`.
pub const COPY_BUFFER: u32 = 0x0069_2c40;
pub const COPY_DEPTH_BUFFER: u32 = 0x0069_2e90;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::c2_post::{
        BEGIN_SCENE, END_SCENE, SET_COLOR_TARGET, SET_DEPTH_STENCIL, SURFACE_CTOR, SURFACE_DTOR,
        TWO_32,
    };
    use ffx_graphic::host::{Conv, Host};

    /// A quality's integer parameter.
    type Value = fn(u32) -> u32;

    fn r32(h: &impl Host, at: u32) -> u32 {
        h.read(at)
    }

    /// A surface handle's texture: its first word unless its byte `+4` is
    /// set (then 0).
    fn texture(h: &impl Host, s: u32) -> u32 {
        if h.read::<u8>(s + 4) == 0 {
            r32(h, s)
        } else {
            0
        }
    }

    /// An unsigned word as the x87 makes it a float: `fild`, `2^32` added
    /// when negative, rounded to `f32`.
    fn unsigned_f32(h: &impl Host, v: u32) -> u32 {
        let mut x = f64::from(v.cast_signed());
        if v.cast_signed() < 0 {
            x = x87::add(x, x87::load(r32(h, h.rebase(TWO_32))));
        }
        x87::store(x)
    }

    /// The parameter `name` of the list `params`, 0 when absent.
    fn find(h: &mut impl Host, params: u32, name: u32) -> u32 {
        let name = h.rebase(name);
        h.call_sym(FIND_BY_NAME, Conv::Thiscall(params), &[name])
    }

    /// A texture record `{0xff, 0, 0, texture}` at `rec` (written in the
    /// original's order) through `0x0066e4f0`.
    fn set_texture(h: &mut impl Host, buffer: u32, param: u32, rec: u32, tex: u32) -> u32 {
        h.write(rec, 0xffu32);
        h.write(rec + 0xc, tex);
        h.write(rec + 8, 0u32);
        h.write(rec + 4, 0u32);
        h.call_sym(SET_TEXTURE, Conv::Thiscall(buffer), &[param, rec])
    }

    /// One pass's render surfaces: the colour target (`this` field
    /// `colour`, constructed at `colour_at`) and the depth target (field
    /// `+0x122c0` at `depth_at`; none: `setDepthStencilTarget(0)`).
    struct Targets {
        colour: u32,
        colour_at: u32,
        depth_at: Option<u32>,
    }

    /// The steps of one pass after its surfaces: the targets set, the
    /// viewport of the colour target's texture (re-read from `this`), a
    /// black clear (and stencil 0 with `stencil`), `beginScene(flags)`,
    /// the context, the pass mesh (`+0x122b8`) and `endScene(1)`. The
    /// first non-zero result.
    fn pass_steps(
        h: &mut impl Host,
        this: u32,
        renderer: u32,
        t: &Targets,
        stencil: bool,
        flags: u32,
        context: u32,
    ) -> u32 {
        let r = Conv::Thiscall(renderer);
        let e = h.call_sym(SET_COLOR_TARGET, r, &[t.colour_at, 0]);
        if e != 0 {
            return e;
        }
        let e = h.call_sym(SET_DEPTH_STENCIL, r, &[t.depth_at.unwrap_or(0)]);
        if e != 0 {
            return e;
        }
        let handle = r32(h, this + t.colour);
        let tex = texture(h, handle);
        let ht = r32(h, tex.wrapping_add(0x20));
        let w = r32(h, tex.wrapping_add(0x1c));
        h.call_sym(SET_VIEWPORT, r, &[0, 0, w, ht]);
        let e = h.call_sym(SET_CLEAR_COLOR, r, &[0, 0, 0, 0]);
        if e != 0 {
            return e;
        }
        if stencil {
            let e = h.call_sym(SET_CLEAR_STENCIL, r, &[0]);
            if e != 0 {
                return e;
            }
        }
        let e = h.call_sym(BEGIN_SCENE, r, &[flags]);
        if e != 0 {
            return e;
        }
        let context = h.rebase(context);
        h.write(renderer + 0x48, context);
        let mesh = r32(h, this + 0x122b8);
        let e = h.call_sym(RENDER_FOR_PASS, r, &[mesh]);
        if e != 0 {
            return e;
        }
        h.call_sym(END_SCENE, r, &[1])
    }

    /// One pass: its surfaces constructed, [`pass_steps`], the surfaces
    /// destroyed in reverse.
    fn pass(
        h: &mut impl Host,
        this: u32,
        renderer: u32,
        t: &Targets,
        stencil: bool,
        flags: u32,
        context: u32,
    ) -> u32 {
        let colour = r32(h, this + t.colour);
        h.call_sym(SURFACE_CTOR, Conv::Thiscall(t.colour_at), &[colour, 0]);
        if let Some(at) = t.depth_at {
            let depth = r32(h, this + 0x122c0);
            h.call_sym(SURFACE_CTOR, Conv::Thiscall(at), &[depth, 0]);
        }
        let e = pass_steps(h, this, renderer, t, stencil, flags, context);
        if let Some(at) = t.depth_at {
            h.call_sym(SURFACE_DTOR, Conv::Thiscall(at), &[]);
        }
        h.call_sym(SURFACE_DTOR, Conv::Thiscall(t.colour_at), &[]);
        e
    }

    /// `PostProcessManager_renderSMAA(this; renderer)` (`ret 4`, frame at
    /// `ebp`) -> 0 or the first error. Runs with the SMAA shaders loaded
    /// (`+0x123d4`, `+0x123d5`), a quality (`+0x1231c`, 1-4), the area and
    /// search textures (`+0x122c4`, `+0x122c8`) and the effect allowed on
    /// the map. Sets the pass material's parameters (samplers `+0x122cc`
    /// and `+0x122f0`, the two lookup textures, `{1/w, 1/h, w, h}` of the
    /// colour buffer (`+0x8c`), the quality's threshold, search steps,
    /// diagonal steps and corner rounding, the colour and depth textures),
    /// then the passes: edges into `+0xa4` (depth `+0x122c0`, stencil
    /// cleared, scene flags 0xb), blend weights into `+0x9c` (edges as
    /// input), the blend into `+0xa4` (weights as input), and copies the
    /// result to the colour buffer (with depth when `+0x12318`).
    #[allow(clippy::too_many_lines)] // as the original
    pub fn render_smaa(h: &mut impl Host, ebp: u32, this: u32, renderer: u32) -> u32 {
        if h.read::<u8>(this + 0x123d4) == 0
            || h.read::<u8>(this + 0x123d5) == 0
            || r32(h, this + 0x1231c) == 0
            || r32(h, this + 0x122c4) == 0
            || r32(h, this + 0x122c8) == 0
        {
            return 0;
        }
        let map = r32(h, h.rebase(CURRENT_MAP));
        let allowed = h.call_sym(IS_EFFECT_ALLOWED, Conv::Stdcall, &[map]) as u8;
        if allowed == 0 {
            return 0;
        }
        let material = r32(h, r32(h, this + 0x122b8) + 0x10);
        let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(material), &[]);
        let params = r32(h, r32(h, data));
        h.write(ebp - 0x2c, params);
        let data = h.call_sym(ARRAY_DATA, Conv::Thiscall(material), &[]);
        let buffer = r32(h, r32(h, data) + 4);
        h.write(ebp - 0x30, buffer);
        let b = Conv::Thiscall(buffer);

        for (name, value) in [(LINEAR_SAMPLER, 0x122cc), (POINT_SAMPLER, 0x122f0)] {
            let p = find(h, params, name);
            if p != 0 {
                let e = h.call_sym(SET_VALUE_DESC, b, &[p, this + value]);
                if e != 0 {
                    return e;
                }
            }
        }
        for (name, rec, tex) in [
            (AREA_TEX, ebp - 0x94, 0x122c4),
            (SEARCH_TEX, ebp - 0x44, 0x122c8),
        ] {
            let p = find(h, params, name);
            if p != 0 {
                let t = r32(h, this + tex);
                h.write(rec + 0xc, t);
                h.write(rec + 8, this + 0x122f0);
                h.write(rec, 0xffu32);
                h.write(rec + 4, 0u32);
                let e = h.call_sym(SET_TEXTURE, b, &[p, rec]);
                if e != 0 {
                    return e;
                }
            }
        }
        let p = find(h, params, RT_METRICS);
        h.write(ebp - 0x34, p);
        if p != 0 {
            let colour = r32(h, this + 0x8c);
            let tex = texture(h, colour);
            let ht = r32(h, tex.wrapping_add(0x20));
            let w = r32(h, tex.wrapping_add(0x1c));
            let (wf, hf) = (unsigned_f32(h, w), unsigned_f32(h, ht));
            let inv = |v: u32| x87::store(x87::div(1.0, x87::load(v)));
            let metrics = ebp - 0x74;
            for (k, v) in [inv(wf), inv(hf), wf, hf].into_iter().enumerate() {
                h.write(metrics + 4 * k as u32, v);
            }
            // the original's temporaries
            h.write(ebp - 0x10, hf);
            h.write(ebp - 0x24, wf);
            h.write(ebp - 0x20, hf);
            let e = h.call_sym(SET_VALUE, b, &[p, metrics]);
            if e != 0 {
                return e;
            }
        }
        let p = find(h, params, THRESHOLD);
        if p != 0 {
            let at = match r32(h, this + 0x1231c) {
                1 => Some(THRESHOLD_LOW),
                2 | 3 => Some(THRESHOLD_MID),
                4 => Some(THRESHOLD_ULTRA),
                _ => None,
            };
            let v = at.map_or(1f32.to_bits(), |at| {
                x87::store(x87::load(r32(h, h.rebase(at))))
            });
            h.write(ebp - 0x1c, v);
            let e = h.call_sym(SET_VALUE2, b, &[p, ebp - 0x1c]);
            if e != 0 {
                return e;
            }
        }
        let ints: [(u32, u32, Value); 3] = [
            (MAX_SEARCH_STEPS, ebp - 0x14, |q| match q {
                1 => 4,
                2 => 8,
                4 => 0x20,
                _ => 0x10,
            }),
            (MAX_SEARCH_STEPS_DIAG, ebp - 0x18, |q| match q {
                1 | 2 => 0,
                4 => 0x10,
                _ => 8,
            }),
            (CORNER_ROUNDING, ebp - 0x28, |q| match q {
                1 | 2 => 0,
                _ => 0x19,
            }),
        ];
        for (name, at, value) in ints {
            let p = find(h, params, name);
            if p != 0 {
                let v = value(r32(h, this + 0x1231c));
                h.write(at, v);
                let e = h.call_sym(SET_INT, b, &[p, at]);
                if e != 0 {
                    return e;
                }
            }
        }
        for (name, rec, field) in [(COLOR_TEX, ebp - 0xb4, 0x8c), (DEPTH_TEX, ebp - 0xa4, 0x90)] {
            let p = find(h, params, name);
            if p != 0 {
                let tex = texture(h, r32(h, this + field));
                let e = set_texture(h, buffer, p, rec, tex);
                if e != 0 {
                    return e;
                }
            }
        }
        let edges = Targets {
            colour: 0xa4,
            colour_at: ebp - 0xf4,
            depth_at: Some(ebp - 0xc4),
        };
        let e = pass(h, this, renderer, &edges, true, 0xb, CONTEXT_EDGES);
        if e != 0 {
            return e;
        }
        let params = r32(h, ebp - 0x2c);
        let buffer = r32(h, ebp - 0x30);
        let p = find(h, params, EDGES_TEX);
        if p != 0 {
            let tex = texture(h, r32(h, this + 0xa4));
            let e = set_texture(h, buffer, p, ebp - 0x84, tex);
            if e != 0 {
                return e;
            }
        }
        let weights = Targets {
            colour: 0x9c,
            colour_at: ebp - 0xe4,
            depth_at: Some(ebp - 0xd4),
        };
        let e = pass(h, this, renderer, &weights, false, 1, CONTEXT_BLEND);
        if e != 0 {
            return e;
        }
        let params = r32(h, ebp - 0x2c);
        let buffer = r32(h, ebp - 0x30);
        let p = find(h, params, BLEND_TEX);
        if p != 0 {
            let tex = texture(h, r32(h, this + 0x9c));
            let e = set_texture(h, buffer, p, ebp - 0x54, tex);
            if e != 0 {
                return e;
            }
        }
        let resolve = Targets {
            colour: 0xa4,
            colour_at: ebp - 0x104,
            depth_at: None,
        };
        let e = pass(h, this, renderer, &resolve, false, 0, CONTEXT_RESOLVE);
        if e != 0 {
            return e;
        }
        let target = r32(h, this + 0x8c);
        if h.read::<u8>(this + 0x12318) == 0 {
            let tex = texture(h, r32(h, this + 0xa4));
            h.call_sym(
                COPY_BUFFER,
                Conv::Thiscall(this),
                &[renderer, tex, target, 1, 0, 0],
            );
        } else {
            let colour = ebp - 0x64;
            for k in 0..4 {
                h.write(colour + 4 * k, 1f32.to_bits());
            }
            let tex = texture(h, r32(h, this + 0x9c));
            let context = h.rebase(CONTEXT_DEPTH_COPY);
            h.call_sym(
                COPY_DEPTH_BUFFER,
                Conv::Thiscall(this),
                &[renderer, tex, target, context, colour, 1, 0, 0, 0],
            );
        }
        0
    }
}
