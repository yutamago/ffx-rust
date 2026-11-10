//! Round 39's pieces of the 16 KB scene tasks of `magic_0515`
//! (0x10004ce0), `0611` (0x10004d10), `0667` (0x10004af0) and `0668`
//! (0x10004ad0). The four are one source: the same prologue, a state
//! dispatched through a byte table (states 0, 1, 3, 6, 8, 9, 0xd, 0x10,
//! 0x15, 0x17, 0x18, 0x64) and the same end; states 1, 6, 8, 9, 0xd and
//! 0x10 are the same code in all four up to their exit jumps, so each
//! state's port takes the shift `d` of its block against `magic_0515`'s
//! and reads every operand at `0515's offset` (the comments give
//! `magic_0515`'s offsets).
//!
//! `magic_0515`'s is bound ([`scene_task_0515`]); the other three need
//! their own states 0, 3, 0x15, 0x17 (and 0x18's tail in `0667`/`0668`,
//! 0x9's middle in `0668`).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A scene task's code offset and frame.
pub struct Scene {
    f: u32,
    /// `esp` with `esi` and `edi` pushed (`S`: `[esp + x]` is `S + x`).
    s: u32,
    /// `magic_0515`'s offsets to this copy's: from each `(start, here)`
    /// on (sorted by `start`), `o - start + here`; empty for `magic_0515`.
    map: &'static [(u32, u32)],
    /// This copy calls slot 276 through `call_276` (`magic_0611`) where
    /// `magic_0515` calls the slot itself.
    alt: bool,
}

impl Scene {
    /// `magic_0515`'s offset `o` in this copy.
    fn m(&self, o: u32) -> u32 {
        let mut r = o;
        for &(start, here) in self.map {
            if o >= start {
                r = o - start + here;
            }
        }
        r
    }

    /// This scene with another map.
    fn with(&self, map: &'static [(u32, u32)]) -> Scene {
        Scene {
            f: self.f,
            s: self.s,
            map,
            alt: self.alt,
        }
    }

    fn w(&self, img: &Image, o: u32) -> u32 {
        img.word(self.f + self.m(o))
    }

    /// The run of constant stores from `from` to `to` (one block).
    fn stores<G: Game>(&self, g: &mut G, img: &Image, from: u32, to: u32) {
        let a = self.m(from);
        crate::r25::abs_stores(g, img, self.f, a, a + (to - from));
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
        img.rel32_target(self.f + self.m(o))
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

/// The scene task of `magic_0515` (0x10004ce0, 16333 bytes with its
/// tables; cdecl, no arguments, a /GS frame of 0x108 bytes aligned to 8;
/// returns 0): the prologue's checks, then by the state `G_S` (a byte
/// table up to 0x64): 0 [`state_0_0515`], 1 [`state_1`], 3
/// [`state_3_0515`], 6 [`state_6`], 8 [`state_8`], 9 [`state_9`], 0xd
/// [`state_d`], 0x10 [`state_10`], 0x15 [`state_15_0515`], 0x17
/// [`state_17_0515`], 0x18 [`state_18`], 0x64 [`state_64`]; the others
/// and states past 0x64 only the [`end`]. The frame cookie is the CRT's
/// own (`__security_check_cookie` at the end, not ported).
pub fn scene_task_0515<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    // push ebp; and esp, -8; sub esp, 0x108; push esi; push edi
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x108);
    let c = Scene {
        f: fr.at,
        s: a.wrapping_sub(8),
        map: &[],
        alt: false,
    };
    if let Exit::Out = prologue(g, img, &c) {
        return Ret::eax(0);
    }
    match c.gw(g, img, 0x141) {
        0 => state_0_0515(g, img, &c),
        1 => state_1(g, img, &c),
        3 => state_3_0515(g, img, &c),
        6 => state_6(g, img, &c),
        8 => state_8(g, img, &c),
        9 => state_9(g, img, &c),
        0xd => state_d(g, img, &c),
        0x10 => state_10(g, img, &c),
        0x15 => state_15_0515(g, img, &c),
        0x17 => state_17_0515(g, img, &c),
        0x18 => state_18(g, img, &c),
        0x64 => state_64(g, img, &c),
        _ => {}
    }
    end(g, img, &c);
    Ret::eax(0)
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
fn state_1<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    if !c.busy(g, img, 0x48e) {
        let at = c.w(img, 0x49a);
        let t = g.read::<u16>(at).wrapping_add(1);
        g.write::<u16>(at, t);
    }
    let lim = c.gw(g, img, 0x49f).wrapping_add(0xfffd_3000);
    if c.gw(g, img, 0x4aa).cast_signed() >= lim.cast_signed() {
        c.stores(g, img, 0x4b4, 0x4be);
    }
}

/// State 0x64 (`+0x3daf`): slot 311 `(*G_X)`, slot 271 with 1, 4 and
/// 0x10, slot 332 `(L)`, the two record groups placed, slot 276 `(*G_M,
/// 0)`, the models' slot 276 and slot 214 `(*G_H)`.
fn state_64<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    let x = c.gw(g, img, 0x3db6);
    // call width: 1
    g.call(c.slot(g, img, 0x3db0, 311), 0, &[x]);
    for (op, v) in [(0x3dc3u32, 1u32), (0x3dd5, 4), (0x3de7, 0x10)] {
        // call width: 1
        g.call(c.slot(g, img, op, 271), 0, &[v]);
    }
    // call width: 1
    let eax = g
        .call(c.slot(g, img, 0x3df9, 332), 0, &[c.w(img, 0x3e07)])
        .eax;
    let place = |g: &mut G, call: u32, recs: u32, n: u32, eax: u32| {
        crate::sa_misc::place_records(g, img, c.target(img, call), recs, n, eax)
    };
    let eax = place(g, 0x3e1b, c.w(img, 0x3e16), 3, eax);
    let n = c.gw(g, img, 0x3e21);
    place(g, 0x3e2b, c.w(img, 0x3e26), n, eax);
    let eax = if c.alt {
        // mov ecx, M; call call_276
        let mut fr = c.frame(img, 0x3e35, 0);
        fr.ecx = c.w(img, 0x3e30);
        crate::s4_small::call_276(g, img, &fr).eax
    } else {
        let m = c.gw(g, img, 0x3e38);
        // call width: 2
        g.call(c.slot(g, img, 0x3e30, 276), 0, &[m, 0]).eax
    };
    let mf = Frame {
        eax,
        ..c.frame(img, 0x3e45, if c.alt { 0 } else { 2 })
    };
    crate::s4_misc::models_276(g, img, &mf);
    let h = c.gw(g, img, 0x3e50);
    // call width: 1
    g.call(c.slot(g, img, 0x3e4a, 214), 0, &[h]);
}

/// The common end (`+0x3e5f`): while not busy, the halfwords of the 32
/// records (0xa8 bytes apart) step, three counters and the `G_N` records'
/// counters (0xcc bytes apart) step, and the camera frame `G_T` (u16);
/// with a camera record `R = *G_R` its count steps, and at its last frame
/// (`R + 0x24` = the old frame) the record count steps, `G_R = R + 0x28`
/// and the next record's tracks start. Then both part groups' axes, and
/// with a record the cameras.
fn end<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    if !c.busy(g, img, 0x3e60) {
        let (mut p, stop) = (c.w(img, 0x3e6e), c.w(img, 0x3e7b));
        loop {
            let t = g.read::<u16>(p).wrapping_add(1);
            g.write::<u16>(p, t);
            p = p.wrapping_add(0xa8);
            if p.cast_signed() >= stop.cast_signed() {
                break;
            }
        }
        let n = c.gw(g, img, 0x3e83);
        for op in [0x3e89u32, 0x3e8f, 0x3e95] {
            c.inc(g, img, op);
        }
        if n.cast_signed() > 0 {
            let mut p = c.w(img, 0x3e9e);
            for _ in 0..n {
                let v = g.read::<u32>(p).wrapping_add(1);
                g.write::<u32>(p, v);
                p = p.wrapping_add(0xcc);
            }
        }
        let at = c.w(img, 0x3eaf);
        let t = g.read::<i16>(at);
        g.write::<u16>(c.w(img, 0x3eba), t.cast_unsigned().wrapping_add(1));
        let r = c.gw(g, img, 0x3ebf);
        if r != 0 {
            c.inc(g, img, 0x3ec9);
            if i32::from(t).cast_unsigned() == g.read::<u32>(r.wrapping_add(0x24)) {
                c.inc(g, img, 0x3ed4);
                c.set(g, img, 0x3edc, r.wrapping_add(0x28));
                c.stores(g, img, 0x3ee0, 0x3eea);
                crate::s6_misc::camera_tracks(g, img, &c.frame(img, 0x3eeb, 0));
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
    axes(g, 0x3efa, c.w(img, 0x3ef5), 3);
    let n = c.gw(g, img, 0x3f00);
    axes(g, 0x3f0a, c.w(img, 0x3f05), n);
    if c.gw(g, img, 0x3f10) != 0 {
        crate::r36::cameras_0515(g, img, &c.frame(img, 0x3f18, 0));
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
fn state_6<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    if half(g, img, c, 0xc2a) == 0 {
        slot_222(g, img, c, (0xc36, 0xc4b), 5, 9);
        set_half(g, img, c, 0xc5d, 1);
        c.stores(g, img, 0xc61, 0xc6b);
        crate::sa_misc::turn_matrix(g, img, c.target(img, 0xc6c), 0);
        let mut fr = c.frame(img, 0xc76, 0);
        fr.ecx = c.w(img, 0xc71);
        crate::s6_misc::camera_tracks_from(g, img, &fr);
        crate::sa_misc::register_records(g, img, c.target(img, 0xc7b));
        crate::sa_misc::records_clear(g, img, c.target(img, 0xc80));
        crate::s4_misc::actors_296(g, img, &c.frame(img, 0xc87, 0));
        set_half(g, img, c, 0xc92, 0x5400);
        set_half(g, img, c, 0xc9d, 0x6800);
        c.stores(g, img, 0xca6, 0xd1e);
        set_half(g, img, c, 0xd20, 0xfc00);
    }
    if c.busy(g, img, 0xd25) {
        return;
    }
    let mut n = half(g, img, c, 0xd34);
    if n == 0 {
        let mut fr = c.frame(img, 0xd45, 1);
        fr.ecx = c.gw(g, img, 0xd40);
        crate::s4_misc::call_220(g, img, &fr);
        n = half(g, img, c, 0xd4b);
    }
    if n == 0x2d {
        crate::sa_misc::register_records(g, img, c.target(img, 0xd5d));
        crate::sa_misc::records_clear(g, img, c.target(img, 0xd62));
        c.stores(g, img, 0xd6b, 0xd93);
        set_half(g, img, c, 0xd95, 0x4000);
        c.stores(g, img, 0xd99, 0xde9);
    }
    // the three models (0xcc bytes apart): the last two with the word at
    // `+0xe25`/`+0xe06` lowered by `K1`, the first with `+0xe33` by `K2`
    let speed = kf(g, img, c, 0xded);
    let mut m = c.w(img, 0xdf4);
    for i in 0..3u32 {
        let (call, word, k) = match i {
            0 => (0xe38, 0xe33, 0xe44),
            1 => (0xe12, 0xe25, 0xe1e),
            _ => (0xe12, 0xe06, 0xe1e),
        };
        let word = c.w(img, word);
        sparks(g, img, c, call, (m.wrapping_sub(0x40), word), speed);
        let y = crate::sb_sse::sub(g.read::<f32>(m), kf(g, img, c, k));
        g.write::<f32>(m, y);
        m = m.wrapping_add(0xcc);
    }
    let n = half(g, img, c, 0xe65).wrapping_add(1);
    set_half(g, img, c, 0xe6d, n);
    if n.cast_signed() >= 0x69 {
        c.stores(g, img, 0xe7b, 0xe85);
    }
}

/// State 8 (`+0xe8a`): once (the halfword `G_O`), the camera record
/// started, the records registered and cleared, the three models' ring.
/// While not busy: slot 220 on `*G_A` at the count's 0x55; the three
/// sound records stepped ([`sound_step_8`]); then by the count: 0x69 slot
/// 306 and a sound, 0x7d a part's motion, 0xa5 three parts' motions, 0xe1
/// slot 222; on to state 9 at 0x109.
fn state_8<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    if half(g, img, c, 0xe8d) == 0 {
        set_half(g, img, c, 0xea0, 1);
        let mut fr = c.frame(img, 0xea5, 0);
        fr.ecx = c.w(img, 0xe9a);
        crate::s6_misc::camera_tracks_from(g, img, &fr);
        crate::sa_misc::register_records(g, img, c.target(img, 0xeaa));
        crate::sa_misc::records_clear(g, img, c.target(img, 0xeaf));
        for op in [0xecdu32, 0xed2, 0xed7] {
            c.set(g, img, op, 0);
        }
        let r = kf(g, img, c, 0xeb7);
        ring(g, img, c, 0xedc, (c.w(img, 0xec8), 3), r, (0, 0x8000));
        c.stores(g, img, 0xee3, 0xeed);
    }
    if c.busy(g, img, 0xeee) {
        return;
    }
    if half(g, img, c, 0xefe) == 0x55 {
        let mut fr = c.frame(img, 0xf0d, 1);
        fr.ecx = c.gw(g, img, 0xf08);
        crate::s4_misc::call_220(g, img, &fr);
    }
    let mut p = c.w(img, 0xf23);
    for i in 0..3u32 {
        sound_step_8(g, img, c, p, i);
        let at = p.wrapping_add(0x12);
        let t = g.read::<u16>(at).wrapping_add(1);
        g.write::<u16>(at, t);
        p = p.wrapping_add(0xcc);
    }
    let n = half(g, img, c, 0x1181);
    let n = match n {
        0x69 => {
            let x = c.gw(g, img, 0x118d);
            // call width: 3
            g.call(c.slot(g, img, 0x1192, 306), 0, &[7, 1, x]);
            crate::sa_misc::sound_start(g, img, c.target(img, 0x11b0), c.w(img, 0x11ab), 0xc5);
            half(g, img, c, 0x12e3)
        }
        0x7d => {
            let x3 = crate::sb_sse::add(kf(g, img, c, 0x11c3), kf(g, img, c, 0x11cb));
            let a = c.gw(g, img, 0x11d3);
            let x2 = kf(g, img, c, 0x11db);
            let part = (c.w(img, 0x1218), 0xd);
            motion(g, img, c, 0x1222, part, (x2, x3), (a, 0x3d4c_cccd));
            half(g, img, c, 0x12e3)
        }
        0xa5 => {
            let (mut m, mut part) = (c.w(img, 0x123d), c.w(img, 0x1242));
            loop {
                let a = g.read::<u32>(m.wrapping_add(4));
                let x3 = g.read::<f32>(m);
                let x2 = g.read::<f32>(m.wrapping_sub(4));
                motion(g, img, c, 0x129e, (part, 0x12), (x2, x3), (a, 0x3dac_0831));
                part = part.wrapping_add(0x80);
                m = m.wrapping_add(0xcc);
                if part.cast_signed() >= c.w(img, 0x12b0).cast_signed() {
                    break;
                }
            }
            half(g, img, c, 0x12e3)
        }
        0xe1 => {
            slot_222(g, img, c, (0x12c3, 0x12d8), 0xa, 0xe);
            half(g, img, c, 0x12e3)
        }
        _ => n,
    };
    let n = n.wrapping_add(1);
    set_half(g, img, c, 0x12f0, n);
    if n.cast_signed() >= 0x109 {
        c.stores(g, img, 0x12fd, 0x1307);
    }
}

/// One of state 8's sound records `R = p - 8` (`i` 0..3; the last two
/// alike, the first with other kinds): by the kind `p[0]`, the sound
/// moves on at a frame `u16 p[0x12]` or once `p[8] >= p[4]`; between, the
/// sparks run and the record's height `p[0x38]` follows two sines of the
/// frame.
fn sound_step_8<G: Game>(g: &mut G, img: &Image, c: &Scene, p: u32, i: u32) {
    use crate::sb_sse::{add, mul, sub};
    let rec = p.wrapping_sub(8);
    let kind = g.read::<u32>(p);
    let frame = |g: &G| i32::from(g.read::<u16>(p.wrapping_add(0x12)));
    let done = g.read::<u32>(p.wrapping_add(8)).cast_signed()
        >= g.read::<u32>(p.wrapping_add(4)).cast_signed();
    let start = |g: &mut G, call: u32, x: u32| {
        crate::sa_misc::sound_start(g, img, c.target(img, call), rec, x);
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
    let speed = kf(g, img, c, 0xf18);
    sparks(g, img, c, call, (rec, c.w(img, word)), speed);
    let x = frame(g) << 14;
    let a = if first {
        let hi = ((i64::from(x) * i64::from(0xae4c_415du32.cast_signed())) >> 32) as i32;
        hi.wrapping_add(x) >> 6
    } else {
        ((i64::from(x) * i64::from(0x51eb_851fu32.cast_signed())) >> 32) as i32 >> 4
    };
    let a = a.wrapping_add((a.cast_unsigned() >> 31).cast_signed());
    let tab = c.gw(g, img, 0x10f8);
    let sin =
        |g: &G, k: i32| g.read::<f32>(tab.wrapping_add(4 * ((k >> 4).cast_unsigned() & 0xfff)));
    let s1 = mul(sub(kf(g, img, c, 0x1100), sin(g, a)), kf(g, img, c, 0x1135));
    let y = sub(kf(g, img, c, 0x1108), s1);
    let s2 = mul(
        add(sin(g, edi.wrapping_add(0x4000)), kf(g, img, c, 0x1146)),
        kf(g, img, c, 0x114e),
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
fn state_9<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    use crate::sb_sse::div;
    if half(g, img, c, 0x130f) == 0 {
        set_half(g, img, c, 0x1321, 1);
        stop_part(g, img, c, (0x1326, 0x1331), (0x1336, 0x132b));
        set_half(g, img, c, 0x134e, 0);
        let mut fr = c.frame(img, 0x1353, 0);
        fr.ecx = c.w(img, 0x1348);
        crate::s6_misc::camera_tracks_from(g, img, &fr);
        let x3 = kf(g, img, c, 0x135b);
        let part = (c.w(img, 0x139b), 5);
        motion(g, img, c, 0x13a7, part, (0.0, x3), (0, 0x3e48_b3b3));
        crate::sa_misc::sound_start(g, img, c.target(img, 0x13b9), c.w(img, 0x13b4), 0xc5);
        c.stores(g, img, 0x13bd, 0x13db);
    }
    if c.busy(g, img, 0x13dc) {
        return;
    }
    if half(g, img, c, 0x13ec) == 0 {
        let mut fr = c.frame(img, 0x13fb, 1);
        fr.ecx = c.gw(g, img, 0x13f6);
        crate::s4_misc::call_220(g, img, &fr);
    }
    let rec = |o: u32| c.w(img, o);
    let frame = |g: &G| c.gw(g, img, 0x141d).cast_signed();
    match c.gw(g, img, 0x1403) {
        0xc5 => {
            let lim = c.gw(g, img, 0x1414);
            if frame(g) == lim.wrapping_sub(0xa).cast_signed() {
                let part = (rec(0x1462), 7);
                motion(g, img, c, 0x146e, part, (0.0, 0.0), (0, 0x3d4c_cccd));
            } else if frame(g) >= lim.cast_signed() {
                crate::sa_misc::sound_start(g, img, c.target(img, 0x148e), rec(0x1489), 0xc6);
            }
        }
        0xc6 if frame(g) >= 0x1c => {
            crate::sa_misc::sound_start(g, img, c.target(img, 0x14b4), rec(0x14af), 0xc7);
        }
        0xc7 if frame(g) >= 0x17 && c.gw(g, img, 0x14d7) == 0 => {
            curve_9(g, img, c);
        }
        _ => {}
    }
    if half(g, img, c, 0x15ea) != 0 {
        if c.gw(g, img, 0x15f7) == 0 {
            let x = c.gw(g, img, 0x1614);
            // call width: 4
            g.call(c.slot(g, img, 0x15ff, 237), 0, &[x, 0x7b, 0, rec(0x1604)]);
        } else if c.gw(g, img, 0x1621).cast_signed() > 0x32 {
            stop_part(g, img, c, (0x1629, 0x1634), (0x1639, 0x162e));
            set_half(g, img, c, 0x164c, 0);
        } else {
            let n = c.gw(g, img, 0x1656);
            #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
            let share = div(n.cast_signed() as f32, kf(g, img, c, 0x1666));
            let s = c.slot(g, img, 0x165e, 270);
            c.inc(g, img, 0x1672);
            let args = [rec(0x1689), share.to_bits(), rec(0x167e), rec(0x1679), 3];
            // call width: 5
            g.call(s, 0, &args);
        }
        c.stores(g, img, 0x1692, 0x169c);
    }
    if half(g, img, c, 0x16a4) == 0x83 {
        let mut fr = c.frame(img, 0x16b2, 1);
        fr.ecx = c.gw(g, img, 0x16ad);
        crate::s4_misc::call_220(g, img, &fr);
        let x3 = kf(g, img, c, 0x16ba);
        let part = (rec(0x16f9), 4);
        motion(g, img, c, 0x1705, part, (0.0, x3), (0, 0x3f8c_cccd));
    } else if half(g, img, c, 0x1719) == 0x97 {
        let (mut o, mut pts, mut part) = (rec(0x1727), rec(0x172c), rec(0x1731));
        let mut i = 0u32;
        loop {
            let a = g.read::<u32>(pts.wrapping_add(8));
            let x3 = g.read::<f32>(pts.wrapping_add(4));
            let x2 = g.read::<f32>(pts);
            motion(g, img, c, 0x178e, (part, 3), (x2, x3), (a, 0x3f99_999a));
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
            spline(g, img, c, 0x17d5, (o, pts), words);
            i += 4;
            o = o.wrapping_add(0xa8);
            part = part.wrapping_add(0x80);
            pts = pts.wrapping_add(0x30);
            if part.cast_signed() >= rec(0x17f1).cast_signed() {
                break;
            }
        }
    }
    if c.gw(g, img, 0x17fd).cast_signed() >= 0 {
        c.inc(g, img, 0x1806);
    }
    let n = half(g, img, c, 0x180c).wrapping_add(1);
    set_half(g, img, c, 0x1819, n);
    if n.cast_signed() >= 0xbe {
        c.stores(g, img, 0x1826, 0x1830);
    }
}

/// State 9's curve (`+0x14e2`, kind 0xc7 at frame 0x17, once): slot 237
/// `(*G_M, 0x7b, 0, P)` gives the point `P`; the curve's points `Q`: `Q.x
/// = P.x K1 + K2 K3`, `Q.y = P.y K4 - K5`, `Q.z = P.z K1 + K2 K3` (in
/// doubles, rounded to f32), the constant words; slot 269 `(Q, P, 3)`;
/// the flag on, the timer 0.
fn curve_9<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    let rec = |o: u32| c.w(img, o);
    let k64 = |g: &G, o: u32| g.read::<f64>(c.w(img, o));
    let p = rec(0x14e8);
    let x = c.gw(g, img, 0x14f8);
    // call width: 4
    g.call(c.slot(g, img, 0x14e3, 237), 0, &[x, 0x7b, 0, p]);
    let px = f64::from(kf(g, img, c, 0x1502));
    let lift = k64(g, 0x150a) * k64(g, 0x1512);
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let q0 = (px * k64(g, 0x1522) + lift) as f32;
    let s = c.slot(g, img, 0x151a, 269);
    c.stores(g, img, 0x1528, 0x153c);
    c.stores(g, img, 0x1540, 0x1568);
    g.write::<f32>(rec(0x1575), q0);
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let q1 = (f64::from(kf(g, img, c, 0x157d)) * k64(g, 0x158d) - k64(g, 0x1595)) as f32;
    g.write::<f32>(rec(0x15a1), q1);
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let q2 = (f64::from(kf(g, img, c, 0x15a9)) * k64(g, 0x15b4) + lift) as f32;
    g.write::<f32>(rec(0x15c4), q2);
    // call width: 3
    g.call(s, 0, &[rec(0x1585), rec(0x156d), 3]);
    c.stores(g, img, 0x15d0, 0x15e4);
}

/// State 0xd (`+0x1835`): once (the halfword `G_O`), the matrix turned,
/// slot 222 `(.., 0xf, 0xf)`, the three sounds, the records registered
/// and cleared, the models' ring, part P1 stopped, the camera record
/// started, each model's place copied to its points and its spline object
/// made, part P4's motion. While not busy the count steps; on to state
/// 0x10 at 0x3c.
fn state_d<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    let rec = |o: u32| c.w(img, o);
    if half(g, img, c, 0x1838) == 0 {
        c.stores(g, img, 0x1845, 0x184f);
        crate::sa_misc::turn_matrix(g, img, c.target(img, 0x1850), 0);
        slot_222(g, img, c, (0x1855, 0x186a), 0xf, 0xf);
        for (call, r, x) in [
            (0x187eu32, 0x1879u32, 0xc6u32),
            (0x188d, 0x1888, 0xc4),
            (0x189c, 0x1897, 0xc4),
        ] {
            crate::sa_misc::sound_start(g, img, c.target(img, call), rec(r), x);
        }
        set_half(g, img, c, 0x18a7, 1);
        crate::sa_misc::register_counted(g, img, c.target(img, 0x18ac), 1);
        crate::s4_misc::records_clear(g, img, &c.frame(img, 0x18b1, 0));
        let r = kf(g, img, c, 0x18b9);
        ring(g, img, c, 0x18d0, (rec(0x18cb), 3), r, (0, 0x8000));
        stop_part(g, img, c, (0x18d5, 0x18de), (0x18e3, 0x18e8));
        set_half(g, img, c, 0x1900, 0);
        let mut fr = c.frame(img, 0x1905, 0);
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
            spline(g, img, c, 0x1991, (o, pts), words);
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
            0x1a16,
            (rec(0x1a0a), 0x15),
            (0.0, 0.0),
            (0, 0x3f00_0000),
        );
    }
    if c.busy(g, img, 0x1a1e) {
        return;
    }
    let n = half(g, img, c, 0x1a2d).wrapping_add(1);
    set_half(g, img, c, 0x1a35, n);
    if n.cast_signed() >= 0x3c {
        c.stores(g, img, 0x1a43, 0x1a4d);
    }
}

/// `cvttss2si`/`cvttsd2si`: truncated, `0x8000_0000` out of range.
fn trunc(x: f64) -> u32 {
    ffx_bridge::x87::cvttsd2si(x)
}

/// Slot 307's float (ST0 stored as f32) through operand `op`.
fn frand<G: Game>(g: &mut G, img: &Image, c: &Scene, op: u32) -> f32 {
    // call width: 0
    crate::m0018::st0_f32(g.call(c.slot(g, img, op, 307), 0, &[]).st0)
}

/// Slot `n` (operand `op`) with `args`.
fn call_slot<G: Game>(g: &mut G, img: &Image, c: &Scene, (op, n): (u32, u32), args: &[u32]) -> u32 {
    // call width: 5
    g.call(c.slot(g, img, op, n), 0, args).eax
}

/// A part started for state 0x10 (`P` at operand `p`): `*G_H = L`, slot
/// 941 `(*G_D)`'s word `k` to pppStartPart (`ecx` = `P`, 0x1000, `(word,
/// 1, 0)`); `P + 0x44 = y`, `P + 0x50` and `+0x60` the 16-byte constants
/// at operands `src`, u16 `P + 0x1e` 1, `P + 0x40` and `+0x48` 0, `P +
/// 0x4c` 1.0. The operands: `(G_H, L, imports, G_D, call, P)`.
fn part_10<G: Game>(
    g: &mut G,
    img: &Image,
    c: &Scene,
    ops: [u32; 6],
    k: u32,
    y: f32,
    src: [u32; 2],
) {
    let [h, l, imp, dd, call, p] = ops;
    let holder = c.gw(g, img, h);
    g.write::<u32>(holder, c.w(img, l));
    let dv = c.gw(g, img, dd);
    let table = call_slot(g, img, c, (imp, 941), &[dv]);
    let word = g.read::<u32>(table.wrapping_add(k));
    let part = c.w(img, p);
    crate::opu_parts::start_part(g, img, c.target(img, call), part, 0x1000, word, 1);
    g.write::<f32>(part + 0x44, y);
    for (i, op) in (0u32..).zip(src) {
        let from = c.w(img, op);
        for j in 0..4 {
            let v = g.read::<u32>(from + 4 * j);
            g.write::<u32>(part + 0x50 + 0x10 * i + 4 * j, v);
        }
    }
    g.write::<u16>(part + 0x1e, 1);
    g.write::<u32>(part + 0x40, 0);
    g.write::<u32>(part + 0x48, 0);
    g.write::<u32>(part + 0x4c, 0x3f80_0000);
}

/// State 0x10 (`+0x1a52`, the same 4887 bytes in all four tasks): once,
/// [`setup_10`]; while not busy: slot 220 at the count's 0, the three
/// beams launched at 0x14 ([`launch_10`]), the bursts from 0x14 to 0x77
/// ([`burst_10`]), then the rings, colours and parts by the count
/// ([`late_10`]); on to state 0x15 at 0x130.
fn state_10<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    if half(g, img, c, 0x1a55) == 0 {
        setup_10(g, img, c);
    }
    if c.busy(g, img, 0x1cb0) {
        return;
    }
    let mut n = half(g, img, c, 0x1cc0);
    if n == 0 {
        let x = c.gw(g, img, 0x1cdd);
        call_slot(g, img, c, (0x1cca, 220), &[x, 0x40, 0x100]);
        n = half(g, img, c, 0x1ce6);
    }
    if n == 0x14 {
        launch_10(g, img, c);
        n = half(g, img, c, 0x209a);
        c.stores(g, img, 0x209e, 0x20a8);
    }
    let since = i32::from(n.cast_signed()) - 0x14;
    if since >= 0 {
        if since < 0x64 {
            burst_10(g, img, c, since);
            n = half(g, img, c, 0x2694);
        }
        n = late_10(g, img, c, (n, since));
    }
    let n = n.wrapping_add(1);
    set_half(g, img, c, 0x2d4d, n);
    if n.cast_signed() >= 0x130 {
        c.stores(g, img, 0x2d5a, 0x2d64);
    }
}

/// State 0x10's set-up: the counted records registered and the records
/// cleared, the matrix turned, the models' ring, parts P4 and P1 stopped,
/// the camera record started; for each model `M` (0xcc bytes apart) its
/// direction `V = (M.x, 0, M.z, 0)` (slot 232), its beam's three points
/// `Q` (`M.x K1`, `M.y`, `M.z K1`; `M.x K2`, `M.y`, `M.z K2`; `V.x K3 +
/// M.x`, `M.y`, `V.z K3 + M.z`), its place copied and its spline object;
/// the three parts P5 stopped.
fn setup_10<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    use crate::sb_sse::{add, mul};
    let rec = |o: u32| c.w(img, o);
    set_half(g, img, c, 0x1a67, 1);
    crate::sa_misc::register_counted(g, img, c.target(img, 0x1a6c), 1);
    crate::s4_misc::records_clear(g, img, &c.frame(img, 0x1a71, 0));
    c.stores(g, img, 0x1a77, 0x1a81);
    crate::sa_misc::turn_matrix(g, img, c.target(img, 0x1a82), 0);
    let r = kf(g, img, c, 0x1a8a);
    ring(g, img, c, 0x1aa1, (rec(0x1a9c), 3), r, (0, 0x8000));
    stop_part(g, img, c, (0x1aa6, 0x1aaf), (0x1ab4, 0x1ab9));
    set_half(g, img, c, 0x1ac9, 0);
    stop_part(g, img, c, (0x1ace, 0x1ad7), (0x1adc, 0x1ae1));
    set_half(g, img, c, 0x1af9, 0);
    let mut fr = c.frame(img, 0x1afe, 0);
    fr.ecx = rec(0x1af3);
    crate::s6_misc::camera_tracks_from(g, img, &fr);
    let (mut m, mut o, mut p) = (rec(0x1b03), rec(0x1b0b), rec(0x1b13));
    let (mut v, mut q) = (rec(0x1b18), rec(0x1b1d));
    let mut i = 0u32;
    loop {
        let x = g.read::<u32>(m.wrapping_sub(8));
        g.write::<u32>(v, x);
        let z = g.read::<u32>(m);
        g.write::<u32>(v + 8, z);
        g.write::<u32>(v + 0xc, 0);
        g.write::<u32>(v + 4, 0);
        call_slot(g, img, c, (0x1b3b, 232), &[v, v]);
        let mx = g.read::<f32>(m.wrapping_sub(8));
        let mz = g.read::<f32>(m);
        let my = g.read::<u32>(m.wrapping_sub(4));
        let (k1, k2, k3) = (
            kf(g, img, c, 0x1b6e),
            kf(g, img, c, 0x1b9d),
            kf(g, img, c, 0x1bca),
        );
        g.write::<f32>(q.wrapping_sub(8), mul(mx, k1));
        g.write::<u32>(q.wrapping_sub(4), my);
        g.write::<f32>(q, mul(mz, kf(g, img, c, 0x1b86)));
        g.write::<u32>(q + 0xc, my);
        g.write::<f32>(q + 8, mul(mx, k2));
        g.write::<f32>(q + 0x10, mul(mz, kf(g, img, c, 0x1bb4)));
        g.write::<u32>(q + 0x1c, my);
        let vx = g.read::<f32>(v);
        g.write::<f32>(q + 0x18, add(mul(vx, k3), mx));
        let vz = g.read::<f32>(v + 8);
        g.write::<f32>(q + 0x20, add(mul(vz, kf(g, img, c, 0x1be0)), mz));
        for j in 0..4 {
            let w = g.read::<u32>(m.wrapping_sub(8) + 4 * j);
            g.write::<u32>(p + 4 * j, w);
        }
        let a = g.read::<u32>(rec(0x1c05).wrapping_add(i));
        let b = g.read::<u32>(rec(0x1c0f).wrapping_add(i));
        let words = [
            3,
            0x14,
            0,
            0,
            rec(0x1c2e),
            0x4000_0000,
            0xbe4c_cccd,
            0x3d23_d70a,
            b,
            a,
            8,
        ];
        spline(g, img, c, 0x1c3b, (o, q.wrapping_sub(8)), words);
        p = p.wrapping_add(0x80);
        i += 4;
        o = o.wrapping_add(0xa8);
        m = m.wrapping_add(0xcc);
        v = v.wrapping_add(0x10);
        q = q.wrapping_add(0x30);
        if m.cast_signed() >= rec(0x1c6a).cast_signed() {
            break;
        }
    }
    let mut h = rec(0x1c75);
    loop {
        let holder = c.gw(g, img, 0x1c81);
        g.write::<u32>(holder, rec(0x1c87));
        call_slot(g, img, c, (0x1c90, 713), &[h.wrapping_sub(0x1e)]);
        g.write::<u16>(h, 0);
        h = h.wrapping_add(0x80);
        if h.cast_signed() >= rec(0x1ca9).cast_signed() {
            break;
        }
    }
}

/// The local `S + o` of the scene task's frame.
fn local(c: &Scene, o: u32) -> u32 {
    c.s.wrapping_add(o)
}

/// State 0x10 at the count's 0x14: for each model `M` (0xcc bytes apart,
/// with its direction `V`, its record `R`, its part `P`, its beam `T`, 0x40
/// bytes): part `P` started at `(V.x K + M.x, M.y, V.z K + M.z)` with the
/// angle `atan2(V.x, V.z) K1 / K2` (u16) and its constants; the beam's
/// direction from `R` (the third model from the first's) to `M` (slot
/// 234), normalised (232), its side `(D.z, 0, -D.x)` normalised and their
/// cross product (266) normalised; `T` holds side, up and direction; a
/// ring entry (the index `G_I`, wrapping to `G_W` at 0x200) gets `M`, the
/// other model's place and its constants.
#[allow(clippy::too_many_lines)] // the original's beams, in order
fn launch_10<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    use crate::sb_sse::{add, div, mul};
    let rec = |o: u32| c.w(img, o);
    let (mut m, mut v, mut r) = (rec(0x1cf8), rec(0x1cfd), rec(0x1d16));
    let (mut t, mut p) = (rec(0x1d1f), rec(0x1d02));
    let mut i = 0u32;
    loop {
        let a = g.read::<f32>(v.wrapping_sub(8));
        let x0 = add(mul(a, kf(g, img, c, 0x1d3f)), g.read::<f32>(m));
        let y = g.read::<u32>(m + 4);
        let b = g.read::<f32>(v);
        let x2 = add(mul(b, kf(g, img, c, 0x1d66)), g.read::<f32>(m + 8));
        let holder = c.gw(g, img, 0x1d70);
        g.write::<u32>(holder, rec(0x1d76));
        let dv = c.gw(g, img, 0x1d81);
        let table = call_slot(g, img, c, (0x1d7b, 941), &[dv]);
        let word = g.read::<u32>(table.wrapping_add(0x50));
        crate::opu_parts::start_part(
            g,
            img,
            c.target(img, 0x1da6),
            p.wrapping_sub(0x40),
            0x1000,
            word,
            1,
        );
        g.write::<u16>(p.wrapping_sub(0x22), 1);
        g.write::<f32>(p, x0);
        g.write::<u32>(p + 4, y);
        g.write::<f32>(p + 8, x2);
        let ang = g.atan2(f64::from(a), f64::from(b));
        g.write::<u32>(p + 0xc, 0x3f80_0000);
        g.write::<u32>(p + 0x10, 0);
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let ang = div(
            mul(ang as f32, kf(g, img, c, 0x1e2a)),
            kf(g, img, c, 0x1e40),
        );
        g.write::<u32>(p + 0x18, 0);
        g.write::<u32>(p + 0x1c, 1);
        g.write::<u32>(p + 0x20, 0x3f00_0000);
        g.write::<u32>(p + 0x24, 0x3f00_0000);
        g.write::<u32>(p + 0x14, trunc(f64::from(ang)) & 0xffff);
        g.write::<u32>(p + 0x28, 0x3f00_0000);
        g.write::<u32>(p + 0x2c, 0x3f80_0000);
        let from = if i < 2 { r } else { rec(0x1e6b) }.wrapping_add(0x3c);
        let (dir, n) = (local(c, 0xfc), local(c, 0x68));
        call_slot(g, img, c, (0x1e81, 234), &[dir, from, m]);
        call_slot(g, img, c, (0x1e9e, 232), &[n, dir]);
        let side = local(c, 0x58);
        let nz = g.read::<u32>(local(c, 0x70));
        g.write::<u32>(side, nz);
        g.write::<u32>(side + 4, 0);
        let nx = g.read::<u32>(n) ^ 0x8000_0000;
        g.write::<u32>(side + 8, nx);
        call_slot(g, img, c, (0x1ed2, 232), &[side, side]);
        let up = local(c, 0xa8);
        call_slot(g, img, c, (0x1ef7, 266), &[up, n, side]);
        call_slot(g, img, c, (0x1f10, 232), &[up, up]);
        let idx = c.gw(g, img, 0x1f23);
        let copy = |g: &mut G, to: u32, from: u32| {
            let x = g.read::<u32>(from);
            g.write::<u32>(to, x);
        };
        copy(g, t.wrapping_sub(8), side);
        copy(g, t.wrapping_sub(4), side + 4);
        copy(g, t, side + 8);
        copy(g, t + 8, up);
        copy(g, t + 0xc, up + 4);
        let e = idx.wrapping_mul(15).wrapping_mul(4);
        let next = idx.wrapping_add(1);
        let next = if next.cast_signed() >= 0x200 {
            c.gw(g, img, 0x1f79)
        } else {
            next
        };
        copy(g, t + 0x10, up + 8);
        c.set(g, img, 0x1f8d, next);
        copy(g, t + 0x18, n);
        copy(g, t + 0x1c, n + 4);
        let kind = i32::from(g.read::<i16>(rec(0x1f94)))
            .cast_unsigned()
            .wrapping_add(rec(0x1faf));
        copy(g, t + 0x20, n + 8);
        g.write::<u32>(rec(0x1fb6).wrapping_add(e), kind);
        for (j, op) in (0u32..).zip([0x1fccu32, 0x1fda]) {
            let x = g.read::<u64>(m + 8 * j);
            g.write::<u64>(rec(op).wrapping_add(e), x);
        }
        for (j, op) in (0u32..).zip([0x1fe7u32, 0x1ff9]) {
            let x = g.read::<u64>(from + 8 * j);
            g.write::<u64>(rec(op).wrapping_add(e), x);
        }
        for (op, x) in [
            (0x2000u32, 1u32),
            (0x200b, 0xa),
            (0x2016, 0),
            (0x2021, 0x40a0_0000),
            (0x202c, 0),
        ] {
            g.write::<u32>(rec(op).wrapping_add(e), x);
        }
        for o in [4u32, 0x14, 0x24, 0x28, 0x2c, 0x30] {
            g.write::<u32>(t + o, 0);
        }
        g.write::<u32>(t + 0x34, 0x3f80_0000);
        i += 1;
        v = v.wrapping_add(0x10);
        r = r.wrapping_add(0xcc);
        p = p.wrapping_add(0x80);
        m = m.wrapping_add(0xcc);
        t = t.wrapping_add(0x40);
        if p.cast_signed() >= rec(0x208d).cast_signed() {
            break;
        }
    }
}

/// A ring entry's address: entry `idx` (0x3c bytes) of the table whose
/// field the operand `op` names.
fn entry(c: &Scene, img: &Image, op: u32, idx: u32) -> u32 {
    c.w(img, op).wrapping_add(idx.wrapping_mul(0x3c))
}

/// The next ring index after `idx`: `G_W` (operand `wrap`) at 0x200.
fn next_index<G: Game>(g: &G, img: &Image, c: &Scene, idx: u32, wrap: u32) -> u32 {
    let next = idx.wrapping_add(1);
    if next.cast_signed() >= 0x200 {
        c.gw(g, img, wrap)
    } else {
        next
    }
}

/// State 0x10's bursts, `since` (0..0x64) frames after the beams:
/// `(since >> 3) + 2` rounds (0x20 at 0), each two sparks per model from three of
/// slot 307's randoms: a direction `A` (every 32nd frame up to 0x45 pushed
/// out by `K` sideways, with a third point `C`) scaled to 20 (slots 232,
/// 233), its mirror `B`, turned by the beam `T` (229); the ends `M + A`
/// and `Q + B` (248, `Q` the model's record, the first model's for the
/// third); two more randoms give a size and a life. Pushed ones become a
/// spline object from a free record ([`record_free`]); the others a ring
/// entry.
///
/// [`record_free`]: crate::sa_misc::record_free
#[allow(clippy::too_many_lines)] // the original's loops, in order
fn burst_10<G: Game>(g: &mut G, img: &Image, c: &Scene, since: i32) {
    use crate::sb_sse::{add, div, mul, sub};
    let rec = |o: u32| c.w(img, o);
    let k64 = |g: &G, o: u32| g.read::<f64>(c.w(img, o));
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let fr = div(since as f32, kf(g, img, c, 0x20d4));
    let rounds = if since == 0 { 0x20 } else { (since >> 3) + 2 };
    #[allow(clippy::cast_possible_truncation)] // cvtsd2ss
    let size = (f64::from(add(fr, kf(g, img, c, 0x20e2))) * k64(g, 0x2100)) as f32;
    let life = mul(
        sub(kf(g, img, c, 0x211e), mul(fr, kf(g, img, c, 0x20ea))),
        kf(g, img, c, 0x212a),
    );
    let (a, b, cc) = (local(c, 0x78), local(c, 0x98), local(c, 0x88));
    let (d1, d2, d3) = (local(c, 0xb8), local(c, 0xd8), local(c, 0xc8));
    let mut count = (since + 0x14).cast_unsigned();
    for _ in 0..rounds {
        let (mut m, mut r, mut t) = (rec(0x214c), rec(0x2154), rec(0x215c));
        let mut j = 0u32;
        loop {
            for k in 0..2u32 {
                let rnd =
                    |g: &mut G, op: u32, kop: u32| sub(frand(g, img, c, op), kf(g, img, c, kop));
                let r1 = rnd(g, 0x2163, 0x217d);
                #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
                let mut x = ((f64::from(r1) - k64(g, 0x218d)) * k64(g, 0x2195)) as f32;
                let r2 = rnd(g, 0x2182, 0x21b9);
                #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
                let mut y = ((f64::from(r2) - k64(g, 0x21c9)) * k64(g, 0x21d1)) as f32;
                let k1 = kf(g, img, c, 0x21f5);
                let r3 = sub(frand(g, img, c, 0x21be), k1);
                #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
                let z = (f64::from(r3) + k64(g, 0x2204)) as f32;
                let push = (since - 1).cast_unsigned() <= 0x44 && count.trailing_zeros() >= 5;
                if push {
                    x = if x > 0.0 { add(x, k1) } else { sub(x, k1) };
                    y = if y > 0.0 { add(y, k1) } else { sub(y, k1) };
                }
                g.write::<f32>(a, x);
                g.write::<f32>(a + 4, y);
                g.write::<f32>(a + 8, z);
                g.write::<u32>(a + 0xc, 0);
                call_slot(g, img, c, (0x2278, 232), &[a, a]);
                call_slot(g, img, c, (0x22a0, 233), &[a, a, 0x41a0_0000]);
                let (x, y) = (g.read::<f32>(a), g.read::<f32>(a + 4));
                let z = g.read::<u32>(a + 8) ^ 0x8000_0000;
                g.write::<u32>(a + 0xc, 0x3f80_0000);
                g.write::<f32>(b, x);
                g.write::<f32>(b + 4, y);
                g.write::<u32>(b + 8, z);
                g.write::<u32>(b + 0xc, 0x3f80_0000);
                if push {
                    g.write::<f32>(cc, mul(x, kf(g, img, c, 0x230a)));
                    g.write::<u32>(cc + 8, 0);
                    g.write::<f32>(cc + 4, mul(y, kf(g, img, c, 0x2312)));
                    g.write::<u32>(cc + 0xc, 0x3f80_0000);
                }
                call_slot(g, img, c, (0x2349, 229), &[a, t, a]);
                call_slot(g, img, c, (0x2366, 229), &[b, t, b]);
                if push {
                    call_slot(g, img, c, (0x2387, 229), &[cc, t, cc]);
                }
                let q = if j < 2 { r } else { rec(0x23b2) }.wrapping_add(0x3c);
                call_slot(g, img, c, (0x23ad, 248), &[d1, m, a]);
                g.write::<u32>(d1 + 0xc, 0x3f80_0000);
                call_slot(g, img, c, (0x23df, 248), &[d2, q, b]);
                g.write::<u32>(d2 + 0xc, 0x3f80_0000);
                let r4 = rnd(g, 0x23f7, 0x241f);
                #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
                let span = (f64::from(r4) * k64(g, 0x243b) * f64::from(size)) as f32;
                let r5 = rnd(g, 0x2424, 0x2465);
                let n = trunc(f64::from(r5) * k64(g, 0x2476) * f64::from(life));
                if push {
                    call_slot(g, img, c, (0x249c, 248), &[d3, m, q]);
                    call_slot(g, img, c, (0x24bc, 233), &[d3, d3, 0x3f00_0000]);
                    call_slot(g, img, c, (0x24dd, 248), &[d3, d3, cc]);
                    g.write::<u32>(d3 + 0xc, 0x3f80_0000);
                    let p = crate::sa_misc::record_free(g, img, c.target(img, 0x24f8));
                    if p != 0 {
                        let len = (n.cast_signed() >> 2).wrapping_add(5).cast_unsigned();
                        let words = [
                            3,
                            len,
                            1,
                            k & 1,
                            rec(0x252f),
                            0x3d4c_cccd,
                            0x3acc_cccd,
                            0,
                            0x8080_8ca0,
                            0x0404_0404,
                            1,
                        ];
                        spline(g, img, c, 0x254a, (p, d1), words);
                    }
                } else {
                    let idx = c.gw(g, img, 0x2557);
                    let e = |op: u32| entry(c, img, op, idx);
                    g.write::<u32>(e(0x2566), 1);
                    g.write::<u32>(e(0x2571), n);
                    g.write::<u32>(e(0x2578), 0);
                    g.write::<u32>(e(0x2583), 0x4080_8080);
                    let (first, second) = if k == 0 { (d1, d2) } else { (d2, d1) };
                    let ops = if k == 0 {
                        [0x259du32, 0x25af, 0x25c1]
                    } else {
                        [0x25de, 0x25f0, 0x2602]
                    };
                    for (op, from) in ops.into_iter().zip([first, first + 8, second]) {
                        let v = g.read::<u64>(from);
                        g.write::<u64>(e(op), v);
                    }
                    let next = next_index(g, img, c, idx, 0x2617);
                    c.set(g, img, 0x261c, next);
                    let kind = i32::from(g.read::<i16>(rec(0x2623)))
                        .cast_unsigned()
                        .wrapping_add(rec(0x2637));
                    let v = g.read::<u64>(second + 8);
                    g.write::<u64>(e(0x262c), v);
                    g.write::<f32>(e(0x2640), span);
                    g.write::<u32>(e(0x2647), kind);
                }
                count = count.wrapping_add(1);
            }
            j += 1;
            r = r.wrapping_add(0xcc);
            m = m.wrapping_add(0xcc);
            t = t.wrapping_add(0x40);
            if t.cast_signed() >= rec(0x267d).cast_signed() {
                break;
            }
        }
    }
}

/// State 0x10 from `+0x269c` (the count `n`, `since` = `n - 0x14`, 0 or
/// more): below 0xb8 the ring entries age out, and from 0xf frames the
/// first three take a size, life and colour by the frame; at 0x96 the
/// cameras' points, slot 222 and the actors' slot 296; from 0x96 to 0xdc
/// the models' ring shrinks (a sine) with the ring entries on the models'
/// places, and before 0xb4 the parts' halfwords; from 0xbe to 0xf9 the
/// colours fade, at 0xfa the parts registered and the ring cleared; at
/// 0xb8 part P6, at 0xc2 the three parts P5 stopped and the records
/// cleared, at 0x112 the closing curve and part P4. Returns the count to
/// step.
#[allow(clippy::too_many_lines)] // the original's blocks, in order
fn late_10<G: Game>(g: &mut G, img: &Image, c: &Scene, (n, since): (u16, i32)) -> u16 {
    use crate::sb_sse::{add, div, mul, sub};
    let rec = |o: u32| c.w(img, o);
    let mut n = n;
    if n.cast_signed() < 0xb8 {
        let mut p = rec(0x26b0);
        for _ in 0..0x1fd {
            if g.read::<u32>(p.wrapping_sub(8)) != 0 {
                let v = g.read::<u32>(p).wrapping_add(1);
                g.write::<u32>(p, v);
                if v.cast_signed() >= g.read::<u32>(p + 4).cast_signed() {
                    g.write::<u32>(p.wrapping_sub(8), 0);
                }
            }
            p = p.wrapping_add(0x3c);
        }
        if since >= 0xf {
            let s = since - 0xf;
            let size = (s >> 2).min(0x14).cast_unsigned();
            let life = (0x1e - div_magic(s.wrapping_mul(30), 0x6060_6061, false, 5))
                .max(4)
                .cast_unsigned();
            let shade = div_magic(s.wrapping_mul(96), 0x6060_6061, false, 5)
                .min(0x60)
                .cast_unsigned();
            let colour = shade.wrapping_mul(0x0101_0101);
            let mut p = rec(0x2765);
            for _ in 0..3 {
                if g.read::<u32>(p.wrapping_sub(4)) != 0 {
                    let v = g.read::<u32>(p + 4).wrapping_add(1);
                    g.write::<u32>(p + 4, v);
                    g.write::<u32>(p, size);
                    let v = if v.cast_signed() >= life.cast_signed() {
                        0
                    } else {
                        v
                    };
                    g.write::<u32>(p + 4, v);
                    g.write::<u32>(p + 8, life);
                    g.write::<u32>(p + 0xc, colour);
                }
                p = p.wrapping_add(0x3c);
            }
        }
    }
    if n == 0x96 {
        for (from, to) in [
            (0x27bau32, 0x27c7u32),
            (0x27cf, 0x27d7),
            (0x27df, 0x27e7),
            (0x27ef, 0x27f7),
            (0x27ff, 0x280c),
            (0x2814, 0x2826),
        ] {
            let v = g.read::<u32>(rec(from));
            g.write::<u32>(rec(to), v);
        }
        slot_222(g, img, c, (0x27bf, 0x281e), 0x14, 0x18);
        let mut p = rec(0x2836);
        for _ in 0..3 {
            let actor = g.read::<u32>(p);
            if actor != 0xff {
                call_slot(g, img, c, (0x284d, 296), &[actor, 0]);
            }
            p = p.wrapping_add(0xcc);
        }
        n = half(g, img, c, 0x2868);
    }
    let s = i32::from(n.cast_signed()) - 0x96;
    if s.cast_unsigned() <= 0x46 {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
        let x = mul(div(s as f32, kf(g, img, c, 0x288e)), kf(g, img, c, 0x289b));
        let ang = (0x4000u32.wrapping_sub(trunc(f64::from(x))).cast_signed() >> 4).cast_unsigned()
            & 0xfff;
        let tab = c.gw(g, img, 0x28ad);
        let sn = g.read::<f32>(tab.wrapping_add(4 * ang));
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let r = (f64::from(sn) * g.read::<f64>(rec(0x28cc))) as f32;
        ring(g, img, c, 0x28d5, (rec(0x28c4), 3), r, (0, 0x8000));
        // the models' places (A, B, C, two words each) into the ring
        // entries and the parts' points
        let place: Vec<u64> = [0x28ddu32, 0x28e5, 0x28ed, 0x28f5, 0x28fd, 0x2905]
            .iter()
            .map(|&o| g.read::<u64>(rec(o)))
            .collect();
        for (k, src) in (0u32..).zip([0usize, 1, 2, 3, 0, 1, 2, 3, 4, 5, 2, 3, 4, 5, 0, 1, 4, 5]) {
            g.write::<u64>(rec(0x2910 + 8 * k), place[src]);
        }
        if s < 0x1e {
            #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
            let t = sub(
                kf(g, img, c, 0x29af),
                div((s << 12) as f32, kf(g, img, c, 0x29b7)),
            );
            let h = (trunc(f64::from(t)) & 0xffff) as u16;
            for k in 0..12 {
                g.write::<u16>(rec(0x29c8 + 6 * k), h);
            }
        }
    }
    let e = i32::from(n.cast_signed()) - 0xbe;
    if e.cast_unsigned() <= 0x3b {
        let s = 0x3c - e;
        let shade = div_magic(s * 96, 0x8888_8889, true, 5).cast_unsigned();
        let colour = shade.wrapping_mul(0x0101_0101);
        let life = div_magic(s * 20, 0x8888_8889, true, 5).cast_unsigned();
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
        let f = div(s as f32, kf(g, img, c, 0x2a77));
        let mut p = rec(0x2a81);
        for _ in 0..3 {
            if g.read::<u32>(p.wrapping_sub(0x10)) != 0 {
                g.write::<u32>(p, colour);
                g.write::<u32>(p.wrapping_sub(0xc), life);
                g.write::<f32>(p + 4, f);
            }
            p = p.wrapping_add(0x3c);
        }
    } else if e == 0x3c {
        let fr = Frame {
            eax: 0x3c,
            ..c.frame(img, 0x2aae, 0)
        };
        crate::s4_misc::register_parts_b(g, img, &fr);
        let mut p = rec(0x2ab8);
        for _ in 0..0x200 {
            g.write::<u32>(p.wrapping_sub(0x18), 0);
            g.write::<u32>(p, 0);
            p = p.wrapping_add(0x3c);
        }
        n = half(g, img, c, 0x2ad6);
        c.set(g, img, 0x2adc, 0);
        c.set(g, img, 0x2ae2, 0);
    }
    match n {
        0xb8 => {
            let y = kf(g, img, c, 0x2afd);
            let ops = [0x2af5, 0x2b03, 0x2b08, 0x2b0e, 0x2b35, 0x2b30];
            part_10(g, img, c, ops, 0x3c, y, [0x2b4b, 0x2b5a]);
        }
        0xc2 => {
            let mut h = rec(0x2b9e);
            loop {
                stop_part_at(g, img, c, (0x2ba3, 0x2ba9, 0x2bb2), h.wrapping_sub(0x1e));
                g.write::<u16>(h, 0);
                h = h.wrapping_add(0x80);
                if h.cast_signed() >= rec(0x2bcb).cast_signed() {
                    break;
                }
            }
            crate::sa_misc::register_counted(g, img, c.target(img, 0x2bd2), 0);
            let mut p = rec(0x2bd7);
            loop {
                g.write::<u32>(p.wrapping_sub(0x24), 0);
                g.write::<u32>(p, 0);
                p = p.wrapping_add(0xa8);
                if p.cast_signed() >= rec(0x2bf3).cast_signed() {
                    break;
                }
            }
        }
        0x112 => {
            call_slot(g, img, c, (0x2c0d, 325), &[rec(0x2c1d), rec(0x2c12)]);
            call_slot(g, img, c, (0x2c24, 325), &[rec(0x2c34), rec(0x2c29)]);
            let v = add(kf(g, img, c, 0x2c3e), kf(g, img, c, 0x2c46));
            g.write::<f32>(rec(0x2c8b), v);
            let words = [
                2,
                0x3c,
                0,
                0,
                rec(0x2c7b),
                0x4080_0000,
                0xbecc_cccd,
                0x3da3_d70a,
                0x1080_8ca0,
                0,
                8,
            ];
            spline(g, img, c, 0x2c90, (rec(0x2c5f), rec(0x2c4e)), words);
            let y = add(kf(g, img, c, 0x2c98), kf(g, img, c, 0x2ca5));
            let ops = [0x2c9d, 0x2cab, 0x2cb0, 0x2cbf, 0x2ce0, 0x2cdb];
            part_10(g, img, c, ops, 0x38, y, [0x2cf6, 0x2d05]);
        }
        _ => return n,
    }
    half(g, img, c, 0x2d3f)
}

/// Slot 713 on the part `p` with `*G_H = L` first (operands `(G_H, L,
/// imports)`).
fn stop_part_at<G: Game>(g: &mut G, img: &Image, c: &Scene, (h, l, op): (u32, u32, u32), p: u32) {
    let holder = c.gw(g, img, h);
    g.write::<u32>(holder, c.w(img, l));
    call_slot(g, img, c, (op, 713), &[p]);
}

/// State 0x18 (`+0x3961`): once (the halfword `G_O`), the actors' slot
/// 296, slot 325 `(C, X)` gives the centre `C` that all three models take,
/// the models' angle copied, the second model moved by `(9.5, 0, 10.5)`
/// turned by the first's angle (slots 247, 229), its actor's slot 296
/// `(id, 1)`, its sound, the matrix turned by the angle, the camera record
/// started. While not busy: slot 220 and slot 306 `(3)` at the count's 0;
/// the three models' sounds ([`sound_step_18`]); the flash `G_F` while
/// its frame is below 0xf; on to state 0x64 at 0xa0.
fn state_18<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    use crate::sb_sse::{add, div, sub};
    let rec = |o: u32| c.w(img, o);
    if half(g, img, c, 0x3964) == 0 {
        set_half(g, img, c, 0x3978, 1);
        crate::s4_misc::actors_296(g, img, &c.frame(img, 0x397d, 0));
        call_slot(g, img, c, (0x3982, 325), &[rec(0x3992), rec(0x3987)]);
        let ang = half(g, img, c, 0x399a);
        let (c0, c1) = (g.read::<u64>(rec(0x39a2)), g.read::<u64>(rec(0x39aa)));
        set_half(g, img, c, 0x39b0, ang);
        set_half(g, img, c, 0x39b6, ang);
        let turn = 0x4000u32.wrapping_sub(i32::from(ang.cast_signed()).cast_unsigned());
        let (m, v) = (local(c, 0xb8), local(c, 0x58));
        for (k, op) in (0usize..).zip([0x39d4u32, 0x39dc, 0x39e4, 0x39ec, 0x39f4, 0x39fc]) {
            g.write::<u64>(rec(op), if k % 2 == 0 { c0 } else { c1 });
        }
        for (i, x) in (0u32..).zip([0x4118_0000u32, 0, 0x4128_0000, 0x3f80_0000]) {
            g.write::<u32>(v + 4 * i, x);
        }
        call_slot(g, img, c, (0x39cc, 247), &[m, turn]);
        call_slot(g, img, c, (0x3a3b, 229), &[v, m, v]);
        let id = c.gw(g, img, 0x3a56);
        let x = add(kf(g, img, c, 0x3a4b), g.read::<f32>(v));
        g.write::<f32>(rec(0x3a61), x);
        let z = add(kf(g, img, c, 0x3a69), g.read::<f32>(v + 8));
        g.write::<f32>(rec(0x3a77), z);
        if id != 0xff {
            call_slot(g, img, c, (0x3a86, 296), &[id, 1]);
        }
        crate::sa_misc::sound_start(g, img, c.target(img, 0x3aa0), rec(0x3a9b), 0xc8);
        let a = i32::from(g.read::<i16>(rec(0x3aa7))).cast_unsigned();
        crate::sa_misc::turn_matrix(g, img, c.target(img, 0x3aac), a);
        let mut fr = c.frame(img, 0x3ab6, 0);
        fr.ecx = rec(0x3ab1);
        crate::s6_misc::camera_tracks_from(g, img, &fr);
    }
    if c.busy(g, img, 0x3abb) {
        return;
    }
    if half(g, img, c, 0x3acb) == 0 {
        let mut fr = c.frame(img, 0x3ada, 1);
        fr.ecx = c.gw(g, img, 0x3ad5);
        crate::s4_misc::call_220(g, img, &fr);
        crate::sa_misc::slot306(g, img, c.target(img, 0x3ae4), 3);
    }
    let mut p = rec(0x3af2);
    for i in 0..3u32 {
        sound_step_18(g, img, c, p, i);
        p = p.wrapping_add(0xcc);
    }
    let t = half(g, img, c, 0x3c70);
    if t.cast_signed() >= 0xf {
        c.stores(g, img, 0x3d7d, 0x3d84);
    } else {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
        let f = div(f32::from(t.cast_signed()), kf(g, img, c, 0x3c92));
        let at = rec(0x3c99);
        let n = g.read::<u16>(at).wrapping_add(1);
        g.write::<u16>(at, n);
        let v = sub(kf(g, img, c, 0x3c8a), f);
        c.stores(g, img, 0x3ca1, 0x3cbc);
        g.write::<f32>(rec(0x3cc0), v);
        g.write::<f32>(rec(0x3cc8), v);
    }
    let n = half(g, img, c, 0x3d86).wrapping_add(1);
    set_half(g, img, c, 0x3d93, n);
    if n.cast_signed() >= 0xa0 {
        c.stores(g, img, 0x3da0, 0x3daa);
    }
}

/// One of state 0x18's model sounds (`p` the record's `+8`, `i` 0..3): at
/// the count's 0x28 the third and first models' actors take slot 296 `(id,
/// 1)` and kind 0xc8; with kind 0xc8, the third model at frame 0x54 sends
/// part P7 to the middle of its points 0x1c and 0x11 (slot 237), the
/// first at frame 0x11 part P8 (the model's place, lowered); any at its
/// limit moves on to 0xc9.
fn sound_step_18<G: Game>(g: &mut G, img: &Image, c: &Scene, p: u32, i: u32) {
    use crate::sb_sse::{add, mul, sub};
    let rec = |o: u32| c.w(img, o);
    let r = p.wrapping_sub(8);
    let count = |g: &G, o: u32| half(g, img, c, o);
    let (start_at, hit) = match i {
        2 => ((0x3b1c, 0x3b2a, 0x3b37), 0x54),
        0 => ((0x3cd4, 0x3ce4, 0x3cf1), 0x11),
        _ => ((0, 0, 0), 0),
    };
    if i != 1 && count(g, start_at.0) == 0x28 {
        crate::sa_misc::record_296(g, img, c.target(img, start_at.1), r, 1);
        crate::sa_misc::sound_start(g, img, c.target(img, start_at.2), r, 0xc8);
    }
    if g.read::<u32>(p) != 0xc8 {
        return;
    }
    let frame = g.read::<u32>(p + 8);
    if i == 1 || frame != hit {
        if frame == g.read::<u32>(p + 4) {
            crate::sa_misc::sound_start(g, img, c.target(img, 0x3c56), r, 0xc9);
        }
        return;
    }
    if i == 2 {
        let (a, b) = (local(c, 0x68), local(c, 0xa8));
        let m = g.read::<u32>(p.wrapping_sub(4));
        call_slot(g, img, c, (0x3b5d, 237), &[m, 0x1c, 0, a]);
        call_slot(g, img, c, (0x3b79, 237), &[m, 0x11, 0, b]);
        let k = kf(g, img, c, 0x3bc6);
        let mid = |g: &G, o: u32| mul(add(g.read::<f32>(b + o), g.read::<f32>(a + o)), k);
        let (x, y, z) = (mid(g, 0), mid(g, 4), mid(g, 8));
        g.write::<f32>(a, x);
        g.write::<f32>(a + 4, y);
        g.write::<f32>(a + 8, z);
        motion(
            g,
            img,
            c,
            0x3c34,
            (rec(0x3c0f), 0x13),
            (x, y),
            (z.to_bits(), 0x3dcc_cccd),
        );
    } else {
        let y = sub(g.read::<f32>(p + 0x38), kf(g, img, c, 0x3d1a));
        let a = g.read::<u32>(p + 0x3c);
        let x = g.read::<f32>(p + 0x34);
        motion(
            g,
            img,
            c,
            0x3d69,
            (rec(0x3d5f), frame - 6),
            (x, y),
            (a, 0x3e2e_147b),
        );
        set_half(g, img, c, 0x3d74, 0);
    }
}

/// [`part_motion`](crate::s6_misc::part_motion) with all seven stack
/// words given.
fn motion_words<G: Game>(
    g: &mut G,
    img: &Image,
    c: &Scene,
    call: u32,
    (part, index): (u32, u32),
    (x2, x3): (f32, f32),
    words: [u32; 7],
) {
    let sp = c.s.wrapping_sub(0x1c);
    for (i, v) in (0u32..).zip(words) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let mut fr = c.frame(img, call, 7);
    fr.ecx = part;
    fr.edx = index;
    fr.xmm[2] = u128::from(x2.to_bits());
    fr.xmm[3] = u128::from(x3.to_bits());
    crate::s6_misc::part_motion(g, img, &fr);
}

/// State 0 of `magic_0515` (`+0x15c`): once (the halfword `G_O`): with
/// either flag `G_A`/`G_B`, slot 278 `(*G_M, 4..=7, 1)`; slot 306 `(0)`,
/// the actors' slot 296, slot 309 on the sounds, the three records' slot
/// 296, the records' with 0, both part groups' live axes, the first
/// model's sound (its answer the count `G_N`), the first model on the
/// centre, the matrix turned by its angle, the camera record started.
/// While not busy: at the camera's second record (just begun) slot 295 on
/// the records; by the count: 0x3e slot 220 and part P9's motion, 0x19
/// the centre from the first model's point 5 (slot 237) lowered by its
/// height (slot 289), part P10's motion and slot 276; between 6 and 0x15
/// the spread `(n - 6) / K`, else the flash off; the count steps down,
/// and at 0 the model's sound gives the count and state 1 starts.
#[allow(clippy::too_many_lines)] // the original's state, in order
fn state_0_0515<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    use crate::sb_sse::{div, sub};
    let rec = |o: u32| c.w(img, o);
    if half(g, img, c, 0x15f) == 0 {
        set_half(g, img, c, 0x178, 1);
        if c.gw(g, img, 0x16c) != 0 || c.gw(g, img, 0x180) != 0 {
            for (op, m, k) in [
                (0x188u32, 0x198u32, 4u32),
                (0x19f, 0x1b2, 5),
                (0x1b9, 0x1cc, 6),
                (0x1d3, 0x1e6, 7),
            ] {
                let x = c.gw(g, img, m);
                call_slot(g, img, c, (op, 278), &[x, k, 1]);
            }
        }
        let eax = crate::sa_misc::slot306(g, img, c.target(img, 0x1f3), 0);
        let fr = Frame {
            eax,
            ..c.frame(img, 0x1fd, 0)
        };
        let eax = crate::s4_misc::actors_296(g, img, &fr).eax;
        crate::sa_misc::repeat_309(g, img, c.target(img, 0x202), eax);
        for (call, r) in [(0x20eu32, 0x209u32), (0x21a, 0x215), (0x226, 0x221)] {
            crate::sa_misc::record_296(g, img, c.target(img, call), rec(r), 0);
        }
        g.write::<u32>(c.s.wrapping_sub(4), 0);
        crate::s4_small::records_296_with(g, img, &c.frame(img, 0x22d, 1));
        let axes = |g: &mut G, call: u32, ecx: u32, edx: u32| {
            let fr = Frame {
                ecx,
                edx,
                ..c.frame(img, call, 0)
            };
            crate::s6_misc::live_parts_axes(g, img, &fr);
        };
        axes(g, 0x23f, rec(0x23a), 3);
        let n = c.gw(g, img, 0x245);
        axes(g, 0x24f, rec(0x24a), n);
        let s = crate::sa_misc::sound_start(g, img, c.target(img, 0x25e), rec(0x259), 0xc0);
        let p0 = g.read::<u64>(rec(0x266));
        let a = i32::from(g.read::<i16>(rec(0x26d))).cast_unsigned();
        g.write::<u64>(rec(0x275), p0);
        let p1 = g.read::<u64>(rec(0x27d));
        g.write::<u16>(rec(0x283), s as u16);
        g.write::<u64>(rec(0x28b), p1);
        crate::sa_misc::turn_matrix(g, img, c.target(img, 0x290), a);
        let mut fr = c.frame(img, 0x29a, 0);
        fr.ecx = rec(0x295);
        crate::s6_misc::camera_tracks_from(g, img, &fr);
    }
    if c.busy(g, img, 0x29f) {
        return;
    }
    if c.gw(g, img, 0x2ae) == 2 && c.gw(g, img, 0x2b7) == 0 {
        crate::sa_misc::records_295(g, img, c.target(img, 0x2bf));
    }
    let n = half(g, img, c, 0x2c6);
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let angle = |g: &G, o: u32| f32::from(g.read::<i16>(rec(o))).to_bits();
    let n = if n == 0x3e {
        let mut fr = c.frame(img, 0x2d8, 1);
        fr.ecx = c.gw(g, img, 0x2d3);
        crate::s4_misc::call_220(g, img, &fr);
        let ang = angle(g, 0x2df);
        let (x3, x2) = (kf(g, img, c, 0x2e7), kf(g, img, c, 0x2ef));
        let a = c.gw(g, img, 0x326);
        let k = 0x3e4c_cccd;
        motion_words(
            g,
            img,
            c,
            0x342,
            (rec(0x338), 0xa),
            (x2, x3),
            [a, 0, ang, 0, k, k, k],
        );
        half(g, img, c, 0x412)
    } else if n == 0x19 {
        let x = c.gw(g, img, 0x36e);
        call_slot(g, img, c, (0x359, 237), &[x, 5, 0, rec(0x35e)]);
        let id = c.gw(g, img, 0x376);
        let (h, _) = crate::sb_nodes::float_289(g, img, c.target(img, 0x37b), id);
        let ang = angle(g, 0x382);
        let x2 = kf(g, img, c, 0x38a);
        let x3 = sub(h, kf(g, img, c, 0x395));
        let a = c.gw(g, img, 0x3d1);
        g.write::<f32>(rec(0x3e6), x3);
        let k = 0x3db8_51ec;
        motion_words(
            g,
            img,
            c,
            0x3f0,
            (rec(0x3de), 0xc),
            (x2, x3),
            [a, 0, ang, 0, k, k, k],
        );
        let m = c.gw(g, img, 0x406);
        call_slot(g, img, c, (0x3f5, 276), &[m, 1]);
        half(g, img, c, 0x412)
    } else {
        n
    };
    let w = c.gw(g, img, 0x417).wrapping_sub(6);
    if (w as u16) <= 0xf {
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
        let s = div(
            (i32::from(n.cast_signed()) - 6) as f32,
            kf(g, img, c, 0x436),
        );
        let mut fr = c.frame(img, 0x443, 1);
        fr.ecx = rec(0x43b);
        fr.xmm[1] = u128::from(s.to_bits());
        fr.xmm[2] = u128::from(s.to_bits());
        crate::sa_misc::spread_set(g, &fr);
    } else {
        c.stores(g, img, 0x44c, 0x453);
    }
    let at = rec(0x456);
    let t = g.read::<u16>(at).wrapping_add(1);
    g.write::<u16>(at, t);
    let n = n.wrapping_sub(1);
    set_half(g, img, c, 0x45f, n);
    if n == 0 {
        let s = crate::sa_misc::sound_start(g, img, c.target(img, 0x474), rec(0x46f), 0xc1);
        g.write::<u16>(rec(0x47a), s as u16);
        c.stores(g, img, 0x47e, 0x488);
    }
}

/// [`parts_placed`](crate::s8_misc::parts_placed) (the call at `call`,
/// three words pushed: `(junk, angle, word)`, the first never read and 0
/// here) with the radius in `xmm3`.
fn placed<G: Game>(
    g: &mut G,
    img: &Image,
    c: &Scene,
    call: u32,
    r: f32,
    (angle, word): (u32, u32),
) {
    let sp = c.s.wrapping_sub(0xc);
    for (i, v) in (0u32..).zip([0, angle, word]) {
        g.write::<u32>(sp + 4 * i, v);
    }
    let mut fr = c.frame(img, call, 3);
    fr.xmm[3] = u128::from(r.to_bits());
    crate::s8_misc::parts_placed(g, img, &fr);
}

/// State 3 of `magic_0515` (`+0x4c3`): once (the halfword `G_O`), part
/// P10 stopped, slot 222 `(.., 0, 3)`, the centre 0, the matrix turned,
/// the camera record started, slots 276, 267 (1, 4, 0x10), the block's
/// slot 331, slot 332, the records' slot 295, the three actors' slot 296
/// `(id, 1)` and the actors', the three sounds, the models' ring and the
/// parts placed, part P11's motion. While not busy: the fade by the count
/// (1 up to 0xcd, down to 0 over 0x3c frames), slot 220 at 0, slot 300
/// `(&fade, &out)`; by the count: 0x5a the three parts P12 on the models
/// (slots 247, 229), 0x6e part P13, 0xd7 part P4, 0x116 P13 stopped; at
/// 0x104 the three sounds again; from 0x104 the models' sounds
/// ([`sound_step_3`]); from 0xcd the colour, the parts placed and the
/// records' sizes by the fade; on to state 6 at 0x140.
#[allow(clippy::too_many_lines)] // the original's state, in order
fn state_3_0515<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    use crate::sb_sse::{add, div, mul, sub};
    let rec = |o: u32| c.w(img, o);
    if half(g, img, c, 0x4c6) == 0 {
        let holder = c.gw(g, img, 0x4d2);
        g.write::<u32>(holder, rec(0x4dd));
        set_half(g, img, c, 0x4f4, 1);
        call_slot(g, img, c, (0x4e2, 713), &[rec(0x4e7)]);
        set_half(g, img, c, 0x50f, 0);
        slot_222(g, img, c, (0x514, 0x509), 0, 3);
        c.stores(g, img, 0x522, 0x540);
        crate::sa_misc::turn_matrix(g, img, c.target(img, 0x541), 0);
        let mut fr = c.frame(img, 0x54b, 0);
        fr.ecx = rec(0x546);
        crate::s6_misc::camera_tracks_from(g, img, &fr);
        if c.alt {
            // mov ecx, M; call call_276
            let mut fr = c.frame(img, 0x555, 0);
            fr.ecx = c.w(img, 0x550);
            crate::s4_small::call_276(g, img, &fr);
        } else {
            let m = c.gw(g, img, 0x558);
            call_slot(g, img, c, (0x550, 276), &[m, 0]);
        }
        for (op, v) in [(0x565u32, 1u32), (0x573, 4), (0x585, 0x10)] {
            call_slot(g, img, c, (op, 267), &[v]);
        }
        crate::sa_misc::block_331(g, img, c.target(img, 0x59a));
        call_slot(g, img, c, (0x59f, 332), &[rec(0x5a4)]);
        crate::sa_misc::records_295(g, img, c.target(img, 0x5b4));
        for (call, r) in [(0x5c0u32, 0x5bbu32), (0x5cc, 0x5c7), (0x5d8, 0x5d3)] {
            crate::sa_misc::record_296(g, img, c.target(img, call), rec(r), 1);
        }
        let mut fr = c.frame(img, 0x5df, 0);
        fr.ecx = 1;
        crate::s4_misc::actors_296(g, img, &fr);
        for (call, r, x) in [
            (0x5eeu32, 0x5e9u32, 0xc2u32),
            (0x5fd, 0x5f8, 0xc0),
            (0x60c, 0x607, 0xc0),
        ] {
            crate::sa_misc::sound_start(g, img, c.target(img, call), rec(r), x);
        }
        let r = kf(g, img, c, 0x614);
        ring(g, img, c, 0x629, (rec(0x624), 3), r, (0, 0x8000));
        let r = kf(g, img, c, 0x636);
        placed(g, img, c, 0x63c, r, (0, 0x8000));
        let a = c.gw(g, img, 0x644);
        let (x3, x2) = (kf(g, img, c, 0x64c), kf(g, img, c, 0x654));
        let k = 0x3d50_e560;
        motion_words(
            g,
            img,
            c,
            0x697,
            (rec(0x68d), 0),
            (x2, x3),
            [a, 0, 0x45aa_a800, 0, k, k, k],
        );
    }
    if c.busy(g, img, 0x69f) {
        return;
    }
    let n = half(g, img, c, 0x6af);
    let e = i32::from(n.cast_signed()) - 0xcd;
    #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
    let fade = if e <= 0 {
        1.0
    } else if e >= 0x3c {
        0.0
    } else {
        div((0x3c - e) as f32, kf(g, img, c, 0x6eb))
    };
    let (fade_at, out) = (local(c, 0x14), local(c, 0x18));
    g.write::<f32>(fade_at, fade);
    if n == 0 {
        let mut fr = c.frame(img, 0x702, 1);
        fr.ecx = c.gw(g, img, 0x6fd);
        crate::s4_misc::call_220(g, img, &fr);
    }
    call_slot(g, img, c, (0x714, 300), &[fade_at, out]);
    let fade = g.read::<f32>(out);
    let n = half(g, img, c, 0x723).wrapping_add(1);
    g.write::<f32>(fade_at, fade);
    set_half(g, img, c, 0x73b, n);
    let n = match n {
        0x5a => {
            let (mut m, mut part, mut ix) = (rec(0x74a), rec(0x752), rec(0x757));
            let (mat, v) = (local(c, 0xb8), local(c, 0x48));
            loop {
                let ang = i32::from(g.read::<i16>(m.wrapping_sub(0x26)));
                let r = mul(g.read::<f32>(m.wrapping_sub(0x20)), kf(g, img, c, 0x76d));
                g.write::<f32>(local(c, 0x28), r);
                call_slot(
                    g,
                    img,
                    c,
                    (0x77d, 247),
                    &[mat, ang.wrapping_neg().cast_unsigned()],
                );
                for (i, x) in (0u32..).zip([r.to_bits(), 0, 0, 0x3f80_0000]) {
                    g.write::<u32>(v + 4 * i, x);
                }
                call_slot(g, img, c, (0x7a8, 229), &[v, mat, v]);
                let lift = mul(g.read::<f32>(m.wrapping_sub(0x1c)), kf(g, img, c, 0x7db));
                let ang = i32::from(g.read::<i16>(m.wrapping_sub(0x26)));
                let y = sub(g.read::<f32>(m + 4), lift);
                let z = add(g.read::<f32>(m + 8), g.read::<f32>(v + 8));
                let x = add(g.read::<f32>(v), g.read::<f32>(m));
                let index = g.read::<u32>(ix);
                #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
                let turn = ((-0x4000i32).wrapping_sub(ang) as f32).to_bits();
                g.write::<f32>(v, x);
                g.write::<f32>(v + 4, y);
                g.write::<f32>(v + 8, z);
                let h = 0x3f00_0000;
                motion_words(
                    g,
                    img,
                    c,
                    0x85d,
                    (part, index),
                    (x, y),
                    [z.to_bits(), 0, turn, 0, h, h, h],
                );
                part = part.wrapping_add(0x80);
                ix = ix.wrapping_add(4);
                m = m.wrapping_add(0xcc);
                if ix.cast_signed() >= rec(0x874).cast_signed() {
                    break;
                }
            }
            half(g, img, c, 0x973)
        }
        0x6e => {
            let h = 0x3f00_0000;
            motion_words(
                g,
                img,
                c,
                0x8d4,
                (rec(0x8c8), 0x10),
                (0.0, 0.0),
                [0, 0, 0, 0, h, h, h],
            );
            half(g, img, c, 0x973)
        }
        0xd7 => {
            let h = 0x3f00_0000;
            motion_words(
                g,
                img,
                c,
                0x935,
                (rec(0x929), 2),
                (0.0, 0.0),
                [0, 0, 0xc5aa_a800, 0, h, h, h],
            );
            half(g, img, c, 0x973)
        }
        0x116 => {
            stop_part(g, img, c, (0x949, 0x954), (0x959, 0x94e));
            set_half(g, img, c, 0x96c, 0);
            half(g, img, c, 0x973)
        }
        _ => n,
    };
    let n = if n.cast_signed() < 0x104 {
        n
    } else {
        if n == 0x104 {
            for (call, r, x) in [
                (0x990u32, 0x98bu32, 0xc3u32),
                (0x99f, 0x99a, 0xc1),
                (0x9ae, 0x9a9, 0xc1),
            ] {
                crate::sa_misc::sound_start(g, img, c.target(img, call), rec(r), x);
            }
        }
        let mut p = rec(0x9bd);
        for i in 0..3u32 {
            sound_step_3(g, img, c, p, i);
            p = p.wrapping_add(0xcc);
        }
        half(g, img, c, 0xaee)
    };
    if n.cast_signed() >= 0xcd {
        let v = mul(fade, kf(g, img, c, 0xb06));
        for op in [0xb0eu32, 0xb16, 0xb1e] {
            g.write::<f32>(rec(op), v);
        }
    }
    let s = add(mul(fade, kf(g, img, c, 0xb2c)), kf(g, img, c, 0xb3a));
    let word = c.gw(g, img, 0xb31) << 8;
    g.write::<f32>(local(c, 0x18), s);
    let r = mul(s, kf(g, img, c, 0xb4d));
    placed(g, img, c, 0xb52, r, (word, 0));
    let count = c.gw(g, img, 0xb58).cast_signed();
    let mut p = rec(0xb68);
    for _ in 0..count.max(0) {
        let v = mul(g.read::<f32>(p + 0x24), s);
        g.write::<f32>(p + 0x28, v);
        p = p.wrapping_add(0xcc);
    }
    if n.cast_signed() >= 0x140 {
        c.stores(g, img, 0xc18, 0xc22);
    }
}

/// One of state 3's model sounds (`p` the record's `+0x10`, `i` 0..3):
/// the kinds 0xc1 (the last two) or 0xc3 (the first) wait for their
/// frame (0x1c, 0x14, 0x17) and move on at their limit; from then (and
/// with the next kind) the sparks run and the model's height `p[0x30]`
/// drops by `K`.
fn sound_step_3<G: Game>(g: &mut G, img: &Image, c: &Scene, p: u32, i: u32) {
    let r = p.wrapping_sub(0x10);
    let kind = g.read::<u32>(p.wrapping_sub(8));
    let frame = g.read::<u32>(p);
    let lim = g.read::<u32>(p.wrapping_sub(4));
    // (the waiting kind, its first frame, the next kind, the sound's call,
    // the sparks' word and call, the drop)
    let (wait, first, next, call, mut word, sparks_call, drop) = match i {
        2 => (0xc1, 0x1c, 0xc2, 0x9f9, 0xa13, 0xa1f, 0xa2c),
        1 => (0xc1, 0x14, 0xc2, 0xa58, 0xa73, 0xa1f, 0xa2c),
        _ => (0xc3, 0x17, 0xc4, 0xa98, 0xab5, 0xaba, 0xac7),
    };
    if kind == wait {
        if frame.cast_signed() < first {
            return;
        }
        if frame == lim {
            crate::sa_misc::sound_start(g, img, c.target(img, call), r, next);
            if i == 1 {
                // (this way loads its word at `+0xa65`)
                word = 0xa65;
            }
        }
    } else if kind != next {
        return;
    }
    let speed = kf(g, img, c, 0x9b6);
    sparks(g, img, c, sparks_call, (r, c.w(img, word)), speed);
    let y = crate::sb_sse::sub(g.read::<f32>(p + 0x30), kf(g, img, c, drop));
    g.write::<f32>(p + 0x30, y);
}

/// [`spread_set`](crate::sa_misc::spread_set) (the call at `call`, one
/// word pushed) on the object at operand `o` with both spreads `s`.
fn spread<G: Game>(g: &mut G, img: &Image, c: &Scene, call: u32, o: u32, s: f32) {
    let mut fr = c.frame(img, call, 1);
    fr.ecx = c.w(img, o);
    fr.xmm[1] = u128::from(s.to_bits());
    fr.xmm[2] = u128::from(s.to_bits());
    crate::sa_misc::spread_set(g, &fr);
}

/// State 0x15 of `magic_0515` (`+0x2d69`): once (the halfword `G_O`),
/// slot 222 `(.., 0x1e, 0x20)`, the counted records registered and the
/// records cleared, the centre `(0, 0, 0, 1)`, the matrix turned, parts
/// P4, P6 and P1 stopped, the camera record started, part P1's motion,
/// the actors' slot 296, the parts placed. While not busy: at the count's
/// 0 slot 220 and slot 306 `(2)`, and (still 0) the closing curve from
/// slot 325's two points; from 0xf to 0x1d the spread falls, below 0x37
/// the flash is off, from 0x37 the spread by `n / 150` and (unless slot
/// 279 says 1) the next actor's slot 239 once its wait `G_W` ran out (a
/// random wait); 0xf parts P14 and P4 move, 0x37 part P6, 0x3d part P3
/// stopped and the actors' slot 296; on to state 0x17 at 0xcd.
#[allow(clippy::too_many_lines)] // the original's state, in order
fn state_15_0515<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    use crate::sb_sse::sub;
    let rec = |o: u32| c.w(img, o);
    if half(g, img, c, 0x2d6c) == 0 {
        let v = c.gw(g, img, 0x2d92);
        set_half(g, img, c, 0x2d99, 1);
        let eax = call_slot(g, img, c, (0x2d7d, 222), &[v, 0x1e, 0x20, 0x100]);
        crate::sa_misc::register_counted(g, img, c.target(img, 0x2da0), eax);
        crate::s4_misc::records_clear(g, img, &c.frame(img, 0x2da5, 0));
        c.stores(g, img, 0x2dab, 0x2dd3);
        crate::sa_misc::turn_matrix(g, img, c.target(img, 0x2dd4), 0);
        stop_part(g, img, c, (0x2dd9, 0x2de4), (0x2de9, 0x2dde));
        set_half(g, img, c, 0x2df9, 0);
        stop_part(g, img, c, (0x2dfe, 0x2e07), (0x2e0c, 0x2e11));
        set_half(g, img, c, 0x2e21, 0);
        stop_part(g, img, c, (0x2e26, 0x2e2f), (0x2e34, 0x2e39));
        set_half(g, img, c, 0x2e51, 0);
        let mut fr = c.frame(img, 0x2e56, 0);
        fr.ecx = rec(0x2e4b);
        crate::s6_misc::camera_tracks_from(g, img, &fr);
        let x3 = kf(g, img, c, 0x2e5e);
        let h = 0x3f00_0000;
        motion_words(
            g,
            img,
            c,
            0x2ea8,
            (rec(0x2e9c), 0x15),
            (0.0, x3),
            [0, 0, 0, 0, h, h, h],
        );
        let mut fr = c.frame(img, 0x2eb2, 0);
        fr.ecx = 1;
        crate::s4_misc::actors_296(g, img, &fr);
        let r = kf(g, img, c, 0x2eba);
        placed(g, img, c, 0x2ec3, r, (0, 0));
    }
    if c.busy(g, img, 0x2ecb) {
        return;
    }
    let mut n = half(g, img, c, 0x2eda);
    if n == 0 {
        let mut fr = c.frame(img, 0x2eef, 1);
        fr.ecx = c.gw(g, img, 0x2eea);
        crate::s4_misc::call_220(g, img, &fr);
        crate::sa_misc::slot306(g, img, c.target(img, 0x2ef9), 2);
        n = half(g, img, c, 0x2eff);
        if n == 0 {
            call_slot(g, img, c, (0x2f10, 325), &[rec(0x2f20), rec(0x2f15)]);
            call_slot(g, img, c, (0x2f27, 325), &[rec(0x2f37), rec(0x2f2c)]);
            let v = sub(kf(g, img, c, 0x2f41), kf(g, img, c, 0x2f49));
            g.write::<f32>(rec(0x2f91), v);
            let words = [
                2,
                0xf,
                0,
                0,
                rec(0x2f81),
                0x40b3_3333,
                0xbecc_cccd,
                0x3da3_d70a,
                0x4080_8ca0,
                0x0200_0000,
                4,
            ];
            spline(g, img, c, 0x2f96, (rec(0x2f65), rec(0x2f51)), words);
            n = half(g, img, c, 0x2f9c);
        }
    }
    tail_15(g, img, c, n);
}

/// Slot 307's random through operand `op` as state 0x17's sparks take it:
/// `((r - K) - K1) * K2` in doubles (operands `k`, `k1`, `k2`), rounded to
/// f32.
fn rand_d<G: Game>(
    g: &mut G,
    img: &Image,
    c: &Scene,
    (op, k, k1, k2): (u32, u32, u32, u32),
) -> f32 {
    let r = crate::sb_sse::sub(frand(g, img, c, op), kf(g, img, c, k));
    let k64 = |g: &G, o: u32| g.read::<f64>(c.w(img, o));
    #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
    let v = ((f64::from(r) - k64(g, k1)) * k64(g, k2)) as f32;
    v
}

/// State 0x17 of `magic_0515` (`+0x3202`): once (the halfword `G_O`), the
/// counted records registered, the records cleared, registered and
/// cleared, the models' slot 295, slot 222 `(.., 0x23, 0x24)`, parts P11,
/// P4, P3 and P6 stopped, slot 271 (1, 4, 0x10), slot 332; the centre
/// cleared, slot 310 on the sounds; each of the `G_N` actors (0xcc bytes
/// apart) widens the box of their places (min and max of x and z), keeps
/// its place, takes slot 290's scale and slot 277's size (the size times
/// the scale's y and `K`), its actor slot 296 `(id, 1)` and slot 211
/// `(model, 0)`; the centre is the box's middle at the first actor's
/// height (slot 289), the matrix turned by its angle; with the flags
/// `G_A`/`G_B` slot 278 `(*G_M, 4..=7, 0)` and slot 211 `(*G_M, 0)`, with
/// `G_C`/`G_D` the records' slot 296 `(.., 1)`; the camera record by the
/// flags. While not busy: below 0x3c sixteen sparks from free records
/// (nine randoms each); at 0x1e slot 926/925 clears the actor's byte
/// `+0x614`, slot 239 with -5; on to state 0x18 at 0x5a.
#[allow(clippy::too_many_lines)] // the original's state, in order
fn state_17_0515<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    use crate::sb_sse::{add, mul, sub};
    let rec = |o: u32| c.w(img, o);
    if half(g, img, c, 0x3205) == 0 {
        set_half(g, img, c, 0x3217, 1);
        crate::sa_misc::register_counted(g, img, c.target(img, 0x321c), 1);
        crate::s4_misc::records_clear(g, img, &c.frame(img, 0x3221, 0));
        crate::sa_misc::register_records(g, img, c.target(img, 0x3226));
        let eax = crate::sa_misc::records_clear(g, img, c.target(img, 0x322b));
        let fr = Frame {
            eax,
            ..c.frame(img, 0x3230, 0)
        };
        crate::s4_misc::models_295(g, img, &fr);
        slot_222(g, img, c, (0x3235, 0x324a), 0x23, 0x24);
        for (h, l, s, p, o) in [
            (0x3251u32, 0x325cu32, 0x3261u32, 0x3256u32, 0x3271u32),
            (0x3276, 0x327f, 0x3284, 0x3289, 0x3299),
            (0x329e, 0x32a7, 0x32ac, 0x32b1, 0x32c1),
            (0x32c6, 0x32cf, 0x32d4, 0x32d9, 0x32e9),
        ] {
            stop_part(g, img, c, (h, l), (s, p));
            set_half(g, img, c, o, 0);
        }
        for (op, v) in [(0x32eeu32, 1u32), (0x3300, 4), (0x3312, 0x10)] {
            call_slot(g, img, c, (op, 271), &[v]);
        }
        let eax = call_slot(g, img, c, (0x3324, 332), &[rec(0x3332)]);
        let (k1, k2) = (kf(g, img, c, 0x333c), kf(g, img, c, 0x3350));
        let (mut min_x, mut max_x, mut min_z, mut max_z) = (k1, k2, k1, k2);
        c.stores(g, img, 0x3363, 0x3381);
        let fr = Frame {
            eax,
            ..c.frame(img, 0x3382, 0)
        };
        crate::s4_misc::repeat_310(g, img, &fr);
        let scale = local(c, 0xfc);
        let mut p = rec(0x3395);
        let mut i = 0i32;
        while i < c.gw(g, img, 0x3486).cast_signed() {
            let x = g.read::<f32>(p);
            if min_x > x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            let z = g.read::<f32>(p + 8);
            if min_z > z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
            let (a, b) = (g.read::<u64>(p), g.read::<u64>(p + 8));
            g.write::<u64>(p + 0x10, a);
            g.write::<u64>(p + 0x18, b);
            let id = g.read::<u32>(p.wrapping_sub(0x2c));
            call_slot(g, img, c, (0x33fc, 290), &[id, scale]);
            let m = g.read::<u32>(p.wrapping_sub(0x28));
            // call width: 1
            let size = crate::m0018::st0_f32(g.call(c.slot(g, img, 0x3411, 277), 0, &[m]).st0);
            let v = mul(mul(size, g.read::<f32>(scale + 4)), kf(g, img, c, 0x343f));
            let ang = g.read::<u16>(p.wrapping_sub(0x18));
            g.write::<u16>(p.wrapping_sub(0x16), ang);
            let id = g.read::<u32>(p.wrapping_sub(0x2c));
            g.write::<f32>(p.wrapping_sub(4), v);
            if id != 0xff {
                call_slot(g, img, c, (0x3459, 296), &[id, 1]);
            }
            let m = g.read::<u32>(p.wrapping_sub(0x28));
            call_slot(g, img, c, (0x3469, 211), &[m, 0]);
            i += 1;
            p = p.wrapping_add(0xcc);
        }
        let x = mul(add(max_x, min_x), kf(g, img, c, 0x34ab));
        let first = c.gw(g, img, 0x34a3);
        g.write::<f32>(rec(0x34b3), x);
        // call width: 1
        let h = crate::m0018::st0_f32(g.call(c.slot(g, img, 0x349d, 289), 0, &[first]).st0);
        g.write::<f32>(rec(0x34c1), h);
        let z = mul(add(max_z, min_z), kf(g, img, c, 0x34df));
        let a = i32::from(g.read::<i16>(rec(0x34d4))).cast_unsigned();
        c.stores(g, img, 0x34e3, 0x34ed);
        g.write::<f32>(rec(0x34f1), z);
        crate::sa_misc::turn_matrix(g, img, c.target(img, 0x34f6), a);
        let flag = |g: &mut G, ops: [(u32, u32); 4], last: (u32, u32)| {
            for (k, (op, m)) in (4u32..).zip(ops) {
                let x = c.gw(g, img, m);
                call_slot(g, img, c, (op, 278), &[x, k, 0]);
            }
            let x = c.gw(g, img, last.1);
            call_slot(g, img, c, (last.0, 211), &[x, 0]);
        };
        let r = if c.gw(g, img, 0x34fc) != 0 {
            flag(
                g,
                [
                    (0x3508, 0x3518),
                    (0x351f, 0x3532),
                    (0x3539, 0x354c),
                    (0x3553, 0x3566),
                ],
                (0x356d, 0x357e),
            );
            rec(0x3588)
        } else if c.gw(g, img, 0x3593) != 0 {
            flag(
                g,
                [
                    (0x359f, 0x35af),
                    (0x35b6, 0x35c9),
                    (0x35d0, 0x35e3),
                    (0x35ea, 0x35fd),
                ],
                (0x3604, 0x3615),
            );
            rec(0x361f)
        } else if c.gw(g, img, 0x3627) != 0 {
            g.write::<u32>(c.s.wrapping_sub(4), 1);
            crate::s4_small::records_296_with(g, img, &c.frame(img, 0x3631, 1));
            rec(0x3639)
        } else if c.gw(g, img, 0x3641) != 0 {
            g.write::<u32>(c.s.wrapping_sub(4), 1);
            crate::s4_small::records_296_with(g, img, &c.frame(img, 0x364b, 1));
            rec(0x3653)
        } else if c.gw(g, img, 0x365b) != 0 {
            rec(0x3661)
        } else {
            rec(0x3668)
        };
        let mut fr = c.frame(img, 0x366d, 0);
        fr.ecx = r;
        crate::s6_misc::camera_tracks_from(g, img, &fr);
    }
    if c.busy(g, img, 0x3672) {
        return;
    }
    let mut n = half(g, img, c, 0x3681);
    if n.cast_signed() < 0x3c {
        for _ in 0..0x10 {
            let p = crate::s4_misc::free_record(g, img, &c.frame(img, 0x3696, 0)).eax;
            if p == 0 {
                break;
            }
            g.write::<u32>(p + 0x48, 0);
            g.write::<u32>(p + 0x2c, 1);
            g.write::<u32>(p + 0x30, rec(0x36b8));
            g.write::<u32>(p + 0x28, rec(0x36bf));
            g.write::<u32>(p + 0x44, 0x2080_8080);
            g.write::<u32>(p + 0x34, 0x3ba3_d70a);
            g.write::<u32>(p + 0x38, 0x3a83_126f);
            g.write::<u32>(p + 0x3c, 0xb951_b717);
            let v = rand_d(g, img, c, (0x36aa, 0x36f5, 0x3705, 0x370d));
            g.write::<f32>(p, v);
            let v = rand_d(g, img, c, (0x36fa, 0x372f, 0x373f, 0x3747));
            g.write::<f32>(p + 4, v);
            let r = sub(frand(g, img, c, 0x3734), kf(g, img, c, 0x376a));
            g.write::<u32>(p + 0xc, 0x3f80_0000);
            let v = mul(add(r, kf(g, img, c, 0x377e)), kf(g, img, c, 0x3786));
            g.write::<f32>(p + 8, v);
            for (o, ops) in [
                (0x10u32, (0x376fu32, 0x37a5u32, 0x37b5u32, 0x37bdu32)),
                (0x14, (0x37aa, 0x37e0, 0x37f0, 0x37f8)),
                (0x18, (0x37e5, 0x381b, 0x382b, 0x3833)),
                (0x1c, (0x3820, 0x3856, 0x3861, 0x386e)),
            ] {
                let v = rand_d(g, img, c, ops);
                g.write::<f32>(p + o, v);
            }
            let v = mul(
                sub(frand(g, img, c, 0x3866), kf(g, img, c, 0x3891)),
                kf(g, img, c, 0x389e),
            );
            g.write::<f32>(p + 0x20, v);
            let v = rand_d(g, img, c, (0x3896, 0x38bd, 0x38c8, 0x38d0));
            g.write::<f32>(p + 0x24, v);
        }
        n = half(g, img, c, 0x38e7);
    }
    if n == 0x1e {
        let b = u32::from(g.read::<u8>(rec(0x38f4)));
        if call_slot(g, img, c, (0x38fa, 926), &[b]) != 0 {
            let b = u32::from(g.read::<u8>(rec(0x3910)));
            let q = call_slot(g, img, c, (0x3916, 925), &[b]);
            g.write::<u8>(q.wrapping_add(0x614), 0);
        }
        let mut fr = c.frame(img, 0x3933, 1);
        fr.edx = 0xffff_fffb;
        crate::s4_misc::call_239(g, img, &fr);
        n = half(g, img, c, 0x3939);
    }
    let n = n.wrapping_add(1);
    set_half(g, img, c, 0x3944, n);
    if n.cast_signed() >= 0x5a {
        c.stores(g, img, 0x3952, 0x395c);
    }
}

/// [`scene_task_0515`] (16333 bytes, 1 overlay).
pub const SCENE_TASK_0515: Sig = Sig {
    len: 16333,
    hash: 0x328e_b93d_e293_cbd2,
    calls: &[
        (0x1f3, Callee::Sig(&crate::sa_misc::SLOT306)),
        (0x1fd, Callee::Sig(&crate::s4_misc::ACTORS_296)),
        (0x202, Callee::Sig(&crate::sa_misc::REPEAT_309)),
        (0x20e, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x21a, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x226, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x22d, Callee::Sig(&crate::s4_small::RECORDS_296_WITH)),
        (0x23f, Callee::Sig(&crate::s6_misc::LIVE_PARTS_AXES)),
        (0x24f, Callee::Sig(&crate::s6_misc::LIVE_PARTS_AXES)),
        (0x25e, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x290, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x29a, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x2bf, Callee::Sig(&crate::sa_misc::RECORDS_295)),
        (0x2d8, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0x342, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x37b, Callee::Sig(&crate::sb_nodes::FLOAT_289)),
        (0x3f0, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x443, Callee::Sig(&crate::sa_misc::SPREAD_SET)),
        (0x474, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x541, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x54b, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x59a, Callee::Sig(&crate::sa_misc::BLOCK_331)),
        (0x5b4, Callee::Sig(&crate::sa_misc::RECORDS_295)),
        (0x5c0, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x5cc, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x5d8, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x5df, Callee::Sig(&crate::s4_misc::ACTORS_296)),
        (0x5ee, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x5fd, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x60c, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x629, Callee::Sig(&crate::s6_misc::PARTS_RING)),
        (0x63c, Callee::Sig(&crate::s8_misc::PARTS_PLACED)),
        (0x697, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x702, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0x85d, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x8d4, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x935, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x990, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x99f, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x9ae, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x9f9, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0xa1f, Callee::Sig(&crate::s6_misc::SPARKS_ON_MODEL)),
        (0xa58, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0xa98, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0xaba, Callee::Sig(&crate::s6_misc::SPARKS_ON_MODEL)),
        (0xb52, Callee::Sig(&crate::s8_misc::PARTS_PLACED)),
        (0xc6c, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0xc76, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0xc7b, Callee::Sig(&crate::sa_misc::REGISTER_RECORDS)),
        (0xc80, Callee::Sig(&crate::sa_misc::RECORDS_CLEAR)),
        (0xc87, Callee::Sig(&crate::s4_misc::ACTORS_296)),
        (0xd45, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0xd5d, Callee::Sig(&crate::sa_misc::REGISTER_RECORDS)),
        (0xd62, Callee::Sig(&crate::sa_misc::RECORDS_CLEAR)),
        (0xe12, Callee::Sig(&crate::s6_misc::SPARKS_ON_MODEL)),
        (0xe38, Callee::Sig(&crate::s6_misc::SPARKS_ON_MODEL)),
        (0xea5, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0xeaa, Callee::Sig(&crate::sa_misc::REGISTER_RECORDS)),
        (0xeaf, Callee::Sig(&crate::sa_misc::RECORDS_CLEAR)),
        (0xedc, Callee::Sig(&crate::s6_misc::PARTS_RING)),
        (0xf0d, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0xf55, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0xf7d, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0xfb0, Callee::Sig(&crate::s6_misc::SPARKS_ON_MODEL)),
        (0xfea, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x100a, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x103b, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x1082, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x10ae, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x10df, Callee::Sig(&crate::s6_misc::SPARKS_ON_MODEL)),
        (0x11b0, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x1222, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x129e, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x1353, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x13a7, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x13b9, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x13fb, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0x146e, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x148e, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x14b4, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x16b2, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0x1705, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x178e, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x17d5, Callee::Sig(&crate::s7_misc::SPLINE_OBJECT)),
        (0x1850, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x187e, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x188d, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x189c, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x18ac, Callee::Sig(&crate::sa_misc::REGISTER_COUNTED)),
        (0x18b1, Callee::Sig(&crate::s4_misc::RECORDS_CLEAR)),
        (0x18d0, Callee::Sig(&crate::s6_misc::PARTS_RING)),
        (0x1905, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x1991, Callee::Sig(&crate::s7_misc::SPLINE_OBJECT)),
        (0x1a16, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x1a6c, Callee::Sig(&crate::sa_misc::REGISTER_COUNTED)),
        (0x1a71, Callee::Sig(&crate::s4_misc::RECORDS_CLEAR)),
        (0x1a82, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x1aa1, Callee::Sig(&crate::s6_misc::PARTS_RING)),
        (0x1afe, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x1c3b, Callee::Sig(&crate::s7_misc::SPLINE_OBJECT)),
        (0x1da6, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x1e0b, Callee::Import("_CIatan2")),
        (0x24f8, Callee::Sig(&crate::sa_misc::RECORD_FREE)),
        (0x254a, Callee::Sig(&crate::s7_misc::SPLINE_OBJECT)),
        (0x28d5, Callee::Sig(&crate::s6_misc::PARTS_RING)),
        (0x2aae, Callee::Sig(&crate::s4_misc::REGISTER_PARTS_B)),
        (0x2b35, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x2bd2, Callee::Sig(&crate::sa_misc::REGISTER_COUNTED)),
        (0x2c90, Callee::Sig(&crate::s7_misc::SPLINE_OBJECT)),
        (0x2ce0, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x2da0, Callee::Sig(&crate::sa_misc::REGISTER_COUNTED)),
        (0x2da5, Callee::Sig(&crate::s4_misc::RECORDS_CLEAR)),
        (0x2dd4, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x2e56, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x2ea8, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x2eb2, Callee::Sig(&crate::s4_misc::ACTORS_296)),
        (0x2ec3, Callee::Sig(&crate::s8_misc::PARTS_PLACED)),
        (0x2eef, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0x2ef9, Callee::Sig(&crate::sa_misc::SLOT306)),
        (0x2f96, Callee::Sig(&crate::s7_misc::SPLINE_OBJECT)),
        (0x2fec, Callee::Sig(&crate::sa_misc::SPREAD_SET)),
        (0x3030, Callee::Sig(&crate::sa_misc::SPREAD_SET)),
        (0x3062, Callee::Sig(&crate::s4_misc::FLOAT_307)),
        (0x30b1, Callee::Sig(&crate::s4_misc::CALL_239)),
        (0x3132, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x3195, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x31d3, Callee::Sig(&crate::s4_misc::ACTORS_296)),
        (0x321c, Callee::Sig(&crate::sa_misc::REGISTER_COUNTED)),
        (0x3221, Callee::Sig(&crate::s4_misc::RECORDS_CLEAR)),
        (0x3226, Callee::Sig(&crate::sa_misc::REGISTER_RECORDS)),
        (0x322b, Callee::Sig(&crate::sa_misc::RECORDS_CLEAR)),
        (0x3230, Callee::Sig(&crate::s4_misc::MODELS_295)),
        (0x3382, Callee::Sig(&crate::s4_misc::REPEAT_310)),
        (0x34f6, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x3631, Callee::Sig(&crate::s4_small::RECORDS_296_WITH)),
        (0x364b, Callee::Sig(&crate::s4_small::RECORDS_296_WITH)),
        (0x366d, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x3696, Callee::Sig(&crate::s4_misc::FREE_RECORD)),
        (0x3933, Callee::Sig(&crate::s4_misc::CALL_239)),
        (0x397d, Callee::Sig(&crate::s4_misc::ACTORS_296)),
        (0x3aa0, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x3aac, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x3ab6, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x3ada, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0x3ae4, Callee::Sig(&crate::sa_misc::SLOT306)),
        (0x3b2a, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x3b37, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x3c34, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x3c56, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x3ce4, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x3cf1, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x3d69, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x3e1b, Callee::Sig(&crate::sa_misc::PLACE_RECORDS)),
        (0x3e2b, Callee::Sig(&crate::sa_misc::PLACE_RECORDS)),
        (0x3e45, Callee::Sig(&crate::s4_misc::MODELS_276)),
        (0x3eeb, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS)),
        (0x3efa, Callee::Sig(&crate::s6_misc::PARTS_AXES)),
        (0x3f0a, Callee::Sig(&crate::s6_misc::PARTS_AXES)),
        (0x3f18, Callee::Sig(&crate::r36::CAMERAS_0515)),
        (0x3f2a, Callee::Any), // __security_check_cookie (CRT)
    ],
    imports: &[],
};

/// State 0x15 from the count's reading (`+0x2fa3` in `magic_0515`): the
/// spread falls over 0xf frames from `lo`, below `mid` the flash is off,
/// from `mid` the spread by `n / 150` and (unless slot 279 says 1) the
/// next actor's slot 239 once its wait `G_W` ran out (a random wait); at
/// `burst` parts P14 and P4 move, at `part` part P6, at `stop` part P3
/// stops and the actors' slot 296; on to state 0x17 at `next`. The
/// thresholds are the copy's immediates.
#[allow(clippy::too_many_lines)] // the original's state, in order
fn tail_15<G: Game>(g: &mut G, img: &Image, c: &Scene, n: u16) {
    use crate::sb_sse::{add, div, mul, sub};
    let rec = |o: u32| c.w(img, o);
    // the imm8 of `cmp ax, imm8` (66 83 f8 ib), sign-extended
    let imm8 = |o: u32| i32::from(((c.w(img, o) & 0xff) as u8).cast_signed());
    // lea ecx, [edx - lo]; cmp ax, mid; .. cmp ax, burst; cmp ax, part;
    // cmp ax, stop; mov ecx, next
    let lo = -imm8(0x2fa8);
    let mid = imm8(0x2ffb);
    // (cmp ax, imm8 compares the halfword sign-extended)
    let half_of = |x: i32| x.cast_unsigned() as u16;
    let (burst, part, stop) = (
        half_of(imm8(0x30df)),
        half_of(imm8(0x3145)),
        half_of(imm8(0x31a5)),
    );
    let next = c.w(img, 0x31e0).cast_signed();
    let mut n = n;
    let d = i32::from(n.cast_signed());
    let e = d - lo;
    if e.cast_unsigned() < 0xf {
        // cvtdq2pd and the unsigned fix-up word (0 for these)
        let fix = g.read::<f64>(rec(0x2fc6).wrapping_add(8 * (e.cast_unsigned() >> 31)));
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let x = (f64::from(e) + fix) as f32;
        let s = mul(
            sub(kf(g, img, c, 0x2fbd), div(x, kf(g, img, c, 0x2fd8))),
            kf(g, img, c, 0x2fe4),
        );
        spread(g, img, c, 0x2fec, 0x2fcc, s);
    } else if i32::from(n.cast_signed()) < mid {
        c.stores(g, img, 0x30d5, 0x30dc);
    } else {
        let q = div_magic(d, 0x1b4e_81b5, false, 4);
        #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
        let s = sub(kf(g, img, c, 0x3006), q as f32);
        spread(g, img, c, 0x3030, 0x3028, s);
        let busy = call_slot(g, img, c, (0x3035, 279), &[]);
        if busy != 1 {
            let wait = half(g, img, c, 0x304f);
            if wait == 0 && half(g, img, c, 0x305b).cast_signed() > 0 {
                let r = crate::s4_misc::float_307(g, img, &c.frame(img, 0x3062, 0));
                let x = match r.xmm0 {
                    Some(crate::Xmm::F32(b)) => f32::from_bits(b),
                    _ => 0.0,
                };
                let count = c.gw(g, img, 0x3070).cast_signed();
                let x = add(
                    mul(sub(x, kf(g, img, c, 0x306a)), kf(g, img, c, 0x307f)),
                    kf(g, img, c, 0x3088),
                );
                #[allow(clippy::cast_precision_loss)] // cvtdq2ps: rounded as the original rounds
                let t = div(x, count as f32);
                set_half(g, img, c, 0x3096, trunc(f64::from(t)) as u16);
                let k = i32::from(g.read::<i16>(rec(0x309d)));
                let actor = g.read::<u32>(
                    rec(0x30ac)
                        .wrapping_add(k.wrapping_rem(count).wrapping_mul(0xcc).cast_unsigned()),
                );
                let mut fr = c.frame(img, 0x30b1, 1);
                fr.edx = actor;
                crate::s4_misc::call_239(g, img, &fr);
                let w = half(g, img, c, 0x30b7);
                let at = rec(0x30c1);
                let v = g.read::<u16>(at).wrapping_sub(1);
                g.write::<u16>(at, v);
                set_half(g, img, c, 0x30c9, w.wrapping_sub(1));
            } else {
                set_half(g, img, c, 0x30c9, wait.wrapping_sub(1));
            }
        }
        n = half(g, img, c, 0x30cf);
    }
    let two = 0x4000_0000;
    if n == burst {
        let x3 = kf(g, img, c, 0x30e6);
        motion_words(
            g,
            img,
            c,
            0x3132,
            (rec(0x3126), 8),
            (0.0, x3),
            [0, 0, 0, 0, two, two, two],
        );
        let x3 = kf(g, img, c, 0x3156);
        motion_words(
            g,
            img,
            c,
            0x3195,
            (rec(0x313c), 0x17),
            (0.0, x3),
            [0, 0, 0, 0, two, two, two],
        );
        n = half(g, img, c, 0x319b);
    } else if n == part {
        let x3 = kf(g, img, c, 0x3156);
        motion_words(
            g,
            img,
            c,
            0x3195,
            (rec(0x314e), 1),
            (0.0, x3),
            [0, 0, 0, 0, two, two, two],
        );
        n = half(g, img, c, 0x319b);
    }
    if n == stop {
        stop_part(g, img, c, (0x31a9, 0x31b4), (0x31b9, 0x31ae));
        set_half(g, img, c, 0x31ce, 0);
        crate::s4_misc::actors_296(g, img, &c.frame(img, 0x31d3, 0));
        n = half(g, img, c, 0x31d9);
    }
    let n = n.wrapping_add(1);
    set_half(g, img, c, 0x31e6, n);
    if i32::from(n.cast_signed()) >= next {
        c.stores(g, img, 0x31f3, 0x31fd);
    }
}

/// State 0 of `magic_0611` (`+0x15c`): once (the halfword `G_O`), the
/// camera record cleared, the actors' slot 296, slot 222 `(.., 0, 0x1d)`,
/// with either flag `G_A`/`G_B` slot 278 `(*G_M, 4..=7, 1)`. While not
/// busy the count steps; at 0x14 slot 267 (1, 4, 0x10), the block's slot
/// 331, slot 332, slot 309 on the sounds, the records' slot 296 with 0 and
/// state 0x15 starts.
fn state_0_0611<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    let rec = |o: u32| c.w(img, o);
    if half(g, img, c, 0x15f) == 0 {
        set_half(g, img, c, 0x173, 1);
        c.stores(g, img, 0x177, 0x181);
        crate::s4_misc::actors_296(g, img, &c.frame(img, 0x182, 0));
        slot_222(g, img, c, (0x187, 0x19c), 0, 0x1d);
        if c.gw(g, img, 0x1a7) != 0 || c.gw(g, img, 0x1b0) != 0 {
            for (op, m, k) in [
                (0x1b8u32, 0x1c8u32, 4u32),
                (0x1cf, 0x1e2, 5),
                (0x1e9, 0x1fc, 6),
                (0x203, 0x216, 7),
            ] {
                let x = c.gw(g, img, m);
                call_slot(g, img, c, (op, 278), &[x, k, 1]);
            }
        }
    }
    if c.busy(g, img, 0x220) {
        return;
    }
    let n = half(g, img, c, 0x22f).wrapping_add(1);
    set_half(g, img, c, 0x237, n);
    if n.cast_signed() < 0x14 {
        return;
    }
    for (op, v) in [(0x246u32, 1u32), (0x255, 4), (0x267, 0x10)] {
        call_slot(g, img, c, (op, 267), &[v]);
    }
    crate::sa_misc::block_331(g, img, c.target(img, 0x27c));
    let eax = call_slot(g, img, c, (0x281, 332), &[rec(0x286)]);
    crate::sa_misc::repeat_309(g, img, c.target(img, 0x296), eax);
    crate::sa_misc::record_296(g, img, c.target(img, 0x2a2), rec(0x29d), 0);
    g.write::<u32>(c.s.wrapping_sub(4), 0);
    crate::s4_small::records_296_with(g, img, &c.frame(img, 0x2a9, 1));
    for (call, r) in [(0x2b8u32, 0x2b3u32), (0x2c4, 0x2bf), (0x2d0, 0x2cb)] {
        crate::sa_misc::record_296(g, img, c.target(img, call), rec(r), 0);
    }
    c.stores(g, img, 0x2d4, 0x2de);
}

/// The records' sizes: each of the `G_N` records (0xcc bytes apart from
/// the immediate at `base`) `+0x28 = +0x24 * s`.
fn sizes<G: Game>(g: &mut G, img: &Image, c: &Scene, (count, base): (u32, u32), s: f32) {
    let n = c.gw(g, img, count).cast_signed();
    let mut p = c.w(img, base);
    for _ in 0..n.max(0) {
        let v = crate::sb_sse::mul(g.read::<f32>(p + 0x24), s);
        g.write::<f32>(p + 0x28, v);
        p = p.wrapping_add(0xcc);
    }
}

/// State 0x15's once-block in `magic_0611` (`+0x2bb9`): slot 222 `(..,
/// 0x1e, 0x20)`, the counted records registered and the records cleared,
/// the centre `(0, 0, 0, 1)`, the matrix turned, the records' sizes by
/// `K`, both part groups' live axes, part P11's motion, the camera record
/// started, part P1's motion, the actors' slot 296, the parts placed.
fn once_15_0611<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    let rec = |o: u32| c.w(img, o);
    let v = c.gw(g, img, 0x2be2);
    set_half(g, img, c, 0x2be9, 1);
    let eax = call_slot(g, img, c, (0x2bcd, 222), &[v, 0x1e, 0x20, 0x100]);
    crate::sa_misc::register_counted(g, img, c.target(img, 0x2bf3), eax);
    crate::s4_misc::records_clear(g, img, &c.frame(img, 0x2bf8, 0));
    c.stores(g, img, 0x2bfe, 0x2c26);
    crate::sa_misc::turn_matrix(g, img, c.target(img, 0x2c27), 0);
    let s = kf(g, img, c, 0x2c35);
    sizes(g, img, c, (0x2c2d, 0x2c3c), s);
    let axes = |g: &mut G, call: u32, ecx: u32, edx: u32| {
        let fr = Frame {
            ecx,
            edx,
            ..c.frame(img, call, 0)
        };
        crate::s6_misc::live_parts_axes(g, img, &fr);
    };
    axes(g, 0x2ce5, rec(0x2ce0), 3);
    let n = c.gw(g, img, 0x2ceb);
    axes(g, 0x2cf5, rec(0x2cf0), n);
    let a = c.gw(g, img, 0x2cfd);
    let (x3, x2) = (kf(g, img, c, 0x2d05), kf(g, img, c, 0x2d0d));
    let k = 0x3d50_e560;
    motion_words(
        g,
        img,
        c,
        0x2d51,
        (rec(0x2d47), 0),
        (x2, x3),
        [a, 0, 0x45aa_a800, 0, k, k, k],
    );
    let mut fr = c.frame(img, 0x2d5e, 0);
    fr.ecx = rec(0x2d59);
    crate::s6_misc::camera_tracks_from(g, img, &fr);
    let x3 = kf(g, img, c, 0x2d66);
    let h = 0x3f00_0000;
    motion_words(
        g,
        img,
        c,
        0x2db2,
        (rec(0x2da6), 0x15),
        (0.0, x3),
        [0, 0, 0, 0, h, h, h],
    );
    let mut fr = c.frame(img, 0x2dbc, 0);
    fr.ecx = 1;
    crate::s4_misc::actors_296(g, img, &fr);
    let r = kf(g, img, c, 0x2dc4);
    placed(g, img, c, 0x2dcd, r, (0, 0));
}

/// State 0x15 of `magic_0611` (`+0x2bb9`): once, [`once_15_0611`]; while
/// not busy slot 306 `(2)` at the count's 0, slot 220 at 5, the closing
/// curve at 0x1e; then [`tail_15`] with this copy's thresholds.
fn state_15_0611<G: Game>(g: &mut G, img: &Image, c: &Scene) {
    use crate::sb_sse::sub;
    let rec = |o: u32| c.w(img, o);
    if half(g, img, c, 0x2bbc) == 0 {
        once_15_0611(g, img, c);
    }
    if c.busy(g, img, 0x2dd5) {
        return;
    }
    let mut n = half(g, img, c, 0x2de4);
    if n == 0 {
        crate::sa_misc::slot306(g, img, c.target(img, 0x2df4), 2);
        n = half(g, img, c, 0x2dfa);
    }
    if n == 5 {
        let mut fr = c.frame(img, 0x2e0f, 1);
        fr.ecx = c.gw(g, img, 0x2e0a);
        crate::s4_misc::call_220(g, img, &fr);
        n = half(g, img, c, 0x2e15);
    }
    if n == 0x1e {
        call_slot(g, img, c, (0x2e27, 325), &[rec(0x2e37), rec(0x2e2c)]);
        call_slot(g, img, c, (0x2e3e, 325), &[rec(0x2e4e), rec(0x2e43)]);
        let v = sub(kf(g, img, c, 0x2e58), kf(g, img, c, 0x2e60));
        g.write::<f32>(rec(0x2ea8), v);
        let words = [
            2,
            0xf,
            0,
            0,
            rec(0x2e98),
            0x40b3_3333,
            0xbecc_cccd,
            0x3da3_d70a,
            0x4080_8ca0,
            0x0200_0000,
            4,
        ];
        spline(g, img, c, 0x2ead, (rec(0x2e7c), rec(0x2e68)), words);
        n = half(g, img, c, 0x2eb3);
    }
    let tail = c.with(&MAP_15_0611_TAIL);
    tail_15(g, img, &tail, n);
}

/// `magic_0611`'s state 3 against `magic_0515`'s.
static MAP_3_0611: [(u32, u32); 10] = [
    (0x4c3, 0x319),
    (0x564, 0x3af),
    (0x577, 0x3c2),
    (0x57a, 0x3c5),
    (0x62d, 0x480),
    (0x632, 0x478),
    (0x63a, 0x485),
    (0x760, 0x5a6),
    (0x9c1, 0x810),
    (0xb80, 0x9d0),
];
/// `magic_0611`'s state 0x15 from the count's reading.
static MAP_15_0611_TAIL: [(u32, u32); 1] = [(0x2fa3, 0x2eba)];
/// `magic_0611`'s state 0x17.
static MAP_17_0611: [(u32, u32); 2] = [(0x3202, 0x3119), (0x33a0, 0x32b0)];
/// `magic_0611`'s state 0x64.
static MAP_64_0611: [(u32, u32); 2] = [(0x3daf, 0x3cbf), (0x3e44, 0x3d49)];
/// `magic_0611`'s end.
static MAP_END_0611: [(u32, u32); 2] = [(0x3e5f, 0x3d64), (0x3e72, 0x3d80)];
/// `magic_0611`'s shared states (one shift each): 1, 6, 8, 9, 0xd, 0x10,
/// 0x18.
static SHIFT_0611: [(u32, [(u32, u32); 1]); 7] = [
    (1, [(0x48d, 0x2e3)]),
    (6, [(0xc27, 0xa77)]),
    (8, [(0xe8a, 0xcda)]),
    (9, [(0x130c, 0x115c)]),
    (0xd, [(0x1835, 0x1685)]),
    (0x10, [(0x1a52, 0x18a2)]),
    (0x18, [(0x3961, 0x3871)]),
];

/// The scene task of `magic_0611` (0x10004d10, 16089 bytes): the same
/// prologue as [`scene_task_0515`]; its own states 0 ([`state_0_0611`])
/// and 0x15 ([`state_15_0611`]); `magic_0515`'s states 1, 6, 8, 9, 0xd,
/// 0x10 and 0x18 shifted, 3 and 0x64 through [`call_276`] instead of the
/// slot, 0x17 and the end with their operands moved
/// ([`MAP_3_0611`] ..).
///
/// [`call_276`]: crate::s4_small::call_276
pub fn scene_task_0611<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let a = (fr.esp.wrapping_sub(4) & !7).wrapping_sub(0x108);
    let c = Scene {
        f: fr.at,
        s: a.wrapping_sub(8),
        map: &[],
        alt: true,
    };
    if let Exit::Out = prologue(g, img, &c) {
        return Ret::eax(0);
    }
    let state = c.gw(g, img, 0x141);
    let shifted = SHIFT_0611
        .iter()
        .find(|(s, _)| *s == state)
        .map(|(_, m)| c.with(m));
    match (state, shifted) {
        (0, _) => state_0_0611(g, img, &c),
        (1, Some(s)) => state_1(g, img, &s),
        (3, _) => state_3_0515(g, img, &c.with(&MAP_3_0611)),
        (6, Some(s)) => state_6(g, img, &s),
        (8, Some(s)) => state_8(g, img, &s),
        (9, Some(s)) => state_9(g, img, &s),
        (0xd, Some(s)) => state_d(g, img, &s),
        (0x10, Some(s)) => state_10(g, img, &s),
        (0x15, _) => state_15_0611(g, img, &c),
        (0x17, _) => state_17_0515(g, img, &c.with(&MAP_17_0611)),
        (0x18, Some(s)) => state_18(g, img, &s),
        (0x64, _) => state_64(g, img, &c.with(&MAP_64_0611)),
        _ => {}
    }
    end(g, img, &c.with(&MAP_END_0611));
    Ret::eax(0)
}

/// [`scene_task_0611`] (16089 bytes, 1 overlay).
pub const SCENE_TASK_0611: Sig = Sig {
    len: 16089,
    hash: 0x0738_5daa_0974_d642,
    calls: &[
        (0x182, Callee::Sig(&crate::s4_misc::ACTORS_296)),
        (0x27c, Callee::Sig(&crate::sa_misc::BLOCK_331)),
        (0x296, Callee::Sig(&crate::sa_misc::REPEAT_309)),
        (0x2a2, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x2a9, Callee::Sig(&crate::s4_small::RECORDS_296_WITH)),
        (0x2b8, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x2c4, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x2d0, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x397, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x3a1, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x3ab, Callee::Sig(&crate::s4_small::CALL_276)),
        (0x3e5, Callee::Sig(&crate::sa_misc::BLOCK_331)),
        (0x3ff, Callee::Sig(&crate::sa_misc::RECORDS_295)),
        (0x40b, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x417, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x423, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x42a, Callee::Sig(&crate::s4_misc::ACTORS_296)),
        (0x439, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x448, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x457, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x474, Callee::Sig(&crate::s6_misc::PARTS_RING)),
        (0x487, Callee::Sig(&crate::s8_misc::PARTS_PLACED)),
        (0x4e2, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x54d, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0x6a3, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x71a, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x77b, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x7d6, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x7e5, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x7f4, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x848, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x86e, Callee::Sig(&crate::s6_misc::SPARKS_ON_MODEL)),
        (0x8a7, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x8e7, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x909, Callee::Sig(&crate::s6_misc::SPARKS_ON_MODEL)),
        (0x9a1, Callee::Sig(&crate::s8_misc::PARTS_PLACED)),
        (0xabc, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0xac6, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0xacb, Callee::Sig(&crate::sa_misc::REGISTER_RECORDS)),
        (0xad0, Callee::Sig(&crate::sa_misc::RECORDS_CLEAR)),
        (0xad7, Callee::Sig(&crate::s4_misc::ACTORS_296)),
        (0xb95, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0xbad, Callee::Sig(&crate::sa_misc::REGISTER_RECORDS)),
        (0xbb2, Callee::Sig(&crate::sa_misc::RECORDS_CLEAR)),
        (0xc62, Callee::Sig(&crate::s6_misc::SPARKS_ON_MODEL)),
        (0xc88, Callee::Sig(&crate::s6_misc::SPARKS_ON_MODEL)),
        (0xcf5, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0xcfa, Callee::Sig(&crate::sa_misc::REGISTER_RECORDS)),
        (0xcff, Callee::Sig(&crate::sa_misc::RECORDS_CLEAR)),
        (0xd2c, Callee::Sig(&crate::s6_misc::PARTS_RING)),
        (0xd5d, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0xda5, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0xdcd, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0xe00, Callee::Sig(&crate::s6_misc::SPARKS_ON_MODEL)),
        (0xe3a, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0xe5a, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0xe8b, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0xed2, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0xefe, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0xf2f, Callee::Sig(&crate::s6_misc::SPARKS_ON_MODEL)),
        (0x1000, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x1072, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x10ee, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x11a3, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x11f7, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x1209, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x124b, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0x12be, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x12de, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x1304, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x1502, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0x1555, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x15de, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x1625, Callee::Sig(&crate::s7_misc::SPLINE_OBJECT)),
        (0x16a0, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x16ce, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x16dd, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x16ec, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x16fc, Callee::Sig(&crate::sa_misc::REGISTER_COUNTED)),
        (0x1701, Callee::Sig(&crate::s4_misc::RECORDS_CLEAR)),
        (0x1720, Callee::Sig(&crate::s6_misc::PARTS_RING)),
        (0x1755, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x17e1, Callee::Sig(&crate::s7_misc::SPLINE_OBJECT)),
        (0x1866, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x18bc, Callee::Sig(&crate::sa_misc::REGISTER_COUNTED)),
        (0x18c1, Callee::Sig(&crate::s4_misc::RECORDS_CLEAR)),
        (0x18d2, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x18f1, Callee::Sig(&crate::s6_misc::PARTS_RING)),
        (0x194e, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x1a8b, Callee::Sig(&crate::s7_misc::SPLINE_OBJECT)),
        (0x1bf6, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x1c5b, Callee::Import("_CIatan2")),
        (0x2348, Callee::Sig(&crate::sa_misc::RECORD_FREE)),
        (0x239a, Callee::Sig(&crate::s7_misc::SPLINE_OBJECT)),
        (0x2725, Callee::Sig(&crate::s6_misc::PARTS_RING)),
        (0x28fe, Callee::Sig(&crate::s4_misc::REGISTER_PARTS_B)),
        (0x2985, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x2a22, Callee::Sig(&crate::sa_misc::REGISTER_COUNTED)),
        (0x2ae0, Callee::Sig(&crate::s7_misc::SPLINE_OBJECT)),
        (0x2b30, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x2bf3, Callee::Sig(&crate::sa_misc::REGISTER_COUNTED)),
        (0x2bf8, Callee::Sig(&crate::s4_misc::RECORDS_CLEAR)),
        (0x2c27, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x2ce5, Callee::Sig(&crate::s6_misc::LIVE_PARTS_AXES)),
        (0x2cf5, Callee::Sig(&crate::s6_misc::LIVE_PARTS_AXES)),
        (0x2d51, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x2d5e, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x2db2, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x2dbc, Callee::Sig(&crate::s4_misc::ACTORS_296)),
        (0x2dcd, Callee::Sig(&crate::s8_misc::PARTS_PLACED)),
        (0x2df4, Callee::Sig(&crate::sa_misc::SLOT306)),
        (0x2e0f, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0x2ead, Callee::Sig(&crate::s7_misc::SPLINE_OBJECT)),
        (0x2f03, Callee::Sig(&crate::sa_misc::SPREAD_SET)),
        (0x2f47, Callee::Sig(&crate::sa_misc::SPREAD_SET)),
        (0x2f79, Callee::Sig(&crate::s4_misc::FLOAT_307)),
        (0x2fc8, Callee::Sig(&crate::s4_misc::CALL_239)),
        (0x3049, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x30ac, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x30ea, Callee::Sig(&crate::s4_misc::ACTORS_296)),
        (0x3133, Callee::Sig(&crate::sa_misc::REGISTER_COUNTED)),
        (0x3138, Callee::Sig(&crate::s4_misc::RECORDS_CLEAR)),
        (0x313d, Callee::Sig(&crate::sa_misc::REGISTER_RECORDS)),
        (0x3142, Callee::Sig(&crate::sa_misc::RECORDS_CLEAR)),
        (0x3147, Callee::Sig(&crate::s4_misc::MODELS_295)),
        (0x3299, Callee::Sig(&crate::s4_misc::REPEAT_310)),
        (0x3406, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x3541, Callee::Sig(&crate::s4_small::RECORDS_296_WITH)),
        (0x355b, Callee::Sig(&crate::s4_small::RECORDS_296_WITH)),
        (0x357d, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x35a6, Callee::Sig(&crate::s4_misc::FREE_RECORD)),
        (0x3843, Callee::Sig(&crate::s4_misc::CALL_239)),
        (0x388d, Callee::Sig(&crate::s4_misc::ACTORS_296)),
        (0x39b0, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x39bc, Callee::Sig(&crate::sa_misc::TURN_MATRIX)),
        (0x39c6, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS_FROM)),
        (0x39ea, Callee::Sig(&crate::s4_misc::CALL_220)),
        (0x39f4, Callee::Sig(&crate::sa_misc::SLOT306)),
        (0x3a3a, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x3a47, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x3b44, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x3b66, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x3bf4, Callee::Sig(&crate::sa_misc::RECORD_296)),
        (0x3c01, Callee::Sig(&crate::sa_misc::SOUND_START)),
        (0x3c79, Callee::Sig(&crate::s6_misc::PART_MOTION)),
        (0x3d2b, Callee::Sig(&crate::sa_misc::PLACE_RECORDS)),
        (0x3d3b, Callee::Sig(&crate::sa_misc::PLACE_RECORDS)),
        (0x3d45, Callee::Sig(&crate::s4_small::CALL_276)),
        (0x3d4a, Callee::Sig(&crate::s4_misc::MODELS_276)),
        (0x3df9, Callee::Sig(&crate::s6_misc::CAMERA_TRACKS)),
        (0x3e08, Callee::Sig(&crate::s6_misc::PARTS_AXES)),
        (0x3e18, Callee::Sig(&crate::s6_misc::PARTS_AXES)),
        (0x3e26, Callee::Sig(&crate::r36::CAMERAS_0515)),
        (0x3e38, Callee::Any), // __security_check_cookie (CRT)
    ],
    imports: &[],
};
