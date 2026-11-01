//! `magic_0641.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::part_draw::{Camera, camera};
use crate::runtime::slot_target;
use crate::sb_sse::sub;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Clears the effect's pool (0x10002e60, internal, no arguments): the
/// 0x5c-byte header at operand `+0x11` zeroed (`memset`), its count
/// `+0x1f` 100, its first `+0x29` the pool `*G_P` (`+0x07`), `+0x2f`,
/// `+0x39`, `+0x43` 0; then the pool's 100 entries of 0x94 bytes
/// zeroed. Returns the last entry (`memset`'s `eax`).
pub fn clear_pool<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let mut p = g.read::<u32>(w(0x7));
    g.write_bytes(w(0x11), &[0u8; 0x5c]);
    g.write::<u32>(w(0x1f), 100);
    g.write::<u32>(w(0x29), p);
    for op in [0x2f, 0x39, 0x43] {
        g.write::<u32>(w(op), 0);
    }
    let mut last = 0;
    for _ in 0..100 {
        g.write_bytes(p, &[0u8; 0x94]);
        last = p;
        g.write::<u32>(p, 0);
        p = p.wrapping_add(0x94);
    }
    last
}

/// [`clear_pool`] (117 bytes, 1 overlay).
pub const CLEAR_POOL: Sig = Sig {
    len: 117,
    hash: 0xf94d_0259_f407_9b6e,
    calls: &[
        (0x16, Callee::Import("memset")),
        (0x59, Callee::Import("memset")),
    ],
    imports: &[],
};

/// Moves the nine request words to the current ones (0x10001490,
/// internal, no arguments): `CUR[k] = REQ[k]` (the operand pairs
/// `+0x01`/`+0x06` ... `+0x51`/`+0xa6`), the requests cleared. Returns
/// the last request word.
pub fn take_requests<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    for (src, dst) in [
        (0x01, 0x06),
        (0x0b, 0x10),
        (0x15, 0x1a),
        (0x1f, 0x24),
        (0x29, 0x2e),
        (0x33, 0x38),
        (0x3d, 0x42),
        (0x47, 0x4c),
    ] {
        let v = g.read::<u32>(w(src));
        g.write::<u32>(w(dst), v);
    }
    let last = g.read::<u32>(w(0x51));
    for op in [0x57, 0x61, 0x6b, 0x75, 0x7f, 0x89, 0x93, 0x9d] {
        g.write::<u32>(w(op), 0);
    }
    g.write::<u32>(w(0xa6), last);
    g.write::<u32>(w(0xac), 0);
    last
}

/// [`take_requests`] (181 bytes, 1 overlay).
pub const TAKE_REQUESTS: Sig = Sig {
    len: 181,
    hash: 0x0e2b_e19f_f323_81e6,
    calls: &[],
    imports: &[],
};

/// `lahf` after `ucomiss a, b`: `ZF`, `PF`, `CF` from the compare, bit 1
/// set.
fn lahf(a: f32, b: f32) -> u32 {
    match a.partial_cmp(&b) {
        None => 0x47,
        Some(core::cmp::Ordering::Less) => 0x03,
        Some(core::cmp::Ordering::Equal) => 0x42,
        Some(core::cmp::Ordering::Greater) => 0x02,
    }
}

/// Steps the nine flashes (0x10001550, internal: `ecx` = whether to
/// start the timers): per flash `k` (the arrays at the operands
/// `+0x17` request, `+0x22` on, `+0x31` level, `+0x41` current, `+0x4a`
/// hold, `+0x58` timer on, `+0x62` timer): a request 1 while off turns it
/// on at level 1.0; no request with current 1 holds it (hold 1.0; with
/// `ecx`, the timer starts at 24). While the hold is exactly `H`
/// (`+0x04`) the level drops by `S` (`+0x0c`), clamped to 0..`H`; at 0
/// the flash goes off (on and hold 0). A running timer counts down and
/// stops at 0. Returns the last request word with the last compare's
/// `lahf` byte in bits 8..15.
pub fn step_flashes<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let hold_k = g.read::<f32>(w(0x4));
    let step = g.read::<f32>(w(0xc));
    let at = |op: u32, k: u32| w(op).wrapping_add(k);
    let mut eax = 0u32;
    let mut k = 0u32;
    while k < 0x24 {
        let req = g.read::<u32>(at(0x17, k));
        eax = req;
        if req == 1 && g.read::<u32>(at(0x22, k)) == 0 {
            g.write::<u32>(at(0x2b, k), 1);
            g.write::<u32>(at(0x31, k), ONE);
        } else if req == 0 && g.read::<u32>(at(0x41, k)) == 1 {
            g.write::<u32>(at(0x4a, k), ONE);
            if fr.ecx != 0 {
                g.write::<u32>(at(0x58, k), 1);
                g.write::<u32>(at(0x62, k), 0x18);
            }
        }
        let hold = g.read::<f32>(at(0x6e, k));
        let ah = lahf(hold, hold_k);
        eax = eax & !0xff00 | ah << 8;
        if ah == 0x42 {
            let mut y = sub(g.read::<f32>(at(0x7f, k)), step);
            g.write::<f32>(at(0x8e, k), y);
            // comiss 0, y; jbe: below, equal or unordered
            if 0.0 > y {
                y = 0.0;
            } else if y > hold_k {
                y = hold_k;
            }
            let ah = lahf(y, 0.0);
            g.write::<f32>(at(0xa8, k), y);
            eax = eax & !0xff00 | ah << 8;
            if ah == 0x42 {
                g.write::<u32>(at(0xb4, k), 0);
                g.write::<u32>(at(0xbe, k), 0);
            }
        }
        if g.read::<u32>(at(0xc8, k)) != 0 {
            let n = g.read::<u32>(at(0xd1, k)).wrapping_sub(1);
            g.write::<u32>(at(0xd1, k), n);
            if n == 0 {
                g.write::<u32>(at(0xd9, k), 0);
            }
        }
        k += 4;
    }
    eax
}

/// [`step_flashes`] (238 bytes, 1 overlay).
pub const STEP_FLASHES: Sig = Sig {
    len: 238,
    hash: 0x970a_bf91_92a0_291e,
    calls: &[],
    imports: &[],
};

/// Starts a part with its defaults (0x10001640, internal: `ecx` = the
/// object): [`start_part`](crate::part_draw::start_part_eax) with
/// program 0 (`*G_P = ITEM` `+0x01`/`+0x08`, `G_O` `+0x0d`, slot 941
/// `+0x17`); `+0x40..+0x48` 0, `+0x4c` 1.0, `+0x50..+0x58` 0, `+0x5c`
/// 1, `+0x60..+0x6c` 1.0. Returns the wrapper's `eax`.
pub fn part_defaults<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let obj = fr.ecx;
    let (_, eax) =
        crate::part_draw::start_part_eax(g, img, fr.at, obj, [0x1, 0x8, 0xd, 0x17, 0x31], 0, &[]);
    for (off, v) in [
        (0x40, 0),
        (0x44, 0),
        (0x48, 0),
        (0x4c, ONE),
        (0x50, 0),
        (0x54, 0),
        (0x58, 0),
        (0x5c, 1),
        (0x60, ONE),
        (0x64, ONE),
        (0x68, ONE),
        (0x6c, ONE),
    ] {
        g.write::<u32>(obj + off, v);
    }
    eax
}

/// [`part_defaults`] (142 bytes, 1 overlay).
pub const PART_DEFAULTS: Sig = Sig {
    len: 142,
    hash: 0x926d_f9c0_d265_0a0f,
    calls: &[(0x31, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// Nine parts of 0x80 bytes from `base` started with program
/// `index / 4` ([`start_part_eax`](crate::part_draw::start_part_eax):
/// `*G_P = ITEM` `+0x11`/`+0x17`, `G_O` `+0x1c`, slot 941 `+0x24`, the
/// wrapper's `rel32` at `call`), each given the defaults of
/// [`part_defaults`]. Returns the last wrapper's `eax`.
fn nine_parts<G: Game>(g: &mut G, img: &Image, f: u32, base: u32, index: u32, call: u32) -> u32 {
    let mut eax = 0;
    for k in 0..9 {
        let obj = base + 0x80 * k;
        eax = crate::part_draw::start_part_eax(
            g,
            img,
            f,
            obj,
            [0x11, 0x17, 0x1c, 0x24, call],
            index,
            &[],
        )
        .1;
        for (off, v) in [
            (0x40, 0),
            (0x44, 0),
            (0x48, 0),
            (0x4c, ONE),
            (0x50, 0),
            (0x54, 0),
            (0x58, 0),
            (0x5c, 1),
            (0x60, ONE),
            (0x64, ONE),
            (0x68, ONE),
            (0x6c, ONE),
        ] {
            g.write::<u32>(obj + off, v);
        }
    }
    eax
}

/// Starts the nine flash parts (0x10001870, an overlay table entry,
/// `(base)`): [`nine_parts`] with program 0. Returns the last wrapper's
/// `eax`.
pub fn flash_parts<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let base = fr.arg(g, 0);
    Ret::eax(nine_parts(g, img, fr.at, base, 0, 0x3f))
}

/// [`flash_parts`] (170 bytes, 1 overlay).
pub const FLASH_PARTS: Sig = Sig {
    len: 170,
    hash: 0x3fcf_ae77_c1df_3b52,
    calls: &[(0x3f, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// Starts the nine glow parts (0x100019b0, an overlay table entry,
/// `(base)`): [`nine_parts`] with program 1. Returns the last wrapper's
/// `eax`.
pub fn glow_parts<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let base = fr.arg(g, 0);
    Ret::eax(nine_parts(g, img, fr.at, base, 4, 0x40))
}

/// [`glow_parts`] (171 bytes, 1 overlay).
pub const GLOW_PARTS: Sig = Sig {
    len: 171,
    hash: 0x0830_272b_5cd9_6a91,
    calls: &[(0x40, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// The effect's frame (0x10001f20, an overlay table entry, no
/// arguments): nothing while the DLL's flag (operand `+0x02`) is 0
/// (returns 0); else the camera matrices ([`camera`]), a word copied
/// through a pointer global, the DLL's frame word kept; with slot 261
/// `(13)` set, timeline A stops
/// ([`timeline_stop`](crate::sb_misc::timeline_stop)) and the effect
/// ends (slots 376 `(*G_S)`, 381 `(0)`, 214 `(*G_H)`); else timeline B
/// runs ([`timeline_run`](crate::sb_misc::timeline_run)) with slot 261
/// `(11)` set, and with slot 261 `(12)` set both stop and the effect
/// ends; otherwise the frame counter goes up. Returns 1.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    if g.read::<u32>(w(0x2)) == 0 {
        return Ret::eax(0);
    }
    let c = Camera {
        gp: 0xd,
        cam: 0x13,
        item: 0x19,
        t723: 0x1e,
        t724: 0x2b,
        m: 0x31,
    };
    let mut stack = camera(g, img, f, &c, &[]);
    let v = g.read::<u32>(w(0x3f));
    let x = g.read::<u32>(w(0x44));
    g.write::<u32>(v, x);
    let k = g.read::<u32>(w(0x4b));
    g.write::<u32>(w(0x50), k);
    let t = slot_target(g, w(0x55), 261);
    stack.insert(0, 0xd);
    let stop = |g: &mut G, tl: u32| crate::sb_misc::timeline_stop(g, tl);
    // call width: 18 at most (the earlier calls' words)
    if g.call(t, 0, &stack).eax == 0 {
        let t = slot_target(g, w(0xb3), 261);
        if g.call(t, 0, &[0xb]).eax != 0 {
            crate::sb_misc::timeline_run(g, img, img.rel32_target(f + 0xce), w(0xc9));
        }
        let t = slot_target(g, w(0xd3), 261);
        if g.call(t, 0, &[0xc]).eax == 0 {
            let n = g.read::<u32>(w(0xfe));
            g.write::<u32>(w(0xfe), n.wrapping_add(1));
            return Ret::eax(1);
        }
        stop(g, w(0xe9));
        stop(g, w(0xf3));
    } else {
        stop(g, w(0x6b));
    }
    let t = slot_target(g, w(0x75), 376);
    let a = g.read::<u32>(w(0x7b));
    let mut stack = vec![a];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x88), 381);
    stack.insert(0, 0);
    // call width: 18 at most (the earlier calls' words)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x97), 214);
    let h = g.read::<u32>(w(0x9d));
    stack.insert(0, h);
    // call width: 18 at most (the earlier calls' words)
    g.call(t, 0, &stack);
    Ret::eax(1)
}

/// [`frame`] (264 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 264,
    hash: 0xbcb2_4bcb_f23f_7a11,
    calls: &[
        (0x70, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
        (0xce, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xee, Callee::Sig(&crate::sb_misc::TIMELINE_STOP)),
    ],
    imports: &[],
};

/// Starts the effect's state (0x10001280, an overlay table entry, no
/// arguments): slot 260 `(20)` into `+0x10`, slot 719 `(*G_NAME, 4, 0)`
/// into `+0x35`; the 0x30-byte block `B` (`+0x30`) zeroed (`memset`),
/// its half-word `+2` 0x80 and `+4` 0x80808080; the pool cleared
/// ([`clear_pool`], slot 260's 20 under it), three globals (`+0x60`,
/// `+0x6a`, `+0x74`) 0. Returns 1.
pub fn state_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 260);
    let v = g.call(t, 0, &[0x14]).eax;
    g.write::<u32>(w(0x10), v);
    let t = slot_target(g, w(0x15), 719);
    let name = g.read::<u32>(w(0x25));
    let d = g.call(t, 0, &[name, 4, 0, 0x14]).eax;
    g.write::<u32>(w(0x35), d);
    g.write_bytes(w(0x30), &[0u8; 0x30]);
    g.write::<u16>(w(0x48), 0x80);
    g.write::<u32>(w(0x4e), 0x8080_8080);
    let clear = Frame {
        at: img.rel32_target(f + 0x57),
        esp: fr.esp.wrapping_sub(8),
        ..*fr
    };
    clear_pool(g, img, &clear);
    for op in [0x60, 0x6a, 0x74] {
        g.write::<u32>(w(op), 0);
    }
    Ret::eax(1)
}

/// [`state_start`] (130 bytes, 1 overlay).
pub const STATE_START: Sig = Sig {
    len: 130,
    hash: 0x093c_4c7f_98f5_32b7,
    calls: &[
        (0x3a, Callee::Import("memset")),
        (0x57, Callee::Sig(&CLEAR_POOL)),
    ],
    imports: &[],
};

/// The flashes' per-frame step (0x100013a0, an overlay table entry, no
/// arguments): nothing (0) unless slot 261 `(9)` says so. Slot 315
/// `(0)`; when slot 261 `(10)` answers, the requests are taken
/// ([`take_requests`]) and the flashes step with `ecx` 0
/// ([`step_flashes`]); otherwise the nine words at `+0x56`.. are copied
/// to `+0x5b`.., each request `R[i]` (`+0xcc`) becomes whether slot 261
/// `(i)` answers, and the flashes step with `ecx` 1. Either way the
/// timeline `+0x4c`/`+0xe2` runs last
/// ([`timeline_run`](crate::sb_misc::timeline_run), a tail jump), whose
/// answer it returns.
pub fn flash_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = slot_target(g, w(0x1), 261);
    if g.call(t, 0, &[9]).eax == 0 {
        return Ret::eax(0);
    }
    let t = slot_target(g, w(0x1b), 315);
    g.call(t, 0, &[0]);
    let t = slot_target(g, w(0x2a), 261);
    let (step_call, tl_call, tl) = if g.call(t, 0, &[0xa, 0]).eax != 0 {
        let take = Frame {
            at: img.rel32_target(f + 0x40),
            esp: fr.esp.wrapping_sub(4),
            ..*fr
        };
        take_requests(g, img, &take);
        (0x47, 0x51, w(0x4c))
    } else {
        for (from, to) in [
            (0x56, 0x5b),
            (0x60, 0x65),
            (0x6a, 0x6f),
            (0x74, 0x79),
            (0x7e, 0x83),
            (0x88, 0x8d),
            (0x92, 0x97),
            (0x9c, 0xa1),
            (0xa6, 0xac),
        ] {
            let v = g.read::<u32>(w(from));
            g.write::<u32>(w(to), v);
        }
        for i in 0..9u32 {
            let t = slot_target(g, w(0xb3), 261);
            let v = g.call(t, 0, &[i]).eax;
            g.write::<u32>(w(0xcc).wrapping_add(4 * i), u32::from(v != 0));
        }
        (0xdc, 0xe7, w(0xe2))
    };
    let ecx = u32::from(step_call == 0xdc);
    // push esi on the second way; the return address
    let flashes = Frame {
        at: img.rel32_target(f + step_call),
        esp: fr.esp.wrapping_sub(4 + 4 * ecx),
        ecx,
        ..*fr
    };
    step_flashes(g, img, &flashes);
    Ret::eax(crate::sb_misc::timeline_run(
        g,
        img,
        img.rel32_target(f + tl_call),
        tl,
    ))
}

/// [`flash_step`] (236 bytes, 1 overlay).
pub const FLASH_STEP: Sig = Sig {
    len: 236,
    hash: 0xc487_dbc8_973c_5efc,
    calls: &[
        (0x40, Callee::Sig(&TAKE_REQUESTS)),
        (0x47, Callee::Sig(&STEP_FLASHES)),
        (0x51, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
        (0xdc, Callee::Sig(&STEP_FLASHES)),
        (0xe7, Callee::Sig(&crate::sb_misc::TIMELINE_RUN)),
    ],
    imports: &[],
};
