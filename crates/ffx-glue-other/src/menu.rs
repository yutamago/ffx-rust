//! The menus' and the text layer's glue: the escape menu's handlers and
//! sounds, `IggyMenu` and `IggyMenuManager`, `IconPanel`, `TextUtil`, the
//! dynamic geometries, the create-once singletons, the async loader's
//! request bodies.

use ffx_bridge::{Global, Mem};
use ffx_graphic::host::{Conv, Host};

use crate::misc::{
    IGGY_MENU_MANAGER, IGGY_SET_ENABLE, OPERATOR_NEW, PSTRING_ASSIGN, TEXT_TITLE_DIRTY,
};
use crate::sound::OPERATOR_DELETE;

/// `IggyMenuHandlers_addCall(this; f, entry)` (thiscall).
pub const ADD_CALL: u32 = 0x00a2_8df0;
/// The handler `0x00702570` registers: `0x0068e560`.
pub const ESC_HANDLER: u32 = 0x0068_e560;
/// The 24-byte handler entries `0x00702570` indexes (`0x00c3a2d4`).
pub const ESC_ENTRIES: u32 = 0x00c3_a2d4;
/// `IggyPlayer_QueueDraw_00a3d9b0(player; a)` (thiscall).
pub const QUEUE_DRAW: u32 = 0x00a3_d9b0;
/// `Phyre_PCluster_create(name, root, 0)` (stdcall).
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
/// The cluster root `0x006f7a00` creates in (`0x00ccbe88`).
pub const CLUSTER_ROOT: u32 = 0x00cc_be88;
/// `ClusterManager_GetInstance()`.
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
/// `ClusterManager_loadPCluster(this; path)` (thiscall).
pub const LOAD_PCLUSTER: u32 = 0x0069_b940;
/// `AsyncLoadManager_sGetInstance()`.
pub const ASYNC_LOAD_MANAGER: u32 = 0x0069_1b40;
/// The `MotionBlur` loaded handler `0x006c5150` (thiscall; a, b).
pub const MOTION_BLUR_ON_LOADED: u32 = 0x006c_5150;
/// `graphicVFXGetRenderDataByInstance(lo, hi)` (cdecl).
pub const VFX_RENDER_DATA: u32 = 0x0064_53d0;
/// The instance `0x007185e0` looks up: `(0x78, 0x5bc)`.
pub const VIDEO_INSTANCE: [u32; 2] = [0x78, 0x5bc];
/// The word `0x007185e0` needs set (`0x00ccb9cc`).
pub const VIDEO_READY: Global<u32> = Global::new(0x00cc_b9cc);
/// `graphicTextureVideoPlay(on)` (cdecl).
pub const TEXTURE_VIDEO_PLAY: u32 = 0x0064_4280;
/// `SndForcePlayClick(id)` (cdecl).
pub const FORCE_PLAY_CLICK: u32 = 0x0088_60f0;
/// `FFEscMenu +0x117`: a sound played this frame (byte).
pub const ESC_SOUND_PLAYED: u32 = 0x117;
/// `TextUtil_doOnAllTextUtilAssetsLoaded(this)` (thiscall).
pub const TEXT_ALL_LOADED: u32 = 0x006f_60a0;
/// `SetFields_006f7700(panel)` (thiscall), the `IconPanel` constructor.
pub const ICON_PANEL_CTOR: u32 = 0x006f_7700;
/// `g_IconPanel` (`0x00ce82e4`).
pub const ICON_PANEL: Global<u32> = Global::new(0x00ce_82e4);
/// The battle-character load countdown (`0x00c421dc`).
pub const CHR_LOAD_COUNT: Global<u32> = Global::new(0x00c4_21dc);
/// Its message (`.rdata`).
pub const CHR_LOAD_MESSAGE: u32 = 0x00b5_494c;
/// The import slot of `__iob_func`.
pub const IOB_FUNC: Global<u32> = Global::new(0x00b0_c350);
/// The import slot of `fprintf`.
pub const FPRINTF: Global<u32> = Global::new(0x00b0_c38c);
/// `IggyMenuManager_SetFields_00a25db0(mgr)` (thiscall), the constructor.
pub const IGGY_MANAGER_CTOR: u32 = 0x00a2_5db0;
/// `IggyMenu_dtor_IggyMenu(menu)` (thiscall).
pub const IGGY_MENU_DTOR: u32 = 0x00a2_8a40;
/// The post effect `0x006c5790` creates: its global, size, constructor.
pub const POST_EFFECT: (Global<u32>, u32, u32) = (Global::new(0x00cd_ec1c), 0x5b4, 0x006c_5670);
/// `RemapButtonIcon`: global, size, constructor.
pub const REMAP_BUTTON_ICON: (Global<u32>, u32, u32) =
    (Global::new(0x00ce_6dc0), 0x168, 0x006e_6330);
/// `IggyMenuManager`: global, size, constructor.
pub const IGGY_MENU_MANAGER_NEW: (Global<u32>, u32, u32) =
    (IGGY_MENU_MANAGER, 0x34, IGGY_MANAGER_CTOR);
/// `inputIsPressButton(mask)` (cdecl).
pub const IS_PRESS_BUTTON: u32 = 0x0063_0cf0;
/// `g_EncounterKey` (`0x00ce82d0`).
pub const ENCOUNTER_KEY: Global<u32> = Global::new(0x00ce_82d0);
/// `iggyDataValueSet_00a28d70(dst, src)` (cdecl).
pub const DATA_VALUE_SET: u32 = 0x00a2_8d70;
/// `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// The one-byte object `graphicInitialize` creates (`0x00ce63c4`).
pub const ONE_BYTE_2: Global<u32> = Global::new(0x00ce_63c4);

/// `FFEscMenu_addHandler_00702570(this; i)` (thiscall, `ecx` passed on):
/// `IggyMenuHandlers_addCall(this; 0x0068e560, &entries[i])`'s `eax`.
pub fn esc_add_handler(h: &mut impl Host, this: u32, i: u32) -> u32 {
    let entry = h.rebase(ESC_ENTRIES).wrapping_add(i.wrapping_mul(24));
    let f = h.rebase(ESC_HANDLER);
    h.call_sym(ADD_CALL, Conv::Thiscall(this), &[f, entry])
}

/// `ClassDynamicGeometry_applyBuffer_006a46c0(this)` (thiscall, slot 7 of
/// `ClassDynamicGeometry_vftable` and `VFXDynamicGeometry_vftable`): the
/// mesh (`+0xc`) gets the buffer `+0x14` (or its own first word + 0x30
/// when that is 0) at `+0x10` and `+0x10` at `+0x18`; returns 0.
pub fn geometry_apply_buffer(h: &mut impl Mem, this: u32) -> u32 {
    let mut buffer = h.read::<u32>(this + 0x14);
    let mesh = h.read::<u32>(this + 0xc);
    if buffer == 0 {
        buffer = h.read::<u32>(mesh).wrapping_add(0x30);
    }
    h.write(mesh.wrapping_add(0x10), buffer);
    let mesh = h.read::<u32>(this + 0xc);
    let v = h.read::<u32>(this + 0x10);
    h.write(mesh.wrapping_add(0x18), v);
    0
}

/// `IggyMenu_Draw_00a28eb0(this)` (thiscall): when enabled (`+0x70`) with
/// a draw argument (`+0x68`) and a player (`+0x6c`),
/// `IggyPlayer_QueueDraw(player; arg)`'s `eax`, else 0.
pub fn iggy_menu_draw(h: &mut impl Host, this: u32) -> u32 {
    if h.read::<u8>(this + 0x70) == 0 {
        return 0;
    }
    let arg = h.read::<u32>(this + 0x68);
    if arg == 0 {
        return 0;
    }
    let player = h.read::<u32>(this + 0x6c);
    if player == 0 {
        return 0;
    }
    h.call_sym(QUEUE_DRAW, Conv::Thiscall(player), &[arg])
}

/// `PCluster_createInRoot_006f7a00(name)` (`ret 4`, `ecx` passed on):
/// `Phyre_PCluster_create(name, 0x00ccbe88, 0)` (callee pops, `ecx`
/// untouched on the way: `tools/check-thisarg.py` finds it read) less 4
/// (the object before it), or 0.
pub fn cluster_create_in_root(h: &mut impl Host, ecx: u32, name: u32) -> u32 {
    let root = h.rebase(CLUSTER_ROOT);
    let r = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(ecx), &[name, root, 0]);
    if r == 0 { 0 } else { r.wrapping_sub(4) }
}

/// `AsyncLoadManager_LoadClusterRequest_run_00691320(this)` (thiscall,
/// slot 1 of `AsyncLoadManager_LoadClusterRequest_vftable`):
/// `ClusterManager_loadPCluster(ClusterManager_GetInstance(); path)` (the
/// path pushed before the getter), then `done(cluster, user)` (cdecl,
/// `+8`, `+0xc`) when set; the last call's `eax`.
pub fn load_cluster_request_run(h: &mut impl Host, this: u32) -> u32 {
    let path = h.read::<u32>(this + 4);
    let cm = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
    let cluster = h.call_sym(LOAD_PCLUSTER, Conv::Thiscall(cm), &[path]);
    let done = h.read::<u32>(this + 8);
    if done == 0 {
        return cluster;
    }
    let user = h.read::<u32>(this + 0xc);
    h.call(done, Conv::Cdecl, &[cluster, user])
}

/// `MotionBlur_onShaderClusterLoaded_006c53e0(cluster, &args)` (cdecl,
/// stored by `MotionBlur_initialize`): `AsyncLoadManager_sGetInstance()`
/// (unused), `args[0]->+0x50 = cluster`, `0x006c5150(args[0]; args[1],
/// args[2])`'s `eax`.
pub fn motion_blur_on_loaded(h: &mut impl Host, cluster: u32, args: u32) -> u32 {
    h.call_sym(ASYNC_LOAD_MANAGER, Conv::Cdecl, &[]);
    let a = h.read::<u32>(args + 4);
    let o = h.read::<u32>(args);
    let b = h.read::<u32>(args + 8);
    h.write(o.wrapping_add(0x50), cluster);
    h.call_sym(MOTION_BLUR_ON_LOADED, Conv::Thiscall(o), &[a, b])
}

/// `graphicVideoPlayIfReady_007185e0()` (from `Sg_MainLoop`): the render
/// data of instance `(0x78, 0x5bc)`; when it exists and `0x00ccb9cc` is
/// set, `graphicTextureVideoPlay(1)`'s `eax`, else the render data.
pub fn video_play_if_ready(h: &mut impl Host) -> u32 {
    let data = h.call_sym(VFX_RENDER_DATA, Conv::Cdecl, &VIDEO_INSTANCE);
    if VIDEO_READY.get(h) == 0 || data == 0 {
        return data;
    }
    h.call_sym(TEXTURE_VIDEO_PLAY, Conv::Cdecl, &[1])
}

/// `FFEscMenu_playMenuSound(this; id)` (0x0068f9b0, thiscall): once per
/// frame (`+0x117`), `SndForcePlayClick(id)`. Callers read no result.
pub fn esc_play_menu_sound(h: &mut impl Host, this: u32, id: u32) {
    if h.read::<u8>(this + ESC_SOUND_PLAYED) != 0 {
        return;
    }
    h.call_sym(FORCE_PLAY_CLICK, Conv::Cdecl, &[id]);
    h.write(this + ESC_SOUND_PLAYED, 1u8);
}

/// `TextUtil_onTextUtilAssetLoaded(asset, &slot)` (0x006f63f0, cdecl):
/// `AsyncLoadManager_sGetInstance()` (unused), `text->assets[i] = asset`
/// (`slot` = `(text, i)`); after the second (`i + 1 == 2`), a jump to
/// `TextUtil_doOnAllTextUtilAssetsLoaded(text)`, whose `eax` it returns,
/// else `i + 1`.
pub fn text_on_asset_loaded(h: &mut impl Host, asset: u32, slot: u32) -> u32 {
    h.call_sym(ASYNC_LOAD_MANAGER, Conv::Cdecl, &[]);
    let text = h.read::<u32>(slot);
    let i = h.read::<u32>(slot + 4);
    h.write(text.wrapping_add(i.wrapping_mul(4)), asset);
    let next = i.wrapping_add(1);
    if next != 2 {
        return next;
    }
    h.call_sym(TEXT_ALL_LOADED, Conv::Thiscall(text), &[])
}

/// `IconPanel_create_006f77f0()` (from `PhyreScene_initScene`):
/// `g_IconPanel = SetFields_006f7700(operator_new(0x40))` (0 when the
/// allocation fails); `al` = 1 over the last value's upper bytes.
pub fn icon_panel_create(h: &mut impl Host) -> u32 {
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x40]);
    if p == 0 {
        ICON_PANEL.set(h, 0);
        return 1;
    }
    let panel = h.call_sym(ICON_PANEL_CTOR, Conv::Thiscall(p), &[]);
    ICON_PANEL.set(h, panel);
    (panel & !0xff) | 1
}

/// `PhyreScene_chrLoadCountdown_00782fa0()` (from
/// `PhyreScene_doOnCharacterAllAssetsLoaded`): while the countdown is not
/// negative, counts it down; when it goes negative, `fprintf(stderr,
/// "Something wrong with PS3 battle character loading!!!\n")` (the message
/// pushed before `__iob_func`). Its caller reads no result.
pub fn chr_load_countdown(h: &mut impl Host) {
    let n = CHR_LOAD_COUNT.get(h).cast_signed();
    if n < 0 {
        return;
    }
    let n = n.wrapping_sub(1);
    CHR_LOAD_COUNT.set(h, n.cast_unsigned());
    if n >= 0 {
        return;
    }
    let message = h.rebase(CHR_LOAD_MESSAGE);
    let iob = IOB_FUNC.get(h);
    let files = h.call(iob, Conv::Cdecl, &[]);
    let fprintf = FPRINTF.get(h);
    h.call(fprintf, Conv::Cdecl, &[files.wrapping_add(0x40), message]);
}

/// `TextUtil_changeLine(this; i, s)` (0x006f6070, thiscall): for `i` below
/// the line count (`+0x74`, top bit masked, unsigned),
/// `Phyre_PString_Assign(&lines[i]; s)` (`+0x78`) and the dirty byte; the
/// assignment's `eax`, else the masked count.
pub fn text_change_line(h: &mut impl Host, this: u32, i: u32, s: u32) -> u32 {
    let count = h.read::<u32>(this + 0x74) & 0x7fff_ffff;
    if i >= count {
        return count;
    }
    let lines = h.read::<u32>(this + 0x78);
    let line = lines.wrapping_add(i.wrapping_mul(4));
    let r = h.call_sym(PSTRING_ASSIGN, Conv::Thiscall(line), &[s]);
    h.write(this + TEXT_TITLE_DIRTY, 1u8);
    r
}

/// The create-once singletons (`IggyMenuManager_createInstance`,
/// `RemapButtonIcon_sCreatInstance`, `PostEffect_createInstance_006c5790`):
/// unless the global is set, `ctor(operator_new(size))` into it (0 when
/// the allocation fails). Callers read no result.
pub fn create_instance(h: &mut impl Host, (global, size, ctor): (Global<u32>, u32, u32)) {
    if global.get(h) != 0 {
        return;
    }
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[size]);
    let v = if p == 0 {
        0
    } else {
        h.call_sym(ctor, Conv::Thiscall(p), &[])
    };
    global.set(h, v);
}

/// `IconPanel_setVisible(this; on)` (0x006f7940, thiscall): each of the
/// seven icons' `IggyMenu_setEnable(icon; on)`, then the byte at `+0x3f`.
/// Callers read no result.
pub fn icon_panel_set_visible(h: &mut impl Host, this: u32, on: u32) {
    for i in 0..7 {
        let icon = h.read::<u32>(this + 4 * i);
        if icon != 0 {
            h.call_sym(IGGY_SET_ENABLE, Conv::Thiscall(icon), &[on]);
        }
    }
    h.write(this + 0x3f, on as u8);
}

/// `IconPanel_hideAllIcons(this)` (0x006f78d0, thiscall): each of the
/// seven icons destroyed (`IggyMenu_dtor_IggyMenu`, `operator_delete`) and
/// its slot cleared. Callers read no result.
pub fn icon_panel_hide_all(h: &mut impl Host, this: u32) {
    for i in 0..7 {
        let icon = h.read::<u32>(this + 4 * i);
        if icon != 0 {
            h.call_sym(IGGY_MENU_DTOR, Conv::Thiscall(icon), &[]);
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[icon]);
            h.write(this + 4 * i, 0u32);
        }
    }
}

/// `MenuDynamicGeometry_resetMaterials(this)` (0x006a2c60, thiscall): the
/// material array (`+0x14`): with more than one entry, the first entry's
/// word cleared (`eax` its array); with one, the inline word at `+4`
/// cleared (`eax` its address); with none, 0.
pub fn reset_materials(h: &mut impl Mem, this: u32) -> u32 {
    let arr = h.read::<u32>(this + 0x14);
    let n = h.read::<u32>(arr) & 0x7fff_ffff;
    if n == 0 {
        return 0;
    }
    if n > 1 {
        let data = h.read::<u32>(arr.wrapping_add(4));
        h.write(data, 0u32);
        return data;
    }
    let slot = arr.wrapping_add(4);
    h.write(slot, 0u32);
    slot
}

/// `BrokenScreenEffect_getEntryWord_006d5190(this; i, k)` (thiscall):
/// word 0 (`k` 1) or 1 (`k` 2) of entry `i` of the table at `+0x38`, else
/// 0.
pub fn broken_screen_entry_word(h: &impl Mem, this: u32, i: u32, k: u32) -> u32 {
    let word = match k {
        1 => 0,
        2 => 4,
        _ => return 0,
    };
    let table = h.read::<u32>(this + 0x38);
    let entry = h.read::<u32>(table.wrapping_add(i.wrapping_mul(4)));
    h.read(entry.wrapping_add(word))
}

/// `CheckEncounterKey()` (0x006f75c0): `g_EncounterKey` =
/// `inputIsPressButton(0x400000) != 0`; returns 1 when pressed, else the
/// (cleared) global.
pub fn check_encounter_key(h: &mut impl Host) -> u32 {
    ENCOUNTER_KEY.set(h, 0);
    if h.call_sym(IS_PRESS_BUTTON, Conv::Cdecl, &[0x40_0000]) != 0 {
        ENCOUNTER_KEY.set(h, 1);
        return 1;
    }
    ENCOUNTER_KEY.get(h)
}

/// `LocString_ToValue_0068e270(this; out)` (thiscall):
/// `iggyDataValueSet(this + 0x100, this)`, then the 16 bytes at
/// `this + 0x100` copied to `out`; returns `out`.
pub fn loc_string_to_value(h: &mut impl Host, this: u32, out: u32) -> u32 {
    let value = this + 0x100;
    h.call_sym(DATA_VALUE_SET, Conv::Cdecl, &[value, this]);
    for k in 0..4 {
        let w = h.read::<u32>(value + 4 * k);
        h.write(out.wrapping_add(4 * k), w);
    }
    out
}

/// `PArray_destroy_006b01f0` (stride `0x20`) / `_006b0230` (`0x40`)
/// (cdecl `(array, n)`): unless the array is null, each element's virtual
/// slot 0 (`(elem; 0)`, the deleting destructor without the delete), then
/// `Phyre_PhyreFree(array)`. Callers read no result.
pub fn parray_destroy(h: &mut impl Host, array: u32, n: u32, stride: u32) {
    if array == 0 {
        return;
    }
    let mut e = array;
    for _ in 0..n {
        h.call_virtual(e, 0, &[0]);
        e = e.wrapping_add(stride);
    }
    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[array]);
}

/// `graphicInitialize_allocOnce2_006e56a0()`: unless `0x00ce63c4` is
/// set, `operator_new(1)` with its byte cleared into it (0 when the
/// allocation fails). Its caller reads no result.
pub fn alloc_once_2(h: &mut impl Host) {
    if ONE_BYTE_2.get(h) != 0 {
        return;
    }
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[1]);
    if p != 0 {
        h.write(p, 0u8);
    }
    ONE_BYTE_2.set(h, p);
}

/// The import slot of `_IggyValueRefFromTempRef@12`.
pub const IGGY_REF_FROM_TEMP: Global<u32> = Global::new(0x00b0_c828);
/// The import slot of `_IggyValuePathFromRef@12`.
pub const IGGY_PATH_FROM_REF: Global<u32> = Global::new(0x00b0_c82c);
/// `RemapButtonIcon_remapUIButtonData(this; b, c, d, e)` (thiscall).
pub const REMAP_UI_BUTTON_DATA: u32 = 0x006e_7300;
/// `RemapButtonIcon_remapMessgeInputData(this; a, c, d, e)` (thiscall).
pub const REMAP_MESSAGE_INPUT_DATA: u32 = 0x006e_7050;
/// `MenuManager_SwapMenuBufferChain(this)` (thiscall).
pub const SWAP_MENU_BUFFER_CHAIN: u32 = 0x0067_e7f0;
/// `MenuManager_SetUpAllMenuInstance(this)` (thiscall).
pub const SET_UP_ALL_MENU_INSTANCE: u32 = 0x0067_e6c0;
/// The `0x20`-byte map node maker `0x006a0400` (thiscall head).
pub const MAP_NODE_NEW: u32 = 0x006a_0400;

/// `IconPanel_hideIcon(this; i)` (0x006f7900, thiscall): icon `i` (below
/// 7, signed) destroyed and its slot cleared, as `IconPanel_hideAllIcons`
/// does for each. Callers read no result.
pub fn icon_panel_hide_icon(h: &mut impl Host, this: u32, i: u32) {
    if i.cast_signed() >= 7 {
        return;
    }
    let slot = this.wrapping_add(i.wrapping_mul(4));
    let icon = h.read::<u32>(slot);
    if icon != 0 {
        h.call_sym(IGGY_MENU_DTOR, Conv::Thiscall(icon), &[]);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[icon]);
        h.write(slot, 0u32);
    }
}

/// `IggyValueRef_create_00a288b0(this; player, temp)` (thiscall): marks
/// the reference live (`+0` = 1), keeps the player (`+0x38`), takes a
/// reference from the temporary one (`IggyValueRefFromTempRef(player,
/// temp, 0)`, kept at `+4`) and makes its path at `+8`
/// (`IggyValuePathFromRef(this + 8, player, ref)`); returns `this`.
pub fn iggy_value_ref_create(h: &mut impl Host, this: u32, player: u32, temp: u32) -> u32 {
    h.write(this, 1u8);
    h.write(this + 0x38, player);
    let f = IGGY_REF_FROM_TEMP.get(h);
    let r = h.call(f, Conv::Stdcall, &[player, temp, 0]);
    let player = h.read::<u32>(this + 0x38);
    h.write(this + 4, r);
    let f = IGGY_PATH_FROM_REF.get(h);
    h.call(f, Conv::Stdcall, &[this + 8, player, r]);
    this
}

/// `IggyMenuHandlers_flush_00a28ff0(this)` (thiscall, slot 0 of
/// `FFIggyPauseMenu_vftable` too): each of the five pending handler
/// records (`+4`, 0x14 bytes: flag, method, arg, this, invoker) run as
/// `invoker(method, arg, this)` (cdecl) and cleared. Callers read no
/// result.
pub fn iggy_handlers_flush(h: &mut impl Host, this: u32) {
    for k in 0..5 {
        let rec = this + 4 + 0x14 * k;
        if h.read::<u8>(rec) == 0 {
            continue;
        }
        let c = h.read::<u32>(rec + 0xc);
        let invoker = h.read::<u32>(rec + 0x10);
        let b = h.read::<u32>(rec + 8);
        let a = h.read::<u32>(rec + 4);
        h.call(invoker, Conv::Cdecl, &[a, b, c]);
        h.write(rec, 0u8);
    }
}

/// `RemapButtonIcon_remapButtonIcon(this; a, b, c, d, e)` (0x006e69e0,
/// thiscall): `RemapButtonIcon_remapUIButtonData(this; b, c, d, e)`, then
/// `RemapButtonIcon_remapMessgeInputData(this; a, c, d, e)`'s `eax`.
pub fn remap_button_icon(h: &mut impl Host, this: u32, args: [u32; 5]) -> u32 {
    let [a, b, c, d, e] = args;
    h.call_sym(REMAP_UI_BUTTON_DATA, Conv::Thiscall(this), &[b, c, d, e]);
    h.call_sym(
        REMAP_MESSAGE_INPUT_DATA,
        Conv::Thiscall(this),
        &[a, c, d, e],
    )
}

/// `MenuManager_UpdateMenu(this)` (0x0067faa0, thiscall): with a swap
/// pending (`+0x5b9c`), `MenuManager_SwapMenuBufferChain` and the ready
/// byte `+0x5b9d`; else, unless the scene's bytes `+0xfa9d` or `+0xfa9c`
/// are set, a jump to `MenuManager_SetUpAllMenuInstance`. Returns the
/// call's `eax`, or the scene pointer when nothing runs.
pub fn menu_update(h: &mut impl Host, this: u32) -> u32 {
    if h.read::<u8>(this + 0x5b9c) != 0 {
        let r = h.call_sym(SWAP_MENU_BUFFER_CHAIN, Conv::Thiscall(this), &[]);
        h.write(this + 0x5b9d, 1u8);
        return r;
    }
    let scene = crate::misc::PHYRE_SCENE.get(h);
    if h.read::<u8>(scene.wrapping_add(0xfa9d)) != 0
        || h.read::<u8>(scene.wrapping_add(0xfa9c)) != 0
    {
        return scene;
    }
    h.call_sym(SET_UP_ALL_MENU_INSTANCE, Conv::Thiscall(this), &[])
}

/// `MenuManager_GetMenuRecord_0067b990(this; i)` (thiscall): 0 until the
/// menus are ready (`+0x5b9d`); else the first word of menu `i`'s record,
/// from the table at `+0x15f0` or, with `+0x5b7c` set, `+0x4248`.
pub fn menu_record(h: &impl Mem, this: u32, i: u32) -> u32 {
    if h.read::<u8>(this + 0x5b9d) == 0 {
        return 0;
    }
    let table = if h.read::<u32>(this + crate::misc::MENU_ALT) == 0 {
        0x15f0
    } else {
        0x4248
    };
    let rec = h.read::<u32>(this.wrapping_add(table).wrapping_add(i.wrapping_mul(4)));
    h.read(rec)
}

/// `IggyMenuManager_removeMenu_00a26c50(this; menu)` (thiscall, from
/// `IggyMenu_dtor_IggyMenu`): the menu's slot in the list (`+8` count, top
/// bit masked; `+0xc` the slots) cleared, unless the menu is null or not
/// there. Its caller reads no result.
pub fn iggy_manager_remove_menu(h: &mut impl Mem, this: u32, menu: u32) {
    if menu == 0 {
        return;
    }
    let n = h.read::<u32>(this + 8) & 0x7fff_ffff;
    if n == 0 {
        return;
    }
    let slots = h.read::<u32>(this + 0xc);
    for i in 0..n {
        let at = slots.wrapping_add(4 * i);
        if h.read::<u32>(at) == menu {
            h.write(at, 0u32);
            return;
        }
    }
}

/// `TreeHead_copy_0069a1e0(this)` (thiscall): a `0x18`-byte node from
/// `operator_new` (`std::_Xbad_alloc` called when it fails) whose three
/// links are `*this` (each read again); returns it.
pub fn tree_head_copy(h: &mut impl Host, this: u32) -> u32 {
    let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x18]);
    if p == 0 {
        let f = crate::movie::XBAD_ALLOC.get(h);
        h.call(f, Conv::Cdecl, &[]);
    }
    let v = h.read::<u32>(this);
    h.write(p, v);
    for k in [4u32, 8] {
        if p.wrapping_add(k) != 0 {
            let v = h.read::<u32>(this);
            h.write(p.wrapping_add(k), v);
        }
    }
    p
}

/// `TreeNode_newWithKey_0069c580(this; key)` (thiscall, `ecx` passed on
/// to `0x006a0400`): a new map node, its word at `+0xc` cleared and the
/// 16-byte key copied to `+0x10`; returns the node.
pub fn tree_node_new_with_key(h: &mut impl Host, this: u32, key: u32) -> u32 {
    let node = h.call_sym(MAP_NODE_NEW, Conv::Thiscall(this), &[]);
    h.write(node.wrapping_add(0xc), 0u16);
    let dst = node.wrapping_add(0x10);
    if dst != 0 {
        for k in 0..4 {
            let w = h.read::<u32>(key.wrapping_add(4 * k));
            h.write(dst.wrapping_add(4 * k), w);
        }
    }
    node
}

/// `IggyMenu`'s vtable after its destructor ran (`0x00b78c54`, the base
/// class's).
pub const IGGY_MENU_BASE_VTABLE: u32 = 0x00b7_8c54;
/// The player release `0x00a3d720` (thiscall player; arg).
pub const PLAYER_RELEASE: u32 = 0x00a3_d720;
/// The player destructor thunk `0x00a3d4d0` (thiscall).
pub const PLAYER_DTOR: u32 = 0x00a3_d4d0;
/// `iSfxPause(id)` (cdecl).
pub const SFX_PAUSE: u32 = 0x0067_a380;
/// `iVoiceSetPaused(flag)` (cdecl).
pub const VOICE_SET_PAUSED: u32 = 0x0067_a750;
/// `graphicVideoPause()`.
pub const VIDEO_PAUSE: u32 = 0x0064_5de0;
/// `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `TextUtil_resize(text; w, h)` (thiscall).
pub const TEXT_RESIZE: u32 = 0x006f_6670;
/// `TextUtil_setPosition_006f6910(text; f32 x, f32 y)` (thiscall).
pub const TEXT_SET_POSITION: u32 = 0x006f_6910;
/// The video text's position (`.rdata` `0x00b92268`, `0x00b4a64c`: 2.0,
/// 17.0).
pub const VIDEO_TEXT_X: Global<u32> = Global::new(0x00b9_2268);
/// See [`VIDEO_TEXT_X`].
pub const VIDEO_TEXT_Y: Global<u32> = Global::new(0x00b4_a64c);
/// `graphicGetLocTextWithLength(id, dst, n)` (cdecl).
pub const LOC_TEXT: u32 = 0x0064_12f0;
/// The import slot of `sprintf`.
pub const SPRINTF: Global<u32> = Global::new(0x00b0_c3dc);
/// An empty string (`.rdata` `0x00b8fbeb`).
pub const EMPTY: u32 = 0x00b8_fbeb;
/// The escape menu's dialog (`0x00ccc870`).
pub const ESC_DIALOG: Global<u32> = Global::new(0x00cc_c870);
/// `SaveDataGetLoc_00647f30(id, dst)` (cdecl).
pub const SAVE_DATA_LOC: u32 = 0x0064_7f30;
/// The text converter `0x006e96b0` (cdecl src, dst, n).
pub const TEXT_CONVERT: u32 = 0x006e_96b0;
/// `IggyMenu_callIggyAS3External(menu; 0, name, n, values, 1)` (thiscall).
pub const CALL_AS3: u32 = 0x00a2_8ba0;
/// The save menu's text buffers: raw (`0x00ceb460`), converted
/// (`0x00ceb260`), the Iggy value (`0x00ceb250`).
pub const SAVE_TEXT_RAW: u32 = 0x00ce_b460;
/// See [`SAVE_TEXT_RAW`].
pub const SAVE_TEXT: u32 = 0x00ce_b260;
/// See [`SAVE_TEXT_RAW`].
pub const SAVE_TEXT_VALUE: u32 = 0x00ce_b250;
/// `"showPopUpStd"` (`.rdata`).
pub const SHOW_POPUP_STD: u32 = 0x00b4_c548;

/// `IggyMenu_dtor_IggyMenu(this)` (0x00a28a40, thiscall): the base
/// vtable; with a player (`+0x6c`), the menu removed from
/// `g_IggyMenuManager`, the player released (`+0x68`), destroyed and
/// freed, and `+0x6c` cleared. Callers read no result.
pub fn iggy_menu_dtor(h: &mut impl Host, this: u32) {
    let vtable = h.rebase(IGGY_MENU_BASE_VTABLE);
    let player = h.read::<u32>(this + 0x6c);
    h.write(this, vtable);
    if player == 0 {
        return;
    }
    let mgr = IGGY_MENU_MANAGER.get(h);
    h.call_sym(0x00a2_6c50, Conv::Thiscall(mgr), &[this]);
    let arg = h.read::<u32>(this + 0x68);
    let player = h.read::<u32>(this + 0x6c);
    h.call_sym(PLAYER_RELEASE, Conv::Thiscall(player), &[arg]);
    let player = h.read::<u32>(this + 0x6c);
    if player != 0 {
        h.call_sym(PLAYER_DTOR, Conv::Thiscall(player), &[]);
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[player]);
    }
    h.write(this + 0x6c, 0u32);
}

/// `FFEscMenu_unpauseAudioVideo(this)` (0x0068ff60, thiscall): what the
/// escape menu paused, resumed: sound effects (`+0x116`,
/// `iSfxPause(0)`), voice (`+0x115`, `iVoiceSetPaused(0)`), video
/// (`+0x114`, `graphicVideoPause()`), each flag cleared. Callers read no
/// result.
pub fn esc_unpause(h: &mut impl Host, this: u32) {
    if h.read::<u8>(this + 0x116) != 0 {
        h.call_sym(SFX_PAUSE, Conv::Cdecl, &[0]);
        h.write(this + 0x116, 0u8);
    }
    if h.read::<u8>(this + 0x115) != 0 {
        h.call_sym(VOICE_SET_PAUSED, Conv::Cdecl, &[0]);
        h.write(this + 0x115, 0u8);
    }
    if h.read::<u8>(this + 0x114) != 0 {
        h.call_sym(VIDEO_PAUSE, Conv::Cdecl, &[]);
        h.write(this + 0x114, 0u8);
    }
}

/// `PArray_copyNew_006affc0` / `_006b0010` (cdecl `(n, src)`): `n` words
/// from `src` into a new `Phyre_PhyreMalloc_0042fb80(4n, 4)` block (each
/// to a non-null slot, `src` advancing only then); the block, or 0 for
/// `n` 0.
pub fn parray_copy_new(h: &mut impl Host, n: u32, src: u32) -> u32 {
    if n == 0 {
        return 0;
    }
    let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[n.wrapping_mul(4), 4]);
    if p == 0 {
        return 0;
    }
    let mut s = src;
    for k in 0..n {
        let d = p.wrapping_add(4 * k);
        if d != 0 {
            let w = h.read::<u32>(s);
            h.write(d, w);
            s = s.wrapping_add(4);
        }
    }
    p
}

/// `VideoText_setSize_006dded0(this; w, h)` (thiscall, from
/// `PhyreScene_resize`): the size kept (`+0x158`, `+0x15c`); when the
/// bytes `+4` and `+5` are set, `TextUtil_resize(text; w, h)` and
/// `TextUtil_setPosition_006f6910(text; 2.0, 17.0)` (`.rdata` floats
/// through `fld`/`fstp`), the resize's `eax` (the setter leaves it); else
/// `w`.
pub fn video_text_set_size(h: &mut impl Host, this: u32, w: u32, hh: u32) -> u32 {
    h.write(this + 0x158, w);
    h.write(this + 0x15c, hh);
    if h.read::<u8>(this + 4) == 0 || h.read::<u8>(this + 5) == 0 {
        return w;
    }
    let text = h.read::<u32>(this);
    let r = h.call_sym(TEXT_RESIZE, Conv::Thiscall(text), &[w, hh]);
    let y = ffx_bridge::x87::round_trip(f32::from_bits(VIDEO_TEXT_Y.get(h))).to_bits();
    let text = h.read::<u32>(this);
    let x = ffx_bridge::x87::round_trip(f32::from_bits(VIDEO_TEXT_X.get(h))).to_bits();
    h.call_sym(TEXT_SET_POSITION, Conv::Thiscall(text), &[x, y]);
    r
}

/// `FFEscMenu_dialogText_0068e560(id)` (cdecl, the handler
/// `FFEscMenu_addHandler_00702570` registers): the dialog's text
/// (`+0x131`, 0x100 bytes) from `graphicGetLocTextWithLength(*id, ...)`,
/// or `sprintf(text, "")` for a null `id`; its kind (`+0x234`) = 5; the
/// call's `eax`.
pub fn esc_dialog_text(h: &mut impl Host, id: u32) -> u32 {
    let dialog = ESC_DIALOG.get(h);
    let text = dialog.wrapping_add(0x131);
    let r = if id == 0 {
        let empty = h.rebase(EMPTY);
        let f = SPRINTF.get(h);
        h.call(f, Conv::Cdecl, &[text, empty])
    } else {
        let v = h.read::<u32>(id);
        h.call_sym(LOC_TEXT, Conv::Cdecl, &[v, text, 0x100])
    };
    h.write(dialog.wrapping_add(0x234), 5u32);
    r
}

/// `SaveDataMachine_showPopUp_00702e90(id)` (cdecl): the save text `id`
/// (`SaveDataGetLoc_00647f30`) converted (`0x006e96b0`, 0x200 bytes) into
/// an Iggy string value, then `IggyMenu_callIggyAS3External(save menu; 0,
/// "showPopUpStd", 1, value, 1)`'s `eax`.
pub fn save_show_popup(h: &mut impl Host, id: u32) -> u32 {
    let raw = h.rebase(SAVE_TEXT_RAW);
    let text = h.rebase(SAVE_TEXT);
    let value = h.rebase(SAVE_TEXT_VALUE);
    h.call_sym(SAVE_DATA_LOC, Conv::Cdecl, &[id, raw]);
    h.call_sym(TEXT_CONVERT, Conv::Cdecl, &[raw, text, 0x200]);
    h.call_sym(DATA_VALUE_SET, Conv::Cdecl, &[value, text]);
    let menu = crate::misc::SAVE_MENU.get(h);
    let name = h.rebase(SHOW_POPUP_STD);
    h.call_sym(CALL_AS3, Conv::Thiscall(menu), &[0, name, 1, value, 1])
}

/// The import slots of `_IggyPlayerCreateFastNameUTF8@12`,
/// `_IggyPlayerCallFunctionRS@20`, `_IggyPlayerCallbackResultPath@4` and
/// `_IggyValueSetStringUTF8RS@20`, and `Sleep`.
pub const IGGY_CREATE_FAST_NAME: Global<u32> = Global::new(0x00b0_c850);
/// See [`IGGY_CREATE_FAST_NAME`].
pub const IGGY_CALL_FUNCTION: Global<u32> = Global::new(0x00b0_c854);
/// See [`IGGY_CREATE_FAST_NAME`].
pub const IGGY_CALLBACK_RESULT_PATH: Global<u32> = Global::new(0x00b0_c818);
/// See [`IGGY_CREATE_FAST_NAME`].
pub const IGGY_SET_STRING: Global<u32> = Global::new(0x00b0_c81c);
/// See [`IGGY_CREATE_FAST_NAME`].
pub const SLEEP: Global<u32> = Global::new(0x00b0_c100);
/// `IggyMenuManager_ApplyViewport_00a265a0(mgr)` (thiscall).
pub const APPLY_VIEWPORT: u32 = 0x00a2_65a0;
/// `LocalizationManager_GetInstance()`.
pub const LOCALIZATION: u32 = 0x006d_b070;
/// `GetDerefU32_00a28d10(p)` (thiscall).
pub const DEREF: u32 = 0x00a2_8d10;
/// The language suffixes `"_kr"`, `"_ch"`, `"_s"` (`.rdata`).
pub const SUFFIX_KR: u32 = 0x00b4_b50c;
/// See [`SUFFIX_KR`].
pub const SUFFIX_CH: u32 = 0x00b4_b510;
/// See [`SUFFIX_KR`].
pub const SUFFIX_S: u32 = 0x00b4_b514;
/// The escape menu's audio preview handlers and their names: four
/// `(name slot, handler)` pairs, then `(name, handler)`.
pub const AUDIO_PREVIEW: [(u32, u32); 4] = [
    (0x00c3_9638, 0x006f_f5b0),
    (0x00c3_9650, 0x006f_f4f0),
    (0x00c3_9668, 0x006f_f530),
    (0x00c3_9680, 0x006f_f570),
];
/// The fifth handler's name (`.rdata`) and handler.
pub const AUDIO_PREVIEW_LAST: (u32, u32) = (0x00b4_c7c0, 0x006f_f5f0);
/// `IggyMenu_addCallback_00a28e90(menu; name, handler)` (thiscall).
pub const ADD_CALLBACK_MENU: u32 = 0x00a2_8e90;

/// A C string's length, as the inline `strlen` loops count it.
fn strlen(h: &impl Mem, s: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// `IggyMenu_callIggyAS3External(this; target, name, n, values, unused)`
/// (0x00a28ba0, thiscall): the player (`**(this + 0x6c)`); the name made
/// a fast name (`IggyPlayerCreateFastNameUTF8(player, name, strlen)`,
/// retried after `Sleep(5)` while it returns 0, `Sleep` read once), then
/// `IggyPlayerCallFunctionRS(player, target, fast, n, values)`'s `eax`.
pub fn iggy_call_as3(h: &mut impl Host, this: u32, args: [u32; 4]) -> u32 {
    let [target, name, n, values] = args;
    let holder = h.read::<u32>(this + 0x6c);
    let player = h.read::<u32>(holder);
    let sleep = SLEEP.get(h);
    let fast = loop {
        let len = strlen(h, name);
        let f = IGGY_CREATE_FAST_NAME.get(h);
        let r = h.call(f, Conv::Stdcall, &[player, name, len]);
        if r != 0 {
            break r;
        }
        h.call(sleep, Conv::Stdcall, &[5]);
    };
    let f = IGGY_CALL_FUNCTION.get(h);
    h.call(f, Conv::Stdcall, &[player, target, fast, n, values])
}

/// `IggyMenuManager_setViewportRect_00a26bd0(this; r0..r5)` (thiscall,
/// from `PostProcessManager_setNon16By9Viewport`): unless the six words
/// at `+0x18` already equal them, stores them and returns
/// `IggyMenuManager_ApplyViewport_00a265a0`'s `eax`; else the last word.
pub fn iggy_set_viewport_rect(h: &mut impl Host, this: u32, r: [u32; 6]) -> u32 {
    let same = (0..6).all(|k| h.read::<u32>(this + 0x18 + 4 * k) == r[k as usize]);
    if same {
        return r[5];
    }
    for (k, v) in (0u32..).zip(r) {
        h.write(this + 0x18 + 4 * k, v);
    }
    h.call_sym(APPLY_VIEWPORT, Conv::Thiscall(this), &[])
}

/// `IggyPlayer_resultLanguageSuffix_006e9510(callback)` (cdecl, the
/// result callback `IggyPlayerOpen*` installs): the language's suffix
/// (`LocalizationManager +4`: 9 `"_kr"`, 10 `"_ch"`, else `"_s"`) as the
/// callback's UTF-8 result (`IggyValueSetStringUTF8RS(
/// IggyPlayerCallbackResultPath(*callback), 0, 0, suffix, strlen)`'s
/// `eax`).
pub fn iggy_language_suffix(h: &mut impl Host, callback: u32) -> u32 {
    let loc = h.call_sym(LOCALIZATION, Conv::Cdecl, &[]);
    let suffix = match h.read::<u32>(loc.wrapping_add(4)) {
        9 => SUFFIX_KR,
        10 => SUFFIX_CH,
        _ => SUFFIX_S,
    };
    let suffix = h.rebase(suffix);
    let v = h.call_sym(DEREF, Conv::Thiscall(callback), &[]);
    let f = IGGY_CALLBACK_RESULT_PATH.get(h);
    let path = h.call(f, Conv::Stdcall, &[v]);
    let len = strlen(h, suffix);
    let f = IGGY_SET_STRING.get(h);
    h.call(f, Conv::Stdcall, &[path, 0, 0, suffix, len])
}

/// `FFEscMenu_registerAudioPreviewCallbacks(this)` (0x006ffa20,
/// thiscall): five `IggyMenu_addCallback_00a28e90(this; name, handler)`;
/// the last one's `eax`.
pub fn esc_register_audio_preview(h: &mut impl Host, this: u32) -> u32 {
    for (slot, handler) in AUDIO_PREVIEW {
        let name = h.read::<u32>(h.rebase(slot));
        let handler = h.rebase(handler);
        h.call_sym(ADD_CALLBACK_MENU, Conv::Thiscall(this), &[name, handler]);
    }
    let (name, handler) = AUDIO_PREVIEW_LAST;
    let name = h.rebase(name);
    let handler = h.rebase(handler);
    h.call_sym(ADD_CALLBACK_MENU, Conv::Thiscall(this), &[name, handler])
}

/// `IggyHeap_takeFree_00a39a30(heap)` (cdecl, from `opd_config_memory`
/// and `0x00a3b610`): the first block of the free ring (`heap + 0x10`,
/// links `+0x1c` next / `+0x20` prev) unlinked, linked in front of its
/// owner's used ring (`[block + 0x18] + 0x160`), marked (`+0x30` =
/// 0x60000000, `+0x14` = 0) and returned; 0 when the ring is empty.
pub fn iggy_heap_take_free(h: &mut impl Mem, heap: u32) -> u32 {
    let ring = heap.wrapping_add(0x10);
    let n = h.read::<u32>(ring + 0x1c);
    if n == ring {
        return 0;
    }
    let prev = h.read::<u32>(n + 0x20);
    let next = h.read::<u32>(n + 0x1c);
    let owner = h.read::<u32>(n + 0x18);
    h.write(prev.wrapping_add(0x1c), next);
    let next = h.read::<u32>(n + 0x1c);
    let prev = h.read::<u32>(n + 0x20);
    h.write(next.wrapping_add(0x20), prev);
    let used = owner.wrapping_add(0x160);
    h.write(n + 0x1c, used);
    let last = h.read::<u32>(used.wrapping_add(0x20));
    h.write(n + 0x20, last);
    h.write(last.wrapping_add(0x1c), n);
    let next = h.read::<u32>(n + 0x1c);
    h.write(next.wrapping_add(0x20), n);
    h.write(n + 0x30, 0x6000_0000u32);
    h.write(n + 0x14, 0u32);
    n
}
