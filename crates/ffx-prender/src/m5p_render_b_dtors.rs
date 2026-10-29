//! M5 stage 1, list `phyre-render-b`: the renderers' destructor bodies
//! and constructor, the character-lighting and shader managers' tree
//! teardown, the render interface's blend/sampler state setters, the
//! shader resources' COM release loops, and two `PArray` helpers.
//!
//! Calls to the engine's empty destructors (`0x00613dd0`, `0x005b3640`,
//! `0x005852f0`: a lone `ret`) are left out: they change nothing.

/// `Phyre_PhyreFree`, `operator_delete` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Phyre_PhyreMalloc_0042fb80(size, align)` (cdecl).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `Phyre_PRedBlackTree_clear`, `_orphanAllContents`, `_remove(tree;
/// node)` and `Phyre_PFreeList_reset`, `_dtor_PFreeList` (thiscall).
pub const TREE_CLEAR: u32 = 0x0057_4760;
pub const TREE_ORPHAN_ALL: u32 = 0x0057_4970;
pub const TREE_REMOVE: u32 = 0x0057_5000;
pub const FREE_LIST_RESET: u32 = 0x0049_cc10;
pub const FREE_LIST_DTOR: u32 = 0x0049_c5b0;
/// `Phyre_PRendering_PooledHandle_release_0061c890` (thiscall).
pub const POOLED_HANDLE_RELEASE: u32 = 0x0061_c890;
/// `Phyre_PRendering_PRendererBase_vftable`.
pub const RENDERER_BASE_VTABLE: u32 = 0x00b3_63d8;
/// The CRT's `eh vector destructor iterator(p, size, count, dtor)`
/// (stdcall) and the empty `PRenderSurface` destructor it runs.
pub const EH_VECTOR_DTOR: u32 = 0x0094_96d2;
pub const RENDER_SURFACE_DTOR: u32 = 0x005b_3640;
/// The renderer job constructor's vtables and the clear it calls.
pub const JOB_BASE_VTABLE: u32 = 0x00b2_a2e0;
pub const JOB_VTABLE: u32 = 0x00b3_6880;
pub const CLEAR_WORD0: u32 = 0x0061_3fb0;
/// `0x005c9930(this)` (cdecl) and the two `PArray`s `0x005c82a0`
/// releases.
pub const RECORDS_RELEASE: u32 = 0x005c_9930;
/// The ring-buffer job allocation of `0x005b3680`:
/// `Phyre_PPreprocessRingBuffer_GetSingleton` (cdecl),
/// `Phyre_PPreprocessRingBuffer_allocate(ring; size)` (thiscall),
/// `Phyre_PThreadPool_GetSingleton` (cdecl), its slot 6 (thiscall, the
/// help-out), `Phyre_PhyreOS_Sleep(ms)` (cdecl) and the job constructor
/// `0x005b4750`.
pub const RING_SINGLETON: u32 = 0x005c_d010;
pub const RING_ALLOCATE: u32 = 0x005c_d0f0;
pub const POOL_SINGLETON: u32 = 0x0044_cd30;
pub const SLEEP: u32 = 0x0043_0030;
pub const JOB_CTOR: u32 = 0x005b_4750;
/// `PRendererD3D11_mapBlendFactor(f)`, `Phyre_PRendering_MapMode8_0058a9a0(m)`
/// and `0x0058aaa0(op)` (cdecl).
pub const MAP_BLEND: u32 = 0x0058_a8f0;
pub const MAP_MODE8: u32 = 0x0058_a9a0;
pub const MAP_OP: u32 = 0x0058_aaa0;
/// `PShaderD3D11_releaseViews_0059c950` (thiscall).
pub const RELEASE_VIEWS: u32 = 0x0059_c950;
/// `TextUtil_setEnabled(util; flag)`, `TextUtil_resize(util; w, h)`
/// (thiscall).
pub const TEXT_UTIL_SET_ENABLED: u32 = 0x006f_68f0;
pub const TEXT_UTIL_RESIZE: u32 = 0x006f_6670;
/// `RenderTarget_release_00440ea0` (thiscall).
pub const RENDER_TARGET_RELEASE: u32 = 0x0044_0ea0;
/// The `PArray<PBitmapFontCharInfo>` descriptor made on first use:
/// guard, descriptor, constructor, flags word, vtable, `atexit`
/// destructor; and `PClassDescriptor::swapEndiannessOfObject(desc; p,
/// flag)` (thiscall).
pub const CHAR_INFO_GUARD: u32 = 0x01a8_5144;
pub const CHAR_INFO_DESC: u32 = 0x01a8_50b0;
pub const CHAR_INFO_CTOR: u32 = 0x00a2_b780;
pub const CHAR_INFO_FLAGS: u32 = 0x01a8_5140;
pub const CHAR_INFO_VTABLE: u32 = 0x00b7_926c;
pub const CHAR_INFO_ATEXIT: u32 = 0x00b0_90e0;
pub const ATEXIT: u32 = 0x0094_9207;
pub const SWAP_ENDIANNESS: u32 = 0x0043_d6a0;
/// The 16-byte element copy and zeroing `0x005af070` uses (cdecl).
pub const COPY16: u32 = 0x005a_8800;
pub const ZERO16: u32 = 0x005a_d320;

/// The render interface's mode index (0..4) as the state value (1..5);
/// 1 past it.
#[must_use]
pub const fn mode_value(i: u32) -> u32 {
    if i <= 4 { i + 1 } else { 1 }
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, f: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Cdecl, args)
    }

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    fn release_com(h: &mut impl Host, at: u32) {
        let obj: u32 = h.read(at);
        if obj != 0 {
            let vt: u32 = h.read(obj);
            let f: u32 = h.read(vt + 8);
            h.call(f, Conv::Stdcall, &[obj]);
            h.write(at, 0u32);
        }
    }

    /// `0x006a6720`, `0x006a6780(tree)`: the tree cleared twice (the
    /// destructor and its base); the second clear's `eax`.
    pub fn tree_clear_twice(h: &mut impl Host, this: u32) -> u32 {
        this_call(h, TREE_CLEAR, this, &[]);
        this_call(h, TREE_CLEAR, this, &[])
    }

    /// `0x006a67e0(map)`: the tree's contents orphaned, its free list
    /// (`+0x18`) reset and destroyed, the tree cleared twice.
    pub fn tree_map_dtor(h: &mut impl Host, this: u32) -> u32 {
        this_call(h, TREE_ORPHAN_ALL, this, &[]);
        free_list_and_clear(h, this)
    }

    fn free_list_and_clear(h: &mut impl Host, this: u32) -> u32 {
        this_call(h, FREE_LIST_RESET, this + 0x18, &[]);
        this_call(h, FREE_LIST_DTOR, this + 0x18, &[]);
        tree_clear_twice(h, this)
    }

    /// `0x006a68f0(map)`: every node removed (`+0xc` until it is the
    /// tree itself or 0), then as [`tree_map_dtor`] without the orphaning.
    pub fn tree_map_remove_all(h: &mut impl Host, this: u32) -> u32 {
        loop {
            let node: u32 = h.read(this + 0xc);
            if node == this || node == 0 {
                break;
            }
            this_call(h, TREE_REMOVE, this, &[node]);
        }
        free_list_and_clear(h, this)
    }

    /// `0x00614300(this)`: the pooled handle released; its `eax`.
    pub fn handle_dtor(h: &mut impl Host, this: u32) -> u32 {
        this_call(h, POOLED_HANDLE_RELEASE, this, &[])
    }

    /// `0x005b49e0`, and with `flags` the deleting `0x005b4fa0`,
    /// `0x005b5010(renderer; flags)`: the base vtable, `PhyreFree` when
    /// `flags & 1`; `this`.
    pub fn renderer_base_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(RENDERER_BASE_VTABLE);
        h.write(this, vt);
        if flags & 1 != 0 {
            cdecl(h, PHYRE_FREE, &[this]);
        }
        this
    }

    /// `0x005b4950`, and with `flags` the deleting `0x005b4ef0`: the four
    /// 16-byte render surfaces at `+0xe8` destroyed through the CRT's
    /// vector destructor, then [`renderer_base_dtor`].
    pub fn renderer_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let dtor = h.rebase(RENDER_SURFACE_DTOR);
        h.call_sym(EH_VECTOR_DTOR, Conv::Stdcall, &[this + 0xe8, 0x10, 4, dtor]);
        renderer_base_dtor(h, this, flags)
    }

    /// `0x005b4750(job)`: the renderer job's constructor: base vtable and
    /// 0 at `+4`/`+8`, its vtable, the cursor at `+0x18` cleared, six
    /// words from `+0x1c` zeroed; `this`.
    pub fn job_ctor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(JOB_BASE_VTABLE);
        h.write(this + 4, vt);
        h.write(this + 8, 0u32);
        let vt = h.rebase(JOB_VTABLE);
        h.write(this, vt);
        this_call(h, CLEAR_WORD0, this + 0x18, &[]);
        h.write(this + 0x1c, [0u32; 6]);
        this
    }

    /// `0x005b3680()`: a 0x40-byte job from the preprocess ring buffer
    /// (helping the thread pool and sleeping 1 ms until one is free),
    /// constructed at `+0x10` with the block in its `+0xc`; the job.
    pub fn new_job(h: &mut impl Host) -> u32 {
        let mut waited = false;
        let block = loop {
            if waited {
                let pool = cdecl(h, POOL_SINGLETON, &[]);
                h.call_virtual(pool, 6, &[]);
                cdecl(h, SLEEP, &[1]);
            }
            waited = true;
            let ring = cdecl(h, RING_SINGLETON, &[]);
            let block = this_call(h, RING_ALLOCATE, ring, &[0x40]);
            if block != 0 {
                break block;
            }
        };
        let job = this_call(h, JOB_CTOR, block.wrapping_add(0x10), &[]);
        h.write(job + 0xc, block);
        job
    }

    /// `0x005c82a0(this)`: the records released (`0x005c9930`), then the
    /// owned buffers of the `PArray`s at `+0x10` and `+0x8` freed and
    /// both arrays cleared.
    pub fn records_dtor(h: &mut impl Host, this: u32) {
        cdecl(h, RECORDS_RELEASE, &[this]);
        for at in [0x10, 0x8] {
            let count: u32 = h.read(this + at);
            let buffer: u32 = h.read(this + at + 4);
            if count & 0x8000_0000 == 0 && buffer != 0 {
                cdecl(h, PHYRE_FREE, &[buffer]);
            }
            h.write(this + at + 4, 0u32);
            h.write(this + at, 0u32);
        }
    }

    /// `0x00599820(state; a, b)`: `+0x74` and `+0x80` the mode values of
    /// `a` and `b` ([`mode_value`]); 0.
    pub fn set_modes(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        h.write(this + 0x74, mode_value(a));
        h.write(this + 0x80, mode_value(b));
        0
    }

    /// `0x0059a070(state; mode)`: `+0x578` = 1, 2, 3 for modes 0, 1, 2
    /// (left alone otherwise), `+0x57c` = 0; 0.
    pub fn set_578(h: &mut impl Host, this: u32, mode: u32) -> u32 {
        match mode {
            0 | 1 => {
                h.write(this + 0x578, mode + 1);
                h.write(this + 0x57c, 0u32);
            }
            2 => {
                h.write(this + 0x57c, 0u32);
                h.write(this + 0x578, 3u32);
            }
            _ => h.write(this + 0x57c, 0u32),
        }
        0
    }

    /// `0x00599950(state; enable, src, dst, src_alpha, dst_alpha)`: the
    /// blend state: `+0x5d4` = 0, `+0x5d8` the enable byte, the four
    /// factors mapped to `+0x5dc`, `+0x5e0`, `+0x5e8`, `+0x5ec`; 0.
    pub fn set_blend(h: &mut impl Host, this: u32, enable: u32, factors: [u32; 4]) -> u32 {
        h.write(this + 0x5d4, 0u32);
        h.write(this + 0x5d8, enable & 0xff);
        for (at, f) in [0x5dc, 0x5e0, 0x5e8, 0x5ec].into_iter().zip(factors) {
            let v = cdecl(h, MAP_BLEND, &[f]);
            h.write(this + at, v);
        }
        0
    }

    /// `0x0059bac0(state; 15 words)`: the depth-stencil state: words 0,
    /// 3, 7 as bytes to `+0x5a8` (a word), `+0x5ac`, `+0x5ad`; words 1
    /// and 8 through `MapMode8` to `+0x5bc`, `+0x5cc`; words 4, 5, 6 and
    /// 11, 12, 13 through `0x0058aaa0` to `+0x5b0..0x5b8` and
    /// `+0x5c0..0x5c8`; 0.
    pub fn set_depth_stencil(h: &mut impl Host, this: u32, w: [u32; 15]) -> u32 {
        h.write(this + 0x5a8, w[0] & 0xff);
        h.write(this + 0x5ac, w[3] as u8);
        h.write(this + 0x5ad, w[7] as u8);
        let v = cdecl(h, MAP_MODE8, &[w[1]]);
        h.write(this + 0x5bc, v);
        for (at, i) in [(0x5b0, 4), (0x5b4, 5), (0x5b8, 6)] {
            let v = cdecl(h, MAP_OP, &[w[i]]);
            h.write(this + at, v);
        }
        let v = cdecl(h, MAP_MODE8, &[w[8]]);
        h.write(this + 0x5cc, v);
        for (at, i) in [(0x5c0, 11), (0x5c4, 12), (0x5c8, 13)] {
            let v = cdecl(h, MAP_OP, &[w[i]]);
            h.write(this + at, v);
        }
        0
    }

    /// `0x0059c190(shader)`: its views released, the sixteen COM objects
    /// at `+0..0x3c` and those at `+0x44`, `+0x4c`, `+0x50`, `+0x54`,
    /// `+0x48` (that order) released and cleared; 0.
    pub fn shader_release(h: &mut impl Host, this: u32) -> u32 {
        this_call(h, RELEASE_VIEWS, this + 0x60, &[]);
        for k in 0..16 {
            release_com(h, this + 4 * k);
        }
        for at in [0x44, 0x4c, 0x50, 0x54, 0x48] {
            release_com(h, this + at);
        }
        0
    }

    /// `0x0059c370(this)`: the COM objects at `+0x20`, `+0x24`, `+0x28`,
    /// `+0x2c`, `+0`, `+8`, `+4`, `+0xc` (that order) released and
    /// cleared. Its caller `0x0059c400` reads no `eax`.
    pub fn release_eight(h: &mut impl Host, this: u32) {
        for at in [0x20, 0x24, 0x28, 0x2c, 0, 8, 4, 0xc] {
            release_com(h, this + at);
        }
    }

    /// `Phyre_CharacterLightingTweakUtil_setEnabled(tw; on)`: `+0xc8` =
    /// `on` and the utility switched on (byte `+0`); the text utility at
    /// `+4` follows, and when on it is resized to `+0xe0`/`+0xe4`; the
    /// last call's `eax`.
    pub fn tweak_set_enabled(h: &mut impl Host, this: u32, on: u32) -> u32 {
        let on = on & 0xff != 0;
        let en = u32::from(on && h.read::<u8>(this) != 0);
        h.write(this + 0xc8, en as u8);
        let r = this_call(h, TEXT_UTIL_SET_ENABLED, this + 4, &[en]);
        if on && h.read::<u8>(this + 0xc8) != 0 {
            let w: u32 = h.read(this + 0xe0);
            let hgt: u32 = h.read(this + 0xe4);
            return this_call(h, TEXT_UTIL_RESIZE, this + 4, &[w, hgt]);
        }
        r
    }

    /// `0x006ab540(this)`: the `PArray` at `[+0xf0]` released (nothing
    /// for an empty one) and the pointer cleared; the render targets at
    /// `+0`, `+0x50`, `+0xa0` released (byte `+0x1cc` cleared before the
    /// third), the tree at `+0x158` orphaned and the free list at `+0x170`
    /// reset (a tail call: its `eax`).
    pub fn tweak_release(h: &mut impl Host, this: u32) -> u32 {
        let arr: u32 = h.read(this + 0xf0);
        if arr != 0 {
            let count: u32 = h.read(arr);
            if count != 0 {
                let buffer: u32 = h.read(arr + 4);
                if buffer != 0 && count & 0x8000_0000 == 0 {
                    cdecl(h, PHYRE_FREE, &[buffer]);
                }
                h.write(arr + 4, 0u32);
                h.write(arr, 0u32);
            }
            h.write(this + 0xf0, 0u32);
        }
        this_call(h, RENDER_TARGET_RELEASE, this, &[]);
        this_call(h, RENDER_TARGET_RELEASE, this + 0x50, &[]);
        h.write(this + 0x1cc, 0u8);
        this_call(h, RENDER_TARGET_RELEASE, this + 0xa0, &[]);
        this_call(h, TREE_ORPHAN_ALL, this + 0x158, &[]);
        this_call(h, FREE_LIST_RESET, this + 0x170, &[])
    }

    /// `0x00a2b3b0` (flag 1), `0x00a2b510` (flag 0) `(p)`: the
    /// `PArray<PBitmapFontCharInfo>` descriptor made on first use, then
    /// its `swapEndiannessOfObject(p, flag)`; its `eax`.
    pub fn char_info_swap(h: &mut impl Host, p: u32, flag: u32) -> u32 {
        let guard = h.rebase(CHAR_INFO_GUARD);
        let g: u32 = h.read(guard);
        let desc = h.rebase(CHAR_INFO_DESC);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            this_call(h, CHAR_INFO_CTOR, desc, &[]);
            let flags_at = h.rebase(CHAR_INFO_FLAGS);
            let flags: u32 = h.read(flags_at);
            h.write(flags_at, flags & !2);
            let vt = h.rebase(CHAR_INFO_VTABLE);
            h.write(desc, vt);
            let f = h.rebase(CHAR_INFO_ATEXIT);
            cdecl(h, ATEXIT, &[f]);
        }
        this_call(h, SWAP_ENDIANNESS, desc, &[p, flag])
    }

    /// `0x005af070(array; n)`: a `PArray` of 16-byte elements resized:
    /// nothing when `n` equals the count word; else a new buffer (none
    /// for 0) with the old elements that fit copied and the rest zeroed
    /// (0xd when the allocation fails), the old owned non-null buffer
    /// freed when it differs; 0.
    pub fn resize16(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let word: u32 = h.read(this);
        if n == word {
            return 0;
        }
        let count = word & 0x7fff_ffff;
        let keep = if n < count { n } else { count };
        let old: u32 = h.read(this + 4);
        let mut p = 0;
        if n != 0 {
            p = cdecl(h, PHYRE_MALLOC, &[n.wrapping_mul(16), 4]);
            if p == 0 {
                return 0xd;
            }
            cdecl(h, COPY16, &[p, old, keep]);
            cdecl(
                h,
                ZERO16,
                &[p.wrapping_add(keep.wrapping_mul(16)), n - keep],
            );
        }
        let old: u32 = h.read(this + 4);
        let word: u32 = h.read(this);
        if old != p && word & 0x8000_0000 == 0 && old != 0 {
            cdecl(h, PHYRE_FREE, &[old]);
        }
        h.write(this + 4, p);
        h.write(this, n);
        0
    }
}
