//! `magic_0393.dll`'s own functions (besides the runtime and the object
//! host): the summon's state block `*G_STATE` (its position `+0x1c10`,
//! the model frame `+0x1bb0`, the target `+0x1ee8`..), the start of the
//! summon's facing, its camera and model, the per-frame steps and the
//! callbacks the effect stores.

use crate::l5_objhost::{self as host, ONE};
use crate::ob_model::{Angle, ModelOps, Who, scale_model, st0_f32, wrap_once};
use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The caster's selection byte of the overlays' character table
/// (`G_CHRS[(*G_SEL)[1]]`) at these operand offsets.
const fn caster(sel: u32, table: u32) -> Who {
    Who::Sel { sel, at: 1, table }
}

/// The summon's facing and target (0x10001770, no arguments): the
/// summon's position (slot 262 `(*G_SUMMON, &+0x1c10..)`, `w` = 1), its
/// facing `A - Ch_GetRot` (slot 333) wrapped once into `+0x1ee0`; slot
/// 627's target (negative: none, `+0x1eec` = -1), else its index in
/// `+0x1ee8`, its model (slot 390) in `+0x1eec`, its table word in
/// `+0x1ef0` and that entry's stream cued (slot 551 `(slot 719 (E, 6,
/// word), 0, 1)`); `+0x1ee4` = -1, `+0x1f00` = 0 and three flags.
/// Returns slot 551's `eax` (the target's index without one).
pub fn face<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let state = g.read::<u32>(w(0x08));
    let pos = state + 0x1c10;
    let chr = g.read::<u32>(w(0x22));
    let s262 = slot_target(g, w(0x1b), 262);
    let mut stack = vec![chr, pos, pos + 4, pos + 8];
    g.call(s262, 0, &stack);
    g.write::<u32>(pos + 0xc, ONE);
    let s333 = slot_target(g, w(0x2f), 333);
    stack.insert(0, g.read::<u32>(w(0x3c)));
    // call width: 7 at most (the earlier calls' words stay on the stack)
    let rot = st0_f32(g.call(s333, 0, &stack).st0);
    let x = g.read::<f32>(w(0x4f)) - rot;
    let x = wrap_once(g, img, f, x, [0x5e, 0x68, 0x72, 0x7f]);
    g.write::<f32>(state + 0x1ee0, x);
    let s627 = slot_target(g, w(0x84), 627);
    let r = g.call(s627, 0, &[]).eax;
    let eax = if r.cast_signed() < 0 {
        g.write::<u32>(state + 0x1eec, 0xffff_ffff);
        r
    } else {
        g.write::<u32>(state + 0x1ee8, r);
        let s390 = slot_target(g, w(0xa0), 390);
        let model = g.call(s390, 0, &[r]).eax;
        g.write::<u32>(state + 0x1eec, model);
        let word = g.read::<u32>(w(0xbc).wrapping_add(4 * r));
        g.write::<u32>(state + 0x1ef0, word);
        let s719 = slot_target(g, w(0xc8), 719);
        let mut stack = vec![w(0xd5), 6, word, r];
        let e = g.call(s719, 0, &stack).eax;
        let s551 = slot_target(g, w(0xe1), 551);
        stack.splice(0..0, [e, 0, 1]);
        g.call(s551, 0, &stack).eax
    };
    g.write::<u32>(state + 0x1ee4, 0xffff_ffff);
    g.write::<u32>(state + 0x1f00, 0);
    g.write::<u32>(w(0x113), 1);
    g.write::<u32>(w(0x11d), 1);
    g.write::<u32>(w(0x127), 0xffff_ffff);
    eax
}

/// [`face`] (308 bytes, 1 overlay).
pub const FACE: Sig = Sig {
    len: 308,
    hash: 0x83f7_65d6_f2bc_b756,
    calls: &[],
    imports: &[],
};

/// A task step (0x100016f0, no arguments): nothing while the game is
/// paused (slot 561's result returned); runs the objects
/// ([`host::run_objects`]) and, once none ran and the state's `+0x1f00`
/// is 1, shows the weapon again (slot 257 `(slot 219 (*G_CHR, 0), 0)`),
/// slot 631, and ends the effect (slot 214 `(G_HANDLE)`, its `eax`
/// returned).
pub fn step<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s561 = slot_target(g, w(0x01), 561);
    let state = g.read::<u32>(w(0x0e));
    let p = g.call(s561, 0, &[]).eax;
    if p != 0 {
        return Ret::eax(p);
    }
    // push esi; call
    let r = host::run_objects(g, img, img.rel32_target(f + 0x19), fr.esp - 8);
    if r != 1 || g.read::<u32>(state + 0x1f00) != 1 {
        return Ret::eax(r);
    }
    let s219 = slot_target(g, w(0x2b), 219);
    let chr = g.read::<u32>(w(0x33));
    let mut stack = vec![chr, 0];
    let h = g.call(s219, 0, &stack).eax;
    let s257 = slot_target(g, w(0x43), 257);
    stack.splice(0..0, [h, 0]);
    g.call(s257, 0, &stack);
    let s631 = slot_target(g, w(0x50), 631);
    g.call(s631, 0, &stack);
    let s214 = slot_target(g, w(0x5d), 214);
    stack.insert(0, g.read::<u32>(w(0x63)));
    // call width: 5 at most (the earlier calls' words stay on the stack)
    Ret::eax(g.call(s214, 0, &stack).eax)
}

/// [`step`] (116 bytes, 1 overlay).
pub const STEP: Sig = Sig {
    len: 116,
    hash: 0x2cff_9057_0d0a_1517,
    calls: &[(0x19, Callee::Sig(&host::RUN_OBJECTS))],
    imports: &[],
};

/// Loads the two models (0x100019f0, no arguments): the target's model
/// `+0x1eec` (unless -1) and model 0x4029, each allocated (slot 0), its
/// motion group read (slot 26 `(model, 1)`), kept with its selection
/// byte (0x21, 0x20) and clipped (slot 10 `(chr, 0)`), the model count
/// `G_N` counted up; then with a target the effect model is bound to it
/// (slot 632 `(effect, target, elem)`, elem 4 after hiding battle
/// character 6 (slot 278 `(slot 215 (6), 1, 0)`) when the target is 6,
/// else 14). Returns slot 632's `eax` (slot 10's without a target).
pub fn models<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let state = g.read::<u32>(w(0x05));
    let model = g.read::<u32>(state + 0x1eec);
    let mut target = 0;
    if model != 0xffff_ffff {
        let s0 = slot_target(g, w(0x17), 0);
        target = g.call(s0, 0, &[model]).eax;
        let s26 = slot_target(g, w(0x22), 26);
        g.call(s26, 0, &[model, 1, model]);
        g.write::<u8>(w(0x37), 0x21);
        g.write::<u32>(w(0x3e), target);
        let s10 = slot_target(g, w(0x31), 10);
        g.call(s10, 0, &[target, 0, model, 1, model]);
        let n = g.read::<u32>(w(0x4f));
        g.write::<u32>(w(0x4f), n.wrapping_add(1));
    }
    let s0 = slot_target(g, w(0x54), 0);
    let effect = g.call(s0, 0, &[0x4029]).eax;
    let s26 = slot_target(g, w(0x63), 26);
    g.call(s26, 0, &[0x4029, 1, 0x4029]);
    g.write::<u8>(w(0x7c), 0x20);
    g.write::<u32>(w(0x83), effect);
    let s10 = slot_target(g, w(0x76), 10);
    let r = g.call(s10, 0, &[effect, 0, 0x4029, 1, 0x4029]).eax;
    let n = g.read::<u32>(w(0x91));
    g.write::<u32>(w(0x91), n.wrapping_add(1));
    if target == 0 {
        return Ret::eax(r);
    }
    let elem = if g.read::<u32>(state + 0x1ee8) == 6 {
        let s215 = slot_target(g, w(0xa6), 215);
        let h = g.call(s215, 0, &[6]).eax;
        let s278 = slot_target(g, w(0xbf), 278);
        g.call(s278, 0, &[h, 1, 0, 6]);
        4
    } else {
        0xe
    };
    let s632 = slot_target(g, w(0xd6), 632);
    Ret::eax(g.call(s632, 0, &[effect, target, elem]).eax)
}

/// [`models`] (236 bytes, 1 overlay).
pub const MODELS: Sig = Sig {
    len: 236,
    hash: 0xbf1d_c252_4541_7d7e,
    calls: &[],
    imports: &[],
};

/// An object's attachment callback (0x10002370, cdecl `(obj)`): the
/// summon's option position 8 into `obj + 0xd0` (slot 268 `(*G_SUMMON,
/// 8, obj + 0xd0)`), `w` = 1, `obj + 0x104` = 0. Returns slot 268's
/// `eax`.
pub fn attach<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let s268 = slot_target(g, img.word(f + 0x04), 268);
    let chr = g.read::<u32>(img.word(f + 0x1e));
    let r = g.call(s268, 0, &[chr, 8, obj + 0xd0]).eax;
    g.write::<u32>(obj + 0xdc, ONE);
    g.write::<u32>(obj + 0x104, 0);
    Ret::eax(r)
}

/// [`attach`] (60 bytes, 1 overlay).
pub const ATTACH: Sig = Sig {
    len: 60,
    hash: 0x687a_4596_66fa_a5a2,
    calls: &[],
    imports: &[],
};

/// A callback (0x10002460, no arguments): the camera point `+0x1ba0` =
/// the position `+0x1c10` with `y - 40`, then a tail jump to slot 633
/// (`ecx` = the state).
pub fn camera_above<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let s = g.read::<u32>(img.word(f + 0x02));
    let x = g.read::<u32>(s + 0x1c10);
    let y = g.read::<f32>(s + 0x1c14) - g.read::<f32>(img.word(f + 0x18));
    g.write::<u32>(s + 0x1ba0, x);
    let z = g.read::<u32>(s + 0x1c18);
    g.write::<u32>(s + 0x1ba8, z);
    let w = g.read::<u32>(s + 0x1c1c);
    g.write::<u32>(s + 0x1bac, w);
    g.write::<f32>(s + 0x1ba4, y);
    Ret::jump(slot_target(g, img.word(f + 0x3b), 633))
}

/// [`camera_above`] (79 bytes, 1 overlay).
pub const CAMERA_ABOVE: Sig = Sig {
    len: 79,
    hash: 0x3048_eedc_0e4e_f673,
    calls: &[],
    imports: &[],
};

/// A callback (0x100024b0, no arguments): 1 when there is no target
/// model (`+0x1eec` = -1), else 0.
pub fn no_target<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let s = g.read::<u32>(img.word(fr.at + 0x02));
    Ret::eax(u32::from(g.read::<u32>(s + 0x1eec) == 0xffff_ffff))
}

/// [`no_target`] (19 bytes, 1 overlay).
pub const NO_TARGET: Sig = Sig {
    len: 19,
    hash: 0xd142_7701_efc8_25ff,
    calls: &[],
    imports: &[],
};

/// Places the summon and its camera (0x100024d0, no arguments; `esp`
/// the entry stack pointer): `ang` = the facing `+0x1ee0` + 3/4 pi
/// wrapped once; the camera eye `+0x1c30` = the position (the locals
/// `ebp - 0x14..-0x8`) moved by `(0, -17, 100)` turned by X 0 and Y
/// `ang` (slots 235, 328, 329 into the matrix at `ebp - 0x74`, slot 229
/// into `ebp - 0x34`, slot 248), `w` = 1; the target `+0x1c40` = the
/// position moved by `(0, -40, -10)` the same way (the matrix at `ebp -
/// 0xb4`); the countdown `+0x1ef4` = 45, the model turns `+0x1bf0..` = (0,
/// ang, 0, 0) and the model frame `+0x1bb0` built ([`scale_model`] for
/// the caster); `+0x1be0` and `+0x1c00` get the moved position; then
/// the caster's root matrix (slot 210, or for a null frame the battle
/// character moved to its own position, slots 217 and 255) and its
/// original matrix (slot 211). Returns slot 211's `eax`. (The frame's
/// stack cookie is the CRT's; nothing else reads it.)
#[allow(clippy::too_many_lines)] // one straight sequence of calls
pub fn place<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let l = |off: u32| ebp.wrapping_sub(off);
    let s = g.read::<u32>(w(0x18));
    let ang = g.read::<f32>(s + 0x1ee0) + g.read::<f32>(w(0x30));
    for k in 0..4 {
        let v = g.read::<u32>(s + 0x1c10 + 4 * k);
        g.write::<u32>(l(0x14) + 4 * k, v);
    }
    let ang = wrap_once(g, img, f, ang, [0x51, 0x75, 0x7f, 0x8c]);
    let a = ang.to_bits();
    let (m1, m2, v, out, p) = (l(0x74), l(0xb4), l(0x24), l(0x34), l(0x14));
    for (k, x) in [0u32, 0xc188_0000, 0x42c8_0000, ONE]
        .into_iter()
        .enumerate()
    {
        g.write::<u32>(v + 4 * k as u32, x);
    }
    let s235 = slot_target(g, w(0x9d), 235);
    g.call(s235, 0, &[m1]);
    let s328 = slot_target(g, w(0xd2), 328);
    g.call(s328, 0, &[m1, m1, 0]);
    let s329 = slot_target(g, w(0xf4), 329);
    g.call(s329, 0, &[m1, m1, a]);
    g.write::<u32>(v + 0xc, ONE);
    let s229 = slot_target(g, w(0x10d), 229);
    let mut stack = vec![out, m1, v, m1, m1, a];
    g.call(s229, 0, &stack);
    let s248 = slot_target(g, w(0x12a), 248);
    stack.splice(0..0, [p, p, out]);
    // call width: 9 (the earlier calls' words stay on the stack)
    g.call(s248, 0, &stack);
    for k in 0..3 {
        let x = g.read::<u32>(p + 4 * k);
        g.write::<u32>(s + 0x1c30 + 4 * k, x);
    }
    g.write::<u32>(s + 0x1c3c, ONE);
    let tgt = s + 0x1c40;
    for k in 0..4 {
        let x = g.read::<u32>(s + 0x1c10 + 4 * k);
        g.write::<u32>(tgt + 4 * k, x);
    }
    g.write::<u32>(l(0x8), ONE);
    for (k, x) in [0u32, 0xc220_0000, 0xc120_0000, ONE]
        .into_iter()
        .enumerate()
    {
        g.write::<u32>(v + 4 * k as u32, x);
    }
    let s235 = slot_target(g, w(0x1a5), 235);
    stack.insert(0, m2);
    // call width: 10 at most (the earlier calls' words stay on the stack)
    g.call(s235, 0, &stack);
    let s328 = slot_target(g, w(0x1e7), 328);
    g.call(s328, 0, &[m2, m2, 0]);
    let s329 = slot_target(g, w(0x20c), 329);
    g.call(s329, 0, &[m2, m2, a]);
    g.write::<u32>(v + 0xc, ONE);
    let s229 = slot_target(g, w(0x228), 229);
    let mut stack = vec![out, m2, v, m2, m2, a];
    g.call(s229, 0, &stack);
    let s248 = slot_target(g, w(0x240), 248);
    stack.splice(0..0, [tgt, tgt, out]);
    g.call(s248, 0, &stack);
    g.write::<u32>(tgt + 0xc, ONE);
    g.write::<u32>(s + 0x1ef4, 0x2d);
    g.write::<u32>(s + 0x1bf0, 0);
    g.write::<f32>(s + 0x1bf4, ang);
    g.write::<u32>(s + 0x1bf8, 0);
    g.write::<u32>(s + 0x1bfc, 0);
    let m = s + 0x1bb0;
    let ops = ModelOps {
        s235: 0x257,
        c277: caster(0x2a2, 0x2ad),
        s277: 0x2b2,
        k: 0x2d0,
        c: caster(0x2e3, 0x2ee),
        s15: 0x2f3,
        s16: 0x310,
        s17: 0x332,
        s330: 0x355,
        s328: 0x381,
        s329: 0x3a0,
    };
    let turns = [
        Angle::At(s + 0x1bf8),
        Angle::At(s + 0x1bf0),
        Angle::At(s + 0x1bf4),
    ];
    let _ = scale_model(g, img, f, &ops, m, &stack, turns);
    let moved: Vec<u32> = (0..4).map(|k| g.read::<u32>(p + 4 * k)).collect();
    for (k, x) in (0u32..).zip(&moved) {
        g.write::<u32>(s + 0x1be0 + 4 * k, *x);
    }
    for (k, x) in (0u32..).zip(&moved) {
        g.write::<u32>(s + 0x1c00 + 4 * k, *x);
    }
    let selp = g.read::<u32>(w(0x40b)).wrapping_add(1);
    let i = u32::from(g.read::<u8>(selp));
    let chr = g.read::<u32>(w(0x419).wrapping_add(4 * i));
    if m == 0 {
        if g.read::<u32>(w(0x437)) == 0 && i.cast_signed() < 0x20 {
            let s217 = slot_target(g, w(0x448), 217);
            g.call(s217, 0, &[i, out]);
            let s255 = slot_target(g, w(0x45a), 255);
            g.call(s255, 0, &[i, out, i, out]);
        }
    } else {
        let s210 = slot_target(g, w(0x422), 210);
        g.call(s210, 0, &[chr, m]);
    }
    let s211 = slot_target(g, w(0x46b), 211);
    Ret::eax(g.call(s211, 0, &[chr, m]).eax)
}

/// [`place`] (1,165 bytes, 1 overlay).
pub const PLACE: Sig = Sig {
    len: 1165,
    hash: 0x8f75_90ed_724a_f680,
    // __security_check_cookie
    calls: &[(0x485, Callee::Any)],
    imports: &[],
};

/// The model's per-frame update (0x10002960, no arguments): the
/// countdown `+0x1ef4` blends the camera eye into the target (`t =
/// count / 45` after the count drops, 1 once it is out: `+0x1c00 =
/// eye * t + target * (1 - t)`, `w` = 1); the model's X turn `+0x1bf0`
/// grows by `D` (0.3 pi) wrapped once; the model frame `+0x1bb0` is
/// built again ([`scale_model`]). Returns slot 329's `eax`.
pub fn update<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let one = g.read::<f32>(w(0x08));
    let s = g.read::<u32>(w(0x0f));
    let n = g.read::<u32>(s + 0x1ef4).cast_signed();
    let t = if n > 0 {
        let n = n - 1;
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps rounds the same
        let t = n as f32 / g.read::<f32>(w(0x31));
        g.write::<u32>(s + 0x1ef4, n.cast_unsigned());
        t
    } else {
        one
    };
    let u = one - t;
    let r = |k: u32| g.read::<f32>(s + k);
    let x = r(0x1c30) * t + r(0x1c40) * u;
    let y = r(0x1c34) * t + r(0x1c44) * u;
    let z = r(0x1c38) * t + r(0x1c48) * u;
    g.write::<f32>(s + 0x1c00, x);
    g.write::<f32>(s + 0x1c04, y);
    g.write::<u32>(s + 0x1c0c, ONE);
    g.write::<f32>(s + 0x1c08, z);
    let a = g.read::<f32>(s + 0x1bf0) + g.read::<f32>(w(0xc0));
    let a = wrap_once(g, img, f, a, [0xc7, 0xd1, 0xdb, 0xe8]);
    g.write::<f32>(s + 0x1bf0, a);
    let ops = ModelOps {
        s235: 0xed,
        c277: caster(0x109, 0x114),
        s277: 0x119,
        k: 0x131,
        c: caster(0x144, 0x14f),
        s15: 0x154,
        s16: 0x16b,
        s17: 0x187,
        s330: 0x1a4,
        s328: 0x1d0,
        s329: 0x1ef,
    };
    let turns = [
        Angle::At(s + 0x1bf8),
        Angle::At(s + 0x1bf0),
        Angle::At(s + 0x1bf4),
    ];
    Ret::eax(scale_model(g, img, f, &ops, s + 0x1bb0, &[], turns))
}

/// [`update`] (527 bytes, 1 overlay).
pub const UPDATE: Sig = Sig {
    len: 527,
    hash: 0xa227_29a2_cf49_be92,
    calls: &[],
    imports: &[],
};

/// A callback (0x10002c90, no arguments): slot 939's phase picks the
/// case through the function's jump table (in this DLL 1 and 2 return
/// it; 0, 3 and above cue the target's sound: slot 220 `(slot 719 (E,
/// 6, +0x1ef0), 0x40, 0x100)`, its `eax` returned).
pub fn cue<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s939 = slot_target(g, w(0x01), 939);
    let state = g.read::<u32>(w(0x0e));
    let r = g.call(s939, 0, &[]).eax;
    // ja to +0x20; else jmp [eax * 4 + TABLE] (the table's entries are
    // relocated words, read from the image)
    let to = if r > 3 {
        f + 0x20
    } else {
        img.word(w(0x1c).wrapping_add(4 * r))
    };
    if to == f + 0x52 {
        return Ret::eax(r);
    }
    let s719 = slot_target(g, w(0x27), 719);
    let mut stack = vec![w(0x34), 6, g.read::<u32>(state + 0x1ef0)];
    let e = g.call(s719, 0, &stack).eax;
    let s220 = slot_target(g, w(0x43), 220);
    stack.splice(0..0, [e, 0x40, 0x100]);
    Ret::eax(g.call(s220, 0, &stack).eax)
}

/// [`cue`] (100 bytes with its jump table, 1 overlay).
pub const CUE: Sig = Sig {
    len: 100,
    hash: 0xc80c_6c5e_e925_753c,
    calls: &[],
    imports: &[],
};

/// An object's frame callback (0x10002d00, cdecl `(obj)`; `esp` the
/// entry stack pointer): the screen and camera matrices into the locals
/// at `ebp - 0x94` and `ebp - 0x54` (slots 723, 724), `obj + 0x80` set
/// from the camera (slot 327), `obj + 0xd0` = `(0, 0, 5)` through it
/// (slot 229), `w` = 1; the scale `+0x60..+0x6c` = `1024 / depth * K`
/// (slot 282's ST0); with the frame count `+0x118` at most 8 the colour
/// words `+0x70..+0x76` fade in (`count * 4096 / 8`), after 27 frames
/// they fade out with the countdown `+0x120` (`* 4096 / 6`). Returns
/// the last colour (slot 282's `eax` between the two).
pub fn frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ebp = fr.esp.wrapping_sub(4);
    let (scr, cam, v) = (ebp - 0x94, ebp - 0x54, ebp - 0x14);
    let obj = fr.arg(g, 0);
    let s723 = slot_target(g, w(0x20), 723);
    let mut stack = vec![scr];
    g.call(s723, 0, &stack);
    let s724 = slot_target(g, w(0x31), 724);
    stack.insert(0, cam);
    // call width: 7 at most (the earlier calls' words stay on the stack)
    g.call(s724, 0, &stack);
    let m = obj + 0x80;
    let s327 = slot_target(g, w(0x42), 327);
    stack.splice(0..0, [m, cam]);
    // call width: 7 at most (the earlier calls' words stay on the stack)
    g.call(s327, 0, &stack);
    for (k, x) in [0u32, 0, 0x40a0_0000, ONE].into_iter().enumerate() {
        g.write::<u32>(v + 4 * k as u32, x);
    }
    let s229 = slot_target(g, w(0x62), 229);
    stack.splice(0..0, [obj + 0xd0, m, v]);
    // call width: 7 (the earlier calls' words stay on the stack)
    g.call(s229, 0, &stack);
    g.write::<u32>(obj + 0xdc, ONE);
    let s282 = slot_target(g, w(0x8b), 282);
    let out = g.call(s282, 0, &[]);
    let d = st0_f32(out.st0);
    let x = g.read::<f32>(w(0xae)) / d;
    let n = g.read::<u32>(obj + 0x118).cast_signed();
    let x = x * g.read::<f32>(w(0xc4));
    for off in [0x6c, 0x68, 0x64, 0x60] {
        g.write::<f32>(obj + off, x);
    }
    let colour = |g: &mut G, c: i32| {
        for off in [0x76, 0x74, 0x72, 0x70] {
            g.write::<u16>(obj + off, c.cast_unsigned() as u16);
        }
    };
    let mut eax = out.eax;
    if n <= 8 {
        let c = n.wrapping_shl(12) / 8;
        colour(g, c);
        eax = c.cast_unsigned();
    }
    if n > 0x1b {
        let k = g.read::<u32>(obj + 0x120).cast_signed();
        if k > 0 {
            g.write::<u32>(obj + 0x120, (k - 1).cast_unsigned());
        }
        let k = g.read::<u32>(obj + 0x120).cast_signed();
        let c = k.wrapping_shl(12) / 6;
        colour(g, c);
        eax = c.cast_unsigned();
    }
    Ret::eax(eax)
}

/// [`frame`] (332 bytes, 1 overlay).
pub const FRAME: Sig = Sig {
    len: 332,
    hash: 0x14e2_9b7a_5bd1_77aa,
    // __security_check_cookie
    calls: &[(0x144, Callee::Any)],
    imports: &[],
};

/// A callback (0x10003d90, no arguments): the camera eye `+0x1c30` =
/// the point `+0x1ba0` with its `y` = the position's `y - 100`; the turn
/// `+0x1b90` grows by pi, wrapped once; `+0x1efc` = 20. Returns the
/// point's `w`.
pub fn eye<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s = g.read::<u32>(w(0x02));
    let mut last = 0;
    for k in 0..4 {
        last = g.read::<u32>(s + 0x1ba0 + 4 * k);
        g.write::<u32>(s + 0x1c30 + 4 * k, last);
    }
    let y = g.read::<f32>(s + 0x1c14) - g.read::<f32>(w(0x42));
    g.write::<f32>(s + 0x1c34, y);
    let a = g.read::<f32>(s + 0x1b90) + g.read::<f32>(w(0x5a));
    let a = wrap_once(g, img, f, a, [0x61, 0x6b, 0x86, 0x93]);
    g.write::<f32>(s + 0x1b90, a);
    g.write::<u32>(s + 0x1efc, 0x14);
    Ret::eax(last)
}

/// [`eye`] (170 bytes, 1 overlay).
pub const EYE: Sig = Sig {
    len: 170,
    hash: 0x09f6_b006_b06c_a7b6,
    calls: &[],
    imports: &[],
};
