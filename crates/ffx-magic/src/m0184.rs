//! `magic_0184.dll`'s own functions (besides the runtime, the object
//! host and the shared code).

use crate::runtime::slot_target;
use crate::{Frame, Game, Image, Ret, Sig};

/// The parts' release (0x10001900, no arguments): for each of four
/// objects (the immediates `+0x06`, `+0x23`, `+0x40`, `+0x5d`), `**Q` the
/// immediate after it (`Q` at `+0x01`..) and slot 713 `(obj)`, each call
/// seeing the earlier ones' words. Returns 0.
pub fn release_four<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut stack: Vec<u32> = Vec::new();
    for (q, obj, imm, table) in [
        (0x01, 0x06, 0x0c, 0x11),
        (0x1e, 0x23, 0x29, 0x2e),
        (0x3b, 0x40, 0x46, 0x4b),
        (0x58, 0x5d, 0x63, 0x68),
    ] {
        let qp = g.read::<u32>(w(q));
        stack.insert(0, w(obj));
        g.write::<u32>(qp, w(imm));
        let s = slot_target(g, w(table), 713);
        // call width: 4 (the earlier calls' words left on the stack)
        g.call(s, 0, &stack);
    }
    Ret::eax(0)
}

/// [`release_four`] (122 bytes, 1 overlay).
pub const RELEASE_FOUR: Sig = Sig {
    len: 122,
    hash: 0x00ec_4b97_734f_8a8a,
    calls: &[],
    imports: &[],
};

/// The particle set-up (0x10001390, no arguments): for the effect `NAME`
/// (the immediate at `+0x06`, again in each call) slot 718 `(NAME)`,
/// slots 222 and 221 `(NAME, 0, 1000, 0x100)` and slot 719 `(NAME, 6,
/// i)` for i = 0..3 into four globals (`+0x65`, `+0x80`, `+0x9e`,
/// `+0xbc`). Returns the last one. Each slot call passes the words the
/// original leaves on the stack.
pub fn part_setup<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    let w = |off: u32| img.word(fr.at + off);
    let mut p = crate::pushes::Pushes::default();
    p.push(w(0x06));
    let s = slot_target(g, w(0x01), 0xb38 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0x100, 1000, 0, w(0x2a)]);
    let s = slot_target(g, w(0x13), 0x378 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    p.push_all(&[0x100, 1000, 0, w(0x48)]);
    let s = slot_target(g, w(0x31), 0x374 / 4);
    // call width: 12 (Pushes caps the words at 12)
    p.call(g, s, 0);
    let mut last = 0;
    for (i, table, name, dst) in [
        (0, 0x4f, 0x5e, 0x65),
        (1, 0x6a, 0x79, 0x80),
        (2, 0x85, 0x94, 0x9e),
        (3, 0xa3, 0xb2, 0xbc),
    ] {
        p.push_all(&[i, 6, w(name)]);
        let s = slot_target(g, w(table), 0xb3c / 4);
        // call width: 12 (Pushes caps the words at 12)
        last = p.call(g, s, 0).eax;
        if i == 2 {
            // add esp, 0x48: everything pushed so far
            p.pop(18);
        }
        g.write::<u32>(w(dst), last);
    }
    Ret::eax(last)
}

/// [`part_setup`] (193 bytes, 1 overlay).
pub const PART_SETUP: Sig = Sig {
    len: 193,
    hash: 0x8e45_ba1b_2eb6_e0e2,
    calls: &[],
    imports: &[],
};

/// Scatters two records (0x10001280, internal: `ecx` = a base point `B`,
/// `xmm1` a spread `a`, `xmm2` a scale `b`; locals at `ebp - 4`): twice, a
/// free record `R` ([`first_free`](crate::s4_small::first_free); none
/// ends it) gets, from four answers `r` of slot 241 `(&r)` (the earlier
/// calls' words left under each): `R.x = (r1 - K) a + B.x` (`r1 - K`
/// kept in the local), `R.y = B.y - r2 a`, `R.z = (r3 - K) a + B.z`, `R.w`
/// 1.0, `R + 0x10 = (r4 + D) b` (in doubles; `K` at `+0x4b`/`+0xa4`, the
/// double `D` at `+0xe8`) and `R + 0x14` 0. Returns 0.
pub fn scatter<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    use crate::sb_sse::{add, mul, sub};
    let f = fr.at;
    let w = |o: u32| img.word(f + o);
    let base = fr.ecx;
    let (a, b) = (fr.xmm_f32(1), fr.xmm_f32(2));
    let ebp = fr.esp.wrapping_sub(4);
    let r = ebp.wrapping_sub(4);
    // push ebp; sub esp, 0xc; push ebx/esi/edi; the call
    let below = ebp.wrapping_sub(0x1c);
    g.write::<f32>(ebp.wrapping_sub(0xc), b);
    g.write::<f32>(ebp.wrapping_sub(8), a);
    for _ in 0..2 {
        let free = Frame {
            at: img.rel32_target(f + 0x22),
            esp: below,
            ..*fr
        };
        let rec = crate::s4_small::first_free(g, img, &free).eax;
        if rec == 0 {
            break;
        }
        let ask = |g: &mut G, n: usize, o: u32| {
            let t = slot_target(g, w(o), 241);
            g.call(t, 0, &[r; 4][..n]);
            g.read::<f32>(r)
        };
        let v = sub(ask(g, 1, 0x32), g.read::<f32>(w(0x4b)));
        g.write::<f32>(r, v);
        g.write::<f32>(rec, add(mul(v, a), g.read::<f32>(base)));
        let v = ask(g, 2, 0x54);
        g.write::<f32>(rec + 4, sub(g.read::<f32>(base + 4), mul(v, a)));
        let v = sub(ask(g, 3, 0x8a), g.read::<f32>(w(0xa4)));
        g.write::<f32>(r, v);
        g.write::<u32>(rec + 0xc, 0x3f80_0000);
        g.write::<f32>(rec + 8, add(mul(v, a), g.read::<f32>(base + 8)));
        let v = ask(g, 4, 0xad);
        g.write::<u32>(rec + 0x14, 0);
        let d = g.read::<f64>(w(0xe8));
        #[allow(clippy::cast_possible_truncation)] // cvtpd2ps
        let y = ((f64::from(v) + d) * f64::from(b)) as f32;
        g.write::<f32>(rec + 0x10, y);
    }
    Ret::eax(0)
}

/// [`scatter`] (272 bytes, 1 overlay).
pub const SCATTER: Sig = Sig {
    len: 272,
    hash: 0x5336_705c_250a_04c4,
    calls: &[(0x22, crate::Callee::Sig(&crate::s4_small::FIRST_FREE))],
    imports: &[],
};
