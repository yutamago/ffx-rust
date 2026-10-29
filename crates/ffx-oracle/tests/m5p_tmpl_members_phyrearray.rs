//! M5 stage 1, lists `tmpl-members-phyrearray-a` and `-b`
//! (`ffx_pload::m5p_tmpl_members_phyrearray`): every copy runs twice from
//! the same memory, as the original (`FFX.exe` at its address) and as the
//! port with its table row; `eax` and every byte written must agree, and
//! the oracle checks `esp`. On the image alone and on the snapshot
//! (`FFX_SNAPSHOT`). Needs `FFX_GAME_DIR`.
//!
//! None of this ran in a coverage run, so the inputs are synthetic: a
//! member descriptor (`+0xc` its class with the base `+0x88`, `+0x24` the
//! member's offset) over an object holding the array, at several offsets
//! and bases; array headers with counts 0, 1, 2 and more, owned and
//! external; data words 0, the new buffer and another block; negative
//! counts. Callees are recording stubs at their addresses. The element
//! type getters also run for real on the snapshot.
#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

mod m5p_tmpl_twins;

use ffx_pload::m5p_tmpl_members_phyrearray as t;
use m5p_tmpl_twins::{JUNK, Twins};

/// A member descriptor, its class and an object: `(member, object,
/// array)` for each layout the tests use.
fn layouts(tw: &mut Twins) -> Vec<(u32, u32, u32)> {
    let mut out = Vec::new();
    for (off, base) in [(0x10u32, 0u32), (0x2c, 0x8), (0x48, 0x48)] {
        let member = tw.zeros(0x40);
        let class = tw.zeros(0x100);
        let object = tw.zeros(0x80);
        tw.w32(member + 0xc, class);
        tw.w32(member + 0x24, off);
        tw.w32(class + 0x88, base);
        out.push((member, object, object + off - base));
    }
    out
}

const HEADERS: [u32; 7] = [0, 1, 2, 7, 0x8000_0000, 0x8000_0001, 0x8000_0005];

#[test]
fn m5p_tmpl_phyrearray_getters() {
    for (what, mut tw) in Twins::all() {
        let lay = layouts(&mut tw);
        for &(member, object, array) in &lay {
            for count in HEADERS {
                tw.w32(array, count);
                tw.w32(array + 4, 0x00ab_cd00 ^ count);
                for &f in t::GET_START {
                    let label = format!("{what} getArrayStart {f:#x} count {count:#x}");
                    let r = tw.run(&label, f, member, &[object], |g| {
                        t::bridge::array_start(g, member, object)
                    });
                    assert_eq!(r, 0x00ab_cd00 ^ count, "{label}");
                }
                for &f in t::GET_COUNT {
                    let label = format!("{what} getCount {f:#x} count {count:#x}");
                    let r = tw.run(&label, f, member, &[object], |g| {
                        t::bridge::array_count(g, member, object)
                    });
                    assert_eq!(r, count & 0x7fff_ffff, "{label}");
                }
                for &f in t::SHARRAY_START {
                    let label = format!("{what} sharray getArrayStart {f:#x} count {count:#x}");
                    tw.run(&label, f, member, &[object], |g| {
                        t::bridge::sharray_start(g, member, object)
                    });
                }
            }
        }
    }
}

#[test]
fn m5p_tmpl_phyrearray_calls() {
    for (what, mut tw) in Twins::all() {
        let lay = layouts(&mut tw);
        tw.stub(t::MEMBER_ARRAY_DESTRUCT, true, 0, 0, 0x0bad_0001);
        tw.stub(t::FREE, false, 1, 0, 0x0bad_0002);
        let (member, object, array) = lay[1];
        for flags in [0, 1, 2, 0xffff_ffff] {
            for &(f, destruct) in t::DTORS {
                let label = format!("{what} dtor {f:#x} flags {flags:#x}");
                let r = tw.run(&label, f, member, &[flags], |g| {
                    t::bridge::dtor_without_vtable(g, member, flags, destruct)
                });
                assert_eq!(r, member, "{label}");
                assert_eq!(tw.logged(), 1 + (flags & 1), "{label}: calls");
            }
        }
        for &(_, resize) in t::SET_COUNT {
            if !tw.saved.contains_key(&tw.rebase(resize)) {
                tw.stub(resize, true, 1, 1, resize ^ 0x5a5a_0000);
            }
        }
        for count in [0, 3, 0xffff_ffff] {
            for &(f, resize) in t::SET_COUNT {
                let label = format!("{what} setCount {f:#x} count {count:#x}");
                let r = tw.run(&label, f, member, &[object, count], |g| {
                    t::bridge::array_set_count(g, member, object, count, resize)
                });
                assert_eq!(r, resize ^ 0x5a5a_0000, "{label}");
                assert_eq!(
                    (tw.entry(0, 0), tw.entry(0, 1), tw.entry(0, 2)),
                    (resize, array, count),
                    "{label}"
                );
            }
        }
        for &(_, f) in t::SET_EXTERNAL_FORWARD.iter().chain(t::SET_EXTERNAL_MARKED) {
            if !tw.saved.contains_key(&tw.rebase(f)) {
                tw.stub(f, true, 2, 2, f ^ 0x5a5a_0000);
            }
        }
        for count in [0, 3, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
            for &(f, setter) in t::SET_EXTERNAL_FORWARD {
                let label = format!("{what} setToExternalBuffer forward {f:#x} count {count:#x}");
                let r = tw.run(&label, f, member, &[object, JUNK[0], count], |g| {
                    t::bridge::set_external_forward(g, member, object, JUNK[0], count, setter)
                });
                assert_eq!(r, setter ^ 0x5a5a_0000, "{label}");
                assert_eq!(
                    (tw.entry(0, 1), tw.entry(0, 2), tw.entry(0, 3)),
                    (array, JUNK[0], count),
                    "{label}"
                );
            }
            for &(f, setter) in t::SET_EXTERNAL_MARKED {
                let label = format!("{what} setToExternalBuffer marked {f:#x} count {count:#x}");
                let r = tw.run(&label, f, member, &[object, JUNK[0], count], |g| {
                    t::bridge::set_external_marked(g, member, object, JUNK[0], count, setter)
                });
                if count >> 31 != 0 {
                    assert_eq!((r, tw.logged()), (5, 0), "{label}");
                } else {
                    assert_eq!(r, 0, "{label}");
                    assert_eq!(
                        (tw.entry(0, 1), tw.entry(0, 2), tw.entry(0, 3)),
                        (array, JUNK[0], count | 0x8000_0000),
                        "{label}"
                    );
                }
            }
        }
        for &(f, d) in t::ELEMENT_TYPE {
            let label = format!("{what} element type {f:#x}");
            let r = tw.run(&label, f, member, &[], |g| t::bridge::element_type(g, d));
            assert_eq!(r, tw.rebase(d), "{label}");
        }
        if tw.snapshot {
            for &(f, target) in t::ELEMENT_TYPE_JUMP {
                let label = format!("{what} element type {f:#x} (real)");
                tw.run(&label, f, member, &[], |g| {
                    t::bridge::element_type_jump(g, target)
                });
            }
        }
        for &(_, target) in t::ELEMENT_TYPE_JUMP {
            if !tw.saved.contains_key(&tw.rebase(target)) {
                tw.stub(target, false, 0, 0, target ^ 0x5a5a_0000);
            }
        }
        for &(f, target) in t::ELEMENT_TYPE_JUMP {
            let label = format!("{what} element type {f:#x}");
            let r = tw.run(&label, f, member, &[], |g| {
                t::bridge::element_type_jump(g, target)
            });
            assert_eq!(r, target ^ 0x5a5a_0000, "{label}");
        }
        tw.unstub();
    }
}

/// `setToExternalBuffer` with its own release: every count, data and
/// ownership case.
#[test]
fn m5p_tmpl_phyrearray_set_external() {
    for (what, mut tw) in Twins::all() {
        let lay = layouts(&mut tw);
        tw.stub(t::FREE, false, 1, 0, 0x0bad_0002);
        for &(_, how) in t::SET_EXTERNAL {
            if let t::Release::Elements(f) = how
                && !tw.saved.contains_key(&tw.rebase(f))
            {
                tw.stub(f, false, 2, 0, 0x0bad_0003);
            }
        }
        let buffer = tw.zeros(0x20);
        let other = tw.zeros(0x20);
        for &(member, object, array) in &lay[..2] {
            for header in [3, 0x8000_0003, 0, 0x7fff_ffff] {
                for data in [0, buffer, other] {
                    for count in [0, 5, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
                        for &(f, how) in t::SET_EXTERNAL {
                            tw.w32(array, header);
                            tw.w32(array + 4, data);
                            let label = format!(
                                "{what} setToExternalBuffer {f:#x} header {header:#x} data {data:#x} count {count:#x}"
                            );
                            let r = tw.run(&label, f, member, &[object, buffer, count], |g| {
                                t::bridge::set_external(g, member, object, buffer, count, how)
                            });
                            assert_eq!(r, if count >> 31 != 0 { 5 } else { 0 }, "{label}");
                        }
                    }
                }
            }
        }
        tw.unstub();
    }
}

/// Every address of the two lists is in exactly one table.
#[test]
fn m5p_tmpl_phyrearray_tables_cover_the_shims() {
    let mut all: Vec<u32> = Vec::new();
    all.extend(t::DTORS.iter().map(|r| r.0));
    all.extend(t::GET_COUNT);
    all.extend(t::SET_COUNT.iter().map(|r| r.0));
    all.extend(t::GET_START);
    all.extend(t::SHARRAY_START);
    all.extend(t::SET_EXTERNAL.iter().map(|r| r.0));
    all.extend(t::SET_EXTERNAL_FORWARD.iter().map(|r| r.0));
    all.extend(t::SET_EXTERNAL_MARKED.iter().map(|r| r.0));
    all.extend(t::ELEMENT_TYPE.iter().map(|r| r.0));
    all.extend(t::ELEMENT_TYPE_JUMP.iter().map(|r| r.0));
    let n = all.len();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), n, "an address in two tables");
    assert_eq!(n, 405);
}
