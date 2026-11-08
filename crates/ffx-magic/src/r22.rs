//! Round 22's own ports (own rows of `magic_0450` and up): the DLLs' own
//! code written out, and copies of ported functions bound by their own
//! `Sig`.

use crate::m0018::st0_f32;
use crate::runtime::slot_target;
use crate::sb_sse::{mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `magic_0645`'s summon object set-up (0x10007460, 789 bytes; internal:
/// `ecx` = the object, `edx` = the summon `S`; a cookie frame, `ebp = E -
/// 4`, the points `A` at `ebp - 0x24` and `B` at `ebp - 0x14`). The
/// object joins the DLL's ring and its node lists (`b = obj + 0x30`, `c =
/// obj + 0x5c`, vtable first); `A` = the sum of four bones of `S` (slot
/// 262 of `S[0x54]`, `S[0xec]`, `S[0x184]`, `S[0x21c]`, slot 248) and
/// `obj + 0x4c` = `A / 4` (slot 233), `w` 1.0; `+0x78`, `+0x80` 0, `+0x7c`
/// = `(K - y) K1 / K2` truncated, `y` slot 333 of `S[0x54]` (bit 0 of
/// `S[0x90]`) or slot 601 `(S[0x50], &y)`; the node insert
/// ([`node_insert`](crate::nodes::node_insert)) of `[obj + 0x88, LIST,
/// G, b, c, NAME]`, the bone walk
/// ([`bone_plain`](crate::sa_summon::bone_plain), the cursor `[obj +
/// 0x88, -, pass *G_PASS]`); four effect objects
/// ([`effect_setup_0645`](crate::r20::effect_setup_0645)) at `obj + 0x105c + 0x1088 i` over `S + 0x98
/// i`; `+0x28`, `+0x2c` the point's `x` and `z`, `+0x1058` = `S`, `+0x18`
/// 0. Returns the point's `z`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn summon_setup_0645<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::h6_setup::{ring_join, static_child};
    use crate::s4_summon::trunc;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let (obj, s) = (fr.ecx, fr.edx);
    let ebp = fr.esp.wrapping_sub(4);
    let (a, b4, y, group, cursor) = (
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x28),
        ebp.wrapping_sub(0x48),
        ebp.wrapping_sub(0x2c),
    );
    // push ebp; sub esp, 0x4c; three pushes: a call's return address
    let esp = fr.esp.wrapping_sub(0x60);
    ring_join(g, img, f, obj, [0x1f, 0x26, 0x2e, 0x38, 0x3d, 0x49, 0x50]);
    let b = obj + 0x30;
    static_child(
        g,
        img,
        f,
        b,
        [0x63, 0x6a, 0x6f, 0x77, 0x7f, 0x87, 0x93, 0xaa],
    );
    g.write::<u32>(b, w(0xb0));
    // junk read: [ebp-0x18] and [ebp-0x8] (the `w` of `A` and `B`, which
    // slot 262 leaves) never written; 0 (brief 2026-10-24): slot 248 adds
    // them, slot 233 scales them into `obj + 0x58`, which 1.0 overwrites
    g.write::<u32>(a + 0xc, 0);
    g.write::<u32>(b4 + 0xc, 0);
    let m = g.read::<u32>(s + 0x54);
    // call width: 4
    g.call(t(g, 0xa4, 262), 0, &[m, a, a + 4, a + 8]);
    for (o, t262, t248) in [
        (0xecu32, 0xc0u32, 0xe8u32),
        (0x184, 0xf5, 0x120),
        (0x21c, 0x12d, 0x158),
    ] {
        let m = g.read::<u32>(s + o);
        // call width: 4
        g.call(t(g, t262, 262), 0, &[m, b4, b4 + 4, b4 + 8]);
        // call width: 3
        g.call(t(g, t248, 248), 0, &[a, a, b4]);
    }
    let p = obj + 0x4c;
    // call width: 3
    g.call(t(g, 0x177, 233), 0, &[p, a, 0x3e80_0000]);
    g.write::<u32>(obj + 0x58, 0x3f80_0000);
    let c = obj + 0x5c;
    g.write::<u32>(c, w(0x192));
    static_child(
        g,
        img,
        f,
        c,
        [0x1a5, 0x1ac, 0x1b1, 0x1b9, 0x1be, 0x1c6, 0x1d2, 0x1d8],
    );
    g.write::<u32>(obj + 0x80, 0);
    g.write::<u32>(obj + 0x78, 0);
    let v = if g.read::<u8>(s + 0x90) & 1 != 0 {
        let r = st0_f32(g.call(t(g, 0x1f7, 333), 0, &[g.read::<u32>(s + 0x54)]).st0);
        sub(k(g, 0x20d), r)
    } else {
        // call width: 2
        g.call(t(g, 0x220, 601), 0, &[g.read::<u32>(s + 0x50), y]);
        sub(k(g, 0x233), g.read::<f32>(y))
    };
    let v = crate::sb_sse::div(mul(v, k(g, 0x243)), k(g, 0x254));
    let node = obj + 0x88;
    g.write::<u32>(obj + 0x7c, trunc(v));
    for (i, x) in (0u32..).zip([node, w(0x262), w(0x26f), b, c, w(0x27c)]) {
        g.write::<u32>(group + 4 * i, x);
    }
    crate::nodes::node_insert(
        g,
        img,
        &Frame {
            at: img.rel32_target(f + 0x281),
            esp,
            ecx: group,
            ..Frame::default()
        },
    );
    let pass = g.read::<u8>(w(0x286));
    g.write::<u32>(cursor, node);
    g.write::<u8>(cursor + 5, pass);
    let bf = Frame {
        at: img.rel32_target(f + 0x294),
        esp,
        ecx: cursor,
        ..*fr
    };
    crate::sa_summon::bone_plain(g, img, &bf);
    // push esi: the return address at E - 0x64
    let esp = fr.esp.wrapping_sub(0x64);
    for (i, call) in (0u32..).zip([0x2a5u32, 0x2ba, 0x2cf, 0x2e4]) {
        g.write::<u32>(esp + 4, node);
        let ef = Frame {
            at: img.rel32_target(f + call),
            esp,
            ecx: obj + 0x105c + 0x1088 * i,
            edx: s + 0x98 * i,
            ..*fr
        };
        crate::r20::effect_setup_0645(g, img, &ef);
    }
    let x = g.read::<u32>(obj + 0x4c);
    g.write::<u32>(obj + 0x28, x);
    let z = g.read::<u32>(obj + 0x54);
    g.write::<u32>(obj + 0x1058, s);
    g.write::<u32>(obj + 0x2c, z);
    g.write::<u32>(obj + 0x18, 0);
    Ret::eax(z)
}

/// [`summon_setup_0645`] (789 bytes, 1 overlay).
pub const SUMMON_SETUP_0645: Sig = Sig {
    len: 789,
    hash: 0xaaff_678e_0eb1_b3e2,
    calls: &[
        (0x281, Callee::Sig(&crate::nodes::INSERT)),
        (0x294, Callee::Sig(&crate::sa_summon::BONE)),
        (0x2a5, Callee::Sig(&crate::r20::EFFECT_SETUP_0645)),
        (0x2ba, Callee::Sig(&crate::r20::EFFECT_SETUP_0645)),
        (0x2cf, Callee::Sig(&crate::r20::EFFECT_SETUP_0645)),
        (0x2e4, Callee::Sig(&crate::r20::EFFECT_SETUP_0645)),
        // __security_check_cookie
        (0x30d, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0603`'s summon task (0x1000b4d0, 383 bytes; a task function
/// (its address pushed at 0x1000b6c6), cdecl `(task)`, the object `O =
/// task[4]`; a cookie frame, `ebp = E - 4`: the vector `V` at `ebp -
/// 0x18`, a cursor at `ebp - 0x1c`). While `*G_HOLD` is set nothing runs
/// (returns the task). Else `O + 0x2930..` += `O + 0x2a8c..` (three
/// words), `V` = slot 233 `(V, G_A, O[0x514])` with `w` 0, slot 248
/// `(G_B, G_C, V)`; by the state `O + 0x2a9c`:
/// - 2: the count `O + 0x2aac` steps by `K`; once it reaches `K'` the
///   node `O + 0x530` leaves its parent
///   ([`leave`](crate::sa_misc::leave)), its sound handles are registered
///   ([`handles_registered`](crate::s7_misc::handles_registered) over a
///   cursor holding the node: `ecx` as the leave left it, which never
///   writes it) and the link `O + 0x418` is unlinked (returns its
///   `+4`);
/// - 0: past `K''` of `O[0x514]` the timer `O + 0xe60` drops by `K'''`;
///   below 0 the halfword `+0x51e`, the timer and the count are cleared
///   and the state is 2;
/// - else returns the state less 2 (0 on the other ways).
pub fn task_0603<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 4);
    if g.read::<u32>(w(0x12)) != 0 {
        return Ret::eax(task);
    }
    for i in 0..3u32 {
        let d = g.read::<u32>(o + 0x2a8c + 4 * i);
        let at = o + 0x2930 + 4 * i;
        let v = g.read::<u32>(at).wrapping_add(d);
        g.write::<u32>(at, v);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let v = ebp.wrapping_sub(0x18);
    let x = g.read::<u32>(o + 0x514);
    // call width: 3
    g.call(t(g, 0x60, 233), 0, &[v, w(0x5a), x]);
    g.write::<u32>(v + 0xc, 0);
    // call width: 3
    g.call(t(g, 0x7a, 248), 0, &[w(0x91), w(0x8c), v]);
    let state = g.read::<u32>(o + 0x2a9c);
    match state {
        2 => {
            let n = add(g.read::<f32>(o + 0x2aac), k(g, 0xba));
            g.write::<f32>(o + 0x2aac, n);
            // comiss; jb: below or unordered waits
            if n.partial_cmp(&k(g, 0xc1))
                .is_none_or(std::cmp::Ordering::is_lt)
            {
                return Ret::eax(0);
            }
            let node = o + 0x530;
            crate::sa_misc::leave(g, node, 0);
            // the cursor's node is the `ecx` the leave leaves (it never
            // writes `ecx`: the node itself, brief 2026-11-08); its pass
            // byte `[ebp - 0x17]` is never written and never read
            let cursor = ebp.wrapping_sub(0x1c);
            g.write::<u32>(cursor, node);
            let hf = Frame {
                at: img.rel32_target(f + 0xe5),
                ecx: cursor,
                ..*fr
            };
            crate::s7_misc::handles_registered(g, img, &hf);
            let l = o + 0x418;
            let (prev, next) = (g.read::<u32>(l + 4), g.read::<u32>(l + 8));
            g.write::<u32>(prev + 8, next);
            let (next, prev) = (g.read::<u32>(l + 8), g.read::<u32>(l + 4));
            g.write::<u32>(next + 4, prev);
            g.write::<u32>(l + 8, l);
            g.write::<u32>(l + 4, l);
            Ret::eax(prev)
        }
        0 => {
            if g.read::<f32>(o + 0x514) > k(g, 0x127) {
                let e = sub(g.read::<f32>(o + 0xe60), k(g, 0x139));
                g.write::<f32>(o + 0xe60, e);
            }
            // comiss 0, e; jbe: only a timer below 0 ends
            if 0.0 > g.read::<f32>(o + 0xe60) {
                g.write::<u16>(o + 0x51e, 0);
                g.write::<u32>(o + 0xe60, 0);
                g.write::<u32>(o + 0x2aac, 0);
                g.write::<u32>(o + 0x2a9c, 2);
            }
            Ret::eax(0)
        }
        _ => Ret::eax(state.wrapping_sub(2)),
    }
}

/// [`task_0603`] (383 bytes, 1 overlay).
pub const TASK_0603: Sig = Sig {
    len: 383,
    hash: 0x8c4e_3966_d830_3d43,
    calls: &[
        (0xda, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0xe5, Callee::Sig(&crate::s7_misc::HANDLES_REGISTERED)),
        // __security_check_cookie
        (0x114, Callee::Any),
        (0x177, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0602`'s step (0x10006ad0, 490 bytes; a table entry, cdecl
/// `(node)`): [`step_0511`](crate::r20::step_0511) instruction for
/// instruction, whose end call stops the effect's sounds
/// ([`sounds_stop`](crate::m0602::sounds_stop)) where `magic_0511` ends
/// its models.
pub fn step_0602<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r20::step_with(g, img, fr, |g, img, mf| {
        crate::m0602::sounds_stop(g, img, mf);
    })
}

/// [`step_0602`] (490 bytes, 1 overlay).
pub const STEP_0602: Sig = Sig {
    len: 490,
    hash: 0xff39_82b1_fc92_95f7,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x38, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x44, Callee::Sig(&crate::sb_misc::SCRIPT_RUN)),
        (0x1a6, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x1b0, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x1ba, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x1c4, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x1cc, Callee::Sig(&crate::s8_misc::CAMERA_RIGS_STEP)),
        (0x1db, Callee::Sig(&crate::m0602::SOUNDS_STOP)),
        (0x1e6, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// A node of `magic_0645`'s summon task appended to a static list: its
/// own empty child list (`+0x1a` 0xff, `+0x10`/`+0xc` itself), the list
/// `+0x14` and `+0x4` (operands `ops[0]`, `ops[1]`), then
/// [`attach`](crate::l3_common::attach)'s last/first/count words
/// (operands `ops[2..8]`).
fn list_node_0645<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, ops: [u32; 8]) {
    use crate::l3_common::{ListAt, attach};
    let w = |o: u32| img.word(f + o);
    g.write::<u16>(node + 0x1a, 0xff);
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

/// `magic_0645`'s summon task (0x10007c60, 1514 bytes; a task function
/// (its address pushed at 0x100083c6), cdecl `(task)`, the object `O =
/// task[4]`; an 8-aligned cookie frame: `B = ((E - 4) & !7) - 0xc8`, the
/// cursors at `B + 0x14`, `+0x50`, `+0x40`, `+0x60`, `+0x28`, a group at
/// `B + 0x28`, the point `V` at `B + 0x70`, the matrix `M` at `B +
/// 0x80`). By the state `O + 0x18` (each change runs the new state at
/// once) with the count `O + 0x1c`:
/// - 0: five model nodes `O + 0x5880 + 0x98 i` (the DLL's vtable, the
///   list `G_L`, objects 0x14..0x18:
///   [`model_node_init`](crate::sa_summon::model_node_init)); slot 8 of
///   the handle `O[0x58d4]` with slot 7's flags less 0x30 (`O + 0x58ec`
///   1); the value task `O + 0x5e0`
///   ([`value_start`](crate::m0645::value_start)), the mover `O + 0x20`
///   ([`mover`](crate::m0645::mover)) and the summon set-up `O + 0x604`
///   ([`summon_setup_0645`]) over the first two models; the colour node
///   `O + 0x6520` (white 255, alpha 128; slot 234; flag bit 0, 1, 0.025,
///   0); the node `N = O + 0x5c54` (its vtable after it joins `G_L`) at
///   `(0, 0, -1000)` of the handle's matrix (slots 210, 229); the node
///   `O + 0x657c` with slot 719's part `(PART, 6, 0)`, `N` and its
///   words; count 0x1e, state 0x14;
/// - 0x14: when the count runs out, count 0x82 and state 10;
/// - 10: when the count runs out, the node `O + 0x5c80` appended to
///   `G_L` ([`node_append`](crate::sb_misc::node_append)), its colour
///   (1, 1, 1, 0.3), the node `O + 0x6090` inserted
///   ([`node_insert`](crate::nodes::node_insert)) with a group holding
///   the `ecx` and `edx` the append leaves (it writes neither: the node
///   and `G_L`, brief 2026-11-08), the particle node `O + 0x62e0`
///   ([`particle_node_with`](crate::s3_nodes::particle_node_with));
///   state 11, the count 0xdb, `O + 0x48`, `O + 0x5f8` 20000 (returns the
///   particle node's `eax`);
/// - 11: at count 0 slots 639 `(0x5a)` and 8 `(handle, 0)`; state 12,
///   count 0x3b (returns slot 8's `eax`);
/// - 12: at count 0, unless bit 2 of `O + 0x63c8`, the particle node's
///   cursor freed ([`free_then_unlink`](crate::nodes::free_then_unlink))
///   and, unless bit 2 of its `+0xe8`, `*G_P` = its `+0x50` and slot 713
///   `(+0x60)`; state 20000;
/// - 20000: the first model faded in
///   ([`chr_fade`](crate::l3_common::chr_fade) `(1.0, 0)`), its node
///   leaves ([`leave`](crate::sa_misc::leave)), its model set up
///   ([`model_setup`](crate::sa_summon::model_setup)), the task node
///   unlinked (returns its `+4`).
///
/// Every way that waits counts down once and returns what `eax` holds
/// there (the state, or the state less 10 or 11).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0645<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 4);
    let b = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0xc8);
    // a call without pushes: its return address at `B - 4`
    let at = |c: u32, ecx: u32, edx: u32| Frame {
        at: call(c),
        esp: b.wrapping_sub(4),
        ecx,
        edx,
        ..*fr
    };
    let count_down = |g: &mut G, eax: u32| {
        let n = g.read::<u32>(o + 0x1c);
        g.write::<u32>(o + 0x1c, n.wrapping_sub(1));
        Ret::eax(eax)
    };
    loop {
        let state = g.read::<u32>(o + 0x18);
        match state {
            0 => {
                let first = o + 0x5880;
                for (i, (cur, list, vt, c)) in (0u32..).zip([
                    (0x14u32, 0x5du32, 0x6fu32, 0x74u32),
                    (0x50, 0x8a, 0x9d, 0xa2),
                    (0x40, 0xb8, 0xcb, 0xd0),
                    (0x60, 0xe6, 0xf9, 0xfe),
                    (0x28, 0x114, 0x127, 0x12c),
                ]) {
                    let node = first + 0x98 * i;
                    let args = b + cur;
                    for (j, x) in (0u32..).zip([node, w(list), 0x14 + i]) {
                        g.write::<u32>(args + 4 * j, x);
                    }
                    // the first cursor's flags byte is `al` (the state, 0)
                    g.write::<u8>(args + 0xc, 0);
                    g.write::<u32>(node, w(vt));
                    crate::sa_summon::model_node_init(g, img, call(c), args);
                }
                let h = g.read::<u32>(o + 0x58d4);
                g.write::<u32>(o + 0x58ec, 1);
                let fl = g.call(t(g, 0x131, 7), 0, &[h]).eax & !0x30;
                // call width: 2
                g.call(t(g, 0x152, 8), 0, &[g.read::<u32>(o + 0x58d4), fl]);
                crate::m0645::value_start(g, img, &at(0x16d, o + 0x5e0, first));
                crate::m0645::mover(g, img, &at(0x177, o + 0x20, first));
                summon_setup_0645(g, img, &at(0x184, o + 0x604, first + 0x98));
                // the colour node
                let c = o + 0x6520;
                g.write::<u32>(c, w(0x190));
                list_node_0645(
                    g,
                    img,
                    f,
                    c,
                    [0x1a3, 0x1aa, 0x1af, 0x1b7, 0x1bc, 0x1c4, 0x1d0, 0x1d6],
                );
                for (off, x) in [(0x4cu32, 0u32), (0x50, 0), (0x54, 0), (0x58, 0x0048_0000)] {
                    g.write::<u32>(c + off, x);
                }
                for (off, x) in [
                    (0x6544u32, 0x437f_0000u32),
                    (0x6540, 0x437f_0000),
                    (0x653c, 0x437f_0000),
                    (0x6548, 0x4300_0000),
                    (0x6554, 0),
                    (0x6550, 0),
                    (0x654c, 0),
                    (0x6558, 0x4300_0000),
                ] {
                    g.write::<u32>(o + off, x);
                }
                // call width: 3
                g.call(t(g, 0x203, 234), 0, &[c + 0x3c, c + 0x2c, c + 0x1c]);
                let fl = g.read::<u16>(c + 0x58) | 1;
                g.write::<u16>(c + 0x58, fl);
                g.write::<u32>(c + 0x54, 1);
                g.write::<u32>(c + 0x50, 0x3ccc_cccd);
                g.write::<u32>(c + 0x4c, 0);
                // the node N, its vtable after it joins the list
                let n = o + 0x5c54;
                list_node_0645(
                    g,
                    img,
                    f,
                    n,
                    [0x291, 0x298, 0x29d, 0x2a5, 0x2aa, 0x2b2, 0x2be, 0x2c4],
                );
                g.write::<u32>(n, w(0x2ca));
                let (v, m) = (b + 0x70, b + 0x80);
                for (j, x) in (0u32..).zip([0u32, 0, 0xc47a_0000, 0x3f80_0000]) {
                    g.write::<u32>(v + 4 * j, x);
                }
                // call width: 2
                g.call(t(g, 0x2d7, 210), 0, &[g.read::<u32>(o + 0x58d4), m]);
                // call width: 3
                g.call(t(g, 0x321, 229), 0, &[o + 0x5c70, m, v]);
                // the part node
                let p = o + 0x657c;
                list_node_0645(
                    g,
                    img,
                    f,
                    p,
                    [0x345, 0x34c, 0x351, 0x359, 0x35e, 0x366, 0x377, 0x37f],
                );
                // call width: 3
                let part = g.call(t(g, 0x370, 719), 0, &[w(0x38c), 6, 0]).eax;
                g.write::<u32>(p + 0x24, part);
                g.write::<u32>(p + 0x1c, w(0x398));
                g.write::<u32>(p + 0x20, n);
                g.write::<u32>(p + 0x34, 0x40);
                g.write::<u32>(p + 0x28, 0x100);
                g.write::<u32>(p + 0x2c, 0);
                g.write::<u32>(p, w(0x3b6));
                let fl = g.read::<u32>(p + 0x2c) | 1;
                g.write::<u32>(p + 0x2c, fl);
                g.write::<u32>(p + 0x30, 0);
                g.write::<u32>(o + 0x1c, 0x1e);
                g.write::<u32>(o + 0x18, 0x14);
            }
            0xc => {
                if g.read::<u32>(o + 0x1c) != 0 {
                    return count_down(g, state);
                }
                if g.read::<u8>(o + 0x63c8) & 4 == 0 {
                    let s = o + 0x62e0;
                    let cursor = b + 0x10;
                    g.write::<u32>(cursor, s);
                    crate::nodes::free_then_unlink(g, img, &at(0x3fd, cursor, fr.edx));
                    if g.read::<u8>(s + 0xe8) & 4 == 0 {
                        let x = g.read::<u32>(s + 0x50);
                        g.write::<u32>(g.read::<u32>(w(0x40b)), x);
                        g.call(t(g, 0x415, 713), 0, &[s + 0x60]);
                    }
                }
                g.write::<u32>(o + 0x18, 0x2710);
            }
            0x14 => {
                if g.read::<i32>(o + 0x1c) > 0 {
                    return count_down(g, state);
                }
                g.write::<u32>(o + 0x1c, 0x82);
                g.write::<u32>(o + 0x18, 0xa);
            }
            0xa => {
                if g.read::<i32>(o + 0x1c) > 0 {
                    return count_down(g, 0);
                }
                let (node, list) = (o + 0x5c80, w(0x4c9));
                crate::sb_misc::node_append(g, img, call(0x4ce), node, list, 0);
                let s = o + 0x6090;
                for off in [0x5c9cu32, 0x5ca0, 0x5ca4] {
                    g.write::<u32>(o + off, 0x3f80_0000);
                }
                g.write::<u32>(o + 0x5ca8, 0x3e99_999a);
                // the group: the append's `ecx` and `edx` as it leaves
                // them (the node and the list)
                let group = b + 0x28;
                for (j, x) in (0u32..).zip([s, list, w(0x51a), o + 0x5c54, w(0x526), node]) {
                    g.write::<u32>(group + 4 * j, x);
                }
                crate::nodes::node_insert(g, img, &at(0x52b, group, list));
                // push 0; sub esp, 8; push the node: the return address at
                // B - 0x14 (the two middle words are never written; the
                // callee does not read them)
                let esp = b.wrapping_sub(0x14);
                g.write::<u32>(esp + 4, s);
                g.write::<u32>(esp + 0x10, 0);
                let pf = Frame {
                    at: call(0x53c),
                    esp,
                    ecx: o + 0x62e0,
                    edx: list,
                    ..*fr
                };
                let eax = crate::s3_nodes::particle_node_with(g, img, &pf);
                let st = g.read::<u32>(o + 0x18);
                g.write::<u32>(o + 0x18, st.wrapping_add(1));
                g.write::<u32>(o + 0x1c, 0xdb);
                g.write::<u32>(o + 0x48, 0x2710);
                g.write::<u32>(o + 0x5f8, 0x2710);
                return Ret::eax(eax);
            }
            0xb => {
                if g.read::<u32>(o + 0x1c) != 0 {
                    return count_down(g, 0);
                }
                g.call(t(g, 0x470, 639), 0, &[0x5a]);
                // call width: 2
                let eax = g
                    .call(t(g, 0x47f, 8), 0, &[g.read::<u32>(o + 0x58d4), 0])
                    .eax;
                let st = g.read::<u32>(o + 0x18);
                g.write::<u32>(o + 0x18, st.wrapping_add(1));
                g.write::<u32>(o + 0x1c, 0x3b);
                return Ret::eax(eax);
            }
            0x2710 => {
                let first = o + 0x5880;
                let cursor = b + 0x14;
                for (j, x) in (0u32..).zip([first, 0x3f80_0000, 0]) {
                    g.write::<u32>(cursor + 4 * j, x);
                }
                crate::l3_common::chr_fade(g, img, call(0x59c), cursor);
                crate::sa_misc::leave(g, first, 0);
                let cursor = b + 0x10;
                g.write::<u32>(cursor, first);
                crate::sa_summon::model_setup(g, img, &at(0x5b0, cursor, fr.edx));
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                return count_down(g, prev);
            }
            s if s < 0xc => return count_down(g, s.wrapping_sub(0xb)),
            s => return count_down(g, s),
        }
    }
}

/// [`summon_task_0645`] (1514 bytes, 1 overlay).
pub const SUMMON_TASK_0645: Sig = Sig {
    len: 1514,
    hash: 0x3a3c_d5e1_e65e_f68c,
    calls: &[
        (0x74, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0xa2, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0xd0, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0xfe, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x12c, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x16d, Callee::Sig(&crate::m0645::VALUE_START)),
        (0x177, Callee::Sig(&crate::m0645::MOVER)),
        (0x184, Callee::Sig(&SUMMON_SETUP_0645)),
        (0x3fd, Callee::Sig(&crate::nodes::FREE_THEN_UNLINK)),
        // __security_check_cookie
        (0x4b0, Callee::Any),
        (0x4ce, Callee::Sig(&crate::sb_misc::NODE_APPEND)),
        (0x52b, Callee::Sig(&crate::nodes::INSERT)),
        (0x53c, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x56e, Callee::Any),
        (0x59c, Callee::Sig(&crate::l3_common::CHR_FADE)),
        (0x5a3, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x5b0, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x5e2, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0645`'s spark task (0x10006990, 1314 bytes; a task function
/// (its address pushed at 0x10006ed9), cdecl `(task)`, the object `O =
/// task[4]`; a 16-aligned cookie frame: `B = ((E - 4) & !0xf) - 0x130`,
/// the handle's matrix `M` at `B + 0xa8`, a render record `R` at `B +
/// 0x20`, the scale matrix at `B + 0x68`, a second matrix at `B + 0xe8`,
/// the point at `B + 0x58`). First `M` = slot 210 `(O[0xe80][0x54])`.
/// By the state `O + 0xe6c` (a change runs the new state at once):
/// - 0: state 1 and the wait `O + 0xe70` 20;
/// - 1: once the wait runs out, the alpha `O + 0xe78` 0xff and state 10;
/// - 10: with the alpha above 0 a spark is born (below), else state
///   20000;
/// - 11: the sparks drawn (below);
/// - 20000: the task node unlinked (returns its `+4`) and state 40000.
///
/// A spark (index `i = O[0xe74]`, counted up; slot `j = i % 96`): its
/// record word `O + 0xcd0 + 4 j` 0 and its `u16 +2` slot 324; its point
/// `O + 0x10 + 0x10 j` = `M` times `((K - (r - K')) K'' K''', (...), 0,
/// 1)` (slot 229, `r` slot 307's ST0); its speed `O + 0x610 + 0x10 j`
/// three `K - (r - K')`, `+0xc` the size `((r0 - K') K4 + K5) K6 K`; `u16
/// O + 0xc10 + 2 j` its slot 324 word. Then the draw: `R` (`u16 +2` 0x80,
/// colour 0x808080 with the alpha byte, `+0x10` `M`, `+0x1c` `B + 0xe8`,
/// `+0x14` the saved `*G_Z`), for each of the first `min(O[0xe74], 0x60)`
/// sparks: slot 229 `(B + 0x98, G_V, point)`, the scale its size on the
/// diagonal, slot 230 into `M` and `B + 0xe8`, the second matrix's `y`
/// lowered by `K7`, the record's sequence step
/// ([`motion_sequence_step`](crate::s6_misc::motion_sequence_step), `ecx`
/// the record word, `edx` = `O[0xe50]`, the stack `(R)`), the point moved
/// by the speed's `x`/`y`, the speed's `y` less `K8`. `*G_Z` restored;
/// with `O + 0xe7c` set the alpha drops by 4. Every way ends with the
/// wait counted down; returns what `eax` holds there.
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn spark_task_0645<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let random = |g: &mut G, o: u32| st0_f32(g.call(t(g, o, 307), 0, &[]).st0);
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 4);
    let b = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x130);
    let m = b + 0xa8;
    let h = g.read::<u32>(g.read::<u32>(o + 0xe80).wrapping_add(0x54));
    // call width: 2
    g.call(t(g, 0x38, 210), 0, &[h, m]);
    let exit = |g: &mut G, eax: u32| {
        let n = g.read::<u32>(o + 0xe70);
        g.write::<u32>(o + 0xe70, n.wrapping_sub(1));
        Ret::eax(eax)
    };
    let mut state = g.read::<u32>(o + 0xe6c);
    let spawn = loop {
        match state {
            0 | 1 => {
                if state == 0 {
                    g.write::<u32>(o + 0xe70, 0x14);
                    g.write::<u32>(o + 0xe6c, 1);
                }
                if g.read::<i32>(o + 0xe70) > 0 {
                    // `eax`: 1 after the start (`lea eax, [ecx + 1]`), 0
                    // for state 1
                    return exit(g, 1 - state);
                }
                g.write::<u32>(o + 0xe78, 0xff);
                g.write::<u32>(o + 0xe6c, 10);
            }
            10 => {
                if g.read::<i32>(o + 0xe78) > 0 {
                    break true;
                }
                g.write::<u32>(o + 0xe6c, 0x2710);
            }
            0xb => break false,
            0x2710 => {
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                g.write::<u32>(o + 0xe6c, 0x4e20);
                return exit(g, prev);
            }
            s if s < 10 => return exit(g, s.wrapping_sub(1)),
            s => return exit(g, s),
        }
        state = g.read::<u32>(o + 0xe6c);
    };
    g.write::<u32>(b + 0x10, o);
    if spawn {
        let n = g.read::<i32>(o + 0xe74);
        let end = n.wrapping_add(1);
        g.write::<u32>(o + 0xe74, end.cast_unsigned());
        let mut i = n;
        while i < end {
            let j = (i % 96).cast_unsigned();
            let at = |base: u32, s: u32| o.wrapping_add(base).wrapping_add(j.wrapping_mul(s));
            g.write::<u32>(at(0xcd0, 4), 0);
            let a = sub(random(g, 0x13b), k(g, 0x155));
            let s324 = g.call(t(g, 0x15a, 324), 0, &[]).eax;
            g.write::<u16>(at(0xcd2, 4), s324 as u16);
            let lane = |g: &mut G, r: u32, (k1, k0, k2, k3): (u32, u32, u32, u32)| {
                let r = random(g, r);
                mul(mul(sub(k(g, k0), sub(r, k(g, k1))), k(g, k2)), k(g, k3))
            };
            let x = lane(g, 0x175, (0x18f, 0x197, 0x1a8, 0x1b0));
            g.write::<f32>(b + 0x58, x);
            let y = lane(g, 0x19c, (0x1d0, 0x1d8, 0x1fa, 0x207));
            g.write::<f32>(b + 0x5c, y);
            g.write::<u32>(b + 0x60, 0);
            g.write::<u32>(b + 0x64, 0x3f80_0000);
            let p = at(0x10, 0x10);
            // call width: 3
            g.call(t(g, 0x20d, 229), 0, &[p, m, b + 0x58]);
            for (off, r, k1, k0) in [
                (0x610u32, 0x230u32, 0x255u32, 0x243u32),
                (0x614, 0x26c, 0x286, 0x28e),
                (0x618, 0x2a4, 0x2be, 0x2c6),
            ] {
                let r = random(g, r);
                let v = sub(k(g, k0), sub(r, k(g, k1)));
                g.write::<f32>(at(off, 0x10), v);
            }
            i = i.wrapping_add(1);
            let size = mul(
                mul(add(mul(a, k(g, 0x2ee)), k(g, 0x2f6)), k(g, 0x2fe)),
                k(g, 0x306),
            );
            g.write::<f32>(at(0x61c, 0x10), size);
            let s = g.read::<u16>(at(0xcd2, 4));
            g.write::<u16>(at(0xc10, 2), s);
        }
    }
    // the draw
    let zg = w(0x331);
    let z = g.read::<u32>(zg);
    let r = b + 0x20;
    g.write::<u32>(r + 0x10, m);
    g.write::<u32>(r + 0x1c, b + 0xe8);
    g.write::<u16>(r + 2, 0x80);
    g.write::<u32>(r + 4, 0x0080_8080);
    for off in [0x6cu32, 0x70, 0x74, 0x78, 0x80, 0x84, 0x88, 0x8c, 0x94] {
        g.write::<u32>(b + off, 0);
    }
    g.write::<u32>(r + 8, 0);
    g.write::<u8>(r + 0x18, 0);
    g.write::<u32>(r + 0x14, z);
    let alpha = g.read::<u8>(o + 0xe78);
    g.write::<u8>(r + 7, alpha);
    let count = g.read::<i32>(o + 0xe74).min(0x60);
    let mut eax = count.cast_unsigned();
    if count > 0 {
        let mut p = o + 0x10;
        let mut rec = o + 0xcd0;
        for _ in 0..count {
            // call width: 3
            g.call(t(g, 0x40f, 229), 0, &[b + 0x98, w(0x409), p]);
            let s = g.read::<u32>(p + 0x60c);
            for off in [0x90u32, 0x7c, 0x68] {
                g.write::<u32>(b + off, s);
            }
            // call width: 2
            g.call(t(g, 0x434, 230), 0, &[m, b + 0x68]);
            // call width: 2
            g.call(t(g, 0x466, 230), 0, &[b + 0xe8, b + 0x68]);
            let y = sub(g.read::<f32>(b + 0x120), k(g, 0x47f));
            g.write::<f32>(b + 0x120, y);
            // push R: the return address at B - 0x10
            let esp = b.wrapping_sub(0x10);
            g.write::<u32>(esp + 4, r);
            let sf = Frame {
                at: img.rel32_target(f + 0x49f),
                esp,
                ecx: rec,
                edx: g.read::<u32>(o + 0xe50),
                ..*fr
            };
            eax = crate::s6_misc::motion_sequence_step(g, img, &sf);
            rec += 4;
            g.write::<u32>(b + 0x10, rec);
            let px = add(g.read::<f32>(p), g.read::<f32>(p + 0x600));
            let vy = g.read::<f32>(p + 0x604);
            g.write::<f32>(p, px);
            let py = add(vy, g.read::<f32>(p + 4));
            g.write::<f32>(p + 4, py);
            g.write::<f32>(p + 0x604, sub(vy, k(g, 0x4d7)));
            p += 0x10;
        }
    }
    g.write::<u32>(zg, z);
    if g.read::<u32>(o + 0xe7c) != 0 {
        let a = g.read::<u32>(o + 0xe78);
        g.write::<u32>(o + 0xe78, a.wrapping_sub(4));
    }
    exit(g, eax)
}

/// [`spark_task_0645`] (1314 bytes, 1 overlay).
pub const SPARK_TASK_0645: Sig = Sig {
    len: 1314,
    hash: 0x208c_eb7f_f093_f9d2,
    calls: &[
        (0x49f, Callee::Sig(&crate::s6_misc::MOTION_SEQUENCE_STEP)),
        // __security_check_cookie
        (0x51a, Callee::Any),
    ],
    imports: &[],
};
