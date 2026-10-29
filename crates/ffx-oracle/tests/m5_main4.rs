//! M5 stage 1, lists `small` and `main`, round 4 (`ffx_sg::m5main4_*`)
//! against the original: on the image alone with synthetic state, and on
//! the snapshots (`FFX_SNAPSHOT`) over the game's memory. Results and
//! every byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! None of these functions ran in a coverage run (debug tools, the PS2
//! movie viewer, teardown and error paths), so every branch is reached
//! with synthetic data; the snapshots add the game's own globals
//! underneath. Callees that stay original are recording stubs in both
//! oracles alike (`m5_main3_common`).

#![allow(
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::similar_names
)]

mod m5_main3_common;

use m5_main3_common::{Out, Ret, Rng, Twins, printf_stub, seq_store, set_edx, store_arg};

use ffx_oracle::Conv;

// ---- FMOD glue ----

use ffx_sg::m5main4_sound::bridge as sound;

#[test]
fn unload_project_by_seq_matches_the_original() {
    // (tree shape, append results, playing, bank 0)
    // shape 1 (a null root) faults in the original (`[0 + 0x10]`): not
    // run
    let cases: [(u32, &[u32], u32, bool); 8] = [
        (0, &[1], 0, false),
        (2, &[1], 0, false),
        (2, &[0, 1], 1, false),
        (2, &[0, 0, 0], 0, false),
        (2, &[1], 0, true),
        (3, &[0, 1], 0, false),
        (3, &[0, 0, 1], 2, false),
        (2, &[0, 1], 0, true),
    ];
    Twins::each(|t| {
        for (k, &(shape, found, playing, null_bank)) in cases.iter().enumerate() {
            for free in [0u32, 0x00ab_0000] {
                t.reset();
                let sfx = t.zeros(0x100);
                let head = sfx + 0x34;
                // three nodes: n2 <- n1 -> n3 (left +0, right +4, key +0xc,
                // bank +0x10); the sentinel head's own key sits between
                let banks = [t.zeros(0x20), t.zeros(0x20), t.zeros(0x20)];
                let n2 = t.words(&[head, head, 0, 5, if null_bank { 0 } else { banks[1] }]);
                let n3 = t.words(&[head, head, 0, 9, banks[2]]);
                let n1 = t.words(&[n2, n3, 0, 7, banks[0]]);
                let root = match shape {
                    0 => head,
                    1 => 0,
                    _ => n1,
                };
                t.w32(head + 0xc, root);
                if shape == 3 {
                    // n1's key equals n2's: the removal finds n1 first
                    t.w32(n1 + 0xc, 5);
                }
                t.stub_ex(
                    sound::TREE_SUCCESSOR,
                    true,
                    1,
                    1,
                    &Ret::Seq(vec![n1, n3, head]),
                    &[],
                );
                let mut extra = store_arg(1, 8, free);
                extra.extend(store_arg(1, 0, 0x77));
                t.stub_ex(
                    sound::APPEND_EVENT_BY_ID,
                    true,
                    2,
                    2,
                    &Ret::Seq(found.iter().map(|&f| 0xab00 | f).collect()),
                    &extra,
                );
                t.stub(sound::STOP_ALL_EVENTS, true, 1, 1, 0x11);
                t.stub(sound::PLAYING_EVENTS, true, 0, 0, playing);
                t.stub(sound::BANK_DTOR, true, 0, 0, 0x12);
                t.stub(sound::OPERATOR_DELETE, false, 1, 0, 0x13);
                t.stub(sound::INT_TREE_ADD, true, 2, 2, 0x14);
                t.stub(sound::TREE_REMOVE, true, 1, 1, 0x15);
                t.stub(sound::FREELIST_DEALLOCATE, true, 1, 1, 0x16);
                t.stub(sound::USER_FREE, false, 1, 0, 0x17);
                let what = format!("unloadProjectBySeq {k} {free:#x}");
                t.skip = 0x10;
                t.check_framed(
                    &what,
                    0x0070_dbb0,
                    Conv::Thiscall(sfx),
                    &[0x33],
                    0x1c,
                    |m, ebp| {
                        Out::U32(sound::unload_project_by_seq(
                            m,
                            sfx,
                            0x33,
                            ebp - 0x1c,
                            ebp + 8,
                        ))
                    },
                );
                t.skip = 4;
            }
        }
    });
}

#[test]
fn shout_start_matches_the_original() {
    let mut rng = Rng(0x0070_e480);
    Twins::each(|t| {
        for k in 0..160 {
            t.reset();
            let names: [&[u8]; 3] = [b"ab\0", b"cd\0", b"x\0"];
            let mut blob = vec![b'M', b'L', 0, 0];
            if k % 5 == 4 {
                blob[(k / 5 % 4) as usize] ^= 0x20;
            }
            let ids = [0x10u32, 0x20, 0x30];
            blob.extend(3u32.to_le_bytes());
            let mut tail = Vec::new();
            for (i, id) in ids.iter().enumerate() {
                blob.extend(id.to_le_bytes());
                blob.extend((8 + 3 * 8 + tail.len() as u32).to_le_bytes());
                tail.extend_from_slice(names[i]);
            }
            blob.extend(tail);
            let blob = t.alloc(&blob);
            let count = rng.pick(&[0u32, 1, 2, 3, 0xffff_ffff]);
            let shout = t.words(&[rng.pick(&[0, 0x5050]), 0, 0, count]);
            if rng.chance(3) {
                t.w32(shout + 4, 0x6060);
            }
            let n0 = t.words(&[0]);
            let n1 = t.words(&[0]);
            // the names the sub-sounds report, in turn
            let pool: Vec<u32> = [&b"cd\0"[..], b"ab\0", b"x\0", b"zz\0"]
                .iter()
                .map(|s| {
                    let mut w = [0u8; 4];
                    w[..s.len()].copy_from_slice(s);
                    u32::from_le_bytes([w[0], w[1], w[2], w[3]])
                })
                .collect();
            let reported: Vec<u32> = (0..4).map(|_| rng.pick(&pool)).collect();
            let sub_name = seq_store(t, 1, &reported);
            t.stub_ex(sound::GET_NAME, false, 3, 3, &Ret::Fixed(0), &sub_name);
            let subs = seq_store(t, 2, &[0x7100, 0x7200, 0x7300, 0x7400]);
            t.stub_ex(sound::GET_SUB_SOUND, false, 3, 3, &Ret::Fixed(0), &subs);
            t.stub(sound::ERRCHECK, false, 3, 0, 0x21);
            t.stub(sound::MEMSET, false, 3, 0, 0x22);
            t.stub(sound::PLAY_SOUND, false, 5, 5, 0x23);
            t.g32(sound::NO_LOAD, rng.pick(&[0, 0, 0, 1, 2]));
            let a = sound::ShoutStart {
                id: rng.pick(&[0x10, 0x20, 0x30, 0x40]),
                system: rng.pick(&[0, 0x9900, 0x9900, 0x9900]),
                mute: rng.pick(&[0, 0, 0, 1, 2]),
                status0: n0,
                status1: n1,
                blob,
            };
            let what = format!("FmodShout_start {k}");
            t.check_framed(
                &what,
                0x0070_e480,
                Conv::Thiscall(shout),
                &[a.id, 0xdead, 0xbeef, a.system, a.mute, n0, n1, blob],
                0x104,
                |m, ebp| Out::U32(sound::shout_start(m, shout, a, ebp - 0x104)),
            );
        }
    });
}

// ---- the movie player manager ----

use ffx_sg::m5main4_movie::bridge as movie;

#[test]
fn fmv_manager_dtor_matches_the_original() {
    let mut rng = Rng(0x006d_6d00);
    Twins::each(|t| {
        for k in 0..12 {
            t.reset();
            for (sym, this, n, ret) in [
                (movie::DESTROY_FMV_PLAYER, true, 0, 0x31),
                (movie::OPERATOR_DELETE, false, 1, 0x32),
                (movie::PHYRE_FREE, false, 1, 0x34),
                (movie::BUFFER_RELEASE, true, 0, 0x35),
                (movie::SAMPLER_DTOR, true, 0, 0x36),
                (movie::CLUSTER_DTOR, true, 0, 0x37),
                (movie::PLAYBACK_DTOR, true, 0, 0x38),
                (movie::USER_FREE, false, 1, 0x39),
            ] {
                t.stub(sym, this, n, 0, ret);
            }
            t.stub(movie::TREE_ERASE_RANGE, true, 3, 3, 0x33);
            let mgr = t.zeros(0x800);
            let sentinel = t.zeros(8);
            let mut next = sentinel;
            for _ in 0..rng.below(4) {
                next = t.words(&[next, 0]);
            }
            t.w32(sentinel, next);
            t.w32(mgr + 0x724, sentinel);
            t.w32(mgr + 0x728, 3);
            let map_head = t.words(&[0x4400, 0, 0]);
            t.w32(mgr + 0x6e8, map_head);
            t.w32(mgr + 0x644, rng.pick(&[0x1000, 0x1001, 0]));
            t.w32(
                mgr + 0x4e8,
                rng.pick(&[0, 1, 2, 0x8000_0005, 0x7fff_ffff, 0x8000_0000]),
            );
            t.w32(mgr + 0x4ec, 0x4ec0);
            t.w32(mgr + 0x18, rng.pick(&[0, 0x1800]));
            t.w32(mgr + 0xc, rng.pick(&[0, 0xc00]));
            let what = format!("PhyFMVPlayerManager_dtor {k}");
            t.skip = 0x10;
            t.check_framed(
                &what,
                0x006d_6d00,
                Conv::Thiscall(mgr),
                &[],
                0x14,
                |m, ebp| Out::U32(movie::dtor(m, mgr, ebp - 0x14)),
            );
            t.skip = 4;
        }
    });
}

#[test]
fn fmv_process_requests_matches_the_original() {
    let mut rng = Rng(0x006d_8cb0);
    Twins::each(|t| {
        for k in 0..80 {
            t.reset();
            t.stub(movie::PHYRE_PRINTF, false, 2, 0, 0x41);
            t.stub(movie::PLAY_VIDEO, true, 1, 1, 0x42);
            t.stub(movie::ARRAY_RESERVE, true, 1, 1, 0x43);
            t.stub(movie::CALL_IGGY_AS3, false, 2, 0, 0x44);
            t.stub(movie::WAIT_FILL_BUFFER, false, 0, 0, 0x45);
            t.stub(movie::SET_MEMORY_ACCESSOR, true, 1, 1, 0x46);
            t.stub(movie::DYN_GEO_CREATE_POOL, true, 1, 1, 0x47);
            t.stub(movie::DYN_GEO_SELECT_BUFFER, true, 1, 1, 0x48);
            t.stub(movie::DYN_GEO_RELEASE_POOL, true, 1, 1, 0x49);
            t.stub(movie::QUEUE_PUSH, true, 2, 2, 0x4a);
            let n = rng.below(5);
            let kinds: Vec<u32> = (0..n)
                .map(|_| {
                    if k % 3 == 0 {
                        rng.pick(&[0, 5, 9])
                    } else {
                        rng.pick(&[0, 1, 2, 3, 4, 5])
                    }
                })
                .collect();
            let flags: Vec<u32> = (0..n).map(|_| rng.pick(&[0, 1, 7])).collect();
            let kinds = t.words(&[kinds, vec![0]].concat());
            let flags = t.words(&[flags, vec![0]].concat());
            let q = t.zeros(0x40);
            t.w32(q + 4, n);
            t.w32(q + 8, rng.pick(&[0, 0xffff_ffff, 4]));
            // a negative count (`Array_Reserve` on `+4`) would loop 2^31
            // times first: not run
            t.w32(q + 0xc, kinds);
            t.w32(q + 0x10, rng.pick(&[0, 0xffff_ffff, 3]));
            t.w32(q + 0x14, rng.pick(&[0, 0xffff_ffff, 3]));
            t.w32(q + 0x18, flags);
            t.w32(q + 0x2c, rng.pick(&[0, 1, 2, 3, 0xffff_ffff]));
            let mgr = t.zeros(0x800);
            for i in 0..4 {
                t.w32(mgr + 0x700 + 4 * i, u32::from(!rng.chance(6)));
            }
            t.w8(mgr + 0x6d2, u8::from(rng.chance(5)));
            // without the singleton, kind 3 reads `[0 + 0x700]` and faults
            let none = (0..n).all(|i| {
                let kind: u32 = ffx_bridge::Mem::read(&t.a, kinds + 4 * i);
                kind != 3
            }) && rng.chance(4);
            t.g32(movie::FMV_PLAYER_MANAGER, if none { 0 } else { mgr });
            let fill = t.words(&[0, rng.pick(&[0, 1])]);
            t.g32(movie::DYN_GEO_FILL_BUFFER, fill);
            let what = format!("processRequests {k}");
            t.check(&what, 0x006d_8cb0, Conv::Thiscall(q), &[], |m| {
                movie::process_requests(m, q);
                Out::None
            });
        }
    });
}

#[test]
fn fmv_update_video_callback_matches_the_original() {
    let mut rng = Rng(0x006d_ae00);
    Twins::each(|t| {
        for k in 0..60 {
            t.reset();
            t.stub_import(movie::INTERLOCKED_COMPARE_EXCHANGE, 3, 3, 0);
            t.stub(
                movie::UPDATE_TEXTURE,
                true,
                3,
                3,
                rng.pick(&[0, 1, 0x100, 0xff01]),
            );
            let (lo, hi) = match k % 4 {
                0 => (rng.next(), rng.below(4)),
                1 => (rng.next(), rng.next()),
                2 => (rng.below(100_000), 0),
                _ => (rng.next(), 0x8000_0000 | rng.below(16)),
            };
            t.stub_ex(
                movie::GET_CURRENT_TIME,
                true,
                0,
                0,
                &Ret::Fixed(lo),
                &set_edx(hi),
            );
            t.stub(movie::OPERATOR_DELETE, false, 1, 0, 0x51);
            let frame = t.words(&[rng.pick(&[0, 5, 0xfffd, 0xfffe, 0xffff_ffff, 0x10000])]);
            let last = t.words(&[0x1234]);
            let updated = t.zeros(4);
            let visible = t.words(&[rng.pick(&[0, 1, 0x100])]);
            let job = t.words(&[0x7700, 1, 2, 3, frame, last, updated, visible, 0x2020]);
            let task = t.words(&[0, job]);
            let what = format!("updateVideoCallback {k}");
            t.check(&what, 0x006d_ae00, Conv::Cdecl, &[task], |m| {
                Out::U32(movie::update_video_callback(m, task))
            });
        }
    });
}

// ---- the PS2 movie and HDD helpers ----

fn printf_all(t: &mut Twins) {
    printf_stub(
        t,
        movie::DBG_PRINTF,
        &[
            (movie::DASHES, 1),
            (movie::RESOURCE_TITLE, 1),
            (movie::WRITE_ERR1, 1),
            (movie::WRITE_ERR2, 1),
            (movie::WRITE_ERR3, 1),
            (movie::READ_END, 1),
            (movie::READ_WAIT, 3),
        ],
        2,
        0x61,
    );
}

#[test]
fn hdd_resource_write_matches_the_original() {
    let mut rng = Rng(0x0077_9d20);
    Twins::each(|t| {
        for k in 0..30 {
            t.reset();
            printf_all(t);
            t.stub_import(movie::STRNCPY, 3, 0, 0x62);
            t.stub(movie::MEMSET, false, 3, 0, 0x63);
            t.stub(movie::MEMCPY, false, 3, 0, 0x64);
            let fails = |rng: &mut Rng| {
                if rng.chance(4) { 0xffff_fff0 } else { 0x200 }
            };
            let writes: Vec<u32> = (0..4).map(|_| fails(&mut rng)).collect();
            let seeks: Vec<u32> = (0..3).map(|_| fails(&mut rng)).collect();
            t.stub_ex(movie::SCE_WRITE, false, 3, 0, &Ret::Seq(writes), &[]);
            t.stub_ex(movie::SCE_LSEEK, false, 3, 0, &Ret::Seq(seeks), &[]);
            let buffer = t.zeros(0x80);
            t.g32(movie::RESOURCE_BUFFER, buffer);
            t.g32(movie::SYSTEM_CNF, 0x5c00);
            t.g32(
                movie::SYSTEM_CNF_SIZE,
                rng.pick(&[0, 1, 0x1ff, 0x200, 0x201, 0xffff_fff0, 0x7fff_ffff]),
            );
            t.g32(movie::ICON_DATA, 0x1c00);
            t.g32(movie::ICON_SIZE, rng.pick(&[0, 0x3ff, 0x10000]));
            let title_len = rng.pick(&[0u32, 3, 0x20, 0x1ff]);
            for i in 0..=title_len.min(0x1f) {
                t.g8(movie::ICON_TITLE + i, if i == title_len { 0 } else { b'a' });
            }
            let what = format!("resource write {k}");
            t.check_framed(&what, 0x0077_9d20, Conv::Cdecl, &[7], 0x204, |m, ebp| {
                Out::U32(movie::resource_write(m, 7, ebp - 0x204))
            });
        }
    });
}

#[test]
fn movie_open_and_read_match_the_original() {
    let mut rng = Rng(0x0077_7240);
    Twins::each(|t| {
        for k in 0..30 {
            t.reset();
            printf_all(t);
            t.stub(movie::HDD_STATE_CLEAR, false, 0, 0, 0x71);
            t.stub(movie::HDD_FILE_OPEN, false, 2, 0, 0x72);
            let viewer = t.zeros(0x5_0800);
            t.w8(viewer + 0x5_075d, rng.pick(&[0, 1, 0x80]));
            t.w8(viewer + 0x5_0764, rng.pick(&[0, 3, 0xfe]));
            t.w32(viewer + 0x5_06e4, rng.next());
            t.w32(viewer + 0x5_06e8, rng.next());
            let what = format!("movieOpenCd {k}");
            t.check(&what, 0x0077_7240, Conv::Cdecl, &[viewer], |m| {
                Out::U32(movie::open_cd(m, viewer))
            });
        }
        for k in 0..40 {
            t.reset();
            printf_all(t);
            t.stub(movie::GET_HDD_NAME, false, 1, 0, 0x5a00);
            t.stub(movie::SCE_OPEN, false, 2, 0, rng.pick(&[3, 3, 0xffff_ffff]));
            t.stub(
                movie::SCE_LSEEK64,
                false,
                3,
                0,
                rng.pick(&[0, 0, 0x8000_0000]),
            );
            let busy: Vec<u32> = (0..rng.below(4)).map(|i| i + 1).chain([0]).collect();
            let ioctl = seq_store(t, 2, &busy);
            t.stub_ex(movie::SCE_IOCTL, false, 3, 0, &Ret::Fixed(0x73), &ioctl);
            t.stub(movie::TASK_WAIT, false, 1, 0, 0x74);
            let n = rng.below(51);
            let e = movie::HDD_HEADS + 12 * n;
            t.g32(e, rng.below(0x1000));
            t.g32(e + 4, rng.pick(&[0, 1]));
            let sector = rng.below(0x2000);
            let what = format!("movie_hdd_file_open {k}");
            let ebp = t.frame_ebp(2);
            t.check(&what, 0x0077_9be0, Conv::Cdecl, &[n, sector], |m| {
                Out::U32(movie::hdd_file_open(m, n, sector, ebp + 8))
            });
        }
        for k in 0..30 {
            t.reset();
            printf_all(t);
            // an open failure waits forever: not run
            t.stub(movie::SCE_OPEN, false, 2, 0, 4);
            let size = rng.pick(&[0x100, 0, 0xffff_ffff]);
            let rewind = rng.pick(&[0, 0, 0xffff_fff0]);
            t.stub_ex(
                movie::SCE_LSEEK,
                false,
                3,
                0,
                &Ret::Seq(vec![size, rewind]),
                &[],
            );
            t.stub(movie::FLUSH_CACHE, false, 1, 0, 0x81);
            t.stub(movie::SCE_READ, false, 3, 0, rng.pick(&[size, size, 3]));
            t.stub(movie::SCE_CLOSE, false, 1, 0, 0x82);
            let name = t.alloc(b"file.bin\0");
            let what = format!("readSizeFile {k}");
            t.check(&what, 0x0093_da60, Conv::Cdecl, &[name, 0x4000], |m| {
                Out::U32(movie::read_size_file(m, name, 0x4000))
            });
        }
    });
}

// ---- leftovers ----

use ffx_sg::m5main4_misc::bridge as misc;

#[test]
fn camera_default_points_match_the_original() {
    let mut rng = Rng(0x0094_1980);
    Twins::each(|t| {
        for k in 0..8 {
            t.reset();
            let list = rng.below(2);
            t.g32(misc::CUR_LIST, list);
            t.g32(misc::LIST_COUNT + 4 * list, rng.pick(&[0, 0, 3]));
            let flags = t.zeros(0x40);
            let points = t.zeros(0x60);
            t.g32(misc::LIST_FLAGS + 4 * list, flags);
            t.g32(misc::LIST_POINTS + 4 * list, points);
            let what = format!("camera default points {k}");
            t.check(&what, 0x0094_1980, Conv::Cdecl, &[], |m| {
                misc::default_points(m);
                Out::None
            });
        }
    });
}

// ---- libogg's bit packer ----

use ffx_sg::m5main4_ogg::bridge as ogg;

/// A packer buffer over `n` bytes of storage (random bytes), at end byte
/// `end` and end bit `bit`.
fn ogg_buffer(t: &mut Twins, rng: &mut Rng, n: u32, end: u32, bit: u32, null: bool) -> u32 {
    let bytes: Vec<u8> = (0..n + 8).map(|_| rng.next() as u8).collect();
    let storage = t.alloc(&bytes);
    let ptr = if null { 0 } else { storage + end };
    t.words(&[end, bit, if null { 0 } else { storage }, ptr, n])
}

#[test]
fn oggpack_write_matches_the_original() {
    let mut rng = Rng(0x0042_e350);
    Twins::each(|t| {
        for k in 0..300 {
            t.reset();
            let grown = t.zeros(0x200);
            let realloc = t.stub_code(
                ogg::REALLOC,
                false,
                2,
                0,
                &Ret::Fixed(rng.pick(&[grown, grown, 0])),
                &[],
            );
            let free = t.stub_code(ogg::FREE, false, 1, 0, &Ret::Fixed(0x91), &[]);
            t.g32(ogg::REALLOC, realloc);
            t.g32(ogg::FREE, free);
            let n = rng.pick(&[0x10u32, 0x20, 0x7fff_ff00, 0x7fff_feff, 0x8000_0000]);
            let n_real = if n > 0x100 { 0x20 } else { n };
            let mut end = rng.pick(&[0u32, 3, n_real - 5, n_real - 4, n_real - 1]);
            if n == 0x7fff_ff00 && rng.chance(2) {
                // at the grow mark of a huge storage: the size check decides
                end = n - 4;
            }
            let bit = rng.pick(&[0u32, 1, 3, 7, 8, 9, 0xffff_ffe0, 0xffff_fff9]);
            // a null cursor below the grow mark writes to 0 (faults)
            let null = rng.chance(8) && n == n_real && end >= n_real - 4;
            let b = ogg_buffer(t, &mut rng, n_real, end.min(n_real), bit, null);
            if n != n_real {
                t.w32(b + 0x10, n);
                t.w32(b, end);
                if end > n_real {
                    // a cursor the size check keeps the writes from
                    t.w32(b + 0xc, 0x10);
                }
            }
            let bits = rng.pick(&[0u32, 1, 5, 8, 9, 16, 17, 24, 25, 31, 32, 33, 0xffff_ffff]);
            let v = rng.next();
            let what = format!("oggpack_write {k}");
            t.check(&what, 0x0042_e350, Conv::Cdecl, &[b, v, bits], |m| {
                ogg::write(m, b, v, bits);
                Out::None
            });
            let what = format!("oggpackB_write {k}");
            t.check(&what, 0x0042_e480, Conv::Cdecl, &[b, v, bits], |m| {
                ogg::write_b(m, b, v, bits);
                Out::None
            });
        }
    });
}

#[test]
fn oggpack_read_matches_the_original() {
    let mut rng = Rng(0x0042_eb10);
    Twins::each(|t| {
        for k in 0..400 {
            t.reset();
            let n = rng.pick(&[0x10u32, 0x20, 4, 1]);
            let end = rng.pick(&[
                0u32,
                3,
                n.saturating_sub(5),
                n.saturating_sub(4),
                n - 1,
                n,
                n + 1,
            ]);
            let bit = rng.pick(&[0u32, 1, 3, 7, 8, 9]);
            let b = ogg_buffer(t, &mut rng, n, end, bit, false);
            let mut bits = rng.pick(&[0u32, 1, 5, 8, 9, 16, 17, 24, 25, 31, 32, 33, 0xffff_ffff]);
            if k % 7 == 0 {
                // past the end with nothing to read: the cursor is never read
                t.w32(b, n);
                t.w32(b + 4, 0);
                t.w32(b + 0xc, 0);
                bits = 0;
            }
            let what = format!("oggpack_look {k}");
            t.check(&what, 0x0042_e840, Conv::Fastcall, &[b, bits], |m| {
                Out::U32(ogg::look(m, b, bits))
            });
            let what = format!("oggpack_read {k}");
            t.check(&what, 0x0042_eb10, Conv::Fastcall, &[b, bits], |m| {
                Out::U32(ogg::read(m, b, bits))
            });
            let what = format!("oggpack_adv {k}");
            t.check(&what, 0x0042_ea30, Conv::Fastcall, &[b, bits], |m| {
                ogg::adv(m, b, bits);
                Out::None
            });
        }
    });
}

// ---- libwebm and the heap ----

use ffx_sg::m5main4_lib::bridge as lib;

#[test]
fn webm_block_time_and_frame_match_the_original() {
    let mut rng = Rng(0x0042_df70);
    Twins::each(|t| {
        for k in 0..60 {
            t.reset();
            t.stub_import(lib::WASSERT, 3, 0, 0xa1);
            let status = rng.pick(&[0u32, 1, 0xffff_fffd, 0x4000_0001, 0x8000_0000]);
            let mut extra = store_arg(0, 0, rng.next());
            extra.extend(store_arg(0, 4, rng.below(3)));
            extra.extend(store_arg(1, 0, 1));
            t.stub_ex(lib::CLUSTER_LOAD, true, 2, 2, &Ret::Fixed(status), &extra);
            let info = t.words(&[0; 10]);
            t.w32(info + 0x28, rng.pick(&[0, 1, 1_000_000, 0xffff_ffff]));
            t.w32(info + 0x2c, rng.pick(&[0, 0, 1, 0xffff_ffff]));
            let segment = t.zeros(0x80);
            t.w32(segment + 0x70, info);
            let cluster = t.zeros(0x40);
            t.w32(cluster, segment);
            t.w32(cluster + 0x28, rng.next());
            t.w32(cluster + 0x2c, rng.pick(&[0, 1, 0x8000_0000, 0xffff_ffff]));
            let block = t.zeros(0x40);
            t.write(block + 0x18, &(rng.next() as u16).to_le_bytes());
            let what = format!("Block::GetTime {k}");
            t.check_framed(
                &what,
                0x0042_df70,
                Conv::Thiscall(block),
                &[cluster],
                0x10,
                |m, ebp| {
                    let r = lib::block_get_time(m, block, cluster, ebp - 0x10, ebp + 8);
                    Out::U64(r)
                },
            );
        }
        for k in 0..40 {
            t.reset();
            t.stub_import(lib::WASSERT, 3, 0, 0xa1);
            let frames: Vec<u32> = (0..16)
                .map(|_| rng.pick(&[0, 1, 5, 0xffff_ffff, 0x8000_0000]))
                .collect();
            let frames = t.words(&frames);
            let block = t.zeros(0x40);
            t.w32(block + 0x1c, frames);
            t.w32(block + 0x20, rng.pick(&[0, 2, 4, 0xffff_ffff]));
            let idx = rng.pick(&[0u32, 1, 2, 3, 4, 0xffff_ffff]);
            let what = format!("Block::GetFrame {k}");
            t.check(&what, 0x0042_e0d0, Conv::Thiscall(block), &[idx], |m| {
                Out::U32(lib::block_get_frame(m, block, idx))
            });
        }
    });
}

#[test]
fn mspace_release_segments_matches_the_original() {
    let mut rng = Rng(0x006e_bda0);
    Twins::each(|t| {
        for k in 0..40 {
            t.reset();
            // VirtualQuery fills the 28-byte info from a table, per call
            let calls = 8;
            let mut fields: Vec<Vec<u32>> = vec![Vec::new(); 5];
            let base = 0x0100_0000;
            for i in 0..calls {
                let b = base + 0x1000 * i;
                fields[0].push(if rng.chance(8) { b + 1 } else { b });
                fields[1].push(if rng.chance(8) { 0 } else { b });
                fields[2].push(rng.pick(&[0x1000, 0x1000, 0x1000, 0x2000, 0x8000]));
                fields[3].push(if rng.chance(8) { 0x2000 } else { 0x1000 });
                fields[4].push(0);
            }
            let mut extra = Vec::new();
            for (f, off) in [(0usize, 0u32), (1, 4), (2, 0xc), (3, 0x10)] {
                let vals = fields[f].clone();
                // write through arg 1 at `off`: the table entry for this call
                let mut c = seq_store(t, 1, &vals);
                if off != 0 {
                    // `mov [ecx], edx` -> `mov [ecx + off], edx`
                    let n = c.len();
                    c.truncate(n - 2);
                    c.extend([0x89, 0x51, off as u8]);
                }
                extra.extend(c);
            }
            let q = rng.pick(&[1, 1, 1, 0]);
            t.stub_import(lib::VIRTUAL_QUERY, 3, 3, 0);
            let at = t.rebase(lib::VIRTUAL_QUERY);
            let code = t.stub_code(lib::VIRTUAL_QUERY, false, 3, 3, &Ret::Fixed(q), &extra);
            t.write(at, &code.to_le_bytes());
            t.stub_import(lib::VIRTUAL_FREE, 3, 3, rng.pick(&[1, 1, 1, 0]));
            // segments: base, size, next, flags
            let mut next = 0;
            for i in 0..rng.below(4) {
                let flags = rng.pick(&[1u32, 1, 9, 0, 3]);
                let size = rng.pick(&[0u32, 0x1000, 0x2000, 0x3000]);
                next = t.words(&[base + 0x1000 * i, size, next, flags]);
            }
            let m = t.zeros(0x1d8);
            let first: Vec<u32> = vec![
                base,
                rng.pick(&[0, 0x1000, 0x2000]),
                next,
                rng.pick(&[1, 9, 0]),
            ];
            for (i, w) in first.iter().enumerate() {
                t.w32(m + 0x1c8 + 4 * i as u32, *w);
            }
            let what = format!("release segments {k}");
            t.check_framed(&what, 0x006e_bda0, Conv::Cdecl, &[m], 0x20, |mm, ebp| {
                Out::U32(lib::release_segments(mm, m, ebp - 0x20))
            });
        }
    });
}

// ---- characters: texture animation packets, the model viewer ----

use ffx_sg::m5main4_chara::bridge as chara;

#[test]
fn texanim_image_tags_match_the_original() {
    let mut rng = Rng(0x0077_eff0);
    Twins::each(|t| {
        for k in 0..60 {
            t.reset();
            let q = t.zeros(0x100);
            let a: [u32; 7] = std::array::from_fn(|_| rng.word());
            let what = format!("setLoadImageTagsTexAnim {k}");
            t.check(
                &what,
                chara::SET_LOAD_IMAGE_TAGS,
                Conv::Cdecl,
                &[q, a[0], a[1], a[2], a[3], a[4], a[5], a[6]],
                |m| {
                    Out::U32(chara::set_load_image_tags(
                        m, q, a[0], a[1], a[2], a[3], a[4], a[5], a[6],
                    ))
                },
            );
        }
    });
}

/// A texture animation work area: `n` records of 0x34 bytes from `first`,
/// skip bytes at `skips`.
fn texanim_work(t: &mut Twins, rng: &mut Rng, size: u32, first: u32, skips: u32, n: u32) -> u32 {
    let w = t.zeros(size as usize);
    for k in 0..n.min(8) {
        let rec = w + first + 0x34 * k;
        t.w32(rec, rng.pick(&[0, 0x8012_3400, 0x400]));
        for o in (8..0x14).step_by(2) {
            let v = rng.pick(&[0u16, 4, 0x20, 0x40, 0xfff0, 0x100]);
            t.write(rec + o, &v.to_le_bytes());
        }
        t.write(rec + 0x1c, &rng.pick(&[0u16, 3, 1]).to_le_bytes());
        t.write(rec + 0x1e, &rng.pick(&[0u16, 0, 0, 2]).to_le_bytes());
        t.write(rec + 0x20, &rng.pick(&[0u16, 1, 0xffff]).to_le_bytes());
        t.w8(w + skips + k, rng.pick(&[0, 1, 0xff]));
    }
    w
}

#[test]
fn texanim_draw_matches_the_original() {
    let mut rng = Rng(0x0077_c830);
    Twins::each(|t| {
        for k in 0..60 {
            t.reset();
            printf_stub(
                t,
                chara::DBG_PRINTF,
                &[(chara::AREA_OVER, 1), (chara::PKT_SIZE, 2)],
                2,
                0xb1,
            );
            t.stub(chara::GET_IMAGE_BASE, false, 2, 0, 0xb2);
            t.stub(chara::PAD_PRINT_FLAG, false, 0, 0, 0xb3);
            t.stub(chara::ANIM_GET_HANDLE, false, 1, 0, 0xb4);
            t.stub(chara::GET_NAME, false, 1, 0, 0xb5);
            let q = 0x0040_0000;
            let step = rng.pick(&[0x80, 0x80, 0x400, 0x500]);
            let ends: Vec<u32> = (1..10).map(|i| q + step * i).collect();
            t.stub_ex(
                chara::SET_LOAD_IMAGE_TAGS,
                false,
                8,
                0,
                &Ret::Seq(ends),
                &[],
            );
            let i = rng.below(64);
            let e = chara::TEX_ANIM + 0x30 * i;
            let src = t.words(&[rng.pick(&[2, 2, 2, 1, 0x102])]);
            t.g32(e, if rng.chance(8) { 0 } else { src });
            let n = rng.pick(&[0u32, 1, 3, 6, 0xffff_ffff]);
            let old = texanim_work(t, &mut rng, 0x600, 0x408, 0x5a8, n);
            t.w32(old + 0x400, n);
            t.w32(old + 0x404, 0x5150);
            let new = texanim_work(t, &mut rng, 0xd50, 8, 0xd08, n);
            t.w32(new, n);
            t.w32(new + 4, 0x5151);
            t.w32(new + 0xd48, rng.pick(&[0, 0, 0, 1, 2]));
            t.g32(e + 4, old);
            let what = format!("chr_texanim_draw_sub_old {k}");
            t.check(&what, chara::DRAW_SUB_OLD, Conv::Cdecl, &[i, q], |m| {
                Out::U32(chara::draw_sub_old(m, i, q))
            });
            t.g32(e + 4, new);
            let what = format!("chr_texanim_draw_sub_new {k}");
            t.check(&what, chara::DRAW_SUB_NEW, Conv::Cdecl, &[i, q], |m| {
                Out::U32(chara::draw_sub_new(m, i, q))
            });
        }
        for f in [0u8, 1, 2, 0xff] {
            t.reset();
            t.stub(chara::DRAW_SUB_OLD, false, 2, 0, 0xc1);
            t.stub(chara::DRAW_SUB_NEW, false, 2, 0, 0xc2);
            let i = 5;
            t.g8(chara::TEX_ANIM + 0x30 * i + 0xd, f);
            let what = format!("chr_texanim_draw {f}");
            t.check(&what, 0x0077_c830, Conv::Cdecl, &[i, 0x777], |m| {
                Out::U32(chara::draw(m, i, 0x777))
            });
        }
    });
}

#[test]
fn viewer_motion_groups_and_names_match_the_original() {
    let mut rng = Rng(0x0083_69a0);
    Twins::each(|t| {
        for k in 0..30 {
            t.reset();
            let data = t.zeros(0x10);
            t.write(data + 8, &rng.pick(&[0u16, 5, 0x8001]).to_le_bytes());
            let entry = t.words(&[0, 0, 0, data]);
            t.stub(chara::FIND_MOTION_GROUP, false, 3, 0, rng.pick(&[0, 0, 1]));
            t.stub(chara::CACHE_EXIST, false, 2, 0, entry);
            t.stub(
                chara::CACHE_CHECK_YET,
                false,
                2,
                0,
                rng.pick(&[0, 1, 2, 2, 3]),
            );
            t.stub(chara::MGRP_READ, false, 2, 0, rng.pick(&[0, 1]));
            t.stub(chara::READ_SYNC, false, 1, 0, 0xd1);
            t.stub(chara::CACHE_USE, false, 2, 0, 0xd2);
            t.stub(chara::ADD_MOTION_GROUP, false, 4, 0, 0xd3);
            let (id, n) = (rng.below(0x100), rng.below(0x10));
            let what = format!("ChN_ReadSystemMGRP_ViewerAnimation {k}");
            t.check(&what, 0x0083_69a0, Conv::Cdecl, &[id, n], |m| {
                Out::U32(chara::read_mgrp_viewer(m, id, n))
            });
        }
        for k in 0..12 {
            t.reset();
            t.stub_import(chara::STRNCPY, 3, 0, 0xe1);
            t.stub(
                chara::CREATE_PREALLOCATE,
                false,
                1,
                0,
                rng.pick(&[0, 0xe2, 0xe3]),
            );
            let task = t.zeros(0x110);
            let path = rng.pick(&[
                &b"chr/c001.cfg\0"[..],
                b"abcdefg\0",
                b"x/y/z/cloth_0001.bin\0",
            ]);
            t.write(task, path);
            let (a, b) = (t.zeros(4), t.zeros(4));
            t.w32(task + 0x100, a);
            t.w32(task + 0x104, b);
            t.w32(task + 0x10c, 9);
            t.w8(task + 0x108, 1);
            let what = format!("Ch_LoadClothFileTask {k}");
            t.check(&what, 0x0082_a140, Conv::Cdecl, &[task], |m| {
                Out::U32(chara::load_cloth_file_task(m, task))
            });
        }
        let names: [&[u8]; 4] = [b"c001\0", b"m002\0", b"c001x\0", b"\0"];
        let dirs: [&[u8]; 3] = [b"chr\0", b"mon\0", b"Chr\0"];
        for k in 0..30 {
            t.reset();
            let copies: Vec<u32> = (0..3)
                .map(|_| {
                    let s = rng.pick(&[&b"abc\0"[..], b"Zed\0", b"{x\0", b"`y\0"]);
                    t.alloc(s)
                })
                .collect();
            let at = t.rebase(chara::STRDUP);
            let code = t.stub_code(chara::STRDUP, false, 1, 0, &Ret::Seq(copies), &[]);
            t.patch(at, &code.to_le_bytes());
            let count = rng.pick(&[0u32, 1, 3, 5]);
            for i in 0..count {
                let d = rng.pick(&dirs);
                let d = t.alloc(d);
                let n = rng.pick(&names);
                let n = t.alloc(n);
                let e = chara::VIEWER_MODELS + 12 * i;
                t.g32(e, d);
                t.g32(e + 8, n);
            }
            t.g32(chara::VIEWER_MODEL_COUNT, count);
            let name = rng.pick(&names);
            let name = t.alloc(name);
            let dir = rng.pick(&dirs);
            let dir = t.alloc(dir);
            let what = format!("Ch_IdGetFromName {k}");
            t.check(&what, 0x0082_6dc0, Conv::Cdecl, &[name, dir], |m| {
                Out::U32(chara::id_get_from_name(m, name, dir))
            });
        }
    });
}
