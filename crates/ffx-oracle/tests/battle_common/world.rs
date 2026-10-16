//! The battle world the damage and command tests share: 31 actors (the
//! snapshot's live ones, else our own), kernel tables (the snapshot's, else
//! fake ones), a scratch command, result and out values.

use ffx_battle::actor::bridge::{CHR_TOP, WEAPON_TOP};
use ffx_battle::damage::bridge as d;
use ffx_oracle::Oracle;

use super::*;
/// `TOGetFFXLang`, which `DmgCalc_Bribe` asks (another module's).
pub const TO_GET_FFX_LANG: u32 = 0x0064_10e0;
/// `achievementUnlockAchievement` (main), stubbed in both oracles.
pub const ACHIEVEMENT: u32 = 0x0082_2380;

/// Where the cases live: the actors, a command, a result and scratch for
/// the out values.
pub struct World {
    pub actors: u32,
    pub command: u32,
    pub info: u32,
    pub out: u32,
    /// A results block (`MsCalcDamage`): 0x18 bytes, then 16 of 0x2c.
    pub results: u32,
    /// The kernel's commands (on a snapshot), else empty.
    pub commands: Vec<u32>,
    pub synthetic: bool,
}

pub const ACTOR: u32 = ffx_battle::actor::ACTOR_SIZE;
pub const ACTORS: u32 = ffx_battle::actor::ACTORS;

/// A float's bits with signalling NaNs made quiet (the emulator keeps them
/// signalling, the CPU does not: `docs/porting.md`).
pub fn quiet(bits: u32) -> u32 {
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        bits | 0x0040_0000
    } else {
        bits
    }
}

/// A fake kernel table (`MsGetExcelData`'s layout) of one segment holding
/// `len` random bytes, records 0..`len` of stride 1.
pub fn fake_table(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, len: u32) -> u32 {
    fake_records(a, b, rng, len, 1)
}

/// A fake kernel table of `n` random records of `stride` bytes.
pub fn fake_records(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    n: u32,
    stride: u16,
) -> u32 {
    let len = n * u32::from(stride);
    let t = scratch(a, b, 0x20 + len as usize);
    let mut head = vec![0u8; 0x20];
    head[0] = 1;
    head[8..10].copy_from_slice(&0u16.to_le_bytes());
    head[10..12].copy_from_slice(&u16::try_from(n - 1).unwrap().to_le_bytes());
    head[12..14].copy_from_slice(&stride.to_le_bytes());
    head[16..20].copy_from_slice(&0x20u32.to_le_bytes());
    poke(a, b, t, &head);
    fill(a, b, rng, t + 0x20, len);
    t
}

pub fn world(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, synthetic: bool) -> World {
    let live = !synthetic
        && readable(a, CHR_TOP.get(b), (ACTORS * ACTOR) as usize)
        && readable(a, WEAPON_TOP.get(b), 0x390)
        && readable(a, ffx_kernel::bridge::CTB_BASE.get(b), 0x20);
    // without a battle running, actors of our own on the snapshot's globals
    let actors = if live {
        CHR_TOP.get(b)
    } else {
        let actors = scratch(a, b, (ACTORS * ACTOR) as usize);
        set(a, b, CHR_TOP, actors);
        let weapons = scratch(a, b, (2 * ACTORS * ffx_battle::actor::WEAPON_SIZE) as usize);
        set(a, b, WEAPON_TOP, weapons);
        let t = fake_table(a, b, rng, 0x100);
        set(a, b, ffx_kernel::bridge::CTB_BASE, t);
        actors
    };
    // the command tables, fake where the snapshot has none
    for g in [
        ffx_kernel::bridge::COMMAND,
        ffx_kernel::bridge::ITEM,
        ffx_kernel::bridge::MON_MAGIC1,
        ffx_kernel::bridge::MON_MAGIC2,
    ] {
        if synthetic || !readable(a, g.get(b), 0x20) {
            let t = fake_records(a, b, rng, 0x40, 0x60);
            set(a, b, g, t);
        }
    }
    let mut commands = Vec::new();
    if !synthetic && readable(a, ffx_kernel::bridge::COMMAND.get(b), 0x20) {
        for id in (0x3000..0x3100).step_by(3) {
            let p = ffx_kernel::bridge::rom(b, ffx_kernel::bridge::COMMAND, id, 0);
            let q = ffx_kernel::bridge::rom(a, ffx_kernel::bridge::COMMAND, id, 0);
            assert_eq!(p, q);
            if readable(a, p, 0x60) {
                commands.push(p);
            }
        }
    }
    let w = World {
        actors,
        command: scratch(a, b, 0x60),
        info: scratch(a, b, 0x2c),
        out: scratch(a, b, 0x100),
        results: scratch(a, b, 0x2d8),
        commands,
        synthetic: !live,
    };
    same("world", a, b);
    w
}

impl World {
    pub fn actor(&self, i: u32) -> u32 {
        self.actors + i * ACTOR
    }

    /// A command: the scratch one with random bytes, or a kernel one.
    pub fn command(&self, a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) -> u32 {
        if !self.commands.is_empty() && rng.below(2) == 0 {
            return self.commands[rng.below(u32::try_from(self.commands.len()).unwrap()) as usize];
        }
        fill(a, b, rng, self.command, 0x60);
        self.command
    }

    /// Random values over the actors' battle fields (each round on the
    /// synthetic image, every other round on a snapshot), indices sane
    /// most of the time.
    pub fn shuffle(&self, a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, round: u32) {
        if !self.synthetic && round.is_multiple_of(2) {
            return;
        }
        for i in 0..ACTORS {
            let p = self.actor(i);
            for (at, len) in [
                (0x3b, 1),
                (0x590, 0x1a0),
                (0xd20, 0x20),
                (0xdf0, 8),
                (0xf5c, 4),
            ] {
                fill(a, b, rng, p + at, len);
            }
            let id = if rng.below(8) == 0 {
                rng.byte()
            } else {
                i.to_le_bytes()[0]
            };
            poke(a, b, p + 0xc, &[id]);
            for at in [0xd2c, 0xd30] {
                let f = match rng.below(4) {
                    0 => 1.0f32.to_bits() + rng.below(0x80_0000),
                    1 => quiet(rng.next()),
                    _ => (f32::from(i16::try_from(rng.range(-500, 500)).unwrap()) / 7.0).to_bits(),
                };
                poke(a, b, p + at, &f.to_le_bytes());
            }
        }
        fill(a, b, rng, self.info, 0x2c);
        fill(a, b, rng, self.out, 0x100);
        for g in [
            d::DEBUG_ALWAYS_CRIT,
            d::DEBUG_ALWAYS_HIT,
            d::DEBUG_ALL_MISS,
            d::COUNTER_ON,
            d::DEBUG_NO_DAMAGE_MONSTER,
            d::DEBUG_NO_DAMAGE_PARTY,
        ] {
            let v = u8::from(rng.below(5) == 0);
            set(a, b, g, v);
        }
        set(
            a,
            b,
            d::ESCAPE_RULE,
            rng.range(-1, 4).to_le_bytes()[0].cast_signed(),
        );
        set(a, b, d::GIL_AMOUNT, rng.word().cast_signed());
        if self.synthetic {
            set(a, b, ffx_bridge::x87::SSE2_AVAILABLE, rng.below(2));
        }
    }
}

/// A damage value: small, large, negative or at a limit.
pub fn damage(rng: &mut Xorshift) -> u32 {
    match rng.below(5) {
        0 => rng.word(),
        1 => rng.range(-10000, 0).cast_unsigned(),
        _ => rng.below(100_000),
    }
}
