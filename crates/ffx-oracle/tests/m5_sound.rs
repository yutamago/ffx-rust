//! M5 stage 1, list `sound` (`ffx_sound::m5_sound`) against the
//! original: on the image alone with synthetic objects, and on the
//! snapshots (`FFX_SNAPSHOT`) over the game's memory. Results and every
//! byte written must agree. Needs `FFX_GAME_DIR`.
//!
//! None of these ran in a coverage run (start-up, shutdown and the
//! settings screens), so every branch is reached with synthetic objects
//! on both kinds of oracle. The callees (the Win32 imports through their
//! slots, `operator new`/`delete`, the allocator, the `PRedBlackTree` and
//! `PFreeList` members, the `SfxEvent`/`SfxBank` methods, the queue's
//! `Fmod_PushCommand`) are recording stubs in both oracles alike: each
//! call appends its tag, `ecx` and its arguments to a log in memory, so
//! the diff checks that both made the same calls in the same order.
//! `sprintf` and `memset` are `MSVCR110`'s own.
//!
//! Paths only a broken object reaches are left out: a failed `operator
//! new` in `createVsyncEmuThread` (the original then writes to `0x10`),
//! and a tree whose first-node word is 0 in `FmodSfx_stopAllSound` /
//! `FmodSfx_shutdown` (the original then reads node 0's fields).

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

#[path = "m5_small/twins.rs"]
mod twins;

use ffx_sound::m5_sound::bridge as b;
use twins::{Conv, Mem, Out, Rng, Twins};

/// `Fmod_PushCommand` (thiscall `(mgr; cmd, a, b, c, d)` ret 0x14).
const PUSH_COMMAND: u32 = 0x006f_a300;
/// `g_FmodManager`.
const FMOD_MANAGER: u32 = 0x00ce_9010;

/// A `0x20`-byte thread object whose vtable slots are recording thiscall
/// stubs (`slot 0`: 1 word, `slot 1`: none), with the given words from
/// `+4` on.
fn thread_object(t: &mut Twins, fields: [u32; 7], ret0: u32, ret1: u32) -> u32 {
    let s0 = t.stub_code(0x5100, true, 1, 1, ret0);
    let s1 = t.stub_code(0x5101, true, 0, 0, ret1);
    let vt = t.words(&[s0, s1]);
    let mut w = vec![vt];
    w.extend(fields);
    t.words(&w)
}

#[test]
fn create_vsync_emu_thread_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x1357_9bdf);
        for i in 0..6 {
            t.reset();
            let obj = t.alloc(&[0xcc; 0x20]);
            t.stub(b::OPERATOR_NEW, false, 1, 0, obj);
            t.stub_import(b::CREATE_SEMAPHORE_IAT, false, 4, 4, 0x1234 + i);
            t.stub(b::THREAD_START, true, 4, 4, 0x77);
            t.stub_import(b::SET_THREAD_PRIORITY_IAT, false, 2, 2, 1);
            t.stub_import(b::RESUME_THREAD_IAT, false, 1, 1, rng.word());
            let handler = rng.word();
            t.skip = 4;
            t.check_framed(
                &format!("createVsyncEmuThread #{i}"),
                0x0067_9e60,
                Conv::Cdecl,
                &[handler],
                0x68,
                |m, ebp| Out::U32(b::create_vsync_emu_thread(m, ebp, handler)),
            );
            t.skip = 0;
        }
    });
}

#[test]
fn end_vsync_emu_thread_matches_the_original() {
    Twins::each(|t| {
        for case in 0..3 {
            t.reset();
            let thread = if case == 1 { 0 } else { 0x4444 };
            let obj = thread_object(t, [thread, 9, 0, 0x5555, 0, 1, 0x88], 0x31, 0);
            t.g32(b::VSYNC_THREAD.sym, if case == 0 { 0 } else { obj });
            t.stub_import(b::WAIT_IAT, false, 2, 2, 0);
            t.stub_import(b::CLOSE_HANDLE_IAT, false, 1, 1, 0x66);
            t.check(
                &format!("endVsyncEmuThread #{case}"),
                0x0067_9f20,
                Conv::Cdecl,
                &[],
                |m| Out::U32(b::end_vsync_emu_thread(m)),
            );
        }
    });
}

#[test]
fn vsync_emu_wake_matches_the_original() {
    Twins::each(|t| {
        t.reset();
        let obj = thread_object(t, [1, 2, 3, 4, 5, 6, 0x99], 0, 0);
        t.g32(b::VSYNC_THREAD.sym, obj);
        t.stub_import(b::RELEASE_SEMAPHORE_IAT, false, 3, 3, 0x2a);
        t.check("VsyncEmu_Wake", 0x0067_9f80, Conv::Cdecl, &[], |m| {
            Out::U32(b::vsync_emu_wake(m))
        });
    });
}

#[test]
fn thread_deleting_dtors_match_the_original() {
    Twins::each(|t| {
        for sym in [0x0067_9bb0, 0x0067_9c00] {
            for flags in [0, 1, 2, 3, 0xffff_ff01] {
                for thread in [0, 0x4321] {
                    t.reset();
                    let obj = thread_object(t, [thread, 7, 1, 2, 3, 4, 0x88], 0, 0);
                    t.stub_import(b::TERMINATE_THREAD_IAT, false, 2, 2, 1);
                    t.stub_import(b::CLOSE_HANDLE_IAT, false, 1, 1, 1);
                    t.stub(b::OPERATOR_DELETE, false, 1, 0, 0x55);
                    let what =
                        format!("deleting dtor {sym:#x} flags {flags:#x} thread {thread:#x}");
                    t.check(&what, sym, Conv::Thiscall(obj), &[flags], |m| {
                        Out::U32(if sym == 0x0067_9bb0 {
                            b::sys_thread_deleting_dtor(m, obj, flags)
                        } else {
                            b::vsync_thread_deleting_dtor(m, obj, flags)
                        })
                    });
                }
            }
        }
    });
}

/// The handler: logs, and clears the thread's `+0x18` on its `n`-th call.
fn handler(t: &mut Twins, obj: u32, n: u32) -> u32 {
    let ctr = t.words(&[n]);
    let log = t.stub_code(0x6000, false, 0, 0, 0xabc);
    let mut c = vec![0xff, 0x0d];
    c.extend(ctr.to_le_bytes()); // dec dword [ctr]
    c.extend([0x75, 0x0a]); // jnz +10
    c.extend([0xc7, 0x05]);
    c.extend((obj + 0x18).to_le_bytes());
    c.extend(0u32.to_le_bytes()); // mov dword [obj+0x18], 0
    c.push(0xe9);
    let here = t.cave_next + c.len() as u32 + 4;
    c.extend(log.wrapping_sub(here).to_le_bytes()); // jmp log
    t.cave_code(&c)
}

#[test]
fn vsync_thread_run_matches_the_original() {
    Twins::each(|t| {
        for (case, (running, calls, thread)) in [(0, 1, 0x10), (1, 1, 0), (1, 3, 0x10), (1, 5, 0)]
            .into_iter()
            .enumerate()
        {
            t.reset();
            let obj = thread_object(t, [thread, 2, 0, 0, 0, running, 0x88], 0, 0);
            let f = handler(t, obj, calls);
            t.w32(obj + 0x10, f);
            t.stub_import(b::WAIT_IAT, false, 2, 2, 0x102);
            t.stub_import(b::TERMINATE_THREAD_IAT, false, 2, 2, 1);
            t.stub_import(b::CLOSE_HANDLE_IAT, false, 1, 1, 1);
            t.check(
                &format!("VsyncThreadImpl_run #{case}"),
                0x0067_9c60,
                Conv::Thiscall(obj),
                &[],
                |m| Out::U32(b::vsync_thread_run(m, obj)),
            );
        }
    });
}

#[test]
fn thread_start_matches_the_original() {
    Twins::each(|t| {
        for code in [0, 1, 0xdead] {
            t.reset();
            // ExitThread returns here: past the call, the int3 padding
            // becomes `pop esi; pop ebp; ret 4` (the port never runs it)
            let after = t.rebase(0x0067_9dc9);
            t.patch(after, &[0x5e, 0x5d, 0xc2, 0x04, 0x00]);
            let obj = thread_object(t, [1, 2, 0x0101_0100, 0, 0, 0, 0], 0, code);
            t.stub_import(b::EXIT_THREAD_IAT, false, 1, 1, 0x3e);
            t.check(
                &format!("Thread_Start code {code:#x}"),
                0x0067_9db0,
                Conv::Stdcall,
                &[obj],
                |m| Out::U32(b::thread_start(m, obj)),
            );
        }
    });
}

#[test]
fn phyre_free_thunk_matches_the_original() {
    Twins::each(|t| {
        for p in [0, 0x1234_5678] {
            t.reset();
            t.stub(b::PHYRE_FREE, false, 1, 0, p ^ 0x5a);
            t.check("code_0044cd10", 0x0044_cd10, Conv::Cdecl, &[p], |m| {
                Out::U32(b::phyre_free_thunk(m, p))
            });
        }
    });
}

#[test]
fn queue_entry_points_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x2468_1357);
        for i in 0..8 {
            t.reset();
            let mgr = t.zeros(0x50);
            t.g32(FMOD_MANAGER, mgr);
            let ret = rng.word();
            t.stub(PUSH_COMMAND, true, 5, 5, ret);
            let (x, y, z) = (rng.word(), rng.word(), rng.word());
            t.check("iSfxStopNonLoopSound", 0x0067_a4c0, Conv::Cdecl, &[], |m| {
                Out::U32(b::i_sfx_stop_non_loop_sound(m))
            });
            t.check("iShoutStart", 0x0067_a4e0, Conv::Cdecl, &[x, y, z], |m| {
                Out::U32(b::i_shout_start(m, x, y, z))
            });
            for (sym, cmd) in [
                (0x0067_a530, 0x42),
                (0x0067_a550, 0x43),
                (0x0067_a590, 0x45),
                (0x0067_a5b0, 0x44),
                (0x0067_a6d0, 0xc),
            ] {
                t.check(
                    &format!("i set1 {sym:#x} #{i}"),
                    sym,
                    Conv::Cdecl,
                    &[x],
                    |m| Out::U32(b::i_set1(m, cmd, x)),
                );
            }
            for (sym, cmd) in [(0x0067_a570, 0), (0x0067_a6f0, 0xb)] {
                t.check(
                    &format!("i set2 {sym:#x} #{i}"),
                    sym,
                    Conv::Cdecl,
                    &[x, y],
                    |m| Out::U32(b::i_set2(m, cmd, x, y)),
                );
            }
        }
    });
}

#[test]
fn voice_ctor_matches_the_original() {
    Twins::each(|t| {
        for case in 0..4 {
            t.reset();
            let this = t.alloc(&[0xcd; 0x90]);
            let info = t.zeros(0x34);
            let block = t.zeros(0x10);
            let seq = [
                if case & 1 == 0 { info } else { 0 },
                if case & 2 == 0 { block } else { 0 },
            ];
            t.stub_seq(b::OPERATOR_NEW, false, 1, 0, &seq);
            let bufs = [t.alloc(&[0xee; 0x54]), t.alloc(&[0xdd; 0x54])];
            t.stub_seq(b::USER_MALLOC, false, 1, 0, &bufs);
            t.stub(b::EVENT_INFO_INIT, true, 0, 0, 0x4444);
            t.stub(b::BLOCK_INIT, true, 0, 0, 0x5555);
            t.check(
                &format!("FmodVoice ctor #{case}"),
                0x0070_a1a0,
                Conv::Thiscall(this),
                &[],
                |m| Out::U32(b::voice_ctor(m, this)),
            );
        }
    });
}

#[test]
fn handle_close_matches_the_original() {
    Twins::each(|t| {
        t.reset();
        let this = t.words(&[0x777, 1]);
        t.stub_import(b::CLOSE_HANDLE_IAT, false, 1, 1, 0x1f);
        t.check("0x006f9f60", 0x006f_9f60, Conv::Thiscall(this), &[], |m| {
            Out::U32(b::handle_close(m, this))
        });
    });
}

#[test]
fn music_and_voice_dtors_match_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x0bad_cafe);
        for case in 0..16u32 {
            t.reset();
            let mut w: Vec<u32> = (0..0x24).map(|_| rng.next()).collect();
            // which of the freed fields are set
            for (k, off) in [0x10usize, 0x14, 0x18, 0x24, 0x28, 0xc]
                .into_iter()
                .enumerate()
            {
                if case >> (k % 4) & 1 == 1 {
                    w[off / 4] = 0;
                }
            }
            if case & 8 != 0 {
                w[0x84 / 4] = 0;
            }
            let this = t.words(&w);
            t.stub(b::USER_FREE, false, 1, 0, 0x11);
            t.stub(b::OPERATOR_DELETE, false, 1, 0, 0x22);
            t.stub(b::BLOCK_CLEAR, true, 0, 0, 0x33);
            t.check(
                &format!("FmodMusic dtor #{case}"),
                0x0070_86b0,
                Conv::Thiscall(this),
                &[],
                |m| {
                    b::music_dtor(m, this);
                    Out::None
                },
            );
            let this = t.words(&w);
            t.check(
                &format!("FmodVoice dtor #{case}"),
                0x0070_a330,
                Conv::Thiscall(this),
                &[],
                |m| {
                    b::voice_dtor(m, this);
                    Out::None
                },
            );
        }
    });
}

/// A tree header (`0x30` bytes) whose first-node word (`+0xc`) is
/// `first`, and a `Phyre_PRedBlackTree_remove` stub that moves it along
/// `after` (then the tree itself).
fn drained_tree(t: &mut Twins, first: u32, after: &[u32]) -> u32 {
    let tree = t.zeros(0x30);
    t.w32(tree + 0xc, if first == u32::MAX { tree } else { first });
    let mut seq: Vec<u32> = after.to_vec();
    seq.push(tree);
    seq.push(tree);
    let code = t.seq_code(b::TREE_REMOVE, true, 1, 1, &seq, Some(tree + 0xc));
    t.jump(b::TREE_REMOVE, code);
    tree
}

/// A node with a buffer word at `+0x18` (`0x20` bytes).
fn node(t: &mut Twins, buf: u32) -> u32 {
    t.words(&[0, 0, 0, 0, 0, 0, buf, 0])
}

#[test]
fn sfx_node_tree_dtor_matches_the_original() {
    Twins::each(|t| {
        for case in 0..4 {
            t.reset();
            let n1 = node(t, 0x1000);
            let n2 = node(t, 0);
            let n3 = node(t, 0x3000);
            let (first, after): (u32, Vec<u32>) = match case {
                0 => (u32::MAX, vec![]),
                1 => (0, vec![]),
                2 => (n1, vec![]),
                _ => (n1, vec![n2, n3]),
            };
            let tree = drained_tree(t, first, &after);
            t.stub(b::USER_FREE, false, 1, 0, 0);
            t.stub(b::FREE_LIST_RESET, true, 0, 0, 0);
            t.stub(b::FREE_LIST_DTOR, true, 0, 0, 0);
            t.stub(b::TREE_CLEAR, true, 0, 0, 0);
            t.check(
                &format!("0x0070c3c0 #{case}"),
                0x0070_c3c0,
                Conv::Thiscall(tree),
                &[],
                |m| {
                    b::sfx_node_tree_dtor(m, tree);
                    Out::None
                },
            );
        }
    });
}

/// An `FmodSfx` (`0xd4` bytes) whose tree at `off` holds `nodes` in order
/// (linked for the left-most walk: `+0xc` -> the last ... -> the first,
/// each `+0` the next to the left, the first's `+0` the tree), with the
/// successor stub answering them in order. `fill(node index)` sets each
/// node's fields. Returns the object.
fn sfx_with_tree(t: &mut Twins, off: u32, nodes: &[u32], deep: bool) -> u32 {
    let sfx = t.zeros(0xd4);
    let tree = sfx + off;
    if nodes.is_empty() {
        t.w32(tree + 0xc, tree);
    } else {
        // the walk from +0xc: the last node, then along +0 to the first
        let mut chain: Vec<u32> = if deep {
            nodes.iter().rev().copied().collect()
        } else {
            vec![nodes[0]]
        };
        if !deep && nodes.len() > 1 {
            chain = vec![nodes[nodes.len() - 1], nodes[0]];
        }
        t.w32(tree + 0xc, chain[0]);
        for w in chain.windows(2) {
            t.w32(w[0], w[1]);
        }
        let first = *chain.last().unwrap();
        t.w32(first, tree);
    }
    let mut succ: Vec<u32> = nodes.iter().skip(1).copied().collect();
    succ.push(tree);
    succ.push(tree);
    t.stub_seq(b::TREE_SUCCESSOR, true, 1, 1, &succ);
    sfx
}

#[test]
fn sfx_stop_all_sound_matches_the_original() {
    Twins::each(|t| {
        let mut rng = Rng(0x5f5f_1234);
        for case in 0..6u32 {
            t.reset();
            let count = [0, 1, 2, 3, 2, 1][case as usize];
            let mut nodes = Vec::new();
            for k in 0..count {
                let n_events = [0, 1, 3][((case + k) % 3) as usize];
                let ev: Vec<u32> = (0..n_events)
                    .map(|e| {
                        if (case + e) % 3 == 0 {
                            0
                        } else {
                            0x4000 + 0x10 * e
                        }
                    })
                    .collect();
                let arr = t.words(&if ev.is_empty() { vec![0] } else { ev });
                let n = t.words(&[0, 0, 0, 0, n_events, 0, arr, 0]);
                nodes.push(n);
            }
            let sfx = sfx_with_tree(t, 0x64, &nodes, case % 2 == 0);
            t.w32(sfx + 0xcc, 0x9999);
            // the drain: the tree's +0xc moves through the nodes
            let tree = sfx + 0x64;
            let mut seq: Vec<u32> = nodes.iter().skip(1).copied().collect();
            seq.extend([tree, tree]);
            let code = t.seq_code(b::TREE_REMOVE, true, 1, 1, &seq, Some(tree + 0xc));
            t.jump(b::TREE_REMOVE, code);
            t.stub_import(b::WAIT_IAT, false, 2, 2, 0);
            t.stub_import(b::RELEASE_MUTEX_IAT, false, 1, 1, rng.word());
            t.stub(b::SFX_EVENT_STOP, true, 1, 1, 0);
            t.stub(b::SFX_EVENT_DTOR, true, 0, 0, 0);
            t.stub(b::OPERATOR_DELETE, false, 1, 0, 0);
            t.stub(b::USER_FREE, false, 1, 0, 0);
            t.stub(b::FREE_LIST_RESET, true, 0, 0, 0);
            let immediate = case & 1;
            t.check(
                &format!("FmodSfx_stopAllSound #{case}"),
                0x0070_da90,
                Conv::Thiscall(sfx),
                &[immediate],
                |m| Out::U32(b::sfx_stop_all_sound(m, sfx, immediate)),
            );
        }
    });
}

/// `stop` clearing (or replacing) the slot it was called for: the
/// original reads the slot again before destroying the event.
#[test]
fn sfx_stop_all_sound_rereads_the_slot() {
    Twins::each(|t| {
        for replace in [0, 0x4440] {
            t.reset();
            let arr = t.words(&[0x4400]);
            let n = t.words(&[0, 0, 0, 0, 1, 0, arr, 0]);
            let sfx = sfx_with_tree(t, 0x64, &[n], false);
            let tree = sfx + 0x64;
            let code = t.seq_code(b::TREE_REMOVE, true, 1, 1, &[tree, tree], Some(tree + 0xc));
            t.jump(b::TREE_REMOVE, code);
            let stop = t.seq_code(b::SFX_EVENT_STOP, true, 1, 1, &[replace], Some(arr));
            t.jump(b::SFX_EVENT_STOP, stop);
            t.stub_import(b::WAIT_IAT, false, 2, 2, 0);
            t.stub_import(b::RELEASE_MUTEX_IAT, false, 1, 1, 1);
            t.stub(b::SFX_EVENT_DTOR, true, 0, 0, 0);
            t.stub(b::OPERATOR_DELETE, false, 1, 0, 0);
            t.stub(b::USER_FREE, false, 1, 0, 0);
            t.stub(b::FREE_LIST_RESET, true, 0, 0, 0);
            t.check(
                &format!("FmodSfx_stopAllSound reread {replace:#x}"),
                0x0070_da90,
                Conv::Thiscall(sfx),
                &[1],
                |m| Out::U32(b::sfx_stop_all_sound(m, sfx, 1)),
            );
        }
    });
}

#[test]
fn sfx_bank_stop_all_events_matches_the_original() {
    Twins::each(|t| {
        for (case, immediate) in [0u32, 1].into_iter().enumerate() {
            t.reset();
            let mut vt = vec![0u32; 9];
            vt.push(t.stub_code(0x5124, false, 2, 2, 0x8));
            let vt = t.words(&vt);
            let project = t.words(&[vt, 0]);
            let bank = t.words(&[if case == 0 { 0 } else { project }, 0]);
            t.check(
                &format!("SfxBank_stopAllEvents #{case}"),
                0x0070_fcd0,
                Conv::Thiscall(bank),
                &[immediate],
                |m| {
                    b::sfx_bank_stop_all_events(m, bank, immediate);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn sfx_shutdown_matches_the_original() {
    Twins::each(|t| {
        for case in 0..8u32 {
            t.reset();
            let count = [0, 1, 3, 2][(case % 4) as usize];
            let nodes: Vec<u32> = (0..count)
                .map(|k| {
                    let bank = if (case + k) % 2 == 0 { 0x7000 + k } else { 0 };
                    t.words(&[0, 0, 0, 0, bank, 0])
                })
                .collect();
            let sfx = sfx_with_tree(t, 0x94, &nodes, case < 4);
            t.w32(sfx + 0xc8, if case & 1 == 0 { 0x8800 } else { 0 });
            t.w32(sfx + 0xc4, if case & 2 == 0 { 0x8900 } else { 0 });
            t.stub(b::SFX_STOP_ALL_SOUND, true, 1, 1, 0);
            t.stub(b::SFX_UNLOAD_PROJECTS, true, 2, 2, 0);
            t.stub(b::SFX_BANK_DTOR, true, 0, 0, 0);
            t.stub(b::OPERATOR_DELETE, false, 1, 0, 0);
            t.stub(b::TREE_ORPHAN, true, 0, 0, 0);
            t.stub(b::FREE_LIST_RESET, true, 0, 0, 0);
            t.stub(b::SFX_EVENT_STOP, true, 1, 1, 0);
            t.stub(b::SFX_EVENT_DTOR, true, 0, 0, 0);
            t.stub(b::SFX_BANK_STOP_ALL, true, 1, 1, 0);
            t.check(
                &format!("FmodSfx_shutdown #{case}"),
                0x0070_d780,
                Conv::Thiscall(sfx),
                &[],
                |m| {
                    b::sfx_shutdown(m, sfx);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn sfx_dtor_matches_the_original() {
    Twins::each(|t| {
        t.reset();
        let sfx = t.zeros(0xd4);
        t.w32(sfx + 0xcc, 0x4321);
        t.stub(b::SFX_SHUTDOWN, true, 0, 0, 0);
        t.stub_import(b::CLOSE_HANDLE_IAT, false, 1, 1, 1);
        t.stub(b::TREE_ORPHAN, true, 0, 0, 0);
        t.stub(b::FREE_LIST_RESET, true, 0, 0, 0);
        t.stub(b::FREE_LIST_DTOR, true, 0, 0, 0);
        t.stub(b::TREE_CLEAR, true, 0, 0, 0);
        t.stub(b::SFX_NODE_TREE_DTOR, true, 0, 0, 0);
        t.check("FmodSfx dtor", 0x0070_c4a0, Conv::Thiscall(sfx), &[], |m| {
            b::sfx_dtor(m, sfx);
            Out::None
        });
    });
}

#[test]
fn ffx_fmod_dtor_matches_the_original() {
    Twins::each(|t| {
        for case in 0..8u32 {
            t.reset();
            let w = [
                1,
                2,
                3,
                if case & 1 == 0 { 0x1100 } else { 0 },
                if case & 2 == 0 { 0x2200 } else { 0 },
                if case & 4 == 0 { 0x3300 } else { 0 },
                7,
            ];
            let this = t.words(&w);
            t.stub(b::MUSIC_DTOR, true, 0, 0, 0);
            t.stub(b::VOICE_DTOR, true, 0, 0, 0);
            t.stub(b::SFX_DTOR, true, 0, 0, 0);
            t.stub(b::OPERATOR_DELETE, false, 1, 0, 0);
            t.check(
                &format!("FfxFmod dtor #{case}"),
                0x0070_6590,
                Conv::Thiscall(this),
                &[],
                |m| {
                    b::ffx_fmod_dtor(m, this);
                    Out::None
                },
            );
        }
    });
}

#[test]
fn snapshot_globals_are_readable() {
    // the snapshots hold the game's own manager; the entry points above
    // replace it with a synthetic one, so check the real one is there
    Twins::each(|t| {
        if t.snapshot {
            let m: u32 = Mem::read(&t.a, t.rebase(FMOD_MANAGER));
            assert_ne!(m, 0, "the snapshot has an FmodManager");
        }
    });
}
