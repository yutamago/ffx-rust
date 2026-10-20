//! The actors' effects started through vfx's `op_et_eff`
//! (`docs/rewrite-plan.md`, M2): the battle scripts' effect natives and
//! the helpers behind them (`MsSetChrEffect_0079eb10`, the guard, weapon
//! attribute, critical and magic-start effects), round 31.
//!
//! Each takes an ID from `MsGetEffID` (a per-actor counter, one per effect
//! kind) and hands it to `op_et_eff` through the game (`call`): the effect
//! started when that returns a positive number.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::exec::bridge::Call;
    use crate::status::bridge::EFFECT_OWNER;
    use crate::util::bridge::ms_get_eff_id;

    /// `op_et_eff(owner, group, effect, id)` (vfx).
    pub const OP_ET_EFF: u32 = 0x007f_c2e0;
    /// The battle debug menu's toggle 0xcc (env option 0x1f): effects
    /// [`ms_set_chr_effect`] starts with `global` set go to owner 0 (the
    /// battle's) instead of the actor's `+0x4c` (`0x0112a917`).
    pub const DEBUG_EFFECT_NO_OWNER: Global<u8> = Global::new(0x0112_a917);

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    /// The next ID of the counter (`u16`) at `actor + counter`, counted on:
    /// `MsGetEffID(count, kind, i)`.
    fn next_id(mem: &mut impl Mem, i: u32, actor: u32, counter: u32, kind: u32) -> u32 {
        let at = actor.wrapping_add(counter);
        let n: u16 = mem.read(at);
        mem.write(at, n.wrapping_add(1));
        ms_get_eff_id(u32::from(n), kind, i)
    }

    /// `op_et_eff(owner, group, effect, id)` through the game: did the
    /// effect start (a positive result)?
    fn et_eff<M: Mem>(mem: &mut M, call: Call<'_, M>, args: [u32; 4]) -> bool {
        let f = code(mem, OP_ET_EFF);
        call(mem, f, &args).cast_signed() > 0
    }

    /// `MsSetChrEffect_0079eb10(i, group, kind, effect, global)`
    /// (`0x0079eb10`): effect `effect` of group `group` for actor `i` (none
    /// for 0xff), its ID from counter `+0xe22` when `kind` is 1, else
    /// `+0xe1e` (`MsGetEffID(n, kind, i)`); owned by the actor's `+0x4c`,
    /// or the battle (0) when `global` and [`DEBUG_EFFECT_NO_OWNER`].
    /// The ID once started, else 0.
    pub fn ms_set_chr_effect<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        group: u32,
        kind: u32,
        effect: u32,
        global: u32,
    ) -> u32 {
        if i == 0xff {
            return 0;
        }
        let actor = ms_get_chr(mem, i);
        let counter = if kind == 1 { 0xe22 } else { 0xe1e };
        let id = next_id(mem, i, actor, counter, kind);
        let owner = if global != 0 && DEBUG_EFFECT_NO_OWNER.get(mem) != 0 {
            0
        } else {
            mem.read(actor.wrapping_add(0x4c))
        };
        if et_eff(mem, call, [owner, group, effect, id]) {
            id
        } else {
            0
        }
    }

    /// `0x0079eba0(i, group, effect)` (`FUN_0079eba0`): a kind 8 effect for
    /// actor `i` ([`ms_set_chr_effect`], `global`), counted at `+0xe13`
    /// once started: 1, else 0.
    pub fn ms_set_chr_effect8<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        group: u32,
        effect: u32,
    ) -> u32 {
        if ms_set_chr_effect(mem, call, i, group, 8, effect, 1) == 0 {
            return 0;
        }
        let at = ms_get_chr(mem, i).wrapping_add(0xe13);
        let n: u8 = mem.read(at);
        mem.write(at, n.wrapping_add(1));
        1
    }

    /// An effect of the battle's owner for actor `i` with an ID of counter
    /// `+0xe1e` (kind `kind`), counted at `actor + count` once started: 1,
    /// else 0 (`0x0079ec40` and `0x0079eca0`).
    fn owner_effect_counted<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        group: u32,
        effect: u32,
        kind: u32,
        count: u32,
    ) -> u32 {
        let actor = ms_get_chr(mem, i);
        let id = next_id(mem, i, actor, 0xe1e, kind);
        let owner = EFFECT_OWNER.get(mem);
        if !et_eff(mem, call, [owner, group, effect, id]) {
            return 0;
        }
        let at = actor.wrapping_add(count);
        let n: u8 = mem.read(at);
        mem.write(at, n.wrapping_add(1));
        1
    }

    /// `0x0079ec40(i, group, effect)` (`FUN_0079ec40`): a kind 8 effect of
    /// the battle's owner for actor `i`, counted at `+0xe13`: 1 once
    /// started, else 0.
    pub fn ms_set_owner_effect8<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        group: u32,
        effect: u32,
    ) -> u32 {
        owner_effect_counted(mem, call, i, group, effect, 8, 0xe13)
    }

    /// `0x0079eca0(i, group, effect)` (`FUN_0079eca0`): a kind 7 effect of
    /// the battle's owner for actor `i`, counted at `+0xe12`: 1 once
    /// started, else 0.
    pub fn ms_set_owner_effect7<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        group: u32,
        effect: u32,
    ) -> u32 {
        owner_effect_counted(mem, call, i, group, effect, 7, 0xe12)
    }

    /// `MsEtCriticalEffect(i)` (`0x0079e590`): the critical hit's effect
    /// (group 0, 0x36) of the battle's owner for actor `i`, its ID from
    /// counter `+0xe1e` (kind 0xc); the ID, started or not.
    pub fn ms_et_critical_effect<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32) -> u32 {
        let actor = ms_get_chr(mem, i);
        let id = next_id(mem, i, actor, 0xe1e, 0xc);
        let owner = EFFECT_OWNER.get(mem);
        et_eff(mem, call, [owner, 0, 0x36, id]);
        id
    }

    /// `MsEffectSetGuard(i)` (`0x0079e510`): for an actor in range
    /// (`MsBtlChrNumCheck`) with guard flags `+0x616` 0x40 (effect 0x48)
    /// or 0x2000 (effect 0x4a, winning): sound 0x77 (`reg_se`, `MsRegSEplay`)
    /// and the effect ([`ms_set_owner_effect8`], group 0). 0.
    pub fn ms_effect_set_guard<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        reg_se: &mut dyn FnMut(&mut M, u32, u32) -> u32,
        i: u32,
    ) -> u32 {
        if crate::actor::bridge::ms_btl_chr_num_check(i) == 0 {
            return 0;
        }
        let flags: u16 = mem.read(ms_get_chr(mem, i).wrapping_add(0x616));
        let effect = if flags & 0x2000 != 0 {
            0x4a
        } else if flags & 0x40 != 0 {
            0x48
        } else {
            return 0;
        };
        reg_se(mem, i, 0x77);
        ms_set_owner_effect8(mem, call, i, 0, effect);
        0
    }

    /// `MsSetWeaponEffectAttribute(i, actor)` (`0x0079ed40`): for an actor
    /// without `+0xdcd`, the weapon's attribute effects 0x3c-0x3f (group 0,
    /// the battle's owner) for each bit 0-3 of `+0x5d9` whose slot
    /// `+0xe90` (a word each) is empty, their IDs from counter `+0xe28`
    /// (kind 0xd) and kept in the slots once started. How many started.
    pub fn ms_set_weapon_effect_attribute<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        actor: u32,
    ) -> u32 {
        if byte(mem, actor, 0xdcd) != 0 {
            return 0;
        }
        let bits = byte(mem, actor, 0x5d9);
        let mut started = 0;
        for k in 0..4 {
            let slot = actor.wrapping_add(0xe90 + 4 * k);
            if (bits >> k) & 1 == 0 || mem.read::<u32>(slot) != 0 {
                continue;
            }
            let id = next_id(mem, i, actor, 0xe28, 0xd);
            let owner = EFFECT_OWNER.get(mem);
            if et_eff(mem, call, [owner, 0, 0x3c + k, id]) {
                started += 1;
                mem.write(slot, id);
            }
        }
        started
    }

    /// `MsMagicEffectStart(i, summon)` (`0x0079e8c0`): the casting effect
    /// for actor `i` (0 for 0xff) unless one runs (`+0xea0`) or it has
    /// none (`+0xe0f` 0). A summon (`summon` set): 0x20, 0x21 under water
    /// or for an actor `+0x505`; else 0x34 (0x35 under water or `+0x505`)
    /// for an actor `+0x5c0`, else `+0xe10` (`+0xe11`), none for 0. Its
    /// sound (`reg_se`, `MsRegSEplay`): 0x19 for effects 0x1e, 0x1f, 0x37
    /// and 0x38, 0x1a for 0x20 and 0x21 (the summoner's `+0xf76` too),
    /// 0x3e for 0x34 and 0x35, else none; kept at `+0xf76`. The effect
    /// (group 0, the battle's owner, ID from counter `+0xe24`, kind 5) is
    /// kept at `+0xea0`. -1 for an actor (0 for 0xff).
    pub fn ms_magic_effect_start<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        reg_se: &mut dyn FnMut(&mut M, u32, u32) -> u32,
        i: u32,
        summon: u32,
    ) -> u32 {
        use crate::state::bridge::UNDERWATER;
        if i == 0xff {
            return 0;
        }
        let actor = ms_get_chr(mem, i);
        if mem.read::<u32>(actor.wrapping_add(0xea0)) != 0 || byte(mem, actor, 0xe0f) == 0 {
            return u32::MAX;
        }
        let wet = UNDERWATER.get(mem) != 0 || byte(mem, actor, 0x505) != 0;
        let effect = if summon != 0 {
            if wet { 0x21 } else { 0x20 }
        } else if byte(mem, actor, 0x5c0) != 0 {
            if wet { 0x35 } else { 0x34 }
        } else {
            let e = u32::from(byte(mem, actor, if wet { 0xe11 } else { 0xe10 }));
            if e == 0 {
                return u32::MAX;
            }
            e
        };
        let mut summoner = 0;
        let se: u16 = match effect {
            0x1e | 0x1f | 0x37 | 0x38 => 0x19,
            0x20 | 0x21 => {
                let g = i32::from(crate::target::bridge::SUMMONER.get(mem)).cast_unsigned();
                summoner = ms_get_chr(mem, g);
                0x1a
            }
            0x34 | 0x35 => 0x3e,
            _ => 0,
        };
        reg_se(mem, i, u32::from(se));
        let id = next_id(mem, i, actor, 0xe24, 5);
        mem.write(actor.wrapping_add(0xf76), se);
        if summoner != 0 {
            mem.write(summoner.wrapping_add(0xf76), se);
        }
        let owner = EFFECT_OWNER.get(mem);
        et_eff(mem, call, [owner, 0, effect, id]);
        mem.write(actor.wrapping_add(0xea0), id);
        u32::MAX
    }
}
