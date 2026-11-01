//! `magic_0577.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::h6_setup::{ring_join, unlink_task};
use crate::l3_common::{detach, list_unlink_current};
use crate::runtime::slot_target;
use crate::s4_summon::trunc;
use crate::sa_summon::model_setup;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Sets a node's colour from its halfwords (0x100090a0, internal: `ecx`
/// = the node, `edx` = a word for slot 325): `+0x2c..+0x38` = the
/// halfwords `+0xd0..+0xd6` times `K` (operand `+0x04`); slot 325
/// `(node + 0x3c, edx)`; flag bit 0 of `+0xe8`, `+0xe4` = 0xa000a.
/// Returns slot 325's `eax`.
pub fn colour<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let node = fr.ecx;
    let k = g.read::<f32>(w(0x4));
    for i in 0..4 {
        let h = u32::from(g.read::<u16>(node + 0xd0 + 2 * i));
        g.write::<f32>(node + 0x2c + 4 * i, mul(cvt(h), k));
    }
    let t = slot_target(g, w(0x68), 325);
    let r = g.call(t, 0, &[node + 0x3c, fr.edx]).eax;
    let fl = g.read::<u32>(node + 0xe8);
    g.write::<u32>(node + 0xe8, fl | 1);
    g.write::<u32>(node + 0xe4, 0x000a_000a);
    r
}

/// [`colour`] (143 bytes, 1 overlay).
pub const COLOUR: Sig = Sig {
    len: 143,
    hash: 0xfcfd_13fc_bfff_847a,
    calls: &[],
    imports: &[],
};

/// Each model of the effect's model table (`*G_M`, the table at `+0x18`,
/// the count halfword `+0xa`), its parts (the part count `+0x6` signed,
/// 8-byte entries from `+0x10` with a signed offset), calls `each` with
/// the model and the part's address.
pub(crate) fn parts<G: Game>(
    g: &mut G,
    m_at: [u32; 2],
    mut each: impl FnMut(&mut G, u32, u32),
) -> u32 {
    let mut m = g.read::<u32>(m_at[0]);
    let mut p = g.read::<u32>(m.wrapping_add(0x18));
    let mut i = 0u32;
    if g.read::<u16>(m.wrapping_add(0xa)) == 0 {
        return p;
    }
    loop {
        let model = g.read::<u32>(p);
        if model != 0 && 0 < g.read::<i16>(model + 6) {
            let mut j = 0i32;
            loop {
                let off = i32::from(g.read::<i16>(model + 0x10 + 8 * j.cast_unsigned()));
                let part = model.wrapping_add(off.cast_unsigned());
                each(g, model, part);
                j += 1;
                if j >= i32::from(g.read::<i16>(model + 6)) {
                    break;
                }
            }
        }
        p = p.wrapping_add(4);
        i += 1;
        m = g.read::<u32>(m_at[1]);
        if i.cast_signed() >= i32::from(g.read::<u16>(m.wrapping_add(0xa))) {
            return p;
        }
    }
}

/// Switches the models' flash (0x1000de80, internal, no arguments): in
/// every part of kind 0x3580 (`+0x10` low 14 bits) whose data
/// (`part + part[1]`) has the byte `+0x4` 0x48, the byte becomes 0x44
/// ([`parts`]). Returns the model table's owner (`*G_M`).
pub fn flash_off<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    parts(g, [w(0x5), w(0x69)], |g, _, part| {
        let data = g.read::<u32>(part + 4).wrapping_add(part);
        if g.read::<u32>(part + 0x10) & 0x3fff == 0x3580 && g.read::<u8>(data + 4) == 0x48 {
            g.write::<u8>(data + 4, 0x44);
        }
    });
    g.read::<u32>(w(0x78))
}

/// [`flash_off`] (136 bytes, 1 overlay).
pub const FLASH_OFF: Sig = Sig {
    len: 136,
    hash: 0x4345_2595_9b4c_12bd,
    calls: &[],
    imports: &[],
};

/// Switches the models' glow (0x1000e050, internal, no arguments): in
/// every part of kind 0x3700, each of its entries (the count halfword
/// `+0x2` signed, 8 bytes each from `part[1] + 4`) whose first byte is
/// 0x48 becomes 0x42 ([`parts`]). Returns the model table's cursor past
/// its end.
pub fn glow_off<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    parts(g, [w(0x8), w(0x9e)], |g, _, part| {
        if g.read::<u32>(part + 0x10) & 0x3fff != 0x3700 {
            return;
        }
        let mut e = g.read::<u32>(part + 4).wrapping_add(4).wrapping_add(part);
        let mut j = 0i32;
        while j < i32::from(g.read::<i16>(part + 2)) {
            if g.read::<u8>(e) == 0x48 {
                g.write::<u8>(e, 0x42);
            }
            e = e.wrapping_add(8);
            j += 1;
        }
    })
}

/// [`glow_off`] (194 bytes, 1 overlay).
pub const GLOW_OFF: Sig = Sig {
    len: 194,
    hash: 0x3a28_c87e_7515_a564,
    calls: &[],
    imports: &[],
};

/// Starts a node's move (0x100025f0, internal: `ecx` = a record `[node,
/// mode, a, b]`): `+0xcc` = `a`, `+0xd0` 0; with a mode, flag bit 0 of
/// `+0xe4`; without, the bit cleared and the deltas `+0x3c..+0x48` =
/// `+0x2c.. - +0x1c..` and `+0x6c..+0x78` = `+0x5c.. - +0x4c..`; then
/// flag bit 2 and `+0xd4` = `b`. Returns `b`.
pub fn move_start<G: Game>(g: &mut G, rec: u32) -> u32 {
    let mode = g.read::<u32>(rec + 4);
    let node = g.read::<u32>(rec);
    let a = g.read::<u32>(rec + 8);
    g.write::<u32>(node + 0xcc, a);
    g.write::<u32>(node + 0xd0, 0);
    let fl = g.read::<u32>(node + 0xe4);
    let b;
    if mode != 0 {
        g.write::<u32>(node + 0xe4, fl | 1);
        b = g.read::<u32>(rec + 0xc);
    } else {
        g.write::<u32>(node + 0xe4, fl & !1);
        b = g.read::<u32>(rec + 0xc);
        for i in 0..4u32 {
            for base in [0x2cu32, 0x5c] {
                let v = sub(
                    g.read::<f32>(node + base + 4 * i),
                    g.read::<f32>(node + base - 0x10 + 4 * i),
                );
                g.write::<f32>(node + base + 0x10 + 4 * i, v);
            }
        }
    }
    let fl = g.read::<u32>(node + 0xe4);
    g.write::<u32>(node + 0xe4, fl | 4);
    g.write::<u32>(node + 0xd4, b);
    b
}

/// [`move_start`] (195 bytes, 1 overlay).
pub const MOVE_START: Sig = Sig {
    len: 195,
    hash: 0xc3e5_2a19_54b6_c15a,
    calls: &[],
    imports: &[],
};

/// Takes the holder's node off its list and draws it (0x10001ed0,
/// internal: `ecx` = a holder whose first word is the node; a frame
/// with a stack cookie): [`detach`]; the locals `ebp - 0x14` four 1.0
/// and `ebp - 0x24` the node's `+0x7c..+0x88` truncated; slots 332
/// `(node + 0xbc)`, 353 `(ebp - 0x14)`, 267 `(4)`, 271 `(1)`, 669 `(ebp -
/// 0x24)`, each seeing the earlier calls' words. Returns slot 669's
/// `eax`.
pub fn draw_detached<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let node = g.read::<u32>(fr.ecx);
    detach(g, node);
    let node = g.read::<u32>(fr.ecx);
    let (ones, ints) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    for i in 0..4 {
        g.write::<u32>(ones + 4 * i, ONE);
        let v = trunc(g.read::<f32>(node + 0x7c + 4 * i));
        g.write::<u32>(ints + 4 * i, v);
    }
    let mut stack = vec![node + 0xbc];
    for (op, slot, word) in [
        (0x83, 332, 0),
        (0x94, 353, ones),
        (0xa1, 267, 4),
        (0xb0, 271, 1),
    ] {
        if slot != 332 {
            stack.insert(0, word);
        }
        let t = slot_target(g, w(op), slot);
        // call width: 4 at most (the earlier calls' words stay on the stack)
        g.call(t, 0, &stack);
    }
    stack.insert(0, ints);
    let t = slot_target(g, w(0xc3), 669);
    // call width: 5 (the earlier calls' words stay on the stack)
    g.call(t, 0, &stack).eax
}

/// [`draw_detached`] (226 bytes, 1 overlay).
pub const DRAW_DETACHED: Sig = Sig {
    len: 226,
    hash: 0x98f2_de7f_8b00_0653,
    // __security_check_cookie
    calls: &[(0xda, Callee::Any)],
    imports: &[],
};

/// Places a node from its holder (0x10002480, internal: `ecx` = a
/// holder `[node, target, mark, mode]`; a frame with a stack cookie and
/// the local vector `v` at `ebp - 0x14`). Unless bit 3 of `+0xe4` is
/// set, `v` = the words `+0xbc..+0xc8` times `+0x1c..+0x28` and slot
/// 325 `(node + 0xac, v)`; else slot 248 `(v, node + 0x8c, node +
/// 0x9c)`. Then slot 325 `(node + 0x8c, v)`; with a mode slot 234
/// `(node + 0x9c, node + 0xac, v)` and flag bit 4, without one slot 234
/// `(node + 0x9c, target, v)`. Then `+0xdc` 0, flag bit 3, `+0xe0` =
/// `mark`. Returns `mark`.
pub fn place<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x14);
    let h = fr.ecx;
    let node = g.read::<u32>(h);
    if g.read::<u8>(node + 0xe4) & 8 == 0 {
        for i in 0..4 {
            let n = cvt(g.read::<u32>(node + 0xbc + 4 * i));
            let s = g.read::<f32>(node + 0x1c + 4 * i);
            g.write::<f32>(v + 4 * i, mul(n, s));
        }
        let t = slot_target(g, w(0x50), 325);
        g.call(t, 0, &[node + 0xac, v]);
    } else {
        let t = slot_target(g, w(0xa3), 248);
        g.call(t, 0, &[v, node + 0x8c, node + 0x9c]);
    }
    let t = slot_target(g, w(0xbe), 325);
    g.call(t, 0, &[node + 0x8c, v]);
    if g.read::<u32>(h + 0xc) != 0 {
        let t = slot_target(g, w(0xe6), 234);
        g.call(t, 0, &[node + 0x9c, node + 0xac, v]);
        let fl = g.read::<u32>(node + 0xe4);
        g.write::<u32>(node + 0xe4, fl | 0x10);
    } else {
        let target = g.read::<u32>(h + 4);
        let t = slot_target(g, w(0x109), 234);
        g.call(t, 0, &[node + 0x9c, target, v]);
    }
    g.write::<u32>(node + 0xdc, 0);
    let mark = g.read::<u32>(h + 8);
    let fl = g.read::<u32>(node + 0xe4);
    g.write::<u32>(node + 0xe4, fl | 8);
    g.write::<u32>(node + 0xe0, mark);
    mark
}

/// [`place`] (322 bytes, 1 overlay).
pub const PLACE: Sig = Sig {
    len: 322,
    hash: 0xdfba_3474_1f5e_f9fa,
    // __security_check_cookie
    calls: &[(0x13a, Callee::Any)],
    imports: &[],
};

/// Halves the models' texture offsets (0x1000df10, internal, no
/// arguments): in every part of kind 0x34c0 or 0x3cc0, each entry (the
/// count halfword `+0x2` signed; 24 bytes from `+0x32` when bit 3 of the
/// part's first byte is set, else 16 bytes from `+0x2a`) has its
/// halfwords `+0x0` and `+0x4` shifted right one (signed), kind 0x3cc0
/// adding 0x800 to the first ([`parts`]). Returns the model count
/// (`*G_M + 0xa`), or `*G_M` when there are no models.
pub fn halve<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let m = g.read::<u32>(w(0x7));
    if g.read::<u16>(m.wrapping_add(0xa)) == 0 {
        return m;
    }
    parts(g, [w(0x7), w(0x116)], |g, _, part| {
        let kind = g.read::<u32>(part + 0x10) & 0x3fff;
        if kind != 0x34c0 && kind != 0x3cc0 {
            return;
        }
        let (mut e, step) = if g.read::<u8>(part) & 8 != 0 {
            (part + 0x32, 0x18)
        } else {
            (part + 0x2a, 0x10)
        };
        let mut j = 0i32;
        while j < i32::from(g.read::<i16>(part + 2)) {
            let a = g.read::<i16>(e);
            let b = g.read::<i16>(e + 4);
            g.write::<i16>(e + 4, b >> 1);
            let mut a = (a >> 1).cast_unsigned();
            g.write::<u16>(e, a);
            if kind == 0x3cc0 {
                a = a.wrapping_add(0x800);
                g.write::<u16>(e, a);
            }
            e = e.wrapping_add(step);
            j += 1;
        }
    });
    let m = g.read::<u32>(w(0x116));
    u32::from(g.read::<u16>(m.wrapping_add(0xa)))
}

/// [`halve`] (311 bytes, 1 overlay).
pub const HALVE: Sig = Sig {
    len: 311,
    hash: 0x70ff_67a5_e7f0_c052,
    calls: &[],
    imports: &[],
};

/// The effect's end (0x1000dbb0, an overlay table entry, no arguments):
/// the current node off its list ([`list_unlink_current`], inline, the
/// operands 4 bytes on), the model of the list object (immediate
/// `+0x69`) set up ([`model_setup`], a cursor at `ebp - 4`), the next
/// current node unlinked (the called copy); slots 250 `(0, 0)`, 24
/// `(0x301e, 0)` and 25 `(0x301e, 1)`, each seeing the earlier calls'
/// words. Returns 0.
pub fn end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    list_unlink_current(g, img, f + 4, fr.eax);
    // push ebp; push ecx: the cursor is [ebp - 4]
    let cursor = fr.esp.wrapping_sub(8);
    g.write::<u32>(cursor, w(0x69));
    let callee = Frame {
        at: img.rel32_target(f + 0x6e),
        esp: cursor.wrapping_sub(4),
        ecx: cursor,
        ..*fr
    };
    let eax = model_setup(g, img, &callee).eax;
    list_unlink_current(g, img, img.rel32_target(f + 0x73), eax);
    let t = slot_target(g, w(0x78), 250);
    let mut stack = vec![0, 0];
    g.call(t, 0, &stack);
    for (op, slot, k) in [(0x97, 24, 0), (0xa8, 25, 1)] {
        stack.splice(0..0, [0x301e, k]);
        let t = slot_target(g, w(op), slot);
        // call width: 6 at most (the earlier calls' words stay on the stack)
        g.call(t, 0, &stack);
    }
    Ret::eax(0)
}

/// [`end`] (193 bytes, 1 overlay).
pub const END: Sig = Sig {
    len: 193,
    hash: 0x3b06_d6a3_39c3_0713,
    calls: &[
        (0x6e, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x73, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
    ],
    imports: &[],
};

/// The centre of the targets (0x10008c10, internal: `ecx` = the output
/// vector, `edx` = the source of the base; a frame with a stack cookie):
/// the base `b` at `ebp - 0x14` (slot 325 `(b, edx)`); for each of the
/// `*G_N` target bytes (operand `+0x55`) that slot 684 accepts, slot 531
/// `(target, v)` gives a position at `v = ebp - 0x24`, which becomes
/// `v - b` and adds to the sum. The output is `b + sum * (K / count)`
/// (operand `+0xfe`), `+0xc` 1.0. Returns the last call's `eax`.
pub fn centre<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (b, v) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    let out = fr.ecx;
    let t = slot_target(g, w(0x19), 325);
    let mut eax = g.call(t, 0, &[b, fr.edx]).eax;
    let mut sum = [0.0f32; 3];
    let mut count = 0u32;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(0x48)) {
        let tgt = u32::from(g.read::<u8>(w(0x55) + i));
        let t = slot_target(g, w(0x5b), 684);
        eax = g.call(t, 0, &[tgt]).eax;
        if eax != 0 {
            let tgt = u32::from(g.read::<u8>(w(0x75) + i));
            let t = slot_target(g, w(0x7b), 531);
            eax = g.call(t, 0, &[tgt, v]).eax;
            let d: [f32; 3] = std::array::from_fn(|k| {
                sub(
                    g.read::<f32>(v + 4 * k as u32),
                    g.read::<f32>(b + 4 * k as u32),
                )
            });
            for k in [1usize, 2, 0] {
                g.write::<f32>(v + 4 * k as u32, d[k]);
            }
            for k in 0..3 {
                sum[k] = add(d[k], sum[k]);
            }
            count += 1;
        }
        i += 1;
    }
    let k = div(g.read::<f32>(w(0xfe)), cvt(count));
    let x = add(g.read::<f32>(b), mul(k, sum[0]));
    g.write::<f32>(out, x);
    g.write::<u32>(out + 0xc, ONE);
    let y = add(g.read::<f32>(b + 4), mul(k, sum[1]));
    g.write::<f32>(out + 4, y);
    let z = add(g.read::<f32>(b + 8), mul(k, sum[2]));
    g.write::<f32>(out + 8, z);
    eax
}

/// [`centre`] (352 bytes, 1 overlay).
pub const CENTRE: Sig = Sig {
    len: 352,
    hash: 0x24ce_1d62_2215_2740,
    // __security_check_cookie
    calls: &[(0x158, Callee::Any)],
    imports: &[],
};

/// [`task`]'s state 10 when slot 313 says 0: the node `+0x4c`, the
/// scales `+0x68..+0x74`, the owned node `+0x1c4` and the particle node
/// `+0x410` appended, the state on. Returns the particle node's `eax`.
fn add_nodes<G: Game>(g: &mut G, img: &Image, fr: &Frame, r: u32) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let eax = 0;
    let parent = w(0x11c);
    let a = img.rel32_target(f + 0x121);
    crate::sb_misc::node_append(g, img, a, r + 0x4c, parent, eax);
    for (off, v) in [(0x68, ONE), (0x6c, ONE), (0x70, ONE), (0x74, 0x417c_0000)] {
        g.write::<u32>(r + off, v);
    }
    // push ecx (node_append keeps it); three pushes; the call
    let esp = fr.esp.wrapping_sub(4 + 0x1c + 8 + 0x14);
    let (x, y) = (g.read::<u32>(r + 0x1b4), g.read::<u32>(r + 0x1a0));
    for (i, v) in (0u32..).zip([y, x, w(0x143), r + 0x4c]) {
        g.write::<u32>(esp + 4 + 4 * i, v);
    }
    let owned = r + 0x1c4;
    let callee = Frame {
        at: img.rel32_target(f + 0x15c),
        esp,
        ecx: owned,
        edx: parent,
        ..*fr
    };
    crate::sa_misc::node_append_owned(g, img, &callee);
    let o = g.read::<u32>(w(0x161));
    let p = r + 0x410;
    let ebp = fr.esp.wrapping_sub(4);
    let rec = ebp.wrapping_sub(0x1c);
    g.write::<u32>(rec, p);
    g.write::<u32>(p, w(0x170));
    for (i, v) in (1u32..).zip([w(0x17d), owned, o, w(0x18a), 4]) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let eax = crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x196), rec);
    let s = g.read::<u32>(r + 0x1c);
    g.write::<u32>(r + 0x1c, s.wrapping_add(1));
    eax
}

/// The effect's task (0x1000a6c0, `(task)` on the record `r =
/// task[1]`, the timer `+0x18` counting down each call, the state
/// `+0x1c`): state 0 starts a 176-call wait (state 1), which then moves
/// on to 2; state 2 shrinks `+0x48` by `K` (operand `+0x95`) until it is
/// at most `L` (`+0x9d`; then state 10); state 10 calls slot 222 `(G_X,
/// 11, 20, 2)`, moves on, and when slot 313 `(2)` says 0 appends the node
/// `+0x4c` (state 11 retries slot 313) ([`node_append`](crate::sb_misc::node_append)), sets `+0x68..
/// +0x74` (1, 1, 1, 15.75), appends the owned node `+0x1c4`
/// ([`node_append_owned`](crate::sa_misc::node_append_owned)) and the
/// particle node `+0x410` from a local record at `ebp - 0x1c`
/// ([`particle_node`](crate::sb_nodes::particle_node)), state 12; state
/// 12 becomes 10000, which waits for bit 2 of `+0x4f8 & +0x3f8` and
/// takes the task's node off its ring (state 20000). Returns the last
/// value in `eax` (see the code).
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let tick = |g: &mut G| {
        let n = g.read::<u32>(r + 0x18);
        g.write::<u32>(r + 0x18, n.wrapping_sub(1));
    };
    let mut state = g.read::<u32>(r + 0x1c);
    if state == 12 {
        g.write::<u32>(r + 0x1c, 0x2710);
        state = 0x2710;
    }
    let eax = match state {
        0 | 1 => {
            if state == 0 {
                g.write::<u32>(r + 0x18, 0xb0);
                g.write::<u32>(r + 0x1c, 1);
            }
            if g.read::<i32>(r + 0x18) <= 0 {
                let s = g.read::<u32>(r + 0x1c);
                g.write::<u32>(r + 0x1c, s.wrapping_add(1));
            }
            1
        }
        2 => {
            let x = mul(g.read::<f32>(r + 0x48), g.read::<f32>(w(0x95)));
            let l = g.read::<f32>(w(0x9d));
            g.write::<f32>(r + 0x48, x);
            // comiss L, x; jb: below or unordered
            if !(l < x || l.is_nan() || x.is_nan()) {
                g.write::<u32>(r + 0x1c, 10);
            }
            2
        }
        10 | 11 => {
            if state == 10 {
                let t = slot_target(g, w(0xe1), 222);
                g.call(t, 0, &[w(0xf2), 0xb, 0x14, 2]);
                let s = g.read::<u32>(r + 0x1c);
                g.write::<u32>(r + 0x1c, s.wrapping_add(1));
            }
            let t = slot_target(g, w(0xff), 313);
            let eax = g.call(t, 0, &[2]).eax;
            if eax != 0 {
                tick(g);
                return Ret::eax(eax);
            }
            add_nodes(g, img, fr, r)
        }
        0x2710 => {
            let bits = g.read::<u32>(r + 0x4f8) & g.read::<u32>(r + 0x3f8);
            if bits & 4 == 0 {
                bits
            } else {
                let node = g.read::<u32>(task);
                let next = g.read::<u32>(node + 4);
                let prev = g.read::<u32>(node + 8);
                g.write::<u32>(next + 8, prev);
                let prev = g.read::<u32>(node + 8);
                let next = g.read::<u32>(node + 4);
                g.write::<u32>(prev + 4, next);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                tick(g);
                g.write::<u32>(r + 0x1c, 0x4e20);
                return Ret::eax(next);
            }
        }
        s if s <= 10 => s,
        s => s.wrapping_sub(0x2710),
    };
    tick(g);
    Ret::eax(eax)
}

/// [`task`] (422 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 422,
    hash: 0x54e3_293a_82d5_50ff,
    calls: &[
        (0x121, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x15c, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x196, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// The summon's exit task (0x1000aac0, `(task)`, a task step) on the
/// record `r = task[1]` and its object `o = r[0x20]`, by the state
/// `+0x1c` (the wait `+0x18`); each motion is slot 228 `(o[0x50], id)`
/// and the next state:
/// - 0 and 51: once the motion is done (slot 411 `(o[0x50])`), motion
///   0x18; 1 and 52: once done, motion 0x19; 50: motion 0x40;
/// - 10000: slot 239 `(*G_A, o[0x50], 2)`, slot 388 `(o[0x50], 15, 1)`,
///   then at once 10001: once done, slot 213 `(o[0x50], 0)` and `(o[0x50],
///   5)`, the task's node leaves its ring, state 20000.
///
/// The wait goes down at every exit. Returns the last call's `eax`, the
/// node's next after the unlink, or what the original's dispatch left in
/// `eax`.
pub fn exit_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let model = |g: &G| {
        let o = g.read::<u32>(r + 0x20);
        g.read::<u32>(o + 0x50)
    };
    let done = |g: &mut G, op: u32| {
        let t = slot_target(g, w(op), 411);
        let m = model(g);
        g.call(t, 0, &[m]).eax
    };
    let play = |g: &mut G, id: u32| {
        let m = model(g);
        let t = slot_target(g, w(0x13e), 228);
        let e = g.call(t, 0, &[m, id]).eax;
        let s = g.read::<u32>(r + 0x1c);
        g.write::<u32>(r + 0x1c, s.wrapping_add(1));
        e
    };
    let state = g.read::<u32>(r + 0x1c);
    let eax = match state {
        0 | 0x33 => {
            let d = done(g, 0x38);
            if d == 0 { d } else { play(g, 0x18) }
        }
        1 | 0x34 => {
            let d = done(g, 0x122);
            if d == 0 { d } else { play(g, 0x19) }
        }
        0x32 => play(g, 0x40),
        0x2710 | 0x2711 => {
            if state == 0x2710 {
                let m = model(g);
                let t = slot_target(g, w(0x79), 239);
                let a = g.read::<u32>(w(0x7f));
                let mut stack = vec![a, m, 2];
                g.call(t, 0, &stack);
                let m = model(g);
                let t = slot_target(g, w(0x96), 388);
                stack.splice(0..0, [m, 0xf, 1]);
                // call width: 6
                g.call(t, 0, &stack);
                let s = g.read::<u32>(r + 0x1c);
                g.write::<u32>(r + 0x1c, s.wrapping_add(1));
            }
            let d = done(g, 0xaf);
            if d == 0 {
                d
            } else {
                let m = model(g);
                let t = slot_target(g, w(0xcf), 213);
                let mut stack = vec![m, 0];
                g.call(t, 0, &stack);
                let m = model(g);
                let t = slot_target(g, w(0xe4), 213);
                stack.splice(0..0, [m, 5]);
                g.call(t, 0, &stack);
                let next = unlink_task(g, task);
                let n = g.read::<u32>(r + 0x18);
                g.write::<u32>(r + 0x18, n.wrapping_sub(1));
                g.write::<u32>(r + 0x1c, 0x4e20);
                return Ret::eax(next);
            }
        }
        // the dispatch's remainders
        2..=0x31 => state - 0x32,
        s => s.wrapping_sub(0x2711),
    };
    let n = g.read::<u32>(r + 0x18);
    g.write::<u32>(r + 0x18, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`exit_task`] (343 bytes, 1 overlay).
pub const EXIT_TASK: Sig = Sig {
    len: 343,
    hash: 0x363c_adc8_8d44_9f88,
    calls: &[],
    imports: &[],
};

/// Sets up a spark object (0x1000b3a0, internal: `ecx` = the object,
/// `edx` = the source, three stack words `i, v, c`; a /GS frame): the
/// object joins the DLL's ring ([`ring_join`], its step at `+0x26`);
/// slot 236 `(src[0x54], T[i], obj + 0x34)` (`T` operand `+0x56`), slot
/// 234 `(&L, obj + 0x64, v)` and slot 232 `(&L, &L)` the direction `L`
/// (`ebp - 0x14`), slot 233 `(obj + 0xb4, &L, 5.0)`; `+0xb8` down by `K`,
/// `+0xc0..+0xcc` 0; three random offsets (slot 324, `& 0x7ff` less
/// 0x400) into `+0xd4..+0xdc`; `+0x30` = `c`, `+0x24` 1.0, `+0x1c` =
/// `i`, `+0x2c` the source, `+0x20` 0. Returns the source.
pub fn spark_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.ecx;
    let src = fr.edx;
    let v = fr.arg(g, 1);
    let i = fr.arg(g, 0);
    ring_join(g, img, f, obj, [0x1f, 0x26, 0x2e, 0x38, 0x3d, 0x4a, 0x61]);
    let tab = g.read::<u32>(w(0x56).wrapping_add(i.wrapping_mul(4)));
    let t = slot_target(g, w(0x5b), 236);
    let h = g.read::<u32>(src + 0x54);
    let mut stack = vec![h, tab, obj + 0x34];
    g.call(t, 0, &stack);
    let l = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
    let t = slot_target(g, w(0x7d), 234);
    stack.splice(0..0, [l, obj + 0x64, v]);
    // call width: 6
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x8f), 232);
    stack.splice(0..0, [l, l]);
    // call width: 8
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0xb1), 233);
    g.call(t, 0, &[obj + 0xb4, l, 0x40a0_0000]);
    let x = sub(g.read::<f32>(obj + 0xb8), g.read::<f32>(w(0xc9)));
    for o in [0xc0u32, 0xc4, 0xc8] {
        g.write::<u32>(obj + o, 0);
    }
    g.write::<f32>(obj + 0xb8, x);
    g.write::<u32>(obj + 0xcc, 0);
    for (op, dst) in [(0xceu32, 0xd4u32), (0x11e, 0xd8), (0x13b, 0xdc)] {
        let t = slot_target(g, w(op), 324);
        let r = g.call(t, 0, &[]).eax;
        g.write::<u32>(obj + dst, (r & 0x7ff).wrapping_sub(0x400));
    }
    let c = fr.arg(g, 2);
    g.write::<u32>(obj + 0x30, c);
    g.write::<u32>(obj + 0x24, 0x3f80_0000);
    g.write::<u32>(obj + 0x1c, i);
    g.write::<u32>(obj + 0x2c, src);
    g.write::<u32>(obj + 0x20, 0);
    src
}

/// [`spark_setup`] (389 bytes, 1 overlay).
pub const SPARK_SETUP: Sig = Sig {
    len: 389,
    hash: 0x1fab_9d18_978f_d63f,
    calls: &[
        // __security_check_cookie
        (0x17d, Callee::Any),
    ],
    imports: &[],
};

/// The spark's task (0x1000b190, `(task)`, a task step; a /GS frame) on
/// the spark `r = task[1]` and its source `o = r[0x2c]`, by the state
/// `+0x20` (a wait `+0x18`, the alpha `+0x28`, the speed `+0x24`):
/// - 0: a wait of `40 + rand % 20` (slot 324), state 1; 1: when the wait
///   is out, state 2 and the alpha 1.0;
/// - 2: the alpha down by `K`; at 0 or below it is 0 and state 10000;
///   either way slot 21 `(o[0x54], C[r[0x1c]], alpha)` (`C` operand
///   `+0xa0`);
/// - 10000: the task's node leaves its ring, state 20000.
///
/// Every step then moves the spark: `+0xb8` up by `K2`, slot 233
/// `(&L, r + 0xb4, speed)` and slot 248 `(r + 0x64, r + 0x64, &L)` (`L`
/// at `ebp - 0x14`); the three angles `+0xc4..+0xcc` up by the steps
/// `+0xd4..+0xdc` times the speed (truncated); the speed times `K4` while
/// at least `K3`; the DLL's matrix function (the pointer at operand
/// `+0x1a7`, `(&M, r + 0xc4)`, `ecx` the angles) builds `M` (`ebp -
/// 0x54`), slot 231 `(&M, r + 0x34, &M)` and `(r + 0x74, r[0x30], &M)`
/// place it and slot 9 `(o[0x54], D[r[0x1c]], r + 0x74)` (`D` operand
/// `+0x1e3`) sets the model's matrix. The wait goes down. Returns slot
/// 9's `eax`.
pub fn spark_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let state = g.read::<u32>(r + 0x20);
    let next = |g: &mut G| {
        let s = g.read::<u32>(r + 0x20);
        g.write::<u32>(r + 0x20, s.wrapping_add(1));
    };
    match state {
        0 => {
            let t = slot_target(g, w(0x34), 324);
            let x = g.call(t, 0, &[]).eax.cast_signed();
            next(g);
            g.write::<u32>(r + 0x18, (x % 20 + 0x28).cast_unsigned());
        }
        1 if g.read::<i32>(r + 0x18) <= 0 => {
            next(g);
            g.write::<u32>(r + 0x28, 0x3f80_0000);
        }
        2 => {
            let a = sub(g.read::<f32>(r + 0x28), k(g, 0x70));
            g.write::<f32>(r + 0x28, a);
            // comiss + jb: only an ordered "not above 0" ends it
            if 0.0 >= a {
                g.write::<u32>(r + 0x28, 0);
                g.write::<u32>(r + 0x20, 0x2710);
            }
            let idx = g.read::<u32>(r + 0x1c);
            let alpha = g.read::<u32>(r + 0x28);
            let c = g.read::<u32>(w(0xa0).wrapping_add(idx.wrapping_mul(4)));
            let o = g.read::<u32>(r + 0x2c);
            let h = g.read::<u32>(o + 0x54);
            let t = slot_target(g, w(0xab), 21);
            g.call(t, 0, &[h, c, alpha]);
        }
        0x2710 => {
            unlink_task(g, task);
            g.write::<u32>(r + 0x20, 0x4e20);
        }
        _ => {}
    }
    let y = add(g.read::<f32>(r + 0xb8), k(g, 0xed));
    let l = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
    let m = fr.esp.wrapping_sub(4).wrapping_sub(0x54);
    g.write::<f32>(r + 0xb8, y);
    let sp = g.read::<u32>(r + 0x24);
    let t = slot_target(g, w(0x110), 233);
    let mut stack = vec![l, r + 0xb4, sp];
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x126), 248);
    stack.splice(0..0, [r + 0x64, r + 0x64, l]);
    // call width: 6
    g.call(t, 0, &stack);
    for (step, at) in [(0xd4u32, 0xc4u32), (0xd8, 0xc8), (0xdc, 0xcc)] {
        let x = trunc(mul(cvt(g.read::<u32>(r + step)), g.read::<f32>(r + 0x24)));
        let v = g.read::<u32>(r + at).wrapping_add(x);
        g.write::<u32>(r + at, v);
    }
    let sp = g.read::<f32>(r + 0x24);
    if sp >= k(g, 0x18d) {
        g.write::<f32>(r + 0x24, mul(sp, k(g, 0x197)));
    }
    let angles = r + 0xc4;
    let build = g.read::<u32>(w(0x1a7));
    let mut stack = vec![m, angles];
    g.call(build, angles, &stack);
    let t = slot_target(g, w(0x1b8), 231);
    stack.splice(0..0, [m, r + 0x34, m]);
    // call width: 5
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x1cc), 231);
    let p = g.read::<u32>(r + 0x30);
    stack.splice(0..0, [r + 0x74, p, m]);
    // call width: 8
    g.call(t, 0, &stack);
    let idx = g.read::<u32>(r + 0x1c);
    let d = g.read::<u32>(w(0x1e3).wrapping_add(idx.wrapping_mul(4)));
    let o = g.read::<u32>(r + 0x2c);
    let h = g.read::<u32>(o + 0x54);
    let t = slot_target(g, w(0x1ee), 9);
    stack.splice(0..0, [h, d, r + 0x74]);
    // call width: 11
    let e = g.call(t, 0, &stack).eax;
    let n = g.read::<u32>(r + 0x18);
    g.write::<u32>(r + 0x18, n.wrapping_sub(1));
    Ret::eax(e)
}

/// [`spark_task`] (525 bytes, 1 overlay).
pub const SPARK_TASK: Sig = Sig {
    len: 525,
    hash: 0x9603_0d2c_5d66_db3d,
    calls: &[
        // __security_check_cookie
        (0x205, Callee::Any),
    ],
    imports: &[],
};

/// A light node set up from a group (0x10001d20, internal: `ecx` =
/// `[node, parent]`; a /GS frame): the node's own list fields and, with a
/// parent, the node appended to the parent's child list; slot 331
/// `(node + 0xbc)`; slot 558 fills four integers `I` on the frame (`ebp -
/// 0x14`), which become the floats `+0x7c..+0x88`; the 4x4 block `+0x1c`
/// gets 1.0 in rows 0, 1, 3 and 4 and 0.0 in row 2 (`+0x3c..`) and the
/// last (`+0x6c..`); slot 353 `(node + 0x1c)` and slot 271 `(4)`, each
/// seeing the earlier calls' words; `+0xe4` 1, `+0xd4` 1.0, `+0xd8`
/// 0.005, `+0xd0` 1.0. Returns slot 271's `eax`.
pub fn light_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let node = g.read::<u32>(fr.ecx);
    let parent = g.read::<u32>(fr.ecx + 4);
    crate::l3_common::node_init(g, node, parent);
    if parent != 0 {
        crate::l3_common::attach(g, node, parent, crate::l3_common::ListAt::of(parent));
    }
    let t = slot_target(g, w(0x50), 331);
    let mut stack = vec![node + 0xbc];
    g.call(t, 0, &stack);
    let ints = fr.esp.wrapping_sub(4).wrapping_sub(0x14);
    let t = slot_target(g, w(0x61), 558);
    stack.insert(0, ints);
    // call width: 18 at most (the earlier calls' words)
    g.call(t, 0, &stack);
    for k in 0..4u32 {
        let x = cvt(g.read::<u32>(ints + 4 * k));
        g.write::<f32>(node + 0x7c + 4 * k, x);
        // column k of the block: rows 0, 1, 3, 4 1.0, rows 2 and 5 0.0
        for (row, v) in [
            (0u32, 0x3f80_0000u32),
            (5, 0),
            (2, 0),
            (3, 0x3f80_0000),
            (4, 0x3f80_0000),
            (1, 0x3f80_0000),
        ] {
            g.write::<u32>(node + 0x1c + 0x10 * row + 4 * k, v);
        }
    }
    let t = slot_target(g, w(0x124), 353);
    stack.insert(0, node + 0x1c);
    // call width: 18 at most (the earlier calls' words)
    g.call(t, 0, &stack);
    let t = slot_target(g, w(0x163), 271);
    stack.insert(0, 4);
    // call width: 18 at most (the earlier calls' words)
    let e = g.call(t, 0, &stack).eax;
    g.write::<u32>(node + 0xe4, 1);
    g.write::<u32>(node + 0xd4, 0x3f80_0000);
    g.write::<u32>(node + 0xd8, 0x3ba3_d70a);
    g.write::<u32>(node + 0xd0, 0x3f80_0000);
    e
}

/// [`light_node`] (427 bytes, 1 overlay).
pub const LIGHT_NODE: Sig = Sig {
    len: 427,
    hash: 0xc349_b5da_290d_184f,
    calls: &[
        // __security_check_cookie
        (0x1a3, Callee::Any),
    ],
    imports: &[],
};

/// [`draw_detached`] on the holder `arg` (0x100025d0, cdecl `(arg)`: `ecx`
/// = `arg`, a tail jump). Returns its `eax`.
pub fn draw_detached_arg<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let callee = Frame {
        at: img.rel32_target(fr.at + 0x8),
        ecx: fr.arg(g, 0),
        ..*fr
    };
    Ret::eax(draw_detached(g, img, &callee))
}

/// [`draw_detached_arg`] (12 bytes, 1 overlay).
pub const DRAW_DETACHED_ARG: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(0x8, Callee::Sig(&DRAW_DETACHED))],
    imports: &[],
};
