//! What a hit leaves behind (`docs/rewrite-plan.md`, M2): the pieces of
//! `AfterDamageProcess` (`0x0078f060`), which `MsSetDamage` runs on each
//! target once its damage is set: MP and CTB lost, revivals, the steal
//! effects.
//!
//! The effects are vfx's (`op_et_eff`): they go to the game's code through
//! `s.call`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, amounts
pub mod bridge {
    use ffx_bridge::{IMAGE_BASE, Mem};
    use ffx_kernel::bridge::ms_check_range;

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::Script;
    use crate::exec::bridge::Call;
    use crate::target::bridge::{Dm, Host};

    fn word(mem: &impl Mem, p: u32, o: u32) -> i32 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    /// `MsSubMP(i, actor, n, other, x, flags, style)` (`0x0078e3b0`): actor
    /// `i` loses `n` MP (within 0 and its maximum `+0x598`; none when
    /// `DebugCanTakeDamage` says so), its number (kind 1) shown with
    /// `flags` bit 1.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_sub_mp(
        mem: &mut impl Mem,
        i: u32,
        actor: u32,
        n: i32,
        other: i32,
        x: u32,
        flags: u32,
        style: u32,
    ) {
        if flags & 2 != 0 {
            crate::input::bridge::ms_number_regist(mem, i, 1, n, other, x, flags, style);
        }
        if crate::damage::bridge::debug_can_take_damage(mem, i) != 0 {
            let v = word(mem, actor, 0x5d4).wrapping_sub(n);
            let max = word(mem, actor, 0x598);
            mem.write(actor.wrapping_add(0x5d4), ms_check_range(v, 0, max));
        }
    }

    /// `MsSubCTB(i, actor, n, x, flags, style)` (`0x0078e1e0`): actor `i`'s
    /// wait (`+0x65c`) grows by `n` (within 0 and 0xff), its number (kind
    /// 2) shown with `flags` bit 2.
    pub fn ms_sub_ctb(
        mem: &mut impl Mem,
        i: u32,
        actor: u32,
        n: i32,
        x: u32,
        flags: u32,
        style: u32,
    ) {
        if flags & 4 != 0 {
            crate::input::bridge::ms_number_regist(mem, i, 2, n, n, x, flags, style);
        }
        let w = i32::from(mem.read::<u8>(actor.wrapping_add(0x65c)));
        let v = ms_check_range(w.wrapping_add(n), 0, 0xff);
        mem.write(actor.wrapping_add(0x65c), v.to_le_bytes()[0]);
    }

    /// `MsAliveProcess(i, actor)` (`0x00789160`): Life on a KO'd actor
    /// (`+0x606` bit 0 still set: its HP stays 0, 1); else it comes back
    /// ([`crate::chr_damage::bridge::ms_relife`], full) with its voice 10
    /// (`MsSetVoiceCheck(i, 10, 1)`): 0.
    pub fn ms_alive_process<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, i: u32, actor: u32) -> u32 {
        if mem.read::<u8>(actor.wrapping_add(0x606)) & 1 != 0 {
            mem.write(actor.wrapping_add(0x5d0), 0u32);
            return 1;
        }
        crate::chr_damage::bridge::ms_relife(mem, &mut *s.call, i, actor, 1);
        crate::atel::bridge::ms_set_voice_check(mem, s, i, 10, 1, 0, 0);
        0
    }

    /// `MsAutoRelifeProcess(i, actor, by, helper)` (`0x0078d8d0`): when
    /// `helper` may act (`MsAutoCommandStatusCheck(helper, 1, 1, 1)`):
    /// `actor`'s `+0x6dd` from `+0x446` being 1, and `by`'s command
    /// `helper +0x6c6` on `i` inserted after `i`'s
    /// ([`crate::death::bridge::insert_one`], priority 0): 1; else 0.
    /// `helper +0x6dc` cleared either way.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_auto_relife_command<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
        by: u32,
        helper: u32,
    ) -> u32 {
        let r = if crate::actor::bridge::ms_auto_command_status_check(mem, helper, 1, 1, 1) != 0 {
            let v = mem.read::<u8>(actor.wrapping_add(0x446)) == 1;
            mem.write(actor.wrapping_add(0x6dd), u8::from(v));
            let id = u32::from(mem.read::<u16>(helper.wrapping_add(0x6c6)));
            crate::death::bridge::insert_one(mem, host, s, by & 0xff, i, i, id | 0x00ff_0000, 0);
            1
        } else {
            0
        };
        mem.write(helper.wrapping_add(0x6dc), 0u8);
        r
    }

    /// `0x0079ebe0(i, a, b)` (`MsSetChrEffectNext_0079ebe0`): effect `b`
    /// of group `a` (`op_et_eff` by the battle's owner, through the game)
    /// with actor `i`'s next ID (`MsGetEffID(+0xe2a, 0xe, i)`, `+0xe2a`
    /// counted on); once it started (positive) `COMMAND_HOLD2` counts it:
    /// 1, else 0.
    pub fn ms_set_chr_effect_next<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        a: u32,
        b: u32,
    ) -> u32 {
        let p = ms_get_chr(mem, i);
        let n: u16 = mem.read(p.wrapping_add(0xe2a));
        let id = crate::util::bridge::ms_get_eff_id(u32::from(n), 0xe, i);
        mem.write(p.wrapping_add(0xe2a), n.wrapping_add(1));
        let owner = crate::status::bridge::EFFECT_OWNER.get(mem);
        let f = code(mem, crate::death::bridge::OP_ET_EFF);
        if call(mem, f, &[owner, a, b, id]).cast_signed() > 0 {
            use crate::chr_damage::bridge::COMMAND_HOLD2;
            COMMAND_HOLD2.set(mem, COMMAND_HOLD2.get(mem).wrapping_add(1));
            1
        } else {
            0
        }
    }

    /// `MsSetStealEffect(i, got)` (`0x0079ed00`): the steal effect, 10 when
    /// something was stolen, else 9 ([`ms_set_chr_effect_next`]).
    pub fn ms_set_steal_effect<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32, got: u32) {
        ms_set_chr_effect_next(mem, call, i, 0, 9 + u32::from(got != 0));
    }

    /// `MsSetStealGillEffect(i, got)` (`0x0079ed20`): the gil steal effect,
    /// 0x4d or 0x4c.
    pub fn ms_set_steal_gill_effect<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32, got: u32) {
        ms_set_chr_effect_next(mem, call, i, 0, 0x4c + u32::from(got != 0));
    }
}
