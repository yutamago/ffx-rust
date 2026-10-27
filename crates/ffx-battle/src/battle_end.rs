//! The end of a battle (`docs/rewrite-plan.md`, M2): the `g_BtlStatus`
//! state machine `MsBattleProcess` runs each frame
//! (`MsBattleProcess_Status_007917d0`, `0x007917d0`) and its helpers: the
//! end checks, the closing camera and message, the victory fanfare, the
//! closing event effect and the fade.
//!
//! Other modules' functions (script, vfx, sound, the system's magic files)
//! go to the game's code through `call`/`s.call`. `FlushCache` and
//! `dbgPrintf` are empty in this build (`ret`); the ports leave them out.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::Script;
    use crate::exec::bridge::Call;
    use crate::menu::bridge::{END_FORCED, ms_get_battle_end_status};
    use crate::state::bridge::{EVENT_EFFECT_A, EVENT_EFFECT_B, EVENT_EFFECT_STAT, STATUS};
    use crate::target::bridge::{Dm, Host};

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = code(mem, sym);
        call(mem, f, args)
    }

    /// `op_ovl_effect_forced_free` (vfx).
    pub const OP_OVL_EFFECT_FORCED_FREE: u32 = 0x007f_cd50;
    /// `op_oef2_sep_free_all` (vfx).
    pub const OP_OEF2_SEP_FREE_ALL: u32 = 0x007f_ff30;
    /// `Sg_SetKeepFps` (system).
    pub const SG_SET_KEEP_FPS: u32 = 0x0082_1ae0;
    /// `Sg_FadeOut` (system).
    pub const SG_FADE_OUT: u32 = 0x0082_cbc0;
    /// `SndMusicRead` (sound).
    pub const SND_MUSIC_READ: u32 = 0x0088_6960;
    /// `SndSyncMusicStandby` (sound).
    pub const SND_SYNC_MUSIC_STANDBY: u32 = 0x0088_7900;
    /// `SndMusicReplayPointClear` (sound).
    pub const SND_MUSIC_REPLAY_POINT_CLEAR: u32 = 0x0088_69e0;
    /// `SndMusicStart` (sound).
    pub const SND_MUSIC_START: u32 = 0x0088_6a20;
    /// `SndMusicVolumeFadeOut(music, frames)` (sound).
    pub const SND_MUSIC_VOLUME_FADE_OUT: u32 = 0x0088_6af0;
    /// `TOAllCloseMessageWindow` (system).
    pub const TO_ALL_CLOSE_MESSAGE_WINDOW: u32 = 0x008a_b7b0;
    /// `AtelIsBattleEndStatusClear8` (script).
    pub const ATEL_IS_BATTLE_END_STATUS_CLEAR8: u32 = 0x0086_c280;
    /// `AtelIsBattleEndStatusClear9` (script).
    pub const ATEL_IS_BATTLE_END_STATUS_CLEAR9: u32 = 0x0086_c290;
    /// `AtelGetStatusWinFadeOut` (script).
    pub const ATEL_GET_STATUS_WIN_FADE_OUT: u32 = 0x0086_c4d0;
    /// `AtelBGSepFade(frames, 0)` (script).
    pub const ATEL_BG_SEP_FADE: u32 = 0x0086_1000;
    /// `op_et_battle_genko_counter_get` (vfx).
    pub const OP_ET_BATTLE_GENKO_COUNTER_GET: u32 = 0x007f_b240;

    /// Set: the event effect reads file 0 (`MsEventEffectRead`).
    pub const NO_EVENT_MAGIC_FILE: Global<u8> = Global::new(0x0112_a903);
    /// The event effect's file number (`MsEventEffectRead`).
    pub const EVENT_EFFECT_FILE: Global<u32> = Global::new(0x00c4_21e0);
    /// Cleared when an event effect read starts.
    pub const EVENT_EFFECT_CLEAR: Global<u32> = Global::new(0x00c4_21e4);
    /// The event effects started (`MsEventEffectStart`'s effect group).
    pub const EVENT_EFFECT_COUNT: Global<u32> = Global::new(0x0113_3358);
    /// What `Sg_SetKeepFps(1)` returned when an event effect started.
    pub const KEEP_FPS: Global<u8> = Global::new(0x0112_c9f9);

    /// The battle's end kind copied when the closing starts (the end
    /// status: 1 the party fell, 2 and 4 the monsters, 3 and 5 escape).
    pub const END_KIND: Global<i8> = Global::new(0x0112_a8e3);
    /// Frames of the closing fade.
    pub const END_FADE: Global<u8> = Global::new(0x0112_c9ec);
    /// The fade's next frames (`0x19`).
    pub const END_FADE_NEXT: Global<u8> = Global::new(0x0112_c9ee);
    /// The end's music (`0x8c` game over, `0x8a` victory fanfare).
    pub const END_MUSIC: Global<u16> = Global::new(0x0112_c94c);
    /// The closing event effect (script `0x711c`).
    pub const END_EFFECT: Global<u16> = Global::new(0x0112_c94e);
    /// Frames the screen fades out over (0: no fade), counted elsewhere.
    pub const END_FADE_OUT: Global<u8> = Global::new(0x0112_c9ed);
    /// The closing event effect: 1 to play, -1 played, 0 none.
    pub const END_EVENT_EFFECT: Global<i8> = Global::new(0x0112_c9ef);
    /// The frames for the fade out (`0x2d` or 10).
    pub const END_FADE_FRAMES: Global<u8> = Global::new(0x0112_c9f0);
    /// Set by a victory with the battle-end status bit 8 clear (fanfare).
    pub const END_VICTORY: Global<u8> = Global::new(0x0112_c9f2);
    /// Set while the closing waits for actors' effects and the vfx
    /// counter.
    pub const END_WAIT: Global<u8> = Global::new(0x0112_c9f3);
    /// Set by a game over with the battle-end status bit 9 clear.
    pub const END_GAME_OVER: Global<u8> = Global::new(0x0112_c9f4);
    /// Set: the fade out waits (`0x1f`).
    pub const END_HOLD: Global<u8> = Global::new(0x0112_ca01);
    /// Set (script `0x7118`): an aeon is not sent back at the end.
    pub const NO_SUMMON_RETURN: Global<u8> = Global::new(0x0112_ca11);
    /// The battle's pad buttons the frame before ([`PAD`], copied by
    /// `MsBattleProcess`); -1 when a game over plays its event effect, so
    /// that only a new press ends it.
    pub const PAD_OLD: Global<u32> = Global::new(0x0112_c908);
    /// The battle's pad buttons this frame (`MsBattleProcess`); the low
    /// byte's high nibble ends a game over's event effect, bit 5 closes a
    /// battle message.
    pub const PAD: Global<u8> = Global::new(0x0112_c904);

    /// `MsEffectForceFree(id)` (`0x00787ab0`): event effect 0x29f ends
    /// through the effect overlay table's `+0x20`
    /// (`MagicFile_GetEffectOverlayTable(0)`) with the event effect's ID
    /// ([`EVENT_EFFECT_B`]); any other through
    /// `op_ovl_effect_forced_free`. Their `eax`.
    pub fn ms_effect_force_free<M: Mem>(mem: &mut M, call: Call<'_, M>, id: u32) -> u32 {
        if id != 0x29f {
            return game(mem, call, OP_OVL_EFFECT_FORCED_FREE, &[]);
        }
        let t = game(
            mem,
            &mut *call,
            crate::parse::bridge::MAGIC_FILE_GET_EFFECT_OVERLAY_TABLE,
            &[0],
        );
        let f: u32 = mem.read(t.wrapping_add(0x20));
        let e = EVENT_EFFECT_B.get(mem);
        call(mem, f, &[e])
    }

    /// `MsEffectSepClearAll()` (`0x007882b0`): `op_oef2_sep_free_all(0xffff)`;
    /// its `eax`.
    pub fn ms_effect_sep_clear_all<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        game(mem, call, OP_OEF2_SEP_FREE_ALL, &[0xffff])
    }

    /// `MsEventEffectRead(id)` (`0x00788c60`): with no event effect
    /// ([`EVENT_EFFECT_STAT`] 0) the read of `id` starts
    /// ([`crate::misc::bridge::LAST_MAGIC2`]; file 0 with
    /// [`NO_EVENT_MAGIC_FILE`]): the CD library told 0x11 (its `+0x10`),
    /// [`EVENT_EFFECT_FILE`] its file number, the file set and loaded. A
    /// load to wait for (positive) makes the state 1, the asynchronous
    /// file or the overlay effect ([`EVENT_EFFECT_A`] the default table,
    /// `op_ovl_effect_init`) playing meanwhile, [`EVENT_EFFECT_CLEAR`]
    /// cleared. The state, sign-extended (1 for the asynchronous file).
    pub fn ms_event_effect_read<M: Mem>(mem: &mut M, call: Call<'_, M>, id: u32) -> i32 {
        use crate::parse::bridge as pa;
        if EVENT_EFFECT_STAT.get(mem) != 0 {
            return i32::from(EVENT_EFFECT_STAT.get(mem));
        }
        crate::misc::bridge::LAST_MAGIC2.set(mem, id);
        let file = if NO_EVENT_MAGIC_FILE.get(mem) == 0 {
            id
        } else {
            0
        };
        let cd = game(mem, &mut *call, pa::GET_CD, &[]);
        let f: u32 = mem.read(cd.wrapping_add(0x10));
        call(mem, f, &[0x11]);
        let n = crate::exec::bridge::ms_effect_file_number(mem, file.cast_signed(), -1);
        EVENT_EFFECT_FILE.set(mem, n.cast_unsigned());
        game(mem, &mut *call, pa::MAGIC_FILE_SET_MAGIC_ID, &[file]);
        if game(mem, &mut *call, pa::MAGIC_FILE_LOAD, &[]).cast_signed() < 1 {
            return i32::from(EVENT_EFFECT_STAT.get(mem));
        }
        if game(
            mem,
            &mut *call,
            pa::GRAPHIC_IS_MAGIC_FILE_ASYNC_LOAD_AVAILABLE,
            &[],
        ) != 0
        {
            EVENT_EFFECT_STAT.set(mem, 1);
            EVENT_EFFECT_CLEAR.set(mem, 0);
            return 1;
        }
        EVENT_EFFECT_STAT.set(mem, 1);
        let table = pa::EFFECT_OVERLAY_DEFAULT.ptr(mem).addr;
        EVENT_EFFECT_A.set(mem, table);
        game(mem, &mut *call, pa::OP_OVL_EFFECT_INIT, &[]);
        let st = EVENT_EFFECT_STAT.get(mem);
        EVENT_EFFECT_CLEAR.set(mem, 0);
        i32::from(st)
    }

    /// `MsEventEffectStart(b)` (`0x00788d60`): a read event effect (state
    /// 3) starts: [`KEEP_FPS`] what `Sg_SetKeepFps(1)` returns, its ID
    /// `MsGetEffID(`[`EVENT_EFFECT_COUNT`]`, 0x80, b)` in
    /// [`EVENT_EFFECT_B`], state 4, the effect table's first function
    /// ([`EVENT_EFFECT_A`]) called with it, the count up. The state,
    /// sign-extended.
    pub fn ms_event_effect_start<M: Mem>(mem: &mut M, call: Call<'_, M>, b: u32) -> i32 {
        let st = EVENT_EFFECT_STAT.get(mem);
        if st != 3 {
            return i32::from(st);
        }
        let keep = game(mem, &mut *call, SG_SET_KEEP_FPS, &[1]);
        KEEP_FPS.set(mem, keep.to_le_bytes()[0]);
        let id = crate::util::bridge::ms_get_eff_id(EVENT_EFFECT_COUNT.get(mem), 0x80, b);
        let table = EVENT_EFFECT_A.get(mem);
        EVENT_EFFECT_B.set(mem, id);
        EVENT_EFFECT_STAT.set(mem, 4);
        let f: u32 = mem.read(table);
        call(mem, f, &[id]);
        let st = EVENT_EFFECT_STAT.get(mem);
        let n = EVENT_EFFECT_COUNT.get(mem);
        EVENT_EFFECT_COUNT.set(mem, n.wrapping_add(1));
        i32::from(st)
    }

    /// `0x00793290()` (`MsSetEndDirAll`, name ours): every actor's end
    /// direction (`msSetEndDir`).
    pub fn ms_set_end_dir_all(mem: &mut impl Mem) {
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            crate::actor::bridge::ms_set_end_dir(mem, a);
        }
    }

    /// One system script request run as `MsAtelRequestSystemCheckExe`
    /// runs it: queued, run, its tags parsed when it ran.
    #[allow(clippy::many_single_char_names)] // as in the original
    fn system_request<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        owners: &[u32],
        n: u16,
        v: u32,
        args: [u32; 3],
    ) -> i32 {
        use crate::atel::bridge as at;
        use crate::state::bridge as st;
        st::ms_atel_request_init(mem);
        let v16 = u16::try_from(v & 0xffff).unwrap_or_default();
        for &i in owners {
            let i16 = u16::try_from(i).unwrap_or_default();
            st::ms_atel_request_add(mem, [2, i16, n, v16]);
        }
        let i = owners.last().copied().unwrap_or_default();
        let [flags, actor, out] = args;
        let r = at::ms_atel_request_exe(mem, s, 2, i, i32::from(n), flags, actor, out);
        if r >= 0 {
            at::ms_atel_parse_sys_tag(mem, s, 2, i, i32::from(n));
        }
        r
    }

    /// `MsAtelRequestSystemCheckExe(v, flags, actor, out)` (`0x007982a0`):
    /// system script handler 0x6c of owner 1 with `v`, then handler 0x89
    /// (with the voice event flag, `MsGetVoiceEventFlag`, queued for owners
    /// 1 and 0 and run for 0), each run (`MsAtelRequestExe`) and its tags
    /// parsed when it ran. The second's result.
    pub fn ms_atel_request_system_check_exe<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        v: u32,
        flags: u32,
        actor: u32,
        out: u32,
    ) -> i32 {
        let args = [flags, actor, out];
        system_request(mem, s, &[1], 0x6c, v, args);
        if crate::state::bridge::ms_get_voice_event_flag(mem) == 0 {
            system_request(mem, s, &[1], 0x89, v, args)
        } else {
            system_request(mem, s, &[1, 0], 0x89, v, args)
        }
    }

    /// `MsSEPbattleClear()` (`0x007a0370`): every sound in transfer
    /// ([`crate::minor::bridge::SEP_TRANS_TABLE`]) cleared (`SndSepClear`)
    /// and its slot emptied; the transfer flag's low byte cleared.
    pub fn ms_sep_battle_clear<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        use crate::minor::bridge::{SEP_TRANS_FLAG, SEP_TRANS_TABLE};
        let t = SEP_TRANS_TABLE.ptr(mem).addr;
        for k in 0..0x80u32 {
            let at = t + 4 * k;
            let v: u32 = mem.read(at);
            if v != 0 {
                game(mem, &mut *call, crate::exec::bridge::SND_SEP_CLEAR, &[v]);
                mem.write(at, 0u32);
            }
        }
        mem.write(SEP_TRANS_FLAG.ptr(mem).addr, 0u8);
    }

    /// `MsMusicReadStart(music)` (`0x0079fff0`): the battle's sounds
    /// cleared ([`ms_sep_battle_clear`], [`ms_effect_sep_clear_all`]), the
    /// music playing stopped and cleared, then `SndMusicRead(music)` (a
    /// tail jump: the original passes its own argument on; its `eax`).
    pub fn ms_music_read_start<M: Mem>(mem: &mut M, call: Call<'_, M>, music: u32) -> u32 {
        use crate::process::bridge as p;
        ms_sep_battle_clear(mem, &mut *call);
        ms_effect_sep_clear_all(mem, &mut *call);
        let m = game(
            mem,
            &mut *call,
            crate::exec::bridge::SND_GET_NOW_PLAY_MUSIC,
            &[],
        );
        game(mem, &mut *call, p::SND_MUSIC_STOP, &[m]);
        game(mem, &mut *call, p::SND_MUSIC_CLEAR, &[m]);
        game(mem, call, SND_MUSIC_READ, &[music])
    }

    /// `MsMusicSync(music)` (`0x007a0040`): once the music stands by
    /// (`SndSyncMusicStandby(0)` 0) the playing one is stopped and cleared
    /// and `music` starts from its beginning: -1. Else 0.
    pub fn ms_music_sync<M: Mem>(mem: &mut M, call: Call<'_, M>, music: u32) -> i32 {
        use crate::process::bridge as p;
        if game(mem, &mut *call, SND_SYNC_MUSIC_STANDBY, &[0]) != 0 {
            return 0;
        }
        let m = game(
            mem,
            &mut *call,
            crate::exec::bridge::SND_GET_NOW_PLAY_MUSIC,
            &[],
        );
        game(mem, &mut *call, p::SND_MUSIC_STOP, &[m]);
        game(mem, &mut *call, p::SND_MUSIC_CLEAR, &[m]);
        game(mem, &mut *call, SND_MUSIC_REPLAY_POINT_CLEAR, &[music]);
        game(mem, call, SND_MUSIC_START, &[music]);
        -1
    }

    /// `MsLimitTypeWinCheck()` (`0x007b1540`): each party actor 0-6 that
    /// can act (`+0xdc8`) earns overdrive experience of kind 0xb
    /// (`MsLimitTypeExpGet`); one whose overdrive mode (`+0x5bb`) is 0xb
    /// (victory) gains 20 (`MsLimitUp`). How many could act.
    pub fn ms_limit_type_win_check(mem: &mut impl Mem) -> u32 {
        let mut n = 0;
        for i in 0..7 {
            let a = ms_get_chr(mem, i);
            if byte(mem, a, 0xdc8) == 0 {
                continue;
            }
            n += 1;
            crate::exec::bridge::ms_limit_type_exp_get(mem, i, 0xb, 0);
            if byte(mem, a, 0x5bb) == 0xb {
                crate::turn::bridge::ms_limit_up(mem, i, a, 0x14);
            }
        }
        n
    }

    /// `MsDeathYoujinbou(i)` (`0x007b2ad0`): for actor 0xe (Yojimbo) the
    /// action request `MsActionRequest(0xe, 0xff, 0xb, 1, 2, 0)` and its
    /// result; else -1.
    pub fn ms_death_youjinbou<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
    ) -> i32 {
        if i != 0xe {
            return -1;
        }
        crate::action::bridge::ms_action_request(mem, host, s, 0xe, 0xff, 0xb, 1, 2, 0)
    }

    /// `SndIsSepPlay` (sound).
    pub const SND_IS_SEP_PLAY: u32 = 0x0088_6680;
    /// `SndSepStop` (sound).
    pub const SND_SEP_STOP: u32 = 0x0088_6eb0;
    /// `SndSepVolumeDirect(se, volume)` (sound).
    pub const SND_SEP_VOLUME_DIRECT: u32 = 0x0088_6f60;
    /// `SndSepPanDirect(se, pan)` (sound).
    pub const SND_SEP_PAN_DIRECT: u32 = 0x0088_6df0;
    /// `SndSyncWaveTrans` (sound).
    pub const SND_SYNC_WAVE_TRANS: u32 = 0x0088_7930;
    /// `SndStandbySepWave` (sound).
    pub const SND_STANDBY_SEP_WAVE: u32 = 0x0088_7550;
    /// `SndStandbyBattleWave(data, size)` (sound).
    pub const SND_STANDBY_BATTLE_WAVE: u32 = 0x0088_7450;
    /// `SndSepTransSepOnly(data, size)` (sound).
    pub const SND_SEP_TRANS_SEP_ONLY: u32 = 0x0088_6f30;
    /// `Sg_FreeA` (main).
    pub const SG_FREE_A: u32 = 0x0083_f090;

    /// The sound transfers queued (`MsSEPtransRegist`): 16 records of 12
    /// bytes (data, size, type: 0 effects only, else a wave; actor, 0xff
    /// the scene's; state: 0 queued, 1 sent, 2 wave standing by).
    pub const SEP_TRANS_QUEUE: Global<u32> = Global::new(0x0113_60c8);
    /// The scene's sound transfers still running.
    pub const SEP_TRANS_SCENE: Global<u8> = Global::new(0x0112_c02a);

    /// `MsEventEffectProcess()` (`0x00788c00`): an event effect that is
    /// done (state 6, a table in [`EVENT_EFFECT_A`]) ends: its table's
    /// second function called with 0, the script camera kept
    /// (`MsEffectCameraEnd(1)`), the overlay effect freed, state 0 and no
    /// table, the magic file stopped, the frame rate back
    /// (`Sg_SetKeepFps(`[`KEEP_FPS`]`)`, sign-extended).
    pub fn ms_event_effect_process<M: Mem>(mem: &mut M, s: &mut Script<'_, M>) {
        let table = EVENT_EFFECT_A.get(mem);
        if EVENT_EFFECT_STAT.get(mem) != 6 || table == 0 {
            return;
        }
        let f: u32 = mem.read(table.wrapping_add(4));
        (s.call)(mem, f, &[0]);
        crate::chr_damage::bridge::ms_effect_camera_end(mem, s, 1);
        game(
            mem,
            &mut *s.call,
            crate::damage_end::bridge::OP_OVL_EFFECT_FREE,
            &[],
        );
        EVENT_EFFECT_STAT.set(mem, 0);
        EVENT_EFFECT_A.set(mem, 0);
        game(
            mem,
            &mut *s.call,
            crate::damage_end::bridge::MAGIC_FILE_STOP,
            &[],
        );
        let keep = i32::from(KEEP_FPS.get(mem).cast_signed()).cast_unsigned();
        game(mem, &mut *s.call, SG_SET_KEEP_FPS, &[keep]);
    }

    /// `MsBtlReadSceneFree()` (`0x00783e00`): the scene's read buffer
    /// ([`crate::misc::bridge::BTL_READ3`]) freed (`Sg_FreeA(p, 5)`).
    pub fn ms_btl_read_scene_free<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let p = crate::misc::bridge::BTL_READ3.get(mem);
        if p != 0 {
            game(mem, call, SG_FREE_A, &[p, 5]);
            crate::misc::bridge::BTL_READ3.set(mem, 0);
        }
    }

    /// `TOBtlDrawStdChrNameMessageWindow(actor, message)` (system).
    pub const TO_BTL_DRAW_STD_CHR_NAME_MESSAGE_WINDOW: u32 = 0x0089_7200;
    /// `TOBtlDrawFirstStrikePlayerMessageWindow()` (system).
    pub const TO_BTL_DRAW_FIRST_STRIKE_PLAYER_MESSAGE_WINDOW: u32 = 0x0089_34f0;
    /// `TOBtlDrawFirstStrikeEnemyMessageWindow()` (system).
    pub const TO_BTL_DRAW_FIRST_STRIKE_ENEMY_MESSAGE_WINDOW: u32 = 0x0089_34d0;
    /// `TOBtlDrawGetItemMessageWindow(item, count)` (system).
    pub const TO_BTL_DRAW_GET_ITEM_MESSAGE_WINDOW: u32 = 0x0089_3510;
    /// `TOBtlDrawCaptureMonsterMessageWindow(a, b)` (system).
    pub const TO_BTL_DRAW_CAPTURE_MONSTER_MESSAGE_WINDOW: u32 = 0x0089_2870;
    /// `TOBtlDrawLearningMessageWindow(a, b)` (system).
    pub const TO_BTL_DRAW_LEARNING_MESSAGE_WINDOW: u32 = 0x0089_5320;
    /// `TOBtlDrawGetLimitTypeMessageWindow(a, b)` (system).
    pub const TO_BTL_DRAW_GET_LIMIT_TYPE_MESSAGE_WINDOW: u32 = 0x0089_35f0;
    /// `TOBtlDrawGetMoneyMessageWindow(gil)` (system).
    pub const TO_BTL_DRAW_GET_MONEY_MESSAGE_WINDOW: u32 = 0x0089_3660;
    /// `TOBtlCloseSimpleHelpMes()` (system).
    pub const TO_BTL_CLOSE_SIMPLE_HELP_MES: u32 = 0x0089_0ef0;

    /// The message window on screen (0 none; negative: closed at once).
    pub const MESSAGE_WINDOW: Global<i32> = Global::new(0x0112_c910);
    /// The shown message's frames left, then the frames before a button
    /// may close it (two `u8`).
    pub const MESSAGE_TIME: Global<[u8; 2]> = Global::new(0x0112_c946);

    /// `MsMessageCueProcess()` (`0x0079cdf0`): the battle messages queued
    /// ([`crate::state::bridge::MESSAGE_CUE`], a ring of 4): the shown
    /// one's frames count down ([`MESSAGE_TIME`]); with none shown the
    /// next opens (by kind 1-8 its `TOBtlDraw*MessageWindow`; 6 and 7 with
    /// sound 0x39), its frames and button delay from `+2` and `+1`; a
    /// shown one stays while its frames last, unless a button (pad bit 5,
    /// newly pressed) comes after the delay; then the help closes and the
    /// next goes on, in the same call. The messages left.
    #[allow(clippy::many_single_char_names)] // entry, its words, the call
    pub fn ms_message_cue_process<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        call: Call<'_, M>,
    ) -> u32 {
        use crate::state::bridge::{MESSAGE_CUE, MESSAGE_CUE_COUNT, MESSAGE_CUE_START};
        let [mut left, mut delay] = MESSAGE_TIME.get(mem);
        if left != 0 {
            left -= 1;
            MESSAGE_TIME.set(mem, [left, delay]);
        }
        if MESSAGE_CUE_COUNT.get(mem) == 0 {
            return 0;
        }
        let mut win = MESSAGE_WINDOW.get(mem);
        loop {
            if win == 0 {
                let k = u32::from(MESSAGE_CUE_START.get(mem) & 3);
                let e = MESSAGE_CUE.ptr(mem).addr.wrapping_add(12 * k);
                let (x, y): (u32, u32) = (mem.read(e.wrapping_add(4)), mem.read(e.wrapping_add(8)));
                let kind = byte(mem, e, 0);
                let draw = match kind {
                    1 => Some((TO_BTL_DRAW_STD_CHR_NAME_MESSAGE_WINDOW, 2)),
                    2 => Some((TO_BTL_DRAW_FIRST_STRIKE_PLAYER_MESSAGE_WINDOW, 0)),
                    3 => Some((TO_BTL_DRAW_FIRST_STRIKE_ENEMY_MESSAGE_WINDOW, 0)),
                    4 => Some((TO_BTL_DRAW_GET_ITEM_MESSAGE_WINDOW, 2)),
                    5 => Some((TO_BTL_DRAW_CAPTURE_MONSTER_MESSAGE_WINDOW, 2)),
                    6 => Some((TO_BTL_DRAW_LEARNING_MESSAGE_WINDOW, 2)),
                    7 => Some((TO_BTL_DRAW_GET_LIMIT_TYPE_MESSAGE_WINDOW, 2)),
                    8 => Some((TO_BTL_DRAW_GET_MONEY_MESSAGE_WINDOW, 1)),
                    _ => None,
                };
                win = match draw {
                    Some((f, n)) => {
                        if kind == 6 || kind == 7 {
                            crate::after_damage::bridge::ms_reg_se_play(
                                mem, trig, &mut *call, 0xff, 0x39,
                            );
                        }
                        game(mem, &mut *call, f, &[x, y][..n]).cast_signed()
                    }
                    None => -1,
                };
                MESSAGE_WINDOW.set(mem, win);
                left = byte(mem, e, 2);
                delay = byte(mem, e, 1);
                MESSAGE_TIME.set(mem, [left, delay]);
            } else {
                if win > 0 {
                    let pressed = PAD.get(mem) & 0x20 != 0
                        && PAD_OLD.get(mem) & 0x20 == 0
                        && left.cast_signed() < delay.cast_signed();
                    if !pressed && left != 0 {
                        return u32::from(MESSAGE_CUE_COUNT.get(mem));
                    }
                    game(mem, &mut *call, TO_BTL_CLOSE_SIMPLE_HELP_MES, &[]);
                }
                let start = MESSAGE_CUE_START.get(mem);
                win = 0;
                left = 0;
                delay = 0;
                MESSAGE_WINDOW.set(mem, 0);
                MESSAGE_TIME.set(mem, [0, 0]);
                MESSAGE_CUE_COUNT.set(mem, MESSAGE_CUE_COUNT.get(mem).wrapping_sub(1));
                MESSAGE_CUE_START.set(mem, start.wrapping_add(1) & 3);
            }
            if MESSAGE_CUE_COUNT.get(mem) == 0 {
                return 0;
            }
        }
    }

    /// `MsSEPtransProcess()` (`0x007a0540`): with transfers running
    /// ([`crate::state::bridge::SEP_TRANS`] above 0) the current record
    /// ([`SEP_TRANS_QUEUE`] at the index in
    /// [`crate::minor::bridge::SEP_TRANS_STATE`]) goes on: a queued one
    /// is sent (state 1; type 0 `SndSepTransSepOnly`, else
    /// `SndStandbyBattleWave`, with its data and size). A sent one, once
    /// the transfer is done (`SndSyncWaveTrans(0)` 0): a wave's goes to
    /// standing by (state 2, `SndStandbySepWave`); else it ends: one
    /// transfer fewer, its actor's count `+0xf2d` down (at 0 `+0xf2c` set)
    /// or for the scene's ([`SEP_TRANS_SCENE`] down, at 0 the transfer
    /// flag set and the scene's buffer freed), the index on (mod 16). -1;
    /// with none running 0. (`dbgPrintf`, empty, gets the record's first
    /// data byte: the original reads it.)
    #[allow(clippy::many_single_char_names)] // record, index, actor
    pub fn ms_sep_trans_process<M: Mem>(mem: &mut M, call: Call<'_, M>) -> i32 {
        use crate::state::bridge::SEP_TRANS;
        if SEP_TRANS.get(mem).cast_signed() <= 0 {
            return 0;
        }
        let k = crate::minor::bridge::SEP_TRANS_STATE.get(mem)[0];
        let r = SEP_TRANS_QUEUE
            .ptr(mem)
            .addr
            .wrapping_add(k.wrapping_mul(12));
        let state: u8 = mem.read(r.wrapping_add(0xa));
        if state == 0 {
            let kind: u8 = mem.read(r.wrapping_add(8));
            let data: u32 = mem.read(r);
            mem.write(r.wrapping_add(0xa), 1u8);
            let _: u8 = mem.read(data);
            let size: u32 = mem.read(r.wrapping_add(4));
            let f = if kind == 0 {
                SND_SEP_TRANS_SEP_ONLY
            } else {
                SND_STANDBY_BATTLE_WAVE
            };
            let data: u32 = mem.read(r);
            game(mem, call, f, &[data, size]);
            return -1;
        }
        if game(mem, &mut *call, SND_SYNC_WAVE_TRANS, &[0]) != 0 {
            return -1;
        }
        let kind: u8 = mem.read(r.wrapping_add(8));
        if kind == 1 && mem.read::<u8>(r.wrapping_add(0xa)) == 1 {
            mem.write(r.wrapping_add(0xa), 2u8);
            game(mem, call, SND_STANDBY_SEP_WAVE, &[]);
            return -1;
        }
        SEP_TRANS.set(mem, SEP_TRANS.get(mem).wrapping_sub(1));
        let i: u8 = mem.read(r.wrapping_add(9));
        if i == 0xff {
            let n = SEP_TRANS_SCENE.get(mem).wrapping_sub(1);
            SEP_TRANS_SCENE.set(mem, n);
            if n == 0 {
                mem.write(crate::minor::bridge::SEP_TRANS_FLAG.ptr(mem).addr, 1u8);
                ms_btl_read_scene_free(mem, &mut *call);
            }
        } else {
            let a = ms_get_chr(mem, u32::from(i));
            let n = byte(mem, a, 0xf2d).wrapping_sub(1);
            mem.write(a.wrapping_add(0xf2d), n);
            if n == 0 {
                mem.write(a.wrapping_add(0xf2c), 1u8);
            }
        }
        let k = crate::minor::bridge::SEP_TRANS_STATE.get(mem)[0];
        let at = crate::minor::bridge::SEP_TRANS_STATE.ptr(mem).addr;
        // (the original's signed `% 16`)
        mem.write(at, (k.wrapping_add(1).cast_signed() % 16).cast_unsigned());
        -1
    }

    /// `MsSEP3Dprocess()` (`0x007a0930`): each 3D sound
    /// ([`crate::minor::bridge::SEP_3D`]) counts its delay (`+3`) down;
    /// after it, one no longer playing (`SndIsSepPlay(+8)` 0) is cleared
    /// (`+0`) and unlinked; a playing one's volume (0x100, or while fading
    /// `+6` frames of `+4` left: `(+6 << 8) / +4`, at 0 the sound stopped)
    /// and pan follow its actor (`MsSEP3DPos(+1, +2, +0x10)`,
    /// `op_3d_vol_pan_get`). A node linked to itself is cut. The nodes
    /// walked.
    pub fn ms_sep_3d_process<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        call: Call<'_, M>,
        vol_pan: &mut dyn FnMut(&mut M, u32, u32) -> (u32, u32),
    ) -> u32 {
        let mut link = crate::minor::bridge::SEP_3D.ptr(mem).addr;
        let mut node: u32 = mem.read(link);
        let mut n = 0u32;
        while node != 0 {
            let delay = byte(mem, node, 3);
            if delay != 0 {
                mem.write(node.wrapping_add(3), delay - 1);
                link = node.wrapping_add(0xc);
            } else {
                let se: u32 = mem.read(node.wrapping_add(8));
                if game(mem, &mut *call, SND_IS_SEP_PLAY, &[se]) == 0 {
                    mem.write(node, 0u8);
                    let next: u32 = mem.read(node.wrapping_add(0xc));
                    mem.write(link, next);
                } else {
                    let total: i16 = mem.read(node.wrapping_add(4));
                    let mut vol = 0x100u32;
                    if total != 0 {
                        let left = mem.read::<i16>(node.wrapping_add(6)).wrapping_sub(1);
                        mem.write(node.wrapping_add(6), left);
                        if left > 0 {
                            vol = ((i32::from(left) << 8) / i32::from(total)).cast_unsigned();
                        } else {
                            vol = 0;
                            game(mem, &mut *call, SND_SEP_STOP, &[se]);
                        }
                    }
                    let (kind, i) = (byte(mem, node, 1), byte(mem, node, 2));
                    crate::after_damage::bridge::ms_sep_3d_pos(
                        mem,
                        trig,
                        u32::from(kind),
                        u32::from(i),
                        node.wrapping_add(0x10),
                    );
                    let (pan, volume) = vol_pan(mem, node.wrapping_add(0x10), vol);
                    game(mem, &mut *call, SND_SEP_VOLUME_DIRECT, &[se, volume]);
                    game(mem, &mut *call, SND_SEP_PAN_DIRECT, &[se, pan]);
                    link = node.wrapping_add(0xc);
                }
            }
            node = mem.read(node.wrapping_add(0xc));
            n += 1;
            if node != 0 && node == mem.read::<u32>(node.wrapping_add(0xc)) {
                mem.write(node.wrapping_add(0xc), 0u32);
            }
        }
        n
    }

    /// The waits before the closing: nothing being read or streamed, no
    /// shout voice, no system message.
    fn sound_idle<M: Mem>(mem: &mut M, call: Call<'_, M>) -> bool {
        use crate::process::bridge as p;
        game(mem, &mut *call, p::SND_IS_READ_STATUS, &[]) == 0
            && game(mem, &mut *call, p::SND_GET_STREAM_STATE, &[]) == 0
            && game(mem, &mut *call, p::SND_IS_READ_SHOUT_VOICE_STATUS, &[]) == 0
            && crate::exec::bridge::ms_system_message_check(mem, call) == 0
    }

    /// Where the closing's event-effect step goes on (`0x00791e07`,
    /// `0x00791e14`, `0x00791e21`).
    #[derive(PartialEq, Eq)]
    enum Next {
        Wait,
        Fade,
        Close,
    }

    /// `MsBattleProcess_Status_007917d0()`: the battle-end steps by
    /// [`STATUS`]:
    ///
    /// - 1: the end status (`MsGetBattleEndStatus`) set: 2.
    /// - 2: no command queued or numbers shown: with an aeon out
    ///   ([`crate::target::bridge::SUMMONER`], not [`NO_SUMMON_RETURN`])
    ///   and the party gone (1) or unable to act (3), its summoner's
    ///   revival time (`MsSetPartyRelifeTime`, `MsDeathYoujinbou`) and,
    ///   for an aeon whose `+0x702` is 1, a command inserted for it
    ///   (`+0xdee`, kind 2), else the party back (`MsReturnSummon(0)`,
    ///   `MsCommandReturnChrStatusAll(0)`): 1. No end: 1. Else 0x17.
    /// - 0x17: no sound read, stream, shout voice or system message: the
    ///   end status kept ([`crate::menu::bridge::END_FORCED`]), 0x18, the
    ///   system script's check 4.
    /// - 0x18: no script camera and the same waits: the message windows
    ///   closed; a game over (1, battle-end bit 9 clear) fades over 0x1e
    ///   frames then 0x14 with music 0x8c ([`END_GAME_OVER`]), a victory
    ///   (2, bit 8 clear) over 10 then 0xc with 0x8a ([`END_VICTORY`]):
    ///   0x19. Else 0x1b, fading over 0x1e frames for 4, 0 otherwise.
    /// - 0x19: the music fades out: 0x1a. 0x1a: the end's music read
    ///   (`MsMusicReadStart`): 0x1b. 0x1c: it plays (`MsMusicSync`): 0x1d.
    /// - 0x1b: actors back to standing; an actor still moving, turning or
    ///   changing (`+0x508`, `+0x42e`, `+0x415`, `+0x41c` with `+0x41d`)
    ///   is waited on ([`crate::flow::bridge::EFFECT_WAIT`] it and 0xfd).
    ///   Else 0x1c with a fanfare, else on to 0x1d.
    /// - 0x1d: 0x1e: the end directions, the effect sounds cleared, the
    ///   kind's system script check and camera, the fade and its frames,
    ///   overdrive experience (`MsLimitTypeWinCheck`); a game over plays
    ///   its event effect without plain aeons.
    /// - 0x1e: the waits on actors' effects and the vfx counter, the
    ///   victory motion 0x2d, the 3D sounds faded, the event effect read
    ///   and started, then the fade out: 0x1f.
    /// - 0x1f: the fade done: the event effect freed, 0x20, or 0x21.
    /// - 0x20: no event effect: 0x21.
    #[allow(clippy::too_many_lines)]
    pub fn ms_battle_process_status<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
    ) {
        match STATUS.get(mem) {
            1 => {
                if ms_get_battle_end_status(mem) != 0 {
                    STATUS.set(mem, 2);
                }
            }
            2 => {
                if crate::state::bridge::COMMAND_CUE_COUNT.get(mem) != 0
                    || crate::minor::bridge::NUMBERS_SHOWN.get(mem) != 0
                {
                    return;
                }
                let end = ms_get_battle_end_status(mem);
                let summoner = crate::target::bridge::SUMMONER.get(mem);
                if summoner != 0 && NO_SUMMON_RETURN.get(mem) == 0 && (end == 1 || end == 3) {
                    let i =
                        crate::util::bridge::ms_check_megas(i32::from(summoner)).cast_unsigned();
                    let a = ms_get_chr(mem, i);
                    crate::minor::bridge::ms_set_party_relife_time(mem);
                    ms_death_youjinbou(mem, host, s, i);
                    if byte(mem, a, 0x702) == 1 {
                        let t = i32::from(byte(mem, a, 0xdee).cast_signed()).cast_unsigned();
                        crate::action::bridge::ms_insert_chr_command(mem, i, t, 2);
                    } else {
                        crate::damage_end::bridge::ms_return_summon(mem, host, s, 0);
                        crate::chr_damage::bridge::ms_command_return_chr_status_all(
                            mem, host, s, 0,
                        );
                    }
                    STATUS.set(mem, 1);
                    return;
                }
                if end == 0 {
                    STATUS.set(mem, 1);
                    return;
                }
                STATUS.set(mem, 0x17);
                end_wait(mem, s);
            }
            0x17 => end_wait(mem, s),
            0x18 => end_close(mem, s),
            0x19 => {
                let next = END_FADE_NEXT.get(mem);
                END_FADE.set(mem, next);
                STATUS.set(mem, 0x1a);
                let m = game(
                    mem,
                    &mut *s.call,
                    crate::exec::bridge::SND_GET_NOW_PLAY_MUSIC,
                    &[],
                );
                game(
                    mem,
                    &mut *s.call,
                    SND_MUSIC_VOLUME_FADE_OUT,
                    &[m, u32::from(next)],
                );
            }
            0x1a => {
                END_FADE.set(mem, 10);
                let music = u32::from(END_MUSIC.get(mem));
                ms_music_read_start(mem, &mut *s.call, music);
                STATUS.set(mem, 0x1b);
            }
            0x1b => {
                for i in 0..0x1f {
                    let a = ms_get_chr(mem, i);
                    if crate::chr_damage::bridge::ms_check_rest_motion(mem, i, a) != 0 {
                        crate::atel::bridge::ms_set_motion(mem, s, i, 0, 0, 0, 1, 0, 0);
                    }
                    let b = |o| byte(mem, a, o) != 0;
                    if !b(0xdce)
                        && b(0x15)
                        && (b(0x508) || b(0x42e) || b(0x415) || (b(0x41c) && b(0x41d)))
                    {
                        crate::flow::bridge::EFFECT_WAIT.set(mem, [i.to_le_bytes()[0], 0xfd]);
                        return;
                    }
                }
                if END_VICTORY.get(mem) != 0 || END_GAME_OVER.get(mem) != 0 {
                    STATUS.set(mem, 0x1c);
                    return;
                }
                end_start(mem, s);
            }
            0x1c => {
                let music = u32::from(END_MUSIC.get(mem));
                if ms_music_sync(mem, &mut *s.call, music) != 0 {
                    STATUS.set(mem, 0x1d);
                }
            }
            0x1d => end_start(mem, s),
            0x1e => end_effect(mem, s),
            0x1f => end_fade(mem, s),
            0x20 if crate::state::bridge::ms_event_effect_get_stat(mem) == 0 => {
                STATUS.set(mem, 0x21);
            }
            _ => {}
        }
    }

    /// 0x17 (`0x007918e5`).
    fn end_wait<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>) {
        if !sound_idle(mem, &mut *s.call) {
            return;
        }
        let end = ms_get_battle_end_status(mem);
        STATUS.set(mem, 0x18);
        END_FORCED.set(mem, end.to_le_bytes()[0]);
        ms_atel_request_system_check_exe(mem, s, 4, 1, 0, 0);
        end_close(mem, s);
    }

    /// 0x18 (`0x0079193b`).
    fn end_close<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>) {
        if crate::atel::bridge::CAMERA_STATE.get(mem) != 0 || !sound_idle(mem, &mut *s.call) {
            return;
        }
        game(mem, &mut *s.call, TO_ALL_CLOSE_MESSAGE_WINDOW, &[]);
        let (fade, next, music) = match END_FORCED.get(mem) {
            1 if game(mem, &mut *s.call, ATEL_IS_BATTLE_END_STATUS_CLEAR9, &[]) != 0 => {
                END_GAME_OVER.set(mem, 1);
                (0x1e, 0x14, 0x8c)
            }
            2 if game(mem, &mut *s.call, ATEL_IS_BATTLE_END_STATUS_CLEAR8, &[]) == 1 => {
                END_VICTORY.set(mem, 1);
                (10, 0xc, 0x8a)
            }
            4 => {
                END_FADE.set(mem, 0x1e);
                STATUS.set(mem, 0x1b);
                return;
            }
            _ => {
                END_FADE.set(mem, 0);
                STATUS.set(mem, 0x1b);
                return;
            }
        };
        END_FADE.set(mem, fade);
        END_FADE_NEXT.set(mem, next);
        END_MUSIC.set(mem, music);
        STATUS.set(mem, 0x19);
    }

    /// 0x1d (`0x00791b88`).
    fn end_start<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>) {
        let kind = END_FORCED.get(mem);
        STATUS.set(mem, 0x1e);
        END_KIND.set(mem, kind.cast_signed());
        ms_set_end_dir_all(mem);
        ms_effect_sep_clear_all(mem, &mut *s.call);
        let kind = END_KIND.get(mem);
        let (mut fade_out, frames, camera, check) = match kind {
            1 => (0x3c, 0x2d, 0x43, Some(1)),
            2 => (0x58, 10, 0x42, Some(0)),
            3 | 5 => (0, 10, 0x44, Some(2)),
            4 => (0, 10, 0x45, Some(3)),
            _ => (0, 10, -1i32, None),
        };
        if let Some(v) = check {
            ms_atel_request_system_check_exe(mem, s, v, 1, 0, 0);
        }
        if game(mem, &mut *s.call, ATEL_GET_STATUS_WIN_FADE_OUT, &[]) == 0
            && END_GAME_OVER.get(mem) == 0
        {
            END_WAIT.set(mem, 1);
            if END_KIND.get(mem) != 2 || END_VICTORY.get(mem) == 0 {
                fade_out = 0;
            }
        } else if camera >= 0 {
            crate::atel::bridge::ms_atel_request_camera(
                mem,
                s,
                0,
                0xff,
                camera.cast_unsigned(),
                0xffff,
                1,
                1,
                0,
                0,
            );
        }
        END_FADE.set(mem, 0);
        END_FADE_OUT.set(mem, fade_out);
        END_FADE_FRAMES.set(mem, frames);
        ms_limit_type_win_check(mem);
        if END_GAME_OVER.get(mem) != 0 && crate::action::bridge::NO_MAGIC_EFFECT.get(mem) == 0 {
            END_EVENT_EFFECT.set(mem, 1);
            PAD_OLD.set(mem, u32::MAX);
        }
        end_effect(mem, s);
    }

    /// 0x1e (`0x00791c8d`).
    #[allow(clippy::too_many_lines)]
    fn end_effect<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>) {
        use crate::atel::bridge::CAMERA_STATE;
        use crate::parse::bridge::SND_GET_STREAM_STATE;
        use crate::state::bridge::ms_stone_effect_check;
        if END_WAIT.get(mem) != 0 {
            let busy = (0..0x1f).any(|i| {
                let a = ms_get_chr(mem, i);
                byte(mem, a, 0x432) != 0 || byte(mem, a, 0x436) != 0
            });
            if !busy && game(mem, &mut *s.call, OP_ET_BATTLE_GENKO_COUNTER_GET, &[]) == 0 {
                END_WAIT.set(mem, 0);
            }
        }
        if END_KIND.get(mem) == 2 && END_VICTORY.get(mem) != 0 {
            for i in 0..0x12 {
                let a = ms_get_chr(mem, i);
                let b = |o| byte(mem, a, o) != 0;
                if b(0xdc8)
                    && b(0xdd2)
                    && b(0x43b)
                    && !b(0x43a)
                    && !b(0xdce)
                    && !b(0x608)
                    && !b(0xdcc)
                {
                    mem.write(a.wrapping_add(0x43a), 1u8);
                    crate::atel::bridge::ms_set_motion(mem, s, i, 0x2d, 0, 0, 1, 0, 0);
                }
            }
        }
        if END_FADE_OUT.get(mem) == 0 {
            crate::after_damage::bridge::SEP_3D_OFF.set(mem, 1);
            crate::minor::bridge::ms_sep3d_fade_out_all(mem, 10);
        }
        let next = if END_EVENT_EFFECT.get(mem) == 0 {
            Next::Wait
        } else {
            match crate::state::bridge::ms_event_effect_get_stat(mem) {
                0 => {
                    if CAMERA_STATE.get(mem) == 0
                        && game(mem, &mut *s.call, SND_GET_STREAM_STATE, &[]) == 0
                        && ms_stone_effect_check(mem) == 0
                        && crate::exec::bridge::ms_system_message_check(mem, &mut *s.call) == 0
                    {
                        let id = if crate::parse::bridge::NO_MAGIC_FILE.get(mem) == 0 {
                            u32::from(END_EFFECT.get(mem))
                        } else {
                            0
                        };
                        ms_event_effect_read(mem, &mut *s.call, id);
                    }
                    Next::Wait
                }
                3 => {
                    ms_event_effect_start(mem, &mut *s.call, 0);
                    END_FADE_OUT.set(mem, 0x3c);
                    return;
                }
                5 => {
                    if END_FADE_OUT.get(mem) != 0 {
                        return;
                    }
                    if PAD_OLD.get(mem) != 0 || PAD.get(mem) & 0xf0 == 0 {
                        Next::Fade
                    } else {
                        END_EVENT_EFFECT.set(mem, -1);
                        Next::Close
                    }
                }
                _ => Next::Wait,
            }
        };
        if next == Next::Wait && END_FADE_OUT.get(mem) != 0 {
            return;
        }
        if next != Next::Close && END_EVENT_EFFECT.get(mem) > 0 {
            return;
        }
        if END_WAIT.get(mem) != 0
            || CAMERA_STATE.get(mem) != 0
            || crate::flow::bridge::LIMIT_LEARNT.get(mem) != 0
            || game(mem, &mut *s.call, SND_GET_STREAM_STATE, &[]) != 0
            || crate::exec::bridge::ms_system_message_check(mem, &mut *s.call) != 0
            || ms_stone_effect_check(mem) != 0
        {
            return;
        }
        if game(mem, &mut *s.call, ATEL_GET_STATUS_WIN_FADE_OUT, &[]) == 0
            && END_GAME_OVER.get(mem) == 0
        {
            STATUS.set(mem, 0x1f);
        } else {
            let frames = END_FADE_FRAMES.get(mem);
            END_FADE_OUT.set(mem, frames);
            STATUS.set(mem, 0x1f);
            game(mem, &mut *s.call, SG_FADE_OUT, &[u32::from(frames)]);
            if END_GAME_OVER.get(mem) != 0 {
                let frames = u32::from(END_FADE_FRAMES.get(mem));
                game(mem, &mut *s.call, ATEL_BG_SEP_FADE, &[frames, 0]);
                let frames = u32::from(END_FADE_FRAMES.get(mem));
                let m = game(
                    mem,
                    &mut *s.call,
                    crate::exec::bridge::SND_GET_NOW_PLAY_MUSIC,
                    &[],
                );
                game(mem, &mut *s.call, SND_MUSIC_VOLUME_FADE_OUT, &[m, frames]);
            }
        }
        end_fade(mem, s);
    }

    /// 0x1f (`0x00791ed7`).
    fn end_fade<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>) {
        if END_FADE_OUT.get(mem) != 0
            || END_HOLD.get(mem) != 0
            || game(
                mem,
                &mut *s.call,
                crate::parse::bridge::SND_GET_STREAM_STATE,
                &[],
            ) != 0
            || crate::exec::bridge::ms_system_message_check(mem, &mut *s.call) != 0
        {
            return;
        }
        if END_EVENT_EFFECT.get(mem) != 0 {
            let id = u32::from(END_EFFECT.get(mem));
            ms_effect_force_free(mem, &mut *s.call, id);
            STATUS.set(mem, 0x20);
            return;
        }
        STATUS.set(mem, 0x21);
    }

    /// The results screen's weapon spoils: how many (`i32`), the list's
    /// height in 1/4096 rows (`i32`, scaled by the remapped 66.0 for a
    /// row) and the weapon IDs (`u16`, a pointer).
    pub const SPOIL_WEAPON_COUNT: Global<i32> = Global::new(0x0186_a188);
    pub const SPOIL_WEAPON_ROWS: Global<i32> = Global::new(0x0186_a18c);
    pub const SPOIL_WEAPONS: Global<u32> = Global::new(0x025c_c580);
    /// `TODrawMenuPlateXYWHType(x, y, w, h, kind)` and
    /// `DrawCrossMenuIconWeaponName(id, x, y, equippable)`.
    pub const TO_DRAW_MENU_PLATE_XYWH_TYPE: u32 = 0x008f_5f70;
    pub const DRAW_CROSS_MENU_ICON_WEAPON_NAME: u32 = 0x008e_67f0;

    /// Which `graphicUiRemap*2` a call means: x (`0x006447e0`) or y
    /// (`0x00644820`).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Remap {
        X2,
        Y2,
    }

    /// `BtlEnd_DrawWeaponPlate_008be0d0(n, x, y, fixed)` (`0x008be0d0`,
    /// name ours): weapon spoil `n` (of [`SPOIL_WEAPON_COUNT`]) drawn at
    /// `x`, `y` (`y` without `fixed` moved down by the list's rows: the
    /// remapped 66.0 times [`SPOIL_WEAPON_ROWS`] (an `f32`) / 4096 plus
    /// `y`, truncated) when its weapon (`MsGetSaveWeapon`, the name into
    /// `local`, the original's stack) is held (`+2`): a plate of the
    /// remapped width 740.0 and height 60.0, kind 5, and its icon and
    /// name, equippable when `+6` is not 0xff.
    #[allow(clippy::too_many_arguments)]
    pub fn btl_end_draw_weapon_plate<M: Mem>(
        mem: &mut M,
        remap: &dyn Fn(Remap, f32) -> f32,
        call: Call<'_, M>,
        n: u32,
        x: u32,
        y: u32,
        fixed: u32,
        local: u32,
    ) {
        use ffx_bridge::x87::{add, load, mul, store};
        let n = n.cast_signed();
        if n >= SPOIL_WEAPON_COUNT.get(mem) || n < 0 {
            return;
        }
        let list = SPOIL_WEAPONS.get(mem);
        let id = u32::from(mem.read::<u16>(list.wrapping_add(n.cast_unsigned().wrapping_mul(2))));
        let int = |v: u32| load(store(f64::from(v.cast_signed())));
        let y = if fixed == 0 {
            let row = f64::from(remap(Remap::Y2, 66.0));
            let rows = int(SPOIL_WEAPON_ROWS.get(mem).cast_unsigned());
            let v = add(mul(mul(row, rows), 1.0 / 4096.0), int(y));
            ffx_bridge::x87::ftol(mem, v)
        } else {
            y
        };
        let weapon = crate::save::bridge::ms_get_save_weapon(mem, id, local);
        if mem.read::<u8>(weapon.wrapping_add(2)) == 0 {
            return;
        }
        let (xf, yf) = (store(int(x)), store(int(y)));
        let height = remap(Remap::Y2, 60.0).to_bits();
        let width = remap(Remap::X2, 740.0).to_bits();
        let plate = code(mem, TO_DRAW_MENU_PLATE_XYWH_TYPE);
        call(mem, plate, &[xf, yf, width, height, 5]);
        let equippable = u32::from(mem.read::<u8>(weapon.wrapping_add(6)) != 0xff);
        let name = code(mem, DRAW_CROSS_MENU_ICON_WEAPON_NAME);
        call(mem, name, &[id, xf, yf, equippable]);
    }
}
