//! The render-side glue, seventh batch (M4 port round 3, list
//! `h3-glue-render`): the functions round 2 left for want of a host path,
//! now that [`ffx_graphic::host::Host`] returns `double`s from `ST0`
//! (`call_st0_f64`) and passes cdecl calls of up to twelve words: the
//! clocks, the PS2 point-light setup, the post-process shader-bin loader
//! and the scene's async load requests.

use ffx_bridge::Global;

pub use crate::glue::SCENE;

/// `Phyre_PFramework_PApplication_GetApplication`.
pub const GET_APPLICATION: u32 = crate::glue::GET_APPLICATION;
/// `FFXApplication_getAppTime(this)` (thiscall): `float` in `ST0`.
pub const GET_APP_TIME: u32 = 0x0042_f690;
/// `Phyre_PTimerWin32_GetTime()` (cdecl): seconds as a `double` in `ST0`.
pub const TIMER_GET_TIME: u32 = 0x0042_fd80;
/// `g_AppStartTime` (`double`).
pub const APP_START_TIME: Global<f64> = Global::new(0x00c9_0278);
/// The microseconds per second `graphicGetTimeInMicroseconds` scales by
/// (`double` in `.rdata`).
pub const MICROSECONDS: Global<f64> = Global::new(0x00b3_ecc0);
/// `g_GraphicSkipFrame`.
pub const GRAPHIC_SKIP_FRAME: Global<u32> = crate::glue2::GRAPHIC_SKIP_FRAME;
/// `Phyre_PS2LightingSetup_registerWithPointLightEffect` (cdecl, ten
/// words).
pub const REGISTER_POINT_LIGHT: u32 = 0x006e_5c90;
/// `AsyncLoadManager_sGetInstance`.
pub const ASYNC_LOAD_INSTANCE: u32 = crate::glue::ASYNC_LOAD_INSTANCE;
/// `AsyncLoadManager_loadCluster(manager, 8 words)` (cdecl).
pub const LOAD_CLUSTER: u32 = 0x0069_1660;
/// `AsyncLoadManager_waitTasks_00691d40(this)`.
pub const WAIT_TASKS: u32 = 0x0069_1d40;
/// `PostProcessManager_onShaderBinLoaded_00694c20`, the callback the
/// shader-bin loads register.
pub const ON_SHADER_BIN_LOADED: u32 = 0x0069_4c20;
/// The 14 post-process shader-bin file names (`.data`, string pointers).
pub const SHADER_BIN_NAMES: u32 = 0x00c3_4114;
/// How many shader bins the post-process manager loads.
pub const SHADER_BIN_COUNT: u32 = 14;
/// `AsyncLoadManager_registAsyncLoadTask(manager, 8 words)` (cdecl).
pub const REGIST_ASYNC_LOAD_TASK: u32 = 0x0069_18b0;
/// `operator_new_006304c0(n)` (cdecl).
pub const OPERATOR_NEW_6304C0: u32 = 0x0063_04c0;
/// `AsyncLoadNewCharacter()` (cdecl).
pub const ASYNC_LOAD_NEW_CHARACTER: u32 = 0x0078_2f80;
/// The task bodies the scene's async requests register.
pub const TASK_LOAD_CHARACTER: u32 = 0x0065_7850;
pub const TASK_LOAD_MAGIC_FILE_VFX_TEXTURE: u32 = 0x0065_7880;
pub const TASK_LOAD_VFX_TEXTURE: u32 = 0x0065_78b0;

/// `_ftol2` (`0x00949816`): `x` to a 64-bit integer, toward zero
/// (`fistp qword` at the game's round-to-nearest, then one step back
/// toward zero when the rounding went past `x`), in `edx:eax`. NaN and
/// anything outside `i64` give the x87's integer indefinite.
#[must_use]
pub fn ftol2_64(x: f64) -> u64 {
    if x.is_nan() || x >= 9_223_372_036_854_775_808.0 || x < -9_223_372_036_854_775_808.0 {
        0x8000_0000_0000_0000
    } else {
        // in range, checked
        (x.trunc() as i64).cast_unsigned()
    }
}

/// A word sign-extended to the two words `cdq` makes (low, high).
#[must_use]
pub const fn cdq(v: u32) -> [u32; 2] {
    [v, ((v as i32) >> 31) as u32]
}

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use crate::glue::bridge::instance;
    use ffx_bridge::x87::round_trip;
    use ffx_graphic::host::{Conv, Host};

    /// `graphicGetTime()` `0x00641410`: the application's
    /// `FFXApplication_getAppTime` (a `float` it leaves in `ST0`, here
    /// as the `double` the caller's x87 holds).
    pub fn get_time(h: &mut impl Host) -> f64 {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        h.call_sym_st0_f64(GET_APP_TIME, Conv::Thiscall(app), &[])
    }

    /// `graphicGetTimeInMicroseconds()` `0x00641420`:
    /// `(GetTime() - g_AppStartTime) * 1e6`, each step rounded to the
    /// x87's 53 bits, to a 64-bit integer by `_ftol2` (`edx:eax`).
    pub fn get_time_in_microseconds(h: &mut impl Host) -> u64 {
        let now = h.call_sym_st0_f64(TIMER_GET_TIME, Conv::Cdecl, &[]);
        let since = now - APP_START_TIME.get(h);
        ftol2_64(since * MICROSECONDS.get(h))
    }

    /// `graphicPS2LightingSetupEffect(effect, a1, .., a10)` `0x00642750`:
    /// unless the frame is skipped or `effect` is 0,
    /// `registerWithPointLightEffect(effect, a2, a3, a4, a5, a6, a7, a8,
    /// a9, a10)` (`a1` unused; `a4` and `a7` pass through the x87 as
    /// floats). Returns 0.
    pub fn ps2_lighting_setup_effect(h: &mut impl Host, a: [u32; 11]) -> u32 {
        if GRAPHIC_SKIP_FRAME.get(h) != 0 || a[0] == 0 {
            return 0;
        }
        let f4 = round_trip(f32::from_bits(a[4])).to_bits();
        let f7 = round_trip(f32::from_bits(a[7])).to_bits();
        h.call_sym(
            REGISTER_POINT_LIGHT,
            Conv::Cdecl,
            &[a[0], a[2], a[3], f4, a[5], a[6], f7, a[8], a[9], a[10]],
        );
        0
    }

    /// `PostProcessManager_loadShaderBinFiles_00693d20(this)`: for each of
    /// the 14 shader bins, `loadCluster(manager, 0, name, onShaderBinLoaded,
    /// 2, (i64) this, (i64) i)` and a wait for the manager's tasks.
    /// Returns 0.
    pub fn post_load_shader_bin_files(h: &mut impl Host, this: u32) -> u32 {
        let [this_lo, this_hi] = cdq(this);
        for i in 0..SHADER_BIN_COUNT {
            let names = h.rebase(SHADER_BIN_NAMES);
            let name: u32 = h.read(names + 4 * i);
            let callback = h.rebase(ON_SHADER_BIN_LOADED);
            let manager = instance(h, ASYNC_LOAD_INSTANCE);
            let [i_lo, i_hi] = cdq(i);
            h.call_sym(
                LOAD_CLUSTER,
                Conv::Cdecl,
                &[manager, 0, name, callback, 2, this_lo, this_hi, i_lo, i_hi],
            );
            let manager = instance(h, ASYNC_LOAD_INSTANCE);
            h.call_sym(WAIT_TASKS, Conv::Thiscall(manager), &[]);
        }
        0
    }

    /// The scene's async load requests (`PhyreScene_loadCharacterAsync`
    /// and the VFX texture ones): a copy of `name` in a new buffer
    /// (`operator_new(strlen + 1)`), then `registAsyncLoadTask(manager,
    /// task, 3, (i64) this, (i64) copy, (i64) value)`.
    fn regist_load(h: &mut impl Host, this: u32, task: u32, name: u32, value: u32) {
        let mut len = 0u32;
        while h.read::<u8>(name.wrapping_add(len)) != 0 {
            len += 1;
        }
        let copy = h.call_sym(OPERATOR_NEW_6304C0, Conv::Cdecl, &[len + 1]);
        let mut i = 0u32;
        loop {
            let c: u8 = h.read(name.wrapping_add(i));
            h.write(copy.wrapping_add(i), c);
            if c == 0 {
                break;
            }
            i += 1;
        }
        let task = h.rebase(task);
        let manager = instance(h, ASYNC_LOAD_INSTANCE);
        let [v_lo, v_hi] = cdq(value);
        let [c_lo, c_hi] = cdq(copy);
        let [t_lo, t_hi] = cdq(this);
        h.call_sym(
            REGIST_ASYNC_LOAD_TASK,
            Conv::Cdecl,
            &[manager, task, 3, t_lo, t_hi, c_lo, c_hi, v_lo, v_hi],
        );
    }

    /// `PhyreScene_loadCharacterAsync(this, name, id)` `0x006642f0`
    /// (thiscall, `ret 8`): the request, then `AsyncLoadNewCharacter()`.
    /// Returns 0.
    pub fn scene_load_character_async(h: &mut impl Host, this: u32, name: u32, id: u32) -> u32 {
        regist_load(h, this, TASK_LOAD_CHARACTER, name, id);
        h.call_sym(ASYNC_LOAD_NEW_CHARACTER, Conv::Cdecl, &[]);
        0
    }

    /// `PhyreScene_loadMagicFileVFXTextureAsync(this, id, name)`
    /// `0x00664d10` (thiscall, `ret 8`). Returns 0.
    pub fn scene_load_magic_file_vfx_texture_async(
        h: &mut impl Host,
        this: u32,
        id: u32,
        name: u32,
    ) -> u32 {
        regist_load(h, this, TASK_LOAD_MAGIC_FILE_VFX_TEXTURE, name, id);
        0
    }

    /// `PhyreScene_loadVFXTextureAsync(this, id, name)` `0x00665f60`
    /// (thiscall, `ret 8`). Returns 0.
    pub fn scene_load_vfx_texture_async(h: &mut impl Host, this: u32, id: u32, name: u32) -> u32 {
        regist_load(h, this, TASK_LOAD_VFX_TEXTURE, name, id);
        0
    }
}
