//! `magic_0481.dll`'s own functions (besides the runtime, the OPU host
//! and the shared code): the shape-burst records (128 of 0x30 bytes at
//! `G_REC`: a position `+0x0`, a velocity `+0x10`, a scale `+0x20`, the
//! shape `+0x26`, a frame count `+0x28`, live while non-zero), the
//! effect's start and end, the per-frame code and its facing matrix.

use crate::pushes::Pushes;
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

/// The per-frame code's view of its frame and operands: `e` is the entry
/// `esp` (at the return address), `ebp = e - 4`; after `push esi` the
/// body runs at `e - 0x40` with nothing pushed.
struct Burst<'a> {
    img: &'a Image,
    f: u32,
    e: u32,
}

impl Burst<'_> {
    /// The operand at `+off`.
    fn w(&self, off: u32) -> u32 {
        self.img.word(self.f + off)
    }

    /// The word at the operand at `+off`.
    fn r<G: Game>(&self, g: &G, off: u32) -> u32 {
        g.read::<u32>(self.w(off))
    }

    /// The `f32` at the operand at `+off`.
    fn rf<G: Game>(&self, g: &G, off: u32) -> f32 {
        f32::from_bits(self.r(g, off))
    }

    /// `ebp - k` (the locals the slots fill).
    fn local(&self, k: u32) -> u32 {
        self.e.wrapping_sub(4).wrapping_sub(k)
    }

    /// `esp` at a callee's return address with `n` words pushed.
    fn callee_esp(&self, n: u32) -> u32 {
        self.e.wrapping_sub(0x44 + 4 * n)
    }

    /// The target of the `rel32` at `+off`.
    fn rel(&self, off: u32) -> u32 {
        self.img.rel32_target(self.f + off)
    }

    /// Slot `slot` through the table global (operand `+t`) with the words
    /// pushed so far.
    fn slot<G: Game>(&self, g: &mut G, p: &Pushes, t: u32, slot: u32) -> u32 {
        let target = slot_target(g, self.w(t), slot);
        // call width: 12 (Pushes caps the words at 12)
        p.call(g, target, 0).eax
    }

    /// Slot 261 `(k)` (the cue test) on its own: pushed, called, popped.
    fn cue<G: Game>(&self, g: &mut G, t: u32, k: u32) -> bool {
        let mut p = Pushes::default();
        p.push(k);
        self.slot(g, &p, t, 261) != 0
    }

    /// Whether the effect is paused: `**G_PAUSE` (operand `+off`).
    fn paused<G: Game>(&self, g: &G, off: u32) -> bool {
        g.read::<u32>(self.r(g, off)) != 0
    }

    /// `*G_PPP = ENV` (operands `+ppp`, `+env`), then slot 713 `(obj)`
    /// pushed (and left on the stack).
    fn free<G: Game>(&self, g: &mut G, p: &mut Pushes, (ppp, env, t): (u32, u32, u32), obj: u32) {
        let at = self.r(g, ppp);
        g.write::<u32>(at, self.w(env));
        p.push(obj);
        self.slot(g, p, t, 713);
    }

    /// The part refresh ([`part_refresh_b`](crate::sa_misc::part_refresh_b),
    /// the `rel32` at `+rel`) of the part at `obj`.
    fn refresh<G: Game>(&self, g: &mut G, rel: u32, obj: u32) {
        crate::sa_misc::part_refresh_b(g, self.img, self.rel(rel), obj, 0);
    }

    /// [`part_motion`](crate::s6_misc::part_motion) (the `rel32` at
    /// `+rel`): `ecx` = `part`, `edx` = `index`, `xmm2`/`xmm3` = `x2`/`x3`
    /// (bits), the seven stack words `words` (top first) where the
    /// original has them, with nothing else pushed.
    fn motion<G: Game>(
        &self,
        g: &mut G,
        (rel, part, index): (u32, u32, u32),
        (x2, x3): (u32, u32),
        words: [u32; 7],
    ) {
        let esp = self.callee_esp(7);
        for (i, v) in (0u32..).zip(words) {
            g.write::<u32>(esp + 4 + 4 * i, v);
        }
        let fr = Frame {
            at: self.rel(rel),
            esp,
            eax: 0,
            ecx: part,
            edx: index,
            xmm: [0, 0, u128::from(x2), u128::from(x3)],
        };
        crate::s6_misc::part_motion(g, self.img, &fr);
    }

    /// A callee taking only registers, with `n` words pushed.
    fn frame(&self, rel: u32, n: u32, ecx: u32, edx: u32) -> Frame {
        Frame {
            at: self.rel(rel),
            esp: self.callee_esp(n),
            eax: 0,
            ecx,
            edx,
            xmm: [0; 4],
        }
    }

    /// The program `k` of slot 941 `(*G_D)` (operands `+t`, `+d`) started
    /// on the part `obj` ([`start_part`](crate::opu_parts::start_part),
    /// the `rel32` at `+rel`, `edx` 0x1000, `(prog, 1)`); `p` holds the
    /// words pushed before 941's.
    fn start_program<G: Game>(
        &self,
        g: &mut G,
        p: &mut Pushes,
        (t, d): (u32, u32),
        k: u32,
        (rel, obj): (u32, u32),
    ) {
        p.push(self.r(g, d));
        let table = self.slot(g, p, t, 941);
        let prog = g.read::<u32>(table.wrapping_add(4 * k));
        crate::opu_parts::start_part(g, self.img, self.rel(rel), self.w(obj), 0x1000, prog, 1);
    }

    /// The program `+0x38` of [`call_941`](crate::s4_small::call_941)
    /// `(*G_D)` (operand `+d`, the `rel32` at `+rel941`, its word pushed
    /// on 0 and 1) started on the part `obj`
    /// ([`start_part`](crate::opu_parts::start_part), `rel32` at `+rel`).
    fn start_program_38<G: Game>(&self, g: &mut G, d: u32, rel941: u32, (rel, obj): (u32, u32)) {
        let x = self.r(g, d);
        let esp = self.callee_esp(3);
        g.write::<u32>(esp + 4, x);
        let fr = Frame {
            at: self.rel(rel941),
            esp,
            eax: 0,
            ecx: 0,
            edx: 0,
            xmm: [0; 4],
        };
        let table = crate::s4_small::call_941(g, self.img, &fr).eax;
        let prog = g.read::<u32>(table.wrapping_add(0x38));
        crate::opu_parts::start_part(g, self.img, self.rel(rel), self.w(obj), 0x1000, prog, 1);
    }

    /// Copies the 16 bytes at the operand `+src` to the operand `+dst`
    /// (`movaps`/`movdqa`).
    fn copy16<G: Game>(&self, g: &mut G, src: u32, dst: u32) {
        let (s, d) = (self.w(src), self.w(dst));
        for i in 0..4 {
            let v = g.read::<u32>(s + 4 * i);
            g.write::<u32>(d + 4 * i, v);
        }
    }

    /// `+1` on the counter at the operand `+off`.
    fn inc<G: Game>(&self, g: &mut G, off: u32) {
        let at = self.w(off);
        let v = g.read::<u32>(at).wrapping_add(1);
        g.write::<u32>(at, v);
    }
}

/// The operands of one camera sweep ([`sweep`]).
struct Sweep {
    /// The time `*G_T` and the constant `K` of `x = t / K` (`div`) or
    /// `t * K`.
    t: u32,
    k: u32,
    div: bool,
    /// The second point's offset `K2` (`x - K2`).
    k2: u32,
    /// Slot 270's tables, as pushed for the first and the second point.
    tab: (u32, u32, u32, u32),
    /// The locals (`ebp - n`) of the second point and of the direction.
    second: u32,
    dir: u32,
    /// The callees' `rel32` fields: facing, slot 231, the locked slot 712.
    rel: (u32, u32, u32),
    /// `G_CAM`, `G_A`/`G_B` (pushed in this order), `G_LOCK` (read twice),
    /// the part (`edx`) and its halfword flag.
    cam: u32,
    ab: (u32, u32),
    lock: (u32, u32),
    part: u32,
    flag: u32,
}

/// The camera sweep of cues 3 and 1: two points on slot 270's path
/// (`(&first, x, TAB, 4)` and `(&second, x - K2, ..)`), slot 234 `(&dir,
/// &second, &first)` their direction, its facing matrix ([`facing`]), the
/// first point into `G_CAM + 0x30`, slot 231
/// ([`call_231`](crate::s4_small::call_231)), slot 230 `(*G_B, *G_A)`, and
/// the part through the locked slot 712
/// ([`locked_712`](crate::s4_small::locked_712)) with `**G_LOCK` 1 and
/// then 0; a non-zero answer clears the part's flag.
fn sweep<G: Game>(g: &mut G, b: &Burst, tbl: u32, s: &Sweep) {
    use crate::sb_sse::{cvt, div, mul, sub};
    let t = cvt(b.r(g, s.t));
    let k = b.rf(g, s.k);
    let x = if s.div { div(t, k) } else { mul(t, k) };
    let first = b.local(0x14);
    let (second, dir) = (b.local(s.second), b.local(s.dir));
    let mut p = Pushes::default();
    p.push_all(&[4, b.w(s.tab.0), b.w(s.tab.1), x.to_bits(), first]);
    b.slot(g, &p, tbl, 270);
    let y = sub(x, b.rf(g, s.k2));
    p.pop(5);
    p.push_all(&[4, b.w(s.tab.2), b.w(s.tab.3), y.to_bits(), second]);
    b.slot(g, &p, tbl, 270);
    p.push_all(&[first, second, dir]);
    b.slot(g, &p, tbl, 234);
    facing(g, b.img, b.rel(s.rel.0), dir, b.callee_esp(8));
    let cam = b.r(g, s.cam);
    for i in 0..3 {
        let v = g.read::<u32>(first + 4 * i);
        g.write::<u32>(cam + 0x30 + 4 * i, v);
    }
    // `add esp, 0x1c` leaves the second path call's 4 under the next calls
    let fr = b.frame(s.rel.1, 1, 0, 0);
    crate::s4_small::call_231(g, b.img, &fr);
    let mut p = Pushes::default();
    p.push(4);
    p.push(b.r(g, s.ab.0));
    p.push(b.r(g, s.ab.1));
    b.slot(g, &p, tbl, 230);
    let lock = b.r(g, s.lock.0);
    g.write::<u32>(lock, 1);
    let fr = b.frame(s.rel.2, 1, 0, b.w(s.part));
    if crate::s4_small::locked_712(g, b.img, &fr).eax != 0 {
        g.write::<u16>(b.w(s.flag), 0);
    }
    let lock = b.r(g, s.lock.1);
    g.write::<u32>(lock, 0);
}

/// The effect's per-frame code (0x10001920, an overlay table entry, no
/// arguments; a /GS frame). Nothing while `G_ON` (operand `+0x12`) is 0.
/// On the first frame (`G_FRAME`, `+0x1f`, 0) slot 711 `(PROG, LIST, 0)`
/// under `**G_LOCK` 1 and slot 710 `(BLOCK, PROG, *G_BUF, 0x80000)`. Then
/// `**G_PAUSE` = slot 253 answering 0, slots 723 `(*G_X)` and 724
/// `(*G_Y)`, `**G_CURSOR = *G_BUF + 0x80400`, and the first cue of slot
/// 261 that is set picks the stage (10, 11, 8 (nothing), 7, 6, 5, 9
/// (nothing), 4 after its frame 0x2f, 3, 2 with the go flag, 1, 0); each
/// stage starts its parts once (a flag word each: slot 713 frees, the
/// motions of [`part_motion`](crate::s6_misc::part_motion), programs of
/// slot 941), refreshes them and, unless paused, counts its frames. Then
/// cue 15 runs slot 214 `(*G_SOUND)`, and `G_FRAME` counts. Slot calls
/// see the words the earlier ones left on the stack. Returns 0.
pub fn burst_frame<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let b = Burst {
        img,
        f: fr.at,
        e: fr.esp,
    };
    if b.r(g, 0x12) == 0 {
        return Ret::eax(0);
    }
    let tbl = 0x37;
    if b.r(g, 0x1f) == 0 {
        let lockp = b.r(g, 0x28);
        let saved = g.read::<u32>(lockp);
        g.write::<u32>(lockp, 1);
        let mut p = Pushes::default();
        p.push_all(&[0, b.w(0x3c), b.w(0x47)]);
        b.slot(g, &p, tbl, 711);
        p.push(0x8_0000);
        p.push(b.r(g, 0x59));
        let lockp = b.r(g, 0x4e);
        g.write::<u32>(lockp, saved);
        p.push_all(&[b.w(0x65), b.w(0x70)]);
        b.slot(g, &p, tbl, 710);
    }
    let mut p = Pushes::default();
    let running = b.slot(g, &p, tbl, 253) == 0;
    let pause = b.r(g, 0x88);
    p.push(b.r(g, 0x8e));
    g.write::<u32>(pause, u32::from(running));
    b.slot(g, &p, tbl, 723);
    p.push(b.r(g, 0xad));
    b.slot(g, &p, tbl, 724);
    let cursor = b.r(g, 0xbb).wrapping_add(0x8_0400);
    g.write::<u32>(b.r(g, 0xc0), cursor);
    p.push(0xa);
    if b.slot(g, &p, tbl, 261) != 0 {
        stage_10(g, &b, tbl);
    } else if b.cue(g, tbl, 0xb) {
        stage_11(g, &b, tbl);
    } else if b.cue(g, tbl, 8) {
        // nothing this frame
    } else if b.cue(g, tbl, 7) {
        stage_7(g, &b, tbl);
    } else if b.cue(g, tbl, 6) {
        stage_6(g, &b, tbl);
    } else if b.cue(g, tbl, 5) {
        stage_5(g, &b, tbl);
    } else if b.cue(g, tbl, 9) {
        // nothing this frame
    } else if b.cue(g, tbl, 4) && b.r(g, 0x7a6).cast_signed() > 0x2f {
        stage_4(g, &b, tbl);
    } else if b.cue(g, tbl, 3) {
        stage_3(g, &b, tbl);
    } else if b.cue(g, tbl, 2) && b.r(g, 0xa9f) != 0 {
        stage_2(g, &b, tbl);
    } else if b.cue(g, tbl, 1) {
        stage_1(g, &b, tbl);
    } else if b.cue(g, tbl, 0) {
        stage_0(g, &b);
    }
    if b.cue(g, tbl, 0xf) {
        let mut p = Pushes::default();
        p.push(b.r(g, 0xf69));
        b.slot(g, &p, tbl, 214);
    }
    b.inc(g, 0xf7a);
    Ret::eax(0)
}

/// Cue 10: once, slot 267 `(0x400)`, a part freed and program 5 started
/// on another, three vectors copied and its flags set; from the stage's
/// frame 0x5a the part refreshed; unless paused the frame counted.
fn stage_10<G: Game>(g: &mut G, b: &Burst, tbl: u32) {
    if b.r(g, 0xe8) == 0 {
        let mut p = Pushes::default();
        p.push(0x400);
        b.slot(g, &p, tbl, 267);
        b.free(g, &mut p, (0x106, 0x111, tbl), b.w(0x10b));
        g.write::<u32>(b.r(g, 0x123), b.w(0x129));
        b.start_program(g, &mut p, (tbl, 0x134), 5, (0x152, 0x14d));
        b.copy16(g, 0x159, 0x160);
        b.copy16(g, 0x168, 0x175);
        b.copy16(g, 0x17c, 0x18c);
        g.write::<u16>(b.w(0x185), 1);
        g.write::<u32>(b.w(0x191), 1);
    }
    let mut n = b.r(g, 0x197);
    if n.cast_signed() >= 0x5a {
        b.refresh(g, 0x1a6, b.w(0x1a1));
        n = b.r(g, 0x1ac);
    }
    if !b.paused(g, 0x1b1) {
        g.write::<u32>(b.w(0x1c1), n.wrapping_add(1));
    }
}

/// Cue 11: once, two parts freed and program 12 started on one of them,
/// three vectors copied and its flags set; the part refreshed; unless
/// paused the frame counted.
fn stage_11<G: Game>(g: &mut G, b: &Burst, tbl: u32) {
    if b.r(g, 0x1e6) == 0 {
        let mut p = Pushes::default();
        b.free(g, &mut p, (0x1f2, 0x1fd, tbl), b.w(0x1f7));
        b.free(g, &mut p, (0x20f, 0x21a, tbl), b.w(0x214));
        g.write::<u32>(b.r(g, 0x22c), b.w(0x232));
        b.start_program(g, &mut p, (tbl, 0x23d), 0xc, (0x25b, 0x256));
        b.copy16(g, 0x262, 0x269);
        b.copy16(g, 0x271, 0x27e);
        b.copy16(g, 0x285, 0x295);
        g.write::<u16>(b.w(0x28e), 1);
        g.write::<u32>(b.w(0x29a), 1);
    }
    b.refresh(g, 0x2a4, b.w(0x29f));
    if !b.paused(g, 0x2a9) {
        b.inc(g, 0x2b8);
    }
}

/// Slot 713 for each of the eight 0x80-byte parts from the operand
/// `+from` up to the one at `+to` (a do-while), each popped after its call.
fn free_all<G: Game>(g: &mut G, b: &Burst, (from, to): (u32, u32), ops: (u32, u32, u32)) {
    let end = b.w(to);
    let mut part = b.w(from);
    loop {
        let mut p = Pushes::default();
        b.free(g, &mut p, ops, part);
        part = part.wrapping_add(0x80);
        if part.cast_signed() >= end.cast_signed() {
            break;
        }
    }
}

/// Cue 7: once, four parts freed and two part motions (programs 10, 11);
/// the two parts refreshed; unless paused 3 records spawned ([`spawn`]);
/// the records drawn ([`draw_records`]); unless paused the frame counted.
fn stage_7<G: Game>(g: &mut G, b: &Burst, tbl: u32) {
    if b.r(g, 0x2f7) == 0 {
        let mut p = Pushes::default();
        for (ppp, obj, env) in [
            (0x303, 0x308, 0x30e),
            (0x320, 0x325, 0x32b),
            (0x33d, 0x342, 0x348),
            (0x35a, 0x35f, 0x365),
        ] {
            b.free(g, &mut p, (ppp, env, tbl), b.w(obj));
        }
        // the four freed parts' words become the motion's last four
        let k = 0x3e23_d684;
        let words = [0xc291_7eba, 0, 0xc698_0000, 0, k, k, k];
        let x = (b.r(g, 0x382), b.r(g, 0x37a));
        b.motion(g, (0x3ca, b.w(0x3be), 0xa), x, words);
        let k = 0x3cf5_c28f;
        let words = [0xc228_0000, 0, 0xc680_0000, 0, k, k, k];
        let x = (b.r(g, 0x3da), b.r(g, 0x3d2));
        b.motion(g, (0x420, b.w(0x414), 0xb), x, words);
        g.write::<u32>(b.w(0x429), 1);
    }
    b.refresh(g, 0x437, b.w(0x432));
    b.refresh(g, 0x441, b.w(0x43c));
    if !b.paused(g, 0x446) {
        let (a, k) = (b.rf(g, 0x45b), b.rf(g, 0x453));
        spawn(g, b.img, b.rel(0x465), b.callee_esp(0), 3, a, k);
    }
    draw_records(g, b.img, b.rel(0x46a));
    if !b.paused(g, 0x46f) {
        b.inc(g, 0x47e);
    }
}

/// Cue 6: once, a part freed and three part motions (programs 7, 8, 9);
/// while its count `n` is below 0x22, the shade `trunc((C - (n / D)^2) *
/// E)` into two halfwords, slot 268 `(*G_M, 6, V)` and `V.y -= F`, and the
/// first part refreshed; the other two dropped
/// ([`part_drop`](crate::s4_small::part_drop)); unless paused 8 records
/// spawned; the records drawn; unless paused the count goes up.
fn stage_6<G: Game>(g: &mut G, b: &Burst, tbl: u32) {
    use crate::sb_sse::{cvt, div, mul, sub};
    if b.r(g, 0x4a3) == 0 {
        let mut p = Pushes::default();
        b.free(g, &mut p, (0x4af, 0x4ba, tbl), b.w(0x4b4));
        let k = 0x3d23_d70a;
        let words = [0xc28c_0000, 0, 0xc680_0000, 0, k, k, k];
        let x = (b.r(g, 0x4d7), b.r(g, 0x4cf));
        b.motion(g, (0x51f, b.w(0x513), 7), x, words);
        let k = 0x3dcc_cccd;
        let words = [0x4170_0000, 0, 0xc70e_0000, 0, k, k, k];
        let x = (b.r(g, 0x52f), b.r(g, 0x527));
        b.motion(g, (0x575, b.w(0x569), 8), x, words);
        let words = [0x41c6_420a, 0, 0xc4c0_0000, 0, k, k, k];
        let x = (b.r(g, 0x585), b.r(g, 0x57d));
        b.motion(g, (0x5cb, b.w(0x5bf), 9), x, words);
        g.write::<u32>(b.w(0x5d4), 1);
    }
    let n = b.r(g, 0x5dd);
    if n.cast_signed() < 0x22 {
        let t = div(cvt(n), b.rf(g, 0x5f9));
        let c = mul(sub(b.rf(g, 0x5f1), mul(t, t)), b.rf(g, 0x616));
        let mut p = Pushes::default();
        p.push_all(&[b.w(0x5fe), 6, b.r(g, 0x60a)]);
        // the low halfword of cvttss2si
        let v = crate::s4_summon::trunc(c) as u16;
        g.write::<u16>(b.w(0x620), v);
        g.write::<u16>(b.w(0x626), v);
        b.slot(g, &p, tbl, 268);
        let y = sub(b.rf(g, 0x63b), b.rf(g, 0x643));
        g.write::<u32>(b.w(0x653), y.to_bits());
        b.refresh(g, 0x658, b.w(0x64b));
    }
    for (rel, obj) in [(0x662, 0x65d), (0x66c, 0x667)] {
        let fr = b.frame(rel, 0, b.w(obj), 0);
        crate::s4_small::part_drop(g, b.img, &fr);
    }
    if !b.paused(g, 0x671) {
        let (a, k) = (b.rf(g, 0x686), b.rf(g, 0x67e));
        spawn(g, b.img, b.rel(0x690), b.callee_esp(0), 8, a, k);
    }
    draw_records(g, b.img, b.rel(0x695));
    if !b.paused(g, 0x69a) {
        b.inc(g, 0x6a9);
    }
}

/// Cue 5: once, a part freed and a part motion (program 2); the part
/// refreshed; unless paused the frame counted.
fn stage_5<G: Game>(g: &mut G, b: &Burst, tbl: u32) {
    if b.r(g, 0x6ce) == 0 {
        let mut p = Pushes::default();
        b.free(g, &mut p, (0x6d6, 0x6e1, tbl), b.w(0x6db));
        let half = 0x3f00_0000;
        let words = [0, 0, 0xc680_0000, 0, half, half, half];
        b.motion(g, (0x73c, b.w(0x730), 2), (0, 0), words);
        g.write::<u32>(b.w(0x745), 1);
    }
    b.refresh(g, 0x753, b.w(0x74e));
    if !b.paused(g, 0x758) {
        b.inc(g, 0x767);
    }
}

/// Cue 4 (after its frame 0x2f): once, the eight parts freed and a part
/// motion (program 6); from the stage's frame 0x19 the part refreshed;
/// unless paused the frame counted.
fn stage_4<G: Game>(g: &mut G, b: &Burst, tbl: u32) {
    if b.r(g, 0x7b3) == 0 {
        free_all(g, b, (0x7bf, 0x7e4), (0x7c4, 0x7cb, tbl));
        let k = 0x3fc0_0000;
        let words = [0xc377_203a, 0, 0, 0, k, k, k];
        let x = (b.r(g, 0x7f6), b.r(g, 0x7ee));
        b.motion(g, (0x83f, b.w(0x833), 6), x, words);
        g.write::<u32>(b.w(0x848), 1);
    }
    let mut n = b.r(g, 0x852);
    if n.cast_signed() >= 0x19 {
        b.refresh(g, 0x861, b.w(0x85c));
        n = b.r(g, 0x867);
    }
    if !b.paused(g, 0x86c) {
        g.write::<u32>(b.w(0x87c), n.wrapping_add(1));
    }
}

/// Cue 3: once, the eight parts freed, slot 269 `(PATH, TAB, 4)`, the
/// part's program (`+0x38` of slot 941's table) started, its rates 0.36
/// and its flag set; while the flag is set, the camera sweep ([`sweep`]);
/// unless paused the time goes up.
fn stage_3<G: Game>(g: &mut G, b: &Burst, tbl: u32) {
    let first = b.r(g, 0x8a1) == 0;
    if first {
        free_all(g, b, (0x8ad, 0x8d2), (0x8b2, 0x8b9, tbl));
        let mut p = Pushes::default();
        p.push_all(&[4, b.w(0x8e6), b.w(0x8eb)]);
        b.slot(g, &p, tbl, 269);
        g.write::<u32>(b.r(g, 0x8f2), b.w(0x8fb));
        b.start_program_38(g, 0x905, 0x90a, (0x920, 0x91b));
        for off in [0x92e, 0x938, 0x942] {
            g.write::<u32>(b.w(off), 0x3eb8_51ec);
        }
        g.write::<u16>(b.w(0x94c), 1);
        g.write::<u32>(b.w(0x951), 1);
    }
    if first || g.read::<u16>(b.w(0x95a)) != 0 {
        let s = Sweep {
            t: 0x969,
            k: 0x976,
            div: true,
            k2: 0x9a9,
            tab: (0x97b, 0x980, 0x9b6, 0x9bb),
            second: 0x24,
            dir: 0x34,
            rel: (0x9f0, 0xa1b, 0xa4c),
            cam: 0x9fa,
            ab: (0xa21, 0xa2c),
            lock: (0xa39, 0xa60),
            part: 0xa41,
            flag: 0xa5b,
        };
        sweep(g, b, tbl, &s);
    }
    if !b.paused(g, 0xa6b) {
        b.inc(g, 0xa7a);
    }
}

/// Cue 2 (with the go flag): once, slot 269 `(PATH, TAB, 4)`, a part
/// freed and the eight parts' flags cleared; from the count 0xdc, every
/// fifth frame the next of the eight parts (`*G_NEXT`, wrapping at 8) is
/// freed if live, placed on slot 270's path at `(n - 0xdc) / K` and given
/// its motion (program 0, rates 0.4); then every live part is refreshed;
/// unless paused the count goes up.
fn stage_2<G: Game>(g: &mut G, b: &Burst, tbl: u32) {
    use crate::sb_sse::{cvt, div};
    if b.r(g, 0xaac) == 0 {
        let mut p = Pushes::default();
        p.push_all(&[4, b.w(0xac1), b.w(0xac6)]);
        g.write::<u32>(b.w(0xacc), 1);
        b.slot(g, &p, tbl, 269);
        b.free(g, &mut p, (0xad7, 0xae2, tbl), b.w(0xadc));
        for off in [0xafa, 0xb00, 0xb06, 0xb0c, 0xb12, 0xb18, 0xb1e, 0xb24] {
            g.write::<u16>(b.w(off), 0);
        }
    }
    let n = b.r(g, 0xb2a).cast_signed().wrapping_sub(0xdc);
    if n >= 0 {
        if n % 5 == 0 {
            let x = div(cvt(n.cast_unsigned()), b.rf(g, 0xb6a));
            let next = b.r(g, 0xb62);
            let part = (next << 7).wrapping_add(b.w(0xb73));
            if g.read::<u16>(part.wrapping_add(0x1e)) != 0 {
                let mut p = Pushes::default();
                b.free(g, &mut p, (0xb83, 0xb8a, tbl), part);
            }
            let at = b.local(0x14);
            let mut p = Pushes::default();
            p.push_all(&[4, b.w(0xba6), b.w(0xbab), x.to_bits(), at]);
            b.slot(g, &p, tbl, 270);
            let k = 0x3ecc_cccd;
            let (px, py, pz) = (
                g.read::<u32>(at),
                g.read::<u32>(at + 4),
                g.read::<u32>(at + 8),
            );
            b.motion(g, (0xc11, part, 0), (px, py), [pz, 0, 0, 0, k, k, k]);
            let next = b.r(g, 0xc16).wrapping_add(1);
            let next = if next.cast_signed() >= 8 { 0 } else { next };
            g.write::<u32>(b.w(0xc27), next);
        }
        let end = b.w(0xc43);
        let mut part = b.w(0xc2c);
        loop {
            if g.read::<u16>(part + 0x1e) != 0 {
                b.refresh(g, 0xc3a, part);
            }
            part = part.wrapping_add(0x80);
            if part.cast_signed() >= end.cast_signed() {
                break;
            }
        }
    }
    let n = b.r(g, 0xc4b);
    if !b.paused(g, 0xc50) {
        g.write::<u32>(b.w(0xc60), n.wrapping_add(1));
    }
}

/// Cue 1: once, slot 271 `(0x400)`, a part freed, slot 269 `(PATH, TAB,
/// 4)`, the part's program (`+0x38`) started, its rates 0.54 and its flag
/// set; while the flag is set, the camera sweep ([`sweep`]); unless
/// paused the time goes up and cue 2 sets the go flag.
fn stage_1<G: Game>(g: &mut G, b: &Burst, tbl: u32) {
    let first = b.r(g, 0xc85) == 0;
    if first {
        let mut p = Pushes::default();
        p.push(0x400);
        b.slot(g, &p, tbl, 271);
        b.free(g, &mut p, (0xca3, 0xcae, tbl), b.w(0xca8));
        p.push_all(&[4, b.w(0xccd), b.w(0xcd2)]);
        g.write::<u32>(b.w(0xcd8), 1);
        b.slot(g, &p, tbl, 269);
        g.write::<u32>(b.r(g, 0xce3), b.w(0xcec));
        b.start_program_38(g, 0xcf6, 0xcfb, (0xd11, 0xd0c));
        for off in [0xd1f, 0xd29, 0xd33] {
            g.write::<u32>(b.w(off), 0x3f0a_3d71);
        }
        g.write::<u16>(b.w(0xd3d), 1);
    }
    if first || g.read::<u16>(b.w(0xd46)) != 0 {
        let s = Sweep {
            t: 0xd55,
            k: 0xd62,
            div: false,
            k2: 0xd95,
            tab: (0xd67, 0xd6c, 0xda2, 0xda7),
            second: 0x34,
            dir: 0x24,
            rel: (0xddc, 0xe07, 0xe38),
            cam: 0xde6,
            ab: (0xe0d, 0xe18),
            lock: (0xe25, 0xe4c),
            part: 0xe2d,
            flag: 0xe47,
        };
        sweep(g, b, tbl, &s);
    }
    if !b.paused(g, 0xe57) {
        b.inc(g, 0xe6b);
        if b.cue(g, tbl, 2) {
            g.write::<u32>(b.w(0xe86), 1);
        }
    }
}

/// Cue 0: once, a part motion (program 1); from the stage's frame 0xaa
/// the part refreshed; unless paused the frame counted.
fn stage_0<G: Game>(g: &mut G, b: &Burst) {
    if b.r(g, 0xeaf) == 0 {
        let k = 0x3f66_6666;
        let words = [0xc34f_8b28, 0, 0, 0, k, k, k];
        let x = (b.r(g, 0xec2), b.r(g, 0xeba));
        b.motion(g, (0xf0b, b.w(0xeff), 1), x, words);
        g.write::<u32>(b.w(0xf14), 1);
    }
    let mut n = b.r(g, 0xf1e);
    if n.cast_signed() >= 0xaa {
        b.refresh(g, 0xf30, b.w(0xf2b));
        n = b.r(g, 0xf36);
    }
    if !b.paused(g, 0xf3b) {
        g.write::<u32>(b.w(0xf47), n.wrapping_add(1));
    }
}

/// [`burst_frame`] (3982 bytes, 1 overlay).
pub const BURST_FRAME: Sig = Sig {
    len: 3982,
    hash: 0x17f8_0541_fdf3_0653,
    calls: &[
        (0x152, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x1a6, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x25b, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x2a4, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x3ca, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x420, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x437, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x441, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x465, Callee::Sig(&SPAWN)),
        (0x46a, Callee::Sig(&DRAW_RECORDS)),
        (0x51f, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x575, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x5cb, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x658, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x662, Callee::Sig(&crate::s4_small::PART_DROP)),
        (0x66c, Callee::Sig(&crate::s4_small::PART_DROP)),
        (0x690, Callee::Sig(&SPAWN)),
        (0x695, Callee::Sig(&DRAW_RECORDS)),
        (0x73c, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x753, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x83f, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x861, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0x90a, Callee::Sig(&crate::s4_small::CALL_941)),
        (0x920, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x9f0, Callee::Sig(&FACING)),
        (0xa1b, Callee::Sig(&crate::s4_small::CALL_231)),
        (0xa4c, Callee::Sig(&crate::s4_small::LOCKED_712)),
        (0xc11, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0xc3a, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        (0xcfb, Callee::Sig(&crate::s4_small::CALL_941)),
        (0xd11, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xddc, Callee::Sig(&FACING)),
        (0xe07, Callee::Sig(&crate::s4_small::CALL_231)),
        (0xe38, Callee::Sig(&crate::s4_small::LOCKED_712)),
        (0xf0b, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0xf30, Callee::Sig(&crate::sa_misc::PART_REFRESH_B)),
        // __security_check_cookie
        (0xf86, Callee::Any),
    ],
    imports: &[],
};
