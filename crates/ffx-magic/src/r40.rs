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
