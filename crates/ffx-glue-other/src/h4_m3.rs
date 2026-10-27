//! Continuation round 11 of list `h4-glue-other`, batch 1: the movie
//! player manager's start of a movie.

use ffx_bridge::Global;
use ffx_bridge::x87::{add, div, ftol2, load, mul, store};
use ffx_graphic::host::{Conv, Host};

/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PHYRE_PRINTF: u32 = 0x0043_52f0;
/// `PhyFMVPlayerManager_DestroyFMVPlayer(this)` (thiscall).
pub const DESTROY_PLAYER: u32 = 0x006d_71b0;
/// `sprintf` through its import slot (cdecl).
pub const SPRINTF: Global<u32> = Global::new(0x00b0_c3dc);
/// `Phyre_PFramework_PApplication_GetApplication()` (cdecl): `+0x26c` the
/// window's width, `+0x270` its height.
pub const GET_APPLICATION: u32 = 0x0062_3b90;
/// `g_CurrentMapNo` (`0x012fbc40`).
pub const CURRENT_MAP: u32 = 0x012f_bc40;
/// `g_FMVSkipped` (`0x00cded31`, a byte).
pub const FMV_SKIPPED: u32 = 0x00cd_ed31;
/// `g_FMVUnskippableIds` (`0x00b49b08`, 16 words).
pub const UNSKIPPABLE: u32 = 0x00b4_9b08;
/// `PhyFMVPlayerManager_getVideoName(this; id, out)` (thiscall, ret 8).
pub const VIDEO_NAME: u32 = 0x006d_93e0;
/// `Configs_ConfigFile_saveConfigToFile(name)` (stdcall).
pub const SAVE_CONFIG: u32 = 0x006d_a4a0;
/// `PTextureD3D11_register_0058cf80(texture)` (thiscall) -> 0 when done.
pub const REGISTER_TEXTURE: u32 = 0x0058_cf80;
/// `Phyre_PRendering_PTexture2DBase_setDimensions(texture; width, height,
/// format, mips)` (thiscall, ret 0x10).
pub const SET_DIMENSIONS: u32 = 0x004c_40c0;
/// `Phyre_PRendering_PTexture2DBase_updateMaxLevel(texture)` (thiscall).
pub const UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
/// `Phyre_PRendering_PTexture2DD3D11_isBound(texture)` (thiscall) -> `al`.
pub const IS_BOUND: u32 = 0x0059_5f90;
/// `FUN_00599b70(texture; levels)` (thiscall, ret 4) -> 0 when done.
pub const ALLOCATE_LEVELS: u32 = 0x0059_9b70;
/// `fiosUnifyFilename(path, out, size)` (cdecl).
pub const UNIFY_FILENAME: u32 = 0x0067_9820;
/// `AsyncLoadManager_sGetInstance()` (cdecl).
pub const ASYNC_LOAD_MANAGER: u32 = 0x0069_1b40;
/// `AsyncLoadManager_loadCluster(manager, 0, path, callback, 1, arg lo,
/// arg hi)` (cdecl, varargs).
pub const LOAD_CLUSTER: u32 = 0x0069_1660;
/// `PhyFMVPlayerManager_onShaderClusterLoaded` (`0x006da460`), the load's
/// callback.
pub const ON_CLUSTER_LOADED: u32 = 0x006d_a460;
/// `PhyFMVPlayerManager_fmvPlayer_initialize(arg)` (stdcall).
pub const PLAYER_INITIALIZE: u32 = 0x006d_8f30;
/// `ButtonIcon_setUvs_006e8020(icon; id)` (thiscall, ret 4).
pub const SET_UVS: u32 = 0x006e_8020;
/// `g_PhyreScene` (`0x00ccb9d8`, a pointer): `+0x10de0` the skip prompt's
/// button icon.
pub const PHYRE_SCENE: u32 = 0x00cc_b9d8;
/// The movie shader cluster's path (`0x00c3441c`, a pointer).
pub const SHADER_CLUSTER_PATH: u32 = 0x00c3_441c;
/// Strings: `""` (`0x00b8fbeb`), the destroy message (`0x00b49cf8`), the
/// missing name message (`0x00b49d48`, `%d`), the bound texture message
/// (`0x00b42278`).
pub const EMPTY: u32 = 0x00b8_fbeb;
/// See [`EMPTY`].
pub const DESTROY_MESSAGE: u32 = 0x00b4_9cf8;
/// See [`EMPTY`].
pub const NO_NAME_MESSAGE: u32 = 0x00b4_9d48;
/// See [`EMPTY`].
pub const BOUND_MESSAGE: u32 = 0x00b4_2278;
/// The texture format (`0x00ca2fcc`).
pub const FORMAT: u32 = 0x00ca_2fcc;
/// `f32` 2^32 (`0x00b11f14`), the movie sizes 1920 (`0x00b4a1ac`), 1080
/// (`0x00b4a1a0`), 1280 (`0x00b4a1a8`), 720 (`0x00b4a198`), and `f64` 0.5
/// (`0x00b92220`).
pub const TWO_POW_32: u32 = 0x00b1_1f14;
/// See [`TWO_POW_32`].
pub const WIDTH_WIDE: u32 = 0x00b4_a1ac;
/// See [`TWO_POW_32`].
pub const HEIGHT_WIDE: u32 = 0x00b4_a1a0;
/// See [`TWO_POW_32`].
pub const WIDTH_NARROW: u32 = 0x00b4_a1a8;
/// See [`TWO_POW_32`].
pub const HEIGHT_NARROW: u32 = 0x00b4_a198;
/// See [`TWO_POW_32`].
pub const HALF: u32 = 0x00b9_2220;
/// Bytes of [`initialize`]'s frame, from the original's `ebp - 0x104`: the
/// shader cluster's unified path.
pub const INIT_FRAME: u32 = 0x100;

/// A `u32` as the original's `fild` plus 2^32 for the negative ones,
/// stored as `f32`.
fn unsigned_to_f32(h: &impl Host, v: u32) -> u32 {
    let x = f64::from(v.cast_signed());
    let x = if v.cast_signed() < 0 {
        add(x, load(h.read(h.rebase(TWO_POW_32))))
    } else {
        x
    };
    store(x)
}

/// One of the movie's three textures: `width` x `height`, its levels
/// allocated unless it holds 3, flagged `0x200`.
fn setup_texture(h: &mut impl Host, texture: u32, width: u32, height: u32) {
    let format = h.rebase(FORMAT);
    h.call_sym(
        SET_DIMENSIONS,
        Conv::Thiscall(texture),
        &[width, height, format, 0],
    );
    let flags: u32 = h.read(texture + 0x14);
    h.write(texture + 0x14, flags & !2);
    h.call_sym(UPDATE_MAX_LEVEL, Conv::Thiscall(texture), &[]);
    let n = h.read::<u32>(texture + 0x24) & 0x7fff_ffff;
    if n != 3 {
        if h.call_sym(IS_BOUND, Conv::Thiscall(texture), &[]) & 0xff != 0 {
            let m = h.rebase(BOUND_MESSAGE);
            h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[1, m]);
        }
        if h.call_sym(ALLOCATE_LEVELS, Conv::Thiscall(texture), &[3]) == 0 {
            h.write(texture + 8, 0u32);
        }
    }
    let flags: u32 = h.read(texture + 0x14);
    h.write(texture + 0x14, flags | 0x200);
}

/// `PhyFMVPlayerManager_initialize(this; id, wide, flag)` (`0x006d9c80`,
/// thiscall, ret 0xc; `frame` see [`INIT_FRAME`]): destroys a running
/// player (`+0x6d2`), resets the movie state (window size, skip flags,
/// the id at `+0x6dc`, unskippable unless listed), looks up the movie's
/// name (`+0x4f0`; none: a message and 0), registers the texture at
/// `+0x364` (its error returned), sizes the three textures (`+0x38c` full
/// size, `+0x400` and `+0x474` half size; 1920x1080 for `wide`, else
/// 1280x720), stores the aspect ratio (`+0x6cc`), loads the movie shader
/// cluster and starts the player. Returns 0 or the register's error.
#[allow(clippy::too_many_lines)] // the original's straight run
pub fn initialize(h: &mut impl Host, frame: u32, this: u32, id: u32, wide: u32, flag: u32) -> u32 {
    if h.read::<u8>(this + 0x6d2) != 0 {
        let m = h.rebase(DESTROY_MESSAGE);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, m]);
        h.call_sym(DESTROY_PLAYER, Conv::Thiscall(this), &[]);
    }
    let name = this + 0x4f0;
    let empty = h.rebase(EMPTY);
    let sprintf = SPRINTF.get(h);
    h.call(sprintf, Conv::Cdecl, &[name, empty]);
    let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
    let w = unsigned_to_f32(h, h.read(app + 0x26c));
    h.write(this + 0x6f4, w);
    let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
    let hh = unsigned_to_f32(h, h.read(app + 0x270));
    h.write(this + 0x6f8, hh);
    let map: u32 = h.read(h.rebase(CURRENT_MAP));
    let skipped = h.rebase(FMV_SKIPPED);
    h.write(skipped, 0u8);
    h.write(this + 0x74e, u8::from(map == 0xb8));
    h.write(this + 0x6e0, 0u32);
    h.write(this + 0x6d8, 0u32);
    h.write(this + 0x6dc, id);
    let list = h.rebase(UNSKIPPABLE);
    let listed = (0..16u32).any(|k| h.read::<u32>(list + 4 * k) == id);
    h.write(this + 0x74d, u8::from(!listed));
    h.write(this + 0x6e4, 0u8);
    h.write(this + 0x6fc, 0u16);
    h.write(this + 0x6d0, 0u16);
    h.write(this + 0x720, 0u8);
    h.write(this + 0x6fe, flag as u8);
    h.write(this + 0x6f0, 0u32);
    let always = id == 0x2e || id == 0x48 || id == 0x20 || h.read::<u8>(this + 0x6fe) != 0;
    h.write(this + 0x570, u8::from(always));
    h.call_sym(VIDEO_NAME, Conv::Thiscall(this), &[id, name]);
    h.call_sym(SAVE_CONFIG, Conv::Stdcall, &[name]);
    if crate::h4_m1::same_text(h, name, empty) {
        let m = h.rebase(NO_NAME_MESSAGE);
        h.call_sym(PHYRE_PRINTF, Conv::Cdecl, &[2, m, id]);
        return 0;
    }
    let texture = this + 0x364;
    if h.read::<u32>(texture + 0x1c) & 0x80 == 0 {
        for (k, v) in [1u8, 1, 2, 2, 2].into_iter().enumerate() {
            h.write(texture + k as u32, v);
        }
    }
    let r = h.call_sym(REGISTER_TEXTURE, Conv::Thiscall(texture), &[]);
    if r != 0 {
        return r;
    }
    let (wsym, hsym) = if wide as u8 == 0 {
        (WIDTH_NARROW, HEIGHT_NARROW)
    } else {
        (WIDTH_WIDE, HEIGHT_WIDE)
    };
    let w = store(load(h.read(h.rebase(wsym))));
    h.write(this + 0x6f4, w);
    let hh = store(load(h.read(h.rebase(hsym))));
    h.write(this + 0x6f8, hh);
    let full_w = ftol2(load(h.read(this + 0x6f4)));
    let full_h = ftol2(load(h.read(this + 0x6f8)));
    setup_texture(h, this + 0x38c, full_w, full_h);
    let half = f64::from_bits(h.read(h.rebase(HALF)));
    for texture in [this + 0x400, this + 0x474] {
        let half_h = ftol2(mul(load(h.read(this + 0x6f8)), half));
        let half_w = ftol2(mul(half, load(h.read(this + 0x6f4))));
        setup_texture(h, texture, half_w, half_h);
    }
    let aspect = store(div(load(h.read(this + 0x6f4)), load(h.read(this + 0x6f8))));
    h.write(this + 0x6cc, aspect);
    let path: u32 = h.read(h.rebase(SHADER_CLUSTER_PATH));
    h.call_sym(UNIFY_FILENAME, Conv::Cdecl, &[path, frame, 0x100]);
    let manager = h.call_sym(ASYNC_LOAD_MANAGER, Conv::Cdecl, &[]);
    let callback = h.rebase(ON_CLUSTER_LOADED);
    let hi = (this.cast_signed() >> 31).cast_unsigned();
    h.call_sym(
        LOAD_CLUSTER,
        Conv::Cdecl,
        &[manager, 0, frame, callback, 1, this, hi],
    );
    h.call_sym(PLAYER_INITIALIZE, Conv::Stdcall, &[0]);
    let scene: u32 = h.read(h.rebase(PHYRE_SCENE));
    let icon: u32 = h.read(scene.wrapping_add(0x10de0));
    h.call_sym(SET_UVS, Conv::Thiscall(icon), &[7]);
    0
}
