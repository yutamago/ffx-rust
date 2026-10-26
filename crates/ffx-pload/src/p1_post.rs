//! List `p1-load`, continuation round 3, batch 2: the constructors of
//! the post-processing objects a cluster instantiates
//! (`PDeferredLightingBase`, `PDeferredLightingD3D11`,
//! `PDepthOfFieldBase`; their class descriptors' `constructInstance`
//! call them).
//!
//! Each starts like a `PRenderInterfaceObject` (`+4..+0xc` zero, the
//! 16-bit `+0x10` 0x100) and builds its sampler states
//! (`PSamplerStateD3D11_ctor`, thiscall) before giving them their
//! defaults: the filters and address modes (bytes `+0..+4`) unless the
//! state's flags (`+0x1c`) have bit 7 (each byte checks it again). The
//! float defaults are the image's constants, loaded and stored through
//! the FPU (`fld`/`fst`); the originals' SEH frames only unwind the
//! half-built members, and nothing here throws.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `PSamplerStateD3D11_ctor` (thiscall).
const SAMPLER_CTOR: u32 = 0x0058_3b20;
/// The deferred lighting's light constructor (thiscall, 0x7c bytes).
const LIGHT_CTOR: u32 = 0x009f_c6f0;
/// The base constructor `PDeferredLightingD3D11` starts with.
pub const DEFERRED_BASE_CTOR: u32 = 0x009f_b830;

/// The vtables.
const DEFERRED_BASE_VTABLE: u32 = 0x00b7_4160;
const DEFERRED_D3D11_VTABLE: u32 = 0x00b7_4270;
const DOF_BASE_VTABLE: u32 = 0x00b7_4488;

/// `1.0` as an f32.
const ONE: u32 = 0x3f80_0000;

/// The f32 at the image's `sym`, as `fld dword` / `fst dword` store it.
fn constant(h: &impl Host, sym: u32) -> u32 {
    x87::store(x87::load(h.read(h.rebase(sym))))
}

/// A sampler state's byte defaults `bytes` (`+0..+4`), each unless its
/// flags (`+0x1c`) have bit 7.
fn sampler_defaults(h: &mut impl Host, s: u32, bytes: [u8; 5]) {
    for (k, b) in (0u32..).zip(bytes) {
        if (h.read::<u32>(s.wrapping_add(0x1c)) >> 7) & 1 == 0 {
            h.write(s.wrapping_add(k), b);
        }
    }
}

/// The render-interface object's start: `+4..+0xc` zero, `+0x10` 0x100.
fn object_start(h: &mut impl Host, this: u32) {
    for off in [4, 8, 0xc] {
        h.write(this.wrapping_add(off), 0u32);
    }
    h.write(this.wrapping_add(0x10), 0x100u16);
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        Conv, DEFERRED_BASE_CTOR, DEFERRED_BASE_VTABLE, DEFERRED_D3D11_VTABLE, DOF_BASE_VTABLE,
        Host, LIGHT_CTOR, ONE, SAMPLER_CTOR, constant, object_start, sampler_defaults,
    };

    /// `PDeferredLightingBase::PDeferredLightingBase` (`0x009fb830`,
    /// thiscall) -> `this`: the 128 lights from `+0x14` (0x7c bytes each)
    /// and the one at `+0x3e14` constructed; the light defaults
    /// `+0x3e90..+0x3ecc` (0.2 three times, 1, 0 three times, 1, 100, 20,
    /// 0, 0.5, 0.05, 1.3, 1, 1: the image's constants), `+0x3ed0..+0x3ee4`
    /// zero; the samplers `+0x3ee8`, `+0x3f0c`, `+0x3f30`; `+0x3f54..+0x3f8c`
    /// zero, the byte `+0x3f90` 1; the samplers' defaults (point, clamp:
    /// `+0x3ee8` 0 0 2 2 2, `+0x3f30` 1 1 2 2 2, `+0x3f0c` 1 1 2 2, then
    /// its flags' bit 6 set, all unless bit 7).
    pub fn deferred_lighting_base_ctor(h: &mut impl Host, this: u32) -> u32 {
        object_start(h, this);
        let vt = h.rebase(DEFERRED_BASE_VTABLE);
        h.write(this, vt);
        for i in 0..128 {
            h.call_sym(LIGHT_CTOR, Conv::Thiscall(this + 0x14 + 0x7c * i), &[]);
        }
        h.call_sym(LIGHT_CTOR, Conv::Thiscall(this + 0x3e14), &[]);
        let fifth = constant(h, 0x00b4_12c4);
        let hundred = constant(h, 0x00b3_04f8);
        let twenty = constant(h, 0x00b4_ae10);
        let half = constant(h, 0x00b9_2194);
        let twentieth = constant(h, 0x00b4_7c5c);
        let more = constant(h, 0x00b4_3d30);
        let values = [
            fifth, fifth, fifth, ONE, 0, 0, 0, ONE, hundred, twenty, 0, half, twentieth, more, ONE,
            ONE,
        ];
        for (k, v) in (0u32..).zip(values) {
            h.write(this + 0x3e90 + 4 * k, v);
        }
        for k in 0..6 {
            h.write(this + 0x3ed0 + 4 * k, 0u32);
        }
        let (a, b, c) = (this + 0x3ee8, this + 0x3f0c, this + 0x3f30);
        for s in [a, b, c] {
            h.call_sym(SAMPLER_CTOR, Conv::Thiscall(s), &[]);
        }
        for k in 0..15 {
            h.write(this + 0x3f54 + 4 * k, 0u32);
        }
        h.write(this + 0x3f90, 1u8);
        sampler_defaults(h, a, [0, 0, 2, 2, 2]);
        sampler_defaults(h, c, [1, 1, 2, 2, 2]);
        for (k, v) in (0u32..).zip([1u8, 1, 2, 2]) {
            if (h.read::<u32>(b + 0x1c) >> 7) & 1 == 0 {
                h.write(b + k, v);
            }
        }
        let flags: u32 = h.read(b + 0x1c);
        if (flags >> 7) & 1 == 0 {
            h.write(b + 0x1c, flags | 0x40);
        }
        this
    }

    /// `PDeferredLightingD3D11::PDeferredLightingD3D11` (`0x009fbb30`,
    /// thiscall) -> `this`: the base constructor (`0x009fb830`), its
    /// vtable, `+0x3f94..+0x4028` zero, the byte `+0x402c` 1,
    /// `+0x4030..+0x403c` zero.
    pub fn deferred_lighting_d3d11_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(DEFERRED_BASE_CTOR, Conv::Thiscall(this), &[]);
        let vt = h.rebase(DEFERRED_D3D11_VTABLE);
        h.write(this, vt);
        for k in 0..38 {
            h.write(this + 0x3f94 + 4 * k, 0u32);
        }
        h.write(this + 0x402c, 1u8);
        for k in 0..4 {
            h.write(this + 0x4030 + 4 * k, 0u32);
        }
        this
    }

    /// `PDepthOfFieldBase::PDepthOfFieldBase` (`0x009fbd90`, thiscall) ->
    /// `this`: `+0x14`, `+0x18`, `+0x1c` the image's 5, 2 and 0.5, its
    /// vtable, `+0xf38`/`+0xf3c` zero; the samplers `+0xf40` and `+0xf64`;
    /// `+0xf88..+0xfac`, `+0x20..+0x70`, `+0xf2c..+0xf34` and the two runs
    /// of 49 words from `+0x384` and `+0x448` zero; the samplers'
    /// defaults (`+0xf40` 0 0 2 2 2, `+0xf64` 1 1 2 2 2, unless bit 7).
    pub fn depth_of_field_base_ctor(h: &mut impl Host, this: u32) -> u32 {
        object_start(h, this);
        let v = constant(h, 0x00b3_eb98);
        h.write(this + 0x14, v);
        let v = constant(h, 0x00b9_2268);
        h.write(this + 0x18, v);
        let v = constant(h, 0x00b9_2194);
        let vt = h.rebase(DOF_BASE_VTABLE);
        h.write(this, vt);
        h.write(this + 0x1c, v);
        h.write(this + 0xf38, 0u32);
        h.write(this + 0xf3c, 0u32);
        let (a, b) = (this + 0xf40, this + 0xf64);
        h.call_sym(SAMPLER_CTOR, Conv::Thiscall(a), &[]);
        h.call_sym(SAMPLER_CTOR, Conv::Thiscall(b), &[]);
        for k in 0..10 {
            h.write(this + 0xf88 + 4 * k, 0u32);
        }
        // `fldz` stores: +0.0
        for k in 0..21 {
            h.write(this + 0x20 + 4 * k, 0u32);
        }
        for off in [0xf2c, 0xf30, 0xf34] {
            h.write(this + off, 0u32);
        }
        for k in 0..49 {
            h.write(this + 0x384 + 4 * k, 0u32);
            h.write(this + 0x448 + 4 * k, 0u32);
        }
        sampler_defaults(h, a, [0, 0, 2, 2, 2]);
        sampler_defaults(h, b, [1, 1, 2, 2, 2]);
        this
    }
}
