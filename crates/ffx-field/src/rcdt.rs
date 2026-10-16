//! `rcdt`: data chunks and the text parser over them (`docs/formats/field.md`).
//!
//! A chunk is a span of loaded data; a parser walks one with a cursor and
//! splits tokens at separator bytes.

/// A data chunk (`rcdtChunkInit`, 8 bytes).
pub mod chunk {
    /// `+0x0`: the data (`rcdtChunkGetBegin`).
    pub const BEGIN: u32 = 0x0;
    /// `+0x4`: its size; `0x7fff_ffff` when set from a bare pointer
    /// (`rcdtChunkSetBin`).
    pub const SIZE: u32 = 0x4;
    /// The size `rcdtChunkSetBin` gives a chunk: unbounded.
    pub const UNBOUNDED: u32 = 0x7fff_ffff;
}

/// A parser (`rcdtParseInit`).
pub mod parse {
    /// `+0x0`: the chunk it reads.
    pub const CHUNK: u32 = 0x0;
    /// `+0x4`: the cursor, a pointer into the chunk's data.
    pub const CURSOR: u32 = 0x4;
    /// `+0x8`: the mode (`rcdtParseSetMode`), 0 after `rcdtParseInit`.
    pub const MODE: u32 = 0x8;
    /// `+0xc`: 128 bytes, one per ASCII code: non-zero for a separator.
    /// Token scanners (`0x0092bd20`, `0x0092be40`) index it with the byte
    /// at the cursor.
    pub const SEPARATORS: u32 = 0xc;
    /// Entries in [`SEPARATORS`].
    pub const SEPARATOR_COUNT: u32 = 0x80;
}

/// Whether byte `c` separates tokens by default (`rcdtParseInitSeparators`):
/// the control characters and the space.
#[must_use]
pub const fn default_separator(c: u8) -> bool {
    c <= 0x20
}

/// The parser where the game keeps it, through [`ffx_bridge::Mem`].
/// MSVCR110's `atoi` import slot (`0x00b0c40c`).
pub const CRT_ATOI: u32 = 0x00b0_c40c;
/// MSVCR110's `atof` import slot (`0x00b0c408`).
pub const CRT_ATOF: u32 = 0x00b0_c408;
/// MSVCR110's `strncpy` import slot (`0x00b0c368`).
pub const CRT_STRNCPY: u32 = 0x00b0_c368;
/// `_strnicmp`'s import thunk (`0x0094993c`).
pub const CRT_STRNICMP: u32 = 0x0094_993c;

pub mod bridge {
    use ffx_bridge::Mem;

    use super::{chunk, default_separator, parse};

    /// `rcdtChunkInit(chunk)` (`0x0092b840`): empty.
    pub fn rcdt_chunk_init(mem: &mut impl Mem, c: u32) {
        mem.write(c + chunk::BEGIN, 0u32);
        mem.write(c + chunk::SIZE, 0u32);
    }

    /// `rcdtChunkGetBegin(chunk)` (`0x0092b810`).
    pub fn rcdt_chunk_get_begin(mem: &impl Mem, c: u32) -> u32 {
        mem.read(c + chunk::BEGIN)
    }

    /// `rcdtChunkSetBin(chunk, data)` (`0x0092b940`): the chunk over `data`,
    /// unbounded.
    pub fn rcdt_chunk_set_bin(mem: &mut impl Mem, c: u32, data: u32) {
        mem.write(c + chunk::BEGIN, data);
        mem.write(c + chunk::SIZE, chunk::UNBOUNDED);
    }

    /// `rcdtParseAdvance(parse, n)` (`0x0092b960`): moves the cursor `n`
    /// bytes on.
    pub fn rcdt_parse_advance(mem: &mut impl Mem, p: u32, n: u32) {
        let at = mem.read::<u32>(p + parse::CURSOR);
        mem.write(p + parse::CURSOR, at.wrapping_add(n));
    }

    /// `rcdtParseGetBegin(parse)` (`0x0092ba70`): its chunk's data.
    pub fn rcdt_parse_get_begin(mem: &impl Mem, p: u32) -> u32 {
        let c = mem.read::<u32>(p + parse::CHUNK);
        rcdt_chunk_get_begin(mem, c)
    }

    /// `rcdtParseSet(parse, chunk)` (`0x0092c0a0`): reads `chunk` from its
    /// start.
    pub fn rcdt_parse_set(mem: &mut impl Mem, p: u32, c: u32) {
        mem.write(p + parse::CHUNK, c);
        let begin = rcdt_chunk_get_begin(mem, c);
        mem.write(p + parse::CURSOR, begin);
    }

    /// `rcdtParseSetPos(parse, offset)` (`0x0092c0d0`): the cursor `offset`
    /// bytes into the data.
    pub fn rcdt_parse_set_pos(mem: &mut impl Mem, p: u32, offset: u32) {
        let begin = rcdt_parse_get_begin(mem, p);
        mem.write(p + parse::CURSOR, begin.wrapping_add(offset));
    }

    /// `rcdtParseGetBin(parse, out, n)` (`0x0092ba90`): copies `n` bytes at
    /// the cursor to `out` and moves past them.
    pub fn rcdt_parse_get_bin(mem: &mut impl Mem, p: u32, out: u32, n: u32) {
        let at = mem.read::<u32>(p + parse::CURSOR);
        let mut bytes = vec![0u8; n as usize];
        mem.read_bytes(at, &mut bytes);
        mem.write_bytes(out, &bytes);
        rcdt_parse_advance(mem, p, n);
    }

    /// `rcdtParseInitSeparators(parse)` (`0x0092c000`): bytes `0x00..=0x20`
    /// separate tokens, no others.
    pub fn rcdt_parse_init_separators(mem: &mut impl Mem, p: u32) {
        let table: [u8; parse::SEPARATOR_COUNT as usize] =
            std::array::from_fn(|c| u8::from(default_separator(u8::try_from(c).unwrap_or(0xff))));
        mem.write_bytes(p + parse::SEPARATORS, &table);
    }

    /// `rcdtParseSetMode(parse, mode)` (`0x0092c0c0`).
    pub fn rcdt_parse_set_mode(mem: &mut impl Mem, p: u32, mode: u32) {
        mem.write(p + parse::MODE, mode);
    }

    /// `rcdtParseAddSeparators(parse, chars)` (`0x0092c0f0`): every byte
    /// of the C string `chars` becomes a separator (the table is indexed
    /// unchecked).
    pub fn rcdt_parse_add_separators(mem: &mut impl Mem, p: u32, chars: u32) {
        let mut at = chars;
        loop {
            let c = mem.read::<u8>(at);
            if c == 0 {
                break;
            }
            mem.write(p + parse::SEPARATORS + u32::from(c), 1u8);
            at = at.wrapping_add(1);
        }
    }

    /// `rcdtChunkSet(chunk, data, size)` (`0x0092b920`).
    pub fn rcdt_chunk_set(mem: &mut impl Mem, c: u32, data: u32, size: u32) {
        mem.write(c + chunk::BEGIN, data);
        mem.write(c + chunk::SIZE, size);
    }

    /// `rcdtChunkGetEnd(chunk)` (`0x0092b820`): data + size.
    pub fn rcdt_chunk_get_end(mem: &impl Mem, c: u32) -> u32 {
        let size = mem.read::<u32>(c + chunk::SIZE);
        size.wrapping_add(mem.read(c + chunk::BEGIN))
    }

    /// `rcdtChunkIsEmpty(chunk)` (`0x0092b860`): no data or no size.
    pub fn rcdt_chunk_is_empty(mem: &impl Mem, c: u32) -> bool {
        mem.read::<u32>(c + chunk::BEGIN) == 0 || mem.read::<u32>(c + chunk::SIZE) == 0
    }

    /// `rcdtChunkGetSize(chunk)` (`0x0092b830`).
    pub fn rcdt_chunk_get_size(mem: &impl Mem, c: u32) -> u32 {
        mem.read(c + chunk::SIZE)
    }

    /// `rcdtParseGetChunk(parse)` (`0x0092bb20`).
    pub fn rcdt_parse_get_chunk(mem: &impl Mem, p: u32) -> u32 {
        mem.read(p + parse::CHUNK)
    }

    /// `rcdtParseGetPos(parse)` (`0x0092bda0`): the cursor.
    pub fn rcdt_parse_get_pos(mem: &impl Mem, p: u32) -> u32 {
        mem.read(p + parse::CURSOR)
    }

    /// `rcdtParseGetMode(parse)` (`0x0092bd10`).
    pub fn rcdt_parse_get_mode(mem: &impl Mem, p: u32) -> u32 {
        mem.read(p + parse::MODE)
    }

    /// `rcdtTm2Init(tm2)` (`0x009237e0`): a TIM2 texture holder, `0x50`
    /// bytes, cleared.
    pub fn rcdt_tm2_init(mem: &mut impl Mem, t: u32) {
        mem.write_bytes(t, &[0; 0x50]);
    }

    /// `rcdtParseInit(parse)` (`0x0092bf50`): no chunk, default separators.
    pub fn rcdt_parse_init(mem: &mut impl Mem, p: u32) {
        mem.write(p + parse::CHUNK, 0u32);
        mem.write(p + parse::CURSOR, 0u32);
        mem.write(p + parse::MODE, 0u32);
        rcdt_parse_init_separators(mem, p);
    }

    /// `rcdtChunkHandleGetEnd(&chunk)` (`0x0092bb90`):
    /// [`rcdt_chunk_get_end`] of the chunk a handle points to.
    pub fn rcdt_chunk_handle_get_end(mem: &impl Mem, h: u32) -> u32 {
        let c = mem.read::<u32>(h);
        rcdt_chunk_get_end(mem, c)
    }

    /// `rcdtParseIsEnd(parse)` (`0x0092bfb0`): 1 once the position
    /// (`+4`) reaches the chunk's end ([`rcdt_chunk_handle_get_end`]).
    pub fn rcdt_parse_is_end(mem: &impl Mem, p: u32) -> u32 {
        let end = rcdt_chunk_handle_get_end(mem, p);
        u32::from(mem.read::<u32>(p.wrapping_add(4)) >= end)
    }

    /// `rcdtParseSkipSeparators(parse)` (`0x0092bd60`): the first byte at
    /// or after the position that is not a separator (the 256-byte table
    /// at `+0xc`), or the end.
    pub fn rcdt_parse_skip_separators(mem: &impl Mem, p: u32) -> u32 {
        scan_while(mem, p, |mem, b| {
            mem.read::<u8>(p.wrapping_add(0xc).wrapping_add(u32::from(b))) != 0
        })
    }

    /// `rcdtParseSkipToken(parse)` (`0x0092bd20`): the first separator at
    /// or after the position, or the end.
    pub fn rcdt_parse_skip_token(mem: &impl Mem, p: u32) -> u32 {
        scan_while(mem, p, |mem, b| {
            mem.read::<u8>(p.wrapping_add(0xc).wrapping_add(u32::from(b))) == 0
        })
    }

    /// `rcdtParseFindChar(parse, c)` (`0x0092bae0`): the first `c` (a
    /// sign-extended char compared with the zero-extended byte) at or
    /// after the position, or the end.
    pub fn rcdt_parse_find_char(mem: &impl Mem, p: u32, c: i8) -> u32 {
        let want = i32::from(c).cast_unsigned();
        scan_while(mem, p, |_, b| u32::from(b) != want)
    }

    fn scan_while<M: Mem>(mem: &M, p: u32, go: impl Fn(&M, u8) -> bool) -> u32 {
        let mut at = mem.read::<u32>(p.wrapping_add(4));
        let mut b = mem.read::<u8>(at);
        while go(mem, b) {
            if at >= rcdt_chunk_handle_get_end(mem, p) {
                break;
            }
            b = mem.read::<u8>(at.wrapping_add(1));
            at = at.wrapping_add(1);
        }
        at
    }

    /// `rcdtChunkFini(chunk)` (`0x0092b7f0`): frees the chunk's data.
    pub fn rcdt_chunk_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        c: u32,
        call: &mut C,
    ) -> u32 {
        crate::model::bridge::free_field(mem, call, c)
    }

    /// `rcdtChunkDelete(chunk)` (`0x0092b7d0`): [`rcdt_chunk_fini`], then
    /// the chunk freed.
    pub fn rcdt_chunk_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        c: u32,
        call: &mut C,
    ) -> u32 {
        rcdt_chunk_fini(mem, c, call);
        crate::model::bridge::rc_free(mem, call, c)
    }

    /// `rcdtParseNextToken(parse)` (`0x0092ba30`): the cursor past the
    /// token ([`rcdt_parse_skip_token`]) and then the separators
    /// ([`rcdt_parse_skip_separators`]). Returns the new cursor.
    pub fn rcdt_parse_next_token(mem: &mut impl Mem, p: u32) -> u32 {
        let at = rcdt_parse_skip_token(mem, p);
        mem.write(p.wrapping_add(4), at);
        let at = rcdt_parse_skip_separators(mem, p);
        mem.write(p.wrapping_add(4), at);
        at
    }

    /// `rcdtParseTokenLen(parse)` (`0x0092bf30`): the current token's
    /// length.
    pub fn rcdt_parse_token_len(mem: &impl Mem, p: u32) -> u32 {
        rcdt_parse_skip_token(mem, p).wrapping_sub(mem.read::<u32>(p.wrapping_add(4)))
    }

    /// `rcdtParseTokenLenText(parse)` (`0x0092be10`): the token's length
    /// in text mode (`+8` 1), else 0.
    pub fn rcdt_parse_token_len_text(mem: &impl Mem, p: u32) -> u32 {
        if mem.read::<u32>(p.wrapping_add(8)) == 1 {
            rcdt_parse_token_len(mem, p)
        } else {
            0
        }
    }

    /// `rcdtParseSeekChar(parse, c)` (`0x0092b990`): the cursor moved to
    /// `c` ([`rcdt_parse_find_char`]). Returns it.
    pub fn rcdt_parse_seek_char(mem: &mut impl Mem, p: u32, c: i8) -> u32 {
        let at = rcdt_parse_find_char(mem, p, c);
        mem.write(p.wrapping_add(4), at);
        at
    }

    /// `rcdtTm2Fini(tm2)` (`0x00923470`): its two buffers (`+0x40`,
    /// `+0x44`) freed (`rcFree`) and cleared.
    pub fn rcdt_tm2_fini<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        call: &mut C,
    ) {
        for o in [0x40u32, 0x44] {
            let p = mem.read::<u32>(t + o);
            if p != 0 {
                crate::model::bridge::rc_free(mem, call, p);
            }
            mem.write(t + o, 0u32);
        }
    }

    /// `rcdtTm2New()` (`0x00923b40`): [`rcdt_tm2_init`] on a new 0x50-byte
    /// block (`rcAlloc`).
    pub fn rcdt_tm2_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let t = crate::model::bridge::rc_alloc(mem, call, 0x50);
        rcdt_tm2_init(mem, t);
        t
    }

    /// `rcdtParseNew()` (`0x0092bfd0`): [`rcdt_parse_init`] on a new
    /// 0x8c-byte parser (`rcAlloc`).
    pub fn rcdt_parse_new<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let p = crate::model::bridge::rc_alloc(mem, call, 0x8c);
        rcdt_parse_init(mem, p);
        p
    }

    /// `rcdtParseSkipLine(parse)` (`0x0092b970`): to the next newline
    /// ([`rcdt_parse_seek_char`]) and past it.
    pub fn rcdt_parse_skip_line(mem: &mut impl Mem, p: u32) {
        rcdt_parse_seek_char(mem, p, 10);
        let at = mem.read::<u32>(p + 4);
        mem.write(p + 4, at.wrapping_add(1));
    }

    /// `rcdtParseGetInt(parse)` (`0x0092bc50`): binary mode: the word at
    /// the cursor, which moves past it; text mode: the token through the
    /// CRT's `atoi` (its import slot `0x00b0c40c`), then the next token
    /// ([`rcdt_parse_next_token`]).
    pub fn rcdt_parse_get_int<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        call: &mut C,
    ) -> u32 {
        if mem.read::<u32>(p + parse::MODE) == 0 {
            let v = mem.read::<u32>(mem.read::<u32>(p + parse::CURSOR));
            rcdt_parse_advance(mem, p, 4);
            return v;
        }
        let atoi = mem.read::<u32>(mem.rebase(super::CRT_ATOI));
        let at = mem.read::<u32>(p + parse::CURSOR);
        let v = call(mem, atoi, &[at]);
        rcdt_parse_next_token(mem, p);
        v
    }

    /// `rcdtParseAdvanceLine(parse)` (`0x0092b9b0`): past the line
    /// ([`rcdt_parse_skip_line`]) and the separators after it.
    pub fn rcdt_parse_advance_line(mem: &mut impl Mem, p: u32) {
        rcdt_parse_skip_line(mem, p);
        let at = rcdt_parse_skip_separators(mem, p);
        mem.write(p + 4, at);
    }

    /// `rcdtParseGetKeyword(parse, table)` (`0x0092be40`): the index of the
    /// token in the null-ended table of strings (same length, `_strnicmp`
    /// through its import thunk `0x0094993c`), else -1; then the next
    /// token. Without a token (a separator skipped first): -1, the cursor
    /// left there.
    pub fn rcdt_parse_get_keyword<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        table: u32,
        call: &mut C,
    ) -> u32 {
        let c = mem.read::<u8>(mem.read::<u32>(p + 4));
        if mem.read::<u8>(p.wrapping_add(0xc).wrapping_add(u32::from(c))) != 0 {
            let at = rcdt_parse_skip_separators(mem, p);
            mem.write(p + 4, at);
        }
        let n = rcdt_parse_token_len(mem, p);
        if n.cast_signed() < 1 {
            return u32::MAX;
        }
        let strnicmp = mem.rebase(super::CRT_STRNICMP);
        let mut i = 0u32;
        let found = loop {
            let s = mem.read::<u32>(table.wrapping_add(4 * i));
            if s == 0 {
                break u32::MAX;
            }
            let mut len = 0u32;
            while mem.read::<u8>(s.wrapping_add(len)) != 0 {
                len += 1;
            }
            if len == n {
                let cur = mem.read::<u32>(p + 4);
                if call(mem, strnicmp, &[cur, s, n]) == 0 {
                    break i;
                }
            }
            i += 1;
        };
        rcdt_parse_next_token(mem, p);
        found
    }

    /// `rcdtParseGetFloat(parse)` (`0x0092bbb0`): binary mode: the float at
    /// the cursor, which moves past it; text mode: the token through the
    /// CRT's `atof` (import slot `0x00b0c408`, a double in `ST0`, through
    /// `fcall`) rounded to `f32`, then the next token.
    pub fn rcdt_parse_get_float<M: Mem>(
        mem: &mut M,
        p: u32,
        fcall: &mut impl FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    ) -> f32 {
        if mem.read::<u32>(p + parse::MODE) == 0 {
            let v = crate::x87::copy(mem.read::<u32>(mem.read::<u32>(p + parse::CURSOR)));
            rcdt_parse_advance(mem, p, 4);
            return f32::from_bits(v);
        }
        let atof = mem.read::<u32>(mem.rebase(super::CRT_ATOF));
        let at = mem.read::<u32>(p + parse::CURSOR);
        let v =
            ffx_bridge::x87::Reg::constant(fcall(mem, atof, crate::camera::FArgs::Stack(&[at])))
                .store();
        rcdt_parse_next_token(mem, p);
        v
    }

    /// `rcdtParseGetText(parse, out)` (`0x0092bdd0`): in text mode the
    /// token copied to `out` (the CRT's `strncpy`, import slot
    /// `0x00b0c368`) and ended, then the next token; returns its length (0
    /// in binary mode, nothing copied).
    pub fn rcdt_parse_get_text<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        out: u32,
        call: &mut C,
    ) -> u32 {
        let n = rcdt_parse_token_len_text(mem, p);
        if n.cast_signed() > 0 {
            let strncpy = mem.read::<u32>(mem.rebase(super::CRT_STRNCPY));
            let cur = mem.read::<u32>(p + 4);
            call(mem, strncpy, &[out, cur, n]);
            mem.write(out.wrapping_add(n), 0u8);
            rcdt_parse_next_token(mem, p);
        }
        n
    }

    /// `rcdtTm2Delete(tm2)` (`0x00923450`): [`rcdt_tm2_fini`], then the
    /// holder freed (`rcFree`, a tail call).
    pub fn rcdt_tm2_delete<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        t: u32,
        call: &mut C,
    ) -> u32 {
        rcdt_tm2_fini(mem, t, call);
        crate::model::bridge::rc_free(mem, call, t)
    }
}

#[cfg(test)]
mod tests {
    use ffx_bridge::{Mem, VecMem};

    use super::*;

    #[test]
    fn a_parser_reads_its_chunk() {
        let mut m = VecMem::new(0x1000, 0x400);
        let (c, p) = (0x1000, 0x1100);
        bridge::rcdt_chunk_set_bin(&mut m, c, 0x1300);
        m.write_bytes(0x1300, b"abcdef");
        bridge::rcdt_parse_init(&mut m, p);
        assert_eq!(m.read::<u8>(p + parse::SEPARATORS + 0x20), 1);
        assert_eq!(m.read::<u8>(p + parse::SEPARATORS + 0x21), 0);
        bridge::rcdt_parse_set(&mut m, p, c);
        bridge::rcdt_parse_set_pos(&mut m, p, 2);
        bridge::rcdt_parse_get_bin(&mut m, p, 0x1200, 3);
        assert_eq!(m.read::<[u8; 3]>(0x1200), *b"cde");
        assert_eq!(m.read::<u32>(p + parse::CURSOR), 0x1305);
        assert_eq!(m.read::<u32>(c + chunk::SIZE), chunk::UNBOUNDED);
    }
}
