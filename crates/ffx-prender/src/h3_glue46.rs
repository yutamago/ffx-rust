//! The render-side glue, forty-sixth batch (M4 port round 12, list
//! `h3-glue-render`): the scene's per-frame update.

/// The application getter (cdecl; the frame time `f32` at `+0x24`),
/// `g_RenderDelegationHelper` and its `0x006adad0` and
/// `RenderDelegationHelper_runFlag4` (both `this` only).
pub const GET_APPLICATION: u32 = crate::h3_glue12::GET_APPLICATION;
pub const DELEGATION_HELPER: u32 = 0x00cc_ca98;
pub const DELEGATION_UPDATE: u32 = 0x006a_dad0;
pub const DELEGATION_RUN: u32 = 0x006a_d4e0;

/// `TextureAnimation_Animate_00672000(scene; dt)`,
/// `MorphManager_GetInstance()` and `MorphManager_update(mm; dt)`,
/// `Phyre_CharacterLighting_GetInstance()` and
/// `CharacterLighting_updateFlash_006ab630(cl; dt)`.
pub const TEX_ANIM_ANIMATE: u32 = 0x0067_2000;
pub const MORPH_INSTANCE: u32 = 0x006a_bf40;
pub const MORPH_UPDATE: u32 = 0x006a_cd20;
pub const LIGHTING_INSTANCE: u32 = 0x006a_6e30;
pub const LIGHTING_UPDATE_FLASH: u32 = 0x006a_b630;

/// `g_Settings` (its byte `+0x638`: the first-run tip shown),
/// `TipSystem_Get()` and `TipSystem_PushTip(tips; id)`,
/// `graphicPushTipByLocId(id)` (cdecl, `eax` non-zero when shown).
pub const SETTINGS: u32 = 0x022f_b504;
pub const TIP_SYSTEM: u32 = 0x006e_8750;
pub const PUSH_TIP: u32 = 0x006e_8770;
pub const PUSH_TIP_BY_LOC: u32 = 0x0064_2930;

/// `g_PauseRequest` and `Pause_Update(request)` (cdecl).
pub const PAUSE_REQUEST: u32 = 0x0134_0850;
pub const PAUSE_UPDATE: u32 = 0x008a_b340;

/// `IconPanel_get()` and its `update`, `showIcon(i)`, `hideIcon(i)`,
/// `setVisible(on)` and `hideAllIcons` (thiscall).
pub const ICON_PANEL: u32 = 0x006f_78b0;
pub const ICON_UPDATE: u32 = 0x006f_79d0;
pub const ICON_SHOW: u32 = 0x006f_7970;
pub const ICON_HIDE: u32 = 0x006f_7900;
pub const ICON_SET_VISIBLE: u32 = 0x006f_7940;
pub const ICON_HIDE_ALL: u32 = 0x006f_78d0;

/// `g_CurrentMapNo` (maps 0x17, 0x15c and 0x15d reset the cheats),
/// `SpdCtrl_Init`, `SpdCtrl_Tick`; the key checks (cdecl, `eax`):
/// auto-battle, invincible, encounter; `canUseAutoBattle()`,
/// `MsCheatPlayerPowerfulOff()`; the three modes: the auto-battle
/// toggle (`0x0133d6e0`), `g_BtlCheatPowerup`, `g_EncountMode`.
pub const CURRENT_MAP: u32 = 0x012f_bc40;
pub const SPD_INIT: u32 = 0x006f_72f0;
pub const SPD_TICK: u32 = 0x006f_7470;
pub const CHECK_AUTO_BATTLE_KEY: u32 = 0x006f_7580;
pub const CHECK_INVINCIBLE_KEY: u32 = 0x006f_76c0;
pub const CHECK_ENCOUNTER_KEY: u32 = 0x006f_75c0;
pub const CAN_USE_AUTO_BATTLE: u32 = 0x0078_6f50;
pub const POWERFUL_OFF: u32 = 0x0078_46e0;
pub const AUTO_BATTLE: u32 = 0x0133_d6e0;
pub const CHEAT_POWERUP: u32 = 0x012f_b7cc;
pub const ENCOUNT_MODE: u32 = 0x00c4_21d8;

/// `inputIsPressButton(mask)` (cdecl); the free camera
/// (`FreeCamera_CreateInstance_0042f260()`, `setFreePause` `this`
/// only, `handleInput(camera; dt, word)`) and the debug word that lets
/// its state toggle the pause (`0x0133c930`).
pub const IS_PRESS_BUTTON: u32 = 0x0063_0cf0;
pub const FREE_CAMERA: u32 = 0x0042_f260;
pub const FREE_CAMERA_PAUSE: u32 = 0x0068_0320;
pub const FREE_CAMERA_INPUT: u32 = 0x0068_0030;
pub const DEBUG_WORD: u32 = 0x0133_c930;

/// The per-frame updates at the end (getter, then the update with the
/// frame time unless noted): `PhyFMVPlayerManager_processRequests`
/// (`0x006dadf0`, a `jmp`; `this` only), the blur, the blur-ex, the
/// character fog fade, `ClothInstancing_releaseExpired` (cdecl),
/// `g_LoadingScreen`'s `0x006e50a0`, `BootText_update` (on the
/// instance's `+0x110`) and `TextUtil_animate` (on `0x00ce8260`).
pub const FMV_MANAGER: u32 = 0x006d_73a0;
pub const FMV_PROCESS: u32 = 0x006d_adf0;
pub const BLUR_INSTANCE: u32 = 0x006c_3580;
pub const BLUR_UPDATE: u32 = 0x006c_3d40;
pub const BLUR_EX_INSTANCE: u32 = 0x006c_4a70;
pub const BLUR_EX_UPDATE: u32 = 0x006c_4e00;
pub const FOG_INSTANCE: u32 = 0x006e_32c0;
pub const FOG_UPDATE: u32 = 0x006e_3f40;
pub const CLOTH_RELEASE: u32 = 0x006e_2f70;
pub const LOADING_SCREEN: u32 = 0x00ce_517c;
pub const LOADING_UPDATE: u32 = 0x006e_50a0;
pub const BOOT_TEXT_INSTANCE: u32 = 0x006d_c8a0;
pub const BOOT_TEXT_UPDATE: u32 = 0x006d_e010;
pub const TEXT_UTIL: u32 = 0x00ce_8260;
pub const TEXT_ANIMATE: u32 = 0x006f_69f0;

pub mod bridge {
    //! The originals over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn global(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    /// The application's frame time (`+0x24`) through `fld`/`fstp`.
    fn frame_time(h: &mut impl Host) -> u32 {
        let app = h.call_sym(GET_APPLICATION, Conv::Cdecl, &[]);
        x87::store(x87::load(h.read(app + 0x24)))
    }

    /// `getter()`, then `update(instance; args)`.
    fn on_instance(h: &mut impl Host, getter: u32, sym: u32, args: &[u32]) {
        let instance = h.call_sym(getter, Conv::Cdecl, &[]);
        h.call_sym(sym, Conv::Thiscall(instance), args);
    }

    fn icon(h: &mut impl Host, sym: u32, i: u32) {
        on_instance(h, ICON_PANEL, sym, &[i]);
    }

    fn tip(h: &mut impl Host, id: u32) -> u32 {
        h.call_sym(PUSH_TIP_BY_LOC, Conv::Cdecl, &[id])
    }

    /// `(v + 1) % n` with C's signed remainder.
    fn next(v: u32, n: i32) -> i32 {
        (v as i32).wrapping_add(1).wrapping_rem(n)
    }

    /// The auto-battle key toggles the mode (tip 0x37f on, 0x380 off,
    /// icon 5); the mode drops (tip 0x37d) when auto-battle can't be
    /// used; icon 5 hidden while off.
    fn auto_battle(h: &mut impl Host) {
        if h.call_sym(CHECK_AUTO_BATTLE_KEY, Conv::Cdecl, &[]) != 0 {
            let v = next(global(h, AUTO_BATTLE), 2);
            h.write(h.rebase(AUTO_BATTLE), v);
            if v == 0 {
                icon(h, ICON_HIDE, 5);
                tip(h, 0x380);
            } else {
                icon(h, ICON_SHOW, 5);
                tip(h, 0x37f);
            }
        }
        let can = h.call_sym(CAN_USE_AUTO_BATTLE, Conv::Cdecl, &[]);
        let v = global(h, AUTO_BATTLE);
        if can == 0 && v == 1 {
            icon(h, ICON_HIDE, 5);
            h.write(h.rebase(AUTO_BATTLE), 0u32);
            tip(h, 0x37d);
        } else if v == 0 {
            icon(h, ICON_HIDE, 5);
        }
    }

    /// The invincible key toggles the power-up cheat (tip 0x2f6 on,
    /// 0x2f7 off, which also ends the cheat); kept only when the tip
    /// shows. Icon 2 follows the cheat.
    fn invincible(h: &mut impl Host) {
        let mut on = None;
        if h.call_sym(CHECK_INVINCIBLE_KEY, Conv::Cdecl, &[]) != 0 {
            let v = next(global(h, CHEAT_POWERUP), 2);
            let shown = if v == 0 {
                let shown = tip(h, 0x2f7);
                h.call_sym(POWERFUL_OFF, Conv::Cdecl, &[]);
                shown
            } else {
                tip(h, 0x2f6)
            };
            if shown != 0 {
                h.write(h.rebase(CHEAT_POWERUP), v);
                on = Some(v);
            }
        }
        let on = on.unwrap_or_else(|| global(h, CHEAT_POWERUP) as i32);
        icon(h, if on == 0 { ICON_HIDE } else { ICON_SHOW }, 2);
    }

    /// The encounter key steps the mode (0 to 2; tips 0x2fa, 0x2f9,
    /// 0x2fb), kept only when the tip shows; icons 3 and 4 show it.
    fn encounters(h: &mut impl Host) {
        let mut mode = None;
        if h.call_sym(CHECK_ENCOUNTER_KEY, Conv::Cdecl, &[]) != 0 {
            let old = global(h, ENCOUNT_MODE);
            let m = next(old, 3);
            let id = match m {
                0 => Some(0x2fa),
                1 => Some(0x2f9),
                2 => Some(0x2fb),
                _ => None,
            };
            match id {
                // a negative remainder: the mode read before the step
                None => mode = Some(old as i32),
                Some(id) => {
                    if tip(h, id) != 0 {
                        h.write(h.rebase(ENCOUNT_MODE), m);
                        mode = Some(m);
                    }
                }
            }
        }
        let mode = mode.unwrap_or_else(|| global(h, ENCOUNT_MODE) as i32);
        match mode {
            0 => {
                icon(h, ICON_HIDE, 4);
                icon(h, ICON_SHOW, 3);
            }
            1 => {
                icon(h, ICON_HIDE, 3);
                icon(h, ICON_HIDE, 4);
            }
            2 => {
                icon(h, ICON_HIDE, 3);
                icon(h, ICON_SHOW, 4);
            }
            _ => {}
        }
    }

    /// The free camera: button 0x100000 or (with the debug word) its
    /// state toggles its pause; then its input with the frame time and
    /// the scene's `+0xdc`.
    fn free_camera(h: &mut impl Host, this: u32) {
        if h.call_sym(IS_PRESS_BUTTON, Conv::Cdecl, &[0x10_0000]) != 0 {
            on_instance(h, FREE_CAMERA, FREE_CAMERA_PAUSE, &[]);
        }
        if global(h, DEBUG_WORD) != 0 {
            let camera = h.call_sym(FREE_CAMERA, Conv::Cdecl, &[]);
            if h.read::<u8>(camera + 0x338) != 0
                || h.read::<u8>(camera + 0x33c) != 0
                || h.read::<u32>(camera) == 2
            {
                on_instance(h, FREE_CAMERA, FREE_CAMERA_PAUSE, &[]);
            }
        }
        let time = frame_time(h);
        let word: u32 = h.read(this + 0xdc);
        on_instance(h, FREE_CAMERA, FREE_CAMERA_INPUT, &[time, word]);
    }

    /// `PhyreScene_animate(scene)` (`this` in `ecx`, no stack words)
    /// -> 0.
    ///
    /// With the application's frame time: the delegations, the texture
    /// animations and morphs (unless `+0x1195`), the first-run tip
    /// 0x30a, the lighting flash, a pending pause, the icon panel. On
    /// maps 0x17, 0x15c and 0x15d the cheats reset (icons hidden, the
    /// HUD `+0x10dcc` on); elsewhere the speed control, the auto-battle,
    /// invincible and encounter keys, button 0x80000 (the HUD toggle,
    /// tips 0x2f0/0x2f1) and the free camera. Then the movie requests,
    /// blurs, fog, cloth, delegations, loading screen, boot text and
    /// text animation.
    pub fn animate(h: &mut impl Host, this: u32) -> u32 {
        let time = frame_time(h);
        let helper = global(h, DELEGATION_HELPER);
        h.call_sym(DELEGATION_UPDATE, Conv::Thiscall(helper), &[]);
        if h.read::<u8>(this + 0x1195) == 0 {
            h.call_sym(TEX_ANIM_ANIMATE, Conv::Thiscall(this), &[time]);
            on_instance(h, MORPH_INSTANCE, MORPH_UPDATE, &[time]);
        }
        if h.read::<u8>(global(h, SETTINGS) + 0x638) == 0 {
            on_instance(h, TIP_SYSTEM, PUSH_TIP, &[0x30a]);
            let settings = global(h, SETTINGS);
            h.write(settings + 0x638, 1u8);
        }
        on_instance(h, LIGHTING_INSTANCE, LIGHTING_UPDATE_FLASH, &[time]);
        let request = global(h, PAUSE_REQUEST);
        if request != 0 {
            h.call_sym(PAUSE_UPDATE, Conv::Cdecl, &[request]);
        }
        on_instance(h, ICON_PANEL, ICON_UPDATE, &[]);
        if matches!(global(h, CURRENT_MAP), 0x17 | 0x15c | 0x15d) {
            h.call_sym(SPD_INIT, Conv::Cdecl, &[]);
            h.write(h.rebase(CHEAT_POWERUP), 0u32);
            h.write(h.rebase(ENCOUNT_MODE), 1u32);
            h.write(h.rebase(AUTO_BATTLE), 0u32);
            h.write(this + 0x10dcc, 1u8);
            on_instance(h, ICON_PANEL, ICON_HIDE_ALL, &[]);
            icon(h, ICON_SET_VISIBLE, 1);
        } else {
            h.call_sym(SPD_TICK, Conv::Cdecl, &[]);
            auto_battle(h);
            invincible(h);
            encounters(h);
            if h.call_sym(IS_PRESS_BUTTON, Conv::Cdecl, &[0x8_0000]) != 0 {
                let on = h.read::<u8>(this + 0x10dcc) == 0;
                h.write(this + 0x10dcc, u8::from(on));
                on_instance(h, TIP_SYSTEM, PUSH_TIP, &[0x2f0 + u32::from(on)]);
                let on = u32::from(h.read::<u8>(this + 0x10dcc));
                icon(h, ICON_SET_VISIBLE, on);
            }
            free_camera(h, this);
        }
        on_instance(h, FMV_MANAGER, FMV_PROCESS, &[]);
        on_instance(h, BLUR_INSTANCE, BLUR_UPDATE, &[time]);
        on_instance(h, BLUR_EX_INSTANCE, BLUR_EX_UPDATE, &[time]);
        on_instance(h, FOG_INSTANCE, FOG_UPDATE, &[time]);
        h.call_sym(CLOTH_RELEASE, Conv::Cdecl, &[]);
        let helper = global(h, DELEGATION_HELPER);
        h.call_sym(DELEGATION_RUN, Conv::Thiscall(helper), &[]);
        let screen = global(h, LOADING_SCREEN);
        h.call_sym(LOADING_UPDATE, Conv::Thiscall(screen), &[time]);
        let text = h.call_sym(BOOT_TEXT_INSTANCE, Conv::Cdecl, &[]);
        h.call_sym(BOOT_TEXT_UPDATE, Conv::Thiscall(text + 0x110), &[time]);
        let util = h.rebase(TEXT_UTIL);
        h.call_sym(TEXT_ANIMATE, Conv::Thiscall(util), &[time]);
        0
    }
}
