//! M5 stage 1, list `vfx`, round 5 batch 18: Iggy `GDraw` D3D11's indexed
//! triangle draw (`gdraw_DrawIndexedTriangles_00a387e0`, a stdcall
//! callback): a primitive from a vertex buffer handle, from copies of its
//! vertices and indices in the dynamic buffers, or as quads in chunks
//! through the shared quad index buffer.
//!
//! The stride and offsets the original keeps in its frame come in through
//! its `ebp` (the shim passes a `StackFrame`).

use ffx_bridge::Global;

/// The frame: `ebp` minus its lowest local.
pub const FRAME: u32 = 0x10;
/// `GDraw`'s state (`+4` the immediate context, `+0x260` the input
/// layouts, `+0x274` the vertex shaders (8 bytes apart), `+0x7a0` the quad
/// index buffer, `+0x998`/`+0x9a8` the dynamic vertex and index buffers
/// (`+0x9a0` the vertex one's offset), `+0x9c0` the quad chunk,
/// `+0xac0`/`+0xac4` the target size).
pub const GDRAW: Global<u32> = Global::new(0x01a8_58b4);
/// The vertex strides by format, the render state's colour.
pub const STRIDES: u32 = 0x00c6_d50c;
pub const STATE_COLOUR: u32 = 0x00b7_a838;
/// `DXGI_FORMAT_R16_UINT`.
pub const R16_UINT: u32 = 0x39;

/// The device context's methods (`ID3D11DeviceContext`, stdcall with the
/// context first).
pub mod vt {
    pub const VS_SET_SHADER: u32 = 0x2c;
    pub const DRAW_INDEXED: u32 = 0x30;
    pub const UNMAP: u32 = 0x3c;
    pub const IA_SET_INPUT_LAYOUT: u32 = 0x44;
    pub const IA_SET_VERTEX_BUFFERS: u32 = 0x48;
    pub const IA_SET_INDEX_BUFFER: u32 = 0x4c;
}

// ---- callees ----

/// `gdraw_setRenderState_00a3c6c0(format, r, stats, colour)` -> 0 to skip,
/// `gdraw_lockDynamicBuffer_00a3cf40(buffer, n)` -> the bytes or 0,
/// `GDraw_releaseHandle_00a362c0(buffer)` -> its offset (the unmap),
/// `memcpy(dst, src, n)` (cdecl).
pub const SET_RENDER_STATE: u32 = 0x00a3_c6c0;
pub const LOCK_DYNAMIC: u32 = 0x00a3_cf40;
pub const UNLOCK: u32 = 0x00a3_62c0;
pub const MEMCPY: u32 = 0x0094_9256;

pub mod bridge {
    use super::{
        GDRAW, LOCK_DYNAMIC, MEMCPY, R16_UINT, SET_RENDER_STATE, STATE_COLOUR, STRIDES, UNLOCK, vt,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn method(h: &mut impl Host, ctx: u32, off: u32, args: &[u32]) {
        let vtable: u32 = h.read(ctx);
        let f: u32 = h.read(vtable + off);
        let mut all = vec![ctx];
        all.extend_from_slice(args);
        // call width: 6
        h.call(f, Conv::Stdcall, &all);
    }

    fn state(h: &impl Host) -> u32 {
        GDRAW.get(h)
    }

    /// The dynamic vertex buffer unmapped (through the state's context) and
    /// its offset read.
    fn unmap_vertices(h: &mut impl Host) -> u32 {
        let s = state(h);
        let ctx: u32 = h.read(s + 4);
        let buffer: u32 = h.read(s + 0x998);
        method(h, ctx, vt::UNMAP, &[buffer, 0]);
        h.read(s + 0x9a0)
    }

    /// `gdraw_DrawIndexedTriangles_00a387e0(r, p, buffers, stats)` (stdcall)
    /// -> `p.num_vertices`, or 0 when the render state is skipped: the
    /// vertex shader (`r +4` bit 4: [state + 0x274 + format * 8] directly,
    /// else `gdraw_setRenderState`, `p +0x10` the format) and its input
    /// layout; then from `buffers` (`+0` vertices at `p +0`, `+4` indices
    /// at `p +4`), from `p`'s vertices and indices copied to the dynamic
    /// buffers, or, with no indices, quads of up to [state + 0x9c0]
    /// vertices at a time on the quad index buffer (none with that 0). A
    /// failed lock skips the draw. `buffers`, `r +0x1c` and `r +0x20` take
    /// the target size; `stats` gets bit 0, a draw, the indices (`+0x10`)
    /// and vertices (`+0x14`). Writes the `p` slot (the vertex offset, or
    /// the index bytes) and its frame (`ebp - 4` the stride).
    pub fn draw_indexed_triangles(
        h: &mut impl Host,
        ebp: u32,
        slots: u32,
        r: u32,
        p: u32,
        buffers: u32,
        stats: u32,
    ) -> u32 {
        let s = state(h);
        let ctx: u32 = h.read(s + 4);
        h.write(ebp - 0xc, ctx);
        let format: u32 = h.read(p + 0x10);
        if h.read::<u8>(r + 4) & 0x10 == 0 {
            let colour = h.rebase(STATE_COLOUR);
            if cdecl(h, SET_RENDER_STATE, &[format, r, stats, colour]) == 0 {
                return 0;
            }
        } else {
            let shader: u32 = h.read((s + 0x274).wrapping_add(format.wrapping_mul(8)));
            method(h, ctx, vt::VS_SET_SHADER, &[shader, 0, 0]);
        }
        let stride: u32 = h.read(h.rebase(STRIDES).wrapping_add(format.wrapping_mul(4)));
        h.write(ebp - 4, stride);
        let layout: u32 = h.read((state(h) + 0x260).wrapping_add(format.wrapping_mul(4)));
        method(h, ctx, vt::IA_SET_INPUT_LAYOUT, &[layout]);
        let stride_at = ebp - 4;
        let drawn = if buffers != 0 {
            let offset: u32 = h.read(p);
            h.write(slots + 4, offset);
            method(
                h,
                ctx,
                vt::IA_SET_VERTEX_BUFFERS,
                &[0, 1, buffers, stride_at, slots + 4],
            );
            let index: u32 = h.read(buffers + 4);
            let start: u32 = h.read(p + 4);
            method(h, ctx, vt::IA_SET_INDEX_BUFFER, &[index, R16_UINT, start]);
            true
        } else if h.read::<u32>(p + 4) != 0 {
            draw_copies(h, ebp, slots, p, ctx)
        } else {
            draw_quads(h, ebp, p, ctx);
            false
        };
        if drawn {
            let count: u32 = h.read(p + 0xc);
            method(h, ctx, vt::DRAW_INDEXED, &[count, 0, 0]);
        }
        let s = state(h);
        let second: u32 = h.read(r + 0x20);
        let w: u32 = h.read(s + 0xac0);
        let first: u32 = h.read(r + 0x1c);
        let hh: u32 = h.read(s + 0xac4);
        for t in [buffers, first, second] {
            if t != 0 {
                h.write(t + 0x28, w);
                h.write(t + 0x2c, hh);
            }
        }
        let flags: u16 = h.read(stats);
        h.write(stats, flags | 1);
        let draws: u16 = h.read(stats + 2);
        h.write(stats + 2, draws.wrapping_add(1));
        let indices: u32 = h.read(p + 0xc);
        let total: u32 = h.read(stats + 0x10);
        h.write(stats + 0x10, total.wrapping_add(indices));
        let vertices: u32 = h.read(p + 8);
        let total: u32 = h.read(stats + 0x14);
        h.write(stats + 0x14, total.wrapping_add(vertices));
        vertices
    }

    /// The vertices and indices copied into the dynamic buffers and bound
    /// -> whether both locks held.
    fn draw_copies(h: &mut impl Host, ebp: u32, slots: u32, p: u32, ctx: u32) -> bool {
        let stride: u32 = h.read(ebp - 4);
        let bytes = h.read::<u32>(p + 8).wrapping_mul(stride);
        let index_bytes = h.read::<u32>(p + 0xc).wrapping_mul(2);
        h.write(slots + 4, index_bytes);
        let at = state(h) + 0x998;
        let dst = cdecl(h, LOCK_DYNAMIC, &[at, bytes]);
        if dst == 0 {
            return false;
        }
        let src: u32 = h.read(p);
        cdecl(h, MEMCPY, &[dst, src, bytes]);
        let offset = unmap_vertices(h);
        h.write(ebp - 8, offset);
        let at = state(h) + 0x9a8;
        let dst = cdecl(h, LOCK_DYNAMIC, &[at, index_bytes]);
        if dst == 0 {
            return false;
        }
        let src: u32 = h.read(p + 4);
        cdecl(h, MEMCPY, &[dst, src, index_bytes]);
        let at = state(h) + 0x9a8;
        let start = cdecl(h, UNLOCK, &[at]);
        let vertices = state(h) + 0x998;
        method(
            h,
            ctx,
            vt::IA_SET_VERTEX_BUFFERS,
            &[0, 1, vertices, ebp - 4, ebp - 8],
        );
        let index: u32 = h.read(state(h) + 0x9a8);
        method(h, ctx, vt::IA_SET_INDEX_BUFFER, &[index, R16_UINT, start]);
        true
    }

    /// Quads: the vertices in chunks of up to [state + 0x9c0] (a failed lock
    /// skips its chunk), each drawn as `n / 4 * 6` indices of the quad
    /// index buffer.
    fn draw_quads(h: &mut impl Host, ebp: u32, p: u32, ctx: u32) {
        let quads: u32 = h.read(state(h) + 0x7a0);
        method(h, ctx, vt::IA_SET_INDEX_BUFFER, &[quads, R16_UINT, 0]);
        if h.read::<u32>(state(h) + 0x9c0) == 0 {
            return;
        }
        let mut done = 0_i32;
        let mut total = h.read::<i32>(p + 8);
        while done < total {
            let chunk = h.read::<i32>(state(h) + 0x9c0);
            let n = total.wrapping_sub(done).min(chunk);
            let stride: u32 = h.read(ebp - 4);
            let bytes = n.cast_unsigned().wrapping_mul(stride);
            h.write(ebp - 8, bytes);
            let at = state(h) + 0x998;
            let dst = cdecl(h, LOCK_DYNAMIC, &[at, bytes]);
            if dst != 0 {
                let base: u32 = h.read(p);
                let src = done.cast_unsigned().wrapping_mul(stride).wrapping_add(base);
                let bytes: u32 = h.read(ebp - 8);
                cdecl(h, MEMCPY, &[dst, src, bytes]);
                let offset = unmap_vertices(h);
                h.write(ebp - 0x10, offset);
                let vertices = state(h) + 0x998;
                method(
                    h,
                    ctx,
                    vt::IA_SET_VERTEX_BUFFERS,
                    &[0, 1, vertices, ebp - 4, ebp - 0x10],
                );
                let count = (n >> 2).wrapping_mul(6).cast_unsigned();
                method(h, ctx, vt::DRAW_INDEXED, &[count, 0, 0]);
            }
            done = done.wrapping_add(n);
            total = h.read::<i32>(p + 8);
        }
    }
}
