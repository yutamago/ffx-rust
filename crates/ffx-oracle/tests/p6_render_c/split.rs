//! Continuation round 5: the split full-screen mesh `0x00a06e60`
//! (`ffx_prender::p6_quads::bridge::split`) against its original.

#![allow(clippy::cast_precision_loss)] // small test values

use ffx_oracle::{Conv, Oracle};
use ffx_prender::p6_quads as qd;

use super::{G, Rng, Twins, object};

/// A stream as quads.rs builds them, with a vertex buffer of 64 words.
fn stream(t: &mut Twins, rng: &mut Rng, at: u32) {
    for k in 0..16 {
        t.w32(at + 4 * k, rng.next());
    }
    let record = object(t, rng, 3, false);
    t.w32(at + 0xc, record);
    let buffer = object(t, rng, 64, false);
    t.w32(at + 0x3c, buffer);
    let (a, b) = (rng.below(0x40), rng.below(0x20));
    t.w32(at, a);
    t.w32(at + 4, b);
    if rng.chance(2) {
        t.w32(at + 0x38, a * b);
    }
    let v = rng.below(4);
    t.w32(at + 0x10, v);
    let wrap = if rng.chance(2) { v + 1 } else { rng.below(6) };
    t.w32(at + 0x18, wrap | (rng.next() & 0x8000_0000));
}

fn segment(t: &mut Twins, rng: &mut Rng) -> u32 {
    let seg = object(t, rng, 0x1b, false);
    let streams = t.zeros(0xc0);
    for k in 0..3 {
        stream(t, rng, streams + 0x40 * k);
    }
    t.w32(seg + 0x2c, streams);
    let count = if rng.chance(2) { 3 } else { rng.below(5) };
    t.w32(seg + 0x28, count);
    seg
}

/// A float argument: ordinary values, 0, 1, 0.5, negatives, a large
/// value, a denormal, a quiet NaN (no signalling NaNs).
fn coordinate(rng: &mut Rng) -> u32 {
    match rng.below(10) {
        0 => 0,
        1 => 1.0f32.to_bits(),
        2 => 0.5f32.to_bits(),
        3 => (-(rng.below(1000) as f32) / 999.0).to_bits(),
        4 => 1.0e30f32.to_bits(),
        5 => rng.below(0x7f_ffff) + 1,
        6 => 0x7fc0_0000 | rng.below(0x1000),
        _ => ((rng.next() >> 8) as f32 / 16_777_216.0).to_bits(),
    }
}

/// `run_framed` without the argument slot comparison: the original
/// stores temporaries and the instance into its own argument slots
/// (`ebp + 0xc`, `ebp + 0x10`), which its one caller (`0x006d1b40`)
/// never reads (check-argslots); the shim's slots are its own.
fn run(t: &mut Twins, what: &str, cluster: u32, u: u32, v: u32) {
    let args = [cluster, u, v];
    let top = t.a.stack_range().end - 0x100;
    let ebp = top - 4 * args.len() as u32 - 8;
    let lo = ebp - 0x38;
    let junk: Vec<u8> = (0..0x38 + 8 + 12).map(|k| (k * 37 + 11) as u8).collect();
    t.write(lo, &junk);
    t.reset_log();
    let addr = t.rebase(0x00a0_6e60);
    let ra =
        t.a.call(addr, Conv::Cdecl, &args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
    t.b.set_call_top(Some(lo - 0x100));
    let rb = qd::bridge::split(&mut G(&mut t.b), cluster, lo, u, v);
    t.b.set_call_top(None);
    assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
    assert_eq!(ra.eax, rb, "{what}: eax");
    // the map slots
    let fa = Oracle::read(&mut t.a, lo, 0x18).unwrap();
    let fb = Oracle::read(&mut t.b, lo, 0x18).unwrap();
    assert_eq!(fa, fb, "{what}: the frame");
    // the argument slots: put the original's back before the diff
    let slots = Oracle::read(&mut t.a, ebp + 8, 12).unwrap();
    t.b.write(ebp + 8, &slots).unwrap();
    t.diff(what);
}

#[test]
fn split_mesh_matches_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_6e60);
        let create = t.seq_stub(qd::CLUSTER_CREATE, true, 3, 3);
        let list = t.seq_stub(qd::ALLOCATE_INSTANCE_LIST, true, 3, 3);
        let malloc = t.seq_stub(qd::MALLOC_ALIGNED, false, 2, 0);
        let bound = t.seq_stub(qd::IS_BOUND, true, 0, 0);
        let unmap = t.seq_stub(qd::UNMAP, true, 2, 2);
        t.stub(qd::SEGMENTS_CONSTRUCT, false, 2, 0, 0x5e90_0001);
        t.stub(qd::SEGMENTS_DESTROY, false, 2, 0, 0x5e90_0002);
        t.stub(qd::STREAM_CTOR, true, 0, 0, 0x5e90_0003);
        t.stub(qd::STREAMS_SET, true, 2, 2, 0x5e90_0004);
        t.stub(qd::STREAM_FORMAT, true, 3, 3, 0x5e90_0005);
        t.stub(qd::RELEASE, true, 0, 0, 0x5e90_0006);
        t.stub(qd::INSTANCE_CTOR, true, 1, 1, 0x5e90_0007);
        let mut c = t.log_code(qd::MAP, true, 1);
        c.extend([0x8b, 0x41, 0x3c]); // mov eax, [ecx+0x3c]
        c.extend([0x89, 0x02]); // mov [edx], eax
        c.extend([0xc2, 0x04, 0x00]); // ret 4
        let map = t.code(&c);
        t.jump(qd::MAP, map);
        for i in 0..64u32 {
            let cluster = object(&mut t, &mut rng, 4, false);
            let mesh = object(&mut t, &mut rng, 2, false);
            let old = segment(&mut t, &mut rng);
            let fresh = segment(&mut t, &mut rng);
            t.w32(mesh + 4, old);
            let mut mallocs = vec![];
            match i % 4 {
                0 => t.w32(mesh, 1),
                1 => {
                    t.w32(mesh, rng.below(4) | (rng.next() & 0x8000_0000));
                    mallocs.push(fresh);
                }
                2 => {
                    t.w32(mesh, 0x8000_0001);
                    mallocs.push(0);
                }
                _ => {
                    t.w32(mesh, rng.below(3));
                    t.w32(mesh + 4, fresh);
                    mallocs.push(fresh);
                }
            }
            let null = rng.chance(3);
            let block = object(&mut t, &mut rng, 0x30, null);
            mallocs.push(block);
            t.seq_set(malloc, &mallocs);
            t.seq_set(create, &[mesh]);
            let bounds: Vec<u32> = (0..3)
                .map(|_| (rng.next() & 0xffff_ff00) | u32::from(rng.chance(2)))
                .collect();
            t.seq_set(bound, &bounds);
            let unmaps: Vec<u32> = (0..3)
                .map(|_| if rng.chance(2) { 0 } else { rng.next() })
                .collect();
            t.seq_set(unmap, &unmaps);
            let lst = match i % 3 {
                0 => 0,
                _ => {
                    let l = object(&mut t, &mut rng, 0x10, false);
                    if i % 6 == 1 {
                        t.w32(l + 0x34, 0);
                    }
                    l
                }
            };
            t.seq_set(list, &[lst]);
            t.a.clear_dirty();
            t.b.clear_dirty();
            let (u, v) = (coordinate(&mut rng), coordinate(&mut rng));
            run(&mut t, &format!("{name} split #{i}"), cluster, u, v);
            assert!(t.log_entries() >= 11, "{name} #{i}: the calls");
        }
    }
}
