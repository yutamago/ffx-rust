//! Requests the main loop takes from the game and passes on: the blur and
//! cross-fade (`Sg_Blur`, `Sg_AccSetAlpha`), sound pause and mute, the
//! map's fast alpha, light directions, the end of the GS packet, resets.
//! The work behind them (the renderer, the sound system) stays with its
//! own module: the bridges call it through [`Host`].

/// The originals, over game memory.
#[allow(clippy::many_single_char_names)] // pointers and words
pub mod bridge {
    use ffx_bridge::{Global, Host, Mem, x87};

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

    pub const USER_MALLOC: u32 = 0x0068_7020;

    /// `Sg_PLightAllocate(n)` (`0x00831950`): the old lights disposed
    /// ([`plight_dispose`]), a table of `n` 44-byte lights allocated
    /// (`user_malloc`), the count `n`; each light off with `+0xc` and
    /// `+0x1c` 1.0 (the rest as allocated). Returns the table.
    pub fn plight_allocate(h: &mut impl Host, n: u32) -> u32 {
        plight_dispose(h);
        let at = h.rebase(USER_MALLOC);
        let t = h.call(at, &[n.wrapping_mul(0x2c)]);
        PLIGHT_TABLE.set(h, t);
        PLIGHT_COUNT.set(h, n);
        let mut t = t;
        let mut e = 0u32;
        for _ in 0..n.cast_signed().max(0) {
            h.write(t.wrapping_add(e).wrapping_add(0x28), 0u8);
            t = PLIGHT_TABLE.get(h);
            h.write(t.wrapping_add(e).wrapping_add(0xc), 1.0f32);
            t = PLIGHT_TABLE.get(h);
            h.write(t.wrapping_add(e).wrapping_add(0x1c), 1.0f32);
            t = PLIGHT_TABLE.get(h);
            e = e.wrapping_add(0x2c);
        }
        t
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

    /// `Sg_AccInitAll()` (`0x0082bcd0`): no cross-fade
    /// (`Sg_AccSetAlpha(0, 0)`), map drawing off, the current frame
    /// requested ([`acc_request_current`]), background and characters
    /// off, the clear colour 0.
    pub fn acc_init_all(h: &mut impl Host) {
        use crate::state::bridge as st;
        acc_set_alpha(h, 0, 0);
        st::acc_set_draw_map(h, 0);
        acc_request_current(h);
        st::acc_enable_bg(h, 0);
        st::acc_set_draw_chr(h, 0);
        st::acc_set_clear(h, 0, 0, 0, 0);
    }

    /// `Sg_LightSetDirXYZ(i, x, y, z)` (`0x008364f0`): [`light_set_dir`]
    /// with `(x, y, z, 0)` from the stack (each through `fld`/`fstp`).
    pub fn light_set_dir_xyz(h: &mut impl Host, i: u32, x: f32, y: f32, z: f32) -> u32 {
        use crate::scratch::{SCRATCH, Scratch};
        let mut s = Scratch::new(h);
        let v = [x, y, z].map(x87::round_trip);
        s.write(SCRATCH, [v[0], v[1], v[2], 0.0]);
        light_set_dir(&mut s, i, SCRATCH)
    }

    /// `g_FFXIsDebug`.
    pub const IS_DEBUG: Global<u8> = Global::new(0x0133_c910);

    /// `Sg_AccRequestCurrent`'s flag (`g_AccCurrentRequest`).
    pub const ACC_CURRENT_REQUEST: Global<u32> = Global::new(0x012f_fae4);
    /// The five screen fades, 0x10 bytes each: mode, red, green, blue
    /// (bytes), then `i16` frames left, total (or the level), target,
    /// step.
    pub const FADE_SLOTS: u32 = 0x0130_0708;
    /// The slots still running, a bit each.
    pub const FADE_ACTIVE: Global<u32> = Global::new(0x0130_0704);
    /// The focus (depth of field) requested.
    pub const FOCUS_ON: Global<u32> = Global::new(0x0130_0768);
    /// The focus drawn.
    pub const FOCUS_DRAW: Global<u32> = Global::new(0x0130_076c);
    /// The two focus depths.
    pub const FOCUS_Z: [Global<u32>; 2] = [Global::new(0x0130_0758), Global::new(0x0130_075c)];
    /// The packet the filter builds into.
    pub const FILTER_PACKET: u32 = 0x012f_fb00;
    /// The PS2 screen's width (0x200 draws the mode 0 level).
    pub const SCREEN_W: Global<u32> = Global::new(0x023c_bbe4);
    /// The fade mode the dream effect sees (-1 none).
    pub const DREAM_FADE_MODE: Global<u32> = Global::new(0x00c4_96d0);
    /// The dream fade's frames.
    pub const DREAM_FADE_TOTAL: Global<i32> = Global::new(0x012f_faf0);
    /// The dream fade's share (`f32`).
    pub const DREAM_FADE_SHARE: Global<f32> = Global::new(0x012f_faf4);
    /// Set when a fade ran for the dream effect.
    pub const DREAM_FADE_SEEN: Global<u32> = Global::new(0x012f_fae8);
    /// Set once the level fade reached its last frames.
    pub const DREAM_FADE_OUT: Global<u32> = Global::new(0x012f_faec);
    /// 127 (`f64`).
    pub const FADE_127: Global<f64> = Global::new(0x00b9_22e8);
    /// 1/256 (`f64`).
    pub const FADE_BYTE: Global<f64> = Global::new(0x00b5_0198);
    /// 255 (`f64`).
    pub const FADE_255: Global<f64> = Global::new(0x00b1_70c0);
    /// `graphicHasRunDrawFilterInThisFrame()`.
    pub const GRAPHIC_HAS_RUN_DRAW_FILTER: u32 = 0x0064_1660;
    /// `graphicResetHasRunDrawFocusInThisFrame()`.
    pub const GRAPHIC_RESET_HAS_RUN_DRAW_FOCUS: u32 = 0x0064_2c70;
    /// `graphicHasRunDrawFocusInThisFrame()`.
    pub const GRAPHIC_HAS_RUN_DRAW_FOCUS: u32 = 0x0064_1670;
    /// `graphicSetFocusZ(which, z)`.
    pub const GRAPHIC_SET_FOCUS_Z: u32 = 0x0064_30f0;
    /// `graphicAddScreenFadeTask(r, g, b, a, flags)`.
    pub const GRAPHIC_ADD_SCREEN_FADE_TASK: u32 = 0x0063_9240;
    /// The focus packet for one depth (GS glue).
    pub const SG_DRAW_FOCUS: u32 = 0x0082_bb90;
    /// The level quad of a mode 0 fade (GS glue).
    pub const SG_DRAW_LEVEL: u32 = 0x0082_baf0;

    fn call_sym(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        let at = h.rebase(sym);
        h.call(at, args)
    }

    /// `_Sg_PcRead(path, kind)` (`Sg_PcRead_0083b1d0`, platform: reads
    /// the file).
    pub const SG_PC_READ_FILE: u32 = 0x0083_b1d0;
    /// `Sg_MemSetCurrentAddr(p)` (platform).
    pub const SG_MEM_SET_CURRENT_ADDR: u32 = 0x0083_c4e0;

    /// `Sg_PcRead(path)` (`0x0083ae30`): the file read by `_Sg_PcRead(path,
    /// 0)`, then `Sg_MemSetCurrentAddr` to the result; returns it.
    pub fn pc_read(h: &mut impl Host, path: u32) -> u32 {
        let r = call_sym(h, SG_PC_READ_FILE, &[path, 0]);
        call_sym(h, SG_MEM_SET_CURRENT_ADDR, &[r]);
        r
    }

    /// `Sg_PcReadD(path)` (`0x0083ae50`): [`pc_read`] with
    /// `_Sg_PcRead(path, 1)`.
    pub fn pc_read_d(h: &mut impl Host, path: u32) -> u32 {
        let r = call_sym(h, SG_PC_READ_FILE, &[path, 1]);
        call_sym(h, SG_MEM_SET_CURRENT_ADDR, &[r]);
        r
    }

    /// `Sg_DrawFilter()` (`0x0082c0c0`): the frame's focus and fades.
    ///
    /// With a fade or the focus running, the filter packet is started; the
    /// focus draws both depths (`graphicSetFocusZ`). Then each slot with
    /// frames left (else its bit in [`FADE_ACTIVE`] is cleared) runs by
    /// mode, colors as `byte / 256`, levels over 127, through
    /// `graphicAddScreenFadeTask(r, g, b, a, flags)`:
    ///
    /// - 0 a level moving by its step to the target, its quad drawn when
    ///   the screen is 0x200 wide unless a request is current; the dream
    ///   effect gets its share.
    /// - 2 black at `1 - (127 - n' 127 / total) / 127`, then
    ///   `Sg_AccSetAlpha(0, 0)`; 3 black at `1 - (n' 127 / total) / 127`,
    ///   held at its last frame.
    /// - 4 the color at `n 255 / total / 127`; 5 rising to 255 and held; 6
    ///   at `n 127 / total / 127`; 7 and 8 at the level `+6 / 127` (8 with
    ///   flag 0x10).
    ///
    /// Modes 1 and above 8 do nothing. A total of 0 divides by zero in the
    /// original (a crash); here the quotient counts as 0.
    #[allow(clippy::too_many_lines, clippy::cast_precision_loss)] // small integers
    pub fn draw_filter(h: &mut impl Host) {
        use x87::Reg;
        call_sym(h, GRAPHIC_HAS_RUN_DRAW_FILTER, &[]);
        call_sym(h, GRAPHIC_RESET_HAS_RUN_DRAW_FOCUS, &[]);
        if FADE_ACTIVE.get(h) == 0 && FOCUS_ON.get(h) == 0 {
            return;
        }
        let packet = h.rebase(FILTER_PACKET);
        crate::gs::bridge::set_packet(h, packet);
        crate::gs::bridge::start(h);
        if FOCUS_DRAW.get(h) != 0 && FOCUS_ON.get(h) != 0 {
            crate::gs::bridge::test_1(h, [1, 1, 0, 0, 0, 0, 1, 2]);
            for z in FOCUS_Z {
                let v = z.get(h);
                if v != 0 {
                    call_sym(h, SG_DRAW_FOCUS, &[v]);
                }
            }
            for (i, z) in (0u32..).zip(FOCUS_Z) {
                let v = z.get(h);
                call_sym(h, GRAPHIC_SET_FOCUS_Z, &[i, v]);
            }
            call_sym(h, GRAPHIC_HAS_RUN_DRAW_FOCUS, &[]);
        }
        crate::gs::bridge::test_1(h, [1, 1, 0, 0, 0, 0, 1, 1]);
        let k127 = Reg::constant(FADE_127.get(h));
        let byte = Reg::constant(FADE_BYTE.get(h));
        let k255 = Reg::constant(FADE_255.get(h));
        let base = h.rebase(FADE_SLOTS);
        for i in 0..5u32 {
            let s = base + 0x10 * i;
            let n: i16 = h.read(s + 4);
            if n == 0 {
                let x = FADE_ACTIVE.get(h) & !(1 << i);
                FADE_ACTIVE.set(h, x);
                continue;
            }
            let mode: u8 = h.read(s);
            let total: i16 = h.read(s + 6);
            let div = |x: i32| x.checked_div(i32::from(total)).unwrap_or(0);
            let level = |x: i32| (Reg::load(x as f32) / k127).store();
            let rgb = [1, 2, 3].map(|o| {
                let c: u8 = h.read(s + o);
                (Reg::constant(f64::from(c)) * byte).store()
            });
            let dream = DREAM_ON.get(h) != 0;
            let fade = |h: &mut _, rgb: [f32; 3], a: f32, flags: u32| {
                let [r, g, b] = rgb.map(f32::to_bits);
                call_sym(
                    h,
                    GRAPHIC_ADD_SCREEN_FADE_TASK,
                    &[r, g, b, a.to_bits(), flags],
                );
            };
            match mode {
                0 => {
                    let n1 = n.wrapping_sub(1);
                    h.write(s + 4, n1);
                    let cur: i16 = h.read(s + 6);
                    let target: i16 = h.read(s + 8);
                    let step: i16 = h.read(s + 0xa);
                    if target != cur {
                        let v = if target > cur {
                            cur.wrapping_add(step)
                        } else {
                            cur.wrapping_sub(step)
                        };
                        h.write(s + 6, v);
                        if (target > cur && target < v) || (target < cur && target > v) {
                            h.write(s + 6, target);
                        }
                    }
                    let cur: i16 = h.read(s + 6);
                    if cur != 0 && n1 == 0 {
                        h.write(s + 4, 1i16);
                    }
                    if ACC_CURRENT_REQUEST.get(h) == 0 {
                        if SCREEN_W.get(h) != 0x200 {
                            continue;
                        }
                        #[allow(clippy::cast_sign_loss)]
                        call_sym(h, SG_DRAW_LEVEL, &[i32::from(cur) as u32]);
                    }
                    if dream {
                        let n: i16 = h.read(s + 4);
                        if DREAM_FADE_MODE.get(h) == u32::MAX {
                            DREAM_FADE_TOTAL.set(h, i32::from(n));
                        }
                        DREAM_FADE_SEEN.set(h, 1);
                        if n < 2 {
                            DREAM_FADE_OUT.set(h, 1);
                            DREAM_FADE_MODE.set(h, u32::MAX);
                        } else {
                            DREAM_FADE_MODE.set(h, u32::from(mode));
                            let t = DREAM_FADE_TOTAL.get(h) as f32;
                            let r = Reg::constant(f64::from(n)).div_m32(t);
                            let r = if DREAM_FADE_OUT.get(h) == 0 {
                                r
                            } else {
                                Reg::ONE - r
                            };
                            DREAM_FADE_SHARE.set(h, r.store());
                        }
                    }
                }
                2 | 3 => {
                    let n1 = n.wrapping_sub(1);
                    h.write(s + 4, n1);
                    let q = if mode == 2 {
                        0x7f - div(i32::from(n1) * 0x7f)
                    } else if n1 == 0 {
                        h.write(s + 4, 1i16);
                        0
                    } else {
                        div(i32::from(n1) * 0x7f)
                    };
                    let a = (Reg::ONE - Reg::load(q as f32) / k127).store();
                    fade(h, [0.0; 3], a, 0);
                    if mode == 2 {
                        acc_set_alpha(h, 0, 0);
                    }
                }
                4 => {
                    let q = div(i32::from(n) * 0xff);
                    fade(h, rgb, level(q), 0);
                    if dream {
                        DREAM_FADE_MODE.set(h, u32::from(mode));
                        DREAM_FADE_SEEN.set(h, 1);
                        DREAM_FADE_SHARE.set(h, (Reg::load(q as f32) / k255).store());
                        if n < 3 {
                            DREAM_FADE_MODE.set(h, u32::MAX);
                        }
                    }
                    h.write(s + 4, n.wrapping_sub(1));
                }
                5 => {
                    let n1 = n.wrapping_sub(1);
                    h.write(s + 4, n1);
                    let v = if n1 == 0 {
                        h.write(s + 4, 1i16);
                        0xff
                    } else {
                        0xff - div(i32::from(n1) * 0xff)
                    };
                    fade(h, rgb, level(v), 0);
                    if dream {
                        DREAM_FADE_MODE.set(h, u32::from(mode));
                        DREAM_FADE_SHARE.set(h, (Reg::load(v as f32) / k255).store());
                    }
                }
                6 => {
                    fade(h, rgb, level(div(i32::from(n) * 0x7f)), 0);
                    h.write(s + 4, n.wrapping_sub(1));
                }
                7 | 8 => {
                    if mode == 7 && dream {
                        h.write(s + 6, 0i16);
                    }
                    let v: i16 = h.read(s + 6);
                    let a = (Reg::constant(f64::from(v)) / k127).store();
                    fade(h, rgb, a, if mode == 8 { 0x10 } else { 0 });
                }
                _ => {}
            }
        }
        gs_end(h);
    }

    /// `ExportValidateInterface(id)` (`0x008202e0`): 1 for the interface
    /// ids the executable exports (0x40400003, 0x40800001..=0x40800003),
    /// else 0.
    #[must_use]
    pub const fn export_validate_interface(id: u32) -> u32 {
        matches!(id, 0x4040_0003 | 0x4080_0001 | 0x4080_0002 | 0x4080_0003) as u32
    }

    /// `Sg_InitArgs(argc, argv)` (`0x00820310`): the L/R button mode 0,
    /// `0x012fb8fa` 0xff and `0x012fb8f9`, `0x012fb8f8` 0; in a debug
    /// build with an argument, the user name from `argv[1]`
    /// (`Sg_SetUserName`).
    pub fn init_args(h: &mut impl Host, count: u32, args: u32) {
        use crate::state::bridge as st;
        st::set_lr_button_mode(h, 0);
        Global::<u8>::new(0x012f_b8fa).set(h, 0xff);
        Global::<u8>::new(0x012f_b8f9).set(h, 0);
        Global::<u8>::new(0x012f_b8f8).set(h, 0);
        if IS_DEBUG.get(h) != 0 && count.cast_signed() > 1 {
            let name: u32 = h.read(args.wrapping_add(4));
            st::set_user_name(h, name);
        }
    }

    pub const GET_SCENERIO_FLAG: u32 = 0x0078_7360;
    pub const MS_BATTLE_CHECK: u32 = 0x0078_0c90;
    pub const MAGIC_FILE_GET_MAGIC_ID: u32 = 0x009d_a3b0;
    /// `0x012fb7e4`: set by `setFramSync`.
    pub const FRAM_SYNC: Global<u32> = Global::new(0x012f_b7e4);

    /// `setFramSync()` (`0x00822720`): `getScenerioFlag()` (unused), then
    /// [`FRAM_SYNC`] 1 in a battle (`MsBattleCheck`) with a magic file
    /// loaded (`MagicFile_GetMagicId() != -1`), else 0. Returns the last
    /// call's `eax`.
    pub fn set_fram_sync(h: &mut impl Host) -> u32 {
        let at = h.rebase(GET_SCENERIO_FLAG);
        h.call(at, &[]);
        let at = h.rebase(MS_BATTLE_CHECK);
        let r = h.call(at, &[]);
        if r == 0 {
            FRAM_SYNC.set(h, 0);
            return r;
        }
        let at = h.rebase(MAGIC_FILE_GET_MAGIC_ID);
        let id = h.call(at, &[]);
        FRAM_SYNC.set(h, u32::from(id != u32::MAX));
        id
    }

    pub const YN_PRE_BG_LAYER_DISP_GET: u32 = 0x0091_a770;
    pub const GRAPHIC_MENU_LOAD_SPECIAL_TEXTURE: u32 = 0x0064_2300;
    pub const GRAPHIC_MENU_UNLOAD_SPECIAL_TEXTURE: u32 = 0x0064_2390;
    pub const GRAPHIC_2D_HIDE_LAYER: u32 = 0x0063_8f30;
    pub const GRAPHIC_2D_SHOW_LAYER: u32 = 0x0063_8f60;
    pub const GRAPHIC_TEXANIM_SET_MATERIAL_VISIBILITY: u32 = 0x0064_4120;
    pub const GRAPHIC_DRAW_DREAM: u32 = 0x0063_df70;
    /// The special texture's name (`0x00b20b30`).
    pub const DREAM_TEXTURE: u32 = 0x00b2_0b30;
    /// The dream effect's state: on (`0x012fb8f0`) and its phase
    /// (`0x012fb8f4`, 5 drawing).
    pub const DREAM_ON: Global<u32> = Global::new(0x012f_b8f0);
    pub const DREAM_PHASE: Global<u32> = Global::new(0x012f_b8f4);

    /// The dream effect over layers (`recallSceneEffect`'s and
    /// `recallSceneEffect2`'s common work): for each layer's display byte
    /// (`Yn_PreBgLayerDispGet`), a shown layer without bits 0-1 starts it
    /// (once: on, phase 0, special texture 10 loaded), hides those layers
    /// (`graphic2DHideLayer`, texture animation off) and, in phase 5,
    /// draws it (`graphicDrawDream`, a tail call). Otherwise a layer with
    /// bits 0-1 ends a started effect past phase 0: texture unloaded,
    /// `dreamParamReset`, the shown layers back on. Returns the last
    /// call's `eax`.
    fn dream_layers(h: &mut impl Host, layers: &[u32]) -> u32 {
        let mut r = 0;
        let mut disp = [0u8; 2];
        for (d, &l) in disp.iter_mut().zip(layers) {
            let at = h.rebase(YN_PRE_BG_LAYER_DISP_GET);
            r = h.call(at, &[l]);
            *d = crate::lo8(r);
        }
        let disp = &disp[..layers.len()];
        let hide = disp.iter().any(|&d| d != 0 && d.trailing_zeros() >= 2);
        if hide {
            if DREAM_ON.get(h) == 0 {
                DREAM_ON.set(h, 1);
                DREAM_PHASE.set(h, 0);
                let at = h.rebase(GRAPHIC_MENU_LOAD_SPECIAL_TEXTURE);
                let name = h.rebase(DREAM_TEXTURE);
                r = h.call(at, &[10, name, 0]);
            }
            for (&d, &l) in disp.iter().zip(layers) {
                if layers.len() == 1 || d != 0 {
                    let at = h.rebase(GRAPHIC_2D_HIDE_LAYER);
                    h.call(at, &[l]);
                    let at = h.rebase(GRAPHIC_TEXANIM_SET_MATERIAL_VISIBILITY);
                    r = h.call(at, &[l, 0]);
                }
            }
            if DREAM_PHASE.get(h) == 5 {
                let at = h.rebase(GRAPHIC_DRAW_DREAM);
                r = h.call(at, &[]);
            }
            return r;
        }
        if disp.iter().any(|&d| d & 3 != 0) && DREAM_ON.get(h) != 0 && DREAM_PHASE.get(h) != 0 {
            let at = h.rebase(GRAPHIC_MENU_UNLOAD_SPECIAL_TEXTURE);
            h.call(at, &[10]);
            dream_param_reset(h);
            for (&d, &l) in disp.iter().zip(layers) {
                if layers.len() == 1 || d != 0 {
                    let at = h.rebase(GRAPHIC_2D_SHOW_LAYER);
                    h.call(at, &[l]);
                    let at = h.rebase(GRAPHIC_TEXANIM_SET_MATERIAL_VISIBILITY);
                    r = h.call(at, &[l, 1]);
                }
            }
        }
        r
    }

    /// `recallSceneEffect(layer)` (`0x00822580`): the dream effect over
    /// one layer (see [`dream_layers`]).
    pub fn recall_scene_effect(h: &mut impl Host, layer: u32) -> u32 {
        dream_layers(h, &[layer])
    }

    /// `recallSceneEffect2(a, b)` (`0x00822620`): the dream effect over two
    /// layers; only shown ones are hidden or shown again.
    pub fn recall_scene_effect2(h: &mut impl Host, a: u32, b: u32) -> u32 {
        dream_layers(h, &[a, b])
    }

    /// `iSyncGetData(&time, &rate, &frame)`, the sound sync manager's.
    pub const ISYNC_GET_DATA: u32 = 0x0067_a930;
    /// The cut scene's sync state (`0x012fb8a0` on, `0x012fb8a4`,
    /// `0x012fb8a8`, the time `0x012fb8ac` and rate `0x012fb8b0` as `f32`,
    /// the frame table `0x012fb828` (30 words) and its index
    /// `0x012fb814`).
    pub const SYNC_ON: Global<u32> = Global::new(0x012f_b8a0);
    pub const SYNC_TIME: Global<f32> = Global::new(0x012f_b8ac);
    pub const SYNC_RATE: Global<f32> = Global::new(0x012f_b8b0);
    pub const SYNC_FRAMES: u32 = 0x012f_b828;
    pub const SYNC_INDEX: Global<u32> = Global::new(0x012f_b814);
    /// `1000.0` (`f64`).
    pub const THOUSAND: Global<f64> = Global::new(0x00b0_dba0);

    /// `Sg_SyncCutScene()` (`0x00821d60`): asks the sound sync manager for
    /// the time, rate and frame (`iSyncGetData`, the frame in a local the
    /// host's callee scratch holds). An answer of 999 (no sync) clears
    /// everything: the floats 0, the frame table, `0x00c494d4` 1; returns
    /// the table (`memset`'s `eax`). Otherwise the sync is on, the frame
    /// `<< 7` goes into the table at the index, the time and rate become
    /// milliseconds (`x 1000`); returns the index. A host without a scratch
    /// does nothing and returns 0.
    pub fn sync_cut_scene(h: &mut impl Host) -> u32 {
        let Some(local) = h.callee_scratch() else {
            return 0;
        };
        h.write(local, 0u32);
        let at = h.rebase(ISYNC_GET_DATA);
        let (time, rate) = (h.rebase(SYNC_TIME.sym), h.rebase(SYNC_RATE.sym));
        let r = h.call(at, &[time, rate, local]);
        let table = h.rebase(SYNC_FRAMES);
        if r == 999 {
            for g in [0x012f_b8a4, 0x012f_b8a8, 0x012f_b8ac] {
                Global::<u32>::new(g).set(h, 0);
            }
            SYNC_ON.set(h, 0);
            Global::<u32>::new(0x012f_b8b0).set(h, 0);
            Global::<u32>::new(0x00c4_94d4).set(h, 1);
            Global::<u32>::new(0x012f_b818).set(h, 0);
            Global::<u32>::new(0x012f_b81c).set(h, 0);
            SYNC_INDEX.set(h, 0);
            h.write_bytes(table, &[0; 0x78]);
            return table;
        }
        let k = THOUSAND.get(h);
        let t = x87::load(SYNC_TIME.get(h).to_bits());
        let frame: u32 = h.read(local);
        let i = SYNC_INDEX.get(h);
        SYNC_ON.set(h, 1);
        h.write(table.wrapping_add(i.wrapping_mul(4)), frame << 7);
        SYNC_TIME.set(h, f32::from_bits(x87::store(x87::mul(t, k))));
        let rt = x87::load(SYNC_RATE.get(h).to_bits());
        SYNC_RATE.set(h, f32::from_bits(x87::store(x87::mul(k, rt))));
        i
    }

    /// `g_GraphicMapId`.
    pub const GRAPHIC_MAP_ID: Global<u32> = Global::new(0x0133_0844);

    /// `drawDreamEffect()` (`0x008223a0`): the dream effect on the maps
    /// that have one: layer 0 on maps `0x86`, `0xa5`, `0xbf`, `0xf9` and
    /// `0xfc` ([`recall_scene_effect`]), layers 3 and 4 on `0x129` and 7 and 8
    /// on `0x187` ([`recall_scene_effect2`]); elsewhere a started effect is
    /// reset (`dreamParamReset`). Returns the call's `eax`, or what the
    /// comparison chain left (`map - 0xbf` or `map - 0x187`).
    pub fn draw_dream_effect(h: &mut impl Host) -> u32 {
        let map = GRAPHIC_MAP_ID.get(h);
        match map {
            0x86 | 0xa5 | 0xbf | 0xf9 | 0xfc => recall_scene_effect(h, 0),
            0x129 => recall_scene_effect2(h, 3, 4),
            0x187 => recall_scene_effect2(h, 7, 8),
            _ => {
                if DREAM_ON.get(h) != 0 {
                    return dream_param_reset(h);
                }
                if map.cast_signed() > 0xf9 {
                    map.wrapping_sub(0x187)
                } else {
                    map.wrapping_sub(0xbf)
                }
            }
        }
    }

    /// The effect strip's last three vertices (`0x01303354`, 8 bytes each:
    /// `u16` x, y, u, v), the newest one's slot and the vertex count.
    pub const STRIP_RING: u32 = 0x0130_3354;
    pub const STRIP_CUR: Global<u32> = Global::new(0x0130_336c);
    pub const STRIP_COUNT: Global<u16> = Global::new(0x0130_3370);
    /// `Ch_EffectPacketStorePoly(a, b, c)`.
    pub const CH_EFFECT_PACKET_STORE_POLY: u32 = 0x0084_5820;

    /// `Ch_EffectPacketStoreStrip(v)` (`0x00844370`): the next vertex of a
    /// triangle strip (null starts a new strip: count 0, returns 0). The
    /// vertex goes into the ring slot `count % 3`. An x or y with bit 15
    /// set refers back: `n = (x & 0x7fff) * 4 / 3` words before the
    /// vertex's own x (y: one word further back), the value there taken
    /// instead. From the third vertex on, each makes a triangle with the
    /// two before it, wound alternately (even counts `(n-1, n-2, new)`,
    /// odd `(n-2, n-1, new)`), stored by `Ch_EffectPacketStorePoly`. Returns
    /// that call's `eax`, or the vertex's v.
    pub fn effect_store_strip(h: &mut impl Host, v: u32) -> u32 {
        if v == 0 {
            STRIP_COUNT.set(h, 0);
            return 0;
        }
        let n = STRIP_COUNT.get(h);
        let ring = h.rebase(STRIP_RING);
        let slot = |k: u32| ring.wrapping_add(8 * (k % 3));
        let cur = slot(u32::from(n));
        STRIP_CUR.set(h, cur);
        let back = |h: &mut _, w: u16, extra: u32| -> u16 {
            if w & 0x8000 == 0 {
                return w;
            }
            let q = (u32::from(w & 0x7fff) << 2) / 3;
            Mem::read(h, v.wrapping_sub(2 * q).wrapping_sub(extra))
        };
        let x: u16 = h.read(v);
        let x = back(h, x, 0);
        h.write(cur, x);
        let y: u16 = h.read(v.wrapping_add(2));
        let y = back(h, y, 2);
        h.write(cur.wrapping_add(2), y);
        let u: u16 = h.read(v.wrapping_add(4));
        h.write(cur.wrapping_add(4), u);
        let w: u16 = h.read(v.wrapping_add(6));
        h.write(cur.wrapping_add(6), w);
        if n <= 1 {
            STRIP_COUNT.set(h, n.wrapping_add(1));
            return u32::from(w);
        }
        let k = u32::from(n);
        let (p, q) = (slot(k - 1), slot(k - 2));
        let (first, second) = if n & 1 == 0 { (p, q) } else { (q, p) };
        let at = h.rebase(CH_EFFECT_PACKET_STORE_POLY);
        let r = h.call(at, &[first, second, cur]);
        let n = STRIP_COUNT.get(h);
        STRIP_COUNT.set(h, n.wrapping_add(1));
        r
    }
}
