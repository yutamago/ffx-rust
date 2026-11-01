//! `magic_0397.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A task's end (0x10004250, cdecl `(node)`): the script runs
/// ([`run`](crate::sa_script::run), `+0x0c`); while the game is paused
/// slot 561's result is returned; else the motion steps
/// ([`motion_step`](crate::s3_motion::motion_step), `+0x25`) and when
/// neither is 0 the effect's `+0x2408` (`*E`, `+0x07`) becomes 1, slot
/// 628 `(0)` runs and the node goes back to the pool
/// ([`give_back`](crate::sa_pool::give_back), a tail jump, `+0x54`).
/// Returns the motion step's result otherwise.
pub fn script_end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let e = g.read::<u32>(w(0x07));
    let a = crate::sa_script::run(g, img, img.rel32_target(f + 0x0c));
    let s = slot_target(g, w(0x12), 561);
    let paused = g.call(s, 0, &[]).eax;
    if paused != 0 {
        return Ret::eax(paused);
    }
    let m = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x25));
    if a == 0 || m == 0 {
        return Ret::eax(m);
    }
    g.write::<u32>(e + 0x2408, 1);
    let s = slot_target(g, w(0x32), 628);
    g.call(s, 0, &[0]);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x54),
        node,
    ))
}

/// [`script_end`] (92 bytes, 1 overlay).
pub const SCRIPT_END: Sig = Sig {
    len: 92,
    hash: 0xdb50_4c0e_96fd_55b7,
    calls: &[
        (0x0c, Callee::Sig(&crate::sa_script::RUN)),
        (0x25, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x54, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// An actor's position (0x10005800, cdecl `(obj)`): the actor `i`
/// (`+0x120`; -1 is the caster `*X`, `+0x1b`, else the table `+0x2d` at
/// the byte `B[i]`, `B` = `*P`, `+0x22`); slot 262 `(a, v, v + 4, v + 8)`
/// and slot 268 `(a, 0x10, v)` with `v` = `obj + 0xd0`, then `v + 0xc`
/// = 1.0. Returns slot 268's `eax`.
pub fn actor_position<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let obj = fr.arg(g, 0);
    let i = g.read::<u32>(obj + 0x120);
    let v = obj + 0xd0;
    let a = if i == u32::MAX {
        g.read::<u32>(w(0x1b))
    } else {
        let b = g.read::<u8>(g.read::<u32>(w(0x22)).wrapping_add(i));
        g.read::<u32>(w(0x2d).wrapping_add(4 * u32::from(b)))
    };
    let s = slot_target(g, w(0x3a), 0x418 / 4);
    g.call(s, 0, &[a, v, v + 4, v + 8]);
    let s = slot_target(g, w(0x49), 0x430 / 4);
    // call width: 7 (the earlier call's words under it)
    let eax = g.call(s, 0, &[a, 0x10, v, a, v, v + 4, v + 8]).eax;
    g.write::<u32>(v + 0xc, 0x3f80_0000);
    Ret::eax(eax)
}

/// [`actor_position`] (103 bytes, 1 overlay).
pub const ACTOR_POSITION: Sig = Sig {
    len: 103,
    hash: 0x874f_7ae0_52ca_7564,
    calls: &[],
    imports: &[],
};

/// The party check (0x100064b0, cdecl `(state)`, locals `n` at `ebp - 8`
/// and a byte at `ebp - 1`): slot 288 `(9, &n)` gives the party's bytes;
/// each one but the caster's own byte `*S` (`+0x21`) is asked slot 637
/// `(b)`, and any non-zero answer clears the result; with none the
/// state's byte `+0x56` becomes 1. Returns the state then, else the last
/// value the code left in `eax` (slot 637's answer, the list with its low
/// byte the last skipped byte, or the list).
pub fn party_check<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let n_at = ebp.wrapping_sub(8);
    let s = slot_target(g, w(0x0e), 0x480 / 4);
    let list = g.call(s, 0, &[9, n_at]).eax;
    let own = g.read::<u8>(w(0x21));
    g.write::<u8>(ebp.wrapping_sub(1), own);
    let mut eax = list;
    let mut ok = true;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(n_at) {
        let b = g.read::<u8>(list.wrapping_add(i));
        eax = (eax & !0xff) | u32::from(b);
        if b != own {
            let s = slot_target(g, w(0x44), 0x9f4 / 4);
            eax = g.call(s, 0, &[u32::from(b)]).eax;
            if eax != 0 {
                ok = false;
            }
        }
        i += 1;
    }
    if !ok {
        return Ret::eax(eax);
    }
    let state = fr.arg(g, 0);
    g.write::<u8>(state + 0x56, 1);
    Ret::eax(state)
}

/// [`party_check`] (120 bytes, 1 overlay).
pub const PARTY_CHECK: Sig = Sig {
    len: 120,
    hash: 0x10d6_f17f_1e4b_7a72,
    calls: &[],
    imports: &[],
};

/// The two sprite tasks (0x10006240, no arguments, no frame): with `E` =
/// `*G` (`+0x04`, read again for the second), the sprite blocks `E +
/// 0x19c0` and `E + 0x1a60` get their colour bytes and words and each a
/// pool node ([`take`](crate::sa_pool::take), owner the callback `+0x10`
/// / `+0xb6`) holding it (`+0x14`); the nodes go last on the task list
/// (first `+0x6d`, last `+0x83`/`+0x11c`), the first block's `+0x94`
/// becomes 1 between them. Returns the second node.
pub fn sprite_tasks<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let a = g.read::<u32>(w(0x04)).wrapping_add(0x19c0);
    g.write::<u32>(a + 0x80, 0);
    g.write::<u16>(a + 0x85, 0xffff);
    g.write::<u8>(a + 0x84, 0xff);
    g.write::<u8>(a + 0x87, 0x80);
    g.write::<u32>(a + 0x88, 0);
    g.write::<u16>(a + 0x98, 0x48);
    g.write::<u32>(a + 0x90, 0x1e);
    g.write::<u32>(a + 0x8c, 0x1e);
    g.write::<u32>(a + 0x94, 0);
    let first = crate::sa_pool::take(g, img, img.rel32_target(f + 0x67), w(0x10));
    let mut head = g.read::<u32>(w(0x6d));
    g.write::<u32>(first + 0x14, a);
    g.write::<u32>(first + 4, 0);
    if head == 0 {
        head = first;
        g.write::<u32>(first, 0);
        g.write::<u32>(w(0x98), first);
    } else {
        let last = g.read::<u32>(w(0x83));
        g.write::<u32>(first, last);
        g.write::<u32>(last + 4, first);
    }
    g.write::<u32>(first + 8, first);
    g.write::<u32>(a + 0x94, 1);
    let b = g.read::<u32>(w(0xab)).wrapping_add(0x1a60);
    g.write::<u32>(b + 0x81, 0x0080_ffff);
    g.write::<u8>(b + 0x80, 0xff);
    g.write::<u16>(b + 0x86, 0);
    g.write::<u8>(b + 0x85, 0);
    g.write::<u32>(b + 0x88, 0x1f);
    g.write::<u16>(b + 0x98, 0x48);
    g.write::<u32>(b + 0x90, 0x3c);
    g.write::<u32>(b + 0x8c, 0x3c);
    g.write::<u32>(b + 0x94, 0);
    let second = crate::sa_pool::take(g, img, img.rel32_target(f + 0x10d), w(0xb6));
    g.write::<u32>(second + 0x14, b);
    g.write::<u32>(second + 4, 0);
    g.write::<u32>(w(0x11c), second);
    if head == 0 {
        // only for a null first node
        g.write::<u32>(second, 0);
        g.write::<u32>(w(0x139), second);
    } else {
        g.write::<u32>(second, first);
        g.write::<u32>(first + 4, second);
    }
    g.write::<u32>(second + 8, second);
    Ret::eax(second)
}

/// [`sprite_tasks`] (322 bytes, 1 overlay).
pub const SPRITE_TASKS: Sig = Sig {
    len: 322,
    hash: 0xb2d9_7460_5cac_88c2,
    calls: &[
        (0x67, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x10d, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};
