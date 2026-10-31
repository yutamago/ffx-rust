//! M5 stage 1, list `vfx`, round 5 batch 3: Iggy `GDraw` D3D11 helpers
//! around the render targets: a handle's free and a pool's reclaim, the
//! render-target unbind, the viewport and projection set-ups, the
//! texture-unit and sampler resets, the output binding.
//!
//! The immediate context's methods are called through its vtable
//! (stdcall, `this` first). Where the original hands a method a frame
//! word, the bridge takes its address (the shim passes a `StackFrame`).

use ffx_bridge::Global;

/// `g_GDrawState` (`[0x01a858b4]`).
pub const GDRAW: Global<u32> = Global::new(0x01a8_58b4);
/// The three null shader-resource views `GDraw` unbinds with.
pub const NULL_VIEWS: u32 = 0x01a8_58b8;
/// f64 2.0 (the projection's scale).
pub const TWO: u32 = 0x00b9_2270;
/// `IggyDiscardVertexBuffer`-like import (stdcall, 2 words) and
/// `IggyGDrawSendWarning` (cdecl) slots; the two thrashing warnings.
pub const DISCARD_SLOT: u32 = 0x00b0_c878;
pub const SEND_WARNING_SLOT: u32 = 0x00b0_c874;
pub const THRASH_VERTEX_TEXT: u32 = 0x00b7_a428;
pub const THRASH_TEXTURE_TEXT: u32 = 0x00b7_a448;

// ---- callees ----

/// `GDraw_releaseTextureViews_00a34bc0(handle)` (cdecl).
pub const RELEASE_TEXTURE_VIEWS: u32 = 0x00a3_4bc0;
/// `FUN_00a3b480(handle, stats)` (cdecl, this batch).
pub const FREE_HANDLE: u32 = 0x00a3_b480;
/// `FUN_00a3ce00(x, y, w, h)` (cdecl): the viewport.
pub const SET_VIEWPORT: u32 = 0x00a3_ce00;
/// `FUN_00a3b590()` (cdecl, this batch): the render target unbind.
pub const UNBIND: u32 = 0x00a3_b590;
/// `gdraw_createDepthBuffer_00a3b840(stats)` (cdecl).
pub const CREATE_DEPTH_BUFFER: u32 = 0x00a3_b840;
/// `crt_memset_thunk(p, v, n)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;

/// `ID3D11DeviceContext` vtable byte offsets used here.
pub mod vt {
    pub const PS_SET_SHADER_RESOURCES: u32 = 0x20;
    pub const PS_SET_SAMPLERS: u32 = 0x28;
    pub const IA_SET_VERTEX_BUFFERS: u32 = 0x48;
    pub const IA_SET_INDEX_BUFFER: u32 = 0x4c;
    pub const IA_SET_PRIMITIVE_TOPOLOGY: u32 = 0x60;
    pub const OM_SET_RENDER_TARGETS: u32 = 0x84;
    pub const RS_SET_STATE: u32 = 0xac;
    pub const CLEAR_STATE: u32 = 0x1b8;
}

pub mod bridge {
    use super::{
        CREATE_DEPTH_BUFFER, DISCARD_SLOT, FREE_HANDLE, GDRAW, MEMSET, NULL_VIEWS,
        RELEASE_TEXTURE_VIEWS, SEND_WARNING_SLOT, SET_VIEWPORT, THRASH_TEXTURE_TEXT,
        THRASH_VERTEX_TEXT, TWO, UNBIND, vt,
    };
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// The immediate context (`[g_GDrawState + 4]`).
    fn context(h: &impl Host) -> u32 {
        h.read(GDRAW.get(h) + 4)
    }

    /// A context method at byte offset `off` (stdcall, `ctx` first).
    fn com(h: &mut impl Host, ctx: u32, off: u32, args: &[u32]) -> u32 {
        let vtable: u32 = h.read(ctx);
        let f: u32 = h.read(vtable + off);
        let mut words = vec![ctx];
        words.extend_from_slice(args);
        h.call(f, Conv::Stdcall, &words)
    }

    /// `FUN_00a3b480(handle, stats)` -> the pool's free-ring head: the
    /// handle's views released, the stats counted (flag 8 at `+0`, `+6`
    /// incremented, the size at `+0x20`), the size given back to its pool
    /// (`[+0x18]`), its id cleared and the handle moved, size bits cleared,
    /// to the front of the pool's free ring (`+0x10`).
    pub fn free_handle(h: &mut impl Host, handle: u32, stats: u32) -> u32 {
        cdecl(h, RELEASE_TEXTURE_VIEWS, &[handle]);
        let flags: u16 = h.read(stats);
        h.write(stats, flags | 8);
        let n: u16 = h.read(stats + 6);
        h.write(stats + 6, n.wrapping_add(1));
        let size = h.read::<u32>(handle + 0x30) & 0x0fff_ffff;
        let total: u32 = h.read(stats + 0x20);
        h.write(stats + 0x20, total.wrapping_add(size));
        let size = h.read::<u32>(handle + 0x30) & 0x0fff_ffff;
        let pool: u32 = h.read(handle + 0x18);
        let used: u32 = h.read(pool);
        h.write(pool, used.wrapping_add(size));
        let next: u32 = h.read(handle + 0x20);
        let prev: u32 = h.read(handle + 0x1c);
        let state = h.read::<u32>(handle + 0x30) & 0xf000_0000;
        h.write(handle + 0x30, state);
        let pool: u32 = h.read(handle + 0x18);
        h.write(handle + 0x14, 0_u32);
        h.write(next + 0x1c, prev);
        let prev: u32 = h.read(handle + 0x1c);
        let next: u32 = h.read(handle + 0x20);
        h.write(prev + 0x20, next);
        let head = pool.wrapping_add(0x10);
        h.write(handle + 0x1c, head);
        let first: u32 = h.read(head + 0x20);
        h.write(handle + 0x20, first);
        h.write(first + 0x1c, handle);
        let prev: u32 = h.read(handle + 0x1c);
        h.write(prev + 0x20, handle);
        let v = h.read::<u32>(handle + 0x30) & 0x0fff_ffff;
        h.write(handle + 0x30, v);
        prev
    }

    /// `GDraw_pool_reclaim_00a3b500(pool, stats)` -> 1, or 0 with an
    /// empty used ring (`+0x64` back at `+0x48`, or none): the oldest used
    /// handle's buffer discarded (a vertex pool, flag 1 of `+0xc`), a
    /// thrashing warning once (flag 2) when it was used since the pool's
    /// frame (`+0x1a0`, 64 bits, against the handle's `+0x28`), and freed.
    pub fn pool_reclaim(h: &mut impl Host, pool: u32, stats: u32) -> u32 {
        let r: u32 = h.read(pool + 0x64);
        if r == pool + 0x48 || r == 0 {
            return 0;
        }
        if h.read::<u8>(pool + 0xc) & 1 != 0 {
            let buf: u32 = h.read(r + 0x14);
            if buf != 0 {
                let f: u32 = h.read(h.rebase(DISCARD_SLOT));
                h.call(f, Conv::Stdcall, &[buf, r]);
            }
        }
        let frame =
            u64::from(h.read::<u32>(pool + 0x1a0)) | (u64::from(h.read::<u32>(pool + 0x1a4)) << 32);
        let used = u64::from(h.read::<u32>(r + 0x28)) | (u64::from(h.read::<u32>(r + 0x2c)) << 32);
        if frame < used {
            let flags: u32 = h.read(pool + 0xc);
            if flags & 2 == 0 {
                let text = h.rebase(if flags & 1 != 0 {
                    THRASH_VERTEX_TEXT
                } else {
                    THRASH_TEXTURE_TEXT
                });
                let f: u32 = h.read(h.rebase(SEND_WARNING_SLOT));
                h.call(f, Conv::Cdecl, &[0, text]);
                let flags: u32 = h.read(pool + 0xc);
                h.write(pool + 0xc, flags | 2);
            }
        }
        cdecl(h, FREE_HANDLE, &[r, stats]);
        1
    }

    /// `gdraw_unbindTarget_00a3b590()` -> the render target to restore:
    /// with a render-target stack entry (`[+0xaac]` set), the shader views,
    /// vertex buffer (the null buffer, stride and offset in `frame`, the
    /// original's `ebp - 8`) and index buffer are unbound and the entry's
    /// `+8` returned; without, the main target (`+0x780`).
    pub fn unbind_target(h: &mut impl Host, frame: u32) -> u32 {
        let s = GDRAW.get(h);
        let top: u32 = h.read(s + 0xaac);
        if h.read::<u32>(top) == 0 {
            return h.read(s + 0x780);
        }
        let ctx: u32 = h.read(s + 4);
        h.write(frame, 0_u32);
        h.write(frame + 4, 0_u32);
        let views = h.rebase(NULL_VIEWS);
        com(h, ctx, vt::PS_SET_SHADER_RESOURCES, &[0, 3, views]);
        com(
            h,
            ctx,
            vt::IA_SET_VERTEX_BUFFERS,
            &[0, 1, frame, frame + 4, frame + 4],
        );
        com(h, ctx, vt::IA_SET_INDEX_BUFFER, &[0, 0, 0]);
        let top: u32 = h.read(GDRAW.get(h) + 0xaac);
        let entry: u32 = h.read(top);
        h.read(entry + 8)
    }

    /// `gdraw_setViewportForTarget_00a3cd30()` -> the viewport call's
    /// `eax`: the tile (`+0x6c8`, `+0x6cc`, `+0x6d8`, `+0x6dc`) on the main
    /// target, a stack entry's size, or the full screen (`+0x6e0`,
    /// `+0x6e4`) when drawing whole (`+0x6f8`) or the entry has none.
    pub fn set_viewport_for_target(h: &mut impl Host) -> u32 {
        let s = GDRAW.get(h);
        let full = [0, 0, h.read(s + 0x6e0), h.read(s + 0x6e4)];
        let args = if h.read::<u32>(s + 0x6f8) != 0 {
            full
        } else {
            let top: u32 = h.read(s + 0xaac);
            if top == s + 0x9cc {
                [
                    h.read(s + 0x6c8),
                    h.read(s + 0x6cc),
                    h.read(s + 0x6d8),
                    h.read(s + 0x6dc),
                ]
            } else if h.read::<u32>(top + 0x18) != 0 {
                [0, 0, h.read(top + 0xc), h.read(top + 0x10)]
            } else {
                full
            }
        };
        // call width: 4
        cdecl(h, SET_VIEWPORT, &args)
    }

    /// `gdraw_setTextureUnit_00a3ccb0(unit, texture, wrap, filter)` -> the
    /// last method's `eax`: no texture unbinds the unit (a null view from
    /// the argument slot `slot`, the original's `ebp + 0xc`); a texture
    /// binds its view (`+4`) and the sampler for `wrap` and `filter`
    /// (`+0x7b4 + 4 (4 wrap + filter)`).
    pub fn set_texture_unit(
        h: &mut impl Host,
        slot: u32,
        unit: u32,
        texture: u32,
        wrap: u32,
        filter: u32,
    ) -> u32 {
        let ctx = context(h);
        if texture == 0 {
            h.write(slot, 0_u32);
            return com(h, ctx, vt::PS_SET_SHADER_RESOURCES, &[unit, 1, slot]);
        }
        com(h, ctx, vt::PS_SET_SHADER_RESOURCES, &[unit, 1, texture + 4]);
        let k = filter.wrapping_add(wrap.wrapping_mul(4));
        let samplers = GDRAW.get(h).wrapping_add(k.wrapping_mul(4)) + 0x7b4;
        com(h, ctx, vt::PS_SET_SAMPLERS, &[unit, 1, samplers])
    }

    /// `gdraw_resetState_00a3c280()` -> the `GDraw` state: the context's
    /// state cleared, triangle lists, unit 7's view (`+0x79c`) and sampler
    /// and units 0..2's samplers (`+0x7b4`) set, the cached shaders
    /// (`+0x9c4`, `+0x9c8`) forgotten.
    pub fn reset_state(h: &mut impl Host) -> u32 {
        let ctx = context(h);
        com(h, ctx, vt::CLEAR_STATE, &[]);
        com(h, ctx, vt::IA_SET_PRIMITIVE_TOPOLOGY, &[4]);
        let views = GDRAW.get(h) + 0x79c;
        com(h, ctx, vt::PS_SET_SHADER_RESOURCES, &[7, 1, views]);
        let samplers = GDRAW.get(h) + 0x7b4;
        com(h, ctx, vt::PS_SET_SAMPLERS, &[7, 1, samplers]);
        for unit in 0..3 {
            let samplers = GDRAW.get(h) + 0x7b4;
            com(h, ctx, vt::PS_SET_SAMPLERS, &[unit, 1, samplers]);
        }
        let s = GDRAW.get(h);
        h.write(s + 0x9c4, 0xffff_ffff_u32);
        h.write(s + 0x9c8, 0xffff_ffff_u32);
        s
    }

    /// `fild` then `fstp dword`: an integer as the float the original
    /// keeps.
    fn int_float(v: u32) -> u32 {
        x87::store(f64::from(v.cast_signed()))
    }

    /// `gdraw_setProjection_00a3c450(x0, x1, y0, y1)` -> the `GDraw`
    /// state: the 2D scale and offset (`+0x70c` = 2 / (x1 - x0), `+0x710`
    /// = 2 / (y1 - y0), `+0x714` = (x0 + x1) / (x0 - x1), `+0x718` = (y0 +
    /// y1) / (y0 - y1), each operand an `f32`), copied into the zeroed
    /// matrix at `+0x71c` (`+0x71c`, `+0x730`, `+0x73c`, `+0x740`, 1.0 at
    /// `+0x744` and `+0x748`); the argument slots (`slots`, `ebp + 8`) are
    /// the original's temporaries.
    pub fn set_projection(
        h: &mut impl Host,
        slots: u32,
        x0: u32,
        x1: u32,
        y0: u32,
        y1: u32,
    ) -> u32 {
        let s = GDRAW.get(h);
        let two = f64::from_bits(h.read(h.rebase(TWO)));
        let w = int_float(x1.wrapping_sub(x0));
        let sx = x87::store(x87::div(two, x87::load(w)));
        let hh = int_float(y1.wrapping_sub(y0));
        h.write(s + 0x70c, sx);
        let sy = x87::store(x87::div(two, x87::load(hh)));
        h.write(slots, s);
        h.write(s + 0x710, sy);
        let ax = int_float(x0.wrapping_add(x1));
        let dx = int_float(x0.wrapping_sub(x1));
        let ox = x87::store(x87::div(x87::load(ax), x87::load(dx)));
        h.write(slots + 0xc, s);
        h.write(s + 0x714, ox);
        let ay = int_float(y0.wrapping_add(y1));
        let dy = int_float(y0.wrapping_sub(y1));
        let oy = x87::store(x87::div(x87::load(ay), x87::load(dy)));
        h.write(slots + 4, dy);
        h.write(s + 0x718, oy);
        cdecl(h, MEMSET, &[s + 0x71c, 0, 0x30]);
        for (from, to) in [
            (0x70c, 0x71c),
            (0x710, 0x730),
            (0x714, 0x73c),
            (0x718, 0x740),
        ] {
            let v: u32 = h.read(s + from);
            h.write(s + to, x87::store(x87::load(v)));
        }
        let one = 1.0_f32.to_bits();
        h.write(s + 0x744, one);
        h.write(s + 0x748, one);
        s
    }

    /// `gdraw_bindOutput_00a3c570(stats)` -> `RSSetState`'s `eax`: the
    /// target `gdraw_unbindTarget_00a3b590` returns (kept in `frame`, the
    /// original's `ebp - 4`) bound as the render target, with the main
    /// depth buffer (`+0x784`) and rasterizer state `+0x7ac` indexed by
    /// `+0x790` for the main target, else with a depth buffer only when
    /// the stack entry asks for one (`+0x14` bits 2-3:
    /// `gdraw_createDepthBuffer_00a3b840`) and state `+0x7ac`; the stats
    /// count it (flag 0x20, `+0xc`).
    pub fn bind_output(h: &mut impl Host, frame: u32, stats: u32) -> u32 {
        let target = cdecl(h, UNBIND, &[]);
        h.write(frame, target);
        let s = GDRAW.get(h);
        let r = if target == h.read::<u32>(s + 0x780) {
            let ctx: u32 = h.read(s + 4);
            let depth: u32 = h.read(s + 0x784);
            com(h, ctx, vt::OM_SET_RENDER_TARGETS, &[1, frame, depth]);
            let s = GDRAW.get(h);
            let k: u32 = h.read(s + 0x790);
            let ctx: u32 = h.read(s + 4);
            let state: u32 = h.read(s.wrapping_add(k.wrapping_mul(4)) + 0x7ac);
            com(h, ctx, vt::RS_SET_STATE, &[state])
        } else {
            let top: u32 = h.read(s + 0xaac);
            let depth = if h.read::<u8>(top + 0x14) & 0xc != 0 {
                cdecl(h, CREATE_DEPTH_BUFFER, &[stats])
            } else {
                0
            };
            let ctx = context(h);
            com(h, ctx, vt::OM_SET_RENDER_TARGETS, &[1, frame, depth]);
            let s = GDRAW.get(h);
            let ctx: u32 = h.read(s + 4);
            let state: u32 = h.read(s + 0x7ac);
            com(h, ctx, vt::RS_SET_STATE, &[state])
        };
        let flags: u16 = h.read(stats);
        h.write(stats, flags | 0x20);
        let n: u16 = h.read(stats + 0xc);
        h.write(stats + 0xc, n.wrapping_add(1));
        r
    }
}
