//! `magic_0481.dll`'s own functions (besides the runtime, the OPU host
//! and the shared code): the shape-burst records (128 of 0x30 bytes at
//! `G_REC`: a position `+0x0`, a velocity `+0x10`, a scale `+0x20`, the
//! shape `+0x26`, a frame count `+0x28`, live while non-zero), the
//! effect's start and end, and the facing matrix of the per-frame code.

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// The first free record (0x10001040, no arguments): the first of the
/// 128 at `G_REC` (operand `+0x06`) whose count `+0x28` is 0, else 0.
pub fn free_record<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let base = img.word(f + 0x06);
    (0..0x80u32)
        .map(|i| base.wrapping_add(0x30 * i))
        .find(|&r| g.read::<u16>(r + 0x28) == 0)
        .unwrap_or(0)
}

/// [`free_record`] (34 bytes, 1 overlay).
pub const FREE_RECORD: Sig = Sig {
    len: 34,
    hash: 0xd88e_f91a_e402_b524,
    calls: &[],
    imports: &[],
};

/// Draws the live records (0x100011c0, no arguments): a sprite block
/// from slot 264 `(0xc0)`; its `+0xb0` is the `+0x4` of slot 941's
/// `(*G_SHAPES)` result and its `+0xb4` the game's draw cursor
/// `**G_CURSOR`. For each record with a count: a unit matrix at `+0x30`
/// (slot 235) scaled by the record's `+0x20` on the diagonal, the
/// record's position moved by `*G_VIEW` into `+0x60` (slot 229), the
/// matrix copied to `+0x70` and itself (slot 230), `+0xa8 -= K` (100), the
/// matrix at `+0x70` copied to itself, the sprite's fields set and the
/// record's shape drawn (slot 242 `(shapes + shapes[shape].+0x10,
/// sprite)`, its result the new cursor); unless `**G_PAUSE` is set, the
/// position moves by the velocity, `+0x14 += 1.0` and the count drops.
/// Then the cursor goes back to `**G_CURSOR` and slot 265 `(0xc0)`
/// frees the block. Returns slot 265's `eax`.
pub fn draw_records<G: Game>(g: &mut G, img: &Image, f: u32) -> u32 {
    let w = |off: u32| img.word(f + off);
    let s264 = slot_target(g, w(0x07), 264);
    let spr = g.call(s264, 0, &[0xc0]).eax;
    let s941 = slot_target(g, w(0x22), 941);
    let shapes_arg = g.read::<u32>(w(0x28));
    let p = g.call(s941, 0, &[shapes_arg, 0xc0]).eax;
    let shapes = g.read::<u32>(p + 4);
    g.write::<u32>(spr + 0xb0, shapes);
    let cursor = g.read::<u32>(w(0x43));
    let v = g.read::<u32>(cursor);
    g.write::<u32>(spr + 0xb4, v);
    // ebx: the record's +0x26
    let first = w(0x53);
    let one = g.read::<f32>(w(0x178));
    let k = g.read::<f32>(w(0xd9));
    for left in (0..0x80u32).rev() {
        let rec = first.wrapping_add(0x30 * (0x7f - left)).wrapping_sub(0x26);
        if g.read::<u16>(rec + 0x28) == 0 {
            continue;
        }
        let m30 = spr + 0x30;
        let s235 = slot_target(g, w(0x67), 235);
        let mut stack = vec![m30];
        g.call(s235, 0, &stack);
        let scale = g.read::<u32>(rec + 0x20);
        let view = g.read::<u32>(w(0x85));
        g.write::<u32>(spr + 0x58, scale);
        g.write::<u32>(spr + 0x44, scale);
        g.write::<u32>(m30, scale);
        let s229 = slot_target(g, w(0x8e), 229);
        let a = [spr + 0x60, view, rec];
        stack.splice(0..0, a);
        g.call(s229, 0, &stack);
        let m70 = spr + 0x70;
        let s230 = slot_target(g, w(0xa9), 230);
        stack.splice(0..0, [m70, m30]);
        g.call(s230, 0, &stack);
        let s230 = slot_target(g, w(0xbb), 230);
        stack.splice(0..0, [m30, m30]);
        g.call(s230, 0, &stack);
        let z = g.read::<f32>(spr + 0xa8) - k;
        g.write::<f32>(spr + 0xa8, z);
        let s230 = slot_target(g, w(0xe7), 230);
        stack.splice(0..0, [m70, m70]);
        g.call(s230, 0, &stack);
        g.write::<u32>(spr + 0x10, m30);
        g.write::<u32>(spr + 0x8, 0);
        let shapes = g.read::<u32>(spr + 0xb0);
        g.write::<u8>(spr + 0x18, 0);
        g.write::<u32>(spr + 0x1c, m70);
        g.write::<u16>(spr + 0x2, 0x80);
        g.write::<u32>(spr + 0x4, 0x7080_8080);
        let shape = u32::from(g.read::<u16>(rec + 0x26));
        let off = g.read::<i16>(shapes.wrapping_add(8 * shape).wrapping_add(0x10));
        let c = g.read::<u32>(spr + 0xb4);
        g.write::<u32>(spr + 0x14, c);
        let s242 = slot_target(g, w(0x132), 242);
        let shape = shapes.wrapping_add(i32::from(off).cast_unsigned());
        stack.splice(0..0, [shape, spr]);
        // call width: 12 (the loop's six calls leave all their words)
        let c = g.call(s242, 0, &stack).eax;
        g.write::<u32>(spr + 0xb4, c);
        let pause = g.read::<u32>(w(0x14b));
        if g.read::<u32>(pause) != 0 {
            continue;
        }
        let x = g.read::<f32>(rec + 0x10) + g.read::<f32>(rec);
        let vy = g.read::<f32>(rec + 0x14);
        g.write::<f32>(rec, x);
        let y = g.read::<f32>(rec + 0x4) + vy;
        let vy = vy + one;
        g.write::<f32>(rec + 0x4, y);
        let z = g.read::<f32>(rec + 0x18) + g.read::<f32>(rec + 0x8);
        g.write::<f32>(rec + 0x14, vy);
        g.write::<f32>(rec + 0x8, z);
        let n = g.read::<u16>(rec + 0x28);
        g.write::<u16>(rec + 0x28, n.wrapping_sub(1));
    }
    let cursor = g.read::<u32>(w(0x1a5));
    let c = g.read::<u32>(spr + 0xb4);
    g.write::<u32>(cursor, c);
    let s265 = slot_target(g, w(0x1b2), 265);
    g.call(s265, 0, &[0xc0]).eax
}

/// [`draw_records`] (461 bytes, 1 overlay).
pub const DRAW_RECORDS: Sig = Sig {
    len: 461,
    hash: 0xdd1f_a4a2_7b2d_412f,
    calls: &[],
    imports: &[],
};

/// The table's start entry (0x10001390, cdecl `(arg)`): keeps `arg` in
/// `G_ARG`; a 1 MiB block from slot 803 `(0x100000, 0x10)` into `G_BUF`;
/// slot 861 `(buf, G_EFFECT, 0x6b280)`; slot 259; `G_CHR` = slot 260
/// `(0)`; the effect's slots 718, 222 and 221 (`(G_EFFECT, 0, 1000,
/// 0x100)` for the cues); every record's count cleared and 24 state
/// words zeroed. Each call sees the words the earlier ones left on the
/// stack. Returns 0.
pub fn start<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let arg = fr.arg(g, 0);
    g.write::<u32>(w(0x07), arg);
    let s803 = slot_target(g, w(0x0c), 803);
    let mut stack = vec![0x10_0000, 0x10];
    let buf = g.call(s803, 0, &stack).eax;
    g.write::<u32>(w(0x25), buf);
    let s861 = slot_target(g, w(0x30), 861);
    stack.splice(0..0, [buf, w(0x2a), 0x6_b280]);
    g.call(s861, 0, &stack);
    let s259 = slot_target(g, w(0x3d), 259);
    g.call(s259, 0, &stack);
    let s260 = slot_target(g, w(0x4a), 260);
    stack.insert(0, 0);
    // call width: 15 at most (the earlier calls' words stay on the stack)
    let chr = g.call(s260, 0, &stack).eax;
    g.write::<u32>(w(0x59), chr);
    let s718 = slot_target(g, w(0x5e), 718);
    stack.insert(0, w(0x63));
    // call width: 15 at most (the earlier calls' words stay on the stack)
    g.call(s718, 0, &stack);
    let s222 = slot_target(g, w(0x70), 222);
    stack.splice(0..0, [w(0x87), 0, 1000, 0x100]);
    // call width: 11 and 15 (the earlier calls' words stay on the stack)
    g.call(s222, 0, &stack);
    let s221 = slot_target(g, w(0x8e), 221);
    stack.splice(0..0, [w(0xa5), 0, 1000, 0x100]);
    // call width: 15 at most (the earlier calls' words stay on the stack)
    g.call(s221, 0, &stack);
    // do-while over the counts: `cmp eax, END; jl`
    let end = w(0xbc);
    let mut a = w(0xaf);
    loop {
        g.write::<u16>(a, 0);
        a = a.wrapping_add(0x30);
        if a.cast_signed() >= end.cast_signed() {
            break;
        }
    }
    for off in (0xc4..=0x14e).step_by(6) {
        g.write::<u32>(w(off), 0);
    }
    Ret::eax(0)
}

/// [`start`] (342 bytes, 1 overlay).
pub const START: Sig = Sig {
    len: 342,
    hash: 0xf08a_eca1_67fc_521f,
    calls: &[],
    imports: &[],
};

/// The table's end entry (0x100014f0, no arguments): the effect's two
/// object pools freed (slot 713, each with `*G_PPP = G_ENV` first), the
/// block freed (slot 804 `(G_BUF)`) and `G_BUF` cleared. Returns 0.
pub fn end<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let f = fr.at;
    let w = |off: u32| img.word(f + off);
    let ppp = g.read::<u32>(w(0x01));
    g.write::<u32>(ppp, w(0x0c));
    let s713 = slot_target(g, w(0x11), 713);
    let mut stack = vec![w(0x06)];
    g.call(s713, 0, &stack);
    let ppp = g.read::<u32>(w(0x1e));
    g.write::<u32>(ppp, w(0x29));
    let s713 = slot_target(g, w(0x2e), 713);
    stack.insert(0, w(0x23));
    // call width: 3 at most (the earlier calls' words stay on the stack)
    g.call(s713, 0, &stack);
    let s804 = slot_target(g, w(0x3b), 804);
    stack.insert(0, g.read::<u32>(w(0x41)));
    // call width: 3 at most (the earlier calls' words stay on the stack)
    g.call(s804, 0, &stack);
    g.write::<u32>(w(0x52), 0);
    Ret::eax(0)
}

/// [`end`] (93 bytes, 1 overlay).
pub const END: Sig = Sig {
    len: 93,
    hash: 0xf86d_849c_2606_21eb,
    calls: &[],
    imports: &[],
};

/// The facing matrix (0x100017f0, `edx` = a direction, `esp` the entry
/// stack pointer): `a` = the direction normalised (slot 232 into the
/// local at `ebp - 0x24`), `b = (a.z, 0, -a.x)` normalised (`ebp -
/// 0x14`), `c = a x b` normalised (slot 266, slot 232; `ebp - 0x34`);
/// `*G_MATRIX` gets the rows `b`, `c`, `a` (`w` = 0) and `(0, 0, 0, 1)`.
/// (The frame's stack cookie is the CRT's; nothing else reads it.)
pub fn facing<G: Game>(g: &mut G, img: &Image, f: u32, dir: u32, esp: u32) {
    let w = |off: u32| img.word(f + off);
    let ebp = esp.wrapping_sub(4);
    let (a, b, c) = (
        ebp.wrapping_sub(0x24),
        ebp.wrapping_sub(0x14),
        ebp.wrapping_sub(0x34),
    );
    let m = g.read::<u32>(w(0x13));
    let s232 = slot_target(g, w(0x1d), 232);
    let mut stack = vec![a, dir];
    g.call(s232, 0, &stack);
    let az = g.read::<u32>(a + 8);
    g.write::<u32>(b, az);
    // xorps with the sign mask
    let ax = g.read::<u32>(a) ^ g.read::<u32>(w(0x3e));
    g.write::<u32>(b + 4, 0);
    g.write::<u32>(b + 8, ax);
    let s232 = slot_target(g, w(0x45), 232);
    stack.splice(0..0, [b, b]);
    g.call(s232, 0, &stack);
    let s266 = slot_target(g, w(0x6a), 266);
    stack.splice(0..0, [c, a, b]);
    g.call(s266, 0, &stack);
    let s232 = slot_target(g, w(0x7c), 232);
    stack.splice(0..0, [c, c]);
    // call width: 9 (the earlier calls' words stay on the stack)
    g.call(s232, 0, &stack);
    for (dst, src) in [
        (0x00, b),
        (0x04, b + 4),
        (0x08, b + 8),
        (0x10, c),
        (0x14, c + 4),
        (0x18, c + 8),
        (0x20, a),
        (0x24, a + 4),
    ] {
        let v = g.read::<u32>(src);
        g.write::<u32>(m + dst, v);
    }
    let az = g.read::<u32>(a + 8);
    for (dst, v) in [
        (0x0c, 0),
        (0x1c, 0),
        (0x28, az),
        (0x2c, 0),
        (0x30, 0),
        (0x34, 0),
        (0x38, 0),
        (0x3c, 0x3f80_0000),
    ] {
        g.write::<u32>(m + dst, v);
    }
}

/// [`facing`] (292 bytes, 1 overlay).
pub const FACING: Sig = Sig {
    len: 292,
    hash: 0xb79e_02af_ef9b_7fbc,
    // __security_check_cookie
    calls: &[(0x11c, Callee::Any)],
    imports: &[],
};

/// Spawns `n` records (0x10001070, internal: `edx` = `n`, `xmm2` = `a`,
/// `xmm3` = `b`, `esp` at the return address). `k` = slot 263 with
/// `(*G_KIND)` (operand `+0x0e`), once. Then for each record taken
/// ([`free_record`], the `rel32` at `+0x32`; none left ends the loop):
/// halfword `+0x24` 0, `+0x26` = 0x80000, and five draws of slot 241
/// with `(&r)`, a float into the local `r` at `ebp - 4`. The shape: slot
/// 237 with `(*G_KIND, trunc(k * r), 0, rec)`; `+0xc` = 1.0; `+0x20` =
/// `(r + C1) * C2` (operands `+0xa2`, `+0xb3`); `+0x10` = `(r + a) * b`;
/// `+0x18` = `(r - D1) * D2` and `+0x14` = `(r + D1) * D3` in double
/// precision (`+0xf8`, `+0x106`, `+0x132`). Each call sees the loop
/// body's earlier words on the stack after its own. Returns the last
/// call's `eax`, or 0 when the loop ends for want of a record.
#[allow(clippy::too_many_arguments)]
pub fn spawn<G: Game>(g: &mut G, img: &Image, f: u32, esp: u32, n: u32, a: f32, b: f32) -> u32 {
    use crate::sb_sse::{add, cvt, mul};
    let w = |off: u32| img.word(f + off);
    let f32_at = |g: &G, at: u32| f32::from_bits(g.read::<u32>(at));
    let f64_at = |g: &G, at: u32| f64::from_bits(g.read::<u64>(at));
    // the frame: ebp = esp - 4; r at ebp - 4
    let r_at = esp.wrapping_sub(8);
    let kind = g.read::<u32>(w(0x0e));
    let s263 = slot_target(g, w(0x07), 263);
    let k = g.call(s263, 0, &[kind]).eax;
    let mut left = n;
    loop {
        left = left.wrapping_sub(1);
        let rec = free_record(g, img, img.rel32_target(f + 0x32));
        if rec == 0 {
            return 0;
        }
        g.write::<u16>(rec + 0x24, 0);
        g.write::<u32>(rec + 0x26, 0x0008_0000);
        let s241 = slot_target(g, w(0x4b), 241);
        g.call(s241, 0, &[r_at]);
        // cvtdq2ps, mulss, cvttss2si
        let x = mul(cvt(k), f32_at(g, r_at));
        let shape = ffx_bridge::x87::cvttsd2si(f64::from(x));
        let s237 = slot_target(g, w(0x74), 237);
        g.call(s237, 0, &[kind, shape, 0, rec, r_at]);
        g.write::<u32>(rec + 0xc, 0x3f80_0000);
        let s241 = slot_target(g, w(0x86), 241);
        g.call(s241, 0, &[r_at, kind, shape, 0, rec, r_at]);
        let scale = mul(add(f32_at(g, r_at), f32_at(g, w(0xa2))), f32_at(g, w(0xb3)));
        g.write::<u32>(rec + 0x20, scale.to_bits());
        let s241 = slot_target(g, w(0xab), 241);
        g.call(s241, 0, &[r_at, r_at, kind, shape, 0, rec, r_at]);
        let vx = mul(add(f32_at(g, r_at), a), b);
        g.write::<u32>(rec + 0x10, vx.to_bits());
        let s241 = slot_target(g, w(0xd3), 241);
        g.call(s241, 0, &[r_at, r_at, r_at, kind, shape, 0, rec, r_at]);
        // cvtps2pd, subsd, mulsd, cvtpd2ps
        let d1 = f64_at(g, w(0xf8));
        let vz = (f64::from(f32_at(g, r_at)) - d1) * f64_at(g, w(0x106));
        g.write::<u32>(rec + 0x18, (vz as f32).to_bits());
        let s241 = slot_target(g, w(0xfe), 241);
        let eax = g
            .call(
                s241,
                0,
                &[r_at, r_at, r_at, r_at, kind, shape, 0, rec, r_at],
            )
            .eax;
        let vy = (f64::from(f32_at(g, r_at)) + f64_at(g, w(0x12a))) * f64_at(g, w(0x132));
        g.write::<u32>(rec + 0x14, (vy as f32).to_bits());
        if left == 0 {
            return eax;
        }
    }
}

/// [`spawn`] (334 bytes, 1 overlay).
pub const SPAWN: Sig = Sig {
    len: 334,
    hash: 0xeea7_f273_b1fa_d94a,
    calls: &[(0x32, Callee::Sig(&FREE_RECORD))],
    imports: &[],
};
