//! M5 stage 1, list `tmpl-concrete` (`ffx_pload::m5p_tmpl_concrete`):
//! every row of every table runs twice from the same memory, as the
//! original copy (`FFX.exe` at its address, thiscall on a descriptor) and
//! as the generic port with the row's constants; `eax` (`al` for the
//! `bool` slots) and every byte written must agree, and the oracle checks
//! `esp`, so each copy pops what its shim declares. On the image alone and
//! on the snapshot (`FFX_SNAPSHOT`). Needs `FFX_GAME_DIR`.
//!
//! None of this code ran in a coverage run, so the inputs are synthetic:
//! the callees are recording stubs at their addresses in both oracles
//! (they log their tag, `ecx` and their stack words, so the diff checks
//! every call and its arguments), and every branch gets its own data:
//! flags bit 0 set and clear, `p` 0 and not 0, a default instance 0, 4 and
//! real, a name 0 and not 0, and for the named-semantic `PArray` fill: an
//! empty, a one- and a three-entry list against an array of the same and
//! of another count, owned and external, with `PhyreMalloc` failing, and
//! the old block equal to the new one. The named-semantic lookups also
//! run for real (no stubs) on the snapshot.
#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

mod m5p_tmpl_twins;

use ffx_bridge::Mem;
use ffx_pload::m5p_tmpl_abstract_ab as ab;
use ffx_pload::m5p_tmpl_concrete as t;
use m5p_tmpl_twins::{JUNK, Twins};

#[test]
fn m5p_tmpl_concrete_dtors() {
    for (what, mut tw) in Twins::all() {
        tw.stub(ab::DESTRUCT, true, 0, 0, 0x0bad_0001);
        tw.stub(t::FREE, false, 1, 0, 0x0bad_0002);
        tw.stub(0x005a_0270, true, 0, 0, 0x0bad_0003);
        let obj = tw.zeros(0xa0);
        for flags in [0, 1, 2, 3, 0x100, 0xffff_ff00, 0xffff_ffff] {
            for &(f, vt) in t::DTORS {
                tw.write(obj, &[0x55; 0xa0]);
                let label = format!("{what} dtor {f:#x} flags {flags:#x}");
                let r = tw.run(&label, f, obj, &[flags], |g| {
                    t::bridge::scalar_deleting_dtor(g, obj, flags, vt)
                });
                assert_eq!(r, obj, "{label}: returns this");
                assert_eq!(tw.logged(), 1 + (flags & 1), "{label}: calls");
                assert_eq!((tw.entry(0, 0), tw.entry(0, 1)), (ab::DESTRUCT, obj));
                if flags & 1 != 0 {
                    assert_eq!((tw.entry(1, 0), tw.entry(1, 2)), (t::FREE, obj));
                }
                let stored: u32 = Mem::read(&tw.b, obj);
                assert_eq!(stored, tw.rebase(vt), "{label}: vtable");
            }
            for &(f, destruct) in t::NAMED_DTORS {
                tw.write(obj, &[0x55; 0xa0]);
                let label = format!("{what} named dtor {f:#x} flags {flags:#x}");
                let r = tw.run(&label, f, obj, &[flags], |g| {
                    t::bridge::dtor_without_vtable(g, obj, flags, destruct)
                });
                assert_eq!(r, obj, "{label}: returns this");
                assert_eq!(tw.logged(), 1 + (flags & 1), "{label}: calls");
                assert_eq!((tw.entry(0, 0), tw.entry(0, 1)), (destruct, obj));
                if flags & 1 != 0 {
                    assert_eq!((tw.entry(1, 0), tw.entry(1, 2)), (t::FREE, obj));
                }
            }
        }
        tw.unstub();
    }
}

#[test]
fn m5p_tmpl_concrete_wrappers() {
    for (what, mut tw) in Twins::all() {
        tw.stub(t::REGISTER_TYPE, false, 1, 0, 0x0bad_0004);
        for this in [0, 0x0012_3450, u32::MAX] {
            for &f in t::WRAPPER_BIND {
                let label = format!("{what} wrapper bind {f:#x} this {this:#x}");
                let r = tw.run(&label, f, this, &[], |g| t::bridge::register_self(g, this));
                assert_eq!(r, 0x0bad_0004, "{label}");
                assert_eq!(tw.logged(), 1, "{label}: calls");
                assert_eq!((tw.entry(0, 0), tw.entry(0, 2)), (t::REGISTER_TYPE, this));
            }
            for &f in t::RETURN_TRUE {
                for p in [0, JUNK[0]] {
                    let label = format!("{what} constructInstance {f:#x} p {p:#x}");
                    let r = tw.run_al(
                        &label,
                        f,
                        this,
                        &[p],
                        |_| u8::from(t::bridge::constructed()),
                    );
                    assert_eq!(r, 1, "{label}");
                    assert_eq!(tw.logged(), 0, "{label}: calls");
                }
            }
            for args in [[0, 0], [JUNK[0], JUNK[1]]] {
                let label = format!("{what} PType slot 0 {args:x?}");
                let r = tw.run_al(&label, t::TYPE_SLOT0, this, &args, |_| {
                    u8::from(t::bridge::not_matched())
                });
                assert_eq!(r, 0, "{label}");
            }
        }
        tw.unstub();
    }
}

#[test]
fn m5p_tmpl_concrete_script_slots() {
    for (what, mut tw) in Twins::all() {
        tw.stub(t::PRINTF, false, 3, 0, 0x0bad_0005);
        let desc = tw.zeros(0x40);
        let name = tw.cstr("PTypeName");
        for named in [0, name] {
            tw.w32(desc + 0x18, named);
            for &(f, message) in t::SCRIPT_POINTER {
                let label = format!("{what} script pointer {f:#x} name {named:#x}");
                let r = tw.run(&label, f, desc, &JUNK[..2], |g| {
                    t::bridge::script_pointer_unsupported(g, desc, message)
                });
                assert_eq!(r, 0x0bad_0005, "{label}");
                assert_eq!(tw.logged(), 1, "{label}: calls");
                let shown = if named == 0 {
                    tw.rebase(t::UNNAMED)
                } else {
                    named
                };
                assert_eq!(
                    (tw.entry(0, 2), tw.entry(0, 3), tw.entry(0, 4)),
                    (2, tw.rebase(message), shown),
                    "{label}"
                );
            }
        }
        // PClassDescriptorConcrete<PEntity> slot 3: separate buffers, then
        // the output overlapping the source one word on (the copy order
        // decides what lands)
        let src = tw.zeros(0x40);
        let out = tw.zeros(0x40);
        let words: Vec<u8> = (0u8..0x40).map(|i| i.wrapping_mul(7) ^ 0x5a).collect();
        for (s, o) in [(src, out), (src, src + 4), (src + 4, src), (src, src)] {
            tw.write(src, &words);
            tw.write(out, &[0x33; 0x40]);
            tw.stub(t::ENTITY_GET, false, 1, 0, s);
            let label = format!("{what} entity getValue src {s:#x} out {o:#x}");
            tw.run(&label, t::ENTITY_GET_VALUE, desc, &[0x0bad_beef, o], |g| {
                t::bridge::entity_get_value(g, 0x0bad_beef, o)
            });
            assert_eq!(
                (tw.entry(0, 0), tw.entry(0, 2)),
                (t::ENTITY_GET, 0x0bad_beef)
            );
        }
        tw.unstub();
    }
}

#[test]
fn m5p_tmpl_concrete_named_lookups() {
    for (what, mut tw) in Twins::all() {
        let entry = tw.zeros(0x10);
        tw.w32(entry + 4, 0x0abc_def0);
        for e in [0, entry] {
            for &f in t::NAMED_NAME {
                let label = format!("{what} semantic name {f:#x} entry {e:#x}");
                let r = tw.run(&label, f, 0x0012_3450, &[e], |g| {
                    t::bridge::semantic_name(g, e)
                });
                assert_eq!(r, if e == 0 { 0 } else { 0x0abc_def0 }, "{label}");
            }
        }
        // the lookups for real where the lists exist, then stubbed
        let name = tw.cstr("Directional");
        if tw.snapshot {
            for &(f, find) in t::NAMED_FIND {
                for n in [0, name] {
                    let label = format!("{what} semantic find {f:#x} name {n:#x} (real)");
                    tw.run(&label, f, 0x0012_3450, &[n], |g| {
                        t::bridge::semantic_find(g, find, n)
                    });
                }
            }
        }
        for &(f, find) in t::NAMED_FIND {
            tw.stub(find, false, 1, 0, find ^ 0x5a5a_0000);
            for n in [0, name, JUNK[0]] {
                let label = format!("{what} semantic find {f:#x} name {n:#x}");
                let r = tw.run(&label, f, 0x0012_3450, &[n], |g| {
                    t::bridge::semantic_find(g, find, n)
                });
                if n == 0 {
                    assert_eq!((r, tw.logged()), (0, 0), "{label}");
                } else {
                    assert_eq!(r, find ^ 0x5a5a_0000, "{label}");
                    assert_eq!((tw.entry(0, 0), tw.entry(0, 2)), (find, n), "{label}");
                }
            }
        }
        tw.unstub();
    }
}

/// The named-semantic `PArray` fill (`NAMED_VALUES`).
#[test]
fn m5p_tmpl_concrete_named_values() {
    for (what, mut tw) in Twins::all() {
        let entries = tw.zeros(0x40);
        let array = tw.zeros(0x10);
        let block = tw.zeros(0x40);
        let old = tw.zeros(0x40);
        tw.stub(t::FREE, false, 1, 0, 0x0bad_0006);
        for &(f, head) in t::NAMED_VALUES {
            let head_at = tw.rebase(head);
            let saved: u32 = Mem::read(&tw.a, head_at);
            for n in [0u32, 1, 3] {
                // a chain of n entries 0x10 apart (link at +8)
                tw.write(entries, &[0x44; 0x40]);
                for i in 0..n {
                    let next = if i + 1 < n {
                        entries + 0x10 * (i + 1)
                    } else {
                        0
                    };
                    tw.w32(entries + 0x10 * i + 8, next);
                }
                tw.w32(head_at, if n == 0 { 0 } else { entries });
                for count in [0, 1, 3, 0x8000_0001, 0x8000_0003] {
                    for (malloc_ret, data) in [(block, old), (0, old), (block, block), (block, 0)] {
                        if count & 0x7fff_ffff == n && (malloc_ret, data) != (block, old) {
                            continue;
                        }
                        tw.stub(t::MALLOC, false, 2, 0, malloc_ret);
                        tw.write(block, &[0x66; 0x40]);
                        tw.write(old, &[0x77; 0x40]);
                        tw.w32(array, count);
                        tw.w32(array + 4, data);
                        let label = format!(
                            "{what} semantic values {f:#x} n {n} count {count:#x} malloc {malloc_ret:#x} data {data:#x}"
                        );
                        let r = tw.run(&label, f, 0x0012_3450, &[array], |g| {
                            t::bridge::semantic_values(g, head, array)
                        });
                        let failed = n != count && n != 0 && malloc_ret == 0;
                        assert_eq!(r, if failed { 0xd } else { 0 }, "{label}");
                    }
                }
            }
            tw.w32(head_at, saved);
        }
        tw.unstub();
    }
}

#[test]
fn m5p_tmpl_concrete_construct() {
    for (what, mut tw) in Twins::all() {
        let obj = tw.zeros(0x100);
        for &(f, (ctor, stores)) in t::CONSTRUCT {
            tw.stub(ctor, true, 0, 0, ctor ^ 0x5a5a_0000);
            for p in [0, obj] {
                tw.write(obj, &[0x55; 0x100]);
                let label = format!("{what} constructInstance {f:#x} p {p:#x}");
                let r = tw.run_al(&label, f, 0x0012_3450, &[p], |g| {
                    u8::from(t::bridge::construct_with(g, p, ctor, stores))
                });
                assert_eq!(r, 1, "{label}");
                assert_eq!(tw.logged(), u32::from(p != 0), "{label}: calls");
                if p != 0 {
                    assert_eq!((tw.entry(0, 0), tw.entry(0, 1)), (ctor, p), "{label}");
                }
            }
        }
        tw.unstub();
        // without a default constructor: no default instance, 4, a real one
        for def in [0, 4, 0x0123_4560] {
            tw.stub(t::GET_DEFAULT, true, 0, 0, def);
            for &(f, (desc, ctor)) in t::FROM_DEFAULT {
                tw.stub(ctor, true, 1, 1, 0x0bad_0007);
                for p in [0, obj] {
                    let label =
                        format!("{what} constructInstance {f:#x} default {def:#x} p {p:#x}");
                    let r = tw.run_al(&label, f, 0x0012_3450, &[p], |g| {
                        u8::from(t::bridge::construct_from_default(g, p, desc, ctor))
                    });
                    assert_eq!(r, u8::from(def != 0 && def != 4), "{label}");
                    assert_eq!(
                        (tw.entry(0, 0), tw.entry(0, 1)),
                        (t::GET_DEFAULT, tw.rebase(desc)),
                        "{label}"
                    );
                    if r != 0 && p != 0 {
                        assert_eq!(
                            (tw.entry(1, 0), tw.entry(1, 1), tw.entry(1, 2)),
                            (ctor, p, def - 4),
                            "{label}"
                        );
                    }
                }
            }
            for &(f, desc) in t::DEFAULT_PAIR {
                for p in [0, obj] {
                    tw.write(obj, &[0x55; 0x100]);
                    let label =
                        format!("{what} constructInstance {f:#x} default {def:#x} p {p:#x}");
                    let r = tw.run_al(&label, f, 0x0012_3450, &[p], |g| {
                        u8::from(t::bridge::construct_default_pair(g, p, desc))
                    });
                    assert_eq!(r, u8::from(def != 0), "{label}");
                    assert_eq!(tw.entry(0, 1), tw.rebase(desc), "{label}");
                }
            }
        }
        tw.unstub();
    }
}

/// Every address of the list is in exactly one table (or one of the
/// single ports), so no copy goes untested.
#[test]
fn m5p_tmpl_concrete_tables_cover_the_shims() {
    let mut all: Vec<u32> = Vec::new();
    all.extend(t::DTORS.iter().map(|r| r.0));
    all.extend(t::NAMED_DTORS.iter().map(|r| r.0));
    all.extend(t::WRAPPER_BIND);
    all.extend(t::RETURN_TRUE);
    all.extend(t::SCRIPT_POINTER.iter().map(|r| r.0));
    all.extend(t::NAMED_NAME);
    all.extend(t::NAMED_FIND.iter().map(|r| r.0));
    all.extend(t::NAMED_VALUES.iter().map(|r| r.0));
    all.extend(t::CONSTRUCT.iter().map(|r| r.0));
    all.extend(t::FROM_DEFAULT.iter().map(|r| r.0));
    all.extend(t::DEFAULT_PAIR.iter().map(|r| r.0));
    all.extend([t::TYPE_SLOT0, t::ENTITY_GET_VALUE]);
    let n = all.len();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), n, "an address in two tables");
    assert_eq!(n, 330);
}
