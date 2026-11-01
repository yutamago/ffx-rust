//! `magic_0396.dll`'s own functions (besides the runtime, the object
//! host and [`crate::ob_summon`], which has its facing and small
//! entries): the start, the parts' refresh, the cues
//! and an object's fade-out.

use crate::m0596::host_start;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Table `+0x0` (0x10004570, cdecl `(arg)`): the object host's start
/// ([`host_start`]), then the effect's stream cued (slot 551 `(slot 719
/// (E, 6, 5), 0, 1)`, the earlier calls' words on the stack) and
/// `G_DONE` = 0. Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    let mut stack = host_start(g, img, f, arg);
    let s719 = slot_target(g, w(0x10f), 719);
    stack.splice(0..0, [w(0x11e), 6, 5]);
    // call width: 17 at most (the earlier calls' words stay on the stack)
    let e = g.call(s719, 0, &stack).eax;
    let s551 = slot_target(g, w(0x12a), 551);
    stack.splice(0..0, [e, 0, 1]);
    // call width: 17 (the earlier calls' words stay on the stack)
    g.call(s551, 0, &stack);
    g.write::<u32>(w(0x13b), 0);
    Ret::eax(0)
}

/// [`start`] (327 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 327,
    hash: 0x9302_0298_b79c_26d7,
    calls: &[
        (0x0d, Callee::Sig(&crate::l5_objhost::OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&crate::l5_objhost::TASKS_INIT)),
    ],
    imports: &[],
};

/// Refreshes the effect's parts (0x10004c80, no arguments): the game
/// unpaused (slot 628 `(0)`); for each of the four part handles at
/// `G_PARTS` that is set, its 64 entries: slot 779 with what slot 772
/// `(handle, i)` returns in `edx:eax` (the game flag `*G_LOCK` set to 1
/// around it). Returns `eax` as the loop leaves it: the last handle when
/// it is 0, else the lock's address.
pub fn refresh<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let s628 = slot_target(g, w(0x0a), 628);
    g.call(s628, 0, &[0]);
    let (first, end) = (w(0x1c), w(0x89));
    let mut p = first;
    let mut eax;
    loop {
        let h = g.read::<u32>(p);
        eax = h;
        if h != 0 {
            for i in 0..0x40u32 {
                let s772 = slot_target(g, w(0x41), 772);
                let r = g.call(s772, 0, &[h, i]);
                let lock = g.read::<u32>(w(0x51));
                let saved = g.read::<u32>(lock);
                g.write::<u32>(lock, 1);
                let s779 = slot_target(g, w(0x5a), 779);
                g.call(s779, 0, &[r.eax, r.edx, h, i]);
                let lock = g.read::<u32>(w(0x6d));
                g.write::<u32>(lock, saved);
                eax = lock;
            }
        }
        p = p.wrapping_add(4);
        if p.cast_signed() >= end.cast_signed() {
            break;
        }
    }
    eax
}

/// [`refresh`] (150 bytes, 1 overlay).
pub const REFRESH: Sig = Sig {
    len: 150,
    hash: 0x2e7c_09d1_de22_e3e0,
    calls: &[],
    imports: &[],
};

/// A callback (0x10004d90, no arguments): the controller vibrates with
/// the effect's stream (slot 306 `(7, mode, slot 719 (E, 9, 0))`, mode 1
/// when `*G_ALT` is set, else 2) and the sound `k` (3, else 1) is cued
/// (slot 220 `(slot 719 (E, 6, k), 0x40, 0x100)`). Returns slot 220's
/// `eax`.
pub fn vibrate<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let alt = g.read::<u32>(w(0x02)) != 0;
    let s719 = slot_target(g, w(0x08), 719);
    let mut stack = vec![w(0x17), 9, 0];
    let e = g.call(s719, 0, &stack).eax;
    let (table, mode, k) = if alt {
        (w(0x21), 1, 3)
    } else {
        (w(0x3c), 2, 1)
    };
    let s306 = slot_target(g, table, 306);
    stack.splice(0..0, [7, mode, e]);
    g.call(s306, 0, &stack);
    let s719 = slot_target(g, w(0x56), 719);
    let mut stack = vec![w(0x63), 6, k];
    let e = g.call(s719, 0, &stack).eax;
    let s220 = slot_target(g, w(0x72), 220);
    stack.splice(0..0, [e, 0x40, 0x100]);
    Ret::eax(g.call(s220, 0, &stack).eax)
}

/// [`vibrate`] (130 bytes, 1 overlay).
pub const VIBRATE: Sig = Sig {
    len: 130,
    hash: 0xf168_15a7_ce7e_16c5,
    calls: &[],
    imports: &[],
};

/// A callback (0x10004e20, no arguments): the sound `k` (4 when `*G_ALT`
/// is set, else 2) cued (slot 220 `(slot 719 (E, 6, k), 0x40, 0x100)`).
/// Returns slot 220's `eax`.
pub fn cue<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = if g.read::<u32>(w(0x04)) != 0 { 4 } else { 2 };
    let s719 = slot_target(g, w(0x14), 719);
    let mut stack = vec![w(0x21), 6, k];
    let e = g.call(s719, 0, &stack).eax;
    let s220 = slot_target(g, w(0x30), 220);
    stack.splice(0..0, [e, 0x40, 0x100]);
    Ret::eax(g.call(s220, 0, &stack).eax)
}

/// [`cue`] (64 bytes, 1 overlay).
pub const CUE: Sig = Sig {
    len: 64,
    hash: 0x07e9_7d15_aae0_7fd3,
    calls: &[],
    imports: &[],
};

/// An object's fade-out step (0x10005100, cdecl `(obj)`; the callback
/// [`fade_start`] installs): the countdown `+0x120` drops, the colour
/// words `+0x70..+0x76` = `count * 4096 / 30`; at 0 the step `+0x104` is
/// cleared. Returns the colour.
pub fn fade<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let obj = fr.arg(g, 0);
    let n = g.read::<u32>(obj + 0x120).wrapping_sub(1);
    g.write::<u32>(obj + 0x120, n);
    let c = n.cast_signed().wrapping_shl(12) / 30;
    for off in [0x76, 0x74, 0x72, 0x70] {
        g.write::<u16>(obj + off, c.cast_unsigned() as u16);
    }
    if g.read::<u32>(obj + 0x120) == 0 {
        g.write::<u32>(obj + 0x104, 0);
    }
    Ret::eax(c.cast_unsigned())
}

/// [`fade`] (79 bytes, 1 overlay).
pub const FADE: Sig = Sig {
    len: 79,
    hash: 0xae71_b820_9eb1_3218,
    calls: &[],
    imports: &[],
};

/// Starts an object's fade-out (0x10005150, cdecl `(obj)`): its step
/// `+0x104` = [`fade`] (the operand at `+0x0c`), the countdown `+0x120`
/// = 30. Returns `obj`.
pub fn fade_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let obj = fr.arg(g, 0);
    g.write::<u32>(obj + 0x104, img.word(fr.at + 0x0c));
    g.write::<u32>(obj + 0x120, 0x1e);
    Ret::eax(obj)
}

/// [`fade_start`] (28 bytes, 1 overlay).
pub const FADE_START: Sig = Sig {
    len: 28,
    hash: 0x8f55_1d42_c453_39e3,
    calls: &[],
    imports: &[],
};

/// An object's frame callback (0x10004f90, cdecl `(obj)`; `esp` the
/// entry stack pointer): the caster's weapon (slot 219 `(*G_CASTER,
/// 0)`), its element 1's matrix into the local at `ebp - 0x44` (slot
/// 236), the first three columns of its first three rows scaled by
/// `1000 / Ch_GetDataScale` (slot 277); a unit matrix at `ebp - 0x84`
/// (slot 235) turned by Z pi/2 (slot 330); `obj + 0x80` = their product
/// (slot 231); the weapon's option position 14 into `obj + 0xd0` (slot
/// 268). Each call sees the words the earlier ones left on the stack.
/// Returns slot 268's `eax`. (The frame's stack cookie is the CRT's.)
pub fn weapon_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, rot) = (ebp.wrapping_sub(0x44), ebp.wrapping_sub(0x84));
    let obj = fr.arg(g, 0);
    let s219 = slot_target(g, w(0x14), 219);
    let caster = g.read::<u32>(w(0x27));
    let mut stack = vec![caster, 0];
    let wpn = g.call(s219, 0, &stack).eax;
    let s236 = slot_target(g, w(0x2f), 236);
    stack.splice(0..0, [wpn, 1, m]);
    g.call(s236, 0, &stack);
    let s277 = slot_target(g, w(0x45), 277);
    stack.insert(0, wpn);
    // call width: 9 at most (the earlier calls' words stay on the stack)
    let d = crate::ob_model::st0_f32(g.call(s277, 0, &stack).st0);
    let k = g.read::<f32>(w(0x5c)) / d;
    for row in 0..3u32 {
        for col in 0..3u32 {
            let at = m + 0x10 * row + 4 * col;
            let v = g.read::<f32>(at) * k;
            g.write::<f32>(at, v);
        }
    }
    let s235 = slot_target(g, w(0x87), 235);
    stack.insert(0, rot);
    // call width: 9 at most (the earlier calls' words stay on the stack)
    g.call(s235, 0, &stack);
    let s330 = slot_target(g, w(0x10d), 330);
    g.call(s330, 0, &[rot, rot, 0x3fc9_0fdb]);
    let s231 = slot_target(g, w(0x12c), 231);
    let mut stack = vec![obj + 0x80, m, rot, rot, rot, 0x3fc9_0fdb];
    g.call(s231, 0, &stack);
    let s268 = slot_target(g, w(0x140), 268);
    stack.splice(0..0, [wpn, 0xe, obj + 0xd0]);
    // call width: 9 (the earlier calls' words stay on the stack)
    Ret::eax(g.call(s268, 0, &stack).eax)
}

/// [`weapon_frame`] (354 bytes, 1 overlay).
pub const WEAPON_FRAME: Sig = Sig {
    len: 354,
    hash: 0xddab_0f27_92c5_d77b,
    // __security_check_cookie
    calls: &[(0x15a, Callee::Any)],
    imports: &[],
};

/// A task's end (0x10004af0, cdecl `(node)`): with `E` = `*G` (`+0x06`,
/// read first) the 0x30-byte block `B` (`+0x10`, `memset`) is cleared,
/// its halfword `*H` (`+0x21`) gets 0x80 and `*W` (`+0x2e`) the word `*V`
/// (`+0x26`); the script runs ([`run`](crate::sa_script::run), `+0x33`);
/// while the game is paused (slot 561, called through `ecx`) its result
/// is returned; else the motion steps
/// ([`motion_step`](crate::s3_motion::motion_step), `+0x4c`) and when
/// neither is 0 `E + 0x2458` becomes 1, the parts are refreshed
/// ([`refresh`], `+0x63`) and the node goes back to the pool
/// ([`give_back`](crate::sa_pool::give_back), a tail jump, `+0x6e`).
/// Returns the motion step's result otherwise.
pub fn script_end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let e = g.read::<u32>(w(0x06));
    g.write_bytes(w(0x10), &[0u8; 0x30]);
    let h = g.read::<u16>(w(0x21));
    g.write::<u16>(w(0x21), h | 0x80);
    let v = g.read::<u32>(w(0x26));
    g.write::<u32>(w(0x2e), v);
    let a = crate::sa_script::run(g, img, img.rel32_target(f + 0x33));
    let s = slot_target(g, w(0x39), 0x8c4 / 4);
    let paused = g.call(s, s, &[]).eax;
    if paused != 0 {
        return Ret::eax(paused);
    }
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x4c));
    if a == 0 || m == 0 {
        return Ret::eax(m);
    }
    g.write::<u32>(e + 0x2458, 1);
    refresh(g, img, img.rel32_target(f + 0x63));
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x6e),
        node,
    ))
}

/// [`script_end`] (118 bytes, 1 overlay).
pub const SCRIPT_END: Sig = Sig {
    len: 118,
    hash: 0x17fd_c111_04cf_a420,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x33, Callee::Sig(&crate::sa_script::RUN)),
        (0x4c, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x63, Callee::Sig(&REFRESH)),
        (0x6e, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};
