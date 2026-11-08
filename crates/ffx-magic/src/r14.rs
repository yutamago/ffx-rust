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

/// A summon's spark task (cdecl `(task)`, a task callback; a /GS frame of
/// 0x5c bytes, `ebp` = `esp - 4` at the entry; `magic_0132` and `magic_0134`
/// 0x10002210, 934 bytes; `magic_0133` 0x10004ac0 and `magic_0135`
/// 0x10004bc0, 937 bytes: one more `movaps` at `+0x171`, their operands
/// from there on three bytes later). On the summon `S = task[0x14]`:
/// - while the wait `S+0x50c` runs it counts down, and at 0 the object
///   between the actors starts
///   ([`summon_object_between`](crate::s9_misc::summon_object_between),
///   `ecx` = `S`), whose `eax` it returns (else the new wait);
/// - else while the pause `S+0x510` runs it counts down (returned);
/// - else the curve's parameter `t = S+0x4fc` moves on by `S+0x500` (held
///   at 1.0 above `K` `+0x7b`) and the point `L` (`ebp - 0x14`) on the
///   cubic through `S+0x4bc`, `+0x4cc`, `+0x4dc`, `+0x4ec` (`K3` `+0xa2`
///   the middle weights): per axis `((v v u K3) B + v^3 A) + u^2 v K3 C +
///   u^3 D` with `v = K - t`, `L.w` 1.0;
/// - with the burst count `S+0x508` at 0: the fade `S+0x484` halves while
///   above 0 (then the trail), else the task is given back
///   ([`give_back`](crate::sa_pool::give_back), its `eax` returned);
/// - else the count goes down, at 0 the spark burst
///   ([`spark_burst`](crate::s8_misc::spark_burst), `ecx` = `S`); then up
///   to six sparks from the pool
///   ([`pool_take`](crate::sb_misc::pool_take)): entry slot 719 `(PART, 9,
///   1)` at `+0x30`, `+0x34` 1, u16 `+0x38` 0xffff and `+0x3a` 0x200,
///   slot 325 `(n, L)`, the velocity `(R - K4) K5` from three slot 307
///   ST0s at `+0x10..`, the rest the code's immediates;
/// - the trail: slot 235 makes the matrix `M` (`ebp - 0x54`), its
///   diagonal 0.03, and the trail draws
///   ([`trail_draw`](crate::s4_misc::trail_draw), `ecx` = `S+0x260`, `edx`
///   = `M`, the stack word `L`), whose `eax` it returns.
#[allow(clippy::too_many_lines)] // the original's ways, one by one
pub fn spark_task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::m0018::st0_f32;
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    // `movaps xmm1, xmm6` (0x0f) or `addss xmm2, xmm0` (0xf3)
    let d = if img.word(f + 0x171) & 0xff == 0x0f {
        3
    } else {
        0
    };
    let m = |o: u32| if o >= 0x171 { o + d } else { o };
    let w = |o: u32| img.word(f + m(o));
    let k = |g: &G, o: u32| g.read::<f32>(w(o));
    let call = |o: u32| img.rel32_target(f + m(o));
    let task = fr.arg(g, 0);
    let s = g.read::<u32>(task + 0x14);
    let ebp = fr.esp.wrapping_sub(4);
    // push ebp, the 0x5c-byte frame, edi; the return address
    let inner = Frame {
        esp: ebp.wrapping_sub(0x64),
        ecx: s,
        ..*fr
    };
    let wait = g.read::<i32>(s + 0x50c);
    if wait > 0 {
        let left = wait - 1;
        g.write::<i32>(s + 0x50c, left);
        if left != 0 {
            return Ret::eax(left.cast_unsigned());
        }
        let between = Frame {
            at: call(0x34),
            ..inner
        };
        return crate::s9_misc::summon_object_between(g, img, &between);
    }
    let pause = g.read::<i32>(s + 0x510);
    if pause > 0 {
        let left = pause - 1;
        g.write::<i32>(s + 0x510, left);
        return Ret::eax(left.cast_unsigned());
    }
    let one = k(g, 0x7b);
    let t = add(g.read::<f32>(s + 0x500), g.read::<f32>(s + 0x4fc));
    g.write::<f32>(s + 0x4fc, t);
    if t > one {
        g.write::<u32>(s + 0x4fc, 0x3f80_0000);
    }
    let three = k(g, 0xa2);
    let p = |g: &G, o: u32| g.read::<f32>(s + o);
    let l = ebp.wrapping_sub(0x14);
    for axis in 0..3u32 {
        let u = p(g, 0x4fc);
        let v = sub(one, u);
        let uu = mul(u, u);
        let a = mul(mul(v, v), v);
        let b = mul(mul(mul(v, u), v), three);
        let first = add(
            mul(b, p(g, 0x4cc + 4 * axis)),
            mul(a, p(g, 0x4bc + 4 * axis)),
        );
        let c = mul(mul(uu, v), three);
        let x = add(
            add(first, mul(c, p(g, 0x4dc + 4 * axis))),
            mul(mul(uu, u), p(g, 0x4ec + 4 * axis)),
        );
        g.write::<f32>(l + 4 * axis, x);
    }
    g.write::<u32>(l + 0xc, 0x3f80_0000);
    let count = g.read::<u32>(s + 0x508);
    if count == 0 {
        let fade = g.read::<i32>(s + 0x484);
        if fade <= 0 {
            return Ret::eax(crate::sa_pool::give_back(g, img, call(0x217), task));
        }
        g.write::<i32>(s + 0x484, fade / 2);
    } else {
        let count = count.wrapping_sub(1);
        g.write::<u32>(s + 0x508, count);
        if count == 0 {
            let burst = Frame {
                at: call(0x236),
                ..inner
            };
            crate::s8_misc::spark_burst(g, img, &burst);
        }
        for _ in 0..6 {
            let n = crate::sb_misc::pool_take(g, img, call(0x242));
            if n == 0 {
                break;
            }
            // call width: 3
            let e = g
                .call(slot_target(g, w(0x252), 719), 0, &[w(0x261), 9, 1])
                .eax;
            g.write::<u32>(n + 0x30, e);
            g.write::<u16>(n + 0x3a, 0x200);
            g.write::<u32>(n + 0x34, 1);
            g.write::<u16>(n + 0x38, 0xffff);
            // call width: 5 (slot 719's words under it)
            g.call(slot_target(g, w(0x278), 325), 0, &[n, l, w(0x261), 9, 1]);
            for (o, (tab, k1, k2)) in [
                (0x10u32, (0x291u32, 0x2ac, 0x2b9)),
                (0x14, (0x2b1, 0x2d6, 0x2e3)),
                (0x18, (0x2db, 0x300, 0x31e)),
            ] {
                // call width: 0
                let r = st0_f32(g.call(slot_target(g, w(tab), 307), 0, &[]).st0);
                let x = mul(sub(r, k(g, k1)), k(g, k2));
                if o == 0x18 {
                    // `mov dword [esi + x], imm` (`magic_0133`/`0135`'s
                    // `+0x24` is -0.02, the others' 0)
                    for (z, imm) in [
                        (0x1cu32, 0x308u32),
                        (0x20, 0x30f),
                        (0x24, 0x316),
                        (0x28, 0x325),
                        (0x2c, 0x32c),
                    ] {
                        g.write::<u32>(n + z, w(imm));
                    }
                }
                g.write::<f32>(n + o, x);
            }
            for (z, imm) in [(0x3cu32, 0x338u32), (0x4c, 0x33f), (0x50, 0x346)] {
                g.write::<u32>(n + z, w(imm));
            }
            g.write::<u8>(n + 0x6b, w(0x34d) as u8);
        }
    }
    let mat = ebp.wrapping_sub(0x54);
    // call width: 1
    g.call(slot_target(g, w(0x361), 235), 0, &[mat]);
    for o in [0x28u32, 0x14, 0] {
        g.write::<u32>(mat + o, 0x3cf5_c28f);
    }
    // the stack words: `L` over `M` (slot 235's, not popped)
    let esp = ebp.wrapping_sub(0x6c);
    g.write::<u32>(esp + 4, l);
    g.write::<u32>(esp + 8, mat);
    let trail = Frame {
        at: call(0x390),
        esp,
        ecx: s + 0x260,
        edx: mat,
        ..*fr
    };
    Ret::eax(crate::s4_misc::trail_draw(g, img, &trail))
}

/// [`spark_task`] in `magic_0132` (0x10002210, 934 bytes; its own object
/// between and spark burst).
pub const SPARK_TASK_0132: Sig = Sig {
    len: 934,
    hash: 0xe954_e609_449f_510a,
    calls: &[
        (
            0x34,
            Callee::Sig(&crate::s9_misc::SUMMON_OBJECT_BETWEEN_0132),
        ),
        (0x3f, Callee::Any),
        (0x5f, Callee::Any),
        (0x217, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x222, Callee::Any),
        (0x236, Callee::Sig(&crate::s8_misc::SPARK_BURST_0132)),
        (0x242, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x390, Callee::Sig(&crate::s4_misc::TRAIL_DRAW)),
        (0x39e, Callee::Any),
    ],
    imports: &[],
};

/// [`spark_task`] in `magic_0134` (0x10002210, 934 bytes).
pub const SPARK_TASK_0134: Sig = Sig {
    hash: 0xe954_e609_449f_510a,
    calls: &[
        (0x34, Callee::Sig(&crate::s9_misc::SUMMON_OBJECT_BETWEEN)),
        (0x3f, Callee::Any),
        (0x5f, Callee::Any),
        (0x217, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x222, Callee::Any),
        (0x236, Callee::Sig(&crate::s8_misc::SPARK_BURST)),
        (0x242, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x390, Callee::Sig(&crate::s4_misc::TRAIL_DRAW)),
        (0x39e, Callee::Any),
    ],
    ..SPARK_TASK_0132
};

/// [`spark_task`] in `magic_0135` (0x10004bc0, 937 bytes).
pub const SPARK_TASK_0135: Sig = Sig {
    len: 937,
    hash: 0x97ca_deab_be09_f397,
    calls: &[
        (0x34, Callee::Sig(&crate::s9_misc::SUMMON_OBJECT_BETWEEN)),
        (0x3f, Callee::Any),
        (0x5f, Callee::Any),
        (0x21a, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x225, Callee::Any),
        (0x239, Callee::Sig(&crate::s8_misc::SPARK_BURST)),
        (0x245, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x393, Callee::Sig(&crate::s4_misc::TRAIL_DRAW)),
        (0x3a1, Callee::Any),
    ],
    imports: &[],
};

/// [`spark_task`] in `magic_0133` (0x10004ac0, 937 bytes).
pub const SPARK_TASK_0133: Sig = Sig {
    hash: 0x97ca_deab_be09_f397,
    calls: &[
        (
            0x34,
            Callee::Sig(&crate::s9_misc::SUMMON_OBJECT_BETWEEN_0133),
        ),
        (0x3f, Callee::Any),
        (0x5f, Callee::Any),
        (0x21a, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
        (0x225, Callee::Any),
        (0x239, Callee::Sig(&crate::s8_misc::SPARK_BURST_0133)),
        (0x245, Callee::Sig(&crate::sb_misc::POOL_TAKE)),
        (0x393, Callee::Sig(&crate::s4_misc::TRAIL_DRAW)),
        (0x3a1, Callee::Any),
    ],
    ..SPARK_TASK_0135
};
