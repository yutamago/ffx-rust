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
}
