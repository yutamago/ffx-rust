//! M5 stage 1, round 10 (`tail10`, list `pc-ui`): Iggy's `GDraw` D3D11
//! `gdraw_rt_end_00a39200` (stdcall `(r, x0, y0, x1, y1, stats)`, `ret
//! 0x18`, the function table's slot `0x01a85844`): the end of an
//! off-screen render target.
//!
//! The rectangle is clipped to the target's origin (`+0x6f0`, `+0x6f4`)
//! and size (`+0x6e0`, `+0x6e4`) in the `GDraw` state (`[0x01a858b4]`); an
//! empty one ends there. Else its texture coordinates (`(x - origin) /
//! the texture's size +0x6c0 / +0x6c4`), the blend and depth states, the
//! pending scissor rectangle (`+0x9c4`), then by the record's kind (`+0xc`):
//! a filter (4: by its type `+0xe`, the blur passes, one pass or the
//! three-input pass with or without its flag), a resolve or copy into the
//! main target (5: through the multisampled view `+0x78c` when the stack
//! holds only the main target `+0x9cc`, else a fresh target block and
//! `CopySubresourceRegion`), then the draw state and its rectangle for
//! every kind but a filter. The draw count at `+0x77c` is put back at the
//! end. The locals the callees see stay in a frame at the original's
//! `ebp` offsets ([`frame`]).

use ffx_graphic::host::{Conv, Host};

/// The `GDraw` state pointer.
pub const GDRAW: u32 = 0x01a8_58b4;
/// The blur passes' shared block (`0x01a85828`).
pub const FILTER_BLOCK: u32 = 0x01a8_5828;
/// The blend factor `OMSetBlendState` gets (`0x00b7a3f8`).
pub const BLEND_FACTOR: u32 = 0x00b7_a3f8;
/// The pass callback stored in the blur set-up (`0x00a38aa0`).
pub const PASS_CALLBACK: u32 = 0x00a3_8aa0;

/// `op_node_slot_00a3ba10(node)` (cdecl): the filter's four words.
pub const NODE_SLOT: u32 = 0x00a3_ba10;
/// `GDraw_filterPasses_00a36490` (cdecl, six words).
pub const FILTER_PASSES: u32 = 0x00a3_6490;
/// `gdraw_setViewportForTarget_00a3cd30()`.
pub const SET_VIEWPORT: u32 = 0x00a3_cd30;
/// `gdraw_targetProjection_00a3c350()`.
pub const TARGET_PROJECTION: u32 = 0x00a3_c350;
/// `gdraw_filterPass_00a374e0(r, rect, uv, stats)`.
pub const FILTER_PASS: u32 = 0x00a3_74e0;
/// `gdraw_filterPass3_00a38c40(r, rect, uv, kind, stats)`.
pub const FILTER_PASS3: u32 = 0x00a3_8c40;
/// `gdraw_unbindTarget_00a3b590()` -> the view to restore.
pub const UNBIND_TARGET: u32 = 0x00a3_b590;
/// `gdraw_allocRenderTarget_00a3b610(stats)`.
pub const ALLOC_TARGET: u32 = 0x00a3_b610;
/// `gdraw_setTextureUnit_00a3ccb0(unit, texture, wrap, filter)`.
pub const SET_TEXTURE_UNIT: u32 = 0x00a3_ccb0;
/// `gdraw_setRenderState_00a3c6c0(format, r, stats, scale)`.
pub const SET_RENDER_STATE: u32 = 0x00a3_c6c0;
/// `GDraw_rectCall_00a36040(rect, uv, stats)`.
pub const RECT_CALL: u32 = 0x00a3_6040;
/// `op_node_requeue_00a39f00(block)`.
pub const REQUEUE: u32 = 0x00a3_9f00;
/// `gdraw_freeHandle_00a3b480(block, stats)`.
pub const FREE_HANDLE: u32 = 0x00a3_b480;

/// The frame below `ebp` the callees see, from `ebp - 0xe4` (the bridge's
/// `f`), up to the cookie at `ebp - 4`.
pub mod frame {
    /// `ebp - 0xe4`: the multisampled view's description (`GetDesc`).
    pub const VIEW_DESC: u32 = 0;
    /// `ebp - 0xcc`: the blur set-up (target width, height, texture
    /// width, height, the pass callback).
    pub const PASSES: u32 = 0x18;
    /// `ebp - 0xb8`: the filter's four words.
    pub const SLOT: u32 = 0x2c;
    /// `ebp - 0xa8`: the texture coordinates (four floats).
    pub const UV: u32 = 0x3c;
    /// `ebp - 0x94`: the draw state's scale and offset (four floats).
    pub const SCALE: u32 = 0x50;
    /// `ebp - 0x84`: the view `gdraw_unbindTarget` gives its resource.
    pub const SOURCE: u32 = 0x60;
    /// `ebp - 0x80`: the multisampled view's resource.
    pub const TEXTURE: u32 = 0x64;
    /// `ebp - 0x68`: that texture's description (0x2c bytes).
    pub const DESC: u32 = 0x7c;
    /// `ebp - 0x3c`: the scissor rectangle.
    pub const SCISSOR: u32 = 0xa8;
    /// `ebp - 0x2c`: the copy's box.
    pub const BOX: u32 = 0xb8;
    /// `ebp - 0x14`: the rectangle (x0, y0, x1, y1).
    pub const RECT: u32 = 0xd0;
    /// The bytes from `ebp - 0xe4` to `ebp - 4`.
    pub const BYTES: u32 = 0xe0;
}

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the original's registers (r, g, f, ...)
pub mod bridge {
    use super::{
        ALLOC_TARGET, BLEND_FACTOR, Conv, FILTER_BLOCK, FILTER_PASS, FILTER_PASS3, FILTER_PASSES,
        FREE_HANDLE, GDRAW, Host, NODE_SLOT, PASS_CALLBACK, RECT_CALL, REQUEUE, SET_RENDER_STATE,
        SET_TEXTURE_UNIT, SET_VIEWPORT, TARGET_PROJECTION, UNBIND_TARGET, frame,
    };
    use ffx_bridge::x87;

    fn gdraw(h: &impl Host) -> u32 {
        h.read(h.rebase(GDRAW))
    }

    /// A word of the `GDraw` state.
    fn gw(h: &impl Host, off: u32) -> u32 {
        h.read(gdraw(h).wrapping_add(off))
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// COM method `slot` of `obj` (stdcall, `obj` first).
    fn com(h: &mut impl Host, obj: u32, slot: u32, args: &[u32]) -> u32 {
        let vt: u32 = h.read(obj);
        let f: u32 = h.read(vt.wrapping_add(4 * slot));
        let mut w = [0u32; 9];
        w[0] = obj;
        w[1..=args.len()].copy_from_slice(args);
        h.call(f, Conv::Stdcall, &w[..=args.len()])
    }

    /// `(float)i` as `fild` / `fstp m32` give it.
    fn int_f(i: u32) -> f64 {
        f64::from(f32::from_bits(x87::store(f64::from(i.cast_signed()))))
    }

    /// An unsigned word as `fild` plus 2^32 when negative, `fstp m32`.
    fn uint_f(u: u32) -> f64 {
        f64::from(f32::from_bits(x87::store(f64::from(u))))
    }

    /// `a / b` stored as `fstp m32`.
    fn quot(a: f64, b: f64) -> u32 {
        x87::store(x87::div(a, b))
    }

    /// The draw statistics: `or word [stats], 2`, `inc word [stats+4]`,
    /// `[stats+0x18] += pixels`.
    fn count(h: &mut impl Host, stats: u32, pixels: u32) {
        let at = stats.wrapping_add(0x18);
        let v: u32 = h.read(at);
        h.write(at, v.wrapping_add(pixels));
        let flags: u16 = h.read(stats);
        h.write(stats, flags | 2);
        let n: u16 = h.read(stats.wrapping_add(4));
        h.write(stats.wrapping_add(4), n.wrapping_add(1));
    }

    /// `gdraw_rt_end_00a39200(r, x0, y0, x1, y1, stats)` with its frame at
    /// `f` ([`frame`]); returns the `eax` it leaves: the clipped `y1` when
    /// the rectangle is empty, else the draw count it puts back.
    #[allow(clippy::too_many_lines)] // one original
    pub fn rt_end(h: &mut impl Host, f: u32, args: [u32; 6]) -> u32 {
        let [r, ax0, ay0, ax1, ay1, stats] = args;
        let at = |o: u32| f.wrapping_add(o);
        let ctx = gw(h, 4);
        let saved = gw(h, 0x77c);
        let ox = gw(h, 0x6f0);
        let x0 = if ax0.cast_signed() > ox.cast_signed() {
            ax0
        } else {
            ox
        };
        let oy = gw(h, 0x6f4);
        let y0 = if ay0.cast_signed() > oy.cast_signed() {
            ay0
        } else {
            oy
        };
        let xl = ox.wrapping_add(gw(h, 0x6e0));
        let x1 = if ax1.cast_signed() < xl.cast_signed() {
            ax1
        } else {
            xl
        };
        let yl = oy.wrapping_add(gw(h, 0x6e4));
        let y1 = if ay1.cast_signed() < yl.cast_signed() {
            ay1
        } else {
            yl
        };
        let rect = at(frame::RECT);
        for (i, v) in (0u32..).zip([x0, y0, x1, y1]) {
            h.write(rect.wrapping_add(4 * i), v);
        }
        if x1.cast_signed() < x0.cast_signed() || y1.cast_signed() < y0.cast_signed() {
            return y1;
        }
        // the texture coordinates
        let tw = int_f(gw(h, 0x6c0));
        let uv = at(frame::UV);
        h.write(uv, quot(int_f(x0.wrapping_sub(ox)), tw));
        let th = int_f(gw(h, 0x6c4));
        h.write(uv.wrapping_add(4), quot(int_f(y0.wrapping_sub(oy)), th));
        h.write(uv.wrapping_add(8), quot(int_f(x1.wrapping_sub(ox)), tw));
        h.write(uv.wrapping_add(12), quot(int_f(y1.wrapping_sub(oy)), th));

        let blend = gw(h, 0x7d4);
        let factor = h.rebase(BLEND_FACTOR);
        com(h, ctx, 35, &[blend, factor, u32::MAX]); // OMSetBlendState
        let depth = gw(h, 0x7f0);
        com(h, ctx, 36, &[depth, 0]); // OMSetDepthStencilState
        if gw(h, 0x9c4) != 0 {
            let s = at(frame::SCISSOR);
            let (left, top) = (gw(h, 0x6fc), gw(h, 0x700));
            let right = gw(h, 0x704).wrapping_add(left);
            let bottom = gw(h, 0x708).wrapping_add(top);
            for (i, v) in (0u32..).zip([left, top, right, bottom]) {
                h.write(s.wrapping_add(4 * i), v);
            }
            let c = gw(h, 4);
            com(h, c, 45, &[1, s]); // RSSetScissorRects
            let g = gdraw(h);
            h.write(g.wrapping_add(0x9c4), 0_u32);
        }
        let g = gdraw(h);
        h.write(g.wrapping_add(0x9c8), 0_u32);
        h.write(g.wrapping_add(0x77c), 0_u32);

        let kind: u8 = h.read(r.wrapping_add(0xc));
        if kind == 4 {
            let uv = at(frame::UV);
            match h.read::<u8>(r.wrapping_add(0xe)) {
                0 => {
                    let node: u32 = h.read(r.wrapping_add(0x1c));
                    let words = cdecl(h, NODE_SLOT, &[node]);
                    let slot = at(frame::SLOT);
                    for i in 0..4 {
                        let w: u32 = h.read(words.wrapping_add(4 * i));
                        h.write(slot.wrapping_add(4 * i), w);
                    }
                    let (ox, oy) = (gw(h, 0x6f0), gw(h, 0x6f4));
                    for (i, d) in (0u32..).zip([ox, oy, ox, oy]) {
                        let p = rect.wrapping_add(4 * i);
                        let v: u32 = h.read(p);
                        h.write(p, v.wrapping_sub(d));
                    }
                    let passes = at(frame::PASSES);
                    for (i, o) in (0u32..).zip([0x6e0, 0x6e4, 0x6c0, 0x6c4]) {
                        let v = gw(h, o);
                        h.write(passes.wrapping_add(4 * i), v);
                    }
                    let callback = h.rebase(PASS_CALLBACK);
                    h.write(passes.wrapping_add(0x10), callback);
                    let g = gdraw(h);
                    h.write(g.wrapping_add(0x6f8), 1_u32);
                    let block = h.rebase(FILTER_BLOCK);
                    cdecl(h, FILTER_PASSES, &[block, passes, r, rect, slot, stats]);
                    let g = gdraw(h);
                    h.write(g.wrapping_add(0x6f8), 0_u32);
                    cdecl(h, SET_VIEWPORT, &[]);
                    cdecl(h, TARGET_PROJECTION, &[]);
                }
                1 => {
                    cdecl(h, FILTER_PASS, &[r, rect, uv, stats]);
                }
                2 => {
                    cdecl(h, FILTER_PASS3, &[r, rect, uv, 1, stats]);
                }
                3 => {
                    cdecl(h, FILTER_PASS3, &[r, rect, uv, 0, stats]);
                }
                _ => {
                    let g = gdraw(h);
                    h.write(g.wrapping_add(0x77c), saved);
                    return saved;
                }
            }
            let g = gdraw(h);
            h.write(g.wrapping_add(0x77c), saved);
            return saved;
        }

        let scale = at(frame::SCALE);
        let one = 1.0f32.to_bits();
        for (i, v) in (0u32..).zip([one, one, 0, 0]) {
            h.write(scale.wrapping_add(4 * i), v);
        }
        let mut target = 0u32;
        if kind == 5 {
            let ctx = gw(h, 4);
            let source = at(frame::SOURCE);
            let view = cdecl(h, UNBIND_TARGET, &[]);
            com(h, view, 7, &[source]); // GetResource
            let g = gdraw(h);
            let main = g.wrapping_add(0x9cc);
            if h.read::<u32>(g.wrapping_add(0xaac)) == main && gw(h, 0x790) != 0 {
                // resolve the multisampled target into the main one
                let ms = gw(h, 0x78c);
                let view_desc = at(frame::VIEW_DESC);
                com(h, ms, 8, &[view_desc]); // GetDesc
                let ms = gw(h, 0x78c);
                let texture = at(frame::TEXTURE);
                com(h, ms, 7, &[texture]); // GetResource
                let tex: u32 = h.read(texture);
                let desc = at(frame::DESC);
                com(h, tex, 10, &[desc]); // GetDesc
                let src: u32 = h.read(source);
                let format: u32 = h.read(view_desc);
                com(h, ctx, 57, &[tex, 0, src, 0, format]); // ResolveSubresource
                let tex: u32 = h.read(texture);
                com(h, tex, 2, &[]); // Release
                let width: u32 = h.read(desc);
                let height: u32 = h.read(desc.wrapping_add(4));
                count(h, stats, height.wrapping_mul(width));
                let views = gdraw(h).wrapping_add(0x78c);
                com(h, ctx, 8, &[1, 1, views]); // PSSetShaderResources
                let samplers = gdraw(h).wrapping_add(0x7b4);
                com(h, ctx, 10, &[1, 1, samplers]); // PSSetSamplers
                let w = uint_f(width);
                let tw = int_f(gw(h, 0x6c0));
                h.write(scale, quot(tw, w));
                let hgt = uint_f(height);
                let th = int_f(gw(h, 0x6c4));
                h.write(scale.wrapping_add(4), quot(th, hgt));
                let dx = gw(h, 0x6f0)
                    .wrapping_sub(gw(h, 0x6e8))
                    .wrapping_add(gw(h, 0x6c8));
                h.write(scale.wrapping_add(8), quot(int_f(dx), w));
                let dy = gw(h, 0x6f4)
                    .wrapping_sub(gw(h, 0x6ec))
                    .wrapping_add(gw(h, 0x6cc));
                h.write(scale.wrapping_add(12), quot(int_f(dy), hgt));
            } else {
                // copy into a fresh target block
                let bx = at(frame::BOX);
                for (i, v) in (0u32..).zip([0, 0, 0, 0, 0, 1]) {
                    h.write(bx.wrapping_add(4 * i), v);
                }
                let (mut dx, mut dy) = (0u32, 0u32);
                target = cdecl(h, ALLOC_TARGET, &[stats]);
                let g = gdraw(h);
                if h.read::<u32>(g.wrapping_add(0xaac)) == g.wrapping_add(0x9cc) {
                    let (l, t) = (gw(h, 0x6c8), gw(h, 0x6cc));
                    let r = gw(h, 0x6d8).wrapping_add(l);
                    let b = gw(h, 0x6dc).wrapping_add(t);
                    // left, top, (front 0), right, bottom
                    for (i, v) in [(0, l), (1, t), (3, r), (4, b)] {
                        h.write(bx.wrapping_add(4 * i), v);
                    }
                    dx = gw(h, 0x6e8).wrapping_sub(gw(h, 0x6f0));
                    dy = gw(h, 0x6ec).wrapping_sub(gw(h, 0x6f4));
                } else {
                    let (w, hh) = (gw(h, 0x6e0), gw(h, 0x6e4));
                    h.write(bx.wrapping_add(12), w);
                    h.write(bx.wrapping_add(16), hh);
                }
                let dst: u32 = h.read(target);
                let src: u32 = h.read(source);
                // CopySubresourceRegion
                com(h, ctx, 46, &[dst, 0, dx, dy, 0, src, 0, bx]);
                let side = |i: u32| -> u32 { h.read(bx.wrapping_add(4 * i)) };
                let (left, top, right, bottom) = (side(0), side(1), side(3), side(4));
                let pixels = bottom
                    .wrapping_sub(top)
                    .wrapping_mul(right.wrapping_sub(left));
                count(h, stats, pixels);
                cdecl(h, SET_TEXTURE_UNIT, &[1, target, 0, 0]);
            }
            let src: u32 = h.read(source);
            com(h, src, 2, &[]); // Release
        }
        if cdecl(h, SET_RENDER_STATE, &[2, r, stats, scale]) == 0 {
            let g = gdraw(h);
            h.write(g.wrapping_add(0x77c), saved);
            return saved;
        }
        let uv = at(frame::UV);
        cdecl(h, RECT_CALL, &[rect, uv, stats]);
        let (a, b): (u32, u32) = (h.read(r.wrapping_add(0x1c)), h.read(r.wrapping_add(0x20)));
        let (lo, hi) = (gw(h, 0xac0), gw(h, 0xac4));
        for p in [a, b] {
            if p != 0 {
                h.write(p.wrapping_add(0x28), lo);
                h.write(p.wrapping_add(0x2c), hi);
            }
        }
        if target != 0 {
            let g = gdraw(h);
            if h.read::<u32>(target.wrapping_add(0x18)) == g.wrapping_add(0x290) {
                cdecl(h, REQUEUE, &[target]);
            } else {
                cdecl(h, FREE_HANDLE, &[target, stats]);
            }
        }
        let g = gdraw(h);
        h.write(g.wrapping_add(0x77c), saved);
        saved
    }
}
