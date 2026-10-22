//! The character object's accessors (`Ch_Get*`, `Ch_Set*`, `Ch_En*`): the
//! fields and flags of one `g_ChrArray` element ([`crate::layout`]).
//!
//! The core is the little logic they have (flag bits, defaults for
//! out-of-range values, fades, the angle wrap); [`bridge`] reads and writes
//! the fields where the game keeps them. All the originals are cdecl with
//! the character first.

use crate::x87;

/// A value moving toward a target over some frames (12 bytes: current,
/// target, frames). Alpha, shade and colours fade this way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fade {
    /// The value now.
    pub current: f32,
    /// Where it goes.
    pub target: f32,
    /// Frames to get there; 0 = at once.
    pub frames: i32,
}

impl Fade {
    /// A new target (`Chr_FadeSet`): reached at once when `frames` is 0.
    pub fn set(&mut self, target: f32, frames: i32) {
        self.target = target;
        self.frames = frames;
        if frames == 0 {
            self.current = target;
        }
    }
}

/// `word` with `bit` set to bit 0 of `on`, as the `En`/`Set` flag setters
/// do (`((on << n) ^ word) & bit ^ word`). Also returns the bit that
/// changed (what is left in `eax`).
#[must_use]
pub const fn put_bit(word: u32, bit: u32, on: u32) -> (u32, u32) {
    let diff = (on.wrapping_mul(bit) ^ word) & bit;
    (word ^ diff, diff)
}

/// `word` with `bit` set to the inverse of bit 0 of `on`
/// (`Ch_EnBodyHit`, `Ch_EnMapID`: `(-(on << n) - 1 ^ word) & bit ^ word`).
#[must_use]
pub const fn put_bit_inverted(word: u32, bit: u32, on: u32) -> u32 {
    word ^ ((!on.wrapping_mul(bit) ^ word) & bit)
}

/// `Chr_WrapAngle(a)` (`0x0082ba70`), `Ch_GetRot`'s wrap: steps of 6.283184 until the angle is
/// within -3.141592 ..= 3.141592 (the `double` widenings of those `f32`s),
/// each step rounded to `f32`. Like the original it never ends for
/// infinities or values too large for a step to change them.
#[must_use]
pub fn wrap_angle(a: f32) -> f32 {
    let hi = f64::from_bits(0x4009_21fb_0000_0000); // 3.141592f
    let step = f64::from_bits(0x4019_21fb_0000_0000); // 6.283184f
    let lo = f64::from_bits(0xc009_21fb_0000_0000); // -3.141592f
    let mut a = x87::load(a);
    #[allow(clippy::cast_possible_truncation)] // each step is stored as f32
    {
        if f64::from(a) > hi {
            loop {
                a = (f64::from(a) - step) as f32;
                if f64::from(a) <= hi || a.is_nan() {
                    break;
                }
            }
        }
        if f64::from(a) < lo {
            loop {
                a = (f64::from(a) + step) as f32;
                if f64::from(a) >= lo || a.is_nan() {
                    break;
                }
            }
        }
    }
    a
}

/// `Ch_MoveSetChgRun`'s new value: 18 unless `v` is above 0 (NaN too).
#[must_use]
pub fn chg_run(v: f32) -> f32 {
    if v > 0.0 { x87::load(v) } else { 18.0 }
}

/// `Ch_SetCamHeight`, `Ch_SetFixHeight`, `Ch_SetThicknessBtlCam`: `v`, or
/// the default when `v` is negative (0 and NaN stay).
#[must_use]
pub fn or_default_if_negative(v: f32, default: f32) -> f32 {
    x87::load(if v < 0.0 { default } else { v })
}

/// `Ch_SetThickness`: `v`, or the default when `v` is 0 or less (NaN
/// stays).
#[must_use]
pub fn or_default_if_not_positive(v: f32, default: f32) -> f32 {
    x87::load(if v <= 0.0 { default } else { v })
}

/// `Ch_MotionSetNextHokan`'s stored frames: -1 is 0, 0 is 1.
#[must_use]
pub const fn next_hokan(frames: i32) -> u16 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a u16 field
    match frames {
        -1 => 0,
        0 => 1,
        n => n as u16,
    }
}

/// `Chr_SetHideFlags`' new hide byte and its result: whether any `mask`
/// bit was set (the byte is sign-extended before the test).
#[must_use]
pub const fn hide_flags(hide: u8, on: bool, mask: u32) -> (u8, u32) {
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)] // movsx
    let was = ((hide as i8 as i32 as u32) & mask != 0) as u32;
    #[allow(clippy::cast_possible_truncation)] // the low byte of the mask
    let m = mask as u8;
    (if on { hide | m } else { hide & !m }, was)
}

/// The accessors where the game keeps the characters, through
/// [`ffx_bridge::Mem`]. `chr` is the address of a `g_ChrArray` element.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{
        Fade, chg_run, hide_flags, next_hokan, or_default_if_negative, or_default_if_not_positive,
        put_bit, put_bit_inverted, wrap_angle,
    };
    use crate::layout as l;
    use crate::x87;

    /// The default motions by slot, for `Ch_MotionSetMoves(chr, slot, -1)`.
    pub const DEFAULT_MOVES: Global<u32> = Global::new(0x00c4_9680);

    pub(crate) fn f(mem: &impl Mem, at: u32) -> f32 {
        mem.read(at)
    }

    /// A float as the original loads it into `st0` and stores it.
    pub(crate) fn put_f(mem: &mut impl Mem, at: u32, v: f32) {
        mem.write(at, x87::load(v));
    }

    pub(crate) fn blk(mem: &impl Mem, chr: u32) -> u32 {
        mem.read(chr + l::BLK)
    }

    fn model(mem: &impl Mem, chr: u32, index: u32) -> u32 {
        let models: u32 = mem.read(chr + l::MODELS);
        models.wrapping_add(index.wrapping_mul(l::MODEL_SIZE))
    }

    pub(crate) fn flag(mem: &impl Mem, chr: u32, word: u32, bit: u32) -> u32 {
        u32::from(mem.read::<u32>(chr + word) & bit != 0)
    }

    /// Sets a flag; the bit that changed.
    pub(crate) fn set_flag(mem: &mut impl Mem, chr: u32, word: u32, bit: u32, on: u32) -> u32 {
        let (w, diff) = put_bit(mem.read(chr + word), bit, on);
        mem.write(chr + word, w);
        diff
    }

    /// Sets a flag; what it was.
    pub(crate) fn en_flag(mem: &mut impl Mem, chr: u32, word: u32, bit: u32, on: u32) -> u32 {
        let was = flag(mem, chr, word, bit);
        set_flag(mem, chr, word, bit, on);
        was
    }

    // ---- getters

    /// `Ch_EffectGetEffectLevel(chr)` (`0x0082a9a0`).
    pub fn ch_effect_get_effect_level(mem: &impl Mem, chr: u32) -> u32 {
        mem.read(chr + l::EFFECT_LEVEL)
    }

    /// `Ch_EffectGetResidentPtr(chr)` (`0x0082a9b0`): the model block's
    /// resident data.
    pub fn ch_effect_get_resident_ptr(mem: &impl Mem, chr: u32) -> u32 {
        mem.read(blk(mem, chr) + l::BLK_RESIDENT)
    }

    /// `Ch_GetBodyHit(chr)` (`0x0082ab10`): 1 when other characters
    /// collide with it.
    pub fn ch_get_body_hit(mem: &impl Mem, chr: u32) -> u32 {
        1 - flag(mem, chr, l::FLAGS, l::F_NO_BODY_HIT)
    }

    /// `Ch_GetCamHeight(chr)` (`0x0082ab30`).
    pub fn ch_get_cam_height(mem: &impl Mem, chr: u32) -> f32 {
        f(mem, chr + l::CAM_HEIGHT)
    }

    /// `Ch_GetClip(chr)` (`0x0082ab40`).
    pub fn ch_get_clip(mem: &impl Mem, chr: u32) -> u32 {
        flag(mem, chr, l::FLAGS, l::F_CLIP)
    }

    /// `Ch_GetDataScale(chr)` (`0x0082ab60`): the model block's scale.
    pub fn ch_get_data_scale(mem: &impl Mem, chr: u32) -> f32 {
        f(mem, blk(mem, chr) + l::BLK_DATA_SCALE)
    }

    /// `Ch_GetDir(chr)` (`0x0082ab80`).
    pub fn ch_get_dir(mem: &impl Mem, chr: u32) -> f32 {
        f(mem, chr + l::DIR)
    }

    /// `Ch_GetFieldMode(chr)` (`0x0082ab90`), sign-extended.
    pub fn ch_get_field_mode(mem: &impl Mem, chr: u32) -> i32 {
        i32::from(mem.read::<i8>(chr + l::FIELD_MODE))
    }

    /// `Ch_GetFixHeight(chr)` (`0x0082aba0`).
    pub fn ch_get_fix_height(mem: &impl Mem, chr: u32) -> f32 {
        f(mem, chr + l::FIX_HEIGHT)
    }

    /// `Ch_GetGrav(chr)` (`0x0082abb0`).
    pub fn ch_get_grav(mem: &impl Mem, chr: u32) -> f32 {
        f(mem, chr + l::GRAV)
    }

    /// `Ch_GetGravMode(chr)` (`0x0082abc0`), sign-extended.
    pub fn ch_get_grav_mode(mem: &impl Mem, chr: u32) -> i32 {
        i32::from(mem.read::<i8>(chr + l::GRAV_MODE))
    }

    /// `Ch_GetHide(chr)` (`0x0082abd0`) and `Ch_GetResHide(chr)`
    /// (`0x0082acd0`): the hide byte, sign-extended.
    pub fn ch_get_hide(mem: &impl Mem, chr: u32) -> i32 {
        i32::from(mem.read::<i8>(chr + l::HIDE))
    }

    /// `Ch_GetHideCalc(chr)` (`0x0082abe0`): 1 when the calculation pass
    /// skips the character: hidden from it, flagged, or at alpha 0.
    pub fn ch_get_hide_calc(mem: &impl Mem, chr: u32) -> u32 {
        let alpha = f(mem, chr + l::FADE_ALPHA);
        u32::from(
            mem.read::<u8>(chr + l::HIDE_CALC) != 0
                || flag(mem, chr, l::FLAGS, l::F_DRAW_EFFECT) != 0
                || alpha == 0.0,
        )
    }

    /// `Ch_GetHideEvent(chr)` (`0x0082ac20`).
    pub fn ch_get_hide_event(mem: &impl Mem, chr: u32) -> u32 {
        u32::from(mem.read::<u8>(chr + l::HIDE) & l::HIDE_EVENT)
    }

    /// `Ch_GetJimen(chr)` (`0x0082ac40`).
    pub fn ch_get_jimen(mem: &impl Mem, chr: u32) -> f32 {
        f(mem, chr + l::JIMEN)
    }

    /// `Ch_GetMotionSpeed(chr)` (`0x0082ac50`), sign-extended.
    pub fn ch_get_motion_speed(mem: &impl Mem, chr: u32) -> i32 {
        i32::from(mem.read::<i16>(chr + l::MOTION_SPEED))
    }

    /// `Ch_GetOrgMatrix(chr)` (`0x0082ac60`).
    pub fn ch_get_org_matrix(mem: &impl Mem, chr: u32) -> u32 {
        mem.read(chr + l::ORG_MATRIX)
    }

    /// `Ch_GetOverlapHit(chr)` (`0x0082ac70`).
    pub fn ch_get_overlap_hit(mem: &impl Mem, chr: u32) -> u32 {
        flag(mem, chr, l::FLAGS, l::F_OVERLAP_HIT)
    }

    /// Copies three floats to optional outputs (`Ch_GetPos`, `Ch_GetScale`).
    fn get3(mem: &mut impl Mem, from: u32, out: [u32; 3]) {
        for (i, o) in (0u32..).zip(out) {
            if o != 0 {
                let v = f(mem, from + 4 * i);
                put_f(mem, o, v);
            }
        }
    }

    /// `Ch_GetPos(chr, &x, &y, &z)` (`0x0082ac90`); null outputs are
    /// skipped.
    pub fn ch_get_pos(mem: &mut impl Mem, chr: u32, x: u32, y: u32, z: u32) {
        get3(mem, chr + l::POS, [x, y, z]);
    }

    /// `Ch_GetRot(chr)` (`0x0082ace0`): the rotation wrapped to ±π
    /// ([`wrap_angle`]).
    pub fn ch_get_rot(mem: &impl Mem, chr: u32) -> f32 {
        wrap_angle(f(mem, chr + l::ROT))
    }

    /// `Ch_GetScale(chr, &x, &y, &z)` (`0x0082ad00`); null outputs are
    /// skipped.
    pub fn ch_get_scale(mem: &mut impl Mem, chr: u32, x: u32, y: u32, z: u32) {
        get3(mem, chr + l::SCALE, [x, y, z]);
    }

    /// `Ch_GetScaleX/Y/Z(chr)` (`0x0082ad30`, `0x0082ad40`, `0x0082ad50`).
    pub fn ch_get_scale_axis(mem: &impl Mem, chr: u32, axis: u32) -> f32 {
        f(mem, chr + l::SCALE + 4 * axis)
    }

    /// `Ch_GetSp(chr)` (`0x0082ad60`).
    pub fn ch_get_sp(mem: &impl Mem, chr: u32) -> f32 {
        f(mem, chr + l::SP)
    }

    /// `Ch_GetThickness(chr)` (`0x0082ad70`): the model block's.
    pub fn ch_get_thickness(mem: &impl Mem, chr: u32) -> f32 {
        f(mem, blk(mem, chr) + l::BLK_THICKNESS)
    }

    /// `Ch_GetThicknessBtl(chr)` (`0x0082ad90`): the model block's.
    pub fn ch_get_thickness_btl(mem: &impl Mem, chr: u32) -> f32 {
        f(mem, blk(mem, chr) + l::BLK_THICKNESS_BTL)
    }

    /// `Ch_GetThicknessBtlCam(chr)` (`0x0082adb0`).
    pub fn ch_get_thickness_btl_cam(mem: &impl Mem, chr: u32) -> f32 {
        f(mem, chr + l::THICKNESS_BTL_CAM)
    }

    /// `Ch_IdGetPNum(chr)` (`0x0082adc0`), sign-extended.
    pub fn ch_id_get_p_num(mem: &impl Mem, chr: u32) -> i32 {
        i32::from(mem.read::<i16>(chr + l::PNUM))
    }

    /// `Ch_ModelGetHide(chr, model)` (`0x0082add0`), sign-extended; the
    /// index is not checked.
    pub fn ch_model_get_hide(mem: &impl Mem, chr: u32, index: u32) -> i32 {
        i32::from(mem.read::<i8>(model(mem, chr, index) + l::MODEL_HIDE))
    }

    /// `Ch_ModelGetNromPtr(chr, model)` (`0x0082adf0`).
    pub fn ch_model_get_nrom_ptr(mem: &impl Mem, chr: u32, index: u32) -> u32 {
        mem.read(model(mem, chr, index) + l::MODEL_NROM)
    }

    /// `Ch_ModelGetVtexPtr(chr, model)` (`0x0082ae10`).
    pub fn ch_model_get_vtex_ptr(mem: &impl Mem, chr: u32, index: u32) -> u32 {
        mem.read(model(mem, chr, index) + l::MODEL_VTEX)
    }

    /// `Ch_TextureGetAnimId(chr)` (`0x0082b960`).
    pub fn ch_texture_get_anim_id(mem: &impl Mem, chr: u32) -> u32 {
        mem.read(chr + l::TEXTURE_ANIM_ID)
    }

    // ---- flags

    /// `Ch_EnBodyHit(chr, on)` (`0x0082a9f0`): on (bit 0) lets other
    /// characters collide; returns the old "no body hit" bit.
    pub fn ch_en_body_hit(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        let w: u32 = mem.read(chr + l::FLAGS);
        mem.write(chr + l::FLAGS, put_bit_inverted(w, l::F_NO_BODY_HIT, on));
        u32::from(w & l::F_NO_BODY_HIT != 0)
    }

    /// `Ch_EnClip(chr, on)` (`0x0082aa30`): returns the old bit.
    pub fn ch_en_clip(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        en_flag(mem, chr, l::FLAGS, l::F_CLIP, on)
    }

    /// `Ch_EnMapID(chr, on)` (`0x0082aa70`): on (bit 0) keeps the
    /// character on the ground; off resets its normal to (0, -1, 0). Then
    /// `Ch_IdEnableShade(chr, on)` (a tail call: its result).
    pub fn ch_en_map_id(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        let w: u32 = mem.read(chr + l::FLAGS);
        let w = put_bit_inverted(w, l::F_MAP_ID, on);
        mem.write(chr + l::FLAGS, w);
        let off = w & l::F_MAP_ID != 0;
        if off {
            mem.write(chr + l::NORMAL, 0.0f32);
            mem.write(chr + l::NORMAL + 4, -1.0f32);
            mem.write(chr + l::NORMAL + 8, 0.0f32);
        }
        ch_id_enable_shade(mem, chr, u32::from(!off))
    }

    /// `Ch_EnRotHokan(chr, on)` (`0x0082aae0`): returns the old bit.
    pub fn ch_en_rot_hokan(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        en_flag(mem, chr, l::FLAGS, l::F_ROT_HOKAN, on)
    }

    /// `Ch_IdEnableShade(chr, on)` (`0x008340f0`): returns the bit that
    /// changed (`0x40000` or 0).
    pub fn ch_id_enable_shade(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        set_flag(mem, chr, l::FLAGS, l::F_ID_SHADE, on)
    }

    /// `Ch_SetChocobo(chr, on)` (`0x0082b130`): returns the bit that
    /// changed.
    pub fn ch_set_chocobo(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        set_flag(mem, chr, l::FLAGS2, l::F2_CHOCOBO, on)
    }

    /// `Ch_SetOverlapHit(chr, on)` (`0x0082b420`): returns the bit that
    /// changed.
    pub fn ch_set_overlap_hit(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        set_flag(mem, chr, l::FLAGS, l::F_OVERLAP_HIT, on)
    }

    /// `Ch_Vu1SetClipmode(chr, on)` (`0x0082b970`): returns the bit that
    /// changed.
    pub fn ch_vu1_set_clipmode(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        set_flag(mem, chr, l::FLAGS2, l::F2_VU1_CLIP, on)
    }

    /// `Chr_SetHideFlags(chr, on, mask)` (`0x0082ba30`): sets or clears the
    /// `mask` hide bits (clearing marks the character dirty); 1 when any
    /// was set.
    pub fn chr_set_hide_flags(mem: &mut impl Mem, chr: u32, on: u32, mask: u32) -> u32 {
        let (hide, was) = hide_flags(mem.read(chr + l::HIDE), on != 0, mask);
        if on == 0 {
            let d: u32 = mem.read(chr + l::DIRTY);
            mem.write(chr + l::DIRTY, d | 4);
        }
        mem.write(chr + l::HIDE, hide);
        was
    }

    /// `Ch_SetHideBattle(chr, on)` (`0x0082b260`).
    pub fn ch_set_hide_battle(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        chr_set_hide_flags(mem, chr, on, u32::from(l::HIDE_BATTLE))
    }

    /// `Ch_SetHideEffect(chr, on)` (`0x0082b2a0`).
    pub fn ch_set_hide_effect(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        chr_set_hide_flags(mem, chr, on, u32::from(l::HIDE_EFFECT))
    }

    // ---- setters

    /// `Ch_EffectSetEffectLevel(chr, level)` (`0x0082a9d0`).
    pub fn ch_effect_set_effect_level(mem: &mut impl Mem, chr: u32, level: u32) {
        mem.write(chr + l::EFFECT_LEVEL, level);
    }

    /// `Ch_AddGrav(chr, g)` (`0x0082a890`).
    pub fn ch_add_grav(mem: &mut impl Mem, chr: u32, g: f32) {
        let v = x87::add(g, f(mem, chr + l::GRAV));
        mem.write(chr + l::GRAV, v);
    }

    /// `Ch_MotionSetHokan(chr, frames)` (`0x0082afb0`); 0 marks the
    /// character dirty.
    pub fn ch_motion_set_hokan(mem: &mut impl Mem, chr: u32, frames: u32) {
        let w: u32 = mem.read(chr + l::FLAGS);
        mem.write(chr + l::FLAGS, w & !l::F_HOKAN_16);
        #[allow(clippy::cast_possible_truncation)] // a u16 field
        mem.write(chr + l::HOKAN, frames as u16);
        if frames == 0 {
            let d: u32 = mem.read(chr + l::DIRTY);
            mem.write(chr + l::DIRTY, d | 4);
        }
    }

    /// `Ch_MotionSetMoves(chr, slot, motion)` (`0x0082afe0`): -1 takes the
    /// slot's default ([`DEFAULT_MOVES`]); the slot is not checked.
    pub fn ch_motion_set_moves(mem: &mut impl Mem, chr: u32, slot: u32, motion: u32) {
        let v = if motion == u32::MAX {
            DEFAULT_MOVES.ptr(mem).at(slot).get(mem)
        } else {
            motion
        };
        mem.write(
            chr.wrapping_add(l::MOVES)
                .wrapping_add(slot.wrapping_mul(4)),
            v,
        );
    }

    /// `Ch_MotionSetNextHokan(chr, frames)` (`0x0082b010`).
    pub fn ch_motion_set_next_hokan(mem: &mut impl Mem, chr: u32, frames: i32) {
        mem.write(chr + l::NEXT_HOKAN, next_hokan(frames));
    }

    /// `Ch_MoveSetChgRun(chr, speed)` (`0x0082b040`): returns the old
    /// value.
    pub fn ch_move_set_chg_run(mem: &mut impl Mem, chr: u32, speed: f32) -> f32 {
        let old = x87::load(f(mem, chr + l::CHG_RUN));
        mem.write(chr + l::CHG_RUN, chg_run(speed));
        old
    }

    /// `Ch_ResetMotion(chr)` (`0x0082b080`): before every new motion.
    pub fn ch_reset_motion(mem: &mut impl Mem, chr: u32) {
        mem.write(chr + l::MOTION_730, 1u32);
        mem.write(chr + l::MOTION_SCRIPTED, 0u8);
        mem.write(chr + l::MOTION_728, 0u16);
    }

    /// `Ch_ResetPos(chr)` (`0x0082b0b0`).
    pub fn ch_reset_pos(mem: &mut impl Mem, chr: u32) {
        mem.write(chr + l::PNUM, -1i16);
    }

    /// `Ch_SetBattleChrNum(chr, n)` (`0x0082b0d0`).
    pub fn ch_set_battle_chr_num(mem: &mut impl Mem, chr: u32, n: u32) {
        mem.write(chr + l::BATTLE_CHR_NUM, n);
    }

    /// `Ch_SetCamHeight(chr, h)` (`0x0082b0f0`): negative takes the model
    /// block's.
    pub fn ch_set_cam_height(mem: &mut impl Mem, chr: u32, h: f32) {
        let d = f(mem, blk(mem, chr) + l::BLK_CAM_HEIGHT);
        mem.write(chr + l::CAM_HEIGHT, or_default_if_negative(h, d));
    }

    /// `Ch_SetFixHeight(chr, h)` (`0x0082b200`): negative takes the model
    /// block's.
    pub fn ch_set_fix_height(mem: &mut impl Mem, chr: u32, h: f32) {
        let d = f(mem, blk(mem, chr) + l::BLK_FIX_HEIGHT);
        mem.write(chr + l::FIX_HEIGHT, or_default_if_negative(h, d));
    }

    /// `Ch_SetThicknessBtlCam(chr, t)` (`0x0082b8c0`): negative takes the
    /// model block's.
    pub fn ch_set_thickness_btl_cam(mem: &mut impl Mem, chr: u32, t: f32) {
        let d = f(mem, blk(mem, chr) + l::BLK_THICKNESS_BTL_CAM);
        mem.write(chr + l::THICKNESS_BTL_CAM, or_default_if_negative(t, d));
    }

    /// `Ch_SetThickness(chr, t)` (`0x0082b880`): 0 or less takes the model
    /// block's; returns the old value.
    pub fn ch_set_thickness(mem: &mut impl Mem, chr: u32, t: f32) -> f32 {
        let old = x87::load(f(mem, chr + l::THICKNESS));
        let d = f(mem, blk(mem, chr) + l::BLK_THICKNESS);
        mem.write(chr + l::THICKNESS, or_default_if_not_positive(t, d));
        old
    }

    /// `Ch_SetWeight(chr, w)` (`0x0082b940`): returns the old weight.
    pub fn ch_set_weight(mem: &mut impl Mem, chr: u32, w: f32) -> f32 {
        let old = x87::load(f(mem, chr + l::WEIGHT));
        put_f(mem, chr + l::WEIGHT, w);
        old
    }

    /// A float field set as is: `Ch_SetDir` (`0x0082b190`, [`l::DIR`]),
    /// `Ch_SetHitHeight` (`0x0082b320`, [`l::HIT_HEIGHT`]), `Ch_SetHiju`
    /// (`0x0082b340`, [`l::HIJU`]), `Ch_SetJimen` (`0x0082b360`,
    /// [`l::JIMEN`]), `Ch_SetOrgMatrixScale` (`0x0082b400`,
    /// [`l::ORG_MATRIX_SCALE`]), `Ch_SetSp` (`0x0082b840`, [`l::SP`]).
    pub fn ch_set_f32(mem: &mut impl Mem, chr: u32, field: u32, v: f32) {
        put_f(mem, chr + field, v);
    }

    /// A byte field: `Ch_SetFieldMode` (`0x0082b1e0`), `Ch_SetGravMode`
    /// (`0x0082b240`), `Ch_SetHideCalc` (`0x0082b280`), `Ch_SetMotionType`
    /// (`0x0082b3a0`).
    pub fn ch_set_u8(mem: &mut impl Mem, chr: u32, field: u32, v: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(chr + field, v as u8);
    }

    /// `Ch_SetMotionSpeed(chr, speed)` (`0x0082b380`): 8.8 fixed.
    pub fn ch_set_motion_speed(mem: &mut impl Mem, chr: u32, speed: u32) {
        #[allow(clippy::cast_possible_truncation)] // a 16-bit field
        mem.write(chr + l::MOTION_SPEED, speed as u16);
    }

    /// `Ch_SetOrgMatrix(chr, matrix)` (`0x0082b3e0`).
    pub fn ch_set_org_matrix(mem: &mut impl Mem, chr: u32, matrix: u32) {
        mem.write(chr + l::ORG_MATRIX, matrix);
    }

    /// `Ch_SetOffset(chr, x, y, z)` (`0x0082b3c0`).
    pub fn ch_set_offset(mem: &mut impl Mem, chr: u32, x: f32, y: f32, z: f32) {
        put_f(mem, chr + l::OFFSET, x);
        put_f(mem, chr + l::OFFSET + 4, y);
        put_f(mem, chr + l::OFFSET + 8, z);
    }

    /// After a new position: the copy, the flag, [`l::PNUM`] = -1.
    fn pos_set(mem: &mut impl Mem, chr: u32) {
        let p: [u32; 4] = mem.read(chr + l::POS);
        mem.write(chr + l::POS_NEXT, p);
        let w: u32 = mem.read(chr + l::FLAGS);
        mem.write(chr + l::FLAGS, w | l::F_POS_SET);
        mem.write(chr + l::PNUM, -1i16);
    }

    /// `Ch_SetPos(chr, x, z)` (`0x0082b440`): y stays.
    pub fn ch_set_pos(mem: &mut impl Mem, chr: u32, x: f32, z: f32) {
        put_f(mem, chr + l::POS, x);
        put_f(mem, chr + l::POS + 8, z);
        pos_set(mem, chr);
    }

    /// Set by `Ch_GetPosSg`: the next `Ch_SetPos3` only clears it.
    pub const SKIP_SET_POS3: Global<u8> = Global::new(0x012f_fad8);

    /// `Ch_SetPos3(chr, x, y, z)` (`0x0082b480`).
    pub fn ch_set_pos3(mem: &mut impl Mem, chr: u32, x: f32, y: f32, z: f32) {
        if SKIP_SET_POS3.get(mem) != 0 {
            SKIP_SET_POS3.set(mem, 0);
            return;
        }
        put_f(mem, chr + l::POS, x);
        put_f(mem, chr + l::POS + 4, y);
        put_f(mem, chr + l::POS + 8, z);
        pos_set(mem, chr);
    }

    /// `Ch_SetScale(chr, axes, x, y, z)` (`0x0082b560`): the axes whose
    /// bit (1, 2, 4) is set.
    pub fn ch_set_scale(mem: &mut impl Mem, chr: u32, axes: u32, x: f32, y: f32, z: f32) {
        for (i, v) in (0u32..).zip([x, y, z]) {
            if axes & (1 << i) != 0 {
                put_f(mem, chr + l::SCALE + 4 * i, v);
            }
        }
    }

    /// `Ch_SetScaleAll(chr, s)` (`0x0082b590`).
    pub fn ch_set_scale_all(mem: &mut impl Mem, chr: u32, s: f32) {
        ch_set_scale(mem, chr, 7, s, s, s);
    }

    /// `Ch_SetShadowMode(chr, mode)` (`0x0082b820`): returns the old mode,
    /// sign-extended.
    pub fn ch_set_shadow_mode(mem: &mut impl Mem, chr: u32, mode: u32) -> i32 {
        let old = i32::from(mem.read::<i8>(chr + l::SHADOW_MODE));
        ch_set_u8(mem, chr, l::SHADOW_MODE, mode);
        old
    }

    /// `Ch_SetVelY(chr, v)` (`0x0082b920`): stored × 0.05 (the `double`
    /// widening of 0.05f).
    pub fn ch_set_vel_y(mem: &mut impl Mem, chr: u32, v: f32) {
        let c = f64::from_bits(0x3fa9_9999_a000_0000);
        mem.write(chr + l::VEL_Y, x87::mul_f64(v, c));
    }

    fn fade(mem: &impl Mem, at: u32) -> Fade {
        Fade {
            current: mem.read(at),
            target: mem.read(at + 4),
            frames: mem.read(at + 8),
        }
    }

    /// `Chr_FadeSet(fade, target, frames)` (`0x0082b990`): a new target for
    /// the fade at `at`.
    pub fn chr_fade_set(mem: &mut impl Mem, at: u32, target: f32, frames: i32) {
        let mut fd = fade(mem, at);
        fd.set(x87::load(target), frames);
        mem.write(at + 4, fd.target);
        mem.write(at + 8, fd.frames);
        if frames == 0 {
            mem.write(at, fd.current);
        }
    }

    /// [`chr_fade_set`] on the fade at `chr + field`: `Ch_SetTransparent`
    /// (`0x0082b900`, [`l::FADE_ALPHA`]), `Ch_SetShade` (`0x0082b5b0`,
    /// [`l::FADE_SHADE`]), `Ch_SetSpecularAlpha` (`0x0082b860`,
    /// [`l::FADE_SPECULAR`]).
    pub fn ch_set_fade(mem: &mut impl Mem, chr: u32, field: u32, target: f32, frames: i32) {
        chr_fade_set(mem, chr + field, target, frames);
    }

    /// `Ch_SetShadeColEffect(chr, r, g, b)` (`0x0082b6e0`): the colour
    /// multipliers and the colour fades' targets.
    pub fn ch_set_shade_col_effect(mem: &mut impl Mem, chr: u32, r: f32, g: f32, b: f32) {
        for (i, v) in (0u32..).zip([r, g, b]) {
            let v = x87::load(v);
            mem.write(chr + l::SHADE_COL_EFFECT + 4 * i, v);
            mem.write(chr + l::FADE_SHADE_COL + 12 * i + 4, v);
        }
    }

    /// `Ch_WindGet(chr, &a, &b, &c)` (`0x00824c30`): the wind's `+0x80c`,
    /// `+0x804` and `+0x808` floats (through the x87); null outputs are
    /// skipped.
    pub fn ch_wind_get(mem: &mut impl Mem, chr: u32, a: u32, b: u32, c: u32) {
        for (out, at) in [(b, 0x804), (c, 0x808), (a, 0x80c)] {
            if out != 0 {
                let v = crate::x87::load(f(mem, chr + at));
                mem.write(out, v);
            }
        }
    }

    /// `Ch_WindGetEnable(chr)` (`0x00824c70`): bit 1 of [`l::FLAGS2`].
    pub fn ch_wind_get_enable(mem: &impl Mem, chr: u32) -> u32 {
        flag(mem, chr, l::FLAGS2, 1 << 1)
    }

    /// `Ch_Set7f8(chr, a, b)` (`0x00833940`): the words at `+0x7f8` and
    /// `+0x7fc`.
    pub fn ch_set_7f8(mem: &mut impl Mem, chr: u32, a: u32, b: u32) {
        mem.write(chr + 0x7f8, [a, b]);
    }

    /// `Chr_GetU16(p)` (`0x0082ba20`): `*p` in `ax`; the rest of `eax`
    /// keeps `p`'s high half.
    pub fn chr_get_u16(mem: &impl Mem, p: u32) -> u32 {
        (p & 0xffff_0000) | u32::from(mem.read::<u16>(p))
    }

    /// `Ch_CalcHeight(chr)` (`0x00830b20`): `+0x70` = −(the highest
    /// vertex y of all models / 1000).
    pub fn ch_calc_height(mem: &mut impl Mem, chr: u32) {
        let mut top = 0.0f32;
        let header: u32 = mem.read(chr + l::MODEL_HEADER);
        let n = u32::from(mem.read::<u16>(header + l::HEADER_MODELS));
        let models: u32 = mem.read(chr + l::MODELS);
        for i in 0..n {
            let m = models.wrapping_add(i * l::MODEL_SIZE);
            let data: u32 = mem.read(m);
            let count = u32::from(mem.read::<u16>(data + 4));
            let verts: u32 = mem.read(m + 8);
            for k in 0..count {
                let y = f(mem, verts.wrapping_add(16 * k + 4));
                if top < y {
                    top = crate::x87::load(y);
                }
            }
        }
        let h = crate::x87::Reg::load(top)
            .div_st(crate::x87::Reg::constant(1000.0))
            .store();
        put_f(mem, chr + 0x70, -h);
    }

    /// `Ch_WindSetEnable(chr, on)` (`0x00824cc0`): bit 1 of
    /// [`l::FLAGS2`] from bit 0 of `on`.
    pub fn ch_wind_set_enable(mem: &mut impl Mem, chr: u32, on: u32) {
        let w: u32 = mem.read(chr + l::FLAGS2);
        mem.write(chr + l::FLAGS2, w ^ ((on.wrapping_mul(2) ^ w) & 2));
    }

    /// `Ch_WindSet(chr, a, b, c)` (`0x00824c90`): the wind (`+0x80c` = a,
    /// `+0x804` = b, `+0x808` = c), then `Ch_WindSetEnable(chr, 1)`.
    pub fn ch_wind_set(mem: &mut impl Mem, chr: u32, a: f32, b: f32, c: f32) {
        put_f(mem, chr + 0x804, x87::load(b));
        put_f(mem, chr + 0x808, x87::load(c));
        put_f(mem, chr + 0x80c, x87::load(a));
        ch_wind_set_enable(mem, chr, 1);
    }

    /// `Ch_DrawSetSituation(chr, v)` (`0x00830160`): the byte at `+0x187`,
    /// and the render instance's (`+0x830`, when there is one) `+0x15`
    /// (`graphicCharacterInstanceSetDrawSituation`).
    pub fn ch_draw_set_situation(mem: &mut impl Mem, chr: u32, v: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        let b = v as u8;
        mem.write(chr + 0x187, b);
        let inst: u32 = mem.read(chr + l::RENDER_INSTANCE);
        if inst != 0 {
            mem.write(inst + 0x15, b);
        }
    }

    /// The effect packet the characters reference (`0x008439f0` returns
    /// it).
    pub const EFFECT_PACKET: Global<u32> = Global::new(0x0130_33d0);

    /// `Ch_PacketRefEffect(_, out)` (`0x00830df0`): a DMA reference tag to
    /// [`EFFECT_PACKET`] (8 quadwords); returns `out + 0x10`.
    pub fn ch_packet_ref_effect(mem: &mut impl Mem, out: u32) -> u32 {
        let at = EFFECT_PACKET.ptr(mem).addr;
        mem.write(out, [0x3000_0008u32, at, 0, 0]);
        out + 0x10
    }

    /// `crossAssertIsValidFloat(x)` (`0x006ef2d0`) as `Ch_SetRot` uses it:
    /// neither NaN nor infinite (`_isnan`, `_finite`).
    fn is_valid_float(x: f32) -> bool {
        x.is_finite()
    }

    /// `Ch_SetRot(chr, a)` (`0x0082b520`): the angle at [`l::ROT`] (through
    /// the x87); 0 instead when it is not a valid float (then a debug
    /// print, a no-op in this build).
    pub fn ch_set_rot(mem: &mut impl Mem, chr: u32, a: f32) {
        let a = x87::load(a);
        put_f(mem, chr + l::ROT, if is_valid_float(a) { a } else { 0.0 });
    }

    /// `Ch_SetDirF(chr, a)` (`0x0082b1b0`): `Ch_SetRot`, then `Ch_SetDir`
    /// with the same angle.
    pub fn ch_set_dir_f(mem: &mut impl Mem, chr: u32, a: f32) {
        ch_set_rot(mem, chr, a);
        put_f(mem, chr + l::DIR, x87::load(a));
    }

    /// `62.83184` and `-62.83184` (`double`), `Ch_CheckRadian`'s limits.
    pub const RADIAN_LIMITS: [Global<f64>; 2] =
        [Global::new(0x00b5_99e0), Global::new(0x00b5_99f0)];

    /// `Ch_CheckRadian(a)` (`0x00836790`): for an angle of ten turns or more
    /// (or NaN), `sprintf(debug line, "radian value error =%f", a)`; then
    /// it wraps the angle like `Chr_WrapAngle` (for ever on infinities, as
    /// the original) and returns it in `ST0`: callers rotate characters by
    /// it, and dropping it hid every model in the game.
    pub fn ch_check_radian(mem: &mut impl Mem, a: f32) -> f32 {
        let x = f64::from(x87::load(a));
        let [hi, lo] = RADIAN_LIMITS.map(|g| g.get(mem));
        if x.partial_cmp(&hi) != Some(core::cmp::Ordering::Less) || x <= lo {
            let mut s = b"radian value error =".to_vec();
            s.extend(crate::msvc::fmt_f(x, 6).bytes());
            let at = crate::handle::bridge::DEBUG_LINE.ptr(mem).addr;
            crate::handle::bridge::put_c_string(mem, at, &s);
        }
        super::wrap_angle(a)
    }

    /// `Ch_NeckSetEnable(chr, on)` (`0x00834f60`): the neck switch (byte
    /// `+0x40c`) and its weight `+0x414` (1.0 or 0.0); turning it on with
    /// the neck's target changed (byte `+0x40e` != `+0x40d`) restarts the
    /// blend: `Sg_MatrixToQuaternion(chr + 0x46c, chr + 0x4ac)` (passed in)
    /// and `+0x4bc` = 1.0. The target is then marked current.
    pub fn ch_neck_set_enable<M: Mem>(
        mem: &mut M,
        chr: u32,
        on: u32,
        matrix_to_quaternion: impl FnOnce(&mut M, u32, u32),
    ) {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(chr + 0x40c, on as u8);
        if on == 0 {
            mem.write(chr + 0x414, 0f32);
        } else {
            mem.write(chr + 0x414, 1f32);
            if mem.read::<u8>(chr + 0x40e) != mem.read::<u8>(chr + 0x40d) {
                matrix_to_quaternion(mem, chr + 0x46c, chr + 0x4ac);
                mem.write(chr + 0x4bc, 1f32);
            }
        }
        let t: u8 = mem.read(chr + 0x40d);
        mem.write(chr + 0x40e, t);
    }

    /// `Ch_SetRotX(chr, a, speed)` (`0x008357f0`): the x turn: target
    /// `+0x160` = the wrapped angle (`Angle_Wrap_00836720`, passed in); with
    /// a speed (not 0; NaN counts) the step `+0x164` = speed * 3.141592 /
    /// 180 (in double, stored as `f32`), else the current x angle `+0x15c`
    /// too and step 0. Returns `chr` (left in EAX).
    pub fn ch_set_rot_x(
        mem: &mut impl Mem,
        chr: u32,
        a: f32,
        speed: f32,
        wrap: impl FnOnce(f32) -> f32,
    ) -> u32 {
        let w = wrap(x87::load(a));
        mem.write(chr + 0x160, w);
        let s = x87::load(speed);
        if s == 0.0 {
            mem.write(chr + 0x15c, w);
            mem.write(chr + 0x164, 0f32);
        } else {
            let pi = f64::from_bits(0x4009_21fb_0000_0000); // 3.141592f
            #[allow(clippy::cast_possible_truncation)] // the original stores f32
            let step = (f64::from(s) * pi / 180.0) as f32;
            mem.write(chr + 0x164, step);
        }
        chr
    }

    // ---- round 8: setters that pass on to the renderer and the neck

    /// `Ch_NeckSetTargetCamera(chr)` (`0x00835080`): the neck follows the
    /// camera (mode `+0x40d` = 1), then `Ch_NeckSetEnable(chr, 1)`.
    pub fn ch_neck_set_target_camera<M: Mem>(
        mem: &mut M,
        chr: u32,
        quat: impl FnOnce(&mut M, u32, u32),
    ) {
        mem.write(chr + 0x40d, 1u8);
        ch_neck_set_enable(mem, chr, 1, quat);
    }

    /// `Ch_NeckSetTarget(chr, target)` (`0x00835050`): the neck follows
    /// another character (mode 2, `+0x4c0` = `target`).
    pub fn ch_neck_set_target<M: Mem>(
        mem: &mut M,
        chr: u32,
        target: u32,
        quat: impl FnOnce(&mut M, u32, u32),
    ) {
        mem.write(chr + 0x40d, 2u8);
        mem.write(chr + 0x4c0, target);
        ch_neck_set_enable(mem, chr, 1, quat);
    }

    /// `Ch_NeckSetTargetPos(chr, &pos)` (`0x008350a0`): the neck looks at a
    /// point (mode 3, the four words at `pos` into `+0x4c0`).
    pub fn ch_neck_set_target_pos<M: Mem>(
        mem: &mut M,
        chr: u32,
        pos: u32,
        quat: impl FnOnce(&mut M, u32, u32),
    ) {
        mem.write(chr + 0x40d, 3u8);
        let v: [u32; 4] = mem.read(pos);
        mem.write(chr + 0x4c0, v);
        ch_neck_set_enable(mem, chr, 1, quat);
    }

    /// `Ch_NeckSetRot(chr, x, y)` (`0x00834ff0`): the neck turns by fixed
    /// angles (mode 4, `+0x4c0` and `+0x4c4`).
    pub fn ch_neck_set_rot<M: Mem>(
        mem: &mut M,
        chr: u32,
        x: f32,
        y: f32,
        quat: impl FnOnce(&mut M, u32, u32),
    ) {
        mem.write(chr + 0x4c0, x87::load(x));
        mem.write(chr + 0x40d, 4u8);
        mem.write(chr + 0x4c4, x87::load(y));
        ch_neck_set_enable(mem, chr, 1, quat);
    }

    /// `Chr_SetHideFlags` (`0x0082ba30`).
    pub const SET_HIDE_FLAGS: u32 = 0x0082_ba30;
    /// `MsGetBattleScene` (`0x00782970`).
    pub const GET_BATTLE_SCENE: u32 = 0x0078_2970;
    /// `graphicLevelGetNameHash` (`0x00641da0`).
    pub const LEVEL_NAME_HASH: u32 = 0x0064_1da0;
    /// `graphicCharacterInstanceSetVisible` (`0x0063c390`).
    pub const SET_VISIBLE: u32 = 0x0063_c390;
    /// `graphicCharacterLightingSetShadeCoeff` (`0x0063cdd0`).
    pub const SET_SHADE_COEFF: u32 = 0x0063_cdd0;
    /// `graphicCharacterLightingSetShadeColor` (`0x0063d050`).
    pub const SET_SHADE_COLOR: u32 = 0x0063_d050;
    /// `Chr_FadeSet` (`0x0082b990`).
    pub const FADE_SET: u32 = 0x0082_b990;
    /// The battle scene `Ch_SetHideEvent` treats apart (compared as built,
    /// not relocated).
    pub const HIDE_EVENT_SCENE: u32 = 0x0237_0000;
    /// That scene's level name hash.
    pub const HIDE_EVENT_LEVEL: u32 = 0x21e8;
    /// The character id hidden from it.
    pub const HIDE_EVENT_ID: u16 = 0x108a;

    /// `Ch_SetHideEvent(chr, flags)` (`0x0082b2c0`): `Chr_SetHideFlags(chr,
    /// flags, 1)`, except in one battle scene and level (`0x02370000`, hash
    /// `0x21e8`) where character `0x108a` keeps no flags. `call(f, args)`
    /// runs the original at `f`; returns what `Chr_SetHideFlags` does.
    pub fn ch_set_hide_event<M: Mem>(
        mem: &mut M,
        chr: u32,
        flags: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let scene = call(mem, mem.rebase(GET_BATTLE_SCENE), &[]);
        let mut f = flags;
        // the image has no relocation for this constant: it stays as built
        if scene == HIDE_EVENT_SCENE
            && call(mem, mem.rebase(LEVEL_NAME_HASH), &[]) == HIDE_EVENT_LEVEL
            && mem.read::<u16>(chr) == HIDE_EVENT_ID
        {
            f = 0;
        }
        call(mem, mem.rebase(SET_HIDE_FLAGS), &[chr, f, 1])
    }

    /// `Ch_SetResHide(chr, hide)` (`0x0082b4e0`): byte `+0x180` = `hide`,
    /// and the renderer's instance (`+0x830`) visible when it is 0.
    pub fn ch_set_res_hide<M: Mem>(
        mem: &mut M,
        chr: u32,
        hide: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(chr + 0x180, hide as u8);
        let inst: u32 = mem.read(chr + 0x830);
        call(mem, mem.rebase(SET_VISIBLE), &[inst, u32::from(hide == 0)])
    }

    /// `Ch_SetClipZ(chr, z)` (`0x0082b150`): `+0x174` = `z * 100`; returns
    /// the old value / 100. Asks the level name hash first, unused.
    pub fn ch_set_clip_z<M: Mem>(
        mem: &mut M,
        chr: u32,
        z: f32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> f32 {
        call(mem, mem.rebase(LEVEL_NAME_HASH), &[]);
        let old = x87::load(f(mem, chr + 0x174));
        #[allow(clippy::cast_possible_truncation)] // f32 stores
        {
            mem.write(chr + 0x174, (f64::from(x87::load(z)) * 100.0) as f32);
            (f64::from(old) / 100.0) as f32
        }
    }

    /// `Ch_SetShadeIDRelation(chr, v, frames)` (`0x0082b7a0`, below),
    /// called through the game.
    pub const SET_SHADE_ID_RELATION: u32 = 0x0082_b7a0;
    /// `Ch_SetShadeID(chr, v, frames)` (`0x0082b720`), called through the
    /// game.
    pub const SET_SHADE_ID: u32 = 0x0082_b720;

    /// `Ch_SetShadeIDRelation(chr, v, frames)` (`0x0082b7a0`): every other
    /// active character bound to `chr` (`+0x19c`) gets `Ch_SetShadeID(c,
    /// v, frames)` (`v` through the x87, `fld`/`fstp`); the array and its
    /// count are read again after each call, as the walk's end. EAX is that
    /// end (`array + count * 0x880`).
    pub fn ch_set_shade_id_relation<M: Mem>(
        mem: &mut M,
        chr: u32,
        v: f32,
        frames: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        use crate::handle::bridge::{CHR_ARRAY, CHR_COUNT};
        let end = |mem: &M| {
            let n = CHR_COUNT.get(mem).cast_unsigned();
            CHR_ARRAY.get(mem).wrapping_add(n.wrapping_mul(l::SIZE))
        };
        let v = x87::load(v).to_bits();
        let mut c = CHR_ARRAY.get(mem);
        while c != end(mem) {
            if mem.read::<u8>(c.wrapping_add(l::ACTIVE)) != 0
                && c != chr
                && mem.read::<u32>(c.wrapping_add(0x19c)) == chr
            {
                call(mem, mem.rebase(SET_SHADE_ID), &[c, v, frames]);
            }
            c = c.wrapping_add(l::SIZE);
        }
        c
    }

    /// `Ch_SetShadeID(chr, v, frames)` (`0x0082b720`; callers
    /// `Ch_Allocate`, `Ch_SetShadeIDRelation`, `Ch_IdShadeCalc`,
    /// `Native_5088`): the shade-id fade (`+0x36c`) to `v` over `frames`
    /// (`Chr_FadeSet`), `Ch_SetShadeIDRelation(chr, v, frames)`, then the
    /// renderer's shade colour from the colour fades' current values
    /// (`+0x330`, `+0x33c`, `+0x348`) and its shade coefficient `v` on
    /// the instance (`+0x830`). Each float goes through the x87 (`fld`,
    /// `fstp`). EAX is the last call's.
    pub fn ch_set_shade_id<M: Mem>(
        mem: &mut M,
        chr: u32,
        v: f32,
        frames: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        chr_fade_set(mem, chr + 0x36c, x87::load(v), frames.cast_signed());
        let v = x87::load(v).to_bits();
        call(mem, mem.rebase(SET_SHADE_ID_RELATION), &[chr, v, frames]);
        let inst: u32 = mem.read(chr + 0x830);
        let [r, g, b] = [0x330, 0x33c, 0x348].map(|o| x87::load(f(mem, chr + o)).to_bits());
        call(mem, mem.rebase(SET_SHADE_COLOR), &[inst, r, g, b]);
        let inst: u32 = mem.read(chr + 0x830);
        call(mem, mem.rebase(SET_SHADE_COEFF), &[inst, v])
    }

    /// `Ch_CalcLight(chr, &coeff)` (`0x0083c6f0`): the renderer's shade
    /// coefficient of the instance (`+0x830`).
    pub fn ch_calc_light<M: Mem>(
        mem: &mut M,
        chr: u32,
        coeff: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let c = x87::load(f(mem, coeff));
        let inst: u32 = mem.read(chr + 0x830);
        call(mem, mem.rebase(SET_SHADE_COEFF), &[inst, c.to_bits()])
    }

    /// `Ch_SetShadeCol(chr, r, g, b, frames)` (`0x0082b5d0`): a colour
    /// whose largest component is above 1.3 is scaled down so it is 1.3
    /// (by `f32(1.3 / max)`); each component fades there over `frames`
    /// (`Chr_FadeSet` on `+0x330`, `+0x33c`, `+0x348`), and the renderer's
    /// shade colour is the components times the character's own (`+0xbc`,
    /// `+0xc0`, `+0xc4`). NaNs: the first comparison keeps `g`, the
    /// second takes `b`, a NaN maximum scales nothing.
    pub fn ch_set_shade_col<M: Mem>(
        mem: &mut M,
        chr: u32,
        [r, g, b]: [f32; 3],
        frames: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let (r, g, b) = (x87::load(r), x87::load(g), x87::load(b));
        // fcom: "below" only when ordered and less
        let mx = if g < r { r } else { g };
        let mx = if b < mx { mx } else { b };
        let limit = f32::from_bits(0x3fa6_6666); // 1.3f
        #[allow(clippy::cast_possible_truncation)] // f32 stores
        let (r, g, b) = if limit < mx {
            let s = (f64::from(limit) / f64::from(mx)) as f32;
            let m = |v: f32| (f64::from(v) * f64::from(s)) as f32;
            (m(r), m(g), m(b))
        } else {
            (r, g, b)
        };
        for (off, v) in [(0x330, r), (0x33c, g), (0x348, b)] {
            call(mem, mem.rebase(FADE_SET), &[chr + off, v.to_bits(), frames]);
        }
        #[allow(clippy::cast_possible_truncation)] // f32 stores
        let scaled = [(0xbc, r), (0xc0, g), (0xc4, b)]
            .map(|(off, v)| (f64::from(x87::load(f(mem, chr + off))) * f64::from(v)) as f32);
        let inst: u32 = mem.read(chr + 0x830);
        let [sr, sg, sb] = scaled.map(f32::to_bits);
        call(mem, mem.rebase(SET_SHADE_COLOR), &[inst, sr, sg, sb])
    }

    /// The import slots `Ch_ModelSetHide` calls through: `__iob_func`,
    /// `fprintf`.
    pub const IOB_FUNC_SLOT: u32 = 0x00b0_c350;
    pub const FPRINTF_SLOT: u32 = 0x00b0_c38c;
    /// `graphicCharacterInstanceSetMeshSegmentHide(inst, i, hide)`
    /// (`0x0063bdf0`).
    pub const SET_MESH_SEGMENT_HIDE: u32 = 0x0063_bdf0;
    /// Pending hides of model `f117`'s segment 0: count, then up to 4
    /// (hide, instance) pairs.
    pub const F117_HIDE_COUNT: Global<i32> = Global::new(0x012f_fad4);
    pub const F117_HIDES: Global<[u32; 2]> = Global::new(0x023c_44c0);

    /// `graphicCharacterInstanceSetMeshSegmentAlpha(inst, id, i, a)`
    /// (`0x0063bc50`) and `Ch_ModelSetAlpha`'s error format.
    pub const SET_MESH_SEGMENT_ALPHA: u32 = 0x0063_bc50;
    pub const MODEL_SET_ALPHA_FORMAT: u32 = 0x00b5_97e8;

    /// `Ch_ModelSetAlpha(chr, i, a)` (`0x0082ae30`): `a` (through the x87,
    /// so a signalling NaN is quieted) into model `i`'s alpha (`+0x14` of
    /// the 0x38-byte models at `+0x1bc`) and to the renderer
    /// (`graphicCharacterInstanceSetMeshSegmentAlpha(+0x830, id, i, a)`),
    /// whose `eax` it returns. For `i` not below the header's model count
    /// (compared signed) it prints `"VIRTUOS ERROR: Ch_ModelSetAlpha..."`
    /// to `stderr` instead and returns `fprintf`'s `eax`.
    pub fn ch_model_set_alpha<M: Mem>(
        mem: &mut M,
        chr: u32,
        i: u32,
        a: f32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let hdr: u32 = mem.read(chr + 0x1c0);
        if i.cast_signed() >= i32::from(mem.read::<u16>(hdr + 6)) {
            let name: u32 = mem.read(chr + 4);
            let f = mem.read(mem.rebase(IOB_FUNC_SLOT));
            let iob = call(mem, f, &[]);
            let f = mem.read(mem.rebase(FPRINTF_SLOT));
            let fmt = mem.rebase(MODEL_SET_ALPHA_FORMAT);
            return call(mem, f, &[iob.wrapping_add(0x40), fmt, name, i]);
        }
        let models: u32 = mem.read(chr + 0x1bc);
        let a = x87::load(a);
        mem.write(
            models.wrapping_add(i.wrapping_mul(0x38)).wrapping_add(0x14),
            a,
        );
        let id = u32::from(mem.read::<u16>(chr));
        let inst: u32 = mem.read(chr + 0x830);
        call(
            mem,
            mem.rebase(SET_MESH_SEGMENT_ALPHA),
            &[inst, id, i, a.to_bits()],
        )
    }

    /// `Ch_ModelSetHide(chr, i, hide)` (`0x0082aea0`): bit 0 of `hide`
    /// into model `i`'s hide byte (`+0x18` of the 0x38-byte models at
    /// `+0x1bc`), told to the renderer
    /// (`graphicCharacterInstanceSetMeshSegmentHide(+0x830, i, hide)`).
    /// Characters whose name holds `"f117"` instead queue a non-zero hide
    /// of model 0 for their instance, unless already queued (4 at most;
    /// a full queue starts over). Returns the old hide byte
    /// (sign-extended); for `i` not below the header's model count
    /// (`u16` `+6` of `+0x1c0`, compared signed) it prints `"VIRTUOS
    /// ERROR: ..."` to `stderr` (`fprintf` through the import slot) and
    /// returns 0.
    pub fn ch_model_set_hide<M: Mem>(
        mem: &mut M,
        chr: u32,
        i: u32,
        hide: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let hdr: u32 = mem.read(chr + 0x1c0);
        let name: u32 = mem.read(chr + 4);
        if i.cast_signed() >= i32::from(mem.read::<u16>(hdr + 6)) {
            let f = mem.read(mem.rebase(IOB_FUNC_SLOT));
            let iob = call(mem, f, &[]);
            let f = mem.read(mem.rebase(FPRINTF_SLOT));
            call(
                mem,
                f,
                &[iob.wrapping_add(0x40), mem.rebase(0x00b5_9778), name, i],
            );
            return 0;
        }
        let models: u32 = mem.read(chr + 0x1bc);
        let m = models.wrapping_add(i.wrapping_mul(0x38));
        let old = i32::from(mem.read::<i8>(m + 0x18)).cast_unsigned();
        #[allow(clippy::cast_possible_truncation)] // bit 0
        mem.write(m + 0x18, (hide & 1) as u8);
        let inst: u32 = mem.read(chr + 0x830);
        if !crate::handle::bridge::c_string(mem, name)
            .windows(4)
            .any(|w| w == b"f117")
        {
            call(mem, mem.rebase(SET_MESH_SEGMENT_HIDE), &[inst, i, hide]);
            return old;
        }
        if hide == 0 || i != 0 {
            return old;
        }
        let pairs = F117_HIDES.ptr(mem).addr;
        let count = F117_HIDE_COUNT.get(mem);
        if (0..count).any(|k| mem.read::<u32>(pairs + 8 * k.cast_unsigned() + 4) == inst) {
            return old;
        }
        let k = if count >= 4 { 0 } else { count };
        let at = pairs.wrapping_add(8u32.wrapping_mul(k.cast_unsigned()));
        mem.write(at, hide);
        mem.write(at + 4, inst);
        F117_HIDE_COUNT.set(mem, k.wrapping_add(1));
        old
    }

    /// `Sg_MemGetSize(p)` (`0x0083c050`) and `user_free_thunk(p)`
    /// (`0x00687010`).
    pub const MEM_GET_SIZE: u32 = 0x0083_c050;
    pub const USER_FREE: u32 = 0x0068_7010;
    /// `Ch_MemMoveSetTrigger()` (`0x00827250`).
    pub const MEM_MOVE_SET_TRIGGER: u32 = 0x0082_7250;

    /// `Ch_ModelPurgePacket(chr, all)` (`0x00827700`): once (flag bit 5 of
    /// `+0x198`), hides the renderer's instance and frees each model's
    /// packets (`+8`, `+4`, and with `all` > 0 `+0xc` too, of the 0x38-byte
    /// models at `+0x1bc`, count `u16` `+6` of `+0x1c0`), adding up their
    /// sizes (`Sg_MemGetSize` before `user_free_thunk`); then
    /// `Ch_MemMoveSetTrigger()`. Returns the bytes freed (0 the second
    /// time).
    pub fn ch_model_purge_packet<M: Mem>(
        mem: &mut M,
        chr: u32,
        all: i32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let flags: u32 = mem.read(chr + 0x198);
        if flags & 0x20 != 0 {
            return 0;
        }
        mem.write(chr + 0x198, flags | 0x20);
        let inst: u32 = mem.read(chr + 0x830);
        call(mem, mem.rebase(SET_VISIBLE), &[inst, 0]);
        let (size, free) = (mem.rebase(MEM_GET_SIZE), mem.rebase(USER_FREE));
        let mut total = 0u32;
        let mut i = 0u32;
        let count = |mem: &M| {
            let h: u32 = mem.read(chr + 0x1c0);
            u32::from(mem.read::<u16>(h + 6))
        };
        while i < count(mem) {
            let models: u32 = mem.read(chr + 0x1bc);
            let m = models.wrapping_add(0x38 * i);
            for off in [8, 4] {
                let p: u32 = mem.read(m + off);
                total = total.wrapping_add(call(mem, size, &[p]));
            }
            for off in [8, 4] {
                let p: u32 = mem.read(m + off);
                call(mem, free, &[p]);
                mem.write(m + off, 0u32);
            }
            if all > 0 {
                let p: u32 = mem.read(m + 0xc);
                total = total.wrapping_add(call(mem, size, &[p]));
                let p: u32 = mem.read(m + 0xc);
                call(mem, free, &[p]);
                mem.write(m + 0xc, 0u32);
            }
            i += 1;
        }
        call(mem, mem.rebase(MEM_MOVE_SET_TRIGGER), &[]);
        total
    }

    /// `FFXVu0Normalize(out, v)` (`0x006ed6f0`), `Ch_ClothReset(chr)`
    /// (`0x00824650`), `MagicFile_GetMagicId()` (`0x009da3b0`).
    pub const VU0_NORMALIZE: u32 = 0x006e_d6f0;
    pub const CLOTH_RESET: u32 = 0x0082_4650;
    pub const GET_MAGIC_ID: u32 = 0x009d_a3b0;
    /// Bytes of scratch `ch_model_create_packet` needs (a block's name).
    pub const CREATE_PACKET_SCRATCH: u32 = 0x40;

    /// `Ch_ModelCreatePacket(chr, scratch)` (`0x00827280`): flag bit 11
    /// (`+0x198`) on while each model (0x38 bytes at `+0x1bc`, count `u16`
    /// `+6` of `+0x1c0`) gets what it lacks, each block named `"<name>:
    /// vtex"`, `":norm"`, `":physics"` (`Sg_MemSetStr(block, name, model)`
    /// with the name in `scratch`, the original's stack; type 1): `+8` its
    /// vertices (`u16` `+4` of its data, `i16` triples at `+8` as `f32` x,
    /// y, z, 1), `+4` its normals (`u16` `+0x16`, triples at `+0xc`, each
    /// normalised with `FFXVu0Normalize`), and with physics entries (`u16`
    /// `+0x18`, 0x24 bytes at `+0x1c`) `+0xc` room for the sum of their
    /// `i8` `+0x13` and `i16` `+0x1c` counts (`+0x10`) × 0x30 bytes. A
    /// character with flag bit 5 that got physics is reset
    /// (`Ch_ClothReset`); flag bits 5 and 11 then cleared. The renderer's
    /// instance is shown, except character 0x402e during magic 0x295,
    /// which stays hidden (hide byte `+0x180` = 1). Returns
    /// `graphicCharacterInstanceSetVisible`'s result.
    #[allow(clippy::too_many_lines)] // one pass over the models
    pub fn ch_model_create_packet<M: Mem>(
        mem: &mut M,
        chr: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let malloc = mem.rebase(crate::handle::bridge::USER_MALLOC);
        let set_str = mem.rebase(crate::handle::bridge::MEM_SET_STR);
        let set_type = mem.rebase(crate::sys::bridge::MEM_SET_TYPE);
        let flags: u32 = mem.read(chr + 0x198);
        mem.write(chr + 0x198, flags | 0x800);
        let name_at: u32 = mem.read(chr + 4);
        let name = crate::handle::bridge::c_string(mem, name_at);
        let label = |mem: &mut M,
                     block: u32,
                     what: &[u8],
                     i: u32,
                     call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32| {
            let mut s = name.clone();
            s.push(b':');
            s.extend(what);
            crate::handle::bridge::put_c_string(mem, scratch, &s);
            call(mem, set_str, &[block, scratch, i]);
            call(mem, set_type, &[block, 1]);
        };
        let count = |mem: &M| {
            let h: u32 = mem.read(chr + 0x1c0);
            u32::from(mem.read::<u16>(h + 6))
        };
        let triple = |mem: &mut M, src: u32, dst: u32| {
            for k in 0..3 {
                mem.write(dst + 4 * k, f32::from(mem.read::<i16>(src + 2 * k)));
            }
            mem.write(dst + 0xc, 1f32);
        };
        let mut physics = false;
        let mut i = 0u32;
        while i < count(mem) {
            let m = mem.read::<u32>(chr + 0x1bc).wrapping_add(0x38 * i);
            if mem.read::<u32>(m + 8) == 0 {
                let d: u32 = mem.read(m);
                let n = u32::from(mem.read::<u16>(d + 4));
                let p = call(mem, malloc, &[n << 4]);
                mem.write(m + 8, p);
                let d: u32 = mem.read(m);
                let mut src: u32 = mem.read(d + 8);
                let mut k = 0u32;
                while k < u32::from(mem.read::<u16>(mem.read::<u32>(m) + 4)) {
                    let p: u32 = mem.read(m + 8);
                    triple(mem, src, p.wrapping_add(0x10 * k));
                    src = src.wrapping_add(6);
                    k += 1;
                }
                let p: u32 = mem.read(m + 8);
                label(mem, p, b"vtex", i, call);
            }
            if mem.read::<u32>(m + 4) == 0 {
                let d: u32 = mem.read(m);
                let n = u32::from(mem.read::<u16>(d + 0x16));
                let p = call(mem, malloc, &[n << 4]);
                mem.write(m + 4, p);
                let d: u32 = mem.read(m);
                let mut src: u32 = mem.read(d + 0xc);
                let mut k = 0u32;
                while k < u32::from(mem.read::<u16>(mem.read::<u32>(m) + 0x16)) {
                    let p: u32 = mem.read(m + 4);
                    let v = p.wrapping_add(0x10 * k);
                    triple(mem, src, v);
                    let p: u32 = mem.read(m + 4);
                    let v = p.wrapping_add(0x10 * k);
                    call(mem, mem.rebase(VU0_NORMALIZE), &[v, v]);
                    src = src.wrapping_add(6);
                    k += 1;
                }
                let p: u32 = mem.read(m + 4);
                label(mem, p, b"norm", i, call);
            }
            if mem.read::<u32>(m + 0xc) == 0 {
                let d: u32 = mem.read(m);
                let n = u32::from(mem.read::<u16>(d + 0x18));
                if n != 0 {
                    let mut e = mem.read::<u32>(d + 0x1c).wrapping_add(0x13);
                    let mut total = 0i32;
                    for _ in 0..n {
                        total = total
                            .wrapping_add(i32::from(mem.read::<i8>(e)))
                            .wrapping_add(i32::from(mem.read::<i16>(e + 9)));
                        e = e.wrapping_add(0x24);
                    }
                    mem.write(m + 0x10, total);
                    let p = call(mem, malloc, &[total.cast_unsigned().wrapping_mul(0x30)]);
                    mem.write(m + 0xc, p);
                    label(mem, p, b"physics", i, call);
                    physics = true;
                }
            }
            i += 1;
        }
        if mem.read::<u32>(chr + 0x198) & 0x20 != 0 && physics {
            call(mem, mem.rebase(CLOTH_RESET), &[chr]);
        }
        let flags: u32 = mem.read(chr + 0x198);
        mem.write(chr + 0x198, flags & 0xffff_f7df);
        let inst: u32 = mem.read(chr + 0x830);
        if mem.read::<u16>(chr) == 0x402e && call(mem, mem.rebase(GET_MAGIC_ID), &[]) == 0x295 {
            let r = call(mem, mem.rebase(SET_VISIBLE), &[inst, 0]);
            mem.write(chr + 0x180, 1u8);
            return r;
        }
        let inst: u32 = mem.read(chr + 0x830);
        call(mem, mem.rebase(SET_VISIBLE), &[inst, 1])
    }

    /// The callees `Ch_MemMove` adds: `Ch_FunctionHistory(file, line)`
    /// (`0x00833770`), `Ch_ElemRemapping(chr, delta)` (`0x00826980`),
    /// `Ch_MotionRemappingFcurve(chr, delta)` (`0x00839890`),
    /// `Ch_TextureReMapping(chr)` (`0x0083cdd0`), `TexAnim_GetWork(i)`
    /// (`0x0077e9f0`), `texanim_malloc_memory_set(i, delta)` (`0x0077ea10`),
    /// `Ch_ClutMapping(chr, copy)` (`0x0083ca20`), `Ch_ClutEnable(chr, on)`
    /// (`0x0083c940`), `op_mdc_mem_transfer()` (`0x00803610`),
    /// `Ch_MemMoveEventMgrp()` (`0x00837050`), `Sg_PLightGetPtr()`
    /// (`0x00831b70`).
    pub const FUNCTION_HISTORY: u32 = 0x0083_3770;
    pub const ELEM_REMAPPING: u32 = 0x0082_6980;
    pub const MOTION_REMAPPING_FCURVE: u32 = 0x0083_9890;
    pub const TEXTURE_REMAPPING: u32 = 0x0083_cdd0;
    pub const TEX_ANIM_GET_WORK: u32 = 0x0077_e9f0;
    pub const TEX_ANIM_MEMORY_SET: u32 = 0x0077_ea10;
    pub const CLUT_MAPPING: u32 = 0x0083_ca20;
    pub const CLUT_ENABLE_FN: u32 = 0x0083_c940;
    pub const MDC_MEM_TRANSFER: u32 = 0x0080_3610;
    pub const MEM_MOVE_EVENT_MGRP: u32 = 0x0083_7050;
    pub const PLIGHT_GET_PTR: u32 = 0x0083_1b70;
    /// `g_ChrMemMoveTrigger` (`u8`) and the source file name
    /// `Ch_FunctionHistory` gets.
    pub const MEM_MOVE_TRIGGER: u32 = 0x012f_bda0;
    pub const MEM_MOVE_FILE: u32 = 0x00b5_9520;
    /// Bytes of scratch `ch_mem_move` needs: the moved flag (`[ebp-4]`)
    /// and a work pointer (`[ebp-0xc]`).
    pub const MEM_MOVE_SCRATCH: u32 = 0xc;

    /// `Ch_MemMove()` (`0x00826fa0`, one function; the dump splits it at
    /// the early return): with [`MEM_MOVE_TRIGGER`] set, moves each active
    /// character's heap blocks that `Sg_MemMoveWork(&ptr, &moved)` moves,
    /// fixing up what points into them: elements `+0x328`
    /// (`Ch_ElemRemapping`), curves `+0x748` (`Ch_MotionRemappingFcurve`),
    /// the model table `+0x1bc`, `+0x32c` and each model's `+8`, `+4`,
    /// `+0xc`, the texture list `+0x6dc` (`Ch_TextureReMapping`), the
    /// texture animation's work (`+0x700`) and the cluts `+0x704`
    /// (`Ch_ClutMapping`, `Ch_ClutEnable(+0x708)`). Characters with flag
    /// bits 10, 11 of `+0x198` are skipped. A character that moved
    /// anything ends the pass (`op_mdc_mem_transfer`): 1. Then the event
    /// motion groups (`Ch_MemMoveEventMgrp`) and the point lights
    /// (`Sg_PLightGetPtr`): 1 when something moved; with nothing left the
    /// trigger is cleared: 0. (`dbgPrintf` calls are left out: it is
    /// empty.)
    #[allow(clippy::too_many_lines)]
    pub fn ch_mem_move<M: Mem>(
        mem: &mut M,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let moved = scratch + 0xc - 4;
        let work = scratch;
        mem.write(moved, 0u32);
        if mem.read::<u8>(mem.rebase(MEM_MOVE_TRIGGER)) == 0 {
            return 0;
        }
        let r = |mem: &M, a: u32| mem.rebase(a);
        let file = r(mem, MEM_MOVE_FILE);
        call(mem, r(mem, FUNCTION_HISTORY), &[file, 0xb52]);
        let mw = r(mem, crate::motion::bridge::MEM_MOVE_WORK);
        let end = |mem: &M| {
            let n = crate::handle::bridge::CHR_COUNT.get(mem).cast_unsigned();
            crate::handle::bridge::CHR_ARRAY
                .get(mem)
                .wrapping_add(n.wrapping_mul(0x880))
        };
        mem.write(moved, 0u32);
        let mut chr = crate::handle::bridge::CHR_ARRAY.get(mem);
        let mut stop = end(mem);
        while chr != stop {
            if mem.read::<u8>(chr + 2) == 0 || mem.read::<u32>(chr + 0x198) & 0xc00 != 0 {
                chr = chr.wrapping_add(0x880);
                continue;
            }
            let d = call(mem, mw, &[chr + 0x328, moved]);
            if d != 0 {
                call(mem, r(mem, ELEM_REMAPPING), &[chr, d]);
            }
            let d = call(mem, mw, &[chr + 0x748, moved]);
            if d != 0 {
                call(mem, r(mem, MOTION_REMAPPING_FCURVE), &[chr, d]);
            }
            call(mem, mw, &[chr + 0x1bc, moved]);
            call(mem, mw, &[chr + 0x32c, moved]);
            if mem.read::<u32>(chr + 0x1bc) != 0 {
                let mut i = 0u32;
                while i < u32::from(mem.read::<u16>(mem.read::<u32>(chr + 0x1c0).wrapping_add(6))) {
                    for off in [8u32, 4, 0xc] {
                        let m = mem.read::<u32>(chr + 0x1bc).wrapping_add(0x38 * i);
                        call(mem, mw, &[m.wrapping_add(off), moved]);
                    }
                    i += 1;
                }
            }
            if call(mem, mw, &[chr + 0x6dc, moved]) != 0 {
                call(mem, r(mem, TEXTURE_REMAPPING), &[chr]);
            }
            if mem.read::<u32>(chr + 0x6fc) != 0 {
                let anim: u32 = mem.read(chr + 0x700);
                let w = call(mem, r(mem, TEX_ANIM_GET_WORK), &[anim]);
                mem.write(work, w);
                let d = call(mem, mw, &[work, moved]);
                if d != 0 {
                    let anim: u32 = mem.read(chr + 0x700);
                    call(mem, r(mem, TEX_ANIM_MEMORY_SET), &[anim, d]);
                }
            }
            if mem.read::<u32>(chr + 0x704) != 0 {
                call(mem, mw, &[chr + 0x704, moved]);
                call(mem, r(mem, CLUT_MAPPING), &[chr, 0]);
                let on = i32::from(mem.read::<i8>(chr + 0x708)).cast_unsigned();
                call(mem, r(mem, CLUT_ENABLE_FN), &[chr, on]);
            }
            if mem.read::<u32>(moved) != 0 {
                call(mem, r(mem, MDC_MEM_TRANSFER), &[]);
                return 1;
            }
            stop = end(mem);
            chr = chr.wrapping_add(0x880);
        }
        call(mem, r(mem, FUNCTION_HISTORY), &[file, 0xb85]);
        if call(mem, r(mem, MEM_MOVE_EVENT_MGRP), &[]) != 0 {
            return 1;
        }
        let p = call(mem, r(mem, PLIGHT_GET_PTR), &[moved]);
        if call(mem, mw, &[p, moved]) != 0 {
            return 1;
        }
        mem.write(r(mem, MEM_MOVE_TRIGGER), 0u8);
        0
    }

    /// `Cache_Unuse(kind, id)` (`0x00840d70`), `Ch_EnableSetTexture(on)`
    /// (`0x00830330`), `Ch_GuiSetCalcNormAll(on)` (`0x00834090`),
    /// `Ch_InitCircleTable()` (`0x0082e5a0`), `ChN_DisposeSystemMGRP(id, k)`
    /// (`0x00836830`).
    pub const CACHE_UNUSE: u32 = 0x0084_0d70;
    pub const ENABLE_SET_TEXTURE: u32 = 0x0083_0330;
    pub const GUI_SET_CALC_NORM_ALL: u32 = 0x0083_4090;
    pub const INIT_CIRCLE_TABLE: u32 = 0x0082_e5a0;
    pub const DISPOSE_SYSTEM_MGRP: u32 = 0x0083_6830;

    /// `Ch_BlkDispose(blk)` (`0x008258d0`): a used model block (id `+0` not
    /// -1) gives back what it holds. A resident block (flag bit 0 of
    /// `+0x70`) disposes its four system motion groups and its cache entry
    /// (`Cache_Unuse(1, id)`) and frees `+0x124` (cleared); others free
    /// `+4`, `+0x14`, `+0x10c`, `+0x108`. A resident one with bit 1 frees
    /// `+0x6c` (and with bit 2 `+0x14`). Then `+0x11c`, `+0x68`, `+0x100`
    /// (cleared) are freed and the id becomes -1. Returns what EAX holds:
    /// -1 for an unused block, else the last `user_free`'s result.
    pub fn ch_blk_dispose<M: Mem>(
        mem: &mut M,
        blk: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let id: u32 = mem.read(blk);
        if id == u32::MAX {
            return id;
        }
        let free = mem.rebase(USER_FREE);
        let free_at = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, off: u32| {
            let p: u32 = mem.read(blk + off);
            call(mem, free, &[p])
        };
        if mem.read::<u8>(blk + 0x70) & 1 != 0 {
            for k in 0..4 {
                let id: u32 = mem.read(blk);
                call(mem, mem.rebase(DISPOSE_SYSTEM_MGRP), &[id, k]);
            }
            let id: u32 = mem.read(blk);
            call(mem, mem.rebase(CACHE_UNUSE), &[1, id]);
            if mem.read::<u32>(blk + 0x124) != 0 {
                free_at(mem, call, 0x124);
                mem.write(blk + 0x124, 0u32);
            }
        } else {
            for off in [4u32, 0x14, 0x10c, 0x108] {
                free_at(mem, call, off);
            }
        }
        let flags = mem.read::<u8>(blk + 0x70);
        if flags & 3 == 3 {
            free_at(mem, call, 0x6c);
            if mem.read::<u8>(blk + 0x70) & 4 != 0 {
                free_at(mem, call, 0x14);
            }
        }
        free_at(mem, call, 0x11c);
        free_at(mem, call, 0x68);
        let r = free_at(mem, call, 0x100);
        mem.write(blk + 0x100, 0u32);
        mem.write(blk, u32::MAX);
        r
    }

    /// `Ch_IsShadowExceptionId(chr)` (`0x0082e200`): 1 for characters 3
    /// and 0x67 (`u16` `+0`), else 0.
    pub fn ch_is_shadow_exception_id<M: Mem>(mem: &mut M, chr: u32) -> u32 {
        u32::from(matches!(mem.read::<u16>(chr), 3 | 0x67))
    }

    /// `Ch_MemMove_Over()` (`0x00827260`): `Ch_MemMove` (`0x00826fa0`)
    /// until it moves nothing; the number of passes that moved something.
    pub fn ch_mem_move_over<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut n = 0u32;
        while call(mem, mem.rebase(0x0082_6fa0), &[]) != 0 {
            n += 1;
        }
        n
    }

    /// `Ch_DrawInitAll()` (`0x0082ed30`): `Ch_EnableSetTexture(1)`,
    /// `Ch_GuiSetCalcNormAll(0)`, then (a tail call) `Ch_InitCircleTable()`.
    pub fn ch_draw_init_all<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        call(mem, mem.rebase(ENABLE_SET_TEXTURE), &[1]);
        call(mem, mem.rebase(GUI_SET_CALC_NORM_ALL), &[0]);
        call(mem, mem.rebase(INIT_CIRCLE_TABLE), &[]);
    }

    /// The shadow globals: mode (`u8`, `g_ChrShadowMode`; 0 lets each
    /// character's `i8` `+0x183` decide, else mode + 1) and direction (two
    /// `i32`, `g_ChrShadowDir`), and the callees `Sg_GetSpadPtr()`
    /// (`0x00843220`), `Ch_IsShadowHidden(chr)` (`0x008303a0`),
    /// `Ch_IsNotExceptionId(id)` (`0x00831780`), `Ch_DrawShadowQuad(chr,
    /// p2, p3)` (`0x0082e3e0`) and the shadow direction setter
    /// `0x00642fb0(x, y)`.
    pub const SHADOW_MODE: u32 = 0x0130_08bd;
    pub const SHADOW_DIR: u32 = 0x0130_08c4;
    pub const GET_SPAD_PTR: u32 = 0x0084_3220;
    pub const IS_SHADOW_HIDDEN: u32 = 0x0083_03a0;
    pub const IS_NOT_EXCEPTION_ID: u32 = 0x0083_1780;
    pub const DRAW_SHADOW_QUAD: u32 = 0x0082_e3e0;
    pub const SET_SHADOW_DIR: u32 = 0x0064_2fb0;

    /// The shadow mode of a character that may draw one (active, flag bit
    /// 4 of `+0x198`, not hidden, `+0x186` not 2 or 3); `None` otherwise.
    fn shadow_mode<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> Option<i32> {
        if mem.read::<u8>(chr + 2) == 0 || mem.read::<u8>(chr + 0x198) & 0x10 == 0 {
            return None;
        }
        if call(mem, mem.rebase(IS_SHADOW_HIDDEN), &[chr]) != 0
            || matches!(mem.read::<u8>(chr + 0x186), 2 | 3)
        {
            return None;
        }
        let g = mem.read::<u8>(mem.rebase(SHADOW_MODE));
        Some(if g == 0 {
            i32::from(mem.read::<i8>(chr + 0x183))
        } else {
            i32::from(g) - 1
        })
    }

    /// `Ch_DrawShadow(on)` (`0x00830180`): with `on`, every character in
    /// shadow mode 1 that is not an exception id gets its shadow quad
    /// (`Ch_DrawShadowQuad(chr, chr + 0x114, Sg_GetSpadPtr())`).
    pub fn ch_draw_shadow<M: Mem>(
        mem: &mut M,
        on: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        if on == 0 {
            return;
        }
        let spad = call(mem, mem.rebase(GET_SPAD_PTR), &[]);
        let end = |mem: &M| {
            let n = crate::handle::bridge::CHR_COUNT.get(mem).cast_unsigned();
            crate::handle::bridge::CHR_ARRAY
                .get(mem)
                .wrapping_add(n.wrapping_mul(0x880))
        };
        let mut chr = crate::handle::bridge::CHR_ARRAY.get(mem);
        if chr == end(mem) {
            return;
        }
        loop {
            if shadow_mode(mem, chr, call) == Some(1) {
                let id = u32::from(mem.read::<u16>(chr));
                if call(mem, mem.rebase(IS_NOT_EXCEPTION_ID), &[id]) != 0 {
                    call(mem, mem.rebase(DRAW_SHADOW_QUAD), &[chr, chr + 0x114, spad]);
                }
            }
            chr = chr.wrapping_add(0x880);
            if chr == end(mem) {
                return;
            }
        }
    }

    /// `Ch_DrawSingleShadow(chr)` (`0x00830240`): one character's shadow:
    /// mode 3 first points the shadow direction ([`SHADOW_DIR`] as floats,
    /// `0x00642fb0`), modes 3 and 1 draw the quad (`Ch_DrawShadowQuad(chr,
    /// chr + 0x114, 0)`), for ids that are not exceptions.
    pub fn ch_draw_single_shadow<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let Some(mode) = shadow_mode(mem, chr, call) else {
            return;
        };
        if mode != 3 && mode != 1 {
            return;
        }
        let id = u32::from(mem.read::<u16>(chr));
        if call(mem, mem.rebase(IS_NOT_EXCEPTION_ID), &[id]) == 0 {
            return;
        }
        if mode == 3 {
            #[allow(clippy::cast_precision_loss)] // fild, fstp dword
            let [x, y] =
                [0u32, 4].map(|o| (mem.read::<i32>(mem.rebase(SHADOW_DIR) + o) as f32).to_bits());
            call(mem, mem.rebase(SET_SHADOW_DIR), &[x, y]);
        }
        call(mem, mem.rebase(DRAW_SHADOW_QUAD), &[chr, chr + 0x114, 0]);
    }

    /// `Ch_BlkDisposeAll()` (`0x008259c0`): `Ch_BlkDispose` on every used
    /// model block (id not -1).
    pub fn ch_blk_dispose_all<M: Mem>(
        mem: &mut M,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let base = crate::handle::bridge::CHR_BLK.ptr(mem).addr;
        for k in 0..40u32 {
            let b = base + 0x12c * k;
            if mem.read::<u32>(b) != u32::MAX {
                call(mem, mem.rebase(0x0082_58d0), &[b]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits() {
        assert_eq!(put_bit(0xffff_ffff, 1 << 11, 0), (0xffff_f7ff, 1 << 11));
        assert_eq!(put_bit(0, 1 << 11, 3), (1 << 11, 1 << 11));
        assert_eq!(put_bit(1 << 11, 1 << 11, 1), (1 << 11, 0));
        assert_eq!(put_bit_inverted(0, 1 << 8, 0), 1 << 8);
        assert_eq!(put_bit_inverted(1 << 8, 1 << 8, 1), 0);
        assert_eq!(hide_flags(0x80, true, 0x100), (0x80, 1));
        assert_eq!(hide_flags(0x13, false, 0x10), (0x03, 1));
        assert_eq!(next_hokan(-1), 0);
        assert_eq!(next_hokan(0), 1);
        assert_eq!(next_hokan(0x1_0005), 5);
    }

    #[test]
    fn angles_wrap_by_steps() {
        assert_eq!(wrap_angle(1.0), 1.0);
        assert!((wrap_angle(4.0) - (4.0 - 6.283_184)).abs() < 1e-6);
        assert!((wrap_angle(-10.0) - (-10.0 + 2.0 * 6.283_184)).abs() < 1e-5);
        assert!(wrap_angle(f32::NAN).is_nan());
    }

    #[test]
    fn defaults() {
        assert_eq!(chg_run(f32::NAN), 18.0);
        assert_eq!(chg_run(0.0), 18.0);
        assert!(or_default_if_negative(f32::NAN, 1.0).is_nan());
        assert_eq!(or_default_if_negative(-0.5, 1.0), 1.0);
        assert_eq!(or_default_if_negative(-0.0, 1.0).to_bits(), 0x8000_0000);
        assert_eq!(or_default_if_not_positive(0.0, 1.0), 1.0);
        assert!(or_default_if_not_positive(f32::NAN, 1.0).is_nan());
    }
}
