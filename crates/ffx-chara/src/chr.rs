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
