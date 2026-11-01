//! `magic_0449.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x10005450, no arguments, a /GS frame of 0x14
/// bytes): the objects run ([`run_objects`](crate::l5_objhost::run_objects));
/// when they answer 1 and `E + 0x56fc` is 1 (`E` = `*G`, `+0x13`, read
/// first): unless the word `+0x2b` is set, an actor `i` (`*I`, `+0x3b`)
/// below 32 gets slot 217 `(i, &M)` and slot 255 `(i, &M)` (`M` on the
/// frame, `ebp - 0x14`); then slot 211 `(m, 0)` and slot 214 `(*X, m, 0)`
/// (`m` = `*Q`, `+0x33`, read before the test; `X` `+0x82`). Returns the
/// last slot's `eax`, else the objects' answer or 1. Each slot call
/// passes the words the original leaves on the stack.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let e = g.read::<u32>(w(0x13));
    // push ebp; sub esp, 0x14; push esi; the call's return address
    let r = crate::l5_objhost::run_objects(
        g,
        img,
        img.rel32_target(f + 0x18),
        fr.esp.wrapping_sub(0x20),
    );
    if r != 1 || g.read::<u32>(e + 0x56fc) != 1 {
        return Ret::eax(r);
    }
    let m = g.read::<u32>(w(0x33));
    let mut p = crate::pushes::Pushes::default();
    if g.read::<u32>(w(0x2b)) == 0 {
        let i = g.read::<u32>(w(0x3b));
        if i.cast_signed() < 0x20 {
            let buf = ebp.wrapping_sub(0x14);
            p.push_all(&[buf, i]);
            let s = slot_target(g, w(0x49), 0x364 / 4);
            // call width: 12 (Pushes caps the words at 12)
            p.call(g, s, 0);
            p.push_all(&[buf, i]);
            let s = slot_target(g, w(0x5b), 0x3fc / 4);
            // call width: 12 (Pushes caps the words at 12)
            p.call(g, s, 0);
            p.pop(4);
        }
    }
    p.push_all(&[0, m]);
    let s = slot_target(g, w(0x6c), 0x34c / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push(g.read::<u32>(w(0x82)));
    let s = slot_target(g, w(0x7c), 0x358 / 4);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(p.call(g, s, 0).eax)
}

/// [`frame`] (161 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 161,
    hash: 0x18af_6997_c1c8_0f17,
    calls: &[
        (0x18, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        // __security_check_cookie
        (0x99, Callee::Any),
    ],
    imports: &[],
};

/// The camera's start (0x10005500, no arguments): with `E` = `*G`
/// (`+0x08`) and `C` = `E + 0xf70`: slot 262 `(*A, C, C + 4, C + 8)`
/// (`A` `+0x22`), `C + 0xc` = 1.0, slot 333 `(*A)`'s angle `a` (`A`
/// read again, `+0x3c`) turned into `h = K - a` (`K` `+0x4f`) wrapped once by `T` (`+0x68`/`+0x7f`)
/// above `HI` (`+0x5e`) or below `LO` (`+0x72`) into `E + 0x56dc`; `E +
/// 0x56f8` = slot 622 `(*S, *B)` (`S` `+0x9b`, `B` = the byte at `*P`,
/// `+0x8c`), `E + 0x56f4` = -1, `E + 0x56fc` = 0 and `*D` (`+0xc7`) = 1.
/// Returns slot 622's `eax`. The first two slot calls pass the words the
/// original leaves on the stack.
pub fn camera_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let (e, _) = camera_angle(g, img, f, 0xf70, 0x56dc);
    let b = u32::from(g.read::<u8>(g.read::<u32>(w(0x8c))));
    let s = slot_target(g, w(0x95), 0x9b8 / 4);
    let v = g.read::<u32>(w(0x9b));
    let r = g.call(s, 0, &[v, b]).eax;
    g.write::<u32>(e + 0x56f8, r);
    g.write::<u32>(e + 0x56f4, u32::MAX);
    g.write::<u32>(e + 0x56fc, 0);
    g.write::<u32>(w(0xc7), 1);
    Ret::eax(r)
}

/// The camera's angle (shared by [`camera_start`] and `m0431`'s camera
/// start, the same operands up to `+0x83`): with `E` = `*G` (`+0x08`)
/// and `C` = `E + c_off`: slot 262 `(*A, C, C + 4, C + 8)` (`A` `+0x22`),
/// `C + 0xc` = 1.0, slot 333 `(*A)`'s angle `a` (`A` read again,
/// `+0x3c`) turned into `h = K - a` (`K` `+0x4f`) wrapped once by `T`
/// (`+0x68`/`+0x7f`) above `HI` (`+0x5e`) or below `LO` (`+0x72`) into
/// `E + h_off`. Returns `E` and slot 333's `eax`. The two slot calls pass
/// the words the original leaves on the stack.
pub(crate) fn camera_angle<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    c_off: u32,
    h_off: u32,
) -> (u32, u32) {
    use crate::sb_sse::{add, sub};
    let w = |off: u32| img.word(f + off);
    let e = g.read::<u32>(w(0x08));
    let c = e + c_off;
    let mut p = crate::pushes::Pushes::default();
    p.push_all(&[c + 8, c + 4, c, g.read::<u32>(w(0x22))]);
    let s = slot_target(g, w(0x1b), 0x418 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    g.write::<u32>(c + 0xc, 0x3f80_0000);
    p.push(g.read::<u32>(w(0x3c)));
    let s = slot_target(g, w(0x2f), 0x534 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let out = p.call(g, s, 0);
    let a = crate::m0018::st0_f32(out.st0);
    let h = sub(g.read::<f32>(w(0x4f)), a);
    let h = if h > g.read::<f32>(w(0x5e)) {
        sub(h, g.read::<f32>(w(0x68)))
    } else if g.read::<f32>(w(0x72)) > h {
        add(h, g.read::<f32>(w(0x7f)))
    } else {
        h
    };
    g.write::<f32>(e + h_off, h);
    (e, out.eax)
}

/// [`camera_start`] (212 bytes, 1 overlay).
pub const CAMERA_START: Sig = Sig {
    len: 212,
    hash: 0x8c0a_f2e4_add4_e5d2,
    calls: &[],
    imports: &[],
};

/// The actors' pass (0x100055e0, no arguments): for each actor `a` of the
/// 16-word table `T` (`+0x0c`, to `+0x6c`) that is set, its 50 parts `i`
/// get slot 772 `(a, i)`'s pair `v` and slot 779 `(v, a, i)` with the
/// word `*L` (`L` = `*P`, `+0x34`) at 1 during the call and put back
/// after. Returns `*P` (read again, `+0x50`) after a set last actor, else
/// 0.
pub fn actors_pass<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut eax = 0;
    let mut q = w(0x0c);
    while q < w(0x6c) {
        let a = g.read::<u32>(q);
        eax = a;
        if a != 0 {
            for i in 0..0x32 {
                let s = slot_target(g, w(0x24), 0xc10 / 4);
                let v = g.call(s, 0, &[a, i]);
                let l = g.read::<u32>(w(0x34));
                let old = g.read::<u32>(l);
                g.write::<u32>(l, 1);
                let s = slot_target(g, w(0x3d), 0xc2c / 4);
                g.call(s, 0, &[v.eax, v.edx, a, i]);
                eax = g.read::<u32>(w(0x50));
                g.write::<u32>(eax, old);
            }
        }
        q += 4;
    }
    Ret::eax(eax)
}

/// [`actors_pass`] (121 bytes, 1 overlay).
pub const ACTORS_PASS: Sig = Sig {
    len: 121,
    hash: 0x02ef_fec0_1691_f8ac,
    calls: &[],
    imports: &[],
};

/// A task step (the owner value of the start-up's task node, cdecl
/// `(node)`; 0x100053b0): the block `BLK` (the immediate `+0x10`, 0x30
/// bytes) cleared (`memset`), bit 7 of its halfword `+0x21` set and
/// `*G_SRC` (`+0x26`) copied into `+0x2e`; the script
/// ([`run`](crate::sa_script::run)) and the motion
/// ([`motion_step`](crate::s3_motion::motion_step)). Until both report
/// done, the four words `S + 0xf60..` go to `S + 0xd40..` (`S` = `*G_S`
/// `+0x07`, read first; displacements from the code) and the last is
/// returned; then `S + 0x56fc` = 1, the actors' pass ([`actors_pass`])
/// and the node given back to its pool
/// ([`give_back`](crate::sa_pool::give_back), a tail jump with `ecx` =
/// `node`), whose result is returned.
pub fn task_step_pass<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let s = g.read::<u32>(w(0x07));
    // memset(BLK, 0, 0x30)
    g.write_bytes(w(0x10), &[0; 0x30]);
    let v = g.read::<u16>(w(0x21));
    g.write::<u16>(w(0x21), v | 0x80);
    let src = g.read::<u32>(w(0x26));
    g.write::<u32>(w(0x2e), src);
    let script = crate::sa_script::run(g, img, img.rel32_target(f + 0x33));
    let motion = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x3a));
    if script == 0 || motion == 0 {
        let mut last = 0;
        for (from, to) in [(0x62, 0x68), (0x6e, 0x74), (0x7a, 0x80), (0x86, 0x8c)] {
            last = g.read::<u32>(s.wrapping_add(w(from)));
            g.write::<u32>(s.wrapping_add(w(to)), last);
        }
        return Ret::eax(last);
    }
    g.write::<u32>(s.wrapping_add(w(0x48)), 1);
    // push ebp; push esi; push edi; call
    let pass = Frame {
        at: img.rel32_target(f + 0x51),
        esp: fr.esp.wrapping_sub(0x10),
        ..*fr
    };
    actors_pass(g, img, &pass);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x5c),
        node,
    ))
}

/// [`task_step_pass`] (148 bytes, 1 overlay).
pub const TASK_STEP_PASS: Sig = Sig {
    len: 148,
    hash: 0x79f1_93f9_16d6_a019,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x33, Callee::Sig(&crate::sa_script::RUN)),
        (0x3a, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x51, Callee::Sig(&ACTORS_PASS)),
        (0x5c, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};
