//! Continuation round 2 of list `h4-glue-other`, batch 3: the menu
//! texture manager's font, Sa-menu, monster-book and language loaders.
//!
//! Bridges that hand callees a local take `frame`, the block laid out as
//! the original's frame from its lowest local up.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

/// `crt_memset_thunk(p, c, n)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `MenuTextureManager_MenuUnloadTexture(this; name)` (thiscall, ret 4).
pub const MENU_UNLOAD_TEXTURE: u32 = 0x0067_dbf0;
/// `PhyreScene_loadMenuTexture(scene; name)` (thiscall, ret 4).
pub const LOAD_MENU_TEXTURE: u32 = 0x0066_51d0;
/// `g_PhyreScene`.
pub const PHYRE_SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// The four font-texture suffixes (pointers to C strings).
pub const FONT_SUFFIXES: [u32; 4] = [0x00c5_9bc0, 0x00c5_9bc4, 0x00c5_9c00, 0x00c5_9c04];

fn strlen(h: &impl Host, s: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// `strcpy(dst, src)` byte by byte, the terminator included.
fn strcpy(h: &mut impl Host, dst: u32, src: u32) {
    let mut k = 0u32;
    loop {
        let c: u8 = h.read(src.wrapping_add(k));
        h.write(dst.wrapping_add(k), c);
        if c == 0 {
            return;
        }
        k = k.wrapping_add(1);
    }
}

/// `strcat(dst, src)`: `strlen(src) + 1` bytes after `dst`'s text.
fn strcat(h: &mut impl Host, dst: u32, src: u32) {
    let n = strlen(h, src).wrapping_add(1);
    let end = dst.wrapping_add(strlen(h, dst));
    for k in 0..n {
        let c: u8 = h.read(src.wrapping_add(k));
        h.write(end.wrapping_add(k), c);
    }
}

/// Bytes of [`unload_font_texture`]'s frame: `this`, the suffix, the
/// saved name (0x100), the cookie.
pub const UNLOAD_FONT_FRAME: u32 = 0x10c;

/// `MenuTextureManager_UnloadSceneBattleFontTexture(this)` (flag `+5`,
/// name `+0x106`) and `MenuTextureManager_UnloadSceneEventFontTexture`
/// (`+4`, `+6`) (thiscall; `frame` their locals from `ebp - 0x10c`):
/// when loaded, the name (a prefix) is saved in the frame and cleared;
/// for each of the four suffixes the name becomes prefix + suffix, its
/// texture is unloaded (`MenuUnloadTexture(this; name)`) and the name
/// cleared (`memset`); the flag is cleared. Returns the last `memset`'s
/// `eax`, or `this` when not loaded.
pub fn unload_font_texture(h: &mut impl Host, frame: u32, this: u32, flag: u32, name: u32) -> u32 {
    h.write(frame, this);
    if h.read::<u8>(this + flag) == 0 {
        return this;
    }
    let buf = this + name;
    let saved = frame + 8;
    strcpy(h, saved, buf);
    h.call_sym(MEMSET, Conv::Cdecl, &[buf, 0, 0x100]);
    let mut eax = 0;
    for (k, &suffix) in FONT_SUFFIXES.iter().enumerate() {
        strcpy(h, buf, saved);
        let s = Global::<u32>::new(suffix).get(h);
        // the last suffix stays in a register
        if k < 3 {
            h.write(frame + 4, s);
        }
        strcat(h, buf, s);
        h.call_sym(MENU_UNLOAD_TEXTURE, Conv::Thiscall(this), &[buf]);
        eax = h.call_sym(MEMSET, Conv::Cdecl, &[buf, 0, 0x100]);
    }
    h.write(this + flag, 0u8);
    eax
}

/// Bytes of [`load_battle_font_texture`]'s frame: the suffix, `this`.
pub const LOAD_FONT_FRAME: u32 = 8;

/// `MenuTextureManager_LoadSceneBattleFontTexture(this; prefix)`
/// (thiscall, ret 4; `frame` its locals from `ebp - 8`): the name
/// (`+0x106`) cleared and set to `prefix`; for each of the four suffixes
/// prefix + suffix is loaded into the scene (`PhyreScene_loadMenuTexture`)
/// and the name cleared and set to `prefix` again; loaded flag `+5` set.
/// Returns the last `memset`'s `eax` with its low byte 0 (the copy's
/// terminator passes through `al`).
pub fn load_battle_font_texture(h: &mut impl Host, frame: u32, this: u32, prefix: u32) -> u32 {
    h.write(frame + 4, this);
    let buf = this + 0x106;
    let mut eax = h.call_sym(MEMSET, Conv::Cdecl, &[buf, 0, 0x100]);
    strcpy(h, buf, prefix);
    for suffix in FONT_SUFFIXES {
        let s = Global::<u32>::new(suffix).get(h);
        h.write(frame, s);
        strcat(h, buf, s);
        let scene = PHYRE_SCENE.get(h);
        h.call_sym(LOAD_MENU_TEXTURE, Conv::Thiscall(scene), &[buf]);
        eax = h.call_sym(MEMSET, Conv::Cdecl, &[buf, 0, 0x100]);
        strcpy(h, buf, prefix);
    }
    h.write(this + 5, 1u8);
    eax & !0xff
}

/// `TOGetFFXLang()` (cdecl).
pub const GET_LANG: u32 = 0x0064_10e0;
/// `TOGetFFXLangJmp()` (cdecl, a jump to [`GET_LANG`]).
pub const GET_LANG_JMP: u32 = 0x008a_c2f0;

/// The scene loads of `names` (pointers to C strings, each read as the
/// load's argument). Returns the last load's `eax`.
fn load_menu_textures(h: &mut impl Host, names: &[u32]) -> u32 {
    let mut eax = 0;
    for &g in names {
        let name = Global::<u32>::new(g).get(h);
        let scene = PHYRE_SCENE.get(h);
        eax = h.call_sym(LOAD_MENU_TEXTURE, Conv::Thiscall(scene), &[name]);
    }
    eax
}

/// The monster book's four textures by language: 1..5, 9, 10 their own,
/// 0, 6..8 and the others the default (`0x00c59dcc`).
const BOKU_TEXTURES: [u32; 8] = [
    0x00c5_9dcc,
    0x00c5_9ddc,
    0x00c5_9dec,
    0x00c5_9dfc,
    0x00c5_9e0c,
    0x00c5_9e1c,
    0x00c5_9e2c,
    0x00c5_9e3c,
];

/// `MenuTextureManager_LoadMon_BokuTextures()` (no `this` read): the
/// monster book's four textures for `TOGetFFXLang()`'s language loaded
/// into the scene (four pointers from the language's group). Returns the
/// last load's `eax`.
pub fn load_mon_boku_textures(h: &mut impl Host) -> u32 {
    let lang = h.call_sym(GET_LANG, Conv::Cdecl, &[]);
    let group = match lang {
        1..=5 => lang as usize,
        9 => 6,
        10 => 7,
        _ => 0,
    };
    let base = BOKU_TEXTURES[group];
    load_menu_textures(h, &[base, base + 4, base + 8, base + 0xc])
}

/// `MenuTextureManager_PreLoadSaSubMenu`.
pub const PRELOAD_SA_SUB_MENU: u32 = 0x0067_dd70;
/// `MenuTextureManager_LoadMon_BokuTextures`.
pub const LOAD_MON_BOKU: u32 = 0x0067_cbd0;
/// The Sa menu's cached title and caption textures (`0x025d16cc`,
/// `0x025d16c8`).
pub const SA_TITLE: u32 = 0x025d_16cc;
/// See [`SA_TITLE`].
pub const SA_CAPTION: u32 = 0x025d_16c8;
/// Their language tables (0..10; 0: the default, read back from the
/// cache).
const SA_TITLE_NAMES: [u32; 11] = [
    0x00c5_9d9c,
    0x00c5_9da8,
    0x00c5_9d94,
    0x00c5_9da4,
    0x00c5_9d90,
    0x00c5_9d98,
    0,
    0,
    0,
    0x00c5_9da0,
    0x00c5_9d88,
];
const SA_CAPTION_NAMES: [u32; 11] = [
    0x00c5_9dac,
    0x00c5_9dc8,
    0x00c5_9db8,
    0x00c5_9dc4,
    0x00c5_9db4,
    0x00c5_9dbc,
    0,
    0,
    0,
    0x00c5_9dac,
    0x00c5_9db0,
];

/// The cached texture at `cache`, chosen once by language from `names`.
fn cached_name(h: &mut impl Host, cache: u32, names: &[u32; 11]) -> u32 {
    let cache = Global::<u32>::new(cache);
    let name = cache.get(h);
    if name != 0 {
        return name;
    }
    let lang = h.call_sym(GET_LANG_JMP, Conv::Cdecl, &[]);
    match names.get(lang as usize) {
        Some(&g) if g != 0 => {
            let name = Global::<u32>::new(g).get(h);
            cache.set(h, name);
            name
        }
        _ => cache.get(h),
    }
}

/// `MenuTextureManager_LoadSaMenuTexture(this; arg)` (thiscall, ret 4):
/// `arg` kept (`+0x208`), the Sa sub-menu preloaded
/// (`PreLoadSaSubMenu(arg)`, `ecx` this), the four language textures of
/// Japanese (0), Korean (9) or Chinese (10) loaded, then the cached title
/// and caption textures (chosen by language the first time) and the
/// monster book's (`LoadMon_BokuTextures`). Returns the last's `eax`.
pub fn load_sa_menu_texture(h: &mut impl Host, this: u32, arg: u32) -> u32 {
    h.write(this + 0x208, arg);
    h.call_sym(PRELOAD_SA_SUB_MENU, Conv::Thiscall(this), &[arg]);
    let names: &[u32] = match h.call_sym(GET_LANG_JMP, Conv::Cdecl, &[]) {
        0 => &[0x00c5_9938, 0x00c5_993c, 0x00c5_9878, 0x00c5_987c],
        9 => &[0x00c5_99c0, 0x00c5_99c4, 0x00c5_98f8, 0x00c5_98fc],
        10 => &[0x00c5_9980, 0x00c5_9984, 0x00c5_98b8, 0x00c5_98bc],
        _ => &[],
    };
    load_menu_textures(h, names);
    for (cache, table) in [(SA_TITLE, &SA_TITLE_NAMES), (SA_CAPTION, &SA_CAPTION_NAMES)] {
        let name = cached_name(h, cache, table);
        let scene = PHYRE_SCENE.get(h);
        h.call_sym(LOAD_MENU_TEXTURE, Conv::Thiscall(scene), &[name]);
    }
    h.call_sym(LOAD_MON_BOKU, Conv::Thiscall(this), &[])
}

/// `ClusterManager_GetInstance()` (cdecl).
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
/// `ClusterManager_loadPClusterJump_0069bb00(manager; path)` (thiscall,
/// ret 4).
pub const LOAD_CLUSTER: u32 = 0x0069_bb00;

/// `MenuTextureManager_LoadLanguageTextures_0067c920(this; lang)`
/// (thiscall, ret 4): the language's clusters loaded (eight paths for
/// Japanese (0), Chinese (10) and Korean (9), four for the others; each
/// `loadPClusterJump(ClusterManager_GetInstance(); path)`, the path read
/// first), then `+0` cleared. Returns the last load's `eax`.
pub fn load_language_textures(h: &mut impl Host, this: u32, lang: u32) -> u32 {
    let paths: &[u32] = match lang {
        0 => &[
            0x00c5_9568,
            0x00c5_956c,
            0x00c5_95a8,
            0x00c5_95ac,
            0x00c5_9a00,
            0x00c5_9a04,
            0x00c5_9b00,
            0x00c5_9b04,
        ],
        10 => &[
            0x00c5_9668,
            0x00c5_966c,
            0x00c5_96e8,
            0x00c5_96ec,
            0x00c5_9a40,
            0x00c5_9a44,
            0x00c5_9b40,
            0x00c5_9b44,
        ],
        9 => &[
            0x00c5_9768,
            0x00c5_976c,
            0x00c5_97a8,
            0x00c5_97ac,
            0x00c5_9ac0,
            0x00c5_9ac4,
            0x00c5_9b80,
            0x00c5_9b84,
        ],
        _ => &[0x00c5_95e8, 0x00c5_95ec, 0x00c5_9628, 0x00c5_962c],
    };
    let mut eax = 0;
    for &g in paths {
        let path = Global::<u32>::new(g).get(h);
        let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
        eax = h.call_sym(LOAD_CLUSTER, Conv::Thiscall(manager), &[path]);
    }
    h.write(this, 0u32);
    eax
}

// ---- the distortion grid

/// `Distortion_buildGridIndices_006c2990(this)` (thiscall): the index
/// buffer (`+0x74`, re-read for every store) of a 9 x 9 vertex grid: for
/// each of the 8 x 8 quads, top-left vertex `t`, the triangles `(t, t +
/// 10, t + 1)` and `(t, t + 9, t + 10)`, row by row. Returns the bytes
/// written (0x600), the original's running offset.
pub fn distortion_grid_indices(h: &mut impl Host, this: u32) -> u32 {
    let mut at = 0u32;
    for row in 0..8u32 {
        for col in 0..8u32 {
            let t = 9 * row + col;
            for v in [t, t + 10, t + 1, t, t + 9, t + 10] {
                let buf: u32 = h.read(this + 0x74);
                h.write(buf.wrapping_add(at), v);
                at += 4;
            }
        }
    }
    at
}

// ---- the loading screen

/// The loading screen's twenty texture clusters (`0x00c34f70`, path
/// pointers) and its tip cluster (`0x00c34fc0`).
pub const LOADING_CLUSTERS: u32 = 0x00c3_4f70;
/// See [`LOADING_CLUSTERS`].
pub const LOADING_TIP_CLUSTER: u32 = 0x00c3_4fc0;
/// `ClusterManager_loadPCluster(manager; path)` (thiscall, ret 4).
pub const LOAD_PCLUSTER: u32 = 0x0069_b940;
/// `Phyre_PhyreMalloc_0042fb80(bytes, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `PArray_PushInto_0069b650(array; storage, &value)` (thiscall, ret 8).
pub const PUSH_CLUSTER: u32 = 0x0069_b650;
/// `PArray_PushInto_006b24e0(array; storage, &value)` (thiscall, ret 8).
pub const PUSH_TEXTURE: u32 = 0x006b_24e0;
/// `Phyre_PFramework_PApplication_GetApplication()` (cdecl).
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `PApplication_FixupClusters(clusters, n)` (cdecl).
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;
/// `Phyre_PClusterHeader_iterSeek_00464f30(cursor)` (thiscall).
pub const ITER_SEEK: u32 = 0x0046_4f30;
/// The texture class the cursors walk (`0x00ca37d0`).
pub const TEXTURE_CLASS: u32 = 0x00ca_37d0;
/// Bytes of [`loading_screen_load_textures`]'s frame: the cursor (0x20)
/// and the pushed value.
pub const LOADING_FRAME: u32 = 0x24;

/// A cluster cursor at `frame` over `cluster`'s list (`+0x1c`) of
/// textures, seeked. Returns the seek's `eax`.
fn seek_textures(h: &mut impl Host, frame: u32, cluster: u32) -> u32 {
    let list = cluster.wrapping_add(0x1c);
    let first: u32 = h.read(list);
    let head = if first == list { 0 } else { first };
    for k in 0..5u32 {
        h.write(frame + 4 * k, 0u32);
    }
    h.write(frame + 0x14, list);
    h.write(frame + 0x18, head);
    h.write(frame + 0x1c, h.rebase(TEXTURE_CLASS));
    h.call_sym(ITER_SEEK, Conv::Thiscall(frame), &[])
}

/// `LoadingScreen_loadTextures(this)` (thiscall; `frame` its locals from
/// `ebp - 0x24`: a cluster cursor, the value word `+0x20`): the twenty
/// texture clusters loaded (`ClusterManager_loadPCluster`) and the loaded
/// ones appended to `+0x18` (`PArray_PushInto_0069b650` into
/// `PhyreMalloc((n + 1) * 4, 4)`) and fixed up; the tip cluster loaded
/// into `+0x28` and fixed up; each cluster's first texture (cursor seek)
/// appended to `+0x20` (`PArray_PushInto_006b24e0`); the tip's first
/// texture in `+0x2c` (0 without one). Returns what the original leaves
/// in `eax`: 0 without a tip, else the tip texture or the seek's result.
pub fn loading_screen_load_textures(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let value = frame + 0x20;
    let clusters = h.rebase(LOADING_CLUSTERS);
    for k in 0..20u32 {
        let path: u32 = h.read(clusters + 4 * k);
        let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
        let cluster = h.call_sym(LOAD_PCLUSTER, Conv::Thiscall(manager), &[path]);
        h.write(value, cluster);
        if cluster != 0 {
            let n = h.read::<u32>(this + 0x18) & 0x7fff_ffff;
            let storage = h.call_sym(
                PHYRE_MALLOC,
                Conv::Cdecl,
                &[n.wrapping_add(1).wrapping_mul(4), 4],
            );
            h.call_sym(PUSH_CLUSTER, Conv::Thiscall(this + 0x18), &[storage, value]);
        }
    }
    if h.read::<u32>(this + 0x18) & 0x7fff_ffff != 0 {
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        let n = h.read::<u32>(this + 0x18) & 0x7fff_ffff;
        let list: u32 = h.read(this + 0x1c);
        h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[list, n]);
    }
    let path = Global::<u32>::new(LOADING_TIP_CLUSTER).get(h);
    let manager = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
    let tip = h.call_sym(LOAD_PCLUSTER, Conv::Thiscall(manager), &[path]);
    h.write(this + 0x28, tip);
    if tip != 0 {
        h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[this + 0x28, 1]);
    }
    let mut i = 0u32;
    while i < h.read::<u32>(this + 0x18) & 0x7fff_ffff {
        let list: u32 = h.read(this + 0x1c);
        let cluster: u32 = h.read(list.wrapping_add(4 * i));
        seek_textures(h, frame, cluster);
        if h.read::<u32>(frame + 0xc) != 0 {
            let texture = h.read::<u32>(frame + 0x10).wrapping_add(h.read(frame));
            h.write(value, texture);
            let n = h.read::<u32>(this + 0x20) & 0x7fff_ffff;
            let storage = h.call_sym(
                PHYRE_MALLOC,
                Conv::Cdecl,
                &[n.wrapping_add(1).wrapping_mul(4), 4],
            );
            h.call_sym(PUSH_TEXTURE, Conv::Thiscall(this + 0x20), &[storage, value]);
        }
        i += 1;
    }
    let tip: u32 = h.read(this + 0x28);
    h.write(this + 0x2c, 0u32);
    if tip == 0 {
        return 0;
    }
    let r = seek_textures(h, frame, tip);
    if h.read::<u32>(frame + 0xc) == 0 {
        return r;
    }
    let texture = h.read::<u32>(frame + 0x10).wrapping_add(h.read(frame));
    h.write(this + 0x2c, texture);
    texture
}

// ---- the FMOD command queue

/// `WaitForSingleObject` (import slot `0x00b0c0f8`; stdcall).
pub const WAIT_FOR_SINGLE_OBJECT: Global<u32> = Global::new(0x00b0_c0f8);
/// `WaitForSingleObjectEx` (import slot `0x00b0c080`).
pub const WAIT_FOR_SINGLE_OBJECT_EX: Global<u32> = Global::new(0x00b0_c080);
/// `ReleaseMutex` (import slot `0x00b0c1c0`).
pub const RELEASE_MUTEX: Global<u32> = Global::new(0x00b0_c1c0);
/// `SetEvent` (import slot `0x00b0c0ec`).
pub const SET_EVENT: Global<u32> = Global::new(0x00b0_c0ec);
/// `ResetEvent` (import slot `0x00b0c0f0`).
pub const RESET_EVENT: Global<u32> = Global::new(0x00b0_c0f0);
/// The command ring: 150 records of five words (command, four
/// arguments) at `0x00ce8458`; the manager's write index `+0x28`, read
/// index `+0x2c`.
pub const COMMAND_RING: u32 = 0x00ce_8458;
/// Records in [`COMMAND_RING`].
pub const RING_LEN: u32 = 150;
/// `g_AtelBattleSepsLoaded`.
pub const BATTLE_SEPS_LOADED: Global<u32> = Global::new(0x0132_5b7c);
/// `g_CurrentMapNo`.
pub const CURRENT_MAP: Global<u32> = Global::new(0x012f_bc40);
/// `isEternalCalmScene(map)` (cdecl).
pub const IS_ETERNAL_CALM: u32 = 0x0067_ab50;
/// `printToOutput(prefix, fmt, ...)` (cdecl).
pub const PRINT_TO_OUTPUT: u32 = 0x0070_7b20;
/// Its prefix, the full ring's per-record line and its last line.
pub const RING_FULL_TEXTS: [u32; 3] = [0x00b4_c30c, 0x00b4_c2f4, 0x00b4_c31c];
/// `g_FmodCommandHandlers` (70 cdecl handlers `(fmod, &args)`).
pub const COMMAND_HANDLERS: u32 = 0x00c3_a368;
/// The synchronous command's time-out line.
pub const SYNC_TIMEOUT_TEXT: u32 = 0x00b4_c348;

/// The ring record `i` (as given: the original reduces some indices,
/// not all).
fn ring_record(h: &impl Host, i: u32) -> u32 {
    h.rebase(COMMAND_RING).wrapping_add(i.wrapping_mul(20))
}

/// `i % 150` as `idiv` computes it (signed).
fn ring_index(i: u32) -> u32 {
    (i.cast_signed() % RING_LEN.cast_signed()).cast_unsigned()
}

fn stdcall_import(h: &mut impl Host, slot: Global<u32>, args: &[u32]) -> u32 {
    let f = slot.get(h);
    h.call(f, Conv::Stdcall, args) // call width: 3 at most
}

/// `Fmod_PushCommand(manager; cmd, a, b, c, d)` (thiscall, ret 0x14):
/// under the queue mutex (`+0x10`): commands 0x3b and 0x3d first look for
/// a queued record (from the read index up to the write index, at most
/// 150) with the same command and `a`, and update its `b, c, d` in
/// place; with battle sounds loaded command 0x31 is dropped; else the
/// record is appended at the write index (when the ring is full every
/// queued command is printed); in the Eternal Calm scenes commands 8 and
/// 0x3a also wake the FMOD thread (`SetEvent(+0x18)`). Returns 0. The
/// original keeps the new write index in `a`'s argument slot (`slots`).
pub fn push_command(h: &mut impl Host, slots: u32, this: u32, args: [u32; 5]) -> u32 {
    let [cmd, a, b, c, d] = args;
    let mutex: u32 = h.read(this + 0x10);
    stdcall_import(h, WAIT_FOR_SINGLE_OBJECT, &[mutex, u32::MAX]);
    let mut done = false;
    if cmd == 0x3b || cmd == 0x3d {
        let mut pos: u32 = h.read(this + 0x2c);
        for _ in 0..RING_LEN {
            let i = ring_index(pos);
            if i == h.read::<u32>(this + 0x28) {
                break;
            }
            let rec = ring_record(h, i);
            if h.read::<u32>(rec) == cmd && h.read::<u32>(rec + 4) == a {
                h.write(rec + 8, b);
                h.write(rec + 0xc, c);
                h.write(rec + 0x10, d);
                done = true;
                break;
            }
            pos = pos.wrapping_add(1);
        }
    }
    if !done && (BATTLE_SEPS_LOADED.get(h) == 0 || cmd != 0x31) {
        let w: u32 = h.read(this + 0x28);
        let rec = ring_record(h, w);
        for (k, v) in [cmd, a, b, c, d].into_iter().enumerate() {
            h.write(rec + 4 * k as u32, v);
        }
        let next = ring_index(w.wrapping_add(1));
        h.write(slots + 4, next);
        if next == h.read::<u32>(this + 0x2c) {
            let [prefix, line, last] = RING_FULL_TEXTS.map(|t| h.rebase(t));
            for i in 0..RING_LEN {
                let at = ring_index(h.read::<u32>(this + 0x2c).wrapping_add(i));
                let rec = ring_record(h, at);
                let queued: u32 = h.read(rec);
                h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, line, queued]);
            }
            h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, last]);
        }
        let mutex: u32 = h.read(this + 0x10);
        h.write(this + 0x28, next);
        stdcall_import(h, RELEASE_MUTEX, &[mutex]);
        let map = CURRENT_MAP.get(h);
        if h.call_sym(IS_ETERNAL_CALM, Conv::Cdecl, &[map]) != 0 && (cmd == 8 || cmd == 0x3a) {
            let event: u32 = h.read(this + 0x18);
            stdcall_import(h, SET_EVENT, &[event]);
        }
        return 0;
    }
    let mutex: u32 = h.read(this + 0x10);
    stdcall_import(h, RELEASE_MUTEX, &[mutex]);
    0
}

/// `FmodManager_registCommandSync(manager; cmd, a, b, c, d)` (thiscall,
/// ret 0x14): under the sync mutex (`+0x14`) the command is stored
/// (`+0x30`, arguments `+0x34..+0x40`) and run: commands 0x11..0x13,
/// 0x16, 0x19 and 0x41 directly by their handler (`(fmod +8, &+0x34)`,
/// result `+0x48`); 0x3f too, and when that answers 0 the result is 1 if
/// a queued 0x33 command has the same `a`; the others go to the FMOD
/// thread: result 1, done flag `+0x44` cleared, up to two times the
/// thread is woken (`SetEvent(+0x18)`) and its answer awaited
/// (`WaitForSingleObjectEx(+0x1c, 5, 1)` until it stops returning -1,
/// `ResetEvent(+0x1c)`) until the flag is set; still unset, a time-out
/// line is printed; the flag is then set. Returns the result.
pub fn regist_command_sync(h: &mut impl Host, this: u32, args: [u32; 5]) -> u32 {
    let [cmd, a, b, c, d] = args;
    let mutex: u32 = h.read(this + 0x14);
    stdcall_import(h, WAIT_FOR_SINGLE_OBJECT, &[mutex, u32::MAX]);
    h.write(this + 0x30, cmd);
    h.write(this + 0x38, b);
    h.write(this + 0x3c, c);
    h.write(this + 0x40, d);
    h.write(this + 0x34, a);
    let direct = matches!(cmd, 0x11..=0x13 | 0x16 | 0x19 | 0x41);
    if direct || cmd == 0x3f {
        let handlers = h.rebase(COMMAND_HANDLERS);
        let f: u32 = h.read(handlers.wrapping_add(cmd.wrapping_mul(4)));
        let fmod: u32 = h.read(this + 8);
        let r = h.call(f, Conv::Cdecl, &[fmod, this + 0x34]);
        h.write(this + 0x48, r);
        if cmd == 0x3f && r == 0 {
            let mut pos: u32 = h.read(this + 0x2c);
            let write: u32 = h.read(this + 0x28);
            while pos != write {
                let rec = ring_record(h, pos);
                if h.read::<u32>(rec) == 0x33 && h.read::<u32>(rec + 4) == a {
                    h.write(this + 0x48, 1u32);
                    break;
                }
                pos = ring_index(pos.wrapping_add(1));
            }
        }
    } else {
        h.write(this + 0x48, 1u32);
        h.write(this + 0x44, 0u8);
        for _ in 0..2 {
            let event: u32 = h.read(this + 0x18);
            stdcall_import(h, SET_EVENT, &[event]);
            loop {
                let answer: u32 = h.read(this + 0x1c);
                if stdcall_import(h, WAIT_FOR_SINGLE_OBJECT_EX, &[answer, 5, 1]) != u32::MAX {
                    break;
                }
            }
            let answer: u32 = h.read(this + 0x1c);
            stdcall_import(h, RESET_EVENT, &[answer]);
            if h.read::<u8>(this + 0x44) != 0 {
                break;
            }
        }
        if h.read::<u8>(this + 0x44) == 0 {
            let prefix = h.rebase(RING_FULL_TEXTS[0]);
            let line = h.rebase(SYNC_TIMEOUT_TEXT);
            h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[prefix, line, cmd]);
        }
        h.write(this + 0x44, 1u8);
    }
    let mutex: u32 = h.read(this + 0x14);
    let result: u32 = h.read(this + 0x48);
    stdcall_import(h, RELEASE_MUTEX, &[mutex]);
    result
}

// ---- small constructors and the PC button icons

/// `operator_new_006304c0(bytes)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_04c0;

/// `count * size` as the CRT's array `new` asks for it: all ones on an
/// unsigned overflow.
fn array_bytes(count: u32, size: u32) -> u32 {
    count.saturating_mul(size)
}

/// `SlotTable_ctor_006af420(this; n)` (thiscall, ret 4): `n` index words
/// (`+0`, `new(4 n)`, each -1) and `n` 20-byte records (`new(20 n)`,
/// zeroed: a word and four `+0.0` floats; none when the allocation
/// fails), the records at `+4` and `+0xc`, the index words also at `+8`,
/// the count at `+0x10`. Returns `this`.
pub fn slot_table_ctor(h: &mut impl Host, this: u32, n: u32) -> u32 {
    let index = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[array_bytes(n, 4)]);
    h.write(this, index);
    let records = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[array_bytes(n, 20)]);
    if records != 0 {
        let mut rec = records;
        let mut left = n.wrapping_sub(1);
        while left.cast_signed() >= 0 {
            h.write(rec.wrapping_add(8), 0u32);
            h.write(rec, 0u32);
            h.write(rec.wrapping_add(4), 0u32);
            h.write(rec.wrapping_add(0x10), 0u32);
            h.write(rec.wrapping_add(0xc), 0u32);
            rec = rec.wrapping_add(20);
            left = left.wrapping_sub(1);
        }
    }
    let index: u32 = h.read(this);
    h.write(this + 4, records);
    h.write(this + 8, index);
    h.write(this + 0xc, records);
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let index: u32 = h.read(this);
        h.write(index.wrapping_add(4 * i), u32::MAX);
        i += 1;
    }
    h.write(this + 0x10, n);
    this
}

/// `RadialLine_ctor_006f7a30(this)` (thiscall): words `+0..+0x34` and
/// `+0x50..+0x74` zeroed, the six floats `+0x38..+0x4c` 1.0. Returns
/// `this`.
pub fn radial_line_ctor(h: &mut impl Host, this: u32) -> u32 {
    for k in 0..14u32 {
        h.write(this + 4 * k, 0u32);
    }
    for k in 0..6u32 {
        h.write(this + 0x38 + 4 * k, 1.0f32.to_bits());
    }
    for off in [0x68, 0x6c, 0x70, 0x74, 0x50, 0x54, 0x58, 0x5c, 0x60, 0x64] {
        h.write(this + off, 0u32);
    }
    this
}

/// `g_Settings` (a pointer; the key codes from `+0x60`).
pub const SETTINGS: Global<u32> = Global::new(0x022f_b504);

/// `RemapButtonIcon_getPcButtonUv(key, &u, &v)` (stdcall, ret 0xc): the
/// icon cell of key setting `key` (the code `settings + 0x60 + 4 key`):
/// column `(code - 0x1c) % 12`, row `(code - 0x1c) / 12` (signed), with
/// row 7 and columns 3 (code 0x74), 4 (0x76, 0x79), 6 (0x77, 0x7a), 5
/// (0x75, 0x78), 10 (`0x8000_0000`) for the special codes; `u = column /
/// 12`, `v = row * 0.125` (each through an `f32`). Returns `&v`. The
/// original also leaves the row (as `f32` bits) in `key`'s argument slot
/// and the row in its push-`ecx` slot.
pub fn pc_button_uv(h: &mut impl Host, key: u32, u: u32, v: u32) -> u32 {
    let settings = SETTINGS.get(h);
    let code: u32 = h.read(
        settings
            .wrapping_add(0x60)
            .wrapping_add(key.wrapping_mul(4)),
    );
    let x = code.wrapping_sub(0x1c).cast_signed();
    let (mut row, mut col) = (x.wrapping_div(12), x.wrapping_rem(12));
    match code {
        0x74 => (row, col) = (7, 3),
        0x76 | 0x79 => (row, col) = (7, 4),
        0x77 | 0x7a => (row, col) = (7, 6),
        0x75 | 0x78 => (row, col) = (7, 5),
        0x8000_0000 => (row, col) = (7, 10),
        _ => {}
    }
    store_cell(h, u, v, col, row);
    v
}

/// `g_ButtonData` (`0x00c5955c`) and its second word.
pub const BUTTON_DATA: [u32; 2] = [0x00c5_955c, 0x00c5_9560];

/// The icon cell's texture coordinates, each through an `f32` as the
/// original stores them: `u = col / 12`, `v = row * 0.125` (`fild`, then
/// `fstp` to an `f32` slot: the integer rounded to `f32` once).
#[allow(clippy::cast_precision_loss)]
fn store_cell(h: &mut impl Host, u: u32, v: u32, col: i32, row: i32) {
    let col = f64::from(col as f32);
    h.write(u, ((col / 12.0) as f32).to_bits());
    let row = f64::from(row as f32);
    h.write(v, ((row * 0.125) as f32).to_bits());
}

/// `RemapButtonIcon_getJoypadUV(this; key, &u, &v)` (thiscall, ret
/// 0xc): the pad icon cell of key setting `key + 0xe` (0x32, 0x33, 0x34
/// read 0x18, 0x19, 0x16; the code `settings + 0x60 + 4 k`): column
/// `(code - 0xb) % 12`, row `(code - 0xb) / 12` (signed); with the pad
/// glyph set (`g_ButtonData[1] - 0x3c` in 0..7) keys 0..3 take row 1,
/// column set / 2; without button data (both -1) and the remap mode
/// (`+0x150`) outside 0..3, keys 0..3 take row 2, columns 0 (key 0), 4,
/// 6, 2; the special key `0x8000_0000` takes row 3 by the remapped code
/// (the `settings + 0x98` word whose bit `+0x15c` names, else the code):
/// 0xb..0xe column 0, 0xf..0x12 1, 0x13..0x14 2, 0x15..0x16 3, then
/// with a glyph set row 2, column set; `0x4000_0000` row 1, column 4.
/// Returns `&v`. The original also leaves the code and then the row (as
/// `f32` bits) in `key`'s argument slot.
pub fn joypad_uv(h: &mut impl Host, this: u32, key: u32, u: u32, v: u32) -> u32 {
    let mut k = key.wrapping_add(0xe);
    k = match k {
        0x32 => 0x18,
        0x33 => 0x19,
        0x34 => 0x16,
        _ => k,
    };
    let settings = SETTINGS.get(h);
    let code: u32 = h.read(settings.wrapping_add(0x60).wrapping_add(k.wrapping_mul(4)));
    let remap: u32 = h.read(this + 0x15c);
    let mut remapped = code;
    if remap != u32::MAX {
        let mut bit = 1u32;
        for j in 0..20u32 {
            if remap == bit {
                remapped = h.read(settings.wrapping_add(0x98 + 4 * j));
            }
            bit = bit.rotate_left(1);
        }
    }
    let x = code.wrapping_sub(0xb).cast_signed();
    let (mut row, mut col) = (x.wrapping_div(12), x.wrapping_rem(12));
    let glyphs = Global::<u32>::new(BUTTON_DATA[1]).get(h);
    let set = glyphs.wrapping_sub(0x3c).cast_signed();
    let index = k.wrapping_sub(0xe);
    let pad_key = index <= 3;
    if (0..=7).contains(&set) && pad_key {
        row = 1;
        col = set / 2;
    }
    if glyphs == u32::MAX && pad_key && Global::<u32>::new(BUTTON_DATA[0]).get(h) == u32::MAX {
        let mode = h.read::<u32>(this + 0x150).cast_signed();
        if !(0..=3).contains(&mode) {
            col = match index {
                0 => 0,
                1 => 4,
                2 => 6,
                3 => 2,
                _ => col,
            };
            row = 2;
        }
    }
    if index == 0x8000_0000 {
        let r = remapped;
        for (lo, n, c) in [(0xb, 3, 0), (0xf, 3, 1), (0x13, 1, 2), (0x15, 1, 3)] {
            if r.wrapping_sub(lo) <= n {
                (row, col) = (3, c);
            }
        }
        if (0..=7).contains(&set) {
            (row, col) = (2, set);
        }
    }
    if index == 0x4000_0000 {
        (row, col) = (1, 4);
    }
    store_cell(h, u, v, col, row);
    v
}
