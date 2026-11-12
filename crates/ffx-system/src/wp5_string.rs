//! M5 finish, WP5 part 2: MSVC 2012's `std::basic_string<char>` as
//! compiled into `FFX.exe` (`0x00402e20`..`0x006590d0`), which the ports
//! called in Unicorn (about 1.2 M emulated blocks in gate run 1, most of
//! them `assign(str, pos, n)` and the buffer growth).
//!
//! A string: 16 bytes of buffer at `+0` (or, from capacity 16 on, the heap
//! pointer there), the size at `+0x10`, the capacity at `+0x14`; the
//! characters are followed by a 0. Every function is thiscall. The checks
//! call `MSVCP110`'s `std::_Xout_of_range("invalid string position")` and
//! `std::_Xlength_error("string too long")` through `FFX.exe`'s import
//! table, as the original does; they throw, so the port returns after
//! them. `memcpy`/`memmove`/`memchr` (calls in the original) are copies
//! here; `operator new`/`operator delete` stay calls to the game's
//! (`Host::call_sym`).

/// `operator new(n)` / `operator delete(p)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// The import slots of `MSVCP110`'s `std::_Xout_of_range`,
/// `std::_Xlength_error` and `std::_Xbad_alloc` (cdecl; they throw).
pub const XOUT_OF_RANGE_IAT: u32 = 0x00b0_c25c;
pub const XLENGTH_IAT: u32 = 0x00b0_c29c;
pub const XBAD_ALLOC_IAT: u32 = 0x00b0_c2a0;
/// `"invalid string position"` and `"string too long"`.
pub const INVALID_POSITION: u32 = 0x00b7_cae4;
pub const TOO_LONG: u32 = 0x00b7_cafc;

/// The largest size (`max_size()`).
pub const MAX_SIZE: u32 = 0xffff_fffe;

pub mod bridge {
    //! The ports over game memory.
    #![allow(clippy::many_single_char_names)] // the standard's names: s, n, p

    use ffx_graphic::host::{Conv, Host};

    use super::{
        INVALID_POSITION, MAX_SIZE, OPERATOR_DELETE, OPERATOR_NEW, TOO_LONG, XBAD_ALLOC_IAT,
        XLENGTH_IAT, XOUT_OF_RANGE_IAT,
    };
    use crate::stl::bridge::memcmp;

    fn size(h: &impl Host, s: u32) -> u32 {
        h.read(s + 0x10)
    }

    fn cap(h: &impl Host, s: u32) -> u32 {
        h.read(s + 0x14)
    }

    /// The characters: the heap pointer from capacity 16 on, else the
    /// string itself.
    fn data(h: &impl Host, s: u32) -> u32 {
        if cap(h, s) >= 16 { h.read(s) } else { s }
    }

    /// Calls the import in `slot` with the message at `msg`.
    fn throw(h: &mut impl Host, slot: u32, msg: Option<u32>) {
        let f: u32 = h.read(h.rebase(slot));
        match msg {
            Some(m) => {
                let m = h.rebase(m);
                h.call(f, Conv::Cdecl, &[m])
            }
            None => h.call(f, Conv::Cdecl, &[]),
        };
    }

    fn out_of_range(h: &mut impl Host) {
        throw(h, XOUT_OF_RANGE_IAT, Some(INVALID_POSITION));
    }

    fn too_long(h: &mut impl Host) {
        throw(h, XLENGTH_IAT, Some(TOO_LONG));
    }

    /// `n` bytes from `src` to `dst` (overlap allowed, as `memmove`).
    fn move_bytes(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        if n == 0 {
            return;
        }
        let mut buf = vec![0u8; n as usize];
        h.read_bytes(src, &mut buf);
        h.write_bytes(dst, &buf);
    }

    /// The size set to `n` and the 0 after it.
    fn set_size(h: &mut impl Host, s: u32, n: u32) {
        h.write(s + 0x10, n);
        let d = data(h, s);
        h.write(d.wrapping_add(n), 0u8);
    }

    /// `0x00403d80` and `0x004045c0` (`begin`/`c_str`, no stack words):
    /// the characters' address.
    pub fn begin(h: &impl Host, this: u32) -> u32 {
        data(h, this)
    }

    /// `std::basic_string::_Copy(new_size, old_size)` (`0x00404740`, `ret
    /// 8`): a buffer of `new_size | 15` (but at least 1.5 times the old
    /// capacity, capped at `max_size`; `new_size` itself past `max_size`)
    /// plus one from `operator new`, the first `old_size` characters
    /// copied into it, an old heap buffer freed; size `old_size`. The new
    /// buffer.
    pub fn grow_copy(h: &mut impl Host, this: u32, new_size: u32, old_size: u32) -> u32 {
        let mut res = new_size | 0xf;
        if res > MAX_SIZE {
            res = new_size;
        } else {
            let old = cap(h, this);
            if old / 2 > res / 3 {
                res = if old <= MAX_SIZE - old / 2 {
                    old / 2 + old
                } else {
                    MAX_SIZE
                };
            }
        }
        let n = res.wrapping_add(1);
        let mut ptr = 0;
        if n != 0 {
            ptr = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[n]);
            if ptr == 0 {
                throw(h, XBAD_ALLOC_IAT, None);
                return 0;
            }
        }
        if old_size != 0 {
            let from = data(h, this);
            move_bytes(h, ptr, from, old_size);
        }
        if cap(h, this) >= 16 {
            let old: u32 = h.read(this);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[old]);
        }
        h.write(this, ptr);
        h.write(this + 0x14, res);
        set_size(h, this, old_size);
        ptr
    }

    /// `std::basic_string::erase(off, count)` (`0x004043b0`, `ret 8`): the
    /// characters from `off` on (`count` of them or to the end) removed.
    /// `this`.
    pub fn erase(h: &mut impl Host, this: u32, off: u32, count: u32) -> u32 {
        let size = size(h, this);
        if size < off {
            out_of_range(h);
            return this;
        }
        if size - off <= count {
            set_size(h, this, off);
        } else if count != 0 {
            let at = data(h, this).wrapping_add(off);
            let new_size = size - count;
            move_bytes(h, at, at.wrapping_add(count), new_size - off);
            set_size(h, this, new_size);
        }
        this
    }

    /// `0x00404370` (`ret 4`): the string cut to `n` characters (not past
    /// its size). `this`.
    pub fn cut(h: &mut impl Host, this: u32, n: u32) -> u32 {
        if size(h, this) < n {
            out_of_range(h);
            return this;
        }
        set_size(h, this, n);
        this
    }

    /// Room for `n` characters (after the checks of the callers): grown
    /// when the capacity is below. Whether there are characters to copy.
    fn room(h: &mut impl Host, this: u32, n: u32, keep: u32) -> bool {
        if cap(h, this) < n {
            grow_copy(h, this, n, keep);
            n != 0
        } else if n == 0 {
            set_size(h, this, 0);
            false
        } else {
            true
        }
    }

    /// `std::basic_string::assign(str, pos, n)` (`0x00403a10`, `ret 0xc`):
    /// the string becomes up to `n` characters of `str` from `pos` (itself:
    /// cut to the end, then the front erased). `this`.
    pub fn assign_substr(h: &mut impl Host, this: u32, src: u32, pos: u32, n: u32) -> u32 {
        let src_size = size(h, src);
        if src_size < pos {
            out_of_range(h);
            return this;
        }
        let num = n.min(src_size - pos);
        if this == src {
            let end = num.wrapping_add(pos);
            if size(h, this) < end {
                out_of_range(h);
                return this;
            }
            set_size(h, this, end);
            erase(h, this, 0, pos);
            return this;
        }
        if num > MAX_SIZE {
            too_long(h);
            return this;
        }
        let keep = size(h, this);
        if room(h, this, num, keep) {
            let from = data(h, src).wrapping_add(pos);
            let to = data(h, this);
            move_bytes(h, to, from, num);
            set_size(h, this, num);
        }
        this
    }

    /// Whether `p` points into the string's characters.
    fn inside(h: &impl Host, this: u32, p: u32) -> bool {
        let d = data(h, this);
        p != 0 && p >= d && d.wrapping_add(size(h, this)) > p
    }

    /// `std::basic_string::assign(p, n)` (`0x00404270`, `ret 8`): the
    /// string becomes the `n` characters at `p` (characters of its own:
    /// [`assign_substr`] of itself). `this`.
    pub fn assign(h: &mut impl Host, this: u32, p: u32, n: u32) -> u32 {
        if inside(h, this, p) {
            let off = p.wrapping_sub(data(h, this));
            return assign_substr(h, this, this, off, n);
        }
        if n > MAX_SIZE {
            too_long(h);
            return this;
        }
        let keep = size(h, this);
        if room(h, this, n, keep) {
            let to = data(h, this);
            move_bytes(h, to, p, n);
            set_size(h, this, n);
        }
        this
    }

    /// The length of the C string at `p`.
    fn strlen(h: &impl Host, p: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(p.wrapping_add(n)) != 0 {
            n += 1;
        }
        n
    }

    /// `std::basic_string::assign(const char *)` (`0x00403b30`, `ret 4`).
    pub fn assign_cstr(h: &mut impl Host, this: u32, p: u32) -> u32 {
        let n = strlen(h, p);
        assign(h, this, p, n)
    }

    /// The `n` characters at `from` appended (sizes checked by the
    /// callers' rules). `this`.
    fn append_at(
        h: &mut impl Host,
        this: u32,
        from_string: Option<(u32, u32)>,
        p: u32,
        n: u32,
    ) -> u32 {
        let size = size(h, this);
        if u32::MAX - size <= n {
            too_long(h);
            return this;
        }
        if n == 0 {
            return this;
        }
        let new_size = size + n;
        if new_size > MAX_SIZE {
            too_long(h);
            return this;
        }
        if room(h, this, new_size, size) {
            // the source's characters after the growth (it may be this one)
            let from = match from_string {
                Some((src, pos)) => data(h, src).wrapping_add(pos),
                None => p,
            };
            let to = data(h, this).wrapping_add(size);
            move_bytes(h, to, from, n);
            set_size(h, this, new_size);
        }
        this
    }

    /// `std::basic_string::append(str, pos, n)` (`0x006575a0`, `ret 0xc`).
    /// `this`.
    pub fn append_substr(h: &mut impl Host, this: u32, src: u32, pos: u32, n: u32) -> u32 {
        let src_size = size(h, src);
        if src_size < pos {
            out_of_range(h);
            return this;
        }
        let num = n.min(src_size - pos);
        append_at(h, this, Some((src, pos)), 0, num)
    }

    /// `std::basic_string::append(p, n)` (`0x006576d0`, `ret 8`; its own
    /// characters: [`append_substr`] of itself). `this`.
    pub fn append(h: &mut impl Host, this: u32, p: u32, n: u32) -> u32 {
        if inside(h, this, p) {
            let off = p.wrapping_sub(data(h, this));
            return append_substr(h, this, this, off, n);
        }
        append_at(h, this, None, p, n)
    }

    /// `std::basic_string::append(const char *)` (`0x00657690`, `ret 4`).
    pub fn append_cstr(h: &mut impl Host, this: u32, p: u32) -> u32 {
        let n = strlen(h, p);
        append(h, this, p, n)
    }

    /// `std::basic_string::substr(pos, n)` (`0x00402e20`, `ret 0xc`, the
    /// result in `out`): `out` made empty, then [`assign_substr`]. `out`.
    pub fn substr(h: &mut impl Host, this: u32, out: u32, pos: u32, n: u32) -> u32 {
        h.write(out + 0x14, 0xfu32);
        h.write(out + 0x10, 0u32);
        h.write(out, 0u8);
        assign_substr(h, out, this, pos, n);
        out
    }

    /// The string's move (`0x004039b0`, `ret 4`): a short `from` copied
    /// (its size plus the 0), a long one's pointer taken; size and
    /// capacity taken, `from` left empty (capacity 15). The old buffer of
    /// `this` is not freed (a move into fresh storage). `eax`: `from`'s
    /// old capacity.
    pub fn move_from(h: &mut impl Host, this: u32, from: u32) -> u32 {
        let from_cap = cap(h, from);
        if from_cap < 16 {
            let n = size(h, from).wrapping_add(1);
            move_bytes(h, this, from, n);
        } else {
            if this != 0 {
                let p: u32 = h.read(from);
                h.write(this, p);
            }
            h.write(from, 0u32);
        }
        let n = size(h, from);
        h.write(this + 0x10, n);
        let c = cap(h, from);
        h.write(this + 0x14, c);
        h.write(from + 0x14, 0xfu32);
        h.write(from + 0x10, 0u32);
        h.write(from, 0u8);
        c
    }

    /// `std::basic_string::compare(pos, n, s, len)` (`0x006590d0`, `ret
    /// 0x10`): up to `n` characters from `pos` against the `len` at `s`:
    /// `memcmp` of the shorter, then the lengths.
    #[allow(clippy::cast_sign_loss)] // the result as `eax`
    pub fn compare(h: &mut impl Host, this: u32, pos: u32, n: u32, s: u32, len: u32) -> u32 {
        let size = size(h, this);
        if size < pos {
            out_of_range(h);
            return 0;
        }
        let rlen = n.min(size - pos);
        let m = rlen.min(len);
        let at = data(h, this).wrapping_add(pos);
        let r = memcmp(h, at, s, m);
        if r != 0 {
            return r as u32;
        }
        if rlen < len {
            u32::MAX
        } else {
            u32::from(rlen != len)
        }
    }

    /// `std::basic_string::find(s, pos, n)` (`0x00403bd0`, `ret 0xc`): the
    /// first place from `pos` where the `n` characters at `s` stand
    /// (`pos` for none at or before the end), else `npos`.
    pub fn find(h: &mut impl Host, this: u32, s: u32, pos: u32, n: u32) -> u32 {
        let size = size(h, this);
        if n == 0 && pos <= size {
            return pos;
        }
        if pos >= size || n > size - pos {
            return u32::MAX;
        }
        let first: u8 = h.read(s);
        let base = data(h, this);
        let mut rem = size - pos - n + 1;
        let mut p = base.wrapping_add(pos);
        while rem != 0 {
            let Some(k) = (0..rem).find(|&k| h.read::<u8>(p.wrapping_add(k)) == first) else {
                break;
            };
            let q = p.wrapping_add(k);
            if memcmp(h, q, s, n) == 0 {
                return q.wrapping_sub(base);
            }
            rem -= k + 1;
            p = q.wrapping_add(1);
        }
        u32::MAX
    }

    /// `std::basic_string::_Grow(new_size, trim)` (`0x00404480`, `ret 8`):
    /// room for `new_size` characters ([`grow_copy`] keeping the size);
    /// with `trim` and `new_size` below 16 a heap buffer is given up for
    /// the inline one (the first `min(size, new_size)` characters kept);
    /// `new_size` 0 empties it. `eax`: `new_size != 0`.
    pub fn grow(h: &mut impl Host, this: u32, new_size: u32, trim: u32) -> u32 {
        if new_size > MAX_SIZE {
            too_long(h);
            return u32::from(new_size != 0);
        }
        let c = cap(h, this);
        if c < new_size {
            let keep = size(h, this);
            grow_copy(h, this, new_size, keep);
        } else if trim & 0xff != 0 && new_size < 16 {
            let keep = size(h, this).min(new_size);
            if c >= 16 {
                let p: u32 = h.read(this);
                move_bytes(h, this, p, keep);
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
            }
            h.write(this + 0x10, keep);
            h.write(this + 0x14, 0xfu32);
            h.write(this.wrapping_add(keep), 0u8);
        } else if new_size == 0 {
            set_size(h, this, 0);
        }
        u32::from(new_size != 0)
    }
}
