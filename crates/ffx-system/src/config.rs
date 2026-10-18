//! Configuration and mode flags (debug, Asian-version mode, language swap,
//! video mode, confirm button) and the Eternal Calm file set.

/// The slot of `n` in a run of 7 numbers from `first`
/// (`isEternalCalmScene` and friends): `n - first`, wrapping, when it is
/// in `0..7`.
#[must_use]
pub const fn eternal_calm_slot(n: i32, first: i32) -> Option<u32> {
    let d = n.wrapping_sub(first);
    if d >= 0 && d < 7 {
        Some(d.cast_unsigned())
    } else {
        None
    }
}

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::eternal_calm_slot;

    /// `FFXIsDebug`'s flag.
    pub const IS_DEBUG: Global<i8> = Global::new(0x0133_c910);
    /// `FFXGetAsiaMode` / `FFXSetAsiaMode`'s flag.
    pub const ASIA_MODE: Global<u8> = Global::new(0x0133_0833);
    /// `TOGetFFXLangSwapFlag`'s flag.
    pub const LANG_SWAP: Global<u32> = Global::new(0x0186_5cdc);
    /// `TOGetFFXVideoMode`'s value.
    pub const VIDEO_MODE: Global<u32> = Global::new(0x0186_5f00);
    /// The confirm button assignment, -1 when unset.
    pub const CONFIRM_BUTTON: Global<i32> = Global::new(0x00c5_9554);
    /// The first map of the Eternal Calm set (7 consecutive): a map is a
    /// PS2 file number over 18, as `CDfileSize_CD` asks.
    pub const ETERNAL_CALM_FIRST: Global<i32> = Global::new(0x00c3_3468);
    /// The Eternal Calm scenes' file names (7 pointers).
    pub const ETERNAL_CALM_NAMES: Global<u32> = Global::new(0x00c3_34b8);
    /// `setBattleEnable`'s value (read by `MsBattleEncountExe`).
    pub const BATTLE_ENABLE: Global<u32> = Global::new(0x00c4_21cc);

    /// `FFXIsDebug()` (`0x00887c50`).
    pub fn is_debug(mem: &impl Mem) -> i32 {
        i32::from(IS_DEBUG.get(mem))
    }

    /// `FFXGetAsiaMode()` (`0x00887c20`).
    pub fn get_asia_mode(mem: &impl Mem) -> u32 {
        u32::from(ASIA_MODE.get(mem))
    }

    /// `FFXSetAsiaMode(mode)` (`0x00887c70`).
    pub fn set_asia_mode(mem: &mut impl Mem, mode: u8) {
        ASIA_MODE.set(mem, mode);
    }

    /// `TOGetFFXLangSwapFlag()` (`0x008ac390`).
    pub fn lang_swap_flag(mem: &impl Mem) -> u32 {
        LANG_SWAP.get(mem)
    }

    /// `TOGetFFXVideoMode()` (`0x008ac3a0`).
    pub fn video_mode(mem: &impl Mem) -> u32 {
        VIDEO_MODE.get(mem)
    }

    /// `TOGetSystemConfirmButtonAssign()` (`0x008acf30`): the assignment,
    /// -1 replaced by 0 (and stored back).
    pub fn confirm_button_assign(mem: &mut impl Mem) -> i32 {
        let v = CONFIRM_BUTTON.get(mem);
        let v = if v == -1 { 0 } else { v };
        CONFIRM_BUTTON.set(mem, v);
        v
    }

    /// `isEternalCalmScene(n)` (`0x0067ab50`) and `isEternalCalmMgrp(n)`
    /// (`0x0067ab30`), the same test.
    pub fn is_eternal_calm(mem: &impl Mem, n: i32) -> u32 {
        u32::from(eternal_calm_slot(n, ETERNAL_CALM_FIRST.get(mem)).is_some())
    }

    /// `getEternalCalmSceneName(n)` (`0x0067aac0`): the scene's file name,
    /// or null.
    pub fn eternal_calm_scene_name(mem: &impl Mem, n: i32) -> u32 {
        match eternal_calm_slot(n, ETERNAL_CALM_FIRST.get(mem)) {
            Some(i) => ETERNAL_CALM_NAMES.ptr(mem).at(i).get(mem),
            None => 0,
        }
    }

    /// `setBattleEnable(on)` (`0x00782f70`).
    pub fn set_battle_enable(mem: &mut impl Mem, on: u32) {
        BATTLE_ENABLE.set(mem, on);
    }

    /// The video mode byte `FFXGetVideoMode` reads (signed).
    pub const FFX_VIDEO_MODE: Global<i8> = Global::new(0x0133_0831);
    /// The byte `sceGsSetDefDrawEnv2` returns (signed; 1 in the image).
    pub const DEF_DRAW_ENV2: Global<i8> = Global::new(0x00b5_ec4f);

    /// `FFXGetVideoMode()` (`0x00887c40`).
    pub fn ffx_video_mode(mem: &impl Mem) -> i32 {
        i32::from(FFX_VIDEO_MODE.get(mem))
    }

    /// `sceGsSetDefDrawEnv2(...)` (`0x00887c60`): sets nothing on PC.
    pub fn gs_set_def_draw_env2(mem: &impl Mem) -> i32 {
        i32::from(DEF_DRAW_ENV2.get(mem))
    }

    /// `g_Settings` (`0x022fb504`): the settings block's address.
    pub const SETTINGS: Global<u32> = Global::new(0x022f_b504);

    /// `LocalizationManager_Initialize(this)` (`0x006db090`, thiscall):
    /// the manager's three words from the settings' language index
    /// (`*g_Settings`): 1..=5 the Western languages (`+0` 1, `+8` 1, `+4`
    /// the game language 1 us, 2 fr, 4 de, 5 it, 3 es), 6 Korean and 7
    /// Chinese (`+0` 3, `+4` 9 or 10, `+8` 0); anything else Japanese
    /// (all 0). Returns 0.
    pub fn localization_initialize(mem: &mut impl Mem, this: u32) -> u32 {
        let settings = SETTINGS.get(mem);
        let [kind, lang, western] = match mem.read::<u32>(settings) {
            1 => [1, 1, 1],
            2 => [1, 2, 1],
            3 => [1, 4, 1],
            4 => [1, 5, 1],
            5 => [1, 3, 1],
            6 => [3, 9, 0],
            7 => [3, 10, 0],
            _ => [0, 0, 0],
        };
        mem.write(this, kind);
        mem.write(this.wrapping_add(4), lang);
        mem.write(this.wrapping_add(8), western);
        0
    }
}
