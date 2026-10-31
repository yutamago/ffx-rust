//! `magic_0585.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// A task `t` (its vtable and callback the immediates at `ops[0]`,
/// `ops[1]`) linked at the end of the task ring: the ring's last word
/// `ops[2]`/`ops[4]`, its sentinel `ops[3]`, the flag word `ops[5]`
/// (bit 0 cleared), the last word written `ops[6]`. With `own`, `+0x14`
/// is the task itself.
fn task_link<G: Game>(g: &mut G, img: &Image, f: u32, t: u32, ops: [u32; 7], own: bool) {
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
