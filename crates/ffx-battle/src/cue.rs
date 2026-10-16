//! The battle's turn queues (`docs/rewrite-plan.md`, M2): the input cue,
//! who chooses a command next, and the command cue, the commands chosen
//! and waiting to run.
//!
//! The input cue (`0x0112aa80`, count `i8` at `0x0112bde0`, at most 0x3e)
//! has entries of 8 bytes:
//!
//! | Offset | Field |
//! |---|---|
//! | `+0` | the actor |
//! | `+1`, `+2` | two flags from the caller (`+2` set: the turn's own entry) |
//! | `+3` | running (`MsExeInputCue`) |
//! | `+4` | priority; its high nibble orders inserted entries |
//!
//! The command cue (`0x0112ac70`, count `i8` at `0x0112bde1`) has entries of
//! 0x48 bytes: `+0` the actor, `+4` running (`MsExeComandCue`), `+7` a
//! kind in the high nibble (4: not the actor's own). Each actor keeps its
//! input entry at `+0xde4`, its command entry at `+0xde5` (0xff for none)
//! and counts at `+0xde6` and `+0xde7`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // count, entry, index
pub mod bridge {
    use ffx_bridge::Mem;

    use crate::actor::bridge::{ms_btl_chr_num_check, ms_get_chr};
    use crate::state::bridge::{
        COMMAND_CUE, COMMAND_CUE_COUNT, CUE_SIZE, INPUT_CUE, INPUT_CUE_COUNT,
    };

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn input(mem: &impl Mem, k: i32) -> u32 {
        INPUT_CUE
            .ptr(mem)
            .addr
            .wrapping_add(k.cast_unsigned().wrapping_mul(8))
    }

    fn command(mem: &impl Mem, k: i32) -> u32 {
        COMMAND_CUE
            .ptr(mem)
            .addr
            .wrapping_add(k.cast_unsigned().wrapping_mul(CUE_SIZE))
    }

    /// `MsCheckInputCueExe(chr)` (`0x007b1fd0`): 1 when the input cue
    /// holds an entry of `chr` whose priority kind is not 4.
    pub fn ms_check_input_cue_exe(mem: &impl Mem, chr: u32) -> u32 {
        let n = i32::from(INPUT_CUE_COUNT.get(mem));
        let hit = (0..n.max(0)).any(|k| {
            let e = input(mem, k);
            u32::from(byte(mem, e, 0)) == chr && byte(mem, e, 4) & 0xf0 != 0x40
        });
        u32::from(hit)
    }

    /// `MsCheckCommandCueExe(chr)` (`0x007b07d0`): 1 when the command cue
    /// holds an entry of `chr`, other than its own (`+0xde5`), of a kind
    /// other than 4.
    pub fn ms_check_command_cue_exe(mem: &impl Mem, chr: u32) -> u32 {
        let a = ms_get_chr(mem, chr);
        let n = i32::from(COMMAND_CUE_COUNT.get(mem));
        let hit = (0..n.max(0)).any(|k| {
            let e = command(mem, k);
            u32::from(byte(mem, e, 0)) == chr
                && i32::from(byte(mem, a, 0xde5)) != k
                && byte(mem, e, 7) & 0xf0 != 0x40
        });
        u32::from(hit)
    }

    /// `MsCheckInputCuePos(chr)` (`0x007b2020`): when the first input
    /// entry is running and `chr`'s, `chr` without an entry gets entry 0.
    pub fn ms_check_input_cue_pos(mem: &mut impl Mem, chr: u32) {
        let e = input(mem, 0);
        if INPUT_CUE_COUNT.get(mem) > 0 && byte(mem, e, 3) != 0 && u32::from(byte(mem, e, 0)) == chr
        {
            let a = ms_get_chr(mem, chr);
            if byte(mem, a, 0xde4) == 0xff {
                mem.write(a.wrapping_add(0xde4), 0u8);
            }
        }
    }

    /// `chr`'s own input entry (`+0xde4`) when it is there.
    fn own_input(mem: &impl Mem, chr: u32) -> Option<u32> {
        if INPUT_CUE_COUNT.get(mem) <= 0 {
            return None;
        }
        let s = byte(mem, ms_get_chr(mem, chr), 0xde4);
        let e = input(mem, i32::from(s));
        (s != 0xff && u32::from(byte(mem, e, 0)) == chr).then_some(e)
    }

    /// `MsCheckTurnInputCue(chr)` (`0x007b2060`): 1 when `chr`'s own input
    /// entry is its turn's (`+2`).
    pub fn ms_check_turn_input_cue(mem: &impl Mem, chr: u32) -> u32 {
        own_input(mem, chr).map_or(0, |e| u32::from(byte(mem, e, 2) != 0))
    }

    /// `MsExeInputCue(chr, on)` (`0x007b2290`): `chr`'s own input entry
    /// running or not; 1, or 0 without one.
    pub fn ms_exe_input_cue(mem: &mut impl Mem, chr: u32, on: u8) -> u32 {
        own_input(mem, chr).map_or(0, |e| {
            mem.write(e + 3, on);
            1
        })
    }

    /// `MsExeComandCue(chr, on)` (`0x007b0960`): `chr`'s own command entry
    /// (`+0xde5`) running or not; 1, or 0 without one.
    pub fn ms_exe_comand_cue(mem: &mut impl Mem, chr: u32, on: u8) -> u32 {
        if COMMAND_CUE_COUNT.get(mem) <= 0 {
            return 0;
        }
        let s = byte(mem, ms_get_chr(mem, chr), 0xde5);
        let e = command(mem, i32::from(s));
        if s == 0xff || u32::from(byte(mem, e, 0)) != chr {
            return 0;
        }
        mem.write(e + 4, on);
        1
    }

    /// `MsRegistInputCue(chr, prio, turn, flag)` (`0x007b2430`): appends an
    /// entry (not running) and counts it for `chr` (`+0xde6`); -1, or 0
    /// when the cue is full.
    pub fn ms_regist_input_cue(mem: &mut impl Mem, chr: u32, prio: u8, turn: u8, flag: u8) -> i32 {
        let n = INPUT_CUE_COUNT.get(mem);
        if n >= 0x3e {
            return 0;
        }
        let a = ms_get_chr(mem, chr);
        let c = byte(mem, a, 0xde6).wrapping_add(1);
        mem.write(a.wrapping_add(0xde6), c);
        let e = input(mem, i32::from(n));
        mem.write(e + 1, flag);
        mem.write(e + 2, turn);
        mem.write(e, chr.to_le_bytes()[0]);
        mem.write(e + 4, prio);
        mem.write(e + 3, 0u8);
        INPUT_CUE_COUNT.set(mem, n.wrapping_add(1));
        -1
    }

    /// `MsInsertInputCue(chr, prio, turn, flag, after, on)` (`0x007b2300`):
    /// inserts an entry for `chr` right after `after`'s own entry (or at
    /// the first entry not running), moved on past the entries whose
    /// priority (with a high nibble) is at most `prio` when `prio` has one;
    /// the rest move down. `flag` 0 clears `chr`'s `+0x65c`. -1, or 0 when
    /// the cue is full.
    pub fn ms_insert_input_cue(
        mem: &mut impl Mem,
        chr: u32,
        prio: u32,
        turn: u8,
        flag: u32,
        after: u32,
        on: u8,
    ) -> i32 {
        let n = i32::from(INPUT_CUE_COUNT.get(mem));
        if n >= 0x3e {
            return 0;
        }
        let mut pos = 0;
        let own = if ms_btl_chr_num_check(after) == 0 {
            None
        } else {
            let s = byte(mem, ms_get_chr(mem, after), 0xde4);
            (s != 0xff && u32::from(byte(mem, input(mem, i32::from(s)), 0)) == after)
                .then_some(i32::from(s))
        };
        if let Some(s) = own {
            pos = s + 1;
        } else if let Some(k) = (0..n).find(|&k| byte(mem, input(mem, k), 3) == 0) {
            pos = k;
        }
        if prio & 0xf0 != 0 {
            let mut k = pos;
            while k < n {
                let p = byte(mem, input(mem, k), 4);
                if p & 0xf0 == 0 || i32::from(p) > prio.cast_signed() {
                    pos = k;
                    break;
                }
                k += 1;
            }
            if k == n {
                pos = k;
            }
        }
        let mut k = n;
        while k > pos {
            let v: [u32; 2] = mem.read(input(mem, k - 1));
            mem.write(input(mem, k), v);
            k -= 1;
        }
        let a = ms_get_chr(mem, chr);
        let c = byte(mem, a, 0xde6).wrapping_add(1);
        mem.write(a.wrapping_add(0xde6), c);
        let e = input(mem, pos);
        mem.write(e + 3, on);
        mem.write(e, chr.to_le_bytes()[0]);
        mem.write(e + 1, flag.to_le_bytes()[0]);
        mem.write(e + 2, turn);
        mem.write(e + 4, prio.to_le_bytes()[0]);
        if flag == 0 {
            mem.write(a.wrapping_add(0x65c), 0u8);
        }
        #[allow(clippy::cast_possible_truncation)] // the count is an i8
        INPUT_CUE_COUNT.set(mem, (n as i8).wrapping_add(1));
        -1
    }

    /// `MsDeleteCommandCue(chr, slot, keep)` (`0x007b0860`): drops command
    /// entry `slot` when it is `chr`'s (unless it is `chr`'s own and `keep`
    /// is set), the later entries moving up; `chr` loses its own entry,
    /// one count (`+0xde7`) and its flags `+0x6de`, `+0x717`, `+0x446`.
    /// How many it dropped.
    pub fn ms_delete_command_cue(mem: &mut impl Mem, chr: u32, slot: u32, keep: u32) -> i32 {
        let n = COMMAND_CUE_COUNT.get(mem);
        if n <= 0 || slot == 0xff {
            return 0;
        }
        let a = ms_get_chr(mem, chr);
        let (mut dst, mut dropped) = (0, 0);
        for src in 0..i32::from(n) {
            let e = command(mem, src);
            let own = i32::from(byte(mem, a, 0xde5)) == src;
            if src.cast_unsigned() == slot
                && u32::from(byte(mem, e, 0)) == chr
                && !(own && keep != 0)
            {
                if own {
                    mem.write(a.wrapping_add(0xde5), 0xffu8);
                }
                let c = byte(mem, a, 0xde7).wrapping_sub(1);
                mem.write(a.wrapping_add(0xde7), c);
                dropped += 1;
                for o in [0x6de, 0x717, 0x446] {
                    mem.write(a.wrapping_add(o), 0u8);
                }
                continue;
            }
            if src != dst {
                let v: [u32; 0x12] = mem.read(e);
                let to = command(mem, dst);
                mem.write(to, v);
            }
            dst += 1;
        }
        if dropped > 0 {
            COMMAND_CUE_COUNT.set(mem, n - 1);
        }
        dropped
    }
    /// The two damage buffers of an actor (`+0x774`, `+0xa4c`, 0x2d8 bytes
    /// each: `+0` the next result, `+1` the count, `+2` the user, `+3` the
    /// command; results of 0x2c bytes from `+0x18`).
    const BUFFERS: [u32; 2] = [0x774, 0xa4c];

    /// `MsGetDamageResult(chr, target, &damage, &kind, &more)`
    /// (`0x0079a920`): in `target`'s buffers for `chr`'s (low byte) running
    /// command (`+2` of `MsGetChrCommand`), the next result (or the last):
    /// damage (`+0x32`, `u16`), kind (`+0x1a`) and its signed `+0x18`
    /// returned; `more` counts the buffers with results left. The second
    /// buffer wins; all 0 for no actor.
    pub fn ms_get_damage_result(
        mem: &mut impl Mem,
        chr: u32,
        target: u32,
        damage: u32,
        kind: u32,
        more: u32,
    ) -> i32 {
        let c = chr & 0xff;
        let (mut d, mut k, mut r, mut m) = (0u32, 0u32, 0i32, 0u32);
        if ms_btl_chr_num_check(c) != 0 {
            let id = byte(mem, crate::actor::bridge::ms_get_chr_command(mem, c), 2);
            let t = ms_get_chr(mem, target);
            for o in BUFFERS {
                let b = t.wrapping_add(o);
                if u32::from(byte(mem, b, 2)) != c || byte(mem, b, 3) != id {
                    continue;
                }
                let (next, count) = (i32::from(byte(mem, b, 0)), i32::from(byte(mem, b, 1)));
                let idx = if next < count {
                    m += 1;
                    next
                } else {
                    count - 1
                };
                let e = b.wrapping_add(idx.wrapping_mul(0x2c).cast_unsigned());
                d = u32::from(mem.read::<u16>(e.wrapping_add(0x32)));
                r = i32::from(byte(mem, e, 0x18).cast_signed());
                k = u32::from(byte(mem, e, 0x1a));
            }
        }
        for (p, v) in [(damage, d), (more, m), (kind, k)] {
            if p != 0 {
                mem.write(p, v);
            }
        }
        r
    }

    /// `MsGetDamageInfo(chr, target, &a, &b, &c)` (`0x0079a680`): over
    /// `target`'s buffers for `chr`'s running command, the sums of the
    /// results left (words `+0x20`, `+0x24`, `+0x28`), and how many are left
    /// (returned).
    pub fn ms_get_damage_info(mem: &mut impl Mem, chr: u32, target: u32, out: [u32; 3]) -> i32 {
        let c = chr & 0xff;
        let mut sums = [0i32; 3];
        let mut left = 0i32;
        if ms_btl_chr_num_check(c) != 0 {
            let id = byte(mem, crate::actor::bridge::ms_get_chr_command(mem, c), 2);
            let t = ms_get_chr(mem, target);
            for o in BUFFERS {
                let b = t.wrapping_add(o);
                if u32::from(byte(mem, b, 2)) != c || byte(mem, b, 3) != id {
                    continue;
                }
                let (next, count) = (i32::from(byte(mem, b, 0)), i32::from(byte(mem, b, 1)));
                left = left.wrapping_add(count - next);
                for k in next..count {
                    let e = b.wrapping_add(k.cast_unsigned() * 0x2c);
                    for (s, f) in sums.iter_mut().zip([0x38u32, 0x3c, 0x40]) {
                        *s = s.wrapping_add(mem.read(e.wrapping_add(f)));
                    }
                }
            }
        }
        for (p, v) in out.into_iter().zip(sums) {
            if p != 0 {
                mem.write(p, v);
            }
        }
        left
    }

    /// `MsGetTargetList(chr, &n)` (`0x007b7090`): `chr`'s (low byte) target
    /// list (`+0xd40`, 31 bytes): its running command's targets (`+0x10`
    /// words after its `+2`, else the actor's `+0xdc0`), party members
    /// first in party order, then the rest by index, 0xff after; `n` their
    /// number. Null, and `n` 0, for no actor.
    pub fn ms_get_target_list(mem: &mut impl Mem, chr: u32, n: u32) -> u32 {
        let c = chr & 0xff;
        let mut count = 0u32;
        let mut list = 0;
        if ms_btl_chr_num_check(c) != 0 {
            let a = ms_get_chr(mem, c);
            list = a.wrapping_add(0xd40);
            let cmd = crate::actor::bridge::ms_get_chr_command2(mem, c);
            let set: u32 = if cmd == 0 {
                mem.read(a.wrapping_add(0xdc0))
            } else {
                let k = u32::from(byte(mem, cmd, 2)) + 1;
                mem.read(cmd.wrapping_add(k * 0x10))
            };
            let set = set.cast_signed();
            let mut party = 0i32;
            for p in crate::menu::bridge::PARTY.get(mem) {
                if p != 0xff && (set >> (p & 31)) & 1 != 0 {
                    mem.write(list.wrapping_add(count), p);
                    party |= 1 << (p & 31);
                    count += 1;
                }
            }
            for i in 0u8..0x1f {
                if (set >> i) & 1 != 0 && (party >> i) & 1 == 0 {
                    mem.write(list.wrapping_add(count), i);
                    count += 1;
                }
            }
            for k in count..0x1f {
                mem.write(list.wrapping_add(k), 0xffu8);
            }
        }
        if n != 0 {
            mem.write(n, count);
        }
        list
    }
    /// Writes a command entry at `k`: the 0x48 bytes of `command`, then
    /// actor, flags and kind over them (`+3` and `+5` cleared or, from
    /// `MsRegistCommandCue`, `+4` too), counting it for `chr` (`+0xde7`);
    /// the command's items are used (`MsUseItem(chr, command, -1)`).
    #[allow(clippy::too_many_arguments)]
    fn put_command(
        mem: &mut impl Mem,
        k: i32,
        chr: u32,
        command: u32,
        flag: u8,
        turn: u8,
        running: Option<u8>,
        kind: u8,
    ) {
        let a = ms_get_chr(mem, chr);
        let c = byte(mem, a, 0xde7).wrapping_add(1);
        mem.write(a.wrapping_add(0xde7), c);
        let e = super::bridge::command(mem, k);
        let v: [u32; 0x12] = mem.read(command);
        mem.write(e, v);
        mem.write(e + 1, flag);
        match running {
            Some(on) => mem.write(e + 4, on),
            None => mem.write(e + 4, 0u16),
        }
        mem.write(e + 6, turn);
        mem.write(e, chr.to_le_bytes()[0]);
        mem.write(e + 2, 0u8);
        if running.is_some() {
            mem.write(e + 5, 0u8);
        }
        mem.write(e + 7, kind);
        crate::turn::bridge::ms_use_item(mem, chr, command, -1);
    }

    /// `MsRegistCommandCue(chr, command, turn, kind, flag)` (`0x007b0b90`):
    /// appends `command` (not running) for `chr`; -1, or 0 when full.
    pub fn ms_regist_command_cue(
        mem: &mut impl Mem,
        chr: u32,
        command: u32,
        turn: u8,
        kind: u8,
        flag: u8,
    ) -> i32 {
        let n = COMMAND_CUE_COUNT.get(mem);
        if n >= 0x3e {
            return 0;
        }
        put_command(mem, i32::from(n), chr, command, flag, turn, None, kind);
        COMMAND_CUE_COUNT.set(mem, n.wrapping_add(1));
        -1
    }

    /// `MsInsertCommandCue(chr, command, turn, kind, flag, on, after)`
    /// (`0x007b0a30`): inserts `command` as `MsInsertInputCue` does: after
    /// `after`'s own entry or at the first not running, past the entries
    /// whose kind (`+7`, with a high nibble) is at most `kind`.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_insert_command_cue(
        mem: &mut impl Mem,
        chr: u32,
        command: u32,
        turn: u8,
        kind: u32,
        flag: u8,
        on: u8,
        after: u32,
    ) -> i32 {
        let n = i32::from(COMMAND_CUE_COUNT.get(mem));
        if n >= 0x3e {
            return 0;
        }
        let mut pos = 0;
        let own = if ms_btl_chr_num_check(after) == 0 {
            None
        } else {
            let s = byte(mem, ms_get_chr(mem, after), 0xde5);
            (s != 0xff
                && u32::from(byte(mem, super::bridge::command(mem, i32::from(s)), 0)) == after)
                .then_some(i32::from(s))
        };
        if let Some(s) = own {
            pos = s + 1;
        } else if let Some(k) = (0..n).find(|&k| byte(mem, super::bridge::command(mem, k), 4) == 0)
        {
            pos = k;
        }
        if kind & 0xf0 != 0 {
            let mut k = pos;
            while k < n {
                let p = byte(mem, super::bridge::command(mem, k), 7);
                if p & 0xf0 == 0 || i32::from(p) > kind.cast_signed() {
                    pos = k;
                    break;
                }
                k += 1;
            }
            if k == n {
                pos = k;
            }
        }
        let mut k = n;
        while k > pos {
            let v: [u32; 0x12] = mem.read(super::bridge::command(mem, k - 1));
            let to = super::bridge::command(mem, k);
            mem.write(to, v);
            k -= 1;
        }
        put_command(
            mem,
            pos,
            chr,
            command,
            flag,
            turn,
            Some(on),
            kind.to_le_bytes()[0],
        );
        #[allow(clippy::cast_possible_truncation)] // the count is an i8
        COMMAND_CUE_COUNT.set(mem, (n as i8).wrapping_add(1));
        -1
    }

    /// `MsCleanCommandCue(chr, keep)` (`0x007b0830`): drops `chr`'s command
    /// entries from the last to the first (`MsDeleteCommandCue`); how many.
    pub fn ms_clean_command_cue(mem: &mut impl Mem, chr: u32, keep: u32) -> i32 {
        let n = i32::from(COMMAND_CUE_COUNT.get(mem));
        let mut dropped = 0i32;
        let mut k = n - 1;
        while k >= 0 {
            dropped =
                dropped.wrapping_add(ms_delete_command_cue(mem, chr, k.cast_unsigned(), keep));
            k -= 1;
        }
        dropped
    }
}
