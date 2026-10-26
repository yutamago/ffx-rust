//! Continuation round, batch 10: the render thread's helpers
//! (`ffx_prender::p6_thread`) against their originals.

use ffx_oracle::Conv;
use ffx_prender::p6_thread as th;

use super::{G, Out, Rng, Twins, object};

/// An `InterlockedCompareExchange` stand-in (stdcall, three words): the
/// exchange done, except while the counter at `fails` is above 0: then
/// it counts down, sets bit 15 of the destination (another thread's
/// change) and returns the new value without exchanging.
fn compare_exchange_stub(t: &mut Twins, fails: u32) -> u32 {
    let mut c = vec![0xa1];
    c.extend(fails.to_le_bytes()); // mov eax, [fails]
    c.extend([0x85, 0xc0, 0x74, 0x15]); // test eax, eax; je real
    c.extend([0xff, 0x0d]);
    c.extend(fails.to_le_bytes()); // dec dword [fails]
    c.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp+4]
    c.extend([0x81, 0x09, 0x00, 0x80, 0x00, 0x00]); // or dword [ecx], 0x8000
    c.extend([0x8b, 0x01]); // mov eax, [ecx]
    c.extend([0xc2, 0x0c, 0x00]); // ret 0xc
    // real:
    c.extend([0x8b, 0x4c, 0x24, 0x04]); // mov ecx, [esp+4]
    c.extend([0x8b, 0x01]); // mov eax, [ecx]
    c.extend([0x3b, 0x44, 0x24, 0x0c]); // cmp eax, [esp+0xc]
    c.extend([0x75, 0x06]); // jne done
    c.extend([0x8b, 0x54, 0x24, 0x08]); // mov edx, [esp+8]
    c.extend([0x89, 0x11]); // mov [ecx], edx
    c.extend([0xc2, 0x0c, 0x00]); // done: ret 0xc
    t.code(&c)
}

#[test]
fn thread_helpers_match_the_original() {
    for (name, mut t) in Twins::all() {
        let mut rng = Rng(0x8a8a_000c);
        let fails = t.zeros(4);
        let cas = compare_exchange_stub(&mut t, fails);
        let slot = t.rebase(th::COMPARE_EXCHANGE);
        t.w32(slot, cas);
        t.stub(th::THREAD_CTOR, true, 0, 0, 0x55);
        t.stub(th::ATEXIT, false, 1, 0, 0);
        t.stub(th::EVENT_FLAG_INIT, true, 1, 1, 0x66);
        t.stub(th::EVENT_FLAG_RESET, true, 0, 0, 0x67);
        t.stub(th::SET_BLEND_FACTOR, true, 4, 4, 0);
        t.stub(th::SET_SCISSOR_RECT, true, 1, 1, 0x1234);
        t.stub(th::SET_VIEWPORT_SLOT, true, 2, 2, 0x4321);
        t.stub(th::STATE_APPLY, true, 1, 1, 0x8765_4321);
        for i in 0..40u32 {
            let log = t.log;
            t.w32(log, 0);
            // the singleton
            let guard = t.rebase(th::INSTANCE_GUARD);
            let g = rng.next() & !1 | u32::from(i % 2 == 1);
            t.w32(guard, g);
            t.check(
                &format!("{name} GetInstance #{i}"),
                0x0061_4410,
                Conv::Cdecl,
                &[],
                |g| Out::U32(th::bridge::get_instance(g)),
            );
            // the flag pool (4 flags): one of bits 0..3 free at least
            let thread = object(&mut t, &mut rng, 0x70, false);
            let taken = rng.next() & 0xf & !(1 << rng.below(4)) | (rng.next() & 0xffff_fff0);
            t.w32(thread + th::FLAG_MASK, taken);
            // flags set up before or not (their byte +4)
            for k in 0..4 {
                let b = if rng.chance(2) { 0 } else { 1 + rng.below(255) };
                t.write(thread + th::FLAGS + 8 * k + 4, &[b as u8]);
            }
            t.w32(fails, rng.below(3));
            t.w32(log, 0);
            t.check(
                &format!("{name} getEventFlag #{i}"),
                0x0061_4b70,
                Conv::Thiscall(thread),
                &[],
                |g| Out::U32(th::bridge::get_event_flag(g, thread)),
            );
            let flag = match i % 4 {
                0 => 0,
                1 => thread + th::FLAGS + 8 * rng.below(4) + rng.below(8),
                _ => thread + th::FLAGS + 8 * rng.below(4),
            };
            let taken = rng.next();
            t.w32(thread + th::FLAG_MASK, taken);
            t.w32(fails, rng.below(3));
            t.w32(log, 0);
            let addr = t.rebase(0x0061_5ac0);
            t.a.call(addr, Conv::Thiscall(thread), &[flag]).unwrap();
            th::bridge::release_event_flag(&mut G(&mut t.b), thread, flag);
            t.diff(&format!("{name} release flag #{i}"));
            // the state forwarders
            let state = object(&mut t, &mut rng, 0x20, false);
            let rgba = [
                rng.next() & 0xbfff_ffff,
                0x7fc0_0001,
                0,
                rng.next() & 0x807f_ffff,
            ];
            t.w32(log, 0);
            let addr = t.rebase(0x0061_5bc0);
            t.a.call(addr, Conv::Thiscall(state), &rgba).unwrap();
            th::bridge::set_blend_factor(&mut G(&mut t.b), state, rgba);
            t.diff(&format!("{name} blend factor #{i}"));
            let rect = object(&mut t, &mut rng, 4, i % 3 == 0);
            t.w32(log, 0);
            t.check(
                &format!("{name} scissor #{i}"),
                0x0061_5d00,
                Conv::Thiscall(state),
                &[rect],
                |g| Out::U32(th::bridge::set_scissor_rect(g, state, rect)),
            );
            let vp = object(&mut t, &mut rng, 4, i % 3 == 1);
            let k = rng.below(12);
            let state = object(&mut t, &mut rng, 0xe0, false);
            t.w32(log, 0);
            t.check(
                &format!("{name} viewport {k} #{i}"),
                0x0061_5c50,
                Conv::Thiscall(state),
                &[vp, k],
                |g| Out::U32(th::bridge::set_viewport_slot(g, state, vp, k)),
            );
            let busy = if i % 2 == 0 { 0 } else { rng.next() | 0x100 };
            t.stub(th::STATE_BUSY, false, 1, 0, busy);
            let a = rng.next();
            t.w32(log, 0);
            t.check(
                &format!("{name} apply state #{i}"),
                0x0061_47b0,
                Conv::Thiscall(state),
                &[a],
                |g| Out::U32(th::bridge::apply_state(g, state, a)),
            );
            t.unstub(th::STATE_BUSY);
        }
    }
}
