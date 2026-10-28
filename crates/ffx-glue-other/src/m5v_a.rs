//! M5 stage 1, list `vfx`, batch 1: what `graphicFinalize`, the C++
//! unwind funclets and the effect scripts reach of the remaster's VFX
//! glue and no coverage run ran. The singletons' destroys (the UI object,
//! the character shader renderer, the VFX render map and data table, the
//! VFX shader manager, the broken-screen effect), their destructors, the
//! scalar and vector deleting destructors of `ClassDynamicGeometry` and
//! `ClassCharacter`, MSVC `std::map` iterator steps and a 64-bit key
//! compare, the copy-buffer and draw-event `graphicVFX*` entry points,
//! the pointer array's push and the depth comparator of the VFX merge.
//!
//! SEH frames and stack cookies are left out: nothing on these paths
//! throws. Where the original hands a callee the address of a frame word,
//! the bridge takes the original's `ebp` and keeps the word at its offset
//! (the shim passes a `StackFrame`). Callees stay calls to the originals
//! (through their hooks where they are ported), with the convention their
//! `ret` says.

use ffx_bridge::Global;

/// `g_OpDebugMode` (byte): `op_loop_end_ffx`'s only read.
pub const OP_DEBUG_MODE: Global<u8> = Global::new(0x0232_2790);
/// `g_GraphicUi`.
pub const GRAPHIC_UI: Global<u32> = Global::new(0x00cc_c81c);
/// `g_MenuTextureManager`.
pub const MENU_TEXTURE_MANAGER: Global<u32> = Global::new(0x00cc_c820);
/// `g_ChrShaderRender`.
pub const CHR_SHADER_RENDER: Global<u32> = Global::new(0x00cc_ca74);
/// `g_VFXRenderMap_00ccca70`: the 8-byte holder of a `std::map` head.
pub const VFX_RENDER_MAP: Global<u32> = Global::new(0x00cc_ca70);
/// `g_VFXRenderDataTable`.
pub const VFX_RENDER_DATA_TABLE: Global<u32> = Global::new(0x00cc_ca6c);
/// `g_VfxShaderManager_00cdeb80` (`VfxShaderManager_CreateInstance_006c1420`).
pub const VFX_SHADER_MANAGER: Global<u32> = Global::new(0x00cd_eb80);
/// `g_BrokenScreenEffect_00cdec98` (`BrokenScreenEffect_GetInstance`).
pub const BROKEN_SCREEN_EFFECT: Global<u32> = Global::new(0x00cd_ec98);
/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_GraphicSkipFrame`: the VFX entry points do nothing while set.
pub const SKIP_FRAME: Global<u32> = Global::new(0x012f_b7d0);

/// `g_ChrClothFlip` (`f32` -1.0): the comparator's depth for no geometry.
pub const MINUS_ONE: u32 = 0x00b9_2340;
/// `ClassCharacter::vftable`.
pub const CLASS_CHARACTER_VFTABLE: u32 = 0x00b4_7e0c;
/// `ClassDynamicGeometry::vftable`.
pub const DYNAMIC_GEOMETRY_VFTABLE: u32 = 0x00b4_7e14;
/// The element destructor `eh_vector_destructor_iterator` gets from
/// `ClassDynamicGeometry`'s vector deleting destructor (a label).
pub const DYNAMIC_GEOMETRY_ELEMENT_DTOR: u32 = 0x0069_ea10;

// ---- callees ----

/// `operator_delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `operator_delete_00630540(p)` (cdecl, `delete[]`).
pub const OPERATOR_DELETE_ARRAY: u32 = 0x0063_0540;
/// `operator_new(n)` (cdecl).
pub const OPERATOR_NEW: u32 = 0x0063_0310;
/// `operator_new_006304c0(n)` (cdecl, `new[]`).
pub const OPERATOR_NEW_ARRAY: u32 = 0x0063_04c0;
/// `memset` (the CRT thunk, cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `Phyre_PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `GraphicUi_dtor_0067b230(ui)` (thiscall).
pub const GRAPHIC_UI_DTOR: u32 = 0x0067_b230;
/// `GraphicUi_ctor_0067ad30(ui)` (thiscall) -> `ui`.
pub const GRAPHIC_UI_CTOR: u32 = 0x0067_ad30;
/// `MenuTextureManager_ctor_0067b180(m)` (thiscall) -> `m`.
pub const MENU_TEXTURE_MANAGER_CTOR: u32 = 0x0067_b180;
/// `ChrShaderRender_FreeBuffers_006a0070(r)` (thiscall).
pub const CHR_SHADER_FREE_BUFFERS: u32 = 0x006a_0070;
/// `std_Tree_erase_range_006a1660(tree; out, first, last)` (thiscall,
/// `ret 0xc`).
pub const TREE_ERASE_RANGE_1660: u32 = 0x006a_1660;
/// `std_Tree_erase_range_006a19d0(tree; out, first, last)` (thiscall,
/// `ret 0xc`).
pub const TREE_ERASE_RANGE_19D0: u32 = 0x006a_19d0;
/// `VFXRenderDataTable_dtor_0069e900(table)` (thiscall).
pub const VFX_RENDER_DATA_TABLE_DTOR: u32 = 0x0069_e900;
/// `Phyre_PRedBlackTree_remove(tree; node)` (thiscall, `ret 4`).
pub const TREE_REMOVE: u32 = 0x0057_5000;
/// `Phyre_PRedBlackTree_clear(tree)` (thiscall).
pub const TREE_CLEAR: u32 = 0x0057_4760;
/// `Phyre_PRedBlackTree_orphanAllContents(tree)` (thiscall).
pub const TREE_ORPHAN_ALL: u32 = 0x0057_4970;
/// `Phyre_PFreeList_reset(list)` (thiscall).
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
/// `Phyre_PFreeList_dtor_PFreeList(list)` (thiscall).
pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
/// `VfxShaderManager_releaseClusters_006c1f20(m)` (thiscall).
pub const VFX_SHADER_RELEASE_CLUSTERS: u32 = 0x006c_1f20;
/// `PTreeFreeList_clear_006c0520(t)` (thiscall).
pub const TREE_FREE_LIST_CLEAR: u32 = 0x006c_0520;
/// `VfxShaderManager_dtor_006bfaa0(m)` (thiscall).
pub const VFX_SHADER_MANAGER_DTOR: u32 = 0x006b_faa0;
/// `BrokenScreenEffect_release_006d5a00(e)` (thiscall).
pub const BROKEN_SCREEN_RELEASE: u32 = 0x006d_5a00;
/// `PhyreScene_addCopyBufferEvent_006568a0(scene; a, b, clear, rgb)`
/// (thiscall, `ret 0x10`).
pub const SCENE_ADD_COPY_BUFFER_EVENT: u32 = 0x0065_68a0;
/// `PhyreScene_addVFXDrawEvent(scene; event)` (thiscall, `ret 4`).
pub const SCENE_ADD_VFX_DRAW_EVENT: u32 = 0x0065_6940;
/// `Array_Reserve_00667e80(array; n)` (thiscall, `ret 4`).
pub const ARRAY_RESERVE: u32 = 0x0066_7e80;
/// `ClassDynamicGeometry_clear(g)` (thiscall).
pub const DYNAMIC_GEOMETRY_CLEAR: u32 = 0x006a_1040;
/// `ClassCharacter_dtor_0069e830(c)` (thiscall).
pub const CLASS_CHARACTER_DTOR: u32 = 0x0069_e830;
/// `eh_vector_destructor_iterator(array, size, n, dtor)` (stdcall).
pub const EH_VECTOR_DTOR_ITERATOR: u32 = 0x0094_96d2;

/// `x` as `fld dword`/`fstp dword` leaves it: a signalling NaN gets its
/// quiet bit.
#[must_use]
pub const fn quiet(x: u32) -> u32 {
    if x & 0x7f80_0000 == 0x7f80_0000 && x & 0x007f_ffff != 0 {
        x | 0x0040_0000
    } else {
        x
    }
}

pub mod bridge {
    use super::{
        ARRAY_RESERVE, BROKEN_SCREEN_EFFECT, BROKEN_SCREEN_RELEASE, CHR_SHADER_FREE_BUFFERS,
        CHR_SHADER_RENDER, CLASS_CHARACTER_DTOR, CLASS_CHARACTER_VFTABLE, DYNAMIC_GEOMETRY_CLEAR,
        DYNAMIC_GEOMETRY_ELEMENT_DTOR, DYNAMIC_GEOMETRY_VFTABLE, EH_VECTOR_DTOR_ITERATOR,
        FREE_LIST_DTOR, FREE_LIST_RESET, GRAPHIC_UI, GRAPHIC_UI_CTOR, GRAPHIC_UI_DTOR, MEMSET,
        MENU_TEXTURE_MANAGER, MENU_TEXTURE_MANAGER_CTOR, MINUS_ONE, OPERATOR_DELETE,
        OPERATOR_DELETE_ARRAY, OPERATOR_NEW, OPERATOR_NEW_ARRAY, PHYRE_FREE, SCENE,
        SCENE_ADD_COPY_BUFFER_EVENT, SCENE_ADD_VFX_DRAW_EVENT, SKIP_FRAME, TREE_CLEAR,
        TREE_ERASE_RANGE_19D0, TREE_ERASE_RANGE_1660, TREE_FREE_LIST_CLEAR, TREE_ORPHAN_ALL,
        TREE_REMOVE, VFX_RENDER_DATA_TABLE, VFX_RENDER_DATA_TABLE_DTOR, VFX_RENDER_MAP,
        VFX_SHADER_MANAGER, VFX_SHADER_MANAGER_DTOR, VFX_SHADER_RELEASE_CLUSTERS, quiet,
    };
    use ffx_graphic::host::{Conv, Host};

    fn delete(h: &mut impl Host, p: u32) -> u32 {
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p])
    }

    fn delete_array(h: &mut impl Host, p: u32) -> u32 {
        h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[p])
    }

    fn phyre_free(h: &mut impl Host, p: u32) {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[p]);
    }

    fn method(h: &mut impl Host, sym: u32, this: u32) -> u32 {
        h.call_sym(sym, Conv::Thiscall(this), &[])
    }

    /// Deletes the singleton in `global` after `dtor` (thiscall) on it,
    /// and clears the global (`clear_always`: even when it was 0).
    fn destroy(h: &mut impl Host, global: ffx_bridge::Global<u32>, dtor: u32, clear_always: bool) {
        let p = global.get(h);
        if p != 0 {
            method(h, dtor, p);
            delete(h, p);
            global.set(h, 0);
        } else if clear_always {
            global.set(h, 0);
        }
    }

    /// `op_loop_end_ffx()` (`0x007ff670`): with `g_OpDebugMode` set it
    /// tail-jumps to `0x00a5bd90`, a bare `ret`; either way nothing
    /// happens.
    pub fn op_loop_end_ffx(_h: &mut impl Host) {}

    /// `GraphicUi_Destroy_0067b820()`: the UI object's destructor
    /// (`0x0067b230`), `operator_delete`, and `g_GraphicUi` = 0 (even
    /// when it was 0).
    pub fn graphic_ui_destroy(h: &mut impl Host) {
        destroy(h, GRAPHIC_UI, GRAPHIC_UI_DTOR, true);
    }

    /// `GraphicUi_ReleaseSlotObjects_0067b850(ui)`: four arrays of
    /// 16-byte slots (`+0x2bd4` and `+0x582c` 600 slots, `+0x2bdc` and
    /// `+0x5834` 800), each slot whose first or second word is set gets
    /// its object's first virtual method (the destructor) called with 0.
    /// The object is the first word even when only the second is set (the
    /// original reads its vtable at 0 then).
    pub fn graphic_ui_release_slot_objects(h: &mut impl Host, ui: u32) {
        for (field, bytes) in [
            (0x2bd4_u32, 0x2580_u32),
            (0x2bdc, 0x3200),
            (0x582c, 0x2580),
            (0x5834, 0x3200),
        ] {
            let mut i = 0;
            while i < bytes {
                let base: u32 = h.read(ui + field);
                let obj: u32 = h.read(base.wrapping_add(i));
                let second: u32 = h.read(base.wrapping_add(i).wrapping_add(4));
                if obj | second != 0 {
                    h.call_virtual(obj, 0, &[0]);
                }
                i += 0x10;
            }
        }
    }

    /// `ChrShaderRender_FreeBuffers_006a0070(r)`: for the two buffers at
    /// `r[0]` and `r[1]`: the five arrays of their `+0x94` record
    /// (`+0x14`, `+0x1c`, `+0x10`, `+0x18`, `+0xc`, `delete[]`), then the
    /// record and the buffer (`delete`).
    pub fn chr_shader_free_buffers(h: &mut impl Host, r: u32) {
        for i in 0..2 {
            for off in [0x14_u32, 0x1c, 0x10, 0x18, 0xc] {
                let buffer: u32 = h.read(r + 4 * i);
                let rec: u32 = h.read(buffer.wrapping_add(0x94));
                let p: u32 = h.read(rec.wrapping_add(off));
                delete_array(h, p);
            }
            let buffer: u32 = h.read(r + 4 * i);
            let rec: u32 = h.read(buffer.wrapping_add(0x94));
            delete(h, rec);
            let buffer: u32 = h.read(r + 4 * i);
            delete(h, buffer);
        }
    }

    /// `ChrShaderRender_Destroy_0069f540()`: with `g_ChrShaderRender`
    /// set: its buffers freed (`0x006a0070`), the object at `+8` given its
    /// first virtual method with 1 and the field cleared, the renderer
    /// deleted and the global cleared.
    pub fn chr_shader_destroy(h: &mut impl Host) {
        let r = CHR_SHADER_RENDER.get(h);
        if r == 0 {
            return;
        }
        method(h, CHR_SHADER_FREE_BUFFERS, r);
        let obj: u32 = h.read(r + 8);
        if obj != 0 {
            h.call_virtual(obj, 0, &[1]);
            h.write(r + 8, 0_u32);
        }
        delete(h, r);
        CHR_SHADER_RENDER.set(h, 0);
    }

    /// `VFXRenderMap_Destroy_0069f580()` (frame: the erase's out iterator
    /// at `ebp - 4`): the map's nodes erased
    /// (`0x006a1660(map; &out, head->left, head)`), the head and the
    /// holder deleted, the global cleared.
    pub fn vfx_render_map_destroy(h: &mut impl Host, ebp: u32) {
        let map = VFX_RENDER_MAP.get(h);
        if map == 0 {
            return;
        }
        let head: u32 = h.read(map);
        let first: u32 = h.read(head);
        h.call_sym(
            TREE_ERASE_RANGE_1660,
            Conv::Thiscall(map),
            &[ebp - 4, first, head],
        );
        let head: u32 = h.read(map);
        delete(h, head);
        delete(h, map);
        VFX_RENDER_MAP.set(h, 0);
    }

    /// `VFXRenderDataTable_dtor_0069e900(table)` (frame: `this` at
    /// `ebp - 0x10`, the erase's out iterator): three Phyre arrays freed
    /// (`+0x18`/`+0x1c` whenever the count's top bit is clear, the
    /// others only with data), each pair zeroed; the map's nodes erased
    /// (`0x006a19d0`) and its head deleted.
    pub fn vfx_render_data_table_dtor(h: &mut impl Host, ebp: u32, table: u32) {
        h.write(ebp - 0x10, table);
        let n: u32 = h.read(table + 0x18);
        if n >> 31 == 0 {
            let p: u32 = h.read(table + 0x1c);
            phyre_free(h, p);
        }
        h.write(table + 0x1c, 0_u32);
        h.write(table + 0x18, 0_u32);
        for (count, data) in [(0x10_u32, 0x14_u32), (0x8, 0xc)] {
            let n: u32 = h.read(table + count);
            if n >> 31 == 0 {
                let p: u32 = h.read(table + data);
                if p != 0 {
                    phyre_free(h, p);
                }
            }
            h.write(table + data, 0_u32);
            h.write(table + count, 0_u32);
        }
        let head: u32 = h.read(table);
        let first: u32 = h.read(head);
        h.call_sym(
            TREE_ERASE_RANGE_19D0,
            Conv::Thiscall(table),
            &[ebp - 0x10, first, head],
        );
        let head: u32 = h.read(table);
        delete(h, head);
    }

    /// `VFXRenderDataTable_Destroy_0069f5c0()`.
    pub fn vfx_render_data_table_destroy(h: &mut impl Host) {
        destroy(h, VFX_RENDER_DATA_TABLE, VFX_RENDER_DATA_TABLE_DTOR, false);
    }

    /// `PTreeFreeList_clear_006c0520(t)`: while the tree's `+0xc` node is
    /// neither the tree nor 0: removed (`Phyre_PRedBlackTree_remove`),
    /// and its `+0xc` word freed (`Phyre_PhyreFree`) when that word's low
    /// bit is clear; then tail-jumps to `Phyre_PFreeList_reset(t + 0x18)`
    /// -> its `eax`.
    pub fn tree_free_list_clear(h: &mut impl Host, t: u32) -> u32 {
        loop {
            let node: u32 = h.read(t + 0xc);
            if node == t || node == 0 {
                break;
            }
            h.call_sym(TREE_REMOVE, Conv::Thiscall(t), &[node]);
            let flags: u8 = h.read(node + 0xc);
            if flags & 1 == 0 {
                let p: u32 = h.read(node + 0xc);
                phyre_free(h, p);
            }
        }
        method(h, FREE_LIST_RESET, t + 0x18)
    }

    /// `VfxShaderManager_dtor_006bfaa0(m)`: the clusters released
    /// (`0x006c1f20`); the trees at `+0x68` and `+0x30` emptied
    /// (`0x006c0520`), their free lists (`+0x18`) destroyed and each
    /// cleared twice; then the manager's own tree orphaned, its free list
    /// reset and destroyed, the tree cleared twice.
    pub fn vfx_shader_manager_dtor(h: &mut impl Host, m: u32) {
        method(h, VFX_SHADER_RELEASE_CLUSTERS, m);
        for t in [m + 0x68, m + 0x30] {
            method(h, TREE_FREE_LIST_CLEAR, t);
            method(h, FREE_LIST_DTOR, t + 0x18);
            method(h, TREE_CLEAR, t);
            method(h, TREE_CLEAR, t);
        }
        method(h, TREE_ORPHAN_ALL, m);
        method(h, FREE_LIST_RESET, m + 0x18);
        method(h, FREE_LIST_DTOR, m + 0x18);
        method(h, TREE_CLEAR, m);
        method(h, TREE_CLEAR, m);
    }

    /// `VfxShaderManager_Destroy_006c14a0()`: the manager's destructor
    /// and `operator_delete`; the global stays as it is.
    pub fn vfx_shader_manager_destroy(h: &mut impl Host) {
        let m = VFX_SHADER_MANAGER.get(h);
        if m != 0 {
            method(h, VFX_SHADER_MANAGER_DTOR, m);
            delete(h, m);
        }
    }

    /// `BrokenScreenEffect_Destroy_006d4db0()`.
    pub fn broken_screen_destroy(h: &mut impl Host) {
        destroy(h, BROKEN_SCREEN_EFFECT, BROKEN_SCREEN_RELEASE, false);
    }

    /// `graphicVFXAddCopyBufferEvent*(a, b)`: unless the frame is
    /// skipped, `0x006568a0(scene; a, b, clear, rgb)` (the three entry
    /// points pass (0, 0), (1, 1) and (0, 1)).
    pub fn add_copy_buffer_event(h: &mut impl Host, a: u32, b: u32, clear: u32, rgb: u32) {
        if SKIP_FRAME.get(h) != 0 {
            return;
        }
        let scene = SCENE.get(h);
        h.call_sym(
            SCENE_ADD_COPY_BUFFER_EVENT,
            Conv::Thiscall(scene),
            &[a, b, clear, rgb],
        );
    }

    /// `graphicVFXAddDrawCharacterEvent(chr, flag)` and
    /// `graphicVFXAddDrawZPrePassEvent(chr)` (frame: the 16-byte event at
    /// `ebp - 0x14`): unless the frame is skipped or `chr` is 0, the
    /// event {kind, -, chr, flag} goes to `PhyreScene_addVFXDrawEvent`.
    /// The character event's flag is a byte (`Some`); the pre-pass event
    /// writes no flag.
    ///
    /// junk read: [ebp-0x10] never written; 0 (brief 2026-10-24), and for
    /// the character event the three bytes above the flag at [ebp-0x7];
    /// for the pre-pass event the whole word at [ebp-0x8] too. The
    /// callee copies the event, so the shim's zeroed frame passes zeros
    /// there.
    pub fn add_draw_event(h: &mut impl Host, ebp: u32, kind: u32, chr: u32, flag: Option<u8>) {
        if SKIP_FRAME.get(h) != 0 || chr == 0 {
            return;
        }
        let scene = SCENE.get(h);
        h.write(ebp - 0xc, chr);
        if let Some(flag) = flag {
            h.write(ebp - 8, flag);
        }
        h.write(ebp - 0x14, kind);
        h.call_sym(
            SCENE_ADD_VFX_DRAW_EVENT,
            Conv::Thiscall(scene),
            &[ebp - 0x14],
        );
    }

    /// `PArray_PushBack_006560c0(array; &value)` -> 0: the count (`+0`)
    /// goes up; past the capacity (`+4`, signed) the array grows to
    /// `count + 2 + count / 8` (`Array_Reserve_00667e80`); the word goes
    /// into the last slot of `+8` unless that slot's address is 0.
    pub fn parray_push_back(h: &mut impl Host, array: u32, value: u32) -> u32 {
        let n = h.read::<u32>(array).wrapping_add(1);
        h.write(array, n);
        let cap: i32 = h.read(array + 4);
        if n as i32 > cap {
            let grow = ((n as i32) / 8) as u32;
            h.call_sym(
                ARRAY_RESERVE,
                Conv::Thiscall(array),
                &[grow.wrapping_add(n.wrapping_add(2))],
            );
        }
        let n: u32 = h.read(array);
        let data: u32 = h.read(array + 8);
        let slot = data.wrapping_add(n.wrapping_sub(1).wrapping_mul(4));
        if slot != 0 {
            let v: u32 = h.read(value);
            h.write(slot, v);
        }
        0
    }

    /// `VFXGeometry_CompareDepth(&a, &b)` -> -1 when a's depth (`+0xdc`,
    /// -1.0 for no geometry) is below b's, else 1 (equal and unordered
    /// too). Its argument slots end up holding b's depth and a's (as
    /// `fstp dword` stores them: quiet).
    pub fn compare_depth(h: &mut impl Host, slots: u32, a: u32, b: u32) -> i32 {
        let minus_one: u32 = h.read(h.rebase(MINUS_ONE));
        let ga: u32 = h.read(a);
        let gb: u32 = h.read(b);
        let da = quiet(if ga != 0 {
            h.read(ga + 0xdc)
        } else {
            minus_one
        });
        let db = quiet(if gb != 0 {
            h.read(gb + 0xdc)
        } else {
            minus_one
        });
        h.write(slots + 4, da);
        h.write(slots, db);
        if f32::from_bits(da) < f32::from_bits(db) {
            -1
        } else {
            1
        }
    }

    /// `GraphicUi_AllocBuffers_0067b770(ui)`: `+0x5b94` = 0x218, `+0x5b98`
    /// = 0xadc, two 0x26e020-byte arrays (`new[]`) at `+0x5b80` and
    /// `+0x5b84`, both cleared (`memset`) -> the second `memset`'s `eax`.
    pub fn graphic_ui_alloc_buffers(h: &mut impl Host, ui: u32) -> u32 {
        const N: u32 = 0x26_e020;
        h.write(ui + 0x5b94, 0x218_u32);
        h.write(ui + 0x5b98, 0xadc_u32);
        let a = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[N]);
        h.write(ui + 0x5b80, a);
        let b = h.call_sym(OPERATOR_NEW_ARRAY, Conv::Cdecl, &[N]);
        h.write(ui + 0x5b84, b);
        let a: u32 = h.read(ui + 0x5b80);
        h.call_sym(MEMSET, Conv::Cdecl, &[a, 0, N]);
        let b: u32 = h.read(ui + 0x5b84);
        h.call_sym(MEMSET, Conv::Cdecl, &[b, 0, N])
    }

    /// `GraphicUi_CreateInstance_0067b6c0()`: without `g_GraphicUi`, a
    /// 0x2ec34-byte object (`operator_new`) built by `0x0067ad30` (0 when
    /// the allocation fails) becomes it.
    pub fn graphic_ui_create_instance(h: &mut impl Host) {
        if GRAPHIC_UI.get(h) != 0 {
            return;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x2_ec34]);
        let ui = if p != 0 {
            method(h, GRAPHIC_UI_CTOR, p)
        } else {
            0
        };
        GRAPHIC_UI.set(h, ui);
    }

    /// `MenuTextureManager_CreateInstance_0067b740()`: the same for the
    /// 0x210-byte menu texture manager (`0x0067b180`).
    pub fn menu_texture_manager_create_instance(h: &mut impl Host) {
        if MENU_TEXTURE_MANAGER.get(h) != 0 {
            return;
        }
        let p = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0x210]);
        let m = if p != 0 {
            method(h, MENU_TEXTURE_MANAGER_CTOR, p)
        } else {
            0
        };
        MENU_TEXTURE_MANAGER.set(h, m);
    }

    /// `PArray_release_0069e4a0(a)` (`nonnull` false: frees the data
    /// even when 0) and `PArray_releaseNonNull_0069e4d0(a)`: a Phyre
    /// array {count (top bit: not owned), data} freed when owned, both
    /// words zeroed.
    pub fn parray_release(h: &mut impl Host, a: u32, nonnull: bool) {
        let n: u32 = h.read(a);
        if n >> 31 == 0 {
            let p: u32 = h.read(a + 4);
            if !nonnull || p != 0 {
                phyre_free(h, p);
            }
        }
        if nonnull {
            h.write(a + 4, 0_u32);
            h.write(a, 0_u32);
        } else {
            h.write(a, 0_u32);
            h.write(a + 4, 0_u32);
        }
    }

    /// `PRedBlackTree_dtor_0069e500(t)`: `Phyre_PRedBlackTree_clear`
    /// twice (the derived and the base destructor).
    pub fn tree_dtor(h: &mut impl Host, t: u32) {
        method(h, TREE_CLEAR, t);
        method(h, TREE_CLEAR, t);
    }

    /// `std_Map_dtor_0069e770(map)` (frame: the erase's out iterator at
    /// `ebp - 4`): every node erased (`0x006a19d0`), the head deleted.
    pub fn std_map_dtor(h: &mut impl Host, ebp: u32, map: u32) {
        let head: u32 = h.read(map);
        let first: u32 = h.read(head);
        h.call_sym(
            TREE_ERASE_RANGE_19D0,
            Conv::Thiscall(map),
            &[ebp - 4, first, head],
        );
        let head: u32 = h.read(map);
        delete(h, head);
    }

    /// `std_String_dtor_0069e7d0(s)`: MSVC `std::string`'s tidy: a heap
    /// buffer (capacity `+0x14` 16 or more) deleted, then capacity 15,
    /// size 0 and an empty inline string.
    pub fn std_string_dtor(h: &mut impl Host, s: u32) {
        let cap: u32 = h.read(s + 0x14);
        if cap >= 0x10 {
            let p: u32 = h.read(s);
            delete(h, p);
        }
        h.write(s + 0x14, 0xf_u32);
        h.write(s + 0x10, 0_u32);
        h.write(s, 0_u8);
    }

    /// `ClassCharacter_dtor_0069e830(c)`: the vtable back to
    /// `ClassCharacter`'s, the `+0x18` pointers of the `+0x14` array and
    /// the array itself `delete[]`d (when set), the `+8` buffer deleted
    /// and `+8`/`+0xc`/`+0x10` cleared (when set).
    pub fn class_character_dtor(h: &mut impl Host, c: u32) {
        let vt = h.rebase(CLASS_CHARACTER_VFTABLE);
        h.write(c, vt);
        let mut i = 0_i32;
        while i < h.read::<i32>(c + 0x18) {
            let list: u32 = h.read(c + 0x14);
            let p: u32 = h.read(list.wrapping_add(4 * i as u32));
            if p != 0 {
                delete_array(h, p);
            }
            i += 1;
        }
        let list: u32 = h.read(c + 0x14);
        if list != 0 {
            delete_array(h, list);
        }
        let buffer: u32 = h.read(c + 8);
        if buffer != 0 {
            delete(h, buffer);
            h.write(c + 8, 0_u32);
            h.write(c + 0xc, 0_u32);
            h.write(c + 0x10, 0_u32);
        }
    }

    /// `std_TreeIterator_dec_0069eca0(it)` and its copy at `0x0069ed00`:
    /// MSVC `_Tree` iterator `--` (nodes: left `+0`, parent `+4`, right
    /// `+8`, nil byte `+0xd`) -> `it`.
    pub fn tree_iterator_dec(h: &mut impl Host, it: u32) -> u32 {
        let node: u32 = h.read(it);
        if h.read::<u8>(node + 0xd) != 0 {
            let right: u32 = h.read(node + 8);
            h.write(it, right);
            return it;
        }
        let left: u32 = h.read(node);
        if h.read::<u8>(left + 0xd) == 0 {
            // the rightmost node of the left subtree
            let mut n = left;
            loop {
                let r: u32 = h.read(n + 8);
                if h.read::<u8>(r + 0xd) != 0 {
                    break;
                }
                n = r;
            }
            h.write(it, n);
            return it;
        }
        // up while the node is its parent's left child
        let mut parent: u32 = h.read(node + 4);
        while h.read::<u8>(parent + 0xd) == 0 {
            let cur: u32 = h.read(it);
            if cur != h.read::<u32>(parent) {
                break;
            }
            h.write(it, parent);
            parent = h.read(parent + 4);
        }
        let cur: u32 = h.read(it);
        if h.read::<u8>(cur + 0xd) == 0 {
            h.write(it, parent);
        }
        it
    }

    /// `Key64_less_0069eda0(&a, &b)` (stdcall): a < b as unsigned 64-bit
    /// {low, high} keys, in `al`; the rest of `eax` is the word the last
    /// compare loaded from `a` (high, or low when the highs are equal).
    pub fn key64_less(h: &mut impl Host, a: u32, b: u32) -> u32 {
        let ah: u32 = h.read(a + 4);
        let bh: u32 = h.read(b + 4);
        let (word, less) = if ah == bh {
            let al: u32 = h.read(a);
            let bl: u32 = h.read(b);
            (al, al < bl)
        } else {
            (ah, ah < bh)
        };
        (word & !0xff) | u32::from(less)
    }

    /// `ClassDynamicGeometry_vecDelDtor_0069edd0(g; flags)`: bit 1, the
    /// array form: the count at `g - 4`, each 0xf8-byte element destroyed
    /// (`eh_vector_destructor_iterator`), the block (`g - 4`) `delete[]`d
    /// with bit 0 -> `g - 4`; else the vtable reset, `ClassDynamicGeometry_clear`,
    /// `delete` with bit 0 -> `g`.
    pub fn dynamic_geometry_vec_del_dtor(h: &mut impl Host, g: u32, flags: u32) -> u32 {
        if flags & 2 != 0 {
            let n: u32 = h.read(g.wrapping_sub(4));
            let dtor = h.rebase(DYNAMIC_GEOMETRY_ELEMENT_DTOR);
            h.call_sym(EH_VECTOR_DTOR_ITERATOR, Conv::Stdcall, &[g, 0xf8, n, dtor]);
            if flags & 1 != 0 {
                delete_array(h, g.wrapping_sub(4));
            }
            return g.wrapping_sub(4);
        }
        dynamic_geometry_scalar_del_dtor(h, g, flags)
    }

    /// `ClassDynamicGeometry_scalarDelDtor_*(g; flags)` (`0x0069ef00`,
    /// `0x0069efa0`, `0x0069efd0`): the vtable reset,
    /// `ClassDynamicGeometry_clear`, `delete` with bit 0 -> `g`.
    pub fn dynamic_geometry_scalar_del_dtor(h: &mut impl Host, g: u32, flags: u32) -> u32 {
        let vt = h.rebase(DYNAMIC_GEOMETRY_VFTABLE);
        h.write(g, vt);
        method(h, DYNAMIC_GEOMETRY_CLEAR, g);
        if flags & 1 != 0 {
            delete(h, g);
        }
        g
    }

    /// `ClassCharacter_scalarDelDtor_0069ee90(c; flags)`: the destructor
    /// (`0x0069e830`), `delete` with bit 0 -> `c`.
    pub fn class_character_scalar_del_dtor(h: &mut impl Host, c: u32, flags: u32) -> u32 {
        method(h, CLASS_CHARACTER_DTOR, c);
        if flags & 1 != 0 {
            delete(h, c);
        }
        c
    }
}
