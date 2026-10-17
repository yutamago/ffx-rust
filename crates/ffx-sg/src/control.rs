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
}
