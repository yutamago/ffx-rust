//! h2-render-b continuation round, batch 4: the mesh instance's bounds
//! from its skeleton, and the box transform it uses (two exact copies:
//! `0x004f63d0` and `0x004ffe40`).

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

use crate::descriptor::PHYRE_FREE;

/// `updateBoundsFromSkeleton`'s callees and constants.
const MALLOC_ALIGNED: u32 = 0x0042_fb80;
const JOINTS_FROM_IDENTITY: u32 = 0x004f_6f70;
const TRANSFORM_BOX: u32 = 0x004f_63d0;
const VEC3_MIN: u32 = 0x004f_8580;
const VEC3_MAX: u32 = 0x004f_84f0;
const SET_MIN_AND_MAX: u32 = 0x0050_0bd0;
/// The bounds' starting minimum (`FLT_MAX`) and maximum (`-FLT_MAX`).
const START_MIN: u32 = 0x00b1_7504;
const START_MAX: u32 = 0x00b1_f2d0;
/// `PMatrix4::transformPoint3(this; out, in)` (thiscall), the box's
/// first corner.
const TRANSFORM3: u32 = 0x004c_9eb0;

/// A float word through the FPU (`fld`/`fstp`: a signalling NaN comes
/// out quiet).
fn fpu(w: u32) -> u32 {
    x87::store(x87::load(w))
}

/// `fld a; fadd b; fstp`: the sum of two floats rounded to `f32`.
fn add(a: u32, b: u32) -> u32 {
    x87::store(x87::add(x87::load(a), x87::load(b)))
}

/// Three sums, lane by lane.
fn sum(a: [u32; 3], b: [u32; 3]) -> [u32; 3] {
    [add(a[0], b[0]), add(a[1], b[1]), add(a[2], b[2])]
}

/// A vector helper `f(out; a, b)` (cdecl): its result pointer.
fn pair(h: &mut impl Host, f: u32, out: u32, a: u32, b: u32) -> u32 {
    h.call_sym(f, Conv::Cdecl, &[out, a, b])
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        Conv, Host, JOINTS_FROM_IDENTITY, MALLOC_ALIGNED, PHYRE_FREE, SET_MIN_AND_MAX, START_MAX,
        START_MIN, TRANSFORM_BOX, TRANSFORM3, VEC3_MAX, VEC3_MIN, fpu, pair, sum, x87,
    };
    use super::{DEPTH_ZERO_ONE, FROM_3X4, MATRIX_COPY, MULTIPLY, TO_WORLD_3X4};

    /// The bytes of `updateBoundsFromSkeleton`'s frame below its
    /// exception registration (`ebp - 0xac` up to `ebp - 0xc`).
    pub const BOUNDS_FRAME: u32 = 0xa0;

    /// `PMeshInstance::updateBoundsFromSkeleton(this)` (`0x004f96a0`,
    /// thiscall): with bounds (`+0x1c`) and a mesh (`[this]`) that has
    /// bones (`+8`) and bone boxes (`+0x14`), the joint matrices of the
    /// mesh's joints (`+0x18`) plus one into a `PhyreMalloc` block (from
    /// the identity, `0x004f6f70`, after the first 64 bytes), each bone's
    /// box (32 bytes: minimum, its joint index, maximum) transformed by
    /// its joint (`0x004f63d0`) and folded into the running minimum
    /// (from `FLT_MAX`) and maximum (from `-FLT_MAX`) by the vector
    /// min/max helpers, then `setMinAndMax`; the block freed. Returns 0,
    /// or 0xd when the allocation fails. Every float goes through the FPU.
    /// `frame` is the original's `ebp - 0xac`.
    #[allow(clippy::too_many_lines)]
    pub fn update_bounds_from_skeleton(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        let local = |off: u32| frame + 0xac - off;
        h.write(local(0x10), this);
        let mesh: u32 = h.read(this);
        let bounds: u32 = h.read(this + 0x1c);
        let bones = h.read::<u32>(mesh + 8) & 0x7fff_ffff;
        let boxes: u32 = h.read(mesh + 0x14);
        h.write(local(0x34), bounds);
        h.write(local(0x18), boxes);
        if bounds == 0 || bones == 0 || boxes == 0 {
            return 0;
        }
        let joints = (h.read::<u32>(mesh + 0x18) & 0x7fff_ffff).wrapping_add(1);
        h.write(local(0x14), joints);
        h.write(local(0x1c), 0u32);
        let block = if joints == 0 {
            0
        } else {
            let p = h.call_sym(MALLOC_ALIGNED, Conv::Cdecl, &[joints << 6, 4]);
            h.write(local(0x1c), p);
            p
        };
        let owned = if block != 0 { joints } else { 0 };
        h.write(local(0x58), block);
        h.write(local(0x30), owned);
        h.write(local(0x5c), owned);
        let result = if block == 0 {
            0xd
        } else {
            let lo = fpu(h.read(h.rebase(START_MIN)));
            for off in [0x44, 0x40, 0x3c] {
                h.write(local(off), lo);
            }
            let matrices = block + 0x40;
            h.write(local(0x10), matrices);
            let hi = fpu(h.read(h.rebase(START_MAX)));
            for off in [0x2c, 0x28, 0x24] {
                h.write(local(off), hi);
            }
            h.call_sym(JOINTS_FROM_IDENTITY, Conv::Thiscall(this), &[matrices]);
            let mut b = boxes + 0x10;
            for _ in 0..bones {
                // the box's maximum, then its minimum, through the frame
                let max = [h.read(b), h.read(b + 4), h.read(b + 8)].map(fpu);
                h.write(local(0x18), max[0]);
                let joint: u32 = h.read(b - 4);
                let m = (joint << 6).wrapping_add(matrices);
                h.write(local(0x10), max[1]);
                h.write(local(0x14), max[2]);
                for (k, off) in [0x54, 0x50, 0x4c].into_iter().enumerate() {
                    h.write(local(off), fpu(max[k]));
                }
                let min = [h.read(b - 0x10), h.read(b - 0xc), h.read(b - 8)].map(fpu);
                h.write(local(0x18), min[0]);
                h.write(local(0x10), min[1]);
                h.write(local(0x14), min[2]);
                for (k, off) in [0x6c, 0x68, 0x64].into_iter().enumerate() {
                    h.write(local(off), fpu(min[k]));
                }
                h.call_sym(
                    TRANSFORM_BOX,
                    Conv::Cdecl,
                    &[local(0x6c), local(0x54), local(0x7c), local(0x8c), m],
                );
                let lo = pair(h, VEC3_MIN, local(0x9c), local(0x44), local(0x7c));
                for (k, off) in [0x44, 0x40, 0x3c].into_iter().enumerate() {
                    let w: u32 = h.read(lo + 4 * k as u32);
                    h.write(local(off), fpu(w));
                }
                let hi = pair(h, VEC3_MAX, local(0xac), local(0x2c), local(0x8c));
                for (k, off) in [0x2c, 0x28, 0x24].into_iter().enumerate() {
                    let w: u32 = h.read(hi + 4 * k as u32);
                    h.write(local(off), fpu(w));
                }
                b = b.wrapping_add(0x20);
            }
            h.call_sym(
                SET_MIN_AND_MAX,
                Conv::Thiscall(bounds),
                &[local(0x44), local(0x2c)],
            );
            0
        };
        if owned >> 31 == 0 && block != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[block]);
        }
        result
    }

    /// The bytes of the box transform's frame (`ebp - 0x130` up to
    /// `ebp`).
    pub const BOX_FRAME: u32 = 0x130;

    /// `transformBox(lo, hi, out_lo, out_hi, m)` (`0x004f63d0`, and its
    /// copy `0x004ffe40`; cdecl): the box `lo`..`hi` through matrix `m`:
    /// the axes `m.row(k) * hi[k]` (each `hi[k]` through the FPU, by way
    /// of the original's argument slot), the corner `m * lo`
    /// (`transformPoint3`), the other seven corners as sums of the corner
    /// and the axes (each sum rounded to `f32` and stored before it is
    /// used again, as the original does), then `out_lo` = the minimum
    /// and `out_hi` = the maximum of the eight by the vector helpers, in
    /// the original's pairing. All in the frame at the original's slots;
    /// `frame` is its `ebp - 0x130`.
    #[allow(clippy::too_many_lines, clippy::similar_names)]
    pub fn transform_box(
        h: &mut impl Host,
        lo: u32,
        hi: u32,
        out_lo: u32,
        out_hi: u32,
        m: u32,
        frame: u32,
    ) {
        let local = |off: u32| frame + 0x130 - off;
        // the axes: row k of m times hi[k] at ebp-0x20, -0x30, -0x40
        let mut axes = [[0u32; 3]; 3];
        for (k, slot) in [(0u32, 0x20u32), (1, 0x30), (2, 0x40)] {
            let s = x87::load(fpu(h.read(hi + 4 * k)));
            for j in 0..3u32 {
                let mw = x87::load(h.read(m + 0x10 * k + 4 * j));
                let c = x87::store(x87::mul(mw, s));
                axes[k as usize][j as usize] = c;
                h.write(local(slot - 4 * j), c);
            }
        }
        // the raw words the original parks at ebp-0x10..-0x4 (row 2's
        // left last)
        for j in 0..4u32 {
            let w: u32 = h.read(m + 0x20 + 4 * j);
            h.write(local(0x10 - 4 * j), w);
        }
        // the first corner: m * lo
        for j in 0..3u32 {
            let w: u32 = h.read(lo + 4 * j);
            h.write(local(0x60 - 4 * j), fpu(w));
        }
        let r = h.call_sym(TRANSFORM3, Conv::Thiscall(m), &[local(0x70), local(0x60)]);
        let mut p = [0u32; 3];
        for (j, v) in (0u32..).zip(p.iter_mut()) {
            *v = fpu(h.read(r + 4 * j));
            h.write(local(0x50 - 4 * j), *v);
        }
        let [c0, c1, c2] = axes;
        let q0 = sum(p, c0);
        let q1 = sum(p, c1);
        let q01 = sum(q0, c1);
        let q2 = sum(p, c2);
        let q02 = sum(q0, c2);
        let q12 = sum(q1, c2);
        let q012 = sum(q01, c2);
        for (slot, v) in [
            (0x10, q0),
            (0x20, q1),
            (0x30, q01),
            (0x60, q2),
            (0x70, q02),
            (0x80, q12),
            (0x40, q012),
        ] {
            for (j, w) in (0u32..).zip(v) {
                h.write(local(slot - 4 * j), w);
            }
        }
        let r1 = pair(h, VEC3_MIN, local(0xe0), local(0x80), local(0x40));
        let r2 = pair(h, VEC3_MIN, local(0xc0), local(0x60), local(0x70));
        let r3 = pair(h, VEC3_MIN, local(0x120), r2, r1);
        let r4 = pair(h, VEC3_MIN, local(0x100), local(0x20), local(0x30));
        let r5 = pair(h, VEC3_MIN, local(0x90), local(0x50), local(0x10));
        let r6 = pair(h, VEC3_MIN, local(0xb0), r5, r4);
        let r7 = pair(h, VEC3_MIN, local(0xa0), r6, r3);
        for j in 0..3u32 {
            let w: u32 = h.read(r7 + 4 * j);
            h.write(out_lo + 4 * j, fpu(w));
        }
        let m1 = pair(h, VEC3_MAX, local(0xa0), local(0x80), local(0x40));
        let m2 = pair(h, VEC3_MAX, local(0xb0), local(0x60), local(0x70));
        let m3 = pair(h, VEC3_MAX, local(0x90), m2, m1);
        let m4 = pair(h, VEC3_MAX, local(0xd0), local(0x20), local(0x30));
        let m5 = pair(h, VEC3_MAX, local(0xf0), local(0x50), local(0x10));
        let m6 = pair(h, VEC3_MAX, local(0x110), m5, m4);
        let m7 = pair(h, VEC3_MAX, local(0x130), m6, m3);
        for j in 0..3u32 {
            let w: u32 = h.read(m7 + 4 * j);
            h.write(out_hi + 4 * j, fpu(w));
        }
    }

    /// The bytes of the scene key initialize's frame (`ebp - 0x94` up to
    /// `ebp`).
    pub const KEY_FRAME: u32 = 0x94;

    /// `PSceneWideParametersKey::initialize(this; camera, vector,
    /// viewport)` (`0x004fe940`, thiscall, ret 12): the projection
    /// (`camera + 0x30`, depth range moved to 0..1 by `0x00454260` with
    /// the camera's near and far `+0xb4`, `+0xb8`), the camera's 3x4
    /// matrix (`+0`, at `+0x80`), the view-projection (`+0x40`, the
    /// projection times the camera matrix made 4x4), the world matrix
    /// (`camera + 0xb0`'s 3x4, or the identity's) at `+0xb0`, near and
    /// far, the viewport's width and height as floats (unsigned words
    /// `+0x10`, `+0x14`), the vector's three words and its `+0xd0` with
    /// zeros between. Every float through the FPU. Returns `vector` (the
    /// original's last `eax`). `frame` is the original's `ebp - 0x94`.
    #[allow(clippy::too_many_lines)]
    pub fn scene_key_initialize(
        h: &mut impl Host,
        this: u32,
        camera: u32,
        vector: u32,
        viewport: u32,
        frame: u32,
    ) -> u32 {
        let local = |off: u32| frame + 0x94 - off;
        let width: u32 = h.read(viewport + 0x10);
        let height: u32 = h.read(viewport + 0x14);
        let world: u32 = h.read(camera + 0xb0);
        h.write(local(4), height);
        h.call_sym(MATRIX_COPY, Conv::Thiscall(this), &[camera + 0x30]);
        let near = fpu(h.read(camera + 0xb4));
        let far = fpu(h.read(camera + 0xb8));
        h.call_sym(DEPTH_ZERO_ONE, Conv::Cdecl, &[this, near, far]);
        let view = this + 0x80;
        for k in 0..12 {
            let w: u32 = h.read(camera + 4 * k);
            h.write(view + 4 * k, fpu(w));
        }
        let m4 = local(0x54);
        h.call_sym(FROM_3X4, Conv::Thiscall(m4), &[view]);
        let product = h.call_sym(MULTIPLY, Conv::Cdecl, &[local(0x94), this, m4]);
        h.call_sym(MATRIX_COPY, Conv::Thiscall(this + 0x40), &[product]);
        let w34 = local(0x84);
        if world != 0 {
            for k in 0..12 {
                let w: u32 = h.read(world + 4 * k);
                h.write(w34 + 4 * k, w);
            }
        } else {
            // the identity, as a 4x4 at ebp-0x54, made 3x4
            for k in 0..16u32 {
                let one = k % 5 == 0;
                h.write(m4 + 4 * k, if one { 1.0f32 } else { 0.0 });
            }
            h.call_sym(TO_WORLD_3X4, Conv::Thiscall(w34), &[m4]);
        }
        for k in 0..12 {
            let w: u32 = h.read(w34 + 4 * k);
            h.write(this + 0xb0 + 4 * k, fpu(w));
        }
        let near: u32 = h.read(camera + 0xb4);
        h.write(this + 0xe0, fpu(near));
        let far: u32 = h.read(camera + 0xb8);
        h.write(this + 0xe4, fpu(far));
        h.write(this + 0xe8, x87::store(f64::from(width)));
        h.write(this + 0xec, x87::store(f64::from(height)));
        let v: [u32; 3] = [h.read(vector), h.read(vector + 4), h.read(vector + 8)].map(fpu);
        for (k, w) in (0u32..).zip(v) {
            h.write(local(0x14 - 4 * k), w);
            h.write(this + 0xf0 + 4 * k, w);
        }
        h.write(this + 0xfc, 0.0f32);
        let w: u32 = h.read(vector + 0xd0);
        h.write(this + 0x100, fpu(w));
        for off in [0x104, 0x108, 0x10c] {
            h.write(this + off, 0.0f32);
        }
        vector
    }
}

/// The scene key's matrix callees.
const MATRIX_COPY: u32 = 0x0045_3340;
const DEPTH_ZERO_ONE: u32 = 0x0045_4260;
const FROM_3X4: u32 = 0x0045_1080;
const MULTIPLY: u32 = 0x004c_a360;
const TO_WORLD_3X4: u32 = 0x0045_1190;
