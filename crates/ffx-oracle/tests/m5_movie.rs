//! M5 stage 1, list `movie` (`ffx_movie::m5_movie`) against the
//! original: on the image alone with synthetic objects, and on the
//! snapshots (`FFX_SNAPSHOT`). Results and every byte written must agree.
//! Needs `FFX_GAME_DIR`.
//!
//! The movie player never ran in a coverage snapshot, so every branch is
//! reached with synthetic managers, lists and work areas. The callees
//! (the video clock, the `std` containers' helpers, `dbgPrintf`, the PS2
//! file and task layers, `operator delete`) are recording stubs in both
//! oracles alike; the clock stubs answer in `edx:eax`.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

#[path = "m5_small/twins.rs"]
mod twins;

use ffx_movie::m5_movie::bridge as b;
use twins::{Conv, Out, Rng, Twins};

#[test]
fn manager_helpers_match_the_original() {
    Twins::each(|t| {
        for m in [0, 0x4321] {
            t.reset();
            t.g32(b::FMV_PLAYER_MANAGER.sym, m);
            t.stub(b::MANAGER_DTOR, true, 0, 0, 0);
            t.stub(b::OPERATOR_DELETE, false, 1, 0, 0);
            t.check(
                &format!("DestroyInstance {m:#x}"),
                0x006d_7370,
                Conv::Cdecl,
                &[],
                |h| {
                    b::destroy_instance(h);
                    Out::None
                },
            );
        }
        // lists of 0..3 nodes, the head first
        for n in 0..4u32 {
            for (sym, off) in [(0x006d_6c30u32, 0u32), (0x006d_af30, 0xc)] {
                t.reset();
                t.stub(b::OPERATOR_DELETE, false, 1, 0, 0);
                let head = t.zeros(0x10);
                let nodes: Vec<u32> = (0..n).map(|_| t.zeros(0x10)).collect();
                let mut chain = vec![head];
                chain.extend(&nodes);
                for (i, &x) in chain.iter().enumerate() {
                    let next = chain[(i + 1) % chain.len()];
                    t.w32(x, next);
                    t.w32(x + 4, 0x7777);
                }
                let this = t.words(&[
                    0x11,
                    0x22,
                    0x33,
                    0,
                    0,
                    if n % 2 == 0 { 0 } else { 0x5555 },
                    0,
                ]);
                t.w32(this + off, head);
                t.w32(this + off + 4, n);
                let what = format!("list {sym:#x} n {n}");
                t.check(&what, sym, Conv::Thiscall(this), &[], |h| {
                    if sym == 0x006d_6c30 {
                        b::list_dtor(h, this);
                    } else {
                        b::request_queue_dtor(h, this);
                    }
                    Out::None
                });
            }
        }
        t.reset();
        // erase writes the returned iterator through its first word
        let log = t.stub_code(b::TREE_ERASE_RANGE, true, 3, 3, 0);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0xc7, 0x00]);
        c.extend(0x0e0e_0e0eu32.to_le_bytes()); // mov dword [eax], it
        c.push(0xe9);
        let here = t.cave_next + c.len() as u32 + 4;
        c.extend(log.wrapping_sub(here).to_le_bytes());
        let code = t.cave_code(&c);
        t.jump(b::TREE_ERASE_RANGE, code);
        t.stub(b::OPERATOR_DELETE, false, 1, 0, 0);
        let head = t.words(&[0x1234, 0, 0]);
        let map = t.words(&[head, 3]);
        t.check_framed(
            "0x006d6c70",
            0x006d_6c70,
            Conv::Thiscall(map),
            &[],
            4,
            |h, ebp| {
                b::map_dtor(h, ebp, map);
                Out::None
            },
        );
        for (a, c) in [(0, 0), (0x10, 0), (0, 0x20), (0x10, 0x20)] {
            t.reset();
            t.stub(b::USER_FREE, false, 1, 0, 0x66);
            let mut w = vec![0u32; 8];
            w[6] = a;
            w[3] = c;
            let this = t.words(&w);
            t.check(
                &format!("0x006d6cd0 {a} {c}"),
                0x006d_6cd0,
                Conv::Thiscall(this),
                &[],
                |h| Out::U32(b::free_buffers(h, this)),
            );
        }
        t.reset();
        t.stub(b::LIST_HEAD_NEW, true, 2, 2, 0x0bad_0000);
        let this = t.alloc(&[0xcc; 0x20]);
        t.check("0x006daee0", 0x006d_aee0, Conv::Thiscall(this), &[], |h| {
            Out::U32(b::request_queue_ctor(h, this))
        });
        t.reset();
        t.stub(b::PROCESS_REQUESTS, true, 0, 0, 0x99);
        t.check(
            "0x006dadf0",
            0x006d_adf0,
            Conv::Thiscall(0x1234),
            &[],
            |h| Out::U32(b::process_requests_thunk(h, 0x1234)),
        );
    });
}

/// A stub over `sym` (thiscall, no words) logging `ecx` and answering
/// `value` in `edx:eax`.
fn clock_stub(t: &mut Twins, sym: u32, value: u64) {
    let log = t.stub_code(sym, true, 0, 0, 0);
    let mut c = vec![0xe8];
    let here = t.cave_next + 5;
    c.extend(log.wrapping_sub(here).to_le_bytes()); // call log
    c.push(0xb8);
    c.extend((value as u32).to_le_bytes()); // mov eax, low
    c.push(0xba);
    c.extend(((value >> 32) as u32).to_le_bytes()); // mov edx, high
    c.push(0xc3);
    let code = t.cave_code(&c);
    t.jump(sym, code);
}

#[test]
fn manager_times_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0f3e_7111);
        for case in 0..24u32 {
            t.reset();
            let end = match case % 6 {
                0 => 0,
                1 => 1000,
                2 => u64::from(rng.next()) * 7919,
                3 => 0x8000_0000_0000_0000 | u64::from(rng.next()),
                4 => u64::MAX,
                _ => 123_456_789,
            };
            let now = match case % 4 {
                0 => end,
                1 => end.wrapping_sub(1),
                2 => end.wrapping_add(1),
                _ => u64::from(rng.next()),
            };
            clock_stub(t, b::VIDEO_END_TIME, end);
            let mut w = vec![0u32; 0x1c0];
            let bytes = if case < 12 {
                [1, 1, 1]
            } else {
                [rng.below(2) as u8, rng.below(2) as u8, rng.below(2) as u8]
            };
            w[0x6d0 / 4] = u32::from(bytes[0]) | u32::from(bytes[1]) << 16;
            w[0x6e4 / 4] = u32::from(bytes[2]);
            let this = t.words(&w);
            t.g32(b::FMV_SKIPPED.sym, u32::from(case % 5 == 0 && case >= 12));
            t.check(
                &format!("getEndTimeMs #{case}"),
                0x006d_91d0,
                Conv::Thiscall(this),
                &[],
                |h| Out::U32(b::get_end_time_ms(h, this)),
            );
            clock_stub(t, b::VIDEO_CURRENT_TIME, now);
            t.check(
                &format!("isVideoAtEndFrame #{case}"),
                0x006d_a240,
                Conv::Thiscall(this),
                &[],
                |h| Out::U8(u8::from(b::is_video_at_end_frame(h, this))),
            );
        }
    });
}

#[test]
fn ps2_movie_layer_matches_the_original() {
    Twins::each(|t| {
        let stubs: [(u32, u32, bool); 10] = [
            (0x0076_ed80, 0x00b5_2e28, false),
            (0x0076_ed90, 0x00b5_2f10, false),
            (0x0076_eda0, 0x00b5_2f80, false),
            (0x0076_efb0, 0x00b5_2810, true),
            (0x0076_f010, 0x00b5_2410, false),
            (0x0076_f030, 0x00b5_2e98, false),
            (0x0076_f040, 0x00b5_27a0, false),
            (0x0076_f070, 0x00b5_25e0, false),
            (0x0076_f0a0, 0x00b5_2730, true),
            (0x0076_f0b0, 0x00b5_26c0, true),
        ];
        for (sym, msg, zero) in stubs {
            t.reset();
            t.stub(b::DBG_PRINTF, false, 1, 0, 0x5a);
            t.check(&format!("stub {sym:#x}"), sym, Conv::Cdecl, &[], |h| {
                b::debug_stub(h, msg);
                if zero { Out::U32(0) } else { Out::None }
            });
        }
        t.reset();
        t.stub(b::TASK_WAIT, false, 1, 0, 0x31);
        t.check("movie_wait", 0x0077_daa0, Conv::Cdecl, &[], |h| {
            Out::U32(b::movie_wait(h))
        });
        t.stub(b::TO_DEBUG_PAD_PRINT_FLAG, false, 0, 0, 0x32);
        t.check("0x0077da90", 0x0077_da90, Conv::Cdecl, &[], |h| {
            Out::U32(b::jmp_thunk(h, b::TO_DEBUG_PAD_PRINT_FLAG, &[]))
        });
        t.stub(b::SCE_OPEN, false, 2, 0, 0x7);
        t.stub(b::SCE_CLOSE, false, 1, 0, 0x8);
        t.stub(b::DBG_PRINTF, false, 3, 0, 0);
        t.check("movieOpen", 0x0093_da30, Conv::Cdecl, &[0x2222], |h| {
            Out::U32(b::movie_open(h, 0x2222))
        });
        t.reset();
        t.stub(b::SCE_CLOSE, false, 1, 0, 0x8);
        t.stub(b::DBG_PRINTF, false, 2, 0, 0);
        t.check("movieClose", 0x0093_da00, Conv::Cdecl, &[7], |h| {
            Out::U32(b::movie_close(h, 7))
        });
        for convert in [0, 1, 5] {
            t.reset();
            t.stub(b::TK_SYNC_V, false, 1, 0, 0x41);
            t.stub(b::MOVIE_CSC, false, 0, 0, 0);
            t.stub(b::TASK_WAIT, false, 1, 0, 0x42);
            let slots = t.frame_ebp(1) + 8;
            t.check(
                &format!("0x0077dc50 {convert}"),
                0x0077_dc50,
                Conv::Cdecl,
                &[convert],
                |h| Out::U32(b::movie_frame_wait(h, convert, slots)),
            );
        }
        for layout in [0, 1, 2, 0xffff_ffff] {
            t.reset();
            let w = t.zeros(0x5_0800);
            t.w32(w + 0x506c8, layout);
            t.w32(w + 0x5059c, 0x0bb0_0000);
            t.w32(w + 0x50758, 0x400);
            t.g32(b::AUDIO_WORK.sym, w);
            t.stub(b::AUDIO_MIX_HALVES, false, 3, 0, 0);
            t.check(
                &format!("0x0077de40 {layout}"),
                0x0077_de40,
                Conv::Cdecl,
                &[],
                |h| {
                    b::movie_audio_mix(h);
                    Out::None
                },
            );
        }
        for open in [0u32, 1, 2] {
            t.reset();
            let at = t.rebase(b::MOVIE_VIEW_OPEN.sym);
            t.write(at, &[open as u8]);
            t.stub(b::GU_WINDOW_CREATE, false, 6, 0, 0);
            t.check(
                &format!("movie_view {open}"),
                0x0094_1830,
                Conv::Cdecl,
                &[],
                |h| {
                    b::movie_view(h);
                    Out::None
                },
            );
        }
    });
}
