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
//!
//! The array methods: every resize from headers of counts 0, 1, 2 and 5,
//! owned and external, to counts equal and other, with the helper's or
//! `PhyreMalloc`'s block new, 0 (the 0xd path) and the old block itself;
//! the `PSharray` versions also with the inline word as the new block and
//! an inline string with bit 0 set and clear; the block helpers for every
//! kept count and a failing `PhyreMalloc`; the element constructors for 0,
//! 1 and several elements and an element at address 0.
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

/// The resizes' stubs: every helper, copy, constructor and release of the
/// tables, at its address (tags are the addresses).
fn stub_resize_callees(tw: &mut Twins) {
    let mut done = std::collections::HashSet::new();
    let mut stub = |tw: &mut Twins, f: u32, this: bool, n: u32, pops: u32| {
        if f != 0 && done.insert(f) {
            tw.stub(f, this, n, pops, 0x0bad_0000 | (f & 0xffff));
        }
    };
    stub(tw, t::FREE, false, 1, 0);
    for &(_, (realloc, how)) in t::RESIZE_REALLOC {
        stub(tw, realloc, false, 3, 0);
        if let t::Release::Elements(f) = how {
            stub(tw, f, false, 2, 0);
        }
    }
    for &(_, g) in t::RESIZE_MALLOC {
        stub(tw, g.copy, false, 3, 0);
        stub(tw, g.ctor, false, 2, 0);
        if let t::Release::Elements(f) = g.release {
            stub(tw, f, false, 2, 0);
        }
    }
    let [_, copy, ctor, setter] = t::DATA_BLOCK_RESIZE_PARTS;
    stub(tw, copy, false, 3, 0);
    stub(tw, ctor, false, 2, 0);
    stub(tw, setter, true, 2, 2);
    stub(tw, t::STRING_RELEASE, false, 2, 0);
    stub(tw, t::SHARRAY_STRING_COPY, false, 3, 0);
}

/// Header words and counts for the resizes: equal, other counts, owned
/// and external.
const RESIZE_HEADERS: [u32; 6] = [0, 1, 2, 5, 0x8000_0002, 0x8000_0005];
const RESIZE_COUNTS: [u32; 5] = [0, 1, 2, 5, 0x8000_0002];

#[test]
fn m5p_tmpl_phyrearray_resizes() {
    for (what, mut tw) in Twins::all() {
        stub_resize_callees(&mut tw);
        let array = tw.zeros(0x10);
        let old = tw.zeros(0x80);
        let block = tw.zeros(0x400);
        for header in RESIZE_HEADERS {
            for count in RESIZE_COUNTS {
                for data in [0, old] {
                    // the helper's / PhyreMalloc's block: new, none, the old one
                    for got in [block, 0, data] {
                        let case = format!(
                            "header {header:#x} count {count:#x} data {data:#x} block {got:#x}"
                        );
                        for &(f, (realloc, how)) in t::RESIZE_REALLOC {
                            tw.stub(realloc, false, 3, 0, got);
                            tw.w32(array, header);
                            tw.w32(array + 4, data);
                            let label = format!("{what} resize {f:#x} {case}");
                            tw.run(&label, f, array, &[count], |g| {
                                t::arrays::resize_realloc(g, array, count, realloc, how)
                            });
                        }
                        tw.stub(t::MALLOC, false, 2, 0, got);
                        for &(f, grow) in t::RESIZE_MALLOC {
                            tw.w32(array, header);
                            tw.w32(array + 4, data);
                            let label = format!("{what} resize {f:#x} {case}");
                            tw.run(&label, f, array, &[count], |g| {
                                t::arrays::resize_malloc(g, array, count, grow)
                            });
                        }
                        tw.w32(array, header);
                        tw.w32(array + 4, data);
                        let label = format!("{what} resize {:#x} {case}", t::DATA_BLOCK_RESIZE);
                        tw.run(&label, t::DATA_BLOCK_RESIZE, array, &[count], |g| {
                            t::arrays::resize_malloc_set(
                                g,
                                array,
                                count,
                                t::DATA_BLOCK_RESIZE_PARTS,
                            )
                        });
                    }
                }
            }
        }
        tw.unstub();
    }
}

#[test]
fn m5p_tmpl_phyrearray_sharray_resizes() {
    for (what, mut tw) in Twins::all() {
        stub_resize_callees(&mut tw);
        let array = tw.zeros(0x10);
        let old = tw.zeros(0x80);
        let block = tw.zeros(0x400);
        for header in RESIZE_HEADERS.into_iter().chain([0x8000_0000, 0x8000_0001]) {
            for count in RESIZE_COUNTS {
                for data in [0, old, 0x0123_4560, 0x0123_4561] {
                    for got in [block, 0, array + 4, old] {
                        let case = format!(
                            "header {header:#x} count {count:#x} data {data:#x} block {got:#x}"
                        );
                        for &(f, alloc) in t::SHARRAY_RESIZE {
                            tw.stub(alloc, true, 3, 3, got);
                            tw.w32(array, header);
                            tw.w32(array + 4, data);
                            let label = format!("{what} sharray resize {f:#x} {case}");
                            tw.run(&label, f, array, &[count], |g| {
                                t::arrays::sharray_resize(g, array, count, alloc)
                            });
                        }
                        tw.stub(t::SHARRAY_STRING_ALLOC, true, 3, 3, got);
                        tw.w32(array, header);
                        tw.w32(array + 4, data);
                        let label = format!("{what} PSharray<PString>::resize {case}");
                        tw.run(&label, t::SHARRAY_STRING_RESIZE, array, &[count], |g| {
                            t::arrays::sharray_string_resize(g, array, count)
                        });
                    }
                }
            }
        }
        tw.unstub();
    }
}

#[test]
fn m5p_tmpl_phyrearray_sharray_blocks() {
    for (what, mut tw) in Twins::all() {
        stub_resize_callees(&mut tw);
        let array = tw.zeros(0x10);
        let src = tw.zeros(0x40);
        let words: Vec<u8> = (0u8..0x40).map(|i| i ^ 0x3c).collect();
        tw.write(src, &words);
        let block = tw.zeros(0x40);
        for got in [block, 0] {
            tw.stub(t::MALLOC, false, 2, 0, got);
            for count in [0u32, 1, 2, 5] {
                for kept in 0..=count {
                    tw.write(block, &[0x5a; 0x40]);
                    tw.w32(array + 4, 0x0777_7770);
                    let case = format!("count {count} kept {kept} malloc {got:#x}");
                    let label = format!("{what} PSharray<PString> block {case}");
                    tw.run(
                        &label,
                        t::SHARRAY_STRING_ALLOC,
                        array,
                        &[count, src, kept],
                        |g| t::arrays::sharray_string_alloc(g, array, count, src, kept),
                    );
                    tw.write(block, &[0x5a; 0x40]);
                    tw.w32(array + 4, 0x0777_7770);
                    let label = format!("{what} PSharray<PPhysicsShape *> block {case}");
                    tw.run(
                        &label,
                        t::SHARRAY_ALLOC_INLINE,
                        array,
                        &[count, src, kept],
                        |g| t::arrays::sharray_alloc_inline(g, array, count, src, kept),
                    );
                }
            }
        }
        tw.unstub();
    }
}

#[test]
fn m5p_tmpl_phyrearray_sharray_external() {
    for (what, mut tw) in Twins::all() {
        tw.stub(t::FREE, false, 1, 0, 0x0bad_0002);
        let array = tw.zeros(0x10);
        let buffer = tw.zeros(0x10);
        tw.w32(buffer, 0x0abc_0001);
        let old = tw.zeros(0x10);
        for header in [0, 1, 3, 0x8000_0001, 0x8000_0003] {
            for data in [0, old, buffer] {
                for (buf, count) in [
                    (buffer, 0),
                    (buffer, 1),
                    (buffer, 3),
                    (buffer, u32::MAX),
                    (array + 4, 1),
                ] {
                    tw.w32(array, header);
                    tw.w32(array + 4, data);
                    let label = format!(
                        "{what} PSharray setToExternalBuffer header {header:#x} data {data:#x} buffer {buf:#x} count {count:#x}"
                    );
                    tw.run(&label, t::SHARRAY_SET_EXTERNAL, array, &[buf, count], |g| {
                        t::arrays::sharray_set_external(g, array, buf, count)
                    });
                }
            }
        }
        // the override: the global object's slot 1 answers 0 or an object
        tw.stub(t::LOD_SET_ONE, true, 2, 2, 0x0bad_0010);
        tw.stub(t::SHARRAY_SET_EXTERNAL, true, 2, 2, 0x0bad_0011);
        let global = tw.rebase(t::LOD_OBJECT);
        let saved: u32 = ffx_bridge::Mem::read(&tw.a, global);
        let vtable = tw.zeros(0x10);
        tw.w32(global, vtable);
        for answer in [0, 0x0012_3450] {
            let slot1 = tw.stub_code(0x0000_0101, true, 0, 0, answer);
            tw.w32(vtable + 4, slot1);
            for (buf, count) in [(array + 4, 1), (array + 4, 2), (buffer, 1), (buffer, 0)] {
                let label = format!(
                    "{what} PUtilityLOD setToExternalBuffer answer {answer:#x} buffer {buf:#x} count {count}"
                );
                let r = tw.run(
                    &label,
                    t::SHARRAY_SET_EXTERNAL_LOD,
                    array,
                    &[buf, count],
                    |g| t::arrays::sharray_set_external_lod(g, array, buf, count),
                );
                assert_eq!(r, 0, "{label}");
                assert_eq!(tw.logged(), 2, "{label}: calls");
                assert_eq!((tw.entry(0, 0), tw.entry(0, 1)), (0x101, global), "{label}");
                let direct = buf == array + 4 && count == 1 && answer != 0;
                let (f, this) = if direct {
                    (t::LOD_SET_ONE, answer)
                } else {
                    (t::SHARRAY_SET_EXTERNAL, array)
                };
                assert_eq!((tw.entry(1, 0), tw.entry(1, 1)), (f, this), "{label}");
                assert_eq!((tw.entry(1, 2), tw.entry(1, 3)), (buf, count), "{label}");
            }
        }
        tw.w32(global, saved);
        tw.unstub();
    }
}

#[test]
fn m5p_tmpl_phyrearray_elements() {
    for (what, mut tw) in Twins::all() {
        tw.stub(t::PBASE_DESCRIPTOR, false, 0, 0, 0x00de_5c00);
        let buffer = tw.zeros(0x200);
        for (dst, n) in [
            (buffer, 0u32),
            (buffer, 1),
            (buffer, 3),
            (buffer + 4, 2),
            (0, 1),
        ] {
            for &(f, (stride, words)) in t::ELEMENTS {
                tw.write(buffer, &[0x99; 0x200]);
                let label = format!("{what} elements {f:#x} dst {dst:#x} n {n}");
                tw.run_conv(&label, f, ffx_oracle::Conv::Cdecl, &[dst, n], |g| {
                    t::arrays::construct_elements(g, dst, n, stride, words);
                    None
                });
            }
            tw.write(buffer, &[0x99; 0x200]);
            let label = format!("{what} typed elements dst {dst:#x} n {n}");
            tw.run_conv(
                &label,
                t::TYPED_OBJECT_ELEMENTS,
                ffx_oracle::Conv::Cdecl,
                &[dst, n],
                |g| {
                    t::arrays::construct_typed_objects(g, dst, n);
                    None
                },
            );
            assert_eq!(tw.logged(), if dst == 0 { 0 } else { n }, "{label}: calls");
        }
        tw.unstub();
    }
}

/// `PClassDataMemberDynamicType<PAssetReference>`'s two slots.
#[test]
fn m5p_tmpl_phyrearray_dynamic_type() {
    for (what, mut tw) in Twins::all() {
        let [_, destruct, free] = t::DYNAMIC_TYPE_DTOR_PARTS;
        tw.stub(destruct, true, 0, 0, 0x0bad_0020);
        tw.stub(free, false, 1, 0, 0x0bad_0021);
        let this = tw.zeros(0x40);
        for flags in [0, 1, 2, 0xffff_ffff] {
            let label = format!("{what} dynamic type dtor flags {flags:#x}");
            let r = tw.run(&label, t::DYNAMIC_TYPE_DTOR, this, &[flags], |g| {
                t::arrays::dtor_with(g, this, flags, t::DYNAMIC_TYPE_DTOR_PARTS)
            });
            assert_eq!(r, this, "{label}");
            assert_eq!(tw.logged(), 1 + (flags & 1), "{label}: calls");
        }
        let ty = tw.zeros(0x20);
        tw.w32(ty + 0x18, 0x0d00_0018);
        tw.w32(this + 4, ty);
        tw.w32(this + 0xc, 0x30);
        for (with, answer) in [(false, 0), (true, 0), (true, 0x0e00_0001)] {
            let getter = tw.stub_code(0x0000_0202, true, 0, 0, answer);
            tw.w32(this + 8, if with { getter } else { 0 });
            let label = format!("{what} dynamic type slot 1 getter {with} answer {answer:#x}");
            let r = tw.run(&label, t::DYNAMIC_TYPE_SLOT1, this, &[0x0001_0000], |g| {
                t::arrays::dynamic_type_descriptor(g, this, 0x0001_0000)
            });
            assert_eq!(r, if answer != 0 { answer } else { 0x0d00_0018 }, "{label}");
            if with {
                assert_eq!(tw.entry(0, 1), 0x0001_0030, "{label}: the getter's this");
            }
        }
        tw.unstub();
    }
}

/// Every address of the two lists is in exactly one table (or one of
/// the single ports).
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
    all.extend(t::RESIZE_REALLOC.iter().map(|r| r.0));
    all.extend(t::RESIZE_MALLOC.iter().map(|r| r.0));
    all.extend(t::SHARRAY_RESIZE.iter().map(|r| r.0));
    all.extend(t::ELEMENTS.iter().map(|r| r.0));
    all.extend([
        t::DATA_BLOCK_RESIZE,
        t::SHARRAY_STRING_RESIZE,
        t::SHARRAY_STRING_ALLOC,
        t::SHARRAY_ALLOC_INLINE,
        t::SHARRAY_SET_EXTERNAL,
        t::SHARRAY_SET_EXTERNAL_LOD,
        t::DYNAMIC_TYPE_DTOR,
        t::DYNAMIC_TYPE_SLOT1,
        t::TYPED_OBJECT_ELEMENTS,
    ]);
    let n = all.len();
    all.sort_unstable();
    all.dedup();
    assert_eq!(all.len(), n, "an address in two tables");
    assert_eq!(n, 440);
}
