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
