//! M5 stage 1, list `scene`, round big-phyre2 (`ffx_prender::m5p_scene_bp2`):
//! the two occluder classes' `Bind()` functions. Both have the shape
//! [`crate::h1_bind`] runs (register, members once each under a guard
//! word, an `atexit` after each, stores, `addToNamespace`), including the
//! class (method) member step [`crate::h1_bind::Step::Class`] that
//! `POccluderGeometryInstance` needs, so they are tables for its
//! interpreter. Read off the machine code (`0x00a30c90`, `0x00a40270`);
//! parity tests: `crates/ffx-oracle/tests/m5p_scene.rs`
//! (`m5p_scene_bp2_*`).

use crate::h1_bind::{Bind, Step, Ty, Val};

/// `PClassDataMember::PClassDataMember` (thiscall, six words).
const DATA_MEMBER: u32 = 0x0057_52a0;

/// `Phyre_POccluderGeometry_POccluderGeometryInstance_Bind` (`0x00a30c90`).
pub const INSTANCE_BIND_ADDR: u32 = 0x00a3_0c90;
/// The instance class's guard word.
pub const INSTANCE_GUARD: u32 = 0x01a8_54e0;

/// `POccluderGeometryInstance::Bind()`: `m_occluder` (the object class's
/// descriptor `0x01a85990`), `m_localToWorldMatrix` (`0x00c92090`), the
/// method `getLocalToWorldMatrix` (linked into the descriptor's method
/// list), then on every call its caller object: vtable
/// `PMethodCallerConcrete<POccluderGeometryInstance, PWorldMatrix>`
/// (`0x00b79f20`), method `0x00a32220`, and the member's caller word
/// (`+0x14`) pointing at it.
pub const INSTANCE_BIND: Bind = Bind {
    d: 0x01a8_5420,
    steps: &[
        Step::Guard(
            INSTANCE_GUARD,
            1,
            &[
                Step::Member {
                    ctor: DATA_MEMBER,
                    member: 0x01a8_54b4,
                    ty: Ty::Addr(0x01a8_5990),
                    name: 0x00b7_9f10,
                    words: [0, 2, 0],
                },
                Step::AtExit(0x00b0_a630),
            ],
        ),
        Step::Guard(
            INSTANCE_GUARD,
            2,
            &[
                Step::Member {
                    ctor: DATA_MEMBER,
                    member: 0x01a8_54e4,
                    ty: Ty::Addr(0x00c9_2090),
                    name: 0x00b1_0d80,
                    words: [4, 2, 0],
                },
                Step::AtExit(0x00b0_a620),
            ],
        ),
        Step::Guard(
            INSTANCE_GUARD,
            4,
            &[
                Step::Class {
                    member: 0x01a8_5510,
                    name: 0x00b1_071c,
                    flags: 0,
                },
                Step::AtExit(0x00b0_a5e0),
            ],
        ),
        Step::Store(0x01a8_5528, Val::Addr(0x00b7_9f20)),
        Step::Store(0x01a8_552c, Val::Addr(0x00a3_2220)),
        Step::Store(0x01a8_5524, Val::Addr(0x01a8_5528)),
    ],
};

/// `Phyre_POccluderGeometry_POccluderGeometryObject_Bind` (`0x00a40270`).
pub const OBJECT_BIND_ADDR: u32 = 0x00a4_0270;
/// The object class's guard word.
pub const OBJECT_GUARD: u32 = 0x01a8_5a58;

/// `PTypeDispenser::GetType<unsigned int>` (cdecl `()`).
const GET_U32: u32 = 0x0043_7fc0;
/// `PTypeDispenser::GetClassDescriptor<Vectormath::Aos::Vector3>` (cdecl
/// `()`).
const GET_VECTOR3: u32 = 0x0045_fd90;

/// A data member of the object class: guard bit, member global, type
/// getter, name, offset and `atexit` destructor.
macro_rules! object_member {
    ($bit:expr, $member:expr, $get:expr, $name:expr, $offset:expr, $dtor:expr) => {
        Step::Guard(
            OBJECT_GUARD,
            $bit,
            &[
                Step::Member {
                    ctor: DATA_MEMBER,
                    member: $member,
                    ty: Ty::Call($get),
                    name: $name,
                    words: [$offset, 0, 0],
                },
                Step::AtExit($dtor),
            ],
        )
    };
}

/// `POccluderGeometryObject::Bind()`: flag `0x10` of the descriptor's
/// word `+0x90` set on every call, then `m_planeCount`, `m_vertexCount`,
/// `m_edgeCount` (unsigned ints at 0, 4, 8) and `m_minBounds`,
/// `m_maxBounds` (`Vector3` at `0x10`, `0x20`). No method members.
pub const OBJECT_BIND: Bind = Bind {
    d: 0x01a8_5990,
    steps: &[
        Step::Or(0x01a8_5a20, 0x10),
        object_member!(1, 0x01a8_5a2c, GET_U32, 0x00b7_ac8c, 0, 0x00b0_ba30),
        object_member!(2, 0x01a8_5a5c, GET_U32, 0x00b1_6ec4, 4, 0x00b0_ba40),
        object_member!(4, 0x01a8_5a88, GET_U32, 0x00b7_ac9c, 8, 0x00b0_ba00),
        object_member!(8, 0x01a8_5ab4, GET_VECTOR3, 0x00b3_4afc, 0x10, 0x00b0_ba20),
        object_member!(
            0x10,
            0x01a8_5ae0,
            GET_VECTOR3,
            0x00b3_4b08,
            0x20,
            0x00b0_ba10
        ),
    ],
};

/// `0x005d6f00`'s callees: `PMatrix4_FromMatrix3x4` (thiscall `(out; m)`),
/// `PMatrix4_FromAxes` (thiscall `(out; a, b, c, d)`), `PMatrix4_Copy`
/// (thiscall `(dst; src)`), `PMatrix4_toWorldMatrix3x4` (thiscall
/// `(out; m)`).
pub const LOOK_AT_CALLS: [u32; 4] = [0x0045_1080, 0x0045_0de0, 0x0045_3340, 0x0045_1190];
/// The `f64` 0.999 `0x005d6f00` compares the direction's `y` with.
pub const LOOK_AT_LIMIT: u32 = 0x00b3_9370;
/// The words of `0x005d6f00`'s frame below `ebp`.
pub const LOOK_AT_FRAME_WORDS: u32 = 36;

pub mod look {
    //! `0x005d6f00` over game memory.

    use super::{LOOK_AT_CALLS, LOOK_AT_LIMIT};
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// `fstp dword` then `fld dword`: the value as the original keeps it.
    fn st(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    fn put(h: &mut impl Host, at: u32, v: [f64; 3]) {
        h.write(at, v.map(x87::store));
    }

    /// `1 / sqrt(x² + y² + z²)` as the original sums it: each partial sum,
    /// the root and the reciprocal stored as `f32`.
    fn inverse_length(v: [f64; 3]) -> f64 {
        let mut s = st(x87::mul(v[0], v[0]));
        s = st(x87::add(s, x87::mul(v[1], v[1])));
        s = st(x87::add(s, x87::mul(v[2], v[2])));
        let root = st(x87::sqrt(s));
        st(x87::div(1.0, root))
    }

    /// `v * k`, each component stored.
    fn scale(v: [f64; 3], k: f64) -> [f64; 3] {
        v.map(|c| st(x87::mul(c, k)))
    }

    /// `Phyre_PGameplay_PWorldMatrixFlyController_lookAt_005d6f00(this;
    /// target)` (thiscall, ret 4): the 4x4 of the controller's 3x4
    /// (`+4`) into its matrix (`+0x34`, every float through the x87);
    /// from its position (that matrix's row 3) towards `target`: the
    /// direction `f` normalised, an up vector `(0, 0, 1)` when `|f.y|` is
    /// over 0.999 (a NaN is not) else `(0, 1, 0)`, `c = up x f` and
    /// `r = c` normalised, `v = f x r`; `FromAxes(r, v, f, position)`
    /// copied into the matrix and written back as the 3x4. The vectors
    /// live in the frame at the original's `ebp` offsets (the 4x4 at
    /// `- 0x90`, the position at `- 0x50`, `f` at `- 0x40`, `r` at
    /// `- 0x30`, the position again at `- 0x20`, `v` at `- 0x10`). `eax`
    /// is `toWorldMatrix3x4`'s.
    pub fn look_at(h: &mut impl Host, ebp: u32, this: u32, target: u32) -> u32 {
        let [from_3x4, from_axes, copy, to_world] = LOOK_AT_CALLS;
        let m = ebp - 0x90;
        let src = this + 4;
        let dst = this + 0x34;
        h.call_sym(from_3x4, Conv::Thiscall(m), &[src]);
        for k in 0..16 {
            let w = x87::store(ld(h, m + 4 * k));
            h.write(dst + 4 * k, w);
        }
        let pos = [0x64, 0x68, 0x6c].map(|o| st(ld(h, this + o)));
        put(h, ebp - 0x50, pos);
        // `fsub dword [target + 4k]`: a memory operand
        let t: [u32; 3] = [0, 4, 8].map(|o| h.read(target + o));
        let d = [0, 1, 2].map(|k| {
            st(x87::Reg::constant(pos[k])
                .sub_m32(f32::from_bits(t[k]))
                .value())
        });
        let f = scale(d, inverse_length(d));
        put(h, ebp - 0x40, f);
        let limit = f64::from_bits(h.read(h.rebase(LOOK_AT_LIMIT)));
        let up = if st(f[1].abs()) > limit {
            [0.0, 0.0, 1.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let c = [
            st(x87::sub(x87::mul(up[1], f[2]), x87::mul(up[2], f[1]))),
            st(x87::sub(x87::mul(up[2], f[0]), x87::mul(f[2], up[0]))),
            st(x87::sub(x87::mul(f[1], up[0]), x87::mul(up[1], f[0]))),
        ];
        let r = scale(c, inverse_length(c));
        put(h, ebp - 0x30, r);
        let v = [
            st(x87::sub(x87::mul(r[2], f[1]), x87::mul(r[1], f[2]))),
            st(x87::sub(x87::mul(f[2], r[0]), x87::mul(r[2], f[0]))),
            st(x87::sub(x87::mul(f[0], r[1]), x87::mul(r[0], f[1]))),
        ];
        put(h, ebp - 0x10, v);
        put(h, ebp - 0x20, pos);
        let axes = h.call_sym(
            from_axes,
            Conv::Thiscall(m),
            &[ebp - 0x30, ebp - 0x10, ebp - 0x40, ebp - 0x20],
        );
        h.call_sym(copy, Conv::Thiscall(dst), &[axes]);
        h.call_sym(to_world, Conv::Thiscall(src), &[dst])
    }
}

/// `0x009f2a60`'s callees: `PMatrix4_TransformPoint3` (thiscall `(m; out,
/// p)`) and the plane test `planesBelow(planes, point)` (cdecl).
pub const BOX_PLANES_CALLS: [u32; 2] = [0x004c_9eb0, 0x009f_2890];
/// The words of `0x009f2a60`'s frame below `ebp`.
pub const BOX_PLANES_FRAME_WORDS: u32 = 32;

pub mod box_planes {
    //! `0x009f2a60` over game memory.

    use super::BOX_PLANES_CALLS;
    use ffx_bridge::x87::{self, Reg};
    use ffx_graphic::host::{Conv, Host};

    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    fn st(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    fn mem(h: &impl Host, at: u32) -> f32 {
        f32::from_bits(h.read(at))
    }

    /// `e * row`: the first two products in the register forms the
    /// original uses for the row (`fmul` with the row's word loaded, or
    /// with `e` loaded and the word as the memory operand for `x`'s
    /// second), the third with the row's word as the memory operand.
    fn scaled_row(h: &impl Host, row: u32, e: f64, second_from_memory: bool) -> [f64; 3] {
        let first = st(x87::mul(ld(h, row), e));
        let second = if second_from_memory {
            st(Reg::constant(e).mul_m32(mem(h, row + 4)).value())
        } else {
            st(x87::mul(ld(h, row + 4), e))
        };
        let third = st(Reg::constant(e).mul_m32(mem(h, row + 8)).value());
        [first, second, third]
    }

    pub(super) fn add3(p: [f64; 3], q: [f64; 3]) -> [f64; 3] {
        [0, 1, 2].map(|k| st(x87::add(p[k], q[k])))
    }

    /// A box's corners from its origin `t` and edges `a`, `b`, `c`, in the
    /// originals' order of sums (`0x009f2a60`, `0x009f4100`): `t + a`,
    /// `t + b`, `b + (t + a)`, `t + c`, `(t + a) + c`, `(t + b) + c`,
    /// `c + (b + (t + a))`, each component stored as `f32`.
    pub(super) fn box_points(t: [f64; 3], a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> [[f64; 3]; 7] {
        let ta = add3(t, a);
        let tb = add3(t, b);
        let tab = add3(b, ta);
        let tc = add3(t, c);
        let tac = add3(ta, c);
        let tbc = add3(tb, c);
        let tabc = add3(c, tab);
        [ta, tb, tab, tc, tac, tbc, tabc]
    }

    fn put(h: &mut impl Host, at: u32, v: [f64; 3]) {
        h.write(at, v.map(x87::store));
    }

    /// The eight corners' plane tests against the planes at `planes`, in
    /// the original's order.
    fn corners(h: &mut impl Host, ebp: u32, planes: u32) -> [u32; 8] {
        [0x50, 0x60, 0x70, 0x80, 0x10, 0x20, 0x30, 0x40]
            .map(|o| h.call_sym(BOX_PLANES_CALLS[1], Conv::Cdecl, &[planes, ebp - o]))
    }

    /// `Phyre_PRendering_PInternal_PCheckVisibleBoundsJob_boxPlanes_009f2a60(first,
    /// end, inner, origin, extent, m)` (cdecl): the box at `origin` (moved
    /// by `m`'s `TransformPoint3`) with edges `extent.x`, `.y`, `.z` times
    /// `m`'s first three rows; its eight corners (each sum stored as
    /// `f32`, in the frame at the original's `ebp` offsets) tested
    /// against every plane set from `first` up to `end` (0x40 bytes each,
    /// at least once): -1 when a corner is below one of them. Else 1 when
    /// some corner is below none of the `inner` set's planes, 0 when every
    /// corner is below one.
    #[allow(clippy::too_many_arguments)] // the original's six words, the host and the frame
    pub fn box_planes(
        h: &mut impl Host,
        ebp: u32,
        first: u32,
        end: u32,
        inner: u32,
        origin: u32,
        extent: u32,
        m: u32,
    ) -> u32 {
        let e = [0, 4, 8].map(|o| st(ld(h, extent + o)));
        let a = scaled_row(h, m, e[0], true);
        let b = scaled_row(h, m + 0x10, e[1], false);
        let c = scaled_row(h, m + 0x20, e[2], false);
        // the rows went through the words at `ebp - 0x40` (four each):
        // the transform overwrites three, the last row's fourth stays
        let w: u32 = h.read(m + 0x2c);
        h.write(ebp - 0x34, w);
        let p = [0, 4, 8].map(|o| st(ld(h, origin + o)));
        put(h, ebp - 0x80, p);
        h.call_sym(
            BOX_PLANES_CALLS[0],
            Conv::Thiscall(m),
            &[ebp - 0x40, ebp - 0x80],
        );
        let t = [0, 4, 8].map(|o| ld(h, ebp - 0x40 + o));
        let [ta, tb, tab, tc, tac, tbc, tabc] = box_points(t, a, b, c);
        for (o, v) in [
            (0x30, ta),
            (0x20, tb),
            (0x10, tab),
            (0x80, tc),
            (0x70, tac),
            (0x60, tbc),
            (0x50, tabc),
        ] {
            put(h, ebp - o, v);
        }
        let mut planes = first;
        loop {
            if corners(h, ebp, planes).iter().any(|&r| r != 0) {
                return u32::MAX;
            }
            planes = planes.wrapping_add(0x40);
            if planes >= end {
                break;
            }
        }
        u32::from(corners(h, ebp, inner).contains(&0))
    }
}

/// The words of `PVisibilityCheck_isBoxOccluded`'s frame below `ebp`.
pub const BOX_OCCLUDED_FRAME_WORDS: u32 = 0x118 / 4;

pub mod box_occluded {
    //! `PVisibilityCheck_isBoxOccluded` (`0x009f4100`) over game memory.

    use super::BOX_PLANES_CALLS;
    use super::box_planes::box_points;
    use ffx_bridge::x87::{self, Reg};
    use ffx_graphic::host::{Conv, Host};

    fn st(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// `e * m[at]` with the word as the memory operand.
    fn mul_m(h: &impl Host, e: f64, at: u32) -> f64 {
        st(Reg::constant(e).mul_m32(f32::from_bits(h.read(at))).value())
    }

    fn put(h: &mut impl Host, at: u32, v: [f64; 3]) {
        h.write(at, v.map(x87::store));
    }

    /// The corners' plane tests against the planes at `planes`, in the
    /// original's order.
    fn corners(h: &mut impl Host, ebp: u32, planes: u32) -> [u32; 8] {
        [0xd4, 0x104, 0xe4, 0xf4, 0x40, 0x54, 0x14, 0x24]
            .map(|o| h.call_sym(BOX_PLANES_CALLS[1], Conv::Cdecl, &[planes, ebp - o]))
    }

    /// The box at `origin` (moved by `m`'s `TransformPoint3`, the copy at
    /// `ebp - 0x118`, the result at `ebp - 0x24`) with edges `extent.x`,
    /// `.y`, `.z` times `m`'s rows: its eight corners, each sum stored as
    /// `f32` at the original's offsets.
    fn box_corners(h: &mut impl Host, ebp: u32, origin: u32, extent: u32, m: u32) {
        let ex = st(ld(h, extent));
        let a = [mul_m(h, ex, m), mul_m(h, ex, m + 4), mul_m(h, ex, m + 8)];
        let ey = st(ld(h, extent + 4));
        let b = [
            st(x87::mul(ld(h, m + 0x10), ey)),
            st(x87::mul(ld(h, m + 0x14), ey)),
            mul_m(h, ey, m + 0x18),
        ];
        let ez = st(ld(h, extent + 8));
        let c = [
            st(x87::mul(ld(h, m + 0x20), ez)),
            st(x87::mul(ld(h, m + 0x24), ez)),
            mul_m(h, ez, m + 0x28),
        ];
        let p = [0, 4, 8].map(|o| st(ld(h, origin + o)));
        put(h, ebp - 0x118, p);
        h.call_sym(
            BOX_PLANES_CALLS[0],
            Conv::Thiscall(m),
            &[ebp - 0x24, ebp - 0x118],
        );
        let t = [0, 4, 8].map(|o| ld(h, ebp - 0x24 + o));
        let [ta, tb, tab, tc, tac, tbc, tabc] = box_points(t, a, b, c);
        for (o, v) in [
            (0x14, ta),
            (0x54, tb),
            (0x40, tab),
            (0xf4, tc),
            (0xe4, tac),
            (0x104, tbc),
            (0xd4, tabc),
        ] {
            put(h, ebp - o, v);
        }
    }

    /// `PVisibilityCheck_isBoxOccluded(occluders; origin, extent, m)`
    /// (thiscall, ret 0xc; the occluder set `gatherOccluders_00a2f7d0`
    /// fills: four plane counts, the plane sets from `+0x10`, a block per
    /// occluder from `+0x310`): for each occluder in turn (a count of 0
    /// ends the list), the box's corners (built again each time, at the
    /// original's `ebp` offsets) against its plane sets; when no corner is
    /// below any of them and every corner is below one of its block's
    /// planes, 0 (occluded). Else 1. The result is `al`.
    pub fn is_box_occluded(
        h: &mut impl Host,
        ebp: u32,
        occluders: u32,
        origin: u32,
        extent: u32,
        m: u32,
    ) -> u8 {
        let counts_end = occluders.wrapping_add(0x10);
        let mut set = counts_end;
        let mut block = occluders.wrapping_add(0x310);
        let mut at = occluders;
        while at < counts_end {
            let n: u32 = h.read(at);
            at = at.wrapping_add(4);
            if n == 0 {
                return 1;
            }
            let end = (n << 6).wrapping_add(set);
            box_corners(h, ebp, origin, extent, m);
            let mut planes = set;
            let mut outside = false;
            loop {
                if corners(h, ebp, planes).iter().any(|&r| r != 0) {
                    outside = true;
                    break;
                }
                planes = planes.wrapping_add(0x40);
                if planes >= end {
                    break;
                }
            }
            if !outside && !corners(h, ebp, block).contains(&0) {
                return 0;
            }
            set = end;
            block = block.wrapping_add(0x40);
        }
        1
    }
}

/// `0x00a2fe70`'s callees: `PMatrix4_TransformPoint3` (thiscall `(m; out,
/// p)`), `PMatrix4_Rotate3` (thiscall `(m; out, v)`), `Phyre_Vec3Min`,
/// `Phyre_Vec3Max` (cdecl `(out, a, b)`).
pub const RAY_BOX_CALLS: [u32; 4] = [0x004c_9eb0, 0x004c_a140, 0x004f_8580, 0x004f_84f0];
/// Its constants: `f32` `FLT_MAX` (`0x00b17504`), `f32` `-FLT_MAX`
/// (`0x00b1f2d0`), `f64` `-FLT_MAX` (`0x00b6fe88`), `f64` `FLT_MAX`
/// (`0x00b368b0`).
pub const RAY_BOX_CONSTANTS: [u32; 4] = [0x00b1_7504, 0x00b1_f2d0, 0x00b6_fe88, 0x00b3_68b0];
/// The words of `0x00a2fe70`'s frame below `ebp`.
pub const RAY_BOX_FRAME_WORDS: u32 = 0x4c / 4;

pub mod ray_box {
    //! `0x00a2fe70` over game memory.

    use super::{RAY_BOX_CALLS, RAY_BOX_CONSTANTS};
    use ffx_bridge::x87::{self, Reg};
    use ffx_graphic::host::{Conv, Host};

    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    fn st(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    fn put(h: &mut impl Host, at: u32, v: [f64; 3]) {
        h.write(at, v.map(x87::store));
    }

    /// `m - x` with the word at `at` as the memory operand (`fsubr`).
    fn subr_m(h: &impl Host, x: f64, at: u32) -> f64 {
        Reg::constant(x)
            .subr_m32(f32::from_bits(h.read(at)))
            .value()
    }

    /// `Phyre_POccluderGeometry_rayBox_00a2fe70(m, lo, hi, p, d, near,
    /// far)` (cdecl) -> `al`: the ray from `p` along `d`, both moved into
    /// the box's space by `m` (`TransformPoint3` into `ebp - 0x4c`,
    /// `Rotate3` into `ebp - 0x2c`), against the box `lo..hi` (a slab
    /// test). A start inside the box (no coordinate outside; a NaN is
    /// inside) sets `near` = 0 and `far` = `FLT_MAX` first. The entry and
    /// exit distances per axis (`(lo - p) / d`, `(hi - p) / d` with the
    /// reciprocals stored, at `ebp - 0x2c` and `- 0x1c`) go through
    /// `Vec3Min`/`Vec3Max` (into `ebp - 0x4c`, `- 0x3c`); the interval
    /// starts at `(-FLT_MAX, FLT_MAX)`, x clipped against the `f64`
    /// limits, y and z plainly, and it is empty (0) when it ends before it
    /// starts or before 0 at any step. Else `near` and `far` (when not
    /// null) take it: 1. Comparisons with a NaN keep going.
    #[allow(clippy::too_many_arguments)] // the original's seven words, the host and the frame
    pub fn ray_box(
        h: &mut impl Host,
        ebp: u32,
        m: u32,
        lo: u32,
        hi: u32,
        p: u32,
        d: u32,
        near: u32,
        far: u32,
    ) -> u8 {
        let [transform, rotate, vmin, vmax] = RAY_BOX_CALLS;
        h.call_sym(transform, Conv::Thiscall(m), &[ebp - 0x4c, p]);
        h.call_sym(rotate, Conv::Thiscall(m), &[ebp - 0x2c, d]);
        let o = [0, 4, 8].map(|k| ld(h, ebp - 0x4c + k));
        let b_lo = [0, 4, 8].map(|k| ld(h, lo + k));
        let b_hi = [0, 4, 8].map(|k| ld(h, hi + k));
        #[allow(clippy::neg_cmp_op_on_partial_ord)] // a NaN coordinate counts as inside
        let inside = (0..3).all(|k| !(b_hi[k] < o[k]) && !(b_lo[k] > o[k]));
        let [max32, min32, min64, max64] = RAY_BOX_CONSTANTS;
        let max = st(ld(h, h.rebase(max32)));
        if inside {
            if near != 0 {
                h.write(near, 0u32);
            }
            if far != 0 {
                h.write(far, x87::store(max));
            }
        }
        let r = [0, 4, 8].map(|k| ld(h, ebp - 0x2c + k));
        let inv = [
            st(x87::div(1.0, r[0])),
            st(x87::div(1.0, r[1])),
            st(Reg::ONE.div_m32(f32::from_bits(h.read(ebp - 0x24))).value()),
        ];
        let mut t_lo = st(ld(h, h.rebase(min32)));
        let mut t_hi = max;
        // the bounds read again (after `near` and `far` are written)
        let a = [0u32, 1, 2].map(|k| st(x87::sub(ld(h, lo + 4 * k), o[k as usize])));
        let b = [
            st(subr_m(h, o[0], hi)),
            st(x87::sub(ld(h, hi + 4), o[1])),
            st(subr_m(h, o[2], hi + 8)),
        ];
        let ta = [0, 1, 2].map(|k| st(x87::mul(a[k], inv[k])));
        let tb = [0, 1, 2].map(|k| st(x87::mul(inv[k], b[k])));
        put(h, ebp - 0x2c, ta);
        put(h, ebp - 0x1c, tb);
        h.call_sym(vmin, Conv::Cdecl, &[ebp - 0x4c, ebp - 0x2c, ebp - 0x1c]);
        h.call_sym(vmax, Conv::Cdecl, &[ebp - 0x3c, ebp - 0x2c, ebp - 0x1c]);
        let enter = [0, 4, 8].map(|k| ld(h, ebp - 0x4c + k));
        let leave = [0, 4, 8].map(|k| ld(h, ebp - 0x3c + k));
        if enter[0] > f64::from_bits(h.read(h.rebase(min64))) {
            t_lo = enter[0];
        }
        if leave[0] < f64::from_bits(h.read(h.rebase(max64))) {
            t_hi = leave[0];
        }
        if t_hi < t_lo || t_hi < 0.0 {
            return 0;
        }
        for k in 1..3 {
            if enter[k] > t_lo {
                t_lo = enter[k];
            }
            if leave[k] < t_hi {
                t_hi = leave[k];
            }
            if t_hi < t_lo || t_hi < 0.0 {
                return 0;
            }
        }
        if near != 0 {
            h.write(near, x87::store(t_lo));
        }
        if far != 0 {
            h.write(far, x87::store(t_hi));
        }
        1
    }
}

/// `0x00a30260`'s callees: `Phyre_PhyreFree` (cdecl), `PArray8::reset`
/// (thiscall `(array; n)`), `PMatrix4_Identity` (cdecl `(out)`),
/// `PMatrix4_FromMatrix3x4` (thiscall `(out; m)`), `PMatrix4_Inverse`
/// (cdecl `(out, m)`), `PMatrix4_Copy_004c9430` (thiscall `(dst; src)`),
/// `PMatrix4_TransformPoint3` (thiscall `(m; out, p)`),
/// `FFXApplication_pointInBox` (cdecl `(p, lo, hi)` -> `al`),
/// `PMatrix4_Multiply` (cdecl `(out, a, b)` -> `out`),
/// `POccluderGeometry_boxVisibleBetween` (cdecl `(lo, hi, m)` -> `al`),
/// `Phyre_Vector3_lengthTo` (cdecl `(out, v)` -> `out`), `rayBox`, the
/// occluder's weight `0x00a2ef70` (cdecl, five words, `ST0`),
/// `PArray::shrink` (thiscall `(array; n)`).
pub const SELECT_CALLS: [u32; 14] = [
    0x0042_fb20,
    0x00a3_0780,
    0x0045_1bb0,
    0x0045_1080,
    0x004c_c0c0,
    0x004c_9430,
    0x004c_9eb0,
    0x00a2_fc90,
    0x004c_a360,
    0x00a2_f9b0,
    0x0054_bd60,
    0x00a2_fe70,
    0x00a2_ef70,
    0x00a3_07f0,
];
/// The word `0x00a30260` clears before it selects.
pub const SELECT_CLEARED: u32 = 0x01a8_5414;
/// The words of `0x00a30260`'s frame below `ebp`.
pub const SELECT_FRAME_WORDS: u32 = 0x34c / 4;

pub mod select {
    //! `0x00a30260` over game memory.

    use super::{SELECT_CALLS, SELECT_CLEARED};
    use ffx_bridge::x87::{self, Reg};
    use ffx_graphic::host::{Conv, Host};

    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    fn copy_words(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        for k in 0..n {
            let w: u32 = h.read(src.wrapping_add(4 * k));
            h.write(dst.wrapping_add(4 * k), w);
        }
    }

    /// `Phyre_POccluderGeometry_selectOccluders_00a30260(array, items,
    /// count, stride, owner, ray)` (cdecl) -> 0: the candidate occluders
    /// (`items`, `stride` bytes apart: `{geometry, 3x4 or 0}`) kept in
    /// `array` (8-byte `{item, weight}` entries). Nothing without a
    /// stride; no items: the array emptied (its buffer freed when owned);
    /// a null list: nothing. The array grows to `count` first (its error
    /// ends it), the word `0x01a85414` is cleared. For each item: its
    /// matrix `M` (`ebp - 0xcc`; the identity without a 3x4) and its
    /// inverse (`ebp - 0x10c`); the owner's camera (`[owner + 0xb0] +
    /// 0x20`, the origin without) in the occluder's space (`ebp - 0x8c`);
    /// an item is dropped when the camera is inside its box, when the box
    /// is not visible through the owner's view matrix (`owner + 0x70`,
    /// times `M` for a 3x4), and, with a ray (`{from, to}`), when the ray
    /// meets the box beyond its start (`near > 0`) and leaves it before
    /// the ray's end. A kept item gets the weight `0x00a2ef70(lo, hi,
    /// owner, M, inverse)`. The array is shrunk to the entries written.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one original
    pub fn select_occluders(
        h: &mut impl Host,
        ebp: u32,
        array: u32,
        items: u32,
        count: u32,
        stride: u32,
        owner: u32,
        ray: u32,
    ) -> u32 {
        let [
            free,
            reset,
            identity,
            from_3x4,
            inverse,
            copy,
            transform,
            in_box,
            multiply,
            visible,
            length,
            ray_box,
            weight,
            shrink,
        ] = SELECT_CALLS;
        if stride == 0 {
            return 0;
        }
        if count == 0 {
            let w: u32 = h.read(array);
            let w = w & 0x8000_0000;
            h.write(array, w);
            if w == 0 {
                let buffer: u32 = h.read(array + 4);
                h.call_sym(free, Conv::Cdecl, &[buffer]);
                h.write(array + 4, 0u32);
            }
            return 0;
        }
        if items == 0 {
            return 0;
        }
        if count > h.read::<u32>(array) & 0x7fff_ffff
            && h.call_sym(reset, Conv::Thiscall(array), &[count]) != 0
        {
            return 0;
        }
        let cleared = h.rebase(SELECT_CLEARED);
        h.write(cleared, 0u32);
        let start: u32 = h.read(array + 4);
        let mut cursor = start;
        let ident = ebp - 0x14c;
        h.call_sym(identity, Conv::Cdecl, &[ident]);
        let camera: u32 = h.read(owner + 0xb0);
        let at = if camera == 0 {
            h.write(ebp - 0x68, [0u32; 3]);
            ebp - 0x68
        } else {
            let c = [0x20, 0x24, 0x28].map(|o| x87::store(ld(h, camera + o)));
            h.write(ebp - 0x38, c);
            h.write(ebp - 0x78, c);
            ebp - 0x78
        };
        copy_words(h, ebp - 0x48, at, 3);
        // junk read: [ebp-0x6c] / [ebp-0x5c] never written; 0 (brief 2026-10-24)
        h.write(ebp - 0x3c, 0u32);
        let end = count.wrapping_mul(stride).wrapping_add(items);
        let m = ebp - 0xcc;
        let inv = ebp - 0x10c;
        let mut item = items;
        while item < end {
            let geometry: u32 = h.read(item);
            let local: u32 = h.read(item + 4);
            let src = if local == 0 {
                copy_words(h, ebp - 0x20c, ident, 16);
                ebp - 0x20c
            } else {
                h.call_sym(from_3x4, Conv::Thiscall(ebp - 0x18c), &[local]);
                ebp - 0x18c
            };
            copy_words(h, m, src, 16);
            let src = if local == 0 {
                copy_words(h, ebp - 0x28c, ident, 16);
                ebp - 0x28c
            } else {
                let r = h.call_sym(inverse, Conv::Cdecl, &[ebp - 0x30c, m]);
                h.call_sym(copy, Conv::Thiscall(ebp - 0x2cc), &[r]);
                ebp - 0x2cc
            };
            copy_words(h, inv, src, 16);
            let lo = geometry.wrapping_add(0x10);
            let hi = geometry.wrapping_add(0x20);
            h.call_sym(transform, Conv::Thiscall(inv), &[ebp - 0x8c, ebp - 0x48]);
            let keep = if h.call_sym(in_box, Conv::Cdecl, &[ebp - 0x8c, lo, hi]) as u8 != 0 {
                false
            } else {
                let view = owner.wrapping_add(0x70);
                let r = if local == 0 {
                    copy_words(h, ebp - 0x1cc, view, 16);
                    ebp - 0x1cc
                } else {
                    h.call_sym(multiply, Conv::Cdecl, &[ebp - 0x34c, view, m])
                };
                copy_words(h, ebp - 0x24c, r, 16);
                if h.call_sym(visible, Conv::Cdecl, &[lo, hi, ebp - 0x24c]) as u8 == 0 {
                    false
                } else {
                    ray == 0 || !blocks_ray(h, ebp, ray, lo, hi, [length, ray_box])
                }
            };
            if keep {
                let w = h.call_sym_st0_f64(weight, Conv::Cdecl, &[lo, hi, owner, m, inv]);
                h.write(cursor + 4, x87::store(w));
                h.write(cursor, item);
                cursor = cursor.wrapping_add(8);
            }
            item = item.wrapping_add(stride);
        }
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)] // sar
        let n = (cursor.wrapping_sub(start) as i32 >> 3) as u32;
        h.call_sym(shrink, Conv::Thiscall(array), &[n]);
        0
    }

    /// Whether the ray `{from, to}` meets the box (in the occluder's space,
    /// the inverse at `ebp - 0x10c`) beyond its start and leaves it before
    /// its end: the direction `to - from` (at `ebp - 0x28`), its length
    /// (`lengthTo` into `ebp - 0x7c`), the unit direction at `ebp - 0x58`,
    /// `rayBox` with `near`/`far` at `ebp - 0x14`/`- 0x10`.
    fn blocks_ray(
        h: &mut impl Host,
        ebp: u32,
        ray: u32,
        lo: u32,
        hi: u32,
        calls: [u32; 2],
    ) -> bool {
        let [length, ray_box] = calls;
        let d = [0u32, 4, 8].map(|o| {
            let to = ld(h, ray + 0x10 + o);
            x87::store(
                Reg::constant(to)
                    .sub_m32(f32::from_bits(h.read(ray + o)))
                    .value(),
            )
        });
        h.write(ebp - 0x28, d);
        let r = h.call_sym(length, Conv::Cdecl, &[ebp - 0x7c, ebp - 0x28]);
        let len = x87::load(x87::store(ld(h, r)));
        h.write(ebp - 4, x87::store(len));
        let unit = d.map(|c| x87::store(x87::div(x87::load(c), len)));
        h.write(ebp - 0x58, unit);
        let hit = h.call_sym(
            ray_box,
            Conv::Cdecl,
            &[ebp - 0x10c, lo, hi, ray, ebp - 0x58, ebp - 0x14, ebp - 0x10],
        ) as u8;
        if hit == 0 {
            return false;
        }
        let near = ld(h, ebp - 0x14);
        let far = ld(h, ebp - 0x10);
        0.0 < near && len > far
    }
}

/// `0x00a2ef70`'s callees: `FFXApplication_pointInBox` (cdecl `(p, lo,
/// hi)` -> `al`), `PMatrix4_TransformPoint3` (thiscall `(m; out, p)`),
/// `FFXApplication_boxOutcode` (cdecl `(lo, hi, p)`), `PMatrix4_Multiply`
/// (cdecl `(out, a, b)`), `FFXApplication_vectorMin`/`vectorMax` (cdecl
/// `(out, a, b)` -> `out`), `FFXApplication_clipPolygonPlane` (cdecl
/// `(poly, n, out, &n, plane)` -> the clipped polygon),
/// `Phyre_PGameplay_polygonArea` (cdecl `(poly, n)`, `ST0`).
pub const COVER_CALLS: [u32; 8] = [
    0x00a2_fc90,
    0x004c_9eb0,
    0x005a_4180,
    0x004c_a360,
    0x00a1_b620,
    0x00a1_b560,
    0x00a2_f5e0,
    0x005a_3af0,
];
/// The clip planes `0x00a2ef70` hands `clipPolygonPlane`: x = 1, y = 1,
/// x = -1, y = -1 (`FFXApplication_axisX`, `axisY`, `axisXNeg`,
/// `axisYNeg`).
pub const COVER_PLANES: [u32; 4] = [0x00a2_f780, 0x00a2_f7a0, 0x00a2_f740, 0x00a2_f760];
/// The silhouette table (8 bytes per outcode: a corner count, then each
/// corner's lo/hi bits), `FLT_MAX`, `-FLT_MAX`, the `f32` 1e-5, the `f64`
/// 0.5, 0.25 and 0.
pub const COVER_DATA: [u32; 7] = [
    0x00b7_9c20,
    0x00b1_7504,
    0x00b1_f2d0,
    0x00b1_f2ac,
    0x00b9_2220,
    0x00b3_4f90,
    0x00b0_db90,
];
/// The words of `0x00a2ef70`'s frame below `ebp`.
pub const COVER_FRAME_WORDS: u32 = 0x2b8 / 4;

pub mod cover {
    //! `0x00a2ef70` over game memory.

    use super::{COVER_CALLS, COVER_DATA, COVER_PLANES};
    use ffx_bridge::x87::{self, Reg};
    use ffx_graphic::host::{Conv, Host};

    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    fn st(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    fn put(h: &mut impl Host, at: u32, v: &[f64]) {
        for (k, &x) in v.iter().enumerate() {
            h.write(at + 4 * k as u32, x87::store(x));
        }
    }

    fn f64_at(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    /// `x + m` with the word at `at` as the memory operand.
    fn add_m(h: &impl Host, x: f64, at: u32) -> f64 {
        Reg::constant(x).add_m32(f32::from_bits(h.read(at))).value()
    }

    /// One corner `(x, y, z)` through the 4x4 `v` (the words at `vm`):
    /// the four sums in the original's order, each stored.
    fn project(h: &impl Host, vm: u32, p: [f64; 3]) -> [f64; 4] {
        let v = |k: u32| ld(h, vm + 4 * k);
        let [x, y, z] = p;
        let px = x87::add(
            x87::add(
                x87::mul(v(8), z),
                x87::add(x87::mul(v(0), x), x87::mul(v(4), y)),
            ),
            v(12),
        );
        let py = add_m(
            h,
            x87::add(
                x87::add(x87::mul(v(5), y), x87::mul(v(1), x)),
                x87::mul(v(9), z),
            ),
            vm + 0x34,
        );
        let pz = add_m(
            h,
            x87::add(
                x87::add(x87::mul(v(6), y), x87::mul(v(2), x)),
                x87::mul(v(10), z),
            ),
            vm + 0x38,
        );
        let pw = x87::add(
            v(15),
            x87::add(
                x87::mul(z, v(11)),
                x87::add(x87::mul(x, v(3)), x87::mul(y, v(7))),
            ),
        );
        [st(px), st(py), st(pz), st(pw)]
    }

    /// `Phyre_POccluderGeometry_screenCover_00a2ef70(lo, hi, owner, m,
    /// inverse)` (cdecl, `ST0`): how much of the screen the box `lo..hi`
    /// covers from the owner's camera (`[owner + 0xb0] + 0x20`, the origin
    /// without; its fourth word `[ebp - 0x210]` is never written: 0): 1
    /// with the camera inside the box; else the camera in the box's space
    /// (`inverse`) picks the box's silhouette (`boxOutcode` into the table
    /// at `0x00b79c20`), whose corners go through the owner's view matrix
    /// times `m` (`ebp - 0x298`) and the perspective divide (into `ebp -
    /// 0x64`, 16 bytes each, the bounds by `vectorMin`/`vectorMax`); 1 with
    /// no silhouette or a corner whose `1/w` is below 1e-5. The polygon is
    /// clipped to the screen square on each side it passes (`ebp - 0x124`,
    /// `- 0x1e4`, its corner count at `ebp - 0x1e8`), and its area times
    /// 0.5 and 0.25 kept between 0 and 1 (a NaN gives 1).
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)] // one original
    pub fn screen_cover(
        h: &mut impl Host,
        ebp: u32,
        lo: u32,
        hi: u32,
        owner: u32,
        m: u32,
        inverse: u32,
    ) -> f32 {
        let [in_box, transform, outcode, multiply, vmin, vmax, _, area] = COVER_CALLS;
        let [table, max32, min32, eps32, half, quarter, zero] = COVER_DATA;
        let count_at = ebp - 0x1e8;
        h.write(count_at, 1.0f32.to_bits());
        let camera: u32 = h.read(owner + 0xb0);
        let c = if camera == 0 {
            [0u32; 3]
        } else {
            [0x20, 0x24, 0x28].map(|o| x87::store(ld(h, camera + o)))
        };
        h.write(ebp - 0x21c, c);
        h.write(ebp - 0x204, c);
        // junk read: [ebp-0x210] never written; 0 (brief 2026-10-24)
        h.write(ebp - 0x1f8, 0u32);
        if h.call_sym(in_box, Conv::Cdecl, &[ebp - 0x204, lo, hi]) as u8 != 0 {
            return 1.0;
        }
        h.call_sym(
            transform,
            Conv::Thiscall(inverse),
            &[ebp - 0x21c, ebp - 0x204],
        );
        let code = h.call_sym(outcode, Conv::Cdecl, &[lo, hi, ebp - 0x21c]);
        let tab = h.rebase(table).wrapping_add(code.wrapping_mul(8));
        let vm = ebp - 0x298;
        h.call_sym(multiply, Conv::Cdecl, &[vm, owner.wrapping_add(0x70), m]);
        let n = u32::from(h.read::<u8>(tab));
        if n == 0 {
            return 1.0;
        }
        let max = st(ld(h, h.rebase(max32)));
        let min = st(ld(h, h.rebase(min32)));
        put(h, ebp - 0x24c, &[max; 4]);
        put(h, ebp - 0x23c, &[min; 4]);
        let eps = ld(h, h.rebase(eps32));
        let mut behind = false;
        for i in 0..n {
            let bits = h.read::<u8>(tab + 1 + i);
            let corner = [(1u8, 0u32), (2, 4), (4, 8)].map(|(bit, o)| {
                let from = if bits & bit == 0 { lo } else { hi };
                st(ld(h, from + o))
            });
            let p = project(h, vm, corner);
            let at = ebp - 0x64 + 0x10 * i;
            put(h, at, &p);
            let r = st(x87::div(1.0, p[3]));
            if eps > r {
                behind = true;
            }
            let q = [
                st(x87::mul(p[0], r)),
                st(x87::mul(p[1], r)),
                st(x87::mul(p[2], r)),
                st(x87::mul(p[3], r)),
            ];
            put(h, at, &q);
            let out = h.call_sym(vmin, Conv::Cdecl, &[ebp - 0x2b8, ebp - 0x24c, at]);
            copy4(h, ebp - 0x24c, out);
            let out = h.call_sym(vmax, Conv::Cdecl, &[ebp - 0x2a8, ebp - 0x23c, at]);
            copy4(h, ebp - 0x23c, out);
        }
        if behind {
            return 1.0;
        }
        h.write(count_at, n);
        let mut poly = ebp - 0x64;
        let mut count = n;
        let [px, py, nx, ny] = COVER_PLANES;
        let a = st(x87::sub(ld(h, ebp - 0x23c), 1.0));
        if 0.0 < a {
            poly = clip_to(h, poly, count, ebp - 0x124, count_at, px);
            count = h.read(count_at);
        }
        let b = st(x87::sub(ld(h, ebp - 0x238), 1.0));
        if b > 0.0 {
            poly = clip_to(h, poly, count, ebp - 0x1e4, count_at, py);
            count = h.read(count_at);
        }
        let c = st(-x87::add(ld(h, ebp - 0x24c), 1.0));
        if c > 0.0 {
            let out = if poly == ebp - 0x124 {
                ebp - 0x1e4
            } else {
                ebp - 0x124
            };
            poly = clip_to(h, poly, count, out, count_at, nx);
            count = h.read(count_at);
        }
        let d = st(-x87::add(1.0, ld(h, ebp - 0x248)));
        if 0.0 < d {
            let out = if poly == ebp - 0x124 {
                ebp - 0x1e4
            } else {
                ebp - 0x124
            };
            poly = clip_to(h, poly, count, out, count_at, ny);
            count = h.read(count_at);
        }
        let s = h.call_sym_st0_f64(area, Conv::Cdecl, &[poly, count]);
        let s = st(x87::mul(x87::mul(s, f64_at(h, half)), f64_at(h, quarter)));
        // `1 <= s` or a NaN: 1
        let s = if 1.0 > s { s } else { 1.0 };
        let s = st(s);
        let s = if s < f64_at(h, zero) { 0.0 } else { s };
        let r = f32::from_bits(x87::store(s));
        h.write(count_at, r.to_bits());
        r
    }

    fn copy4(h: &mut impl Host, dst: u32, src: u32) {
        for k in 0..4 {
            let x = ld(h, src + 4 * k);
            h.write(dst + 4 * k, x87::store(x));
        }
    }

    /// `clipPolygonPlane(poly, n, out, &n, plane)`: the clipped polygon.
    fn clip_to(h: &mut impl Host, poly: u32, n: u32, out: u32, count_at: u32, plane: u32) -> u32 {
        let [.., clip, _] = COVER_CALLS;
        let plane = h.rebase(plane);
        h.call_sym(clip, Conv::Cdecl, &[poly, n, out, count_at, plane])
    }
}

/// `0x005d7450`'s CRT intrinsics (the import thunks): `_CIsqrt`,
/// `_CIasin`, `_CIatan2`, `_CIcos`, `_CIsin`; and the `f64` 2pi (rounded
/// to `f32`) its angles wrap at.
pub const TURN_INTRINSICS: [u32; 5] = [
    0x0094_97b8,
    0x0094_98f2,
    0x0094_98bc,
    0x0094_98c2,
    0x0094_98c8,
];
/// `g_TwoPiF64`.
pub const TWO_PI: u32 = 0x00b3_9378;
/// The words of `0x005d7450`'s frame below `ebp`.
pub const TURN_FRAME_WORDS: u32 = 0x9c / 4;

pub mod turn {
    //! `0x005d7450` over game memory.

    use super::{LOOK_AT_CALLS, TURN_INTRINSICS, TWO_PI};
    use ffx_bridge::x87::{self, Reg};
    use ffx_graphic::host::{Conv, Host};

    fn ld(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    fn st(x: f64) -> f64 {
        x87::load(x87::store(x))
    }

    fn put(h: &mut impl Host, at: u32, v: [f64; 3]) {
        h.write(at, v.map(x87::store));
    }

    fn mul_m(x: f64, m: f64) -> f64 {
        // `fmul dword [m]` of a stored value (no signalling NaN)
        Reg::constant(x)
            .mul_m32(f32::from_bits(x87::store(m)))
            .value()
    }

    /// `1 / sqrt(x² + y² + z²)`, each partial sum, the root and the
    /// reciprocal stored as `f32` (`_CIsqrt` rounds as the x87's `fsqrt`).
    fn inverse_length(v: [f64; 3]) -> f64 {
        let mut s = st(x87::mul(v[0], v[0]));
        s = st(x87::add(s, x87::mul(v[1], v[1])));
        s = st(x87::add(s, x87::mul(v[2], v[2])));
        let root = st(x87::sqrt(s));
        st(x87::div(1.0, root))
    }

    /// An angle wrapped into `0..=2pi` as the original steps it: `+ 2pi`
    /// while below 0, then `- 2pi` while above `2pi`, each step stored.
    fn wrap(a: f64, two_pi: f64) -> f64 {
        let mut a = a;
        while a < 0.0 {
            a = st(x87::add(a, two_pi));
        }
        while a > two_pi {
            a = st(x87::sub(a, two_pi));
        }
        a
    }

    /// `Phyre_PGameplay_cameraTurn_005d7450(this; yaw, pitch)` (thiscall,
    /// ret 8): the 4x4 of the controller's 3x4 (`+4`) into its matrix
    /// (`+0x34`, through the x87); its forward axis (row 2, `+0x54`)
    /// normalised gives the current pitch (`asin` of `y`) and yaw
    /// (`atan2(x, z)`); the new pitch is that less `pitch`, the new yaw
    /// that plus `yaw`, both wrapped into `0..=2pi`. The new forward
    /// `F = (sin yaw cos pitch, sin pitch, cos yaw cos pitch)` (at `ebp -
    /// 0x5c`), the side `A = (cos yaw, 0, -sin yaw)` (`ebp - 0x4c`), the
    /// up `R = A x F` normalised (`ebp - 0x28`) and the position (row 3,
    /// at `ebp - 0x18`) through `FromAxes(A, R, F, position)` into the
    /// matrix and back to the 3x4; `toWorldMatrix3x4`'s `eax`.
    pub fn turn(h: &mut impl Host, ebp: u32, this: u32, yaw: u32, pitch: u32) -> u32 {
        let [from_3x4, from_axes, copy, to_world] = LOOK_AT_CALLS;
        let [_, asin, atan2, cos, sin] = TURN_INTRINSICS.map(|a| h.rebase(a));
        let m = ebp - 0x9c;
        let src = this + 4;
        let dst = this + 0x34;
        h.call_sym(from_3x4, Conv::Thiscall(m), &[src]);
        for k in 0..16 {
            let w = x87::store(ld(h, m + 4 * k));
            h.write(dst + 4 * k, w);
        }
        let f = [0x54, 0x58, 0x5c].map(|o| ld(h, this + o));
        let inv = inverse_length(f);
        let n = [
            st(x87::mul(f[0], inv)),
            st(x87::mul(f[1], inv)),
            st(Reg::constant(inv)
                .mul_m32(f32::from_bits(h.read(this + 0x5c)))
                .value()),
        ];
        let a = f64::from(h.call_intrinsic(asin, n[1]));
        let p = st(Reg::constant(a).sub_m32(f32::from_bits(pitch)).value());
        let y0 = f64::from(h.call_intrinsic2(atan2, n[0], n[2]));
        let y = st(Reg::constant(y0).add_m32(f32::from_bits(yaw)).value());
        let two_pi = f64::from_bits(h.read(h.rebase(TWO_PI)));
        let p = wrap(p, two_pi);
        let y = wrap(y, two_pi);
        let cp = f64::from(h.call_intrinsic(cos, p));
        let sy = f64::from(h.call_intrinsic(sin, y));
        let cy = f64::from(h.call_intrinsic(cos, y));
        let fx = st(mul_m(sy, cp));
        let sp = f64::from(h.call_intrinsic(sin, p));
        let fz = st(mul_m(cy, cp));
        put(h, ebp - 0x5c, [fx, sp, fz]);
        // junk read: [ebp-0x1c] never written; 0 (brief 2026-10-24)
        h.write(ebp - 0x50, 0u32);
        let az = st(-sy);
        put(h, ebp - 0x4c, [cy, 0.0, az]);
        let c = [
            st(x87::sub(x87::mul(az, sp), x87::mul(fz, 0.0))),
            st(x87::sub(x87::mul(fz, cy), x87::mul(az, fx))),
            st(x87::sub(x87::mul(0.0, fx), x87::mul(cy, sp))),
        ];
        let inv = inverse_length(c);
        let r = [
            st(x87::mul(c[0], inv)),
            st(x87::mul(c[1], inv)),
            st(mul_m(inv, c[2])),
        ];
        put(h, ebp - 0x28, r);
        let pos = [0x64, 0x68, 0x6c].map(|o| st(ld(h, this + o)));
        put(h, ebp - 0x3c, pos);
        put(h, ebp - 0x18, pos);
        let axes = h.call_sym(
            from_axes,
            Conv::Thiscall(m),
            &[ebp - 0x4c, ebp - 0x28, ebp - 0x5c, ebp - 0x18],
        );
        h.call_sym(copy, Conv::Thiscall(dst), &[axes]);
        h.call_sym(to_world, Conv::Thiscall(src), &[dst])
    }
}
