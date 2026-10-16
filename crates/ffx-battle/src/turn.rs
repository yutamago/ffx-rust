//! The battle's turn bookkeeping (`docs/rewrite-plan.md`, M2): the first
//! CTB order, items used by a command, HP and MP kept in range, what a
//! command's results do to an actor's status, AP and the item spoils.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // actor, index, value
pub mod bridge {
    use ffx_bridge::{Global, Mem};
    use ffx_kernel::bridge::ms_check_range;

    use crate::actor::bridge::{ms_btl_chr_num_check, ms_get_chr, ms_get_mon};

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn half(mem: &impl Mem, p: u32, o: u32) -> u16 {
        mem.read(p.wrapping_add(o))
    }

    /// `MsAdjustCTB` on an actor's counter: the delay with haste (`+0x613`)
    /// and slow (`+0x614`), 0 under bit 1 of `+0x6bc`; kept in `+0x65c`.
    fn first_ctb(mem: &mut impl Mem, a: u32, delay: i32) -> u8 {
        let delay = if byte(mem, a, 0x6bc) & 2 != 0 {
            0
        } else {
            delay
        };
        let r = crate::damage::bridge::ms_adjust_ctb(
            delay,
            i32::from(byte(mem, a, 0x613)),
            i32::from(byte(mem, a, 0x614)),
        );
        let r = r.to_le_bytes()[0];
        mem.write(a.wrapping_add(0x65c), r);
        r
    }

    /// `MsSetFirstCTB(mode)` (`0x0078ded0`): every actor's CTB base
    /// (`+0x65d`) is three times its agility's count; then the first
    /// counters: the party's 0 (mode 1), the base (mode 2) or the base less
    /// a random part of its range; the monsters' the base (mode 1), 0 (mode
    /// 2) or `base·100 / (100 − r % 11)` (not below 0). Every actor that
    /// can act moves up by the lowest counter, which is returned (-1 when
    /// no one can act).
    pub fn ms_set_first_ctb(mem: &mut impl Mem, mode: i32) -> i32 {
        use crate::damage::bridge::ms_get_rnd_chr;
        use crate::rng::bridge::brnd;
        use ffx_kernel::bridge::{ms_get_rom_ctb_base, ms_get_rom_ctb_count};
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            let n = ms_get_rom_ctb_count(mem, i32::from(byte(mem, a, 0x5ac)));
            mem.write(a.wrapping_add(0x65d), n.wrapping_mul(3));
        }
        let mut lowest = -1i32;
        for i in 0..0x12 {
            let a = ms_get_chr(mem, i);
            let base = i32::from(byte(mem, a, 0x65d));
            let d = match mode {
                1 => 0,
                2 => base,
                _ => {
                    let rom = ms_get_rom_ctb_base(mem, i32::from(byte(mem, a, 0x5ac)));
                    let g = ms_get_rnd_chr(mem, i, 0);
                    let r = brnd(mem, g).cast_signed();
                    base - r.wrapping_rem(i32::from(byte(mem, rom, 1)) + 1)
                }
            };
            let r = first_ctb(mem, a, d);
            if byte(mem, a, 0xdc8) != 0 && (lowest < 0 || i32::from(r) < lowest) {
                lowest = i32::from(r);
            }
        }
        for i in 0..8 {
            let a = ms_get_mon(mem, i);
            let base = i32::from(byte(mem, a, 0x65d));
            let d = match mode {
                1 => base,
                2 => 0,
                _ => {
                    let g = ms_get_rnd_chr(mem, i + 0x14, 0);
                    let r = brnd(mem, g).cast_signed();
                    let q = (base * 100) / (100 - r.wrapping_rem(11));
                    q.max(0)
                }
            };
            let r = first_ctb(mem, a, d);
            if byte(mem, a, 0xdc8) != 0 && (lowest < 0 || i32::from(r) < lowest) {
                lowest = i32::from(r);
            }
        }
        let step = lowest.to_le_bytes()[0];
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            if byte(mem, a, 0xdc8) != 0 {
                let v = byte(mem, a, 0x65c).wrapping_sub(step);
                mem.write(a.wrapping_add(0x65c), v);
            }
        }
        lowest
    }

    /// `MsUseItem(chr, command, n)` (`0x007b0c20`): for each of the
    /// command's entries (`+3` of them, two IDs each from `+8`, 0x10 apart)
    /// up to Pray (0x311e), a party member's item (0x2xxx) is used `n` times
    /// (`MsSaveItemUse`); the sum of what that returns.
    pub fn ms_use_item(mem: &mut impl Mem, chr: u32, command: u32, n: i32) -> i32 {
        let mut sum = 0i32;
        let mut k = 0u32;
        while k < u32::from(byte(mem, command, 3)) {
            let list = command.wrapping_add(8 + 0x10 * k);
            let (_, id) = crate::command::bridge::get_command(mem, u32::MAX, list, 0xff);
            if id == 0x311e {
                break;
            }
            if id & 0xffff_f000 == 0x2000 && crate::actor::bridge::ms_get_ram_chr_monster(chr) == 0
            {
                sum = sum.wrapping_add(crate::save::bridge::ms_save_item_use(mem, id, n));
            }
            k += 1;
        }
        sum
    }

    /// `MsCheckStatus()` (`0x0079a0e0`): every present actor's (`+0x10`) HP
    /// (`+0x5d0`) and MP (`+0x5d4`) kept within 0 and their maxima
    /// (`+0x594`, `+0x598`).
    pub fn ms_check_status(mem: &mut impl Mem) {
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            if byte(mem, a, 0x10) == 0 {
                continue;
            }
            for (v, max) in [(0x5d0, 0x594), (0x5d4, 0x598)] {
                let x: i32 = mem.read(a.wrapping_add(v));
                let m: i32 = mem.read(a.wrapping_add(max));
                mem.write(a.wrapping_add(v), ms_check_range(x, 0, m));
            }
        }
    }

    /// What `MsGetDamageStatus` reports: status bits (`+0x606`, `+0x616`)
    /// and status counters (13 from `+0x608`) the results add and remove.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub struct StatusChange {
        pub added: u16,
        pub removed: u16,
        pub added2: u16,
        pub removed2: u16,
        pub counters_on: u32,
        pub counters_off: u32,
        /// The target ends up dead: HP less the damage at most 0, or the
        /// last status has bit 0.
        pub dead: bool,
    }

    /// `MsGetDamageStatus(chr, target, &added, &on, &added2, &removed,
    /// &off, &removed2)` (`0x0079aa50`): over `target`'s damage buffers for
    /// `chr`'s (low byte) running command, each result left against the
    /// status before it (bits at result `+0x2c` and `+0x2e`) and against
    /// the target's current counters (result bytes `+0x1f`), summing the
    /// damage (`+0x38`).
    pub fn damage_status(mem: &mut impl Mem, chr: u32, target: u32) -> StatusChange {
        let mut s = StatusChange::default();
        let c = chr & 0xff;
        if ms_btl_chr_num_check(c) == 0 {
            return s;
        }
        let id = byte(mem, crate::actor::bridge::ms_get_chr_command(mem, c), 2);
        let t = ms_get_chr(mem, target);
        let (mut s1, mut s2) = (half(mem, t, 0x606), half(mem, t, 0x616));
        let mut damage = 0i32;
        for o in [0x774u32, 0xa4c] {
            let b = t.wrapping_add(o);
            if u32::from(byte(mem, b, 2)) != c || byte(mem, b, 3) != id {
                continue;
            }
            for k in u32::from(byte(mem, b, 0))..u32::from(byte(mem, b, 1)) {
                let e = b.wrapping_add(k * 0x2c);
                damage = damage.wrapping_add(mem.read(e.wrapping_add(0x38)));
                let (n1, n2) = (half(mem, e, 0x2c), half(mem, e, 0x2e));
                for bit in 0..16 {
                    let m = 1u16 << bit;
                    if (s1 ^ n1) & m != 0 {
                        if n1 & m != 0 {
                            s.added |= m;
                        } else {
                            s.removed |= m;
                        }
                    }
                    if (s2 ^ n2) & m != 0 {
                        if n2 & m != 0 {
                            s.added2 |= m;
                        } else {
                            s.removed2 |= m;
                        }
                    }
                }
                for j in 0..13 {
                    let new = byte(mem, e, 0x1f + j);
                    if byte(mem, t, 0x608 + j) != new {
                        if new != 0 {
                            s.counters_on |= 1 << j;
                        } else {
                            s.counters_off |= 1 << j;
                        }
                    }
                }
                (s1, s2) = (n1, n2);
            }
        }
        let hp: i32 = mem.read(t.wrapping_add(0x5d0));
        s.dead = hp.wrapping_sub(damage) <= 0 || s1 & 1 != 0;
        s
    }

    /// `MsGetDamageStatus` (`0x0079aa50`) with its six out pointers (null
    /// ones skipped): [`damage_status`]; 1 when the target ends up dead.
    pub fn ms_get_damage_status(mem: &mut impl Mem, chr: u32, target: u32, out: [u32; 6]) -> u32 {
        let s = damage_status(mem, chr, target);
        let v = [
            u32::from(s.added),
            s.counters_on,
            u32::from(s.added2),
            u32::from(s.removed),
            s.counters_off,
            u32::from(s.removed2),
        ];
        for (p, v) in out.into_iter().zip(v) {
            if p != 0 {
                mem.write(p, v);
            }
        }
        u32::from(s.dead)
    }

    /// The AP each of 18 characters has gained this battle.
    pub const AP: Global<[u32; 18]> = Global::new(0x0231_0f20);

    /// `MsApUp(chr, actor, ap, ret)` (`0x00798a00`): adds `ap` to
    /// character `chr`'s battle AP (none with ability bit 0x40 of `+0x6be`;
    /// ×3 with 0x20, else ×2 with 0x10, for an actor that can act and is
    /// in), at most 999999999. Nothing for an actor out (`+0xdcd`). Returns
    /// `ret`, or 2 with bit 0x4000 for an actor that can act and is in.
    pub fn ms_ap_up(mem: &mut impl Mem, chr: u32, actor: u32, ap: u32, ret: u32) -> u32 {
        if byte(mem, actor, 0xdcd) != 0 {
            return ret;
        }
        let f = half(mem, actor, 0x6be);
        let mut ap = if f & 0x40 != 0 { 0 } else { ap };
        let mut ret = ret;
        if byte(mem, actor, 0xdc8) != 0 && byte(mem, actor, 0xdcc) == 0 {
            if f & 0x4000 != 0 {
                ret = 2;
            }
            if f & 0x20 != 0 {
                ap = ap.wrapping_mul(3);
            } else if f & 0x10 != 0 {
                ap = ap.wrapping_mul(2);
            }
        }
        let at = AP.ptr(mem).addr.wrapping_add(chr.wrapping_mul(4));
        let v: u32 = mem.read(at);
        mem.write(
            at,
            ms_check_range(v.wrapping_add(ap).cast_signed(), 0, 999_999_999),
        );
        ret
    }

    /// `BattleRewards_AddItem(item, n, spoils)` (`0x00798ac0`): `n` (above
    /// 0) of `item` into the spoils (up to 8 kinds: `i8` count at `+0xd0`,
    /// `u16` IDs at `+0xd4`, amounts at `+0xe4`, at most 99).
    pub fn battle_rewards_add_item(mem: &mut impl Mem, item: u32, n: i32, spoils: u32) {
        if item == 0 || n <= 0 {
            return;
        }
        let kinds = i32::from(byte(mem, spoils, 0xd0).cast_signed());
        for k in 0..kinds.max(0) {
            let k = k.cast_unsigned();
            if u32::from(half(mem, spoils, 0xd4 + 2 * k)) == item {
                let v = i32::from(byte(mem, spoils, 0xe4 + k)).wrapping_add(n);
                mem.write(spoils + 0xe4 + k, ms_check_range(v, 0, 99).to_le_bytes()[0]);
                return;
            }
        }
        if kinds < 8 {
            let k = kinds.cast_unsigned();
            #[allow(clippy::cast_possible_truncation)] // a u16 store
            mem.write(
                spoils.wrapping_add(0xd4).wrapping_add(k.wrapping_mul(2)),
                item as u16,
            );
            let v = ms_check_range(n, 0, 99).to_le_bytes()[0];
            mem.write(spoils.wrapping_add(0xe4).wrapping_add(k), v);
            let c = byte(mem, spoils, 0xd0).wrapping_add(1);
            mem.write(spoils + 0xd0, c);
        }
    }
    /// Set: overdrive gauges do not fill (`MsLimitUp`).
    pub const LIMIT_FROZEN: Global<u8> = Global::new(0x0112_a916);
    /// 0.9 as the `f64` `MsLimitUp` scales the overdrive-to-AP rate by.
    pub const AP_RATE_DECAY: Global<f64> = Global::new(0x00b4_9958);

    /// `MsLimitUp(chr, actor, n)` (`0x007b1590`): fills the overdrive gauge
    /// (`+0x5bc`, at most `+0x5bd`) by `n`, ×3 with ability bit 2 of
    /// `+0x6be`, else ×2 with bit 1 or (bit 4) when weak; ×1.5 and ×2 with
    /// bits 0x20 and 0x40 of `+0x640`; none under status 2 bit 0x40, ×2 with
    /// bit 0x80. With ability bit 8 (Overdrive → AP) the fill becomes AP
    /// instead (`MsGetNextAP · n / max · rate`, `+0x70c` then ×0.9, both
    /// `f32`) and is 0. A gauge just filled counts a record
    /// (`MsSaveAddInfoData(chr, 3)`). Returns the fill; `n` unchanged when
    /// frozen, under status 2 bit 0x400 or out.
    pub fn ms_limit_up(mem: &mut impl Mem, chr: u32, actor: u32, n: i32) -> i32 {
        use ffx_bridge::x87::{load, mul, store};
        if LIMIT_FROZEN.get(mem) != 0
            || half(mem, actor, 0x616) & 0x400 != 0
            || byte(mem, actor, 0xdcc) != 0
            || byte(mem, actor, 0xdce) != 0
        {
            return n;
        }
        let abil = half(mem, actor, 0x6be);
        let mut v = if abil & 2 != 0 {
            n.wrapping_mul(3)
        } else if abil & 1 != 0
            || abil & 4 != 0 && {
                let hp: i32 = mem.read(actor.wrapping_add(0x5d0));
                let max: i32 = mem.read(actor.wrapping_add(0x594));
                crate::actor::bridge::ms_calc_weak_level(hp, max).cast_signed() >= 1
            }
        {
            n.wrapping_mul(2)
        } else {
            n
        };
        let st = byte(mem, actor, 0x640);
        if st & 0x20 != 0 {
            v = v.wrapping_mul(3) / 2;
        }
        if st & 0x40 != 0 {
            v = v.wrapping_mul(2);
        }
        let s2 = half(mem, actor, 0x616);
        if s2 & 0x40 != 0 {
            v = 0;
        }
        if s2 & 0x80 != 0 {
            v = v.wrapping_mul(2);
        }
        if byte(mem, actor, 0x6be) & 8 != 0 {
            let next = crate::menu::bridge::ms_get_next_ap(mem, chr.cast_signed());
            let q = next.wrapping_mul(v) / i32::from(byte(mem, actor, 0x5bd));
            #[allow(clippy::cast_precision_loss)] // fild then fstp dword
            let f = store(f64::from(q as f32));
            let rate: u32 = mem.read(actor.wrapping_add(0x70c));
            let f = store(mul(load(f), load(rate)));
            let ap = ffx_bridge::x87::ftol(mem, load(f));
            ms_ap_up(mem, chr, actor, ap, 0);
            let rate: u32 = mem.read(actor.wrapping_add(0x70c));
            mem.write(
                actor.wrapping_add(0x70c),
                store(mul(load(rate), AP_RATE_DECAY.get(mem))),
            );
            v = 0;
        }
        let old = i32::from(byte(mem, actor, 0x5bc));
        let max = byte(mem, actor, 0x5bd);
        let g = ms_check_range(old.wrapping_add(v), 0, i32::from(max));
        mem.write(actor.wrapping_add(0x5bc), g.to_le_bytes()[0]);
        if old < g && g == i32::from(max) {
            crate::flow::bridge::ms_save_add_info_data(mem, chr.cast_signed(), 3);
        }
        v
    }
}
