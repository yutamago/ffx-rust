//! More of the battle's small routines (`docs/rewrite-plan.md`, M2): the
//! subsystems' resets, the battle field list's lookups, the motion
//! binary's frame flags, the script managers' queues, the stone and
//! damage-number lists, and a few actor setters.

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::state::bridge::FIELD_RECORDS;

    fn byte(mem: &impl Mem, p: u32, o: u32) -> u8 {
        mem.read(p.wrapping_add(o))
    }

    fn half(mem: &impl Mem, p: u32, o: u32) -> u16 {
        mem.read(p.wrapping_add(o))
    }

    /// Set while the party moves freely (`MsSetDirThetaMode`: direction
    /// mode 1).
    pub const DIR_FREE: Global<u8> = Global::new(0x0112_c9e7);
    /// The battle fields' record count (`i16` here).
    pub const FIELD_COUNT: Global<i16> = Global::new(0x0112_a9d0);
    /// The encounter counters `ResetEncountExe` resets: the encounter type
    /// (a `u16` write here), two counters (`f32` 0) and two rates (1.0).
    pub const ENCOUNT_TYPE: Global<u16> = Global::new(0x0112_a9d4);
    pub const ENCOUNT_STEPS: Global<[u32; 2]> = Global::new(0x0112_a9d8);
    pub const ENCOUNT_RATES: Global<[u32; 2]> = Global::new(0x0112_a9e0);
    /// The effect camera `MsEffectCameraCall` asks for, and its mode.
    pub const EFFECT_CAMERA_CALL: Global<i32> = Global::new(0x0112_a8f0);
    pub const EFFECT_CAMERA_MODE: Global<u8> = Global::new(0x0112_c9d7);
    /// Zanmato's separate list (`resetZanmatoSepList`), ten words.
    pub const ZANMATO_SEP: Global<[u32; 10]> = Global::new(0x0113_63c0);
    /// The motion binary (`MsSetMotBinAddress`): a `u16` count, then `u16`
    /// offsets from its start.
    pub const MOT_BIN: Global<u32> = Global::new(0x0112_a954);
    /// The script managers (`MsAtelRegist`): 0x54 bytes each from
    /// `0x01134470`, a count, then eight scripts from `+0x14` and eight
    /// words from `+0x34`.
    pub const ATEL_MANAGERS: Global<u8> = Global::new(0x0113_4470);
    /// `MsSEPtransInit`: 0x80 words, three more, and a `u16`.
    pub const SEP_TRANS_TABLE: Global<[u32; 0x80]> = Global::new(0x0112_c02c);
    pub const SEP_TRANS_STATE: Global<[u32; 3]> = Global::new(0x0113_6188);
    pub const SEP_TRANS_FLAG: Global<u16> = Global::new(0x0112_c029);
    /// `MsAtelCamInit`: a byte, two words, and five words at `0x01134518`.
    pub const ATEL_CAM_ON: Global<u8> = Global::new(0x0113_5df0);
    pub const ATEL_CAM: Global<[u32; 2]> = Global::new(0x0113_5df4);
    pub const ATEL_CAM2: Global<[u32; 5]> = Global::new(0x0113_4518);
    /// The 3D sound effects (`MsSEP3DfadeOutAll`): a list of nodes, the
    /// fade at `+4` and `+6`, the next at `+0xc`.
    pub const SEP_3D: Global<u32> = Global::new(0x0113_61a0);
    /// `MsEffectCameraBufInit`: eight words.
    pub const EFFECT_CAMERA_BUF: Global<[u32; 8]> = Global::new(0x0112_c22c);
    /// The equipment's wearers (`MsInitSaveWeaponEquipFlag`): byte `+0` of
    /// 200 records of 0x16 bytes, the character who wears it.
    pub const WEAPON_EQUIP: Global<u8> = Global::new(0x0113_0f32);
    /// `g_PlySave`'s `+0x2d` (weapon slot) of the first character; the
    /// armor slot follows, 0x94 bytes per character.
    pub const EQUIPPED: Global<u8> = Global::new(0x0113_2089);
    /// The petrified actors `MsStoneRegist` keeps (up to 31), their count
    /// [`crate::state::bridge::STONE_EFFECT`], and the actors that do not
    /// count (a bit mask).
    pub const STONES: Global<[u32; 31]> = Global::new(0x0112_c950);
    pub const STONE_EXCLUDED: Global<u32> = Global::new(0x0112_c90c);
    /// The commands each character has used (`MsSetSaveCommandUse`): 18 of
    /// 0x2c bytes, a bit per command.
    pub const SAVE_COMMAND_USE: Global<u8> = Global::new(0x0113_2ac4);
    /// `MsAtelCameraInit`: four entries of three words.
    pub const ATEL_CAMERA: Global<[u32; 12]> = Global::new(0x0113_6f88);
    /// The damage numbers on screen (`MsNumberCalcProcess`): a list, its
    /// last link, and the counts by kind.
    pub const NUMBER_LIST: Global<u32> = Global::new(0x0113_60bc);
    pub const NUMBER_TAIL: Global<u32> = Global::new(0x0113_60c0);
    pub const NUMBERS_KIND1: Global<u8> = Global::new(0x0112_c9d3);
    pub const NUMBERS_KIND12: Global<u8> = Global::new(0x0112_c9d4);
    pub const NUMBERS_SHOWN: Global<u8> = Global::new(0x0112_c9d5);

    /// `MsSetDirThetaMode(actor)` (`0x00796940`): the actor's direction
    /// mode (`+0x511`): 2 with status bit 0x800, 3 with `+0x6d4` set, 1
    /// while [`DIR_FREE`], else 0; returned.
    pub fn ms_set_dir_theta_mode(mem: &mut impl Mem, actor: u32) -> u32 {
        let m = if half(mem, actor, 0x606) & 0x800 != 0 {
            2
        } else if byte(mem, actor, 0x6d4) != 0xff {
            3
        } else {
            u8::from(DIR_FREE.get(mem) != 0)
        };
        mem.write(actor.wrapping_add(0x511), m);
        u32::from(m)
    }

    /// `MsBtlListField(i)` (`0x0079d190`): battle field record `i` (14
    /// bytes; the first for `i` out of range).
    pub fn ms_btl_list_field(mem: &impl Mem, i: i32) -> u32 {
        let i = if i < 0 || i >= i32::from(FIELD_COUNT.get(mem)) {
            0
        } else {
            i
        };
        FIELD_RECORDS
            .get(mem)
            .wrapping_add(i.cast_unsigned().wrapping_mul(14))
    }

    /// `MsBtlListFieldNum(map)` (`0x0079d1c0`): the first field whose map
    /// (`i16` at `+0`) is `map` (above 0), else -1.
    pub fn ms_btl_list_field_num(mem: &impl Mem, map: i32) -> i32 {
        let n = i32::from(FIELD_COUNT.get(mem));
        let t = FIELD_RECORDS.get(mem);
        for i in 0..n {
            let m: i16 = mem.read(t.wrapping_add(i.cast_unsigned() * 14));
            if i32::from(m) == map && map > 0 {
                return i;
            }
        }
        -1
    }

    /// `ResetEncountExe(all)` (`0x00780ff0`): the counters to 0 and the
    /// rates to 1, the encounter type too when `all`.
    pub fn reset_encount_exe(mem: &mut impl Mem, all: u32) {
        if all != 0 {
            ENCOUNT_TYPE.set(mem, 0);
        }
        ENCOUNT_STEPS.set(mem, [0, 0]);
        ENCOUNT_RATES.set(mem, [1.0f32.to_bits(); 2]);
    }

    /// `MsEffectCameraCall(camera, mode)` (`0x00787740`): the camera when
    /// not negative, and the mode; EAX `camera` with `mode` in its low byte.
    pub fn ms_effect_camera_call(mem: &mut impl Mem, camera: i32, mode: u32) -> u32 {
        if camera >= 0 {
            EFFECT_CAMERA_CALL.set(mem, camera);
        }
        let m = mode.to_le_bytes()[0];
        EFFECT_CAMERA_MODE.set(mem, m);
        (camera.cast_unsigned() & !0xff) | u32::from(m)
    }

    /// `resetZanmatoSepList` (`0x007a8f20`); EAX 0.
    pub fn reset_zanmato_sep_list(mem: &mut impl Mem) -> u32 {
        ZANMATO_SEP.set(mem, [0; 10]);
        0
    }

    /// `MsSetMotBinAddress(actor, i)` (`0x0079cd80`): the actor's motion
    /// table (`+0x4f8`), entry `i` of the motion binary, null when out of
    /// range or without one; returns the binary.
    pub fn ms_set_mot_bin_address(mem: &mut impl Mem, actor: u32, i: i32) -> u32 {
        let bin = MOT_BIN.get(mem);
        let mut v = 0;
        if bin != 0 && i >= 0 && i < i32::from(half(mem, bin, 0)) {
            let o = half(mem, bin, 2 + 2 * i.cast_unsigned());
            v = u32::from(o).wrapping_add(bin);
        }
        mem.write(actor.wrapping_add(0x4f8), v);
        bin
    }

    /// `MsAtelRegist(manager, name, script)` (`0x00797a40`): queues
    /// `script` in a manager with room (0, else -1). `name` is unused.
    pub fn ms_atel_regist(mem: &mut impl Mem, manager: i32, script: u32) -> i32 {
        let base = ATEL_MANAGERS.ptr(mem).addr;
        let at = base.wrapping_add(manager.cast_unsigned().wrapping_mul(0x54));
        let n: i32 = mem.read(at);
        if n >= 8 {
            return -1;
        }
        let slot = manager.wrapping_mul(0x15).wrapping_add(n).cast_unsigned();
        mem.write(
            base.wrapping_add(0x14).wrapping_add(slot.wrapping_mul(4)),
            script,
        );
        mem.write(
            base.wrapping_add(0x34).wrapping_add(slot.wrapping_mul(4)),
            0u32,
        );
        mem.write(at, n.wrapping_add(1));
        0
    }

    /// `MsSEPtransInit` (`0x007a0500`); EAX 0.
    pub fn ms_sep_trans_init(mem: &mut impl Mem) -> u32 {
        SEP_TRANS_TABLE.set(mem, [0; 0x80]);
        SEP_TRANS_STATE.set(mem, [0; 3]);
        SEP_TRANS_FLAG.set(mem, 0);
        0
    }

    /// `MsAtelCamInit` (`0x00797500`).
    pub fn ms_atel_cam_init(mem: &mut impl Mem) {
        ATEL_CAM_ON.set(mem, 0);
        ATEL_CAM.set(mem, [0; 2]);
        ATEL_CAM2.set(mem, [0; 5]);
    }

    /// `Mon_FindHandler(file, i, &k)` (`0x00797410`): in a monster file's
    /// handler section (`+1` a count, `+2` a byte per handler, then `i16`
    /// offsets), handler `i`'s table; `k` becomes its `k`-th entry, or -1
    /// past its end or for 0xffff. Null for none.
    #[allow(clippy::many_single_char_names)]
    pub fn mon_find_handler(mem: &mut impl Mem, file: u32, i: i32, k: u32) -> u32 {
        if file == 0 {
            return 0;
        }
        let h = byte(mem, file, 2u32.wrapping_add(i.cast_unsigned()));
        let want: i32 = mem.read(k);
        let n = i32::from(byte(mem, file, 1));
        if h == 0xff || i >= n {
            return 0;
        }
        let e = ((n + 1) >> 1) + 2 * i32::from(h);
        let e = e.cast_unsigned();
        let o: i16 = mem.read(file.wrapping_add(e * 2 + 4));
        let at = file.wrapping_add(e * 2);
        let table = file.wrapping_add(i32::from(o).cast_unsigned());
        let mut out = u32::MAX;
        if want < i32::from(half(mem, table, 0)) {
            let v = half(
                mem,
                table,
                want.cast_unsigned().wrapping_mul(2).wrapping_add(2),
            );
            if v != 0xffff {
                out = u32::from(v);
            }
        }
        mem.write(k, out);
        at.wrapping_add(2)
    }

    /// `MsMotBinFrame(_, actor, motion, frame)` (`0x0079cb40`): in the
    /// actor's motion table (`+0x4f8`: a `u16` count, then sorted `u16`
    /// motion and `u16` offset pairs), the first record of `motion` that
    /// lists `frame` (or 0xffff) among its "on" frames and not among its
    /// "off" frames: its byte 0; 0 for none.
    pub fn ms_mot_bin_frame(mem: &impl Mem, actor: u32, motion: u32, frame: u32) -> u32 {
        let table: u32 = mem.read(actor.wrapping_add(0x4f8));
        if table == 0 {
            return 0;
        }
        let entries = table.wrapping_add(2);
        let (mut lo, mut hi) = (0i32, i32::from(half(mem, table, 0)) - 1);
        let found = loop {
            if hi < lo {
                return 0;
            }
            let mid = i32::midpoint(lo, hi);
            let key = i32::from(half(mem, entries, mid.cast_unsigned() * 4));
            let m = motion.cast_signed();
            if key == m {
                break entries.wrapping_add(mid.cast_unsigned() * 4);
            }
            if key > m {
                hi = mid - 1;
            } else {
                lo = mid + 1;
            }
        };
        if found == 0 {
            return 0;
        }
        let list = u32::from(half(mem, found, 2)).wrapping_add(table);
        let n = half(mem, list, 0);
        let mut result = 0;
        for j in 0..u32::from(n) {
            let rec = u32::from(half(mem, list, 2 + 2 * j)).wrapping_add(list);
            let mut p = rec.wrapping_add(4);
            let hit = |v: u16| v == 0xffff || u32::from(v) == frame;
            for _ in 0..byte(mem, rec, 1) {
                let v = half(mem, p, 0);
                p = p.wrapping_add(2);
                if hit(v) {
                    result = u32::from(byte(mem, rec, 0));
                    break;
                }
            }
            let mut off = false;
            for _ in 0..byte(mem, rec, 2) {
                let v = half(mem, p, 0);
                p = p.wrapping_add(2);
                if hit(v) {
                    off = true;
                    break;
                }
            }
            if off {
                result = 0;
            } else if result != 0 {
                return result;
            }
        }
        result
    }

    /// `MsSEP3DfadeOutAll(time)` (`0x007a08e0`): every 3D sound effect not
    /// fading yet starts fading over `time`; how many.
    pub fn ms_sep3d_fade_out_all(mem: &mut impl Mem, time: u16) -> u32 {
        let mut n = 0;
        let mut p = SEP_3D.get(mem);
        while p != 0 {
            if half(mem, p, 4) == 0 {
                mem.write(p.wrapping_add(4), time);
                mem.write(p.wrapping_add(6), time);
                n += 1;
            }
            p = mem.read(p.wrapping_add(0xc));
        }
        n
    }

    /// `MsWeaponReadSet(actor, slot, weapon)` (`0x007ad940`): when the
    /// actor's motion phase (`+0xdfb`) is free, asks for a weapon to be
    /// read: phase 8, slot `+0xf64`, weapon `+0xf66` (12 bits).
    pub fn ms_weapon_read_set(mem: &mut impl Mem, actor: u32, slot: u8, weapon: u32) {
        if byte(mem, actor, 0xdfb) == 0 {
            mem.write(actor.wrapping_add(0xf64), slot);
            mem.write(actor.wrapping_add(0xdfb), 8u8);
            #[allow(clippy::cast_possible_truncation)] // 12 bits
            mem.write(actor.wrapping_add(0xf66), (weapon & 0xfff) as u16);
        }
    }

    /// `MsEffectCameraBufInit` (`0x007876e0`).
    pub fn ms_effect_camera_buf_init(mem: &mut impl Mem) {
        EFFECT_CAMERA_BUF.set(mem, [0; 8]);
    }

    /// `MsInitSaveWeaponEquipFlag` (`0x007abe30`): every equipment's wearer
    /// to 0xff, then each of the 18 characters' weapon and armor to that
    /// character; EAX the end of the walk.
    pub fn ms_init_save_weapon_equip_flag(mem: &mut impl Mem) -> u32 {
        let w = WEAPON_EQUIP.ptr(mem).addr;
        for i in 0..200 {
            mem.write(w + 0x16 * i, 0xffu8);
        }
        let e = EQUIPPED.ptr(mem).addr;
        for c in 0u8..18 {
            for k in 0..2 {
                let slot = byte(mem, e, 0x94 * u32::from(c) + k);
                if slot != 0xff {
                    mem.write(w + 0x16 * u32::from(slot), c);
                }
            }
        }
        e + 1 + 0x94 * 18
    }

    /// `MsStoneRegist(chr)` (`0x007a2780`): adds a petrified actor to the
    /// list (once, up to 31); 1 when it was new, 0 when already there or
    /// excluded (bit `chr & 31` of [`STONE_EXCLUDED`]).
    pub fn ms_stone_regist(mem: &mut impl Mem, chr: u32) -> u32 {
        let count = crate::state::bridge::STONE_EFFECT.ptr(mem).addr;
        let n: i8 = mem.read(count);
        let list = STONES.ptr(mem).addr;
        let mut new = 1;
        for i in 0..i32::from(n).max(0) {
            if mem.read::<u32>(list + 4 * i.cast_unsigned()) == chr {
                new = 0;
                break;
            }
        }
        if (STONE_EXCLUDED.get(mem).cast_signed() >> (chr & 31)) & 1 != 0 {
            return 0;
        }
        if new != 0 && n < 0x1f {
            let at = list.wrapping_add(i32::from(n).cast_unsigned().wrapping_mul(4));
            mem.write(at, chr);
            mem.write(count, n.wrapping_add(1));
        }
        new
    }

    /// `MsSetSaveCommandUse(chr, command, on)` (`0x00785e00`): sets or
    /// clears character `chr`'s (signed, below 18) bit for a `0x3xxx`
    /// command; 0.
    pub fn ms_set_save_command_use(mem: &mut impl Mem, chr: i32, command: u32, on: u32) -> u32 {
        if command & 0xffff_f000 != 0x3000 || chr >= 0x12 {
            return 0;
        }
        let i = (command & 0xfff) >> 4;
        let bit = 1u16 << (command & 0xf);
        let at = SAVE_COMMAND_USE
            .ptr(mem)
            .addr
            .wrapping_add(chr.cast_unsigned().wrapping_mul(0x2c))
            .wrapping_add(2 * i);
        let w: u16 = mem.read(at);
        let w = (w & !bit) | if on != 0 { bit } else { 0 };
        mem.write(at, w);
        0
    }

    /// `MsAtelCameraInit` (`0x007ba650`).
    pub fn ms_atel_camera_init(mem: &mut impl Mem) {
        ATEL_CAMERA.set(mem, [0; 12]);
    }

    /// `MsNumberCalcProcess` (`0x0079f5e0`): ages the damage numbers (`+5`
    /// a delay counted down, `+6` an age, `+7` the life; `+0x10` the next)
    /// and unlinks the expired (kind `+0` cleared); counts the rest by kind
    /// (`+0` & 0x7f: 1, 1 or 2, any). EAX 0.
    pub fn ms_number_calc_process(mem: &mut impl Mem) -> u32 {
        let (mut k1, mut k12, mut shown) = (0u8, 0u8, 0u8);
        let mut link = NUMBER_LIST.ptr(mem).addr;
        let mut p = NUMBER_LIST.get(mem);
        while p != 0 {
            let delay = byte(mem, p, 5).cast_signed();
            if delay > 0 {
                mem.write(p + 5, delay - 1);
            }
            let age = byte(mem, p, 6).wrapping_add(1);
            mem.write(p + 6, age);
            let next_at = p.wrapping_add(0x10);
            if age.cast_signed() > byte(mem, p, 7).cast_signed() {
                mem.write(p, 0u8);
                let next: u32 = mem.read(next_at);
                mem.write(link, next);
            } else {
                match byte(mem, p, 0) & 0x7f {
                    1 => {
                        k1 = k1.wrapping_add(1);
                        k12 = k12.wrapping_add(1);
                    }
                    2 => k12 = k12.wrapping_add(1),
                    _ => {}
                }
                shown = shown.wrapping_add(1);
                link = next_at;
            }
            p = mem.read(next_at);
        }
        NUMBER_TAIL.set(mem, link);
        NUMBERS_KIND12.set(mem, k12);
        NUMBERS_KIND1.set(mem, k1);
        NUMBERS_SHOWN.set(mem, shown);
        0
    }
    /// `MsEffectResetMotionDisable(actor)` (`0x00788270`): with an effect
    /// motion (`+0x3f3`), drops it (`+0x3f2` to 0xff, `+0xdf2` and bit 1
    /// of `+0xf5e` cleared) and ends the motion (`MsTerminateMotion`); 1,
    /// else 0.
    pub fn ms_effect_reset_motion_disable(mem: &mut impl Mem, actor: u32) -> u32 {
        if byte(mem, actor, 0x3f3) == 0 {
            return 0;
        }
        let f = byte(mem, actor, 0xf5e) & 0xfd;
        mem.write(actor.wrapping_add(0xf5e), f);
        mem.write(actor.wrapping_add(0x3f2), 0xffu16);
        mem.write(actor.wrapping_add(0xdf2), 0u8);
        crate::actor::bridge::ms_terminate_motion(mem, actor);
        1
    }
    fn actor_of(mem: &impl Mem, i: u32) -> u32 {
        crate::actor::bridge::ms_get_chr(mem, i)
    }

    /// `MsMenuCountStatus(i)` (`0x0078fa80`): the actor's status counters
    /// (`+0x608`).
    pub fn ms_menu_count_status(mem: &impl Mem, i: u32) -> u32 {
        actor_of(mem, i & 0xff).wrapping_add(0x608)
    }

    /// `MsMenuDefCountStatus(i)` (`0x0078faa0`): `+0x641`.
    pub fn ms_menu_def_count_status(mem: &impl Mem, i: u32) -> u32 {
        actor_of(mem, i & 0xff).wrapping_add(0x641)
    }

    /// `MsMenuHalfElement(i)` (`0x0078fe40`): byte `+0x5dc`, in AL over the
    /// actor's address (EAX as the original leaves it).
    pub fn ms_menu_half_element(mem: &impl Mem, i: u32) -> u32 {
        let a = actor_of(mem, i & 0xff);
        (a & !0xff) | u32::from(byte(mem, a, 0x5dc))
    }

    /// `MsMenuBitStatus(i)` (`0x0078f950`): `u16` `+0x616` in AX over the
    /// actor's address.
    pub fn ms_menu_bit_status(mem: &impl Mem, i: u32) -> u32 {
        let a = actor_of(mem, i & 0xff);
        (a & !0xffff) | u32::from(half(mem, a, 0x616))
    }

    /// `MsGetSummonerNum(i)` (`0x00788e10`): `+0x6d1`.
    pub fn ms_get_summoner_num(mem: &impl Mem, i: u32) -> u32 {
        u32::from(byte(mem, actor_of(mem, i & 0xff), 0x6d1))
    }

    /// `MsMenuWeakElement(i)` (`0x007a9a20`): `+0x3f0`.
    pub fn ms_menu_weak_element(mem: &impl Mem, i: u32) -> u32 {
        u32::from(byte(mem, actor_of(mem, i & 0xff), 0x3f0))
    }

    /// `MsMenuCheckChrLive(i)` (`0x0078f970`): bit 0 of `+0x616`.
    pub fn ms_menu_check_chr_live(mem: &impl Mem, i: u32) -> u32 {
        u32::from(half(mem, actor_of(mem, i & 0xff), 0x616) & 1)
    }

    /// `MsGetSummonEnergy(i)` (`0x007b2b00`): `+0x6e1`.
    pub fn ms_get_summon_energy(mem: &impl Mem, i: u32) -> u32 {
        u32::from(byte(mem, actor_of(mem, i), 0x6e1))
    }

    /// `MsEffectGetTargetFlag(i)` (`0x00787c10`): `+0xdda`, signed.
    pub fn ms_effect_get_target_flag(mem: &impl Mem, i: u32) -> i32 {
        i32::from(byte(mem, actor_of(mem, i & 0xff), 0xdda).cast_signed())
    }

    /// The motion phase (`+0xdfb`) of actor `i`; EAX the actor
    /// (`MsSetMotionAttackWait`: 0xf, `MsSetMotionReturn`: 7).
    fn set_phase(mem: &mut impl Mem, i: u32, phase: u8) -> u32 {
        let a = actor_of(mem, i);
        mem.write(a.wrapping_add(0xdfb), phase);
        a
    }

    /// `MsSetMotionAttackWait(i)` (`0x007ab500`).
    pub fn ms_set_motion_attack_wait(mem: &mut impl Mem, i: u32) -> u32 {
        set_phase(mem, i, 0xf)
    }

    /// `MsSetMotionReturn(i)` (`0x007ab520`).
    pub fn ms_set_motion_return(mem: &mut impl Mem, i: u32) -> u32 {
        set_phase(mem, i, 7)
    }

    /// `MsDefensePosOff(actor)` (`0x007af880`): no defense position
    /// (`+0x6d3` 0xffff), then [`ms_set_dir_theta_mode`].
    pub fn ms_defense_pos_off(mem: &mut impl Mem, actor: u32) -> u32 {
        mem.write(actor.wrapping_add(0x6d3), 0xffffu16);
        ms_set_dir_theta_mode(mem, actor)
    }

    /// `MsPushBtlPosAll()` (`0x007ac680`): `MsPushBtlPos` on every actor.
    pub fn ms_push_btl_pos_all(mem: &mut impl Mem) {
        for i in 0..0x1f {
            let a = actor_of(mem, i);
            crate::actor::bridge::ms_push_btl_pos(mem, a);
        }
    }

    /// `g_BtlDamageRates`, which `MsInitParamData` clears: 4 bytes per
    /// actor (31), the damage bonuses and reductions of
    /// `DmgCalc_StatBoostAbility`.
    pub const DAMAGE_RATES: Global<u8> = Global::new(0x0231_1240);
    /// The mimic state `MsInitMimic` clears (0x48 bytes).
    pub const MIMIC: Global<u8> = Global::new(0x0112_be38);
    /// The abilities map `MsInitChrAbilityMap` clears (8 × 0x1c bytes).
    pub const ABILITY_MAP: Global<u8> = Global::new(0x0113_5e00);
    /// The battle's memory `MsBattleMemInit` clears (0x2150 bytes), and a
    /// word after it.
    pub const BATTLE_MEM: Global<u8> = Global::new(0x0112_a8d0);
    pub const BATTLE_MEM_WORD: Global<u32> = Global::new(0x0112_ca30);

    /// `MsInitParamData()` (`0x00785960`): [`DAMAGE_RATES`] cleared.
    pub fn ms_init_param_data(mem: &mut impl Mem) {
        let p = DAMAGE_RATES.ptr(mem).addr;
        crate::util::bridge::ms_struct_clear(mem, p, 0x7c);
    }

    /// `MsInitMimic()` (`0x00799ab0`).
    pub fn ms_init_mimic(mem: &mut impl Mem) {
        let p = MIMIC.ptr(mem).addr;
        crate::util::bridge::ms_struct_clear(mem, p, 0x48);
    }

    /// `MsInitChrAbilityMap()` (`0x00798820`).
    pub fn ms_init_chr_ability_map(mem: &mut impl Mem) {
        let p = ABILITY_MAP.ptr(mem).addr;
        for k in 0..8 {
            crate::util::bridge::ms_struct_clear(mem, p + 0x1c * k, 0x1c);
        }
    }

    /// `MsBattleMemInit()` (`0x00781d00`): the word, the battle's memory,
    /// then `MsEtEffectReadInit`.
    pub fn ms_battle_mem_init(mem: &mut impl Mem) {
        BATTLE_MEM_WORD.set(mem, 0);
        let p = BATTLE_MEM.ptr(mem).addr;
        crate::util::bridge::ms_struct_clear(mem, p, 0x2150);
        crate::misc::bridge::ms_et_effect_read_init(mem);
    }

    /// `frnd()` (`0x00798930`): a float in [1, 2) from `rnd`'s low 23
    /// bits.
    pub fn frnd(mem: &mut impl Mem) -> f32 {
        f32::from_bits((crate::rng::bridge::rnd(mem) & 0x7f_ffff) + 0x3f80_0000)
    }

    /// `MsGetStatusThreat(i)` (`0x0079afc0`): 1 under status bit 0x800
    /// with no provoker (`+0x5c6` 0xff).
    pub fn ms_get_status_threat(mem: &impl Mem, i: u32) -> u32 {
        let a = actor_of(mem, i);
        u32::from(half(mem, a, 0x606) & 0x800 != 0 && byte(mem, a, 0x5c6) == 0xff)
    }

    /// `MsCheckChrHitMotionAll()` (`0x007af2b0`): -1 while an actor that
    /// can act (`+0xdc8`) plays a hit motion (`+0x433`) with `+0x612` set.
    pub fn ms_check_chr_hit_motion_all(mem: &impl Mem) -> i32 {
        let hit = (0..0x1f).any(|i| {
            let a = actor_of(mem, i);
            byte(mem, a, 0xdc8) != 0 && byte(mem, a, 0x433) != 0 && byte(mem, a, 0x612) != 0
        });
        if hit { -1 } else { 0 }
    }

    /// `MsSetPartyRelifeTime()` (`0x0078e0a0`): each party member's HP
    /// (`+0x5d0`) to 0 and `+0x6d8` to `+0x6d9` + 1.
    pub fn ms_set_party_relife_time(mem: &mut impl Mem) {
        for i in crate::menu::bridge::PARTY.get(mem) {
            if i == 0xff {
                continue;
            }
            let a = actor_of(mem, u32::from(i));
            let t = byte(mem, a, 0x6d9).wrapping_add(1);
            mem.write(a.wrapping_add(0x5d0), 0u32);
            mem.write(a.wrapping_add(0x6d8), t);
        }
    }

    /// `MsCheckPlyBtlPosAll()` (`0x00792170`): 1 while the read cue runs,
    /// or while one of the first 18 actors is busy (`+0x508`, marked in
    /// [`crate::body::bridge::MOTION_BUSY`]); else 0.
    pub fn ms_check_ply_btl_pos_all(mem: &mut impl Mem) -> u32 {
        if crate::state::bridge::ms_btl_read_cue_level(mem) != 0 {
            return 1;
        }
        for i in 0..0x12 {
            if byte(mem, actor_of(mem, i), 0x508) != 0 {
                let busy = crate::body::bridge::MOTION_BUSY;
                let m = busy.get(mem);
                busy.set(mem, m | 1 << i);
                return 1;
            }
        }
        0
    }

    /// `MsCommandStartChrStatusAll()` (`0x00793ef0`): every actor's motion
    /// signal (`+0x39c`) cleared.
    pub fn ms_command_start_chr_status_all(mem: &mut impl Mem) {
        for i in 0..0x1f {
            let a = actor_of(mem, i);
            mem.write(a.wrapping_add(0x39c), 0u32);
        }
    }

    /// `MsEffectCheck()` (`0x00787810`): how many actors are in a motion
    /// phase (`+0xdfb`).
    pub fn ms_effect_check(mem: &impl Mem) -> u32 {
        let n = (0..0x1f)
            .filter(|&i| byte(mem, actor_of(mem, i), 0xdfb) != 0)
            .count();
        #[allow(clippy::cast_possible_truncation)] // at most 31
        {
            n as u32
        }
    }

    /// `MsEffectMotionLevel(id)` (`0x00787ca0`): for effect kinds (byte 1)
    /// 2, 4 and 7 actor `id & 0xff`'s motion level (`+0xe0d`, signed), 1
    /// for kind 0x80, else 0.
    pub fn ms_effect_motion_level(mem: &impl Mem, id: u32) -> i32 {
        match (id >> 8) & 0xff {
            2 | 4 | 7 => i32::from(byte(mem, actor_of(mem, id & 0xff), 0xe0d).cast_signed()),
            0x80 => 1,
            _ => 0,
        }
    }

    /// `MsMenuStatusNumSize()` (`0x00790220`): the status table's first
    /// segment's record count.
    pub fn ms_menu_status_num_size(mem: &impl Mem) -> i32 {
        let t = crate::menu::bridge::STATUS_ROM.get(mem);
        ffx_kernel::bridge::ms_get_excel_data_num(mem, t, 0)
    }

    /// `MsProvOff(chr)` (`0x0078d4e0`): ends provoke (status bit 0x400) on
    /// every actor provoked by `chr` (`+0x5c4`).
    pub fn ms_prov_off(mem: &mut impl Mem, chr: u32) {
        for i in 0..0x1f {
            let a = actor_of(mem, i);
            let s = half(mem, a, 0x606);
            if s & 0x400 != 0 && u32::from(byte(mem, a, 0x5c4)) == chr {
                mem.write(a.wrapping_add(0x606), s & 0xfbff);
            }
        }
    }

    /// Encounters on (`MsSaveEncountCheck`): a byte.
    pub const ENCOUNT_ON: Global<u8> = Global::new(0x0112_a9d7);

    /// `MsSaveEncountCheck()` (`0x00785b10`): encounters stay on unless a
    /// joined character (`MsGetSavePlyJoin`) has bit 1 of `g_PlySave`
    /// `+0x4e`; kept in [`ENCOUNT_ON`] and returned.
    pub fn ms_save_encount_check(mem: &mut impl Mem) -> u32 {
        ENCOUNT_ON.set(mem, 1);
        let mut on = 1u8;
        let base = crate::save::bridge::PLY_SAVE.ptr(mem).addr + 0x4e;
        for c in 0..18 {
            if crate::save::bridge::ms_get_save_ply_join(mem, c) != 0
                && byte(mem, base, 0x94 * c) & 2 != 0
            {
                on = 0;
            }
        }
        ENCOUNT_ON.set(mem, on);
        u32::from(on)
    }

    /// The important items' kernel table (`MsImportantName`).
    pub const IMPORTANT_ROM: Global<u32> = Global::new(0x0113_34c0);

    /// `MsImportantName(id)` (`0x00790860`): the key item's name (12
    /// bits), in kana when the save says so.
    pub fn ms_important_name(mem: &impl Mem, id: u32) -> u32 {
        let t = IMPORTANT_ROM.get(mem);
        #[allow(clippy::cast_possible_wrap)] // 12 bits
        let (r, strings) = ffx_kernel::bridge::excel_data(mem, (id & 0xfff) as i32, t);
        let k = crate::save::bridge::ms_get_save_config_hiragana(mem);
        u32::from(half(mem, r, k.wrapping_mul(4))).wrapping_add(strings)
    }

    /// `MsSetSaveConfigEnglish(on)` (`0x00785ed0`): the save's English bit
    /// (16 of the menu config) to `on` (bit 0) unless the language
    /// (`TkGetLang`, from the caller) is English (1) or 0x12, which clear it.
    pub fn ms_set_save_config_english(mem: &mut impl Mem, on: u32, lang: i32) {
        let m = crate::save::bridge::MENU;
        let v = m.get(mem);
        if crate::menu::bridge::ms_get_check_english(lang) != 0 && lang != 0x12 {
            m.set(mem, v ^ ((on << 16 ^ v) & 0x1_0000));
        } else {
            m.set(mem, v & 0xfffe_ffff);
        }
    }

    /// `MsBtlListEncount(i)` (`0x0079d170`): field `i`'s encounter block
    /// (its `i16` `+2` into the second table).
    pub fn ms_btl_list_encount(mem: &impl Mem, i: i32) -> u32 {
        let f = ms_btl_list_field(mem, i);
        let o: i16 = mem.read(f.wrapping_add(2));
        i32::from(o)
            .cast_unsigned()
            .wrapping_add(crate::misc::bridge::FIELD_TABLE2.get(mem))
    }

    // Round 29: actor and save accessors, the wrappers around them.

    /// `MsGetCheckEnglish` (`0x00784e60`), called through the game.
    pub const MS_GET_CHECK_ENGLISH: u32 = 0x0078_4e60;
    /// `op_et_weapon_init(oef2, no)` (vfx), called through the game.
    pub const OP_ET_WEAPON_INIT: u32 = 0x007f_cb60;
    /// `op_et_bindeff_send_common_signal(h, bits)` (vfx), called through
    /// the game.
    pub const OP_ET_SEND_COMMON_SIGNAL: u32 = 0x007f_bdf0;
    /// `TOGetFFXLangJmp` (the thunk to `TOGetFFXLang`, system), called
    /// through the game.
    pub const TO_GET_FFX_LANG_JMP: u32 = 0x008a_c2f0;
    /// The battle message flag `MsGetBattleMessageFlag` checks.
    pub const BATTLE_MESSAGE_OFF: Global<u8> = Global::new(0x0112_ca17);
    /// The spoils `MsFieldItemGetMes` clears (kind 0xb) and fills.
    pub const FIELD_ITEM_MES: Global<u8> = Global::new(0x0231_0ce0);
    /// The password text file: a table of `(offset, _)` pairs, 8 bytes each,
    /// whose offsets are from the file's start (`PasswordText_GetEntry`).
    pub const PASSWORD_TEXT: Global<u32> = Global::new(0x0231_1088);

    fn game<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        sym: u32,
        args: &[u32],
    ) -> u32 {
        let f = mem.rebase(sym);
        call(mem, f, args)
    }

    /// `MsGetRomBtlText(id, second)` (`0x0078f8f0`): menu text 0's name
    /// (`MsMenuGetText(0, id, second)`).
    pub fn ms_get_rom_btl_text(mem: &impl Mem, id: u32, second: u32) -> u32 {
        crate::menu::bridge::ms_menu_get_text(mem, 0, id, second)
    }

    /// `MsMenuGetBtlEndText(id, second)` (`0x0078fb60`): menu text 4's name
    /// (`MsMenuGetText(4, id, second)`).
    pub fn ms_menu_get_btl_end_text(mem: &impl Mem, id: u32, second: u32) -> u32 {
        crate::menu::bridge::ms_menu_get_text(mem, 4, id, second)
    }

    /// `MsSetChrVisibleMain(i, v)` (`0x00796860`): visibility request 0 of
    /// actor `i` (`msSetChrVisibleReq(i, 0, v)`).
    pub fn ms_set_chr_visible_main<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        i: u32,
        v: u32,
    ) -> u32 {
        crate::visible::bridge::ms_set_chr_visible_req(mem, call, i, 0, v)
    }

    /// `MsGetChrLibraName_0078fae0(i)` (`0x0078fae0`): actor `i & 0xff`'s
    /// address plus 0x64d (the Libra window's text, `TOBtlSetLibraWindowParam`).
    pub fn ms_get_chr_libra_name(mem: &impl Mem, i: u32) -> u32 {
        actor_of(mem, i & 0xff).wrapping_add(0x64d)
    }

    /// `MsGetRamChrStr(i)` (`0x0079af40`): byte `+0x5a8` of actor `i`
    /// (`MsGetChr` masks it), zero-extended.
    pub fn ms_get_ram_chr_str(mem: &impl Mem, i: u32) -> u32 {
        u32::from(byte(mem, actor_of(mem, i), 0x5a8))
    }

    /// A byte of actor `i & 0xff` in AL over the actor's address (EAX as
    /// the original's `mov al` leaves it).
    fn byte_over(mem: &impl Mem, i: u32, o: u32) -> u32 {
        let a = actor_of(mem, i & 0xff);
        (a & !0xff) | u32::from(byte(mem, a, o))
    }

    /// `MsMenuAbsElement_0078f910(i)` (`0x0078f910`): the absorbed elements
    /// (`+0x5da`), in AL over the actor's address.
    pub fn ms_menu_abs_element(mem: &impl Mem, i: u32) -> u32 {
        byte_over(mem, i, 0x5da)
    }

    /// `MsMenuInvElement(i)` (`0x0078fe80`): the nullified elements
    /// (`+0x5db`), in AL over the actor's address.
    pub fn ms_menu_inv_element(mem: &impl Mem, i: u32) -> u32 {
        byte_over(mem, i, 0x5db)
    }

    /// `MsMenuWeakElement_00790240(i)` (`0x00790240`): the weak elements
    /// (`+0x5dd`), in AL over the actor's address.
    pub fn ms_menu_weak_element2(mem: &impl Mem, i: u32) -> u32 {
        byte_over(mem, i, 0x5dd)
    }

    /// `MsGetChrLibraWord_0078fac0(i)` (`0x0078fac0`): the `u16` `+0x65a`
    /// of actor `i & 0xff` in AX over the actor's address.
    pub fn ms_get_chr_libra_word(mem: &impl Mem, i: u32) -> u32 {
        let a = actor_of(mem, i & 0xff);
        (a & !0xffff) | u32::from(mem.read::<u16>(a.wrapping_add(0x65a)))
    }

    /// `MsGetChrMotionLevel(i)` (`0x00795640`): the signed byte `+0x420` of
    /// actor `i & 0xff`.
    pub fn ms_get_chr_motion_level(mem: &impl Mem, i: u32) -> u32 {
        i32::from(byte(mem, actor_of(mem, i & 0xff), 0x420).cast_signed()).cast_unsigned()
    }

    /// `MsMotionSpeedResetChr_007aaff0(i)` (`0x007aaff0`): `MsMotionSpeedReset`
    /// on actor `i & 0xff`; `EAX` is the actor, as that leaves it.
    pub fn ms_motion_speed_reset_chr(mem: &mut impl Mem, i: u32) -> u32 {
        let a = actor_of(mem, i & 0xff);
        crate::actor::bridge::ms_motion_speed_reset(mem, a);
        a
    }

    /// `MsGetBattleMessageFlag()` (`0x00784e40`): -1 when `MsGetCheckEnglish`
    /// (through `call`) is not 0 and [`BATTLE_MESSAGE_OFF`] is 0, else 0.
    pub fn ms_get_battle_message_flag<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
    ) -> u32 {
        let english = game(mem, call, MS_GET_CHECK_ENGLISH, &[]);
        if english != 0 && BATTLE_MESSAGE_OFF.get(mem) == 0 {
            u32::MAX
        } else {
            0
        }
    }

    /// `MsSetSaveCommandWithPrefix(chr, id, on)` (`0x00874200`):
    /// `MsSetSaveCommand(chr, 0x3000 | id & 0xfff, on)`.
    pub fn ms_set_save_command_with_prefix(mem: &mut impl Mem, chr: u32, id: u32, on: u32) -> u32 {
        crate::ability::bridge::ms_set_save_command(mem, chr, id & 0xfff | 0x3000, on)
    }

    /// `MsEffectWeaponInit(oef2, i)` (`0x00788b70`): actor `i`'s byte
    /// `+0x35` set, then `op_et_weapon_init(oef2, i)` (through `call`; the
    /// original jumps to it), whose result it returns.
    pub fn ms_effect_weapon_init<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        oef2: u32,
        i: u32,
    ) -> u32 {
        let a = actor_of(mem, i);
        mem.write(a.wrapping_add(0x35), 1u8);
        game(mem, call, OP_ET_WEAPON_INIT, &[oef2, i])
    }

    /// `MsFieldItemGet(i)` (`0x00798fd0`): field item `i` into the battle's
    /// spoils (`msFieldItemGet(i, g_BtlCountsBattle)`); returns them.
    pub fn ms_field_item_get_battle<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        i: u32,
    ) -> u32 {
        let spoils = crate::flow::bridge::COUNTS_BATTLE.ptr(mem).addr;
        crate::reward::bridge::ms_field_item_get(mem, call, i, spoils);
        spoils
    }

    /// `MsFieldItemGetMes(i)` (`0x00798ff0`): [`FIELD_ITEM_MES`] cleared
    /// (`MsBtlGetClear_007993e0(it, 0xb)`), field item `i` into it; returns
    /// it.
    pub fn ms_field_item_get_mes<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        i: u32,
    ) -> u32 {
        let spoils = FIELD_ITEM_MES.ptr(mem).addr;
        crate::init::bridge::ms_btl_get_clear(mem, spoils, 0xb);
        crate::reward::bridge::ms_field_item_get(mem, call, i, spoils);
        spoils
    }

    /// `MsGetRamChrFemale(i)` (`0x0079adb0`): bit 1 of actor `i`'s `+0x590`.
    pub fn ms_get_ram_chr_female(mem: &impl Mem, i: u32) -> u32 {
        u32::from(byte(mem, actor_of(mem, i), 0x590)) >> 1 & 1
    }

    /// `MsClearChrDfd_007b0500(cue)` (`0x007b0500`): byte `+0xdfd` of the
    /// actor the cue names (its byte `+0xc`) cleared.
    pub fn ms_clear_chr_dfd(mem: &mut impl Mem, cue: u32) {
        let i = u32::from(byte(mem, cue, 0xc));
        let a = actor_of(mem, i);
        mem.write(a.wrapping_add(0xdfd), 0u8);
    }

    /// `MsEffectSendSignal(i, bits)` (`0x0079e4f0`): `bits` to actor `i`'s
    /// character (`MsGetChrHandle`) through
    /// `op_et_bindeff_send_common_signal(handle, bits)` (`call`), its result;
    /// 0 without a character.
    pub fn ms_effect_send_signal<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        i: u32,
        bits: u32,
    ) -> u32 {
        let h = crate::actor::bridge::ms_get_chr_handle(mem, i);
        if h == 0 {
            return 0;
        }
        game(mem, call, OP_ET_SEND_COMMON_SIGNAL, &[h, bits])
    }

    /// `MsChrMotionSpeedOn_007aaac0(i)` (`0x007aaac0`): 1 when actor `i`'s
    /// motion speed byte (`+0x415`, which `MsMotionSpeedReset` clears) is
    /// set.
    pub fn ms_chr_motion_speed_on(mem: &impl Mem, i: u32) -> u32 {
        u32::from(byte(mem, actor_of(mem, i), 0x415) != 0)
    }

    /// `PasswordGetTotalIndex()` (`0x007902d0`): 9 when the language
    /// (`TOGetFFXLang` through its thunk and `call`) is 9; else, asking
    /// again, 10 for 10 and 0 for the rest.
    pub fn password_get_total_index<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
    ) -> u32 {
        let lang = game(mem, call, TO_GET_FFX_LANG_JMP, &[]);
        if lang == 9 {
            return 9;
        }
        if game(mem, call, TO_GET_FFX_LANG_JMP, &[]) == 10 {
            10
        } else {
            0
        }
    }

    /// `MsCheckDefenseMotion(actor)` (`0x007af2f0`): 1 when the actor's
    /// `u16` `+0x616` has a bit of 0x2840, else 0.
    pub fn ms_check_defense_motion(mem: &impl Mem, actor: u32) -> u32 {
        u32::from(mem.read::<u16>(actor.wrapping_add(0x616)) & 0x2840 != 0)
    }

    /// `MsEffectChrMotionSync(i)` (`0x007b7e80`): 1 while actor `i & 0xff`
    /// has a motion (`+0x432` set), else 0.
    pub fn ms_effect_chr_motion_sync(mem: &impl Mem, i: u32) -> u32 {
        u32::from(byte(mem, actor_of(mem, i & 0xff), 0x432) != 0)
    }

    /// `MsFldGetSceneHigh_00782940(scene)` (`0x00782940`): the high nibble of
    /// byte `+1` of the scene's formation (`MsFldGetScene(scene, 0, 0, 0)`),
    /// 0 without one.
    pub fn ms_fld_get_scene_high(mem: &mut impl Mem, scene: u32) -> u32 {
        let f = crate::process::bridge::ms_fld_get_scene(mem, scene, 0, 0, 0);
        if f == 0 {
            0
        } else {
            u32::from(byte(mem, f, 1)) >> 4
        }
    }

    /// `MsSetSavePartyMember(a, b, c)` (`0x00786950`): the three into front
    /// slots 0, 1, 2 (`MsExchangeSaveParty`), in that order; the last one's
    /// result.
    pub fn ms_set_save_party_member(mem: &mut impl Mem, a: u32, b: u32, c: u32) -> u32 {
        use crate::save::bridge::ms_exchange_save_party;
        ms_exchange_save_party(mem, a, 0);
        ms_exchange_save_party(mem, b, 1);
        ms_exchange_save_party(mem, c, 2)
    }

    /// `MsSetSavePlyBit4_00786aa0(chr, on)` (`0x00786aa0`): bit 4 of
    /// character `chr & 0xff`'s `ply_save` flags (`+0x2c`, which
    /// `MsSetSavePlyJoin` sets with bit 0) to bit 0 of `on`.
    pub fn ms_set_save_ply_bit4(mem: &mut impl Mem, chr: u32, on: u32) {
        let at = crate::save::bridge::PLY_SAVE.ptr(mem).addr + (chr & 0xff) * 0x94 + 0x2c;
        let old: u8 = mem.read(at);
        let bit = on.to_le_bytes()[0] << 4;
        mem.write(at, old ^ ((bit ^ old) & 0x10));
    }

    /// `PasswordText_GetEntry_007902a0(i)` (`0x007902a0`): entry `i` of the
    /// password text file ([`PASSWORD_TEXT`], through `get_ptr32_func`: the
    /// pointer itself): the file plus the offset at `+8 i`.
    pub fn password_text_get_entry(mem: &impl Mem, i: u32) -> u32 {
        let file = PASSWORD_TEXT.get(mem);
        let off: u32 = mem.read(file.wrapping_add(i.wrapping_mul(8)));
        file.wrapping_add(off)
    }

    // Round 29 C: more accessors, loops over the actors, the equipment.

    /// `TkCdSize(file, _)` (menu), called through the game.
    pub const TK_CD_SIZE: u32 = 0x0088_d390;
    /// `TKMallocAndReadEventMessage(name)` (menu), called through the game.
    pub const TK_READ_EVENT_MESSAGE: u32 = 0x0088_cac0;
    /// `op_et_eff_off_signal(id)` (vfx), called through the game.
    pub const OP_ET_EFF_OFF_SIGNAL: u32 = 0x007f_c450;
    /// "monster1", "monster2", "monster3": the files `readBtlMonsterName`
    /// reads into [`crate::read::bridge::MONSTER_TEXTS`].
    pub const MONSTER_TEXT_NAMES: [u32; 3] = [0x00b5_48fc, 0x00b5_4908, 0x00b5_4914];
    /// The size of file 0x2a in KB (rounded up) plus 0x20, cached by
    /// `TkCdSizeKBCached_008741d0` (negative until then).
    pub const CD_SIZE_2A: Global<i32> = Global::new(0x00c5_2760);
    /// The weapon spoils' list position (`MsExchangeSaveWeapon`: the list
    /// `[g_BtlEndSpoilWeapons]` at `+0x0186a190` + `+0x0186a1a0`) and the
    /// equipped slot's index into the `u16`s at `0x02310f8e`.
    pub const SPOIL_WEAPON_TOP: Global<u32> = Global::new(0x0186_a190);
    pub const SPOIL_WEAPON_CURSOR: Global<u32> = Global::new(0x0186_a1a0);
    pub const SPOIL_WEAPON_SLOT: Global<u32> = Global::new(0x0186_a1a4);
    pub const SPOIL_WEAPON_SLOTS: Global<u16> = Global::new(0x0231_0f8e);
    pub const SPOIL_WEAPONS: Global<u32> = Global::new(0x025c_c580);
    /// 200 equipment slots of 22 bytes (`g_Equipment`): `+2` in use, `+3`
    /// flags (8 held, 2 the one `0x007ad710` marks).
    pub const EQUIPMENT: Global<u8> = Global::new(0x0113_0f2c);

    /// `MsBtlChrNumCheck`'s actors only: `get` of actor `i & 0xff`, 0 for
    /// the others.
    fn actor_only(mem: &impl Mem, i: u32, get: impl FnOnce(u32) -> u32) -> u32 {
        let i = i & 0xff;
        if crate::actor::bridge::ms_btl_chr_num_check(i) == 0 {
            return 0;
        }
        get(actor_of(mem, i))
    }

    /// A signed byte of an actor, sign-extended.
    fn sbyte(mem: &impl Mem, a: u32, o: u32) -> u32 {
        i32::from(byte(mem, a, o).cast_signed()).cast_unsigned()
    }

    /// `MsGetChrEffectVariable(i)` (`0x00794e80`): byte `+0xe0c` of an actor
    /// in AL over its address; 0 for the others.
    pub fn ms_get_chr_effect_variable(mem: &impl Mem, i: u32) -> u32 {
        actor_only(mem, i, |a| (a & !0xff) | u32::from(byte(mem, a, 0xe0c)))
    }

    /// `MsGetChrSbyte508_00794240(i)` (`0x00794240`): the signed byte
    /// `+0x508` of an actor; 0 for the others.
    pub fn ms_get_chr_sbyte508(mem: &impl Mem, i: u32) -> u32 {
        actor_only(mem, i, |a| sbyte(mem, a, 0x508))
    }

    /// `MsGetChrSbyte4fc_00794280(i)` (`0x00794280`): the signed byte
    /// `+0x4fc` of an actor; 0 for the others.
    pub fn ms_get_chr_sbyte4fc(mem: &impl Mem, i: u32) -> u32 {
        actor_only(mem, i, |a| sbyte(mem, a, 0x4fc))
    }

    /// `MsGetChrByte41f_00794d70(i)` (`0x00794d70`): byte `+0x41f` of an
    /// actor; 0 for the others.
    pub fn ms_get_chr_byte41f(mem: &impl Mem, i: u32) -> u32 {
        actor_only(mem, i, |a| u32::from(byte(mem, a, 0x41f)))
    }

    /// `MsGetChrFly(i)` (`0x00794f70`): the signed fly byte `+0x505` of an
    /// actor; 0 for the others.
    pub fn ms_get_chr_fly(mem: &impl Mem, i: u32) -> u32 {
        actor_only(mem, i, |a| sbyte(mem, a, 0x505))
    }

    /// `MsGetChrByte3f4_00795a60(i)` (`0x00795a60`): byte `+0x3f4` of an
    /// actor; 0 for the others.
    pub fn ms_get_chr_byte3f4(mem: &impl Mem, i: u32) -> u32 {
        actor_only(mem, i, |a| u32::from(byte(mem, a, 0x3f4)))
    }

    /// `MsSetChrDirLeave(i)` (`0x00795e70`): an actor with `+0x42e` set
    /// gets 0xc in `+0x42b`.
    pub fn ms_set_chr_dir_leave(mem: &mut impl Mem, i: u32) {
        let a = actor_only(mem, i, |a| a);
        if a != 0 && byte(mem, a, 0x42e) != 0 {
            mem.write(a.wrapping_add(0x42b), 0xcu8);
        }
    }

    /// `MsGetChrHeadPos(i, out)` (`0x007950b0`): with a character
    /// (`MsGetChrHandle`), its point 1 (`Ch_OptposGet`) into `out`, `y`
    /// lowered by 2.0 (`g_Two`, an `f64`, rounded to `f32`).
    pub fn ms_get_chr_head_pos(mem: &mut impl Mem, i: u32, out: u32) {
        use ffx_bridge::x87::{load, store, sub};
        let h = crate::actor::bridge::ms_get_chr_handle(mem, i);
        if h == 0 {
            return;
        }
        ffx_chara::attach::bridge::ch_optpos_get(mem, h, 1, out);
        let y: u32 = mem.read(out.wrapping_add(4));
        let two = crate::screen::bridge::TWO.get(mem);
        mem.write(out.wrapping_add(4), store(sub(load(y), two)));
    }

    /// `MsSetMonsVisible_00796880(v)` (`0x00796880`): `MsSetChrVisible(i,
    /// v)` for the monsters 0x14..0x1c; 8.
    pub fn ms_set_mons_visible<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        v: u32,
    ) -> u32 {
        for i in 0x14..0x1c {
            crate::visible::bridge::ms_set_chr_visible(mem, call, i, v);
        }
        8
    }

    /// `MsSetPlayersVisible_007968b0(v)` (`0x007968b0`): `MsSetChrVisible(i,
    /// v)` for the players 0..0x12; 0x12.
    pub fn ms_set_players_visible<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        v: u32,
    ) -> u32 {
        for i in 0..0x12 {
            crate::visible::bridge::ms_set_chr_visible(mem, call, i, v);
        }
        0x12
    }

    /// `MsSetAllBodyHit_00795da0(on)` (`0x00795da0`): `msSetChrBodyHit(actor,
    /// 2, on)` for the 31 actors.
    pub fn ms_set_all_body_hit(mem: &mut impl Mem, on: u32) {
        for i in 0..0x1f {
            let a = actor_of(mem, i);
            crate::body::bridge::ms_set_chr_body_hit(mem, a, 2, on.cast_signed());
        }
    }

    /// `MsClearBodyHitBits_00793dc0(bits)` (`0x00793dc0`): `msSetChrBodyHit(
    /// actor, 2, 0)` for the actors (below 31) whose bit is set in `bits`.
    pub fn ms_clear_body_hit_bits(mem: &mut impl Mem, bits: u32) {
        for i in 0..0x1f {
            if bits >> i & 1 != 0 {
                let a = actor_of(mem, i);
                crate::body::bridge::ms_set_chr_body_hit(mem, a, 2, 0);
            }
        }
    }

    /// `MsBattleStatusMenuless_007a8c20()` (`0x007a8c20`): 1 for the battle
    /// statuses 1, 2, 0x18, 0x1b and 0x1e, else 0.
    pub fn ms_battle_status_menuless(mem: &impl Mem) -> u32 {
        let st = crate::state::bridge::STATUS.get(mem);
        u32::from(matches!(st, 1 | 2 | 0x18 | 0x1b | 0x1e))
    }

    /// `MsBattleStartReady_00781650()` (`0x00781650`): for the encounter
    /// types 1-3 ([`crate::process::bridge::ENCOUNT_TYPE2`]) 1 when the
    /// status is 0x15, 0x16 or 1; for the others 1 while
    /// `g_BtlStartReady` is set; else 0.
    pub fn ms_battle_start_ready(mem: &impl Mem) -> u32 {
        let kind = crate::process::bridge::ENCOUNT_TYPE2.get(mem);
        if kind.wrapping_sub(1) <= 2 {
            let st = crate::state::bridge::STATUS.get(mem).cast_unsigned();
            u32::from(matches!(st, 0x15 | 0x16 | 1))
        } else {
            u32::from(START_READY.get(mem) != 0)
        }
    }

    /// `g_BtlStartReady` (`MsBattleStartReady_00781650`).
    pub const START_READY: Global<u8> = Global::new(0x0112_c9cc);

    /// `MsSaveAddHP(chr, n)` (`0x007859d0`): the character's HP (`+0x1c`)
    /// plus `n` (wrapping), kept within 0..=max HP (`+0x24`,
    /// `MsCheckRange`).
    pub fn ms_save_add_hp(mem: &mut impl Mem, chr: u32, n: u32) {
        save_add(mem, chr, n, 0x1c);
    }

    /// `MsSaveAddMP_00785ae0(chr, n)` (`0x00785ae0`): [`ms_save_add_hp`] for
    /// the MP (`+0x20`, max `+0x28`).
    pub fn ms_save_add_mp(mem: &mut impl Mem, chr: u32, n: u32) {
        save_add(mem, chr, n, 0x20);
    }

    fn save_add(mem: &mut impl Mem, chr: u32, n: u32, at: u32) {
        let p = crate::save::bridge::ms_get_save_player_ptr(mem, chr);
        let v = mem.read::<u32>(p.wrapping_add(at)).wrapping_add(n);
        mem.write(p.wrapping_add(at), v);
        let max: i32 = mem.read(p.wrapping_add(at + 8));
        let v = ffx_kernel::bridge::ms_check_range(v.cast_signed(), 0, max);
        mem.write(p.wrapping_add(at), v);
    }

    /// `MsSetWeaponName(gear)` (`0x007993b0`): the gear record's name number
    /// (`MsWeaponNameNum`) into its `u16` `+0`, then its name
    /// (`MsWeaponName(number, +4, 0, gear + 0xc)`); that call's result.
    pub fn ms_set_weapon_name(mem: &mut impl Mem, gear: u32) -> u32 {
        let n = crate::gear::bridge::ms_weapon_name_num(mem, gear);
        let n = n & 0xffff;
        #[allow(clippy::cast_possible_truncation)] // the original's `mov word`
        mem.write(gear, n as u16);
        let kind = u32::from(byte(mem, gear, 4));
        crate::menu::bridge::ms_weapon_name(mem, n, kind, 0, gear.wrapping_add(0xc))
    }

    /// `MsWaitMotionPick_007a8c50(chr, actor, v)` (`0x007a8c50`): `v`,
    /// unless the character waits (`Ch_GetWaitMotion`) and the actor's
    /// `+0x434` is 0, then 0.
    pub fn ms_wait_motion_pick(mem: &impl Mem, chr: u32, actor: u32, v: u32) -> u32 {
        if ffx_chara::parts::bridge::ch_get_wait_motion(mem, chr) != 0
            && byte(mem, actor, 0x434) == 0
        {
            0
        } else {
            v
        }
    }

    /// The equipment slots in use (`+2`) whose flags (`+3`) pass `pick`:
    /// the first one's flags byte, or `None`.
    fn equipment_find(mem: &impl Mem, pick: impl Fn(u8) -> bool) -> Option<u32> {
        let base = EQUIPMENT.ptr(mem).addr;
        (0..200)
            .map(|k| base + 22 * k + 3)
            .find(|&f| mem.read::<u8>(f - 1) != 0 && pick(byte(mem, f, 0)))
    }

    /// `MsEquipmentHasUnmarked_007ad6e0()` (`0x007ad6e0`): 1 when a held
    /// piece of equipment (in use, flag 8) lacks flag 2, else 0.
    pub fn ms_equipment_has_unmarked(mem: &impl Mem) -> u32 {
        u32::from(equipment_find(mem, |f| f & 8 != 0 && f & 2 == 0).is_some())
    }

    /// `MsEquipmentMarkFirst_007ad710(on)` (`0x007ad710`): the first held
    /// piece of equipment loses flag 2 with `on`, gets it without; 1, or 0
    /// with none held.
    pub fn ms_equipment_mark_first(mem: &mut impl Mem, on: u32) -> u32 {
        let Some(f) = equipment_find(mem, |f| f & 8 != 0) else {
            return 0;
        };
        let v = byte(mem, f, 0);
        mem.write(f, if on != 0 { v & !2 } else { v | 2 });
        1
    }

    /// `MsWeaponSetTransparent_007ad5b0(i, n, frames)` (`0x007ad5b0`): actor
    /// `i`'s weapon `n` (`MsGetWeapon`) with a character fades to alpha 0.0
    /// over `frames` (`Ch_SetTransparent`).
    pub fn ms_weapon_set_transparent(mem: &mut impl Mem, i: u32, n: u32, frames: u32) {
        let w = crate::actor::bridge::ms_get_weapon(mem, i, n.cast_signed());
        let chr: u32 = mem.read(w);
        if chr != 0 {
            use ffx_chara::{chr::bridge::ch_set_fade, layout::FADE_ALPHA};
            ch_set_fade(mem, chr, FADE_ALPHA, 0.0, frames.cast_signed());
        }
    }

    /// `MsGetSaveChrArmorModel(chr)` (`0x007abb70`): the model (`+0xc`) of
    /// the character's armor (`ply_save` `+0x2e`, `MsGetSaveWeapon`), 0
    /// without one.
    pub fn ms_get_save_chr_armor_model(mem: &mut impl Mem, chr: u32) -> u32 {
        save_chr_gear_model(mem, chr, 0x2e)
    }

    /// `MsGetSaveChrWeaponModel(chr)` (`0x007abba0`): as
    /// [`ms_get_save_chr_armor_model`] for the weapon (`+0x2d`).
    pub fn ms_get_save_chr_weapon_model(mem: &mut impl Mem, chr: u32) -> u32 {
        save_chr_gear_model(mem, chr, 0x2d)
    }

    fn save_chr_gear_model(mem: &mut impl Mem, chr: u32, slot: u32) -> u32 {
        let ply = crate::save::bridge::PLY_SAVE.ptr(mem).addr;
        let id = u32::from(byte(mem, ply.wrapping_add(chr.wrapping_mul(0x94)), slot));
        let w = crate::save::bridge::ms_get_save_weapon(mem, id, 0);
        if w == 0 {
            0
        } else {
            u32::from(mem.read::<u16>(w.wrapping_add(0xc)))
        }
    }

    /// `MsExchangeSaveWeapon()` (`0x008be0a0`): the results screen's
    /// exchange: `SwapSaveGear(spoil, equipped)`, the spoil the `u16` of
    /// [`SPOIL_WEAPONS`] at the list's top plus cursor, the equipped one
    /// [`SPOIL_WEAPON_SLOTS`]`[SPOIL_WEAPON_SLOT]`; its result.
    pub fn ms_exchange_save_weapon(mem: &mut impl Mem) -> u32 {
        let slot = SPOIL_WEAPON_SLOT.get(mem);
        let equipped = SPOIL_WEAPON_SLOTS
            .ptr(mem)
            .addr
            .wrapping_add(slot.wrapping_mul(2));
        let equipped = u32::from(mem.read::<u16>(equipped));
        let k = SPOIL_WEAPON_TOP
            .get(mem)
            .wrapping_add(SPOIL_WEAPON_CURSOR.get(mem));
        let list = SPOIL_WEAPONS.get(mem);
        let spoil = u32::from(mem.read::<u16>(list.wrapping_add(k.wrapping_mul(2))));
        crate::save::bridge::swap_save_gear(mem, spoil, equipped).cast_unsigned()
    }

    /// `TkCdSizeKBCached_008741d0()` (`0x008741d0`): while [`CD_SIZE_2A`] is
    /// negative, it becomes file 0x2a's size (`TkCdSize(0x2a, 0)`, through
    /// `call`) in KB rounded up (signed) plus 0x20. `EAX` ends as the
    /// cached value, or as it was read.
    pub fn tk_cd_size_kb_cached<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
    ) -> u32 {
        let v = CD_SIZE_2A.get(mem);
        if v >= 0 {
            return v.cast_unsigned();
        }
        let size = game(mem, call, TK_CD_SIZE, &[0x2a, 0]).cast_signed();
        let kb = size.wrapping_add(0x3ff) / 0x400;
        let v = kb.wrapping_add(0x20);
        CD_SIZE_2A.set(mem, v);
        v.cast_unsigned()
    }

    /// `MsCountSlotsOf_007b1e40(p, v)` (`0x007b1e40`): how many of the three
    /// bytes at `p` are `v`.
    pub fn ms_count_slots_of(mem: &impl Mem, p: u32, v: u32) -> u32 {
        (0..3)
            .map(|k| u32::from(u32::from(byte(mem, p, k)) == v))
            .sum()
    }

    /// `MsEffectCameraGet(owner, k, kind)` (`0x007877d0`): effect camera `k`
    /// (0-7, unsigned) of [`EFFECT_CAMERA_BUF`]: kept, or got now
    /// (`MsCameraGet(owner, k, kind)`) and kept; 0 past 7.
    pub fn ms_effect_camera_get(mem: &mut impl Mem, owner: u32, k: u32, kind: u32) -> u32 {
        if k > 7 {
            return 0;
        }
        let at = EFFECT_CAMERA_BUF.ptr(mem).addr + 4 * k;
        let v: u32 = mem.read(at);
        if v != 0 {
            return v;
        }
        let v = crate::camera::bridge::ms_camera_get(mem, owner, k, kind);
        mem.write(at, v);
        v
    }

    /// `MsImportantReset_007907a0(id)` (`0x007907a0`): key item `id & 0xfff`
    /// (below 0x80) no longer held (its bit in `g_SaveImportant` cleared).
    pub fn ms_important_reset(mem: &mut impl Mem, id: u32) {
        let id = id & 0xfff;
        if id < 0x80 {
            let at = crate::save::bridge::IMPORTANT.ptr(mem).addr + 2 * (id >> 4);
            let w: u16 = mem.read(at);
            mem.write(at, w & !(1 << (id & 0xf)));
        }
    }

    /// `MsResetWeaponEffectAttribute(_, actor)` (`0x0079ea90`): each of the
    /// actor's four weapon effects (`+0xe90`) set is turned off
    /// (`op_et_eff_off_signal`, through `call`) and cleared; how many.
    pub fn ms_reset_weapon_effect_attribute<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        actor: u32,
    ) -> u32 {
        let mut n = 0;
        for k in 0..4 {
            let at = actor.wrapping_add(0xe90 + 4 * k);
            let e: u32 = mem.read(at);
            if e != 0 {
                n += 1;
                game(mem, call, OP_ET_EFF_OFF_SIGNAL, &[e]);
                mem.write(at, 0u32);
            }
        }
        n
    }

    /// `MsCueMoveSpeed_007a88e0(cue)` (`0x007a88e0`): the cue's `f32`
    /// `+0x4d4` (through a local), doubled for kind 0x1d (`+0xc`) when its
    /// actor (`+0x14`) has `+0x3fb` set; the `f32` left in ST0, as bits.
    pub fn ms_cue_move_speed(mem: &impl Mem, cue: u32) -> u32 {
        use ffx_bridge::x87::{add, load, store};
        let f = store(load(mem.read(cue.wrapping_add(0x4d4))));
        if byte(mem, cue, 0xc) == 0x1d {
            let a = actor_of(mem, u32::from(byte(mem, cue, 0x14)));
            if byte(mem, a, 0x3fb) != 0 {
                return store(add(load(f), load(f)));
            }
        }
        f
    }

    /// `readBtlMonsterName()` (`0x007844c0`): once (while the first is
    /// null), the three monster text files ([`MONSTER_TEXT_NAMES`]) read
    /// (`TKMallocAndReadEventMessage`, through `call`) into
    /// [`crate::read::bridge::MONSTER_TEXTS`].
    pub fn read_btl_monster_name<M: Mem>(mem: &mut M, call: crate::exec::bridge::Call<'_, M>) {
        use crate::read::bridge::MONSTER_TEXTS;
        if mem.read::<u32>(mem.rebase(MONSTER_TEXTS[0])) != 0 {
            return;
        }
        for (to, name) in MONSTER_TEXTS.into_iter().zip(MONSTER_TEXT_NAMES) {
            let name = mem.rebase(name);
            let p = game(mem, call, TK_READ_EVENT_MESSAGE, &[name]);
            let to = mem.rebase(to);
            mem.write(to, p);
        }
    }

    /// `MsEffectReadStatInit()` (`0x007881c0`): with
    /// `g_BtlEffectReadClear` not negative, the loading actor's
    /// (`g_BtlEffectReadChr`) `+0xdfd` and `+0xf8c` cleared and that actor
    /// 0; else `g_BtlEventEffectStat` 0.
    pub fn ms_effect_read_stat_init(mem: &mut impl Mem) {
        use crate::parse::bridge::{EFFECT_READ_CHR, EFFECT_READ_CLEAR};
        if EFFECT_READ_CLEAR.get(mem).cast_signed() >= 0 {
            let a = actor_of(mem, EFFECT_READ_CHR.get(mem));
            mem.write(a.wrapping_add(0xdfd), 0u8);
            mem.write(a.wrapping_add(0xf8c), 0u32);
            EFFECT_READ_CHR.set(mem, 0);
        } else {
            crate::state::bridge::EVENT_EFFECT_STAT.set(mem, 0);
        }
    }

    // Round 29 D: setters and checks on the actors, the message and effect
    // glue, the equipment list's free slot.

    /// Functions of other modules the group calls through the game:
    /// `yiGetBattleCenter(out)` (map), `TODrawAMES_CENTERMES(window, text)`,
    /// `FFXIsDebug()` (system), `dbgPrintf` (platform),
    /// `AtelGetBattleIdAdrs(i)` (script), `op_et_bindeff_draw_on`/`_off(h)`
    /// and `op_ovl_effect_init()` (vfx), `graphicIsMagicFileAsyncLoadAvailable`
    /// (render), `MagicFile_Start`, `MagicFile_GetEffectOverlayTable(0)`,
    /// the free at `0x00687010` and `is_ptr32_null` (system).
    pub const YI_GET_BATTLE_CENTER: u32 = 0x0090_87f0;
    pub const TO_DRAW_AMES_CENTERMES: u32 = 0x0089_b1d0;
    pub const FFX_IS_DEBUG: u32 = 0x0088_7c50;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const ATEL_GET_BATTLE_ID_ADRS: u32 = 0x0086_a970;
    pub const OP_ET_BINDEFF_DRAW_ON: u32 = 0x007f_bab0;
    pub const OP_ET_BINDEFF_DRAW_OFF: u32 = 0x007f_ba40;
    pub const OP_OVL_EFFECT_INIT: u32 = 0x007f_cdd0;
    pub const GRAPHIC_MAGIC_ASYNC: u32 = 0x0064_1c60;
    pub const MAGIC_FILE_START: u32 = 0x009d_a7f0;
    pub const MAGIC_FILE_OVERLAY_TABLE: u32 = 0x009d_a370;
    pub const FREE_00687010: u32 = 0x0068_7010;
    pub const IS_PTR32_NULL: u32 = 0x00a4_46a0;
    /// "debug:RAND %04x %d<%d\n" (`MsRandCheck_007a8ad0`).
    pub const RAND_FORMAT: u32 = 0x00b5_4ee4;

    /// The file whose tables `SetFileTables_00776f00` resolves (offsets
    /// `+8`, `+0xc`, `+0x14` into it, `+4` a word) and a second one
    /// (`+0x10`); the resolved pointers.
    pub const TABLE_FILE: Global<u32> = Global::new(0x0112_a03c);
    pub const TABLE_FILE2: Global<u32> = Global::new(0x0112_a040);
    pub const TABLE_08: Global<u32> = Global::new(0x0112_a01c);
    pub const TABLE_0C: Global<u32> = Global::new(0x0112_a020);
    pub const TABLE2_10: Global<u32> = Global::new(0x0112_a024);
    pub const TABLE_14: Global<u32> = Global::new(0x0112_a028);
    pub const TABLE_WORD4: Global<u32> = Global::new(0x0112_a080);
    /// Eight `(u16, u16, u16)` entries after the count
    /// [`crate::misc::bridge::COM_BUF`] (`MsComBufAdd_0079d2a0`).
    pub const COM_BUF_ENTRIES: Global<u16> = Global::new(0x0113_6074);
    /// Set: `MsGetMapCenter` gives the battle's own center (the first four
    /// words of [`BATTLE_MEM`]).
    pub const MAP_CENTER_OWN: Global<u8> = Global::new(0x0112_c9fd);
    /// The battle message `MsShowBattleMessage_0079d090` was asked for, the
    /// message table (`u16` offsets, two per message and kana mode) and the
    /// window system's "messages on" flag.
    pub const BATTLE_MESSAGE: Global<u32> = Global::new(0x0112_c250);
    pub const BATTLE_MESSAGES: Global<u32> = Global::new(0x0112_a998);
    pub const MESSAGE_WINDOWS_ON: Global<u32> = Global::new(0x0184_1d20);
    /// The fixed random check (`MsRandCheck_007a8ad0`: 0 for none).
    pub const RAND_FIXED: Global<u16> = Global::new(0x0113_6410);
    /// The actor an effect waits on and its motion (`MsEffectCheckMotion2`).
    pub const EFFECT_WAIT: Global<[u8; 2]> = Global::new(0x0112_ca08);
    /// The event effect's overlay table, and the constant it starts as.
    pub const EVENT_EFFECT_TABLE: Global<u32> = Global::new(0x0113_3360);
    pub const EVENT_EFFECT_TABLE0: u32 = 0x0134_0870;
    /// Three 12-byte records (`+0` on, `+4` a character) at `g_BtlChrExtra`.
    pub const CHR_EXTRA: Global<u32> = Global::new(0x0231_1050);
    /// The password text's loaded flag (`PasswordTextDispose`).
    pub const PASSWORD_TEXT_ON: Global<u32> = Global::new(0x0231_1084);

    /// `SetFileTables_00776f00()` (`0x00776f00`): with [`TABLE_FILE`] loaded,
    /// its tables at `+8`, `+0xc`, `+0x14` (offsets from the file) and its
    /// word `+4`, and [`TABLE_FILE2`]'s table at `+0x10`, into their
    /// globals.
    pub fn set_file_tables(mem: &mut impl Mem) {
        let d = TABLE_FILE.get(mem);
        if d == 0 {
            return;
        }
        let at = |mem: &mut _, o: u32| d.wrapping_add(read_u32(mem, d.wrapping_add(o)));
        let t = at(mem, 8);
        TABLE_08.set(mem, t);
        let w = read_u32(mem, d.wrapping_add(4));
        TABLE_WORD4.set(mem, w);
        let t = at(mem, 0xc);
        TABLE_0C.set(mem, t);
        let e = TABLE_FILE2.get(mem);
        let t2 = e.wrapping_add(read_u32(mem, e.wrapping_add(0x10)));
        let t = at(mem, 0x14);
        TABLE2_10.set(mem, t2);
        TABLE_14.set(mem, t);
    }

    fn read_u32(mem: &impl Mem, at: u32) -> u32 {
        mem.read(at)
    }

    /// `MsComBufAdd_0079d2a0(a, b, c)` (`0x0079d2a0`): the three `u16`s as
    /// the next entry of [`COM_BUF_ENTRIES`] while fewer than 8 (the count
    /// signed); 1, or 0 when full.
    pub fn ms_com_buf_add(mem: &mut impl Mem, first: u32, second: u32, third: u32) -> u32 {
        use crate::misc::bridge::COM_BUF;
        let count = COM_BUF.get(mem);
        if count.cast_signed() >= 8 {
            return 0;
        }
        COM_BUF.set(mem, count.wrapping_add(1));
        let entry = COM_BUF_ENTRIES
            .ptr(mem)
            .addr
            .wrapping_add(count.wrapping_mul(6));
        #[allow(clippy::cast_possible_truncation)] // the original's `mov word`s
        for (k, v) in (0..).zip([first, second, third]) {
            mem.write(entry.wrapping_add(2 * k), v as u16);
        }
        1
    }

    /// `MsSetChrScriptTargets_00796430(i, bits)` (`0x00796430`): an actor's
    /// script targets (`+0xdc0`) = `bits`; how many of bits 0-30 are set
    /// (0 for a non-actor).
    pub fn ms_set_chr_script_targets(mem: &mut impl Mem, i: u32, bits: u32) -> u32 {
        let a = actor_only(mem, i, |a| a);
        if a == 0 {
            return 0;
        }
        mem.write(a.wrapping_add(0xdc0), bits);
        (bits & 0x7fff_ffff).count_ones()
    }

    /// `MsGetMapCenter(out)` (`0x00782ae0`): with [`MAP_CENTER_OWN`], the
    /// battle's own center (four words of [`BATTLE_MEM`]) through the
    /// emulated VU register `0x00c8f588` into `out` (`EAX` its last word);
    /// else the map's (`yiGetBattleCenter(out)`, through `call`; the
    /// original jumps to it).
    pub fn ms_get_map_center<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        out: u32,
    ) -> u32 {
        if MAP_CENTER_OWN.get(mem) == 0 {
            return game(mem, call, YI_GET_BATTLE_CENTER, &[out]);
        }
        let src = BATTLE_MEM.ptr(mem).addr;
        let vf = crate::camera::bridge::EE_MOVE.ptr(mem).addr;
        let w = [0, 4, 8, 12].map(|o| read_u32(mem, src + o));
        for k in 1..4 {
            mem.write(vf + 4 * k, w[k as usize]);
        }
        mem.write(vf, w[0]);
        mem.write(out, w[0]);
        let mut last = 0;
        for k in 1..4u32 {
            last = read_u32(mem, vf + 4 * k);
            mem.write(out.wrapping_add(4 * k), last);
        }
        last
    }

    /// `MsAnyChrMotion_007a8be0(bits)` (`0x007a8be0`): 1 when one of the
    /// actors (below 31) in `bits` has a motion (`+0x432`), else 0.
    pub fn ms_any_chr_motion(mem: &impl Mem, bits: u32) -> u32 {
        u32::from((0..0x1f).any(|i| bits >> i & 1 != 0 && byte(mem, actor_of(mem, i), 0x432) != 0))
    }

    /// `MsEffectSetSpeed(i, speed)` (`0x00788430`): a negative speed turns
    /// actor `i & 0xff`'s effect speed off (`+0x404` 0); another becomes
    /// its `u16` `+0x402` with `+0x404` 1; then `MsCalcMotionSpeed`, whose
    /// result it returns.
    pub fn ms_effect_set_speed(mem: &mut impl Mem, i: u32, speed: u32) -> u32 {
        let a = actor_of(mem, i & 0xff);
        if speed.cast_signed() < 0 {
            mem.write(a.wrapping_add(0x404), 0u8);
        } else {
            #[allow(clippy::cast_possible_truncation)] // the original's `mov word`
            mem.write(a.wrapping_add(0x402), speed as u16);
            mem.write(a.wrapping_add(0x404), 1u8);
        }
        crate::motion::bridge::ms_calc_motion_speed(mem, a)
    }

    /// `MsShowBattleMessage_0079d090(msg)` (`0x0079d090`): [`BATTLE_MESSAGE`]
    /// = `msg`; with a message table and `MsGetBattleMessageFlag`, the
    /// message's text (the `u16` offset at `4 (kana + 2 msg)`, kana from
    /// `MsGetSaveConfigHiragana`) into the centre window
    /// (`TODrawAMES_CENTERMES(7, text)`, through `call`) while
    /// [`MESSAGE_WINDOWS_ON`]. 0.
    pub fn ms_show_battle_message<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        msg: u32,
    ) -> u32 {
        let table = BATTLE_MESSAGES.get(mem);
        BATTLE_MESSAGE.set(mem, msg);
        if table == 0 || ms_get_battle_message_flag(mem, call) == 0 {
            return 0;
        }
        let kana = crate::save::bridge::ms_get_save_config_hiragana(mem);
        let at = table.wrapping_add(kana.wrapping_add(msg.wrapping_mul(2)).wrapping_mul(4));
        let off = u32::from(mem.read::<u16>(at));
        if MESSAGE_WINDOWS_ON.get(mem) != 0 {
            game(
                mem,
                call,
                TO_DRAW_AMES_CENTERMES,
                &[7, table.wrapping_add(off)],
            );
        }
        0
    }

    /// `MsAbilityMapSetCommand_00798840(chr, id)` (`0x00798840`): a command
    /// ID (0x3xxx) sets its bit in the character's ability map
    /// (`MsGetAbilityMap_007987f0`, `u16`s from `+0x10`).
    pub fn ms_ability_map_set_command(mem: &mut impl Mem, chr: u32, id: u32) {
        if id & 0xffff_f000 != 0x3000 {
            return;
        }
        let map = crate::save_param::bridge::ms_get_ability_map(mem, chr);
        let k = id & 0xfff;
        let at = map.wrapping_add(0x10 + 2 * (k >> 4));
        let w: u16 = mem.read(at);
        mem.write(at, w | 1 << (k & 0xf));
    }

    /// `MsSetChrGravMode_00796070(i, mode)` (`0x00796070`): an actor's
    /// gravity mode byte (`+0x34`) = `mode`, its character's too
    /// (`Ch_SetGravMode`, sign-extended); the old byte (signed), 1 for a
    /// non-actor.
    pub fn ms_set_chr_grav_mode(mem: &mut impl Mem, i: u32, mode: u32) -> u32 {
        let a = actor_only(mem, i, |a| a);
        if a == 0 {
            return 1;
        }
        let chr: u32 = mem.read(a);
        let old = sbyte(mem, a, 0x34);
        let m = mode.to_le_bytes()[0];
        mem.write(a.wrapping_add(0x34), m);
        if chr != 0 {
            let v = i32::from(m.cast_signed()).cast_unsigned();
            ffx_chara::chr::bridge::ch_set_u8(mem, chr, ffx_chara::layout::GRAV_MODE, v);
        }
        old
    }

    /// `MsEffectCheckMotion2(i)` (`0x007878a0`): with actor `i & 0xff`'s
    /// character in a motion (`u16` `+0x400` above 0) it waits on
    /// (`Ch_GetWaitMotion`) and `+0x434` 0, the effect waits on it:
    /// [`EFFECT_WAIT`] = (`i`, its motion `+0x3f0`), 0; else -1.
    pub fn ms_effect_check_motion2(mem: &mut impl Mem, i: u32) -> u32 {
        let a = actor_of(mem, i & 0xff);
        let chr: u32 = mem.read(a);
        if chr == 0
            || mem.read::<u16>(a.wrapping_add(0x400)) == 0
            || ffx_chara::parts::bridge::ch_get_wait_motion(mem, chr) == 0
            || byte(mem, a, 0x434) != 0
        {
            return u32::MAX;
        }
        EFFECT_WAIT.set(mem, [i.to_le_bytes()[0], byte(mem, a, 0x3f0)]);
        0
    }

    /// `MsChangeWeaponInvisible(chr, off)` (`0x007ad5e0`): for a party
    /// character (`chr & 0xff` below 7), bit 1 of its weapon's and armor's
    /// flags (`+3` of their equipment slots, `MsGetSaveWeapon`) = bit 0 of
    /// `off`.
    pub fn ms_change_weapon_invisible(mem: &mut impl Mem, chr: u32, off: u32) {
        let chr = chr & 0xff;
        if chr >= 7 {
            return;
        }
        let ply = crate::save::bridge::PLY_SAVE.ptr(mem).addr + chr * 0x94;
        let bit = off.to_le_bytes()[0].wrapping_add(off.to_le_bytes()[0]);
        for slot in [0x2d, 0x2e] {
            let id = u32::from(byte(mem, ply, slot));
            if id == 0xff {
                continue;
            }
            let w = crate::save::bridge::ms_get_save_weapon(mem, id, 0);
            let f = byte(mem, w, 3);
            mem.write(w.wrapping_add(3), f ^ ((bit ^ f) & 2));
        }
    }

    /// `MsGetChrLiftHeight_00794990(i)` (`0x00794990`): an actor's height
    /// (`f32` `+0x534`, through a local) unless it stands on the ground
    /// (as `MsGetChrBtlPos` asks: `+0x608` 0 and alive (`+0xdcc` 0) or
    /// `+0xde7`, and `+0x50d` 0), then 0.0 (also for a non-actor); the
    /// `f32` in ST0, as bits.
    pub fn ms_get_chr_lift_height(mem: &impl Mem, i: u32) -> u32 {
        let a = actor_only(mem, i, |a| a);
        if a == 0 {
            return 0;
        }
        let air = byte(mem, a, 0x50d) != 0;
        let lifted = if byte(mem, a, 0x608) != 0 {
            air
        } else {
            byte(mem, a, 0xdcc) == 0 || byte(mem, a, 0xde7) != 0 || air
        };
        if lifted {
            crate::camera::x87_copy(mem.read(a.wrapping_add(0x534)))
        } else {
            0
        }
    }

    /// `MsRandCheck_007a8ad0(id, n)` (`0x007a8ad0`): 1 for a negative `n`;
    /// with `id` and a fixed check ([`RAND_FIXED`]), 1 when they are the
    /// same, else 0; otherwise a roll (`brnd(0x12) & 0xff`, printed with
    /// `dbgPrintf("debug:RAND %04x %d<%d\n", id, roll, n)` through `call`)
    /// below `n` (signed) gives 1, else 0.
    pub fn ms_rand_check<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        id: u32,
        n: u32,
    ) -> u32 {
        if n.cast_signed() < 0 {
            return 1;
        }
        let fixed = u32::from(RAND_FIXED.get(mem));
        if id != 0 && fixed != 0 {
            return u32::from(id == fixed);
        }
        let roll = crate::rng::bridge::brnd(mem, 0x12) & 0xff;
        let fmt = mem.rebase(RAND_FORMAT);
        game(mem, call, DBG_PRINTF, &[fmt, id, roll, n]);
        u32::from(roll.cast_signed() < n.cast_signed())
    }

    /// `MsBattleDraw()` (`0x00780d00`): `FFXIsDebug()` (through `call`),
    /// which it returns; when set the original jumps to `0x007c8ae0`, a
    /// bare `ret` (the debug draw left out of the build).
    pub fn ms_battle_draw<M: Mem>(mem: &mut M, call: crate::exec::bridge::Call<'_, M>) -> u32 {
        game(mem, call, FFX_IS_DEBUG, &[])
    }

    /// `MsEffectReadError()` (`0x007881b0`): the actor of the effect file
    /// being read (`g_BtlEffectReadChr`'s low byte).
    pub fn ms_effect_read_error(mem: &impl Mem) -> u32 {
        let i = crate::parse::bridge::EFFECT_READ_CHR.get(mem) & 0xff;
        actor_of(mem, i)
    }

    /// `MsSetAllVisible_00796680(v)` (`0x00796680`): the players' and then
    /// the monsters' visibility (`MsSetPlayersVisible_007968b0`,
    /// `MsSetMonsVisible_00796880`); 0x1a.
    pub fn ms_set_all_visible<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        v: u32,
    ) -> u32 {
        let n = ms_set_players_visible(mem, call, v);
        n.wrapping_add(ms_set_mons_visible(mem, call, v))
    }

    /// `MsChrUnableToAct_0079a080(i)` (`0x0079a080`): 1 when actor `i` has
    /// no HP (`i32` `+0x5d0` at most 0), is not in the battle (`+0xdd4` 0),
    /// is out (`+0xdcd`) or held (`+0x6d8`); else 0.
    pub fn ms_chr_unable_to_act(mem: &impl Mem, i: u32) -> u32 {
        let a = actor_of(mem, i);
        let hp: i32 = mem.read(a.wrapping_add(0x5d0));
        u32::from(
            hp <= 0
                || byte(mem, a, 0xdd4) == 0
                || byte(mem, a, 0xdcd) != 0
                || byte(mem, a, 0x6d8) != 0,
        )
    }

    /// `MsAtelSetHandlerByte_007987b0(kind, i, n, v)` (`0x007987b0`): with
    /// handler `n` of script owner `kind`/`i` (`MsAtelGetHandler`), the
    /// byte of its battle work (`AtelGetBattleIdAdrs(handler byte + base)`,
    /// through `call`) = `v`; 1, or 0 without a handler.
    pub fn ms_atel_set_handler_byte<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        kind: u32,
        owner: u32,
        n: u32,
        value: u32,
    ) -> u32 {
        let (handler, base) =
            crate::atel::bridge::atel_get_handler(mem, kind, owner, n.cast_signed());
        if handler == 0 {
            return 0;
        }
        let work = u32::from(byte(mem, handler, 0)).wrapping_add(base.unwrap_or(0));
        let at = game(mem, call, ATEL_GET_BATTLE_ID_ADRS, &[work]);
        mem.write(at, value.to_le_bytes()[0]);
        1
    }

    /// `MsChrGroundFlag_00794db0(i)` (`0x00794db0`): 1 for an actor with
    /// `+0x41e` or `+0xdce` set, else 0.
    pub fn ms_chr_ground_flag(mem: &impl Mem, i: u32) -> u32 {
        actor_only(mem, i, |a| {
            u32::from(byte(mem, a, 0x41e) != 0 || byte(mem, a, 0xdce) != 0)
        })
    }

    /// `MsPauseChrEffect(i, on)` (`0x0079ea40`): with an effect (`+0x4c`)
    /// and a character, its bound effects drawn (`on`) or not
    /// (`op_et_bindeff_draw_on`/`_off(chr)`, through `call`); 1, else 0.
    pub fn ms_pause_chr_effect<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        i: u32,
        on: u32,
    ) -> u32 {
        let a = actor_of(mem, i);
        let chr: u32 = mem.read(a);
        if read_u32(mem, a.wrapping_add(0x4c)) == 0 || chr == 0 {
            return 0;
        }
        let f = if on != 0 {
            OP_ET_BINDEFF_DRAW_ON
        } else {
            OP_ET_BINDEFF_DRAW_OFF
        };
        game(mem, call, f, &[chr]);
        1
    }

    /// `MsEventEffectReadComplete()` (`0x00788d10`): when the magic file
    /// loads asynchronously (`graphicIsMagicFileAsyncLoadAvailable`), the
    /// event effect's state 1 and table [`EVENT_EFFECT_TABLE0`], then
    /// `op_ovl_effect_init()`; always `MagicFile_Start()` and the table
    /// from `MagicFile_GetEffectOverlayTable(0)` (state 1 becomes 3 with
    /// one). All through `call`; returns the table.
    pub fn ms_event_effect_read_complete<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
    ) -> u32 {
        use crate::state::bridge::EVENT_EFFECT_STAT;
        if game(mem, call, GRAPHIC_MAGIC_ASYNC, &[]) != 0 {
            EVENT_EFFECT_STAT.set(mem, 1);
            let t = mem.rebase(EVENT_EFFECT_TABLE0);
            EVENT_EFFECT_TABLE.set(mem, t);
            game(mem, call, OP_OVL_EFFECT_INIT, &[]);
        }
        game(mem, call, MAGIC_FILE_START, &[]);
        let t = game(mem, call, MAGIC_FILE_OVERLAY_TABLE, &[0]);
        EVENT_EFFECT_TABLE.set(mem, t);
        if t != 0 && EVENT_EFFECT_STAT.get(mem) == 1 {
            EVENT_EFFECT_STAT.set(mem, 3);
        }
        t
    }

    /// `MsChrInExtra_00796d10(i)` (`0x00796d10`): 1 when an actor's
    /// character is one of the three [`CHR_EXTRA`] records in use, else 0.
    pub fn ms_chr_in_extra(mem: &impl Mem, i: u32) -> u32 {
        let a = actor_only(mem, i, |a| a);
        if a == 0 {
            return 0;
        }
        let chr = read_u32(mem, a);
        let extra = CHR_EXTRA.get(mem);
        u32::from((0..3).any(|k| {
            let e = extra.wrapping_add(12 * k);
            byte(mem, e, 0) != 0 && read_u32(mem, e.wrapping_add(4)) == chr
        }))
    }

    /// `MsAddNextCtb_0079a020(i, rank, ctb, haste, slow)` (`0x0079a020`):
    /// actor `i`'s next CTB (`MsGetNextCTB(actor, rank, haste[i],
    /// slow[i])`) added to `ctb[i]` (`u32`s), then each of the two
    /// counters (`u8`s) above 0 and below 0xfe one less.
    pub fn ms_add_next_ctb(mem: &mut impl Mem, i: u32, rank: u32, ctb: u32, haste: u32, slow: u32) {
        let actor = actor_of(mem, i);
        let (haste_at, slow_at) = (haste.wrapping_add(i), slow.wrapping_add(i));
        let ctb_at = ctb.wrapping_add(i.wrapping_mul(4));
        let next = crate::damage::bridge::ms_get_next_ctb(
            mem,
            actor,
            rank.cast_signed(),
            i32::from(byte(mem, haste_at, 0)),
            i32::from(byte(mem, slow_at, 0)),
        );
        let sum = read_u32(mem, ctb_at).wrapping_add(next.cast_unsigned());
        mem.write(ctb_at, sum);
        for counter in [haste_at, slow_at] {
            let left = byte(mem, counter, 0);
            if left != 0 && left < 0xfe {
                mem.write(counter, left - 1);
            }
        }
    }

    /// `MsResetAvoidFlag(i, selector)` (`0x0078d620`): for an actor, the
    /// avoid flag (`+0x425`) of every actor `selector` picks
    /// (`MsGetChrBit(i, selector, 1, 1)`, below 31) cleared; how many.
    pub fn ms_reset_avoid_flag(mem: &mut impl Mem, i: u32, selector: u32) -> u32 {
        if crate::actor::bridge::ms_btl_chr_num_check(i) == 0 {
            return 0;
        }
        let bits = crate::actor::bridge::ms_get_chr_bit(mem, i, selector, 1, 1);
        let mut n = 0;
        for j in 0..0x1f {
            if bits >> j & 1 != 0 {
                n += 1;
                let a = actor_of(mem, j);
                mem.write(a.wrapping_add(0x425), 0u8);
            }
        }
        n
    }

    /// `MsGetDamageLimit(i, j)` (`0x0079a860`): for actor `i & 0xff`, the
    /// limit byte (`+0x13`) of the first of actor `j`'s two 0x2d8-byte
    /// records from `+0x776` naming `i` and its command's byte `+2`
    /// (`MsGetChrCommand`); 0 for none or a non-actor.
    pub fn ms_get_damage_limit(mem: &impl Mem, i: u32, j: u32) -> u32 {
        let i = i & 0xff;
        if crate::actor::bridge::ms_btl_chr_num_check(i) == 0 {
            return 0;
        }
        let cmd = u32::from(byte(
            mem,
            crate::actor::bridge::ms_get_chr_command(mem, i),
            2,
        ));
        let b = actor_of(mem, j);
        for k in 0..2 {
            let e = b.wrapping_add(0x776 + 0x2d8 * k);
            if u32::from(byte(mem, e, 0)) == i && u32::from(byte(mem, e, 1)) == cmd {
                return u32::from(byte(mem, e, 0x13));
            }
        }
        0
    }

    /// `MsSetChrByte446_007b75a0(i, v)` (`0x007b75a0`): actor `i & 0xff`'s
    /// `+0x446` = the low byte of `v`.
    pub fn ms_set_chr_byte446(mem: &mut impl Mem, i: u32, v: u32) {
        let a = actor_of(mem, i & 0xff);
        mem.write(a.wrapping_add(0x446), v.to_le_bytes()[0]);
    }

    /// `MsSetChrSpeedBytes_00795d40(i, v)` (`0x00795d40`): an actor's
    /// `+0x4fc` and `+0x509` = the low byte of `v`; -1, 0 for a non-actor.
    pub fn ms_set_chr_speed_bytes(mem: &mut impl Mem, i: u32, v: u32) -> u32 {
        let a = actor_only(mem, i, |a| a);
        if a == 0 {
            return 0;
        }
        let v = v.to_le_bytes()[0];
        mem.write(a.wrapping_add(0x4fc), v);
        mem.write(a.wrapping_add(0x509), v);
        u32::MAX
    }

    /// `MsSetChrFloat53c_00795ff0(i, flag, f)` (`0x00795ff0`): an actor's
    /// `f32` `+0x53c` = `f` (through `fld`/`fstp`) and `+0x510` = bit 0 of
    /// `flag`; -1, 0 for a non-actor.
    pub fn ms_set_chr_float53c(mem: &mut impl Mem, i: u32, flag: u32, f: u32) -> u32 {
        let a = actor_only(mem, i, |a| a);
        if a == 0 {
            return 0;
        }
        mem.write(a.wrapping_add(0x53c), crate::camera::x87_copy(f));
        mem.write(a.wrapping_add(0x510), flag.to_le_bytes()[0] & 1);
        u32::MAX
    }

    /// `MsSetChrRot_00795dd0(i, f)` (`0x00795dd0`): an actor's character
    /// turned to `f` (`Ch_SetRot`); -1, 0 without one.
    pub fn ms_set_chr_rot(mem: &mut impl Mem, i: u32, f: u32) -> u32 {
        let a = actor_only(mem, i, |a| a);
        if a == 0 {
            return 0;
        }
        let chr = read_u32(mem, a);
        if chr == 0 {
            return 0;
        }
        let f = f32::from_bits(crate::camera::x87_copy(f));
        ffx_chara::chr::bridge::ch_set_rot(mem, chr, f);
        u32::MAX
    }

    /// `PasswordTextDispose()` (`0x007902f0`): a loaded password text
    /// ([`PASSWORD_TEXT_ON`], the file through `get_ptr32_func`: itself) is
    /// freed (`0x00687010(file)`, `is_ptr32_null(handle)`, through `call`)
    /// and marked unloaded.
    pub fn password_text_dispose<M: Mem>(mem: &mut M, call: crate::exec::bridge::Call<'_, M>) {
        if PASSWORD_TEXT_ON.get(mem) == 0 {
            return;
        }
        let file = PASSWORD_TEXT.get(mem);
        if file == 0 {
            return;
        }
        game(mem, call, FREE_00687010, &[file]);
        game(mem, call, IS_PTR32_NULL, &[file]);
        PASSWORD_TEXT_ON.set(mem, 0);
    }

    /// `MsGetChrReelKind_007b19f0(i)` (`0x007b19f0`): for an actor whose
    /// `u16` `+0xf5c` is 0x3074-0x3077, 0-3 by it; else 0.
    pub fn ms_get_chr_reel_kind(mem: &impl Mem, i: u32) -> u32 {
        let i = i & 0xff;
        if crate::actor::bridge::ms_btl_chr_num_check(i) == 0 {
            return 0;
        }
        let v =
            u32::from(mem.read::<u16>(actor_of(mem, i).wrapping_add(0xf5c))).wrapping_sub(0x3074);
        if v <= 3 { v } else { 0 }
    }

    /// `MsAddEquipment_007ab910(rec)` (`0x007ab910`): the 22-byte record into
    /// the first free equipment slot (`+2` 0), then in use (`+2` 1) and
    /// unworn (`+6` 0xff); 0x5000 plus its index, 0 when all 200 are used.
    pub fn ms_add_equipment(mem: &mut impl Mem, rec: u32) -> u32 {
        let base = EQUIPMENT.ptr(mem).addr;
        let Some(k) = (0..200).find(|k| byte(mem, base + 22 * k, 2) == 0) else {
            return 0;
        };
        let slot = base + 22 * k;
        let bytes: [u8; 22] = mem.read(rec);
        mem.write(slot, bytes);
        mem.write(slot + 2, 1u8);
        mem.write(slot + 6, 0xffu8);
        0x5000 + k
    }

    // Round 29 G: the command window, the party, encounters, shadows,
    // transparency, equipment lists.

    /// Set: `MsCheckLeftWindow` gives the actor its full `+0x5bc` back
    /// (from `+0x5bd`) first.
    pub const LEFT_WINDOW_RESET: Global<u32> = Global::new(0x0113_33e8);
    /// `MsGetSaveMenuPanel()`'s panel: the commands' 0x3000-0x305f bits,
    /// then the shared ones' from `+0xc`.
    pub const MENU_PANEL: Global<u16> = Global::new(0x0113_07b8);
    /// `MsGetSavePlyLeft(chr)` (an auto-port), called through the game.
    pub const MS_GET_SAVE_PLY_LEFT: u32 = 0x0078_53c0;

    /// `MsCheckLeftWindow(i)` (`0x0078f700`): what actor `i`'s eight menu
    /// commands (`g_BtlItemMem2` `+0x28` + 0x478 `i`, `u16`s, 0xff none)
    /// allow when usable (`MsGetCommandUse` 0): bit 0 for one without a
    /// cost (`MsGetComData` `+0x26` 0), bit 1 for one whose cost the actor
    /// can pay (`+0x5bc` at least `+0x5bd`, or `g_BtlDebugNoCost`). With
    /// [`LEFT_WINDOW_RESET`] `+0x5bc` is `+0x5bd` first.
    pub fn ms_check_left_window(mem: &mut impl Mem, i: u32) -> u32 {
        let a = actor_of(mem, i);
        if LEFT_WINDOW_RESET.get(mem) != 0 {
            let v = byte(mem, a, 0x5bd);
            mem.write(a.wrapping_add(0x5bc), v);
        }
        let mut bits = 0;
        for k in 0..8 {
            let list = crate::misc::bridge::ITEM_MEM2.get(mem);
            let at = list
                .wrapping_add(i.wrapping_mul(0x478))
                .wrapping_add(0x28 + 2 * k);
            let id = u32::from(mem.read::<u16>(at));
            if id == 0xff {
                continue;
            }
            let rec = crate::stat::bridge::ms_get_com_data(mem, id, 0);
            if crate::turn::bridge::ms_get_command_use(mem, i, id) != 0 {
                continue;
            }
            if byte(mem, rec, 0x26) == 0 {
                bits |= 1;
            } else if byte(mem, a, 0x5bc) >= byte(mem, a, 0x5bd)
                || crate::damage::bridge::DEBUG_NO_COST.get(mem) != 0
            {
                bits |= 2;
            }
        }
        bits
    }

    /// `MsSetSavePlyJoin(chr, on)` (`0x007869b0`): character `chr & 0xff`
    /// joins (`on`: weapon shown, `MsChangeWeaponInvisible(chr, 0)`; its
    /// flags' bits 0 and 4 = bit 0 of `on`; one of the first 8 not in the
    /// party (20 slots) takes the first free one of the others,
    /// `MsGetSaveOutParty(&n)` with `n` in `local`, the original's
    /// argument slot) or leaves (unless `MsGetSavePlyLeft`, through `call`:
    /// bit 0 cleared, its party slot 0xff). Then `MsSaveEncountCheck`,
    /// whose result it returns.
    pub fn ms_set_save_ply_join<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        chr: u32,
        on: u32,
        local: u32,
    ) -> u32 {
        use crate::save::bridge::{PLY_SAVE, ms_get_save_in_party, ms_get_save_out_party};
        let chr = chr & 0xff;
        let flags = PLY_SAVE.ptr(mem).addr + chr * 0x94 + 0x2c;
        if on != 0 {
            ms_change_weapon_invisible(mem, chr, 0);
            let bit = on.to_le_bytes()[0] & 1;
            let f = (byte(mem, flags, 0) & 0xfe | bit) & 0xef | bit << 4;
            mem.write(flags, f);
            if chr < 8 {
                let party = ms_get_save_in_party(mem, 0);
                if !(0..0x14).any(|k| u32::from(byte(mem, party, k)) == chr) {
                    let out = ms_get_save_out_party(mem, local);
                    let n: i32 = mem.read(local);
                    if let Some(k) =
                        (0..n.max(0).cast_unsigned()).find(|&k| byte(mem, out, k) == 0xff)
                    {
                        mem.write(out + k, chr.to_le_bytes()[0]);
                    }
                }
            }
        } else if game(mem, call, MS_GET_SAVE_PLY_LEFT, &[chr]) == 0 {
            let f = byte(mem, flags, 0) & 0xfe;
            mem.write(flags, f);
            let party = ms_get_save_in_party(mem, 0);
            if let Some(k) = (0..0x14).find(|&k| u32::from(byte(mem, party, k)) == chr) {
                mem.write(party + k, 0xffu8);
            }
        }
        ms_save_encount_check(mem)
    }

    /// `Enc_ResetWeights(map, keep)` (`0x007828a0`): each encounter group of
    /// map `map`'s field (`MsBtlListFieldNum`) gets its weight total (`+4`)
    /// from its formations' weights (the high nibble of their second byte,
    /// which, unless `keep`, is first reset to the low one).
    pub fn enc_reset_weights(mem: &mut impl Mem, map: u32, keep: u32) {
        let f = ms_btl_list_field_num(mem, map.cast_signed());
        if f < 0 {
            return;
        }
        let list = ms_btl_list_encount(mem, f);
        let mut g = 0;
        while g < u32::from(byte(mem, list, 1)) {
            let grp = crate::flow::bridge::ms_btl_list_group(mem, f, g.cast_signed());
            let mut sum = 0u8;
            let mut k = 0;
            while k < u32::from(byte(mem, grp, 0)) {
                let at = grp + 6 + 2 * k;
                if keep == 0 {
                    let low = byte(mem, at, 0) & 0xf;
                    mem.write(at, low << 4 | low);
                }
                sum = sum.wrapping_add(byte(mem, at, 0) >> 4);
                k += 1;
            }
            mem.write(grp + 4, sum);
            g += 1;
        }
    }

    /// `MsGetSaveCommandMenuPanel(id)` (`0x007850b0`): whether command `id`
    /// (0x3xxx) is on the menu panel ([`MENU_PANEL`]): below 0x3060 its bit
    /// from `+0`, else (`id - 0x60`) from `+0xc`, 0 past ten words.
    pub fn ms_get_save_command_menu_panel(mem: &impl Mem, id: u32) -> u32 {
        if id & 0xffff_f000 != 0x3000 {
            return 0;
        }
        let panel = MENU_PANEL.ptr(mem).addr;
        let (at, k) = if id.cast_signed() < 0x3060 {
            let k = id & 0xfff;
            (panel + 2 * (k >> 4), k)
        } else {
            let k = id.wrapping_sub(0x60) & 0xfff;
            if k >> 4 >= 10 {
                return 0;
            }
            (panel + 0xc + 2 * (k >> 4), k)
        };
        u32::from(mem.read::<u16>(at) & 1 << (k & 0xf) != 0)
    }

    /// `MsSetChrShadowMode(i, k, v)` (`0x007962d0`): shadow request `k`
    /// (`msSetChrShadowReq`) of actor `i & 0xff`, its two weapons, and its
    /// followers (`+0x440`: the actors whose `+0x43f` is `i`, recursively).
    /// 0.
    #[allow(clippy::many_single_char_names)]
    pub fn ms_set_chr_shadow_mode(mem: &mut impl Mem, i: u32, k: u32, v: u32) -> u32 {
        use crate::visible::bridge::ms_set_chr_shadow_req;
        let i = i & 0xff;
        let a = actor_of(mem, i);
        let v8 = v.to_le_bytes()[0];
        ms_set_chr_shadow_req(mem, a, k.cast_signed(), v8);
        for n in 0..2 {
            let w = crate::actor::bridge::ms_get_weapon(mem, i, n);
            if w != 0 {
                ms_set_chr_shadow_req(mem, w, k.cast_signed(), v8);
            }
        }
        if byte(mem, a, 0x440) != 0 {
            for j in 0..0x1f {
                if u32::from(byte(mem, actor_of(mem, j), 0x43f)) == i {
                    ms_set_chr_shadow_mode(mem, j, k, v);
                }
            }
        }
        0
    }

    /// `MsSetChrShadow(i, v)` (`0x007962b0`): shadow request 2
    /// ([`ms_set_chr_shadow_mode`]).
    pub fn ms_set_chr_shadow(mem: &mut impl Mem, i: u32, v: u32) -> u32 {
        ms_set_chr_shadow_mode(mem, i, 2, v)
    }

    /// `MsSetChrTransparentVisible(i, from, to, frames, flags)`
    /// (`0x00796550`): with a character and flag bit 0, actor `i & 0xff`
    /// fades from `from` to `to` over `frames` (`Ch_SetTransparent` twice,
    /// the first at once; `to` its `f32` `+0x40`, `+0x423` 0, then
    /// `MsSetChrVisibleAppear(i, 1)`), its weapons alike, and its followers
    /// (recursively); 1, or 0 without.
    pub fn ms_set_chr_transparent_visible<M: Mem>(
        mem: &mut M,
        call: crate::exec::bridge::Call<'_, M>,
        i: u32,
        from: u32,
        to: u32,
        frames: u32,
        flags: u32,
    ) -> u32 {
        use ffx_chara::{chr::bridge::ch_set_fade, layout::FADE_ALPHA};
        let i = i & 0xff;
        let a = actor_of(mem, i);
        let chr = read_u32(mem, a);
        if chr == 0 || flags & 1 == 0 {
            return 0;
        }
        let (f0, f1) = (
            f32::from_bits(crate::camera::x87_copy(from)),
            f32::from_bits(crate::camera::x87_copy(to)),
        );
        let frames = frames.cast_signed();
        mem.write(a.wrapping_add(0x40), crate::camera::x87_copy(to));
        mem.write(a.wrapping_add(0x423), 0u8);
        crate::visible::bridge::ms_set_chr_visible_appear(mem, call, i, 1);
        ch_set_fade(mem, chr, FADE_ALPHA, f0, 0);
        ch_set_fade(mem, chr, FADE_ALPHA, f1, frames);
        for n in 0..2 {
            let w = crate::actor::bridge::ms_get_weapon(mem, i, n);
            if w == 0 {
                continue;
            }
            let wc = read_u32(mem, w);
            mem.write(w.wrapping_add(0x40), crate::camera::x87_copy(to));
            if wc != 0 {
                ch_set_fade(mem, wc, FADE_ALPHA, f0, 0);
                ch_set_fade(mem, wc, FADE_ALPHA, f1, frames);
            }
        }
        if byte(mem, a, 0x440) != 0 {
            for j in 0..0x1f {
                if u32::from(byte(mem, actor_of(mem, j), 0x43f)) == i {
                    ms_set_chr_transparent_visible(
                        mem,
                        call,
                        j,
                        from,
                        to,
                        frames.cast_unsigned(),
                        flags,
                    );
                }
            }
        }
        1
    }

    /// `Equip_RebuildWearers(id)` (`0x007abca0`): equipment `id` in use
    /// (`MsGetSaveWeapon` `+2`) no longer is, and no character wears it
    /// any more (their weapon and armor slots `+0x2d`/`+0x2e` with it become
    /// 0xff; not for the 0x7000 IDs); 0, -1 for 0xff or one not in use.
    pub fn equip_rebuild_wearers(mem: &mut impl Mem, id: u32) -> u32 {
        if id == 0xff {
            return u32::MAX;
        }
        let slot = crate::save::bridge::ms_get_save_weapon(mem, id, 0);
        if byte(mem, slot, 2) == 0 {
            return u32::MAX;
        }
        mem.write(slot.wrapping_add(2), 0u8);
        if id & 0xffff_f000 != 0x7000 {
            let k = id & 0xfff;
            let ply = crate::save::bridge::PLY_SAVE.ptr(mem).addr;
            for c in 0..18 {
                for o in [0x2d, 0x2e] {
                    let at = ply + c * 0x94 + o;
                    if u32::from(byte(mem, at, 0)) == k {
                        mem.write(at, 0xffu8);
                    }
                }
            }
        }
        0
    }

    /// `MsCueAddCommand_007ac940(i, id, word)` (`0x007ac940`): while an
    /// action runs (`crate::misc::bridge::ACTION` not -1), and unless the
    /// actor's linked actor (`+0xded`) is in state 3 (`+0xdf0`), the
    /// command `id` (with 0xff as its second) and `word` added to the
    /// linked actor's cue (`MsGetChrCommand2` of its `+0xc`; its count
    /// `+2`). 0.
    pub fn ms_cue_add_command(mem: &mut impl Mem, i: u32, id: u32, word: u32) -> u32 {
        if crate::misc::bridge::ACTION.get(mem) == -1 {
            return 0;
        }
        let link = sbyte(mem, actor_of(mem, i), 0xded);
        let b = actor_of(mem, link);
        if byte(mem, b, 0xdf0) == 3 {
            return 0;
        }
        let cue = crate::actor::bridge::ms_get_chr_command2(mem, u32::from(byte(mem, b, 0xc)));
        if cue == 0 {
            return 0;
        }
        let n = u32::from(byte(mem, cue, 2));
        #[allow(clippy::cast_possible_truncation)] // the original's `mov word`
        mem.write(cue + 8 + 16 * n, id as u16);
        mem.write(cue + 0xa + 16 * n, 0xffu16);
        let n = u32::from(byte(mem, cue, 2));
        mem.write(cue + 16 * (n + 1), word);
        0
    }

    /// `MsCopyPointsOffset_007abeb0(base, other, k)` (`0x007abeb0`): record
    /// `k` (32 bytes from `base`; `k` within its signed count `+1`) has
    /// points (`+0x10` an offset from `base`, count `+4`); each of `other`'s
    /// records (signed count `+1`, 0x60 bytes from `+4`: count `+0`, the
    /// points' offset from `other` at `+0xc`, an offset vector at `+0x2c`)
    /// gets as many of them as both have, each moved by its vector (`f32`
    /// adds, 16 bytes apart).
    #[allow(clippy::many_single_char_names)]
    pub fn ms_copy_points_offset(mem: &mut impl Mem, base: u32, other: u32, k: u32) {
        use ffx_bridge::x87::{add, load, store};
        let k = k.cast_signed();
        if k < 0 || k >= sbyte(mem, base, 1).cast_signed() {
            return;
        }
        let rec = base + 32 * k.cast_unsigned();
        let n = sbyte(mem, other, 1).cast_signed();
        for j in 0..n.max(0).cast_unsigned() {
            let e = other + 4 + 0x60 * j;
            let mut src = base.wrapping_add(read_u32(mem, rec + 0x10));
            let mut dst = other.wrapping_add(read_u32(mem, e + 0xc));
            let count = sbyte(mem, rec, 4)
                .cast_signed()
                .min(sbyte(mem, e, 0).cast_signed());
            for _ in 0..count.max(0) {
                for c in 0..3 {
                    let o: u32 = mem.read(e + 0x2c + 4 * c);
                    let p: u32 = mem.read(src + 4 * c);
                    mem.write(dst + 4 * c, store(add(load(o), load(p))));
                }
                src += 0x10;
                dst += 0x10;
            }
        }
    }

    /// `MsBuildGearList_0079ca80(i)` (`0x0079ca80`): character `i`'s
    /// weapons, then its armors (`g_BtlItemMem2` `+0x158` and `+0x2e8` +
    /// 0x478 `i`): 0x5000 + slot for each equipment slot in use (`+2`) of
    /// that kind (`+5`) owned by `i` (`+4`) without flag 2 (`+3`).
    pub fn ms_build_gear_list(mem: &mut impl Mem, i: u32) {
        let eq = EQUIPMENT.ptr(mem).addr;
        for kind in 0..2u32 {
            let list = crate::misc::bridge::ITEM_MEM2.get(mem);
            let mut out = list
                .wrapping_add(if kind == 0 { 0x158 } else { 0x2e8 })
                .wrapping_add(i.wrapping_mul(0x478));
            for k in 0..200 {
                let s = eq + 22 * k;
                if byte(mem, s, 2) != 0
                    && u32::from(byte(mem, s, 5)) == kind
                    && u32::from(byte(mem, s, 4)) == i
                    && byte(mem, s, 3) & 2 == 0
                {
                    #[allow(clippy::cast_possible_truncation)] // below 0x50c8
                    mem.write(out, (0x5000 + k) as u16);
                    out = out.wrapping_add(2);
                }
            }
        }
    }
}
