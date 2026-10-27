//! The render-side glue (M4, list `h3-glue-render`, agent h3c):
//! `PhyreScene_isVFXTextureLoaded` (`0x006604d0`), which finds the file
//! of a PS2 effect texture (its VRAM address `p2`, format `p3`, size
//! `p6` x `p7`) among the scene's loaded VFX textures, with the
//! remaster's special cases.
//!
//! The scene's texture list: `+0x34` the count, `+0x3c` the names,
//! `+0x48` records of 0x14 bytes (`+0` the kind: 0 character, 1 field,
//! 2 effect pass; `+4` the address, `+8` the format, `+0xe`/`+0x10` the
//! size), `+0x54` the file paths. `+0x64..+0x70` cache the last hit (the
//! six key words and the index). `+0xfaf4` is the map, `+0xfc00`/`+0xfc64`
//! the render-target texture ("RWTT").

/// `MagicFile_GetMagicId`, `MagicFile_IsNexData`, `MagicFile_GetHandle`,
/// `MagicFile_GetToBeDeleteMagicId` (cdecl, no arguments).
pub const GET_MAGIC_ID: u32 = 0x009d_a3b0;
pub const IS_NEX_DATA: u32 = 0x009d_a420;
pub const GET_HANDLE: u32 = 0x009d_a390;
pub const GET_TO_BE_DELETE: u32 = 0x009d_a3d0;
/// `LocalizationManager_GetWord0()` (cdecl): 0 for the Japanese text.
pub const GET_WORD0: u32 = 0x0064_1100;
/// `fiosUnifyFilename(src, out, size)` (cdecl).
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
/// The CRT's `sprintf` and `strstr` (cdecl), through their import slots.
pub const SPRINTF: u32 = 0x00b0_c3dc;
pub const STRSTR: u32 = 0x00b0_c378;
/// `"%d"`.
pub const FMT_D: u32 = 0x00b0_f604;
/// `"RWTT"` (five bytes with the terminator).
pub const STR_RWTT: u32 = 0x00b4_366c;
/// `magic_0662`'s Japanese texture (`..._512_512_JP.dds.phyre`).
pub const PATH_JP: u32 = 0x00b4_3678;
/// `g_VfxPass`, `g_VfxYiqMode`, `g_EtChrHidden`, `g_VfxPhase`.
pub const VFX_PASS: u32 = 0x0113_335c;
pub const VFX_YIQ_MODE: u32 = 0x00c3_a504;
pub const ET_CHR_HIDDEN: u32 = 0x00cc_b9d0;
pub const VFX_PHASE: u32 = 0x00cc_b9d4;
/// `g_GraphicAbmap`.
pub const GRAPHIC_ABMAP: u32 = 0x00cc_c838;

/// The frame (`ebp` offsets): `sprintf`'s buffer (16 bytes).
pub const NUMBER: u32 = 0x14;

/// What a special case tests besides the format (always 0x13) and the
/// address.
#[derive(Debug, Clone, Copy)]
pub enum Gate {
    /// The map.
    Map(u32),
    /// The map, with `g_VfxYiqMode` 8.
    MapYiq(u32),
    /// `g_VfxPass` set and the map.
    PassMap(u32),
    /// `MagicFile_GetMagicId()` (called first, whatever the rest).
    Magic(u32),
    /// `MagicFile_GetHandle()` not -1, then `MagicFile_GetMagicId()`.
    HandleMagic(u32),
    /// The sphere grid is up (`g_GraphicAbmap`).
    Abmap,
}

/// A special case: the gate, the address, the size when tested, the path
/// handed to `fiosUnifyFilename` (an address in `.rdata`).
#[derive(Debug, Clone, Copy)]
pub struct Special {
    pub gate: Gate,
    pub addr: u16,
    pub size: Option<(u16, u16)>,
    pub path: u32,
}

const fn sp(gate: Gate, addr: u16, size: Option<(u16, u16)>, path: u32) -> Special {
    Special {
        gate,
        addr,
        size,
        path,
    }
}

/// The special cases in the original's order (the first that matches
/// wins).
pub const SPECIALS: [Special; 35] = [
    sp(Gate::Map(0x1b8a), 0x3f00, None, 0x00b4_2a98),
    sp(Gate::Map(0x2416), 0x3ac0, None, 0x00b4_2af0),
    sp(Gate::Map(0x1b68), 0x37c0, Some((0x80, 0x20)), 0x00b4_2b48),
    sp(Gate::Map(0x1f7), 0x3ac0, Some((0x80, 0x80)), 0x00b4_2ba0),
    sp(Gate::Map(0x1b68), 0x37c0, Some((0x80, 0x40)), 0x00b4_2bf8),
    sp(Gate::Magic(0x1e7), 0x3980, None, 0x00b4_2c50),
    sp(Gate::Magic(0x1e7), 0x3ac0, None, 0x00b4_2ca8),
    sp(Gate::Magic(0x2c6), 0x3900, None, 0x00b4_2d00),
    sp(Gate::Map(0x18bd), 0x3ac0, None, 0x00b4_2d58),
    sp(
        Gate::Magic(0x281),
        0x3600,
        Some((0x200, 0x100)),
        0x00b4_2db0,
    ),
    sp(Gate::Map(0x18cd), 0x3340, Some((0x80, 0x40)), 0x00b4_2e08),
    sp(Gate::Map(0x18cd), 0x3360, Some((0x80, 0x40)), 0x00b4_2e60),
    sp(
        Gate::MapYiq(0x11d3),
        0x3700,
        Some((0x80, 0x80)),
        0x00b4_2eb8,
    ),
    sp(
        Gate::Magic(0x157),
        0x3400,
        Some((0x100, 0x200)),
        0x00b4_2f10,
    ),
    sp(Gate::Magic(0x174), 0x3ac0, None, 0x00b4_2f68),
    sp(
        Gate::Magic(0x281),
        0x3820,
        Some((0x200, 0x100)),
        0x00b4_2fc0,
    ),
    sp(Gate::Map(0x25b0), 0x3ac0, Some((0x80, 0x80)), 0x00b4_3018),
    sp(Gate::HandleMagic(0x231), 0x3800, None, 0x00b4_3070),
    sp(
        Gate::HandleMagic(0x231),
        0x3800,
        Some((0x80, 0x40)),
        0x00b4_30c8,
    ),
    sp(Gate::Magic(0x1f9), 0x3800, None, 0x00b4_3120),
    sp(
        Gate::PassMap(0x633),
        0x3ac0,
        Some((0x80, 0x80)),
        0x00b4_3178,
    ),
    sp(Gate::Map(0x125c), 0x3640, Some((0x80, 4)), 0x00b4_31d0),
    sp(Gate::Magic(0xa5), 0x3400, Some((0x80, 4)), 0x00b4_3228),
    sp(Gate::Magic(0xa5), 0x3420, Some((0x80, 4)), 0x00b4_3280),
    sp(Gate::Abmap, 0x3960, Some((0x80, 0x80)), 0x00b4_32d8),
    sp(Gate::Abmap, 0x39a0, Some((0x80, 4)), 0x00b4_3320),
    sp(Gate::Abmap, 0x39c0, Some((0x80, 4)), 0x00b4_3368),
    sp(
        Gate::HandleMagic(0x1f1),
        0x3ac0,
        Some((0x80, 0x100)),
        0x00b4_33b0,
    ),
    sp(
        Gate::HandleMagic(0x1f9),
        0x3980,
        Some((0x80, 0x40)),
        0x00b4_3408,
    ),
    sp(Gate::HandleMagic(0x1b0), 0x3ac0, None, 0x00b4_3460),
    sp(
        Gate::HandleMagic(0x1b6),
        0x3ac0,
        Some((0x80, 4)),
        0x00b4_34b8,
    ),
    sp(
        Gate::HandleMagic(0x1b5),
        0x3ac0,
        Some((0x80, 4)),
        0x00b4_3510,
    ),
    sp(
        Gate::HandleMagic(0x171),
        0x3480,
        Some((0x80, 4)),
        0x00b4_3568,
    ),
    sp(
        Gate::HandleMagic(0x171),
        0x3480,
        Some((0x200, 0x100)),
        0x00b4_35c0,
    ),
    sp(Gate::HandleMagic(0xbc), 0x3ac0, None, 0x00b4_3618),
];

/// What the per-effect rules make of a matching texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The texture is the one: copy its path.
    Found,
    /// Not this one; the count of skipped matches becomes 1.
    Mark,
    /// Not this one; the count goes up by one.
    Inc,
}

/// The per-effect rules (magic `magic`, `g_VfxPhase` `phase`, `skipped`
/// matches so far, address `p`, format `fmt`; `size` the asked size,
/// `rec_size` the texture's): which matching texture of a multi-phase
/// effect is meant. `None`: skip it without counting.
#[must_use]
#[allow(clippy::too_many_lines)] // one rule per effect, as the original
#[allow(clippy::similar_names)]
pub fn rule(
    magic: u32,
    phase: u32,
    skipped: i32,
    p: u16,
    fmt: u16,
    size: (u16, u16),
    rec_size: (u16, u16),
) -> Option<Step> {
    let s = skipped;
    let ph = phase.cast_signed();
    let bit = |b: u32| phase & b != 0;
    let first = s == 0;
    let mark = |c: bool| if c { Some(Step::Mark) } else { None };
    let inc = |c: bool| if c { Some(Step::Inc) } else { None };
    let found = Some(Step::Found);
    match magic {
        0xa8 | 0x24b => mark(ph > 0 && first && (p == 0x3600 || p == 0x3680))
            .or_else(|| mark(ph > 1 && first && (p == 0x3700 || p == 0x3500)))
            .or(found),
        0xaa | 0x25f => {
            if p == 0x3400 && fmt == 0x13 && size != rec_size {
                return None;
            }
            if first && p == 0x3ac0 {
                return Some(Step::Mark);
            }
            if magic != 0xaa || phase == 0 || !first {
                return found;
            }
            mark(p == 0x3880).or(found)
        }
        0x9a => {
            if !first {
                found
            } else if phase == 0 {
                mark(p == 0x3740 && fmt == 0x13).or(found)
            } else {
                mark(matches!(p, 0x3480 | 0x3840 | 0x3880 | 0x3b00)).or(found)
            }
        }
        0x130 | 0x238 => mark(ph >= 1 && first && p == 0x3400)
            .or_else(|| mark(ph >= 2 && first && p == 0x39a0))
            .or_else(|| mark(ph >= 3 && first && p == 0x38a0))
            .or(found),
        0x94 | 0x239 | 0xd0 => mark(bit(1) && first && p == 0x3580)
            .or_else(|| mark(bit(2) && first && p == 0x34c0))
            .or_else(|| mark(bit(4) && first && p == 0x3500))
            .or_else(|| mark(bit(8) && first && p == 0x3540))
            .or(found),
        0x1ae | 0x24c => mark(phase != 0 && first && p == 0x3740).or(found),
        0x1ff | 0x24d => {
            mark(phase == 1 && first && matches!(p, 0x3540 | 0x3640 | 0x3840)).or(found)
        }
        0x46 | 0x23a => mark(phase == 0 && first && p == 0x3400).or(found),
        0x133 => inc(ph >= 1 && s <= 0 && p == 0x3a00)
            .or_else(|| inc(ph >= 2 && s <= 0 && p == 0x3800))
            .or_else(|| inc(ph >= 4 && s <= 1 && p == 0x3800))
            .or_else(|| inc(ph >= 2 && s <= 0 && p == 0x3600))
            .or_else(|| inc(ph >= 3 && s <= 1 && p == 0x3600))
            .or(Some(tail_3400(ph, s, p))),
        0x233 => {
            if ph >= 1 && s <= 0 && p == 0x3a00 {
                return Some(Step::Inc);
            }
            if ph == 2 {
                if s > 0 {
                    return found;
                }
                if p == 0x3800 {
                    return Some(Step::Inc);
                }
            }
            if ph >= 2 && s <= 0 && p == 0x3600 {
                return Some(Step::Inc);
            }
            Some(tail_3400(ph, s, p))
        }
        0x203 | 0x29b => mark(bit(1) && first && p == 0x3680)
            .or_else(|| mark(bit(2) && first && p == 0x3980))
            .or_else(|| mark(bit(4) && first && p == 0x36e0))
            .or_else(|| inc(bit(8) && s < 2 && p == 0x36e0))
            .or_else(|| mark(bit(0x10) && first && p == 0x36c0))
            .or_else(|| inc(bit(0x20) && s < 2 && p == 0x3980))
            .or_else(|| mark(bit(0x20) && first && p == 0x3a80))
            .or_else(|| inc(bit(0x40) && s < 2 && p == 0x3a80))
            .or_else(|| mark(bit(0x40) && first && p == 0x3400))
            .or(found),
        0x263 | 0x29c => mark(bit(1) && first && (p == 0x3680 || p == 0x3980))
            .or_else(|| inc(bit(1) && s < 2 && p == 0x36e0))
            .or_else(|| mark(bit(1) && first && p == 0x36c0))
            .or_else(|| inc(bit(2) && s < 2 && p == 0x3980))
            .or_else(|| mark(bit(2) && first && p == 0x3a80))
            .or_else(|| inc(bit(4) && s < 2 && p == 0x3a80))
            .or_else(|| mark(bit(4) && first && p == 0x3400))
            .or(found),
        0xa2 | 0x24a | 0x18d | 0x15f | 0x246 => mark(first && p == 0x3ac0).or(found),
        0x207 => mark(
            bit(1) && first && matches!(p, 0x3400 | 0x3500 | 0x3520 | 0x35a0 | 0x3620 | 0x3640),
        )
        .or_else(|| mark(bit(2) && first && matches!(p, 0x36c0 | 0x3700 | 0x38a0 | 0x38c0)))
        .or_else(|| inc(bit(4) && s < 2 && p == 0x3400))
        .or_else(|| mark(bit(4) && first && p == 0x3800))
        .or(found),
        0x280 => mark(phase != 0 && first && matches!(p, 0x3440 | 0x3980 | 0x36e0)).or(found),
        0x241 => mark(bit(1) && first && matches!(p, 0x3700 | 0x3940))
            .or_else(|| mark(bit(2) && first && matches!(p, 0x3400 | 0x34c0)))
            .or(found),
        _ => found,
    }
}

/// The 0x3400 rules `0x133` and `0x233` end with.
fn tail_3400(ph: i32, s: i32, p: u16) -> Step {
    if p == 0x3400 && ((ph >= 1 && s <= 0) || (ph >= 4 && s <= 1)) {
        Step::Inc
    } else {
        Step::Found
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

    /// `strcpy(out, src)` as the original's byte loop.
    fn copy_string(h: &mut impl Host, mut out: u32, mut src: u32) {
        loop {
            let c: u8 = h.read(src);
            h.write(out, c);
            if c == 0 {
                return;
            }
            out = out.wrapping_add(1);
            src = src.wrapping_add(1);
        }
    }

    /// The key words: `p2` (the address; the special case's 0x3601),
    /// `p3` (the format), `p4`, `p5`, `p6`/`p7` (the size), as the
    /// original compares them (16 bits).
    struct Key {
        p: u16,
        fmt: u16,
        p4: u16,
        p5: u16,
        size: (u16, u16),
    }

    #[allow(clippy::cast_possible_truncation)] // the low halves
    fn half(w: u32) -> u16 {
        w as u16
    }

    /// `PhyreScene_isVFXTextureLoaded(scene; p2, p3, p4, p5, p6, p7,
    /// out)`: writes the texture's file into `out`. 1 for the cached hit
    /// or a special case, 2 for a list hit (cached then), "RWTT" or the
    /// Japanese `magic_0662` texture, 0 when none. `ebp` is the frame
    /// ([`NUMBER`]).
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn is_vfx_texture_loaded(
        h: &mut impl Host,
        ebp: u32,
        scene: u32,
        p2: u32,
        p3: u32,
        p4: u32,
        p5: u32,
        p6: u32,
        p7: u32,
        out: u32,
    ) -> u32 {
        let mut k = Key {
            p: half(p2),
            fmt: half(p3),
            p4: half(p4),
            p5: half(p5),
            size: (half(p6), half(p7)),
        };
        if h.read::<u32>(scene + 0xfaf4) == 0x1bfe
            && global(h, VFX_PASS) != 0
            && get(h, GET_MAGIC_ID) == 0x21b
            && get(h, IS_NEX_DATA) == 1
            && k.p == 0x3600
        {
            k.p = 0x3601;
        }
        let cached = h.read::<u16>(scene + 0x64) == k.p
            && h.read::<u16>(scene + 0x66) == k.fmt
            && h.read::<u16>(scene + 0x68) == k.p4
            && h.read::<u16>(scene + 0x6a) == k.p5
            && h.read::<u16>(scene + 0x6c) == k.size.0
            && h.read::<u16>(scene + 0x6e) == k.size.1;
        if cached {
            let index = u32::from(h.read::<u16>(scene + 0x70));
            if index < h.read::<u32>(scene + 0x34) {
                let src: u32 = h.read(h.read::<u32>(scene + 0x54).wrapping_add(index * 4));
                copy_string(h, out, src);
                return 1;
            }
        }
        for s in &SPECIALS {
            if special(h, scene, &k, s) {
                let path = h.rebase(s.path);
                h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[path, out, 0x100]);
                return 1;
            }
        }
        if h.read::<u8>(scene + 0xfc00) != 0 && h.read::<u32>(scene + 0xfc64) == u32::from(k.p) {
            let mut word = [0u8; 5];
            h.read_bytes(h.rebase(STR_RWTT), &mut word);
            h.write_bytes(out, &word);
            return 2;
        }
        list(h, ebp, scene, &k, out)
    }

    /// Whether special case `s` matches (making its gate's calls).
    fn special(h: &mut impl Host, scene: u32, k: &Key, s: &Special) -> bool {
        let map: u32 = h.read(scene + 0xfaf4);
        let gate = match s.gate {
            Gate::Map(m) => map == m,
            Gate::MapYiq(m) => map == m && global(h, VFX_YIQ_MODE) == 8,
            Gate::PassMap(m) => global(h, VFX_PASS) != 0 && map == m,
            Gate::Magic(m) => get(h, GET_MAGIC_ID) == m,
            Gate::HandleMagic(m) => get(h, GET_HANDLE) != u32::MAX && get(h, GET_MAGIC_ID) == m,
            Gate::Abmap => global(h, GRAPHIC_ABMAP) != 0,
        };
        gate && k.p == s.addr && k.fmt == 0x13 && s.size.is_none_or(|z| k.size == z)
    }

    /// `MagicFile_GetHandle()` not -1 and `MagicFile_GetMagicId()` `m`.
    fn handle_magic(h: &mut impl Host, m: u32) -> bool {
        get(h, GET_HANDLE) != u32::MAX && get(h, GET_MAGIC_ID) == m
    }

    /// The loaded textures: the first that matches the key and the
    /// effect's rules.
    #[allow(clippy::similar_names)]
    fn list(h: &mut impl Host, ebp: u32, scene: u32, k: &Key, out: u32) -> u32 {
        let mut skipped: i32 = 0;
        let mut i: u32 = 0;
        while i < h.read::<u32>(scene + 0x34) {
            let at = i;
            i += 1;
            let name: u32 = h.read(h.read::<u32>(scene + 0x3c).wrapping_add(at * 4));
            if name == 0 || h.read::<u8>(name) == 0 {
                continue;
            }
            let rec = h.read::<u32>(scene + 0x48).wrapping_add(at * 0x14);
            if global(h, VFX_PASS) == 0 && h.read::<u32>(rec) == 2 {
                continue;
            }
            if global(h, ET_CHR_HIDDEN) != 0 && h.read::<u32>(rec) == 0 {
                continue;
            }
            if global(h, VFX_PASS) != 0 && h.read::<u32>(rec) == 1 {
                continue;
            }
            if u32::from(k.p) != h.read::<u32>(rec + 4) || k.fmt != h.read::<u16>(rec + 8) {
                continue;
            }
            let magic = get(h, GET_MAGIC_ID);
            let deleting = get(h, GET_TO_BE_DELETE);
            if magic != u32::MAX && deleting != u32::MAX {
                let buf = ebp - NUMBER;
                let fmt = h.rebase(FMT_D);
                let sprintf: u32 = h.read(h.rebase(SPRINTF));
                h.call(sprintf, Conv::Cdecl, &[buf, fmt, magic]);
                let path: u32 = h.read(h.read::<u32>(scene + 0x54).wrapping_add(at * 4));
                let strstr: u32 = h.read(h.rebase(STRSTR));
                if h.call(strstr, Conv::Cdecl, &[path, buf]) == 0 {
                    continue;
                }
            }
            let phase = global(h, VFX_PHASE);
            if handle_magic(h, 0x2a1)
                && phase != 0
                && skipped == 0
                && k.p == 0x38c0
                && k.fmt == 0x13
            {
                skipped = 1;
                continue;
            }
            let rec_size = (h.read::<u16>(rec + 0xe), h.read::<u16>(rec + 0x10));
            if handle_magic(h, 0x296) && k.p == 0x3100 && k.fmt == 0x13 && k.size != rec_size {
                continue;
            }
            if get(h, GET_WORD0) == 0 && handle_magic(h, 0x296) && k.p == 0x2e00 {
                let path = h.rebase(PATH_JP);
                h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[path, out, 0x100]);
                return 2;
            }
            if handle_magic(h, 0xfd)
                && k.p == 0x3400
                && k.fmt == 0x13
                && k.size == (0x200, 0x100)
                && rec_size != (0x200, 0x100)
            {
                continue;
            }
            let step = if get(h, GET_HANDLE) == u32::MAX {
                Some(Step::Found)
            } else {
                let magic = get(h, GET_MAGIC_ID);
                let phase = global(h, VFX_PHASE);
                rule(magic, phase, skipped, k.p, k.fmt, k.size, rec_size)
            };
            match step {
                None => {}
                Some(Step::Mark) => skipped = 1,
                Some(Step::Inc) => skipped += 1,
                Some(Step::Found) => {
                    let src: u32 = h.read(h.read::<u32>(scene + 0x54).wrapping_add(at * 4));
                    copy_string(h, out, src);
                    h.write(scene + 0x64, k.p);
                    #[allow(clippy::cast_possible_truncation)] // `mov word`
                    h.write(scene + 0x70, at as u16);
                    h.write(scene + 0x66, k.fmt);
                    return 2;
                }
            }
        }
        0
    }
}
