//! Status effects and the motions statuses bring (`docs/rewrite-plan.md`,
//! M2): `MsStatusEffect` and `MsStatusEffectCheck` start and end an
//! actor's status effects as its statuses say, `MsCheckStatusAfterInput`
//! and `MsCheckStatusAfterAction` count statuses down around a turn, and
//! `MsDamageSetMotion` picks the motion a hit (or petrification) plays,
//! with the weapon re-attached (`MsAttachWeapon`) and the actor put back
//! on its place (`MsResetChrPos`).
//!
//! The effect calls (`op_et_eff`, `op_et_astatus_effect_change`,
//! `Ch_ModelSetHide`) go to the game's code through a [`Call`]; ending an
//! effect (`op_et_eff_off_signal`) is the caller's `off`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, records
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::ms_get_chr;
    use crate::atel::bridge::{Script, ms_reset_head_target, ms_set_motion};
    use crate::exec::bridge::Call;
    use crate::target::bridge::{Dm, Host, ms_set_chr_dir_target};

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    /// `op_et_eff` and `op_et_astatus_effect_change` (vfx).
    pub const OP_ET_EFF: u32 = 0x007f_c2e0;
    pub const OP_ET_ASTATUS_EFFECT_CHANGE: u32 = 0x007f_aed0;
    /// The effect owner `MsStatusEffect` starts status effects on.
    pub const EFFECT_OWNER: Global<u32> = Global::new(0x0112_a95c);
    /// The status effect tables: 12 for `+0x606`'s bits, 13 for the
    /// counters `+0x608`, 16 for `+0x616`'s bits; 4 bytes each (effect, its
    /// underwater one, slot in `+0xe30`, flags).
    pub const STATUS_EFFECTS: Global<[u8; 4]> = Global::new(0x00c4_23e8);
    pub const STATUS_EFFECTS2: Global<[u8; 4]> = Global::new(0x00c4_2420);
    pub const STATUS_COUNT_EFFECTS: Global<[u8; 4]> = Global::new(0x00c4_2464);

    /// `MsStatusEffect(i, actor, rec, on, slots)` (`0x0079ef40`): status
    /// effect record `rec` (effect `+0` not 0xff; underwater `+1`, while
    /// `g_BtlUnderwater`; slot `+2` in `slots`; flags `+3`) on or off: on
    /// with its slot free, a new effect ID (`MsGetEffID(+0xe20, 3, i)`,
    /// `+0xe20` counted on) starts it (`op_et_eff(owner, 0, effect, id)`
    /// into the slot) unless flags bit 4; off, a running one ends
    /// (`op_et_eff_off_signal`, slot cleared).
    #[allow(clippy::too_many_arguments)]
    pub fn ms_status_effect<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        off: &mut dyn FnMut(&mut M, u32),
        i: u32,
        actor: u32,
        rec: u32,
        on: u32,
        slots: u32,
    ) {
        let r: [u8; 4] = mem.read(rec);
        if r[0] == 0xff {
            return;
        }
        let slot = slots.wrapping_add(4 * u32::from(r[2]));
        let cur: u32 = mem.read(slot);
        if on == 0 {
            if cur != 0 {
                off(mem, cur);
                mem.write(slot, 0u32);
            }
            return;
        }
        if cur != 0 {
            return;
        }
        let n = mem.read::<u16>(actor.wrapping_add(0xe20));
        let id = crate::util::bridge::ms_get_eff_id(u32::from(n), 3, i);
        mem.write(actor.wrapping_add(0xe20), n.wrapping_add(1));
        let water = crate::state::bridge::UNDERWATER.get(mem) != 0;
        let e = u32::from(if water { r[1] } else { r[0] });
        if r[3] & 0x10 != 0 {
            return;
        }
        let owner = EFFECT_OWNER.get(mem);
        let f = code(mem, OP_ET_EFF);
        call(mem, f, &[owner, 0, e, id]);
        mem.write(slot, id);
    }

    /// `0x0079e7c0(i)` (`MsGetStatusLook`, name ours): the statuses that
    /// change how actor `i & 0xff` looks, as bits: 1 `+0x606` bit 1, 2
    /// petrified (`+0xdce`), 4 `+0x606` bit 9, 8 `+0x613`, 0x10 `+0x614`,
    /// 0x20 `+0x616` bit 10.
    pub fn ms_get_status_look(mem: &impl Mem, i: u32) -> u32 {
        let a = ms_get_chr(mem, i & 0xff);
        let s = mem.read::<u16>(a.wrapping_add(0x606));
        let mut m = u32::from(s & 2 != 0);
        if byte(mem, a, 0xdce) != 0 {
            m |= 2;
        }
        if s & 0x200 != 0 {
            m |= 4;
        }
        if byte(mem, a, 0x613) != 0 {
            m |= 8;
        }
        if byte(mem, a, 0x614) != 0 {
            m |= 0x10;
        }
        if mem.read::<u16>(a.wrapping_add(0x616)) & 0x400 != 0 {
            m |= 0x20;
        }
        m
    }

    /// `MsStatusEffectCheck(i)` (`0x0079eff0`): unless `+0xdcd`, every
    /// status effect on or off as the actor's statuses say
    /// ([`ms_status_effect`] over the three tables); then its look
    /// ([`ms_get_status_look`] into `+0x6f8`) and, when that changed from
    /// `+0x6fc`, its character's (`op_et_astatus_effect_change(chr,
    /// look)`) with `+0x6fc` the new one.
    pub fn ms_status_effect_check<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        off: &mut dyn FnMut(&mut M, u32),
        i: u32,
    ) {
        let a = ms_get_chr(mem, i);
        if byte(mem, a, 0xdcd) == 0 {
            let slots = a.wrapping_add(0xe30);
            let t = STATUS_EFFECTS.ptr(mem);
            for k in 0..12 {
                let on = u32::from(mem.read::<u16>(a.wrapping_add(0x606))) >> k & 1;
                ms_status_effect(mem, call, off, i, a, t.at(k).addr, on, slots);
            }
            let t = STATUS_COUNT_EFFECTS.ptr(mem);
            for k in 0..13 {
                let on = u32::from(byte(mem, a, 0x608 + k));
                ms_status_effect(mem, call, off, i, a, t.at(k).addr, on, slots);
            }
            let t = STATUS_EFFECTS2.ptr(mem);
            for k in 0..16 {
                let on = u32::from(mem.read::<u16>(a.wrapping_add(0x616))) >> k & 1;
                ms_status_effect(mem, call, off, i, a, t.at(k).addr, on, slots);
            }
        }
        let look = ms_get_status_look(mem, i);
        mem.write(a.wrapping_add(0x6f8), look);
        let chr: u32 = mem.read(a);
        if look != mem.read::<u32>(a.wrapping_add(0x6fc)) && chr != 0 {
            let f = code(mem, OP_ET_ASTATUS_EFFECT_CHANGE);
            call(mem, f, &[chr, look]);
        }
        mem.write(a.wrapping_add(0x6fc), look);
    }

    /// `MsCheckAutoStatus(i, actor)` (`0x007af260`): when the actor's
    /// automatic statuses changed (`+0x63c` set, `+0x63e` not `+0x63f`):
    /// reset and set again (`MsResetAutoStatus`, `MsSetAutoStatus`) and
    /// the status effects checked ([`ms_status_effect_check`]): 1, else 0.
    pub fn ms_check_auto_status<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        off: &mut dyn FnMut(&mut M, u32),
        i: u32,
        actor: u32,
    ) -> u32 {
        if byte(mem, actor, 0x63c) == 0 || byte(mem, actor, 0x63e) == byte(mem, actor, 0x63f) {
            return 0;
        }
        crate::actor::bridge::ms_reset_auto_status(mem, actor);
        crate::actor::bridge::ms_set_auto_status(mem, actor);
        ms_status_effect_check(mem, call, off, i);
        1
    }

    /// `MsCheckStatusAfterInput()` (`0x007af4b0`): every actor's `+0x6f0`
    /// and `+0x6da` cleared, its battle position back (`MsPopBtlPos`) and
    /// its automatic statuses checked ([`ms_check_auto_status`]); how many
    /// changed.
    pub fn ms_check_status_after_input<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        off: &mut dyn FnMut(&mut M, u32),
    ) -> u32 {
        let mut n = 0u32;
        for i in 0..0x1f {
            let a = ms_get_chr(mem, i);
            mem.write(a.wrapping_add(0x6f0), 0u32);
            mem.write(a.wrapping_add(0x6da), 0u8);
            crate::actor::bridge::ms_pop_btl_pos(mem, a);
            n = n.wrapping_add(ms_check_auto_status(mem, call, off, i, a));
        }
        n
    }

    /// `Ch_ModelSetHide` (chara).
    pub const CH_MODEL_SET_HIDE: u32 = 0x0082_aea0;
    /// The frame `MsResetChrPos` and `MsAttachWeapon` keep their locals in.
    pub const STATUS_FRAME: u32 = 0xff20_0000;

    /// `MsHideArm(i, hide)` (`0x00795c00`): for actor 6 when its `+4` is
    /// 7, its model's part 1 hidden or shown (`Ch_ModelSetHide(handle, 1,
    /// hide)`).
    pub fn ms_hide_arm<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32, hide: u32) {
        if i != 6 {
            return;
        }
        let a = ms_get_chr(mem, 6);
        if mem.read::<u32>(a.wrapping_add(4)) != 7 {
            return;
        }
        let h = crate::actor::bridge::ms_get_chr_handle(mem, 6);
        if h != 0 {
            let f = code(mem, CH_MODEL_SET_HIDE);
            call(mem, f, &[h, 1, hide]);
        }
    }

    /// `MsResetChrPos(i, actor)` (`0x0078d680`): an actor with a character,
    /// no `+0x43f` and `+0x426` and not `+0x4ac` bit 15, off its battle
    /// place (`MsCheckBtlPos(i, actor, +0x34)` with `+0x50d` set for the
    /// check) goes to its character's optional position 6
    /// (`Ch_OptposGet`, `MsSetChrPos`): 1, else 0.
    pub fn ms_reset_chr_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        i: u32,
        actor: u32,
    ) -> u32 {
        let chr: u32 = mem.read(actor);
        if chr == 0
            || byte(mem, actor, 0x43f) != 0xff
            || byte(mem, actor, 0x426) != 0
            || mem.read::<u32>(actor.wrapping_add(0x4ac)) & 0x8000 != 0
        {
            return 0;
        }
        let saved = byte(mem, actor, 0x50d);
        mem.write(actor.wrapping_add(0x50d), 1u8);
        let flat = i32::from(mem.read::<i8>(actor.wrapping_add(0x34))).cast_unsigned();
        let off = crate::pos::bridge::ms_check_btl_pos(mem, trig, i, actor, flat);
        mem.write(actor.wrapping_add(0x50d), saved);
        if off == 0 {
            return 0;
        }
        let mut f = ffx_bridge::frame::Frame::new(mem, STATUS_FRAME, 0x10);
        let at = f.at(0);
        ffx_chara::attach::bridge::ch_optpos_get(&mut f, chr, 6, at);
        crate::body::bridge::ms_set_chr_pos(&mut f, i, at);
        1
    }

    /// `0x0078f040(i, actor)` (`MsSetChrMovePos`, name ours): `+0x425` set
    /// and the actor's position into `+0x3c0` (`MsGetChrPos`, its result).
    pub fn ms_set_chr_move_pos<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(crate::pos::bridge::Trig, u32) -> u32,
        i: u32,
        actor: u32,
    ) -> i32 {
        mem.write(actor.wrapping_add(0x425), 1u8);
        crate::pos::bridge::ms_get_chr_pos(mem, trig, i, actor.wrapping_add(0x3c0))
    }

    /// `MsAttachWeapon(i, k)` (`0x00793330`): actor `i & 0xff`'s weapon `k`
    /// (`MsGetWeapon`), when it and the actor have characters and the
    /// actor `+0x4b5`, bound to the actor's hand (`MsHideArm(i, 1)`,
    /// `Ch_BindToElem(weapon, handle, weapon +0xd)`). A pending
    /// re-attach (`+0x4b6[k]`) is cleared; for actor 5 its special weapon
    /// (`MsGetWeaponSpecial(5, k)`) then faces the target
    /// `MsGetChrStatInfo(5, 0xdb, -13)` gives (`MsSetChrDirTarget(5, 1, 0,
    /// -1, that, 0)`) with `+0x420` 2. 1, else 0.
    pub fn ms_attach_weapon<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        call: Call<'_, Dm<'d>>,
        i: u32,
        k: u32,
    ) -> u32 {
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        let w = crate::actor::bridge::ms_get_weapon(mem, i, k.cast_signed());
        let wchr: u32 = mem.read(w);
        let h = crate::actor::bridge::ms_get_chr_handle(mem, i);
        if wchr == 0 || h == 0 || byte(mem, a, 0x4b5) == 0 {
            return 0;
        }
        ms_hide_arm(mem, call, i, 1);
        let elem = u32::from(byte(mem, w, 0xd));
        ffx_chara::attach::bridge::ch_bind_to_elem(mem, wchr, h, elem);
        let pending = a.wrapping_add(0x4b6).wrapping_add(k);
        if mem.read::<u8>(pending) != 0 {
            mem.write(pending, 0u8);
            if i == 5 {
                let s = crate::actor::bridge::ms_get_weapon_special(mem, 5, k);
                if s != 0 {
                    let t = mem.frame(4, |mem, f| {
                        crate::stat::bridge::ms_get_chr_stat_info(
                            mem,
                            host.trig,
                            5,
                            0xdb,
                            (-13i32).cast_unsigned(),
                            f,
                        );
                        mem.read::<u32>(f)
                    });
                    ms_set_chr_dir_target(mem, host, 5, 1, 0, u32::MAX, t, 0);
                    mem.write(s.wrapping_add(0x420), 2u8);
                }
            }
        }
        1
    }

    /// `MsDetachWeapon(i, k)` (`0x00793f10`): actor `i & 0xff`'s special
    /// weapon `k` (`MsGetWeaponSpecial`) takes the actor's `+0x49c`; its
    /// weapon `k` (`MsGetWeapon`), when it has a character and the actor
    /// `+0x4b5`, is let go where it is: re-attach pending (`+0x4b6[k]`),
    /// the arm shown (`MsHideArm(i, 0)`), unbound (`Ch_BindToElem(w, 0,
    /// 0)`) and placed at the hand's optional position (`Ch_OptposGet` of
    /// the actor's character, weapon `+0xd`) turned as the actor
    /// (`Ch_GetRot`, `Ch_SetDirF`). 1, else 0.
    pub fn ms_detach_weapon<'d>(mem: &mut Dm<'d>, call: Call<'_, Dm<'d>>, i: u32, k: u32) -> u32 {
        use ffx_chara::chr::bridge as ch;
        let i = i & 0xff;
        let a = ms_get_chr(mem, i);
        let special = crate::actor::bridge::ms_get_weapon_special(mem, i, k);
        if special != 0 {
            let v = crate::camera::x87_copy(mem.read(a.wrapping_add(0x49c)));
            mem.write(special.wrapping_add(0x49c), v);
        }
        let w = crate::actor::bridge::ms_get_weapon(mem, i, k.cast_signed());
        let wchr: u32 = mem.read(w);
        let h = crate::actor::bridge::ms_get_chr_handle(mem, i);
        if wchr == 0 || byte(mem, a, 0x4b5) == 0 {
            return 0;
        }
        mem.write(a.wrapping_add(0x4b6).wrapping_add(k), 1u8);
        ms_hide_arm(mem, call, i, 0);
        let elem = u32::from(byte(mem, w, 0xd));
        mem.frame(0x10, |mem, hand| {
            ffx_chara::attach::bridge::ch_optpos_get(mem, h, elem, hand);
            let rot = ch::ch_get_rot(mem, h);
            ffx_chara::attach::bridge::ch_bind_to_elem(mem, wchr, 0, 0);
            let [x, y, z]: [u32; 3] = mem.read(hand);
            let f = |v: u32| f32::from_bits(crate::camera::x87_copy(v));
            ch::ch_set_pos3(mem, wchr, f(x), f(y), f(z));
            ch::ch_set_dir_f(mem, wchr, f(rot.to_bits()));
        });
        1
    }

    /// `MsDamageSetMotion(i, kind, flags)` (`0x0078ca20`): the motion actor
    /// `i`'s mover (`+0x422`) plays after a hit of `kind`, its effects'
    /// motion lock and head target reset and `+0x50e` from `+0x50f`
    /// (`+0x50f`, `+0x44e` cleared), unless `+0x436`. Petrified
    /// (`+0xdce`): `+0x41a` 1, `+0x41c` 0, put back ([`ms_reset_chr_pos`]),
    /// its own hits (`+0xded` itself) re-attach its weapon (actor 5 also
    /// runs actor 0x1d's motion 0), then motion 0x38. Otherwise by `kind`
    /// (1 0xc with its position in `+0x3c0`, unless `+0x4ac` bit 14; 2 and
    /// 7 0x10, put back only without `+0x43e`; 3 0xe; 4 0xf; 5 and 8 9; 6
    /// 0x30; 12 0x1b not put back; 13-16 9 or 0x30 with `+0x44e` 1 or
    /// 0xff; 11 and, after an earlier one (`+0x41a`), the rest motion 0
    /// without waiting) `+0x433` set, 9 and 0x30 becoming 0x43 with
    /// `+0x6da`; put back and the motion set (`MsSetMotion(i, m, wait, 0,
    /// flags)`); `+0x41a` cleared.
    #[allow(clippy::too_many_lines)]
    pub fn ms_damage_set_motion<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        kind: u32,
        flags: u32,
    ) {
        let trig = host.trig;
        let a0 = ms_get_chr(mem, i & 0xff);
        let who = i32::from(mem.read::<i8>(a0.wrapping_add(0x422))).cast_unsigned();
        let a = ms_get_chr(mem, who);
        crate::minor::bridge::ms_effect_reset_motion_disable(mem, a);
        ms_reset_head_target(mem, &mut *s.call, a);
        let v = byte(mem, a, 0x50f);
        mem.write(a.wrapping_add(0x50e), v);
        mem.write(a.wrapping_add(0x50f), 0u8);
        mem.write(a.wrapping_add(0x44e), 0u8);
        if byte(mem, a, 0x436) != 0 {
            return;
        }
        if byte(mem, a, 0xdce) != 0 {
            mem.write(a.wrapping_add(0x41a), 1u8);
            mem.write(a.wrapping_add(0x41c), 0u8);
            ms_reset_chr_pos(mem, trig, who, a);
            if i32::from(mem.read::<i8>(a.wrapping_add(0xded))).cast_unsigned() == who {
                if who == 5 {
                    ms_set_motion(mem, s, 0x1d, 0, 1, 0, flags, 0, 0);
                }
                ms_attach_weapon(mem, host, &mut *s.call, who, 0);
            }
            ms_set_motion(mem, s, who, 0x38, 1, 0, flags, 0, 0);
            mem.write(a.wrapping_add(0x508), 0u8);
            return;
        }
        let (mut m, mut wait, mut back) = (-1i32, 1u32, true);
        let mut hit = |mem: &mut Dm<'d>, motion: i32| {
            mem.write(a.wrapping_add(0x433), 1u8);
            m = motion;
        };
        match kind {
            1 => {
                if mem.read::<u32>(a.wrapping_add(0x4ac)) & 0x4000 == 0 {
                    mem.write(a.wrapping_add(0x433), 1u8);
                    let d = byte(mem, a, 0xded);
                    mem.write(a.wrapping_add(0xdef), d);
                    ms_set_chr_move_pos(mem, trig, who, a);
                    m = 0xc;
                }
            }
            2 | 7 => {
                hit(mem, 0x10);
                back = byte(mem, a, 0x43e) == 0;
            }
            3 => hit(mem, 0xe),
            4 => hit(mem, 0xf),
            5 | 8 => hit(mem, 9),
            6 => hit(mem, 0x30),
            12 => {
                hit(mem, 0x1b);
                back = false;
            }
            13 | 14 => {
                hit(mem, 9);
                mem.write(a.wrapping_add(0x44e), if kind == 13 { 1u8 } else { 0xff });
            }
            15 | 16 => {
                hit(mem, 0x30);
                mem.write(a.wrapping_add(0x44e), if kind == 15 { 1u8 } else { 0xff });
            }
            11 => {
                m = 0;
                wait = 0;
            }
            _ => {
                if byte(mem, a, 0x41a) != 0 {
                    m = 0;
                    wait = 0;
                }
            }
        }
        if byte(mem, a, 0x6da) != 0 && (m == 9 || m == 0x30) {
            m = 0x43;
        }
        if m >= 0 {
            if back {
                ms_reset_chr_pos(mem, trig, who, a);
            }
            ms_set_motion(mem, s, who, m, wait, 0, flags, 0, 0);
        }
        mem.write(a.wrapping_add(0x41a), 0u8);
    }

    /// `MsCheckStatusAfterAction(i, actor)` (`0x007af390`): an able actor's
    /// (`+0xdc8`) counted statuses (`+0x608`, 13, between 1 and 0xfd)
    /// that count down (table flags bit 2) go down by one, their effects
    /// updated (flags bit 3, unless `+0xdcb`: [`ms_status_effect`] with the
    /// new count). Sleep (`+0x608`) running out wakes it (`+0x50e` 1,
    /// `+0x433` set, `MsSetMotion(i, 0x1b, 1, 0, 1)`); else `+0x609` or
    /// `+0x60a` running out plays the hit motion when it can
    /// (`MsCheckSetMotion`, [`ms_damage_set_motion`] kind 2). 0.
    pub fn ms_check_status_after_action<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
    ) -> u32 {
        if byte(mem, actor, 0xdc8) == 0 {
            return 0;
        }
        let before = [
            byte(mem, actor, 0x608),
            byte(mem, actor, 0x609),
            byte(mem, actor, 0x60a),
        ];
        let t = STATUS_COUNT_EFFECTS.ptr(mem);
        for k in 0..13 {
            let at = actor.wrapping_add(0x608 + k);
            let c = mem.read::<u8>(at);
            let rec = t.at(k).addr;
            let fl = byte(mem, rec, 3);
            if (1..=0xfd).contains(&c) && fl & 4 != 0 {
                let c = c - 1;
                mem.write(at, c);
                if fl & 8 != 0 && byte(mem, actor, 0xdcb) == 0 {
                    ms_status_effect(
                        mem,
                        &mut *s.call,
                        &mut *s.off,
                        i,
                        actor,
                        rec,
                        u32::from(c),
                        actor.wrapping_add(0xe30),
                    );
                }
            }
        }
        if before[0] != 0 && byte(mem, actor, 0x608) == 0 {
            mem.write(actor.wrapping_add(0x50e), 1u16);
            mem.write(actor.wrapping_add(0x433), 1u8);
            ms_set_motion(mem, s, i, 0x1b, 1, 0, 1, 0, 0);
            return 0;
        }
        let woke = (before[1] != 0 && byte(mem, actor, 0x609) == 0)
            || (before[2] != 0 && byte(mem, actor, 0x60a) == 0);
        if woke && crate::actor::bridge::ms_check_set_motion(mem, actor, 0) != 0 {
            ms_damage_set_motion(mem, host, s, i, 2, 1);
        }
        0
    }
}
