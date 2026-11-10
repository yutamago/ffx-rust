//! Round 39's pieces of the 16 KB scene tasks of `magic_0515`
//! (0x10004ce0), `0611` (0x10004d10), `0667` (0x10004af0) and `0668`
//! (0x10004ad0). The four are one source: the same prologue, a state
//! dispatched through a byte table (states 0, 1, 3, 6, 8, 9, 0xd, 0x10,
//! 0x15, 0x17, 0x18, 0x64) and the same end; states 1, 6, 8, 9, 0xd and
//! 0x10 are the same code in all four up to their exit jumps, so each
//! state's port takes the shift `d` of its block against `magic_0515`'s
//! and reads every operand at `0515's offset + d` (the comments give
//! `magic_0515`'s offsets).
//!
//! Not bound yet: the states are ported and tested one at a time
//! ([`scene_0515`] answers the ported ones); the `Sig` goes in once a
//! DLL's every state is.

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret};

/// A scene task's code offset and frame.
pub struct Scene {
    f: u32,
    /// `esp` with `esi` and `edi` pushed (`S`: `[esp + x]` is `S + x`).
    s: u32,
}

impl Scene {
    fn w(&self, img: &Image, o: u32) -> u32 {
        img.word(self.f + o)
    }

    /// The global at operand `o`.
    fn gw<G: Game>(&self, g: &G, img: &Image, o: u32) -> u32 {
        g.read::<u32>(self.w(img, o))
    }

    fn set<G: Game>(&self, g: &mut G, img: &Image, o: u32, v: u32) {
        g.write::<u32>(self.w(img, o), v);
    }

    /// `inc DWORD PTR [G]` (operand `o`).
    fn inc<G: Game>(&self, g: &mut G, img: &Image, o: u32) -> u32 {
        let v = self.gw(g, img, o).wrapping_add(1);
        self.set(g, img, o, v);
        v
    }

    fn target(&self, img: &Image, o: u32) -> u32 {
        img.rel32_target(self.f + o)
    }

    /// Slot `n` through the imports pointer operand `op` names.
    fn slot<G: Game>(&self, g: &G, img: &Image, op: u32, n: u32) -> u32 {
        slot_target(g, self.w(img, op), n)
    }

    /// The game's busy word `*[G_B]` (operand `o`) non-zero.
    fn busy<G: Game>(&self, g: &G, img: &Image, o: u32) -> bool {
        g.read::<u32>(self.gw(g, img, o)) != 0
    }

    /// A callee's frame (the call at `c`) with `pushed` words on the stack.
    fn frame(&self, img: &Image, c: u32, pushed: u32) -> Frame {
        Frame {
            at: self.target(img, c),
            esp: self.s.wrapping_sub(4 + 4 * pushed),
            ..Frame::default()
        }
    }
}

/// How a state block left: on to the common end, or straight out (the
/// prologue's checks).
enum Exit {
    End,
    Out,
}

/// The scene task of `magic_0515` (0x10004ce0, 16333 bytes; cdecl, no
/// arguments, a /GS frame of 0x108 bytes aligned to 8; returns 0), as far
/// as it is ported: the prologue, states 1 and 0x64 and the end. `None`
/// for a state not ported yet.
pub fn scene_0515<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Option<Ret> {
    // push ebp; and esp, -8; sub esp, 0x108; push esi; push edi
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x108);
    let c = Scene {
        f: fr.at,
        s: a.wrapping_sub(8),
    };
    if let Exit::Out = prologue(g, img, &c) {
        return Some(Ret::eax(0));
    }
    let state = c.gw(g, img, 0x141);
    if state <= 0x64 {
        match state {
            1 => state_1(g, img, &c, 0),
            6 => state_6(g, img, &c, 0),
            8 => state_8(g, img, &c, 0),
            9 => state_9(g, img, &c, 0),
            0xd => state_d(g, img, &c, 0),
            0x64 => state_64(g, img, &c, 0),
            2 | 4 | 5 | 7 | 0xa..=0xc | 0xe | 0xf | 0x11..=0x14 | 0x16 | 0x19..=0x63 => {}
            _ => return None,
        }
    }
    end(g, img, &c, 0);
    Some(Ret::eax(0))
}

/// `+0x1a` to `+0x140`: slots 225 (must answer 0), 226 and 227 (must
/// answer 1 or more) once each with `H = *G_H`; the busy word = slot 253
/// answered 0; the first time slot 313 `(0x100)` must answer 0, then
/// slots 711 (the lock word `*[G_L]` 1 around it) and 710.
fn prologue<G: Game>(g: &mut G, img: &Image, c: &Scene) -> Exit {
    let h = |g: &G| c.gw(g, img, 0x2c);
    if c.gw(g, img, 0x1c) == 0 {
        let hv = h(g);
        // call width: 1
        if g.call(c.slot(g, img, 0x26, 225), 0, &[hv]).eax != 0 {
            return Exit::Out;
        }
        c.inc(g, img, 0x45);
    }
    if c.gw(g, img, 0x4b) == 0 {
        let hv = h(g);
        c.set(g, img, 0x67, 1);
        // call width: 2
        g.call(c.slot(g, img, 0x53, 226), 0, &[hv, 0]);
    }
    if c.gw(g, img, 0x76) == 0 {
        let hv = h(g);
        // call width: 1
        let r = g.call(c.slot(g, img, 0x7e, 227), 0, &[hv]).eax;
        if r.cast_signed() < 1 {
            return Exit::Out;
        }
        c.inc(g, img, 0x9e);
    }
    // call width: 0
    let r = g.call(c.slot(g, img, 0xa3, 253), 0, &[]).eax;
    let busy = c.gw(g, img, 0xb1);
    g.write::<u32>(busy, u32::from(r == 0));
    if c.gw(g, img, 0xbc) == 0 {
        // call width: 1
        if g.call(c.slot(g, img, 0xc6, 313), 0, &[0x100]).eax != 0 {
            return Exit::Out;
        }
        let lock = c.gw(g, img, 0xe3);
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        c.set(g, img, 0x108, 1);
        let s = c.slot(g, img, 0xf2, 711);
        // call width: 3
        g.call(s, 0, &[c.w(img, 0x102), c.w(img, 0xf7), 0]);
        let lock = c.gw(g, img, 0x113);
        g.write::<u32>(lock, saved);
        let s = c.slot(g, img, 0x11d, 710);
        let args = [c.w(img, 0x137), c.w(img, 0x132), c.w(img, 0x12d), 0x13_b800];
        // call width: 4
        g.call(s, 0, &args);
    }
    Exit::End
}

/// State 1 (`+0x48d`): while not busy the halfword `G_T1` steps; on to
/// state 3 once `G_B - 0x2d000 <= G_A` (signed).
fn state_1<G: Game>(g: &mut G, img: &Image, c: &Scene, d: u32) {
    if !c.busy(g, img, 0x48e + d) {
        let at = c.w(img, 0x49a + d);
        let t = g.read::<u16>(at).wrapping_add(1);
        g.write::<u16>(at, t);
    }
    let lim = c.gw(g, img, 0x49f + d).wrapping_add(0xfffd_3000);
    if c.gw(g, img, 0x4aa + d).cast_signed() >= lim.cast_signed() {
        crate::r25::abs_stores(g, img, c.f, 0x4b4 + d, 0x4be + d);
    }
}

/// State 0x64 (`+0x3daf`): slot 311 `(*G_X)`, slot 271 with 1, 4 and
/// 0x10, slot 332 `(L)`, the two record groups placed, slot 276 `(*G_M,
/// 0)`, the models' slot 276 and slot 214 `(*G_H)`.
fn state_64<G: Game>(g: &mut G, img: &Image, c: &Scene, d: u32) {
    let x = c.gw(g, img, 0x3db6 + d);
    // call width: 1
    g.call(c.slot(g, img, 0x3db0 + d, 311), 0, &[x]);
    for (op, v) in [(0x3dc3u32, 1u32), (0x3dd5, 4), (0x3de7, 0x10)] {
        // call width: 1
        g.call(c.slot(g, img, op + d, 271), 0, &[v]);
    }
    // call width: 1
    let eax = g
        .call(c.slot(g, img, 0x3df9 + d, 332), 0, &[c.w(img, 0x3e07 + d)])
        .eax;
    let place = |g: &mut G, call: u32, recs: u32, n: u32, eax: u32| {
        crate::sa_misc::place_records(g, img, c.target(img, call), recs, n, eax)
    };
    let eax = place(g, 0x3e1b + d, c.w(img, 0x3e16 + d), 3, eax);
    let n = c.gw(g, img, 0x3e21 + d);
    place(g, 0x3e2b + d, c.w(img, 0x3e26 + d), n, eax);
    let m = c.gw(g, img, 0x3e38 + d);
    // call width: 2
    let eax = g.call(c.slot(g, img, 0x3e30 + d, 276), 0, &[m, 0]).eax;
    let mf = Frame {
        eax,
        ..c.frame(img, 0x3e45 + d, 2)
    };
    crate::s4_misc::models_276(g, img, &mf);
    let h = c.gw(g, img, 0x3e50 + d);
    // call width: 1
    g.call(c.slot(g, img, 0x3e4a + d, 214), 0, &[h]);
}

/// The common end (`+0x3e5f`): while not busy, the halfwords of the 32
/// records (0xa8 bytes apart) step, three counters and the `G_N` records'
/// counters (0xcc bytes apart) step, and the camera frame `G_T` (u16);
/// with a camera record `R = *G_R` its count steps, and at its last frame
/// (`R + 0x24` = the old frame) the record count steps, `G_R = R + 0x28`
/// and the next record's tracks start. Then both part groups' axes, and
/// with a record the cameras.
fn end<G: Game>(g: &mut G, img: &Image, c: &Scene, d: u32) {
    if !c.busy(g, img, 0x3e60 + d) {
        let (mut p, stop) = (c.w(img, 0x3e6e + d), c.w(img, 0x3e7b + d));
        loop {
            let t = g.read::<u16>(p).wrapping_add(1);
            g.write::<u16>(p, t);
            p = p.wrapping_add(0xa8);
            if p.cast_signed() >= stop.cast_signed() {
                break;
            }
        }
        let n = c.gw(g, img, 0x3e83 + d);
        for op in [0x3e89u32, 0x3e8f, 0x3e95] {
            c.inc(g, img, op + d);
        }
        if n.cast_signed() > 0 {
            let mut p = c.w(img, 0x3e9e + d);
            for _ in 0..n {
                let v = g.read::<u32>(p).wrapping_add(1);
                g.write::<u32>(p, v);
                p = p.wrapping_add(0xcc);
            }
        }
        let at = c.w(img, 0x3eaf + d);
        let t = g.read::<i16>(at);
        g.write::<u16>(c.w(img, 0x3eba + d), t.cast_unsigned().wrapping_add(1));
        let r = c.gw(g, img, 0x3ebf + d);
        if r != 0 {
            c.inc(g, img, 0x3ec9 + d);
            if i32::from(t).cast_unsigned() == g.read::<u32>(r.wrapping_add(0x24)) {
                c.inc(g, img, 0x3ed4 + d);
                c.set(g, img, 0x3edc + d, r.wrapping_add(0x28));
                crate::r25::abs_stores(g, img, c.f, 0x3ee0 + d, 0x3eea + d);
                crate::s6_misc::camera_tracks(g, img, &c.frame(img, 0x3eeb + d, 0));
            }
        }
    }
    let axes = |g: &mut G, call: u32, ecx: u32, edx: u32| {
        let fr = Frame {
            ecx,
            edx,
            ..c.frame(img, call, 0)
        };
        crate::s6_misc::parts_axes(g, img, &fr);
    };
    axes(g, 0x3efa + d, c.w(img, 0x3ef5 + d), 3);
    let n = c.gw(g, img, 0x3f00 + d);
    axes(g, 0x3f0a + d, c.w(img, 0x3f05 + d), n);
    if c.gw(g, img, 0x3f10 + d) != 0 {
        crate::r36::cameras_0515(g, img, &c.frame(img, 0x3f18 + d, 0));
    }
}

/// The float constant at operand `o`.
fn kf<G: Game>(g: &G, img: &Image, c: &Scene, o: u32) -> f32 {
    g.read::<f32>(c.w(img, o))
}

/// `u16` at the global of operand `o`.
fn half<G: Game>(g: &G, img: &Image, c: &Scene, o: u32) -> u16 {
    g.read::<u16>(c.w(img, o))
}

fn set_half<G: Game>(g: &mut G, img: &Image, c: &Scene, o: u32, v: u16) {
    g.write::<u16>(c.w(img, o), v);
}

/// Slot 222 (operand `op`) with `(*G_S, a, b, 0x100)` (`G_S` at `s`).
fn slot_222<G: Game>(g: &mut G, img: &Image, c: &Scene, (op, s): (u32, u32), a: u32, b: u32) {
    let v = c.gw(g, img, s);
    // call width: 4
    g.call(c.slot(g, img, op, 222), 0, &[v, a, b, 0x100]);
}

/// [`sparks_on_model`](crate::s6_misc::sparks_on_model) (the call at
/// `call`, one word pushed) on the record `rec` with `edx` and the speed in
/// `xmm2`.
fn sparks<G: Game>(
    g: &mut G,
    img: &Image,
    c: &Scene,
    call: u32,
    (rec, edx): (u32, u32),
    speed: f32,
) {
    let mut fr = c.frame(img, call, 1);
    fr.ecx = rec;
    fr.edx = edx;
    fr.xmm[2] = u128::from(speed.to_bits());
    crate::s6_misc::sparks_on_model(g, img, &fr);
}

/// [`part_motion`](crate::s6_misc::part_motion) (the call at `call`) of
/// the part `part` with program `index`, `xmm2`/`xmm3` and the seven
/// stack words `(a, 0, 0, 0, k, k, k)`.
fn motion<G: Game>(
    g: &mut G,
    img: &Image,
    c: &Scene,
    call: u32,
    (part, index): (u32, u32),
    (x2, x3): (f32, f32),
    (a, k): (u32, u32),
) {
    let sp = c.s.wrapping_sub(0x1c);
    for (i, v) in (0u32..).zip([a, 0, 0, 0, k, k, k]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let mut fr = c.frame(img, call, 7);
    fr.ecx = part;
    fr.edx = index;
    fr.xmm[2] = u128::from(x2.to_bits());
    fr.xmm[3] = u128::from(x3.to_bits());
    crate::s6_misc::part_motion(g, img, &fr);
}

/// [`parts_ring`](crate::s6_misc::parts_ring) (the call at `call`) of `n`
/// parts at `base`, the radius in `xmm3`, the stack `(junk, angle, word)`
/// (the first word `ecx` as it was: never read; 0 here).
fn ring<G: Game>(
    g: &mut G,
    img: &Image,
    c: &Scene,
    call: u32,
    (base, n): (u32, u32),
    r: f32,
    (angle, word): (u32, u32),
) {
    let sp = c.s.wrapping_sub(0xc);
    for (i, v) in (0u32..).zip([0, angle, word]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let mut fr = c.frame(img, call, 3);
    fr.ecx = base;
    fr.edx = n;
    fr.xmm[3] = u128::from(r.to_bits());
    crate::s6_misc::parts_ring(g, img, &fr);
}

/// `x / k` as the compiler's multiply-high divides (`magic`, the extra
/// `add` of `x` when `plus`, the shift `sh`), rounded toward 0.
fn div_magic(x: i32, magic: u32, plus: bool, sh: u32) -> i32 {
    let hi = ((i64::from(x) * i64::from(magic.cast_signed())) >> 32) as i32;
    let q = if plus { hi.wrapping_add(x) } else { hi } >> sh;
    q.wrapping_add(((q.cast_unsigned()) >> 31).cast_signed())
}

/// State 6 (`+0xc27`): once (the halfword `G_O`), slot 222 `(.., 5, 9)`,
/// the matrix turned, the camera record started, the records registered
/// and cleared, the actors' slot 296, the three models' angles and places.
/// While not busy: slot 220 on `*G_A` at the count's 0; at 0x2d the
/// records again and the places; sparks on the three models, each
/// model's height lowered; on to state 8 at 0x69.
fn state_6<G: Game>(g: &mut G, img: &Image, c: &Scene, d: u32) {
    if half(g, img, c, 0xc2a + d) == 0 {
        slot_222(g, img, c, (0xc36 + d, 0xc4b + d), 5, 9);
        set_half(g, img, c, 0xc5d + d, 1);
        crate::r25::abs_stores(g, img, c.f, 0xc61 + d, 0xc6b + d);
        crate::sa_misc::turn_matrix(g, img, c.target(img, 0xc6c + d), 0);
        let mut fr = c.frame(img, 0xc76 + d, 0);
        fr.ecx = c.w(img, 0xc71 + d);
        crate::s6_misc::camera_tracks_from(g, img, &fr);
        crate::sa_misc::register_records(g, img, c.target(img, 0xc7b + d));
        crate::sa_misc::records_clear(g, img, c.target(img, 0xc80 + d));
        crate::s4_misc::actors_296(g, img, &c.frame(img, 0xc87 + d, 0));
        set_half(g, img, c, 0xc92 + d, 0x5400);
        set_half(g, img, c, 0xc9d + d, 0x6800);
        crate::r25::abs_stores(g, img, c.f, 0xca6 + d, 0xd1e + d);
        set_half(g, img, c, 0xd20 + d, 0xfc00);
    }
    if c.busy(g, img, 0xd25 + d) {
        return;
    }
    let mut n = half(g, img, c, 0xd34 + d);
    if n == 0 {
        let mut fr = c.frame(img, 0xd45 + d, 1);
        fr.ecx = c.gw(g, img, 0xd40 + d);
        crate::s4_misc::call_220(g, img, &fr);
        n = half(g, img, c, 0xd4b + d);
    }
    if n == 0x2d {
        crate::sa_misc::register_records(g, img, c.target(img, 0xd5d + d));
        crate::sa_misc::records_clear(g, img, c.target(img, 0xd62 + d));
        crate::r25::abs_stores(g, img, c.f, 0xd6b + d, 0xd93 + d);
        set_half(g, img, c, 0xd95 + d, 0x4000);
        crate::r25::abs_stores(g, img, c.f, 0xd99 + d, 0xde9 + d);
    }
    // the three models (0xcc bytes apart): the last two with the word at
    // `+0xe25`/`+0xe06` lowered by `K1`, the first with `+0xe33` by `K2`
    let speed = kf(g, img, c, 0xded + d);
    let mut m = c.w(img, 0xdf4 + d);
    for i in 0..3u32 {
        let (call, word, k) = match i {
            0 => (0xe38, 0xe33, 0xe44),
            1 => (0xe12, 0xe25, 0xe1e),
            _ => (0xe12, 0xe06, 0xe1e),
        };
        let word = c.w(img, word + d);
        sparks(g, img, c, call + d, (m.wrapping_sub(0x40), word), speed);
        let y = crate::sb_sse::sub(g.read::<f32>(m), kf(g, img, c, k + d));
        g.write::<f32>(m, y);
        m = m.wrapping_add(0xcc);
    }
    let n = half(g, img, c, 0xe65 + d).wrapping_add(1);
    set_half(g, img, c, 0xe6d + d, n);
    if n.cast_signed() >= 0x69 {
        crate::r25::abs_stores(g, img, c.f, 0xe7b + d, 0xe85 + d);
    }
}

/// State 8 (`+0xe8a`): once (the halfword `G_O`), the camera record
/// started, the records registered and cleared, the three models' ring.
/// While not busy: slot 220 on `*G_A` at the count's 0x55; the three
/// sound records stepped ([`sound_step_8`]); then by the count: 0x69 slot
/// 306 and a sound, 0x7d a part's motion, 0xa5 three parts' motions, 0xe1
/// slot 222; on to state 9 at 0x109.
fn state_8<G: Game>(g: &mut G, img: &Image, c: &Scene, d: u32) {
    if half(g, img, c, 0xe8d + d) == 0 {
        set_half(g, img, c, 0xea0 + d, 1);
        let mut fr = c.frame(img, 0xea5 + d, 0);
        fr.ecx = c.w(img, 0xe9a + d);
        crate::s6_misc::camera_tracks_from(g, img, &fr);
        crate::sa_misc::register_records(g, img, c.target(img, 0xeaa + d));
        crate::sa_misc::records_clear(g, img, c.target(img, 0xeaf + d));
        for op in [0xecdu32, 0xed2, 0xed7] {
            c.set(g, img, op + d, 0);
        }
        let r = kf(g, img, c, 0xeb7 + d);
        ring(
            g,
            img,
            c,
            0xedc + d,
            (c.w(img, 0xec8 + d), 3),
            r,
            (0, 0x8000),
        );
        crate::r25::abs_stores(g, img, c.f, 0xee3 + d, 0xeed + d);
    }
    if c.busy(g, img, 0xeee + d) {
        return;
    }
    if half(g, img, c, 0xefe + d) == 0x55 {
        let mut fr = c.frame(img, 0xf0d + d, 1);
        fr.ecx = c.gw(g, img, 0xf08 + d);
        crate::s4_misc::call_220(g, img, &fr);
    }
    let mut p = c.w(img, 0xf23 + d);
    for i in 0..3u32 {
        sound_step_8(g, img, c, d, p, i);
        let at = p.wrapping_add(0x12);
        let t = g.read::<u16>(at).wrapping_add(1);
        g.write::<u16>(at, t);
        p = p.wrapping_add(0xcc);
    }
    let n = half(g, img, c, 0x1181 + d);
    let n = match n {
        0x69 => {
            let x = c.gw(g, img, 0x118d + d);
            // call width: 3
            g.call(c.slot(g, img, 0x1192 + d, 306), 0, &[7, 1, x]);
            crate::sa_misc::sound_start(
                g,
                img,
                c.target(img, 0x11b0 + d),
                c.w(img, 0x11ab + d),
                0xc5,
            );
            half(g, img, c, 0x12e3 + d)
        }
        0x7d => {
            let x3 = crate::sb_sse::add(kf(g, img, c, 0x11c3 + d), kf(g, img, c, 0x11cb + d));
            let a = c.gw(g, img, 0x11d3 + d);
            let x2 = kf(g, img, c, 0x11db + d);
            let part = (c.w(img, 0x1218 + d), 0xd);
            motion(g, img, c, 0x1222 + d, part, (x2, x3), (a, 0x3d4c_cccd));
            half(g, img, c, 0x12e3 + d)
        }
        0xa5 => {
            let (mut m, mut part) = (c.w(img, 0x123d + d), c.w(img, 0x1242 + d));
            loop {
                let a = g.read::<u32>(m.wrapping_add(4));
                let x3 = g.read::<f32>(m);
                let x2 = g.read::<f32>(m.wrapping_sub(4));
                motion(
                    g,
                    img,
                    c,
                    0x129e + d,
                    (part, 0x12),
                    (x2, x3),
                    (a, 0x3dac_0831),
                );
                part = part.wrapping_add(0x80);
                m = m.wrapping_add(0xcc);
                if part.cast_signed() >= c.w(img, 0x12b0 + d).cast_signed() {
                    break;
                }
            }
            half(g, img, c, 0x12e3 + d)
        }
        0xe1 => {
            slot_222(g, img, c, (0x12c3 + d, 0x12d8 + d), 0xa, 0xe);
            half(g, img, c, 0x12e3 + d)
        }
        _ => n,
    };
    let n = n.wrapping_add(1);
    set_half(g, img, c, 0x12f0 + d, n);
    if n.cast_signed() >= 0x109 {
        crate::r25::abs_stores(g, img, c.f, 0x12fd + d, 0x1307 + d);
    }
}

/// One of state 8's sound records `R = p - 8` (`i` 0..3; the last two
/// alike, the first with other kinds): by the kind `p[0]`, the sound
/// moves on at a frame `u16 p[0x12]` or once `p[8] >= p[4]`; between, the
/// sparks run and the record's height `p[0x38]` follows two sines of the
/// frame.
fn sound_step_8<G: Game>(g: &mut G, img: &Image, c: &Scene, d: u32, p: u32, i: u32) {
    use crate::sb_sse::{add, mul, sub};
    let rec = p.wrapping_sub(8);
    let kind = g.read::<u32>(p);
    let frame = |g: &G| i32::from(g.read::<u16>(p.wrapping_add(0x12)));
    let done = g.read::<u32>(p.wrapping_add(8)).cast_signed()
        >= g.read::<u32>(p.wrapping_add(4)).cast_signed();
    let start = |g: &mut G, call: u32, x: u32| {
        crate::sa_misc::sound_start(g, img, c.target(img, call + d), rec, x);
    };
    // (the `edi` angle, the `edx` word, the sparks' call, the second
    // sine's divide)
    let (edi, word, call, first) = match (i, kind) {
        (0, 0xc4) => {
            if frame(g) == 0x32 {
                start(g, 0x1082, kind + 1);
            }
            (0, 0x10da, 0x10df, true)
        }
        (0, 0xc5) => {
            if done {
                start(g, 0x10ae, 0xc6);
                return;
            }
            (
                div_magic((frame(g) - 0x32) << 15, 0x2e8b_a2e9, false, 3),
                0x10da,
                0x10df,
                true,
            )
        }
        (1 | 2, 0xc2) => {
            if frame(g) == 0x14 {
                start(g, if i == 1 { 0x100a } else { 0xf55 }, kind + 1);
            }
            (0, if i == 1 { 0x1019 } else { 0xfa4 }, 0xfb0, false)
        }
        (1 | 2, 0xc3) => {
            if done {
                start(g, if i == 1 { 0x103b } else { 0xf7d }, 0xc4);
                return;
            }
            let e = div_magic((frame(g) - 0x14) << 15, 0x8888_8889, true, 4);
            (e, if i == 1 { 0x1062 } else { 0xfa4 }, 0xfb0, false)
        }
        (2, 0xc5) => {
            if done {
                start(g, 0xfea, 0xc6);
            }
            return;
        }
        _ => return,
    };
    let speed = kf(g, img, c, 0xf18 + d);
    sparks(g, img, c, call + d, (rec, c.w(img, word + d)), speed);
    let x = frame(g) << 14;
    let a = if first {
        let hi = ((i64::from(x) * i64::from(0xae4c_415du32.cast_signed())) >> 32) as i32;
        hi.wrapping_add(x) >> 6
    } else {
        ((i64::from(x) * i64::from(0x51eb_851fu32.cast_signed())) >> 32) as i32 >> 4
    };
    let a = a.wrapping_add((a.cast_unsigned() >> 31).cast_signed());
    let tab = c.gw(g, img, 0x10f8 + d);
    let sin =
        |g: &G, k: i32| g.read::<f32>(tab.wrapping_add(4 * ((k >> 4).cast_unsigned() & 0xfff)));
    let s1 = mul(
        sub(kf(g, img, c, 0x1100 + d), sin(g, a)),
        kf(g, img, c, 0x1135 + d),
    );
    let y = sub(kf(g, img, c, 0x1108 + d), s1);
    let s2 = mul(
        add(sin(g, edi.wrapping_add(0x4000)), kf(g, img, c, 0x1146 + d)),
        kf(g, img, c, 0x114e + d),
    );
    g.write::<f32>(p.wrapping_add(0x38), sub(y, s2));
}

/// Slot 713 on the part `P` (operand `p`) with `*G_H = L` first (`G_H`
/// at `h`, `L` the immediate at `l`).
fn stop_part<G: Game>(g: &mut G, img: &Image, c: &Scene, (h, l): (u32, u32), (op, p): (u32, u32)) {
    let holder = c.gw(g, img, h);
    g.write::<u32>(holder, c.w(img, l));
    // call width: 1
    g.call(c.slot(g, img, op, 713), 0, &[c.w(img, p)]);
}

/// [`spline_object`](crate::s7_misc::spline_object) (the call at `call`)
/// on the object `o` with the points `pts`, the eleven stack words.
fn spline<G: Game>(
    g: &mut G,
    img: &Image,
    c: &Scene,
    call: u32,
    (o, pts): (u32, u32),
    words: [u32; 11],
) {
    let sp = c.s.wrapping_sub(0x2c);
    for (i, v) in (0u32..).zip(words) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let mut fr = c.frame(img, call, 11);
    fr.ecx = o;
    fr.edx = pts;
    crate::s7_misc::spline_object(g, img, &fr);
}

/// State 9 (`+0x130c`): once (the halfword `G_O`), part P1 stopped, the
/// camera record started, part P2's motion and the first sound. While not
/// busy: slot 220 at the count's 0; the sound record's kinds (0xc5: a
/// motion ten frames before its limit, the next kind at it; 0xc6 at frame
/// 0x1c; 0xc7 at 0x17, once: [`curve_9`]); a running part P3: slot 237
/// before the curve, else stopped after 0x32 frames or slot 270 with the
/// frame's share; then by the count: 0x83 slot 220 and a motion, 0x97
/// three motions and their spline objects; the sound timer steps; on to
/// state 0xd at 0xbe.
#[allow(clippy::too_many_lines)] // the original's state, in order
fn state_9<G: Game>(g: &mut G, img: &Image, c: &Scene, d: u32) {
    use crate::sb_sse::div;
    if half(g, img, c, 0x130f + d) == 0 {
        set_half(g, img, c, 0x1321 + d, 1);
        stop_part(
            g,
            img,
            c,
            (0x1326 + d, 0x1331 + d),
            (0x1336 + d, 0x132b + d),
        );
        set_half(g, img, c, 0x134e + d, 0);
        let mut fr = c.frame(img, 0x1353 + d, 0);
        fr.ecx = c.w(img, 0x1348 + d);
        crate::s6_misc::camera_tracks_from(g, img, &fr);
        let x3 = kf(g, img, c, 0x135b + d);
        let part = (c.w(img, 0x139b + d), 5);
        motion(g, img, c, 0x13a7 + d, part, (0.0, x3), (0, 0x3e48_b3b3));
        crate::sa_misc::sound_start(
            g,
            img,
            c.target(img, 0x13b9 + d),
            c.w(img, 0x13b4 + d),
            0xc5,
        );
        crate::r25::abs_stores(g, img, c.f, 0x13bd + d, 0x13db + d);
    }
    if c.busy(g, img, 0x13dc + d) {
        return;
    }
    if half(g, img, c, 0x13ec + d) == 0 {
        let mut fr = c.frame(img, 0x13fb + d, 1);
        fr.ecx = c.gw(g, img, 0x13f6 + d);
        crate::s4_misc::call_220(g, img, &fr);
    }
    let rec = |o: u32| c.w(img, o + d);
    let frame = |g: &G| c.gw(g, img, 0x141d + d).cast_signed();
    match c.gw(g, img, 0x1403 + d) {
        0xc5 => {
            let lim = c.gw(g, img, 0x1414 + d);
            if frame(g) == lim.wrapping_sub(0xa).cast_signed() {
                let part = (rec(0x1462), 7);
                motion(g, img, c, 0x146e + d, part, (0.0, 0.0), (0, 0x3d4c_cccd));
            } else if frame(g) >= lim.cast_signed() {
                crate::sa_misc::sound_start(g, img, c.target(img, 0x148e + d), rec(0x1489), 0xc6);
            }
        }
        0xc6 if frame(g) >= 0x1c => {
            crate::sa_misc::sound_start(g, img, c.target(img, 0x14b4 + d), rec(0x14af), 0xc7);
        }
        0xc7 if frame(g) >= 0x17 && c.gw(g, img, 0x14d7 + d) == 0 => {
            curve_9(g, img, c, d);
        }
        _ => {}
    }
    if half(g, img, c, 0x15ea + d) != 0 {
        if c.gw(g, img, 0x15f7 + d) == 0 {
            let x = c.gw(g, img, 0x1614 + d);
            // call width: 4
            g.call(
                c.slot(g, img, 0x15ff + d, 237),
                0,
                &[x, 0x7b, 0, rec(0x1604)],
            );
        } else if c.gw(g, img, 0x1621 + d).cast_signed() > 0x32 {
            stop_part(
                g,
                img,
                c,
                (0x1629 + d, 0x1634 + d),
                (0x1639 + d, 0x162e + d),
            );
            set_half(g, img, c, 0x164c + d, 0);
        } else {
            let n = c.gw(g, img, 0x1656 + d);
            #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
            let share = div(n.cast_signed() as f32, kf(g, img, c, 0x1666 + d));
            let s = c.slot(g, img, 0x165e + d, 270);
            c.inc(g, img, 0x1672 + d);
            let args = [rec(0x1689), share.to_bits(), rec(0x167e), rec(0x1679), 3];
            // call width: 5
            g.call(s, 0, &args);
        }
        crate::r25::abs_stores(g, img, c.f, 0x1692 + d, 0x169c + d);
    }
    if half(g, img, c, 0x16a4 + d) == 0x83 {
        let mut fr = c.frame(img, 0x16b2 + d, 1);
        fr.ecx = c.gw(g, img, 0x16ad + d);
        crate::s4_misc::call_220(g, img, &fr);
        let x3 = kf(g, img, c, 0x16ba + d);
        let part = (rec(0x16f9), 4);
        motion(g, img, c, 0x1705 + d, part, (0.0, x3), (0, 0x3f8c_cccd));
    } else if half(g, img, c, 0x1719 + d) == 0x97 {
        let (mut o, mut pts, mut part) = (rec(0x1727), rec(0x172c), rec(0x1731));
        let mut i = 0u32;
        loop {
            let a = g.read::<u32>(pts.wrapping_add(8));
            let x3 = g.read::<f32>(pts.wrapping_add(4));
            let x2 = g.read::<f32>(pts);
            motion(g, img, c, 0x178e + d, (part, 3), (x2, x3), (a, 0x3f99_999a));
            let b = g.read::<u32>(rec(0x17a9).wrapping_add(i));
            let a = g.read::<u32>(rec(0x17a1).wrapping_add(i));
            let words = [
                3,
                0x4c,
                0,
                0,
                rec(0x17c8),
                0x4000_0000,
                0xbe4c_cccd,
                0x3d23_d70a,
                b,
                a,
                4,
            ];
            spline(g, img, c, 0x17d5 + d, (o, pts), words);
            i += 4;
            o = o.wrapping_add(0xa8);
            part = part.wrapping_add(0x80);
            pts = pts.wrapping_add(0x30);
            if part.cast_signed() >= rec(0x17f1).cast_signed() {
                break;
            }
        }
    }
    if c.gw(g, img, 0x17fd + d).cast_signed() >= 0 {
        c.inc(g, img, 0x1806 + d);
    }
    let n = half(g, img, c, 0x180c + d).wrapping_add(1);
    set_half(g, img, c, 0x1819 + d, n);
    if n.cast_signed() >= 0xbe {
        crate::r25::abs_stores(g, img, c.f, 0x1826 + d, 0x1830 + d);
    }
}

/// State 9's curve (`+0x14e2`, kind 0xc7 at frame 0x17, once): slot 237
/// `(*G_M, 0x7b, 0, P)` gives the point `P`; the curve's points `Q`: `Q.x
/// = P.x K1 + K2 K3`, `Q.y = P.y K4 - K5`, `Q.z = P.z K1 + K2 K3` (in
/// doubles, rounded to f32), the constant words; slot 269 `(Q, P, 3)`;
/// the flag on, the timer 0.
fn curve_9<G: Game>(g: &mut G, img: &Image, c: &Scene, d: u32) {
    let rec = |o: u32| c.w(img, o + d);
    let k64 = |g: &G, o: u32| g.read::<f64>(c.w(img, o + d));
    let p = rec(0x14e8);
    let x = c.gw(g, img, 0x14f8 + d);
    // call width: 4
    g.call(c.slot(g, img, 0x14e3 + d, 237), 0, &[x, 0x7b, 0, p]);
    let px = f64::from(kf(g, img, c, 0x1502 + d));
    let lift = k64(g, 0x150a) * k64(g, 0x1512);
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let q0 = (px * k64(g, 0x1522) + lift) as f32;
    let s = c.slot(g, img, 0x151a + d, 269);
    crate::r25::abs_stores(g, img, c.f, 0x1528 + d, 0x153c + d);
    crate::r25::abs_stores(g, img, c.f, 0x1540 + d, 0x1568 + d);
    g.write::<f32>(rec(0x1575), q0);
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let q1 = (f64::from(kf(g, img, c, 0x157d + d)) * k64(g, 0x158d) - k64(g, 0x1595)) as f32;
    g.write::<f32>(rec(0x15a1), q1);
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let q2 = (f64::from(kf(g, img, c, 0x15a9 + d)) * k64(g, 0x15b4) + lift) as f32;
    g.write::<f32>(rec(0x15c4), q2);
    // call width: 3
    g.call(s, 0, &[rec(0x1585), rec(0x156d), 3]);
    crate::r25::abs_stores(g, img, c.f, 0x15d0 + d, 0x15e4 + d);
}

/// State 0xd (`+0x1835`): once (the halfword `G_O`), the matrix turned,
/// slot 222 `(.., 0xf, 0xf)`, the three sounds, the records registered
/// and cleared, the models' ring, part P1 stopped, the camera record
/// started, each model's place copied to its points and its spline object
/// made, part P4's motion. While not busy the count steps; on to state
/// 0x10 at 0x3c.
fn state_d<G: Game>(g: &mut G, img: &Image, c: &Scene, d: u32) {
    let rec = |o: u32| c.w(img, o + d);
    if half(g, img, c, 0x1838 + d) == 0 {
        crate::r25::abs_stores(g, img, c.f, 0x1845 + d, 0x184f + d);
        crate::sa_misc::turn_matrix(g, img, c.target(img, 0x1850 + d), 0);
        slot_222(g, img, c, (0x1855 + d, 0x186a + d), 0xf, 0xf);
        for (call, r, x) in [
            (0x187eu32, 0x1879u32, 0xc6u32),
            (0x188d, 0x1888, 0xc4),
            (0x189c, 0x1897, 0xc4),
        ] {
            crate::sa_misc::sound_start(g, img, c.target(img, call + d), rec(r), x);
        }
        set_half(g, img, c, 0x18a7 + d, 1);
        crate::sa_misc::register_counted(g, img, c.target(img, 0x18ac + d), 1);
        crate::s4_misc::records_clear(g, img, &c.frame(img, 0x18b1 + d, 0));
        let r = kf(g, img, c, 0x18b9 + d);
        ring(g, img, c, 0x18d0 + d, (rec(0x18cb), 3), r, (0, 0x8000));
        stop_part(
            g,
            img,
            c,
            (0x18d5 + d, 0x18de + d),
            (0x18e3 + d, 0x18e8 + d),
        );
        set_half(g, img, c, 0x1900 + d, 0);
        let mut fr = c.frame(img, 0x1905 + d, 0);
        fr.ecx = rec(0x18fa);
        crate::s6_misc::camera_tracks_from(g, img, &fr);
        let (mut o, mut pts, mut m, mut p) = (rec(0x1914), rec(0x1919), rec(0x1922), rec(0x190a));
        let mut i = 0u32;
        loop {
            for j in 0..4 {
                let v = g.read::<u32>(m + 4 * j);
                g.write::<u32>(pts + 0x30 + 4 * j, v);
                g.write::<u32>(p + 4 * j, v);
            }
            let b = g.read::<u32>(rec(0x1943).wrapping_add(i));
            let a = g.read::<u32>(rec(0x1938).wrapping_add(i));
            let words = [
                4,
                0x40,
                0,
                0,
                rec(0x1974),
                0x4000_0000,
                0xbe4c_cccd,
                0x3d23_d70a,
                b,
                a,
                2,
            ];
            spline(g, img, c, 0x1991 + d, (o, pts), words);
            p = p.wrapping_add(0x80);
            i += 4;
            o = o.wrapping_add(0xa8);
            m = m.wrapping_add(0xcc);
            pts = pts.wrapping_add(0x50);
            if m.cast_signed() >= rec(0x19c1).cast_signed() {
                break;
            }
        }
        motion(
            g,
            img,
            c,
            0x1a16 + d,
            (rec(0x1a0a), 0x15),
            (0.0, 0.0),
            (0, 0x3f00_0000),
        );
    }
    if c.busy(g, img, 0x1a1e + d) {
        return;
    }
    let n = half(g, img, c, 0x1a2d + d).wrapping_add(1);
    set_half(g, img, c, 0x1a35 + d, n);
    if n.cast_signed() >= 0x3c {
        crate::r25::abs_stores(g, img, c.f, 0x1a43 + d, 0x1a4d + d);
    }
}
