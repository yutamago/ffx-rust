//! M5 finish, WP5 part 5 (wp5f): the `std::stringstream` code `FFX.exe`
//! instantiates for the settings file (`Settings_Format`,
//! `Settings_Parse`, `Settings_JoinBindings`): the three streams'
//! constructors, `str()`, destructors, `std::stringbuf`'s virtual methods
//! (`overflow`, `pbackfail`, `underflow`, `seekoff`, `seekpos`), its
//! `str()` and `_Init`, `operator<<` for a `char`, a C string and a
//! `std::string`, `operator>>` into a `std::string`, `use_facet<ctype>`
//! and the facet list. MSVC 2012's `<sstream>`/`<ostream>`/`<istream>`
//! templates, read from the machine code; `MSVCP110` and `MSVCR110` stay
//! imports (called through `FFX.exe`'s import slots).
//!
//! Layouts (MSVC 2012, 32-bit): a `basic_streambuf` keeps pointers to its
//! six get/put pointers and counts: `+0xc` get first, `+0x10` put first,
//! `+0x1c` get next, `+0x20` put next, `+0x2c` get count, `+0x30` put
//! count; `basic_stringbuf` adds the seek high-water mark `+0x38` and its
//! mode `+0x3c` (1 allocated, 2 constant, 4 no read, 8 append, 0x10 at
//! end). A stream's `basic_ios` sits at the offset its virtual-base table
//! (`[stream]`, word 1) gives: state `+0xc`, flags `+0x14`, width `+0x20`
//! (64-bit), its `streambuf` `+0x38`, `tie` `+0x3c`, fill `+0x40`.
//!
//! The SEH registrations, security cookies and the originals' unwind
//! states are not written: the ports keep only the stores other code
//! reads. A C++ exception out of a callee (`bad_alloc`, `length_error`,
//! a streambuf that throws) is not caught here, where the originals'
//! `catch (...)` set `badbit`; the game's stringbufs throw only when out
//! of memory.

/// The originals over game memory.
#[allow(
    clippy::many_single_char_names,
    clippy::cast_possible_truncation // bytes and words of the original's registers
)]
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    // ---- import slots (MSVCP110 unless said otherwise) ----
    pub const IOS_CTOR: u32 = 0x00b0_c23c;
    pub const IOSTREAM_CTOR: u32 = 0x00b0_c230;
    pub const STREAMBUF_CTOR: u32 = 0x00b0_c22c;
    pub const ISTREAM_CTOR: u32 = 0x00b0_c238;
    pub const OSTREAM_CTOR: u32 = 0x00b0_c308;
    pub const STREAMBUF_DTOR: u32 = 0x00b0_c30c;
    pub const IOSTREAM_DTOR: u32 = 0x00b0_c290;
    pub const ISTREAM_DTOR: u32 = 0x00b0_c298;
    pub const OSTREAM_DTOR: u32 = 0x00b0_c2c0;
    pub const IOS_DTOR: u32 = 0x00b0_c2f8;
    pub const FLUSH: u32 = 0x00b0_c2b4;
    pub const SPUTC: u32 = 0x00b0_c28c;
    pub const SPUTN: u32 = 0x00b0_c220;
    pub const SETSTATE: u32 = 0x00b0_c2ec;
    pub const UNCAUGHT_EXCEPTION: u32 = 0x00b0_c210;
    pub const OSFX: u32 = 0x00b0_c224;
    pub const PNINC: u32 = 0x00b0_c244;
    pub const XBAD_ALLOC: u32 = 0x00b0_c2a0;
    pub const XLENGTH_ERROR: u32 = 0x00b0_c29c;
    pub const IPFX: u32 = 0x00b0_c234;
    pub const GETLOC: u32 = 0x00b0_c2e8;
    pub const SGETC: u32 = 0x00b0_c310;
    pub const SNEXTC: u32 = 0x00b0_c314;
    pub const LOCKIT_CTOR: u32 = 0x00b0_c248;
    pub const LOCKIT_DTOR: u32 = 0x00b0_c1f8;
    /// `ctype<char>::id`'s import slot (a data import: the slot holds the
    /// id's address).
    pub const CTYPE_ID: u32 = 0x00b0_c20c;
    pub const ID_TO_UINT: u32 = 0x00b0_c2c4;
    pub const GLOBAL_LOCALE: u32 = 0x00b0_c214;
    pub const CTYPE_GETCAT: u32 = 0x00b0_c2cc;
    /// MSVCR110's `bad_cast(const char *)` and `memmove`.
    pub const BAD_CAST_CTOR: u32 = 0x00b0_c4a8;
    pub const MEMMOVE: u32 = 0x00b0_c34c;

    // ---- FFX.exe ----
    pub const OPERATOR_NEW: u32 = 0x0063_0310;
    pub const OPERATOR_DELETE: u32 = 0x0063_0390;
    pub const MEMCPY: u32 = 0x0094_9256;
    /// `_CxxThrowException(object, info)`'s thunk.
    pub const CXX_THROW: u32 = 0x0094_924a;
    pub const STRING_ASSIGN: u32 = 0x0040_4270;
    pub const STRING_CTOR_MOVE: u32 = 0x0040_2cf0;
    pub const STRING_COPY: u32 = 0x0040_4740;
    pub const FACET_REGISTER: u32 = 0x0094_a684;
    pub const STRINGBUF_STR: u32 = 0x0040_3ec0;
    pub const STRINGBUF_INIT: u32 = 0x0040_45d0;
    pub const STRINGBUF_TIDY: u32 = 0x0040_3010;
    pub const USE_CTYPE: u32 = 0x0040_49a0;
    pub const STRING_PUSH_BACK: u32 = 0x0040_41c0;

    /// The cached `ctype<char>` facet and the facet list (`_Fac_head`).
    pub const CTYPE_CACHE: u32 = 0x022f_b530;
    pub const FACET_LIST: u32 = 0x0193_ed28;

    // ---- vtables and their virtual-base tables ----
    pub const SS_VBTABLE: u32 = 0x00b7_cb78;
    pub const SS_OSTREAM_VBTABLE: u32 = 0x00b8_f53c;
    pub const SS_IOS_VTABLE: u32 = 0x00b7_cb6c;
    pub const STRINGBUF_VTABLE: u32 = 0x00b7_cb2c;
    pub const ISS_VBTABLE: u32 = 0x00b8_f79c;
    pub const ISS_IOS_VTABLE: u32 = 0x00b8_f790;
    pub const OSS_VBTABLE: u32 = 0x00b7_cb80;
    pub const OSS_IOS_VTABLE: u32 = 0x00b7_cb1c;
    /// "bad cast", `bad_cast`'s throw info, "string too long".
    pub const BAD_CAST_TEXT: u32 = 0x00b7_cb0c;
    pub const BAD_CAST_THROW_INFO: u32 = 0x00c0_4e4c;
    pub const TOO_LONG_TEXT: u32 = 0x00b7_cafc;

    /// `badbit`, `failbit`, `eofbit`; `left` within `adjustfield`.
    const BAD: u32 = 4;
    const FAIL: u32 = 2;
    const EOF_BIT: u32 = 1;
    const EOF: u32 = u32::MAX;

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// Virtual method at byte offset `off` of `obj`'s vtable (thiscall).
    fn vcall(h: &mut impl Host, obj: u32, off: u32, args: &[u32]) -> u32 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(at(vt, off));
        h.call(f, Conv::Thiscall(obj), args)
    }

    /// The stream `s`'s `basic_ios` (its virtual base).
    fn ios_of(h: &impl Host, s: u32) -> u32 {
        let vb: u32 = h.read(s);
        at(s, h.read(at(vb, 4)))
    }

    /// The stream's `rdbuf()`.
    fn rdbuf(h: &impl Host, s: u32) -> u32 {
        h.read(at(ios_of(h, s), 0x38))
    }

    /// `*p` for a streambuf's pointer-to-pointer member at `off`.
    fn get(h: &impl Host, sb: u32, off: u32) -> u32 {
        let p: u32 = h.read(at(sb, off));
        h.read(p)
    }

    fn set(h: &mut impl Host, sb: u32, off: u32, v: u32) {
        let p: u32 = h.read(at(sb, off));
        h.write(p, v);
    }

    /// The virtual base's vtable stored through the virtual-base table
    /// and the displacement word below it (`dist - base`), each re-read as
    /// the original does.
    fn set_ios_vtable(h: &mut impl Host, s: u32, vtable: u32, base: u32) {
        let vb: u32 = h.read(s);
        let dist: u32 = h.read(at(vb, 4));
        let vt = h.rebase(vtable);
        h.write(at(s, dist), vt);
        let vb: u32 = h.read(s);
        let dist: u32 = h.read(at(vb, 4));
        h.write(at(s, dist).wrapping_sub(4), dist.wrapping_sub(base));
    }

    /// The stringbuf part of a constructor: `basic_streambuf()`, the
    /// vtable, no seek mark, `mode`.
    fn stringbuf_ctor(h: &mut impl Host, sb: u32, mode: u32) {
        import(h, STREAMBUF_CTOR, Conv::Thiscall(sb), &[]);
        let vt = h.rebase(STRINGBUF_VTABLE);
        h.write(sb, vt);
        h.write(at(sb, 0x38), 0u32);
        h.write(at(sb, 0x3c), mode);
    }

    /// `0x00402f00(this; 2 words)` (thiscall, `ret 8`; the words are not
    /// read): `std::stringstream()`: the virtual-base tables, `basic_ios`
    /// at `+0x68`, `basic_iostream(&buf, 0)`, the `basic_ios` vtable, the
    /// `stringbuf` at `+0x18` (mode 0). Returns `this`.
    pub fn stringstream_ctor(h: &mut impl Host, this: u32) -> u32 {
        let vb = h.rebase(SS_VBTABLE);
        h.write(this, vb);
        let vb = h.rebase(SS_OSTREAM_VBTABLE);
        h.write(at(this, 0x10), vb);
        import(h, IOS_CTOR, Conv::Thiscall(at(this, 0x68)), &[]);
        let sb = at(this, 0x18);
        import(h, IOSTREAM_CTOR, Conv::Thiscall(this), &[sb, 0]);
        set_ios_vtable(h, this, SS_IOS_VTABLE, 0x68);
        stringbuf_ctor(h, sb, 0);
        this
    }

    /// `0x00403620(this; str, 2 words)` (thiscall, `ret 0xc`; the words
    /// are not read): `std::istringstream(str)`: `basic_ios` at `+0x60`,
    /// `basic_istream(&buf, false, 0)`, the `stringbuf` at `+0x10` (mode 2)
    /// filled from `str`'s characters when it has any (`_Init`). Returns
    /// `this`.
    pub fn istringstream_ctor(h: &mut impl Host, this: u32, s: u32) -> u32 {
        let vb = h.rebase(ISS_VBTABLE);
        h.write(this, vb);
        import(h, IOS_CTOR, Conv::Thiscall(at(this, 0x60)), &[]);
        let sb = at(this, 0x10);
        import(h, ISTREAM_CTOR, Conv::Thiscall(this), &[sb, 0, 0]);
        set_ios_vtable(h, this, ISS_IOS_VTABLE, 0x60);
        import(h, STREAMBUF_CTOR, Conv::Thiscall(sb), &[]);
        let vt = h.rebase(STRINGBUF_VTABLE);
        h.write(sb, vt);
        let size: u32 = h.read(at(s, 0x10));
        let chars = if h.read::<u32>(at(s, 0x14)) >= 0x10 {
            h.read(s)
        } else {
            s
        };
        h.write(at(sb, 0x38), 0u32);
        h.write(at(sb, 0x3c), 2u32);
        if size != 0 {
            h.call_sym(STRINGBUF_INIT, Conv::Thiscall(sb), &[chars, size]);
        }
        this
    }

    /// `0x00403730(this; 2 words)` (thiscall, `ret 8`): `std::ostringstream()`:
    /// `basic_ios` at `+0x50`, `basic_ostream(&buf, false, 0)`, the
    /// `stringbuf` at `+4` (mode 4). Returns `this`.
    pub fn ostringstream_ctor(h: &mut impl Host, this: u32) -> u32 {
        let vb = h.rebase(OSS_VBTABLE);
        h.write(this, vb);
        import(h, IOS_CTOR, Conv::Thiscall(at(this, 0x50)), &[]);
        let sb = at(this, 4);
        import(h, OSTREAM_CTOR, Conv::Thiscall(this), &[sb, 0, 0]);
        set_ios_vtable(h, this, OSS_IOS_VTABLE, 0x50);
        stringbuf_ctor(h, sb, 4);
        this
    }

    /// `0x00402ff0` / `0x00403810` (thiscall, `ret 4`): a stream's `str()`
    /// into `out` (its stringbuf at `this + buf`). Returns `out`.
    pub fn stream_str(h: &mut impl Host, this: u32, buf: u32, out: u32) -> u32 {
        h.call_sym(STRINGBUF_STR, Conv::Thiscall(at(this, buf)), &[out]);
        out
    }

    /// `0x00403010(this)` (thiscall): `~basic_stringbuf()`'s body: the
    /// vtable, the buffer freed when the stringbuf owns it (mode bit 1),
    /// the six pointers cleared, the bit and the seek mark cleared, then a
    /// tail jump to `~basic_streambuf` (its `eax`).
    pub fn stringbuf_tidy(h: &mut impl Host, this: u32) -> u32 {
        let owns = h.read::<u8>(at(this, 0x3c)) & 1 != 0;
        let vt = h.rebase(STRINGBUF_VTABLE);
        h.write(this, vt);
        if owns {
            let block = get(h, this, 0xc);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[block]);
        }
        for off in [0xc, 0x1c, 0x2c, 0x10, 0x20, 0x30] {
            set(h, this, off, 0);
        }
        let mode: u32 = h.read(at(this, 0x3c));
        h.write(at(this, 0x3c), mode & !1);
        h.write(at(this, 0x38), 0u32);
        import(h, STREAMBUF_DTOR, Conv::Thiscall(this), &[])
    }

    /// `0x00403890(this; flags)` (thiscall, `ret 4`): the stringbuf's
    /// deleting destructor. Returns `this`.
    pub fn stringbuf_deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        h.call_sym(STRINGBUF_TIDY, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// A stream class: its `basic_ios` offset, the `basic_ios` vtable, the
    /// stringbuf's offset and the middle class's destructor.
    #[derive(Clone, Copy, Debug)]
    pub struct Stream {
        pub ios: u32,
        pub ios_vtable: u32,
        pub buf: u32,
        pub base_dtor: u32,
        /// Where the middle class's destructor gets its `this`: the
        /// `basic_ios` less 0x48 (`+0x20` for the `iostream`, `+0x18` the
        /// `istream`, `+8` the `ostream`).
        pub base: u32,
    }

    /// `std::stringstream` (`0x00403830`, `0x00402fc0`).
    pub const STRINGSTREAM: Stream = Stream {
        ios: 0x68,
        ios_vtable: SS_IOS_VTABLE,
        buf: 0x18,
        base_dtor: IOSTREAM_DTOR,
        base: 0x20,
    };
    /// `std::istringstream` (`0x004038c0`, `0x00403700`).
    pub const ISTRINGSTREAM: Stream = Stream {
        ios: 0x60,
        ios_vtable: ISS_IOS_VTABLE,
        buf: 0x10,
        base_dtor: ISTREAM_DTOR,
        base: 0x18,
    };
    /// `std::ostringstream` (`0x00403920`, `0x004037e0`).
    pub const OSTRINGSTREAM: Stream = Stream {
        ios: 0x50,
        ios_vtable: OSS_IOS_VTABLE,
        buf: 4,
        base_dtor: OSTREAM_DTOR,
        base: 8,
    };

    /// The deleting destructors through the `basic_ios` (`0x00403830`
    /// stringstream, `0x004038c0` istringstream, `0x00403920`
    /// ostringstream; thiscall, `ret 4`; `this`
    /// is the `basic_ios`): the full object below it is destroyed (the
    /// `basic_ios` vtable, the displacement word, the stringbuf, the middle
    /// class at `base`, `~basic_ios`) and deleted when `flags` bit 0 is
    /// set. Returns the full object.
    pub fn stream_deleting_dtor(h: &mut impl Host, this: u32, flags: u32, s: Stream) -> u32 {
        let full = this.wrapping_sub(s.ios);
        set_ios_vtable(h, full, s.ios_vtable, s.ios);
        h.call_sym(STRINGBUF_TIDY, Conv::Thiscall(at(full, s.buf)), &[]);
        import(h, s.base_dtor, Conv::Thiscall(at(full, s.base)), &[]);
        import(h, IOS_DTOR, Conv::Thiscall(at(full, s.ios)), &[]);
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[full]);
        }
        full
    }

    /// The vbase destructors (`0x00402fc0` stringstream, `0x00403700`
    /// istringstream, `0x004037e0` ostringstream; thiscall, `this` the
    /// `basic_ios`): the `basic_ios` vtable and displacement word, the
    /// stringbuf's body, then a tail jump to the middle class's destructor
    /// at `ios - 0x48` (its `eax`).
    pub fn stream_vbase_dtor(h: &mut impl Host, this: u32, s: Stream) -> u32 {
        let full = this.wrapping_sub(s.ios);
        set_ios_vtable(h, full, s.ios_vtable, s.ios);
        h.call_sym(STRINGBUF_TIDY, Conv::Thiscall(at(full, s.buf)), &[]);
        import(h, s.base_dtor, Conv::Thiscall(this.wrapping_sub(0x48)), &[])
    }

    /// `0x00403080(this; c)` (thiscall, `ret 4`): `basic_stringbuf::overflow`.
    /// -1 for a constant buffer; 0 for `c == EOF`; with append mode the put
    /// pointer moves to the seek mark first; a free put position takes `c`
    /// (`_Pninc`); else the buffer grows by half its size (at least 32
    /// bytes, halved while the total would pass `0x7fffffff`; -1 when
    /// nothing fits): `operator new` (`_Xbad_alloc` on 0), the old bytes
    /// copied, the six pointers and the seek mark moved, the old buffer
    /// freed when owned, the buffer owned now, and `c` stored. Returns `c`.
    #[allow(clippy::too_many_lines)] // the original's two re-pointing paths
    pub fn stringbuf_overflow<H: Host>(h: &mut H, this: u32, c: u32) -> u32 {
        let mode: u32 = h.read(at(this, 0x3c));
        if mode & 2 != 0 {
            return EOF;
        }
        if c == EOF {
            return 0;
        }
        if mode & 8 != 0 {
            let next = get(h, this, 0x20);
            let high: u32 = h.read(at(this, 0x38));
            if next != 0 && next < high {
                let count = get(h, this, 0x30).wrapping_add(next);
                set(h, this, 0x20, high);
                set(h, this, 0x30, count.wrapping_sub(high));
            }
        }
        let next = get(h, this, 0x20);
        let old_size = if next == 0 {
            0
        } else {
            if next < get(h, this, 0x30).wrapping_add(next) {
                let p = import(h, PNINC, Conv::Thiscall(this), &[]);
                h.write(p, c as u8);
                return c;
            }
            get(h, this, 0x30)
                .wrapping_sub(get(h, this, 0xc))
                .wrapping_add(next)
        };
        let mut grow = old_size >> 1;
        if grow < 0x20 {
            grow = 0x20;
        }
        while grow != 0 && 0x7fff_ffff - grow < old_size {
            grow >>= 1;
        }
        if grow == 0 {
            return EOF;
        }
        let size = grow.wrapping_add(old_size);
        let mut new = 0;
        if size != 0 {
            new = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[size]);
            if new == 0 {
                import(h, XBAD_ALLOC, Conv::Cdecl, &[]);
            }
        }
        let old = get(h, this, 0xc);
        let no_read = |h: &mut H| h.read::<u8>(at(this, 0x3c)) & 4 != 0;
        if old_size == 0 {
            h.write(at(this, 0x38), new);
            set(h, this, 0x10, new);
            set(h, this, 0x20, new);
            set(h, this, 0x30, size);
            set(h, this, 0xc, new);
            if no_read(h) {
                set(h, this, 0x1c, 0);
                set(h, this, 0x2c, new);
            } else {
                set(h, this, 0x1c, new);
                set(h, this, 0x2c, 1);
            }
        } else {
            h.call_sym(MEMCPY, Conv::Cdecl, &[new, old, old_size]);
            let shift = new.wrapping_sub(old);
            let high: u32 = h.read(at(this, 0x38));
            h.write(at(this, 0x38), high.wrapping_add(shift));
            let put = get(h, this, 0x20).wrapping_sub(old);
            let first = get(h, this, 0x10);
            set(h, this, 0x10, first.wrapping_add(shift));
            let put = put.wrapping_add(new);
            set(h, this, 0x20, put);
            set(h, this, 0x30, new.wrapping_sub(put).wrapping_add(size));
            if no_read(h) {
                set(h, this, 0xc, new);
                set(h, this, 0x1c, 0);
                set(h, this, 0x2c, new);
            } else {
                let put = get(h, this, 0x20);
                let gnext = get(h, this, 0x1c).wrapping_sub(old).wrapping_add(new);
                set(h, this, 0xc, new);
                set(h, this, 0x1c, gnext);
                set(h, this, 0x2c, put.wrapping_sub(gnext).wrapping_add(1));
            }
        }
        if h.read::<u8>(at(this, 0x3c)) & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[old]);
        }
        let mode: u32 = h.read(at(this, 0x3c));
        h.write(at(this, 0x3c), mode | 1);
        let p = import(h, PNINC, Conv::Thiscall(this), &[]);
        h.write(p, c as u8);
        c
    }

    /// `0x00403260(this; c)` (thiscall, `ret 4`): `pbackfail`: with a get
    /// position past the start, `c` put back (EOF, the same character, or
    /// a writable buffer): the get pointer down one, its count up one, `c`
    /// stored (0 returned for EOF); else -1.
    pub fn stringbuf_pbackfail(h: &mut impl Host, this: u32, c: u32) -> u32 {
        let next = get(h, this, 0x1c);
        if next != 0 && get(h, this, 0xc) < next {
            let same = c as u8 == h.read::<u8>(next.wrapping_sub(1));
            if c == EOF || same || h.read::<u8>(at(this, 0x3c)) & 2 == 0 {
                let n = get(h, this, 0x2c).wrapping_add(1);
                set(h, this, 0x2c, n);
                let p = get(h, this, 0x1c).wrapping_sub(1);
                set(h, this, 0x1c, p);
                if c == EOF {
                    return 0;
                }
                let p = get(h, this, 0x1c);
                h.write(p, c as u8);
                return c;
            }
        }
        EOF
    }

    /// `0x004032c0(this)` (thiscall): `underflow`: -1 with no get area;
    /// the next character while the get count is positive; else, for a
    /// readable buffer whose put pointer or seek mark is past the get
    /// pointer, the get area extended to the mark (raised to the put
    /// pointer first) and its next character; else -1.
    pub fn stringbuf_underflow(h: &mut impl Host, this: u32) -> u32 {
        let next = get(h, this, 0x1c);
        if next == 0 {
            return EOF;
        }
        if next < get(h, this, 0x2c).wrapping_add(next) {
            return u32::from(h.read::<u8>(next));
        }
        if h.read::<u8>(at(this, 0x3c)) & 4 == 0 {
            let put = get(h, this, 0x20);
            if put != 0 && (next < put || next < h.read::<u32>(at(this, 0x38))) {
                if h.read::<u32>(at(this, 0x38)) < put {
                    h.write(at(this, 0x38), put);
                }
                let high: u32 = h.read(at(this, 0x38));
                let n = high.wrapping_sub(get(h, this, 0x1c));
                set(h, this, 0x2c, n);
                let p = get(h, this, 0x1c);
                return u32::from(h.read::<u8>(p));
            }
        }
        EOF
    }

    /// The seek mark raised to the put pointer (`seekoff`/`seekpos`).
    fn raise_mark(h: &mut impl Host, this: u32) {
        let put = get(h, this, 0x20);
        if put != 0 && h.read::<u32>(at(this, 0x38)) < put {
            h.write(at(this, 0x38), put);
        }
    }

    /// `off` (64-bit) within `0..=seek mark - buffer start` (signed
    /// compare of the high words, as the original).
    fn in_range(h: &impl Host, this: u32, off: u64) -> bool {
        let span = h
            .read::<u32>(at(this, 0x38))
            .wrapping_sub(get(h, this, 0xc));
        let hi = ((off >> 32) as u32).cast_signed();
        let span_hi = span.cast_signed() >> 31;
        hi >= 0 && (hi < span_hi || (hi == span_hi && (off as u32) <= span))
    }

    /// `x + y` with `y` sign-extended (the original's `add`/`adc`).
    fn add_signed(x: u64, y: u32) -> u64 {
        x.wrapping_add(i64::from(y.cast_signed()).cast_unsigned())
    }

    /// The `streampos` result: offset, 0, 0, 0 (state and `fpos`).
    fn write_pos(h: &mut impl Host, out: u32, pos: u64) {
        h.write(at(out, 4), (pos >> 32) as u32);
        h.write(out, pos as u32);
        for o in [8, 0xc, 0x10] {
            h.write(at(out, o), 0u32);
        }
    }

    /// `_BADOFF`'s import slot (MSVCP110 data: the slot holds its
    /// address; a 64-bit -1).
    pub const BADOFF: u32 = 0x00b0_c208;

    fn badoff(h: &impl Host) -> u64 {
        let p: u32 = h.read(h.rebase(BADOFF));
        let lo: u32 = h.read(p);
        let hi: u32 = h.read(at(p, 4));
        u64::from(hi) << 32 | u64::from(lo)
    }

    /// `0x00403320(this; out, off (64-bit), way, which)` (thiscall,
    /// `ret 0x14`): `seekoff`: the seek mark raised; for the get side
    /// (`which` bit 0, a get area) or else the put side (bit 1, a put
    /// area): `off` from the start (`way` 0), the current position (1;
    /// for the get side only when bit 1 is clear) or the end (2), a bad
    /// way giving `_BADOFF`; a position within the mark moves the pointer
    /// (the get side moves a put pointer along when bit 1 is set); else
    /// `_BADOFF`. Neither side with `off` 0 succeeds as it is. The
    /// position in `out`; returns `out`.
    pub fn stringbuf_seekoff<H: Host>(
        h: &mut H,
        this: u32,
        out: u32,
        off: u64,
        way: u32,
        which: u32,
    ) -> u32 {
        raise_mark(h, this);
        let start = |h: &mut H| get(h, this, 0xc);
        let high = |h: &mut H| h.read::<u32>(at(this, 0x38));
        let gnext = get(h, this, 0x1c);
        let pos = if which & 1 != 0 && gnext != 0 {
            let off = match way {
                2 => Some(add_signed(off, high(h).wrapping_sub(start(h)))),
                1 if which & 2 == 0 => Some(add_signed(off, gnext.wrapping_sub(start(h)))),
                0 => Some(off),
                _ => None,
            };
            match off {
                Some(off) if in_range(h, this, off) => {
                    let delta = start(h).wrapping_sub(gnext).wrapping_add(off as u32);
                    let n = get(h, this, 0x2c).wrapping_sub(delta);
                    set(h, this, 0x2c, n);
                    let p = get(h, this, 0x1c).wrapping_add(delta);
                    set(h, this, 0x1c, p);
                    let put = get(h, this, 0x20);
                    if which & 2 != 0 && put != 0 {
                        let count = get(h, this, 0x30);
                        let g = get(h, this, 0x1c);
                        set(h, this, 0x20, g);
                        set(h, this, 0x30, count.wrapping_add(put).wrapping_sub(g));
                    }
                    off
                }
                _ => badoff(h),
            }
        } else if which & 2 != 0 && get(h, this, 0x20) != 0 {
            let pnext = get(h, this, 0x20);
            let off = match way {
                2 => add_signed(off, high(h).wrapping_sub(start(h))),
                1 => add_signed(off, pnext.wrapping_sub(start(h))),
                0 => off,
                _ => badoff(h),
            };
            if in_range(h, this, off) {
                let delta = start(h).wrapping_sub(pnext).wrapping_add(off as u32);
                let n = get(h, this, 0x30).wrapping_sub(delta);
                set(h, this, 0x30, n);
                let p = get(h, this, 0x20).wrapping_add(delta);
                set(h, this, 0x20, p);
                off
            } else {
                badoff(h)
            }
        } else if off == 0 {
            0
        } else {
            badoff(h)
        };
        write_pos(h, out, pos);
        out
    }

    /// `0x004034f0(this; out, pos (fpos: off 64-bit, fpos 64-bit, state),
    /// which)` (thiscall, `ret 0x20`): `seekpos`: the target is `off +
    /// fpos`; `_BADOFF` stays as it is; else the get side (bit 0, a get
    /// area) or the put side (bit 1, a put area) moves there when it lies
    /// within the seek mark (the get side moves a put pointer along when
    /// bit 1 is set), else `_BADOFF`. Returns `out`.
    pub fn stringbuf_seekpos(
        h: &mut impl Host,
        this: u32,
        out: u32,
        off: u64,
        fpos: u64,
        which: u32,
    ) -> u32 {
        let target = fpos.wrapping_add(off);
        raise_mark(h, this);
        let bad = badoff(h);
        let mut result = bad;
        if target != bad {
            let start = get(h, this, 0xc);
            if which & 1 != 0 && get(h, this, 0x1c) != 0 {
                if in_range(h, this, target) {
                    let delta = start
                        .wrapping_sub(get(h, this, 0x1c))
                        .wrapping_add(target as u32);
                    let n = get(h, this, 0x2c).wrapping_sub(delta);
                    set(h, this, 0x2c, n);
                    let p = get(h, this, 0x1c).wrapping_add(delta);
                    set(h, this, 0x1c, p);
                    result = target;
                    let put = get(h, this, 0x20);
                    if which & 2 != 0 && put != 0 {
                        let count = get(h, this, 0x30);
                        let g = get(h, this, 0x1c);
                        set(h, this, 0x20, g);
                        set(h, this, 0x30, count.wrapping_add(put).wrapping_sub(g));
                    }
                }
            } else if which & 2 != 0 && get(h, this, 0x20) != 0 && in_range(h, this, target) {
                let delta = start
                    .wrapping_sub(get(h, this, 0x20))
                    .wrapping_add(target as u32);
                let n = get(h, this, 0x30).wrapping_sub(delta);
                set(h, this, 0x30, n);
                let p = get(h, this, 0x20).wrapping_add(delta);
                set(h, this, 0x20, p);
                result = target;
            }
        }
        write_pos(h, out, result);
        out
    }

    /// `0x00403ec0(this; out)` (thiscall, `ret 4`): `basic_stringbuf::str()`
    /// into the `std::string` `out`: a writable buffer with a put area
    /// gives its bytes up to the seek mark (or the put pointer past it), a
    /// readable one with a get area its get area; each built in a
    /// temporary (`tmp`, the original's 24-byte string at `ebp - 0x20`:
    /// `assign(p, n)`, moved into `out`, its block freed when it had one);
    /// else `out` is the empty string (the temporary's bytes `memmove`d in).
    /// Returns `out`.
    pub fn stringbuf_str(h: &mut impl Host, this: u32, out: u32, tmp: u32) -> u32 {
        let mode: u32 = h.read(at(this, 0x3c));
        let range = if mode & 2 == 0 && get(h, this, 0x20) != 0 {
            let put = get(h, this, 0x20);
            let high: u32 = h.read(at(this, 0x38));
            let end = if high < put { put } else { high };
            let first = get(h, this, 0x10);
            Some((first, end.wrapping_sub(first)))
        } else if mode & 4 == 0 && get(h, this, 0x1c) != 0 {
            let next = get(h, this, 0x1c);
            let first = get(h, this, 0xc);
            let n = get(h, this, 0x2c).wrapping_add(next).wrapping_sub(first);
            Some((first, n))
        } else {
            None
        };
        h.write(at(tmp, 0x14), 0xfu32);
        h.write(at(tmp, 0x10), 0u32);
        h.write(tmp, 0u8);
        match range {
            Some((p, n)) => {
                h.call_sym(STRING_ASSIGN, Conv::Thiscall(tmp), &[p, n]);
                h.call_sym(STRING_CTOR_MOVE, Conv::Thiscall(out), &[tmp]);
                if h.read::<u32>(at(tmp, 0x14)) >= 0x10 {
                    let block: u32 = h.read(tmp);
                    h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[block]);
                }
            }
            None => {
                h.write(at(out, 0x14), 0xfu32);
                h.write(at(out, 0x10), 0u32);
                h.write(out, 0u8);
                // the temporary is the empty small string: its size + 1
                // bytes copied
                import(h, MEMMOVE, Conv::Cdecl, &[out, tmp, 1]);
                h.write(at(out, 0x10), 0u32);
                h.write(at(out, 0x14), 0xfu32);
            }
        }
        out
    }

    /// `0x004045d0(this; p, n)` (thiscall, `ret 8`): `basic_stringbuf::_Init`
    /// for `n` bytes at `p`: nothing for a constant no-read buffer (mode
    /// bits 2 and 4); else a copy (`operator new`, `_Xbad_alloc` on 0),
    /// the seek mark at its end, the get area over it unless no-read, the
    /// put area over it unless constant (at its end in at-end mode; a
    /// missing get area then becomes `(copy, 0, copy)`), and the buffer
    /// owned.
    pub fn stringbuf_init(h: &mut impl Host, this: u32, p: u32, n: u32) {
        if h.read::<u8>(at(this, 0x3c)) & 6 == 6 {
            return;
        }
        let new = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[n]);
        if new == 0 {
            import(h, XBAD_ALLOC, Conv::Cdecl, &[]);
        }
        h.call_sym(MEMCPY, Conv::Cdecl, &[new, p, n]);
        h.write(at(this, 0x38), new.wrapping_add(n));
        if h.read::<u8>(at(this, 0x3c)) & 4 == 0 {
            set(h, this, 0xc, new);
            set(h, this, 0x1c, new);
            set(h, this, 0x2c, n);
        }
        let mode: u32 = h.read(at(this, 0x3c));
        if mode & 2 == 0 {
            set(h, this, 0x10, new);
            let put = if mode & 0x10 != 0 {
                new.wrapping_add(n)
            } else {
                new
            };
            set(h, this, 0x20, put);
            set(h, this, 0x30, new.wrapping_add(n).wrapping_sub(put));
            if get(h, this, 0x1c) == 0 {
                set(h, this, 0xc, new);
                set(h, this, 0x1c, 0);
                set(h, this, 0x2c, new);
            }
        }
        let mode: u32 = h.read(at(this, 0x3c));
        h.write(at(this, 0x3c), mode | 1);
    }

    /// An `ostream::sentry`: the streambuf locked (`_Lock`, vtable slot
    /// 1), a good stream's `tie` flushed; whether the stream is good.
    fn osentry_enter(h: &mut impl Host, os: u32) -> bool {
        let sb = rdbuf(h, os);
        if sb != 0 {
            vcall(h, sb, 4, &[]);
        }
        let ios = ios_of(h, os);
        if h.read::<u32>(at(ios, 0xc)) == 0 {
            let tie: u32 = h.read(at(ios, 0x3c));
            if tie != 0 {
                import(h, FLUSH, Conv::Thiscall(tie), &[]);
            }
        }
        h.read::<u32>(at(ios_of(h, os), 0xc)) == 0
    }

    /// The sentry's end: `_Osfx()` unless an exception is in flight, the
    /// streambuf unlocked (`_Unlock`, vtable slot 2).
    fn osentry_leave(h: &mut impl Host, os: u32) {
        // a `bool` in `al`
        if import(h, UNCAUGHT_EXCEPTION, Conv::Cdecl, &[]) as u8 == 0 {
            import(h, OSFX, Conv::Thiscall(os), &[]);
        }
        let sb = rdbuf(h, os);
        if sb != 0 {
            vcall(h, sb, 8, &[]);
        }
    }

    /// The stream's width (64-bit at `+0x20`).
    fn width(h: &impl Host, ios: u32) -> i64 {
        let lo: u32 = h.read(at(ios, 0x20));
        let hi: u32 = h.read(at(ios, 0x24));
        (u64::from(hi) << 32 | u64::from(lo)).cast_signed()
    }

    /// `sputc(fill)`: whether it failed. The original pushes a frame word
    /// whose low byte it set to the fill; `high` is that word's upper bytes
    /// (`sputc` reads the byte alone): where the word held a register
    /// (the string's address) the port passes it, else it is never
    /// written: junk read, 0 (brief 2026-10-24).
    fn put_fill(h: &mut impl Host, os: u32, high: u32) -> bool {
        let ios = ios_of(h, os);
        let fill = u32::from(h.read::<u8>(at(ios, 0x40)));
        let sb: u32 = h.read(at(ios, 0x38));
        import(h, SPUTC, Conv::Thiscall(sb), &[high & !0xff | fill]) == EOF
    }

    /// Left adjustment (`flags & adjustfield == left`).
    fn left(h: &impl Host, os: u32) -> bool {
        h.read::<u32>(at(ios_of(h, os), 0x14)) & 0x1c0 == 0x40
    }

    /// The width cleared, `setstate(state, false)`.
    fn finish(h: &mut impl Host, os: u32, state: u32) {
        let ios = ios_of(h, os);
        h.write(at(ios, 0x20), 0u32);
        h.write(at(ios, 0x24), 0u32);
        let ios = ios_of(h, os);
        import(h, SETSTATE, Conv::Thiscall(ios), &[state, 0]);
    }

    /// `0x00404a90(os)` (`ecx` = the stream; no stack words):
    /// `os << ','` (`operator<<(ostream &, char)` with the comma inlined):
    /// padded to the width with the fill (before the character unless
    /// left-adjusted, else after), `badbit` at the first failed `sputc`.
    /// Returns `os`.
    pub fn put_comma(h: &mut impl Host, os: u32) -> u32 {
        let w = width(h, ios_of(h, os));
        let mut state = 0u32;
        if osentry_enter(h, os) {
            let mut pad = if w > 1 { w - 1 } else { 0 };
            let mut put_char = true;
            if !left(h, os) {
                loop {
                    if state != 0 {
                        put_char = false;
                        break;
                    }
                    if pad <= 0 {
                        break;
                    }
                    if put_fill(h, os, 0) {
                        state = BAD;
                    }
                    pad -= 1;
                }
            }
            if put_char {
                let sb = rdbuf(h, os);
                if import(h, SPUTC, Conv::Thiscall(sb), &[u32::from(b',')]) == EOF {
                    state = BAD;
                }
                while state == 0 && pad > 0 {
                    if put_fill(h, os, 0) {
                        state = BAD;
                    }
                    pad -= 1;
                }
            }
        }
        finish(h, os, state);
        osentry_leave(h, os);
        os
    }

    /// `0x00404ca0(os, s)` (fastcall: `ecx` the stream, `edx` the C
    /// string): `os << s`: padded to the width (before unless
    /// left-adjusted, else after), `sputn` of the whole string (`badbit`
    /// when it writes less), `badbit` at a failed fill or a bad stream.
    /// Returns `os`.
    pub fn put_cstr(h: &mut impl Host, os: u32, s: u32) -> u32 {
        let mut len = 0u32;
        while h.read::<u8>(at(s, len)) != 0 {
            len = len.wrapping_add(1);
        }
        let w = width(h, ios_of(h, os));
        let mut pad = if w > 0 && w > i64::from(len) {
            w - i64::from(len)
        } else {
            0
        };
        let mut state = 0u32;
        if osentry_enter(h, os) {
            let mut failed = false;
            if !left(h, os) {
                while pad > 0 {
                    if put_fill(h, os, 0) {
                        state |= BAD;
                        failed = true;
                        break;
                    }
                    pad -= 1;
                }
            }
            if !failed {
                let sb = rdbuf(h, os);
                let f = import_addr(h, SPUTN);
                let n = h.call_u64(f, Conv::Thiscall(sb), &[s, len, 0]);
                if n == u64::from(len) {
                    while pad > 0 {
                        if put_fill(h, os, s) {
                            state |= BAD;
                            break;
                        }
                        pad -= 1;
                    }
                } else {
                    state = BAD;
                }
            }
            let ios = ios_of(h, os);
            h.write(at(ios, 0x20), 0u32);
            h.write(at(ios, 0x24), 0u32);
        } else {
            state = BAD;
        }
        let ios = ios_of(h, os);
        import(h, SETSTATE, Conv::Thiscall(ios), &[state, 0]);
        osentry_leave(h, os);
        os
    }

    fn import_addr(h: &impl Host, slot: u32) -> u32 {
        h.read(h.rebase(slot))
    }

    /// `0x00404ed0(os, s)` (fastcall: `ecx` the stream, `edx` the
    /// `std::string`): `os << s`: the pad is the width's low word past the
    /// size (its high word only tested for a sign and for 0), before the
    /// characters unless left-adjusted; `sputn` of the characters; the
    /// padding after them runs when the first padding failed too (MSVC
    /// 2012's `else` binds to the `sputn` test). Returns `os`.
    pub fn put_string(h: &mut impl Host, os: u32, s: u32) -> u32 {
        let size: u32 = h.read(at(s, 0x10));
        let ios = ios_of(h, os);
        let lo: u32 = h.read(at(ios, 0x20));
        let hi = h.read::<u32>(at(ios, 0x24)).cast_signed();
        let mut pad = if hi < 0 || (hi == 0 && lo == 0) || lo <= size {
            0
        } else {
            lo - size
        };
        let mut state = 0u32;
        if osentry_enter(h, os) {
            if !left(h, os) {
                while pad != 0 {
                    if put_fill(h, os, 0) {
                        state |= BAD;
                        break;
                    }
                    pad -= 1;
                }
            }
            let mut tail = true;
            if state == 0 {
                let chars = if h.read::<u32>(at(s, 0x14)) >= 0x10 {
                    h.read(s)
                } else {
                    s
                };
                let sb = rdbuf(h, os);
                let f = import_addr(h, SPUTN);
                let n = h.call_u64(f, Conv::Thiscall(sb), &[chars, size, 0]);
                if n != u64::from(size) {
                    state = BAD;
                    tail = false;
                }
            }
            if tail {
                while pad != 0 {
                    if put_fill(h, os, s) {
                        state |= BAD;
                        break;
                    }
                    pad -= 1;
                }
            }
            let ios = ios_of(h, os);
            h.write(at(ios, 0x20), 0u32);
            h.write(at(ios, 0x24), 0u32);
        } else {
            state = BAD;
        }
        let ios = ios_of(h, os);
        import(h, SETSTATE, Conv::Thiscall(ios), &[state, 0]);
        osentry_leave(h, os);
        os
    }

    /// `0x00405800(is, s)` (fastcall: `ecx` the stream, `edx` the
    /// `std::string`): `is >> s`: the streambuf locked, `_Ipfx(false)`;
    /// when good: the stream's locale (`getloc` into `loc`, a word of the
    /// original's frame at `ebp - 0x1c`), its `ctype<char>`
    /// ([`USE_CTYPE`]), the locale released (`_Locimp`'s `_Decref`, slot 2,
    /// and the deleting destructor of what it returns, slot 0); `s`
    /// cleared, then characters appended up to the width (`0xfffffffe`
    /// without one) until a space (`ctype` mask 0x48) or the end
    /// (`eofbit`). `failbit` when nothing was read; the width cleared,
    /// `setstate`, the streambuf unlocked. Returns `is`.
    pub fn get_string(h: &mut impl Host, is: u32, s: u32, loc: u32) -> u32 {
        let sb = rdbuf(h, is);
        if sb != 0 {
            vcall(h, sb, 4, &[]);
        }
        let mut state = 0u32;
        let mut changed = false;
        if import(h, IPFX, Conv::Thiscall(is), &[0]) & 0xff != 0 {
            let ios = ios_of(h, is);
            let l = import(h, GETLOC, Conv::Thiscall(ios), &[loc]);
            let ctype = h.call_sym(USE_CTYPE, Conv::Fastcall, &[l, 0]);
            let imp: u32 = h.read(loc);
            if imp != 0 {
                let dead = vcall(h, imp, 8, &[]);
                if dead != 0 {
                    vcall(h, dead, 0, &[1]);
                }
            }
            h.write(at(s, 0x10), 0u32);
            let chars = if h.read::<u32>(at(s, 0x14)) >= 0x10 {
                h.read(s)
            } else {
                s
            };
            h.write(chars, 0u8);
            let ios = ios_of(h, is);
            let lo: u32 = h.read(at(ios, 0x20));
            let hi = h.read::<u32>(at(ios, 0x24)).cast_signed();
            let mut count = if hi < 0 || (hi == 0 && lo == 0) || lo >= 0xffff_fffe {
                0xffff_fffe
            } else {
                lo
            };
            let sb: u32 = h.read(at(ios, 0x38));
            let mut c = import(h, SGETC, Conv::Thiscall(sb), &[]);
            while count != 0 {
                if c == EOF {
                    state |= EOF_BIT;
                    break;
                }
                let table: u32 = h.read(at(ctype, 0xc));
                if h.read::<u8>(at(table, (c & 0xff) * 2)) & 0x48 != 0 {
                    break;
                }
                h.call_sym(STRING_PUSH_BACK, Conv::Thiscall(s), &[c & 0xff, c]);
                changed = true;
                count -= 1;
                let sb = rdbuf(h, is);
                c = import(h, SNEXTC, Conv::Thiscall(sb), &[]);
            }
        }
        let ios = ios_of(h, is);
        h.write(at(ios, 0x20), 0u32);
        h.write(at(ios, 0x24), 0u32);
        if !changed {
            state |= FAIL;
        }
        let ios = ios_of(h, is);
        import(h, SETSTATE, Conv::Thiscall(ios), &[state, 0]);
        let sb = rdbuf(h, is);
        if sb != 0 {
            vcall(h, sb, 8, &[]);
        }
        is
    }

    /// `0x004049a0(loc)` (fastcall: `ecx` the `std::locale`):
    /// `use_facet<ctype<char>>(loc)` under a `_Lockit(0)`: the locale's
    /// facet for `ctype<char>::id`, else the global locale's when the
    /// locale is transparent, else the cached one ([`CTYPE_CACHE`]); else
    /// `ctype<char>::_Getcat(&cache, &loc)` makes one (-1: `bad_cast("bad
    /// cast")` thrown; it never returns), cached, `_Incref` (slot 1) and
    /// registered ([`FACET_REGISTER`]). `frame` is the original's 0x14
    /// bytes at `ebp - 0x20`: the `bad_cast` (+0), the `_Lockit` (+0xc),
    /// the cache copy `_Getcat` fills (+0x10). Returns the facet.
    pub fn use_ctype<H: Host>(h: &mut H, loc: u32, frame: u32) -> u32 {
        let lockit = at(frame, 0xc);
        let saved = at(frame, 0x10);
        import(h, LOCKIT_CTOR, Conv::Thiscall(lockit), &[0]);
        let cache: u32 = h.read(h.rebase(CTYPE_CACHE));
        h.write(saved, cache);
        let id: u32 = h.read(h.rebase(CTYPE_ID));
        let id = import(h, ID_TO_UINT, Conv::Thiscall(id), &[]);
        let imp: u32 = h.read(loc);
        let lookup = |h: &mut H, imp: u32| -> Option<u32> {
            if id < h.read::<u32>(at(imp, 0xc)) {
                let v: u32 = h.read(at(imp, 8));
                Some(h.read(at(v, id.wrapping_mul(4))))
            } else {
                None
            }
        };
        let mut facet = lookup(h, imp).unwrap_or(0);
        if facet == 0 && h.read::<u8>(at(imp, 0x14)) != 0 {
            let global = import(h, GLOBAL_LOCALE, Conv::Cdecl, &[]);
            if let Some(f) = lookup(h, global) {
                facet = f;
            }
        }
        if facet == 0 {
            facet = h.read(saved);
            if facet == 0 {
                let r = import(h, CTYPE_GETCAT, Conv::Cdecl, &[saved, loc]);
                if r == u32::MAX {
                    let text = h.rebase(BAD_CAST_TEXT);
                    import(h, BAD_CAST_CTOR, Conv::Thiscall(frame), &[text]);
                    let info = h.rebase(BAD_CAST_THROW_INFO);
                    h.call_sym(CXX_THROW, Conv::Cdecl, &[frame, info]);
                }
                facet = h.read(saved);
                h.write(h.rebase(CTYPE_CACHE), facet);
                vcall(h, facet, 4, &[]);
                h.call_sym(FACET_REGISTER, Conv::Cdecl, &[facet]);
            }
        }
        import(h, LOCKIT_DTOR, Conv::Thiscall(lockit), &[]);
        facet
    }

    /// `Facet_Register(f)` (`0x0094a684`, cdecl): a node `{next, facet}`
    /// (`operator new(8)`; none on 0) pushed on the facet list. Returns
    /// the node (0 when the allocation failed, which also empties the
    /// list head as the original does).
    pub fn facet_register(h: &mut impl Host, facet: u32) -> u32 {
        let node = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[8]);
        if node != 0 {
            let head: u32 = h.read(h.rebase(FACET_LIST));
            h.write(node, head);
            h.write(at(node, 4), facet);
        }
        h.write(h.rebase(FACET_LIST), node);
        node
    }

    /// `0x004041c0(this; _, c)` (thiscall, `ret 8`; the first word is not
    /// read): `std::string::push_back(c)`: `_Xlength_error("string too
    /// long")` at the size limit, `_Copy` when the capacity is short, the
    /// character and the terminating 0. Returns `this`.
    pub fn string_push_back<H: Host>(h: &mut H, this: u32, c: u32) -> u32 {
        let size: u32 = h.read(at(this, 0x10));
        if u32::MAX.wrapping_sub(size) <= 1 {
            let t = h.rebase(TOO_LONG_TEXT);
            import(h, XLENGTH_ERROR, Conv::Cdecl, &[t]);
        }
        let new = size.wrapping_add(1);
        if new > 0xffff_fffe {
            let t = h.rebase(TOO_LONG_TEXT);
            import(h, XLENGTH_ERROR, Conv::Cdecl, &[t]);
        }
        let chars = |h: &mut H| -> u32 {
            if h.read::<u32>(at(this, 0x14)) >= 0x10 {
                h.read(this)
            } else {
                this
            }
        };
        if h.read::<u32>(at(this, 0x14)) < new {
            h.call_sym(STRING_COPY, Conv::Thiscall(this), &[new, size]);
        } else if new == 0 {
            h.write(at(this, 0x10), 0u32);
            let p = chars(h);
            h.write(p, 0u8);
            return this;
        }
        let size: u32 = h.read(at(this, 0x10));
        let p = chars(h);
        h.write(at(p, size), c as u8);
        h.write(at(this, 0x10), new);
        let p = chars(h);
        h.write(at(p, new), 0u8);
        this
    }
}
