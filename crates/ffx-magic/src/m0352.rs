//! `magic_0352.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The frame callback (0x10004c10, cdecl `(node)`, an aligned frame):
/// the script ([`run`](crate::sa_script::run)) and the motion step
/// ([`motion_step`](crate::s3_motion::motion_step)); when both answer 1
/// the effect's (`*E`, `+0x0a`) `+0xbd8` becomes 1, the handle of the part
/// 0x160 is registered (slot 772 `(0x160, 0)`'s 64-bit result to slot 779
/// with the game flag `*L` 1 around it, `L` `+0x3f`/`+0x5b`) and the node
/// goes back to its pool ([`give_back`](crate::sa_pool::give_back)).
/// Returns that, else the motion step's answer.
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x0a));
    let script = crate::sa_script::run(g, img, img.rel32_target(f + 0x0f));
    let step = crate::s3_motion::motion_step(g, img, img.rel32_target(f + 0x16));
    if script != 1 || step != 1 {
        return Ret::eax(step);
    }
    g.write::<u32>(fx + 0xbd8, 1);
    let s = slot_target(g, w(0x24), 0xc10 / 4);
    let h = g.call(s, 0, &[0x160, 0]);
    let lock = g.read::<u32>(w(0x3f));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s = slot_target(g, w(0x48), 0xc2c / 4);
    g.call(s, 0, &[h.eax, h.edx, 0x160, 0]);
    let lock = g.read::<u32>(w(0x5b));
    g.write::<u32>(lock, saved);
    let node = fr.arg(g, 0);
    Ret::eax(crate::sa_pool::give_back(
        g,
        img,
        img.rel32_target(f + 0x68),
        node,
    ))
}

/// [`frame`] (114 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 114,
    hash: 0xabe1_3f75_5211_f3de,
    calls: &[
        (0x0f, Callee::Sig(&crate::sa_script::RUN)),
        (0x16, Callee::Sig(&crate::s3_motion::MOTION_STEP)),
        (0x68, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// An object's camera point (0x10004e60, cdecl `(obj)`, a /GS frame of
/// 0x84 bytes): slot 723 `(&A)` and slot 724 `(&B)` (`A` `ebp - 0x84`,
/// `B` `ebp - 0x44` on the frame), slot 327 `(obj + 0x80, &B)`, then the
/// point `p` = `obj + 0xd0` set to `(0, 0, 5.0, 1.0)` goes through slot
/// 229 `(p, obj + 0x80, p)` and `p + 0xc` = 1.0. Returns slot 229's `eax`.
/// The slot calls pass the words the original leaves on the stack.
pub fn camera_point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    Ret::eax(point(g, img, fr).1)
}

/// [`camera_point`]'s body (also [`camera_point_show`]'s, the same
/// operands): returns the words left on the stack and slot 229's `eax`.
fn point<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> (crate::pushes::Pushes, u32) {
    let w = |off: u32| img.word(fr.at + off);
    let ebp = fr.esp.wrapping_sub(4);
    let obj = fr.arg(g, 0);
    let (m, p) = (obj + 0x80, obj + 0xd0);
    let (a, b) = (ebp.wrapping_sub(0x84), ebp.wrapping_sub(0x44));
    let mut st = crate::pushes::Pushes::default();
    st.push(a);
    let s = slot_target(g, w(0x2c), 0xb4c / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    st.push(b);
    let s = slot_target(g, w(0x3d), 0xb50 / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    st.push_all(&[b, m]);
    let s = slot_target(g, w(0x4e), 0x51c / 4);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    let s = slot_target(g, w(0x5c), 0x394 / 4);
    st.push_all(&[p, m, p]);
    for (k, v) in (0u32..).zip([0, 0, 0x40a0_0000, 0x3f80_0000]) {
        g.write::<u32>(p + 4 * k, v);
    }
    // call width: 12 (Pushes caps the words at 12)
    let eax = st.call(g, s, 0).eax;
    g.write::<u32>(p + 0xc, 0x3f80_0000);
    (st, eax)
}

/// [`camera_point`] (160 bytes, 1 overlay).
pub const CAMERA_POINT: Sig = Sig {
    len: 160,
    hash: 0x017f_21c6_c0c1_693d,
    calls: &[
        // __security_check_cookie
        (0x98, Callee::Any),
    ],
    imports: &[],
};

/// The camera point and its show (0x10004f60, cdecl `(obj)`, a /GS frame
/// of 0x84 bytes): [`camera_point`]'s steps, then slot 904 `(2, 1)`.
/// Returns slot 904's `eax`. The slot calls pass the words the original
/// leaves on the stack.
pub fn camera_point_show<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let (mut st, _) = point(g, img, fr);
    st.push_all(&[1, 2]);
    let s = slot_target(g, img.word(fr.at + 0x87), 0xe20 / 4);
    // call width: 12 (Pushes caps the words at 12)
    Ret::eax(st.call(g, s, 0).eax)
}

/// [`camera_point_show`] (177 bytes, 1 overlay).
pub const CAMERA_POINT_SHOW: Sig = Sig {
    len: 177,
    hash: 0xe4e7_52fc_536b_3325,
    calls: &[
        // __security_check_cookie
        (0xa9, Callee::Any),
    ],
    imports: &[],
};

/// The camera's start (0x10004cd0, internal, no arguments): with `E` = `*G`
/// (`+0x0e`): `E + 0x980` = 0; slot 333 `(*A)`'s angle `a` (`A` `+0x26`)
/// turned into `h = K - a` (`K` `+0x36`) wrapped once by `T`
/// (`+0x4f`/`+0x66`) above `HI` (`+0x45`) or below `LO` (`+0x59`) into
/// `E + 0x984`, `E + 0x988` and `E + 0x98c` 0; with `C` = `E + 0x990` slot
/// 262 `(*A, C, C + 4, C + 8)` (`A` again `+0x88`) and slot 12 `(*A)`'s
/// float into `C + 4`, `C + 0xc` = 1.0, `E + 0xbd8` = 0 and `*D` (`+0xd1`)
/// = 1. Returns `E`. The slot calls pass the words the original leaves on
/// the stack.
pub fn camera_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, sub};
    let w = |off: u32| img.word(fr.at + off);
    let e = g.read::<u32>(w(0x0e));
    g.write::<u32>(e + 0x980, 0);
    let s = slot_target(g, w(0x07), 0x534 / 4);
    let x = g.read::<u32>(w(0x26));
    let a = crate::m0018::st0_f32(g.call(s, 0, &[x]).st0);
    let h = sub(g.read::<f32>(w(0x36)), a);
    let h = if h > g.read::<f32>(w(0x45)) {
        sub(h, g.read::<f32>(w(0x4f)))
    } else if g.read::<f32>(w(0x59)) > h {
        add(h, g.read::<f32>(w(0x66)))
    } else {
        h
    };
    g.write::<f32>(e + 0x984, h);
    g.write::<u32>(e + 0x988, 0);
    g.write::<u32>(e + 0x98c, 0);
    let x = g.read::<u32>(w(0x88));
    let c = e + 0x990;
    let mut st = crate::pushes::Pushes::default();
    let s = slot_target(g, w(0x97), 0x418 / 4);
    st.push_all(&[c + 8, c + 4, c, x]);
    // call width: 12 (Pushes caps the words at 12)
    st.call(g, s, 0);
    st.push(x);
    let s = slot_target(g, w(0xaa), 0x30 / 4);
    // call width: 12 (Pushes caps the words at 12)
    let v = crate::m0018::st0_f32(st.call(g, s, 0).st0);
    g.write::<f32>(c + 4, v);
    g.write::<u32>(c + 0xc, 0x3f80_0000);
    g.write::<u32>(e + 0xbd8, 0);
    g.write::<u32>(w(0xd1), 1);
    Ret::eax(e)
}

/// [`camera_start`] (222 bytes, 1 overlay).
pub const CAMERA_START: Sig = Sig {
    len: 222,
    hash: 0xbaf9_8392_9182_d72c,
    calls: &[],
    imports: &[],
};
