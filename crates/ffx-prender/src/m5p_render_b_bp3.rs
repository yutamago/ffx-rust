//! M5 stage 1, list `phyre-render-b`, round big-phyre3
//! (`ffx_prender::m5p_render_b_bp3`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_render_b.rs` (`bp3_*`).
//!
//! Functions that hand callees the addresses of their locals take `ebp`,
//! where the original's frame pointer is, and keep those locals at the
//! original's offsets below it; the shim passes the top of a
//! `StackFrame`, the tests the original's frame address. None of the
//! callees here throws, so the originals' exception frames (which only
//! destroy the render surfaces on unwinding) have no counterpart.

pub use crate::m5p_render_b_bp2::{
    BEGIN_SCENE, END_SCENE, MEMCPY, QUEUE_COMMAND_18, RENDER_FOR_PASS, RENDER_MESH_INSTANCE,
    SET_COLOR_TARGET, SET_DEPTH_TARGET, SET_TEXTURE_CHECKED, SET_VALUE, SURFACE_CTOR, SURFACE_DTOR,
    TROPHY_INSTALL,
};

/// The glow passes' render states (`renderer + 0x48`): the bright pass,
/// the blur pass, the combining pass, the last combining pass with the
/// flag set.
pub const GLOW_STATES: [u32; 4] = [0x0194_4e4c, 0x0194_4e5c, 0x0194_4e6c, 0x0194_4e8c];
/// The combining scale of levels below 3 (`float` 1.4).
pub const GLOW_SCALE_LOW: u32 = 0x00b5_f188;
/// The combining scale of levels 3 and up (`float` 3.0; never used, the
/// level count is at most 3).
pub const GLOW_SCALE_HIGH: u32 = 0x00b3_eb94;
/// `0x00a1d2b0`'s frame below `ebp` (down to the surface at
/// `ebp - 0xf0`), in words.
pub const GLOW_PASSES_FRAME_WORDS: u32 = 0xf0 / 4;

pub mod glow {
    //! `0x00a1d2b0` over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// The parameter buffer (`[[fx + 0x60] + 4]`).
    fn buffer(h: &impl Host, fx: u32) -> u32 {
        let params: u32 = h.read(fx + 0x60);
        h.read(params + 4)
    }

    /// Where a parameter's bytes go in `buffer`: `None` (3) when the
    /// offset is below 4 or they don't fit the buffer's size word.
    fn place(h: &impl Host, param: u32, buffer: u32) -> Option<(u32, u32)> {
        let offset = u32::from(h.read::<u16>(param + 8));
        let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
        if offset.wrapping_add(size) > h.read::<u32>(buffer) || offset < 4 {
            None
        } else {
            Some((offset, size))
        }
    }

    /// The bytes at `src` copied into the parameter's place: 3 when they
    /// don't fit, else 0.
    fn copy_in(h: &mut impl Host, param: u32, buffer: u32, src: u32) -> u32 {
        match place(h, param, buffer) {
            None => 3,
            Some((offset, size)) => {
                h.call_sym(
                    MEMCPY,
                    Conv::Cdecl,
                    &[offset.wrapping_add(buffer), src, size],
                );
                0
            }
        }
    }

    /// The texture parameter `[fx + slot]` set as `setTextureChecked`
    /// does, inline: the binding `{0x42, 0, fx + owner, texture}` written
    /// at `binding` first; 5 for a type other than 0x42, 0xd9..0xdc or
    /// 0x12, `TrophyManager_Install`'s error, 3 when it doesn't fit; 0
    /// (also without the parameter).
    fn set_texture(
        h: &mut impl Host,
        fx: u32,
        slot: u32,
        owner: u32,
        texture: u32,
        binding: u32,
    ) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let buffer = buffer(h, fx);
        let kind: u8 = h.read(param + 2);
        let owner = fx.wrapping_add(owner);
        h.write(binding, [0x42u32, 0, owner, texture]);
        if !matches!(kind, 0x42 | 0xd9..=0xdc | 0x12) {
            return 5;
        }
        if owner != 0 && texture != 0 {
            let err = this_call(h, TROPHY_INSTALL, owner, &[texture]);
            if err != 0 {
                return err;
            }
        }
        copy_in(h, param, buffer, binding)
    }

    /// The parameter `[fx + slot]` taking the bytes at `src` (a local);
    /// 3 when they don't fit, 0.
    fn set_local(h: &mut impl Host, fx: u32, slot: u32, src: u32) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let buffer = buffer(h, fx);
        copy_in(h, param, buffer, src)
    }

    /// The parameter `[fx + slot]` taking the effect's bytes at
    /// `fx + src`: 1 when that address is 0, 3 when they don't fit; 0.
    fn set_member(h: &mut impl Host, fx: u32, slot: u32, src: u32) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let buffer = buffer(h, fx);
        let src = fx.wrapping_add(src);
        if src == 0 {
            return 1;
        }
        copy_in(h, param, buffer, src)
    }

    /// A render target's texture: `[target]` unless its byte `+4` is set
    /// (0 then).
    fn texture_of(h: &impl Host, target: u32) -> u32 {
        if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        }
    }

    /// The level's target `[fx + slot + 4 * level]`.
    fn target(h: &impl Host, fx: u32, slot: u32, level: u32) -> u32 {
        h.read(fx + slot + 4 * level)
    }

    /// An unsigned word as a `float`, as `fild` and the 2^32 correction
    /// round it once at the store.
    fn unsigned_float(v: u32) -> u32 {
        x87::store(f64::from(v))
    }

    /// The level's target size `{width, height}` as floats at `at` (the
    /// texture's `+0x1c` and `+0x20`).
    fn write_size(h: &mut impl Host, fx: u32, level: u32, at: u32) {
        let texture = texture_of(h, target(h, fx, 0x90, level));
        let height: u32 = h.read(texture + 0x20);
        let width: u32 = h.read(texture + 0x1c);
        h.write(at, unsigned_float(width));
        h.write(at + 4, unsigned_float(height));
    }

    /// The binding `{0x42, 0, fx + 0x38, texture}` at `binding` through
    /// `setTextureChecked(buffer; [fx + 0x74], binding)`, when that
    /// parameter is set.
    fn bind(h: &mut impl Host, fx: u32, texture: u32, binding: u32) -> u32 {
        let param: u32 = h.read(fx + 0x74);
        if param == 0 {
            return 0;
        }
        h.write(binding, [0x42u32, 0, fx.wrapping_add(0x38), texture]);
        let buffer = buffer(h, fx);
        this_call(h, SET_TEXTURE_CHECKED, buffer, &[param, binding])
    }

    /// How a pass draws the effect's mesh instance (`+0x5c`).
    #[derive(Clone, Copy)]
    enum Draw {
        /// `renderMeshInstance` with the instance's list (`+0x10`), 0 and
        /// the renderer's contexts (`+0x13c`).
        Mesh,
        /// `renderMeshInstanceForPass(instance)`.
        ForPass,
    }

    /// A pass into the surface at `surface` (already made): the colour
    /// target, no depth target, a scene (with command 0x18 `(0, 6, 7)`
    /// when `command`), the render state, the mesh drawn, the scene
    /// ended; the first error, or 0.
    fn scene(
        h: &mut impl Host,
        fx: u32,
        r: u32,
        surface: u32,
        command: bool,
        state: u32,
        draw: Draw,
    ) -> u32 {
        let err = this_call(h, SET_COLOR_TARGET, r, &[surface, 0]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, SET_DEPTH_TARGET, r, &[0]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, BEGIN_SCENE, r, &[0]);
        if err != 0 {
            return err;
        }
        if command {
            this_call(h, QUEUE_COMMAND_18, r, &[0, 6, 7]);
        }
        let err = draw_mesh(h, fx, r, state, draw);
        if err != 0 {
            return err;
        }
        this_call(h, END_SCENE, r, &[1])
    }

    /// The render state set and the mesh instance drawn: its error.
    fn draw_mesh(h: &mut impl Host, fx: u32, r: u32, state: u32, draw: Draw) -> u32 {
        let instance: u32 = h.read(fx + 0x5c);
        match draw {
            Draw::Mesh => {
                let contexts: u32 = h.read(r + 0x13c);
                let s = h.rebase(state);
                h.write(r + 0x48, s);
                let list: u32 = h.read(instance + 0x10);
                this_call(h, RENDER_MESH_INSTANCE, r, &[instance, list, 0, contexts])
            }
            Draw::ForPass => {
                let s = h.rebase(state);
                h.write(r + 0x48, s);
                this_call(h, RENDER_FOR_PASS, r, &[instance])
            }
        }
    }

    /// A surface of `texture` at `surface`, a [`scene`] into it, the
    /// surface destroyed; the scene's error.
    #[allow(clippy::too_many_arguments)] // the pass's parts
    fn pass(
        h: &mut impl Host,
        fx: u32,
        r: u32,
        texture: u32,
        surface: u32,
        command: bool,
        state: u32,
        draw: Draw,
    ) -> u32 {
        this_call(h, SURFACE_CTOR, surface, &[texture, 0]);
        let err = scene(h, fx, r, surface, command, state, draw);
        this_call(h, SURFACE_DTOR, surface, &[]);
        err
    }

    /// One level on the way down: the size and binding parameters, the
    /// bright pass into the level's first target (`+0x90`) and the blur
    /// pass into its second (`+0x9c`); the first error.
    fn down(h: &mut impl Host, ebp: u32, fx: u32, r: u32, level: u32, source: u32) -> u32 {
        let [bright, blur, _, _] = GLOW_STATES;
        let size = ebp - 0x80;
        write_size(h, fx, level, size);
        let err = set_texture(h, fx, 0x74, 0x38, source, ebp - 0x70);
        if err != 0 {
            return err;
        }
        let err = set_local(h, fx, 0x7c, ebp - 0x60);
        if err != 0 {
            return err;
        }
        let err = set_local(h, fx, 0x80, size);
        if err != 0 {
            return err;
        }
        let first = target(h, fx, 0x90, level);
        let err = pass(h, fx, r, first, ebp - 0x50, true, bright, Draw::Mesh);
        if err != 0 {
            return err;
        }
        let texture = texture_of(h, target(h, fx, 0x90, level));
        let err = bind(h, fx, texture, ebp - 0x30);
        if err != 0 {
            return err;
        }
        let second = target(h, fx, 0x9c, level);
        pass(h, fx, r, second, ebp - 0x40, true, blur, Draw::ForPass)
    }

    /// One level on the way up: the parameters from the level's second
    /// target, the bright pass into its first target, then the combining
    /// scale and the combining pass into the level's second target, or
    /// for level 0 into `out` (no colour target without one); the first
    /// error.
    #[allow(clippy::too_many_arguments)] // the original's words, the host and the frame
    fn up(h: &mut impl Host, ebp: u32, fx: u32, r: u32, level: u32, out: u32, flag: u32) -> u32 {
        let [bright, _, combine, last] = GLOW_STATES;
        let size = ebp - 0xc0;
        write_size(h, fx, level, size);
        if h.read::<u32>(fx + 0x74) != 0 {
            let texture = texture_of(h, target(h, fx, 0x9c, level));
            let err = set_texture(h, fx, 0x74, 0x38, texture, ebp - 0xb0);
            if err != 0 {
                return err;
            }
        }
        if h.read::<u32>(fx + 0x7c) != 0 {
            let one = 1.0f32.to_bits();
            h.write(ebp - 0xa0, [one; 4]);
            let err = set_local(h, fx, 0x7c, ebp - 0xa0);
            if err != 0 {
                return err;
            }
        }
        let err = set_local(h, fx, 0x80, size);
        if err != 0 {
            return err;
        }
        let first = target(h, fx, 0x90, level);
        let err = pass(h, fx, r, first, ebp - 0xf0, true, bright, Draw::Mesh);
        if err != 0 {
            return err;
        }
        let texture = texture_of(h, target(h, fx, 0x90, level));
        let err = bind(h, fx, texture, ebp - 0x40);
        if err != 0 {
            return err;
        }
        let param: u32 = h.read(fx + 0x7c);
        if param != 0 {
            let at = if level < 3 {
                GLOW_SCALE_LOW
            } else {
                GLOW_SCALE_HIGH
            };
            let at = h.rebase(at);
            let mut scale = x87::store(x87::load(h.read(at)));
            if level == 0 {
                // the effect's own scale (`+0x68`), through fld/fstp
                scale = x87::store(x87::load(h.read(fx + 0x68)));
            }
            let values = ebp - 0x30;
            h.write(values, [scale, scale, scale, 1.0f32.to_bits()]);
            let buffer = buffer(h, fx);
            let err = this_call(h, SET_VALUE, buffer, &[param, values]);
            if err != 0 {
                return err;
            }
        }
        if level != 0 {
            let second = target(h, fx, 0x9c, level);
            return pass(h, fx, r, second, ebp - 0xe0, false, combine, Draw::Mesh);
        }
        // the last pass: a surface of `out` at `ebp - 0x50`, or of nothing
        // at `ebp - 0xd0`, destroyed before the scene
        let surface = if out == 0 { ebp - 0xd0 } else { ebp - 0x50 };
        this_call(h, SURFACE_CTOR, surface, &[out, 0]);
        let err = this_call(h, SET_COLOR_TARGET, r, &[surface, 0]);
        this_call(h, SURFACE_DTOR, surface, &[]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, SET_DEPTH_TARGET, r, &[0]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, BEGIN_SCENE, r, &[0]);
        if err != 0 {
            return err;
        }
        let state = if flag.to_le_bytes()[0] == 0 {
            combine
        } else {
            last
        };
        let err = draw_mesh(h, fx, r, state, Draw::Mesh);
        if err != 0 {
            return err;
        }
        this_call(h, END_SCENE, r, &[1])
    }

    /// `Phyre_PPostProcessing_PGlow_renderPasses_00a1d2b0(effect;
    /// renderer, out, texture, bloom, flag)` (thiscall, ret 0x14; only the
    /// low byte of `flag` is read): the
    /// texture parameter `+0x78` with `texture` (owner `+0x14`, binding at
    /// `ebp - 0x90`) and the byte parameters `+0x88`/`+0x8c` from
    /// `+0x6c`/`+0x70` into the parameter buffer (`[[+0x60] + 4]`); then
    /// for each of the `min([+0x64], 3)` levels down, the size, source
    /// (`bloom`, else `texture`, for level 0; the previous level's second
    /// target after) and the passes of [`down`]; then each level back up
    /// (the last first) as [`up`] does. The first error, or 0. The
    /// constant halves of the size and scale locals (0, 0 at `ebp - 0x78`
    /// and `- 0xb8`, 1.0 x 4 at `ebp - 0x60`) are written once, before
    /// the levels, when there is at least one.
    #[allow(clippy::too_many_arguments)] // the original's five words, the host and the frame
    pub fn glow_render_passes(
        h: &mut impl Host,
        ebp: u32,
        fx: u32,
        renderer: u32,
        out: u32,
        texture: u32,
        bloom: u32,
        flag: u32,
    ) -> u32 {
        let levels = h.read::<u32>(fx + 0x64).min(3);
        let err = set_texture(h, fx, 0x78, 0x14, texture, ebp - 0x90);
        if err != 0 {
            return err;
        }
        let err = set_member(h, fx, 0x88, 0x6c);
        if err != 0 {
            return err;
        }
        let err = set_member(h, fx, 0x8c, 0x70);
        if err != 0 {
            return err;
        }
        if levels == 0 {
            return 0;
        }
        let one = 1.0f32.to_bits();
        h.write(ebp - 0x78, [0u32; 2]);
        h.write(ebp - 0x60, [one; 4]);
        for level in 0..levels {
            let source = if level != 0 {
                texture_of(h, target(h, fx, 0x98, level))
            } else if bloom != 0 {
                bloom
            } else {
                texture
            };
            let err = down(h, ebp, fx, renderer, level, source);
            if err != 0 {
                return err;
            }
        }
        h.write(ebp - 0xb8, [0u32; 2]);
        for level in (0..levels).rev() {
            let err = up(h, ebp, fx, renderer, level, out, flag);
            if err != 0 {
                return err;
            }
        }
        0
    }
}

/// `0x00a127c0`'s callees: `PMatrix4_FromMatrix3x4` (thiscall `(out;
/// m)`), `PConstantBufferRing_next` (thiscall `(ring; &mapped)`),
/// `PMatrix4_RowTimes` (thiscall `(m; out, v)` -> out),
/// `Vector3_normalize` (cdecl `(out, v)` -> out), `PMatrix4_Multiply`
/// (cdecl `(out, a, b)` -> out), `sce_Vectormath_Scalar_Aos_inverse`
/// (cdecl `(out, m)`), `PMatrix4_Transpose` (cdecl `(out, m)` -> out),
/// `PTextureArray_markSlot` (thiscall `(array; i, _)`),
/// `PParameterBuffer_writeParameter` (thiscall `(buffer; param, src)`),
/// `PRendererBase_queueDeferredPass` (thiscall, five words).
pub const LIGHT_CALLS: [u32; 10] = [
    0x0045_1080,
    0x004f_8460,
    0x004c_9c20,
    0x0045_25a0,
    0x004c_a360,
    0x0057_3d00,
    0x004f_93f0,
    0x0059_d320,
    0x004d_57d0,
    0x005b_6620,
];
/// `_CItan` and `_CIcos` (the import thunks: `ST0` in and out).
pub const LIGHT_INTRINSICS: [u32; 2] = [0x0094_97be, 0x0094_98c2];
/// The `f64` threshold a main-light colour channel must pass (1e-4 as a
/// `float`, widened).
pub const LIGHT_THRESHOLD: u32 = 0x00b3_eb50;
/// `g_ConeAngleEpsilon` (`f64` 1e-6), taken off the cone angles before
/// their cosines.
pub const CONE_EPSILON: u32 = 0x00b3_68a8;
/// The deferred-lighting pass's render state (`renderer + 0x48`).
pub const LIGHTING_STATE: u32 = 0x0194_4c1c;
/// `0x00a127c0`'s frame below `ebp` (down to the product at
/// `ebp - 0x1e4`), in words.
pub const LIGHTS_FRAME_WORDS: u32 = 0x1e4 / 4;

pub mod lights {
    //! `0x00a127c0` over game memory.

    use super::{
        CONE_EPSILON, LIGHT_CALLS, LIGHT_INTRINSICS, LIGHT_THRESHOLD, LIGHTING_STATE, MEMCPY,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// A `float` through `fld`/`fstp` (a signalling NaN comes out quiet).
    fn q(h: &impl Host, at: u32) -> u32 {
        x87::store(x87::load(h.read(at)))
    }

    /// A stored `float` as the x87 holds it.
    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// `x` rounded to a `float` and back (an `fstp m32` reloaded).
    fn st(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    /// `n` floats from `src` to `dst` through the x87.
    fn copy_floats(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        for k in 0..n {
            let v = q(h, src + 4 * k);
            h.write(dst + 4 * k, v);
        }
    }

    /// `v / |v|` as the original computes it inline: the partial sums of
    /// squares, the root (`_CIsqrt`, rounding as `fsqrt`) and its
    /// reciprocal each stored as a `float`, then each component times
    /// the reciprocal; the three `float`s.
    fn normalise(v: [f64; 3]) -> [u32; 3] {
        let s = st(x87::mul(v[0], v[0]));
        let s = st(x87::add(s, x87::mul(v[1], v[1])));
        let s = st(x87::add(x87::mul(v[2], v[2]), s));
        let root = st(x87::sqrt(s));
        let inv = st(x87::div(1.0, root));
        v.map(|c| x87::store(x87::mul(inv, c)))
    }

    /// The channel mask at `ebp - 0x20`: four zeros, 1.0 in channel `i`
    /// when `on`; the four words. An index past 3 would write the
    /// original's frame words above the mask (the cookie, the exception
    /// record, the saved `ebp`) and end it; the port writes nothing then.
    fn mask(h: &mut impl Host, ebp: u32, on: bool, i: u32) -> [u32; 4] {
        let at = ebp - 0x20;
        h.write(at, [0u32; 4]);
        if on && i < 4 {
            h.write(at + 4 * i, 1.0f32.to_bits());
        }
        h.read(at)
    }

    /// `PMatrix4_RowTimes(view at ebp - 0x164; out, v)`: the first three
    /// floats at the address it returns.
    fn view_times(h: &mut impl Host, ebp: u32, out: u32, v: u32) -> [f64; 3] {
        let row_times = LIGHT_CALLS[2];
        let r = h.call_sym(row_times, Conv::Thiscall(ebp - 0x164), &[out, v]);
        [ld(h, r), ld(h, r + 4), ld(h, r + 8)]
    }

    /// The point `{x, y, z, w}` at `at` (the record's floats through the
    /// x87, then `w`).
    fn put_point(h: &mut impl Host, at: u32, src: u32, w: u32) {
        let p = [q(h, src), q(h, src + 4), q(h, src + 8)];
        h.write(at, p);
        h.write(at + 12, w);
    }

    /// The main light (the effect's `+0x3e24` direction, `+0x3e44`
    /// colour, `+0x3e78`/`+0x3e7c` channel) as the buffer's first record
    /// at `out`: the colour, the channel mask, and the direction through
    /// the view (`w` 0) normalised by `Vector3_normalize` at `ebp - 0x20`.
    fn main_light(h: &mut impl Host, ebp: u32, fx: u32, out: u32) {
        let normalize = LIGHT_CALLS[3];
        put_point(h, ebp - 0x50, fx + 0x3e24, 0);
        let r = view_times(h, ebp, ebp - 0xb8, ebp - 0x50);
        let v = ebp - 0x20;
        h.write(v, r.map(x87::store));
        let n = h.call_sym(normalize, Conv::Cdecl, &[ebp - 0xb8, v]);
        copy_floats(h, v, n, 3);
        copy_floats(h, out + 0x20, v, 3);
        copy_floats(h, out, fx + 0x3e44, 4);
        let on = h.read::<u32>(fx + 0x3e78) != 0;
        let i: u32 = h.read(fx + 0x3e7c);
        let m = mask(h, ebp, on, i);
        h.write(out + 0x10, m);
    }

    /// Whether the main light is on: a colour channel (`+0x3e44..`) above
    /// the threshold (NaN is not).
    fn main_light_on(h: &impl Host, fx: u32) -> bool {
        let at = h.rebase(LIGHT_THRESHOLD);
        let threshold = f64::from_bits(h.read(at));
        (0..3).any(|k| ld(h, fx + 0x3e44 + 4 * k) > threshold)
    }

    /// A directional light (type 1; `rec` is the light's `+0x28`
    /// relative base) as a 0x30-byte record at `out`: its colour
    /// (`rec + 8`), channel mask (`rec + 0x3c`/`+ 0x40`) and its
    /// direction (`rec - 0x18`, `w` 0, at `ebp - 0x64`) through the view
    /// normalised (copied at `ebp - 0x50`).
    fn directional(h: &mut impl Host, ebp: u32, rec: u32, out: u32) {
        put_point(h, ebp - 0x64, rec - 0x18, 0);
        let r = view_times(h, ebp, ebp - 0xb8, ebp - 0x64);
        let n = normalise(r);
        h.write(ebp - 0x50, n);
        copy_floats(h, out + 0x20, ebp - 0x50, 3);
        copy_floats(h, out, rec + 8, 4);
        let on = h.read::<u32>(rec + 0x3c) != 0;
        let i: u32 = h.read(rec + 0x40);
        let m = mask(h, ebp, on, i);
        h.write(out + 0x10, m);
    }

    /// `ST0` in, a `float` out (the store after the call).
    fn intrinsic(h: &mut impl Host, f: u32, x: f64) -> f64 {
        let f = h.rebase(f);
        f64::from(h.call_intrinsic(f, x))
    }

    /// Another light (type other than 1) as a 0xa0-byte record at `out`:
    /// the inverse transpose of view times its 3x4 (`rec - 0x28`); its
    /// position (`rec - 8`, `w` 1, at `ebp - 0x64`) through the view;
    /// its direction (`rec - 0x18`, `w` 0, at `ebp - 0xdc`) through the
    /// view normalised; the range words (`rec + 0x2c`, `+ 0x30`), `+0x30`
    /// times `tan` of the outer angle (`rec + 0x38`), the cosines of
    /// both angles (`+0x34`, `+0x38`) less the epsilon, `tan` of the
    /// outer angle, the colour, the channel mask, the type and
    /// `rec + 0x4c`.
    fn other(h: &mut impl Host, ebp: u32, rec: u32, out: u32) {
        let [from_3x4, _, _, _, multiply, inverse, transpose, ..] = LIGHT_CALLS;
        let [tan, cos] = LIGHT_INTRINSICS;
        let local = ebp - 0x1a4;
        h.call_sym(from_3x4, Conv::Thiscall(local), &[rec - 0x28]);
        let p = h.call_sym(multiply, Conv::Cdecl, &[ebp - 0x1e4, ebp - 0x164, local]);
        h.call_sym(inverse, Conv::Cdecl, &[ebp - 0xa4, p]);
        let t = h.call_sym(transpose, Conv::Cdecl, &[ebp - 0x1e4, ebp - 0xa4]);
        copy_floats(h, ebp - 0xa4, t, 16);
        copy_floats(h, out, ebp - 0xa4, 16);
        put_point(h, ebp - 0x64, rec - 8, 1.0f32.to_bits());
        let row_times = LIGHT_CALLS[2];
        h.call_sym(
            row_times,
            Conv::Thiscall(ebp - 0x164),
            &[ebp - 0xb8, ebp - 0x64],
        );
        put_point(h, ebp - 0xdc, rec - 0x18, 0);
        let r = view_times(h, ebp, ebp - 0xc8, ebp - 0xdc);
        let n = normalise(r);
        h.write(ebp - 0x50, n);
        copy_floats(h, out + 0x60, ebp - 0xb8, 3);
        copy_floats(h, out + 0x70, ebp - 0x50, 3);
        copy_floats(h, out + 0x88, rec + 0x2c, 2);
        let outer = ld(h, rec + 0x38);
        let t = intrinsic(h, tan, st(outer));
        let scaled = x87::store(x87::mul(ld(h, rec + 0x30), t));
        h.write(out + 0x6c, scaled);
        let eps = f64::from_bits(h.read(h.rebase(CONE_EPSILON)));
        let inner = ld(h, rec + 0x34);
        let c = intrinsic(h, cos, st(x87::sub(inner, eps)));
        h.write(out + 0x80, x87::store(c));
        let c = intrinsic(h, cos, st(x87::sub(outer, eps)));
        h.write(out + 0x84, x87::store(c));
        let t = intrinsic(h, tan, st(outer));
        h.write(out + 0x90, x87::store(t));
        copy_floats(h, out + 0x50, rec + 8, 4);
        let on = h.read::<u32>(rec + 0x3c) != 0;
        let i: u32 = h.read(rec + 0x40);
        let m = mask(h, ebp, on, i);
        h.write(out + 0x40, m);
        let kind: u32 = h.read(rec + 0x28);
        h.write(out + 0x94, kind);
        let v = q(h, rec + 0x4c);
        h.write(out + 0x7c, v);
    }

    /// `PTextureArray_markSlot(array; i, slot)` for the ring's next slot
    /// (`[+4] + 1`, 0 at the count `[+0xc] & 0x7fffffff`, a count of 0
    /// read as 1); on success the slot is the ring's and byte `+8` is
    /// cleared. Its error.
    fn mark(h: &mut impl Host, ring: u32, slot_at: u32) -> u32 {
        let mark_slot = LIGHT_CALLS[7];
        let next = h.read::<u32>(ring + 4).wrapping_add(1);
        let count = match h.read::<u32>(ring + 0xc) & 0x7fff_ffff {
            0 => 1,
            n => n,
        };
        let i = if next == count { 0 } else { next };
        let err = h.call_sym(mark_slot, Conv::Thiscall(ring), &[i, slot_at]);
        if err == 0 {
            h.write(ring + 4, i);
            h.write::<u8>(ring + 8, 0);
        }
        err
    }

    /// The parameter buffer (`[[fx + 0x3ed8] + 4]`).
    fn buffer(h: &impl Host, fx: u32) -> u32 {
        let params: u32 = h.read(fx + 0x3ed8);
        h.read(params + 4)
    }

    /// The bytes at `src` into the parameter's place: 3 when they don't
    /// fit (an offset below 4 or past the buffer's size word), else 0.
    fn copy_in(h: &mut impl Host, param: u32, buffer: u32, src: u32) -> u32 {
        let offset = u32::from(h.read::<u16>(param + 8));
        let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
        if offset.wrapping_add(size) > h.read::<u32>(buffer) || offset < 4 {
            return 3;
        }
        h.call_sym(
            MEMCPY,
            Conv::Cdecl,
            &[offset.wrapping_add(buffer), src, size],
        );
        0
    }

    /// A ring parameter `[fx + slot]`: the binding `{0x4a, the ring's
    /// slot, 0, ring}` at `at`; 5 for a type other than 0x4a, 0x13, 0x90,
    /// 0x91, 3 when it doesn't fit; 0.
    fn ring_param(h: &mut impl Host, fx: u32, slot: u32, ring: u32, at: u32) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let ring: u32 = h.read(fx + ring);
        let current: u32 = h.read(ring + 4);
        h.write(at, [0x4au32, current, 0, ring]);
        let buffer = buffer(h, fx);
        if !matches!(h.read::<u8>(param + 2), 0x4a | 0x13 | 0x90 | 0x91) {
            return 5;
        }
        copy_in(h, param, buffer, at)
    }

    /// A texture parameter `[fx + slot]` of the texture `[fx + texture]`:
    /// the binding `{0x49, 0, 0, texture, 0}` at `at`; 5 for a type other
    /// than 0x49, 0x14, 0x15 or a texture without flag 8 (`+8`); then the
    /// bytes copied in, or `writeParameter`'s result when `write`; 0
    /// without the parameter.
    fn texture_param(
        h: &mut impl Host,
        fx: u32,
        slot: u32,
        texture: u32,
        at: u32,
        write: bool,
    ) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let buffer = buffer(h, fx);
        let texture: u32 = h.read(fx + texture);
        h.write(at, [0x49u32, 0, 0, texture, 0]);
        if !matches!(h.read::<u8>(param + 2), 0x49 | 0x14 | 0x15) {
            return 5;
        }
        if texture != 0 && h.read::<u8>(texture + 8) & 8 == 0 {
            return 5;
        }
        if write {
            let write_parameter = LIGHT_CALLS[8];
            h.call_sym(write_parameter, Conv::Thiscall(buffer), &[param, at])
        } else {
            copy_in(h, param, buffer, at)
        }
    }

    /// The parameters after the two light buffers, the pass queued, and
    /// the last parameter: the first error, or 0.
    fn parameters(h: &mut impl Host, ebp: u32, fx: u32, renderer: u32) -> u32 {
        let [.., write_parameter, queue_pass] = LIGHT_CALLS;
        let err = ring_param(h, fx, 0x4000, 0x3fc8, ebp - 0xec);
        if err != 0 {
            return err;
        }
        let err = ring_param(h, fx, 0x4004, 0x3fcc, ebp - 0xfc);
        if err != 0 {
            return err;
        }
        for (slot, src) in [(0x4008, ebp - 0x54), (0x400c, ebp - 0x40)] {
            let param: u32 = h.read(fx + slot);
            if param != 0 {
                let buffer = buffer(h, fx);
                let err = copy_in(h, param, buffer, src);
                if err != 0 {
                    return err;
                }
            }
        }
        let param: u32 = h.read(fx + 0x3ffc);
        if param != 0 {
            let buffer = buffer(h, fx);
            let src = fx.wrapping_add(0x3e90);
            if src == 0 {
                return 1;
            }
            let err = copy_in(h, param, buffer, src);
            if err != 0 {
                return err;
            }
        }
        let err = texture_param(h, fx, 0x4014, 0x3fd0, ebp - 0x124, false);
        if err != 0 {
            return err;
        }
        let err = texture_param(h, fx, 0x4028, 0x3fd4, ebp - 0x110, true);
        if err != 0 {
            return err;
        }
        // the pass over the target's size in 8-pixel tiles
        let target: u32 = h.read(fx + 0x3fc4);
        let texture = if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        };
        let width: u32 = h.read(texture + 0x1c);
        let height: u32 = h.read(texture + 0x20);
        let state = h.rebase(LIGHTING_STATE);
        h.write(renderer + 0x48, state);
        let tiles = |n: u32| (n >> 3) + u32::from(n & 7 != 0);
        let holder: u32 = h.read(fx + 0x3ed8);
        let material: u32 = h.read(holder);
        let instance: u32 = h.read(holder + 4);
        let err = h.call_sym(
            queue_pass,
            Conv::Thiscall(renderer),
            &[material, instance, tiles(width), tiles(height), 1],
        );
        if err != 0 {
            return err;
        }
        let param: u32 = h.read(fx + 0x4010);
        if param == 0 {
            return 0;
        }
        let shadow: u32 = h.read(fx + 0x3fd0);
        let at = ebp - 0xcc;
        h.write(at, [0x48u32, 0, 0, shadow, 0]);
        if h.read::<u8>(param + 2) != 0x48 {
            return 5;
        }
        let buffer = buffer(h, fx);
        h.call_sym(write_parameter, Conv::Thiscall(buffer), &[param, at])
    }

    /// `Phyre_PPostProcessing_PDeferredLighting_uploadLights_00a127c0(effect;
    /// renderer, view)` (thiscall, ret 8): the 4x4 of the 3x4 `view` (at
    /// `ebp - 0xa4`, copied to `ebp - 0x164`); the light buffer of the
    /// ring `+0x3fcc` (mapped through `PConstantBufferRing_next` into
    /// `ebp - 0x38`): the main light when on ([`main_light`]), then each
    /// directional light of the `[+0x3ed0]` records (0x7c bytes from
    /// `+0x14`) as [`directional`] does, counted at `ebp - 0x40`; the
    /// ring's slot marked; the buffer of the ring `+0x3fc8` (at
    /// `ebp - 0x3c`): every other light as [`other`] does, counted at
    /// `ebp - 0x54`; that ring's slot marked; then the parameters, the
    /// deferred pass and the shadow binding ([`parameters`]). The first
    /// error, or 0. The two ring cursors' constructor and destructor
    /// stubs (`Phyre_ReturnThis_005844e0`, `0x005859f0`) do nothing and
    /// are not called.
    pub fn upload_lights(h: &mut impl Host, ebp: u32, fx: u32, renderer: u32, view: u32) -> u32 {
        let [from_3x4, next, ..] = LIGHT_CALLS;
        h.write(ebp - 0xa8, renderer);
        h.call_sym(from_3x4, Conv::Thiscall(ebp - 0xa4), &[view]);
        let m: [u32; 16] = h.read(ebp - 0xa4);
        h.write(ebp - 0x164, m);
        h.write(ebp - 0x54, 0u32);
        h.write(ebp - 0x40, 0u32);
        let ring: u32 = h.read(fx + 0x3fcc);
        let err = h.call_sym(next, Conv::Thiscall(ring), &[ebp - 0x38]);
        if err != 0 {
            return err;
        }
        let lights: u32 = h.read(ebp - 0x38);
        let count: u32 = h.read(fx + 0x3ed0);
        let mut made = 0u32;
        if main_light_on(h, fx) {
            main_light(h, ebp, fx, lights);
            made = 1;
            h.write(ebp - 0x40, made);
        }
        for i in 0..count {
            let rec = fx + 0x3c + 0x7c * i;
            if h.read::<u32>(rec + 0x28) == 1 {
                directional(h, ebp, rec, lights + 0x30 * made);
                made += 1;
                h.write(ebp - 0x40, made);
            }
        }
        let ring: u32 = h.read(fx + 0x3fcc);
        let err = mark(h, ring, ebp - 0x38);
        if err != 0 {
            return err;
        }
        let ring: u32 = h.read(fx + 0x3fc8);
        let err = h.call_sym(next, Conv::Thiscall(ring), &[ebp - 0x3c]);
        if err != 0 {
            return err;
        }
        let others: u32 = h.read(ebp - 0x3c);
        let mut made = 0u32;
        for i in 0..h.read::<u32>(fx + 0x3ed0) {
            let rec = fx + 0x3c + 0x7c * i;
            if h.read::<u32>(rec + 0x28) != 1 {
                other(h, ebp, rec, others + 0xa0 * made);
                made += 1;
                h.write(ebp - 0x54, made);
            }
        }
        let ring: u32 = h.read(fx + 0x3fc8);
        let err = mark(h, ring, ebp - 0x3c);
        if err != 0 {
            return err;
        }
        parameters(h, ebp, fx, renderer)
    }
}

/// `0x00a1f220`'s callees besides the scene calls: `PMatrix4_FromMatrix3x4`,
/// `sce_Vectormath_Scalar_Aos_inverse`, `Phyre_maxFloat_005126e0` (cdecl
/// two floats -> `ST0`), `ShaderParamBuffer_SetValue2` (thiscall
/// `(buffer; param, src)`), `uploadLights` (`0x00a127c0`),
/// `setTextureParameters` (`0x00a13400`),
/// `PRendererBase_setClearColor` (thiscall, four floats), the light
/// drawer `0x00a206d0` (thiscall `(effect; renderer, light)`).
pub const LIGHTING_CALLS: [u32; 8] = [
    0x0045_1080,
    0x0057_3d00,
    0x0051_26e0,
    0x0063_86c0,
    0x00a1_27c0,
    0x00a1_3400,
    0x005b_ada0,
    0x00a2_06d0,
];
/// The smallest depth range `maxFloat` keeps (`float` 1e-4).
pub const DEPTH_EPSILON: u32 = 0x00b9_2154;
/// The deferred lighting's two render states (`renderer + 0x48`): the
/// light-buffer pass and the final pass.
pub const LIGHTING_STATES: [u32; 2] = [0x0194_4c0c, 0x0194_4bfc];
/// `0x00a1f220`'s frame below `ebp` (down to the matrix copy at
/// `ebp - 0xf0`), in words.
pub const LIGHTING_FRAME_WORDS: u32 = 0xf0 / 4;

pub mod lighting {
    //! `0x00a1f220` over game memory.

    use super::{
        BEGIN_SCENE, DEPTH_EPSILON, END_SCENE, LIGHTING_CALLS, LIGHTING_STATES, RENDER_FOR_PASS,
        SET_COLOR_TARGET, SET_DEPTH_TARGET, SET_TEXTURE_CHECKED, SET_VALUE, SURFACE_CTOR,
        SURFACE_DTOR,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// A `float` through `fld`/`fstp`.
    fn q(h: &impl Host, at: u32) -> u32 {
        x87::store(x87::load(h.read(at)))
    }

    /// The parameter buffer (`[[fx + 0x3ed8] + 4]`).
    fn buffer(h: &impl Host, fx: u32) -> u32 {
        let params: u32 = h.read(fx + 0x3ed8);
        h.read(params + 4)
    }

    /// A render target's texture: `[target]` unless its byte `+4` is set.
    fn texture_of(h: &impl Host, target: u32) -> u32 {
        if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        }
    }

    /// `setTextureChecked(buffer; [fx + slot], at)` with the binding
    /// `{0x42, 0, fx + 0x3ee8, the target's texture}` at `at`, when the
    /// parameter is set.
    fn bind(h: &mut impl Host, fx: u32, slot: u32, target: u32, at: u32) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let texture = texture_of(h, target);
        h.write(at, [0x42u32, 0, fx.wrapping_add(0x3ee8), texture]);
        let buffer = buffer(h, fx);
        this_call(h, SET_TEXTURE_CHECKED, buffer, &[param, at])
    }

    /// `f(buffer; [fx + slot], src)` (`ShaderParamBuffer_SetValue` or
    /// `SetValue2`) when the parameter is set.
    fn set(h: &mut impl Host, f: u32, fx: u32, slot: u32, src: u32) -> u32 {
        let param: u32 = h.read(fx + slot);
        if param == 0 {
            return 0;
        }
        let buffer = buffer(h, fx);
        this_call(h, f, buffer, &[param, src])
    }

    /// The parameters: the five target textures, the depth range
    /// `{near, far, 1 / max(far - near, 1e-4), +0x3eb8}`, `+0x3ea0`, the
    /// view's reciprocal scales, `+0x3ec8`, `+0x3ecc`, the texture of
    /// `+0x4030`; the first error.
    fn parameters(h: &mut impl Host, ebp: u32, fx: u32, view: u32) -> u32 {
        let [.., max_float, set_value2, _, _, _, _] = LIGHTING_CALLS;
        let binding = ebp - 0x1c;
        for (slot, target) in [
            (0x3fd8, 0x3fb8),
            (0x3f58, 0x3fc4),
            (0x3f54, 0x3fbc),
            (0x3fdc, 0x3fc0),
            (0x3fe0, 0x3fb4),
        ] {
            let target: u32 = h.read(fx + target);
            let err = bind(h, fx, slot, target, binding);
            if err != 0 {
                return err;
            }
        }
        let param: u32 = h.read(fx + 0x3f88);
        if param != 0 {
            let range = x87::store(x87::sub(
                x87::load(h.read(fx + 0x3eb0)),
                x87::load(h.read(fx + 0x3eb4)),
            ));
            let epsilon = q(h, h.rebase(DEPTH_EPSILON));
            let wide = h.call_sym_st0_f64(max_float, Conv::Cdecl, &[range, epsilon]);
            let inverse = x87::store(x87::div(1.0, wide));
            let values = [
                q(h, fx + 0x3eb4),
                q(h, fx + 0x3eb0),
                inverse,
                q(h, fx + 0x3eb8),
            ];
            // the far value's copy at ebp - 0x40 (a frame local)
            h.write(ebp - 0x40, values[1]);
            h.write(binding, values);
            let buffer = buffer(h, fx);
            let err = this_call(h, SET_VALUE, buffer, &[param, binding]);
            if err != 0 {
                return err;
            }
        }
        let err = set(h, SET_VALUE, fx, 0x3f8c, fx.wrapping_add(0x3ea0));
        if err != 0 {
            return err;
        }
        let param: u32 = h.read(fx + 0x3f84);
        if param != 0 {
            // the view's last row, copied to ebp - 0x38 (frame locals)
            let row: [u32; 4] = h.read(view + 0x40);
            h.write(ebp - 0x38, row);
            let sx = x87::store(x87::div(1.0, x87::load(h.read(view + 0x30))));
            let sy = x87::store(x87::div(1.0, x87::load(h.read(view + 0x44))));
            h.write(binding, [sx, sy, 0, 0]);
            let buffer = buffer(h, fx);
            let err = this_call(h, SET_VALUE, buffer, &[param, binding]);
            if err != 0 {
                return err;
            }
        }
        for (slot, src) in [(0x4018, 0x3ec8), (0x401c, 0x3ecc)] {
            let err = set(h, set_value2, fx, slot, fx.wrapping_add(src));
            if err != 0 {
                return err;
            }
        }
        let target: u32 = h.read(fx + 0x4030);
        bind(h, fx, 0x4020, target, ebp - 0x38)
    }

    /// The final pass into the surface at `surface` (already made): colour
    /// target, scene, render state, the effect's mesh (`+0x3edc`) for
    /// its pass, the scene ended, both targets cleared; the first error.
    fn final_pass(h: &mut impl Host, fx: u32, r: u32, surface: u32, state: u32) -> u32 {
        let err = this_call(h, SET_COLOR_TARGET, r, &[surface, 0]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, BEGIN_SCENE, r, &[1]);
        if err != 0 {
            return err;
        }
        let s = h.rebase(state);
        h.write(r + 0x48, s);
        let instance: u32 = h.read(fx + 0x3edc);
        let err = this_call(h, RENDER_FOR_PASS, r, &[instance]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, END_SCENE, r, &[1]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, SET_COLOR_TARGET, r, &[0, 0]);
        if err != 0 {
            return err;
        }
        this_call(h, SET_DEPTH_TARGET, r, &[0])
    }

    /// The light-buffer path after its two surfaces are made: the light
    /// buffer target (`ebp - 0x50`) cleared to `+0x3e90`, the effect's
    /// main light (`+0x3e14`) and each light record drawn by
    /// `0x00a206d0`, then the final pass into `out` (surface at
    /// `ebp - 0x60`, destroyed here); the first error.
    fn light_buffer(h: &mut impl Host, ebp: u32, fx: u32, r: u32, out: u32) -> u32 {
        let [.., clear_colour, draw_light] = LIGHTING_CALLS;
        let err = this_call(h, SET_COLOR_TARGET, r, &[ebp - 0x50, 0]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, SET_DEPTH_TARGET, r, &[0]);
        if err != 0 {
            return err;
        }
        let colour = [0, 4, 8, 0xc].map(|k| q(h, fx + 0x3e90 + k));
        let err = this_call(h, clear_colour, r, &colour);
        if err != 0 {
            return err;
        }
        let err = this_call(h, BEGIN_SCENE, r, &[1]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, draw_light, fx, &[r, fx.wrapping_add(0x3e14)]);
        if err != 0 {
            return err;
        }
        let mut i = 0;
        while i < h.read::<u32>(fx + 0x3ed0) {
            let light = fx.wrapping_add(0x14 + 0x7c * i);
            let err = this_call(h, draw_light, fx, &[r, light]);
            if err != 0 {
                return err;
            }
            i += 1;
        }
        let err = this_call(h, END_SCENE, r, &[1]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, SET_COLOR_TARGET, r, &[0, 0]);
        if err != 0 {
            return err;
        }
        let err = this_call(h, SET_DEPTH_TARGET, r, &[0]);
        if err != 0 {
            return err;
        }
        let surface = ebp - 0x60;
        this_call(h, SURFACE_CTOR, surface, &[out, 0]);
        let err = final_pass(h, fx, r, surface, LIGHTING_STATES[1]);
        this_call(h, SURFACE_DTOR, surface, &[]);
        err
    }

    /// `Phyre_PPostProcessing_PDeferredLighting_render_00a1f220(effect;
    /// renderer, view, out)` (thiscall, ret 0xc): 0x14 unless the effect
    /// is set up (`+0x3ed4` 3, the targets `+0x3fbc`, `+0x3fb0`,
    /// `+0x3fc0`, `+0x3fb8`, `+0x3fc4`); `out` 0 means `+0x3fb0`; the
    /// inverse of the 3x4 `view` (at `ebp - 0xb0`, from a copy at
    /// `ebp - 0xf0`; never read); the parameters ([`parameters`], the
    /// first error returned); with the byte `+0x402c` the light
    /// constants (`0x00a127c0`) and texture parameters (`0x00a13400`,
    /// both results dropped) and the final pass into `out` (surface at
    /// `ebp - 0x38`, state 0x01944c0c); else the light buffer
    /// ([`light_buffer`]: surfaces of `+0x3fb4` at `ebp - 0x50` and of
    /// `+0x3fc4` at `ebp - 0x70`, made first and destroyed last). The
    /// first error, or 0.
    pub fn render(h: &mut impl Host, ebp: u32, fx: u32, renderer: u32, view: u32, out: u32) -> u32 {
        let [from_3x4, inverse, _, _, upload_lights, texture_params, ..] = LIGHTING_CALLS;
        let ready = h.read::<u32>(fx + 0x3ed4) == 3
            && [0x3fbc, 0x3fb0, 0x3fc0, 0x3fb8, 0x3fc4]
                .into_iter()
                .all(|k| h.read::<u32>(fx + k) != 0);
        if !ready {
            return 0x14;
        }
        let out = if out == 0 { h.read(fx + 0x3fb0) } else { out };
        let lights: u32 = h.read(fx + 0x3fb4);
        h.write(ebp - 0x3c, lights);
        let m = ebp - 0xb0;
        this_call(h, from_3x4, m, &[view]);
        let words: [u32; 16] = h.read(m);
        h.write(ebp - 0xf0, words);
        h.call_sym(inverse, Conv::Cdecl, &[m, ebp - 0xf0]);
        let err = parameters(h, ebp, fx, view);
        if err != 0 {
            return err;
        }
        if h.read::<u8>(fx + 0x402c) != 0 {
            this_call(h, upload_lights, fx, &[renderer, view]);
            this_call(h, texture_params, fx, &[renderer, view]);
            let surface = ebp - 0x38;
            this_call(h, SURFACE_CTOR, surface, &[out, 0]);
            let err = final_pass(h, fx, renderer, surface, LIGHTING_STATES[0]);
            this_call(h, SURFACE_DTOR, surface, &[]);
            return err;
        }
        let target: u32 = h.read(fx + 0x3fc4);
        this_call(h, SURFACE_CTOR, ebp - 0x50, &[lights, 0]);
        this_call(h, SURFACE_CTOR, ebp - 0x70, &[target, 0]);
        let err = light_buffer(h, ebp, fx, renderer, out);
        this_call(h, SURFACE_DTOR, ebp - 0x70, &[]);
        this_call(h, SURFACE_DTOR, ebp - 0x50, &[]);
        err
    }
}

/// `0x00a206d0`'s callees: `PMatrix4_FromMatrix3x4`, `PMatrix4_RowTimes`,
/// `PMatrix4_Copy` (thiscall `(this; m)`),
/// `PRendererBase_clearScene` (thiscall, one word).
pub const LIGHT_DRAW_CALLS: [u32; 4] = [0x0045_1080, 0x004c_9c20, 0x0045_3340, 0x005b_6330];
/// The light shapes' render states (`renderer + 0x48`) by light type:
/// 0 (the effect's `+0x3ee0` mesh), 2 (`+0x3ee4`), 1 (`+0x3edc`).
pub const LIGHT_DRAW_STATES: [u32; 3] = [0x0194_4bcc, 0x0194_4bdc, 0x0194_4bec];
/// `0x00a206d0`'s frame below `ebp` (down to `ebp - 0x130`), in words.
pub const LIGHT_DRAW_FRAME_WORDS: u32 = 0x130 / 4;

pub mod light_draw {
    //! `0x00a206d0` over game memory.

    use super::{
        CONE_EPSILON, LIGHT_DRAW_CALLS, LIGHT_DRAW_STATES, LIGHT_INTRINSICS, MEMCPY,
        RENDER_MESH_INSTANCE,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// A `float` through `fld`/`fstp`.
    fn q(h: &impl Host, at: u32) -> u32 {
        x87::store(x87::load(h.read(at)))
    }

    /// The parameter buffer (`[[fx + 0x3ed8] + 4]`).
    fn buffer(h: &impl Host, fx: u32) -> u32 {
        let params: u32 = h.read(fx + 0x3ed8);
        h.read(params + 4)
    }

    /// The bytes at `src` into the place of the parameter `param`:
    /// `false` (3) when they don't fit (an offset below 4 or past the
    /// buffer's size word).
    fn copy_in(h: &mut impl Host, fx: u32, param: u32, src: u32) -> bool {
        let buffer = buffer(h, fx);
        let offset = u32::from(h.read::<u16>(param + 8));
        let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
        if offset.wrapping_add(size) > h.read::<u32>(buffer) || offset < 4 {
            return false;
        }
        h.call_sym(
            MEMCPY,
            Conv::Cdecl,
            &[offset.wrapping_add(buffer), src, size],
        );
        true
    }

    /// The scale matrix of the light's shape: `diag(s, s, s, 1)` with
    /// `s = +0x58`, for a spot light (type 2) `diag(t, t, s, 1)` with
    /// `t = s * tan(+0x60)`, at `ebp - 0x70`, times the light's 3x4
    /// (`PMatrix4_FromMatrix3x4` at `ebp - 0x70`, copied to
    /// `ebp - 0x110`) row by row (`RowTimes` into `ebp - 0x130`,
    /// `- 0x120`, `- 0x14`, `- 0xd0`), copied back over the scale matrix
    /// and through `PMatrix4_Copy` into `ebp - 0x110`.
    fn shape(h: &mut impl Host, ebp: u32, light: u32) {
        let [from_3x4, row_times, copy, _] = LIGHT_DRAW_CALLS;
        let m = ebp - 0x70;
        let world = ebp - 0x110;
        h.call_sym(from_3x4, Conv::Thiscall(m), &[light]);
        let s = q(h, light + 0x58);
        let mut scale = [s, s, s];
        let words: [u32; 16] = h.read(m);
        h.write(world, words);
        if h.read::<u32>(light + 0x50) == 2 {
            let tan = h.rebase(LIGHT_INTRINSICS[0]);
            let t = h.call_intrinsic(tan, x87::load(h.read(light + 0x60)));
            let t = x87::store(x87::mul(f64::from(t), x87::load(h.read(light + 0x58))));
            scale = [t, t, s];
        }
        let mut diag = [0u32; 16];
        diag[0] = scale[0];
        diag[5] = scale[1];
        diag[10] = scale[2];
        diag[15] = 1.0f32.to_bits();
        h.write(m, diag);
        let rows = [
            (ebp - 0xd0, m + 0x30),
            (ebp - 0x14, m + 0x20),
            (ebp - 0x120, m + 0x10),
            (ebp - 0x130, m),
        ];
        let got = rows.map(|(out, v)| h.call_sym(row_times, Conv::Thiscall(world), &[out, v]));
        for (row, &r) in [0u32, 0x10, 0x20, 0x30].iter().zip(got.iter().rev()) {
            for c in 0..4 {
                let v = q(h, r + 4 * c);
                h.write(m + row + 4 * c, v);
            }
        }
        h.call_sym(copy, Conv::Thiscall(world), &[m]);
    }

    /// The parameters of the light (each when its parameter is set):
    /// its position `+0x20` (at `ebp - 0xc0`), direction `+0x10` (at
    /// `ebp - 0x80`), the shape matrix ([`shape`], `ebp - 0x110`), the
    /// words at `+0x30`, the cone `{inner - eps, outer + eps, cos, cos}`
    /// (`ebp - 0xb0`), `{+0x54, +0x58, 0, 0}` (`ebp - 0xa0`), the channel
    /// mask (`ebp - 0x90`); 3 when one doesn't fit, 1 without the words
    /// at `+0x30`, else 0.
    #[allow(clippy::too_many_lines)] // the original's seven parameters in order
    fn parameters(h: &mut impl Host, ebp: u32, fx: u32, light: u32) -> u32 {
        let param: u32 = h.read(fx + 0x3fe4);
        if param != 0 {
            let at = ebp - 0xc0;
            let v = [q(h, light + 0x20), q(h, light + 0x24), q(h, light + 0x28)];
            h.write(at, v);
            if !copy_in(h, fx, param, at) {
                return 3;
            }
        }
        let param: u32 = h.read(fx + 0x3fe8);
        if param != 0 {
            let at = ebp - 0x80;
            let v = [q(h, light + 0x10), q(h, light + 0x14), q(h, light + 0x18)];
            h.write(at, v);
            // junk read: [ebp-0x8] never written; 0 (brief 2026-10-24)
            h.write(at + 0xc, 0u32);
            if !copy_in(h, fx, param, at) {
                return 3;
            }
        }
        if h.read::<u32>(fx + 0x3f5c) != 0 {
            shape(h, ebp, light);
            let param: u32 = h.read(fx + 0x3f5c);
            if !copy_in(h, fx, param, ebp - 0x110) {
                return 3;
            }
        }
        let param: u32 = h.read(fx + 0x3fec);
        if param != 0 {
            let src = light.wrapping_add(0x30);
            if src == 0 {
                return 1;
            }
            if !copy_in(h, fx, param, src) {
                return 3;
            }
        }
        let param: u32 = h.read(fx + 0x3ff0);
        if param != 0 {
            let eps = f64::from_bits(h.read(h.rebase(CONE_EPSILON)));
            let inner = x87::store(x87::sub(x87::load(h.read(light + 0x5c)), eps));
            let outer = x87::store(x87::add(eps, x87::load(h.read(light + 0x60))));
            let cos = h.rebase(LIGHT_INTRINSICS[1]);
            let c1 = x87::store(f64::from(h.call_intrinsic(cos, x87::load(inner))));
            let c2 = x87::store(f64::from(h.call_intrinsic(cos, x87::load(outer))));
            let at = ebp - 0xb0;
            h.write(at, [inner, outer, c1, c2]);
            if !copy_in(h, fx, param, at) {
                return 3;
            }
        }
        let param: u32 = h.read(fx + 0x3ff4);
        if param != 0 {
            let at = ebp - 0xa0;
            h.write(at, [q(h, light + 0x54), q(h, light + 0x58), 0, 0]);
            if !copy_in(h, fx, param, at) {
                return 3;
            }
        }
        let param: u32 = h.read(fx + 0x3ff8);
        if param != 0 {
            // the mask at ebp - 0x14; a channel past 3 would overwrite the
            // original's cookie and end it: the port writes nothing then
            let mut mask = [0u32; 4];
            if h.read::<u32>(light + 0x64) != 0 {
                let i: u32 = h.read(light + 0x68);
                if let Some(m) = usize::try_from(i).ok().and_then(|i| mask.get_mut(i)) {
                    *m = 1.0f32.to_bits();
                }
            }
            h.write(ebp - 0x14, mask);
            let at = ebp - 0x90;
            h.write(at, mask);
            if !copy_in(h, fx, param, at) {
                return 3;
            }
        }
        0
    }

    /// `Phyre_PPostProcessing_PDeferredLighting_drawLight_00a206d0(effect;
    /// renderer, light)` (thiscall, ret 8): the light's parameters
    /// ([`parameters`], their 3 or 1 returned), `clearScene(8)` (result
    /// dropped), then by the light's type (`+0x50`) its shape drawn:
    /// type 0 the effect's `+0x3ee0` mesh, 2 `+0x3ee4`, 1 `+0x3edc`, each
    /// with its render state, through `renderMeshInstance`; its error, or
    /// 0 (also for any other type).
    pub fn draw_light(h: &mut impl Host, ebp: u32, fx: u32, renderer: u32, light: u32) -> u32 {
        let err = parameters(h, ebp, fx, light);
        if err != 0 {
            return err;
        }
        let clear_scene = LIGHT_DRAW_CALLS[3];
        h.call_sym(clear_scene, Conv::Thiscall(renderer), &[8]);
        let [s0, s2, s1] = LIGHT_DRAW_STATES;
        let (state, mesh) = match h.read::<u32>(light + 0x50) {
            0 => (s0, 0x3ee0),
            2 => (s2, 0x3ee4),
            1 => (s1, 0x3edc),
            _ => return 0,
        };
        let s = h.rebase(state);
        h.write(renderer + 0x48, s);
        let instance: u32 = h.read(fx + mesh);
        let list: u32 = h.read(instance + 0x10);
        let contexts: u32 = h.read(renderer + 0x13c);
        h.call_sym(
            RENDER_MESH_INSTANCE,
            Conv::Thiscall(renderer),
            &[instance, list, 0, contexts],
        )
    }
}

/// `0x00a04b20`'s callees besides [`crate::p6_quads`]'s:
/// `Phyre_PGeometry_FormatSize_0055bc40` (cdecl, one word),
/// `PIndexDataBlock_isBound_00562db0` (`ecx`, `al`),
/// `PIndexBufferD3D11_release_005648c0` (`ecx`),
/// `PRingBuffer_submit_00434cd0` (the index map: thiscall `(buffer;
/// &slot)`), `PConstantBufferD3D11_unmapEntry_00565000` (thiscall `(buffer;
/// i, &slot)`), `_CIcos`, `_CIsin`.
pub const CONE_CALLS: [u32; 7] = [
    0x0055_bc40,
    0x0056_2db0,
    0x0056_48c0,
    0x0043_4cd0,
    0x0056_5000,
    0x0094_98c2,
    0x0094_98c8,
];
/// `g_TwoPiF64` (the full turn the cone's rim is divided into).
pub const CONE_TWO_PI: u32 = 0x00b3_9378;
/// `g_PiF64` (the half turn of the sphere's meridians).
pub const SPHERE_PI: u32 = 0x00b9_2288;
/// `0x00a06ab0`'s frame below `ebp` (the map slots at `ebp - 0x5c` and
/// `- 0x54`), in words.
pub const SPHERE_FRAME_WORDS: u32 = 0x5c / 4;
/// `0x00a04b20`'s frame below `ebp` (the map slots at `ebp - 0x38` and
/// `- 0x30`), in words.
pub const CONE_FRAME_WORDS: u32 = 0x38 / 4;

pub mod cone {
    //! `0x00a04b20` over game memory.

    use super::{CONE_CALLS, CONE_TWO_PI, SPHERE_PI};
    use crate::p6_quads::{
        ALLOCATE_INSTANCE_LIST, CLUSTER_CREATE, INSTANCE_CTOR, INSTANCE_TYPE, IS_BOUND,
        MALLOC_ALIGNED, MAP, MESH_TYPE, MINUS_ONE, NAME_POSITION, RELEASE, SEGMENTS_CONSTRUCT,
        SEGMENTS_DESTROY, STREAM_CTOR, STREAM_FORMAT, STREAMS_SET, UNMAP,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// The next version of a ring (`[at] + 1`, 0 at the count `[count_at]
    /// & 0x7fffffff`).
    fn next_version(h: &impl Host, at: u32, count_at: u32) -> u32 {
        let v = h.read::<u32>(at).wrapping_add(1);
        if v == h.read::<u32>(count_at) & 0x7fff_ffff {
            0
        } else {
            v
        }
    }

    /// The mesh's one segment (as the full-screen meshes make it): a new
    /// 0x6c-byte block unless the mesh array (`[mesh]`) holds one
    /// already; the old data destroyed when owned (bit 31 clear) and not
    /// the block.
    fn one_segment(h: &mut impl Host, mesh: u32) {
        if h.read::<u32>(mesh) == 1 {
            return;
        }
        let block = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[0x6c, 4]);
        if block == 0 {
            return;
        }
        h.call_sym(SEGMENTS_CONSTRUCT, Conv::Cdecl, &[block, 1]);
        let head: u32 = h.read(mesh);
        let data: u32 = h.read(mesh + 4);
        if data != block && head >> 31 == 0 {
            h.call_sym(SEGMENTS_DESTROY, Conv::Cdecl, &[data, head & 0x7fff_ffff]);
        }
        h.write(mesh + 4, block);
        h.write(mesh, 1u32);
    }

    /// The rim's vertices after the apex (`(cos a, sin a, -1)` for `a`
    /// stepping by `2pi / n` from 0, each sum stored as a `float`) at
    /// `vertices`, and the two triangles of each rim edge
    /// `(0, i + 2, j + 2)`, `(j + 2, i + 2, 1)` (`j = (i + 1) % n`, 16-bit)
    /// at `indices`.
    fn rim(h: &mut impl Host, n: u32, vertices: u32, indices: u32) {
        let [.., cos, sin] = CONE_CALLS;
        let (cos, sin) = (h.rebase(cos), h.rebase(sin));
        let minus_one: u32 = h.read(h.rebase(MINUS_ONE));
        let minus_one = x87::store(x87::load(minus_one));
        let count = x87::store(f64::from(n));
        let two_pi = f64::from_bits(h.read(h.rebase(CONE_TWO_PI)));
        let step = x87::store(x87::div(two_pi, x87::load(count)));
        let mut angle = 0u32;
        for i in 0..n {
            let v = vertices.wrapping_add(0xc * i);
            let c = x87::store(f64::from(h.call_intrinsic(cos, x87::load(angle))));
            h.write(v, c);
            let s = x87::store(f64::from(h.call_intrinsic(sin, x87::load(angle))));
            h.write(v + 4, s);
            h.write(v + 8, minus_one);
            angle = x87::store(x87::add(x87::load(angle), x87::load(step)));
            let a = u16::try_from(i.wrapping_add(2) & 0xffff).unwrap_or(0);
            let b = u16::try_from(((i + 1) % n + 2) & 0xffff).unwrap_or(0);
            let t = indices.wrapping_add(0xc * i);
            h.write(t, [0u16, a, b, b, a, 1]);
        }
    }

    /// A mapped position mesh: the mesh array, its index block and its
    /// first stream.
    struct Mapped {
        mesh: u32,
        ib: u32,
        vb: u32,
    }

    /// The mesh both builders make: a mesh (`PCluster::create(cluster; 1,
    /// PMesh, 0)`) with one segment ([`one_segment`], `+0x10` = 2) and two
    /// streams when it has another count (`0x0055dc40` each,
    /// `0x00434e40`); the index block (`segment + 0x30`: `indices`
    /// 16-bit indices, `{0, vertices, indices, 0xc}`, its byte size
    /// `FormatSize(0xc) * indices` at `+0x34`, released first when bound
    /// and it changes) mapped into `index_slot`; the first stream
    /// (`0x004835d0(0xc, vertices, 1)`, its size `+0x38` released first
    /// when bound and it changes, semantic `{0, POSITION, 2}`) mapped
    /// into `vertex_slot`.
    fn map_mesh(
        h: &mut impl Host,
        cluster: u32,
        (indices, vertices): (u32, u32),
        (index_slot, vertex_slot): (u32, u32),
    ) -> Mapped {
        let [format_size, index_bound, index_release, index_map, ..] = CONE_CALLS;
        let mesh_type = h.rebase(MESH_TYPE);
        let mesh = this_call(h, CLUSTER_CREATE, cluster, &[1, mesh_type, 0]);
        one_segment(h, mesh);
        let seg: u32 = h.read(mesh + 4);
        h.write(seg + 0x10, 2u32);
        if h.read::<u32>(seg + 0x28) != 2 {
            let block = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[0x80, 4]);
            if block != 0 {
                for k in 0..2 {
                    let s = block.wrapping_add(0x40 * k);
                    if s != 0 {
                        this_call(h, STREAM_CTOR, s, &[]);
                    }
                }
                this_call(h, STREAMS_SET, seg + 0x28, &[block, 2]);
            }
        }
        let ib = seg + 0x30;
        h.write(ib + 8, indices);
        h.write::<u8>(ib + 0xc, 0xc);
        h.write(ib, 0u32);
        h.write(ib + 4, vertices);
        let bytes = h
            .call_sym(format_size, Conv::Cdecl, &[0xc])
            .wrapping_mul(indices);
        if h.read::<u32>(ib + 0x34) != bytes {
            if this_call(h, index_bound, ib, &[]) & 0xff != 0 {
                this_call(h, index_release, ib, &[]);
            }
            h.write(ib + 0x34, bytes);
        }
        this_call(h, index_map, ib, &[index_slot]);
        let vb: u32 = h.read(seg + 0x2c);
        this_call(h, STREAM_FORMAT, vb, &[0xc, vertices, 1]);
        let size = h.read::<u32>(vb).wrapping_mul(h.read(vb + 4));
        if h.read::<u32>(vb + 0x38) != size {
            if this_call(h, IS_BOUND, vb, &[]) & 0xff != 0 {
                this_call(h, RELEASE, vb, &[]);
            }
            h.write(vb + 0x38, size);
        }
        let record: u32 = h.read(vb + 0xc);
        h.write::<u8>(record + 8, 2);
        let name = h.rebase(NAME_POSITION);
        h.write(record + 4, name);
        h.write(record, 0u32);
        this_call(h, MAP, vb, &[vertex_slot]);
        Mapped { mesh, ib, vb }
    }

    /// Both blocks unmapped at their next version (kept, and the busy
    /// byte cleared, on success), then a `PMeshInstance` of the mesh
    /// (`allocateInstanceList(cluster; 1, PMeshInstance, 0)`,
    /// `0x004f27e0`'s `eax`), or 0.
    fn finish(
        h: &mut impl Host,
        cluster: u32,
        m: &Mapped,
        (index_slot, vertex_slot): (u32, u32),
    ) -> u32 {
        let index_unmap = CONE_CALLS[4];
        let Mapped { mesh, ib, vb } = *m;
        let version = next_version(h, vb + 0x10, vb + 0x18);
        if this_call(h, UNMAP, vb, &[version, vertex_slot]) == 0 {
            h.write(vb + 0x10, version);
            h.write::<u8>(vb + 0x14, 0);
        }
        let version = next_version(h, ib + 0x10, ib + 0x14);
        if this_call(h, index_unmap, ib, &[version, index_slot]) == 0 {
            h.write(ib + 0x10, version);
            h.write::<u8>(ib + 0xe, 0);
        }
        let instance_type = h.rebase(INSTANCE_TYPE);
        let list = this_call(h, ALLOCATE_INSTANCE_LIST, cluster, &[1, instance_type, 0]);
        if list == 0 {
            return 0;
        }
        let instance: u32 = h.read(list + 0x34);
        if instance == 0 {
            return 0;
        }
        this_call(h, INSTANCE_CTOR, instance, &[mesh])
    }

    /// `Phyre_PPostProcessing_PDeferredLighting_createCone_00a04b20(cluster,
    /// n)` (cdecl): the mesh of [`map_mesh`] with 6n indices and n + 2
    /// vertices (index slot `ebp - 0x38`, vertex slot `ebp - 0x30`); the
    /// apex `(0, 0, 0)` and `(0, 0, -1)`, the rim ([`rim`]); then
    /// [`finish`]. Writes its argument slot `+4` (the instance) as
    /// scratch; no caller reads it.
    pub fn create_cone(h: &mut impl Host, ebp: u32, cluster: u32, n: u32) -> u32 {
        let slots = (ebp - 0x38, ebp - 0x30);
        let counts = (n.wrapping_mul(6), n.wrapping_add(2));
        let m = map_mesh(h, cluster, counts, slots);
        let vertices: u32 = h.read(slots.1);
        let indices: u32 = h.read(slots.0);
        let minus_one = x87::store(x87::load(h.read(h.rebase(MINUS_ONE))));
        h.write(vertices, [0u32, 0, 0, 0, 0, minus_one]);
        rim(h, n, vertices.wrapping_add(0x18), indices);
        finish(h, cluster, &m, slots)
    }

    /// A CRT intrinsic of a stored `float`, its result stored as one.
    fn trig(h: &mut impl Host, f: u32, a: u32) -> f64 {
        let r = h.call_intrinsic(f, x87::load(a));
        x87::load(x87::store(f64::from(r)))
    }

    /// The sphere's `(n + 1)²` vertices and `n²` quads: for each `u` and
    /// `v` in `0..=n` (`phi = u * 2pi/n`, `theta = v * pi/n`, each step
    /// summed as a `float`) the point `(cos phi sin theta, cos theta,
    /// sin phi sin theta)`, and for `u, v < n` the triangles
    /// `(a, b, a + 1)`, `(a + 1, b, b + 1)` with `a = (n + 1) u + v`,
    /// `b = (n + 1)(u + 1) + v` (16-bit) at quad `n u + v`.
    fn globe(h: &mut impl Host, n: u32, vertices: u32, indices: u32) {
        let [.., cos, sin] = CONE_CALLS;
        let (cos, sin) = (h.rebase(cos), h.rebase(sin));
        let count = x87::load(x87::store(f64::from(n)));
        let two_pi = f64::from_bits(h.read(h.rebase(CONE_TWO_PI)));
        let pi = f64::from_bits(h.read(h.rebase(SPHERE_PI)));
        let step_u = x87::load(x87::store(x87::div(two_pi, count)));
        let step_v = x87::load(x87::store(x87::div(pi, count)));
        let row = n.wrapping_add(1);
        let mut v_at = vertices;
        let mut phi = 0u32;
        let mut u = 0u32;
        loop {
            let cu = trig(h, cos, phi);
            let su = trig(h, sin, phi);
            let mut theta = 0u32;
            let mut v = 0u32;
            loop {
                let sv = trig(h, sin, theta);
                h.write(v_at, x87::store(x87::mul(cu, sv)));
                let cv = trig(h, cos, theta);
                h.write(v_at + 4, x87::store(cv));
                h.write(v_at + 8, x87::store(x87::mul(su, sv)));
                v_at = v_at.wrapping_add(0xc);
                if v < n && u < n {
                    let (u16_, v16) = (u & 0xffff, v & 0xffff);
                    let a = row.wrapping_mul(u16_).wrapping_add(v16);
                    let b = u16_.wrapping_add(1).wrapping_mul(row).wrapping_add(v16);
                    let w = |x: u32| u16::try_from(x & 0xffff).unwrap_or(0);
                    let quad = n.wrapping_mul(u).wrapping_add(v);
                    let t = indices.wrapping_add(quad.wrapping_mul(0xc));
                    h.write(
                        t,
                        [
                            w(a),
                            w(b),
                            w(a.wrapping_add(1)),
                            w(a.wrapping_add(1)),
                            w(b),
                            w(b.wrapping_add(1)),
                        ],
                    );
                }
                theta = x87::store(x87::add(x87::load(theta), step_v));
                v += 1;
                if v > n {
                    break;
                }
            }
            phi = x87::store(x87::add(x87::load(phi), step_u));
            u += 1;
            if u > n {
                break;
            }
        }
    }

    /// `Phyre_PPostProcessing_PDeferredLighting_createSphere_00a06ab0(cluster,
    /// n)` (cdecl): the mesh of [`map_mesh`] with 6n² indices and
    /// `(n + 1)²` vertices (index slot `ebp - 0x5c`, vertex slot
    /// `ebp - 0x54`), the globe ([`globe`]), then [`finish`]. Writes its
    /// argument slot `+4` (the instance) as scratch; no caller reads it.
    pub fn create_sphere(h: &mut impl Host, ebp: u32, cluster: u32, n: u32) -> u32 {
        let slots = (ebp - 0x5c, ebp - 0x54);
        let row = n.wrapping_add(1);
        let counts = (n.wrapping_mul(n).wrapping_mul(6), row.wrapping_mul(row));
        let m = map_mesh(h, cluster, counts, slots);
        let vertices: u32 = h.read(slots.1);
        let indices: u32 = h.read(slots.0);
        globe(h, n, vertices, indices);
        finish(h, cluster, &m, slots)
    }
}

/// `0x006dfad0`'s callees: `DynGeoTree_Find` (thiscall `(tree; &key)`),
/// `PCluster::create`, `Phyre_PArray_resizeInline_0048c3b0` (thiscall
/// `(array; n)`), `PEffectVariant::createMaterial` (thiscall, three
/// words), `ShaderParams_FindByName` (thiscall `(shader; name)`),
/// `PhyreMalloc(size, align)`, `PhyreFree(p)`,
/// `PMaterial::setEffectVariant` (thiscall, two words), the static
/// object unlink `0x004dbab0` (`ecx`), `CharacterShaderManager::insert`
/// (thiscall `(manager; &key, &value)`).
pub const VARIANT_CALLS: [u32; 10] = [
    0x006d_edd0,
    0x0044_1010,
    0x0048_c3b0,
    0x004e_6a40,
    0x0056_cbf0,
    0x0042_fb80,
    0x0042_fb20,
    0x004d_8940,
    0x004d_bab0,
    0x006d_f340,
];
/// The globals: the shader type's variant `g_DynGeoFillBuffer + 0x14`
/// and the three it is chosen from (`+0x18`..`+0x20`), the new and the
/// old material arrays (`+0x24`, `+0x28`), the cluster (`+0x30`), the
/// two trees (`0x00c34b1c` materials made, `0x00c34b4c` originals) and
/// the material array type `0x00c998e8`.
pub const VARIANT_GLOBALS: [u32; 4] = [0x00cd_ee04, 0x00cd_ee14, 0x00cd_ee18, 0x00cd_ee20];
/// `0x00c34b1c`, `0x00c34b4c`, `0x00c998e8`.
pub const VARIANT_TREES: [u32; 3] = [0x00c3_4b1c, 0x00c3_4b4c, 0x00c9_98e8];
/// `0x006dfad0`'s frame below `ebp` (the static object at `ebp - 0x38`),
/// in words.
pub const VARIANT_FRAME_WORDS: u32 = 0x38 / 4;

pub mod variants {
    //! `0x006dfad0` over game memory.

    use super::{MEMCPY, VARIANT_CALLS, VARIANT_GLOBALS, VARIANT_TREES};
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// A word array's data (`{count | flag << 31, inline word or data}`):
    /// `[a + 4]` past one element, `a + 4` for one, 0 for none.
    fn data(h: &impl Host, array: u32) -> u32 {
        match h.read::<u32>(array) & 0x7fff_ffff {
            0 => 0,
            1 => array + 4,
            _ => h.read(array + 4),
        }
    }

    /// A tagged string pointer (bit 0 a flag).
    fn untag(h: &impl Host, at: u32) -> u32 {
        let p: u32 = h.read(at);
        p.wrapping_sub(u32::from(h.read::<u8>(at) & 1))
    }

    /// The bytes of the parameter `p` of the source buffer `sp` copied
    /// into the material's buffer `pb` at the place of its parameter of
    /// the same name, for each of the source's parameters (16-byte
    /// records at `[sp + 0xc]`, `[sp + 8] & 0x7fffffff` of them), when
    /// both fit.
    fn copy_parameters(h: &mut impl Host, sp: u32, pb: u32) {
        let find = VARIANT_CALLS[4];
        let n = h.read::<u32>(sp + 8) & 0x7fff_ffff;
        for j in 0..n {
            let table: u32 = h.read(sp + 0xc);
            let e = table.wrapping_add(0x10 * j);
            let name = untag(h, e + 4);
            let shader: u32 = h.read(pb + 4);
            let p = this_call(h, find, shader, &[name]);
            if p == 0 {
                continue;
            }
            let off = u32::from(h.read::<u16>(e + 8));
            let size = u32::from(h.read::<u16>(e + 0xa) & 0x1fff);
            if off.wrapping_add(size) > h.read::<u32>(sp) || off < 4 {
                continue;
            }
            let src = off.wrapping_add(sp);
            if src == 0 {
                continue;
            }
            let off2 = u32::from(h.read::<u16>(p + 8));
            let size2 = u32::from(h.read::<u16>(p + 0xa) & 0x1fff);
            if off2.wrapping_add(size2) > h.read::<u32>(pb) || off2 < 4 {
                continue;
            }
            h.call_sym(MEMCPY, Conv::Cdecl, &[off2.wrapping_add(pb), src, size2]);
        }
    }

    /// The source's name (`[src + 0x10]`, tagged) copied into a new
    /// block (`PhyreMalloc(len + 1, 2)`) for the material (`+0x10`, the
    /// old one freed when owned); nothing when the allocation fails; 0
    /// for no name.
    fn copy_name(h: &mut impl Host, src: u32, mat: u32) {
        let [.., malloc, free, _, _, _] = VARIANT_CALLS;
        let name = untag(h, src + 0x10);
        let copy = if name == 0 {
            0
        } else {
            let mut len = 0u32;
            while h.read::<u8>(name.wrapping_add(len)) != 0 {
                len = len.wrapping_add(1);
            }
            let bytes = len.wrapping_add(1);
            let block = h.call_sym(malloc, Conv::Cdecl, &[bytes, 2]);
            if block == 0 {
                return;
            }
            h.call_sym(MEMCPY, Conv::Cdecl, &[block, name, bytes]);
            block
        };
        if h.read::<u8>(mat + 0x10) & 1 == 0 {
            let old: u32 = h.read(mat + 0x10);
            h.call_sym(free, Conv::Cdecl, &[old]);
        }
        h.write(mat + 0x10, copy);
    }

    /// `Phyre_CharacterShaderManager_makeVariants_006dfad0(key, kind)`
    /// (cdecl; `slots` the caller's argument words): the shader type's
    /// variant `+0x14` set from `+0x18`/`+0x1c`/`+0x20` for kind 0/1/2
    /// (else kept); nothing more (its value in `eax`) without one, or
    /// when the materials tree (`0x00c34b1c`) has the key
    /// (`DynGeoTree_Find(&key)`, `eax` its node + 0x10). Else a new
    /// material array (`PCluster::create(cluster; 1, 0x00c998e8, 0)`,
    /// global `+0x24`) as long as the key's (`[key + 0x10]`, global
    /// `+0x28`); for each of its materials: a 4-word static object at
    /// `ebp - 0x38` copied from `[[[material] + 0x10]]`, a material of
    /// the variant (`createMaterial(variant; cluster, 0, 0)`), the
    /// source's parameters ([`copy_parameters`]; the source buffer kept
    /// in the argument slot `+4`), its name ([`copy_name`]),
    /// `setEffectVariant(variant, buffer)`, the material stored in the
    /// new array (inside its count), the static object unlinked
    /// (`0x004dbab0`); then the key's array is the new one (or
    /// `[key] + 0x30` without it) and both arrays go into the trees
    /// (`insert(&key, &global)`), the last insert's `eax`.
    pub fn make_variants(h: &mut impl Host, ebp: u32, slots: u32, key: u32, kind: u32) -> u32 {
        let [
            find,
            create,
            resize,
            create_material,
            _,
            _,
            _,
            set_variant,
            unlink,
            insert,
        ] = VARIANT_CALLS;
        let [g_variant, g_new, g_old, g_cluster] = VARIANT_GLOBALS.map(|a| h.rebase(a));
        let [made_tree, old_tree, array_type] = VARIANT_TREES.map(|a| h.rebase(a));
        if kind < 3 {
            let v: u32 = h.read(g_variant + 4 + 4 * kind);
            h.write(g_variant, v);
        }
        let variant: u32 = h.read(g_variant);
        if variant == 0 {
            return 0;
        }
        let node = this_call(h, find, made_tree, &[slots]);
        if node != 0 && node.wrapping_add(0x10) != 0 {
            return node.wrapping_add(0x10);
        }
        let new = this_call(h, create, g_cluster, &[1, array_type, 0]);
        h.write(g_new, new);
        let old: u32 = h.read(key + 0x10);
        h.write(g_old, old);
        let n = h.read::<u32>(old) & 0x7fff_ffff;
        this_call(h, resize, new, &[n]);
        let local = ebp - 0x38;
        for i in 0..n {
            let old: u32 = h.read(g_old);
            let src: u32 = h.read(data(h, old).wrapping_add(4 * i));
            let head: u32 = h.read(src);
            let p: u32 = h.read(head + 0x10);
            let from: u32 = h.read(p);
            let words: [u32; 4] = h.read(from);
            h.write(local, words);
            let variant: u32 = h.read(g_variant);
            let mat = this_call(h, create_material, variant, &[g_cluster, 0, 0]);
            let pb: u32 = h.read(mat + 4);
            let sp: u32 = h.read(src + 4);
            h.write(slots + 4, sp);
            copy_parameters(h, sp, pb);
            copy_name(h, src, mat);
            let variant: u32 = h.read(g_variant);
            this_call(h, set_variant, mat, &[variant, pb]);
            let new: u32 = h.read(g_new);
            if i < h.read::<u32>(new) & 0x7fff_ffff {
                let d = data(h, new);
                h.write(d.wrapping_add(4 * i), mat);
            }
            this_call(h, unlink, local, &[]);
        }
        let new: u32 = h.read(g_new);
        let array = if new == 0 {
            h.read::<u32>(key).wrapping_add(0x30)
        } else {
            new
        };
        h.write(key + 0x10, array);
        this_call(h, insert, old_tree, &[slots, g_old]);
        this_call(h, insert, made_tree, &[slots, g_new])
    }
}

/// `0x005b9ee0`'s callees: `PPreprocessRingBuffer::GetSingleton` (cdecl),
/// `PPreprocessRingBuffer::allocate` (thiscall, one word),
/// `PMeshInstance::computeJointMatrices` (thiscall, one word),
/// `uploadSkinMatrices` (thiscall, one word), `PMatrix4_Inverse` (cdecl
/// `(out, m)` -> out), `PMatrix4_Copy_004c9430` (thiscall `(this; m)`),
/// `PDynamicMesh::getDataBlockCount` (thiscall, one word),
/// `countKinds` (cdecl, one word), `allocateAndFillParameterBuffers`
/// (thiscall, eight words), `PRenderRingBufferSegmented::allocate`
/// (thiscall, one word), `queueMeshCommands` (thiscall, eight words).
pub const BATCH_CALLS: [u32; 11] = [
    0x005c_d010,
    0x005c_d0f0,
    0x004f_6ef0,
    0x004f_98b0,
    0x004c_c0c0,
    0x004c_9430,
    0x0049_8770,
    0x005b_52b0,
    0x005b_5a50,
    0x0061_3e10,
    0x005b_a380,
];
/// The pass names `"Opaque"` and `"Transparent"` (string table entries
/// the renderer and the pass descriptors point at).
pub const BATCH_PASS_NAMES: [u32; 2] = [0x00c1_69b4, 0x00c1_69d4];
/// `0x005b9ee0`'s frame below `ebp` (down to the inverse at
/// `ebp - 0x164`), in words.
pub const BATCH_FRAME_WORDS: u32 = 0x164 / 4;

pub mod batch {
    //! `0x005b9ee0` over game memory, split at its `_alloca_probe_16`
    //! (docs/porting.md, "Run-time frames and varargs").

    use super::{BATCH_CALLS, BATCH_PASS_NAMES};
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// The joint matrices' block size (`([mesh + 0x18] & 0x7fffffff) + 1`
    /// matrices) when the original carves it with `_alloca_probe_16`:
    /// `None` when the renderer is not ready (`+4` bit 0 clear) or past
    /// 0x80 matrices (the preprocess ring buffer gives the block then).
    pub fn joint_bytes(h: &impl Host, renderer: u32, batch: u32) -> Option<u32> {
        if h.read::<u32>(renderer + 4) & 1 == 0 {
            return None;
        }
        let instance: u32 = h.read(batch);
        let mesh: u32 = h.read(instance);
        let n = (h.read::<u32>(mesh + 0x18) & 0x7fff_ffff).wrapping_add(1);
        if n > 0x80 { None } else { Some(n << 6) }
    }

    /// The draw of segment `i` of the item at `[*cursor]` in the pass
    /// (`ebp - 0xe0` the draw context, its segment, index, joint
    /// matrix, the joint's inverse and the item's two words): when the
    /// renderer's pass is "Opaque" and the item's pass descriptor
    /// (`+0x1c`) is "Transparent" without being so at `+0xc`, nothing;
    /// else the parameter buffers sized and filled
    /// (`allocateAndFillParameterBuffers` into `ebp - 0x70`, its count
    /// word at `ebp - 0x18`) and on success the mesh commands queued into
    /// a ring block. `Some(error)` when filling fails.
    #[allow(clippy::too_many_arguments)] // the original's locals
    fn draw(
        h: &mut impl Host,
        ebp: u32,
        r: u32,
        joints: u32,
        dynamic: u32,
        seg: u32,
        i: u32,
        item: u32,
    ) -> Option<u32> {
        let [
            _,
            _,
            _,
            _,
            inverse,
            copy,
            block_count,
            count_kinds,
            fill,
            ring_alloc,
            queue,
        ] = BATCH_CALLS;
        let [opaque, transparent] = BATCH_PASS_NAMES.map(|a| h.rebase(a));
        let first: u32 = h.read(item);
        let material: u32 = h.read(item + 4);
        h.write(ebp - 0xc4, [seg, i]);
        h.write(ebp - 0x78, [first, material]);
        let joint = joints.wrapping_add(h.read::<u32>(seg + 4) << 6);
        h.write(ebp - 0xbc, joint);
        let inv = h.call_sym(inverse, Conv::Cdecl, &[ebp - 0x164, joint]);
        this_call(h, copy, ebp - 0x120, &[inv]);
        for k in 0..16 {
            let v = x87::store(x87::load(h.read(ebp - 0x120 + 4 * k)));
            h.write(ebp - 0xb8 + 4 * k, v);
        }
        let desc: u32 = h.read(item + 0x1c);
        if h.read::<u32>(r + 0x48) == opaque
            && h.read::<u32>(desc + 8) == transparent
            && h.read::<u32>(desc + 0xc) != transparent
        {
            return None;
        }
        let head: u32 = h.read(material);
        let pass: u32 = h.read(desc + 4);
        let mut extra = 0u32;
        if dynamic != 0 {
            let m: u32 = h.read(dynamic + 4);
            let blocks = this_call(h, block_count, m, &[i]);
            let table: u32 = h.read(m + 0x14);
            let more = if h.read::<u32>(table.wrapping_add(0x10 * i + 0xc)) != 0 {
                0x40
            } else {
                0
            };
            extra = ((blocks << 6).wrapping_add(0xf) & !0xf).wrapping_add(more);
        }
        let k1 = h.call_sym(count_kinds, Conv::Cdecl, &[pass.wrapping_add(8)]);
        let k2 = h.call_sym(count_kinds, Conv::Cdecl, &[material.wrapping_add(0xc)]);
        let mut n = (h.read::<u32>(material + 0x14) & 0x7fff_ffff)
            .wrapping_add(2)
            .wrapping_add(k1)
            .wrapping_add(k2);
        if dynamic != 0 {
            n = n.wrapping_add(2);
        }
        let size = if n > 1 {
            n.wrapping_mul(4).wrapping_add(4)
        } else {
            8
        };
        h.write(ebp - 0x70, [0u32; 7]);
        h.write(ebp - 0x18, 0u32);
        let err = this_call(
            h,
            fill,
            r,
            &[
                ebp - 0x70,
                head,
                pass,
                ebp - 0xe0,
                ebp - 0x18,
                item.wrapping_add(8),
                extra,
                dynamic,
            ],
        );
        if err != 0 {
            return Some(err);
        }
        let block = this_call(h, ring_alloc, r + 0x1c, &[size.wrapping_add(0xf) & !0xf]);
        let words: u32 = h.read(ebp - 0x18);
        this_call(
            h,
            queue,
            r,
            &[first, material, dynamic, seg, i, ebp - 0x70, block, words],
        );
        None
    }

    /// `Phyre_PRendering_PRendererBase_renderBatch_005b9ee0(renderer;
    /// batch, cursor, frame)` (thiscall, ret 0xc): 0x16 unless the
    /// renderer is ready (`+4` bit 0); the joint matrices of the batch's
    /// instance (`[batch]`, its mesh's `[+0x18] + 1` matrices, the first
    /// skipped) computed into `alloca` (from [`joint_bytes`]) or, past
    /// 0x80, a preprocess ring block (`+0x10`; its word `+4` cleared at
    /// the end); skin matrices uploaded when the batch says so (byte
    /// `+8`) and the instance's frame (`+0x80`) changes; the draw
    /// context at `ebp - 0xe0`; then for each pass 0..=3 the items from
    /// `*cursor` again, one per segment `[batch + 4]..[batch + 6]`
    /// (16-bit), each drawn ([`draw`]) when it has a material, the
    /// segment (0x6c bytes of the mesh's `[+4]`) is visible (`+0x14`
    /// clear) and in the pass (`+0x1c`), `*cursor` advancing by 0x20.
    /// The last fill error, or 0.
    pub fn render_batch(
        h: &mut impl Host,
        ebp: u32,
        r: u32,
        (batch, cursor, frame): (u32, u32, u32),
        alloca: u32,
    ) -> u32 {
        let [singleton, allocate, compute, upload, ..] = BATCH_CALLS;
        if h.read::<u32>(r + 4) & 1 == 0 {
            return 0x16;
        }
        let instance: u32 = h.read(batch);
        let dynamic: u32 = h.read(instance + 0x18);
        let skin: u32 = h.read(instance + 0x14);
        let mesh: u32 = h.read(instance);
        let n = (h.read::<u32>(mesh + 0x18) & 0x7fff_ffff).wrapping_add(1);
        let mut block = 0;
        let base = if n > 0x80 {
            let ring = h.call_sym(singleton, Conv::Cdecl, &[]);
            block = this_call(h, allocate, ring, &[n << 6]);
            block.wrapping_add(0x10)
        } else {
            alloca
        };
        h.write(ebp - 0x124, block);
        let joints = base.wrapping_add(0x40);
        this_call(h, compute, instance, &[joints]);
        if h.read::<u8>(batch + 8) != 0 {
            let inst: u32 = h.read(batch);
            if h.read::<u32>(inst + 0x80) != frame {
                h.write(inst + 0x80, frame);
                this_call(h, upload, inst, &[joints]);
            }
        }
        let view: u32 = h.read(r + 0x18);
        let context = [
            h.read::<u32>(r + 0x5c),
            r.wrapping_add(0x60),
            r.wrapping_add(0xa0),
            h.read::<u32>(view + 0xc4),
            instance,
            joints,
            skin,
            0,
            0,
            0,
        ];
        h.write(ebp - 0xe0, context);
        h.write(ebp - 0x78, [0u32; 2]);
        let start: u32 = h.read(cursor);
        let mut result = 0;
        for pass in 0..4u32 {
            h.write(cursor, start);
            let mut i = u32::from(h.read::<u16>(batch + 4));
            while i < u32::from(h.read::<u16>(batch + 6)) {
                let item: u32 = h.read(cursor);
                let material: u32 = h.read(item + 4);
                let segs: u32 = h.read(mesh + 4);
                let seg = segs.wrapping_add(0x6c * i);
                if material != 0
                    && h.read::<u8>(seg + 0x14) == 0
                    && h.read::<u32>(seg + 0x1c) == pass
                    && let Some(err) = draw(h, ebp, r, joints, dynamic, seg, i, item)
                {
                    result = err;
                }
                let at: u32 = h.read(cursor);
                h.write(cursor, at.wrapping_add(0x20));
                i += 1;
            }
        }
        if block != 0 {
            h.write(block + 4, 0u32);
        }
        result
    }
}
