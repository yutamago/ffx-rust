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
    /// The weapon equip flags (`MsInitSaveWeaponEquipFlag`): byte `+0` of
    /// 200 records of 0x16 bytes, the character who wears it.
    pub const WEAPON_EQUIP: Global<u8> = Global::new(0x0113_0f32);
    /// The two equipped weapon slots of 18 characters, 0x94 bytes apart.
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

    /// `MsInitSaveWeaponEquipFlag` (`0x007abe30`): every weapon's wearer to
    /// 0xff, then each of the 18 characters' two equipped weapons to that
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
}
