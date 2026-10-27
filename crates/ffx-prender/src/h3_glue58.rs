//! The render-side glue, fifty-eighth batch (M4 port round 17, list
//! `h3-glue-render`): the broken-screen shards' lighting.

/// `_CIsqrt`'s import thunk (the port computes it: [`ffx_bridge::x87::sqrt`]
/// rounds as its SSE2 path does) and `_CIpow`'s (`ST1` to the power
/// `ST0`).
pub const CI_SQRT: u32 = 0x0094_97b8;
pub const CI_POW: u32 = 0x0094_98f8;

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::Host;

    const ONE: u32 = 0x3f80_0000;
    const HALF: u32 = 0x3f00_0000;

    fn f(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// The shards' rotation (rows `+0x60`, `+0x70`, `+0x80`) applied to
    /// `v`: `(r0.j v0 + r1.j v1) + r2.j v2` for each column `j`.
    fn rotate(h: &impl Host, this: u32, v: [f64; 3]) -> [f64; 3] {
        let mut out = [0.0; 3];
        for (j, o) in (0u32..).zip(&mut out) {
            let a = x87::mul(f(h, this + 0x60 + 4 * j), v[0]);
            let b = x87::mul(f(h, this + 0x70 + 4 * j), v[1]);
            let c = x87::mul(f(h, this + 0x80 + 4 * j), v[2]);
            *o = x87::add(x87::add(a, b), c);
        }
        out
    }

    /// Three f32 at `at`, loaded.
    fn vec3(h: &impl Host, at: u32) -> [f64; 3] {
        [f(h, at), f(h, at.wrapping_add(4)), f(h, at.wrapping_add(8))]
    }

    fn load3(v: [u32; 3]) -> [f64; 3] {
        v.map(x87::load)
    }

    /// `v` divided by its length (`(x^2 + y^2) + z^2`, the root stored as
    /// f32), each stored as f32.
    fn normalize(v: [u32; 3]) -> [u32; 3] {
        let [x, y, z] = load3(v);
        let sum = x87::add(x87::add(x87::mul(x, x), x87::mul(y, y)), x87::mul(z, z));
        let len = x87::load(x87::store(x87::sqrt(sum)));
        load3(v).map(|c| x87::store(x87::div(c, len)))
    }

    /// The normal of shard vertex `src` (`+0x44`) rotated, less the
    /// rotated origin, normalized.
    fn normal(h: &impl Host, this: u32, src: u32) -> [u32; 3] {
        let origin = rotate(h, this, [0.0; 3]).map(x87::store);
        let normals: u32 = h.read(this + 0x44);
        let v = vec3(h, normals.wrapping_add(src));
        let p = rotate(h, this, v).map(x87::store);
        let d = [0, 1, 2].map(|j| x87::store(x87::sub(x87::load(p[j]), x87::load(origin[j]))));
        normalize(d)
    }

    /// `light - w` for the vertex position `w`.
    fn towards(light: [u32; 3], w: [f64; 3]) -> [u32; 3] {
        [0, 1, 2].map(|j| x87::store(x87::sub(x87::load(light[j]), w[j])))
    }

    /// `(a1 b1 + a0 b0) + a2 b2` (the order of the light term) as f32.
    fn dot10(a: [u32; 3], b: [u32; 3]) -> u32 {
        let [a0, a1, a2] = load3(a);
        let [b0, b1, b2] = load3(b);
        x87::store(x87::add(
            x87::add(x87::mul(a1, b1), x87::mul(a0, b0)),
            x87::mul(a2, b2),
        ))
    }

    /// The vertex's fourth word: 1.0 where its position's x is within
    /// -1..=1 and its z not negative (unordered compares count as in).
    fn inside(h: &impl Host, this: u32, src: u32) -> u32 {
        let positions: u32 = h.read(this + 0x40);
        let x = f(h, positions.wrapping_add(src));
        if 1.0 < x || -1.0 > x {
            return 0;
        }
        let z = f(h, positions.wrapping_add(src).wrapping_add(8));
        if 0.0 > z { 0 } else { ONE }
    }

    /// The vertex's colour words at `[+0x48] + dst` (the base reloaded for
    /// each store), then its fourth word.
    fn put(h: &mut impl Host, this: u32, src: u32, dst: u32, rgb: [u32; 3]) {
        for (k, v) in (0u32..).zip(rgb) {
            let out: u32 = h.read(this + 0x48);
            h.write(out.wrapping_add(dst).wrapping_add(4 * k), v);
        }
        let flag = inside(h, this, src);
        let out: u32 = h.read(this + 0x48);
        h.write(out.wrapping_add(dst).wrapping_add(0xc), flag);
    }

    /// `FUN_006d3650(shards; light x, y, z, unused, r, g, b, strength)`
    /// (thiscall, `ret 0x20`) -> `eax`: the vertex count done, or the mode
    /// when nothing ran. For each of the `+8` vertices (positions `+0x40`,
    /// normals `+0x44`, 12 bytes each; colours `+0x48`, 16 bytes each):
    /// mode 1 (`+0`) a diffuse grey, max(0, n.l) * 8 + 0.5 times
    /// `strength` times 1.2, at most 1; mode 2 a specular colour,
    /// (pow(r.e, 4) - 0.5) * 1.5 (at least 0) times `strength` times
    /// (r, g, b), each at most 0.5, with r the light reflected about the
    /// rotated normal and e the direction to the origin. The fourth word
    /// marks the vertices inside the screen.
    pub fn shade_shards(
        h: &mut impl Host,
        this: u32,
        light: [u32; 3],
        colour: [u32; 3],
        strength: u32,
    ) -> u32 {
        let mode: u32 = h.read(this);
        if mode != 1 && mode != 2 {
            return mode;
        }
        let mut i = 0u32;
        while i < h.read::<u32>(this + 8) {
            let (src, dst) = (i.wrapping_mul(12), i.wrapping_mul(16));
            let rgb = if mode == 1 {
                diffuse(h, this, src, light, strength)
            } else {
                specular(h, this, src, light, colour, strength)
            };
            put(h, this, src, dst, rgb);
            i += 1;
        }
        if i == 0 { mode } else { i }
    }

    fn diffuse(h: &impl Host, this: u32, src: u32, light: [u32; 3], strength: u32) -> [u32; 3] {
        let n = normal(h, this, src);
        let positions: u32 = h.read(this + 0x40);
        let w = vec3(h, positions.wrapping_add(src));
        let l = normalize(towards(light, w));
        let lit = x87::load(dot10(l, n));
        let lit = if 0.0 > lit { 0.0 } else { lit };
        let v = x87::mul(x87::add(x87::mul(lit, 8.0), 0.5), x87::load(strength));
        let v = x87::store(v);
        // the image's f64 1.2000000476837158 (1.2 as f32)
        let v = x87::store(x87::mul(x87::load(v), f64::from(1.2_f32)));
        let v = if x87::load(v) > 1.0 { ONE } else { v };
        [v; 3]
    }

    fn specular(
        h: &mut impl Host,
        this: u32,
        src: u32,
        light: [u32; 3],
        colour: [u32; 3],
        strength: u32,
    ) -> [u32; 3] {
        let n = normal(h, this, src);
        let positions: u32 = h.read(this + 0x40);
        let w = vec3(h, positions.wrapping_add(src));
        let l = normalize(towards(light, w));
        let [l0, l1, l2] = load3(l);
        let [n0, n1, n2] = load3(n);
        let d = x87::add(
            x87::add(x87::mul(l0, n0), x87::mul(l1, n1)),
            x87::mul(l2, n2),
        );
        let d = x87::load(x87::store(d));
        // 2 (n.l) n - l
        let q = [n0, n1, n2].map(|c| x87::store(x87::mul(c, d)));
        let twice = q.map(|c| x87::store(x87::mul(x87::load(c), 2.0)));
        let r = [0, 1, 2].map(|j| x87::store(x87::sub(x87::load(twice[j]), [l0, l1, l2][j])));
        let r = normalize(r);
        // towards the origin: the position negated (read again)
        let w = vec3(h, positions.wrapping_add(src));
        let e = normalize(w.map(|c| x87::store(-c)));
        let s = dot10(e, r);
        let s = h
            .call_intrinsic2(h.rebase(CI_POW), x87::load(s), 4.0)
            .to_bits();
        let s = x87::store(x87::sub(x87::load(s), 0.5));
        let s = x87::load(x87::store(x87::mul(x87::load(s), 1.5)));
        let s = if 0.0 > s { 0.0 } else { s };
        let k = x87::load(x87::store(x87::mul(s, x87::load(strength))));
        let half = x87::load(HALF);
        colour.map(|c| {
            let v = x87::store(x87::mul(x87::load(c), k));
            if x87::load(v) > half { HALF } else { v }
        })
    }
}
