//! `PVisibilityCheck` and `PCheckVisibleBoundsJob`: the culling and depth
//! keys of a world's bounds (docs/formats/scene.md, docs/scene-builder.md).
//!
//! - `PVisibilityCheck::PVisibilityCheck` `0x009f2580` (thiscall
//!   `(this, camera, pass)`, `ret 8`, returns `this`): the job list heads
//!   and the depth plane, the z row of `FromMatrix3x4(camera)` (elements 2,
//!   6, 10, 14), negated when `pass >= 0`.
//! - `PCheckVisibleBoundsJob_execute` `0x009f3160` (vtable slot 1,
//!   `ecx` = the job): per `PMeshInstanceBounds` of the job, the box test
//!   on `viewproj . world`, the optional second test (`job +0x3c`), the
//!   entry (instance, key, render order), then the job's entries sorted.
//!
//! The job, as `execute` reads it:
//!
//! | Offset | Field |
//! |---|---|
//! | `+0x10` | first bounds |
//! | `+0x14` | bounds count |
//! | `+0x18` | bounds stride (bytes) |
//! | `+0x1c..+0x28` | the depth plane (x, y, z, w) |
//! | `+0x2c` | entry cursor (set to `+0x30` first) |
//! | `+0x30` | first entry |
//! | `+0x34`, `+0x38` | depth-bias table (12-byte `[bounds, scale, bias]`) and its count |
//! | `+0x3c` | second culler (`0x009f4100`'s `this`), or 0 |
//! | `+0x44` | the view-projection matrix (camera `+0x70`) |
//!
//! A bounds: `+0x00` min, `+0x0c` world (3x4) or 0, `+0x10` size,
//! `+0x1c` mesh instance (0: skipped), `+0x20` render order.
//!
//! x87 at 53 bits: the products are exact, each sum rounds once, the
//! original stores to `f32` where noted ([`ffx_bridge::x87::Reg`]).

use ffx_bridge::Mem;
use ffx_bridge::x87::Reg;
use ffx_graphic::host::{Conv, Host};

/// `PMatrix4_FromMatrix3x4` (thiscall `(out, m3x4)`, returns `out`).
pub const FROM_MATRIX3X4: u32 = 0x0045_1080;
/// `PMatrix4_Multiply` (cdecl `(out, a, b)`, returns `out`).
pub const MULTIPLY: u32 = 0x004c_a360;
/// `PMatrix4_TransformPoint` (thiscall `(m, out, point)`, `ret 8`,
/// returns `out`).
pub const TRANSFORM_POINT: u32 = 0x004c_9eb0;
/// `BoundingBox_IsVisible_00663a30` (thiscall `(bounds, matrix)`, `al`).
pub const BOX_VISIBLE: u32 = 0x0066_3a30;
/// The second culler `0x009f4100` (thiscall `(obj, min, size, matrix)`,
/// `ret 0xc`, `al`).
pub const SECOND_TEST: u32 = 0x009f_4100;
/// `PCheckVisibleBoundsJob_compareEntries`, the sort's comparator.
pub const COMPARE_ENTRIES: u32 = 0x009f_2d70;

/// Bytes of the frame [`bridge::execute`] needs.
pub const EXECUTE_FRAME: u32 = 0x210;
/// Bytes of the frame [`bridge::init`] needs.
pub const INIT_FRAME: u32 = 0x40;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BOX_VISIBLE, COMPARE_ENTRIES, Conv, FROM_MATRIX3X4, Host, MULTIPLY, Mem, Reg, SECOND_TEST,
        TRANSFORM_POINT,
    };
    use crate::sort;

    /// `PVisibilityCheck::PVisibilityCheck(this, camera, pass)`: `frame`
    /// holds the camera's 4x4 matrix ([`super::INIT_FRAME`] bytes).
    pub fn init(h: &mut impl Host, frame: u32, this: u32, camera: u32, pass: i32) -> u32 {
        h.write(this + 4, camera.wrapping_add(0x70));
        h.write(this, camera);
        h.write(this + 0x18, 0u32);
        h.write(this + 0x1c, 0u32);
        h.write(this + 0x20, 0u32);
        h.write(this + 0x24, this + 0x18);
        h.write(this + 0x28, this + 0x1c);
        h.write(this + 0x2c, this + 0x20);
        for off in [0x30, 0x34, 0x38, 0x3c] {
            h.write(this + off, 0u32);
        }
        let fm = h.rebase(FROM_MATRIX3X4);
        h.call(fm, Conv::Thiscall(frame), &[camera]);
        // the z row, copied through the x87 (fld/fstp: only a signalling
        // NaN changes)
        let row: [f32; 4] = [2, 6, 10, 14].map(|i| {
            let v: f32 = h.read(frame + 4 * i);
            Reg::load(v).store()
        });
        for (i, v) in row.iter().enumerate() {
            h.write(this + 8 + 4 * i as u32, Reg::load(*v).store());
        }
        if pass >= 0 {
            for (i, v) in row.iter().enumerate() {
                h.write(this + 8 + 4 * i as u32, (-Reg::load(*v)).store());
            }
        }
        this
    }

    /// Frame layout of [`execute`]: the original's locals at their
    /// offsets from its `ebp - 0x200` (so a test can put the port's frame
    /// where the original's is and see the same pointers).
    const fn local(ebp_off: u32) -> u32 {
        0x200 - ebp_off
    }
    /// `Multiply`'s out, the box test's matrix with a world.
    const PRODUCT: u32 = local(0x1fc);
    /// `FromMatrix3x4(world)` for the box test; the identity's copy for
    /// the second test.
    const WORLD4: u32 = local(0x19c);
    /// The view-projection's copy (no world); `FromMatrix3x4(world)` for
    /// the second test and the key.
    const SCRATCH4: u32 = local(0x15c);
    const IDENTITY: u32 = local(0x11c);
    const POINT_BIASED: u32 = local(0x1ac);
    const POINT: u32 = local(0x1bc);
    const MIN: u32 = local(0x4c);
    const SIZE: u32 = local(0x5c);
    const CENTER: u32 = local(0x18);
    /// The sort's entry (the port's own; the original's sort keeps it in
    /// its frame).
    const TMP: u32 = 0x200;

    /// `PCheckVisibleBoundsJob_execute(job)`. `frame` is
    /// [`super::EXECUTE_FRAME`] bytes for what the original keeps in its
    /// frame and passes by address.
    #[allow(clippy::too_many_lines)] // one original
    pub fn execute(h: &mut impl Host, frame: u32, job: u32) {
        // the identity matrix (fld1/fldz stores)
        for i in 0..16u32 {
            let v = if i % 5 == 0 { 1.0f32 } else { 0.0 };
            h.write(frame + IDENTITY + 4 * i, v);
        }
        let start: u32 = h.read(job + 0x30);
        let viewproj: u32 = h.read(job + 0x44);
        let mut bounds: u32 = h.read(job + 0x10);
        h.write(job + 0x2c, start);
        let fm = h.rebase(FROM_MATRIX3X4);
        let mul = h.rebase(MULTIPLY);
        let tp = h.rebase(TRANSFORM_POINT);
        let visible = h.rebase(BOX_VISIBLE);
        let second = h.rebase(SECOND_TEST);
        let mut i = 0u32;
        while i < h.read::<u32>(job + 0x14) {
            let inst: u32 = h.read(bounds + 0x1c);
            if inst != 0 {
                let rt = |h: &mut _, a: u32| -> f32 {
                    let v: f32 = Mem::read(h, a);
                    Reg::load(v).store()
                };
                let min = [rt(h, bounds), rt(h, bounds + 4), rt(h, bounds + 8)];
                let size = [
                    rt(h, bounds + 0x10),
                    rt(h, bounds + 0x14),
                    rt(h, bounds + 0x18),
                ];
                let world: u32 = h.read(bounds + 0xc);
                for k in 0..3u32 {
                    h.write(frame + MIN + 4 * k, min[k as usize]);
                    h.write(frame + SIZE + 4 * k, size[k as usize]);
                }
                let matrix = if world == 0 {
                    let m: [u32; 16] = h.read(viewproj);
                    h.write(frame + SCRATCH4, m);
                    frame + SCRATCH4
                } else {
                    h.call(fm, Conv::Thiscall(frame + WORLD4), &[world]);
                    h.call(
                        mul,
                        Conv::Cdecl,
                        &[frame + PRODUCT, viewproj, frame + WORLD4],
                    )
                };
                let mut keep = h.call(visible, Conv::Thiscall(bounds), &[matrix]) & 0xff != 0;
                let culler: u32 = h.read(job + 0x3c);
                if keep && culler != 0 {
                    let m = if world == 0 {
                        // the identity, copied to the frame (rep movsd)
                        let id: [u32; 16] = h.read(frame + IDENTITY);
                        h.write(frame + WORLD4, id);
                        frame + WORLD4
                    } else {
                        h.call(fm, Conv::Thiscall(frame + SCRATCH4), &[world])
                    };
                    keep = h.call(
                        second,
                        Conv::Thiscall(culler),
                        &[frame + MIN, frame + SIZE, m],
                    ) & 0xff
                        != 0;
                }
                if keep {
                    entry(h, frame, job, bounds, inst, world, min, size, fm, tp);
                }
            }
            let stride: u32 = h.read(job + 0x18);
            bounds = bounds.wrapping_add(stride);
            i += 1;
        }
        let start: u32 = h.read(job + 0x30);
        let end: u32 = h.read(job + 0x2c);
        let pred = h.rebase(COMPARE_ENTRIES);
        sort::bridge::sort(h, frame + TMP, start, end, sort::count(start, end), pred);
    }

    /// The entry of a visible bounds: its key from the box centre.
    #[allow(clippy::too_many_arguments)] // the original's locals
    fn entry(
        h: &mut impl Host,
        frame: u32,
        job: u32,
        bounds: u32,
        inst: u32,
        world: u32,
        min: [f32; 3],
        size: [f32; 3],
        fm: u32,
        tp: u32,
    ) {
        let half = Reg::constant(0.5);
        let cursor: u32 = h.read(job + 0x2c);
        h.write(cursor, inst);
        let hs = size.map(|s| (Reg::load(s) * half).store());
        let center: [f32; 3] = std::array::from_fn(|k| Reg::load(hs[k]).add_m32(min[k]).store());
        // fld/fstp copies to the centre's frame slots
        let center = center.map(|c| Reg::load(c).store());
        for (k, c) in center.iter().enumerate() {
            h.write(frame + CENTER + 4 * k as u32, *c);
        }
        let count: i32 = h.read(job + 0x38);
        let table: u32 = h.read(job + 0x34);
        let mut bias = 0u32;
        for n in 0..count.max(0) as u32 {
            let e = table.wrapping_add(n * 12);
            if h.read::<u32>(e) == bounds {
                bias = e;
                break;
            }
        }
        let point: [f32; 3] = if world == 0 {
            center
        } else {
            h.call(fm, Conv::Thiscall(frame + SCRATCH4), &[world]);
            let at = if bias == 0 { POINT } else { POINT_BIASED };
            let out = h.call(
                tp,
                Conv::Thiscall(frame + SCRATCH4),
                &[frame + at, frame + CENTER],
            );
            [h.read(out), h.read(out + 4), h.read(out + 8)]
        };
        let rt = |v: f32| Reg::load(v).store();
        let w = rt(h.read(job + 0x28));
        let p = point.map(rt);
        let plane: [f32; 3] = [
            rt(h.read(job + 0x1c)),
            rt(h.read(job + 0x20)),
            rt(h.read(job + 0x24)),
        ];
        let s1 = Reg::load(plane[0]).mul_m32(p[0]).store();
        let s2 = (Reg::load(s1) + Reg::load(plane[1]).mul_m32(p[1])).store();
        let s3 = (Reg::load(s2) + Reg::load(plane[2]).mul_m32(p[2])).store();
        let mut key = Reg::load(w).add_m32(s3).store();
        if bias != 0 {
            let scale: f32 = h.read(bias + 4);
            let add: f32 = h.read(bias + 8);
            key = Reg::load(key).mul_m32(scale).add_m32(add).store();
        }
        let cursor: u32 = h.read(job + 0x2c);
        h.write(cursor + 4, Reg::load(key).store());
        let order: u32 = h.read(bounds + 0x20);
        let cursor: u32 = h.read(job + 0x2c);
        h.write(cursor + 8, order);
        let cursor: u32 = h.read(job + 0x2c);
        h.write(job + 0x2c, cursor + 12);
    }
}
