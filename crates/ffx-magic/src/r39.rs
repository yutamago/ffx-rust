//! Round 39's ports: `magic_0519`'s scene task (0x10002d60).

use crate::runtime::slot_target;
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// [`scene_task_0519`]'s code offset and entry `esp`.
struct Ctx0519 {
    f: u32,
    /// `esp` at a callee's return address with nothing pushed (`E - 0x1c`:
    /// `ebp`, 0x10 bytes of locals and `esi` below the entry).
    ce: u32,
}

impl Ctx0519 {
    fn w(&self, img: &Image, o: u32) -> u32 {
        img.word(self.f + o)
    }

    /// The global at operand `o`.
    fn gw<G: Game>(&self, g: &G, img: &Image, o: u32) -> u32 {
        g.read::<u32>(self.w(img, o))
    }

    /// `inc DWORD PTR [G]` (operand `o`); returns the new value.
    fn inc<G: Game>(&self, g: &mut G, img: &Image, o: u32) -> u32 {
        let v = self.gw(g, img, o).wrapping_add(1);
        g.write::<u32>(self.w(img, o), v);
        v
    }

    fn target(&self, img: &Image, o: u32) -> u32 {
        img.rel32_target(self.f + o)
    }

    /// Slot `n` through the imports pointer the instruction at operand
    /// `op` reads.
    fn slot<G: Game>(&self, g: &G, img: &Image, op: u32, n: u32) -> u32 {
        slot_target(g, self.w(img, op), n)
    }

    /// The game's busy word: `*[G_B]` (operand `+0x108`) non-zero.
    fn busy<G: Game>(&self, g: &G, img: &Image) -> bool {
        g.read::<u32>(self.gw(g, img, 0x108)) != 0
    }

    /// The callee frame `esp - 4 * pushed` with nothing else on it.
    fn frame(&self, img: &Image, call: u32, pushed: u32) -> Frame {
        Frame {
            at: self.target(img, call),
            esp: self.ce.wrapping_sub(4 * pushed),
            ..Frame::default()
        }
    }

    /// One particle part started (a block of the task; `P` the part,
    /// operand `p`): the record's constant stores `runs` (its words at `P -
    /// 0x20`), `*G_H = L` (`G_H` and `L` the first block's operands
    /// `+0x2a8` and `+0x2f1`), slot 941 `(*G_D)` (`+0x2f6`, `G_D` at
    /// `+0x2fc`) and its table's word `+k` to pppStartPart (the call at
    /// `+0x318`, every block's callee) with `ecx` = `P`, `edx` 0x1000 and
    /// `(word, 1, 0)`; then `P + 0x40`, `+0x50`, `+0x60` from the 16-byte
    /// constants at operands `src`, and the halfword `P + 0x1e` 1.
    fn part<G: Game>(
        &self,
        g: &mut G,
        img: &Image,
        runs: &[(u32, u32)],
        (k, p): (u32, u32),
        src: [u32; 3],
    ) {
        let holder = self.gw(g, img, 0x2a8);
        for &(from, to) in runs {
            crate::r25::abs_stores(g, img, self.f, from, to);
        }
        g.write::<u32>(holder, self.w(img, 0x2f1));
        let d = self.gw(g, img, 0x2fc);
        // call width: 1
        let table = g.call(self.slot(g, img, 0x2f6, 941), 0, &[d]).eax;
        let word = g.read::<u32>(table.wrapping_add(k));
        let part = self.w(img, p);
        crate::opu_parts::start_part(g, img, self.target(img, 0x318), part, 0x1000, word, 1);
        for (i, op) in (0u32..).zip(src) {
            let from = self.w(img, op);
            for j in 0..4 {
                let v = g.read::<u32>(from + 4 * j);
                g.write::<u32>(part + 0x40 + 0x10 * i + 4 * j, v);
            }
        }
        g.write::<u16>(part + 0x1e, 1);
    }
}

/// The parts of the `G_C3 == 0x64` block onwards: `(runs, (k, P), src)`.
type Part = (&'static [(u32, u32)], (u32, u32), [u32; 3]);

const PART_64: Part = (&[(0x2ac, 0x2ef)], (0, 0x313), [0x31f, 0x32e, 0x33d]);
const PART_87: Part = (&[(0x369, 0x3ac)], (4, 0x3d2), [0x3de, 0x3ed, 0x3fc]);
const PART_8A: Part = (&[(0x428, 0x46b)], (8, 0x491), [0x49d, 0x4ac, 0x4bb]);
const PARTS_122: [Part; 2] = [
    (&[(0x4de, 0x521)], (0x10, 0x547), [0x553, 0x567, 0x576]),
    (&[(0x588, 0x5cb)], (0xc, 0x5f1), [0x5fd, 0x60c, 0x61b]),
];
const PART_290: Part = (&[(0x64a, 0x68d)], (0x30, 0x6b3), [0x6bf, 0x6ce, 0x6dd]);
const PART_2E1: Part = (&[(0x739, 0x77c)], (0x34, 0x7a2), [0x7ae, 0x7bd, 0x7cc]);
const PART_CAMERA: Part = (&[(0x92a, 0x96d)], (0x14, 0x993), [0x99f, 0x9ae, 0x9bd]);
const PARTS_C8: [Part; 2] = [
    (&[(0xa8c, 0xacf)], (0x18, 0xaf5), [0xb01, 0xb15, 0xb24]),
    (&[(0xb36, 0xb79)], (0x18, 0xb9f), [0xbab, 0xbba, 0xbc9]),
];
const PART_C40_0: Part = (&[(0xbf5, 0xc38)], (0x2c, 0xc5e), [0xc6a, 0xc79, 0xc88]);
const PARTS_C40_64: [Part; 6] = [
    (&[(0xcb4, 0xcf7)], (0x28, 0xd1d), [0xd29, 0xd3d, 0xd4c]),
    (&[(0xd5e, 0xda1)], (0x28, 0xdc7), [0xdd3, 0xde7, 0xdf6]),
    (&[(0xe08, 0xe4b)], (0x28, 0xe71), [0xe7d, 0xe8c, 0xe9b]),
    (
        &[(0xead, 0xec8), (0xecd, 0xef5)],
        (0x28, 0xf1b),
        [0xf27, 0xf3b, 0xf4a],
    ),
    (&[(0xf5c, 0xf9f)], (0x24, 0xfc8), [0xfd4, 0xfe8, 0xff7]),
    (
        &[(0x1009, 0x104c)],
        (0x24, 0x1072),
        [0x107e, 0x108d, 0x109c],
    ),
];
const PARTS_C3C_0: [Part; 2] = [
    (
        &[(0x10c8, 0x110b)],
        (0x18, 0x1131),
        [0x113d, 0x1151, 0x1160],
    ),
    (
        &[(0x1172, 0x11b5)],
        (0x18, 0x11db),
        [0x11e7, 0x11f6, 0x1205],
    ),
];

/// `magic_0519`'s scene task (0x10002d60, 4797 bytes; cdecl, no
/// arguments, returns 0). With `H = *G_H` (operand `+0x16`):
/// - once each: slot 225 `(H)` must answer 0 (else return), slot 226
///   `(H, 0)`, slot 227 `(H)` must answer 1 or more (else return); the
///   first time also slots 711 and 710 with the lock word `*[G_L]` 1
///   around 711;
/// - slot 261 asked for keys 7 down to 0, the first set one counting:
///   7 `G_C1`, 6 `G_C2`, 5 `G_C5` (at 10 the ratio of the record at
///   `+0x18d`), 4 `G_C4` and `G_C6`, 3 `G_C6`, 2 `G_C7`, 1 `G_C3` and
///   `G_C8`, 0 `G_C3`; the game's busy word stops it there (and after);
/// - `G_C3` at 0x64, 0x87, 0x8a, 0x122 or `G_C8` at 0x290 starts parts
///   (0x122 clears the halves first); `G_C8` at 0x2e1 stops the part P8
///   if it runs (slot 713) and starts it again;
/// - `G_C8` at 0x106, 0x14d, 0x20f, 0x2e1 stops P0's halfword, at 0x122,
///   0x1a4, 0x23f sets it; else the first of `G_C7` 0 or 0x1e (slot 222),
///   `G_C6` 0 (the camera's set-up: slots 267, 331, 332, the light placed,
///   the camera record, a part), `G_C4` 0 (the camera stopped: slots 334,
///   271, 332), `G_C5` 0 (slot 222) or 0xc8 (two parts), `G_C2` 0 (a part)
///   or 0x64 (six parts), `G_C1` 0 (two parts) or 0x12c (slots 271, 332,
///   214);
/// - while not busy, with a camera record, the camera steps; the cameras
///   ([`cameras_0515`](crate::r36::cameras_0515)) run with one.
pub fn scene_task_0519<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let c = Ctx0519 {
        f: fr.at,
        ce: fr.esp.wrapping_sub(0x1c),
    };
    let h = |g: &G| c.gw(g, img, 0x16);
    if c.gw(g, img, 0x08) == 0 {
        let hv = h(g);
        // call width: 1
        if g.call(c.slot(g, img, 0x10, 225), 0, &[hv]).eax != 0 {
            return Ret::eax(0);
        }
        c.inc(g, img, 0x2f);
    }
    if c.gw(g, img, 0x35) == 0 {
        let hv = h(g);
        g.write::<u32>(c.w(img, 0x51), 1);
        // call width: 2
        g.call(c.slot(g, img, 0x3d, 226), 0, &[hv, 0]);
    }
    if c.gw(g, img, 0x60) == 0 {
        let hv = h(g);
        // call width: 1
        let r = g.call(c.slot(g, img, 0x68, 227), 0, &[hv]).eax;
        if r.cast_signed() < 1 {
            return Ret::eax(0);
        }
        c.inc(g, img, 0x88);
    }
    if c.gw(g, img, 0x8e) == 0 {
        let lock = c.gw(g, img, 0x97);
        let saved = g.read::<u32>(lock);
        g.write::<u32>(lock, 1);
        g.write::<u32>(c.w(img, 0xbc), 1);
        let s = c.slot(g, img, 0xa6, 711);
        // call width: 3
        g.call(s, 0, &[c.w(img, 0xb6), c.w(img, 0xab), 0]);
        let lock = c.gw(g, img, 0xc7);
        g.write::<u32>(lock, saved);
        let s = c.slot(g, img, 0xd3, 710);
        // call width: 4
        g.call(
            s,
            0,
            &[c.w(img, 0xe8), c.w(img, 0xe3), c.w(img, 0xd8), 0x80000],
        );
    }
    if keys(g, img, &c) {
        return Ret::eax(tail(g, img, &c));
    }
    parts(g, img, &c);
    rest(g, img, &c);
    if c.busy(g, img) {
        return Ret::eax(tail(g, img, &c));
    }
    if c.gw(g, img, 0x129c) == 0 {
        return Ret::eax(0);
    }
    let eax = c.gw(g, img, 0x1291);
    crate::r23::camera_step_0519(g, img, c.target(img, 0x12a4), eax);
    Ret::eax(tail(g, img, &c))
}

/// Slot 261 (operand `+0x121`) asked for keys 7 down to 0; the first set
/// one counts its globals. Returns true when the busy word stops the task.
fn keys<G: Game>(g: &mut G, img: &Image, c: &Ctx0519) -> bool {
    // (key, the slot's operand)
    for (key, op) in [
        (7u32, 0xf2u32),
        (6, 0x121),
        (5, 0x150),
        (4, 0x19c),
        (3, 0x1d1),
        (2, 0x200),
        (1, 0x22c),
        (0, 0x25e),
    ] {
        let s = c.slot(g, img, op, 261);
        // call width: 1
        if g.call(s, 0, &[key]).eax == 0 {
            continue;
        }
        if c.busy(g, img) {
            return true;
        }
        match key {
            7 => {
                c.inc(g, img, 0x117);
            }
            6 => {
                c.inc(g, img, 0x146);
            }
            5 => {
                if c.inc(g, img, 0x174) == 10 {
                    let fr = c.frame(img, 0x192, 0);
                    crate::o4_more::float_ratio_half(
                        g,
                        img,
                        fr.at,
                        fr.esp,
                        c.w(img, 0x18d),
                        0x1084,
                    );
                }
            }
            4 => {
                c.inc(g, img, 0x1c1);
                c.inc(g, img, 0x1c7);
            }
            3 => {
                c.inc(g, img, 0x1f6);
            }
            2 => {
                c.inc(g, img, 0x225);
            }
            1 => {
                c.inc(g, img, 0x251);
                c.inc(g, img, 0x257);
            }
            _ => {
                c.inc(g, img, 0x283);
            }
        }
        break;
    }
    c.busy(g, img)
}

/// The parts `G_C3` and `G_C8` start (`+0x295` to `+0x7e1`).
fn parts<G: Game>(g: &mut G, img: &Image, c: &Ctx0519) {
    let part = |g: &mut G, p: &Part| c.part(g, img, p.0, p.1, p.2);
    let c3 = c.gw(g, img, 0x297);
    if c3 == 0x64 {
        part(g, &PART_64);
    } else if c3 == 0x87 {
        part(g, &PART_87);
    } else if c3 == 0x8a {
        part(g, &PART_8A);
    } else if c3 == 0x122 {
        crate::o4_more2::clear_halves_slot(g, img, &c.frame(img, 0x4d5, 0));
        for p in &PARTS_122 {
            part(g, p);
        }
    } else if c.gw(g, img, 0x637) == 0x290 {
        part(g, &PART_290);
    }
    if c.gw(g, img, 0x6f4) == 0x2e1 {
        // the part P8 stopped first if it runs
        if g.read::<u16>(c.w(img, 0x705)) != 0 {
            let holder = c.gw(g, img, 0x70d);
            g.write::<u32>(holder, c.w(img, 0x718));
            let s = c.slot(g, img, 0x71d, 713);
            // call width: 1
            g.call(s, 0, &[c.w(img, 0x712)]);
            g.write::<u16>(c.w(img, 0x730), 0);
        }
        part(g, &PART_2E1);
    }
}

/// `+0x7e1` to `+0x1290`: P0's halfword by `G_C8`, else the first block
/// of the chain that applies.
fn rest<G: Game>(g: &mut G, img: &Image, c: &Ctx0519) {
    let part = |g: &mut G, p: &Part| c.part(g, img, p.0, p.1, p.2);
    match c.gw(g, img, 0x7e2) {
        0x106 | 0x14d | 0x20f | 0x2e1 => {
            g.write::<u16>(c.w(img, 0x128c), 0);
            return;
        }
        0x122 | 0x1a4 | 0x23f => {
            g.write::<u16>(c.w(img, 0x1282), 1);
            return;
        }
        _ => {}
    }
    // slot 222 (`+0xa5d`) with `(*G_S, a, b, 0x100)` (`G_S` at `+0xa63`)
    let s222 = |g: &mut G, a: u32, b: u32| {
        let s = c.slot(g, img, 0xa5d, 222);
        let v = c.gw(g, img, 0xa63);
        // call width: 4
        g.call(s, 0, &[v, a, b, 0x100]);
    };
    let c7 = c.gw(g, img, 0x835);
    if c7 == 0 {
        crate::o4_more2::clear_halves_slot(g, img, &c.frame(img, 0x83d, 0));
        s222(g, 0, 0x13);
    } else if c.gw(g, img, 0x851) == 0x1e {
        s222(g, 0x14, 0x27);
    } else if c.gw(g, img, 0x868) == 0 {
        camera_start(g, img, c);
        part(g, &PART_CAMERA);
    } else if c.gw(g, img, 0x9d9) == 0 {
        crate::o4_more2::clear_halves_slot(g, img, &c.frame(img, 0x9e1, 0));
        let s = c.slot(g, img, 0x9e6, 334);
        // call width: 1
        g.call(s, 0, &[0]);
        g.write::<u32>(c.w(img, 0x9f6), 0);
        crate::o4_fam3::slot_list(g, img, &c.frame(img, 0x9ff, 1));
        lights(g, img, c, [0xa04, 0xa13, 0xa22, 0xa31], 271);
    } else if c.gw(g, img, 0xa4c) == 0 {
        s222(g, 0x28, 0x31);
    } else if c.gw(g, img, 0xa79) == 0xc8 {
        for p in &PARTS_C8 {
            part(g, p);
        }
    } else if c.gw(g, img, 0xbe5) == 0 {
        part(g, &PART_C40_0);
    } else if c.gw(g, img, 0xca4) == 0x64 {
        for p in &PARTS_C40_64 {
            part(g, p);
        }
    } else if c.gw(g, img, 0x10b8) == 0 {
        for p in &PARTS_C3C_0 {
            part(g, p);
        }
    } else if c.gw(g, img, 0x121e) == 0x12c {
        lights(g, img, c, [0x1229, 0x1238, 0x1247, 0x1256], 271);
        let s = c.slot(g, img, 0x1268, 214);
        let hv = c.gw(g, img, 0x126e);
        // call width: 1
        g.call(s, 0, &[hv]);
    }
}

/// Slot `n` (operands `ops[0..3]`) with 1, 4 and 0x10, then slot 332
/// (`ops[3]`) with the list `L` (the immediate at `+0x8ab`).
fn lights<G: Game>(g: &mut G, img: &Image, c: &Ctx0519, ops: [u32; 4], n: u32) {
    for (op, v) in ops[..3].iter().zip([1u32, 4, 0x10]) {
        let s = c.slot(g, img, *op, n);
        // call width: 1
        g.call(s, 0, &[v]);
    }
    let s = c.slot(g, img, ops[3], 332);
    // call width: 1
    g.call(s, 0, &[c.w(img, 0x8ab)]);
}

/// The camera's set-up (`G_C6` 0, `+0x873`): the list's slot calls, slots
/// 267 (1, 4, 0x10), 331 `(L)`, 332 `(M)`, the halves cleared; the light
/// placed at `(0, 0, 0, 1)` (the frame's `ebp - 0x10`), `G_K` = slot
/// 287's answer, the camera record `R` (`+0x90c`) in `G_R`, the frame and
/// counts 0, the record's tracks started.
fn camera_start<G: Game>(g: &mut G, img: &Image, c: &Ctx0519) {
    crate::o4_fam3::slot_list(g, img, &c.frame(img, 0x874, 0));
    for (op, v) in [(0x879u32, 1u32), (0x888, 4), (0x897, 0x10)] {
        let s = c.slot(g, img, op, 267);
        // call width: 1
        g.call(s, 0, &[v]);
    }
    let s = c.slot(g, img, 0x8a6, 331);
    // call width: 1
    g.call(s, 0, &[c.w(img, 0x8ab)]);
    let s = c.slot(g, img, 0x8b8, 332);
    // call width: 1
    g.call(s, 0, &[c.w(img, 0x8bd)]);
    // (five words pushed, four popped after the halves)
    crate::o4_more2::clear_halves_slot(g, img, &c.frame(img, 0x8ca, 5));
    // the point at ebp - 0x10 (ebp = E - 4)
    let p = c.ce.wrapping_add(0x1c - 0x14);
    for (i, v) in (0u32..).zip([0, 0, 0, 0x3f80_0000]) {
        g.write::<u32>(p + 4 * i, v);
    }
    let k = crate::o4_tiny::slot_imm8x3(g, img, &c.frame(img, 0x8ee, 1)).eax;
    g.write::<u32>(c.w(img, 0x8f6), k);
    crate::m0519::place(g, img, c.target(img, 0x8fb), p);
    let r = c.w(img, 0x902);
    g.write::<u32>(c.w(img, 0x908), c.w(img, 0x90c));
    g.write::<u16>(c.w(img, 0x912), 0);
    g.write::<u32>(c.w(img, 0x917), 0);
    g.write::<u32>(c.w(img, 0x91c), 0);
    crate::r23::camera_tracks_0519(g, img, c.target(img, 0x921), r);
}

/// `+0x12a8`: the cameras when `G_R` holds a record. Returns 0.
fn tail<G: Game>(g: &mut G, img: &Image, c: &Ctx0519) -> u32 {
    if c.gw(g, img, 0x12aa) != 0 {
        crate::r36::cameras_0515(g, img, &c.frame(img, 0x12b2, 0));
    }
    0
}

/// [`scene_task_0519`] (4797 bytes, 1 overlay).
pub const SCENE_TASK_0519: Sig = Sig {
    len: 4797,
    hash: 0xbb7f_99fb_3cdb_6a60,
    calls: &[
        (
            0x192,
            Callee::Sig(&crate::o4_more::FLOAT_RATIO_HALF_0519_2750),
        ),
        (0x318, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x3d7, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x496, Callee::Sig(&crate::opu_parts::START_PART)),
        (
            0x4d5,
            Callee::Sig(&crate::o4_more2::CLEAR_HALVES_SLOT_0519_2CF0),
        ),
        (0x54c, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x5f6, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x6b8, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x7a7, Callee::Sig(&crate::opu_parts::START_PART)),
        (
            0x83d,
            Callee::Sig(&crate::o4_more2::CLEAR_HALVES_SLOT_0519_2CF0),
        ),
        (0x874, Callee::Sig(&crate::o4_fam3::SLOT_LIST_0519_2A50)),
        (
            0x8ca,
            Callee::Sig(&crate::o4_more2::CLEAR_HALVES_SLOT_0519_2CF0),
        ),
        (0x8ee, Callee::Sig(&crate::o4_tiny::SLOT_IMM8X3_0519_4E40)),
        (0x8fb, Callee::Sig(&crate::m0519::PLACE)),
        (0x921, Callee::Sig(&crate::r23::CAMERA_TRACKS_0519)),
        (0x998, Callee::Sig(&crate::opu_parts::START_PART)),
        (
            0x9e1,
            Callee::Sig(&crate::o4_more2::CLEAR_HALVES_SLOT_0519_2CF0),
        ),
        (0x9ff, Callee::Sig(&crate::o4_fam3::SLOT_LIST_0519_2AC0)),
        (0xafa, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xba4, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xc63, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xd22, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xdcc, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xe76, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xf20, Callee::Sig(&crate::opu_parts::START_PART)),
        (0xfcd, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x1077, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x1136, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x11e0, Callee::Sig(&crate::opu_parts::START_PART)),
        (0x12a4, Callee::Sig(&crate::r23::CAMERA_STEP_0519)),
        (0x12b2, Callee::Sig(&crate::r36::CAMERAS_0515)),
    ],
    imports: &[],
};

/// `magic_0070`'s calling thunk over its node ease (0x100046d0, 15 bytes;
/// cdecl `(node)`): `push ebp; mov ebp, esp; push ecx` (the caller's
/// `ecx`, never read), `ecx` = the node, a call to
/// [`node_ease`](crate::s9_misc::node_ease) (the copy at 0x100040e0).
/// Returns its `eax`, nothing popped.
pub fn call_node_ease_0070<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let ecx = fr.arg(g, 0);
    // ebp, the pushed ecx, the call's return address
    let ease = Frame {
        at: img.rel32_target(fr.at + 8),
        esp: fr.esp.wrapping_sub(12),
        ecx,
        ..*fr
    };
    Ret::eax(crate::s9_misc::node_ease(g, img, &ease).eax)
}

/// [`call_node_ease_0070`] (15 bytes, 1 overlay).
pub const CALL_NODE_EASE_0070: Sig = Sig {
    len: 15,
    hash: crate::s3_wrap::ECX_CALL_HASH,
    calls: &[(8, Callee::Sig(&crate::s9_misc::NODE_EASE_0070))],
    imports: &[],
};
