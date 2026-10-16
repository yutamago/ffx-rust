//! Container helpers the game's C++ inlines and the compiler kept as
//! out-of-line copies: MSVC `std::string` / `std::vector` / `std::list`
//! tidying, Phyre's owned arrays (the top bit of the count word marks a
//! borrowed buffer), and a `std::map` head node.
//!
//! The frees are callbacks: `operator delete` (`0x00630390`) and
//! `PhyreFree` (`0x0042fb20`) are ported in [`crate::heap`], and the hook
//! passes those; the tests pass recording stand-ins.

/// `std::string::_Tidy(true)` copies (`thiscall`): `0x00402da0`,
/// `0x00634410`, `0x00634440`.
pub const STRING_TIDY: [u32; 3] = [0x0040_2da0, 0x0063_4410, 0x0063_4440];

/// `std::vector::_Tidy` copies (`thiscall`): `0x00634470`, `0x006344a0`,
/// `0x006344d0`.
pub const VECTOR_TIDY: [u32; 3] = [0x0063_4470, 0x0063_44a0, 0x0063_44d0];

/// Phyre owned-array resets (`thiscall`), (address, whether a null buffer
/// skips the free).
pub const OWNED_RESET: [(u32, bool); 16] = [
    (0x0063_3c40, false),
    (0x0063_3c70, false),
    (0x0063_3ca0, false),
    (0x0063_3cd0, false),
    (0x0063_3d00, false),
    (0x0063_3d30, false),
    (0x0063_3d60, false),
    (0x0063_3d90, true),
    (0x0067_b1d0, false),
    (0x0067_b200, true),
    (0x0068_95c0, true),
    (0x0068_95f0, true),
    (0x006f_bca0, true),
    (0x006f_bcd0, true),
    (0x009e_1d70, false),
    (0x009e_1da0, false),
];

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    /// `std::string::_Tidy(true)` ([`super::STRING_TIDY`]): a heap buffer
    /// (capacity `+0x14` of 16 or more) deleted (its pointer at `+0`); then
    /// capacity 15, size (`+0x10`) 0, and the inline buffer's first byte 0.
    pub fn string_tidy<M: Mem>(mem: &mut M, this: u32, mut delete: impl FnMut(&mut M, u32)) {
        if mem.read::<u32>(this.wrapping_add(0x14)) >= 0x10 {
            let p: u32 = mem.read(this);
            delete(mem, p);
        }
        mem.write(this.wrapping_add(0x14), 0xfu32);
        mem.write(this.wrapping_add(0x10), 0u32);
        mem.write(this, 0u8);
    }

    /// `std::vector::_Tidy` ([`super::VECTOR_TIDY`]): with storage (`+0`),
    /// it deleted and first, last and end (`+0`, `+4`, `+8`) zeroed.
    pub fn vector_tidy<M: Mem>(mem: &mut M, this: u32, delete: impl FnOnce(&mut M, u32)) {
        let p: u32 = mem.read(this);
        if p != 0 {
            delete(mem, p);
            mem.write(this, [0u32; 3]);
        }
    }

    /// A Phyre owned array reset ([`super::OWNED_RESET`]): the buffer (`+4`)
    /// freed unless the count word (`+0`) has its top bit (borrowed) or,
    /// with `null_check`, the buffer is null; then both words zeroed.
    pub fn owned_reset<M: Mem>(
        mem: &mut M,
        this: u32,
        null_check: bool,
        free: impl FnOnce(&mut M, u32),
    ) {
        let count: u32 = mem.read(this);
        let p: u32 = mem.read(this.wrapping_add(4));
        if count >> 31 == 0 && !(null_check && p == 0) {
            free(mem, p);
        }
        mem.write(this.wrapping_add(4), 0u32);
        mem.write(this, 0u32);
    }

    /// `0x0069a130(items, n)`: for a non-null `items`, each of the `n`
    /// eight-byte entries' buffer (`+4`) freed when its flag byte (`+4`'s
    /// low bit, the pointer's own) is clear, then `items` itself.
    pub fn free_owned_array<M: Mem>(
        mem: &mut M,
        items: u32,
        n: u32,
        mut free: impl FnMut(&mut M, u32),
    ) {
        if items == 0 {
            return;
        }
        let mut at = items.wrapping_add(4);
        for _ in 0..n {
            let p: u32 = mem.read(at);
            if p & 1 == 0 {
                free(mem, p);
            }
            at = at.wrapping_add(8);
        }
        free(mem, items);
    }

    /// `0x00686da0()` (`thiscall`): a `std::list<std::string>` cleared: the
    /// head (`*this`) made empty, the size (`+4`) 0, then every former node
    /// (next at `+0`, the string at `+8`) tidied and deleted.
    pub fn string_list_clear<M: Mem>(mem: &mut M, this: u32, mut delete: impl FnMut(&mut M, u32)) {
        let head: u32 = mem.read(this);
        let mut node: u32 = mem.read(head);
        mem.write(head, head);
        let head: u32 = mem.read(this);
        mem.write(head.wrapping_add(4), head);
        mem.write(this.wrapping_add(4), 0u32);
        while node != mem.read::<u32>(this) {
            let next: u32 = mem.read(node);
            string_tidy(mem, node.wrapping_add(8), &mut delete);
            delete(mem, node);
            node = next;
        }
    }

    /// `0x0069a1b0()`: a `std::map` head node: 0x18 bytes from `new`, its
    /// left, parent and right links (`+0`, `+4`, `+8`) to itself, colour
    /// and nil flags (`+0xc`) 1, 1. On a failed allocation it calls
    /// `bad_alloc` (`std::_Xbad_alloc`, which throws).
    pub fn map_head_new<M: Mem>(
        mem: &mut M,
        new: impl FnOnce(&mut M, u32) -> u32,
        bad_alloc: impl FnOnce(&mut M) -> u32,
    ) -> u32 {
        let p = new(mem, 0x18);
        if p == 0 {
            return bad_alloc(mem);
        }
        mem.write(p, [p; 3]);
        mem.write(p.wrapping_add(0xc), 0x101u16);
        p
    }

    /// `0x0061c270()` (`thiscall`): a parser state reset: byte `+0` and the
    /// first bytes of the buffers at `+8`, `+0xc`, `+0x10` zeroed, `+0x14`,
    /// `+0x18` 0; with an owned block (`+0x1c` set) its pointer `+0x20`
    /// deleted; `+0x1c`, `+0x50`, `+0x24` 0.
    pub fn parser_reset<M: Mem>(mem: &mut M, this: u32, delete: impl FnOnce(&mut M, u32)) {
        let p: u32 = mem.read(this.wrapping_add(8));
        mem.write(this, 0u8);
        mem.write(p, 0u8);
        for off in [0xc, 0x10] {
            let p: u32 = mem.read(this.wrapping_add(off));
            mem.write(p, 0u8);
        }
        mem.write(this.wrapping_add(0x14), 0u32);
        mem.write(this.wrapping_add(0x18), 0u32);
        if mem.read::<u32>(this.wrapping_add(0x1c)) != 0 {
            let p: u32 = mem.read(this.wrapping_add(0x20));
            delete(mem, p);
        }
        for off in [0x1c, 0x50, 0x24] {
            mem.write(this.wrapping_add(off), 0u32);
        }
    }

    /// The Phyre array "append into a new buffer" copies (`thiscall`,
    /// `ret 8`; `0x00629b90`, `0x0069b530`, `0x0069b5c0`, `0x0069b650`):
    /// for a null `dst` error 0xd; else the `count` (`+0`, top bit masked)
    /// words of the old buffer (`+4`) copied to `dst` (each to a non-null
    /// address), `*value` stored after them, the old buffer freed when it
    /// is not `dst` and was owned, then `+4` = `dst` and `+0` = `count + 1`
    /// (owned). Returns 0.
    pub fn parray_push_into<M: Mem>(
        mem: &mut M,
        this: u32,
        dst: u32,
        value: u32,
        free: impl FnOnce(&mut M, u32),
    ) -> u32 {
        if dst == 0 {
            return 0xd;
        }
        let count = mem.read::<u32>(this) & 0x7fff_ffff;
        let mut src: u32 = mem.read(this.wrapping_add(4));
        for k in 0..count {
            let d = dst.wrapping_add(4 * k);
            if d != 0 {
                let w: u32 = mem.read(src);
                mem.write(d, w);
                src = src.wrapping_add(4);
            }
        }
        let slot = dst.wrapping_add(4 * count);
        if slot != 0 {
            let w: u32 = mem.read(value);
            mem.write(slot, w);
        }
        let old: u32 = mem.read(this.wrapping_add(4));
        if old != dst && mem.read::<u32>(this) >> 31 == 0 {
            free(mem, old);
        }
        mem.write(this.wrapping_add(4), dst);
        mem.write(this, count + 1);
        0
    }

    /// `0x00689620()` (`thiscall`, a destructor): the owned arrays at
    /// `+0x158` then `+0x150` reset ([`owned_reset`] with the null check).
    pub fn reset_arrays_689620<M: Mem>(mem: &mut M, this: u32, mut free: impl FnMut(&mut M, u32)) {
        owned_reset(mem, this.wrapping_add(0x158), true, &mut free);
        owned_reset(mem, this.wrapping_add(0x150), true, free);
    }

    /// `0x009e2420()` (`thiscall`): a Phyre array of objects destroyed:
    /// each non-null element deleted through its vtable's first entry
    /// (`delete_object(obj)`, the scalar deleting destructor with 1; the
    /// count is re-read after each), then, for a non-zero count word, the
    /// owned non-null buffer freed and both words zeroed, and once more
    /// the (now zero) buffer freed unless borrowed and both words zeroed.
    pub fn parray_delete_all<M: Mem>(
        mem: &mut M,
        this: u32,
        mut delete_object: impl FnMut(&mut M, u32),
        mut free: impl FnMut(&mut M, u32),
    ) {
        let mut i = 0u32;
        while i < mem.read::<u32>(this) & 0x7fff_ffff {
            let data: u32 = mem.read(this.wrapping_add(4));
            let obj: u32 = mem.read(data.wrapping_add(4 * i));
            if obj != 0 {
                delete_object(mem, obj);
            }
            i += 1;
        }
        if mem.read::<u32>(this) != 0 {
            owned_reset(mem, this, true, &mut free);
        }
        owned_reset(mem, this, false, free);
    }
}
