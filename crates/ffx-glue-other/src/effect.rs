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
