//! The render-side glue, fifty-second batch (M4 port round 15, list
//! `h3-glue-render`): what the scene does once all of a character's assets
//! are loaded. The exception frame and cookie are left out, as in
//! [`crate::glue10`].

/// The MSVC string's `assign(this; text, n)` (`ret 8`), `substr(this;
/// out, pos, n)` (`ret 0xc`, returns `out`), move-assign `(this; other)`
/// (`ret 4`), `rfind(this; &c, pos, n)` (`ret 0xc`) and `append(this;
/// text, n)` (`ret 8`).
pub const STRING_ASSIGN: u32 = 0x0040_4270;
pub const STRING_SUBSTR: u32 = 0x0040_2e20;
pub const STRING_MOVE_ASSIGN: u32 = 0x0040_39b0;
pub const STRING_RFIND: u32 = 0x0063_8340;
pub const STRING_APPEND: u32 = 0x0065_76d0;
/// `operator+(out, string*, text)` (cdecl, returns `out`).
pub const STRING_CONCAT: u32 = 0x0064_95b0;
/// `Phyre_PFramework_PApplication_GetApplication()` (cdecl; the result is
/// not used).
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `AssetListReader_getAssetCount(reader)` (thiscall), `getSymbolId(reader;
/// name)` (`ret 4`, -1 none) and the reader's destructor (thiscall).
pub const ASSET_COUNT: u32 = 0x006f_4170;
pub const GET_SYMBOL_ID: u32 = 0x006f_4350;
pub const READER_DTOR: u32 = 0x0063_4610;
/// `PApplication_FixupClusters(clusters, count)` (cdecl, 0 on success).
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;
/// `operator delete` and `operator delete[]` (cdecl), the user heap's
/// `free` (cdecl, a jump to 0x006fb8b0).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
pub const USER_FREE: u32 = 0x0068_7010;
/// The loaded-character tree: `make_pair(out, name string*, &value)`
/// (cdecl, returns `out`), `_Buynode(tree; pair)` (`ret 4`) and the
/// insert `(tree; &out, 0, key, node)` (`ret 0x10`).
pub const MAKE_PAIR: u32 = 0x0064_bde0;
pub const BUY_NODE: u32 = 0x0064_9b30;
pub const TREE_INSERT: u32 = 0x0064_a360;
/// `ShaderTexture_rebindAlt_006f6a90(this; owner, alt, force)` (`ret 0xc`).
pub const REBIND_ALT: u32 = 0x006f_6a90;
/// `fiosIsFileExist(name)`, `fiosOpen(name, mode)`, `fiosRead(stream, buf,
/// n)` and `fiosClose(stream)` (all cdecl).
pub const FIOS_IS_FILE_EXIST: u32 = 0x0067_96f0;
pub const FIOS_OPEN: u32 = 0x0067_9730;
pub const FIOS_READ: u32 = 0x0067_9800;
pub const FIOS_CLOSE: u32 = 0x0067_9510;
/// `report_rangecheckfailure()` (cdecl, never returns).
pub const RANGECHECK_FAILURE: u32 = 0x0094_9b02;
/// `TextureAnimationManager_GetInstance()` (cdecl) and
/// `addAnimatedMaterials(manager; model, array, 0, symbol, name, extra,
/// text)` (`ret 0x1c`).
pub const TEXTURE_ANIMATION_INSTANCE: u32 = 0x006b_02e0;
pub const ADD_ANIMATED_MATERIALS: u32 = 0x006b_05d0;
/// `ClassLoadedCharacterTable_GetInstance()` (cdecl),
/// `insertPrototype(table; name, request)` (`ret 8`) and
/// `ChrMeshInstances_buildSuffixes_006a2390(request)` (thiscall).
pub const TABLE_INSTANCE: u32 = 0x0069_f6c0;
pub const INSERT_PROTOTYPE: u32 = 0x006a_28b0;
pub const BUILD_SUFFIXES: u32 = 0x006a_2390;
/// `PhyreScene_chrLoadCountdown_00782fa0()` (cdecl).
pub const CHR_LOAD_COUNTDOWN: u32 = 0x0078_2fa0;
/// The image's `"textureAnimationChar"`, `"/textureAnimationChar.ovp"`
/// and empty string.
pub const TEXTURE_ANIMATION_CHAR: u32 = 0x00b4_2680;
pub const TEXTURE_ANIMATION_OVP: u32 = 0x00b4_2698;
pub const EMPTY: u32 = 0x00b8_fbeb;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::h3_glue49::bridge::{clear_string, text};
    use ffx_graphic::host::{Conv, Host};

    /// The frame's strings (below `ebp`): the name, the pair handed to
    /// the tree (its string; reused as a temporary), the name cut to its
    /// file stem, a temporary (later the `.ovp` file's text), the `.ovp`
    /// path, a second temporary and the read buffer (0x400 bytes).
    const NAME: u32 = 0x470;
    const PAIR: u32 = 0x48c;
    const STEM: u32 = 0x428;
    const TEMP: u32 = 0x458;
    const OVP: u32 = 0x440;
    const TEMP2: u32 = 0x488;
    const BUF: u32 = 0x410;
    /// The byte `rfind` searches for.
    const CHAR: u32 = 0x490;

    /// The C string's length.
    fn strlen(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The string's heap buffer freed (capacity 16 or more).
    fn free_buffer(h: &mut impl Host, s: u32) {
        if h.read::<u32>(s + 0x14) >= 0x10 {
            let p: u32 = h.read(s);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
        }
    }

    /// `s = std::move(*from)` unless `from` is `s`: `s`'s buffer freed,
    /// `s` emptied, then the original's move-assign.
    fn move_from(h: &mut impl Host, s: u32, from: u32) {
        if s != from {
            free_buffer(h, s);
            clear_string(h, s);
            h.call_sym(STRING_MOVE_ASSIGN, Conv::Thiscall(s), &[from]);
        }
    }

    /// `s` emptied and assigned the C string `name`.
    fn assign_name(h: &mut impl Host, s: u32, name: u32) {
        clear_string(h, s);
        let n = strlen(h, name);
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(s), &[name, n]);
    }

    /// The last index of byte `c` in `s` (`rfind(&c, npos, 1)`, the byte
    /// at `ebp - 0x490`).
    fn rfind(h: &mut impl Host, ebp: u32, s: u32, c: u8) -> u32 {
        h.write(ebp - CHAR, c);
        h.call_sym(STRING_RFIND, Conv::Thiscall(s), &[ebp - CHAR, u32::MAX, 1])
    }

    /// The reader destroyed and freed (when there is one), the asset
    /// array's data freed with the user heap, the array freed.
    fn free_inputs(h: &mut impl Host, ebp: u32, reader: u32) {
        if reader != 0 {
            h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[reader]);
        }
        let array: u32 = h.read(ebp - 0x494);
        let data: u32 = h.read(array.wrapping_add(8));
        if data != 0 {
            h.call_sym(USER_FREE, Conv::Cdecl, &[data]);
        }
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[array]);
    }

    /// `PhyreScene_doOnCharacterAllAssetsLoaded(this; out, reader, array,
    /// request, name, extra)` (thiscall, `ret 0x18`; frame from `ebp -
    /// 0x4ac`): the arguments kept at `ebp - 0x4a0` (`out`, also at
    /// `- 0x4a8`, the value the tree gets), `- 0x494` (`array`: count,
    /// capacity, data), `- 0x4a4` (`request`), `- 0x49c` (`extra`) and
    /// `- 0x498` (`this`, later the symbol id). The name copied into a
    /// string (`ebp - 0x470`); the array's clusters fixed up
    /// (`FixupClusters(data, reader's asset count)`). On failure the
    /// reader, the data and the array freed, `request + 0x1c` = 1, and
    /// the fix-up's code returned. Else the name paired with `out` and
    /// inserted into the scene's loaded-character tree (`+0x11dc`); the
    /// name's file stem (between its last `/` and its last `.`) looked up
    /// in the reader, and the asset of that symbol rebound
    /// (`rebindAlt(this; &data[id], 0, 1)`). With a `textureAnimationChar`
    /// symbol, the name's directory plus `/textureAnimationChar.ovp` read
    /// (when it exists and opens) into a string in chunks of 0x3ff bytes,
    /// and the manager's `addAnimatedMaterials(*out, array, 0, symbol,
    /// stem, extra, the text or 0)`. Then the inputs freed, `request + 4`
    /// = `*out`, the request inserted into the loaded-character table as
    /// the name's prototype, its instance suffixes built, `request + 0x1c`
    /// = 1, the name and `extra` freed (`delete[]`), the battle load
    /// countdown stepped; 0.
    #[allow(clippy::too_many_arguments)] // the original's six arguments
    pub fn on_character_all_assets_loaded(
        h: &mut impl Host,
        ebp: u32,
        this: u32,
        out: u32,
        reader: u32,
        array: u32,
        request: u32,
        name: u32,
        extra: u32,
    ) -> u32 {
        h.write(ebp - 0x498, this);
        h.write(ebp - 0x4a0, out);
        h.write(ebp - 0x4a8, out);
        h.write(ebp - 0x4a4, request);
        h.write(ebp - 0x494, array);
        h.write(ebp - 0x49c, extra);
        assign_name(h, ebp - NAME, name);
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let data: u32 = h.read(array.wrapping_add(8));
        let count = h.call_sym(ASSET_COUNT, Conv::Thiscall(reader), &[]);
        let failed = h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[data, count]);
        if failed != 0 {
            free_inputs(h, ebp, reader);
            let request: u32 = h.read(ebp - 0x4a4);
            h.write(request.wrapping_add(0x1c), 1u8);
            free_buffer(h, ebp - NAME);
            return failed;
        }
        // the tree: name -> out
        let pair = h.call_sym(
            MAKE_PAIR,
            Conv::Cdecl,
            &[ebp - PAIR, ebp - NAME, ebp - 0x4a8],
        );
        let this: u32 = h.read(ebp - 0x498);
        let tree = this.wrapping_add(0x11dc);
        let node = h.call_sym(BUY_NODE, Conv::Thiscall(tree), &[pair]);
        h.call_sym(
            TREE_INSERT,
            Conv::Thiscall(tree),
            &[ebp - 0x4ac, 0, node.wrapping_add(0x10), node],
        );
        free_buffer(h, ebp - PAIR);
        // the stem's asset rebound
        let stem = ebp - STEM;
        assign_name(h, stem, name);
        let slash = rfind(h, ebp, stem, b'/');
        let dot = rfind(h, ebp, stem, b'.');
        let cut = h.call_sym(
            STRING_SUBSTR,
            Conv::Thiscall(stem),
            &[
                ebp - TEMP,
                slash.wrapping_add(1),
                dot.wrapping_sub(slash).wrapping_sub(1),
            ],
        );
        move_from(h, stem, cut);
        free_buffer(h, ebp - TEMP);
        let stem_text = text(h, stem);
        let id = h.call_sym(GET_SYMBOL_ID, Conv::Thiscall(reader), &[stem_text]);
        if id != u32::MAX {
            let array: u32 = h.read(ebp - 0x494);
            let this: u32 = h.read(ebp - 0x498);
            let data: u32 = h.read(array.wrapping_add(8));
            h.call_sym(
                REBIND_ALT,
                Conv::Thiscall(this),
                &[data.wrapping_add(id.wrapping_mul(4)), 0, 1],
            );
        }
        let symbol = h.call_sym(
            GET_SYMBOL_ID,
            Conv::Thiscall(reader),
            &[h.rebase(TEXTURE_ANIMATION_CHAR)],
        );
        h.write(ebp - 0x498, symbol);
        if symbol != u32::MAX && !texture_animations(h, ebp, name) {
            return 0;
        }
        free_inputs(h, ebp, reader);
        let request: u32 = h.read(ebp - 0x4a4);
        let out: u32 = h.read(ebp - 0x4a0);
        let model: u32 = h.read(out);
        h.write(request.wrapping_add(4), model);
        let table = h.call_sym(TABLE_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(INSERT_PROTOTYPE, Conv::Thiscall(table), &[name, request]);
        h.call_sym(BUILD_SUFFIXES, Conv::Thiscall(request), &[]);
        h.write(request.wrapping_add(0x1c), 1u8);
        h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[name]);
        let extra: u32 = h.read(ebp - 0x49c);
        if extra != 0 {
            h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[extra]);
        }
        h.call_sym(CHR_LOAD_COUNTDOWN, Conv::Cdecl, &[]);
        free_buffer(h, stem);
        clear_string(h, stem);
        free_buffer(h, ebp - NAME);
        0
    }

    /// The `textureAnimationChar` step: the name's directory plus
    /// `/textureAnimationChar.ovp` (`ebp - 0x440`), its text read into
    /// `ebp - 0x458`, the manager's `addAnimatedMaterials`. False only
    /// after `report_rangecheckfailure` (a read longer than asked, which
    /// the original does not survive).
    fn texture_animations(h: &mut impl Host, ebp: u32, name: u32) -> bool {
        let ovp = ebp - OVP;
        assign_name(h, ovp, name);
        let slash = rfind(h, ebp, ovp, b'/');
        let dir = h.call_sym(STRING_SUBSTR, Conv::Thiscall(ovp), &[ebp - TEMP2, 0, slash]);
        move_from(h, ovp, dir);
        free_buffer(h, ebp - TEMP2);
        let path = h.call_sym(
            STRING_CONCAT,
            Conv::Cdecl,
            &[ebp - TEMP2, ovp, h.rebase(TEXTURE_ANIMATION_OVP)],
        );
        move_from(h, ovp, path);
        free_buffer(h, ebp - TEMP2);
        let contents = ebp - TEMP;
        clear_string(h, contents);
        h.call_sym(
            STRING_ASSIGN,
            Conv::Thiscall(contents),
            &[h.rebase(EMPTY), 0],
        );
        let mut loaded = 0;
        let ovp_text = text(h, ovp);
        if h.call_sym(FIOS_IS_FILE_EXIST, Conv::Cdecl, &[ovp_text]) != 0 {
            let ovp_text = text(h, ovp);
            let stream = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[ovp_text, 1]);
            if stream != 0 {
                let buf = ebp - BUF;
                loop {
                    let n = h.call_sym(FIOS_READ, Conv::Cdecl, &[stream, buf, 0x3ff]);
                    if n.cast_signed() <= 0 {
                        break;
                    }
                    if n >= 0x400 {
                        h.call_sym(RANGECHECK_FAILURE, Conv::Cdecl, &[]);
                        return false;
                    }
                    h.write(buf + n, 0u8);
                    let len = strlen(h, buf);
                    h.call_sym(STRING_APPEND, Conv::Thiscall(contents), &[buf, len]);
                }
                h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[stream]);
                loaded = text(h, contents);
            }
        }
        let stem_text = text(h, ebp - STEM);
        let extra: u32 = h.read(ebp - 0x49c);
        let symbol: u32 = h.read(ebp - 0x498);
        let array: u32 = h.read(ebp - 0x494);
        let out: u32 = h.read(ebp - 0x4a0);
        let model: u32 = h.read(out);
        let manager = h.call_sym(TEXTURE_ANIMATION_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(
            ADD_ANIMATED_MATERIALS,
            Conv::Thiscall(manager),
            &[model, array, 0, symbol, stem_text, extra, loaded],
        );
        free_buffer(h, contents);
        clear_string(h, contents);
        free_buffer(h, ovp);
        true
    }
}
