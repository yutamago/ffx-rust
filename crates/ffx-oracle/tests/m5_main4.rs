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

// ---- the PS2 movie viewer ----

use ffx_sg::m5main4_viewer::bridge as viewer;

/// A stub extra that sets the byte `[arg0 + off]` to 1 on the stub's
/// `n`-th call (counting from 1).
fn set_byte_on_call(t: &mut Twins, n: u32, off: u32) -> Vec<u8> {
    let ctr = t.words(&[n]);
    let mut c = vec![0xff, 0x0d];
    c.extend(ctr.to_le_bytes()); // dec dword [ctr]
    c.extend([0x75, 0x0b]); // jnz +11
    c.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp + 4]
    c.extend([0xc6, 0x81]);
    c.extend(off.to_le_bytes());
    c.push(1); // mov byte [ecx + off], 1
    c
}

/// A viewer of 0x51000 zero bytes as the global viewer.
fn new_viewer(t: &mut Twins) -> u32 {
    let v = t.zeros(0x5_1000);
    t.g32(viewer::VIEWER, v);
    v
}

#[test]
fn viewer_tags_and_block_step_match_the_original() {
    let mut rng = Rng(0x0077_f160);
    Twins::each(|t| {
        for n in [
            0u32,
            0xffff_fffb,
            1,
            0x10,
            0xf_fff0,
            0xf_fff1,
            0x20_0000,
            0x2ff_ffff,
        ] {
            t.reset();
            t.stub(viewer::GS_PACK_TAG, false, 4, 0, 0x11);
            let what = format!("ref tags {n:#x}");
            t.check(
                &what,
                viewer::REF_TAGS,
                Conv::Cdecl,
                &[0x100, 0x5000, n],
                |m| {
                    viewer::ref_tags(m, 0x100, 0x5000, n);
                    Out::None
                },
            );
        }
        for k in 0..30 {
            t.reset();
            for s in [viewer::ADD_BLOCK1, viewer::ADD_BLOCK2, viewer::CLEAR_BLOCK0] {
                t.stub(s, false, 1, 0, 0x21);
            }
            t.stub(viewer::LOAD_IMAGE_REGS, false, 3, 0, 0x22);
            let block = t.zeros(0x400);
            t.g32(viewer::BLOCK, block);
            let v = new_viewer(t);
            t.w32(v + 0x5_03cc, 0x0060_0000);
            let (i, x, y) = (rng.below(8), rng.word(), rng.word());
            let mode = rng.pick(&[0u32, 1, 2, 3, 5, 0xffff_ffff]);
            let what = format!("block step {k}");
            t.check(
                &what,
                viewer::BLOCK_STEP,
                Conv::Cdecl,
                &[i, x, y, mode],
                |m| Out::U32(viewer::block_step(m, i, x, y, mode)),
            );
        }
        for k in 0..30 {
            t.reset();
            t.stub(viewer::FLUSH_CACHE, false, 1, 0, 0x31);
            t.stub(viewer::SET_DMA_TAG, false, 7, 0, 0x32);
            t.stub(viewer::SET_GIF_TAG, false, 9, 0, 0x33);
            t.stub(viewer::SET_TAG, false, 5, 0, 0x34);
            t.stub(viewer::TRXREG, false, 3, 0, 0x35);
            t.stub(viewer::TRXPOS, false, 4, 0, 0x36);
            t.stub(viewer::TRXDIR, false, 2, 0, 0x37);
            let w = rng.pick(&[0u32, 0x10, 0x30, 0x40, 0x1f, 0xffff_fff0]);
            let hh = rng.pick(&[0u32, 0x10, 0x20, 0x3f, 0xffff_ffe0]);
            let half = rng.pick(&[0, 1, 0x100]);
            let a = [0x4000, 0x8000, 0x400, w, hh, 0x3400, half, 0x13];
            let what = format!("tiled upload {k}");
            t.check(&what, viewer::TILED_UPLOAD, Conv::Cdecl, &a, |m| {
                Out::U32(viewer::tiled_upload(
                    m, a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7],
                ))
            });
        }
    });
}

#[test]
fn viewer_ipu_movie_matches_the_original() {
    let mut rng = Rng(0x0077_c4c0);
    Twins::each(|t| {
        for k in 0..30 {
            t.reset();
            printf_stub(
                t,
                viewer::DBG_PRINTF,
                &[(viewer::PICTURE_MOVIE, 4)],
                2,
                0x41,
            );
            for (s, n) in [
                (viewer::IPU_INIT, 0),
                (viewer::HARDWARE_INIT, 0),
                (viewer::TILED_UPLOAD, 8),
                (viewer::CLUT_SWIZZLE, 3),
                (viewer::FLUSH_CACHE, 1),
                (viewer::REF_TAGS, 3),
                (viewer::IPU_SYNC, 2),
                (viewer::SYNC_PATH, 1),
                (viewer::LOAD_IMAGE, 2),
                (viewer::TASK_WAIT, 1),
            ] {
                t.stub(s, false, n, 0, 0x42);
            }
            t.stub(viewer::GS_SYNC_PATH, false, 2, 0, rng.pick(&[0x43, 0x44]));
            let stop = set_byte_on_call(t, 1 + rng.below(8), 0x60fc);
            t.stub_ex(viewer::SYNC_PATH2, false, 1, 0, &Ret::Fixed(0x45), &stop);
            // the header and its records (frames 1..3; the last rewinds)
            let mut e = Vec::new();
            e.extend(rng.next().to_le_bytes());
            e.extend(rng.next().to_le_bytes());
            e.extend(rng.next().to_le_bytes());
            e.extend((rng.pick(&[0x40u16, 0x200, 0x10])).to_le_bytes());
            e.extend((rng.pick(&[0x40u16, 0x1a0, 0x1f])).to_le_bytes());
            e.resize(0x20, 0);
            // 1..3 records of 1..3 frames, then one that rewinds
            let records = 1 + rng.below(3);
            for r in 0..=records {
                let size = 0x10 * (1 + rng.below(3));
                let frames = if r == records {
                    u32::MAX
                } else {
                    1 + rng.below(3)
                };
                e.extend(0u32.to_le_bytes());
                e.extend(size.to_le_bytes());
                e.extend(frames.to_le_bytes());
                e.extend(0u32.to_le_bytes());
                e.extend(vec![0u8; size as usize]);
            }
            let mut head = vec![0u8; 0x10];
            head[8..12].copy_from_slice(&0x10u32.to_le_bytes());
            head.extend(e);
            let head = t.alloc(&head);
            let v = t.zeros(0x4_6100);
            t.w32(v + 0x4_60f4, head);
            t.w32(v + 0x4_60e8, 9);
            t.w32(v + 0x4_60ec, 0x10);
            let what = format!("ipu movie {k}");
            t.check(&what, viewer::IPU_MOVIE_SYM, Conv::Cdecl, &[v], |m| {
                Out::U32(viewer::ipu_movie(m, v))
            });
        }
    });
}

#[test]
fn viewer_decode_and_frame_out_match_the_original() {
    let mut rng = Rng(0x0077_b480);
    Twins::each(|t| {
        for k in 0..30 {
            t.reset();
            t.stub(viewer::LOAD_IMAGE_101, false, 2, 0, 0x51);
            t.stub(viewer::LOAD_IMAGE, false, 2, 0, 0x52);
            // each register load adds the GS register to a cell: the
            // commands written before it become visible
            let cell = t.zeros(4);
            let mut sum = vec![0xa1];
            sum.extend(t.rebase(viewer::GS_SINK).to_le_bytes()); // mov eax, [sink]
            sum.extend([0x01, 0x05]);
            sum.extend(cell.to_le_bytes()); // add [cell], eax
            t.stub_ex(
                viewer::LOAD_IMAGE_REGS,
                false,
                3,
                0,
                &Ret::Fixed(0x53),
                &sum,
            );
            t.stub(viewer::BLOCK_STEP, false, 4, 0, 0x54);
            let mut ready: Vec<u32> = (0..rng.below(4)).map(|_| rng.pick(&[1, 2])).collect();
            ready.push(0);
            t.stub_ex(viewer::SINK_READY, false, 1, 0, &Ret::Seq(ready), &[]);
            let block = t.zeros(0x100);
            t.g32(viewer::BLOCK, block);
            let v = new_viewer(t);
            let rec: Vec<u8> = (0..0x20).map(|_| rng.next() as u8).collect();
            let rec = t.alloc(&rec);
            t.w32(v + 0x5_059c, rec);
            t.w32(v + 0x5_03c8, 0x0070_0000);
            let what = format!("intra decode {k}");
            t.check(&what, viewer::INTRA_DECODE, Conv::Cdecl, &[], |m| {
                Out::U32(viewer::intra_decode(m))
            });
        }
        for k in 0..40 {
            t.reset();
            t.stub(viewer::GS_SYNC_PATH, false, 2, 0, 0x61);
            t.stub(viewer::LOAD_IMAGE_101, false, 2, 0, 0x62);
            t.stub(viewer::LOAD_IMAGE, false, 2, 0, 0x63);
            t.stub(viewer::FRAME_COPY, false, 11, 0, 0x64);
            t.stub(viewer::SYNC_PATH2, false, 1, 0, 0x65);
            let v = new_viewer(t);
            t.w8(v + 0x5_076e, rng.pick(&[0, 1, 0x80]));
            t.w8(v + 0x5_0761, rng.pick(&[0, 1]));
            t.w8(v + 0x5_0762, rng.pick(&[0, 1, 3]));
            t.w8(v + 0x5_075f, rng.pick(&[0, 1]));
            t.w8(v + 0x5_0763, rng.pick(&[0, 1]));
            t.w32(v + 0x5_0534, rng.pick(&[0, 0, 1]));
            t.w32(v + 0x5_0518, rng.pick(&[0, 1, 3, 0xffff_ffff]));
            t.w32(v + 0x5_0514, rng.pick(&[1, 0x20, 0xff]));
            t.w32(v + 0x5_051c, rng.pick(&[0x200, 0x140]));
            t.w32(v + 0x5_0520, rng.pick(&[0x1a0, 0xe0]));
            t.w32(v + 0x5_0524, rng.pick(&[0, 0, 1]));
            t.w32(v + 0x5_06e0, rng.word());
            for o in [0x5_03c0, 0x5_03c4, 0x5_03cc, 0x5_03d0] {
                t.w32(v + o, rng.next());
            }
            let what = format!("frame out {k}");
            t.check(&what, viewer::FRAME_OUT, Conv::Cdecl, &[], |m| {
                Out::U32(viewer::frame_out(m))
            });
        }
    });
}

#[test]
fn viewer_end_and_setup_match_the_original() {
    let mut rng = Rng(0x0077_dab0);
    Twins::each(|t| {
        for k in 0..60 {
            t.reset();
            printf_stub(
                t,
                viewer::DBG_PRINTF,
                &[(viewer::MSG_END, 2), (viewer::MSG_FD, 2)],
                1,
                0x71,
            );
            let seq = |rng: &mut Rng| -> Vec<u32> {
                let mut s: Vec<u32> = (0..rng.below(3))
                    .map(|_| rng.pick(&[1, 2, u32::MAX]))
                    .collect();
                s.push(0);
                s
            };
            t.stub(viewer::AUDIO_STOP, false, 1, 0, 0x72);
            let s = seq(&mut rng);
            t.stub_ex(viewer::SYNC_STR_CHECK_HDD, false, 1, 0, &Ret::Seq(s), &[]);
            t.stub(viewer::FRAME_WAIT, false, 1, 0, 0x73);
            t.stub(viewer::MOVIE_CLOSE, false, 1, 0, 0x74);
            let s = seq(&mut rng);
            t.stub_ex(viewer::SYNC_STR_CHECK, false, 1, 0, &Ret::Seq(s), &[]);
            t.stub(viewer::HDD_STATE_CLEAR, false, 0, 0, 0x75);
            let c: Vec<u32> = (0..3).map(|_| rng.pick(&[0, 1, 10, 2, 5])).collect();
            t.stub_ex(viewer::CD_OPEN_CHECK, false, 1, 0, &Ret::Seq(c), &[]);
            t.stub(viewer::SND_DIGITAL_MODE, false, 1, 0, 0x76);
            t.stub(viewer::TK_SND_UPDATE2, false, 0, 0, 0x77);
            let s = seq(&mut rng);
            t.stub_ex(viewer::SND_STANDBY, false, 1, 0, &Ret::Seq(s), &[]);
            let g = new_viewer(t);
            t.w8(g + 0x5_075c, rng.pick(&[0, 1]));
            let v = if rng.chance(3) { g } else { t.zeros(0x5_0800) };
            t.w32(v + 0x5_0544, rng.pick(&[0, 5, 0xffff_ffff]));
            t.w8(v + 0x5_075d, rng.pick(&[0, 1]));
            t.w32(v + 0x5_0714, rng.pick(&[0, 1]));
            let conv = rng.pick(&[0, 1]);
            let what = format!("mpegstrendsub {k}");
            t.check(&what, viewer::MPEG_STR_END, Conv::Cdecl, &[v, conv], |m| {
                Out::U32(viewer::mpeg_str_end(m, v, conv))
            });
        }
        // a null viewer returns the caller's `eax`
        t.reset();
        t.check(
            "mpegstrendsub null",
            viewer::MPEG_STR_END,
            Conv::Cdecl,
            &[0, 0],
            |m| {
                viewer::mpeg_str_end(m, 0, 0);
                Out::None
            },
        );
        for k in 0..30 {
            t.reset();
            printf_stub(t, viewer::DBG_PRINTF, &[(viewer::MSG_BUFF_END, 4)], 2, 0x81);
            t.stub(viewer::MOVIE_ERR, false, 3, 0, 0x82);
            let v = new_viewer(t);
            let mem = t.zeros(0x1_0000);
            t.w32(v + 0x5_06f4, mem);
            t.w32(v + 0x5_0514, 1 + rng.below(4));
            t.w32(v + 0x5_0518, rng.below(5));
            t.w8(v + 0x5_075f, rng.pick(&[0, 1, 0x80]));
            if k == 0 {
                // MBX 1, MBY 0: the end (guard + 4) lands on the limit
                t.w32(v + 0x5_06f4, 0x01f0_0000 - 0x8b4);
                t.w32(v + 0x5_0514, 1);
                t.w32(v + 0x5_0518, 0);
                t.w8(v + 0x5_075f, 0);
            }
            for o in [0x5_051c, 0x5_0520, 0x5_0524] {
                t.w32(v + o, rng.next());
            }
            let what = format!("alloc buffers {k}");
            t.check(&what, viewer::ALLOC_BUFFERS, Conv::Cdecl, &[], |m| {
                Out::U32(viewer::alloc_buffers(m))
            });
        }
        for k in 0..40 {
            t.reset();
            printf_stub(t, viewer::DBG_PRINTF, &[], 2, 0x91);
            t.stub(viewer::SET_CRT_MODE, false, 2, 0, 0x92);
            t.stub(viewer::SA_WARN, false, 1, 0, 0x93);
            t.stub(viewer::CRT_RESET, false, 0, 0, 0x94);
            t.stub(viewer::CRT_SIZE, false, 2, 0, 0x95);
            t.stub(viewer::ALLOC_BUFFERS, false, 0, 0, 0x96);
            t.stub(viewer::IPU_RESET, false, 0, 0, 0x97);
            t.stub(viewer::SPAD_CALL, false, 4, 0, 0x98);
            let block = t.zeros(0x10);
            t.g32(viewer::BLOCK, block);
            let v = new_viewer(t);
            let mut rec = vec![0u8; 0x10];
            rec[8] = rng.pick(&[0, 1]);
            rec[0xb] = rng.pick(&[0x20, 0x14, 0x28, 0x10]);
            rec[0xc] = rng.pick(&[0x1a, 0x0e, 0x1b]);
            let rec = t.alloc(&rec);
            t.w32(v + 0x5_059c, rec);
            t.w32(v + 0x5_05c0, rng.pick(&[0, 0x1_0000, 0xffff]));
            t.w8(v + 0x5_075f, rng.pick(&[0, 1, 2, 4, 5, 6, 9]));
            t.w8(v + 0x5_0760, rng.pick(&[0, 1, 2, 4, 5, 6]));
            t.w32(v + 0x5_0534, rng.next());
            let what = format!("picture setup {k}");
            t.check(&what, viewer::PICTURE_SETUP, Conv::Cdecl, &[], |m| {
                Out::U32(viewer::picture_setup(m))
            });
        }
    });
}

#[test]
fn dlmalloc_tmalloc_small_matches_the_original() {
    let mut rng = Rng(0x006e_b950);
    Twins::each(|t| {
        for k in 0..200 {
            t.reset();
            let m = t.zeros(0x200);
            let i = rng.below(8);
            t.w32(m + 4, (1 << i) | rng.pick(&[0, 0x100, 0x8000_0000]));
            let nb = if k % 10 == 9 {
                0xffff_ff00
            } else {
                0x40 + 8 * rng.below(8)
            };
            // up to five tree chunks, each in its own 0x400-byte block
            let n = 1 + rng.below(5) as usize;
            let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x400)).collect();
            let mut free_slots: Vec<(usize, u32)> = vec![(0, 0x10), (0, 0x14)];
            let bin = m + 0x130 + 4 * i;
            for (j, &c) in nodes.iter().enumerate() {
                let size = (nb + 8 * rng.below(40)) | rng.pick(&[0, 1, 3]);
                t.w32(c + 4, size);
                t.w32(c + 8, c);
                t.w32(c + 0xc, c);
                t.w32(c + 0x1c, if rng.chance(6) { i ^ 1 } else { i });
                if j == 0 {
                    t.w32(bin, c);
                    t.w32(c + 0x18, if rng.chance(5) { 0 } else { bin });
                } else {
                    let s = rng.below(free_slots.len() as u32) as usize;
                    let (p, off) = free_slots.swap_remove(s);
                    t.w32(nodes[p] + off, c);
                    t.w32(c + 0x18, nodes[p]);
                    free_slots.push((j, 0x10));
                    free_slots.push((j, 0x14));
                }
                if rng.chance(4) {
                    // a same-size chunk on the node's list
                    let d = t.zeros(0x40);
                    t.w32(d + 4, size);
                    t.w32(d + 8, c);
                    t.w32(d + 0xc, c);
                    t.w32(c + 8, d);
                    t.w32(c + 0xc, d);
                }
            }
            let dv = t.zeros(0x20);
            t.w32(m + 8, rng.pick(&[0, 0x18, 0x20, 0x38]));
            t.w32(m + 0x14, dv);
            t.w32(m, rng.pick(&[0, 0xff, 0x8]));
            for b in 0..8 {
                let fd = rng.pick(&[m + 0x1c0, dv, m + 0x28 + 8 * b]);
                t.w32(m + 0x28 + 8 * b + 8, fd);
            }
            let least = match rng.below(7) {
                6 => m + 0x1c0,
                0 => nodes[rng.below(n as u32) as usize],
                1 => 0xffff_0000,
                _ => 0,
            };
            t.w32(m + 0x10, least);
            let what = format!("tmalloc_small {k}");
            t.check(&what, 0x006e_b950, Conv::Cdecl, &[m, nb], |mm| {
                Out::U32(lib::tmalloc_small(mm, m, nb))
            });
        }
    });
}

/// An MSVC `std::map` of `keys` (inserted in that order, unbalanced): its
/// head (nil byte +0xd set) and the nodes; node +0x14 gets `values[i]`.
fn std_map(t: &mut Twins, keys: &[u32], values: &[u32]) -> u32 {
    let head = t.zeros(0x20);
    t.w8(head + 0xd, 1);
    let mut nodes: Vec<(u32, u32)> = Vec::new(); // (key, address)
    let mut root = head;
    for (&k, &v) in keys.iter().zip(values) {
        let n = t.zeros(0x20);
        t.w32(n, head);
        t.w32(n + 8, head);
        t.w32(n + 0x14, v);
        if root == head {
            root = n;
            t.w32(n + 4, head);
        } else {
            let mut p = root;
            loop {
                let pk = nodes.iter().find(|x| x.1 == p).unwrap().0;
                let off = if k < pk { 0 } else { 8 };
                let c: u32 = ffx_bridge::Mem::read(&t.a, p + off);
                if c == head {
                    t.w32(p + off, n);
                    t.w32(n + 4, p);
                    break;
                }
                p = c;
            }
        }
        nodes.push((k, n));
    }
    t.w32(head + 4, root);
    let min = nodes.iter().min_by_key(|x| x.0).map_or(head, |x| x.1);
    let max = nodes.iter().max_by_key(|x| x.0).map_or(head, |x| x.1);
    t.w32(head, min);
    t.w32(head + 8, max);
    head
}

#[test]
fn cluster_cache_clear_matches_the_original() {
    let mut rng = Rng(0x0069_a9c0);
    Twins::each(|t| {
        for k in 0..40 {
            t.reset();
            t.stub_import(chara::WAIT_FOR_SINGLE_OBJECT, 2, 2, 0);
            t.stub_import(chara::RELEASE_MUTEX, 1, 1, 1);
            let n = rng.pick(&[0usize, 1, 2, 3, 5]);
            let cache = t.zeros(0x6500);
            let bufs: Vec<u32> = (0..=n)
                .map(|_| {
                    let w: Vec<u32> = (0..8).map(|_| rng.pick(&[0, 0x7a00, 0x7b00])).collect();
                    t.words(&w)
                })
                .collect();
            t.stub_ex(chara::PHYRE_MALLOC, false, 2, 0, &Ret::Seq(bufs), &[]);
            // PushInto: data = dst, count + 1 (and the "not owned" bit)
            let mut push = vec![0x8b, 0x44, 0x24, 0x04, 0x89, 0x41, 0x04, 0xff, 0x01];
            if rng.chance(3) {
                push.extend([0x81, 0x09]);
                push.extend(0x8000_0000u32.to_le_bytes());
            }
            t.stub_ex(chara::PARRAY_PUSH_INTO, true, 2, 2, &Ret::Fixed(0), &push);
            t.stub(chara::OPERATOR_DELETE, false, 1, 0, 0xc1);
            t.stub(chara::ERASE_SUBTREE, true, 1, 1, 0xc2);
            t.stub(chara::CLUSTER_DTOR, true, 0, 0, 0xc3);
            t.stub(chara::PHYRE_FREE, false, 1, 0, 0xc4);
            let mutex = t.words(&[0x77]);
            t.w32(cache + 0x18, mutex);
            t.w32(cache + 0x641c, rng.pick(&[0, 3, 0x400]));
            let keys: Vec<u32> = (0..n).map(|_| rng.below(100)).collect();
            let values: Vec<u32> = (0..n)
                .map(|_| match rng.below(3) {
                    0 => cache + 0x1c + 0x18 * rng.below(0x400),
                    1 => cache + 0x1c + 0x18 * 0x400,
                    _ => 0x5500 + rng.below(0x100),
                })
                .collect();
            let head = std_map(t, &keys, &values);
            t.w32(cache + 0x10, head);
            t.w32(cache + 0x14, n as u32);
            let what = format!("cluster cache clear {k}");
            t.skip = 0x18;
            t.check_framed(
                &what,
                0x0069_a9c0,
                Conv::Thiscall(cache),
                &[],
                0x20,
                |m, ebp| Out::U32(chara::cluster_cache_clear(m, cache, ebp - 0x20)),
            );
            t.skip = 4;
        }
    });
}

/// [`printf_stub`] for an `fprintf`-like import slot: the format is the
/// second argument.
fn fprintf_import_stub(t: &mut Twins, slot: u32, fmts: &[(u32, u32)], other: u32) {
    let mut c = vec![0x8b, 0x54, 0x24, 0x08]; // mov edx, [esp + 8]
    for &(fmt, n) in fmts {
        let to = t.stub_code(slot, false, n, 0, &Ret::Fixed(0xd0), &[]);
        c.extend([0x81, 0xfa]);
        c.extend(t.rebase(fmt).to_le_bytes()); // cmp edx, fmt
        c.extend([0x75, 7]); // jne next
        c.push(0xb8);
        c.extend(to.to_le_bytes()); // mov eax, to
        c.extend([0xff, 0xe0]); // jmp eax
    }
    let rest = t.stub_code(slot, false, other, 0, &Ret::Fixed(0xd0), &[]);
    c.push(0xb8);
    c.extend(rest.to_le_bytes());
    c.extend([0xff, 0xe0]);
    let at = t.code(&c);
    let s = t.rebase(slot);
    t.patch(s, &at.to_le_bytes());
}

#[test]
fn load_hd_render_data_matches_the_original() {
    let mut rng = Rng(0x0082_a1c0);
    Twins::each(|t| {
        for k in 0..30 {
            t.reset();
            t.stub(chara::MEMSET, false, 3, 0, 0xe0);
            let found = rng.pick(&[0, 0x6600]);
            t.stub_import(chara::STRSTR, 2, 0, found);
            let name = store_arg(1, 0, u32::from_le_bytes(*b"c001"));
            t.stub_ex(
                chara::ANALYSIS_CHR_NUM,
                false,
                2,
                0,
                &Ret::Fixed(0xe1),
                &name,
            );
            t.stub_import(chara::IOB_FUNC, 0, 0, 0x7700);
            fprintf_import_stub(t, chara::FPRINTF, &[(chara::STR_LOAD_CHARACTER, 3)], 2);
            t.stub_import(chara::FFLUSH, 1, 0, 0xe2);
            let dir = store_arg(1, 0, u32::from_le_bytes(*b"/dd\0"));
            t.stub_ex(chara::UNIFY_FILENAME, false, 3, 0, &Ret::Fixed(0xe3), &dir);
            let len = rng.pick(&[2u32, 5, 8]);
            let mut full = store_arg(0, 0, u32::from_le_bytes(*b"a/bc"));
            full.extend(store_arg(0, 4, u32::from_le_bytes(*b"defg")));
            full.extend(store_arg(0, len as u8, 0));
            let sprintf = t.stub_code(chara::SPRINTF, false, 4, 0, &Ret::Fixed(len), &full);
            let s = t.rebase(chara::SPRINTF);
            t.patch(s, &sprintf.to_le_bytes());
            t.stub_import(chara::STRNCPY, 3, 0, 0xe4);
            t.stub(chara::CHARACTER_LOAD, false, 2, 0, rng.pick(&[0, 0xe5]));
            t.stub(chara::TRY_ASYNC_LOAD, false, 2, 0, 0xe6);
            let busy = rng.pick(&[0u32, 3, 50]);
            for i in 0..50 {
                t.g8(chara::CLOTH_LOADS + 0x110 * i + 0x108, u8::from(i < busy));
            }
            let a = rng.pick(&[0, 0x8800]);
            let b = rng.pick(&[0, 0x8900]);
            let path = t.alloc(b"chr/c001/x\0");
            let what = format!("Ch_LoadHdRenderData {k}");
            t.check_framed(
                &what,
                0x0082_a1c0,
                Conv::Cdecl,
                &[path, a, b, 7],
                0x284,
                |m, ebp| Out::U32(chara::load_hd_render_data(m, path, a, b, 7, ebp - 0x284)),
            );
        }
    });
}

#[test]
fn viewer_motion_name_and_sys_motion_match_the_original() {
    let mut rng = Rng(0x0083_71f0);
    Twins::each(|t| {
        for k in 0..60 {
            t.reset();
            t.stub(chara::CREATE_NAME, false, 1, 0, 0x5a00);
            t.stub(chara::CREATE_CHRPATH, false, 1, 0, 0x5b00);
            fprintf_import_stub(t, chara::SPRINTF, &[(chara::FMT_MOTION_TBL, 3)], 4);
            let data = t.zeros(0x200);
            for j in 0..4u32 {
                let id = rng.pick(&[1u16, 2, 7, 0x22]);
                t.write(data + 0x10 + 0x40 * j, &id.to_le_bytes());
            }
            t.stub(chara::PC_READ_D, false, 1, 0, rng.pick(&[0, data, data]));
            t.stub(
                chara::PC_READ_GET_LEN,
                false,
                0,
                0,
                rng.pick(&[0x10, 0x40, 0x90, 0x100, 0x7]),
            );
            t.g8(chara::IS_DEBUG, rng.pick(&[0, 1, 1]));
            let id = rng.pick(&[0x0001u32, 0x0042, 0xf001, 0x8001]);
            for i in 0..50 {
                let e = chara::MOTION_TABLES + 16 * i;
                let used = u32::from(rng.chance(if k % 4 == 0 { 1 } else { 3 }));
                t.g32(e + 4, used);
                t.g32(e + 0xc, if rng.chance(6) { id } else { 0x99 });
                t.g32(e, rng.pick(&[0, data]));
                t.g32(e + 8, rng.pick(&[0, 2, 4, 0xffff_ffff]));
            }
            let key = id << 16 | rng.pick(&[1, 2, 7, 0x22, 5]);
            let what = format!("motion name {k}");
            t.check_framed(&what, 0x0083_71f0, Conv::Cdecl, &[key], 0x44, |m, ebp| {
                Out::U32(chara::motion_name(m, key, ebp - 0x44))
            });
        }
        for k in 0..80 {
            t.reset();
            t.stub(chara::TEXTURE_SET_ANIM, false, 3, 0, 0x61);
            t.stub(chara::SET_SEQUENCE, false, 2, 0, 0x62);
            let row: Vec<u32> = (0..0x20).map(|_| rng.pick(&[0, 0, 0, 0x300])).collect();
            let row = t.words(&row);
            let tbl = t.zeros(0x40);
            for s in 0..4 {
                t.w32(tbl + 0x10 + 8 * s, if rng.chance(5) { 0 } else { row });
            }
            let pairs: Vec<u32> = (0..6)
                .map(|_| rng.below(0x14) | rng.below(0x100) << 16)
                .collect();
            let pairs = t.words(&pairs);
            let list = t.zeros(0x40);
            for s in 0..4 {
                t.w32(list + 8 * s + 8, rng.pick(&[0, 3, 6]));
                t.w32(list + 8 * s + 0xc, pairs);
            }
            let def = t.zeros(0x120);
            t.w32(def, rng.below(0x100));
            t.w8(def + 0x70, rng.pick(&[0, 1]));
            t.w32(def + 0x6c, tbl);
            t.w32(def + 0x108, if rng.chance(6) { 0 } else { list });
            let chr = t.zeros(0x200);
            t.w32(chr + 0x1c4, def);
            t.w32(chr + 0x198, rng.pick(&[0, 0x200]));
            let set = rng.below(3);
            t.g8(chara::SYS_MOTION_SETS + set, rng.below(4) as u8);
            t.g32(chara::SEARCH_COUNT, rng.pick(&[0, 0, 5]));
            let max = rng.pick(&[0u32, 3, 0x13, 0x1f]);
            let idx = rng.pick(&[0u32, 1, 0x10, max]);
            let dir = rng.pick(&[0, 1]);
            let set = if rng.chance(4) { 1 } else { set };
            let what = format!("Ch_SetSysMotion_ViewerAnimation {k}");
            t.check(
                &what,
                0x0083_7e10,
                Conv::Cdecl,
                &[chr, set, idx, max, dir],
                |m| Out::U32(chara::set_sys_motion_viewer(m, chr, set, idx, max, dir)),
            );
        }
    });
}

#[test]
fn anim_window_and_motion_viewer_match_the_original() {
    let mut rng = Rng(0x0093_e370);
    Twins::each(|t| {
        for k in 0..40 {
            t.reset();
            let st = t.words(&[0x4400, rng.pick(&[0, 3, 0xffff_ffff])]);
            let fresh = t.zeros(8);
            let msg = rng.pick(&[0x8001u32, 0x8003, 0x8007, 0x8007, 0x8002]);
            let id = rng.pick(&[0u32, 5, 0x1f, 0x20]);
            // a button press without state reads [0] in the original
            let arg = if msg == 0x8007 && id < 0x20 {
                st
            } else {
                rng.pick(&[st, st, 0])
            };
            t.stub(chara::WINDOW_GET_ARG, false, 1, 0, arg);
            t.stub(chara::USER_MALLOC, false, 1, 0, fresh);
            for (s, n) in [
                (chara::WINDOW_SET_ARG, 2),
                (chara::WINDOW_SET_TITLE, 2),
                (chara::WINDOW_ADD_BUTTON, 7),
                (chara::BUTTON_SET_TOGGLE, 3),
                (chara::BUTTON_SET_TOGGLE_VALUE, 3),
                (chara::WINDOW_CLOSE_TREE, 1),
                (chara::TEX_ANIM_NO_SET, 5),
                (chara::USER_FREE, 1),
                (chara::GET_NAME, 1),
            ] {
                t.stub(s, false, n, 0, 0x70 + n);
            }
            t.stub(chara::TEXTURE_GET_ANIM_ID, false, 1, 0, 0x7a);
            t.stub(
                chara::CHECK_HANDLE,
                false,
                1,
                0,
                rng.pick(&[0, 1, 0xffff_ffff]),
            );
            printf_stub(t, chara::DBG_PRINTF, &[], 2, 0x7b);
            fprintf_import_stub(t, chara::SPRINTF, &[], 3);
            t.g32(chara::ANIM_WIN_CHR, 0x4500);
            let what = format!("anim window proc {k}");
            t.check_framed(
                &what,
                0x0093_e370,
                Conv::Cdecl,
                &[0x9900, msg, id],
                0x84,
                |m, ebp| Out::U32(chara::anim_window_proc(m, 0x9900, msg, id, ebp - 0x84)),
            );
        }
        for k in 0..40 {
            t.reset();
            printf_stub(t, chara::DBG_PRINTF, &[(chara::FMT_MTYPE, 4)], 1, 0x81);
            fprintf_import_stub(t, chara::SPRINTF, &[(chara::FMT_ANM_PROJ, 5)], 4);
            let data = t.zeros(0x10);
            t.write(data, &rng.pick(&[0x12u16, 0x34]).to_le_bytes());
            t.write(data + 8, &rng.pick(&[0u16, 1, 2, 0x100]).to_le_bytes());
            t.stub(chara::PC_READ, false, 1, 0, rng.pick(&[0, data, data]));
            t.stub(chara::USER_FREE, false, 1, 0, 0x82);
            t.stub_import(chara::STRDUP, 1, 0, 0x8300);
            t.stub(chara::SET_MOTION_SEQ, false, 5, 0, 0x84);
            let n = rng.pick(&[0u32, 3, 0x22]);
            t.stub_import(chara::ATOI, 1, 0, n);
            let db = rng.pick(&[0u32, 1, 2]);
            t.stub(chara::GET_DB_NUM, false, 1, 0, db);
            let key = n | db << 12;
            let entries: Vec<u32> = (0..4)
                .map(|_| {
                    let v = rng.pick(&[key, key + 1, 0x8000 | key]);
                    t.words(&[v])
                })
                .collect();
            let list = t.words(&entries);
            let def = t.zeros(0x120);
            t.w8(def + 0xf8, u8::from(rng.chance(6)));
            t.w32(def + 0x100, rng.pick(&[0, 0x9100]));
            t.w32(def + 0x114, if rng.chance(5) { 0 } else { list });
            t.write(def + 0x110, &rng.pick(&[0u16, 2, 4, 0xffff]).to_le_bytes());
            let chr = t.zeros(0x810);
            t.write(chr, &rng.pick(&[0xf001u16, 0x0001, 0x7001]).to_le_bytes());
            t.w32(chr + 0x1c4, def);
            t.w32(chr + 0x194, rng.next());
            let sub = t.alloc(b"m012\0");
            let what = format!("Ch_ReadMotionViewer {k}");
            t.check_framed(
                &what,
                0x0082_83c0,
                Conv::Cdecl,
                &[chr, 0x6600, sub, 0x6700],
                0x84,
                |m, ebp| {
                    Out::U32(chara::read_motion_viewer(
                        m,
                        chr,
                        0x6600,
                        sub,
                        0x6700,
                        ebp - 0x84,
                    ))
                },
            );
        }
    });
}

// ---- the page heap ----

use ffx_sg::m5main4_pages::bridge as pages;

/// Critical section imports as plain stubs.
fn cs_stubs(t: &mut Twins) {
    for s in [
        pages::INITIALIZE_CS,
        pages::ENTER_CS,
        pages::LEAVE_CS,
        pages::DELETE_CS,
    ] {
        t.stub_import(s, 1, 1, 0xcc);
    }
}

/// The used blocks' memory addresses in the pool's pages (from `a`).
fn used_blocks(t: &Twins, pool: u32) -> Vec<u32> {
    let rd = |a: u32| -> u32 { ffx_bridge::Mem::read(&t.a, a) };
    let size = rd(pool + 4);
    let mut page = rd(pool);
    let mut out = Vec::new();
    while page != 0 {
        let end = page + size - 0x10;
        let mut b = page + 0x24;
        while b < end && rd(b + 8) != 0 {
            if rd(b) == 0 && rd(b + 4) == 0 {
                out.push(b + 0x10);
            }
            b += rd(b + 8);
        }
        page = rd(page + 0x10);
    }
    out
}

#[test]
fn page_pool_matches_the_original() {
    let mut rng = Rng(0x0094_3a50);
    Twins::each(|t| {
        for round in 0..6 {
            t.reset();
            let size = rng.pick(&[0x200u32, 0x400, 0x1000]);
            let mut seq: Vec<u32> = (0..64).map(|_| t.zeros(size as usize)).collect();
            if round % 3 == 2 {
                seq[3] = 0;
            }
            t.stub_ex(pages::PAGE_ALLOC, false, 2, 0, &Ret::Seq(seq), &[]);
            let commit = if round % 3 == 1 {
                vec![1, 1, 0x100, 1]
            } else {
                vec![1]
            };
            t.stub_ex(pages::PAGE_COMMIT, false, 2, 0, &Ret::Seq(commit), &[]);
            t.stub(pages::PAGE_QUEUE_DECOMMIT, false, 2, 0, 0xd1);
            t.stub(pages::PAGE_RELEASE, false, 1, 0, 0xd2);
            let pool = t.words(&[0, size]);
            for k in 0..60 {
                let live = used_blocks(t, pool);
                if live.is_empty() || rng.chance(2) {
                    // at most what a fresh page holds: more grows the pool forever
                    let n = rng.pick(&[0u32, 1, 8, 0x20, 0x50, size / 3, size - 0x50, size - 0x44]);
                    let what = format!("pool alloc {round} {k}");
                    t.check(&what, pages::POOL_ALLOC, Conv::Thiscall(pool), &[n], |m| {
                        Out::U32(pages::pool_alloc(m, pool, n))
                    });
                } else {
                    let p = live[rng.below(live.len() as u32) as usize];
                    let what = format!("pool free {round} {k}");
                    t.check(&what, pages::POOL_FREE, Conv::Thiscall(pool), &[p], |m| {
                        Out::U32(pages::pool_free(m, pool, p))
                    });
                }
            }
            if !used_blocks(t, pool).is_empty() {
                let what = format!("pool free all {round}");
                t.check(
                    &what,
                    pages::POOL_FREE_ALL,
                    Conv::Thiscall(pool),
                    &[],
                    |m| {
                        pages::pool_free_all(m, pool);
                        Out::None
                    },
                );
            }
            for commit in [0u32, 1, 0x100] {
                t.reset();
                let page = t.zeros(size as usize);
                t.stub(
                    pages::PAGE_ALLOC,
                    false,
                    2,
                    0,
                    if rng.chance(4) { 0 } else { page },
                );
                t.stub(pages::PAGE_COMMIT, false, 2, 0, commit);
                let pool = t.words(&[0, size]);
                t.check(
                    "new page",
                    pages::NEW_PAGE,
                    Conv::Thiscall(pool),
                    &[],
                    |m| Out::U32(pages::new_page(m, pool)),
                );
            }
        }
    });
}

/// A stub extra: on the stub's `n`-th call, `mov dword [ecx + off], 0`.
fn zero_this_field_on_call(t: &mut Twins, n: u32, off: u8) -> Vec<u8> {
    let ctr = t.words(&[n]);
    let mut c = vec![0xff, 0x0d];
    c.extend(ctr.to_le_bytes()); // dec dword [ctr]
    c.extend([0x75, 0x07]); // jnz +7
    c.extend([0xc7, 0x41, off]);
    c.extend(0u32.to_le_bytes()); // mov dword [ecx + off], 0
    c
}

#[test]
fn page_heap_teardown_matches_the_original() {
    let mut rng = Rng(0x0094_2a80);
    Twins::each(|t| {
        for k in 0..30 {
            t.reset();
            // an arena: blocks of 0x20 from +0x1010, tags 2 (used) or 1,
            // the last one (tag 2) at the arena's end - 0x10
            let blocks = 1 + rng.below(6);
            let arena_size = 0x1010 + 0x20 * blocks + 0x10;
            let arena = t.zeros(arena_size as usize + 0x20);
            for i in 0..blocks {
                let b = arena + 0x1010 + 0x20 * i;
                t.w32(b + 4, 0x20);
                t.w32(b + 0xc, rng.pick(&[1, 2, 2, 3]));
            }
            let end = arena + arena_size - 0x10;
            t.w32(end + 4, 0x10);
            t.w32(end + 0xc, 2);
            let heap = t.words(&[arena_size, 0, 0, arena]);
            let frees = 1 + rng.below(8);
            let stop = zero_this_field_on_call(t, frees, 0xc);
            t.stub_ex(pages::USER_HEAP_FREE, true, 1, 1, &Ret::Fixed(1), &stop);
            // the arena keeps its blocks until the heap drops it: a heap
            // whose blocks are all free (no tag 2 before the end) would
            // spin forever, so give it at least one
            t.w32(arena + 0x1010 + 0xc, 2);
            let what = format!("heap free all {k}");
            t.check(
                &what,
                pages::HEAP_FREE_ALL,
                Conv::Thiscall(heap),
                &[],
                |m| {
                    pages::heap_free_all(m, heap);
                    Out::None
                },
            );
        }
        for k in 0..4 {
            t.reset();
            cs_stubs(t);
            t.stub(pages::HEAP_FREE_ALL, true, 0, 0, 0xe1);
            t.stub(pages::POOL_FREE_ALL, true, 0, 0, 0xe2);
            let cs = t.zeros(0x40);
            let what = format!("heap destroy {k}");
            t.check(&what, 0x0094_2630, Conv::Thiscall(cs), &[], |m| {
                Out::U32(pages::heap_destroy(m, cs))
            });
            t.check(&what, 0x0094_2680, Conv::Thiscall(cs), &[], |m| {
                Out::U32(pages::pool_destroy(m, cs))
            });
        }
    });
}

#[test]
fn page_allocators_match_the_original() {
    let mut rng = Rng(0x0094_4080);
    Twins::each(|t| {
        for k in 0..30 {
            t.reset();
            cs_stubs(t);
            let table = t.zeros(0x10);
            t.stub(pages::POOL_ALLOC, true, 1, 1, table);
            let a = t.zeros(0x40);
            let size = rng.pick(&[1u32, 0x10, 0x30, 0x1000, 0x2000, 0xffff_fff0]);
            let align = rng.pick(&[0x10u32, 0x1000, 0x10000, 0xffff_0000]);
            let what = format!("allocator init {k}");
            t.check(
                &what,
                0x0094_4080,
                Conv::Thiscall(a),
                &[size, align, 0x7700],
                |m| Out::U32(pages::alloc_init(m, a, size, align, 0x7700)),
            );
        }
        for k in 0..30 {
            t.reset();
            cs_stubs(t);
            // CHUNK_REMOVE: the list head moves to the chunk's next
            let pop = vec![0x8b, 0x44, 0x24, 0x04, 0x8b, 0x40, 0x18, 0x89, 0x41, 0x2c];
            t.stub_ex(pages::CHUNK_REMOVE, true, 1, 1, &Ret::Fixed(0xf1), &pop);
            t.stub(pages::POOL_FREE, true, 1, 1, 0xf2);
            let mut next = 0;
            for _ in 0..rng.below(3) {
                let c = t.zeros(0x20);
                t.w32(c + 0x18, next);
                next = c;
            }
            let mut list = 0;
            for _ in 0..rng.below(3) {
                list = t.words(&[list, 0]);
            }
            let a = t.zeros(0x40);
            t.w32(a + 0x2c, next);
            t.w32(a + 0x34, 0x6600);
            t.w32(a + 0x38, list);
            t.w32(a + 0x3c, 0x7700);
            let what = format!("allocator destroy {k}");
            t.check(&what, 0x0094_4190, Conv::Thiscall(a), &[], |m| {
                Out::U32(pages::alloc_destroy(m, a))
            });
        }
        for k in 0..40 {
            t.reset();
            cs_stubs(t);
            t.stub(pages::PAGE_QUEUE_DECOMMIT, false, 2, 0, 0xa1);
            t.stub(pages::PAGE_RELEASE, false, 1, 0, 0xa2);
            t.stub(pages::BUCKET_LOCKED_CALL, true, 0, 0, 0xa3);
            t.stub(pages::POOL_FREE, true, 1, 1, 0xa4);
            let n = 1 + rng.below(3);
            let chunks: Vec<u32> = (0..n).map(|_| t.zeros(0x20)).collect();
            for (i, &c) in chunks.iter().enumerate() {
                let next = chunks.get(i + 1).copied().unwrap_or(0);
                t.w32(c + 0x18, next);
                t.w32(c, 0x1_0000 * (i as u32 + 1));
                let bits: Vec<u32> = (0..4)
                    .map(|_| rng.pick(&[0, 1, 0x8000_0000, 0x10]))
                    .collect();
                let bits = t.words(&bits);
                t.w32(c + 8, bits);
            }
            let table: Vec<u32> = (0..=n).map(|i| 0x1_0000 * (n - i + 1)).collect();
            let table = t.words(&table);
            let a = t.zeros(0x40);
            t.w32(a + 0x2c, chunks[0]);
            t.w32(a + 0x34, table);
            // no items per page loops forever on a chunk with items
            let count = rng.pick(&[0, 0x20, 0x40, 0x80]);
            t.w32(a + 0x24, count);
            t.w32(
                a + 0x28,
                if count == 0 {
                    rng.pick(&[0, 8])
                } else {
                    rng.pick(&[8, 0x20, 1, 3, 0x18])
                },
            );
            t.w32(a + 0x1c, 0x1000);
            t.w32(a + 0x3c, 0x7700);
            let chunk = chunks[rng.below(n) as usize];
            let what = format!("chunk remove {k}");
            t.check(
                &what,
                pages::CHUNK_REMOVE,
                Conv::Thiscall(a),
                &[chunk],
                |m| Out::U32(pages::chunk_remove(m, a, chunk)),
            );
        }
    });
}

#[test]
fn page_big_table_matches_the_original() {
    let mut rng = Rng(0x0094_4f80);
    Twins::each(|t| {
        for k in 0..8 {
            t.reset();
            let it = t.zeros(0xc);
            let table = t.words(&[rng.pick(&[0, 4]), 0x5500]);
            let what = format!("iterator {k}");
            t.check(&what, 0x0094_4b40, Conv::Thiscall(it), &[table], |m| {
                Out::U32(pages::iter_init(m, it, table))
            });
        }
        for k in 0..20 {
            t.reset();
            t.stub(pages::VIRTUAL_DECOMMIT, false, 2, 0, 0xb1);
            t.stub_ex(
                pages::VIRTUAL_RELEASE,
                false,
                2,
                0,
                &Ret::Seq(vec![0xb2, 0xb3, 0xb4]),
                &[],
            );
            let mut next = 0;
            for _ in 0..rng.below(3) {
                next = t.words(&[next, 0]);
            }
            let tb = t.words(&[
                rng.pick(&[0, 4, 0x400, 0x401]),
                0x5500,
                3,
                0,
                next,
                7,
                0x10_0000,
            ]);
            let what = format!("table clear {k}");
            t.check(&what, pages::TABLE_CLEAR, Conv::Thiscall(tb), &[], |m| {
                Out::U32(pages::table_clear(m, tb))
            });
        }
        for k in 0..30 {
            t.reset();
            cs_stubs(t);
            let found = rng.pick(&[0u32, 1, 0x100]);
            let size = store_arg(1, 0, rng.pick(&[0, 1, 0x1000, 0x1001]));
            t.stub_ex(pages::HASH_FIND, true, 2, 2, &Ret::Fixed(found), &size);
            t.stub(pages::PAGE_QUEUE_DECOMMIT, false, 2, 0, 0xc1);
            t.stub(pages::PAGE_RELEASE, false, 1, 0, 0xc2);
            t.stub(pages::HASH_REMOVE, true, 1, 1, 0xc3);
            let big = t.zeros(0x40);
            let p = rng.next();
            let what = format!("big free {k}");
            t.skip = 0xc;
            t.check_framed(
                &what,
                pages::BIG_FREE,
                Conv::Thiscall(big),
                &[p],
                0x1c,
                |m, ebp| Out::U32(pages::big_free(m, big, p, ebp - 0x1c)),
            );
            t.skip = 4;
        }
        for k in 0..30 {
            t.reset();
            cs_stubs(t);
            t.stub(pages::BIG_FREE, true, 1, 1, 0xd1);
            t.stub(pages::TABLE_CLEAR, true, 0, 0, 0xd2);
            let n = rng.pick(&[0u32, 1, 4, 8]);
            let slots: Vec<u32> = (0..n)
                .map(|_| {
                    if rng.chance(2) {
                        0
                    } else {
                        t.words(&[rng.next()])
                    }
                })
                .collect();
            let slots = t.words(&[slots, vec![0]].concat());
            let big = t.zeros(0x40);
            t.w32(big + 0x18, n);
            t.w32(big + 0x1c, slots);
            let what = format!("big destroy {k}");
            t.check(&what, 0x0094_4ba0, Conv::Thiscall(big), &[], |m| {
                Out::U32(pages::big_destroy(m, big))
            });
        }
        for set in [0u32, 1, 0x1234] {
            t.reset();
            t.stub(pages::MEM_PRE_INIT, false, 0, 0, 0xe1);
            t.stub(pages::DESC_INIT, true, 0, 0, 0xe2);
            t.stub(pages::SYSTEM_INIT, true, 1, 1, 0xe3);
            t.stub(pages::START_THREAD, false, 0, 0, 0xe4);
            t.g32(pages::MEM_SYSTEM, set);
            let what = format!("mem system ensure {set}");
            t.skip = 0xc;
            t.check_framed(&what, 0x0094_28b0, Conv::Cdecl, &[], 0x44, |m, ebp| {
                pages::mem_system_ensure(m, ebp - 0x44);
                Out::None
            });
            t.skip = 4;
        }
    });
}

#[test]
fn page_hash_matches_its_formula() {
    // the formula against the original's own words on a few keys is in
    // big_free's test (the key's hash word); here only fixed points
    assert_eq!(pages::hash(0), {
        let a = (0x3du32).wrapping_mul(9);
        let b = (a >> 4 ^ a).wrapping_mul(0x27d4_eb2d);
        b >> 15 ^ b
    });
}

#[test]
fn camera_point_orbit_matches_the_original() {
    let mut rng = Rng(0x0094_2520);
    Twins::each(|t| {
        for k in 0..30 {
            t.reset();
            t.stub(misc::GET_ANGLE, false, 3, 0, 0x31);
            t.stub(misc::SET_RECT, false, 3, 0, 0x32);
            t.stub(misc::SET_POLAR, false, 5, 0, 0x33);
            let list = rng.below(2);
            t.g32(misc::CUR_LIST, list);
            let n = rng.pick(&[0u32, 1, 3, 0xffff_ffff]);
            t.g32(misc::LIST_COUNT + 4 * list, n);
            let flags: Vec<u32> = (0..9).map(|_| rng.pick(&[0, 0, 2, 0x10, 0xff])).collect();
            let flags = t.words(&flags);
            let pts: Vec<u32> = (0..12)
                .map(|_| (rng.below(2000) as f32 - 1000.0).to_bits())
                .collect();
            let pts = t.words(&pts);
            t.g32(misc::LIST_FLAGS + 4 * list, flags);
            t.g32(misc::LIST_POINTS + 4 * list, pts);
            for g in [misc::YAW, misc::PITCH, misc::DISTANCE] {
                t.g32(g, (rng.below(700) as f32 / 100.0 - 3.5).to_bits());
            }
            let what = format!("camera point orbit {k}");
            t.check_framed(&what, 0x0094_2520, Conv::Cdecl, &[], 0x14, |m, ebp| {
                Out::U32(misc::point_orbit(m, ebp - 0x14))
            });
        }
    });
}

// ---- the CD emulation's channel reads ----

use ffx_sg::m5main4_cd::bridge as cdr;

/// The stubs every channel read calls; `find` the ex-file lookups in
/// turn.
fn cd_stubs(t: &mut Twins, rng: &mut Rng, cd: u32, find: Vec<u32>) {
    t.stub(cdr::GET_CD, false, 0, 0, cd);
    t.stub(cdr::GET_READ_BUFFER, false, 0, 0, 0x6000);
    t.stub(cdr::CHANNEL_CLEAR, false, 1, 0, 0x11);
    t.stub_ex(cdr::HDD_INSTALL_FIND, false, 2, 0, &Ret::Seq(find), &[]);
    t.stub_import(cdr::SPRINTF, 3, 0, 0x12);
    t.stub(cdr::SCE_OPEN, false, 2, 0, rng.pick(&[0, 5]));
    t.stub(cdr::DBG_PRINTF, false, 2, 0, 0x13);
    let busy: Vec<u32> = (0..rng.below(3)).map(|i| i + 1).chain([0]).collect();
    let io = seq_store(
        t,
        2,
        &[busy.clone(), busy.clone(), busy.clone(), busy].concat(),
    );
    t.stub_ex(cdr::SCE_IOCTL, false, 3, 0, &Ret::Fixed(0x14), &io);
    // task_wait clears the background decode's busy byte
    let clear = m5_main3_common::set_byte(t.rebase(cdr::BG_BUSY), 0);
    t.stub_ex(cdr::TASK_WAIT, false, 1, 0, &Ret::Fixed(0x15), &clear);
    for (s, n) in [
        (cdr::SCE_LSEEK, 3),
        (cdr::FLUSH_CACHE, 1),
        (cdr::SCE_READ, 3),
        (cdr::SCE_CLOSE, 1),
        (cdr::READ_CALLBACK, 1),
        (cdr::LOADING_PUSH, 1),
        (cdr::CD_READ_TASK, 5),
        (cdr::MEMCPY, 3),
        (cdr::SIGNAL_BG, 0),
        (cdr::DECODE_DATA, 1),
        (cdr::OPEN_CHECK, 2),
    ] {
        t.stub(s, false, n, 0, 0x20 + n);
    }
    let cfg = t.zeros(0x20);
    t.w8(cfg + 0x10, rng.pick(&[0, 1]));
    t.stub(cdr::GET_CONFIG, false, 0, 0, cfg);
    t.g8(cdr::HDD_MODE, rng.pick(&[0, 1, 1]));
    t.g8(cdr::BG_BUSY, rng.pick(&[0, 1]));
}

/// Channel `i` set up at random (active, busy, cancelled, packed, sizes).
fn cd_channel(t: &mut Twins, rng: &mut Rng, i: u32) {
    let ch = cdr::CHANNELS + 0x38 * i;
    t.write(t.rebase(ch), &[0u8; 0x38]);
    t.g32(
        ch,
        rng.pick(&[
            0x10,
            0x7fb,
            0x7fc,
            0x800,
            0x1000,
            0x2345,
            0xffff_fff0,
            0xffff_e000,
        ]),
    );
    t.g32(ch + 4, 0x7000);
    t.g32(ch + 8, 3);
    t.g32(ch + 0x1c, rng.pick(&[0, 0x40]));
    t.g32(ch + 0x20, rng.pick(&[0, 0x30]));
    t.g32(ch + 0x28, 0x99);
    t.g32(
        ch + 0x2c,
        rng.pick(&[
            0,
            0x10,
            0x800,
            0x1000,
            0x2345,
            0xffff_f000,
            0x1400,
            0xffff_f800,
        ]),
    );
    t.g8(ch + 0x31, u8::from(!rng.chance(4)));
    t.g8(ch + 0x32, u8::from(rng.chance(6)));
    t.g8(ch + 0x33, u8::from(rng.chance(8)));
    t.g8(ch + 0x34, rng.pick(&[0, 1]));
}

/// The I/O status word (`ebp - 4`) of both oracles after a framed check.
fn same_io_word(t: &Twins, what: &str, ebp: u32) {
    let a: u32 = ffx_bridge::Mem::read(&t.a, ebp - 4);
    let b: u32 = ffx_bridge::Mem::read(&t.b, ebp - 4);
    assert_eq!(a, b, "{what}: the I/O status word");
}

#[test]
fn cd_channel_reads_match_the_original() {
    let mut rng = Rng(0x00a4_32a0);
    Twins::each(|t| {
        for k in 0..20 {
            t.reset();
            let cd = t.zeros(0x80);
            let e = t.zeros(0x40);
            let find: Vec<u32> = (0..64).map(|_| if rng.chance(4) { e } else { 0 }).collect();
            cd_stubs(t, &mut rng, cd, find);
            for i in 0..64 {
                cd_channel(t, &mut rng, i);
            }
            let what = format!("sa_HddExFileRead {k}");
            t.skip = 0xc;
            t.check_framed(&what, 0x00a4_32a0, Conv::Cdecl, &[], 0x10, |m, ebp| {
                Out::U32(cdr::hdd_ex_file_read(
                    m,
                    cdr::Frame {
                        io: ebp - 4,
                        cd: ebp - 0x10,
                    },
                ))
            });
            t.skip = 4;
            same_io_word(t, &what, t.frame_ebp(0));
        }
        for k in 0..120 {
            t.reset();
            let cd = t.zeros(0x80);
            let e = t.zeros(0x40);
            let find = vec![if rng.chance(4) { e } else { 0 }];
            cd_stubs(t, &mut rng, cd, find);
            let i = rng.below(64);
            cd_channel(t, &mut rng, i);
            let what = format!("channel task {k}");
            t.skip = 8;
            t.check_framed(&what, 0x00a4_3740, Conv::Cdecl, &[i], 0xc, |m, ebp| {
                Out::U32(cdr::channel_task(
                    m,
                    i,
                    cdr::Frame {
                        io: ebp - 4,
                        cd: ebp - 0xc,
                    },
                ))
            });
            t.skip = 4;
            same_io_word(t, &what, t.frame_ebp(1));
        }
    });
}

#[test]
fn viewer_movie_start_matches_the_original() {
    let mut rng = Rng(0x0077_cfa0);
    Twins::each(|t| {
        for k in 0..40 {
            t.reset();
            printf_stub(
                t,
                viewer::DBG_PRINTF,
                &[(viewer::MSG_DAT, 3), (viewer::MSG_SECTOR, 2)],
                2,
                0x41,
            );
            t.stub(viewer::MOVIE_CD_READ, false, 4, 0, 1);
            t.stub(
                viewer::HDD_FILE_OPEN,
                false,
                2,
                0,
                rng.pick(&[3, 0xffff_ffff]),
            );
            t.stub(viewer::MOVIE_ERR, false, 3, 0, 0x42);
            t.stub(viewer::SCE_CLOSE, false, 1, 0, 0x43);
            t.stub(viewer::READ_SIZE_FILE, false, 3, 0, 1);
            t.stub(viewer::FLUSH_CACHE, false, 1, 0, 0x44);
            // an open failure waits forever: not run
            t.stub(viewer::MOVIE_OPEN_CD, false, 1, 0, 7);
            t.stub(viewer::MOVIE_OPEN, false, 2, 0, 8);
            t.stub(viewer::TAKE_SIZES, false, 1, 0, rng.pick(&[0x100, 0x800]));
            t.stub(viewer::MOVIE_READ, false, 5, 0, 0x45);
            t.stub(viewer::STR_SELECT, false, 3, 0, 0x46);
            let v = new_viewer(t);
            for i in 0..0x80 {
                t.w8(v + 0x1_0080 + i, rng.next() as u8);
            }
            t.w8(v + 0x5_075c, rng.pick(&[0, 1]));
            t.w8(v + 0x5_075d, rng.pick(&[0, 1]));
            t.w8(v + 0x5_0764, rng.pick(&[0, 2, 0x80]));
            t.w32(v + 0x5_06ec, rng.below(0x1000));
            t.w32(v + 0x5_06f0, rng.below(0x10000));
            let ring = rng.pick(&[1u32, 2, 4, 0xffff_ffff]);
            t.w32(v + 0x5_0754, ring);
            t.w32(v + 0x5_053c, rng.below(2));
            t.w32(v + 0x5_0548, rng.pick(&[0, 5]));
            for s in 0..4 {
                t.w32(v + 0x5_0560 + 4 * s, 0x9000 + 0x100 * s);
                t.w32(v + 0x5_054c + 4 * s, rng.pick(&[0, 5]));
            }
            let what = format!("movie start {k}");
            t.check(
                &what,
                viewer::MOVIE_START_SYM,
                Conv::Cdecl,
                &[0x5100, 0x5200],
                |m| Out::U32(viewer::movie_start(m, 0x5100, 0x5200)),
            );
        }
    });
}

#[test]
fn viewer_init_matches_the_original() {
    let mut rng = Rng(0x0077_d280);
    Twins::each(|t| {
        for k in 0..20 {
            t.reset();
            t.stub(viewer::GET_VIDEO_MODE, false, 0, 0, rng.pick(&[0, 1, 2]));
            let v: Vec<u8> = (0..0x5_1000).map(|_| rng.next() as u8).collect();
            let v = t.alloc(&v);
            let w = rng.pick(&[0u32, 0x200, 0x1f, 0xffff_fff1]);
            let hh = rng.pick(&[0u32, 0x1a0, 0x11]);
            let big = rng.pick(&[0, 1]);
            let what = format!("viewer init {k}");
            t.check(
                &what,
                viewer::VIEWER_INIT_SYM,
                Conv::Cdecl,
                &[v, w, hh, big],
                |m| Out::U32(viewer::viewer_init(m, v, w, hh, big)),
            );
        }
    });
}

#[test]
fn viewer_adpcm_decode_matches_the_original() {
    let mut rng = Rng(0x0077_a930);
    Twins::each(|t| {
        for k in 0..40 {
            t.reset();
            let mut src = Vec::new();
            for _ in 0..16 {
                let filter: u8 = rng.pick(&[0, 1, 2, 3, 4, 0xf]);
                let shift = rng.below(16) as u8;
                src.push(filter << 4 | shift);
                src.extend((0..8).map(|_| rng.next() as u8));
            }
            let src = t.alloc(&src);
            let out = t.zeros(0x200);
            let state = t.words(&[rng.word(), rng.word()]);
            let what = format!("adpcm decode {k}");
            t.check(
                &what,
                viewer::ADPCM_DECODE_SYM,
                Conv::Cdecl,
                &[src, out, state],
                |m| Out::U32(viewer::adpcm_decode(m, src, out, state)),
            );
        }
    });
}

#[test]
fn id_draw_lines_matches_the_original() {
    let mut rng = Rng(0x0084_4490);
    Twins::each(|t| {
        for k in 0..40 {
            t.reset();
            t.stub(misc::ID_EXIST_SHADE, false, 0, 0, rng.pick(&[0, 1, 0x100]));
            t.stub(misc::ID_DRAW_TRIANGLES, false, 5, 0, 0x91);
            let n = rng.pick(&[0u16, 1, 3, 6]);
            let mut tris = Vec::new();
            for _ in 0..n {
                for _ in 0..3 {
                    tris.extend((rng.below(8) as u16).to_le_bytes());
                }
                tris.extend([0u8; 6]);
                tris.extend(rng.next().to_le_bytes());
            }
            tris.extend([0u8; 16]);
            let tris = t.alloc(&tris);
            t.stub(misc::GET_PTR32, false, 1, 0, tris);
            let proj: Vec<u32> = (0..32).map(|_| u32::from(rng.chance(6))).collect();
            t.stub_ex(misc::ID_PROJECT, false, 3, 0, &Ret::Seq(proj), &[]);
            let verts: Vec<u8> = (0..64).map(|_| rng.next() as u8).collect();
            let verts = t.alloc(&verts);
            let model = t.zeros(0x10);
            t.write(model + 8, &n.to_le_bytes());
            t.w32(model + 0xc, 0x4321);
            let packet = t.zeros(0x400);
            t.w32(packet + 0x30, rng.next());
            t.g32(misc::ID_MODE, rng.pick(&[0, 2, 3, 3, 5]));
            let what = format!("id draw lines {k}");
            t.check_framed(
                &what,
                misc::ID_DRAW_LINES_SYM,
                Conv::Cdecl,
                &[verts, model, 0x5300, packet, 0x77],
                0x34,
                |m, ebp| {
                    Out::U32(misc::id_draw_lines(
                        m,
                        verts,
                        model,
                        0x5300,
                        packet,
                        0x77,
                        ebp - 0x34,
                    ))
                },
            );
        }
    });
}

// ---- the sphere grid's node shapes ----

use ffx_sg::m5main4_abmap::bridge as abmap;

/// [`Twins::check`] for an original that aligns its frame to 16 bytes
/// (`and esp, -16` after `push ebx`): the port's frame at the original's
/// aligned `ebp`, the `low` bytes below it compared after the call (but
/// the cookie word).
fn check_aligned(
    t: &mut Twins,
    what: &str,
    sym: u32,
    args: &[u32],
    low: u32,
    bridge: impl FnOnce(&mut m5_main3_common::G, u32) -> Out,
) {
    let ebp = (t.frame_ebp(args.len()) - 8) & !15;
    // the frame below the compared part (the original's own locals, never
    // handed to a callee) is zeroed too: never-written words read as 0
    t.write(ebp - 0x190, &[0; 0x190]);
    let lo = ebp - low;
    t.b.set_call_top(Some(lo - 0x100));
    t.check(what, sym, Conv::Cdecl, args, |m| bridge(m, ebp));
    t.b.set_call_top(None);
    let n = (low - 4) as usize;
    let mut x = vec![0; n];
    let mut y = vec![0; n];
    ffx_bridge::Mem::read_bytes(&t.a, lo, &mut x);
    ffx_bridge::Mem::read_bytes(&t.b, lo, &mut y);
    if let Some(i) = x.iter().zip(&y).position(|(u, v)| u != v) {
        panic!(
            "{what}: frame byte ebp - {:#x}: {} != {}",
            low - i as u32,
            x[i],
            y[i]
        );
    }
}

#[test]
fn abmap_node_shapes_match_the_original() {
    let mut rng = Rng(0x00a5_1560);
    Twins::each(|t| {
        for k in 0..30 {
            t.reset();
            t.stub(abmap::MAKE_NODE_MATRIX, false, 3, 0, 0x51);
            t.stub(abmap::MUL_MATRIX, false, 3, 0, 0x52);
            t.stub(abmap::SHAPE_DRAW, false, 4, 0, 0x53);
            let base = t.zeros(0x1_1700);
            let n = rng.pick(&[0u16, 1, 3, 5]);
            t.write(base + 2, &n.to_le_bytes());
            for i in 0..6u32 {
                let node = base + 0x808 + 0x28 * i;
                t.write(node + 6, &rng.pick(&[0u16, 1, 2, 0xffff]).to_le_bytes());
                t.w8(node + 0x21, rng.pick(&[0, 1]));
            }
            for k in 0..3u32 {
                t.w32(base + 0xf828 + 0x30 * k, 0x7100 + k);
                t.w32(base + 0xf82c + 0x30 * k, 0x7200 + k);
                t.w32(base + 0xf830 + 0x30 * k, 0x7300 + k);
            }
            t.w32(base + 0x1_16a4, 0x99);
            t.g32(abmap::ABMAP, base);
            let i = rng.pick(&[0u32, 1, 4, 6, 0xffff_ffff]);
            let what = format!("node highlight {k}");
            check_aligned(
                t,
                &what,
                abmap::NODE_HIGHLIGHT_SYM,
                &[i],
                0x100,
                |m, ebp| Out::U32(abmap::node_highlight(m, i, ebp)),
            );
            let what = format!("node shapes {k}");
            check_aligned(t, &what, abmap::NODE_SHAPES_SYM, &[], 0x100, |m, ebp| {
                abmap::node_shapes(m, ebp);
                Out::None
            });
        }
    });
}

// ---- WMainProc ----

use ffx_sg::m5main4_wmain::bridge as wmain;

/// Every callee of `WMainProc` but itself as a recording stub (getters
/// return a value from `rng`), the count globals filled by the list
/// builders.
fn wmain_stubs(t: &mut Twins, rng: &mut Rng, st: u32) {
    let get = |rng: &mut Rng| rng.pick(&[0u32, 1, 2]);
    let stubs: Vec<(u32, u32, u32)> = vec![
        (wmain::GET_ARG, 1, st),
        (wmain::USER_MALLOC, 1, st),
        (wmain::SELECTOR_ALLOCATE, 0, 0x5e1),
        (wmain::SET_ARG, 2, 0),
        (wmain::ADD_BUTTON, 7, 0),
        (wmain::ADD_STATIC, 7, 0),
        (wmain::CHR_MENU_INIT, 2, 0),
        (wmain::SG_ADD_STATIC, 3, 0),
        (wmain::SG_ADD_BUTTON_S, 3, 0),
        (wmain::SG_ADD_BUTTON, 3, 0),
        (wmain::MAPNAME, 1, 0),
        (wmain::SET_TOGGLE, 3, 0),
        (wmain::TOGGLE_VALUE, 3, 0),
        (wmain::GET_TOGGLE_VALUE, 2, get(rng)),
        (wmain::GETTH_DISP_ENABLE, 0, get(rng)),
        (wmain::GET_RESOURCE_STR, 1, 0x6e1),
        (wmain::GET_RESOURCE, 2, get(rng)),
        (wmain::SET_RESOURCE, 2, 0),
        (wmain::SELECT_MAIN_MENU, 2, 0),
        (wmain::EVE_INIT, 0, 0),
        (wmain::LR_BUTTON_MODE, 0, get(rng)),
        (wmain::DBG_PRINTF, 1, 0),
        (wmain::SELECTOR_DISPOSE, 1, 0),
        (wmain::CHR_SELECTOR, 3, 0),
        (wmain::EVE_SELECTOR, 2, 0),
        (wmain::SELECTOR_ITEM_NAME, 1, 0x6e2),
        (wmain::MAP_SET_NAME, 2, 0),
        (wmain::MAP_INIT, 0, 0),
        (wmain::SET_MOUSE_FUKKI, 2, 0),
        (wmain::RESERVE_CALC_NORM, 0, 0),
        (wmain::CAPTURE_ENABLE, 0, 0),
        (wmain::COMMAND_CHR, 3, 0),
        (wmain::COMMAND_MENU, 3, 0),
        (wmain::MAP_GET_DIR, 0, rng.pick(&[0, 1, 2, 5, 0xffff_fffe])),
        (wmain::MAP_SET_DIR, 1, 0),
        (wmain::CONTROL_DISABLE, 3, 0),
        (wmain::SET_TEXT, 3, 0),
        (wmain::SET_TITLE, 2, 0),
        (wmain::MASAKI_INIT, 1, 0),
        (wmain::MAP_GET_NAME, 1, 0x6e3),
        (wmain::SET_SELECT_STR, 2, 0),
        (wmain::SELECTOR_OPEN, 3, 0),
        (wmain::MESSAGE_BOX, 1, 0),
        (wmain::WINDOW_CREATE, 6, 0),
        (wmain::HIDE, 1, get(rng)),
        (wmain::TEX_GET, 0, get(rng)),
        (wmain::TEX_SET, 1, 0),
        (wmain::TOGGLE_RESOURCE9, 0, get(rng)),
        (wmain::SWITCH_DRAW_ID, 0, get(rng)),
        (0x0085_1e10, 0, 0),
        (0x0085_3780, 0, 0),
        (0x0084_e8c0, 0, 0),
        (0x0094_1830, 0, 0),
        (0x0084_efd0, 1, 0),
        (0x0085_3ea0, 0, 0),
        (0x0082_0480, 0, 0),
        (0x0093_0d80, 1, 0),
        (0x0083_bb50, 1, 0),
        (0x0084_e870, 0, 0),
        (0x0085_5430, 0, 0),
        (0x0094_2110, 1, 0),
        (0x0084_4cb0, 0, 0),
        (0x0085_37b0, 0, 0),
        (0x0082_1e30, 0, 0),
        (0x0082_0570, 0, get(rng)),
        (0x0082_d810, 0, get(rng)),
        (0x0082_da60, 1, 0),
        (0x0082_daa0, 1, 0),
        (0x0084_e7c0, 0, 0),
        (0x0090_8f90, 0, 0),
        (0x0084_4ce0, 0, get(rng)),
        (0x0084_4fa0, 1, 0),
        (0x0084_0e40, 0, 0),
        (0x0083_17d0, 0, get(rng)),
        (0x0083_1d00, 1, 0),
        (0x0082_0490, 0, get(rng)),
        (0x0082_03a0, 1, 0),
        (0x0085_4fd0, 0, 0),
        (0x007f_b940, 0, 0),
        (0x007f_bc20, 0, 0),
        (0x0091_9b80, 0, 0),
        (0x0082_05d0, 0, get(rng)),
        (0x0082_1b00, 1, 0),
        (0x0083_3f50, 0, get(rng)),
        (0x0083_57c0, 1, 0),
        (0x0082_05e0, 0, get(rng)),
        (0x0082_1cb0, 1, 0),
        (0x0082_d7e0, 0, get(rng)),
        (0x0082_05a0, 0, get(rng)),
    ];
    for (sym, n, ret) in stubs {
        t.stub(sym, false, n, 0, ret);
    }
    let count = rng.pick(&[0u32, 3, 0xffff_ffff]);
    for sym in [wmain::DIR_LIST, wmain::FILE_LIST] {
        let fill = store_arg(0, 0, count);
        t.stub_ex(sym, false, 2, 0, &Ret::Fixed(0), &fill);
    }
    fprintf_import_stub(t, wmain::SPRINTF, &[(0x00b5_b490, 5)], 3);
}

#[test]
fn wmain_proc_matches_the_original() {
    let mut rng = Rng(0x0084_f550);
    let mut cases: Vec<(u32, u32)> = vec![
        (0x8001, 0),
        (0x8003, 0),
        (0x8011, 0),
        (0x8002, 0),
        (0x8000, 0),
        (0x8020, 0),
    ];
    cases.extend((0..0x6a).map(|id| (0x8006, id)));
    cases.extend([0xf, 0x19, 0x45, 3].map(|id| (0x800a, id)));
    cases.extend([0x16, 0x17, 0x1b, 5].map(|id| (0x8012, id)));
    cases.extend([0xffff_fffd, 0, 1, 7].map(|id| (0x8017, id)));
    Twins::each(|t| {
        for _ in 0..2 {
            for &(msg, id) in &cases {
                t.reset();
                let st = t.words(&[0x5e0, 0x5e2]);
                wmain_stubs(t, &mut rng, st);
                for (g, v) in [
                    (wmain::CUR_PAGE, rng.below(7)),
                    (wmain::CLOCK_FLAGS, rng.below(4)),
                ] {
                    t.g32(g, v);
                }
                t.g32(wmain::CLOCK_FLAGS + 4, 12);
                t.g32(wmain::CLOCK_FLAGS + 8, 34);
                let what = format!("WMainProc {msg:#x} {id:#x}");
                t.check(
                    &what,
                    wmain::WMAIN_PROC,
                    Conv::Cdecl,
                    &[0x9900, msg, id],
                    |m| Out::U32(wmain::wmain_proc(m, 0x9900, msg, id)),
                );
            }
        }
    });
}

#[test]
fn viewer_clut_upload_matches_the_original() {
    Twins::each(|t| {
        t.reset();
        for (s, n) in [
            (viewer::GS_SYNC_PATH, 2),
            (viewer::IPU_INIT, 0),
            (viewer::HARDWARE_INIT, 0),
            (viewer::CLUT_SWIZZLE, 3),
            (viewer::FLUSH_CACHE, 1),
            (viewer::LOAD_IMAGE_101, 2),
        ] {
            t.stub(s, false, n, 0, 0x10 + n);
        }
        let v = new_viewer(t);
        let _ = v;
        t.check(
            "clut upload",
            viewer::CLUT_UPLOAD_SYM,
            Conv::Cdecl,
            &[],
            |m| Out::U32(viewer::clut_upload(m)),
        );
    });
}

#[test]
fn abmap_node_spheres_match_the_original() {
    let mut rng = Rng(0x00a5_0ed0);
    Twins::each(|t| {
        for k in 0..30 {
            t.reset();
            t.stub(abmap::MAKE_NODE_MATRIX, false, 3, 0, 0x61);
            t.stub(abmap::MAKE_LINK_MATRIX, false, 4, 0, 0x62);
            t.stub(abmap::MUL_MATRIX, false, 3, 0, 0x63);
            t.stub(abmap::DRAW_SPHERE, false, 4, 0, 0x64);
            let base = t.zeros(0x1_1700);
            let i = rng.below(4);
            let node = base + 0x808 + 0x28 * i;
            let kind = rng.below(3);
            t.write(node + 6, &(kind as u16).to_le_bytes());
            t.w8(node + 0x21, rng.next() as u8);
            t.write(node + 0x26, &(rng.next() as u16).to_le_bytes());
            t.w8(base + 0x1_15c9, rng.pick(&[0, 0x7f, 0xff, 0x15]));
            t.w32(base + 0x1_1698, rng.next());
            let row = base + 0xf828 + 0x30 * kind;
            t.w32(row + 0x10, (0.5f32 + rng.below(4) as f32).to_bits());
            let link = if rng.chance(3) {
                0x1000u16
            } else {
                rng.next() as u16
            };
            t.write(row + 0x14, &link.to_le_bytes());
            t.g32(abmap::ABMAP, base);
            t.g32(abmap::SPHERE_MODEL, 0x7700);
            for j in 0..7 {
                t.g32(abmap::LIT_COLOURS + 4 * j, rng.next());
                t.g32(abmap::DARK_COLOURS + 4 * j, rng.next());
            }
            let what = format!("node spheres {k}");
            check_aligned(t, &what, abmap::NODE_SPHERES_SYM, &[i], 0x140, |m, ebp| {
                abmap::node_spheres(m, i, ebp);
                Out::None
            });
        }
    });
}

/// `lock cmpxchg` as `InterlockedCompareExchange(p, new, old)` (stdcall,
/// not logged: the original takes its lock with an inline `xchg`).
fn cas_code(t: &mut Twins) -> u32 {
    t.code(&[
        0x8b, 0x4c, 0x24, 0x04, // mov ecx, [esp + 4]
        0x8b, 0x54, 0x24, 0x08, // mov edx, [esp + 8]
        0x8b, 0x44, 0x24, 0x0c, // mov eax, [esp + 0xc]
        0xf0, 0x0f, 0xb1, 0x11, // lock cmpxchg [ecx], edx
        0xc2, 0x0c, 0x00, // ret 0xc
    ])
}

#[test]
fn dlmalloc_mspace_malloc_matches_the_original() {
    let mut rng = Rng(0x006e_c5c0);
    Twins::each(|t| {
        for k in 0..300 {
            t.reset();
            let m = t.zeros(0x200);
            let arena = t.zeros(0x10000);
            let chunk = |n: u32| arena + 0x400 * n;
            // the lock
            let cas = cas_code(t);
            let slot = t.rebase(lib::INTERLOCKED_COMPARE_EXCHANGE);
            t.patch(slot, &cas.to_le_bytes());
            t.stub_import(lib::GET_CURRENT_THREAD_ID, 0, 0, 0x1234);
            let release = m5_main3_common::set_word(m + 0x1bc, 0);
            let sleep = t.stub_code(lib::SLEEP_EX, false, 2, 2, &Ret::Fixed(0), &release);
            let s = t.rebase(lib::SLEEP_EX);
            t.patch(s, &sleep.to_le_bytes());
            t.w32(m + 0x1b8, rng.pick(&[0, 2, 2, 3]));
            match rng.below(4) {
                0 => t.w32(m + 0x1bc, 0),
                1 => {
                    t.w32(m + 0x1bc, 1);
                    t.w32(m + 0x1c0, 2);
                    t.w32(m + 0x1c4, 0x1234);
                }
                2 => {
                    // another thread's: released by the 64th try's sleep
                    t.w32(m + 0x1bc, 1);
                    t.w32(m + 0x1c0, 1);
                    t.w32(m + 0x1c4, 0x99);
                }
                _ => {}
            }
            // small bins
            let mut map = 0u32;
            for i in 2..32u32 {
                if k % 3 == 0 || !rng.chance(4) {
                    continue;
                }
                map |= 1 << i;
                let b = m + 0x28 + 8 * i;
                let p = chunk(i);
                t.w32(p + 4, (i << 3) | 1);
                let second = rng.chance(3);
                if second {
                    let q = p + 0x200;
                    t.w32(q + 4, (i << 3) | 1);
                    t.w32(p + 8, q);
                    t.w32(p + 0xc, b);
                    t.w32(q + 8, b);
                    t.w32(q + 0xc, p);
                    t.w32(b + 0xc, q);
                } else {
                    t.w32(p + 8, b);
                    t.w32(p + 0xc, b);
                    t.w32(b + 0xc, p);
                }
                t.w32(b + 8, p);
            }
            t.w32(m, map);
            t.w32(m + 4, rng.pick(&[0, 0, 4]));
            t.w32(m + 0x10, rng.pick(&[0, 0, 0, m + 0x100]));
            let dvs = rng.pick(&[0u32, 0x18, 0x40, 0x100]);
            t.w32(m + 8, dvs);
            t.w32(m + 0x14, if dvs == 0 { 0 } else { chunk(0) + 8 });
            t.w32(chunk(0) + 0xc, dvs | 1);
            t.w32(m + 0xc, rng.pick(&[0u32, 0x78, 0x80, 0x80, 0x1000]));
            t.w32(m + 0x18, chunk(33));
            let block = |t: &mut Twins, n: u32, head: u32| {
                t.w32(chunk(n) + 4, head);
                chunk(n) + 8
            };
            let small = block(t, 34, 0x81);
            let large = block(t, 35, 0x203);
            let sys = block(t, 36, rng.pick(&[0x400, 0x401]));
            t.stub(lib::TMALLOC_SMALL, false, 2, 0, rng.pick(&[0, small]));
            t.stub(lib::TMALLOC_LARGE, false, 2, 0, rng.pick(&[0, large]));
            t.stub(lib::SYS_ALLOC, false, 2, 0, sys);
            t.stub(lib::MEMSET, false, 3, 0, 0x55);
            let bytes = rng.pick(&[
                0u32,
                1,
                0xa,
                0xb,
                0x10,
                0x30,
                0x70,
                0x75,
                0xe0,
                0xf4,
                0xf5,
                0x200,
                0xffff_ffc0,
            ]);
            let what = format!("mspace_malloc {k}");
            let ebp = t.frame_ebp(2);
            t.check(
                &what,
                lib::MSPACE_MALLOC_SYM,
                Conv::Cdecl,
                &[m, bytes],
                |mm| Out::U32(lib::mspace_malloc(mm, m, bytes, ebp + 8)),
            );
        }
    });
}

/// The lock's imports for a heap at `m` (as in the malloc test).
fn dl_lock_stubs(t: &mut Twins, rng: &mut Rng, m: u32) {
    let cas = cas_code(t);
    let slot = t.rebase(lib::INTERLOCKED_COMPARE_EXCHANGE);
    t.patch(slot, &cas.to_le_bytes());
    t.stub_import(lib::GET_CURRENT_THREAD_ID, 0, 0, 0x1234);
    let release = m5_main3_common::set_word(m + 0x1bc, 0);
    let sleep = t.stub_code(lib::SLEEP_EX, false, 2, 2, &Ret::Fixed(0), &release);
    let s = t.rebase(lib::SLEEP_EX);
    t.patch(s, &sleep.to_le_bytes());
    t.w32(m + 0x1b8, rng.pick(&[0, 2, 2]));
    match rng.below(3) {
        0 => {
            t.w32(m + 0x1bc, 1);
            t.w32(m + 0x1c0, 1);
            t.w32(m + 0x1c4, 0x1234);
        }
        1 => {
            t.w32(m + 0x1bc, 1);
            t.w32(m + 0x1c0, 1);
            t.w32(m + 0x1c4, 0x99);
        }
        _ => {}
    }
}

/// dlmalloc's tree bin index of a large size.
fn dl_tree_index(size: u32) -> u32 {
    let x = size >> 8;
    if x == 0 {
        0
    } else if x > 0xffff {
        31
    } else {
        let k = x.ilog2();
        ((size >> (k + 7)) & 1) + k * 2
    }
}

/// Free chunk `x` (size `size`) put alone in its bin.
fn dl_bin(t: &mut Twins, m: u32, x: u32, size: u32) {
    if size >> 3 < 0x20 {
        let i = size >> 3;
        let b = m + 0x28 + 8 * i;
        let map: u32 = ffx_bridge::Mem::read(&t.a, m);
        if map & 1 << i == 0 {
            t.w32(x + 8, b);
            t.w32(x + 0xc, b);
            t.w32(b + 8, x);
            t.w32(b + 0xc, x);
        } else {
            // in front of the list already there
            let old: u32 = ffx_bridge::Mem::read(&t.a, b + 8);
            t.w32(x + 8, old);
            t.w32(x + 0xc, b);
            t.w32(old + 0xc, x);
            t.w32(b + 8, x);
        }
        t.w32(m, map | 1 << i);
    } else {
        let i = dl_tree_index(size);
        let head = m + 0x130 + 4 * i;
        t.w32(head, x);
        t.w32(x + 0x18, head);
        t.w32(x + 8, x);
        t.w32(x + 0xc, x);
        t.w32(x + 0x10, 0);
        t.w32(x + 0x14, 0);
        t.w32(x + 0x1c, i);
        let map: u32 = ffx_bridge::Mem::read(&t.a, m + 4);
        t.w32(m + 4, map | 1 << i);
    }
}

#[test]
fn dlmalloc_mspace_free_matches_the_original() {
    let mut rng = Rng(0x006e_bee0);
    Twins::each(|t| {
        for k in 0..400 {
            t.reset();
            let m = t.zeros(0x200);
            let arena = t.zeros(0x6000) + 0x10;
            dl_lock_stubs(t, &mut rng, m);
            t.stub(lib::MEMSET, false, 3, 0, 0x55);
            t.stub(lib::SYS_TRIM, false, 2, 0, 0x56);
            t.stub(lib::RELEASE_UNUSED, false, 1, 0, 0x57);
            t.stub(lib::TMALLOC_SMALL, false, 2, 0, 0);
            // other chunks already in the bins (the inserts walk them)
            let sizes = [0x20u32, 0x80, 0x100, 0x180, 0x400, 0x408];
            for n in 0..rng.below(4) {
                let x = arena + 0x4000 + 0x400 * n;
                let size = rng.pick(&sizes);
                // bit 2 is not part of the size
                t.w32(x + 4, size | 1 | rng.pick(&[0, 4]));
                dl_bin(t, m, x, size);
            }
            // A (before), P (freed), N (after)
            let prev = rng.pick(&[0x20u32, 0x100, 0x300]);
            let psize = if k % 37 == 5 { 0 } else { rng.pick(&sizes) };
            let nsize = rng.pick(&[0x20u32, 0x100, 0x300]);
            let a = arena + 0x400;
            let p = a + prev;
            let n = p + psize;
            let prev_free = rng.chance(2);
            let mmapped = rng.chance(10);
            let bits = if mmapped {
                0
            } else if rng.chance(20) {
                1
            } else if prev_free {
                2
            } else {
                3
            };
            t.w32(p + 4, psize | bits);
            t.w32(p, prev);
            t.w32(a + 4, prev | 1);
            let dv_pick = rng.below(4);
            if prev_free && !mmapped {
                if dv_pick == 0 {
                    t.w32(m + 0x14, a);
                    t.w32(m + 8, prev);
                } else {
                    dl_bin(t, m, a, prev);
                }
            }
            let next_free = rng.chance(2);
            let nbits = rng.pick(&[1u32, 1, 1, 0]) | if next_free { 0 } else { 2 };
            t.w32(n + 4, nsize | nbits);
            t.w32(n + nsize + 4, 0x11);
            match rng.below(4) {
                0 => {
                    t.w32(m + 0x18, n);
                    t.w32(m + 0xc, nsize);
                }
                1 if next_free && dv_pick != 0 => {
                    t.w32(m + 0x14, n);
                    t.w32(m + 8, nsize);
                }
                _ => {
                    if next_free {
                        dl_bin(t, m, n, nsize);
                    }
                    t.w32(m + 0x18, arena + 0x3000);
                    t.w32(m + 0xc, 0x100);
                }
            }
            let trim = rng.pick(&[0, 0x200, 0x10_0000, nsize + psize, nsize + psize + prev]);
            t.w32(m + 0x1c, trim);
            t.w32(m + 0x20, rng.pick(&[1, 5]));
            t.w32(m + 0x1b0, 0x10_0000);
            t.w32(m + 0x10, rng.pick(&[0, 0, 0, arena, arena + 0x500]));
            // a mapped chunk's regions
            let total = prev + 0x10 + psize;
            let base = p - prev;
            let mut info = Vec::new();
            for (off, v) in [
                (0u8, base),
                (4, base),
                (0xc, rng.pick(&[total, total, total + 8, total - 8])),
                (0x10, rng.pick(&[0x1000, 0x1000, 0x2000])),
            ] {
                info.extend(store_arg(1, off, v));
            }
            let q = t.stub_code(
                lib::VIRTUAL_QUERY,
                false,
                3,
                3,
                &Ret::Fixed(rng.pick(&[1, 1, 0])),
                &info,
            );
            let qs = t.rebase(lib::VIRTUAL_QUERY);
            t.patch(qs, &q.to_le_bytes());
            t.stub_import(lib::VIRTUAL_FREE, 3, 3, rng.pick(&[1, 1, 0]));
            let mem = if k % 50 == 0 { 0 } else { p + 8 };
            let what = format!("mspace_free {k}");
            let ebp = t.frame_ebp(2);
            // the original's own locals (ebp - 0xc .. - 1) are not compared
            t.skip = 0xc;
            t.check_framed(
                &what,
                lib::MSPACE_FREE_SYM,
                Conv::Cdecl,
                &[m, mem],
                0x28,
                |mm, ebp2| {
                    lib::mspace_free(mm, m, mem, ebp + 0xc, ebp2 - 0x28);
                    Out::None
                },
            );
            t.skip = 4;
        }
    });
}

/// dlmalloc's `internal_memalign` (`0x006ea5f0`, m5-wp5e): `mspace_malloc`
/// and `mspace_free` recording stubs, the malloc's block placed at every
/// 8-byte offset of a 0x40 window (aligned or not: a lead of under 16
/// moves on), chunks in use, free-bit set or mapped, tails long enough
/// to split or not; alignments 0..8 (plain malloc), 9..0x40 (rounded up
/// to powers of two), requests 0..0x60 and too large ones; the lock as
/// in the malloc test. Result and memory (the stubs' logs).
#[test]
fn dlmalloc_internal_memalign_matches_the_original() {
    let mut rng = Rng(0x006e_a5f0);
    Twins::each(|t| {
        for k in 0..400 {
            t.reset();
            let m = t.zeros(0x200);
            dl_lock_stubs(t, &mut rng, m);
            // junk inside (the new chunk's old head bit 0 is kept)
            let junk: Vec<u32> = (0..0x400).map(|_| rng.next()).collect();
            let arena = t.words(&junk) + 0x100;
            let mem = arena + 8 * rng.below(8);
            let flags = rng.pick(&[1u32, 3, 3, 0]);
            let align = rng.pick(&[0u32, 1, 8, 9, 0x10, 0x11, 0x20, 0x30, 0x40]);
            let bytes = if rng.chance(20) {
                rng.pick(&[0xffff_ffc0u32 - align.max(0x10), 0xffff_ff00])
            } else {
                rng.below(0x60)
            };
            // the block as large as asked (nb + alignment + 12, the chunk
            // 4 more) and some spare
            let nb = if bytes < 0xb {
                0x10
            } else {
                bytes.wrapping_add(0xb) & !7
            };
            let a = align.max(0x10).next_power_of_two();
            let size = (nb.min(0x100) + a + 0x10 + 7) & !7;
            let size = size + 8 * rng.below(6);
            t.w32(mem - 8, rng.pick(&[0u32, 0x40, 0x123]));
            t.w32(mem - 4, size | flags);
            let ret = if rng.chance(8) { 0 } else { mem };
            t.stub(lib::MSPACE_MALLOC_SYM, false, 2, 0, ret);
            t.stub(lib::MSPACE_FREE_SYM, false, 2, 0, 0x77);
            let what = format!(
                "internal_memalign {k} (align {align:#x}, {bytes:#x}, size {size:#x}|{flags})"
            );
            let spins = t.frame_ebp(3) - 0x200;
            t.check(&what, 0x006e_a5f0, Conv::Cdecl, &[m, align, bytes], |mm| {
                Out::U32(lib::internal_memalign(mm, m, align, bytes, spins))
            });
        }
    });
}
