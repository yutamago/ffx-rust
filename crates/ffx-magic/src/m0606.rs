//! `magic_0606.dll`'s own functions (besides the runtime and the shared
//! code).

use crate::runtime::slot_target;
use crate::sb_sse::{add, sub};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// Shows the targets (0x10007a10, an overlay table entry, no arguments):
/// for each of the `*G_N` targets (operand `+0x05`, signed; the list
/// `*G_L`, `+0x19`): outside a battle (`+0x12` 0) a party member (the
/// list byte, or `G_ME` `+0x20` for target -1, below 32) gets slot 530
/// `(id, 0)`; otherwise slot 249 `(OBJS[id], 0)` (the table at `+0x55`;
/// `+0x48` for target -1). Returns the last call's `eax`, or the caller's
/// `eax` without targets.
pub fn show_targets<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut eax = fr.eax;
    let mut i = 0u32;
    while i.cast_signed() < g.read::<i32>(w(if i == 0 { 0x5 } else { 0x6f })) {
        let battle = g.read::<u32>(w(0x12)) != 0;
        let list = g.read::<u32>(w(0x19));
        let mut member = None;
        if !battle {
            let a = if i == u32::MAX {
                g.read::<u32>(w(0x20))
            } else {
                u32::from(g.read::<u8>(list.wrapping_add(i)))
            };
            if a.cast_signed() < 0x20 {
                member = Some(a);
            }
        }
        if let Some(a) = member {
            let t = slot_target(g, w(0x36), 530);
            eax = g.call(t, 0, &[a, 0]).eax;
        } else {
            let o = if i == u32::MAX {
                g.read::<u32>(w(0x48))
            } else {
                let b = u32::from(g.read::<u8>(list.wrapping_add(i)));
                g.read::<u32>(w(0x55).wrapping_add(4 * b))
            };
            let t = slot_target(g, w(0x5d), 249);
            eax = g.call(t, 0, &[o, 0]).eax;
        }
        i = i.wrapping_add(1);
    }
    Ret::eax(eax)
}

/// [`show_targets`] (119 bytes, 1 overlay).
pub const SHOW_TARGETS: Sig = Sig {
    len: 119,
    hash: 0xf005_e27b_623a_7a0d,
    calls: &[],
    imports: &[],
};

/// Places the effect's camera points (0x10008990, an overlay table
/// entry, no arguments; a frame with a stack cookie): the matrix `m` at
/// `ebp - 0x54` from the data at `+0x1b` (slot 327), the point `R +
/// 0x9f0` (`R = *G_R`, operand `+0x13`) through it into `v` at `ebp -
/// 0x14` (slot 229, the first call's words under it). The look point
/// (`+0x62`, `+0x77`, `+0x8c`) is `v + (A, -B, C)` (operands `+0x52`,
/// `+0x6f`, `+0x84`); when slot 520 says 0x1590000 its z is `v.z + D`
/// (`+0xac`). The eye (`+0xd1`, `+0xee`, `+0xe6`) is `v - (0, E, 0)`
/// (`+0xde`). Returns slot 520's `eax`.
pub fn camera_points<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let k = |g: &mut G, off: u32| g.read::<f32>(w(off));
    let ebp = fr.esp.wrapping_sub(4);
    let (m, v) = (ebp.wrapping_sub(0x54), ebp.wrapping_sub(0x14));
    let r = g.read::<u32>(w(0x13));
    let t = slot_target(g, w(0x21), 327);
    g.call(t, 0, &[m, w(0x1b)]);
    let t = slot_target(g, w(0x3d), 229);
    g.call(t, 0, &[v, m, r + 0x9f0, m, w(0x1b)]);
    let x = add(g.read::<f32>(v), k(g, 0x52));
    g.write::<f32>(w(0x62), x);
    let y = sub(g.read::<f32>(v + 4), k(g, 0x6f));
    g.write::<f32>(w(0x77), y);
    let z = add(g.read::<f32>(v + 8), k(g, 0x84));
    g.write::<f32>(w(0x8c), z);
    let t = slot_target(g, w(0x57), 520);
    let eax = g.call(t, 0, &[]).eax;
    let vz = g.read::<f32>(v + 8);
    if eax == 0x0159_0000 {
        let z = add(vz, k(g, 0xac));
        g.write::<f32>(w(0xb4), z);
    }
    let ex = sub(g.read::<f32>(v), 0.0);
    g.write::<f32>(w(0xd1), ex);
    let ey = sub(g.read::<f32>(v + 4), k(g, 0xde));
    g.write::<f32>(w(0xe6), sub(vz, 0.0));
    g.write::<f32>(w(0xee), ey);
    Ret::eax(eax)
}

/// [`camera_points`] (251 bytes, 1 overlay).
pub const CAMERA_POINTS: Sig = Sig {
    len: 251,
    hash: 0xa7d8_4779_751d_daab,
    // __security_check_cookie
    calls: &[(0xf3, Callee::Any)],
    imports: &[],
};
