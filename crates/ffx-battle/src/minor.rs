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
}
