//! M5 stage 1, list `phyre-registry-load-b`, round big-phyre2
//! (`ffx_pload::m5p_registry_b_bp2`): the visibility check's occluder
//! gathering (`0x00a2f7d0`). Parity tests:
//! `crates/ffx-oracle/tests/m5p_registry_b.rs` (`bp2_*`).

/// `PMatrix4_Copy_004c9430` (thiscall `(dst; src)`).
pub const MATRIX_COPY: u32 = 0x004c_9430;
/// `PMatrix4_FromMatrix3x4_00451080` (thiscall `(out; m)`).
pub const FROM_3X4: u32 = 0x0045_1080;
/// `Phyre_PRendering_PInternal_PCheckVisibleBoundsJob_boxPlanes_009f2a60`
/// (cdecl, six words).
pub const BOX_PLANES: u32 = 0x009f_2a60;
/// `0x00a312c0` (thiscall `(item; a, planes, out)`, ret 0xc): an
/// occluder's planes, the end of what it wrote.
pub const OCCLUDER_PLANES: u32 = 0x00a3_12c0;
/// The words of `0x00a2f7d0`'s frame below `ebp`.
pub const GATHER_FRAME_WORDS: u32 = 78;

pub mod bridge {
    //! The originals over game memory.

    use super::{BOX_PLANES, FROM_3X4, MATRIX_COPY, OCCLUDER_PLANES};
    use ffx_bridge::x87::{self, Reg};
    use ffx_graphic::host::{Conv, Host};

    /// The identity's rows as `fld1`/`fldz` store them.
    const IDENTITY: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    fn copy_words(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        for k in 0..n {
            let w: u32 = h.read(src + 4 * k);
            h.write(dst + 4 * k, w);
        }
    }

    /// `Phyre_PRendering_PVisibilityCheck_gatherOccluders_00a2f7d0(out,
    /// items, count, a)` (cdecl): `out` holds four plane counts, then the
    /// plane sets (from `+0x10`, 0x40 bytes each), then from `+0x310` one
    /// 0x40-byte block per kept occluder. For each item (`{geometry,
    /// 3x4}`, 8 bytes) while counts are free: its matrix (the 3x4 as a
    /// 4x4, or the identity copied through `ebp - 0xf8`) at `ebp - 0x138`;
    /// the geometry's box (origin `+0x10`, extent `+0x20` minus it, each
    /// `f32`, at `ebp - 0x78`) tested by `boxPlanes` against each kept
    /// occluder's plane set with the next block (from `+0x310`) as the
    /// inner set; a result of 0 (hidden) drops the item. A kept item's
    /// planes come from `0x00a312c0(item; a, next plane, next block)`:
    /// when it wrote any, their count (`/ 0x40`) goes into the next count
    /// and both cursors move on. After the last item the free counts are
    /// zeroed; when the counts fill up first, nothing more is written.
    pub fn gather_occluders(h: &mut impl Host, ebp: u32, out: u32, items: u32, count: u32, a: u32) {
        let counts_end = out.wrapping_add(0x10);
        let blocks = out.wrapping_add(0x310);
        let mut counts = out;
        if count != 0 {
            let identity = ebp - 0x68;
            let saved = ebp - 0xf8;
            let m = ebp - 0x138;
            let extent = ebp - 0x78;
            h.write(identity, IDENTITY.map(f32::to_bits));
            h.call_sym(MATRIX_COPY, Conv::Thiscall(saved), &[identity]);
            let end = items.wrapping_add(count.wrapping_mul(8));
            let mut item_at = items;
            let mut planes = counts_end;
            let mut block = blocks;
            loop {
                let item: u32 = h.read(item_at);
                let geometry: u32 = h.read(item);
                let local: u32 = h.read(item + 4);
                let src = if local == 0 {
                    copy_words(h, ebp - 0xb8, saved, 16);
                    ebp - 0xb8
                } else {
                    h.call_sym(FROM_3X4, Conv::Thiscall(identity), &[local]);
                    identity
                };
                copy_words(h, m, src, 16);
                let origin = geometry.wrapping_add(0x10);
                let mut set = counts_end;
                let mut inner = blocks;
                let mut at = out;
                let mut visible = true;
                if at < counts {
                    let d = [0u32, 4, 8].map(|o| {
                        let to: u32 = h.read(geometry.wrapping_add(0x20 + o));
                        let from: u32 = h.read(origin.wrapping_add(o));
                        Reg::constant(x87::load(to))
                            .sub_m32(f32::from_bits(from))
                            .store()
                    });
                    while at < counts {
                        let n: u32 = h.read(at);
                        let set_end = (n << 6).wrapping_add(set);
                        h.write(extent, d.map(f32::to_bits));
                        let r = h.call_sym(
                            BOX_PLANES,
                            Conv::Cdecl,
                            &[set, set_end, inner, origin, extent, m],
                        );
                        if r == 0 {
                            visible = false;
                            break;
                        }
                        inner = inner.wrapping_add(0x40);
                        at = at.wrapping_add(4);
                        set = set_end;
                    }
                }
                if visible {
                    let next =
                        h.call_sym(OCCLUDER_PLANES, Conv::Thiscall(item), &[a, planes, block]);
                    if next != planes {
                        #[allow(clippy::cast_sign_loss, clippy::cast_possible_wrap)] // sar
                        let n = (next.wrapping_sub(planes) as i32 >> 6) as u32;
                        h.write(counts, n);
                        counts = counts.wrapping_add(4);
                        block = block.wrapping_add(0x40);
                        planes = next;
                    }
                }
                item_at = item_at.wrapping_add(8);
                if item_at == end {
                    break;
                }
                if counts >= counts_end {
                    return;
                }
            }
        }
        // the free counts zeroed (`rep stos`)
        if counts < counts_end {
            let words = ((counts_end - counts - 1) >> 2) + 1;
            for k in 0..words {
                h.write(counts + 4 * k, 0u32);
            }
        }
    }
}
