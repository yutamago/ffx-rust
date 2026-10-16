//! The C++ standard library code the compiler instantiated into `FFX.exe`
//! for the system layer's containers (MSVC 2012's `std::map` tree).
//!
//! A tree node: left at `+0`, parent at `+4`, right at `+8`, `_Isnil` (the
//! head sentinel) at `+0xd`.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    fn left(mem: &impl Mem, n: u32) -> u32 {
        mem.read(n)
    }

    fn parent(mem: &impl Mem, n: u32) -> u32 {
        mem.read(n.wrapping_add(4))
    }

    fn right(mem: &impl Mem, n: u32) -> u32 {
        mem.read(n.wrapping_add(8))
    }

    fn is_nil(mem: &impl Mem, n: u32) -> bool {
        mem.read::<u8>(n.wrapping_add(0xd)) != 0
    }

    /// `std::_Tree_const_iterator::operator++` (fastcall, the iterator in
    /// ECX; compiled four times at `0x006352b0`, `0x00635300`, `0x00635350`,
    /// `0x00699ee0`): the in-order successor, the leftmost node of the right
    /// subtree, else the first ancestor reached from a left child; the head
    /// stays. One store of the node when it moves; returns the iterator.
    pub fn tree_iterator_inc(mem: &mut impl Mem, it: u32) -> u32 {
        let node: u32 = mem.read(it);
        if is_nil(mem, node) {
            return it;
        }
        let r = right(mem, node);
        if !is_nil(mem, r) {
            let mut n = r;
            while !is_nil(mem, left(mem, n)) {
                n = left(mem, n);
            }
            mem.write(it, n);
            return it;
        }
        let mut p = parent(mem, node);
        while !is_nil(mem, p) {
            let cur: u32 = mem.read(it);
            if cur != right(mem, p) {
                break;
            }
            mem.write(it, p);
            p = parent(mem, p);
        }
        mem.write(it, p);
        it
    }

    /// `std::_Uninitialized_copy` of 12-byte elements (`0x0059e830`,
    /// cdecl `(first, last, dest)`): each element's three words copied
    /// (each destination tested for null: a null `dest` skips only the
    /// first, as `new (p) T` does); returns the end of
    /// `dest`.
    pub fn uninitialized_copy12(mem: &mut impl Mem, first: u32, last: u32, dest: u32) -> u32 {
        let (mut s, mut d) = (first, dest);
        while s != last {
            if d != 0 {
                for k in 0..3 {
                    let w: u32 = mem.read(s + 4 * k);
                    mem.write(d + 4 * k, w);
                }
            }
            s = s.wrapping_add(12);
            d = d.wrapping_add(12);
        }
        d
    }

    /// `memcmp` as compiled into the game (`0x00401020`, cdecl `(a, b,
    /// n)`): words while four bytes remain, then bytes from the first
    /// different word or the tail; -1, 0 or 1 (unsigned bytes).
    pub fn memcmp(mem: &impl Mem, left: u32, right: u32, len: u32) -> i32 {
        if len == 0 {
            return 0;
        }
        let (mut lp, mut rp) = (left, right);
        let mut rest = len;
        while rest >= 4 {
            if mem.read::<u32>(lp) != mem.read::<u32>(rp) {
                break;
            }
            lp = lp.wrapping_add(4);
            rp = rp.wrapping_add(4);
            rest -= 4;
        }
        for k in 0..rest.min(4) {
            let (lb, rb): (u8, u8) = (mem.read(lp + k), mem.read(rp + k));
            if lb != rb {
                return if lb < rb { -1 } else { 1 };
            }
        }
        0
    }

    /// `0x00402a00` (`thiscall(this, i, v)`, `ret 8`): word `i` of
    /// `this` set; returns 0.
    pub fn set_word(mem: &mut impl Mem, this: u32, i: u32, v: u32) -> u32 {
        mem.write(this.wrapping_add(i.wrapping_mul(4)), v);
        0
    }

    /// A `std::map<u32, ...>` lookup (`0x0062fe50`, `thiscall(head, &key)`,
    /// `ret 4`): from the root (`head + 0xc`), key at `+0xc`, children at
    /// `+0` (smaller) and `+4`; the node with the key, or 0 when the walk
    /// reaches the head.
    pub fn map_find_u32(mem: &impl Mem, head: u32, key: u32) -> u32 {
        let k: u32 = mem.read(key);
        let mut n: u32 = mem.read(head + 0xc);
        while n != head {
            let v: u32 = mem.read(n + 0xc);
            if v == k {
                return n;
            }
            n = mem.read(n + if v > k { 0 } else { 4 });
        }
        0
    }

    /// `std::map<u32, ...>::equal_range` (`0x0069a250`, `thiscall(this,
    /// out, &key)`, `ret 8`): nodes keep the key at `+0x10` and the nil
    /// flag at `+0xd`, children at `+0`/`+8`, the head at `*this` (root
    /// `+4`). Writes the lower bound and the upper bound to `out`;
    /// returns `out`.
    pub fn equal_range_u32(mem: &mut impl Mem, this: u32, out: u32, key: u32) -> u32 {
        let head: u32 = mem.read(this);
        let root: u32 = mem.read(head.wrapping_add(4));
        let k: u32 = mem.read(key);
        let (mut lower, mut upper) = (head, head);
        let mut n = root;
        while mem.read::<u8>(n.wrapping_add(0xd)) == 0 {
            let v: u32 = mem.read(n.wrapping_add(0x10));
            if v < k {
                n = mem.read(n.wrapping_add(8));
            } else {
                if mem.read::<u8>(upper.wrapping_add(0xd)) != 0 && k < v {
                    upper = n;
                }
                lower = n;
                n = mem.read(n);
            }
        }
        let mut n = if mem.read::<u8>(upper.wrapping_add(0xd)) == 0 {
            mem.read(upper)
        } else {
            root
        };
        while mem.read::<u8>(n.wrapping_add(0xd)) == 0 {
            if k < mem.read::<u32>(n.wrapping_add(0x10)) {
                upper = n;
                n = mem.read(n);
            } else {
                n = mem.read(n.wrapping_add(8));
            }
        }
        mem.write(out, lower);
        mem.write(out.wrapping_add(4), upper);
        out
    }

    /// Whether key `a` sorts before key `b` in the string maps: both
    /// strings, by unsigned bytes; when either is null, by address.
    fn cstr_less(mem: &impl Mem, left: u32, right: u32) -> bool {
        if left == 0 || right == 0 {
            return left < right;
        }
        let mut k = 0u32;
        loop {
            let (lb, rb): (u8, u8) = (
                mem.read(left.wrapping_add(k)),
                mem.read(right.wrapping_add(k)),
            );
            if lb != rb {
                return lb < rb;
            }
            if lb == 0 {
                return false;
            }
            k += 1;
        }
    }

    /// A string-keyed map lookup (`0x006dc010`, `thiscall(head, &key)`,
    /// `ret 4`): keys are tagged pointers (bit 0 stripped) at `+0xc`,
    /// children at `+0` (smaller) and `+4`, the root at `head + 0xc` and the
    /// leaves pointing back at `head`. The node with an equal key, or 0.
    pub fn map_find_cstr(mem: &impl Mem, head: u32, key: u32) -> u32 {
        let untag = |p: u32| p.wrapping_sub(p & 1);
        let k = untag(mem.read(key));
        let mut n: u32 = mem.read(head.wrapping_add(0xc));
        while n != head {
            let v = untag(mem.read(n.wrapping_add(0xc)));
            if cstr_less(mem, k, v) {
                n = mem.read(n);
            } else if cstr_less(mem, v, k) {
                n = mem.read(n.wrapping_add(4));
            } else {
                return n;
            }
        }
        0
    }
}
