//! `ffx-text`: the game's text decoders.

use super::{call0, mem, ported};

ported! {
    /// `ConvertGameCodeToUTF8(src, out)`: `Text_ToUtf8` in the game's language.
    ConvertGameCodeToUTF8 @ 0x0064_5f30 => extern "C" fn convert_game_code_to_utf8(src: u32, out: u32) {
        // TOGetFFXLang, through the thunk the original calls
        #[allow(clippy::cast_possible_wrap)]
        let lang = call0(0x008a_c2f0) as i32;
        ffx_text::bridge::convert_game_code_to_utf8(&mut mem(), src, out, lang);
    }
    /// `Text_ToUtf8(src, out, lang)`: game text to UTF-8.
    Text_ToUtf8 @ 0x0064_5f50 => extern "C" fn text_to_utf8(src: u32, out: u32, lang: i32) {
        ffx_text::bridge::text_to_utf8(&mut mem(), src, out, lang);
    }
    /// `DecodingGamecodeJPCHKR(src, pos, out, lang)`: one jp/kr/ch character.
    DecodingGamecodeJPCHKR @ 0x0064_60a0 => extern "C" fn decoding_gamecode_jpchkr(src: u32, pos: i32, out: u32, lang: i32) -> i32 {
        ffx_text::bridge::decoding_gamecode_jpchkr(&mut mem(), src, pos, out, lang)
    }
    /// `Text_DecodeCharWestern(src, pos, out)`: one western character.
    Text_DecodeCharWestern @ 0x0064_61f0 => extern "C" fn text_decode_char_western(src: u32, pos: i32, out: u32) -> i32 {
        ffx_text::bridge::text_decode_char_western(&mut mem(), src, pos, out)
    }
}
