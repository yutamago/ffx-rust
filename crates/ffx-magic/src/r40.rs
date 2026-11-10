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
/// byte at `+5`). `O` is `[ebp - 4]`, the upper half of the function's own
/// 8-byte cursor (`[ebp - 8]`), read before anything writes it: `owner`.
/// Returns the handler's `eax`, or the walk's.
///
/// Unreachable in the game: the function's only way in is the thunk
/// 0x1000a5d0, stored only at `+0x1c` of the class record `EgoTailSpr`
/// (0x100115ac, a class's vtable is the address of its name). None of the
/// DLL's 2442 relocated words points at that record or at the thunk's slot
/// 0x100115c8 (the one pointer into 0x10011580..0x100115e0 is `EgoTask`'s
/// 0x100115cc), and no export or RVA names them, so no object ever carries
/// that vtable. The undefined pointer is therefore never dereferenced; the
/// port's entry passes 0 for it (r42, 2026-11-10).
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
    // [ebp-4] is never written (an uninitialised local used as a pointer);
    // the thunk is unreachable (see walk_ring_junk_0150), 0 stands in for it
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
pub(crate) fn flat_angle<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    x: f32,
    z: f32,
    k: [u32; 7],
) -> f32 {
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

/// `magic_0183`'s set-up (0x10001090, 561 bytes; cdecl `(actor)`, a frame
/// word at `ebp - 4`): `*G_ACTOR` = the actor; slot 168 its character;
/// slot 218 `(*G_ACTOR, &n)` the target list, its first byte the target;
/// slot 215 of the character and of the target; slot 219 `(target, 0)`
/// and `(target, 1)`; slot 217 `(target, G_P)` its position; slot 216
/// `(target)` (ST0, kept in the argument word) times the double `K` into
/// `*G_H`; two halfwords 0, `*G_W` -1; slot 210 `(*G_T, G_Q)`; the colour
/// words; the part handles
/// ([`part_handles`](crate::s4_small::part_handles)); slot 711 `(NAME,
/// BUF, 0)` under the game lock, slot 710
/// `(PART, NAME, BUF2, 0x100000)`; two particle parts started (slot 941's
/// table words 0 and 1, [`start_part`](crate::opu_parts::start_part)) and
/// their halfwords 1, a word 0. Returns 0.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn setup_0183<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let actor = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    g.write::<u32>(w(0x9), actor);
    let chr = g.call(t(g, 0xf, 168), 0, &[actor]).eax;
    g.write::<u32>(w(0x1c), chr);
    let a = rd(g, 0x2b);
    // call width: 2
    let list = g.call(t(g, 0x25, 218), 0, &[a, ebp.wrapping_sub(4)]).eax;
    let target = u32::from(g.read::<u8>(list));
    let c = rd(g, 0x3c);
    g.write::<u32>(w(0x41), target);
    let x = g.call(t(g, 0x46, 215), 0, &[c]).eax;
    let tg = rd(g, 0x54);
    g.write::<u32>(w(0x59), x);
    let x = g.call(t(g, 0x5e, 215), 0, &[tg]).eax;
    g.write::<u32>(w(0x6b), x);
    let tg = rd(g, 0x78);
    // call width: 2
    let x = g.call(t(g, 0x70, 219), 0, &[tg, 0]).eax;
    g.write::<u32>(w(0x85), x);
    let tg = rd(g, 0x92);
    // call width: 2
    let x = g.call(t(g, 0x8a, 219), 0, &[tg, 1]).eax;
    g.write::<u32>(w(0x9f), x);
    let tg = rd(g, 0xaf);
    // call width: 2
    g.call(t(g, 0xa4, 217), 0, &[tg, w(0xa9)]);
    let tg = rd(g, 0xc2);
    let h = crate::ob_model::st0_f32(g.call(t(g, 0xbc, 216), 0, &[tg]).st0);
    // fstp [ebp + 8]: the argument word
    g.write::<f32>(fr.esp + 4, h);
    let h = (f64::from(h) * g.read::<f64>(w(0xdf))) as f32;
    g.write::<u16>(w(0xe5), 0);
    g.write::<u16>(w(0xeb), 0);
    let q = rd(g, 0x105);
    g.write::<f32>(w(0x10d), h);
    abs_stores(g, img, f, 0x111, 0x11b);
    // call width: 2
    g.call(t(g, 0xf0, 210), 0, &[q, w(0xff)]);
    abs_stores(g, img, f, 0x11d, 0x14f);
    // push ebp; push ecx; push esi; fourteen words the slots left
    let ce = fr.esp.wrapping_sub(0x48);
    let ph = Frame {
        at: img.rel32_target(f + 0x150),
        esp: ce,
        ..Frame::default()
    };
    crate::s4_small::part_handles(g, img, &ph);
    let lock = rd(g, 0x155);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(t(g, 0x164, 711), 0, &[w(0x174), w(0x169), 0]);
    g.write::<u32>(rd(g, 0x17b), saved);
    // call width: 4
    g.call(
        t(g, 0x18a, 710),
        0,
        &[w(0x19f), w(0x19a), w(0x18f), 0x10_0000],
    );
    let start = img.rel32_target(f + 0x1d4);
    for (holder, val, op941, src, part, k) in [
        (0x1a6u32, 0x1acu32, 0x1b1u32, 0x1b7u32, 0x1cfu32, 0u32),
        (0x1d9, 0x1df, 0x1e4, 0x1ea, 0x203, 4),
    ] {
        g.write::<u32>(rd(g, holder), w(val));
        let s = rd(g, src);
        let table = g.call(t(g, op941, 941), 0, &[s]).eax;
        let word = g.read::<u32>(table.wrapping_add(k));
        crate::opu_parts::start_part(g, img, start, w(part), 0x1000, word, 1);
    }
    g.write::<u16>(w(0x216), 1);
    g.write::<u16>(w(0x21c), 1);
    g.write::<u32>(w(0x222), 0);
    Ret::eax(0)
}

/// [`setup_0183`] (561 bytes, 1 overlay).
pub const SETUP_0183: Sig = Sig {
    len: 561,
    hash: 0x2388_6ebc_1f58_2321,
    calls: &[
        (0x150, Callee::Sig(&crate::s4_small::PART_HANDLES)),
        (0x1d4, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x208, Callee::Sig(&crate::opu_parts::START_PART)),
    ],
    imports: &[],
};

/// `magic_0183`'s camera task (0x10001350, 1192 bytes; no arguments; an
/// `ebp` frame with a cookie: the matrix `M` at `ebp - 0x74`, the lift `V`
/// at `ebp - 0x34`, the point `W` at `ebp - 0x24`, the camera point `C`
/// at `ebp - 0x14`). The load steps (slot 225 busy, slot 226 asked once,
/// slot 227 loaded; each step's flag up); by the kind `*G_K`
/// (0x47/0x60/0x83) slot 228 `(*G_A, 0xb0..0xb2)`; the frame `n = K -
/// 0xbf` (0..0x46, else nothing more): at 0 and 0x1e slot 228 `(*G_T,
/// 0x80/0x81)`; slot 244 `(M, *G_ANG)`, 231, 229 `(V, M, (0, h, 0, 1))`
/// less `h`, slot 233 `(R, R, 0.8)` on three rows. Below 0x1e `C` moves
/// from `W` (slot 233 `(W, G, *G_S)` plus the start) toward the camera's
/// start by `n / K1`; below 0x32 `C` is the camera point
/// ([`camera_point`](crate::m0183::camera_point)); below 0x46 it moves
/// from there to the end by `((n - 0x32) / K2)^2`; at 0x46 slot 296
/// `(*G_T, 0)` ends it. `C.y` is held at the top `*G_TOP` (else slot 212
/// `(*G_C, 0)` once, when `*G_ID` is -1); slot 211 `(*G_C, P)` with `P` =
/// `C + V`; the turn halfword up by `*G_STEP + 0x80`. Returns 0.
#[allow(clippy::too_many_lines)] // one state machine
pub fn camera_task_0183<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let inc = |g: &mut G, o: u32| {
        let v = rd(g, o).wrapping_add(1);
        g.write::<u32>(w(o), v);
    };
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v, wp, c) = (
        ebp.wrapping_sub(0x74),
        ebp.wrapping_sub(0x34),
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
    );
    if rd(g, 0x12) == 0 {
        let x = rd(g, 0x20);
        if g.call(t(g, 0x1a, 225), 0, &[x]).eax != 0 {
            return Ret::eax(0);
        }
        inc(g, 0x39);
    }
    if rd(g, 0x3f) == 0 {
        let x = rd(g, 0x4f);
        g.write::<u32>(w(0x5b), 1);
        // call width: 2
        g.call(t(g, 0x47, 226), 0, &[x, 0]);
    }
    if rd(g, 0x6a) == 0 {
        let x = rd(g, 0x78);
        if g.call(t(g, 0x72, 227), 0, &[x]).eax.cast_signed() < 1 {
            return Ret::eax(0);
        }
        inc(g, 0x92);
    }
    let mut kind = rd(g, 0x97);
    let id = match kind {
        0x47 => Some(0xb0),
        0x60 => Some(0xb1),
        0x83 => Some(0xb2),
        _ => None,
    };
    if let Some(id) = id {
        let a = rd(g, 0xc6);
        // call width: 2
        g.call(t(g, 0xc0, 228), 0, &[a, id]);
        kind = rd(g, 0xd3);
    }
    let n = kind.wrapping_sub(0xbf);
    if n > 0x46 {
        return Ret::eax(0);
    }
    if n == 0 || n == 0x1e {
        let a = rd(g, 0x106);
        // call width: 2
        g.call(t(g, 0x100, 228), 0, &[a, if n == 0 { 0x80 } else { 0x81 }]);
    }
    let ang = u32::from(g.read::<u16>(w(0x118))) as i16;
    // call width: 2
    g.call(t(g, 0x122, 244), 0, &[m, i32::from(ang).cast_unsigned()]);
    // call width: 3
    g.call(t(g, 0x133, 231), 0, &[w(0x143), w(0x138), m]);
    let h = k(g, 0x14d);
    for (i, x) in (0u32..).zip([0, h.to_bits(), 0, 0x3f80_0000]) {
        g.write::<u32>(v + 4 * i, x);
    }
    // call width: 3
    g.call(t(g, 0x15c, 229), 0, &[v, w(0x156), v]);
    let y = sub(rf(g, v + 4), k(g, 0x18b));
    g.write::<f32>(v + 4, y);
    for (op, a, b) in [
        (0x190u32, 0x1afu32, 0x1aau32),
        (0x1b6, 0x1d0, 0x1cb),
        (0x1d7, 0x1f1, 0x1ec),
    ] {
        // call width: 3
        g.call(t(g, op, 233), 0, &[w(a), w(b), 0x3f4c_cccd]);
    }
    let cam = img.rel32_target(f + 0x31f);
    if n < 0x1e {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps of n < 0x1e
        let s = div(n as f32, k(g, 0x20f));
        let z = rd(g, 0x21f);
        // call width: 3
        g.call(t(g, 0x22f, 233), 0, &[wp, w(0x229), z]);
        let r = sub(k(g, 0x251), s);
        let wx = add(rf(g, wp), k(g, 0x249));
        let wy = add(add(k(g, 0x259), k(g, 0x261)), rf(g, wp + 4));
        let wz = add(rf(g, wp + 8), k(g, 0x276));
        g.write::<f32>(wp, wx);
        g.write::<f32>(wp + 4, wy);
        g.write::<f32>(wp + 8, wz);
        g.write::<f32>(c, add(mul(wx, s), mul(k(g, 0x269), r)));
        g.write::<f32>(c + 4, add(mul(wy, s), mul(k(g, 0x2a1), r)));
        g.write::<f32>(c + 8, add(mul(k(g, 0x2ba), r), mul(wz, s)));
        let ww = mul(rf(g, wp + 0xc), s);
        g.write::<f32>(c + 0xc, add(mul(k(g, 0x2cb), r), ww));
    } else if n < 0x32 {
        crate::m0183::camera_point(g, img, cam, c);
    } else if n < 0x46 {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps of n - 0x32 < 0x14
        let u = div((n - 0x32) as f32, k(g, 0x33f));
        let u = mul(u, u);
        crate::m0183::camera_point(g, img, img.rel32_target(f + 0x34d), wp);
        let r = sub(k(g, 0x35a), u);
        for (j, op) in (0u32..).zip([0x367u32, 0x37f, 0x39d, 0x3bb]) {
            let x = add(mul(rf(g, wp + 4 * j), r), mul(k(g, op), u));
            g.write::<f32>(c + 4 * j, x);
        }
    } else {
        let a = rd(g, 0x3e5);
        // call width: 2
        g.call(t(g, 0x3dd, 296), 0, &[a, 0]);
        return Ret::eax(0);
    }
    let top = k(g, 0x2ff);
    // comiss C.y, top; jbe: not above it (or NaN) keeps it
    if rf(g, c + 4) > top {
        g.write::<f32>(c + 4, top);
    } else if rd(g, 0x407) == u32::MAX {
        let a = rd(g, 0x417);
        // call width: 2
        let id = g.call(t(g, 0x40f, 212), 0, &[a, 0]).eax;
        g.write::<u32>(w(0x427), id);
    }
    let a = rd(g, 0x441);
    for (j, op) in (0u32..).zip([0x449u32, 0x45b, 0x46d]) {
        let x = add(rf(g, c + 4 * j), rf(g, v + 4 * j));
        g.write::<f32>(w(op), x);
    }
    // call width: 2
    g.call(t(g, 0x436, 211), 0, &[a, w(0x43b)]);
    let step = g.read::<u16>(w(0x47b)).wrapping_add(0x80);
    let turn = g.read::<u16>(w(0x48d)).wrapping_add(step);
    g.write::<u16>(w(0x48d), turn);
    g.write::<u16>(w(0x493), step);
    Ret::eax(0)
}

/// [`camera_task_0183`] (1192 bytes, 1 overlay).
pub const CAMERA_TASK_0183: Sig = Sig {
    len: 1192,
    hash: 0x7b6b_1bad_385e_2947,
    calls: &[
        (0x31f, Callee::Sig(&crate::m0183::CAMERA_POINT)),
        (0x34d, Callee::Sig(&crate::m0183::CAMERA_POINT)),
        // __security_check_cookie
        (0x3fd, Callee::Any),
        (0x4a0, Callee::Any),
    ],
    imports: &[],
};

/// One of `magic_0183`'s two part draws in [`draw_0183`]: slot 233
/// `(*G_M, B + 0x68, 0.4)` with the colour words 0.4, `(*G_M + 0x10, B +
/// 0x78, c1)`, `(*G_M + 0x20, B + 0x88, c2)`, the matrix's translation the
/// middle point `B + 0x98`; with `lift` slot 233 `(B + 0x18, G_UP, 5.0)`
/// moves it by `B + 0x18`; slots 231 `(*G_A, *G_B, M)` and 230, then
/// `pppRunPart(P, 0x30)` (slot 712) under the game lock; done clears the
/// part's halfword. The operands: `[colour stores x4, 233 x3 (imports,
/// matrix, colour), M, (lift: imports, up), M again, 231's two, 230's
/// imports and two, holder, value, lock, part, 712's imports, lock again,
/// the halfword]`.
#[allow(clippy::too_many_lines)] // one straight sequence
fn part_draw_0183<G: Game>(g: &mut G, img: &Image, f: u32, b: u32, ops: &PartDraw0183) {
    use crate::r25::abs_stores;
    use crate::sb_sse::add;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let m = rd(g, ops.m233[0].1);
    abs_stores(g, img, f, ops.colour.0, ops.colour.1);
    // call width: 3
    g.call(t(g, ops.m233[0].0, 233), 0, &[m, b + 0x68, 0x3ecc_cccd]);
    for (k, (imp, mat, col)) in [(1u32, ops.m233[1]), (2, ops.m233[2])] {
        let c = rd(g, col);
        let m = rd(g, mat).wrapping_add(0x10 * k);
        // call width: 3
        g.call(t(g, imp, 233), 0, &[m, b + 0x68 + 0x10 * k, c]);
    }
    let m = rd(g, ops.mat);
    for j in 0..4 {
        let x = g.read::<u32>(b + 0x98 + 4 * j);
        g.write::<u32>(m + 0x30 + 4 * j, x);
    }
    if let Some((imp, up)) = ops.lift {
        // call width: 3
        g.call(t(g, imp, 233), 0, &[b + 0x18, w(up), 0x40a0_0000]);
        let m = rd(g, ops.mat2);
        for j in 0..3 {
            let (mv, uv) = (
                g.read::<f32>(m + 0x30 + 4 * j),
                g.read::<f32>(b + 0x18 + 4 * j),
            );
            // the y sum's operands the other way round
            let x = if j == 1 { add(uv, mv) } else { add(mv, uv) };
            g.write::<f32>(m + 0x30 + 4 * j, x);
        }
    }
    let m = rd(g, ops.mat2);
    let (x, y) = (rd(g, ops.m231.1), rd(g, ops.m231.2));
    // call width: 3
    g.call(t(g, ops.m231.0, 231), 0, &[y, x, m]);
    let (x, y) = (rd(g, ops.m230.1), rd(g, ops.m230.2));
    // call width: 2
    g.call(t(g, ops.m230.0, 230), 0, &[y, x]);
    g.write::<u32>(rd(g, ops.holder.0), w(ops.holder.1));
    let lock = rd(g, ops.lock.0);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 2
    let done = g.call(t(g, ops.run.0, 712), 0, &[w(ops.run.1), 0x30]).eax;
    g.write::<u32>(rd(g, ops.lock.1), saved);
    if done != 0 {
        g.write::<u16>(w(ops.half), 0);
    }
}

/// The operands of a [`part_draw_0183`].
struct PartDraw0183 {
    colour: (u32, u32),
    m233: [(u32, u32, u32); 3],
    mat: u32,
    lift: Option<(u32, u32)>,
    mat2: u32,
    m231: (u32, u32, u32),
    m230: (u32, u32, u32),
    holder: (u32, u32),
    lock: (u32, u32),
    run: (u32, u32),
    half: u32,
}

/// `magic_0183`'s draw (0x10001800, 2172 bytes; no arguments; an 8-aligned
/// cookie frame, `B = ((E - 4) & !7) - 0x150` after `push esi; push
/// edi`). By the frame `K = *G_K`: at 0xa1 slot 220 `(H, 0x40, 0x100)` of
/// two handles; nothing more before 0xa1. Four bone points (slot 237 of
/// the target: 0x65, 0x135, 0xa8, 0xeb) and bone 0x2d into `G_P`; slot 236
/// `(T, 0x2d, B + 0xa8)`; the height and the middle point the halved
/// sums (times the double `K2`); at `K = 0xbf` the flat distance from the
/// start (`_libm_sse2_sqrt_precise`) into `*G_D`; below `K = 0xbf + 0x5a`
/// the two spans' cross into `G_X` (slots 234, 266, 232); the side and
/// up vectors (slots 232, 266), the basis into a matrix with the middle
/// point, slot 243 `(R, -0x4000)`, slot 231 `(B + 0x68, basis, R)`; three
/// part tables (slot 941), slots 723 and 724, the holder; the two parts
/// drawn while their halfwords are set ([`part_draw_0183`], from `K =
/// 0xbf` and `0xa1`); at `K = 0x183` slots 211 `(*G_C, 0)`, 213 `(*G_T,
/// 0)`, 214 `(*G_A)`; `K` up by one. Returns 0.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn draw_0183<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let b = ((fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x148)).wrapping_sub(8);
    let kind = rd(g, 0x1b);
    let esi = kind.wrapping_sub(0xbf);
    let edi = kind.wrapping_sub(0xa1);
    let mut kind = kind;
    if kind == 0xa1 {
        let h = rd(g, 0x48);
        // call width: 3
        g.call(t(g, 0x35, 220), 0, &[h, 0x40, 0x100]);
        let h = rd(g, 0x62);
        // call width: 3
        g.call(t(g, 0x4f, 220), 0, &[h, 0x40, 0x100]);
        kind = rd(g, 0x69);
    }
    if esi.cast_signed() >= 0 || edi.cast_signed() >= 0 {
        for (imp, src, bone, out) in [
            (0x85u32, 0x95u32, 0x65u32, b + 0xe8),
            (0xa4, 0xb7, 0x135, b + 0xf8),
            (0xc3, 0xd6, 0xa8, b + 0x38),
            (0xe5, 0xf8, 0xeb, b + 0x58),
        ] {
            let s = rd(g, src);
            // call width: 4
            g.call(t(g, imp, 237), 0, &[s, bone, 0, out]);
        }
        let s = rd(g, 0x117);
        // call width: 4
        g.call(t(g, 0xff, 237), 0, &[s, 0x2d, 0, w(0x10d)]);
        let half = g.read::<f64>(w(0x145));
        let scaled = |x: f32| (f64::from(x) * half) as f32;
        let hy = add(rf(g, b + 0x3c), rf(g, b + 0x5c));
        let s = rd(g, 0x13a);
        g.write::<f32>(w(0x151), scaled(hy));
        // call width: 3
        g.call(t(g, 0x132, 236), 0, &[s, 0x2d, b + 0xa8]);
        let half = g.read::<f64>(w(0x16d));
        let scaled = |x: f32| (f64::from(x) * half) as f32;
        let mx = scaled(add(rf(g, b + 0x38), rf(g, b + 0x58)));
        g.write::<f32>(b + 0x14, mx);
        let my = scaled(add(rf(g, b + 0x3c), rf(g, b + 0x5c)));
        g.write::<f32>(b + 0xc, my);
        let mz = scaled(add(rf(g, b + 0x40), rf(g, b + 0x60)));
        g.write::<f32>(b + 0x10, mz);
        let cross = if esi == 0 {
            let dz = sub(rf(g, w(0x1cf)), rf(g, w(0x1d7)));
            let dx = sub(rf(g, w(0x1c7)), rf(g, w(0x1df)));
            let sq = add(mul(dz, dz), mul(dx, dx));
            // `_libm_sse2_sqrt_precise` of the double: exact, as Rust's
            g.write::<f32>(w(0x1ff), f64::from(sq).sqrt() as f32);
            true
        } else {
            esi.cast_signed() < 0x5a
        };
        if cross {
            let (p1, p2, p3, p4) = (b + 0xe8, b + 0xf8, b + 0x38, b + 0x58);
            // call width: 3
            g.call(t(g, 0x21c, 234), 0, &[p1, p1, p2]);
            // call width: 3
            g.call(t(g, 0x234, 234), 0, &[p3, p3, p4]);
            // call width: 3
            g.call(t(g, 0x24e, 266), 0, &[w(0x253), p1, p3]);
            // call width: 2
            g.call(t(g, 0x260, 232), 0, &[w(0x270), w(0x265)]);
        }
        let side = b + 0x48;
        // call width: 2
        g.call(t(g, 0x284, 232), 0, &[side, w(0x27e)]);
        let fwd = b + 0x28;
        let x = g.read::<u32>(b + 0x50);
        let z = g.read::<u32>(side) ^ g.read::<u32>(w(0x2a9));
        g.write::<u32>(fwd, x);
        g.write::<u32>(fwd + 4, 0);
        g.write::<u32>(fwd + 8, z);
        // call width: 2
        g.call(t(g, 0x2b0, 232), 0, &[fwd, fwd]);
        let up = b + 0x18;
        // call width: 3
        g.call(t(g, 0x2da, 266), 0, &[up, side, fwd]);
        // call width: 2
        g.call(t(g, 0x2ed, 232), 0, &[up, up]);
        let basis = b + 0xa8;
        for (dst, src) in [(0u32, fwd), (0x10, up), (0x20, side)] {
            for j in 0..3 {
                let v = g.read::<u32>(src + 4 * j);
                g.write::<u32>(basis + dst + 4 * j, v);
            }
        }
        let rot = b + 0x108;
        for (o, v) in [
            (0xb4u32, 0u32),
            (0xc4, 0),
            (0xd4, 0),
            (0xd8, 0),
            (0xdc, 0),
            (0xe0, 0),
            (0xe4, 0x3f80_0000),
        ] {
            g.write::<u32>(b + o, v);
        }
        // call width: 2
        g.call(t(g, 0x385, 243), 0, &[rot, 0xffff_c000]);
        // call width: 3
        g.call(t(g, 0x400, 231), 0, &[b + 0x68, basis, rot]);
        let mid = b + 0x98;
        for (j, v) in [b + 0x14, b + 0xc, b + 0x10].into_iter().enumerate() {
            let x = g.read::<u32>(v);
            g.write::<u32>(mid + 4 * j as u32, x);
        }
        g.write::<u32>(mid + 0xc, 0x3f80_0000);
        for (imp, src, out) in [
            (0x44bu32, 0x414u32, 0x45eu32),
            (0x463, 0x459, 0x470),
            (0x475, 0x484, 0x491),
        ] {
            let s = rd(g, src);
            let x = g.call(t(g, imp, 941), 0, &[s]).eax;
            g.write::<u32>(w(out), x);
        }
        let a = rd(g, 0x48c);
        g.write::<u32>(rd(g, 0x496), w(0x49c));
        g.call(t(g, 0x4a1, 723), 0, &[a]);
        let a = rd(g, 0x4b4);
        g.call(t(g, 0x4ae, 724), 0, &[a]);
        g.write::<u32>(rd(g, 0x4c1), w(0x4ca));
        if g.read::<u16>(w(0x4d1)) != 0 && esi.cast_signed() >= 0 {
            part_draw_0183(
                g,
                img,
                f,
                b,
                &PartDraw0183 {
                    colour: (0x4fc, 0x524),
                    m233: [
                        (0x4f2, 0x4f8, 0),
                        (0x54b, 0x542, 0x530),
                        (0x579, 0x570, 0x55b),
                    ],
                    mat: 0x586,
                    lift: Some((0x5d7, 0x5d1)),
                    mat2: 0x5e4,
                    m231: (0x626, 0x5f6, 0x5fc),
                    m230: (0x639, 0x634, 0x63f),
                    holder: (0x64c, 0x654),
                    lock: (0x659, 0x679),
                    run: (0x66b, 0x65e),
                    half: 0x68a,
                },
            );
        }
        if g.read::<u16>(w(0x691)) != 0 && edi.cast_signed() >= 0 {
            part_draw_0183(
                g,
                img,
                f,
                b,
                &PartDraw0183 {
                    colour: (0x6bc, 0x6e4),
                    m233: [
                        (0x6b2, 0x6b8, 0),
                        (0x70b, 0x702, 0x6f0),
                        (0x739, 0x730, 0x71b),
                    ],
                    mat: 0x746,
                    lift: None,
                    mat2: 0x746,
                    m231: (0x78a, 0x780, 0x790),
                    m230: (0x7a3, 0x79e, 0x7a9),
                    holder: (0x7b6, 0x7be),
                    lock: (0x7c3, 0x7e3),
                    run: (0x7d5, 0x7c8),
                    half: 0x7f4,
                },
            );
        }
        kind = rd(g, 0x7f9);
    }
    if kind == 0x183 {
        let a = rd(g, 0x80d);
        // call width: 2
        g.call(t(g, 0x805, 211), 0, &[a, 0]);
        let a = rd(g, 0x822);
        // call width: 2
        g.call(t(g, 0x81a, 213), 0, &[a, 0]);
        let a = rd(g, 0x835);
        g.call(t(g, 0x82f, 214), 0, &[a]);
        let k = rd(g, 0x843).wrapping_add(1);
        g.write::<u32>(w(0x843), k);
    } else {
        g.write::<u32>(w(0x86b), kind.wrapping_add(1));
    }
    Ret::eax(0)
}

/// [`draw_0183`] (2172 bytes, 1 overlay).
pub const DRAW_0183: Sig = Sig {
    len: 2172,
    hash: 0x56d7_91fc_18e3_d3f6,
    calls: &[
        (0x1f3, Callee::Import("_libm_sse2_sqrt_precise")),
        // __security_check_cookie
        (0x858, Callee::Any),
        (0x874, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0185`'s spark burst (0x10001810, 493 bytes; internal, `ecx` = the
/// point `P`; an `ebp` frame with a cookie: the matrix `M` at `ebp - 0x64`,
/// the offset `V` at `ebp - 0x14`, the direction `U` at `ebp - 0x24`, a
/// random word at `ebp - 0x68`). `V` = `(1, 0, 0, 1)` turned by `-*G_A`
/// (slots 247, 229) and scaled in `x` and `z`; then four sparks, each in
/// the first free record ([`first_free`](crate::s4_small::first_free),
/// none ends it): a random turn (slot 241 times `K`, `cvttss2si`, negated
/// on odd sparks) of `(1, 0, 0, 1)` into `U`, a random distance `s` (slot
/// 241 `K2 + K3`), the record's point `(P.x + V.x + U.x s, P.y - K4, P.z +
/// V.z + U.z s, 1)`, its `+0x14` 0 and `+0x10` a random size `(r K5 + D1)
/// D2` (doubles). Returns 0.
pub fn spark_burst_0185<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, p: u32) -> u32 {
    use crate::sb_sse::{add, mul, sub};
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rf = |g: &G, a: u32| g.read::<f32>(a);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let ebp = esp.wrapping_sub(4);
    let (m, v, u, r) = (
        ebp.wrapping_sub(0x64),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x68),
    );
    let one = |g: &mut G, at: u32| {
        for (j, x) in (0u32..).zip([0x3f80_0000, 0, 0, 0x3f80_0000]) {
            g.write::<u32>(at + 4 * j, x);
        }
    };
    let a = g.read::<u32>(w(0x11)).wrapping_neg();
    // call width: 2
    g.call(t(g, 0x20, 247), 0, &[m, a]);
    one(g, v);
    // call width: 3
    g.call(t(g, 0x40, 229), 0, &[v, m, v]);
    let x = mul(rf(g, v), k(g, 0x71));
    g.write::<f32>(v, x);
    let z = mul(rf(g, v + 8), k(g, 0x86));
    g.write::<f32>(v + 8, z);
    let take = img.rel32_target(f + 0x92);
    for n in (0..4u32).rev() {
        let rec = crate::s4_small::first_free(
            g,
            img,
            &Frame {
                at: take,
                ..Frame::default()
            },
        )
        .eax;
        if rec == 0 {
            return 0;
        }
        g.call(t(g, 0xa2, 241), 0, &[r]);
        let turn = ffx_bridge::x87::cvttsd2si(f64::from(mul(rf(g, r), k(g, 0xbb))));
        let turn = if n & 1 != 0 {
            turn.wrapping_neg()
        } else {
            turn
        };
        let a = g.read::<u32>(w(0xce)).wrapping_add(turn).wrapping_neg();
        // call width: 2
        g.call(t(g, 0xdc, 247), 0, &[m, a]);
        one(g, u);
        // call width: 3
        g.call(t(g, 0xf5, 229), 0, &[u, m, u]);
        g.call(t(g, 0x122, 241), 0, &[r]);
        let s = add(mul(rf(g, r), k(g, 0x137)), k(g, 0x148));
        let x = add(add(rf(g, p), rf(g, v)), mul(rf(g, u), s));
        g.write::<f32>(rec, x);
        g.write::<f32>(rec + 4, sub(rf(g, p + 4), k(g, 0x16f)));
        let z = add(add(rf(g, p + 8), rf(g, v + 8)), mul(rf(g, u + 8), s));
        g.write::<u32>(rec + 0xc, 0x3f80_0000);
        g.write::<f32>(rec + 8, z);
        g.call(t(g, 0x156, 241), 0, &[r]);
        let x = mul(rf(g, r), k(g, 0x1ac));
        g.write::<u32>(rec + 0x14, 0);
        let size = (f64::from(x) + g.read::<f64>(w(0x1bf))) * g.read::<f64>(w(0x1c7));
        g.write::<f32>(rec + 0x10, size as f32);
    }
    0
}

/// [`spark_burst_0185`] (493 bytes, 1 overlay).
pub const SPARK_BURST_0185: Sig = Sig {
    len: 493,
    hash: 0xc60d_2d92_9e9e_cbc2,
    calls: &[
        (0x92, Callee::Sig(&crate::s4_small::FIRST_FREE)),
        // __security_check_cookie
        (0x1e5, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0185`'s spark task (0x10001a00, 278 bytes; no arguments; an `ebp`
/// frame with a cookie, the point `P` at `ebp - 0x14`): the load steps
/// (slot 225 busy, slot 226 asked once, slot 227 loaded); by the frame
/// `*G_K` (0x24/0x8f/0xa1) slot 228 `(*G_A, 0xc9..0xcb)`; from frame 0x28
/// to 0x4a the target's bone 0x11 (slot 237) and a
/// [`spark_burst_0185`] there. Returns 0.
pub fn spark_task_0185<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let inc = |g: &mut G, o: u32| {
        let v = rd(g, o).wrapping_add(1);
        g.write::<u32>(w(o), v);
    };
    if rd(g, 0x12) == 0 {
        let x = rd(g, 0x20);
        if g.call(t(g, 0x1a, 225), 0, &[x]).eax != 0 {
            return Ret::eax(0);
        }
        inc(g, 0x39);
    }
    if rd(g, 0x3f) == 0 {
        let x = rd(g, 0x4f);
        g.write::<u32>(w(0x5b), 1);
        // call width: 2
        g.call(t(g, 0x47, 226), 0, &[x, 0]);
    }
    if rd(g, 0x6a) == 0 {
        let x = rd(g, 0x78);
        if g.call(t(g, 0x72, 227), 0, &[x]).eax.cast_signed() < 1 {
            return Ret::eax(0);
        }
        inc(g, 0x8e);
    }
    let mut kind = rd(g, 0x93);
    let id = match kind {
        0x24 => Some(0xc9),
        0x8f => Some(0xca),
        0xa1 => Some(0xcb),
        _ => None,
    };
    if let Some(id) = id {
        let a = rd(g, 0xc4);
        // call width: 2
        g.call(t(g, 0xbe, 228), 0, &[a, id]);
        kind = rd(g, 0xd1);
    }
    if kind.wrapping_sub(0x28) > 0x22 {
        return Ret::eax(0);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let p = ebp.wrapping_sub(0x14);
    let a = rd(g, 0xf5);
    // call width: 4
    g.call(t(g, 0xe5, 237), 0, &[a, 0x11, 0, p]);
    // the burst's frame: below `ebp - 0x14` and the return address
    spark_burst_0185(
        g,
        img,
        img.rel32_target(f + 0x102),
        ebp.wrapping_sub(0x18),
        p,
    );
    Ret::eax(0)
}

/// [`spark_task_0185`] (278 bytes, 1 overlay).
pub const SPARK_TASK_0185: Sig = Sig {
    len: 278,
    hash: 0x2f37_d246_36b0_648c,
    calls: &[
        (0x102, Callee::Sig(&SPARK_BURST_0185)),
        // __security_check_cookie
        (0x10e, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0185`'s spark sprites (0x100015a0, 609 bytes; internal, no
/// arguments; a 16-aligned cookie frame, `B = ((E - 4) & !0xf) - 0xa0`
/// after `push esi; push edi`: the sprite record `S` at `B + 0x20`, the
/// matrix `M` at `B + 0x58`). For each used record `R` (0x18 bytes from
/// `G_R`; `u16 R + 0x16` its frame index, 0xffff free): `M` the identity
/// (slot 235) scaled by `R + 0x10`, its translation `R` through `*G_V`
/// (slot 229), slot 231 `(M, *G_W, M)`; `S`'s words (the matrix twice,
/// 0x80, the colour 0x70608080, `*G_Z`), the frame's sprite `F` from the
/// table at `T` (`T + 8 i` an offset from `D`); slot 772 `(R, 0)` gives a
/// handle `h` (`hi` its sign) and under the game lock slot 773 `(h, hi)`
/// says whether it is ready: then slot 875 `(h, hi, F, S)` and slot 775
/// `(h, hi)`, else slot 873 `(h, hi, F, S, 776(F's byte) | 777(M))`. The
/// record's timer `R + 0x14` grows by 0x200; past the frame's length it
/// steps to the next frame and wraps, and after the last frame the record
/// is free again. `eax` is the last value it held (its caller overwrites
/// it).
pub fn sprites_0185<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) {
    sprites_with(
        g,
        img,
        f,
        esp,
        &Sprites {
            map: |o| o,
            transform: true,
            cdq: true,
        },
    );
}

/// The layout of a [`sprites_0185`] copy: `map` takes `magic_0185`'s
/// operand offsets to the copy's; without `transform` the copy has no
/// slot 231 after placing the matrix; without `cdq` slot 772's `edx` is
/// the handle's high word instead of its sign.
pub(crate) struct Sprites {
    pub map: fn(u32) -> u32,
    pub transform: bool,
    pub cdq: bool,
}

/// [`sprites_0185`] on the layout `l`.
#[allow(clippy::too_many_lines)] // one loop
pub(crate) fn sprites_with<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, l: &Sprites) {
    let w = |o: u32| img.word(f + (l.map)(o));
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let b = ((esp.wrapping_sub(4) & !0xf).wrapping_sub(0x98)).wrapping_sub(8);
    let (s, m) = (b + 0x20, b + 0x58);
    let end = w(0x243);
    let mut idx_at = w(0x1d);
    while idx_at.cast_signed() < end.cast_signed() {
        g.write::<u32>(b + 0x10, idx_at);
        if g.read::<u16>(idx_at) != 0xffff {
            let rec = idx_at.wrapping_sub(0x16);
            g.call(t(g, 0x35, 235), 0, &[m]);
            let size = g.read::<u32>(idx_at.wrapping_sub(6));
            for o in [0x28u32, 0x14, 0] {
                g.write::<u32>(m + o, size);
            }
            let v = rd(g, 0x4f);
            // call width: 3
            g.call(t(g, 0x5c, 229), 0, &[b + 0x88, v, rec]);
            if l.transform {
                let x = rd(g, 0x87);
                // call width: 3
                g.call(t(g, 0x8d, 231), 0, &[m, x, m]);
            }
            g.write::<u32>(b + 0x3c, m);
            g.write::<u32>(b + 0x30, m);
            g.write::<u16>(b + 0x22, 0x80);
            let idx = i32::from(g.read::<i16>(idx_at)).cast_unsigned();
            g.write::<u32>(b + 0x18, idx);
            g.write::<u32>(b + 0x28, 0);
            g.write::<u8>(b + 0x38, 0);
            // mov dword [esp + 0x30], imm32: the colour
            g.write::<u32>(b + 0x24, w(0xc7));
            let off = i32::from(g.read::<i16>(w(0xcf).wrapping_add(idx.wrapping_mul(8))));
            let sprite = off.cast_unsigned().wrapping_add(w(0xd4));
            g.write::<u32>(b + 8, sprite);
            let z = g.read::<u32>(rd(g, 0xdd));
            g.write::<u32>(b + 0x34, z);
            // call width: 2
            let o = g.call(t(g, 0xeb, 772), 0, &[rec, 0]);
            let h = o.eax;
            // cdq, or the 64-bit answer's edx
            let hi = if !l.cdq {
                o.edx
            } else if h.cast_signed() < 0 {
                u32::MAX
            } else {
                0
            };
            let lock = rd(g, 0xfc);
            let saved = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            g.write::<u32>(b + 0xc, hi);
            g.write::<u32>(b + 0x1c, h);
            g.write::<u32>(b + 0x14, saved);
            // call width: 2
            let ready = g.call(t(g, 0x10f, 773), 0, &[h, hi]).eax;
            let lock = rd(g, 0x12b);
            g.write::<u32>(lock, saved);
            if ready != 0 {
                g.write::<u32>(lock, 1);
                // call width: 4
                g.call(t(g, 0x146, 875), 0, &[h, hi, sprite, s]);
                let lock = rd(g, 0x15e);
                g.write::<u32>(lock, 1);
                // call width: 2
                g.call(t(g, 0x16d, 775), 0, &[h, hi]);
                g.write::<u32>(rd(g, 0x17b), saved);
            } else {
                let sp = g.read::<u32>(b + 8);
                let at = g.read::<u32>(sp + 4).wrapping_add(sp).wrapping_add(4);
                let byte = u32::from(g.read::<u8>(at));
                let x = g.call(t(g, 0x198, 776), 0, &[byte]).eax;
                let mm = g.read::<u32>(b + 0x3c);
                let y = g.call(t(g, 0x1a6, 777), 0, &[mm]).eax;
                let lock = rd(g, 0x1bd);
                let saved = g.read::<u32>(lock);
                g.write::<u32>(lock, 1);
                // call width: 5
                g.call(t(g, 0x1d3, 873), 0, &[h, hi, sp, s, x | y]);
                g.write::<u32>(rd(g, 0x1ee), saved);
            }
            let timer = g.read::<u16>(idx_at.wrapping_sub(2)).wrapping_add(0x200);
            g.write::<u16>(idx_at.wrapping_sub(2), timer);
            let idx = g.read::<u32>(b + 0x18);
            let len = g.read::<u16>(w(0x216).wrapping_add(idx.wrapping_mul(8)));
            // cmp cx, ax; jl (signed halfwords)
            if timer.cast_signed() >= len.cast_signed() {
                let frame = g.read::<u16>(idx_at).wrapping_add(1);
                g.write::<u16>(idx_at, frame);
                g.write::<u16>(idx_at.wrapping_sub(2), timer.wrapping_sub(len));
                if frame.cast_signed() >= g.read::<i16>(w(0x22e)) {
                    g.write::<u16>(idx_at, 0xffff);
                }
            }
        }
        idx_at = idx_at.wrapping_add(0x18);
    }
    g.write::<u32>(b + 0x10, idx_at);
}

/// `magic_0185`'s sprite loop offset in `magic_0184`'s copy (no slot 231
/// block, no `cdq`).
fn map_sprites_0184(o: u32) -> u32 {
    match o {
        0..0x7d => o,
        0x7d..0x101 => o - 0x1c,
        _ => o - 0x1d,
    }
}

/// `magic_0184`'s spark sprites (0x10001030, 580 bytes; internal):
/// [`sprites_0185`] without slot 231 on the matrix, slot 772's handle a
/// 64-bit answer (`edx` its high word), the colour 0x70808080.
pub fn sprites_0184<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) {
    sprites_with(
        g,
        img,
        f,
        esp,
        &Sprites {
            map: map_sprites_0184,
            transform: false,
            cdq: false,
        },
    );
}

/// [`sprites_0184`] (580 bytes, 1 overlay).
pub const SPRITES_0184: Sig = Sig {
    len: 580,
    hash: 0xdd67_b82d_59a3_d126,
    calls: &[
        // __security_check_cookie
        (0x23c, Callee::Any),
    ],
    imports: &[],
};

/// [`sprites_0185`] (609 bytes, 1 overlay).
pub const SPRITES_0185: Sig = Sig {
    len: 609,
    hash: 0x6eff_0e53_312d_31b8,
    calls: &[
        // __security_check_cookie
        (0x259, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0185`'s set-up (0x10001090, 1136 bytes; cdecl `(actor)`; an
/// `ebp` frame with a cookie: the matrix `M` at `ebp - 0x64`, `U` at
/// `ebp - 0x24`, `V` at `ebp - 0x14`). The actor, its character (slot 168),
/// the target list (slot 218), the first target's character (slot 215)
/// and its size (slot 246, ST0) as a count `s K / K2`; per target its
/// record (`G_R + 0x88 i`: the selection byte, slot 215's character, slot
/// 210's point, copied 0x40 bytes on); slot 210 `(*G_T, P)`; the camera
/// words moved on and cleared, three rows scaled by 100 (slot 233), two
/// points placed (slots 247, 231, 229, 248 with `(+-50, 0, -50, 1)`), the
/// part handles ([`part_handles`](crate::s4_small::part_handles)), slot
/// 711 under the game lock, slot 710, three particle parts
/// ([`start_part`](crate::opu_parts::start_part) with slot 941's words
/// 0, 1 and 2), their halfwords 1, and every spark record free. Returns
/// 0.
#[allow(clippy::too_many_lines)] // one straight sequence
pub fn setup_0185<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let copy = |g: &mut G, from: u32, to: u32, n: u32| {
        for j in 0..n {
            let x = g.read::<u32>(from + 4 * j);
            g.write::<u32>(to + 4 * j, x);
        }
    };
    let actor = fr.arg(g, 0);
    let ebp = fr.esp.wrapping_sub(4);
    let (m, u, v) = (
        ebp.wrapping_sub(0x64),
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
    );
    g.write::<u32>(w(0x15), actor);
    let chr = g.call(t(g, 0x1b, 168), 0, &[actor]).eax;
    g.write::<u32>(w(0x28), chr);
    let a = rd(g, 0x38);
    // call width: 2
    let list = g.call(t(g, 0x2d, 218), 0, &[a, w(0x32)]).eax;
    g.write::<u32>(ebp.wrapping_sub(0x6c), list);
    let c = rd(g, 0x4c);
    let tc = g.call(t(g, 0x46, 215), 0, &[c]).eax;
    g.write::<u32>(w(0x5e), tc);
    let size = crate::ob_model::st0_f32(g.call(t(g, 0x64, 246), 0, &[tc]).st0);
    let n = div(mul(size, g.read::<f32>(w(0x7c))), g.read::<f32>(w(0x87)));
    g.write::<u32>(w(0x90), ffx_bridge::x87::cvttsd2si(f64::from(n)));
    let mut i = 0u32;
    let mut rec = w(0xa4);
    let mut count = rd(g, 0x9b);
    while i.cast_signed() < count.cast_signed() {
        let b = u32::from(g.read::<u8>(list.wrapping_add(i)));
        g.write::<u32>(rec.wrapping_sub(8), b);
        let x = g.call(t(g, 0xb1, 215), 0, &[b]).eax;
        g.write::<u32>(rec.wrapping_sub(4), x);
        // call width: 2
        g.call(t(g, 0xc3, 210), 0, &[x, rec]);
        copy(g, rec, rec + 0x40, 0x10);
        i += 1;
        rec = rec.wrapping_add(0x88);
        count = rd(g, 0xf0);
    }
    g.write::<u32>(ebp.wrapping_sub(0x68), i);
    let x = rd(g, 0x104);
    // call width: 2
    g.call(t(g, 0xf9, 210), 0, &[x, w(0xfe)]);
    copy(g, w(0x114), w(0x121), 2);
    copy(g, w(0x129), w(0x131), 2);
    for o in [0x13fu32, 0x14e] {
        g.write::<u32>(w(o), 0);
        g.write::<u32>(w(o) + 4, 0);
    }
    g.write::<u32>(w(0x154), 0x3f80_0000);
    g.write::<u32>(w(0x15e), 0x3f80_0000);
    for (imp, a, b) in [
        (0x119u32, 0x172u32, 0x16du32),
        (0x179, 0x193, 0x18e),
        (0x19a, 0x1b4, 0x1af),
    ] {
        // call width: 3
        g.call(t(g, imp, 233), 0, &[w(a), w(b), 0x42c8_0000]);
    }
    // call width: 2
    g.call(t(g, 0x1c1, 247), 0, &[m, 0]);
    // call width: 3
    g.call(t(g, 0x1d2, 231), 0, &[w(0x1e2), w(0x1d7), m]);
    copy(g, w(0x1ec), w(0x1f4), 2);
    copy(g, w(0x1fc), w(0x204), 2);
    let y = sub(g.read::<f32>(w(0x20c)), g.read::<f32>(w(0x214)));
    g.write::<f32>(w(0x227), y);
    for (s247, s231, b231, a231, vx, s229, b229, s248, b248, a248, one) in [
        (
            0x21fu32,
            0x238u32,
            0x23du32,
            0x248u32,
            0x4248_0000u32,
            0x25cu32,
            0x256u32,
            0x28cu32,
            0x291u32,
            0x29cu32,
            None,
        ),
        (
            0x2a9,
            0x2c4,
            0x2c9,
            0x2d4,
            0xc248_0000,
            0x2e8,
            0x2e2,
            0x315,
            0x31a,
            0x325,
            Some(0x2af),
        ),
    ] {
        if let Some(o) = one {
            g.write::<u32>(w(o), 0x3f80_0000);
        }
        // call width: 2
        g.call(t(g, s247, 247), 0, &[m, 0]);
        // call width: 3
        g.call(t(g, s231, 231), 0, &[w(a231), w(b231), m]);
        for (j, x) in (0u32..).zip([vx, 0, 0xc248_0000, 0x3f80_0000]) {
            g.write::<u32>(v + 4 * j, x);
        }
        // call width: 3
        g.call(t(g, s229, 229), 0, &[u, w(b229), v]);
        // call width: 3
        g.call(t(g, s248, 248), 0, &[w(a248), w(b248), u]);
    }
    crate::r25::abs_stores(g, img, f, 0x32b, 0x33f);
    let ph = Frame {
        at: img.rel32_target(f + 0x340),
        // push ebp; the frame; push esi; fourteen words the slots left
        esp: ebp.wrapping_sub(0xac),
        ..Frame::default()
    };
    crate::s4_small::part_handles(g, img, &ph);
    let lock = rd(g, 0x345);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(t(g, 0x354, 711), 0, &[w(0x364), w(0x359), 0]);
    g.write::<u32>(rd(g, 0x36b), saved);
    // call width: 4
    g.call(
        t(g, 0x375, 710),
        0,
        &[w(0x38f), w(0x38a), w(0x385), 0x10_0000],
    );
    let start = img.rel32_target(f + 0x3c4);
    for (holder, val, imp, src, part, k) in [
        (0x396u32, 0x39cu32, 0x3a1u32, 0x3a7u32, 0x3bfu32, 0u32),
        (0x3c9, 0x3cf, 0x3d4, 0x3da, 0x3f3, 4),
        (0x3fd, 0x403, 0x408, 0x40e, 0x427, 8),
    ] {
        g.write::<u32>(rd(g, holder), w(val));
        let s = rd(g, src);
        let table = g.call(t(g, imp, 941), 0, &[s]).eax;
        let word = g.read::<u32>(table.wrapping_add(k));
        crate::opu_parts::start_part(g, img, start, w(part), 0x1000, word, 1);
    }
    for o in [0x43au32, 0x440, 0x446] {
        g.write::<u16>(w(o), 1);
    }
    let mut at = w(0x44b);
    while at.cast_signed() < w(0x45a).cast_signed() {
        g.write::<u16>(at, 0xffff);
        at = at.wrapping_add(0x18);
    }
    Ret::eax(0)
}

/// [`setup_0185`] (1136 bytes, 1 overlay).
pub const SETUP_0185: Sig = Sig {
    len: 1136,
    hash: 0x28bc_b1c4_4705_fe5b,
    calls: &[
        (0x340, Callee::Sig(&crate::s4_small::PART_HANDLES)),
        (0x3c4, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x3f8, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x42c, Callee::Sig(&crate::opu_parts::START_PART)),
        // __security_check_cookie
        (0x468, Callee::Any),
    ],
    imports: &[],
};

/// The operands of one of [`frame_0185`]'s part draws: the colour words
/// (store runs), the colour value, slot 233's three calls (imports, the
/// matrix pointer, the row's source, the colour word), the matrix, the
/// translation's two qwords, slot 231's and 230's imports and words, the
/// holder and its value, the lock, `pppRunPart`'s imports and part, the
/// lock again, the halfword the done part clears.
struct PartDraw0185 {
    colour: [(u32, u32); 2],
    first: u32,
    m233: [(u32, u32, u32, u32); 3],
    mat: u32,
    tr: [(u32, u32); 2],
    m231: (u32, u32, u32),
    m230: (u32, u32, u32),
    holder: (u32, u32),
    lock: (u32, u32),
    run: (u32, u32),
    half: u32,
}

/// One part draw of [`frame_0185`]: the colour words, three rows of the
/// matrix `*G_M` (slot 233), its translation (two qwords), slots 231 and
/// 230, the holder, `pppRunPart(P, 0x30)` (slot 712) under the game lock;
/// done clears the part's halfword.
fn part_draw_0185<G: Game>(g: &mut G, img: &Image, f: u32, ops: &PartDraw0185) {
    use crate::r25::abs_stores;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    for (from, to) in ops.colour {
        abs_stores(g, img, f, from, to);
    }
    for (k, (imp, mat, src, col)) in (0u32..).zip(ops.m233) {
        let c = if k == 0 { ops.first } else { rd(g, col) };
        let m = rd(g, mat).wrapping_add(0x10 * k);
        // call width: 3
        g.call(t(g, imp, 233), 0, &[m, w(src), c]);
    }
    let m = rd(g, ops.mat);
    // movq: two words each
    for (src, dst) in ops.tr {
        for i in 0..2 {
            let x = g.read::<u32>(w(src) + 4 * i);
            g.write::<u32>(m + dst + 4 * i, x);
        }
    }
    let (x, y) = (rd(g, ops.m231.1), rd(g, ops.m231.2));
    // call width: 3
    g.call(t(g, ops.m231.0, 231), 0, &[y, x, m]);
    let (x, y) = (rd(g, ops.m230.1), rd(g, ops.m230.2));
    // call width: 2
    g.call(t(g, ops.m230.0, 230), 0, &[y, x]);
    g.write::<u32>(rd(g, ops.holder.0), w(ops.holder.1));
    let lock = rd(g, ops.lock.0);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 2
    let done = g.call(t(g, ops.run.0, 712), 0, &[w(ops.run.1), 0x30]).eax;
    g.write::<u32>(rd(g, ops.lock.1), saved);
    if done != 0 {
        g.write::<u16>(w(ops.half), 0);
    }
}

/// `magic_0185`'s frame (0x10001b20, 2048 bytes; no arguments; an `ebp`
/// frame, the shake's scale at `ebp - 4`). Nothing before the data is
/// loaded (`*G_LOADED`). At frame 0 slot 220 `(H, 0x40, 0x100)` of two
/// handles; slots 723, 724, the holder; from frame 0x28 to 0x6a slot 230
/// and the spark sprites ([`sprites_0185`]). From frame 0x65 (`n1 = K -
/// 0x65`) or 0x6f (`n2 = K - 0x6f`): three part tables (slot 941), the
/// holder, and three part draws ([`part_draw_0185`]: while their
/// halfwords are set, the first from `n1 >= 0`, the others from `n2 >=
/// 0`). Then by `n2`: 0 sends 0x40 to every target (slot 228); 10..0x13
/// slot 249 `(target, 0)`; 0x14 `(target, 0x100)`; from 0x14 slot 240 and
/// 213 for each target done; below 0x32 the camera shakes (slot 250 `(0,
/// sin(n2 / K) (1 - n2 / K)^2 K2)` from the game's sine table); 0x32 stops
/// it (slot 250 `(0, 0)`); 0x14 also slots 211 `(target, 0)` and 239 for
/// each; below 0x14 each target sways (its `+0x3c` less `(sin + 1) t K3`,
/// slot 211 `(target, R + 0x48)`). At frame 0xde slots 213 and 214. The
/// frame counts up. Returns 0.
#[allow(clippy::too_many_lines)] // one state machine
pub fn frame_0185<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let count = |g: &G, o: u32| rd(g, o).cast_signed();
    let ebp = fr.esp.wrapping_sub(4);
    if rd(g, 0x6) == 0 {
        return Ret::eax(0);
    }
    if rd(g, 0x13) == 0 {
        let h = rd(g, 0x2e);
        // call width: 3
        g.call(t(g, 0x1b, 220), 0, &[h, 0x40, 0x100]);
        let h = rd(g, 0x48);
        // call width: 3
        g.call(t(g, 0x35, 220), 0, &[h, 0x40, 0x100]);
    }
    let a = rd(g, 0x58);
    g.call(t(g, 0x52, 723), 0, &[a]);
    let a = rd(g, 0x6b);
    g.call(t(g, 0x65, 724), 0, &[a]);
    g.write::<u32>(rd(g, 0x78), w(0x81));
    if rd(g, 0x86).wrapping_sub(0x28) <= 0x42 {
        let (x, y) = (rd(g, 0x94), rd(g, 0x9f));
        // call width: 2
        g.call(t(g, 0x99, 230), 0, &[y, x]);
        // push ebp; push ecx: the return address below
        sprites_0185(g, img, img.rel32_target(f + 0xaf), ebp.wrapping_sub(8));
    }
    let k = rd(g, 0xb4);
    let n1 = k.wrapping_sub(0x65).cast_signed();
    let n2 = k.wrapping_sub(0x6f).cast_signed();
    if n1 >= 0 || n2 >= 0 {
        for (imp, src, out) in [
            (0xcdu32, 0xd3u32, 0xe6u32),
            (0xeb, 0xe1, 0xfe),
            (0x103, 0xf9, 0x110),
        ] {
            let s = rd(g, src);
            let x = g.call(t(g, imp, 941), 0, &[s]).eax;
            g.write::<u32>(w(out), x);
        }
        g.write::<u32>(rd(g, 0x115), w(0x11e));
        if g.read::<u16>(w(0x125)) != 0 && n1 >= 0 {
            part_draw_0185(
                g,
                img,
                f,
                &PartDraw0185 {
                    colour: [(0x13e, 0x148), (0x15a, 0x178)],
                    first: 0x3df5_c28f,
                    m233: [
                        (0x139, 0x156, 0x150, 0),
                        (0x19f, 0x189, 0x199, 0x184),
                        (0x1ca, 0x1b4, 0x1c4, 0x1af),
                    ],
                    mat: 0x1d7,
                    tr: [(0x1df, 0x30), (0x1ec, 0x38)],
                    m231: (0x1fd, 0x1f3, 0x203),
                    m230: (0x216, 0x211, 0x21c),
                    holder: (0x229, 0x231),
                    lock: (0x236, 0x256),
                    run: (0x248, 0x23b),
                    half: 0x267,
                },
            );
        }
        if g.read::<u16>(w(0x26e)) != 0 && n2 >= 0 {
            part_draw_0185(
                g,
                img,
                f,
                &PartDraw0185 {
                    colour: [(0x287, 0x291), (0x2a3, 0x2c1)],
                    first: 0x3e4c_cccd,
                    m233: [
                        (0x282, 0x29f, 0x299, 0),
                        (0x2e8, 0x2d2, 0x2e2, 0x2cd),
                        (0x313, 0x2fd, 0x30d, 0x2f8),
                    ],
                    mat: 0x320,
                    tr: [(0x328, 0x30), (0x335, 0x38)],
                    m231: (0x346, 0x33c, 0x34c),
                    m230: (0x35f, 0x35a, 0x365),
                    holder: (0x372, 0x37a),
                    lock: (0x37f, 0x39f),
                    run: (0x391, 0x384),
                    half: 0x3b0,
                },
            );
        }
        let mut go = true;
        if g.read::<u16>(w(0x3b7)) != 0 {
            if n2 < 0 {
                go = false;
            } else {
                part_draw_0185(
                    g,
                    img,
                    f,
                    &PartDraw0185 {
                        colour: [(0x3d0, 0x3da), (0x3ec, 0x40a)],
                        first: 0x3e4c_cccd,
                        m233: [
                            (0x3cb, 0x3e8, 0x3e2, 0),
                            (0x431, 0x41b, 0x42b, 0x416),
                            (0x45c, 0x446, 0x456, 0x441),
                        ],
                        mat: 0x469,
                        tr: [(0x471, 0x30), (0x47e, 0x38)],
                        m231: (0x48f, 0x485, 0x495),
                        m230: (0x4a8, 0x4a3, 0x4ae),
                        holder: (0x4bb, 0x4c3),
                        lock: (0x4c8, 0x4e8),
                        run: (0x4da, 0x4cd),
                        half: 0x4f9,
                    },
                );
            }
        }
        if go && n2 >= 0 {
            // the targets' records (0x88 bytes each) by their first word
            let each = |g: &mut G, base: u32, op: u32, f2: &mut dyn FnMut(&mut G, u32)| {
                let mut i = 0i32;
                while i < count(g, op) {
                    let r = w(base).wrapping_add(0x88 * i.cast_unsigned());
                    f2(g, r);
                    i += 1;
                }
            };
            let mut shake = n2 == 0;
            if n2 != 0 {
                if (10..0x14).contains(&n2) {
                    each(g, 0x55e, 0x574, &mut |g, r| {
                        let x = g.read::<u32>(r);
                        // call width: 2
                        g.call(t(g, 0x563, 249), 0, &[x, 0]);
                    });
                }
                if n2 >= 0x14 {
                    if n2 == 0x14 {
                        each(g, 0x598, 0x5b5, &mut |g, r| {
                            let x = g.read::<u32>(r);
                            // call width: 2
                            g.call(t(g, 0x5a1, 249), 0, &[x, 0x100]);
                        });
                    }
                    each(g, 0x5ce, 0x5fd, &mut |g, r| {
                        let x = g.read::<u32>(r);
                        if g.call(t(g, 0x5d3, 240), 0, &[x]).eax != 0 {
                            let x = g.read::<u32>(r);
                            // call width: 2
                            g.call(t(g, 0x5e9, 213), 0, &[x, 0]);
                        }
                    });
                }
                if n2 >= 0x32 {
                    if n2 == 0x32 {
                        // call width: 2
                        g.call(t(g, 0x74b, 250), 0, &[0, 0]);
                    }
                } else {
                    shake = true;
                }
            } else {
                each(g, 0x517, 0x53d, &mut |g, r| {
                    let x = g.read::<u32>(r);
                    // call width: 2
                    g.call(t(g, 0x521, 228), 0, &[x, 0x40]);
                });
            }
            if shake {
                let one = g.read::<f64>(w(0x620));
                #[allow(clippy::cast_precision_loss)] // cvtdq2ps of n2 < 0x32
                let q = div(n2 as f32, g.read::<f32>(w(0x628)));
                let s = (one - f64::from(q)) as f32;
                let table = rd(g, 0x62d);
                let idx = ((n2 << 13) >> 4).cast_unsigned() & 0xfff;
                let sine = g.read::<f32>(table.wrapping_add(4 * idx));
                let v = mul(mul(mul(sine, s), s), g.read::<f32>(w(0x669)));
                // call width: 2
                g.call(t(g, 0x64f, 250), 0, &[0, v.to_bits()]);
                if n2 == 0x14 {
                    each(g, 0x777, 0x7b4, &mut |g, r| {
                        let x = g.read::<u32>(r);
                        // call width: 2
                        g.call(t(g, 0x781, 211), 0, &[x, 0]);
                        let a = rd(g, 0x7a2);
                        // call width: 3
                        g.call(t(g, 0x792, 239), 0, &[a, 0xffff_fffb, 0]);
                    });
                } else if n2 < 0x14 {
                    let one = g.read::<f64>(w(0x698));
                    #[allow(clippy::cast_precision_loss)] // cvtdq2ps of n2 < 0x14
                    let q = div(n2 as f32, g.read::<f32>(w(0x6a0)));
                    let tt = (one - f64::from(q)) as f32;
                    g.write::<f32>(ebp.wrapping_sub(4), tt);
                    let mut i = 0i32;
                    let mut e = n2 << 13;
                    while i < count(g, 0x73c) {
                        let r = w(0x6c9).wrapping_add(0x88 * i.cast_unsigned());
                        let table = rd(g, 0x6d1);
                        let idx = (e >> 4).cast_unsigned() & 0xfff;
                        let sine = g.read::<f32>(table.wrapping_add(4 * idx));
                        let x = (f64::from(sine) + one) as f32;
                        let x = mul(mul(x, tt), g.read::<f32>(w(0x709)));
                        let y = sub(g.read::<f32>(r.wrapping_sub(0x40)), x);
                        g.write::<f32>(r, y);
                        let c = g.read::<u32>(r.wrapping_sub(0x78));
                        // call width: 2
                        g.call(t(g, 0x6f1, 211), 0, &[c, r.wrapping_sub(0x34)]);
                        i += 1;
                        e = e.wrapping_add(0x2000);
                    }
                }
            }
        }
    }
    if rd(g, 0x7bd) == 0xde {
        let a = rd(g, 0x7d2);
        // call width: 2
        g.call(t(g, 0x7ca, 213), 0, &[a, 0]);
        let a = rd(g, 0x7e5);
        g.call(t(g, 0x7df, 214), 0, &[a]);
    }
    let k = rd(g, 0x7f6).wrapping_add(1);
    g.write::<u32>(w(0x7f6), k);
    Ret::eax(0)
}

/// [`frame_0185`] (2048 bytes, 1 overlay).
pub const FRAME_0185: Sig = Sig {
    len: 2048,
    hash: 0x350c_8a20_4c9d_de0c,
    calls: &[(0xaf, Callee::Sig(&SPRITES_0185))],
    imports: &[],
};

/// `magic_0178`'s effect group task (0x100066e0, 696 bytes; a task
/// function, cdecl `(task)`, the record `R = task[1]`; an 8-aligned frame,
/// `A = ((E - 4) & !7) - 0x1c`, the set-up records at `A + 4`). The timer
/// `R + 0x7e4` grows by `K`; past `K2` the state `R + 0x538` is 2. State 0
/// waits for `R + 0x524`, then as 1: the static nodes `S1 = R + 0x640`
/// and `S2 = R + 0x66c` join the static list with their vtables, two
/// colour qwords and three words (`+0x8000` on the second) copied,
/// [`node_insert`](crate::nodes::node_insert) of `(R + 0x698, .., S1, S2,
/// ..)`, [`particle_node`](crate::sb_nodes::particle_node) of `R + 0x540`,
/// `R + 0x4fc` and `+ 0x504` scaled,
/// [`effect_node`](crate::sb_nodes::effect_node) and its vtable, a flag bit and a word, slot 239 `(*G_A,
/// -5, 0)`, state 2. State 2 (with `R + 0x520` set) counts the byte `R +
/// 0x2e3` down by 0x10; at 0 the node `R + 0x2bc` is unlinked
/// ([`unlink`](crate::m0042::unlink)) and so is the task node. Returns
/// slot 239's `eax`, the task node's old `+4`, or the state less 2.
#[allow(clippy::too_many_lines)] // one state machine
pub fn group_task_0178<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x1c);
    let timer = add(g.read::<f32>(r + 0x7e4), k(g, 0x23));
    let k2 = k(g, 0x2a);
    g.write::<f32>(r + 0x7e4, timer);
    // comiss timer, K2; jb: below (or NaN) keeps the state
    if !(timer.is_nan() || k2.is_nan() || timer < k2) {
        g.write::<u16>(r + 0x538, 2);
    }
    let state = i32::from(g.read::<i16>(r + 0x538));
    match state {
        2 => {
            if g.read::<u16>(r + 0x520) == 0 {
                return Ret::eax(0);
            }
            let b = g.read::<u8>(r + 0x2e3).wrapping_add(0xf0);
            g.write::<u8>(r + 0x2e3, b);
            if b != 0 {
                return Ret::eax(0);
            }
            // lea ecx, [esp + 0xc]: the frame word at `A`
            let cur = a;
            g.write::<u32>(cur, r + 0x2bc);
            crate::m0042::unlink(g, img, img.rel32_target(f + 0x7e), cur);
            let node = g.read::<u32>(task);
            let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
            g.write::<u32>(prev + 8, next);
            let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
            g.write::<u32>(next + 4, prev);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            Ret::eax(prev)
        }
        0 | 1 => {
            if state == 0 && g.read::<u16>(r + 0x524) == 0 {
                return Ret::eax(0);
            }
            let link = |g: &mut G, s: u32, ops: [u32; 6]| {
                // ops: the parent (two stores), the list's last (two reads),
                // the empty list's compare, its first's store
                g.write::<u16>(s + 0x1a, 0xff);
                g.write::<u32>(s + 0x10, s);
                g.write::<u32>(s + 0xc, s);
                g.write::<u32>(s + 0x14, w(ops[0]));
                g.write::<u32>(s + 4, w(ops[1]));
                let last = rd(g, ops[2]);
                g.write::<u32>(s + 8, last);
                let last = rd(g, ops[3]);
                if last == w(ops[4]) {
                    g.write::<u32>(w(ops[5]), s);
                } else {
                    g.write::<u32>(last.wrapping_add(4), s);
                }
            };
            let s1 = r + 0x640;
            link(g, s1, [0xc6, 0xcd, 0xd2, 0xda, 0xdf, 0xe7]);
            let n = g.read::<u16>(w(0xf3)).wrapping_add(1);
            g.write::<u16>(w(0xf3), n);
            g.write::<u32>(w(0xf9), s1);
            g.write::<u32>(s1, w(0xff));
            for (src, dst) in [(0x107u32, 0x65cu32), (0x117, 0x664)] {
                for j in 0..2 {
                    let x = g.read::<u32>(w(src) + 4 * j);
                    g.write::<u32>(r + dst + 4 * j, x);
                }
            }
            let s2 = r + 0x66c;
            g.write::<u32>(s2, w(0x12b));
            link(g, s2, [0x13e, 0x145, 0x14a, 0x152, 0x157, 0x15f]);
            let x = rd(g, 0x169);
            let n = g.read::<u16>(w(0x170)).wrapping_add(1);
            g.write::<u16>(w(0x170), n);
            g.write::<u32>(w(0x176), s2);
            g.write::<u32>(r + 0x688, x);
            let y = rd(g, 0x181).wrapping_add(0x8000);
            g.write::<u32>(r + 0x68c, y);
            let z = rd(g, 0x191);
            let rec = a + 4;
            g.write::<u32>(rec + 0xc, s1);
            g.write::<u32>(r + 0x690, z);
            let n1 = r + 0x698;
            for (i, v) in [
                (0u32, n1),
                (4, w(0x1b1)),
                (8, w(0x1b9)),
                (0x10, s2),
                (0x14, w(0x1c5)),
            ] {
                g.write::<u32>(rec + i, v);
            }
            let fr2 = Frame {
                at: img.rel32_target(f + 0x1ca),
                esp: a.wrapping_sub(0x10),
                ecx: rec,
                ..Frame::default()
            };
            crate::nodes::node_insert(g, img, &fr2);
            let x = rd(g, 0x1cf);
            let p = r + 0x540;
            g.write::<u32>(rec, p);
            g.write::<u32>(p, w(0x1df));
            for (i, v) in [
                (4u32, w(0x1eb)),
                (8, n1),
                (0xc, x),
                (0x10, w(0x1fb)),
                (0x14, 0),
            ] {
                g.write::<u32>(rec + i, v);
            }
            crate::sb_nodes::particle_node(g, img, img.rel32_target(f + 0x208), rec);
            let v = mul(g.read::<f32>(r + 0x4fc), k(g, 0x218));
            g.write::<f32>(r + 0x4fc, v);
            let v = mul(g.read::<f32>(r + 0x504), k(g, 0x23c));
            g.write::<f32>(r + 0x504, v);
            for (i, v) in [
                (0u32, w(0x224)),
                (4, w(0x244)),
                (8, w(0x24c)),
                (0xc, w(0x25c)),
                (0x10, w(0x264)),
                (0x14, 1),
            ] {
                g.write::<u32>(rec + i, v);
            }
            crate::sb_nodes::effect_node(g, img, img.rel32_target(f + 0x271), rec);
            let node = g.read::<u32>(rec);
            g.write::<u32>(node, w(0x27d));
            let fl = rd(g, 0x288);
            g.write::<u32>(w(0x288), fl | 1);
            let c = rd(g, 0x291);
            g.write::<u32>(w(0x297), 0);
            // call width: 3
            let eax = g
                .call(slot_target(g, w(0x282), 239), 0, &[c, 0xffff_fffb, 0])
                .eax;
            g.write::<u16>(r + 0x538, 2);
            Ret::eax(eax)
        }
        s => Ret::eax(s.wrapping_sub(2).cast_unsigned()),
    }
}

/// [`group_task_0178`] (696 bytes, 1 overlay).
pub const GROUP_TASK_0178: Sig = Sig {
    len: 696,
    hash: 0x0f2f_7526_b9ef_3ad3,
    calls: &[
        (0x7e, Callee::Sig(&crate::m0042::UNLINK)),
        (0x1ca, Callee::Sig(&crate::nodes::INSERT)),
        (0x208, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x271, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
    ],
    imports: &[],
};
