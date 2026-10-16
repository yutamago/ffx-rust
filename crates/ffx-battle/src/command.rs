//! Battle commands (`docs/rewrite-plan.md`, M2): finding a command's record
//! from its ID, what it costs, whether it can be used, and its CTB effects.
//!
//! A command ID's top nibble picks the kernel table (`MsGetCommand`): 2
//! items, 3 player commands, 4 and 6 the two monster command tables; the
//! low 12 bits are the record (`ffx_kernel::bridge::rom`). A command cue
//! entry (`MsCheckCommandCue`) has the actor at `+0`, the number of
//! commands at `+3`, and per command 16 bytes from `+8`: two command IDs
//! (`u16`; `0xff` none), then at `+8` the targets (bit by actor index).
//!
//! Fields of a command record used here (more in [`crate::damage`]):
//!
//! | Offset | Type | Field |
//! |---|---|---|
//! | `+0x16` | `u8` | `& 0xf8`: 8 and 0x10 do not replace a command found first |
//! | `+0x17` | `u8` | 1 black magic, 2 white magic (`DmgCalc_MagicBooster`) |
//! | `+0x1c` | `u32` | bit 0x20000: not while silenced |
//! | `+0x24` | `u8` | the action's CTB rank (3 when 0) |
//! | `+0x25`, `+0x26` | `u8` | MP cost, overdrive cost |
//! | `+0x39` | `u8` | resets the target's CTB from the user's |
//! | `+0x45`, `+0x46` | `u8` | haste, slow; their durations at `+0x52`, `+0x53` |

/// Command IDs that end `MsGetCommand`'s search (with the caller's own).
pub const STOP_IDS: [u32; 3] = [0x311e, 0x3024, 0x302a];

/// `MsGetCommandMP`'s cost from the command's MP cost and the user's
/// abilities: halved (rounded up) with half MP (`+0x6bc` bit 0x4000), 1
/// with one MP (bit 0x8000; wins), doubled by the magic booster, 0 with
/// `+0x640` bit 2.
#[must_use]
pub fn mp_cost(cost: u8, abilities: u16, booster: bool, free: bool) -> i32 {
    let cost = i32::from(cost);
    let (mut cost, mut div, mut add) = if abilities & 0x4000 != 0 {
        (cost, 2, 1)
    } else {
        (cost, 1, 0)
    };
    if abilities & 0x8000 != 0 && cost != 0 {
        (cost, div, add) = (1, 1, 0);
    }
    let mul = if booster {
        add = 0;
        2
    } else {
        1
    };
    if free { 0 } else { (add + mul * cost) / div }
}

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::frame::Frame;
    use ffx_bridge::{Global, Mem, Ptr};
    use ffx_kernel::bridge::{COMMAND, ITEM, MON_MAGIC1, MON_MAGIC2, ms_check_range, rom};

    use super::{STOP_IDS, mp_cost};
    use crate::actor::bridge::{ms_get_chr, ms_get_ram_chr_monster};
    use crate::damage::bridge::{
        DEBUG_NO_COST, dmg_calc_magic_booster, ms_calc_damage_command, ms_get_next_ctb,
    };

    /// Debug: commands cost no MP (`MsCheckUseCommand`); debug native
    /// case 4.
    pub const DEBUG_FREE_MP: Global<u8> = Global::new(0x0112_a901);
    /// Where [`ms_calc_magic`] keeps the command ID it looks up.
    pub const MAGIC_FRAME: u32 = 0xffc0_0000;
    /// Where [`ms_calc_cost_ctb`] keeps its one-command list.
    pub const COST_FRAME: u32 = 0xffb0_0000;

    fn u8_at(mem: &impl Mem, p: u32, o: u32) -> u8 {
        Ptr::<u8>::new(p).field::<u8>(o).get(mem)
    }

    fn u16_at(mem: &impl Mem, p: u32, o: u32) -> u16 {
        Ptr::<u8>::new(p).field::<u16>(o).get(mem)
    }

    fn u32_at(mem: &impl Mem, p: u32, o: u32) -> u32 {
        Ptr::<u8>::new(p).field::<u32>(o).get(mem)
    }

    /// The record of command `id` in the table its top nibble names, null
    /// for others.
    pub fn record(mem: &mut impl Mem, id: u32) -> u32 {
        let table = match id.cast_signed() >> 12 {
            2 => ITEM,
            3 => COMMAND,
            4 => MON_MAGIC1,
            6 => MON_MAGIC2,
            _ => return 0,
        };
        rom(mem, table, id & 0xfff, 0)
    }

    /// `MsGetCommand(chr, _, stop, list, &id)` (`0x0078ce50`): the record
    /// of the command in `list` (two `u16` IDs), or of `*id` (`0xff` when
    /// `id` is null too) without a list; the second ID replaces the first
    /// unless its record's `+0x16` & 0xf8 is 8 or 0x10. The search stops at
    /// [`STOP_IDS`] and `stop`. The ID found goes to `id` (0 when none).
    pub fn ms_get_command(mem: &mut impl Mem, stop: u32, list: u32, idp: u32) -> u32 {
        let n = if list == 0 { 1 } else { 2 };
        let (mut found, mut found_id) = (0, 0);
        for k in 0..n {
            let id = if list != 0 {
                u32::from(mem.read::<u16>(list + 2 * k))
            } else if idp != 0 {
                mem.read(idp)
            } else {
                0xff
            };
            let r = record(mem, id);
            if r != 0 && !(found != 0 && matches!(u8_at(mem, r, 0x16) & 0xf8, 8 | 0x10)) {
                found = r;
                found_id = id;
            }
            if STOP_IDS.contains(&id) || id == stop {
                break;
            }
        }
        if idp != 0 {
            mem.write(idp, if found != 0 { found_id } else { 0 });
        }
        found
    }

    /// `MsGetCommandMP(chr, command)` (`0x0078cf70`): [`mp_cost`]; 0 for a
    /// null command or with `+0x6ce` set (no cost).
    pub fn ms_get_command_mp(mem: &mut impl Mem, chr: u32, command: u32) -> i32 {
        if command == 0 {
            return 0;
        }
        let a = ms_get_chr(mem, chr);
        let cost = if u8_at(mem, a, 0x6ce) == 0 {
            u8_at(mem, command, 0x25)
        } else {
            0
        };
        let abilities = u16_at(mem, a, 0x6bc);
        let booster = dmg_calc_magic_booster(mem, a, command, 0, 0) != 0;
        mp_cost(cost, abilities, booster, u8_at(mem, a, 0x640) & 4 != 0)
    }

    /// `MsCheckUseCommand(chr, command, extra)` (`0x0078c690`): the MP the
    /// command costs, or -1 when it cannot be used: no command, not enough
    /// overdrive (`+0x5bc` below `+0x5bd`, or status 2 bit 0x400) for one
    /// that needs it, silenced (counter `+0x609`) for magic, or less MP than
    /// the cost plus `extra` (monsters and [`DEBUG_FREE_MP`] pay nothing).
    pub fn ms_check_use_command(mem: &mut impl Mem, chr: u32, command: u32, extra: i32) -> i32 {
        if command == 0 {
            return -1;
        }
        let a = ms_get_chr(mem, chr);
        if u8_at(mem, command, 0x26) != 0
            && !((u8_at(mem, a, 0x5bc) >= u8_at(mem, a, 0x5bd)
                && u16_at(mem, a, 0x616) & 0x400 == 0)
                || DEBUG_NO_COST.get(mem) != 0)
        {
            return -1;
        }
        if u32_at(mem, command, 0x1c) & 0x2_0000 != 0 && u8_at(mem, a, 0x609) != 0 {
            return -1;
        }
        let mut mp = ms_get_command_mp(mem, chr, command);
        let mut extra = extra;
        if ms_get_ram_chr_monster(chr) == 1 || DEBUG_FREE_MP.get(mem) != 0 {
            mp = 0;
            extra = 0;
        }
        if u32_at(mem, a, 0x5d4).cast_signed() >= extra.wrapping_add(mp) {
            mp
        } else {
            -1
        }
    }

    /// `MsCalcMagic(cue, k)` (`0x0078ab20`): whether command `k` of a cue
    /// entry can be paid for (-1) or not (0); stores its MP and overdrive
    /// cost at `+0x6cc`, `+0x6cd`, and at `+0x5c0` whether it uses overdrive.
    /// An actor with `+0x6de` set uses command 0x3028 (free with cue `+2`).
    pub fn ms_calc_magic(mem: &mut impl Mem, cue: u32, k: u32) -> i32 {
        if cue == 0 {
            return 0;
        }
        let chr = u32::from(u8_at(mem, cue, 0));
        let a = ms_get_chr(mem, chr);
        let mut f = Frame::new(mem, MAGIC_FRAME, 4);
        let idp = f.at(0);
        let (r, mut mp) = if u8_at(&f, a, 0x6de) != 0 {
            f.write(idp, 0x3028u32);
            let r = ms_get_command(&mut f, u32::MAX, 0, idp);
            (r, ms_check_use_command(&mut f, chr, r, 0))
        } else {
            let list = cue.wrapping_add(8).wrapping_add(k.wrapping_mul(16));
            let r = ms_get_command(&mut f, 0x305f, list, idp);
            (r, ms_check_use_command(&mut f, chr, r, 0))
        };
        let mem = f.inner();
        if u8_at(mem, a, 0x6de) != 0 && u8_at(mem, cue, 2) != 0 {
            mp = 0;
        }
        let od = if DEBUG_NO_COST.get(mem) == 0 && r != 0 {
            i32::from(u8_at(mem, r, 0x26))
        } else {
            0
        };
        let uses = r != 0 && u8_at(mem, r, 0x26) != 0;
        mem.write(a.wrapping_add(0x5c0), u8::from(uses));
        if u32_at(mem, a, 0x5d4).cast_signed() >= mp
            && mp >= 0
            && i32::from(u8_at(mem, a, 0x5bc)) >= od
        {
            mem.write(a.wrapping_add(0x6cc), mp.to_le_bytes()[0]);
            mem.write(a.wrapping_add(0x6cd), od.to_le_bytes()[0]);
            return -1;
        }
        0
    }

    /// `MsCalcCostCTB(cue)` (`0x007895b0`): the CTB rank (`+0x24`, 3 when
    /// 0) of the last command of a cue entry found (of its two), stopping at
    /// 0x3029, 0x305f and 0x311e; 3 for none.
    pub fn ms_calc_cost_ctb(mem: &mut impl Mem, cue: u32) -> u32 {
        if cue == 0 {
            return 3;
        }
        let chr = u32::from(u8_at(mem, cue, 0));
        let a = ms_get_chr(mem, chr);
        let mut f = Frame::new(mem, COST_FRAME, 4);
        let list = f.at(0);
        let mut found = 0;
        for k in 0..2 {
            let mut id = u16_at(&f, cue, 8 + 2 * k);
            if id == 0xff {
                continue;
            }
            if u8_at(&f, a, 0x6de) != 0 {
                id = 0x3028;
            }
            f.write(list, [id, 0xff]);
            let r = ms_get_command(&mut f, u32::MAX, list, 0);
            if r != 0 {
                found = r;
            }
            if matches!(id, 0x3029 | 0x305f | 0x311e) {
                break;
            }
        }
        let mem = f.inner();
        if found == 0 {
            return 3;
        }
        match u8_at(mem, found, 0x24) {
            0 => 3,
            rank => u32::from(rank),
        }
    }

    /// `MsCalcDamageCTB(cue, ctb, rank, haste, slow)` (`0x007898d0`): the
    /// CTB effects of a cue entry's commands on their targets (`ctb`, 31
    /// `i32`, skipping negative ones; `haste` and `slow`, 31 bytes each):
    /// delay (1.5 or 3 times the target's base CTB), a CTB taken from the
    /// user's next turn (`+0x39`), CTB damage, and haste and slow, which
    /// replace each other or wear off (removing commands), each keeping the
    /// CTB as it was when it fails. Commands with the weapon's stats use the
    /// user's weapon haste and slow too.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ms_calc_damage_ctb(
        mem: &mut impl Mem,
        cue: u32,
        ctb: u32,
        rank: i32,
        haste: u32,
        slow: u32,
    ) {
        let chr = u32::from(u8_at(mem, cue, 0));
        let user = ms_get_chr(mem, chr);
        let mut k = 0;
        while k < u32::from(u8_at(mem, cue, 3)) {
            let entry = cue.wrapping_add(16 * k);
            k += 1;
            let c = ms_get_command(mem, u32::MAX, entry.wrapping_add(8), 0);
            if c == 0 {
                continue;
            }
            let weapon = u32_at(mem, c, 0x1c) & 0x40000 != 0;
            let (formula, power) = if weapon {
                (u8_at(mem, user, 0x5c1), u8_at(mem, user, 0x5c7))
            } else {
                (u8_at(mem, c, 0x28), u8_at(mem, c, 0x2a))
            };
            let mask = u32_at(mem, entry, 0x10).cast_signed();
            for i in 0..0x1f_u32 {
                let at = ctb.wrapping_add(4 * i);
                if (mask >> i) & 1 == 0 || mem.read::<i32>(at) < 0 {
                    continue;
                }
                let t = ms_get_chr(mem, i);
                let hp = u32_at(mem, t, 0x5d0);
                mem.write(t.wrapping_add(0x6e4), hp);
                let mp = u32_at(mem, t, 0x5d4);
                mem.write(t.wrapping_add(0x6e8), mp);
                let base = u32::from(u8_at(mem, t, 0x65c));
                mem.write(t.wrapping_add(0x6ec), base);
                let amount =
                    |mem: &mut _, on: u32, weapon_on: u32, dur: u32, weapon_dur: u32| -> u8 {
                        if u8_at(mem, c, on) != 0 || (u8_at(mem, user, weapon_on) != 0 && weapon) {
                            let d = u8_at(mem, c, dur);
                            let w = u8_at(mem, user, weapon_dur);
                            if w > d && weapon { w } else { d }
                        } else {
                            0
                        }
                    };
                let h = amount(mem, 0x45, 0x5f5, 0x52, 0x602);
                let s = amount(mem, 0x46, 0x5f6, 0x53, 0x603);
                let f = u32_at(mem, c, 0x1c);
                let delay = if f & 0x4000 != 0 {
                    2
                } else {
                    i32::from(f & 0x2000 != 0)
                };
                if delay != 0 {
                    let b = ffx_kernel::bridge::ms_get_rom_ctb_count(
                        mem,
                        i32::from(u8_at(mem, t, 0x5ac)),
                    );
                    let v = mem
                        .read::<i32>(at)
                        .wrapping_add(i32::from(b) * 3 * delay / 2);
                    mem.write(at, v);
                }
                let (hp_at, sp_at) = (haste.wrapping_add(i), slow.wrapping_add(i));
                if u8_at(mem, c, 0x39) != 0 || (u8_at(mem, user, 0x5e9) != 0 && weapon) {
                    let n = ms_get_next_ctb(
                        mem,
                        user,
                        rank,
                        i32::from(u8_at(mem, user, 0x613)),
                        i32::from(u8_at(mem, user, 0x614)),
                    );
                    mem.write(at, n.wrapping_add(i32::from(u8_at(mem, user, 0x65c))));
                    mem.write(hp_at, 0u8);
                    mem.write(sp_at, 0u8);
                }
                let saved: i32 = mem.read(at);
                if u8_at(mem, c, 0x23) & 4 != 0 {
                    let status = u32::from(u16_at(mem, t, 0x606));
                    let d = ms_calc_damage_command(
                        mem,
                        user,
                        t,
                        c,
                        i32::from(formula),
                        i32::from(power),
                        status,
                        4,
                        0,
                        0,
                        0,
                        0,
                    );
                    let v = mem.read::<i32>(at).wrapping_add(d);
                    mem.write(at, v.max(0));
                }
                let removing = u8_at(mem, c, 0x20) & 0x20 != 0;
                for (amount, mine, other) in [(h, hp_at, sp_at), (s, sp_at, hp_at)] {
                    if amount == 0 {
                        continue;
                    }
                    let cur: u8 = mem.read(mine);
                    if removing {
                        if cur < 0xff {
                            let v = ms_check_range(i32::from(cur) - i32::from(amount), 0, 0xff);
                            mem.write(mine, v.to_le_bytes()[0]);
                        } else {
                            mem.write(at, saved);
                        }
                    } else if cur == 0 {
                        mem.write(mine, amount);
                        mem.write(other, 0u8);
                    } else {
                        mem.write(at, saved);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mp_costs() {
        assert_eq!(mp_cost(9, 0, false, false), 9);
        assert_eq!(mp_cost(9, 0x4000, false, false), 5);
        assert_eq!(mp_cost(9, 0xc000, false, false), 1);
        assert_eq!(mp_cost(9, 0x4000, true, false), 9);
        assert_eq!(mp_cost(9, 0, true, true), 0);
    }
}
