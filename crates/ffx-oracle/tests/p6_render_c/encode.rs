//! Continuation round, batch 10: `PRenderer`'s command encoders
//! (`ffx_prender::p6_encode`) against their originals.
//!
//! The rings' check writes through a pointer to the original's locals:
//! the port gets its frame at the original's `ebp - 8` and its calls below
//! it (`Oracle::set_call_top`), so both log the same pointers, and both
//! frames are compared after the call.

use ffx_oracle::Conv;
use ffx_prender::p6_encode as enc;

use super::{G, Rng, Twins, object};

/// The ring check's stand-in (thiscall, three words, `ret 0xc`): logs
/// `ecx` and its words, then for its `k`-th call (counter at `tab`)
/// stores `tab[1 + 2k]` through its third word and returns `tab[2 + 2k]`.
pub(super) fn ring_check_stub(t: &mut Twins, tab: u32) -> u32 {
    let log = t.log;
    let mut c = vec![0xa1];
    c.extend(log.to_le_bytes()); // mov eax, [log]
    c.extend([0xc1, 0xe0, 0x05]); // shl eax, 5
    c.push(0x05);
    c.extend((log + 4).to_le_bytes()); // add eax, log + 4
    c.extend([0xc7, 0x00]);
    c.extend(enc::RING_CHECK.to_le_bytes()); // mov [eax], tag
    c.extend([0x89, 0x48, 0x04]); // mov [eax+4], ecx
    for i in 0..3u8 {
        c.extend([0x8b, 0x54, 0x24, 4 + 4 * i]); // mov edx, [esp+4+4i]
        c.extend([0x89, 0x50, 8 + 4 * i]); // mov [eax+8+4i], edx
    }
    c.extend([0xff, 0x05]);
    c.extend(log.to_le_bytes()); // inc dword [log]
    c.push(0xa1);
    c.extend(tab.to_le_bytes()); // mov eax, [tab]
    c.extend([0xff, 0x05]);
    c.extend(tab.to_le_bytes()); // inc dword [tab]
    c.extend([0x8b, 0x14, 0xc5]);
    c.extend((tab + 4).to_le_bytes()); // mov edx, [tab+4+eax*8]
    c.extend([0x8b, 0x4c, 0x24, 0x0c]); // mov ecx, [esp+0xc]
    c.extend([0x89, 0x11]); // mov [ecx], edx
    c.extend([0x8b, 0x04, 0xc5]);
    c.extend((tab + 8).to_le_bytes()); // mov eax, [tab+8+eax*8]
    c.extend([0xc2, 0x0c, 0x00]); // ret 0xc
    t.code(&c)
}

/// A renderer: vtable slot 2 (`submitFrame`) a logged stand-in; its
/// rings' cursors and ends, `+0x4c` empty or not, the scene bit random.
pub(super) fn renderer(t: &mut Twins, rng: &mut Rng, submit: u32) -> u32 {
    let r = object(t, rng, 0x18, false);
    let vtable = t.zeros(0x10);
    t.w32(vtable + 8, submit);
    t.w32(r, vtable);
    for (cur, end) in [(0x24, 0x28), (0x38, 0x3c)] {
        let base = 0x1000_0000 + (rng.below(16) << 8);
        let c = base + rng.below(0x200);
        let e = if rng.chance(4) {
            c.wrapping_sub(rng.below(0x40))
        } else {
            c + rng.below(0x200)
        };
        t.w32(r + cur, c);
        t.w32(r + end, e);
    }
    if rng.chance(2) {
        t.w32(r + 0x4c, 0);
    }
    r
}

/// The ring check's answers for one call: (wrapped cursor, new cursor)
/// around the ring's cursor and end, so that every branch is taken.
pub(super) fn answers(t: &mut Twins, rng: &mut Rng, r: u32, tab: u32) {
    t.w32(tab, 0);
    for (k, (cur, end)) in [(0x24, 0x28), (0x38, 0x3c)].into_iter().enumerate() {
        let c: u32 = super::Mem::read(&t.b, r + cur);
        let e: u32 = super::Mem::read(&t.b, r + end);
        let o = match rng.below(3) {
            0 => c.wrapping_sub(1 + rng.below(8)),
            1 => c,
            _ => c + rng.below(0x100),
        };
        let n = match rng.below(3) {
            0 => e,
            1 => e.wrapping_add(1),
            _ => e.wrapping_sub(rng.below(0x100)),
        };
        t.w32(tab + 4 + 8 * k as u32, o);
        t.w32(tab + 8 + 8 * k as u32, n);
    }
}

/// Runs the original at `f` on `a` (thiscall `r`, `args`) and `port` on
/// `b` with its frame at the original's `ebp - 8`; compares `eax`, the
/// frames and every byte written.
fn run(
    t: &mut Twins,
    what: &str,
    f: u32,
    r: u32,
    args: &[u32],
    port: impl FnOnce(&mut G, u32) -> u32,
) {
    // `Oracle::call` pushes the words and the return address below the
    // top; the original then pushes `ebp`
    let top = t.a.stack_range().end - 0x100;
    let ebp = top - 4 * args.len() as u32 - 8;
    let lo = ebp - 8;
    let junk = [0x5a, 0x11, 0xc3, 0x7e, 0x99, 0x02, 0x40, 0xee];
    t.write(lo, &junk);
    let log = t.log;
    t.w32(log, 0);
    let addr = t.rebase(f);
    let ra =
        t.a.call(addr, Conv::Thiscall(r), args)
            .unwrap_or_else(|e| panic!("{what}: the original: {e}"));
    t.b.set_call_top(Some(lo - 0x100));
    let rb = port(&mut G(&mut t.b), lo);
    t.b.set_call_top(None);
    assert!(ra.st0_raw.is_none(), "{what}: the original leaves ST0 set");
    assert_eq!(ra.eax, rb, "{what}: eax");
    let fa = ffx_oracle::Oracle::read(&mut t.a, lo, 8).unwrap();
    let fb = ffx_oracle::Oracle::read(&mut t.b, lo, 8).unwrap();
    assert_eq!(fa, fb, "{what}: the frame");
    t.diff(what);
}

/// A float argument: ordinary, zero, negative, infinite, a quiet NaN or
/// a denormal (signalling NaNs stay out: the emulator keeps them).
pub(super) fn float_word(rng: &mut Rng) -> u32 {
    match rng.below(6) {
        0 => 0,
        1 => 0x8000_0000,
        2 => 0x7f80_0000,
        3 => 0x7fc0_1234,
        4 => 1 + rng.below(0x7f_ffff),
        _ => (rng.next() & 0xbfff_ffff) | 0x3000_0000,
    }
}

#[test]
fn encoders_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_000b);
        let tab = t.zeros(0x40);
        let check = ring_check_stub(&mut t, tab);
        t.jump(enc::RING_CHECK, check);
        let submit = t.stub_code(0x005b_6c50, true, 0, 0, 0x77);
        let mut cases = 0;
        for i in 0..60 {
            let cmd = if i % 7 == 6 { 0 } else { t.zeros(0x48) };
            t.stub(enc::RING_ALLOCATE, true, 1, 1, cmd);
            for &(f, opcode, payload) in enc::ENCODERS {
                let r = renderer(&mut t, &mut rng, submit);
                answers(&mut t, &mut rng, r, tab);
                let args: Vec<u32> = (0..payload.words())
                    .map(|_| match payload {
                        enc::Payload::Floats(_) => float_word(&mut rng),
                        _ => rng.next(),
                    })
                    .collect();
                run(
                    &mut t,
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    r,
                    &args,
                    |g, frame| enc::bridge::encode(g, r, frame, opcode, payload, &args),
                );
                cases += 1;
            }
            for &(f, opcode, rule) in enc::SCENE {
                let r = renderer(&mut t, &mut rng, submit);
                answers(&mut t, &mut rng, r, tab);
                let w = rng.next();
                run(
                    &mut t,
                    &format!("{name} {f:#x} #{i}"),
                    f,
                    r,
                    &[w],
                    |g, frame| enc::bridge::encode_scene(g, r, frame, opcode, rule, w),
                );
            }
            let r = renderer(&mut t, &mut rng, submit);
            answers(&mut t, &mut rng, r, tab);
            let cb = if i % 5 == 0 { 0 } else { rng.next() };
            let data = rng.next();
            run(
                &mut t,
                &format!("{name} queueCallback #{i}"),
                enc::QUEUE_CALLBACK,
                r,
                &[cb, data],
                |g, frame| enc::bridge::queue_callback(g, r, frame, cb, data),
            );
            t.unstub(enc::RING_ALLOCATE);
        }
        t.unstub(enc::RING_CHECK);
        assert_eq!(cases, 60 * enc::ENCODERS.len());
    }
}
