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
}
