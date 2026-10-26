//! Continuation round of list `h4-glue-other`, batch 3: `FmodVoice`'s
//! stop, data release and state update, the sync manager's asynchronous
//! data change, the shape-field effect's initialisation, and the menu
//! geometry's texture-unload flag.
//!
//! Bridges that hand callees a local take `frame`, the block laid out as
//! the original's frame from its lowest local up.

use ffx_graphic::host::{Conv, Host};

use crate::frames::{ERRCHECK, FMOD_GET_STATE, FMOD_VOICE_FILE};
use crate::r3::{ASYNC_LOAD_GET, DJB_HASH};

// ---- FmodVoice

/// `FMOD::Event::setCallback` (the import's thunk, stdcall `(event, f,
/// data)`).
pub const EVENT_SET_CALLBACK: u32 = 0x0094_8cac;
/// `FMOD::Event::stop` (the import's thunk, stdcall `(event,
/// immediate)`).
pub const EVENT_STOP: u32 = 0x0094_8ca0;
/// `FmodVoice_clear(this)` (thiscall).
pub const VOICE_CLEAR: u32 = 0x0070_a590;
/// `FmodVoice_stop(this)` (thiscall).
pub const VOICE_STOP: u32 = 0x0070_b6b0;
/// `SleepThreadInMillisecond(ms)` (cdecl).
pub const SLEEP_MS: u32 = crate::h4_c2::SLEEP_MS;

/// `FmodVoice_stop(this)` (`0x0070b6b0`, thiscall; `frame` its state word,
/// which starts as `this`, the prologue's `push ecx`): with an event
/// (`+0x14`), its state (`ERRCHECK` line 0x2b1); a playing one (bit 3)
/// loses its callback and is stopped (line 0x2b7), and the play words
/// `+0x40`, `+0x44`, `+0x60`, `+0x58`, `+0x6c`..`+0x74` are cleared; then
/// `FmodVoice_clear`. Returns 0.
pub fn voice_stop(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    h.write(frame, this);
    let event: u32 = h.read(this + 0x14);
    if event == 0 {
        return 0;
    }
    let file = h.rebase(FMOD_VOICE_FILE);
    let r = h.call_sym(FMOD_GET_STATE, Conv::Stdcall, &[event, frame]);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x2b1]);
    if h.read::<u8>(frame) & 8 != 0 {
        let event: u32 = h.read(this + 0x14);
        h.call_sym(EVENT_SET_CALLBACK, Conv::Stdcall, &[event, 0, 0]);
        let event: u32 = h.read(this + 0x14);
        let r = h.call_sym(EVENT_STOP, Conv::Stdcall, &[event, 0]);
        h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x2b7]);
        for off in [0x40, 0x44, 0x60, 0x58, 0x6c, 0x70, 0x74] {
            h.write(this + off, 0u32);
        }
    }
    h.call_sym(VOICE_CLEAR, Conv::Thiscall(this), &[]);
    0
}

/// The voice banks `FmodVoice_releaseData` walks: 21 words at `+0x18`
/// (projects) and `+0xc`.
pub const VOICE_BANKS: u32 = 21;

/// `FmodVoice_releaseData(this)` (`0x0070b0b0`, thiscall):
/// `FmodVoice_stop`, the play words `+0x40`, `+0x44`, `+0x4c`, `+0x10`
/// cleared; each of the 21 projects of `+0x18` has its virtual `+0x28`
/// (stdcall) checked (`ERRCHECK` line 0x25d), 100 ms of sleep, then each
/// its virtual `+0` (line 0x265) and its slot cleared; each word of
/// `+0xc` cleared too. Returns 0.
pub fn voice_release_data(h: &mut impl Host, this: u32) -> u32 {
    h.call_sym(VOICE_STOP, Conv::Thiscall(this), &[]);
    for off in [0x40, 0x44, 0x4c, 0x10] {
        h.write(this + off, 0u32);
    }
    let file = h.rebase(FMOD_VOICE_FILE);
    for i in 0..VOICE_BANKS {
        let projects: u32 = h.read(this + 0x18);
        let project: u32 = h.read(projects.wrapping_add(4 * i));
        if project != 0 {
            let vtable: u32 = h.read(project);
            let f: u32 = h.read(vtable.wrapping_add(0x28));
            let r = h.call(f, Conv::Stdcall, &[project]);
            h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x25d]);
        }
    }
    h.call_sym(SLEEP_MS, Conv::Cdecl, &[100]);
    for i in 0..VOICE_BANKS {
        let projects: u32 = h.read(this + 0x18);
        let project: u32 = h.read(projects.wrapping_add(4 * i));
        if project != 0 {
            let vtable: u32 = h.read(project);
            let f: u32 = h.read(vtable);
            let r = h.call(f, Conv::Stdcall, &[project]);
            h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x265]);
            let projects: u32 = h.read(this + 0x18);
            h.write(projects.wrapping_add(4 * i), 0u32);
        }
        let words: u32 = h.read(this + 0xc);
        h.write(words.wrapping_add(4 * i), 0u32);
    }
    0
}

/// `graphicGetTimeInMicroseconds()` (cdecl, `edx:eax`).
pub const TIME_US: u32 = 0x0064_1420;
/// `FmodVoice_checkEvent_0070a4a0(this; arg)` (thiscall, ret 4).
pub const VOICE_UPDATE_PLAY: u32 = 0x0070_a4a0;

/// `FmodVoice_updateState(this; arg)` (`0x0070b750`, thiscall, ret 4):
/// with an event (`+0x14`): a playing voice (`+0x40 == 1`) with a time
/// limit (`+0x6c`, ms) notes its start (`+0x70`, 64-bit µs) when it just
/// started (`+0x54 == 1`) and, once started, its now (`+0x78`) and the
/// elapsed ms (`(now - start) / 1000`, unsigned, low word); then
/// `0x0070a4a0(this; arg)`; a voice that stopped since the last update
/// (`+0x40` 0, `+0x48` not) or ran past its limit (elapsed and limit
/// signed, elapsed above both 0 and the limit) is cleared
/// (`FmodVoice_clear`); `+0x48 = +0x40`. Returns 0. `time(h, f)` calls the
/// clock at `f` and returns its `edx:eax`.
pub fn voice_update_state<H: Host>(
    h: &mut H,
    this: u32,
    arg: u32,
    time: &mut impl FnMut(&mut H, u32) -> u64,
) -> u32 {
    if h.read::<u32>(this + 0x14) == 0 {
        return 0;
    }
    let clock = h.rebase(TIME_US);
    let mut elapsed = 0u32;
    if h.read::<u32>(this + 0x40) == 1 && h.read::<u32>(this + 0x6c) != 0 {
        if h.read::<u32>(this + 0x54) == 1 {
            let t = time(h, clock);
            h.write(this + 0x70, t);
        }
        let start: u64 = h.read(this + 0x70);
        if start != 0 {
            let now = time(h, clock);
            h.write(this + 0x78, now as u32);
            h.write(this + 0x7c, (now >> 32) as u32);
            let start: u64 = h.read(this + 0x70);
            elapsed = (now.wrapping_sub(start) / 1000) as u32;
        }
    }
    h.call_sym(VOICE_UPDATE_PLAY, Conv::Thiscall(this), &[arg]);
    let state: u32 = h.read(this + 0x40);
    let clear = if state == 0 && h.read::<u32>(this + 0x48) != 0 {
        true
    } else if state == 1 {
        let limit: u32 = h.read(this + 0x6c);
        limit != 0 && elapsed.cast_signed() > 0 && elapsed.cast_signed() > limit.cast_signed()
    } else {
        false
    };
    if clear {
        h.call_sym(VOICE_CLEAR, Conv::Thiscall(this), &[]);
    }
    let state: u32 = h.read(this + 0x40);
    h.write(this + 0x48, state);
    0
}

// ---- the sync manager

/// `SyncManager_releaseSyncData(this)` (thiscall).
pub const SYNC_RELEASE_DATA: u32 = 0x006f_aee0;
/// `SyncManager_syncDataChangeAsyncTask` (`0x006fb130`), the task the
/// change registers.
pub const SYNC_CHANGE_TASK: u32 = 0x006f_b130;
/// `AsyncLoadManager_registAsyncLoadTask(mgr, f, kind, a_lo, a_hi, b_lo,
/// b_hi)` (cdecl, seven words).
pub const REGIST_ASYNC_TASK: u32 = 0x0069_18b0;

/// `SyncManager_syncDataChangeAsync(this; id)` (`0x006fb080`, thiscall,
/// ret 4): an id in 1..=500 other than the current one (`+0xc`) becomes
/// it, `+0x1c` (found) and `+8` cleared; when the id is in the list
/// (`+0x40`, count `+0x18`, signed) found is set and, after waiting for
/// the loader's done flag `+0x20` (5 ms sleeps), the old data is released,
/// the flag cleared and `SyncManager_syncDataChangeAsyncTask` registered
/// with the manager and the id (each as a sign-extended 64-bit value).
/// Returns 0.
pub fn sync_data_change_async(h: &mut impl Host, this: u32, id: u32) -> u32 {
    if id.wrapping_sub(1) > 0x1f3 || h.read::<u32>(this + 0xc) == id {
        return 0;
    }
    let n: u32 = h.read(this + 0x18);
    h.write(this + 0xc, id);
    h.write(this + 0x1c, 0u32);
    h.write(this + 8, 0u32);
    let list: u32 = h.read(this + 0x40);
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        if h.read::<u32>(list.wrapping_add(4 * i)) == id {
            h.write(this + 0x1c, 1u32);
            break;
        }
        i += 1;
    }
    if h.read::<u32>(this + 0x1c) != 1 {
        return 0;
    }
    while h.read::<u32>(this + 0x20) == 0 {
        h.call_sym(SLEEP_MS, Conv::Cdecl, &[5]);
    }
    h.call_sym(SYNC_RELEASE_DATA, Conv::Thiscall(this), &[]);
    let hi = |v: u32| if v.cast_signed() < 0 { u32::MAX } else { 0 };
    let task = h.rebase(SYNC_CHANGE_TASK);
    h.write(this + 0x20, 0u32);
    let mgr = h.call_sym(ASYNC_LOAD_GET, Conv::Cdecl, &[]);
    h.call_sym(
        REGIST_ASYNC_TASK,
        Conv::Cdecl,
        &[mgr, task, 2, this, hi(this), id, hi(id)],
    );
    0
}

// ---- effects

/// `createVFXRenderDataBufferShapeField(a, b, c, d, e)` (cdecl).
pub const CREATE_SHAPE_FIELD: u32 = 0x0071_4d40;
/// `graphicVFXSetup(a, b)` (cdecl).
pub const VFX_SETUP: u32 = 0x0064_5780;
/// `graphicVFXGetRenderDataByInstance(a, b)` (cdecl).
pub const VFX_RENDER_DATA: u32 = 0x0064_53d0;
/// `graphicVFXDestroy(a, b)` (cdecl).
pub const VFX_DESTROY: u32 = 0x0064_5280;

/// `InitVFXRenderDataShapeField(a, b, c, d, e)` (`0x007113d0`, cdecl;
/// also a `g_MagicImports` entry): the shape-field buffers
/// (`createVFXRenderDataBufferShapeField(a, b, c, d, e)`; failing, the
/// effect is destroyed), `graphicVFXSetup(a, b)`, then every one of the
/// render data's `+0x90` (signed) 0x6c-byte records (`+0x94`) has its
/// word `+8` divided by 3; a failed setup destroys the effect. Returns
/// what the original leaves in `eax`: `graphicVFXDestroy`'s, else the low
/// word of the last division's product (`w * 0xaaaaaaab`), else the
/// render data.
pub fn init_vfx_render_data_shape_field(h: &mut impl Host, args: [u32; 5]) -> u32 {
    let [a, b, ..] = args;
    if h.call_sym(CREATE_SHAPE_FIELD, Conv::Cdecl, &args) == 0 {
        return h.call_sym(VFX_DESTROY, Conv::Cdecl, &[a, b]);
    }
    let ok = h.call_sym(VFX_SETUP, Conv::Cdecl, &[a, b]);
    let data = h.call_sym(VFX_RENDER_DATA, Conv::Cdecl, &[a, b]);
    let mut eax = data;
    if data != 0 {
        let mut i = 0u32;
        while i.cast_signed() < h.read::<u32>(data + 0x90).cast_signed() {
            let records: u32 = h.read(data + 0x94);
            let at = records.wrapping_add(i.wrapping_mul(0x6c)).wrapping_add(8);
            let w: u32 = h.read(at);
            let product = u64::from(w) * 0xaaaa_aaab;
            h.write(at, (product >> 33) as u32);
            eax = product as u32;
            i += 1;
        }
    }
    if ok == 0 {
        return h.call_sym(VFX_DESTROY, Conv::Cdecl, &[a, b]);
    }
    eax
}

// ---- menu geometry

/// The menu geometry's texture tree (`0x00c3419c`, a `PRedBlackTree`:
/// `+0xc` its leftmost entry).
pub const MENU_TEXTURE_TREE: u32 = 0x00c3_419c;
/// `Phyre_PRedBlackTree_successor(tree; node)` (thiscall, ret 4).
pub const TREE_SUCCESSOR: u32 = crate::h4_c2::TREE_SUCCESSOR;

/// `MenuDynamicGeometry_setTextureInMaterialUnloaded(name)`
/// (`0x006a3740`, stdcall): the first entry of the texture tree (in
/// order) whose record (`+0xc`) holds the name's `DJB_hash` gets its
/// loaded word (`+4`) cleared. Returns what the original leaves in `eax`:
/// that record, else the last record compared, else the hash (an empty
/// tree).
pub fn set_texture_in_material_unloaded(h: &mut impl Host, name: u32) -> u32 {
    let hash = h.call_sym(DJB_HASH, Conv::Cdecl, &[name]);
    let tree = h.rebase(MENU_TEXTURE_TREE);
    let mut node: u32 = h.read(tree + 0xc);
    if node != tree && node != 0 {
        loop {
            let next: u32 = h.read(node);
            if next == tree {
                break;
            }
            node = next;
        }
    }
    if node == tree {
        return hash;
    }
    let mut next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(tree), &[node]);
    let mut eax;
    loop {
        let record: u32 = h.read(node + 0xc);
        eax = record;
        if h.read::<u32>(record) == hash {
            h.write(record + 4, 0u32);
            return record;
        }
        node = next;
        if next != tree {
            next = h.call_sym(TREE_SUCCESSOR, Conv::Thiscall(tree), &[next]);
        }
        if node == tree {
            return eax;
        }
    }
}

// ---- the FMOD bank's two event groups

/// `Phyre_PFreeList_PFreeList(this; size, count, align, name)` (thiscall,
/// ret 0x10).
pub const FREE_LIST_CTOR: u32 = 0x0049_c480;
/// The free lists' name `"PMapPair"` (`0x00b13bc8`).
pub const MAP_PAIR_NAME: u32 = 0x00b1_3bc8;
/// `"loop"` (`0x00b4fea4`) and `"nonloop"` (`0x00b4fe9c`), the groups the
/// two constructors ask the project for.
pub const GROUP_NAMES: [u32; 2] = [0x00b4_fea4, 0x00b4_fe9c];
/// `SfxEventGroup_load_00710c50(this; arg)` (thiscall, ret 4).
pub const GROUP_LOAD: u32 = 0x0071_0c50;
/// `FmodEventTree_load_00710370(this; arg)` (thiscall, ret 4).
pub const TREE_LOAD: u32 = 0x0071_0370;
/// `Phyre_PRedBlackTree_orphanAllContents(this)` (thiscall).
pub const TREE_ORPHAN_ALL: u32 = crate::h4_c2::TREE_ORPHAN_ALL;
/// `Phyre_PFreeList_reset(this)` (thiscall).
pub const FREE_LIST_RESET: u32 = crate::h4_c2::FREE_LIST_RESET;

/// The tree at `this + 4` empty (its head, `+0xc`..`+0x14` all itself)
/// with a `"PMapPair"` free list of `size`-byte nodes at `+0x1c`.
fn empty_tree(h: &mut impl Host, this: u32, size: u32) {
    let tree = this + 4;
    for off in [0, 4, 8, 0x14, 0x10, 0xc] {
        h.write(tree + off, tree);
    }
    let name = h.rebase(MAP_PAIR_NAME);
    h.call_sym(
        FREE_LIST_CTOR,
        Conv::Thiscall(tree + 0x18),
        &[size, 8, 4, name],
    );
}

/// The project's group `name` into `+0` (its virtual `+8`, stdcall
/// `(project, name, 1, &group)`); 0 is success.
fn get_group(h: &mut impl Host, this: u32, project: u32, name: u32) -> u32 {
    let vtable: u32 = h.read(project);
    let f: u32 = h.read(vtable.wrapping_add(8));
    let name = h.rebase(name);
    h.call(f, Conv::Stdcall, &[project, name, 1, this])
}

/// `SfxEventGroup_ctor_00710780(this; project, arg)` (thiscall, ret 8):
/// an empty event tree (`+4`, 0x1c-byte nodes); with a project whose
/// `"loop"` group (into `+0`) it gets, `SfxEventGroup_load_00710c50(this;
/// arg)`; otherwise `+0 = 0`. Returns `this`. The SEH frame is not kept.
pub fn sfx_event_group_ctor(h: &mut impl Host, this: u32, project: u32, arg: u32) -> u32 {
    empty_tree(h, this, 0x1c);
    if project != 0 && get_group(h, this, project, GROUP_NAMES[0]) == 0 {
        h.call_sym(GROUP_LOAD, Conv::Thiscall(this), &[arg]);
        return this;
    }
    h.write(this, 0u32);
    this
}

/// `FmodEventTree_ctor_0070ff50(this; project, arg)` (thiscall, ret 8): an
/// empty event tree (`+4`, 0x14-byte nodes); without a project `+0 = 0`
/// and the tree orphaned and its free list reset; with one whose
/// `"nonloop"` group (into `+0`) it gets, `FmodEventTree_load_00710370(this;
/// arg)`, else `+0 = 0`. Returns `this`. The SEH frame is not kept.
pub fn fmod_event_tree_ctor(h: &mut impl Host, this: u32, project: u32, arg: u32) -> u32 {
    empty_tree(h, this, 0x14);
    if project == 0 {
        h.write(this, 0u32);
        h.call_sym(TREE_ORPHAN_ALL, Conv::Thiscall(this + 4), &[]);
        h.call_sym(FREE_LIST_RESET, Conv::Thiscall(this + 0x1c), &[]);
    } else if get_group(h, this, project, GROUP_NAMES[1]) == 0 {
        h.call_sym(TREE_LOAD, Conv::Thiscall(this), &[arg]);
    } else {
        h.write(this, 0u32);
    }
    this
}

// ---- Iggy callbacks

/// `GetDerefU32_00a28d10(this)` (thiscall): `**(this + 0x6c)`.
pub const GET_DEREF_U32: u32 = 0x00a2_8d10;

/// `IggyMenuManager_dispatchCallback_00a26100(this; a, player, call)`
/// (thiscall, ret 0xc; `IggyMenuManager_callback_00a260f0` jumps here):
/// the name `**call` hashed as `DJB_hash` does (seed 0x1505, `h * 33 +
/// c` over signed bytes, top bit cleared); each active (byte `+0`) of the
/// `+0x10` (top bit masked, read again each step) 16-byte handlers at
/// `+0x14` with that hash (`+4`) whose object's player
/// (`GetDerefU32_00a28d10(+0xc)`) is `player` gets `+8(object, call +
/// 0x10)` (cdecl). Returns 1.
pub fn dispatch_callback(h: &mut impl Host, this: u32, player: u32, call: u32) -> u32 {
    let mut p: u32 = h.read(call);
    let mut hash = 0x1505u32;
    loop {
        let c = h.read::<u8>(p);
        if c == 0 {
            break;
        }
        hash = hash
            .wrapping_mul(33)
            .wrapping_add(i32::from(c.cast_signed()).cast_unsigned());
        p = p.wrapping_add(1);
    }
    let hash = hash & 0x7fff_ffff;
    let mut i = 0u32;
    while i < h.read::<u32>(this + 0x10) & 0x7fff_ffff {
        let handlers: u32 = h.read(this + 0x14);
        let at = handlers.wrapping_add(i.wrapping_mul(0x10));
        if h.read::<u8>(at) != 0 && h.read::<u32>(at.wrapping_add(4)) == hash {
            let object: u32 = h.read(at.wrapping_add(0xc));
            if h.call_sym(GET_DEREF_U32, Conv::Thiscall(object), &[]) == player {
                let handlers: u32 = h.read(this + 0x14);
                let at = handlers.wrapping_add(i.wrapping_mul(0x10));
                let object: u32 = h.read(at.wrapping_add(0xc));
                let f: u32 = h.read(at.wrapping_add(8));
                h.call(f, Conv::Cdecl, &[object, call.wrapping_add(0x10)]);
            }
        }
        i += 1;
    }
    1
}

// ---- strings

/// `memcmp_00401020(a, b, n)` (cdecl).
pub const MEMCMP: u32 = 0x0040_1020;
/// `std_string_begin_00403d80(this)` (thiscall): the string's first
/// character.
pub const STRING_BEGIN: u32 = 0x0040_3d80;

/// The characters of the MSVC `std::string` at `this` (`+0x14` capacity:
/// 16 or more points at `+0`, else inline).
fn string_data(h: &impl Host, this: u32) -> u32 {
    if h.read::<u32>(this + 0x14) >= 0x10 {
        h.read(this)
    } else {
        this
    }
}

/// `std_string_rfind_00638340(this; s, pos, n)` (thiscall, ret 0xc): the
/// last place at or before `pos` (at most `size - n`) where the `n` bytes
/// of `s` occur (first byte, then `memcmp`), walking down until the
/// string's begin (`0x00403d80`, asked after each miss); -1 when `n`
/// exceeds the size or nothing matches; an empty `s` gives `pos` when it
/// is below the size, else the size. The original keeps `s`'s first byte
/// in its `pos` slot's top byte, which its callers do not read.
pub fn string_rfind(h: &mut impl Host, this: u32, [s, pos, n]: [u32; 3]) -> u32 {
    let size: u32 = h.read(this + 0x10);
    if n == 0 {
        return if pos < size { pos } else { size };
    }
    if n > size {
        return u32::MAX;
    }
    let start = pos.min(size - n);
    let mut at = string_data(h, this).wrapping_add(start);
    let first = h.read::<u8>(s);
    loop {
        if h.read::<u8>(at) == first && h.call_sym(MEMCMP, Conv::Cdecl, &[at, s, n]) == 0 {
            return at.wrapping_sub(string_data(h, this));
        }
        if at == h.call_sym(STRING_BEGIN, Conv::Thiscall(this), &[]) {
            return u32::MAX;
        }
        at = at.wrapping_sub(1);
    }
}

// ---- FmodMusic

/// The file name `FmodMusic` passes to `ERRCHECK` (`0x00b4f4a0`).
pub const FMOD_MUSIC_FILE: u32 = 0x00b4_f4a0;
/// The music slots: 182 records of 0x3c bytes at `+0x10`.
pub const MUSIC_SLOTS: u32 = 0xb6;

/// The project `FmodMusic` uses: `+4` when `+0x2c` is 1, else `+0`.
fn music_project(h: &impl Host, this: u32) -> u32 {
    if h.read::<u32>(this + 0x2c) == 1 {
        h.read(this + 4)
    } else {
        h.read(this)
    }
}

/// Frees slot `i`'s event through the project's virtual `+8` (stdcall
/// `(project, event, 1)`, `ERRCHECK` at `line`), then clears the slot's
/// words (`+0x10` as 0.0) and the music's `+0x20`.
fn free_music_slot(h: &mut impl Host, this: u32, project: u32, i: u32, event: u32, line: u32) {
    let vtable: u32 = h.read(project);
    let f: u32 = h.read(vtable.wrapping_add(8));
    let r = h.call(f, Conv::Stdcall, &[project, event, 1]);
    let file = h.rebase(FMOD_MUSIC_FILE);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, line]);
    h.write(this + 0x20, 0u32);
    for off in [0, 4, 8, 0x20, 0xc, 0x10, 0x14, 0x18, 0x1c] {
        let slots: u32 = h.read(this + 0x10);
        let at = slots.wrapping_add(i.wrapping_mul(0x3c)).wrapping_add(off);
        // `+0x10` is the float 0.0 (`fldz`/`fstp`), the same bits
        h.write(at, 0u32);
    }
}

/// One slot of [`music_clear`] and [`music_clear_all`]: a loaded (`+0`),
/// not playing (`+4` 0) and not kept (`+8` other than 1) event is freed
/// (`ERRCHECK` line 0x15d).
fn clear_music_slot(h: &mut impl Host, this: u32, i: u32) {
    let project = music_project(h, this);
    if project == 0 || i > 0xb5 {
        return;
    }
    let slots: u32 = h.read(this + 0x10);
    let at = slots.wrapping_add(i.wrapping_mul(0x3c));
    let event: u32 = h.read(at);
    if event != 0 && h.read::<u32>(at + 4) == 0 && h.read::<u32>(at + 8) != 1 {
        free_music_slot(h, this, project, i, event, 0x15d);
    }
}

/// `FmodMusic_clear(this; i)` (`0x00708720`, thiscall, ret 4): slot `i`
/// (at most 0xb5) freed when loaded, idle and not kept. Returns 0.
pub fn music_clear(h: &mut impl Host, this: u32, i: u32) -> u32 {
    clear_music_slot(h, this, i);
    0
}

/// `FmodMusic_clearAll(this)` (`0x00708810`, thiscall): [`music_clear`]
/// on every slot (the project chosen again for each). Returns 0.
pub fn music_clear_all(h: &mut impl Host, this: u32) -> u32 {
    for i in 0..MUSIC_SLOTS {
        clear_music_slot(h, this, i);
    }
    0
}

/// `FmodMusic_forceClearAll(this)` (`0x00708af0`, thiscall): with a
/// project (chosen once), every loaded slot is freed (`ERRCHECK` line
/// 0x355), playing or not. Its one caller (`FmodMusic_switchBGM`) reads
/// no result.
pub fn music_force_clear_all(h: &mut impl Host, this: u32) {
    let project = music_project(h, this);
    if project == 0 {
        return;
    }
    for i in 0..MUSIC_SLOTS {
        let slots: u32 = h.read(this + 0x10);
        let event: u32 = h.read(slots.wrapping_add(i.wrapping_mul(0x3c)));
        if event != 0 {
            free_music_slot(h, this, project, i, event, 0x355);
        }
    }
}
