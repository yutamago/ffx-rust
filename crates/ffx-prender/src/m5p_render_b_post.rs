//! M5 stage 1, list `phyre-render-b`, the post-processing part: the
//! small virtual methods of `PPostEffectBase` and its effects
//! (`PDeferredLighting`, `PGlow`, `PScreenSpaceReflection`, ...), the
//! post-processing utility's teardown, and the input-action accessors the
//! list's post-processing family holds.
//!
//! `PPostEffectBase` (offsets the methods use): `+0x08` the effect
//! material (a handle whose word 0 is the material), `+0x0c` the material
//! in use, byte `+0x10` set once the effect has run its setup (slot
//! `+0x04`), byte `+0x11` enabled. Slot numbers are the vtable offsets;
//! the slots' names are ours (Fahrenheit's symbols name none of them).

/// `Phyre_PPostProcessing_PPostEffectBase_vftable`.
pub const POST_EFFECT_BASE_VTABLE: u32 = 0x00b7_4058;
/// The pass types slot `+0x14` compares with (addresses of the
/// `SceneRenderPassType` globals): the base's and
/// `PScreenSpaceReflection`'s.
pub const PASS_BASE: u32 = 0x00c1_69d4;
pub const PASS_SSR: u32 = 0x00c1_6a84;
/// The handle getter `0x00a161b0` (`thiscall`, the effect: its material
/// handle's word 0, or 0).
pub const MATERIAL_OF: u32 = 0x00a1_61b0;
/// The post-processing utility's render target (`0x01943da0`) and its
/// release `RenderTarget_release_00440ea0` (`thiscall`).
pub const UTILITY_TARGET: u32 = 0x0194_3da0;
pub const RENDER_TARGET_RELEASE: u32 = 0x0044_0ea0;
/// `PSamplerStateD3D11_release_0059c770` (`thiscall`).
pub const SAMPLER_RELEASE: u32 = 0x0059_c770;

/// The effects whose slot `+0x08` (re)initialises their shaders:
/// `(address, initializeShaders)`, `thiscall(effect; target, material)`
/// with `ret 8`.
pub const INIT_SHADERS: [(u32, u32); 3] = [
    (0x00a1_b6e0, 0x00a1_a4f0), // PDeferredLighting
    (0x00a1_b7b0, 0x00a1_a830), // PGlow
    (0x00a1_b820, 0x00a1_ae00), // PScreenSpaceReflection
];

/// The effects whose slot `+0x0c` renders once set up: `(address,
/// render)`, `thiscall(effect; a, b)` with `ret 8`.
pub const RENDER_IF_SET_UP: [(u32, u32); 3] = [
    (0x00a1_b850, 0x00a2_2be0), // PDeferredLighting
    (0x00a1_b960, 0x00a2_3280), // PGlow
    (0x00a1_b9b0, 0x00a2_3780), // PScreenSpaceReflection
];

/// The [`INIT_SHADERS`] callee of `f`.
#[must_use]
pub const fn init_shaders_of(f: u32) -> u32 {
    let mut i = 0;
    while i < INIT_SHADERS.len() {
        if INIT_SHADERS[i].0 == f {
            return INIT_SHADERS[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The [`RENDER_IF_SET_UP`] callee of `f`.
#[must_use]
pub const fn render_of(f: u32) -> u32 {
    let mut i = 0;
    while i < RENDER_IF_SET_UP.len() {
        if RENDER_IF_SET_UP[i].0 == f {
            return RENDER_IF_SET_UP[i].1;
        }
        i += 1;
    }
    panic!("not in the table");
}

/// `al` of the pass test: all ones when `pass` is `want`, else 0, and
/// the flag byte (`setne al; dec al; and al, flag`).
#[must_use]
pub const fn pass_mask(pass: u32, want: u32, flag: u32) -> u8 {
    let m: u8 = if pass == want { 0xff } else { 0 };
    m & (flag as u8)
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        MATERIAL_OF, PASS_BASE, PASS_SSR, POST_EFFECT_BASE_VTABLE, RENDER_TARGET_RELEASE,
        SAMPLER_RELEASE, UTILITY_TARGET, pass_mask,
    };
    use ffx_graphic::host::{Conv, Host};

    /// `PPostEffectBase`'s destructor body (`0x009fe780`): the base
    /// vtable only.
    pub fn base_dtor(h: &mut impl Host, this: u32) {
        let vt = h.rebase(POST_EFFECT_BASE_VTABLE);
        h.write(this, vt);
    }

    /// Slot `+0x04` (`0x00a19c00`, `(a, b)`): byte `+0x10` = 1; 0.
    pub fn mark_set_up(h: &mut impl Host, this: u32) -> u32 {
        h.write(this + 0x10, 1u8);
        0
    }

    /// Slot `+0x08` of the base (`0x00a1b810`): the material in use is
    /// the effect material; 0.
    pub fn use_material(h: &mut impl Host, this: u32) -> u32 {
        let m: u32 = h.read(this + 8);
        h.write(this + 0xc, m);
        0
    }

    /// Slot `+0x08` of an effect with shaders ([`super::INIT_SHADERS`]):
    /// with a material handle that holds one, `init(effect; utility
    /// target, material)`, whose non-zero result it returns; else (or on
    /// 0) the material in use is the effect material, 0.
    pub fn init_shaders(h: &mut impl Host, this: u32, init: u32) -> u32 {
        let handle: u32 = h.read(this + 8);
        if handle != 0 && h.read::<u32>(handle) != 0 {
            let material = h.call_sym(MATERIAL_OF, Conv::Thiscall(this), &[]);
            let target = h.rebase(UTILITY_TARGET);
            let r = h.call_sym(init, Conv::Thiscall(this), &[target, material]);
            if r != 0 {
                return r;
            }
        }
        use_material(h, this)
    }

    /// Slot `+0x0c` of an effect ([`super::RENDER_IF_SET_UP`], `(a, b)`):
    /// `render(effect; a, b)` once set up (byte `+0x10`), else 0.
    pub fn render_if_set_up(h: &mut impl Host, this: u32, render: u32, a: u32, b: u32) -> u32 {
        if h.read::<u8>(this + 0x10) == 0 {
            return 0;
        }
        h.call_sym(render, Conv::Thiscall(this), &[a, b])
    }

    /// Slot `+0x10` (`0x00a1b200`): whether the effect can run (a material
    /// handle holding one, and byte `+0x11`); `eax` as the original
    /// leaves it (the handle with `al` = the answer).
    pub fn can_run(h: &mut impl Host, this: u32) -> u32 {
        let handle: u32 = h.read(this + 8);
        let yes = handle != 0 && h.read::<u32>(handle) != 0 && h.read::<u8>(this + 0x11) != 0;
        handle & !0xff | u32::from(yes)
    }

    /// Slot `+0x14` (`0x00a1b190`, `(pass, flag)`): the effect's slot
    /// `+0x10` (`eax` as it returns when `al` is 0), then `al` = the flag
    /// byte when `pass` is the base pass type, else 0.
    pub fn runs_in_pass(h: &mut impl Host, this: u32, pass: u32, flag: u32) -> u32 {
        let r = h.call_virtual(this, 4, &[]);
        if r as u8 == 0 {
            return r;
        }
        let want = h.rebase(PASS_BASE);
        r & !0xff | u32::from(pass_mask(pass, want, flag))
    }

    /// `PScreenSpaceReflection`'s slot `+0x14` (`0x00a1b1c0`): `al` = the
    /// flag byte when `pass` is its pass type, else 0.
    pub fn ssr_runs_in_pass(h: &mut impl Host, pass: u32, flag: u32) -> u8 {
        let want = h.rebase(PASS_SSR);
        pass_mask(pass, want, flag)
    }

    /// Slot `+0x18` (`0x00a21cc0`, five words, the third a size record):
    /// before setup (byte `+0x10` clear), the effect's slot `+0x04` with
    /// the record's `+0x1c` and `+0x20` (0, 0 without one), whose non-zero
    /// result it returns; else 0.
    pub fn set_up_once(h: &mut impl Host, this: u32, record: u32) -> u32 {
        if h.read::<u8>(this + 0x10) != 0 {
            return 0;
        }
        let (w, hgt) = if record == 0 {
            (0, 0)
        } else {
            (h.read::<u32>(record + 0x1c), h.read::<u32>(record + 0x20))
        };
        h.call_virtual(this, 1, &[w, hgt])
    }

    /// `PUtilityPostProcessing::terminate` (`0x00a24e70`): the utility's
    /// render target released; 0.
    pub fn utility_terminate(h: &mut impl Host) -> u32 {
        let target = h.rebase(UTILITY_TARGET);
        h.call_sym(RENDER_TARGET_RELEASE, Conv::Thiscall(target), &[]);
        0
    }

    /// `0x00a24d60`: the two sampler states at `+0x00` and `+0x24`
    /// released when bit 7 of their `+0x1c` word is set (the second a
    /// tail call); `eax` as the original leaves it.
    pub fn release_samplers(h: &mut impl Host, this: u32) -> u32 {
        let mut eax: u32 = h.read::<u32>(this + 0x1c) >> 7;
        if eax & 1 != 0 {
            h.call_sym(SAMPLER_RELEASE, Conv::Thiscall(this), &[]);
        }
        let flags: u32 = h.read(this + 0x40);
        eax = flags >> 7;
        if eax & 1 != 0 {
            return h.call_sym(SAMPLER_RELEASE, Conv::Thiscall(this + 0x24), &[]);
        }
        eax
    }

    /// `0x009ecc70`: word `+4` with its bit 0 cleared (`sub` of the low
    /// bit).
    pub fn input_word_even(h: &mut impl Host, this: u32) -> u32 {
        let w: u32 = h.read(this + 4);
        w.wrapping_sub(w & 1)
    }

    /// `0x009ecc80`: element `i` of the `PArray` at `this` (count masked
    /// to 31 bits, buffer at `+4`), 0 past its end (unsigned).
    pub fn array_at(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let count = h.read::<u32>(this) & 0x7fff_ffff;
        if i >= count {
            return 0;
        }
        let data: u32 = h.read(this + 4);
        h.read(data.wrapping_add(i.wrapping_mul(4)))
    }

    /// `0x009eca80`: entry `i` of the table at `[this + 0xc]`, plus 4 when
    /// not null.
    pub fn entry_plus4(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let table: u32 = h.read(this + 0xc);
        let p: u32 = h.read(table.wrapping_add(i.wrapping_mul(4)));
        if p == 0 { 0 } else { p.wrapping_add(4) }
    }

    /// `0x009ecaa0`: word `+4` of entry `i` of the table at
    /// `[this + 0xc]`.
    pub fn entry_word4(h: &mut impl Host, this: u32, i: u32) -> u32 {
        let table: u32 = h.read(this + 0xc);
        let p: u32 = h.read(table.wrapping_add(i.wrapping_mul(4)));
        h.read(p.wrapping_add(4))
    }
}
