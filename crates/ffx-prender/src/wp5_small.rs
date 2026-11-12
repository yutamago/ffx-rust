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
