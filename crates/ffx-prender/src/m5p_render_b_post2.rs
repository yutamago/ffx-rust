//! M5 stage 1, list `phyre-render-b`, post-processing part 2: the
//! effects' setup with the default size, their per-frame entry (slot
//! `+0x1c` of the effect vtables: set up once, refresh the material,
//! render), the render targets' resize checks, and the effects'
//! destructor bodies.
//!
//! An effect's render target is a handle `{texture, kind byte}`: kind 0
//! holds a texture; [`Resize`] says what the check found.

/// The render interface singleton (made once: guard bit 0, constructor,
/// `atexit` destructor) whose default target size the setups use.
pub const INTERFACE_GUARD: u32 = 0x00c9_628c;
pub const INTERFACE: u32 = 0x00c9_4f00;
pub const INTERFACE_CTOR: u32 = 0x004f_ee70;
pub const INTERFACE_ATEXIT: u32 = 0x00af_b710;
pub const ATEXIT: u32 = 0x0094_9207;
/// The interface's default width and height.
pub const DEFAULT_WIDTH: u32 = 0x00c9_5100;
pub const DEFAULT_HEIGHT: u32 = 0x00c9_5104;
/// The post-processing utility's render target.
pub const UTILITY_TARGET: u32 = 0x0194_3da0;

/// The setups with the default size `(address, init)`: `init(effect;
/// target, w, h)` (thiscall, `ret 0xc`), byte `+0x10` set on success.
pub const SETUPS: [(u32, u32); 4] = [
    (0x00a1_98d0, 0x00a1_76d0), // PDepthOfField
    (0x00a1_9ac0, 0x00a1_79a0), // PGlow
    (0x00a1_9b60, 0x00a1_7e00), // PMotionBlur
    (0x00a1_9c10, 0x00a1_7ec0), // PScreenSpaceReflection
];

/// How an effect's per-frame entry renders, once its material is ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Render {
    /// `f(effect; a, [a + 0x5c], b)` (`PDeferredLighting`).
    Lighting(u32),
    /// `f(effect; a, [a + 0x5c], c, d, b)` (`PDepthOfField`).
    DepthOfField(u32),
    /// `f(effect; a, b, c, 1)` (`PFXAA`).
    Fxaa(u32),
    /// 5 without `e`, else `f(effect; a, b, c, d, e)`
    /// (`PScreenSpaceReflection`).
    Reflection(u32),
    /// `pre(effect; a, c)`, then `f(effect; a, b, c, the bloom texture,
    /// 1)` (`PGlow`).
    Glow { pre: u32, f: u32 },
}

/// The per-frame entries `(address, how)`, `(effect; a, b, c, d, e)`
/// with `ret 0x14`.
pub const ENTRIES: [(u32, Render); 5] = [
    (0x00a2_1a20, Render::Lighting(0x00a1_f220)),
    (0x00a2_1aa0, Render::DepthOfField(0x00a1_ca10)),
    (0x00a2_1b20, Render::Fxaa(0x00a2_0360)),
    (0x00a2_1cf0, Render::Reflection(0x00a2_20b0)),
    (
        0x00a2_1ba0,
        Render::Glow {
            pre: 0x00a1_39c0,
            f: 0x00a1_d2b0,
        },
    ),
];

/// The [`SETUPS`] init of `f`.
#[must_use]
pub const fn setup_of(f: u32) -> u32 {
    let mut i = 0;
    while i < SETUPS.len() {
        if SETUPS[i].0 == f {
            return SETUPS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The [`ENTRIES`] renderer of `f`.
#[must_use]
pub const fn entry_of(f: u32) -> Render {
    let mut i = 0;
    while i < ENTRIES.len() {
        if ENTRIES[i].0 == f {
            return ENTRIES[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// `Phyre_PRendering_PTexture2DD3D11_isBound` (thiscall -> `al`),
/// `Phyre_PRendering_PTexture2DD3D11_unbind` (thiscall) and
/// `Phyre_PRendering_PTexture2DBase_setDimensions(texture; w, h, word 0,
/// 0)` (thiscall).
pub const IS_BOUND: u32 = 0x0059_5f90;
pub const UNBIND: u32 = 0x0059_cc40;
pub const SET_DIMENSIONS: u32 = 0x004c_40c0;
/// `PSamplerStateD3D11_release_0059c770`, `PSpriteArrays_release_00495a10`
/// and `Phyre_PRendering_PSamplerStateD3D11_dtor` (thiscall).
pub const SAMPLER_RELEASE: u32 = 0x0059_c770;
pub const SPRITE_ARRAYS_RELEASE: u32 = 0x0049_5a10;
pub const SAMPLER_DTOR: u32 = 0x0058_5230;
/// `PDeferredLightingBase`'s resources release (`0x00a24a60`).
pub const LIGHTING_RELEASE: u32 = 0x00a2_4a60;
/// The vtables the destructor bodies store: `PDeferredLightingBase`,
/// `PDepthOfField`, `PPostEffectBase`.
pub const LIGHTING_BASE_VTABLE: u32 = 0x00b7_4160;
pub const DEPTH_OF_FIELD_VTABLE: u32 = 0x00b7_4590;
pub const POST_EFFECT_BASE_VTABLE: u32 = 0x00b7_4058;
/// `Phyre_PPostProcessing_renderTargetRelease_00a0e520(target)` (cdecl)
/// and `Phyre_PPostProcessing_PDepthOfFieldBase_dtor_009fdd30`
/// (thiscall).
pub const RENDER_TARGET_RELEASE: u32 = 0x00a0_e520;
pub const DEPTH_OF_FIELD_BASE_DTOR: u32 = 0x009f_dd30;
/// `0x00a23080`: `PDepthOfField`'s three half-size targets (thiscall).
pub const HALF_TARGETS: u32 = 0x00a2_3080;
/// `0x009fe3a0`: `PMeshParticleSystemBase`'s member release (thiscall).
pub const PARTICLE_MEMBERS_RELEASE: u32 = 0x009f_e3a0;
/// `Phyre_PhyreFree` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;

/// What a render target's size check found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resize {
    /// No texture behind the handle.
    Missing,
    /// The size matches, or the texture took the new size.
    Done,
    /// The unbind or the resize failed with this code.
    Failed(u32),
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_graphic::host::{Conv, Host};

    fn this_call(h: &mut impl Host, f: u32, this: u32, args: &[u32]) -> u32 {
        h.call_sym(f, Conv::Thiscall(this), args)
    }

    /// The render interface made once (its guard's bit 0).
    fn interface_once(h: &mut impl Host) {
        let guard = h.rebase(INTERFACE_GUARD);
        let flags: u32 = h.read(guard);
        if flags & 1 == 0 {
            h.write(guard, flags | 1);
            let obj = h.rebase(INTERFACE);
            this_call(h, INTERFACE_CTOR, obj, &[]);
            let f = h.rebase(INTERFACE_ATEXIT);
            h.call_sym(ATEXIT, Conv::Cdecl, &[f]);
        }
    }

    /// [`super::SETUPS`] `(effect; w, h)`: the size, the interface's
    /// default for a 0, handed to `init` with the utility's target;
    /// byte `+0x10` set when it returns 0; its `eax`.
    pub fn setup(h: &mut impl Host, this: u32, init: u32, w: u32, hgt: u32) -> u32 {
        interface_once(h);
        let w = if w == 0 {
            let at = h.rebase(DEFAULT_WIDTH);
            h.read(at)
        } else {
            w
        };
        let hgt = if hgt == 0 {
            let at = h.rebase(DEFAULT_HEIGHT);
            h.read(at)
        } else {
            hgt
        };
        let target = h.rebase(UTILITY_TARGET);
        let r = this_call(h, init, this, &[target, w, hgt]);
        if r == 0 {
            h.write(this + 0x10, 1u8);
        }
        r
    }

    /// [`super::ENTRIES`] `(effect; a, b, c, d, e)`: before setup, slot
    /// `+0x04` with the size record `c`'s `+0x1c`/`+0x20` (its error
    /// returned); 5 without `c`; the material refreshed (slot `+0x08`)
    /// when it changed; 0x15 without one; else the effect renders.
    pub fn entry(h: &mut impl Host, this: u32, how: Render, args: [u32; 5]) -> u32 {
        let [a, b, c, d, e] = args;
        if h.read::<u8>(this + 0x10) == 0 {
            let (w, hgt) = if c == 0 {
                (0, 0)
            } else {
                (h.read::<u32>(c + 0x1c), h.read::<u32>(c + 0x20))
            };
            let r = h.call_virtual(this, 1, &[w, hgt]);
            if r != 0 {
                return r;
            }
        }
        if c == 0 {
            return 5;
        }
        if h.read::<u32>(this + 8) != h.read::<u32>(this + 0xc) {
            h.call_virtual(this, 2, &[]);
        }
        let material: u32 = h.read(this + 8);
        if material == 0 || h.read::<u32>(material) == 0 {
            return 0x15;
        }
        match how {
            Render::Lighting(f) => {
                let w: u32 = h.read(a + 0x5c);
                this_call(h, f, this, &[a, w, b])
            }
            Render::DepthOfField(f) => {
                let w: u32 = h.read(a + 0x5c);
                this_call(h, f, this, &[a, w, c, d, b])
            }
            Render::Fxaa(f) => this_call(h, f, this, &[a, b, c, 1]),
            Render::Reflection(f) => {
                if e == 0 {
                    return 5;
                }
                this_call(h, f, this, &[a, b, c, d, e])
            }
            Render::Glow { pre, f } => {
                let r = this_call(h, pre, this, &[a, c]);
                if r != 0 {
                    return r;
                }
                let bloom: u32 = h.read(this + 0xa8);
                let tex = if h.read::<u8>(bloom + 4) == 0 {
                    h.read(bloom)
                } else {
                    0
                };
                this_call(h, f, this, &[a, b, c, tex, 1])
            }
        }
    }

    /// The size check of the target behind `handle` (`{texture, kind}`):
    /// [`Resize::Missing`] without a texture; else, when its `+0x1c`/
    /// `+0x20` differ from `w`/`hgt`, a bound texture unbound, then
    /// `setDimensions(w, hgt, word 0, 0)`; an error from either fails.
    pub fn resize_target(h: &mut impl Host, handle: u32, w: u32, hgt: u32) -> Resize {
        if handle == 0 || h.read::<u8>(handle + 4) != 0 {
            return Resize::Missing;
        }
        let tex: u32 = h.read(handle);
        if tex == 0 {
            return Resize::Missing;
        }
        if h.read::<u32>(tex + 0x1c) == w && h.read::<u32>(tex + 0x20) == hgt {
            return Resize::Done;
        }
        if this_call(h, IS_BOUND, tex, &[]) & 0xff != 0 {
            let r = this_call(h, UNBIND, tex, &[]);
            if r != 0 {
                return Resize::Failed(r);
            }
        }
        let word0: u32 = h.read(tex);
        let r = this_call(h, SET_DIMENSIONS, tex, &[w, hgt, word0, 0]);
        if r != 0 {
            return Resize::Failed(r);
        }
        Resize::Done
    }

    fn code(r: Resize) -> u32 {
        match r {
            Resize::Missing => 1,
            Resize::Done => 0,
            Resize::Failed(e) => e,
        }
    }

    /// `0x00a23200(effect; w, h)`: the target at `+0x5c` resized; 1
    /// without one, the error, or 0.
    pub fn resize_5c(h: &mut impl Host, this: u32, w: u32, hgt: u32) -> u32 {
        let handle: u32 = h.read(this + 0x5c);
        code(resize_target(h, handle, w, hgt))
    }

    /// `0x00a1b8f0(effect; w, h)`: [`resize_5c`] once set up (byte
    /// `+0x10`), else 0.
    pub fn resize_5c_if_set_up(h: &mut impl Host, this: u32, w: u32, hgt: u32) -> u32 {
        if h.read::<u8>(this + 0x10) == 0 {
            return 0;
        }
        resize_5c(h, this, w, hgt)
    }

    /// `0x00a23080(effect; w, h)`: the three targets at `+0xf2c` resized
    /// to half the size; 1 at the first missing, the first error, or 0.
    pub fn resize_halves(h: &mut impl Host, this: u32, w: u32, hgt: u32) -> u32 {
        let (w, hgt) = (w >> 1, hgt >> 1);
        for k in 0..3 {
            let handle: u32 = h.read(this + 0xf2c + 4 * k);
            match resize_target(h, handle, w, hgt) {
                Resize::Done => {}
                r => return code(r),
            }
        }
        0
    }

    /// `0x00a1b870(effect; w, h)`: once set up, the target at `+0xfb0`
    /// resized, then [`HALF_TARGETS`]; 1 without the target, an error, or
    /// 0.
    pub fn resize_depth_of_field(h: &mut impl Host, this: u32, w: u32, hgt: u32) -> u32 {
        if h.read::<u8>(this + 0x10) == 0 {
            return 0;
        }
        let handle: u32 = h.read(this + 0xfb0);
        match resize_target(h, handle, w, hgt) {
            Resize::Done => this_call(h, HALF_TARGETS, this, &[w, hgt]),
            r => code(r),
        }
    }

    /// `0x00a24a60(effect)`: `PDeferredLightingBase`'s samplers at
    /// `+0x3ee8`, `+0x3f30`, `+0x3f0c` (that order) released when bit 7
    /// of their `+0x1c` word is set, and the sprite arrays of the meshes
    /// at `+0x3edc`, `+0x3ee0`, `+0x3ee4` released and cleared.
    pub fn lighting_release(h: &mut impl Host, this: u32) {
        for at in [0x3ee8, 0x3f30, 0x3f0c] {
            if (h.read::<u32>(this + at + 0x1c) >> 7) & 1 != 0 {
                this_call(h, SAMPLER_RELEASE, this + at, &[]);
            }
        }
        for at in [0x3edc, 0x3ee0, 0x3ee4] {
            let p: u32 = h.read(this + at);
            if p != 0 {
                let arrays: u32 = h.read(p);
                this_call(h, SPRITE_ARRAYS_RELEASE, arrays, &[]);
                h.write(this + at, 0u32);
            }
        }
    }

    /// `0x009fdb80(effect)`: `PDeferredLightingBase`'s destructor body:
    /// its vtable, [`LIGHTING_RELEASE`], the three samplers destroyed
    /// (`+0x3f30`, `+0x3f0c`, `+0x3ee8`), the base vtable; the last
    /// destructor's `eax`.
    pub fn lighting_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(LIGHTING_BASE_VTABLE);
        h.write(this, vt);
        this_call(h, LIGHTING_RELEASE, this, &[]);
        let mut eax = 0;
        for at in [0x3f30, 0x3f0c, 0x3ee8] {
            eax = this_call(h, SAMPLER_DTOR, this + at, &[]);
        }
        let vt = h.rebase(POST_EFFECT_BASE_VTABLE);
        h.write(this, vt);
        eax
    }

    /// `0x009fde10(effect)`: `PDepthOfField`'s destructor body: its
    /// vtable, its target (`+0xfb0`) released, then
    /// [`DEPTH_OF_FIELD_BASE_DTOR`]; its `eax`.
    pub fn depth_of_field_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(DEPTH_OF_FIELD_VTABLE);
        h.write(this, vt);
        let target: u32 = h.read(this + 0xfb0);
        h.call_sym(RENDER_TARGET_RELEASE, Conv::Cdecl, &[target]);
        this_call(h, DEPTH_OF_FIELD_BASE_DTOR, this, &[])
    }

    /// `0x00a02d90(effect; flags)`: `PMeshParticleSystemBase`'s deleting
    /// destructor: the sampler at `+0x3084` destroyed, the members
    /// (`+0x14`) released, the base vtable, `PhyreFree` when `flags & 1`;
    /// `this`.
    pub fn particles_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        this_call(h, SAMPLER_DTOR, this + 0x3084, &[]);
        this_call(h, PARTICLE_MEMBERS_RELEASE, this + 0x14, &[]);
        let vt = h.rebase(POST_EFFECT_BASE_VTABLE);
        h.write(this, vt);
        if flags & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }
}
