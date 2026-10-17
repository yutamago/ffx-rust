//! An actor going down (`docs/rewrite-plan.md`, M2): `MsSetDeath`, run for
//! every actor each frame (`MsSetDeathAll`), takes one that was KO'd
//! (`+0xdcc` 1) through its death: the camera, its motion and effect (a
//! fade, dissolving, petrification shattering), and the stone effects the
//! battle queued (`MsStoneEffectProcess`).
//!
//! The effects are vfx's (`op_et_eff`), the sound battle's own but not
//! ported yet (`MsRegSEplay`); the trophy check is main's and the
//! character's ID and neck chara's: those go to the game's code through
//! `s.call`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, the script calls
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::{Script, ms_set_motion};
    use crate::target::bridge::{Dm, Host};

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    /// The actor dying last (`MsSetDeath`).
    pub const DYING: Global<u8> = Global::new(0x0112_c9e3);
    /// Stone effects begun this round (up to 3 at once).
    pub const STONE_STARTED: Global<i8> = Global::new(0x0112_ca0c);

    /// The game's functions the deaths call.
    pub const MS_REG_SE_PLAY: u32 = 0x007a_0100;
    pub const OP_ET_EFF: u32 = 0x007f_c2e0;
    pub const MS_RESET_BIND_EFFECT: u32 = 0x0078_8e60;
    pub const CH_GET_CHAR_ID: u32 = 0x0082_a080;
    pub const TROPHY_CHECK_DEFEAT_MONSTER: u32 = 0x0082_2760;

    /// `MsCheckDeathMotion(i, actor)` (`0x0078c4c0`): 1 when the actor may
    /// play its death: `+0xf5e` bits 1 and 2 clear, not busy (`+0x508`)
    /// when `+0x3f9`, and neither an input nor a command entry of `i`
    /// running (`MsCheckInputCueExe`, `MsCheckCommandCueExe`); else 0.
    pub fn ms_check_death_motion(mem: &impl Mem, i: u32, actor: u32) -> u32 {
        let f = byte(mem, actor, 0xf5e);
        let mut ok = f & 6 == 0;
        if byte(mem, actor, 0x3f9) != 0 && byte(mem, actor, 0x508) != 0 {
            ok = false;
        }
        if crate::cue::bridge::ms_check_input_cue_exe(mem, i) != 0
            || crate::cue::bridge::ms_check_command_cue_exe(mem, i) != 0
        {
            return 0;
        }
        u32::from(ok)
    }

    /// `MsDeathTerminate(i, actor)` (`0x0078cca0`): the death is over:
    /// unless its effect still plays (`+0x3f7`) `+0x436` cleared, `+0x3f4`
    /// set, the status effects redone (`MsStatusEffectCheck`) and the bound
    /// effects reset (`MsResetBindEffect`, through the game); what that
    /// returns.
    pub fn ms_death_terminate<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        i: u32,
        actor: u32,
    ) -> u32 {
        if byte(mem, actor, 0x3f7) == 0 {
            mem.write(actor.wrapping_add(0x436), 0u8);
        }
        mem.write(actor.wrapping_add(0x3f4), 1u8);
        crate::status::bridge::ms_status_effect_check(mem, &mut *s.call, &mut *s.off, i);
        let f = code(mem, MS_RESET_BIND_EFFECT);
        (s.call)(mem, f, &[i])
    }

    /// `MsSetDeathProcess(i, actor)` (`0x0078ddc0`): KO state 2, its
    /// height (`+0xf70`) from its position (`MsGetChrPos`'s y; the
    /// original's stack garbage for a non-actor, here 0), the death camera
    /// (0x3f), `+0x50e` cleared, its body hit off (`msSetChrBodyHit(3, 0)`)
    /// and `+0x41c` cleared.
    pub fn ms_set_death_process<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
    ) {
        mem.write(actor.wrapping_add(0xdcc), 2u8);
        let y = mem.frame(0x10, |mem, p| {
            crate::pos::bridge::ms_get_chr_pos(mem, host.trig, i, p);
            mem.read::<u32>(p + 4)
        });
        mem.write(actor.wrapping_add(0xf70), crate::camera::x87_copy(y));
        crate::atel::bridge::ms_atel_request_camera(mem, s, 0, i, 0x3f, 0xffff, 0, 1, 0, 0);
        mem.write(actor.wrapping_add(0x50e), 0u16);
        crate::body::bridge::ms_set_chr_body_hit(mem, actor, 3, 0);
        mem.write(actor.wrapping_add(0x41c), 0u8);
    }

    /// `MsDeathEffectStart(i, kind)` (`0x0079e3d0`): actor `i`'s death
    /// effect by `kind` (2-5 the four fades, 7 shattering with sound 0x58
    /// and effect group 0xf, 8 one without sound; an actor of `+0x701` 3
    /// none): its sound (`MsRegSEplay`), the effect (`op_et_eff` of the
    /// battle's effects, ID `MsGetEffID(+0xe1e, group, i)`, `+0xe1e` counted
    /// on) and `+0x3f7` set: 1; else 0.
    pub fn ms_death_effect_start<M: Mem>(
        mem: &mut M,
        s: &mut Script<'_, M>,
        i: u32,
        kind: u32,
    ) -> u32 {
        if i == 0xff {
            return 0;
        }
        let a = ms_get_chr(mem, i);
        let k = if byte(mem, a, 0x701) == 3 { 1 } else { kind };
        let mut group = 4;
        let (effect, sound) = match k.wrapping_sub(2) {
            0 => (0x29, Some(0x4c)),
            1 => (0x2a, Some(0x1c)),
            2 => (0x2b, Some(0x1b)),
            3 => (0x2c, Some(0x49)),
            5 => {
                group = 0xf;
                mem.write(a.wrapping_add(0xe17), 0u8);
                (0x39, Some(0x58))
            }
            6 => (0x2c, None),
            _ => return 0,
        };
        if let Some(se) = sound {
            let f = code(mem, MS_REG_SE_PLAY);
            (s.call)(mem, f, &[i, se]);
        }
        let n: u16 = mem.read(a.wrapping_add(0xe1e));
        let id = crate::util::bridge::ms_get_eff_id(u32::from(n), group, i);
        mem.write(a.wrapping_add(0xe1e), n.wrapping_add(1));
        mem.write(a.wrapping_add(0x3f7), 1u8);
        let owner = crate::status::bridge::EFFECT_OWNER.get(mem);
        let f = code(mem, OP_ET_EFF);
        (s.call)(mem, f, &[owner, 0, effect, id]);
        1
    }

    /// `MsStoneEffectProcess()` (`0x007a2690`): with petrified actors
    /// queued (`MsStoneRegist`) and the message cue started, up to three at
    /// a time shatter: the first is taken off the queue, marked in
    /// `minor::STONE_EXCLUDED` and its effect started
    /// ([`ms_death_effect_start`] kind 7, counted in [`STONE_STARTED`]). 1
    /// when the queue was looked at, else 0.
    pub fn ms_stone_effect_process<M: Mem>(mem: &mut M, s: &mut Script<'_, M>) -> u32 {
        use crate::minor::bridge::{STONE_EXCLUDED, STONES};
        let count = crate::state::bridge::STONE_EFFECT.ptr(mem).addr;
        let started = crate::misc::bridge::MESSAGE_CUE_INIT.ptr(mem).addr;
        if mem.read::<i8>(count) <= 0 || mem.read::<u8>(started) == 0 {
            return 0;
        }
        let list = STONES.ptr(mem).addr;
        loop {
            if STONE_STARTED.get(mem) >= 3 {
                return 1;
            }
            let chr: u32 = mem.read(list);
            let n = mem.read::<i8>(count).wrapping_sub(1);
            mem.write(count, n);
            let bits = STONE_EXCLUDED.get(mem) | 1u32.wrapping_shl(chr & 0xff);
            STONE_EXCLUDED.set(mem, bits);
            for k in 0..i32::from(n).max(0).cast_unsigned() {
                let v: u32 = mem.read(list + 4 * (k + 1));
                mem.write(list + 4 * k, v);
            }
            if ms_death_effect_start(mem, s, chr, 7) != 0 {
                let v = STONE_STARTED.get(mem).wrapping_add(1);
                STONE_STARTED.set(mem, v);
            }
            if mem.read::<i8>(count) <= 0 {
                return 1;
            }
        }
    }

    /// `MsSetDeath(i, flags)` (`0x0078dc30`): a KO'd actor (`+0xdcc` 1)
    /// able to die ([`ms_check_death_motion`]) does: the defeat trophy
    /// checked (`trophyCheckDefeatMonster(Ch_GetCharID(+0xe))`), `+0x436`
    /// set, [`DYING`] `i`, its effect motions freed, its position reset and
    /// its death processed ([`ms_set_death_process`]), its head target off
    /// (`Ch_NeckSetEnable(chr, 0)`). By its death kind (`+0x449`): 1 ends
    /// it at once (or motion 0x39 out of battle), 6 with its effect
    /// (`+0x3f5`); otherwise out of battle (`+0xdcd`) a petrified one
    /// (`+0xdce`) is queued to shatter (motion 0x3a), any other plays
    /// motion 0x39, an actor in battle its effect and motion 8 (or with
    /// `+0xdd0` effect 2 and motion 0x40); the motions with `flags`. 1;
    /// else 0.
    pub fn ms_set_death<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        flags: u32,
    ) -> u32 {
        let a = ms_get_chr(mem, i);
        if byte(mem, a, 0xdcc) != 1 || ms_check_death_motion(mem, i, a) == 0 {
            return 0;
        }
        let model = u32::from(mem.read::<u16>(a.wrapping_add(0xe)));
        let f = code(mem, CH_GET_CHAR_ID);
        let id = (s.call)(mem, f, &[model]);
        let f = code(mem, TROPHY_CHECK_DEFEAT_MONSTER);
        (s.call)(mem, f, &[id]);
        mem.write(a.wrapping_add(0x436), 1u8);
        DYING.set(mem, i.to_le_bytes()[0]);
        crate::minor::bridge::ms_effect_reset_motion_disable(mem, a);
        crate::status::bridge::ms_reset_chr_pos(mem, host.trig, i, a);
        ms_set_death_process(mem, host, s, i, a);
        mem.write(a.wrapping_add(0x438), 0xffu8);
        let chr: u32 = mem.read(a);
        let f = code(mem, crate::atel::bridge::CH_NECK_SET_ENABLE);
        (s.call)(mem, f, &[chr, 0]);
        let motion = |mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>, m: i32| {
            ms_set_motion(mem, s, i, m, 1, 0, flags, 0, 0);
        };
        match byte(mem, a, 0x449) {
            1 => {
                if byte(mem, a, 0xdcd) != 0 {
                    motion(mem, s, 0x39);
                } else {
                    ms_death_terminate(mem, s, i, a);
                }
            }
            6 => {
                let k = u32::from(byte(mem, a, 0x3f5));
                ms_death_effect_start(mem, s, i, k);
                ms_death_terminate(mem, s, i, a);
            }
            _ => {
                if byte(mem, a, 0xdcd) != 0 {
                    if byte(mem, a, 0xdce) == 0 {
                        motion(mem, s, 0x39);
                    } else {
                        crate::minor::bridge::ms_stone_regist(mem, i);
                        motion(mem, s, 0x3a);
                    }
                } else if byte(mem, a, 0xdd0) != 0 {
                    ms_death_effect_start(mem, s, i, 2);
                    motion(mem, s, 0x40);
                } else {
                    let k = u32::from(byte(mem, a, 0x3f5));
                    ms_death_effect_start(mem, s, i, k);
                    motion(mem, s, 8);
                }
            }
        }
        1
    }

    /// `MsGetItem` (not ported yet: its gear rewards are
    /// `BattleRewards_AddGear`'s).
    pub const MS_GET_ITEM: u32 = 0x0079_90d0;
    /// `forceAutoBattleOff` (the auto-battle option).
    pub const FORCE_AUTO_BATTLE_OFF: u32 = 0x0078_71f0;
    /// `TOBtlCloseSignWindow` (system): a capture message's sign closes.
    pub const TO_BTL_CLOSE_SIGN_WINDOW: u32 = 0x0089_0ee0;
    /// Set: no kill counts as an overkill (`MsDamageCheckDeath`).
    pub const NO_OVERKILL: Global<u8> = Global::new(0x0112_a914);

    /// `MsLimitTypeDeathCheck(i, actor, j, victim)` (`0x007b0f80`): the
    /// overdrive modes a death earns: a party member KO'd by a monster
    /// gives every other able party member (0-6) Comrade (7; the gauge
    /// by 0x1e with it in use, `+0x5bb`): how many, else 0; a monster
    /// killed by a party member gives it Slayer (8, by 0x14), Hero (9)
    /// when the monster's max HP is above 20 times its own `+0x6f4` or
    /// above 9999, the gauge raised by 0x14 for Hero in use when above
    /// three times (unsigned).
    pub fn ms_limit_type_death_check(
        mem: &mut impl Mem,
        i: u32,
        actor: u32,
        j: u32,
        victim: u32,
    ) -> u32 {
        use crate::actor::bridge::ms_get_ram_chr_monster;
        use crate::exec::bridge::ms_limit_type_exp_get;
        use crate::turn::bridge::ms_limit_up;
        let (by, of) = (ms_get_ram_chr_monster(i), ms_get_ram_chr_monster(j));
        if by == 1 {
            if of != 0 {
                return 0;
            }
            let mut n = 0;
            for k in 0..7 {
                let a = ms_get_chr(mem, k);
                if byte(mem, a, 0xdc8) == 0 || k == j {
                    continue;
                }
                n += 1;
                ms_limit_type_exp_get(mem, k, 7, 0);
                if byte(mem, a, 0x5bb) == 7 {
                    ms_limit_up(mem, k, a, 0x1e);
                }
            }
            return n;
        }
        if by != 0 || of != 1 {
            return 0;
        }
        ms_limit_type_exp_get(mem, i, 8, 0);
        if byte(mem, actor, 0x5bb) == 8 {
            ms_limit_up(mem, i, actor, 0x14);
        }
        let own: i32 = mem.read(actor.wrapping_add(0x6f4));
        let max: i32 = mem.read(victim.wrapping_add(0x594));
        if max > own.wrapping_mul(0x14) || max >= 10_000 {
            ms_limit_type_exp_get(mem, i, 9, 0);
        }
        if byte(mem, actor, 0x5bb) == 9 && max.cast_unsigned() > own.wrapping_mul(3).cast_unsigned()
        {
            ms_limit_up(mem, i, actor, 0x14);
        }
        0
    }

    /// An inserted command block (`0x007b2740`/`0x007b26d0`): actor `i`'s
    /// command `ids` (low half the command, high half the second ID) on
    /// `target` alone, after `after`'s at priority `prio`
    /// (`MsInsertBtlCommand(block, prio, 1, after)`), the block cleared
    /// first.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn insert_one<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        target: u32,
        after: u32,
        ids: u32,
        prio: u32,
    ) {
        mem.locals(0x48, |mem, cmd| {
            crate::util::bridge::ms_struct_clear(mem, cmd, 0x48);
            mem.write(cmd, i.to_le_bytes()[0]);
            mem.write(cmd.wrapping_add(8), ids);
            mem.write(cmd.wrapping_add(0x10), 1u32.wrapping_shl(target & 0xff));
            mem.write(cmd.wrapping_add(3), 1u8);
            crate::action::bridge::ms_insert_btl_command(mem, host, s, cmd, prio, 1, after);
        });
    }

    /// `0x007b2740(i, target, after, id, prio)` (`MsAutoInsertAbility`,
    /// name ours): command `id` (second ID 0xff) by `i` on `target`
    /// ([`insert_one`]).
    #[allow(clippy::too_many_arguments)]
    pub fn ms_auto_insert_ability<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        target: u32,
        after: u32,
        id: u32,
        prio: u32,
    ) {
        insert_one(
            mem,
            host,
            s,
            i,
            target,
            after,
            (id & 0xffff) | 0x00ff_0000,
            prio,
        );
    }

    /// `MsAutoInsertCommand_007b26d0(i, target, after, item, prio)`
    /// (`0x007b26d0`): item `item` used (command 0x3001) by `i` on
    /// `target` ([`insert_one`]).
    #[allow(clippy::too_many_arguments)]
    pub fn ms_auto_insert_command<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        target: u32,
        after: u32,
        item: u32,
        prio: u32,
    ) {
        insert_one(mem, host, s, i, target, after, 0x3001 | (item << 16), prio);
    }

    /// `MsAutoRelifeProcess_007b2910(i, actor, killer, status)`
    /// (`0x007b2910`): Auto-Life (`status`, the `+0x616` the actor had,
    /// bit 9) revives it: command 0x311f on itself after `killer`'s at
    /// priority 0x40 ([`ms_auto_insert_ability`]): 0. Else `+0x5ca`
    /// cleared: -1.
    pub fn ms_auto_relife_process<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
        killer: u32,
        status: u32,
    ) -> i32 {
        if status & 0x200 != 0 {
            ms_auto_insert_ability(mem, host, s, i, i, killer, 0x311f, 0x40);
            return 0;
        }
        mem.write(actor.wrapping_add(0x5ca), 0u8);
        -1
    }

    /// `MsAutoPhenixProcess(i, actor, killer)` (`0x007b27b0`): Auto-Phoenix:
    /// the first of `i`'s side (party 0-0x13 with a Phoenix Down 0x2006 in
    /// the bag, monsters 0x14-0x1b) not KO'd, with the ability
    /// (`+0x6bc` bit 12) and free to act
    /// (`MsAutoCommandStatusCheck(.., 1, 1, 1)`) throws one on `i` after
    /// `killer`'s at priority 0x30 ([`ms_auto_insert_command`]): 0x2006.
    /// Else 0.
    pub fn ms_auto_phenix_process<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
        killer: u32,
    ) -> u32 {
        let _ = actor;
        let (from, to) = if crate::actor::bridge::ms_get_ram_chr_monster(i) == 0 {
            if crate::save::bridge::ms_get_save_item_num(mem, 0x2006).cast_signed() < 1 {
                return 0;
            }
            (0, 0x14)
        } else {
            (0x14, 0x1c)
        };
        for u in from..to {
            let a = ms_get_chr(mem, u);
            if byte(mem, a, 0xdcc) != 0
                || mem.read::<u16>(a.wrapping_add(0x6bc)) & 0x1000 == 0
                || crate::actor::bridge::ms_auto_command_status_check(mem, a, 1, 1, 1) == 0
            {
                continue;
            }
            ms_auto_insert_command(mem, host, s, u, i, killer, 0x2006, 0x30);
            return 0x2006;
        }
        0
    }

    /// `MsDamageCheckDeath(i, j, no_reward, flags)` (`0x0078c740`): actor
    /// `j` (present, `+0x10`, not KO'd yet) dies of `i`'s hit when its HP
    /// is gone (kind 1) or it is to be removed (`+0x617` bit 0, kind 2,
    /// `+0x702`; its look `+0x700` from `+0x701`, 1 for a removal without
    /// one): `+0xdee` the killer, the killer's `+0x6dd` cleared, `+0xdcf`
    /// with `+0xdcd` and `+0xdce`. A monster rewarding the kill: counted
    /// in the save (`MsSetMonsterCheck`), captured (`+0xdd0`,
    /// `MsMonsterCapture`; `+0xdd1`), `END_ALT` on, the killer's kills
    /// (`MsSaveAddInfoData(i, 1)`) and, with its overkill HP (`+0xf60`)
    /// spent and no [`NO_OVERKILL`], an overkill (`+0xf5f`, its word:
    /// number kind 5); else
    /// `+0xdd0` cleared. Then its death counted, KO'd (`+0xdcc` 1),
    /// Provoke and Threaten off, its statuses cleared, an aeon's gauge
    /// emptied, `+0x606` bit 0, its second command's count (`+0x6d0`) used
    /// up, its input and command entries gone, `+0xdc8` cleared with
    /// `+0x3f5`, its death action 4 (`MsActionRequest(j, i, 4, 0, 1)`),
    /// the spoils (`MsGetItem`, through the game) unless `no_reward`,
    /// Auto-Life or else Auto-Phoenix for kind 1, the overdrive modes
    /// ([`ms_limit_type_death_check`]), the death itself
    /// ([`ms_set_death`]) and its pose. When that started: an aeon of the
    /// party (8-0xe, `+0x591` 0x11), or the last of the three magus sisters
    /// (0xf-0x11), turns auto-battle off (`forceAutoBattleOff`). What
    /// [`ms_set_death`] returned; 0 when nobody died.
    pub fn ms_damage_check_death<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        j: u32,
        no_reward: u32,
        flags: u32,
    ) -> u32 {
        let a = ms_get_chr(mem, i);
        let v = ms_get_chr(mem, j);
        let hp = |mem: &Dm<'_>, x: u32| mem.read::<i32>(x.wrapping_add(0x5d0));
        let kind = if byte(mem, v, 0x617) & 1 != 0 {
            2u8
        } else {
            u8::from(hp(mem, v) < 1)
        };
        if byte(mem, v, 0x10) == 0 || kind == 0 || byte(mem, v, 0xdcc) != 0 {
            return 0;
        }
        let mut look = byte(mem, v, 0x701);
        if look == 0 && kind == 2 {
            look = 1;
        }
        mem.write(v.wrapping_add(0x702), kind);
        mem.write(v.wrapping_add(0x700), look);
        mem.write(v.wrapping_add(0xdee), i.to_le_bytes()[0]);
        mem.write(a.wrapping_add(0x6dd), 0u8);
        if byte(mem, v, 0xdcd) != 0 && byte(mem, v, 0xdce) != 0 {
            mem.write(v.wrapping_add(0xdcf), 1u8);
        }
        if crate::actor::bridge::ms_get_ram_chr_monster(j) != 0 && no_reward == 0 {
            use crate::save::bridge::{ms_get_save_monster_defeat, ms_set_monster_check};
            let t = ms_get_save_monster_defeat(mem);
            ms_set_monster_check(mem, u32::from(mem.read::<u16>(v.wrapping_add(0xe))), t);
            if byte(mem, v, 0xdd0) != 0 {
                let monster = u32::from(mem.read::<u16>(v.wrapping_add(0x6d6)));
                let call = &mut *s.call;
                let got = crate::state::bridge::ms_monster_capture(mem, j, monster, |m| {
                    let f = code(m, TO_BTL_CLOSE_SIGN_WINDOW);
                    call(m, f, &[]);
                });
                mem.write(v.wrapping_add(0xdd0), got.to_le_bytes()[0]);
            }
            mem.write(v.wrapping_add(0xdd1), byte(mem, v, 0xdd0));
            let n = crate::menu::bridge::END_ALT.get(mem).wrapping_add(1);
            crate::menu::bridge::END_ALT.set(mem, n);
            crate::flow::bridge::ms_save_add_info_data(mem, i.cast_signed(), 1);
            if mem.read::<i32>(v.wrapping_add(0xf60)) <= 0 && NO_OVERKILL.get(mem) == 0 {
                mem.write(v.wrapping_add(0xf5f), 1u8);
                crate::input::bridge::ms_number_regist(mem, j, 5, -1, -1, 0, 0, 0x81);
            }
        } else {
            mem.write(v.wrapping_add(0xdd0), 0u8);
        }
        crate::flow::bridge::ms_save_add_info_data(mem, j.cast_signed(), 2);
        let status = u32::from(mem.read::<u16>(v.wrapping_add(0x616)));
        mem.write(v.wrapping_add(0xdcc), 1u8);
        crate::minor::bridge::ms_prov_off(mem, j);
        crate::exchange::bridge::ms_threat_off(mem, host, s, j, v);
        crate::flow::bridge::ms_clear_status(mem, j, 0);
        if crate::damage::bridge::ms_get_ram_chr_summon(mem, j) != 0 {
            mem.write(v.wrapping_add(0x5bc), 0u8);
        }
        let st = mem.read::<u16>(v.wrapping_add(0x606));
        mem.write(v.wrapping_add(0x606), st | 1);
        if byte(mem, v, 0x6d0) != 0 {
            let c = crate::actor::bridge::ms_get_chr_command2(mem, j);
            if c != 0 {
                mem.write(c.wrapping_add(3), byte(mem, c, 2).wrapping_add(1));
            }
        }
        crate::action::bridge::ms_clean_input_cue(mem, host, s, j, 1);
        crate::cue::bridge::ms_clean_command_cue(mem, j, 1);
        if byte(mem, v, 0x3f5) != 0 {
            mem.write(v.wrapping_add(0xdc8), 0u8);
        }
        crate::action::bridge::ms_action_request(mem, host, s, j, i, 4, 0, 1, 0);
        if no_reward == 0 {
            let f = code(mem, MS_GET_ITEM);
            let args = [
                i,
                v,
                mem.read::<u32>(v.wrapping_add(0xf88)),
                u32::from(byte(mem, v, 0xf5f)),
                status,
            ];
            (s.call)(mem, f, &args);
        }
        if kind == 1 && ms_auto_relife_process(mem, host, s, j, v, i, status) != 0 {
            ms_auto_phenix_process(mem, host, s, j, v, i);
        }
        ms_limit_type_death_check(mem, i, a, j, v);
        let r = ms_set_death(mem, host, s, j, flags);
        crate::input::bridge::ms_set_chr_weak(mem, j, u32::MAX);
        if r != 0 && byte(mem, v, 0x591) == 0x11 {
            let sisters = (0xf..0x12).all(|k| {
                let c = ms_get_chr(mem, k);
                hp(mem, c) == 0
            });
            if j.wrapping_sub(8) <= 6 || (j.wrapping_sub(0xf) <= 2 && sisters) {
                let f = code(mem, FORCE_AUTO_BATTLE_OFF);
                (s.call)(mem, f, &[]);
            }
        }
        r
    }
}
