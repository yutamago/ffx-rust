//! Round 33's own ports (own rows of `magic_0450` and up): the DLLs' own
//! code written out, and copies bound by their own `Sig`.

use crate::ob_model::st0_f32;
use crate::pushes::Pushes;
use crate::r31::{append_0601, append_pair_0601};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

const ONE: u32 = 0x3f80_0000;
/// `magic_0609`'s summon task (0x10009c80, 2719 bytes; a task callback
/// `(A)`, cdecl; an aligned /GS frame `E` below three pushes): as
/// [`summon_task_0613`](crate::r31::summon_task_0613) with the block 0x10 further on and models 0x278
/// apart, these differences: 0x190 sets its words without asking slot 520
/// (the turn speeds `0 K K'`), the start passes the summon set-up
/// ([`summon_setup_0609`](crate::r31::summon_setup_0609)) the height `-112` and links the effect's object
/// ([`link_object`](crate::m0609::link_object)) after it, and 0x2bc stores
/// fixed camera words after slot 229. Returns `eax` as the original
/// leaves it.
#[allow(clippy::too_many_lines)] // the original's states, one by one
pub fn summon_task_0609<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::r25::abs_stores;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x74; three pushes
    let a0 = fr.esp.wrapping_sub(4) & !7;
    let cookie = g.read::<u32>(w(0xa)) ^ a0.wrapping_sub(0x74);
    g.write::<u32>(a0.wrapping_sub(4), cookie);
    let e = a0.wrapping_sub(0x80);
    let a = fr.arg(g, 0);
    let s = g.read::<u32>(a.wrapping_add(4));
    let at = |o: u32| s.wrapping_add(o);
    let up = |g: &mut G| {
        let x = g.read::<u32>(at(0x18)).wrapping_add(1);
        g.write::<u32>(at(0x18), x);
    };
    let state = g.read::<u32>(at(0x18));
    let eax = match state {
        0 => {
            g.write::<u32>(at(0x18), 1);
            1
        }
        0x190 => {
            let x = sub(g.read::<f32>(w(0x5d)), k(g, 0x65));
            let spin = mul(mul(0.0, k(g, 0x70)), k(g, 0x84));
            abs_stores(g, img, f, 0x76, 0x80);
            abs_stores(g, img, f, 0x88, 0xd8);
            for o in [0x28u32, 0x2c, 0x30] {
                g.write::<u32>(e + o, 0);
            }
            abs_stores(g, img, f, 0xf0, 0x14a);
            g.write::<f32>(w(0x14e), x);
            g.write::<u16>(w(0x154), 0);
            g.write::<f32>(w(0x15c), spin);
            g.write::<f32>(w(0x164), spin);
            g.write::<u16>(w(0x16a), 0);
            g.write::<u32>(w(0x16f), 0);
            abs_stores(g, img, f, 0x173, 0x17d);
            up(g);
            0
        }
        0x191 | 0x192 => {
            if state == 0x191 {
                abs_stores(g, img, f, 0x18e, 0x19f);
                up(g);
            }
            let on = g.read::<u8>(w(0x1b1)) & 1 != 0;
            let x = add(g.read::<f32>(w(0x1ba)), k(g, 0x1c2));
            g.write::<f32>(w(0x1ca), x);
            if on {
                abs_stores(g, img, f, 0x1d9, 0x1e3);
                // call width: 2
                let r = g.call(t(g, 0x1d5, 325), 0, &[w(0x1ef), w(0x1ea)]).eax;
                abs_stores(g, img, f, 0x1f8, 0x202);
                up(g);
                r
            } else {
                state
            }
        }
        0x19a..=0x19c => {
            let mut eax = 0;
            if state == 0x19a {
                eax = summon_start_0609(g, img, fr, e);
                let x = g.read::<u32>(at(0x18)).wrapping_add(2);
                g.write::<u32>(at(0x18), x);
                g.write::<u32>(at(0x38), 0);
            } else if state == 0x19b {
                up(g);
                g.write::<u32>(at(0x38), 0);
            }
            let y = sub(g.read::<f32>(w(0x931)), k(g, 0x939));
            g.write::<f32>(w(0x941), y);
            if g.read::<u32>(at(0x38)).cast_signed() >= 0x28 {
                up(g);
            }
            eax
        }
        0x2bc => {
            abs_stores(g, img, f, 0x978, 0x989);
            let mut st = Pushes::default();
            st.push_all(&[w(0x990), w(0x995)]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x974, 325), 0);
            let x = gw(g, 0x9b1);
            st.push_all(&[e + 0x30, e + 0x2c, e + 0x28, x]);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x9ab, 262), 0);
            st.push(gw(g, 0x9c4));
            // call width: 12 (the most `Pushes::words` holds)
            let v = st0_f32(st.call(g, t(g, 0x9be, 12), 0).st0);
            g.write::<f32>(e + 0x2c, v);
            let y = sub(v, k(g, 0x9db));
            let m = e + 0x38;
            let mut st = Pushes::default();
            st.push_all(&[w(0x9e7), m]);
            g.write::<f32>(w(0x9f5), y);
            // call width: 12 (the most `Pushes::words` holds)
            st.call(g, t(g, 0x9ed, 327), 0);
            st.push_all(&[w(0xa02), m, w(0xa11)]);
            // call width: 12 (the most `Pushes::words` holds)
            let r = st.call(g, t(g, 0xa0c, 229), 0).eax;
            abs_stores(g, img, f, 0xa20, 0xa5c);
            up(g);
            r
        }
        0x2710 => {
            crate::l3_common::list_unlink_current(g, img, call(0xa69), state);
            let node = g.read::<u32>(a);
            let (nx, pv) = (g.read::<u32>(node + 4), g.read::<u32>(node + 8));
            g.write::<u32>(nx + 8, pv);
            let (pv, nx) = (g.read::<u32>(node + 8), g.read::<u32>(node + 4));
            g.write::<u32>(pv + 4, nx);
            g.write::<u32>(node + 8, node);
            g.write::<u32>(node + 4, node);
            nx
        }
        0x19d..=0x2bb => state - 0x19c,
        _ => state,
    };
    for o in [0x34u32, 0x38] {
        let x = g.read::<u32>(at(o)).wrapping_add(1);
        g.write::<u32>(at(o), x);
    }
    Ret::eax(eax)
}

/// [`summon_task_0609`]'s state 0x19a (as `summon_start_0613`'s).
#[allow(clippy::too_many_lines)] // the original's steps, in order
fn summon_start_0609<G: Game>(g: &mut G, img: &Image, fr: &Frame, e: u32) -> u32 {
    use crate::r25::abs_stores;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let t = |g: &G, o: u32, n: u32| slot_target(g, w(o), n);
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let call = |o: u32| img.rel32_target(f + o);
    let grp = e + 0xc;
    let x = gw(g, 0x20b);
    for (i, v) in (0u32..).zip([w(0x217), w(0x21f), x]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    g.write::<u8>(grp + 0xc, 0);
    abs_stores(g, img, f, 0x22c, 0x236);
    crate::sa_summon::model_node_init(g, img, call(0x237), grp);
    let ts = Frame {
        at: call(0x241),
        edx: w(0x23c),
        ..*fr
    };
    crate::m0609::task_start(g, img, &ts);
    let old = gw(g, 0x246);
    abs_stores(g, img, f, 0x24a, 0x27b);
    append_0601(g, img, f, old, [0x27c, 0x281, 0x287, 0x296, 0x2a2]);
    let x = gw(g, 0x2b7);
    abs_stores(g, img, f, 0x2bb, 0x2cf);
    let mut st = Pushes::default();
    st.push_all(&[w(0x2a7), w(0x2ac), w(0x2b1), x]);
    // call width: 12 (the most `Pushes::words` holds)
    st.call(g, t(g, 0x29b, 262), 0);
    st.push(gw(g, 0x2de));
    // call width: 12 (the most `Pushes::words` holds)
    let h = st0_f32(st.call(g, t(g, 0x2d8, 12), 0).st0);
    g.write::<f32>(w(0x2e9), h);
    let old = gw(g, 0x2ee);
    abs_stores(g, img, f, 0x2f5, 0x2ff);
    abs_stores(g, img, f, 0x2ff, 0x33a);
    append_0601(g, img, f, old, [0x33b, 0x340, 0x346, 0x355, 0x35c]);
    abs_stores(g, img, f, 0x364, 0x36e);
    g.write::<u32>(grp, w(0x372));
    g.write::<u32>(grp + 4, w(0x37a));
    crate::m0042::time_left(g, img, call(0x37f), grp, e.wrapping_sub(4));
    let owned = |g: &mut G, ops: [u32; 4], (edx, ecx): (u32, u32), at: u32| {
        for (i, o) in (0u32..).zip(ops) {
            g.write::<u32>(e.wrapping_sub(0x10 - 4 * i), w(o));
        }
        let fa = Frame {
            at: call(at),
            esp: e.wrapping_sub(0x14),
            ecx: w(ecx),
            edx: w(edx),
            ..*fr
        };
        crate::sa_misc::node_append_owned(g, img, &fa);
    };
    owned(g, [0x393, 0x38e, 0x389, 0x384], (0x398, 0x39d), 0x3a2);
    g.write::<u32>(grp, w(0x3ae));
    g.write::<u32>(grp + 4, w(0x3b6));
    abs_stores(g, img, f, 0x3ba, 0x3c4);
    let colour = Frame {
        at: call(0x3c5),
        esp: e.wrapping_sub(0x14),
        ecx: grp,
        ..*fr
    };
    let eax = crate::s3_misc::colour_node(g, img, &colour);
    let x2 = k(g, 0x3cd);
    abs_stores(g, img, f, 0x3d1, 0x417);
    crate::sb_misc::move_start(g, img, call(0x418), x2, eax);
    let x = gw(g, 0x41d);
    for (i, v) in (0u32..).zip([w(0x429), w(0x431), x, 0]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    g.write::<u8>(grp + 0x10, 0);
    g.write::<u32>(grp + 0x14, 0xc2e0_0000);
    abs_stores(g, img, f, 0x44e, 0x458);
    let ss = Frame {
        at: call(0x459),
        esp: e.wrapping_sub(0x14),
        ecx: grp,
        ..*fr
    };
    crate::r31::summon_setup_0609(g, img, &ss);
    let lo = Frame {
        at: call(0x45e),
        esp: e.wrapping_sub(0x14),
        ..*fr
    };
    crate::m0609::link_object(g, img, &lo);
    let sl = Frame {
        at: call(0x466),
        esp: e.wrapping_sub(8),
        ..*fr
    };
    crate::s4_small::static_link_word(g, img, &sl);
    let old = gw(g, 0x46b);
    abs_stores(g, img, f, 0x472, 0x4a3);
    g.write::<u32>(w(0x4a4), old);
    if old == w(0x4a9) {
        abs_stores(g, img, f, 0x4af, 0x4b9);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(0x4be));
    }
    let old2 = gw(g, 0x4c3);
    let n = g.read::<u16>(w(0x4ca)).wrapping_add(1);
    g.write::<u16>(w(0x4ca), n);
    abs_stores(g, img, f, 0x4ce, 0x514);
    abs_stores(g, img, f, 0x514, 0x545);
    append_pair_0601(
        g,
        img,
        f,
        old2,
        [0x546, 0x54b, 0x551, 0x560, 0x566],
        (0x56b, (0x571, 0x5e8), (0x5ea, 0x5f0, 0x5fc), 0x632),
    );
    abs_stores(g, img, f, 0x626, 0x630);
    abs_stores(g, img, f, 0x636, 0x654);
    owned(g, [0x618, 0x613, 0x60c, 0x607], (0x61d, 0x622), 0x655);
    let parent = w(0x61d);
    let old = gw(g, 0x65a);
    abs_stores(g, img, f, 0x661, 0x67e);
    g.write::<u32>(w(0x680), parent);
    g.write::<u32>(w(0x686), parent);
    g.write::<u32>(w(0x68b), old);
    if old == parent {
        abs_stores(g, img, f, 0x693, 0x69d);
    } else {
        g.write::<u32>(old.wrapping_add(4), w(0x6a2));
    }
    let n = g.read::<u16>(w(0x6b0)).wrapping_add(2);
    for (src, dst) in [(0x6aau32, 0x6b8u32), (0x6c0, 0x6c8)] {
        let x = g.read::<u32>(w(src));
        g.write::<u32>(w(dst), x);
    }
    let z = g.read::<u32>(w(0x6d0));
    abs_stores(g, img, f, 0x6db, 0x6f9);
    g.write::<u32>(w(0x6fd), z);
    abs_stores(g, img, f, 0x701, 0x73c);
    if w(0x6d5) == w(0x73e) {
        abs_stores(g, img, f, 0x744, 0x74e);
    } else {
        abs_stores(g, img, f, 0x750, 0x75a);
    }
    abs_stores(g, img, f, 0x760, 0x76a);
    g.write::<u16>(w(0x76c), n);
    abs_stores(g, img, f, 0x770, 0x7ac);
    for (i, v) in (0u32..).zip([w(0x7b0), w(0x7b8), 0x4444_8000]) {
        g.write::<u32>(grp + 4 * i, v);
    }
    abs_stores(g, img, f, 0x7c4, 0x7ce);
    crate::o6_fam_node::node_reset_sound(g, img, call(0x7cf), grp);
    abs_stores(g, img, f, 0x7dd, 0x88b);
    // call width: 2
    let list = g.call(t(g, 0x7d9, 288), 0, &[0x10, grp]).eax;
    let mut eax = list;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<u32>(grp).cast_signed() {
        let who = u32::from(g.read::<u8>(list.wrapping_add(i)));
        i += 1;
        if who == gw(g, 0x8b5) {
            continue;
        }
        let n = gw(g, 0x8a3).cast_signed();
        let mut found = false;
        let mut j = 0i32;
        let mut p = w(0x8c2);
        if n > 0 {
            eax = p;
        }
        while j < n {
            if who == g.read::<u32>(p) {
                found = true;
                break;
            }
            j += 1;
            p = p.wrapping_add(0x278);
            eax = p;
        }
        if found {
            continue;
        }
        // call width: 1
        let kind = g.call(t(g, 0x8d5, 390), 0, &[who]).eax;
        let mut d = kind;
        let mut skip = false;
        for step in [0x106au32, 0x8e, 2] {
            d = d.wrapping_sub(step);
            eax = d;
            if d == 0 {
                skip = true;
                break;
            }
        }
        if skip {
            continue;
        }
        let b = u32::from(g.read::<u8>(list.wrapping_add(i - 1)));
        // call width: 2
        eax = g.call(t(g, 0x8ff, 296), 0, &[b, 0]).eax;
    }
    eax
}

/// [`summon_task_0609`] (the `rel32`s: its helpers and the summon start's).
pub const SUMMON_TASK_0609: Sig = Sig {
    len: 2719,
    hash: 0x2ff5_ab08_7421_e39d,
    calls: &[
        (0x237, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0x241, Callee::Sig(&crate::m0609::TASK_START)),
        (0x37f, Callee::Sig(&crate::m0042::TIME_LEFT)),
        (0x3a2, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (0x3c5, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x418, Callee::Sig(&crate::sb_misc::MOVE_START)),
        (0x459, Callee::Sig(&crate::r31::SUMMON_SETUP_0609)),
        (0x45e, Callee::Sig(&crate::m0609::LINK_OBJECT)),
        (0x466, Callee::Sig(&crate::s4_small::STATIC_LINK_WORD)),
        (0x655, Callee::Sig(&crate::sa_misc::NODE_APPEND_OWNED)),
        (
            0x7cf,
            Callee::Sig(&crate::o6_fam_node::NODE_RESET_SOUND_0155_2FC0),
        ),
        (0xa69, Callee::Sig(&crate::l3_common::LIST_UNLINK_CURRENT)),
        (0xa97, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0606`'s summon placement (0x10006a20, 529 bytes; internal, no
/// arguments; a /GS `ebp` frame: the caster's point `P` at `ebp - 0x14`
/// (`w` 1.0 at `ebp - 8`), the angles `(0, yaw, 0, 0)` at `ebp - 0x24`):
/// [`place_0165`](crate::r30::place_0165) without the scene list and the
/// actors' slot 536 loop. `S = *G_S`; `P` = the caster's position (slot
/// 262) into `S + 0x9e0` and `S + 0x9d0` (`w` 1.0); the yaw `K - slot 333`
/// (ST0 stored as f32) wrapped once into `-pi..pi` at `S + 0x9c4` and `S +
/// 0x5ee8`; the summon's part shown
/// ([`part_toggle`](crate::s4_misc::part_toggle)) and turned
/// ([`scale_rotate`](crate::sa_misc::scale_rotate) by the angles), slot 8
/// `(chr, 0)`; the scene point
/// ([`scene_point`](crate::s6_misc::scene_point)) into `S + 0x9f0` and its
/// ground at `S + 0x9f4` (scene 0x18b0046, slot 520: the caster's height;
/// else slot 12 of the point's actor: the caster's for index -1, else the
/// selected actor's table word); the others shown
/// ([`show_others`](crate::sb_vm::show_others)); the sparks
/// ([`sparks_0165`](crate::r30::sparks_0165)); `S + 0x5ee0`, `+ 0x5ee4`,
/// `+ 0x5efc` cleared and `*G_SET` = 1. Returns the sparks' `eax`.
pub fn place_0606<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> u32 {
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x25), n);
    let call = |o: u32| img.rel32_target(f + o);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |o: u32| ebp.wrapping_sub(o);
    let cookie = gw(g, 0x7) ^ ebp;
    g.write::<u32>(l(4), cookie);
    // push esi; push edi below the frame: a callee's return address here
    let inner = fr.esp.wrapping_sub(0x38);
    let s = gw(g, 0x14);
    let p = l(0x14);
    let chr = gw(g, 0x2b);
    // call width: 4
    g.call(slot(g, 262), 0, &[chr, p, p + 4, p + 8]);
    for i in 0..3u32 {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(s + 0x9e0 + 4 * i, x);
    }
    g.write::<u32>(s + 0x9ec, ONE);
    let chr = gw(g, 0x6f);
    g.write::<u32>(l(8), ONE);
    g.write::<u32>(l(0x24), 0);
    // call width: 1
    let r = st0_f32(g.call(slot(g, 333), 0, &[chr]).st0);
    g.write::<f32>(l(0x28), r);
    let yaw = sub(k(g, 0x90), r);
    let yaw = if yaw > k(g, 0x9f) {
        sub(yaw, k(g, 0xa9))
    } else if k(g, 0xb3) > yaw {
        add(yaw, k(g, 0xc0))
    } else {
        yaw
    };
    g.write::<u32>(s + 0x9c0, 0);
    g.write::<f32>(s + 0x9c4, yaw);
    g.write::<u32>(s + 0x9c8, 0);
    g.write::<u32>(s + 0x9cc, 0);
    for i in 0..3u32 {
        let x = g.read::<u32>(p + 4 * i);
        g.write::<u32>(s + 0x9d0 + 4 * i, x);
    }
    let ang = l(0x24);
    g.write::<f32>(ang + 4, yaw);
    g.write::<u32>(ang + 8, 0);
    g.write::<u32>(ang + 0xc, 0);
    let one = g.read::<u32>(l(8));
    g.write::<u32>(s + 0x9dc, one);
    let m = s + 0x980;
    let pt = Frame {
        at: call(0x138),
        esp: inner,
        edx: m,
        ..Frame::default()
    };
    crate::s4_misc::part_toggle(g, img, &pt);
    crate::sa_misc::scale_rotate(g, img, call(0x147), m, ang);
    let chr = gw(g, 0x154);
    // call width: 2
    g.call(slot(g, 8), 0, &[chr, 0]);
    let sp = Frame {
        at: call(0x167),
        esp: inner,
        ecx: s + 0x9f0,
        ..Frame::default()
    };
    let idx = crate::s6_misc::scene_point(g, img, &sp);
    // call width: 0
    let scene = g.call(slot(g, 520), 0, &[]).eax;
    if scene == 0x018b_0046 {
        let y = g.read::<u32>(s + 0x9e4);
        g.write::<u32>(s + 0x9f4, y);
    } else {
        let chr = if idx == u32::MAX {
            gw(g, 0x196)
        } else {
            let b = u32::from(g.read::<u8>(gw(g, 0x19d).wrapping_add(idx)));
            g.read::<u32>(w(0x1a8).wrapping_add(4 * b))
        };
        // call width: 1
        let y = st0_f32(g.call(slot(g, 12), 0, &[chr]).st0);
        g.write::<f32>(s + 0x9f4, y);
    }
    let yaw = g.read::<u32>(ang + 4);
    g.write::<u32>(s + 0x5ee8, yaw);
    crate::sb_vm::show_others(g, img, call(0x1d0), inner, 0);
    let sf = Frame {
        at: call(0x1d5),
        esp: inner,
        ..Frame::default()
    };
    let eax = crate::r30::sparks_0165(g, img, &sf);
    for o in [0x5ee0u32, 0x5ee4, 0x5efc] {
        g.write::<u32>(s + o, 0);
    }
    g.write::<u32>(w(0x1ff), 1);
    eax
}

/// [`place_0606`].
pub const PLACE_0606: Sig = Sig {
    len: 529,
    hash: 0xce17_9675_38d6_6029,
    calls: &[
        (0x138, Callee::Sig(&crate::s4_misc::PART_TOGGLE)),
        (0x147, Callee::Sig(&crate::sa_misc::SCALE_ROTATE)),
        (0x167, Callee::Sig(&crate::s6_misc::SCENE_POINT)),
        (0x1d0, Callee::Sig(&crate::sb_vm::SHOW_OTHERS)),
        (0x1d5, Callee::Sig(&crate::r30::SPARKS_0165)),
        (0x209, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0606`'s start-up (0x10008a90, 336 bytes):
/// [`start_up_0165`](crate::r30::start_up_0165)'s code with its own
/// placement ([`place_0606`]). Returns 0.
pub fn start_up_0606<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r30::start_up_0165_with(g, img, fr, place_0606)
}

/// [`start_up_0606`].
pub const START_UP_0606: Sig = Sig {
    len: 336,
    hash: 0x078f_38e0_51f1_9730,
    calls: &[
        (0x56, Callee::Sig(&crate::sb_misc::PART_SETUP)),
        (0x5b, Callee::Sig(&PLACE_0606)),
        (0x60, Callee::Sig(&crate::s4_small::RESET_AND_SCALE_165)),
        (0xc4, Callee::Sig(&crate::sa_pool::TAKE)),
        (0x102, Callee::Sig(&crate::sa_misc::RECORDS_INIT)),
        (0x10c, Callee::Sig(&crate::sa_pool::TAKE)),
    ],
    imports: &[],
};

/// `magic_0606`'s load step (0x10006610, the 118-byte shape) running
/// [`start_up_0606`].
pub fn load_0606<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    crate::r9_load::load_step(g, img, fr, crate::r9_load::Shape::Zero, start_up_0606)
}

/// [`load_0606`].
pub const LOAD_0606: Sig = Sig {
    len: 118,
    hash: 0xf93b_1968_cd8e_f184,
    calls: &[(0x5b, Callee::Sig(&START_UP_0606))],
    imports: &[],
};

/// `magic_0610`'s actor clips (0x1000c7b0, 1607 bytes; a table entry, no
/// arguments read; an aligned /GS frame `E`: the vertex base `vb` at `E`,
/// `S` at `E + 4`, the output `out` at `E + 8`, the actor's point `Q` at
/// `E + 0x24`, the caster's `P` at `E + 0x34`, the matrices `W` at `E +
/// 0x44`, `U` (unit, slot 235) at `E + 0x84`, `T` at `E + 0xc4`, a copy at
/// `E + 0x104`). `S = *G_S`, `vb = *G_VB`, `out = *G_OUT`, `P` = the
/// caster's place (slot 262, `w` 1.0).
///
/// With `S + 0x1a80` set, the list `*G_L` gets `N - 1` and then the
/// selected actors' words `1..N` (`N = *G_N`). Then for each selected
/// actor `i` from 1 (records `R = S + 0xd78 + 0xc8 (i - 1)`):
/// - slot 530 `(sel[i], 0)` for a party member (`*G_ALL` 0 and the byte
///   below 0x20), else slot 249 `(actor, 0)`; `R + 0x1c` = `S + 0x1a80`;
/// - with it 1: `W` = `U` turned by the caster's yaw `K - slot 333` (slot
///   329, wrapped once), copied (slot 230); `Q` = the actor's place (slot
///   262); `W = T W` (slot 231) with `T` = `U` turned by `K - (K -
///   atan2(dz, dx))` (each wrapped once; `_CIatan2`); `R = vb`; the
///   actor's model clipped ([`clip_0434`](crate::r30::clip_0434), `ecx`
///   the actor, `edx` `W`, `(actor, out, &vb, R - 8, R - 4, R + 4, R +
///   8)`, `*G_I` = `i`) into `out`; slot 210 `(actor, R - 0xa8)` and
///   `(actor, R - 0x68)`; `R + 8` = the `y` of slot 229 `((0, R[8], 0, 1),
///   R - 0xa8)`; `R - 0x28`, `- 0x24`, `- 0x20`, `- 0x18` cleared, `R -
///   0x14`, `R - 0x10` slot 314 `(0.08)`, `(0.1)` (ST0), `R - 0xc` = slot
///   404 `(1.0)` + `K`, `R - 0x1c` that times `K'`; `R + 0x14`, `+ 0x18`,
///   `+ 0xc` = 0x14, 0x1e, 0x80; slot 238 `(sel[i], 0)`, slot 548 `(actor)`;
///   the next `vb` is the one the clip left;
/// - else slot 296 `(sel[i], 1)` for a party member, else slot 257
///   `(actor, 0)`;
/// - `R + 0x10` = 0x37.
///
/// The end: a task node off the free ring (inlined take: the count u16
/// `*G_FREE_N`, the head `*G_FREE`; the node's callback the DLL word
/// operand `+0x5ec`) into `S + 0x1a64` and appended to the task list.
/// With no free node the original writes through address 4, as this does.
/// Returns the list's old last node, or 0xffff when the list was empty.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn clips_0610<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x3b), n);
    // push ebp; and esp, -8; sub esp, 0x14c; push ebx, esi, edi
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x14c);
    let cookie = gw(g, 0xd) ^ e;
    g.write::<u32>(e + 0x148, cookie);
    let out0 = gw(g, 0x1b);
    let mut vb = gw(g, 0x22);
    g.write::<u32>(e + 8, out0);
    let s = gw(g, 0x2e);
    g.write::<u32>(e + 4, s);
    g.write::<u32>(e, vb);
    let (p, q) = (e + 0x34, e + 0x24);
    let (wm, unit, turn, copy) = (e + 0x44, e + 0x84, e + 0xc4, e + 0x104);
    // call width: 1
    g.call(slot(g, 235), 0, &[unit]);
    let caster = gw(g, 0x68);
    // call width: 4
    g.call(slot(g, 262), 0, &[caster, p, p + 4, p + 8]);
    g.write::<u32>(p + 0xc, ONE);
    // the actor word of selection index `i` (the code also takes the
    // caster's word for an index of -1, which the loops starting at 1
    // never reach)
    let actor = |g: &G, i: u32, (sel, table): (u32, u32)| {
        let b = u32::from(g.read::<u8>(gw(g, sel).wrapping_add(i)));
        g.read::<u32>(w(table).wrapping_add(4 * b))
    };
    let n = |g: &G| gw(g, 0x75).cast_signed();
    if g.read::<u32>(s + 0x1a80) != 0 {
        let list = gw(g, 0x8f);
        g.write::<i32>(list, n(g) - 1);
        let mut i = 1u32;
        while i.cast_signed() < n(g) {
            let v = actor(g, i, (0xb1, 0xbc));
            g.write::<u32>(list + 4 * i, v);
            i += 1;
        }
    }
    // a party member: `*G_ALL` 0 and the selected byte below 0x20
    let member = |g: &G, i: u32, (all, sel): (u32, u32)| -> Option<u32> {
        if gw(g, all) != 0 {
            return None;
        }
        let b = u32::from(g.read::<u8>(gw(g, sel).wrapping_add(i)));
        (b < 0x20).then_some(b)
    };
    let wrap = |x: f32, (hi, lo, d): (f32, f32, f32)| {
        if x > hi {
            sub(x, d)
        } else if lo > x {
            add(x, d)
        } else {
            x
        }
    };
    let mut i = 1u32;
    let mut r = s + 0xd78;
    while i.cast_signed() < n(g) {
        if let Some(b) = member(g, i, (0xe2, 0xe9)) {
            // call width: 2
            g.call(slot(g, 530), 0, &[b, 0]);
        } else {
            let a = actor(g, i, (0xe9, 0x125));
            // call width: 2
            g.call(slot(g, 249), 0, &[a, 0]);
        }
        let mode = g.read::<u32>(s + 0x1a80);
        g.write::<u32>(r + 0x1c, mode);
        if mode == 1 {
            let a = actor(g, i, (0x15f, 0x16a));
            // call width: 1
            let turn0 = st0_f32(g.call(slot(g, 333), 0, &[a]).st0);
            g.write::<f32>(e + 0xc, turn0);
            let yaw = wrap(
                sub(k(g, 0x184), turn0),
                (k(g, 0x194), k(g, 0x1a8), k(g, 0x19e)),
            );
            // call width: 3
            g.call(slot(g, 329), 0, &[wm, unit, yaw.to_bits()]);
            // call width: 2
            g.call(slot(g, 230), 0, &[copy, wm]);
            let a = actor(g, i, (0x206, 0x211));
            // call width: 4
            g.call(slot(g, 262), 0, &[a, q, q + 4, q + 8]);
            let rf = |g: &G, at: u32| g.read::<f32>(at);
            let dz = sub(rf(g, q + 8), rf(g, p + 8));
            let dx = sub(rf(g, q), rf(g, p));
            g.write::<u32>(q + 0xc, ONE);
            let ang = g.atan2(f64::from(dz), f64::from(dx));
            g.write::<f64>(e + 0xc, ang);
            #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
            let ang = ang as f32;
            let (hi, lo, d) = (k(g, 0x28a), k(g, 0x292), k(g, 0x29a));
            let a1 = wrap(sub(k(g, 0x282), ang), (hi, lo, d));
            let a2 = wrap(sub(hi, a1), (hi, lo, d));
            // call width: 3
            g.call(slot(g, 329), 0, &[turn, unit, a2.to_bits()]);
            // call width: 3
            g.call(slot(g, 231), 0, &[wm, turn, wm]);
            g.write::<u32>(r, vb);
            let a = actor(g, i, (0x32d, 0x338));
            // the clip's seven words below `E - 0xc`, its return address
            let sp = e.wrapping_sub(0x28);
            let out = g.read::<u32>(e + 8);
            for (j, v) in (0u32..).zip([a, out, e, r - 8, r - 4, r + 4, r + 8]) {
                g.write::<u32>(sp + 4 * j, v);
            }
            g.write::<u32>(w(0x35c), i);
            let cf = Frame {
                at: img.rel32_target(f + 0x361),
                esp: sp.wrapping_sub(4),
                ecx: a,
                edx: wm,
                ..Frame::default()
            };
            let end = crate::r30::clip_0434(g, img, &cf);
            g.write::<u32>(e + 8, end);
            let a = actor(g, i, (0x37a, 0x385));
            // call width: 2
            g.call(slot(g, 210), 0, &[a, r - 0xa8]);
            let a = actor(g, i, (0x3af, 0x3ba));
            // call width: 2
            g.call(slot(g, 210), 0, &[a, r - 0x68]);
            let y = g.read::<u32>(r + 8);
            for (j, v) in (0u32..).zip([0, y, 0, ONE]) {
                g.write::<u32>(q + 4 * j, v);
            }
            // call width: 3
            g.call(slot(g, 229), 0, &[q, r - 0xa8, q]);
            let y = g.read::<u32>(q + 4);
            g.write::<u32>(r + 8, y);
            for o in [0x28u32, 0x24, 0x20, 0x18] {
                g.write::<u32>(r - o, 0);
            }
            for (o, x) in [(0x14u32, 0x3da3_d70au32), (0x10, 0x3dcc_cccd)] {
                // call width: 1
                let v = st0_f32(g.call(slot(g, 314), 0, &[x]).st0);
                g.write::<f32>(r - o, v);
            }
            // call width: 1
            let v = st0_f32(g.call(slot(g, 404), 0, &[ONE]).st0);
            g.write::<f32>(e + 0xc, v);
            let v = add(v, k(g, 0x481));
            g.write::<f32>(r - 0xc, v);
            g.write::<f32>(r - 0x1c, mul(v, k(g, 0x491)));
            g.write::<u32>(r + 0x14, 0x14);
            g.write::<u32>(r + 0x18, 0x1e);
            g.write::<u32>(r + 0xc, 0x80);
            let b = u32::from(g.read::<u8>(gw(g, 0x4bc).wrapping_add(i)));
            // call width: 2
            g.call(slot(g, 238), 0, &[b, 0]);
            let a = actor(g, i, (0x4e4, 0x4ef));
            // call width: 1
            g.call(slot(g, 548), 0, &[a]);
            vb = g.read::<u32>(e);
        } else if let Some(b) = member(g, i, (0x50c, 0x513)) {
            // call width: 2
            g.call(slot(g, 296), 0, &[b, 1]);
        } else {
            let a = actor(g, i, (0x513, 0x54f));
            // call width: 2
            g.call(slot(g, 257), 0, &[a, 0]);
        }
        g.write::<u32>(r + 0x10, 0x37);
        i += 1;
        r += 0xc8;
    }
    // the task node: the free ring's take inlined
    let free_n = g.read::<u16>(w(0x587));
    let mut eax = 0xffff;
    let node = if free_n == 0 {
        0
    } else {
        let node = gw(g, 0x596);
        let (next, prev) = (g.read::<u32>(node), g.read::<u32>(node + 4));
        g.write::<u32>(next + 4, prev);
        let (prev, next) = (g.read::<u32>(node + 4), g.read::<u32>(node));
        g.write::<u32>(prev, next);
        if free_n > 1 {
            let p = g.read::<u32>(node + 4);
            g.write::<u32>(w(0x5b3), p);
        } else {
            g.write::<u32>(w(0x5bb), 0);
        }
        for o in [0u32, 4, 0xc, 0x14] {
            g.write::<u32>(node + o, 0);
        }
        g.write::<u32>(node + 8, node);
        g.write::<u32>(node + 0x10, w(0x5ec));
        g.write::<u8>(node + 0x18, 0);
        g.write::<u16>(w(0x5f7), free_n - 1);
        node
    };
    let listed = gw(g, 0x5fd) != 0;
    g.write::<u32>(s + 0x1a64, node);
    g.write::<u32>(node.wrapping_add(4), 0);
    if listed {
        let last = gw(g, 0x612);
        g.write::<u32>(node, last);
        g.write::<u32>(last.wrapping_add(4), node);
        eax = last;
    } else {
        g.write::<u32>(node, 0);
        g.write::<u32>(w(0x625), node);
    }
    g.write::<u32>(w(0x637), node);
    g.write::<u32>(node.wrapping_add(8), node);
    Ret::eax(eax)
}

/// [`clips_0610`].
pub const CLIPS_0610: Sig = Sig {
    len: 1607,
    hash: 0x6b9b_1805_f4cf_3f5c,
    calls: &[
        (0x270, Callee::Import("_CIatan2")),
        (0x361, Callee::Sig(&crate::r30::CLIP_0434)),
        (0x63f, Callee::Any),
    ],
    imports: &[],
};
