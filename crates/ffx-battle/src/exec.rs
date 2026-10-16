//! Pieces of the battle's command execution (`docs/rewrite-plan.md`, M2):
//! what the per-turn driver (`0x00792210`) and the command parser
//! (`MsParseCommand`) ask before and after an action runs: paying gil,
//! Mimic, Rikku's Mix, the system message gate, the action request slot
//! and the overdrive gains after an action.
//!
//! The calls into other modules (the message windows, the CD library's
//! read queue) go through a [`Call`] callback to the game's code.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices and cue words
pub mod bridge {
    use ffx_bridge::frame::Frame;
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::{ms_get_chr, ms_get_chr_stat_death, ms_get_ram_chr_monster};
    use crate::pos::bridge::Trig;
    use crate::save::bridge::GIL;

    /// Calls the game's code at a run-time address with cdecl `u32`
    /// arguments, returning `eax`.
    pub type Call<'a, M> = &'a mut dyn FnMut(&mut M, u32, &[u32]) -> u32;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    /// The run-time address of the function at `sym`.
    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    /// Gil never exceeds this (`MsSetGIL`).
    pub const GIL_LIMIT: i32 = 999_999_999;
    /// What a Mimic repeats: the last command cue that could be mimicked
    /// (0x48 bytes), the gil it threw (`+0x5c`).
    pub const MIMIC: Global<u8> = Global::new(0x0112_be38);
    pub const MIMIC_GIL: Global<i32> = Global::new(0x0112_be94);
    /// The command targets a script set (`0x007a3d00`: an actor's
    /// `+0xdc0`), which `0x007ae2e0` gives the command it queues.
    pub const SCRIPT_TARGETS: Global<u32> = Global::new(0x0112_c8d0);
    /// The action request slot (`MsActionInit` sets it to -1) and the
    /// request block after it (0x48 bytes from `0x01136a78`).
    pub const ACTION_REQUEST: Global<u32> = Global::new(0x0113_6a6c);
    pub const ACTION_REQUEST_STEP: Global<u32> = Global::new(0x0113_6a70);
    pub const ACTION_REQUEST_BLOCK: Global<u8> = Global::new(0x0113_6a78);
    /// `g_CdLib`, the CD library's function table.
    pub const CD_LIB: Global<u32> = Global::new(0x0231_0c40);
    /// `0x007b0500`, which the Mix table's read calls when done.
    pub const MIX_LOADED: u32 = 0x007b_0500;
    /// The frame `0x007ae2e0` builds its command in.
    pub const SCRIPT_COMMAND_FRAME: u32 = 0xff50_0000;

    /// `MsSetGIL(n)` (`0x00785c20`, name ours): gil set to `n`, 0 when
    /// negative, at most [`GIL_LIMIT`].
    pub fn ms_set_gil(mem: &mut impl Mem, n: i32) {
        GIL.set(mem, n.clamp(0, GIL_LIMIT).cast_unsigned());
    }

    /// `MsPayGIL(n)` (`0x007859a0`): pays `n` gil when there is that much
    /// (0), else -1; either way the gil goes through [`ms_set_gil`] (which
    /// clamps what was there before).
    pub fn ms_pay_gil(mem: &mut impl Mem, n: i32) -> i32 {
        let gil = GIL.get(mem).cast_signed();
        let left = gil.wrapping_sub(n);
        if left >= 0 {
            ms_set_gil(mem, left);
            0
        } else {
            ms_set_gil(mem, gil);
            -1
        }
    }

    /// `0x00791170(i, actor, cue)` (name ours): whether `actor`'s `+0x4b6`
    /// is set; the driver asks it for actor 5 only.
    pub fn ms_check_chr_hold(mem: &impl Mem, actor: u32) -> u32 {
        u32::from(byte(mem, actor, 0x4b6) != 0)
    }

    /// `0x00792880(bits)` (name ours): how many of the actors in `bits`
    /// have both `+0x40b` and `+0x40a` set.
    pub fn ms_count_chr_ready(mem: &impl Mem, bits: u32) -> u32 {
        let mut n = 0;
        for i in (0..0x1f).filter(|&i| bits >> i & 1 != 0) {
            let a = ms_get_chr(mem, i);
            if byte(mem, a, 0x40b) != 0 && byte(mem, a, 0x40a) != 0 {
                n += 1;
            }
        }
        n
    }

    /// `MsSystemMessageCheck()` (`0x0079d050`): 0 when message window 7 is
    /// closed (`TOGetMessageStatus`) or is done (status 3) while its window
    /// is shown (`TOGetWindowStatus` 2), which closes it
    /// (`TOCloseMessageWindow`); -1 while it is up.
    pub fn ms_system_message_check<M: Mem>(mem: &mut M, call: Call<'_, M>) -> i32 {
        let f = code(mem, 0x008b_7ca0);
        let s = call(mem, f, &[7]);
        if s == 0 {
            return 0;
        }
        let f = code(mem, 0x0090_6c20);
        let w = call(mem, f, &[7]);
        if s == 3 && w == 2 {
            let f = code(mem, 0x008a_b8f0);
            call(mem, f, &[7]);
            return 0;
        }
        -1
    }

    /// `MsCheckRikkuLimit(i, actor, cue)` (`0x007afbd0`): when the cue's
    /// command (`MsGetCommand` on its entry `+2`) is Mix (0x311e): the
    /// actor's end direction set (`msSetEndDir`), the cue's `+4` 4, the
    /// actor's `+0xdfd` and `+0x5c0` 1, and the Mix table read: the CD
    /// library's slot 4 with 0xd, slot 5 with 0x20, then slot 0 (the read
    /// queue) for file 0x20 into `g_MixTable`, [`MIX_LOADED`] when done,
    /// with a block whose `+0xc` is `i` (`call` gets `[0x20, table, done,
    /// i]` for it; the original's other three block words are its stack
    /// garbage). 1, else 0.
    pub fn ms_check_rikku_limit<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        i: u32,
        actor: u32,
        cue: u32,
    ) -> u32 {
        let list = cue.wrapping_add(u32::from(byte(mem, cue, 2)) * 0x10 + 8);
        let (_, id) = crate::command::bridge::get_command(mem, u32::MAX, list, 0xff);
        if id != 0x311e {
            return 0;
        }
        crate::actor::bridge::ms_set_end_dir(mem, actor);
        mem.write(cue.wrapping_add(4), 4u8);
        mem.write(actor.wrapping_add(0xdfd), 1u8);
        mem.write(actor.wrapping_add(0x5c0), 1u8);
        let lib = CD_LIB.ptr(mem).addr;
        let f = mem.read::<u32>(lib.wrapping_add(0x10));
        call(mem, f, &[0xd]);
        let f = mem.read::<u32>(lib.wrapping_add(0x14));
        call(mem, f, &[0x20]);
        let f = mem.read::<u32>(lib);
        let table = crate::target::bridge::MIX_TABLE.ptr(mem).addr;
        let done = code(mem, MIX_LOADED);
        call(mem, f, &[0x20, table, done, i]);
        1
    }

    /// `0x007995c0(i, bits)` (name ours): whether `i` is on the same side
    /// (`MsGetRamChrMonster`) as the first actor in `bits`; 1 when `bits`
    /// has none.
    #[must_use]
    pub fn ms_check_same_side(i: u32, bits: u32) -> u32 {
        match (0..0x1f).find(|&j| bits >> j & 1 != 0) {
            None => 1,
            Some(j) => u32::from(ms_get_ram_chr_monster(i) == ms_get_ram_chr_monster(j)),
        }
    }

    /// `0x00799610(bits, command)` (name ours): 1 when some actor in `bits`
    /// has gone (not `+0xdc8` and `+0xdd9`, or KO'd when the command's
    /// `+0x1a` bit 6 is clear), so its targets have to be chosen again;
    /// 1 too when `bits` is empty.
    pub fn ms_check_targets_gone(mem: &impl Mem, bits: u32, command: u32) -> u32 {
        let ko_ok = mem.read::<u16>(command.wrapping_add(0x1a)) >> 6 & 1 != 0;
        let (mut n, mut ok) = (0, 0);
        for i in (0..0x1f).filter(|&i| bits >> i & 1 != 0) {
            n += 1;
            let a = ms_get_chr(mem, i);
            if byte(mem, a, 0xdc8) != 0
                && byte(mem, a, 0xdd9) != 0
                && (ms_get_chr_stat_death(mem, i) == 0 || ko_ok)
            {
                ok += 1;
            }
        }
        u32::from(n == 0 || ok < n)
    }

    /// `MsSetChrMimic(i, actor, cue)` (`0x00799b40`): for a party member's
    /// (`i` < 8) first-entry cue (`+1`, `+2` 0). A cue of Mimic (0x3028 in
    /// either ID) becomes the saved [`MIMIC`] cue for `i` (its targets
    /// chosen again where they have gone, `MsGetCommandTarget` on the
    /// side of the old ones), its items used (`MsUseItem`), the actor's
    /// `+0x6de` set and the gil it throws capped by what there is: 1 when
    /// items were used or it has none (`+3` 0), else 0. A cue that can be
    /// mimicked (a command's `MsGetComData` `+0x1c` bit 28) is saved:
    /// 0. Either way a Mimic saves it again, with its gil.
    pub fn ms_set_chr_mimic<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        i: u32,
        actor: u32,
        cue: u32,
    ) -> u32 {
        if byte(mem, cue, 1) != 0 || byte(mem, cue, 2) != 0 || i.cast_signed() >= 8 {
            return 0;
        }
        let mut save = 0i32;
        let mut mimic = false;
        for k in 0..2 {
            let id = u32::from(mem.read::<u16>(cue.wrapping_add(8 + 2 * k)));
            if id == 0x3028 {
                mimic = true;
                break;
            }
            if id != 0xff {
                let d = crate::stat::bridge::ms_get_com_data(mem, id, 0);
                save |= (mem.read::<u32>(d.wrapping_add(0x1c)) >> 28 & 1).cast_signed();
            }
        }
        let r = if mimic {
            save = -1;
            replay(mem, trig, i, actor, cue)
        } else {
            0
        };
        if save != 0 {
            let m = MIMIC.ptr(mem).addr;
            let mut v = [0u8; 0x48];
            mem.read_bytes(cue, &mut v);
            mem.write_bytes(m, &v);
            let g = crate::damage::bridge::GIL_AMOUNT.get(mem);
            MIMIC_GIL.set(mem, g);
        }
        r
    }

    /// The Mimic cue replayed for `i` ([`ms_set_chr_mimic`]).
    fn replay<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        i: u32,
        actor: u32,
        cue: u32,
    ) -> u32 {
        let m = MIMIC.ptr(mem).addr;
        let mut v = [0u8; 0x48];
        mem.read_bytes(m, &mut v);
        mem.write_bytes(cue, &v);
        #[allow(clippy::cast_possible_truncation)] // i < 8
        mem.write(cue, i as u8);
        let none = byte(mem, cue, 3) == 0;
        if crate::turn::bridge::ms_use_item(mem, i, cue, -1) != 0 || none {
            return 1;
        }
        for k in 0..u32::from(byte(mem, cue, 3)) {
            let at = cue.wrapping_add(0x10 + 0x10 * k);
            let mut t: u32 = mem.read(at);
            let (c, id) =
                crate::command::bridge::get_command(mem, u32::MAX, at.wrapping_sub(8), 0xff);
            if byte(mem, c, 0x1a) & 0xc == 0xc {
                t = 1u32.wrapping_shl(i);
            }
            if ms_check_targets_gone(mem, t, c) != 0 {
                let side = ms_check_same_side(i, t).cast_signed();
                t = crate::stat::bridge::ms_get_command_target(mem, trig, i, id, side, 1);
            }
            mem.write(at, t);
        }
        mem.write(actor.wrapping_add(0x6de), 1u8);
        let gil = crate::save::bridge::ms_get_gil(mem).cast_signed();
        let g = MIMIC_GIL.get(mem).min(gil);
        crate::damage::bridge::GIL_AMOUNT.set(mem, g);
        0
    }

    /// `0x007acb00(kind)` (name ours): a new action request of `kind` in
    /// [`ACTION_REQUEST_BLOCK`] (cleared) when [`crate::misc::bridge::ACTION`]
    /// is not -1: 0, else -1.
    pub fn ms_action_request_set(mem: &mut impl Mem, kind: u8) -> i32 {
        if crate::misc::bridge::ACTION.get(mem) == -1 {
            return -1;
        }
        let p = ACTION_REQUEST_BLOCK.ptr(mem).addr;
        crate::util::bridge::ms_struct_clear(mem, p, 0x48);
        mem.write(p, kind);
        ACTION_REQUEST_STEP.set(mem, 0);
        ACTION_REQUEST.set(mem, 0);
        0
    }

    /// `0x007af800()` (name ours): when a guard took a hit
    /// ([`crate::stat::bridge::GUARD_TOOK`]), every actor with a defense
    /// position (`+0x6d4` not 0xff) leaves it (`MsDefensePosOff`) and the
    /// flag is cleared.
    pub fn ms_defense_pos_off_all(mem: &mut impl Mem) {
        let took = crate::stat::bridge::GUARD_TOOK;
        if took.get(mem) == 0 {
            return;
        }
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            if byte(mem, a, 0x6d4) != 0xff {
                crate::minor::bridge::ms_defense_pos_off(mem, a);
            }
        }
        took.set(mem, 0);
    }

    /// `0x007afb60(i, actor)` (name ours): after an action, an actor with
    /// `+0x714` set gains its overdrive (`+0x5bc`, up to `+0x5bd`) and
    /// `+0x6e1` (up to 100) by the signed bytes `+0x728` and `+0x726`
    /// (or the next ones when `+0x715` bit 0 is set).
    pub fn ms_add_chr_gain(mem: &mut impl Mem, actor: u32) {
        use ffx_kernel::bridge::ms_check_range;
        if byte(mem, actor, 0x714) == 0 {
            return;
        }
        let s = u32::from(byte(mem, actor, 0x715) & 1);
        let g = i32::from(mem.read::<i8>(actor.wrapping_add(0x728 + s)))
            + i32::from(byte(mem, actor, 0x5bc));
        let v = ms_check_range(g, 0, i32::from(byte(mem, actor, 0x5bd)));
        mem.write(actor.wrapping_add(0x5bc), v.to_le_bytes()[0]);
        let g = i32::from(mem.read::<i8>(actor.wrapping_add(0x726 + s)))
            + i32::from(byte(mem, actor, 0x6e1));
        let v = ms_check_range(g, 0, 100);
        mem.write(actor.wrapping_add(0x6e1), v.to_le_bytes()[0]);
    }

    /// `0x007ae0f0(ids)` (name ours): whether either of the two `u16` IDs
    /// at `ids` (not null) is 0x3029.
    pub fn ms_check_command_3029(mem: &impl Mem, ids: u32) -> u32 {
        u32::from(ids != 0 && (0..2).any(|k| mem.read::<u16>(ids.wrapping_add(2 * k)) == 0x3029))
    }

    /// `0x007ae120(kind, command)` (name ours): for kinds 1 and 5, 0 when
    /// the command's `+0x1c` has bit 20, 2 when bit 21, else 1; 1 for the
    /// other kinds.
    pub fn ms_command_kind_mode(mem: &impl Mem, kind: u32, command: u32) -> u32 {
        if kind == 1 || kind == 5 {
            let f: u32 = mem.read(command.wrapping_add(0x1c));
            if f & 0x10_0000 != 0 {
                return 0;
            }
            if f & 0x20_0000 != 0 {
                return 2;
            }
        }
        1
    }

    /// `0x007ae2e0(i, after, id)` (name ours): `i` gets command `id` next,
    /// on the script's targets ([`SCRIPT_TARGETS`]): `+0xdf2` cleared, an
    /// input cue entry after `after` (`MsInsertInputCue(i, 0, 0, 0, after,
    /// 1)`) and a command cue of one entry (`+2` 0x100, `+8` the ID, `+0x10`
    /// the targets; `MsInsertCommandCue(i, cue, 8, 0, 0, 0, after)`). The
    /// targets.
    pub fn ms_insert_script_command(mem: &mut impl Mem, i: u32, after: u32, id: u16) -> u32 {
        let a = ms_get_chr(mem, i);
        mem.write(a.wrapping_add(0xdf2), 0u8);
        let targets = SCRIPT_TARGETS.get(mem);
        crate::cue::bridge::ms_insert_input_cue(mem, i, 0, 0, 0, after, 1);
        {
            let mut f = Frame::new(mem, SCRIPT_COMMAND_FRAME, 0x48);
            let c = f.at(0);
            f.write(c.wrapping_add(2), 0x100u16);
            f.write(c.wrapping_add(8), [id, 0]);
            f.write(c.wrapping_add(0x10), targets);
            crate::cue::bridge::ms_insert_command_cue(&mut f, i, c, 8, 0, 0, 0, after);
        }
        SCRIPT_TARGETS.get(mem)
    }
}
