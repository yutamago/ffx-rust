//! The battle's action requests and the cue entries they make
//! (`docs/rewrite-plan.md`, M2): `MsActionRequest` runs an actor's
//! action script (its monster file's handler 0x3d, or the battle's
//! handler for party members) and turns the command the script left in the
//! request block into a cue entry (`MsSetBtlCommand`,
//! `MsInsertBtlCommand`); `MsDeleteInputCue` ends a turn's input entry
//! and with it the actor's turn (statuses, CTB, the next action request).
//!
//! These call each other (`MsDeleteInputCue` asks `MsActionRequest`, which
//! may insert a command and delete the input entry again), so they share
//! [`Dm`], [`Host`] and [`Script`].

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, cues
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::actor::bridge::{ms_btl_chr_num_check, ms_get_chr};
    use crate::atel::bridge::{Script, ms_atel_parse_sys_tag, ms_atel_request_exe};
    use crate::exec::bridge::{ACTION_REQUEST, ACTION_REQUEST_BLOCK, ACTION_REQUEST_STEP};
    use crate::state::bridge::{
        INPUT_CUE, INPUT_CUE_COUNT, ms_atel_request_add, ms_atel_request_init,
    };
    use crate::target::bridge::{Dm, Host};

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn half(v: u32) -> u16 {
        u16::from_le_bytes([v.to_le_bytes()[0], v.to_le_bytes()[1]])
    }

    /// Cleared by `MsSetBtlCommand`.
    pub const SET_COMMAND_FLAG: Global<u8> = Global::new(0x0112_c9f7);
    /// The actor whose turn ended with a status `+0x606` bit 3 left
    /// (`MsDeleteInputCue` with `last`).
    pub const TURN_END_ACTOR: Global<u8> = Global::new(0x0112_c9e4);
    /// The request block's entry count (`+3`) and turn byte (`+4`).
    pub const REQUEST_ENTRIES: Global<u8> = Global::new(0x0113_6a7b);
    pub const REQUEST_TURN: Global<u8> = Global::new(0x0113_6a7c);

    /// `MsSetBtlCommand(cue, gil, flag)` (`0x00792d60`): [`SET_COMMAND_FLAG`]
    /// cleared; for an actor's cue (`+0`) with `flag` 0 or 1, the gil it
    /// throws set (`g_BtlGilAmount`) and the cue appended to the command
    /// cue (`MsRegistCommandCue(chr, cue, 0, 0, flag)`): 0; when that is
    /// full the actor's input entry (`+0xde4`) goes ([`ms_delete_input_cue`])
    /// and -1, as for the rest.
    pub fn ms_set_btl_command<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        cue: u32,
        gil: i32,
        flag: u32,
    ) -> i32 {
        SET_COMMAND_FLAG.set(mem, 0);
        if cue == 0 {
            return -1;
        }
        let chr = u32::from(byte(mem, cue, 0));
        let a = ms_get_chr(mem, chr);
        crate::damage::bridge::GIL_AMOUNT.set(mem, gil);
        if ms_btl_chr_num_check(chr) == 0 || flag > 1 {
            return -1;
        }
        if crate::cue::bridge::ms_regist_command_cue(mem, chr, cue, 0, 0, flag.to_le_bytes()[0])
            != 0
        {
            return 0;
        }
        let k = u32::from(byte(mem, a, 0xde4));
        ms_delete_input_cue(mem, host, s, chr, k, 0, 0);
        -1
    }

    /// `MsInsertBtlCommand(cue, prio, flag, after)` (`0x007929b0`): for an
    /// actor's cue with `flag` 0 or 1, an input entry
    /// (`MsInsertInputCue(chr, prio, 0, flag, after, 1)`) and the command
    /// (`MsInsertCommandCue(chr, cue, 0, prio, flag, 0, after)`) next to
    /// `after`'s: 0; the input entry goes again ([`ms_delete_input_cue`])
    /// when the command cue is full, and -1, as for the rest.
    pub fn ms_insert_btl_command<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        cue: u32,
        prio: u32,
        flag: u32,
        after: u32,
    ) -> i32 {
        if cue == 0 {
            return -1;
        }
        let chr = u32::from(byte(mem, cue, 0));
        let a = ms_get_chr(mem, chr);
        if ms_btl_chr_num_check(chr) == 0 || flag > 1 {
            return -1;
        }
        let f = flag.to_le_bytes()[0];
        if crate::cue::bridge::ms_insert_input_cue(mem, chr, prio, 0, flag, after, 1) == 0 {
            return -1;
        }
        if crate::cue::bridge::ms_insert_command_cue(mem, chr, cue, 0, prio, f, 0, after) != 0 {
            return 0;
        }
        let k = u32::from(byte(mem, a, 0xde4));
        ms_delete_input_cue(mem, host, s, chr, k, 0, 0);
        -1
    }

    /// `MsActionRequest(i, who, n, special, mode, out)` (`0x007aceb0`):
    /// actor `i & 0xff`'s action script for `n` (`who` into `+0xded`
    /// unless 0xff): [`crate::misc::bridge::ACTION`] is `mode` meanwhile and
    /// the request block is new (`MsActionRequestSet(i)`). A monster runs
    /// its file's handler 0x3d; a party member the battle's handler
    /// `i + 0x29` of record 1 (with `special`, actors 0xe-0x11 their
    /// handlers 0x25-0x28 of record 1, then 0), each [`ms_atel_request_exe`]
    /// with `k` `n`, then its tags ([`ms_atel_parse_sys_tag`]). A command
    /// the script left (block `+3` entries) for `mode` 0 or 1 goes to
    /// `out` (the block, its turn byte `+4` from `out`'s) or, without
    /// `out`, to the command cue: mode 0 [`ms_set_btl_command`], mode 1
    /// [`ms_insert_btl_command`] (after `who`, unless that actor already
    /// has its entry, `+0xdf1`, or a command of `i` is pending:
    /// `MsCheckCommandCue`). `n` up to 8 is also a voice
    /// ([`crate::atel::bridge::ms_set_voice_check`]). The script's result.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_action_request<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        who: u32,
        n: i32,
        special: u32,
        mode: u32,
        out: u32,
    ) -> i32 {
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        if who != 0xff {
            mem.write(a.wrapping_add(0xded), who.to_le_bytes()[0]);
        }
        crate::misc::bridge::ACTION.set(mem, mode.cast_signed());
        crate::exec::bridge::ms_action_request_set(mem, i.to_le_bytes()[0]);
        let k = half(n.cast_unsigned());
        let r = if crate::actor::bridge::ms_get_ram_chr_monster(i) != 0 {
            ms_atel_request_init(mem);
            ms_atel_request_add(mem, [3, half(i), 0x3d, k]);
            let r = ms_atel_request_exe(mem, s, 3, i, 0x3d, 1, 0, 0);
            if r >= 0 {
                ms_atel_parse_sys_tag(mem, s, 3, i, 0x3d);
            }
            r
        } else {
            let h = if special != 0 && (0xe..=0x11).contains(&i) {
                Some(i + 0x17)
            } else {
                None
            };
            ms_atel_request_init(mem);
            match h {
                None => {
                    let h = i + 0x29;
                    ms_atel_request_add(mem, [2, 1, half(h), k]);
                    let r = ms_atel_request_exe(mem, s, 2, 1, h.cast_signed(), 1, 0, 0);
                    if r >= 0 {
                        ms_atel_parse_sys_tag(mem, s, 2, 1, h.cast_signed());
                    }
                    r
                }
                Some(h) => {
                    ms_atel_request_add(mem, [2, 1, half(h), k]);
                    ms_atel_request_add(mem, [2, 0, half(h), k]);
                    let r = ms_atel_request_exe(mem, s, 2, 0, h.cast_signed(), 1, 0, 0);
                    if r >= 0 {
                        ms_atel_parse_sys_tag(mem, s, 2, 0, h.cast_signed());
                    }
                    r
                }
            }
        };
        if REQUEST_ENTRIES.get(mem) != 0 && mode <= 1 {
            let block = ACTION_REQUEST_BLOCK.ptr(mem).addr;
            if out != 0 {
                let t = byte(mem, out, 4);
                REQUEST_TURN.set(mem, t);
                let mut v = [0u8; 0x48];
                mem.read_bytes(block, &mut v);
                mem.write_bytes(out, &v);
            } else if mode == 0 {
                ms_set_btl_command(mem, host, s, block, 0, 0);
            } else {
                let insert = if who == 0xff || ACTION_REQUEST.get(mem) != 0 {
                    true
                } else {
                    let w = ms_get_chr(mem, who);
                    w == 0
                        || (byte(mem, w, 0xdf1) == 0
                            && crate::state::bridge::ms_check_command_cue(mem, i, 1) != 0)
                };
                if insert {
                    let step = ACTION_REQUEST_STEP.get(mem);
                    ms_insert_btl_command(mem, host, s, block, step, mode, who);
                }
            }
        }
        if n <= 8 {
            crate::atel::bridge::ms_set_voice_check(mem, s, i, n, 1, 0, 0);
        }
        crate::misc::bridge::ACTION.set(mem, -1);
        r
    }

    /// `MsActionTarget(bits, who, _, _, n, mode)` (`0x007ad0c0`): every
    /// actor in `bits` remembers `who` (`+0xded`) and, unless `mode` is 1
    /// and it has a command pending (`MsCheckCommandCue`), requests action
    /// `n` ([`ms_action_request`] with `mode`); how many got 0.
    pub fn ms_action_target<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        bits: u32,
        who: u32,
        n: i32,
        mode: u32,
    ) -> u32 {
        let mut count = 0;
        for i in 0..0x1f {
            if bits.cast_signed() >> i & 1 == 0 {
                continue;
            }
            let a = ms_get_chr(mem, i);
            mem.write(a.wrapping_add(0xded), who.to_le_bytes()[0]);
            if mode == 1 && crate::state::bridge::ms_check_command_cue(mem, i, 1) == 0 {
                continue;
            }
            if ms_action_request(mem, host, s, i, who, n, 0, mode, 0) == 0 {
                count += 1;
            }
        }
        count
    }

    /// `MsCheckSummon(i, actor, x, cue)` (`0x007b2980`): without `x`, an
    /// aeon's summon cue: for actor 0xe, command 0x3057 requests action 0xa
    /// (mode 2), 0x3123 action 9 into the cue (`+0xdf4` set); for actors
    /// 0xf-0x11, commands 0x3124-0x312b actions 0xd-0x14 into the cue
    /// (`+0xdf4` set). 0 when one was requested, else -1.
    pub fn ms_check_summon<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
        x: u32,
        cue: u32,
    ) -> i32 {
        if x != 0 {
            return -1;
        }
        let ids = cue.wrapping_add(u32::from(byte(mem, cue, 2)) * 0x10 + 8);
        if i == 0xe {
            for k in 0..2 {
                match mem.read::<u16>(ids.wrapping_add(2 * k)) {
                    0x3057 => {
                        ms_action_request(mem, host, s, 0xe, 0xff, 0xa, 1, 2, 0);
                        return 0;
                    }
                    0x3123 => {
                        mem.write(actor.wrapping_add(0xdf4), 1u8);
                        ms_action_request(mem, host, s, 0xe, 0xff, 9, 1, 0, cue);
                        return 0;
                    }
                    _ => {}
                }
            }
            return -1;
        }
        if !(0xf..=0x11).contains(&i) {
            return -1;
        }
        for k in 0..2 {
            let id = u32::from(mem.read::<u16>(ids.wrapping_add(2 * k)));
            let d = id.wrapping_sub(0x3124);
            if d <= 7 {
                mem.write(actor.wrapping_add(0xdf4), 1u8);
                ms_action_request(mem, host, s, i, 0xff, (d + 0xd).cast_signed(), 1, 0, cue);
                return 0;
            }
        }
        -1
    }

    /// `MsDeleteInputCue(chr, k, last, keep)` (`0x007b20e0`): input entry
    /// `k` when it is `chr`'s (its own entry `+0xde4` stays with `keep`,
    /// else is cleared) leaves the cue, the rest moving up, and ends a
    /// turn: the gil thrown cleared, `+0xde6` counted down, the defense
    /// positions off, the statuses after input, the overdrive gains; for a
    /// first entry (`+1` 0) the next CTB (`MsGetNextCTB(actor, +0xde8,
    /// +0x613, +0x614)` onto `+0x65c`), the statuses after the action, the
    /// MP limit; with `last` an able actor with `+0x606` bit 3 (and no
    /// `+0xdf8`) becomes [`TURN_END_ACTOR`]. Then its action 6 is
    /// requested ([`ms_action_request`]). How many entries went.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_delete_input_cue<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        chr: u32,
        k: u32,
        last: u32,
        keep: u32,
    ) -> u32 {
        let n = i32::from(INPUT_CUE_COUNT.get(mem));
        if n <= 0 || k == 0xff {
            return 0;
        }
        let a = ms_get_chr(mem, chr);
        let base = INPUT_CUE.ptr(mem).addr;
        let (mut removed, mut dst, mut count) = (false, 0u32, 0u32);
        for e in 0..n.cast_unsigned() {
            let entry = base + 8 * e;
            let mine = e == k && u32::from(byte(mem, entry, 0)) == chr;
            let own = u32::from(byte(mem, a, 0xde4));
            if mine && !(e == own && keep != 0) {
                if e == own {
                    mem.write(a.wrapping_add(0xde4), 0xffu8);
                }
                crate::damage::bridge::GIL_AMOUNT.set(mem, 0);
                let c = byte(mem, a, 0xde6).wrapping_sub(1);
                mem.write(a.wrapping_add(0xde6), c);
                removed = true;
                crate::exec::bridge::ms_defense_pos_off_all(mem);
                crate::status::bridge::ms_check_status_after_input(mem, &mut *s.call, &mut *s.off);
                crate::exec::bridge::ms_add_chr_gain(mem, a);
                if byte(mem, entry, 1) == 0 {
                    let ctb = crate::damage::bridge::ms_get_next_ctb(
                        mem,
                        a,
                        i32::from(byte(mem, a, 0xde8)),
                        i32::from(byte(mem, a, 0x613)),
                        i32::from(byte(mem, a, 0x614)),
                    );
                    let v = byte(mem, a, 0x65c).wrapping_add(ctb.to_le_bytes()[0]);
                    mem.write(a.wrapping_add(0x65c), v);
                    crate::status::bridge::ms_check_status_after_action(mem, host, s, chr, a);
                    crate::flow::bridge::ms_use_chr_mp_limit(mem, chr);
                    if last != 0
                        && mem.read::<u16>(a.wrapping_add(0x606)) & 8 != 0
                        && byte(mem, a, 0xdcc) == 0
                        && byte(mem, a, 0xdc8) != 0
                        && byte(mem, a, 0xdf8) == 0
                    {
                        TURN_END_ACTOR.set(mem, chr.to_le_bytes()[0]);
                    }
                }
                count += 1;
            } else {
                if e != dst {
                    let v: [u32; 2] = mem.read(entry);
                    mem.write(base + 8 * dst, v);
                }
                dst += 1;
            }
        }
        if count > 0 {
            let left = (n.cast_unsigned()).wrapping_sub(count).to_le_bytes()[0];
            INPUT_CUE_COUNT.set(mem, left.cast_signed());
        }
        if removed {
            ms_action_request(mem, host, s, chr, 0xff, 6, 0, 0, 0);
        }
        count
    }

    /// Flags that make an aeon's first command fall back to its plain one
    /// (`MsInsertChrCommand` kind 0).
    pub const AEON_PLAIN: Global<u8> = Global::new(0x0112_a904);
    pub const AEON_PLAIN2: Global<u8> = Global::new(0x0112_a900);
    /// The frame `MsInsertChrCommand` builds its cue in.
    pub const CHR_COMMAND_FRAME: u32 = 0xff10_0000;

    /// `MsCleanInputCue(chr, keep)` (`0x007b20b0`): every input entry,
    /// last first, given to [`ms_delete_input_cue`] for `chr`; how many
    /// went.
    pub fn ms_clean_input_cue<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        chr: u32,
        keep: u32,
    ) -> u32 {
        let mut n = 0u32;
        let mut e = i32::from(INPUT_CUE_COUNT.get(mem)) - 1;
        while e >= 0 {
            n = n.wrapping_add(ms_delete_input_cue(
                mem,
                host,
                s,
                chr,
                e.cast_unsigned(),
                0,
                keep,
            ));
            e -= 1;
        }
        n
    }

    /// `MsInsertChrCommand(i, target, kind)` (`0x007ae1a0`): aeon `i`'s
    /// command 0x3100 on `target` next: its value
    /// ([`crate::exec::bridge::ms_get_aeon_command`] for `kind`, kind 0
    /// with [`AEON_PLAIN`] or [`AEON_PLAIN2`] or none: 0 with `+0x1e` set)
    /// written into the kernel's command 0x3100 (`+0x10`, `+0x12`; `+0x1a`
    /// bit 6, `+0x1c` bit 27 and bit 30 from the pair's byte), its kind
    /// (`+0x15`: 0x11 for kind 0, after the party's exchange
    /// `MsExchangeParty`, 2 for 1, 0x16 for 2), and an input entry and a
    /// one-entry command cue after `target` (priority 0x40 for kind 2 with
    /// `+0x3f9`). The command's value.
    pub fn ms_insert_chr_command(mem: &mut impl Mem, i: u32, target: u32, kind: u32) -> u32 {
        let a = ms_get_chr(mem, i);
        let mut f = ffx_bridge::frame::Frame::new(mem, CHR_COMMAND_FRAME, 0x50);
        let (cue, mode_at, val_at) = (f.at(0), f.at(0x48), f.at(0x4c));
        let mut v =
            crate::exec::bridge::ms_get_aeon_command(&mut f, i, 0x3100, kind, mode_at, val_at);
        if kind == 0 && (AEON_PLAIN.get(&f) != 0 || AEON_PLAIN2.get(&f) != 0 || v == 0) {
            v = 0;
            f.write(a.wrapping_add(0x1e), 1u8);
        }
        f.write(a.wrapping_add(0xdf2), 0u8);
        f.write(cue + 2, 0x100u16);
        f.write(cue + 8, 0x3100u32);
        f.write(cue + 0x10, 1u32.wrapping_shl(target));
        let rec = ffx_kernel::bridge::rom(&mut f, ffx_kernel::bridge::COMMAND, 0x3100, 0);
        let val: u32 = f.read(val_at);
        let flags: u32 = f.read(rec + 0x1c);
        let w: u16 = f.read(rec + 0x1a);
        f.write(rec + 0x1a, w | 0x40);
        let flags = ((flags & !0x4000_0000) | ((val << 30) & 0x4000_0000)) | 0x800_0000;
        f.write(rec + 0x1c, flags);
        let h = v.to_le_bytes();
        f.write(rec + 0x10, u16::from_le_bytes([h[0], h[1]]));
        f.write(rec + 0x12, u16::from_le_bytes([h[0], h[1]]));
        let mut prio = 0u32;
        match kind {
            1 => f.write(rec + 0x15, 2u8),
            2 => {
                f.write(rec + 0x15, 0x16u8);
                if byte(&f, a, 0x3f9) != 0 {
                    prio = 0x40;
                }
            }
            _ => {
                if kind == 0 {
                    crate::flow::bridge::ms_exchange_party(&mut f);
                }
                f.write(rec + 0x15, 0x11u8);
            }
        }
        crate::cue::bridge::ms_insert_input_cue(&mut f, i, prio, 0, 1, target, 1);
        let mode: u32 = f.read(mode_at);
        crate::cue::bridge::ms_insert_command_cue(
            &mut f,
            i,
            cue,
            mode.to_le_bytes()[0],
            prio,
            1,
            0,
            target,
        );
        v
    }

    /// `dbgPrintf` and [`battle_queue_command`]'s message (no target).
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const TARGET_ERROR_TEXT: u32 = 0x00b5_4f94;
    /// The action request's entries: 16 bytes each from `0x01136a80` (the
    /// command and a 0 `u16` after it, `+8` the targets).
    pub const REQUEST_LIST: u32 = 0x0113_6a80;

    /// `Battle_QueueCommand(i, command, targets, request, step)`
    /// (`0x007ac9c0`, natives `0x700b`, `0x705a`, `0x7117`): while an
    /// action runs ([`crate::misc::bridge::ACTION`] not -1) for actor `i`
    /// (the request block's byte 0) with fewer than 4 entries
    /// ([`REQUEST_ENTRIES`]), the next entry gets `command` (and a 0
    /// after it) and, when it is a command (`MsGetCommand(i, 0, -1, entry,
    /// 0)`), its targets: those of the 31 actors in the mask `targets`
    /// that can be: alive (`+0xdcc` 0) and there (`+0xdc8`), or dead with a
    /// command for the dead (`+0x1a` bit 6). With some, the actor's script
    /// targets (`MsSetChrScriptTargets_00796430`), the entries counted up,
    /// the request word `request` and 0; a non-negative `step` goes to the
    /// request's second word first. Without, -1 (printed when some actor
    /// of the mask could not be: the mask of those and the last).
    pub fn battle_queue_command<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        [i, command, targets, request, step]: [u32; 5],
    ) -> u32 {
        use crate::exec::bridge::{ACTION_REQUEST, ACTION_REQUEST_BLOCK, ACTION_REQUEST_STEP};
        if crate::misc::bridge::ACTION.get(mem) == -1
            || i != u32::from(ACTION_REQUEST_BLOCK.get(mem))
            || REQUEST_ENTRIES.get(mem) >= 4
        {
            return u32::MAX;
        }
        let n = u32::from(REQUEST_ENTRIES.get(mem));
        let entry = mem.rebase(REQUEST_LIST) + 0x10 * n;
        #[allow(clippy::cast_possible_truncation)] // the entry's u16
        mem.write(entry, command as u16);
        mem.write(entry + 2, 0u16);
        let c = crate::command::bridge::ms_get_command(mem, u32::MAX, entry, 0);
        if c == 0 {
            return u32::MAX;
        }
        if step.cast_signed() >= 0 {
            ACTION_REQUEST_STEP.set(mem, step);
        }
        let for_dead = mem.read::<u16>(c + 0x1a) >> 6 & 1 != 0;
        let (mut ok, mut bad, mut last) = (0u32, 0u32, 0xffu32);
        for k in 0..0x1f {
            let a = crate::actor::bridge::ms_get_chr(mem, k);
            if (targets.cast_signed() >> k) & 1 == 0 {
                continue;
            }
            let dead = mem.read::<u8>(a + 0xdcc) != 0;
            let can = if dead {
                for_dead
            } else {
                mem.read::<u8>(a + 0xdc8) != 0
            };
            if can {
                ok |= 1 << k;
            } else {
                bad |= 1 << k;
                last = k;
            }
        }
        mem.write(entry + 8, ok);
        if ok != 0 {
            crate::minor::bridge::ms_set_chr_script_targets(mem, i, ok);
            REQUEST_ENTRIES.set(mem, REQUEST_ENTRIES.get(mem).wrapping_add(1));
            ACTION_REQUEST.set(mem, request);
            return 0;
        }
        if bad != 0 {
            let f = mem.rebase(DBG_PRINTF);
            let fmt = mem.rebase(TARGET_ERROR_TEXT);
            call(mem, f, &[fmt, i, bad, last]);
        }
        u32::MAX
    }
}
