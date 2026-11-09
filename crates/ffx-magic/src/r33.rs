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

/// The 100 summon records' per-frame step of `magic_0165` (0x10003990)
/// and `magic_0606` (0x10007bd0) (2882 bytes; a task callback, no
/// arguments read; an aligned /GS frame `E`: the parts' words `E + 4`,
/// `+ 0x14`, `+ 0x24`, `+ 0x2c`, the record `E + 0x18` and its index `E +
/// 0x1c`, the scale `L8` at `E + 8`, `L10`/`L20`/`L28` at `E + 0x10`, `+
/// 0x20`, `+ 0x28`, the matrices `N` (`E + 0x30`), `T` (`E + 0x70`), `M`
/// (`E + 0xb0`) and `U` (`E + 0x100`)). `S = *G_S`.
///
/// - a countdown `S + C` (`C` the displacement at `+0x24`, 0x5f14 or
///   0x5ef4) above 0 counts down; reaching 0 calls slot 239 `(*G_X, -5,
///   0)`. Slot 382 `(0, 0)`, slot 559 `(E + 0x24, E + 0x14, E + 4)`, slot
///   558 `(E + 0xf0)`, slot 382 again; the parts' words from slot 719
///   `(NAME, 2, 7)` and `(NAME, 2, 6)`; `U` a unit matrix (slot 235);
/// - each record `R = S + 0xa00 + 0xc8 i` (`i < 100`), with `L8 = k`,
///   `L10 = R[0xb8]`, `L20 = R[0xbc]`, `L28 = 0x80`: a delay `R + 0xac`
///   above 0 counts down and skips the motion; else the angles `R +
///   0x10..` move by `R + 0x40..` (which move by `R + 0x50..`; each angle
///   wrapped once), the place `R + 0..` by `R + 0x20..` (which move by
///   `R + 0x30..`); a wait `R + 0xa8` above 0 counts down, else the fade
///   `R + 0xa4` counts down to 0 and `L28 = (f << 7) / R[0xa0]`, `L10 =
///   R[0xb8] f / R[0xa0]`, `L20 = R[0xbc] f / R[0xa0]` (integer, toward 0);
///   then (unless the motion was skipped) the scale count `R + 0xb4`
///   counts down to 0 and `L8 = (k - s) K + s`, `s = R[0xb4] / R[0xb0]`;
/// - the drawing: `M` a unit matrix (slot 235) with `R + 0x80..` times `L8`
///   on its diagonal, `N` = `M` turned by `R + 0x18`, `R + 0x10`, `R +
///   0x14` (slots 330, 328, 329), its last row the place plus `S + 0x9f0`
///   (`w` copied); the spins `R + 0x60..` move by `R + 0x70..` (wrapped
///   once), `T` = `U` turned by them; the parts by `R + 0xc0` (2: slot 719
///   `(NAME, 4, 4)` and `(NAME, 4, 5)`, 1: `(.., 4, 2)`, `(.., 4, 3)`,
///   else `(.., 4, 1)` twice); `R + 0x90`, `R + 0x94` move by `R + 0x98`,
///   `R + 0x9c` (wrapped by one constant both ways); with `L28` above 0
///   the parts drawn ([`three_parts_draw`](crate::s5_misc::three_parts_draw),
///   `ecx` the first part's handle, `edx` the second's, `(N, T, the four
///   parts' words, R[0x90], R[0x94], L28, L10, L20, R[0xc4], R, i)`);
/// - slot 382 `(0, 0)` twice; two rounds each of two kinds of particles:
///   a record picked by `rand % 100`, not delayed and with its scale count
///   above 10 (above 50 for the second kind), gives a pool record
///   ([`pool_take`](crate::sb_misc::pool_take)) at its place plus `S +
///   0x9f0` and slot 314 jitter, with slot 314 sizes and spins.
///
/// `k` is a constant the compiler keeps in a register: `K(0x121)` for the
/// first record, then `K(0x644)` after a drawn record or `K(0x597)` after
/// one not drawn (the same 1.0 in both DLLs). Returns the last particle
/// round's `eax`: slot 314's, 0 without a pool record, or the picked
/// record's offset `0xc8 (rand % 100)` when it was skipped.
#[allow(clippy::too_many_lines)] // the original's steps, in order
pub fn records_task_0165<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, div, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let rf = |g: &G, at: u32| g.read::<f32>(at);
    let slot = |g: &G, n: u32| slot_target(g, w(0x3a), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x148; push esi; push edi
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x148);
    let cookie = gw(g, 0xd) ^ e;
    g.write::<u32>(e + 0x144, cookie);
    let s = gw(g, 0x1d);
    g.write::<u32>(e + 0xc, s);
    let count = s.wrapping_add(w(0x24));
    let c = g.read::<i32>(count);
    if c > 0 {
        g.write::<i32>(count, c - 1);
        if c == 1 {
            let x = gw(g, 0x4a);
            // call width: 3
            g.call(slot(g, 239), 0, &[x, 0xffff_fffb, 0]);
        }
    }
    // call width: 2
    g.call(slot(g, 382), 0, &[0, 0]);
    // call width: 3
    g.call(slot(g, 559), 0, &[e + 0x24, e + 0x14, e + 4]);
    // call width: 1
    g.call(slot(g, 558), 0, &[e + 0xf0]);
    // call width: 2
    g.call(slot(g, 382), 0, &[0, 0]);
    let parts = |g: &mut G, name: u32, a: u32, b: u32| {
        // call width: 3
        let p = g.call(slot(g, 719), 0, &[name, a, b]).eax;
        (g.read::<u32>(p), g.read::<u32>(p + 4))
    };
    let (p7, q7) = parts(g, w(0xbc), 2, 7);
    g.write::<u32>(e + 0x14, q7);
    let (p6, q6) = parts(g, w(0xde), 2, 6);
    g.write::<u32>(e + 4, p7);
    g.write::<u32>(e + 0x2c, q6);
    g.write::<u32>(e + 0x24, p6);
    let (n, t, m, u) = (e + 0x30, e + 0x70, e + 0xb0, e + 0x100);
    // call width: 1
    g.call(slot(g, 235), 0, &[u]);
    g.write::<u32>(e + 0x6c, ONE);
    // the constants the loop keeps in registers (see above)
    let (mut hi1, mut d1) = (k(g, 0x111), k(g, 0x119));
    let mut kk = k(g, 0x121);
    let wrap = |x: f32, hi: f32, lo: f32, d: f32| {
        if x > hi {
            sub(x, d)
        } else if lo > x {
            add(x, d)
        } else {
            x
        }
    };
    for i in 0..100u32 {
        let r = s + 0xa00 + 0xc8 * i;
        g.write::<u32>(e + 0x18, r);
        g.write::<u32>(e + 0x1c, i);
        let at = |o: u32| r + o;
        let (mut l10, mut l20, mut l28) =
            (g.read::<u32>(at(0xb8)), g.read::<u32>(at(0xbc)), 0x80u32);
        let mut l8 = kk;
        let delay = g.read::<i32>(at(0xac));
        if delay > 0 {
            g.write::<i32>(at(0xac), delay - 1);
        } else {
            // the angles: their rates move, then they move (wrapped once)
            for j in 0..3u32 {
                let v = add(rf(g, at(0x50 + 4 * j)), rf(g, at(0x40 + 4 * j)));
                g.write::<f32>(at(0x40 + 4 * j), v);
            }
            // (the operands in the original's order: `R40 + R10`, `R14 + R44`,
            // `R48 + R18`)
            let x = add(rf(g, at(0x40)), rf(g, at(0x10)));
            g.write::<f32>(at(0x10), x);
            let x = add(rf(g, at(0x14)), rf(g, at(0x44)));
            g.write::<f32>(at(0x14), x);
            let x = add(rf(g, at(0x48)), rf(g, at(0x18)));
            g.write::<f32>(at(0x18), x);
            let lo1 = k(g, 0x181);
            for j in 0..3u32 {
                let x = wrap(rf(g, at(0x10 + 4 * j)), hi1, lo1, d1);
                g.write::<f32>(at(0x10 + 4 * j), x);
            }
            // the place: its speed moves, then it moves
            for j in 0..3u32 {
                let v = add(rf(g, at(0x30 + 4 * j)), rf(g, at(0x20 + 4 * j)));
                g.write::<f32>(at(0x20 + 4 * j), v);
            }
            for j in 0..3u32 {
                let x = add(rf(g, at(0x20 + 4 * j)), rf(g, at(4 * j)));
                g.write::<f32>(at(4 * j), x);
            }
            let wait = g.read::<i32>(at(0xa8));
            if wait > 0 {
                g.write::<i32>(at(0xa8), wait - 1);
            } else {
                let fade = g.read::<i32>(at(0xa4));
                if fade > 0 {
                    g.write::<i32>(at(0xa4), fade - 1);
                }
                let fade = g.read::<i32>(at(0xa4));
                let len = g.read::<i32>(at(0xa0));
                // idiv: the original faults on a zero length
                let q = |x: i32| x.checked_div(len).unwrap_or(0).cast_unsigned();
                l28 = q(fade.wrapping_shl(7));
                l10 = q(g.read::<i32>(at(0xb8)).wrapping_mul(fade));
                l20 = q(g.read::<i32>(at(0xbc)).wrapping_mul(fade));
            }
            let sc = g.read::<i32>(at(0xb4));
            if sc > 0 {
                g.write::<i32>(at(0xb4), sc - 1);
            }
            #[allow(clippy::cast_precision_loss)] // cvtdq2ps
            let s1 = div(
                g.read::<i32>(at(0xb4)) as f32,
                g.read::<i32>(at(0xb0)) as f32,
            );
            l8 = add(mul(sub(kk, s1), k(g, 0x2f6)), s1);
        }
        g.write::<u32>(e + 0x10, l10);
        g.write::<u32>(e + 0x20, l20);
        g.write::<u32>(e + 0x28, l28);
        g.write::<f32>(e + 8, l8);
        // the drawing matrices
        // call width: 1
        g.call(slot(g, 235), 0, &[m]);
        for (j, o) in (0u32..).zip([0x80u32, 0x84, 0x88]) {
            g.write::<f32>(m + 0x14 * j, mul(rf(g, at(o)), l8));
        }
        for (sl, o, src) in [(330u32, 0x18u32, m), (328, 0x10, n), (329, 0x14, n)] {
            let x = g.read::<u32>(at(o));
            // call width: 3
            g.call(slot(g, sl), 0, &[n, src, x]);
        }
        for j in 0..3u32 {
            let x = add(rf(g, at(4 * j)), rf(g, s + 0x9f0 + 4 * j));
            g.write::<f32>(n + 0x30 + 4 * j, x);
        }
        let x = g.read::<u32>(s + 0x9fc);
        g.write::<u32>(n + 0x3c, x);
        let (hi2, lo2, d2) = (k(g, 0x3c1), k(g, 0x3d5), k(g, 0x3dd));
        for j in 0..3u32 {
            let x = wrap(
                add(rf(g, at(0x70 + 4 * j)), rf(g, at(0x60 + 4 * j))),
                hi2,
                lo2,
                d2,
            );
            g.write::<f32>(at(0x60 + 4 * j), x);
        }
        for (sl, o, src) in [(330u32, 0x68u32, u), (328, 0x60, t), (329, 0x64, t)] {
            let x = g.read::<u32>(at(o));
            // call width: 3
            g.call(slot(g, sl), 0, &[t, src, x]);
        }
        let (a, b, ops) = match g.read::<u32>(at(0xc0)) {
            2 => (4, 5, (0x50b, 0x525)),
            1 => (2, 3, (0x541, 0x57b)),
            _ => (1, 1, (0x55e, 0x57b)),
        };
        // call width: 3
        let first = g.call(slot(g, 719), 0, &[w(ops.0), 4, a]).eax;
        g.write::<u32>(e + 8, first);
        // call width: 3
        let second = g.call(slot(g, 719), 0, &[w(ops.1), 4, b]).eax;
        let (hi3, d3) = (k(g, 0x58f), k(g, 0x597));
        for j in 0..2u32 {
            let x = add(rf(g, at(0x98 + 4 * j)), rf(g, at(0x90 + 4 * j)));
            g.write::<f32>(at(0x90 + 4 * j), x);
            g.write::<f32>(at(0x90 + 4 * j), wrap(x, hi3, d3, d3));
        }
        kk = k(g, 0x597);
        if l28.cast_signed() > 0 {
            let words = [
                n,
                t,
                p7,
                q7,
                p6,
                q6,
                g.read::<u32>(at(0x90)),
                g.read::<u32>(at(0x94)),
                l28,
                l10,
                l20,
                g.read::<u32>(at(0xc4)),
                r,
                i,
            ];
            let sp = e.wrapping_sub(0x40);
            for (j, v) in (0u32..).zip(words) {
                g.write::<u32>(sp + 4 * j, v);
            }
            let df = Frame {
                at: call(0x63c),
                esp: sp.wrapping_sub(4),
                ecx: first,
                edx: second,
                ..Frame::default()
            };
            crate::s5_misc::three_parts_draw(g, img, &df);
            kk = k(g, 0x644);
        }
        (hi1, d1) = (k(g, 0x65f), k(g, 0x657));
    }
    for _ in 0..2 {
        // call width: 2
        g.call(slot(g, 382), 0, &[0, 0]);
    }
    // a record by `rand % 100` (cdq; idiv: the remainder)
    let pick = |g: &mut G| {
        g.rand()
            .cast_signed()
            .wrapping_rem(100)
            .cast_unsigned()
            .wrapping_mul(0xc8)
    };
    // the particles' place: the record's plus `S + 0x9f0`, then slot 314
    // (5.0) jitter on each coordinate (the first added before, the others
    // after the node's word)
    let place = |g: &mut G, node: u32, rec: u32| {
        for j in 0..3u32 {
            let x = add(rf(g, rec + 4 * j), rf(g, s + 0x9f0 + 4 * j));
            g.write::<f32>(node + 4 * j, x);
        }
        let x = g.read::<u32>(s + 0x9fc);
        g.write::<u32>(node + 0xc, x);
        let jit = |g: &mut G| {
            // call width: 1
            let v = st0_f32(g.call(slot(g, 314), 0, &[0x40a0_0000]).st0);
            g.write::<f32>(e + 4, v);
            v
        };
        let v = jit(g);
        g.write::<f32>(node, add(v, rf(g, node)));
        for j in 1..3u32 {
            let v = jit(g);
            g.write::<f32>(node + 4 * j, add(rf(g, node + 4 * j), v));
        }
        g.write::<u32>(node + 0xc, ONE);
    };
    let st0 = |g: &mut G, sl: u32, x: u32| -> (f32, u32) {
        // call width: 1
        let o = g.call(slot(g, sl), 0, &[x]);
        (st0_f32(o.st0), o.eax)
    };
    for _ in 0..2 {
        let rec = s.wrapping_add(0xa00).wrapping_add(pick(g));
        if g.read::<i32>(rec + 0xac) > 0 || g.read::<i32>(rec + 0xb4) <= 10 {
            continue;
        }
        let node = crate::sb_misc::pool_take(g, img, call(0x6e4));
        g.write::<u32>(e + 4, node);
        if node == 0 {
            continue;
        }
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps
        let s1 = div(
            g.read::<i32>(rec + 0xb4) as f32,
            g.read::<i32>(rec + 0xb0) as f32,
        );
        let h = g.read::<u32>(gw(g, 0x70f) + 4);
        g.write::<u32>(node + 0x30, h);
        g.write::<u32>(node + 0x34, 1);
        g.write::<u16>(node + 0x38, 0xffff);
        g.write::<u16>(node + 0x3a, 0x200);
        let t2 = add(mul(sub(k(g, 0x71b), s1), k(g, 0x73b)), s1);
        g.write::<f32>(e + 0x14, t2);
        place(g, node, rec);
        for o in [0x10u32, 0x14, 0x18] {
            let v = st0(g, 314, 0x3f00_0000).0;
            g.write::<f32>(node + o, v);
        }
        let v = st0(g, 314, 0x4049_0fdb).0;
        g.write::<f32>(node + 0x1c, v);
        let k881 = k(g, 0x881);
        g.write::<f32>(node + 0x20, mul(rf(g, node + 0x10), k881));
        g.write::<u32>(node + 0x2c, ONE);
        g.write::<f32>(node + 0x24, mul(rf(g, node + 0x14), k881));
        g.write::<f32>(node + 0x28, mul(rf(g, node + 0x18), k881));
        let v = st0(g, 314, 0x3f00_0000).0;
        g.write::<f32>(e + 4, v);
        g.write::<u32>(node + 0x4c, 0x13);
        g.write::<u32>(node + 0x50, 0);
        let x = mul(mul(add(v, k(g, 0x8d3)), k(g, 0x8ec)), t2);
        g.write::<f32>(node + 0x3c, x);
    }
    // the second kind's rounds give the result
    let mut eax = 0;
    for _ in 0..2 {
        let off = pick(g);
        let rec = s.wrapping_add(0xa00).wrapping_add(off);
        g.write::<u32>(e + 4, off);
        eax = off;
        if g.read::<i32>(rec + 0xac) > 0 || g.read::<i32>(rec + 0xb4) <= 0x32 {
            continue;
        }
        let node = crate::sb_misc::pool_take(g, img, call(0x949));
        eax = node;
        if node == 0 {
            continue;
        }
        let h = g.read::<u32>(gw(g, 0x959));
        g.write::<u16>(node + 0x38, 0xffff);
        g.write::<u32>(node + 0x30, h);
        g.write::<u32>(node + 0x34, 1);
        g.write::<u16>(node + 0x3a, 0x200);
        place(g, node, rec);
        let v = st0(g, 314, 0x3c23_d70a).0;
        g.write::<f32>(node + 0x10, v);
        let (v, _) = st0(g, 404, 0x3c23_d70a);
        g.write::<f32>(e + 4, v);
        let neg = f32::from_bits(v.to_bits() ^ gw(g, 0xa7d));
        g.write::<f32>(node + 0x14, neg);
        let v = st0(g, 314, 0x3c23_d70a).0;
        g.write::<f32>(node + 0x18, v);
        let v = st0(g, 314, 0x4049_0fdb).0;
        g.write::<f32>(node + 0x1c, v);
        for o in [0x20u32, 0x24, 0x28] {
            g.write::<u32>(node + o, 0);
        }
        g.write::<u32>(node + 0x2c, ONE);
        let (v, r) = st0(g, 314, 0x3f00_0000);
        g.write::<f32>(e + 4, v);
        eax = r;
        let x = mul(add(v, k(g, 0xaf2)), k(g, 0xafd));
        let fl = g.read::<u16>(node + 0x70) | 1;
        g.write::<u16>(node + 0x70, fl);
        g.write::<u32>(node + 0x4c, 0x20);
        g.write::<f32>(node + 0x3c, x);
        g.write::<u32>(node + 0x50, 0);
        g.write::<u8>(node + 0x6b, 0x40);
        g.write::<u32>(node + 0x44, 0xc2c8_0000);
    }
    Ret::eax(eax)
}

/// [`records_task_0165`] in `magic_0165` (0x10003990; the `rel32`s:
/// [`three_parts_draw`](crate::s5_misc::three_parts_draw), the pool's
/// take twice, `__security_check_cookie`; `rand` twice).
pub const RECORDS_TASK_0165: Sig = Sig {
    len: 2882,
    hash: 0xe136_4a65_ada0_f141,
    calls: &[
        (0x63c, Callee::Sig(&crate::s5_misc::THREE_PARTS_DRAW)),
        (0x6e4, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x949, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0xb3a, Callee::Any),
    ],
    imports: &[(0x6b8, "rand"), (0x914, "rand")],
};

/// [`records_task_0165`] in `magic_0606` (0x10007bd0; its countdown at
/// `S + 0x5ef4`).
pub const RECORDS_TASK_0606: Sig = Sig {
    len: 2882,
    hash: 0x7762_2bae_f56c_82c7,
    calls: RECORDS_TASK_0165.calls,
    imports: RECORDS_TASK_0165.imports,
};

/// `magic_0512`'s second effect task (0x10008fc0, 394 bytes; cdecl `(task)`, `r = task[1]`,
/// its step `+0x18`, the wait `+0x20`):
/// - 0: step 1, the wait 35;
/// - 1: once the wait is down: the effect node `r + 0x5a0`
///   ([`crate::s4_fam446::effect_node_a`], stack `(ecx, r + 0x2bc, ecx,
///   1)` with the entry `ecx`), `+0x5cc |= 1`, `+0x5d0` 0; with no
///   `+0x1c` one particle node (`r + 0x3a0`, `(r + 0x24, 1)`), otherwise
///   two (`r + 0x3a0`, `(r + 0x24, 2)`, and `r + 0x4a0`, `(r + 0x170, 7)`)
///   and the word `+0x576` = `trunc(max(K2 - (r[0x2dc] - r[0x334]) / K1,
///   0) * K3)`; then slot 239 `(*G_A, *G_B as u8, 0)` and the next step;
/// - 2: once bit 2 of `+0x488` is set (and with `+0x1c` that of `+0x588`
///   too), step 10000 and as 10000;
/// - 10000: the task's node `*task` leaves its ring, step 20000.
///
/// The wait goes down at every exit. Returns slot 239's `eax`, the node's
/// next after the unlink, or what is left of the step.
pub fn effect_task_0512<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{div, mul, sub};
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = |g: &G, op: u32| g.read::<f32>(w(op));
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    // push ebp; and esp, -8; push esi; push edi
    let m = fr.esp.wrapping_sub(4) & !7;
    let mut state = g.read::<u32>(r + 0x18);
    if state == 2
        && g.read::<u8>(r + 0x488) & 4 != 0
        && (g.read::<u32>(r + 0x1c) == 0 || g.read::<u8>(r + 0x588) & 4 != 0)
    {
        g.write::<u32>(r + 0x18, 0x2710);
        state = 0x2710;
    }
    let eax = match state {
        0 => {
            let s = g.read::<u32>(r + 0x18);
            g.write::<u32>(r + 0x18, s.wrapping_add(1));
            g.write::<u32>(r + 0x20, 0x23);
            0
        }
        1 if g.read::<i32>(r + 0x20) > 0 => 0,
        1 => {
            let sp = m.wrapping_sub(0x1c);
            for (i, x) in (1u32..).zip([fr.ecx, r + 0x2bc, fr.ecx, 1]) {
                g.write::<u32>(sp + 4 * i, x);
            }
            let callee = Frame {
                at: img.rel32_target(f + 0x79),
                esp: sp,
                ecx: r + 0x5a0,
                ..Frame::default()
            };
            crate::s4_fam446::effect_node_a(g, img, &callee);
            let v = g.read::<u32>(r + 0x5cc);
            g.write::<u32>(r + 0x5cc, v | 1);
            g.write::<u32>(r + 0x5d0, 0);
            if g.read::<u32>(r + 0x1c) == 0 {
                crate::m0512::node_with(g, img, f + 0x113, m, r + 0x3a0, r + 0x24, 1);
            } else {
                crate::m0512::node_with(g, img, f + 0xa7, m, r + 0x3a0, r + 0x24, 2);
                crate::m0512::node_with(g, img, f + 0xc1, m, r + 0x4a0, r + 0x170, 7);
                let d = sub(g.read::<f32>(r + 0x2dc), g.read::<f32>(r + 0x334));
                let x = sub(k(g, 0xd9), div(d, k(g, 0xe4)));
                // comiss 0, x; jbe: below 0 is 0 (unordered stays)
                let x = if 0.0 > x { 0.0 } else { x };
                let v = crate::s4_summon::trunc(mul(x, k(g, 0xfb)));
                #[allow(clippy::cast_possible_truncation)] // the low word
                g.write::<u16>(r + 0x576, v as u16);
            }
            let b = u32::from(g.read::<u8>(w(0x11d)));
            let t = slot_target(g, w(0x125), 239);
            let a = g.read::<u32>(w(0x12b));
            // call width: 3
            let eax = g.call(t, 0, &[a, b, 0]).eax;
            let s = g.read::<u32>(r + 0x18);
            g.write::<u32>(r + 0x18, s.wrapping_add(1));
            eax
        }
        0x2710 => {
            let next = crate::h6_setup::unlink_task(g, task);
            g.write::<u32>(r + 0x18, 0x4e20);
            next
        }
        _ => state,
    };
    let n = g.read::<u32>(r + 0x20);
    g.write::<u32>(r + 0x20, n.wrapping_sub(1));
    Ret::eax(eax)
}

/// [`effect_task_0512`] (r16's draft, own5-hi-work).
pub const EFFECT_TASK_0512: Sig = Sig {
    len: 394,
    hash: 0x01de_fb38_d18f_107b,
    calls: &[
        (0x79, Callee::Sig(&crate::s4_fam446::EFFECT_NODE_A)),
        (0xa7, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0xc1, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
        (0x113, Callee::Sig(&crate::s3_nodes::PARTICLE_NODE_WITH)),
    ],
    imports: &[],
};

/// `magic_0512`'s summon task (0x10009620, 2475 bytes; a task callback
/// `(A)`, cdecl, `R = A[1]`, its state `R + 0x18`, the wait `R + 0x50`;
/// an aligned /GS frame `E`; the states below 0x1f from the byte and jump
/// tables after the code, read from the image). The states run on into
/// each other inside one call where the code jumps back to its dispatch:
/// - 0: the summon's nodes: slot 719 `(NAME, 5, 0)` into slot 306 `(7, 0,
///   h)`; the model node `R + 0x54c` ([`model_node_init`](crate::sa_summon::model_node_init),
///   `[N, P, *G_A, 0]` at `E + 0x40`) and the model nodes `R + 0x5c`
///   ([`model_nodes`](crate::s3_summon::model_nodes), `[N, P, *G_A, 0, 0]`
///   at `E + 4`); children of the static parent `P`
///   ([`child_of_0512`](crate::sa_misc::child_of_0512)) `R + 0x5e4`, `R +
///   0x600` (the model's place, slot 262, copied by slot 325 to `R +
///   0x3c`), `R + 0x610` (the model's state into `R + 0x62c`,
///   [`model_state`](crate::m0070::model_state)), `R + 0x694` (its facing
///   into `R + 0x6b0`, [`facing_int`](crate::sa_misc::facing_int)); the
///   node `R + 0x6c0` inserted ([`node_insert`](crate::nodes::node_insert));
///   two big nodes `R + 0x958`, `R + 0xaac`
///   ([`node_init_big_table`](crate::m0070::node_init_big_table)) with `R +
///   0x5a0` and 0x58 / 0x8c after them; the child `R + 0x63c`; slot 268
///   `(R[0xcc], 0x11, R + 0x658)`, and unless the model takes the direct
///   way ([`model_direct`](crate::s6_misc::model_direct) of `R[0xc8]`) slot
///   12's height into `R + 0x65c`, `R + 0x664` 1.0; the child `R + 0x668`
///   (the state of `R + 0x78` into `R + 0x684`); the node `R + 0x80c`
///   inserted; `R + 0x1c` = `R + 0x658 - R + 0x600` (slot 234), the
///   direction `D` (slot 232 into `E + 0x50`) scaled by slot 251 `(R[0x59c])
///   K` (slot 233) and added back (slot 234); the colour node of the
///   static `G_C` ([`colour_node`](crate::s3_misc::colour_node)) and its
///   colour words (`1 - c` and 0.5s, a flag bit cleared); the effect node
///   `R + 0xc00` ([`effect_node`](crate::sb_nodes::effect_node)) given its
///   vtable, `+0x2c |= 1`, `+0x30` 0; the ring node `R + 0xc38` on the
///   DLL's ring (its handler, `+0x20` the model node); then state 10;
/// - 10: the two effect objects `R + 0xcc0`, `R + 0xfc0`
///   ([`setup`](crate::m0512::setup) with the big nodes and 0 / 1), state
///   11, `R + 0xc50` 0, the wait 45;
/// - 11: once the wait is out, state 0x1e;
/// - 0x1e: the objects `R + 0x12c0`, `R + 0x15c4`
///   ([`setup_from`](crate::m0512::setup_from)), the ground `R + 0x18d0`
///   ([`ground`](crate::m0512::ground), `edx = R + 0x6c0`), the "between"
///   objects `R + 0x1bc0`, `R + 0x2120`
///   ([`between_setup_0512`](crate::r29::between_setup_0512), `(R + 0x63c,
///   0 / 1, d)`) and the two-part object `R + 0x2680`
///   ([`object_setup_0512`](crate::r31::object_setup_0512), `(R + 0x54c,
///   d)`), `d` the direct-way answer; state 0x1f, `R + 0xc50` 0x2710, the
///   wait 60;
/// - 0x1f: once the wait is out, state 0x3c; 0x3c: the wait 0 and state
///   0x3d; 0x3d: once the wait is out, the colour words again (1.0s, the
///   flag word's bit 2 set, bit 0 cleared), state 10000;
/// - 10000: with `R + 0xc50 & 0x4e20` the task's node leaves its ring.
///
/// Every exit counts `R + 0x4c` up and the wait down. Returns `eax` as
/// the original leaves it (the table index, a state, the last call's).
#[allow(clippy::too_many_lines)] // the original's states, in order
pub fn summon_task_0512<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sa_misc::child_of_0512 as child;
    use crate::sb_sse::{mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let gw = |g: &G, o: u32| g.read::<u32>(w(o));
    let slot = |g: &G, n: u32| slot_target(g, w(0x5a), n);
    let call = |o: u32| img.rel32_target(f + o);
    // push ebp; and esp, -8; sub esp, 0x64; three pushes
    let e = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x64);
    let cookie = gw(g, 0xa) ^ e;
    g.write::<u32>(e + 0x60, cookie);
    // the callees' return address below the three pushes
    let inner = e.wrapping_sub(0x10);
    let task = fr.arg(g, 0);
    let r = g.read::<u32>(task + 4);
    g.write::<u32>(e + 0x24, task);
    g.write::<u32>(e + 0x1c, r);
    let at = |o: u32| r.wrapping_add(o);
    let wait = |g: &G| g.read::<i32>(at(0x50));
    // `1 - x` stores of the colour words (the constant `one` in a register)
    let colours = |g: &mut G, one: f32, ops: &[(u32, u32)]| {
        for &(src, dst) in ops {
            let x = sub(one, k(g, src));
            g.write::<f32>(w(dst), x);
        }
    };
    let one = k(g, 0x29);
    let mut eax;
    let mut state = g.read::<u32>(at(0x18));
    loop {
        if state <= 0x1e {
            let idx = img.word(w(0x4e) + state) & 0xff;
            eax = idx;
            match img.word(w(0x55) + 4 * idx).wrapping_sub(f) {
                0x59 => {
                    // call width: 3
                    let h = g.call(slot(g, 719), 0, &[w(0x69), 5, 0]).eax;
                    // call width: 3
                    g.call(slot(g, 306), 0, &[7, 0, h]);
                    let model = at(0x54c);
                    let grp = e + 0x40;
                    let a = gw(g, 0x85);
                    for (i, v) in (0u32..).zip([model, w(0x9e), a]) {
                        g.write::<u32>(grp + 4 * i, v);
                    }
                    g.write::<u8>(grp + 0xc, 0);
                    g.write::<u32>(model, w(0xad));
                    crate::sa_summon::model_node_init(g, img, call(0xb2), grp);
                    let a = gw(g, 0xb7);
                    let grp = e + 4;
                    g.write::<u32>(grp, at(0x5c));
                    g.write::<u32>(at(0x5c), w(0xc4));
                    for (i, v) in (1u32..).zip([w(0xd0), a, 0]) {
                        g.write::<u32>(grp + 4 * i, v);
                    }
                    g.write::<u8>(grp + 0x10, 0);
                    let mn = Frame {
                        at: call(0xe6),
                        esp: inner,
                        ecx: grp,
                        ..Frame::default()
                    };
                    crate::s3_summon::model_nodes(g, img, &mn);
                    let c1 = at(0x5e4);
                    child(
                        g,
                        img,
                        f,
                        c1,
                        [0xff, 0x106, 0x10b, 0x113, 0x118, 0x120, 0x12c, 0x138],
                    );
                    let pl = at(0x600);
                    g.write::<u32>(c1, w(0x14b));
                    let m = g.read::<u32>(model + 0x54);
                    // call width: 4
                    g.call(slot(g, 262), 0, &[m, pl, pl + 4, pl + 8]);
                    g.write::<u32>(at(0x60c), ONE);
                    // call width: 2
                    g.call(slot(g, 325), 0, &[at(0x3c), pl]);
                    let c2 = at(0x610);
                    child(
                        g,
                        img,
                        f,
                        c2,
                        [0x18f, 0x196, 0x19b, 0x1a3, 0x1a8, 0x1b0, 0x1bc, 0x1c8],
                    );
                    g.write::<u32>(c2, w(0x1d2));
                    g.write::<u32>(e + 0x30, model);
                    g.write::<u32>(e + 0x34, at(0x62c));
                    crate::m0070::model_state(g, img, call(0x1df), e + 0x30);
                    let c3 = at(0x694);
                    g.write::<u32>(c3, w(0x1eb));
                    child(
                        g,
                        img,
                        f,
                        c3,
                        [0x1fe, 0x205, 0x20a, 0x212, 0x217, 0x21f, 0x22b, 0x23d],
                    );
                    crate::sa_misc::facing_int(g, img, call(0x242), model, at(0x6b0), inner);
                    for (i, v) in (0u32..).zip([at(0x6c0), w(0x25e), w(0x266), c1, c3, c2]) {
                        g.write::<u32>(grp + 4 * i, v);
                    }
                    let ins = Frame {
                        at: call(0x277),
                        esp: inner,
                        ecx: grp,
                        ..Frame::default()
                    };
                    crate::nodes::node_insert(g, img, &ins);
                    for (node, ops, call_at, (tail, n)) in [
                        (
                            at(0x958),
                            [0x28d, 0x295, 0x29d, 0x2a5],
                            0x2aa,
                            (0xaa4, 0x58),
                        ),
                        (
                            at(0xaac),
                            [0x2d6, 0x2de, 0x2e6, 0x2ee],
                            0x2f3,
                            (0xbf8, 0x8c),
                        ),
                    ] {
                        g.write::<u32>(grp, node);
                        for (i, o) in (1u32..).zip(ops) {
                            g.write::<u32>(grp + 4 * i, w(o));
                        }
                        crate::m0070::node_init_big_table(g, img, call(call_at), grp);
                        let v = g.read::<u32>(at(0x5a0));
                        g.write::<u32>(at(tail), v);
                        g.write::<u32>(at(tail + 4), n);
                    }
                    let c4 = at(0x63c);
                    child(
                        g,
                        img,
                        f,
                        c4,
                        [0x322, 0x329, 0x32e, 0x336, 0x33b, 0x343, 0x34f, 0x355],
                    );
                    g.write::<u32>(c4, w(0x35b));
                    let h = g.read::<u32>(at(0xc8));
                    let md = Frame {
                        at: call(0x366),
                        esp: inner,
                        ecx: h,
                        ..Frame::default()
                    };
                    let direct = crate::s6_misc::model_direct(g, img, &md).eax;
                    let m = g.read::<u32>(at(0xcc));
                    // call width: 3
                    g.call(slot(g, 268), 0, &[m, 0x11, at(0x658)]);
                    g.write::<u32>(at(0x664), ONE);
                    if direct == 0 {
                        let m = g.read::<u32>(at(0xcc));
                        // call width: 1
                        let y = st0_f32(g.call(slot(g, 12), 0, &[m]).st0);
                        g.write::<f32>(at(0x65c), y);
                        g.write::<u32>(at(0x664), ONE);
                    }
                    let c5 = at(0x668);
                    child(
                        g,
                        img,
                        f,
                        c5,
                        [0x3d2, 0x3d9, 0x3de, 0x3e6, 0x3eb, 0x3f3, 0x3ff, 0x412],
                    );
                    g.write::<u32>(e + 0x28, at(0x78));
                    g.write::<u32>(c5, w(0x41c));
                    g.write::<u32>(e + 0x2c, at(0x684));
                    crate::m0070::model_state(g, img, call(0x425), e + 0x28);
                    for (i, v) in (0u32..).zip([at(0x80c), w(0x441), w(0x449), c4, c3, c5]) {
                        g.write::<u32>(grp + 4 * i, v);
                    }
                    let ins = Frame {
                        at: call(0x45a),
                        ..ins
                    };
                    crate::nodes::node_insert(g, img, &ins);
                    let (v, d) = (at(0x1c), e + 0x50);
                    // call width: 3
                    g.call(slot(g, 234), 0, &[v, at(0x658), pl]);
                    // call width: 2
                    g.call(slot(g, 232), 0, &[d, v]);
                    let m = g.read::<u32>(at(0x59c));
                    // call width: 1
                    let s = st0_f32(g.call(slot(g, 251), 0, &[m]).st0);
                    g.write::<f32>(e + 0x20, s);
                    let s = mul(s, k(g, 0x4b4));
                    // call width: 3
                    g.call(slot(g, 233), 0, &[d, d, s.to_bits()]);
                    // call width: 3
                    g.call(slot(g, 234), 0, &[v, v, d]);
                    let cg = e + 0x38;
                    g.write::<u32>(cg, w(0x4f5));
                    g.write::<u32>(cg + 4, w(0x4fd));
                    g.write::<u32>(w(0x503), w(0x507));
                    let cn = Frame {
                        at: call(0x50c),
                        esp: inner,
                        ecx: cg,
                        ..Frame::default()
                    };
                    crate::s3_misc::colour_node(g, img, &cn);
                    let one = k(g, 0x514);
                    let fl = gw(g, 0x51a) & !1;
                    g.write::<u32>(w(0x51a), fl);
                    colours(g, one, &[(0x526, 0x556)]);
                    crate::r25::abs_stores(g, img, f, 0x52a, 0x552);
                    colours(g, one, &[(0x561, 0x59b)]);
                    crate::r25::abs_stores(g, img, f, 0x565, 0x597);
                    let fx = at(0xc00);
                    colours(g, one, &[(0x5a6, 0x5c2), (0x5cd, 0x5f1)]);
                    for (i, v) in (0u32..).zip([fx, w(0x5d5), w(0x5dd), c1, w(0x5e9), 0]) {
                        g.write::<u32>(grp + 4 * i, v);
                    }
                    colours(g, one, &[(0x5fc, 0x60c), (0x617, 0x61f)]);
                    let x = sub(k(g, 0x627), k(g, 0x62f));
                    g.write::<f32>(w(0x637), x);
                    colours(g, one, &[(0x642, 0x64a)]);
                    crate::sb_nodes::effect_node(g, img, call(0x64f), grp);
                    let node = g.read::<u32>(grp);
                    g.write::<u32>(node, w(0x65f));
                    let x = g.read::<u32>(fx + 0x2c) | 1;
                    g.write::<u32>(fx + 0x2c, x);
                    g.write::<u32>(fx + 0x30, 0);
                    // the ring node
                    let rn = at(0xc38);
                    g.write::<u32>(rn, w(0x670));
                    g.write::<u32>(rn + 8, rn);
                    g.write::<u32>(rn + 4, rn);
                    g.write::<u32>(rn + 0x10, w(0x67d));
                    g.write::<u32>(rn + 0x14, rn);
                    let last = gw(g, 0x685);
                    g.write::<u32>(rn + 8, last);
                    g.write::<u32>(rn + 4, w(0x68f));
                    let last = gw(g, 0x694);
                    g.write::<u32>(last.wrapping_add(4), rn);
                    let fl = gw(g, 0x69d) & !1;
                    g.write::<u32>(w(0x69d), fl);
                    g.write::<u32>(w(0x6a4), rn);
                    g.write::<u32>(rn + 0x20, model);
                    g.write::<u32>(rn + 0x18, 0);
                    state = 10;
                    g.write::<u32>(at(0x18), state);
                }
                0x6c4 => {
                    let mut r0 = 0;
                    // one word pushed: the return address below it
                    let sp = inner.wrapping_sub(4);
                    for (obj, src, n, c) in [
                        (0xcc0u32, 0x958u32, 0u32, 0x6d3u32),
                        (0xfc0, 0xaac, 1, 0x6e9),
                    ] {
                        g.write::<u32>(sp + 4, n);
                        let sf = Frame {
                            at: call(c),
                            esp: sp,
                            ecx: at(obj),
                            edx: at(src),
                            ..Frame::default()
                        };
                        r0 = crate::m0512::setup(g, img, &sf);
                    }
                    eax = r0;
                    state += 1;
                    g.write::<u32>(at(0x18), state);
                    g.write::<u32>(at(0xc50), 0);
                    g.write::<u32>(at(0x50), 0x2d);
                    // the wait just set: out at once
                    break;
                }
                0x70c => {
                    if wait(g) > 0 {
                        break;
                    }
                    state = 0x1e;
                    g.write::<u32>(at(0x18), state);
                }
                0x722 => {
                    let h = g.read::<u32>(at(0xc8));
                    let md = Frame {
                        at: call(0x729),
                        esp: inner,
                        ecx: h,
                        ..Frame::default()
                    };
                    let direct = crate::s6_misc::model_direct(g, img, &md).eax;
                    let sp = inner.wrapping_sub(4);
                    for (obj, src, n, c) in [
                        (0x12c0u32, 0x958u32, 0u32, 0x73eu32),
                        (0x15c4, 0xaac, 1, 0x754),
                    ] {
                        g.write::<u32>(sp + 4, n);
                        let sf = Frame {
                            at: call(c),
                            esp: sp,
                            ecx: at(obj),
                            edx: at(src),
                            ..Frame::default()
                        };
                        crate::m0512::setup_from(g, img, &sf);
                    }
                    let gf = Frame {
                        at: call(0x76e),
                        esp: inner,
                        ecx: at(0x18d0),
                        edx: at(0x6c0),
                        ..Frame::default()
                    };
                    crate::m0512::ground(g, img, &gf);
                    let sp = inner.wrapping_sub(0xc);
                    for (obj, b, c) in [(0x1bc0u32, 0u32, 0x789u32), (0x2120, 1, 0x7a1)] {
                        for (i, v) in (1u32..).zip([at(0x63c), b, direct]) {
                            g.write::<u32>(sp + 4 * i, v);
                        }
                        crate::r29::between_setup_0512(g, img, call(c), sp, at(obj), at(0x6c0));
                    }
                    let sp = inner.wrapping_sub(8);
                    g.write::<u32>(sp + 4, at(0x54c));
                    g.write::<u32>(sp + 8, direct);
                    let of = Frame {
                        at: call(0x7be),
                        esp: sp,
                        ecx: at(0x2680),
                        edx: at(0x78),
                        ..Frame::default()
                    };
                    eax = crate::r31::object_setup_0512(g, img, &of).eax;
                    state += 1;
                    g.write::<u32>(at(0x18), state);
                    g.write::<u32>(at(0xc50), 0x2710);
                    g.write::<u32>(at(0x50), 0x3c);
                    // the wait just set: out at once
                    break;
                }
                _ => break,
            }
        } else if state == 0x1f {
            eax = state;
            if wait(g) > 0 {
                break;
            }
            state = 0x3c;
            g.write::<u32>(at(0x18), state);
        } else if state == 0x3c || state == 0x3d {
            if state == 0x3c {
                state = 0x3d;
                g.write::<u32>(at(0x50), 0);
                g.write::<u32>(at(0x18), state);
            }
            eax = state;
            if wait(g) > 0 {
                break;
            }
            let fl = (gw(g, 0x864) & !1) | 4;
            colours(g, one, &[(0x85f, 0x87c)]);
            crate::r25::abs_stores(g, img, f, 0x86e, 0x878);
            colours(g, one, &[(0x887, 0x8b7)]);
            crate::r25::abs_stores(g, img, f, 0x88b, 0x8b3);
            colours(g, one, &[(0x8c2, 0x8f2)]);
            crate::r25::abs_stores(g, img, f, 0x8c6, 0x8ee);
            colours(g, one, &[(0x8fd, 0x914)]);
            g.write::<u32>(w(0x902), fl);
            crate::r25::abs_stores(g, img, f, 0x906, 0x910);
            colours(g, one, &[(0x91f, 0x927), (0x932, 0x93a)]);
            let x = sub(k(g, 0x942), k(g, 0x94a));
            g.write::<f32>(w(0x952), x);
            colours(g, one, &[(0x95d, 0x965)]);
            state = 0x2710;
            g.write::<u32>(at(0x18), state);
        } else if state == 0x2710 {
            eax = state;
            if g.read::<u32>(at(0xc50)) & 0x4e20 != 0 {
                let t = g.read::<u32>(e + 0x24);
                eax = crate::h6_setup::unlink_task(g, t);
            }
            break;
        } else {
            eax = state;
            break;
        }
    }
    let n = g.read::<u32>(at(0x4c)).wrapping_add(1);
    g.write::<u32>(at(0x4c), n);
    let n = g.read::<u32>(at(0x50)).wrapping_sub(1);
    g.write::<u32>(at(0x50), n);
    Ret::eax(eax)
}

/// [`summon_task_0512`] (the `rel32`s: its callees in order,
/// `__security_check_cookie`).
pub const SUMMON_TASK_0512: Sig = Sig {
    len: 2475,
    hash: 0xf8dd_4990_9d54_ee84,
    calls: &[
        (0xb2, Callee::Sig(&crate::sa_summon::MODEL_NODE_INIT)),
        (0xe6, Callee::Sig(&crate::s3_summon::MODEL_NODES)),
        (0x1df, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x242, Callee::Sig(&crate::sa_misc::FACING_INT)),
        (0x277, Callee::Sig(&crate::nodes::INSERT)),
        (0x2aa, Callee::Sig(&crate::m0070::NODE_INIT_BIG_TABLE)),
        (0x2f3, Callee::Sig(&crate::m0070::NODE_INIT_BIG_TABLE)),
        (0x366, Callee::Sig(&crate::s6_misc::MODEL_DIRECT)),
        (0x425, Callee::Sig(&crate::m0070::MODEL_STATE)),
        (0x45a, Callee::Sig(&crate::nodes::INSERT)),
        (0x50c, Callee::Sig(&crate::s3_misc::COLOUR_NODE)),
        (0x64f, Callee::Sig(&crate::sb_nodes::EFFECT_NODE)),
        (0x6d3, Callee::Sig(&crate::m0512::SETUP)),
        (0x6e9, Callee::Sig(&crate::m0512::SETUP)),
        (0x729, Callee::Sig(&crate::s6_misc::MODEL_DIRECT)),
        (0x73e, Callee::Sig(&crate::m0512::SETUP_FROM)),
        (0x754, Callee::Sig(&crate::m0512::SETUP_FROM)),
        (0x76e, Callee::Sig(&crate::m0512::GROUND)),
        (0x789, Callee::Sig(&crate::r29::BETWEEN_SETUP_0512)),
        (0x7a1, Callee::Sig(&crate::r29::BETWEEN_SETUP_0512)),
        (0x7be, Callee::Sig(&crate::r31::OBJECT_SETUP_0512)),
        (0x83f, Callee::Any),
    ],
    imports: &[],
};
