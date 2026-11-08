//! List `phyre-render-b`, round 10 (`tail10`): the character lighting
//! tweak tool's light gizmo, `CharacterLightingTweakUtil_drawGizmo_006a7850`
//! (thiscall `(util; character)`, `ret 4`).
//!
//! With the tool on (`util + 0xc8`) for the light `util + 0xc0`: the
//! gizmo's anchor (`util + 0xe8..0xf4`) follows the character (its matrix
//! `character + 0xb0`, three units of 87.5 along its `-z` from its
//! position) when it has drifted more than 150 away; under the render
//! scope lock (its interface made once behind the guard `0x00c9628c`), a
//! lit light (colour length² > 0.01) gets a 3 x 3 x 3 grid of arrows
//! (`PRendererD3D11_drawPoints_00591dd0` line strips of three points:
//! from each grid point along the light's direction times 50, back by an
//! arrowhead 0.015 of its length across it, turned by the character's
//! matrix), and a point light (`PLightType`, length² > 0.01) a line to the
//! anchor and its three axes (red, green, blue) at its range
//! (`light + 0x2c`). The vectors the callees see stay in a frame at the
//! original's `ebp` offsets ([`frame`]); x87 arithmetic in the original's
//! order, `_CIsqrt` the CRT's.

use ffx_graphic::host::{Conv, Host};

/// `g_CharacterLighting` (the lighting object pointer).
pub const LIGHTING: u32 = 0x00cc_ca84;
/// `Phyre_CharacterLighting_lightDirection_006a84d0(lighting; out, i)`.
pub const LIGHT_DIRECTION: u32 = 0x006a_84d0;
/// `Phyre_CharacterLighting_lightColour_006a8440(lighting; out, i)`.
pub const LIGHT_COLOUR: u32 = 0x006a_8440;
/// `Phyre_CharacterLighting_lightPosition_006a8b40(lighting; out, i)`.
pub const LIGHT_POSITION: u32 = 0x006a_8b40;
/// `PMatrix4_FromMatrix3x4_00451080(out; m)`.
pub const FROM_3X4: u32 = 0x0045_1080;
/// `PMatrix4_transformColumn_005b4c00(m; out, v)`.
pub const TRANSFORM: u32 = 0x005b_4c00;
/// `PRendererD3D11_drawPoints_00591dd0(renderer; primitive, points,
/// colour, indices, n, m)`.
pub const DRAW_POINTS: u32 = 0x0059_1dd0;
/// `PRenderScopeLock_enter_00463d50(lock)`.
pub const LOCK_ENTER: u32 = 0x0046_3d50;
/// `PRenderScopeLock_leave_00463e90(lock)`.
pub const LOCK_LEAVE: u32 = 0x0046_3e90;
/// The render interface's guard (bit 0), constructor (thiscall on
/// [`INTERFACE`]), `atexit` and its handler.
pub const GUARD: u32 = 0x00c9_628c;
pub const INTERFACE: u32 = 0x00c9_4f00;
pub const INTERFACE_CTOR: u32 = 0x004f_ee70;
pub const ATEXIT: u32 = 0x0094_9207;
pub const INTERFACE_DTOR: u32 = 0x00af_b710;
/// `_CIsqrt`.
pub const SQRT: u32 = 0x0094_97b8;
/// `PLightType`'s vtable (a point light's `+0x18`).
pub const POINT_LIGHT_VTABLE: u32 = 0x00c1_0814;

/// The constants (`.rdata`).
mod k {
    pub const MINUS_ONE_D: u32 = 0x00b9_2350;
    pub const MINUS_ONE: u32 = 0x00b9_2340;
    pub const THREE: u32 = 0x00b2_c5c8;
    pub const SPACING: u32 = 0x00b4_8370;
    pub const REACH: u32 = 0x00b4_8388;
    pub const LIT: u32 = 0x00b1_f2b0;
    pub const LENGTH: u32 = 0x00b1_f2c0;
    pub const HALF_GRID: u32 = 0x00b4_1d00;
    pub const HEAD: u32 = 0x00b4_82e0;
}

/// The frame from `ebp - 0x1b4` (the bridge's `f`) up to the cookie at
/// `ebp - 0x10`: what the callees see.
pub mod frame {
    /// `ebp - 0x1b4`: the character's 4x4 matrix.
    pub const MATRIX: u32 = 0;
    /// `ebp - 0x174`: the light direction's output.
    pub const DIRECTION: u32 = 0x40;
    /// `ebp - 0x164`: the light position's output.
    pub const POSITION: u32 = 0x50;
    /// `ebp - 0x154`: the transform's output (the arrowhead).
    pub const TURNED: u32 = 0x60;
    /// `ebp - 0x144`: the transform's input (the arrowhead).
    pub const HEAD: u32 = 0x70;
    /// `ebp - 0x134`, `- 0x124`, `- 0x104`: red, green, blue.
    pub const RED: u32 = 0x80;
    pub const GREEN: u32 = 0x90;
    pub const BLUE: u32 = 0xb0;
    /// `ebp - 0x114`: the matrix's `-z` input.
    pub const DOWN: u32 = 0xa0;
    /// `ebp - 0xf4`: the light's colour.
    pub const COLOUR: u32 = 0xc0;
    /// `ebp - 0xe4`: the character's `-z`.
    pub const AXIS: u32 = 0xd0;
    /// `ebp - 0xc0`: the render scope lock.
    pub const LOCK: u32 = 0xf4;
    /// `ebp - 0xbc`: the point light's colour (its first four words).
    pub const POINT: u32 = 0xf8;
    /// `ebp - 0xa8`: a line's two points.
    pub const LINE: u32 = 0x10c;
    /// `ebp - 0x40`: an arrow's three points.
    pub const ARROW: u32 = 0x174;
    /// The bytes.
    pub const BYTES: u32 = 0x1a4;
}

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // h, f, x87 terms
pub mod bridge {
    use super::{
        ATEXIT, Conv, DRAW_POINTS, FROM_3X4, GUARD, Host, INTERFACE, INTERFACE_CTOR,
        INTERFACE_DTOR, LIGHT_COLOUR, LIGHT_DIRECTION, LIGHT_POSITION, LIGHTING, LOCK_ENTER,
        LOCK_LEAVE, POINT_LIGHT_VTABLE, SQRT, TRANSFORM, frame, k,
    };
    use ffx_bridge::x87;

    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    fn put(h: &mut impl Host, at: u32, x: f64) {
        h.write(at, x87::store(x));
    }

    /// `fstp m32` then `fld`.
    fn r32(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    fn kd(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    fn kf(h: &impl Host, sym: u32) -> f64 {
        x87::load(h.read(h.rebase(sym)))
    }

    fn this_call(h: &mut impl Host, sym: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Thiscall(this), args)
    }

    fn sqrt(h: &mut impl Host, x: f64) -> f64 {
        let f = h.rebase(SQRT);
        f64::from(h.call_intrinsic(f, x))
    }

    /// The four-term length² the original sums: `((a² + b²) + c²) + d²`,
    /// each partial sum stored as `f32`.
    fn len2(v: [f64; 4]) -> f64 {
        let s = r32(x87::mul(v[0], v[0]));
        let s = r32(x87::add(s, x87::mul(v[1], v[1])));
        let s = r32(x87::add(s, x87::mul(v[2], v[2])));
        r32(x87::add(x87::mul(v[3], v[3]), s))
    }

    /// A vector of four `f32` at `at`.
    fn vec4(h: &impl Host, at: u32) -> [f64; 4] {
        std::array::from_fn(|i| ld(h, at.wrapping_add(4 * i as u32)))
    }

    fn write4(h: &mut impl Host, at: u32, v: [u32; 4]) {
        for (i, w) in (0u32..).zip(v) {
            h.write(at.wrapping_add(4 * i), w);
        }
    }

    /// The light `i`'s record when it is a point light of the set-up.
    fn point_light(h: &impl Host, i: u32) -> Option<u32> {
        let lighting: u32 = h.read(h.rebase(LIGHTING));
        if h.read::<u8>(lighting.wrapping_add(0x1cc)) == 0 {
            return None;
        }
        let set: u32 = h.read(lighting.wrapping_add(0xf0));
        let lights: u32 = h.read(set.wrapping_add(4));
        let light: u32 = h.read(lights.wrapping_add(4 * i));
        let vt: u32 = h.read(light.wrapping_add(0x18));
        (vt == h.rebase(POINT_LIGHT_VTABLE)).then_some(light)
    }

    /// `drawPoints(renderer; primitive, points, colour, 0, n, m)`.
    fn draw(h: &mut impl Host, primitive: u32, points: u32, colour: u32, n: u32, m: u32) {
        let renderer = h.rebase(INTERFACE);
        this_call(
            h,
            DRAW_POINTS,
            renderer,
            &[primitive, points, colour, 0, n, m],
        );
    }

    /// `CharacterLightingTweakUtil_drawGizmo_006a7850(util; character)`
    /// with its frame at `f` ([`frame`]).
    #[allow(clippy::too_many_lines)] // one original
    pub fn draw_gizmo(h: &mut impl Host, util: u32, character: u32, f: u32) {
        if h.read::<u8>(util.wrapping_add(0xc8)) == 0 {
            return;
        }
        let at = |o: u32| f.wrapping_add(o);
        let i: u32 = h.read(util.wrapping_add(0xc0));
        let lighting: u32 = h.read(h.rebase(LIGHTING));
        let dir = this_call(h, LIGHT_DIRECTION, lighting, &[at(frame::DIRECTION), i]);
        let d = [
            ld(h, dir),
            ld(h, dir.wrapping_add(4)),
            ld(h, dir.wrapping_add(8)),
        ];
        let i: u32 = h.read(util.wrapping_add(0xc0));
        let lighting: u32 = h.read(h.rebase(LIGHTING));
        this_call(h, LIGHT_COLOUR, lighting, &[at(frame::COLOUR), i]);
        let m = character.wrapping_add(0x70);
        let src: u32 = h.read(character.wrapping_add(0xb0));
        this_call(h, FROM_3X4, at(frame::MATRIX), &[src]);
        let pos = [
            ld(h, src.wrapping_add(0x20)),
            ld(h, src.wrapping_add(0x24)),
            ld(h, src.wrapping_add(0x28)),
        ];
        // the matrix's -z
        let z = x87::store(x87::mul(0.0, kd(h, k::MINUS_ONE_D)));
        let down = [z, z, h.read(h.rebase(k::MINUS_ONE)), z];
        write4(h, at(frame::DOWN), down);
        this_call(
            h,
            TRANSFORM,
            at(frame::MATRIX),
            &[at(frame::AXIS), at(frame::DOWN)],
        );
        let e = vec4(h, at(frame::AXIS));
        let three = kd(h, k::THREE);
        let spacing = kd(h, k::SPACING);
        let target: [f64; 3] = std::array::from_fn(|c| {
            let v = r32(x87::mul(e[c], three));
            let v = r32(x87::mul(v, spacing));
            r32(x87::add(v, pos[c]))
        });
        // the anchor follows when it drifted too far
        let anchor = util.wrapping_add(0xe8);
        let b = vec4(h, anchor);
        let off = [
            r32(x87::sub(b[0], target[0])),
            r32(x87::sub(b[1], target[1])),
            r32(x87::sub(b[2], target[2])),
            r32(x87::sub(b[3], 1.0)),
        ];
        let dist = r32(sqrt(h, len2(off)));
        if kf(h, k::REACH) < dist {
            for (c, v) in target.into_iter().enumerate() {
                put(h, anchor.wrapping_add(4 * c as u32), v);
            }
            h.write(anchor.wrapping_add(12), 1.0f32.to_bits());
        }
        let lock = at(frame::LOCK);
        this_call(h, LOCK_ENTER, lock, &[]);
        let guard = h.rebase(GUARD);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            let interface = h.rebase(INTERFACE);
            this_call(h, INTERFACE_CTOR, interface, &[]);
            let dtor = h.rebase(INTERFACE_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        let colour = vec4(h, at(frame::COLOUR));
        let lit = kf(h, k::LIT);
        if lit < len2([colour[0], colour[1], colour[2], colour[3]]) {
            arrows(h, util, f, d, [e[0], e[1], e[2]], m);
        }

        // a point light: a line to the anchor and its axes at its range
        let i: u32 = h.read(util.wrapping_add(0xc0));
        let pc = at(frame::POINT);
        match point_light(h, i) {
            Some(light) => {
                for c in 0..4u32 {
                    let w: u32 = h.read(light.wrapping_add(4 * c));
                    h.write(pc.wrapping_add(4 * c), w);
                }
            }
            None => write4(h, pc, [0; 4]),
        }
        if lit < len2(vec4(h, pc)) {
            let lighting: u32 = h.read(h.rebase(LIGHTING));
            let p = this_call(h, LIGHT_POSITION, lighting, &[at(frame::POSITION), i]);
            let line = at(frame::LINE);
            for c in 0..4u32 {
                let w: u32 = h.read(p.wrapping_add(4 * c));
                h.write(line.wrapping_add(4 * c), w);
            }
            for c in 0..4u32 {
                let w: u32 = h.read(anchor.wrapping_add(4 * c));
                h.write(line.wrapping_add(16 + 4 * c), w);
            }
            draw(h, 1, line, pc, 2, m);
            let i: u32 = h.read(util.wrapping_add(0xc0));
            let range = match point_light(h, i) {
                Some(light) => ld(h, light.wrapping_add(0x2c)),
                None => 0.0,
            };
            let zero = r32(x87::mul(0.0, range));
            let p = vec4(h, line);
            let one = 1.0f32.to_bits();
            for (colour, axis) in [(frame::RED, 0usize), (frame::GREEN, 1), (frame::BLUE, 2)] {
                let mut rgb = [0u32, 0, 0, one];
                rgb[axis] = one;
                write4(h, at(colour), rgb);
                let q: [u32; 4] = std::array::from_fn(|c| {
                    let step = if c == axis { range } else { zero };
                    x87::store(x87::add(step, p[c]))
                });
                write4(h, line.wrapping_add(16), q);
                draw(h, 1, line, at(colour), 2, m);
            }
        }
        this_call(h, LOCK_LEAVE, lock, &[]);
    }

    /// The 3 x 3 x 3 grid of arrows along the light's direction `d`,
    /// their heads across it and the character's `-z` `e`.
    fn arrows(h: &mut impl Host, util: u32, f: u32, d: [f64; 3], e: [f64; 3], m: u32) {
        let at = |o: u32| f.wrapping_add(o);
        let length = kd(h, k::LENGTH);
        let s: [f64; 3] = std::array::from_fn(|c| r32(x87::mul(d[c], length)));
        let half = kd(h, k::HALF_GRID);
        let spacing = kd(h, k::SPACING);
        let head = kd(h, k::HEAD);
        let one = 1.0f32.to_bits();
        h.write(at(frame::HEAD + 12), one);
        let offset = |n: i32| r32(x87::mul(x87::sub(f64::from(n), half), spacing));
        let arrow = at(frame::ARROW);
        for gi in 0..3 {
            let ox = offset(gi);
            for gj in 0..3 {
                let oy = offset(gj);
                for gk in 0..3 {
                    let oz = offset(gk);
                    let b = vec4(h, util.wrapping_add(0xe8));
                    let p = [
                        r32(x87::add(ox, b[0])),
                        r32(x87::add(oy, b[1])),
                        r32(x87::add(b[2], oz)),
                    ];
                    let q = [
                        r32(x87::add(p[0], s[0])),
                        r32(x87::add(p[1], s[1])),
                        r32(x87::add(s[2], p[2])),
                    ];
                    let dv: [f64; 3] = std::array::from_fn(|c| r32(x87::sub(q[c], p[c])));
                    let cross = [
                        r32(x87::sub(x87::mul(dv[1], e[2]), x87::mul(dv[2], e[1]))),
                        r32(x87::sub(x87::mul(e[0], dv[2]), x87::mul(dv[0], e[2]))),
                        r32(x87::sub(x87::mul(dv[0], e[1]), x87::mul(e[0], dv[1]))),
                    ];
                    // the arrow's tail and tip
                    write4(
                        h,
                        arrow,
                        [x87::store(p[0]), x87::store(p[1]), x87::store(p[2]), one],
                    );
                    write4(
                        h,
                        arrow.wrapping_add(16),
                        [x87::store(q[0]), x87::store(q[1]), x87::store(q[2]), one],
                    );
                    let l = r32(x87::add(len2_3(dv), 0.0));
                    let len = r32(sqrt(h, l));
                    let nl = r32(sqrt(h, len2_3_rev(cross)));
                    let inv = r32(x87::div(1.0, nl));
                    let tip: [f64; 3] = std::array::from_fn(|c| {
                        let n = r32(x87::mul(inv, cross[c]));
                        let n = r32(x87::mul(len, n));
                        r32(x87::mul(n, head))
                    });
                    let input = at(frame::HEAD);
                    for (c, v) in (0u32..).zip(tip) {
                        put(h, input.wrapping_add(4 * c), v);
                    }
                    let out =
                        this_call(h, TRANSFORM, at(frame::MATRIX), &[at(frame::TURNED), input]);
                    let o = vec4(h, out);
                    let c3 = [
                        x87::store(x87::add(p[0], o[0])),
                        x87::store(x87::add(o[1], p[1])),
                        x87::store(x87::add(o[2], p[2])),
                        x87::store(x87::add(o[3], 1.0)),
                    ];
                    write4(h, arrow.wrapping_add(32), c3);
                    draw(h, 2, arrow, at(frame::COLOUR), 3, m);
                }
            }
        }
    }

    /// `((x² + y²) + z²)`, each partial sum stored as `f32`.
    fn len2_3(v: [f64; 3]) -> f64 {
        let s = r32(x87::mul(v[0], v[0]));
        let s = r32(x87::add(s, x87::mul(v[1], v[1])));
        r32(x87::add(s, x87::mul(v[2], v[2])))
    }

    /// `(z² + (x² + y²))` (the normal's order), each partial sum `f32`.
    fn len2_3_rev(v: [f64; 3]) -> f64 {
        let s = r32(x87::mul(v[0], v[0]));
        let s = r32(x87::add(s, x87::mul(v[1], v[1])));
        r32(x87::add(x87::mul(v[2], v[2]), s))
    }
}
