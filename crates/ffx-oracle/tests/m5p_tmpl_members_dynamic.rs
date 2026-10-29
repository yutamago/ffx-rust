//! M5 stage 1, list `tmpl-members-dynamic`
//! (`ffx_pload::m5p_tmpl_members_dynamic`): every copy runs twice from
//! the same memory, as the original (`FFX.exe` at its address, thiscall on
//! a member descriptor) and as the generic port; `eax` and every byte
//! written must agree, and the oracle checks `esp`. On the image alone and
//! on the snapshot (`FFX_SNAPSHOT`). Needs `FFX_GAME_DIR`.
//!
//! None of this ran in a coverage run, so the inputs are synthetic: the
//! member's accessors (`+0x34`, `+0x38`, `+0x3c`) are recording stubs or
//! 0 (both branches of every copy), the fallback reads the member's word
//! out of an object at several offsets, and the `PClassDataMemberArray`
//! slots run on a member whose vtable holds recording stubs, with slot
//! 10's `al` set, clear and clear with other bits of `eax` set.
#![allow(clippy::cast_possible_truncation, clippy::too_many_lines)]

mod m5p_tmpl_twins;

use ffx_bridge::Mem;
use ffx_pload::m5p_tmpl_members_dynamic as t;
use m5p_tmpl_twins::{JUNK, Twins};

#[test]
fn m5p_tmpl_dynamic_dtors() {
    for (what, mut tw) in Twins::all() {
        tw.stub(t::MEMBER_ARRAY_DESTRUCT, true, 0, 0, 0x0bad_0001);
        tw.stub(0x0042_fb20, false, 1, 0, 0x0bad_0002);
        let obj = tw.zeros(0x60);
        for flags in [0, 1, 2, 3, 0xffff_fffe, 0xffff_ffff] {
            for &(f, destruct) in t::DTORS {
                let label = format!("{what} dtor {f:#x} flags {flags:#x}");
                let r = tw.run(&label, f, obj, &[flags], |g| {
                    t::bridge::dtor_without_vtable(g, obj, flags, destruct)
                });
                assert_eq!(r, obj, "{label}");
                assert_eq!(tw.logged(), 1 + (flags & 1), "{label}: calls");
                assert_eq!((tw.entry(0, 0), tw.entry(0, 1)), (destruct, obj));
                if flags & 1 != 0 {
                    assert_eq!(tw.entry(1, 2), obj, "{label}");
                }
            }
        }
        tw.unstub();
    }
}

#[test]
fn m5p_tmpl_dynamic_accessors() {
    for (what, mut tw) in Twins::all() {
        let member = tw.zeros(0x60);
        let class = tw.zeros(0x100);
        let object = tw.zeros(0x100);
        let words: Vec<u8> = (0u8..=0xff).map(|i| i ^ 0xa5).collect();
        tw.write(object, &words);
        let getter = tw.stub_code(0x0000_6e70, true, 0, 0, 0x0ace_0001);
        let counter = tw.stub_code(0x0000_6e74, true, 0, 0, 0x0ace_0003);
        let setter = tw.stub_code(0x0000_5e70, true, 2, 2, 0x0ace_0002);
        for (with, label_with) in [(false, "none"), (true, "set")] {
            tw.w32(member + 0x34, if with { counter } else { 0 });
            tw.w32(member + 0x38, if with { getter } else { 0 });
            tw.w32(member + 0x3c, if with { setter } else { 0 });
            tw.w32(member + 0xc, class);
            for (off, base) in [(0x10, 0), (0x24, 0x8), (0x40, 0x40), (0x80, 0x10)] {
                tw.w32(member + 0x24, off);
                tw.w32(class + 0x88, base);
                for &f in t::GET_START {
                    let label = format!("{what} getArrayStart {f:#x} {label_with} off {off:#x}");
                    let r = tw.run(&label, f, member, &[object], |g| {
                        t::bridge::member_start(g, member, object)
                    });
                    if with {
                        assert_eq!(r, 0x0ace_0001, "{label}");
                        assert_eq!((tw.entry(0, 0), tw.entry(0, 1)), (0x6e70, object));
                    } else {
                        let at = object + off - base;
                        let want: u32 = Mem::read(&tw.b, at);
                        assert_eq!(r, want, "{label}");
                    }
                }
            }
            for &f in t::GET_COUNT {
                let label = format!("{what} getCount {f:#x} {label_with}");
                let r = tw.run(&label, f, member, &[object], |g| {
                    t::bridge::member_count(g, member, object)
                });
                assert_eq!(r, if with { 0x0ace_0003 } else { 0 }, "{label}");
                assert_eq!(tw.logged(), u32::from(with), "{label}: calls");
            }
            for &f in t::SET_EXTERNAL {
                let label = format!("{what} setToExternalBuffer {f:#x} {label_with}");
                let r = tw.run(&label, f, member, &[object, JUNK[0], JUNK[1]], |g| {
                    t::bridge::member_set_external(g, member, object, JUNK[0], JUNK[1])
                });
                if with {
                    assert_eq!(r, 0x0ace_0002, "{label}");
                    assert_eq!(
                        (tw.entry(0, 1), tw.entry(0, 2), tw.entry(0, 3)),
                        (object, JUNK[0], JUNK[1]),
                        "{label}"
                    );
                } else {
                    assert_eq!((r, tw.logged()), (0x15, 0), "{label}");
                }
            }
        }
        for &f in t::SET_COUNT {
            let label = format!("{what} setCount {f:#x}");
            let r = tw.run(&label, f, member, &[object, 3], |_| {
                t::bridge::set_count_unsupported()
            });
            assert_eq!(r, 0x17, "{label}");
        }
    }
}

/// The three `PClassDataMemberArray` slots.
#[test]
fn m5p_tmpl_dynamic_array_slots() {
    for (what, mut tw) in Twins::all() {
        tw.stub(t::SCRIPT_GET, false, 4, 0, 0x0bad_0003);
        tw.stub(t::SCRIPT_PUT_FLAGGED, false, 4, 0, 0x0bad_0004);
        tw.stub(t::SCRIPT_PUT, false, 4, 0, 0x0bad_0005);
        let member = tw.zeros(0x40);
        let vtable = tw.zeros(0x40);
        tw.w32(member, vtable);
        for (slot, words) in [(4u32, 1u32), (6, 1), (7, 1), (9, 0)] {
            let code = tw.stub_code(0x100 + slot, true, words, words, 0x0ace_0000 + slot);
            tw.w32(vtable + 4 * slot, code);
        }
        for flag in [1u32, 0, 0x100, 0xffff_ff01] {
            let code = tw.stub_code(0x10a, true, 0, 0, flag);
            tw.w32(vtable + 40, code);
            for (state, object) in [(0, 0), (JUNK[0], JUNK[1])] {
                let label = format!("{what} array slot 2 state {state:#x} flag {flag:#x}");
                let r = tw.run(&label, t::ARRAY_SLOT2, member, &[state, object], |g| {
                    t::bridge::script_get_array(g, member, state, object)
                });
                assert_eq!(r, 0x0bad_0003, "{label}");
                assert_eq!(tw.logged(), 4, "{label}: calls");
                let label = format!("{what} array slot 3 state {state:#x} flag {flag:#x}");
                let r = tw.run(&label, t::ARRAY_SLOT3, member, &[state, object], |g| {
                    t::bridge::script_put_array(g, member, state, object)
                });
                assert_eq!(
                    r,
                    if flag & 0xff != 0 {
                        0x0bad_0004
                    } else {
                        0x0bad_0005
                    }
                );
                assert_eq!(tw.logged(), 5, "{label}: calls");
                assert_eq!(
                    (
                        tw.entry(4, 2),
                        tw.entry(4, 3),
                        tw.entry(4, 4),
                        tw.entry(4, 5)
                    ),
                    (state, 0x0ace_0004, 0x0ace_0007, 0x0ace_0009),
                    "{label}"
                );
            }
        }
        for flags in [0u32, 1, 2, 3, 0xffff_fffd, 0xffff_ffff] {
            tw.w32(member + 0x14, flags);
            let label = format!("{what} array slot 10 flags {flags:#x}");
            let r = tw.run(&label, t::ARRAY_SLOT10, member, &[], |g| {
                t::bridge::flag_bit1(g, member)
            });
            assert_eq!(r, (flags >> 1) & 1, "{label}");
        }
        tw.unstub();
    }
}

/// Every address of the list is in exactly one table (or one of the three
/// single ports).
#[test]
fn m5p_tmpl_dynamic_tables_cover_the_shims() {
    let mut all: Vec<u32> = Vec::new();
    all.extend(t::DTORS.iter().map(|r| r.0));
    all.extend(t::GET_COUNT);
    all.extend(t::SET_COUNT);
    all.extend(t::GET_START);
    all.extend(t::SET_EXTERNAL);
    all.extend([t::ARRAY_SLOT2, t::ARRAY_SLOT3, t::ARRAY_SLOT10]);
    let n = all.len();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), n, "an address in two tables");
    assert_eq!(n, 362);
}
