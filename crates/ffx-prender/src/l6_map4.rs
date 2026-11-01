//! M5 stage 1, round 6 (list `left6`, from `map2.left`), batch 3: the
//! scroll model's VFX render data (the magic overlays' import slot
//! `g_MagicImports + 0xd8c`, `code_00a5bf50`): its creation from the
//! model's sub-meshes and its per-frame update.
//!
//! Callees stay calls to the originals (cdecl). The update keeps four
//! values in its own argument slots (`slots`); the creation's frame (its
//! three count arrays and the texture-load buffers) comes in as the
//! original's `ebp` (the shim passes a `StackFrame`).

/// `g_GraphicSkipFrame`.
pub const SKIP_FRAME: u32 = 0x012f_b7d0;
/// The Vu0 cells the update sets (`0x00c8f8e0`, `0x00c8f920`,
/// `0x00c8f850`, `0x00c8f8c0`: four words each) and the Q word
/// `0x00c8f78c`.
pub const CELL_ZERO: u32 = 0x00c8_f8e0;
pub const CELL_ONE: u32 = 0x00c8_f920;
pub const CELL_HALF: u32 = 0x00c8_f850;
pub const CELL_UV: u32 = 0x00c8_f8c0;
pub const VU0_Q: u32 = 0x00c8_f78c;
/// f32 0.5, `g_ScissorOutside` (-100000.0) and 100000.0.
pub const HALF_F32: u32 = 0x00b9_2194;
pub const SCISSOR_OUT: u32 = 0x00b3_68b8;
pub const SCISSOR_IN: u32 = 0x00b1_0f50;
/// `g_pppFillMatrix`.
pub const FILL_MATRIX: u32 = 0x0230_fca0;
/// The texture failure's format and the three mode names.
pub const LOAD_FAIL_FORMAT: u32 = 0x00b7_b798;
pub const MODE_NAMES: [u32; 3] = [0x00b7_b778, 0x00b7_b780, 0x00b7_b78c];

// ---- callees ----

pub const PACKET_GET_PTR: u32 = 0x0083_b650;
pub const SET_PTR32: u32 = 0x00a4_4700;
pub const GET_RENDER_DATA: u32 = 0x0064_53d0;
pub const VFX_UPDATE: u32 = 0x0064_58a0;
pub const VFX_SETUP: u32 = 0x0064_5780;
pub const VFX_DESTROY: u32 = 0x0064_5280;
pub const VFX_CREATE: u32 = 0x0064_4de0;
pub const VFX_CREATE_DATA_BUFFER: u32 = 0x0064_5040;
pub const VFX_IS_TEXTURE_LOADED: u32 = 0x0064_5450;
pub const VFX_SET_VISIBLE: u32 = 0x0064_5710;
pub const DBG_PRINTF: u32 = 0x0062_f500;
pub const MEMSET: u32 = 0x0094_964c;
pub const LOAD_MATRIX2: u32 = 0x00a5_bec0;
pub const LOAD_MATRIX1: u32 = 0x00a5_be30;
pub const FILL_REQUEST: u32 = 0x0072_8660;
pub const COPY_MATRIX: u32 = 0x006e_d360;
pub const MUL_MATRIX: u32 = 0x006e_d5c0;
pub const BUFFER_FOR_SEGMENT: u32 = 0x0063_fdb0;
/// `createVFXRenderDataBuffer_DrawScrollModel` and
/// `updateVFXRenderData_DrawModelScroll` (this batch).
pub const CREATE_BUFFER: u32 = 0x00a5_c130;
pub const UPDATE: u32 = 0x00a5_c370;

/// The creation's frame.
pub const CREATE_FRAME: u32 = 0x1f78;

pub mod bridge {
    use super::{
        BUFFER_FOR_SEGMENT, CELL_HALF, CELL_ONE, CELL_UV, CELL_ZERO, COPY_MATRIX, CREATE_BUFFER,
        DBG_PRINTF, FILL_MATRIX, FILL_REQUEST, GET_RENDER_DATA, HALF_F32, LOAD_FAIL_FORMAT,
        LOAD_MATRIX1, LOAD_MATRIX2, MEMSET, MODE_NAMES, MUL_MATRIX, PACKET_GET_PTR, SCISSOR_IN,
        SCISSOR_OUT, SET_PTR32, SKIP_FRAME, UPDATE, VFX_CREATE, VFX_CREATE_DATA_BUFFER,
        VFX_DESTROY, VFX_IS_TEXTURE_LOADED, VFX_SET_VISIBLE, VFX_SETUP, VFX_UPDATE, VU0_Q,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn at(h: &impl Host, a: u32, off: u32) -> u32 {
        h.read(a.wrapping_add(off))
    }

    /// The `i16` at `a + off`, sign-extended.
    fn i16_at(h: &impl Host, a: u32, off: u32) -> u32 {
        i32::from(h.read::<i16>(a.wrapping_add(off))).cast_unsigned()
    }

    /// An `f32` copied through the x87 (`fld dword`, `fstp dword`).
    fn copy_f32(bits: u32) -> u32 {
        x87::store(x87::load(bits))
    }

    /// `code_00a5bf50(instance lo, hi, model, a3..a16; buf)` -> the last
    /// callee's `eax`: the scroll model's per-frame entry (the magic
    /// overlays call it through `g_MagicImports + 0xd8c`). With render
    /// data for the instance it is updated (unless the frame is skipped)
    /// and `graphicVFXUpdate`d; without, it is created
    /// (`createVFXRenderDataBuffer_DrawScrollModel(lo, hi, model, buf, a6,
    /// a7, a8, a9, a16)`), set up, updated and `graphicVFXUpdate`d, or on
    /// a failure destroyed. `buf` is the address of a frame buffer the
    /// original hands the creation (it never reads it: the shim's frame).
    pub fn scroll_entry(h: &mut impl Host, args: [u32; 17], buf: u32) -> u32 {
        let (lo, hi, model) = (args[0], args[1], args[2]);
        cdecl(h, PACKET_GET_PTR, &[]);
        cdecl(h, SET_PTR32, &[model]);
        let mut update = args;
        update[10] = copy_f32(args[10]);
        update[11] = copy_f32(args[11]);
        if cdecl(h, GET_RENDER_DATA, &[lo, hi]) != 0 {
            if h.read::<u32>(h.rebase(SKIP_FRAME)) == 0 {
                h.call_sym_wide(UPDATE, &update);
            }
            return cdecl(h, VFX_UPDATE, &[lo, hi]);
        }
        let create = [
            lo, hi, model, buf, args[6], args[7], args[8], args[9], args[16],
        ];
        if cdecl(h, CREATE_BUFFER, &create) != 0 && cdecl(h, VFX_SETUP, &[lo, hi]) != 0 {
            h.call_sym_wide(UPDATE, &update);
            return cdecl(h, VFX_UPDATE, &[lo, hi]);
        }
        cdecl(h, VFX_DESTROY, &[lo, hi])
    }

    /// One sub-mesh's texture (`(lo, hi)`, a 64-bit descriptor: the id in
    /// bits 0..13, a palette in 20..25, the widths `1 << 26..29` and
    /// `1 << 30..33`) loaded into `out` (`graphicVFXIsTextureWithBpLoaded`).
    /// Returns the id and the result.
    fn load_texture(h: &mut impl Host, lo: u32, hi: u32, out: u32) -> (u32, u32) {
        let v = u64::from(hi) << 32 | u64::from(lo);
        let id = lo & 0x3fff;
        #[allow(clippy::cast_possible_truncation)] // the fields' bits
        let field = |s: u32, m: u64| -> u32 { ((v >> s) & m) as u32 };
        let w2 = u32::from(1_u16.wrapping_shl(field(30, 0xf)));
        let w1 = u32::from(1_u16.wrapping_shl(field(26, 0xf)));
        let pal = field(20, 0x3f);
        let r = cdecl(h, VFX_IS_TEXTURE_LOADED, &[id, pal, 0, 0, w1, w2, out]);
        (id, r)
    }

    /// `createVFXRenderDataBuffer_DrawScrollModel(lo, hi, model, _, d1 lo,
    /// d1 hi, d2 lo, d2 hi, mode)` -> 1, or 0 when a texture fails to load
    /// (after its `dbgPrintf`): the model's sub-meshes (`[model + 4]`
    /// from `model`, `i16 [model + 0x10]` index counts in all; each 5n + 2
    /// quadwords: `+1` its kind, `i16 +2` its count n) of kind 3 get their
    /// vertex and index counts (3n, in `ebp - 0x1ef4` and `- 0x1e7c`) and
    /// a texture (mode 1 and 2 the descriptors `d1`, `d2`, else the
    /// sub-mesh's `+8`) loaded into its 0x100-byte slot from `ebp -
    /// 0x1e04`; then `graphicVFXCreate(lo, hi, mode ? 10 : 8)` and the data
    /// buffer (`graphicVFXCreateDataBuffer`) over the counts, the slots
    /// and the flags (`ebp - 0x1f6c`, its first word 1). A sub-mesh of
    /// another kind steps to what the register held (the last texture's
    /// next sub-mesh): before any of kind 3 that is a never-written frame
    /// word (`ebp - 0x1f78`; junk read, 0 here: brief 2026-10-24).
    pub fn create_buffer(h: &mut impl Host, ebp: u32, args: [u32; 9]) -> u32 {
        let [lo, hi, model, _, d1lo, d1hi, d2lo, d2hi, mode] = args;
        let fr = |off: u32| ebp.wrapping_sub(off);
        h.write(fr(0x1ef4), 0_u32);
        cdecl(h, MEMSET, &[fr(0x1ef0), 0, 0x74]);
        h.write(fr(0x1e7c), 0_u32);
        cdecl(h, MEMSET, &[fr(0x1e78), 0, 0x74]);
        h.write(fr(0x1f6c), 1_u32);
        cdecl(h, MEMSET, &[fr(0x1f68), 0, 0x74]);
        let mut left = i16_at(h, model, 0x10).cast_signed();
        let mut sub = at(h, model, 4).wrapping_add(model);
        let mut k = 0_u32;
        h.write(fr(0x1f74), k);
        // junk read: [ebp-0x1f78] never written before the first kind-3
        // sub-mesh; 0 (brief 2026-10-24)
        let mut next = 0_u32;
        if left > 0 {
            let mut slot = fr(0x1e04);
            h.write(fr(0x1f70), slot);
            loop {
                if h.read::<u8>(sub.wrapping_add(1)) == 3 {
                    let n = i16_at(h, sub, 2).wrapping_mul(3);
                    h.write(fr(0x1ef4).wrapping_add(4 * k), n);
                    h.write(fr(0x1e7c).wrapping_add(4 * k), n);
                    let (dlo, dhi) = match mode {
                        1 => (d1lo, d1hi),
                        2 => (d2lo, d2hi),
                        _ => (at(h, sub, 8), at(h, sub, 0xc)),
                    };
                    let (id, ok) = load_texture(h, dlo, dhi, slot);
                    h.write(fr(0x1f78), id);
                    if ok == 0 {
                        let name = MODE_NAMES[match mode {
                            1 => 0,
                            2 => 1,
                            _ => 2,
                        }];
                        let name = h.rebase(name);
                        let fmt = h.rebase(LOAD_FAIL_FORMAT);
                        cdecl(h, DBG_PRINTF, &[fmt, name, id]);
                        return 0;
                    }
                    let n = i16_at(h, sub, 2);
                    next = sub.wrapping_add(n.wrapping_mul(5).wrapping_add(2).wrapping_mul(8));
                }
                let n = i16_at(h, sub, 2).cast_signed();
                slot = slot.wrapping_add(0x100);
                h.write(fr(0x1f70), slot);
                left = left.wrapping_sub(n);
                k = k.wrapping_add(1);
                sub = next;
                h.write(fr(0x1f74), k);
                if left <= 0 {
                    break;
                }
            }
        }
        cdecl(h, VFX_CREATE, &[lo, hi, if mode != 0 { 10 } else { 8 }]);
        cdecl(
            h,
            VFX_CREATE_DATA_BUFFER,
            &[lo, hi, k, fr(0x1ef4), fr(0x1e7c), fr(0x1e04), fr(0x1f6c)],
        );
        1
    }

    /// `(v << 7) / 128` toward zero: the low 25 bits, sign-extended.
    fn low25(v: u32) -> u32 {
        (v.wrapping_shl(7).cast_signed() / 128).cast_unsigned()
    }

    /// `updateVFXRenderData_DrawModelScroll(lo, hi, model, a3, m1, m2, a6,
    /// a7, a8, a9, u, v, frame, w0, w1, base, a16; slots)` -> 1, or 0
    /// without render data or a segment buffer: the Vu0 cells (zero, one,
    /// half, the scroll `(u, v)`), the two matrices loaded
    /// (`AbmapVu0LoadMatrix2(m2)`, `...1(m1)`), the fill request (kind 2:
    /// the model, `base + 0x40`, the widths' low 25 bits, both matrices,
    /// `f32(frame)`, `u`, `v`, `a16`), each sub-mesh's segment buffer (only
    /// the first sub-mesh's kind is read where it should be: the walk goes
    /// on from `model` itself, as the original's) with its blend flag (2,
    /// 1 when `[model] & 4`), the render data's matrix (`m1` x the fill
    /// matrix, its w column cleared, the scissor box wide open) and
    /// `graphicVFXSetVisible(lo, hi, 1)`. The original keeps the widths,
    /// the segment index and offset in its argument slots (words 13, 14,
    /// 15, 10), which the bridge leaves as it does.
    #[allow(clippy::too_many_lines)]
    pub fn update(h: &mut impl Host, slots: u32, a: [u32; 17]) -> u32 {
        let slot = |i: u32| slots.wrapping_add(4 * i);
        let base = a[15].wrapping_add(0x40);
        let (w0, w1) = (low25(a[13]), low25(a[14]));
        h.write(slot(13), w0);
        h.write(slot(14), w1);
        let (one, half) = (1.0_f32.to_bits(), copy_f32(h.read(h.rebase(HALF_F32))));
        let cells = [
            (CELL_ZERO, [0, 0, 0, 0]),
            (CELL_ONE, [one, one, one, 0]),
            (CELL_HALF, [half, half, 0, 0]),
            (CELL_UV, [copy_f32(a[10]), copy_f32(a[11]), 0, 0]),
        ];
        for (cell, words) in cells {
            let c = h.rebase(cell);
            for (k, w) in (0_u32..).zip(words) {
                h.write(c.wrapping_add(4 * k), w);
            }
        }
        let q = h.rebase(VU0_Q);
        h.write(q, 0x437f_0000_u32);
        cdecl(h, LOAD_MATRIX2, &[a[5]]);
        let model = a[2];
        let mut sub = at(h, model, 4).wrapping_add(model);
        cdecl(h, LOAD_MATRIX1, &[a[4]]);
        cdecl(h, PACKET_GET_PTR, &[]);
        cdecl(h, PACKET_GET_PTR, &[]);
        let rd = cdecl(h, GET_RENDER_DATA, &[a[0], a[1]]);
        if rd == 0 {
            return 0;
        }
        let n = i16_at(h, model, 0x10);
        h.write(slot(15), 0_u32);
        let req = cdecl(h, FILL_REQUEST, &[n]);
        for (off, v) in [
            (0x14_u32, model),
            (0x20, base),
            (0x24, 0x40),
            (0x30, 0x80),
            (0x28, 0x80),
            (0x2c, w0),
            (0x34, 0x80),
            (0x38, 0x80),
            (0x3c, w1),
        ] {
            h.write(req.wrapping_add(off), v);
        }
        h.write(req, 2_u8);
        h.write(req.wrapping_add(0x10), rd);
        h.write(req.wrapping_add(0x18), a[3]);
        cdecl(h, COPY_MATRIX, &[req.wrapping_add(0x40), a[4]]);
        cdecl(h, COPY_MATRIX, &[req.wrapping_add(0x80), a[5]]);
        for k in 0..4_u32 {
            h.write(req.wrapping_add(0xc0 + 4 * k), 0_u32);
        }
        h.write(
            req.wrapping_add(0xd0),
            x87::store(f64::from(a[12].cast_signed())),
        );
        h.write(req.wrapping_add(0xd4), copy_f32(a[10]));
        h.write(req.wrapping_add(0xd8), copy_f32(a[11]));
        h.write(req.wrapping_add(0xdc), a[16]);
        let mut left = n.cast_signed();
        if left > 0 {
            h.write(slot(10), 0_u32);
            loop {
                if h.read::<u8>(sub.wrapping_add(1)) == 3 {
                    let m = i16_at(h, sub, 2).wrapping_mul(3);
                    let seg: u32 = h.read(slot(15));
                    if cdecl(h, BUFFER_FOR_SEGMENT, &[rd, seg, m, m]) == 0 {
                        return 0;
                    }
                    let flag = if h.read::<u8>(model) & 4 != 0 { 1 } else { 2 };
                    let off: u32 = h.read(slot(10));
                    let parts = at(h, rd, 0x94);
                    h.write(parts.wrapping_add(off).wrapping_add(0x24), flag);
                }
                let k = i16_at(h, sub, 2).cast_signed();
                let seg: u32 = h.read(slot(15));
                h.write(slot(15), seg.wrapping_add(1));
                let off: u32 = h.read(slot(10));
                h.write(slot(10), off.wrapping_add(0x6c));
                sub = model;
                left = left.wrapping_sub(k);
                if left <= 0 {
                    break;
                }
            }
        }
        h.write(rd.wrapping_add(0xb0), 0_u32);
        for k in 0..16_u32 {
            let w = at(h, a[4], 4 * k);
            h.write(rd.wrapping_add(4 * k), w);
        }
        let fill = h.rebase(FILL_MATRIX);
        cdecl(h, MUL_MATRIX, &[rd, fill, rd]);
        for off in [0xc_u32, 0x1c, 0x2c] {
            h.write(rd.wrapping_add(off), 0_u32);
        }
        h.write(rd.wrapping_add(0x3c), one);
        h.write(rd.wrapping_add(0x80), 0_u32);
        let out = copy_f32(h.read(h.rebase(SCISSOR_OUT)));
        let inn = copy_f32(h.read(h.rebase(SCISSOR_IN)));
        for (k, off) in (0_u32..).zip([0x98_u32, 0x9c, 0xa0, 0xa4, 0xa8, 0xac]) {
            h.write(rd.wrapping_add(off), if k < 3 { out } else { inn });
        }
        cdecl(h, VFX_SET_VISIBLE, &[a[0], a[1], 1]);
        1
    }
}
