//! M5 stage 1, round 6 (list `left6`, from `vfx2.left`): Iggy `GDraw`
//! D3D11 pieces earlier rounds left for time: the projection for the
//! current target and the render-target bind callback.
//!
//! The immediate context's methods are called through its vtable
//! (stdcall, `this` first). Where the original hands a callee a frame
//! buffer, the bridge takes its address (the shim passes a `StackFrame`).

pub use crate::m5v_h::{GDRAW, vt};

// ---- callees ----

/// `gdraw_setProjection_00a3c450(x0, x1, y0, y1)` (cdecl).
pub const SET_PROJECTION: u32 = 0x00a3_c450;
/// `gdraw_getDepthStencilState_00a3d020(depth, write, read_mask,
/// write_mask)`, `gdraw_drawTargetQuad_00a3bed0(target, state, sub)`,
/// `gdraw_bindOutput_00a3c570(frame)`,
/// `gdraw_setViewportForTarget_00a3cd30()` (cdecl).
pub const DEPTH_STENCIL_STATE: u32 = 0x00a3_d020;
pub const DRAW_TARGET_QUAD: u32 = 0x00a3_bed0;
pub const BIND_OUTPUT: u32 = 0x00a3_c570;
pub const SET_VIEWPORT_FOR_TARGET: u32 = 0x00a3_cd30;

/// `pppGetRotMatrixXYZ(out, angles)`, `FFXVu0MulMatrix(dst, a, b)`
/// (cdecl); `g_pppViewMatrixCopy`; the scale cells `0x00c8f820` (the
/// factor) and `0x00c8f830` (the row scaled).
pub const GET_ROT_MATRIX_XYZ: u32 = 0x0072_c800;
pub const MUL_MATRIX: u32 = 0x006e_d5c0;
pub const VIEW_MATRIX_COPY: u32 = 0x0230_fe20;
pub const SCALE_FACTOR: u32 = 0x00c8_f820;
pub const SCALE_ROW: u32 = 0x00c8_f830;

/// `gdraw_rt_bind_00a37410`'s frame (the target description at
/// `ebp - 0x30`).
pub const RT_BIND_FRAME: u32 = 0x30;

pub mod bridge {
    use super::{
        BIND_OUTPUT, DEPTH_STENCIL_STATE, DRAW_TARGET_QUAD, GDRAW, GET_ROT_MATRIX_XYZ, MUL_MATRIX,
        SCALE_FACTOR, SCALE_ROW, SET_PROJECTION, SET_VIEWPORT_FOR_TARGET, VIEW_MATRIX_COPY, vt,
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

    /// A context method at byte offset `off` (stdcall, `ctx` first).
    fn com(h: &mut impl Host, ctx: u32, off: u32, args: &[u32]) -> u32 {
        let vtable: u32 = h.read(ctx);
        let f: u32 = h.read(vtable.wrapping_add(off));
        let mut words = vec![ctx];
        words.extend_from_slice(args);
        h.call(f, Conv::Stdcall, &words)
    }

    /// `FUN_00a3c350()` -> `gdraw_setProjection_00a3c450`'s `eax`: the
    /// projection for the current target: none while drawing whole
    /// (`+0x6f8`; `None`, the caller's `eax`); on the main target (stack
    /// top `+0xaac` at its base `+0x9cc`) the tile (`+0x6e8`, `+0x6ec`
    /// origin, `+0x6d8`, `+0x6dc` size), on a stack entry with a size
    /// (`+0x18`) the entry's (`+4`, `+8` origin, `+0xc`, `+0x10` size),
    /// else the screen (`+0x6f0`, `+0x6f4`, `+0x6e0`, `+0x6e4`). The
    /// first case passes y as (origin + size, origin), the others
    /// (origin, origin + size) for the entry and (origin + size, origin)
    /// for the screen, as the original.
    pub fn target_projection(h: &mut impl Host) -> Option<u32> {
        let s = GDRAW.get(h);
        if h.read::<u32>(s.wrapping_add(0x6f8)) != 0 {
            return None;
        }
        let top: u32 = h.read(s.wrapping_add(0xaac));
        let args = if top == s.wrapping_add(0x9cc) {
            let (x, y) = (at(h, s, 0x6e8), at(h, s, 0x6ec));
            let (w, ht) = (at(h, s, 0x6d8), at(h, s, 0x6dc));
            [x, w.wrapping_add(x), ht.wrapping_add(y), y]
        } else if at(h, top, 0x18) != 0 {
            let (x, y) = (at(h, top, 4), at(h, top, 8));
            let (w, ht) = (at(h, top, 0xc), at(h, top, 0x10));
            [x, w.wrapping_add(x), y, ht.wrapping_add(y)]
        } else {
            let (x, y) = (at(h, s, 0x6f0), at(h, s, 0x6f4));
            let (w, ht) = (at(h, s, 0x6e0), at(h, s, 0x6e4));
            [x, w.wrapping_add(x), ht.wrapping_add(y), y]
        };
        Some(cdecl(h, SET_PROJECTION, &args))
    }

    /// `gdraw_rt_bind_00a37410(write_mask)` (stdcall) -> `eax` as the
    /// original: the depth-stencil state for the mask
    /// (`gdraw_getDepthStencilState_00a3d020(0, 0, 0, mask)`); the main
    /// colour target unbound against the depth view `+0x784`
    /// (`OMSetRenderTargets(0, NULL, view)`), the rasterizer state
    /// `+0x7ac`, a target quad into the frame's target (`ebp - 0x30`,
    /// sub 1); again with the second depth view `+0x788` when there is
    /// one (sub 0); the output bound (`gdraw_bindOutput_00a3c570`), the
    /// viewport and the projection for the target ([`target_projection`],
    /// whose `None` leaves the viewport call's `eax`).
    pub fn rt_bind(h: &mut impl Host, ebp: u32, mask: u32) -> u32 {
        let frame = ebp.wrapping_sub(0x30);
        let state = cdecl(h, DEPTH_STENCIL_STATE, &[0, 0, 0, mask]);
        for (view_off, sub) in [(0x784_u32, 1_u32), (0x788, 0)] {
            let s = GDRAW.get(h);
            let view: u32 = h.read(s.wrapping_add(view_off));
            if sub == 0 && view == 0 {
                break;
            }
            let ctx: u32 = h.read(s.wrapping_add(4));
            com(h, ctx, vt::OM_SET_RENDER_TARGETS, &[0, 0, view]);
            let s = GDRAW.get(h);
            let ctx: u32 = h.read(s.wrapping_add(4));
            let rs: u32 = h.read(s.wrapping_add(0x7ac));
            com(h, ctx, vt::RS_SET_STATE, &[rs]);
            cdecl(h, DRAW_TARGET_QUAD, &[frame, state, sub]);
        }
        cdecl(h, BIND_OUTPUT, &[frame]);
        let v = cdecl(h, SET_VIEWPORT_FOR_TARGET, &[]);
        target_projection(h).unwrap_or(v)
    }

    /// `ppp_mdl_rev_matrix_007401d0(work, _, rec)` -> `FFXVu0MulMatrix`'s
    /// `eax`: a model's matrix from the program's data. `[rec + 0xc]` holds
    /// four offsets into `work + 0xa0` (the position, the angles, the
    /// scale, the three sign flags); the rotation (`pppGetRotMatrixXYZ`)
    /// into `work + 0x10`, its rows 0..2 scaled by the scale's x, y, z (each
    /// negated by its flag) through the cells `0x00c8f820` (the factor) and
    /// `0x00c8f830` (the row: x, y and z scaled, w kept), the position into
    /// row 3, then `work + 0x50` = view copy x it. The second word is not
    /// read.
    pub fn rev_matrix(h: &mut impl Host, work: u32, rec: u32) -> u32 {
        let d: u32 = h.read(rec.wrapping_add(0xc));
        let rd =
            |h: &mut _, off: u32| -> u32 { at(h, d, off).wrapping_add(work).wrapping_add(0xa0) };
        let flips = rd(h, 0x14);
        let scale = rd(h, 8);
        let pos = rd(h, 0);
        let angles = rd(h, 4);
        let rot = work.wrapping_add(0x10);
        cdecl(h, GET_ROT_MATRIX_XYZ, &[rot, angles]);
        let mut s = [0_u32; 3];
        for (k, v) in (0_u32..).zip(&mut s) {
            let x = x87::load(h.read(scale.wrapping_add(4 * k)));
            let neg = h.read::<u8>(flips.wrapping_add(1 + k)) != 0;
            *v = x87::store(if neg { -x } else { x });
        }
        let factor = h.rebase(SCALE_FACTOR);
        let cell = h.rebase(SCALE_ROW);
        for (k, f) in (0_u32..).zip(s) {
            let row = rot.wrapping_add(0x10 * k);
            let f64v = x87::load(f);
            h.write(factor, x87::store(f64v));
            let w: [u32; 4] = std::array::from_fn(|j| h.read(row.wrapping_add(4 * j as u32)));
            for (j, x) in (0_u32..).zip(w) {
                h.write(cell.wrapping_add(4 * j), x);
            }
            for j in 0..3_u32 {
                let x = x87::load(h.read(cell.wrapping_add(4 * j)));
                h.write(cell.wrapping_add(4 * j), x87::store(x87::mul(x, f64v)));
            }
            for j in 0..4_u32 {
                let x: u32 = h.read(cell.wrapping_add(4 * j));
                h.write(row.wrapping_add(4 * j), x);
            }
        }
        for j in 0..3_u32 {
            let x: u32 = h.read(pos.wrapping_add(4 * j));
            h.write(work.wrapping_add(0x40 + 4 * j), x87::store(x87::load(x)));
        }
        let view = h.rebase(VIEW_MATRIX_COPY);
        cdecl(h, MUL_MATRIX, &[work.wrapping_add(0x50), view, rot])
    }
}
