//! The battle's action flow (`docs/rewrite-plan.md`, M2): starting an
//! effect once its actors are free, head targets, ending status effects,
//! the overdrive statuses' HP and MP, the party's return, and the battle
//! records kept per character. Effect calls into `ffx_vfx` (which depends
//! on this crate through `ffx_field`) come from the caller.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // actor, index, value
pub mod bridge {
    use ffx_bridge::{Global, Mem};
    use ffx_kernel::bridge::ms_check_range;

    use crate::actor::bridge::ms_get_chr;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn half(mem: &impl Mem, p: u32, o: u32) -> u16 {
        mem.read(p.wrapping_add(o))
    }

    fn word(mem: &impl Mem, p: u32, o: u32) -> u32 {
        mem.read(p.wrapping_add(o))
    }

    /// Who an effect waits on (`MsEffectCheckMotion`, `MsCheckStartEffect`):
    /// the actor and its motion (0xff: any).
    pub const EFFECT_WAIT: Global<[u8; 2]> = Global::new(0x0112_ca08);
    /// Set: no head target (`MsSetHeadTarget`).
    pub const NO_HEAD_TARGET: Global<u8> = Global::new(0x0112_a926);
    /// Set: the menu always looks (`MsMenuCheckLook`).
    pub const MENU_LOOK: Global<u8> = Global::new(0x0112_a915);
    /// The party saved while it moved freely (`MsReturnParty`), and the 17
    /// bytes after the party it brings back too.
    pub const PARTY_SAVED: Global<[u8; 7]> = Global::new(0x0112_c89c);
    pub const PARTY_EXTRA: Global<[u8; 17]> = Global::new(0x0112_c8a3);
    pub const PARTY_EXTRA_SAVED: Global<[u8; 17]> = Global::new(0x0112_c8b4);
    /// `g_BtlStealRule`: 2 (nothing to steal) also keeps the battle records
    /// from counting (`MsSaveAddInfoData`).
    pub const RECORDS_OFF: Global<u8> = Global::new(0x0112_c9e5);

    /// `MsStatusEffectClear(i)` (`0x0079f110`): ends the actor's 24 status
    /// effects (`+0xe30`, `op_et_eff_off_signal` from the caller); how
    /// many there were.
    pub fn ms_status_effect_clear<M: Mem>(
        mem: &mut M,
        i: u32,
        mut off: impl FnMut(&mut M, u32),
    ) -> u32 {
        let a = ms_get_chr(mem, i);
        let mut n = 0;
        for k in 0..24 {
            let at = a.wrapping_add(0xe30 + 4 * k);
            let e: u32 = mem.read(at);
            if e != 0 {
                off(mem, e);
                mem.write(at, 0u32);
                n += 1;
            }
        }
        n
    }

    /// `MsResetBindEffect(i)` (`0x00788e60`): ends the bind effect of the
    /// actor's character (`+0x1f` cleared, effect level 0,
    /// `op_et_bindeff_off_signal` from the caller).
    pub fn ms_reset_bind_effect<M: Mem>(mem: &mut M, i: u32, off: impl FnOnce(&mut M, u32)) {
        let a = ms_get_chr(mem, i & 0xff);
        let c = word(mem, a, 0);
        if c != 0 {
            mem.write(a.wrapping_add(0x1f), 0u8);
            ffx_chara::chr::bridge::ch_effect_set_effect_level(mem, c, 0);
            off(mem, c);
        }
    }

    /// `MsResetAfterImage(actor)` (`0x00788e30`): fades the weapon's after
    /// image (`+0xea8`, `op_et_weapon_afterimage_fade` from the caller).
    pub fn ms_reset_after_image<M: Mem>(mem: &mut M, actor: u32, fade: impl FnOnce(&mut M, u32)) {
        let e = word(mem, actor, 0xea8);
        if e != 0 {
            fade(mem, e);
            mem.write(actor.wrapping_add(0xea8), 0u32);
        }
    }

    /// `MsEffectCheckMotion(i)` (`0x00787840`): 0 when the actor's character
    /// waits in a motion (a motion speed at `+0x400`, `Ch_GetWaitMotion`),
    /// marking it in [`EFFECT_WAIT`] with its motion (`+0x3f0`); else -1.
    pub fn ms_effect_check_motion(mem: &mut impl Mem, i: u32) -> i32 {
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        let c = word(mem, a, 0);
        if c != 0
            && half(mem, a, 0x400) != 0
            && ffx_chara::parts::bridge::ch_get_wait_motion(mem, c) != 0
        {
            let m = byte(mem, a, 0x3f0);
            EFFECT_WAIT.set(mem, [i.to_le_bytes()[0], m]);
            return 0;
        }
        -1
    }

    /// `MsCheckStartEffect(chr, set)` (`0x007a92c0`): how many actors of
    /// `set` keep `chr`'s effect waiting: others still busy (`+0x508`) or
    /// any waiting on a motion (`+0x436`); each marks [`EFFECT_WAIT`].
    pub fn ms_check_start_effect(mem: &mut impl Mem, chr: u32, set: u32) -> u32 {
        let mut n = 0;
        for i in 0..0x1f {
            if (set.cast_signed() >> i) & 1 == 0 {
                continue;
            }
            let a = ms_get_chr(mem, i);
            if (chr != i && byte(mem, a, 0x508) != 0) || byte(mem, a, 0x436) != 0 {
                EFFECT_WAIT.set(mem, [chr.to_le_bytes()[0], 0xff]);
                n += 1;
            }
        }
        n
    }

    /// `MsSetHeadTarget(chr, set)` (`0x007b1e80`): an actor with a head
    /// (`+0x437`) looks at the one other actor of `set` (`+0x438`, mode 5 at
    /// `+0x439`), or at none (0xff, mode 5) otherwise; how many others
    /// there are.
    pub fn ms_set_head_target(mem: &mut impl Mem, chr: u32, set: u32) -> u32 {
        if NO_HEAD_TARGET.get(mem) != 0 {
            return 0;
        }
        let a = ms_get_chr(mem, chr);
        if word(mem, a, 0) == 0 || byte(mem, a, 0x437) == 0 {
            return 0;
        }
        let mut n = 0;
        let mut last = chr;
        for i in 0..0x1f {
            if set & (1 << i) != 0 && chr != i {
                last = i;
                n += 1;
            }
        }
        if n == 1 {
            mem.write(a.wrapping_add(0x438), last.to_le_bytes()[0]);
            mem.write(a.wrapping_add(0x439), 5u8);
        } else {
            mem.write(a.wrapping_add(0x438), 0x5ffu16);
        }
        n
    }

    /// `MsSaveAddInfoData(chr, kind)` (`0x00785a00`): counts one more of
    /// battle record `kind` (0-3, `g_PlySave` `+0x50 + 4·kind`, at most
    /// 999999999) for character `chr` (signed, below 18), unless records
    /// are off; the new count, else 0.
    pub fn ms_save_add_info_data(mem: &mut impl Mem, chr: i32, kind: u32) -> i32 {
        if chr >= 0x12 || RECORDS_OFF.get(mem) == 2 || kind > 3 {
            return 0;
        }
        let at = crate::save::bridge::PLY_SAVE
            .ptr(mem)
            .addr
            .wrapping_add(chr.cast_unsigned().wrapping_mul(0x94))
            .wrapping_add(0x50 + 4 * kind);
        let v: i32 = mem.read(at);
        let v = ms_check_range(v.wrapping_add(1), 0, 999_999_999);
        mem.write(at, v);
        v
    }

    /// `MsLimitStatusProcess(_, actor, flags)` (`0x0078d270`): the
    /// overdrive statuses (`+0x640`: bit 0 doubles max HP, bit 1 max MP;
    /// `flags` not negative replaces them): as a bit turns on (or is on
    /// while kept), the maximum (`+0x594`/`+0x598`) becomes twice the base
    /// (`+0x59c`/`+0x5a0`) up to 9999 (99999 with ability 0x200 of
    /// `+0x6be`) or 999 (9999 with 0x400); as it turns off, the base; HP or
    /// MP (`+0x5d0`/`+0x5d4`) kept within. 0.
    pub fn ms_limit_status_process(mem: &mut impl Mem, actor: u32, flags: i32) -> u32 {
        let old = u32::from(byte(mem, actor, 0x640));
        let keep = flags < 0;
        let new = if keep {
            old
        } else {
            mem.write(actor.wrapping_add(0x640), flags.to_le_bytes()[0]);
            flags.cast_unsigned()
        };
        let abil = half(mem, actor, 0x6be);
        for (bit, base, max, cur, cap, more, ab) in [
            (0, 0x59c, 0x594, 0x5d0, 9999, 90000, 0x200u16),
            (1, 0x5a0, 0x598, 0x5d4, 999, 9000, 0x400),
        ] {
            let (o, n) = ((old >> bit) & 1, (new >> bit) & 1);
            let m = if n != 0 && (o == 0 || keep) {
                let b: i32 = mem.read(actor.wrapping_add(base));
                let hi = if abil & ab != 0 { cap + more } else { cap };
                ms_check_range(b.wrapping_mul(2), 0, hi)
            } else if (o != 0 || keep) && n == 0 {
                mem.read(actor.wrapping_add(base))
            } else {
                continue;
            };
            mem.write(actor.wrapping_add(max), m);
            let v: i32 = mem.read(actor.wrapping_add(cur));
            mem.write(actor.wrapping_add(cur), ms_check_range(v, 0, m));
        }
        0
    }

    /// `MsMenuCheckLook()` (`0x0078f9e0`): -1 when the menu always looks
    /// or a party member with Scan (`+0x6bc` bit 0) is in and can act
    /// (`MsAutoCommandStatusCheck(actor, 1, 1, 0)`); else 0.
    pub fn ms_menu_check_look(mem: &impl Mem) -> i32 {
        if MENU_LOOK.get(mem) != 0 {
            return -1;
        }
        for i in 0..0x12 {
            let a = ms_get_chr(mem, i);
            if byte(mem, a, 0x6bc) & 1 != 0
                && byte(mem, a, 0xdcc) == 0
                && crate::actor::bridge::ms_auto_command_status_check(mem, a, 1, 1, 0) != 0
            {
                return -1;
            }
        }
        0
    }

    /// `MsEffectCheckTarget(set)` (`0x00787900`): with several actors in
    /// `set`, only those with an effect target flag (`+0xdda`); else `set`.
    pub fn ms_effect_check_target(mem: &impl Mem, set: u32) -> u32 {
        let (mut n, mut flagged) = (0, 0u32);
        for i in 0..0x1f {
            if (set.cast_signed() >> i) & 1 != 0 {
                n += 1;
                if byte(mem, ms_get_chr(mem, i), 0xdda) != 0 {
                    flagged |= 1 << i;
                }
            }
        }
        if n > 1 { flagged } else { set }
    }

    /// `MsReturnParty()` (`0x007aeec0`): after moving freely
    /// ([`crate::minor::bridge::DIR_FREE`]), the saved party and the 17
    /// bytes after it come back.
    pub fn ms_return_party(mem: &mut impl Mem) {
        let free = crate::minor::bridge::DIR_FREE;
        if free.get(mem) == 0 {
            return;
        }
        free.set(mem, 0);
        let p = PARTY_SAVED.get(mem);
        crate::menu::bridge::PARTY.set(mem, p);
        let x = PARTY_EXTRA_SAVED.get(mem);
        PARTY_EXTRA.set(mem, x);
    }
}
