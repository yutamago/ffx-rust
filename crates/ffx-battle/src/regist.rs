//! Registering the actors' characters (`docs/rewrite-plan.md`, M2): an
//! actor's model read, its character made (`Ch_Allocate`) or kept and set
//! up from the actor (`MsBtlChrRegist`), the read queue's player, monster
//! and weapon steps (`MsBtlRead*Process`, `MsBtlWeaponRegist`). The
//! characters are `ffx_chara`'s; script motions, actions, status effects
//! and the vfx calls go through a [`Call`].

/// The game's memory, through [`ffx_bridge::Mem`].
#[allow(clippy::many_single_char_names)] // indices, actors, records
pub mod bridge {
    use ffx_bridge::x87::{Reg, round_trip};
    use ffx_bridge::{Global, IMAGE_BASE, Mem};
    use ffx_chara::chr::bridge as ch;
    use ffx_chara::layout as l;

    use crate::actor::bridge::{ms_get_chr, ms_get_weapon, ms_get_weapon_special};
    use crate::exec::bridge::Call;
    use crate::pos::bridge::Trig;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn put(mem: &mut impl Mem, p: u32, o: u32, v: u8) {
        mem.write(p.wrapping_add(o), v);
    }

    /// An `f32` field copied through the x87 (`fld`/`fstp`).
    fn float(mem: &impl Mem, p: u32, o: u32) -> f32 {
        round_trip(f32::from_bits(mem.read(p.wrapping_add(o))))
    }

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, f: u32, args: &[u32]) -> u32 {
        let f = f.wrapping_sub(IMAGE_BASE).wrapping_add(mem.image_base());
        call(mem, f, args)
    }

    /// `opus_change_chrhd_ffx_to_battle()` (vfx, `0x007fc290`).
    pub const OPUS_CHANGE_CHRHD: u32 = 0x007f_c290;
    /// `op_et_bindeff(chr)` (vfx).
    pub const OP_ET_BINDEFF: u32 = 0x007f_b290;
    /// `Ch_SetCalcNorm2(actor)` (`0x007969c0`).
    pub const SET_CALC_NORM2: u32 = 0x0079_69c0;
    /// `op_et_bindeff2(chr)` (vfx).
    pub const OP_ET_BINDEFF2: u32 = 0x007f_b360;
    /// `MsActionRequest(i, who, n, special, mode, out)`.
    pub const ACTION_REQUEST: u32 = 0x007a_ceb0;
    /// `MsSetMotion(i, motion, wait, x, flags, actor, out)`.
    pub const SET_MOTION: u32 = 0x007a_b360;
    /// `MsStatusEffectCheck(i)`.
    pub const STATUS_EFFECT_CHECK: u32 = 0x0079_eff0;
    /// `MsGetChrEscapePos(i, vec)`.
    pub const GET_CHR_ESCAPE_POS: u32 = 0x0079_4ec0;
    /// `MsAttachWeapon(i, k)`.
    pub const ATTACH_WEAPON: u32 = 0x0079_3330;
    /// -1.0 (`f32`): the weight of an actor with `+0xdce` set.
    pub const MINUS_ONE: Global<u32> = Global::new(0x00b9_2340);
    /// 13.0 and 14.0 (`f64`): an actor's thickness height is 13/14 of its
    /// height.
    pub const THIRTEEN: Global<f64> = Global::new(0x00b4_b470);
    pub const FOURTEEN: Global<f64> = Global::new(0x00b4_b388);
    /// `g_BtlMonsterSlots`: the counters monsters of kinds 0x14-0x16
    /// (`+0x5c2`) take their `+0x5c3` from.
    pub const MONSTER_SLOTS: Global<[u32; 3]> = Global::new(0x0112_ca84);
    /// The weapon actor a kind 0x1e special weapon becomes.
    const WEAPON_ACTOR: u32 = 0x1e;

    /// `MsSetMotion(i, motion, 0, 0, 1, 0, 0)` through the game.
    fn set_motion<M: Mem>(mem: &mut M, call: Call<'_, M>, i: u32, motion: u32) {
        game(mem, call, SET_MOTION, &[i, motion, 0, 0, 1, 0, 0]);
    }

    /// `MsBtlChrRegist(i, key)` (`0x00793670`): actor `i` (its low byte)
    /// gets its draw packets (`MsChrCreatePacket`) and, without one, a new
    /// character for model `key` (`Ch_Allocate`). The character takes the
    /// actor's number and height; a new one, any while the map read has
    /// started ([`crate::read::bridge::MAP_STARTED`]) or with `+0x506` set
    /// is placed at the battle position (`MsGetChrBtlPos` into `scratch`,
    /// 16 bytes) with its cloth reset, others only read theirs back
    /// (`Ch_GetPos`, unused). Unless set, `+0x4e4` takes the character's
    /// battle thickness (`+0x44c` clear) and `+0x4e8` 13/14 of the actor's
    /// height (`+0x44d` clear). The character then gets the motion speed
    /// (`+0x400`), thickness, weight (`+0x4dc`, -1 with `+0xdce`),
    /// direction (`+0x514[+0x511]`), scale (`+0x3a0`..), bind scale
    /// (`+0x3ac`), speed 0, full opacity, effect level (`+0xe2c`), the
    /// actor's normals and motion file, the engine set-up
    /// (`MsSetChrEngine_00796f70`) and, when `+0x4c` is unset, the
    /// resident effects pointer there. After `opus_change_chrhd_ffx_to_battle`
    /// the actor appears (`MsSetChrVisibleAppear`) unless `+0x43d` bit 0
    /// or 2 is set (bit 0, or bit 3 for a new character, also set `+0x423`
    /// bit 0); `+0x1e` binds its effects (`op_et_bindeff`, `+0x1f` set).
    /// Returns the character.
    #[allow(clippy::too_many_lines)]
    pub fn ms_btl_chr_regist<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        call: Call<'_, M>,
        i: u32,
        key: u32,
        scratch: u32,
    ) -> u32 {
        let i = i & 0xff;
        let actor = ms_get_chr(mem, i);
        crate::free::bridge::ms_chr_create_packet(mem, call, actor);
        let created = mem.read::<u32>(actor) == 0;
        if created {
            let chr =
                ffx_chara::handle::bridge::ch_allocate(mem, key, &mut |m, f, a| call(m, f, a));
            mem.write(actor, chr);
        }
        let chr: u32 = mem.read(actor);
        ch::ch_set_battle_chr_num(mem, chr, i);
        crate::flow::bridge::ms_set_chr_height(mem, i);
        let started = crate::read::bridge::MAP_STARTED.get(mem) != 0;
        if created || started || byte(mem, actor, 0x506) != 0 {
            crate::pos::bridge::ms_get_chr_btl_pos(mem, trig, i, scratch);
            let [x, y, z] = [0, 4, 8].map(|o| float(mem, scratch, o));
            ch::ch_set_pos3(mem, chr, x, y, z);
            ffx_chara::cloth::bridge::ch_cloth_reset(mem, chr);
        } else {
            ch::ch_get_pos(mem, chr, scratch, scratch + 4, scratch + 8);
        }
        if byte(mem, actor, 0x44c) == 0 {
            let t = ch::ch_get_thickness_btl(mem, chr);
            mem.write(actor.wrapping_add(0x4e4), t.to_bits());
        }
        if byte(mem, actor, 0x44d) == 0 {
            let h = crate::body::bridge::ms_get_chr_height(mem, i);
            let thirteen = Reg::constant(THIRTEEN.get(mem));
            let fourteen = Reg::constant(FOURTEEN.get(mem));
            let v = (Reg::load(h) * thirteen / fourteen).store();
            mem.write(actor.wrapping_add(0x4e8), v.to_bits());
        }
        let weight = if byte(mem, actor, 0xdce) == 0 {
            float(mem, actor, 0x4dc)
        } else {
            round_trip(f32::from_bits(MINUS_ONE.get(mem)))
        };
        let speed: u16 = mem.read(actor.wrapping_add(0x400));
        ch::ch_set_motion_speed(mem, chr, u32::from(speed));
        let t = float(mem, actor, 0x4e4);
        ch::ch_set_thickness(mem, chr, t);
        ch::ch_set_weight(mem, chr, weight);
        let side = i32::from(mem.read::<i8>(actor.wrapping_add(0x511))).cast_unsigned();
        let dir = float(mem, actor, 0x514u32.wrapping_add(side.wrapping_mul(4)));
        ch::ch_set_dir_f(mem, chr, dir);
        let [x, y, z] = [0x3a0, 0x3a4, 0x3a8].map(|o| float(mem, actor, o));
        ch::ch_set_scale(mem, chr, 7, x, y, z);
        let s = float(mem, actor, 0x3ac);
        ffx_chara::parts::bridge::ch_bind_set_scale(mem, chr, s);
        ch::ch_set_f32(mem, chr, l::SP, 0.0);
        ch::ch_set_fade(mem, chr, l::FADE_ALPHA, 1.0, 0);
        let level: u32 = mem.read(actor.wrapping_add(0xe2c));
        ch::ch_effect_set_effect_level(mem, chr, level);
        // through the game: the original also logs the change (dbgPrintf)
        game(mem, call, SET_CALC_NORM2, &[actor]);
        let file = i32::from(byte(mem, actor, 0x4bb));
        crate::minor::bridge::ms_set_mot_bin_address(mem, actor, file);
        crate::visible::bridge::ms_set_chr_engine(mem, call, chr, actor);
        if mem.read::<u32>(actor.wrapping_add(0x4c)) == 0 {
            let p = ch::ch_effect_get_resident_ptr(mem, chr);
            mem.write(actor.wrapping_add(0x4c), p);
        }
        game(mem, call, OPUS_CHANGE_CHRHD, &[]);
        let flags = byte(mem, actor, 0x43d);
        let mut appear = flags & 1 == 0;
        if !appear {
            let v = byte(mem, actor, 0x423);
            put(mem, actor, 0x423, v | 1);
        }
        appear &= flags & 4 == 0;
        if created && flags & 8 != 0 {
            appear = false;
            let v = byte(mem, actor, 0x423);
            put(mem, actor, 0x423, v | 1);
        }
        if byte(mem, actor, 0x1e) != 0 {
            put(mem, actor, 0x1f, 1);
            game(mem, call, OP_ET_BINDEFF, &[chr]);
        }
        crate::visible::bridge::ms_set_chr_visible_appear(mem, call, i, u32::from(appear));
        chr
    }

    /// `MsBtlReadPlayerModelProcess(i)` (`0x00783b90`): the read queue's
    /// model step: [`ms_btl_chr_regist`] with the actor's model (`+4`),
    /// then `MsStatusEffectCheck(i)`.
    pub fn ms_btl_read_player_model_process<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        call: Call<'_, M>,
        i: u32,
        scratch: u32,
    ) {
        let key: u32 = mem.read(ms_get_chr(mem, i).wrapping_add(4));
        ms_btl_chr_regist(mem, trig, call, i, key, scratch);
        game(mem, call, STATUS_EFFECT_CHECK, &[i]);
    }

    /// `MsBtlReadMonsterProcess(i)` (`0x007839b0`): a monster with a model
    /// (`+0xe` not 0xffff) and `+0x24` set (else 0xffff back) gets
    /// `+0x38` set and its motions enabled (`MsEffectResetMotionDisable`);
    /// with `+0xdd7` its slot `+0x5c3` comes from its kind `+0x5c2`
    /// (0x14-0x16: the next of [`MONSTER_SLOTS`], 0x17: 0, others 1;
    /// jump table `0x00783ae4`). Then [`ms_btl_chr_regist`] with its model
    /// (`+4`, read first), `MsActionRequest(i, 0xff, 5, 0, 2, 0)`, motion
    /// 0x27, and either (with `+0x43d` bit 1) `+0x3f0` 0xff, `+0x433` 1
    /// and motion 0x31, or a fade in (`MsSetChrTransparentVisible(i, 0, 1,
    /// 8, +0x423)`) and motion 0; last `MsStatusEffectCheck(i)`.
    pub fn ms_btl_read_monster_process<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        call: Call<'_, M>,
        i: u32,
        scratch: u32,
    ) -> u32 {
        let actor = ms_get_chr(mem, i);
        let key: u32 = mem.read(actor.wrapping_add(4));
        if mem.read::<u16>(actor.wrapping_add(0xe)) == 0xffff || byte(mem, actor, 0x24) == 0 {
            return 0xffff;
        }
        put(mem, actor, 0x38, 1);
        crate::minor::bridge::ms_effect_reset_motion_disable(mem, actor);
        if byte(mem, actor, 0xdd7) != 0 {
            let slot = match u32::from(byte(mem, actor, 0x5c2)).wrapping_sub(0x14) {
                k @ 0..=2 => {
                    let mut slots = MONSTER_SLOTS.get(mem);
                    let k = k as usize;
                    let v = slots[k].to_le_bytes()[0];
                    slots[k] = slots[k].wrapping_add(1);
                    MONSTER_SLOTS.set(mem, slots);
                    v
                }
                3 => 0,
                _ => 1,
            };
            put(mem, actor, 0x5c3, slot);
        }
        ms_btl_chr_regist(mem, trig, call, i, key, scratch);
        game(mem, call, ACTION_REQUEST, &[i, 0xff, 5, 0, 2, 0]);
        set_motion(mem, call, i, 0x27);
        let motion = if byte(mem, actor, 0x43d) & 2 != 0 {
            put(mem, actor, 0x3f0, 0xff);
            put(mem, actor, 0x433, 1);
            0x31
        } else {
            let flags = u32::from(byte(mem, actor, 0x423));
            let (from, to) = (0.0f32.to_bits(), 1.0f32.to_bits());
            crate::minor::bridge::ms_set_chr_transparent_visible(mem, call, i, from, to, 8, flags);
            0
        };
        set_motion(mem, call, i, motion);
        game(mem, call, STATUS_EFFECT_CHECK, &[i])
    }

    /// `MsBtlReadPlayerProcess(i, mode)` (`0x00783bc0`): a party member's
    /// step of the read queue: `+0x40` 1.0, `+0x38` 1, `+0xdfc` 0,
    /// `+0x6fc` -1, its motions enabled, its character registered
    /// ([`ms_btl_chr_regist`] with `+4`) unless `mode` 2, effects bound
    /// for the first eight (`op_et_bindeff2`), `MsActionRequest(i, 0xff,
    /// 5, 0, 2, 0)` and motion 0x27. Mode 6 (escape) sets `+0x400` 0xffff,
    /// `+0x3f0` 0xff, `+0x41c` 1 and moves it to its escape position
    /// (motion 0x32); mode 8 sets `+0x3f0` 0xff, turns off body hit 2 and
    /// moves it to its battle position (motion 0, no status check); mode
    /// 10 sets `+0x3f0` 0xff, `+0x400` 0xffff (motion 0x39); others motion
    /// 0x31. `MsStatusEffectCheck(i)` but for mode 8, `+0x44f` 0,
    /// `MsSetChrWeak(i, -1)`. With `+0xdce` `+0x508` is 0 (and mode 10
    /// hides it: `MsSetChrVisibleAppear(i, 0)`); else `+0x508` takes
    /// `MsCheckBtlPos(i, actor, +0x34)` and the motion runs.
    #[allow(clippy::too_many_lines)]
    pub fn ms_btl_read_player_process<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        call: Call<'_, M>,
        i: u32,
        mode: u32,
        scratch: u32,
    ) {
        let actor = ms_get_chr(mem, i);
        mem.write(actor.wrapping_add(0x40), 1.0f32.to_bits());
        put(mem, actor, 0x38, 1);
        put(mem, actor, 0xdfc, 0);
        mem.write(actor.wrapping_add(0x6fc), u32::MAX);
        crate::minor::bridge::ms_effect_reset_motion_disable(mem, actor);
        if mode != 2 {
            let key: u32 = mem.read(actor.wrapping_add(4));
            ms_btl_chr_regist(mem, trig, call, i, key, scratch);
        }
        if i.cast_signed() < 8 {
            let chr: u32 = mem.read(actor);
            game(mem, call, OP_ET_BINDEFF2, &[chr]);
        }
        game(mem, call, ACTION_REQUEST, &[i, 0xff, 5, 0, 2, 0]);
        set_motion(mem, call, i, 0x27);
        let mut motion = 0x31;
        let mut status = true;
        match mode {
            6 => {
                mem.write(actor.wrapping_add(0x400), 0xffffu16);
                put(mem, actor, 0x3f0, 0xff);
                put(mem, actor, 0x41c, 1);
                game(mem, call, GET_CHR_ESCAPE_POS, &[i, scratch]);
                crate::chr_damage::bridge::ms_set_chr_pos_reset(mem, i, scratch);
                motion = 0x32;
            }
            8 => {
                put(mem, actor, 0x3f0, 0xff);
                crate::body::bridge::ms_set_chr_body_hit(mem, actor, 2, 0);
                crate::pos::bridge::ms_get_chr_btl_pos(mem, trig, i, scratch);
                crate::chr_damage::bridge::ms_set_chr_pos_reset(mem, i, scratch);
                motion = 0;
                status = false;
            }
            10 => {
                put(mem, actor, 0x3f0, 0xff);
                mem.write(actor.wrapping_add(0x400), 0xffffu16);
                motion = 0x39;
            }
            _ => {}
        }
        if status {
            game(mem, call, STATUS_EFFECT_CHECK, &[i]);
        }
        put(mem, actor, 0x44f, 0);
        crate::input::bridge::ms_set_chr_weak(mem, i, u32::MAX);
        if byte(mem, actor, 0xdce) != 0 {
            put(mem, actor, 0x508, 0);
            if mode == 10 {
                crate::visible::bridge::ms_set_chr_visible_appear(mem, call, i, 0);
            }
            return;
        }
        let flat = i32::from(mem.read::<i8>(actor.wrapping_add(0x34))).cast_unsigned();
        let on = crate::pos::bridge::ms_check_btl_pos(mem, trig, i, actor, flat);
        put(mem, actor, 0x508, on.to_le_bytes()[0]);
        set_motion(mem, call, i, motion);
    }

    /// `MsBtlWeaponRegist(i, mode, k)` (`0x007938e0`): weapon `k` of actor
    /// `i` (`MsGetWeapon`) follows the save's model
    /// (`MsGetSaveWeaponModel(+0x592[k])`): a changed model replaces the
    /// record's (`+4`) and disposes its character (`Ch_Dispose`, then
    /// `ChN_DataDispose` of the new model, as the original passes it). It
    /// gets its draw packets and, with a model and no character, a new one
    /// (`Ch_Allocate`); without a character nothing more (0 back). A
    /// special weapon (`MsGetWeaponSpecial`) takes the character and its
    /// kind (`+0xc`) sets the flags: 0x1c clears `+0x34`, 0x1d sets
    /// `+0x2b`/`+0x2e`/`+0x32`, 0x1e those and becomes actor 0x1e standing
    /// at its owner (`+0x4fc` 0xe00, `+0x4fe` the owner, its battle
    /// position, the owner's directions, hidden: `+0x4b5` 0). The special
    /// record is set up (owner's `+0x13` the kind, `+0x14` the owner,
    /// `+0x12`/`+0x15` 1, the flags, motion file, the character's number
    /// and weight; with the weapon's `+8` motions 0x27 and 0). The weapon
    /// record takes the flags and the engine set-up; the owner's `+0x4b5`
    /// whether it shows: shown and `+0x1d` set, a new character fades in
    /// (`Ch_SetTransparent` 0 then 1 over 8), else it is hidden in battle
    /// and from effects. Then `MsAttachWeapon(i, k)` and, with `mode` 0,
    /// `MsSetWeaponEffectAttribute(i, actor)`. Returns the character.
    #[allow(clippy::too_many_lines)]
    pub fn ms_btl_weapon_regist<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        call: Call<'_, M>,
        [i, mode, k]: [u32; 3],
        scratch: u32,
    ) -> u32 {
        let owner = ms_get_chr(mem, i);
        let w = ms_get_weapon(mem, i, k.cast_signed());
        let save = u32::from(byte(mem, owner.wrapping_add(k), 0x592));
        let model = crate::exchange::bridge::ms_get_save_weapon_model(mem, save);
        if mem.read::<u32>(w.wrapping_add(4)) != model {
            mem.write(w.wrapping_add(4), model);
            let old: u32 = mem.read(w);
            if old != 0 {
                let f = &mut |m: &mut M, f: u32, a: &[u32]| call(m, f, a);
                ffx_chara::handle::bridge::ch_dispose(mem, old, f);
                let new: u32 = mem.read(w.wrapping_add(4));
                ffx_chara::handle::bridge::ch_n_data_dispose(mem, new, 0, f);
                mem.write(w, 0u32);
            }
        }
        let mut chr: u32 = mem.read(w);
        crate::free::bridge::ms_chr_create_packet(mem, call, w);
        let mut created = false;
        if model != 0 && chr == 0 {
            created = true;
            chr = ffx_chara::handle::bridge::ch_allocate(mem, model, &mut |m, f, a| call(m, f, a));
            mem.write(w, chr);
        }
        if chr == 0 {
            return 0;
        }
        let mut shown = true;
        // the flags +0x2b, +0x2e, +0x32 and +0x34
        let mut flags = [0u8, 0, 0, 1];
        let special = ms_get_weapon_special(mem, i, k);
        if special != 0 {
            let kind = byte(mem, special, 0xc);
            mem.write(special, chr);
            match kind {
                0x1c => flags[3] = 0,
                0x1d | 0x1e => {
                    if kind == 0x1e {
                        shown = false;
                        mem.write(special.wrapping_add(0x4fc), 0x0e00u16);
                        put(mem, special, 0x4fe, i.to_le_bytes()[0]);
                        crate::actor::bridge::ms_push_btl_pos(mem, special);
                        crate::pos::bridge::ms_get_chr_btl_pos(mem, trig, WEAPON_ACTOR, scratch);
                        crate::chr_damage::bridge::ms_set_chr_pos_reset(mem, WEAPON_ACTOR, scratch);
                        for o in [0x514, 0x518] {
                            let v = float(mem, owner, o);
                            mem.write(special.wrapping_add(o), v.to_bits());
                        }
                        let side = i32::from(mem.read::<i8>(special.wrapping_add(0x511)));
                        let o = 0x514u32.wrapping_add(side.cast_unsigned().wrapping_mul(4));
                        let dir = float(mem, special, o);
                        crate::chr_damage::bridge::ms_set_chr_dir_f(
                            mem,
                            WEAPON_ACTOR,
                            dir.to_bits(),
                        );
                    }
                    flags[..3].fill(1);
                }
                _ => {}
            }
            put(mem, owner, 0x13, kind);
            put(mem, special, 0x14, i.to_le_bytes()[0]);
            put(mem, special, 0x12, 1);
            put(mem, special, 0x15, 1);
            for (o, v) in [0x2b, 0x2e, 0x32, 0x34].into_iter().zip(flags) {
                put(mem, special, o, v);
            }
            let file = i32::from(byte(mem, special, 0x4bb));
            crate::minor::bridge::ms_set_mot_bin_address(mem, special, file);
            ch::ch_set_battle_chr_num(mem, chr, u32::from(kind));
            let weight = float(mem, special, 0x4dc);
            ch::ch_set_weight(mem, chr, weight);
            if mem.read::<u32>(w.wrapping_add(8)) != 0 {
                put(mem, special, 0x38, 1);
                set_motion(mem, call, u32::from(kind), 0x27);
                set_motion(mem, call, u32::from(kind), 0);
            }
        }
        for (o, v) in [0x2b, 0x2e, 0x32, 0x34].into_iter().zip(flags) {
            put(mem, w, o, v);
        }
        crate::visible::bridge::ms_set_chr_engine(mem, call, chr, w);
        put(mem, owner, 0x4b5, u8::from(shown));
        if shown && byte(mem, owner, 0x1d) != 0 {
            if created {
                ch::ch_set_fade(mem, chr, l::FADE_ALPHA, 0.0, 0);
                ch::ch_set_fade(mem, chr, l::FADE_ALPHA, 1.0, 8);
            }
        } else {
            ch::ch_set_hide_battle(mem, chr, 1);
            ch::ch_set_hide_effect(mem, chr, 1);
        }
        game(mem, call, ATTACH_WEAPON, &[i, k]);
        if mode == 0 {
            crate::et_effect::bridge::ms_set_weapon_effect_attribute(mem, call, i, owner);
        }
        chr
    }
}
