//! The overlay parity harness itself (`magic/mod.rs`): several DLLs on one
//! thread, `ecx` at call-outs, float arguments in `xmm0`..`xmm3` and the
//! `xmm0` result. Needs `FFX_GAME_DIR`.

mod magic;

use ffx_magic::{Ret, Xmm};
use magic::{Call, CallOut, Eax, TRAPS, Twin};

/// `Twin::load` again on the same thread: each load clears the heap that
/// held this thread's Unicorn thread block, so the harness starts a new
/// instance (`ffx_overlay::release_thread`); before, the second
/// `DllMain` faulted on a zeroed `fs`.
#[test]
fn loads_several_dlls_on_one_thread() {
    for n in [604u16, 3, 604, 87] {
        let Some(mut t) = Twin::load(n) else { return };
        t.init();
    }
}

fn call_out(slot: u32, ecx: u32) -> CallOut {
    CallOut {
        target: TRAPS + 16 * slot,
        slot: Some(slot),
        ecx,
        args: vec![1, 2, 3],
    }
}

/// `ecx` is compared where the slot's function reads it (a method: slot
/// 345, `Ch_ClutCreate`) and nowhere else.
#[test]
fn ecx_is_compared_for_methods() {
    let Some(t) = Twin::load(3) else { return };
    // a cdecl slot: ecx is junk
    t.compare_calls(0, &[call_out(12, 0x1234)], &[call_out(12, 0)]);
    t.compare_calls(0, &[call_out(345, 0x1234)], &[call_out(345, 0x1234)]);
    let wrong = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        t.compare_calls(0, &[call_out(345, 0x1234)], &[call_out(345, 0)]);
    }));
    assert!(wrong.is_err(), "a method's ecx is compared");
}

/// The float registers reach the original (and come back unchanged from
/// a lone `ret`) and the port's frame; an `xmm0` result is compared.
#[test]
fn xmm_arguments_and_result() {
    let Some(mut t) = Twin::load(3) else { return };
    // magic_0003 0x10001000: a lone `ret`
    let f = t.at(0x1000);
    let xs = [
        Xmm::f32(1.5),
        Xmm::f64(-2.25),
        Xmm::f32(f32::INFINITY),
        Xmm::f64(1.0e-300),
    ];
    let c = Call::default().with_xmm(&xs);
    t.check(f, c, &[], Eax::Junk, |_, _, fr| {
        assert_eq!(fr.xmm, xs.map(Xmm::reg), "the port's frame");
        assert_eq!(fr.xmm_f32(0), 1.5);
        assert_eq!(fr.xmm_f64(1), -2.25);
        // the original leaves xmm0 as it came
        Ret::xmm0(Xmm::f32(1.5), 0)
    });
    for (i, x) in [t.last.xmm0, t.last.xmm1, t.last.xmm2, t.last.xmm3]
        .iter()
        .enumerate()
    {
        assert_eq!(
            u128::from_le_bytes(*x),
            xs[i].reg(),
            "xmm{i} of the original"
        );
    }
    let wrong = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        t.check(f, c, &[], Eax::Junk, |_, _, _| Ret::xmm0(Xmm::f32(1.25), 0));
    }));
    assert!(wrong.is_err(), "a wrong xmm0 result fails");
}
