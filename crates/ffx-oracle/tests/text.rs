//! The text decoders (`ffx_text::bridge`) against the original, on a
//! snapshot of the running game: the western table is the one the game
//! loaded; the jp/kr/ch tables come from the player's `FFX_Data.vbf`, put
//! where `g_CharsetCjk` points in both oracles. Every character code goes
//! through both decoders in every language; `Text_ToUtf8` runs on shipped
//! messages and on random bytes. Needs `FFX_GAME_DIR` and `FFX_SNAPSHOT`;
//! skipped otherwise.

use std::path::PathBuf;

use ffx_oracle::{Conv, Oracle, Pages};
use ffx_text::{Language, bridge, gamecode, messages::MessageTable};
use ffx_vbf::Archive;

const CONVERT_GAME_CODE_TO_UTF8: u32 = 0x0064_5f30;
const TEXT_TO_UTF8: u32 = 0x0064_5f50;
const DECODING_GAMECODE_JPCHKR: u32 = 0x0064_60a0;
const TEXT_DECODE_CHAR_WESTERN: u32 = 0x0064_61f0;
/// `TOGetFFXLang` through the thunk the game calls.
const LANG_GET: u32 = 0x008a_c2f0;

/// Languages as `TOGetFFXLang` numbers them, and some it does not use.
const LANGS: [i32; 9] = [0, 1, 2, 4, 9, 10, 11, -1, 12];

struct Twins {
    a: Oracle,
    b: Oracle,
    archive: Archive,
}

fn setup() -> Option<Twins> {
    let (Some(dir), Some(snap)) = (
        std::env::var_os("FFX_GAME_DIR"),
        std::env::var_os("FFX_SNAPSHOT"),
    ) else {
        eprintln!("FFX_GAME_DIR or FFX_SNAPSHOT is not set; skipping");
        return None;
    };
    let dir = PathBuf::from(dir);
    let exe = dir.join("FFX.exe");
    let pages = Pages::read(PathBuf::from(snap)).unwrap();
    Some(Twins {
        a: Oracle::from_snapshot(&exe, &pages).unwrap(),
        b: Oracle::from_snapshot(&exe, &pages).unwrap(),
        archive: Archive::open(dir.join("data/FFX_Data.vbf")).unwrap(),
    })
}

impl Twins {
    fn read(&self, path: &str) -> Vec<u8> {
        self.archive.read(self.archive.find(path).unwrap()).unwrap()
    }

    /// The same bytes at the same new address in both.
    fn alloc(&mut self, bytes: &[u8]) -> u32 {
        let x = self.a.alloc_bytes(bytes).unwrap();
        assert_eq!(self.b.alloc_bytes(bytes).unwrap(), x);
        x
    }

    fn write(&mut self, at: u32, bytes: &[u8]) {
        self.a.write(at, bytes).unwrap();
        self.b.write(at, bytes).unwrap();
    }

    /// Loads `language`'s table as `MallocAndReadGamecode` would (padded to
    /// 16 bytes plus a 0) and points `g_CharsetCjk` at it.
    fn load_cjk(&mut self, language: Language) -> Vec<u8> {
        let mut table = self.read(&language.charset_path());
        table.resize(table.len().div_ceil(16) * 16 + 1, 0);
        // codes below the first lead byte read up to 0x7e0 bytes before
        // the table: keep that in memory both oracles share
        self.alloc(&[0x5a; 0x1000]);
        let at = self.alloc(&table);
        let g = self.a.rebase(bridge::CHARSET_CJK.sym);
        self.write(g, &at.to_le_bytes());
        table
    }

    fn western(&mut self) -> Vec<u8> {
        let at = bridge::CHARSET_WESTERN.get(&self.b);
        assert_ne!(at, 0, "the snapshot is of a western game");
        self.a.read(at, 0x200).unwrap()
    }

    fn diff(&mut self, what: &str) {
        assert_eq!(self.a.diff(&mut self.b).unwrap(), [], "{what}: memory");
    }
}

/// Every byte sequence a decoder can be handed: single bytes, lead and
/// trail pairs (leads below `2c` too, which reach before the table), and
/// the same behind `04`.
fn sequences(bank: bool) -> Vec<Vec<u8>> {
    let mut v = Vec::new();
    for b in 0x30..=0xffu8 {
        v.push(vec![b]);
    }
    for lead in 0x28..=0x2fu8 {
        for t in 0..=0xffu8 {
            v.push(vec![lead, t]);
        }
    }
    if bank {
        for s in v.clone() {
            v.push([&[4u8][..], &s].concat());
        }
    }
    v
}

#[test]
fn character_decoders_match_the_original() {
    let Some(mut t) = setup() else { return };
    let src = t.alloc(&[0; 8]);
    let out = t.alloc(&[0; 8]);
    let western = t.western();
    for (language, langs) in [
        (Language::Japanese, &LANGS[..]),
        (Language::Korean, &[9, 0, 1][..]),
        (Language::Chinese, &[10, 0, 11][..]),
    ] {
        let cjk = t.load_cjk(language);
        for &lang in langs {
            for (n, seq) in sequences(true).iter().enumerate() {
                let mut s = seq.clone();
                s.resize(8, 0);
                t.write(src, &s);
                t.write(out, &[0xcc; 8]);
                #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
                let pos = (n % 7) as i32 - 2;
                #[allow(clippy::cast_sign_loss)]
                let args = [src, pos as u32, out, lang as u32];
                let ra =
                    t.a.call(t.a.rebase(DECODING_GAMECODE_JPCHKR), Conv::Cdecl, &args)
                        .unwrap()
                        .eax;
                let rb = bridge::decoding_gamecode_jpchkr(&mut t.b, src, pos, out, lang);
                #[allow(clippy::cast_sign_loss)]
                let rb = rb as u32;
                let what = format!("{language:?} lang {lang} {seq:02x?}");
                assert_eq!(ra, rb, "{what}: result");
                let oa = t.a.read(out, 8).unwrap();
                assert_eq!(oa, t.b.read(out, 8).unwrap(), "{what}: output");
                // the core on host bytes agrees
                let g = gamecode::cjk(|i| s[i as usize], lang);
                if let Some(bytes) = g.bytes(&cjk) {
                    assert_eq!(bytes, oa[..bytes.len()], "{what}: core");
                }
            }
            t.diff(&format!("{language:?} lang {lang}"));
        }
    }
    for (n, seq) in sequences(false).iter().enumerate() {
        let mut s = seq.clone();
        s.resize(8, 0);
        t.write(src, &s);
        t.write(out, &[0xcc; 8]);
        #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
        let pos = (n % 7) as i32 - 2;
        #[allow(clippy::cast_sign_loss)]
        let args = [src, pos as u32, out];
        let ra =
            t.a.call(t.a.rebase(TEXT_DECODE_CHAR_WESTERN), Conv::Cdecl, &args)
                .unwrap()
                .eax;
        let rb = bridge::text_decode_char_western(&mut t.b, src, pos, out);
        #[allow(clippy::cast_sign_loss)]
        let rb = rb as u32;
        let what = format!("western {seq:02x?}");
        assert_eq!(ra, rb, "{what}: result");
        let oa = t.a.read(out, 8).unwrap();
        assert_eq!(oa, t.b.read(out, 8).unwrap(), "{what}: output");
        if let Some(bytes) = gamecode::western(|i| s[i as usize]).bytes(&western) {
            assert_eq!(bytes, oa[..bytes.len()], "{what}: core");
        }
    }
    t.diff("western");
}

struct Xorshift(u32);

impl Xorshift {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }
}

/// Up to `per_table` messages from every `step`th event table of a language.
fn messages(t: &Twins, language: Language, step: usize, per_table: usize) -> Vec<Vec<u8>> {
    let prefix = format!(
        "ffx_ps2/ffx/master/new_{}pc/event/obj_ps3/",
        language.code()
    );
    let mut out = Vec::new();
    for entry in t
        .archive
        .entries()
        .iter()
        .filter(|e| e.path.starts_with(&prefix) && e.path.ends_with(".bin"))
        .step_by(step)
    {
        let table = MessageTable::parse(&t.archive.read(entry).unwrap()).unwrap();
        out.extend(
            table
                .messages
                .into_iter()
                .filter_map(|m| m.text)
                .take(per_table),
        );
    }
    out
}

#[test]
fn text_to_utf8_matches_the_original() {
    let Some(mut t) = setup() else { return };
    let western = t.western();
    let mut rng = Xorshift(0x2545_f491);
    for (language, cjk_language, langs) in [
        (Language::English, Language::Japanese, &[1, 2, 4][..]),
        (Language::German, Language::Japanese, &[4][..]),
        (Language::Japanese, Language::Japanese, &[0][..]),
        (Language::Korean, Language::Korean, &[9][..]),
        (Language::Chinese, Language::Chinese, &[10, 11][..]),
    ] {
        let cjk = t.load_cjk(cjk_language);
        let strings = messages(&t, language, 5, 40);
        eprintln!("{language:?}: {} messages", strings.len());
        assert!(strings.len() > 1000, "{language:?}");
        for &lang in langs {
            for (i, s) in strings.iter().enumerate() {
                convert(
                    &mut t,
                    s,
                    lang,
                    (&cjk, &western),
                    &format!("{language:?} {lang} #{i}"),
                );
            }
            t.diff(&format!("{language:?} lang {lang}"));
        }
        // random bytes, every grammar construct in any order
        for i in 0..400 {
            let len = rng.next() % 40;
            let s: Vec<u8> = (0..len)
                .map(|_| {
                    #[allow(clippy::cast_possible_truncation)]
                    let b = rng.next() as u8;
                    b.max(1)
                })
                .collect();
            convert(
                &mut t,
                &s,
                langs[0],
                (&cjk, &western),
                &format!("random #{i}"),
            );
        }
        t.diff(&format!("{language:?} random"));
    }
    // ConvertGameCodeToUTF8 in the snapshot's own language
    let lang_get = t.b.rebase(LANG_GET);
    #[allow(clippy::cast_possible_wrap)]
    let lang = t.b.call(lang_get, Conv::Cdecl, &[]).unwrap().eax as i32;
    let _ = t.a.call(t.a.rebase(LANG_GET), Conv::Cdecl, &[]).unwrap();
    assert_eq!(lang, 1, "an English snapshot");
    let text = messages(&t, Language::English, 300, 30);
    for (i, s) in text.iter().enumerate() {
        let src = t.alloc(&[&s[..], &[0; 4]].concat());
        let out = t.alloc(&vec![0xcc; 3 * s.len() + 16]);
        t.a.call(
            t.a.rebase(CONVERT_GAME_CODE_TO_UTF8),
            Conv::Cdecl,
            &[src, out],
        )
        .unwrap();
        bridge::convert_game_code_to_utf8(&mut t.b, src, out, lang);
        let n = 3 * s.len() + 16;
        assert_eq!(
            t.a.read(out, n).unwrap(),
            t.b.read(out, n).unwrap(),
            "ConvertGameCodeToUTF8 #{i}"
        );
    }
    t.diff("ConvertGameCodeToUTF8");
}

/// Runs `Text_ToUtf8` on `text` in both oracles and the core; compares.
fn convert(t: &mut Twins, text: &[u8], lang: i32, tables: (&[u8], &[u8]), what: &str) {
    // padding: an argument byte is skipped even when it is the terminator
    let src = t.alloc(&[text, &[0; 4]].concat());
    let n = 3 * text.len() + 16;
    let out = t.alloc(&vec![0xcc; n]);
    #[allow(clippy::cast_sign_loss)]
    let args = [src, out, lang as u32];
    t.a.call(t.a.rebase(TEXT_TO_UTF8), Conv::Cdecl, &args)
        .unwrap();
    bridge::text_to_utf8(&mut t.b, src, out, lang);
    let oa = t.a.read(out, n).unwrap();
    assert_eq!(oa, t.b.read(out, n).unwrap(), "{what}: output");
    if let Some(core) = gamecode::to_utf8(text, lang, tables.0, tables.1) {
        assert_eq!(core, oa[..core.len()], "{what}: core");
        assert_eq!(oa[core.len()], 0, "{what}: terminator");
    }
}
