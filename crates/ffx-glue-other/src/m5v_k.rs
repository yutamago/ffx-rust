//! M5 stage 1, list `vfx`, round 5 batch 6: Iggy `GDraw` D3D11's
//! external-texture wrap and its context teardown.
//!
//! Frame buffers handed to callees come in as addresses (the shims pass a
//! `StackFrame`).

use ffx_bridge::Global;

/// `g_GDrawState` (`[0x01a858b4]`).
pub const GDRAW: Global<u32> = Global::new(0x01a8_58b4);
/// The allocator's free (an Iggy import slot; stdcall, one word).
pub const FREE_SLOT: u32 = 0x00b0_c870;

// ---- callees ----

/// `crt_memset_thunk(p, v, n)` (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// `opd_config_memory(pool, owner, stats)` (cdecl): a handle from the
/// texture pool (named after Fahrenheit's match; it allocates here).
pub const POOL_ALLOC: u32 = 0x00a3_b210;
/// `GDraw_releaseSlots_00a3cfc0()`, `FUN_00a35c90()`,
/// `GDraw_freeResources_00a36300(stats)`, `GDraw_pool_freeAll_00a3b380(pool,
/// stats)` (cdecl).
pub const RELEASE_SLOTS: u32 = 0x00a3_cfc0;
pub const RELEASE_SHADERS: u32 = 0x00a3_5c90;
pub const FREE_RESOURCES: u32 = 0x00a3_6300;
pub const POOL_FREE_ALL: u32 = 0x00a3_b380;
/// `ID3D11DeviceContext::ClearState` (vtable byte offset).
pub const CLEAR_STATE: u32 = 0x1b8;

pub mod bridge {
    use super::{
        CLEAR_STATE, FREE_RESOURCES, FREE_SLOT, GDRAW, MEMSET, POOL_ALLOC, POOL_FREE_ALL,
        RELEASE_SHADERS, RELEASE_SLOTS,
    };
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// Zeroes the 0x30-byte stats at `stats` as the originals do.
    fn clear_stats(h: &mut impl Host, stats: u32) {
        h.write(stats, 0_u16);
        cdecl(h, MEMSET, &[stats + 2, 0, 0x2e]);
    }

    /// Releases the COM object at `at` (vtable `+8`, stdcall) and clears
    /// the word.
    fn release(h: &mut impl Host, at: u32) {
        let obj: u32 = h.read(at);
        if obj != 0 {
            let vt: u32 = h.read(obj);
            let f: u32 = h.read(vt + 8);
            h.call(f, Conv::Stdcall, &[obj]);
            h.write(at, 0_u32);
        }
    }

    /// `gdraw_wrapTexture_00a383a0(texture)` -> the handle: a handle from
    /// the texture pool (`+0xab0`, zeroed stats at `stats`, the original's
    /// `ebp - 0x30`) holding `texture` (`+4`, view and id cleared, 1 x 1),
    /// moved to the front of the pool's wrapped ring (`+0x128`) in state 5.
    pub fn wrap_texture(h: &mut impl Host, stats: u32, texture: u32) -> u32 {
        clear_stats(h, stats);
        let pool: u32 = h.read(GDRAW.get(h) + 0xab0);
        let r = cdecl(h, POOL_ALLOC, &[pool, 0, stats]);
        let state = h.read::<u32>(r + 0x30) & 0xf000_0000;
        h.write(r + 0x30, state);
        let prev: u32 = h.read(r + 0x1c);
        let pool: u32 = h.read(r + 0x18);
        h.write(r + 4, texture);
        let next: u32 = h.read(r + 0x20);
        for (off, v) in [(0_u32, 0_u32), (8, 0), (0xc, 1), (0x10, 1), (0x14, 0)] {
            h.write(r + off, v);
        }
        h.write(next + 0x1c, prev);
        let prev: u32 = h.read(r + 0x1c);
        let next: u32 = h.read(r + 0x20);
        h.write(prev + 0x20, next);
        let head = pool.wrapping_add(0x128);
        h.write(r + 0x1c, head);
        let first: u32 = h.read(head + 0x20);
        h.write(r + 0x20, first);
        h.write(first + 0x1c, r);
        let prev: u32 = h.read(r + 0x1c);
        h.write(prev + 0x20, r);
        let v = (h.read::<u32>(r + 0x30) & 0x0fff_ffff) | 0x5000_0000;
        h.write(r + 0x30, v);
        r
    }

    /// `gdraw_destroyContext_00a37960()`: with a `GDraw` state that is up
    /// (`+0`), the context's state cleared, the slots and shaders released,
    /// the views and buffers `+0x798`, `+0x79c`, `+0x998`, `+0x9a8`
    /// released, the resources and both pools (`+0xab0`, `+0xab4`) freed
    /// (zeroed stats at `stats`, the original's `ebp - 0x30`) and `+0`
    /// cleared; then the pools and the state go back to the allocator and
    /// `g_GDrawState` is cleared.
    pub fn destroy_context(h: &mut impl Host, stats: u32) {
        let s = GDRAW.get(h);
        if s == 0 {
            return;
        }
        if h.read::<u32>(s) != 0 {
            clear_stats(h, stats);
            let ctx: u32 = h.read(s + 4);
            let vt: u32 = h.read(ctx);
            let f: u32 = h.read(vt + CLEAR_STATE);
            h.call(f, Conv::Stdcall, &[ctx]);
            cdecl(h, RELEASE_SLOTS, &[]);
            cdecl(h, RELEASE_SHADERS, &[]);
            for off in [0x798, 0x79c, 0x998, 0x9a8] {
                let s = GDRAW.get(h);
                release(h, s + off);
            }
            cdecl(h, FREE_RESOURCES, &[stats]);
            for off in [0xab0, 0xab4] {
                let pool: u32 = h.read(GDRAW.get(h) + off);
                if pool != 0 {
                    cdecl(h, POOL_FREE_ALL, &[pool, stats]);
                }
            }
            let s = GDRAW.get(h);
            h.write(s, 0_u32);
        }
        let free: u32 = h.read(h.rebase(FREE_SLOT));
        for off in [0xab0, 0xab4] {
            let pool: u32 = h.read(GDRAW.get(h) + off);
            if pool != 0 {
                h.call(free, Conv::Stdcall, &[pool]);
            }
        }
        let s = GDRAW.get(h);
        h.call(free, Conv::Stdcall, &[s]);
        GDRAW.set(h, 0);
    }
}
