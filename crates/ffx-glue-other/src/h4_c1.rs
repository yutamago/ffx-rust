//! Continuation round of list `h4-glue-other`, batch 1: the accumulation
//! buffer's GS quads (`Sg_*`), menu record and name lookups, array
//! helpers, the FMOD event collectors, the character cache's
//! delayed-release flag, the escape menu's page state and its string
//! arrays, the texture-animation records, a mip-level dispatcher and an
//! icon packet.
//!
//! Bridges that hand callees the addresses of their own locals take
//! `frame` as in [`crate::frames`]: a block laid out as the original's
//! frame from its lowest local up. Bridges that hand callees the address
//! of their own argument words take `args`, the address of the first one.

use ffx_bridge::Global;
use ffx_graphic::host::{Conv, Host};

use crate::frames::{OPERATOR_DELETE, RELEASE_MUTEX, WAIT_FOR_SINGLE_OBJECT};
use crate::menu::{DATA_VALUE_SET_BOOL, PHYRE_FREE};
use crate::r3::{ALIGNED_ALLOC, CALL_FLASH_1};

// ---- the accumulation buffer's GS quads

/// `Sg_GsTEXFLUSH()` (cdecl, as all the `Sg_Gs*` below).
pub const GS_TEXFLUSH: u32 = 0x0084_1ee0;
/// `Sg_GsGet()`: the packet cursor.
pub const GS_GET: u32 = 0x0084_1a30;
/// `Sg_GsInc()`.
pub const GS_INC: u32 = 0x0084_1a50;
/// `Sg_GsTexHokan(on)`.
pub const GS_TEX_HOKAN: u32 = 0x0084_1f10;
/// `Sg_GsALPHA(a, b, c, d, fix)`.
pub const GS_ALPHA: u32 = 0x0084_1670;
/// `Sg_GsPRIM(prim)`.
pub const GS_PRIM: u32 = 0x0084_1a60;
/// `Sg_GsUV(u, v)`.
pub const GS_UV: u32 = 0x0084_1f60;
/// `Sg_GsXYZF2(x, y)`.
pub const GS_XYZF2: u32 = 0x0084_2140;
/// `Sg_GsUV2(u, v)`.
pub const GS_UV2: u32 = 0x0084_1fb0;
/// `Sg_GsRGBAQ(r, g, b, a)`.
pub const GS_RGBAQ: u32 = 0x0084_1aa0;
/// `Sg_GsXYZF2V(x, y, z)`.
pub const GS_XYZF2V: u32 = 0x0084_2210;
/// `Sg_GsSetPacket(p)`.
pub const GS_SET_PACKET: u32 = crate::misc::GS_SET_PACKET;
/// `Sg_GsStart()`.
pub const GS_START: u32 = 0x0084_1d10;
/// `Sg_GsFRAME_16(fbp, mask)`.
pub const GS_FRAME_16: u32 = crate::misc::GS_FRAME_16;
/// `Sg_GsFRAME(fbp, mask)`.
pub const GS_FRAME: u32 = 0x0084_1870;
/// `Sg_GsEnd()`.
pub const GS_END: u32 = 0x0084_1770;
/// The draw buffer's GS address (`0x012ffafc`; `ffx_sg::gs` `DRAW_FBP`).
pub const DRAW_FBP: Global<u32> = Global::new(0x012f_fafc);
/// `g_AccCurrentRequest` (`0x012ffae4`).
pub const ACC_CURRENT_REQUEST: Global<u32> = Global::new(0x012f_fae4);
/// The filter packet `sg_AccSetCurrent` builds (`0x012ffb00`).
pub const FILTER_PACKET: u32 = 0x012f_fb00;

/// The `TEX0` word pair a quad sets: the first word's kept bits and the
/// bits set in it (the base pointer added by the caller), and the same for
/// the second.
const TEX0_LO_KEEP: u32 = 0xe412_0000;
const TEX0_LO_SET: u32 = 0x2412_0000;

/// Writes the texture register pair of the packet at `p`: the first word
/// `lo`, the second `([p+4] & 0xffffffa9) | 0x29`, then the address
/// words 6 and 0.
fn tex0(h: &mut impl Host, p: u32, lo: u32) {
    let hi = (h.read::<u32>(p.wrapping_add(4)) & 0xffff_ffa9) | 0x29;
    h.write(p, lo);
    h.write(p.wrapping_add(4), hi);
    h.write(p.wrapping_add(8), 6u32);
    h.write(p.wrapping_add(0xc), 0u32);
}

/// `Sg_AccDrawAlphaQuad_0082baf0(fix)` (cdecl): a 512x416 sprite over the
/// screen with the accumulation texture (`TEX0` word `([p] & 0xe4a21a00)
/// | 0x24a21a00`), no filtering, blend `(Cs - Cd) * fix` (`Sg_GsALPHA(0,
/// 1, 2, 1, fix)`), prim 0x156. Returns the last `Sg_GsXYZF2`'s `eax`.
pub fn acc_draw_alpha_quad(h: &mut impl Host, fix: u32) -> u32 {
    h.call_sym(GS_TEXFLUSH, Conv::Cdecl, &[]);
    let p = h.call_sym(GS_GET, Conv::Cdecl, &[]);
    let lo = (h.read::<u32>(p) & 0xe4a2_1a00) | 0x24a2_1a00;
    tex0(h, p, lo);
    h.call_sym(GS_INC, Conv::Cdecl, &[]);
    h.call_sym(GS_TEX_HOKAN, Conv::Cdecl, &[0]);
    h.call_sym(GS_ALPHA, Conv::Cdecl, &[0, 1, 2, 1, fix]);
    h.call_sym(GS_PRIM, Conv::Cdecl, &[0x156]);
    h.call_sym(GS_UV, Conv::Cdecl, &[0, 0]);
    h.call_sym(GS_XYZF2, Conv::Cdecl, &[0, 0]);
    h.call_sym(GS_UV2, Conv::Cdecl, &[0x200, 0x1a0]);
    h.call_sym(GS_XYZF2, Conv::Cdecl, &[0x200, 0x1a0])
}

/// `Sg_DrawFilterQuad_0082bb90(z)` (cdecl): a 510x414 sprite at depth `z`
/// with the draw buffer as texture (`TEX0` base `(fbp >> 6) & 0x3fff`),
/// bilinear, blend fix 0x40, colour `(0xff, 0, 0, 0)`, UVs from (2, 2).
/// Returns the last `Sg_GsXYZF2V`'s `eax`.
pub fn draw_filter_quad(h: &mut impl Host, z: u32) -> u32 {
    h.call_sym(GS_TEXFLUSH, Conv::Cdecl, &[]);
    let p = h.call_sym(GS_GET, Conv::Cdecl, &[]);
    let base = (DRAW_FBP.get(h) >> 6) & 0x3fff;
    let lo = base | (h.read::<u32>(p) & TEX0_LO_KEEP) | TEX0_LO_SET;
    tex0(h, p, lo);
    h.call_sym(GS_INC, Conv::Cdecl, &[]);
    h.call_sym(GS_TEX_HOKAN, Conv::Cdecl, &[1]);
    h.call_sym(GS_ALPHA, Conv::Cdecl, &[0, 1, 2, 1, 0x40]);
    h.call_sym(GS_RGBAQ, Conv::Cdecl, &[0xff, 0, 0, 0]);
    h.call_sym(GS_PRIM, Conv::Cdecl, &[0x156]);
    h.call_sym(GS_UV, Conv::Cdecl, &[2, 2]);
    h.call_sym(GS_XYZF2V, Conv::Cdecl, &[0, 0, z]);
    h.call_sym(GS_UV2, Conv::Cdecl, &[0x200, 0x1a0]);
    h.call_sym(GS_XYZF2V, Conv::Cdecl, &[0x1fe, 0x19e, z])
}

/// `sg_AccSetCurrent()` (`0x0082ce90`, cdecl): with a request pending
/// (`g_AccCurrentRequest`), the filter packet copies the draw buffer to
/// the accumulation buffer: `Sg_GsSetPacket(0x012ffb00)`, `Sg_GsStart`,
/// `Sg_GsFRAME_16(0xd0, 0)`, a 512x416 sprite with the draw buffer as
/// texture (first `TEX0` word `((fbp >> 6) & 0x3fff) | 0x24120000` over
/// a cleared pair, second 0x29), prim 0x116, `Sg_GsFRAME(fbp >> 11, 0)`
/// (`fbp` read again); then the request cleared and `Sg_GsEnd()`. The
/// one caller (`Sg_MainLoop`) reads no result.
pub fn acc_set_current(h: &mut impl Host) {
    if ACC_CURRENT_REQUEST.get(h) == 0 {
        return;
    }
    let packet = h.rebase(FILTER_PACKET);
    h.call_sym(GS_SET_PACKET, Conv::Cdecl, &[packet]);
    h.call_sym(GS_START, Conv::Cdecl, &[]);
    h.call_sym(GS_FRAME_16, Conv::Cdecl, &[0xd0, 0]);
    h.call_sym(GS_TEXFLUSH, Conv::Cdecl, &[]);
    let p = h.call_sym(GS_GET, Conv::Cdecl, &[]);
    let base = (DRAW_FBP.get(h) >> 6) & 0x3fff;
    // the pair is cleared first, so nothing of it survives
    h.write(p, 0u32);
    h.write(p.wrapping_add(4), 0u32);
    tex0(h, p, base | TEX0_LO_SET);
    h.call_sym(GS_INC, Conv::Cdecl, &[]);
    h.call_sym(GS_TEX_HOKAN, Conv::Cdecl, &[0]);
    h.call_sym(GS_PRIM, Conv::Cdecl, &[0x116]);
    h.call_sym(GS_UV, Conv::Cdecl, &[0, 0]);
    h.call_sym(GS_XYZF2, Conv::Cdecl, &[0, 0]);
    h.call_sym(GS_UV2, Conv::Cdecl, &[0x200, 0x1a0]);
    h.call_sym(GS_XYZF2, Conv::Cdecl, &[0x200, 0x1a0]);
    let fbp = DRAW_FBP.get(h) >> 11;
    h.call_sym(GS_FRAME, Conv::Cdecl, &[fbp, 0]);
    ACC_CURRENT_REQUEST.set(h, 0);
    h.call_sym(GS_END, Conv::Cdecl, &[]);
}

// ---- menu records and names

/// `MenuManager_TakeSingleRecord_0067c210(this; chain)` (thiscall, ret
/// 4): as `MenuManager_TakeMultiRecord_0067c170` without the busy byte:
/// chain 0 takes the next record of the pool at `+0x15e0`/`+0x15e4` (used
/// `+0x2bf8`) into the list at `+0` (count `+0x2c50`), chain 1 the pool at
/// `+0x4238`/`+0x423c` (used `+0x5838`) into the list at `+0x2c58` (count
/// `+0x5840`). The record is read again after the store (the list can
/// overlap the pool's words). Returns it, or `chain` when the pool is used
/// up or the chain is another.
pub fn take_single_record(h: &mut impl Host, this: u32, chain: u32) -> u32 {
    let (cap, pool, used, list, count) = match chain {
        0 => (0x15e0, 0x15e4, 0x2bf8, 0, 0x2c50),
        1 => (0x4238, 0x423c, 0x5838, 0x2c58, 0x5840),
        _ => return chain,
    };
    let taken: u32 = h.read(this + used);
    if taken >= h.read::<u32>(this + cap) & 0x7fff_ffff {
        return chain;
    }
    let records: u32 = h.read(this + pool);
    let record: u32 = h.read(records.wrapping_add(taken.wrapping_mul(4)));
    let k: u32 = h.read(this + count);
    h.write(
        this.wrapping_add(list).wrapping_add(k.wrapping_mul(4)),
        record,
    );
    let next = h.read::<u32>(this + used).wrapping_add(1);
    let records: u32 = h.read(this + pool);
    let record: u32 = h.read(records.wrapping_add(next.wrapping_mul(4)).wrapping_sub(4));
    let k: u32 = h.read(this + count);
    h.write(this + count, k.wrapping_add(1));
    h.write(this + used, next);
    record
}

/// The menu name table `MenuManager_SeveralStoresCopyIntoTable_0067e4a0`
/// fills (`0x00ccc7d4`, string pointers).
pub const MENU_NAMES: u32 = 0x00cc_c7d4;

fn strlen(h: &impl Host, s: u32) -> u32 {
    let mut n = 0u32;
    while h.read::<u8>(s.wrapping_add(n)) != 0 {
        n = n.wrapping_add(1);
    }
    n
}

/// `MenuManager_findNameIndex_0067e310(this; name)` (thiscall, ret 4):
/// the first of the `+0x5844` (unsigned) names of [`MENU_NAMES`] equal to
/// `name` (same length, then the bytes compared from the end); its index,
/// or -1.
pub fn find_name_index(h: &mut impl Host, this: u32, name: u32) -> u32 {
    let n: u32 = h.read(this + 0x5844);
    let table = h.rebase(MENU_NAMES);
    for i in 0..n {
        let s: u32 = h.read(table.wrapping_add(4 * i));
        let len = strlen(h, s);
        if len != strlen(h, name) {
            continue;
        }
        let same =
            (0..len).all(|k| h.read::<u8>(s.wrapping_add(k)) == h.read::<u8>(name.wrapping_add(k)));
        if same {
            return i;
        }
    }
    u32::MAX
}

// ---- arrays

/// `PArray_reset16_0068bc60(this)` (thiscall): an array (`+0` count, top
/// bit not owned; `+4` storage) not already at 16 elements gets a zeroed
/// block of 16 words (`Phyre_PhyreMalloc_0042fb80(0x40, 4)`); the old
/// storage is freed unless it is the new block or not owned; then 16. On a
/// failed allocation nothing changes. Then every element (the count read
/// again each step) is cleared. Callers read no result.
pub fn parray_reset16(h: &mut impl Host, this: u32) {
    if h.read::<u32>(this) != 0x10 {
        let p = h.call_sym(ALIGNED_ALLOC, Conv::Cdecl, &[0x40, 4]);
        if p != 0 {
            for k in 0..16 {
                h.write(p + 4 * k, 0u32);
            }
            let old: u32 = h.read(this + 4);
            if old != p && h.read::<u32>(this) >> 31 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
            }
            h.write(this + 4, p);
            h.write(this, 0x10u32);
        }
    }
    let mut i = 0u32;
    while i < h.read::<u32>(this) & 0x7fff_ffff {
        let data: u32 = h.read(this + 4);
        h.write(data.wrapping_add(4 * i), 0u32);
        i += 1;
    }
}

/// `PArray8_appendInto_006a2790(this; dst, &value)` (thiscall, ret 8):
/// `PArray_appendInto_006a25e0` for 8-byte elements: `dst` 0 gives 0xd;
/// else the count's (`+0`, top bit not owned) elements of `+4` copied
/// into `dst` (a null destination element skipped, the source then not
/// advancing), `*value` appended, the old storage freed unless it is
/// `dst`, not owned or null; `+4 = dst`, `+0` = count + 1 (top bit
/// dropped). Returns 0.
pub fn parray8_append_into(h: &mut impl Host, this: u32, dst: u32, value: u32) -> u32 {
    if dst == 0 {
        return 0xd;
    }
    let n = h.read::<u32>(this) & 0x7fff_ffff;
    let mut src: u32 = h.read(this + 4);
    let mut at = dst;
    for _ in 0..n {
        let d = at;
        at = at.wrapping_add(8);
        if d != 0 {
            let v: u64 = h.read(src);
            h.write(d, v);
            src = src.wrapping_add(8);
        }
    }
    let end = dst.wrapping_add(n.wrapping_mul(8));
    if end != 0 {
        let lo: u32 = h.read(value);
        h.write(end, lo);
        let hi: u32 = h.read(value.wrapping_add(4));
        h.write(end.wrapping_add(4), hi);
    }
    let old: u32 = h.read(this + 4);
    if old != dst && h.read::<u32>(this) >> 31 == 0 && old != 0 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
    }
    h.write(this + 4, dst);
    h.write(this, n + 1);
    0
}

// ---- mip levels

/// `op_texels_halve_00a385f0(dst, src, n, size)` (cdecl).
pub const TEXELS_HALVE: u32 = 0x00a3_85f0;
/// `op_texels_halve_v_00a38560(dst, dst_step, src, src_step, n, size)`.
pub const TEXELS_HALVE_V: u32 = 0x00a3_8560;
/// `op_texels_box_00a38670(dst, row_a, row_b, n, size)`.
pub const TEXELS_BOX: u32 = 0x00a3_8670;

/// `op_texels_mip_00a384d0(dst, dst_step, w, h, src, src_step, size)`
/// (cdecl): one mip level of a `2w` by `2h` image: no rows halves one
/// row (`op_texels_halve(dst, src, w, size)`), no columns one column
/// (`op_texels_halve_v(dst, dst_step, src, src_step, h, size)`), else
/// each output row is the box average of two input rows. The original
/// keeps `2 src_step` in its `h` slot, which its one caller (`0x00a3a200`)
/// does not read; the result is not read either.
pub fn texels_mip(h: &mut impl Host, args: [u32; 7]) {
    let [dst, dst_step, w, rows, src, src_step, size] = args;
    if rows == 0 {
        h.call_sym(TEXELS_HALVE, Conv::Cdecl, &[dst, src, w, size]);
        return;
    }
    if w == 0 {
        h.call_sym(
            TEXELS_HALVE_V,
            Conv::Cdecl,
            &[dst, dst_step, src, src_step, rows, size],
        );
        return;
    }
    let step2 = src_step.wrapping_mul(2);
    let (mut d, mut s) = (dst, src);
    for _ in 0..rows {
        h.call_sym(
            TEXELS_BOX,
            Conv::Cdecl,
            &[d, s, s.wrapping_add(src_step), w, size],
        );
        d = d.wrapping_add(dst_step);
        s = s.wrapping_add(step2);
    }
}

// ---- FMOD events

/// `SfxEventTree_Find_007104c0(tree; &key)` (thiscall, ret 4).
pub const SFX_EVENT_TREE_FIND: u32 = 0x0071_04c0;
/// `WordArray_reserve_0070fbf0(this; n)` (thiscall, ret 4).
pub const WORD_ARRAY_RESERVE: u32 = 0x0070_fbf0;
/// `FmodEventTree_getEvent_00710290(this; key)` (thiscall, ret 4).
pub const EVENT_TREE_GET_EVENT: u32 = 0x0071_0290;
/// `SfxEventGroup_appendEvents_00710ae0` (thiscall, ret 8).
pub const APPEND_GROUP_EVENTS: u32 = 0x0071_0ae0;

/// Appends `v` to the word array `out` (`+0` count, `+4` capacity, `+8`
/// storage), growing it by `n / 8 + 2` past a full one (signed), as the
/// FMOD collectors do inline; a null slot is skipped. Returns the slot.
fn push_word(h: &mut impl Host, out: u32, v: u32) -> u32 {
    let n = h.read::<u32>(out).wrapping_add(1);
    h.write(out, n);
    if n.cast_signed() > h.read::<u32>(out + 4).cast_signed() {
        let grow = (n.cast_signed() / 8)
            .wrapping_add(n.cast_signed())
            .wrapping_add(2)
            .cast_unsigned();
        h.call_sym(WORD_ARRAY_RESERVE, Conv::Thiscall(out), &[grow]);
    }
    let n: u32 = h.read(out);
    let data: u32 = h.read(out + 8);
    let slot = data.wrapping_add(n.wrapping_mul(4)).wrapping_sub(4);
    if slot != 0 {
        h.write(slot, v);
    }
    slot
}

/// `SfxEventGroup_appendEvents_00710ae0(this; key, out)` (thiscall, ret
/// 8; `args` the address of its `key` slot): with a project (`*this`),
/// the node of `key` in the tree at `+4` (`SfxEventTree_Find_007104c0`,
/// handed the `key` slot, the node then kept there); for each of its
/// count (`node+0x10`) ids (`node+0x18`), `out` grown to the count first
/// (signed), the project's virtual `+0x24(project, id, 0, &slot)` writes
/// the event into the `out` slot, which is appended to `out`. Returns 1
/// in `al`, 0 without a project or node (callers read only `al`).
pub fn append_group_events(h: &mut impl Host, this: u32, args: u32) -> u8 {
    if h.read::<u32>(this) == 0 {
        return 0;
    }
    let node = h.call_sym(SFX_EVENT_TREE_FIND, Conv::Thiscall(this + 4), &[args]);
    h.write(args, node);
    if node == 0 || node.wrapping_add(0x10) == 0 {
        return 0;
    }
    let count: u32 = h.read(node.wrapping_add(0x10));
    let out: u32 = h.read(args + 4);
    if count.cast_signed() > h.read::<u32>(out + 4).cast_signed() {
        h.call_sym(WORD_ARRAY_RESERVE, Conv::Thiscall(out), &[count]);
    }
    for i in 0..count {
        let project: u32 = h.read(this);
        let vtable: u32 = h.read(project);
        let node: u32 = h.read(args);
        let ids: u32 = h.read(node.wrapping_add(0x18));
        let id: u32 = h.read(ids.wrapping_add(4 * i));
        let f: u32 = h.read(vtable.wrapping_add(0x24));
        h.call(f, Conv::Stdcall, &[project, id, 0, args + 4]);
        let v: u32 = h.read(args + 4);
        push_word(h, out, v);
    }
    1
}

/// `SfxBank_appendEventById_0070fa80(this; id, out)` (thiscall, ret 8):
/// `out` (a word array) reserved to 0 when its capacity is negative; with
/// a tree at `+4` and its event for `id` (`FmodEventTree_getEvent`), the
/// event appended and 1; else with a group at `+8`
/// `SfxEventGroup_appendEvents_00710ae0(group; id, out)`, else 0. The
/// result is `al` (callers read only that).
pub fn append_event_by_id(h: &mut impl Host, this: u32, id: u32, out: u32) -> u8 {
    if h.read::<u32>(out + 4).cast_signed() < 0 {
        h.call_sym(WORD_ARRAY_RESERVE, Conv::Thiscall(out), &[0]);
    }
    let tree: u32 = h.read(this + 4);
    if tree != 0 {
        let event = h.call_sym(EVENT_TREE_GET_EVENT, Conv::Thiscall(tree), &[id]);
        if event != 0 {
            push_word(h, out, event);
            return 1;
        }
    }
    let group: u32 = h.read(this + 8);
    if group != 0 {
        return h.call_sym(APPEND_GROUP_EVENTS, Conv::Thiscall(group), &[id, out]) as u8;
    }
    0
}

// ---- the character cache

/// `g_ClusterManager` (`0x00ccca4c`).
pub const CLUSTER_MANAGER: Global<u32> = Global::new(0x00cc_ca4c);

fn nil(h: &impl Host, node: u32) -> bool {
    h.read::<u8>(node.wrapping_add(0xd)) != 0
}

/// The in-order successor of `node` in an MSVC `std::map` (`+0` left,
/// `+4` parent, `+8` right, `+0xd` nil); a nil node stays.
fn successor(h: &impl Host, node: u32) -> u32 {
    if nil(h, node) {
        return node;
    }
    let right: u32 = h.read(node.wrapping_add(8));
    if nil(h, right) {
        let mut child = node;
        let mut parent: u32 = h.read(node.wrapping_add(4));
        while !nil(h, parent) && child == h.read::<u32>(parent.wrapping_add(8)) {
            child = parent;
            parent = h.read(parent.wrapping_add(4));
        }
        parent
    } else {
        let mut node = right;
        loop {
            let left: u32 = h.read(node);
            if nil(h, left) {
                return node;
            }
            node = left;
        }
    }
}

/// `ChrResourceCacheManager_enableChrDelayRelease(this)` (`0x0069ae80`,
/// thiscall): in state 2 (`+0`), under the cluster manager's mutex
/// (`**(mgr+0x18)`), every cluster entry of its map (`mgr+0x10`, value
/// `+0x14`) gets its delayed-release flag `+0xc` = (`+8` set and `+0x10`
/// clear); then the state 0. Returns `ReleaseMutex`'s `eax`, or `this` in
/// another state (its one caller, `graphicEnableChrClusterDelayRelease`,
/// jumps here with `eax` = `ecx`).
pub fn enable_chr_delay_release(h: &mut impl Host, this: u32) -> u32 {
    if h.read::<u32>(this) != 2 {
        return this;
    }
    let mgr = CLUSTER_MANAGER.get(h);
    let holder: u32 = h.read(mgr.wrapping_add(0x18));
    let mutex: u32 = h.read(holder);
    let wait: u32 = h.read(h.rebase(WAIT_FOR_SINGLE_OBJECT));
    h.call(wait, Conv::Stdcall, &[mutex, u32::MAX]);
    let head: u32 = h.read(mgr.wrapping_add(0x10));
    let mut node: u32 = h.read(head);
    if node != head {
        loop {
            let entry: u32 = h.read(node.wrapping_add(0x14));
            let flag = h.read::<u32>(entry.wrapping_add(8)) != 0
                && h.read::<u32>(entry.wrapping_add(0x10)) == 0;
            h.write(entry.wrapping_add(0xc), u32::from(flag));
            node = successor(h, node);
            if node == h.read::<u32>(mgr.wrapping_add(0x10)) {
                break;
            }
        }
    }
    let holder: u32 = h.read(mgr.wrapping_add(0x18));
    let mutex: u32 = h.read(holder);
    let release: u32 = h.read(h.rebase(RELEASE_MUTEX));
    let r = h.call(release, Conv::Stdcall, &[mutex]);
    h.write(this, 0u32);
    r
}

// ---- the escape menu

/// `g_MovieFading` (`0x01340824`).
pub const MOVIE_FADING: Global<u32> = Global::new(0x0134_0824);
/// `g_CurrentMapNo` (`0x012fbc40`).
pub const CURRENT_MAP_NO: Global<u32> = Global::new(0x012f_bc40);
/// `getScenerioFlag()` (cdecl, `ax`).
pub const GET_SCENARIO_FLAG: u32 = 0x0078_7360;
/// `isEternalCalmScene(map)` (cdecl).
pub const IS_ETERNAL_CALM_SCENE: u32 = 0x0067_ab50;
/// `FFEscMenu_setEnableParaAndSpecial_006fca50(this; on)` (thiscall).
pub const SET_ENABLE_PARA_AND_SPECIAL: u32 = 0x006f_ca50;
/// `"setEnableKeyBindings"` (`0x00b4c4c0`), the Flash function the
/// refresh calls after it.
pub const REFRESH_FUNCTION: u32 = 0x00b4_c4c0;
/// The bytes of [`esc_refresh_page_state`]'s frame (the Iggy value).
pub const REFRESH_FRAME: u32 = 0x10;

/// `FFEscMenu_refreshParameterAndSpecialPageEnableState(this)`
/// (`0x006fc9b0`, thiscall; `frame` the 16-byte Iggy value at `ebp -
/// 0x10`): the parameter and special pages enabled when no movie fades,
/// the scenario flag (`getScenerioFlag`, `ax`) is set and the map is not
/// the Eternal Calm (`FFEscMenu_setEnableParaAndSpecial_006fca50`); then
/// the boolean "not the Eternal Calm" (map read again) to the Flash
/// function `setEnableKeyBindings`. Returns that call's `eax`.
pub fn esc_refresh_page_state(h: &mut impl Host, frame: u32, this: u32) -> u32 {
    let no_fade = MOVIE_FADING.get(h) == 0;
    let flag = h.call_sym(GET_SCENARIO_FLAG, Conv::Cdecl, &[]) & 0xffff != 0;
    let map = CURRENT_MAP_NO.get(h);
    let calm = h.call_sym(IS_ETERNAL_CALM_SCENE, Conv::Cdecl, &[map]) != 0;
    let on = u32::from(!calm && flag && no_fade);
    h.call_sym(SET_ENABLE_PARA_AND_SPECIAL, Conv::Thiscall(this), &[on]);
    let map = CURRENT_MAP_NO.get(h);
    let calm = h.call_sym(IS_ETERNAL_CALM_SCENE, Conv::Cdecl, &[map]) != 0;
    h.call_sym(DATA_VALUE_SET_BOOL, Conv::Cdecl, &[frame, u32::from(!calm)]);
    let kind: u32 = h.read(frame);
    let value: u32 = h.read(frame + 8);
    let name = h.rebase(REFRESH_FUNCTION);
    // junk read: [ebp-0xc] and [ebp-0x4] never written; 0 (brief 2026-10-24)
    h.call_sym(CALL_FLASH_1, Conv::Cdecl, &[this, name, kind, 0, value, 0])
}

/// `_IggyValueRefCreateArray@8` (import slot).
pub const IGGY_REF_CREATE_ARRAY: u32 = 0x00b0_c824;
/// `_IggyValuePathFromRef@12` (import slot).
pub const IGGY_PATH_FROM_REF: u32 = 0x00b0_c82c;
/// `_IggyValuePathMakeArrayRef@12` (import slot).
pub const IGGY_PATH_MAKE_ARRAY_REF: u32 = 0x00b0_c830;
/// `_IggyValuePathSetArrayIndex@8` (import slot).
pub const IGGY_PATH_SET_ARRAY_INDEX: u32 = 0x00b0_c834;
/// `_IggyValueSetStringUTF8RS@20` (import slot).
pub const IGGY_SET_STRING_UTF8: u32 = 0x00b0_c81c;
/// The bytes of [`iggy_string_array_ctor`]'s frame: the two Iggy paths
/// (`ebp - 0x34`, `ebp - 0x1c`) and the saved `this` (`ebp - 4`).
pub const STRING_ARRAY_FRAME: u32 = 0x34;

fn import(h: &impl Host, slot: u32) -> u32 {
    h.read(h.rebase(slot))
}

/// `IggyStringArray_ctor_006fcbc0(this; root, names, n)` (thiscall, ret
/// 0xc; `frame` as [`STRING_ARRAY_FRAME`]): `+0 = root`, `+4` a new Iggy
/// array of `n` under it (`IggyValueRefCreateArray(root, n)`), a path to
/// it (`IggyValuePathFromRef(&path, +0, +4)`) and to its elements
/// (`IggyValuePathMakeArrayRef(&element, &path, 0)`), then for each of
/// the `n` (signed) strings `names[i]` the index set and the string
/// stored (`IggyValueSetStringUTF8RS(&element, 0, 0, s, strlen(s))`).
/// Returns `this`.
pub fn iggy_string_array_ctor(
    h: &mut impl Host,
    frame: u32,
    this: u32,
    [root, names, n]: [u32; 3],
) -> u32 {
    let path = frame;
    let element = frame + 0x18;
    h.write(frame + 0x30, this);
    h.write(this, root);
    let f = import(h, IGGY_REF_CREATE_ARRAY);
    let array = h.call(f, Conv::Stdcall, &[root, n]);
    let root2: u32 = h.read(this);
    h.write(this + 4, array);
    let f = import(h, IGGY_PATH_FROM_REF);
    h.call(f, Conv::Stdcall, &[path, root2, array]);
    let f = import(h, IGGY_PATH_MAKE_ARRAY_REF);
    h.call(f, Conv::Stdcall, &[element, path, 0]);
    let mut i = 0u32;
    while i.cast_signed() < n.cast_signed() {
        let f = import(h, IGGY_PATH_SET_ARRAY_INDEX);
        h.call(f, Conv::Stdcall, &[element, i]);
        let s: u32 = h.read(names.wrapping_add(4 * i));
        let len = strlen(h, s);
        let f = import(h, IGGY_SET_STRING_UTF8);
        h.call(f, Conv::Stdcall, &[element, 0, 0, s, len]);
        i += 1;
    }
    this
}

// ---- texture-animation records

/// `TextureAnimationInfo::vftable` (`0x00b48420`).
pub const TEXTURE_ANIMATION_INFO_VTABLE: u32 = 0x00b4_8420;
/// `TextureAnimationInfoChar::vftable` (`0x00b4842c`).
pub const TEXTURE_ANIMATION_INFO_CHAR_VTABLE: u32 = 0x00b4_842c;
/// `TextureAnimationManager::vftable` (`0x00b48438`).
pub const TEXTURE_ANIMATION_MANAGER_VTABLE: u32 = 0x00b4_8438;
/// `PSamplerStateD3D11_ctor(this)` (thiscall).
pub const SAMPLER_STATE_CTOR: u32 = 0x0058_3b20;
/// `PArray_destroyObjects_006af630(this)` (thiscall).
pub const DESTROY_OBJECTS: u32 = 0x006a_f630;
/// `PArray_copyNew_006affc0(n, src)` (cdecl).
pub const COPY_NEW_A: u32 = 0x006a_ffc0;
/// `PArray_copyNew_006b0010(n, src)` (cdecl).
pub const COPY_NEW_B: u32 = 0x006b_0010;
/// `PArray40_assign_006afa70(this; src)` (thiscall).
pub const PARRAY40_ASSIGN: u32 = 0x006a_fa70;
/// `Phyre_PString_Assign(this; text)` (thiscall).
pub const PSTRING_ASSIGN: u32 = 0x0044_5cf0;

/// A `PString` at `at` (its word; bit 0 set for a static string) freed
/// unless static.
fn free_string(h: &mut impl Host, at: u32) {
    let s: u32 = h.read(at);
    if s & 1 == 0 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[s]);
    }
}

/// `TextureAnimationInfo`'s body: the word array at `+8`/`+0xc` freed
/// when owned (top bit of `+8` clear) and not null, both cleared; the
/// name `+4` freed unless static.
fn info_body(h: &mut impl Host, this: u32) {
    if h.read::<u32>(this + 8) >> 31 == 0 {
        let data: u32 = h.read(this + 0xc);
        if data != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
        }
    }
    h.write(this + 0xc, 0u32);
    h.write(this + 8, 0u32);
    free_string(h, this + 4);
}

/// `TextureAnimationInfo_dtor_006afd60(this; flags)` (thiscall, ret 4;
/// its vtable's slot 0): the vtable set back to `TextureAnimationInfo`'s,
/// the body freed, `operator_delete(this)` for bit 0 of `flags`. Returns
/// `this`. The SEH frame is not kept.
pub fn texture_animation_info_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
    let vtable = h.rebase(TEXTURE_ANIMATION_INFO_VTABLE);
    h.write(this, vtable);
    info_body(h, this);
    if flags & 1 != 0 {
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
    }
    this
}

/// `TextureAnimationInfoChar_dtor_006af7f0(this)` (thiscall): its vtable,
/// its name `+0x20` freed unless static, then `TextureAnimationInfo`'s
/// vtable and body. The SEH frame is not kept; callers (its deleting
/// destructor `0x006afe00`, an unwind funclet) read no result.
pub fn texture_animation_info_char_dtor(h: &mut impl Host, this: u32) {
    let vtable = h.rebase(TEXTURE_ANIMATION_INFO_CHAR_VTABLE);
    h.write(this, vtable);
    free_string(h, this + 0x20);
    let vtable = h.rebase(TEXTURE_ANIMATION_INFO_VTABLE);
    h.write(this, vtable);
    info_body(h, this);
}

/// `TextureAnimationManager_ctor_006af370(this)` (thiscall): its vtable,
/// two sampler states (`+0x14`, `+0x38`), ten words from `+0x5c`
/// cleared. Returns `this`. The SEH frame is not kept.
pub fn texture_animation_manager_ctor(h: &mut impl Host, this: u32) -> u32 {
    let vtable = h.rebase(TEXTURE_ANIMATION_MANAGER_VTABLE);
    h.write(this, vtable);
    h.call_sym(SAMPLER_STATE_CTOR, Conv::Thiscall(this + 0x14), &[]);
    h.call_sym(SAMPLER_STATE_CTOR, Conv::Thiscall(this + 0x38), &[]);
    for k in 0..10 {
        h.write(this + 0x5c + 4 * k, 0u32);
    }
    this
}

/// `TextureAnimationMaterial_dtor_006af690(this)` (thiscall): the names
/// `+0x1c` and `+0x18` freed unless static, the word array `+0x10`/`+0x14`
/// freed when owned (null too) and cleared, the object array at `+8`
/// destroyed (`PArray_destroyObjects_006af630`), the array `+0`/`+4`
/// freed when owned and cleared. The SEH frame is not kept.
pub fn texture_animation_material_dtor(h: &mut impl Host, this: u32) {
    free_string(h, this + 0x1c);
    free_string(h, this + 0x18);
    if h.read::<u32>(this + 0x10) >> 31 == 0 {
        let data: u32 = h.read(this + 0x14);
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
    }
    h.write(this + 0x14, 0u32);
    h.write(this + 0x10, 0u32);
    h.call_sym(DESTROY_OBJECTS, Conv::Thiscall(this + 8), &[]);
    if h.read::<u32>(this) >> 31 == 0 {
        let data: u32 = h.read(this + 4);
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
    }
    h.write(this, 0u32);
    h.write(this + 4, 0u32);
}

/// One word array of [`texture_animation_material_assign`]: `src`'s count
/// (top bit masked) copied into a new block (`copy(n, data)`), the old
/// storage freed unless it is the new one or not owned, then the block
/// and the count.
fn assign_words(h: &mut impl Host, this: u32, src: u32, copy: u32) {
    let n = h.read::<u32>(src) & 0x7fff_ffff;
    let data: u32 = h.read(src + 4);
    let p = h.call_sym(copy, Conv::Cdecl, &[n, data]);
    let old: u32 = h.read(this + 4);
    if old != p && h.read::<u32>(this) >> 31 == 0 {
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
    }
    h.write(this + 4, p);
    h.write(this, n);
}

/// One `PString` of [`texture_animation_material_assign`]: assigned from
/// `src`'s text (its word with bit 0 cleared) unless it is the same
/// string.
fn assign_string(h: &mut impl Host, this: u32, src: u32) {
    if this == src {
        return;
    }
    let word: u32 = h.read(src);
    let text = word.wrapping_sub(word & 1);
    h.call_sym(PSTRING_ASSIGN, Conv::Thiscall(this), &[text]);
}

/// `TextureAnimationMaterial_assign_006afaf0(this; src)` (thiscall, ret
/// 4): the word arrays `+0` (`PArray_copyNew_006affc0`) and `+0x10`
/// (`PArray_copyNew_006b0010`), the object array `+8`
/// (`PArray40_assign_006afa70`), the names `+0x18` and `+0x1c`. Returns
/// `this`.
pub fn texture_animation_material_assign(h: &mut impl Host, this: u32, src: u32) -> u32 {
    assign_words(h, this, src, COPY_NEW_A);
    h.call_sym(PARRAY40_ASSIGN, Conv::Thiscall(this + 8), &[src + 8]);
    assign_words(h, this + 0x10, src + 0x10, COPY_NEW_B);
    assign_string(h, this + 0x18, src + 0x18);
    assign_string(h, this + 0x1c, src + 0x1c);
    this
}

// ---- menu icons

/// `MsGetRomItem(id, &strings)` (cdecl).
pub const MS_GET_ROM_ITEM: u32 = 0x0079_09f0;
/// `TOAddPktMenuIcon(pkt, x, y, icon)` (cdecl).
pub const ADD_PKT_MENU_ICON: u32 = 0x008e_b240;
/// `TOAddPktBaseMesFontLAbmap(...)` (cdecl, twelve words: wider than
/// [`Host::call`]).
pub const ADD_PKT_BASE_MES_FONT: u32 = 0x008e_8fb0;
/// The bytes of [`adp_menu_icon_item`]'s frame (the string table word).
pub const ICON_ITEM_FRAME: u32 = 4;

/// `TOAdpMenuIconItemAbmap(pkt, item, x, y, colour)` (`0x008f0700`,
/// cdecl; `frame` the word at `ebp - 4` `MsGetRomItem` fills):
/// `TOAddPktCommandIconName_008f0590` for an item: the record's icon
/// (`+0x14`) at `(x + 0xc, y + 1)`, then its name (`+0`, an offset into
/// the strings) at `(x + 0x1a, y + 1)` in `colour`, after the icon.
/// Returns the text call's `eax`; `wide(h, f, words)` calls the cdecl
/// function at `f` (rebased) with its twelve words.
pub fn adp_menu_icon_item<H: Host>(
    h: &mut H,
    frame: u32,
    [pkt, item, x, y, colour]: [u32; 5],
    wide: &mut impl FnMut(&mut H, u32, &[u32]) -> u32,
) -> u32 {
    let record = h.call_sym(MS_GET_ROM_ITEM, Conv::Cdecl, &[item, frame]);
    let strings: u32 = h.read(frame);
    let name = u32::from(h.read::<u16>(record)).wrapping_add(strings);
    let icon = u32::from(h.read::<u8>(record.wrapping_add(0x14)));
    let y = y.wrapping_add(1);
    let at = h.call_sym(
        ADD_PKT_MENU_ICON,
        Conv::Cdecl,
        &[pkt, x.wrapping_add(0xc), y, icon],
    );
    let f = h.rebase(ADD_PKT_BASE_MES_FONT);
    wide(
        h,
        f,
        &[
            at,
            u32::MAX,
            name,
            x.wrapping_add(0x1a),
            y,
            colour,
            0,
            0x80,
            0x80,
            0x80,
            0x80,
            0,
        ],
    )
}
