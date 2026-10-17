//! The end of a command's damage step (`docs/rewrite-plan.md`, M2): what
//! `MsBattleProcess_ChrDamage_007911e0` (`0x007911e0`) runs once an
//! actor's command is over: its effects freed, a summoned aeon placed, the
//! party back from a summon, characters freed.
//!
//! Other modules' functions (script, vfx, sound, the C runtime's free) go
//! to the game's code through `s.call`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices and actors
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::Script;
    use crate::exec::bridge::Call;
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

    /// `AtelRemoveCtrlWork` (script).
    pub const ATEL_REMOVE_CTRL_WORK: u32 = 0x0086_ed40;
    /// `op_ovl_effect_free` (vfx).
    pub const OP_OVL_EFFECT_FREE: u32 = 0x007f_cd60;
    /// `SndStopBattleStream` (sound).
    pub const SND_STOP_BATTLE_STREAM: u32 = 0x0088_7760;
    /// `MagicFile_Stop` (system).
    pub const MAGIC_FILE_STOP: u32 = 0x009d_a880;
    /// The magic effect's state (`MsEffectFree` 0xff).
    pub const MAGIC_STATE: Global<u8> = Global::new(0x0112_c9e1);
    /// The debug camera's sleep value (`MsEffectFree` copies it from the
    /// next byte).
    pub const DEBUG_CAMERA_SLEEP: Global<[u8; 2]> = Global::new(0x0112_a8fc);

    /// `0x007a0b90(se)` (`MsSEP3DFind`, name ours): the first 3D sound
    /// node playing `se` in the list ([`crate::minor::bridge::SEP_3D`]),
    /// or 0.
    pub fn ms_sep_3d_find(mem: &impl Mem, se: u32) -> u32 {
        let mut n = crate::minor::bridge::SEP_3D.get(mem);
        while n != 0 {
            if mem.read::<u32>(n.wrapping_add(8)) == se {
                return n;
            }
            n = mem.read(n.wrapping_add(0xc));
        }
        0
    }

    /// `MsSEP3DfadeOut(se, frames)` (`0x007a08a0`): a playing sound not
    /// fading yet (`+4` 0) fades out over `frames` (`+4` and `+6`): 1.
    /// Else 0.
    pub fn ms_sep_3d_fade_out(mem: &mut impl Mem, se: u32, frames: u32) -> u32 {
        let n = ms_sep_3d_find(mem, se);
        if n == 0 || mem.read::<u16>(n + 4) != 0 {
            return 0;
        }
        let f = frames.to_le_bytes();
        let f = u16::from_le_bytes([f[0], f[1]]);
        mem.write(n + 4, f);
        mem.write(n + 6, f);
        1
    }

    /// `MsRegSEfade(i, se, frames)` (`0x007a00d0`): sound `se` (bit 31
    /// set) fades out ([`ms_sep_3d_fade_out`]): -1; 0 for no sound.
    pub fn ms_reg_se_fade(mem: &mut impl Mem, i: u32, se: u32, frames: u32) -> i32 {
        let _ = i;
        if se == 0 {
            return 0;
        }
        ms_sep_3d_fade_out(mem, se | 0x8000_0000, frames);
        -1
    }

    /// `MsAtelRequestCancel(kind, i, n)` (`0x00797cd0`): handler `n` of
    /// script owner `i` ([`crate::atel::bridge::ms_atel_get_handler`])
    /// stops: its work's request skipped and returned
    /// (`AtelSkipReqLevel2`, `AtelExecReturn2`) and its signals dropped
    /// (`AtelDecodeSignal(w, -1)`): 0. -1 without one. The current
    /// control is put back (`AtelChangeCurrentCtrl(kind)` first).
    pub fn ms_atel_request_cancel<'d>(
        mem: &mut Dm<'d>,
        s: &mut Script<'_, Dm<'d>>,
        kind: u32,
        i: u32,
        n: i32,
    ) -> i32 {
        use crate::atel::bridge as at;
        let prev = game(mem, &mut *s.call, at::ATEL_CHANGE_CURRENT_CTRL, &[kind]);
        let r = mem.locals(4, |mem, base| {
            let h = at::ms_atel_get_handler(mem, kind, i, n, base);
            if h == 0 {
                return -1;
            }
            let w = u32::from(byte(mem, h, 0)).wrapping_add(mem.read(base));
            game(mem, &mut *s.call, at::ATEL_SKIP_REQ_LEVEL2, &[w, 0]);
            game(mem, &mut *s.call, at::ATEL_EXEC_RETURN2, &[w, 0, 0]);
            game(mem, &mut *s.call, at::ATEL_DECODE_SIGNAL, &[w, u32::MAX]);
            0
        });
        game(mem, &mut *s.call, at::ATEL_SET_CURRENT_CTRL, &[prev]);
        r
    }

    /// `MsEffectFree(i)` (`0x00787af0`): actor `i`'s magic effect
    /// (`+0xdfb` 6) ends: [`MAGIC_STATE`] 0xff, the last magic
    /// ([`crate::parse::bridge::LAST_MAGIC`]) -1; its scripts (`+0xdfe` 2:
    /// handlers 0-2 of kind 2, [`ms_atel_request_cancel`]) stopped; the
    /// script camera freed; the effect read buffer
    /// ([`crate::parse::bridge::EFFECT_READ_STAT`]) freed; a loaded effect
    /// (`+0xf8c`) ended (the debug camera's sleep value from the next
    /// byte, its `+4` called with 0, `op_ovl_effect_free`,
    /// `SndStopBattleStream`, `MagicFile_Stop`); `+0xdfb` cleared.
    /// (`FlushCache` around the call is empty in this build.)
    pub fn ms_effect_free<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>, i: u32) {
        use crate::parse::bridge::EFFECT_READ_STAT;
        if crate::actor::bridge::ms_btl_chr_num_check(i) == 0 {
            return;
        }
        let a = ms_get_chr(mem, i);
        if byte(mem, a, 0xdfb) != 6 {
            return;
        }
        MAGIC_STATE.set(mem, 0xff);
        crate::parse::bridge::LAST_MAGIC.set(mem, u32::MAX);
        if byte(mem, a, 0xdfe) == 2 {
            for n in 0..3 {
                ms_atel_request_cancel(mem, s, 2, 0, n);
            }
            mem.write(a.wrapping_add(0xdfe), 0u8);
        }
        let call = &mut *s.call;
        crate::flow::bridge::ms_atel_cam_free(mem, |m, k| {
            game(m, &mut *call, ATEL_REMOVE_CTRL_WORK, &[k]);
        });
        let buf = EFFECT_READ_STAT.get(mem);
        if buf != 0 {
            game(mem, &mut *s.call, crate::process::bridge::USER_FREE, &[buf]);
            EFFECT_READ_STAT.set(mem, 0);
        }
        let eff: u32 = mem.read(a.wrapping_add(0xf8c));
        if eff != 0 {
            let [_, next] = DEBUG_CAMERA_SLEEP.get(mem);
            DEBUG_CAMERA_SLEEP.set(mem, [next, next]);
            let f: u32 = mem.read(eff.wrapping_add(4));
            (s.call)(mem, f, &[0]);
            game(mem, &mut *s.call, OP_OVL_EFFECT_FREE, &[]);
            game(mem, &mut *s.call, SND_STOP_BATTLE_STREAM, &[]);
            mem.write(a.wrapping_add(0xf8c), 0u32);
            game(mem, &mut *s.call, MAGIC_FILE_STOP, &[]);
        }
        mem.write(a.wrapping_add(0xdfb), 0u8);
    }

    /// `MsSetChrPosAfterSummon(i)` (`0x007ac6a0`): the aeon `i` takes the
    /// field. Each of the battle party
    /// ([`crate::ability::bridge::PARTY`]): its end direction, `+0x508`,
    /// `+0x6df`, `+0x42e`, `+0x415` cleared, back at its battle position
    /// facing its direction `+0x514[+0x511]`, shown, its status effects
    /// redone. The aeon: body part 3 off, its end direction, `+0x4fd`
    /// 0xd, its battle position pushed, its sound `+0xf76` faded over 20
    /// frames, its own weight; then its entrance (motion 1, waiting) unless
    /// a party member's `+4` is a command 0x3001-0x300b but 0x3005, when
    /// it stands at its battle position (`+0x41c`, motion 0).
    pub fn ms_set_chr_pos_after_summon<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
    ) {
        use crate::chr_damage::bridge::{ms_set_chr_dir_f, ms_set_chr_pos_reset};
        use crate::pos::bridge::ms_get_chr_btl_pos;
        mem.locals(0x10, |mem, v| {
            let mut entrance = true;
            let party = crate::ability::bridge::PARTY.get(mem);
            for q in party {
                if q == 0xff {
                    continue;
                }
                let q = u32::from(q);
                let a = ms_get_chr(mem, q);
                crate::actor::bridge::ms_set_end_dir(mem, a);
                let cmd: u32 = mem.read(a.wrapping_add(4));
                if (0x3001..=0x300b).contains(&cmd) && cmd != 0x3005 {
                    entrance = false;
                }
                for o in [0x508, 0x6df, 0x42e, 0x415] {
                    mem.write(a.wrapping_add(o), 0u8);
                }
                ms_get_chr_btl_pos(mem, host.trig, q, v);
                ms_set_chr_pos_reset(mem, q, v);
                let k = i32::from(mem.read::<i8>(a.wrapping_add(0x511)));
                let dir: u32 = mem.read(a.wrapping_add(0x514).wrapping_add_signed(4 * k));
                ms_set_chr_dir_f(mem, q, dir);
                crate::visible::bridge::ms_set_chr_visible_appear(mem, &mut *s.call, q, 1);
                crate::status::bridge::ms_status_effect_check(mem, &mut *s.call, &mut *s.off, q);
            }
            let a = ms_get_chr(mem, i);
            crate::body::bridge::ms_set_chr_body_hit(mem, a, 3, 0);
            crate::actor::bridge::ms_set_end_dir(mem, a);
            mem.write(a.wrapping_add(0x4fd), 0xdu8);
            crate::actor::bridge::ms_push_btl_pos(mem, a);
            let se = u32::from(mem.read::<u16>(a.wrapping_add(0xf76)));
            ms_reg_se_fade(mem, i, se, 0x14);
            crate::body::bridge::ms_set_chr_weight(mem, i, 1);
            if entrance {
                crate::atel::bridge::ms_set_motion(mem, s, i, 1, 1, 0, 1, 0, 0);
                return;
            }
            mem.write(a.wrapping_add(0x41c), 1u8);
            ms_get_chr_btl_pos(mem, host.trig, i, v);
            ms_set_chr_pos_reset(mem, i, v);
            crate::atel::bridge::ms_set_motion(mem, s, i, 0, 1, 0, 1, 0, 0);
        });
    }

    /// `Ch_ModelPurgePacket`, `Ch_Dispose`, `ChN_DisposeSystemMGRP`,
    /// `ChN_DataDispose` (chara) and `Sg_FreeA` (main).
    pub const CH_MODEL_PURGE_PACKET: u32 = 0x0082_7700;
    pub const CH_DISPOSE: u32 = 0x0082_66f0;
    pub const CHN_DISPOSE_SYSTEM_MGRP: u32 = 0x0083_6830;
    pub const CHN_DATA_DISPOSE: u32 = 0x0082_4f00;
    pub const SG_FREE_A: u32 = 0x0083_f090;
    /// `op_et_bindeff_off_signal` (vfx).
    pub const OP_ET_BINDEFF_OFF_SIGNAL: u32 = 0x007f_bcd0;

    /// A record's model (`+0`, an actor's or a weapon's) by `flags`: bit
    /// 3 its packets purged (`+0x3a` set), bit 0 the model disposed and
    /// cleared.
    fn dispose_model<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>, rec: u32, flags: u32) {
        let model: u32 = mem.read(rec);
        if model == 0 {
            return;
        }
        if flags & 8 != 0 {
            game(mem, &mut *s.call, CH_MODEL_PURGE_PACKET, &[model, 1]);
            mem.write(rec.wrapping_add(0x3a), 1u8);
        }
        if flags & 1 != 0 {
            game(mem, &mut *s.call, CH_DISPOSE, &[model]);
            mem.write(rec, 0u32);
        }
    }

    /// `MsDebugFullCommand_00796dc0(i, actor, flags)` (`0x00796dc0`; what
    /// it does: `MsChrDispose`, name ours): the actor's model goes by
    /// `flags` (bit 3 its packets, bit 0 the model); a loaded actor
    /// (`+0x11`, not `+0x12`) also loses its weapon effect
    /// ([`crate::exchange::bridge::ms_effect_weapon_free`]), its motion
    /// group (bit 2) and data (bit 1) unless shared (`+0x3e`), its
    /// allocation `+0x48` (`Sg_FreeA(p, 5)`) and, for an actor below 0x1f,
    /// its sound slots ([`crate::exec::bridge::ms_sep_chr_clear`]): 1.
    /// Else 0.
    pub fn ms_chr_dispose<'d>(
        mem: &mut Dm<'d>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
        flags: u32,
    ) -> u32 {
        dispose_model(mem, s, actor, flags);
        if byte(mem, actor, 0x11) == 0 || byte(mem, actor, 0x12) != 0 {
            return 0;
        }
        crate::exchange::bridge::ms_effect_weapon_free(mem, s, i);
        let mgrp: u32 = mem.read(actor.wrapping_add(8));
        let data: u32 = mem.read(actor.wrapping_add(4));
        if mgrp != 0 && flags & 4 != 0 && byte(mem, actor, 0x3e) == 0 {
            let slot = i32::from(mem.read::<i8>(actor.wrapping_add(0x20))).cast_unsigned();
            game(mem, &mut *s.call, CHN_DISPOSE_SYSTEM_MGRP, &[mgrp, slot]);
            mem.write(actor.wrapping_add(0x38), 0u8);
        }
        if data != 0 && flags & 2 != 0 && byte(mem, actor, 0x3e) == 0 {
            game(mem, &mut *s.call, CHN_DATA_DISPOSE, &[data, 0]);
        }
        let p: u32 = mem.read(actor.wrapping_add(0x48));
        if p != 0 {
            game(mem, &mut *s.call, SG_FREE_A, &[p, 5]);
            mem.write(actor.wrapping_add(0x48), 0u32);
        }
        if i.cast_signed() < 0x1f {
            crate::exec::bridge::ms_sep_chr_clear(mem, &mut *s.call, actor, 0);
        }
        1
    }

    /// `MsWeaponDispose(i, k, flags, weapon)` (`0x007ad750`): a weapon
    /// record gives up its model (bits 3 and 0), motion group (bit 2) and
    /// data (bit 1, both kept when shared, `+0x3e`), each cleared; a
    /// special weapon actor for it
    /// ([`crate::actor::bridge::ms_get_weapon_special`]) is freed and takes
    /// what is left (`+0`, `+4`, `+8`, `+0x38`).
    pub fn ms_weapon_dispose<'d>(
        mem: &mut Dm<'d>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        k: u32,
        flags: u32,
        weapon: u32,
    ) {
        if weapon == 0 {
            return;
        }
        dispose_model(mem, s, weapon, flags);
        let mgrp: u32 = mem.read(weapon + 8);
        if mgrp != 0 && flags & 4 != 0 && byte(mem, weapon, 0x3e) == 0 {
            let slot = i32::from(mem.read::<i8>(weapon + 0x20)).cast_unsigned();
            game(mem, &mut *s.call, CHN_DISPOSE_SYSTEM_MGRP, &[mgrp, slot]);
            mem.write(weapon + 8, 0u32);
            mem.write(weapon + 0x38, 0u8);
        }
        let data: u32 = mem.read(weapon + 4);
        if data != 0 && flags & 2 != 0 && byte(mem, weapon, 0x3e) == 0 {
            game(mem, &mut *s.call, CHN_DATA_DISPOSE, &[data, 0]);
            mem.write(weapon + 4, 0u32);
        }
        let sp = crate::actor::bridge::ms_get_weapon_special(mem, i, k);
        if sp != 0 {
            crate::actor::bridge::ms_btl_chr_info_free(mem, sp);
            let v: [u32; 3] = mem.read(weapon);
            mem.write(sp, v);
            mem.write(sp + 0x38, byte(mem, weapon, 0x38));
        }
    }

    /// `MsBtlChrFree(i, flags)` (`0x00793410`): actor `i` leaves: hidden
    /// ([`crate::visible::bridge::ms_set_chr_visible_appear`]), its bind
    /// and status effects ended, its script handler `i + 5` of kind 2
    /// cancelled ([`ms_atel_request_cancel`]), its model and weapons
    /// disposed by `flags` ([`ms_chr_dispose`], [`ms_weapon_dispose`]),
    /// its info freed ([`crate::actor::bridge::ms_btl_chr_info_free`]).
    pub fn ms_btl_chr_free<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>, i: u32, flags: u32) {
        let a = ms_get_chr(mem, i);
        crate::visible::bridge::ms_set_chr_visible_appear(mem, &mut *s.call, i, 0);
        let call = &mut *s.call;
        crate::flow::bridge::ms_reset_bind_effect(mem, i, |m, c| {
            game(m, &mut *call, OP_ET_BINDEFF_OFF_SIGNAL, &[c]);
        });
        crate::flow::bridge::ms_status_effect_clear(mem, i, &mut *s.off);
        ms_atel_request_cancel(mem, s, 2, 0, i.wrapping_add(5).cast_signed());
        ms_chr_dispose(mem, s, i, a, flags);
        for k in 0..2 {
            let w = crate::actor::bridge::ms_get_weapon(mem, i, k);
            ms_weapon_dispose(mem, s, i, k.cast_unsigned(), flags, w);
        }
        crate::actor::bridge::ms_btl_chr_info_free(mem, a);
    }

    /// `0x007ae0a0(i)` (`MsFreePlayer`, name ours): party member `i`
    /// leaves the field: `+0xdc8` cleared, its input entries gone
    /// (`MsCleanInputCue(i, 0)`), no longer the turn's last actor
    /// ([`crate::action::bridge::TURN_END_ACTOR`]), and with a model
    /// freed ([`ms_btl_chr_free`], flags 7).
    pub fn ms_free_player<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
    ) {
        use crate::action::bridge::TURN_END_ACTOR;
        let a = ms_get_chr(mem, i);
        mem.write(a.wrapping_add(0xdc8), 0u8);
        crate::action::bridge::ms_clean_input_cue(mem, host, s, i, 0);
        if i32::from(TURN_END_ACTOR.get(mem).cast_signed()) == i.cast_signed() {
            TURN_END_ACTOR.set(mem, 0xff);
        }
        if mem.read::<u32>(a) != 0 {
            ms_btl_chr_free(mem, s, i, 7);
        }
    }

    /// `MsExchangePlayer()` (`0x007adf10`): each of the battle party comes
    /// back to its own battle position (`+0x4fc` from `+0x500`) and joins
    /// (`MsExchangeChr(0xff, q, mode, 0)`, mode -1 for the aeon's slot
    /// [`crate::ability::bridge::LIST_SLOT`], else 0).
    pub fn ms_exchange_player<'d>(mem: &mut Dm<'d>, host: &Host<'_>, s: &mut Script<'_, Dm<'d>>) {
        for k in 0..7 {
            let q: u8 = mem.read(crate::ability::bridge::PARTY.ptr(mem).addr + k);
            if q == 0xff {
                continue;
            }
            let q = u32::from(q);
            let a = ms_get_chr(mem, q);
            let slot = i32::from(crate::ability::bridge::LIST_SLOT.get(mem));
            let mode = if q.cast_signed() == slot { -1 } else { 0 };
            let v: [u8; 3] = mem.read(a.wrapping_add(0x500));
            mem.write(a.wrapping_add(0x4fc), v);
            crate::exchange::bridge::ms_exchange_chr(mem, host, s, 0xff, q, mode, 0);
        }
    }

    /// `MsReturnSummon(revive)` (`0x007aef20`): with an aeon out
    /// ([`crate::target::bridge::SUMMONER`] not 0), each of the battle
    /// party gets its gauge back (`MsSummonLimitLoad`), with `revive` and
    /// no HP is revived ([`crate::chr_damage::bridge::ms_relife`]), and
    /// leaves ([`ms_free_player`]); then the party comes back
    /// ([`crate::flow::bridge::ms_return_party`], [`ms_exchange_player`]),
    /// the monsters face it ([`crate::exchange::bridge::ms_change_monster_dir`]),
    /// the party-leaves flag is set and the summoner and slot become 0 and
    /// 0xff.
    pub fn ms_return_summon<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        revive: u32,
    ) {
        use crate::target::bridge::SUMMONER;
        if SUMMONER.get(mem) == 0 {
            return;
        }
        for k in 0..7 {
            let q: u8 = mem.read(crate::ability::bridge::PARTY.ptr(mem).addr + k);
            if q == 0xff {
                continue;
            }
            let q = u32::from(q);
            let a = ms_get_chr(mem, q);
            crate::misc::bridge::ms_summon_limit_load(mem, a);
            if revive != 0 && mem.read::<i32>(a.wrapping_add(0x5d0)) <= 0 {
                crate::chr_damage::bridge::ms_relife(mem, &mut *s.call, q, a, 0);
            }
            ms_free_player(mem, host, s, q);
        }
        crate::flow::bridge::ms_return_party(mem);
        ms_exchange_player(mem, host, s);
        crate::exchange::bridge::ms_change_monster_dir(mem, host, s);
        crate::parse::bridge::PARTY_LEAVES.set(mem, 1);
        SUMMONER.set(mem, 0);
        crate::ability::bridge::LIST_SLOT.set(mem, -1);
    }

    /// Cleared when a command ends (read with the command cue count
    /// elsewhere).
    pub const CUE_FLAG_C9DE: Global<u8> = Global::new(0x0112_c9de);
    /// Two words a script pops (shown as `VS%8x:%4x` by the debug
    /// display), cleared when a command ends.
    pub const SCRIPT_VS: Global<[u32; 2]> = Global::new(0x0112_c24c);
    /// Set while an aeon is entering; cleared once it is placed.
    pub const SUMMON_ENTERING: Global<u8> = Global::new(0x0112_ca10);
    /// Set to wait while a party member is dying
    /// ([`crate::chr_damage::bridge::ms_check_party_dying`]).
    pub const DYING_WAIT: Global<u8> = Global::new(0x0112_ca14);
    /// Asks for the script camera when a command ends, even with commands
    /// queued.
    pub const CAMERA_RESET: Global<u8> = Global::new(0x0112_ca13);
    /// Frames since the command started (counted up elsewhere).
    pub const COMMAND_FRAMES: Global<u32> = Global::new(0x0112_c900);
    /// The debug option that keeps the screen focus when a command ends.
    pub const KEEP_FOCUS: Global<u8> = Global::new(0x0112_a927);
    /// `MagicFile_GetHandle`'s handle (-1 none).
    pub const MAGIC_HANDLE: Global<i32> = Global::new(0x00c6_4cac);
    /// `Sg_Blur` (main).
    pub const SG_BLUR: u32 = 0x0082_bee0;
    /// `op_et_weapon_afterimage_fade` (vfx).
    pub const OP_ET_WEAPON_AFTERIMAGE_FADE: u32 = 0x007f_ca40;

    /// `MsBattleProcess_ChrDamage_007911e0()` (`0x007911e0`): unless files
    /// are being read or sounds moved (`MsBtlReadCueLevel`,
    /// `MsSEPtransAmount`), each of the 31 actors with a command in hand
    /// (`+0xdf0` not 0 or 4). One still hitting (not 5, no `+0xdf2`, no
    /// effect but a magic one, `+0xdfb` 0 or 6) waits while the title
    /// window is up past its top − 20 with `+0x434` and no magic file or
    /// command wait, while a party member dies ([`DYING_WAIT`]), or for the
    /// turn's last actor whose `+0xdc0` holds itself while numbers show;
    /// else its hits land ([`crate::after_damage::bridge::ms_set_damage`]
    /// with selector -5, flags 0x400, until none are left), the last one
    /// poisons ([`crate::chr_damage::bridge::ms_poison_process`]), and the
    /// command's end is checked
    /// ([`crate::chr_damage::bridge::ms_check_command_end`]; also for 5).
    /// A command ended (1): its effect freed ([`ms_effect_free`]), the
    /// title window closed, its block `+0x72c` cleared; a cue entry with
    /// hits left (`+2` counted on, below `+3`) goes back to 4 (`+4`
    /// cleared). The camera ends, the screen focus resets (unless
    /// [`KEEP_FOCUS`]). By `+0xdea`: 1 the aeon placed
    /// ([`ms_set_chr_pos_after_summon`]) with the summoner's voice 12, 2
    /// and 3 the party back ([`ms_return_summon`], 3 reviving), 4 and 5
    /// escapes, 6 the actor freed ([`ms_btl_chr_free`], 0xc), 7 the party
    /// leaves. The music back up, the command state reset (motion busy,
    /// running, reflected, the parsed command 0xff, the cue flags, the
    /// message cue, the script words), its magic effect ended, every
    /// actor's statuses back, the special camera freed, its after-image
    /// faded, `+0xde9`, `+0xf5e` cleared, `+0xe0f` 1, `+0xe1a` 0xffff, its
    /// weight back (`+0x4ac` bit 16). Done for good: `+0xdf0`, `+0x6d0`,
    /// `+0x450` cleared, its cue entry deleted, its weak pose. Its rest
    /// motion when it may ([`crate::chr_damage::bridge::ms_check_rest_motion`]);
    /// the blur off. The first end of a command
    /// ([`crate::parse::bridge::COMMAND_STARTED`]) with the battle going
    /// on, input entries left, the party back or [`CAMERA_RESET`]: the
    /// command frames reset and the script camera (`MsAtelRequestCamera(0,
    /// i, 0x1c, 0xffff, 1, 1)`), or with commands queued the command
    /// camera flag. [`CAMERA_RESET`] cleared.
    #[allow(clippy::too_many_lines)]
    pub fn ms_battle_process_chr_damage<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
    ) {
        use crate::chr_damage::bridge as cd;
        use crate::parse::bridge as pa;
        if crate::state::bridge::ms_btl_read_cue_level(mem) != 0
            || crate::state::bridge::ms_sep_trans_amount(mem) != 0
        {
            return;
        }
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            let state = byte(mem, a, 0xdf0);
            if state == 0 || state == 4 {
                continue;
            }
            if state == 5 {
                cd::ms_check_command_end(mem, a);
            } else if byte(mem, a, 0xdf2) == 0 && matches!(byte(mem, a, 0xdfb), 0 | 6) {
                let [title, top] = crate::exec::bridge::TITLE_WINDOW.get(mem);
                let (title, top) = (i32::from(title.cast_signed()), i32::from(top.cast_signed()));
                if title > 0
                    && title > top - 0x14
                    && byte(mem, a, 0x434) != 0
                    && MAGIC_HANDLE.get(mem) == -1
                    && pa::COMMAND_WAIT.get(mem) == -1
                {
                    continue;
                }
                if DYING_WAIT.get(mem) != 0 {
                    if cd::ms_check_party_dying(mem) != 0 {
                        continue;
                    }
                    DYING_WAIT.set(mem, 0);
                }
                let last = i32::from(crate::action::bridge::TURN_END_ACTOR.get(mem).cast_signed());
                if last == i.cast_signed()
                    && mem.read::<u32>(a.wrapping_add(0xdc0)) & (1 << i) != 0
                    && crate::minor::bridge::NUMBERS_KIND1.get(mem) != 0
                {
                    continue;
                }
                while crate::after_damage::bridge::ms_set_damage(
                    mem,
                    host,
                    s,
                    i,
                    0xffff_fffb,
                    0x400,
                ) != 0
                {}
                let c = crate::actor::bridge::ms_get_chr_command2(mem, i);
                if u32::from(byte(mem, c, 2)) + 1 >= u32::from(byte(mem, c, 3)) {
                    cd::ms_poison_process(mem, host, s, i, a);
                }
                cd::ms_check_command_end(mem, a);
            }
            if byte(mem, a, 0xdf0) != 1 {
                continue;
            }
            end_command(mem, host, s, i, a);
        }
    }

    /// A command's end in [`ms_battle_process_chr_damage`].
    #[allow(clippy::too_many_lines)]
    fn end_command<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        a: u32,
    ) {
        use crate::ability::bridge::LIST_SLOT;
        use crate::chr_damage::bridge as cd;
        use crate::parse::bridge as pa;
        use crate::target::bridge::SUMMONER;
        let mut done = true;
        let mut back = false;
        ms_effect_free(mem, s, i);
        let call = &mut *s.call;
        crate::state::bridge::ms_menu_close_title_window(mem, 0, |m| {
            game(
                m,
                &mut *call,
                crate::death::bridge::TO_BTL_CLOSE_SIGN_WINDOW,
                &[],
            );
        });
        crate::util::bridge::ms_struct_clear(mem, a.wrapping_add(0x72c), 0x48);
        let c = crate::actor::bridge::ms_get_chr_command2(mem, i);
        if c != 0 {
            let n = byte(mem, c, 2).wrapping_add(1);
            mem.write(c + 2, n);
            if n < byte(mem, c, 3) {
                mem.write(a.wrapping_add(0xdf0), 4u8);
                mem.write(c + 4, 0u8);
                done = false;
            }
        }
        crate::minor::bridge::ms_effect_camera_call(mem, -1, 0);
        cd::ms_effect_camera_end(mem, s, 1);
        if KEEP_FOCUS.get(mem) == 0 {
            cd::ms_screen_focus_reset_all(mem, &mut *s.call);
        }
        match mem.read::<i8>(a.wrapping_add(0xdea)) {
            1 => {
                SUMMON_ENTERING.set(mem, 0);
                let slot = i32::from(LIST_SLOT.get(mem)).cast_unsigned();
                ms_set_chr_pos_after_summon(mem, host, s, slot);
                let who = i32::from(SUMMONER.get(mem)).cast_unsigned();
                crate::atel::bridge::ms_set_voice_check(mem, s, who, 0xc, 1, 0, 0);
            }
            2 => {
                back = true;
                ms_return_summon(mem, host, s, 0);
            }
            3 => ms_return_summon(mem, host, s, 1),
            4 => cd::ms_set_escape_flag(mem, &mut *s.call, i, a, 1),
            5 => cd::ms_set_escape_flag_all(mem, &mut *s.call, 1),
            6 => ms_btl_chr_free(mem, s, i, 0xc),
            7 => pa::PARTY_LEAVES.set(mem, 1),
            _ => {}
        }
        if pa::COMMAND_MUSIC_DOWN.get(mem) != 0 {
            pa::COMMAND_MUSIC_DOWN.set(mem, 0);
            crate::exec::bridge::ms_music_volume(mem, &mut *s.call, 0x7f, 0x1e);
        }
        crate::body::bridge::MOTION_BUSY.set(mem, 0);
        pa::COMMAND_RUNNING.set(mem, 0);
        crate::stat::bridge::REFLECTED.set(mem, 0);
        pa::PARSED_COMMAND2.set(mem, 0xff);
        CUE_FLAG_C9DE.set(mem, 0);
        pa::COMMAND_FLAG29.set(mem, 0);
        mem.write(crate::misc::bridge::MESSAGE_CUE_INIT.ptr(mem).addr, 1u8);
        SCRIPT_VS.set(mem, [0, 0]);
        cd::ms_magic_effect_end(mem, &mut *s.call, i);
        cd::ms_command_return_chr_status_all(mem, host, s, u32::MAX);
        crate::misc::bridge::ms_battle_special_camera_free(mem);
        let call = &mut *s.call;
        crate::flow::bridge::ms_reset_after_image(mem, a, |m, w| {
            game(m, &mut *call, OP_ET_WEAPON_AFTERIMAGE_FADE, &[w]);
        });
        mem.write(a.wrapping_add(0xde9), 0u16);
        mem.write(a.wrapping_add(0xf5e), 0u8);
        mem.write(a.wrapping_add(0xe0f), 1u8);
        mem.write(a.wrapping_add(0xe1a), 0xffffu16);
        if mem.read::<u32>(a.wrapping_add(0x4ac)) & 0x1_0000 != 0 {
            crate::body::bridge::ms_set_chr_weight(mem, i, 1);
        }
        if done {
            mem.write(a.wrapping_add(0xdf0), 0u8);
            mem.write(a.wrapping_add(0x6d0), 0u8);
            pa::COMMAND_3029_COUNT.set(mem, 0);
            mem.write(a.wrapping_add(0x450), 0u8);
            let slot = u32::from(byte(mem, a, 0xde5));
            crate::cue::bridge::ms_delete_command_cue(mem, i, slot, 0);
            crate::input::bridge::ms_set_chr_weak(mem, i, u32::MAX);
        }
        if cd::ms_check_rest_motion(mem, i, a) != 0 {
            crate::atel::bridge::ms_set_motion(mem, s, i, 0, 0, 0, 1, 0, 0);
        }
        game(mem, &mut *s.call, SG_BLUR, &[0]);
        if pa::COMMAND_STARTED.get(mem) != 0 {
            let reset = CAMERA_RESET.get(mem);
            let go = crate::menu::bridge::ms_get_battle_end_status(mem) == 0
                || crate::state::bridge::INPUT_CUE_COUNT.get(mem) != 0
                || back
                || reset != 0;
            if go {
                COMMAND_FRAMES.set(mem, 0);
                pa::COMMAND_STARTED.set(mem, 0);
                if crate::state::bridge::COMMAND_CUE_COUNT.get(mem) != 0 && reset == 0 {
                    pa::COMMAND_CAMERA.set(mem, 1);
                } else {
                    crate::atel::bridge::ms_atel_request_camera(
                        mem, s, 0, i, 0x1c, 0xffff, 1, 1, 0, 0,
                    );
                }
            }
        }
        CAMERA_RESET.set(mem, 0);
    }
}
