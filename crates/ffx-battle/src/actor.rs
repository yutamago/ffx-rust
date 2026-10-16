//! Battle actors (`chr`) and their status (`docs/rewrite-plan.md`, M2).
//!
//! The battle code addresses everyone by an index:
//! - `0x00`-`0x1e`: actors of 0xf90 bytes from `*0x011334d4`
//!   (`MsGetChrTop`); `0x14`-`0x1b` among them are the monsters
//!   (`MsGetRamChrMonster`);
//! - `0x1f`-`0x5c`: records of 0x390 bytes from `*0x0113446c`, two per
//!   actor (`MsGetWeapon`);
//! - anything else gets a spare record at `0x011334d8`.
//!
//! Status fields of an actor that the functions here use:
//!
//! | Offset | Type | Field |
//! |---|---|---|
//! | `+0x3b` | `u8` | auto-life forever (`MsSetEternalRelife`) |
//! | `+0x3f0` | `u8` | motion; `0x28`/`0x29` let a new one start (`MsCheckSetMotion`) |
//! | `+0x415`, `+0x417` | `u8`, `u16` | motion speed (`MsMotionSpeedReset`) |
//! | `+0x404`, `+0x432`, `+0x43e` | `u16`, `u16`, `u8` | motion state (`MsTerminateMotion`) |
//! | `+0x49c` | `f32` | end direction (`msSetEndDir`) |
//! | `+0x4fc`-`+0x4fe` | 3 × `u8` | battle position, saved at `+0x509` (`MsPushBtlPos`) |
//! | `+0x508` | `u8` | busy (`MsCheckSetMotion`) |
//! | `+0x511`, `+0x514` | `i8`, `f32` × n | direction index and directions (`msSetEndDir`) |
//! | `+0x606` | `u16` | status bits; `0x300` (KO, petrify), `0x400`, `0x800` block auto commands |
//! | `+0x608` | 13 × `u8` | status counters, `0xff` forever |
//! | `+0x616` | `u16` | status bits 2; `0x100` blocks auto commands, `0x200` auto-life |
//! | `+0x618`, `+0x61a`, `+0x628` | | the three saved while an auto-status overrides them |
//! | `+0x62a` | 3 × `u16` | auto-statuses in force (bits of the three above) |
//! | `+0x630`, `+0x636` | 3 × `u16` each | auto-statuses always, and in mode 1 and 2 |
//! | `+0x63e`, `+0x63f` | `u8` | auto-status mode, the mode last applied |
//! | `+0x6d2` | `u8` | cleared when the counter at `+0x612` starts (`MsSetAutoStatus`) |
//! | `+0xdc8`, `+0xdce` | `u8` | auto commands: allowed, blocked |
//! | `+0xdfb` | `u8` | motion phase; 2 becomes 3 (`MsTerminateMotion`) |

/// Actors of 0xf90 bytes.
pub const ACTORS: u32 = 0x1f;
/// Bytes per actor.
pub const ACTOR_SIZE: u32 = 0xf90;
/// Bytes per record from index `0x1f` on.
pub const WEAPON_SIZE: u32 = 0x390;
/// Indices below this are actors or records.
pub const INDICES: u32 = 0x5d;
/// The first monster actor.
pub const FIRST_MONSTER: u32 = 0x14;
/// Monster actors.
pub const MONSTERS: u32 = 8;
/// Status counters (`+0x608`).
pub const COUNTERS: usize = 13;

/// What index `i` addresses (`MsGetChr`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chr {
    /// Actor `n` of the first table.
    Actor(u32),
    /// Record `n` of the second table.
    Weapon(u32),
    /// The spare record.
    Spare,
}

/// What `MsGetChr(i)` addresses (only the low byte of `i` counts).
#[must_use]
pub const fn chr(i: u32) -> Chr {
    let i = i & 0xff;
    if i < ACTORS {
        Chr::Actor(i)
    } else if i < INDICES {
        Chr::Weapon(i - ACTORS)
    } else {
        Chr::Spare
    }
}

/// `MsCalcWeakLevel(hp, max)` (`0x0078bf00`): 3 when `hp` is 0 or less,
/// else by `4 * hp / max` (signed, 0 when `max` is 0): 2 below 1, 1 below
/// 2, 0 from 2 on. (The original faults on `i32::MIN / -1`; this wraps.)
#[must_use]
pub const fn weak_level(hp: i32, max: i32) -> u32 {
    let q = if max != 0 {
        hp.wrapping_mul(4).wrapping_div(max)
    } else {
        0
    };
    if hp < 1 {
        3
    } else if q < 1 {
        2
    } else if q < 2 {
        1
    } else {
        0
    }
}

/// The status fields at actor `+0x606..+0x640` that auto-statuses move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AutoStatus {
    /// `+0x606`.
    pub status: u16,
    /// `+0x608`.
    pub counters: [u8; COUNTERS],
    /// `+0x615`, kept as it is.
    pub pad615: u8,
    /// `+0x616`.
    pub status2: u16,
    /// `+0x618`.
    pub saved_status: u16,
    /// `+0x61a`.
    pub saved_counters: [u8; COUNTERS],
    /// `+0x627`, kept as it is.
    pub pad627: u8,
    /// `+0x628`.
    pub saved_status2: u16,
    /// `+0x62a`: in force, for `status`, `counters` and `status2`.
    pub auto: [u16; 3],
    /// `+0x630`: always.
    pub auto_base: [u16; 3],
    /// `+0x636`: added in modes 1 and 2.
    pub auto_extra: [u16; 3],
    /// `+0x63c`, kept as it is.
    pub pad63c: [u8; 2],
    /// `+0x63e`.
    pub mode: u8,
    /// `+0x63f`.
    pub mode_applied: u8,
}

/// First offset of [`AutoStatus`] in an actor.
pub const AUTO_STATUS_AT: u32 = 0x606;
/// Its size.
pub const AUTO_STATUS_SIZE: usize = 0x3a;

fn u16_at(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}

fn put_u16(b: &mut [u8], o: usize, v: u16) {
    b[o..o + 2].copy_from_slice(&v.to_le_bytes());
}

impl AutoStatus {
    /// From the actor's bytes `+0x606..+0x640`.
    #[must_use]
    pub fn from_bytes(b: &[u8; AUTO_STATUS_SIZE]) -> Self {
        let words = |o: usize| [u16_at(b, o), u16_at(b, o + 2), u16_at(b, o + 4)];
        let mut s = Self {
            status: u16_at(b, 0),
            pad615: b[0x0f],
            status2: u16_at(b, 0x10),
            saved_status: u16_at(b, 0x12),
            pad627: b[0x21],
            saved_status2: u16_at(b, 0x22),
            auto: words(0x24),
            auto_base: words(0x2a),
            auto_extra: words(0x30),
            pad63c: [b[0x36], b[0x37]],
            mode: b[0x38],
            mode_applied: b[0x39],
            ..Self::default()
        };
        s.counters.copy_from_slice(&b[2..2 + COUNTERS]);
        s.saved_counters.copy_from_slice(&b[0x14..0x14 + COUNTERS]);
        s
    }

    /// Back to bytes.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; AUTO_STATUS_SIZE] {
        let mut b = [0u8; AUTO_STATUS_SIZE];
        put_u16(&mut b, 0, self.status);
        b[2..2 + COUNTERS].copy_from_slice(&self.counters);
        b[0x0f] = self.pad615;
        put_u16(&mut b, 0x10, self.status2);
        put_u16(&mut b, 0x12, self.saved_status);
        b[0x14..0x14 + COUNTERS].copy_from_slice(&self.saved_counters);
        b[0x21] = self.pad627;
        put_u16(&mut b, 0x22, self.saved_status2);
        for i in 0..3 {
            put_u16(&mut b, 0x24 + 2 * i, self.auto[i]);
            put_u16(&mut b, 0x2a + 2 * i, self.auto_base[i]);
            put_u16(&mut b, 0x30 + 2 * i, self.auto_extra[i]);
        }
        b[0x36] = self.pad63c[0];
        b[0x37] = self.pad63c[1];
        b[0x38] = self.mode;
        b[0x39] = self.mode_applied;
        b
    }

    /// `MsSetAutoStatus`: forces the auto-statuses on, saving what each
    /// replaces. Returns whether `+0x6d2` is to be cleared (the counter at
    /// `+0x612` was 0 and is not any more).
    pub fn set(&mut self) -> bool {
        let c612 = self.counters[10];
        let c613 = self.counters[11];
        self.auto = self.auto_base;
        if self.mode == 1 || self.mode == 2 {
            for i in 0..3 {
                self.auto[i] = self.auto_extra[i] | self.auto_base[i];
            }
        }
        self.mode_applied = self.mode;
        (self.saved_status, self.status) = force(self.auto[0], 12, self.saved_status, self.status);
        for i in 0..COUNTERS {
            if (self.auto[1] >> i) & 1 != 0 {
                self.saved_counters[i] = self.counters[i];
                self.counters[i] = 0xff;
            }
        }
        if c613 == 0 && self.counters[11] != 0 && self.counters[12] != 0xff {
            self.counters[12] = 0;
        }
        (self.saved_status2, self.status2) =
            force(self.auto[2], 16, self.saved_status2, self.status2);
        self.counters[10] != 0 && c612 == 0
    }

    /// `MsResetAutoStatus`: puts back what the auto-statuses in force
    /// replaced.
    pub fn reset(&mut self) {
        self.status = restore(self.auto[0], 12, self.status, self.saved_status);
        for i in 0..COUNTERS {
            if (self.auto[1] >> i) & 1 != 0 {
                self.counters[i] = self.saved_counters[i];
            }
        }
        self.status2 = restore(self.auto[2], 16, self.status2, self.saved_status2);
    }
}

/// For each of the low `bits` bits of `mask`: the saved word takes the
/// live bit, the live bit is set. Returns `(saved, live)`.
fn force(mask: u16, bits: u32, mut saved: u16, mut live: u16) -> (u16, u16) {
    for i in 0..bits {
        if (mask >> i) & 1 != 0 {
            let b = 1u16 << i;
            saved = (!b & saved) | (b & live);
            live |= b;
        }
    }
    (saved, live)
}

/// For each of the low `bits` bits of `mask`: the live bit from the saved
/// word.
fn restore(mask: u16, bits: u32, mut live: u16, saved: u16) -> u16 {
    for i in 0..bits {
        if (mask >> i) & 1 != 0 {
            let b = 1u16 << i;
            live = (!b & live) | (b & saved);
        }
    }
    live
}

/// What `MsAutoCommandStatusCheck` reads of an actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandStatus {
    /// `+0xdce`.
    pub blocked: u8,
    /// `+0x606`.
    pub status: u16,
    /// `+0x608`, the first status counter.
    pub counter0: u8,
    /// `+0x617`, the high byte of status bits 2.
    pub status2_high: u8,
    /// `+0xdc8`.
    pub allowed: u8,
}

/// `MsAutoCommandStatusCheck(actor, need_allowed, no_400, no_800)`: whether
/// an auto command may run (1) or not (0).
#[must_use]
pub fn auto_command_ok(s: CommandStatus, need_allowed: bool, no_400: bool, no_800: bool) -> u32 {
    let ok = s.blocked == 0
        && s.status & 0x300 == 0
        && s.counter0 == 0
        && s.status2_high & 1 == 0
        && (!need_allowed || s.allowed != 0)
        && (!no_400 || s.status & 0x400 == 0)
        && (!no_800 || s.status & 0x800 == 0);
    u32::from(ok)
}

/// `MsCheckSetMotion(actor, mode)` from the actor's `+0x508`, `+0x608`,
/// `+0x432` and `+0x3f0`: whether a motion may be set (1) or not (0).
#[must_use]
pub fn motion_settable(busy: u8, counter0: u8, playing: u8, motion: u8, mode: i32) -> u32 {
    if busy != 0 || counter0 != 0 {
        return 0;
    }
    let ending = motion == 0x28 || motion == 0x29;
    u32::from(match mode {
        1 => playing == 0 || ending,
        2 => playing == 0 && !ending,
        _ => playing == 0,
    })
}

/// `msCheckSameTarget(chr, actor, targets)` given the actor's `+0x3f8`
/// (`multi`): 0 when `targets` is nobody or only `chr` (bit `chr & 0x1f`),
/// or, for a `multi` actor among them, more than two of bits 0-30; else -1.
#[must_use]
pub fn same_target(chr: u32, multi: impl FnOnce() -> u8, targets: u32) -> i32 {
    let me = 1u32 << (chr & 0x1f);
    if targets == me || targets == 0 {
        return 0;
    }
    if targets & me != 0 && multi() != 0 && (targets & 0x7fff_ffff).count_ones() > 2 {
        return 0;
    }
    -1
}

/// The actors where the game keeps them, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    use super::{
        ACTOR_SIZE, ACTORS, AUTO_STATUS_AT, AUTO_STATUS_SIZE, AutoStatus, Chr, CommandStatus,
        FIRST_MONSTER, MONSTERS, WEAPON_SIZE, auto_command_ok, chr, motion_settable, same_target,
        weak_level,
    };

    /// Pointer to the first actor (`MsGetChrTop`).
    pub const CHR_TOP: Global<u32> = Global::new(0x0113_34d4);
    /// The spare record `MsGetChr` returns for other indices.
    pub const CHR_SPARE: Global<u8> = Global::new(0x0113_34d8);
    /// Pointer to the monster actors (`MsGetMon`).
    pub const MON_TOP: Global<u32> = Global::new(0x0113_4468);
    /// Pointer to the 0x390-byte records (`MsGetWeapon`).
    pub const WEAPON_TOP: Global<u32> = Global::new(0x0113_446c);

    /// `MsGetChr(i)` (`0x00794020`): the actor or record of index `i`.
    pub fn ms_get_chr(mem: &impl Mem, i: u32) -> u32 {
        match chr(i) {
            Chr::Actor(n) => (n * ACTOR_SIZE).wrapping_add(CHR_TOP.get(mem)),
            Chr::Weapon(n) => (n * WEAPON_SIZE).wrapping_add(WEAPON_TOP.get(mem)),
            Chr::Spare => CHR_SPARE.ptr(mem).addr,
        }
    }

    /// `MsGetChrTop` (`0x00795970`).
    pub fn ms_get_chr_top(mem: &impl Mem) -> u32 {
        CHR_TOP.get(mem)
    }

    /// `MsGetMon(i)` (`0x00795aa0`): monster actor `i & 0xff` (not
    /// checked).
    pub fn ms_get_mon(mem: &impl Mem, i: u32) -> u32 {
        ((i & 0xff) * ACTOR_SIZE).wrapping_add(MON_TOP.get(mem))
    }

    /// `MsGetWeapon(i, second)` (`0x00795b10`): actor `i & 0xff`'s first or
    /// (`second` not 0) second 0x390-byte record; null from 0x1f on.
    pub fn ms_get_weapon(mem: &impl Mem, i: u32, second: i32) -> u32 {
        let mut i = i & 0xff;
        if i >= ACTORS {
            return 0;
        }
        if second != 0 {
            i += ACTORS;
        }
        (i * WEAPON_SIZE).wrapping_add(WEAPON_TOP.get(mem))
    }

    /// `MsBtlChrNumCheck(i)` (`0x00793650`): 1 when `i & 0xff` is an actor.
    #[must_use]
    pub fn ms_btl_chr_num_check(i: u32) -> u32 {
        u32::from(i & 0xff < ACTORS)
    }

    /// `MsGetRamChrMonster(i)` (`0x0079aef0`): 1 when `i & 0xff` is a
    /// monster actor.
    #[must_use]
    pub fn ms_get_ram_chr_monster(i: u32) -> u32 {
        u32::from((i & 0xff).wrapping_sub(FIRST_MONSTER) < MONSTERS)
    }

    /// `MsCalcWeakLevel(hp, max)` (`0x0078bf00`).
    #[must_use]
    pub fn ms_calc_weak_level(hp: i32, max: i32) -> u32 {
        weak_level(hp, max)
    }

    fn byte(mem: &impl Mem, actor: u32, offset: u32) -> u8 {
        Ptr::<u8>::new(actor).field::<u8>(offset).get(mem)
    }

    /// `MsAutoCommandStatusCheck(actor, need_allowed, no_400, no_800)`
    /// (`0x007b24a0`).
    pub fn ms_auto_command_status_check(
        mem: &impl Mem,
        actor: u32,
        need_allowed: i32,
        no_400: i32,
        no_800: i32,
    ) -> u32 {
        let a = Ptr::<u8>::new(actor);
        let s = CommandStatus {
            blocked: byte(mem, actor, 0xdce),
            status: a.field::<u16>(0x606).get(mem),
            counter0: byte(mem, actor, 0x608),
            status2_high: byte(mem, actor, 0x617),
            allowed: byte(mem, actor, 0xdc8),
        };
        auto_command_ok(s, need_allowed != 0, no_400 != 0, no_800 != 0)
    }

    /// `MsCheckSetMotion(actor, mode)` (`0x007af320`).
    pub fn ms_check_set_motion(mem: &impl Mem, actor: u32, mode: i32) -> u32 {
        motion_settable(
            byte(mem, actor, 0x508),
            byte(mem, actor, 0x608),
            byte(mem, actor, 0x432),
            byte(mem, actor, 0x3f0),
            mode,
        )
    }

    fn copy3(mem: &mut impl Mem, actor: u32, from: u32, to: u32) {
        for i in 0..3 {
            let v = byte(mem, actor, from + i);
            mem.write(actor.wrapping_add(to + i), v);
        }
    }

    /// `msCheckSameTarget(chr, actor, targets)` (`0x00796d70`): see
    /// [`same_target`]; `+0x3f8` is read only when `chr` is a target.
    pub fn ms_check_same_target(mem: &impl Mem, chr: u32, actor: u32, targets: u32) -> i32 {
        same_target(chr, || byte(mem, actor, 0x3f8), targets)
    }

    /// `MsBtlChrInfoFree(actor)` (`0x00793570`): clears the busy and motion
    /// bytes and the word at `+0x4c`.
    pub fn ms_btl_chr_info_free(mem: &mut impl Mem, actor: u32) {
        for o in [0x508, 0x432, 0x42e, 0x41c, 0x415] {
            mem.write(actor.wrapping_add(o), 0u8);
        }
        mem.write(actor.wrapping_add(0x4c), 0u32);
    }

    /// `MsPushBtlPos(actor)` (`0x007ac650`): saves the battle position
    /// (`+0x4fc`, 3 bytes) at `+0x509`.
    pub fn ms_push_btl_pos(mem: &mut impl Mem, actor: u32) {
        copy3(mem, actor, 0x4fc, 0x509);
    }

    /// `MsPopBtlPos(actor)` (`0x007ac600`): restores it.
    pub fn ms_pop_btl_pos(mem: &mut impl Mem, actor: u32) {
        copy3(mem, actor, 0x509, 0x4fc);
    }

    fn auto_status(mem: &impl Mem, actor: u32) -> AutoStatus {
        let b: [u8; AUTO_STATUS_SIZE] = mem.read(actor.wrapping_add(AUTO_STATUS_AT));
        AutoStatus::from_bytes(&b)
    }

    fn put_auto_status(mem: &mut impl Mem, actor: u32, s: &AutoStatus) {
        mem.write(actor.wrapping_add(AUTO_STATUS_AT), s.to_bytes());
    }

    /// `MsSetAutoStatus(actor)` (`0x0079b290`): [`AutoStatus::set`].
    pub fn ms_set_auto_status(mem: &mut impl Mem, actor: u32) {
        let mut s = auto_status(mem, actor);
        let clear = s.set();
        put_auto_status(mem, actor, &s);
        if clear {
            mem.write(actor.wrapping_add(0x6d2), 0u8);
        }
    }

    /// `MsResetAutoStatus(actor)` (`0x0079b1a0`): [`AutoStatus::reset`].
    pub fn ms_reset_auto_status(mem: &mut impl Mem, actor: u32) {
        let mut s = auto_status(mem, actor);
        s.reset();
        put_auto_status(mem, actor, &s);
    }

    /// `MsSetEternalRelife(actor)` (`0x0078deb0`): auto-life (`+0x616` bit
    /// `0x200`) when `+0x3b` is set.
    pub fn ms_set_eternal_relife(mem: &mut impl Mem, actor: u32) {
        if byte(mem, actor, 0x3b) != 0 {
            let p = Ptr::<u8>::new(actor).field::<u16>(0x616);
            let v = p.get(mem) | 0x200;
            p.set(mem, v);
        }
    }

    /// `MsMotionSpeedReset(actor)` (`0x007ab730`).
    pub fn ms_motion_speed_reset(mem: &mut impl Mem, actor: u32) {
        mem.write(actor.wrapping_add(0x415), 0u8);
        mem.write(actor.wrapping_add(0x417), 0u16);
    }

    /// `MsTerminateMotion(actor)` (`0x007ab6c0`): clears the motion state;
    /// phase 2 (`+0xdfb`) becomes 3.
    pub fn ms_terminate_motion(mem: &mut impl Mem, actor: u32) {
        let phase = byte(mem, actor, 0xdfb);
        mem.write(actor.wrapping_add(0x432), 0u16);
        mem.write(actor.wrapping_add(0x43e), 0u8);
        mem.write(actor.wrapping_add(0x404), 0u16);
        if phase == 2 {
            mem.write(actor.wrapping_add(0xdfb), 3u8);
        }
    }

    /// `msSetEndDir(actor)` (`0x00793260`): the end direction (`+0x49c`) is
    /// direction `+0x511` (signed) of `+0x514` turned half way round. The
    /// original adds `(double)(float)pi` on the x87 at 53 bits and stores a
    /// float: the same as this in `f64`.
    pub fn ms_set_end_dir(mem: &mut impl Mem, actor: u32) {
        let i = Ptr::<u8>::new(actor).field::<i8>(0x511).get(mem);
        let at = actor
            .wrapping_add(0x514)
            .wrapping_add(i32::from(i).cast_unsigned().wrapping_mul(4));
        let d: f32 = mem.read(at);
        #[allow(clippy::cast_possible_truncation)] // fstps
        let r = (f64::from(d) + f64::from(std::f32::consts::PI)) as f32;
        mem.write(actor.wrapping_add(0x49c), r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_map_to_tables() {
        assert_eq!(chr(0x11e), Chr::Actor(0x1e));
        assert_eq!(chr(0x1f), Chr::Weapon(0));
        assert_eq!(chr(0x5d), Chr::Spare);
        assert_eq!(weak_level(0, 10), 3);
        assert_eq!(weak_level(2, 10), 2);
        assert_eq!(weak_level(3, 10), 1);
        assert_eq!(weak_level(5, 0), 2);
    }

    #[test]
    fn auto_status_round_trips() {
        let mut b = [0u8; AUTO_STATUS_SIZE];
        for (i, v) in b.iter_mut().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            {
                *v = (i * 7 + 3) as u8;
            }
        }
        assert_eq!(AutoStatus::from_bytes(&b).to_bytes(), b);
        let mut s = AutoStatus {
            status: 0b0101,
            auto_base: [0b0011, 0b1, 0],
            ..AutoStatus::default()
        };
        s.set();
        assert_eq!((s.status, s.saved_status), (0b0111, 0b0001));
        assert_eq!((s.counters[0], s.saved_counters[0]), (0xff, 0));
        s.reset();
        assert_eq!(s.status, 0b0101);
        assert_eq!(s.counters[0], 0);
    }
}
