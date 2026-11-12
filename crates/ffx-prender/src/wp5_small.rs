//! M5 finish, WP5 part 2: small originals right after ported symbols that
//! the ports called in Unicorn (the "unnamed functions after a ported
//! symbol" rows of `~/ffx-work/m5/finish/wp5/unicorn-446.tsv`). Each
//! follows its original's machine code; `eax` is the original's.

use ffx_graphic::host::{Conv, Host};

/// `Phyre_PArray_count_004320c0(this)` (thiscall, no stack words; 72
/// callers): the count word without its "external buffer" bit 31.
pub fn parray_count(h: &impl Host, this: u32) -> u32 {
    h.read::<u32>(this) & 0x7fff_ffff
}

/// `ZeroThreeWords_006906b0(this)` (thiscall): words `+0`, `+4`, `+8`
/// zeroed. `this`.
pub fn zero_three_words(h: &mut impl Host, this: u32) -> u32 {
    for off in [0, 4, 8] {
        h.write(this + off, 0u32);
    }
    this
}

/// `JobSchedule::JobWorker`'s vtable.
pub const JOB_WORKER_VTABLE: u32 = 0x00b4_cf30;

/// `JobWorker_ctor_00703960(this)` (thiscall): `+4`, `+8`, the byte
/// `+0xc` and `+0x10` zeroed, the vtable at `+0`. `this`.
pub fn job_worker_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.write(this + 4, 0u32);
    h.write(this + 8, 0u32);
    h.write(this + 0xc, 0u8);
    let vt = h.rebase(JOB_WORKER_VTABLE);
    h.write(this, vt);
    h.write(this + 0x10, 0u32);
    this
}

/// `ClearWord10_007038f0(this)` (thiscall): `+0x10` zeroed. `this`.
pub fn clear_word10(h: &mut impl Host, this: u32) -> u32 {
    h.write(this + 0x10, 0u32);
    this
}

/// `g_GraphicAbmap` and `operator delete`.
pub const GRAPHIC_ABMAP: u32 = 0x00cc_c838;
pub const OPERATOR_DELETE: u32 = 0x0063_0390;

/// `graphicAbmapFree_00684ae0()` (cdecl): `g_GraphicAbmap` freed when set,
/// then cleared. `eax`: `operator delete`'s, or 0.
pub fn abmap_free(h: &mut impl Host) -> u32 {
    let at = h.rebase(GRAPHIC_ABMAP);
    let p: u32 = h.read(at);
    let r = if p == 0 {
        0
    } else {
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p])
    };
    h.write(at, 0u32);
    r
}

/// `Phyre_PCluster_deleteObject` (thiscall `(cluster; obj)`, `ret 4`).
pub const CLUSTER_DELETE_OBJECT: u32 = 0x0044_1240;
/// `PhyreScene_deletePMeshInstanceInCluster` (thiscall `(scene; mesh,
/// cluster)`, `ret 8`) and `g_PhyreScene`.
pub const SCENE_DELETE_MESH_INSTANCE: u32 = 0x0065_a4c0;
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;

/// `ClassCharacterInstance_deleteObject_006a1550(this)` (thiscall): the
/// object at `+0x14` deleted from the cluster at `+0x2c`, `+0x14`
/// cleared. The delete's `eax`.
pub fn character_delete_object(h: &mut impl Host, this: u32) -> u32 {
    let obj: u32 = h.read(this + 0x14);
    let cluster: u32 = h.read(this + 0x2c);
    let r = h.call_sym(CLUSTER_DELETE_OBJECT, Conv::Thiscall(cluster), &[obj]);
    h.write(this + 0x14, 0u32);
    r
}

/// `ClassCharacterInstance_deleteMeshInstance_006a1570(this)` (thiscall):
/// the mesh instance at `+0xc` deleted from the scene (cluster `+0x2c`),
/// `+0xc` cleared. The delete's `eax`.
pub fn character_delete_mesh_instance(h: &mut impl Host, this: u32) -> u32 {
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let mesh: u32 = h.read(this + 0xc);
    let cluster: u32 = h.read(this + 0x2c);
    let r = h.call_sym(
        SCENE_DELETE_MESH_INSTANCE,
        Conv::Thiscall(scene),
        &[mesh, cluster],
    );
    h.write(this + 0xc, 0u32);
    r
}

/// `operator new` and `MSVCP110!std::_Xbad_alloc`'s import slot.
pub const OPERATOR_NEW: u32 = 0x0063_0310;
pub const XBAD_ALLOC_IAT: u32 = 0x00b0_c2a0;

/// `std_List_buyNode_00654f00(next, prev)` (stdcall, `ret 8`; no `ecx`
/// read): a 12-byte list node from `operator new` (`_Xbad_alloc` on
/// NULL), its links `next`/`prev`, or itself twice when `next` is 0. The
/// node.
pub fn list_buy_node(h: &mut impl Host, next: u32, prev: u32) -> u32 {
    let n = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[0xc]);
    if n == 0 {
        let f: u32 = h.read(h.rebase(XBAD_ALLOC_IAT));
        h.call(f, Conv::Cdecl, &[]);
        return 0;
    }
    let (a, b) = if next == 0 { (n, n) } else { (next, prev) };
    h.write(n, a);
    h.write(n + 4, b);
    n
}

/// `cd_channel_clear(n)` (cdecl) and `hdd_state_clear()` (cdecl).
pub const CD_CHANNEL_CLEAR: u32 = 0x0076_d530;
pub const HDD_STATE_CLEAR: u32 = 0x0077_7420;
/// The CD reader's words `cd_resetChannels_0076dcb0` zeroes, and its
/// byte `0x01128a84`.
pub const CD_WORDS: [u32; 6] = [
    0x0112_8acc,
    0x0112_8ad0,
    0x0112_8a88,
    0x0112_8a8c,
    0x0112_8ad4,
    0x0112_8a80,
];
pub const CD_BYTE: u32 = 0x0112_8a84;

/// `cd_resetChannels_0076dcb0()` (cdecl, after `cd_AllocReadBuffer`): the
/// 64 channels cleared, the reader's byte and six words zeroed, then a
/// tail jump to `hdd_state_clear`. Its `eax`.
pub fn cd_reset_channels(h: &mut impl Host) -> u32 {
    for i in 0..0x40 {
        h.call_sym(CD_CHANNEL_CLEAR, Conv::Cdecl, &[i]);
    }
    let at = h.rebase(CD_BYTE);
    h.write(at, 0u8);
    for w in CD_WORDS {
        let at = h.rebase(w);
        h.write(at, 0u32);
    }
    h.call_sym(HDD_STATE_CLEAR, Conv::Cdecl, &[])
}

/// `updateFFX`'s step `0x00822490(a, b)` (cdecl).
pub const ASYNC_INIT_STEP: u32 = 0x0082_2490;

/// `asyncFFXInitialize(out, &a, &b)` (`0x00636410`, cdecl): `*out =
/// 0x00822490(*a, *b)`. Its `eax`.
pub fn async_ffx_initialize(h: &mut impl Host, out: u32, a: u32, b: u32) -> u32 {
    let x: u32 = h.read(a);
    let y: u32 = h.read(b);
    let r = h.call_sym(ASYNC_INIT_STEP, Conv::Cdecl, &[x, y]);
    h.write(out, r);
    r
}

/// `PhyreScene`'s mesh instance cluster (`+0x11e4`).
pub const SCENE_MESH_CLUSTER: u32 = 0x11e4;

/// `ClassCharacterInstance_destroyMeshes(this)` (`0x006a15e0`, thiscall;
/// a `PArray` of mesh instances: `+0` the count with bit 31, `+4` the
/// items): each deleted from the scene's cluster
/// (`PhyreScene_deletePMeshInstanceInCluster`, the scene re-read each
/// time) and its slot cleared.
pub fn character_destroy_meshes(h: &mut impl Host, this: u32) {
    let mut i = 0u32;
    while i < h.read::<u32>(this) & 0x7fff_ffff {
        let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
        let items: u32 = h.read(this + 4);
        let mesh: u32 = h.read(items.wrapping_add(4 * i));
        h.call_sym(
            SCENE_DELETE_MESH_INSTANCE,
            Conv::Thiscall(scene),
            &[mesh, scene.wrapping_add(SCENE_MESH_CLUSTER)],
        );
        let items: u32 = h.read(this + 4);
        h.write(items.wrapping_add(4 * i), 0u32);
        i += 1;
    }
}

/// The cloth instance's teardown with its mesh (thiscall `(cloth;
/// mesh)`, `ret 4`).
pub const CLOTH_RELEASE: u32 = 0x006e_2e40;

/// `ClassCharacterInstance_destroyClothInstances(this)` (`0x006a1590`,
/// thiscall; the cloths `PArray` at `+8`/`+0xc`, the meshes' items at
/// `+4`): each cloth that is set released with its mesh, freed and its
/// slot cleared.
pub fn character_destroy_cloths(h: &mut impl Host, this: u32) {
    let mut i = 0u32;
    while i < h.read::<u32>(this + 8) & 0x7fff_ffff {
        let cloths: u32 = h.read(this + 0xc);
        let cloth: u32 = h.read(cloths.wrapping_add(4 * i));
        if cloth != 0 {
            let items: u32 = h.read(this + 4);
            let mesh: u32 = h.read(items.wrapping_add(4 * i));
            h.call_sym(CLOTH_RELEASE, Conv::Thiscall(cloth), &[mesh]);
            let cloths: u32 = h.read(this + 0xc);
            let p: u32 = h.read(cloths.wrapping_add(4 * i));
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
            let cloths: u32 = h.read(this + 0xc);
            h.write(cloths.wrapping_add(4 * i), 0u32);
        }
        i += 1;
    }
}

/// The party's limit records (0x94 bytes each from `0x01132000`; the
/// level byte `+0x97`, the gauge byte `+0x98`) and the clamp
/// `0x0079a0c0(value, low, high)` (cdecl, `al`).
pub const SLV_LEVEL: u32 = 0x0113_2097;
pub const SLV_GAUGE: u32 = 0x0113_2098;
pub const CLAMP: u32 = 0x0079_a0c0;

/// `spendSlv(who, n)` (`0x00786ef0`, cdecl): -1 when `who`'s (low byte)
/// level byte is below `n` (signed); else the level lowered by `n`, the
/// gauge raised by it and clamped to 0..=0x65; 0.
pub fn spend_slv(h: &mut impl Host, who: u32, n: u32) -> u32 {
    let e = (who & 0xff) * 0x94;
    let level_at = h.rebase(SLV_LEVEL) + e;
    let level: u8 = h.read(level_at);
    #[allow(clippy::cast_possible_wrap)] // the original's js
    if (u32::from(level).wrapping_sub(n) as i32) < 0 {
        return u32::MAX;
    }
    let gauge_at = h.rebase(SLV_GAUGE) + e;
    let gauge = u32::from(h.read::<u8>(gauge_at)).wrapping_add(n);
    #[allow(clippy::cast_possible_truncation)] // the original's byte maths
    h.write(level_at, level.wrapping_sub(n as u8));
    let r = h.call_sym(CLAMP, Conv::Cdecl, &[gauge, 0, 0x65]);
    #[allow(clippy::cast_possible_truncation)] // al
    h.write(gauge_at, r as u8);
    0
}

/// `PHeap_alloc_0042fb80(size, kind)` (cdecl) and the CRT `memcpy` thunk
/// (cdecl).
pub const PHYRE_ALLOC: u32 = 0x0042_fb80;
pub const MEMCPY: u32 = 0x0094_9256;

/// `NamedPair_copy_00699b70(this; &src)` (thiscall, `ret 4`): the word
/// copied; the name (`+4`, its low bit a flag that is dropped) copied into
/// a new Phyre block of its length plus the 0 (`PHYRE_ALLOC(n, 2)`, the
/// bytes by the `memcpy` thunk when the block is not NULL), else NULL.
/// `this`.
pub fn named_pair_copy(h: &mut impl Host, this: u32, src: u32) -> u32 {
    let w: u32 = h.read(src);
    h.write(this, w);
    let name: u32 = h.read(src + 4);
    let p = name.wrapping_sub(name & 1);
    if p == 0 {
        h.write(this + 4, 0u32);
        return this;
    }
    let mut n = 0u32;
    while h.read::<u8>(p.wrapping_add(n)) != 0 {
        n += 1;
    }
    let len = n + 1;
    let buf = h.call_sym(PHYRE_ALLOC, Conv::Cdecl, &[len, 2]);
    if buf != 0 {
        h.call_sym(MEMCPY, Conv::Cdecl, &[buf, p, len]);
    }
    h.write(this + 4, buf);
    this
}

/// The debug report `0x006dd5a0(this; name, tag, buffer, size)`
/// (thiscall, `ret 0x10`) and its `"MapLoad"` tag.
pub const REPORT: u32 = 0x006d_d5a0;
pub const MAP_LOAD_TAG: u32 = 0x00b4_a5e4;

/// `Instance_006dc800_reportMapLoad_006dcd50(this; name)` (thiscall, `ret
/// 4`): when the bytes `+0` and `+1` are set, a zeroed 0x100-byte buffer
/// (`frame`, game memory) and the report `(name, "MapLoad", buffer,
/// 0x100)`.
pub fn report_map_load(h: &mut impl Host, this: u32, name: u32, frame: u32) {
    if h.read::<u8>(this) == 0 || h.read::<u8>(this + 1) == 0 {
        return;
    }
    h.write_bytes(frame, &[0; 0x100]);
    let tag = h.rebase(MAP_LOAD_TAG);
    h.call_sym(REPORT, Conv::Thiscall(this), &[name, tag, frame, 0x100]);
}

/// `g_CharacterSwitchTweakUtil` (`0x00cccac4`), its size, the member
/// constructor at `+4` (thiscall) and the float at `0x00b482d8` its `+0xd8`
/// starts with.
pub const CHAR_SWITCH: u32 = 0x00cc_cac4;
pub const CHAR_SWITCH_SIZE: u32 = 0xf4;
pub const CHAR_SWITCH_MEMBER_CTOR: u32 = 0x006f_5b50;
pub const CHAR_SWITCH_D8: u32 = 0x00b4_82d8;

/// `CharacterSwitchTweakUtil_CharacterSwitchTweakUtil(this)` (`0x006b55d0`,
/// thiscall): the byte `+0` cleared, the member at `+4` constructed, then
/// `+0xd8` the constant, `+0xcc`, `+0xd0`, the byte `+0xd4` 0, `+0xdc`
/// 1280, `+0xe0` 720, `+0xe4`..`+0xec` 0.0, `+0xf0` 1.0, `+0x94`..`+0xac`
/// zeroed. `this`.
pub fn char_switch_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.write(this, 0u8);
    h.call_sym(CHAR_SWITCH_MEMBER_CTOR, Conv::Thiscall(this + 4), &[]);
    let c: u32 = h.read(h.rebase(CHAR_SWITCH_D8));
    h.write(this + 0xd8, c);
    h.write(this + 0xcc, 0u32);
    h.write(this + 0xd0, 0u32);
    h.write(this + 0xd4, 0u8);
    h.write(this + 0xdc, 0x500u32);
    h.write(this + 0xe0, 0x2d0u32);
    for off in [0xe4, 0xe8, 0xec] {
        h.write(this + off, 0u32);
    }
    h.write(this + 0xf0, 0x3f80_0000u32);
    for k in 0..7 {
        h.write(this + 0x94 + 4 * k, 0u32);
    }
    this
}

/// `CharacterSwitchTweakUtil_CreateInstance()` (`0x006b5eb0`, cdecl): once,
/// `operator new(0xf4)` and the constructor (NULL when the allocation
/// fails) into the global. The new instance; the original's `eax` when it
/// already exists is a stack address (its frame's), for which this
/// returns the instance (no caller reads it).
pub fn char_switch_create(h: &mut impl Host) -> u32 {
    let at = h.rebase(CHAR_SWITCH);
    let g: u32 = h.read(at);
    if g != 0 {
        return g;
    }
    let n = h.call_sym(OPERATOR_NEW, Conv::Cdecl, &[CHAR_SWITCH_SIZE]);
    let v = if n == 0 {
        0
    } else {
        h.call_sym(0x006b_55d0, Conv::Thiscall(n), &[])
    };
    h.write(at, v);
    v
}

/// `KERNEL32` import slots: `InitializeCriticalSectionEx`, `GetLastError`.
pub const INIT_CS_EX_IAT: u32 = 0x00b0_c048;
pub const GET_LAST_ERROR_IAT: u32 = 0x00b0_c030;
/// ATL's version GUID and the "the module's lock failed" byte.
pub const ATL_GUID: u32 = 0x00b7_0290;
pub const ATL_INIT_FAILED: u32 = 0x00cc_9c17;

/// `CComCriticalSection::Init(this)` (`0x009d9dfe`, thiscall):
/// `InitializeCriticalSectionEx(this, 0, 0)`; 0 (`S_OK`), or the last
/// error as an `HRESULT` (`0x8007xxxx`; 0 when it is not positive).
pub fn atl_cs_init(h: &mut impl Host, this: u32) -> u32 {
    let f: u32 = h.read(h.rebase(INIT_CS_EX_IAT));
    if h.call(f, Conv::Stdcall, &[this, 0, 0]) != 0 {
        return 0;
    }
    let f: u32 = h.read(h.rebase(GET_LAST_ERROR_IAT));
    let e = h.call(f, Conv::Stdcall, &[]);
    if e.cast_signed() > 0 {
        (e & 0xffff) | 0x8007_0000
    } else {
        e
    }
}

/// `CAtlBaseModule::CAtlBaseModule(this)` (`0x009d9c20`, thiscall; the
/// global at `0x0193fcc4` from `StaticInit_Ctor_00af1308`): the lock's 0x18
/// bytes (`+0x14`) and the words `+0x2c`..`+0x34` zeroed
/// (`_ATL_BASE_MODULE70`'s base, `0x009d9c82`), `cbSize` 0x38, both
/// instance handles the image base, version 0xb00, the version GUID; the
/// lock initialised ([`atl_cs_init`]), a failure noted in
/// [`ATL_INIT_FAILED`]. `this`.
pub fn atl_base_module_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.write_bytes(this + 0x14, &[0; 0x18]);
    for off in [0x2c, 0x30, 0x34] {
        h.write(this + off, 0u32);
    }
    let base = h.rebase(0x0040_0000);
    h.write(this, 0x38u32);
    h.write(this + 8, base);
    h.write(this + 4, base);
    h.write(this + 0xc, 0xb00u32);
    let guid = h.rebase(ATL_GUID);
    h.write(this + 0x10, guid);
    if atl_cs_init(h, this + 0x14).cast_signed() < 0 {
        let at = h.rebase(ATL_INIT_FAILED);
        h.write(at, 1u8);
    }
    this
}

/// `user_malloc(size)`, `user_free_thunk(p)` (cdecl).
pub const USER_MALLOC: u32 = 0x0068_7020;
pub const USER_FREE: u32 = 0x0068_7010;

/// `PtrArray_reserve_00638240(this; n)` (thiscall, `ret 4`; `+0` the count,
/// `+4` the capacity, `+8` the words): without words a block of `n` words
/// (`user_malloc`); else a new block (for `n > 0`, signed) with the
/// `count` words copied (the `memcpy` thunk; none when the block is
/// NULL), the old block freed (`user_free_thunk`); the capacity `n`. `eax`:
/// the first path's block, else the free's.
pub fn ptr_array_reserve(h: &mut impl Host, this: u32, n: u32) -> u32 {
    let old: u32 = h.read(this + 8);
    if old == 0 {
        let p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[n.wrapping_mul(4)]);
        h.write(this + 4, n);
        h.write(this + 8, p);
        return p;
    }
    let mut p = 0;
    if n.cast_signed() > 0 {
        p = h.call_sym(USER_MALLOC, Conv::Cdecl, &[n.wrapping_mul(4)]);
        if p != 0 {
            let count: u32 = h.read(this);
            h.call_sym(MEMCPY, Conv::Cdecl, &[p, old, count.wrapping_mul(4)]);
        }
    }
    let r = h.call_sym(USER_FREE, Conv::Cdecl, &[old]);
    h.write(this + 4, n);
    h.write(this + 8, p);
    r
}

/// `Table256_ctor_009f2510(this)` (thiscall; the static initialiser
/// `StaticInit_TailThiscall_00af2610`'s object): 256 entries of three words
/// from `+4`, each (0, 1.0, 0.0) (the original's two x87 loops write the
/// same values twice), and the count `+0` 0. `this`.
pub fn table256_ctor(h: &mut impl Host, this: u32) -> u32 {
    let mut bytes = Vec::with_capacity(12 * 256);
    for _ in 0..256 {
        bytes.extend(0u32.to_le_bytes());
        bytes.extend(1.0f32.to_bits().to_le_bytes());
        bytes.extend(0u32.to_le_bytes());
    }
    h.write_bytes(this + 4, &bytes);
    h.write(this, 0u32);
    this
}

/// The byte set once the parameter blocks got their slots
/// (`PParameterBlocks_assignSlots_004e4ec0`).
pub const SLOTS_ASSIGNED: u32 = 0x00ca_5fe8;
/// The list of scene render pass types (link at `+4`).
pub const PASS_TYPES: u32 = 0x00ca_54f8;
/// The number of pass types, written by [`pass_assign_indices`].
pub const PASS_COUNT: u32 = 0x00ca_5fec;
/// `PEffectVariant`'s class descriptor.
pub const EFFECT_VARIANT_CLASS: u32 = 0x00ca_5ff0;
/// `PSceneRenderPassType::InvalidateLookupIndices` (cdecl).
pub const INVALIDATE_LOOKUP_INDICES: u32 = 0x004d_f2d0;
/// `PClassDescriptor::setOverallocation` (thiscall, two stack words).
pub const SET_OVERALLOCATION: u32 = 0x0043_d590;

/// `PSceneRenderPassType_assignIndices_004e62c0()` (cdecl; the tail of
/// `PUtilityRendering::initialize`) -> 0x12 once the slots were assigned,
/// else 0: `InvalidateLookupIndices()`, each pass type's `+0xc` = its
/// position in the list, the count stored, and `PEffectVariant`'s
/// descriptor over-allocated by four bytes per pass type
/// (`setOverallocation(count * 4, 4)`).
pub fn pass_assign_indices(h: &mut impl Host) -> u32 {
    let assigned = h.rebase(SLOTS_ASSIGNED);
    if h.read::<u8>(assigned) != 0 {
        return 0x12;
    }
    h.call_sym(INVALIDATE_LOOKUP_INDICES, Conv::Cdecl, &[]);
    let list = h.rebase(PASS_TYPES);
    let mut node: u32 = h.read(list);
    let mut n = 0u32;
    while node != 0 {
        h.write(node + 0xc, n);
        node = h.read(node + 4);
        n = n.wrapping_add(1);
    }
    let count = h.rebase(PASS_COUNT);
    h.write(count, n);
    let class = h.rebase(EFFECT_VARIANT_CLASS);
    h.call_sym(
        SET_OVERALLOCATION,
        Conv::Thiscall(class),
        &[n.wrapping_mul(4), 4],
    );
    0
}

/// `Phyre_PhyreFree` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `PAnimationChannel`'s destructor (thiscall), 0x34-byte elements.
pub const CHANNEL_DTOR: u32 = 0x0050_d1d0;

/// The `eax` an owned-storage test leaves when the storage is borrowed:
/// `shr eax, 31` (1), then `not al`.
const BORROWED_EAX: u32 = 0xfe;

/// `PArray<PAnimationConstantChannel>::~PArray` (`0x00513240`, thiscall)
/// -> its `eax`: storage owned (count bit 31 clear) and not null has each
/// of its count elements destroyed (`PAnimationChannel`'s destructor,
/// 0x34 bytes apart) and is freed (`Phyre_PhyreFree`'s `eax`); then data
/// and count are 0. Borrowed storage leaves 0xfe, null storage 0.
pub fn constant_channel_array_dtor(h: &mut impl Host, this: u32) -> u32 {
    let count: u32 = h.read(this);
    let eax = if count & 0x8000_0000 != 0 {
        BORROWED_EAX
    } else {
        let data: u32 = h.read(this + 4);
        if data == 0 {
            0
        } else {
            let mut el = data;
            for _ in 0..count {
                h.call_sym(CHANNEL_DTOR, Conv::Thiscall(el), &[]);
                el = el.wrapping_add(0x34);
            }
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data])
        }
    };
    h.write(this + 4, 0u32);
    h.write(this, 0u32);
    eax
}

/// `PAnimationClip::~PAnimationClip` (`0x005133f0`, thiscall) -> its
/// `eax`: `+0` = 0; the word at `+0x1c` freed when its bit 0 is clear;
/// the constant channels at `+0xc` destroyed
/// ([`constant_channel_array_dtor`]); the array at `+4` (count, data)
/// freed when owned (bit 31 clear), then both words 0. `eax` =
/// the last free's, or 0xfe when the `+4` array is borrowed.
pub fn animation_clip_dtor(h: &mut impl Host, this: u32) -> u32 {
    h.write(this, 0u32);
    let name: u32 = h.read(this + 0x1c);
    if name & 1 == 0 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[name]);
    }
    constant_channel_array_dtor(h, this + 0xc);
    let count: u32 = h.read(this + 4);
    let eax = if count & 0x8000_0000 == 0 {
        let data: u32 = h.read(this + 8);
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data])
    } else {
        BORROWED_EAX
    };
    h.write(this + 4, 0u32);
    h.write(this + 8, 0u32);
    eax
}

/// `Phyre_PBase_GetClassDescriptor` (cdecl, no arguments).
pub const BASE_CLASS_DESCRIPTOR: u32 = 0x0043_5740;

/// `PAnimationChannel::PAnimationChannel` (`0x0050d150`, thiscall) ->
/// `this`: `+0` = `PBase::GetClassDescriptor()`, `+4` to `+0x20` = 0.
pub fn channel_ctor(h: &mut impl Host, this: u32) -> u32 {
    let desc = h.call_sym(BASE_CLASS_DESCRIPTOR, Conv::Cdecl, &[]);
    h.write(this, desc);
    h.write_bytes(this + 4, &[0; 0x20]);
    this
}

/// `PAnimationChannel::~PAnimationChannel` (`0x0050d1d0`, thiscall):
/// `+0x1c` = 0, then `Phyre_PhyreFree([+0x14])` (its `eax`) when that
/// word's bit 0 is clear. With bit 0 set the original returns its
/// caller's `eax` with `al` = the byte inverted; the port returns that
/// byte alone (no caller reads more than `al`: the `destructInstance`
/// copies set `al` = 1, the array destructors drop it).
pub fn channel_dtor(h: &mut impl Host, this: u32) -> u32 {
    h.write(this + 0x1c, 0u32);
    let name: u32 = h.read(this + 0x14);
    if name & 1 == 0 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[name])
    } else {
        u32::from(!(name as u8))
    }
}

/// `PMatrix4_Copy_004c9430` (thiscall `(out; m)`, `ret 4`).
pub const MATRIX_COPY: u32 = 0x004c_9430;
/// `Phyre_PBaseRef_ctor_005d3860` (thiscall).
pub const BASE_REF_CTOR: u32 = 0x005d_3860;
/// The image floats `PPhysicsWorld`'s defaults load (`fld` from memory):
/// `+0x28` (-19.6), then `+0xdc` to `+0xf0` (10, 60, 1.5, 0.8, 0.6, 5).
pub const PHYSICS_DEFAULTS: [(u32, u32); 7] = [
    (0x28, 0x00b3_ebbc),
    (0xdc, 0x00b9_2318),
    (0xe0, 0x00b3_eb9c),
    (0xe4, 0x00b3_eb58),
    (0xe8, 0x00b3_4254),
    (0xec, 0x00b3_eb48),
    (0xf0, 0x00b3_eb98),
];

/// The 16 words of an identity matrix.
fn identity_words() -> [u8; 64] {
    let mut m = [0u8; 64];
    for i in 0..4 {
        m[i * 20..i * 20 + 4].copy_from_slice(&1.0f32.to_bits().to_le_bytes());
    }
    m
}

/// `0x005e0050(this)` (thiscall; `Phyre_PPhysics_DestroyCall_005f7c30`
/// and the derived constructors call it; its defaults: gravity -19.6 on
/// `+0x28`) -> `this`: `+0` = 0, the floats `+4..+0x0c` 0, `+0x14..+0x1c`
/// 1, `+0x24` 0, `+0x28` -19.6, `+0x2c` and `+0x34..+0x3c` 0 (`+0x10`,
/// `+0x20`, `+0x30`, `+0x40` untouched); identity matrices copied into
/// `+0x44` and `+0x84` (`PMatrix4_Copy` from the 16-word `frame`, the
/// original's `ebp - 0x40`); `+0xc4..+0xd8` 0, the word `+0xf4` 0, the
/// floats `+0xdc..+0xf0` ([`PHYSICS_DEFAULTS`]), the byte `+0xf6` and
/// `+0xf8`, `+0xfc` 0; `PBaseRef`'s constructor on `+0x100`.
pub fn physics_defaults_ctor(h: &mut impl Host, this: u32, frame: u32) -> u32 {
    let one = 1.0f32.to_bits();
    h.write(this, 0u32);
    for off in [4, 8, 0xc, 0x24, 0x2c, 0x34, 0x38, 0x3c] {
        h.write(this + off, 0u32);
    }
    for off in [0x14, 0x18, 0x1c] {
        h.write(this + off, one);
    }
    let gravity = h.rebase(PHYSICS_DEFAULTS[0].1);
    let gravity: u32 = h.read(gravity);
    h.write(this + 0x28, gravity);
    for out in [0x44, 0x84] {
        h.write_bytes(frame, &identity_words());
        h.call_sym(MATRIX_COPY, Conv::Thiscall(this + out), &[frame]);
    }
    h.write_bytes(this + 0xc4, &[0; 0x18]);
    h.write(this + 0xf4, 0u16);
    for &(off, at) in &PHYSICS_DEFAULTS[1..] {
        let at = h.rebase(at);
        let v: u32 = h.read(at);
        h.write(this + off, v);
    }
    h.write(this + 0xf6, 0u8);
    h.write(this + 0xf8, 0u32);
    h.write(this + 0xfc, 0u32);
    h.call_sym(BASE_REF_CTOR, Conv::Thiscall(this + 0x100), &[]);
    this
}

/// `PPhysicsDefaults_ctor_005e0050` (thiscall).
pub const PHYSICS_DEFAULTS_CTOR: u32 = 0x005e_0050;

/// `0x005e01c0(this)` (thiscall; `Phyre_PPhysics_DestroyCall_005f7c80`'s
/// callee) -> `this`: [`PHYSICS_DEFAULTS_CTOR`], then `+0x10c..+0x118` 0
/// and an identity matrix copied into `+0x11c` (`PMatrix4_Copy` from the
/// 16-word `frame`, the original's `ebp - 0x40`).
pub fn physics_defaults_ctor_b(h: &mut impl Host, this: u32, frame: u32) -> u32 {
    h.call_sym(PHYSICS_DEFAULTS_CTOR, Conv::Thiscall(this), &[]);
    h.write_bytes(this + 0x10c, &[0; 0x10]);
    h.write_bytes(frame, &identity_words());
    h.call_sym(MATRIX_COPY, Conv::Thiscall(this + 0x11c), &[frame]);
    this
}

/// `PhyreCtor_zeroWord_0042fcc0` (thiscall).
pub const ZERO_WORD_CTOR: u32 = 0x0042_fcc0;
/// `Phyre_PSemaphoreWin32_PSemaphoreWin32` (thiscall).
pub const SEMAPHORE_CTOR: u32 = 0x0042_fc90;

/// `0x005e0330(this)` (thiscall; `Phyre_PPhysics_DestroyCall_005f7de0`'s
/// callee) -> `this`: the byte `+0` 0, `+4` 0, `+8` 0x10, `+0xc`, `+0x1c`,
/// `+0x20` 0; the one-word constructor on `+0x28`, the byte `+0x2c` 0, a
/// semaphore constructed at `+0x30`, the byte `+0x34` 0, one at `+0x38`,
/// the byte `+0x3c` 0; the float `+0x44` 0, `+0x40` 0, the word `+0x48` 0.
pub fn physics_header_ctor(h: &mut impl Host, this: u32) -> u32 {
    h.write(this, 0u8);
    h.write(this + 4, 0u32);
    h.write(this + 8, 0x10u32);
    for off in [0xc, 0x1c, 0x20] {
        h.write(this + off, 0u32);
    }
    h.call_sym(ZERO_WORD_CTOR, Conv::Thiscall(this + 0x28), &[]);
    h.write(this + 0x2c, 0u8);
    h.call_sym(SEMAPHORE_CTOR, Conv::Thiscall(this + 0x30), &[]);
    h.write(this + 0x34, 0u8);
    h.call_sym(SEMAPHORE_CTOR, Conv::Thiscall(this + 0x38), &[]);
    h.write(this + 0x3c, 0u8);
    h.write(this + 0x44, 0u32);
    h.write(this + 0x40, 0u32);
    h.write(this + 0x48, 0u16);
    this
}

/// `PMatrix4_toWorldMatrix3x4` (thiscall `(dst; m)`, `ret 4`).
pub const TO_WORLD_3X4: u32 = 0x0045_1190;
/// The image float `0x005e0590` loads for `+0xa8`/`+0xac` (0.5).
pub const PHYSICS_BODY_HALF: u32 = 0x00b9_2194;

/// `0x005e0590(this)` (thiscall; `Phyre_PPhysics_DestroyCall_005f80b0`'s
/// callee) -> `this`: `+0` = 0; an identity matrix in the 16-word `frame`
/// (the original's `ebp - 0x50`) made a 3x4 at `+4`
/// (`PMatrix4_toWorldMatrix3x4`); then the floats `+0x34..+0x3c`,
/// `+0x44..+0x4c` 0, `+0x50` 1, `+0x54..+0x5c`, `+0x64..+0x6c`,
/// `+0x74..+0x7c` 0, `+0x84..+0x8c` 1 (`+0x40`, `+0x60`, `+0x70`, `+0x80`,
/// `+0x90` untouched); `+0x94..+0xa4` 0; `+0xa8`, `+0xac` the image's 0.5
/// ([`PHYSICS_BODY_HALF`]); `+0xb0..+0xb8` 0, the byte `+0xbc` 0xf, the
/// float `+0xc0` 0, the word `+0xc4` 0x100, `+0xc8`, `+0xcc` 0;
/// `PBaseRef`'s constructor on `+0xd0`.
pub fn physics_body_ctor(h: &mut impl Host, this: u32, frame: u32) -> u32 {
    h.write_bytes(frame, &identity_words());
    h.write(this, 0u32);
    h.call_sym(TO_WORLD_3X4, Conv::Thiscall(this + 4), &[frame]);
    let one = 1.0f32.to_bits();
    for off in [0x34, 0x38, 0x3c, 0x44, 0x48, 0x4c] {
        h.write(this + off, 0u32);
    }
    h.write(this + 0x50, one);
    for off in [0x54, 0x58, 0x5c, 0x64, 0x68, 0x6c, 0x74, 0x78, 0x7c] {
        h.write(this + off, 0u32);
    }
    for off in [0x84, 0x88, 0x8c] {
        h.write(this + off, one);
    }
    h.write_bytes(this + 0x94, &[0; 0x14]);
    let half = h.rebase(PHYSICS_BODY_HALF);
    let half: u32 = h.read(half);
    h.write(this + 0xa8, half);
    h.write(this + 0xac, half);
    h.write_bytes(this + 0xb0, &[0; 0xc]);
    h.write(this + 0xbc, 0xfu8);
    h.write(this + 0xc0, 0u32);
    h.write(this + 0xc4, 0x100u16);
    h.write(this + 0xc8, 0u32);
    h.write(this + 0xcc, 0u32);
    h.call_sym(BASE_REF_CTOR, Conv::Thiscall(this + 0xd0), &[]);
    this
}

/// `PPhysicsRigidBodyBase::~PPhysicsRigidBodyBase` (`0x005e2660`,
/// thiscall) -> its `eax`: `Phyre_PhyreFree([+0xd0])` when that word's
/// bit 0 is clear; the `PSharray` at `+0xa0` (count, data) has its data
/// freed when owned (bit 31 clear) and holding more than one element
/// (one is inline); then `+0xa4` and `+0xa0` 0. `eax`: that free's, else
/// 0xfe for a borrowed array, 0xff for an owned one of at most one.
pub fn rigid_body_base_dtor(h: &mut impl Host, this: u32) -> u32 {
    let name: u32 = h.read(this + 0xd0);
    if name & 1 == 0 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[name]);
    }
    let count: u32 = h.read(this + 0xa0);
    let eax = if count & 0x8000_0000 != 0 {
        BORROWED_EAX
    } else if count > 1 {
        let data: u32 = h.read(this + 0xa4);
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data])
    } else {
        0xff
    };
    h.write(this + 0xa4, 0u32);
    h.write(this + 0xa0, 0u32);
    eax
}

/// `Phyre_PPhysics_PPhysicsModel_removeBody_00601a60` (thiscall `(model;
/// body)`).
pub const MODEL_REMOVE_BODY: u32 = 0x0060_1a60;
/// `Phyre_PPhysics_PPhysicsRigidBody_unlinkFromWorld_00601a20` (thiscall
/// `(body; head)`).
pub const BODY_UNLINK_FROM_WORLD: u32 = 0x0060_1a20;
/// `Phyre_PPhysics_PPhysicsRigidBodyBase_dtor_005e2660` (thiscall).
pub const RIGID_BODY_BASE_DTOR: u32 = 0x005e_2660;

/// `PPhysicsRigidBody::~PPhysicsRigidBody` (`0x005e2570`, thiscall) ->
/// the base destructor's `eax`: with a model (`+0xb0`) whose world
/// (`+8`) has a simulation object (`+0x54`) and a shape (`+0xdc`), the
/// object's virtual 20 `(shape)`; the body removed from the model
/// (`removeBody`) and, with a world, unlinked from its list (`world +
/// 0x50`); the shape's and `+0xe0`'s deleting destructors (virtuals 1 and
/// 0, argument 1); `+0xe0`, `+0xdc`, `+0xb0` 0; then
/// [`RIGID_BODY_BASE_DTOR`].
pub fn rigid_body_dtor(h: &mut impl Host, this: u32) -> u32 {
    let model: u32 = h.read(this + 0xb0);
    if model != 0 {
        let world: u32 = h.read(model + 8);
        if world != 0 {
            let sim: u32 = h.read(world + 0x54);
            let shape: u32 = h.read(this + 0xdc);
            if sim != 0 && shape != 0 {
                h.call_virtual(sim, 20, &[shape]);
            }
        }
        h.call_sym(MODEL_REMOVE_BODY, Conv::Thiscall(model), &[this]);
        let world: u32 = h.read(model + 8);
        if world != 0 {
            h.call_sym(
                BODY_UNLINK_FROM_WORLD,
                Conv::Thiscall(this),
                &[world + 0x50],
            );
        }
    }
    let shape: u32 = h.read(this + 0xdc);
    if shape != 0 {
        h.call_virtual(shape, 1, &[1]);
    }
    let other: u32 = h.read(this + 0xe0);
    if other != 0 {
        h.call_virtual(other, 0, &[1]);
    }
    h.write(this + 0xe0, 0u32);
    h.write(this + 0xdc, 0u32);
    h.write(this + 0xb0, 0u32);
    h.call_sym(RIGID_BODY_BASE_DTOR, Conv::Thiscall(this), &[])
}

/// The image floats `0x005e0900` loads: `+4` (-9.8), `+0x14..+0x1c`
/// (-5000), `+0x24..+0x2c` (5000).
pub const WORLD_GRAVITY: u32 = 0x00b3_ebb8;
/// See [`WORLD_GRAVITY`].
pub const WORLD_MIN: u32 = 0x00b3_ebc0;
/// See [`WORLD_GRAVITY`].
pub const WORLD_MAX: u32 = 0x00b3_eba0;
/// `PPhysicsWorldSettings_ctor_005e0900` (thiscall).
pub const WORLD_SETTINGS_CTOR: u32 = 0x005e_0900;

/// `0x005e0900(this)` (thiscall; `Phyre_PPhysics_DestroyCall_005f8350`'s
/// callee) -> `this`: the floats `+0` 0, `+4` the image's -9.8, `+8` 0,
/// `+0x10` 1, `+0x14..+0x1c` -5000, `+0x24..+0x2c` 5000 (`+0xc`, `+0x20`,
/// `+0x30` untouched); `+0x34..+0x50` 0.
pub fn world_settings_ctor(h: &mut impl Host, this: u32) -> u32 {
    let gravity = h.rebase(WORLD_GRAVITY);
    let gravity: u32 = h.read(gravity);
    let min = h.rebase(WORLD_MIN);
    let min: u32 = h.read(min);
    let max = h.rebase(WORLD_MAX);
    let max: u32 = h.read(max);
    h.write(this, 0u32);
    h.write(this + 4, gravity);
    h.write(this + 8, 0u32);
    h.write(this + 0x10, 1.0f32.to_bits());
    for off in [0x14, 0x18, 0x1c] {
        h.write(this + off, min);
    }
    for off in [0x24, 0x28, 0x2c] {
        h.write(this + off, max);
    }
    h.write_bytes(this + 0x34, &[0; 0x20]);
    this
}

/// `0x005e0970(this)` (thiscall; `Phyre_PPhysics_DestroyCall_005f8330`
/// and `_005f8370`'s callee) -> `this`: [`WORLD_SETTINGS_CTOR`], then
/// `+0x54..+0x5c` and `+0x80..+0xa4` 0, `+0x60..+0x78` all ones and the
/// word `+0x7c` 0xffff, the word `+0x62` with bit 1 cleared, the word
/// `+0x64` 3.
pub fn world_settings_ctor_b(h: &mut impl Host, this: u32) -> u32 {
    h.call_sym(WORLD_SETTINGS_CTOR, Conv::Thiscall(this), &[]);
    h.write_bytes(this + 0x54, &[0; 0xc]);
    h.write_bytes(this + 0x80, &[0; 0x28]);
    h.write_bytes(this + 0x60, &[0xff; 0x1e]);
    let w: u16 = h.read(this + 0x62);
    h.write(this + 0x62, w & 0xfffd);
    h.write(this + 0x64, 3u16);
    this
}

/// `Phyre_releaseParts_00605660` (thiscall).
pub const RELEASE_PARTS: u32 = 0x0060_5660;

/// `PPhysicsWorldBullet::~PPhysicsWorldBullet` (`0x005e2950`, thiscall)
/// -> its `eax`: `Phyre_releaseParts(this)`, then the arrays (count,
/// data) at `+0x9c`, `+0x94`, `+0x88`, `+0x80` in that order: data freed
/// when owned (count bit 31 clear) and not null, both words 0. `eax`: the
/// last array's free, else 0xfe (borrowed) or 0 (null data).
pub fn world_bullet_dtor(h: &mut impl Host, this: u32) -> u32 {
    h.call_sym(RELEASE_PARTS, Conv::Thiscall(this), &[]);
    let mut eax = 0;
    for off in [0x9c, 0x94, 0x88, 0x80] {
        let count: u32 = h.read(this + off);
        eax = if count & 0x8000_0000 != 0 {
            BORROWED_EAX
        } else {
            let data: u32 = h.read(this + off + 4);
            if data == 0 {
                0
            } else {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data])
            }
        };
        h.write(this + off + 4, 0u32);
        h.write(this + off, 0u32);
    }
    eax
}

/// `Phyre_releaseGroupResources_006019b0` (thiscall).
pub const RELEASE_GROUP_RESOURCES: u32 = 0x0060_19b0;
/// `Phyre_PPhysics_PPhysicsWorld_removeController_006016c0` (thiscall
/// `(world; ctrl)`).
pub const WORLD_REMOVE_CONTROLLER: u32 = 0x0060_16c0;
/// `Phyre_PPhysics_PPhysicsWorldBullet_dtor_005e2950` (thiscall).
pub const WORLD_BULLET_DTOR: u32 = 0x005e_2950;

/// `PPhysicsWorld::~PPhysicsWorld` (`0x005e28d0`, thiscall) -> the
/// Bullet part's destructor's `eax`: `Phyre_releaseGroupResources(this)`;
/// each controller of the list at `+0x4c` (next at `+0`, read first)
/// removed (`removeController`) and its `+0xcc` cleared, stopping at the
/// first non-zero result; then [`WORLD_BULLET_DTOR`].
pub fn world_dtor(h: &mut impl Host, this: u32) -> u32 {
    h.call_sym(RELEASE_GROUP_RESOURCES, Conv::Thiscall(this), &[]);
    let mut node: u32 = h.read(this + 0x4c);
    while node != 0 {
        let next: u32 = h.read(node);
        let r = h.call_sym(WORLD_REMOVE_CONTROLLER, Conv::Thiscall(this), &[node]);
        if r != 0 {
            break;
        }
        h.write(node + 0xcc, 0u32);
        node = next;
    }
    h.call_sym(WORLD_BULLET_DTOR, Conv::Thiscall(this), &[])
}

/// `Phyre_PPhysics_PPhysicsShapeBase_vftable`.
pub const SHAPE_BASE_VTABLE: u32 = 0x00b3_9508;

/// `PPhysicsShapeBase::PPhysicsShapeBase` (`0x005e0740`, thiscall; the
/// shapes' base constructor, 48 callers) -> `this`: the vtable
/// ([`SHAPE_BASE_VTABLE`]), the byte `+4` 0, the floats `+8`, `+0xc` 0,
/// `+0x10` 0; an identity matrix in the 16-word `frame` (the original's
/// `ebp - 0x40`) made a 3x4 at `+0x14` (`PMatrix4_toWorldMatrix3x4`);
/// the floats `+0x44..+0x4c` 1 (`+0x50` untouched), `+0x54` 8.
pub fn shape_base_ctor(h: &mut impl Host, this: u32, frame: u32) -> u32 {
    h.write(this + 8, 0u32);
    h.write(this + 0xc, 0u32);
    let vt = h.rebase(SHAPE_BASE_VTABLE);
    h.write(this, vt);
    h.write(this + 4, 0u8);
    h.write(this + 0x10, 0u32);
    h.write_bytes(frame, &identity_words());
    h.call_sym(TO_WORLD_3X4, Conv::Thiscall(this + 0x14), &[frame]);
    for off in [0x44, 0x48, 0x4c] {
        h.write(this + off, 1.0f32.to_bits());
    }
    h.write(this + 0x54, 8u32);
    this
}

/// `Phyre_PPhysics_PPhysicsMeshBullet_vftable`.
pub const MESH_BULLET_VTABLE: u32 = 0x00b3_9ee8;

/// `PPhysicsMeshBullet::~PPhysicsMeshBullet` (`0x005e2360`, thiscall):
/// the class's vtable ([`MESH_BULLET_VTABLE`]); the objects at `+0x60`,
/// `+0x64`, `+0x68` deleted when set (virtual 0, argument 1); those three
/// and `+0x58` 0; then `PPhysicsShapeBase`'s vtable. `eax`: the last
/// deletion's. With none the original leaves a stack address (its SEH
/// record's) in `eax`; the port returns 0 there (both callers, the
/// scalar deleting destructors 0x005e6c90 and 0x005e6cf0, return `this`;
/// the thunk 0x005e2340 has no reference).
pub fn mesh_bullet_dtor(h: &mut impl Host, this: u32) -> u32 {
    let vt = h.rebase(MESH_BULLET_VTABLE);
    h.write(this, vt);
    let mut eax = 0;
    for off in [0x60, 0x64, 0x68] {
        let obj: u32 = h.read(this + off);
        if obj != 0 {
            eax = h.call_virtual(obj, 0, &[1]);
        }
    }
    for off in [0x60, 0x64, 0x68, 0x58] {
        h.write(this + off, 0u32);
    }
    let vt = h.rebase(SHAPE_BASE_VTABLE);
    h.write(this, vt);
    eax
}

/// `Phyre_PPhysics_PPhysicsCapsuleBullet_vftable`.
pub const CAPSULE_BULLET_VTABLE: u32 = 0x00b3_a4e0;
/// `Phyre_PPhysics_PPhysicsCylinderBullet_vftable`.
pub const CYLINDER_BULLET_VTABLE: u32 = 0x00b3_a1e0;

/// `PPhysicsCapsuleBullet::~PPhysicsCapsuleBullet` (`0x005e2030`,
/// thiscall), the cylinder's destructor inlined: the capsule's vtable,
/// `+0x6c` deleted when set (virtual 0, argument 1), `+0x6c` and `+0x58`
/// 0; the cylinder's vtable, `+0x68` the same way, `+0x68` and `+0x58` 0;
/// `PPhysicsShapeBase`'s vtable. `eax`: the last deletion's; with none a
/// stack address in the original, 0 in the port (the callers, the scalar
/// deleting destructors 0x005e6800 and 0x005e68c0, return `this`; the
/// thunk 0x005e1fb0 has no reference).
pub fn capsule_bullet_dtor(h: &mut impl Host, this: u32) -> u32 {
    let mut eax = 0;
    for (vtable, off) in [
        (CAPSULE_BULLET_VTABLE, 0x6c),
        (CYLINDER_BULLET_VTABLE, 0x68),
    ] {
        let vt = h.rebase(vtable);
        h.write(this, vt);
        let obj: u32 = h.read(this + off);
        if obj != 0 {
            eax = h.call_virtual(obj, 0, &[1]);
        }
        h.write(this + off, 0u32);
        h.write(this + 0x58, 0u32);
    }
    let vt = h.rebase(SHAPE_BASE_VTABLE);
    h.write(this, vt);
    eax
}

/// `Phyre_PPhysics_PPhysicsWorld_removeModel_006018b0` (thiscall `(world;
/// model)`).
pub const WORLD_REMOVE_MODEL: u32 = 0x0060_18b0;

/// The link at `head` that points at `item` (links `next` bytes into each
/// item) replaced by `item`'s own; `item`'s link 0. Nothing when `item`
/// is not on the list.
fn unlink(h: &mut impl Host, head: u32, item: u32, next: u32) {
    let mut slot = head;
    let mut cur: u32 = h.read(slot);
    while cur != 0 {
        if cur == item {
            let after: u32 = h.read(cur + next);
            h.write(slot, after);
            h.write(cur + next, 0u32);
            return;
        }
        slot = cur + next;
        cur = h.read(slot);
    }
}

/// `PPhysicsModel::~PPhysicsModel` (`0x005e23f0`, thiscall) -> `this +
/// 4`: with a world (`+8`, read once) the model removed from it
/// (`removeModel`); then each body of the model's list (`+4`, linked
/// through `+0`; the next read first) gets `+0xb0` 0, is unlinked from
/// the model's list and, with a world, from the world's body list
/// (`world + 0x50`, linked through `+0xb8`).
pub fn model_dtor(h: &mut impl Host, this: u32) -> u32 {
    let world: u32 = h.read(this + 8);
    if world != 0 {
        h.call_sym(WORLD_REMOVE_MODEL, Conv::Thiscall(world), &[this]);
    }
    let head = this + 4;
    let mut body: u32 = h.read(head);
    while body != 0 {
        let next: u32 = h.read(body);
        h.write(body + 0xb0, 0u32);
        unlink(h, head, body, 0);
        if world != 0 {
            unlink(h, world + 0x50, body, 0xb8);
        }
        body = next;
    }
    head
}
