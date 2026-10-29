//! List `phyre-anim-left`, batch 11 (M5 stage 1): `PAnimatableComponent`'s
//! network set-up (`0x0053a2e0`): the network's nodes counted and
//! collected by two visitors (in the caller's frame, `frame`), the
//! records ordered and laid out, the targets bound, and the batch block
//! the animation jobs read built.

#![allow(clippy::many_single_char_names)]

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

use crate::m5p_anim_small::{NOT_OWNED, PHYRE_FREE};

/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// The CRT `memset` (cdecl `(dst, c, n)`).
pub const MEMSET: u32 = 0x0094_964c;
/// The node-count and node-collect visitors' vtables, and their
/// destructor (thiscall).
pub const COUNT_VISITOR: u32 = 0x00b2_a2e8;
/// See [`COUNT_VISITOR`].
pub const COLLECT_VISITOR: u32 = 0x00b2_a304;
/// See [`COUNT_VISITOR`].
pub const VISITOR_DTOR: u32 = 0x0052_1230;
/// The records' resize `0x0053bfd0`, the ordering `0x0053cb90`, the
/// batch-data resize `0x0053bf50`, the target resize `0x0053bde0` (in
/// this list).
pub const RECORDS_RESIZE: u32 = 0x0053_bfd0;
/// See [`RECORDS_RESIZE`].
pub const ORDER_RECORDS: u32 = 0x0053_cb90;
/// See [`RECORDS_RESIZE`].
pub const DATA_RESIZE: u32 = 0x0053_bf50;
/// See [`RECORDS_RESIZE`].
pub const TARGETS_RESIZE: u32 = 0x0053_bde0;
/// The records' count fix-up after collecting (thiscall on `+4`, `(n)`).
pub const RECORDS_SET_COUNT: u32 = 0x0053_cf20;
/// The target set's descriptor array (thiscall on the set, `(0)`).
pub const TARGET_ARRAY: u32 = 0x0053_afd0;
/// A node's batch preparation (thiscall).
pub const NODE_PREPARE: u32 = 0x0052_41a0;
/// How many words of the caller's frame the two visitors take (`ebp -
/// 0x28` and `ebp - 0x1c`).
pub const FRAME_WORDS: u32 = 6;

pub mod bridge {
    use super::{
        COLLECT_VISITOR, COUNT_VISITOR, Conv, DATA_RESIZE, Host, MEMSET, Mem, NODE_PREPARE,
        NOT_OWNED, ORDER_RECORDS, PHYRE_FREE, PHYRE_MALLOC, RECORDS_RESIZE, RECORDS_SET_COUNT,
        TARGET_ARRAY, TARGETS_RESIZE, VISITOR_DTOR,
    };

    fn count(m: &impl Mem, at: u32) -> u32 {
        m.read::<u32>(at) & !NOT_OWNED
    }

    fn record(m: &impl Mem, this: u32, k: u32) -> u32 {
        let base: u32 = m.read(this.wrapping_add(8));
        base.wrapping_add(k.wrapping_mul(0x94))
    }

    /// The node visitors and the record layout; the first error.
    fn visit_nodes(h: &mut impl Host, this: u32, root: u32, frame: u32) -> u32 {
        let (v1, v2) = (frame, frame.wrapping_add(0xc));
        let vt1 = h.rebase(COUNT_VISITOR);
        h.write(v1, vt1);
        h.write(v1.wrapping_add(4), 0u32);
        h.write(v1.wrapping_add(8), 0u32);
        h.call_virtual(root, 3, &[v1, 0]);
        let n: u32 = h.read(v1.wrapping_add(8));
        let r = h.call_sym(RECORDS_RESIZE, Conv::Thiscall(this.wrapping_add(4)), &[n]);
        if r != 0 {
            h.call_sym(VISITOR_DTOR, Conv::Thiscall(v1), &[]);
            return r;
        }
        let raw: u32 = h.read(this.wrapping_add(0x20));
        if raw != 0 {
            let data: u32 = h.read(this.wrapping_add(0x24));
            if data != 0 && raw & NOT_OWNED == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
            h.write(this.wrapping_add(0x24), 0u32);
            h.write(this.wrapping_add(0x20), 0u32);
        }
        let vt2 = h.rebase(COLLECT_VISITOR);
        let recs: u32 = h.read(this.wrapping_add(8));
        h.write(v2, vt2);
        h.write(v2.wrapping_add(4), recs);
        h.write(v2.wrapping_add(8), 0u32);
        h.call_virtual(root, 3, &[v2, 0]);
        let c: u32 = h.read(v2.wrapping_add(8));
        h.call_sym(
            RECORDS_SET_COUNT,
            Conv::Thiscall(this.wrapping_add(4)),
            &[c],
        );
        let mut r = h.call_sym(ORDER_RECORDS, Conv::Thiscall(this), &[]);
        if r == 0 {
            let mut total = 0u32;
            h.write(this.wrapping_add(0x28), 0u32);
            for k in (0..c).rev() {
                let rec = record(h, this, k);
                h.write(rec.wrapping_add(0x8c), total);
                let size: u32 = h.read(this.wrapping_add(0x28));
                h.write(rec.wrapping_add(0x90), size);
                let node: u32 = h.read(record(h, this, k));
                let v = h.call_virtual(node, 5, &[]);
                total = total.wrapping_add(v).wrapping_add(0xf) & !0xf;
                let node: u32 = h.read(record(h, this, k));
                let w = h.call_virtual(node, 6, &[]);
                let size: u32 = h.read(this.wrapping_add(0x28));
                h.write(
                    this.wrapping_add(0x28),
                    size.wrapping_add(0xf).wrapping_add(w) & !0xf,
                );
            }
            r = h.call_sym(
                DATA_RESIZE,
                Conv::Thiscall(this.wrapping_add(0x20)),
                &[total >> 4],
            );
            if r == 0 {
                let data: u32 = h.read(this.wrapping_add(0x24));
                if data != 0 {
                    h.call_sym(MEMSET, Conv::Cdecl, &[data, 0, total]);
                }
            }
        }
        h.call_sym(VISITOR_DTOR, Conv::Thiscall(v2), &[]);
        h.call_sym(VISITOR_DTOR, Conv::Thiscall(v1), &[]);
        r
    }

    /// `0x0053a2e0` (`thiscall(network, binders, n)`, `ret 0xc`): see the
    /// module. 0 without a network or targets; else the first error, 0xd
    /// when the batch block cannot be allocated, or 0.
    pub fn set_network(
        h: &mut impl Host,
        this: u32,
        network: u32,
        binders: u32,
        n: u32,
        frame: u32,
    ) -> u32 {
        if network == 0 {
            return 0;
        }
        h.write(this.wrapping_add(0x18), network);
        let set: u32 = h.read(network.wrapping_add(8));
        if set == 0 {
            return 0;
        }
        let root: u32 = h.read(network.wrapping_add(0xc));
        if root != 0 {
            let r = visit_nodes(h, this, root, frame);
            if r != 0 {
                return r;
            }
        }
        if set != h.read::<u32>(this.wrapping_add(0x14)) {
            h.write(this.wrapping_add(0x14), set);
            let nt = count(h, set.wrapping_add(8));
            let r = h.call_sym(
                TARGETS_RESIZE,
                Conv::Thiscall(this.wrapping_add(0xc)),
                &[nt],
            );
            if r != 0 {
                return r;
            }
            let d = h.call_sym(TARGET_ARRAY, Conv::Thiscall(set), &[0]);
            for i in 0..n {
                let b: u32 = h.read(binders.wrapping_add(4 * i));
                let targets: u32 = h.read(this.wrapping_add(0x10));
                let r = h.call_virtual(b, 1, &[targets, d, nt]);
                if r != 0 {
                    return r;
                }
            }
        }
        let n2 = count(h, set.wrapping_add(0x10));
        let cw = count(h, this.wrapping_add(0x2c));
        let nrec = count(h, this.wrapping_add(4));
        h.write(this.wrapping_add(0x1c), n2.wrapping_mul(cw));
        let old: u32 = h.read(this.wrapping_add(0x34));
        h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        let size = nrec.wrapping_sub(1).wrapping_mul(0xa0).wrapping_add(0xc8);
        let blk = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[size, 0x10]);
        if blk == 0 {
            return 0xd;
        }
        h.write(this.wrapping_add(0x34), blk);
        h.write(blk.wrapping_add(0x24), nrec);
        h.write(blk, u32::MAX);
        let data: u32 = h.read(this.wrapping_add(0x24));
        h.write(blk.wrapping_add(8), data);
        let words = count(h, this.wrapping_add(0x20)) << 4;
        h.write(blk.wrapping_add(0xc), words);
        h.write(blk.wrapping_add(0x14), n2 << 5);
        h.write(blk.wrapping_add(4), 0u32);
        h.write(blk.wrapping_add(0x10), 0u32);
        h.write(blk.wrapping_add(0x18), n2);
        h.write(blk.wrapping_add(0x1c), cw);
        h.write(blk.wrapping_add(0x20), 0u32);
        let mut e = blk.wrapping_add(0x28);
        for k in (0..nrec).rev() {
            let rec = record(h, this, k);
            let node: u32 = h.read(rec);
            h.call_sym(NODE_PREPARE, Conv::Thiscall(node), &[]);
            for off in [0u32, 4, 8] {
                h.write(e.wrapping_add(off), 0u32);
            }
            let inputs: u32 = h.read(rec.wrapping_add(4));
            h.write(e.wrapping_add(0xc), inputs);
            let mut q = 0u32;
            while q < h.read::<u32>(rec.wrapping_add(4)) {
                let w: u32 = h.read(rec.wrapping_add(8 + 4 * q));
                h.write(e.wrapping_add(0x10 + 4 * q), w);
                q += 1;
            }
            let w: u32 = h.read(rec.wrapping_add(0x88));
            h.write(e.wrapping_add(0x90), w);
            h.write(e.wrapping_add(0x94), 0u32);
            h.write(e.wrapping_add(0x98), 0u32);
            let w: u32 = h.read(rec.wrapping_add(0x8c));
            h.write(e.wrapping_add(0x9c), w);
            e = e.wrapping_add(0xa0);
        }
        0
    }
}
