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
