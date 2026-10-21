//! Wakka's reels (`docs/rewrite-plan.md`, M2): `MsSetReelResult`
//! (`0x007b1b50`) turns the four spins' three symbols each into the reel
//! entries (`MsGetReelEntry_007b19d0`: kind, hits, command) and deals their
//! hits, and `MsGetReelTargets_007b1910` (`0x007b1910`, a callback of the
//! effects' table `0x00c656fc`) hands a spin's targets to the effect.
//!
//! A spin's symbols (three bytes, [`ms_reel_kind`]) give its kind: 0
//! (some symbol 0xff: no spin), 0xf (three of symbol 10), 2/4/6/8 (three
//! of symbol 0/1/2/3), 1/3/5/7 (two of them), 0xa/0xc/0xe and 9/0xb/0xd
//! (three or two of symbol 7/8/9), else 0x11 with a hit count from
//! symbols 5 and 6 (0x10 with none). The kind's command is the `u16` at
//! [`REEL_COMMANDS`]` + 2 kind`.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_bridge::x87::{load, store};

    use crate::actor::bridge::ms_get_chr;
    use crate::exec::bridge::Call;
    use crate::minor::bridge::ms_count_slots_of;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = mem.rebase(sym);
        call(mem, f, args)
    }

    /// `MsGetReelResultKind_007b1710(symbols, &hits)` (`0x007b1710`, name
    /// ours; caller `MsSetReelResult`): a spin's kind (see the module) and
    /// its hits (`*hits`: 0 with a symbol 0xff, 1 for the matches, 0xc or 6
    /// for three of symbol 6 or 5, else `fives + 2 sixes`).
    pub fn ms_reel_kind(mem: &impl Mem, symbols: u32) -> (u32, u32) {
        let count = |v: u32| ms_count_slots_of(mem, symbols, v);
        if count(0xff) != 0 {
            return (0, 0);
        }
        if count(10) >= 3 {
            return (0xf, 1);
        }
        let four = [count(0), count(1), count(2), count(3)];
        for (k, &c) in (0u32..).zip(&four) {
            if c >= 3 {
                return (2 + 2 * k, 1);
            }
        }
        for (k, &c) in (0u32..).zip(&four) {
            if c >= 2 {
                return (1 + 2 * k, 1);
            }
        }
        let three = [count(7), count(8), count(9)];
        for (k, &c) in (0u32..).zip(&three) {
            if c >= 3 {
                return (0xa + 2 * k, 1);
            }
        }
        for (k, &c) in (0u32..).zip(&three) {
            if c >= 2 {
                return (9 + 2 * k, 1);
            }
        }
        let (fives, sixes) = (count(5), count(6));
        if sixes >= 3 {
            (0x11, 0xc)
        } else if fives >= 3 {
            (0x11, 6)
        } else if fives == 0 && sixes == 0 {
            (0x10, 1)
        } else {
            (0x11, fives + 2 * sixes)
        }
    }

    /// `MsGetReelResultKind_007b1710(symbols, hits)` (`0x007b1710`):
    /// [`ms_reel_kind`] with the hits into `*hits`.
    pub fn ms_get_reel_result_kind(mem: &mut impl Mem, symbols: u32, hits: u32) -> u32 {
        let (kind, n) = ms_reel_kind(mem, symbols);
        mem.write(hits, n);
        kind
    }

    /// `dbgPrintf` (`0x0062f500`, a bare `ret` on the PC) and its two
    /// formats here: the reels' times (Wakka) and each spin's result.
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const REEL_TIME_TEXT: u32 = 0x00b5_4fc8;
    pub const REEL_RESULT_TEXT: u32 = 0x00b5_5054;
    /// The reel kinds' commands, `u16` by kind from 1 (`g_ReelCommands`
    /// at `0x00c43960`, after `g_TidusLimitNeeds`): kind `k`'s at this
    /// plus `2 k`.
    pub const REEL_COMMANDS: u32 = 0x00c4_395e;
    /// Bytes of [`ms_set_reel_result`]'s `local`.
    pub const REEL_LOCAL: u32 = 0x10;

    /// `MsSetReelResult(i, symbols, time, start)` (`0x007b1b50`): actor
    /// `i & 0xff`'s reels. Wakka (4) keeps the two times (`+0xd2c`,
    /// `+0xd30`, `f32` as the x87 copies them), `+0xd26` set, and prints
    /// them. The targets are the actors with a results block (`+0x774`,
    /// two 0x2d8 bytes apart) of this user (`+2`) and command number (`+3`,
    /// its command cue entry's byte 2, `MsGetChrCommand`): the first such
    /// block each, its first two bytes cleared. Each of the four spins
    /// (three symbols each from `symbols`) gets its reel entry: with a
    /// kind ([`ms_reel_kind`]) and targets, its command
    /// ([`REEL_COMMANDS`], `MsGetCommand(i, 0, -1, 0, &id)`) by its target
    /// kind (`+0x1a` bits 2-3): 0 the spin's hits each on a random target
    /// (`brnd(0x13) % targets`), 1 and 2 one hit on every target, 3 none;
    /// each hit is `MsCalcDamage(i, actor, t, MsGetChr(t), command, id,
    /// block, 0, &hits, &misses, 1)` with the target in the actor's hit
    /// list (`+0xd80` on) and the block's `+0` counted up. The entry is
    /// the kind (0 without one or targets), the hits (the targets for 1
    /// and 2) and the command ID (`u16`, 0 without); each spin is printed.
    /// Then the targets' blocks' `+0` cleared and `+0xd26` cleared. `local`
    /// is the original's frame (the ID, the two counts); `eax` the actor.
    #[allow(clippy::too_many_lines)]
    pub fn ms_set_reel_result<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        [i, symbols, time, start]: [u32; 4],
        local: u32,
    ) -> u32 {
        let i = i & 0xff;
        let actor = ms_get_chr(mem, i);
        if i == 4 {
            mem.write(actor.wrapping_add(0xd2c), store(load(time)));
            mem.write(actor.wrapping_add(0xd26), 1u8);
            mem.write(actor.wrapping_add(0xd30), store(load(start)));
            let (t, s) = (load(time).to_bits(), load(start).to_bits());
            #[allow(clippy::cast_possible_truncation)] // the doubles' words
            let words = [t as u32, (t >> 32) as u32, s as u32, (s >> 32) as u32];
            let fmt = mem.rebase(REEL_TIME_TEXT);
            game(
                mem,
                &mut *call,
                DBG_PRINTF,
                &[fmt, words[0], words[1], words[2], words[3]],
            );
        }
        let cue = crate::actor::bridge::ms_get_chr_command(mem, i);
        let number = byte(mem, cue, 2);
        let mut who = [0u8; 0x1f];
        let mut blocks = [0u32; 0x1f];
        let mut n = 0usize;
        for k in 0..0x1f {
            let a = ms_get_chr(mem, k);
            for r in 0..2 {
                let block = a.wrapping_add(0x774 + r * 0x2d8);
                if u32::from(byte(mem, block, 2)) == i && byte(mem, block, 3) == number {
                    #[allow(clippy::cast_possible_truncation)] // below 0x1f
                    let k8 = k as u8;
                    who[n] = k8;
                    blocks[n] = block;
                    n += 1;
                    mem.write(block, 0u16);
                    break;
                }
            }
        }
        let (id_at, hits_at, misses_at) = (local, local.wrapping_add(4), local.wrapping_add(8));
        let lang = crate::init::bridge::TO_GET_FFX_LANG_FN;
        let mut dealt = 0u32;
        let mut spin = symbols;
        for e in 0..4 {
            let entry = crate::misc::bridge::ms_get_reel_entry(mem, e);
            let (kind, hits) = ms_reel_kind(mem, spin);
            mem.write(id_at, 0u32);
            let (kind, count) = if kind == 0 || n == 0 {
                (0, 0)
            } else {
                let table = mem.rebase(REEL_COMMANDS);
                let id = u32::from(mem.read::<u16>(table.wrapping_add(2 * kind)));
                mem.write(id_at, id);
                let command = crate::command::bridge::ms_get_command(mem, u32::MAX, 0, id_at);
                let id: u32 = mem.read(id_at);
                let targets = (u32::from(mem.read::<u16>(command.wrapping_add(0x1a))) >> 2) & 3;
                let mut hit = |mem: &mut M, call: Call<'_, M>, j: usize| {
                    let (t, block) = (u32::from(who[j]), blocks[j]);
                    let target = ms_get_chr(mem, t);
                    mem.write(actor.wrapping_add(0xd80).wrapping_add(dealt), who[j]);
                    dealt += 1;
                    let out = crate::damage::bridge::DmgCalcOut {
                        damage: 0,
                        hits: hits_at,
                        misses: misses_at,
                        changed: 0,
                    };
                    crate::damage::bridge::ms_calc_damage(
                        mem,
                        i,
                        actor,
                        t,
                        target,
                        command,
                        id,
                        block,
                        out,
                        1,
                        |m: &mut M| game(m, &mut *call, lang, &[]),
                    );
                    let v = byte(mem, block, 0);
                    mem.write(block, v.wrapping_add(1));
                };
                match targets {
                    0 => {
                        for _ in 0..hits.cast_signed().max(0) {
                            let r = crate::rng::bridge::brnd(mem, 0x13).cast_signed();
                            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                            let j = r.wrapping_rem(n as i32);
                            hit(mem, &mut *call, j.cast_unsigned() as usize);
                        }
                        (kind, hits)
                    }
                    1 | 2 => {
                        for j in 0..n {
                            hit(mem, &mut *call, j);
                        }
                        #[allow(clippy::cast_possible_truncation)] // below 0x1f
                        (kind, n as u32)
                    }
                    _ => (kind, hits),
                }
            };
            mem.write(entry.wrapping_add(1), count.to_le_bytes()[0]);
            mem.write(entry, kind.to_le_bytes()[0]);
            let id: u32 = mem.read(id_at);
            #[allow(clippy::cast_possible_truncation)] // the entry's u16
            mem.write(entry.wrapping_add(2), id as u16);
            let fmt = mem.rebase(REEL_RESULT_TEXT);
            let s = [byte(mem, spin, 0), byte(mem, spin, 1), byte(mem, spin, 2)].map(u32::from);
            game(
                mem,
                &mut *call,
                DBG_PRINTF,
                &[fmt, e, s[0], s[1], s[2], kind, count, id],
            );
            spin = spin.wrapping_add(3);
        }
        for &block in &blocks[..n] {
            mem.write(block, 0u8);
        }
        mem.write(actor.wrapping_add(0xd26), 0u8);
        actor
    }

    /// `MsGetReelTargets_007b1910(i, &hits, spin, &kind)` (`0x007b1910`,
    /// name ours; entry of the callback table `0x00c656fc`): actor
    /// `i & 0xff`'s targets of spin `spin` (its reel entry's hits, after
    /// those of the spins before it in the actor's hit list `+0xd80`) into
    /// its list `+0xd40` (31 bytes, 0xff after them); the entry's hits and
    /// kind into `*hits` and `*kind` when not null. The list, or 0 for a
    /// non-actor.
    pub fn ms_get_reel_targets(mem: &mut impl Mem, [i, hits, spin, kind]: [u32; 4]) -> u32 {
        let i = i & 0xff;
        if crate::actor::bridge::ms_btl_chr_num_check(i) == 0 {
            return 0;
        }
        let actor = ms_get_chr(mem, i);
        let list = actor.wrapping_add(0xd40);
        let all = actor.wrapping_add(0xd80);
        let mut e = 0u32;
        let mut before = 0u32;
        while e.cast_signed() < spin.cast_signed() {
            let entry = crate::misc::bridge::ms_get_reel_entry(mem, e);
            before = before.wrapping_add(u32::from(byte(mem, entry, 1)));
            e += 1;
        }
        let entry = crate::misc::bridge::ms_get_reel_entry(mem, e);
        if hits != 0 {
            mem.write(hits, u32::from(byte(mem, entry, 1)));
        }
        if kind != 0 {
            mem.write(kind, u32::from(byte(mem, entry, 0)));
        }
        let mut j = 0u32;
        if byte(mem, entry, 1) != 0 {
            while j < u32::from(byte(mem, entry, 1)) {
                let t = byte(mem, all, before.wrapping_add(j));
                mem.write(list.wrapping_add(j), t);
                j += 1;
            }
            if j >= 0x1f {
                return list;
            }
        }
        let rest = vec![0xffu8; (0x1f - j) as usize];
        mem.write_bytes(list.wrapping_add(j), &rest);
        list
    }
}
