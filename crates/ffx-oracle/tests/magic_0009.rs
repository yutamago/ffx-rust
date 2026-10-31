//! `magic_0009.dll` against its Rust port (`ffx_magic::m0009`): `DllMain`
//! against every bound port, the effect set-up and the model fill (both
//! internal). Needs `FFX_GAME_DIR`.

mod magic;
mod magic_l3;
mod magic_owna;

use ffx_magic::{Ret, m0009};
use magic::{Call, Eax, SCRATCH};
use magic_owna::{f, rec, recs, twin};

const N: u16 = 9;

#[test]
fn dll_main_writes_only_crt_state() {
    magic_l3::dll_main(N);
}

#[test]
fn setup() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x19d0);
    let fx = SCRATCH + 0x1000;
    t.set_u32(t.img.word(at + 0x11), fx);
    rec(&mut t, 719, 6, |_, a| 0x0719_0000 + a[2] * 0x10 + a[1]);
    recs(&mut t, &[(235, 1), (330, 3), (328, 3), (329, 3)]);
    let (pos, from, rot) = (SCRATCH + 0x100, SCRATCH + 0x110, SCRATCH + 0x120);
    for (p, q) in [
        ([1.0f32, 2.0, 3.0, 1.0], [0.5f32, -1.0, 7.0]),
        ([-4.5, 0.25, 10.0, 0.0], [-4.5, 0.25, 10.0]),
        ([100.0, -3.0, 0.125, 2.0], [1e-3, 8.0, -6.0]),
    ] {
        for (i, v) in (0u32..).zip(p) {
            t.set_u32(pos + 4 * i, f(v));
        }
        for (i, v) in (0u32..).zip(q) {
            t.set_u32(from + 4 * i, f(v));
            t.set_u32(rot + 4 * i, f(v * 0.5 + 0.1));
        }
        // the block dirty before
        t.write(fx, &[0xa5; 0xa00]);
        let c = Call {
            ecx: pos,
            edx: from,
            ..Call::default()
        };
        t.check(at, c, &[rot], Eax::Junk, |g, img, fr| {
            m0009::setup(g, img, fr.at, fr.ecx, fr.edx, fr.arg(g, 0));
            Ret::default()
        });
    }
}

#[test]
fn fill() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x2aa0);
    let lock = SCRATCH + 0x200;
    t.set_u32(t.img.word(at + 0x21), lock);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 + a[1]);
    // the flag during the call, recorded with the arguments
    rec(&mut t, 779, 4, move |m, a| {
        let v = m.r32(lock);
        m.w32(SCRATCH + 0x300 + 4 * (a[3] % 0x46), v ^ a[0]);
        0
    });
    for saved in [0u32, 7] {
        t.set_u32(lock, saved);
        let c = Call::this(SCRATCH + 0x800);
        t.check(at, c, &[], Eax::Junk, |g, img, fr| {
            m0009::fill(g, img, fr.at, fr.ecx);
            Ret::default()
        });
    }
}

/// 0x100032b0: a part object with no free pool node, as the first node
/// of an empty list and appended to a list.
#[test]
fn part_object() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x32b0);
    let w = |t: &magic::Twin, off: u32| t.img.word(at + off);
    let (obj, owner, table) = (SCRATCH + 0x1000, SCRATCH + 0x300, SCRATCH + 0x400);
    t.set_u32(w(&t, 0x04), SCRATCH + 0x200);
    t.set_u32(w(&t, 0x5e), owner);
    t.set_u32(owner + 0x10, 0x00a1_0010);
    for k in 0..4 {
        t.set_u32(table + 4 * k, 0x0f00_0000 + k);
    }
    rec(&mut t, 941, 1, move |_, _| table);
    recs(&mut t, &[(714, 5)]);
    let alloc = t.img.rel32_target(at + 0x9f);
    let (free, nodes) = (t.img.word(alloc + 0x04), t.img.word(alloc + 0x27) - 8);
    let tail = SCRATCH + 0x500;
    for (case, (n_free, has_tail, index)) in [(0u16, false, 0u32), (5, false, 1), (5, true, 3)]
        .into_iter()
        .enumerate()
    {
        t.write(obj, &[0x3c; 0x130]);
        t.set_u16(free, n_free);
        t.write(nodes, &[0x5d; 0x30]);
        for i in 0..4 {
            t.set_u32(nodes + 12 * i + 8, if i == 1 { 0x77 } else { 0 });
        }
        t.set_u32(w(&t, 0xaa), if has_tail { tail } else { 0 });
        t.write(tail, &[0x55; 0x10]);
        let scale = f([0.5f32, -2.0, 1.0e3][case]);
        let c = Call {
            edx: obj,
            ..Call::default()
        }
        .with_xmm(&[
            ffx_magic::Xmm::f32(9.0),
            ffx_magic::Xmm::f32(8.0),
            ffx_magic::Xmm::f32(7.0),
            ffx_magic::Xmm::f32(f32::from_bits(scale)),
        ]);
        t.check(
            at,
            c,
            &[index, 0x00b0_0000 + index],
            Eax::Checked,
            |g, img, fr| {
                Ret::eax(m0009::part_object(
                    g,
                    img,
                    fr.at,
                    fr.edx,
                    fr.xmm_f32(3).to_bits(),
                    fr.arg(g, 0),
                    fr.arg(g, 1),
                ))
            },
        );
    }
}

/// 0x10001930: the objects' frame with an object still running and with
/// none (the four records filled).
#[test]
fn objects_frame() {
    let Some(mut t) = twin(N) else { return };
    let at = t.at(0x1930);
    let ro = t.img.rel32_target(at + 0xb);
    let lock = SCRATCH + 0x200;
    t.set_u32(t.img.word(at + 0x56), lock);
    let recs_at = SCRATCH + 0x4000;
    t.set_u32(t.img.word(at + 0x19), recs_at - 0x70);
    for i in 0..4 {
        t.set_u32(recs_at + 0x250 * i, 0x00c0_0000 + i);
    }
    rec(&mut t, 941, 1, |_, a| 0x0941_0000 ^ a[0]);
    rec(&mut t, 772, 2, |_, a| 0x0772_0000 + a[1]);
    t.stub_edx(772, |_, a| 0x0edd_0000 ^ a[0]);
    // the flag during each call, folded into a running hash (the
    // records' own fills call the slot after the frame's calls)
    rec(&mut t, 779, 4, move |m, a| {
        let v = m.r32(lock);
        let h = m.r32(SCRATCH + 0x300).wrapping_mul(31);
        m.w32(SCRATCH + 0x300, h.wrapping_add(v ^ a[0]));
        0
    });
    let (task, record) = (SCRATCH + 0x100, SCRATCH + 0x180);
    let (node, obj) = (SCRATCH + 0x9f00, SCRATCH + 0xa000);
    let list = t.img.word(ro + 0x7c);
    for busy in [true, false] {
        t.write(task, &[0x11; 0x20]);
        t.set_u32(task + 8, record);
        t.write(record, &[0x22; 0x20]);
        t.set_u32(lock, 5);
        t.set_u32(node + 8, obj);
        t.set_u16(obj + 0x124, 0x8001);
        t.set_u32(list + 8, if busy { node } else { 0 });
        // run_objects starts the list from its head global
        t.set_u32(t.img.word(ro + 0x14), if busy { node } else { 0 });
        t.check_entry(at, Call::default(), &[task], Eax::Checked);
    }
}
