//! `PhyreEngine`'s scripting support outside the script layer, list
//! `phyre-scripting` (`ffx_pother_a::m5p_scripting*`, M5 stage 1), against
//! the original. None of this code ran in a coverage run, so every case is
//! synthetic memory built for its branch, on the image alone and over a
//! snapshot (`FFX_SNAPSHOT`, run once with each of field.snap and
//! battle.snap). Results and every byte written must agree. Needs
//! `FFX_GAME_DIR`.
//!
//! Callees outside the list (Lua's C API, `PhyreFree`, the type
//! dispenser, the stream file) are replaced in both oracles alike by
//! recording stubs: each call appends its tag, `ecx` and its stack words
//! to a log in memory, so the diff checks that both made the same calls
//! with the same words in the same order. Stubs pop like the originals.

#![allow(
    clippy::cast_possible_truncation,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::unreadable_literal
)]

mod m5p_scripting_twins;

use ffx_pother_a::m5p_scripting_lua::{self as lb, bridge as lua};
use ffx_pother_a::m5p_scripting_mid::{self as mid, bridge as m};
use ffx_pother_a::m5p_scripting_more::{self as mo_, bridge as mo};
use ffx_pother_a::m5p_scripting_run::{self as ru_, bridge as ru};
use ffx_pother_a::m5p_scripting_sched::{self as sc_, bridge as sc};
use ffx_pother_a::m5p_scripting_small::{self as s, bridge as b};
use ffx_pother_a::m5p_scripting_state::{self as st_, bridge as st};
use ffx_pother_a::m5p_scripting_vec::{self as ve_, bridge as ve};
use m5p_scripting_twins::{JUNK, Rng, Twins};

/// Floats for the copies through the x87: signs, fractions, huge, tiny,
/// infinities, quiet NaNs with payloads (no signalling NaNs: the emulator
/// keeps them signalling, the CPU quiets them).
const FLOATS: [u32; 10] = [
    0x0000_0000,
    0x8000_0000,
    0x3f80_0000,
    0xbfc0_0000,
    0x7149_f2ca,
    0x0000_0001,
    0x7f80_0000,
    0xff80_0000,
    0x7fc0_0000,
    0xffc1_2345,
];

#[test]
fn b1_destructors_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(s::DESCRIPTOR_DESTRUCT, true, 0, 0, 0xde57);
        for d in &s::TAIL_DTORS {
            let obj = t.junk(0x40, 3);
            let what = format!("{label}: tail dtor {:#x}", d.addr);
            t.this(&what, d.addr, obj, &[], |g| Some(b::tail_dtor(g, d, obj)));
            assert_eq!(t.logged(), 1, "{what}");
        }
        t.stub(s::OPERATOR_DELETE, false, 1, 0, 0x0de1);
        t.stub(s::PHYRE_FREE, false, 1, 0, 0x0de2);
        t.stub(0x005c_e550, true, 0, 0, 0x0de3);
        t.stub(0x00a8_da90, true, 0, 0, 0x0de4);
        t.stub(0x004f_a470, true, 0, 0, 0x0de5);
        t.stub(s::BATCH_UNLINK, true, 1, 1, 0x0de6);
        for d in &s::DELETING_DTORS {
            for flags in [0u32, 1, 2, 3, 0xffff_fffe] {
                for batch in [0u32, 0x00c9_5678] {
                    let obj = t.junk(0x40, 7);
                    t.w32(obj + 0x10, batch);
                    let what = format!(
                        "{label}: deleting dtor {:#x} flags {flags:#x} batch {batch:#x}",
                        d.addr
                    );
                    t.this(&what, d.addr, obj, &[flags], |g| {
                        Some(b::deleting_dtor(g, d, obj, flags))
                    });
                }
            }
        }
        t.stub(s::OPERATOR_DELETE_2, false, 1, 0, 0x0de7);
        for p in [0u32, 0x1234_5678] {
            let what = format!("{label}: delete forward {p:#x}");
            t.cdecl(&what, 0x00a8_d8b4, &[p], |g| Some(b::delete_forward(g, p)));
        }
        // the array destroyers: null, empty, three objects
        for &f in &s::ARRAY_DESTROYERS {
            for n in [0u32, 1, 3] {
                for null in [false, true] {
                    let array = if null {
                        0
                    } else {
                        let a = t.zeros(0x20);
                        for i in 0..3 {
                            let o = t.object(0x7100 + 0x10 * i, &[1], &[0x55]);
                            let vt = t.r32(o);
                            t.w32(a + 8 * i, vt);
                            t.w32(a + 8 * i + 4, 0x4444_0000 + i);
                        }
                        a
                    };
                    let what = format!("{label}: destroy array {f:#x} n {n} null {null}");
                    t.cdecl(&what, f, &[array, n], |g| {
                        b::destroy_array(g, array, n);
                        None
                    });
                }
            }
        }
        // the pair array release
        t.stub(s::FREE_PAIR_ARRAY, false, 2, 0, 0x0fa1);
        for head in [0u32, 3, 0x8000_0003, 0x7fff_ffff] {
            let arr = t.zeros(8);
            t.w32(arr, head);
            t.w32(arr + 4, 0x00c9_1000);
            let what = format!("{label}: pair array release {head:#x}");
            t.this(&what, 0x005c_f2a0, arr, &[], |g| {
                Some(b::pair_array_release(g, arr))
            });
        }
        t.unstub();
    }
}

#[test]
fn b1_small_methods_match_the_original() {
    for (label, mut t) in Twins::all() {
        let obj = t.junk(0x100, 9);
        t.this(
            &format!("{label}: 5215f0"),
            0x0052_15f0,
            obj,
            &JUNK[..1],
            |_| Some(0),
        );
        t.this(
            &format!("{label}: 6e1f40"),
            0x006e_1f40,
            obj,
            &JUNK[..2],
            |_| Some(0),
        );
        for v in [0u32, 1, 0x8000_0000] {
            let what = format!("{label}: setters {v:#x}");
            t.this(&what, 0x0055_34f0, obj, &[v], |g| {
                Some(b::set_14(g, obj, v))
            });
            t.this(&what, 0x0055_4a70, obj, &[v], |g| {
                Some(b::set_14(g, obj, v))
            });
            t.this(&what, 0x0055_34d0, obj, &[v], |g| {
                Some(b::set_nonnull(g, obj, 0x18, v))
            });
            t.this(&what, 0x0055_3500, obj, &[v], |g| {
                Some(b::set_nonnull(g, obj, 0x1c, v))
            });
        }
        for m in 0..8u32 {
            let [x, y, z] = [m & 1, m & 2, m & 4].map(|k| if k == 0 { 0 } else { 0x00c9_0000 + k });
            let what = format!("{label}: scriptable initialize {m}");
            t.this(&what, 0x0055_31d0, obj, &[x, y, z], |g| {
                Some(b::scriptable_initialize(g, obj, x, y, z))
            });
        }
        t.this(&format!("{label}: clear"), 0x0055_3590, obj, &[], |g| {
            b::clear_script(g, obj);
            None
        });
        let arr = t.junk(0x50, 2);
        t.this(
            &format!("{label}: aligned init"),
            0x00a8_e620,
            arr,
            &[],
            |g| Some(b::aligned_array_init(g, arr)),
        );
        let mgr = t.junk(0x50, 4);
        t.this(
            &format!("{label}: island ctor"),
            0x00a8_da40,
            mgr,
            &[],
            |g| Some(b::island_manager_ctor(g, mgr)),
        );
        // va_arg
        let args = t.junk(0x20, 5);
        let va = t.zeros(4);
        t.w32(va, args);
        for i in 0..3 {
            let what = format!("{label}: va_arg {i}");
            t.cdecl(&what, 0x005c_fdc0, &[va], |g| Some(b::va_arg_word(g, va)));
        }
        // the sort predicate
        let x = t.zeros(0x10);
        let y = t.zeros(0x10);
        for (p, q) in [(1u32, 2u32), (2, 1), (5, 5), (0x8000_0000, 1)] {
            t.w32(x + 0xc, p);
            t.w32(y + 0xc, q);
            let what = format!("{label}: less {p:#x} {q:#x}");
            t.cdecl(&what, 0x004d_3bd0, &[x, y], |g| Some(b::less_c(g, x, y)));
        }
        // the bounded tables
        for &(f, table) in &s::TABLE_GETTERS {
            for i in [0u32, 1, 0x1f, 0x20, 0x21, 0xffff_ffff] {
                let what = format!("{label}: table {f:#x}[{i:#x}]");
                t.cdecl(&what, f, &[i], |g| Some(b::table_get(g, table, i)));
            }
        }
        // ring lists of 0..4 nodes
        for n in 0..5u32 {
            let list = t.zeros(4 + 8 * 4);
            let mut prev = list;
            for i in 0..n {
                let node = list + 4 + 8 * i;
                t.w32(prev, node);
                prev = node;
            }
            t.w32(prev, list);
            let what = format!("{label}: ring {n}");
            t.cdecl(&what, 0x0043_61d0, &[list], |g| {
                Some(b::ring_count(g, list))
            });
        }
        // the swizzle letters: every byte, with junk above it
        for c in 0..=0xffu32 {
            for hi in [0u32, 0xabcd_ef00] {
                let w = c | hi;
                let what = format!("{label}: swizzle {w:#x}");
                t.cdecl(&what, 0x0057_10b0, &[w], |_| Some(b::swizzle_index(w)));
            }
        }
        // PString compare: tagged pointers, equal, shorter, signed bytes
        let words = [
            ("abc", 1u32),
            ("abc", 0),
            ("abd", 0),
            ("ab", 1),
            ("", 0),
            ("\u{e9}x", 1),
            ("ab\u{7f}", 0),
        ];
        let mut ps = Vec::new();
        for (str_, tag) in words {
            let s_ = t.cstr(str_);
            let cell = t.zeros(4);
            t.w32(cell, s_ + tag);
            ps.push(cell);
        }
        for &p in &ps {
            for &q in &ps {
                let what = format!("{label}: compare {p:#x} {q:#x}");
                t.cdecl(&what, 0x005d_6760, &[p, q], |g| {
                    Some(b::string_compare(g, p, q))
                });
            }
        }
        // the name getters
        for &f in &s::NAME_GETTERS {
            for str_ in ["", "a", "Phyre::PScene"] {
                let p = t.zeros(8);
                let sp = t.cstr(str_);
                t.w32(p + 4, sp);
                let out = t.junk(8, 1);
                let what = format!("{label}: name {f:#x} {str_:?}");
                t.cdecl(&what, f, &[p, out, out + 4], |g| {
                    Some(b::name_get(g, p, out, out + 4))
                });
            }
        }
        // first of the context
        for ctx in [0u32, 1] {
            let o = t.zeros(0x10);
            if ctx != 0 {
                let c = t.junk(8, 3);
                t.w32(o + 8, c);
            }
            let what = format!("{label}: first of {ctx}");
            t.this(&what, 0x00a1_61b0, o, &[], |g| Some(b::first_of(g, o)));
        }
        // the PArray entry getters
        let pa = t.zeros(8);
        let data = t.zeros(0x10);
        let e1 = t.junk(8, 6);
        t.w32(data, e1);
        t.w32(data + 8, e1 + 4);
        t.w32(pa + 4, data);
        for head in [0u32, 3, 0x8000_0003] {
            t.w32(pa, head);
            for i in [0u32, 1, 2, 3, 0xffff_ffff] {
                let what = format!("{label}: entries {head:#x} {i}");
                t.this(&what, 0x00a1_6980, pa, &[i], |g| {
                    Some(b::entry_plus_4(g, pa, i))
                });
                t.this(&what, 0x00a1_69b0, pa, &[i], |g| {
                    Some(b::entry_word_4(g, pa, i))
                });
            }
        }
        // owners of the instances
        for n in [0u32, 3] {
            let p = t.zeros(0x40);
            let owner = t.zeros(0x20);
            t.w32(owner + 0x1c, 0xc);
            let recs = t.junk(0x40, 8);
            t.w32(p + 0x3c, n);
            t.w32(p + 0x30, recs);
            t.w32(p + 0x2c, owner);
            let what = format!("{label}: set owner {n}");
            t.cdecl(&what, 0x0044_db10, &[p], |g| Some(b::set_owner_all(g, p)));
        }
        // the counters
        for (start, step) in [
            (0u32, 1u32),
            (5, 7),
            (0x100, 1),
            (3, 0),
            (0xff, 1),
            (0x10, 0x30),
            (0xffff_ffff, 1),
        ] {
            let o = t.junk(0x300, 2);
            let what = format!("{label}: count every {start:#x} {step:#x}");
            t.this(&what, 0x005d_0e90, o, &[start, step], |g| {
                Some(b::count_every(g, o, start, step))
            });
        }
        // pop if last
        for (has_array, has_elem, n, last) in [
            (false, true, 1, true),
            (true, false, 1, true),
            (true, true, 0, true),
            (true, true, 3, true),
            (true, true, 3, false),
            (true, false, 1, false),
        ] {
            let array = t.zeros(0x40);
            let entries = t.zeros(0x60);
            t.w32(array + 0x2c, entries);
            t.w32(array + 0x30, n);
            if !has_elem && !last {
                // a null array start: the last entry is at 0, as the null element
                t.w32(array + 0x2c, 0);
            }
            let elem = if last {
                entries + 20 * n.max(1) - 20
            } else {
                entries
            };
            let o = t.zeros(8);
            t.w32(o, if has_array { array } else { 0 });
            t.w32(o + 4, if has_elem { elem } else { 0 });
            let what = format!("{label}: pop if last {has_array} {has_elem} {n} {last}");
            t.this(&what, 0x005c_f3f0, o, &[], |g| {
                b::pop_if_last(g, o);
                None
            });
        }
        let cells = t.junk(0x20, 1);
        t.w32(cells + 8, cells + 0x10);
        let what = format!("{label}: unlink cell");
        t.this(
            &what,
            0x006e_21c0,
            obj,
            &[JUNK[0], cells, JUNK[1], JUNK[2], cells + 8],
            |g| Some(b::unlink_cell(g, cells, cells + 8)),
        );
        t.unstub();
    }
}

#[test]
fn b1_forwarders_match_the_original() {
    for (label, mut t) in Twins::all() {
        let obj = t.junk(0x40, 1);
        t.stub(s::STREAM_CLOSE, true, 0, 0, 0xc105);
        t.stub(s::STREAM_IS_OPEN, true, 0, 0, 0x0000_0101);
        t.stub(s::STREAM_WRITE, true, 2, 2, 0x5717);
        t.this(&format!("{label}: close"), 0x009f_08a0, obj, &[], |g| {
            Some(b::close_script_context(g, obj))
        });
        t.this(&format!("{label}: is open"), 0x009f_08b0, obj, &[], |g| {
            Some(b::stream_is_open(g, obj))
        });
        t.this(
            &format!("{label}: write"),
            0x009f_0900,
            obj,
            &JUNK[..2],
            |g| Some(b::stream_write(g, obj, JUNK[0], JUNK[1])),
        );
        t.stub(s::STRING_ASSIGN, true, 1, 1, 0x0a55);
        for r in [0u32, 0x00c9_0001] {
            t.stub(s::STREAM_OPEN, true, 5, 5, r);
            let what = format!("{label}: open {r:#x}");
            t.this(&what, 0x009f_08c0, obj, &[0x00c9_0100], |g| {
                Some(b::stream_writer_open(g, obj, 0x00c9_0100))
            });
        }
        for (r1, r3) in [
            (0x1234_5600u32, 0u32),
            (0x1234_5601, 0x0042_4200),
            (0x1234_5601, 0x0042_4201),
        ] {
            for len in [0u32, 7, 8] {
                let w = t.object(0x7000, &[0, 0, 0, 1], &[0, r1, 0, r3]);
                t.w32(w + 8, len);
                t.w32(w + 0xc, 0x5555);
                let what = format!("{label}: flush {r1:#x} {r3:#x} {len}");
                t.this(&what, 0x009f_0910, w, &[], |g| {
                    Some(b::stream_writer_flush(g, w))
                });
            }
        }
        let o10 = t.object(
            0x7200,
            &[0; 10].iter().copied().chain([1]).collect::<Vec<_>>(),
            &[],
        );
        t.cdecl(
            &format!("{label}: construct through"),
            0x0043_b9d0,
            &[o10, 0x77],
            |g| Some(b::construct_through(g, o10, 0x77)),
        );
        for set in [false, true] {
            let o = t.zeros(0x20);
            if set {
                let p = t.object(0x7300, &[0, 1], &[0, 0x6789]);
                t.w32(o + 0x14, p);
            }
            let what = format!("{label}: forward 14 {set}");
            t.this(&what, 0x0043_7320, o, &[0x99], |g| {
                Some(b::forward_14(g, o, 0x99))
            });
        }
        t.stub(s::GLOBAL_NAMESPACE, false, 1, 0, 0x00c9_0b00);
        t.stub(s::REGISTER_TYPES, false, 3, 0, 0x7e9);
        t.cdecl(
            &format!("{label}: register namespace"),
            0x0043_6f70,
            &[0x1100],
            |g| Some(b::register_phyre_namespace(g, 0x1100)),
        );
        t.stub(s::STAGE_STREAMS, true, 3, 3, 0x57a6);
        let cloth = t.zeros(0x40);
        let mesh = t.object(0x7400, &[0, 0, 0, 0, 0, 0, 3], &[]);
        t.w32(cloth + 0x1c, mesh);
        t.this(
            &format!("{label}: cloth"),
            0x006e_2190,
            cloth,
            &[1, 2, 3],
            |g| Some(b::cloth_stage_and_draw(g, cloth, 1, 2, 3)),
        );
        t.stub(s::SCHEDULER_ADD, true, 5, 5, 0xadd);
        t.stub(s::SCHEDULER_INSERT, true, 6, 6, 0x1257);
        let a1 = t.junk(0x10, 4);
        t.this(
            &format!("{label}: scheduler add"),
            0x005d_0260,
            obj,
            &[1, a1, 3, 4],
            |g| Some(b::scheduler_add(g, obj, [1, a1, 3, 4])),
        );
        t.this(
            &format!("{label}: scheduler insert"),
            0x005d_0370,
            obj,
            &[1, 2, 3, 4, 5],
            |g| Some(b::scheduler_insert(g, obj, [1, 2, 3, 4, 5])),
        );
        // the print hook: unset, then a stdcall stub
        let hook = t.rebase(s::PRINT_HOOK);
        t.w32(hook, 0);
        t.cdecl(
            &format!("{label}: print hook unset"),
            0x005d_6050,
            &[0x42],
            |g| Some(b::print_hook(g, 0x42)),
        );
        let f = t.stub_code(0x7500, false, 1, 1, 0x1111);
        t.w32(hook, f);
        t.cdecl(&format!("{label}: print hook"), 0x005d_6050, &[0x42], |g| {
            Some(b::print_hook(g, 0x42))
        });
        t.w32(hook, 0);
        t.stub(s::MEMCPY, false, 3, 0, 0x3e3c);
        for n in [5u32, 0x8000_0005] {
            let o = t.junk(0x20, 3);
            t.w32(o + 8, n);
            let what = format!("{label}: copy block {n:#x}");
            t.this(&what, 0x0061_8130, o, &[], |g| Some(b::copy_block(g, o)));
        }
        // the async header: not run, running, done
        for state in [0u32, 1, 2, 3] {
            let h = t.junk(0x20, 1);
            let f = t.stub_code(0x7600, false, 1, 0, 0xf00d);
            t.w32(h + 0xc, f);
            t.w32(h + 0x10, state);
            let what = format!("{label}: async process {state}");
            t.this(&what, 0x0061_7f30, h, &[], |g| Some(b::async_process(g, h)));
            // the wait: Sleep finishes the job (state 3)
            t.w32(h + 0x10, state);
            let mut pre = vec![0xc7, 0x05];
            pre.extend((h + 0x10).to_le_bytes());
            pre.extend(3u32.to_le_bytes()); // mov dword [state], 3
            let sleep = t.stub_code_with(&pre, s::SLEEP, false, 1, 0, 0);
            t.place(s::SLEEP, sleep);
            let out = t.zeros(8);
            let what = format!("{label}: async wait {state}");
            t.this(&what, 0x0061_7e40, h, &[out], |g| {
                Some(b::async_wait(g, h, out))
            });
        }
        // the event forward through the x87
        for &x in &FLOATS {
            let o = t.object(0x7700, &[0, 3], &[]);
            let p = t.zeros(0xc);
            t.w32(p, o);
            t.w32(p + 4, x);
            t.w32(p + 8, x ^ 0x8000_0000);
            let what = format!("{label}: event forward {x:#x}");
            t.cdecl(&what, 0x0052_4850, &[0x31, p], |g| {
                Some(b::event_forward(g, 0x31, p))
            });
        }
        t.stub(s::CREATE_THREAD, true, 1, 1, 0x7d);
        let c = t.junk(0x40, 2);
        t.this(
            &format!("{label}: scripted initialize"),
            0x0055_4830,
            c,
            &[0, 5, 6],
            |g| Some(b::scripted_initialize(g, c, 0, 5, 6)),
        );
        // FXAA: no context, an empty one, init failing and succeeding
        for (ctx, first, r) in [
            (false, 0u32, 0u32),
            (true, 0, 0),
            (true, 0x00c9_0200, 0),
            (true, 0x00c9_0200, 1),
        ] {
            t.stub(s::FXAA_INIT_SHADERS, true, 2, 2, r);
            let fx = t.junk(0x20, 6);
            if ctx {
                let cx = t.zeros(8);
                t.w32(cx, first);
                t.w32(fx + 8, cx);
            } else {
                t.w32(fx + 8, 0);
            }
            let what = format!("{label}: fxaa {ctx} {first:#x} {r}");
            t.this(&what, 0x00a1_b780, fx, &[], |g| Some(b::fxaa_init(g, fx)));
        }
        // find enabled: none, wrong class, says no, says yes
        let class = t.rebase(s::POSTFX_CLASS);
        let arr = t.zeros(8);
        let data = t.zeros(0x10);
        let objs: Vec<u32> = [(0x99u32, 1u32), (class, 0x100), (class, 0x101), (class, 1)]
            .iter()
            .enumerate()
            .map(|(i, &(cl, r))| {
                let o = t.object(0x7900 + 0x10 * i as u32, &[0, 0, 0, 0, 0], &[0, 0, 0, 0, r]);
                t.w32(o + 4, cl);
                o
            })
            .collect();
        for (i, &o) in objs.iter().enumerate() {
            t.w32(data + 4 * i as u32, o);
        }
        t.w32(arr + 4, data);
        for n in [0u32, 2, 3, 0x8000_0004] {
            t.w32(arr, n);
            let what = format!("{label}: find enabled {n:#x}");
            t.this(&what, 0x00a1_6100, arr, &[], |g| {
                Some(b::find_enabled(g, arr))
            });
        }
        t.stub(s::LUA_CLOSE, false, 1, 0, 0xc1);
        t.cdecl(
            &format!("{label}: close state"),
            0x005d_5450,
            &[0x1100],
            |g| Some(b::close_state(g, 0x1100)),
        );
        t.unstub();
    }
}

#[test]
fn b1_links_match_the_original() {
    for (label, mut t) in Twins::all() {
        // scene lists: this+8 holds the head; nodes {next, owner}
        for (owned, pos) in [(false, 0u32), (true, 0), (true, 1), (true, 2), (true, 9)] {
            let list = t.zeros(0x10);
            let nodes = t.zeros(0x30);
            for i in 0..3 {
                let n = nodes + 8 * i;
                t.w32(n, if i < 2 { n + 8 } else { 0 });
                t.w32(n + 4, list);
            }
            t.w32(list + 8, nodes);
            let node = if pos < 3 { nodes + 8 * pos } else { t.zeros(8) };
            t.w32(node + 4, if owned { list } else { 0x00c9_0100 });
            let what = format!("{label}: scene unlink {owned} {pos}");
            let ea = t.this(&what, 0x0050_7360, list, &[node], |g| {
                b::scene_unlink(g, list, node)
            });
            let _ = ea;
        }
        // an empty list
        let list = t.zeros(0x10);
        let node = t.zeros(8);
        t.w32(node + 4, list);
        t.this(
            &format!("{label}: scene unlink empty"),
            0x0050_7360,
            list,
            &[node],
            |g| b::scene_unlink(g, list, node),
        );
        t.stub(s::SCENE_UNLINK, true, 1, 1, 0x0ff);
        for owner in [0u32, 0x00c9_0300] {
            let list = t.junk(0x10, 2);
            let node = t.zeros(8);
            t.w32(node + 4, owner);
            let what = format!("{label}: scene link {owner:#x}");
            t.this(&what, 0x0050_6620, list, &[node], |g| {
                Some(b::scene_link(g, list, node))
            });
        }
        t.stub(s::ENTITY_UNLINK, true, 1, 1, 0x0ee);
        for (same, owner, ok) in [
            (true, 0, 1u32),
            (false, 0u32, 0x100),
            (false, 0, 1),
            (false, 0x00c9_0400, 1),
        ] {
            t.stub(s::ENTITY_LINK_CHECK, true, 1, 1, ok);
            let e = t.junk(0x10, 2);
            let node = t.zeros(0xc);
            t.w32(node + 4, if same { e } else { owner });
            t.w32(node + 8, 0x00c9_0500);
            let what = format!("{label}: entity link {same} {owner:#x} {ok:#x}");
            t.this(&what, 0x0044_fca0, e, &[node], |g| {
                Some(b::entity_link(g, e, node))
            });
        }
        t.unstub();
    }
}

#[test]
fn b1_lua_helpers_match_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        t.stub(s::LUA_CHECKSTACK, false, 2, 0, 0x0c5);
        t.stub(s::LUA_PUSHNIL, false, 1, 0, 0x0a11);
        t.stub(s::LUA_PUSHINTEGER, false, 2, 0, 0x1a7);
        t.stub(s::LUA_PUSHLIGHTUSERDATA, false, 2, 0, 0x119);
        t.stub(s::LUA_SETGLOBAL, false, 2, 0, 0x5e7);
        t.stub(s::LUA_GETGLOBAL, false, 2, 0, 0x6e7);
        t.stub(s::LUA_TOUSERDATA, false, 2, 0, 0x00c9_2200);
        t.stub(s::LUA_SETTOP, false, 2, 0, 0x7020);
        t.stub(s::LUA_GETFIELD, false, 3, 0, 0x6f1);
        t.stub(s::LUA_SETMETATABLE, false, 2, 0, 0x3e7a);
        t.stub(s::LUA_CREATETABLE, false, 3, 0, 0xc7ab);
        t.stub(s::LUA_SETTABLE, false, 2, 0, 0x5e7b);
        t.stub(s::MATRIX_COPY, true, 1, 1, 0x3c0);
        t.cdecl(
            &format!("{label}: set global"),
            0x005d_5330,
            &[l, 0x55],
            |g| Some(b::set_script_global(g, l, 0x55)),
        );
        t.cdecl(&format!("{label}: get pscript"), 0x005d_5780, &[l], |g| {
            Some(b::get_pscript(g, l))
        });
        for ud in [false, true] {
            let u = if ud { t.junk(0x40, 3) } else { 0 };
            t.stub(s::LUA_NEWUSERDATA, false, 2, 0, u);
            let what = format!("{label}: push object {ud}");
            t.cdecl(&what, 0x0043_6e80, &[l, 0x10, 0x20], |g| {
                Some(b::push_object(g, l, 0x10, 0x20))
            });
            let o = t.junk(0x10, 1);
            t.cdecl(&what, 0x0043_6ed0, &[l, o], |g| Some(b::push_self(g, l, o)));
            t.cdecl(&what, 0x0057_1030, &[l], |g| {
                Some(b::new_userdata(g, l, 0x40, s::STR_MATRIX))
            });
            t.cdecl(&what, 0x0057_1070, &[l], |g| {
                Some(b::new_userdata(g, l, 0x10, s::STR_VECTOR))
            });
            t.cdecl(&what, 0x0057_1f50, &[l, 0x44], |g| {
                Some(b::push_matrix(g, l, 0x44))
            });
        }
        t.stub(s::PUSH_OBJECT, false, 3, 0, 0x0b1);
        for p in [0u32, 0x00c9_3300] {
            let pp = t.zeros(4);
            t.w32(pp, p);
            let what = format!("{label}: push base {p:#x}");
            t.cdecl(&what, 0x0043_5580, &[l, pp], |g| {
                Some(b::push_base(g, l, pp))
            });
        }
        for n in [0u32, 1, 3] {
            let ty = t.object(0x7a00, &[0, 0, 0, 0, 0, 0, 2], &[]);
            let arr = t.junk(0x10, 2);
            let what = format!("{label}: push array {n}");
            t.cdecl(&what, 0x0043_6f20, &[l, n, arr, ty], |g| {
                Some(b::push_array(g, l, n, arr, ty))
            });
        }
        t.unstub();
    }
}

#[test]
fn b1_vararg_pushers_match_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        let mut rng = Rng(0x1234_5678);
        t.stub(s::LUA_CHECKSTACK, false, 2, 0, 0x0c5);
        t.stub(s::LUA_PUSHNIL, false, 1, 0, 0x0a11_0000);
        t.stub(s::LUA_PUSHINTEGER, false, 2, 0, 0x1a7_1a00);
        t.stub(s::LUA_PUSHNUMBER, false, 2, 0, 0x1b7_1b01);
        t.stub(s::PUSH_OBJECT, false, 3, 0, 0x0b1_0b00);
        for p in &s::VA_PUSHERS {
            let ty = 0x00c9_5000 + (p.addr & 0xfff0);
            t.stub(p.getter, false, 0, 0, ty);
            let want = if p.kind == s::VaKind::Object {
                ty | 1
            } else {
                ty
            };
            // the wrong type, then values from the va_list
            let what = format!("{label}: push {:#x} wrong type", p.addr);
            let va = t.zeros(4);
            t.cdecl(&what, p.addr, &[want ^ 0x10, va, l], |g| {
                Some(b::va_push(g, p, want ^ 0x10, va, l))
            });
            let words: Vec<u32> = match p.kind {
                s::VaKind::F64 => [
                    0.0f64,
                    -1.5,
                    3.4e38,
                    1e300,
                    1e-50,
                    f64::INFINITY,
                    f64::NAN,
                    0.1,
                ]
                .iter()
                .flat_map(|x| {
                    let b_ = x.to_bits();
                    [b_ as u32, (b_ >> 32) as u32]
                })
                .collect(),
                s::VaKind::Object => vec![0, 0x00c9_4400, 0, 0x00c9_4500],
                _ => (0..16)
                    .map(|i| match i {
                        0 => 0,
                        1 => 0xffff_ffff,
                        2 => 0x8000_0080,
                        3 => 0x7fff_7f7f,
                        _ => rng.next(),
                    })
                    .collect(),
            };
            let area = t.alloc(
                &words
                    .iter()
                    .flat_map(|w| w.to_le_bytes())
                    .collect::<Vec<_>>(),
            );
            t.w32(va, area);
            let n = match p.kind {
                s::VaKind::F64 | s::VaKind::I64 => words.len() / 2,
                _ => words.len(),
            };
            for i in 0..n {
                let what = format!("{label}: push {:#x} #{i}", p.addr);
                t.cdecl(&what, p.addr, &[want, va, l], |g| {
                    Some(b::va_push(g, p, want, va, l))
                });
            }
        }
        t.unstub();
    }
}

#[test]
fn b1_poppers_match_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        t.stub(s::LUA_SETTOP, false, 2, 0, 0x7020_7000);
        for p in &s::POPPERS {
            let ty = 0x00c9_6000 + (p.addr & 0xfff0);
            t.stub(p.getter, false, 0, 0, ty);
            let out = t.junk(0x10, 5);
            let what = format!("{label}: pop {:#x} wrong type", p.addr);
            t.cdecl(&what, p.addr, &[ty + 4, l, out], |g| {
                Some(b::pop(g, p, ty + 4, l, out))
            });
            let values: Vec<u32> = match p.kind {
                s::PopKind::F32 | s::PopKind::F64 => FLOATS.to_vec(),
                _ => vec![0, 1, 0xffff_ffff, 0x8000_0000, 0x1234_5678, 0x0000_ff80],
            };
            for &v in &values {
                match p.kind {
                    s::PopKind::F32 | s::PopKind::F64 => t.stub_float(s::LUA_TONUMBERX, 3, v),
                    s::PopKind::Bool => t.stub(s::LUA_TOBOOLEAN, false, 2, 0, v),
                    _ => t.stub(s::LUA_TOINTEGERX, false, 3, 0, v),
                }
                for o in [out, 0] {
                    let what = format!("{label}: pop {:#x} {v:#x} out {o:#x}", p.addr);
                    t.cdecl(&what, p.addr, &[ty, l, o], |g| Some(b::pop(g, p, ty, l, o)));
                }
            }
        }
        t.unstub();
    }
}

// ---- batch 2: middle-sized Lua glue and fixups ----

/// A descriptor's instance list for the `fixObjectsAfterLoading` steps:
/// `n` records of `stride` bytes; `+0x30` points `back` bytes into the
/// first. Returns (p, first record).
fn instance_list(t: &mut Twins, n: u32, stride: u32, back: u32) -> (u32, u32) {
    let p = t.zeros(0x40);
    let class = t.zeros(0x20);
    t.w32(class + 0x1c, stride);
    let recs = t.zeros((stride * n.max(1) + 0x20) as usize);
    t.w32(p + 0x2c, class);
    t.w32(p + 0x30, recs + back);
    t.w32(p + 0x3c, n);
    (p, recs)
}

#[test]
fn b2_lua_glue_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        t.stub(s::LUA_CHECKSTACK, false, 2, 0, 0x0c5);
        t.stub(s::LUA_PUSHINTEGER, false, 2, 0, 0x1a7);
        t.stub(s::LUA_GETFIELD, false, 3, 0, 0x6f1);
        t.stub(s::LUA_SETMETATABLE, false, 2, 0, 0x3e7a);
        t.stub(s::LUA_CREATETABLE, false, 3, 0, 0xc7ab);
        t.stub(s::LUA_SETTABLE, false, 2, 0, 0x5e7b);
        t.stub(s::LUA_SETTOP, false, 2, 0, 0x7020_7000);
        t.stub(s::LUA_SETGLOBAL, false, 2, 0, 0x5e7);
        t.stub(mid::LUA_GETTABLE, false, 2, 0, 0x6e7a);
        t.stub(mid::LUA_PUSHCCLOSURE, false, 3, 0, 0xcc10);
        t.stub(mid::LUA_REMOVE, false, 2, 0, 0x7e30);
        t.stub(mid::LUA_GETTOP, false, 1, 0, 3);
        t.stub(mid::PRINTF, false, 4, 0, 0x9f);
        t.stub(mid::LUA_TOLSTRING, false, 3, 0, 0x00c9_7700);
        // the vector userdata
        for ok in [false, true] {
            let u = if ok { t.junk(0x10, 2) } else { 0 };
            t.stub(s::LUA_NEWUSERDATA, false, 2, 0, u);
            let v = t.zeros(0x10);
            for (i, &f) in FLOATS[..4].iter().enumerate() {
                t.w32(v + 4 * i as u32, f ^ u32::from(!ok));
            }
            let what = format!("{label}: push vector {ok}");
            t.cdecl(&what, 0x0057_2150, &[l, v], |g| {
                Some(m::push_vector(g, l, v))
            });
        }
        // reading and pushing arrays of values
        for ty_ in [3u32, 5] {
            t.stub(mid::LUA_TYPE, false, 2, 0, ty_);
            for n in [0u32, 1, 3] {
                let ty = t.object(0x7b00, &[0, 0, 0, 2, 2], &[]);
                t.w32(ty + 0x1c, 0xc);
                let at = t.zeros(0x40);
                let what = format!("{label}: read array {ty_} {n}");
                t.cdecl(&what, 0x0043_63b0, &[l, n, at, ty], |g| {
                    Some(m::read_array(g, l, n, at, ty))
                });
                let what = format!("{label}: push values {n}");
                t.cdecl(&what, 0x0043_6e20, &[l, n, at, ty], |g| {
                    Some(m::push_values(g, l, n, at, ty))
                });
            }
        }
        // running a chunk
        for r in [0u32, 2] {
            t.stub(mid::LUA_PCALLK, false, 6, 0, r);
            let pl = t.zeros(4);
            t.w32(pl, l);
            let what = format!("{label}: run chunk {r}");
            t.cdecl(&what, 0x005d_65f0, &[pl], |g| Some(m::run_chunk(g, pl)));
        }
        // the allocator
        t.stub(s::PHYRE_FREE, false, 1, 0, 0xf4ee);
        t.stub(s::MEMCPY, false, 3, 0, 0x3e3c);
        for (old, osize, nsize, got) in [
            (0x00c9_0100u32, 8u32, 0u32, 0x00c9_0200u32),
            (0, 0, 0x20, 0x00c9_0200),
            (0x00c9_0100, 8, 0x20, 0x00c9_0200),
            (0x00c9_0100, 0x40, 0x20, 0x00c9_0200),
            (0x00c9_0100, 8, 0x20, 0),
        ] {
            t.stub(mid::MALLOC_ALIGNED, false, 2, 0, got);
            let what = format!("{label}: alloc {old:#x} {osize} {nsize} {got:#x}");
            t.cdecl(&what, 0x005d_5ba0, &[JUNK[0], old, osize, nsize], |g| {
                Some(m::lua_alloc(g, old, osize, nsize))
            });
        }
        // loading a script buffer
        for (buf, r) in [(0u32, 0u32), (0x00c9_8800, 0), (0x00c9_8800, 3)] {
            t.stub(mid::LUA_LOADBUFFERX, false, 5, 0, r);
            let sc = t.zeros(0x20);
            t.w32(sc, 0x8000_0123);
            t.w32(sc + 4, buf);
            t.w32(sc + 0x1c, 0x00c9_9901);
            let what = format!("{label}: load script {buf:#x} {r}");
            t.cdecl(&what, 0x0043_6750, &[l, sc], |g| {
                Some(m::load_script(g, l, sc))
            });
        }
        // the library openers
        let names: Vec<u32> = ["base", "table", "io", "string"]
            .iter()
            .map(|n| t.cstr(n))
            .collect();
        let table = t.zeros(0x20);
        for (i, &n) in names.iter().enumerate() {
            t.w32(table + 8 * i as u32, n);
            t.w32(table + 8 * i as u32 + 4, 0x00c9_a000 + i as u32);
        }
        let (cnt, tab) = (t.rebase(mid::OPENERS_COUNT), t.rebase(mid::OPENERS));
        let (old_cnt, old_tab) = (t.r32(cnt), t.r32(tab));
        t.w32(tab, table);
        for count in [0u32, 4, 0x8000_0003] {
            t.w32(cnt, count);
            for want in ["io", "string", "math", "", "tabl"] {
                let w = t.cstr(want);
                let what = format!("{label}: opener {count:#x} {want}");
                t.cdecl(&what, 0x005d_5710, &[w], |g| Some(m::library_opener(g, w)));
            }
        }
        // a count with bits above 16 (masked to 31 bits): found early
        t.w32(cnt, 0x8001_0000);
        for want in ["base", "io"] {
            let w = t.cstr(want);
            let what = format!("{label}: opener big count {want}");
            t.cdecl(&what, 0x005d_5710, &[w], |g| Some(m::library_opener(g, w)));
        }
        t.w32(cnt, old_cnt);
        t.w32(tab, old_tab);
        // the userdata checks
        for &(f, name) in &mid::USERDATA_CHECKS {
            for (u, mt, eq) in [
                (0u32, 1u32, 1u32),
                (0x00c9_b000, 0, 1),
                (0x00c9_b000, 1, 0),
                (0x00c9_b000, 1, 1),
            ] {
                t.stub(s::LUA_TOUSERDATA, false, 2, 0, u);
                t.stub(mid::LUA_GETMETATABLE, false, 2, 0, mt);
                t.stub(mid::LUA_RAWEQUAL, false, 3, 0, eq);
                let what = format!("{label}: check {f:#x} {u:#x} {mt} {eq}");
                t.cdecl(&what, f, &[l, 2], |g| {
                    Some(m::check_userdata(g, name, l, 2))
                });
            }
        }
        // the component index: no name, no upvalue, not found, found
        // the port's index word: low in its own stack (the original's is in its frame;
        // neither is compared)
        let out = t.b.stack_range().start + 0x40;
        for (name, u, found, writes) in [
            (0u32, 1u32, 1u32, true),
            (0x00c9_c000, 0, 1, true),
            (0x00c9_c000, 1, 0x100, true),
            (0x00c9_c000, 1, 1, true),
            // a callee that says yes without writing: the zeroed word is pushed
            (0x00c9_c000, 1, 1, false),
        ] {
            t.stub(mid::LUA_TOLSTRING, false, 3, 0, name);
            let ud = if u == 0 { 0 } else { t.zeros(0x20) };
            t.stub(s::LUA_TOUSERDATA, false, 2, 0, ud);
            // the callee writes the index through its second word
            let mut pre = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
            pre.extend([0xc7, 0x00]);
            pre.extend(0x0000_0042u32.to_le_bytes()); // mov dword [eax], 0x42
            if !writes {
                pre.clear();
            }
            let c = t.stub_code_with(&pre, mid::COMPONENT_INDEX, true, 1, 2, found);
            t.place(mid::COMPONENT_INDEX, c);
            let what = format!("{label}: component index {name:#x} {u} {found:#x} {writes}");
            t.cdecl(&what, 0x0043_6240, &[l], |g| {
                Some(m::component_index(g, l, out))
            });
        }
        t.cdecl(
            &format!("{label}: register functions"),
            0x005d_6070,
            &[l],
            |g| Some(m::register_functions(g, l)),
        );
        // the loaded modules
        for (t1, t2, keep) in [
            (0u32, 5u32, 1u32),
            (5, 0, 1),
            (5, 5, 0),
            (5, 5, 0x100),
            (5, 5, 1),
        ] {
            let ty_code = {
                // lua_type: the first call answers t1, the second t2
                let cell = t.zeros(4);
                let mut pre = vec![0xff, 0x05];
                pre.extend(cell.to_le_bytes()); // inc dword [cell]
                pre.extend([0x83, 0x3d]);
                pre.extend(cell.to_le_bytes());
                pre.push(0x01); // cmp dword [cell], 1
                pre.extend([0xb8]);
                pre.extend(t1.to_le_bytes()); // mov eax, t1
                pre.extend([0x74, 0x05]); // je +5
                pre.push(0xb8);
                pre.extend(t2.to_le_bytes()); // mov eax, t2
                let mut c = pre;
                c.push(0xc3);
                (c, cell)
            };
            let code = t.code(&ty_code.0);
            t.place(mid::LUA_TYPE, code);
            let name = t.cstr("mod");
            let what = format!("{label}: loaded module {t1} {t2} {keep:#x}");
            t.w32(ty_code.1, 0);
            let ea = {
                let addr = t.rebase(0x005d_5a90);
                t.w32(t.log, 0);
                let ra =
                    t.a.call(addr, ffx_oracle::Conv::Cdecl, &[l, name, keep])
                        .unwrap();
                ra.eax
            };
            let eb = m::loaded_module(&mut m5p_scripting_twins::G(&mut t.b), l, name, keep);
            assert_eq!(ea, eb, "{what}: eax");
            t.diff(&what);
        }
        // the upvalue callers
        for &f in &mid::UPVALUE_CALLERS {
            for (has_u, has_obj, flags, sig_n) in [
                (false, true, 0u8, 1u32),
                (true, false, 0, 1),
                (true, true, 0, 1),
                (true, true, 4, 1),
                (true, true, 0, 3),
                (true, true, 0, 5),
            ] {
                let u = if has_u { t.zeros(0x20) } else { 0 };
                t.stub(s::LUA_TOUSERDATA, false, 2, 0, u);
                if has_u {
                    t.w8(u + 0x10, flags);
                    if has_obj {
                        let sig = t.zeros(0x20);
                        for i in 0..sig_n {
                            t.w32(sig + 4 * i, 0x100 + i);
                        }
                        let o = t.object(0x7c00, &[0, 1, 0, 0, 0], &[0, 0x0bb, 0, 0, sig]);
                        t.w32(u + 0x14, o);
                    }
                }
                let what =
                    format!("{label}: upvalue call {f:#x} {has_u} {has_obj} {flags} {sig_n}");
                t.cdecl(&what, f, &[l], |g| Some(m::upvalue_call(g, l)));
            }
        }
        t.unstub();
    }
}

#[test]
fn b2_objects_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(s::OPERATOR_DELETE, false, 1, 0, 0x0de1);
        t.stub(s::PHYRE_FREE, false, 1, 0, 0x0de2);
        // registrants: first, middle, absent, empty list
        let list = t.rebase(mid::UTILITY_LIST);
        let old = t.r32(list);
        for (case, flags) in [(0u32, 0u32), (1, 1), (2, 0), (3, 1)] {
            let objs: Vec<u32> = (0..3u8).map(|i| t.junk(0x10, i)).collect();
            for i in 0..3 {
                t.w32(objs[i] + 4, if i < 2 { objs[i + 1] } else { 0 });
            }
            let this = match case {
                0 => objs[0],
                1 => objs[1],
                _ => t.junk(0x10, 9),
            };
            t.w32(list, if case == 3 { 0 } else { objs[0] });
            let what = format!("{label}: registrant dtor {case} {flags}");
            t.this(&what, 0x005d_52d0, this, &[flags], |g| {
                Some(m::registrant_dtor(g, this, flags))
            });
        }
        t.w32(list, old);
        // the stream writer
        t.stub(mid::WRITER_BASE_DTOR, true, 0, 0, 0xba5e);
        t.stub(mid::STREAM_DTOR, true, 0, 0, 0x5d);
        for flags in [0u32, 1] {
            let w = t.junk(0x20, 3);
            let what = format!("{label}: writer dtor {flags}");
            t.this(&what, 0x009f_07a0, w, &[flags], |g| {
                Some(m::writer_dtor(g, w, flags))
            });
        }
        t.stub(mid::STREAM_CTOR, true, 5, 5, 0xc7);
        t.stub(mid::READER_CTOR, true, 1, 1, 0xc8);
        for ok in [false, true] {
            let p = if ok { t.junk(0x18, 1) } else { 0 };
            t.stub(mid::MALLOC, false, 1, 0, p);
            let what = format!("{label}: new writer {ok}");
            t.cdecl(&what, 0x009f_0810, &[0x00c9_0400], |g| {
                Some(m::new_writer(g, 0x00c9_0400))
            });
        }
        // the scheduler list: entries {next, owner, key, .., index +0x10, .., state +0x18}
        t.stub(mid::FREELIST_DEALLOCATE, true, 1, 1, 0xfee);
        for states in [[0u32, 0, 0], [1, 0, 1], [1, 1, 1], [0, 1, 0]] {
            let sched = t.junk(0x40, 2);
            let es: Vec<u32> = (0..3u8).map(|i| t.junk(0x20, i + 4)).collect();
            for i in 0..3 {
                t.w32(es[i], if i < 2 { es[i + 1] } else { 0 });
                t.w32(es[i] + 0x18, states[i]);
            }
            t.w32(sched + 0x24, es[0]);
            let what = format!("{label}: reap {states:?}");
            t.this(&what, 0x005d_1220, sched, &[], |g| {
                m::reap_finished(g, sched);
                None
            });
        }
        let sched = t.zeros(0x40);
        t.this(
            &format!("{label}: reap empty"),
            0x005d_1220,
            sched,
            &[],
            |g| {
                m::reap_finished(g, sched);
                None
            },
        );
        // has entry: owner/key/name combinations
        let owner = t.zeros(0x20);
        let names = t.zeros(0x20);
        t.w32(owner + 0xc, names);
        let sched = t.zeros(0x40);
        let es: Vec<u32> = (0..3).map(|_| t.zeros(0x20)).collect();
        for i in 0..3 {
            t.w32(es[i], if i < 2 { es[i + 1] } else { 0 });
            t.w32(es[i] + 4, owner);
            t.w32(es[i] + 8, 0x10 + i as u32);
            t.w32(es[i] + 0x10, i as u32);
            t.w32(es[i] + 0x18, u32::from(i == 0));
        }
        t.w32(sched + 0x24, es[0]);
        let empty = t.cstr("");
        let nm = t.cstr("idle");
        for ok in [0u32, 0x101, 0x100] {
            t.stub(mid::NAME_IS, true, 1, 1, ok);
            for (o, key, name) in [
                (owner, 0x10u32, 0u32),
                (owner, 0x11, 0),
                (owner, 0x11, empty),
                (owner, 0x12, nm),
                (0x55, 0x12, 0),
                (owner, 0x13, nm),
            ] {
                let what = format!("{label}: has entry {ok:#x} {key:#x} {name:#x}");
                let mut al = 0u8;
                let ea = t.this(&what, 0x005d_0e20, sched, &[o, key, name], |g| {
                    al = m::has_entry(g, sched, o, key, name);
                    None
                });
                assert_eq!(ea as u8, al, "{what}: al");
            }
        }
        // the float random number
        let ctr = t.rebase(mid::RANDOM_COUNTER);
        for c in [
            0u32,
            1,
            2,
            0x7fff_ffff,
            0xffff_ffff,
            0x1234_5678,
            41,
            42,
            43,
            1000,
        ] {
            t.w32(ctr, c);
            let what = format!("{label}: random {c:#x}");
            let addr = t.rebase(0x0045_ea50);
            let ra = t.a.call(addr, ffx_oracle::Conv::Cdecl, &[]).unwrap();
            let fb = m::random_float(&mut m5p_scripting_twins::G(&mut t.b));
            assert_eq!(ra.st0_f32().map(f32::to_bits), Some(fb.to_bits()), "{what}");
            t.diff(&what);
        }
        // the bit array reads
        let words = t.alloc(
            &[0x89ab_cdefu32, 0x0123_4567, 0xfedc_ba98, 0x7654_3210]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        let ba = t.zeros(4);
        t.w32(ba, words);
        for (start, n) in [
            (0u32, 0u32),
            (0, 1),
            (3, 5),
            (30, 4),
            (0, 32),
            (4, 32),
            (32, 32),
            (17, 40),
            (1, 31),
            (64, 20),
        ] {
            let what = format!("{label}: bits {start} {n}");
            t.this(&what, 0x004d_3ef0, ba, &[start, n], |g| {
                Some(m::read_bits(g, ba, start, n))
            });
        }
        // the depth of field initialiser
        t.stub(mid::SHADER_PARAM, true, 1, 1, 0x00c9_d000);
        for (ctx, first, r) in [
            (false, 0u32, 0u32),
            (true, 0, 0),
            (true, 0x00c9_0200, 0),
            (true, 0x00c9_0200, 1),
        ] {
            t.stub(mid::DOF_INIT_SHADERS, true, 2, 2, r);
            let fx = t.junk(0x1000, 6);
            if ctx {
                let cx = t.zeros(8);
                t.w32(cx, first);
                t.w32(fx + 8, cx);
            } else {
                t.w32(fx + 8, 0);
            }
            let what = format!("{label}: dof {ctx} {first:#x} {r}");
            t.this(&what, 0x00a1_b710, fx, &[], |g| Some(m::dof_init(g, fx)));
        }
        t.unstub();
    }
}

#[test]
fn b2_fixups_match_the_original() {
    for (label, mut t) in Twins::all() {
        // name components: records of 0x20 bytes, links at +0xc
        let list = t.rebase(mid::NAME_LIST);
        let old = t.r32(list);
        for n in [0u32, 1, 3] {
            let (p, _) = instance_list(&mut t, n, 0x20, 0);
            let head = t.zeros(8);
            t.w32(list, head);
            let what = format!("{label}: link names {n}");
            t.cdecl(&what, 0x0045_ad00, &[p], |g| Some(m::link_names(g, p)));
        }
        t.w32(list, old);
        // dynamic members: class flag on and off
        t.stub(mid::DYNAMIC_MEMBER_FIXUP, true, 1, 1, 0xd1);
        for n in [0u32, 1, 3] {
            let (p, recs) = instance_list(&mut t, n, 0x40, 4);
            for i in 0..n {
                let r = recs + 0x40 * i;
                let class = t.zeros(0x100);
                t.w8(class + 0x90, (i & 1) as u8 | 0x10);
                t.w32(r + 0xc, class);
                t.w32(r + 0x14, 0xffff_ffff);
                t.w32(r + 0x2c, 0x00c9_e001 + 0x10 * i);
            }
            let what = format!("{label}: dynamic members {n}");
            t.cdecl(&what, 0x0044_6650, &[p], |g| {
                Some(m::fix_dynamic_members(g, p))
            });
        }
        // the post effects: slot 1 failing, a context, the initialiser failing
        for &(f, init) in &mid::POSTFX_FIXUPS {
            for (slot1, ctx, r) in [
                (0u32, 0u32, 0u32),
                (5, 0, 0),
                (0, 1, 0),
                (0, 1, 7),
                (0, 2, 0),
            ] {
                t.stub(init, true, 2, 2, r);
                let (p, recs) = instance_list(&mut t, 2, 0x40, 4);
                for i in 0..2u32 {
                    let e = recs + 0x40 * i;
                    let o = t.object(
                        0x7d00 + 0x10 * i,
                        &[0, 2],
                        &[0, if i == 1 { slot1 } else { 0 }],
                    );
                    let vt = t.r32(o);
                    t.w32(e, vt);
                    if ctx != 0 {
                        let c = t.zeros(8);
                        t.w32(c, if ctx == 1 { 0x00c9_0300 } else { 0 });
                        t.w32(e + 8, c);
                    }
                }
                let what = format!("{label}: postfx {f:#x} {slot1} {ctx} {r}");
                t.cdecl(&what, f, &[p], |g| Some(m::fix_postfx(g, init, p)));
            }
            let (p, _) = instance_list(&mut t, 0, 0x40, 4);
            t.cdecl(&format!("{label}: postfx {f:#x} empty"), f, &[p], |g| {
                Some(m::fix_postfx(g, init, p))
            });
        }
        // LODs: count 1, no buffer, two records
        for level in [false, true] {
            let f = if level { 0x005a_5190 } else { 0x005a_5110 };
            t.stub(mid::LOD_GROUP_SET, true, 2, 2, 0x1d1);
            t.stub(mid::LOD_LEVEL_SET, true, 2, 2, 0x1d2);
            let cnt = t.rebase(mid::LOD_COUNT);
            let old = t.r32(cnt);
            for (count, buf, n) in [
                (1u32, true, 2u32),
                (3, false, 2),
                (3, true, 0),
                (3, true, 2),
                (2, true, 1),
            ] {
                t.w32(cnt, count);
                let (p, _) = instance_list(&mut t, n, 0x40, 0);
                if buf {
                    let b_ = t.junk(0x100, 1);
                    t.w32(p + 0x38, b_);
                }
                let what = format!("{label}: lod {level} {count} {buf} {n}");
                t.cdecl(&what, f, &[p], |g| Some(m::lod_fixup(g, p, level)));
            }
            t.w32(cnt, old);
        }
        // script method records
        t.stub(mid::FIND_MEMBER, true, 1, 1, 0);
        for found in [0u32, 0x00c9_f000] {
            t.stub(mid::FIND_MEMBER, true, 1, 1, found);
            for n in [0u32, 2] {
                let (p, recs) = instance_list(&mut t, n, 0x40, 0);
                for i in 0..n {
                    let r = recs + 0x40 * i;
                    let class = t.zeros(0x60);
                    t.w32(class + 0x4c, class + 0x4c);
                    t.w32(class + 0x50, class + 0x4c);
                    t.w32(r + 8, class);
                    t.w32(r + 0x2c, if i == 0 { 0 } else { 0x00c9_e101 });
                }
                let what = format!("{label}: method scripts {found:#x} {n}");
                t.cdecl(&what, 0x0061_9980, &[p], |g| {
                    Some(m::fix_method_scripts(g, p))
                });
            }
        }
        // animatable components
        t.stub(mid::ANIM_EVALUATE, true, 3, 3, 0xe7);
        t.stub(mid::ANIM_BIND, false, 4, 0, 0xb1d);
        t.stub(mid::ANIM_SET_TARGET, true, 2, 2, 0x7a);
        t.stub(mid::ANIM_SET_WEIGHT, true, 2, 2, 0x3e);
        let class = t.rebase(mid::ANIM_SET_CLASS);
        let mk = |t: &mut Twins, set: u32, same: bool, found: bool, ctx: bool| -> u32 {
            let c = t.junk(0x80, 3);
            t.w32(c + 0x10, set);
            t.w32(c + 0x5c, if same { set } else { set ^ 0x40 });
            if ctx {
                let e2 = t.zeros(0x20);
                t.w32(e2 + 8, if found { class } else { 0x77 });
                let e1 = t.zeros(0x20);
                t.w32(e1, e2);
                t.w32(e1 + 8, 0x66);
                let cx = t.zeros(8);
                t.w32(cx, e1);
                t.w32(c + 8, cx);
            } else {
                t.w32(c + 8, 0);
            }
            c
        };
        for (set, same, found, ctx) in [
            (0u32, false, true, true),
            (0x00c9_1200, false, true, false),
            (0x00c9_1200, false, false, true),
            (0x00c9_1200, true, true, true),
            (0x00c9_1200, false, true, true),
        ] {
            let c = mk(&mut t, set, same, found, ctx);
            let what = format!("{label}: animatable bind {set:#x} {same} {found} {ctx}");
            t.this(&what, 0x0054_1c00, c, &[], |g| {
                Some(m::animatable_bind(g, c))
            });
        }
        for (count, index, target, check) in [
            (2u32, 2u32, 1u32, 0u32),
            (0x8000_0002, 1, 0, 0),
            (0x8000_0002, 2, 1, 0),
            (2, 1, 0x00c9_1300, 9),
            (2, 1, 0x00c9_1300, 0),
        ] {
            t.stub(mid::ANIM_SLOT_CHECK, true, 1, 1, check);
            let c = mk(&mut t, 0x00c9_1200, false, true, true);
            let slots = t.junk(0xa0, 2);
            t.w32(c + 0x14, count);
            t.w32(c + 0x18, slots);
            let what = format!("{label}: set slot {count:#x} {index} {target:#x} {check}");
            t.this(&what, 0x0054_3280, c, &[index, target], |g| {
                Some(m::animatable_set_slot(g, c, index, target))
            });
        }
        t.stub(mid::ANIM_SLOT_CHECK, true, 1, 1, 0);
        for find in [0u32, 4] {
            t.stub(mid::ANIM_SLOT_FIND, true, 1, 1, find);
            let (p, recs) = instance_list(&mut t, 3, 0x80, 4);
            for i in 0..3u32 {
                let c = recs + 0x80 * i;
                t.w32(c + 0x10, if i == 1 { 0 } else { 0x00c9_1200 });
                t.w32(c + 0x5c, 0x00c9_1200);
                let cx = t.zeros(8);
                let e1 = t.zeros(0x20);
                t.w32(e1 + 8, class);
                t.w32(cx, e1);
                t.w32(c + 8, cx);
                let tl = t.zeros(0x60);
                t.w32(tl, if i == 2 { 0 } else { 0x00c9_1400 });
                t.w32(c + 0x14, if i == 0 { 2 } else { 0x8000_0001 });
                t.w32(c + 0x18, tl);
            }
            let what = format!("{label}: animatables {find}");
            t.cdecl(&what, 0x0054_0c10, &[p], |g| Some(m::fix_animatables(g, p)));
        }
        t.unstub();
    }
}

// ---- batch 3: the object accessors, Vector and Matrix functions ----

/// Stubs for the userdata check (`lua_touserdata` answering `u`, the
/// metatable and its comparison answering `mt`, `eq`) and the rest of the
/// Lua calls the batch-3 functions make; `luaL_error` logs the words the
/// expected call passes (its varargs vary).
fn lua_b3(t: &mut Twins, u: u32, mt: u32, eq: u32, ty: u32, err_words: u32) {
    t.stub(s::LUA_TOUSERDATA, false, 2, 0, u);
    t.stub(mid::LUA_GETMETATABLE, false, 2, 0, mt);
    t.stub(mid::LUA_RAWEQUAL, false, 3, 0, eq);
    t.stub(mid::LUA_TYPE, false, 2, 0, ty);
    t.stub(s::LUA_GETFIELD, false, 3, 0, 0x6f1);
    t.stub(s::LUA_SETTOP, false, 2, 0, 0x7020);
    t.stub(s::LUA_SETMETATABLE, false, 2, 0, 0x3e7a);
    t.stub(s::LUA_PUSHNUMBER, false, 2, 0, 0x1b7);
    t.stub(lb::LUA_ERROR, false, err_words, 0, 0xe770);
}

/// A `PhyreObject` userdata `{descriptor, object}` whose descriptor's
/// parent chain (`+0x40`) has `depth` links before reaching `class` (or
/// never when `reach` is false). Each descriptor has a name at `+0x18`.
fn object_ud(t: &mut Twins, class: u32, depth: u32, reach: bool, obj: u32, null_d: bool) -> u32 {
    let mut d = class;
    if !reach {
        d = t.zeros(0x48);
        t.w32(d + 0x18, 0x00c9_0001);
    }
    for i in 0..depth {
        let e = t.zeros(0x48);
        t.w32(e + 0x18, 0x00c9_0100 + i);
        t.w32(e + 0x40, d);
        d = e;
    }
    let u = t.zeros(8);
    t.w32(u, if null_d { 0 } else { d });
    t.w32(u + 4, obj);
    u
}

#[test]
fn b3_accessors_match_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        t.stub(lb::DEFAULT_INSTANCE, true, 0, 0, 0x00c9_def0);
        for (i, a) in lb::ACCESSORS.iter().enumerate() {
            let class = t.rebase(a.class);
            // (userdata, obj, depth, reaches, null descriptor, lua type)
            let cases = [
                (false, 0u32, 0u32, true, false, 0u32),
                (false, 0, 0, true, false, 3),
                (true, 0, 0, true, false, 0),
                (true, 0, 0, true, false, 7),
                (true, 0x00c9_5550, 0, true, true, 0),
                (true, 0x00c9_5550, 0, true, false, 0),
                (true, 0x00c9_5550, 2, true, false, 0),
                (true, 0x00c9_5550, 2, false, false, 0),
            ];
            for (k, &(ud, obj, depth, reach, null_d, ty)) in cases.iter().enumerate() {
                let u = if ud {
                    object_ud(&mut t, class, depth, reach, obj, null_d)
                } else {
                    0
                };
                let words = if ud && obj != 0 && !null_d && !reach {
                    4
                } else {
                    3
                };
                lua_b3(&mut t, u, 1, 1, ty, words);
                let what = format!("{label}: accessor {i} case {k}");
                t.cdecl(&what, a.addr, &[l], |g| Some(lua::accessor_get(g, a, l)));
            }
        }
        // the reference accessor
        let class = t.rebase(lb::ENTITY_CLASS);
        let cases = [
            (false, 0u32, 0u32, true, false),
            (true, 0x00c9_5550, 0, true, true),
            (true, 0x00c9_5550, 0, true, false),
            (true, 0x00c9_5550, 3, true, false),
            (true, 0x00c9_5550, 2, false, false),
            (true, 0, 1, true, false),
        ];
        for (k, &(ud, obj, depth, reach, null_d)) in cases.iter().enumerate() {
            let u = if ud {
                object_ud(&mut t, class, depth, reach, obj, null_d)
            } else {
                0
            };
            let words = if ud && !null_d && !reach { 4 } else { 3 };
            lua_b3(&mut t, u, 1, 1, 0, words);
            let what = format!("{label}: entity accessor case {k}");
            t.cdecl(&what, 0x0044_f7f0, &[l], |g| Some(lua::entity_get(g, l)));
        }
        t.unstub();
    }
}

#[test]
fn b3_vectors_match_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        let mut rng = Rng(0x2468_ace1);
        for &(f, n, root) in &lb::LENGTHS {
            // not a Vector: no userdata, no metatable, another metatable
            for (u, mt, eq) in [(0u32, 1u32, 1u32), (1, 0, 1), (1, 1, 0)] {
                let v = if u == 0 { 0 } else { t.zeros(0x10) };
                lua_b3(&mut t, v, mt, eq, 0, 3);
                let what = format!("{label}: length {f:#x} not a vector {u} {mt} {eq}");
                t.cdecl(&what, f, &[l], |g| Some(lua::vector_length(g, l, n, root)));
            }
            for k in 0..40 {
                let v = t.zeros(0x10);
                for i in 0..4 {
                    let w = if k < 10 {
                        FLOATS[(k + i as usize) % FLOATS.len()]
                    } else {
                        // random magnitudes around 1, either sign
                        let r = rng.next();
                        (r & 0x8000_0000) | (0x3c00_0000 + (r & 0x07ff_ffff))
                    };
                    t.w32(v + 4 * i, w);
                }
                lua_b3(&mut t, v, 1, 1, 0, 3);
                let what = format!("{label}: length {f:#x} #{k}");
                t.cdecl(&what, f, &[l], |g| Some(lua::vector_length(g, l, n, root)));
            }
        }
        t.unstub();
    }
}

#[test]
fn b3_matrices_match_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        let table = t.alloc(
            &[0u32, 0x3f80_0000, 0xc020_0000, 0x7149_f2ca, 0xffc1_2345]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        for &f in &[0x0057_1480u32, 0x0057_1540] {
            for (top, ok) in [(1u32, true), (3, true), (0, true), (2, false)] {
                let m = if ok { t.junk(0x40, 1) } else { t.zeros(0x40) };
                lua_b3(&mut t, 0, 1, 1, 0, 3);
                t.stub(s::LUA_NEWUSERDATA, false, 2, 0, m);
                t.stub(lb::LUA_GETTOP, false, 1, 0, top);
                t.stub_float_table(lb::LUA_CHECKNUMBER, 2, 1, table);
                let what = format!("{label}: matrix {f:#x} top {top} {ok}");
                t.cdecl(&what, f, &[l], |g| {
                    Some(if f == 0x0057_1480 {
                        lua::matrix_scale(g, l)
                    } else {
                        lua::matrix_translation(g, l)
                    })
                });
            }
        }
        t.unstub();
    }
}

/// A stand-in for a math routine `(out, in)` (cdecl): copies `in_words`
/// of its input to the returned "seen" cell (so the diff checks what it
/// was handed), writes `out_words` pattern floats to `out`, returns `out`.
fn math_stub(t: &mut Twins, sym: u32, in_words: u32, out_words: u32, pattern: u32) -> u32 {
    let seen = t.zeros(0x40);
    let mut c = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8]
    for k in 0..in_words {
        c.extend([0x8b, 0x50, (4 * k) as u8]); // mov edx, [eax+4k]
        c.extend([0x89, 0x15]);
        c.extend((seen + 4 * k).to_le_bytes()); // mov [seen+4k], edx
    }
    c.extend([0x8b, 0x44, 0x24, 0x04]); // mov eax, [esp+4]
    for k in 0..out_words {
        c.extend([0xc7, 0x40, (4 * k) as u8]);
        c.extend((0x3f80_0000 + pattern + 0x1_0000 * k).to_le_bytes()); // mov dword [eax+4k], pattern
    }
    c.push(0xc3);
    let code = t.code(&c);
    t.place(sym, code);
    seen
}

#[test]
fn b3_unary_math_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        // the port's math frame: low in its own stack, as the original's is in
        // its frame (neither is compared; what reaches the routines is)
        let frame = t.b.stack_range().start + 0x40;
        for (i, op) in lb::UNARY.iter().enumerate() {
            let (inw, outw) = match op.kind {
                lb::UnaryKind::Matrix => (16, 16),
                lb::UnaryKind::Vector4 => (4, 4),
                lb::UnaryKind::Vector3 => (3, 4),
            };
            math_stub(&mut t, op.op, inw, outw, 0x100 * i as u32);
            for (u, mt, eq) in [
                (false, 1u32, 1u32),
                (true, 0, 1),
                (true, 1, 0),
                (true, 1, 1),
            ] {
                let v = if u { t.junk(0x40, 7) } else { 0 };
                lua_b3(&mut t, v, mt, eq, 0, 3);
                let out = t.junk(0x40, 1);
                t.stub(s::LUA_NEWUSERDATA, false, 2, 0, out);
                let what = format!("{label}: unary {:#x} {u} {mt} {eq}", op.addr);
                t.cdecl(&what, op.addr, &[l], |g| Some(lua::unary(g, op, l, frame)));
            }
        }
        // the rotation
        let table = t.alloc(
            &[0u32, 0x42b4_0000, 0xc334_0000, 0x3f80_0000]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        math_stub(&mut t, lb::ROTATION, 3, 16, 0x777);
        lua_b3(&mut t, 0, 1, 1, 0, 3);
        let out = t.junk(0x40, 1);
        t.stub(s::LUA_NEWUSERDATA, false, 2, 0, out);
        t.stub_float_table(lb::LUA_CHECKNUMBER, 2, 1, table);
        let what = format!("{label}: rotation");
        t.cdecl(&what, 0x0057_13e0, &[l], |g| {
            Some(lua::matrix_rotation(g, l, frame))
        });
        t.unstub();
    }
}

// ---- batch 4: the scheduler, scripted components, fixups ----

/// A scheduler: free list at `+4` (`free` entries of 0x20 bytes, `+0xc`
/// the block size), counters at `+0x84`; the lists `+0x20`, `+0x24`.
fn scheduler(t: &mut Twins, free: u32) -> u32 {
    let s_ = t.junk(0x300, 3);
    let mut head = 0;
    for _ in 0..free {
        let e = t.junk(0x20, 5);
        t.w32(e, head);
        head = e;
    }
    t.w32(s_ + 4, head);
    t.w32(s_ + 0xc, 0x10);
    t.w32(s_ + 0x20, 0x00c9_0020);
    t.w32(s_ + 0x24, 0x00c9_0024);
    for i in 0..0x100u32 {
        t.write(
            s_ + 0x84 + 2 * i,
            &((i * 7919 + 3) as u16 % 5).to_le_bytes(),
        );
    }
    s_
}

#[test]
fn b4_scheduler_matches_the_original() {
    for (label, mut t) in Twins::all() {
        // the port's name word: low in its own stack (the original's is in
        // its frame or argument area; the lookup stub logs what it holds)
        let cell = t.b.stack_range().start + 0x40;
        t.stub(sc_::FREELIST_GROW, true, 1, 1, 0x9f0);
        for (idx, free) in [(0x8000_0000u32, 2u32), (3, 0), (3, 2), (0, 2)] {
            t.stub_deref0(sc_::NAME_INDEX, 2, 2, idx);
            let s_ = scheduler(&mut t, free);
            let owner = t.zeros(0x10);
            let a = [owner, 0x11, 0x22, 0x00c9_3300, 0x44];
            let what = format!("{label}: scheduler add {idx:#x} {free}");
            t.this(&what, 0x005d_01d0, s_, &a, |g| {
                Some(sc::scheduler_add(g, s_, a, cell))
            });
            for (name, step) in [
                (0u32, 1u32),
                (0x00c9_3300, 0),
                (0x00c9_3300, 1),
                (0x00c9_3300, 7),
                (0x00c9_3300, 0x100),
                (0x00c9_3300, 0x101),
            ] {
                let s_ = scheduler(&mut t, free);
                let a = [owner, 0x11, 0x22, name, step, 0x66];
                let what = format!("{label}: scheduler insert {idx:#x} {free} {name:#x} {step:#x}");
                t.this(&what, 0x005d_0280, s_, &a, |g| {
                    Some(sc::scheduler_insert(g, s_, a, cell))
                });
            }
        }
        for n in [0u32, 1, 2, 3, 7, 0x80, 0xff, 0x100, 0x101, 0xffff_ffff] {
            let s_ = scheduler(&mut t, 0);
            let what = format!("{label}: least phase {n:#x}");
            t.this(&what, 0x005d_0a80, s_, &[n], |g| {
                Some(sc::least_phase(g, s_, n))
            });
        }
        t.unstub();
    }
}

#[test]
fn b4_scripted_components_match_the_original() {
    for (label, mut t) in Twins::all() {
        let cell = t.b.stack_range().start + 0x40;
        t.stub(s::STRING_ASSIGN, true, 1, 1, 0xa55);
        // the function name: no component, empty, unknown, known, the same string
        let names = [t.cstr(""), t.cstr("update")];
        for (comp, nm, idx, same) in [
            (false, 1usize, 0u32, false),
            (true, 0, 0, false),
            (true, 1, 0x8000_0000, false),
            (true, 1, 2, false),
            (true, 1, 2, true),
            (true, 2, 2, false),
        ] {
            t.stub_deref0(sc_::NAME_INDEX, 2, 2, idx);
            let c = t.junk(0x40, 2);
            let cv = if comp { t.zeros(0x10) } else { 0 };
            t.w32(c + 0x14, cv);
            let pname = if same { c + 0x10 } else { t.zeros(4) };
            let word = if nm == 2 { 0 } else { names[nm] + 1 };
            t.w32(pname, word);
            let what = format!("{label}: set function name {comp} {nm} {idx:#x} {same}");
            t.this(&what, 0x0055_3440, c, &[pname], |g| {
                Some(sc::set_function_name(g, c, pname, cell))
            });
        }
        // the run
        t.stub(sc_::SCHEDULER_REMOVE, true, 3, 3, 0x5e3);
        t.stub(sc_::CLOSE_STATE, false, 1, 0, 0xc1);
        t.stub(mid::NAME_IS, true, 1, 1, 1);
        for (k, (has_sched, has_entry, fname, idx, slot4, owned)) in [
            (true, true, 1usize, 0u32, 0u32, false),
            (false, true, 1, 0, 0, false),
            (true, false, 1, 0, 0, false),
            (true, true, 2, 0, 0, true),
            (true, true, 0, 0, 0, false),
            (true, true, 1, 0x8000_0000, 0, true),
            (true, true, 1, 1, 0x77, false),
            (true, true, 1, 1, 0, true),
            (true, true, 1, 0, 0x78, false),
        ]
        .into_iter()
        .enumerate()
        {
            t.stub_deref0(sc_::NAME_INDEX, 2, 2, idx);
            let c = t.junk(0x40, 4);
            let comp = t.zeros(0x20);
            t.w32(c + 0x14, comp);
            if k == 0 {
                // a scheduler but no component
                let sched = t.zeros(0x40);
                // with an entry for a null owner, which only the component check stops
                let e = t.zeros(0x20);
                t.w32(e + 8, c + 4);
                t.w32(sched + 0x24, e);
                t.w32(c + 0x18, sched);
                t.w32(c + 0x14, 0);
            }
            t.w32(c + 0x10, names[1] + 1);
            t.w32(c + 0x24, if fname == 2 { 0 } else { names[fname] });
            t.w8(c + 0x2d, u8::from(owned));
            t.w32(c + 0x1c, if owned { 0x00c9_4400 } else { 0 });
            if k == 0 {
                // set above
            } else if has_sched {
                // the scheduler: its live list (+0x24) holds the component's entry or not
                let sched = t.object(0x7e00, &[0, 0, 0, 0, 3], &[0, 0, 0, 0, slot4]);
                let e = t.zeros(0x20);
                t.w32(e + 4, if has_entry { comp } else { 0x55 });
                t.w32(e + 8, c + 4);
                t.w32(sched + 0x24, e);
                t.w32(c + 0x18, sched);
            } else {
                t.w32(c + 0x18, 0);
            }
            let what = format!("{label}: scripted run case {k}");
            t.this(&what, 0x0055_4b60, c, &[], |g| {
                Some(sc::scripted_run(g, c, cell))
            });
        }
        // the copy assignment: another component, itself
        for same in [false, true] {
            let c = t.junk(0x40, 6);
            let src = if same { c } else { t.junk(0x40, 9) };
            let what = format!("{label}: scripted assign {same}");
            t.this(&what, 0x0055_3b80, c, &[src], |g| {
                Some(sc::scripted_assign(g, c, src))
            });
        }
        // the name components
        let list = t.rebase(sc_::NAME_LIST);
        let old = t.r32(list);
        let strs = [t.cstr("alpha"), t.cstr("beta")];
        let recs: Vec<u32> = (0..3).map(|_| t.zeros(0x20)).collect();
        for (i, &r) in recs.iter().enumerate() {
            t.w32(r + 0xc, if i < 2 { recs[i + 1] + 0xc } else { list });
            t.w32(r + 0x14, if i == 1 { 0 } else { strs[i / 2] + 1 });
        }
        for start in [list, 0, recs[0] + 0xc] {
            t.w32(list, start);
            let wants = [0u32, strs[0], strs[1], t.cstr("beta"), t.cstr("gamma")];
            for want in wants {
                let what = format!("{label}: find name {start:#x} {want:#x}");
                t.cdecl(&what, 0x0045_ac30, &[want], |g| {
                    Some(sc::find_name_component(g, want))
                });
            }
        }
        t.w32(list, old);
        t.unstub();
    }
}

#[test]
fn b4_fixups_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(s::PHYRE_FREE, false, 1, 0, 0xf4ee);
        // the PSharray external buffers
        for (f, words) in [(0x005a_80f0u32, 1u32), (0x005a_8190, 3)] {
            for head in [0u32, 1, 3, 0x8000_0003] {
                for n in [0u32, 1, 4, 0x8000_0000] {
                    for same in [false, true] {
                        let a = t.junk(0x20, 1);
                        let block = t.junk(0x40, 2);
                        t.w32(a, head);
                        t.w32(a + 4, block);
                        let buf = if same { block } else { t.junk(0x40, 3) };
                        let what = format!("{label}: sharray {f:#x} {head:#x} {n:#x} {same}");
                        t.this(&what, f, a, &[buf, n], |g| {
                            Some(sc::sharray_set_external(g, a, buf, n, words))
                        });
                    }
                }
            }
        }
        // dynamic class descriptors
        t.stub(sc_::DYNAMIC_FIX_MEMBERS, true, 0, 0, 0xd1);
        t.stub(sc_::DYNAMIC_SET_DEFAULTS, true, 1, 1, 0xd2);
        t.stub(sc_::ADD_TO_NAMESPACE, true, 0, 0, 0xd3);
        for (found, defaults) in [(false, 0u32), (true, 0), (true, 0x00c9_d000)] {
            t.stub(sc_::DEFAULT_BUFFER, true, 0, 0, defaults);
            let old = if found {
                t.object(0x7f00, &[0, 0, 0, 0, 0, 0, 0, 1], &[])
            } else {
                0
            };
            t.stub(sc_::NAMESPACE_FIND, true, 1, 1, old);
            for n in [0u32, 2] {
                let (p, _) = instance_list(&mut t, n, 0x80, 0x18);
                t.w32(p + 0x28, 0x00c9_e000);
                let what = format!("{label}: dynamic classes {found} {defaults:#x} {n}");
                t.cdecl(&what, 0x0044_8680, &[p], |g| {
                    Some(sc::fix_dynamic_classes(g, p))
                });
            }
        }
        // the method script record destructor
        for (a, b_, flag) in [(0u32, 0u32, 0u8), (1, 1, 1), (0x00c9_0f00, 0x00c9_0f01, 3)] {
            let r = t.junk(0x40, 5);
            let prev = t.zeros(8);
            let next = t.zeros(8);
            t.w32(r, prev);
            t.w32(r + 4, next);
            t.w32(r + 0x30, a);
            t.w32(r + 0x2c, b_);
            t.w8(r + 0x10, flag);
            let what = format!("{label}: method script dtor {a:#x} {b_:#x} {flag}");
            t.this(&what, 0x0061_9400, r, &[], |g| {
                Some(sc::method_script_dtor(g, r))
            });
        }
        // animation slots
        for (off, ctor) in sc_::SLOT_CTORS {
            t.stub(ctor, true, 0, 0, 0xc700 + off);
        }
        for n in [0u32, 1, 3] {
            let arr = t.junk(0x4c * 3, 1);
            let what = format!("{label}: slots {n}");
            t.cdecl(&what, 0x0054_1780, &[arr, n], |g| {
                sc::construct_slots(g, arr, n);
                None
            });
        }
        // depth of field
        t.stub(sc_::SHADER_PARAM, true, 1, 1, 0x00c9_d000);
        for (slot1, ctx, r) in [
            (0u32, 0u32, 0u32),
            (5, 0, 0),
            (0, 1, 0),
            (0, 1, 7),
            (0, 2, 0),
        ] {
            t.stub(sc_::DOF_INIT_SHADERS, true, 2, 2, r);
            let (p, recs) = instance_list(&mut t, 2, 0x1000, 4);
            for i in 0..2u32 {
                let e = recs + 0x1000 * i;
                let o = t.object(
                    0x7d80 + 0x10 * i,
                    &[0, 2],
                    &[0, if i == 1 { slot1 } else { 0 }],
                );
                let vt = t.r32(o);
                t.w32(e, vt);
                if ctx != 0 {
                    let c = t.zeros(8);
                    t.w32(c, if ctx == 1 { 0x00c9_0300 } else { 0 });
                    t.w32(e + 8, c);
                }
            }
            let what = format!("{label}: dof fixup {slot1} {ctx} {r}");
            t.cdecl(&what, 0x00a0_7f00, &[p], |g| Some(sc::fix_dof(g, p)));
        }
        t.unstub();
    }
}

// ---- batch 5: script states and runs, member setters ----

/// Stubs for every Lua and Phyre callee the batch-5 functions reach;
/// `printf` logs `printf_words`.
fn lua_b5(t: &mut Twins, printf_words: u32) {
    t.stub(st_::LUA_NEWSTATE, false, 2, 0, 0x00c9_1100);
    t.stub(st_::LUA_OPENLIBS, false, 1, 0, 0x0b1);
    t.stub(s::GLOBAL_NAMESPACE, false, 1, 0, 0x00c9_0b00);
    t.stub(s::REGISTER_TYPES, false, 3, 0, 0x7e9);
    t.stub(mid::LUA_PUSHCCLOSURE, false, 3, 0, 0xcc10);
    t.stub(s::LUA_SETGLOBAL, false, 2, 0, 0x5e7);
    t.stub(s::LUA_PUSHLIGHTUSERDATA, false, 2, 0, 0x119);
    t.stub(mid::LUA_TOLSTRING, false, 3, 0, 0x00c9_7700);
    t.stub(s::LUA_SETTOP, false, 2, 0, 0x7020);
    t.stub(s::LUA_CLOSE, false, 1, 0, 0xc1);
    t.stub(mid::PRINTF, false, printf_words, 0, 0x9f);
    t.stub(s::LUA_GETGLOBAL, false, 2, 0, 0x6e7);
    t.stub(s::PUSH_OBJECT, false, 3, 0, 0x0b1);
    t.stub(st_::LUA_GC, false, 3, 0, 0x6c);
}

/// A script: buffer, size, name table at `+0xc` (two names), the name
/// count `+8`, its own name at `+0x1c`.
fn script(t: &mut Twins, names: u32) -> u32 {
    let sc = t.zeros(0x40);
    t.w32(sc, 0x10);
    t.w32(sc + 4, 0x00c9_8800);
    t.w32(sc + 8, names);
    let tab = t.zeros(8);
    let f0 = t.cstr("init");
    let f1 = t.cstr("update");
    t.w32(tab, f0);
    t.w32(tab + 4, f1 + 1);
    t.w32(sc + 0xc, tab);
    let n = t.cstr("test.lua");
    t.w32(sc + 0x1c, n + 1);
    sc
}

#[test]
fn b5_states_match_the_original() {
    for (label, mut t) in Twins::all() {
        let cell = t.b.stack_range().start + 0x40;
        // new states: no state, no script, load failing, names, chunk run ok and failing
        for (k, (state, has_script, load, names, run)) in [
            (0u32, true, 0u32, 0u32, 0u32),
            (1, false, 0, 0, 0),
            (1, true, 1, 0, 0),
            (1, true, 0, 0, 0),
            (1, true, 0, 0, 4),
            (1, true, 0, 3, 0),
            (1, true, 0, 3, 2),
            (1, true, 0, 0x8000_0000, 0),
        ]
        .into_iter()
        .enumerate()
        {
            lua_b5(&mut t, 4);
            t.stub(st_::LUA_NEWSTATE, false, 2, 0, state * 0x00c9_1100);
            t.stub(mid::LUA_LOADBUFFERX, false, 5, 0, load);
            t.stub(mid::LUA_PCALLK, false, 6, 0, run);
            t.stub_deref0_cdecl(st_::GET_FUNCTION_NAMES, 2, run);
            let sc = if has_script { script(&mut t, names) } else { 0 };
            let what = format!("{label}: new state case {k}");
            t.cdecl(&what, 0x005d_53b0, &[sc, 0x00c9_a110], |g| {
                Some(st::new_state(g, sc, 0x00c9_a110, cell))
            });
        }
        // createThread: no scheduler, given state, no script, own state, failing state,
        // existing state, the start function, kicking a started component
        t.stub_deref0(sc_::NAME_INDEX, 2, 2, 0);
        for (k, (sched, l, flag, owned, has, st_ok, insert, start, add, started)) in [
            (false, 0u32, 0u8, 0u8, 0u32, true, 0u32, false, 0u32, 1u8),
            (true, 0x00c9_1200, 0, 0, 0, true, 0, false, 0, 0),
            (true, 0, 0, 0, 0, true, 3, false, 0, 1),
            (true, 0, 1, 1, 1, true, 0, false, 0, 0),
            (true, 0, 1, 0, 0, false, 0, false, 0, 0),
            (true, 0, 1, 0, 0, true, 0, true, 0, 1),
            (true, 0, 1, 0, 0, true, 0, true, 5, 1),
            (true, 0x00c9_1200, 0, 0, 0, true, 0, true, 0, 0),
            // an owned flag without a state, an empty start name, no component
            (true, 0, 1, 1, 0, true, 0, false, 0, 1),
            (true, 0, 1, 0, 0, true, 0, true, 0, 1),
            (true, 0, 0, 0, 0, true, 0, false, 0, 1),
        ]
        .into_iter()
        .enumerate()
        {
            lua_b5(&mut t, 4);
            t.stub(mid::LUA_LOADBUFFERX, false, 5, 0, 0);
            t.stub(mid::LUA_PCALLK, false, 6, 0, 0);
            t.stub(
                st_::LUA_NEWSTATE,
                false,
                2,
                0,
                if st_ok { 0x00c9_1100 } else { 0 },
            );
            t.stub(st_::SCHEDULER_INSERT, true, 6, 6, insert);
            t.stub(st_::SCHEDULER_ADD, true, 4, 4, add);
            t.stub(st_::SCHEDULER_START, true, 4, 4, 0x57a);
            let c = t.junk(0x40, 3);
            let comp = script(&mut t, 2);
            t.w8(comp + 0x20, flag);
            t.w32(c + 0x14, comp);
            t.w32(c + 0x18, if sched { 0x00c9_5000 } else { 0 });
            t.w8(c + 0x2d, owned);
            t.w32(c + 0x1c, has);
            t.w8(c + 0x2e, started);
            t.w8(c + 0x2c, 0);
            let name = t.cstr("tick");
            t.w32(c + 0x10, name);
            let sname = if k == 9 { t.cstr("") } else { t.cstr("start") };
            t.w32(c + 0x20, if start { sname } else { 0 });
            if k == 10 {
                t.w32(c + 0x14, 0);
            }
            let what = format!("{label}: create thread case {k}");
            t.this(&what, 0x0055_4860, c, &[l], |g| {
                Some(st::create_thread(g, c, l, cell, cell + 4))
            });
        }
        t.unstub();
    }
}

#[test]
fn b5_member_setters_match_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        let table = t.alloc(
            &[0u32, 0x3f80_0000, 0xc020_0000, 0x4049_0fdb]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        let keys: Vec<u32> = ["x", "w", "3", "4", "xy", "", "q", "0"]
            .iter()
            .map(|k| t.cstr(k))
            .collect();
        for f in [0x0057_36c0u32, 0x0057_1d60] {
            for key in keys.iter().copied().chain([0]) {
                for ok in [false, true] {
                    lua_b3(&mut t, 0, 1, 1, 0, 3);
                    t.stub(mid::LUA_TOLSTRING, false, 3, 0, key);
                    let target = if ok { t.junk(0x40, 2) } else { 0 };
                    let v = t.junk(0x10, 4);
                    // the Vector check answers the target (Vector setter) or the row (Matrix setter)
                    t.stub(
                        s::LUA_TOUSERDATA,
                        false,
                        2,
                        0,
                        if f == 0x0057_36c0 { target } else { v },
                    );
                    t.stub(st_::CHECK_MATRIX, false, 2, 0, target);
                    t.stub_float_table(lb::LUA_CHECKNUMBER, 2, 1, table);
                    let what = format!("{label}: newindex {f:#x} {key:#x} {ok}");
                    t.cdecl(&what, f, &[l], |g| {
                        Some(if f == 0x0057_36c0 {
                            st::vector_newindex(g, l)
                        } else {
                            st::matrix_newindex(g, l)
                        })
                    });
                }
            }
        }
        t.unstub();
    }
}

#[test]
fn b5_execute_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        // execute: no script, no state, load failing, run failing, no entry, call failing,
        // call with and without a message
        for (k, (has_script, state, load, run, entry, call, msg)) in [
            (false, l, 0u32, 0u32, 0u32, 0u32, 0u32),
            (true, 0, 0, 0, 0, 0, 0),
            (true, l, 1, 0, 0, 0, 0),
            (true, l, 0, 3, 0, 0, 0x00c9_7700),
            (true, l, 0, 0, 0x8000_0000, 0, 0),
            (true, l, 0, 0, 1, 2, 0x00c9_7700),
            (true, l, 0, 0, 1, 0, 0x00c9_7700),
            (true, l, 0, 0, 1, 0, 0),
            (true, l, 0, 0, 0, 0, 0),
        ]
        .into_iter()
        .enumerate()
        {
            let words = match k {
                3 => 5,
                4 | 6 => 3,
                _ => 4,
            };
            lua_b5(&mut t, words);
            t.stub(mid::LUA_LOADBUFFERX, false, 5, 0, load);
            t.stub(mid::LUA_TOLSTRING, false, 3, 0, msg);
            // the first pcall answers `run`, the second `call`
            let cell = t.zeros(4);
            let mut c = vec![0xff, 0x05];
            c.extend(cell.to_le_bytes()); // inc dword [cell]
            c.extend([0x83, 0x3d]);
            c.extend(cell.to_le_bytes());
            c.push(0x01); // cmp dword [cell], 1
            c.push(0xb8);
            c.extend(run.to_le_bytes()); // mov eax, run
            c.extend([0x74, 0x05]); // je +5
            c.push(0xb8);
            c.extend(call.to_le_bytes()); // mov eax, call
            c.push(0xc3);
            let code = t.code(&c);
            t.place(mid::LUA_PCALLK, code);
            let o = t.zeros(0x20);
            let sc = if has_script { script(&mut t, 2) } else { 0 };
            t.w32(o + 4, sc);
            t.w32(o + 8, 0x00c9_6600);
            t.w32(o + 0xc, 0x00c9_6700);
            t.w32(o + 0x10, entry);
            let what = format!("{label}: execute case {k}");
            t.this(&what, 0x0061_8330, o, &[state, 1], |g| {
                Some(st::execute(g, o, state, 1))
            });
        }
        // executePersistent: no script, no state, failing, a message, none
        for (k, (has_script, state, call, msg)) in [
            (false, l, 0u32, 0u32),
            (true, 0, 0, 0),
            (true, l, 2, 0x00c9_7700),
            (true, l, 0, 0x00c9_7700),
            (true, l, 0, 0),
        ]
        .into_iter()
        .enumerate()
        {
            lua_b5(&mut t, if k == 2 { 5 } else { 3 });
            t.stub(mid::LUA_PCALLK, false, 6, 0, call);
            t.stub(mid::LUA_TOLSTRING, false, 3, 0, msg);
            let o = t.zeros(0x20);
            let sc = if has_script { script(&mut t, 2) } else { 0 };
            t.w32(o + 4, sc);
            t.w32(o + 8, 0x00c9_6600);
            t.w32(o + 0xc, 0x00c9_6700);
            t.w32(o + 0x1c, state);
            let what = format!("{label}: execute persistent case {k}");
            t.this(&what, 0x0061_8490, o, &[0], |g| {
                Some(st::execute_persistent(g, o, 0))
            });
        }
        t.unstub();
    }
}

// ---- batch 6: scheduler removal, Vector constructor, type queries ----

/// A scheduler whose live list (+0x24) holds `n` entries of `owner` with
/// keys `key0 + i`, name indices `i`, step 3 and phase `i`; the owner's
/// name table names entry 1 "tick" (by a tagged pointer) and entry 2 by
/// the same pointer as `same`.
fn sched_with(t: &mut Twins, owner: u32, n: u32, same: u32) -> (u32, Vec<u32>) {
    let s_ = scheduler(t, 0);
    let names = t.zeros(0x10);
    let tick = t.cstr("tick");
    t.w32(names + 4, tick + 1);
    t.w32(names + 8, same);
    t.w32(owner + 0xc, names);
    let es: Vec<u32> = (0..n).map(|_| t.zeros(0x20)).collect();
    for (i, &e) in es.iter().enumerate() {
        let i = i as u32;
        t.w32(e, es.get(i as usize + 1).copied().unwrap_or(0));
        t.w32(e + 4, owner);
        t.w32(e + 8, 0x10 + (i & 1));
        t.w32(e + 0x10, i);
        t.write(e + 0x14, &3u16.to_le_bytes());
        t.write(e + 0x16, &(i as u16).to_le_bytes());
    }
    t.w32(s_ + 0x24, es.first().copied().unwrap_or(0));
    (s_, es)
}

#[test]
fn b6_scheduler_entries_match_the_original() {
    for (label, mut t) in Twins::all() {
        t.stub(mo_::FREELIST_DEALLOCATE, true, 1, 1, 0xfee);
        let running = t.rebase(mo_::SCHEDULER_RUNNING);
        let old = t.r32(running);
        let same = t.cstr("same");
        let tick = t.cstr("tick");
        let empty = t.cstr("");
        let other = t.cstr("tock");
        for run in [0u32, 0x00c9_1000] {
            t.w32(running, run);
            for (owner_ok, key, name) in [
                (true, 0x10u32, 0u32),
                (true, 0x11, 0),
                (true, 0x12, 0),
                (false, 0x10, 0),
                (true, 0x11, tick),
                (true, 0x10, same),
                (true, 0x10, empty),
                (true, 0x10, other),
            ] {
                let owner = t.zeros(0x10);
                let (s_, _) = sched_with(&mut t, owner, 4, same);
                let o = if owner_ok { owner } else { 0x55 };
                let what = format!("{label}: remove {run:#x} {owner_ok} {key:#x} {name:#x}");
                t.this(&what, 0x005d_1370, s_, &[o, key, name], |g| {
                    Some(mo::scheduler_remove(g, s_, o, key, name))
                });
                if run == 0 {
                    let (s_, _) = sched_with(&mut t, owner, 4, same);
                    for step in [0u32, 1, 5, 0xffff_0002, 0x0103] {
                        let what =
                            format!("{label}: set step {owner_ok} {key:#x} {name:#x} {step:#x}");
                        t.this(&what, 0x005d_1670, s_, &[o, key, name, step], |g| {
                            Some(mo::scheduler_set_step(g, s_, o, key, name, step))
                        });
                    }
                }
            }
        }
        let owner = t.zeros(0x10);
        let (s_, _) = sched_with(&mut t, owner, 0, same);
        t.this(
            &format!("{label}: remove empty"),
            0x005d_1370,
            s_,
            &[owner, 0x10, 0],
            |g| Some(mo::scheduler_remove(g, s_, owner, 0x10, 0)),
        );
        t.w32(running, old);
        t.unstub();
    }
}

#[test]
fn b6_lua_objects_match_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        let table = t.alloc(
            &[0u32, 0x3f80_0000, 0xc020_0000, 0x4049_0fdb, 0x7fc0_0001]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        for top in [0u32, 1, 2, 4, 6, 0xffff_ffff] {
            for ok in [false, true] {
                lua_b3(&mut t, 0, 1, 1, 0, 3);
                let u = if ok { t.junk(0x10, 1) } else { 0 };
                t.stub(s::LUA_NEWUSERDATA, false, 2, 0, u);
                t.stub(lb::LUA_GETTOP, false, 1, 0, top);
                t.stub_float_table(lb::LUA_CHECKNUMBER, 2, 1, table);
                let what = format!("{label}: vector new {top:#x} {ok}");
                t.cdecl(&what, 0x0057_0f80, &[l], |g| Some(mo::vector_new(g, l)));
            }
        }
        t.stub(s::LUA_CHECKSTACK, false, 2, 0, 0x0c5);
        for (u, d, ud) in [
            (false, 0u32, true),
            (true, 0, true),
            (true, 0x00c9_2200, true),
            (true, 0x00c9_2200, false),
        ] {
            let p = if u {
                let p = t.zeros(8);
                t.w32(p, d);
                t.w32(p + 4, 0x00c9_3300);
                p
            } else {
                0
            };
            lua_b3(&mut t, p, 1, 1, 0, 2);
            let out = if ud { t.junk(8, 1) } else { 0 };
            t.stub(s::LUA_NEWUSERDATA, false, 2, 0, out);
            let what = format!("{label}: type of {u} {d:#x} {ud}");
            t.cdecl(&what, 0x0043_6b90, &[l], |g| Some(mo::get_type_of(g, l)));
        }
        // equality: neither, one, both with the same and other objects
        t.stub(mo_::LUA_PUSHBOOLEAN, false, 2, 0, 0xb0);
        let ud = |t: &mut Twins, obj: u32| {
            let p = t.zeros(8);
            t.w32(p, 0x00c9_2200);
            t.w32(p + 4, obj);
            p
        };
        for (a, b_) in [(0u32, 0u32), (1, 0), (0, 1), (1, 1), (1, 2)] {
            let pa = if a == 0 { 0 } else { ud(&mut t, 0x00c9_3300) };
            let pb = match b_ {
                0 => 0,
                1 => ud(&mut t, 0x00c9_3300),
                _ => ud(&mut t, 0x00c9_4400),
            };
            lua_b3(&mut t, 0, 1, 1, 0, 3);
            // lua_touserdata answers argument 1's, then argument 2's
            let cell = t.zeros(4);
            let mut c = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8] (the index)
            c.extend([0x83, 0xf8, 0x01]); // cmp eax, 1
            c.push(0xb8);
            c.extend(pa.to_le_bytes()); // mov eax, pa
            c.extend([0x74, 0x05]); // je +5
            c.push(0xb8);
            c.extend(pb.to_le_bytes()); // mov eax, pb
            c.extend([0xff, 0x05]);
            c.extend(cell.to_le_bytes()); // inc dword [cell]
            c.push(0xc3);
            let code = t.code(&c);
            t.place(s::LUA_TOUSERDATA, code);
            let what = format!("{label}: objects equal {a} {b_}");
            t.cdecl(&what, 0x0043_6aa0, &[l], |g| Some(mo::objects_equal(g, l)));
        }
        t.unstub();
    }
}

// ---- batch 7: the scheduler runs ----

/// A scheduler for the runs: `n` entries on its live list (+0x24) with
/// periods `steps`, phases `i`, states `+0x18`, own Lua states for odd
/// entries when `own`; a pool of `cap` 20-byte slots of which `used` are
/// taken.
fn run_sched(t: &mut Twins, steps: &[u16], own: bool, cap: u32, used: u32) -> (u32, Vec<u32>) {
    let s_ = scheduler(t, 0);
    let es: Vec<u32> = steps.iter().map(|_| t.zeros(0x20)).collect();
    for (i, (&e, &step)) in es.iter().zip(steps).enumerate() {
        let i = i as u32;
        t.w32(e, es.get(i as usize + 1).copied().unwrap_or(0));
        t.w32(e + 4, 0x00c9_0400);
        t.w32(e + 8, 0x10 + i);
        t.w32(e + 0x10, 0x20 + i);
        t.write(e + 0x14, &step.to_le_bytes());
        t.write(e + 0x16, &(i as u16).to_le_bytes());
        t.w32(e + 0x18, u32::from(i == 2));
        t.w32(e + 0x1c, if own && i % 2 == 1 { 0x00c9_1100 } else { 0 });
    }
    t.w32(s_ + 0x24, es.first().copied().unwrap_or(0));
    let pool = t.junk(20 * 4, 2);
    t.w32(s_ + 0x28, cap);
    t.w32(s_ + 0x2c, pool);
    t.w32(s_ + 0x30, used);
    (s_, es)
}

#[test]
fn b7_scheduler_runs_match_the_original() {
    for (label, mut t) in Twins::all() {
        let running = t.rebase(ru_::SCHEDULER_RUNNING);
        let old = t.r32(running);
        t.stub(ru_::FREELIST_DEALLOCATE, true, 1, 1, 0xfee);
        t.stub(ru_::SLOT_STATE, true, 0, 0, 0x00c9_1200);
        for (r_exec, r_pers) in [(0u32, 0u32), (5, 6)] {
            // the runs record the current entry (0x00cbd9bc) they see
            let cur = t.rebase(ru_::CURRENT_ENTRY);
            let seen = t.zeros(4);
            let mut pre = vec![0xa1];
            pre.extend(cur.to_le_bytes()); // mov eax, [cur]
            pre.push(0xa3);
            pre.extend(seen.to_le_bytes()); // mov [seen], eax
            let c = t.stub_code_with(&pre, ru_::EXECUTE, true, 2, 2, r_exec);
            t.place(ru_::EXECUTE, c);
            let c = t.stub_code_with(&pre, ru_::EXECUTE_PERSISTENT, true, 1, 1, r_pers);
            t.place(ru_::EXECUTE_PERSISTENT, c);
            for (own, cap, used) in [
                (false, 4u32, 0u32),
                (true, 4, 0),
                (false, 0x8000_0001, 1),
                (false, 4, 2),
                (false, 2, 2),
            ] {
                for frame in [0u32, 1, 2, 6, 0xffff_ffff] {
                    t.w32(running, 0);
                    let (s_, _) = run_sched(&mut t, &[1, 2, 3, 0, 4], own, cap, used);
                    let what =
                        format!("{label}: tick {r_exec} {r_pers} {own} {cap:#x} {used} {frame:#x}");
                    t.this(&what, 0x005d_0910, s_, &[frame], |g| {
                        Some(ru::tick(g, s_, frame))
                    });
                }
                t.w32(running, 0);
                let (s_, es) = run_sched(&mut t, &[1, 2, 3], own, cap, used);
                // the one-shot list holds the same entries
                let list = t.zeros(4);
                t.w32(list, es[0]);
                t.w32(s_ + 0x24, 0);
                let what = format!("{label}: run list {r_exec} {r_pers} {own} {cap:#x} {used}");
                t.this(&what, 0x005d_0670, s_, &[list], |g| {
                    Some(ru::run_list(g, s_, list))
                });
                // run now: found and not, a known name or not
                let cell = t.b.stack_range().start + 0x40;
                for (key, idx) in [
                    (0x11u32, 3u32),
                    (0x10, 1),
                    (0x10, 0x8000_0000),
                    (0x99, 1),
                    (0x10, 0),
                    (0x11, 7),
                ] {
                    t.stub_deref0(ru_::NAME_INDEX, 2, 2, idx);
                    t.w32(running, 0);
                    let (s_, _) = run_sched(&mut t, &[1, 2, 3], own, cap, used);
                    let name = if idx == 7 { t.cstr("") } else { t.cstr("tick") };
                    let what = format!(
                        "{label}: run now {r_exec} {own} {cap:#x} {used} {key:#x} {idx:#x}"
                    );
                    t.this(&what, 0x005d_07d0, s_, &[0x00c9_0400, key, name], |g| {
                        Some(ru::run_now(g, s_, 0x00c9_0400, key, name, cell))
                    });
                }
            }
        }
        // already running
        t.w32(running, 0x00c9_9000);
        let (s_, _) = run_sched(&mut t, &[1], false, 4, 0);
        t.this(
            &format!("{label}: tick nested"),
            0x005d_0910,
            s_,
            &[0],
            |g| Some(ru::tick(g, s_, 0)),
        );
        let list = t.zeros(4);
        t.this(
            &format!("{label}: run list nested"),
            0x005d_0670,
            s_,
            &[list],
            |g| Some(ru::run_list(g, s_, list)),
        );
        t.w32(running, old);
        t.unstub();
    }
}

// ---- batch 8: Vector dot and cross products ----

#[test]
fn b8_vector_products_match_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        let mut rng = Rng(0x1357_2468);
        for k in 0..60u32 {
            let a = t.zeros(0x10);
            let b_ = t.zeros(0x10);
            for i in 0..4 {
                let w = |r: u32| (r & 0x8000_0000) | (0x3a00_0000 + (r & 0x0fff_ffff));
                let (x, y) = if k < 10 {
                    (
                        FLOATS[(k + i) as usize % FLOATS.len()],
                        FLOATS[(k * 3 + i) as usize % FLOATS.len()],
                    )
                } else {
                    (w(rng.next()), w(rng.next()))
                };
                t.w32(a + 4 * i, x);
                t.w32(b_ + 4 * i, y);
            }
            // either argument missing, then both
            for (pa, pb) in [(0u32, b_), (a, 0), (a, b_)] {
                lua_b3(&mut t, 0, 1, 1, 0, 3);
                let c = vec_args(&mut t, pa, pb);
                t.place(s::LUA_TOUSERDATA, c);
                let out = t.junk(0x10, 3);
                t.stub(s::LUA_NEWUSERDATA, false, 2, 0, out);
                for &(f, n) in &ve_::DOTS {
                    let what = format!("{label}: dot{n} #{k} {pa:#x} {pb:#x}");
                    t.cdecl(&what, f, &[l], |g| Some(ve::dot(g, l, n)));
                }
                let what = format!("{label}: cross #{k} {pa:#x} {pb:#x}");
                t.cdecl(&what, 0x0057_2690, &[l], |g| Some(ve::cross(g, l)));
            }
        }
        t.unstub();
    }
}

/// A `lua_touserdata` stand-in answering `a` for argument 1 and `b` for
/// argument 2.
fn vec_args(t: &mut Twins, a: u32, b: u32) -> u32 {
    let mut c = vec![0x8b, 0x44, 0x24, 0x08]; // mov eax, [esp+8] (the index)
    c.extend([0x83, 0xf8, 0x01]); // cmp eax, 1
    c.push(0xb8);
    c.extend(a.to_le_bytes()); // mov eax, a
    c.extend([0x74, 0x05]); // je +5
    c.push(0xb8);
    c.extend(b.to_le_bytes()); // mov eax, b
    c.push(0xc3);
    t.code(&c)
}

// ---- batch 9: Vector arithmetic ----

#[test]
fn b9_vector_arithmetic_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        let mut rng = Rng(0x0bad_cafe);
        let w = |r: u32| (r & 0x8000_0000) | (0x3a00_0000 + (r & 0x0fff_ffff));
        for k in 0..30u32 {
            let a = t.zeros(0x10);
            let b_ = t.zeros(0x10);
            for i in 0..4 {
                let (x, y) = if k < 10 {
                    (
                        FLOATS[(k + i) as usize % FLOATS.len()],
                        FLOATS[(k * 3 + i) as usize % FLOATS.len()],
                    )
                } else {
                    (w(rng.next()), w(rng.next()))
                };
                t.w32(a + 4 * i, x);
                t.w32(b_ + 4 * i, y);
            }
            let table = t.alloc(
                &[0u32, w(rng.next()), FLOATS[k as usize % FLOATS.len()]]
                    .iter()
                    .flat_map(|x| x.to_le_bytes())
                    .collect::<Vec<_>>(),
            );
            for (pa, pb) in [(0u32, 0u32), (0, b_), (a, 0), (a, b_)] {
                lua_b3(&mut t, 0, 1, 1, 0, 3);
                let c = vec_args(&mut t, pa, pb);
                t.place(s::LUA_TOUSERDATA, c);
                let out = t.junk(0x10, 3);
                t.stub(s::LUA_NEWUSERDATA, false, 2, 0, out);
                t.stub_float_table(ve_::LUA_CHECKNUMBER, 2, 1, table);
                for &(f, op) in &ve_::ARITH {
                    if pa == 0 && pb == 0 && op == ve_::Arith::Div {
                        // the original divides by the words at address 0 (a fault)
                        continue;
                    }
                    let what = format!("{label}: {op:?} #{k} {pa:#x} {pb:#x}");
                    t.cdecl(&what, f, &[l], |g| Some(ve::arith(g, l, op)));
                }
            }
        }
        t.unstub();
    }
}

// ---- batch 10: Vector and Matrix equality ----

#[test]
fn b10_equality_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        t.stub(mo_::LUA_PUSHBOOLEAN, false, 2, 0, 0xb0);
        for &(f, _, words) in &ve_::EQUALS {
            // equal, one component off at each place, signed zeros, NaNs
            let mut cases: Vec<(Vec<u32>, Vec<u32>)> = Vec::new();
            let base: Vec<u32> = (0..words).map(|i| 0x3f80_0000 + i * 0x1000).collect();
            cases.push((base.clone(), base.clone()));
            for k in 0..words {
                let mut b_ = base.clone();
                b_[k as usize] ^= 1;
                cases.push((base.clone(), b_));
            }
            let mut z = base.clone();
            z[0] = 0x8000_0000;
            let mut z2 = base.clone();
            z2[0] = 0;
            cases.push((z, z2));
            let mut n = base.clone();
            n[words as usize - 1] = 0x7fc0_0000;
            cases.push((n.clone(), n));
            for (k, (x, y)) in cases.iter().enumerate() {
                let a = t.alloc(&x.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>());
                let b_ = t.alloc(&y.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>());
                for (pa, pb) in [(0u32, b_), (a, 0), (a, b_)] {
                    lua_b3(&mut t, 0, 1, 1, 0, 3);
                    let c = vec_args(&mut t, pa, pb);
                    t.place(s::LUA_TOUSERDATA, c);
                    let what = format!("{label}: equals {f:#x} case {k} {pa:#x} {pb:#x}");
                    t.cdecl(&what, f, &[l], |g| {
                        Some(ve::equals(
                            g,
                            l,
                            ve_::EQUALS[usize::from(words == 16)].1,
                            words,
                        ))
                    });
                }
            }
        }
        t.unstub();
    }
}

// ---- batch 11: bit-packed pairs, mesh target binding ----

#[test]
fn b11_decode_and_bind_match_the_original() {
    for (label, mut t) in Twins::all() {
        let mut rng = Rng(0x5eed_0011);
        for count in [0u32, 1, 3, 6, 15] {
            for _ in 0..4 {
                let mut words: Vec<u32> = (0..8).map(|_| rng.next()).collect();
                words[0] = (words[0] & !0xf) | count;
                let bits = t.alloc(
                    &words
                        .iter()
                        .flat_map(|w| w.to_le_bytes())
                        .collect::<Vec<_>>(),
                );
                let cursor = t.zeros(4);
                t.w32(cursor, bits);
                let a = t.junk(0x40, 1);
                let b_ = t.junk(0x40, 2);
                let what = format!("{label}: decode pairs {count}");
                t.cdecl(&what, 0x004d_3d30, &[a, b_, cursor], |g| {
                    Some(mo::decode_pairs(g, a, b_, cursor))
                });
            }
        }
        // mesh target binding
        let class = t.rebase(mo_::MESH_INSTANCE_CLASS);
        let seg_names: Vec<u32> = ["arm", "leg", "", "arm"]
            .iter()
            .map(|n| t.cstr(n))
            .collect();
        // 0x104 names (bit 31 of the count set): four at the start, one at 0x100
        let names = t.zeros(0x104 * 4);
        for (i, &n) in seg_names.iter().enumerate() {
            t.w32(names + 4 * i as u32, if i == 2 { 0 } else { n + 1 });
        }
        t.w32(names + 4 * 0x100, seg_names[0]);
        let first = t.zeros(0x30);
        t.w32(first + 0x24, names);
        t.w32(first + 0x18, 0x8000_0104);
        let ctx = t.zeros(0x10);
        t.w32(ctx, first);
        t.w32(ctx + 0xc, 0x00c9_a000);
        let this = t.zeros(8);
        t.w32(this + 4, ctx);
        let tname = t.cstr("arm");
        for (matches, n, null) in [
            (0u32, 3u32, false),
            (0x101, 3, false),
            (1, 3, false),
            (1, 0, false),
            (1, 3, true),
            (1, 1, false),
            (0x100, 3, false),
        ] {
            t.stub(mid::NAME_IS, true, 1, 1, matches);
            let targets = t.zeros(0x1c * 3);
            let out = t.zeros(0x18 * 3);
            for k in 0..3u32 {
                let tg = targets + 0x1c * k;
                // a chain reaching the class (via a parent), another class, kind 0
                let d = t.zeros(0x48);
                t.w32(d + 0x40, if k == 1 { 0x55 } else { class });
                if k == 1 {
                    let other = t.zeros(0x48);
                    t.w32(d + 0x40, other);
                }
                t.w32(tg, d);
                t.w32(tg + 0x10, if k == 2 { 2 } else { 1 });
                t.w32(tg + 0x14, tname + 1);
            }
            // a result already set for the first target
            if matches == 0x101 {
                t.w32(out, 7);
            }
            let (pt, po) = if null { (0, out) } else { (targets, out) };
            let what = format!("{label}: bind targets {matches:#x} {n} {null}");
            t.this(&what, 0x005c_e320, this, &[po, pt, n], |g| {
                Some(mo::bind_mesh_targets(g, this, po, pt, n))
            });
        }
        t.unstub();
    }
}

// ---- batch 12: the Phyre namespace index ----

#[test]
fn b12_phyre_index_matches_the_original() {
    for (label, mut t) in Twins::all() {
        let l = 0x00c9_1100;
        t.stub(s::GLOBAL_NAMESPACE, false, 1, 0, 0x00c9_0b00);
        t.stub(mo_::PUSH_CLASS, false, 2, 0, 0x9c);
        t.stub(s::LUA_CREATETABLE, false, 3, 0, 0xc7);
        t.stub(lb::LUA_GETTOP, false, 1, 0, 5);
        t.stub(mo_::LUA_PUSHLSTRING, false, 3, 0, 0x15);
        t.stub(s::LUA_PUSHLIGHTUSERDATA, false, 2, 0, 0x16);
        t.stub(mid::LUA_PUSHCCLOSURE, false, 3, 0, 0x17);
        t.stub(mo_::LUA_RAWSET, false, 2, 0, 0x18);
        t.stub(mo_::LUA_PUSHVALUE, false, 2, 0, 0x19);
        t.stub(s::LUA_SETMETATABLE, false, 2, 0, 0x1a);
        let keys = [
            t.cstr("PNode"),
            t.cstr("GetTypeOf"),
            t.cstr("GetType"),
            t.cstr(""),
        ];
        for key in keys.iter().copied().chain([0]) {
            for (class, obj) in [(0u32, 0u32), (0x00c9_c100, 0), (0, 0x00c9_d100)] {
                t.stub(mid::LUA_TOLSTRING, false, 3, 0, key);
                t.stub(mo_::NAMESPACE_CLASS, true, 1, 1, class);
                t.stub(mo_::FIND_NAMED, false, 1, 0, obj);
                let what = format!("{label}: phyre index {key:#x} {class:#x} {obj:#x}");
                t.cdecl(&what, 0x0043_6c80, &[l], |g| Some(mo::phyre_index(g, l)));
            }
        }
        t.unstub();
    }
}

// ---- round left2 (ffx_pother_a::m5p_scripting_left2) ----

/// A stub prefix that replaces stack word `i` by the word it points at
/// (a frame's address differs between the two runs; what it holds must
/// not).
fn deref_arg(i: u8) -> Vec<u8> {
    let off = 4 + 4 * i;
    vec![
        0x8b, 0x44, 0x24, off, // mov eax, [esp+off]
        0x8b, 0x00, // mov eax, [eax]
        0x89, 0x44, 0x24, off, // mov [esp+off], eax
    ]
}

/// A stub prefix that stores `v` where stack word 0 points.
fn store_arg0(v: u32) -> Vec<u8> {
    let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00]; // mov eax, [esp+4]; mov dword [eax], v
    c.extend(v.to_le_bytes());
    c
}

#[test]
fn left2_cloth_create_instance_matches_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        t.stub(l2::CLOTH_INSTANCE[2], true, 1, 1, 0xc1c1);
        let with = t.zeros(0x40);
        t.w32(with + 0x34, 0x0055_0000);
        let without = t.zeros(0x40);
        for (existing, list) in [(0x77u32, with), (0, 0), (0, without), (0, with)] {
            t.stub(l2::CLOTH_INSTANCE[0], true, 3, 3, list);
            let what =
                format!("{label}: cloth createInstance existing {existing:#x} list {list:#x}");
            t.this(
                &what,
                0x006e_1ec0,
                0x0012_0000,
                &[0x0034_0000, existing],
                |g| {
                    Some(l2b::cloth_create_instance(
                        g,
                        0x0012_0000,
                        0x0034_0000,
                        existing,
                    ))
                },
            );
        }
        t.unstub();
    }
}

#[test]
fn left2_require_matches_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        let l = 0x00d0_0000;
        let cb1 = t.stub_code(0x00cb_e6b4, false, 4, 4, 0);
        let cb2 = t.stub_code(0x00cb_e6c8, false, 4, 4, 0);
        for (name, word, loaded, source, script, callbacks) in [
            (0u32, 0x0060_0000u32, 1u32, 1u32, 1u32, 3u32),
            (0x0061_0000, 0x0060_0000, 0, 1, 1, 3),
            (0x0061_0000, 0x0060_0001, 0x100, 1, 1, 3),
            (0x0061_0000, 0x0060_0000, 1, 0, 1, 3),
            (0x0061_0000, 0x0060_0001, 1, 0x0062_0000, 0, 3),
            (0x0061_0000, 0x0060_0000, 1, 0x0062_0000, 0x0063_0000, 3),
            (0x0061_0000, 0x0060_0000, 1, 0x0062_0000, 0x0063_0000, 1),
            (0x0061_0000, 0x0060_0000, 1, 0x0062_0000, 0x0063_0000, 2),
            (0x0061_0000, 0x0060_0000, 1, 0x0062_0000, 0x0063_0000, 0),
        ] {
            t.stub(l2::LUAL_CHECKLSTRING, false, 3, 0, name);
            t.stub(l2::LUA_SETTOP, false, 2, 0, 0);
            let mut pre = store_arg0(word);
            pre.extend(deref_arg(0));
            let s = t.stub_code_with(&pre, l2::REQUIRE_CALLS[0], false, 2, 0, 0);
            t.place(l2::REQUIRE_CALLS[0], s);
            t.stub(l2::REQUIRE_CALLS[1], false, 3, 0, loaded);
            t.stub(l2::LUA_GETFIELD, false, 3, 0, 0);
            t.stub(l2::LUA_PUSHNIL, false, 1, 0, 0);
            t.stub(l2::LUA_SETFIELD, false, 3, 0, 0);
            t.stub(l2::LUA_SETGLOBAL, false, 2, 0, 0);
            t.stub(l2::REQUIRE_CALLS[2], false, 1, 0, source);
            t.stub(l2::REQUIRE_CALLS[3], false, 1, 0, script);
            t.stub(l2::FREE, false, 1, 0, 0);
            let c1 = t.rebase(l2::LOAD_CALLBACKS[0]);
            t.w32(c1, if callbacks & 1 != 0 { cb1 } else { 0 });
            let u1 = t.rebase(l2::LOAD_CALLBACKS[1]);
            t.w32(u1, 0x1111);
            let c2 = t.rebase(l2::LOAD_CALLBACKS[2]);
            t.w32(c2, if callbacks & 2 != 0 { cb2 } else { 0 });
            let u2 = t.rebase(l2::LOAD_CALLBACKS[3]);
            t.w32(u2, 0x2222);
            let what = format!(
                "{label}: require name {name:#x} word {word:#x} loaded {loaded:#x} source {source:#x} script {script:#x} callbacks {callbacks}"
            );
            t.cdecl(&what, 0x005d_67b0, &[l], |g| {
                let s = ffx_graphic::host::Host::scratch(g);
                Some(l2b::require(g, l, s))
            });
            t.unstub();
        }
    }
}

#[test]
fn left2_reload_script_matches_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        for (component, state, index, err, ty, boolean, word) in [
            (0u32, 1u32, 0u32, 0u32, 1u32, 1u32, 0x0060_0000u32),
            (1, 0, 0, 0, 1, 1, 0x0060_0000),
            (1, 1, 0xffff_ffff, 0, 1, 1, 0x0060_0000),
            (1, 1, 2, 5, 1, 1, 0x0060_0000),
            (1, 1, 0, 0, 1, 1, 0x0060_0001),
            (1, 1, 0, 0, 1, 0, 0x0060_0000),
            (1, 1, 0, 0, 4, 1, 0x0060_0000),
        ] {
            t.stub(l2::PRINTF, false, 4, 0, 0);
            t.stub(l2::RELOAD_CALLS[0], true, 2, 2, index);
            let mut pre = store_arg0(word);
            pre.extend(deref_arg(0));
            let s = t.stub_code_with(&pre, l2::REQUIRE_CALLS[0], false, 2, 0, 0);
            t.place(l2::REQUIRE_CALLS[0], s);
            t.stub(l2::LUA_GETGLOBAL, false, 2, 0, 0);
            t.stub(l2::RELOAD_CALLS[1], false, 3, 0, 0);
            let s = t.stub_code_with(&deref_arg(1), l2::RELOAD_CALLS[2], false, 3, 0, 0);
            t.place(l2::RELOAD_CALLS[2], s);
            t.stub(l2::LUA_PCALLK, false, 6, 0, err);
            t.stub(l2::LUA_TOLSTRING, false, 3, 0, 0x0064_0000);
            t.stub(l2::LUA_TYPE, false, 2, 0, ty);
            t.stub(l2::LUA_TOBOOLEAN, false, 2, 0, boolean);
            t.stub(l2::LUA_SETTOP, false, 2, 0, 0);
            t.stub(l2::FREE, false, 1, 0, 0);
            let this = t.object(0x7700, &[0, 3, 0], &[]);
            let names = t.zeros(0x40);
            t.w32(names + 0x1c, 0x0065_0001);
            t.w32(this + 0x14, names);
            t.w32(this + 0x1c, if state != 0 { 0x00d0_0000 } else { 0 });
            t.w32(this + 0xc, 0x24);
            let comp = if component != 0 {
                let c = t.zeros(0x40);
                t.w32(c + 0x1c, 0x0066_0000);
                c
            } else {
                0
            };
            let what = format!(
                "{label}: reloadScript component {component} state {state} index {index:#x} err {err} type {ty} bool {boolean} word {word:#x}"
            );
            t.this(&what, 0x0055_32a0, this, &[comp], |g| {
                let s = ffx_graphic::host::Host::scratch(g);
                l2b::reload_script(g, this, comp, s);
                None
            });
            t.unstub();
        }
    }
}

#[test]
fn left2_member_path_matches_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        let paths: Vec<u32> = ["a.b.c", "single", "end.", "", ".lead", "x..y"]
            .iter()
            .map(|s| t.cstr(s))
            .collect();
        for found in [true, false] {
            for block in [true, false] {
                for &path in paths.iter().chain([0u32].iter()) {
                    t.stub(l2::FREE, false, 1, 0, 0);
                    let b = if block { t.junk(0x20, 9) } else { 0 };
                    t.stub(l2::MALLOC, false, 2, 0, b);
                    t.stub(l2::MEMCPY, false, 3, 0, b);
                    let next_class = t.zeros(0x40);
                    let ty = t.object(0x7800, &[0, 0], &[0, next_class]);
                    let member = t.zeros(0x40);
                    t.w32(member + 0x18, ty);
                    t.stub(l2::FIND_MEMBER, true, 1, 1, if found { member } else { 0 });
                    let what =
                        format!("{label}: member path {path:#x} found {found} block {block}");
                    let mut al = 0u8;
                    let eax = t.cdecl(&what, 0x0050_be90, &[0x0040_0000, path], |g| {
                        al = l2b::has_member_path(g, 0x0040_0000, path);
                        None
                    });
                    assert_eq!(eax & 0xff, u32::from(al), "{what}: al");
                    t.unstub();
                }
            }
        }
    }
}

/// Every Lua API call the registration ports make, stubbed: `meta` is
/// what `luaL_newmetatable` answers, `top` `lua_gettop`.
fn stub_lua_registration(t: &mut Twins, meta: u32, top: u32) {
    use ffx_pother_a::m5p_scripting_left2 as l2;
    let words = [2u32, 3, 1, 3, 3, 2, 2, 2, 3, 2, 4, 3];
    for (i, (&f, &n)) in l2::LUA_MORE.iter().zip(&words).enumerate() {
        let ret = match i {
            0 => meta,
            2 => top,
            _ => 0x100 + i as u32,
        };
        t.stub(f, false, n, 0, ret);
    }
    t.stub(l2::LUA_GETFIELD, false, 3, 0, 0);
    t.stub(l2::LUA_SETTOP, false, 2, 0, 0);
    t.stub(l2::LUA_SETGLOBAL, false, 2, 0, 0x5e7);
}

#[test]
fn left2_register_types_match_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        let l = 0x00d0_0000;
        // a namespace: a class with methods, one with properties, one with
        // neither, one whose lists are 0
        let ns = t.zeros(0x40);
        let head = ns + 8;
        let mut links = vec![head];
        for kind in 0..4u32 {
            let d = t.zeros(0x80);
            t.w32(d + 0x18, 0x0070_0000 + kind);
            let (m, p) = (d + 0x54, d + 0x5c);
            t.w32(m, if kind == 0 { 0x1234 } else { m });
            t.w32(p, if kind == 1 { 0x5678 } else { p });
            if kind == 3 {
                t.w32(m, 0);
                t.w32(p, 0);
            }
            links.push(d + 0x2c);
        }
        for i in 0..links.len() {
            t.w32(links[i], links[(i + 1) % links.len()]);
        }
        for meta in [0u32, 1] {
            for global in [false, true] {
                stub_lua_registration(&mut t, meta, 7);
                t.stub(l2::TYPES_DATA[3], false, 3, 0, 0);
                t.stub(
                    l2::GLOBAL_NAMESPACE,
                    false,
                    0,
                    0,
                    if global { ns } else { 0x0050_0000 },
                );
                t.stub(l2::REGISTER_VECTOR_MATRIX, false, 1, 0, 0x5151);
                let what = format!("{label}: RegisterTypes meta {meta} global {global}");
                t.cdecl(&what, 0x0043_6f90, &[l, ns, 0x0071_0000], |g| {
                    Some(l2b::register_types(g, l, ns, 0x0071_0000))
                });
                t.unstub();
            }
        }
        // an empty namespace and one cut by a 0 link
        let empty = t.zeros(0x40);
        t.w32(empty + 8, empty + 8);
        let cut = t.zeros(0x40);
        t.w32(cut + 8, 0x2c);
        for n in [empty, cut] {
            stub_lua_registration(&mut t, 1, 7);
            t.stub(l2::TYPES_DATA[3], false, 3, 0, 0);
            t.stub(l2::GLOBAL_NAMESPACE, false, 0, 0, 0x0050_0000);
            t.stub(l2::REGISTER_VECTOR_MATRIX, false, 1, 0, 0x5151);
            let what = format!("{label}: RegisterTypes ns {n:#x}");
            t.cdecl(&what, 0x0043_6f90, &[l, n, 0x0071_0000], |g| {
                Some(l2b::register_types(g, l, n, 0x0071_0000))
            });
            t.unstub();
        }
        for meta in [0u32, 1] {
            stub_lua_registration(&mut t, meta, 9);
            // lua_gettop counting up, so the two tops it names differ
            let cell = t.zeros(4);
            t.w32(cell, 9);
            let mut c = vec![0xa1];
            c.extend(cell.to_le_bytes()); // mov eax, [cell]
            c.extend([0x83, 0x05]);
            c.extend(cell.to_le_bytes());
            c.push(1); // add dword [cell], 1
            c.push(0xc3);
            let code = t.code(&c);
            t.place(l2::LUA_MORE[2], code);
            let what = format!("{label}: RegisterVectorMatrixTypes meta {meta}");
            t.cdecl(&what, 0x0057_21b0, &[l], |g| {
                Some(l2b::register_vector_matrix(g, l))
            });
            t.unstub();
        }
    }
}

#[test]
#[allow(clippy::type_complexity)] // one row per case, columns named above
fn left2_require_module_matches_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        let l = 0x00d0_0000;
        let cb1 = t.stub_code(0x00cb_e6b4, false, 4, 4, 0);
        let cb3 = t.stub_code(0x00cb_e6c4, false, 4, 4, 0);
        // (name, last state, depth, max, loaded, opener, sources, source,
        //  compiled, err, script, callbacks, top)
        let cases: [(
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
        ); 16] = [
            (0x61, l, 3, 4, 1, 0, 1, 1, 1, 0, 1, 3, 1),
            (0x61, 0, 0, 0, 1, 0, 1, 1, 1, 0, 1, 3, 0),
            (0x61, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 3, 1),
            (0x61, l, 0x15, 0x15, 0, 0, 1, 1, 1, 0, 1, 3, 1),
            (0x61, l, 0x14, 0x14, 0, 0, 1, 1, 1, 0, 1, 3, 1),
            (0x61, l, 3, 9, 1, 0, 1, 1, 1, 0, 1, 3, 2),
            (0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 3, 2),
            (0x61, 0, 0, 0, 0x100, 0x77, 1, 1, 1, 0, 1, 3, 1),
            (0x61, 0, 0, 0, 0, 0x77, 1, 1, 1, 0, 1, 3, 1),
            (0x61, 0, 0, 0, 0, 0, 0, 1, 1, 0, 1, 3, 1),
            (0x61, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 3, 1),
            (0x61, 0, 0, 0, 0, 0, 1, 1, 0x100, 0, 1, 3, 1),
            (0x61, 0, 0, 0, 0, 0, 1, 1, 1, 4, 1, 3, 1),
            (0x61, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 3, 1),
            (0x61, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 1, 1),
            (0x61, 0, 0, 0, 0, 0, 1, 1, 1, 0, 1, 2, 1),
        ];
        for (
            i,
            &(
                name,
                last,
                depth,
                max,
                loaded,
                opener,
                sources,
                source,
                compiled,
                err,
                script,
                callbacks,
                top,
            ),
        ) in cases.iter().enumerate()
        {
            stub_lua_registration(&mut t, 1, top);
            let name = if name == 0 { 0 } else { t.cstr("module") };
            t.stub(l2::LUAL_CHECKLSTRING, false, 3, 0, name);
            t.stub(l2::LUA_PUSHNIL, false, 1, 0, 0);
            t.stub(l2::LUA_SETFIELD, false, 3, 0, 0);
            t.stub(l2::LUA_GETGLOBAL, false, 2, 0, 0);
            // the messages take 3 words, the call error's 4 (a stub
            // logging more would log the original's stale stack)
            t.stub(l2::PRINTF, false, if err != 0 { 4 } else { 3 }, 0, 0);
            let word = if i % 2 == 0 { 0x0060_0000 } else { 0x0060_0001 };
            let mut pre = store_arg0(word);
            pre.extend(deref_arg(0));
            let s = t.stub_code_with(&pre, l2::REQUIRE_CALLS[0], false, 2, 0, 0);
            t.place(l2::REQUIRE_CALLS[0], s);
            t.stub(l2::REQUIRE_CALLS[1], false, 3, 0, loaded);
            t.stub(l2::REQUIRE_MORE[0], false, 1, 0, opener);
            t.stub(
                l2::REQUIRE_CALLS[2],
                false,
                1,
                0,
                if source != 0 { 0x0062_0000 } else { 0 },
            );
            t.stub(l2::REQUIRE_MORE[1], false, 2, 0, compiled);
            t.stub(l2::LUA_PCALLK, false, 6, 0, err);
            t.stub(l2::LUA_TOLSTRING, false, 3, 0, 0x0064_0000);
            t.stub(
                l2::REQUIRE_CALLS[3],
                false,
                1,
                0,
                if script != 0 { 0x0063_0000 } else { 0 },
            );
            t.stub(l2::FREE, false, 1, 0, 0);
            let state = l2::REQUIRE_STATE.map(|a| t.rebase(a));
            t.w32(state[0], last);
            t.w32(state[1], depth);
            t.w32(state[2], max);
            t.w32(state[3], sources);
            t.w32(state[4], if callbacks & 2 != 0 { cb3 } else { 0 });
            t.w32(state[5], 0x3333);
            let c1 = t.rebase(l2::LOAD_CALLBACKS[0]);
            t.w32(c1, if callbacks & 1 != 0 { cb1 } else { 0 });
            let u1 = t.rebase(l2::LOAD_CALLBACKS[1]);
            t.w32(u1, 0x1111);
            let what = format!("{label}: Require case {i}");
            t.cdecl(&what, 0x005d_63a0, &[l], |g| {
                let s = ffx_graphic::host::Host::scratch(g);
                Some(l2b::require_module(g, l, s))
            });
            t.unstub();
        }
    }
}

/// The stubs both index handlers call; `object` picks how the userdata
/// at 1 resolves (0 none, 1 no metatable, 2 another metatable, 3 a
/// `PhyreObject`), `ud` is its block.
fn stub_index_common(t: &mut Twins, object: u32, ud: u32, key: u32) {
    use ffx_pother_a::m5p_scripting_left2 as l2;
    t.stub(
        l2::LUA_INDEX[0],
        false,
        2,
        0,
        if object == 0 { 0 } else { ud },
    );
    t.stub(l2::LUA_INDEX[1], false, 2, 0, u32::from(object >= 2));
    t.stub(l2::LUA_GETFIELD, false, 3, 0, 0);
    t.stub(l2::LUA_INDEX[2], false, 3, 0, u32::from(object == 3));
    t.stub(l2::LUA_SETTOP, false, 2, 0, 0);
    t.stub(l2::LUA_TOLSTRING, false, 3, 0, key);
    // luaL_error: the state, the message and the key (the names after it
    // vary in number; the message tells the paths apart)
    t.stub(l2::LUA_INDEX[7], false, 3, 0, 0);
    for f in [
        l2::LUA_INDEX[3],
        l2::LUA_INDEX[4],
        l2::LUA_INDEX[5],
        l2::LUA_INDEX[6],
    ] {
        t.stub(f, false, 2, 0, 0);
    }
    t.stub(l2::LUA_MORE[4], false, 3, 0, 0);
    t.stub(l2::LUA_MORE[8], false, 3, 0, 0);
    t.stub(l2::LUA_MORE[9], false, 2, 0, 0);
}

#[test]
#[allow(clippy::type_complexity)] // one row per case, columns named above
fn left2_index_handlers_match_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        let l = 0x00d0_0000;
        let key = t.cstr("key");
        // (object, function, member, method, n, descriptor, counts,
        //  property, method on the descriptor)
        let cases: [(u32, u32, u32, u32, u32, bool, u32, u32, u32); 22] = [
            (0, 0, 0, 0, 0, false, 0, 0, 0),
            (1, 0, 0, 0, 0, false, 0, 0, 0),
            (2, 0, 0, 0, 0, false, 0, 0, 0),
            (3, 0x31, 0, 0, 0, false, 0, 0, 0),
            (3, 0, 1, 0, 0, false, 0, 0, 0),
            (3, 0, 2, 0, 0, false, 0, 0, 0),
            (3, 0, 0, 0x32, 0, false, 0, 0, 0),
            (3, 0, 0, 0, 0, false, 0, 0, 0),
            (3, 0, 0, 0, 1, false, 0, 0, 0),
            (3, 0, 0, 0, 3, false, 0, 0, 0),
            (3, 0, 0, 0, 0xffff_ffff, false, 2, 0, 0),
            (3, 0, 0, 0, 0xffff_ffff, false, 0, 0, 0),
            (3, 0, 0, 0, 0xffff_ffff, true, 0, 0x33, 0),
            (3, 0, 0, 0, 0xffff_ffff, true, 0, 0, 0x34),
            (3, 0, 0, 0, 0xffff_ffff, true, 0, 0, 0),
            (3, 0, 4, 0, 0, false, 0, 0, 0),
            (3, 0, 8, 0, 0, false, 0, 0, 0),
            (3, 0, 0, 0, 2, false, 0, 0, 0),
            (3, 0, 0, 0, 0xffff_ffff, false, 4, 0, 0),
            (3, 0, 0, 0, 0xffff_ffff, false, 1, 0, 0),
            (3, 0, 0, 0, 0xffff_ffff, true, 0, 0x33, 0),
            (3, 0, 0, 0, 0xffff_ffff, true, 0, 0x33, 0),
        ];
        for (i, &(object, function, member, method, n, descriptor, counts, property, method2)) in
            cases.iter().enumerate()
        {
            let mut words = vec![0u32; 14];
            words[13] = 3;
            let mut rets = vec![0u32; 14];
            rets[13] = n;
            // the class: its vtable on a block large enough for the lists
            // at +0x44..+0x5c (the list counter below reads their heads)
            let vt_owner = t.object(0x7300, &words, &rets);
            let desc = t.zeros(0x100);
            let vt = t.r32(vt_owner);
            t.w32(desc, vt);
            // counts: bit 0 the +0x54 list, bit 1 +0x4c, bit 2 +0x44
            t.w32(desc + 0x54, counts & 1);
            t.w32(desc + 0x4c, (counts >> 1) & 1);
            t.w32(desc + 0x44, (counts >> 2) & 1);
            t.w32(desc + 0x18, 0x0042_0000);
            let ud = t.zeros(8);
            t.w32(ud, desc);
            let obj = t.zeros(0x40);
            t.w32(obj + 0x18, 0x0043_0000);
            // no object (0) only where the paths do not read its name
            t.w32(
                ud + 4,
                if i % 2 == 0 || (descriptor && property == 0) {
                    obj + 0x18
                } else {
                    0
                },
            );
            let m = t.object(0x7400, &[0, 0, 2, 2], &[]);
            // member: 1 readable, 2 write-only (bit 3), 4 read-only for
            // the new index (bit 3), 8 constant (bit 4)
            t.w32(
                m + 0x14,
                match member {
                    2 | 4 => 8,
                    8 => 0x10,
                    _ => 0,
                },
            );
            stub_index_common(&mut t, object, ud, key);
            t.stub(l2::INDEX_CALLS[1], true, 1, 1, function);
            t.stub(l2::FIND_MEMBER, true, 1, 1, if member != 0 { m } else { 0 });
            t.stub(
                l2::INDEX_CALLS[0],
                true,
                1,
                1,
                if method != 0 { method } else { method2 },
            );
            t.stub(
                l2::INDEX_CALLS[3],
                true,
                1,
                1,
                u32::from(descriptor) | 0x100,
            );
            // the list counter answers the word at the list
            let code = t.code(&[0x8b, 0x44, 0x24, 0x04, 0x8b, 0x00, 0xc3]);
            t.place(l2::INDEX_CALLS[4], code);
            let target = t.object(0x7500, &[0, 0, 0, 2, 0, 2], &[]);
            let prop = t.zeros(0x20);
            t.w32(prop + 0x14, target);
            t.w32(prop + 0x18, 0x0044_0000);
            t.w8(prop + 0x10, if i % 3 == 0 { 2 } else { 0 });
            t.stub(
                l2::INDEX_CALLS[2],
                true,
                1,
                1,
                if property != 0 { prop } else { 0 },
            );
            t.stub(l2::INDEX_CALLS[5], true, 1, 1, 0);
            let what = format!("{label}: IndexHandler case {i}");
            t.cdecl(&what, 0x0043_6430, &[l], |g| Some(l2b::index_handler(g, l)));
            // the new index: the class chain reaches the descriptor class
            // or not
            if descriptor {
                let class = t.rebase(l2::CLASS_DESCRIPTOR);
                let mid = t.zeros(0x44);
                t.w32(mid + 0x40, class);
                t.w32(desc + 0x40, mid);
            }
            let what = format!("{label}: NewIndexHandler case {i}");
            t.cdecl(&what, 0x0043_6860, &[l], |g| {
                Some(l2b::new_index_handler(g, l))
            });
            t.unstub();
        }
    }
}

#[test]
fn left2_script_function_names_match_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        let l = 0x00d0_0000;
        let s = t.cstr("update");
        for (err, head, data, entries, string, block, cfunc) in [
            (0u32, 0u32, 0u32, 2u32, true, true, 1u32),
            (5, 0, 0, 0, true, true, 0),
            (0, 3, 0x0045_0000, 2, true, true, 0),
            (0, 0x8000_0003, 0x0045_0000, 1, true, true, 0),
            (0, 3, 0, 1, false, true, 0),
            (0, 0, 0, 3, true, false, 0),
        ] {
            t.stub(l2::LUA_PCALLK, false, 6, 0, err);
            t.stub(l2::LUA_TOLSTRING, false, 3, 0, if string { s } else { 0 });
            t.stub(l2::LUA_SETTOP, false, 2, 0, 0);
            t.stub(l2::PRINTF, false, 4, 0, 0);
            t.stub(l2::NAMES_CALLS[3], false, 2, 0, 0);
            t.stub(l2::NAMES_CALLS[0], false, 3, 0, 0);
            t.stub(l2::LUA_MORE[2], false, 1, 0, 5);
            t.stub(l2::LUA_PUSHNIL, false, 1, 0, 0);
            // lua_next: `entries` pairs, then 0
            let cell = t.zeros(4);
            t.w32(cell, entries);
            let mut c = vec![0x31, 0xc0]; // xor eax, eax
            c.extend([0x83, 0x3d]);
            c.extend(cell.to_le_bytes());
            c.push(0); // cmp dword [cell], 0
            c.extend([0x74, 0x07]); // je +7
            c.extend([0xff, 0x0d]);
            c.extend(cell.to_le_bytes()); // dec dword [cell]
            c.push(0x40); // inc eax
            c.push(0xc3);
            let code = t.code(&c);
            t.place(l2::NAMES_CALLS[1], code);
            // lua_type: alternately (4, 6) pairs and others
            let tcell = t.zeros(4);
            let mut c = vec![0xa1];
            c.extend(tcell.to_le_bytes()); // mov eax, [tcell]
            c.extend([0xff, 0x05]);
            c.extend(tcell.to_le_bytes()); // inc dword [tcell]
            c.extend([0x83, 0xe0, 0x03]); // and eax, 3
            c.extend([0x0f, 0xb6, 0x80]);
            let types = t.alloc(&[4, 6, 4, 5]);
            c.extend(types.to_le_bytes()); // movzx eax, byte [eax + types]
            c.push(0xc3);
            let code = t.code(&c);
            t.place(l2::LUA_TYPE, code);
            t.stub(l2::NAMES_CALLS[2], false, 2, 0, cfunc);
            let b = if block { t.zeros(0x10) } else { 0 };
            t.stub(l2::MALLOC, false, 2, 0, b);
            t.stub(l2::MEMCPY, false, 3, 0, b);
            // the push leaves the array external (bit 31) with 2 names
            let mut pre = vec![0xc7, 0x01];
            pre.extend(0x8000_0002u32.to_le_bytes()); // mov dword [ecx], 0x80000002
            pre.extend(deref_arg(1));
            let s5 = t.stub_code_with(&pre, l2::NAMES_CALLS[4], true, 2, 2, 0);
            t.place(l2::NAMES_CALLS[4], s5);
            t.stub(l2::FREE, false, 1, 0, 0);
            let qsort = t.stub_code(l2::NAMES_CALLS[5], false, 4, 0, 0);
            let slot = t.rebase(l2::NAMES_CALLS[5]);
            t.w32(slot, qsort);
            let script = t.zeros(4);
            t.w32(script, l);
            let names = t.zeros(8);
            t.w32(names, head);
            t.w32(names + 4, data);
            let what = format!(
                "{label}: script function names err {err} head {head:#x} data {data:#x} entries {entries} string {string} block {block}"
            );
            t.cdecl(&what, 0x005d_5860, &[script, names], |g| {
                let s = ffx_graphic::host::Host::scratch(g);
                Some(l2b::script_function_names(g, script, names, s))
            });
            t.unstub();
        }
    }
}

/// A cdecl stub at `sym` answering `table[word & 15]` for its stack word
/// `arg` (not logged).
fn index_stub(t: &mut Twins, sym: u32, arg: u8, table: &[u32; 16]) {
    let bytes: Vec<u8> = table.iter().flat_map(|v| v.to_le_bytes()).collect();
    let tab = t.alloc(&bytes);
    let mut c = vec![0x8b, 0x44, 0x24, 4 + 4 * arg]; // mov eax, [esp+4+4arg]
    c.extend([0x83, 0xe0, 0x0f]); // and eax, 15
    c.extend([0x8b, 0x04, 0x85]);
    c.extend(tab.to_le_bytes()); // mov eax, [eax*4 + tab]
    c.push(0xc3);
    let code = t.code(&c);
    t.place(sym, code);
}

/// A cdecl stub at `sym` answering `values` in turn (then the last one
/// again; not logged).
fn seq_stub(t: &mut Twins, sym: u32, values: &[u32]) {
    let mut v = values.to_vec();
    v.push(*values.last().unwrap());
    let bytes: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    let tab = t.alloc(&bytes);
    let cell = t.zeros(4);
    let last = (values.len()) as u32;
    let mut c = vec![0xa1];
    c.extend(cell.to_le_bytes()); // mov eax, [cell]
    c.push(0x3d);
    c.extend(last.to_le_bytes()); // cmp eax, last
    c.extend([0x74, 0x06]); // je +6
    c.extend([0xff, 0x05]);
    c.extend(cell.to_le_bytes()); // inc dword [cell]
    c.extend([0x8b, 0x04, 0x85]);
    c.extend(tab.to_le_bytes()); // mov eax, [eax*4 + tab]
    c.push(0xc3);
    let code = t.code(&c);
    t.place(sym, code);
}

#[test]
fn left2_copy_values_match_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        let (from, to) = (0x00d0_0001u32, 0x00d0_0002u32);
        // index i: 1 nil, 2 boolean, 3 light userdata, 4 number, 5
        // string, 6 table, 7 function, 8 thread, 9-12 userdata (vector,
        // matrix, object, other), 13 an unknown type
        let mut types = [0u32; 16];
        types[1..14].copy_from_slice(&[0, 1, 2, 3, 4, 5, 6, 8, 7, 7, 7, 7, 9]);
        let ud = t.zeros(8);
        t.w32(ud, 0x0071_0000);
        t.w32(ud + 4, 0x0072_0000);
        for checkstack in [0u32, 1] {
            index_stub(&mut t, l2::LUA_TYPE, 1, &types);
            t.stub(l2::LUA_COPY[0], false, 2, 0, checkstack);
            t.stub(l2::LUA_PUSHNIL, false, 1, 0, 0);
            t.stub(l2::LUA_TOBOOLEAN, false, 2, 0, 1);
            t.stub(l2::LUA_COPY[1], false, 2, 0, 0);
            t.stub(l2::LUA_INDEX[0], false, 2, 0, ud);
            t.stub(l2::LUA_INDEX[4], false, 2, 0, 0);
            t.stub_float(l2::LUA_COPY[2], 3, 0x3fc0_0000);
            t.stub(l2::LUA_COPY[3], false, 2, 0, 0);
            t.stub(l2::LUA_TOLSTRING, false, 3, 0, 0x0073_0000);
            t.stub(l2::LUA_COPY[4], false, 2, 0, 0);
            t.stub(l2::LUA_SETTOP, false, 2, 0, 0);
            t.stub(l2::LUA_COPY[5], false, 2, 0, 0x0074_0000);
            // the level and the message (the unknown-userdata message has
            // no third word; the type name's is lua_typename's answer)
            t.stub(l2::PRINTF, false, 2, 0, 0);
            let mut vector = [0u32; 16];
            vector[9] = 0x0075_0000;
            index_stub(&mut t, l2::COPY_USERDATA[0], 1, &vector);
            let mut matrix = [0u32; 16];
            matrix[10] = 0x0076_0000;
            index_stub(&mut t, l2::COPY_USERDATA[1], 1, &matrix);
            t.stub(l2::COPY_USERDATA[2], false, 2, 0, 0);
            t.stub(l2::COPY_USERDATA[3], false, 2, 0, 0);
            let mut meta = [0u32; 16];
            meta[11] = 1;
            index_stub(&mut t, l2::LUA_INDEX[1], 1, &meta);
            t.stub(l2::LUA_GETFIELD, false, 3, 0, 0);
            t.stub(l2::LUA_INDEX[2], false, 3, 0, 1);
            t.stub(l2::RELOAD_CALLS[2], false, 3, 0, 0);
            for (f, to_, first, n) in [
                (0u32, to, 1u32, 13u32),
                (from, 0, 1, 13),
                (from, from, 1, 13),
                (from, to, 0, 13),
                (from, to, 1, 0),
                (from, to, 1, 13),
                (from, to, 4, 3),
            ] {
                let what = format!(
                    "{label}: CopyValues {f:#x} {to_:#x} {first} {n} checkstack {checkstack}"
                );
                t.cdecl(&what, 0x0061_96e0, &[f, to_, first, n], |g| {
                    Some(l2b::copy_values(g, f, to_, first, n))
                });
            }
            t.unstub();
        }
    }
}

#[test]
fn left2_caller_call_matches_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        let l = 0x00d0_0001u32;
        let thread = 0x00d0_0002u32;
        // (name, pool: 0 none, 1 full, 2 free, loaded, the two pcalls,
        //  the function's type)
        let cases: [(bool, u32, u32, [u32; 2], u32); 9] = [
            (false, 2, 1, [0, 0], 6),
            (true, 0, 1, [0, 0], 6),
            (true, 1, 1, [0, 0], 6),
            (true, 2, 1, [0, 0], 6),
            (true, 2, 0, [0, 0], 6),
            (true, 2, 1, [3, 0], 6),
            (true, 2, 1, [0, 0], 0),
            (true, 2, 1, [0, 4], 6),
            (true, 0, 1, [0, 4], 6),
        ];
        for (i, &(named, pool_kind, loaded, pcalls, ty)) in cases.iter().enumerate() {
            let script = t.zeros(0x40);
            t.w32(script + 0x28, 0x0077_0000);
            t.w32(script + 0x30, if named { 0x0078_0000 } else { 1 });
            let this = t.zeros(8);
            t.w32(this + 4, script);
            let cell = t.rebase(l2::CALLER_POOL[0]);
            let pool = t.zeros(0x40);
            let slots = t.zeros(0x80);
            t.w32(pool + 0x2c, slots);
            t.w32(pool + 0x28, 0x8000_0004);
            t.w32(pool + 0x30, if pool_kind == 1 { 4 } else { 1 });
            t.w32(cell, if pool_kind == 0 { 0 } else { pool });
            // the slot's thread: what the pool counts while it is taken
            // is kept in `seen`
            let seen = t.zeros(4);
            let mut pre = vec![0xa1];
            pre.extend((pool + 0x30).to_le_bytes()); // mov eax, [pool + 0x30]
            pre.push(0xa3);
            pre.extend(seen.to_le_bytes()); // mov [seen], eax
            let s = t.stub_code_with(&pre, l2::CALLER_POOL[1], true, 0, 0, thread);
            t.place(l2::CALLER_POOL[1], s);
            t.stub(l2::REQUIRE_MORE[1], false, 2, 0, loaded);
            if pcalls == [0, 0] {
                t.stub(l2::LUA_PCALLK, false, 6, 0, 0);
            } else {
                seq_stub(&mut t, l2::LUA_PCALLK, &pcalls);
            }
            t.stub(l2::LUA_TOLSTRING, false, 3, 0, 0x0079_0000);
            t.stub(l2::PRINTF, false, 4, 0, 0);
            t.stub(l2::LUA_SETTOP, false, 2, 0, 0);
            seq_stub(&mut t, l2::LUA_MORE[2], &[3, 5, 4 + i as u32 % 3]);
            t.stub(l2::LUA_GETGLOBAL, false, 2, 0, 0);
            t.stub(l2::LUA_TYPE, false, 2, 0, ty);
            t.stub(l2::LUA_MORE[9], false, 2, 0, 0);
            t.stub(l2::COPY_VALUES, false, 4, 0, 0);
            let what = format!("{label}: caller call case {i}");
            t.this(&what, 0x0061_9e90, this, &[l], |g| {
                Some(l2b::caller_call(g, this, l))
            });
            t.unstub();
        }
    }
}

/// A thiscall stub at `sym` popping `pops` words and answering `values`
/// in turn (then the last one again); it logs nothing.
fn seq_stub_this(t: &mut Twins, sym: u32, pops: u16, values: &[u32]) {
    let mut v = values.to_vec();
    v.push(*values.last().unwrap());
    let bytes: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
    let tab = t.alloc(&bytes);
    let cell = t.zeros(4);
    let last = values.len() as u32;
    let mut c = vec![0xa1];
    c.extend(cell.to_le_bytes()); // mov eax, [cell]
    c.push(0x3d);
    c.extend(last.to_le_bytes()); // cmp eax, last
    c.extend([0x74, 0x06]); // je +6
    c.extend([0xff, 0x05]);
    c.extend(cell.to_le_bytes()); // inc dword [cell]
    c.extend([0x8b, 0x04, 0x85]);
    c.extend(tab.to_le_bytes()); // mov eax, [eax*4 + tab]
    c.push(0xc2);
    c.extend((pops * 4).to_le_bytes()); // ret pops*4
    let code = t.code(&c);
    t.place(sym, code);
}

#[test]
#[allow(clippy::type_complexity)] // one row per case, columns named above
fn left2_ring_allocate_matches_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        // (size, write offset or none, full answers, stamps of the four
        //  queued blocks, frame, nothing queued); PhyreSleep moves the
        //  frame counter on by one, as the other threads would
        let cases: [(u32, Option<u32>, &[u32], [u32; 4], u32, bool); 15] = [
            (0x400, Some(0x80), &[0], [0; 4], 10, false),
            (0xf0, Some(0x80), &[0], [0; 4], 10, false),
            (0x10, None, &[0], [0; 4], 10, false),
            (0x10, Some(0x80), &[0], [0; 4], 10, false),
            (0x13, Some(0xe0), &[0], [0; 4], 10, false),
            (0x10, Some(0xe0), &[0], [0; 4], 10, false),
            (0x10, Some(0xd0), &[0], [0; 4], 10, false),
            (0x10, Some(0x80), &[1, 0], [0, 20, 20, 20], 10, false),
            (0x10, Some(0x80), &[1, 0], [20, 0, 0, 0], 10, false),
            (0x10, Some(0x80), &[1, 0], [3, 4, 5, 6], 10, false),
            (0x10, Some(0x80), &[1, 0], [0, 0, 20, 20], 0, false),
            (
                0x10,
                Some(0x80),
                &[1, 1, 1, 1, 0],
                [5, 10, 11, 12],
                10,
                false,
            ),
            (0x10, Some(0x80), &[1, 1, 0], [5, 30, 30, 30], 10, false),
            (0x10, Some(0x80), &[1, 0], [0; 4], 10, true),
            (0x10, Some(0x80), &[1, 1, 1, 0], [5, 11, 40, 40], 10, false),
        ];
        for (i, &(size, write, full, stamps, frame, empty)) in cases.iter().enumerate() {
            let buf = t.junk(0x100, 3);
            let ring = t.zeros(0x30);
            t.w32(ring + 0x18, buf);
            t.w32(ring + 0x1c, buf + 0x100);
            t.w32(ring + 0x20, 0x100);
            let w = write.map_or(0, |o| buf + o);
            t.w32(ring + 0x24, w);
            // four queued blocks from the start, then the write block
            // (a stamp from the future, so a reclaim stops there)
            for (k, &stamp) in stamps.iter().enumerate() {
                let b = buf + 0x20 * k as u32;
                let next = if k == 3 { w } else { b + 0x20 };
                t.w32(b, next);
                t.w32(b + 4, stamp);
            }
            if w != 0 {
                t.w32(w, buf);
                t.w32(w + 4, 0);
            }
            t.w32(ring + 0x28, if empty { w } else { buf });
            let fc = t.rebase(l2::RING_FRAME);
            t.w32(fc, frame);
            t.stub(l2::RING_CALLS[0], true, 0, 0, 0);
            t.stub(l2::RING_CALLS[1], true, 0, 0, 0);
            seq_stub_this(&mut t, l2::RING_CALLS[2], 1, full);
            let mut pre = vec![0xff, 0x05];
            pre.extend(fc.to_le_bytes()); // inc dword [frame]
            let sleep = t.stub_code_with(&pre, l2::RING_CALLS[3], false, 1, 0, 0);
            t.place(l2::RING_CALLS[3], sleep);
            t.stub(l2::PRINTF, false, 3, 0, 0);
            let what = format!("{label}: ring allocate case {i}");
            t.this(&what, 0x005c_e6e0, ring, &[size], |g| {
                Some(l2b::ring_allocate(g, ring, size))
            });
            t.unstub();
        }
    }
}

#[test]
fn left2_set_clip_matches_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        for (k, (set_target, range_err)) in [(0u32, 0u32), (0x11, 0), (0, 7), (0, 0)]
            .into_iter()
            .enumerate()
        {
            // the time controller: vt[4] answers its speed plus its offset
            // (the controller's +0xc and +0x10: the object's +0x30, +0x34)
            let c = vec![0xd9, 0x41, 0x0c, 0xd8, 0x41, 0x10, 0xc3]; // fld [ecx+0xc]; fadd [ecx+0x10]; ret
            let off = t.rebase(l2::CLIP_DATA[0]);
            t.w32(off, 0.375f32.to_bits());
            let f = t.code(&c);
            let vt = t.alloc(&[[0u8; 16].as_slice(), &f.to_le_bytes()].concat());
            let this = t.junk(0x60, k as u8);
            t.w32(this + 0x24, vt);
            t.w32(this + 8, 1.25f32.to_bits());
            t.stub(l2::CLIP_DATA[1], true, 1, 1, set_target);
            // the time range: start 1.5, end 4.25 through the pointers
            let mut c = vec![0x8b, 0x44, 0x24, 0x04, 0xc7, 0x00];
            c.extend(1.5f32.to_bits().to_le_bytes());
            c.extend([0x8b, 0x44, 0x24, 0x08, 0xc7, 0x00]);
            c.extend(4.25f32.to_bits().to_le_bytes());
            c.push(0xb8);
            c.extend(range_err.to_le_bytes());
            c.extend([0xc2, 0x08, 0x00]);
            let code = t.code(&c);
            t.place(l2::CLIP_DATA[2], code);
            t.stub(l2::CLIP_DATA[3], true, 1, 1, 0x33);
            let clip = 0x0077_0000 + k as u32;
            let what = format!("{label}: set clip case {k}");
            t.this(&what, 0x0054_3150, this, &[clip], |g| {
                let range = ffx_graphic::host::Host::scratch(g);
                Some(l2b::set_clip(g, this, clip, range))
            });
            t.unstub();
        }
    }
}

#[test]
fn left2_matrix_new_matches_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        let l = 0x00d0_0000;
        let mut vals = Vec::new();
        for i in 0..20u32 {
            vals.extend(
                (0.25f32 * f32::from(i as u16) - 1.0)
                    .to_bits()
                    .to_le_bytes(),
            );
        }
        let table = t.alloc(&vals);
        for (top, ud) in [(17u32, true), (2, true), (1, true), (5, true), (17, false)] {
            t.stub(l2::LUA_MORE[2], false, 1, 0, top);
            let block = if ud { t.junk(0x40, 3) } else { 0 };
            t.stub(l2::LUA_MORE[6], false, 2, 0, block);
            t.stub(l2::LUA_GETFIELD, false, 3, 0, 0);
            t.stub(l2::LUA_MORE[7], false, 2, 0, 0);
            t.stub_float_table(l2::MATRIX_NEW[0], 2, 1, table);
            let what = format!("{label}: Matrix new top {top} ud {ud}");
            t.cdecl(&what, 0x0057_0e30, &[l], |g| {
                let frame = ffx_graphic::host::Host::scratch(g);
                Some(l2b::matrix_new(g, l, frame))
            });
            t.unstub();
        }
    }
}

#[test]
fn left2_matrix_look_at_matches_the_original() {
    use ffx_pother_a::m5p_scripting_left2::{self as l2, bridge as l2b};
    for (label, mut t) in Twins::all() {
        let l = 0x00d0_0000;
        let vecs: Vec<u32> = [
            [0.0f32, 1.0, 5.0, 9.0],
            [0.5, 0.25, -2.0, 9.0],
            [0.0, 1.0, 0.0, 9.0],
        ]
        .iter()
        .map(|v| {
            t.alloc(
                &v.iter()
                    .flat_map(|x| x.to_bits().to_le_bytes())
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
        // which arguments are vectors (touserdata), whether the metatables
        // match (rawequal)
        for (present, same) in [
            ([true; 3], 1u32),
            ([true, false, true], 1),
            ([true; 3], 0),
            ([false, true, true], 1),
            ([true, true, false], 1),
        ] {
            let mut table = [0u32; 16];
            for i in 0..3 {
                if present[i] {
                    table[i + 1] = vecs[i];
                }
            }
            let bytes: Vec<u8> = table.iter().flat_map(|v| v.to_le_bytes()).collect();
            let tab = t.alloc(&bytes);
            let mut c = vec![0x8b, 0x44, 0x24, 0x08, 0x83, 0xe0, 0x0f, 0x8b, 0x04, 0x85]; // mov eax, [esp+8]; and eax, 15; mov eax, [eax*4+tab]
            c.extend(tab.to_le_bytes());
            c.push(0xc3);
            let code = t.code(&c);
            t.place(l2::LUA_INDEX[0], code);
            t.stub(l2::LUA_INDEX[1], false, 2, 0, 1);
            t.stub(l2::LUA_GETFIELD, false, 3, 0, 0);
            t.stub(l2::LUA_INDEX[2], false, 3, 0, same);
            t.stub(l2::LUA_SETTOP, false, 2, 0, 0);
            let ud = t.junk(0x40, 5);
            t.stub(l2::LUA_MORE[6], false, 2, 0, ud);
            t.stub(l2::LUA_MORE[7], false, 2, 0, 0);
            let what = format!("{label}: Matrix.LookAt {present:?} same {same}");
            t.cdecl(&what, 0x0057_1200, &[l], |g| {
                let frame = ffx_graphic::host::Host::scratch(g);
                Some(l2b::matrix_look_at(g, l, frame))
            });
            t.unstub();
        }
    }
}
