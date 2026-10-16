//! The battle's minor routines (`ffx_battle::minor`), the camera's
//! override, screens and slots (`ffx_battle::camera`) and
//! `MsVectorGetSide` against the original, as in `battle.rs`: random
//! globals, tables and arguments on the image alone, then the real state of
//! both snapshots.

// a and b are the two oracles throughout; the checks are long lists
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]

mod battle_common;

use battle_common::world::quiet;
use battle_common::*;
use ffx_battle::camera::bridge as cam;
use ffx_battle::minor::bridge as g;
use ffx_oracle::{Conv, Oracle};

/// The globals the functions here read, filled at random.
fn randomize(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    for (gl, len) in [
        (g::DIR_FREE.ptr(b).addr, 1),
        (g::ENCOUNT_TYPE.ptr(b).addr, 0x14),
        (g::EFFECT_CAMERA_CALL.ptr(b).addr, 4),
        (g::EFFECT_CAMERA_MODE.ptr(b).addr, 1),
        (g::ZANMATO_SEP.ptr(b).addr, 0x28),
        (g::ATEL_MANAGERS.ptr(b).addr, 0x54 * 8),
        (g::SEP_TRANS_TABLE.ptr(b).addr, 0x200),
        (g::SEP_TRANS_STATE.ptr(b).addr, 12),
        (g::SEP_TRANS_FLAG.ptr(b).addr, 2),
        (g::ATEL_CAM_ON.ptr(b).addr, 0xc),
        (g::ATEL_CAM2.ptr(b).addr, 0x14),
        (g::EFFECT_CAMERA_BUF.ptr(b).addr, 0x20),
        (g::WEAPON_EQUIP.ptr(b).addr, 0x16 * 0x100),
        (g::EQUIPPED.ptr(b).addr, 0x94 * 18),
        (g::STONES.ptr(b).addr, 4 * 31),
        (g::STONE_EXCLUDED.ptr(b).addr, 4),
        (g::SAVE_COMMAND_USE.ptr(b).addr, 0x2c * 18),
        (g::ATEL_CAMERA.ptr(b).addr, 0x30),
        (ffx_battle::state::bridge::STONE_EFFECT.ptr(b).addr, 2),
        (cam::SLOTS_AT.ptr(b).addr, cam::CAMERA_SIZE * 3),
        (cam::SCREENS.ptr(b).addr, cam::SCREEN_SIZE * 3),
        (cam::OVERRIDE_ROLL.ptr(b).addr, 0x34),
        (cam::OVERRIDE_DEPTH.ptr(b).addr, 8),
        (cam::EE_MOVE.ptr(b).addr, 16),
    ] {
        fill(a, b, rng, gl, len);
    }
    // the stone count within the list, at most 31
    let n = rng.range(-2, 33);
    poke(
        a,
        b,
        ffx_battle::state::bridge::STONE_EFFECT.ptr(b).addr,
        &[n.to_le_bytes()[0]],
    );
    // a manager's count: often below 8
    for m in 0..8 {
        let at = g::ATEL_MANAGERS.ptr(b).addr + 0x54 * m;
        poke(a, b, at, &rng.range(-1, 10).to_le_bytes());
    }
    set(a, b, cam::SELECTED, u8::try_from(rng.below(3)).unwrap());
}

/// Random links in `area`: a list of `n` nodes of `size` bytes whose next
/// pointer is at `next`, the first stored at `head`.
fn list(
    a: &mut Oracle,
    b: &mut Oracle,
    rng: &mut Xorshift,
    area: u32,
    size: u32,
    next: u32,
    head: u32,
) {
    let n = rng.below(8);
    fill(a, b, rng, area, size * 8);
    let mut prev = head;
    for i in 0..n {
        let node = area + size * i;
        poke(a, b, prev, &node.to_le_bytes());
        prev = node + next;
    }
    poke(a, b, prev, &0u32.to_le_bytes());
}

/// A motion table at `at` (`MsMotBinFrame`): sorted motions, each a list
/// of records with "on" and "off" frames.
fn mot_table(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, at: u32) {
    let motions = rng.below(6);
    let mut t = vec![0u8; 0x800];
    t[..2].copy_from_slice(&u16::try_from(motions).unwrap().to_le_bytes());
    let mut free = 2 + 4 * motions as usize;
    let mut key = 0u16;
    for m in 0..motions as usize {
        key += u16::try_from(1 + rng.below(3)).unwrap();
        t[2 + 4 * m..4 + 4 * m].copy_from_slice(&key.to_le_bytes());
        t[4 + 4 * m..6 + 4 * m].copy_from_slice(&u16::try_from(free).unwrap().to_le_bytes());
        let list = free;
        let recs = rng.below(4) as usize;
        t[list..list + 2].copy_from_slice(&u16::try_from(recs).unwrap().to_le_bytes());
        free = list + 2 + 2 * recs;
        for r in 0..recs {
            let off = free - list;
            t[list + 2 + 2 * r..list + 4 + 2 * r]
                .copy_from_slice(&u16::try_from(off).unwrap().to_le_bytes());
            let (on, offs) = (rng.below(4) as usize, rng.below(3) as usize);
            t[free] = rng.byte();
            t[free + 1] = u8::try_from(on).unwrap();
            t[free + 2] = u8::try_from(offs).unwrap();
            free += 4;
            for _ in 0..on + offs {
                let f: u16 = if rng.below(8) == 0 {
                    0xffff
                } else {
                    u16::try_from(rng.below(6)).unwrap()
                };
                t[free..free + 2].copy_from_slice(&f.to_le_bytes());
                free += 2;
            }
        }
    }
    poke(a, b, at, &t);
}

fn floats(rng: &mut Xorshift) -> u32 {
    match rng.below(8) {
        0 => [0, 0x8000_0000, 0x7f80_0000, 0xff80_0000, 0x7fc0_0000, 1][rng.below(6) as usize],
        1 => quiet(rng.next()),
        _ => (f32::from(i16::try_from(rng.range(-30000, 30000)).unwrap()) / 97.0).to_bits(),
    }
}

/// The calls on whatever state the oracles hold; `actor` is an actor (or
/// actor-sized scratch).
fn calls(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift, actor: u32, area: u32) {
    twin("MsSetDirThetaMode", a, b, 0x0079_6940, &[actor], ALL, |m| {
        g::ms_set_dir_theta_mode(m, actor)
    });
    let i = rng.range(-2, 40);
    twin(
        "MsBtlListField",
        a,
        b,
        0x0079_d190,
        &[i.cast_unsigned()],
        ALL,
        |m| g::ms_btl_list_field(m, i),
    );
    let map = rng.range(-2, 400);
    twin(
        "MsBtlListFieldNum",
        a,
        b,
        0x0079_d1c0,
        &[map.cast_unsigned()],
        ALL,
        |m| g::ms_btl_list_field_num(m, map).cast_unsigned(),
    );
    twin(
        "MsBtlListEncount",
        a,
        b,
        0x0079_d170,
        &[i.cast_unsigned()],
        ALL,
        |m| g::ms_btl_list_encount(m, i),
    );
    if rng.below(4) != 0 {
        let gr = rng.range(-1, 10);
        twin(
            "MsBtlListGroup",
            a,
            b,
            0x0079_d210,
            &[i.cast_unsigned(), gr.cast_unsigned()],
            ALL,
            |m| ffx_battle::flow::bridge::ms_btl_list_group(m, i, gr),
        );
    }
    let all = rng.below(2);
    twin("ResetEncountExe", a, b, 0x0078_0ff0, &[all], 0, |m| {
        g::reset_encount_exe(m, all);
        0
    });
    let (c, mode) = (rng.word(), rng.next());
    twin(
        "MsEffectCameraCall",
        a,
        b,
        0x0078_7740,
        &[c, mode],
        ALL,
        |m| g::ms_effect_camera_call(m, c.cast_signed(), mode),
    );
    let i = rng.range(-1, 0x40);
    twin(
        "MsSetMotBinAddress",
        a,
        b,
        0x0079_cd80,
        &[actor, i.cast_unsigned()],
        ALL,
        |m| g::ms_set_mot_bin_address(m, actor, i),
    );
    let (mgr, name, script) = (rng.range(0, 8), rng.next(), rng.next());
    twin(
        "MsAtelRegist",
        a,
        b,
        0x0079_7a40,
        &[mgr.cast_unsigned(), name, script],
        ALL,
        |m| g::ms_atel_regist(m, mgr, script).cast_unsigned(),
    );
    let (i, slot) = (rng.range(-1, 6), rng.below(6));
    poke(a, b, area + 0x700, &slot.to_le_bytes());
    twin(
        "Mon_FindHandler",
        a,
        b,
        0x0079_7410,
        &[area + 0x600, i.cast_unsigned(), area + 0x700],
        ALL,
        |m| g::mon_find_handler(m, area + 0x600, i, area + 0x700),
    );
    let (motion, frame) = (
        rng.below(12),
        if rng.below(8) == 0 {
            rng.next()
        } else {
            rng.below(6)
        },
    );
    let x = rng.next();
    twin(
        "MsMotBinFrame",
        a,
        b,
        0x0079_cb40,
        &[x, actor, motion, frame],
        ALL,
        |m| g::ms_mot_bin_frame(m, actor, motion, frame),
    );
    let t = rng.next();
    twin("MsSEP3DfadeOutAll", a, b, 0x007a_08e0, &[t], ALL, |m| {
        g::ms_sep3d_fade_out_all(
            m,
            t.to_le_bytes()[..2]
                .try_into()
                .map_or(0, u16::from_le_bytes),
        )
    });
    let (s, w) = (rng.next(), rng.next());
    twin(
        "MsWeaponReadSet",
        a,
        b,
        0x007a_d940,
        &[actor, s, w],
        0,
        |m| {
            g::ms_weapon_read_set(m, actor, s.to_le_bytes()[0], w);
            0
        },
    );
    let chr = if rng.below(4) == 0 {
        rng.next()
    } else {
        rng.below(0x20)
    };
    twin("MsStoneRegist", a, b, 0x007a_2780, &[chr], ALL, |m| {
        g::ms_stone_regist(m, chr)
    });
    let (who, on) = (rng.range(-1, 20), rng.below(2));
    let cmd = if rng.below(4) == 0 {
        rng.next()
    } else {
        0x3000 | rng.below(0x1000)
    };
    twin(
        "MsSetSaveCommandUse",
        a,
        b,
        0x0078_5e00,
        &[who.cast_unsigned(), cmd, on],
        ALL,
        |m| g::ms_set_save_command_use(m, who, cmd, on),
    );
    twin("MsNumberCalcProcess", a, b, 0x0079_f5e0, &[], ALL, |m| {
        g::ms_number_calc_process(m)
    });
    for (name, sym, mask, rust) in [
        (
            "resetZanmatoSepList",
            0x007a_8f20,
            ALL,
            g::reset_zanmato_sep_list as fn(&mut Oracle) -> u32,
        ),
        ("MsSEPtransInit", 0x007a_0500, ALL, g::ms_sep_trans_init),
        ("MsAtelCamInit", 0x0079_7500, 0, |m| {
            g::ms_atel_cam_init(m);
            0
        }),
        ("MsEffectCameraBufInit", 0x0078_76e0, 0, |m| {
            g::ms_effect_camera_buf_init(m);
            0
        }),
        ("MsInitSaveWeaponEquipFlag", 0x007a_be30, ALL, |m| {
            g::ms_init_save_weapon_equip_flag(m)
        }),
        ("MsAtelCameraInit", 0x007b_a650, 0, |m| {
            g::ms_atel_camera_init(m);
            0
        }),
        ("MsGetNowSelectedCamraLevel", 0x007b_bc30, ALL, |m| {
            cam::ms_get_now_selected_camra_level(m).cast_unsigned()
        }),
        (
            "MsEffectCameraInit",
            0x007b_ba70,
            ALL,
            cam::ms_effect_camera_init,
        ),
    ] {
        twin(name, a, b, sym, &[], mask, rust);
    }
    let id = rng.next();
    let id = if rng.below(2) == 0 {
        // one of a camera's slot IDs, bits 20-23 changed sometimes
        let at = cam::SLOTS_AT.ptr(b).addr + cam::CAMERA_SIZE * rng.below(3) + 8 * rng.below(4);
        let v = u32::from_le_bytes(b.read(at, 4).unwrap().try_into().unwrap());
        v ^ (rng.below(2) << 21)
    } else {
        id
    };
    twin("MsCameraFree", a, b, 0x007b_d000, &[id], ALL, |m| {
        cam::ms_camera_free(m, id)
    });
    let la = if rng.below(4) == 0 { 0 } else { area + 0x780 };
    let ey = if rng.below(4) == 0 { 0 } else { area + 0x790 };
    let (d, r) = (floats(rng), floats(rng));
    twin(
        "MsEffectCameraSet",
        a,
        b,
        0x007b_bb10,
        &[la, ey, d, r],
        ALL,
        |m| cam::ms_effect_camera_set(m, la, ey, d, r),
    );
    let i = rng.range(-1, 5);
    let f: [u32; 9] = std::array::from_fn(|_| floats(rng));
    let mut args = vec![i.cast_unsigned()];
    args.extend(f);
    twin("MsSetScreen", a, b, 0x007b_c7f0, &args, 0, |m| {
        cam::ms_set_screen(m, i, f);
        0
    });
    // MsVectorGetSide: the registers' w come from the original's stack
    let f: [u32; 4] = std::array::from_fn(|_| floats(rng));
    let ra = a.call(a.rebase(0x007c_4180), Conv::Cdecl, &f).unwrap().eax;
    let rb = ffx_battle::vector::bridge::ms_vector_get_side(b, f).cast_unsigned();
    assert_eq!(ra, rb, "MsVectorGetSide {f:x?}");
    let m = ffx_battle::vector::bridge::VF_M.ptr(b).addr;
    for w in [m + 0xc, m + 0x1c] {
        let v = a.read(w, 4).unwrap();
        b.write(w, &v).unwrap();
    }
    same("minor", a, b);
}

fn check(a: &mut Oracle, b: &mut Oracle, rng: &mut Xorshift) {
    let area = scratch(a, b, 0x1000);
    let actor = scratch(a, b, 0xf90);
    let fields = scratch(a, b, 14 * 40);
    let blocks = scratch(a, b, 0x2000);
    let motions = scratch(a, b, 0x800);
    for round in 0..1500 {
        randomize(a, b, rng);
        fill(a, b, rng, area, 0x800);
        fill(a, b, rng, actor, 0xf90);
        let table = if round % 3 == 0 { 0 } else { motions };
        mot_table(a, b, rng, motions);
        poke(a, b, actor + 0x4f8, &table.to_le_bytes());
        // the field list: maps mostly small and positive
        let n = rng.range(-1, 40);
        set(a, b, g::FIELD_COUNT, i16::try_from(n).unwrap());
        for k in 0..40u32 {
            let m = i16::try_from(rng.range(-2, 400)).unwrap();
            poke(a, b, fields + 14 * k, &m.to_le_bytes());
        }
        set(a, b, ffx_battle::state::bridge::FIELD_RECORDS, fields);
        // the encounter blocks: small bytes, so that a group walk stays in
        for k in 0..40u32 {
            poke(
                a,
                b,
                fields + 14 * k + 2,
                &u16::try_from(rng.below(0x400)).unwrap().to_le_bytes(),
            );
        }
        let bytes: Vec<u8> = (0..0x2000)
            .map(|_| u8::try_from(rng.below(8)).unwrap())
            .collect();
        poke(a, b, blocks, &bytes);
        set(a, b, ffx_battle::misc::bridge::FIELD_TABLE2, blocks);
        // a motion binary, sometimes none
        let bin = if rng.below(5) == 0 { 0 } else { area + 0x100 };
        poke(a, b, area + 0x100, &rng.below(0x30).to_le_bytes()[..2]);
        set(a, b, g::MOT_BIN, bin);
        // a monster file's handler section at area + 0x600
        let file = area + 0x600;
        let hn = rng.below(8);
        poke(a, b, file + 1, &[u8::try_from(hn).unwrap()]);
        for h in 0..8u32 {
            let v = if rng.below(6) == 0 {
                0xff
            } else {
                u8::try_from(rng.below(8)).unwrap()
            };
            poke(a, b, file + 2 + h, &[v]);
        }
        let first = hn.div_ceil(2);
        for h in 0..8u32 {
            let tab = 0x40 + 8 * h;
            let at = file + (first + 2 * h) * 2 + 4;
            poke(a, b, at, &u16::try_from(tab).unwrap().to_le_bytes());
            poke(
                a,
                b,
                file + tab,
                &u16::try_from(rng.below(5)).unwrap().to_le_bytes(),
            );
        }
        list(a, b, rng, area + 0x200, 0x10, 0xc, g::SEP_3D.ptr(b).addr);
        list(
            a,
            b,
            rng,
            area + 0x300,
            0x14,
            0x10,
            g::NUMBER_LIST.ptr(b).addr,
        );
        calls(a, b, rng, actor, area);
    }
}

#[test]
fn minor_routines_match_the_original() {
    let Some((mut a, mut b)) = synthetic() else {
        return;
    };
    check(&mut a, &mut b, &mut Xorshift(0x1234_5678));
}

#[test]
fn minor_routines_match_the_original_on_a_snapshot() {
    let Some((mut a, mut b)) = snapshot() else {
        return;
    };
    let mut rng = Xorshift(0x8765_4321);
    let area = scratch(&mut a, &mut b, 0x1000);
    // the real state first: actors, field list, numbers, cameras
    let top = ffx_battle::actor::bridge::CHR_TOP.get(&b);
    for i in 0..ffx_battle::actor::ACTORS {
        let actor = top.wrapping_add(i * ffx_battle::actor::ACTOR_SIZE);
        if !readable(&mut a, actor, 0xf90) {
            break;
        }
        calls(&mut a, &mut b, &mut rng, actor, area);
    }
    check(&mut a, &mut b, &mut rng);
}
