//! Party members and aeons coming in and going out (`docs/rewrite-plan.md`,
//! M2): `MsExchangeChr` swaps one actor for another (the leaving one's
//! turn, threat, sounds and weapon effect ended, the coming one placed and
//! its models queued for reading), `MsExchangeSummon` calls an aeon in for
//! the whole party, and `MsBtlReadCueManage` works through the model read
//! queue those fill.
//!
//! The read queue's loads are other modules' (chara's model reads) or not
//! yet ported (the battle's model registration); those, and vfx's
//! `op_et_weapon_free`, go to the game's code through `s.call`.

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, vectors
pub mod bridge {
    use ffx_bridge::{Global, IMAGE_BASE, Mem};

    use crate::actor::bridge::{ms_get_chr, ms_get_ram_chr_monster};
    use crate::atel::bridge::{Script, ms_set_motion};
    use crate::target::bridge::{Dm, Host, Regs};
    use crate::vector::bridge as v;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn code(mem: &impl Mem, sym: u32) -> u32 {
        sym.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base())
    }

    fn quiet(bits: u32) -> u32 {
        ffx_bridge::x87::store(ffx_bridge::x87::load(bits))
    }

    // The model read queue: 32 entries of 12 bytes (`+0` kind, `+1` actor,
    // `+2` state, `+3` what to do when read, `+4` owner, `+6` the file's
    // `u16` ID, `+8` a word), the first one and the count.
    use crate::misc::bridge::{READ_CUE, READ_CUE_INDEX as READ_CUE_HEAD};
    use crate::state::bridge::READ_CUE_LEVEL as READ_CUE_COUNT;
    /// Weapon models that come with a second, bound model
    /// (`0x007ad130`): `u16` IDs ending at 0.
    pub const BOUND_WEAPONS: Global<u16> = Global::new(0x00c4_3984);
    /// The scale `MsCalcChrDir` moves the party's centre by.
    pub const CENTER_SCALE: Global<u32> = Global::new(0x0112_bde4);
    // The party (7 slots) and the reserve (17 slots) the exchanges swap
    // in, and as they were before an aeon came.
    use crate::flow::bridge::{
        PARTY_EXTRA as RESERVE, PARTY_EXTRA_SAVED as RESERVE_SAVED, PARTY_SAVED,
    };
    use crate::menu::bridge::PARTY;

    /// The game's functions the read queue and the exchanges call.
    pub const CH_DATA_READ: u32 = 0x0082_9ef0;
    pub const CH_DATA_READ_SYNC: u32 = 0x0082_a040;
    pub const CHN_READ_SYSTEM_MGRP: u32 = 0x0083_6870;
    pub const CH_MGRP_READ: u32 = 0x0083_6a40;
    pub const CHN_READ_SYSTEM_MGRP_SYNC: u32 = 0x0083_6a50;
    pub const MS_BTL_READ_PLAYER_MODEL_PROCESS: u32 = 0x0078_3b90;
    pub const MS_BTL_READ_PLAYER_PROCESS: u32 = 0x0078_3bc0;
    pub const MS_BTL_READ_MONSTER_PROCESS: u32 = 0x0078_39b0;
    pub const MS_CD_READ_WEAPON: u32 = 0x0078_4120;
    pub const MS_BTL_WEAPON_REGIST: u32 = 0x0079_38e0;
    pub const OP_ET_WEAPON_FREE: u32 = 0x007f_ca90;

    /// `MsGetSaveWeaponModel(id)` (`0x007abdc0`): the model (`+0xc`) of
    /// saved weapon `id` (`MsGetSaveWeapon`), 0 for none or 0xff.
    pub fn ms_get_save_weapon_model(mem: &mut impl Mem, id: u32) -> u32 {
        if id == 0xff {
            return 0;
        }
        let w = crate::save::bridge::ms_get_save_weapon(mem, id, 0);
        if w == 0 {
            return 0;
        }
        u32::from(mem.read::<u16>(w.wrapping_add(0xc)))
    }

    /// `0x007ad130(id)` (`MsCheckBoundWeapon`, name ours): whether weapon
    /// model `id` is in [`BOUND_WEAPONS`].
    pub fn ms_check_bound_weapon(mem: &impl Mem, id: u32) -> u32 {
        let mut at = BOUND_WEAPONS.ptr(mem).addr;
        loop {
            let m = mem.read::<u16>(at);
            if m == 0 {
                return 0;
            }
            if u32::from(m) == id {
                return 1;
            }
            at = at.wrapping_add(2);
        }
    }

    /// `MsBtlReadCueManage()` (`0x007ad190`): works through the read queue:
    /// an entry's read is started (state 0: kind 0 a character's data
    /// `Ch_DataRead(id)` unless the actor has its character already, kind
    /// 2 its motion group `Ch_MgrpRead(id, +0x20)` when the actor has
    /// `+0x11`, kind 3 its weapon `MsCDreadWeapon(actor, p, 2)`), waited
    /// for (state 1: `Ch_DataReadSync`, `ChN_ReadSystemMGRPsync`; kinds 1
    /// and 3 stop the queue there) and finished (state 2: kind 2 with an ID
    /// `ChN_ReadSystemMGRP(id, +0x20)`; then by `+3`: 1 the player's model,
    /// 2, 6, 8, 10 the player, 3 and 7 the monster processes, 4 and 5 the
    /// owner's weapon registered with its `+0xdfd`, which clears). Returns
    /// while a read is still running, else when the queue is empty. (Kinds
    /// 1 and above 3 in state 0, kinds above 3 in state 1 and states above
    /// 2 make the original spin for ever; here too.)
    pub fn ms_btl_read_cue_manage<'d>(mem: &mut Dm<'d>, s: &mut Script<'_, Dm<'d>>) {
        while READ_CUE_COUNT.get(mem) != 0 {
            let h = u32::from(READ_CUE_HEAD.get(mem));
            let e = READ_CUE.ptr(mem).addr + 12 * h;
            let chr = u32::from(byte(mem, e, 1));
            let c = ms_get_chr(mem, chr);
            let kind = byte(mem, e, 0);
            let id = u32::from(mem.read::<u16>(e + 6));
            let set = |mem: &mut Dm<'d>, st: u8| mem.write(e + 2, st);
            let grp = i32::from(mem.read::<i8>(c.wrapping_add(0x20))).cast_unsigned();
            match byte(mem, e, 2) {
                0 => match kind {
                    0 => {
                        if mem.read::<u32>(c) != 0 {
                            set(mem, 2);
                        } else {
                            let f = code(mem, CH_DATA_READ);
                            (s.call)(mem, f, &[id]);
                            set(mem, 1);
                        }
                    }
                    2 => {
                        if byte(mem, c, 0x11) != 0 && id != 0 {
                            let f = code(mem, CH_MGRP_READ);
                            (s.call)(mem, f, &[id, grp]);
                            set(mem, 1);
                        } else {
                            set(mem, 2);
                        }
                    }
                    3 => {
                        let f = code(mem, MS_CD_READ_WEAPON);
                        if (s.call)(mem, f, &[chr, c, 2]) != 0 {
                            set(mem, 2);
                        } else {
                            set(mem, 1);
                            return;
                        }
                    }
                    _ => {}
                },
                1 => {
                    let r = match kind {
                        0 => {
                            let f = code(mem, CH_DATA_READ_SYNC);
                            (s.call)(mem, f, &[id])
                        }
                        2 => {
                            let f = code(mem, CHN_READ_SYSTEM_MGRP_SYNC);
                            (s.call)(mem, f, &[id, grp])
                        }
                        1 | 3 => return,
                        _ => continue,
                    };
                    if r != 0 {
                        return;
                    }
                    set(mem, 2);
                }
                2 => {
                    if kind == 2 && id != 0 {
                        let f = code(mem, CHN_READ_SYSTEM_MGRP);
                        (s.call)(mem, f, &[id, grp]);
                    }
                    let owner = u32::from(byte(mem, e, 4));
                    let t = ms_get_chr(mem, owner);
                    let sub = u32::from(byte(mem, e, 3));
                    match sub {
                        1 => {
                            let f = code(mem, MS_BTL_READ_PLAYER_MODEL_PROCESS);
                            (s.call)(mem, f, &[chr]);
                        }
                        2 | 6 | 8 | 10 => {
                            let f = code(mem, MS_BTL_READ_PLAYER_PROCESS);
                            (s.call)(mem, f, &[chr, sub]);
                        }
                        3 | 7 => {
                            let f = code(mem, MS_BTL_READ_MONSTER_PROCESS);
                            (s.call)(mem, f, &[chr, sub]);
                        }
                        4 | 5 => {
                            let dfd =
                                i32::from(mem.read::<i8>(t.wrapping_add(0xdfd))).cast_unsigned();
                            let f = code(mem, MS_BTL_WEAPON_REGIST);
                            (s.call)(mem, f, &[owner, dfd, sub - 4]);
                            mem.write(t.wrapping_add(0xdfd), 0u8);
                        }
                        _ => {}
                    }
                    let n = READ_CUE_COUNT.get(mem).wrapping_sub(1);
                    READ_CUE_COUNT.set(mem, n);
                    READ_CUE_HEAD.set(mem, ((h + 1) & 0x1f).to_le_bytes()[0]);
                    if n == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
    }

    /// `MsBtlReadCueRegist(kind, chr, owner, id, then)` (`0x007ad3f0`): a
    /// read queued (unless the queue is full) and the queue worked
    /// ([`ms_btl_read_cue_manage`]).
    #[allow(clippy::too_many_arguments)]
    pub fn ms_btl_read_cue_regist<'d>(
        mem: &mut Dm<'d>,
        s: &mut Script<'_, Dm<'d>>,
        kind: u32,
        chr: u32,
        owner: u32,
        id: u32,
        then: u32,
    ) {
        let n = READ_CUE_COUNT.get(mem);
        if n >= 0x20 {
            return;
        }
        let k = (u32::from(READ_CUE_HEAD.get(mem)) + u32::from(n)) & 0x1f;
        let e = READ_CUE.ptr(mem).addr + 12 * k;
        let b = |v: u32| v.to_le_bytes()[0];
        mem.write(e, b(kind));
        mem.write(e + 1, b(chr));
        mem.write(e + 4, b(owner));
        mem.write(
            e + 6,
            u16::from_le_bytes([id.to_le_bytes()[0], id.to_le_bytes()[1]]),
        );
        mem.write(e + 2, 0u8);
        mem.write(e + 3, b(then));
        mem.write(e + 8, 0u32);
        READ_CUE_COUNT.set(mem, n + 1);
        ms_btl_read_cue_manage(mem, s);
    }

    /// `MsBtlReadCueRegistWeapon(i, which)` (`0x007ad470`): actor `i`'s
    /// weapons in `which` (bits 0, 1; saved weapon `+0x592[k]`) with a
    /// model queued as weapon actor `i + 0x1f(k + 1)`: a bound model's
    /// (`0x007ad130`, the weapon record's `+8` its ID) data and motion
    /// group (then 4 + `k`), any other's data (then 4 + `k`, `+8` 0).
    pub fn ms_btl_read_cue_regist_weapon<'d>(
        mem: &mut Dm<'d>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        which: u32,
    ) {
        let a = ms_get_chr(mem, i);
        for k in 0..2u32 {
            if which & (1 << k) == 0 {
                continue;
            }
            let id = ms_get_save_weapon_model(mem, u32::from(byte(mem, a, 0x592 + k)));
            if id == 0 {
                continue;
            }
            let w = crate::actor::bridge::ms_get_weapon(mem, i, k.cast_signed());
            let chr = i + 0x1f * (k + 1);
            if ms_check_bound_weapon(mem, id) != 0 {
                mem.write(w.wrapping_add(8), id);
                ms_btl_read_cue_regist(mem, s, 0, chr, i, id, 0);
                ms_btl_read_cue_regist(mem, s, 2, chr, i, id, k + 4);
            } else {
                mem.write(w.wrapping_add(8), 0u32);
                ms_btl_read_cue_regist(mem, s, 0, chr, i, id, k + 4);
            }
        }
    }

    /// `MsEffectWeaponFree(i)` (`0x00788b30`): the actor's weapon effect
    /// (`+0x4c`, while `+0x35`) ends (`op_et_weapon_free`), both cleared.
    pub fn ms_effect_weapon_free<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, i: u32) {
        let a = ms_get_chr(mem, i);
        let e: u32 = mem.read(a.wrapping_add(0x4c));
        if e == 0 || byte(mem, a, 0x35) == 0 {
            return;
        }
        let f = code(mem, OP_ET_WEAPON_FREE);
        (s.call)(mem, f, &[e]);
        mem.write(a.wrapping_add(0x4c), 0u32);
        mem.write(a.wrapping_add(0x35), 0u8);
    }

    /// `0x0078f6a0(i, actor)` (`MsThreatEndMotion`, name ours): an actor
    /// able to (not KO'd, `+0x606` bit 2, `+0x617` bit 0, `MsCheckSetMotion`)
    /// goes back to its rest motion (`+0x41c` set, `MsSetMotion(i, 0, 1, 0,
    /// 1)`).
    pub fn ms_threat_end_motion<M: Mem>(mem: &mut M, s: &mut Script<'_, M>, i: u32, actor: u32) {
        if byte(mem, actor, 0xdcc) != 0
            || byte(mem, actor, 0x606) & 4 != 0
            || byte(mem, actor, 0x617) & 1 != 0
            || crate::actor::bridge::ms_check_set_motion(mem, actor, 0) == 0
        {
            return;
        }
        mem.write(actor.wrapping_add(0x41c), 1u8);
        ms_set_motion(mem, s, i, 0, 1, 0, 1, 0, 0);
    }

    /// `MsThreatProcess(i, actor)` (`0x0078e460`): with Threaten on
    /// (`+0x606` bit 11), the actor faces its target `+0x5c5`'s way and that
    /// one (if any) is threatened back (`+0x606` bit 11, `+0x5c6` `i`)
    /// facing it: 1. (The actor's own yaw is from an uninitialised local to
    /// its battle position in the original: stack garbage; here the zero
    /// vector.) Without it, both (`+0x5c5`, else `+0x5c6`) are released:
    /// `+0x5c5`, `+0x5c6` 0xff, direction mode reset, rest motion
    /// ([`ms_threat_end_motion`]): 0.
    pub fn ms_threat_process<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
    ) -> u32 {
        use crate::minor::bridge::ms_set_dir_theta_mode;
        use crate::pos::bridge::ms_get_chr_btl_pos;
        let trig = host.trig;
        let mut t = u32::from(byte(mem, actor, 0x5c5));
        if mem.read::<u16>(actor.wrapping_add(0x606)) & 0x800 != 0 {
            let o = ms_get_chr(mem, t);
            mem.frame(0x20, |mem, f| {
                let (p, q) = (f, f + 0x10);
                ms_get_chr_btl_pos(mem, trig, i, p);
                v::ms_vector_rect2_polar(mem, trig, q, p, actor.wrapping_add(0x51c), 0, 0);
                ms_set_dir_theta_mode(mem, actor);
                if t != 0xff {
                    ms_get_chr_btl_pos(mem, trig, t, q);
                    v::ms_vector_rect2_polar(mem, trig, p, q, o.wrapping_add(0x51c), 0, 0);
                    let w: u16 = mem.read(o.wrapping_add(0x606));
                    mem.write(o.wrapping_add(0x606), w | 0x800);
                    mem.write(o.wrapping_add(0x5c6), i.to_le_bytes()[0]);
                    ms_set_dir_theta_mode(mem, o);
                }
            });
            return 1;
        }
        if t == 0xff {
            t = u32::from(byte(mem, actor, 0x5c6));
        }
        let o = ms_get_chr(mem, t);
        mem.write(actor.wrapping_add(0x5c5), 0xffffu16);
        ms_set_dir_theta_mode(mem, actor);
        ms_threat_end_motion(mem, s, i, actor);
        if t != 0xff {
            let w: u16 = mem.read(o.wrapping_add(0x606));
            mem.write(o.wrapping_add(0x606), w & 0xf7ff);
            mem.write(o.wrapping_add(0x5c5), 0xffffu16);
            ms_set_dir_theta_mode(mem, o);
            ms_threat_end_motion(mem, s, t, o);
        }
        0
    }

    /// `MsThreatOff(i, actor)` (`0x0078e410`): Threaten released: the
    /// threatened one's (`+0x5c6`, else the actor itself) flag (`+0x606`
    /// bit 11) cleared and its [`ms_threat_process`].
    pub fn ms_threat_off<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        i: u32,
        actor: u32,
    ) {
        if mem.read::<u16>(actor.wrapping_add(0x606)) & 0x800 == 0 {
            return;
        }
        let t = u32::from(byte(mem, actor, 0x5c6));
        let (k, o) = if t == 0xff {
            (i, actor)
        } else {
            (t, ms_get_chr(mem, t))
        };
        let w: u16 = mem.read(o.wrapping_add(0x606));
        mem.write(o.wrapping_add(0x606), w & 0xf7ff);
        ms_threat_process(mem, host, s, k, o);
    }

    /// `0x00794b70(first, n, out)` (`MsGetChrCenter`, name ours): the mean
    /// battle position of the able (`+0xdc8`) actors among `n` from
    /// `first & 0xff` into `out` (`r0`, so zeros, when none); the divide's
    /// other words are its caller's `edi`, `esi`, `ebx` (`regs`, else the
    /// register words as they were).
    pub fn ms_get_chr_center(
        mem: &mut Dm<'_>,
        host: &Host<'_>,
        first: u32,
        n: i32,
        out: u32,
        regs: Option<Regs>,
    ) {
        let r0 = crate::target::bridge::EE_R0.get(mem);
        mem.write(out, r0);
        let mut i = first & 0xff;
        let mut count = 0i32;
        mem.frame(0x10, |mem, p| {
            for _ in 0..n.max(0) {
                if byte(mem, ms_get_chr(mem, i), 0xdc8) != 0 {
                    crate::pos::bridge::ms_get_chr_btl_pos(mem, host.trig, i, p);
                    v::ms_add_point(mem, out, out, p);
                    count += 1;
                }
                i += 1;
            }
        });
        if count > 0 {
            #[allow(clippy::cast_precision_loss)] // a count of actors
            let f = (count as f32).to_bits();
            let w = match regs {
                Some(r) => [f, r.edi, r.esi, r.ebx],
                None => {
                    let mut w: [u32; 4] = v::VF_B.get(mem);
                    w[0] = f;
                    w
                }
            };
            v::ms_div_vector(mem, out, w);
        }
    }

    /// `MsCalcChrDir(i, actor)` (`0x00793ba0`): the direction actor `i`
    /// faces the other side from: a monster from the monsters' centre moved
    /// [`CENTER_SCALE`] of the way towards the party's centre, a party
    /// member the other way round, to its battle position
    /// (`MsVectorRect2Polar`, as `MsNearRag(0, yaw)`). The scale's other
    /// words are its caller's `esi`, an uninitialised local (left as the
    /// register had it) and the difference's x. An `f32` in ST0.
    pub fn ms_calc_chr_dir(mem: &mut Dm<'_>, host: &Host<'_>, i: u32, regs: Option<Regs>) -> u32 {
        let trig = host.trig;
        mem.frame(0x40, |mem, f| {
            let (party, monsters, d, yaw) = (f, f + 0x10, f + 0x20, f + 0x30);
            ms_get_chr_center(mem, host, 0, 0x12, party, regs);
            ms_get_chr_center(mem, host, 0x14, 8, monsters, regs);
            let (from, to) = if ms_get_ram_chr_monster(i) != 0 {
                (party, monsters)
            } else {
                (monsters, party)
            };
            v::ms_sub_point(mem, d, from, to);
            let k = CENTER_SCALE.get(mem);
            let old: [u32; 4] = v::VF_B.get(mem);
            let esi = regs.map_or(old[1], |r| r.esi);
            let dx: u32 = mem.read(d);
            v::ms_mul_vector(mem, d, [k, esi, old[2], dx]);
            v::ms_add_point(mem, from, from, d);
            crate::pos::bridge::ms_get_chr_btl_pos(mem, trig, i, to);
            v::ms_vector_rect2_polar(mem, trig, from, to, yaw, 0, 0);
            let y: u32 = mem.read(yaw);
            v::ms_near_rag(mem, 0, y).to_bits()
        })
    }

    /// `MsChangeMonsterDir()` (`0x00793cf0`): every present monster
    /// (0x14-0x1b: `+0x10`, not `+0xdcd`, able `+0xdc8` or `+0xdca`, not
    /// KO'd, not `+0x512`) gets its direction mode reset, `+0x41c` set,
    /// with free directions (`g_BtlDirFree`) its battle direction `+0x518`
    /// towards the party ([`ms_calc_chr_dir`]), and its rest motion
    /// (`MsSetMotion(i, 0, 0, 0, 1)`).
    pub fn ms_change_monster_dir<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
    ) {
        let free = i32::from(crate::minor::bridge::DIR_FREE.get(mem).cast_signed());
        for i in 0x14..0x1c {
            let a = ms_get_chr(mem, i);
            if byte(mem, a, 0x10) == 0
                || byte(mem, a, 0xdcd) != 0
                || (byte(mem, a, 0xdc8) == 0 && byte(mem, a, 0xdca) == 0)
                || byte(mem, a, 0xdcc) != 0
                || byte(mem, a, 0x512) != 0
            {
                continue;
            }
            crate::minor::bridge::ms_set_dir_theta_mode(mem, a);
            mem.write(a.wrapping_add(0x41c), 1u8);
            if free != 0 {
                let regs = Regs {
                    edi: i,
                    esi: a,
                    ebx: free.cast_unsigned(),
                };
                let d = ms_calc_chr_dir(mem, host, i, Some(regs));
                mem.write(a.wrapping_add(0x518), d);
            }
            ms_set_motion(mem, s, i, 0, 0, 0, 1, 0, 0);
        }
    }

    /// `MsExchangeChr(out, in, mode, after)` (`0x007adae0`): actor `out`
    /// (unless 0xff or absent) leaves, `in` (unless 0xff or absent) comes.
    /// The leaving one keeps its place (`+0x500` from `+0x509`); unless out
    /// of the battle (`+0xdcd`) it can no longer act (`+0xdc8` 0, `+0xdf8`
    /// 1, `+0x41c` 0), its provoke and threat end (`MsProvOff`,
    /// [`ms_threat_off`]), its input entries go
    /// ([`crate::action::bridge::ms_clean_input_cue`]) and it walks off:
    /// for an aeon (`mode` > 0) as the summoner (`+0xdcb`, `+0x1a`, weight,
    /// `g_BtlSummoner` `in`, position kind 0xc, `+0x518` from `in`'s, motion
    /// 3), else escaping (`+0x4fd` up 3, motion 0x12); its weapon effect and
    /// sounds end and `in` takes its party slot (and it `in`'s reserve
    /// slot). The coming one keeps out of the battle where it is
    /// (`+0xdcd`: then 0xa), takes the leaving one's place (`mode` 0 or
    /// below), can act; an aeon (`mode` > 0) its gauge kept
    /// (`MsSummonLimitSave`, then 8), a party member counts the battle; with
    /// `mode` below 0 it appears at once (then 0; body hit, direction, status
    /// effects), else walks in (then 6, `+0x3d`). Its data, motion group
    /// and weapons are queued for reading; with `out` and `mode` not below 0
    /// it gets its turn after `out`'s (`MsInsertInputCue(in, 0, 1, 0, out,
    /// after)`, `+0x716`). Returns `in`.
    #[allow(clippy::too_many_lines)]
    pub fn ms_exchange_chr<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        out: u32,
        inn: u32,
        mode: i32,
        after: u32,
    ) -> u32 {
        use crate::actor::bridge::ms_push_btl_pos;
        use crate::minor::bridge::ms_set_dir_theta_mode;
        let a = ms_get_chr(mem, out);
        let b = ms_get_chr(mem, inn);
        let mut leaving = None;
        if out != 0xff && byte(mem, a, 0x10) != 0 {
            leaving = Some(a);
            let (x, y, z) = (
                byte(mem, a, 0x509),
                byte(mem, a, 0x50a),
                byte(mem, a, 0x50b),
            );
            mem.write(a.wrapping_add(0x501), y);
            mem.write(a.wrapping_add(0x500), x);
            mem.write(a.wrapping_add(0x502), z);
            if byte(mem, a, 0xdcd) == 0 {
                mem.write(a.wrapping_add(0xdc8), 0u8);
                mem.write(a.wrapping_add(0xdf8), 1u8);
                mem.write(a.wrapping_add(0x41c), 0u8);
                crate::minor::bridge::ms_prov_off(mem, out);
                ms_threat_off(mem, host, s, out, a);
                crate::action::bridge::ms_clean_input_cue(mem, host, s, out, 1);
                let motion = if mode > 0 {
                    mem.write(a.wrapping_add(0xdcb), 1u8);
                    mem.write(a.wrapping_add(0x1a), 1u8);
                    crate::body::bridge::ms_set_chr_weight(mem, out, -1);
                    let ib = inn.to_le_bytes()[0];
                    crate::target::bridge::SUMMONER.set(mem, ib.cast_signed());
                    crate::ability::bridge::LIST_SLOT.set(mem, out.to_le_bytes()[0].cast_signed());
                    let g = byte(mem, b, 0x4fc);
                    mem.write(a.wrapping_add(0x4fc), g);
                    mem.write(a.wrapping_add(0x4fd), 0xcu8);
                    mem.write(a.wrapping_add(0x4fe), ib);
                    ms_set_dir_theta_mode(mem, a);
                    if inn != 0xff {
                        let k = i32::from(mem.read::<i8>(b.wrapping_add(0x511)));
                        let d = quiet(
                            mem.read(b.wrapping_add(0x514).wrapping_add((k * 4).cast_unsigned())),
                        );
                        mem.write(a.wrapping_add(0x518), d);
                    }
                    3
                } else {
                    let v = byte(mem, a, 0x4fd).wrapping_add(3);
                    mem.write(a.wrapping_add(0x4fd), v);
                    mem.write(a.wrapping_add(0x436), 0u8);
                    0x12
                };
                ms_set_motion(mem, s, out, motion, 1, 1, 1, 0, 0);
                ms_push_btl_pos(mem, a);
            }
            ms_effect_weapon_free(mem, s, out);
            crate::exec::bridge::ms_sep_chr_clear(mem, &mut *s.call, a, 1);
            let party = PARTY.ptr(mem).addr;
            if let Some(k) = (0..7).find(|&k| u32::from(byte(mem, party, k)) == out) {
                mem.write(party + k, inn.to_le_bytes()[0]);
            }
            let reserve = RESERVE.ptr(mem).addr;
            if let Some(k) = (0..0x11).find(|&k| u32::from(byte(mem, reserve, k)) == inn) {
                mem.write(reserve + k, out.to_le_bytes()[0]);
            }
        }
        if inn == 0xff || byte(mem, b, 0x10) == 0 {
            return inn;
        }
        let data: u32 = mem.read(b.wrapping_add(4));
        let motions: u32 = mem.read(b.wrapping_add(8));
        let then = if byte(mem, b, 0xdcd) != 0 {
            ms_push_btl_pos(mem, b);
            0xa
        } else {
            if let (true, Some(a)) = (mode <= 0, leaving) {
                for o in [0x4fc, 0x4fd, 0x4fe] {
                    let v = byte(mem, a, o + 4);
                    mem.write(b.wrapping_add(o), v);
                }
                let v = byte(mem, a, 0x4ff);
                mem.write(b.wrapping_add(0x4ff), v);
                for o in [0x514, 0x518] {
                    let v = quiet(mem.read(a.wrapping_add(o)));
                    mem.write(b.wrapping_add(o), v);
                }
                let v = byte(mem, a, 0x3f8);
                mem.write(b.wrapping_add(0x3f8), v);
                mem.write(b.wrapping_add(0x65c), 0u8);
            }
            ms_push_btl_pos(mem, b);
            mem.write(b.wrapping_add(0xdc8), 1u8);
            mem.write(b.wrapping_add(0xdf8), 0u8);
            if mode > 0 {
                crate::misc::bridge::ms_summon_limit_save(mem, b);
                mem.write(b.wrapping_add(0x1e), 0u8);
                mem.write(b.wrapping_add(0x43d), 4u8);
                8
            } else {
                let counts = crate::flow::bridge::COUNTS_BATTLE.ptr(mem).addr;
                mem.write(counts.wrapping_add(inn), 1u8);
                if mode < 0 {
                    mem.write(b.wrapping_add(0xdcb), 0u8);
                    mem.write(b.wrapping_add(0x1a), 0u8);
                    crate::body::bridge::ms_set_chr_body_hit(mem, b, 3, 1);
                    ms_set_dir_theta_mode(mem, b);
                    crate::status::bridge::ms_status_effect_check(
                        mem,
                        &mut *s.call,
                        &mut *s.off,
                        inn,
                    );
                    0
                } else {
                    mem.write(b.wrapping_add(0x3d), 1u8);
                    6
                }
            }
        };
        ms_btl_read_cue_regist(mem, s, 3, inn, inn, 0, 0);
        ms_btl_read_cue_regist(mem, s, 0, inn, inn, data, 0);
        ms_btl_read_cue_regist(mem, s, 2, inn, inn, motions, then);
        ms_btl_read_cue_regist_weapon(mem, s, inn, 3);
        if out != 0xff && mode >= 0 {
            mem.write(b.wrapping_add(0x716), 1u8);
            crate::cue::bridge::ms_insert_input_cue(mem, inn, 0, 1, 0, out, after.to_le_bytes()[0]);
        }
        inn
    }

    /// `MsExchangeSummon(in, summon)` (`0x007adf90`): aeon `summon` called
    /// by `in`: directions free (`g_BtlDirFree`), every actor's ability
    /// (`+0xdc8`) and the party and reserve kept, the rest of the party
    /// leaves ([`ms_exchange_chr`] with nobody coming), the aeon (its
    /// summoner `+0x6d1` `in`) comes for `in`; the magus sisters
    /// (0xf-0x11) come all three, filling the party's first slots. The
    /// monsters turn ([`ms_change_monster_dir`]) and the aeon gets its first
    /// command (`MsInsertChrCommand(summon, in, 0)`).
    pub fn ms_exchange_summon<'d>(
        mem: &mut Dm<'d>,
        host: &Host<'_>,
        s: &mut Script<'_, Dm<'d>>,
        inn: u32,
        summon: u32,
    ) {
        crate::minor::bridge::DIR_FREE.set(mem, 1);
        for i in 0..0x12 {
            let a = ms_get_chr(mem, i);
            let v = byte(mem, a, 0xdc8);
            mem.write(a.wrapping_add(0xdc9), v);
        }
        let (reserve, saved) = (RESERVE.ptr(mem).addr, RESERVE_SAVED.ptr(mem).addr);
        for k in 0..0x11 {
            let v = byte(mem, reserve, k);
            mem.write(saved + k, v);
        }
        let (party, saved) = (PARTY.ptr(mem).addr, PARTY_SAVED.ptr(mem).addr);
        for k in 0..7 {
            let m = byte(mem, party, k);
            mem.write(saved + k, m);
            let m = u32::from(m);
            if m != inn && m != 0xff {
                ms_exchange_chr(mem, host, s, m, 0xff, 0, 0);
            }
        }
        let c = ms_get_chr(mem, summon);
        mem.write(c.wrapping_add(0x6d1), inn.to_le_bytes()[0]);
        ms_exchange_chr(mem, host, s, inn, summon, 1, 0);
        if (0xf..=0x11).contains(&summon) {
            for k in 0xf..=0x11u32 {
                mem.write(party + k - 0xf, k.to_le_bytes()[0]);
                if k != summon {
                    let c = ms_get_chr(mem, k);
                    mem.write(c.wrapping_add(0x6d1), inn.to_le_bytes()[0]);
                    ms_exchange_chr(mem, host, s, 0xff, k, 1, 0);
                }
            }
        }
        ms_change_monster_dir(mem, host, s);
        crate::action::bridge::ms_insert_chr_command(mem, summon, inn, 0);
    }
}
