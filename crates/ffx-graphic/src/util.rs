//! Small helpers of the render layer that decide results game code reads:
//! a clip-space outcode, `std` container steps the compiler instanced for
//! its maps and heaps, and copies.

/// The originals, over game memory.
#[allow(clippy::many_single_char_names)] // vector components, string bytes
pub mod bridge {
    use crate::host::{Conv, Host};
    use ffx_bridge::{Global, Mem, x87};

    /// The map id `graphicIsMapInList` (`0x00642610`) tests.
    pub const MAP_ID: Global<u32> = Global::new(0x0133_0844);

    /// The maps `graphicIsMapInList` lists.
    pub const LISTED_MAPS: [u32; 16] = [
        0x118, 0x52, 0x51, 0xde, 0x13c, 0x17d, 0x5d, 0x12, 0x15d, 0xf8, 0x99, 0xb3, 0x50, 0x137,
        0x16b, 0x84,
    ];

    /// `ClipOutcode(v)` (`0x00652470`): the outcode of a homogeneous
    /// point `(x, y, z, w)`: bits 0-2 for `x`, `y`, `z` above `w`, bits 3-5
    /// for them below `-w` (each difference or sum stored as a float
    /// first; a NaN sets nothing).
    pub fn clip_outcode(m: &impl Mem, v: u32) -> u32 {
        let f = |i: u32| x87::load(x87::store(x87::load(m.read(v + 4 * i))));
        let (x, y, z, w) = (f(0), f(1), f(2), f(3));
        let st = |a: f64| x87::load(x87::store(a));
        let above = [st(x87::sub(x, w)), st(x87::sub(y, w)), st(x87::sub(z, w))];
        let below = [st(x87::add(w, x)), st(x87::add(w, y)), st(x87::add(w, z))];
        let mut code = 0;
        for (i, d) in (0u32..).zip(above) {
            if d > 0.0 {
                code |= 1 << i;
            }
        }
        for (i, s) in (3u32..).zip(below) {
            if s < 0.0 {
                code |= 1 << i;
            }
        }
        code
    }

    /// `graphicIsMapInList()` (`0x00642610`): 1 when the current map
    /// (`0x01330844`) is one of [`LISTED_MAPS`], else 0.
    pub fn is_map_in_list(m: &impl Mem) -> u32 {
        let id = MAP_ID.get(m);
        u32::from(LISTED_MAPS.contains(&id))
    }

    /// `Vec3_UninitializedCopy(first, last, dest)` (`0x006b4f80`): copies
    /// the 12-byte elements `[first, last)` to `dest` (a null `dest`
    /// copies nothing but still advances). Returns the end of `dest`.
    pub fn vec3_uninitialized_copy(m: &mut impl Mem, first: u32, last: u32, dest: u32) -> u32 {
        let (mut from, mut to) = (first, dest);
        while from != last {
            if to != 0 {
                for k in 0..3 {
                    let w: u32 = m.read(from + 4 * k);
                    m.write(to + 4 * k, w);
                }
            }
            from = from.wrapping_add(12);
            to = to.wrapping_add(12);
        }
        to
    }

    /// `ShadowManager_GetCascadeMatrix(this, out, i)` (`0x006d23a0`): the
    /// 4x4 matrix at `this +0x210 + 64 i` to `out`. Returns `out`.
    pub fn shadow_get_cascade_matrix(m: &mut impl Mem, this: u32, out: u32, i: u32) -> u32 {
        let from = this.wrapping_add(0x210).wrapping_add(i << 6);
        for k in 0..16 {
            let w: u32 = m.read(from + 4 * k);
            m.write(out + 4 * k, w);
        }
        out
    }

    /// `FloatHeap_AdjustHeap(base, hole, len, &value, less)` (`0x006c7250`):
    /// `std::_Adjust_heap` on floats with a `bool less(a, b)` (cdecl):
    /// sifts the hole down to a leaf, then pushes `value` up from there.
    /// Returns `&value`.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)] // signed indices
    pub fn float_adjust_heap(
        h: &mut impl Host,
        base: u32,
        hole: u32,
        len: u32,
        value: u32,
        less: u32,
    ) -> u32 {
        let at = |i: i32| base.wrapping_add((i as u32).wrapping_mul(4));
        let top = hole as i32;
        let len = len as i32;
        let mut hole = top;
        let mut child = hole.wrapping_mul(2).wrapping_add(2);
        while child < len {
            let a = rt(h.read(at(child)));
            let b = rt(h.read(at(child - 1)));
            if less_than(h, less, a, b) {
                child -= 1;
            }
            let v = rt(h.read(at(child)));
            h.write(at(hole), v);
            hole = child;
            child = child.wrapping_mul(2).wrapping_add(2);
        }
        if child == len {
            let v = rt(h.read(at(len - 1)));
            h.write(at(hole), v);
            hole = len - 1;
        }
        let mut parent = (hole - 1) / 2;
        while top < hole {
            let a = rt(h.read(at(parent)));
            let b = rt(h.read(value));
            if !less_than(h, less, a, b) {
                break;
            }
            let v = rt(h.read(at(parent)));
            h.write(at(hole), v);
            hole = parent;
            parent = (hole - 1) / 2;
        }
        let v = rt(h.read(value));
        h.write(at(hole), v);
        value
    }

    /// The heap's `bool less(a, b)`: its `al`.
    fn less_than(h: &mut impl Host, less: u32, a: u32, b: u32) -> bool {
        h.call(less, Conv::Cdecl, &[a, b]).to_le_bytes()[0] != 0
    }

    /// A float as `fld`/`fstp` moves it.
    fn rt(bits: u32) -> u32 {
        x87::round_trip(f32::from_bits(bits)).to_bits()
    }

    /// Whether the `std::map` node is the head (its `+0xd` byte).
    fn is_nil(m: &impl Mem, node: u32) -> bool {
        m.read::<u8>(node + 0xd) != 0
    }

    /// `MapIterator_Decrement(it)` (`0x0064f360`, fastcall): `--it` of a
    /// `std::map` iterator (nodes `+0` left, `+4` parent, `+8` right):
    /// from the head to the last node, else the left subtree's last node,
    /// else up past the parents it is the left child of. Returns `it`.
    pub fn map_iterator_decrement(m: &mut impl Mem, it: u32) -> u32 {
        let node: u32 = m.read(it);
        if is_nil(m, node) {
            let last: u32 = m.read(node + 8);
            m.write(it, last);
            return it;
        }
        let left: u32 = m.read(node);
        if !is_nil(m, left) {
            let mut n = left;
            loop {
                let r: u32 = m.read(n + 8);
                if is_nil(m, r) {
                    break;
                }
                n = r;
            }
            m.write(it, n);
            return it;
        }
        let mut p: u32 = m.read(node + 4);
        while !is_nil(m, p) && m.read::<u32>(it) == m.read::<u32>(p) {
            m.write(it, p);
            p = m.read(p + 4);
        }
        let cur: u32 = m.read(it);
        if !is_nil(m, cur) {
            m.write(it, p);
        }
        it
    }

    /// A `PString`'s text: the pointer with its flag bit 0 cleared.
    fn pstring_text(m: &impl Mem, at: u32) -> u32 {
        let p: u32 = m.read(at);
        p.wrapping_sub(p & 1)
    }

    /// `a < b` for two `PString` texts: by pointer when either is null,
    /// else as `strcmp`.
    fn pstring_less(m: &impl Mem, a: u32, b: u32) -> bool {
        if a == 0 || b == 0 {
            return a < b;
        }
        let mut i = 0u32;
        loop {
            let (x, y) = (m.read::<u8>(a + i), m.read::<u8>(b + i));
            if x != y {
                return x < y;
            }
            if x == 0 {
                return false;
            }
            i += 1;
        }
    }

    /// `PStringTree_Find(this, &key)` (`0x006be750`): a search tree under
    /// the head `this` (root `+0xc`; nodes `+0` lower, `+4` higher, `+0xc`
    /// a `PString` key) by text. The node, else 0.
    pub fn pstring_tree_find(m: &impl Mem, this: u32, key: u32) -> u32 {
        let k = pstring_text(m, key);
        let mut node: u32 = m.read(this + 0xc);
        while node != this {
            let nk = pstring_text(m, node + 0xc);
            node = if pstring_less(m, k, nk) {
                m.read(node)
            } else if pstring_less(m, nk, k) {
                m.read(node + 4)
            } else {
                return node;
            };
        }
        0
    }
}
