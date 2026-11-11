//! Callbacks the overlay-bind test found unbound (m5-wp1-bind): tiny
//! functions the DLLs store as callbacks (in objects, in node program
//! tables in `.data`), which the work list counted as CRT because their
//! mnemonic group matched one of the CRT's (`tools/magic-worklist.py`
//! `CRT_FROM`). Operands are read from each copy's code.

use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `mov dword [G], imm32; ret` (11 bytes): `G` (`+0x02`) gets the
/// constant (`+0x06`). `eax` as given. `magic_0249` 0x10001ed0 and
/// `magic_0562` 0x10001ee0 (stored at `[obj + 0x100]` by their start-up),
/// `magic_0443` 0x10001200, `magic_0530`/`magic_0531` 0x10001000 (node
/// program tables in `.data`).
pub fn set_global<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let v = img.word(fr.at + 0x06);
    g.write::<u32>(img.word(fr.at + 0x02), v);
    Ret::eax(fr.eax)
}

/// [`set_global`] with the constant 1 (`magic_0249`, `magic_0562`).
pub const SET_GLOBAL_1: Sig = Sig {
    len: 11,
    hash: 0x6ac4_9dd7_788b_95d1,
    calls: &[],
    imports: &[],
};

/// [`set_global`] with the constant 0 (`magic_0443`).
pub const SET_GLOBAL_0: Sig = Sig {
    len: 11,
    hash: 0x7774_7f01_f5dc_afce,
    calls: &[],
    imports: &[],
};

/// [`set_global`] with the constant 0x80 (`magic_0530`, `magic_0531`).
pub const SET_GLOBAL_80: Sig = Sig {
    len: 11,
    hash: 0xcee5_1440_9d90_2e4e,
    calls: &[],
    imports: &[],
};

/// `jmp rel32` (5 bytes): a tail jump to a ported function, which runs
/// here with the caller's stack as it is (the thunk's frame is the
/// target's). `magic_0165` 0x10002fe0 (to
/// [`glow_0165`](crate::r36::glow_0165)), `magic_0253` 0x10002510 and
/// `magic_0303` 0x10002550 (to
/// [`camera_rigs_step`](crate::s8_misc::camera_rigs_step)), stored as
/// task steps.
pub fn tail_thunk<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let at = img.rel32_target(fr.at + 1);
    match crate::bind(img, at).and_then(crate::entry::<G>) {
        Some(f) => f(g, img, &Frame { at, ..*fr }),
        None => Ret::jump(at),
    }
}

/// [`tail_thunk`] to [`glow_0165`](crate::r36::glow_0165).
pub const THUNK_GLOW_0165: Sig = Sig {
    len: 5,
    hash: 0x2ad2_8090_e3fa_4fe4,
    calls: &[(1, Callee::Sig(&crate::r36::GLOW_0165))],
    imports: &[],
};

/// [`tail_thunk`] to
/// [`camera_rigs_step`](crate::s8_misc::camera_rigs_step).
pub const THUNK_CAMERA_RIGS_STEP: Sig = Sig {
    len: 5,
    hash: 0x2ad2_8090_e3fa_4fe4,
    calls: &[(1, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP))],
    imports: &[],
};
