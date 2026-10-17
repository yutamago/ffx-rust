//! An actor's turn starting (`docs/rewrite-plan.md`, M2):
//! `MsBattleProcess_InputCue` takes the input cue's first entry each frame,
//! runs what happens before the actor acts (`MsCheckStatusBeforeAction`:
//! the counters, the status counts, the statuses that end) and either
//! opens its menu (`TOSetBtlActiveChr`, the menu's) or picks its command
//! itself: Doom (`MsSetDeathSentence`), Provoke (`MsSetProvokingCommand`),
//! Berserk and Confuse (`MsSetBerserkCommand`) or an automatic actor's
//! plain turn. Also the damage numbers (`MsNumberRegist`) and HP loss
//! (`MsSubHP`) those use.
//!
//! The menu is another module's: it goes to the game's code through
//! `s.call`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, the script calls
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::{Script, ms_set_motion};
    use crate::exec::bridge::Call;
    use crate::target::bridge::{Dm, Host};

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn half(mem: &impl Mem, p: u32, o: u32) -> u16 {
        mem.read(p.wrapping_add(o))
    }

    fn word(mem: &impl Mem, p: u32, o: u32) -> i32 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    /// `dbgPrintf` (system).
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// `TOSetBtlActiveChr` (menu): the actor's battle menu opens.
    pub const TO_SET_BTL_ACTIVE_CHR: u32 = 0x0089_bb10;
    /// "battle internal error : %s\n" (`MsBattleError`).
    pub const BATTLE_ERROR_FORMAT: u32 = 0x00b5_4d44;
    /// `MsBattleError("send_menu")`'s message.
    pub const SEND_MENU_ERROR: u32 = 0x00b5_4b9c;

    /// The digits of the last number `MsNumberRegist` split (lowest first,
    /// up to 9).
    pub const NUMBER_DIGITS: Global<[u8; 9]> = Global::new(0x0113_60ac);
    /// Set: every HP loss takes 99999 more (a debug switch; actors with
    /// `+4` 1-7 are spared).
    pub const DEBUG_KILL: Global<u32> = Global::new(0x0113_338c);
    /// Set in battle 0x159/0 by actor 0x14's HP loss, cleared by actor
    /// 0x17's: actor 0x15 keeps at least 1 HP meanwhile.
    pub const KEEP_ALIVE: Global<u32> = Global::new(0x00c4_21ec);
    /// The "player powerup" cheat: party members heal fully each turn.
    pub const CHEAT_POWERUP: Global<u32> = Global::new(0x012f_b7cc);
    /// Set: automatic actors (`+0xdf3`) get their menu as the rest do
    /// (battle flag selector 3, `Native_707d`).
    pub const AUTO_OFF: Global<u8> = Global::new(0x0112_a8fa);
    /// A dword each turn's start clears (written by `0x00792de0` too).
    pub const TURN_WORD: Global<u32> = Global::new(0x0112_c8cc);
    /// The voice wait flag (`MsSetVoiceWait`).
    pub const VOICE_WAIT: Global<u32> = Global::new(0x0112_c8f0);

    /// `MsBattleError(msg)` (`0x0079f210`): "battle internal error : msg"
    /// to the debug log (`dbgPrintf`), then the battle hangs for good.
    pub fn ms_battle_error<M: Mem>(mem: &mut M, call: Call<'_, M>, msg: u32) -> ! {
        let f = code(mem, DBG_PRINTF);
        let fmt = code(mem, BATTLE_ERROR_FORMAT);
        call(mem, f, &[fmt, msg]);
        loop {
            std::hint::spin_loop();
        }
    }

    /// `MsSetChrWeak(i, level)` (`0x0078d7f0`): actor `i`'s weak level
    /// (`+0x63e` the HP's, `MsCalcWeakLevel`) and the pose it takes
    /// (`+0x63d`; returned): 0 with `+0x450`; `level` unless -1; else by
    /// the HP: 0 (1 with Poison, Darkness or Silence counts `+0x608`-`+0x60a`)
    /// or 1 as they are, 2 (and 3 when its death motion may not play yet,
    /// [`crate::death::bridge::ms_check_death_motion`], or once KO'd) the
    /// weak pose, which is 1 underwater and, for an aeon, 1 or 0 (0 when
    /// its death motion waits).
    pub fn ms_set_chr_weak(mem: &mut impl Mem, i: u32, level: u32) -> u32 {
        let a = ms_get_chr(mem, i);
        let w = crate::actor::bridge::ms_calc_weak_level(word(mem, a, 0x5d0), word(mem, a, 0x594));
        let r = if byte(mem, a, 0x450) != 0 {
            0
        } else if level != u32::MAX {
            level
        } else {
            let mut waits = false;
            let r = match w {
                0 => u32::from(
                    byte(mem, a, 0x608) != 0
                        || byte(mem, a, 0x60a) != 0
                        || byte(mem, a, 0x609) != 0,
                ),
                3 => {
                    waits = crate::death::bridge::ms_check_death_motion(mem, i, a) == 0;
                    if byte(mem, a, 0xdcc) != 0 && !waits {
                        3
                    } else {
                        2
                    }
                }
                _ => w,
            };
            if r == 2 {
                let mut r = r;
                if crate::state::bridge::UNDERWATER.get(mem) != 0 {
                    r = 1;
                }
                if crate::damage::bridge::ms_get_ram_chr_summon(mem, i) != 0 {
                    r = u32::from(!waits);
                }
                r
            } else {
                r
            }
        };
        mem.write(a.wrapping_add(0x63e), w.to_le_bytes()[0]);
        mem.write(a.wrapping_add(0x63d), r.to_le_bytes()[0]);
        r
    }

    /// `0x0079fc00(n, &digits)` (`MsNumberDigits`, name ours): the decimal
    /// digits of `n` (lowest first, at most 9) into [`NUMBER_DIGITS`],
    /// whose address goes to `digits` unless null; how many (0 for a
    /// negative `n`).
    pub fn ms_number_digits(mem: &mut impl Mem, n: i32, digits: u32) -> i32 {
        let at = NUMBER_DIGITS.ptr(mem).addr;
        let mut k = 0;
        if n >= 0 {
            let mut n = n;
            loop {
                let q = n / 10;
                mem.write(at + k, (n - q * 10).to_le_bytes()[0]);
                k += 1;
                if q == 0 || k >= 9 {
                    break;
                }
                n = q;
            }
        }
        if digits != 0 {
            mem.write(digits, at);
        }
        k.cast_signed()
    }

    /// `0x0079fc60(...)` (`MsNumberPut`, name ours): one character of a
    /// damage number into the first free one of the 0x80 records of 0x14
    /// bytes at `misc::NUMBERS` (`+0` its style, 0 free), linked at the
    /// list's end (`minor::NUMBER_TAIL`): `+1` the colour, `+2` the actor,
    /// `+3` its place (`+5` one more, a delay), `+4` the character, `+7`
    /// its life, `+8`/`+0xa` its x and y, `+0xc`/`+0xd` its size. 1, or 0
    /// with every record taken.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_number_put(
        mem: &mut impl Mem,
        i: u8,
        k: u8,
        ch: u8,
        x: u16,
        y: u16,
        w: u8,
        h: u8,
        life: u8,
        colour: u8,
        style: u8,
    ) -> u32 {
        use crate::minor::bridge::NUMBER_TAIL;
        let pool = crate::misc::bridge::NUMBERS.get(mem);
        for n in 0..0x80 {
            let e = pool.wrapping_add(0x14 * n);
            if byte(mem, e, 0) != 0 {
                continue;
            }
            let tail = NUMBER_TAIL.get(mem);
            mem.write(tail, e);
            mem.write(e, style);
            mem.write(e.wrapping_add(1), colour);
            mem.write(e.wrapping_add(2), i);
            mem.write(e.wrapping_add(4), ch);
            mem.write(e.wrapping_add(7), life);
            mem.write(e.wrapping_add(8), x);
            mem.write(e.wrapping_add(10), y);
            mem.write(e.wrapping_add(3), k);
            mem.write(e.wrapping_add(0xc), w);
            mem.write(e.wrapping_add(5), k.wrapping_add(1));
            mem.write(e.wrapping_add(6), 0u8);
            mem.write(e.wrapping_add(0xd), h);
            NUMBER_TAIL.set(mem, e.wrapping_add(0x10));
            mem.write(e.wrapping_add(0x10), 0u32);
            return 1;
        }
        0
    }

    /// `MsNumberRegist(i, kind, value, other, x, flags, style)`
    /// (`0x0079fa00`): the damage number over actor `i`: `value`'s digits
    /// (its size; negative, or 0 with `other` negative, a heal's colour 1)
    /// and kind's word: 1 "MP" (one line lower with `flags` bit 0), 3 and
    /// 5 only a word (5 in colour 2), 4 a word one line up when `flags`.
    /// `style` (`& 0x7f`) 1 counts in `NUMBERS_KIND1` and both 1 and 2 in
    /// `NUMBERS_KIND12`; every one in `NUMBERS_SHOWN`. 0, or -1 when the
    /// records ran out.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_number_regist(
        mem: &mut impl Mem,
        i: u32,
        kind: u32,
        value: i32,
        other: i32,
        x: u32,
        flags: u32,
        style: u32,
    ) -> i32 {
        use crate::minor::bridge::{NUMBERS_KIND1, NUMBERS_KIND12, NUMBERS_SHOWN};
        let _ = x;
        let mut n = value.wrapping_abs();
        let mut colour = u32::from(value < 0);
        if n == 0 && other < 0 {
            colour = 1;
        }
        let (mut w, mut h, mut y, mut word) = (0i32, 0i32, 0i32, 0u32);
        match kind {
            1 => {
                (w, h, word) = (0x17, 0x11, 0x00c4_24a8);
                if flags & 1 != 0 {
                    y = 0x11;
                }
            }
            3 => (n, w, h, word) = (-1, 0x2b, 0xf, 0x00c4_24a0),
            4 => {
                (n, w, h, word) = (-1, 0x38, 0x17, 0x00c4_24a4);
                if flags != 0 {
                    y = -0x19;
                }
            }
            5 => (colour, n, w, h, word) = (2, -1, 0x53, 0xf, 0x00c4_24ac),
            _ => {}
        }
        match style & 0x7f {
            1 => {
                NUMBERS_KIND1.set(mem, 1);
                NUMBERS_KIND12.set(mem, 1);
            }
            2 => NUMBERS_KIND12.set(mem, 1),
            _ => {}
        }
        NUMBERS_SHOWN.set(mem, 1);
        let count = ms_number_digits(mem, n, 0);
        let digits = NUMBER_DIGITS.ptr(mem).addr;
        let b = |v: i32| v.to_le_bytes()[0];
        let s = |v: i32| u16::from_le_bytes([v.to_le_bytes()[0], v.to_le_bytes()[1]]);
        let (i, c, st) = (
            i.to_le_bytes()[0],
            colour.to_le_bytes()[0],
            style.to_le_bytes()[0],
        );
        let mut at = w.wrapping_add(count.wrapping_mul(9)) / 2;
        for k in 0..count {
            at = at.wrapping_sub(9);
            let d = byte(mem, digits, k.cast_unsigned());
            if ms_number_put(mem, i, b(k), d, s(at), s(y), 9, 0x11, 0x1e, c, st) == 0 {
                return -1;
            }
        }
        if word != 0 {
            let mut p = code(mem, word);
            loop {
                let ch = byte(mem, p, 0);
                if ch == 0 {
                    break;
                }
                let x = at.wrapping_sub(w);
                if ms_number_put(mem, i, b(count), ch, s(x), s(y), b(w), b(h), 0x1e, c, st) == 0 {
                    return -1;
                }
                p = p.wrapping_add(1);
            }
        }
        0
    }

    /// `MsSubHP(i, actor, n, other, x, flags, style)` (`0x0078e230`):
    /// actor `i` loses `n` HP (within 0 and its maximum; none when
    /// `DebugCanTakeDamage` says so), its number shown with `flags` bit 0
    /// ([`ms_number_regist`]). The scripted fights: in battle 0x159/0 actor
    /// 0x15 keeps 1 HP while [`KEEP_ALIVE`], in battle 0x20/1 actor 0x14
    /// does; in battle 0x209/0 a KO'd 24000/48000 HP actor during a
    /// counted command (`COMMAND_3029_COUNT` 1) sets `COMMAND_SINGLE`.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_sub_hp(
        mem: &mut impl Mem,
        i: u32,
        actor: u32,
        n: i32,
        other: i32,
        x: u32,
        flags: u32,
        style: u32,
    ) {
        use ffx_kernel::bridge::ms_check_range;
        let num = crate::state::bridge::ms_battle_get_num(mem);
        let mut n = n;
        if num == 0x0159_0000 {
            match i {
                0x14 => KEEP_ALIVE.set(mem, 1),
                0x15 => {
                    if KEEP_ALIVE.get(mem) != 0 && word(mem, actor, 0x5d0) <= n {
                        n = word(mem, actor, 0x5d0) - 1;
                    }
                }
                0x17 => KEEP_ALIVE.set(mem, 0),
                _ => {}
            }
        } else if num == 0x0020_0001 && i == 0x14 && word(mem, actor, 0x5d0) <= n {
            n = word(mem, actor, 0x5d0) - 1;
        }
        if flags & 1 != 0 {
            ms_number_regist(mem, i, 0, n, other, x, flags, style);
        }
        let max = word(mem, actor, 0x594);
        if DEBUG_KILL.get(mem) != 0 {
            let k = word(mem, actor, 4);
            if k > 0 && k < 8 {
                return;
            }
            let v = word(mem, actor, 0x5d0).wrapping_sub(99_999);
            mem.write(actor.wrapping_add(0x5d0), ms_check_range(v, 0, max));
        }
        if crate::damage::bridge::debug_can_take_damage(mem, i) != 0 {
            let v = word(mem, actor, 0x5d0).wrapping_sub(n);
            let max = word(mem, actor, 0x594);
            mem.write(actor.wrapping_add(0x5d0), ms_check_range(v, 0, max));
        }
        let max = word(mem, actor, 0x594);
        if num == 0x0209_0000
            && (max == 24_000 || max == 48_000)
            && crate::parse::bridge::COMMAND_3029_COUNT.get(mem) == 1
            && word(mem, actor, 0x5d0) < 1
        {
            crate::parse::bridge::COMMAND_SINGLE.set(mem, 1);
        }
    }

    /// `MsCheatPlayerPowerup(i)` (`0x00784720`): the "player powerup"
    /// cheat ([`CHEAT_POWERUP`]) for a party member (`+0x591` 0x11, no
    /// `+0x606` bit 0): HP and MP full (each gain shown, a heal), the
    /// overdrive gauge (`+0x5bc`) full, its pose redone
    /// ([`ms_set_chr_weak`]) and its standing motion.
    pub fn ms_cheat_player_powerup<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
    ) {
        if crate::actor::bridge::ms_get_chr_top(mem) == 0 {
            return;
        }
        let a = ms_get_chr(mem, i);
        if byte(mem, a, 0x606) & 1 != 0 || byte(mem, a, 0x591) != 0x11 {
            return;
        }
        let mp = word(mem, a, 0x598).wrapping_sub(word(mem, a, 0x5d4));
        let hp = word(mem, a, 0x594).wrapping_sub(word(mem, a, 0x5d0));
        for (k, (gap, other), (v, max)) in
            [(0, (hp, mp), (0x5d0, 0x594)), (1, (mp, hp), (0x5d4, 0x598))]
        {
            if gap <= 0 {
                continue;
            }
            mem.write(a.wrapping_add(v), word(mem, a, v).wrapping_add(gap));
            ms_number_regist(
                mem,
                i,
                k,
                gap.wrapping_neg(),
                other.wrapping_neg(),
                0,
                3,
                0x81,
            );
            if word(mem, a, max).cast_unsigned() < word(mem, a, v).cast_unsigned() {
                mem.write(a.wrapping_add(v), word(mem, a, max));
            }
        }
        mem.write(a.wrapping_add(0x5bc), byte(mem, a, 0x5bd));
        ms_set_chr_weak(mem, i, u32::MAX);
        if crate::actor::bridge::ms_check_set_motion(mem, a, 2) != 0 {
            crate::status::bridge::ms_damage_set_motion(mem, host, s, i, 2, 1);
        }
    }

    /// `0x007af840(i, actor)` (`MsStatusEndMotion`, name ours): a status
    /// that ended (Sleep, Petrify's start, Stop) lets the actor move:
    /// when it may ([`crate::actor::bridge::ms_check_set_motion`] 1),
    /// `+0x433` set and motion 0x36 (`MsSetMotion(i, 0x36, 1, 0, 1)`).
    pub fn ms_status_end_motion<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, i: u32, actor: u32) {
        if crate::actor::bridge::ms_check_set_motion(mem, actor, 1) == 0 {
            return;
        }
        mem.write(actor.wrapping_add(0x433), 1u8);
        ms_set_motion(mem, s, i, 0x36, 1, 0, 1, 0, 0);
    }

    /// `MsCheckStatusBeforeAction(i, actor)` (`0x007af4f0`): what happens
    /// as actor `i` (acting, `+0xdc8`, without `+0x716`) starts its turn:
    /// every able actor with Regen (`+0x612`, alive, not KO'd or out)
    /// gets `(+0x6d2 · max HP >> 8) + 100` HP back (a Zombie, `+0x606`
    /// bit 1, loses it: `+0xded` its own, the number style 0x81 with
    /// `+0xddb`, then [`crate::death::bridge::ms_damage_check_death`], the hit
    /// motion 8 unless that killed it; else the standing motion 2 when it
    /// may), its pose and automatic statuses redone, `+0xdd0` and the
    /// count `+0x6d2` cleared; the actor's 13 status counts (`+0x608`, 1
    /// to 0xfd) of the statuses that count turns (`STATUS_COUNT_EFFECTS`
    /// flags bit 0) go down, their effect redone when bit 1
    /// (`MsStatusEffect`). Then statuses no longer held (`+0x616` against
    /// `+0x62e`: 0x800, 0x1000, 0x2000, 0x40, 0x80) end, three of them
    /// with [`ms_status_end_motion`], and Threaten ends
    /// (`MsThreatOff`). 0.
    pub fn ms_check_status_before_action<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
    ) -> u32 {
        if byte(mem, actor, 0xdc8) != 0 && byte(mem, actor, 0x716) == 0 {
            for j in 0..0x1f {
                let c = ms_get_chr(mem, j);
                if byte(mem, c, 0x612) == 0
                    || byte(mem, c, 0xdc8) == 0
                    || word(mem, c, 0x5d0) <= 0
                    || byte(mem, c, 0xdcc) != 0
                    || byte(mem, c, 0xdce) != 0
                {
                    continue;
                }
                let v = (u32::from(byte(mem, c, 0x6d2))
                    .wrapping_mul(word(mem, c, 0x594).cast_unsigned())
                    >> 8)
                    .wrapping_add(100)
                    .cast_signed();
                if v > 0 {
                    let zombie = byte(mem, c, 0x606) & 2 != 0;
                    mem.write(c.wrapping_add(0xdd0), 0u8);
                    if zombie {
                        mem.write(c.wrapping_add(0xded), j.to_le_bytes()[0]);
                        let style = if byte(mem, c, 0xddb) != 0 { 0x81 } else { 1 };
                        ms_sub_hp(mem, j, c, v, v, 0, 1, style);
                    } else {
                        ms_sub_hp(mem, j, c, -v, -v, 0, 1, 3);
                    }
                    ms_set_chr_weak(mem, j, u32::MAX);
                    crate::status::bridge::ms_check_auto_status(
                        mem,
                        &mut *s.call,
                        &mut *s.off,
                        j,
                        c,
                    );
                    let motion = if zombie {
                        let dead =
                            crate::death::bridge::ms_damage_check_death(mem, host, s, j, j, 0, 1);
                        (dead == 0).then_some(8)
                    } else {
                        (crate::actor::bridge::ms_check_set_motion(mem, c, 2) != 0).then_some(2)
                    };
                    if let Some(m) = motion {
                        crate::status::bridge::ms_damage_set_motion(mem, host, s, j, m, 1);
                    }
                }
                mem.write(c.wrapping_add(0x6d2), 0u8);
            }
            let table = crate::status::bridge::STATUS_COUNT_EFFECTS.ptr(mem).addr;
            for k in 0..13 {
                let at = actor.wrapping_add(0x608 + k);
                let rec = table.wrapping_add(4 * k);
                let v = u32::from(byte(mem, at, 0)).wrapping_sub(1);
                let flags = byte(mem, rec, 3);
                if v > 0xfc || flags & 1 == 0 {
                    continue;
                }
                mem.write(at, v.to_le_bytes()[0]);
                if flags & 2 != 0 {
                    crate::status::bridge::ms_status_effect(
                        mem,
                        &mut *s.call,
                        &mut *s.off,
                        i,
                        actor,
                        rec,
                        v,
                        actor.wrapping_add(0xe30),
                    );
                }
            }
        }
        for (bit, moves) in [
            (0x800, true),
            (0x1000, false),
            (0x2000, true),
            (0x40, true),
            (0x80, false),
        ] {
            let st = half(mem, actor, 0x616);
            if st & bit == 0 || half(mem, actor, 0x62e) & bit != 0 {
                continue;
            }
            mem.write(actor.wrapping_add(0x616), st & !bit);
            if moves {
                ms_status_end_motion(mem, s, i, actor);
            }
        }
        crate::exchange::bridge::ms_threat_off(mem, host, s, i, actor);
        0
    }

    /// `0x00792120(i, actor)` (`MsCheckAutoInput`, name ours): 1 when the
    /// actor picks its own command: `+0xdf3` (sign-extended, as returned),
    /// or for Yojimbo (actor 0xe) a roll (`brnd(0x11) & 0xff`) under a
    /// quarter of his command readiness (`MsGetYoujinbouCom`).
    pub fn ms_check_auto_input(mem: &mut impl Mem, i: u32, actor: u32) -> i32 {
        let own = i32::from(mem.read::<i8>(actor.wrapping_add(0xdf3)));
        if i == 0xe {
            let roll = (crate::rng::bridge::brnd(mem, 0x11) & 0xff).cast_signed();
            let y = crate::save::bridge::ms_get_youjinbou_com(mem).cast_signed();
            if y / 4 > roll {
                return 1;
            }
        }
        own
    }

    /// Fills command block `cmd` with one command of `id` for `targets`
    /// (`+0` the actor, `+3` 1 entry, `+8`/`+0xa` the IDs, `+0x10` the
    /// targets) and queues it (`MsSetBtlCommand(cmd, 0, flag)`).
    #[allow(clippy::too_many_arguments)]
    fn queue_one<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        cmd: u32,
        ids: u32,
        targets: u32,
        flag: u32,
    ) {
        mem.write(cmd, i.to_le_bytes()[0]);
        mem.write(cmd.wrapping_add(3), 1u8);
        mem.write(cmd.wrapping_add(8), ids);
        mem.write(cmd.wrapping_add(0x10), targets);
        crate::action::bridge::ms_set_btl_command(mem, host, s, cmd, 0, flag);
    }

    /// `MsSetDeathSentence(i, actor, cmd, flag)` (`0x00799cd0`): Doom
    /// (`+0x616` bit 14, without `+0x716`): its count `+0x5c8` down; at 0
    /// the actor's command is 0x3120 on itself ([`queue_one`] into `cmd`):
    /// 1. Else 0 (also without `cmd`).
    pub fn ms_set_death_sentence<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
        cmd: u32,
        flag: u32,
    ) -> u32 {
        if half(mem, actor, 0x616) & 0x4000 == 0 || byte(mem, actor, 0x716) != 0 || cmd == 0 {
            return 0;
        }
        let c = byte(mem, actor, 0x5c8);
        if c != 0 {
            mem.write(actor.wrapping_add(0x5c8), c - 1);
        }
        if byte(mem, actor, 0x5c8) != 0 {
            return 0;
        }
        let i = i & 0xff;
        queue_one(
            mem,
            host,
            s,
            i,
            cmd,
            0x00ff_3120,
            1u32.wrapping_shl(i),
            flag,
        );
        1
    }

    /// The command of Berserk and Provoke: `id` on `targets`; none (0xff,
    /// no targets) without an ID or targets: -2, else -1.
    #[allow(clippy::too_many_arguments)]
    fn forced<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        cmd: u32,
        id: u32,
        targets: u32,
        flag: u32,
    ) -> i32 {
        let (r, id, targets) = if id == 0 || targets.cast_signed() < 1 {
            (-2, 0xff, 0)
        } else {
            (-1, id, targets)
        };
        queue_one(mem, host, s, i, cmd, id | 0x00ff_0000, targets, flag);
        r
    }

    /// `MsSetBerserkCommand(i, actor, cmd, flag, side)` (`0x00799ac0`):
    /// Berserk or Confuse: the actor's forced command (`+0x6c6`) on its
    /// default targets on `side` (`MsGetCommandTarget(i, id, side, 1)`):
    /// -1, or -2 for none ([`forced`]); 0 without `cmd`.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_set_berserk_command<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
        cmd: u32,
        flag: u32,
        side: u32,
    ) -> i32 {
        if cmd == 0 {
            return 0;
        }
        let id = u32::from(half(mem, actor, 0x6c6));
        let t = crate::stat::bridge::ms_get_command_target(
            mem,
            host.trig,
            i,
            id,
            side.cast_signed(),
            1,
        );
        forced(mem, host, s, i, cmd, id, t, flag)
    }

    /// `MsSetProvokingCommand(i, actor, cmd, flag)` (`0x00799d50`):
    /// Provoke: once the provoker (`+0x5c4`) is gone (not acting, KO'd or
    /// out) the status ends (`+0x606` bit 10): 0; with `+0x5cb` nothing
    /// (0). Else the forced command (`+0x6c6`) on the provoker as the
    /// cursor may pick it (`MsGetCursorTarget` on its side, -14 or -15):
    /// itself for a one-target or self command (`+0x1a` bits 2-3 0, 3),
    /// its group's picks (1), all picks (2): -1, or -2 for none.
    pub fn ms_set_provoking_command<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
        cmd: u32,
        flag: u32,
    ) -> i32 {
        use crate::actor::bridge::ms_get_chr_group;
        if cmd == 0 {
            return 0;
        }
        let who = u32::from(byte(mem, actor, 0x5c4));
        let id = u32::from(half(mem, actor, 0x6c6));
        let p = ms_get_chr(mem, who);
        if byte(mem, p, 0xdc8) == 0 || byte(mem, p, 0xdcc) != 0 || byte(mem, p, 0xdce) != 0 {
            let st = half(mem, actor, 0x606);
            mem.write(actor.wrapping_add(0x606), st & 0xfbff);
            return 0;
        }
        if byte(mem, actor, 0x5cb) != 0 {
            return 0;
        }
        let monster = crate::actor::bridge::ms_get_ram_chr_monster(who) != 0;
        let rec = crate::stat::bridge::ms_get_com_data(mem, id, 0);
        let sel = (-0xe - i32::from(monster)).cast_unsigned();
        let mut set = crate::actor::bridge::ms_get_cursor_target(mem, i, sel, rec, id);
        let bit = 1u32.wrapping_shl(who);
        if set & bit == 0 {
            set = 0;
        } else {
            match (half(mem, rec, 0x1a) >> 2) & 3 {
                0 | 3 => set = bit,
                1 => {
                    let g = ms_get_chr_group(mem, who);
                    for j in 0..0x1f {
                        if (set.cast_signed() >> j) & 1 != 0 && ms_get_chr_group(mem, j) != g {
                            set &= !(1 << j);
                        }
                    }
                }
                _ => {}
            }
        }
        forced(mem, host, s, i, cmd, id, set, flag)
    }

    /// `MsBattleProcess_InputCue_00792a90()` (`0x00792a90`): unless the
    /// parsed command waits (`COMMAND_FLAG29`), the input cue's first
    /// entry not yet taken (`+3`) starts its actor's turn; a bad actor is
    /// an error (`MsDeleteInputCue`, then [`ms_battle_error`]). It waits
    /// while a Regen tick
    /// ([`crate::flow::bridge::ms_check_status_before_action_damage`])
    /// would come over commands (a Zombie's) or numbers still showing;
    /// then [`ms_check_status_before_action`]. A KO'd actor or an ended
    /// battle drops the entry; else the turn begins (`TURN_ACTOR` the
    /// actor; the item, gil, [`TURN_WORD`] and [`VOICE_WAIT`] cleared;
    /// the turn camera 0x3e when no command is queued; once, `+0xdf7`, the
    /// turn's actions 1 and, for an aeon, 0xc) and, unless a camera runs
    /// (`CAMERA_STATE`), the entry is taken (`+3`; `+0xde4` and `+0xdf7`
    /// cleared) and the command chosen into a block of the original's
    /// stack: Doom's; with the status count `+0x608` an empty turn;
    /// Provoke's; Berserk's (`+0x606` bit 9) or Confuse's (bit 8, on its
    /// own side); an automatic actor's empty turn
    /// ([`ms_check_auto_input`], without [`AUTO_OFF`]); else its menu
    /// opens (`TOSetBtlActiveChr`). The cheat ([`CHEAT_POWERUP`]) heals on
    /// the way. `+0x716` is cleared; then the next entry.
    pub fn ms_battle_process_input_cue<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
    ) {
        use crate::state::bridge as st;
        if crate::parse::bridge::COMMAND_FLAG29.get(mem) != 0 {
            return;
        }
        let taken = |mem: &Dm<'_>, e: u32| e == 0 || byte(mem, e, 3) != 0;
        let mut e = st::ms_get_input_cue(mem);
        loop {
            if taken(mem, e) {
                return;
            }
            let i = u32::from(byte(mem, e, 0));
            if crate::actor::bridge::ms_btl_chr_num_check(i) == 0 {
                crate::action::bridge::ms_delete_input_cue(mem, host, s, i, 0, 0, 0);
                let msg = code(mem, SEND_MENU_ERROR);
                ms_battle_error(mem, &mut *s.call, msg);
            }
            let a = ms_get_chr(mem, i);
            let waiting = match crate::flow::bridge::ms_check_status_before_action_damage(mem, a) {
                2 => st::COMMAND_CUE_COUNT.get(mem) != 0,
                1 => crate::minor::bridge::NUMBERS_KIND1.get(mem) != 0,
                _ => false,
            };
            if waiting {
                return;
            }
            ms_check_status_before_action(mem, host, s, i, a);
            e = st::ms_get_input_cue(mem);
            if taken(mem, e) {
                return;
            }
            if byte(mem, a, 0xdcc) != 0 || crate::menu::bridge::ms_get_battle_end_status(mem) != 0 {
                crate::action::bridge::ms_delete_input_cue(mem, host, s, i, 0, 0, 0);
            } else if turn(mem, host, s, i, a, e) {
                return;
            }
            e = st::ms_get_input_cue(mem);
            if e == 0 {
                return;
            }
        }
    }

    /// The turn of actor `i` from input entry `e`; true when a camera
    /// still runs (the entry stays).
    fn turn<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        a: u32,
        e: u32,
    ) -> bool {
        use crate::action::bridge::{ms_action_request, ms_set_btl_command};
        crate::damage::bridge::GIL_AMOUNT.set(mem, 0);
        VOICE_WAIT.set(mem, 0);
        TURN_WORD.set(mem, 0);
        crate::actor::bridge::TURN_ACTOR.set(mem, i.to_le_bytes()[0]);
        crate::save::bridge::USE_ITEM.set(mem, 0);
        if crate::state::bridge::COMMAND_CUE_COUNT.get(mem) == 0 {
            crate::atel::bridge::ms_atel_request_camera(mem, s, 0, i, 0x3e, 0xffff, 0, 1, 0, 0);
        }
        if byte(mem, a, 0xdf7) == 0 {
            mem.write(a.wrapping_add(0xdf7), 1u8);
            ms_action_request(mem, host, s, i, 0xff, 1, 0, 2, 0);
            if crate::damage::bridge::ms_get_ram_chr_summon(mem, i) != 0 {
                ms_action_request(mem, host, s, i, 0xff, 0xc, 1, 2, 0);
            }
        }
        if crate::atel::bridge::CAMERA_STATE.get(mem) != 0 {
            return true;
        }
        mem.write(e.wrapping_add(3), 1u8);
        mem.write(a.wrapping_add(0xde4), 0u8);
        mem.write(a.wrapping_add(0xdf7), 0u8);
        let flag = u32::from(byte(mem, e, 1));
        // the block of the command chosen (the original's stack)
        mem.locals(0x48, |mem, cmd| {
            let cheat = |mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>| {
                if CHEAT_POWERUP.get(mem) != 0 {
                    ms_cheat_player_powerup(mem, host, s, i);
                }
            };
            let plain = |mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>| {
                mem.write(cmd, i.to_le_bytes()[0]);
                mem.write(cmd.wrapping_add(3), 0u8);
                mem.write(cmd.wrapping_add(0x10), 0u32);
                ms_set_btl_command(mem, host, s, cmd, 0, flag);
            };
            if ms_set_death_sentence(mem, host, s, i, a, cmd, flag) != 0 {
            } else if byte(mem, a, 0x608) != 0 {
                cheat(mem, s);
                plain(mem, s);
            } else {
                let st = half(mem, a, 0x606);
                let side = if st & 0x400 != 0 {
                    None
                } else if st & 0x200 != 0 {
                    Some(0)
                } else if st & 0x100 != 0 {
                    Some(1)
                } else {
                    None
                };
                if let Some(side) = side {
                    cheat(mem, s);
                    ms_set_berserk_command(mem, host, s, i, a, cmd, flag, side);
                } else if st & 0x400 == 0
                    || ms_set_provoking_command(mem, host, s, i, a, cmd, flag) == 0
                {
                    if ms_check_auto_input(mem, i, a) != 0 && AUTO_OFF.get(mem) == 0 {
                        plain(mem, s);
                    } else {
                        let f = code(mem, TO_SET_BTL_ACTIVE_CHR);
                        (s.call)(mem, f, &[i]);
                        cheat(mem, s);
                    }
                }
            }
        });
        mem.write(a.wrapping_add(0x716), 0u8);
        false
    }
}
