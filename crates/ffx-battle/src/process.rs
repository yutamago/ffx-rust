//! The battle loop's per-frame pieces (`docs/rewrite-plan.md`, M2):
//! what `MsBattleProcess` (`0x00790c10`) checks and runs each frame
//! besides the command cue ([`crate::parse`]).
//!
//! Sound, the CD library and the voice status are other modules'; those go
//! to the game's code through the caller's `call`.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::exec::bridge::Call;

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    /// Counts down each frame; the battle waits while it runs.
    pub const PAUSE: Global<u8> = Global::new(0x0112_c9d0);
    /// Set while an actor plays a hit motion (`MsCheckChrHitMotionAll`).
    pub const HIT_MOTION: Global<u8> = Global::new(0x0112_c9ea);
    /// Not 0: a soft reset waits.
    pub const RESET_WAIT: Global<u32> = Global::new(0x0112_c024);
    /// Not 0: a soft reset waits.
    pub const RESET_WAIT2: Global<u8> = Global::new(0x0112_c9d6);

    /// `SndGetNowPlayMusic`, `SndMusicStop`, `SndMusicClear` (sound).
    pub const SND_GET_NOW_PLAY_MUSIC: u32 = 0x0088_63f0;
    pub const SND_MUSIC_STOP: u32 = 0x0088_6a60;
    pub const SND_MUSIC_CLEAR: u32 = 0x0088_6800;
    /// The voice status's sound and CD calls (`msVoiceCheckStanby`).
    pub const GET_CD: u32 = 0x0076_d0d0;
    pub const TK_SND_REPORT_STATUS: u32 = 0x0088_7b30;
    pub const SND_IS_READ_STATUS: u32 = 0x0088_6650;
    pub const SND_GET_STREAM_STATE: u32 = 0x0088_64b0;
    pub const SND_IS_READ_SHOUT_VOICE_STATUS: u32 = 0x0088_6640;

    /// `MsNowMusicStop()` (`0x007a00b0`): the music playing
    /// (`SndGetNowPlayMusic`) stopped and cleared; what `SndMusicClear`
    /// returns.
    pub fn ms_now_music_stop<M: Mem>(mem: &mut M, call: Call<'_, M>) -> u32 {
        let f = code(mem, SND_GET_NOW_PLAY_MUSIC);
        let m = call(mem, f, &[]);
        let f = code(mem, SND_MUSIC_STOP);
        call(mem, f, &[m]);
        let f = code(mem, SND_MUSIC_CLEAR);
        call(mem, f, &[m])
    }

    /// `MsResetNormalCalc()` (`0x00795c40`): every actor with `+0x3d` set
    /// has it cleared and its normals recalculated (`Ch_SetCalcNorm2`);
    /// how many.
    pub fn ms_reset_normal_calc(mem: &mut impl Mem) -> u32 {
        let mut n = 0;
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            if mem.read::<u8>(a.wrapping_add(0x3d)) != 0 {
                n += 1;
                mem.write(a.wrapping_add(0x3d), 0u8);
                ffx_chara::parts::bridge::ch_set_calc_norm2_holder(mem, a);
            }
        }
        n
    }

    /// `0x00791190()` (`MsBattleCanProgress`, name ours): -1 when nothing
    /// holds the battle (no pause, hit motion, turn ending, message, camera
    /// state, overdrive learnt, stone effect or command hold), else 0.
    pub fn ms_battle_can_progress(mem: &impl Mem) -> i32 {
        let quiet = PAUSE.get(mem) == 0
            && HIT_MOTION.get(mem) == 0
            && crate::action::bridge::TURN_END_ACTOR.get(mem).cast_signed() < 0
            && crate::state::bridge::MESSAGE_CUE_COUNT.get(mem) == 0
            && crate::atel::bridge::CAMERA_STATE.get(mem) == 0
            && crate::flow::bridge::LIMIT_LEARNT.get(mem) == 0
            && crate::state::bridge::STONE_EFFECT.get(mem)[0] == 0
            && crate::parse::bridge::COMMAND_HOLD.get(mem) == 0;
        -i32::from(quiet)
    }

    /// `0x00791760()` (`MsSetDeathAll`, name ours): `MsSetDeath(i, 1)`
    /// ([`crate::death::bridge::ms_set_death`]) for every actor.
    pub fn ms_set_death_all<'d>(
        mem: &mut crate::target::bridge::Dm<'d>,
        host: &crate::target::bridge::Host<'_>,
        s: &mut crate::atel::bridge::Script<'_, crate::target::bridge::Dm<'d>>,
    ) {
        for i in 0..0x1f {
            crate::death::bridge::ms_set_death(mem, host, s, i, 1);
        }
    }

    /// `msVoiceCheckStanby(any)` (`0x007b2c10`): -1 while a voice is not
    /// ready: the CD library busy (its `+8` with 1), the sound report 1
    /// (`TkSndReportStatus`), a sound read running (`SndIsReadStatus`,
    /// with `any`), a stream or a shout voice reading; else 0.
    pub fn ms_voice_check_stanby<M: Mem>(mem: &mut M, call: Call<'_, M>, any: u32) -> i32 {
        let f = code(mem, GET_CD);
        let cd = call(mem, f, &[]);
        let f: u32 = mem.read(cd.wrapping_add(8));
        if call(mem, f, &[1]) != 0 {
            return -1;
        }
        let f = code(mem, TK_SND_REPORT_STATUS);
        if call(mem, f, &[]) == 1 {
            return -1;
        }
        let f = code(mem, SND_IS_READ_STATUS);
        if call(mem, f, &[]) != 0 && any != 0 {
            return -1;
        }
        let f = code(mem, SND_GET_STREAM_STATE);
        if call(mem, f, &[]) != 0 {
            return -1;
        }
        let f = code(mem, SND_IS_READ_SHOUT_VOICE_STATUS);
        if call(mem, f, &[]) != 0 {
            return -1;
        }
        0
    }

    /// `0x00792e80()` (`MsCheckSoftResetOk`, name ours): -1 when a soft
    /// reset may end the battle: no read queued, no sound effect in
    /// transfer, the voices ready ([`ms_voice_check_stanby`]), the battle
    /// running (status 1), no effect, nothing waiting, no stone effect;
    /// else 0.
    pub fn ms_check_soft_reset_ok<M: Mem>(mem: &mut M, call: Call<'_, M>) -> i32 {
        use crate::state::bridge as st;
        let ok = st::ms_btl_read_cue_level(mem) == 0
            && st::ms_sep_trans_amount(mem) == 0
            && ms_voice_check_stanby(mem, call, 1) == 0
            && st::STATUS.get(mem) == 1
            && crate::minor::bridge::ms_effect_check(mem) == 0
            && RESET_WAIT.get(mem) == 0
            && RESET_WAIT2.get(mem) == 0
            && st::ms_stone_effect_check(mem) == 0;
        -i32::from(ok)
    }

    /// `MsMotionPosCheck(i, actor)` (`0x007aa100`): a moving actor
    /// (`MsMoveCheckFlag`) is busy (`+0x508`) while off its battle place
    /// (`MsCheckBtlPos(i, actor, +0x34)`); any other is not.
    pub fn ms_motion_pos_check(
        mem: &mut crate::target::bridge::Dm<'_>,
        host: &crate::target::bridge::Host<'_>,
        i: u32,
        actor: u32,
    ) {
        let busy = if crate::flow::bridge::ms_move_check_flag(mem, actor) != 0 {
            let flat = i32::from(mem.read::<i8>(actor.wrapping_add(0x34))).cast_unsigned();
            crate::pos::bridge::ms_check_btl_pos(mem, host.trig, i, actor, flat)
        } else {
            0
        };
        mem.write(actor.wrapping_add(0x508), busy.to_le_bytes()[0]);
    }

    /// `MsMotionPosCheckAll()` (`0x007aa140`): [`ms_motion_pos_check`] for
    /// every actor.
    pub fn ms_motion_pos_check_all(
        mem: &mut crate::target::bridge::Dm<'_>,
        host: &crate::target::bridge::Host<'_>,
    ) {
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            ms_motion_pos_check(mem, host, i, a);
        }
    }

    /// `0x007b0ca0()` (`MsCountChrActing`, name ours): actors whose command
    /// stage (`+0xdf0`) is not 0.
    pub fn ms_count_chr_acting(mem: &impl Mem) -> u32 {
        let mut n = 0;
        for i in 0..0x1f {
            if mem.read::<u8>(ms_get_chr(mem, i).wrapping_add(0xdf0)) != 0 {
                n += 1;
            }
        }
        n
    }

    /// The overdrive mode cue (1 when overdrive modes may be learnt).
    pub const LIMIT_TYPE_CUE: Global<u8> = Global::new(0x0112_c022);
    /// `MsMessageCueRegist` (its window callback system's: through the
    /// game) and `MsSetSaveCommand`.
    pub const MS_MESSAGE_CUE_REGIST: u32 = 0x0079_cfd0;

    /// `MsLimitTypeProcess()` (`0x007b1180`): once nothing acts any more
    /// ([`ms_count_chr_acting`]) after an overdrive was learnt (camera
    /// state 0, [`LIMIT_TYPE_CUE`] 1): each party member's overdrive modes
    /// not yet announced (its save record's `+0x88` bit clear) but met
    /// (its count `+0x60[k]` at 0) get their message (kind 7, `n`, 0x1e,
    /// 0x32), marked when it was queued; then the overdrive learnt last
    /// (`LIMIT_LEARNT_ID`) gets its message (kind 6) and its command
    /// (`MsSetSaveCommand(0, id, 1)`), the ID cleared once queued. Without
    /// one, `LIMIT_LEARNT` clears when no mode was due.
    pub fn ms_limit_type_process<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        use crate::flow::bridge::{LIMIT_LEARNT, LIMIT_LEARNT_ID};
        if crate::atel::bridge::CAMERA_STATE.get(mem) != 0
            || LIMIT_TYPE_CUE.get(mem) != 1
            || LIMIT_LEARNT.get(mem) == 0
            || ms_count_chr_acting(mem) != 0
        {
            return;
        }
        let regist = code(mem, MS_MESSAGE_CUE_REGIST);
        let mut due = false;
        let party = crate::menu::bridge::PARTY.ptr(mem).addr;
        let save = crate::save::bridge::PLY_SAVE.ptr(mem).addr;
        for k in 0..7 {
            let c = u32::from(mem.read::<u8>(party + k));
            if c == 0xff {
                continue;
            }
            let p = save.wrapping_add(c.wrapping_mul(0x94));
            let mut bits: u32 = mem.read(p.wrapping_add(0x88));
            for n in 0..0x14 {
                let count: u16 = mem.read(p.wrapping_add(0x60 + 2 * n));
                if bits.cast_signed() >> n & 1 == 0 && count == 0 {
                    due = true;
                    if call(mem, regist, &[7, c, n, 0x1e, 0x32]) != 0 {
                        bits |= 1 << n;
                    }
                }
            }
            mem.write(p.wrapping_add(0x88), bits);
        }
        let id = u32::from(LIMIT_LEARNT_ID.get(mem));
        if id != 0 {
            if call(mem, regist, &[6, 0, id, 0x1e, 0x32]) != 0 {
                crate::ability::bridge::ms_set_save_command(mem, 0, id, 1);
                LIMIT_LEARNT_ID.set(mem, 0);
            }
            return;
        }
        if !due {
            LIMIT_LEARNT.set(mem, 0);
        }
    }

    /// `op_et_battle_effect_free` (vfx) and `user_free` (the game's heap).
    pub const OP_ET_BATTLE_EFFECT_FREE: u32 = 0x007f_b170;
    pub const USER_FREE: u32 = 0x0068_7010;

    /// `MsEtEffectFree()` (`0x0079e630`): with the battle's effects read
    /// (`misc::ET_EFFECT_READ`), they are freed (`op_et_battle_effect_free`,
    /// the buffer `user_free`d) and both it and the effect owner cleared.
    pub fn ms_et_effect_free<M: Mem>(mem: &mut M, call: Call<'_, M>) {
        let buf = crate::misc::bridge::ET_EFFECT_READ.get(mem);
        if buf == 0 {
            return;
        }
        let f = code(mem, OP_ET_BATTLE_EFFECT_FREE);
        call(mem, f, &[]);
        let f = code(mem, USER_FREE);
        call(mem, f, &[buf]);
        crate::misc::bridge::ET_EFFECT_READ.set(mem, 0);
        crate::status::bridge::EFFECT_OWNER.set(mem, 0);
    }

    /// `MsFldGetScene(scene, &field, &group, &k)` (`0x007827f0`): the
    /// encounter of formation `scene & 0xffff` on map `scene >> 16`
    /// (`MsBtlListFieldNum`): the field, the group (`MsBtlListGroup`) and
    /// the formation's slot `k` in it go to the outputs (unless null) and
    /// the address of the matching byte is returned; 0 when none.
    pub fn ms_fld_get_scene(
        mem: &mut impl Mem,
        scene: u32,
        field: u32,
        group: u32,
        slot: u32,
    ) -> u32 {
        let formation = scene & 0xffff;
        let f =
            crate::minor::bridge::ms_btl_list_field_num(mem, (scene >> 16 & 0xffff).cast_signed());
        if f < 0 {
            return 0;
        }
        let list = crate::minor::bridge::ms_btl_list_encount(mem, f);
        let groups = u32::from(mem.read::<u8>(list.wrapping_add(1)));
        for g in 0..groups {
            let e = crate::flow::bridge::ms_btl_list_group(mem, f, g.cast_signed());
            let n = u32::from(mem.read::<u8>(e));
            for k in 0..n {
                let at = e.wrapping_add(5 + 2 * k);
                if u32::from(mem.read::<u8>(at)) == formation {
                    for (out, v) in [(field, f.cast_unsigned()), (group, g), (slot, k)] {
                        if out != 0 {
                            mem.write(out, v);
                        }
                    }
                    return at;
                }
            }
        }
        0
    }

    /// Set: no battle starts (`MsBattleLabelExe`).
    pub const LABEL_LOCK: Global<u8> = Global::new(0x0112_ca2c);
    /// The encounter's group and slot (`MsBattleLabelExe`).
    pub const GROUP_INDEX: Global<u8> = Global::new(0x0112_c258);
    pub const SLOT_INDEX: Global<u8> = Global::new(0x0112_c259);
    /// The second byte of the encounter type (`MsBattleLabelExe`).
    pub const ENCOUNT_TYPE2: Global<u8> = Global::new(0x0112_a9d5);

    /// `MsBattleLabelExe(scene, a, b)` (`0x00781c90`): outside battle (and
    /// unless [`LABEL_LOCK`]) the encounter of `scene`
    /// ([`ms_fld_get_scene`]) is requested: field, group and slot stored,
    /// the encounter type `a`, `b`, and the battle start request
    /// (`state::EXE` 2). -1.
    pub fn ms_battle_label_exe(
        mem: &mut crate::target::bridge::Dm<'_>,
        scene: u32,
        a: u32,
        b: u32,
    ) -> i32 {
        use crate::state::bridge as st;
        if LABEL_LOCK.get(mem) == 1 || st::ms_battle_get_status(mem) != 0 {
            return -1;
        }
        let found = mem.frame(0xc, |mem, f| {
            let r = ms_fld_get_scene(mem, scene, f, f + 4, f + 8);
            let v: [u32; 3] = mem.read(f);
            (r, v)
        });
        let (r, [field, group, slot]) = found;
        if r != 0 {
            let f = field.to_le_bytes();
            st::FIELD_INDEX.set(mem, i16::from_le_bytes([f[0], f[1]]));
            GROUP_INDEX.set(mem, group.to_le_bytes()[0]);
            SLOT_INDEX.set(mem, slot.to_le_bytes()[0]);
            st::ENCOUNT_TYPE.set(mem, a.to_le_bytes()[0]);
            st::EXE.set(mem, 2);
            ENCOUNT_TYPE2.set(mem, b.to_le_bytes()[0]);
        }
        -1
    }
}
