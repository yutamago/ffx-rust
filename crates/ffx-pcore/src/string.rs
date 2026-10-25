//! `PString`, `PStringBuilder` and `PhyreEngine`'s aligned allocator.
//!
//! `PString`: `+0` the text; bit 0 of the pointer set means the text is
//! not the string's own (never freed). `PStringBuilder`: `+0` capacity
//! (top bit: borrowed storage), `+4` the buffer, `+8` the length with the
//! terminator (0 when empty).
//!
//! The aligned allocator puts the block 16-byte aligned past a header and
//! keeps the raw pointer in the word before it (`(p - 7) +
//! ((7 - p) & 3)`, the originals' formula, which is `p - 4` for an aligned
//! `p`).

/// `user_malloc` (cdecl `(size)`).
pub const USER_MALLOC: u32 = 0x0068_7020;
/// `user_realloc` (cdecl `(p, size)`).
pub const USER_REALLOC: u32 = 0x0068_7060;
/// `PStringBuilder`'s grow (thiscall `(builder, capacity)`).
pub const BUILDER_GROW: u32 = 0x0044_4e90;
/// `PStringBuilder::operator+=`'s text for a null string (12 bytes).
pub const NULL_TEXT: u32 = 0x00b0_f608;
/// `MSVCR!memchr`'s import slot (cdecl).
pub const MEMCHR: u32 = 0x00b0_c390;

/// Where the aligned allocator keeps the raw pointer for block `p`.
#[must_use]
pub const fn header(p: u32) -> u32 {
    let at = p.wrapping_sub(7);
    at.wrapping_add(at.wrapping_neg() & 3)
}

/// The aligned block for raw memory `raw` (16 bytes up from `raw + 4`).
#[must_use]
pub const fn aligned(raw: u32) -> u32 {
    let p = raw.wrapping_add(4);
    p.wrapping_add(p.wrapping_neg() & 0xf)
}

pub mod bridge {
    //! The originals over game memory.

    use ffx_graphic::host::{Conv, Host};

    use super::{BUILDER_GROW, MEMCHR, NULL_TEXT, USER_MALLOC, USER_REALLOC, aligned, header};
    use crate::{memcpy, phyre_free, phyre_malloc, strlen};

    /// A copy of the C string `s` (`PhyreMalloc(len + 1, 2)`), or 0 when
    /// that fails.
    fn copy(h: &mut impl Host, s: u32) -> u32 {
        let n = strlen(h, s).wrapping_add(1);
        let p = phyre_malloc(h, n, 2);
        if p != 0 {
            memcpy(h, p, s, n);
        }
        p
    }

    /// `PString::PString(this, s)`: a copy of `s` (null for null); `this`.
    pub fn new(h: &mut impl Host, this: u32, s: u32) -> u32 {
        let p = if s == 0 { 0 } else { copy(h, s) };
        h.write(this, p);
        this
    }

    /// Frees the string's own text.
    fn drop_text(h: &mut impl Host, this: u32) {
        let old: u32 = h.read(this);
        if old & 1 == 0 {
            phyre_free(h, old);
        }
    }

    /// `PString::Assign(this, s)`: a copy of `s` replaces the text (freed
    /// when owned). 0, or `0xd` when the copy fails (the string kept).
    pub fn assign(h: &mut impl Host, this: u32, s: u32) -> u32 {
        let p = if s == 0 {
            0
        } else {
            let p = copy(h, s);
            if p == 0 {
                return 0xd;
            }
            p
        };
        drop_text(h, this);
        h.write(this, p);
        0
    }

    /// `0x00445d90(this, offset, unused)`: the text becomes `offset + 1`
    /// (a fixed-up pointer; 0 stays 0), the old one freed when owned. 0,
    /// or 5 for an odd `offset`.
    pub fn fixup(h: &mut impl Host, this: u32, offset: u32) -> u32 {
        if offset & 1 != 0 {
            return 5;
        }
        let p = if offset == 0 { 0 } else { offset + 1 };
        drop_text(h, this);
        h.write(this, p);
        0
    }

    /// `0x00444ce0(builder, s, n)`: appends `n` bytes of `s` (growing the
    /// buffer to `len + n + 0x80` when needed) and a terminator, then cuts
    /// the length at the first zero byte of what was appended. Appending
    /// nothing to a non-empty builder does nothing. The result is
    /// `memchr`'s pointer when the length stands, else the new length
    /// (`eax`); `None` for the early return (the caller's `eax`).
    pub fn append(h: &mut impl Host, this: u32, s: u32, n: u32) -> Option<u32> {
        let len: u32 = h.read(this + 8);
        if n == 0 && len != 0 {
            return None;
        }
        let empty = u32::from(len == 0);
        let need = len.wrapping_add(empty).wrapping_add(n);
        if need > h.read::<u32>(this) & 0x7fff_ffff {
            let cap = n.wrapping_add(0x80).wrapping_add(len);
            h.call_sym(BUILDER_GROW, Conv::Thiscall(this), &[cap]);
        }
        let buf: u32 = h.read(this + 4);
        let len: u32 = h.read(this + 8);
        let dst = buf.wrapping_add(len).wrapping_add(empty).wrapping_sub(1);
        memcpy(h, dst, s, n);
        let len = h.read::<u32>(this + 8).wrapping_add(empty.wrapping_add(n));
        h.write(this + 8, len);
        let end = buf.wrapping_sub(1).wrapping_add(len);
        h.write(end, 0u8);
        let f: u32 = h.read(h.rebase(MEMCHR));
        let r = h.call(f, Conv::Cdecl, &[dst, 0, n.wrapping_add(1)]);
        if r == end {
            return Some(r);
        }
        let len = r.wrapping_sub(buf).wrapping_add(1);
        h.write(this + 8, len);
        Some(len)
    }

    /// `PStringBuilder::operator+=(builder, s)`: [`append`] of the C
    /// string `s`, or of the 12-byte text at `0x00b0f608` for null.
    pub fn append_str(h: &mut impl Host, this: u32, s: u32) -> Option<u32> {
        if s == 0 {
            let t = h.rebase(NULL_TEXT);
            return append(h, this, t, 0xc);
        }
        let n = strlen(h, s);
        append(h, this, s, n)
    }

    /// `0x00444c50(n, src, count)`: `PhyreMalloc(n, 1)` filled with
    /// `count` bytes of `src`; the block or 0.
    pub fn alloc_copy(h: &mut impl Host, n: u32, src: u32, count: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        let p = phyre_malloc(h, n, 1);
        if p == 0 {
            return 0;
        }
        let mut src = src;
        for i in 0..count {
            let d = p.wrapping_add(i);
            if d != 0 {
                let b: u8 = h.read(src);
                h.write(d, b);
                src = src.wrapping_add(1);
            }
        }
        p
    }

    /// Places raw memory `raw` as an aligned block; the block, or 0.
    fn place(h: &mut impl Host, raw: u32) -> u32 {
        if raw == 0 {
            return 0;
        }
        let p = aligned(raw);
        h.write(header(p), raw);
        p
    }

    /// `0x0042fae0(n, size)`: an aligned block of `n * size` bytes; 0
    /// when out of memory.
    pub fn calloc_aligned(h: &mut impl Host, n: u32, size: u32) -> u32 {
        let bytes = n.wrapping_mul(size).wrapping_add(0x14);
        let raw = h.call_sym(USER_MALLOC, Conv::Cdecl, &[bytes]);
        place(h, raw)
    }

    /// `0x0042fbd0(p, size)`: `p`'s block resized (`user_realloc` of its
    /// raw memory) or a new one for null; the block, or 0.
    pub fn realloc_aligned(h: &mut impl Host, p: u32, size: u32) -> u32 {
        let bytes = size.wrapping_add(0x14);
        let raw = if p == 0 {
            h.call_sym(USER_MALLOC, Conv::Cdecl, &[bytes])
        } else {
            let old: u32 = h.read(header(p));
            h.call_sym(USER_REALLOC, Conv::Cdecl, &[old, bytes])
        };
        place(h, raw)
    }
}
