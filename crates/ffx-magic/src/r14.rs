//! Round 14's own ports: functions whose closest ported kin differs in
//! its control flow or stack layout, written out for their DLL.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `magic_0384`'s launches (0x10001b60, an overlay table entry, `(arg)`;
/// an 8-aligned frame of 0x1c bytes, 454 bytes): as `magic_0421`'s
/// [`launches_p055`](crate::r11_misc::launches_p055), with the first
/// object's callback by the count (`CB1` `+0x6c`, or `CB1b` `+0x73` while
/// `*N` is below the imm8 `+0x62`, 0x19), slot 482's words popped before
/// it, and the bound above which the argument's word gets bit 2 0x32 (the
/// imm8 `+0x1b8`). The first launch's word under its call is `ecx` as
/// slot 434 left it (`push ecx`; nothing reads it): the port writes 0
/// there.
pub fn launches_0384<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{cvt, div};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let imm8 = |o: u32| i32::from((w(o) as u8).cast_signed());
    // push ebp; and esp, -8; sub esp, 0x1c
    let base = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x1c);
    let n = g.read::<u32>(w(0x0d));
    let q = div(cvt(n), g.read::<f32>(w(0x1d)));
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[
        1,
        q.to_bits(),
        g.read::<u32>(w(0x37)),
        g.read::<u32>(w(0x45)),
    ]);
    // call width: 12 (Pushes caps the words at 12)
    let level = crate::m0018::st0_f32(s.call(g, slot_target(g, w(0x15), 0x788 / 4), 0).st0);
    g.write::<f32>(base, level);
    let cb = if g.read::<i32>(w(0x5e)) < imm8(0x62) {
        w(0x73)
    } else {
        w(0x6c)
    };
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[cb, g.read::<u32>(w(0x79))]);
    // call width: 12 (Pushes caps the words at 12)
    let first = s.call(g, slot_target(g, w(0x55), 0x6c8 / 4), 0).eax;
    let launch = |g: &mut G, obj: u32, edx: u32, call: u32, under: u32| {
        let fr2 = Frame {
            at: img.rel32_target(f + call),
            esp: base.wrapping_sub(0xc),
            ecx: obj,
            edx,
            xmm: [0, u128::from(level.to_bits()), 0, 0],
            ..*fr
        };
        g.write::<u32>(base.wrapping_sub(8), under);
        crate::o6_fam_launch::random_launch(g, img, fr2.at, &fr2);
    };
    launch(g, first, 0, 0x90, 0);
    // test byte [N], 0xf (the count read again)
    if g.read::<u8>(w(0x99)).trailing_zeros() >= 4 {
        let fields = [0xd0u32, 0xd4, 0xd8, 0xe0, 0xe4, 0xe8];
        let v: [u32; 6] = core::array::from_fn(|i| g.read::<u32>(first + fields[i]));
        let t = g.read::<u32>(w(0xad));
        let mut s = crate::pushes::Pushes::default();
        s.push_all(&[w(0xfe), g.read::<u32>(w(0x104))]);
        // call width: 12 (Pushes caps the words at 12)
        let second = s.call(g, g.read::<u32>(t + 0x6c8), 0).eax;
        for (o, x) in fields.into_iter().zip(v) {
            g.write::<u32>(second + o, x);
        }
        g.write::<u32>(second + 0xdc, 0);
        g.write::<u32>(second + 0xec, 0);
    }
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(0x181), g.read::<u32>(w(0x187))]);
    // call width: 12 (Pushes caps the words at 12)
    let third = s.call(g, slot_target(g, w(0x17c), 0x6c8 / 4), 0).eax;
    launch(g, third, 1, 0x1a4, w(0x181));
    let count = g.read::<u32>(w(0x1a9)).wrapping_add(1);
    g.write::<u32>(w(0x1b2), count);
    if count.cast_signed() > imm8(0x1b8) {
        let arg = fr.arg(g, 0);
        let v = g.read::<u32>(arg);
        g.write::<u32>(arg, v | 4);
        return Ret::eax(arg);
    }
    Ret::eax(count)
}

/// [`launches_0384`] (the launch's `Sig`: `magic_0384`'s).
pub const LAUNCHES_0384: Sig = Sig {
    len: 454,
    hash: 0xa491_bc0c_96cf_584a,
    calls: &[
        (
            0x90,
            Callee::Sig(&crate::o6_fam_launch::RANDOM_LAUNCH_0384_15E0),
        ),
        (
            0x1a4,
            Callee::Sig(&crate::o6_fam_launch::RANDOM_LAUNCH_0384_15E0),
        ),
    ],
    imports: &[],
};

/// `magic_0146`'s summon particle start (0x10007a10, a table entry, cdecl
/// `(actor)`; 450 bytes): [`summon_ppp_start`](crate::s5_misc::summon_ppp_start)'s
/// head, then its record and ring links with the earlier calls' words
/// still on the stack (the operands to the ring's tail three bytes
/// earlier, those after slot 575's table load two bytes later), `G_DONE
/// = 0` and slot 575 (`()`, after the stack is popped). Returns 0.
pub fn summon_ppp_start_0146<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    crate::s5_misc::summon_ppp_head(g, img, fr);
    let tail = g.read::<u32>(w(0x15a));
    g.write::<u32>(w(0x15f), tail);
    for (dst, v) in [
        (0x165u32, 0x169u32),
        (0x16f, 0x173),
        (0x179, 0x17d),
        (0x183, 0x187),
    ] {
        g.write::<u32>(w(dst), w(v));
    }
    g.write::<u32>(tail.wrapping_add(4), w(0x18e));
    let t = g.read::<u32>(w(0x193));
    let flags = g.read::<u32>(w(0x199)) & 0xffff_fffe;
    g.write::<u32>(w(0x199), flags);
    g.write::<u32>(w(0x1a0), w(0x1a4));
    g.write::<u32>(w(0x1aa), 0);
    // call width: 0 (`add esp, 0x4c` before the call)
    g.call(g.read::<u32>(t + 0x8fc), 0, &[]);
    Ret::eax(0)
}

/// [`summon_ppp_start_0146`] (the `rel32`: the targets).
pub const SUMMON_PPP_START_0146: Sig = Sig {
    len: 450,
    hash: 0xbfc4_bb18_c6b0_54cf,
    calls: &[(0x8, Callee::Sig(&crate::s4_summon::TARGETS_INIT))],
    imports: &[],
};

/// `magic_0367`'s main object at its actor (no arguments; 0x100014d0, 225
/// bytes): as `magic_0369`'s
/// [`main_object_0369`](crate::r11_misc::main_object_0369), the object
/// started plain ([`start_object_plain`](crate::s4_misc::start_object_plain),
/// index 0, the `rel32` `+0x18`) and the operands two bytes later; the
/// yaw (wrapped or not) always goes to `G_YAW` (`+0xc5`), and then the
/// two words at `+0xcb` and `+0xd5` get their immediates (-5.0). Returns
/// slot 333's `eax`.
pub fn main_object_0367<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::m0018::st0_f32;
    use crate::sb_sse::{add, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    crate::s4_misc::start_object_plain(g, img, img.rel32_target(f + 0x18), w(0x11), k(g, 0x08), 0);
    let actor = g.read::<u32>(w(0x23));
    let mut s = crate::pushes::Pushes::default();
    s.push_all(&[w(0x2e), w(0x33), w(0x38), actor]);
    // call width: 12 (Pushes caps the words at 12)
    s.call(g, slot_target(g, w(0x1d), 0x418 / 4), 0);
    s.push(actor);
    // call width: 12 (Pushes caps the words at 12)
    let top = st0_f32(s.call(g, slot_target(g, w(0x40), 0x30 / 4), 0).st0);
    g.write::<f32>(w(0x4c), top);
    let y = sub(g.read::<f32>(w(0x54)), k(g, 0x5c));
    let t333 = g.read::<u32>(w(0x61));
    s.push(g.read::<u32>(w(0x67)));
    g.write::<u32>(w(0x6d), 0x3f80_0000);
    g.write::<f32>(w(0x79), y);
    // call width: 12 (Pushes caps the words at 12)
    let out = s.call(g, g.read::<u32>(t333.wrapping_add(0x534)), 0);
    let x = sub(k(g, 0x8c), st0_f32(out.st0));
    // `comiss` + `jbe`: a NaN stays
    let yaw = if x > k(g, 0x9c) {
        sub(x, k(g, 0xa6))
    } else if k(g, 0xb0) > x {
        add(x, k(g, 0xbd))
    } else {
        x
    };
    g.write::<f32>(w(0xc5), yaw);
    g.write::<u32>(w(0xcb), w(0xcf));
    g.write::<u32>(w(0xd5), w(0xd9));
    Ret::eax(out.eax)
}

/// [`main_object_0367`] (`magic_0367` 0x100014d0).
pub const MAIN_OBJECT_0367: Sig = Sig {
    len: 225,
    hash: 0x1992_3e31_1463_acc1,
    calls: &[(0x18, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN))],
    imports: &[],
};

/// `magic_0430`'s light restart (0x10002d80, an overlay table entry, no
/// arguments; a cookie frame, 273 bytes): as `magic_0588`'s
/// [`light_restart`](crate::m0588::light_restart) on its own block
/// offsets, the scale task of `*G_FX + 0xe3c` taking its count in `edx`
/// (0x4b; [`scale_task_edx`](crate::s4_small::scale_task_edx)),
/// `+0xe48` 0 and `+0xe58` 1.0; after the list removal `+0x924` grows by
/// `K` (`+0x8d`) and `+0x204`/`+0xd4` take `+0xcc8`; slot 332 gets the
/// four floats at `+0xbd`, `+0xca`, `+0xd7`, `+0xe7`. Returns slot 332's
/// `eax`.
pub fn light_restart_0430<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x1c));
    // push ebp; sub esp, 0x14; push esi, edi, ecx; the return address
    let ebp = fr.esp.wrapping_sub(4);
    let esp = ebp.wrapping_sub(0x14 + 0x10);
    g.write::<u32>(esp + 4, fr.ecx);
    let mut xmm = fr.xmm;
    xmm[1] = 0;
    xmm[2] = u128::from(g.read::<u32>(w(0x14)));
    let task = Frame {
        at: img.rel32_target(f + 0x30),
        esp,
        ecx: fx.wrapping_add(0xe3c),
        edx: w(0x22),
        xmm,
        ..*fr
    };
    crate::s4_small::scale_task_edx(g, img, &task);
    g.write::<u32>(fx + 0xe48, 0);
    g.write::<u32>(fx + 0xe58, 0x3f80_0000);
    let obj = fx.wrapping_add(0x980);
    if crate::l5_objhost::list_remove(g, img, img.rel32_target(f + 0x59), w(0x40), obj) != 0 {
        g.write::<u32>(g.read::<u32>(w(0x62)), w(0x69));
        let t = slot_target(g, w(0x6e), 713);
        g.call(t, 0, &[obj]);
        let v = g.read::<u16>(obj + 0x124);
        g.write::<u16>(obj + 0x124, v & 0x7fff);
    }
    let x = crate::sb_sse::add(g.read::<f32>(fx + 0x924), g.read::<f32>(w(0x95)));
    let y = g.read::<u32>(fx + 0xcc8);
    g.write::<u32>(fx + 0x204, y);
    let y = g.read::<u32>(fx + 0xcc8);
    g.write::<u32>(fx + 0xd4, y);
    g.write::<f32>(fx + 0x924, x);
    let local = ebp.wrapping_sub(0x14);
    for (i, op) in [0xbdu32, 0xca, 0xd7, 0xe7].into_iter().enumerate() {
        let v = g.read::<u32>(w(op));
        g.write::<u32>(local + 4 * i as u32, v);
    }
    let t = slot_target(g, w(0xed), 332);
    Ret::eax(g.call(t, 0, &[local]).eax)
}

/// [`light_restart_0430`] (the `rel32`s: the scale task, the list removal
/// and the cookie check).
pub const LIGHT_RESTART_0430: Sig = Sig {
    len: 273,
    hash: 0x07c5_acf4_98b4_3303,
    calls: &[
        (0x30, Callee::Sig(&crate::s4_small::SCALE_TASK_EDX)),
        (0x59, Callee::Sig(&crate::l5_objhost::LIST_REMOVE)),
        (0x109, Callee::Any),
    ],
    imports: &[],
};

/// `magic_0288`'s task step (0x10001540, a task callback `(task)`; a /GS
/// frame of 0x14 bytes, 714 bytes). Each frame:
/// - two timers `*G_A` (`+0x11`) and `*G_B` (`+0x98`) count down while
///   above 0; when one reaches 0 its object (`+0x36`, `+0xb3`) starts
///   plain ([`start_object_plain`](crate::s4_misc::start_object_plain),
///   index 0 and 1, the scale `K` at `+0x19`) and slot 262 `(T[sel], P.x,
///   P.y, P.z)` fills its point `P` (the actor byte `**G_SEL`, the table
///   `T` at `+0x59` / `+0xd1`); the first then lowers its `P.y` by `K1`
///   (`+0x76`), the second takes `P.y` from slot 12 `(T[sel])`'s ST0 less
///   `K2` (`+0x105`); both set `P.w` 1.0;
/// - a counter `*G_C` (`+0x11f`): from 0 down to -15 slot 250 `(L, *G_L)`
///   with the light `L` = `K3` (`+0x134`) on odd counts, `K4` (`+0x13e`)
///   on even ones, also stored in `G_L2` (`+0x15c`; `*G_L` at `+0x14b`);
///   then (and above 0 or at -16 and below) the counter, read again, goes
///   down by one;
/// - the objects run ([`run_objects`](crate::l5_objhost::run_objects)),
///   the two motion scripts at `+0x183` (0x20 apart, up to `+0x193`)
///   step ([`motion_script`](crate::s4_misc::motion_script)), slot 262
///   `(*G_AC, &x, &y, &z)` gives the camera point `Q` (`+0x1db`: `x`, `y`,
///   `z`, 1.0) and its script steps
///   ([`obj_script`](crate::l5_objhost::obj_script)), whose `eax` is bit 15
///   of `Q + 0x30`;
/// - when the objects are done (1), the script stopped (1) and the two
///   scripts' done words (`+0x213`, `+0x21f`) are 1: slot 278 `(T[sel],
///   k, 1)` for `k` 0, 1, 2, slot 250 `(0, 0)`, the task's `+0x14` 3 and
///   its record's callback `+0x10` the countdown (`+0x2b7`).
///
/// Returns the record (`task + 8`) on that last way, else the script's
/// `eax`.
#[allow(clippy::too_many_lines)] // the original's steps, one by one
pub fn task_0288<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::m0018::st0_f32;
    use crate::sb_sse::sub;
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let task = fr.arg(g, 0);
    let sel = |g: &G, o: u32| u32::from(g.read::<u8>(g.read::<u32>(w(o))));
    let tab = |g: &G, o: u32, s: u32| slot_target(g, w(o), s);
    let scale = k(g, 0x19);
    // timer A
    let a = g.read::<i32>(w(0x11));
    if a > 0 {
        g.write::<i32>(w(0x27), a - 1);
        if a == 1 {
            crate::s4_misc::start_object_plain(
                g,
                img,
                img.rel32_target(f + 0x3b),
                w(0x36),
                scale,
                0,
            );
            let t = g.read::<u32>(w(0x59) + 4 * sel(g, 0x40));
            // call width: 4
            g.call(tab(g, 0x5e, 262), 0, &[t, w(0x52), w(0x4d), w(0x45)]);
            let y = sub(k(g, 0x6e), k(g, 0x76));
            g.write::<u32>(w(0x87), 0x3f80_0000);
            g.write::<f32>(w(0x93), y);
        }
    }
    // timer B
    let b = g.read::<i32>(w(0x98));
    if b > 0 {
        g.write::<i32>(w(0xa4), b - 1);
        if b == 1 {
            crate::s4_misc::start_object_plain(
                g,
                img,
                img.rel32_target(f + 0xb8),
                w(0xb3),
                scale,
                1,
            );
            let t = g.read::<u32>(w(0xd1) + 4 * sel(g, 0xbd));
            // call width: 4
            g.call(tab(g, 0xd6, 262), 0, &[t, w(0xdb), w(0xca), w(0xc2)]);
            // call width: 5 (slot 262's words under it)
            let top = g.call(tab(g, 0xe9, 12), 0, &[t, t, w(0xdb), w(0xca), w(0xc2)]);
            g.write::<f32>(w(0xf5), st0_f32(top.st0));
            let y = sub(k(g, 0xfd), k(g, 0x105));
            g.write::<u32>(w(0x10e), 0x3f80_0000);
            g.write::<f32>(w(0x11a), y);
        }
    }
    // the light's counter
    let mut c = g.read::<i32>(w(0x11f));
    if c <= 0 && c > -16 {
        let light = if c & 1 != 0 { k(g, 0x134) } else { k(g, 0x13e) };
        let l = g.read::<u32>(w(0x14b));
        g.write::<f32>(w(0x15c), light);
        // call width: 2
        g.call(tab(g, 0x143, 250), 0, &[light.to_bits(), l]);
        c = g.read::<i32>(w(0x16e));
    }
    g.write::<i32>(w(0x177), c.wrapping_sub(1));
    // push ebp, the 0x14-byte frame, ebx, esi, edi; the return address
    let done = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x17c),
        fr.esp.wrapping_sub(0x28),
    );
    let mut m = w(0x183);
    while m < w(0x193) {
        crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x18a), m);
        m += 0x20;
    }
    let ebp = fr.esp.wrapping_sub(4);
    let (x, y, z) = (
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x10),
        ebp.wrapping_sub(0xc),
    );
    let actor = g.read::<u32>(w(0x1ac));
    // call width: 4
    g.call(tab(g, 0x1a6, 262), 0, &[actor, x, y, z]);
    let q = w(0x1db);
    for (dst, src) in [(0x1c1u32, x), (0x1ce, y)] {
        let v = g.read::<u32>(src);
        g.write::<u32>(w(dst), v);
    }
    g.write::<u32>(ebp.wrapping_sub(8), 0x3f80_0000);
    let v = g.read::<u32>(z);
    g.write::<u32>(w(0x1ea), v);
    g.write::<u32>(w(0x1f0), 0x3f80_0000);
    crate::l5_objhost::obj_script(g, img, img.rel32_target(f + 0x1f9), q);
    // (its `eax`: bit 15 of the flags, which every way out computes)
    let stopped = u32::from(g.read::<u16>(q + 0x30) >> 15);
    if done == 1 && stopped == 1 && g.read::<u32>(w(0x213)) == 1 && g.read::<u32>(w(0x21f)) == 1 {
        let mut s = crate::pushes::Pushes::default();
        for (sel_at, t_at, tab_at, kind) in [
            (0x22bu32, 0x237u32, 0x23cu32, 0u32),
            (0x249, 0x257, 0x25c, 1),
            (0x269, 0x277, 0x27c, 2),
        ] {
            let t = g.read::<u32>(w(t_at) + 4 * sel(g, sel_at));
            s.push_all(&[1, kind, t]);
            // call width: 12 (Pushes caps the words at 12)
            s.call(g, tab(g, tab_at, 278), 0);
        }
        // add esp, 0x1c: two words left, both written over with 0
        s.pop(9);
        s.push_all(&[0, 0]);
        // call width: 12 (Pushes caps the words at 12)
        s.call(g, tab(g, 0x289, 250), 0);
        g.write::<u32>(task + 0x14, 3);
        let rec = g.read::<u32>(task + 8);
        g.write::<u32>(rec + 0x10, w(0x2b7));
        return Ret::eax(rec);
    }
    Ret::eax(stopped)
}

/// [`task_0288`] (the `rel32`s: two plain object starts, the objects'
/// run, the motion script, the object script and the cookie check).
pub const TASK_0288: Sig = Sig {
    len: 714,
    hash: 0xff29_8c42_da75_d50f,
    calls: &[
        (0x3b, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0xb8, Callee::Sig(&crate::s4_misc::START_OBJECT_PLAIN)),
        (0x17c, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        (0x18a, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x1f9, Callee::Sig(&crate::l5_objhost::OBJ_SCRIPT)),
        (0x2c2, Callee::Any),
    ],
    imports: &[],
};
