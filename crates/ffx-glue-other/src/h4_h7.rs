//! Continuation round 6 of list `h4-glue-other`, batch 7: the movie
//! player manager's and the Win32 video playback's constructors.

use ffx_graphic::host::{Conv, Host};

/// `CDataFrameQueue_ctor_00a3dd20(queue)` (thiscall).
pub const FRAME_QUEUE_CTOR: u32 = 0x00a3_dd20;
/// `PhyreCtor_zeroWord_0042fcc0(this)` (thiscall).
pub const ZERO_WORD_CTOR: u32 = 0x0042_fcc0;
/// `VbfMkvReader_ctor_00a3e030(reader)` (thiscall).
pub const MKV_READER_CTOR: u32 = 0x00a3_e030;
/// `Phyre_PEventFlagWin32_PEventFlagWin32(flag)` (thiscall).
pub const EVENT_FLAG_CTOR: u32 = 0x0042_fc80;

/// `PVideoPlaybackWin32_ctor(this)` (`0x00a26e70`, thiscall): three frame
/// queues (`+0`, `+0x40`, `+0x80`), two zero-word objects (`+0xc0`,
/// `+0xc8`) each followed by a cleared byte (`+0xc4`, `+0xcc`), the MKV
/// reader (`+0x118`), two event flags (`+0x2ac`, `+0x2b4`) each followed
/// by a cleared byte (`+0x2b0`, `+0x2b8`). Returns `this`. The SEH frame
/// is not kept.
pub fn video_playback_ctor(h: &mut impl Host, this: u32) -> u32 {
    for off in [0u32, 0x40, 0x80] {
        h.call_sym(
            FRAME_QUEUE_CTOR,
            Conv::Thiscall(this.wrapping_add(off)),
            &[],
        );
    }
    for off in [0xc0u32, 0xc8] {
        h.call_sym(ZERO_WORD_CTOR, Conv::Thiscall(this.wrapping_add(off)), &[]);
        h.write(this.wrapping_add(off + 4), 0u8);
    }
    h.call_sym(
        MKV_READER_CTOR,
        Conv::Thiscall(this.wrapping_add(0x118)),
        &[],
    );
    for off in [0x2acu32, 0x2b4] {
        h.call_sym(EVENT_FLAG_CTOR, Conv::Thiscall(this.wrapping_add(off)), &[]);
        h.write(this.wrapping_add(off + 4), 0u8);
    }
    this
}

/// `PVideoPlaybackWin32_ctor` (the playback at `+0x38`).
pub const VIDEO_PLAYBACK_CTOR: u32 = 0x00a2_6e70;
/// `Phyre_PCluster_operator_new(this)` (`PCluster`'s constructor).
pub const PCLUSTER_CTOR: u32 = crate::h4_h1::PCLUSTER_CTOR;
/// `PSamplerStateD3D11_ctor(this)` (thiscall).
pub const SAMPLER_STATE_CTOR: u32 = crate::h4_h1::SAMPLER_STATE_CTOR;
/// `Phyre_PRendering_PTexture2DD3D11_PTexture2DD3D11(this)` (thiscall).
pub const TEXTURE_CTOR: u32 = 0x0058_4240;
/// `Phyre_PCameraPerspective_PCameraPerspective(this)` (thiscall).
pub const CAMERA_CTOR: u32 = 0x0045_6370;
/// `TreeHead_new_006d78c0()`: an empty tree's head node.
pub const TREE_HEAD_NEW: u32 = 0x006d_78c0;
/// `TreeNode_new_006d78f0(a, b)` (ret 8): a list's head node.
pub const TREE_NODE_NEW: u32 = 0x006d_78f0;

/// `PhyFMVPlayerManager_PhyFMVPlayerManager(this)` (`0x006d6970`,
/// thiscall): words `+0`..`+0x30` 0 but `+0x2c` 2; the playback (`+0x38`),
/// the cluster `+0x310` (its list head `+0x358` pointing at itself), the
/// sampler `+0x364`, `+0x388` 0, the three plane textures (`+0x38c`,
/// `+0x400`, `+0x474`), the small array `+0x4e8` empty, byte `+0x570` 0,
/// the camera `+0x574`; the flags and counters from `+0x6d0`; a name map
/// (`+0x6e8`, size `+0x6ec`) and a list (`+0x724`, size `+0x728`) with
/// new head nodes; `+0x700`..`+0x70c` and `+0x744` 1. Returns `this`.
/// The SEH frame is not kept.
pub fn fmv_manager_ctor(h: &mut impl Host, this: u32) -> u32 {
    for k in 0..13u32 {
        h.write(this.wrapping_add(4 * k), if k == 11 { 2u32 } else { 0 });
    }
    h.call_sym(
        VIDEO_PLAYBACK_CTOR,
        Conv::Thiscall(this.wrapping_add(0x38)),
        &[],
    );
    h.call_sym(PCLUSTER_CTOR, Conv::Thiscall(this.wrapping_add(0x310)), &[]);
    let head = this.wrapping_add(0x358);
    h.write(head, head);
    h.write(head.wrapping_add(4), head);
    h.call_sym(
        SAMPLER_STATE_CTOR,
        Conv::Thiscall(this.wrapping_add(0x364)),
        &[],
    );
    h.write(this.wrapping_add(0x388), 0u32);
    for off in [0x38cu32, 0x400, 0x474] {
        h.call_sym(TEXTURE_CTOR, Conv::Thiscall(this.wrapping_add(off)), &[]);
    }
    h.write(this.wrapping_add(0x4e8), 0u32);
    h.write(this.wrapping_add(0x4ec), 0u32);
    h.write(this.wrapping_add(0x570), 0u8);
    h.call_sym(CAMERA_CTOR, Conv::Thiscall(this.wrapping_add(0x574)), &[]);
    h.write(this.wrapping_add(0x6d0), 0u8);
    h.write(this.wrapping_add(0x6d2), 0u8);
    h.write(this.wrapping_add(0x6d4), 0u32);
    h.write(this.wrapping_add(0x6d8), 0u32);
    h.write(this.wrapping_add(0x6e4), 0u16);
    let map = this.wrapping_add(0x6e8);
    h.write(map, 0u32);
    h.write(map.wrapping_add(4), 0u32);
    let node = h.call_sym(TREE_HEAD_NEW, Conv::Thiscall(map), &[]);
    h.write(map, node);
    h.write(this.wrapping_add(0x6f0), 0u32);
    h.write(this.wrapping_add(0x6fc), 0u16);
    h.write(this.wrapping_add(0x6fe), 0u8);
    h.write(this.wrapping_add(0x710), 0u8);
    h.write(this.wrapping_add(0x720), 0u8);
    let list = this.wrapping_add(0x724);
    h.write(list, 0u32);
    h.write(list.wrapping_add(4), 0u32);
    let node = h.call_sym(TREE_NODE_NEW, Conv::Thiscall(list), &[0, 0]);
    h.write(list, node);
    h.write(this.wrapping_add(0x72c), 0u32);
    h.write(this.wrapping_add(0x730), 0u16);
    h.write(this.wrapping_add(0x732), 0u8);
    h.write(this.wrapping_add(0x744), 1u32);
    h.write(this.wrapping_add(0x748), 0u32);
    h.write(this.wrapping_add(0x74c), 0u16);
    for off in [0x700u32, 0x704, 0x708, 0x70c] {
        h.write(this.wrapping_add(off), 1u32);
    }
    this
}

// ---- an Iggy GDraw callback

/// `_IggyGDrawMallocAnnotated@12(size, file, line)` (import slot,
/// stdcall).
pub const GDRAW_MALLOC: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00b0_c86c);
/// `_IggyGDrawFree@4(block)` (import slot, stdcall).
pub const GDRAW_FREE: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00b0_c870);
/// The source file `GDraw` names in its allocations (`0x00b7a5e8`).
pub const GDRAW_SOURCE: u32 = 0x00b7_a5e8;
/// `GDraw`'s state (`0x01a858b4`).
pub const GDRAW_STATE: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x01a8_58b4);
/// `opd_config_memory(pool, size, c)` (cdecl).
pub const CONFIG_MEMORY: u32 = 0x00a3_b210;

/// `gdraw_allocBuffers_00a3a540(a, unused, n0, n1, out, c)` (stdcall, ret
/// 0x18; a callback `GDraw`'s set-up `0x00a35780` stores): two blocks
/// (`_IggyGDrawMallocAnnotated(n, file, 0x266 / 0x267)`) into `out` and
/// `out + 4`; with both, a record from the state's pool (`+0xab4`;
/// `opd_config_memory(pool, n0 + n1, c)`) cleared and stored at `out +
/// 0x10`, with `+8` n0, `+0xc` n1, `+0x14` a: 1. Otherwise every block
/// obtained is freed (the first only when the second came and the record
/// did not): 0.
pub fn gdraw_alloc_buffers(h: &mut impl Host, a: u32, n0: u32, n1: u32, out: u32, c: u32) -> u32 {
    let malloc = GDRAW_MALLOC.get(h);
    let source = h.rebase(GDRAW_SOURCE);
    let p0 = h.call(malloc, Conv::Stdcall, &[n0, source, 0x266]);
    h.write(out, p0);
    let malloc = GDRAW_MALLOC.get(h);
    let p1 = h.call(malloc, Conv::Stdcall, &[n1, source, 0x267]);
    h.write(out.wrapping_add(4), p1);
    if h.read::<u32>(out) != 0 {
        if p1 != 0 {
            let state = GDRAW_STATE.get(h);
            let pool: u32 = h.read(state.wrapping_add(0xab4));
            let record = h.call_sym(CONFIG_MEMORY, Conv::Cdecl, &[pool, n1.wrapping_add(n0), c]);
            if record != 0 {
                h.write(record, 0u32);
                h.write(record.wrapping_add(4), 0u32);
                h.write(out.wrapping_add(0x10), record);
                h.write(out.wrapping_add(8), n0);
                h.write(out.wrapping_add(0x14), a);
                h.write(out.wrapping_add(0xc), n1);
                return 1;
            }
        }
        let first: u32 = h.read(out);
        if first != 0 {
            let free = GDRAW_FREE.get(h);
            h.call(free, Conv::Stdcall, &[first]);
        }
    }
    let second: u32 = h.read(out.wrapping_add(4));
    if second != 0 {
        let free = GDRAW_FREE.get(h);
        h.call(free, Conv::Stdcall, &[second]);
    }
    0
}
