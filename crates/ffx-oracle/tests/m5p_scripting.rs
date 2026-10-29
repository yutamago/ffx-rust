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

use ffx_pother_a::m5p_scripting_small::{self as s, bridge as b};
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
