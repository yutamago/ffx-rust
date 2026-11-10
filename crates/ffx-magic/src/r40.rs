//! Round 40's ports (own rows of the DLLs below `magic_0250`): copies of
//! ported functions bound by their own `Sig`, and the DLLs' own code.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `magic_0150`'s start-up (0x1000d240, 2986 bytes; a table entry, cdecl
/// `(actor)`): [`start_up_0603`](crate::r25::start_up_0603) with slot 215
/// asked right after slot 325 (its two words still on the stack), the code
/// from `magic_0603`'s `+0x2cd` on 3 bytes earlier, and `magic_0150`'s own
/// model start ([`model_start`](crate::m0150::model_start)) and set-up
/// ([`start_up`](crate::m0150::start_up)). Returns 0.
pub fn start_up_0150<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r25::start_up_0603_with(
        g,
        img,
        fr,
        &crate::r25::StartUp0603 {
            shift_from: 0x2cd,
            early_215: true,
            model: crate::m0150::model_start,
            setup: |g, img, at, esp| {
                let fr = Frame {
                    at,
                    esp,
                    ..Frame::default()
                };
                crate::m0150::start_up(g, img, &fr);
            },
        },
    )
}

/// [`start_up_0150`] (2986 bytes, 1 overlay).
pub const START_UP_0150: Sig = Sig {
    len: 2986,
    hash: 0xff3a_d1bd_1f38_1b91,
    calls: &[
        (0x19, Callee::Sig(&crate::s4_summon::TARGETS_INIT_C)),
        (0x2c4, Callee::Sig(&crate::s5_misc::TARGETS_MEAN_OFFSET)),
        (0x5b9, Callee::Sig(&crate::s8_misc::BONE_NODE)),
        (0x5ea, Callee::Sig(&crate::m0070::NODE_INIT_BIG)),
        (0xb3d, Callee::Sig(&crate::m0150::MODEL_START)),
        (0xb42, Callee::Sig(&crate::m0150::START_UP)),
        // __security_check_cookie
        (0xb65, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0150`'s node walk with its ring visit (0x1000a550, 113 bytes;
/// internal, `ecx` = the cursor): the walk
/// ([`walk`](crate::nodes::walk)); when it ran (the cursor's flag bit 0),
/// the ring `[O + 0x2ec]` gets its handler (`+0x1c` of its table, `ecx`
/// and the one argument a cursor on the frame: the ring, the walk's pass
/// byte at `+5`). `O` is `[ebp - 4]`, a frame word the function never
/// writes: `owner` (the port passes 0, the junk-read rule). Returns the
/// handler's `eax`, or the walk's.
pub fn walk_ring_junk_0150<G: Game>(g: &mut G, fr: &Frame, owner: u32) -> Ret {
    let cursor = fr.ecx;
    let (eax, _) = crate::nodes::walk(g, cursor, cursor, fr.eax);
    if g.read::<u8>(cursor + 4) & 1 == 0 {
        return Ret::eax(eax);
    }
    // push ebp; sub esp, 8: the frame's cursor is [ebp - 8]
    let local = fr.esp.wrapping_sub(0xc);
    let ring = g.read::<u32>(owner.wrapping_add(0x2ec));
    let pass = g.read::<u8>(cursor + 5);
    g.write::<u32>(local, ring);
    g.write::<u8>(local + 5, pass);
    let handler = g.read::<u32>(g.read::<u32>(ring).wrapping_add(0x1c));
    Ret::eax(g.call(handler, local, &[local]).eax)
}

/// [`walk_ring_junk_0150`] (113 bytes, 1 overlay).
pub const WALK_RING_JUNK_0150: Sig = Sig {
    len: 113,
    hash: 0xb253_7c48_6bab_5754,
    calls: &[],
    imports: &[],
};

/// `magic_0150`'s 12-byte thunk over [`walk_ring_junk_0150`]
/// (0x1000a5d0, a vtable entry: `ecx` = the first stack word, then a jump
/// with the caller's stack).
pub fn jmp_walk_ring_junk_0150<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let callee = Frame {
        at: img.rel32_target(fr.at + 8),
        ecx: fr.arg(g, 0),
        ..*fr
    };
    // junk read: [ebp-4] never written; 0 (brief 2026-10-24)
    walk_ring_junk_0150(g, &callee, 0)
}

/// [`jmp_walk_ring_junk_0150`].
pub const JMP_WALK_RING_JUNK_0150: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&WALK_RING_JUNK_0150))],
    imports: &[],
};

/// `magic_0570`'s start-up operand offset in `magic_0070`'s copy.
fn map_0070(o: u32) -> u32 {
    match o {
        0x15..0x148 => o + 0x26,
        0x15e.. => o + 0x12,
        _ => o,
    }
}

/// `magic_0070`'s start-up (0x1000d000, 953 bytes; cdecl `(actor)`):
/// [`start_up_0570`](crate::r27::start_up_0570)'s head with slot 381 `(1)`
/// first ([`start_up_head_0597_with`](crate::r27::start_up_head_0597_with));
/// then the two objects joining each other's rings (the first's `+0x2c`
/// and the second's `+0x34` 0, the second's `+0x18` 0x96), slot 215
/// `(*G_A)` over the words slots 238 and 296 left, slot 276 `(that,
/// 0xff)`, and slot 377 `(F, 0, 0, 0)` with the DLL's summon frame `F`
/// (0x1000cee0), its answer kept in `*G_H`. Returns 0.
pub fn start_up_0070<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    use crate::runtime::slot_target;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let [v238, v296] = crate::r27::start_up_head_0597_with(g, img, fr, map_0070, true);
    let last = rd(g, 0x2bd);
    let actor = rd(g, 0x2c3);
    // the first object joins the second's ring
    g.write::<u32>(w(0x2c8), last);
    abs_stores(g, img, f, 0x2cc, 0x2f4);
    g.write::<u32>(last.wrapping_add(4), w(0x2f7));
    let last = rd(g, 0x2fc);
    let fl = rd(g, 0x302);
    g.write::<u32>(w(0x302), fl & !1);
    g.write::<u32>(w(0x308), last);
    abs_stores(g, img, f, 0x30c, 0x316);
    // esi: the loop's count, 0
    g.write::<u32>(w(0x318), 0);
    g.write::<u32>(w(0x31e), 0);
    abs_stores(g, img, f, 0x322, 0x34a);
    // and the second the first's
    g.write::<u32>(last.wrapping_add(4), w(0x34d));
    let s215 = t(g, 0x352, 215);
    let fl = rd(g, 0x358);
    g.write::<u32>(w(0x358), fl & !1);
    abs_stores(g, img, f, 0x35d, 0x367);
    g.write::<u32>(w(0x369), 0);
    abs_stores(g, img, f, 0x36d, 0x377);
    // call width: 5
    let r = g.call(s215, 0, &[actor, v296, 0, v238, 1]).eax;
    // call width: 2
    g.call(t(g, 0x389, 276), 0, &[r, 0xff]);
    // call width: 6
    let h = g
        .call(t(g, 0x396, 377), 0, &[w(0x3a4), 0, 0, 0, r, 0xff])
        .eax;
    g.write::<u32>(w(0x3ae), h);
    Ret::eax(0)
}

/// [`start_up_0070`] (953 bytes, 1 overlay).
pub const START_UP_0070: Sig = Sig {
    len: 953,
    hash: 0xe845_6e39_7a93_3be7,
    calls: &[(0xc, Callee::Sig(&crate::s4_summon::TARGETS_INIT_C))],
    imports: &[],
};

/// What one entry of a [`motion_task`] copy's jump table runs (the
/// handler's offset in the function).
#[derive(Clone, Copy)]
pub(crate) enum Motion {
    /// Wait for the object's motion (slot 20 of `O[0x54]` with bit 0 of
    /// `O + 0x90`, else slot 411 of `O[0x50]`; the imports at operands
    /// `o20` and `o411`): 0 ends the call, else the state is `next` and
    /// runs at once.
    Wait { o20: u32, o411: u32, next: u32 },
    /// [`set_value`](crate::m0042::set_value) (the call at `call`, the
    /// value the `mov edx, imm32` before it) and the state stepped on.
    Set { call: u32 },
    /// The object's model id ([`model_id`](crate::sa_misc::model_id), the
    /// call at `id`): none ends the call (its `eax`), else `Set { call }`;
    /// with `reset` `R + 0x30` is 0 before its count.
    IdSet { id: u32, call: u32, reset: bool },
    /// A set's tail alone: the state stepped on (`eax` the table index).
    Step,
}

/// A [`motion_task`] copy: the byte and jump tables' operands, the
/// highest table index, the handlers by offset.
pub(crate) struct MotionTask {
    pub bytes: u32,
    pub jumps: u32,
    pub last: u32,
    pub handlers: &'static [(u32, Motion)],
}

/// The summons' motion task (`magic_0609` 0x100093d0's shape, with its
/// state table read from the copy: [`MotionTask`]; a task function, cdecl
/// `(task)`, the record `R = task[1]`; an 8-aligned frame with the
/// model-id pair at `A = ((E - 4) & !7) - 8`). By the state `R + 0x18`:
/// 10000 unlinks the task node; a state above it or past the table, or a
/// table entry with no handler, only counts; the table's handlers wait,
/// set a value or wait for the model id ([`Motion`]). Every way ends with
/// `R + 0x2c` and `R + 0x30` up. Returns the last `eax`: the set's answer,
/// the wait's 0, the model id, the state (less 10 past the table), the
/// table index, the node's old `+4` after the unlink.
pub(crate) fn motion_task<G: Game>(g: &mut G, img: &Image, fr: &Frame, l: &MotionTask) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let bump = |g: &mut G, o: u32| {
        let v = g.read::<u32>(r + o).wrapping_add(1);
        g.write::<u32>(r + o, v);
    };
    let end = |g: &mut G, step: bool, eax: u32| {
        if step {
            bump(g, 0x18);
        }
        bump(g, 0x2c);
        bump(g, 0x30);
        Ret::eax(eax)
    };
    let set = |g: &mut G, call: u32| {
        let obj = g.read::<u32>(r + 0x1c);
        // mov edx, imm32 just before the call (its rel32 at `call`)
        let v = w(call - 5);
        crate::m0042::set_value(g, img, img.rel32_target(f + call), obj, v)
    };
    loop {
        let state = g.read::<u32>(r + 0x18);
        if state > 10000 {
            return end(g, false, state);
        }
        if state == 10000 {
            let node = g.read::<u32>(task);
            let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
            g.write::<u32>(prev + 8, next);
            let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
            g.write::<u32>(next + 4, prev);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            return end(g, false, prev);
        }
        let i = state.wrapping_sub(10);
        if i > l.last {
            return end(g, false, i);
        }
        // movzx eax, byte [i + bytes]; jmp [eax * 4 + jumps]
        let idx = img.word(w(l.bytes) + i) & 0xff;
        let to = img.word(w(l.jumps) + 4 * idx).wrapping_sub(f);
        let Some(&(_, h)) = l.handlers.iter().find(|(o, _)| *o == to) else {
            // the counts' tail: eax still the index
            return end(g, false, idx);
        };
        match h {
            Motion::Wait { o20, o411, next } => {
                let obj = g.read::<u32>(r + 0x1c);
                let eax = if g.read::<u8>(obj + 0x90) & 1 != 0 {
                    let s = slot_target(g, w(o20), 20);
                    g.call(s, 0, &[g.read::<u32>(obj + 0x54)]).eax
                } else {
                    let s = slot_target(g, w(o411), 411);
                    g.call(s, 0, &[g.read::<u32>(obj + 0x50)]).eax
                };
                if eax == 0 {
                    return end(g, false, 0);
                }
                g.write::<u32>(r + 0x18, next);
            }
            Motion::Set { call } => {
                let eax = set(g, call);
                return end(g, true, eax);
            }
            Motion::Step => return end(g, true, idx),
            Motion::IdSet { id, call, reset } => {
                let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(8);
                let obj = g.read::<u32>(r + 0x1c);
                g.write::<u32>(a, obj);
                let m = crate::sa_misc::model_id(g, img, img.rel32_target(f + id), a);
                if g.read::<u32>(a + 4) == 0 {
                    return end(g, false, m);
                }
                let eax = set(g, call);
                if reset {
                    bump(g, 0x18);
                    bump(g, 0x2c);
                    g.write::<u32>(r + 0x30, 1);
                    return Ret::eax(eax);
                }
                return end(g, true, eax);
            }
        }
    }
}

/// The handlers of `magic_0155`'s motion task (0x100092e0).
const MOTION_0155: MotionTask = MotionTask {
    bytes: 0x35,
    jumps: 0x3c,
    last: 0x33,
    handlers: &[
        (
            0x40,
            Motion::Wait {
                o20: 0x50,
                o411: 0x5d,
                next: 0x14,
            },
        ),
        (
            0x7d,
            Motion::Wait {
                o20: 0x8d,
                o411: 0x9a,
                next: 0x28,
            },
        ),
        (0xd2, Motion::Set { call: 0xdb }),
        (0xdf, Motion::Step),
        (0xee, Motion::Set { call: 0xf7 }),
        (0x10a, Motion::Set { call: 0x113 }),
        (0x126, Motion::Set { call: 0x12f }),
        (
            0x142,
            Motion::IdSet {
                id: 0x14e,
                call: 0x162,
                reset: false,
            },
        ),
        (0x175, Motion::Set { call: 0x17e }),
    ],
};

/// `magic_0155`'s motion task (0x100092e0, 532 bytes): [`motion_task`]
/// with the values 0xb9, 0xba, 0xbb, 0xbe, 0xbc (after the model id) and
/// 0xbd.
pub fn motion_task_0155<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    motion_task(g, img, fr, &MOTION_0155)
}

/// [`motion_task_0155`] (532 bytes, 1 overlay).
pub const MOTION_TASK_0155: Sig = Sig {
    len: 532,
    hash: 0x82b8_3e1e_99e5_6603,
    calls: &[
        (0xdb, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0xf7, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x113, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x12f, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x14e, Callee::Sig(&crate::sa_misc::MODEL_ID)),
        (0x162, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x17e, Callee::Sig(&crate::m0042::SET_VALUE)),
    ],
    imports: &[],
};

/// The handlers of `magic_0164`'s motion task (0x10009340).
const MOTION_0164: MotionTask = MotionTask {
    bytes: 0x35,
    jumps: 0x3c,
    last: 0x33,
    handlers: &[
        (
            0x40,
            Motion::Wait {
                o20: 0x50,
                o411: 0x5d,
                next: 0x14,
            },
        ),
        (
            0x7d,
            Motion::Wait {
                o20: 0x8d,
                o411: 0x9a,
                next: 0x28,
            },
        ),
        (0xd2, Motion::Set { call: 0xdb }),
        (0xdf, Motion::Step),
        (0xee, Motion::Set { call: 0xf7 }),
        (0x10a, Motion::Set { call: 0x113 }),
        (0x126, Motion::Set { call: 0x12f }),
        (
            0x142,
            Motion::IdSet {
                id: 0x14e,
                call: 0x166,
                reset: false,
            },
        ),
        (0x179, Motion::Set { call: 0x182 }),
        (
            0x195,
            Motion::IdSet {
                id: 0x1a1,
                call: 0x1b5,
                reset: true,
            },
        ),
    ],
};

/// `magic_0164`'s motion task (0x10009340, 600 bytes): [`motion_task`]
/// with the values 0xb9, 0xba, 0xbb, 0xbe, 0xbc (after the model id) and
/// 0xbd, and a second model-id wait whose set leaves `R + 0x30` at 1.
pub fn motion_task_0164<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    motion_task(g, img, fr, &MOTION_0164)
}

/// [`motion_task_0164`] (600 bytes, 1 overlay).
pub const MOTION_TASK_0164: Sig = Sig {
    len: 600,
    hash: 0x4952_4153_c7bc_a342,
    calls: &[
        (0xdb, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0xf7, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x113, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x12f, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x14e, Callee::Sig(&crate::sa_misc::MODEL_ID)),
        (0x166, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x182, Callee::Sig(&crate::m0042::SET_VALUE)),
        (0x1a1, Callee::Sig(&crate::sa_misc::MODEL_ID)),
        (0x1b5, Callee::Sig(&crate::m0042::SET_VALUE)),
    ],
    imports: &[],
};

/// `magic_0203`'s particle node run (0x100042a0, 841 bytes; internal,
/// `ecx` = the cursor, reached through its thunk 0x10004630): the node
/// family's [`run_part`](crate::sb_ppp::run_part) on another node layout
/// (the part at `+0x70`, flags `+0xf8`, delay `+0xfc`, the model node
/// `+0x5c`), and instead of the colour blend: while not paused with flag
/// bit 0 the halfword `+0xf4` counts down (bit 0 cleared at 0); then
/// always the colour `+0xe0..` (four halfwords) = the colour source's
/// (`[+0x108]`) `+0x1c..` times the node's `+0x4c..` (`cvttss2si`). When
/// `pppRunPart` says done, the handed-over word is given again, slot 713
/// `(part)` and the node leaves its parent
/// ([`leave`](crate::sa_misc::leave)). The user function `+0xf0` gets
/// `+0x104`. Returns the last `eax` the original leaves.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn run_part_0203<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, this: u32) -> u32 {
    use crate::sb_sse::mul;
    use ffx_bridge::x87::cvttsd2si;
    let w = |off: u32| img.word(f + off);
    let rd = |g: &G, off: u32| g.read::<u32>(w(off));
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let ebp = esp.wrapping_sub(4);
    let eax = rd(g, 0x07) ^ ebp;
    let (eax, tail) = crate::nodes::walk(g, this, this, eax);
    let Some(o) = tail else {
        return eax;
    };
    let flags = g.read::<u32>(o + 0xf8);
    if flags & 2 != 0 {
        return eax;
    }
    let delay = g.read::<u32>(o + 0xfc);
    let pause = rd(g, 0x75);
    if delay.cast_signed() > 0 {
        if g.read::<u32>(pause) != 0 {
            return pause;
        }
        let d = delay.wrapping_sub(1);
        g.write::<u32>(o + 0xfc, d);
        return d;
    }
    if g.read::<u32>(pause) == 0 && flags & 1 != 0 {
        let n = g.read::<u16>(o + 0xf4);
        if n == 0 {
            g.write::<u32>(o + 0xf8, flags & !1);
        }
        g.write::<u16>(o + 0xf4, n.wrapping_sub(1));
    }
    for i in 0..4 {
        let src = g.read::<u32>(o + 0x108);
        let x = mul(rf(g, src + 0x1c + 4 * i), rf(g, o + 0x4c + 4 * i));
        g.write::<u16>(o + 0xe0 + 2 * i, cvttsd2si(f64::from(x)) as u16);
    }
    let unit = rd(g, 0x134);
    for i in 0..4 {
        g.write::<u32>(unit + 4 * i, 0x3f80_0000);
    }
    let vec = ebp.wrapping_sub(0x14);
    let part = o + 0x70;
    // call width: 2
    g.call(slot_target(g, w(0x15c), 325), this, &[vec, o + 0x1c]);
    let src = g.read::<u32>(o + 0x5c).wrapping_add(0x9c);
    // call width: 2
    g.call(slot_target(g, w(0x172), 230), this, &[rd(g, 0x178), src]);
    for (k, (mo, to)) in (0u32..).zip([(0x185u32, 0x199u32), (0x1a6, 0x1bd), (0x1ca, 0x1e1)]) {
        let row = rd(g, mo).wrapping_add(0x10 * k);
        let s = g.read::<u32>(vec + 4 * k);
        // call width: 3
        g.call(slot_target(g, w(to), 233), this, &[row, row, s]);
    }
    let scale = rf(g, g.read::<u32>(o + 0x5c).wrapping_add(0x138));
    for i in 0..3 {
        let x = mul(rf(g, vec + 4 * i), scale);
        g.write::<f32>(vec + 4 * i, x);
    }
    let n = g.read::<u32>(o + 0x5c).wrapping_add(0x12c);
    // call width: 3
    g.call(slot_target(g, w(0x231), 668), this, &[vec, vec, n]);
    g.write::<u32>(vec + 0xc, 0x3f80_0000);
    // call width: 2
    g.call(slot_target(g, w(0x249), 325), this, &[o + 0xd0, vec]);
    let (a, b, c) = (rd(g, 0x25e), rd(g, 0x269), rd(g, 0x275));
    // call width: 3
    g.call(slot_target(g, w(0x263), 231), this, &[c, b, a]);
    let (a, b) = (rd(g, 0x27d), rd(g, 0x288));
    // call width: 2
    g.call(slot_target(g, w(0x282), 230), this, &[b, a]);
    let v = rd(g, 0x29b);
    g.write::<u32>(rd(g, 0x296), v);
    let v = g.read::<u32>(o + 0x60);
    g.write::<u32>(rd(g, 0x2a2), v);
    let lock = rd(g, 0x2ac);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 2
    let done = g
        .call(slot_target(g, w(0x2bb), 712), this, &[part, 0x30])
        .eax;
    g.write::<u32>(rd(g, 0x2ca), saved);
    if done != 0 {
        let v = g.read::<u32>(o + 0x60);
        g.write::<u32>(rd(g, 0x2d8), v);
        let e = g.call(slot_target(g, w(0x2e2), 713), this, &[part]).eax;
        crate::sa_misc::leave(g, o, e);
        let fl = g.read::<u32>(o + 0xf8);
        g.write::<u32>(o + 0xf8, fl | 4);
    }
    let v = g.read::<u32>(rd(g, 0x301));
    g.write::<u32>(w(0x308), v);
    let pause = rd(g, 0x30d);
    if g.read::<u32>(pause) != 0 {
        return pause;
    }
    let func = g.read::<u32>(o + 0xf0);
    if func == 0 {
        return 0;
    }
    let arg = g.read::<u32>(o + 0x104);
    // call width: 3
    g.call(slot_target(g, w(0x328), 160), this, &[part, func, arg])
        .eax
}

/// [`run_part_0203`] (841 bytes, 1 overlay).
pub const RUN_PART_0203: Sig = Sig {
    len: 841,
    hash: 0xbfb0_21ff_97d6_99b4,
    calls: &[
        // __security_check_cookie
        (0x98, Callee::Any),
        (0x2f5, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x341, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0203`'s 12-byte thunk over [`run_part_0203`] (0x10004630: `ecx`
/// = the first stack word, then a jump with the caller's stack).
pub fn jmp_run_part_0203<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.arg(g, 0);
    let f = img.rel32_target(fr.at + 8);
    Ret::eax(run_part_0203(g, img, f, fr.esp, this))
}

/// [`jmp_run_part_0203`].
pub const JMP_RUN_PART_0203: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&RUN_PART_0203))],
    imports: &[],
};

/// `magic_0186`'s path follower run (0x10003ce0, 604 bytes; internal,
/// `ecx` = the cursor, reached through its thunk 0x10003f40; an 8-aligned
/// cookie frame, `A = ((E - 4) & !7) - 0x34`: the step `V` at `A`, the
/// target `P` at `A + 0x10`, the visit cursor at `A + 0x20`, the squares
/// at `A + 0x20`). The node walk; when it ran, the owner `N`'s model `M =
/// N[0x54]` is visited (its handler, a cursor with the walk's pass byte),
/// then by `N`'s kind byte `+0x5c` the target `P` comes from slot 262
/// `(M[0x54], &P.x, &P.y, &P.z)`, slot 29 `(M[0x54], P)`, slot 268
/// `(M[0x54], N[0x58], P)`, or, unless bit 0 of `M + 0x90`, slot 366 or
/// 665 `(M[0x50], P)`. The speed `+0x4c` grows by `+0x50` and the
/// distance `+0x48` by the speed; `V` = `P - N.pos` (slot 234 `(V, P, N +
/// 0x2c)`), its length (slot 668 squares, `_libm_sse2_sqrt_precise`),
/// `V` scaled to `K / len * dist` (`w` 1.0) and moved to `N + 0x2c` (slot
/// 248 `(V, V, N + 0x2c)`), slot 325 `(N + 0x1c, V)`; once the distance
/// reaches `P.x` the byte `+0x5d` is 1. Returns the last `eax`.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn path_run_0186<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, this: u32) -> u32 {
    use crate::sb_sse::{add, div, mul};
    let w = |off: u32| img.word(f + off);
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let t = |g: &G, op: u32, n: u32| slot_target(g, w(op), n);
    let a = (esp.wrapping_sub(4) & !7).wrapping_sub(0x34);
    let eax = g.read::<u32>(w(0x0a)) ^ a;
    let (eax, tail) = crate::nodes::walk(g, this, this, eax);
    let Some(n) = tail else {
        return eax;
    };
    let cur = a + 0x20;
    let model = g.read::<u32>(n + 0x54);
    let pass = g.read::<u8>(this + 5);
    g.write::<u32>(cur, model);
    g.write::<u8>(cur + 5, pass);
    let handler = g.read::<u32>(g.read::<u32>(model).wrapping_add(0x1c));
    g.call(handler, cur, &[cur]);
    let p = a + 0x10;
    let m = g.read::<u32>(n + 0x54);
    match g.read::<u8>(n + 0x5c) {
        0 => {
            let x = g.read::<u32>(m + 0x54);
            // call width: 4
            g.call(t(g, 0xa5, 262), 0, &[x, p, p + 4, p + 8]);
        }
        1 => {
            let x = g.read::<u32>(m + 0x54);
            // call width: 2
            g.call(t(g, 0xc2, 29), 0, &[x, p]);
        }
        2 => {
            let (x, k) = (g.read::<u32>(m + 0x54), g.read::<u32>(n + 0x58));
            // call width: 3
            g.call(t(g, 0xda, 268), 0, &[x, k, p]);
        }
        k @ (3 | 4) if g.read::<u8>(m + 0x90) & 1 == 0 => {
            let x = g.read::<u32>(m + 0x50);
            let s = if k == 3 {
                t(g, 0x100, 366)
            } else {
                t(g, 0x121, 665)
            };
            // call width: 2
            g.call(s, 0, &[x, p]);
        }
        _ => {}
    }
    let speed = add(rf(g, n + 0x50), rf(g, n + 0x4c));
    g.write::<f32>(n + 0x4c, speed);
    let dist = add(speed, rf(g, n + 0x48));
    g.write::<f32>(n + 0x48, dist);
    let pos = n + 0x2c;
    // call width: 3
    g.call(t(g, 0x153, 234), 0, &[a, p, pos]);
    let sq = a + 0x20;
    // call width: 3
    g.call(t(g, 0x173, 668), 0, &[sq, a, a]);
    let s = add(add(rf(g, sq + 4), rf(g, sq)), rf(g, sq + 8));
    // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
    let len = f64::from(s).sqrt() as f32;
    let r = div(rf(g, w(0x1a0)), len);
    let d = rf(g, n + 0x48);
    let v = [0u32, 4, 8].map(|i| mul(mul(rf(g, a + i), r), d));
    g.write::<u32>(a + 0xc, 0x3f80_0000);
    for (i, x) in (0u32..).zip(v) {
        g.write::<f32>(a + 4 * i, x);
    }
    // call width: 3
    g.call(t(g, 0x1e3, 248), 0, &[a, a, pos]);
    // call width: 2
    let eax = g.call(t(g, 0x216, 325), 0, &[n + 0x1c, a]).eax;
    let d = rf(g, n + 0x48);
    let lim = rf(g, p);
    // comiss; jb: below or unordered skips
    if !(d.is_nan() || lim.is_nan() || d < lim) {
        g.write::<u8>(n + 0x5d, 1);
    }
    eax
}

/// [`path_run_0186`] (604 bytes, 1 overlay).
pub const PATH_RUN_0186: Sig = Sig {
    len: 604,
    hash: 0x4107_8823_7d0b_6e9f,
    calls: &[
        (0x198, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x23f, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0186`'s 12-byte thunk over [`path_run_0186`] (0x10003f40: `ecx`
/// = the first stack word, then a jump with the caller's stack).
pub fn jmp_path_run_0186<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let this = fr.arg(g, 0);
    let f = img.rel32_target(fr.at + 8);
    Ret::eax(path_run_0186(g, img, f, fr.esp, this))
}

/// [`jmp_path_run_0186`].
pub const JMP_PATH_RUN_0186: Sig = Sig {
    len: 12,
    hash: crate::s3_wrap::ECX_JMP_HASH,
    calls: &[(8, Callee::Sig(&PATH_RUN_0186))],
    imports: &[],
};

/// The angle of the flat vector `(x, z)`, quadrant by quadrant, with the
/// constants at operands `k` (the
/// base for `x <= 0`, the bases for `z > 0` and `z <= 0` when `x z < 0`,
/// `x` negative or NaN, `x == 0` with `z` positive, negative or NaN, and
/// both 0): `x z > 0`: `atan(z / x)` plus the base (0 for `x > 0`); `x z
/// < 0`: the base less `atan(x / z)`; otherwise by the signs (`x > 0`: 0).
/// `atan` is MSVCR110's `_libm_sse2_atan_precise` on the `f64` quotient,
/// rounded to `f32`.
fn flat_angle<G: Game>(g: &mut G, img: &Image, f: u32, x: f32, z: f32, k: [u32; 7]) -> f32 {
    use crate::sb_sse::{add, div, mul, sub};
    let c = |g: &G, i: usize| g.read::<f32>(img.word(f + k[i]));
    let p = mul(x, z);
    if p > 0.0 {
        let base = if x > 0.0 { 0.0 } else { c(g, 0) };
        let t = g.libm(crate::Libm::Atan, f64::from(div(z, x))) as f32;
        return add(t, base);
    }
    if 0.0 > p {
        let base = if z > 0.0 { c(g, 1) } else { c(g, 2) };
        let t = g.libm(crate::Libm::Atan, f64::from(div(x, z))) as f32;
        return sub(base, t);
    }
    // ucomiss + lahf + jnp: taken only when ordered and equal
    if x != 0.0 {
        return if x > 0.0 { 0.0 } else { c(g, 3) };
    }
    if z != 0.0 {
        return if z > 0.0 { c(g, 4) } else { c(g, 5) };
    }
    c(g, 6)
}

/// `magic_0179`'s yaw from one actor to another (0x10006540, 399 bytes;
/// internal, `ecx` and `edx` the two actors; an `ebp` frame with a cookie):
/// slot 217 `(a, P)` and `(b, Q)` (their positions at `ebp - 0x14` and
/// `ebp - 0x24`), slot 234 `(P, Q, P)` (`P - Q`), then the angle of
/// `(P.x, P.z)` ([`flat_angle`]).
/// Returns it in `xmm0` (`eax` junk).
pub fn yaw_0179<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, a: u32, b: u32) -> f32 {
    let w = |o: u32| img.word(f + o);
    let ebp = esp.wrapping_sub(4);
    let (p, q) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    // call width: 2
    g.call(slot_target(g, w(0x16), 217), 0, &[a, p]);
    // call width: 2
    g.call(slot_target(g, w(0x2a), 217), 0, &[b, q]);
    // call width: 3
    g.call(slot_target(g, w(0x44), 234), 0, &[p, q, p]);
    let (x, z) = (g.read::<f32>(p), g.read::<f32>(p + 8));
    flat_angle(
        g,
        img,
        f,
        x,
        z,
        [0x7a, 0xbb, 0xc5, 0x120, 0x148, 0x162, 0x17f],
    )
}

/// [`yaw_0179`] (399 bytes, 1 overlay).
pub const YAW_0179: Sig = Sig {
    len: 399,
    hash: 0x7f44_5a8a_20ec_82a1,
    calls: &[
        (0x8b, Callee::Import("_libm_sse2_atan_precise")),
        // __security_check_cookie
        (0xa5, Callee::Any),
        (0xd6, Callee::Import("_libm_sse2_atan_precise")),
        (0xf1, Callee::Any),
        (0x114, Callee::Any),
        (0x12e, Callee::Any),
        (0x156, Callee::Any),
        (0x170, Callee::Any),
        (0x187, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0179`'s throw set-up (0x10006020, 1071 bytes; internal, `ecx` =
/// a record `R = [N, P, T, v, g, high]`; an 8-aligned cookie frame, `A =
/// ((E - 4) & !7) - 0x88`: the yaw at `A + 4`, the speed at `A + 8`, the
/// time at `A + 0xc`, the pitch at `A + 0x10`, the flat distance at `A +
/// 0x14`, the matrix at `A + 0x40`). The node `N` gets an empty child list
/// (pass halfword 0xff) and joins `P`'s children at the end (when `P` is
/// set); `d = T - P.pos` (`P + 0x1c`), the flat distance `D` (`_libm_sse2_
/// sqrt_precise`) and the yaw (`-`[`flat_angle`] of `(d.x, d.z)`). From
/// the speed `v = R[3]`, 32 tries in steps of `K` look for a flight time
/// `t` (the roots of `g^2 t^2 - 2 (v^2 - d.y g) t + ...`: `(v^2 - d.y g +-
/// sqrt(v^4 - D^2 g^2 - d.y g v^2)) / g^2`, one positive; with `high` the
/// larger, else the smaller positive one); found, the pitch is
/// `-atan2((d.y - t g) / v / sqrt t, D / v / sqrt t)` (`_CIatan2`).
/// `R[3]` = the speed reached; slot 235 `(M)` and slots 330 `(M, M,
/// pitch)`, 329 `(M, M, yaw)` turn the matrix, slot 229 `(N + 0x1c, M,
/// (v, 0, 0, 0))` gives the velocity; `N + 0x2c` = `g`. Returns `g`'s bits.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn throw_setup_0179<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, r: u32) -> u32 {
    use crate::sb_sse::{add, div, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let a = (esp.wrapping_sub(4) & !7).wrapping_sub(0x88);
    let n = g.read::<u32>(r);
    let p = g.read::<u32>(r + 4);
    g.write::<u16>(n + 0x1a, 0xff);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, p);
    if p != 0 {
        g.write::<u32>(n + 4, p);
        let last = g.read::<u32>(p + 0x10);
        g.write::<u32>(n + 8, last);
        let last = g.read::<u32>(p + 0x10);
        if last == p {
            g.write::<u32>(p + 0xc, n);
        } else {
            g.write::<u32>(last + 4, n);
        }
        let c = g.read::<u16>(p + 0x18).wrapping_add(1);
        g.write::<u16>(p + 0x18, c);
        g.write::<u32>(p + 0x10, n);
    }
    let t = g.read::<u32>(r + 8);
    let dx = sub(rf(g, t), rf(g, p + 0x1c));
    let dy = sub(rf(g, t + 4), rf(g, p + 0x20));
    let dz = sub(rf(g, t + 8), rf(g, p + 0x24));
    g.write::<f32>(a + 0x30, dx);
    g.write::<f32>(a + 8, dx);
    g.write::<f32>(a + 0x10, dy);
    g.write::<f32>(a + 0x34, dy);
    g.write::<f32>(a + 4, dz);
    g.write::<f32>(a + 0x38, dz);
    let sq = add(mul(dz, dz), mul(dx, dx));
    // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
    let dist = f64::from(sq).sqrt() as f32;
    g.write::<f32>(a + 0x14, dist);
    g.write::<f32>(a + 0x3c, dist);
    let ops = [0xfa, 0x139, 0x143, 0x189, 0x1a1, 0x1ab, 0x1b5];
    let yaw = flat_angle(g, img, f, dx, dz, ops);
    // xorps with the sign mask
    let yaw = f32::from_bits(yaw.to_bits() ^ g.read::<u32>(w(0x1bc)));
    g.write::<f32>(a + 4, yaw);
    let mut v = rf(g, r + 0xc);
    let gr = rf(g, r + 0x10);
    let high = g.read::<u32>(r + 0x14);
    let g2 = mul(gr, gr);
    let dyg = mul(dy, gr);
    let dgg = mul(mul(dist, g2), dist);
    g.write::<f32>(a + 8, v);
    g.write::<f32>(a + 0x30, v);
    g.write::<u32>(a + 0x18, high);
    g.write::<f32>(a + 0x24, g2);
    g.write::<f32>(a + 0x28, dyg);
    g.write::<f32>(a + 0x2c, dgg);
    let mut found = None;
    for _ in 0..0x20 {
        let v2 = mul(v, v);
        let det = sub(sub(mul(v2, v2), dgg), mul(dyg, v2));
        g.write::<f32>(a + 0xc, v2);
        // comiss 0, disc; ja: a negative discriminant tries the next speed
        if 0.0 <= det || det.is_nan() {
            let root = f64::from(det).sqrt() as f32;
            let b = sub(v2, dyg);
            let t1 = div(add(b, root), g2);
            let t2 = div(sub(b, root), g2);
            g.write::<f32>(a + 0xc, t1);
            // comiss 0, t; jb: positive or unordered
            let use_t = |t: f32| 0.0 < t || t.is_nan();
            if use_t(t1) || use_t(t2) {
                found = Some((t1, t2));
                break;
            }
        }
        v = add(v, k(g, 0x2aa));
        g.write::<f32>(a + 8, v);
        g.write::<f32>(a + 0x30, v);
    }
    let pitch = if let Some((t1, t2)) = found {
        let t = if high != 0 {
            // comiss t1, t2; jae
            if t1 >= t2 { t1 } else { t2 }
        } else if t1 <= 0.0 || t1.is_nan() {
            // comiss t1, 0; jbe: not positive (or NaN) takes t2
            t2
        } else if t2 >= t1 {
            t1
        } else {
            t2
        };
        g.write::<f32>(a + 0xc, t);
        let s = f64::from(t).sqrt() as f32;
        g.write::<f32>(a + 0x38, s);
        let y = div(div(sub(dy, mul(t, gr)), v), s);
        let x = div(div(dist, v), s);
        let ang = g.atan2(f64::from(y), f64::from(x)) as f32;
        f32::from_bits(ang.to_bits() ^ g.read::<u32>(w(0x41a)))
    } else {
        // the loop ran out: the pitch word takes the `z` copy
        rf(g, a + 0x38)
    };
    g.write::<f32>(a + 0x10, pitch);
    g.write::<f32>(r + 0xc, v);
    let m = a + 0x40;
    g.call(slot_target(g, w(0x2da), 235), 0, &[m]);
    // call width: 3
    g.call(slot_target(g, w(0x2fd), 330), 0, &[m, m, pitch.to_bits()]);
    let yaw = g.read::<u32>(a + 4);
    // call width: 3
    g.call(slot_target(g, w(0x31e), 329), 0, &[m, m, yaw]);
    for o in [0x34, 0x38, 0x3c] {
        g.write::<u32>(a + o, 0);
    }
    // call width: 3
    g.call(slot_target(g, w(0x33c), 229), 0, &[n + 0x1c, m, a + 0x30]);
    let gw = g.read::<u32>(r + 0x10);
    g.write::<u32>(n + 0x2c, gw);
    gw
}

/// [`throw_setup_0179`] (1071 bytes, 1 overlay).
pub const THROW_SETUP_0179: Sig = Sig {
    len: 1071,
    hash: 0x3a3c_11b0_33f6_1767,
    calls: &[
        (0xb7, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x10c, Callee::Import("_libm_sse2_atan_precise")),
        (0x155, Callee::Import("_libm_sse2_atan_precise")),
        (0x250, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x375, Callee::Any),
        (0x3a6, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x405, Callee::Import("_CIatan2")),
    ],
    imports: &[],
};

/// `magic_0179`'s thrown object start (0x10006b60, 343 bytes; internal,
/// `ecx` = the object `O`, `edx` = a word for slot 325, cdecl `(T, kind)`
/// with `T` the target point; an 8-aligned frame, `A = ((E - 4) & !7) -
/// 0x1c`, the set-up records at `A + 4`). `O` joins the DLL's ring at its
/// end (vtable, step [0x100069c0], its timer `+0x18` and state `+0x1c`
/// 0); slot 325 `(O + 0x1f0, T)`; the node `S = O + 0x48` joins the
/// static list (pass halfword 0xff, after the list's last: the list's
/// first when it was empty, the count up) with its vtable; slot 325 `(O +
/// 0x64, edx)`; [`node_insert`](crate::nodes::node_insert) of `(O + 0x74,
/// ...)`, [`throw_setup_0179`] of `(O + 0x1c0, S, T, 15.0, 0.05, 0)` and
/// the node `O + 0x1c0`'s vtable; `O + 0x1e` = `kind`. Returns the node
/// word with `kind` in its low half.
pub fn object_start_0179<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    esp: u32,
    o: u32,
    edx: u32,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let rd = |g: &G, off: u32| g.read::<u32>(w(off));
    let t = g.read::<u32>(esp + 4);
    let kind = g.read::<u16>(esp + 8);
    let a = (esp.wrapping_sub(4) & !7).wrapping_sub(0x1c);
    g.write::<u32>(o + 8, o);
    g.write::<u32>(o + 4, o);
    g.write::<u32>(o, w(0x1a));
    g.write::<u32>(o + 0x10, w(0x21));
    g.write::<u32>(o + 0x14, o);
    let last = rd(g, 0x29);
    g.write::<u32>(o + 8, last);
    g.write::<u32>(o + 4, w(0x33));
    let last = rd(g, 0x38);
    // edx kept on the frame (pushed again for slot 325)
    g.write::<u32>(a, edx);
    g.write::<u32>(last.wrapping_add(4), o);
    let fl = rd(g, 0x45);
    g.write::<u32>(w(0x45), fl & !1);
    g.write::<u32>(w(0x4e), o);
    g.write::<u16>(o + 0x1c, 0);
    g.write::<u32>(o + 0x18, 0);
    // call width: 2
    g.call(slot_target(g, w(0x61), 325), 0, &[o + 0x1f0, t]);
    let s = o + 0x48;
    g.write::<u16>(s + 0x1a, 0xff);
    g.write::<u32>(s + 0x10, s);
    g.write::<u32>(s + 0xc, s);
    g.write::<u32>(s + 0x14, w(0x82));
    g.write::<u32>(s + 4, w(0x89));
    let old = rd(g, 0x8e);
    g.write::<u32>(s + 8, old);
    let old = rd(g, 0x96);
    if old == w(0x9b) {
        g.write::<u32>(w(0xa3), s);
    } else {
        g.write::<u32>(old.wrapping_add(4), s);
    }
    let c = g.read::<u16>(w(0xb3)).wrapping_add(1);
    g.write::<u16>(w(0xb3), c);
    g.write::<u32>(w(0xc2), s);
    g.write::<u32>(s, w(0xc8));
    // call width: 2
    g.call(slot_target(g, w(0xbc), 325), 0, &[o + 0x64, edx]);
    let rec = a + 4;
    for (i, x) in (0u32..).zip([o + 0x74, w(0xe6), w(0xee), s, w(0xfa), w(0x102)]) {
        g.write::<u32>(rec + 4 * i, x);
    }
    let fr = Frame {
        at: img.rel32_target(f + 0x107),
        esp: a.wrapping_sub(0x10),
        ecx: rec,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &fr);
    for (i, x) in (0u32..).zip([o + 0x1c0, s, t, 0x4170_0000, 0x3d4c_cccd, 0]) {
        g.write::<u32>(rec + 4 * i, x);
    }
    throw_setup_0179(
        g,
        img,
        img.rel32_target(f + 0x13a),
        a.wrapping_sub(0x10),
        rec,
    );
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x144));
    g.write::<u16>(o + 0x1e, kind);
    (node & !0xffff) | u32::from(kind)
}

/// [`object_start_0179`] (343 bytes, 1 overlay).
pub const OBJECT_START_0179: Sig = Sig {
    len: 343,
    hash: 0xdc32_c50b_32e5_4642,
    calls: &[
        (0x107, Callee::Sig(&crate::nodes::INSERT)),
        (0x13a, Callee::Sig(&THROW_SETUP_0179)),
    ],
    imports: &[],
};

/// `magic_0179`'s throw task (0x10006cc0, 424 bytes; a task function,
/// cdecl `(task)`, the record `R = task[1]`; an `ebp` frame with a cookie,
/// the particle record at `ebp - 0x34`, the target `T` at `ebp - 0x18`):
/// the timer `R + 0x18` grows by `K`; below the next mark `R + 0x1c` the
/// call ends. Else a particle node at `R + 0x30 + 0x100 i` (`i` = `R +
/// 0x20`, [`particle_node`](crate::sb_nodes::particle_node)), `T` = `R +
/// 0x9d4` (slot 325) moved by three random steps `(r - K1) K2 - K3` (slot
/// 307), and the object `R + 0x9f0 + 0x480 i` started at `T`
/// ([`object_start_0179`], kind 1 from the seventh on, `edx` = `R +
/// 0x84c`); `i` up, the task node unlinked after the eighth; the mark up
/// by `K4`. Returns the object start's `eax` (the node's old `+4` after
/// the unlink), or the cookie's on the early way.
pub fn throw_task_0179<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| g.read::<f32>(w(off));
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let ebp = fr.esp.wrapping_sub(4);
    let timer = add(g.read::<f32>(r + 0x18), k(g, 0x21));
    let mark = g.read::<f32>(r + 0x1c);
    g.write::<f32>(r + 0x18, timer);
    // comiss timer, mark; jb: below (or NaN) ends
    if timer.is_nan() || mark.is_nan() || timer < mark {
        return Ret::eax(g.read::<u32>(w(0x7)) ^ ebp);
    }
    let i = u32::from(g.read::<u16>(r + 0x20));
    let rec = ebp.wrapping_sub(0x34);
    let node = r + 0x30 + (i << 8);
    let x = g.read::<u32>(w(0x4a));
    g.write::<u32>(rec, node);
    g.write::<u32>(node, w(0x53));
    for (j, v) in (1u32..).zip([w(0x5d), r + 0x888, x, w(0x67), 1]) {
        g.write::<u32>(rec + 4 * j, v);
    }
    crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x73), rec);
    let tp = ebp.wrapping_sub(0x18);
    // call width: 2
    g.call(slot_target(g, w(0x83), 325), 0, &[tp, r + 0x9d4]);
    for (j, op) in (0u32..).zip([0x90u32, 0xad, 0xe4]) {
        let o = g.call(slot_target(g, w(op), 307), 0, &[tp, r + 0x9d4]);
        let rnd = crate::ob_model::st0_f32(o.st0);
        let (k1, k2, k3) = [
            (0xa8u32, 0xb5u32, 0xbdu32),
            (0xdf, 0xec, 0xf4),
            (0x116, 0x124, 0x12c),
        ][j as usize];
        let d = sub(mul(sub(rnd, k(g, k1)), k(g, k2)), k(g, k3));
        let v = add(d, g.read::<f32>(tp + 4 * j));
        g.write::<f32>(tp + 4 * j, v);
    }
    let i = u32::from(g.read::<u16>(r + 0x20));
    let kind = u32::from(i >= 7);
    // the call's frame: 325's two words and the two arguments below `esi`,
    // `edi`
    let esp = ebp.wrapping_sub(0x3c).wrapping_sub(0x14);
    g.write::<u32>(esp + 4, tp);
    g.write::<u32>(esp + 8, kind);
    let mut eax = object_start_0179(
        g,
        img,
        img.rel32_target(f + 0x15a),
        esp,
        r + 0x9f0 + i * 0x480,
        r + 0x84c,
    );
    let n = g.read::<u16>(r + 0x20).wrapping_add(1);
    g.write::<u16>(r + 0x20, n);
    if n >= 8 {
        eax = unlink_task(g, task);
    }
    let m = add(g.read::<f32>(r + 0x1c), k(g, 0x18f));
    g.write::<f32>(r + 0x1c, m);
    Ret::eax(eax)
}

/// The task node `*task` taken out of its list (its neighbours joined,
/// its links to itself). Returns its old `+4` as last read.
fn unlink_task<G: Game>(g: &mut G, task: u32) -> u32 {
    let node = g.read::<u32>(task);
    let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
    g.write::<u32>(prev + 8, next);
    let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
    g.write::<u32>(next + 4, prev);
    g.write::<u32>(node + 8, node);
    g.write::<u32>(node + 4, node);
    prev
}

/// [`throw_task_0179`] (424 bytes, 1 overlay).
pub const THROW_TASK_0179: Sig = Sig {
    len: 424,
    hash: 0x94ce_c906_aef2_ec34,
    calls: &[
        (0x73, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x15a, Callee::Sig(&OBJECT_START_0179)),
        // __security_check_cookie
        (0x1a0, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0179`'s thrown object step (0x100069c0, 409 bytes; the object's
/// step function, cdecl `(task)`, the object `O = task[1]`; an `ebp` frame
/// with the set-up record at `ebp - 0x1c`): the timer `O + 0x18` grows by
/// `K`, past `K2` the state `O + 0x1c` is 1. State 0, once the timer
/// reaches `K3`: slot 239 `(*G_A, -5, 0)` (kind `O + 0x1e` set), else slot
/// 388 `(*G_B, sound[rand & 1], 1)`; the node `O + 0x300` appended to the
/// list `G_L` ([`node_append`](crate::sb_misc::node_append)), slot 325
/// `(O + 0x31c, O + 0x1f0)`, [`node_insert`](crate::nodes::node_insert) of
/// `(O + 0x32c, ...)`, [`particle_node`](crate::sb_nodes::particle_node)
/// of `(O + 0x200, ...)`, `O + 0x1dc` and `O + 0x1e4` scaled, the state 1,
/// and on into state 1: the task node unlinked. Returns the node's old
/// `+4`, 0 while state 0 waits, the state less 1 for others.
pub fn object_step_0179<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| g.read::<f32>(w(off));
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 4);
    let timer = add(g.read::<f32>(o + 0x18), k(g, 0x1d));
    g.write::<f32>(o + 0x18, timer);
    // comiss timer, K2; jb: below (or NaN) keeps the state
    let k2 = k(g, 0x24);
    if !(timer.is_nan() || k2.is_nan() || timer < k2) {
        g.write::<u16>(o + 0x1c, 1);
    }
    let state = i32::from(g.read::<u16>(o + 0x1c).cast_signed());
    match state {
        0 => {
            // comiss K3, timer; ja: below the mark waits
            if k(g, 0x4e) > timer {
                return Ret::eax(0);
            }
            let eax = if g.read::<u16>(o + 0x1e) != 0 {
                let a = g.read::<u32>(w(0x73));
                // call width: 3
                g.call(slot_target(g, w(0x61), 239), 0, &[a, 0xffff_fffb, 0])
                    .eax
            } else {
                let r = g.call(slot_target(g, w(0x61), 324), 0, &[]).eax & 1;
                let snd = g.read::<u32>(w(0x89).wrapping_add(4 * r));
                let b = u32::from(g.read::<u8>(w(0x90)));
                // call width: 3
                g.call(slot_target(g, w(0x96), 388), 0, &[b, snd, 1]).eax
            };
            let node = o + 0x300;
            crate::sb_misc::node_append(g, img, img.rel32_target(f + 0xb3), node, w(0xac), eax);
            // call width: 2
            g.call(slot_target(g, w(0xc6), 325), 0, &[o + 0x31c, o + 0x1f0]);
            let ebp = fr.esp.wrapping_sub(4);
            let rec = ebp.wrapping_sub(0x1c);
            let s = o + 0x32c;
            for (i, x) in (0u32..).zip([s, w(0xe4), w(0xeb), node, w(0xf5), w(0xfc)]) {
                g.write::<u32>(rec + 4 * i, x);
            }
            let ins = Frame {
                at: img.rel32_target(f + 0x101),
                esp: ebp.wrapping_sub(0x30),
                ecx: rec,
                ..Frame::default()
            };
            crate::nodes::node_insert(g, img, &ins);
            let x = g.read::<u32>(w(0x106));
            let p = o + 0x200;
            g.write::<u32>(p, w(0x115));
            for (i, v) in (0u32..).zip([p, w(0x11f), s, x, w(0x12c), 0]) {
                g.write::<u32>(rec + 4 * i, v);
            }
            crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x138), rec);
            let v = mul(g.read::<f32>(o + 0x1dc), k(g, 0x148));
            g.write::<f32>(o + 0x1dc, v);
            let v = mul(g.read::<f32>(o + 0x1e4), k(g, 0x165));
            g.write::<f32>(o + 0x1e4, v);
            g.write::<u16>(o + 0x1c, 1);
            Ret::eax(unlink_task(g, task))
        }
        1 => Ret::eax(unlink_task(g, task)),
        s => Ret::eax(s.wrapping_sub(1).cast_unsigned()),
    }
}

/// [`object_step_0179`] (409 bytes, 1 overlay).
pub const OBJECT_STEP_0179: Sig = Sig {
    len: 409,
    hash: 0x5472_c1d9_d57e_00b2,
    calls: &[
        (0xb3, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x101, Callee::Sig(&crate::nodes::INSERT)),
        (0x138, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
    ],
    imports: &[],
};

/// A static node appended to the static list as `magic_0179`'s functions
/// inline it: the list's last read (operand `last`), the node's own words
/// (the store run `pre`), its `+8` = the old last (the store at `n8`); an
/// empty list (the old last is the list, operand `cmp`) gets it as its
/// first (the store at `eq`), else the old last's `+4` is it (the
/// immediate `ne`); the count up (operand `count`).
fn static_append_0179<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (last, n8, cmp, eq, ne, count): (u32, u32, u32, u32, u32, u32),
    pre: (u32, u32),
) {
    use crate::r25::abs_stores;
    let w = |o: u32| img.word(f + o);
    let old = g.read::<u32>(w(last));
    abs_stores(g, img, f, pre.0, pre.1);
    g.write::<u32>(w(n8), old);
    if old == w(cmp) {
        abs_stores(g, img, f, eq, eq + 10);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(ne));
    }
    let n = g.read::<u16>(w(count)).wrapping_add(1);
    g.write::<u16>(w(count), n);
}

/// `magic_0179`'s second throw start (0x10006e80, 657 bytes; internal, no
/// arguments; an 8-aligned cookie frame, `A = ((E - 4) & !7) - 0x30`, the
/// vector at `A + 0x1c`, the set-up record at `A + 4`): the DLL's task
/// [`throw_task_0179`] joins the ring at its end (its timer, mark and
/// count 0); the static node `S1` joins the static list and slot 229
/// `(S1 + 0x1c, G_M, (0, -7.5, -10, 1))` places it; the static node `S2` joins
/// it with its colour 1.0s and 0.36;
/// [`node_insert`](crate::nodes::node_insert) and
/// [`effect_node`](crate::sb_nodes::effect_node) of the
/// static records, the node's vtable, a flag bit 0 set and a word 0, slot
/// 325 `(G_D, G_S)`; the word `*G_Y` less `*G_H K`. Returns slot 325's
/// `eax`.
pub fn throw_start_0179<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    use crate::r25::abs_stores;
    use crate::sb_sse::{mul, sub};
    let w = |off: u32| img.word(f + off);
    let rd = |g: &G, off: u32| g.read::<u32>(w(off));
    let a = (esp.wrapping_sub(4) & !7).wrapping_sub(0x30);
    let last = rd(g, 0x15);
    g.write::<u32>(w(0x1a), last);
    abs_stores(g, img, f, 0x1e, 0x46);
    g.write::<u32>(last.wrapping_add(4), w(0x49));
    let fl = rd(g, 0x4f);
    g.write::<u32>(w(0x4f), fl & !1);
    g.write::<u16>(w(0x58), 0);
    g.write::<u32>(w(0x5d), 0);
    g.write::<u32>(w(0x62), 0);
    let old = rd(g, 0x67);
    abs_stores(g, img, f, 0x6b, 0x75);
    abs_stores(g, img, f, 0x75, 0xa6);
    g.write::<u32>(w(0xa7), old);
    if old == w(0xac) {
        abs_stores(g, img, f, 0xb2, 0xbc);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(0xc1));
    }
    let n = g.read::<u16>(w(0xc8)).wrapping_add(1);
    g.write::<u16>(w(0xc8), n);
    abs_stores(g, img, f, 0xd6, 0xea);
    let v = a + 0x1c;
    for (i, x) in (0u32..).zip([0, 0xc0f0_0000, 0xc120_0000, 0x3f80_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 3
    g.call(slot_target(g, w(0xd2), 229), 0, &[w(0xf6), w(0xf1), v]);
    static_append_0179(
        g,
        img,
        f,
        (0x11d, 0x156, 0x15b, 0x161, 0x170, 0x177),
        (0x124, 0x155),
    );
    abs_stores(g, img, f, 0x17f, 0x1bb);
    let rec = a + 4;
    let words = [w(0x1bf), w(0x1c7), w(0x1cf), w(0x1d7), w(0x1df), w(0x1e7)];
    for (i, x) in (0u32..).zip(words) {
        g.write::<u32>(rec + 4 * i, x);
    }
    let fr = Frame {
        at: img.rel32_target(f + 0x1ec),
        esp: a.wrapping_sub(4),
        ecx: rec,
        ..Frame::default()
    };
    crate::nodes::node_insert(g, img, &fr);
    let words = [w(0x1f8), w(0x200), w(0x208), w(0x210), w(0x218), 0];
    for (i, x) in (0u32..).zip(words) {
        g.write::<u32>(rec + 4 * i, x);
    }
    crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x225), rec);
    let node = g.read::<u32>(rec);
    g.write::<u32>(node, w(0x234));
    let fl = rd(g, 0x23f);
    g.write::<u32>(w(0x23f), fl | 1);
    g.write::<u32>(w(0x246), 0);
    // call width: 2
    let eax = g
        .call(slot_target(g, w(0x239), 325), 0, &[w(0x255), w(0x22e)])
        .eax;
    let d = mul(g.read::<f32>(w(0x25f)), g.read::<f32>(w(0x267)));
    let y = sub(g.read::<f32>(w(0x26f)), d);
    g.write::<f32>(w(0x284), y);
    eax
}

/// [`throw_start_0179`] (657 bytes, 1 overlay).
pub const THROW_START_0179: Sig = Sig {
    len: 657,
    hash: 0x3904_b0fe_bfde_6d5f,
    calls: &[
        (0x1ec, Callee::Sig(&crate::nodes::INSERT)),
        (0x225, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        // __security_check_cookie
        (0x289, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0179`'s summon task (0x10007120, 372 bytes; a task function,
/// cdecl `(task)`): by the state halfword `*G_S` (a table of five):
/// 0: slot 696 `(*G_O, 0xba)`, state 1, the timer `*G_T` 0; 1: the timer
/// up by `K`, from `K2` on [`throw_start_0179`] and state 2, then as 2;
/// 2: once slot 240 `(*G_O)` says done, slot 696 `(*G_O, 0xbb)`, state 3
/// and the timer 0; 3: the timer up, from `K3` on slot 696 `(*G_O, 0xbc)`
/// and state 4; 4: once slot 240 says done, slot 213 `(*G_O, 0)` and the
/// task node unlinked. Returns the new state, the old one while waiting
/// (slot 240's 0), or the node's old `+4`.
pub fn summon_task_0179<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, off: u32| g.read::<f32>(w(off));
    let rd = |g: &G, off: u32| g.read::<u32>(w(off));
    let task = fr.arg(g, 0);
    let state = u32::from(g.read::<u16>(w(0x6)));
    // the state's store (operand `o`)
    let set = |g: &mut G, o: u32, s: u16| g.write::<u16>(w(o), s);
    // the wait of states 2 and 4: slot 240 `(*G_O)` (operands `t`, `o`)
    let done = |g: &mut G, t: u32, o: u32| {
        let x = rd(g, o);
        g.call(slot_target(g, w(t), 240), 0, &[x]).eax
    };
    let mut eax = state;
    let mut s = state;
    if s == 0 {
        let x = rd(g, 0x26);
        // call width: 2
        g.call(slot_target(g, w(0x1b), 696), 0, &[x, 0xba]);
        set(g, 0x39, 1);
        g.write::<u32>(w(0x3f), 0);
        return Ret::eax(1);
    }
    if s == 1 {
        let t = add(k(g, 0x50), k(g, 0x58));
        g.write::<f32>(w(0x6b), t);
        // comiss K2, t; ja: below the mark waits
        if k(g, 0x60) > t {
            return Ret::eax(eax);
        }
        throw_start_0179(g, img, img.rel32_target(f + 0x76), fr.esp.wrapping_sub(8));
        set(g, 0x81, 2);
        s = 2;
    }
    if s == 2 {
        eax = done(g, 0x86, 0x8c);
        if eax == 0 {
            return Ret::eax(0);
        }
        let x = rd(g, 0xaf);
        // call width: 2
        g.call(slot_target(g, w(0xa4), 696), 0, &[x, 0xbb]);
        set(g, 0x39, 3);
        g.write::<u32>(w(0x3f), 0);
        return Ret::eax(3);
    }
    if s == 3 {
        let t = add(k(g, 0xc9), k(g, 0xd1));
        let k3 = k(g, 0xd8);
        g.write::<f32>(w(0xe0), t);
        // comiss t, K3; jb: below (or NaN) waits
        if t.is_nan() || k3.is_nan() || t < k3 {
            return Ret::eax(eax);
        }
        let x = rd(g, 0xf2);
        // call width: 2
        g.call(slot_target(g, w(0xe7), 696), 0, &[x, 0xbc]);
        set(g, 0x105, 4);
        return Ret::eax(4);
    }
    if s == 4 {
        if done(g, 0x10f, 0x115) == 0 {
            return Ret::eax(0);
        }
        let x = rd(g, 0x131);
        // call width: 2
        g.call(slot_target(g, w(0x129), 213), 0, &[x, 0]);
        return Ret::eax(unlink_task(g, task));
    }
    Ret::eax(eax)
}

/// [`summon_task_0179`] (372 bytes, 1 overlay).
pub const SUMMON_TASK_0179: Sig = Sig {
    len: 372,
    hash: 0x52db_7dbc_926e_09e1,
    calls: &[(0x76, Callee::Sig(&THROW_START_0179))],
    imports: &[],
};

/// `magic_0179`'s start-up (0x100072a0, 2094 bytes; a table entry, cdecl
/// `(actor)`; an 8-aligned frame, `A = ((E - 4) & !7) - 0x18`, the set-up
/// records at `A`). The targets
/// ([`targets_init_b`](crate::s4_summon::targets_init_b)); the DLL's ring, its statics' list and twelve static
/// nodes with their words, each appended to the list as the compiler
/// unrolled it (the list's last tested against the node before, the count
/// kept in a register and stored three times), the caster's byte `*G_C`
/// and the selection byte `*G_SEL` into three nodes each; two
/// [`node_insert`](crate::nodes::node_insert)s; the yaw from the caster
/// to the selected actor ([`yaw_0179`]) to slot 666 `(*G_C, yaw)` and
/// `*G_YAW`; the count of steps `(K - yaw) K2 / K3` (`cvttss2si`); slot
/// 718, 719, 711 (under the game lock), 710, 222, 221 for the parts; the
/// summon task [`summon_task_0179`] joins the ring with the state 0.
/// Returns 0.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn start_up_0179<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let st = |g: &mut G, from: u32, to: u32| abs_stores(g, img, f, from, to);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let actor = fr.arg(g, 0);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x18);
    // push ebx; push esi: a callee's return address below
    let ce = a.wrapping_sub(0xc);
    crate::s4_summon::targets_init_b(g, img, img.rel32_target(f + 0xf), ce, actor);
    // `cmp ecx, L; jne`: the node before is the list itself (operand
    // `cmp`) or not; the store at `eq` or `ne`
    let link = |g: &mut G, before: u32, cmp: u32, eq: u32, ne: u32| {
        if before == w(cmp) {
            abs_stores(g, img, f, eq, eq + 10);
        } else {
            abs_stores(g, img, f, ne, ne + 10);
        }
    };
    // `cmp eax, L; jne` with the list's last in `eax`
    let link_last = |g: &mut G, last: u32, cmp: u32, eq: u32, ne: u32| {
        if last == w(cmp) {
            abs_stores(g, img, f, eq, eq + 10);
        } else {
            g.write::<u32>(last.wrapping_add(4), w(ne));
        }
    };
    let insert = |g: &mut G, ops: [u32; 6], call: u32| {
        for (i, o) in (0u32..).zip(ops) {
            g.write::<u32>(a + 4 * i, w(o));
        }
        let fr = Frame {
            at: img.rel32_target(f + call),
            esp: ce,
            ecx: a,
            ..Frame::default()
        };
        crate::nodes::node_insert(g, img, &fr);
    };
    let mut count = g.read::<u16>(w(0x15));
    let before = w(0x1a);
    count = count.wrapping_add(1);
    st(g, 0x20, 0x145);
    link(g, before, 0x147, 0x14d, 0x159);
    let before = w(0x164);
    count = count.wrapping_add(1);
    st(g, 0x16a, 0x1d7);
    link(g, before, 0x1d9, 0x1df, 0x1eb);
    let caster = rd(g, 0x1f7);
    count = count.wrapping_add(1);
    st(g, 0x201, 0x20b);
    g.write::<u16>(w(0x20d), count);
    st(g, 0x211, 0x243);
    for o in [0x245, 0x24b, 0x251] {
        g.write::<u8>(w(o), caster as u8);
    }
    insert(g, [0x259, 0x261, 0x269, 0x271, 0x279, 0x281], 0x286);
    let last = rd(g, 0x28b);
    st(g, 0x28f, 0x2c0);
    g.write::<u32>(w(0x2c1), last);
    link_last(g, last, 0x2c6, 0x2cc, 0x2db);
    let mut count = g.read::<u16>(w(0x2e1));
    let before = w(0x2e6);
    count = count.wrapping_add(1);
    st(g, 0x2ec, 0x342);
    link(g, before, 0x344, 0x34a, 0x356);
    let before = w(0x361);
    count = count.wrapping_add(1);
    st(g, 0x367, 0x3d4);
    link(g, before, 0x3d6, 0x3dc, 0x3e8);
    count = count.wrapping_add(1);
    g.write::<u16>(w(0x3f6), count);
    let sel = g.read::<u8>(w(0x3fb));
    st(g, 0x403, 0x43f);
    for o in [0x440, 0x445, 0x44a] {
        g.write::<u8>(w(o), sel);
    }
    insert(g, [0x452, 0x45a, 0x462, 0x46a, 0x472, 0x47a], 0x47f);
    let last = rd(g, 0x484);
    st(g, 0x488, 0x4b9);
    g.write::<u32>(w(0x4ba), last);
    link_last(g, last, 0x4bf, 0x4c5, 0x4d4);
    let mut count = g.read::<u16>(w(0x4da));
    let before = w(0x4df);
    count = count.wrapping_add(1);
    st(g, 0x4e5, 0x552);
    link(g, before, 0x554, 0x55a, 0x566);
    let before = w(0x571);
    count = count.wrapping_add(1);
    st(g, 0x577, 0x5ee);
    link(g, before, 0x5f0, 0x5f6, 0x602);
    let target = u32::from(g.read::<u8>(w(0x60f)));
    count = count.wrapping_add(1);
    st(g, 0x617, 0x621);
    g.write::<u16>(w(0x623), count);
    st(g, 0x627, 0x645);
    let yaw = yaw_0179(g, img, img.rel32_target(f + 0x646), ce, caster, target);
    let c = rd(g, 0x65d);
    g.write::<f32>(w(0x665), yaw);
    // call width: 2
    g.call(t(g, 0x64b, 666), 0, &[c, yaw.to_bits()]);
    let last = rd(g, 0x66c);
    st(g, 0x673, 0x6ae);
    g.write::<u32>(w(0x6af), last);
    link_last(g, last, 0x6b4, 0x6ba, 0x6c9);
    let x = sub(g.read::<f32>(w(0x6d1)), g.read::<f32>(w(0x6d9)));
    let n = g.read::<u16>(w(0x6e0)).wrapping_add(1);
    g.write::<u16>(w(0x6e0), n);
    st(g, 0x6e4, 0x702);
    let x = div(mul(x, g.read::<f32>(w(0x706))), g.read::<f32>(w(0x713)));
    g.write::<u32>(w(0x71c), ffx_bridge::x87::cvttsd2si(f64::from(x)));
    g.call(t(g, 0x721, 718), 0, &[w(0x70b)]);
    // call width: 3
    let h = g.call(t(g, 0x72e, 719), 0, &[w(0x740), 8, 0]).eax;
    let lock = rd(g, 0x748);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(w(0x754), h);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(t(g, 0x75f, 711), 0, &[h, w(0x759), 0]);
    g.write::<u32>(rd(g, 0x772), saved);
    let hh = rd(g, 0x792);
    // call width: 4
    g.call(t(g, 0x77c, 710), 0, &[w(0x797), hh, w(0x78c), 0x20_0000]);
    // call width: 4
    g.call(t(g, 0x79e, 222), 0, &[w(0x7b2), 0, 0x3e8, 2]);
    // call width: 4
    g.call(t(g, 0x7b9, 221), 0, &[w(0x7cd), 0, 0x3e8, 2]);
    let last = rd(g, 0x7d4);
    g.write::<u32>(w(0x7dc), last);
    st(g, 0x7e0, 0x808);
    g.write::<u32>(last.wrapping_add(4), w(0x80b));
    let fl = rd(g, 0x811);
    g.write::<u32>(w(0x811), fl & !1);
    st(g, 0x819, 0x823);
    g.write::<u16>(w(0x825), 0);
    Ret::eax(0)
}

/// [`start_up_0179`] (2094 bytes, 1 overlay).
pub const START_UP_0179: Sig = Sig {
    len: 2094,
    hash: 0xb780_8a0c_bf9b_3df5,
    calls: &[
        (0xf, Callee::Sig(&crate::s4_summon::TARGETS_INIT_B)),
        (0x286, Callee::Sig(&crate::nodes::INSERT)),
        (0x47f, Callee::Sig(&crate::nodes::INSERT)),
        (0x646, Callee::Sig(&YAW_0179)),
    ],
    imports: &[],
};

/// `magic_0174`'s target actor `i` (`-1`: the caster word at operand
/// `caster`; else the actor table at operand `table` by the selection
/// byte `(*G_SEL)[i]`, `G_SEL` at operand `sel`).
fn actor_0174<G: Game>(
    g: &G,
    img: &Image,
    f: u32,
    i: u32,
    (caster, sel, table): (u32, u32, u32),
) -> u32 {
    let w = |o: u32| img.word(f + o);
    if i == u32::MAX {
        g.read::<u32>(w(caster))
    } else {
        let b = u32::from(g.read::<u8>(g.read::<u32>(w(sel)).wrapping_add(i)));
        g.read::<u32>(w(table).wrapping_add(4 * b))
    }
}

/// Slot 256 `(actor, k, k, k, n)` (the colour of an actor's flash; the
/// imports at operand `op`).
fn flash_0174<G: Game>(g: &mut G, img: &Image, f: u32, op: u32, actor: u32, k: u32, n: u32) {
    let s = slot_target(g, img.word(f + op), 256);
    // call width: 5
    g.call(s, 0, &[actor, k, k, k, n]);
}

/// `magic_0174`'s targets' hits (0x10001800, 715 bytes; internal, no
/// arguments; an 8-aligned cookie frame, `A = ((E - 4) & !7) - 0x40`: the
/// done flag at `A + 4`, the actor at `A + 8`, the point `B` at `A +
/// 0xc`, `C` at `A + 0x1c`, the actor's point at `A + 0x2c`). Without
/// the start flag `*G_GO` it returns 0. Per target `i` (record `R` of
/// 0x160 bytes from `G_R`): a hit one (`R + 0x84` 1) counts `R + 0x80`
/// down (at 0 the target flashes 1.0 for 8 frames, slot 256), and on
/// below 0 to -8; else the summon's way `B` (the matrix `*G_M` set, slot
/// 229 `(B, G_P, B)` with `(0, 0, -R[0x7c], 1)`, slot 248 `(B, B, R +
/// 0x6c)`, slot 234 `(C, B, R + 0x5c)`) is checked against `R[0x7c]^2 K`
/// (`_libm_sse2_sqrt_precise` of `C`'s flat length): within it an object
/// starts ([`start_object_at`](crate::s4_misc::start_object_at), scale
/// `K2`, the callback), the target's point `(R - 4, R, R + 4)` (slot 262),
/// slot 268 `(actor, 6, P)`, `R[0]` = `P.y`, `R + 8` 1.0, `R + 0x4c` the
/// selection byte, the hit with 0x28 frames, and a 0.5 flash. Returns 1
/// when every target has finished (counted down to -8), else 0.
#[allow(clippy::too_many_lines)] // one loop
pub fn hits_0174<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    use crate::sb_sse::{add, mul};
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    if rd(g, 0x16).cast_signed() < 1 {
        return 0;
    }
    let a = (esp.wrapping_sub(4) & !7).wrapping_sub(0x40);
    let (flag, b, c) = (a + 4, a + 0xc, a + 0x1c);
    let mut ecx = 1u32;
    g.write::<u32>(flag, 1);
    let mut i = 0u32;
    let mut r = w(0x4a);
    // the count read at operand +0x3f first, +0x2ae after each target
    let mut count = rd(g, 0x3f);
    while i.cast_signed() < count.cast_signed() {
        if g.read::<u32>(r + 0x84) == 1 {
            let n = g.read::<u32>(r + 0x80);
            if n.cast_signed() > 0 {
                let n = n.wrapping_sub(1);
                g.write::<u32>(r + 0x80, n);
                if n == 0 {
                    let actor = actor_0174(g, img, f, i, (0x76, 0x7d, 0x88));
                    flash_0174(g, img, f, 0xaa, actor, 0x3f80_0000, 8);
                }
                ecx = 0;
                g.write::<u32>(flag, 0);
            } else if n.cast_signed() <= -8 {
                ecx = g.read::<u32>(flag);
            } else {
                g.write::<u32>(r + 0x80, n.wrapping_sub(1));
                ecx = 0;
                g.write::<u32>(flag, 0);
            }
        } else {
            // movaps: the 16-byte constant into the game's matrix word
            let (src, dst) = (w(0xe2), w(0xee));
            for j in 0..4 {
                let x = g.read::<u32>(src + 4 * j);
                g.write::<u32>(dst + 4 * j, x);
            }
            let z = g.read::<u32>(r + 0x7c) ^ g.read::<u32>(w(0xfa));
            for (j, x) in (0u32..).zip([0, 0, z, 0x3f80_0000]) {
                g.write::<u32>(b + 4 * j, x);
            }
            g.write::<u32>(flag, 0);
            // call width: 3
            g.call(slot_target(g, w(0x105), 229), 0, &[b, w(0xff), b]);
            // call width: 3
            g.call(slot_target(g, w(0x142), 248), 0, &[b, b, r + 0x6c]);
            // call width: 3
            g.call(slot_target(g, w(0x15d), 234), 0, &[c, b, r + 0x5c]);
            let (cx, cz) = (rf(g, c), rf(g, c + 8));
            let sq = add(mul(cz, cz), mul(cx, cx));
            // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
            let d = f64::from(sq).sqrt() as f32;
            let rr = rf(g, r + 0x7c);
            let reach = mul(mul(rr, rr), g.read::<f32>(w(0x19d)));
            // comiss reach, d; jbe: not beyond the reach (or NaN) skips
            if reach > d {
                let scale = g.read::<f32>(w(0x1ae));
                crate::s4_misc::start_object_at(
                    g,
                    img,
                    img.rel32_target(f + 0x1c3),
                    r.wrapping_sub(0xd4),
                    scale,
                    1,
                    0,
                    w(0x1b3),
                );
                let actor = actor_0174(g, img, f, i, (0x1d1, 0x1d8, 0x1e3));
                g.write::<u32>(a + 8, actor);
                // call width: 4
                g.call(
                    slot_target(g, w(0x1f1), 262),
                    0,
                    &[actor, r.wrapping_sub(4), r, r + 4],
                );
                let p = a + 0x2c;
                // call width: 3
                g.call(slot_target(g, w(0x208), 268), 0, &[actor, 6, p]);
                let y = g.read::<u32>(p + 4);
                let sel = rd(g, 0x222);
                let all = rd(g, 0x227);
                g.write::<u32>(r, y);
                g.write::<u32>(r + 8, 0x3f80_0000);
                let k = if i == u32::MAX {
                    all
                } else {
                    u32::from(g.read::<u8>(sel.wrapping_add(i)))
                };
                g.write::<u32>(r + 0x4c, k);
                g.write::<u32>(r + 0x84, 1);
                g.write::<u32>(r + 0x80, 0x28);
                let actor = if i == u32::MAX {
                    rd(g, 0x25f)
                } else {
                    let b = u32::from(g.read::<u8>(sel.wrapping_add(i)));
                    g.read::<u32>(w(0x26c).wrapping_add(4 * b))
                };
                flash_0174(g, img, f, 0x28e, actor, 0x3f00_0000, 8);
                ecx = g.read::<u32>(flag);
            } else {
                ecx = 0;
            }
        }
        i = i.wrapping_add(1);
        r = r.wrapping_add(0x160);
        count = rd(g, 0x2ae);
    }
    ecx
}

/// [`hits_0174`] (715 bytes, 1 overlay).
pub const HITS_0174: Sig = Sig {
    len: 715,
    hash: 0xd642_f673_e77d_9639,
    calls: &[
        // __security_check_cookie
        (0x2a, Callee::Any),
        (0x188, Callee::Import("_libm_sse2_sqrt_precise")),
        (0x1c3, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x2c3, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0174`'s summon frame (0x10001760, 145 bytes; cdecl `(task)`):
/// the objects run ([`run_objects`](crate::l5_objhost::run_objects)), the
/// targets' hits ([`hits_0174`]) and the motion
/// ([`motion_step`](crate::s3_motion::motion_step)); when both the objects
/// and the motion are done, every target flashes 1.0 once (slot 256), the
/// task's state `+0x14` is 3 and its node's step the DLL's countdown.
/// Returns the motion's `eax`, or the task's `+8` after.
pub fn summon_frame_0174<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    // push ebp; push esi: a callee's return address below
    let ce = fr.esp.wrapping_sub(0xc);
    let objects = crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x5), ce);
    hits_0174(g, img, img.rel32_target(f + 0xc), ce);
    let eax = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x11));
    if eax != 1 || objects != 1 {
        return Ret::eax(eax);
    }
    // the count read at operand +0x22 first, +0x74 after each flash
    let mut i = 0u32;
    let mut count = g.read::<u32>(w(0x22));
    while i.cast_signed() < count.cast_signed() {
        let actor = actor_0174(g, img, f, i, (0x2e, 0x35, 0x40));
        flash_0174(g, img, f, 0x62, actor, 0x3f80_0000, 1);
        i += 1;
        count = g.read::<u32>(w(0x74));
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let node = g.read::<u32>(task + 8);
    g.write::<u32>(node + 0x10, w(0x8a));
    Ret::eax(node)
}

/// [`summon_frame_0174`] (145 bytes, 1 overlay).
pub const SUMMON_FRAME_0174: Sig = Sig {
    len: 145,
    hash: 0xf58b_dd5b_c021_6f04,
    calls: &[
        (0x5, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0xc, Callee::Sig(&HITS_0174)),
        (0x11, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
    ],
    imports: &[],
};

/// `magic_0174`'s object turn (0x10001c50, 634 bytes; an object callback,
/// cdecl `(obj)`; an 8-aligned cookie frame, `A = ((E - 4) & !7) - 0x70`:
/// the angle at `A`, the bone matrix `M` at `A + 8`, the forward point
/// `(0, 0, -1000, 0)` at `A + 0x4c` and its image `V` at `A + 0x5c`). Past
/// 0x50 frames (`obj + 0x118`) the callback `obj + 0x104` is cleared;
/// else slot 236 `(*G_C, 0x15, M)` gives the caster's bone 0x15, slot 277
/// `(*G_C)` its scale `s` (ST0), the three rows of `M` are scaled by `K /
/// s`, the translation cleared, slot 229 `(V, M, F)`, the yaw `K2 -
/// atan2(V.z, V.x)` (`_CIatan2`) wrapped once into `[K3, K4]`; slot 235
/// `(obj + 0x80)` and slot 329 `(obj + 0x80, obj + 0x80, yaw)` turn the
/// object, slot 237 `(*G_C, 0x15, 0, obj + 0xd0)` places it, `obj + 0xdc`
/// 1.0, `obj + 0xd4` = slot 12 `(*G_C)` (ST0) less `K5`, the mode byte
/// `obj + 0x126` 2. Returns slot 12's `eax` (the cookie's on the early
/// way).
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn object_turn_0174<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let obj = fr.arg(g, 0);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x70);
    if g.read::<u32>(obj + 0x118).cast_signed() > 0x50 {
        g.write::<u32>(obj + 0x104, 0);
        return Ret::eax(rd(g, 0xa) ^ a);
    }
    let m = a + 8;
    let c1 = rd(g, 0x4b);
    // call width: 3
    g.call(t(g, 0x43, 236), 0, &[c1, 0x15, m]);
    let c2 = rd(g, 0x5e);
    // 236's words stay below
    let s = crate::ob_model::st0_f32(g.call(t(g, 0x58, 277), 0, &[c2, c1, 0x15, m]).st0);
    g.write::<f32>(a, s);
    let q = div(k(g, 0x72), s);
    for row in [0u32, 0x10, 0x20] {
        for j in 0..3 {
            let at = m + row + 4 * j;
            let x = mul(g.read::<f32>(at), q);
            g.write::<f32>(at, x);
        }
    }
    let (fwd, v) = (a + 0x4c, a + 0x5c);
    for (j, x) in (0u32..).zip([0xc47a_0000, 0, 0, 0x3f80_0000]) {
        g.write::<u32>(fwd + 4 * j, x);
    }
    for o in [0x30, 0x34, 0x38] {
        g.write::<u32>(m + o, 0);
    }
    // call width: 3
    g.call(t(g, 0xc6, 229), 0, &[v, m, fwd]);
    let (vz, vx) = (g.read::<f32>(v + 8), g.read::<f32>(v));
    let ang = g.atan2(f64::from(vz), f64::from(vx)) as f32;
    let mut yaw = sub(k(g, 0x19f), ang);
    // comiss yaw, K3; jbe: not above it (or NaN) tries the low end
    if yaw > k(g, 0x1ae) {
        yaw = sub(yaw, k(g, 0x1be));
    } else if k(g, 0x1c8) > yaw {
        yaw = add(yaw, k(g, 0x1d5));
    }
    g.write::<f32>(a, yaw);
    let rot = obj + 0x80;
    g.call(t(g, 0x1e0, 235), 0, &[rot]);
    // call width: 3
    g.call(t(g, 0x1fa, 329), 0, &[rot, rot, yaw.to_bits()]);
    let pos = obj + 0xd0;
    let c1 = rd(g, 0x225);
    // call width: 4
    g.call(t(g, 0x20e, 237), 0, &[c1, 0x15, 0, pos]);
    g.write::<u32>(pos + 0xc, 0x3f80_0000);
    let c2 = rd(g, 0x239);
    // 237's words stay below
    // call width: 5
    let o = g.call(t(g, 0x22c, 12), 0, &[c2, c1, 0x15, 0, pos]);
    let h = crate::ob_model::st0_f32(o.st0);
    g.write::<f32>(a, h);
    g.write::<f32>(obj + 0xd4, sub(h, k(g, 0x250)));
    g.write::<u8>(obj + 0x126, 2);
    Ret::eax(o.eax)
}

/// [`object_turn_0174`] (634 bytes, 1 overlay).
pub const OBJECT_TURN_0174: Sig = Sig {
    len: 634,
    hash: 0x0aef_a6c5_8421_118a,
    calls: &[
        // __security_check_cookie
        (0x35, Callee::Any),
        (0x18d, Callee::Import("_CIatan2")),
        (0x272, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0174`'s start-up (0x10001340, 1052 bytes; internal, no
/// arguments; an 8-aligned cookie frame, `A = ((E - 4) & !7) - 0x3c`: the
/// actor at `A + 4`, the caster's point `Q` at `A + 8`, `D` at `A +
/// 0x18`, the bone point `P` at `A + 0x28`):
/// [`start_up_0172`](crate::r17::start_up_0172) with two objects: slot 719 `(NAME, 8, 0)`
/// into `G_P` (two words first), the part set up
/// ([`part_init`](crate::l5_objhost::part_init)); the object `O1`
/// ([`start_object_at`](crate::s4_misc::start_object_at) `(0, 0x10, -, 0)`)
/// at the caster (slot 262 into three globals, slot 268's bone 6 height,
/// `w` 1.0) with its `+0x68` scaled; the record words; `O2` (`(2, 0x10,
/// -, 0)`) at the caster with its ground (slot 12), its `+0x68` scaled;
/// `Q` = the caster's point with its ground, `w` 1.0; per target its
/// record (`G_R + 0x160 i`): its point with its ground, `w` 1.0, `Q`, the
/// flat distance (slot 234, `_libm_sse2_sqrt_precise`), `+0x24` 0; slot
/// 215 or 413 (by `*G_F`) of `*G_N`; a node of the free ring
/// ([`ring_take`](crate::r15::ring_take); an empty ring writes through
/// address 0 as the original does) last on the task list. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0174<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::ob_model::st0_f32;
    use crate::r25::abs_stores;
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let kf = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let st = |g: &mut G, from: u32, to: u32| abs_stores(g, img, f, from, to);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x3c);
    let (q, d, p) = (a + 8, a + 0x18, a + 0x28);
    st(g, 0x2b, 0x3f);
    // call width: 3
    let part = g
        .call(slot_target(g, w(0x15), 719), 0, &[w(0x27), 8, 0])
        .eax;
    g.write::<u32>(w(0x45), part);
    crate::l5_objhost::part_init(g, img, call(0x4a));
    let o1 = crate::s4_misc::start_object_at(g, img, call(0x66), w(0x5a), kf(g, 0x52), 0, 0x10, 0);
    let c = g.read::<u32>(w(0x72));
    // call width: 4
    g.call(
        slot_target(g, w(0x6c), 262),
        0,
        &[c, w(0x87), w(0x82), w(0x7d)],
    );
    // call width: 3
    g.call(slot_target(g, w(0x96), 268), 0, &[c, 6, p]);
    let y = g.read::<f32>(p + 4);
    g.write::<f32>(w(0xba), y);
    st(g, 0xbe, 0xc8);
    let sc = mul(g.read::<f32>(o1 + 0x68), kf(g, 0xd1));
    g.write::<f32>(o1 + 0x68, sc);
    st(g, 0xe7, 0x141);
    g.write::<u16>(w(0x143), 0);
    st(g, 0x147, 0x151);
    let o2 = crate::s4_misc::start_object_at(g, img, call(0x152), w(0xe3), kf(g, 0xaf), 2, 0x10, 0);
    let c = g.read::<u32>(w(0x15e));
    // call width: 4
    g.call(
        slot_target(g, w(0x158), 262),
        0,
        &[c, w(0x173), w(0x16e), w(0x169)],
    );
    let ground = st0_f32(g.call(slot_target(g, w(0x17d), 12), 0, &[c]).st0);
    g.write::<f32>(w(0x189), ground);
    st(g, 0x18d, 0x197);
    let sc = mul(g.read::<f32>(o2 + 0x68), kf(g, 0x1a3));
    g.write::<f32>(o2 + 0x68, sc);
    let c = g.read::<u32>(w(0x1c2));
    st(g, 0x1c6, 0x1d0);
    // call width: 4
    g.call(slot_target(g, w(0x1b7), 262), 0, &[c, q, q + 4, q + 8]);
    let ground = st0_f32(g.call(slot_target(g, w(0x1da), 12), 0, &[c]).st0);
    g.write::<f32>(q + 4, ground);
    g.write::<u32>(q + 0xc, 0x3f80_0000);
    let mut i = 0u32;
    let mut count = g.read::<u32>(w(0x1f7));
    while i.cast_signed() < count.cast_signed() {
        let rec = w(0x202).wrapping_add(0x160 * i);
        let actor = actor_0174(g, img, f, i, (0x20d, 0x214, 0x21f));
        // call width: 4
        g.call(
            slot_target(g, w(0x228), 262),
            0,
            &[actor, rec - 4, rec, rec + 4],
        );
        g.write::<u32>(a + 4, actor);
        let ground = st0_f32(g.call(slot_target(g, w(0x23f), 12), 0, &[actor]).st0);
        g.write::<f32>(rec, ground);
        g.write::<u32>(rec + 8, 0x3f80_0000);
        for k in 0..4 {
            let v = g.read::<u32>(q + 4 * k);
            g.write::<u32>(rec + 0xc + 4 * k, v);
        }
        // call width: 3
        g.call(slot_target(g, w(0x280), 234), 0, &[d, q, rec - 4]);
        let (dx, dz) = (g.read::<f32>(d), g.read::<f32>(d + 8));
        let sq = add(mul(dz, dz), mul(dx, dx));
        // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
        let dist = f64::from(sq).sqrt() as f32;
        g.write::<u32>(rec + 0x24, 0);
        g.write::<f32>(rec + 0x1c, dist);
        i += 1;
        count = g.read::<u32>(w(0x2d8));
    }
    let flag = g.read::<u32>(w(0x2e4));
    let n = g.read::<u32>(w(0x2ea));
    g.write::<u32>(w(0x2ef), n);
    st(g, 0x2f9, 0x30d);
    let slot = if flag == 0 { 215 } else { 413 };
    let v = g.call(slot_target(g, w(0x2f5), slot), 0, &[n]).eax;
    let flag = g.read::<u32>(w(0x324));
    g.write::<u32>(w(0x32f), v);
    st(g, 0x333, 0x347);
    g.write::<u32>(w(0x349), 1);
    if flag == 0 {
        g.write::<u16>(w(0x353), 0);
    } else {
        g.write::<u16>(w(0x35c), 1);
    }
    let count = g.read::<u16>(w(0x363));
    let node = if count == 0 {
        0
    } else {
        let node = g.read::<u32>(w(0x372));
        crate::r15::ring_take(g, img, f, node, count, (0x38f, 0x397, 0x3c8, 0x3d3));
        node
    };
    let head = g.read::<u32>(w(0x3d9));
    g.write::<u32>(node.wrapping_add(4), 0);
    if head == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x3fb), node);
    } else {
        let tail = g.read::<u32>(w(0x3e8));
        g.write::<u32>(node, tail);
        g.write::<u32>(tail.wrapping_add(4), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(0x40d), node);
    Ret::eax(0)
}

/// [`start_up_0174`] (1052 bytes, 1 overlay).
pub const START_UP_0174: Sig = Sig {
    len: 1052,
    hash: 0x80de_9974_7342_d6b5,
    calls: &[
        (0x4a, Callee::Sig(&crate::l5_objhost::PART_INIT)),
        (0x66, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x152, Callee::Sig(&crate::s4_misc::START_OBJECT_AT)),
        (0x2bb, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x414, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0174`'s load step (0x10001210, the 118-byte shape) running
/// [`start_up_0174`].
pub fn load_0174<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0174)
}

/// [`load_0174`].
pub const LOAD_0174: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0174))],
    imports: &[],
};
