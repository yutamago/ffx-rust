//! The game's text decoders where the game keeps its charset tables,
//! through [`ffx_bridge::Mem`] (`docs/porting.md`). The logic is in
//! [`crate::gamecode`]; these read the source bytes and the loaded table
//! from game memory and write what the originals write.

use ffx_bridge::{Global, Mem};

use crate::gamecode::{self, Glyph, Step};

/// The jp/kr/ch table `MallocAndReadGamecode` loaded (null in a western
/// game).
pub const CHARSET_CJK: Global<u32> = Global::new(0x00cc_b95c);
/// The western table (`ffxsjistbl_us.bin`; null in a jp/kr/ch game).
pub const CHARSET_WESTERN: Global<u32> = Global::new(0x00cc_b960);

/// A source byte reader at `at`.
fn reader(mem: &impl Mem, at: u32) -> impl Fn(u32) -> u8 + '_ {
    move |i| mem.read::<u8>(at.wrapping_add(i))
}

/// Copies a glyph from `table` to `out`; the new position.
fn put(mem: &mut impl Mem, glyph: Glyph, table: u32, pos: i32, out: u32) -> i32 {
    match glyph {
        Glyph::Table { offset, len } => {
            let mut b = [0u8; 3];
            let b = &mut b[..len as usize];
            mem.read_bytes(table.wrapping_add_signed(offset), b);
            mem.write_bytes(out, b);
        }
        Glyph::Byte(b) => mem.write(out, b),
        Glyph::Nothing => {}
    }
    #[allow(clippy::cast_possible_wrap)] // at most 3
    pos.wrapping_add(glyph.len() as i32)
}

/// `DecodingGamecodeJPCHKR(src, pos, out, lang)` (`0x006460a0`): the
/// character at `src` written to `out`; returns `pos` plus the bytes
/// written.
pub fn decoding_gamecode_jpchkr(
    mem: &mut impl Mem,
    src: u32,
    pos: i32,
    out: u32,
    lang: i32,
) -> i32 {
    let glyph = gamecode::cjk(reader(mem, src), lang);
    let table = CHARSET_CJK.get(mem);
    put(mem, glyph, table, pos, out)
}

/// `Text_DecodeCharWestern(src, pos, out)` (`0x006461f0`): as
/// [`decoding_gamecode_jpchkr`] for the western table.
pub fn text_decode_char_western(mem: &mut impl Mem, src: u32, pos: i32, out: u32) -> i32 {
    let glyph = gamecode::western(reader(mem, src));
    let table = CHARSET_WESTERN.get(mem);
    put(mem, glyph, table, pos, out)
}

/// `Text_ToUtf8(src, out, lang)` (`0x00645f50`): the string at `src` as
/// UTF-8 at `out`, `00`-terminated; line breaks become `\r\n`, control codes
/// vanish.
pub fn text_to_utf8(mem: &mut impl Mem, src: u32, out: u32, lang: i32) {
    let mut p = src;
    let mut pos = 0i32;
    loop {
        let at = out.wrapping_add_signed(pos);
        if mem.read::<u8>(p) == 0 {
            mem.write(at, 0u8);
            return;
        }
        let advance = match gamecode::step(reader(mem, p), lang) {
            Step::Skip(n) => n,
            Step::Newline => {
                mem.write(at, 0x0a0du16);
                pos = pos.wrapping_add(2);
                1
            }
            Step::Cjk(n) => {
                pos = decoding_gamecode_jpchkr(mem, p, pos, at, lang);
                n
            }
            Step::Western(n) => {
                pos = text_decode_char_western(mem, p, pos, at);
                n
            }
        };
        p = p.wrapping_add(advance);
    }
}

/// `ConvertGameCodeToUTF8(src, out)` (`0x00645f30`): [`text_to_utf8`] in
/// the game's language (`lang`, which the caller asks `TOGetFFXLang` for).
pub fn convert_game_code_to_utf8(mem: &mut impl Mem, src: u32, out: u32, lang: i32) {
    text_to_utf8(mem, src, out, lang);
}
