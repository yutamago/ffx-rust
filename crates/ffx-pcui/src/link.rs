//! The sphere grid's links between nodes: the end points and outlines of
//! straight links (`AbmapCalcLinkPointsStraight`, `0x00a581f0`) and of
//! links curving around a centre (`AbmapCalcLinkPointsCurved`,
//! `0x00a57710`). Floats
//! follow the x87 code: 53-bit arithmetic, rounded to `f32` where the
//! original stores; the length goes through the emulated VU0's registers
//! as the PS2 code did.

#![allow(clippy::many_single_char_names)] // points and vector components

use ffx_bridge::x87::{self, Reg};

/// What the emulated VU0 holds after a straight link: `vf09` (`0x00c8f810`,
/// the offset plus `vf00`), `vf10`... as stored, and `Q`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vu0 {
    /// `0x00c8f810..0x00c8f81c`: `(dx + vf00.x, dy + vf00.y, vf00.z + 0)`.
    pub vf09: [f32; 3],
    /// `0x00c8f820..0x00c8f82c`: the unit offset (`vf09 * Q`, three
    /// components).
    pub unit: [f32; 3],
    /// `0x00c8f830..0x00c8f83c`: (length, `dy²`, `0 * 0`, 1).
    pub vf10: [f32; 4],
    /// `Q`: `vf00.w` over the length.
    pub q: f32,
}

/// A straight link from `a` to `b` (each moved toward the other by its
/// node's radius `ra`, `rb` along the unit offset), with `vf00` the
/// emulated VU0's `vf00`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Straight {
    /// `a` moved: `a + ra * unit`.
    pub a: [f32; 2],
    /// `b` moved: `b - rb * unit`.
    pub b: [f32; 2],
    /// The two outline points: `(a, (uy, -ux), (-uy, ux))` then the same
    /// at `b`, 12 floats.
    pub out: [f32; 12],
    /// The VU0 registers left behind.
    pub vu0: Vu0,
}

/// The emulated VU0's in-place squares of `(x, y, 0)` (`vf10`: `x²`,
/// `y²`, `0 * 0`, then `x² + y²`, `+ 0`): returns `y²`, `0 * 0` (what is
/// left of them) and `sqrt|sum|` (`_CIsqrt`, stored).
fn squares(x: f32, y: f32) -> (f32, f32, f32) {
    let xx = (Reg::load(x) * Reg::load(x)).store();
    let yy = Reg::load(y).mul_m32(y).store();
    let zz = Reg::ZERO.mul_m32(0.0).store();
    let sxy = Reg::load(yy).add_m32(xx).store();
    let sum = Reg::load(zz).add_m32(sxy).store();
    let abs = f32::from_bits(x87::store(x87::load(sum.to_bits()).abs()));
    let root = f32::from_bits(x87::store(x87::sqrt(x87::load(abs.to_bits()))));
    (yy, zz, root)
}

/// The VU0 normalisation the links end with: `vf09` = `v9`, the length
/// `root + vf00.x` (stored over `vf10.x`), `Q = vf00.w / length`, the unit
/// `vf09 * Q`.
fn normalize(v9: [f32; 3], vf00: [f32; 4], yy: f32, zz: f32, root: f32) -> Vu0 {
    let len = Reg::load(root).add_m32(vf00[0]).store();
    let q = Reg::load(vf00[3]).div_m32(len).store();
    let ux = (Reg::load(v9[0]) * Reg::load(q)).store();
    let uy = (Reg::load(v9[1]) * Reg::load(q)).store();
    let uz = Reg::load(q).mul_m32(v9[2]).store();
    Vu0 {
        vf09: v9,
        unit: [ux, uy, uz],
        vf10: [len, yy, zz, 1.0],
        q,
    }
}

/// `vf00.z + 0`, as `fld; fldz; fadd`.
fn vz(vf00: [f32; 4]) -> f32 {
    (Reg::load(vf00[2]) + Reg::ZERO).store()
}

/// [`Straight`] for nodes at `a`, `b` with radii `ra`, `rb`.
#[must_use]
pub fn straight(a: [f32; 2], b: [f32; 2], ra: f32, rb: f32, vf00: [f32; 4]) -> Straight {
    let dx = Reg::load(b[0]).sub_m32(a[0]).store();
    let dy = Reg::load(b[1]).sub_m32(a[1]).store();
    let v9 = [
        Reg::load(dx).add_m32(vf00[0]).store(),
        Reg::load(dy).add_m32(vf00[1]).store(),
        vz(vf00),
    ];
    let (yy, zz, root) = squares(dx, dy);
    let vu0 = normalize(v9, vf00, yy, zz, root);
    let [ux, uy, _] = vu0.unit;
    let a = [
        (Reg::load(ux) * Reg::load(ra) + Reg::load(a[0])).store(),
        (Reg::load(ra) * Reg::load(uy) + Reg::load(a[1])).store(),
    ];
    let b = [
        (Reg::load(ux) * Reg::load(rb)).subr_m32(b[0]).store(),
        (Reg::load(rb) * Reg::load(uy)).subr_m32(b[1]).store(),
    ];
    let (nx, ny) = (-ux, -uy);
    Straight {
        a,
        b,
        out: [a[0], a[1], uy, nx, ny, ux, b[0], b[1], uy, nx, ny, ux],
        vu0,
    }
}

/// `f32` pi as the double the original loads (`0x00b92288`).
const PI: f64 = 3.141_592_741_012_573_2;
/// Twice that (`0x00b39378`).
const TAU: f64 = 6.283_185_482_025_146_5;

/// A link curving around a centre: `kind` (`2 + points`), the outline
/// rows `(point, (uy, -ux), (-uy, ux))` for each point between the ends
/// (`unit` the mean of the directions to and from it, normalised), and
/// the VU0 registers left by the last row.
#[derive(Debug, Clone, PartialEq)]
pub struct Curved {
    /// What `*kind` gets: the rows written.
    pub kind: u8,
    /// The outline rows.
    pub rows: Vec<[f32; 6]>,
    /// The VU0 registers left behind.
    pub vu0: Vu0,
}

/// What [`curved`] needs from the C runtime and the game: `_CIatan2(y,
/// x)` (stored as a float), `_ftol2_sse`, and `g_pppSinTable[i]`.
pub trait CurveEnv {
    /// `_CIatan2(y, x)` rounded to `f32`.
    fn atan2(&mut self, y: f64, x: f64) -> f32;
    /// `_ftol2_sse(x)`.
    fn ftol(&mut self, x: f64) -> u32;
    /// `g_pppSinTable[i]` (`i` below 0x1000).
    fn sin(&mut self, i: u32) -> f32;
}

/// The link from `a` to `b` around the centre `c`, the ends moved along
/// the arc by the nodes' radii `ra`, `rb`: the angles of `a`, `b` about
/// `c` (`_CIatan2`), their difference brought within ±pi (the sign the
/// direction), the start and sweep less the radii over the distances;
/// `|arc length| / 20` points (2 to 6) plus the two ends' neighbours at
/// evenly spaced angles and radii between `|a - c|` and `|b - c|` (the
/// sine table), then a row per point between `a` and `b`.
pub fn curved(
    a: [f32; 2],
    b: [f32; 2],
    c: [f32; 2],
    ra: f32,
    rb: f32,
    vf00: [f32; 4],
    env: &mut impl CurveEnv,
) -> Curved {
    let v1 = [
        Reg::load(a[0]).sub_m32(c[0]).store(),
        Reg::load(a[1]).sub_m32(c[1]).store(),
    ];
    let ang1 = env.atan2(f64::from(v1[1]), f64::from(v1[0]));
    let v2 = [
        Reg::load(b[0]).sub_m32(c[0]).store(),
        Reg::load(b[1]).sub_m32(c[1]).store(),
    ];
    let ang2 = env.atan2(f64::from(v2[1]), f64::from(v2[0]));
    let (_, _, root1) = squares(v1[0], v1[1]);
    let r1 = (Reg::load(root1) + Reg::load(vf00[0])).store();
    let (_, _, root2) = squares(v2[0], v2[1]);
    let len2 = Reg::load(root2).add_m32(vf00[0]).store();
    let mut diff = (Reg::load(ang2) - Reg::load(ang1)).store();
    let (d, a1) = (f64::from(diff), f64::from(ang1));
    if a1 <= f64::from(ang2) {
        if d > PI {
            diff = (Reg::load(diff) - Reg::constant(TAU)).store();
        }
    } else if d < -PI {
        diff = (Reg::load(diff) + Reg::constant(TAU)).store();
    }
    let lead = Reg::load(ra) / Reg::load(r1);
    let tail = Reg::load(rb) / Reg::load(len2);
    let (start, sweep) = if 0.0 < f64::from(diff) {
        (Reg::load(ang1) + lead, Reg::load(diff) - lead - tail)
    } else {
        (Reg::load(ang1) - lead, lead + Reg::load(diff) + tail)
    };
    let (mut start, sweep) = (start.store(), sweep.store());
    let mut arc = ((Reg::load(len2) + Reg::load(r1)) * Reg::load(sweep)).store();
    if f64::from(arc) < 0.0 {
        arc = -arc;
    }
    #[allow(clippy::cast_possible_truncation)] // the low 16 bits, as movzx
    let n = (env.ftol((Reg::load(arc) / Reg::constant(20.0)).value()) as u16).cast_signed();
    let n = n.clamp(2, 6);
    let step = (Reg::ONE / Reg::load((Reg::constant(f64::from(n + 1))).store())).store();
    let dang = (Reg::load(sweep) * Reg::load(step)).store();
    let drad = ((Reg::load(len2) - Reg::load(r1)) * Reg::load(step)).store();
    #[allow(clippy::cast_sign_loss)] // 2..=6
    let count = (n + 2) as usize;
    let mut pts = vec![[0f32; 2]; count + 2];
    pts[0] = a.map(x87::round_trip);
    pts[count + 1] = b.map(x87::round_trip);
    let mut r = r1;
    for p in &mut pts[1..=count] {
        let i = env.ftol((Reg::load(start) * Reg::constant(32768.0) / Reg::constant(PI)).value())
            & 0xffff;
        let cos = env.sin(((i + 0x4000) >> 4) & 0xfff);
        let sin = env.sin(i >> 4);
        *p = [
            (Reg::load(cos) * Reg::load(r) + Reg::load(c[0])).store(),
            (Reg::load(sin) * Reg::load(r)).add_m32(c[1]).store(),
        ];
        r = (Reg::load(r) + Reg::load(drad)).store();
        start = (Reg::load(start) + Reg::load(dang)).store();
    }
    let (rows, vu0) = outline(&pts, vf00);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 4..=8
    let kind = (n + 2) as u8;
    Curved { kind, rows, vu0 }
}

/// The outline rows of the points between the ends of `pts`: the units
/// from the previous point and to the next one (no `Q` stored), summed
/// and normalised; returns the rows and the VU0 registers of the last.
fn outline(pts: &[[f32; 2]], vf00: [f32; 4]) -> (Vec<[f32; 6]>, Vu0) {
    let count = pts.len() - 2;
    let vz0 = vz(vf00);
    let mut vu0 = normalize([0.0; 3], vf00, 0.0, 0.0, 0.0);
    let mut rows = Vec::with_capacity(count);
    for k in 1..=count {
        let (prev, p, next) = (pts[k - 1], pts[k], pts[k + 1]);
        // the unit offsets from the previous point and to the next one
        let e1 = [
            Reg::load(p[0]).sub_m32(prev[0]).store(),
            Reg::load(p[1]).sub_m32(prev[1]).store(),
        ];
        let v9 = [
            Reg::load(e1[0]).add_m32(vf00[0]).store(),
            Reg::load(e1[1]).add_m32(vf00[1]).store(),
            vz0,
        ];
        let (_, _, root) = squares(e1[0], e1[1]);
        let u1 = unit_of(v9, root, vf00);
        let e2 = [
            Reg::load(next[0]).sub_m32(p[0]).store(),
            Reg::load(next[1]).sub_m32(p[1]).store(),
        ];
        let v9 = [
            (Reg::load(vf00[0]) + Reg::load(e2[0])).store(),
            Reg::load(e2[1]).add_m32(vf00[1]).store(),
            vz0,
        ];
        let (_, _, root) = squares(e2[0], e2[1]);
        let u2 = unit_of(v9, root, vf00);
        let sum = [
            Reg::load(u2[0]).add_m32(u1[0]).store(),
            Reg::load(u2[1]).add_m32(u1[1]).store(),
        ];
        let v9 = [
            (Reg::load(vf00[0]) + Reg::load(sum[0])).store(),
            Reg::load(sum[1]).add_m32(vf00[1]).store(),
            vz0,
        ];
        let (yy, zz, root) = squares(sum[0], sum[1]);
        let v = normalize(v9, vf00, yy, zz, root);
        let [nx, ny, _] = v.unit;
        rows.push([p[0], p[1], ny, -nx, -ny, nx]);
        vu0 = v;
    }
    (rows, vu0)
}

/// The loop's own normalisation (no `Q` stored): `vf00.w` (as a double)
/// over `root + vf00.x`, times `vf09` (three components).
fn unit_of(v9: [f32; 3], root: f32, vf00: [f32; 4]) -> [f32; 3] {
    let len = (Reg::load(root) + Reg::load(vf00[0])).store();
    let q = (Reg::load(vf00[3]) / Reg::load(len)).store();
    v9.map(|v| Reg::load(q).mul_m32(v).store())
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    use crate::cursor::bridge::{VU0_Q, VU0_VF00};

    /// The emulated VU0's `vf09` (`0x00c8f810`); `vf10` follows 0x20 on.
    pub const VU0_VF09: u32 = 0x00c8_f810;

    /// `vf00`'s four floats.
    fn vf00(mem: &impl Mem) -> [f32; 4] {
        let at = mem.rebase(VU0_VF00.sym);
        std::array::from_fn(|i| {
            #[allow(clippy::cast_possible_truncation)] // 0..4
            mem.read(at + 4 * i as u32)
        })
    }

    /// Writes `v` as floats from `at`.
    fn put(mem: &mut impl Mem, at: u32, v: &[f32]) {
        for (i, x) in (0u32..).zip(v) {
            mem.write(at.wrapping_add(4 * i), *x);
        }
    }

    /// `AbmapCalcLinkPointsStraight(out, a, b, ra, rb, kind)`
    /// (`0x00a581f0`, Fahrenheit's name): `*kind` = 2; the points `a`, `b`
    /// (2 floats each) moved toward each other by `ra`, `rb`
    /// ([`super::straight`]); 12 floats of outline into `out`; the VU0
    /// registers left as the original leaves them. Returns `out + 0x30`.
    pub fn calc_link_points_straight(
        mem: &mut impl Mem,
        out: u32,
        a: u32,
        b: u32,
        ra: f32,
        rb: f32,
        kind: u32,
    ) -> u32 {
        mem.write(kind, 2u8);
        let pa = [mem.read(a), mem.read(a.wrapping_add(4))];
        let pb = [mem.read(b), mem.read(b.wrapping_add(4))];
        let s = super::straight(pa, pb, ra, rb, vf00(mem));
        put_vu0(mem, &s.vu0);
        put(mem, a, &s.a);
        put(mem, b, &s.b);
        put(mem, out, &s.out);
        out.wrapping_add(0x30)
    }

    /// Writes the VU0 registers a link leaves.
    fn put_vu0(mem: &mut impl Mem, v: &super::Vu0) {
        let vf09 = mem.rebase(VU0_VF09);
        put(mem, vf09, &v.vf09);
        put(mem, vf09 + 0x10, &v.unit);
        put(mem, vf09 + 0x20, &v.vf10);
        VU0_Q.set(mem, v.q);
    }

    /// `g_pppSinTable`: 0x1000 sines over a turn.
    pub const SIN_TABLE: u32 = 0x00c4_4bf0;
    /// `_CIatan2`'s import slot (MSVCR110).
    pub const CI_ATAN2_SLOT: u32 = 0x00b0_c3b0;

    /// The game side of [`super::CurveEnv`]: `atan2` calls the C runtime's.
    struct Env<'a, M, F> {
        mem: &'a mut M,
        atan2: F,
    }

    impl<M: Mem, F: FnMut(&mut M, f64, f64) -> f32> super::CurveEnv for Env<'_, M, F> {
        fn atan2(&mut self, y: f64, x: f64) -> f32 {
            (self.atan2)(self.mem, y, x)
        }
        fn ftol(&mut self, x: f64) -> u32 {
            ffx_bridge::x87::ftol(self.mem, x)
        }
        fn sin(&mut self, i: u32) -> f32 {
            let at = self.mem.rebase(SIN_TABLE).wrapping_add(4 * i);
            self.mem.read(at)
        }
    }

    /// `AbmapCalcLinkPointsCurved(out, a, b, c, ra, rb, kind)`
    /// (`0x00a57710`, Fahrenheit's name): the link from `a` to `b` around
    /// `c` ([`super::curved`], `_CIatan2` through `atan2(mem, y, x)`):
    /// `*kind` = the rows, the rows (6 floats each) into `out`, the VU0
    /// registers as the last row leaves them. Returns the end of the rows
    /// (a caller returns it).
    pub fn calc_link_points_curved<M: Mem>(
        mem: &mut M,
        out: u32,
        pts: [u32; 3],
        ra: f32,
        rb: f32,
        kind: u32,
        atan2: impl FnMut(&mut M, f64, f64) -> f32,
    ) -> u32 {
        let point = |mem: &M, p: u32| [mem.read(p), mem.read(p.wrapping_add(4))];
        let [a, b, c] = pts.map(|p| point(mem, p));
        let regs = vf00(mem);
        let curve = super::curved(a, b, c, ra, rb, regs, &mut Env { mem, atan2 });
        mem.write(kind, curve.kind);
        let mut at = out;
        for row in &curve.rows {
            put(mem, at, row);
            at = at.wrapping_add(0x18);
        }
        put_vu0(mem, &curve.vu0);
        at
    }
}
