//! The render-side glue, forty-seventh batch (M4 port round 13, list
//! `h3-glue-render`): the VFX shader collection's loader and its
//! per-shader step. The exception frame and cookie are left out, as in
//! [`crate::glue10`].

/// `fiosUnifyFilename(src, dst, n)`, `fiosOpen(path, mode)`,
/// `fiosRead(stream, buf, n)` and `fiosClose(stream)` (cdecl).
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
pub const FIOS_OPEN: u32 = 0x0067_9730;
pub const FIOS_READ: u32 = 0x0067_9800;
pub const FIOS_CLOSE: u32 = 0x0067_9510;
/// `dbgPrintf(format, ...)`, `memset`'s thunk and
/// `report_rangecheckfailure` (cdecl; the last never returns).
pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const MEMSET: u32 = 0x0094_964c;
pub const RANGE_CHECK_FAILURE: u32 = 0x0094_9b02;
/// The import slots of `sprintf`, `strchr`, `strncpy` and `strstr`
/// (cdecl).
pub const SPRINTF: u32 = crate::glue9::SPRINTF;
pub const STRCHR: u32 = 0x00b0_c3c0;
pub const STRNCPY: u32 = 0x00b0_c368;
pub const STRSTR: u32 = 0x00b0_c378;
/// `"/FFX_Data/GameData/PS3Data/Shaders/"`, `"%s%s"`,
/// `"vfxShader_fx.txt"`, the missing-collection message, `"//"` and
/// `".phyre"`.
pub const SHADERS_FOLDER: u32 = 0x00b4_8f30;
pub const FORMAT_SS: u32 = 0x00b4_0eb0;
pub const COLLECTION: u32 = 0x00b4_8f1c;
pub const NO_COLLECTION: u32 = 0x00b4_8f54;
pub const COMMENT: u32 = 0x00b3_ece0;
pub const PHYRE_EXT: u32 = 0x00b4_8f90;
/// `VfxShaderManager_loadShader(this; dir, entry)` (`ret 8`),
/// `PhyreMalloc(size, align)` and `PhyreFree(p)` (cdecl),
/// `PArray_PushInto(array; dst, &value)` (`ret 8`), the application
/// getter and `PApplication_FixupClusters(clusters, n)` (cdecl).
pub const LOAD_SHADER: u32 = 0x006c_0a70;
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const PARRAY_PUSH: u32 = 0x0069_b650;
pub const GET_APPLICATION: u32 = 0x0062_3b90;
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;

/// `":"`, `"%s%s%s"`, `"D3D11/"` and the missing-shader message.
pub const COLON: u32 = 0x00b4_85cc;
pub const FORMAT_SSS: u32 = 0x00b4_2a3c;
pub const D3D11_FOLDER: u32 = 0x00b4_2a34;
pub const NO_SHADER: u32 = 0x00b4_91c4;
/// The `PStreamReaderFile` on the frame: its constructor (`this; path`,
/// `ret 4`), `isOpen` (`al`), `close` and destructor (`this` only).
pub const READER_CTOR: u32 = 0x0057_66d0;
pub const READER_IS_OPEN: u32 = 0x0057_6980;
pub const READER_CLOSE: u32 = 0x0057_6930;
pub const READER_DTOR: u32 = 0x0057_67c0;
/// `AsyncLoadManager_sGetInstance()` and
/// `AsyncLoadManager_loadCluster(manager, ...)` (cdecl; 9 words here),
/// the load callback `VfxShaderManager_onShaderLoaded_006c13e0`, and
/// `PRedBlackTree_insertNew_006c0810(tree; &key, &value)` (`ret 8`).
pub const ASYNC_LOAD_INSTANCE: u32 = 0x0069_1b40;
pub const LOAD_CLUSTER: u32 = 0x0069_1660;
pub const ON_SHADER_LOADED: u32 = 0x006c_13e0;
pub const TREE_INSERT_NEW: u32 = 0x006c_0810;

/// The first options found win alone (`VSP_GROUND_SPECIAL`,
/// `VSP_Z_BUF_CLEAR`, `VSP_PERS_Z_BUF_CLEAR`, `VSP_DTT_SPECIAL`,
/// `VSP_GRAY_SCALE`, `VSP_NOISE_BLUR_ADDITIVE`, `VSP_NOISE_BLUR`,
/// `VSP_SCREEN_FILTER`): string, flags.
pub const EXCLUSIVE: [(u32, u32); 8] = [
    (0x00b4_8f98, 0x8000_0000),
    (0x00b4_8fac, 0x4000_0000),
    (0x00b4_8fbc, 0x1000_0000),
    (0x00b4_8fd4, 0x1000),
    (0x00b4_8fe4, 0x4000),
    (0x00b4_8ff4, 0x0800_0000),
    (0x00b4_900c, 0x2000_0000),
    (0x00b4_901c, 0x1_0000),
];
/// `VSP_ZMAT` (1) and `VSP_ALPHA_MODULATE` (no flag; ends the blends).
pub const ZMAT: u32 = 0x00b4_9030;
pub const ALPHA_MODULATE: u32 = 0x00b4_903c;
/// The first blend found (`VSP_ALPHA_ADDITIVE`, `VSP_ALPHA_SUBSTRACT3`,
/// `VSP_ALPHA_SUBSTRACT2`, `VSP_ALPHA_SUBSTRACT`).
pub const BLENDS: [(u32, u32); 4] = [
    (0x00b4_9050, 2),
    (0x00b4_9064, 0x100),
    (0x00b4_907c, 0x20),
    (0x00b4_9094, 0x10),
];
/// Every option found (`VSP_ENABLE_TEXTURE`, `VSP_ZWRITE_ENABLE`,
/// `VSP_ALPHA_TEST_ENABLED`, `VSP_ZTEST_DISABLE`,
/// `VSP_TK_RUNTIME_CLUT_16x16`, `VSP_ALPHA_NO_BLENDING`,
/// `VSP_EXTENDED_ALPHA`, `VSP_ENABLE_NCLIP`, `VSP_SHIFT_ON_SCREEN`,
/// `VSP_LOGO_SPECIAL`, `VSP_POINTLIGHT`, `VSP_TEXTURE_VIDEO`,
/// `VSP_LOGO_TEXTURE_VIDEO`, `VSP_YIQ`).
pub const OPTIONS: [(u32, u32); 14] = [
    (0x00b4_90a8, 8),
    (0x00b4_90bc, 0x40),
    (0x00b4_90d0, 0x10_0000),
    (0x00b4_90e8, 0x800),
    (0x00b4_90fc, 0x80),
    (0x00b4_9118, 0x200),
    (0x00b4_9130, 0x400),
    (0x00b4_9144, 0x8000),
    (0x00b4_9158, 0x2000),
    (0x00b4_916c, 0x100_0000),
    (0x00b4_9180, 0x2_0000),
    (0x00b4_9190, 0x4_0000),
    (0x00b4_91a4, 0x20_0000),
    (0x00b4_91bc, 0x8_0000),
];

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    /// A cdecl import through its slot.
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `strlen`.
    fn strlen(h: &impl Host, p: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(p.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// `report_rangecheckfailure()`, which ends the process.
    fn range_check_failure(h: &mut impl Host) -> u32 {
        h.call_sym(RANGE_CHECK_FAILURE, Conv::Cdecl, &[])
    }

    /// Whether `text` holds the image string `s` (`strstr`).
    fn has(h: &mut impl Host, text: u32, s: u32) -> bool {
        let s = h.rebase(s);
        import(h, STRSTR, &[text, s]) != 0
    }

    /// The shader's flags from its option text: the first of
    /// [`EXCLUSIVE`] alone; else `VSP_ZMAT`'s 1, the first of [`BLENDS`]
    /// unless `VSP_ALPHA_MODULATE` comes first, and every one of
    /// [`OPTIONS`].
    fn shader_flags(h: &mut impl Host, options: u32) -> u32 {
        for (s, flag) in EXCLUSIVE {
            if has(h, options, s) {
                return flag;
            }
        }
        let mut flags = u32::from(has(h, options, ZMAT));
        if !has(h, options, ALPHA_MODULATE) {
            for (s, flag) in BLENDS {
                if has(h, options, s) {
                    flags |= flag;
                    break;
                }
            }
        }
        for (s, flag) in OPTIONS {
            if has(h, options, s) {
                flags |= flag;
            }
        }
        flags
    }

    /// `VfxShaderManager_loadShader(this; dir, entry)` (thiscall,
    /// `ret 8`; the frame from `ebp - 0x5c0`) -> 0. `entry` is
    /// `name:options`: the name (under 0x80 bytes) goes to `ebp - 0x490`,
    /// the options (under 0x400) to `ebp - 0x410`, longer ones end the
    /// process (`report_rangecheckfailure`). The options give the flags
    /// (`ebp - 0x594`, see [`shader_flags`]); the file
    /// `dir + "D3D11/" + name` (`ebp - 0x590`) is opened with a
    /// `PStreamReaderFile` at `ebp - 0x5c0`. Found: closed, and the async
    /// loader loads it as a cluster with [`ON_SHADER_LOADED`], kind 2,
    /// `this` (as a signed 64-bit word pair) and the flags. Missing: a
    /// message, and the flags go into `this`'s tree with the value 0
    /// (`ebp - 0x598`, the `dir` slot before). The frame also keeps
    /// `entry` (`ebp - 0x5a4`), `this` (`ebp - 0x5a0`) and the colon
    /// (`ebp - 0x59c`).
    pub fn load_shader(h: &mut impl Host, ebp: u32, this: u32, dir: u32, entry: u32) -> u32 {
        let path = ebp - 0x590;
        let name = ebp - 0x490;
        let options = ebp - 0x410;
        let reader = ebp - 0x5c0;
        h.write(ebp - 0x5a0, this);
        h.write(ebp - 0x598, dir);
        h.write(ebp - 0x5a4, entry);
        h.write(ebp - 0x594, 0u32);
        let colon_text = h.rebase(COLON);
        let colon = import(h, STRSTR, &[entry, colon_text]);
        h.write(ebp - 0x59c, colon);
        if colon == 0 {
            // junk read: [ebp-0x490] and [ebp-0x410] never written; 0
            // (brief 2026-10-24): the name and the options are empty
            h.write(name, 0u8);
            h.write(options, 0u8);
        } else {
            let n = colon.wrapping_sub(entry);
            import(h, STRNCPY, &[name, entry, n]);
            if n >= 0x80 {
                return range_check_failure(h);
            }
            h.write(name + n, 0u8);
            let rest = colon.wrapping_add(1);
            let len = strlen(h, rest);
            import(h, STRNCPY, &[options, rest, len]);
            let len = strlen(h, rest);
            if len >= 0x400 {
                return range_check_failure(h);
            }
            h.write(options + len, 0u8);
        }
        let flags = shader_flags(h, options);
        h.write(ebp - 0x594, flags);
        let format = h.rebase(FORMAT_SSS);
        let folder = h.rebase(D3D11_FOLDER);
        import(h, SPRINTF, &[path, format, dir, folder, name]);
        h.call_sym(READER_CTOR, Conv::Thiscall(reader), &[path]);
        if h.call_sym(READER_IS_OPEN, Conv::Thiscall(reader), &[]) as u8 != 0 {
            h.call_sym(READER_CLOSE, Conv::Thiscall(reader), &[]);
            let manager = h.call_sym(ASYNC_LOAD_INSTANCE, Conv::Cdecl, &[]);
            let callback = h.rebase(ON_SHADER_LOADED);
            let high = if this & 0x8000_0000 == 0 { 0 } else { u32::MAX };
            h.call_sym(
                LOAD_CLUSTER,
                Conv::Cdecl,
                &[manager, 0, path, callback, 2, this, high, flags, 0],
            );
        } else {
            let message = h.rebase(NO_SHADER);
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[message, path]);
            h.write(ebp - 0x598, 0u32);
            h.call_sym(
                TREE_INSERT_NEW,
                Conv::Thiscall(this),
                &[ebp - 0x594, ebp - 0x598],
            );
        }
        h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
        0
    }

    /// `VfxShaderManager_loadVfxShader(this)` (thiscall, plain `ret`;
    /// the frame from `ebp - 0x31a8`): the collection `vfxShader_fx.txt`
    /// in the unified shader folder (`ebp - 0x190`, the path at
    /// `ebp - 0x90`) read in chunks of up to 0x3ff bytes (`ebp - 0x990`)
    /// and appended to the text at `ebp - 0x3190` (0x2800 bytes, cleared
    /// first; `sprintf("%s%s")` onto itself). Each line (`ebp - 0x590`,
    /// under 0x400 bytes, else `report_rangecheckfailure`) with `.phyre`
    /// and without `//` is loaded with [`load_shader`]; a non-zero result
    /// (`ebp - 0x31a8`; the original always returns 0) is pushed into a
    /// `PArray` at `ebp - 0x31a0` (count and flags, data; the count read
    /// back at `ebp - 0x3198`) whose clusters are fixed up at the end and
    /// which is freed unless it does not own its data. The last-line byte
    /// is at `ebp - 0x3191`, `this` at `ebp - 0x31a4`. `eax`: `dbgPrintf`'s
    /// without the collection, `PhyreFree`'s, or `0xfe` (the one caller
    /// drops it).
    pub fn load_vfx_shader(h: &mut impl Host, ebp: u32, this: u32) -> u32 {
        let dir = ebp - 0x190;
        let file = ebp - 0x90;
        let text = ebp - 0x3190;
        let chunk = ebp - 0x990;
        let line = ebp - 0x590;
        let array = ebp - 0x31a0;
        h.write(ebp - 0x31a4, this);
        let folder = h.rebase(SHADERS_FOLDER);
        h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[folder, dir, 0x100]);
        let format = h.rebase(FORMAT_SS);
        let name = h.rebase(COLLECTION);
        import(h, SPRINTF, &[file, format, dir, name]);
        let stream = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[file, 1]);
        if stream == 0 {
            let message = h.rebase(NO_COLLECTION);
            return h.call_sym(DBG_PRINTF, Conv::Cdecl, &[message]);
        }
        h.write(text, 0u8);
        h.call_sym(MEMSET, Conv::Cdecl, &[text + 1, 0, 0x27ff]);
        loop {
            let n = h.call_sym(FIOS_READ, Conv::Cdecl, &[stream, chunk, 0x3ff]);
            if n as i32 <= 0 {
                break;
            }
            if n >= 0x400 {
                return range_check_failure(h);
            }
            h.write(chunk + n, 0u8);
            import(h, SPRINTF, &[text, format, text, chunk]);
        }
        h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[stream]);
        h.write(ebp - 0x3191, 0u8);
        h.write(ebp - 0x3198, 0u32);
        h.write(array, 0u32);
        h.write(array + 4, 0u32);
        let mut cursor = text;
        loop {
            let newline = import(h, STRCHR, &[cursor, 10]);
            let last = if newline == 0 {
                let n = strlen(h, cursor);
                import(h, STRNCPY, &[line, cursor, n]);
                let n = strlen(h, cursor);
                if n >= 0x400 {
                    return range_check_failure(h);
                }
                h.write(line + n, 0u8);
                h.write(ebp - 0x3191, 1u8);
                true
            } else {
                let n = newline.wrapping_sub(cursor);
                import(h, STRNCPY, &[line, cursor, n]);
                if n >= 0x400 {
                    return range_check_failure(h);
                }
                h.write(line + n, 0u8);
                cursor = newline.wrapping_add(1);
                h.read::<u8>(ebp - 0x3191) != 0
            };
            if !has(h, line, COMMENT) && has(h, line, PHYRE_EXT) {
                let shader = h.call_sym(LOAD_SHADER, Conv::Thiscall(this), &[dir, line]);
                h.write(ebp - 0x31a8, shader);
                if shader != 0 {
                    let count: u32 = h.read(ebp - 0x3198);
                    let size = (count & 0x7fff_ffff).wrapping_mul(4).wrapping_add(4);
                    let buffer = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[size, 4]);
                    h.call_sym(PARRAY_PUSH, Conv::Thiscall(array), &[buffer, ebp - 0x31a8]);
                    let count: u32 = h.read(array);
                    h.write(ebp - 0x3198, count);
                }
            }
            if last {
                break;
            }
        }
        let data: u32 = h.read(array + 4);
        let count: u32 = h.read(ebp - 0x3198);
        if count & 0x7fff_ffff != 0 {
            h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
            h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[data, count & 0x7fff_ffff]);
        }
        if count & 0x8000_0000 == 0 {
            return h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
        0xfe
    }
}
