//! `magic_0450.dll`'s own functions (besides the runtime and the shared
//! code). The effect block `*G_FX` (operand `+0x08` of [`orient`] and its
//! neighbours) keeps the effect's state at `+0x1c10..+0x1f00`.

use crate::ob_model::{st0_f32, wrap_once};
use crate::runtime::slot_target;
use crate::sb_sse::{add, cvt, div, mul, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// `1.0f32`'s bits.
const ONE: u32 = 0x3f80_0000;

/// Points the effect at its target (0x10005860, internal, cdecl, no
/// arguments): slot 262 `(*G_ACTOR, v, v + 4, v + 8)` fills the vector
/// `v = *G_FX + 0x1c10` (`+0xc` then 1.0); slot 333 `(*G_ACTOR)` (the
/// earlier words left on the stack) gives an angle in ST0, and
/// `C - angle` wrapped once into the circle ([`wrap_once`]) goes to
/// `+0x1ee0`. `+0x1eec` and `+0x1ee4` become -1, `+0x1efc` 0, the
/// globals at operands `+0xac` and `+0xb6` 1 and `+0xc0` -1. Returns slot
/// 333's `eax`.
pub fn orient<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x8));
    let v = fx.wrapping_add(0x1c10);
    let actor = g.read::<u32>(w(0x22));
    let s262 = slot_target(g, w(0x1b), 262);
    g.call(s262, 0, &[actor, v, v + 4, v + 8]);
    g.write::<u32>(v + 0xc, ONE);
    let s333 = slot_target(g, w(0x2f), 333);
    let actor2 = g.read::<u32>(w(0x3c));
    let o = g.call(s333, 0, &[actor2, actor, v, v + 4, v + 8]);
    let x = sub(g.read::<f32>(w(0x4f)), st0_f32(o.st0));
    let x = wrap_once(g, img, f, x, [0x5e, 0x68, 0x72, 0x7f]);
    g.write::<f32>(fx + 0x1ee0, x);
    g.write::<u32>(fx + 0x1eec, u32::MAX);
    g.write::<u32>(fx + 0x1ee4, u32::MAX);
    g.write::<u32>(fx + 0x1efc, 0);
    g.write::<u32>(w(0xac), 1);
    g.write::<u32>(w(0xb6), 1);
    g.write::<u32>(w(0xc0), u32::MAX);
    o.eax
}

/// [`orient`] (205 bytes, 1 overlay).
pub const ORIENT: Sig = Sig {
    len: 205,
    hash: 0x308e_e112_e95a_1d25,
    calls: &[],
    imports: &[],
};

/// Shows the effect's models (0x10005930, internal, cdecl, no
/// arguments): slot 628 `(0)`, slot 384 `(T[b[1]], 0)` with `b =
/// *G_B` (operand `+0x17`) and `T` the word table at `+0x22`; when the
/// effect has a second model (`*G_FX + 0x1eec` not -1), slot 384
/// `(T[b[2]])` too. Then the tail jump into
/// [`register_parts`](crate::sa_misc::register_parts) with the last
/// slot's `eax`, whose answer it returns.
pub fn show_models<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let s628 = slot_target(g, w(0x1), 628);
    let fx = g.read::<u32>(w(0xe));
    g.call(s628, 0, &[0]);
    let b = g.read::<u32>(w(0x17));
    let i = u32::from(g.read::<u8>(b + 1));
    let m = g.read::<u32>(w(0x22).wrapping_add(4 * i));
    let s384 = slot_target(g, w(0x27), 384);
    let mut eax = g.call(s384, 0, &[m, 0]).eax;
    if g.read::<u32>(fx + 0x1eec) != u32::MAX {
        let b = g.read::<u32>(w(0x41));
        let i = u32::from(g.read::<u8>(b + 2));
        let m = g.read::<u32>(w(0x4c).wrapping_add(4 * i));
        let s384 = slot_target(g, w(0x51), 384);
        eax = g.call(s384, 0, &[m]).eax;
    }
    crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0x61), eax)
}

/// [`show_models`] with the camera's step (`magic_0393` 0x100018b0, 217
/// bytes, a /GS frame; `esp` = the stack at its entry): the same models
/// (operands `+0x1e`, `+0x11`, `+0x28`, `+0x33`, `+0x38`; the second
/// `+0x51`, `+0x5c`, `+0x61`); then unless `*G_C` (`+0x72`) is set, a
/// point `V` (`ebp - 0x14`) for `n = *G_N` (`+0x81`, signed, below 0x20):
/// slot 217 `(n, V)` and slot 255 `(n, V)`; slot 211 `(*G_E, 0)` (`+0x79`)
/// and the tail into [`register_parts`](crate::sa_misc::register_parts)
/// with its `eax`, whose answer it returns.
pub fn show_models_camera<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x1e));
    g.call(slot_target(g, w(0x11), 628), 0, &[0]);
    let b = g.read::<u32>(w(0x28));
    let i = u32::from(g.read::<u8>(b + 1));
    let m = g.read::<u32>(w(0x33).wrapping_add(4 * i));
    // slot 628's word stays under it: call width 2
    g.call(slot_target(g, w(0x38), 384), 0, &[m, 0]);
    if g.read::<u32>(fx + 0x1eec) != u32::MAX {
        let b = g.read::<u32>(w(0x51));
        let i = u32::from(g.read::<u8>(b + 2));
        let m = g.read::<u32>(w(0x5c).wrapping_add(4 * i));
        g.call(slot_target(g, w(0x61), 384), 0, &[m]);
    }
    let held = g.read::<u32>(w(0x72));
    let e = g.read::<u32>(w(0x79));
    if held == 0 {
        let n = g.read::<u32>(w(0x81));
        if n.cast_signed() < 0x20 {
            let v = esp.wrapping_sub(0x18);
            g.call(slot_target(g, w(0x8f), 217), 0, &[n, v]);
            g.call(slot_target(g, w(0xa1), 255), 0, &[n, v]);
        }
    }
    let eax = g.call(slot_target(g, w(0xb2), 211), 0, &[e, 0]).eax;
    crate::sa_misc::register_parts(g, img, img.rel32_target(f + 0xc5), eax)
}

/// [`show_models_camera`] (`magic_0393` 0x100018b0; the `rel32`s: the
/// parts' registration and `__security_check_cookie`).
pub const SHOW_MODELS_CAMERA: Sig = Sig {
    len: 217,
    hash: 0x3cb3_6b15_5af7_d5cb,
    calls: &[
        (0xc5, Callee::Sig(&crate::sa_misc::REGISTER_PARTS)),
        (0xd1, Callee::Any),
    ],
    imports: &[],
};

/// [`show_models`] (101 bytes, 1 overlay).
pub const SHOW_MODELS: Sig = Sig {
    len: 101,
    hash: 0xb2a3_76de_49df_2db1,
    calls: &[(0x61, Callee::Sig(&crate::sa_misc::REGISTER_PARTS))],
    imports: &[],
};

/// The effect's frame (0x100057a0, a task callback, no arguments; a
/// cookie frame): nothing while slot 561 says nonzero or an object still
/// runs ([`run_objects`](crate::l5_objhost::run_objects) not 1) or
/// `*G_FX + 0x1efc` is not 1. Then, unless the flag at operand `+0x3c`
/// is set or the count at `+0x4c` reached 0x20, slot 217 `(n, out)` and
/// slot 255 `(n, out)` with a 4-word local `out` at `ebp - 0x14`; then
/// slot 211 `(*G_ACTOR, 0)` and slot 214 `(*G_C)` (211's words left on
/// the stack). Returns the last call's `eax` (1 or `run_objects`'
/// answer when it stops there).
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let s561 = slot_target(g, w(0x11), 561);
    let fx = g.read::<u32>(w(0x1e));
    let r = g.call(s561, 0, &[]).eax;
    if r != 0 {
        return Ret::eax(r);
    }
    // push ebp; sub esp, 0x14; push esi: the call's return address
    let run = crate::l5_objhost::run_objects(g, img, img.rel32_target(f + 0x29), ebp - 0x1c);
    if run != 1 || g.read::<u32>(fx + 0x1efc) != 1 {
        return Ret::eax(run);
    }
    let actor = g.read::<u32>(w(0x44));
    if g.read::<u32>(w(0x3c)) == 0 {
        let n = g.read::<u32>(w(0x4c));
        if n.cast_signed() < 0x20 {
            let out = ebp - 0x14;
            let s217 = slot_target(g, w(0x5a), 217);
            g.call(s217, 0, &[n, out]);
            let s255 = slot_target(g, w(0x6c), 255);
            g.call(s255, 0, &[n, out, n, out]);
        }
    }
    let s211 = slot_target(g, w(0x7d), 211);
    g.call(s211, 0, &[actor, 0]);
    let s214 = slot_target(g, w(0x8d), 214);
    let c = g.read::<u32>(w(0x93));
    Ret::eax(g.call(s214, 0, &[c, actor, 0]).eax)
}

/// [`frame`] (178 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 178,
    hash: 0x9e25_fc6e_20d0_b6f1,
    calls: &[
        (0x29, Callee::Sig(&crate::l5_objhost::RUN_OBJECTS)),
        // __security_check_cookie
        (0xaa, Callee::Any),
    ],
    imports: &[],
};

/// Loads the effect's two models (0x10005a00, an overlay table entry, no
/// arguments): when `*G_FX + 0x1eec` holds a model id `m` (not -1), slot
/// 0 `(m)` gives a handle `h`, slot 26 `(m, 1)` and slot 10 `(h, 0)`
/// (the earlier words left on the stack), `h` into the global at operand
/// `+0x3e` and 0x21 into the byte at `+0x37`, and the count at `+0x4f`
/// goes up. Then the same for the model 0x4029 (handle `k` into `+0x83`,
/// 0x20 into `+0x7c`, the count up again). With `h`, slot 632 `(k, h, 5)`
/// (4 when `+0x1ee8` is 6). Returns the last call's `eax`.
pub fn load_models<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x3));
    let m = g.read::<u32>(fx + 0x1eec);
    let mut h = 0;
    if m != u32::MAX {
        let s0 = slot_target(g, w(0x17), 0);
        h = g.call(s0, 0, &[m]).eax;
        let s26 = slot_target(g, w(0x22), 26);
        g.call(s26, 0, &[m, 1, m]);
        g.write::<u8>(w(0x37), 0x21);
        g.write::<u32>(w(0x3e), h);
        let s10 = slot_target(g, w(0x31), 10);
        g.call(s10, 0, &[h, 0, m, 1, m]);
        let n = w(0x4f);
        g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
    }
    let s0 = slot_target(g, w(0x54), 0);
    let k = g.call(s0, 0, &[0x4029]).eax;
    let s26 = slot_target(g, w(0x63), 26);
    g.call(s26, 0, &[0x4029, 1, 0x4029]);
    g.write::<u8>(w(0x7c), 0x20);
    g.write::<u32>(w(0x83), k);
    let s10 = slot_target(g, w(0x76), 10);
    let mut eax = g.call(s10, 0, &[k, 0, 0x4029, 1, 0x4029]).eax;
    let n = w(0x91);
    g.write::<u32>(n, g.read::<u32>(n).wrapping_add(1));
    if h != 0 {
        let sel = if g.read::<u32>(fx + 0x1ee8) == 6 {
            4
        } else {
            5
        };
        let s632 = slot_target(g, w(0xad), 632);
        eax = g.call(s632, 0, &[k, h, sel]).eax;
    }
    Ret::eax(eax)
}

/// [`load_models`] (194 bytes, 1 overlay).
pub const LOAD_MODELS: Sig = Sig {
    len: 194,
    hash: 0x2a45_3f14_90a8_48f6,
    calls: &[],
    imports: &[],
};

/// Places the effect's camera point (0x10005e00, an overlay table entry,
/// no arguments; a cookie frame). A height `h`: unless the flag at
/// operand `+0x12` is set or the actor index `b[0]` (`b = *G_B`) is 0x20
/// or more, slot 251 `(b[0])`; else, with `m = T[b[0]]`, slot 19 `(m)`
/// times slot 17 `(m)` (all three floats in ST0, stored as `f32`). The
/// local vector `(0, 0, h + K, 1)` at `ebp - 0x14`. Slot 262 `(T[b[0]],
/// v, v + 4, v + 8)` fills `v = *G_FX + 0x1c20` (`+0xc` 1.0); slot 235
/// makes the local matrix `ebp - 0x64` (the earlier words left on the
/// stack), slot 328 `(mat, mat, 0)` and 329 `(mat, mat, *G_FX +
/// 0x1ee0)` turn it, slot 229 `(out, mat, vec)` moves the vector into
/// `out` at `ebp - 0x24` and slot 248 `(v, v, out)` adds it to `v`; `v`
/// (`+0xc` 1.0 again) is copied to `*G_FX + 0x1ba0`. Returns `v[3]`'s
/// bits.
pub fn place_camera<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (vec, mat, out) = (ebp - 0x14, ebp - 0x64, ebp - 0x24);
    let flag = g.read::<u32>(w(0x12));
    let b = g.read::<u32>(w(0x19));
    let fx = g.read::<u32>(w(0x21));
    g.write::<u32>(vec, 0);
    let angle = g.read::<f32>(fx + 0x1ee0);
    g.write::<u32>(vec + 4, 0);
    let actor = u32::from(g.read::<u8>(b));
    let h = if flag == 0 && actor < 0x20 {
        let s251 = slot_target(g, w(0x4c), 251);
        st0_f32(g.call(s251, 0, &[actor]).st0)
    } else {
        let m = g.read::<u32>(w(0x6b).wrapping_add(4 * actor));
        let s19 = slot_target(g, w(0x70), 19);
        let a = st0_f32(g.call(s19, 0, &[m]).st0);
        let s17 = slot_target(g, w(0x7e), 17);
        let b = st0_f32(g.call(s17, 0, &[m, m]).st0);
        mul(b, a)
    };
    let h = add(h, g.read::<f32>(w(0x9c)));
    let v = fx.wrapping_add(0x1c20);
    g.write::<f32>(vec + 8, h);
    let b = g.read::<u32>(w(0xa1));
    let m = g.read::<u32>(w(0xb6).wrapping_add(4 * u32::from(g.read::<u8>(b))));
    let s262 = slot_target(g, w(0xc3), 262);
    g.call(s262, 0, &[m, v, v + 4, v + 8]);
    let s235 = slot_target(g, w(0xd6), 235);
    g.write::<u32>(v + 0xc, ONE);
    g.call(s235, 0, &[mat, m, v, v + 4, v + 8]);
    let s328 = slot_target(g, w(0xf9), 328);
    g.call(s328, 0, &[mat, mat, 0]);
    let s329 = slot_target(g, w(0x118), 329);
    g.call(s329, 0, &[mat, mat, angle.to_bits()]);
    let s229 = slot_target(g, w(0x131), 229);
    g.write::<u32>(vec + 0xc, ONE);
    g.call(s229, 0, &[out, mat, vec, mat, mat, angle.to_bits()]);
    let s248 = slot_target(g, w(0x149), 248);
    g.call(
        s248,
        0,
        &[v, v, out, out, mat, vec, mat, mat, angle.to_bits()],
    );
    g.write::<u32>(v + 0xc, ONE);
    let mut last = 0;
    for i in 0..4 {
        last = g.read::<u32>(v + 4 * i);
        g.write::<u32>(fx + 0x1ba0 + 4 * i, last);
    }
    Ret::eax(last)
}

/// [`place_camera`] (413 bytes, 1 overlay).
pub const PLACE_CAMERA: Sig = Sig {
    len: 413,
    hash: 0x9acb_0ac2_7b4c_a168,
    calls: &[
        // __security_check_cookie
        (0x195, Callee::Any),
    ],
    imports: &[],
};

/// Blends the camera point (0x100063e0, an overlay table entry, no
/// arguments): with the count `n = *G_FX + 0x1ef0`, `t = (n - 1) / D`
/// when `n > 0` (and the count goes down), else `t = K`; `+0x1ba0..`
/// becomes `a * (K - t) + b * t` per axis, `a` at `+0x1c40`, `b` at
/// `+0x1c30`, `w` 1.0. Returns the count as it ends (`n - 1`, or `n`).
pub fn blend_camera<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let fx = g.read::<u32>(w(0x2));
    let k = g.read::<f32>(w(0xa));
    let mut n = g.read::<u32>(fx + 0x1ef0);
    let t = if n.cast_signed() > 0 {
        n -= 1;
        g.write::<u32>(fx + 0x1ef0, n);
        div(cvt(n), g.read::<f32>(w(0x2a)))
    } else {
        k
    };
    let u = sub(k, t);
    let p = |g: &G, i: u32| {
        add(
            mul(g.read::<f32>(fx + 0x1c40 + 4 * i), u),
            mul(g.read::<f32>(fx + 0x1c30 + 4 * i), t),
        )
    };
    let (x, y, z) = (p(g, 0), p(g, 1), p(g, 2));
    g.write::<f32>(fx + 0x1ba0, x);
    g.write::<f32>(fx + 0x1ba4, y);
    g.write::<u32>(fx + 0x1bac, ONE);
    g.write::<f32>(fx + 0x1ba8, z);
    Ret::eax(n)
}

/// [`blend_camera`] (174 bytes, 1 overlay).
pub const BLEND_CAMERA: Sig = Sig {
    len: 174,
    hash: 0x338b_91e3_ad64_b828,
    calls: &[],
    imports: &[],
};

/// Where a copy of [`scale_turn`] reads the actor: a global (its
/// operand), or `T[b[1]]` with `b = *G_B` (the operands of `G_B`, `T`).
#[derive(Clone, Copy)]
enum Actor {
    Global(u32),
    Indexed(u32, u32),
}

impl Actor {
    fn read<G: Game>(self, g: &G, img: &Image, f: u32) -> u32 {
        match self {
            Actor::Global(op) => g.read::<u32>(img.word(f + op)),
            Actor::Indexed(gb, t) => {
                let b = g.read::<u32>(img.word(f + gb));
                let i = u32::from(g.read::<u8>(b.wrapping_add(1)));
                g.read::<u32>(img.word(f + t).wrapping_add(4 * i))
            }
        }
    }
}

/// The operand offsets of [`scale_turn`]'s code in one copy: the matrix
/// and the angles (`fx` offsets), the tables of slots 235, 277, the actor
/// for 277, the scale `K`, the actor for 15..17, the tables of slots 15,
/// 16, 17, 330, 328, 329.
struct TurnOps {
    mat: u32,
    angles: u32,
    t235: u32,
    t277: u32,
    actor277: Actor,
    k: u32,
    actor: Actor,
    t15: u32,
    t16: u32,
    t17: u32,
    t330: u32,
    t328: u32,
    t329: u32,
}

/// A model matrix `m = *G_FX + o.mat`: slot 235 `(m)`
/// (`below`: the words the caller left under it), the diagonal set to
/// slot 277 `(actor)` times `K`, then scaled per axis by slots 15, 16,
/// 17 `(actor)` (`y_first`: the second product is `s16 * m[5]`, else
/// `m[5] * s16`), and turned by slots 330, 328, 329 `(m, m, a[2] / a[0]
/// / a[1])` with the angles `a = *G_FX + o.angles`. Returns slot 329's
/// `eax`.
fn scale_turn<G: Game>(
    g: &mut G,
    img: &Image,
    f: u32,
    fx: u32,
    o: &TurnOps,
    below: &[u32],
    y_first: bool,
) -> u32 {
    let w = |off: u32| img.word(f + off);
    let m = fx.wrapping_add(o.mat);
    let mut stack = vec![m];
    stack.extend(below);
    let s235 = slot_target(g, w(o.t235), 235);
    // call width: 14
    g.call(s235, 0, &stack);
    let s277 = slot_target(g, w(o.t277), 277);
    stack.insert(0, o.actor277.read(g, img, f));
    // call width: 14
    let s = mul(st0_f32(g.call(s277, 0, &stack).st0), g.read::<f32>(w(o.k)));
    for i in [10u32, 5, 0] {
        g.write::<f32>(m + 4 * i, s);
    }
    let actor = o.actor.read(g, img, f);
    let s15 = slot_target(g, w(o.t15), 15);
    stack.insert(0, actor);
    // call width: 14
    let a = st0_f32(g.call(s15, 0, &stack).st0);
    g.write::<f32>(m, mul(g.read::<f32>(m), a));
    let s16 = slot_target(g, w(o.t16), 16);
    stack.insert(0, actor);
    // call width: 14
    let b = st0_f32(g.call(s16, 0, &stack).st0);
    let y = g.read::<f32>(m + 0x14);
    g.write::<f32>(m + 0x14, if y_first { mul(b, y) } else { mul(y, b) });
    let s17 = slot_target(g, w(o.t17), 17);
    stack.insert(0, actor);
    // call width: 14
    let c = st0_f32(g.call(s17, 0, &stack).st0);
    g.write::<f32>(m + 0x28, mul(g.read::<f32>(m + 0x28), c));
    let mut eax = 0;
    for (t, slot, at) in [(o.t330, 330, 8u32), (o.t328, 328, 0), (o.t329, 329, 4)] {
        let target = slot_target(g, w(t), slot);
        let r = g.read::<u32>(fx.wrapping_add(o.angles + at));
        eax = g.call(target, 0, &[m, m, r]).eax;
    }
    eax
}

/// Starts the camera's swing (0x10006190, an overlay table entry, no
/// arguments; a cookie frame): the point `*G_FX + 0x1ba0` copied to
/// `+0x1c30` and `+0x1c40`, the angle `+0x1b90` = -1.414; the local
/// matrix `ebp - 0x64` (slot 235) turned by slots 328 and 329 (`+0x1b90`,
/// `+0x1b94`) moves the local vector `(0, -50, 0, 1)` (`ebp - 0x14`;
/// slot 229 into `out` at `ebp - 0x24`), which slot 248 adds to
/// `+0x1c40` (`w` 1.0). Then the model matrix ([`scale_turn`]) and the
/// count `+0x1ef0` = 30. Returns slot 329's `eax`.
pub fn swing_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (vec, mat, out) = (ebp - 0x14, ebp - 0x64, ebp - 0x24);
    let fx = g.read::<u32>(w(0x13));
    let p = [0u32, 4, 8, 12].map(|o| g.read::<u32>(fx + 0x1ba0 + o));
    for (i, v) in (0u32..).zip(p) {
        g.write::<u32>(fx + 0x1c30 + 4 * i, v);
    }
    for (i, v) in (0u32..).zip(p).skip(1) {
        g.write::<u32>(fx + 0x1c40 + 4 * i, v);
    }
    let q = fx.wrapping_add(0x1c40);
    g.write::<u32>(vec, 0);
    g.write::<u32>(q, p[0]);
    g.write::<u32>(fx + 0x1b90, 0xbfb4_f4ab);
    g.write::<u32>(vec + 4, 0xc248_0000);
    g.write::<u32>(vec + 8, 0);
    g.write::<u32>(vec + 0xc, ONE);
    let s235 = slot_target(g, w(0x76), 235);
    g.call(s235, 0, &[mat]);
    let r90 = g.read::<u32>(fx + 0x1b90);
    let s328 = slot_target(g, w(0xc5), 328);
    g.call(s328, 0, &[mat, mat, r90]);
    let r94 = g.read::<u32>(fx + 0x1b94);
    let s329 = slot_target(g, w(0xe7), 329);
    g.call(s329, 0, &[mat, mat, r94]);
    let s229 = slot_target(g, w(0x100), 229);
    g.write::<u32>(vec + 0xc, ONE);
    g.call(s229, 0, &[out, mat, vec, mat, mat, r94]);
    let s248 = slot_target(g, w(0x118), 248);
    let below = [q, q, out, out, mat, vec, mat, mat, r94];
    g.call(s248, 0, &below);
    g.write::<u32>(q + 0xc, ONE);
    let ops = TurnOps {
        mat: 0x1b50,
        angles: 0x1b90,
        t235: 0x127,
        t277: 0x142,
        actor277: Actor::Global(0x148),
        k: 0x160,
        actor: Actor::Global(0x174),
        t15: 0x179,
        t16: 0x190,
        t17: 0x1ac,
        t330: 0x1c9,
        t328: 0x1f5,
        t329: 0x214,
    };
    let eax = scale_turn(g, img, f, fx, &ops, &below, false);
    g.write::<u32>(fx + 0x1ef0, 0x1e);
    Ret::eax(eax)
}

/// [`swing_start`] (584 bytes, 1 overlay).
pub const SWING_START: Sig = Sig {
    len: 584,
    hash: 0x3fc6_6035_3264_3e69,
    calls: &[
        // __security_check_cookie
        (0x240, Callee::Any),
    ],
    imports: &[],
};

/// Resets the model matrix (0x10006490, an overlay table entry, no
/// arguments): the angle `*G_FX + 0x1b90` = 0, the model matrix
/// ([`scale_turn`]), and the point `+0x1c10` copied to `+0x1ba0`.
/// Returns the copied `w`.
pub fn model_reset<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let fx = g.read::<u32>(img.word(f + 0xc));
    g.write::<u32>(fx + 0x1b90, 0);
    let ops = TurnOps {
        mat: 0x1b50,
        angles: 0x1b90,
        t235: 0x5,
        t277: 0x2c,
        actor277: Actor::Global(0x32),
        k: 0x4a,
        actor: Actor::Global(0x63),
        t15: 0x4f,
        t16: 0x7a,
        t17: 0x89,
        t330: 0xb3,
        t328: 0xdf,
        t329: 0xfe,
    };
    scale_turn(g, img, f, fx, &ops, &[], true);
    let mut last = 0;
    for i in 0..4 {
        last = g.read::<u32>(fx + 0x1c10 + 4 * i);
        g.write::<u32>(fx + 0x1ba0 + 4 * i, last);
    }
    Ret::eax(last)
}

/// [`model_reset`] (334 bytes, 1 overlay).
pub const MODEL_RESET: Sig = Sig {
    len: 334,
    hash: 0xb4b0_9aad_8bfc_b286,
    calls: &[],
    imports: &[],
};

/// Starts the effect's flight (0x100065e0, an overlay table entry, no
/// arguments; a cookie frame). The angle `a = *G_FX + 0x1ee0 + K`
/// wrapped once ([`wrap_once`]); the point `p = +0x1c10` copied to the
/// local `ebp - 0x14`. A local matrix (slot 235, slots 328 `0` and 329
/// `a`) moves the vector `(0, 20, 100, 1)` (`ebp - 0x24`; slot 229 into
/// `out` at `ebp - 0x34`) and slot 248 adds it to the local `p`, which
/// goes to `+0x1c30` (`w` 1.0); a second local matrix moves `(0, -20,
/// -10, 1)` onto `q = +0x1c40` (a copy of `+0x1c10`; `w` 1.0). The count
/// `+0x1ef0` = 45, the angles `+0x1bf0..` `(0, a, 0, 0)`, the model
/// matrix `+0x1bb0` ([`scale_turn`]), the local `p` copied to `+0x1be0`
/// and `+0x1c00`. Then for the actor `T[b[1]]`: slot 210 `(actor, m)`,
/// or (only when `m`, `*G_FX + 0x1bb0`, is 0, which a mapped block never
/// gives) unless the flag at `+0x437` is set or `b[1]` is 0x20 or more,
/// slots 217 and 255 `(b[1], out)`; last slot 211 `(actor, m)`, whose
/// `eax` it returns.
#[allow(clippy::too_many_lines)]
pub fn flight_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (p, v, out) = (ebp - 0x14, ebp - 0x24, ebp - 0x34);
    let (mat1, mat2) = (ebp - 0x74, ebp - 0xb4);
    let fx = g.read::<u32>(w(0x18));
    let x = add(g.read::<f32>(fx + 0x1ee0), g.read::<f32>(w(0x30)));
    for i in 0..4 {
        let c = g.read::<u32>(fx + 0x1c10 + 4 * i);
        g.write::<u32>(p + 4 * i, c);
    }
    let a = wrap_once(g, img, f, x, [0x51, 0x75, 0x7f, 0x8c]).to_bits();
    for (i, c) in (0u32..).zip([0u32, 0x41a0_0000, 0x42c8_0000, ONE]) {
        g.write::<u32>(v + 4 * i, c);
    }
    let s235 = slot_target(g, w(0x9d), 235);
    g.call(s235, 0, &[mat1]);
    let s328 = slot_target(g, w(0xd2), 328);
    g.call(s328, 0, &[mat1, mat1, 0]);
    let s329 = slot_target(g, w(0xf4), 329);
    g.call(s329, 0, &[mat1, mat1, a]);
    g.write::<u32>(v + 0xc, ONE);
    let s229 = slot_target(g, w(0x10d), 229);
    g.call(s229, 0, &[out, mat1, v, mat1, mat1, a]);
    let s248 = slot_target(g, w(0x12a), 248);
    g.call(s248, 0, &[p, p, out, out, mat1, v, mat1, mat1, a]);
    for i in 0..3 {
        let c = g.read::<u32>(p + 4 * i);
        g.write::<u32>(fx + 0x1c30 + 4 * i, c);
    }
    g.write::<u32>(fx + 0x1c3c, ONE);
    for i in 0..4 {
        let c = g.read::<u32>(fx + 0x1c10 + 4 * i);
        g.write::<u32>(fx + 0x1c40 + 4 * i, c);
    }
    let q = fx.wrapping_add(0x1c40);
    g.write::<u32>(p + 0xc, ONE);
    for (i, c) in (0u32..).zip([0u32, 0xc1a0_0000, 0xc120_0000, ONE]) {
        g.write::<u32>(v + 4 * i, c);
    }
    let s235 = slot_target(g, w(0x1a5), 235);
    g.call(s235, 0, &[mat2, p, p, out, out, mat1, v, mat1, mat1, a]);
    let s328 = slot_target(g, w(0x1e7), 328);
    g.call(s328, 0, &[mat2, mat2, 0]);
    let s329 = slot_target(g, w(0x20c), 329);
    g.call(s329, 0, &[mat2, mat2, a]);
    g.write::<u32>(v + 0xc, ONE);
    let s229 = slot_target(g, w(0x228), 229);
    g.call(s229, 0, &[out, mat2, v, mat2, mat2, a]);
    let s248 = slot_target(g, w(0x240), 248);
    let below = [q, q, out, out, mat2, v, mat2, mat2, a];
    g.call(s248, 0, &below);
    g.write::<u32>(q + 0xc, ONE);
    g.write::<u32>(fx + 0x1ef0, 0x2d);
    for (o, c) in [(0x1bf0u32, 0u32), (0x1bf4, a), (0x1bf8, 0), (0x1bfc, 0)] {
        g.write::<u32>(fx + o, c);
    }
    let ops = TurnOps {
        mat: 0x1bb0,
        angles: 0x1bf0,
        t235: 0x257,
        t277: 0x2b2,
        actor277: Actor::Indexed(0x2a2, 0x2ad),
        k: 0x2d0,
        actor: Actor::Indexed(0x2e3, 0x2ee),
        t15: 0x2f3,
        t16: 0x310,
        t17: 0x332,
        t330: 0x355,
        t328: 0x381,
        t329: 0x3a0,
    };
    scale_turn(g, img, f, fx, &ops, &below, false);
    for i in 0..4 {
        let c = g.read::<u32>(p + 4 * i);
        g.write::<u32>(fx + 0x1be0 + 4 * i, c);
        g.write::<u32>(fx + 0x1c00 + 4 * i, c);
    }
    let m = fx.wrapping_add(0x1bb0);
    let b = g.read::<u32>(w(0x40b));
    let i = u32::from(g.read::<u8>(b.wrapping_add(1)));
    let actor = g.read::<u32>(w(0x419).wrapping_add(4 * i));
    if m != 0 {
        let s210 = slot_target(g, w(0x422), 210);
        g.call(s210, 0, &[actor, m]);
    } else if g.read::<u32>(w(0x437)) == 0 && i < 0x20 {
        let s217 = slot_target(g, w(0x448), 217);
        g.call(s217, 0, &[i, out]);
        let s255 = slot_target(g, w(0x45a), 255);
        g.call(s255, 0, &[i, out, i, out]);
    }
    let s211 = slot_target(g, w(0x46b), 211);
    Ret::eax(g.call(s211, 0, &[actor, m]).eax)
}

/// [`flight_start`] (1165 bytes, 1 overlay).
pub const FLIGHT_START: Sig = Sig {
    len: 1165,
    hash: 0x3145_10a5_b51d_f2f7,
    calls: &[
        // __security_check_cookie
        (0x485, Callee::Any),
    ],
    imports: &[],
};

/// The flight's frame (0x10006a70, an overlay table entry, no
/// arguments): with the count `n = *G_FX + 0x1ef0`, `t = (n - 1) / D`
/// when `n > 0` (and the count goes down), else `t = K`; `+0x1c00..`
/// becomes `b * t + a * (K - t)` per axis (`b` at `+0x1c30`, `a` at
/// `+0x1c40`, `w` 1.0); the angle `+0x1bf0` goes up by a step, wrapped
/// once ([`wrap_once`]); then the model matrix `+0x1bb0`
/// ([`scale_turn`]). Returns slot 329's `eax`.
pub fn flight_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let k = g.read::<f32>(w(0x8));
    let fx = g.read::<u32>(w(0xf));
    let n = g.read::<u32>(fx + 0x1ef0);
    let t = if n.cast_signed() > 0 {
        g.write::<u32>(fx + 0x1ef0, n - 1);
        div(cvt(n - 1), g.read::<f32>(w(0x31)))
    } else {
        k
    };
    let u = sub(k, t);
    let p = |g: &G, i: u32| {
        add(
            mul(g.read::<f32>(fx + 0x1c30 + 4 * i), t),
            mul(g.read::<f32>(fx + 0x1c40 + 4 * i), u),
        )
    };
    let (x, y, z) = (p(g, 0), p(g, 1), p(g, 2));
    g.write::<f32>(fx + 0x1c00, x);
    g.write::<f32>(fx + 0x1c04, y);
    g.write::<u32>(fx + 0x1c0c, ONE);
    g.write::<f32>(fx + 0x1c08, z);
    let a = add(g.read::<f32>(fx + 0x1bf0), g.read::<f32>(w(0xc0)));
    let a = wrap_once(g, img, f, a, [0xc7, 0xd1, 0xdb, 0xe8]);
    g.write::<f32>(fx + 0x1bf0, a);
    let ops = TurnOps {
        mat: 0x1bb0,
        angles: 0x1bf0,
        t235: 0xed,
        t277: 0x119,
        actor277: Actor::Indexed(0x109, 0x114),
        k: 0x131,
        actor: Actor::Indexed(0x144, 0x14f),
        t15: 0x154,
        t16: 0x16b,
        t17: 0x187,
        t330: 0x1a4,
        t328: 0x1d0,
        t329: 0x1ef,
    };
    Ret::eax(scale_turn(g, img, f, fx, &ops, &[], false))
}

/// [`flight_frame`] (527 bytes, 1 overlay).
pub const FLIGHT_FRAME: Sig = Sig {
    len: 527,
    hash: 0x3063_bd3b_af3b_4282,
    calls: &[],
    imports: &[],
};

/// An object's callback (0x10006da0, `(obj)`, pushed by 0x10006f72; a
/// cookie frame): the camera (slot 723 into `ebp - 0x94`) and slot 724's
/// matrix (`ebp - 0x54`), slot 327 `(obj + 0x80, mat)` copies it into
/// the object, slot 229 `(obj + 0xd0, obj + 0x80, v)` moves the local
/// vector `(0, 0, 5, 1)` (`ebp - 0x14`) into the object's position
/// (`+0xdc` 1.0). The four scales `+0x60..` become `C / s * K` with `s`
/// slot 282's float. After frame 0x1b (`+0x118`), the count `+0x120`
/// goes down to 0 and the four halfwords `+0x70..` get `(count << 12) /
/// 6`, which it returns; before, slot 282's `eax`.
pub fn shrink<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (cam, mat, v) = (ebp - 0x94, ebp - 0x54, ebp - 0x14);
    let obj = fr.arg(g, 0);
    let s723 = slot_target(g, w(0x20), 723);
    g.call(s723, 0, &[cam]);
    let s724 = slot_target(g, w(0x31), 724);
    g.call(s724, 0, &[mat, cam]);
    let rows = obj.wrapping_add(0x80);
    let s327 = slot_target(g, w(0x42), 327);
    g.call(s327, 0, &[rows, mat, mat, cam]);
    for (i, c) in (0u32..).zip([0u32, 0, 0x40a0_0000, ONE]) {
        g.write::<u32>(v + 4 * i, c);
    }
    let s229 = slot_target(g, w(0x62), 229);
    g.call(
        s229,
        0,
        &[obj.wrapping_add(0xd0), rows, v, rows, mat, mat, cam],
    );
    g.write::<u32>(obj + 0xdc, ONE);
    let s282 = slot_target(g, w(0x8b), 282);
    let o = g.call(s282, 0, &[]);
    let s = st0_f32(o.st0);
    let scale = mul(div(g.read::<f32>(w(0xb5)), s), g.read::<f32>(w(0xc5)));
    for off in [0x6c, 0x68, 0x64, 0x60] {
        g.write::<f32>(obj + off, scale);
    }
    if g.read::<i32>(obj + 0x118) <= 0x1b {
        return Ret::eax(o.eax);
    }
    let n = g.read::<i32>(obj + 0x120);
    if n > 0 {
        g.write::<i32>(obj + 0x120, n - 1);
    }
    let n = g.read::<i32>(obj + 0x120);
    // imul 0x2aaaaaab: the signed division by 6
    let q = n.wrapping_shl(12) / 6;
    for off in [0x76, 0x74, 0x72, 0x70] {
        // mov word: the low half
        g.write::<u16>(obj + off, q.cast_unsigned() as u16);
    }
    Ret::eax(q.cast_unsigned())
}

/// [`shrink`] (295 bytes, 1 overlay).
pub const SHRINK: Sig = Sig {
    len: 295,
    hash: 0xa21f_3231_7d33_3984,
    calls: &[
        // __security_check_cookie
        (0x11f, Callee::Any),
    ],
    imports: &[],
};

/// Turns the camera a step (0x10007ca0, an overlay table entry, no
/// arguments): the point `*G_FX + 0x1ba0` copied to `+0x1c30`, its `y`
/// then `+0x1c14 - K`; the angle `+0x1b90` goes up by a step, wrapped
/// once ([`wrap_once`]); the count `+0x1ef8` = 60. Returns the copied
/// `w`.
pub fn turn_step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x2));
    let mut last = 0;
    for i in 0..4 {
        last = g.read::<u32>(fx + 0x1ba0 + 4 * i);
        g.write::<u32>(fx + 0x1c30 + 4 * i, last);
    }
    let y = sub(g.read::<f32>(fx + 0x1c14), g.read::<f32>(w(0x42)));
    g.write::<f32>(fx + 0x1c34, y);
    let a = add(g.read::<f32>(fx + 0x1b90), g.read::<f32>(w(0x5a)));
    let a = wrap_once(g, img, f, a, [0x61, 0x6b, 0x86, 0x93]);
    g.write::<f32>(fx + 0x1b90, a);
    g.write::<u32>(fx + 0x1ef8, 0x3c);
    Ret::eax(last)
}

/// [`turn_step`] (170 bytes, 1 overlay).
pub const TURN_STEP: Sig = Sig {
    len: 170,
    hash: 0x8c85_f8f4_4476_19c6,
    calls: &[],
    imports: &[],
};

/// The effect's per-frame task callback (0x100056b0, cdecl `(node)`):
/// the 0x30-byte block `B` (operand `+0x10`) zeroed (`memset`), the
/// half-word `B + 2` (`+0x21`) gets bit 0x80, `+0x2e` = `*G` (`+0x26`);
/// the script runs ([`run`](crate::sa_script::run)) and slot 561 `()`
/// (`ecx` the import table) answers: non-zero is returned as it is.
/// Else the two motion scripts (`+0x51`, `+0x5b`;
/// [`motion_script`](crate::s4_misc::motion_script)) step; when the
/// script and the first motion script are both done, `*G_FX + D` (`+0x70`) =
/// 1, the models are shown ([`show_models`]) and the node is given back
/// ([`give_back`](crate::sa_pool::give_back), a tail jump with `ecx` =
/// the node; its `eax` returned). Otherwise the points `+0x1ba0` and
/// `+0x1c00` are copied to `+0x1b80` and `+0x1be0`, returning the last
/// word copied.
pub fn task<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x7));
    g.write_bytes(w(0x10), &[0u8; 0x30]);
    let h = g.read::<u16>(w(0x21));
    g.write::<u16>(w(0x21), h | 0x80);
    let v = g.read::<u32>(w(0x26));
    g.write::<u32>(w(0x2e), v);
    let script = crate::sa_script::run(g, img, img.rel32_target(f + 0x33));
    let table = g.read::<u32>(w(0x39));
    let t = slot_target(g, w(0x39), 561);
    let busy = g.call(t, table, &[]).eax;
    if busy != 0 {
        return Ret::eax(busy);
    }
    let m1 = crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x56), w(0x51));
    crate::s4_misc::motion_script(g, img, img.rel32_target(f + 0x62), w(0x5b));
    if script != 0 && m1 != 0 {
        // the flag's displacement: 0x1efc here, 0x1f00 in `magic_0393`
        g.write::<u32>(fx + img.word(f + 0x70), 1);
        let shown = img.rel32_target(f + 0x79);
        // `magic_0393`'s copy has a frame (`push ebp`); four pushes and the call
        if img.word(shown) & 0xff == 0x55 {
            show_models_camera(g, img, shown, fr.esp.wrapping_sub(0x14));
        } else {
            show_models(g, img, shown);
        }
        let node = fr.arg(g, 0);
        return Ret::eax(crate::sa_pool::give_back(
            g,
            img,
            img.rel32_target(f + 0x85),
            node,
        ));
    }
    let mut last = 0;
    for (from, to) in [(0x1ba0, 0x1b80), (0x1c00, 0x1be0)] {
        for i in 0..4 {
            last = g.read::<u32>(fx + from + 4 * i);
            g.write::<u32>(fx + to + 4 * i, last);
        }
    }
    Ret::eax(last)
}

/// [`task`] (238 bytes, 1 overlay).
pub const TASK: Sig = Sig {
    len: 238,
    hash: 0x8e91_d266_1956_8854,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x33, Callee::Sig(&crate::sa_script::RUN)),
        (0x56, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x62, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x79, Callee::Sig(&SHOW_MODELS)),
        (0x85, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// [`task`] in `magic_0393` (0x10001600; its models shown with the
/// camera's step, [`show_models_camera`]).
pub const TASK_0393: Sig = Sig {
    len: 238,
    hash: 0xf182_53b3_b2eb_ccdf,
    calls: &[
        (0x15, Callee::Import("memset")),
        (0x33, Callee::Sig(&crate::sa_script::RUN)),
        (0x56, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x62, Callee::Sig(&crate::s4_misc::MOTION_SCRIPT)),
        (0x79, Callee::Sig(&SHOW_MODELS_CAMERA)),
        (0x85, Callee::Sig(&crate::sa_pool::GIVE_BACK)),
    ],
    imports: &[],
};

/// Starts the second object (0x10006ed0, an overlay table entry, no
/// arguments, `ecx` passed on): the point `*G_FX + 0x1c10` copied to
/// `+0x1ba0`; the actor `a = T[b[1]]` (`b = *G_B`, operand `+0x40`, `T`
/// the table at `+0x4b`): when the effect has a second model (`+0x1eec`
/// not -1) slot 632 `(a, 0, 0)`, then slot 632 `(a, *G_M, 0x12)`
/// (`+0x6f`). The object `+0x850` starts
/// ([`start_object_at`](crate::s4_misc::start_object_at): scale `*K`
/// `+0x80`, index 7, owners 0), its callback `+0x954` the immediate
/// `+0xa2`, byte `+0x976` 2, `+0x970` 6. Returns the object.
pub fn second_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x3));
    for i in 0..4 {
        let v = g.read::<u32>(fx + 0x1c10 + 4 * i);
        g.write::<u32>(fx + 0x1ba0 + 4 * i, v);
    }
    let b = g.read::<u32>(w(0x40));
    let i = u32::from(g.read::<u8>(b + 1));
    let a = g.read::<u32>(w(0x4b).wrapping_add(4 * i));
    if g.read::<u32>(fx + 0x1eec) != u32::MAX {
        let t = slot_target(g, w(0x52), 632);
        g.call(t, 0, &[a, 0, 0]);
    }
    let t = slot_target(g, w(0x67), 632);
    let m = g.read::<u32>(w(0x6f));
    g.call(t, 0, &[a, m, 0x12]);
    let scale = g.read::<f32>(w(0x80));
    let obj = fx.wrapping_add(0x850);
    let o =
        crate::s4_misc::start_object_at(g, img, img.rel32_target(f + 0x95), obj, scale, 7, 0, 0);
    g.write::<u32>(fx + 0x954, w(0xa2));
    g.write::<u8>(fx + 0x976, 2);
    g.write::<u32>(fx + 0x970, 6);
    Ret::eax(o)
}

/// [`second_start`] (186 bytes, 1 overlay).
pub const SECOND_START: Sig = Sig {
    len: 186,
    hash: 0x70da_7d1a_9031_3f46,
    calls: &[(0x95, Callee::Sig(&crate::s4_misc::START_OBJECT_AT))],
    imports: &[],
};

/// Starts the first object (0x10006ff0, an overlay table entry, no
/// arguments, `ecx` passed on): the object `*G_FX + 0x5f0` starts
/// ([`start_object_at`](crate::s4_misc::start_object_at): scale `*K`
/// `+0x4`, index 5, owner 0xc, second owner 0); `+0x6b0` = pi/4,
/// `+0x6b4` = the angle `+0x1ee0`, the callback `+0x6f4` the immediate
/// `+0x42`. Returns the angle's bits.
pub fn first_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let scale = g.read::<f32>(w(0x4));
    let fx = g.read::<u32>(w(0xb));
    crate::s4_misc::start_object_at(
        g,
        img,
        img.rel32_target(f + 0x1e),
        fx.wrapping_add(0x5f0),
        scale,
        5,
        0xc,
        0,
    );
    g.write::<u32>(fx + 0x6b0, 0x3f49_0fdb);
    let a = g.read::<u32>(fx + 0x1ee0);
    g.write::<u32>(fx + 0x6b4, a);
    g.write::<u32>(fx + 0x6f4, w(0x42));
    Ret::eax(a)
}

/// [`first_start`] (72 bytes, 1 overlay).
pub const FIRST_START: Sig = Sig {
    len: 72,
    hash: 0x1a41_1254_dcc5_ca94,
    calls: &[(0x1e, Callee::Sig(&crate::s4_misc::START_OBJECT_AT))],
    imports: &[],
};

/// Starts the colour scale (0x10007c40, an overlay table entry, no
/// arguments): the point `*G_FX + 0x1c10` copied to `+0x1ba0`, then the
/// scale task of `+0x1e08`
/// ([`scale_task`](crate::s4_misc::scale_task): `xmm1`/`xmm2` the
/// constants at `+0x18`/`+0xa`, the count 60), whose node it returns.
pub fn scale_start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let fx = g.read::<u32>(w(0x2));
    for i in 0..4 {
        let v = g.read::<u32>(fx + 0x1c10 + 4 * i);
        g.write::<u32>(fx + 0x1ba0 + 4 * i, v);
    }
    // push 0x3c: the task's frame (its return address below the count)
    let esp = fr.esp.wrapping_sub(8);
    g.write::<u32>(esp + 4, 0x3c);
    let mut xmm = fr.xmm;
    xmm[1] = u128::from(g.read::<u32>(w(0x18)));
    xmm[2] = u128::from(g.read::<u32>(w(0xa)));
    let callee = Frame {
        at: img.rel32_target(f + 0x4f),
        esp,
        ecx: fx.wrapping_add(0x1e08),
        xmm,
        ..*fr
    };
    crate::s4_misc::scale_task(g, img, &callee)
}

/// [`scale_start`] (85 bytes, 1 overlay).
pub const SCALE_START: Sig = Sig {
    len: 85,
    hash: 0xc4c6_7b2e_c5dc_3ac9,
    calls: &[(0x4f, Callee::Sig(&crate::s4_misc::SCALE_TASK))],
    imports: &[],
};
