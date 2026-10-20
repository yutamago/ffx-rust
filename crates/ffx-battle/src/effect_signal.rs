//! The effect signals and the effect motions (`docs/rewrite-plan.md`, M2),
//! round 32.
//!
//! An effect signal is a motion event of type 5 (chara's motion event
//! runner, `0x008343ee`) or the battle script native 0x709e: actor `i`
//! (its owner when it is a part) gets bit `signal` of `+0x39c`, and
//! signals 0-9 run their step (`MsEffectSignal_007b74d0`'s jump table at
//! `0x007b7578`). The effect motions are entries of the effect programs'
//! callback tables (`0x00c6552c`, `0x00c656d4`); nothing shows how their
//! callers use `eax`, so the ports return what the originals leave there.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{IMAGE_BASE, Mem};

    use crate::actor::bridge::{ms_btl_chr_num_check, ms_get_chr, ms_get_chr_top};
    use crate::exec::bridge::Call;

    /// `MsEffectSignalHit_007b75c0(i, actor)`: signal 1.
    pub const MS_EFFECT_SIGNAL_HIT: u32 = 0x007b_75c0;
    /// `MsEffectSignalAvoid_007b7650(i, actor)`: signal 2.
    pub const MS_EFFECT_SIGNAL_AVOID: u32 = 0x007b_7650;
    /// `MsEffectSignalDamage_007b7680(i, actor, signal)`: signals 0 and 9.
    pub const MS_EFFECT_SIGNAL_DAMAGE: u32 = 0x007b_7680;
    /// `MsEffectSignalMotion_007b77c0(i, actor, signal)`: signals 3-8.
    pub const MS_EFFECT_SIGNAL_MOTION: u32 = 0x007b_77c0;
    /// `MsChrSePlay_0079fd40(i, kind)`.
    pub const MS_CHR_SE_PLAY: u32 = 0x0079_fd40;
    /// `MsSetDamage(i, sel, flags)`.
    pub const MS_SET_DAMAGE: u32 = 0x0078_d980;
    /// `MsGetChrEscapePos(i, &vec)`.
    pub const MS_GET_CHR_ESCAPE_POS: u32 = 0x0079_4ec0;
    /// `MsStatusEffectCheck(i)`.
    pub const MS_STATUS_EFFECT_CHECK: u32 = 0x0079_eff0;
    /// `MsSetMotion(i, motion, wait, x, flags, actor, out)`.
    pub const MS_SET_MOTION: u32 = 0x007a_b360;

    /// What `MsEffectSignalDamage_007b7680` hands `MsChrSePlay_0079fd40`
    /// for a damage result (its jump table at `0x007b7798`, indexed by
    /// `0x007b77a8`): none for 0, 1 and 0x11, 2 for 4, 0 for 6, 1 for
    /// every other one (above 0x11 or negative too).
    #[must_use]
    pub fn hit_sound(result: i32) -> Option<u32> {
        match result {
            0 | 1 | 0x11 => None,
            4 => Some(2),
            6 => Some(0),
            _ => Some(1),
        }
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = code(mem, sym);
        call(mem, f, args)
    }

    /// `MsEffectSignal_007b74d0(i, signal)` (`0x007b74d0`, name ours; a
    /// third word is passed and unused): for actor `i & 0xff` in range
    /// with the actors set up (`MsGetChrTop`), or its owner (`+0x14`) when
    /// it is a part (`+0x12`), bit `signal` (mod 32) of `+0x39c` set; then
    /// signal 0 counts `+0x429` and runs
    /// `MsEffectSignalDamage_007b7680(i, actor, 0)`, 1 the hit, 2 the
    /// avoid, 3-8 `MsEffectSignalMotion_007b77c0(i, actor, signal)`, 9
    /// `MsEffectSignalDamage_007b7680(i, actor, 9)`, all through the game.
    /// Always 0.
    pub fn ms_effect_signal<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32, signal: u32) -> u32 {
        let mut i = i & 0xff;
        if ms_btl_chr_num_check(i) == 0 || ms_get_chr_top(mem) == 0 {
            return 0;
        }
        let mut actor = ms_get_chr(mem, i);
        if byte(mem, actor, 0x12) != 0 {
            i = u32::from(byte(mem, actor, 0x14));
            actor = ms_get_chr(mem, i);
        }
        let at = actor.wrapping_add(0x39c);
        let bits: u32 = mem.read(at);
        mem.write(at, bits | 1 << (signal & 31));
        match signal {
            0 => {
                let n = byte(mem, actor, 0x429).wrapping_add(1);
                mem.write(actor.wrapping_add(0x429), n);
                game(mem, call, MS_EFFECT_SIGNAL_DAMAGE, &[i, actor, 0]);
            }
            1 => {
                game(mem, call, MS_EFFECT_SIGNAL_HIT, &[i, actor]);
            }
            2 => {
                game(mem, call, MS_EFFECT_SIGNAL_AVOID, &[i, actor]);
            }
            3..=8 => {
                game(mem, call, MS_EFFECT_SIGNAL_MOTION, &[i, actor, signal]);
            }
            9 => {
                game(mem, call, MS_EFFECT_SIGNAL_DAMAGE, &[i, actor, 9]);
            }
            _ => {}
        }
        0
    }

    /// `MsEffectSignalDamage_007b7680(i, actor, signal)` (`0x007b7680`,
    /// name ours; the effect signals 0 and 9 of [`ms_effect_signal`]): for
    /// an actor whose `+0x446` is 1 or 4, its first target
    /// (`Battle_FirstSelectedActor(i, -16, 0, 1)`) and that one's damage
    /// result (`MsGetDamageResult(i, target, &flags, &kind, &more)`, the
    /// three words at `local`, the original's frame). With `more` set: the
    /// hit sound ([`hit_sound`], or `+0xe1a` when not negative) when
    /// `+0xe19` is set and it is not 0xff (`MsChrSePlay_0079fd40`); with
    /// `flags & 7`, `+0xe16` and an effect at `+0xe0e`, the critical
    /// effect (`flags & 0x100`, `MsEtCriticalEffect`) and that effect
    /// (`MsSetChrEffect8_0079eba0(i, 0, effect)`). Then signal 0 on a
    /// `+0x446` of 1 is `MsSetDamage(i, -5, 0)`, and signal 9 clears
    /// `+0xe19` and `+0xe16`. The sound and the damage go through the
    /// game. Its `eax` is never read (the one caller drops it).
    pub fn ms_effect_signal_damage<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        actor: u32,
        signal: u32,
        local: u32,
    ) {
        let k = byte(mem, actor, 0x446);
        if k != 1 && k != 4 {
            return;
        }
        let (flags, kind, more) = (local, local.wrapping_add(4), local.wrapping_add(8));
        let first = crate::body::bridge::battle_first_selected_actor(mem, i, 0xffff_fff0, 0, 1);
        let r = crate::cue::bridge::ms_get_damage_result(
            mem,
            i,
            first.cast_unsigned(),
            flags,
            kind,
            more,
        );
        if mem.read::<u32>(more) != 0 {
            if let Some(mut se) = hit_sound(r)
                && byte(mem, actor, 0xe19) != 0
            {
                let own: i16 = mem.read(actor.wrapping_add(0xe1a));
                if own >= 0 {
                    se = u32::from(own.cast_unsigned());
                }
                if se != 0xff {
                    game(mem, call, MS_CHR_SE_PLAY, &[i, se]);
                }
            }
            let f: u32 = mem.read(flags);
            if f & 7 != 0 && byte(mem, actor, 0xe16) != 0 {
                let effect = byte(mem, actor, 0xe0e);
                if effect != 0 {
                    if f & 0x100 != 0 {
                        crate::et_effect::bridge::ms_et_critical_effect(mem, call, i);
                    }
                    crate::et_effect::bridge::ms_set_chr_effect8(
                        mem,
                        call,
                        i,
                        0,
                        u32::from(effect),
                    );
                }
            }
        }
        match signal {
            0 if k == 1 => {
                game(mem, call, MS_SET_DAMAGE, &[i, (-5i32).cast_unsigned(), 0]);
            }
            9 => {
                mem.write(actor.wrapping_add(0xe19), 0u8);
                mem.write(actor.wrapping_add(0xe16), 0u8);
            }
            _ => {}
        }
    }

    /// `MsEffectChrMotionIn(i)` (`0x007b7d70`): for actor `i & 0xff`
    /// coming back in (`+0xdff`, cleared): `+0x4fd` down 3 and copied to
    /// `+0x50a`, motion byte `+0x3f0` 0xff, `+0x41c` 1, the move flag 0
    /// (`MsSetChrMoveFlag`), the position reset to its escape position
    /// (`MsGetChrEscapePos` into `local`, 16 bytes, the original's frame;
    /// then `MsSetChrPosReset`), `MsStatusEffectCheck`, the weapon effect
    /// (`MsSetWeaponEffectAttribute`) and motion 0x32
    /// (`MsSetMotion(i, 0x32, 1, 0, 1, 0, 0)`, whose `eax` it returns).
    /// The escape position, the status check and the motion go through
    /// the game. Not coming in: the actor (`MsGetChr`'s `eax`).
    pub fn ms_effect_chr_motion_in<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        local: u32,
    ) -> u32 {
        let i = i & 0xff;
        let actor = ms_get_chr(mem, i);
        if byte(mem, actor, 0xdff) == 0 {
            return actor;
        }
        let v = byte(mem, actor, 0x4fd).wrapping_sub(3);
        mem.write(actor.wrapping_add(0x4fd), v);
        mem.write(actor.wrapping_add(0xdff), 0u8);
        mem.write(actor.wrapping_add(0x50a), v);
        mem.write(actor.wrapping_add(0x3f0), 0xffu8);
        mem.write(actor.wrapping_add(0x41c), 1u8);
        crate::body::bridge::ms_set_chr_move_flag(mem, i, 0);
        game(mem, call, MS_GET_CHR_ESCAPE_POS, &[i, local]);
        crate::chr_damage::bridge::ms_set_chr_pos_reset(mem, i, local);
        game(mem, call, MS_STATUS_EFFECT_CHECK, &[i]);
        crate::et_effect::bridge::ms_set_weapon_effect_attribute(mem, call, i, actor);
        game(mem, call, MS_SET_MOTION, &[i, 0x32, 1, 0, 1, 0, 0])
    }

    /// `MsEffectMagicMotion(w)` (`0x00787c30`): for kind `w >> 8 & 0xff`
    /// 2, actor `w & 0xff`: `+0x405` cleared, the motion lock released
    /// (`MsEffectResetMotionDisable`), then the magic motion `+0xe01`
    /// unless 0xff (`MsSetMotion(i, motion, 1, 0, 1, 0, 0)` through the
    /// game, with `+0x41b` 1 during the call and restored after).
    /// `eax`: the kind minus 2 for other kinds, else the last callee's.
    pub fn ms_effect_magic_motion<M: Mem>(mem: &mut M, call: Call<'_, M>, w: u32) -> u32 {
        let (i, kind) = (w & 0xff, w >> 8 & 0xff);
        if kind != 2 {
            return kind.wrapping_sub(2);
        }
        let actor = ms_get_chr(mem, i);
        let motion = byte(mem, actor, 0xe01);
        mem.write(actor.wrapping_add(0x405), 0u8);
        let r = crate::minor::bridge::ms_effect_reset_motion_disable(mem, actor);
        if motion == 0xff {
            return r;
        }
        let keep = byte(mem, actor, 0x41b);
        mem.write(actor.wrapping_add(0x41b), 1u8);
        let r = game(
            mem,
            call,
            MS_SET_MOTION,
            &[i, u32::from(motion), 1, 0, 1, 0, 0],
        );
        mem.write(actor.wrapping_add(0x41b), keep);
        r
    }
}
