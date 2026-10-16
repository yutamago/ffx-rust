//! The battle's action flow (`docs/rewrite-plan.md`, M2): starting an
//! effect once its actors are free, head targets, ending status effects,
//! the overdrive statuses' HP and MP, the party's return, and the battle
//! records kept per character. Effect calls into `ffx_vfx` (which depends
//! on this crate through `ffx_field`) come from the caller.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // actor, index, value
pub mod bridge {
    use ffx_bridge::{Global, Mem};
    use ffx_kernel::bridge::ms_check_range;

    use crate::actor::bridge::ms_get_chr;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn half(mem: &impl Mem, p: u32, o: u32) -> u16 {
        mem.read(p.wrapping_add(o))
    }

    fn word(mem: &impl Mem, p: u32, o: u32) -> u32 {
        mem.read(p.wrapping_add(o))
    }

    /// Who an effect waits on (`MsEffectCheckMotion`, `MsCheckStartEffect`):
    /// the actor and its motion (0xff: any).
    pub const EFFECT_WAIT: Global<[u8; 2]> = Global::new(0x0112_ca08);
    /// Set: no head target (`MsSetHeadTarget`).
    pub const NO_HEAD_TARGET: Global<u8> = Global::new(0x0112_a926);
    /// Set: the menu always looks (`MsMenuCheckLook`).
    pub const MENU_LOOK: Global<u8> = Global::new(0x0112_a915);
    /// The party saved while it moved freely (`MsReturnParty`), and the 17
    /// bytes after the party it brings back too.
    pub const PARTY_SAVED: Global<[u8; 7]> = Global::new(0x0112_c89c);
    pub const PARTY_EXTRA: Global<[u8; 17]> = Global::new(0x0112_c8a3);
    pub const PARTY_EXTRA_SAVED: Global<[u8; 17]> = Global::new(0x0112_c8b4);
    /// `g_BtlStealRule`: 2 (nothing to steal) also keeps the battle records
    /// from counting (`MsSaveAddInfoData`).
    pub const RECORDS_OFF: Global<u8> = Global::new(0x0112_c9e5);

    /// `MsStatusEffectClear(i)` (`0x0079f110`): ends the actor's 24 status
    /// effects (`+0xe30`, `op_et_eff_off_signal` from the caller); how
    /// many there were.
    pub fn ms_status_effect_clear<M: Mem>(
        mem: &mut M,
        i: u32,
        mut off: impl FnMut(&mut M, u32),
    ) -> u32 {
        let a = ms_get_chr(mem, i);
        let mut n = 0;
        for k in 0..24 {
            let at = a.wrapping_add(0xe30 + 4 * k);
            let e: u32 = mem.read(at);
            if e != 0 {
                off(mem, e);
                mem.write(at, 0u32);
                n += 1;
            }
        }
        n
    }

    /// `MsResetBindEffect(i)` (`0x00788e60`): ends the bind effect of the
    /// actor's character (`+0x1f` cleared, effect level 0,
    /// `op_et_bindeff_off_signal` from the caller).
    pub fn ms_reset_bind_effect<M: Mem>(mem: &mut M, i: u32, off: impl FnOnce(&mut M, u32)) {
        let a = ms_get_chr(mem, i & 0xff);
        let c = word(mem, a, 0);
        if c != 0 {
            mem.write(a.wrapping_add(0x1f), 0u8);
            ffx_chara::chr::bridge::ch_effect_set_effect_level(mem, c, 0);
            off(mem, c);
        }
    }

    /// `MsResetAfterImage(actor)` (`0x00788e30`): fades the weapon's after
    /// image (`+0xea8`, `op_et_weapon_afterimage_fade` from the caller).
    pub fn ms_reset_after_image<M: Mem>(mem: &mut M, actor: u32, fade: impl FnOnce(&mut M, u32)) {
        let e = word(mem, actor, 0xea8);
        if e != 0 {
            fade(mem, e);
            mem.write(actor.wrapping_add(0xea8), 0u32);
        }
    }

    /// `MsEffectCheckMotion(i)` (`0x00787840`): 0 when the actor's character
    /// waits in a motion (a motion speed at `+0x400`, `Ch_GetWaitMotion`),
    /// marking it in [`EFFECT_WAIT`] with its motion (`+0x3f0`); else -1.
    pub fn ms_effect_check_motion(mem: &mut impl Mem, i: u32) -> i32 {
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        let c = word(mem, a, 0);
        if c != 0
            && half(mem, a, 0x400) != 0
            && ffx_chara::parts::bridge::ch_get_wait_motion(mem, c) != 0
        {
            let m = byte(mem, a, 0x3f0);
            EFFECT_WAIT.set(mem, [i.to_le_bytes()[0], m]);
            return 0;
        }
        -1
    }

    /// `MsCheckStartEffect(chr, set)` (`0x007a92c0`): how many actors of
    /// `set` keep `chr`'s effect waiting: others still busy (`+0x508`) or
    /// any waiting on a motion (`+0x436`); each marks [`EFFECT_WAIT`].
    pub fn ms_check_start_effect(mem: &mut impl Mem, chr: u32, set: u32) -> u32 {
        let mut n = 0;
        for i in 0..0x1f {
            if (set.cast_signed() >> i) & 1 == 0 {
                continue;
            }
            let a = ms_get_chr(mem, i);
            if (chr != i && byte(mem, a, 0x508) != 0) || byte(mem, a, 0x436) != 0 {
                EFFECT_WAIT.set(mem, [chr.to_le_bytes()[0], 0xff]);
                n += 1;
            }
        }
        n
    }

    /// `MsSetHeadTarget(chr, set)` (`0x007b1e80`): an actor with a head
    /// (`+0x437`) looks at the one other actor of `set` (`+0x438`, mode 5 at
    /// `+0x439`), or at none (0xff, mode 5) otherwise; how many others
    /// there are.
    pub fn ms_set_head_target(mem: &mut impl Mem, chr: u32, set: u32) -> u32 {
        if NO_HEAD_TARGET.get(mem) != 0 {
            return 0;
        }
        let a = ms_get_chr(mem, chr);
        if word(mem, a, 0) == 0 || byte(mem, a, 0x437) == 0 {
            return 0;
        }
        let mut n = 0;
        let mut last = chr;
        for i in 0..0x1f {
            if set & (1 << i) != 0 && chr != i {
                last = i;
                n += 1;
            }
        }
        if n == 1 {
            mem.write(a.wrapping_add(0x438), last.to_le_bytes()[0]);
            mem.write(a.wrapping_add(0x439), 5u8);
        } else {
            mem.write(a.wrapping_add(0x438), 0x5ffu16);
        }
        n
    }

    /// `MsSaveAddInfoData(chr, kind)` (`0x00785a00`): counts one more of
    /// battle record `kind` (0-3, `g_PlySave` `+0x50 + 4·kind`, at most
    /// 999999999) for character `chr` (signed, below 18), unless records
    /// are off; the new count, else 0.
    pub fn ms_save_add_info_data(mem: &mut impl Mem, chr: i32, kind: u32) -> i32 {
        if chr >= 0x12 || RECORDS_OFF.get(mem) == 2 || kind > 3 {
            return 0;
        }
        let at = crate::save::bridge::PLY_SAVE
            .ptr(mem)
            .addr
            .wrapping_add(chr.cast_unsigned().wrapping_mul(0x94))
            .wrapping_add(0x50 + 4 * kind);
        let v: i32 = mem.read(at);
        let v = ms_check_range(v.wrapping_add(1), 0, 999_999_999);
        mem.write(at, v);
        v
    }

    /// `MsLimitStatusProcess(_, actor, flags)` (`0x0078d270`): the
    /// overdrive statuses (`+0x640`: bit 0 doubles max HP, bit 1 max MP;
    /// `flags` not negative replaces them): as a bit turns on (or is on
    /// while kept), the maximum (`+0x594`/`+0x598`) becomes twice the base
    /// (`+0x59c`/`+0x5a0`) up to 9999 (99999 with ability 0x200 of
    /// `+0x6be`) or 999 (9999 with 0x400); as it turns off, the base; HP or
    /// MP (`+0x5d0`/`+0x5d4`) kept within. 0.
    pub fn ms_limit_status_process(mem: &mut impl Mem, actor: u32, flags: i32) -> u32 {
        let old = u32::from(byte(mem, actor, 0x640));
        let keep = flags < 0;
        let new = if keep {
            old
        } else {
            mem.write(actor.wrapping_add(0x640), flags.to_le_bytes()[0]);
            flags.cast_unsigned()
        };
        let abil = half(mem, actor, 0x6be);
        for (bit, base, max, cur, cap, more, ab) in [
            (0, 0x59c, 0x594, 0x5d0, 9999, 90000, 0x200u16),
            (1, 0x5a0, 0x598, 0x5d4, 999, 9000, 0x400),
        ] {
            let (o, n) = ((old >> bit) & 1, (new >> bit) & 1);
            let m = if n != 0 && (o == 0 || keep) {
                let b: i32 = mem.read(actor.wrapping_add(base));
                let hi = if abil & ab != 0 { cap + more } else { cap };
                ms_check_range(b.wrapping_mul(2), 0, hi)
            } else if (o != 0 || keep) && n == 0 {
                mem.read(actor.wrapping_add(base))
            } else {
                continue;
            };
            mem.write(actor.wrapping_add(max), m);
            let v: i32 = mem.read(actor.wrapping_add(cur));
            mem.write(actor.wrapping_add(cur), ms_check_range(v, 0, m));
        }
        0
    }

    /// `MsMenuCheckLook()` (`0x0078f9e0`): -1 when the menu always looks
    /// or a party member with Scan (`+0x6bc` bit 0) is in and can act
    /// (`MsAutoCommandStatusCheck(actor, 1, 1, 0)`); else 0.
    pub fn ms_menu_check_look(mem: &impl Mem) -> i32 {
        if MENU_LOOK.get(mem) != 0 {
            return -1;
        }
        for i in 0..0x12 {
            let a = ms_get_chr(mem, i);
            if byte(mem, a, 0x6bc) & 1 != 0
                && byte(mem, a, 0xdcc) == 0
                && crate::actor::bridge::ms_auto_command_status_check(mem, a, 1, 1, 0) != 0
            {
                return -1;
            }
        }
        0
    }

    /// `MsEffectCheckTarget(set)` (`0x00787900`): with several actors in
    /// `set`, only those with an effect target flag (`+0xdda`); else `set`.
    pub fn ms_effect_check_target(mem: &impl Mem, set: u32) -> u32 {
        let (mut n, mut flagged) = (0, 0u32);
        for i in 0..0x1f {
            if (set.cast_signed() >> i) & 1 != 0 {
                n += 1;
                if byte(mem, ms_get_chr(mem, i), 0xdda) != 0 {
                    flagged |= 1 << i;
                }
            }
        }
        if n > 1 { flagged } else { set }
    }

    /// `MsReturnParty()` (`0x007aeec0`): after moving freely
    /// ([`crate::minor::bridge::DIR_FREE`]), the saved party and the 17
    /// bytes after it come back.
    pub fn ms_return_party(mem: &mut impl Mem) {
        let free = crate::minor::bridge::DIR_FREE;
        if free.get(mem) == 0 {
            return;
        }
        free.set(mem, 0);
        let p = PARTY_SAVED.get(mem);
        crate::menu::bridge::PARTY.set(mem, p);
        let x = PARTY_EXTRA_SAVED.get(mem);
        PARTY_EXTRA.set(mem, x);
    }
    /// `MsSetChrImageDamageAttack(actor)` (`0x0078d790`): the larger of the
    /// actor's damage to itself with formulas 0xe and 0x14 (power 0x10,
    /// `MsCalcDamageCommand`), kept at `+0x6f4` and returned.
    pub fn ms_set_chr_image_damage_attack(mem: &mut impl Mem, actor: u32) -> i32 {
        use crate::damage::bridge::ms_calc_damage_command;
        let a = ms_calc_damage_command(mem, actor, actor, 0, 0xe, 0x10, 0, 1, 0, 0, 0, 0);
        let b = ms_calc_damage_command(mem, actor, actor, 0, 0x14, 0x10, 0, 1, 0, 0, 0, 0);
        let v = if a > b { a } else { b };
        mem.write(actor.wrapping_add(0x6f4), v);
        v
    }

    /// The battle ground's height (`MsSetChrHeight`), an `f32`, and 0.5 as
    /// the `f64` constant it multiplies by.
    pub const GROUND: Global<u32> = Global::new(0x0112_bde8);
    pub const HALF: Global<f64> = Global::new(0x00b9_2220);

    /// `MsSetChrHeight(i)` (`0x007ab300`): the actor's float height
    /// (`+0x534`) by its height mode (`+0x50c`): 0, the ground less half
    /// its height (`MsGetChrHeight`) plus `+0x538` (x87, stored as `f32`);
    /// 1, zero; else unchanged.
    pub fn ms_set_chr_height(mem: &mut impl Mem, i: u32) {
        use ffx_bridge::x87::{add, load, mul, store, sub};
        let a = ms_get_chr(mem, i);
        match byte(mem, a, 0x50c) {
            0 => {
                let g = load(GROUND.get(mem));
                let h = crate::body::bridge::ms_get_chr_height(mem, i).to_bits();
                let v = add(
                    sub(g, mul(load(h), HALF.get(mem))),
                    load(word(mem, a, 0x538)),
                );
                mem.write(a.wrapping_add(0x534), store(v));
            }
            1 => mem.write(a.wrapping_add(0x534), 0u32),
            _ => {}
        }
    }

    /// `MsGetScreenData(&a, &b, &c, &d)` (`0x007bbd00`): the screen's
    /// `+0x40`-`+0x4c` floats truncated to integers (`_ftol2_sse`) into the
    /// pointers that are not null; returns its `+0xc4`.
    pub fn ms_get_screen_data(mem: &mut impl Mem, out: [u32; 4]) -> u32 {
        let screen = crate::misc::bridge::SCREEN;
        for (k, p) in (0u32..).zip(out) {
            if p != 0 {
                let v = ffx_bridge::x87::load(word(mem, screen.get(mem), 0x40 + 4 * k));
                let n = ffx_bridge::x87::ftol(mem, v);
                mem.write(p, n);
            }
        }
        word(mem, screen.get(mem), 0xc4)
    }

    /// The battle positions' state `MsBtlPosInit` resets: a block of 0x34
    /// bytes (three scales of 1.0 at `+0x10`), the words after it (one
    /// pointing back at it), the hash list it names, and -1 at
    /// `0x023114f4`.
    pub const BTL_POS: Global<u8> = Global::new(0x0113_e23c);
    pub const BTL_POS_LIST: Global<u8> = Global::new(0x00c4_4270);
    pub const BTL_POS_FOCUS: Global<i32> = Global::new(0x0231_14f4);

    /// `MsBtlPosInit()` (`0x007c93d0`).
    pub fn ms_btl_pos_init(mem: &mut impl Mem) {
        let l = BTL_POS_LIST.ptr(mem).addr;
        crate::util::bridge::ms_set_hash_value(mem, l);
        let p = BTL_POS.ptr(mem).addr;
        crate::util::bridge::ms_struct_clear(mem, p, 0x34);
        for o in [0x10, 0x14, 0x18] {
            mem.write(p + o, 1.0f32.to_bits());
        }
        mem.write(p + 0x34, [0u32, 0, p, 0, 0, 0]);
        BTL_POS_FOCUS.set(mem, -1);
    }

    /// `MsAtelRegistInit()` (`0x00797ae0`): the script managers' first
    /// words, their 0x1000 and 0x800 bytes of work, and the requests
    /// (`MsAtelRequestInit`).
    pub fn ms_atel_regist_init(mem: &mut impl Mem) {
        let m = crate::minor::bridge::ATEL_MANAGERS.ptr(mem).addr;
        mem.write(m, [0u32; 5]);
        mem.write(m + 0x54, [0u32; 5]);
        crate::util::bridge::ms_struct_clear(mem, m + 0x100, 0x1000);
        crate::util::bridge::ms_struct_clear(mem, m + 0x1100, 0x800);
        crate::state::bridge::ms_atel_request_init(mem);
    }

    /// `MsSetChrMotionInfo(i, k, value)` (`0x007b4a90`): one of the actor's
    /// motion floats (k 0-8: `+0x4bc`, `+0x4c0`, `+0x4c8`, `+0x4d4`,
    /// `+0x4d8`, `+0x4cc`, `+0x4d0`, `+0x4dc`, `+0x4c4`); 9 is its width
    /// (`+0x4e4`, flag `+0x44c`), given to its character too
    /// (`Ch_SetThickness`).
    pub fn ms_set_chr_motion_info(mem: &mut impl Mem, i: u32, k: u32, value: u32) {
        let a = ms_get_chr(mem, i);
        let c = word(mem, a, 0);
        let v = ffx_bridge::x87::store(ffx_bridge::x87::load(value));
        let o = match k {
            0 => 0x4bc,
            1 => 0x4c0,
            2 => 0x4c8,
            3 => 0x4d4,
            4 => 0x4d8,
            5 => 0x4cc,
            6 => 0x4d0,
            7 => 0x4dc,
            8 => 0x4c4,
            9 => {
                mem.write(a.wrapping_add(0x44c), 1u8);
                mem.write(a.wrapping_add(0x4e4), v);
                if c != 0 {
                    ffx_chara::chr::bridge::ch_set_thickness(mem, c, f32::from_bits(v));
                }
                return;
            }
            _ => return,
        };
        mem.write(a.wrapping_add(o), v);
    }

    /// `MsExchangeParty()` (`0x007ade10`): the party and the saved party
    /// swap, the 17 bytes after them too, and the first 18 actors' "can
    /// act" (`+0xdc8`) and its saved copy (`+0xdc9`).
    pub fn ms_exchange_party(mem: &mut impl Mem) {
        let party = crate::menu::bridge::PARTY;
        let (p, q) = (party.get(mem), PARTY_SAVED.get(mem));
        PARTY_SAVED.set(mem, p);
        party.set(mem, q);
        let (x, y) = (PARTY_EXTRA.get(mem), PARTY_EXTRA_SAVED.get(mem));
        PARTY_EXTRA_SAVED.set(mem, x);
        PARTY_EXTRA.set(mem, y);
        for i in 0..0x12 {
            let a = ms_get_chr(mem, i);
            let (s, t) = (byte(mem, a, 0xdc8), byte(mem, a, 0xdc9));
            mem.write(a.wrapping_add(0xdc9), s);
            mem.write(a.wrapping_add(0xdc8), t);
        }
    }

    /// `MsChrSEfoot(i, v)` (`0x0079fd20`): the actor's footstep sound
    /// (`+0x40f`).
    pub fn ms_chr_se_foot(mem: &mut impl Mem, i: u32, v: u8) {
        let a = ms_get_chr(mem, i & 0xff);
        mem.write(a.wrapping_add(0x40f), v);
    }

    /// `MsSEP3Dinit()` (`0x007a0910`): the 3D sound list and its 0x200
    /// bytes of nodes.
    pub fn ms_sep3d_init(mem: &mut impl Mem) {
        crate::minor::bridge::SEP_3D.set(mem, 0);
        let p = crate::minor::bridge::SEP_3D.ptr(mem).addr + 0x10;
        crate::util::bridge::ms_struct_clear(mem, p, 0x200);
    }

    /// `MsEffectLimitSync(i)` (`0x007afde0`): -1 while the actor's
    /// overdrive effect syncs (`+0xd24`), else 0.
    pub fn ms_effect_limit_sync(mem: &impl Mem, i: u32) -> i32 {
        -i32::from(byte(mem, ms_get_chr(mem, i & 0xff), 0xd24) != 0)
    }

    /// The menu's texts `MsMenuInitText` clears (17 of 12 bytes).
    pub const MENU_TEXT: Global<u8> = Global::new(0x0113_33f0);

    /// `MsMenuInitText()` (`0x0078fe60`).
    pub fn ms_menu_init_text(mem: &mut impl Mem) {
        let p = MENU_TEXT.ptr(mem).addr;
        for k in 0..17 {
            crate::util::bridge::ms_struct_clear(mem, p + 12 * k, 12);
        }
    }

    /// `g_SaveYoujinbouCom`: Yojimbo's command counter (`MsAddYoujinbouCom`), 0-255.
    pub const YOJIMBO_COMMANDS: Global<i32> = Global::new(0x0113_0834);

    /// `MsAddYoujinbouCom(n)` (`0x007b2950`): the new count.
    pub fn ms_add_youjinbou_com(mem: &mut impl Mem, n: i32) -> i32 {
        let v = ms_check_range(YOJIMBO_COMMANDS.get(mem).wrapping_add(n), 0, 0xff);
        YOJIMBO_COMMANDS.set(mem, v);
        v
    }

    /// `MsMagicEffectSet(chr, actor)` (`0x0079e890`): the actor's magic
    /// effect pair (`+0xe10`): 0x3837 for a monster, else 0x1f1e.
    pub fn ms_magic_effect_set(mem: &mut impl Mem, chr: u32, actor: u32) {
        let v: u16 = if crate::actor::bridge::ms_get_ram_chr_monster(chr) != 0 {
            0x3837
        } else {
            0x1f1e
        };
        mem.write(actor.wrapping_add(0xe10), v);
    }

    /// `MsMenuCheckChrLook(i)` (`0x0078f990`): 1 when the menu always looks
    /// or `i` is a monster not hidden from Scan (`+0x5b8` bit 3); else 0.
    pub fn ms_menu_check_chr_look(mem: &impl Mem, i: u32) -> u32 {
        if MENU_LOOK.get(mem) != 0 {
            return 1;
        }
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        u32::from(
            crate::actor::bridge::ms_get_ram_chr_monster(i) != 0 && byte(mem, a, 0x5b8) & 8 == 0,
        )
    }

    /// `MsSetChrMoveFlagAll(on)` (`0x00788e90`): every actor's move flag
    /// (`+0x412`, low byte), `+0x404` cleared, specular faded to 0 over 30
    /// frames, and with `on` part 2 of its body hit off.
    pub fn ms_set_chr_move_flag_all(mem: &mut impl Mem, on: u32) {
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            mem.write(a.wrapping_add(0x412), on.to_le_bytes()[0]);
            mem.write(a.wrapping_add(0x404), 0u8);
            crate::body::bridge::ms_set_chr_specular(mem, i, 0, 0x1e);
            if on != 0 {
                crate::body::bridge::ms_set_chr_body_hit(mem, a, 2, 0);
            }
        }
    }
    /// `MsGetFirstAttack(chance)` (`0x0078d090`): a first strike by the
    /// party (1: a random byte below `chance`; with First Strike on an
    /// acting party member, `g_PlySave` `+0x4a` bit 2, always) or an
    /// ambush (2: at least `255 − chance`), else 0.
    pub fn ms_get_first_attack(mem: &mut impl Mem, chance: i32) -> u32 {
        let mut r = (crate::rng::bridge::brnd(mem, 1) & 0xff).cast_signed();
        for i in 0..0x12 {
            if byte(mem, ms_get_chr(mem, i), 0xdc8) != 0
                && byte(
                    mem,
                    crate::save::bridge::ms_get_save_player_ptr(mem, i),
                    0x4a,
                ) & 4
                    != 0
            {
                r = r.wrapping_add((-1i32).wrapping_sub(chance));
                break;
            }
        }
        if r >= 0xff_i32.wrapping_sub(chance) {
            2
        } else {
            u32::from(r < chance)
        }
    }

    /// `MsDefenseProcess()` (`0x007af8a0`): 0 while an acting actor with a
    /// defense partner (`+0x6d4`) or that partner is still busy
    /// (`+0x508`), both marked in [`crate::body::bridge::MOTION_BUSY`];
    /// else 1.
    pub fn ms_defense_process(mem: &mut impl Mem) -> u32 {
        let busy = crate::body::bridge::MOTION_BUSY;
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            let d = byte(mem, a, 0x6d4);
            if d == 0xff || byte(mem, a, 0xdc8) == 0 {
                continue;
            }
            let m = busy.get(mem);
            busy.set(mem, m | (1 << (d & 31)) | (1 << i));
            if byte(mem, a, 0x508) != 0 || byte(mem, ms_get_chr(mem, u32::from(d)), 0x508) != 0 {
                return 0;
            }
        }
        1
    }

    /// `MsBtlChrInit()` (`0x007935b0`): clears the 31 actors, the spare
    /// record, the 62 weapon records and the 0x24 bytes at `*0x02311050`.
    pub const CHR_EXTRA: Global<u32> = Global::new(0x0231_1050);

    /// See [`CHR_EXTRA`].
    pub fn ms_btl_chr_init(mem: &mut impl Mem) {
        use crate::actor::bridge::{CHR_SPARE, CHR_TOP, WEAPON_TOP};
        use crate::util::bridge::ms_struct_clear;
        for k in 0..0x1f {
            let p = CHR_TOP.get(mem).wrapping_add(k * 0xf90);
            ms_struct_clear(mem, p, 0xf90);
        }
        let p = CHR_SPARE.ptr(mem).addr;
        ms_struct_clear(mem, p, 0xf90);
        for k in 0..0x3e {
            let p = WEAPON_TOP.get(mem).wrapping_add(k * 0x390);
            ms_struct_clear(mem, p, 0x390);
        }
        let p = CHR_EXTRA.get(mem);
        ms_struct_clear(mem, p, 0x24);
    }

    /// Tidus' overdrive learning (`MsLimitTidusLearn`): how often he has
    /// used one, the 4 overdrives with the uses each needs, and the one
    /// just learnt.
    pub const TIDUS_LIMIT_USES: Global<i32> = Global::new(0x0113_083c);
    pub const TIDUS_LIMITS: Global<[u16; 4]> = Global::new(0x00c4_3948);
    pub const TIDUS_LIMIT_NEEDS: Global<[i32; 4]> = Global::new(0x00c4_3950);
    pub const LIMIT_LEARNT: Global<u8> = Global::new(0x0112_c02b);
    pub const LIMIT_LEARNT_ID: Global<u16> = Global::new(0x0112_c94a);

    /// `MsLimitTidusLearn(chr)` (`0x007b0cd0`): for Tidus (0), unless
    /// records are off, one more use; each overdrive he lacks whose need
    /// is met is learnt (the last in [`LIMIT_LEARNT_ID`]); 1 when one was.
    pub fn ms_limit_tidus_learn(mem: &mut impl Mem, chr: i32) -> u32 {
        if chr != 0 || RECORDS_OFF.get(mem) == 2 {
            return 0;
        }
        let uses = TIDUS_LIMIT_USES.get(mem).wrapping_add(1);
        TIDUS_LIMIT_USES.set(mem, uses);
        let mut learnt = 0;
        for (id, need) in TIDUS_LIMITS
            .get(mem)
            .into_iter()
            .zip(TIDUS_LIMIT_NEEDS.get(mem))
        {
            if crate::save::bridge::ms_get_save_command(mem, 0, u32::from(id)) == 0 && uses >= need
            {
                learnt = 1;
                LIMIT_LEARNT.set(mem, 1);
                LIMIT_LEARNT_ID.set(mem, id);
            }
        }
        learnt
    }

    /// `MsBattleCameraFree()` (`0x007ba950`): frees the script cameras'
    /// third and fourth requests (`MsCameraFree`), then the screens' focus.
    pub fn ms_battle_camera_free(mem: &mut impl Mem) {
        let at = crate::minor::bridge::ATEL_CAMERA.ptr(mem).addr;
        for k in 0..3 {
            for o in [0x18, 0x24] {
                let p = at + o + 4 * k;
                let id: u32 = mem.read(p);
                crate::camera::bridge::ms_camera_free(mem, id);
                mem.write(p, 0u32);
            }
        }
        crate::misc::bridge::ms_screen_focus_reset(mem);
    }

    /// `MsBindOffCamera(w)` (`0x007ba9c0`): the script worker's camera
    /// (`AtelGetCameraWorkAdrs`, from the caller) leaves the script cameras
    /// and is freed (`MsCameraFree`).
    pub fn ms_bind_off_camera<M: Mem>(mem: &mut M, w: u32, work: impl FnOnce(&mut M, u32) -> u32) {
        let slot = work(mem, w);
        let id: u32 = mem.read(slot);
        if id == 0 {
            return;
        }
        let at = crate::minor::bridge::ATEL_CAMERA.ptr(mem).addr;
        for k in 0..3 {
            for o in [0, 0xc, 0x18, 0x24] {
                let p = at + o + 4 * k;
                if mem.read::<u32>(p) == id {
                    mem.write(p, 0u32);
                }
            }
        }
        crate::camera::bridge::ms_camera_free(mem, id);
        mem.write(slot, 0u32);
    }

    /// `MsCheckStatusBeforeActionDamage(actor)` (`0x007af770`): for an
    /// acting actor without `+0x716`, 2 when an acting actor with a
    /// counter running (`+0x612`) and `(+0x6d2 · max HP) >> 8` above -100
    /// has status bit 2, 1 when some do without it, else 0.
    pub fn ms_check_status_before_action_damage(mem: &impl Mem, actor: u32) -> u32 {
        if byte(mem, actor, 0xdc8) == 0 || byte(mem, actor, 0x716) != 0 {
            return 0;
        }
        let (mut with, mut without) = (0, 0);
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            if byte(mem, a, 0xdc8) == 0 || byte(mem, a, 0x612) == 0 {
                continue;
            }
            let p = u32::from(byte(mem, a, 0x6d2)).wrapping_mul(word(mem, a, 0x594));
            if ((p >> 8).wrapping_add(100)).cast_signed() <= 0 {
                continue;
            }
            if byte(mem, a, 0x606) & 2 != 0 {
                with += 1;
            } else {
                without += 1;
            }
        }
        if with != 0 {
            2
        } else {
            u32::from(without != 0)
        }
    }
}
