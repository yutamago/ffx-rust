//! The render-side glue, thirty-fourth batch (M4 port round 8, list
//! `h3-glue-render`): the field map's asynchronous load request and the
//! asset header read. The exception frames and cookies are left out, as
//! in [`crate::glue10`].

use ffx_bridge::Global;

/// `operator new[]` and `delete[]`, `operator delete` (cdecl),
/// `PhyreScene_convertMapPath(this; path, out, b)` (`ret 0xc`), the MSVC
/// string's `assign(this; p, n)` (`ret 8`),
/// `AssetListReader_getPlatformSpecificPath(out, string)` (cdecl, the
/// string by value: 7 words), `fiosIsFileExist(path)` (cdecl), `strstr`
/// (cdecl, its import slot), `"znkd12"`, `g_PhyreScene3D`,
/// `AsyncLoadManager_sGetInstance`, `registAsyncLoadTask` (cdecl) and
/// the task `PhyreScene_asyncDoFieldMapLoadTask`.
pub const OPERATOR_NEW_ARRAY: u32 = 0x0063_04c0;
pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const CONVERT_MAP_PATH: u32 = 0x0065_9290;
pub const STRING_ASSIGN: u32 = 0x0040_4270;
pub const PLATFORM_PATH: u32 = 0x0069_0590;
pub const FILE_EXISTS: u32 = 0x0067_96f0;
pub const STRSTR: u32 = 0x00b0_c378;
pub const ZNKD12: u32 = 0x00b4_2364;
pub const PHYRE_SCENE_3D: Global<u8> = Global::new(0x00cc_b9c0);
pub const ASYNC_LOAD_INSTANCE: u32 = 0x0069_1b40;
pub const ASYNC_REGIST_TASK: u32 = crate::glue2::ASYNC_REGIST_TASK;
pub const FIELD_MAP_LOAD_TASK: u32 = 0x0065_7820;
/// The map whose load also clears `+0xf9c4`.
pub const MAP_220: u32 = 0x220;
/// The error for a missing map file.
pub const NO_FILE: u32 = 0x13;

/// `PStreamReaderFile`'s constructor (`this; path`, `ret 4`),
/// `isOpen`, `close` and destructor (`this`), the header's name list
/// `0x00667770(vector; path)` (`ret 4`) and
/// `AssetListReader_readAssetHeader(this; name)` (`ret 4`).
pub const READER_CTOR: u32 = 0x0057_66d0;
pub const READER_IS_OPEN: u32 = 0x0057_6980;
pub const READER_CLOSE: u32 = 0x0057_6930;
pub const READER_DTOR: u32 = 0x0057_67c0;
pub const READ_NAMES: u32 = 0x0066_7770;
pub const ADD_ASSET_HEADER: u32 = 0x006f_4900;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// `strlen`.
    fn strlen(h: &impl Host, p: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(p.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// A cdecl import through its slot.
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// The high word of a sign-extended `cdq`.
    fn high(v: u32) -> u32 {
        if v & 0x8000_0000 == 0 { 0 } else { u32::MAX }
    }

    /// `PhyreScene_fieldMapLoadAsync(this, path, b)` (thiscall, `ret 8`;
    /// frame: the converted path at `ebp - 0x110`, the platform path (an
    /// MSVC string) at `- 0x128`, the copy of `path` at `- 0x130`, three
    /// zeroed words at `- 0x13c`, the by-value string handed to
    /// `getPlatformSpecificPath` at `- 0x164`): the scene marks a load
    /// (`+0x1195`), keeps a copy of `path` (`new[]`), converts it and
    /// checks the file; a missing file clears the mark, frees the copy
    /// and returns 0x13. Else `znkd12` maps and map 0x220 clear
    /// `+0xf9c4`; `b` (its low byte) sets `g_PhyreScene3D`, else the
    /// scene's 2D flag `+0x93c` and clears `+0xf95c..+0xf968`; `+0xd8`
    /// is cleared and `asyncDoFieldMapLoadTask` (this, the copy, `b`) is
    /// registered. Returns 0.
    pub fn field_map_load_async(h: &mut impl Host, ebp: u32, this: u32, path: u32, b: u32) -> u32 {
        h.write(this + 0x1195, 1u8);
        let n = strlen(h, path);
        let copy = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[n.wrapping_add(1)]);
        h.write(ebp - 0x130, copy);
        let mut k = 0u32;
        loop {
            let c: u8 = h.read(path.wrapping_add(k));
            h.write(copy.wrapping_add(k), c);
            if c == 0 {
                break;
            }
            k = k.wrapping_add(1);
        }
        let converted = ebp - 0x110;
        h.call_sym(
            CONVERT_MAP_PATH,
            Conv::Thiscall(this),
            &[path, converted, b],
        );
        for at in [0x13c, 0x138, 0x134] {
            h.write(ebp - at, 0u32);
        }
        let string = ebp - 0x164;
        h.write(string + 0x14, 0xfu32);
        h.write(string + 0x10, 0u32);
        h.write(string, 0u8);
        let len = if h.read::<u8>(converted) == 0 {
            0
        } else {
            h.write(ebp - 0x12c, converted + 1);
            strlen(h, converted)
        };
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(string), &[converted, len]);
        let platform = ebp - 0x128;
        // the out pointer, then the string's six words by value
        let args: [u32; 7] = std::array::from_fn(|k| {
            if k == 0 {
                platform
            } else {
                h.read(string + 4 * (k as u32 - 1))
            }
        });
        h.call_sym(PLATFORM_PATH, Conv::Cdecl, &args);
        let heap = h.read::<u32>(platform + 0x14) >= 0x10;
        let text = if heap { h.read(platform) } else { platform };
        if h.call_sym(FILE_EXISTS, Conv::Cdecl, &[text]) == 0 {
            h.write(this + 0x1195, 0u8);
            h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[copy]);
            if h.read::<u32>(platform + 0x14) >= 0x10 {
                let data: u32 = h.read(platform);
                h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[data]);
            }
            return NO_FILE;
        }
        if h.read::<u32>(platform + 0x14) >= 0x10 {
            let data: u32 = h.read(platform);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[data]);
        }
        let znkd = h.rebase(ZNKD12);
        if import(h, STRSTR, &[path, znkd]) != 0 || h.read::<u32>(this + 0xfaf4) == MAP_220 {
            h.write(this + 0xf9c4, 0u8);
        }
        let flag = b as u8;
        if flag != 0 {
            PHYRE_SCENE_3D.set(h, 1);
        } else {
            h.write(this + 0x93c, 1u8);
            for at in [0xf95c, 0xf960, 0xf968, 0xf964] {
                h.write(this + at, 0u32);
            }
        }
        h.write(this + 0xd8, 0u32);
        let manager = h.call_sym(ASYNC_LOAD_INSTANCE, Conv::Cdecl, &[]);
        let task = h.rebase(FIELD_MAP_LOAD_TASK);
        h.call_sym(
            ASYNC_REGIST_TASK,
            Conv::Cdecl,
            &[
                manager,
                task,
                3,
                this,
                high(this),
                copy,
                high(copy),
                u32::from(flag),
                0,
            ],
        );
        0
    }

    /// `PhyreScene_readAssetHeader(this, path, b)` (thiscall, `ret 8`;
    /// frame: the converted path at `ebp - 0x110`, the platform path at `-
    /// 0x128`, a vector of MSVC strings at `- 0x138`, the file reader at
    /// `- 0x154`, the by-value string at `- 0x17c`): the converted path's
    /// platform file is opened; a missing one returns 5. Else the file is
    /// closed, the header's names are read into the vector
    /// (`0x00667770`) and each goes to the scene's asset list (`+0x10e4`
    /// with `b`'s low byte, else `+0x1144`); the names and the vector are
    /// freed. Returns 0.
    pub fn read_asset_header(h: &mut impl Host, ebp: u32, this: u32, path: u32, b: u32) -> u32 {
        let converted = ebp - 0x110;
        h.call_sym(
            CONVERT_MAP_PATH,
            Conv::Thiscall(this),
            &[path, converted, b],
        );
        let names = ebp - 0x138;
        for k in 0..3 {
            h.write(names + 4 * k, 0u32);
        }
        let string = ebp - 0x17c;
        h.write(string + 0x14, 0xfu32);
        h.write(string + 0x10, 0u32);
        h.write(string, 0u8);
        let len = strlen(h, converted);
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(string), &[converted, len]);
        let platform = ebp - 0x128;
        // the out pointer, then the string's six words by value
        let args: [u32; 7] = std::array::from_fn(|k| {
            if k == 0 {
                platform
            } else {
                h.read(string + 4 * (k as u32 - 1))
            }
        });
        h.call_sym(PLATFORM_PATH, Conv::Cdecl, &args);
        let text = if h.read::<u32>(platform + 0x14) >= 0x10 {
            h.read(platform)
        } else {
            platform
        };
        let reader = ebp - 0x154;
        h.call_sym(READER_CTOR, Conv::Thiscall(reader), &[text]);
        let open = h.call_sym(READER_IS_OPEN, Conv::Thiscall(reader), &[]) as u8;
        if open == 0 {
            h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
            free_string(h, platform);
            return 5;
        }
        h.call_sym(READER_CLOSE, Conv::Thiscall(reader), &[]);
        h.call_sym(READ_NAMES, Conv::Thiscall(names), &[platform]);
        h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
        free_string(h, platform);
        let end: u32 = h.read(names + 4);
        let begin: u32 = h.read(names);
        #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)] // the signed / 24
        let count = (end.wrapping_sub(begin) as i32 / 0x18) as u32;
        let list = if b as u8 == 0 {
            this + 0x1144
        } else {
            this + 0x10e4
        };
        let mut at = begin;
        for _ in 0..count {
            let name = if h.read::<u32>(at + 0x14) >= 0x10 {
                h.read(at)
            } else {
                at
            };
            h.call_sym(ADD_ASSET_HEADER, Conv::Thiscall(list), &[name]);
            at = at.wrapping_add(0x18);
        }
        let begin: u32 = h.read(names);
        if begin != 0 {
            let end: u32 = h.read(names + 4);
            let mut at = begin;
            while at != end {
                free_string(h, at);
                h.write(at + 0x14, 0xfu32);
                h.write(at + 0x10, 0u32);
                h.write(at, 0u8);
                at = at.wrapping_add(0x18);
            }
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[begin]);
        }
        0
    }

    /// An MSVC string's heap buffer freed when it has one (capacity 16 or
    /// more).
    fn free_string(h: &mut impl Host, string: u32) {
        if h.read::<u32>(string + 0x14) >= 0x10 {
            let data: u32 = h.read(string);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[data]);
        }
    }
}
