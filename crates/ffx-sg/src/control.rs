//! Requests the main loop takes from the game and passes on: the blur and
//! cross-fade (`Sg_Blur`, `Sg_AccSetAlpha`), sound pause and mute, the
//! map's fast alpha, light directions, the end of the GS packet, resets.
//! The work behind them (the renderer, the sound system) stays with its
//! own module: the bridges call it through [`Host`].

/// The originals, over game memory.
#[allow(clippy::many_single_char_names)] // pointers and words
pub mod bridge {
    use ffx_bridge::{Global, Host, x87};

    /// The accumulation/blur request flags (`0x01300704`: bit 0 the
    /// cross-fade, bit 1 the blur).
    pub const ACC_FLAGS: Global<u32> = Global::new(0x0130_0704);
    /// The cross-fade's pending byte.
    pub const ACC_PENDING: Global<u8> = Global::new(0x0130_0708);
    /// The cross-fade's frames (`u16`).
    pub const ACC_FRAMES: Global<u16> = Global::new(0x0130_070c);
    /// The cross-fade's alpha (`u16`).
    pub const ACC_ALPHA: Global<u16> = Global::new(0x0130_070e);
    /// The cross-fade's target alpha (`u16`).
    pub const ACC_TARGET: Global<u16> = Global::new(0x0130_0710);
    /// The cross-fade's step (`u16`).
    pub const ACC_STEP: Global<u16> = Global::new(0x0130_0712);
    /// The blur's set byte.
    pub const BLUR_SET: Global<u8> = Global::new(0x0130_0718);
    /// The blur's on flag (`u16`) and weight (`u16`, at `+2`).
    pub const BLUR_ON: Global<u16> = Global::new(0x0130_071c);
    pub const BLUR_WEIGHT: Global<u16> = Global::new(0x0130_071e);
    /// `g_CurrentMapNo`.
    pub const MAP_NO: Global<u32> = Global::new(0x012f_bc40);
    /// The sound state (`0x012fb8ec`): 0 normal, 1 muted, 2 paused.
    pub const SOUND_STATE: Global<u32> = Global::new(0x012f_b8ec);
    /// The map's fast-alpha byte.
    pub const MAP_ALPHA_FAST: Global<u8> = Global::new(0x012f_bbb6);

    pub const GRAPHIC_SET_MOTION_BLUR_ENABLE: u32 = 0x0064_3730;
    pub const GRAPHIC_SET_MOTION_BLUR_WEIGHT: u32 = 0x0064_3750;
    pub const GRAPHIC_UPDATE_CROSS_FADE: u32 = 0x0064_4910;
    pub const GRAPHIC_SET_MAP_ALPHA_FAST: u32 = 0x0064_3610;
    pub const IS_IN_TK_MENU_PAUSE: u32 = 0x008a_8e10;
    pub const VOICE_SET_PAUSED: u32 = 0x0067_a750;
    pub const SFX_PAUSE: u32 = 0x0067_a380;
    pub const SFX_STOP_NON_LOOP: u32 = 0x0067_a4c0;
    pub const SOUND_SET_MUTE: u32 = 0x0067_a570;

    /// `Sg_Blur(weight)` (`0x0082bee0`): requests the motion blur (bit 1,
    /// the set byte); a weight turns it on (`+0x1c` 1, the weight's low
    /// half kept) at `weight / 128`, 0 turns it off (the on flag and weight
    /// cleared as one word). Returns the renderer call's `eax`.
    pub fn blur(h: &mut impl Host, weight: u32) -> u32 {
        let f = ACC_FLAGS.get(h) | 2;
        ACC_FLAGS.set(h, f);
        BLUR_SET.set(h, 1);
        if weight == 0 {
            Global::<u32>::new(BLUR_ON.sym).set(h, 0);
            let at = h.rebase(GRAPHIC_SET_MOTION_BLUR_ENABLE);
            return h.call(at, &[0]);
        }
        BLUR_ON.set(h, 1);
        BLUR_WEIGHT.set(h, crate::lo16(weight));
        let at = h.rebase(GRAPHIC_SET_MOTION_BLUR_ENABLE);
        h.call(at, &[1]);
        let w = i32::from(BLUR_WEIGHT.get(h).cast_signed());
        let f = x87::store(x87::mul(f64::from(w), 0.007_812_5));
        let at = h.rebase(GRAPHIC_SET_MOTION_BLUR_WEIGHT);
        h.call(at, &[f])
    }

    /// `Sg_AccSetAlpha(alpha, frames)` (`0x0082bd10`): requests the
    /// cross-fade (bit 0, pending cleared, the frames' low half). Over
    /// frames, the target is `alpha` and the step `|alpha - current| /
    /// frames` (at least 1); at once, the alpha becomes `alpha` (and on map
    /// `0x102` the step 0). A visible alpha with no frames counts one.
    /// Ends with `graphicUpdateCrossFade(&alpha, &frames)` (its `eax`).
    pub fn acc_set_alpha(h: &mut impl Host, alpha: u32, frames: u32) -> u32 {
        let f = ACC_FLAGS.get(h) | 1;
        ACC_FLAGS.set(h, f);
        ACC_PENDING.set(h, 0);
        ACC_FRAMES.set(h, crate::lo16(frames));
        let cur;
        if frames == 0 {
            cur = crate::lo16(alpha);
            ACC_ALPHA.set(h, cur);
            if MAP_NO.get(h) == 0x102 {
                ACC_STEP.set(h, 0);
            }
        } else {
            cur = ACC_ALPHA.get(h);
            ACC_TARGET.set(h, crate::lo16(alpha));
            let a = i32::from(crate::lo16(alpha).cast_signed());
            let c = i32::from(cur.cast_signed());
            let diff = (c - a).abs();
            let step = crate::lo16(diff.wrapping_div(frames.cast_signed()).cast_unsigned());
            ACC_STEP.set(h, if step == 0 { 1 } else { step });
        }
        if cur != 0 && crate::lo16(frames) == 0 {
            ACC_FRAMES.set(h, 1);
        }
        let alpha_at = h.rebase(ACC_ALPHA.sym);
        let frames_at = h.rebase(ACC_FRAMES.sym);
        let at = h.rebase(GRAPHIC_UPDATE_CROSS_FADE);
        h.call(at, &[alpha_at, frames_at])
    }

    /// `Sg_SetPauseAllSounds(on)` (`0x00821cc0`): pauses (from normal) or
    /// resumes (from paused) the voices and effects, unless the menu's
    /// pause holds them.
    pub fn set_pause_all_sounds(h: &mut impl Host, on: u32) {
        let s = SOUND_STATE.get(h);
        let go = match s {
            0 => on != 0,
            2 => on == 0,
            _ => false,
        };
        if !go {
            return;
        }
        let at = h.rebase(IS_IN_TK_MENU_PAUSE);
        if h.call(at, &[]) != 0 {
            return;
        }
        SOUND_STATE.set(h, if on != 0 { 2 } else { 0 });
        let at = h.rebase(VOICE_SET_PAUSED);
        h.call(at, &[on]);
        let at = h.rebase(SFX_PAUSE);
        h.call(at, &[on]);
    }

    /// `Sg_SetMuteSound(on)` (`0x00821c60`): mutes (from normal) or
    /// unmutes (from muted): the one-shot effects stopped, effects paused
    /// or resumed, the mute set (`iSoundSetMute(on, 2)`).
    pub fn set_mute_sound(h: &mut impl Host, on: u32) {
        match SOUND_STATE.get(h) {
            0 if on != 0 => SOUND_STATE.set(h, 1),
            1 if on == 0 => SOUND_STATE.set(h, 0),
            _ => return,
        }
        let at = h.rebase(SFX_STOP_NON_LOOP);
        h.call(at, &[]);
        let at = h.rebase(SFX_PAUSE);
        h.call(at, &[on]);
        let at = h.rebase(SOUND_SET_MUTE);
        h.call(at, &[on, 2]);
    }

    /// `Sg_SetMapAlphaFast(on)` (`0x00821b00`): the byte, then
    /// `graphicSetMapAlphaFast(on)` (a tail call: its `eax`).
    pub fn set_map_alpha_fast(h: &mut impl Host, on: u32) -> u32 {
        MAP_ALPHA_FAST.set(h, crate::lo8(on));
        let at = h.rebase(GRAPHIC_SET_MAP_ALPHA_FAST);
        h.call(at, &[on])
    }

    /// The point lights' table (`0x013008cc`) and count.
    pub const PLIGHT_TABLE: Global<u32> = Global::new(0x0130_08cc);
    pub const PLIGHT_COUNT: Global<u32> = Global::new(0x0130_08d0);
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const MEM_MOVE_SET_TRIGGER: u32 = 0x0082_7250;

    /// `Sg_PLightDispose()` (`0x00831ba0`): frees the point lights, clears
    /// the table and count, and triggers the memory move (a tail call: its
    /// `eax`).
    pub fn plight_dispose(h: &mut impl Host) -> u32 {
        let t = PLIGHT_TABLE.get(h);
        let at = h.rebase(USER_FREE);
        h.call(at, &[t]);
        PLIGHT_TABLE.set(h, 0);
        PLIGHT_COUNT.set(h, 0);
        let at = h.rebase(MEM_MOVE_SET_TRIGGER);
        h.call(at, &[])
    }

    /// `Sg_AccRequestCurrent()` (`0x0082bdf0`, native `0x4019`; its
    /// argument unread): the post-process cross-fade capture turned on
    /// (`0x00639150`), then the request flag `0x012ffae4` that
    /// `sg_AccSetCurrent` (`0x0082ce90`) consumes to keep the current frame.
    /// Returns the call's `eax`.
    pub fn acc_request_current(h: &mut impl Host) -> u32 {
        let at = h.rebase(0x0063_9150);
        let r = h.call(at, &[]);
        Global::<u32>::new(0x012f_fae4).set(h, 1);
        r
    }

    /// `dreamParamReset()` (`0x00822410`): the dream sequence's
    /// parameters cleared (`0x012ffaf4` 0.0, `0x00c496d0` -1, the rest 0)
    /// and its special menu texture (10) unloaded. Returns that call's
    /// `eax`.
    pub fn dream_param_reset(h: &mut impl Host) -> u32 {
        Global::<u32>::new(0x012f_faf4).set(h, 0);
        Global::<u32>::new(0x012f_b8f0).set(h, 0);
        Global::<u32>::new(0x00c4_96d0).set(h, u32::MAX);
        Global::<u32>::new(0x012f_fae8).set(h, 0);
        Global::<u32>::new(0x012f_faec).set(h, 0);
        Global::<u32>::new(0x012f_b8f4).set(h, 0);
        let at = h.rebase(0x0064_2390);
        h.call(at, &[10])
    }

    /// The three light directions (`Sg_LightSetDir`'s slots 0-2).
    pub const LIGHT_DIR: [u32; 3] = [0x00c4_9724, 0x00c4_9734, 0x0130_08e8];
    /// `g_Vu0ClipScreen`.
    pub const VU0_CLIP_SCREEN: u32 = 0x0130_0978;
    pub const VU0_NORMAL_LIGHT_MATRIX: u32 = 0x006e_d620;
    pub const LIGHTING_SET_LIGHT_DIR: u32 = 0x0063_cb50;

    /// `Sg_LightSetDir(i, dir)` (`0x00836430`): light `i`'s (0-2) direction
    /// (four words), then the normal-light matrix from all three
    /// (`FFXVu0NormalLightMatrix` into `g_Vu0ClipScreen`) and the
    /// character lighting's direction (`graphicCharacterLightingSetLightDir`;
    /// its `eax`).
    pub fn light_set_dir(h: &mut impl Host, i: u32, dir: u32) -> u32 {
        if let Some(&slot) = LIGHT_DIR.get(i as usize) {
            let at = h.rebase(slot);
            for k in 0..4 {
                let w: u32 = h.read(dir + 4 * k);
                h.write(at + 4 * k, w);
            }
        }
        let [a, b, c] = LIGHT_DIR.map(|s| h.rebase(s));
        let m = h.rebase(VU0_CLIP_SCREEN);
        let at = h.rebase(VU0_NORMAL_LIGHT_MATRIX);
        h.call(at, &[m, a, b, c]);
        let mut v = [0u32; 3];
        for (k, w) in (0u32..).zip(v.iter_mut()) {
            *w = x87::round_trip(f32::from_bits(h.read(dir + 4 * k))).to_bits();
        }
        let at = h.rebase(LIGHTING_SET_LIGHT_DIR);
        h.call(at, &[i, v[0], v[1], v[2]])
    }

    /// The DMA channel the GS packet goes out on (`0x023cbbf4`).
    pub const GS_DMA_CHANNEL: Global<u32> = Global::new(0x023c_bbf4);
    pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
    pub const DMA_SYNC: u32 = 0x0062_f730;
    pub const DMA_SEND: u32 = 0x0062_f710;

    /// `Sg_GsEnd()` (`0x00841770`): closes the GS packet: the tag's
    /// quadword count (15 bits: count - start - 1), an end quadword at the
    /// count (`0x8001`, `0x10000000`, `0xf`, 0), the first quadword `count
    /// | 0x70000000` and zeros; then `FlushCache(0)`, `sceDmaSync(ch, 0,
    /// 0)` and `sceDmaSend(ch, packet)` (its `eax`).
    pub fn gs_end(h: &mut impl Host) -> u32 {
        use crate::gs::bridge::{COUNT, PACKET, START, TAG};
        let n = COUNT.get(h);
        let tag = TAG.get(h);
        let t: u32 = h.read(tag);
        let q = n.wrapping_sub(START.get(h)).wrapping_sub(1);
        h.write(tag, t ^ ((q ^ t) & 0x7fff));
        let p = PACKET.get(h);
        let at = p.wrapping_add(n << 4);
        h.write(at, [0x8001u32, 0x1000_0000, 0xf, 0]);
        h.write(p + 4, 0u32);
        COUNT.set(h, n.wrapping_add(1));
        h.write(p + 8, 0u32);
        h.write(p + 0xc, 0u32);
        h.write(p, n.wrapping_add(1) | 0x7000_0000);
        let at = h.rebase(FLUSH_CACHE);
        h.call(at, &[0]);
        let ch = GS_DMA_CHANNEL.get(h);
        let at = h.rebase(DMA_SYNC);
        h.call(at, &[ch, 0, 0]);
        let p = PACKET.get(h);
        let ch = GS_DMA_CHANNEL.get(h);
        let at = h.rebase(DMA_SEND);
        h.call(at, &[ch, p])
    }
}
