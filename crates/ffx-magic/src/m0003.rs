//! Functions of the `magic_0003` family (the summon overlays `0003` to
//! `0008`, `0042`, `0070`, `0071`, `0076`, `0077` and others in other
//! lists): a ring of effect nodes walked with a per-node callback, a
//! loading state machine and node set-up. Addresses are `magic_0003`'s
//! unless a comment names another DLL; constants in `.rdata` are read
//! through the instructions' operands like every other address.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Mem, Ret, Sig};

fn f32_at(g: &impl Mem, addr: u32) -> f32 {
    f32::from_bits(g.read::<u32>(addr))
}

/// The walk every ring user starts with (`this` = a cursor: `+0x0` the
/// current node, byte `+0x4` "moved", byte `+0x5` the pass number):
/// when the current node's pass byte `+0x1a` already holds this pass,
/// "not moved"; else each child of the node (the list at `+0x10`, linked
/// through `+0x8`) becomes current in turn and its handler
/// (`[[child] + 0x1c]`, cdecl `(cursor)`) runs; then the node is current
/// again, takes the pass number, and the cursor says "moved".
///
/// Returns the `eax` the walk leaves: `eax_in` (the caller's) or the last
/// handler's, with the low byte the last pass byte loaded.
fn walk<G: Game>(g: &mut G, this: u32, eax_in: u32) -> u32 {
    let node = g.read::<u32>(this);
    let pass = g.read::<u8>(node + 0x1a);
    if pass == g.read::<u8>(this + 5) {
        g.write::<u8>(this + 4, 0);
        return (eax_in & !0xff) | u32::from(pass);
    }
    let mut eax = eax_in;
    let mut child = g.read::<u32>(node + 0x10);
    if child != node {
        loop {
            g.write::<u32>(this, child);
            let cur = g.read::<u32>(this);
            child = g.read::<u32>(child + 8);
            let handler = g.read::<u32>(g.read::<u32>(cur) + 0x1c);
            eax = g.call(handler, this, &[this]).eax;
            if child == node {
                break;
            }
        }
    }
    let pass = g.read::<u8>(this + 5);
    g.write::<u32>(this, node);
    g.write::<u8>(node + 0x1a, pass);
    g.write::<u8>(this + 4, 1);
    (eax & !0xff) | u32::from(pass)
}

/// 0x10002a80 (internal; `ecx` = the cursor, `esp` the frame's entry
/// stack pointer): [`walk`], then the node's time: slot 601 with the
/// node's byte `+0x30` and a pointer to a local (holding `this` until the
/// slot writes it, from the frame's `push ecx`) gives `t`;
/// `n = (int)((t * K0 + K1) / K2)` (SSE, truncated) and the node's `+0x20`
/// becomes `+0x2c - n`. Returns `n`.
pub fn tick<G: Game>(g: &mut G, img: &Image, f: u32, this: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    // push ebp; push ecx: the local is [ebp - 4]
    let local = esp.wrapping_sub(8);
    g.write::<u32>(local, this);
    walk(g, this, 0);
    let node = g.read::<u32>(this);
    let which = u32::from(g.read::<u8>(node + 0x30));
    let slot601 = slot_target(g, w(0x4f), 601);
    g.call(slot601, 0, &[which, local]);
    let t = f32_at(g, local) * f32_at(g, w(0x64));
    let node = g.read::<u32>(this);
    let t = t + f32_at(g, w(0x71));
    let base = g.read::<u32>(node + 0x2c);
    let n = ffx_bridge::x87::cvttsd2si(f64::from(t / f32_at(g, w(0x7e))));
    g.write::<u32>(node + 0x20, base.wrapping_sub(n));
    n
}

/// `tick` (143 bytes; 32 overlays).
pub const TICK: Sig = Sig {
    len: 143,
    hash: 0x1b98_37f6_c9e5_2dc3,
    calls: &[],
    imports: &[],
};

/// 0x10002b10 (a node handler, cdecl `(cursor)`): `ecx` = the cursor and
/// a tail jump into [`tick`] (its frame starts where this one's did).
pub fn tick_handler<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.arg(g, 0);
    let n = tick(g, img, img.rel32_target(fr.at + 8), this, fr.esp);
    Ret::eax(n)
}

/// `tick_handler` (12 bytes; 102 overlays carry the shape, those whose
/// callee is [`TICK`] bind).
pub const TICK_HANDLER: Sig = Sig {
    len: 12,
    hash: 0x896f_4fde_1224_a07f,
    calls: &[(8, Callee::Sig(&TICK))],
    imports: &[],
};

/// 0x10001190 in `magic_0006` (a node handler, cdecl `(cursor)`):
/// [`walk`]; when the cursor moved, the node's position `+0x1c..+0x28`
/// (4 floats) follows: towards the target `+0x2c..` by the rate `+0x4c`
/// (`v += (target - v) * rate`) when `+0x54` is set; else, while the clock
/// `+0x50` is below `K`, by the velocity `+0x3c..` (`v += vel * rate`,
/// the clock advancing by the rate), at or past it (or NaN) snapping to
/// the target. Then slot 791 gets `v[i] * K1 + origin[i] - K` for the
/// first three (`+0x5c..`). Returns slot 791's `eax`, or the walk's.
pub fn follow<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let this = fr.arg(g, 0);
    let eax = walk(g, this, fr.eax);
    if g.read::<u8>(this + 4) & 1 == 0 {
        return Ret::eax(eax);
    }
    let n = g.read::<u32>(this);
    let k = f32_at(g, w(0x53));
    let rate = |g: &G| f32_at(g, n + 0x4c);
    if g.read::<u32>(n + 0x54) != 0 {
        for i in 0..4 {
            let v = f32_at(g, n + 0x1c + 4 * i);
            let to = f32_at(g, n + 0x2c + 4 * i);
            let x = (to - v) * rate(g) + f32_at(g, n + 0x1c + 4 * i);
            g.write::<u32>(n + 0x1c + 4 * i, x.to_bits());
        }
    } else if f32_at(g, n + 0x50) >= k {
        for i in 0..4 {
            let to = g.read::<u32>(n + 0x2c + 4 * i);
            g.write::<u32>(n + 0x1c + 4 * i, to);
        }
    } else {
        for i in 0..4 {
            let x = f32_at(g, n + 0x3c + 4 * i) * rate(g) + f32_at(g, n + 0x1c + 4 * i);
            g.write::<u32>(n + 0x1c + 4 * i, x.to_bits());
        }
        let clock = rate(g) + f32_at(g, n + 0x50);
        g.write::<u32>(n + 0x50, clock.to_bits());
    }
    let k1 = f32_at(g, w(0x14d));
    let slot791 = slot_target(g, w(0x15e), 791);
    let a2 = f32_at(g, n + 0x24) * k1 + f32_at(g, n + 0x64) - k;
    let a1 = f32_at(g, n + 0x20) * k1 + f32_at(g, n + 0x60) - k;
    let a0 = f32_at(g, n + 0x1c) * k1 + f32_at(g, n + 0x5c) - k;
    let r = g.call(slot791, 0, &[a0.to_bits(), a1.to_bits(), a2.to_bits()]);
    Ret::eax(r.eax)
}

/// `follow` (431 bytes; 11 overlays).
pub const FOLLOW: Sig = Sig {
    len: 431,
    hash: 0x1c6b_59b5_157b_857a,
    calls: &[],
    imports: &[],
};

/// 0x10006ab0 (a callback, no arguments): the loading steps, `G_STEP`
/// 0..3, each run until it is not ready: 0 waits for slot 225(2) to say
/// 0, 1 for slot 313(2), 2 for slot 549(2) and then starts slot 226
/// (`G_FILE`, 1); 3 waits for slot 227(`G_FILE`) to reach 1 and ends at
/// step 4. Returns 0.
pub fn load_steps<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let mut step = g.read::<u32>(w(0x01));
    loop {
        match step {
            0..=2 => {
                let (t, slot) = [(0x18, 225), (0x34, 313), (0x4c, 549)][step as usize];
                let s = slot_target(g, w(t), slot);
                if g.call(s, 0, &[2]).eax != 0 {
                    return Ret::eax(0);
                }
                if step == 2 {
                    let s = slot_target(g, w(0x62), 226);
                    let file = g.read::<u32>(w(0x6a));
                    g.call(s, 0, &[file, 1]);
                }
                step = g.read::<u32>(w(0x7a)).wrapping_add(1);
                g.write::<u32>(w(0x80), step);
                if step > 3 {
                    return Ret::eax(0);
                }
            }
            3 => {
                let s = slot_target(g, w(0x8d), 227);
                let file = g.read::<u32>(w(0x93));
                if g.call(s, 0, &[file]).eax.cast_signed() >= 1 {
                    let v = g.read::<u32>(w(0xa9));
                    g.write::<u32>(w(0xa9), v.wrapping_add(1));
                }
                return Ret::eax(0);
            }
            _ => return Ret::eax(0),
        }
    }
}

/// `load_steps` (176 bytes and its 4-entry jump table; 22 overlays).
pub const LOAD_STEPS: Sig = Sig {
    len: 192,
    hash: 0x93c3_a521_1509_b1db,
    calls: &[],
    imports: &[],
};

/// 0x100030f0 in `magic_0004` (internal; `ecx` = `[node, parent,
/// value]`): sets up a node: its handler table `+0x0`, pass byte
/// `+0x1a` 0xff (as a u16), an empty child list (`+0xc`, `+0x10` itself),
/// the parent `+0x14`; with a parent, appended to the parent's child list
/// (`+0x4`/`+0x8` links, the parent's `+0xc` first and `+0x10` last
/// child, u16 count `+0x18`); then scale `+0x28` 1.0, `+0x38` the value,
/// byte `+0x40` 1, `+0x3c` 6, `+0x2c..+0x34` 0. Returns the value.
pub fn node_init(g: &mut impl Mem, img: &Image, f: u32, args: u32) -> u32 {
    let node = g.read::<u32>(args);
    let parent = g.read::<u32>(args + 4);
    g.write::<u32>(node, img.word(f + 0x08));
    g.write::<u16>(node + 0x1a, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, parent);
    if parent != 0 {
        g.write::<u32>(node + 4, parent);
        let last = g.read::<u32>(parent + 0x10);
        g.write::<u32>(node + 8, last);
        let last = g.read::<u32>(parent + 0x10);
        if last == parent {
            g.write::<u32>(parent + 0xc, node);
        } else {
            g.write::<u32>(last + 4, node);
        }
        let count = g.read::<u16>(parent + 0x18);
        g.write::<u16>(parent + 0x18, count.wrapping_add(1));
        g.write::<u32>(parent + 0x10, node);
    }
    let value = g.read::<u32>(args + 8);
    g.write::<u32>(node + 0x28, 1.0f32.to_bits());
    g.write::<u32>(node + 0x38, value);
    g.write::<u8>(node + 0x40, 1);
    g.write::<u32>(node + 0x3c, 6);
    g.write::<u32>(node + 0x2c, 0);
    g.write::<u32>(node + 0x30, 0);
    g.write::<u32>(node + 0x34, 0);
    value
}

/// `node_init` (109 bytes; 10 overlays).
pub const NODE_INIT: Sig = Sig {
    len: 109,
    hash: 0x0df8_ba43_620a_39e9,
    calls: &[],
    imports: &[],
};

/// A static node put at the tail of a static list (the spawners repeat
/// it inline): the halfword `half` (the node's `+0x1a`, addressed through
/// its own operand) 0xff, an empty child list, the list at `ops[0]`, then
/// [`attach`](crate::l3_common::attach) with the list word `ops[1]`, the
/// tail read at `ops[2]` and `ops[3]`, the sentinel `ops[4]`, the first
/// `ops[5]`, the count `ops[6]` and the tail written at `ops[7]`. Every
/// entry is an operand offset in the function at `f`.
pub(crate) fn static_tail<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    half: u32,
    node: u32,
    ops: [u32; 8],
) {
    use crate::l3_common::{ListAt, attach};
    let w = |off: u32| img.word(f + off);
    g.write::<u16>(half, 0xff);
    g.write::<u32>(node + 0x10, node);
    g.write::<u32>(node + 0xc, node);
    g.write::<u32>(node + 0x14, w(ops[0]));
    let at = ListAt {
        last: w(ops[2]),
        last2: w(ops[3]),
        sentinel: w(ops[4]),
        first: w(ops[5]),
        count: w(ops[6]),
        last_w: w(ops[7]),
    };
    attach(g, node, w(ops[1]), at);
}

/// The state of a ring spawner past its shared head ([`spawn_head`]).
pub(crate) struct Spawn {
    /// The spawner's state block (`task[4]`).
    pub s: u32,
    /// The set-up record on the frame (`aligned - 0x18`).
    pub rec: u32,
    /// `esp` at the internal calls (at their return address).
    pub call_esp: u32,
}

impl Spawn {
    /// An internal call at `at` with the record in `ecx`.
    pub fn frame(&self, at: u32) -> Frame {
        Frame {
            at,
            esp: self.call_esp,
            ecx: self.rec,
            ..Frame::default()
        }
    }

    /// The ring index `s + 0x24`.
    pub fn index<G: Game>(&self, g: &G) -> u32 {
        g.read::<u32>(self.s + 0x24)
    }

    /// The clock's tick on the way out, returning `eax`.
    pub fn out<G: Game>(&self, g: &mut G, eax: u32) -> Ret {
        let c = g.read::<u32>(self.s + 0x28);
        g.write::<u32>(self.s + 0x28, c.wrapping_add(1));
        Ret::eax(eax)
    }
}

/// The head every ring spawner of the `magic_0003` family shares (a task
/// step, cdecl `(task)`, an aligned frame of `sub esp, imm8` (`+0x0b`);
/// `S` = `task[4]`): `S + 0x1c` and `S + 0x20` grow by `K` (`+0x1b`,
/// `+0x33`), the clock `S + 0x28` ticks once on entry (and once on every
/// way out), and the state `S + 0x18` picks:
/// - 0: the model node `M0` (`+0x86`) takes its vtable (`+0x8a`) and
///   [`model_nodes`](crate::s3_summon::model_nodes) (`rel32` at `+0x8f`)
///   builds it from a group on the frame `[M0, root, *A, 0, 0]` (`+0x67`,
///   `+0x6f`, `+0x5b`); then as 10;
/// - 10: state 11, the clock the immediate at `+0xa0`, the index
///   `S + 0x24` 0; then as 11;
/// - 11: once the clock passes the immediate at `+0xae`, the caller
///   spawns ring `index` (`Ok`); else nothing;
/// - 10000: the task's node `task[0]` leaves its ring (the neighbours
///   linked past it, the node pointing at itself);
/// - else nothing.
///
/// `Err` returns: the state for the idle paths, `model_nodes`' `eax`
/// after state 0 (the clock never passes the limit right after its
/// reset), the node's old `+0x4` after state 10000.
pub(crate) fn spawn_head<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Result<Spawn, Ret> {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 4);
    // push ebp; and esp, -8; sub esp, n; three pushes: the record at the
    // aligned frame - 0x18 in every variant, the calls below the pushes
    let aligned = fr.esp.wrapping_sub(4) & !7;
    let sub = w(0x0b) & 0xff;
    let sp = Spawn {
        s,
        rec: aligned.wrapping_sub(0x18),
        call_esp: aligned.wrapping_sub(sub + 12 + 4),
    };
    let k = f32_at(g, w(0x1b));
    let x = add(f32_at(g, s + 0x1c), k);
    let state = g.read::<u32>(s + 0x18);
    let c = g.read::<u32>(s + 0x28);
    g.write::<u32>(s + 0x28, c.wrapping_add(1));
    g.write::<u32>(s + 0x1c, x.to_bits());
    let k = f32_at(g, w(0x33));
    let y = add(f32_at(g, s + 0x20), k);
    g.write::<u32>(s + 0x20, y.to_bits());
    let mut eax = state;
    match state {
        0 | 10 | 11 => {}
        10_000 => {
            let n = g.read::<u32>(task);
            let next = g.read::<u32>(n + 4);
            g.write::<u32>(next + 8, g.read::<u32>(n + 8));
            let prev = g.read::<u32>(n + 8);
            eax = g.read::<u32>(n + 4);
            g.write::<u32>(prev + 4, eax);
            g.write::<u32>(n + 8, n);
            g.write::<u32>(n + 4, n);
            return Err(sp.out(g, eax));
        }
        _ => return Err(sp.out(g, eax)),
    }
    if state == 0 {
        let a = g.read::<u32>(w(0x5b));
        for (i, v) in (0u32..).zip([w(0x67), w(0x6f), a, 0]) {
            g.write::<u32>(sp.rec + 4 * i, v);
        }
        g.write::<u8>(sp.rec + 0x10, 0);
        g.write::<u32>(w(0x86), w(0x8a));
        let at = img.rel32_target(f + 0x8f);
        eax = crate::s3_summon::model_nodes(g, img, &sp.frame(at));
        g.write::<u32>(s + 0x18, 10);
    }
    if state != 11 {
        let st = g.read::<u32>(s + 0x18);
        g.write::<u32>(s + 0x18, st.wrapping_add(1));
        g.write::<u32>(s + 0x28, w(0xa0));
        g.write::<u32>(s + 0x24, 0);
    }
    let limit = crate::o4_parts::disp8(img, f + 0xae).cast_signed();
    if g.read::<i32>(s + 0x28) <= limit {
        return Err(sp.out(g, eax));
    }
    Ok(sp)
}

/// Operand offsets of the spawners' shared tail ([`spawn_tail`]), in the
/// order the code reaches them.
pub(crate) struct Tail {
    /// `rel32` of `node insert`.
    pub insert: u32,
    /// The pointer global `A` (`mov eax, ds:A`).
    pub a: u32,
    /// The particle node's vtable.
    pub vt_e: u32,
    /// The particle record's list, part (`+0x4`, `+0x10`).
    pub p: u32,
    pub q: u32,
    /// `rel32` of the particle node.
    pub part: u32,
    /// The effect record's list, node base (`+ 0x38 * index`), words.
    pub root: u32,
    pub fx: u32,
    pub r: u32,
    pub t: u32,
    /// `rel32` of the effect node.
    pub effect: u32,
    /// The effect node's vtable, its flag words, the ring count.
    pub vt_fx: u32,
    pub flags: u32,
    pub flags2: u32,
    pub count: u32,
}

/// The tail every ring spawner shares, given the ring's group `B` and
/// particle node `E`: `node insert` of the record `insert` (`[B, root,
/// .., ..]`, written to the frame first), `B + 0x148` = 1, the particle
/// node `E` (vtable) from `[E, P, B, *A, Q, 0]`, the effect node
/// (`fx + 0x38 * index`) from `[fx, root, B, R, T, 0]` and its vtable;
/// the effect's flag word gets bit 0 and the next word 0; the index goes
/// up and reaching the count sets state 10000; the clock restarts (0,
/// then the way out's tick). Returns the new index.
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_tail<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    sp: &Spawn,
    insert: [u32; 6],
    b: u32,
    e: u32,
    t: &Tail,
) -> Ret {
    let w = |off: u32| img.word(f + off);
    let rec = sp.rec;
    for (k, v) in (0u32..).zip(insert) {
        g.write::<u32>(rec + 4 * k, v);
    }
    crate::nodes::node_insert(g, img, &sp.frame(img.rel32_target(f + t.insert)));
    let a = g.read::<u32>(w(t.a));
    g.write::<u8>(b + 0x148, 1);
    g.write::<u32>(rec, e);
    g.write::<u32>(e, w(t.vt_e));
    for (k, v) in (1u32..).zip([w(t.p), b, a, w(t.q), 0]) {
        g.write::<u32>(rec + 4 * k, v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + t.part), rec);
    let i = sp.index(g);
    let fx = w(t.fx).wrapping_add(i.wrapping_mul(56));
    for (k, v) in (0u32..).zip([fx, w(t.root), b, w(t.r), w(t.t), 0]) {
        g.write::<u32>(rec + 4 * k, v);
    }
    crate::sb_nodes::effect_node(g, img, img.rel32_target(f + t.effect), rec);
    let fx = g.read::<u32>(rec);
    g.write::<u32>(fx, w(t.vt_fx));
    let i = sp.index(g).wrapping_mul(56);
    let flags = g.read::<u32>(w(t.flags).wrapping_add(i));
    g.write::<u32>(w(t.flags).wrapping_add(i), flags | 1);
    g.write::<u32>(w(t.flags2).wrapping_add(i), 0);
    let i = sp.index(g).wrapping_add(1);
    g.write::<u32>(sp.s + 0x24, i);
    if i.cast_signed() >= g.read::<i32>(w(t.count)) {
        g.write::<u32>(sp.s + 0x18, 10_000);
    }
    g.write::<u32>(sp.s + 0x28, 0);
    sp.out(g, i)
}

/// 0x10006540 (a task step, cdecl `(task)`): `magic_0003`'s ring
/// spawner. [`spawn_head`]; ring `i`: two static nodes `N1`
/// (`+0xd9 + 0x2c i`, its halfword through `+0xd1`) and `N2`
/// (`+0xdf + 0x2c i`) appended to the root list ([`static_tail`], their
/// vtables `+0x136`, `+0x1c6`); slot 268 with `(model[0x54], 0x11,
/// N1 + 0x1c)` (`model` = `+0x147 + 0x98 i`), `N1 + 0x28` = 1.0,
/// `N1 + 0x20 += K2` (`+0x175`); `N2`'s scale `+0x1c..+0x24` 1.0 and
/// `+0x28` = slot 251's float `(model[0x50])` clamped to `0..K3` (100.0
/// above it; `+0x22d`) times `K4` (`+0x243`); then [`spawn_tail`] with
/// `B` = `+0xc1 + 0x14c i`, `E` = `+0x109 + 0x100 i` and the insert
/// record `[B, root, X, N1, Y, N2]`.
pub fn spawn_rings<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let sp = match spawn_head(g, img, fr) {
        Ok(sp) => sp,
        Err(r) => return r,
    };
    let i = sp.index(g);
    let b = w(0xc1).wrapping_add(i.wrapping_mul(0x14c));
    let n1 = w(0xd9).wrapping_add(i.wrapping_mul(0x2c));
    let n2 = w(0xdf).wrapping_add(i.wrapping_mul(0x2c));
    let half = w(0xd1).wrapping_add(i.wrapping_mul(0x2c));
    let e = (i << 8).wrapping_add(w(0x109));
    static_tail(
        g,
        img,
        f,
        half,
        n1,
        [0xec, 0xf3, 0xf8, 0x103, 0x116, 0x11e, 0x12a, 0x130],
    );
    g.write::<u32>(n1, w(0x136));
    let m = sp.index(g).wrapping_mul(0x98).wrapping_add(w(0x147));
    let s268 = slot_target(g, w(0x152), 0x430 / 4);
    g.call(s268, n1, &[g.read::<u32>(m + 0x54), 0x11, n1 + 0x1c]);
    g.write::<u32>(n1 + 0x28, 1.0f32.to_bits());
    let z = add(f32_at(g, n1 + 0x20), f32_at(g, w(0x175)));
    g.write::<u32>(n1 + 0x20, z.to_bits());
    static_tail(
        g,
        img,
        f,
        n2 + 0x1a,
        n2,
        [0x18d, 0x194, 0x199, 0x1a1, 0x1a6, 0x1ae, 0x1ba, 0x1c0],
    );
    g.write::<u32>(n2, w(0x1c6));
    let m = sp.index(g).wrapping_mul(0x98).wrapping_add(w(0x1d4));
    for off in [0x1c, 0x20, 0x24] {
        g.write::<u32>(n2 + off, 1.0f32.to_bits());
    }
    let s251 = slot_target(g, w(0x1f8), 0x3ec / 4);
    let v = g
        .call(s251, 0, &[g.read::<u32>(m + 0x50)])
        .st0
        .unwrap_or(0.0) as f32;
    // comiss: NaN keeps its value through both clamps
    g.write::<u32>(n2 + 0x28, v.to_bits());
    if 0.0 > v {
        g.write::<u32>(n2 + 0x28, 0);
    }
    if f32_at(g, n2 + 0x28) > f32_at(g, w(0x22d)) {
        g.write::<u32>(n2 + 0x28, 100.0f32.to_bits());
    }
    let scale = mul(f32_at(g, n2 + 0x28), f32_at(g, w(0x243)));
    g.write::<u32>(n2 + 0x28, scale.to_bits());
    let insert = [b, w(0x25c), w(0x264), n1, w(0x270), n2];
    let tail = Tail {
        insert: 0x279,
        a: 0x286,
        vt_e: 0x297,
        p: 0x2a3,
        q: 0x2b3,
        part: 0x2c0,
        root: 0x2cb,
        fx: 0x2df,
        r: 0x2ef,
        t: 0x2f7,
        effect: 0x304,
        vt_fx: 0x30e,
        flags: 0x321,
        flags2: 0x329,
        count: 0x339,
    };
    spawn_tail(g, img, f, &sp, insert, b, e, &tail)
}

/// `spawn_rings` (898 bytes, 1 overlay).
pub const SPAWN_RINGS: Sig = Sig {
    len: 898,
    hash: 0x2076_27f1_51c3_69b3,
    calls: &[
        (0x8f, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x279, Callee::Sig(&crate::nodes::INSERT)),
        (0x2c0, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x304, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// The middle the `magic_0004`-style spawners share after [`spawn_head`]:
/// a node `D` (`+0xc4 + 0x44 i`) set up by
/// [`node_init`] (`rel32` at `+0x112`) from `[D, root, model]`
/// (`+0x10d`, `model` = `+0xe0 + 0x98 i`), its byte `+0x40` 3 and `+0x30`
/// the immediate at `+0x121`; the static node `N` (`+0xfd + 0x2c i`)
/// appended to the root list ([`static_tail`], operands from `+0x134`,
/// its vtable `+0x16d`), its scale `+0x1c..+0x24` 1.0 and `+0x28` slot
/// 251's float `(model[0x50])` raised to 1.0 below `K1` (`+0x1b9`),
/// limited to `K2` (`+0x1d9`, the immediate at `+0x1e2` above it) and
/// times `K3` (`+0x1ef`). Returns `(B, D, N, E)`: the group
/// (`+0xf3 + 0x14c i`), the two nodes and the particle node
/// (`+0xe5 + 0x100 i`).
fn spawn_node_middle<G: Game>(g: &mut G, img: &Image, f: u32, sp: &Spawn) -> [u32; 4] {
    let w = |off: u32| img.word(f + off);
    let i = sp.index(g);
    let d = w(0xc4).wrapping_add(i.wrapping_mul(0x44));
    let b = w(0xf3).wrapping_add(i.wrapping_mul(0x14c));
    let n = w(0xfd).wrapping_add(i.wrapping_mul(0x2c));
    let model = w(0xe0).wrapping_add(i.wrapping_mul(0x98));
    let e = (i << 8).wrapping_add(w(0xe5));
    for (k, v) in (0u32..).zip([d, w(0x10d), model]) {
        g.write::<u32>(sp.rec + 4 * k, v);
    }
    node_init(g, img, img.rel32_target(f + 0x112), sp.rec);
    g.write::<u8>(d + 0x40, 3);
    g.write::<u32>(d + 0x30, w(0x121));
    static_tail(
        g,
        img,
        f,
        n + 0x1a,
        n,
        [0x134, 0x13b, 0x140, 0x148, 0x14d, 0x155, 0x161, 0x167],
    );
    g.write::<u32>(n, w(0x16d));
    scale_251(g, img, f, sp, n, [0x17b, 0x19f, 0x1b9, 0x1d9, 0x1e2, 0x1ef]);
    [b, d, n, e]
}

/// 0x100065b0 in `magic_0004` and `magic_0005` (a task step, cdecl
/// `(task)`; the two differ in immediates only: the clock's limit and
/// `D`'s `+0x30`): [`spawn_head`], [`spawn_node_middle`], then
/// [`spawn_tail`] with the insert record `[B, root, X, D, Y, N]`.
pub fn spawn_node<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let sp = match spawn_head(g, img, fr) {
        Ok(sp) => sp,
        Err(r) => return r,
    };
    let [b, d, n, e] = spawn_node_middle(g, img, f, &sp);
    let insert = [b, w(0x208), w(0x210), d, w(0x21c), n];
    let tail = Tail {
        insert: 0x225,
        a: 0x22e,
        vt_e: 0x23f,
        p: 0x24b,
        q: 0x25b,
        part: 0x268,
        root: 0x273,
        fx: 0x287,
        r: 0x297,
        t: 0x29f,
        effect: 0x2ac,
        vt_fx: 0x2b6,
        flags: 0x2c9,
        flags2: 0x2d1,
        count: 0x2e1,
    };
    spawn_tail(g, img, f, &sp, insert, b, e, &tail)
}

const SPAWN_NODE_CALLS: &[(u32, Callee)] = &[
    (0x8f, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
    (0x112, Callee::Sig(&NODE_INIT)),
    (0x225, Callee::Sig(&crate::nodes::INSERT)),
    (0x268, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    (0x2ac, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
];

/// `spawn_node` in `magic_0004` (810 bytes).
pub const SPAWN_NODE_0004: Sig = Sig {
    len: 810,
    hash: 0x4312_b75a_8d3c_b594,
    calls: SPAWN_NODE_CALLS,
    imports: &[],
};

/// `spawn_node` in `magic_0005` (810 bytes).
pub const SPAWN_NODE_0005: Sig = Sig {
    len: 810,
    hash: 0x46dd_f174_9d49_1a36,
    calls: SPAWN_NODE_CALLS,
    imports: &[],
};

/// A static node's scale from slot 251 as the later spawners set it:
/// `+0x1c..+0x24` 1.0, `+0x28` = slot 251's float `(model[0x50])`
/// (`model` = `ops[0] + 0x98 * index`, the table global `ops[1]`), 1.0
/// below `K1` (`ops[2]`), the immediate at `ops[4]` above `K2`
/// (`ops[3]`), then times `K3` (`ops[5]`). `comiss`: a NaN keeps its
/// value through both limits.
fn scale_251<G: Game>(g: &mut G, img: &Image, f: u32, sp: &Spawn, n: u32, ops: [u32; 6]) {
    use crate::sb_sse::mul;
    let w = |off: u32| img.word(f + off);
    let m = sp.index(g).wrapping_mul(0x98).wrapping_add(w(ops[0]));
    for off in [0x1c, 0x20, 0x24] {
        g.write::<u32>(n + off, 1.0f32.to_bits());
    }
    let s251 = slot_target(g, w(ops[1]), 0x3ec / 4);
    let v = g
        .call(s251, 0, &[g.read::<u32>(m + 0x50)])
        .st0
        .unwrap_or(0.0) as f32;
    g.write::<u32>(n + 0x28, v.to_bits());
    if f32_at(g, w(ops[2])) > v {
        g.write::<u32>(n + 0x28, 1.0f32.to_bits());
    }
    if f32_at(g, n + 0x28) > f32_at(g, w(ops[3])) {
        g.write::<u32>(n + 0x28, w(ops[4]));
    }
    let scale = mul(f32_at(g, n + 0x28), f32_at(g, w(ops[5])));
    g.write::<u32>(n + 0x28, scale.to_bits());
}

/// 0x10006640 in `magic_0006` (a task step, cdecl `(task)`):
/// [`spawn_head`]; ring `i`: the node `D` (`+0xdb + 0x44 i`) set up by
/// [`node_init`] from `[D, root, model]` (`+0xd4`, `model` =
/// `+0x101 + 0x98 i`), its byte `+0x40` 3 and `+0x30` the immediate at
/// `+0x12a`; the static node `N1` (`+0xf0 + 0x2c i`, vtable `+0x130`)
/// appended to the root list, its facing into `N1 + 0x1c`
/// ([`facing_int`](crate::sa_misc::facing_int) on `+0x188 + 0x98 i`);
/// the static node `N2` (`+0xea + 0x2c i`, vtable `+0x1d9`) appended
/// with its scale from slot 251 ([`scale_251`]); then [`spawn_tail`]
/// with `B` = `+0xc1 + 0x14c i`, `E` = `+0x106 + 0x100 i` and the insert
/// record `[B, root, X, D, N1, N2]`.
pub fn spawn_facing<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let sp = match spawn_head(g, img, fr) {
        Ok(sp) => sp,
        Err(r) => return r,
    };
    let i = sp.index(g);
    let b = w(0xc1).wrapping_add(i.wrapping_mul(0x14c));
    let d = w(0xdb).wrapping_add(i.wrapping_mul(0x44));
    let n2 = w(0xea).wrapping_add(i.wrapping_mul(0x2c));
    let n1 = w(0xf0).wrapping_add(i.wrapping_mul(0x2c));
    let model = w(0x101).wrapping_add(i.wrapping_mul(0x98));
    let e = (i << 8).wrapping_add(w(0x106));
    for (k, v) in (0u32..).zip([d, w(0xd4), model]) {
        g.write::<u32>(sp.rec + 4 * k, v);
    }
    node_init(g, img, img.rel32_target(f + 0x11b), sp.rec);
    g.write::<u8>(d + 0x40, 3);
    g.write::<u32>(d + 0x30, w(0x12a));
    g.write::<u32>(n1, w(0x130));
    static_tail(
        g,
        img,
        f,
        n1 + 0x1a,
        n1,
        [0x143, 0x14a, 0x14f, 0x157, 0x15c, 0x164, 0x170, 0x176],
    );
    let obj = sp.index(g).wrapping_mul(0x98).wrapping_add(w(0x188));
    let facing = img.rel32_target(f + 0x18d);
    crate::sa_misc::facing_int(g, img, facing, obj, n1 + 0x1c, sp.call_esp);
    static_tail(
        g,
        img,
        f,
        n2 + 0x1a,
        n2,
        [0x1a0, 0x1a7, 0x1ac, 0x1b4, 0x1b9, 0x1c1, 0x1cd, 0x1d3],
    );
    g.write::<u32>(n2, w(0x1d9));
    scale_251(
        g,
        img,
        f,
        &sp,
        n2,
        [0x1e7, 0x20b, 0x225, 0x245, 0x24e, 0x25f],
    );
    let insert = [b, w(0x278), w(0x280), d, n1, n2];
    let tail = Tail {
        insert: 0x291,
        a: 0x29e,
        vt_e: 0x2af,
        p: 0x2bb,
        q: 0x2cb,
        part: 0x2d8,
        root: 0x2e3,
        fx: 0x2f7,
        r: 0x307,
        t: 0x30f,
        effect: 0x31c,
        vt_fx: 0x326,
        flags: 0x339,
        flags2: 0x341,
        count: 0x351,
    };
    spawn_tail(g, img, f, &sp, insert, b, e, &tail)
}

/// `spawn_facing` (922 bytes, 1 overlay).
pub const SPAWN_FACING: Sig = Sig {
    len: 922,
    hash: 0xc6a9_07fa_6129_4300,
    calls: &[
        (0x8f, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x11b, Callee::Sig(&NODE_INIT)),
        (0x18d, Callee::Sig(&crate::sa_misc::FACING_INT)),
        (0x291, Callee::Sig(&crate::nodes::INSERT)),
        (0x2d8, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x31c, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// The middle `magic_0007` and `magic_0008` share after [`spawn_head`]:
/// ring `i`'s static nodes `N1` (`+0xd9 + 0x2c i`, its halfword through
/// `+0xd1`, vtable `+0x132`) and `N2` (`+0xdf + 0x2c i`, vtable `+0x1ac`)
/// appended to the root list; between them, unless bit 0 of the model's
/// flags `+0x90` is set (`model` = `+0x140 + 0x98 i`), slot 665 with
/// `(model[0x50], N1 + 0x1c)`; `N2`'s scale from slot 251 limited (as
/// [`scale_251`] without the factor: model `+0x1ba`, table `+0x1de`,
/// limits `+0x1f8`, `+0x218`, the immediate `+0x221`). Returns
/// `[B, N1, N2, E, kind]`: the group (`+0xc1 + 0x14c i`), the nodes, the
/// particle node (`+0x109 + 0x100 i`) and slot 390's answer for
/// `[+0x230 + 0x98 i]` (table `+0x235`).
fn spawn_kind_middle<G: Game>(g: &mut G, img: &Image, f: u32, sp: &Spawn) -> [u32; 5] {
    let w = |off: u32| img.word(f + off);
    let i = sp.index(g);
    let b = w(0xc1).wrapping_add(i.wrapping_mul(0x14c));
    let half = w(0xd1).wrapping_add(i.wrapping_mul(0x2c));
    let n1 = w(0xd9).wrapping_add(i.wrapping_mul(0x2c));
    let n2 = w(0xdf).wrapping_add(i.wrapping_mul(0x2c));
    let e = (i << 8).wrapping_add(w(0x109));
    static_tail(
        g,
        img,
        f,
        half,
        n1,
        [0xec, 0xf3, 0xf8, 0x103, 0x112, 0x11a, 0x126, 0x12c],
    );
    g.write::<u32>(n1, w(0x132));
    let m = sp.index(g).wrapping_mul(0x98).wrapping_add(w(0x140));
    if g.read::<u8>(m + 0x90) & 1 == 0 {
        let s665 = slot_target(g, w(0x155), 0xa64 / 4);
        g.call(s665, n1 + 0x1c, &[g.read::<u32>(m + 0x50), n1 + 0x1c]);
    }
    static_tail(
        g,
        img,
        f,
        n2 + 0x1a,
        n2,
        [0x173, 0x17a, 0x17f, 0x187, 0x18c, 0x194, 0x1a0, 0x1a6],
    );
    g.write::<u32>(n2, w(0x1ac));
    limit_251(g, img, f, sp, n2, [0x1ba, 0x1de, 0x1f8, 0x218, 0x221]);
    let at = sp.index(g).wrapping_mul(0x98).wrapping_add(w(0x230));
    let s390 = slot_target(g, w(0x235), 0x618 / 4);
    let kind = g.call(s390, 0, &[g.read::<u32>(at)]).eax;
    [b, n1, n2, e, kind]
}

/// [`scale_251`]'s first part: the scale words and the limits, no
/// factor (`ops`: model base, table, `K1`, `K2`, the immediate).
fn limit_251<G: Game>(g: &mut G, img: &Image, f: u32, sp: &Spawn, n: u32, ops: [u32; 5]) {
    let w = |off: u32| img.word(f + off);
    let m = sp.index(g).wrapping_mul(0x98).wrapping_add(w(ops[0]));
    for off in [0x1c, 0x20, 0x24] {
        g.write::<u32>(n + off, 1.0f32.to_bits());
    }
    let s251 = slot_target(g, w(ops[1]), 0x3ec / 4);
    let v = g
        .call(s251, 0, &[g.read::<u32>(m + 0x50)])
        .st0
        .unwrap_or(0.0) as f32;
    g.write::<u32>(n + 0x28, v.to_bits());
    if f32_at(g, w(ops[2])) > v {
        g.write::<u32>(n + 0x28, 1.0f32.to_bits());
    }
    if f32_at(g, n + 0x28) > f32_at(g, w(ops[3])) {
        g.write::<u32>(n + 0x28, w(ops[4]));
    }
}

/// 0x10006540 in `magic_0007` (a task step, cdecl `(task)`):
/// [`spawn_head`], [`spawn_kind_middle`]; then by the kind `k` (through
/// the jump table at `+0x255` for `k - 0x3002` in `0..=5`, its targets
/// read from the table): `N1 + 0x20` moves by `s * A / B` and a constant
/// (`addss` in that order: `(s * A / B + N1[0x20]) ± C`), and `N2`'s scale
/// `s` is multiplied by a factor:
/// - `+0x259`: minus `C` (`+0x277`), factor `+0x289`;
/// - `+0x292`, `+0x2b6`, `+0x2df`: plus `C` (`+0x2b0`, `+0x2d4`,
///   `+0x2fd`), factor `+0x32b`;
/// - `+0x303` (and any other kind): no constant, factor `+0x32b`;
///
/// (`A`, `B` read through each case's own operands). Then [`spawn_tail`]
/// with the insert record `[B, root, X, N1, Y, N2]`.
pub fn spawn_kind_table<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let sp = match spawn_head(g, img, fr) {
        Ok(sp) => sp,
        Err(r) => return r,
    };
    let [b, n1, n2, e, kind] = spawn_kind_middle(g, img, f, &sp);
    let k = kind.wrapping_sub(0x3002);
    let case = if k <= 5 {
        img.word(w(0x255).wrapping_add(4 * k)).wrapping_sub(f)
    } else {
        0x303
    };
    // (A, B, C, minus, factor) operand offsets of each case
    let (a, bb, c, minus, factor) = match case {
        0x259 => (0x262, 0x26a, Some(0x277), true, 0x289),
        0x292 => (0x29b, 0x2a3, Some(0x2b0), false, 0x32b),
        0x2b6 => (0x2bf, 0x2c7, Some(0x2d4), false, 0x32b),
        0x2df => (0x2e8, 0x2f0, Some(0x2fd), false, 0x32b),
        _ => (0x30c, 0x314, None, false, 0x32b),
    };
    let s = f32_at(g, n2 + 0x28);
    let q = div(mul(s, f32_at(g, w(a))), f32_at(g, w(bb)));
    let mut y = add(q, f32_at(g, n1 + 0x20));
    if let Some(c) = c {
        let c = f32_at(g, w(c));
        y = if minus { sub(y, c) } else { add(y, c) };
    }
    g.write::<u32>(n1 + 0x20, y.to_bits());
    let s = mul(f32_at(g, n2 + 0x28), f32_at(g, w(factor)));
    g.write::<u32>(n2 + 0x28, s.to_bits());
    let insert = [b, w(0x344), w(0x34c), n1, w(0x358), n2];
    let tail = Tail {
        insert: 0x361,
        a: 0x36e,
        vt_e: 0x37f,
        p: 0x38b,
        q: 0x39b,
        part: 0x3a8,
        root: 0x3b3,
        fx: 0x3c7,
        r: 0x3d7,
        t: 0x3df,
        effect: 0x3ec,
        vt_fx: 0x3f6,
        flags: 0x409,
        flags2: 0x411,
        count: 0x421,
    };
    spawn_tail(g, img, f, &sp, insert, b, e, &tail)
}

/// `spawn_kind_table` (1,132 bytes and its 6-entry jump table, 1
/// overlay).
pub const SPAWN_KIND_TABLE: Sig = Sig {
    len: 1156,
    hash: 0x53c2_e6a3_1ae8_482a,
    calls: &[
        (0x8f, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x361, Callee::Sig(&crate::nodes::INSERT)),
        (0x3a8, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x3ec, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};

/// 0x10006540 in `magic_0008` (a task step, cdecl `(task)`):
/// [`spawn_head`], [`spawn_kind_middle`]; then by the kind `k`:
/// - 0x108c: slot 268 with `(model[0x54], 0x11, N1 + 0x1c)` (`model` =
///   `+0x261 + 0x98 i`, table `+0x26c`), `N1 + 0x28` = 1.0, then as 0x3007;
/// - 0x3007: `N1 + 0x20 += C1` (`+0x28b`), factor `+0x29d`;
/// - 0x3002, 0x3006: `N1 + 0x20 += C2` (`+0x2ac`) or `C3` (`+0x2d9`),
///   factor `+0x29d`;
/// - 0x3003: `N1 + 0x20 -= C4` (`+0x2e8`), factor `+0x2fa`;
/// - other kinds below 0x3002: factor `+0x29d`; above: `+0x2ca`.
///
/// `N2`'s scale times the factor is then held to `L1..L2` (`+0x30b`,
/// `+0x320`; the immediates at `+0x317` below, `+0x329` above; a NaN
/// stays), and [`spawn_tail`] runs with `[B, root, X, N1, Y, N2]`.
pub fn spawn_kind_branch<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let sp = match spawn_head(g, img, fr) {
        Ok(sp) => sp,
        Err(r) => return r,
    };
    let [b, n1, n2, e, kind] = spawn_kind_middle(g, img, f, &sp);
    let kind = kind.cast_signed();
    let add_n1 = |g: &mut G, c: u32, minus: bool| {
        let x = f32_at(g, n1 + 0x20);
        let c = f32_at(g, w(c));
        let y = if minus { sub(x, c) } else { add(x, c) };
        g.write::<u32>(n1 + 0x20, y.to_bits());
    };
    let factor = match kind {
        0x108c | 0x3007 => {
            if kind == 0x108c {
                let m = sp.index(g).wrapping_mul(0x98).wrapping_add(w(0x261));
                let s268 = slot_target(g, w(0x26c), 0x430 / 4);
                g.call(s268, n1 + 0x1c, &[g.read::<u32>(m + 0x54), 0x11, n1 + 0x1c]);
                g.write::<u32>(n1 + 0x28, 1.0f32.to_bits());
            }
            add_n1(g, 0x28b, false);
            0x29d
        }
        0x3002 => {
            add_n1(g, 0x2ac, false);
            0x29d
        }
        0x3006 => {
            add_n1(g, 0x2d9, false);
            0x29d
        }
        0x3003 => {
            add_n1(g, 0x2e8, true);
            0x2fa
        }
        k if k < 0x3002 => 0x29d,
        _ => 0x2ca,
    };
    let s = mul(f32_at(g, n2 + 0x28), f32_at(g, w(factor)));
    g.write::<u32>(n2 + 0x28, s.to_bits());
    // magic_0008's constants: the scale is 1.0..18.0 here and the factors
    // at most 0.014, so the upper limit (0.5) is never passed; both limits
    // are the values they store
    if f32_at(g, w(0x30b)) > s {
        g.write::<u32>(n2 + 0x28, w(0x317));
    } else if s > f32_at(g, w(0x320)) {
        g.write::<u32>(n2 + 0x28, w(0x329));
    }
    let insert = [b, w(0x33d), w(0x345), n1, w(0x351), n2];
    let tail = Tail {
        insert: 0x35a,
        a: 0x367,
        vt_e: 0x378,
        p: 0x384,
        q: 0x394,
        part: 0x3a1,
        root: 0x3ac,
        fx: 0x3c0,
        r: 0x3d0,
        t: 0x3d8,
        effect: 0x3e5,
        vt_fx: 0x3ef,
        flags: 0x402,
        flags2: 0x40a,
        count: 0x41a,
    };
    spawn_tail(g, img, f, &sp, insert, b, e, &tail)
}

/// `spawn_kind_branch` (1,123 bytes, 1 overlay).
pub const SPAWN_KIND_BRANCH: Sig = Sig {
    len: 1123,
    hash: 0x767c_15ed_4c92_c0b2,
    calls: &[
        (0x8f, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x35a, Callee::Sig(&crate::nodes::INSERT)),
        (0x3a1, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x3e5, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};
