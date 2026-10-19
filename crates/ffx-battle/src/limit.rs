//! Overdrives that hit a number of times (`docs/rewrite-plan.md`, M2):
//! `MsMenuLimitSet` (`0x007afe00`) shares an overdrive's hits out among
//! the targets once its minigame ends, and `0x007b0470` starts it from the
//! minigame's window.
//!
//! Each actor holds two results blocks (`+0x774`, 0x2d8 bytes apart: 0x18
//! bytes, then 16 results of 0x2c), whose fields used here are:
//!
//! | Offset | Type | Field |
//! |---|---|---|
//! | `+0` | `u8` | results drawn (set to 0 before the damage is worked out) |
//! | `+1` | `u8` | results wanted |
//! | `+2` | `u8` | the user's actor index |
//! | `+3` | `u8` | the command's number in the user's command cue entry |
//! | `+4` | `u8` | bit 2: the user's own block (a drain, not a target) |
//! | `+0x15` | `u8` | which hit command: 0 none or the first, 1 a repeat, 2 the last |
//! | `+0x16` | `u8` | set: not the target's last hit |
//! | `+0x38 + 0x2c k` | `i32` × 3 | result `k`'s HP, MP and CTB damage |

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // actors, indices
pub mod bridge {
    use ffx_bridge::{IMAGE_BASE, Mem};

    use crate::actor::bridge::{ms_get_chr, ms_get_chr_command};
    use crate::command::bridge::get_command;
    use crate::exec::bridge::Call;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base());
        call(mem, f, args)
    }

    /// The first of an actor's two results blocks, and their size.
    pub const RESULTS: u32 = 0x774;
    pub const RESULTS_SIZE: u32 = 0x2d8;
    /// Words of scratch [`ms_menu_limit_set`] needs for what its callees
    /// write through pointers (the original's stack locals): hits and
    /// misses (`MsCalcDamage`), the random list's length, a drain vector.
    pub const LIMIT_SCRATCH_WORDS: u32 = 8;
    /// `MsAtelRequestMagicCamera(...)` (battle, with the script).
    pub const MS_ATEL_REQUEST_MAGIC_CAMERA: u32 = 0x0079_8000;

    /// The hit command each target of an overdrive kind gets, by whether
    /// the minigame's count is 0 (`none`), else by the block's `+0x16`
    /// (`last` when 0, else `repeat`), with the `+0x15` mark (2 the last,
    /// 1 a repeat).
    fn hit_command(kind: u32, count: u32, more: bool) -> Option<(u8, u32)> {
        let three = |none: u32, last: u32, repeat: u32| {
            Some(if count == 0 {
                (0, none)
            } else if more {
                (1, repeat)
            } else {
                (2, last)
            })
        };
        let two = |none: u32, hit: u32| Some(if count == 0 { (0, none) } else { (1, hit) });
        match kind {
            0x20 => two(0x30eb, 0x3060),
            0x22 => three(0x310a, 0x310e, 0x3064),
            0x30 => two(0x30ec, 0x3061),
            0x32 => three(0x310b, 0x310f, 0x3065),
            0x40 => two(0x30ed, 0x3062),
            0x42 => three(0x310c, 0x3110, 0x3066),
            0x52 => three(0x310d, 0x3111, 0x3067),
            _ => None,
        }
    }

    /// One of the targets' remaining hits used up: down to 0 the block
    /// wants no more than it drew (`+1` = `+0`).
    fn spend(mem: &mut impl Mem, left: &mut i8, slot: u32, by: i8) {
        *left = left.wrapping_sub(by);
        if *left <= 0 {
            *left = 0;
            let drawn = byte(mem, slot, 0);
            mem.write(slot.wrapping_add(1), drawn);
        }
    }

    /// `MsMenuLimitSet(chr, user, count)` (`0x007afe00`): overdrive user
    /// `chr` (`user` its actor) with the minigame's `count` hands out its
    /// hits. The command of the user's `+0xf5c` (`MsGetCommand`) gives the
    /// kind (`+0x58`). Every actor's results block of this user and
    /// command number (the user's command cue entry `+2`) is found; a
    /// target's hits left are its wanted minus drawn, the user's own block
    /// (bit 2) wants none and is kept for the drain. Kind 0x25 takes
    /// `count` (at least 1) off the command's `+0x2b`, kinds 0x30 and 0x52
    /// with `count` 0 the difference between the command's and their
    /// fixed command's `+0x2b`; that many hits come off every target
    /// (unless the command has bit 0x8000 of `+0x1c`, or none), or one at
    /// a time from a random target among those with the most left
    /// (`brnd(0x10)`). Each target with hits left gets the kind's hit
    /// command ([`hit_command`]; 0x25 its own, 0x50 one with `count`, else
    /// none left) and its damage (`MsCalcDamage`, as many results as hits
    /// left); the targets' bits go to the user's `+0xdc0` and the command
    /// cue entry. With a drain block and a command, the random target list
    /// (`MsGetTargetRandomList`, its length into the battle's random
    /// target count) drains each target's next result into it
    /// (`MsCalcDrainResult` with the result's damage negated). The user's
    /// `+0xd26` is set meanwhile for characters 0 and 2. Returns the number
    /// of blocks found (Ghidra: void). `scratch` holds
    /// [`LIMIT_SCRATCH_WORDS`] words.
    #[allow(clippy::too_many_lines)] // one pass of the original
    pub fn ms_menu_limit_set<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        chr: u32,
        user: u32,
        count: i32,
        scratch: u32,
    ) -> u32 {
        let entry = ms_get_chr_command(mem, chr);
        let number = u32::from(byte(mem, entry, 2));
        let own_id = u32::from(mem.read::<u16>(user.wrapping_add(0xf5c)));
        let (mut command, own_id) = get_command(mem, u32::MAX, 0, own_id);
        let kind = u32::from(byte(mem, command, 0x58));
        // each actor's hits left and its block
        let mut left = [0i8; 0x1f];
        let mut slot = [0u32; 0x1f];
        let mut drain_block = 0u32;
        let mut found = 0u32;
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            for k in 0..2 {
                let s = a.wrapping_add(RESULTS + k * RESULTS_SIZE);
                if u32::from(byte(mem, s, 2)) == chr && u32::from(byte(mem, s, 3)) == number {
                    found += 1;
                    slot[i as usize] = s;
                    if byte(mem, s, 4) & 4 == 0 {
                        left[i as usize] =
                            byte(mem, s, 1).wrapping_sub(byte(mem, s, 0)).cast_signed();
                    } else {
                        drain_block = s;
                        mem.write(s.wrapping_add(1), 0u8);
                    }
                    break;
                }
            }
        }
        if chr == 0 || chr == 2 {
            mem.write(user.wrapping_add(0xd26), 1u8);
        }
        // the minigame's count: the hits to take off
        let mut minigame = count;
        let off = match kind {
            0x25 => {
                minigame = count.max(1);
                let off = i32::from(byte(mem, command, 0x2b)) - minigame;
                (off >= 0).then_some(off)
            }
            0x30 | 0x52 if count == 0 => {
                let fixed = if kind == 0x30 { 0x30ec } else { 0x310d };
                let (other, _) = get_command(mem, u32::MAX, 0, fixed);
                let off = i32::from(byte(mem, command, 0x2b)) - i32::from(byte(mem, other, 0x2b));
                command = other;
                Some(off)
            }
            _ => None,
        };
        if let Some(off) = off.filter(|&o| o != 0) {
            let every = command != 0 && mem.read::<u32>(command.wrapping_add(0x1c)) & 0x8000 == 0;
            if every {
                #[allow(clippy::cast_possible_truncation)] // the original's low byte
                let by = off as i8;
                for i in 0..0x1f {
                    if left[i] > 0 {
                        spend(mem, &mut left[i], slot[i], by);
                    }
                }
            } else {
                for _ in 0..off {
                    // the most hits left (positive), then one of those at random
                    let most = left.iter().copied().filter(|&v| v > 0).max().unwrap_or(0);
                    if most == 0 {
                        continue;
                    }
                    let n = left.iter().filter(|&&v| v == most).count();
                    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                    // at most 31
                    let n = n as i32;
                    let pick = crate::rng::bridge::brnd(mem, 0x10)
                        .cast_signed()
                        .wrapping_rem(n);
                    let mut seen = 0;
                    for i in 0..0x1f {
                        if left[i] == most {
                            seen += 1;
                            if seen > pick {
                                spend(mem, &mut left[i], slot[i], 1);
                                break;
                            }
                        }
                    }
                }
            }
        }
        // each target's hit command and damage
        let mut targets = 0u32;
        let mut last_command = 0u32;
        let lang = crate::init::bridge::TO_GET_FFX_LANG_FN;
        for i in 0..0x1f {
            let s = slot[i as usize];
            if s == 0 || left[i as usize] <= 0 {
                continue;
            }
            targets |= 1 << i;
            let target = ms_get_chr(mem, i);
            let (mark, id) = match kind {
                0x25 => (0, own_id),
                0x50 if minigame != 0 => (1, 0x3112),
                0x50 => {
                    left[i as usize] = 0;
                    let drawn = byte(mem, s, 0);
                    mem.write(s.wrapping_add(1), drawn);
                    (0, 0)
                }
                _ => hit_command(kind, minigame.cast_unsigned(), byte(mem, s, 0x16) != 0)
                    .unwrap_or((0, 0)),
            };
            mem.write(s.wrapping_add(0x15), mark);
            if id != 0 {
                mem.write(s, 0u8);
                let (c, id) = get_command(mem, u32::MAX, 0, id);
                last_command = c;
                let out = crate::damage::bridge::DmgCalcOut {
                    damage: 0,
                    hits: scratch,
                    misses: scratch.wrapping_add(4),
                    changed: 0,
                };
                let hits = i32::from(left[i as usize]);
                crate::damage::bridge::ms_calc_damage(
                    mem,
                    chr,
                    user,
                    i,
                    target,
                    c,
                    id,
                    s,
                    out,
                    hits,
                    |m: &mut M| game(m, &mut *call, lang, &[]),
                );
            }
        }
        mem.write(user.wrapping_add(0xdc0), targets);
        if number.cast_signed() < i32::from(byte(mem, entry, 3)) {
            mem.write(entry.wrapping_add((number + 1) * 0x10), targets);
        }
        mem.write(user.wrapping_add(0xd26), 0u8);
        if drain_block != 0 && last_command != 0 {
            let mut left = [0i8; 0x1f];
            let n_at = scratch.wrapping_add(8);
            let list = crate::cue::bridge::ms_get_target_random_list(mem, &mut *call, chr, n_at, 1);
            let n: i32 = mem.read(n_at);
            #[allow(clippy::cast_possible_truncation)] // the original's low byte
            crate::flow::bridge::RANDOM_TARGETS.set(mem, n as i8);
            let vector = scratch.wrapping_add(0x10);
            let mut j = 0;
            while j < mem.read::<i32>(n_at) {
                let t = usize::from(byte(mem, list, j.cast_unsigned()));
                // the list holds actor indices (below 0x1f)
                let (s, k) = (
                    slot.get(t).copied().unwrap_or(0),
                    left.get(t).copied().unwrap_or(0),
                );
                let r = s.wrapping_add((i32::from(k) * 0x2c).cast_unsigned() + 0x38);
                for w in 0..3 {
                    let v: i32 = mem.read(r.wrapping_add(4 * w));
                    mem.write(vector.wrapping_add(4 * w), v.wrapping_neg());
                }
                crate::damage::bridge::ms_calc_drain_result(
                    mem,
                    chr,
                    user,
                    last_command,
                    drain_block,
                    vector,
                    0,
                );
                if let Some(l) = left.get_mut(t) {
                    *l = k.wrapping_add(1);
                }
                j += 1;
            }
        }
        found
    }

    /// `0x007b0470(chr, count, x, y)` (`MsMenuLimitSet_007b0470`): with
    /// actor `chr`'s (its low byte) minigame done (`+0xd24`), its result
    /// kept (`+0xd28` the count, `+0xd2c`, `+0xd30` two floats, as bits),
    /// the done flag cleared, the hits handed out ([`ms_menu_limit_set`];
    /// the original also passes `x` truncated, which it ignores) and the
    /// magic camera asked for (`MsAtelRequestMagicCamera(0, chr, 0, 0x88,
    /// 0xffff, 0, 1, 0, 0)`): 1; 0 while not done.
    pub fn ms_menu_limit_start<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        chr: u32,
        count: u32,
        x: u32,
        y: u32,
        scratch: u32,
    ) -> u32 {
        let chr = chr & 0xff;
        let a = ms_get_chr(mem, chr);
        if byte(mem, a, 0xd24) == 0 {
            return 0;
        }
        // through the x87 (fld, fst): a signalling NaN comes out quiet
        let quiet = |v: u32| {
            if v & 0x7f80_0000 == 0x7f80_0000 && v & 0x007f_ffff != 0 {
                v | 0x0040_0000
            } else {
                v
            }
        };
        mem.write(a.wrapping_add(0xd2c), quiet(x));
        mem.write(a.wrapping_add(0xd24), 0u8);
        mem.write(a.wrapping_add(0xd30), quiet(y));
        mem.write(a.wrapping_add(0xd28), count);
        ms_menu_limit_set(mem, &mut *call, chr, a, count.cast_signed(), scratch);
        game(
            mem,
            call,
            MS_ATEL_REQUEST_MAGIC_CAMERA,
            &[0, chr, 0, 0x88, 0xffff, 0, 1, 0, 0],
        );
        1
    }
}
