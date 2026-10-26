//! Originals that hand the addresses of their own locals to callees: the
//! character shader clusters' loads, the window-event handler tree's
//! insert, a (name, value) pair insert.
//!
//! Each bridge takes `ebp`: the address the original's `ebp` would have,
//! with its locals at the original's (negative) offsets and, where the
//! original passes the address of its own argument, that argument at
//! `ebp + 8`. The shims pass a `StackFrame` laid out that way; the oracle
//! tests put it where the original's frame is (`Oracle::set_call_top`),
//! so callees see the same pointers.

use ffx_graphic::host::{Conv, Host};

/// `ClusterManager_GetInstance` (cdecl).
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
/// `ClusterManager_loadPCluster` (thiscall `(mgr, path)`, `ret 4`).
pub const LOAD_PCLUSTER: u32 = 0x0069_b940;
/// `Phyre_PFramework_PApplication_GetApplication` (cdecl).
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `PApplication_FixupClusters` (cdecl `(clusters, n)`).
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;
/// `PhyreSceneCursor_Seek3` (thiscall `(cursor)`).
pub const CURSOR_SEEK3: u32 = 0x0065_ec40;
/// The class descriptor the shader clusters' cursors look for.
pub const SHADER_DESC: u32 = 0x00ca_5ff0;

/// One character shader cluster: its path, the cluster global, the
/// global for what the cursor found.
#[derive(Debug, Clone, Copy)]
pub struct ShaderCluster {
    /// The `.cgfx.phyre` path (`.rdata`).
    pub path: u32,
    /// Where the loaded cluster goes.
    pub cluster: u32,
    /// Where the found object's address goes.
    pub found: u32,
}

/// `0x006df3d0`: `CharacterBtmToTop.cgfx.phyre`.
pub const BTM_TO_TOP: ShaderCluster = ShaderCluster {
    path: 0x00b4_a710,
    cluster: 0x00cd_edf8,
    found: 0x00cd_ee08,
};
/// `0x006df470`: `MagicFile249.cgfx.phyre`.
pub const MAGIC_249: ShaderCluster = ShaderCluster {
    path: 0x00b4_a754,
    cluster: 0x00cd_edfc,
    found: 0x00cd_ee0c,
};
/// `0x006df510`: `MagicFile297.cgfx.phyre`.
pub const MAGIC_297: ShaderCluster = ShaderCluster {
    path: 0x00b4_a794,
    cluster: 0x00cd_ee00,
    found: 0x00cd_ee10,
};

/// The window-event handler tree (`0x00cc9ee4`).
pub const HANDLER_TREE: u32 = 0x00cc_9ee4;
/// The byte `0x00625c70` passes the tree's insert.
pub const HANDLER_FLAG: u32 = 0x00cc_9d1c;
/// `0x0061eeb0` (thiscall `(tree, out, hint, key, flag)`, `ret 0x10`):
/// the tree's insert; returns `out`.
pub const TREE_INSERT: u32 = 0x0061_eeb0;

/// `StdString_assignSubstr_00403a10` (thiscall `(str, src, pos, n)`).
pub const STRING_ASSIGN: u32 = 0x0040_3a10;
/// `0x00649540` (thiscall `(map, pair, x)`, `ret 8`).
pub const PAIR_INSERT: u32 = 0x0064_9540;
/// `operator_delete` (cdecl `(p)`).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        CLUSTER_MANAGER, CURSOR_SEEK3, Conv, FIXUP_CLUSTERS, GET_APPLICATION, HANDLER_FLAG,
        HANDLER_TREE, Host, LOAD_PCLUSTER, OPERATOR_DELETE, PAIR_INSERT, SHADER_DESC,
        STRING_ASSIGN, ShaderCluster, TREE_INSERT,
    };

    /// The shader cluster loads `0x006df3d0`, `0x006df470`, `0x006df510`
    /// (cdecl `()`): the cluster from `path` (`ClusterManager_GetInstance()
    /// .loadPCluster(path)`) into its global; when there is one,
    /// `PApplication_FixupClusters(&global, 1)` and a cursor (`ebp -
    /// 0x20`, 8 words) over its instance lists (`+0x1c`) for
    /// [`SHADER_DESC`]; what it finds (`[ebp-0x10] + [ebp-0x20]`, when
    /// `[ebp-0x14]` is set) into `found`. Returns that sum, the seek's
    /// `eax`, or 0 without a cluster (`Phyre_CharacterShaderManager_
    /// loadShader` returns the last one's).
    pub fn load_shader_cluster(h: &mut impl Host, ebp: u32, c: &ShaderCluster) -> u32 {
        let path = h.rebase(c.path);
        let mgr = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[path]);
        let cl = h.call_sym(LOAD_PCLUSTER, Conv::Thiscall(mgr), &[path]);
        let g = h.rebase(c.cluster);
        h.write(g, cl);
        if cl == 0 {
            return 0;
        }
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[g, 1]);
        let cl: u32 = h.read(g);
        let lists = cl.wrapping_add(0x1c);
        let first: u32 = h.read(lists);
        let first = if first == lists { 0 } else { first };
        let cur = ebp - 0x20;
        for at in [0, 4, 8, 0xc, 0x10] {
            h.write(cur + at, 0u32);
        }
        h.write(cur + 0x14, lists);
        h.write(cur + 0x18, first);
        h.write(cur + 0x1c, h.rebase(SHADER_DESC));
        let r = h.call_sym(CURSOR_SEEK3, Conv::Thiscall(cur), &[]);
        if h.read::<u32>(ebp - 0x14) == 0 {
            return r;
        }
        let a: u32 = h.read(ebp - 0x10);
        let b: u32 = h.read(ebp - 0x20);
        let v = a.wrapping_add(b);
        h.write(h.rebase(c.found), v);
        v
    }

    /// `0x00625c70(key)` (cdecl): inserts `key` into the window-event
    /// handler tree (`0x0061eeb0(tree, ebp - 8, 0, &key, [0x00cc9d1c])`,
    /// the key at `ebp + 8`), whose `eax` (the out pair's address) it
    /// returns.
    pub fn insert_handler(h: &mut impl Host, ebp: u32, key: u32) -> u32 {
        h.write(ebp + 8, key);
        let flag = u32::from(h.read::<u8>(h.rebase(HANDLER_FLAG)));
        let tree = h.rebase(HANDLER_TREE);
        h.call_sym(
            TREE_INSERT,
            Conv::Thiscall(tree),
            &[ebp - 8, 0, ebp + 8, flag],
        )
    }

    /// `0x0064bde0(map, name, value)` (cdecl) -> `map`: a pair at `ebp -
    /// 0x20` of a `std::string` copy of `name` (`assign(name, 0,
    /// npos)` into an empty one) and `*value` (`ebp - 8`), inserted with
    /// `0x00649540(map, &pair, 0)`; the copy's heap buffer (capacity 16
    /// or more) is deleted after. `ebp - 0x24` gets a 0 no one reads.
    pub fn insert_named(h: &mut impl Host, ebp: u32, map: u32, name: u32, value: u32) -> u32 {
        let s = ebp - 0x20;
        h.write(ebp - 0x24, 0u32);
        h.write(ebp - 0xc, 0xfu32);
        h.write(ebp - 0x10, 0u32);
        h.write(s, 0u8);
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(s), &[name, 0, u32::MAX]);
        let v: u32 = h.read(value);
        h.write(ebp - 8, v);
        h.call_sym(PAIR_INSERT, Conv::Thiscall(map), &[s, 0]);
        if h.read::<u32>(ebp - 0xc) >= 0x10 {
            let buf: u32 = h.read(s);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[buf]);
        }
        map
    }
}

/// `Phyre_PUtf8_decodeNext_0045d640` (cdecl `(&cursor)` -> code point).
pub const UTF8_DECODE_NEXT: u32 = 0x0045_d640;

pub mod bridge2 {
    //! The originals over game memory.

    use super::{Conv, Host, UTF8_DECODE_NEXT};

    /// `Str_Utf8ToWide(src, dst, end)` (`0x00607c40`, cdecl) -> UTF-16
    /// units written, negated when a code point can't be converted (a
    /// surrogate, one past U+10FFFF, or a pair without room); 0 for a
    /// null pointer or `end < dst`. The cursor is the original's own
    /// argument slot, handed to the decoder: here `cursor` (a word the
    /// caller provides), which holds the final position after the call.
    /// Stops before `end` without a terminator; else ends with a 0 unit
    /// when there is room.
    pub fn utf8_to_wide(h: &mut impl Host, cursor: u32, src: u32, dst: u32, end: u32) -> u32 {
        if src == 0 || dst == 0 || end == 0 || end < dst {
            return 0;
        }
        h.write(cursor, src);
        let mut out = dst;
        let mut ok = true;
        loop {
            let at: u32 = h.read(cursor);
            if h.read::<u8>(at) != 0 {
                if out >= end {
                    // full: no terminator
                    break;
                }
                let cp = h.call_sym(UTF8_DECODE_NEXT, Conv::Cdecl, &[cursor]);
                if cp <= 0xffff {
                    if cp.wrapping_sub(0xd800) > 0x7ff {
                        h.write(out, cp as u16);
                        out += 2;
                        continue;
                    }
                } else if cp <= 0x10_ffff && out.wrapping_add(2) < end {
                    let v = cp - 0x1_0000;
                    h.write(out, ((v >> 10).wrapping_sub(0x2800)) as u16);
                    h.write(out + 2, ((v & 0x3ff).wrapping_sub(0x2400)) as u16);
                    out += 4;
                    continue;
                }
                ok = false;
            }
            if out < end {
                h.write(out, 0u16);
                out += 2;
            }
            break;
        }
        let n = ((out.wrapping_sub(dst)) as i32) >> 1;
        (if ok { n } else { n.wrapping_neg() }) as u32
    }
}

/// Import slots `CreateDocumentsFolders` calls through.
pub mod imports {
    /// `SHELL32!SHGetFolderPathW` (stdcall).
    pub const SH_GET_FOLDER_PATH_W: u32 = 0x00b0_c628;
    /// `MSVCR110!wcstombs` (cdecl).
    pub const WCSTOMBS: u32 = 0x00b0_c414;
    /// `MSVCR110!sprintf` (cdecl).
    pub const SPRINTF: u32 = 0x00b0_c3dc;
    /// `MSVCR110!mbstowcs` (cdecl).
    pub const MBSTOWCS: u32 = 0x00b0_c410;
    /// `KERNEL32!GetFileAttributesW` (stdcall).
    pub const GET_FILE_ATTRIBUTES_W: u32 = 0x00b0_c18c;
    /// `KERNEL32!CreateDirectoryW` (stdcall).
    pub const CREATE_DIRECTORY_W: u32 = 0x00b0_c194;
}
/// The `"%s%s"` format both paths are built with (`.rdata`).
pub const JOIN_FORMAT: u32 = 0x00b4_0eb0;
/// The pointer to `"\\Square Enix\\"`.
pub const SQUARE_ENIX: u32 = 0x00c3_0f9c;
/// The pointer to the game's own folder name.
pub const GAME_FOLDER: u32 = 0x00c3_0fa0;

pub mod bridge3 {
    //! The original over game memory.

    use super::{Conv, GAME_FOLDER, Host, JOIN_FORMAT, SQUARE_ENIX, imports};

    fn import(h: &mut impl Host, slot: u32) -> u32 {
        let at = h.rebase(slot);
        h.read(at)
    }

    /// Makes the folder when it is missing or not a directory
    /// (`GetFileAttributesW` is `INVALID_FILE_ATTRIBUTES` or lacks
    /// `FILE_ATTRIBUTE_DIRECTORY`); false when `CreateDirectoryW` fails.
    fn ensure(h: &mut impl Host, path: u32, create: &mut Option<u32>) -> bool {
        let f = import(h, imports::GET_FILE_ATTRIBUTES_W);
        let attr = h.call(f, Conv::Stdcall, &[path]);
        // the original loads CreateDirectoryW's slot once, here
        let c = *create.get_or_insert_with(|| import(h, imports::CREATE_DIRECTORY_W));
        if attr != u32::MAX && attr & 0x10 != 0 {
            return true;
        }
        h.call(c, Conv::Stdcall, &[path, 0]) != 0
    }

    /// `CreateDocumentsFolders()` (`0x0062ac90`, cdecl): `Documents`
    /// (`SHGetFolderPathW(0, CSIDL_PERSONAL, 0, 0, ebp - 0x1204)`, then
    /// `wcstombs` into `ebp - 0x604`); on success the folders
    /// `<Documents>\Square Enix\` (`sprintf` into `ebp - 0x204`, `mbstowcs`
    /// into `ebp - 0xa04`) and, when that one exists or could be made,
    /// `<it><game folder>` (`ebp - 0x404`, `ebp - 0xe04`) are created
    /// where missing. The buffers are the original's locals (0x1204 bytes
    /// below `ebp`); this code reads none of them itself.
    pub fn create_documents_folders(h: &mut impl Host, ebp: u32) {
        let (documents, documents_mb) = (ebp - 0x1204, ebp - 0x604);
        let (square, square_w) = (ebp - 0x204, ebp - 0xa04);
        let (game, game_w) = (ebp - 0x404, ebp - 0xe04);
        let f = import(h, imports::SH_GET_FOLDER_PATH_W);
        let hr = h.call(f, Conv::Stdcall, &[0, 5, 0, 0, documents]);
        let f = import(h, imports::WCSTOMBS);
        h.call(f, Conv::Cdecl, &[documents_mb, documents, 0x200]);
        if (hr as i32) < 0 {
            return;
        }
        let format = h.rebase(JOIN_FORMAT);
        let name: u32 = h.read(h.rebase(SQUARE_ENIX));
        let f = import(h, imports::SPRINTF);
        h.call(f, Conv::Cdecl, &[square, format, documents_mb, name]);
        let f = import(h, imports::MBSTOWCS);
        h.call(f, Conv::Cdecl, &[square_w, square, 0x200]);
        let mut create = None;
        if !ensure(h, square_w, &mut create) {
            return;
        }
        let name: u32 = h.read(h.rebase(GAME_FOLDER));
        let f = import(h, imports::SPRINTF);
        h.call(f, Conv::Cdecl, &[game, format, square, name]);
        let f = import(h, imports::MBSTOWCS);
        h.call(f, Conv::Cdecl, &[game_w, game, 0x200]);
        ensure(h, game_w, &mut create);
    }
}

/// The list `0x00681ad0` appends to: its head node (`+0`) and size
/// (`+4`).
pub const NAME_LIST: u32 = 0x00cc_c840;
/// `0x00404270` (thiscall `(str, p, n)`, `ret 8`): `std::string::assign(p,
/// n)`.
pub const STRING_ASSIGN_N: u32 = 0x0040_4270;
/// `0x00680490` (thiscall `(list, next, prev, &value)`, `ret 0xc`): a new
/// node holding a copy of the value, linked before `next`.
pub const LIST_BUY_NODE: u32 = 0x0068_0490;
/// `MSVCP110!std::_Xlength_error` (cdecl `(message)`, never returns).
pub const XLENGTH_ERROR: u32 = 0x00b0_c29c;
/// `"list<T> too long"`.
pub const LIST_TOO_LONG: u32 = 0x00b4_3ce0;
/// `std::list`'s `max_size` for these 0x24-byte nodes.
pub const LIST_MAX: u32 = 0x071c_71c6;

pub mod bridge4 {
    //! The original over game memory.

    use super::{
        Conv, Host, LIST_BUY_NODE, LIST_MAX, LIST_TOO_LONG, NAME_LIST, OPERATOR_DELETE,
        STRING_ASSIGN, STRING_ASSIGN_N, XLENGTH_ERROR,
    };

    /// An empty `std::string` at `s` (capacity 15, size 0, `""`).
    fn empty(h: &mut impl Host, s: u32) {
        h.write(s + 0x14, 0xfu32);
        h.write(s + 0x10, 0u32);
        h.write(s, 0u8);
    }

    /// `NameList_pushBack(name)` (`0x00681ad0`, cdecl): appends `{name,
    /// 0}` to the list at [`NAME_LIST`]: `name` into a `std::string` at
    /// `ebp - 0x44`, copied into the pair's string at `ebp - 0x2c` (the
    /// value 0 at `ebp - 0x14`), a node of that pair linked at the end
    /// (`std::_Xlength_error` past `max_size`, as the original), then
    /// both strings released (the copy also reset to empty).
    pub fn push_back(h: &mut impl Host, ebp: u32, name: u32) {
        let (s1, s2) = (ebp - 0x44, ebp - 0x2c);
        h.write(s1 + 0x14, 0xfu32);
        let mut len = 0u32;
        while h.read::<u8>(name.wrapping_add(len)) != 0 {
            len += 1;
        }
        h.write(s1 + 0x10, 0u32);
        h.write(s1, 0u8);
        h.call_sym(STRING_ASSIGN_N, Conv::Thiscall(s1), &[name, len]);
        empty(h, s2);
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(s2), &[s1, 0, u32::MAX]);
        let list = h.rebase(NAME_LIST);
        let head: u32 = h.read(list);
        h.write(ebp - 0x14, 0u32);
        let prev: u32 = h.read(head + 4);
        let node = h.call_sym(LIST_BUY_NODE, Conv::Thiscall(list), &[head, prev, s2]);
        let size: u32 = h.read(list + 4);
        if LIST_MAX.wrapping_sub(size) < 1 {
            let f: u32 = h.read(h.rebase(XLENGTH_ERROR));
            let message = h.rebase(LIST_TOO_LONG);
            h.call(f, Conv::Cdecl, &[message]);
        }
        h.write(list + 4, size.wrapping_add(1));
        h.write(head + 4, node);
        let prev: u32 = h.read(node + 4);
        h.write(prev, node);
        if h.read::<u32>(s2 + 0x14) >= 0x10 {
            let buf: u32 = h.read(s2);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[buf]);
        }
        let big = h.read::<u32>(s1 + 0x14) >= 0x10;
        empty(h, s2);
        if big {
            let buf: u32 = h.read(s1);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[buf]);
        }
    }
}
