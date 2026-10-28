//! M5 stage 1, list `render`, batch 7: small destructors, unwind
//! funclets and predicates of the render managers (post-process tweak,
//! debug menu, texture animation info, specular maps, shadow solutions,
//! the VFX render data table), the UI 2D VFX list push, the text slot
//! claim, the blur's float copies, the shadow meshes' class binding, the
//! PRX unload and two PS2 movie helpers.

use crate::m5r_c::quiet;
use ffx_bridge::Global;

/// `g_PhyreScene`.
pub const SCENE: Global<u32> = Global::new(0x00cc_b9d8);
/// `g_DebugMenuManager`.
pub const DEBUG_MENU_MANAGER: Global<u32> = Global::new(0x00cc_cab0);
/// `g_PrxState`.
pub const PRX_STATE: Global<u32> = Global::new(0x00ce_83c4);

pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const RESERVE_4: u32 = 0x0066_8100;
/// `PostProcessTweakUtil_vftable` and `DebugMenuManager_vftable`.
pub const VT_POST_TWEAK: u32 = 0x00b4_73fc;
pub const VT_DEBUG_MENU: u32 = 0x00b4_8630;
pub const TEXT_ENTRY_DTOR: u32 = 0x0069_2950;
pub const PUSH_PAIR_3890: u32 = 0x0065_6b90;
pub const ERASE_SUBTREE: u32 = 0x006a_0540;
pub const TEXANIM_INFO_DTOR: u32 = 0x006a_f7f0;
pub const TEXANIM_INFO2_DTOR: u32 = 0x006a_f8a0;
pub const RENDER_CORRESPOND_SPECULAR: u32 = 0x006c_59c0;
pub const RESET_SPECULAR_MAPS: u32 = 0x006c_5850;
pub const INTERLOCKED_COMPARE_EXCHANGE: u32 = 0x00b0_c12c;
pub const IOB_FUNC: u32 = 0x00b0_c350;
pub const FPRINTF: u32 = 0x00b0_c38c;
pub const FREE_LIBRARY: u32 = 0x00b0_c120;
/// The text slot warning and the dump line formats.
pub const SLOT_WARNING: u32 = 0x00b4_be8c;
pub const DUMP_FORMAT: u32 = 0x00b4_a3f4;
pub const DUMP_TAIL: u32 = 0x00b8_fbeb;
pub const REGISTER_TYPE: u32 = 0x0043_95b0;
pub const ADD_TO_NAMESPACE: u32 = 0x0043_c140;
/// The shadow mesh and shadow mesh instance class records.
pub const CLASS_SHADOW: u32 = 0x00cc_c158;
pub const CLASS_SHADOW_INSTANCE: u32 = 0x00ce_82f8;
pub const STR_PC_A: u32 = 0x0077_7440;
pub const STR_PC_B: u32 = 0x0093_db20;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn method(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn word(h: &impl Mem, at: u32) -> u32 {
        h.read(at)
    }

    fn import(h: &mut impl Host, slot: u32, conv: Conv, args: &[u32]) -> u32 {
        let f: u32 = h.read(h.rebase(slot));
        h.call(f, conv, args)
    }

    /// `PostProcessTweakUtil_scalarDeletingDtor_00692a60(this; flags)` ->
    /// `this`: the vtable stored, the text entry `+8` destroyed, deleted
    /// with flag bit 0.
    pub fn post_tweak_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(VT_POST_TWEAK);
        h.write(this, vt);
        method(h, TEXT_ENTRY_DTOR, this + 8, &[]);
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// A scalar deleting destructor over `dtor` (`this; flags`) -> `this`.
    pub fn delete_with(h: &mut impl Host, this: u32, flags: u32, dtor: u32) -> u32 {
        method(h, dtor, this, &[]);
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `DebugMenuManager_scalarDeletingDtor_006b4920(this; flags)` ->
    /// `this`: the vtable stored, deleted with flag bit 0.
    pub fn debug_menu_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(VT_DEBUG_MENU);
        h.write(this, vt);
        if flags & 1 != 0 {
            cdecl(h, OPERATOR_DELETE, &[this]);
        }
        this
    }

    /// `DebugMenuManager_CreateInstance()`: without one, `operator_new(8)`
    /// with the vtable and `+4` = 10 becomes `g_DebugMenuManager` (0 when
    /// the allocation fails). `eax` is dropped by the caller.
    pub fn debug_menu_create(h: &mut impl Host) {
        if DEBUG_MENU_MANAGER.get(h) != 0 {
            return;
        }
        let p = cdecl(h, OPERATOR_NEW, &[8]);
        if p != 0 {
            let vt = h.rebase(VT_DEBUG_MENU);
            h.write(p, vt);
            h.write(p + 4, 10_u32);
        }
        DEBUG_MENU_MANAGER.set(h, p);
    }

    /// `PhyreScene_AddVFXToUI2DList(scene; vfx)` (`ret 4`): `vfx` appended
    /// to the 4-byte array `+0x3860` (grown by `Array_Reserve_00668100` to
    /// `count + 2 + count / 8` past the capacity); `eax`: `vfx`, or the
    /// data word when the slot address is 0.
    pub fn add_vfx_to_ui2d(h: &mut impl Host, scene: u32, vfx: u32) -> u32 {
        let arr = scene + 0x3860;
        let n = word(h, arr).wrapping_add(1);
        h.write(arr, n);
        if n.cast_signed() > word(h, arr + 4).cast_signed() {
            let grown = n
                .wrapping_add(2)
                .wrapping_add((n.cast_signed() / 8).cast_unsigned());
            method(h, RESERVE_4, arr, &[grown]);
        }
        let n = word(h, arr);
        let data = word(h, arr + 8);
        let slot = data.wrapping_add(n.wrapping_sub(1).wrapping_mul(4));
        if slot == 0 {
            return data;
        }
        h.write(slot, vfx);
        vfx
    }

    /// `VfxRenderData_pushPair_006a0f30`/`_006a0f50(this)`: with a mesh
    /// `+0xc`, `PhyreScene_PushPair3890(scene; mesh, +0x2c)`'s `eax`;
    /// otherwise 0.
    pub fn push_pair(h: &mut impl Host, this: u32) -> u32 {
        let m = word(h, this + 0xc);
        if m == 0 {
            return 0;
        }
        let c = word(h, this + 0x2c);
        let scene = SCENE.get(h);
        method(h, PUSH_PAIR_3890, scene, &[m, c])
    }

    /// `VfxRenderDataTable_clearTree_006a0fe0(table)` -> the head: the
    /// tree erased from its root down the left spine (`0x006a0540` on each
    /// right subtree, the node deleted), the head linked to itself, size
    /// 0.
    pub fn clear_tree(h: &mut impl Host, table: u32) -> u32 {
        let mut node = word(h, word(h, table) + 4);
        while h.read::<u8>(node + 0xd) == 0 {
            let right = word(h, node + 8);
            method(h, ERASE_SUBTREE, table, &[right]);
            let left = word(h, node);
            cdecl(h, OPERATOR_DELETE, &[node]);
            node = left;
        }
        let head = word(h, table);
        h.write(head + 4, head);
        let head = word(h, table);
        h.write(head, head);
        let head = word(h, table);
        h.write(head + 8, head);
        h.write(table + 4, 0_u32);
        head
    }

    /// The owned array resets (`PArray_Reset` copies, unwind funclets):
    /// the buffer `+4` freed unless the count's top bit marks it borrowed
    /// (or, with `null_check`, it is 0); both words cleared.
    pub fn owned_reset(h: &mut impl Host, this: u32, null_check: bool) {
        let data = word(h, this + 4);
        if word(h, this) >> 31 == 0 && !(null_check && data == 0) {
            cdecl(h, PHYRE_FREE, &[data]);
        }
        if null_check {
            h.write(this + 4, 0_u32);
            h.write(this, 0_u32);
        } else {
            h.write(this, 0_u32);
            h.write(this + 4, 0_u32);
        }
    }

    /// `SpecularMapParamsManager_renderSpecularPass(this; x)` (`ret 4`):
    /// `renderCorrespondSpecular(0, x)`, then the maps reset; its `eax`.
    pub fn render_specular_pass(h: &mut impl Host, this: u32, x: u32) -> u32 {
        method(h, RENDER_CORRESPOND_SPECULAR, this, &[0, x]);
        method(h, RESET_SPECULAR_MAPS, this, &[])
    }

    /// `FloatLess_006c94e0(a, b)` (a sort predicate) -> `a < b` (false
    /// when unordered), in `al`.
    #[must_use]
    pub fn float_less(a: u32, b: u32) -> u8 {
        u8::from(f32::from_bits(b) > f32::from_bits(a))
    }

    /// The shadow solution predicates (`0x006cc8e0..0x006cc950`) -> `al`
    /// whether `x` is (`is`) or is not one of `ids`; the rest of `eax` is
    /// `x`'s.
    #[must_use]
    pub fn solution_in(x: u32, ids: &[u32], is: bool) -> u32 {
        let hit = ids.contains(&x);
        (x & !0xff) | u32::from(hit == is)
    }

    /// `TextUtil_dumpEntry_006dd200(entry, file)`: `fprintf(file, fmt,
    /// [+0], [+4], +8, [+0x108], tail)`'s `eax`.
    pub fn dump_entry(h: &mut impl Host, entry: u32, file: u32) -> u32 {
        let fmt = h.rebase(DUMP_FORMAT);
        let tail = h.rebase(DUMP_TAIL);
        let (a, b, c) = (word(h, entry), word(h, entry + 4), word(h, entry + 0x108));
        import(
            h,
            FPRINTF,
            Conv::Cdecl,
            &[file, fmt, a, b, entry + 8, c, tail],
        )
    }

    /// `TextUtil_claimSlot_006f5de0(slots; v)` (`ret 4`): the first of the
    /// 16 words that `InterlockedCompareExchange(word, v, 0)` finds 0
    /// takes `v`; none: a warning with `v` on stderr. `eax` the last
    /// call's.
    pub fn claim_slot(h: &mut impl Host, slots: u32, v: u32) -> u32 {
        for k in 0..16 {
            let r = import(
                h,
                INTERLOCKED_COMPARE_EXCHANGE,
                Conv::Stdcall,
                &[slots + 4 * k, v, 0],
            );
            if r == 0 {
                return 0;
            }
        }
        let iob = import(h, IOB_FUNC, Conv::Cdecl, &[]);
        let fmt = h.rebase(SLOT_WARNING);
        import(h, FPRINTF, Conv::Cdecl, &[iob + 0x40, fmt, v])
    }

    /// `BlurEffect_setFloats4_006f8b10` / `_setFloats2_006f8ba0(this;
    /// src)` (`ret 4`): `n` floats from `src` to `this + off` through the
    /// x87 (quieted); `eax` `src`.
    pub fn copy_floats(h: &mut impl Host, this: u32, src: u32, off: u32, n: u32) -> u32 {
        for k in 0..n {
            let v = word(h, src + 4 * k);
            h.write(this + off + 4 * k, quiet(v));
        }
        src
    }

    /// The shadow meshes' `Bind` (`ShadowDynamicMesh_Bind`,
    /// `ShadowDynamicMeshInstance_Bind`): the class record registered
    /// (`Phyre_PTypeDispenser_RegisterType`) and added to its namespace (a
    /// tail jump to `Phyre_PClassDescriptor_addToNamespace`).
    pub fn bind_class(h: &mut impl Host, class: u32) -> u32 {
        let c = h.rebase(class);
        cdecl(h, REGISTER_TYPE, &[c]);
        method(h, ADD_TO_NAMESPACE, c, &[])
    }

    /// `PhyreFIOS_UnloadPrx(module)`: `FreeLibrary`, `g_PrxState` = 0;
    /// the free's `eax`.
    pub fn unload_prx(h: &mut impl Host, module: u32) -> u32 {
        let r = import(h, FREE_LIBRARY, Conv::Stdcall, &[module]);
        PRX_STATE.set(h, 0);
        r
    }

    /// `syncStrPcSelect_0077e880(a, b, stream)`: `0x00777440(b, stream)`
    /// with the stream's byte `+0x5075c`, else `syncStrPc(b, stream)`;
    /// its `eax` (`a` is never read).
    pub fn sync_str_select(h: &mut impl Host, b: u32, stream: u32) -> u32 {
        let f = if h.read::<u8>(stream + 0x5_075c) == 0 {
            STR_PC_B
        } else {
            STR_PC_A
        };
        cdecl(h, f, &[b, stream])
    }

    /// `GsPackTag_0077ead0(out, a, b, c)` -> `out`: the 64-bit word
    /// `((a << 4 | b) << 28) | c` (the shifts on 64 bits, the ors on the
    /// low word) at `out`.
    pub fn gs_pack_tag(h: &mut impl Host, out: u32, a: u32, b: u32, c: u32) -> u32 {
        let x = ((((u64::from(a) << 4) | u64::from(b)) << 28) | u64::from(c)).to_le_bytes();
        h.write(out, x);
        out
    }
}
