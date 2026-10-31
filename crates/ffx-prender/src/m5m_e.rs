//! M5 stage 1, list `map`, round 5 batch 1: small pieces of the debug map
//! tool (`rcui*`, `rcbg*`): the pre-memory window and its procedure, the
//! particle kick, the full path, the text-load and save helpers, and the
//! `abmap_cheng_panel` thunk.
//!
//! None of it ran in a coverage run. Callees stay calls to the originals,
//! cdecl as their `ret` says. Stack cookies are left out; where the
//! original hands a callee a frame buffer, the bridge takes its address
//! (the shim passes a `StackFrame`), and argument slots the original
//! writes come in as `slots`.

use ffx_bridge::Global;

/// `g_RcuiWork`, `g_YnMapSystem`.
pub const RCUI_WORK: Global<u32> = Global::new(0x0193_c0f0);
pub const YN_MAP_SYSTEM: Global<u32> = Global::new(0x0193_bf80);
/// `g_rcefDebugMain`: an absolute address (`rcuiParProcKick`'s load has
/// no relocation entry, as `rcefMainInit`'s store), not rebased.
pub const RCEF_DEBUG_MAIN: u32 = 0x01b0_003c;

/// `rcuiPreMemWinOpen`'s title and `rcuiPreMemProc` (the window
/// procedure it installs).
pub const PRE_MEM_TITLE: u32 = 0x00b6_87ec;
pub const PRE_MEM_PROC: u32 = 0x0093_63f0;
/// `rcuiPreMemProc`'s texts and its scale factors (f64: 1/65536, 100).
pub const PRE_MEM_LABEL: u32 = 0x00b6_87e8;
pub const PRE_MEM_KB_FORMAT: u32 = 0x00b6_87e0;
pub const PRE_MEM_SCALE: u32 = 0x00b6_8830;
pub const PERCENT_SCALE: u32 = 0x00b9_22e0;
/// `rcuiMakeFullPath`'s format.
pub const FULL_PATH_FORMAT: u32 = 0x00b6_88f4;
/// `rcbgPreLoadText`'s allocation tag.
pub const PRE_LOAD_TAG: u32 = 0x00b6_8be8;
/// The FP texture loads' count pattern and number format.
pub const TRX_COUNT_TEXT: u32 = 0x00b6_8df4;
pub const TRX_NUMBER_FORMAT: u32 = 0x00b6_8df8;
/// `rcbgFpLoadTextSub`'s two path formats (with and without the flag).
pub const FP_PATH_FORMAT: u32 = 0x00b6_8d9c;
pub const FP_PATH_FORMAT_PLAIN: u32 = 0x00b6_8dc8;
/// The two model extensions `0x0093ad60` knows.
pub const MODEL_EXTENSIONS: [u32; 2] = [0x00b6_8d94, 0x00b6_8d98];
/// The window callback `0x0093c1d0` creates its window with.
pub const TREE_WINDOW_PROC: u32 = 0x0093_c200;
/// `MSVCR110` slots: `sprintf`, `strrchr` (cdecl).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
pub const STRRCHR_SLOT: u32 = 0x00b0_c3a4;

// ---- callees ----

/// `rcuiWindowCreate(holder, x, y, w, h, flags, proc, title, parent)`.
pub const RCUI_WINDOW_CREATE: u32 = 0x0093_7ef0;
/// `rcbgSceneLoadTexture(scene)`, `rcbgSceneLoadTextSub(scene)`,
/// `rcbgSceneChkLayers(scene)`.
pub const SCENE_LOAD_TEXTURE: u32 = 0x0093_b6b0;
pub const SCENE_LOAD_TEXT_SUB: u32 = 0x0093_b3e0;
pub const SCENE_CHK_LAYERS: u32 = 0x0092_b400;
/// `abmap_cheng_panel(kind, node)`.
pub const ABMAP_CHENG_PANEL: u32 = 0x00a4_8740;
/// `Gu_WindowCreate(x, y, w, h, flags, proc)`, `Gu_MouseSetWindow(w)`,
/// `Gu_WindowCloseTree(w)`.
pub const GU_WINDOW_CREATE: u32 = 0x0084_a920;
pub const GU_MOUSE_SET_WINDOW: u32 = 0x0084_9b90;
pub const GU_WINDOW_CLOSE_TREE: u32 = 0x0084_ab80;
/// `rcuiGetMapNo(name)`, `rcuiGetStageName2(id, out, flag)`.
pub const RCUI_GET_MAP_NO: u32 = 0x0093_77e0;
pub const RCUI_GET_STAGE_NAME2: u32 = 0x0093_7960;
/// `rcbgDelObjects(kind)`, `rcbgFpTrxTexLoadTextGetTrxCnt(path, text)`,
/// `rcbgFpTrxTexNew()`, `rcbgTrxTexLoadFromFile(t, path, name, kind)`,
/// `rcbgPreAddObject(obj)`.
pub const DEL_OBJECTS: u32 = 0x0092_19e0;
pub const TRX_COUNT: u32 = 0x0093_a910;
pub const TRX_NEW: u32 = 0x0092_20c0;
pub const TRX_LOAD: u32 = 0x0093_beb0;
pub const PRE_ADD_OBJECT: u32 = 0x0092_18c0;
/// `0x0093a890(path)`: the FP texture loads (this batch).
pub const FP_TRX_LOAD_ALL: u32 = 0x0093_a890;
/// `rcbgPreInit()`, `rcAlloc(n, tag, line)`, `rcbgSceneInit(scene)`.
pub const PRE_INIT: u32 = 0x0092_1770;
pub const RC_ALLOC: u32 = 0x0092_58a0;
pub const SCENE_INIT: u32 = 0x0092_09e0;
/// `rcbgSaveChkPackList(out, count)`, `rcbgGuideMapSaveSub(n, names,
/// count)`, `rcbgSaveBinNormal()`.
pub const SAVE_CHK_PACK_LIST: u32 = 0x0093_9480;
pub const GUIDE_MAP_SAVE_SUB: u32 = 0x0093_91d0;
pub const SAVE_BIN_NORMAL: u32 = 0x0093_9040;
/// `sceWrite(fd, buf, n)`.
pub const SCE_WRITE: u32 = 0x0062_fdc0;
/// `rcbgModelCreateFromFile(a, name)`, `_stricmp` (the CRT thunk).
pub const MODEL_CREATE_FROM_FILE: u32 = 0x0093_a280;
pub const STRICMP: u32 = 0x0094_9664;
/// `rcPad16(n)`, `rcbgChunkTag(i)`, `crt_memset_thunk(p, v, n)`.
pub const RC_PAD16: u32 = 0x0092_b2c0;
pub const CHUNK_TAG: u32 = 0x0092_1b90;
pub const MEMSET: u32 = 0x0094_964c;
/// `rcMeshPastIndices(obj)`, `get_ptr32_func(p)`.
pub const MESH_PAST_INDICES: u32 = 0x0092_8270;
pub const GET_PTR32: u32 = 0x00a4_4650;
/// `Gu_PrintSetX(x)`, `Gu_PrintSetY(y)`, `Gu_Printf(fmt, ...)`,
/// `rcuiPercentPrint_00930ba0(x)`.
pub const PRINT_SET_X: u32 = 0x0084_8820;
pub const PRINT_SET_Y: u32 = 0x0084_8860;
pub const GU_PRINTF: u32 = 0x0084_8880;
pub const PERCENT_PRINT: u32 = 0x0093_0ba0;
/// `rcefGetChrHandle(chr)`, `Ch_OptposGetElement(handle, bone, 0)`,
/// `rcefMainKickPar(main, ... 10 words)`.
pub const RCEF_GET_CHR_HANDLE: u32 = 0x0092_e1c0;
pub const CH_OPTPOS_GET_ELEMENT: u32 = 0x0083_5660;
pub const RCEF_MAIN_KICK_PAR: u32 = 0x0092_53a0;
/// `Gu_MakeFileList(list, pattern)`, `MSVCR110!strncmp` (the slot).
pub const MAKE_FILE_LIST: u32 = 0x0084_98e0;
pub const STRNCMP_SLOT: u32 = 0x00b0_c3e4;
/// `Gu_PacketBeginTag(window)`, `Gu_SetPrim(prim)`, `Gu_SetRgbaq(r, g,
/// b, a)`, `Gu_SetXyzf2(x, y)`, `Gu_PacketEndTag()`.
pub const PACKET_BEGIN_TAG: u32 = 0x0084_8530;
pub const SET_PRIM: u32 = 0x0084_8480;
pub const SET_RGBAQ: u32 = 0x0084_84c0;
pub const SET_XYZF2: u32 = 0x0084_8680;
pub const PACKET_END_TAG: u32 = 0x0084_8460;

pub mod bridge {
    use super::{
        ABMAP_CHENG_PANEL, CH_OPTPOS_GET_ELEMENT, CHUNK_TAG, DEL_OBJECTS, FP_PATH_FORMAT,
        FP_PATH_FORMAT_PLAIN, FP_TRX_LOAD_ALL, FULL_PATH_FORMAT, GET_PTR32, GU_MOUSE_SET_WINDOW,
        GU_PRINTF, GU_WINDOW_CLOSE_TREE, GU_WINDOW_CREATE, GUIDE_MAP_SAVE_SUB, MAKE_FILE_LIST,
        MEMSET, MESH_PAST_INDICES, MODEL_CREATE_FROM_FILE, MODEL_EXTENSIONS, PACKET_BEGIN_TAG,
        PACKET_END_TAG, PERCENT_PRINT, PERCENT_SCALE, PRE_ADD_OBJECT, PRE_INIT, PRE_LOAD_TAG,
        PRE_MEM_KB_FORMAT, PRE_MEM_LABEL, PRE_MEM_PROC, PRE_MEM_SCALE, PRE_MEM_TITLE, PRINT_SET_X,
        PRINT_SET_Y, RC_ALLOC, RC_PAD16, RCEF_DEBUG_MAIN, RCEF_GET_CHR_HANDLE, RCEF_MAIN_KICK_PAR,
        RCUI_GET_MAP_NO, RCUI_GET_STAGE_NAME2, RCUI_WINDOW_CREATE, RCUI_WORK, SAVE_BIN_NORMAL,
        SAVE_CHK_PACK_LIST, SCE_WRITE, SCENE_CHK_LAYERS, SCENE_INIT, SCENE_LOAD_TEXT_SUB,
        SCENE_LOAD_TEXTURE, SET_PRIM, SET_RGBAQ, SET_XYZF2, SPRINTF_SLOT, STRICMP, STRNCMP_SLOT,
        STRRCHR_SLOT, TREE_WINDOW_PROC, TRX_COUNT, TRX_COUNT_TEXT, TRX_LOAD, TRX_NEW,
        TRX_NUMBER_FORMAT, YN_MAP_SYSTEM,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// Calls the CRT import whose slot is `slot` (cdecl).
    fn import(h: &mut impl Host, slot: u32, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, Conv::Cdecl, args)
    }

    /// `rcuiPreMemWinOpen()` -> `rcuiWindowCreate`'s `eax`: the
    /// pre-memory window (holder `g_RcuiWork + 0x50`, at 8, 8, 90 x 30,
    /// flags 6, `rcuiPreMemProc`).
    pub fn pre_mem_win_open(h: &mut impl Host) -> u32 {
        let holder = RCUI_WORK.get(h) + 0x50;
        let (proc_, title) = (h.rebase(PRE_MEM_PROC), h.rebase(PRE_MEM_TITLE));
        cdecl(
            h,
            RCUI_WINDOW_CREATE,
            &[holder, 8, 8, 0x5a, 0x1e, 6, proc_, title, 0],
        )
    }

    /// `rcuiPreMemProc(window, msg, param)` -> `eax` as the original
    /// leaves it: with a map (`g_YnMapSystem`, its `+0`), message 0x8003
    /// (close) clears `g_RcuiWork + 0x50`; 0x8011 (draw) prints the label
    /// at (0, 0), the map's used memory (`+0x10`) as a percentage of
    /// 64 KiB at x 0x12 (`rcuiPercentPrint_00930ba0`) and in KiB at x 0x2d.
    pub fn pre_mem_proc(h: &mut impl Host, msg: u32) -> u32 {
        let sys = YN_MAP_SYSTEM.get(h);
        if sys == 0 {
            return 0;
        }
        let map: u32 = h.read(sys);
        if map == 0 {
            return sys;
        }
        match msg {
            0x8003 => {
                let w = RCUI_WORK.get(h);
                h.write(w + 0x50, 0_u32);
                w
            }
            0x8011 => {
                let used: u32 = h.read(map + 0x10);
                cdecl(h, PRINT_SET_X, &[0]);
                cdecl(h, PRINT_SET_Y, &[0]);
                let label = h.rebase(PRE_MEM_LABEL);
                cdecl(h, GU_PRINTF, &[label]);
                cdecl(h, PRINT_SET_X, &[0x12]);
                let x = x87::load(x87::store(f64::from(used.cast_signed())));
                let scale: f64 = f64::from_bits(h.read(h.rebase(PRE_MEM_SCALE)));
                let percent: f64 = f64::from_bits(h.read(h.rebase(PERCENT_SCALE)));
                let v = x87::store(x87::mul(x87::mul(x, scale), percent));
                cdecl(h, PERCENT_PRINT, &[v]);
                cdecl(h, PRINT_SET_X, &[0x2d]);
                let kb = (used.cast_signed() / 1024).cast_unsigned();
                let fmt = h.rebase(PRE_MEM_KB_FORMAT);
                cdecl(h, GU_PRINTF, &[fmt, kb])
            }
            _ => msg.wrapping_sub(0x8011),
        }
    }

    /// `rcbgSceneLoadAll_0093aec0(scene)` -> `rcbgSceneChkLayers`'s `eax`:
    /// the scene's textures, its text data, then its layer check.
    pub fn scene_load_all(h: &mut impl Host, scene: u32) -> u32 {
        cdecl(h, SCENE_LOAD_TEXTURE, &[scene]);
        cdecl(h, SCENE_LOAD_TEXT_SUB, &[scene]);
        cdecl(h, SCENE_CHK_LAYERS, &[scene])
    }

    /// `abmap_cheng_panel_thunk_00a45860(kind, node)`: `jmp
    /// abmap_cheng_panel`, its `eax`.
    pub fn abmap_cheng_panel_thunk(h: &mut impl Host, kind: u32, node: u32) -> u32 {
        cdecl(h, ABMAP_CHENG_PANEL, &[kind, node])
    }

    /// `rcuiTreeWindowOpen_0093c1d0(x, y)` -> `Gu_WindowCloseTree`'s
    /// `eax`: a window at `(x, y)` with the procedure `0x0093c200`, given
    /// the mouse and closed with its tree again.
    pub fn tree_window_open(h: &mut impl Host, x: u32, y: u32) -> u32 {
        let proc_ = h.rebase(TREE_WINDOW_PROC);
        let w = cdecl(h, GU_WINDOW_CREATE, &[x, y, 0, 0, 0, proc_]);
        cdecl(h, GU_MOUSE_SET_WINDOW, &[w]);
        cdecl(h, GU_WINDOW_CLOSE_TREE, &[w])
    }

    /// `rcuiMakeFullPath()` -> the map number: `g_RcuiWork + 0x174` =
    /// the folder (`+0x74`), the area (`+0x274`) and the map (`+0x27c`)
    /// joined, `+0x284` = `rcuiGetMapNo(+0x27c)`.
    pub fn make_full_path(h: &mut impl Host) -> u32 {
        let w = RCUI_WORK.get(h);
        let fmt = h.rebase(FULL_PATH_FORMAT);
        import(
            h,
            SPRINTF_SLOT,
            &[w + 0x174, fmt, w + 0x74, w + 0x274, w + 0x27c],
        );
        let w = RCUI_WORK.get(h);
        let no = cdecl(h, RCUI_GET_MAP_NO, &[w + 0x27c]);
        let w = RCUI_WORK.get(h);
        h.write(w + 0x284, no);
        no
    }

    /// `rcbgFpTrxTexLoadAll_0093a890(path)` -> the count when 0 or less,
    /// else the last `rcbgPreAddObject`'s `eax`: the FP textures (kind
    /// 0x20006) deleted, then each of the counted ones loaded into a new
    /// texture object (its number formatted into `buf`, the original's
    /// `ebp - 0x14`) and added.
    pub fn fp_trx_load_all(h: &mut impl Host, buf: u32, path: u32) -> u32 {
        cdecl(h, DEL_OBJECTS, &[0x2_0006]);
        let text = h.rebase(TRX_COUNT_TEXT);
        let n = cdecl(h, TRX_COUNT, &[path, text]);
        let mut r = n;
        let mut i = 0_u32;
        while i.cast_signed() < n.cast_signed() {
            let t = cdecl(h, TRX_NEW, &[]);
            let fmt = h.rebase(TRX_NUMBER_FORMAT);
            import(h, SPRINTF_SLOT, &[buf, fmt, i]);
            cdecl(h, TRX_LOAD, &[t, path, buf, 2]);
            r = cdecl(h, PRE_ADD_OBJECT, &[t]);
            i += 1;
        }
        r
    }

    /// `rcbgPreLoadText()` -> `rcbgSceneChkLayers`'s `eax`: the pre-scene
    /// reset, then the map (`[g_YnMapSystem]`, none: a null map) gets `+0`
    /// = 1 and a fresh 0x110-byte scene at `+4` and `+8`, initialised and
    /// loaded.
    pub fn pre_load_text(h: &mut impl Host) -> u32 {
        cdecl(h, PRE_INIT, &[]);
        let sys = YN_MAP_SYSTEM.get(h);
        let map = if sys == 0 { 0 } else { h.read(sys) };
        h.write(map, 1_u32);
        let tag = h.rebase(PRE_LOAD_TAG);
        let scene = cdecl(h, RC_ALLOC, &[0x110, tag, 0x69]);
        h.write(map + 4, scene);
        cdecl(h, SCENE_INIT, &[scene]);
        let scene: u32 = h.read(map + 4);
        h.write(map + 8, scene);
        cdecl(h, SCENE_LOAD_TEXTURE, &[scene]);
        cdecl(h, SCENE_LOAD_TEXT_SUB, &[scene]);
        cdecl(h, SCENE_CHK_LAYERS, &[scene])
    }

    /// `rcbgSaveBin_00938fd0()` -> the save's `eax`: the pack list (names
    /// into `frame + 4`, its count at `frame`, the original's `ebp -
    /// 0x108`); with packs, the guide-map save of them, else the plain
    /// binary save.
    pub fn save_bin(h: &mut impl Host, frame: u32) -> u32 {
        let n = cdecl(h, SAVE_CHK_PACK_LIST, &[frame + 4, frame]);
        if n.cast_signed() > 0 {
            let count: u32 = h.read(frame);
            cdecl(h, GUIDE_MAP_SAVE_SUB, &[n, frame + 4, count])
        } else {
            cdecl(h, SAVE_BIN_NORMAL, &[])
        }
    }

    /// `rcbgTrxSave(t, fd)` -> the last `sceWrite`'s `eax`: three 8-byte
    /// fields (`+0x18..+0x30`), the two 4-byte sizes (`+0x30`, `+0x34`)
    /// and their product's bytes from `[t + 0x38]`.
    pub fn trx_save(h: &mut impl Host, t: u32, fd: u32) -> u32 {
        for (off, n) in [(0x18, 8), (0x20, 8), (0x28, 8), (0x30, 4), (0x34, 4)] {
            cdecl(h, SCE_WRITE, &[fd, t + off, n]);
        }
        let size = h.read::<u32>(t + 0x30).wrapping_mul(h.read(t + 0x34));
        let data: u32 = h.read(t + 0x38);
        cdecl(h, SCE_WRITE, &[fd, data, size])
    }

    /// `rcbgFpLoadTextSub(id, flag)` -> 0, or the FP texture load's `eax`:
    /// with an id whose stage name (4 characters into `frame`, the
    /// original's `ebp - 0xc`) is known, the path (formatted into `path`,
    /// `ebp - 0x10c`; the flag picks the format) has its textures loaded.
    pub fn fp_load_text_sub(h: &mut impl Host, frame: u32, path: u32, id: u32, flag: u32) -> u32 {
        if id == 0 {
            return 0;
        }
        if cdecl(h, RCUI_GET_STAGE_NAME2, &[id, frame, flag]) == 0 {
            return 0;
        }
        let c: [u32; 4] =
            core::array::from_fn(|k| i32::from(h.read::<i8>(frame + k as u32)).cast_unsigned());
        let fmt = h.rebase(if flag != 0 {
            FP_PATH_FORMAT
        } else {
            FP_PATH_FORMAT_PLAIN
        });
        import(h, SPRINTF_SLOT, &[path, fmt, c[0], c[1], c[2], c[3], frame]);
        cdecl(h, FP_TRX_LOAD_ALL, &[path])
    }

    /// `rcbgModelLoadByExt_0093ad60(a, name)` -> the model, or 0: only a
    /// name whose extension (after the last '.') is the second known one
    /// (`_stricmp`) goes to `rcbgModelCreateFromFile(a, name)`.
    pub fn model_load_by_ext(h: &mut impl Host, a: u32, name: u32) -> u32 {
        let dot = import(h, STRRCHR_SLOT, &[name, 0x2e]);
        if dot == 0 {
            return 0;
        }
        let ext = dot + 1;
        let mut found = None;
        for (k, &e) in MODEL_EXTENSIONS.iter().enumerate() {
            let e = h.rebase(e);
            if cdecl(h, STRICMP, &[ext, e]) == 0 {
                found = Some(k);
                break;
            }
        }
        if found == Some(1) {
            cdecl(h, MODEL_CREATE_FROM_FILE, &[a, name])
        } else {
            0
        }
    }

    /// `rcbgSaveChunkHeader_00938f40(size, fd)` -> the last `sceWrite`'s
    /// `eax` (header at `frame`, the original's `ebp - 0x14`; `slots` its
    /// argument slots): a 16-byte chunk header (the tag `rcbgChunkTag(1)`,
    /// the 16-byte units of the padding + 4, zeros), the size word from
    /// its argument slot, and `rcPad16(4)` zero bytes.
    pub fn save_chunk_header(h: &mut impl Host, slots: u32, frame: u32, fd: u32) -> u32 {
        let pad = cdecl(h, RC_PAD16, &[4]);
        for k in 0..4 {
            h.write(frame + 4 * k, 0_u32);
        }
        let tag = cdecl(h, CHUNK_TAG, &[1]);
        let tag: u32 = h.read(tag);
        h.write(frame, tag);
        let n = pad.wrapping_add(4).cast_signed();
        h.write(frame + 4, (n + ((n >> 31) & 0xf)) >> 4);
        cdecl(h, SCE_WRITE, &[fd, frame, 0x10]);
        cdecl(h, SCE_WRITE, &[fd, slots, 4]);
        cdecl(h, MEMSET, &[frame, 0, pad]);
        cdecl(h, SCE_WRITE, &[fd, frame, pad])
    }

    /// `rcbgSaveMesh_00938e70(obj, fd)` -> the last `sceWrite`'s `eax`:
    /// `+0x10`, the index data's end (`rcMeshPastIndices`, kept in the
    /// first argument slot `slots` and written from there), that many
    /// bytes of `get_ptr32_func(+0x14)`, then `+0x20` (0x70), `+0x90`
    /// (0x20), `+0xb0` (0x10), `+0xc0` and `+0x100` (0x40 each).
    pub fn save_mesh(h: &mut impl Host, slots: u32, obj: u32, fd: u32) -> u32 {
        cdecl(h, SCE_WRITE, &[fd, obj + 0x10, 4]);
        let end = cdecl(h, MESH_PAST_INDICES, &[obj]);
        h.write(slots, end);
        cdecl(h, SCE_WRITE, &[fd, slots, 4]);
        let p: u32 = h.read(obj + 0x14);
        let data = cdecl(h, GET_PTR32, &[p]);
        let n: u32 = h.read(slots);
        cdecl(h, SCE_WRITE, &[fd, data, n]);
        let mut r = 0;
        for (off, n) in [
            (0x20, 0x70),
            (0x90, 0x20),
            (0xb0, 0x10),
            (0xc0, 0x40),
            (0x100, 0x40),
        ] {
            r = cdecl(h, SCE_WRITE, &[fd, obj + off, n]);
        }
        r
    }

    /// `rcuiParProcKick(force)`: with the debug effect (`g_rcefDebugMain`),
    /// `rcefMainKickPar` with the work's selected particle (`+0x830`
    /// indexed by `+0x8b0`), its bone (`+0x8c4`, or the chosen character's
    /// element `Ch_OptposGetElement(rcefGetChrHandle(p), +0x8d0, 0)` with
    /// `+0x8b4`), the three vectors (`+0x924`, `+0x934`, `+0x944`) and the
    /// flag bit `1 << (+0x920 + 3)` (0 for none).
    pub fn par_proc_kick(h: &mut impl Host, force: u32) {
        let main: u32 = h.read(RCEF_DEBUG_MAIN);
        if main == 0 {
            return;
        }
        let mut w = RCUI_WORK.get(h);
        let sel: u32 = h.read(w + 0x8b0);
        let par: u32 = h.read(w.wrapping_add(sel.wrapping_mul(4)).wrapping_add(0x830));
        let bit: u32 = h.read(w + 0x920);
        let flags = if bit == 0 {
            0
        } else {
            1_u32 << (bit.wrapping_add(3) & 31)
        };
        let bone = if h.read::<u32>(w + 0x8b4) == 0 {
            h.read(w + 0x8c4)
        } else {
            let el: u32 = h.read(w + 0x8d0);
            let handle = cdecl(h, RCEF_GET_CHR_HANDLE, &[par]);
            let r = cdecl(h, CH_OPTPOS_GET_ELEMENT, &[handle, el, 0]);
            w = RCUI_WORK.get(h);
            r
        };
        let (a, b): (u32, u32) = (h.read(w + 0x914), h.read(w + 0x918));
        cdecl(
            h,
            RCEF_MAIN_KICK_PAR,
            &[
                main,
                a,
                b,
                par,
                bone,
                w + 0x924,
                w + 0x934,
                w + 0x944,
                flags,
                force,
            ],
        );
    }

    /// `rcbgCountNumberedFiles_0093acb0(pattern, prefix, digits)` -> how
    /// many names of the file list (`Gu_MakeFileList` into `list`, the
    /// original's 3-word `ebp - 0x10`) start with `prefix` followed by
    /// `digits` decimal digits.
    pub fn count_numbered_files(
        h: &mut impl Host,
        list: u32,
        pattern: u32,
        prefix: u32,
        digits: u32,
    ) -> u32 {
        for k in 0..3 {
            h.write(list + 4 * k, 0_u32);
        }
        cdecl(h, MAKE_FILE_LIST, &[list, pattern]);
        let mut len = 0_u32;
        while h.read::<u8>(prefix + len) != 0 {
            len += 1;
        }
        let mut count = 0_u32;
        let mut i = 0_u32;
        while i.cast_signed() < h.read::<u32>(list).cast_signed() {
            let names: u32 = h.read(list + 8);
            let name: u32 = h.read(names.wrapping_add(4 * i));
            if import(h, STRNCMP_SLOT, &[name, prefix, len]) == 0 {
                let mut k = 0_u32;
                if digits.cast_signed() > 0 {
                    let names: u32 = h.read(list + 8);
                    let at = h.read::<u32>(names.wrapping_add(4 * i)).wrapping_add(len);
                    while k.cast_signed() < digits.cast_signed() {
                        let c: i8 = h.read(at.wrapping_add(k));
                        if !(0x30..=0x39).contains(&c) {
                            break;
                        }
                        k += 1;
                    }
                }
                if k == digits {
                    count += 1;
                }
            }
            i += 1;
        }
        count
    }

    /// `rcuiGaugeDraw_0093c730(window, x, y, value)` -> `Gu_PacketEndTag`'s
    /// `eax` (a tail jump): a line packet in the window: the frame (x + 8,
    /// y .. y + 100), ticks every 10 (long ones at 0, 50 and 100), and,
    /// for a value up to 100, a red line at `y + 100 - value`.
    pub fn gauge_draw(h: &mut impl Host, window: u32, x: u32, y: u32, value: u32) -> u32 {
        cdecl(h, PACKET_BEGIN_TAG, &[window]);
        cdecl(h, SET_PRIM, &[1]);
        cdecl(h, SET_RGBAQ, &[0, 0, 0, 0]);
        cdecl(h, SET_XYZF2, &[x.wrapping_add(8), y]);
        cdecl(h, SET_XYZF2, &[x.wrapping_add(8), y.wrapping_add(0x64)]);
        let mut at = y;
        for tick in (0..=100).step_by(10) {
            let end = if tick % 50 == 0 {
                cdecl(h, SET_XYZF2, &[x, at]);
                x.wrapping_add(0x10)
            } else {
                cdecl(h, SET_XYZF2, &[x.wrapping_add(4), at]);
                x.wrapping_add(0xc)
            };
            cdecl(h, SET_XYZF2, &[end, at]);
            at = at.wrapping_add(10);
        }
        if value.cast_signed() <= 100 {
            cdecl(h, SET_RGBAQ, &[0xff, 0, 0, 0]);
            let at = y.wrapping_sub(value).wrapping_add(0x64);
            cdecl(h, SET_XYZF2, &[x, at]);
            cdecl(h, SET_XYZF2, &[x.wrapping_add(0x10), at]);
        }
        cdecl(h, PACKET_END_TAG, &[window, x, y, value])
    }
}
