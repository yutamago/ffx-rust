//! `magic_0098.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::m0018::st0_f32;
use crate::objcb::{PartAt, imm_stores, part_start};
use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig, Xmm};

/// A target's height (0x100011a0, internal; the result in `xmm0`): slot
/// 390 `(G_T)` gives the target's model id, slot 19 of its handle (slot
/// 215 `(G_T)`; the first call's word left on the stack) its float (ST0,
/// stored as `f32`), corrected for three models: `+ A` for 0x1015 and
/// 0x1127, `- B` for 0x1077. `eax` is slot 19's.
pub fn target_height<G: Game>(g: &mut G, img: &Image, f: u32, ecx: u32) -> Ret {
    let w = |off: u32| img.word(f + off);
    let s390 = slot_target(g, w(0x05), 390);
    let t = g.read::<u32>(w(0x0c));
    let id = g.call(s390, ecx, &[t]).eax;
    let s215 = slot_target(g, w(0x1c), 215);
    // call width: 2 (the word and slot 390's)
    let h = g.call(s215, ecx, &[t, t]).eax;
    let s19 = slot_target(g, w(0x30), 19);
    let o = g.call(s19, ecx, &[h]);
    let v = st0_f32(o.st0);
    let v = match id {
        0x1015 => add(v, g.read::<f32>(w(0x98))),
        0x1077 => sub(v, g.read::<f32>(w(0x82))),
        0x1127 => add(v, g.read::<f32>(w(0x6c))),
        _ => v,
    };
    Ret::xmm0(Xmm::f32(v), o.eax)
}

/// [`target_height`] (161 bytes, 1 overlay).
pub const TARGET_HEIGHT: Sig = Sig {
    len: 161,
    hash: 0xc502_d317_55e3_19ac,
    calls: &[],
    imports: &[],
};

/// Builds a view matrix (0x10001250, internal: `ecx` = the matrix, 16
/// floats): slot 325 `(&D, DIR)`, `D` with its y cleared normalised
/// (slot 232) into `F`, the axes `S = F x UP` and `U = S x F` (slot 266,
/// `UP = (0, 1, 0, 1)` at `ebp - 0x44`), both normalised; the rows `S`,
/// `U`, `F` (their `w` 0) and the position from slot 325 `(m + 0x30,
/// POS)`, `+0x3c` 1.0. Locals: `D` `ebp - 0x54`, `F` `- 0x24`, `S` `-
/// 0x14`, `U` `- 0x34`. Each call gets the words the earlier ones left
/// on the stack. Returns slot 325's result.
#[allow(clippy::similar_names)]
pub fn view_matrix<G: Game>(g: &mut G, img: &Image, f: u32, m: u32, esp: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let ebp = esp - 4;
    let (d, up, fw, s, u) = (ebp - 0x54, ebp - 0x44, ebp - 0x24, ebp - 0x14, ebp - 0x34);
    let s325 = slot_target(g, w(0x1b), 325);
    let mut stack = vec![d, w(0x15)];
    g.call(s325, m, &stack);
    g.write::<u32>(d + 4, 0);
    let s232 = slot_target(g, w(0x32), 232);
    for (o, v) in [(0u32, 0u32), (4, 0x3f80_0000), (8, 0), (0xc, 0x3f80_0000)] {
        g.write::<u32>(up + o, v);
    }
    stack.splice(0..0, [fw, d]);
    // call width: 4
    g.call(s232, m, &stack);
    let s266 = slot_target(g, w(0x6e), 266);
    stack.splice(0..0, [s, up, fw]);
    // call width: 7
    g.call(s266, m, &stack);
    let s266 = slot_target(g, w(0x87), 266);
    stack.splice(0..0, [u, fw, s]);
    // call width: 10
    g.call(s266, m, &stack);
    let s232 = slot_target(g, w(0x99), 232);
    stack.splice(0..0, [s, s]);
    // call width: 12
    g.call(s232, m, &stack);
    let s232 = slot_target(g, w(0xab), 232);
    stack.splice(0..0, [u, u]);
    // call width: 14
    g.call(s232, m, &stack);
    for (row, src) in [(0u32, s), (0x10, u), (0x20, fw)] {
        for k in 0..3u32 {
            let v = g.read::<u32>(src + 4 * k);
            g.write::<u32>(m + row + 4 * k, v);
        }
        if row == 0x20 {
            g.write::<u32>(m + 0x30, 0);
        }
    }
    let s325 = slot_target(g, w(0x111), 325);
    for o in [0xcu32, 0x1c, 0x2c, 0x34, 0x38] {
        g.write::<u32>(m + o, 0);
    }
    g.write::<u32>(m + 0x3c, 0x3f80_0000);
    stack.splice(0..0, [m + 0x30, w(0x10b)]);
    // call width: 16
    g.call(s325, m, &stack).eax
}

/// [`view_matrix`] (360 bytes, 1 overlay).
pub const VIEW_MATRIX: Sig = Sig {
    len: 360,
    hash: 0xfffb_be97_4cae_8d7b,
    // __security_check_cookie
    calls: &[(0x160, Callee::Any)],
    imports: &[],
};

/// A step (0x10002130, no arguments): `G_SCALE` = 1.0, slot 276 `(G_H,
/// 1)`. Returns 1.
pub fn show<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s276 = slot_target(g, w(0x01), 276);
    let h = g.read::<u32>(w(0x09));
    g.write::<u32>(w(0x15), w(0x19));
    g.call(s276, fr.ecx, &[h, 1]);
    Ret::eax(1)
}

/// 0x10002130 (40 bytes, 1 overlay).
pub const SHOW: Sig = Sig {
    len: 40,
    hash: 0xe40a_b4b0_8252_927f,
    calls: &[],
    imports: &[],
};

/// A step (0x10002210, no arguments): slot 276 `(G_H, 0)`, then `G_DONE`
/// = 1. Returns 1.
pub fn hide<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let s276 = slot_target(g, w(0x01), 276);
    let h = g.read::<u32>(w(0x09));
    g.call(s276, fr.ecx, &[h, 0]);
    g.write::<u32>(w(0x1a), 1);
    Ret::eax(1)
}

/// 0x10002210 (40 bytes, 1 overlay).
pub const HIDE: Sig = Sig {
    len: 40,
    hash: 0x8b2e_f348_6359_2958,
    calls: &[],
    imports: &[],
};

/// An object callback (0x10002660, `(obj)`): slots 230 `(G_C, VEC)`, 231
/// `(G_A, G_B, G_C)`, 230 `(G_D, G_A)` and slot 712 `(obj, 0x30)` with
/// `*G_P = ITEM` and the game flag `*G_LOCK` 1 around it (each call with
/// the words the earlier ones left). Returns 1 when 712 gave 0, else 0.
pub fn draw<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ecx = fr.ecx;
    let s230 = slot_target(g, w(0x04), 230);
    let c = g.read::<u32>(w(0x16));
    let mut stack = vec![c, w(0x10)];
    g.call(s230, ecx, &stack);
    let c = g.read::<u32>(w(0x1e));
    let s231 = slot_target(g, w(0x23), 231);
    let b = g.read::<u32>(w(0x29));
    let a = g.read::<u32>(w(0x35));
    stack.splice(0..0, [a, b, c]);
    // call width: 5
    g.call(s231, ecx, &stack);
    let a = g.read::<u32>(w(0x3d));
    let s230 = slot_target(g, w(0x42), 230);
    let d = g.read::<u32>(w(0x48));
    stack.splice(0..0, [d, a]);
    // call width: 7
    g.call(s230, ecx, &stack);
    let p = g.read::<u32>(w(0x55));
    let obj = fr.arg(g, 0);
    g.write::<u32>(p, w(0x60));
    let lock = g.read::<u32>(w(0x65));
    let saved = g.read::<u32>(lock);
    g.write::<u32>(lock, 1);
    let s712 = slot_target(g, w(0x72), 712);
    stack.splice(0..0, [obj, 0x30]);
    // call width: 9
    let r = g.call(s712, lock, &stack).eax;
    let lock = g.read::<u32>(w(0x80));
    g.write::<u32>(lock, saved);
    Ret::eax(u32::from(r == 0))
}

/// 0x10002660 (149 bytes, 1 overlay).
pub const DRAW: Sig = Sig {
    len: 149,
    hash: 0x91e2_46be_5b1e_d450,
    calls: &[],
    imports: &[],
};

/// An object callback (0x100029d0, `(obj)`): the part `list[0]` started
/// on the object ([`part_start`]), slot 325 `(obj + 0x40, POS)` (the part
/// start's words left below), its y `+0x44` raised by `H`, the model's
/// other words. Returns 1.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let obj = fr.arg(g, 0);
    let at = PartAt {
        gp: 0x04,
        item: 0x0b,
        gd: 0x10,
        table: 0x18,
        entry: 0,
        call: 0x35,
    };
    let (_, left) = part_start(g, img, f, fr.ecx, obj, at);
    let s325 = slot_target(g, w(0x43), 325);
    let mut stack = vec![obj + 0x40, w(0x3d)];
    stack.extend(left);
    // call width: 6
    g.call(s325, obj, &stack);
    let y = add(g.read::<f32>(obj + 0x44), g.read::<f32>(w(0x58)));
    imm_stores(g, img, f, obj, &[(0x50, 0x62), (0x54, 0x69), (0x58, 0x70)]);
    g.write::<f32>(obj + 0x44, y);
    imm_stores(
        g,
        img,
        f,
        obj,
        &[
            (0x5c, 0x7c),
            (0x60, 0x83),
            (0x64, 0x8a),
            (0x68, 0x91),
            (0x6c, 0x98),
        ],
    );
    Ret::eax(1)
}

/// 0x100029d0 (164 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 164,
    hash: 0xdd63_7ae1_16cf_22ad,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
