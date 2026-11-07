//! List `system-b`, round 10 (`tail10`): `AssetListReader::readAssetHeader`
//! (`0x006f4900`, thiscall(reader; path) ret 4), which reads an asset
//! list header (a C header the asset tools write) into the reader.
//!
//! The whole file goes into a `std::string` (1023-byte `fiosRead`
//! chunks). The part from the reader's start marker (the string at
//! `+0x2c`) to `#endif` replaces it; each `const Phyre::PUInt32 NAME = N;`
//! line becomes the symbol map entry `NAME -> N + count` (`count` the
//! asset vector's length before; the last line is never added: the loop
//! stops when no next line follows). Then every quoted path after
//! ` = {`: each line's text from after its `"` up to the character
//! before the line end (`\n` or `\r\n`), that one left out (the closing
//! `"`),
//! through `getPlatformSpecificPath`; a path not yet in the asset vector
//! (`+0x14`, `std::string`s) is appended, and a `.dae.phyre` one also
//! records its index in the model vector (`+8`), unless the line counter
//! lies within the `vfxtex_start`..`vfxtex_end` symbols' range. Those two
//! symbols are erased, and the symbol map's values past `vfxtex_end`
//! shift down over the range.
//!
//! A path shorter than ten characters makes `substr(size - 10, 10)`
//! throw `out_of_range` in the original (nothing catches it); the asset
//! lists hold none.
//!
//! The C++ frame (`FuncInfo` `0x00bfbc3c`) has no try blocks: it only
//! destroys the strings when a callee throws, and nothing in it catches.
//! The strings and the read buffer stay in the frame at the original's
//! `ebp` offsets ([`Frame`]); their addresses go to the string helpers.

use ffx_graphic::host::{Conv, Host};

/// `std::string::assign(p, n)` (thiscall ret 8).
pub const STR_ASSIGN: u32 = 0x0040_4270;
/// `std::string::append(p, n)` (thiscall ret 8).
pub const STR_APPEND: u32 = 0x0065_76d0;
/// `std::string::find(p, pos, n)` (thiscall ret 0xc) -> index or -1.
pub const STR_FIND: u32 = 0x0040_3bd0;
/// `std::string::substr(out, pos, n)` (thiscall ret 0xc) -> out.
pub const STR_SUBSTR: u32 = 0x0040_2e20;
/// `std::string::operator=(string&&)` (thiscall ret 4).
pub const STR_ASSIGN_MOVE: u32 = 0x0040_2d60;
/// `std::string::assign(src, pos, n)` (thiscall ret 0xc).
pub const STR_ASSIGN_SUBSTR: u32 = 0x0040_3a10;
/// The move assignment `0x004039b0` (thiscall ret 4).
pub const STR_MOVE_ASSIGN: u32 = 0x0040_39b0;
/// `std::string(const char *)` (thiscall ret 4).
pub const STR_CTOR: u32 = 0x0040_2c90;
/// `~std::string` (ecx).
pub const STR_DTOR: u32 = 0x0040_2da0;
/// `operator delete` (cdecl).
pub const DELETE: u32 = 0x0063_0390;
/// `memcmp_00401020(a, b, n)` (cdecl).
pub const MEMCMP: u32 = 0x0040_1020;
/// `atoi`'s import slot (cdecl).
pub const ATOI_SLOT: u32 = 0x00b0_c40c;
/// `fiosOpen(name, mode)` (cdecl).
pub const FIOS_OPEN: u32 = 0x0067_9730;
/// `fiosRead(file, buffer, n)` (cdecl).
pub const FIOS_READ: u32 = 0x0067_9800;
/// `fiosClose(file)` (cdecl).
pub const FIOS_CLOSE: u32 = 0x0067_9510;
/// `report_rangecheckfailure` (never returns).
pub const RANGE_CHECK: u32 = 0x0094_9b02;
/// The symbol map's `operator[](key)` (thiscall ret 4) -> the value's
/// address.
pub const MAP_INDEX: u32 = 0x006f_3ae0;
/// `AssetListReader::getSymbolId(name)` (thiscall ret 4).
pub const GET_SYMBOL_ID: u32 = 0x006f_4350;
/// `AssetListReader_eraseKey_006f40e0(key)` (thiscall ret 4).
pub const ERASE_KEY: u32 = 0x006f_40e0;
/// `std::vector<std::string>::push_back(&s)` (thiscall ret 4).
pub const PUSH_STRING: u32 = 0x0066_7770;
/// `std::vector<word>::push_back(&v)` (thiscall ret 4).
pub const PUSH_WORD: u32 = 0x006f_4890;
/// `AssetListReader::getPlatformSpecificPath(out, std::string by value)`
/// (cdecl, seven words; the callee destroys its string).
pub const PLATFORM_PATH: u32 = 0x0069_0590;

/// `""`.
pub const EMPTY: u32 = 0x00b8_fbeb;
/// `"#endif"`.
pub const ENDIF: u32 = 0x00b4_bc34;
/// `"const Phyre::PUInt32"`.
pub const PUINT32: u32 = 0x00b4_bc3c;
/// `" = "`.
pub const EQUALS: u32 = 0x00b4_bc54;
/// `";"`.
pub const SEMICOLON: u32 = 0x00b8_f880;
/// `" = {"`: the path table's start.
pub const TABLE: u32 = 0x00b4_bc58;
/// `"\""`.
pub const QUOTE: u32 = 0x00b4_bc60;
/// `"\n"`.
pub const NEWLINE: u32 = 0x00b7_cae0;
/// `"vfxtex_start"`.
pub const VFXTEX_START: u32 = 0x00b4_bc64;
/// `"vfxtex_end"`.
pub const VFXTEX_END: u32 = 0x00b4_bc74;
/// `".dae.phyre"`.
pub const DAE_PHYRE: u32 = 0x00b4_bc80;

/// The frame's part the callees see, from `[ebp-0x48c]` (its address is
/// the bridge's `frame`): the index word pushed to the model vector,
/// three temporary strings, the file text and the read buffer.
pub mod frame {
    /// `[ebp-0x48c]`: the model index.
    pub const INDEX: u32 = 0;
    /// `[ebp-0x470]`: the name / path temporary.
    pub const T1: u32 = 0x1c;
    /// `[ebp-0x458]`: the value / extension temporary.
    pub const T2: u32 = 0x34;
    /// `[ebp-0x440]`: the line's path.
    pub const T3: u32 = 0x4c;
    /// `[ebp-0x428]`: the file text.
    pub const TEXT: u32 = 0x64;
    /// `[ebp-0x410]`: the read buffer (0x400 bytes).
    pub const BUF: u32 = 0x7c;
    /// The bytes from `[ebp-0x48c]` up to the cookie at `[ebp-0x10]`.
    pub const BYTES: u32 = 0x47c;
}

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // h, f, s, t, p
pub mod bridge {
    use super::{
        ATOI_SLOT, Conv, DAE_PHYRE, DELETE, EMPTY, ENDIF, EQUALS, ERASE_KEY, FIOS_CLOSE, FIOS_OPEN,
        FIOS_READ, GET_SYMBOL_ID, Host, MAP_INDEX, MEMCMP, NEWLINE, PLATFORM_PATH, PUINT32,
        PUSH_STRING, PUSH_WORD, QUOTE, RANGE_CHECK, SEMICOLON, STR_APPEND, STR_ASSIGN,
        STR_ASSIGN_MOVE, STR_ASSIGN_SUBSTR, STR_CTOR, STR_DTOR, STR_FIND, STR_MOVE_ASSIGN,
        STR_SUBSTR, TABLE, VFXTEX_END, VFXTEX_START, frame,
    };

    fn this_call(h: &mut impl Host, sym: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Thiscall(this), args)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `s.find(text, pos, n)` on the string at `s`.
    fn find(h: &mut impl Host, s: u32, text: u32, pos: u32, n: u32) -> u32 {
        let text = h.rebase(text);
        this_call(h, STR_FIND, s, &[text, pos, n])
    }

    /// An MSVC `std::string`'s size (`+0x10`).
    fn size(h: &impl Host, s: u32) -> u32 {
        h.read(s.wrapping_add(0x10))
    }

    /// Its capacity (`+0x14`).
    fn cap(h: &impl Host, s: u32) -> u32 {
        h.read(s.wrapping_add(0x14))
    }

    /// Its text: inline below capacity 16, else the heap pointer.
    fn data(h: &impl Host, s: u32) -> u32 {
        if cap(h, s) < 0x10 { s } else { h.read(s) }
    }

    /// The inlined destructor: the heap buffer freed (capacity 16 and up).
    fn free(h: &mut impl Host, s: u32) {
        if cap(h, s) >= 0x10 {
            let p: u32 = h.read(s);
            cdecl(h, DELETE, &[p]);
        }
    }

    /// The inlined clear after a free: capacity 15, size 0, an empty text.
    fn reset(h: &mut impl Host, s: u32) {
        h.write(s.wrapping_add(0x14), 0xf_u32);
        h.write(s.wrapping_add(0x10), 0_u32);
        h.write(s, 0_u8);
    }

    /// The end of the line from `pos`: one before the `\n` (two when a
    /// `\r` precedes it).
    fn line_end(h: &mut impl Host, text: u32, pos: u32) -> u32 {
        let nl = find(h, text, NEWLINE, pos, 1);
        let mut end = nl.wrapping_sub(1);
        if h.read::<u8>(data(h, text).wrapping_add(end)) == b'\r' {
            end = end.wrapping_sub(1);
        }
        end
    }

    /// The asset vector's length (`(end - begin) / 0x18`, signed).
    fn count(h: &impl Host, v: u32) -> i32 {
        let begin: u32 = h.read(v);
        let end: u32 = h.read(v.wrapping_add(4));
        end.wrapping_sub(begin).cast_signed() / 0x18
    }

    /// `AssetListReader::readAssetHeader(reader; path)` with its frame at
    /// `f` ([`frame`], [`super::frame::BYTES`] bytes) and the by-value
    /// string `getPlatformSpecificPath` takes built at `arg` (six words).
    #[allow(clippy::too_many_lines)] // one original
    pub fn read_asset_header(h: &mut impl Host, this: u32, path: u32, f: u32, arg: u32) {
        let t1 = f.wrapping_add(frame::T1);
        let t2 = f.wrapping_add(frame::T2);
        let t3 = f.wrapping_add(frame::T3);
        let text = f.wrapping_add(frame::TEXT);
        let buf = f.wrapping_add(frame::BUF);
        let assets = this.wrapping_add(0x14);
        let base = count(h, assets);
        h.write(text.wrapping_add(0x14), 0xf_u32);
        h.write(text.wrapping_add(0x10), 0_u32);
        h.write(text, 0_u8);
        let empty = h.rebase(EMPTY);
        this_call(h, STR_ASSIGN, text, &[empty, 0]);
        let file = cdecl(h, FIOS_OPEN, &[path, 1]);
        loop {
            let n = cdecl(h, FIOS_READ, &[file, buf, 0x3ff]);
            if n.cast_signed() <= 0 {
                break;
            }
            if n >= 0x400 {
                // never returns
                cdecl(h, RANGE_CHECK, &[]);
                return;
            }
            h.write(buf.wrapping_add(n), 0_u8);
            let mut len = 0u32;
            while h.read::<u8>(buf.wrapping_add(len)) != 0 {
                len = len.wrapping_add(1);
            }
            this_call(h, STR_APPEND, text, &[buf, len]);
        }
        cdecl(h, FIOS_CLOSE, &[file]);
        body(h, this, text, [t1, t2, t3], f, arg, base);
        free(h, text);
    }

    /// Everything after the file is read: the header's part, the symbols,
    /// the paths, the range shift.
    #[allow(clippy::too_many_lines)] // one original
    fn body(h: &mut impl Host, this: u32, text: u32, t: [u32; 3], f: u32, arg: u32, base: i32) {
        let [t1, t2, t3] = t;
        let assets = this.wrapping_add(0x14);
        let marker = this.wrapping_add(0x2c);
        let marker_len: u32 = h.read(this.wrapping_add(0x3c));
        let marker_text = data(h, marker);
        let start = this_call(h, STR_FIND, text, &[marker_text, 0, marker_len]);
        if start == u32::MAX {
            return;
        }
        let endif = find(h, text, ENDIF, start, 6);
        if endif == u32::MAX {
            return;
        }
        let part = this_call(h, STR_SUBSTR, text, &[t1, start, endif.wrapping_sub(start)]);
        this_call(h, STR_ASSIGN_MOVE, text, &[part]);
        free(h, t1);

        // the symbols: NAME -> N + base, each but the last
        let mut at = find(h, text, PUINT32, 0, 0x14);
        if at != u32::MAX {
            loop {
                let name = at.wrapping_add(0x15);
                let eq = find(h, text, EQUALS, name, 3);
                this_call(h, STR_SUBSTR, text, &[t1, name, eq.wrapping_sub(name)]);
                let value = eq.wrapping_add(3);
                let semi = find(h, text, SEMICOLON, value, 1);
                this_call(h, STR_SUBSTR, text, &[t2, value, semi.wrapping_sub(value)]);
                let atoi: u32 = h.read(h.rebase(ATOI_SLOT));
                let digits = data(h, t2);
                let id = h
                    .call(atoi, Conv::Cdecl, &[digits])
                    .wrapping_add(base.cast_unsigned());
                at = find(h, text, PUINT32, semi, 0x14);
                if at == u32::MAX {
                    break;
                }
                let slot = this_call(h, MAP_INDEX, this, &[t1]);
                h.write(slot, id);
                free(h, t2);
                reset(h, t2);
                free(h, t1);
            }
            h.call_sym(STR_DTOR, Conv::Thiscall(t2), &[]);
            h.call_sym(STR_DTOR, Conv::Thiscall(t1), &[]);
        }

        // the table
        let table = find(h, text, TABLE, 0, 4);
        let quote = find(h, text, QUOTE, table, 1);
        let mut from = quote.wrapping_add(1);
        let mut end = line_end(h, text, from);
        let mut line = 0i32;
        let vstart = h.rebase(VFXTEX_START);
        let first = this_call(h, GET_SYMBOL_ID, this, &[vstart]).cast_signed();
        let vend = h.rebase(VFXTEX_END);
        let mut last = this_call(h, GET_SYMBOL_ID, this, &[vend]).cast_signed();
        if last == -1 {
            last = first;
        }
        if first != -1 && last != -1 {
            for name in [vstart, vend] {
                this_call(h, STR_CTOR, t1, &[name]);
                this_call(h, ERASE_KEY, this, &[t1]);
                h.call_sym(STR_DTOR, Conv::Thiscall(t1), &[]);
            }
        }
        if table != u32::MAX {
            loop {
                this_call(h, STR_SUBSTR, text, &[t3, from, end.wrapping_sub(from)]);
                // the by-value copy of the line
                h.write(arg.wrapping_add(0x14), 0xf_u32);
                h.write(arg.wrapping_add(0x10), 0_u32);
                h.write(arg, 0_u8);
                this_call(h, STR_ASSIGN_SUBSTR, arg, &[t3, 0, u32::MAX]);
                let mut words = [0u32; 7];
                words[0] = t1;
                for (i, w) in (0u32..).zip(&mut words[1..]) {
                    *w = h.read(arg.wrapping_add(4 * i));
                }
                let p = cdecl(h, PLATFORM_PATH, &words);
                if p != t3 {
                    free(h, t3);
                    reset(h, t3);
                    this_call(h, STR_MOVE_ASSIGN, t3, &[p]);
                }
                free(h, t1);
                if !known(h, this, t3) && (line < first || line > last) {
                    add(h, this, assets, t2, t3, f);
                }
                let q = find(h, text, QUOTE, end.wrapping_add(1), 1);
                if q == u32::MAX {
                    free(h, t3);
                    break;
                }
                from = q.wrapping_add(1);
                line = line.wrapping_add(1);
                end = line_end(h, text, from);
                free(h, t3);
            }
        }

        // the symbols past the range shift down over it
        if first != -1 && last != -1 {
            let head: u32 = h.read(this);
            let mut node: u32 = h.read(head);
            while node != h.read::<u32>(this) {
                let v: i32 = h.read(node.wrapping_add(0x28));
                if v > last {
                    h.write(
                        node.wrapping_add(0x28),
                        v.wrapping_sub(last).wrapping_sub(1).wrapping_add(first),
                    );
                }
                node = next(h, node);
            }
        }
    }

    /// Whether the asset vector already holds the path `s`.
    fn known(h: &mut impl Host, this: u32, s: u32) -> bool {
        let assets = this.wrapping_add(0x14);
        let mut e: u32 = h.read(assets);
        let end: u32 = h.read(this.wrapping_add(0x18));
        while e != end {
            let len = size(h, s);
            let text = data(h, s);
            let n: u32 = h.read(e.wrapping_add(0x10));
            let other = data(h, e);
            let r = cdecl(h, MEMCMP, &[other, text, n.min(len)]);
            if r == 0 && n >= len && n == len {
                return true;
            }
            e = e.wrapping_add(0x18);
        }
        false
    }

    /// Appends the path `s` to the asset vector, and its index to the
    /// model vector when it ends in `.dae.phyre`.
    fn add(h: &mut impl Host, this: u32, assets: u32, t2: u32, s: u32, f: u32) {
        this_call(h, PUSH_STRING, assets, &[s]);
        let len = size(h, s);
        this_call(h, STR_SUBSTR, s, &[t2, len.wrapping_sub(10), 10]);
        let n = size(h, t2);
        let text = data(h, t2);
        let dae = h.rebase(DAE_PHYRE);
        let r = cdecl(h, MEMCMP, &[text, dae, n.min(10)]);
        // compare(".dae.phyre") == 0: equal bytes and equal length
        if r == 0 && n == 10 {
            let index = count(h, assets).wrapping_sub(1);
            let at = f.wrapping_add(frame::INDEX);
            h.write(at, index);
            this_call(h, PUSH_WORD, this.wrapping_add(8), &[at]);
        }
        free(h, t2);
    }

    /// The in-order successor of a red-black tree node (`+0` left, `+4`
    /// parent, `+8` right, `+0xd` the nil flag).
    fn next(h: &impl Host, node: u32) -> u32 {
        let nil = |p: u32| h.read::<u8>(p.wrapping_add(0xd)) != 0;
        if nil(node) {
            return node;
        }
        let right: u32 = h.read(node.wrapping_add(8));
        if !nil(right) {
            let mut n = right;
            loop {
                let left: u32 = h.read(n);
                if nil(left) {
                    return n;
                }
                n = left;
            }
        }
        let mut child = node;
        let mut parent: u32 = h.read(node.wrapping_add(4));
        while !nil(parent) && child == h.read::<u32>(parent.wrapping_add(8)) {
            child = parent;
            parent = h.read(parent.wrapping_add(4));
        }
        parent
    }
}
