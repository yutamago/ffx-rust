//! The render-side glue (M4, list `h3-glue-render`, agent h3c):
//! `PhyreScene_isVFXTextureLoaded_00661c80`, the by-name sibling of
//! [`crate::h3c_vfxtex`]: finds the file of an effect texture given its
//! name (`"<VRAM address>_19_..."`, the first eight characters the key)
//! among the scene's loaded VFX textures, with its own special cases.
//!
//! The scene's list is [`crate::h3c_vfxtex`]'s (`+0x34`, `+0x3c` names,
//! `+0x54` paths); `+0xfc68` is the render-target texture's name.

pub use crate::h3c_vfxtex::{
    ET_CHR_HIDDEN, FMT_D, GET_HANDLE, GET_MAGIC_ID, GET_TO_BE_DELETE, GRAPHIC_ABMAP, SPRINTF,
    STR_RWTT, STRSTR, Step, UNIFY_FILENAME, VFX_PASS, VFX_PHASE,
};

/// The CRT's `strncpy` (cdecl), through its import slot.
pub const STRNCPY: u32 = 0x00b0_c368;
/// `TOGetFFXLangJmp()` (cdecl): the text language (0xb: the one whose
/// `znkd0600` sign differs).
pub const GET_LANG: u32 = 0x008a_c2f0;
/// `""` (the data root `fiosUnifyFilename` expands), and the empty name.
pub const STR_ROOT: u32 = 0x00b4_3944;
pub const STR_EMPTY: u32 = 0x00b8_fbeb;
/// `"trxtex"` and `"13312_19_0_0_512_256"` (`strstr` needles).
pub const STR_TRXTEX: u32 = 0x00b4_2a18;
pub const STR_FD_NAME: u32 = 0x00b4_3978;

/// The frame (`ebp` offsets): the root path (256 bytes), `sprintf`'s
/// buffer (16), the key (the name's first eight characters and a 0).
pub const ROOT: u32 = 0x124;
pub const NUMBER: u32 = 0x24;
pub const KEY: u32 = 0x14;

/// The keys the rules compare with: the string (`"NNNNN_19"`) and its
/// address as a number.
pub const KEYS: [(u32, u16); 25] = [
    (0x00b4_3710, 0x3980),
    (0x00b4_371c, 0x3ac0),
    (0x00b4_3788, 0x3800),
    (0x00b4_3960, 0x38c0),
    (0x00b4_396c, 0x3100),
    (0x00b4_3990, 0x3680),
    (0x00b4_399c, 0x3500),
    (0x00b4_39a8, 0x3700),
    (0x00b4_39b4, 0x3400),
    (0x00b4_39c0, 0x3740),
    (0x00b4_39cc, 0x3b00),
    (0x00b4_39d8, 0x3480),
    (0x00b4_39e4, 0x3840),
    (0x00b4_39f0, 0x3880),
    (0x00b4_39fc, 0x39a0),
    (0x00b4_3a08, 0x38a0),
    (0x00b4_3a14, 0x3580),
    (0x00b4_3a20, 0x34c0),
    (0x00b4_3a2c, 0x3540),
    (0x00b4_3a38, 0x3640),
    (0x00b4_3a44, 0x3a00),
    (0x00b4_3a50, 0x3600),
    (0x00b4_3a5c, 0x36e0),
    (0x00b4_3a68, 0x36c0),
    (0x00b4_3a74, 0x3a80),
];

/// What a special case tests.
#[derive(Debug, Clone, Copy)]
pub enum Gate {
    /// The map.
    Map(u32),
    /// `g_VfxPass` set and the map.
    PassMap(u32),
    /// `TOGetFFXLangJmp()` (called first) 0xb and the map.
    LangMap(u32),
    /// `MagicFile_GetMagicId()` (called first).
    Magic(u32),
    /// `MagicFile_GetHandle()` not -1, then `MagicFile_GetMagicId()`.
    HandleMagic(u32),
    /// The sphere grid is up.
    Abmap,
}

/// How a special case compares the name with its string.
#[derive(Debug, Clone, Copy)]
pub enum Test {
    /// The whole name equals it.
    Name,
    /// The key (the first eight characters) equals it.
    Key,
    /// The name holds it (`strstr`).
    Holds,
}

/// A special case: gate, test, string, the path for `fiosUnifyFilename`.
#[derive(Debug, Clone, Copy)]
pub struct Special {
    pub gate: Gate,
    pub test: Test,
    pub string: u32,
    pub path: u32,
}

const fn sp(gate: Gate, test: Test, string: u32, path: u32) -> Special {
    Special {
        gate,
        test,
        string,
        path,
    }
}

/// The special cases in the original's order.
pub const SPECIALS: [Special; 25] = [
    sp(Gate::Map(0x1b68), Test::Name, 0x00b4_36d0, 0x00b4_2b48),
    sp(Gate::Map(0x1b68), Test::Name, 0x00b4_36f0, 0x00b4_2bf8),
    sp(Gate::Magic(0x1e7), Test::Key, 0x00b4_3710, 0x00b4_2c50),
    sp(Gate::Magic(0x1e7), Test::Key, 0x00b4_371c, 0x00b4_2ca8),
    sp(Gate::Map(0x18bd), Test::Key, 0x00b4_371c, 0x00b4_2d58),
    sp(Gate::Magic(0x281), Test::Name, 0x00b4_3728, 0x00b4_2db0),
    sp(Gate::Magic(0x281), Test::Name, 0x00b4_3748, 0x00b4_2fc0),
    sp(Gate::Map(0x1346), Test::Name, 0x00b4_3768, 0x00b4_3018),
    sp(
        Gate::HandleMagic(0x231),
        Test::Holds,
        0x00b4_3788,
        0x00b4_3070,
    ),
    sp(
        Gate::HandleMagic(0x231),
        Test::Name,
        0x00b4_3794,
        0x00b4_30c8,
    ),
    sp(Gate::Magic(0x1f9), Test::Key, 0x00b4_3788, 0x00b4_3120),
    sp(Gate::PassMap(0x633), Test::Name, 0x00b4_3768, 0x00b4_3178),
    sp(Gate::Map(0x125c), Test::Name, 0x00b4_37b4, 0x00b4_31d0),
    sp(Gate::LangMap(0x199), Test::Name, 0x00b4_37d4, 0x00b4_37f0),
    sp(Gate::Magic(0xa5), Test::Name, 0x00b4_3844, 0x00b4_3228),
    sp(Gate::Magic(0xa5), Test::Name, 0x00b4_3864, 0x00b4_3280),
    sp(Gate::Abmap, Test::Name, 0x00b4_3884, 0x00b4_32d8),
    sp(Gate::Abmap, Test::Name, 0x00b4_38a4, 0x00b4_3320),
    sp(Gate::Abmap, Test::Name, 0x00b4_38c4, 0x00b4_3368),
    sp(
        Gate::HandleMagic(0x1f1),
        Test::Name,
        0x00b4_38e4,
        0x00b4_33b0,
    ),
    sp(
        Gate::HandleMagic(0x1f9),
        Test::Name,
        0x00b4_3904,
        0x00b4_3408,
    ),
    sp(
        Gate::HandleMagic(0x1b0),
        Test::Holds,
        0x00b4_371c,
        0x00b4_3460,
    ),
    sp(
        Gate::HandleMagic(0x1b6),
        Test::Name,
        0x00b4_3924,
        0x00b4_34b8,
    ),
    sp(
        Gate::HandleMagic(0x1b5),
        Test::Name,
        0x00b4_3924,
        0x00b4_3510,
    ),
    sp(
        Gate::HandleMagic(0xbc),
        Test::Holds,
        0x00b4_371c,
        0x00b4_3618,
    ),
];

/// The per-effect rules by key: [`crate::h3c_vfxtex::rule`]'s, except
/// for the effects this sibling treats differently (`0xa8`, `0xaa`/`0x25f`,
/// `0x46`) or not at all. `key` is the key's number when it is one of
/// [`KEYS`]; `same_name` whether the asked name equals the texture's.
#[must_use]
pub fn rule(
    magic: u32,
    phase: u32,
    skipped: i32,
    key: Option<u16>,
    same_name: bool,
) -> Option<Step> {
    let first = skipped == 0;
    let is = |p: u16| key == Some(p);
    let mark = |c: bool| Some(if c { Step::Mark } else { Step::Found });
    match magic {
        0xa8 => {
            let ph = phase.cast_signed();
            if phase == 0 || !first {
                Some(Step::Found)
            } else if ph >= 1 && is(0x3680) {
                Some(Step::Mark)
            } else {
                mark(ph >= 2 && (is(0x3500) || is(0x3700)))
            }
        }
        0xaa | 0x25f => {
            if is(0x3400) && !same_name {
                None
            } else {
                mark(first && is(0x3ac0))
            }
        }
        0x46 => mark(phase == 0 && first && is(0x3400)),
        0x24b | 0x23a | 0x15f | 0x246 | 0x207 | 0x280 | 0x241 => Some(Step::Found),
        _ => crate::h3c_vfxtex::rule(
            magic,
            phase,
            skipped,
            key.unwrap_or(0),
            0x13,
            (0, 0),
            (0, 0),
        ),
    }
}

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn get(h: &mut impl Host, f: u32) -> u32 {
        h.call_sym(f, Conv::Cdecl, &[])
    }

    fn global(h: &impl Host, at: u32) -> u32 {
        h.read(h.rebase(at))
    }

    /// A C string's bytes (without the 0), at most `max`.
    fn c_string(h: &impl Host, at: u32, max: u32) -> Vec<u8> {
        let mut v = Vec::new();
        for k in 0..max {
            let c: u8 = h.read(at.wrapping_add(k));
            if c == 0 {
                break;
            }
            v.push(c);
        }
        v
    }

    /// `strcmp(a, b) == 0` as the original's inline loop.
    fn same(h: &impl Host, mut a: u32, mut b: u32) -> bool {
        loop {
            let (x, y): (u8, u8) = (h.read(a), h.read(b));
            if x != y {
                return false;
            }
            if x == 0 {
                return true;
            }
            a = a.wrapping_add(1);
            b = b.wrapping_add(1);
        }
    }

    /// An import (`strstr`, `sprintf`, `strncpy`) through its slot.
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `PhyreScene_isVFXTextureLoaded_00661c80(scene; name, out, 4 unused
    /// words)`: the texture's file into `out`; 1 for a special case, 2
    /// for "RWTT" or a list hit, 0 when none. `ebp` is the frame ([`ROOT`],
    /// [`NUMBER`], [`KEY`]).
    pub fn is_vfx_texture_loaded(
        h: &mut impl Host,
        ebp: u32,
        scene: u32,
        name: u32,
        out: u32,
    ) -> u32 {
        let key_at = ebp - KEY;
        import(h, STRNCPY, &[key_at, name, 8]);
        h.write(ebp - 0xc, 0u8);
        for s in &SPECIALS {
            if special(h, scene, name, key_at, s) {
                let path = h.rebase(s.path);
                h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[path, out, 0x100]);
                return 1;
            }
        }
        if h.read::<u8>(scene + 0xfc00) != 0 {
            let rt = scene + 0xfc68;
            let n = c_string(h, rt, u32::MAX).len();
            let prefix = (0..n).all(|k| {
                let k = k as u32;
                h.read::<u8>(name.wrapping_add(k)) == h.read::<u8>(rt + k)
            });
            if prefix {
                let mut word = [0u8; 5];
                h.read_bytes(h.rebase(STR_RWTT), &mut word);
                h.write_bytes(out, &word);
                return 2;
            }
        }
        list(h, ebp, scene, name, out)
    }

    /// Whether special case `s` matches (making its gate's calls).
    fn special(h: &mut impl Host, scene: u32, name: u32, key_at: u32, s: &Special) -> bool {
        let map: u32 = h.read(scene + 0xfaf4);
        let gate = match s.gate {
            Gate::Map(m) => map == m,
            Gate::PassMap(m) => global(h, VFX_PASS) != 0 && map == m,
            Gate::LangMap(m) => get(h, GET_LANG) == 0xb && map == m,
            Gate::Magic(m) => get(h, GET_MAGIC_ID) == m,
            Gate::HandleMagic(m) => get(h, GET_HANDLE) != u32::MAX && get(h, GET_MAGIC_ID) == m,
            Gate::Abmap => global(h, GRAPHIC_ABMAP) != 0,
        };
        if !gate {
            return false;
        }
        let string = h.rebase(s.string);
        match s.test {
            Test::Name => same(h, name, string),
            Test::Key => same(h, key_at, string),
            Test::Holds => import(h, STRSTR, &[name, string]) != 0,
        }
    }

    /// `MagicFile_GetHandle()` not -1 and `MagicFile_GetMagicId()` `m`.
    fn handle_magic(h: &mut impl Host, m: u32) -> bool {
        get(h, GET_HANDLE) != u32::MAX && get(h, GET_MAGIC_ID) == m
    }

    /// Whether the path's part after the data root starts with `word`.
    fn under(h: &impl Host, path: u32, root_len: u32, word: &[u8]) -> bool {
        word.iter()
            .enumerate()
            .all(|(k, &c)| h.read::<u8>(path.wrapping_add(root_len + k as u32)) == c)
    }

    /// The loaded textures: the first whose key and path pass.
    #[allow(clippy::too_many_lines)] // the original's checks in order
    fn list(h: &mut impl Host, ebp: u32, scene: u32, name: u32, out: u32) -> u32 {
        let key_at = ebp - KEY;
        let root = ebp - ROOT;
        let root_str = h.rebase(STR_ROOT);
        h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[root_str, root, 0x100]);
        let root_len = c_string(h, root, u32::MAX).len() as u32;
        let key_bytes = c_string(h, key_at, 9);
        let key = KEYS
            .iter()
            .find(|&&(at, _)| c_string(h, h.rebase(at), 9) == key_bytes)
            .map(|&(_, p)| p);
        let mut skipped: i32 = 0;
        let mut i: u32 = 0;
        while i < h.read::<u32>(scene + 0x34) {
            let at = i;
            i += 1;
            let entry: u32 = h.read(h.read::<u32>(scene + 0x3c).wrapping_add(at * 4));
            if entry == 0 || same(h, entry, h.rebase(STR_EMPTY)) {
                continue;
            }
            let path_at = h.read::<u32>(scene + 0x54).wrapping_add(at * 4);
            let pass = global(h, VFX_PASS);
            if pass == 0 && under(h, h.read(path_at), root_len, b"mag") {
                continue;
            }
            if global(h, ET_CHR_HIDDEN) != 0 && under(h, h.read(path_at), root_len, b"yo") {
                continue;
            }
            if pass != 0
                && (under(h, h.read(path_at), root_len, b"map")
                    || under(h, h.read(path_at), root_len, b"btl"))
            {
                continue;
            }
            // the entry's first eight characters against the key
            let saved: u8 = h.read(entry + 8);
            h.write(entry + 8, 0u8);
            let hit = same(h, entry, key_at);
            h.write(entry + 8, saved);
            if !hit {
                continue;
            }
            let mut src: u32 = h.read(path_at);
            let mut dst = out;
            loop {
                let c: u8 = h.read(src);
                h.write(dst, c);
                if c == 0 {
                    break;
                }
                src = src.wrapping_add(1);
                dst = dst.wrapping_add(1);
            }
            let magic = get(h, GET_MAGIC_ID);
            let deleting = get(h, GET_TO_BE_DELETE);
            if magic != u32::MAX && deleting != u32::MAX {
                let buf = ebp - NUMBER;
                let fmt = h.rebase(FMT_D);
                import(h, SPRINTF, &[buf, fmt, magic]);
                if import(h, STRSTR, &[out, buf]) == 0 {
                    continue;
                }
            }
            if handle_magic(h, 0x2a1)
                && global(h, VFX_PHASE) != 0
                && skipped == 0
                && key == Some(0x38c0)
            {
                skipped = 1;
                continue;
            }
            if handle_magic(h, 0x296) && key == Some(0x3100) && import(h, STRSTR, &[out, name]) == 0
            {
                continue;
            }
            let trxtex = h.rebase(STR_TRXTEX);
            if import(h, STRSTR, &[key_at, trxtex]) != 0 && import(h, STRSTR, &[out, name]) == 0 {
                continue;
            }
            let needle = h.rebase(STR_FD_NAME);
            if handle_magic(h, 0xfd)
                && import(h, STRSTR, &[name, needle]) != 0
                && import(h, STRSTR, &[out, name]) == 0
            {
                continue;
            }
            if get(h, GET_HANDLE) == u32::MAX {
                return 2;
            }
            let magic = get(h, GET_MAGIC_ID);
            let same_name = key == Some(0x3400) && same(h, name, entry);
            match rule(magic, global(h, VFX_PHASE), skipped, key, same_name) {
                None => {}
                Some(Step::Mark) => skipped = 1,
                Some(Step::Inc) => skipped += 1,
                Some(Step::Found) => return 2,
            }
        }
        0
    }
}
