//! M5 stage 1, list `phyre-render-b`, post-processing part 3: the
//! particle system's arrays released, the deferred lighting's buffers
//! (constant and structured, released and remade for a size), and the
//! FXAA effect's setup.

/// `Phyre_PhyreFree`, `Phyre_PhyreMalloc(size)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
pub const PHYRE_MALLOC: u32 = 0x0042_fb40;
/// A buffer kind's callees (thiscall): bound (`al`), unbind, destructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferKind {
    pub is_bound: u32,
    pub unbind: u32,
    pub dtor: u32,
}
/// `PConstantBuffer` (`Phyre_PRendering_PConstantBufferNVN_isBound`,
/// `PConstantBufferD3D11_releaseEntries_0059c540`,
/// `Phyre_PRendering_PConstantBuffer_dtor_PConstantBuffer`).
pub const CONSTANT: BufferKind = BufferKind {
    is_bound: 0x0059_5f70,
    unbind: 0x0059_c540,
    dtor: 0x0049_fa10,
};
/// `PStructuredBuffer` (`Phyre_PRendering_PStructuredBufferNVN_isBound`,
/// `Phyre_PRendering_PStructuredBufferD3D11_unbind_0059ca60`, `0x00501cd0`).
pub const STRUCTURED: BufferKind = BufferKind {
    is_bound: 0x0059_5f80,
    unbind: 0x0059_ca60,
    dtor: 0x0050_1cd0,
};
/// `PDeferredLightingBase`'s resources release (`0x00a24a60`).
pub const LIGHTING_RELEASE: u32 = 0x00a2_4a60;
/// The structured buffer's constructor `Phyre_Ctor_005841a0` and create
/// `0x0058f2b0(buf; count, stride, flags, data)` (thiscall).
pub const STRUCTURED_CTOR: u32 = 0x0058_41a0;
pub const STRUCTURED_CREATE: u32 = 0x0058_f2b0;
/// The FXAA setup's callees: the render interface made once (as the
/// other setups), `Phyre_PCluster_create(cluster; 1, class, 0)`,
/// `Phyre_PRendering_PTexture2DBase_setDimensions`, `_updateMaxLevel`,
/// `Phyre_PCluster_allocateInstanceList(cluster; 1, class, 0)`,
/// `Phyre_PRendering_Ctor_00583a60(target; texture)` (thiscall),
/// `FullscreenMesh_createColoredTriangle_00a06120(cluster)` (cdecl).
pub const INTERFACE_GUARD: u32 = 0x00c9_628c;
pub const INTERFACE: u32 = 0x00c9_4f00;
pub const INTERFACE_CTOR: u32 = 0x004f_ee70;
pub const INTERFACE_ATEXIT: u32 = 0x00af_b710;
pub const ATEXIT: u32 = 0x0094_9207;
pub const DEFAULT_WIDTH: u32 = 0x00c9_5100;
pub const DEFAULT_HEIGHT: u32 = 0x00c9_5104;
pub const UTILITY_TARGET: u32 = 0x0194_3da0;
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
pub const SET_DIMENSIONS: u32 = 0x004c_40c0;
pub const UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
pub const ALLOCATE_INSTANCES: u32 = 0x0044_0b70;
pub const TARGET_CTOR: u32 = 0x0058_3a60;
pub const COLOURED_TRIANGLE: u32 = 0x00a0_6120;
pub const TEXTURE_CLASS: u32 = 0x00ca_37d0;
pub const TARGET_FORMAT: u32 = 0x00ca_310c;
pub const TARGET_CLASS: u32 = 0x00ca_4200;
pub const TARGET_VTABLE: u32 = 0x00b7_6848;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// `0x009fe3a0(particles)`: the five `PArray`s at `+0x3060`, `+0x3058`,
    /// `+0x3050`, `+0x3048`, `+0x3040` released (owned non-null buffers
    /// freed, both words 0).
    pub fn particles_release(h: &mut impl Host, this: u32) {
        for at in [0x3060, 0x3058, 0x3050, 0x3048, 0x3040] {
            let count: u32 = h.read(this + at);
            let buffer: u32 = h.read(this + at + 4);
            if count & 0x8000_0000 == 0 && buffer != 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buffer]);
            }
            h.write(this + at + 4, 0u32);
            h.write(this + at, 0u32);
        }
    }

    /// The buffer at `at` (when set): unbound when bound, destroyed and
    /// freed; the word cleared when `clear`.
    fn drop_buffer(h: &mut impl Host, at: u32, k: BufferKind, clear: bool) {
        let buf: u32 = h.read(at);
        if buf == 0 {
            return;
        }
        if this_call(h, k.is_bound, buf, &[]) & 0xff != 0 {
            this_call(h, k.unbind, buf, &[]);
        }
        let buf: u32 = h.read(at);
        if buf != 0 {
            this_call(h, k.dtor, buf, &[]);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[buf]);
        }
        if clear {
            h.write(at, 0u32);
        }
    }

    /// `0x00a24b00(lighting)`: the constant buffers at `+0x3fc8`,
    /// `+0x3fcc` and the structured buffers at `+0x3fd0`, `+0x3fd4`,
    /// `+0x4034`, `+0x4038` dropped and cleared, then
    /// [`LIGHTING_RELEASE`] (a tail call: its `eax`).
    pub fn lighting_buffers_release(h: &mut impl Host, this: u32) -> u32 {
        for at in [0x3fc8, 0x3fcc] {
            drop_buffer(h, this + at, CONSTANT, true);
        }
        for at in [0x3fd0, 0x3fd4, 0x4034, 0x4038] {
            drop_buffer(h, this + at, STRUCTURED, true);
        }
        this_call(h, LIGHTING_RELEASE, this, &[])
    }

    /// One structured buffer made at `at` for `count` 8-byte elements.
    fn make_buffer(h: &mut impl Host, at: u32, count: u32) -> u32 {
        let p = h.call_sym(PHYRE_MALLOC, Conv::Cdecl, &[0x48]);
        let buf = if p == 0 {
            0
        } else {
            this_call(h, STRUCTURED_CTOR, p, &[]);
            p
        };
        h.write(at, buf);
        this_call(h, STRUCTURED_CREATE, buf, &[count, 8, 8, 0])
    }

    /// `0x00a10590(lighting; w, h)`: the structured buffers at `+0x3fd0`,
    /// `+0x3fd4` dropped (the words not cleared), then for a non-zero
    /// size both made anew for `w * h` elements (the first's error
    /// returned); 0.
    pub fn lighting_buffers_resize(h: &mut impl Host, this: u32, w: u32, hgt: u32) -> u32 {
        drop_buffer(h, this + 0x3fd0, STRUCTURED, false);
        drop_buffer(h, this + 0x3fd4, STRUCTURED, false);
        if w == 0 || hgt == 0 {
            return 0;
        }
        let count = w.wrapping_mul(hgt);
        let r = make_buffer(h, this + 0x3fd0, count);
        if r != 0 {
            return r;
        }
        make_buffer(h, this + 0x3fd4, count)
    }

    /// `0x00a19970(fxaa; w, h)`: the render interface made once, the
    /// size defaulting to the interface's; a texture created in the
    /// utility cluster and sized (`0x00ca310c` format), its `+0x14` bit 1
    /// and byte `+5` cleared and levels updated, a render target
    /// allocated for it (`+0x34` of the instance list) and built; the
    /// target at `+0x5c` (0 when any step fails: 0xd), the coloured
    /// triangle at `+0x60`, byte `+0x10` = 1; 0.
    pub fn fxaa_setup(h: &mut impl Host, this: u32, w: u32, hgt: u32) -> u32 {
        let guard = h.rebase(INTERFACE_GUARD);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            let obj = h.rebase(INTERFACE);
            this_call(h, INTERFACE_CTOR, obj, &[]);
            let f = h.rebase(INTERFACE_ATEXIT);
            h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
        }
        let w = if w == 0 {
            h.read(h.rebase(DEFAULT_WIDTH))
        } else {
            w
        };
        let hgt = if hgt == 0 {
            h.read(h.rebase(DEFAULT_HEIGHT))
        } else {
            hgt
        };
        let cluster = h.rebase(UTILITY_TARGET);
        let target = make_fxaa_target(h, cluster, w, hgt);
        h.write(this + 0x5c, target);
        if target == 0 {
            return 0xd;
        }
        let tri = h.call_sym(COLOURED_TRIANGLE, Conv::Cdecl, &[cluster]);
        h.write(this + 0x60, tri);
        h.write(this + 0x10, 1u8);
        0
    }

    fn make_fxaa_target(h: &mut impl Host, cluster: u32, w: u32, hgt: u32) -> u32 {
        let class = h.rebase(TEXTURE_CLASS);
        let tex = this_call(h, CLUSTER_CREATE, cluster, &[1, class, 0]);
        if tex == 0 {
            return 0;
        }
        let format = h.rebase(TARGET_FORMAT);
        let r = this_call(h, SET_DIMENSIONS, tex, &[w, hgt, format, 0]);
        if r != 0 {
            return 0;
        }
        let flags: u32 = h.read(tex + 0x14);
        h.write(tex + 0x14, flags & !2);
        h.write(tex + 5, 0u8);
        this_call(h, UPDATE_MAX_LEVEL, tex, &[]);
        let class = h.rebase(TARGET_CLASS);
        let list = this_call(h, ALLOCATE_INSTANCES, cluster, &[1, class, 0]);
        if list == 0 {
            return 0;
        }
        let target: u32 = h.read(list + 0x34);
        if target == 0 {
            return 0;
        }
        this_call(h, TARGET_CTOR, target, &[tex]);
        h.write(target + 5, 0u8);
        let vt = h.rebase(TARGET_VTABLE);
        h.write(target + 0xc, vt);
        target
    }
}
