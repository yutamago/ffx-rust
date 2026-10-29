//! The object host: the effect runtime 120 to 224 overlays share besides
//! the OPU host (ported by list 5 from `magic_0219`; addresses in the
//! comments are that DLL's). The overlay keeps its effect objects in a
//! linked list driven once per frame, each object a 0x128-byte block
//! with a model matrix, a part (`pppStartPart`) and a script of commands
//! run by [`obj_script`]:
//!
//! - pool A (`G_NODES` 0x10177180, 0x3ff nodes of 12 bytes, a free count
//!   word and a rover): the nodes of the object list (`G_LIST`
//!   0x10036fd0: head, tail, current, count) and of each object's own
//!   list at `+0x10`. A node is `{prev, next, payload}`; a free node has
//!   payload 0.
//! - pool B (`G_TASKS` 0x1017a190, 256 nodes of 0x1c bytes in a ring):
//!   the task list (`G_ACTIVE` 0x1017a17c head, `+0x178` tail, `+0x180`
//!   the walk's current node), each task `{prev, next, self, _, step,
//!   countdown, group}`, `step` called with the task by [`run_group`].
//!
//! Every port reads the DLL's own globals from its instructions' operands
//! (`w(offset)`); the internal functions take the registers LTCG gave
//! them as arguments. Calls into overlay code (callbacks the overlay
//! stored) go through [`call_code`], which runs the bound port on the
//! stack the original would use.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
pub const ONE: u32 = 0x3f80_0000;

/// A call of overlay code at `target` (`call reg` in the original),
/// cdecl with `args`, `esp` the callee's stack pointer at its return
/// address (the original's stack, which the port uses as the original
/// would). Runs the port bound there; code without a port (or game code)
/// goes to the host. Returns `eax`.
fn call_code<G: Game>(g: &mut G, img: &Image, target: u32, esp: u32, args: &[u32]) -> u32 {
    let port = crate::bind(img, target).and_then(crate::entry::<G>);
    let Some(f) = port else {
        return g.call(target, 0, args).eax;
    };
    for (i, a) in (0u32..).zip(args) {
        g.write::<u32>(esp + 4 + 4 * i, *a);
    }
    let r = f(
        g,
        img,
        &Frame {
            at: target,
            esp,
            ..Frame::default()
        },
    );
    match r.jump {
        Some(t) => g.call(t, 0, args).eax,
        None => r.eax,
    }
}

// ---- pool A and its lists ----

/// The payload of a list's current node (`magic_0219` 0x100018f0, `ecx`
/// = the list): 0 when there is none.
pub fn list_cur<G: Game>(g: &G, list: u32) -> u32 {
    let node = g.read::<u32>(list + 8);
    if node == 0 {
        0
    } else {
        g.read::<u32>(node + 8)
    }
}

/// `list_cur` (12 bytes, 224 overlays).
pub const LIST_CUR: Sig = Sig {
    len: 12,
    hash: 0x2ad9_8d92_fda2_f427,
    calls: &[],
    imports: &[],
};

/// Unlinks a list's tail node (0x100017f0, `ecx` = the list) and frees
/// it (payload 0, the pool's free count up): 1, or 0 for an empty list.
/// `f` is the function's address (the free count's operand).
pub fn list_pop_tail<G: Game>(g: &mut G, img: &Image, f: u32, list: u32) -> u32 {
    let node = g.read::<u32>(list + 4);
    if node == 0 {
        return 0;
    }
    let prev = g.read::<u32>(node);
    g.write::<u32>(list + 4, prev);
    if prev == 0 {
        g.write::<u32>(list, 0);
    } else {
        g.write::<u32>(prev + 4, 0);
    }
    let cur = g.read::<u32>(list + 8);
    if cur == node {
        let next = g.read::<u32>(cur + 4);
        g.write::<u32>(list + 8, next);
    }
    // +0x2b: inc word ds:G_FREE
    let free = img.word(f + 0x2e);
    let n = g.read::<u16>(free);
    g.write::<u16>(free, n.wrapping_add(1));
    g.write::<u32>(node + 8, 0);
    let count = g.read::<u32>(list + 0xc);
    g.write::<u32>(list + 0xc, count.wrapping_sub(1));
    1
}

/// `list_pop_tail` (66 bytes, 169 overlays; list 2's in the work list,
/// ported here because list 5's functions call it).
pub const LIST_POP_TAIL: Sig = Sig {
    len: 66,
    hash: 0xc40d_2564_fafd_732e,
    calls: &[],
    imports: &[],
};

/// Removes the node holding `item` from a list (0x10001840, `ecx` = the
/// list, `edx` = the item) and frees it: 1, or 0 when no node holds it.
/// The tail goes through [`list_pop_tail`] (a tail jump).
pub fn list_remove<G: Game>(g: &mut G, img: &Image, f: u32, list: u32, item: u32) -> u32 {
    let head = g.read::<u32>(list);
    let mut node = head;
    while node != 0 && g.read::<u32>(node + 8) != item {
        node = g.read::<u32>(node + 4);
    }
    if node == 0 {
        return 0;
    }
    let free = |off: u32| img.word(f + off);
    if node == head {
        let next = g.read::<u32>(head + 4);
        g.write::<u32>(list, next);
        if next == 0 {
            g.write::<u32>(list + 4, 0);
        } else {
            g.write::<u32>(next, 0);
        }
        let cur = g.read::<u32>(list + 8);
        if cur == head {
            let n = g.read::<u32>(cur + 4);
            g.write::<u32>(list + 8, n);
        }
        // +0x4a: inc word ds:G_FREE
        let n = g.read::<u16>(free(0x4d));
        g.write::<u16>(free(0x4d), n.wrapping_add(1));
        g.write::<u32>(head + 8, 0);
    } else if node == g.read::<u32>(list + 4) {
        // +0x6c: jmp list_pop_tail
        return list_pop_tail(g, img, img.rel32_target(f + 0x6d), list);
    } else {
        let cur = g.read::<u32>(list + 8);
        if cur == node {
            let n = g.read::<u32>(cur + 4);
            g.write::<u32>(list + 8, n);
        }
        let prev = g.read::<u32>(node);
        let next = g.read::<u32>(node + 4);
        // +0x83: inc word ds:G_FREE
        let n = g.read::<u16>(free(0x86));
        g.write::<u16>(free(0x86), n.wrapping_add(1));
        g.write::<u32>(prev + 4, next);
        let next = g.read::<u32>(node + 4);
        let prev = g.read::<u32>(node);
        g.write::<u32>(next, prev);
        g.write::<u32>(node + 8, 0);
    }
    let count = g.read::<u32>(list + 0xc);
    g.write::<u32>(list + 0xc, count.wrapping_sub(1));
    1
}

/// `list_remove` (166 bytes, 169 overlays; list 2's in the work list,
/// ported here because list 5's functions call it).
pub const LIST_REMOVE: Sig = Sig {
    len: 166,
    hash: 0x9003_94f3_5193_98d8,
    calls: &[(0x6d, Callee::Sig(&LIST_POP_TAIL))],
    imports: &[],
};

/// A free pool-A node (0x10001900, no arguments): from the rover on, the
/// first with payload 0 (stepping by 2 after the first, modulo 0x3ff),
/// or 0 when the free count is 0. The rover moves to the old rover + 1
/// (modulo 0x3ff) and the free count goes down.
pub fn pool_alloc<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let free = g.read::<u16>(w(0x04));
    if free == 0 {
        return 0;
    }
    let rover = u32::from(g.read::<u16>(w(0x16)));
    let mut i = rover;
    // +0x24: cmp [eax*4 + G_NODES + 8], 0; lea ecx, [eax*4 + G_NODES]
    while g.read::<u32>(w(0x27).wrapping_add(12 * i)) != 0 {
        i = (i + 2) % 0x3ff;
    }
    let node = w(0x2f).wrapping_add(12 * i);
    g.write::<u16>(w(0x62), free.wrapping_sub(1));
    g.write::<u16>(w(0x6c), ((rover + 1) % 0x3ff) as u16);
    node
}

/// `pool_alloc` (113 bytes, 224 overlays).
pub const POOL_ALLOC: Sig = Sig {
    len: 113,
    hash: 0x6303_6721_4ce2_ff18,
    calls: &[],
    imports: &[],
};

/// Appends a node holding `item` to a list (inlined in several
/// functions): `prev` = the old tail, `next` = 0, the count up.
fn list_append<G: Game>(g: &mut G, head: u32, tail: u32, count: u32, node: u32, item: u32) {
    let last = g.read::<u32>(tail);
    g.write::<u32>(node + 4, 0);
    if last == 0 {
        g.write::<u32>(node, 0);
        g.write::<u32>(head, node);
    } else {
        g.write::<u32>(node, last);
        let last = g.read::<u32>(tail);
        g.write::<u32>(last + 4, node);
    }
    g.write::<u32>(tail, node);
    g.write::<u32>(node + 8, item);
    let n = g.read::<u32>(count);
    g.write::<u32>(count, n.wrapping_add(1));
}

// ---- pool B: the tasks ----

/// Builds the task ring (0x10002150, no arguments): 256 nodes of 0x1c
/// bytes linked both ways, the first's `prev` the last; the free head is
/// the first, the free count 0x100, the task list empty.
pub fn tasks_init<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let base = w(0x01);
    g.write::<u32>(w(0x07), w(0x0b));
    let mut node = base;
    for _ in 0..0xff {
        let next = node + 0x1c;
        g.write::<u32>(node + 4, next);
        g.write::<u32>(next, node);
        node = next;
    }
    g.write::<u32>(node + 4, w(0x24));
    g.write::<u32>(w(0x2f), 0);
    g.write::<u32>(w(0x35), 0);
    g.write::<u32>(w(0x3b), w(0x3f));
    g.write::<u32>(w(0x45), 0);
    g.write::<u16>(w(0x4b), 0x100);
    0x100
}

/// `tasks_init` (80 bytes, 224 overlays).
pub const TASKS_INIT: Sig = Sig {
    len: 80,
    hash: 0x4fa1_00a0_c902_b924,
    calls: &[],
    imports: &[],
};

// Task nodes (0x10002200, no arguments) come from the free ring through
// `crate::m0032::pop_node` (the same function, list 1's; `+0x70`, the
// node's step, is 0x10001340 here: `task_step`).

/// Runs every task of `group` (0x100021a0, `cl` = the group; `esp` its
/// stack pointer at the return address): `[[task + 8] + 0x10](task)`,
/// the walk's current node kept in a global so a step may unlink its
/// task. Returns how many ran.
pub fn run_group<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, group: u8) -> u32 {
    let w = |off: u32| img.word(f + off);
    let mut node = g.read::<u32>(w(0x01));
    g.write::<u32>(w(0x0d), node);
    let mut ran = 0u32;
    while node != 0 {
        let mut next = node;
        if g.read::<u8>(node + 0x18) == group {
            let owner = g.read::<u32>(node + 8);
            let step = g.read::<u32>(owner + 0x10);
            // push ebx, esi, edi; push task; call
            call_code(g, img, step, esp - 20, &[node]);
            next = g.read::<u32>(w(0x26));
            ran += 1;
        }
        if node == next {
            next = g.read::<u32>(node + 4);
            g.write::<u32>(w(0x36), next);
        }
        node = next;
    }
    g.write::<u32>(w(0x41), 0);
    ran
}

/// `run_group` (87 bytes, 224 overlays).
pub const RUN_GROUP: Sig = Sig {
    len: 87,
    hash: 0xc457_3745_71db_0f50,
    calls: &[],
    imports: &[],
};

// ---- the entries ----
//
// (The table's empty entries, `xor eax, eax; ret` at 0x10001300, bind to
// `runtime::zero_ret`.)

/// Table `+0x0` (0x10001000, cdecl `(arg)`): keeps `arg`, sets the
/// object block up ([`obj_block_init`]), three registrations (slots 556,
/// 555, 331 of the DLL's blocks), 1.0 into three data words, slot 790's
/// float into `G_SCALE`, the task ring ([`tasks_init`]), every pool-A
/// node free, then slot 720 `(0x3400, 0x800)`, 718, 222 and 221 for the
/// DLL's effect data. Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x08), arg);
    obj_block_init(g, img, img.rel32_target(f + 0x0d), arg);
    let s556 = slot_target(g, w(0x12), 556);
    g.call(s556, 0, &[w(0x17)]);
    let s555 = slot_target(g, w(0x24), 555);
    g.call(s555, 0, &[w(0x29), w(0x17)]);
    let s331 = slot_target(g, w(0x36), 331);
    g.call(s331, 0, &[w(0x3b), w(0x29), w(0x17)]);
    let s790 = slot_target(g, w(0x48), 790);
    g.write::<u32>(w(0x57), ONE);
    g.write::<u32>(w(0x61), ONE);
    g.write::<u32>(w(0x6b), ONE);
    // fstp dword: ST0 rounded to f32 (an empty x87 stack stores the
    // indefinite NaN)
    let scale = g
        .call(s790, 0, &[])
        .st0
        .map_or(f32::from_bits(0xffc0_0000), |x| x as f32);
    g.write::<f32>(w(0x77), scale);
    tasks_init(g, img, img.rel32_target(f + 0x7c));
    // +0x80: mov eax, G_NODES + 8; 0x3ff payloads of 12-byte nodes
    let payloads = w(0x81);
    for i in 0..0x3ff {
        g.write::<u32>(payloads + 12 * i, 0);
    }
    g.write::<u16>(w(0xa3), 0x3ff);
    g.write::<u16>(w(0xab), 0);
    let s720 = slot_target(g, w(0xb0), 720);
    g.call(s720, 0, &[0x3400, 0x800]);
    let data = w(0xcc);
    let s718 = slot_target(g, w(0xc7), 718);
    g.call(s718, 0, &[data, 0x3400, 0x800]);
    let s222 = slot_target(g, w(0xd9), 222);
    g.call(s222, 0, &[w(0xed), 0, 0x3e8, 1, data, 0x3400, 0x800]);
    let s221 = slot_target(g, w(0xf4), 221);
    g.call(
        s221,
        0,
        &[
            w(0x108),
            0,
            0x3e8,
            1,
            w(0xed),
            0,
            0x3e8,
            1,
            data,
            0x3400,
            0x800,
        ],
    );
    g.write::<u32>(w(0x113), 0);
    Ret::eax(0)
}

/// `start` (287 bytes, 123 overlays).
pub const START: Sig = Sig {
    len: 287,
    hash: 0x3039_c951_782c_a68e,
    calls: &[
        (0x0d, Callee::Sig(&OBJ_BLOCK_INIT)),
        (0x7c, Callee::Sig(&TASKS_INIT)),
    ],
    imports: &[],
};

/// Table `+0x4` (0x10001120, no arguments): the three releases (slots
/// 353, 397, 332), slot 267 `(4)`, slot 933 `(1.0, 1.0, 1.0, 1.0)`, 1.0
/// into a data word, slot 785 `(G_SCALE)` twice, slot 543 `(0)` and slot
/// 250 `(0, 0)`. Returns 0.
pub fn end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (a, b, c) = (w(0x06), w(0x18), w(0x2a));
    let s353 = slot_target(g, w(0x01), 353);
    g.call(s353, 0, &[a]);
    let s397 = slot_target(g, w(0x13), 397);
    g.call(s397, 0, &[b, a]);
    let s332 = slot_target(g, w(0x25), 332);
    g.call(s332, 0, &[c, b, a]);
    let s267 = slot_target(g, w(0x37), 267);
    g.call(s267, 0, &[4, c, b, a]);
    let s933 = slot_target(g, w(0x46), 933);
    g.call(s933, 0, &[ONE, ONE, ONE, ONE]);
    let s785 = slot_target(g, w(0x72), 785);
    let scale = g.read::<u32>(w(0x7a));
    g.write::<u32>(w(0x89), ONE);
    g.call(s785, 0, &[scale]);
    let s785 = slot_target(g, w(0x99), 785);
    let scale = g.read::<u32>(w(0xa1));
    g.call(s785, 0, &[scale]);
    let s543 = slot_target(g, w(0xb3), 543);
    g.call(s543, 0, &[0, scale]);
    let s250 = slot_target(g, w(0xc2), 250);
    g.call(s250, 0, &[0, 0]);
    Ret::eax(0)
}

/// `end` (227 bytes, 186 overlays).
pub const END: Sig = Sig {
    len: 227,
    hash: 0x367c_a9a4_961b_bedc,
    calls: &[],
    imports: &[],
};

/// The draw callback the per-frame entry installs (0x10001290): clears
/// two floats, runs the group-1 tasks and hands the floats to slot 250.
/// Returns 0.
pub fn draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    g.write::<u32>(w(0x04), 0);
    g.write::<u32>(w(0x0e), 0);
    run_group(g, img, img.rel32_target(f + 0x17), fr.esp - 4, 1);
    let y = g.read::<u32>(w(0x1f));
    let s250 = slot_target(g, w(0x24), 250);
    let x = g.read::<u32>(w(0x3b));
    g.call(s250, 0, &[x, y]);
    Ret::eax(0)
}

/// `draw` (76 bytes, 219 overlays).
pub const DRAW: Sig = Sig {
    len: 76,
    hash: 0xedf8_aba4_5330_1eed,
    calls: &[(0x17, Callee::Sig(&RUN_GROUP))],
    imports: &[],
};

/// The update callback the per-frame entry installs (0x100012e0): runs
/// the group-0 then the group-2 tasks. Returns 0.
pub fn update<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    run_group(g, img, img.rel32_target(f + 0x03), fr.esp - 4, 0);
    run_group(g, img, img.rel32_target(f + 0x0a), fr.esp - 4, 2);
    Ret::eax(0)
}

/// `update` (17 bytes, 224 overlays).
pub const UPDATE: Sig = Sig {
    len: 17,
    hash: 0x6f06_1add_781c_c277,
    calls: &[
        (0x03, Callee::Sig(&RUN_GROUP)),
        (0x0a, Callee::Sig(&RUN_GROUP)),
    ],
    imports: &[],
};

/// A task's step after its objects are done (0x10001310, `(task)`):
/// counts `+0x14` down, then calls slot 214 `(G_ARG)` (its result
/// returned).
pub fn countdown<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let task = fr.arg(g, 0);
    let n = g.read::<u32>(task + 0x14);
    if n.cast_signed() > 0 {
        g.write::<u32>(task + 0x14, n - 1);
        return Ret::eax(n - 1);
    }
    let s214 = slot_target(g, img.word(f + 0x0e), 214);
    let arg = g.read::<u32>(img.word(f + 0x14));
    Ret::eax(g.call(s214, 0, &[arg]).eax)
}

/// `countdown` (43 bytes, 164 overlays).
pub const COUNTDOWN: Sig = Sig {
    len: 43,
    hash: 0x5eb1_fb18_1cbc_44a8,
    calls: &[],
    imports: &[],
};

/// A task's first step (0x10001340, `(task)`): runs the objects
/// ([`run_objects`]); when they are all done, the countdown `+0x14` = 3
/// and the step becomes [`countdown`] (the operand at `+0x1d`). Returns
/// `run_objects`' result, or the task's `self` when done.
pub fn task_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    // push ebp; call
    let r = run_objects(g, img, img.rel32_target(f + 0x04), fr.esp - 8);
    if r != 1 {
        return Ret::eax(r);
    }
    let task = fr.arg(g, 0);
    g.write::<u32>(task + 0x14, 3);
    let owner = g.read::<u32>(task + 8);
    g.write::<u32>(owner + 0x10, img.word(f + 0x1d));
    Ret::eax(owner)
}

/// `task_step` (35 bytes, 39 overlays; list 2's in the work list, ported
/// here because list 5's tasks start with it).
pub const TASK_STEP: Sig = Sig {
    len: 35,
    hash: 0xc482_3643_b1d4_a6f0,
    calls: &[(0x04, Callee::Sig(&RUN_OBJECTS))],
    imports: &[],
};

/// An object's finish callback (0x10002130, `(obj)`): slot 239
/// `(G_HANDLE, -5, 0)`, its result returned.
pub fn finish<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s239 = slot_target(g, img.word(f + 0x01), 239);
    let h = g.read::<u32>(img.word(f + 0x11));
    Ret::eax(g.call(s239, 0, &[h, 0xffff_fffb, 0]).eax)
}

/// `finish` (27 bytes, 82 overlays).
pub const FINISH: Sig = Sig {
    len: 27,
    hash: 0x784c_53e6_be76_d7be,
    calls: &[],
    imports: &[],
};

// ---- the object block and the objects ----

/// The object block's set-up (0x100014b0, `ecx` = `arg`): slot 168 `(arg)` into `G_BLOCK`, its id through
/// slot 215 (or 413 when `G_ALT` is set) into `G_BLOCK + 0x10`, slot 218
/// `(arg, G_BLOCK + 4)`, then [`obj_block_fill`]. Returns what that
/// returns.
pub fn obj_block_init<G: Game>(g: &mut G, img: &Image, f: u32, arg: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let table = w(0x01);
    g.write::<u32>(w(0x0a), 0);
    g.write::<u32>(w(0x14), arg);
    let s168 = slot_target(g, table, 168);
    let block = g.call(s168, 0, &[arg]).eax;
    let alt = g.read::<u32>(w(0x26)) != 0;
    g.write::<u32>(w(0x2c), block);
    let id = slot_target(g, w(0x32), if alt { 413 } else { 215 });
    let r = g.call(id, 0, &[block]).eax;
    g.write::<u32>(w(0x4c), r);
    let s218 = slot_target(g, w(0x51), 218);
    let bytes = g.call(s218, 0, &[arg, w(0x56)]).eax;
    obj_block_fill(g, img, img.rel32_target(f + 0x6c), arg, w(0x65), bytes)
}

/// `obj_block_init` (117 bytes, 202 overlays).
pub const OBJ_BLOCK_INIT: Sig = Sig {
    len: 117,
    hash: 0xc064_0aaf_6064_29bc,
    calls: &[(0x6c, Callee::Sig(&OBJ_BLOCK_FILL))],
    imports: &[],
};

/// Fills the block (0x100016f0, `ecx` = `arg`, `edx` = the block, the
/// stack word = a byte list): 32 ids 0xff and 32 words 0 first; then for
/// each of the block's `+4` entries the byte, its id (slot 215 or 413)
/// into the words, slot 291 `(arg, byte, e - 4, e, e + 4)` into `e + 8`
/// and slot 367 `(arg, byte, e + 0x10, 0, e + 0x14, 0, 0, 0)` into
/// `e + 0xc` (`e` = block + 0x108 + 0x1c * i); `+0x1e4` = the sum of the
/// `e + 8` words, `+8` = the id bytes. Returns them (0 for no entries).
pub fn obj_block_fill<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    arg: u32,
    block: u32,
    bytes: u32,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let ids = block + 0x14;
    for i in 0..0x20 {
        g.write::<u8>(ids + i, 0xff);
        g.write::<u32>(block + 0x44 + 4 * i, 0);
    }
    let mut sum = 0u32;
    let n = g.read::<u32>(block + 4);
    if n.cast_signed() <= 0 {
        g.write::<u32>(block + 0x1e4, 0);
        g.write::<u32>(block + 8, ids);
        return 0;
    }
    let mut e = block + 0x108;
    let mut i = 0u32;
    loop {
        let b = u32::from(g.read::<u8>(bytes + i));
        g.write::<u8>(ids + i, b as u8);
        let alt = g.read::<u32>(w(0x53)) != 0;
        let id = slot_target(g, w(0x4a), if alt { 413 } else { 215 });
        let r = g.call(id, 0, &[b]).eax;
        g.write::<u32>(block + 0x44 + 4 * b, r);
        let s291 = slot_target(g, w(0x7f), 291);
        let a = [arg, b, e - 4, e, e + 4];
        let r = g.call(s291, 0, &a).eax;
        g.write::<u32>(e + 8, r);
        let s367 = slot_target(g, w(0xa6), 367);
        let mut args = vec![arg, b, e + 0x10, 0, e + 0x14, 0, 0, 0];
        args.extend(&a[..4]);
        // call width: 12 (the words this call leaves on the stack at most)
        let r = g.call(s367, 0, &args).eax;
        i += 1;
        g.write::<u32>(e + 0xc, r);
        sum = sum.wrapping_add(g.read::<u32>(e + 8));
        e += 0x1c;
        if i.cast_signed() >= g.read::<u32>(block + 4).cast_signed() {
            break;
        }
    }
    g.write::<u32>(block + 0x1e4, sum);
    g.write::<u32>(block + 8, ids);
    ids
}

/// `obj_block_fill` (253 bytes, 202 overlays).
pub const OBJ_BLOCK_FILL: Sig = Sig {
    len: 253,
    hash: 0x1f82_44ba_4be7_7e71,
    calls: &[],
    imports: &[],
};

/// Runs an object's script (0x10001530, `ecx` = the object): its wait
/// `+0x2c` counts down; each item of its own list (`+0x10`) is offered
/// to slot 408 `(entry, obj, item's +0xa)` (the entry slot 719 gives for
/// `(obj +0x20, 6, item +4)`) and dropped when that returns 0. When the
/// wait is over, the commands (12 bytes each from `+0x24`, the index at
/// `+0x28`) run until one stops:
/// 0 slot 220 `(entry, +8, +0xa)`; 1 slot 393 `(entry, obj, +0xa)`;
/// 2 the command joins the object's list, then as 1; 3 wait `+4`;
/// 4 nothing; 5 the index stays and the object's flag 0x8000 goes on;
/// above 5 skipped. At the end, a flagged object with an empty list gets
/// 0x4000 too.
pub fn obj_script<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32) {
    let w = |off: u32| img.word(f + off);
    let entry = |g: &mut G, off: u32, item: u32| {
        let s719 = slot_target(g, w(off), 719);
        let a = [g.read::<u32>(obj + 0x20), 6, g.read::<u32>(item + 4)];
        (g.call(s719, 0, &a).eax, a)
    };
    let wait = g.read::<u32>(obj + 0x2c);
    if wait.cast_signed() > 0 {
        g.write::<u32>(obj + 0x2c, wait - 1);
    }
    let list = obj + 0x10;
    let head = g.read::<u32>(list);
    g.write::<u32>(list + 8, head);
    let mut item = list_cur(g, list);
    while item != 0 {
        let k = i32::from(g.read::<i16>(item + 0xa)).cast_unsigned();
        let (e, a) = entry(g, 0x38, item);
        let s408 = slot_target(g, w(0x4d), 408);
        let mut args = vec![e, obj, k];
        args.extend(a);
        // call width: 6 (the words this call leaves on the stack at most)
        if g.call(s408, 0, &args).eax == 0 {
            list_remove(g, img, img.rel32_target(f + 0x68), list, item);
        }
        if item == list_cur(g, list) {
            let node = g.read::<u32>(list + 8);
            if node != 0 {
                let next = g.read::<u32>(node + 4);
                g.write::<u32>(list + 8, next);
            }
        }
        item = list_cur(g, list);
    }
    if g.read::<u32>(obj + 0x2c) == 0 {
        loop {
            let i = g.read::<u32>(obj + 0x28);
            let cmd = g.read::<u32>(obj + 0x24).wrapping_add(12 * i);
            g.write::<u32>(obj + 0x28, i.wrapping_add(1));
            match g.read::<u32>(cmd) {
                0 => {
                    let b = i32::from(g.read::<i16>(cmd + 0xa)).cast_unsigned();
                    let a8 = i32::from(g.read::<i16>(cmd + 8)).cast_unsigned();
                    let (e, a) = entry(g, 0xd8, cmd);
                    let s220 = slot_target(g, w(0xe8), 220);
                    let mut args = vec![e, a8, b];
                    args.extend(a);
                    // call width: 6 (the words this call leaves on the stack at most)
                    g.call(s220, 0, &args);
                }
                op @ (1 | 2) => {
                    if op == 2 {
                        let node = pool_alloc(g, img, img.rel32_target(f + 0xfd));
                        if node != 0 {
                            list_append(g, list, list + 4, list + 0xc, node, cmd);
                        }
                    }
                    let k = i32::from(g.read::<i16>(cmd + 0xa)).cast_unsigned();
                    let (e, a) = entry(g, 0x137, cmd);
                    let s393 = slot_target(g, w(0x14c), 393);
                    let mut args = vec![e, obj, k];
                    args.extend(a);
                    // call width: 6 (the words this call leaves on the stack at most)
                    g.call(s393, 0, &args);
                }
                3 => {
                    let n = g.read::<u32>(cmd + 4);
                    g.write::<u32>(obj + 0x2c, n);
                    break;
                }
                5 => {
                    g.write::<u32>(obj + 0x28, i);
                    let fl = g.read::<u16>(obj + 0x30);
                    g.write::<u16>(obj + 0x30, fl | 0x8000);
                    break;
                }
                _ => {}
            }
        }
    }
    let fl = g.read::<u16>(obj + 0x30);
    if fl >> 15 == 1 && g.read::<u32>(list + 0xc) == 0 {
        g.write::<u16>(obj + 0x30, fl | 0x4000);
    }
}

/// `obj_script` (436 bytes, 192 overlays).
pub const OBJ_SCRIPT: Sig = Sig {
    len: 436,
    hash: 0x7885_411a_1c7e_d44d,
    calls: &[
        (0x25, Callee::Sig(&LIST_CUR)),
        (0x68, Callee::Sig(&LIST_REMOVE)),
        (0x6f, Callee::Sig(&LIST_CUR)),
        (0x85, Callee::Sig(&LIST_CUR)),
        (0xfd, Callee::Sig(&POOL_ALLOC)),
    ],
    imports: &[],
};

/// Reads 4 f32 words.
fn row<G: Game>(g: &G, at: u32) -> [f32; 4] {
    [0, 4, 8, 12].map(|o| g.read::<f32>(at + o))
}

/// Runs every object of the list once (0x10001c20, no arguments; `esp`
/// its stack pointer at the return address, its locals where the
/// original keeps them: the camera matrix at `ebp - 0x94`, the model
/// matrix at `ebp - 0x44`, a vector at `ebp - 0x54`, which slots get
/// pointers to). Per object with flag 0x8000 and not 1: a delay
/// `+0x114` counts down, else the callbacks `+0x104` and (when `+0x11c`
/// = `+0x118`) `+0x100`, the model matrix by the mode byte `+0x126` (1:
/// slot 554; 2: the rows `+0x80` and the camera's translation; else
/// slots 330, 328, 329 with the angles `+0xc8`, `+0xc0`, `+0xc4`),
/// scaled by `+0x60..+0x68` per row and moved by `+0xd0` plus slot 229's
/// vector, given to slots 552 and 230, the colour `+0xf0` into the
/// game's, slots 231 and 230, then the part drawn (slot 712) unless the
/// object is flagged 0x4000 or that returns 1, which removes it (slot
/// 713, out of the list). An object with a script (`+0x110`) gets the
/// translation and a step ([`obj_script`]); `+0x118` counts the frames.
/// Returns 1 when no object ran.
#[allow(clippy::too_many_lines)]
pub fn run_objects<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let ebp = esp - 4;
    let cam = ebp - 0x94;
    let model = ebp - 0x44;
    let vec = ebp - 0x54;
    let trans = ebp - 0x14;
    let list_head = g.read::<u32>(w(0x14));
    let ctx = g.read::<u32>(w(0x1a));
    g.write::<u32>(w(0x20), list_head);
    let p = g.read::<u32>(w(0x25));
    g.write::<u32>(p, w(0x2e));
    if ctx != 0 {
        let p = g.read::<u32>(w(0x37));
        g.write::<u32>(p, ctx);
    }
    let a18 = g.read::<u32>(w(0x44));
    let s723 = slot_target(g, w(0x3e), 723);
    let a38 = g.read::<u32>(w(0x50));
    g.call(s723, 0, &[a18]);
    let s724 = slot_target(g, w(0x57), 724);
    g.call(s724, 0, &[a38, a18]);
    let s235 = slot_target(g, w(0x6c), 235);
    g.call(s235, 0, &[cam, a38, a18]);
    let list = w(0x7c);
    let mut ran = 0u32;
    let mut obj = list_cur(g, list);
    // the callbacks' frame: ebp, locals, esi, edi, ebx, the argument
    let cb_esp = esp - 0xac;
    while obj != 0 {
        let flags = g.read::<u16>(obj + 0x124);
        if flags & 0x8000 != 0 {
            ran += 1;
            if flags & 1 == 0 {
                let delay = g.read::<u32>(obj + 0x114);
                if delay == 0 {
                    run_object(g, img, f, obj, cb_esp, [cam, model, vec, trans]);
                } else {
                    g.write::<u32>(obj + 0x114, delay - 1);
                }
            }
        }
        let list = w(0x4ba);
        if obj == list_cur(g, list) {
            let cur = g.read::<u32>(w(0x4c9));
            if cur != 0 {
                let cur = g.read::<u32>(w(0x4d1));
                let next = g.read::<u32>(cur + 4);
                g.write::<u32>(w(0x4d9), next);
            }
        }
        obj = list_cur(g, w(0x4de));
    }
    u32::from(ran == 0)
}

/// One object of [`run_objects`] (the body after the delay test).
#[allow(clippy::too_many_lines)]
fn run_object<G: Game>(g: &mut G, img: &Image, f: u32, obj: u32, cb_esp: u32, locals: [u32; 4]) {
    let w = |off: u32| img.word(f + off);
    let [cam, model, vec, trans] = locals;
    let before = g.read::<u32>(obj + 0x104);
    if before != 0 {
        call_code(g, img, before, cb_esp, &[obj]);
    }
    let each = g.read::<u32>(obj + 0x100);
    if each != 0 && g.read::<u32>(obj + 0x11c) == g.read::<u32>(obj + 0x118) {
        call_code(g, img, each, cb_esp, &[obj]);
    }
    match g.read::<u8>(obj + 0x126) {
        1 => {
            let s554 = slot_target(g, w(0x239), 554);
            g.call(s554, 0, &[model, cam, obj + 0xc0]);
        }
        2 => {
            for i in 0..12 {
                let v = g.read::<u32>(obj + 0x80 + 4 * i);
                g.write::<u32>(model + 4 * i, v);
            }
            for i in 0..4 {
                let v = g.read::<u32>(cam + 0x30 + 4 * i);
                g.write::<u32>(model + 0x30 + 4 * i, v);
            }
        }
        _ => {
            let c8 = g.read::<u32>(obj + 0xc8);
            let s330 = slot_target(g, w(0x121), 330);
            g.call(s330, 0, &[model, cam, c8]);
            let c0 = g.read::<u32>(obj + 0xc0);
            let s328 = slot_target(g, w(0x143), 328);
            g.call(s328, 0, &[model, model, c0]);
            let c4 = g.read::<u32>(obj + 0xc4);
            let s329 = slot_target(g, w(0x165), 329);
            g.call(s329, 0, &[model, model, c4]);
        }
    }
    let s229 = slot_target(g, w(0x258), 229);
    let a229 = [vec, model, obj + 0xe0];
    g.call(s229, 0, &a229);
    // the rows scaled by +0x60, +0x64, +0x68 (mulss)
    for (r, s) in [(0u32, 0x60u32), (0x10, 0x64), (0x20, 0x68)] {
        let k = g.read::<f32>(obj + s);
        for c in 0..3 {
            let a = model + r + 4 * c;
            let v = g.read::<f32>(a) * k;
            g.write::<f32>(a, v);
        }
    }
    let v = row(g, vec);
    for c in 0..3 {
        let x = g.read::<f32>(obj + 0xd0 + 4 * c) + v[c as usize];
        g.write::<f32>(trans + 4 * c, x);
    }
    let s552 = slot_target(g, w(0x297), 552);
    let mut args = vec![obj + 0x40, trans];
    args.extend(a229);
    // call width: 5 (the words this call leaves on the stack at most)
    g.call(s552, 0, &args);
    let a24 = g.read::<u32>(w(0x347));
    let s230 = slot_target(g, w(0x341), 230);
    let mut args2 = vec![a24, model];
    args2.extend(&args);
    // call width: 7 (the words this call leaves on the stack at most)
    g.call(s230, 0, &args2);
    let tex = g.read::<u32>(obj + 0x108);
    if tex != 0 {
        let p = g.read::<u32>(w(0x361));
        g.write::<u32>(p, tex);
    }
    let colour = g.read::<u32>(w(0x36f));
    let a24 = g.read::<u32>(w(0x375));
    for i in 0..4 {
        let v = g.read::<u32>(obj + 0xf0 + 4 * i);
        g.write::<u32>(colour + 4 * i, v);
    }
    let a38 = g.read::<u32>(w(0x383));
    let a34 = g.read::<u32>(w(0x392));
    let s231 = slot_target(g, w(0x3a3), 231);
    g.call(s231, 0, &[a34, a38, a24]);
    let a34 = g.read::<u32>(w(0x3b1));
    let s230 = slot_target(g, w(0x3b6), 230);
    let a1c = g.read::<u32>(w(0x3bc));
    g.call(s230, 0, &[a1c, a34, a34, a38, a24]);
    let flags = g.read::<u16>(obj + 0x124);
    let remove = if flags & 0x4000 == 0 {
        let p = g.read::<u32>(w(0x3da));
        g.write::<u32>(p, w(0x3e2));
        let lock = g.read::<u32>(w(0x3e7));
        g.write::<u32>(lock, 1);
        let s712 = slot_target(g, w(0x3f3), 712);
        let r = g.call(s712, 0, &[obj, 0x30]).eax;
        let lock = g.read::<u32>(w(0x401));
        g.write::<u32>(lock, 0);
        if r == 1 {
            true
        } else {
            let done = g.read::<u32>(obj + 0x10c);
            if done != 0 {
                let s160 = slot_target(g, w(0x420), 160);
                g.call(s160, 0, &[obj, done, obj]);
            }
            false
        }
    } else {
        g.write::<u16>(obj + 0x124, flags & 0xbfff);
        true
    };
    if remove {
        let p = g.read::<u32>(w(0x43f));
        g.write::<u32>(p, w(0x446));
        let s713 = slot_target(g, w(0x44b), 713);
        g.call(s713, 0, &[obj]);
        let fl = g.read::<u16>(obj + 0x124);
        g.write::<u16>(obj + 0x124, fl & 0x7fff);
        list_remove(g, img, img.rel32_target(f + 0x469), w(0x464), obj);
    }
    let script = g.read::<u32>(obj + 0x110);
    if script != 0 {
        for i in 0..4 {
            let v = g.read::<u32>(trans + 4 * i);
            g.write::<u32>(script + 4 * i, v);
        }
        let script = g.read::<u32>(obj + 0x110);
        obj_script(g, img, img.rel32_target(f + 0x4a5), script);
    }
    let n = g.read::<u32>(obj + 0x118);
    g.write::<u32>(obj + 0x118, n.wrapping_add(1));
}

/// `run_objects` (1,289 bytes, 180 overlays).
pub const RUN_OBJECTS: Sig = Sig {
    len: 1289,
    hash: 0xa150_96e4_8687_3077,
    calls: &[
        (0x081, Callee::Sig(&LIST_CUR)),
        (0x469, Callee::Sig(&LIST_REMOVE)),
        (0x4a5, Callee::Sig(&OBJ_SCRIPT)),
        (0x4bf, Callee::Sig(&LIST_CUR)),
        (0x4e3, Callee::Sig(&LIST_CUR)),
        // __security_check_cookie
        (0x501, Callee::Any),
    ],
    imports: &[],
};

/// The part's set-up (0x10001980, no arguments): `pppInitData(G_PART,
/// program, 0)` (slot 711, the game flag `*G_LOCK` set to 1 around it),
/// slot 710 `(env, G_PART, G_BUF, 0x40000)`, the object list emptied.
pub fn part_init<G: Game>(g: &mut G, img: &Image, f: u32) {
    let w = |off: u32| img.word(f + off);
    let lock = g.read::<u32>(w(0x01));
    let part = g.read::<u32>(w(0x07));
    let saved = g.read::<u32>(lock);
    let buf = g.read::<u32>(w(0x11));
    g.write::<u32>(lock, 1);
    let s711 = slot_target(g, w(0x1c), 711);
    let a = [part, w(0x29), 0];
    g.call(s711, 0, &a);
    let lock = g.read::<u32>(w(0x31));
    g.write::<u32>(lock, saved);
    let s710 = slot_target(g, w(0x3d), 710);
    let part = g.read::<u32>(w(0x44));
    let mut args = vec![w(0x4f), part, buf, 0x40000];
    args.extend(a);
    // call width: 7 (the words this call leaves on the stack at most)
    g.call(s710, 0, &args);
    for off in [0x5a, 0x65, 0x6f, 0x79] {
        g.write::<u32>(w(off), 0);
    }
}

/// `part_init` (131 bytes, 149 overlays).
pub const PART_INIT: Sig = Sig {
    len: 131,
    hash: 0x7620_bb3a_d369_6d55,
    calls: &[],
    imports: &[],
};
