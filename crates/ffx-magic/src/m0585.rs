//! `magic_0585.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::h6_setup::unlink_task;
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// A task `t` (its vtable and callback the immediates at `ops[0]`,
/// `ops[1]`) linked at the end of the task ring: the ring's last word
/// `ops[2]`/`ops[4]`, its sentinel `ops[3]`, the flag word `ops[5]`
/// (bit 0 cleared), the last word written `ops[6]`. With `own`, `+0x14`
/// is the task itself.
pub(crate) fn task_link<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, ops: [u32; 7], own: bool) {
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(t + 8, t);
    g.write::<u32>(t + 4, t);
    g.write::<u32>(t, w(ops[0]));
    g.write::<u32>(t + 0x10, w(ops[1]));
    if own {
        g.write::<u32>(t + 0x14, t);
    }
    let tail = g.read::<u32>(w(ops[2]));
    g.write::<u32>(t + 8, tail);
    g.write::<u32>(t + 4, w(ops[3]));
    let tail = g.read::<u32>(w(ops[4]));
    g.write::<u32>(tail + 4, t);
    let fl = g.read::<u32>(w(ops[5]));
    g.write::<u32>(w(ops[5]), fl & !1);
    g.write::<u32>(w(ops[6]), t);
}

/// Takes a task's node (`task[0]`) off its ring. Returns the node's next.
fn unlink<G: Game>(g: &mut G, task: u32) -> u32 {
    let node = g.read::<u32>(task);
    let next = g.read::<u32>(node + 4);
    let prev = g.read::<u32>(node + 8);
    g.write::<u32>(next + 8, prev);
    let prev = g.read::<u32>(node + 8);
    let next = g.read::<u32>(node + 4);
    g.write::<u32>(prev + 4, next);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    next
}

/// Sets up the effect's object (0x100078a0, internal: `ecx` = the
/// object, `edx` = the source; a frame with a stack cookie): `+0xe68` =
/// the object; its task `+0xe54` linked ([`task_link`], no own word);
/// slot 236 `(src[0x54], 0xfa, m)` with the matrix `m` at `ebp - 0x54`;
/// the vector `v` at `ebp - 0x14` = (0, 10, 0, 1); slot 229 `(obj, m,
/// v)`; slot 719 `(DATA, 9, 0)`, each seeing the earlier calls' words.
/// Then `+0xe78` = the source, `+0xe50` = slot 719's answer, `+0xe6c` 0.
/// Returns slot 719's `eax`.
pub fn setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    let (obj, src) = (fr.ecx, fr.edx);
    g.write::<u32>(obj + 0xe68, obj);
    let t = obj + 0xe54;
    task_link(
        g,
        img,
        f,
        t,
        [0x25, 0x32, 0x37, 0x41, 0x46, 0x4f, 0x5f],
        false,
    );
    let t236 = slot_target(g, w(0x59), 236);
    let x = g.read::<u32>(src + 0x54);
    let mut stack = vec![x, 0xfa, m];
    g.call(t236, 0, &stack);
    for (i, b) in (0u32..).zip([0, 0x4120_0000, 0, ONE]) {
        g.write::<u32>(v + 4 * i, b);
    }
    let t229 = slot_target(g, w(0x7c), 229);
    stack.splice(0..0, [obj, m, v]);
    // call width: 6 (the earlier calls' words stay on the stack)
    g.call(t229, 0, &stack);
    let t719 = slot_target(g, w(0xa6), 719);
    stack.splice(0..0, [w(0xb5), 9, 0]);
    // call width: 9 (the earlier calls' words stay on the stack)
    let eax = g.call(t719, 0, &stack).eax;
    g.write::<u32>(obj + 0xe78, src);
    g.write::<u32>(obj + 0xe50, eax);
    g.write::<u32>(obj + 0xe6c, 0);
    eax
}

/// [`setup`] (229 bytes, 1 overlay).
pub const SETUP: Sig = Sig {
    len: 229,
    hash: 0x96a1_894a_9fe7_df88,
    // __security_check_cookie
    calls: &[(0xdd, Callee::Any)],
    imports: &[],
};

/// A sound task's run (`(task)` on `r = task[1]`, the counter `+0x34`
/// counting down each call, the state `+0x18`): state 0 starts a wait of
/// `wait` calls (state 1); when it runs out, state 10 plays the sounds
/// (`play`) and takes the task's node off its ring (state 20000).
/// Returns the unlinked node's next or the state's remainder.
fn sound_task<G: Game>(g: &mut G, fr: &Frame, wait: u32, play: impl FnOnce(&mut G, u32)) -> Ret {
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let tick = |g: &mut G| {
        let n = g.read::<u32>(r + 0x34);
        g.write::<u32>(r + 0x34, n.wrapping_sub(1));
    };
    loop {
        let state = g.read::<u32>(r + 0x18);
        match state {
            0 | 1 => {
                if state == 0 {
                    g.write::<u32>(r + 0x34, wait);
                    g.write::<u32>(r + 0x18, 1);
                }
                if g.read::<i32>(r + 0x34) > 0 {
                    tick(g);
                    return Ret::eax(0);
                }
                g.write::<u32>(r + 0x18, 10);
            }
            10 => {
                play(g, r);
                let next = unlink(g, task);
                g.write::<u32>(r + 0x18, 0x4e20);
                tick(g);
                return Ret::eax(next);
            }
            s => {
                tick(g);
                return Ret::eax(s.wrapping_sub(10));
            }
        }
    }
}

/// The indexed sound's task (0x10007990, `(task)`): [`sound_task`]
/// waiting 119 calls, then slot 709 `(SOUNDS[r[0x3c]].1)` (the halfword
/// at operand `+0x27` + 8 * index).
pub fn sound<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    sound_task(g, fr, 0x77, |g, r| {
        let i = g.read::<u32>(r + 0x3c);
        let id = u32::from(g.read::<u16>(w(0x27).wrapping_add(i.wrapping_mul(8))));
        let t = slot_target(g, w(0x2d), 709);
        g.call(t, 0, &[id]);
    })
}

/// [`sound`] (129 bytes, 1 overlay).
pub const SOUND: Sig = Sig {
    len: 129,
    hash: 0x70ff_9c55_edbd_6cd5,
    calls: &[],
    imports: &[],
};

/// Starts the indexed sound (0x10007a20, internal: `ecx` = the task,
/// `edx` = the index): the task ([`sound`] its callback) linked
/// ([`task_link`]), slot 709 `(SOUNDS[index].0)` (the halfword at
/// operand `+0x37` + 8 * index), `+0x3c` = the index, state 0. Returns
/// slot 709's `eax`.
pub fn sound_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (t, i) = (fr.ecx, fr.edx);
    task_link(
        g,
        img,
        f,
        t,
        [0xe, 0x15, 0x1d, 0x27, 0x2c, 0x3d, 0x4a],
        true,
    );
    let id = u32::from(g.read::<u16>(w(0x37).wrapping_add(i.wrapping_mul(8))));
    let s = slot_target(g, w(0x44), 709);
    let eax = g.call(s, 0, &[id]).eax;
    g.write::<u32>(t + 0x3c, i);
    g.write::<u32>(t + 0x18, 0);
    eax
}

/// [`sound_start`] (102 bytes, 1 overlay).
pub const SOUND_START: Sig = Sig {
    len: 102,
    hash: 0x04ff_8ddd_8842_b084,
    calls: &[],
    imports: &[],
};

/// The two sounds' task (0x10007a90, `(task)`): [`sound_task`] waiting
/// 20 calls, then slot 709 `(0x15)` and `(0x16)` (the first word left
/// under the second).
pub fn two_sounds<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    sound_task(g, fr, 0x14, |g, _| {
        let t = slot_target(g, w(0x21), 709);
        g.call(t, 0, &[0x15]);
        let t = slot_target(g, w(0x30), 709);
        g.call(t, 0, &[0x16, 0x15]);
    })
}

/// [`two_sounds`] (134 bytes, 1 overlay).
pub const TWO_SOUNDS: Sig = Sig {
    len: 134,
    hash: 0xb615_966e_1713_f58d,
    calls: &[],
    imports: &[],
};

/// Starts the two sounds' task (0x10007b20, internal: `ecx` = the task;
/// a frame with a local record at `ebp - 0x18`): the task
/// ([`two_sounds`] its callback) linked ([`task_link`]), slot 709
/// `(0x32)`; the effect node `+0x3c` from `[+0x3c, LIST, A, B, DATA, 1]`
/// ([`effect_node`](crate::sb_nodes::effect_node)), its vtable (`+0x8c`),
/// flags 6 and 1 in `+0x68`, `+0x6c` 0, state 0. Returns the record's
/// first word.
pub fn two_sounds_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let t = fr.ecx;
    task_link(
        g,
        img,
        f,
        t,
        [0x14, 0x1b, 0x23, 0x2d, 0x32, 0x40, 0x47],
        true,
    );
    let s = slot_target(g, w(0x3a), 709);
    g.call(s, 0, &[0x32]);
    let node = t + 0x3c;
    let rec = fr.esp.wrapping_sub(4 + 0x18);
    for (i, v) in (0u32..).zip([node, w(0x62), w(0x69), w(0x70), w(0x77), 1]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x83), rec);
    let first = g.read::<u32>(rec);
    g.write::<u32>(first, w(0x8c));
    let fl = g.read::<u32>(t + 0x68);
    g.write::<u32>(t + 0x68, fl | 6);
    let fl = g.read::<u32>(node + 0x2c);
    g.write::<u32>(node + 0x2c, fl | 1);
    g.write::<u32>(node + 0x30, 0);
    g.write::<u32>(t + 0x18, 0);
    first
}

/// [`two_sounds_start`] (172 bytes, 1 overlay).
pub const TWO_SOUNDS_START: Sig = Sig {
    len: 172,
    hash: 0xdf65_c271_0ba6_9053,
    calls: &[(0x83, Callee::Sig(&crate::sb_nodes::EFFECT_NODE))],
    imports: &[],
};

/// Clears the models' glow entries (0x10008fd0, internal, no
/// arguments): in every part of kind 0x3700, each entry (the count
/// halfword `+0x2` signed, 8 bytes each from `part + part[1]`) gets the
/// halfword 0, the byte `+0x2` 0 and the byte `+0x4` 0x44
/// ([`parts`](crate::m0577::parts)). Returns the model table's cursor
/// past its end.
pub fn glow_clear<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    crate::m0577::parts(g, [w(0x8), w(0xa3)], |g, _, part| {
        if g.read::<u32>(part + 0x10) & 0x3fff != 0x3700 {
            return;
        }
        let mut e = g.read::<u32>(part + 4).wrapping_add(part);
        let mut j = 0i32;
        while j < i32::from(g.read::<i16>(part + 2)) {
            g.write::<u16>(e, 0);
            g.write::<u8>(e + 2, 0);
            g.write::<u8>(e + 4, 0x44);
            e = e.wrapping_add(8);
            j += 1;
        }
    })
}

/// [`glow_clear`] (199 bytes, 1 overlay).
pub const GLOW_CLEAR: Sig = Sig {
    len: 199,
    hash: 0x09ad_d30b_2f5a_262b,
    calls: &[],
    imports: &[],
};

/// The camera drift task (0x10007bd0, `(task)`, a task step) on the
/// record `r = task[1]`, by the state `+0x18` (a wait `+0x34`, the
/// level byte `+0xc3`, two speeds `+0x30`/`+0x38` with their steps
/// `+0x3c`/`+0x40`):
/// - 0: state 1, speeds `(200000, 0)`, steps `(20000, 2000)`, then at
///   once 1: the level up by 4; at 0x50 state 2, steps `(-5000, -500)`,
///   a wait of 60;
/// - 2: when the wait is out, state 3, steps `(10000, 1000)`, a wait of
///   30; 3: when out, state 4, steps 0, a wait of 240; 4: when out,
///   state 5;
/// - 5: the level down by 4; at 0 state 10 at once; 10: bit 4 of the u16
///   `+0xec`, state 11 and a wait of 2; 11: when the wait is out, the
///   task's node leaves its ring, state 20000.
///
/// Every exit moves the point: `+0xe0 -= speed1`, `+0xe4 -= speed2`,
/// each speed plus its step, the wait down. Returns the record.
pub fn drift_task<G: Game>(g: &mut G, _img: &Image, fr: &Frame) -> Ret {
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let wait_out = |g: &G| g.read::<i32>(r + 0x34) <= 0;
    let next = |g: &mut G| {
        let s = g.read::<u32>(r + 0x18);
        g.write::<u32>(r + 0x18, s.wrapping_add(1));
    };
    let steps = |g: &mut G, a: u32, b: u32| {
        g.write::<u32>(r + 0x3c, a);
        g.write::<u32>(r + 0x40, b);
    };
    let mut state = g.read::<u32>(r + 0x18);
    loop {
        match state {
            0 | 1 => {
                if state == 0 {
                    next(g);
                    g.write::<u32>(r + 0x30, 0x3_0d40);
                    g.write::<u32>(r + 0x38, 0);
                    steps(g, 0x4e20, 0x7d0);
                }
                let b = g.read::<u8>(r + 0xc3).wrapping_add(4);
                g.write::<u8>(r + 0xc3, b);
                if g.read::<u8>(r + 0xc3) == 0x50 {
                    next(g);
                    steps(g, 0xffff_ec78, 0xffff_fe0c);
                    g.write::<u32>(r + 0x34, 0x3c);
                }
            }
            2 if wait_out(g) => {
                next(g);
                steps(g, 0x2710, 0x3e8);
                g.write::<u32>(r + 0x34, 0x1e);
            }
            3 if wait_out(g) => {
                next(g);
                steps(g, 0, 0);
                g.write::<u32>(r + 0x34, 0xf0);
            }
            4 if wait_out(g) => next(g),
            5 => {
                let b = g.read::<u8>(r + 0xc3).wrapping_add(0xfc);
                g.write::<u8>(r + 0xc3, b);
                if b == 0 {
                    g.write::<u32>(r + 0x18, 10);
                    state = 10;
                    continue;
                }
            }
            10 | 11 => {
                if state == 10 {
                    let fl = g.read::<u16>(r + 0xec);
                    g.write::<u16>(r + 0xec, fl | 0x10);
                    next(g);
                    g.write::<u32>(r + 0x34, 2);
                }
                if wait_out(g) {
                    unlink_task(g, task);
                    g.write::<u32>(r + 0x18, 0x4e20);
                }
            }
            _ => {}
        }
        break;
    }
    let s1 = g.read::<u32>(r + 0x30);
    let d1 = g.read::<u32>(r + 0x3c);
    let s2 = g.read::<u32>(r + 0x38);
    let x = g.read::<u32>(r + 0xe0);
    g.write::<u32>(r + 0xe0, x.wrapping_sub(s1));
    let y = g.read::<u32>(r + 0xe4);
    g.write::<u32>(r + 0xe4, y.wrapping_sub(s2));
    g.write::<u32>(r + 0x30, d1.wrapping_add(s1));
    let d2 = g.read::<u32>(r + 0x40);
    let n = g.read::<u32>(r + 0x34);
    g.write::<u32>(r + 0x34, n.wrapping_sub(1));
    g.write::<u32>(r + 0x38, d2.wrapping_add(s2));
    Ret::eax(r)
}

/// [`drift_task`] (356 bytes, 1 overlay).
pub const DRIFT_TASK: Sig = Sig {
    len: 356,
    hash: 0xf141_d09a_20b2_1836,
    calls: &[],
    imports: &[],
};

/// Sets up the camera drift object (0x10008250, internal: `ecx` = the
/// object; a /GS frame): its task node `n = obj + 0xb8` (owner `obj`,
/// handlers `+0x23`, step the drift task `+0x30`) joins the DLL's ring;
/// four colour keys `(slot 544 (k, obj + o))` each scaled by a local
/// colour `V` (`ebp - 0x14`) through slot 234 `(obj + o + 0x10, &V, obj +
/// o)`; slot 588 `(2, obj + 0x50)` and slot 232 normalise two vectors
/// (`+0x60` = `(0.5, -1, 0, 1)` first); `+0xb0`, `+0xb4`, `+0xd0` 0.
/// Each call sees the words the earlier ones left on the stack, up to
/// the clean-ups. Returns the last call's `eax`.
pub fn drift_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    let n = obj + 0xb8;
    g.write::<u32>(n + 0x14, obj);
    g.write::<u32>(n, w(0x23));
    g.write::<u32>(n + 8, n);
    g.write::<u32>(n + 4, n);
    g.write::<u32>(n + 0x10, w(0x30));
    let last = g.read::<u32>(w(0x35));
    g.write::<u32>(n + 8, last);
    g.write::<u32>(n + 4, w(0x3f));
    let last = g.read::<u32>(w(0x44));
    g.write::<u32>(last + 4, n);
    let t = slot_target(g, w(0x4e), 544);
    let fl = g.read::<u32>(w(0x54));
    g.write::<u32>(w(0x54), fl & !1);
    g.write::<u32>(w(0x5b), n);
    let mut stack = vec![3, obj];
    g.call(t, 0, &stack);
    let v = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
    let colour = |g: &mut G, c: [u32; 4]| {
        for (i, x) in (0u32..).zip(c) {
            g.write::<u32>(v + 4 * i, x);
        }
    };
    let keys: [(u32, u32, u32, u32, [u32; 4]); 3] = [
        (0x71, 0x20, obj, 0, [0x3e99_999a; 4]),
        (
            0xb6,
            0x40,
            obj + 0x30,
            0x9a,
            [0x3ee6_6666, 0x3e19_999a, 0x3dcc_cccd, 0x3f80_0000],
        ),
        (0x101, 0xa0, obj + 0x90, 0xdf, [0, 0, 0, 0x3f80_0000]),
    ];
    for (k, (tab, dst, key, t544, c)) in (0u32..).zip(keys) {
        if k > 0 {
            let t = slot_target(g, w(t544), 544);
            stack.splice(0..0, [3 - k, key]);
            // call width: 12 at most
            g.call(t, 0, &stack);
        }
        let t = slot_target(g, w(tab), 234);
        colour(g, c);
        stack.splice(0..0, [obj + dst, v, key]);
        // call width: 15 at most
        g.call(t, 0, &stack);
    }
    let t = slot_target(g, w(0x12a), 544);
    stack.splice(0..0, [0, obj + 0x70]);
    // call width: 17
    g.call(t, 0, &stack);
    colour(g, [0, 0, 0, 0x3f80_0000]);
    let t = slot_target(g, w(0x168), 234);
    let mut stack = vec![obj + 0x80, v, obj + 0x70];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x175), 588);
    stack.splice(0..0, [2, obj + 0x50]);
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x188), 232);
    stack.splice(0..0, [obj + 0x50, obj + 0x50]);
    // call width: 7
    g.call(t, 0, &stack);
    for (o, x) in [
        (0x60u32, 0x3f00_0000u32),
        (0x64, 0xbf80_0000),
        (0x68, 0),
        (0x6c, 0x3f80_0000),
    ] {
        g.write::<u32>(obj + o, x);
    }
    let t = slot_target(g, w(0x1a2), 232);
    stack.splice(0..0, [obj + 0x60, obj + 0x60]);
    // call width: 9
    let e = g.call(t, 0, &stack).eax;
    for o in [0xb0u32, 0xb4, 0xd0] {
        g.write::<u32>(obj + o, 0);
    }
    e
}

/// [`drift_setup`] (500 bytes, 1 overlay).
pub const DRIFT_SETUP: Sig = Sig {
    len: 500,
    hash: 0xde3f_d94c_d05b_3b07,
    calls: &[
        // __security_check_cookie
        (0x1ec, Callee::Any),
    ],
    imports: &[],
};

/// Sets up a light object (0x10007d40, internal: `ecx` = the object,
/// `edx` = its parent node, cdecl `(level)`): a ring of its own with the
/// vtable `+0x12` and the method `+0x19` (`+0x10`, `+0x14` itself),
/// appended at the tail of the ring `R` (the immediate `+0x2b`; tail
/// `G_T` `+0x21`, bit 0 of `+0x3d` cleared). Slot 719 `(NAME, 2, 10)`
/// gives the data; the light node `obj + 0x48` (vtable `+0x7e`) set up
/// under the parent ([`light_node`](crate::s4_small::light_node), the
/// local `[node, +0x6a, parent, -, level, data]` at `ebp - 0x18`) and
/// its level ([`light_level`](crate::s4_small::light_level), the local
/// `[node, +0x97, 0x60000, 0xf0000, 0xbb800, 2, **G, 1]` at `ebp -
/// 0x24`). Then `+0x44` the parent, `+0xc0` 0x8c2040, `+0x30` and
/// `+0x18` 0. Returns the level's answer.
pub fn light_object<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (obj, parent) = (fr.ecx, fr.edx);
    g.write::<u32>(obj + 8, obj);
    g.write::<u32>(obj + 4, obj);
    g.write::<u32>(obj, w(0x12));
    g.write::<u32>(obj + 0x10, w(0x19));
    g.write::<u32>(obj + 0x14, obj);
    let tail = g.read::<u32>(w(0x21));
    g.write::<u32>(obj + 8, tail);
    g.write::<u32>(obj + 4, w(0x2b));
    let tail = g.read::<u32>(w(0x30));
    g.write::<u32>(tail + 4, obj);
    let level = fr.arg(g, 0);
    let flags = g.read::<u32>(w(0x3d));
    g.write::<u32>(w(0x3d), flags & !1);
    // ebp = esp - 4; the locals below it
    let ebp = fr.esp.wrapping_sub(4);
    g.write::<u32>(ebp - 0xc, level);
    g.write::<u32>(w(0x50), obj);
    let node = obj + 0x48;
    g.write::<u32>(ebp - 0x18, node);
    g.write::<u32>(ebp - 0x14, w(0x6a));
    g.write::<u32>(ebp - 0x10, parent);
    let t = slot_target(g, w(0x46), 719);
    let data = g.call(t, 0, &[w(0x60), 2, 0xa]).eax;
    g.write::<u32>(ebp - 0x8, data);
    g.write::<u32>(node, w(0x7e));
    // push ebx, esi, edi; the return address
    let esp = ebp.wrapping_sub(0x28 + 0x10);
    let ln = Frame {
        at: img.rel32_target(f + 0x83),
        esp,
        ecx: ebp - 0x18,
        ..*fr
    };
    crate::s4_small::light_node(g, img, &ln);
    let v = g.read::<u32>(g.read::<u32>(w(0x88)));
    for (o, x) in [
        (0x24u32, node),
        (0x20, w(0x97)),
        (0x1c, 0x6_0000),
        (0x18, 0xf_0000),
        (0x14, 0xb_b800),
        (0x10, 2),
        (0xc, v),
        (0x8, 1),
    ] {
        g.write::<u32>(ebp - o, x);
    }
    let ll = Frame {
        at: img.rel32_target(f + 0xc2),
        esp,
        ecx: ebp - 0x24,
        ..*fr
    };
    let eax = crate::s4_small::light_level(g, img, &ll).eax;
    g.write::<u32>(obj + 0x44, parent);
    g.write::<u32>(obj + 0xc0, 0x8c_2040);
    g.write::<u32>(obj + 0x30, 0);
    g.write::<u32>(obj + 0x18, 0);
    Ret::eax(eax)
}

/// [`light_object`] (232 bytes, 1 overlay).
pub const LIGHT_OBJECT: Sig = Sig {
    len: 232,
    hash: 0x313d_4056_b154_298b,
    calls: &[
        (0x83, Callee::Sig(&crate::s4_small::LIGHT_NODE)),
        (0xc2, Callee::Sig(&crate::s4_small::LIGHT_LEVEL)),
    ],
    imports: &[],
};
