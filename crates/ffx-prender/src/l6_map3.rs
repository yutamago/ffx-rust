//! M5 stage 1, round 6 (list `left6`, from `map2.left`), batch 2: the
//! guide map's skeleton of the selected character (its bones projected,
//! linked to their parents, and marked).
//!
//! As [`crate::l6_map`]: callees stay calls to the originals (cdecl), the
//! stack cookie is left out, the frame comes in as the original's `ebp`.

pub use crate::l6_map::{APPLY_MATRIX, CALC_PERS_MATRIX, COPY_C0A004, DIV_VECTOR_XYZ, RCUI_WORK};

/// `get_ptr32_func(p)`, `Ch_ElemGetMatrix(chr, e, out)`,
/// `rcbgScaleMatrix100(m)`, `Ch_ElemGetParent(chr, e)`,
/// `rcPacketLine_00927810(pkt, x0, y0, x1, y1, rgba)`,
/// `rcPacketRect_00927630(pkt, x, y, w, h, rgba)`,
/// `Wrap_0092c600_0092c5e0(pkt, m)` (cdecl).
pub const GET_PTR32: u32 = 0x00a4_4650;
pub const ELEM_GET_MATRIX: u32 = 0x0083_3c30;
pub const SCALE_MATRIX100: u32 = 0x0091_ffd0;
pub const ELEM_GET_PARENT: u32 = 0x0083_3cf0;
pub const PACKET_LINE: u32 = 0x0092_7810;
pub const PACKET_RECT: u32 = 0x0092_7630;
pub const AXIS_MARKER_WRAP: u32 = 0x0092_c5e0;
/// The guide colours (16 bytes each; the skeleton uses 1, 5 and 6).
pub const GUIDE_COLOURS: u32 = 0x00c5_e798;
/// The bone markers' size: 4.0 and 4.666 (f32 constants).
pub const MARK_W: u32 = 0x00b9_22f8;
pub const MARK_H: u32 = 0x00b6_7df4;

/// `0x0092c250`'s frame.
pub const SKELETON_FRAME: u32 = 0x94;

pub mod bridge {
    use super::{
        APPLY_MATRIX, AXIS_MARKER_WRAP, CALC_PERS_MATRIX, COPY_C0A004, DIV_VECTOR_XYZ,
        ELEM_GET_MATRIX, ELEM_GET_PARENT, GET_PTR32, GUIDE_COLOURS, MARK_H, MARK_W, PACKET_LINE,
        PACKET_RECT, RCUI_WORK, SCALE_MATRIX100,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// The word at `a + off`.
    fn at(h: &impl Host, a: u32, off: u32) -> u32 {
        h.read(a.wrapping_add(off))
    }

    /// An `f32` copied through the x87 (`fld dword`, `fstp dword`).
    fn copy_f32(bits: u32) -> u32 {
        x87::store(x87::load(bits))
    }

    /// Whether bone `k` is in the shown range `lo..=hi` (signed).
    fn shown(k: u32, lo: u32, hi: u32) -> bool {
        k.cast_signed() >= lo.cast_signed() && k.cast_signed() <= hi.cast_signed()
    }

    /// `FUN_0092c250(pkt, chr)` -> `eax` as the original: the selected
    /// character's skeleton on the guide map. `g_RcuiWork`: `+0x8b8` the
    /// bone count (read again at each step), `+0x8bc` one bone picked
    /// (`+0x8c4`) or else the ten from `+0x8c0` shown (to the last bone),
    /// `+0x8c8` the 16-byte point buffer (`get_ptr32_func`). Each bone's
    /// matrix (`Ch_ElemGetMatrix`, `rcbgScaleMatrix100`, then
    /// `rcCalcPersMatrix` into `ebp - 0x84`) projects the origin
    /// (`VFX_CopyC0a004_0092cc20`, divided by w) into the buffer; each bone
    /// with a parent gets a line to it (colour 5 when either is shown, else
    /// 6), each a 4 x 4.666 marker (colour 1 shown, else 6); a picked bone
    /// gets the axis marker (`Wrap_0092c600_0092c5e0` on its matrix). The
    /// frame (`ebp - 0x94..`) holds `pkt`, the range, the cursors and the
    /// two matrices, as the original leaves them. `eax` is the last
    /// callee's (the axis marker's, a marker's, a line's or -1 for a bone
    /// without a parent) or the buffer when nothing ran.
    pub fn skeleton(h: &mut impl Host, ebp: u32, pkt: u32, chr: u32) -> u32 {
        let w = RCUI_WORK.get(h);
        let m = ebp.wrapping_sub(0x44);
        let p = ebp.wrapping_sub(0x84);
        h.write(ebp.wrapping_sub(0x94), pkt);
        let picked = at(h, w, 0x8bc) != 0;
        let (lo, hi) = if picked {
            let b = at(h, w, 0x8c4);
            (b, b)
        } else {
            let first = at(h, w, 0x8c0);
            let last = first.wrapping_add(9);
            h.write(ebp.wrapping_sub(0x88), last);
            let n = at(h, w, 0x8b8);
            let last = if last.cast_signed() < n.cast_signed() {
                last
            } else {
                n.wrapping_sub(1)
            };
            (first, last)
        };
        h.write(ebp.wrapping_sub(0x90), lo);
        h.write(ebp.wrapping_sub(0x88), hi);
        let buf = cdecl(h, GET_PTR32, &[at(h, w, 0x8c8)]);
        h.write(ebp.wrapping_sub(0x8c), buf);
        let count = |h: &mut _| -> i32 { at(h, w, 0x8b8).cast_signed() };
        let mut v = buf;
        let mut k = 0_u32;
        while k.cast_signed() < count(h) {
            cdecl(h, ELEM_GET_MATRIX, &[chr, k, m]);
            cdecl(h, SCALE_MATRIX100, &[m]);
            cdecl(h, CALC_PERS_MATRIX, &[p, m]);
            cdecl(h, COPY_C0A004, &[v]);
            cdecl(h, APPLY_MATRIX, &[v, p, v]);
            let s = copy_f32(at(h, v, 0xc));
            cdecl(h, DIV_VECTOR_XYZ, &[v, v, s]);
            k = k.wrapping_add(1);
            v = v.wrapping_add(0x10);
        }
        let mut eax = buf;
        let colours = h.rebase(GUIDE_COLOURS);
        let mut q = buf;
        if count(h) > 0 {
            h.write(ebp.wrapping_sub(0x88), q);
        }
        let mut k = 0_u32;
        while k.cast_signed() < count(h) {
            let par = cdecl(h, ELEM_GET_PARENT, &[chr, k]);
            eax = par;
            if par != u32::MAX {
                let c = if shown(k, lo, hi) || shown(par, lo, hi) {
                    5
                } else {
                    6
                };
                let to = buf.wrapping_add(par.wrapping_mul(0x10));
                let args = [
                    pkt,
                    copy_f32(at(h, q, 0)),
                    copy_f32(at(h, q, 4)),
                    copy_f32(at(h, to, 0)),
                    copy_f32(at(h, to, 4)),
                    colours.wrapping_add(c * 0x10),
                ];
                eax = cdecl(h, PACKET_LINE, &args);
            }
            q = q.wrapping_add(0x10);
            h.write(ebp.wrapping_sub(0x88), q);
            k = k.wrapping_add(1);
        }
        let mut r = buf;
        let mut k = 0_u32;
        while k.cast_signed() < count(h) {
            let c = if shown(k, lo, hi) { 1 } else { 6 };
            let mw = copy_f32(h.read(h.rebase(MARK_W)));
            let mh = copy_f32(h.read(h.rebase(MARK_H)));
            let (x, y) = (copy_f32(at(h, r, 0)), copy_f32(at(h, r, 4)));
            let colour = colours.wrapping_add(c * 0x10);
            eax = cdecl(h, PACKET_RECT, &[pkt, x, y, mw, mh, colour]);
            r = r.wrapping_add(0x10);
            h.write(ebp.wrapping_sub(0x8c), r);
            k = k.wrapping_add(1);
        }
        if at(h, w, 0x8bc) != 0 {
            let b = at(h, w, 0x8c4);
            cdecl(h, ELEM_GET_MATRIX, &[chr, b, m]);
            cdecl(h, SCALE_MATRIX100, &[m]);
            eax = cdecl(h, AXIS_MARKER_WRAP, &[pkt, m]);
        }
        eax
    }
}
