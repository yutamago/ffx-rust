//! Continuation round, batch 8 on: the ports in `ffx_prender::p6_more`
//! (and the copies of other lists' shapes) against their originals.

use std::path::PathBuf;

use ffx_oracle::Conv;
use ffx_prender::declare as dc;
use ffx_prender::p6_more as more;

use super::{DESC, G, Out, Rng, Twins, object};

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
            t.a.call(addr, Conv::Thiscall(helper), &[]).unwrap();
            more::bridge::compact_delegations(&mut G(&mut t.b), helper);
            t.diff(&format!("{name} compact #{i} (n {n})"));
        }
        t.stub(more::DELEGATION_FREE_LIST_INIT, true, 0, 0, rng.next());
        for i in 0..4 {
            let helper = object(&mut t, &mut rng, 4, false);
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
