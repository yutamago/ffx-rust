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
