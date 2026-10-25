//! Glue functions that hand callees the addresses of their own stack
//! words (M4 port round 2, list `h4-glue-other`): Iggy value arrays and
//! paths, `std::string` locals, D3D11 out-parameters, cursors.
//!
//! Each bridge takes `frame`, the address of a block laid out as the
//! original's frame from its lowest local up (`ebp - size`; where the
//! original passes the address of its own argument slot, the block runs
//! past the saved `ebp` and the return address to that slot too). In the
//! game the shim hands a `StackFrame`; the oracle tests place it at the
//! original's own frame address, so the callees see the same pointers. The
//! stack cookie's word is part of the block but never written: nothing
//! but the original's epilogue reads it.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

use crate::menu::{CALL_AS3, LOC_TEXT, SPRINTF, TEXT_CONVERT};

/// `_IggyValuePathMakeArrayRef@12` (`0x00b0c830`, import slot).
pub const IGGY_PATH_MAKE_ARRAY_REF: u32 = 0x00b0_c830;
/// `_IggyValueGetS32RS@16` (`0x00b0c838`, import slot).
pub const IGGY_VALUE_GET_S32: u32 = 0x00b0_c838;
/// `IggyGDrawSendWarning` (`0x00b0c874`, import slot; cdecl, varargs).
pub const GDRAW_SEND_WARNING: u32 = 0x00b0_c874;
/// `vfxIntTree_Find(this; &key)` (thiscall).
pub const INT_TREE_FIND: u32 = 0x0070_fd20;
/// `RenderTarget_release_00440ea0(this)` (thiscall).
pub const RENDER_TARGET_RELEASE: u32 = 0x0044_0ea0;
/// `ShaderParams_FindByName(shader; name)` (thiscall).
pub const SHADER_PARAMS_FIND: u32 = 0x0056_cbf0;
/// `ShaderParams_setValue_0056e670(this; param, &value)` (thiscall).
pub const SHADER_PARAMS_SET: u32 = 0x0056_e670;
/// The parameter name `Distortion_resetRenderTarget_006c24b0` clears
/// (`0x00b425d8`).
pub const DISTORTION_PARAM_NAME: u32 = 0x00b4_25d8;
/// `PClusterCursor_next_00464f30(this)` (thiscall, a cursor in the
/// caller's frame).
pub const CLUSTER_CURSOR_NEXT: u32 = 0x0046_4f30;
/// The class `PostProcessManager_findClusterData_00698a30`'s cursor walks
/// (`0x00ca37d0`).
pub const CLUSTER_CURSOR_CLASS: u32 = 0x00ca_37d0;
/// `Phyre_PFramework_PApplication_GetApplication()`.
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `g_PhyreScene`.
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// `g_RenderDelegationHelper` (`0x00ccca98`).
pub const RENDER_DELEGATION_HELPER: u32 = 0x00cc_ca98;
/// `RenderDelegationHelper_registerDelegation(this; record, size, a, f)`.
pub const REGISTER_DELEGATION: u32 = 0x006a_de20;
/// The callback `ScreenFadeDelegations_registerDelegation` registers
/// (`0x006ae5b0`).
pub const SCREEN_FADE_CALLBACK: u32 = 0x006a_e5b0;
/// The Iggy D3D11 `GDraw` state (`0x01a858b4`): `+4` the device context.
pub const GDRAW_STATE: u32 = 0x01a8_58b4;
/// Three null shader-resource views `GDraw` binds (`0x01a858b8`).
pub const GDRAW_NULL_VIEWS: u32 = 0x01a8_58b8;
/// The warnings `gdraw_MapBuffer_00a3cec0` sends: out of memory, other
/// failure, and their two shared strings.
pub const GDRAW_MAP_WARNINGS: [u32; 4] = [0x00b7_a4ec, 0x00b7_a50c, 0x00b6_868c, 0x00b7_a408];
/// `E_OUTOFMEMORY`.
pub const E_OUTOFMEMORY: u32 = 0x8007_000e;
/// `std::string::assign(this; src, pos, n)` (`0x00403a10`, thiscall).
pub const STRING_ASSIGN: u32 = 0x0040_3a10;
/// `StdString_moveCtor_0069c270(this; src, unused)`.
pub const STRING_MOVE_CTOR: u32 = 0x0069_c270;
/// `operator_delete(p)`.
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `crt_memset_thunk(p, c, n)`.
pub const MEMSET: u32 = 0x0094_964c;
/// `FmodFile_getDataDirectory_00679a10(key, buf, n)` (cdecl).
pub const DATA_DIRECTORY: u32 = 0x0067_9a10;
/// The key `getFullFilename` asks for (`0x00b4f19c`).
pub const DATA_DIRECTORY_KEY: u32 = 0x00b4_f19c;
/// `"%s%s"` (`0x00b40eb0`).
pub const FORMAT_TWO_STRINGS: u32 = 0x00b4_0eb0;

/// Where `IggyMenu_callFlashFunction_<n>` keeps its `n` values (4 words
/// each) below its `ebp`: `16 n` bytes, plus the cookie word from 3 on.
#[must_use]
pub const fn flash_values_below_ebp(n: u32) -> u32 {
    16 * n + if n >= 3 { 4 } else { 0 }
}

/// `IggyMenu_callFlashFunction_<n>(menu, name, values...)` (cdecl; `n`
/// `IggyDataValue`s of 4 words by value, `args` the address of the
/// caller's arguments): copies the values into its frame (`frame`, `4 n`
/// words) and returns `IggyMenu_callIggyAS3External(menu; 0, name, n,
/// values, 1)`.
pub fn iggy_call_flash(h: &mut impl Host, frame: u32, n: u32, args: u32) -> u32 {
    let menu: u32 = h.read(args);
    let name: u32 = h.read(args + 4);
    for i in 0..4 * n {
        let v: u32 = h.read(args + 8 + 4 * i);
        h.write(frame + 4 * i, v);
    }
    h.call_sym(CALL_AS3, Conv::Thiscall(menu), &[0, name, n, frame, 1])
}

/// Bytes of [`iggy_array_s32`]'s frame: the path (`0x18`), the saved
/// `ebp`, the return address, the argument.
pub const ARRAY_S32_FRAME: u32 = 0x24;

/// `IggyMenu_getArrayS32_00a28b20(this; i)` (thiscall, ret 4): an
/// array-element path off `this + 8` (`_IggyValuePathMakeArrayRef`), then
/// `_IggyValueGetS32RS(path, 0, 0, &i)`, which writes the value over the
/// argument slot (`frame + 0x20`); returns that slot.
pub fn iggy_array_s32(h: &mut impl Host, frame: u32, this: u32, i: u32) -> u32 {
    let slot = frame + 0x20;
    h.write(slot, i);
    let make: u32 = h.read(h.rebase(IGGY_PATH_MAKE_ARRAY_REF));
    h.call(make, Conv::Stdcall, &[frame, this.wrapping_add(8), i]);
    let get: u32 = h.read(h.rebase(IGGY_VALUE_GET_S32));
    h.call(get, Conv::Stdcall, &[frame, 0, 0, slot]);
    h.read(slot)
}

/// Bytes of [`fmod_tree_get_event`]'s frame: the result word, the saved
/// `ebp`, the return address, the key argument.
pub const TREE_EVENT_FRAME: u32 = 0x10;

/// `FmodEventTree_getEvent_00710290(this; key)` (thiscall, ret 4): with a
/// project (`*this`), finds the node of `key` in the tree at `this + 4`
/// (`vfxIntTree_Find(&key)`, the argument slot at `frame + 0xc`); with a
/// node, the project's method `+0x24(project, node +0x10's id, 0,
/// &result)` and returns the result word (`frame`, 0 before the call);
/// else 0.
pub fn fmod_tree_get_event(h: &mut impl Host, frame: u32, this: u32, key: u32) -> u32 {
    let project: u32 = h.read(this);
    h.write(frame, 0u32);
    if project == 0 {
        return 0;
    }
    let slot = frame + 0xc;
    h.write(slot, key);
    let node = h.call_sym(INT_TREE_FIND, Conv::Thiscall(this.wrapping_add(4)), &[slot]);
    if node == 0 || node.wrapping_add(0x10) == 0 {
        return 0;
    }
    let id: u32 = h.read(node.wrapping_add(0x10));
    let vtable: u32 = h.read(project);
    let f: u32 = h.read(vtable.wrapping_add(0x24));
    h.call(f, Conv::Stdcall, &[project, id, 0, frame]);
    h.read(frame)
}

/// Bytes of [`distortion_reset_target`]'s frame (the parameter value).
pub const DISTORTION_FRAME: u32 = 0x10;

/// `Distortion_resetRenderTarget_006c24b0(this)` (thiscall): releases the
/// target at `+0x1c`, clears `+0x28`/`+0x24`; with a shader at `+0x2c`
/// whose `DISTORTION_PARAM_NAME` parameter exists, sets it on
/// `(+0x30)+4` to the value `{0x42, 0, 0, 0}` in its frame; clears the
/// byte `+0x7f`. Returns the last callee's `eax` (the original leaves it).
pub fn distortion_reset_target(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let target: u32 = h.read(this.wrapping_add(0x1c));
    let mut eax = h.call_sym(RENDER_TARGET_RELEASE, Conv::Thiscall(target), &[]);
    h.write(this.wrapping_add(0x28), 0u32);
    h.write(this.wrapping_add(0x24), 0u32);
    let shader: u32 = h.read(this.wrapping_add(0x2c));
    if shader != 0 {
        eax = h.call_sym(
            SHADER_PARAMS_FIND,
            Conv::Thiscall(shader),
            &[h.rebase(DISTORTION_PARAM_NAME)],
        );
        if eax != 0 {
            let owner: u32 = h.read(this.wrapping_add(0x30));
            let params: u32 = h.read(owner.wrapping_add(4));
            h.write(frame, 0x42u32);
            h.write(frame + 0xc, 0u32);
            h.write(frame + 8, 0u32);
            h.write(frame + 4, 0u32);
            eax = h.call_sym(SHADER_PARAMS_SET, Conv::Thiscall(params), &[eax, frame]);
        }
    }
    h.write(this.wrapping_add(0x7f), 0u8);
    eax
}

/// Bytes of [`post_find_cluster_data`]'s frame (the cursor).
pub const CLUSTER_CURSOR_FRAME: u32 = 0x20;

/// `PostProcessManager_findClusterData_00698a30(this; i)` (thiscall, ret
/// 4): the cluster `this[1 + i]`; none gives 0. Else a cursor in its frame
/// `{0, 0, 0, 0, 0, list, first, CLUSTER_CURSOR_CLASS}` over the list at
/// the cluster's `+0x1c` (`first` = its head, or 0 when the list is
/// empty: the head points at itself), advanced once
/// (`PClusterCursor_next_00464f30`); if it found something (`+0xc`),
/// `+0x10` plus `+0`, else 0.
pub fn post_find_cluster_data(h: &mut impl Host, frame: u32, this: u32, i: u32) -> u32 {
    let cluster: u32 = h.read(this.wrapping_add(i.wrapping_mul(4)).wrapping_add(4));
    if cluster == 0 {
        return 0;
    }
    let list = cluster.wrapping_add(0x1c);
    let head: u32 = h.read(list);
    let first = if head == list { 0 } else { head };
    for k in 0..5 {
        h.write(frame + 4 * k, 0u32);
    }
    h.write(frame + 0x14, list);
    h.write(frame + 0x18, first);
    let class = h.rebase(CLUSTER_CURSOR_CLASS);
    h.write(frame + 0x1c, class);
    h.call_sym(CLUSTER_CURSOR_NEXT, Conv::Thiscall(frame), &[]);
    let found: u32 = h.read(frame + 0xc);
    if found == 0 {
        return 0;
    }
    let off: u32 = h.read(frame + 0x10);
    let base: u32 = h.read(frame);
    off.wrapping_add(base)
}

/// Bytes of [`screen_fade_register`]'s frame (the delegation record).
pub const FADE_RECORD_FRAME: u32 = 0x20;

/// `ScreenFadeDelegations_registerDelegation(r, g, b, a, x, y)` (cdecl;
/// the first four floats as bits): a record in its frame `{application +
/// 0x124, scene +0xdc ? that + 4 : 0, 0, r, g, b, a, x}` (each float
/// through `fld`/`fstp`), then
/// `RenderDelegationHelper_registerDelegation(helper; &record, 0x20, y,
/// SCREEN_FADE_CALLBACK)`'s `eax`.
pub fn screen_fade_register(
    h: &mut impl Host,
    frame: u32,
    colour: [u32; 4],
    x: u32,
    y: u32,
) -> u32 {
    let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
    h.write(frame, app.wrapping_add(0x124));
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let sub: u32 = h.read(scene.wrapping_add(0xdc));
    h.write(frame + 4, if sub == 0 { 0 } else { sub.wrapping_add(4) });
    let helper: u32 = h.read(h.rebase(RENDER_DELEGATION_HELPER));
    for (k, c) in colour.into_iter().enumerate() {
        let v = ffx_bridge::x87::round_trip(f32::from_bits(c)).to_bits();
        h.write(frame + 0xc + 4 * k as u32, v);
    }
    h.write(frame + 0x1c, x);
    h.write(frame + 8, 0u32);
    h.call_sym(
        REGISTER_DELEGATION,
        Conv::Thiscall(helper),
        &[frame, 0x20, y, h.rebase(SCREEN_FADE_CALLBACK)],
    )
}

/// Bytes of [`gdraw_map_buffer`]'s frame (`D3D11_MAPPED_SUBRESOURCE`).
pub const MAPPED_FRAME: u32 = 0xc;

/// The device context of the Iggy `GDraw` state.
fn gdraw_context(h: &impl Mem) -> u32 {
    let state: u32 = h.read(h.rebase(GDRAW_STATE));
    h.read(state.wrapping_add(4))
}

/// `gdraw_MapBuffer_00a3cec0(resource)` (cdecl): the context's `Map`
/// (slot 14: resource, 0, `WRITE_DISCARD`, 0, &mapped in its frame);
/// success returns `mapped.pData`, `E_OUTOFMEMORY` and other failures send
/// a `GDraw` warning (the code after the others) and return 0.
pub fn gdraw_map_buffer(h: &mut impl Host, frame: u32, resource: u32) -> u32 {
    let ctx = gdraw_context(h);
    let vtable: u32 = h.read(ctx);
    let map: u32 = h.read(vtable.wrapping_add(0x38));
    let hr = h.call(map, Conv::Stdcall, &[ctx, resource, 0, 4, 0, frame]);
    if (hr as i32) >= 0 {
        return h.read(frame);
    }
    let warn: u32 = h.read(h.rebase(GDRAW_SEND_WARNING));
    let [oom, other, file, func] = GDRAW_MAP_WARNINGS.map(|a| h.rebase(a));
    if hr == E_OUTOFMEMORY {
        h.call(warn, Conv::Cdecl, &[0, oom, file, func]);
    } else {
        h.call(warn, Conv::Cdecl, &[0, other, file, func, hr]);
    }
    0
}

/// Bytes of [`gdraw_clear_bindings`]'s frame (the null buffer and its
/// stride/offset word).
pub const CLEAR_BINDINGS_FRAME: u32 = 8;

/// `gdraw_clearBindings_00a3d280()` (cdecl): on the context,
/// `PSSetShaderResources(0, 3, GDRAW_NULL_VIEWS)` (slot 8),
/// `IASetVertexBuffers(0, 1, &null, &zero, &zero)` (slot 18, both words in
/// its frame), `IASetIndexBuffer(0, 0, 0)` (slot 19); returns that call's
/// `eax`.
pub fn gdraw_clear_bindings(h: &mut impl Host, frame: u32) -> u32 {
    let ctx = gdraw_context(h);
    let vtable: u32 = h.read(ctx);
    h.write(frame, 0u32);
    h.write(frame + 4, 0u32);
    let views = h.rebase(GDRAW_NULL_VIEWS);
    let f: u32 = h.read(vtable.wrapping_add(0x20));
    h.call(f, Conv::Stdcall, &[ctx, 0, 3, views]);
    let vtable: u32 = h.read(ctx);
    let f: u32 = h.read(vtable.wrapping_add(0x48));
    h.call(f, Conv::Stdcall, &[ctx, 0, 1, frame, frame + 4, frame + 4]);
    let vtable: u32 = h.read(ctx);
    let f: u32 = h.read(vtable.wrapping_add(0x4c));
    h.call(f, Conv::Stdcall, &[ctx, 0, 0, 0])
}

/// Bytes of [`loc_string_load`]'s frame (the text, then the cookie).
pub const LOC_STRING_FRAME: u32 = 0x104;

/// `LocString_Load_0068e180(this; id)` (thiscall, ret 4): the localized
/// text `id` into a 0x100-byte buffer in its frame
/// (`graphicGetLocTextWithLength`), converted into `this`
/// (`0x006e96b0(buf, this, 0x100)`); returns `this`.
pub fn loc_string_load(h: &mut impl Host, frame: u32, this: u32, id: u32) -> u32 {
    h.call_sym(LOC_TEXT, Conv::Cdecl, &[id, frame, 0x100]);
    h.call_sym(TEXT_CONVERT, Conv::Cdecl, &[frame, this, 0x100]);
    this
}

/// Bytes of [`full_filename`]'s frame (the directory, then the cookie).
pub const FULL_FILENAME_FRAME: u32 = 0x104;

/// `getFullFilename(out, name)` (cdecl): a zeroed 0x100-byte buffer in its
/// frame (byte 0, then `memset` of the other 0xff), the data directory
/// into it (`FmodFile_getDataDirectory_00679a10(key, buf, 0x100)`), then
/// `sprintf(out, "%s%s", buf, name)`'s `eax`.
pub fn full_filename(h: &mut impl Host, frame: u32, out: u32, name: u32) -> u32 {
    h.write(frame, 0u8);
    h.call_sym(MEMSET, Conv::Cdecl, &[frame + 1, 0, 0xff]);
    h.call_sym(
        DATA_DIRECTORY,
        Conv::Cdecl,
        &[h.rebase(DATA_DIRECTORY_KEY), frame, 0x100],
    );
    let sprintf = SPRINTF.get(h);
    let fmt = h.rebase(FORMAT_TWO_STRINGS);
    h.call(sprintf, Conv::Cdecl, &[out, fmt, frame, name])
}

/// Bytes of [`prototype_entry`]'s frame: a word, the `std::string`
/// (`+4`: 16 bytes of text or its pointer, `+0x14` size, `+0x18`
/// capacity), the value word (`+0x1c`), the cookie.
pub const PROTOTYPE_ENTRY_FRAME: u32 = 0x24;

/// `ClassLoadedCharacterTable_makeEntry_0069da20(out, name, value)`
/// (cdecl): a pair in its frame, the string assigned from `name`
/// (`std::string::assign(name, 0, -1)` on an empty one) and `*value`;
/// `StdString_moveCtor_0069c270(out; &string, 0)`; frees the string's
/// buffer when its capacity is 0x10 or more after that; returns `out`.
pub fn prototype_entry(h: &mut impl Host, frame: u32, out: u32, name: u32, value: u32) -> u32 {
    let s = frame + 4;
    h.write(frame, 0u32);
    h.write(s + 0x14, 0xfu32);
    h.write(s + 0x10, 0u32);
    h.write(s, 0u8);
    h.call_sym(STRING_ASSIGN, Conv::Thiscall(s), &[name, 0, u32::MAX]);
    let v: u32 = h.read(value);
    h.write(frame + 0x1c, v);
    h.call_sym(STRING_MOVE_CTOR, Conv::Thiscall(out), &[s, 0]);
    let cap: u32 = h.read(s + 0x14);
    if cap >= 0x10 {
        let p: u32 = h.read(s);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
    }
    out
}
