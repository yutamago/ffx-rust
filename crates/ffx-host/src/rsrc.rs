//! `FindResourceW`, `FindResourceExW`, `LoadResource`, `LockResource`
//! and `SizeofResource` over the resource directory of an image in the
//! arena (docs/host.md, "Imports").
//!
//! The game reads its message strings (`RT_STRING` blocks 0x784 and
//! 0x78b of `FFX.exe`, eight languages each) through ATL's string loader:
//! `AtlFindStringResource_0061b270` (the error boxes of
//! `BigFileStream_fileTaskUncompress` and `Game_Main`, the window's
//! Alt+F4 question), `AtlFindStringResourceInstance_0064f970` and
//! `CStringW_LoadString_00652310` (`Game_InitFileSystem`'s and
//! `PhyreFIOS_LoadMagicfilePrx`'s boxes). `.rsrc` is mapped with the
//! image, so a resource handle is the game address of its
//! `IMAGE_RESOURCE_DATA_ENTRY` and a loaded resource the game address of
//! its bytes, as Windows gives them for a mapped image.
//!
//! The language: Windows picks by the thread's UI language, so the
//! original shows German text on a German Windows. The host is
//! deterministic: neutral first, then en-US (0x409), then the first
//! language the directory lists.

use ffx_bridge::Mem;

/// `LANG_ENGLISH`, `SUBLANG_ENGLISH_US`.
const EN_US: u32 = 0x409;
/// An entry's high bit: a name string (in `Name`) or a subdirectory (in
/// `OffsetToData`).
const HIGH: u32 = 0x8000_0000;

/// A resource type or name as the caller gives it: an integer
/// (`MAKEINTRESOURCE`, below 0x10000) or a UTF-16 string.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Key {
    Id(u32),
    Name(Vec<u16>),
}

fn key(mem: &impl Mem, v: u32) -> Key {
    if v < 0x1_0000 {
        return Key::Id(v);
    }
    let mut s = Vec::new();
    let mut p = v;
    while s.len() < 256 {
        let c: u16 = mem.read(p);
        if c == 0 {
            break;
        }
        s.push(c);
        p += 2;
    }
    // "#123" names the integer 123, as Windows reads it
    if s.first() == Some(&u16::from(b'#'))
        && let Ok(n) = String::from_utf16_lossy(&s[1..]).parse::<u32>()
    {
        return Key::Id(n);
    }
    Key::Name(s)
}

/// The image's resource directory: its game address, or `None`.
fn root(mem: &impl Mem, module: u32) -> Option<u32> {
    if module == 0 || mem.read::<u16>(module) != 0x5a4d {
        return None;
    }
    let pe = module.wrapping_add(mem.read::<u32>(module + 0x3c));
    if mem.read::<u32>(pe) != 0x4550 {
        return None;
    }
    // the optional header's data directories start at +0x78 (PE32);
    // the resource directory is the third
    let rva: u32 = mem.read(pe + 0x78 + 2 * 8);
    (rva != 0).then(|| module.wrapping_add(rva))
}

/// The entries of directory `dir` (relative to `base`): (name, data).
fn entries(mem: &impl Mem, dir: u32) -> Vec<(u32, u32)> {
    let named = u32::from(mem.read::<u16>(dir + 12));
    let ids = u32::from(mem.read::<u16>(dir + 14));
    (0..(named + ids).min(4096))
        .map(|i| {
            let e = dir + 16 + 8 * i;
            (mem.read(e), mem.read(e + 4))
        })
        .collect()
}

/// Whether entry name `name` (relative to `base`) is `k`.
fn matches(mem: &impl Mem, base: u32, name: u32, k: &Key) -> bool {
    match k {
        Key::Id(id) => name & HIGH == 0 && name == *id,
        Key::Name(s) => {
            if name & HIGH == 0 {
                return false;
            }
            let at = base + (name & !HIGH);
            let n = usize::from(mem.read::<u16>(at));
            // names compare case-insensitively (Windows upper-cases the
            // argument; the directory holds them in upper case)
            n == s.len()
                && (0..n).all(|i| {
                    let c: u16 = mem.read(at + 2 + 2 * u32::try_from(i).unwrap_or(0));
                    upper(c) == upper(s[i])
                })
        }
    }
}

fn upper(c: u16) -> u16 {
    if (u16::from(b'a')..=u16::from(b'z')).contains(&c) {
        c - 0x20
    } else {
        c
    }
}

/// The subdirectory (relative to `base`) of the entry `k` in `dir`.
fn sub(mem: &impl Mem, base: u32, dir: u32, k: &Key) -> Option<u32> {
    entries(mem, dir)
        .into_iter()
        .find(|&(name, data)| data & HIGH != 0 && matches(mem, base, name, k))
        .map(|(_, data)| base + (data & !HIGH))
}

/// `FindResourceExW(module, type, name, language)` with `language`
/// `None` for `FindResourceW`: the game address of the resource's
/// `IMAGE_RESOURCE_DATA_ENTRY`, or 0.
pub fn find(mem: &impl Mem, module: u32, kind: u32, name: u32, language: Option<u32>) -> u32 {
    let Some(base) = root(mem, module) else {
        return 0;
    };
    let (kind, name) = (key(mem, kind), key(mem, name));
    let Some(types) = sub(mem, base, base, &kind) else {
        return 0;
    };
    let Some(langs) = sub(mem, base, types, &name) else {
        return 0;
    };
    let list: Vec<(u32, u32)> = entries(mem, langs)
        .into_iter()
        .filter(|&(n, data)| n & HIGH == 0 && data & HIGH == 0)
        .collect();
    let pick = |lang: u32| list.iter().find(|&&(n, _)| n == lang).copied();
    let found = match language {
        // an exact language, else the neutral one (`FindResourceEx`'s own
        // fallback for a language the module lacks)
        Some(l) => pick(l).or_else(|| pick(0)),
        None => pick(0)
            .or_else(|| pick(EN_US))
            .or_else(|| list.first().copied()),
    };
    found.map_or(0, |(_, data)| base + data)
}

/// `LoadResource(module, res)` (and `LockResource` of it): the game
/// address of the resource's bytes, or 0.
pub fn load(mem: &impl Mem, module: u32, res: u32) -> u32 {
    if res == 0 {
        return 0;
    }
    let base = if module == 0 {
        ffx_bridge::IMAGE_BASE
    } else {
        module
    };
    base.wrapping_add(mem.read::<u32>(res))
}

/// `SizeofResource(module, res)`: its size in bytes, or 0.
pub fn size(mem: &impl Mem, res: u32) -> u32 {
    if res == 0 { 0 } else { mem.read(res + 4) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffx_bridge::VecMem;

    const BASE: u32 = 0x0040_0000;

    /// A PE header and a resource directory: `RT_STRING` (6) block 0x784
    /// in German (0x407) and US English (0x409), and a named type "DATA"
    /// with id 7 in neutral; each resource's bytes are 4 words of its own.
    fn image() -> VecMem {
        let mut m = VecMem::new(BASE, 0x4000);
        m.write(BASE, 0x5a4du16);
        m.write(BASE + 0x3c, 0x80u32);
        m.write(BASE + 0x80, 0x4550u32);
        let rsrc = 0x1000u32;
        m.write(BASE + 0x80 + 0x78 + 16, rsrc);
        let r = BASE + rsrc;
        let dir = |m: &mut VecMem, at: u32, named: u16, ids: u16, list: &[(u32, u32)]| {
            m.write(at + 12, named);
            m.write(at + 14, ids);
            for (i, &(n, d)) in list.iter().enumerate() {
                let e = at + 16 + 8 * u32::try_from(i).unwrap();
                m.write(e, n);
                m.write(e + 4, d);
            }
        };
        // root: "DATA" (named, first) and 6
        dir(
            &mut m,
            r,
            1,
            1,
            &[(HIGH | 0x400, HIGH | 0x100), (6, HIGH | 0x40)],
        );
        m.write(r + 0x400, 4u16);
        for (i, c) in "DATA".encode_utf16().enumerate() {
            m.write(r + 0x402 + 2 * u32::try_from(i).unwrap(), c);
        }
        // type 6: block 0x784
        dir(&mut m, r + 0x40, 0, 1, &[(0x784, HIGH | 0x80)]);
        // block 0x784: 0x407, 0x409
        dir(&mut m, r + 0x80, 0, 2, &[(0x407, 0x200), (0x409, 0x210)]);
        // "DATA": id 7 -> neutral
        dir(&mut m, r + 0x100, 0, 1, &[(7, HIGH | 0x140)]);
        dir(&mut m, r + 0x140, 0, 1, &[(0, 0x220)]);
        for (i, entry) in [0x200u32, 0x210, 0x220].into_iter().enumerate() {
            let data = rsrc + 0x800 + 0x100 * u32::try_from(i).unwrap();
            m.write(r + entry, data);
            m.write(r + entry + 4, 16u32);
            m.write(BASE + data, [0x10 * u32::try_from(i).unwrap() + 1; 4]);
        }
        m
    }

    fn wide(m: &mut VecMem, at: u32, s: &str) -> u32 {
        for (i, c) in s.encode_utf16().chain([0]).enumerate() {
            m.write(at + 2 * u32::try_from(i).unwrap(), c);
        }
        at
    }

    #[test]
    fn find_resource_prefers_neutral_then_us_english() {
        let m = image();
        let r = BASE + 0x1000;
        // FindResourceW: no neutral entry, so 0x409 over the first (0x407)
        assert_eq!(find(&m, BASE, 6, 0x784, None), r + 0x210);
        // FindResourceExW: the exact language
        assert_eq!(find(&m, BASE, 6, 0x784, Some(0x407)), r + 0x200);
        // a language the block lacks, and no neutral entry: none
        assert_eq!(find(&m, BASE, 6, 0x784, Some(0x411)), 0);
        // another block, another type, no module
        assert_eq!(find(&m, BASE, 6, 0x785, None), 0);
        assert_eq!(find(&m, BASE, 3, 0x784, None), 0);
        assert_eq!(find(&m, 0, 6, 0x784, None), 0);
    }

    #[test]
    fn named_types_and_hash_ids() {
        let mut m = image();
        let r = BASE + 0x1000;
        let data = wide(&mut m, BASE + 0x3000, "data");
        let seven = wide(&mut m, BASE + 0x3100, "#7");
        assert_eq!(find(&m, BASE, data, 7, None), r + 0x220);
        assert_eq!(find(&m, BASE, data, seven, None), r + 0x220);
        // FindResourceExW in a language the resource lacks: the neutral one
        assert_eq!(find(&m, BASE, data, 7, Some(0x409)), r + 0x220);
        let other = wide(&mut m, BASE + 0x3200, "DAT");
        assert_eq!(find(&m, BASE, other, 7, None), 0);
    }

    #[test]
    fn load_and_size_give_the_bytes() {
        let m = image();
        let res = find(&m, BASE, 6, 0x784, None);
        let at = load(&m, BASE, res);
        assert_eq!(at, BASE + 0x1000 + 0x900);
        assert_eq!(m.read::<[u32; 4]>(at), [0x11; 4]);
        assert_eq!(size(&m, res), 16);
        assert_eq!((load(&m, BASE, 0), size(&m, 0)), (0, 0));
    }
}
