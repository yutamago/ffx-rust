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
pub(crate) fn list_node_0645<G: Game>(g: &mut G, img: &Image, f: u32, node: u32, ops: [u32; 8]) {
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
    // junk read: [B + 0x20] (u16) and [B + 0x2c] never written; 0 (brief
    // 2026-10-24): the render record's words the sequence step's slots get
    g.write::<u16>(r, 0);
    g.write::<u32>(r + 0xc, 0);
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

/// One step of `magic_0478`'s colour fades over DLL globals: `dst = K -
/// src` (or `a - b`), a word set, or the flags word changed.
#[derive(Clone, Copy)]
enum Fade {
    /// `*dst = K - *src` (operands).
    Sub(u32, u32),
    /// `*dst = *a - *b`.
    Diff(u32, u32, u32),
    /// `*dst = value`.
    Set(u32, u32),
}

/// Runs [`Fade`] steps in order (`K` the float at operand `k`).
fn fades_0478<G: Game>(g: &mut G, img: &Image, f: u32, k: u32, steps: &[Fade]) {
    let w = |o: u32| img.word(f + o);
    let c = g.read::<f32>(w(k));
    for &s in steps {
        match s {
            Fade::Sub(dst, src) => {
                let v = sub(c, g.read::<f32>(w(src)));
                g.write::<f32>(w(dst), v);
            }
            Fade::Diff(dst, a, b) => {
                let v = sub(g.read::<f32>(w(a)), g.read::<f32>(w(b)));
                g.write::<f32>(w(dst), v);
            }
            Fade::Set(dst, v) => g.write::<u32>(w(dst), v),
        }
    }
}

/// `magic_0478`'s summon task (0x10006c40, 1813 bytes; a task function
/// (its address pushed at 0x100074d6), cdecl `(task)`, the object `O =
/// task[4]`; an 8-aligned frame: `B = ((E - 4) & !7) - 0x48`, records at
/// `B + 0x20`, `B + 0x30`, `B + 0x18`, a cursor at `B + 0x14`). By the
/// state `O + 0x18` (a change runs the new state at once), the count `O +
/// 0x20`:
/// - 0: the model node `O + 0x514`
///   ([`model_node_init`](crate::sa_summon::model_node_init)) and the
///   model nodes `O + 0x24`
///   ([`model_nodes`](crate::s3_summon::model_nodes)) of the actor `*G_A`;
///   four nodes joining the DLL's list (`O + 0x5ac` with slot 268's
///   colour and slot 12's alpha at `+0x1c`, `O + 0x604`, `O + 0x5d8` with
///   slot 251's scale, the bone `O + 0x77c` over the three); the count
///   `O + 0x624` = `(K - a) M / D` truncated (`a` slot 333 with bit 0 of
///   `O + 0xd0`, else slot 601); the particle node `O + 0x8d0`
///   ([`particle_node`](crate::sb_nodes::particle_node)), the colour node
///   `G_N` ([`colour_node`](crate::s3_misc::colour_node)), the fades at
///   0.5; the part node `O + 0x9d0` (slot 719) and the blink object `O +
///   0xa08` ([`init`](crate::m0478::init)); state 20;
/// - 20: state 21, the count 0x2d;
/// - 21: when the count runs out, the fades at 1.0 and state 20000;
/// - 20000: both model nodes leave their parents
///   ([`leave`](crate::sa_misc::leave)), the second's model set up
///   ([`model_setup`](crate::sa_summon::model_setup), its cursor holding
///   the `ecx` the leave leaves: it never writes it, brief 2026-11-08),
///   the task node unlinked (returns its `+4`).
///
/// Every way ends with `O + 0x1c` up and the count down; returns what
/// `eax` holds there (the state, 21 after a wait).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0478<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use Fade::{Diff, Set, Sub};
    const ONE: u32 = 0x3f80_0000;
    const HALF: u32 = 0x3f00_0000;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 4);
    let b = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x48);
    // a call without pushes: its return address at `B - 4`
    let at = |c: u32, ecx: u32, edx: u32| Frame {
        at: call(c),
        esp: b.wrapping_sub(4),
        ecx,
        edx,
        ..*fr
    };
    let exit = |g: &mut G, eax: u32| {
        let n = g.read::<u32>(o + 0x1c);
        g.write::<u32>(o + 0x1c, n.wrapping_add(1));
        let c = g.read::<u32>(o + 0x20);
        g.write::<u32>(o + 0x20, c.wrapping_sub(1));
        Ret::eax(eax)
    };
    loop {
        let state = g.read::<u32>(o + 0x18);
        match state {
            0 => {
                let a = g.read::<u32>(w(0x1ca));
                let c1 = o + 0x514;
                let r = b + 0x20;
                g.write::<u32>(r, c1);
                g.write::<u32>(c1, w(0x1da));
                g.write::<u32>(r + 4, w(0x1e6));
                g.write::<u32>(r + 8, a);
                g.write::<u8>(r + 0xc, 0);
                crate::sa_summon::model_node_init(g, img, call(0x1f4), r);
                let a = g.read::<u32>(w(0x1f9));
                let c2 = o + 0x24;
                let r = b + 0x30;
                g.write::<u32>(r, c2);
                g.write::<u32>(c2, w(0x206));
                g.write::<u32>(r + 4, w(0x212));
                g.write::<u32>(r + 8, a);
                g.write::<u32>(r + 0xc, 0);
                g.write::<u8>(r + 0x10, 0);
                crate::s3_summon::model_nodes(g, img, &at(0x228, r, fr.edx));
                // the colour node: slot 268's colour, slot 12's alpha
                let n1 = o + 0x5ac;
                list_node_0645(
                    g,
                    img,
                    f,
                    n1,
                    [0x241, 0x248, 0x24d, 0x255, 0x25a, 0x262, 0x26e, 0x27f],
                );
                g.write::<u32>(n1, w(0x286));
                let c = o + 0x5c8;
                // call width: 3
                g.call(t(g, 0x273, 268), 0, &[g.read::<u32>(o + 0x94), 0x11, c]);
                g.write::<u32>(c + 0xc, ONE);
                let al = st0_f32(g.call(t(g, 0x29b, 12), 0, &[g.read::<u32>(o + 0x94)]).st0);
                g.write::<f32>(c + 4, al);
                let x = sub(g.read::<f32>(o + 0x5cc), k(g, 0x2c3));
                g.write::<u32>(o + 0x5d4, ONE);
                g.write::<f32>(o + 0x5cc, x);
                let n2 = o + 0x604;
                g.write::<u32>(n2, w(0x2e4));
                list_node_0645(
                    g,
                    img,
                    f,
                    n2,
                    [0x2f7, 0x2fe, 0x303, 0x30b, 0x310, 0x318, 0x324, 0x32a],
                );
                g.write::<u32>(o + 0x628, 0);
                g.write::<u32>(o + 0x620, 0);
                let a = if g.read::<u8>(o + 0xd0) & 1 != 0 {
                    let r = g.call(t(g, 0x34c, 333), 0, &[g.read::<u32>(o + 0x94)]);
                    sub(k(g, 0x366), st0_f32(r.st0))
                } else {
                    let y = b + 0x14;
                    // call width: 2
                    g.call(t(g, 0x37b, 601), 0, &[g.read::<u32>(o + 0x90), y]);
                    sub(k(g, 0x391), g.read::<f32>(y))
                };
                let n = crate::sb_sse::div(mul(a, k(g, 0x3a2)), k(g, 0x3b0));
                g.write::<u32>(o + 0x624, crate::s4_summon::trunc(n));
                let n3 = o + 0x5d8;
                list_node_0645(
                    g,
                    img,
                    f,
                    n3,
                    [0x3cd, 0x3d4, 0x3d9, 0x3e1, 0x3e6, 0x3ee, 0x3fa, 0x405],
                );
                g.write::<u32>(n3, w(0x40b));
                let h = g.read::<u32>(o + 0x90);
                for off in [0x5f4u32, 0x5f8, 0x5fc] {
                    g.write::<u32>(o + off, ONE);
                }
                let s = st0_f32(g.call(t(g, 0x3ff, 251), 0, &[h]).st0);
                let s = mul(mul(s, k(g, 0x449)), k(g, 0x454));
                g.write::<f32>(o + 0x600, s);
                // comiss; jbe: an unordered scale stays
                if s > k(g, 0x45b) {
                    g.write::<u32>(o + 0x600, ONE);
                }
                // the bone over the three nodes
                let n4 = o + 0x77c;
                g.write::<u32>(n4, w(0x47b));
                list_node_0645(
                    g,
                    img,
                    f,
                    n4,
                    [0x48e, 0x495, 0x49a, 0x4a2, 0x4a7, 0x4af, 0x4bb, 0x4c1],
                );
                g.write::<u32>(n4 + 0x13c, n1);
                g.write::<u32>(n4 + 0x144, n2);
                let owner = g.read::<u32>(w(0x4e4));
                g.write::<u32>(n4 + 0x140, n3);
                g.write::<u32>(n4 + 0x128, w(0x4f4));
                g.write::<u8>(n4 + 0x148, 0);
                let pn = o + 0x8d0;
                let r = b + 0x30;
                g.write::<u32>(r, pn);
                g.write::<u32>(pn, w(0x505));
                for (j, x) in (1u32..).zip([w(0x511), n4, owner, w(0x521), 0]) {
                    g.write::<u32>(r + 4 * j, x);
                }
                crate::sb_nodes::particle_node(g, img, call(0x52e), r);
                let r = b + 0x18;
                g.write::<u32>(r, w(0x53a));
                g.write::<u32>(r + 4, w(0x542));
                g.write::<u32>(w(0x548), w(0x54c));
                crate::s3_misc::colour_node(g, img, &at(0x551, r, fr.edx));
                // `and [G_N + 0xac], ~1`: the colour node has just set that
                // word to 1, so it is always 0 here (r23: why a mutant of
                // this read survives)
                let fl = g.read::<u32>(w(0x56a)) & !1;
                g.write::<u32>(w(0x56a), fl);
                fades_0478(
                    g,
                    img,
                    f,
                    0x559,
                    &[
                        Set(0x577, HALF),
                        Set(0x581, HALF),
                        Sub(0x58d, 0x564),
                        Set(0x59e, HALF),
                        Set(0x5a8, HALF),
                        Set(0x5b2, HALF),
                        Set(0x5bc, HALF),
                        Sub(0x5c8, 0x598),
                        Set(0x5d9, HALF),
                        Set(0x5e3, 0x3cf5_c28f),
                        Set(0x5ed, 0),
                        Sub(0x5f9, 0x5d3),
                        Sub(0x60c, 0x604),
                        Sub(0x61f, 0x617),
                        Sub(0x632, 0x62a),
                        Diff(0x64a, 0x63a, 0x642),
                        Sub(0x65d, 0x655),
                    ],
                );
                // the part node
                let n5 = o + 0x9d0;
                list_node_0645(
                    g,
                    img,
                    f,
                    n5,
                    [0x670, 0x677, 0x67c, 0x684, 0x689, 0x691, 0x6a2, 0x6aa],
                );
                // call width: 3
                let part = g.call(t(g, 0x69b, 719), 0, &[w(0x6b7), 6, 0]).eax;
                g.write::<u32>(n5 + 0x24, part);
                g.write::<u32>(n5 + 0x1c, n4);
                g.write::<u32>(n5 + 0x20, w(0x6c6));
                g.write::<u32>(n5 + 0x34, 0x40);
                g.write::<u32>(n5 + 0x28, 0x100);
                g.write::<u32>(n5 + 0x2c, 0);
                g.write::<u32>(n5, w(0x6e1));
                let fl = g.read::<u32>(n5 + 0x2c) | 1;
                g.write::<u32>(n5 + 0x2c, fl);
                g.write::<u32>(n5 + 0x30, 0);
                crate::m0478::init(g, img, &at(0x6fd, o + 0xa08, o + 0x40));
                g.write::<u32>(o + 0x18, 0x14);
            }
            0x14 | 0x15 => {
                if state == 0x14 {
                    g.write::<u32>(o + 0x20, 0x2d);
                    g.write::<u32>(o + 0x18, 0x15);
                }
                if g.read::<i32>(o + 0x20) > 0 {
                    return exit(g, 0x15);
                }
                // the flags word read first, written in its place
                let fl = (g.read::<u32>(w(0x5e)) & !1) | 4;
                fades_0478(
                    g,
                    img,
                    f,
                    0x24,
                    &[
                        Set(0x6a, ONE),
                        Sub(0x76, 0x59),
                        Set(0x87, ONE),
                        Set(0x91, ONE),
                        Set(0x9b, ONE),
                        Set(0xa5, ONE),
                        Sub(0xb1, 0x81),
                        Set(0xc2, ONE),
                        Set(0xcc, ONE),
                        Set(0xd6, 0x3cf5_c28f),
                        Set(0xe0, 0),
                        Sub(0xec, 0xbc),
                        Set(0xfc, fl),
                        Set(0x102, ONE),
                        Sub(0x10e, 0xf7),
                        Sub(0x121, 0x119),
                        Sub(0x134, 0x12c),
                        Diff(0x14c, 0x13c, 0x144),
                        Sub(0x15f, 0x157),
                    ],
                );
                g.write::<u32>(o + 0x18, 0x2710);
            }
            0x2710 => {
                let c2 = o + 0x24;
                crate::sa_misc::leave(g, c2, 0);
                let c1 = o + 0x514;
                crate::sa_misc::leave(g, c1, 0);
                let cursor = b + 0x14;
                g.write::<u32>(cursor, c1);
                crate::sa_summon::model_setup(g, img, &at(0x19e, cursor, fr.edx));
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                return exit(g, prev);
            }
            s => return exit(g, s),
        }
    }
}

/// [`summon_task_0478`] (1813 bytes, 1 overlay).
pub const SUMMON_TASK_0478: Sig = Sig {
    len: 1813,
    hash: 0x0285_5817_cb80_93e0,
    calls: &[
        (0x186, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x191, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x19e, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
        (0x1f4, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x228, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x52e, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x551, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x6fd, Callee::Sig(&crate::m0478::INIT)),
    ],
    imports: &[],
};

/// A part node of `magic_0482`'s summon task: joins the DLL's list
/// (operands `list`), slot 719 `(PART, 6, n)` its part `+0x24`, `+0x1c` =
/// the DLL word at operand `owner`, `+0x20` = `on`, `+0x34` 0x40, `+0x28`
/// 0x100, the vtable (operand `vt`), flag bit 0 of `+0x2c`, `+0x30` 0.
#[allow(clippy::too_many_arguments)] // the operands of one inlined block
fn part_node_0482<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    node: u32,
    list: [u32; 8],
    (s719, part, n): (u32, u32, u32),
    (owner, on, vt): (u32, u32, u32),
) {
    let w = |o: u32| img.word(f + o);
    list_node_0645(g, img, f, node, list);
    // call width: 3
    let p = g
        .call(slot_target(g, w(s719), 719), 0, &[w(part), 6, n])
        .eax;
    g.write::<u32>(node + 0x24, p);
    g.write::<u32>(node + 0x1c, w(owner));
    g.write::<u32>(node + 0x20, on);
    g.write::<u32>(node + 0x34, 0x40);
    g.write::<u32>(node + 0x28, 0x100);
    g.write::<u32>(node + 0x2c, 0);
    g.write::<u32>(node, w(vt));
    let fl = g.read::<u32>(node + 0x2c) | 1;
    g.write::<u32>(node + 0x2c, fl);
    g.write::<u32>(node + 0x30, 0);
}

/// `magic_0482`'s summon task (0x10006e10, 2233 bytes; a task function
/// (its address pushed at 0x10007846), cdecl `(task)`, the object `O = task[4]`; an 8-aligned frame: `B =
/// ((E - 4) & !7) - 0x40`, records at `B + 0x18`, `B + 0x28`, `B +
/// 0x10`). By the state `O + 0x18` (a change runs the new state at once),
/// the count `O + 0x20`:
/// - 0: the model node `O + 0x514`
///   ([`model_node_init`](crate::sa_summon::model_node_init)) and the
///   model nodes `O + 0x24`
///   ([`model_nodes`](crate::s3_summon::model_nodes)) of `*G_A`; list
///   nodes `O + 0x5ac` (slot 268's colour), `O + 0x6b4` (a bone over
///   three DLL objects), `O + 0x630` (its scale by slot 415 being 0x7a
///   or not), `O + 0x5d8`; the inserted node `O + 0x808`
///   ([`insert`](crate::m0482::insert)), the particle node `O + 0xab0`
///   ([`particle_node`](crate::sb_nodes::particle_node)), the colour node
///   ([`colour_node`](crate::s3_misc::colour_node)), the fades at 0.5,
///   the part node `O + 0xcb0`, the object `O + 0xd20` joining the DLL's
///   ring ([`ring_join`](crate::h6_setup::ring_join)); state 10;
/// - 10: state 11, the count 0x19;
/// - 11: when the count runs out, the list node `O + 0x604`, the inserted
///   node `O + 0x95c` and the particle node `O + 0xbb0`, slot 239, the
///   part node `O + 0xce8`, the fades at their end; state 20000;
/// - 20000: once `O + 0xd38` is 40000, both model nodes leave their
///   parents ([`leave`](crate::sa_misc::leave)), the second's model set
///   up ([`model_setup`](crate::sa_summon::model_setup), its cursor
///   holding the `ecx` the leave leaves: it never writes it), the task
///   node unlinked (returns its `+4`).
///
/// Every way ends with `O + 0x1c` up and the count down; returns what
/// `eax` holds there (the state, 11 after a wait).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0482<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use Fade::{Diff, Set, Sub};
    const ONE: u32 = 0x3f80_0000;
    const HALF: u32 = 0x3f00_0000;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 4);
    let b = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x40);
    // a call without pushes: its return address at `B - 4`
    let at = |c: u32, ecx: u32| Frame {
        at: call(c),
        esp: b.wrapping_sub(4),
        ecx,
        ..*fr
    };
    let exit = |g: &mut G, eax: u32| {
        let n = g.read::<u32>(o + 0x1c);
        g.write::<u32>(o + 0x1c, n.wrapping_add(1));
        let c = g.read::<u32>(o + 0x20);
        g.write::<u32>(o + 0x20, c.wrapping_sub(1));
        Ret::eax(eax)
    };
    // an inserted node and its particle node (the records at `B + 0x28`)
    let insert = |g: &mut G, (node, list, a, bb, c): (u32, u32, u32, u32, u32), ops: (u32, u32)| {
        let r = b + 0x28;
        for (j, x) in (0u32..).zip([node, w(list), a, bb, c]) {
            g.write::<u32>(r + 4 * j, x);
        }
        crate::m0482::insert(g, img, call(ops.0), r);
        let n = g.read::<u32>(r);
        g.write::<u32>(n, w(ops.1));
    };
    let particle = |g: &mut G, pn: u32, src: u32, ops: [u32; 5], index: u32| {
        let r = b + 0x28;
        let owner = g.read::<u32>(w(ops[1]));
        g.write::<u32>(r, pn);
        g.write::<u32>(pn, w(ops[0]));
        for (j, x) in (1u32..).zip([w(ops[2]), src, owner, w(ops[3]), index]) {
            g.write::<u32>(r + 4 * j, x);
        }
        crate::sb_nodes::particle_node(g, img, call(ops[4]), r);
    };
    loop {
        let state = g.read::<u32>(o + 0x18);
        match state {
            0 => {
                let a = g.read::<u32>(w(0x8a));
                let c1 = o + 0x514;
                let r = b + 0x18;
                g.write::<u32>(r, c1);
                g.write::<u32>(c1, w(0x9a));
                g.write::<u32>(r + 4, w(0xa6));
                g.write::<u32>(r + 8, a);
                g.write::<u8>(r + 0xc, 0);
                crate::sa_summon::model_node_init(g, img, call(0xb4), r);
                let a = g.read::<u32>(w(0xb9));
                let c2 = o + 0x24;
                let r = b + 0x28;
                g.write::<u32>(r, c2);
                g.write::<u32>(c2, w(0xc6));
                g.write::<u32>(r + 4, w(0xd2));
                g.write::<u32>(r + 8, a);
                g.write::<u32>(r + 0xc, 0);
                g.write::<u8>(r + 0x10, 0);
                crate::s3_summon::model_nodes(g, img, &at(0xe8, r));
                let n1 = o + 0x5ac;
                list_node_0645(
                    g,
                    img,
                    f,
                    n1,
                    [0x101, 0x108, 0x10d, 0x115, 0x11a, 0x122, 0x12e, 0x13f],
                );
                g.write::<u32>(n1, w(0x146));
                let c = o + 0x5c8;
                // call width: 3
                g.call(t(g, 0x133, 268), 0, &[g.read::<u32>(o + 0x94), 0x11, c]);
                g.write::<u32>(c + 0xc, ONE);
                // the bone over three DLL objects
                let n2 = o + 0x6b4;
                list_node_0645(
                    g,
                    img,
                    f,
                    n2,
                    [0x179, 0x180, 0x185, 0x18d, 0x192, 0x19a, 0x1a6, 0x1ac],
                );
                g.write::<u32>(n2 + 0x13c, w(0x1b6));
                g.write::<u32>(n2 + 0x144, w(0x1c0));
                g.write::<u32>(n2 + 0x140, w(0x1ca));
                g.write::<u32>(n2 + 0x128, 0);
                g.write::<u8>(n2 + 0x148, 0);
                g.write::<u32>(n2, w(0x1e1));
                let x = g.read::<u32>(o + 0x568);
                g.write::<u32>(o + 0x800, x);
                g.write::<u32>(o + 0x804, 0xc);
                let n3 = o + 0x630;
                list_node_0645(
                    g,
                    img,
                    f,
                    n3,
                    [0x210, 0x217, 0x21c, 0x224, 0x229, 0x231, 0x23d, 0x24e],
                );
                let h = g.read::<u32>(w(0x248));
                g.write::<u32>(n3 + 0x38, ONE);
                g.write::<u32>(n3 + 0x3c, n2);
                g.write::<u32>(n3, w(0x25e));
                let kind = g.call(t(g, 0x242, 415), 0, &[h]).eax;
                g.write::<u32>(o + 0x660, 0xbe4c_cccd);
                let (s1, s2) = if kind == 0x7a {
                    (ONE, ONE)
                } else {
                    (0x3fa0_0000, 0xbecc_cccd)
                };
                g.write::<u32>(o + 0x65c, s1);
                g.write::<u32>(o + 0x664, s2);
                let n4 = o + 0x5d8;
                list_node_0645(
                    g,
                    img,
                    f,
                    n4,
                    [0x2bb, 0x2c2, 0x2c7, 0x2cf, 0x2d4, 0x2dc, 0x2e8, 0x2ee],
                );
                g.write::<u32>(n4, w(0x2f4));
                for off in [0x5f4u32, 0x5f8, 0x5fc] {
                    g.write::<u32>(o + off, ONE);
                }
                g.write::<u32>(o + 0x600, 0x3e19_999a);
                let ins = o + 0x808;
                insert(g, (ins, 0x33c, n3, n4, n1), (0x349, 0x359));
                particle(g, o + 0xab0, ins, [0x368, 0x35e, 0x374, 0x384, 0x391], 1);
                let r = b + 0x10;
                g.write::<u32>(r, w(0x39d));
                g.write::<u32>(r + 4, w(0x3a5));
                g.write::<u32>(w(0x3ab), w(0x3af));
                crate::s3_misc::colour_node(g, img, &at(0x3b4, r));
                let fl = g.read::<u32>(w(0x3cd)) & !1;
                g.write::<u32>(w(0x3cd), fl);
                fades_0478(
                    g,
                    img,
                    f,
                    0x3bc,
                    &[
                        Set(0x3da, HALF),
                        Set(0x3e4, HALF),
                        Sub(0x3f0, 0x3c7),
                        Set(0x401, HALF),
                        Set(0x40b, HALF),
                        Set(0x415, HALF),
                        Set(0x41f, HALF),
                        Sub(0x42b, 0x3fb),
                        Set(0x43c, HALF),
                        Set(0x446, 0x3ccc_cccd),
                        Set(0x450, 0),
                        Sub(0x45c, 0x436),
                        Sub(0x46f, 0x467),
                        Sub(0x482, 0x47a),
                        Sub(0x495, 0x48d),
                        Diff(0x4ad, 0x49d, 0x4a5),
                        Sub(0x4c0, 0x4b8),
                    ],
                );
                part_node_0482(
                    g,
                    img,
                    f,
                    o + 0xcb0,
                    [0x4d3, 0x4da, 0x4df, 0x4e7, 0x4ec, 0x4f4, 0x505, 0x50d],
                    (0x4fe, 0x51a, 0),
                    (0x526, n3, 0x544),
                );
                let ring = o + 0xd20;
                crate::h6_setup::ring_join(
                    g,
                    img,
                    f,
                    ring,
                    [0x55e, 0x56b, 0x573, 0x57d, 0x582, 0x58b, 0x592],
                );
                g.write::<u32>(ring + 0x3c, c1);
                g.write::<u32>(ring + 0x30, 0);
                g.write::<u32>(ring + 0x18, 0);
                g.write::<u32>(o + 0x18, 0xa);
            }
            0xa | 0xb => {
                if state == 0xa {
                    g.write::<u32>(o + 0x20, 0x19);
                    g.write::<u32>(o + 0x18, 0xb);
                }
                if g.read::<i32>(o + 0x20) > 0 {
                    return exit(g, 0xb);
                }
                let n5 = o + 0x604;
                list_node_0645(
                    g,
                    img,
                    f,
                    n5,
                    [0x5b, 0x62, 0x67, 0x6f, 0x74, 0x80, 0x5bf, 0x5c5],
                );
                g.write::<u32>(n5, w(0x5cb));
                for off in [0x620u32, 0x624, 0x628] {
                    g.write::<u32>(o + off, ONE);
                }
                g.write::<u32>(o + 0x62c, 0x4002_8f5d);
                let (n1, n3) = (o + 0x5ac, o + 0x630);
                let ins = o + 0x95c;
                insert(g, (ins, 0x619, n1, n5, n3), (0x626, 0x636));
                particle(g, o + 0xbb0, ins, [0x645, 0x63b, 0x651, 0x661, 0x66e], 0);
                let k = u32::from(g.read::<u8>(w(0x675)));
                // call width: 3
                g.call(t(g, 0x67d, 239), 0, &[g.read::<u32>(w(0x683)), k, 0]);
                part_node_0482(
                    g,
                    img,
                    f,
                    o + 0xce8,
                    [0x6a7, 0x6ae, 0x6b3, 0x6bb, 0x6c0, 0x6c8, 0x6d9, 0x6e1],
                    (0x6d2, 0x6ee, 1),
                    (0x702, n1, 0x720),
                );
                let fl = (g.read::<u32>(w(0x730)) & !1) | 4;
                let x1 = sub(g.read::<f32>(w(0x6fb)), 0.0).to_bits();
                let x0 = sub(g.read::<f32>(w(0x73f)), g.read::<f32>(w(0x747))).to_bits();
                let mut steps = vec![
                    Set(0x756, 0),
                    Set(0x760, 0),
                    Set(0x76a, 0),
                    Set(0x774, 0),
                    Set(0x77e, 0),
                    Set(0x788, 0x3dcc_cccd),
                    Set(0x792, 0x3dcc_cccd),
                ];
                for op in [0x79cu32, 0x7a6, 0x7b0, 0x7ba, 0x7c4, 0x7ce, 0x7d8] {
                    steps.push(Set(op, ONE));
                }
                steps.extend([
                    Set(0x7e2, 0x3c75_c28f),
                    Set(0x7ec, 0),
                    Set(0x7f8, x1),
                    Set(0x800, x1),
                    Set(0x808, x1),
                    Set(0x810, x1),
                    Set(0x816, 0x3f66_6666),
                    Set(0x820, 0x3f66_6666),
                    Set(0x82c, x0),
                    Set(0x834, x1),
                    Set(0x839, fl),
                    Set(0x83f, ONE),
                ]);
                fades_0478(g, img, f, 0x6fb, &steps);
                g.write::<u32>(o + 0x18, 0x2710);
            }
            0x2710 => {
                if g.read::<u32>(o + 0xd38) != 0x4e20 {
                    return exit(g, state);
                }
                let c2 = o + 0x24;
                crate::sa_misc::leave(g, c2, 0);
                let c1 = o + 0x514;
                crate::sa_misc::leave(g, c1, 0);
                let cursor = b + 0x10;
                g.write::<u32>(cursor, c1);
                crate::sa_summon::model_setup(g, img, &at(0x88e, cursor));
                let node = g.read::<u32>(task);
                let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
                g.write::<u32>(prev + 8, next);
                let (next, prev) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
                g.write::<u32>(next + 4, prev);
                g.write::<u32>(node + 8, node);
                g.write::<u32>(node + 4, node);
                return exit(g, prev);
            }
            s => return exit(g, s),
        }
    }
}

/// [`summon_task_0482`] (2233 bytes, 1 overlay).
pub const SUMMON_TASK_0482: Sig = Sig {
    len: 2233,
    hash: 0x3fec_d3dd_edbc_7512,
    calls: &[
        (0xb4, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0xe8, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x349, Callee::Sig(&crate::m0482::INSERT)),
        (0x391, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x3b4, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x626, Callee::Sig(&crate::m0482::INSERT)),
        (0x66e, Callee::Sig(&crate::sb_nodes::PARTICLE_NODE)),
        (0x876, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x881, Callee::Sig(&crate::sa_misc::LEAVE)),
        (0x88e, Callee::Sig(&crate::sa_summon::MODEL_SETUP)),
    ],
    imports: &[],
};

/// `magic_0483`'s step (0x10001390, 2362 bytes; a table entry, no
/// arguments; a cookie frame, `ebp = E - 4`, the points `A` at `ebp -
/// 0x14` and `B` at `ebp - 0x24`). Nothing while `*G_ON` is 0. On the
/// first frame (`*G_PHASE` 0) slots 711 and 710 load the effect (the game
/// lock held for the first). Then `*G_OFF` = slot 253 is 0, slots 723
/// `(*G_H)` and 724 `(*G_I)`, `**G_BASE = *G_W + 0x80400`, and by slot
/// 261's answers (2, 1, 0):
/// - 2: once, three parts started from slot 941's programs (the colours
///   and scales of the first two set); while the first part runs, its
///   point the middle of two bones (slot 237, `K` times the sum), slots
///   258, 231, 230 and 712 (the lock 1; its answer ends the part, the
///   lock ends 0); from the count 0x3a the other two parts are drawn at
///   bones 12 and 8 ([`draw`](crate::m0483::draw)); unless `**G_STOP`
///   the count one up;
/// - 1: once two programs freed (slot 713), a point (slot 237), a part
///   started (slot 941's `+4`) at the point moved by constants; from the
///   count 10, while it runs, slot 258, [`call_231`](crate::s4_small::call_231),
///   [`slot_two_globals`](crate::o4_tiny::slot_two_globals) and
///   [`locked_712`](crate::s4_small::locked_712) (its answer ends it);
///   unless `**G_STOP` the second count one up;
/// - 0: up to the third count 0x50: once the parts at slot 262's point
///   (slot 941's `+8` and `+0xc` programs, the second's place in double
///   sums); while the first runs, it follows bone 14
///   ([`slot237_edx`](crate::o4_tiny::slot237_edx)) less constants and
///   is drawn as above; from 0x3c the second; unless `**G_STOP` the third
///   count one up.
///
/// At the count 0x8c slot 214 `(*G_E)`; the phase one up. Returns 0.
#[allow(clippy::too_many_lines)] // the original's branches, in order
pub fn step_0483<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::add;
    const ONE: u32 = 0x3f80_0000;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let set = |g: &mut G, ops: &[u32], v: u32| {
        for &o in ops {
            g.write::<u32>(w(o), v);
        }
    };
    if rd(g, 0x12) == 0 {
        return Ret::eax(0);
    }
    let ebp = fr.esp.wrapping_sub(4);
    let (pa, pb) = (ebp.wrapping_sub(0x14), ebp.wrapping_sub(0x24));
    // push esi: the body's stack
    let body = fr.esp.wrapping_sub(0x2c);
    let inner = |c: u32, ecx: u32, edx: u32, esp: u32| Frame {
        at: call(c),
        esp,
        ecx,
        edx,
        ..*fr
    };
    // the part starts: `pppStartPart` with (program, 1)
    let start = |g: &mut G, c: u32, buf: u32, program: u32| {
        crate::opu_parts::start_part(g, img, call(c), w(buf), 0x1000, program, 1);
    };
    if rd(g, 0x1f) == 0 {
        let lock = rd(g, 0x28);
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        // call width: 3
        g.call(t(g, 0x37, 711), 0, &[w(0x47), w(0x3c), 0]);
        g.write::<u32>(rd(g, 0x4e), saved);
        // call width: 4
        g.call(
            t(g, 0x60, 710),
            0,
            &[w(0x70), w(0x65), rd(g, 0x59), 0x8_0000],
        );
    }
    let on = g.call(t(g, 0x7a, 253), 0, &[]).eax;
    g.write::<u32>(rd(g, 0x88), u32::from(on == 0));
    g.call(t(g, 0x9a, 723), 0, &[rd(g, 0x8e)]);
    g.call(t(g, 0xa7, 724), 0, &[rd(g, 0xad)]);
    let base = rd(g, 0xbb).wrapping_add(0x8_0400);
    g.write::<u32>(rd(g, 0xc0), base);
    if g.call(t(g, 0xcd, 261), 0, &[2]).eax != 0 {
        if rd(g, 0xe8) == 0 && rd(g, 0xf5) == 0 {
            g.write::<u32>(rd(g, 0x101), w(0x10c));
            g.call(t(g, 0x111, 713), 0, &[w(0x106)]);
            g.write::<u32>(rd(g, 0x11e), w(0x124));
            let tab = g.call(t(g, 0x129, 941), 0, &[rd(g, 0x12f)]).eax;
            start(g, 0x14c, 0x147, g.read::<u32>(tab));
            let p = rd(g, 0x151);
            g.write::<u16>(w(0x15d), 1);
            set(g, &[0x163, 0x16d, 0x177], 0);
            set(g, &[0x181, 0x18b, 0x195], 0x3dcc_cccd);
            set(g, &[0x19f, 0x1a9], ONE);
            g.write::<u32>(p, w(0x1b3));
            let tab = g.call(t(g, 0x1b8, 941), 0, &[rd(g, 0x1be)]).eax;
            start(g, 0x1db, 0x1d6, g.read::<u32>(tab.wrapping_add(0x10)));
            let p = rd(g, 0x1e0);
            g.write::<u16>(w(0x1e7), 1);
            g.write::<u32>(w(0x1ed), ONE);
            g.write::<u32>(p, w(0x1f7));
            let tab = g.call(t(g, 0x1fc, 941), 0, &[rd(g, 0x202)]).eax;
            start(g, 0x21f, 0x21a, g.read::<u32>(tab.wrapping_add(0x10)));
            g.write::<u16>(w(0x229), 1);
            g.write::<u32>(w(0x22f), ONE);
            g.write::<u32>(w(0x239), 1);
        }
        if g.read::<u16>(w(0x240)) != 0 {
            // call width: 4
            g.call(t(g, 0x250, 237), 0, &[rd(g, 0x260), 0x34, 0, pa]);
            // call width: 4
            g.call(t(g, 0x26b, 237), 0, &[rd(g, 0x27b), 0x3a, 0, pb]);
            let c = k(g, 0x28f);
            for (i, o) in (0u32..).zip([0x2a5u32, 0x2bb, 0x2d1]) {
                let s = add(g.read::<f32>(pa + 4 * i), g.read::<f32>(pb + 4 * i));
                g.write::<f32>(w(o), mul(s, c));
            }
            g.call(t(g, 0x294, 258), 0, &[w(0x299)]);
            // call width: 3
            g.call(
                t(g, 0x2e4, 231),
                0,
                &[rd(g, 0x2f6), rd(g, 0x2ea), rd(g, 0x2df)],
            );
            // call width: 2
            g.call(t(g, 0x303, 230), 0, &[rd(g, 0x309), rd(g, 0x2fe)]);
            let lock = rd(g, 0x317);
            let p = rd(g, 0x31c);
            g.write::<u32>(lock, 1);
            g.write::<u32>(p, w(0x328));
            let saved = g.read::<u32>(lock);
            g.write::<u32>(lock, 1);
            // call width: 2
            let done = g.call(t(g, 0x32d, 712), 0, &[w(0x342), 0x30]).eax;
            let lock = rd(g, 0x34a);
            g.write::<u32>(lock, saved);
            if done != 0 {
                g.write::<u16>(w(0x35b), 0);
            }
            g.write::<u32>(lock, 0);
        }
        if rd(g, 0x367).cast_signed() >= 0x3a {
            for (ecx, edx, x, c) in [
                (0x37cu32, 0xcu32, 0x372u32, 0x381u32),
                (0x393, 8, 0x389, 0x398),
            ] {
                let df = Frame {
                    xmm: [0, 0, u128::from(rd(g, x)), 0],
                    ..inner(c, w(ecx), edx, body.wrapping_sub(4))
                };
                crate::m0483::draw(g, img, &df);
            }
        }
        if g.read::<u32>(rd(g, 0x39d)) == 0 {
            let n = rd(g, 0x3ac);
            g.write::<u32>(w(0x3ac), n.wrapping_add(1));
        }
    } else if g.call(t(g, 0x3b6, 261), 0, &[1]).eax != 0 {
        let mut ecx = rd(g, 0x3d1);
        let on = if ecx != 0 {
            Some(g.read::<u16>(w(0x521)))
        } else if rd(g, 0x3df) != 0 {
            None
        } else {
            g.write::<u32>(rd(g, 0x3ea), w(0x3f5));
            g.call(t(g, 0x3fa, 713), 0, &[w(0x3ef)]);
            g.write::<u32>(rd(g, 0x407), w(0x412));
            g.call(t(g, 0x417, 713), 0, &[w(0x40c)]);
            // call width: 4
            g.call(t(g, 0x424, 237), 0, &[rd(g, 0x439), 0x15, 0, w(0x429)]);
            g.write::<u32>(rd(g, 0x440), w(0x446));
            let tab = g.call(t(g, 0x44b, 941), 0, &[rd(g, 0x451)]).eax;
            start(g, 0x46f, 0x46a, g.read::<u32>(tab.wrapping_add(4)));
            ecx = rd(g, 0x485);
            let n = rd(g, 0x493);
            g.write::<u32>(w(0x493), n.wrapping_add(1));
            let x = sub(k(g, 0x477), k(g, 0x47f));
            g.write::<f32>(w(0x49b), x);
            let y = rd(g, 0x4a3);
            g.write::<u32>(w(0x4ab), y);
            let z = sub(k(g, 0x4b3), k(g, 0x4bb));
            g.write::<u16>(w(0x4c1), 1);
            set(g, &[0x4c7, 0x4d1], 0);
            g.write::<u32>(w(0x4db), 0xffff_e000);
            set(g, &[0x4e5, 0x4ef, 0x4f9], 0x3e08_8889);
            g.write::<u32>(w(0x503), ONE);
            g.write::<f32>(w(0x50f), z);
            g.write::<u32>(w(0x515), ONE);
            Some(1)
        };
        if on.is_some_and(|on| on != 0) && ecx.cast_signed() >= 0xa {
            // push G_P: its word under the three calls
            g.call(t(g, 0x530, 258), 0, &[w(0x535)]);
            let esp = body.wrapping_sub(8);
            crate::s4_small::call_231(g, img, &inner(0x542, 0, 0, esp));
            crate::o4_tiny::slot_two_globals(g, img, &inner(0x547, 0, 0, esp));
            g.write::<u32>(rd(g, 0x54c), 1);
            let done = crate::s4_small::locked_712(g, img, &inner(0x55c, 0, w(0x551), esp)).eax;
            if done != 0 {
                g.write::<u16>(w(0x56b), 0);
            }
            let lock = rd(g, 0x570);
            ecx = rd(g, 0x576);
            g.write::<u32>(lock, 0);
        }
        if g.read::<u32>(rd(g, 0x581)) == 0 {
            g.write::<u32>(w(0x591), ecx.wrapping_add(1));
        }
    } else if g.call(t(g, 0x59b, 261), 0, &[0]).eax != 0 {
        let mut ecx = rd(g, 0x5b6);
        if ecx.cast_signed() <= 0x50 {
            if ecx == 0 && rd(g, 0x5cd) == 0 {
                // call width: 4
                g.call(
                    t(g, 0x5d8, 262),
                    0,
                    &[rd(g, 0x5f3), w(0x5ed), w(0x5e8), w(0x5dd)],
                );
                g.write::<u32>(rd(g, 0x5fa), w(0x603));
                // push 0; push 1; push the key: the return address at
                // body - 0x10
                let esp = body.wrapping_sub(0x10);
                g.write::<u32>(esp + 4, rd(g, 0x60d));
                let tab = crate::s4_small::call_941(g, img, &inner(0x612, 0, 0, esp)).eax;
                start(g, 0x628, 0x623, g.read::<u32>(tab.wrapping_add(8)));
                let p = rd(g, 0x62d);
                g.write::<u16>(w(0x63e), 1);
                set(g, &[0x644, 0x64e, 0x658], 0);
                set(g, &[0x662, 0x66c, 0x676], 0x3da3_d70b);
                set(g, &[0x680, 0x68a], ONE);
                g.write::<u32>(p, w(0x694));
                g.write::<u32>(esp + 4, rd(g, 0x69b));
                let tab = crate::s4_small::call_941(g, img, &inner(0x6a0, 0, 0, esp)).eax;
                start(g, 0x6b6, 0x6b1, g.read::<u32>(tab.wrapping_add(0xc)));
                let x = sub(k(g, 0x6be), k(g, 0x6c6));
                ecx = rd(g, 0x6cc);
                set(g, &[0x6d5, 0x6df], 0);
                let x = sub(x, k(g, 0x6eb));
                g.write::<u32>(w(0x6f1), 0x4000);
                set(g, &[0x6fb, 0x705, 0x70f], 0x3d9f_be76);
                g.write::<u32>(w(0x71c), ONE);
                g.write::<u32>(w(0x72e), ONE);
                g.write::<u16>(w(0x739), 1);
                g.write::<u32>(w(0x73f), 1);
                let d = |g: &G, o: u32| g.read::<f64>(w(o));
                // cvtps2pd, three addsd, cvtpd2ps: rounded once
                let xd = f64::from(x) + d(g, 0x728) + d(g, 0x747) + d(g, 0x74f);
                g.write::<f32>(w(0x75b), xd as f32);
                let y = add(add(sub(k(g, 0x763), k(g, 0x76b)), k(g, 0x773)), k(g, 0x77b));
                let yd = f64::from(y) + d(g, 0x786) + d(g, 0x78e);
                g.write::<f32>(w(0x79a), yd as f32);
                let z = sub(k(g, 0x7a2), k(g, 0x7aa));
                let z = add(add(sub(z, k(g, 0x7b2)), k(g, 0x7ba)), k(g, 0x7c2));
                g.write::<f32>(w(0x7d2), sub(z, k(g, 0x7ca)));
            }
            if g.read::<u16>(w(0x7d9)) != 0 {
                // push PT; push ecx: the return address at body - 0xc
                let esp = body.wrapping_sub(0xc);
                g.write::<u32>(esp + 4, ecx);
                g.write::<u32>(esp + 8, w(0x7e5));
                crate::o4_tiny::slot237_edx(g, img, &inner(0x7f0, 0, 0xe, esp));
                for (src, kk, dst) in [
                    (0x7f8u32, 0x800u32, 0x812u32),
                    (0x81a, 0x822, 0x82a),
                    (0x832, 0x83a, 0x842),
                ] {
                    let v = sub(k(g, src), k(g, kk));
                    g.write::<f32>(w(dst), v);
                }
                g.call(t(g, 0x805, 258), 0, &[w(0x80a)]);
                // the bone's word under the three calls
                let esp = body.wrapping_sub(8);
                crate::s4_small::call_231(g, img, &inner(0x852, 0, 0, esp));
                crate::o4_tiny::slot_two_globals(g, img, &inner(0x857, 0, 0, esp));
                g.write::<u32>(rd(g, 0x85c), 1);
                let done = crate::s4_small::locked_712(g, img, &inner(0x86c, 0, w(0x861), esp)).eax;
                if done != 0 {
                    g.write::<u16>(w(0x87b), 0);
                }
                let lock = rd(g, 0x880);
                ecx = rd(g, 0x886);
                g.write::<u32>(lock, 0);
            }
            if ecx.cast_signed() >= 0x3c && g.read::<u16>(w(0x898)) != 0 {
                g.call(t(g, 0x8a0, 258), 0, &[w(0x8a5)]);
                let esp = body.wrapping_sub(8);
                crate::s4_small::call_231(g, img, &inner(0x8b2, 0, 0, esp));
                crate::o4_tiny::slot_two_globals(g, img, &inner(0x8b7, 0, 0, esp));
                g.write::<u32>(rd(g, 0x8bc), 0);
                let done = crate::s4_small::locked_712(g, img, &inner(0x8cc, 0, w(0x8c1), esp)).eax;
                if done != 0 {
                    g.write::<u16>(w(0x8db), 0);
                }
                let lock = rd(g, 0x8e0);
                ecx = rd(g, 0x8e6);
                g.write::<u32>(lock, 0);
            }
        }
        if g.read::<u32>(rd(g, 0x8f1)) == 0 {
            g.write::<u32>(w(0x8fd), ecx.wrapping_add(1));
        }
    }
    if rd(g, 0x903) == 0x8c {
        g.call(t(g, 0x90f, 214), 0, &[rd(g, 0x915)]);
    }
    let n = rd(g, 0x926);
    g.write::<u32>(w(0x926), n.wrapping_add(1));
    Ret::eax(0)
}

/// [`step_0483`] (2362 bytes, 1 overlay).
pub const STEP_0483: Sig = Sig {
    len: 2362,
    hash: 0xbbdb_78e5_36c4_4a3c,
    calls: &[
        (0x14c, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x1db, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x21f, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x381, Callee::Sig(&crate::m0483::DRAW)),
        (0x398, Callee::Sig(&crate::m0483::DRAW)),
        (0x46f, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x542, Callee::Sig(&crate::s4_small::CALL_231)),
        (
            0x547,
            Callee::Sig(&crate::o4_tiny::SLOT_TWO_GLOBALS_0483_2650),
        ),
        (0x55c, Callee::Sig(&crate::s4_small::LOCKED_712)),
        (0x612, Callee::Sig(&crate::s4_small::CALL_941)),
        (0x628, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x6a0, Callee::Sig(&crate::s4_small::CALL_941)),
        (0x6b6, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x7f0, Callee::Sig(&crate::o4_tiny::SLOT237_EDX_0483_26A0)),
        (0x852, Callee::Sig(&crate::s4_small::CALL_231)),
        (
            0x857,
            Callee::Sig(&crate::o4_tiny::SLOT_TWO_GLOBALS_0483_2650),
        ),
        (0x86c, Callee::Sig(&crate::s4_small::LOCKED_712)),
        (0x8b2, Callee::Sig(&crate::s4_small::CALL_231)),
        (
            0x8b7,
            Callee::Sig(&crate::o4_tiny::SLOT_TWO_GLOBALS_0483_2650),
        ),
        (0x8cc, Callee::Sig(&crate::s4_small::LOCKED_712)),
        // __security_check_cookie
        (0x932, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0491`'s flight start (0x10003c50, 1346 bytes; internal, `ecx`
/// = the owner `W`; a 16-aligned frame through `ebx`: `ebp = (E - 0xc) &
/// !0xf`, the points `A` at `ebp - 0x14` and `B` at `ebp - 0x24`, the
/// offset `Q` at `ebp - 0x34`, the matrix `M` at `ebp - 0x80`). `A` =
/// bone 0x11 of the actor `S` (`*G_S`, or `ACTORS[MAP[W[0x504]]]`; slots
/// 262, 268; `w` 1.0), `B` = the same bone of `*G_T`. The object `W`
/// starts ([`start_object_anim_2`](crate::s6_misc::start_object_anim_2)
/// `(0, -, 0x10, CB)`, scale `K`) and takes `A` at `+0xd0`; its yaw `+0xc4`
/// = `K' - slot 333 (ACTORS[*MAP])` wrapped once, `+0x120` the actor's
/// number; the record `W + 0x488` (cleared, its two DLL words) is `W +
/// 0x110`. The flight is a cubic: `P0 = A` (`+0x4bc`), `P1` = `A + D +
/// Q` (`+0x4cc`, its `y` at most slot 12's ground less `K2`; `D = (B - A)
/// / K1`), `P2 = A + D K3 - Q` (`+0x4dc`), `P3 = B` (`+0x4ec`), where `Q`
/// = slot 314 `(40)`, slot 404 `(20)`, 0 turned by the yaw `K4 -
/// atan2(dz, dx)` (wrapped once; slots 235, 329, 229). `+0x4fc` 0, the
/// rate `+0x500` 1/12, the counts `+0x508` 12, `+0x510` 16; the trail's
/// sprites `W + 0x260` coloured by `A`
/// ([`sprite_init_colour`](crate::sb_misc::sprite_init_colour)), `+0x484`
/// 0x80, `+0x47c` 0.5, `+0x46c` 0x40404040. Returns the sprites' `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn flight_start_0491<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let o = fr.ecx;
    let ebp = fr.esp.wrapping_sub(0xc) & !0xf;
    let (a, b, q, m) = (
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x34),
        ebp.wrapping_sub(0x80),
    );
    let body = ebp.wrapping_sub(0xa0);
    // the actor of `W[0x504]`: -1 the global at `sole`, else through
    // the map (`map`) and the table (`table`)
    let actor = |g: &G, sole: u32, map: u32, table: u32| {
        let i = g.read::<u32>(o + 0x504);
        if i == u32::MAX {
            rd(g, sole)
        } else {
            let n = u32::from(g.read::<u8>(rd(g, map).wrapping_add(i)));
            g.read::<u32>(w(table).wrapping_add(4 * n))
        }
    };
    let s = actor(g, 0x37, 0x3e, 0x49);
    // call width: 4
    g.call(t(g, 0x5a, 262), 0, &[s, a, a + 4, a + 8]);
    // call width: 3
    g.call(t(g, 0x6c, 268), 0, &[s, 0x11, a]);
    let s = rd(g, 0x7d);
    g.write::<u32>(a + 0xc, 0x3f80_0000);
    // call width: 4
    g.call(t(g, 0x91, 262), 0, &[s, b, b + 4, b + 8]);
    // call width: 3
    g.call(t(g, 0xaa, 268), 0, &[s, 0x11, b]);
    // push CB; push 0x10; push ecx; push 0: the return address at
    // body - 0x14 (the third word is the `ecx` slot 268 leaves: the start
    // does not read it)
    let esp = body.wrapping_sub(0x14);
    g.write::<u32>(esp + 4, 0);
    g.write::<u32>(esp + 0xc, 0x10);
    g.write::<u32>(esp + 0x10, w(0xc7));
    g.write::<u32>(b + 0xc, 0x3f80_0000);
    let sf = Frame {
        at: img.rel32_target(f + 0xd8),
        esp,
        ecx: 0,
        edx: o,
        xmm: [0, 0, 0, u128::from(rd(g, 0xbd))],
        ..*fr
    };
    crate::s6_misc::start_object_anim_2(g, img, &sf);
    for i in 0..4u32 {
        let x = g.read::<u32>(a + 4 * i);
        g.write::<u32>(o + 0xd0 + 4 * i, x);
    }
    let n = u32::from(g.read::<u8>(rd(g, 0x111)));
    let who = g.read::<u32>(w(0x11b).wrapping_add(4 * n));
    let r = st0_f32(g.call(t(g, 0x120, 333), 0, &[who]).st0);
    let yaw = sub(k(g, 0x136), r);
    let yaw = crate::r20::wrap_once(g, img, f, yaw, (0x150, 0x146, 0x160, 0x146));
    g.write::<f32>(o + 0xc4, yaw);
    let i = g.read::<u32>(o + 0x504);
    let num = if i == u32::MAX {
        rd(g, 0x181)
    } else {
        u32::from(g.read::<u8>(rd(g, 0x188).wrapping_add(i)))
    };
    let at = |g: &G, p: u32, j: u32| g.read::<f32>(p + 4 * j);
    let c = k(g, 0x19e);
    let dx = div(sub(at(g, b, 0), at(g, a, 0)), c);
    let dy = div(sub(at(g, b, 1), at(g, a, 1)), c);
    let dz = div(sub(at(g, b, 2), at(g, a, 2)), c);
    g.write::<u32>(o + 0x120, num);
    let rec = o + 0x488;
    for (off, x) in [
        (0x18u32, 0u32),
        (0x14, 0),
        (0x10, 0),
        (0x1c, 0),
        (0x20, w(0x1f0)),
        (0x24, w(0x1fb)),
        (0x28, 0),
        (0x2c, 0),
    ] {
        g.write::<u32>(rec + off, x);
    }
    g.write::<u16>(rec + 0x30, 0);
    g.write::<u32>(o + 0x110, rec);
    for (dst, j) in [(0x4c4u32, 2u32), (0x4bc, 0), (0x4c0, 1), (0x4c8, 3)] {
        let x = g.read::<u32>(a + 4 * j);
        g.write::<u32>(o + dst, x);
    }
    let q0 = st0_f32(g.call(t(g, 0x22f, 314), 0, &[0x4220_0000]).st0);
    g.write::<f32>(q, q0);
    let q1 = st0_f32(g.call(t(g, 0x271, 404), 0, &[0x41a0_0000]).st0);
    g.write::<f32>(q + 4, q1);
    g.write::<u32>(q + 8, 0);
    g.write::<u32>(q + 0xc, 0x3f80_0000);
    // `_CIatan2` on x87 (the two differences as doubles), rounded once
    #[allow(clippy::cast_possible_truncation)] // fstp qword; cvtpd2ps
    let ang = g.atan2(
        f64::from(sub(at(g, b, 2), at(g, a, 2))),
        f64::from(sub(at(g, b, 0), at(g, a, 0))),
    ) as f32;
    let yaw = sub(k(g, 0x2e8), ang);
    let yaw = crate::r20::wrap_once(g, img, f, yaw, (0x2f7, 0x309, 0x313, 0x320));
    g.call(t(g, 0x331, 235), 0, &[m]);
    // call width: 3
    g.call(t(g, 0x350, 329), 0, &[m, m, yaw.to_bits()]);
    // call width: 3
    g.call(t(g, 0x36c, 229), 0, &[q, m, q]);
    g.write::<u32>(o + 0x4d8, 0x3f80_0000);
    for (j, d, dst) in [(0u32, dx, 0x4ccu32), (1, dy, 0x4d0), (2, dz, 0x4d4)] {
        let v = add(add(at(g, a, j), d), at(g, q, j));
        g.write::<f32>(o + dst, v);
    }
    let h = actor(g, 0x3df, 0x3e6, 0x3f1);
    let ground = st0_f32(g.call(t(g, 0x3f7, 12), 0, &[h]).st0);
    // comiss; jbe: an unordered height stays
    if g.read::<f32>(o + 0x4d0) > ground {
        g.write::<f32>(o + 0x4d0, sub(ground, k(g, 0x422)));
    }
    let s3 = k(g, 0x432);
    g.write::<u32>(o + 0x4e8, 0x3f80_0000);
    for (j, d, dst) in [(0u32, dx, 0x4dcu32), (1, dy, 0x4e0), (2, dz, 0x4e4)] {
        let v = sub(add(mul(d, s3), at(g, a, j)), at(g, q, j));
        g.write::<f32>(o + dst, v);
    }
    for j in 0..4u32 {
        let x = g.read::<u32>(b + 4 * j);
        g.write::<u32>(o + 0x4ec + 4 * j, x);
    }
    g.write::<u32>(o + 0x4fc, 0);
    g.write::<u32>(o + 0x500, 0x3daa_aaab);
    g.write::<u32>(o + 0x508, 0xc);
    g.write::<u32>(o + 0x510, 0x10);
    let owner = g.read::<u32>(rd(g, 0x502).wrapping_add(0x10));
    // push A: the return address at body - 8
    let esp = body.wrapping_sub(8);
    g.write::<u32>(esp + 4, a);
    let eax = crate::sb_misc::sprite_init_colour(g, esp, o + 0x260, owner);
    g.write::<u32>(o + 0x484, 0x80);
    g.write::<u32>(o + 0x47c, 0x3f00_0000);
    g.write::<u32>(o + 0x46c, 0x4040_4040);
    Ret::eax(eax)
}

/// [`flight_start_0491`] (1346 bytes, 1 overlay).
pub const FLIGHT_START_0491: Sig = Sig {
    len: 1346,
    hash: 0x78fe_02db_9b82_c640,
    calls: &[
        (0xd8, Callee::Sig(&crate::s6_misc::START_OBJECT_ANIM_2)),
        (0x2d2, Callee::Import("_CIatan2")),
        (0x50a, Callee::Sig(&crate::sb_misc::SPRITE_INIT_COLOUR)),
        // __security_check_cookie
        (0x537, Callee::Any),
    ],
    imports: &[],
};

/// One lane of [`flight_step_0491`]'s cubic: `(1 - t)^3 p0 + 3 (1 - t)^2
/// t p1 + 3 (1 - t) t^2 p2 + t^3 p3` in the original's order (`one` and
/// `three` its constants).
fn cubic(t: f32, one: f32, three: f32, [p0, p1, p2, p3]: [f32; 4]) -> f32 {
    use crate::sb_sse::add;
    let u = sub(one, t);
    let t2 = mul(t, t);
    let mid = mul(mul(mul(mul(u, t), u), three), p1);
    let first = mul(mul(mul(u, u), u), p0);
    let s = add(mid, first);
    let third = mul(mul(mul(t2, u), three), p2);
    let last = mul(mul(t2, t), p3);
    add(add(s, third), last)
}

/// `magic_0491`'s flight step (0x100044f0, 972 bytes; a task function
/// (its address pushed at 0x100036f1), cdecl `(task)`, the owner `W =
/// task[0x14]`; a 16-aligned frame: `B = ((E - 4) & !0xf) - 0x70`, the
/// point `P` at `B + 0x10`, the matrix at `B + 0x20`). While the wait
/// `W + 0x50c` runs it counts down (at 0 the flight starts:
/// [`flight_start_0491`]); then the wait `+0x510`. Then the time `+0x4fc`
/// steps by `+0x500` (at most 1.0) and `P` is the cubic of `+0x4bc..`
/// (`w` 1.0). Without sparks left (`+0x508` 0) the trail fades (`+0x484`
/// halved while above 0) or the task is given back
/// ([`give_back`](crate::sa_pool::give_back)); else, counting them down
/// (at 0 the burst, [`burst_0491`](crate::r20::burst_0491)), six pool
/// records ([`pool_take`](crate::sb_misc::pool_take); none left ends it)
/// get slot 719 `(PART, 9, 1)`, `P` (slot 325) and a random direction
/// (three slot 307 numbers less `K`, scaled by `K'`). The trail drawn at
/// `P` ([`trail_draw`](crate::s4_misc::trail_draw), the matrix slot 235's
/// with 0.03 on its diagonal). Returns what the last step returns.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn flight_step_0491<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let task = fr.arg(g, 0);
    let o = g.read::<u32>(task + 0x14);
    let b = (fr.esp.wrapping_sub(4) & !0xf).wrapping_sub(0x70);
    let (p, m) = (b + 0x10, b + 0x20);
    let c = g.read::<i32>(o + 0x50c);
    if c > 0 {
        let c = c - 1;
        g.write::<u32>(o + 0x50c, c.cast_unsigned());
        if c != 0 {
            return Ret::eax(c.cast_unsigned());
        }
        let sf = Frame {
            at: call(0x3a),
            esp: b.wrapping_sub(4),
            ecx: o,
            ..*fr
        };
        return flight_start_0491(g, img, &sf);
    }
    let d = g.read::<i32>(o + 0x510);
    if d > 0 {
        let d = d - 1;
        g.write::<u32>(o + 0x510, d.cast_unsigned());
        return Ret::eax(d.cast_unsigned());
    }
    let one = k(g, 0x85);
    let u = crate::sb_sse::add(g.read::<f32>(o + 0x500), g.read::<f32>(o + 0x4fc));
    g.write::<f32>(o + 0x4fc, u);
    // comiss; jbe: an unordered time stays
    if u > one {
        g.write::<u32>(o + 0x4fc, 0x3f80_0000);
    }
    let three = k(g, 0xac);
    for j in 0..3u32 {
        let tt = g.read::<f32>(o + 0x4fc);
        let pts = [0x4bcu32, 0x4cc, 0x4dc, 0x4ec].map(|q| g.read::<f32>(o + q + 4 * j));
        g.write::<f32>(p + 4 * j, cubic(tt, one, three, pts));
    }
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    let e = g.read::<u32>(o + 0x508);
    if e == 0 {
        let fade = g.read::<i32>(o + 0x484);
        if fade <= 0 {
            let eax = crate::sa_pool::give_back(g, img, call(0x225), task);
            return Ret::eax(eax);
        }
        g.write::<u32>(o + 0x484, (fade / 2).cast_unsigned());
    } else {
        let e = e.wrapping_sub(1);
        g.write::<u32>(o + 0x508, e);
        if e == 0 {
            let bf = Frame {
                at: call(0x246),
                esp: b.wrapping_sub(4),
                ecx: o,
                ..*fr
            };
            crate::r20::burst_0491(g, img, &bf);
        }
        let take = call(0x251);
        let random = |g: &mut G, o: u32| st0_f32(g.call(t(g, o, 307), 0, &[]).st0);
        for _ in 0..6 {
            let r = crate::sb_misc::pool_take(g, img, take);
            if r == 0 {
                break;
            }
            // call width: 3
            let part = g.call(t(g, 0x261, 719), 0, &[w(0x270), 9, 1]).eax;
            g.write::<u32>(r + 0x30, part);
            g.write::<u16>(r + 0x38, 0xffff);
            g.write::<u16>(r + 0x3a, 0x200);
            g.write::<u32>(r + 0x34, 1);
            // call width: 2
            g.call(t(g, 0x292, 325), 0, &[r, p]);
            for (j, (ro, ko, so)) in [
                (0x2a7u32, 0x2c4u32, 0x2d1u32),
                (0x2c9, 0x2f0, 0x2fd),
                (0x2f5, 0x31c, 0x33a),
            ]
            .into_iter()
            .enumerate()
            {
                let v = sub(random(g, ro), k(g, ko));
                if j == 2 {
                    for off in [0x1cu32, 0x20, 0x24, 0x28] {
                        g.write::<u32>(r + off, 0);
                    }
                    let v = mul(v, k(g, so));
                    g.write::<u32>(r + 0x2c, 0x3f80_0000);
                    g.write::<f32>(r + 0x18, v);
                } else {
                    g.write::<f32>(r + 0x10 + 4 * j as u32, mul(v, k(g, so)));
                }
            }
            g.write::<u32>(r + 0x3c, 0x3c75_c28f);
            g.write::<u32>(r + 0x4c, 0x20);
            g.write::<u32>(r + 0x50, 0x20);
            g.write::<u8>(r + 0x6b, 0);
        }
    }
    g.call(t(g, 0x37d, 235), 0, &[m]);
    for off in [0x48u32, 0x34, 0x20] {
        g.write::<u32>(b + off, 0x3cf5_c28f);
    }
    // push P: the return address at B - 8
    let esp = b.wrapping_sub(8);
    g.write::<u32>(esp + 4, p);
    let tf = Frame {
        at: call(0x3b4),
        esp,
        ecx: o + 0x260,
        edx: m,
        ..*fr
    };
    Ret::eax(crate::s4_misc::trail_draw(g, img, &tf))
}

/// [`flight_step_0491`] (972 bytes, 1 overlay).
pub const FLIGHT_STEP_0491: Sig = Sig {
    len: 972,
    hash: 0xa2cd_e11d_a5b8_24b1,
    calls: &[
        (0x3a, Callee::Sig(&FLIGHT_START_0491)),
        (0x225, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x246, Callee::Sig(&crate::r20::BURST_0491)),
        (0x251, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x3b4, Callee::Sig(&crate::s4_misc::TRAIL_DRAW_0009)),
        // __security_check_cookie
        (0x47, Callee::Any),
        (0x69, Callee::Any),
        (0x232, Callee::Any),
        (0x3c4, Callee::Any),
    ],
    imports: &[],
};

/// `child` appended last to the children of `parent` (the parent's
/// `+0x10` last, `+0x18` count): `+0x4` the parent, `+0x8` the old last,
/// the old last's `+0x4` the child (the list is never empty here: the
/// parent's own `+0x10` stands in), `+0x14` the parent.
fn child_last_0601<G: Game>(g: &mut G, parent: u32, child: u32) {
    g.write::<u32>(child + 4, parent);
    let last = g.read::<u32>(parent + 0x10);
    g.write::<u32>(child + 8, last);
    let last = g.read::<u32>(parent + 0x10);
    g.write::<u32>(last + 4, child);
    g.write::<u32>(parent + 0x10, child);
    g.write::<u32>(child + 0x14, parent);
    let n = g.read::<u16>(parent + 0x18);
    g.write::<u16>(parent + 0x18, n.wrapping_add(1));
}

/// `magic_0601`'s bone build (0x100035f0, 604 bytes; internal, `ecx` =
/// the record `R = [N, parent, P, -, -, -, a, V, b, W]`). The node `N`
/// gets three children with empty child lists: `C = N + 0x1d4` and `B =
/// N + 0x1a8` (vtables first), `A = N + 0x17c` (no parent; vtable after),
/// whose addresses go to `R + 0xc`, `+0x10` (`C`), `+0x14` (`B`). `N`
/// joins `parent` (when there is one; [`attach`](crate::l3_common::attach)
/// of a list object), `+0x128` = `P`, byte `+0x148` 0, its three objects
/// `+0x13c` `A`, `+0x144` `C`, `+0x140` `B`; then `C`, `B`, `A` are
/// appended to `N`'s own children in that order; `+0x15c` = `V`, `+0x164`
/// = `a`, `+0x160` = `W`, `+0x168` = `b`; its words cleared, `+0x1fc` 1,
/// the scales 1.0, byte `+0x178` 0; slot 281 `(1)`, the handle `+0x170`
/// = slot 287 `(1, 0, 7)`, slot 283 `(h, 0)` and `(h, 1)`; the DLL's
/// vtable into `N` last. Returns `N`.
pub fn bone_build_0601<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let r = fr.ecx;
    let n = g.read::<u32>(r);
    let (a, b, c) = (n + 0x17c, n + 0x1a8, n + 0x1d4);
    g.write::<u32>(c, w(0x14));
    g.write::<u16>(c + 0x1a, 0xff);
    g.write::<u32>(c + 0x14, 0);
    g.write::<u32>(r + 0xc, a);
    g.write::<u32>(c + 0x10, c);
    g.write::<u32>(c + 0xc, c);
    g.write::<u16>(b + 0x1a, 0xff);
    g.write::<u32>(b + 0x14, 0);
    g.write::<u32>(b, w(0x5f));
    g.write::<u32>(b + 0x10, b);
    g.write::<u32>(b + 0xc, b);
    g.write::<u32>(r + 0x10, c);
    g.write::<u16>(a + 0x1a, 0xff);
    g.write::<u32>(a + 0x10, a);
    g.write::<u32>(a + 0xc, a);
    g.write::<u32>(a + 0x14, 0);
    g.write::<u32>(a, w(0x93));
    g.write::<u32>(r + 0x14, b);
    let parent = g.read::<u32>(r + 4);
    g.write::<u16>(n + 0x1a, 0xff);
    g.write::<u32>(n + 0x10, n);
    g.write::<u32>(n + 0xc, n);
    g.write::<u32>(n + 0x14, parent);
    if parent != 0 {
        crate::l3_common::attach(g, n, parent, crate::l3_common::ListAt::of(parent));
    }
    let p = g.read::<u32>(r + 8);
    g.write::<u32>(n + 0x128, p);
    g.write::<u8>(n + 0x148, 0);
    g.write::<u32>(n + 0x13c, a);
    g.write::<u32>(n + 0x144, c);
    g.write::<u32>(n + 0x140, b);
    child_last_0601(g, n, c);
    child_last_0601(g, n, b);
    // the last child's links before the record's word is read
    g.write::<u32>(a + 4, n);
    let last = g.read::<u32>(n + 0x10);
    g.write::<u32>(a + 8, last);
    let last = g.read::<u32>(n + 0x10);
    g.write::<u32>(last + 4, a);
    let v = g.read::<u32>(r + 0x1c);
    g.write::<u32>(n + 0x10, a);
    g.write::<u32>(a + 0x14, n);
    let k = g.read::<u16>(n + 0x18);
    g.write::<u16>(n + 0x18, k.wrapping_add(1));
    g.write::<u32>(n + 0x15c, v);
    for (dst, src) in [(0x164u32, 0x18u32), (0x160, 0x24), (0x168, 0x20)] {
        let x = g.read::<u32>(r + src);
        g.write::<u32>(n + dst, x);
    }
    for (off, x) in [
        (0x1f8u32, 0u32),
        (0x1f4, 0),
        (0x1f0, 0),
        (0x1fc, 1),
        (0x1a0, 0),
        (0x19c, 0),
        (0x198, 0),
        (0x1a4, 0x3f80_0000),
        (0x1d0, 0x3f80_0000),
        (0x1cc, 0x3f80_0000),
        (0x1c8, 0x3f80_0000),
        (0x1c4, 0x3f80_0000),
    ] {
        g.write::<u32>(n + off, x);
    }
    g.write::<u8>(n + 0x178, 0);
    g.call(t(g, 0x17b, 281), 0, &[1]);
    // call width: 3
    let h = g.call(t(g, 0x207, 287), 0, &[1, 0, 7]).eax;
    g.write::<u32>(n + 0x170, h);
    // call width: 2
    g.call(t(g, 0x229, 283), 0, &[h, 0]);
    // call width: 2
    g.call(t(g, 0x236, 283), 0, &[g.read::<u32>(n + 0x170), 1]);
    let n = g.read::<u32>(r);
    g.write::<u32>(n, w(0x251));
    Ret::eax(n)
}

/// [`bone_build_0601`] (604 bytes, 1 overlay).
pub const BONE_BUILD_0601: Sig = Sig {
    len: 604,
    hash: 0xa42b_4008_8da5_e2c9,
    calls: &[],
    imports: &[],
};

/// `magic_0601`'s node build (0x10007f80, 82 bytes; internal, `ecx` the
/// node, the stack words 4 and 6 `a`, `b`): `m0152::build_node`'s
/// record `[node, V1, V2, 0, 0, 0, a, V3, b, V2]` in the frame (`esp -
/// 0x2c`) for [`bone_build_0601`]. Returns the node.
pub fn node_build_0601<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let (a, b) = (fr.arg(g, 4), fr.arg(g, 6));
    let rec = fr.esp.wrapping_sub(0x2c);
    let words = [fr.ecx, w(0x18), w(0x1f), 0, 0, 0, a, w(0x3b), b, w(0x45)];
    for (i, v) in (0u32..).zip(words) {
        g.write::<u32>(rec + 4 * i, v);
    }
    let bf = Frame {
        at: img.rel32_target(f + 0x4a),
        esp: fr.esp.wrapping_sub(0x30),
        ecx: rec,
        ..*fr
    };
    bone_build_0601(g, img, &bf)
}

/// [`node_build_0601`] (82 bytes, 1 overlay).
pub const NODE_BUILD_0601: Sig = Sig {
    len: 82,
    hash: 0x6c70_0d28_44d4_8098,
    calls: &[(0x4a, Callee::Sig(&BONE_BUILD_0601))],
    imports: &[],
};

/// A task node taken off `magic_0610`'s free ring and appended to the
/// task list (inlined, the pool's take: the free count `u16` at `count`
/// (read) and `count_w` (written), the free head at `free`, `free_w`,
/// `free_0`; the node's callback the DLL word at `cb`, its byte `+0x18`
/// `flag`; the list's head at `head`, tail at `tail`, `head_w`,
/// `tail_w`). With no free node the original writes through address 0,
/// as this does.
fn task_node_0610<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    (count, free, free_w, free_0, cb, count_w): (u32, u32, u32, u32, u32, u32),
    (head, tail, head_w, tail_w): (u32, u32, u32, u32),
    flag: u8,
) -> u32 {
    let w = |o: u32| img.word(f + o);
    let n = g.read::<u16>(w(count));
    // no free node: the original skips the take's stores (`xor edx, edx`)
    let node = if n == 0 {
        0
    } else {
        let node = g.read::<u32>(w(free));
        let (next, prev) = (g.read::<u32>(node), g.read::<u32>(node + 4));
        g.write::<u32>(next + 4, prev);
        let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node));
        g.write::<u32>(prev, next);
        if n > 1 {
            let p = g.read::<u32>(node + 4);
            g.write::<u32>(w(free_w), p);
        } else {
            g.write::<u32>(w(free_0), 0);
        }
        for off in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node + off, 0);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 0x10, w(cb));
        g.write::<u8>(node + 0x18, 0);
        g.write::<u16>(w(count_w), n - 1);
        node
    };
    let listed = g.read::<u32>(w(head)) != 0;
    g.write::<u8>(node.wrapping_add(0x18), flag);
    g.write::<u32>(node.wrapping_add(4), 0);
    if listed {
        let last = g.read::<u32>(w(tail));
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(head_w), node);
    }
    g.write::<u32>(node.wrapping_add(8), node);
    g.write::<u32>(w(tail_w), node);
    node
}

/// `magic_0610`'s start-up (0x1000d6c0, 943 bytes; internal, no
/// arguments, no frame; called by its load step 0x10007180). Two words
/// and five pointers set, the byte history `H[1..=n]` = `H[0..n]` (`n =
/// *G_N`, one up after); slot 719 `(PART, 8, 0)` into `G_H`, slots 711
/// `(h, BUF, 0)` (the game lock held) and 710 `(OUT, h, DST, 0x40000)`;
/// four words 0; the caster placed
/// ([`caster_place_0588`](crate::r18::caster_place_0588)); 100 records
/// (0x140 apart) cleared and their counts; the effect reset
/// ([`effect_reset`](crate::sb_misc::effect_reset)); the matrix `M` =
/// slot 235 turned by the effect's `+0x1a68` (slot 329) and its last row
/// the effect's `+0xc70..`; two node heads; slot 271 `(4)`; two task
/// nodes (bytes 1 and 2) with the records set up between them
/// ([`records_init`](crate::sa_misc::records_init)); when slot 520 says
/// 0x1ad0001 a global steps by `K`. Returns 0.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn start_up_0610<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r11_fades::stores;
    use crate::sb_sse::add;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let rd = |g: &G, o: u32| g.read::<u32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let call = |o: u32| img.rel32_target(f + o);
    let n = rd(g, 0x1);
    g.write::<u32>(w(0x7), 0);
    stores(g, img, f, &[0x12, 0x1c, 0x26, 0x30, 0x3a]);
    let mut i = n.cast_signed().wrapping_sub(1);
    if i >= 0 {
        loop {
            i = i.wrapping_sub(1);
            let b = g.read::<u8>(w(0x53).wrapping_add(i.cast_unsigned()));
            g.write::<u8>(w(0x59).wrapping_add(i.cast_unsigned()), b);
            if i < 0 {
                break;
            }
        }
    }
    let n = rd(g, 0x60).wrapping_add(1);
    g.write::<u32>(w(0x68), n);
    // call width: 3
    let h = g.call(t(g, 0x6d, 719), 0, &[w(0x7c), 8, 0]).eax;
    g.write::<u32>(w(0x8b), h);
    let lock = rd(g, 0x84);
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    // call width: 3
    g.call(t(g, 0x98, 711), 0, &[h, w(0x92), 0]);
    g.write::<u32>(rd(g, 0xab), saved);
    // call width: 4
    g.call(
        t(g, 0xb7, 710),
        0,
        &[w(0xcd), rd(g, 0xc2), w(0xbc), 0x4_0000],
    );
    for o in [0xd8u32, 0xe2, 0xec, 0xf6] {
        g.write::<u32>(w(o), 0);
    }
    // push esi; push edi: the return address at E - 0xc
    let esp = fr.esp.wrapping_sub(0xc);
    let cf = Frame {
        at: call(0xff),
        esp,
        ..*fr
    };
    crate::r18::caster_place_0588(g, img, &cf);
    stores(g, img, f, &[0x103]);
    let mut rec = w(0x10e);
    for _ in 0..0x64 {
        g.write::<u32>(rec, 0);
        rec = rec.wrapping_add(0x140);
    }
    let e = rd(g, 0x131);
    g.write::<u32>(w(0x137), 0);
    g.write::<u32>(w(0x13d), 0x64);
    crate::sb_misc::effect_reset(g, img, call(0x146));
    g.write::<u8>(w(0x151), 1);
    g.call(t(g, 0x14b, 235), 0, &[w(0x15d)]);
    let a = g.read::<u32>(e.wrapping_add(0x1a68));
    // call width: 3
    g.call(t(g, 0x16c, 329), 0, &[w(0x181), w(0x17c), a]);
    for (j, o) in (0u32..).zip([0x198u32, 0x1a8, 0x1b8, 0x1c8]) {
        let x = g.read::<u32>(e.wrapping_add(0xc70 + 4 * j));
        g.write::<u32>(w(o), x);
    }
    stores(g, img, f, &[0x1cc]);
    g.write::<u32>(w(0x1d8), 0);
    g.write::<u16>(w(0x1e3), 0);
    g.write::<u8>(w(0x1eb), 1);
    stores(g, img, f, &[0x1f0]);
    g.write::<u32>(w(0x1fc), 0);
    g.write::<u16>(w(0x207), 0);
    g.write::<u8>(w(0x20f), 1);
    g.call(t(g, 0x190, 271), 0, &[4]);
    task_node_0610(
        g,
        img,
        f,
        (0x221, 0x238, 0x255, 0x25d, 0x289, 0x294),
        (0x29a, 0x2ad, 0x2c0, 0x2ce),
        1,
    );
    crate::sa_misc::records_init(g, img, call(0x2d3), w(0x2c5));
    task_node_0610(
        g,
        img,
        f,
        (0x2da, 0x2e9, 0x306, 0x30e, 0x33a, 0x345),
        (0x34b, 0x360, 0x373, 0x387),
        2,
    );
    if g.call(t(g, 0x378, 520), 0, &[]).eax == 0x01ad_0001 {
        let v = add(g.read::<f32>(w(0x398)), g.read::<f32>(w(0x3a0)));
        g.write::<f32>(w(0x3a8), v);
    }
    Ret::eax(0)
}

/// [`start_up_0610`] (943 bytes, 1 overlay).
pub const START_UP_0610: Sig = Sig {
    len: 943,
    hash: 0x9d18_766b_cb8e_5a85,
    calls: &[
        (0xff, Callee::Sig(&crate::r18::CASTER_PLACE_0610)),
        (0x146, Callee::Sig(&crate::sb_misc::EFFECT_RESET)),
        (0x2d3, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
    ],
    imports: &[],
};

/// `magic_0610`'s load step (0x10007180, 176 bytes; a table entry):
/// [`cue_start`](crate::m0591::cue_start)'s copy that runs
/// [`start_up_0610`] where the others set up their targets.
pub fn cue_start_0610<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::m0591::cue_start_with(g, img, fr, |g, img, fr| {
        start_up_0610(g, img, fr);
    })
}

/// [`cue_start_0610`] (176 bytes, 1 overlay).
pub const CUE_START_0610: Sig = Sig {
    len: 176,
    hash: 0xa0ea_b140_6d8d_9c5e,
    calls: &[(0x95, Callee::Sig(&START_UP_0610))],
    imports: &[],
};
