//! Continuation round, batch 8 on: the ports in `ffx_prender::p6_more`
//! (and the copies of other lists' shapes) against their originals.

use std::path::PathBuf;

use ffx_oracle::Conv;
use ffx_prender::declare as dc;
use ffx_prender::h1_bind as h1b;
use ffx_prender::h2_bind::{self as hb, Step, Ty};
use ffx_prender::p6_binds as binds;
use ffx_prender::p6_binds1 as binds1;
use ffx_prender::p6_more as more;

use super::{DESC, G, Out, Rng, Twins, object};

/// Empties the call log (it holds 1,023 entries; the count would run on
/// into other allocations).
fn fresh(t: &mut Twins) {
    let log = t.log;
    t.w32(log, 0);
}

#[test]
fn binds_match_the_original() {
    let src = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ffx-hook/src/ported/p6_render_c2.rs"),
    )
    .unwrap();
    for &(a, _) in binds::BINDS {
        let key = format!(" @ {:#06x}_{:04x} => ", a >> 16, a & 0xffff);
        let at = src.find(&key).unwrap_or_else(|| panic!("{a:#x}: no shim"));
        assert!(
            src[at..].contains(&format!("&binds::B_{a:08X})")),
            "{a:#x}: shim table"
        );
    }
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0009);
        for &(f, b) in binds::BINDS {
            // (callee, logs ecx, words logged, words popped)
            let mut callees = vec![
                (dc::REGISTER_TYPE, false, 1, 0),
                (dc::ADD_TO_NAMESPACE, true, 0, 0),
                (dc::ATEXIT, false, 1, 0),
            ];
            for step in b.steps {
                match step {
                    Step::Member(m) => {
                        callees.push((m.ctor, true, 6, 6));
                        match m.ty {
                            Ty::Get(g) => callees.push((g, false, 0, 0)),
                            Ty::GetArg(g, _) => callees.push((g, false, 1, 0)),
                            Ty::Image(_) => {}
                        }
                    }
                    Step::Array(a) => {
                        callees.push((dc::MEMBER_ARRAY_CTOR, true, 6, 7));
                        callees.push((a.desc_ctor, true, 0, 0));
                    }
                    Step::Base { func, .. } => callees.push((*func, false, 1, 0)),
                    Step::Or { .. } => {}
                }
            }
            callees.sort_by_key(|c| c.0);
            callees.dedup_by_key(|c| c.0);
            for &(s, this, n, pops) in &callees {
                t.stub(s, this, n, pops, rng.next());
            }
            for i in 0..12 {
                for step in b.steps {
                    let guards = match step {
                        Step::Member(m) => vec![m.guard.0],
                        Step::Array(a) => {
                            let d = t.rebase(a.desc);
                            let v = rng.next();
                            t.w32(d + 0x90, v);
                            vec![a.guard.0, a.desc_guard.0]
                        }
                        Step::Base { global, .. } => {
                            let g = t.rebase(*global);
                            let v = rng.next();
                            t.w32(g, v);
                            vec![]
                        }
                        Step::Or { at, .. } => {
                            let at = t.rebase(*at);
                            let v = rng.next();
                            t.w32(at, v);
                            vec![]
                        }
                    };
                    for g in guards {
                        let g = t.rebase(g);
                        let v = match i {
                            0 => 0,
                            1 => u32::MAX,
                            _ => rng.next() & rng.next(),
                        };
                        t.w32(g, v);
                    }
                }
                fresh(&mut t);
                t.check(&format!("{name} {f:#x} #{i}"), f, Conv::Cdecl, &[], |g| {
                    Out::U32(hb::bridge::bind(g, b))
                });
            }
            for &(s, ..) in &callees {
                t.unstub(s);
            }
        }
    }
}

#[test]
fn more_shims_match_their_tables() {
    let src = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ffx-hook/src/ported/p6_render_c2.rs"),
    )
    .unwrap();
    let hex = |a: u32| format!("{:#06x}_{:04x}", a >> 16, a & 0xffff);
    let mut want: Vec<(u32, String)> = Vec::new();
    for &(a, _) in more::ABSTRACT {
        want.push((a, format!("more::abstract_of({})", hex(a))));
    }
    for &(a, _) in more::DECLARE {
        want.push((a, format!("more::declare_of({})", hex(a))));
    }
    for &(a, _) in more::DELEGATION_RUNS {
        want.push((a, format!("more::delegation_flag({})", hex(a))));
    }
    for (i, &(a, _, _)) in more::SEMANTIC_CHECKS.iter().enumerate() {
        want.push((a, format!("more::SEMANTIC_CHECKS[{i}]")));
    }
    for (i, &(a, _, _)) in more::CONSTRUCT_CHECKED.iter().enumerate() {
        want.push((a, format!("more::CONSTRUCT_CHECKED[{i}]")));
    }
    for &a in more::MOUSE_DELTA_CHECKS {
        want.push((a, "check_mouse_delta(&mut mem(), this)".into()));
    }
    for (a, body) in &want {
        let key = format!(" @ {} => ", hex(*a));
        let at = src
            .find(&key)
            .unwrap_or_else(|| panic!("{a:#x}: no ported! entry"));
        let end = src[at..].find("\n    }").map_or(src.len(), |e| at + e + 6);
        assert!(
            src[at..end].contains(body.as_str()),
            "{a:#x}: shim body lacks {body:?}"
        );
        assert_eq!(src.matches(&key).count(), 1, "{a:#x}: hooked twice");
    }
    assert_eq!(want.len(), 9 + 9 + 10 + 2 + 4 + 2);
}

#[test]
fn more_abstract_ctors_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0001);
        let sets = [
            dc::SET_DEFAULT_BUFFER,
            dc::SET_VALIDATION_BUFFER,
            dc::SET_WRITE_MASK_BUFFER,
        ];
        for &(f, c) in more::ABSTRACT {
            t.stub(dc::PBASE_DESCRIPTOR, false, 0, 0, rng.next());
            t.stub(dc::GLOBAL_NAMESPACE, false, 0, 0, rng.next());
            t.stub(dc::DESCRIPTOR_CTOR, true, 6, 6, rng.next());
            for s in sets {
                t.stub(s, true, 1, 1, rng.next());
            }
            for i in 0..6 {
                if let dc::Name::Global(g) = c.name {
                    let g = t.rebase(g);
                    let v = if i == 0 { 0 } else { rng.next() };
                    t.w32(g, v);
                }
                let this = object(&mut t, &mut rng, 0x30, false);
                fresh(&mut t);
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(this),
                    &[],
                    |g| Out::U32(dc::bridge::abstract_ctor(g, this, &c)),
                );
            }
            for s in [
                dc::PBASE_DESCRIPTOR,
                dc::GLOBAL_NAMESPACE,
                dc::DESCRIPTOR_CTOR,
            ] {
                t.unstub(s);
            }
            for s in sets {
                t.unstub(s);
            }
        }
    }
}

#[test]
fn more_declares_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0002);
        for &(f, c) in more::DECLARE {
            // (callee, logs ecx, words logged, words popped)
            let mut callees = vec![
                (dc::ATEXIT, false, 1, 0),
                (dc::TYPE_UINT, false, 0, 0),
                (dc::DATA_MEMBER_CTOR, true, 6, 6),
                (dc::ANNOTATION_CTOR, true, 3, 3),
                (dc::ADD_ANNOTATION, true, 1, 1),
                (dc::MEMBER_ARRAY_CTOR, true, 6, 7),
                (dc::REGISTER_TYPE, false, 1, 0),
                (dc::ADD_TO_NAMESPACE, true, 0, 0),
                (c.ctor, true, 0, 0),
            ];
            if let dc::Element::Getter(g) = c.element {
                callees.push((g, false, 0, 0));
            }
            for &(s, this, n, pops) in &callees {
                t.stub(s, this, n, pops, rng.next());
            }
            let d = t.rebase(c.desc);
            for i in 0..16 {
                let g = rng.below(16) | if rng.chance(3) { rng.next() & !0xf } else { 0 };
                t.w32(d + 0x94, g);
                let n = if rng.chance(3) { rng.next() | 1 } else { 0 };
                t.w32(d + 0x18, n);
                let ng = t.rebase(c.name);
                let v = if rng.chance(2) { rng.next() | 1 } else { 0 };
                t.w32(ng, v);
                let head = if rng.chance(2) { d + 0xc8 } else { rng.next() };
                t.w32(d + 0xc8, head);
                let fl = rng.next();
                t.w32(d + 0x90, fl);
                for off in [0x10c, 0x114, 0x118, 0x11c] {
                    let w = rng.next();
                    t.w32(d + off, w);
                }
                let arg = if i % 8 == 7 { 0 } else { rng.next() | 1 };
                fresh(&mut t);
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Cdecl,
                    &[arg],
                    |g| Out::U32(dc::bridge::declare(g, arg, &c)),
                );
            }
            for &(s, ..) in &callees {
                t.unstub(s);
            }
        }
    }
}

#[test]
fn more_skip_empty_copy_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0003);
        // the next-list callee: [ecx+8] = [[ecx+8]+0x20], logged
        let log = t.log;
        let mut c = vec![0x8b, 0x41, 0x08]; // mov eax, [ecx+8]
        c.extend([0x85, 0xc0, 0x74, 0x06]); // test eax, eax; je +6
        c.extend([0x8b, 0x40, 0x20]); // mov eax, [eax+0x20]
        c.extend([0x89, 0x41, 0x08]); // mov [ecx+8], eax
        c.extend([0xff, 0x05]);
        c.extend(log.to_le_bytes()); // inc dword [log]
        c.push(0xc3);
        let code = t.code(&c);
        t.jump(0x0044_3ef0, code);
        for i in 0..40 {
            let cur = object(&mut t, &mut rng, 0x10, false);
            let base = t.zeros(0x1000);
            let lists = 1 + rng.below(3);
            let mut prev = cur + 8 - 0x20;
            for l in 0..=lists {
                let list = base + 0x100 * l + 0x20;
                t.w32(prev + 0x20, list);
                prev = list;
                let class = list + 0x30;
                let stride = 4 * (1 + rng.below(3));
                t.w32(list + 0xc, class);
                t.w32(class + 0x1c, stride);
                let w = rng.next();
                t.w32(class + 0x8c, w);
                let slots = base + 0x500 + 0x40 * l;
                t.w32(list + 0x14, slots);
                let n = rng.below(4);
                t.w32(list + 0x1c, n);
                let free = rng.below(n + 1);
                for k in 0..free {
                    let next = if k + 1 < free {
                        slots + stride * (k + 1)
                    } else {
                        rng.next()
                    };
                    t.w32(slots + stride * k, next);
                }
                let head = if free > 0 { slots } else { rng.next() | 2 };
                t.w32(list - 0x18, head);
            }
            t.w32(prev + 0x20, 0);
            let left = if rng.chance(4) { 1 + rng.below(3) } else { 0 };
            t.w32(cur + 0x18, left);
            let addr = t.rebase(0x009f_3930);
            fresh(&mut t);
            t.a.call(addr, Conv::Fastcall, &[cur]).unwrap();
            ffx_prender::h2_more::bridge::skip_empty(&mut G(&mut t.b), cur);
            t.diff(&format!("{name} skipEmpty #{i}"));
        }
        t.unstub(0x0044_3ef0);
    }
}

/// A semantic object whose kind (virtual slot 1, thiscall) is `kind`;
/// the kind call is logged.
fn semantic(t: &mut Twins, kind: u32) -> u32 {
    let f = t.stub_code(0x5e5e_0000 | (kind & 0xffff), true, 0, 0, kind);
    let vtable = t.zeros(8);
    t.w32(vtable + 4, f);
    let obj = t.zeros(8);
    t.w32(obj, vtable);
    obj
}

#[test]
fn more_semantic_checks_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0004);
        let kinds = [
            0x1a,
            0x1b,
            0x1c,
            0x1d,
            5,
            0xffff_ffff,
            0x30,
            0,
            0xb,
            0xc,
            0xd,
            0xe,
            0xa,
        ];
        let sems: Vec<u32> = kinds.iter().map(|&k| semantic(&mut t, k)).collect();
        let pick = |rng: &mut Rng| {
            if rng.chance(5) {
                0
            } else {
                sems[rng.below(sems.len() as u32) as usize]
            }
        };
        for i in 0..60 {
            let src = object(&mut t, &mut rng, 0x10, false);
            let (a, b) = (pick(&mut rng), pick(&mut rng));
            for &(f, rule, default) in more::SEMANTIC_CHECKS {
                t.w32(src + 0x20, a);
                t.w32(src + 0x24, b);
                fresh(&mut t);
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(src),
                    &[],
                    |g| Out::U32(more::bridge::check_semantics(g, src, rule, default)),
                );
            }
            for &f in more::MOUSE_DELTA_CHECKS {
                t.w32(src + 0x20, a);
                t.w32(src + 0x24, b);
                fresh(&mut t);
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(src),
                    &[],
                    |g| Out::U32(more::bridge::check_mouse_delta(g, src)),
                );
            }
        }
    }
}

#[test]
fn more_input_constructors_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0005);
        for &(f, fields, check) in more::CONSTRUCT_CHECKED {
            t.stub(check, true, 0, 0, rng.next());
            for i in 0..6 {
                let obj = object(&mut t, &mut rng, 0x10, i == 0);
                fresh(&mut t);
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(DESC),
                    &[obj],
                    |g| Out::U8(more::bridge::construct_checked(g, obj, fields, check)),
                );
            }
            t.unstub(check);
        }
        for i in 0..6 {
            let obj = object(&mut t, &mut rng, 0x10, i == 0);
            fresh(&mut t);
            t.check(
                &format!("{name} mouse button #{i}"),
                0x009e_b470,
                Conv::Thiscall(DESC),
                &[obj],
                |g| {
                    Out::U8(more::bridge::construct_mouse_button(
                        g,
                        obj,
                        more::CONSTRUCT_MOUSE_BUTTON,
                    ))
                },
            );
        }
    }
}

/// A delegation entry with flags from `flags` (each bit at even odds),
/// `callback` at `+0x10`, `arg` at `+0`, a buffer at `+4` or none.
fn delegation(t: &mut Twins, rng: &mut Rng, flags: &[u32], callback: u32, arg: u32) -> u32 {
    let e = object(t, rng, 6, false);
    let mut fl = 0;
    for &b in flags {
        if rng.chance(2) {
            fl |= b;
        }
    }
    t.w32(e + 0xc, fl | (rng.next() & 0xfff0_0000));
    if rng.chance(3) {
        t.w32(e + 4, 0);
    }
    t.w32(e + 0x10, callback);
    t.w32(e, arg);
    e
}

/// A delegation helper of `n` slots (count's high bit sometimes set):
/// entries null or [`delegation`]s whose argument is their own slot (the
/// callback then stores the entry in `cell` there) or a scratch word.
fn delegations(
    t: &mut Twins,
    rng: &mut Rng,
    n: u32,
    flags: &[u32],
    callback: u32,
    scratch: u32,
) -> u32 {
    let helper = t.zeros(8);
    let data = t.zeros(4 * n as usize + 4);
    t.w32(helper, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
    t.w32(helper + 4, data);
    for k in 0..n {
        if rng.chance(4) {
            continue;
        }
        let arg = if rng.chance(3) { data + 4 * k } else { scratch };
        let e = delegation(t, rng, flags, callback, arg);
        t.w32(data + 4 * k, e);
    }
    helper
}

#[test]
fn more_delegation_runs_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0006);
        let wait = t.stub_code(0x7777_0001, false, 2, 2, 0);
        let release = t.stub_code(0x7777_0002, false, 1, 1, 0x1234_5678);
        let ws = t.rebase(more::WAIT_FOR_SINGLE_OBJECT);
        let rs = t.rebase(more::RELEASE_MUTEX);
        t.w32(ws, wait);
        t.w32(rs, release);
        let mutex = t.rebase(more::DELEGATION_MUTEX);
        // the callback (cdecl, one word): stores the word at `cell` where
        // its argument points (its own slot, or a scratch word)
        let cell = t.zeros(4);
        let scratch = t.zeros(4);
        let mut c = vec![0x8b, 0x44, 0x24, 0x04]; // mov eax, [esp+4]
        c.extend([0x8b, 0x0d]);
        c.extend(cell.to_le_bytes()); // mov ecx, [cell]
        c.extend([0x89, 0x08, 0xc3]); // mov [eax], ecx; ret
        let callback = t.code(&c);
        t.stub(more::OPERATOR_DELETE, false, 1, 0, 0);
        for &(f, flag) in more::DELEGATION_RUNS {
            for i in 0..12 {
                let r = match i % 3 {
                    0 => 0xffff_ffff,
                    1 => 0,
                    _ => 1 + rng.below(64),
                };
                t.stub(more::SET_DATA_TO_FREE_LIST, true, 1, 1, r);
                let m = rng.next();
                t.w32(mutex, m);
                let replacement = delegation(&mut t, &mut rng, &[flag, 1], callback, scratch);
                t.w32(cell, replacement);
                let n = rng.below(7);
                let flags = [flag, 1, flag << 1];
                let helper = delegations(&mut t, &mut rng, n, &flags, callback, scratch);
                fresh(&mut t);
                t.check(
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    Conv::Thiscall(helper),
                    &[],
                    |g| Out::U32(more::bridge::run_delegations(g, helper, flag)),
                );
                t.unstub(more::SET_DATA_TO_FREE_LIST);
            }
        }
        t.unstub(more::OPERATOR_DELETE);
    }
}

#[test]
fn more_delegation_compact_and_ctor_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_0007);
        for i in 0..80 {
            let n = rng.below(9);
            let helper = t.zeros(8);
            let data = t.zeros(4 * n as usize + 8);
            t.w32(helper, n | if rng.chance(3) { 0x8000_0000 } else { 0 });
            t.w32(helper + 4, data);
            for k in 0..=n {
                let v = if rng.chance(2) { 0 } else { rng.next() | 1 };
                t.w32(data + 4 * k, v);
            }
            let addr = t.rebase(more::COMPACT);
            fresh(&mut t);
            t.a.call(addr, Conv::Thiscall(helper), &[]).unwrap();
            more::bridge::compact_delegations(&mut G(&mut t.b), helper);
            t.diff(&format!("{name} compact #{i} (n {n})"));
        }
        t.stub(more::DELEGATION_FREE_LIST_INIT, true, 0, 0, rng.next());
        for i in 0..4 {
            let helper = object(&mut t, &mut rng, 4, false);
            fresh(&mut t);
            t.check(
                &format!("{name} ctor #{i}"),
                0x006a_d0d0,
                Conv::Thiscall(helper),
                &[],
                |g| Out::U32(more::bridge::delegation_helper_ctor(g, helper)),
            );
        }
        t.unstub(more::DELEGATION_FREE_LIST_INIT);
    }
}

/// The `Bind`s' dropped-result calls that take a word (cdecl): the
/// `PArray<T>` declarations (`DeclareClassDescriptor(name)`), from their
/// call sites' `push dword [global]`.
const ONE_WORD_CALLS: &[u32] = &[
    0x0049_0950,
    0x0050_9da0,
    0x0053_7400,
    0x005a_b580,
    0x005a_b790,
    0x005a_b9a0,
    0x005b_17c0,
    0x005c_18c0,
    0x005c_1ad0,
    0x005c_1ce0,
    0x00a0_74c0,
    0x00a2_c6c0,
];

/// Every guard word, flag word (`Or`/`And`, and the globals `Call`
/// passes), call target (with its
/// words: 0x100 for a descriptor constructor, thiscall) and class member
/// of an h1-format `Bind`'s steps.
fn h1_bind_parts(
    steps: &[h1b::Step],
    guards: &mut Vec<u32>,
    flags: &mut Vec<u32>,
    calls: &mut Vec<(u32, u32)>,
    classes: &mut Vec<u32>,
) {
    use h1b::{Step, Ty as T1};
    let ty = |t: T1, calls: &mut Vec<(u32, u32)>| match t {
        T1::Addr(_) => {}
        T1::Call(f) => calls.push((f, 0)),
        T1::CallWith(f, _) => calls.push((f, 1)),
    };
    for s in steps {
        match *s {
            Step::Guard(g, _, inner) => {
                if !guards.contains(&g) {
                    guards.push(g);
                }
                h1_bind_parts(inner, guards, flags, calls, classes);
            }
            Step::Member { ty: t, .. } | Step::Array { ty: t, .. } => ty(t, calls),
            Step::Class { member, .. } => classes.push(member),
            Step::Call(f, arg) => {
                // the stub's words come from the machine code, not from
                // the table under test
                assert_eq!(arg.is_some(), ONE_WORD_CALLS.contains(&f), "{f:#x}");
                calls.push((f, u32::from(ONE_WORD_CALLS.contains(&f))));
                // the argument's global, randomised like a flag word
                if let Some(g) = arg {
                    flags.push(g);
                }
            }
            Step::Descriptor { ctor, .. } => calls.push((ctor, 0x100)),
            Step::Or(at, _) | Step::And(at, _) => flags.push(at),
            Step::AtExit(_) | Step::Store(..) => {}
        }
    }
}

#[test]
fn binds_with_class_members_match_the_original() {
    let src = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ffx-hook/src/ported/p6_render_c2.rs"),
    )
    .unwrap();
    for &(a, _) in binds1::BINDS {
        let hex = format!("{:#06x}_{:04x}", a >> 16, a & 0xffff);
        let at = src
            .find(&format!(" @ {hex} => "))
            .unwrap_or_else(|| panic!("{a:#x}: no shim"));
        assert!(
            src[at..].contains(&format!("binds1::bind_of({hex})")),
            "{a:#x}: shim table"
        );
    }
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_000a);
        t.stub(dc::ATEXIT, false, 1, 0, 0);
        t.stub(dc::REGISTER_TYPE, false, 1, 0, 0x0101);
        t.stub(dc::ADD_TO_NAMESPACE, true, 0, 0, 0x5a5a_1234);
        for ctor in [dc::DATA_MEMBER_CTOR, 0x0057_5f30, 0x0057_5370] {
            t.stub(ctor, true, 6, 6, 6);
        }
        t.stub(h1b::ARRAY_MEMBER_CTOR, true, 6, 7, 7);
        t.stub(h1b::CLASS_MEMBER_CTOR, true, 3, 3, 3);
        let mut stubbed = vec![];
        for &(f, k) in binds1::BINDS {
            let (mut guards, mut flags, mut calls, mut classes) = (vec![], vec![], vec![], vec![]);
            h1_bind_parts(k.steps, &mut guards, &mut flags, &mut calls, &mut classes);
            for (c, kind) in calls {
                if stubbed.contains(&c) {
                    continue;
                }
                stubbed.push(c);
                if kind == 0x100 {
                    t.stub(c, true, 0, 0, 0xd);
                } else {
                    // cdecl getters: their ecx is the caller's leftover
                    t.stub(c, false, kind, 0, 0x00c9_7000 | (c & 0xfff));
                }
            }
            for case in 0..12u32 {
                assert_eq!(binds1::bind_of(f), k);
                for &g in &guards {
                    let word = match case {
                        0 => 0,
                        1 => u32::MAX,
                        _ => rng.next() & rng.next(),
                    };
                    let at = t.rebase(g);
                    t.w32(at, word);
                }
                for &w in &flags {
                    let at = t.rebase(w);
                    let v = rng.next();
                    t.w32(at, v);
                }
                // class members: an owner with a list (empty, or two
                // nodes), a name or none, the owner knowing it or not
                for &m in &classes {
                    let m = t.rebase(m);
                    let owner = t.zeros(0x60);
                    let head = owner + 0x4c;
                    let first = t.zeros(8);
                    let second = t.zeros(8);
                    if rng.chance(2) {
                        t.w32(head, head);
                        t.w32(head + 4, head);
                    } else {
                        t.w32(head, first);
                        t.w32(first, second);
                        t.w32(second, head);
                        t.w32(head + 4, second);
                        t.w32(second + 4, first);
                        t.w32(first + 4, head);
                    }
                    t.w32(m + 8, owner);
                    let named = if rng.chance(3) { 0 } else { owner };
                    t.w32(m + 0xc, named);
                }
                t.stub(h1b::FIND_MEMBER, true, 1, 1, rng.next() & 1);
                fresh(&mut t);
                t.check(
                    &format!("{name} {f:#x} case {case}"),
                    f,
                    Conv::Cdecl,
                    &[],
                    |g| Out::U32(h1b::bridge::bind(g, k)),
                );
                t.unstub(h1b::FIND_MEMBER);
            }
        }
    }
}
