//! The post effects' and the movie player's singletons and their small
//! methods: `GetInstance` (print once when missing), `resize` (the render
//! target's), the shader-cluster callbacks, the broken-screen opacity.

use ffx_bridge::{Global, Mem, x87};
use ffx_graphic::host::{Conv, Host};

/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
/// The render-target resize helper `0x00a0e4b0` (cdecl target, w, h, 1).
pub const TARGET_RESIZE: u32 = 0x00a0_e4b0;
/// `AsyncLoadManager_sGetInstance()`.
pub const ASYNC_LOAD_MANAGER: u32 = 0x0069_1b40;
/// `ClusterManager_GetInstance()`.
pub const CLUSTER_MANAGER: u32 = 0x0069_a1a0;
/// `ChrResourceCache_DelayReleaseStep_0069c010` (thiscall; f32 dt).
pub const DELAY_RELEASE_STEP: u32 = 0x0069_c010;
/// `ShadowEntries_Init_006d54c0` (thiscall; n, buf).
pub const BROKEN_SCREEN_INIT: u32 = 0x006d_54c0;
/// `BlurEffect_doOnShaderClusterLoaded` (thiscall).
pub const BLUR_ON_LOADED: u32 = 0x006c_35d0;
/// `PhyFMVPlayerManager_doOnShaderBinFilesLoaded` (thiscall).
pub const FMV_ON_LOADED: u32 = 0x006d_86e0;
/// `BlurEffect +0x20`: the loaded shader cluster.
pub const BLUR_CLUSTER: u32 = 0x20;
/// `PhyFMVPlayerManager +0x360`: the loaded shader cluster.
pub const FMV_CLUSTER: u32 = 0x360;
/// `BrokenScreenEffect +8`: the opacity (f32).
pub const BROKEN_SCREEN_OPACITY: u32 = 0x8;

/// A singleton: its global and the message `GetInstance` prints when it
/// is missing (an address in `.rdata`).
#[derive(Debug, Clone, Copy)]
pub struct Singleton {
    /// The instance pointer (its `ffx.sym` address).
    pub global: u32,
    /// The `Phyre_PhyrePrintf` format.
    pub message: u32,
    /// The `GetInstance` function (for tests).
    pub get: u32,
}

/// `Distortion_GetInstance` (0x006c2290).
pub const DISTORTION: Singleton = Singleton {
    global: 0x00cd_eb88,
    message: 0x00b4_92d8,
    get: 0x006c_2290,
};
/// `BlurEffect_GetInstance` (0x006c3580).
pub const BLUR_EFFECT: Singleton = Singleton {
    global: 0x00cd_eba0,
    message: 0x00b4_9414,
    get: 0x006c_3580,
};
/// `BlurEffectEx_GetInstance` (0x006c4a70).
pub const BLUR_EFFECT_EX: Singleton = Singleton {
    global: 0x00cd_ebfc,
    message: 0x00b4_9528,
    get: 0x006c_4a70,
};
/// `MotionBlur_GetInstance` (0x006c5130).
pub const MOTION_BLUR: Singleton = Singleton {
    global: 0x00cd_ec04,
    message: 0x00b4_9560,
    get: 0x006c_5130,
};
/// `BrokenScreenEffect_GetInstance` (0x006d4de0).
pub const BROKEN_SCREEN: Singleton = Singleton {
    global: 0x00cd_ec98,
    message: 0x00b4_9aac,
    get: 0x006d_4de0,
};
/// `PhyFMVPlayerManager_GetInstance` (0x006d73a0).
pub const FMV_PLAYER_MANAGER: Singleton = Singleton {
    global: 0x00cd_ed3c,
    message: 0x00b4_9b98,
    get: 0x006d_73a0,
};

/// `*_GetInstance()`: the instance; when it is null, prints the class's
/// message (`Phyre_PhyrePrintf(1, message)`) and returns the global read
/// again.
pub fn get_instance(h: &mut impl Host, s: Singleton) -> u32 {
    let global = Global::<u32>::new(s.global);
    let g = global.get(h);
    if g != 0 {
        return g;
    }
    let message = h.rebase(s.message);
    h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, message]);
    global.get(h)
}

/// The `resize(this; w, h)` methods (thiscall, `ret 8`) that hand one
/// render target to `0x00a0e4b0(target, w, h, 1)` and return 0:
/// `BrokenScreenEffect_resize` (target at `+0`), `Distortion_resize`
/// (`+0x28`), `RadialBlur_resize_007035f0` (`+0x68`).
pub fn resize(h: &mut impl Host, this: u32, target_at: u32, w: u32, hh: u32) -> u32 {
    let target = h.read::<u32>(this + target_at);
    h.call_sym(TARGET_RESIZE, Conv::Cdecl, &[target, w, hh, 1]);
    0
}

/// `BrokenScreenEffect_setOpacity(this; opacity)` (0x006d59e0, thiscall):
/// the float stored through `fld`/`fstp`. Callers read no result.
pub fn broken_screen_set_opacity(h: &mut impl Mem, this: u32, opacity: u32) {
    let v = x87::round_trip(f32::from_bits(opacity));
    h.write(this + BROKEN_SCREEN_OPACITY, v.to_bits());
}

/// `graphicBrokenScreenInit_00639450(n, buf)` (cdecl):
/// `ShadowEntries_Init_006d54c0(BrokenScreenEffect_GetInstance(); n,
/// buf)`'s `eax` (the arguments pushed before the getter, which takes none).
pub fn graphic_broken_screen_init(h: &mut impl Host, n: u32, buf: u32) -> u32 {
    let inst = h.call_sym(BROKEN_SCREEN.get, Conv::Cdecl, &[]);
    h.call_sym(BROKEN_SCREEN_INIT, Conv::Thiscall(inst), &[n, buf])
}

/// `graphicChrDelayReleaseStep_006442d0(dt)` (cdecl, named
/// `TextLayerInfos_dtor_TextLayerInfos` in the symbols):
/// `ChrResourceCache_DelayReleaseStep_0069c010(ClusterManager_GetInstance();
/// dt)`'s `eax` (`dt` copied through `fld`/`fstp` and pushed before the
/// getter).
pub fn chr_delay_release_step(h: &mut impl Host, dt: u32) -> u32 {
    let dt = x87::round_trip(f32::from_bits(dt)).to_bits();
    let inst = h.call_sym(CLUSTER_MANAGER, Conv::Cdecl, &[]);
    h.call_sym(DELAY_RELEASE_STEP, Conv::Thiscall(inst), &[dt])
}

/// The shader-cluster loaded callbacks (cdecl `(cluster, &object)`):
/// `AsyncLoadManager_sGetInstance()` (result unused), `object->field =
/// cluster`, then a tail jump to the object's `on_loaded` (thiscall, no
/// stack words), whose `eax` they return. `BlurEffect_onShaderClusterLoaded_006c3950`
/// (`+0x20`, `BlurEffect_doOnShaderClusterLoaded`) and
/// `PhyFMVPlayerManager_onShaderClusterLoaded` (`+0x360`,
/// `PhyFMVPlayerManager_doOnShaderBinFilesLoaded`).
pub fn on_shader_cluster_loaded(
    h: &mut impl Host,
    cluster: u32,
    object: u32,
    field: u32,
    on_loaded: u32,
) -> u32 {
    h.call_sym(ASYNC_LOAD_MANAGER, Conv::Cdecl, &[]);
    let o = h.read::<u32>(object);
    h.write(o + field, cluster);
    h.call_sym(on_loaded, Conv::Thiscall(o), &[])
}

/// `ShaderParams_FindByName(params; name)` (thiscall).
pub const FIND_PARAM_BY_NAME: u32 = 0x0056_cbf0;
/// `ShaderParamBuffer_SetValue2(buffer; param, value)` (thiscall).
pub const SET_PARAM_VALUE: u32 = 0x0063_86c0;
/// `"BlendWeight"` (`.rdata`).
pub const BLEND_WEIGHT: u32 = 0x00b4_95d8;
/// `memset` through its import thunk (cdecl dst, byte, n).
pub const MEMSET: u32 = 0x0094_964c;

/// `PhyreScene_setColor4_00638840(this; v)` (thiscall, from
/// `PhyreScene_processVFXDrawEvent`): unless the byte at `+0x123a0` is set,
/// four floats from `v` to `+0x1238c` through `fld`/`fstp`. Callers read
/// no result.
pub fn scene_set_color4(h: &mut impl Mem, this: u32, v: u32) {
    if h.read::<u8>(this + 0x0001_23a0) != 0 {
        return;
    }
    for k in 0..4 {
        let x = h.read::<u32>(v.wrapping_add(4 * k));
        let x = x87::round_trip(f32::from_bits(x)).to_bits();
        h.write(this + 0x0001_238c + 4 * k, x);
    }
}

/// `MotionBlur_setMotionBlurWeight(this; w)` (0x006c5600, thiscall): the
/// weight at `+0x5c` (through `fld`/`fstp`); with shader parameters
/// (`+0x64`) that have `"BlendWeight"` and a parameter buffer (`+0x60`),
/// `ShaderParamBuffer_SetValue2(buffer->+4; param, this + 0x5c)`. Callers
/// read no result.
pub fn motion_blur_set_weight(h: &mut impl Host, this: u32, w: u32) {
    let params = h.read::<u32>(this + 0x64);
    let w = x87::round_trip(f32::from_bits(w)).to_bits();
    h.write(this + 0x5c, w);
    if params == 0 {
        return;
    }
    let name = h.rebase(BLEND_WEIGHT);
    let param = h.call_sym(FIND_PARAM_BY_NAME, Conv::Thiscall(params), &[name]);
    if param == 0 {
        return;
    }
    let buffer = h.read::<u32>(this + 0x60);
    if buffer == 0 {
        return;
    }
    let b = h.read::<u32>(buffer + 4);
    h.call_sym(SET_PARAM_VALUE, Conv::Thiscall(b), &[param, this + 0x5c]);
}

/// `PostEffect_PostEffect_006c5670(this)` (thiscall, the constructor
/// `PostEffect_createInstance_006c5790` calls): four words cleared, four
/// `0x164`-byte blocks from `+0x10` cleared with `memset` and their bytes
/// at `+0x5b0` cleared; returns `this`.
pub fn post_effect_ctor(h: &mut impl Host, this: u32) -> u32 {
    for k in 0..4 {
        h.write(this + 4 * k, 0u32);
    }
    for i in 0..4 {
        h.call_sym(MEMSET, Conv::Cdecl, &[this + 0x10 + 0x164 * i, 0, 0x164]);
        h.write(this + 0x5b0 + i, 0u8);
    }
    this
}

/// `BrokenScreenEffect`'s piece release `0x006d3600` (thiscall, a jump to
/// `0x006d3e00`).
pub const PIECE_RELEASE: u32 = 0x006d_3600;
/// The shader-parameter writer `0x006e60d0` (cdecl param, a, b).
pub const PARAM_WRITE: u32 = 0x006e_60d0;
/// `RadialBlur_resize_007035f0` (thiscall; w, h).
pub const RADIAL_BLUR_RESIZE: u32 = 0x0070_35f0;
/// The line effect's resize `0x006f89b0` (thiscall; w, h).
pub const RADIAL_LINE_RESIZE: u32 = 0x006f_89b0;
/// `BlurEffectEx_postInitialize(this)` (thiscall).
pub const BLUR_EX_POST_INITIALIZE: u32 = 0x006c_4b60;
/// `BlurEffect_isActivated(blur)` (thiscall, `al`).
pub const BLUR_IS_ACTIVATED: u32 = 0x006c_3940;

/// `BrokenScreenEffect_reset(this)` (0x006d5970, thiscall): with pieces
/// (`+0x38`), both halves of each of the `+0x34` (signed, read each
/// round) entries released (`0x006d3600`, the entry read again for the
/// second), the table dropped; then the opacity (`+8`) = 1.0. Callers read
/// no result.
pub fn broken_screen_reset(h: &mut impl Host, this: u32) {
    if h.read::<u32>(this + 0x38) != 0 {
        let mut i = 0u32;
        while i.cast_signed() < h.read::<u32>(this + 0x34).cast_signed() {
            let table = h.read::<u32>(this + 0x38);
            let entry = h.read::<u32>(table.wrapping_add(4 * i));
            if entry != 0 {
                let a = h.read::<u32>(entry);
                if a != 0 {
                    h.call_sym(PIECE_RELEASE, Conv::Thiscall(a), &[]);
                }
                let table = h.read::<u32>(this + 0x38);
                let entry = h.read::<u32>(table.wrapping_add(4 * i));
                let b = h.read::<u32>(entry.wrapping_add(4));
                if b != 0 {
                    h.call_sym(PIECE_RELEASE, Conv::Thiscall(b), &[]);
                }
            }
            i = i.wrapping_add(1);
        }
        h.write(this + 0x38, 0u32);
    }
    h.write(this + BROKEN_SCREEN_OPACITY, 1.0f32.to_bits());
}

/// `ShaderParam_Write_006426e0(params, a, b)` (cdecl): `0x006e60d0(param,
/// a, b)` for each parameter of the array (count `+0`, top bit masked,
/// read each round; items `+4`); 0, also for a null array.
pub fn shader_param_write(h: &mut impl Host, params: u32, a: u32, b: u32) -> u32 {
    if params == 0 {
        return 0;
    }
    let mut i = 0u32;
    while i < h.read::<u32>(params) & 0x7fff_ffff {
        let items = h.read::<u32>(params + 4);
        let p = h.read::<u32>(items.wrapping_add(4 * i));
        h.call_sym(PARAM_WRITE, Conv::Cdecl, &[p, a, b]);
        i += 1;
    }
    0
}

/// `BlurEffectEx_initialize(this; a, b, c, d, e)` (0x006c4b00, thiscall):
/// `+4` = a, `+8` = d, `+0xc` = e; from the `BlurEffect` singleton (asked
/// each time) its `+0x24`, `+0x34`, `+0x38` to `+0x20`, `+0x30`, `+0x34`;
/// `+0x2c` = -1, `+0x28` = 0, `+0x38` = 0.0; returns 0.
pub fn blur_ex_initialize(h: &mut impl Host, this: u32, a: u32, d: u32, e: u32) -> u32 {
    h.write(this + 4, a);
    h.write(this + 8, d);
    h.write(this + 0xc, e);
    let blur = get_instance(h, BLUR_EFFECT);
    let v = h.read::<u32>(blur.wrapping_add(0x24));
    h.write(this + 0x20, v);
    h.write(this + 0x2c, u32::MAX);
    h.write(this + 0x28, 0u8);
    let blur = get_instance(h, BLUR_EFFECT);
    let v = h.read::<u32>(blur.wrapping_add(0x34));
    h.write(this + 0x30, v);
    let blur = get_instance(h, BLUR_EFFECT);
    let v = h.read::<u32>(blur.wrapping_add(0x38));
    h.write(this + 0x38, 0u32);
    h.write(this + 0x34, v);
    0
}

/// `RadialBlur_resizeAll_006c3c40(this; w, h)` (thiscall, from
/// `PhyreScene_resize`): the three render targets (`+0`, `+4`, `+8`)
/// through `0x00a0e4b0(target, w, h, 1)`, then `RadialBlur_resize_007035f0`
/// (`+0x18`) and `0x006f89b0` (`+0x1c`); returns 0.
pub fn radial_blur_resize_all(h: &mut impl Host, this: u32, w: u32, hh: u32) -> u32 {
    for k in 0..3 {
        let target = h.read::<u32>(this + 4 * k);
        h.call_sym(TARGET_RESIZE, Conv::Cdecl, &[target, w, hh, 1]);
    }
    let blur = h.read::<u32>(this + 0x18);
    h.call_sym(RADIAL_BLUR_RESIZE, Conv::Thiscall(blur), &[w, hh]);
    let line = h.read::<u32>(this + 0x1c);
    h.call_sym(RADIAL_LINE_RESIZE, Conv::Thiscall(line), &[w, hh]);
    0
}

/// `BlurEffectEx_update(this; dt)` (0x006c4e00, thiscall): before the
/// first set-up (`+0x10` 0) a successful `BlurEffectEx_postInitialize`
/// stops it (`+0x28` = 0), as does an active `BlurEffect`; stopped, its
/// counter (`+0x3c`) and weight (`+0x38`) are cleared and `+0x2c` = -1;
/// running, the counter goes up. Returns 0.
pub fn blur_ex_update(h: &mut impl Host, this: u32) -> u32 {
    if h.read::<u32>(this + 0x10) == 0
        && h.call_sym(BLUR_EX_POST_INITIALIZE, Conv::Thiscall(this), &[]) != 0
    {
        h.write(this + 0x28, 0u8);
    }
    let blur = get_instance(h, BLUR_EFFECT);
    if h.call_sym(BLUR_IS_ACTIVATED, Conv::Thiscall(blur), &[]) & 0xff != 0 {
        h.write(this + 0x28, 0u8);
    }
    if h.read::<u8>(this + 0x28) != 0 {
        let n = h.read::<u32>(this + 0x3c).wrapping_add(1);
        h.write(this + 0x3c, n);
        return 0;
    }
    h.write(this + 0x3c, 0u32);
    h.write(this + 0x38, 0u32);
    h.write(this + 0x2c, u32::MAX);
    0
}

/// `memcpy` through its import thunk (cdecl dst, src, n).
pub const MEMCPY: u32 = 0x0094_9256;

/// The shader-parameter copies `ShaderParamBuffer_SetValue2`
/// (0x006386c0), `ShaderParam_CopyToBuffer_00638720`,
/// `ShaderParamBuffer_SetValue` (0x00638780) and
/// `ShaderParam_CopyToBuffer_00698960` (thiscall `(buffer; param, src)`,
/// the same code): 1 for a null `src`; 3 when the parameter's range
/// (offset `+8` and size `+0xa` & 0x1fff, words) ends past the buffer's
/// size (`+0`, unsigned) or starts in its 4-byte header; else
/// `memcpy(buffer + offset, src, size)` and 0.
pub fn shader_param_copy(h: &mut impl Host, buffer: u32, param: u32, src: u32) -> u32 {
    if src == 0 {
        return 1;
    }
    let offset = u32::from(h.read::<u16>(param + 8));
    let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
    if offset + size > h.read::<u32>(buffer) || offset < 4 {
        return 3;
    }
    h.call_sym(
        MEMCPY,
        Conv::Cdecl,
        &[buffer.wrapping_add(offset), src, size],
    );
    0
}

/// `BrokenScreenEffect_updateLightParameters(this; p0..p6)` (0x006d5a70,
/// thiscall): each non-null pointer's float copied (through `fld`/`fstp`)
/// to `+0xc`, `+0x10`, `+0x14`, `+0x1c`, `+0x20`, `+0x24`, `+0x28`;
/// returns the last pointer (its `eax`).
pub fn broken_screen_update_light(h: &mut impl Mem, this: u32, p: [u32; 7]) -> u32 {
    for (ptr, off) in p.into_iter().zip([0xc, 0x10, 0x14, 0x1c, 0x20, 0x24, 0x28]) {
        if ptr != 0 {
            let v = h.read::<u32>(ptr);
            h.write(this + off, x87::round_trip(f32::from_bits(v)).to_bits());
        }
    }
    p[6]
}
