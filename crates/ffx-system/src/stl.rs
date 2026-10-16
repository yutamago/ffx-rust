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
}
