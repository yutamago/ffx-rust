//! M5 stage 1, list `scene`, batch 6: the gameplay maths (a polygon's
//! signed area, a 3x3 rotation product), the scene node search by name
//! and the attachable component's target, the fly controller's matrix
//! set and the animation utility's time span. Conventions as in
//! [`crate::m5p_scene_b`]; floats on the x87 at 53 bits
//! ([`ffx_bridge::x87`]), each store rounded to `f32`.
//!
//! Frames: where the original hands a callee the address of its locals,
//! the bridge takes a frame (`frame`, per call: the shim's `StackFrame`)
//! laid out at the original's `ebp` offsets, so the callee writes the
//! same words.
//!
//! Image addresses are rebased before use. Nothing here keeps state of
//! its own (worker threads: per-call scratch only).

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

pub use crate::m5p_scene_b::{PHYRE_FREE, TO_WORLD_MATRIX};
pub use crate::m5p_scene_c::PHYRE_MALLOC;
pub use crate::m5p_scene_e::MEMCPY;

/// `PMatrix4_FromMatrix3x4_00451080` (thiscall `(out; m)`, `ret 4`).
pub const FROM_MATRIX3X4: u32 = 0x0045_1080;
/// `0x00516a20` (thiscall `(&start, &end)`, `ret 8`): the animation's
/// time range; 0 when it has one.
pub const TIME_RANGE: u32 = 0x0051_6a20;
/// The empty name the attachable component points its two name fields
/// at (a `.data` word).
pub const EMPTY_NAME: u32 = 0x00ca_91f8;

pub mod bridge {
    //! The ports over game memory.

    use super::{
        Conv, EMPTY_NAME, FROM_MATRIX3X4, Host, MEMCPY, Mem, PHYRE_FREE, PHYRE_MALLOC, TIME_RANGE,
        TO_WORLD_MATRIX, x87,
    };

    /// A float word as `fld dword` loads it.
    fn ld(m: &impl Mem, at: u32) -> f64 {
        let bits: u32 = m.read(at);
        x87::load(bits)
    }

    /// `0x005a3af0` (cdecl `(points, n)`, a float in `ST0`): twice the
    /// signed area of the polygon of `n` 16-byte points (`x`, `y` first):
    /// the sum over each edge from the previous point of `(prev.y + y) *
    /// (prev.x - x)`, each term and sum rounded to `f32`. 0 for no
    /// points.
    pub fn polygon_area(m: &impl Mem, points: u32, n: u32) -> f32 {
        let mut acc = 0.0f32;
        if n == 0 {
            return acc;
        }
        let mut prev = points.wrapping_add(n.wrapping_mul(16)).wrapping_sub(16);
        let mut cur = points;
        for _ in 0..n {
            let y = x87::store(x87::add(ld(m, prev + 4), ld(m, cur + 4)));
            let dx = x87::store(x87::sub(ld(m, prev), ld(m, cur)));
            let term = x87::store(x87::mul(x87::load(y), x87::load(dx)));
            acc = f32::from_bits(x87::store(x87::add(
                x87::load(term),
                x87::load(acc.to_bits()),
            )));
            prev = cur;
            cur = cur.wrapping_add(16);
        }
        acc
    }

    /// `(a + b) + c` of three products on the x87, stored as `f32`.
    fn dot3(p: [(f64, f64); 3]) -> u32 {
        let [(a0, a1), (b0, b1), (c0, c1)] = p;
        let s = x87::add(x87::mul(a0, a1), x87::mul(b0, b1));
        x87::store(x87::add(s, x87::mul(c0, c1)))
    }

    /// `0x00547870` (thiscall `(out, b)`, `ret 8`): the 3x3 part of the
    /// product of two matrices (rows of 16 bytes) into `out` (its fourth
    /// words untouched), each element summed in the original's order.
    /// `out`.
    pub fn rotation_product(m: &mut impl Mem, this: u32, out: u32, b: u32) -> u32 {
        let a = |i: u32| ld(m, this + 4 * i);
        let bb = |i: u32| ld(m, b + 4 * i);
        let r = [
            dot3([(bb(0), a(0)), (a(4), bb(1)), (bb(2), a(8))]),
            dot3([(a(5), bb(1)), (bb(0), a(1)), (bb(2), a(9))]),
            dot3([(a(6), bb(1)), (a(2), bb(0)), (a(10), bb(2))]),
            dot3([(a(0), bb(4)), (bb(5), a(4)), (a(8), bb(6))]),
            dot3([(bb(5), a(5)), (a(1), bb(4)), (bb(6), a(9))]),
            dot3([(a(2), bb(4)), (bb(5), a(6)), (a(10), bb(6))]),
            dot3([(a(4), bb(9)), (a(0), bb(8)), (bb(10), a(8))]),
            dot3([(a(5), bb(9)), (a(1), bb(8)), (bb(10), a(9))]),
            dot3([(a(2), bb(8)), (a(6), bb(9)), (a(10), bb(10))]),
        ];
        for (k, v) in r.into_iter().enumerate() {
            let k = u32::try_from(k).unwrap_or(0);
            m.write(out + 0x10 * (k / 3) + 4 * (k % 3), v);
        }
        out
    }

    /// Whether the name a node holds (`+0x50`, its bit 0 an ownership
    /// mark) equals `name` (bytes; false for either missing).
    fn node_named(m: &impl Mem, node: u32, name: u32) -> bool {
        let lo: u8 = m.read(node + 0x50);
        let p: u32 = m.read(node + 0x50);
        let s = p.wrapping_sub(u32::from(lo & 1));
        if name == 0 || s == 0 {
            return false;
        }
        let mut i = 0u32;
        loop {
            let a: u8 = m.read(s.wrapping_add(i));
            let b: u8 = m.read(name.wrapping_add(i));
            if a != b {
                return false;
            }
            if a == 0 {
                return true;
            }
            i += 1;
        }
    }

    /// The last node named `name` in the tree under `root` (itself, then
    /// depth first: child `+8`, sibling `+0`, parent `+4`), or `found`.
    fn find_named<M: Mem>(m: &M, root: u32, name: u32, mut found: u32) -> u32 {
        if node_named(m, root, name) {
            found = root;
        }
        let mut n: u32 = m.read(root + 8);
        if n == 0 {
            return found;
        }
        loop {
            if node_named(m, n, name) {
                found = n;
            }
            let child: u32 = m.read(n + 8);
            let next = if child != 0 {
                child
            } else {
                let sibling: u32 = m.read(n);
                if sibling != 0 {
                    sibling
                } else {
                    if n == root {
                        return found;
                    }
                    // up until a parent with a sibling, or the root
                    loop {
                        n = m.read(n + 4);
                        let s: u32 = m.read(n);
                        if s != 0 {
                            if n == root {
                                return found;
                            }
                            break s;
                        }
                        if n == root {
                            return found;
                        }
                    }
                }
            };
            n = next;
            if n == root {
                return found;
            }
        }
    }

    /// `0x00504a00` (cdecl `(&query, root)`): the node named `query[0]`
    /// in the tree under `root` written to `query[1]` (the last match;
    /// untouched for none). 0.
    pub fn find_node(m: &mut impl Mem, query: u32, root: u32) -> u32 {
        let name: u32 = m.read(query);
        // the original writes each match as it walks; the last one stays
        let found = find_named(m, root, name, 0);
        if found != 0 {
            m.write(query + 4, found);
        }
        0
    }

    /// `0x00506b80` (thiscall `(name)`, `ret 4`): the node named `name`
    /// under `this` ([`find_node`]), or 0 (also for a null or empty
    /// name).
    pub fn find_node_named(m: &impl Mem, this: u32, name: u32) -> u32 {
        if name == 0 {
            return 0;
        }
        let c: u8 = m.read(name);
        if c == 0 {
            return 0;
        }
        find_named(m, this, name, 0)
    }

    /// `0x0055ab30` (thiscall `(node, name)`, `ret 8`): the attachable
    /// component's target: the node named `name` under `node` (or `node`
    /// itself for no name); 0x13 when there is none. A copy of the name
    /// (`PhyreMalloc(len + 1, 2)`; 0xd when that fails) replaces the old
    /// one (`+0x54`, freed unless unowned); then the node's transform
    /// (`node + 4`), the target's (`target + 4`) and the empty name
    /// fields. 0.
    pub fn attach_target(h: &mut impl Host, this: u32, node: u32, name: u32) -> u32 {
        let named = name != 0 && {
            let c: u8 = h.read(name);
            c != 0
        };
        let target = if named {
            find_node_named(h, node, name)
        } else {
            node
        };
        if target == 0 {
            return 0x13;
        }
        let mut copy = 0;
        if name != 0 {
            let mut len = 0u32;
            loop {
                let c: u8 = h.read(name.wrapping_add(len));
                if c == 0 {
                    break;
                }
                len += 1;
            }
            copy = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[len + 1, 2]);
            if copy == 0 {
                return 0xd;
            }
            h.call_sym(MEMCPY, Conv::Cdecl, &[copy, name, len + 1]);
        }
        let lo: u8 = h.read(this + 0x54);
        if lo & 1 == 0 {
            let old: u32 = h.read(this + 0x54);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 0x54, copy);
        let empty = h.rebase(EMPTY_NAME);
        h.write(this + 0x4c, if node == 0 { 0 } else { node + 4 });
        h.write(this + 0x50, empty);
        h.write(this + 0x60, target.wrapping_add(4));
        h.write(this + 0x64, empty);
        h.write(this + 0x5c, u32::MAX);
        0
    }

    /// `0x005d77a0` (thiscall `(v)`, `ret 4`): the fly controller's matrix
    /// (`+0x34`) from its 3x4 (`+4`, through `PMatrix4_FromMatrix3x4` into
    /// the frame, 16 words at `ebp-0x40`), the vector `*v` at `+0x64`, then
    /// `PMatrix4_toWorldMatrix3x4` back into `+4`; its `eax`.
    pub fn fly_set_matrix(h: &mut impl Host, this: u32, v: u32, frame: u32) -> u32 {
        h.call_sym(FROM_MATRIX3X4, Conv::Thiscall(frame), &[this + 4]);
        for i in 0..16 {
            let f: f32 = h.read(frame + 4 * i);
            h.write(this + 0x34 + 4 * i, x87::round_trip(f));
        }
        for i in 0..3 {
            let f: f32 = h.read(v.wrapping_add(4 * i));
            h.write(this + 0x64 + 4 * i, x87::round_trip(f));
        }
        h.call_sym(TO_WORLD_MATRIX, Conv::Thiscall(this + 4), &[this + 0x34])
    }

    /// `0x005168b0` (thiscall, a float in `ST0`): the animation's length,
    /// `end - start` from its time range (written to the frame: `start` at
    /// `ebp-8`, `end` at `ebp-4`), or 0 when the range call fails.
    pub fn time_span(h: &mut impl Host, this: u32, frame: u32) -> f32 {
        let r = h.call_sym(TIME_RANGE, Conv::Thiscall(this), &[frame, frame + 4]);
        if r != 0 {
            return 0.0;
        }
        let d = x87::store(x87::sub(ld(h, frame + 4), ld(h, frame)));
        f32::from_bits(d)
    }
}
