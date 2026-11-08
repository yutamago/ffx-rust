//! M5 round 11 (`tail11`): the settings file's text (`GameSetting.ini`,
//! `docs/formats/platform.md`): `Settings_Parse` (`0x00401e00`) and
//! `Settings_Format` (`0x004022f0`).
//!
//! The originals run the text through an inlined `std::istringstream` /
//! `std::ostringstream` (MSVCP110). The ports do the same with Rust strings:
//! the words `istream >> std::string` gives (split at the classic spaces, a
//! last empty word when the text ends with one or is empty), `ostream << int` as signed decimal,
//! `<< const char*` as the bytes up to the NUL. What the callers see is the
//! same: the settings words, the binding tables and the globals for Parse,
//! the text and its length for Format, and the same calls to
//! `Settings_SetValue` and `printf`.

/// The originals as bridges over [`ffx_graphic::host::Host`].
#[allow(clippy::cast_possible_truncation)] // string lengths, far below 4 GiB
pub mod bridge {
    use crate::m5a_settings::PRESETS;
    pub use crate::m5a_settings::bridge::{FORMAT, PARSE};
    use crate::m5a_settings::bridge::{KEY_NAMES, OPERATOR_DELETE, PRINTF_IAT, VALUE_STRINGS};
    use ffx_graphic::host::{Conv, Host};

    /// `Settings_SetValue(this; key, std::string value)` (thiscall, `ret
    /// 0x1c`; the string by value, six words, destroyed by the callee).
    pub const SET_VALUE: u32 = 0x0040_1930;
    /// `operator new(size)` (cdecl).
    pub const OPERATOR_NEW: u32 = 0x0063_0310;
    /// `"="`, `"\n"`, `"*"`.
    pub const EQUALS: u32 = 0x00b7_ca94;
    /// See [`EQUALS`].
    pub const NEWLINE: u32 = 0x00b7_cae0;
    /// See [`EQUALS`].
    pub const STAR: u32 = 0x00b7_c798;
    /// `"[ConfigFile] invalid line at [%s].\n"`.
    pub const MSG_INVALID_LINE: u32 = 0x00b7_ca98;
    /// `"[ConfigFile] invalid key at [%s].\n"`.
    pub const MSG_INVALID_KEY: u32 = 0x00b7_cabc;
    /// The globals Parse sets to 0xc / 0xd for languages other than 0, 6
    /// and 7 (entries 18 and 19 of the three default binding tables).
    pub const DEFAULT_BINDINGS: [u32; 3] = [0x00c8_a538, 0x00c8_a3f0, 0x00c8_a2a8];
    /// Bytes of [`parse`]'s frame: a short C string's buffer (16 bytes,
    /// where a `std::string` keeps up to 15 characters in place).
    pub const PARSE_FRAME: u32 = 0x10;

    const KEYS: u32 = 0x18;
    /// The binding lists (keys 0x15..0x17): their tables' offsets.
    const LISTS: [(u32, u32); 3] = [(0x15, 0x60), (0x16, 0x1a4), (0x17, 0x2e8)];
    /// Words in a binding table.
    const BINDINGS: u32 = 0x51;

    /// A NUL-terminated string's bytes.
    fn c_string(h: &impl Host, mut at: u32) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let c: u8 = h.read(at);
            if c == 0 {
                return out;
            }
            out.push(c);
            at = at.wrapping_add(1);
        }
    }

    fn write_bytes(h: &mut impl Host, at: u32, bytes: &[u8]) {
        for (k, &c) in bytes.iter().enumerate() {
            h.write(at.wrapping_add(k as u32), c);
        }
    }

    /// A `std::string`'s storage for `len` characters: in place below 16
    /// (`None`), else `operator new(len | 0xf + 1)` and that capacity.
    fn heap_block(h: &mut impl Host, len: u32) -> Option<(u32, u32)> {
        (len >= 0x10).then(|| {
            let cap = len | 0xf;
            (h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[cap + 1]), cap)
        })
    }

    /// `printf(format, text)` through the import, `text` a C string in
    /// `frame` (short) or a block of its own (long, freed after).
    fn print(h: &mut impl Host, frame: u32, format: u32, text: &[u8]) {
        let len = text.len() as u32;
        let block = heap_block(h, len);
        let at = block.map_or(frame, |(p, _)| p);
        write_bytes(h, at, text);
        h.write(at.wrapping_add(len), 0u8);
        let printf: u32 = h.read(h.rebase(PRINTF_IAT));
        let format = h.rebase(format);
        h.call(printf, Conv::Cdecl, &[format, at]);
        if let Some((p, _)) = block {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
        }
    }

    /// The key index of `key` (`_stricmp` against [`KEY_NAMES`]), if any.
    fn key_index(h: &impl Host, key: &[u8]) -> Option<u32> {
        let names = h.rebase(KEY_NAMES);
        (0..KEYS).find(|&i| {
            let name = c_string(h, h.read(names.wrapping_add(4 * i)));
            name.eq_ignore_ascii_case(key)
        })
    }

    /// `Settings_SetValue(this; key, value)` with `value` as a by-value
    /// `std::string`: the characters in place below 16 (capacity 15), else
    /// in a block of `operator new` (capacity `len | 0xf`), which the callee
    /// frees.
    fn set_value(h: &mut impl Host, this: u32, key: u32, value: &[u8]) {
        let len = value.len() as u32;
        let mut words = [0u32; 6];
        match heap_block(h, len) {
            Some((p, cap)) => {
                write_bytes(h, p, value);
                h.write(p.wrapping_add(len), 0u8);
                words[0] = p;
                words[5] = cap;
            }
            None => {
                let mut buf = [0u8; 16];
                buf[..value.len()].copy_from_slice(value);
                for (k, w) in words.iter_mut().take(4).enumerate() {
                    *w = u32::from_le_bytes([
                        buf[4 * k],
                        buf[4 * k + 1],
                        buf[4 * k + 2],
                        buf[4 * k + 3],
                    ]);
                }
                words[5] = 0xf;
            }
        }
        words[4] = len;
        let args = [
            key, words[0], words[1], words[2], words[3], words[4], words[5],
        ];
        h.call_sym(SET_VALUE, Conv::Thiscall(this), &args);
    }

    /// The classic `ctype<char>` spaces: `\t`, `\n`, `\v`, `\f`, `\r` and
    /// `' '`.
    const fn is_space(c: u8) -> bool {
        matches!(c, 9..=13 | b' ')
    }

    /// What `while (!ss.eof()) { ss >> word; ... }` reads: the words between
    /// spaces, then an empty one when the text ends in a space or is only
    /// spaces (the last extraction finds the end before a character).
    fn words(text: &[u8]) -> Vec<&[u8]> {
        let mut out: Vec<&[u8]> = text
            .split(|&c| is_space(c))
            .filter(|w| !w.is_empty())
            .collect();
        if text.last().is_none_or(|&c| is_space(c)) {
            out.push(&[]);
        }
        out
    }

    /// `Settings_Parse(this; text)` (`0x00401e00`, thiscall, `ret 4`) -> 0:
    /// each word of the C string `text` (`operator>>` into a
    /// `std::string`, [`words`]): no `=` prints "invalid line"; the part before the first `=` names a key
    /// (`_stricmp` with [`KEY_NAMES`]; none prints "invalid key"), the rest
    /// goes to `Settings_SetValue` and marks the key seen. Then the preset
    /// (`this[3]` as it is now) fills the keys not seen, and keys 11..18
    /// too unless it is 3 (custom), from the table at `0x00b7cba8` (16 bytes
    /// a key); a language (`this[0]`) other than 0, 6 and 7 sets entries 18
    /// and 19 of the three default binding tables to 0xc and 0xd; unless
    /// all three binding lists were seen, the tables (`0x51` words each)
    /// are copied to `+0x60`, `+0x1a4`, `+0x2e8` and the count `+0x42c` is
    /// 0x51. `frame` see [`PARSE_FRAME`].
    pub fn parse(h: &mut impl Host, frame: u32, this: u32, text: u32) -> u32 {
        let text = c_string(h, text);
        let mut seen = [false; KEYS as usize];
        for line in words(&text) {
            let Some(eq) = line.iter().position(|&c| c == b'=') else {
                print(h, frame, MSG_INVALID_LINE, line);
                continue;
            };
            let (key, value) = (&line[..eq], &line[eq + 1..]);
            match key_index(h, key) {
                None => print(h, frame, MSG_INVALID_KEY, key),
                Some(i) => {
                    seen[i as usize] = true;
                    set_value(h, this, i, value);
                }
            }
        }
        let preset: u32 = h.read(this.wrapping_add(0xc));
        let row = h.rebase(PRESETS).wrapping_add(preset.wrapping_mul(4));
        for (k, &was_seen) in (0u32..).zip(seen.iter()) {
            let fixed = (11..=18).contains(&k) && preset != 3;
            if !was_seen || fixed {
                let v: u32 = h.read(row.wrapping_add(16 * k));
                h.write(this.wrapping_add(4 * k), v);
            }
        }
        let language: u32 = h.read(this);
        if language != 0 && language != 6 && language != 7 {
            for table in DEFAULT_BINDINGS {
                let at = h.rebase(table);
                h.write(at + 0x48, 0xcu32);
                h.write(at + 0x4c, 0xdu32);
            }
        }
        if !(seen[0x15] && seen[0x16] && seen[0x17]) {
            for j in 0..BINDINGS {
                for (k, (_, off)) in LISTS.into_iter().enumerate() {
                    let v: u32 = h.read(h.rebase(DEFAULT_BINDINGS[k]) + 4 * j);
                    h.write(this.wrapping_add(off + 4 * j), v);
                }
            }
            h.write(this.wrapping_add(0x42c), BINDINGS);
        }
        0
    }

    /// The ini text of the settings object: per key `Name=value\n`.
    fn text(h: &impl Host, this: u32) -> Vec<u8> {
        let names = h.rebase(KEY_NAMES);
        let values = h.rebase(VALUE_STRINGS);
        let equals = c_string(h, h.rebase(EQUALS));
        let newline = c_string(h, h.rebase(NEWLINE));
        let mut out = Vec::new();
        for i in 0..KEYS {
            let row = values.wrapping_add(32 * i);
            let count = (0..8u32)
                .find(|&j| h.read::<u32>(row.wrapping_add(4 * j)) == 0)
                .unwrap_or(8);
            out.extend(c_string(h, h.read(names.wrapping_add(4 * i))));
            out.extend(&equals);
            let v: u32 = h.read(this.wrapping_add(4 * i));
            if count == 1 {
                if i == 1 {
                    // Resolution: width * height
                    let w = v.cast_signed() >> 16;
                    out.extend(w.to_string().bytes());
                    out.extend(c_string(h, h.rebase(STAR)));
                    out.extend((v & 0xffff).to_string().bytes());
                } else if let Some(&(_, off)) = LISTS.iter().find(|l| l.0 == i) {
                    // Settings_JoinBindings: the words, comma-separated
                    let n: u32 = h.read(this.wrapping_add(0x42c));
                    let list = this.wrapping_add(off);
                    for k in 0..n.cast_signed().max(0).cast_unsigned() {
                        if k > 0 {
                            out.push(b',');
                        }
                        let word: u32 = h.read(list.wrapping_add(4u32.wrapping_mul(k)));
                        out.extend(word.cast_signed().to_string().bytes());
                    }
                } else {
                    out.extend(v.cast_signed().to_string().bytes());
                }
            } else {
                let at = values.wrapping_add(v.wrapping_add(8 * i).wrapping_mul(4));
                out.extend(c_string(h, h.read(at)));
            }
            out.extend(&newline);
        }
        out
    }

    /// `Settings_Format(this; buf)` (`0x004022f0`, thiscall, `ret 4`): the
    /// ini text (per key `Name=value\n`: the value string of the key's
    /// index for keys with several (`0x00c89f48`, 8 a key), `W*H` for
    /// `Resolution`, the binding lists comma-separated (`+0x42c` words of
    /// `+0x60`, `+0x1a4`, `+0x2e8`), the number for the others) copied with
    /// its NUL into `buf` unless 0; returns its length.
    pub fn format(h: &mut impl Host, this: u32, buf: u32) -> u32 {
        let text = text(h, this);
        if buf != 0 {
            write_bytes(h, buf, &text);
            h.write(buf.wrapping_add(text.len() as u32), 0u8);
        }
        text.len() as u32
    }
}
