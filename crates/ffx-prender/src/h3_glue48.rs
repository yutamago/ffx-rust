//! The render-side glue, forty-eighth batch (M4 port round 13, list
//! `h3-glue-render`): a character's load request. The exception frame
//! and cookie are left out, as in [`crate::glue10`].

/// The MSVC string's `assign(this; p, n)` (`ret 8`), its constructor
/// from a C string (`this; p`, `ret 4`), its move-assignment (`this;
/// other`, `ret 4`) and `compare(this; pos, n, p, m)` (`ret 0x10`);
/// `operator new`, `new[]` and `operator delete` (cdecl).
pub const STRING_ASSIGN: u32 = 0x0040_4270;
pub const STRING_CTOR: u32 = 0x0040_2c90;
pub const STRING_MOVE_ASSIGN: u32 = 0x0040_39b0;
pub const STRING_COMPARE: u32 = 0x0065_90d0;
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_NEW_ARRAY: u32 = 0x0063_04c0;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// The scene's loaded-character tree (`+0x11dc`, its head) and its
/// `lower_bound(tree; &key)` (`ret 4`).
pub const LOWER_BOUND: u32 = 0x0065_57a0;
/// `ClassLoadedCharacterTable_GetInstance()` (cdecl),
/// `insertPrototype(table; name, request)` (`ret 8`) and
/// `ChrMeshInstances_buildSuffixes_006a2390(request)`.
pub const TABLE_INSTANCE: u32 = 0x0069_f6c0;
pub const INSERT_PROTOTYPE: u32 = 0x006a_28b0;
pub const BUILD_SUFFIXES: u32 = 0x006a_2390;
/// The image's empty string, which skips the file check.
pub const EMPTY: u32 = 0x00b8_fbeb;
/// `AssetListReader_getPlatformSpecificPath(out, name)` (cdecl; `name`
/// a string by value, 6 words).
pub const PLATFORM_PATH: u32 = 0x0069_0590;
/// The `PStreamReaderFile` on the frame: constructor (`this; path`, `ret
/// 4`), `isOpen` (`al`), `close` and destructor.
pub const READER_CTOR: u32 = 0x0057_66d0;
pub const READER_IS_OPEN: u32 = 0x0057_6980;
pub const READER_CLOSE: u32 = 0x0057_6930;
pub const READER_DTOR: u32 = 0x0057_67c0;
/// The asset list reader: constructor (`this`), `readAssetHeader(this;
/// name)`, `getAssetCount(this)`, `getAssetPath(this; i)` (`ret 4`).
pub const ASSET_READER_CTOR: u32 = 0x0069_04c0;
pub const READ_ASSET_HEADER: u32 = 0x006f_4900;
pub const ASSET_COUNT: u32 = 0x006f_4170;
pub const ASSET_PATH: u32 = 0x006f_41b0;
/// `AsyncLoadManager_sGetInstance()` (cdecl),
/// `createChildrenRequestList(manager; n)` (`ret 4`),
/// `loadCluster(manager, ...)` (cdecl; 21 words here) and the callback
/// `PhyreScene_onCharacterAssetLoaded`.
pub const ASYNC_LOAD_INSTANCE: u32 = 0x0069_1b40;
pub const CREATE_CHILDREN: u32 = 0x0069_0f70;
pub const LOAD_CLUSTER: u32 = 0x0069_1660;
pub const ON_ASSET_LOADED: u32 = 0x0066_63d0;

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

    /// `cdq`'s high word.
    fn high(v: u32) -> u32 {
        if v & 0x8000_0000 == 0 { 0 } else { u32::MAX }
    }

    /// An empty MSVC string at `s` (capacity 15, size 0, the text's
    /// first byte 0, in that order).
    fn clear_string(h: &mut impl Host, s: u32) {
        h.write(s + 0x14, 0xfu32);
        h.write(s + 0x10, 0u32);
        h.write(s, 0u8);
    }

    /// The string's text: its buffer from capacity 16 on, else inline.
    fn text(h: &impl Host, s: u32) -> u32 {
        if h.read::<u32>(s + 0x14) >= 0x10 {
            h.read(s)
        } else {
            s
        }
    }

    /// `operator delete` of a string's buffer when it has one.
    fn free_string(h: &mut impl Host, s: u32) {
        if h.read::<u32>(s + 0x14) >= 0x10 {
            let data: u32 = h.read(s);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[data]);
        }
    }

    /// `PhyreScene_loadCharacter(this; name, request)` (thiscall, `ret
    /// 8`; frame from `ebp - 0xa8`; `scratch`: 0x18 bytes for the string
    /// the original builds in its argument area) -> 0, or 5 when the
    /// character's file is missing. The name, as a string (`ebp - 0x40`),
    /// is looked up in the scene's loaded-character tree (`+0x11dc`):
    /// found, the request takes the node's value (`+4`, the word at the
    /// node's `+0x28`), is marked (`+0x1c`) and goes to the loaded
    /// character table, and its suffixes are built. Not found: unless the
    /// name is the image's empty string, its platform path (`ebp - 0x28`,
    /// from the temporary at `ebp - 0x58`) is opened with a
    /// `PStreamReaderFile` at `ebp - 0xa8`; missing, the request is
    /// marked and the result is 5. Else [`load_assets`].
    pub fn load_character(
        h: &mut impl Host,
        ebp: u32,
        scratch: u32,
        this: u32,
        name: u32,
        request: u32,
    ) -> u32 {
        h.write(ebp - 0x60, this);
        h.write(ebp - 0x64, request);
        let key = ebp - 0x40;
        clear_string(h, key);
        let len = if h.read::<u8>(name) == 0 {
            0
        } else {
            strlen(h, name)
        };
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(key), &[name, len]);
        let tree = this + 0x11dc;
        let node = h.call_sym(LOWER_BOUND, Conv::Thiscall(tree), &[key]);
        let mut found = h.read::<u32>(tree);
        if node != found {
            let node_text = text(h, node + 0x10);
            let node_len: u32 = h.read(node + 0x20);
            let key_len: u32 = h.read(ebp - 0x30);
            let order = h.call_sym(
                STRING_COMPARE,
                Conv::Thiscall(key),
                &[0, key_len, node_text, node_len],
            );
            found = if (order as i32) < 0 {
                h.read(tree)
            } else {
                node
            };
        }
        h.write(ebp - 0x5c, found);
        if found != h.read::<u32>(tree) {
            let value: u32 = h.read(h.read::<u32>(found + 0x28));
            h.write(request + 4, value);
            h.write(request + 0x1c, 1u8);
            let table = h.call_sym(TABLE_INSTANCE, Conv::Cdecl, &[]);
            h.call_sym(INSERT_PROTOTYPE, Conv::Thiscall(table), &[name, request]);
            h.call_sym(BUILD_SUFFIXES, Conv::Thiscall(request), &[]);
            free_string(h, key);
            return 0;
        }
        let path = ebp - 0x28;
        clear_string(h, path);
        let result = if name == h.rebase(EMPTY) || file_exists(h, ebp, scratch, name, path) {
            load_assets(h, ebp, this, name, request);
            0
        } else {
            h.write(request + 0x1c, 1u8);
            5
        };
        free_string(h, path);
        clear_string(h, path);
        free_string(h, key);
        result
    }

    /// Whether the platform path of `name` (into the string at `path`)
    /// opens; see [`load_character`]. The reader is closed and destroyed
    /// either way.
    fn file_exists(h: &mut impl Host, ebp: u32, scratch: u32, name: u32, path: u32) -> bool {
        h.call_sym(STRING_CTOR, Conv::Thiscall(scratch), &[name]);
        let mut args = [ebp - 0x58, 0, 0, 0, 0, 0, 0];
        for (k, w) in args[1..].iter_mut().enumerate() {
            *w = h.read(scratch + 4 * k as u32);
        }
        let got = h.call_sym(PLATFORM_PATH, Conv::Cdecl, &args);
        if got != path {
            free_string(h, path);
            clear_string(h, path);
            h.call_sym(STRING_MOVE_ASSIGN, Conv::Thiscall(path), &[got]);
        }
        free_string(h, ebp - 0x58);
        let file = text(h, path);
        let reader = ebp - 0xa8;
        h.call_sym(READER_CTOR, Conv::Thiscall(reader), &[file]);
        let open = h.call_sym(READER_IS_OPEN, Conv::Thiscall(reader), &[]) as u8 != 0;
        if open {
            h.call_sym(READER_CLOSE, Conv::Thiscall(reader), &[]);
        }
        h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
        open
    }

    /// `operator new(size)`, its words cleared when not null.
    fn new_cleared(h: &mut impl Host, size: u32) -> u32 {
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[size]);
        if p != 0 {
            for k in 0..size / 4 {
                h.write(p + 4 * k, 0u32);
            }
        }
        p
    }

    /// The asset list of `name`: a new reader (0x44 bytes; `ebp - 0x6c`,
    /// then `ebp - 0x5c`), two cleared blocks (0xc and 0x10 bytes), the
    /// header read, a copy of the name (`ebp - 0x68`), a children request
    /// list for the asset count (`ebp - 0x5c`), and one cluster load per
    /// asset with [`ON_ASSET_LOADED`] and the request's eight words as
    /// signed pairs (scene, 0x10 block, reader, 0xc block, request, name
    /// copy, index, 0), kept in the frame from `ebp - 0x8c`.
    fn load_assets(h: &mut impl Host, ebp: u32, this: u32, name: u32, request: u32) {
        let block = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x44]);
        h.write(ebp - 0x6c, block);
        let reader = if block == 0 {
            0
        } else {
            h.call_sym(ASSET_READER_CTOR, Conv::Thiscall(block), &[])
        };
        h.write(ebp - 0x5c, reader);
        let small = new_cleared(h, 0xc);
        h.call_sym(READ_ASSET_HEADER, Conv::Thiscall(reader), &[name]);
        let big = new_cleared(h, 0x10);
        let len = strlen(h, name);
        let copy = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[len.wrapping_add(1)]);
        h.write(ebp - 0x68, copy);
        let mut k = 0u32;
        loop {
            let c: u8 = h.read(name.wrapping_add(k));
            h.write(copy.wrapping_add(k), c);
            if c == 0 {
                break;
            }
            k = k.wrapping_add(1);
        }
        let count = h.call_sym(ASSET_COUNT, Conv::Thiscall(reader), &[]);
        let manager = h.call_sym(ASYNC_LOAD_INSTANCE, Conv::Cdecl, &[]);
        let list = h.call_sym(CREATE_CHILDREN, Conv::Thiscall(manager), &[count]);
        h.write(ebp - 0x5c, list);
        if h.call_sym(ASSET_COUNT, Conv::Thiscall(reader), &[]) as i32 <= 0 {
            return;
        }
        for (at, v) in [
            (0x68, copy),
            (0x88, request),
            (0x64, high(request)),
            (0x78, small),
            (0x74, high(small)),
            (0x70, reader),
            (0x80, high(reader)),
            (0x7c, big),
            (0x84, high(big)),
            (0x6c, high(copy)),
            (0x8c, this),
            (0x60, high(this)),
        ] {
            h.write(ebp - at, v);
        }
        let callback = h.rebase(ON_ASSET_LOADED);
        let mut index = 0u32;
        loop {
            let path = h.call_sym(ASSET_PATH, Conv::Thiscall(reader), &[index]);
            let manager = h.call_sym(ASYNC_LOAD_INSTANCE, Conv::Cdecl, &[]);
            h.call_sym_wide(
                LOAD_CLUSTER,
                &[
                    manager,
                    list,
                    path,
                    callback,
                    8,
                    this,
                    high(this),
                    big,
                    high(big),
                    reader,
                    high(reader),
                    small,
                    high(small),
                    request,
                    high(request),
                    copy,
                    high(copy),
                    index,
                    high(index),
                    0,
                    0,
                ],
            );
            index = index.wrapping_add(1);
            if (index as i32) >= h.call_sym(ASSET_COUNT, Conv::Thiscall(reader), &[]) as i32 {
                break;
            }
        }
    }
}
