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
/// `Phyre_PParameterBuffer_setTextureChecked_0056e670(this; param, &value)` (thiscall).
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
    gdraw_map(h, frame, ctx, resource, 4)
}

/// `gdraw_mapResource_00a3c070(ctx, resource, discard)` (cdecl): as
/// [`gdraw_map_buffer`] on the context it is given, `WRITE_DISCARD` (4)
/// when the byte `discard` is set, else `WRITE_NO_OVERWRITE` (5).
pub fn gdraw_map_resource(
    h: &mut impl Host,
    frame: u32,
    ctx: u32,
    resource: u32,
    discard: u32,
) -> u32 {
    let kind = if discard as u8 == 0 { 5 } else { 4 };
    gdraw_map(h, frame, ctx, resource, kind)
}

/// `Map(ctx, resource, 0, kind, 0, &mapped)` and its warnings.
fn gdraw_map(h: &mut impl Host, frame: u32, ctx: u32, resource: u32, kind: u32) -> u32 {
    let vtable: u32 = h.read(ctx);
    let map: u32 = h.read(vtable.wrapping_add(0x38));
    let hr = h.call(map, Conv::Stdcall, &[ctx, resource, 0, kind, 0, frame]);
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

/// `ERRCHECK(result, file, line)` (cdecl).
pub const ERRCHECK: u32 = 0x0070_79d0;
/// `FMOD_ErrorString(result)` (cdecl).
pub const FMOD_ERROR_STRING: u32 = 0x0070_7590;
/// `printToOutput(prefix, fmt, ...)` (cdecl).
pub const PRINT_TO_OUTPUT: u32 = 0x0070_7b20;
/// `FMOD_Memory_GetStats(&current, &max, blocking)` (`0x00948cdc`, the
/// import's thunk; stdcall).
pub const FMOD_MEMORY_GET_STATS: u32 = 0x0094_8cdc;
/// `g_FmodMaxMemory` (`0x00cec17c`): the largest peak [`errcheck`] has
/// reported (signed).
pub const FMOD_MAX_MEMORY: u32 = 0x00ce_c17c;
/// `ERRCHECK`'s strings: the error's prefix and format, the memory
/// report's prefix and format.
pub const ERRCHECK_STRINGS: [u32; 4] = [0x00b4_c30c, 0x00b4_f140, 0x00b4_f190, 0x00b4_f178];
/// `vsprintf` (`0x00b0c430`, import slot).
pub const VSPRINTF: u32 = 0x00b0_c430;
/// `Phyre_PhyrePrintf(level, text)` (cdecl).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;

/// `ERRCHECK(result, file, line)` (cdecl; `local` its one local word,
/// `args` the address of its arguments, which it writes): a nonzero
/// `result` is printed (`printToOutput(prefix, fmt, result,
/// FMOD_ErrorString(result), file, line)`); then
/// `FMOD_Memory_GetStats(&local, &args[0], 0)` (both zeroed first) writes
/// FMOD's peak over the `result` slot; a peak above `g_FmodMaxMemory`
/// (signed) is printed and stored. Returns the peak (the slot, read
/// again after the print).
pub fn errcheck(h: &mut impl Host, local: u32, args: u32) -> u32 {
    let result: u32 = h.read(args);
    let [prefix, fmt, mem_prefix, mem_fmt] = ERRCHECK_STRINGS.map(|a| h.rebase(a));
    if result != 0 {
        let file: u32 = h.read(args + 4);
        let line: u32 = h.read(args + 8);
        let text = h.call_sym(FMOD_ERROR_STRING, Conv::Cdecl, &[result]);
        h.call_sym(
            PRINT_TO_OUTPUT,
            Conv::Cdecl,
            &[prefix, fmt, result, text, file, line],
        );
    }
    h.write(local, 0u32);
    h.write(args, 0u32);
    h.call_sym(FMOD_MEMORY_GET_STATS, Conv::Stdcall, &[local, args, 0]);
    let peak: u32 = h.read(args);
    let max_at = h.rebase(FMOD_MAX_MEMORY);
    let known: u32 = h.read(max_at);
    if (peak as i32) <= (known as i32) {
        return peak;
    }
    h.call_sym(PRINT_TO_OUTPUT, Conv::Cdecl, &[mem_prefix, mem_fmt, peak]);
    let peak: u32 = h.read(args);
    h.write(max_at, peak);
    peak
}

/// Bytes of [`print_to_output`]'s frame: the line (`0x200`), the text
/// (`0x200`), the cookie.
pub const PRINT_FRAME: u32 = 0x404;

/// `printToOutput(prefix, fmt, ...)` (cdecl; `args` the address of its
/// arguments): `vsprintf(text, fmt, &args[2])` into `frame + 0x200`,
/// `sprintf(line, "%s%s", prefix, text)` into `frame`, then
/// `Phyre_PhyrePrintf(2, line)`'s `eax`.
pub fn print_to_output(h: &mut impl Host, frame: u32, args: u32) -> u32 {
    let prefix: u32 = h.read(args);
    let fmt: u32 = h.read(args + 4);
    let text = frame + 0x200;
    let vsprintf: u32 = h.read(h.rebase(VSPRINTF));
    h.call(vsprintf, Conv::Cdecl, &[text, fmt, args + 8]);
    let sprintf = SPRINTF.get(h);
    let two = h.rebase(FORMAT_TWO_STRINGS);
    h.call(sprintf, Conv::Cdecl, &[frame, two, prefix, text]);
    h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, frame])
}

/// Bytes of [`sfx_bank_playing_events`]'s frame (the event info, then the
/// cookie).
pub const BANK_INFO_FRAME: u32 = 0x120;
/// The file name `SfxBank_getNumPlayingEvents` passes to `ERRCHECK`
/// (`0x00b4fe84`).
pub const SFX_BANK_FILE: u32 = 0x00b4_fe84;

/// `SfxBank_getNumPlayingEvents(this)` (thiscall): without a bank
/// (`*this`) 0; else the info block in its frame (words `+0` and
/// `+0x104..=+0x118` zeroed), the bank's method `+4(bank, &info)`
/// checked by `ERRCHECK(r, file, 0x74)`, and the info's `+0x114`.
pub fn sfx_bank_playing_events(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let bank: u32 = h.read(this);
    if bank == 0 {
        return 0;
    }
    h.write(frame, 0u32);
    for off in (0x104..=0x118).step_by(4) {
        h.write(frame + off, 0u32);
    }
    let vtable: u32 = h.read(bank);
    let f: u32 = h.read(vtable.wrapping_add(4));
    let r = h.call(f, Conv::Stdcall, &[bank, frame]);
    let file = h.rebase(SFX_BANK_FILE);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x74]);
    h.read(frame + 0x114)
}

/// The channel's state query (`0x00948ca6`, an FMOD import's thunk;
/// stdcall `(channel, &state)`).
pub const FMOD_GET_STATE: u32 = 0x0094_8ca6;
/// The channel's volume setter (`0x00948cb2`, an FMOD import's thunk;
/// stdcall `(channel, volume)`).
pub const FMOD_SET_VOLUME: u32 = 0x0094_8cb2;
/// The file name `FmodVoice` passes to `ERRCHECK` (`0x00b4f8c0`).
pub const FMOD_VOICE_FILE: u32 = 0x00b4_f8c0;
/// The divisor `FmodVoice_volumeDirect` scales by (`0x00b922e8`, f64).
pub const VOICE_VOLUME_DIVISOR: ffx_bridge::Global<f64> = ffx_bridge::Global::new(0x00b9_22e8);

/// `FmodVoice_volumeDirect(this; volume, keep)` (thiscall, ret 8; `frame`
/// its state word): with a channel (`+0x14`), its state (`ERRCHECK`, line
/// 0x31c); when bit 8 is set: `keep == 1` stores the volume at `+0x50`; a
/// mute flag (`+0x80 == 1`) makes it 0; the channel's volume becomes
/// `f32(+0x88 * (volume / divisor))` (x87, `ERRCHECK` line 0x326).
/// Returns 0. The state word starts as `this` (`push ecx`).
pub fn voice_volume_direct(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    volume: u32,
    keep: u32,
) -> u32 {
    // `push ecx` makes the state word: `this` until the query writes it
    h.write(frame, this);
    let channel: u32 = h.read(this.wrapping_add(0x14));
    if channel == 0 {
        return 0;
    }
    let file = h.rebase(FMOD_VOICE_FILE);
    let r = h.call_sym(FMOD_GET_STATE, Conv::Stdcall, &[channel, frame]);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x31c]);
    if h.read::<u8>(frame) & 8 == 0 {
        return 0;
    }
    let mut v = ffx_bridge::x87::load(volume);
    if keep == 1 {
        h.write(this.wrapping_add(0x50), ffx_bridge::x87::store(v));
    }
    if h.read::<u32>(this.wrapping_add(0x80)) == 1 {
        v = 0.0;
    }
    let scale = ffx_bridge::x87::load(h.read(this.wrapping_add(0x88)));
    let q = ffx_bridge::x87::div(v, VOICE_VOLUME_DIVISOR.get(h));
    let p = ffx_bridge::x87::store(ffx_bridge::x87::mul(scale, q));
    let channel: u32 = h.read(this.wrapping_add(0x14));
    let r = h.call_sym(FMOD_SET_VOLUME, Conv::Stdcall, &[channel, p]);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0x326]);
    0
}

/// `WaitForSingleObject` (`0x00b0c0f8`, import slot).
pub const WAIT_FOR_SINGLE_OBJECT: u32 = 0x00b0_c0f8;
/// `ReleaseMutex` (`0x00b0c1c0`, import slot).
pub const RELEASE_MUTEX: u32 = 0x00b0_c1c0;
/// `SfxEventTree_Find(tree; &id)` (thiscall).
pub const SFX_EVENT_TREE_FIND: u32 = 0x0070_ba70;
/// `FmodSfx_anyEventFlag_0070ed30(list)` (thiscall, `al`).
pub const SFX_ANY_EVENT_FLAG: u32 = 0x0070_ed30;

/// `FmodSfx_isPlaying(this; id)` (thiscall, ret 4; `frame` its saved
/// `this` word, then the saved `ebp`, the return address and the `id`
/// slot, whose address it passes): under the mutex `+0xcc`, the node of
/// `id` in the tree at `+0x64`; with one, 1 when one of its list's
/// (`+0x10`: count, `+8` the array) non-null events has its flag
/// (`FmodSfx_anyEventFlag_0070ed30`), else 0. Returns that in `al` over
/// `ReleaseMutex`'s `eax`.
pub fn sfx_is_playing(h: &mut impl Host, frame: u32, this: u32, id: u32) -> u32 {
    let wait: u32 = h.read(h.rebase(WAIT_FOR_SINGLE_OBJECT));
    let mutex: u32 = h.read(this.wrapping_add(0xcc));
    h.write(frame, this);
    h.call(wait, Conv::Stdcall, &[mutex, u32::MAX]);
    let slot = frame + 0xc;
    h.write(slot, id);
    let node = h.call_sym(
        SFX_EVENT_TREE_FIND,
        Conv::Thiscall(this.wrapping_add(0x64)),
        &[slot],
    );
    let mut playing = 0;
    if node != 0 && node.wrapping_add(0x10) != 0 {
        let list = node.wrapping_add(0x10);
        let mut i = 0u32;
        while i < h.read::<u32>(list) {
            let events: u32 = h.read(list.wrapping_add(8));
            let event: u32 = h.read(events.wrapping_add(4 * i));
            if event != 0 && h.call_sym(SFX_ANY_EVENT_FLAG, Conv::Thiscall(event), &[]) & 0xff != 0
            {
                playing = 1;
                break;
            }
            i += 1;
        }
    }
    let release: u32 = h.read(h.rebase(RELEASE_MUTEX));
    let mutex: u32 = h.read(this.wrapping_add(0xcc));
    let eax = h.call(release, Conv::Stdcall, &[mutex]);
    (eax & !0xff) | playing
}

/// Bytes of [`gdraw_set_target`]'s frame (the view's description).
pub const VIEW_DESC_FRAME: u32 = 0x14;

/// `gdraw_setRenderTargetView_00a38300(view, a, b, c, d)` (cdecl): bumps
/// the `GDraw` state's 64-bit counter `+0xac0` and clears `+0xab8` when
/// that is set; the view's description (`GetDesc`, vtable `+0x20`, into
/// its frame); then the state's `+0x78c = b`, `+0x780 = view`, `+0x790 =
/// (dimension == 6)`, `+0x784 = a`, `+0x6c8 = c`, `+0x6cc = d`. Returns
/// `d`.
pub fn gdraw_set_target(h: &mut impl Host, frame: u32, view: u32, args: [u32; 4]) -> u32 {
    let [a, b, c, d] = args;
    let state_at = h.rebase(GDRAW_STATE);
    let state: u32 = h.read(state_at);
    if h.read::<u32>(state.wrapping_add(0xab8)) != 0 {
        let lo: u32 = h.read(state.wrapping_add(0xac0));
        let (lo2, carry) = lo.overflowing_add(1);
        h.write(state.wrapping_add(0xac0), lo2);
        h.write(state.wrapping_add(0xab8), 0u32);
        let hi: u32 = h.read(state.wrapping_add(0xac4));
        h.write(state.wrapping_add(0xac4), hi.wrapping_add(u32::from(carry)));
    }
    let vtable: u32 = h.read(view);
    let f: u32 = h.read(vtable.wrapping_add(0x20));
    h.call(f, Conv::Stdcall, &[view, frame]);
    let state: u32 = h.read(state_at);
    h.write(state.wrapping_add(0x78c), b);
    h.write(state.wrapping_add(0x780), view);
    let dim: u32 = h.read(frame + 4);
    h.write(state.wrapping_add(0x790), u32::from(dim == 6));
    h.write(state.wrapping_add(0x784), a);
    h.write(state.wrapping_add(0x6c8), c);
    h.write(state.wrapping_add(0x6cc), d);
    d
}

/// `fiosUnifyFilename(name, buf, n)` (cdecl).
pub const FIOS_UNIFY: u32 = 0x0067_9820;
/// The blur shader cluster's name (`0x00c343e8`, a pointer).
pub const BLUR_CLUSTER_NAME: u32 = 0x00c3_43e8;
/// `AsyncLoadManager_sGetInstance()`.
pub const ASYNC_GET_INSTANCE: u32 = 0x0069_1b40;
/// `AsyncLoadManager_loadCluster(mgr, 0, path, callback, 1, arg64)`
/// (cdecl).
pub const ASYNC_LOAD_CLUSTER: u32 = 0x0069_1660;
/// `BlurEffect_onShaderClusterLoaded_006c3950`.
pub const BLUR_ON_LOADED: u32 = 0x006c_3950;
/// Bytes of [`blur_initialize`]'s frame (the path, then the cookie).
pub const BLUR_PATH_FRAME: u32 = 0x104;

/// `BlurEffect_initialize_006c38b0(this; a, b, c, d, e)` (thiscall, ret
/// 0x14): `+0x14 = a`, `+0x2c = d`, `+0x30 = e`, `+0x34 = b`, `+0x38 = c`;
/// the cluster's path in its frame (`fiosUnifyFilename`), loaded through
/// `AsyncLoadManager_loadCluster(mgr, 0, path,
/// BlurEffect_onShaderClusterLoaded_006c3950, 1, this as i64)`. Returns 0.
pub fn blur_initialize(h: &mut impl Host, frame: u32, this: u32, args: [u32; 5]) -> u32 {
    let [a, b, c, d, e] = args;
    h.write(this.wrapping_add(0x14), a);
    h.write(this.wrapping_add(0x2c), d);
    h.write(this.wrapping_add(0x30), e);
    h.write(this.wrapping_add(0x34), b);
    h.write(this.wrapping_add(0x38), c);
    let name: u32 = h.read(h.rebase(BLUR_CLUSTER_NAME));
    h.call_sym(FIOS_UNIFY, Conv::Cdecl, &[name, frame, 0x100]);
    let mgr = h.call_sym(ASYNC_GET_INSTANCE, Conv::Cdecl, &[]);
    let callback = h.rebase(BLUR_ON_LOADED);
    let high = if (this as i32) < 0 { u32::MAX } else { 0 };
    h.call_sym(
        ASYNC_LOAD_CLUSTER,
        Conv::Cdecl,
        &[mgr, 0, frame, callback, 1, this, high],
    );
    0
}

/// The shout's volume getter (`0x00948d18`, an FMOD import's thunk;
/// stdcall `(channel, &volume)`).
pub const FMOD_SHOUT_GET: u32 = 0x0094_8d18;
/// The shout's volume setter (`0x00948d12`, an FMOD import's thunk;
/// stdcall `(channel, volume)`).
pub const FMOD_SHOUT_SET: u32 = 0x0094_8d12;
/// The file name `FmodShout` passes to `ERRCHECK` (`0x00b4fd34`).
pub const FMOD_SHOUT_FILE: u32 = 0x00b4_fd34;

/// `FmodShout_setVolume(this; volume)` (thiscall, ret 4; `frame` its
/// volume word): with a channel (`+4` set; the handle at `+8`), reads the
/// current volume into its frame (1.0 first); when that succeeds, sets
/// `volume` (through `fld`/`fstp`) and checks it (`ERRCHECK`, line 0xd9).
/// Callers read no result (the original leaves the caller's `eax` when
/// `+4` is 0).
pub fn shout_set_volume(h: &mut impl Host, frame: u32, this: u32, volume: u32) {
    // `push ecx` makes the volume word: `this` until it is set
    h.write(frame, this);
    if h.read::<u32>(this.wrapping_add(4)) == 0 {
        return;
    }
    h.write(frame, 1.0f32.to_bits());
    let channel: u32 = h.read(this.wrapping_add(8));
    if h.call_sym(FMOD_SHOUT_GET, Conv::Stdcall, &[channel, frame]) != 0 {
        return;
    }
    let v = ffx_bridge::x87::round_trip(f32::from_bits(volume)).to_bits();
    let channel: u32 = h.read(this.wrapping_add(8));
    let r = h.call_sym(FMOD_SHOUT_SET, Conv::Stdcall, &[channel, v]);
    let file = h.rebase(FMOD_SHOUT_FILE);
    h.call_sym(ERRCHECK, Conv::Cdecl, &[r, file, 0xd9]);
}

/// `memcpy` (`0x00949256`, the import's thunk; cdecl).
pub const MEMCPY: u32 = 0x0094_9256;

/// Where a post-effect's parameter setter finds things: the word that
/// must be set, the parameter record (`+8` its offset, `+0xa` its size in
/// the low 13 bits), the object whose `+4` is the constant buffer (its
/// first word the buffer's size).
#[derive(Clone, Copy)]
pub struct ParamSlot {
    /// Offset of the word checked for 0 (no parameter: 0x13).
    pub guard: u32,
    /// Offset of the parameter record's pointer.
    pub param: u32,
    /// Offset of the buffer holder's pointer.
    pub holder: u32,
}

/// `BlurEffectEx_setParam48_006c45c0`: `+0x48`, buffer `(+0x20)+4`.
pub const PARAM_48: ParamSlot = ParamSlot {
    guard: 0x48,
    param: 0x48,
    holder: 0x20,
};
/// `BlurEffectEx_setParam4c_006c4680`: `+0x4c`, buffer `(+0x20)+4`.
pub const PARAM_4C: ParamSlot = ParamSlot {
    guard: 0x4c,
    param: 0x4c,
    holder: 0x20,
};
/// `BlurEffectEx_setParam50_006c4730`: checks `+0x54`, writes `+0x50`'s
/// parameter, buffer `(+0x20)+4`.
pub const PARAM_50: ParamSlot = ParamSlot {
    guard: 0x54,
    param: 0x50,
    holder: 0x20,
};
/// `RadialBlur_setParam54_007036f0`: `+0x54`, buffer `(+0x2c)+4`.
pub const PARAM_54: ParamSlot = ParamSlot {
    guard: 0x54,
    param: 0x54,
    holder: 0x2c,
};

/// The post-effects' vector-parameter setters (thiscall(this; x, y, z,
/// w), ret 0x10; `args` the address of its four argument words): no
/// parameter gives 0x13; one whose `offset + size` passes the buffer's
/// size or whose offset is below 4 gives 3; else `memcpy(buffer + offset,
/// args, size)` (the size can pass the four words: the bytes after them
/// are the caller's) and 0.
pub fn post_set_param(h: &mut impl Host, slot: ParamSlot, this: u32, args: u32) -> u32 {
    if h.read::<u32>(this.wrapping_add(slot.guard)) == 0 {
        return 0x13;
    }
    let param: u32 = h.read(this.wrapping_add(slot.param));
    let holder: u32 = h.read(this.wrapping_add(slot.holder));
    let buffer: u32 = h.read(holder.wrapping_add(4));
    let offset = u32::from(h.read::<u16>(param.wrapping_add(8)));
    let size = u32::from(h.read::<u16>(param.wrapping_add(0xa)) & 0x1fff);
    if offset + size > h.read::<u32>(buffer) || offset < 4 {
        return 3;
    }
    h.call_sym(
        MEMCPY,
        Conv::Cdecl,
        &[offset.wrapping_add(buffer), args, size],
    );
    0
}

/// `_IggyGDrawMallocAnnotated@12` (`0x00b0c86c`, import slot).
pub const GDRAW_MALLOC: u32 = 0x00b0_c86c;
/// The pools `gdraw_newPool_00a3bcd0` sizes (`0x00c6d448`: per kind, the
/// node count and the value).
pub const GDRAW_POOL_SIZES: u32 = 0x00c6_d448;
/// `op_node_pool_init_00a39bc0(set, n, v)` (cdecl).
pub const NODE_POOL_INIT: u32 = 0x00a3_9bc0;
/// The source file `GDraw` names in its allocations (`0x00b7a5e8`).
pub const GDRAW_SOURCE: u32 = 0x00b7_a5e8;

/// `gdraw_newPool_00a3bcd0(kind)` (cdecl; `args` the address of its
/// argument, which it overwrites with the pool's value): `n` nodes and a
/// value from the table, `_IggyGDrawMallocAnnotated(0x1a8 + 0x38 n,
/// file, 0x868)`; with memory, `op_node_pool_init_00a39bc0(p, n, value)`
/// and bit 0 of `+0xc` set to `kind == 2`. Returns the pool (0 when the
/// allocation failed).
pub fn gdraw_new_pool(h: &mut impl Host, args: u32) -> u32 {
    let kind: u32 = h.read(args);
    let table = h
        .rebase(GDRAW_POOL_SIZES)
        .wrapping_add(kind.wrapping_mul(8));
    let n: u32 = h.read(table);
    let value: u32 = h.read(table.wrapping_add(4));
    h.write(args, value);
    let malloc: u32 = h.read(h.rebase(GDRAW_MALLOC));
    let source = h.rebase(GDRAW_SOURCE);
    let size = n.wrapping_mul(0x38).wrapping_add(0x1a8);
    let pool = h.call(malloc, Conv::Stdcall, &[size, source, 0x868]);
    if pool == 0 {
        return 0;
    }
    let value: u32 = h.read(args);
    h.call_sym(NODE_POOL_INIT, Conv::Cdecl, &[pool, n, value]);
    let flags: u32 = h.read(pool.wrapping_add(0xc));
    h.write(pool.wrapping_add(0xc), (flags & !1) | u32::from(kind == 2));
    pool
}

/// `PApplication_FixupClusters(&cluster, n)` (cdecl).
pub const FIXUP_CLUSTERS: u32 = 0x0062_3580;
/// The distortion's handler once its shader cluster is fixed up
/// (`0x006c2520`, thiscall).
pub const DISTORTION_ON_CLUSTER: u32 = 0x006c_2520;

/// `Distortion_onShaderClusterLoaded_006c28b0(cluster, &distortion)`
/// (cdecl; `args` the address of its arguments):
/// `AsyncLoadManager_sGetInstance()` (unused), the distortion's `+0x20 =
/// cluster`, `GetApplication()` (unused),
/// `PApplication_FixupClusters(&args[0], 1)`, then a jump to `0x006c2520`
/// on the distortion, whose `eax` it returns.
pub fn distortion_on_cluster_loaded(h: &mut impl Host, args: u32) -> u32 {
    h.call_sym(ASYNC_GET_INSTANCE, Conv::Cdecl, &[]);
    let holder: u32 = h.read(args + 4);
    let distortion: u32 = h.read(holder);
    let cluster: u32 = h.read(args);
    h.write(distortion.wrapping_add(0x20), cluster);
    h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
    h.call_sym(FIXUP_CLUSTERS, Conv::Cdecl, &[args, 1]);
    h.call_sym(DISTORTION_ON_CLUSTER, Conv::Thiscall(distortion), &[])
}
