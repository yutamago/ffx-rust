//! The battle loop's per-frame pieces (`docs/rewrite-plan.md`, M2):
//! what `MsBattleProcess` (`0x00790c10`) checks and runs each frame
//! besides the command cue ([`crate::parse`]).
//!
//! Sound, the CD library and the voice status are other modules'; those go
//! to the game's code through the caller's `call`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, counters
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

    /// A forced encounter: on (with [`FORCED_ON2`]), and its slot.
    pub const FORCED_ON: Global<u32> = Global::new(0x0112_ca24);
    pub const FORCED_ON2: Global<u32> = Global::new(0x0112_ca20);
    pub const FORCED_SLOT: Global<u8> = Global::new(0x0112_ca28);
    /// How steps count (0 none, 1 when encounters are on, 2 ten times).
    pub const ENCOUNT_MODE: Global<u32> = Global::new(0x00c4_21d8);
    /// The group's danger (`+3`) of the last step.
    pub const DANGER: Global<u8> = Global::new(0x0112_a9d6);
    /// Set: an encounter fades in (`Sg_FadeInW(3)`) and counts as fought.
    pub const ENCOUNT_FADE: Global<u8> = Global::new(0x0112_a8f7);
    /// Not 0: battles may start (system's configuration).
    pub const BATTLE_ENABLE: Global<u32> = Global::new(0x00c4_21cc);
    /// `Sg_FadeInW` (sg).
    pub const SG_FADE_IN_W: u32 = 0x0082_cba0;

    /// `MsBattleEncountExe(map, group, step)` (`0x00780d10`): the random
    /// encounter of a step of length `step` (`f32` bits) in encounter
    /// group `group` of map `map`'s field. A forced encounter
    /// ([`FORCED_ON`]) starts at once (slot [`FORCED_SLOT`]). Otherwise the
    /// step counts by [`ENCOUNT_MODE`] (2 ten times, 1 only with encounters
    /// on, 0 not); in a group with danger (`+3`) and formations (`+4`), and
    /// the battles enabled, the steps (`ENCOUNT_STEPS`, the first since the
    /// last check, the second in all) grow; each 10.0 of the first beyond
    /// 10 is a check: once the total over 10 passes half the danger, the
    /// excess (×256 / (4·danger)) is the chance in 256 (`brnd(0)`), the
    /// first rate scaled by what is left, the second by 256; a hit picks
    /// the formation by weight (`+6 + 2k`'s high nibble, `brnd(1)` modulo
    /// `+4`). An encounter sets the field, group, slot and the start (or
    /// with [`ENCOUNT_FADE`] the fade and the battle count) and resets the
    /// counters (`ResetEncountExe(1)`): -1; else 0.
    #[allow(clippy::too_many_lines)]
    pub fn ms_battle_encount_exe<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        map: i32,
        group: i32,
        step: u32,
    ) -> i32 {
        use crate::minor::bridge::{ENCOUNT_ON, ENCOUNT_RATES, ENCOUNT_STEPS, reset_encount_exe};
        use crate::state::bridge as st;
        use ffx_bridge::x87::{add, div, ftol2, load, mul, store, sub};
        let field = crate::minor::bridge::ms_btl_list_field_num(mem, map);
        let set_field = |mem: &mut M| {
            let f = field.to_le_bytes();
            st::FIELD_INDEX.set(mem, i16::from_le_bytes([f[0], f[1]]));
        };
        if FORCED_ON.get(mem) != 0 && FORCED_ON2.get(mem) != 0 {
            GROUP_INDEX.set(mem, group.to_le_bytes()[0]);
            st::EXE.set(mem, 1);
            set_field(mem);
            let slot = FORCED_SLOT.get(mem);
            SLOT_INDEX.set(mem, slot);
            reset_encount_exe(mem, 1);
            return -1;
        }
        let ten = load(Global::<u32>::new(0x00b9_2318).get(mem));
        let ten_d = Global::<f64>::new(0x00b9_22b0).get(mem);
        let full = Global::<f64>::new(0x00b9_22f0).get(mem);
        let mut step = step;
        let counts = match ENCOUNT_MODE.get(mem) {
            2 => {
                step = store(mul(ten_d, load(step)));
                true
            }
            1 => ENCOUNT_ON.get(mem) != 0,
            0 => {
                step = store(0.0);
                false
            }
            _ => true,
        };
        if st::EXE.get(mem) == 2 || field < 0 || load(step) == 0.0 || !counts {
            return 0;
        }
        let list = crate::minor::bridge::ms_btl_list_encount(mem, field);
        if group < 0 || group >= i32::from(mem.read::<u8>(list.wrapping_add(1))) {
            return 0;
        }
        if BATTLE_ENABLE.get(mem) == 0 {
            return 0;
        }
        let e = crate::flow::bridge::ms_btl_list_group(mem, field, group);
        let danger = u32::from(mem.read::<u8>(e.wrapping_add(3)));
        let formations = u32::from(mem.read::<u8>(e.wrapping_add(4)));
        if danger == 0 || formations == 0 {
            return 0;
        }
        let [steps, total] = ENCOUNT_STEPS.get(mem);
        let d = load(step);
        let steps = store(add(load(steps), d));
        let total = store(add(d, load(total)));
        ENCOUNT_STEPS.set(mem, [steps, total]);
        DANGER.set(mem, danger.to_le_bytes()[0]);
        let mut level = ftol2(div(load(total), ten_d)).cast_signed();
        let over = |mem: &M| {
            let s = load(ENCOUNT_STEPS.get(mem)[0]);
            s > ten
        };
        while over(mem) {
            let half = (danger >> 1).cast_signed();
            if level > half {
                let q = (level - half)
                    .wrapping_shl(8)
                    .wrapping_div((danger << 2).cast_signed());
                level = q;
                let left = store(f64::from(0x100 - q));
                let [r0, r1] = ENCOUNT_RATES.get(mem);
                let r0 = store(mul(load(left), load(r0)));
                let r1 = store(mul(full, load(r1)));
                ENCOUNT_RATES.set(mem, [r0, r1]);
                let roll = (crate::rng::bridge::brnd(mem, 0) & 0xff).cast_signed();
                if roll < q {
                    let pick = crate::rng::bridge::brnd(mem, 1)
                        .cast_signed()
                        .wrapping_rem(formations.cast_signed());
                    let n = u32::from(mem.read::<u8>(e));
                    let mut sum = 0;
                    for k in 0..n {
                        sum += i32::from(mem.read::<u8>(e.wrapping_add(6 + 2 * k)) >> 4);
                        if pick < sum {
                            if ENCOUNT_FADE.get(mem) != 0 {
                                let f = code(mem, SG_FADE_IN_W);
                                call(mem, f, &[3]);
                                let c = crate::save::bridge::BATTLE_COUNT.get(mem).wrapping_add(1);
                                crate::save::bridge::BATTLE_COUNT.set(mem, c);
                            } else {
                                st::EXE.set(mem, 1);
                            }
                            set_field(mem);
                            GROUP_INDEX.set(mem, group.to_le_bytes()[0]);
                            SLOT_INDEX.set(mem, k.to_le_bytes()[0]);
                            reset_encount_exe(mem, 1);
                            return -1;
                        }
                    }
                }
            }
            let [s0, s1] = ENCOUNT_STEPS.get(mem);
            ENCOUNT_STEPS.set(mem, [store(sub(load(s0), ten_d)), s1]);
        }
        0
    }

    /// `0x0078f000(i)` (`MsOrderKey`, name ours): the turn order key of
    /// actor `i`: a monster `0x10000 + i`, a party member
    /// `(0xff - +0x5ac) << 8 | i` (its speed counter `+0x5ac`); smaller
    /// goes first.
    pub fn ms_order_key(mem: &impl Mem, i: u32) -> u32 {
        if crate::actor::bridge::ms_get_ram_chr_monster(i) != 0 {
            return i.wrapping_add(0x10000);
        }
        let c = u32::from(mem.read::<u8>(ms_get_chr(mem, i).wrapping_add(0x5ac)));
        ((0xff - c) << 8).wrapping_add(i)
    }

    /// `MsOrderCalc(&n)` (`0x0078d3e0`): the order list (`state::ORDERS`,
    /// `state::ORDER_COUNT` entries, the count to `n` unless null) sorted
    /// by [`ms_order_key`] (selection, the first smallest moved forward);
    /// the list.
    pub fn ms_order_calc(mem: &mut impl Mem, n: u32) -> u32 {
        use crate::state::bridge::{ORDER_COUNT, ORDERS};
        let list = ORDERS.ptr(mem).addr;
        if n != 0 {
            mem.write(n, ORDER_COUNT.get(mem));
        }
        let mut b = 0i32;
        while b < ORDER_COUNT.get(mem) - 1 {
            let at = list + b.cast_unsigned();
            let mut best = ms_order_key(mem, u32::from(mem.read::<u8>(at)));
            let mut k = b;
            let mut j = b + 1;
            while j < ORDER_COUNT.get(mem) {
                let key = ms_order_key(mem, u32::from(mem.read::<u8>(list + j.cast_unsigned())));
                if key.cast_signed() < best.cast_signed() {
                    k = j;
                    best = key;
                }
                j += 1;
            }
            if k != b {
                let other = list + k.cast_unsigned();
                let (x, y) = (mem.read::<u8>(at), mem.read::<u8>(other));
                mem.write(at, y);
                mem.write(other, x);
            }
            b += 1;
        }
        list
    }

    /// `MsLimitTypeTurnCheck(i, actor)` (`0x007b13c0`): a party member's
    /// (0-6) overdrive modes earned by its turn: always 0xd (Stoic), 0xf
    /// with its HP low (`MsCalcWeakLevel` 1 and above), 0x10 (Loner) with
    /// no other able party member, 0xe (Victim) with a bad status; each
    /// counted (`MsLimitTypeExpGet`) and, when it is the mode in use
    /// (`+0x5bb`), the gauge raised (`MsLimitUp` by 3, 5, 0x10, 0x10). How
    /// many; 0 for a non-party actor.
    pub fn ms_limit_type_turn_check<M: Mem>(mem: &mut M, i: u32, actor: u32) -> u32 {
        if i > 6 {
            return 0;
        }
        let earn = |mem: &mut M, k: u32, n: i32| {
            crate::exec::bridge::ms_limit_type_exp_get(mem, i, k, 0);
            if u32::from(mem.read::<u8>(actor.wrapping_add(0x5bb))) == k {
                crate::turn::bridge::ms_limit_up(mem, i, actor, n);
            }
        };
        let byte = |mem: &M, o: u32| mem.read::<u8>(actor.wrapping_add(o));
        let mut r = 1;
        earn(mem, 0xd, 3);
        let hp: i32 = mem.read(actor.wrapping_add(0x5d0));
        let max: i32 = mem.read(actor.wrapping_add(0x594));
        if crate::actor::bridge::ms_calc_weak_level(hp, max) >= 1 {
            r = 2;
            earn(mem, 0xf, 5);
        }
        let others = (0..0x12)
            .filter(|&j| j != i)
            .filter(|&j| {
                let a = ms_get_chr(mem, j);
                let b = |o: u32| mem.read::<u8>(a.wrapping_add(o));
                b(0xdc8) != 0 && b(0xdd6) != 0 && b(0xdcc) == 0 && b(0xdce) == 0
            })
            .count();
        if others == 0 {
            r += 1;
            earn(mem, 0x10, 0x10);
        }
        let status: u16 = mem.read(actor.wrapping_add(0x606));
        let bad = status & 0xa != 0
            || status & 0x100 != 0
            || byte(mem, 0x608) != 0
            || byte(mem, 0x609) != 0
            || byte(mem, 0x60a) != 0
            || byte(mem, 0x614) != 0
            || mem.read::<u16>(actor.wrapping_add(0x616)) & 0x4000 != 0;
        if !bad {
            return r;
        }
        earn(mem, 0xe, 0x10);
        r + 1
    }

    /// Set: no turn order is taken (`MsBattleProcess_Order`).
    pub const ORDER_HOLD: Global<u8> = Global::new(0x0112_a8e1);
    /// The idle rounds counter and its limit (`MsBattleProcess_Order`).
    pub const IDLE_ROUNDS: Global<i8> = Global::new(0x0112_bde2);
    pub const IDLE_ROUNDS_LIMIT: Global<i8> = Global::new(0x0112_bde3);
    /// Set: a waiting status lasts one more round at most.
    pub const STATUS_SHORT: Global<u8> = Global::new(0x0112_a8fb);
    /// `MagicFile_GetMagicId` (system).
    pub const MAGIC_FILE_GET_MAGIC_ID: u32 = 0x009d_a3b0;

    /// `MsBattleProcess_Order_00790fb0()` (`0x00790fb0`): with no input
    /// entry waiting (and no [`ORDER_HOLD`]), the turn order is made: every
    /// actor able to act now (`+0xdc8`, not KO'd, not out (`+0xde6`), no
    /// `+0x606` bit 2, its wait `+0x65c` over, `+0xdd6`) is registered
    /// (`MsOrderRegist`) unless a title window still shows after one of
    /// the commands that wait for it (magic file 0x1ad; 0x312c, 0x4125,
    /// 0x3022, 0x3108, 0x403c, 0x60bd): -1. The first in order
    /// ([`ms_order_calc`]) gets an input entry (`MsRegistInputCue(i, 0, 1,
    /// 0)`) and its turn's overdrive modes ([`ms_limit_type_turn_check`]).
    /// With nobody, after [`IDLE_ROUNDS_LIMIT`] such frames each able
    /// actor's round passes: `+0x6d2` counted (up to 0xff), its wait
    /// `+0x65c` down (one with [`STATUS_SHORT`]); at 0 its input stage
    /// `+0xde8` 3 and the wait `+0x65d` again without `+0xdd6`. -1; 0 when
    /// waiting.
    pub fn ms_battle_process_order<M: Mem>(mem: &mut M, call: Call<'_, M>) -> i32 {
        use crate::state::bridge as st;
        if ORDER_HOLD.get(mem) != 0 || st::INPUT_CUE_COUNT.get(mem) != 0 {
            return 0;
        }
        st::ms_order_init(mem);
        let b = |mem: &M, a: u32, o: u32| mem.read::<u8>(a.wrapping_add(o));
        let able = |mem: &M, a: u32| {
            b(mem, a, 0xde6) == 0
                && b(mem, a, 0xdc8) != 0
                && b(mem, a, 0xdcc) == 0
                && b(mem, a, 0x606) & 4 == 0
        };
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            if !able(mem, a) || b(mem, a, 0x65c) != 0 || b(mem, a, 0xdd6) == 0 {
                continue;
            }
            let f = code(mem, MAGIC_FILE_GET_MAGIC_ID);
            let magic = call(mem, f, &[]);
            let cmd = crate::parse::bridge::PARSED_COMMAND.get(mem);
            let waits =
                magic == 0x1ad || [0x312c, 0x4125, 0x3022, 0x3108, 0x403c, 0x60bd].contains(&cmd);
            if waits && st::TITLE_WINDOW.get(mem)[0] > 0 {
                return -1;
            }
            st::ms_order_regist(mem, i.to_le_bytes()[0]);
        }
        let n = st::ORDER_COUNT.get(mem);
        let list = ms_order_calc(mem, 0);
        if n > 0 {
            let first = u32::from(mem.read::<u8>(list));
            if crate::cue::bridge::ms_regist_input_cue(mem, first, 0, 1, 0) != 0 {
                let a = ms_get_chr(mem, first);
                ms_limit_type_turn_check(mem, first, a);
            }
            return -1;
        }
        let rounds = IDLE_ROUNDS.get(mem).wrapping_add(1);
        IDLE_ROUNDS.set(mem, rounds);
        if rounds < IDLE_ROUNDS_LIMIT.get(mem) {
            return -1;
        }
        IDLE_ROUNDS.set(mem, 0);
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            if !able(mem, a) {
                continue;
            }
            let c = b(mem, a, 0x6d2);
            if c < 0xff {
                mem.write(a.wrapping_add(0x6d2), c + 1);
            }
            let left = i32::from(b(mem, a, 0x65c)) - 1;
            let wait = if left > 0 {
                if STATUS_SHORT.get(mem) != 0 { 1 } else { left }
            } else {
                mem.write(a.wrapping_add(0xde8), 3u8);
                if b(mem, a, 0xdd6) == 0 {
                    i32::from(b(mem, a, 0x65d))
                } else {
                    0
                }
            };
            mem.write(a.wrapping_add(0x65c), wait.to_le_bytes()[0]);
        }
        -1
    }
}
