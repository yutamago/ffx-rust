//! M5 stage 1, list `phyre-render-b`, post-processing part 4: the
//! screen-space reflection's targets made, the glow's render-time target
//! resize, and a structured buffer's shader-resource views.

/// `FullscreenMesh_createTriangle_00a05e70(cluster)` (cdecl),
/// `Phyre_PCluster_create(cluster; 1, class, 0)`,
/// `Phyre_PRendering_PTexture2DBase_setDimensions(texture; w, h, format,
/// 0)`, `_updateMaxLevel`, `0x0064adf0(cluster; texture)` (thiscall).
pub const CREATE_TRIANGLE: u32 = 0x00a0_5e70;
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
pub const SET_DIMENSIONS: u32 = 0x004c_40c0;
pub const UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
pub const MAKE_TARGET: u32 = 0x0064_adf0;
/// The texture class, the two formats and the render target vtable.
pub const TEXTURE_CLASS: u32 = 0x00ca_37d0;
pub const FORMAT_COLOUR: u32 = 0x00ca_310c;
pub const FORMAT_DEPTH: u32 = 0x00ca_3314;
pub const TARGET_VTABLE: u32 = 0x00b7_6848;
/// The size check's callees (thiscall): bound (`al`), unbind.
pub const IS_BOUND: u32 = 0x0059_5f90;
pub const UNBIND: u32 = 0x0059_cc40;
/// The render scope lock (thiscall on a frame word), the device holder
/// (made on first use) and its device word, `Phyre_PRendering_CountOrOne14_00593ba0`.
pub const SCOPE_ENTER: u32 = 0x0046_3d50;
pub const SCOPE_LEAVE: u32 = 0x0046_3e90;
pub const HOLDER_GUARD: u32 = 0x00c9_628c;
pub const HOLDER: u32 = 0x00c9_4f00;
pub const HOLDER_CTOR: u32 = 0x004f_ee70;
pub const HOLDER_DTOR: u32 = 0x00af_b710;
pub const ATEXIT: u32 = 0x0094_9207;
pub const HOLDER_DEVICE: u32 = 0x00c9_4f6c;
pub const COUNT_OR_ONE: u32 = 0x0059_3ba0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// A render target of `w` x `h` in `format` made in `cluster`: 0 when
    /// the texture or the target can't be made or sized.
    fn half_target(h: &mut impl Host, cluster: u32, w: u32, hgt: u32, format: u32) -> u32 {
        let class = h.rebase(TEXTURE_CLASS);
        let tex = this_call(h, CLUSTER_CREATE, cluster, &[1, class, 0]);
        if tex == 0 {
            return 0;
        }
        let format = h.rebase(format);
        if this_call(h, SET_DIMENSIONS, tex, &[w, hgt, format, 0]) != 0 {
            return 0;
        }
        let flags: u32 = h.read(tex + 0x14);
        h.write(tex + 0x14, flags & !2);
        h.write(tex + 5, 0u8);
        this_call(h, UPDATE_MAX_LEVEL, tex, &[]);
        let target = this_call(h, MAKE_TARGET, cluster, &[tex]);
        if target != 0 {
            h.write(target + 5, 0u8);
            let vt = h.rebase(TARGET_VTABLE);
            h.write(target + 0xc, vt);
        }
        target
    }

    /// `0x00a17ec0(ssr; cluster, w, h)`: the fullscreen triangle
    /// (`+0x1c`; 0xe without it), then three half-size targets: colour
    /// (`+0x68`), depth (`+0x6c`), colour (`+0x70`); 0.
    pub fn ssr_targets(h: &mut impl Host, this: u32, cluster: u32, w: u32, hgt: u32) -> u32 {
        let tri = h.call_sym(CREATE_TRIANGLE, Conv::Cdecl, &[cluster]);
        h.write(this + 0x1c, tri);
        if tri == 0 {
            return 0xe;
        }
        for (at, format) in [
            (0x68, FORMAT_COLOUR),
            (0x6c, FORMAT_DEPTH),
            (0x70, FORMAT_COLOUR),
        ] {
            let t = half_target(h, cluster, w >> 1, hgt >> 1, format);
            h.write(this + at, t);
        }
        0
    }

    /// The size check of a handle (`{texture, kind}`): `Some(1)` for a
    /// null handle, a kind without a texture; an error; `None` when done.
    fn resize(h: &mut impl Host, handle: u32, w: u32, hgt: u32) -> Option<u32> {
        if handle == 0 || h.read::<u8>(handle + 4) != 0 {
            return Some(1);
        }
        let tex: u32 = h.read(handle);
        if tex == 0 {
            return Some(1);
        }
        if h.read::<u32>(tex + 0x1c) == w && h.read::<u32>(tex + 0x20) == hgt {
            return None;
        }
        if this_call(h, IS_BOUND, tex, &[]) & 0xff != 0 {
            let r = this_call(h, UNBIND, tex, &[]);
            if r != 0 {
                return Some(r);
            }
        }
        let word0: u32 = h.read(tex);
        let r = this_call(h, SET_DIMENSIONS, tex, &[w, hgt, word0, 0]);
        if r != 0 { Some(r) } else { None }
    }

    /// `0x00a23280(glow; w, h)`: the glow's three levels of two targets
    /// (`+0x90`, `+0x9c` and on) from a quarter of the size, halved per
    /// level, then the bloom target (`+0xa8`) at half the size; 1 at a
    /// missing handle or texture, an error, or 0.
    pub fn glow_resize(h: &mut impl Host, this: u32, w: u32, hgt: u32) -> u32 {
        let (mut lw, mut lh) = (w >> 2, hgt >> 2);
        for k in 0..3 {
            for at in [0x90, 0x9c] {
                let handle: u32 = h.read(this + at + 4 * k);
                if let Some(r) = resize(h, handle, lw, lh) {
                    return r;
                }
            }
            lw >>= 1;
            lh >>= 1;
        }
        let bloom: u32 = h.read(this + 0xa8);
        resize(h, bloom, w >> 1, hgt >> 1).unwrap_or(0)
    }

    /// `0x0058f5d0(buffer)`: for a buffer with shader-resource views
    /// (bit 3 of byte `+0x34`), under the render scope lock (`lock`, the
    /// original's `ebp - 0x18`) a view for each element of the `PSharray`
    /// at `+0x14` (at least one): device method `+0x1c(dev, resource
    /// word 0, 0, &word 4)` (`CreateShaderResourceView`); 9 at the first
    /// failure (negative), else 0.
    pub fn create_views(h: &mut impl Host, this: u32, lock: u32) -> u32 {
        if h.read::<u8>(this + 0x34) & 8 == 0 {
            return 0;
        }
        this_call(h, SCOPE_ENTER, lock, &[]);
        let g = h.rebase(HOLDER_GUARD);
        let v: u32 = h.read(g);
        if v & 1 == 0 {
            h.write(g, v | 1);
            let holder = h.rebase(HOLDER);
            this_call(h, HOLDER_CTOR, holder, &[]);
            let dtor = h.rebase(HOLDER_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        let dev: u32 = h.read(h.rebase(HOLDER_DEVICE));
        h.write(lock + 4, dev);
        let mut i = 0u32;
        while i < this_call(h, COUNT_OR_ONE, this, &[]) {
            let n = h.read::<u32>(this + 0x14) & 0x7fff_ffff;
            let data = if n > 1 {
                h.read(this + 0x18)
            } else if n != 0 {
                this + 0x18
            } else {
                0
            };
            let e = data.wrapping_add(i.wrapping_mul(0x14));
            let resource: u32 = h.read(e);
            let vt: u32 = h.read(dev);
            let f: u32 = h.read(vt + 0x1c);
            h.write(lock + 8, i.wrapping_mul(0x14));
            let hr = h.call(
                f,
                Conv::Thiscall(vt),
                &[dev, resource, 0, e.wrapping_add(4)],
            );
            if (hr as i32) < 0 {
                this_call(h, SCOPE_LEAVE, lock, &[]);
                return 9;
            }
            i += 1;
            h.write(lock + 8, i.wrapping_mul(0x14));
        }
        this_call(h, SCOPE_LEAVE, lock, &[]);
        0
    }
}
