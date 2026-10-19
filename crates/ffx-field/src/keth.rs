//! `KeThBP`: a Ke thread's belt path, the ring of segments a thread
//! effect (a whip, a tail, a stream) grows along its guide path.
//!
//! A belt path object `o` (`KeThBP_Init`) keeps `n` segments of `0x30`
//! bytes from `+0x40` (`n` at `+0x37`): position `+0x00` (xyz), a weight
//! `+0x0c`, the children list `+0x18`, a curl `+0x1c`, direction `+0x20`.
//! Pointers go through `set_ptr32`/`get_ptr32_func`, the identity on PC.

use ffx_bridge::Mem;

use crate::camera::FArgs;
use crate::ke::bridge::ke_th_bp_init;
use crate::x87;

/// The update's VU0 scratch (`0x010d5530`..): the rotation matrix, the
/// direction to the next path point, the growth direction, the head
/// position and the path point, the step lengths, and the copies the
/// children follow.
mod scratch {
    pub const ROT: u32 = 0x010d_5530;
    pub const TO_POINT: u32 = 0x010d_5570;
    pub const DIR: u32 = 0x010d_5580;
    pub const HEAD: u32 = 0x010d_5590;
    pub const POINT: u32 = 0x010d_55a0;
    /// The step length (`f32`, the template's `+0x20`).
    pub const STEP: u32 = 0x010d_55b0;
    /// The distance to the path point.
    pub const DIST: u32 = 0x010d_55b4;
    /// The steering weight this step, and its base (the template's `+0x2c`).
    pub const STEER: u32 = 0x010d_55b8;
    pub const STEER_BASE: u32 = 0x010d_55bc;
    /// The path point's `+0x10`, `+0x18`, `+0x14`, `+0x0c`: the radius the
    /// steering starts at, the radius it is full at, its weight there,
    /// and the reach flag (0: pass the point, else reach it).
    pub const POINT_FAR: u32 = 0x010d_55c0;
    pub const POINT_NEAR: u32 = 0x010d_55c4;
    pub const POINT_WEIGHT: u32 = 0x010d_55c8;
    pub const POINT_REACH: u32 = 0x010d_55cc;
    pub const PREV_DIR: u32 = 0x010d_55d0;
    pub const PREV_HEAD: u32 = 0x010d_55e0;
    pub const STEP_VEC: u32 = 0x010d_55f0;
}

/// `FFXVu0CopyVector(dst, src)`.
pub const COPY_VECTOR: u32 = 0x006e_d370;
/// `KeVu0CopyVecXYZ(dst, src)`.
pub const COPY_XYZ: u32 = 0x0075_ede0;
/// `FFXVu0SubVector(dst, a, b)`.
pub const SUB_VECTOR: u32 = 0x006e_d940;
/// `FFXVu0AddVector(dst, a, b)`.
pub const ADD_VECTOR: u32 = 0x006e_d290;
/// `FFXVu0ScaleVector(dst, v, s)`.
pub const SCALE_VECTOR: u32 = 0x006e_d8f0;
/// `KeVu0VecLen(v)` (in `ST0`).
pub const VEC_LEN: u32 = 0x0075_fda0;
/// `FFXVu0Normalize(dst, src)`.
pub const NORMALIZE: u32 = 0x006e_d6f0;
/// `FFXVu0ApplyMatrix(dst, m, v)`.
pub const APPLY_MATRIX: u32 = 0x006e_d2e0;
/// `KeVu0RndI()`.
pub const RND_I: u32 = 0x0075_f920;
/// The three rotations `(out, p, angle)` a step's wobble picks from
/// (`0x00c3e730`, by the random number mod 3).
pub const ROTATIONS: u32 = 0x00c3_e730;

/// The scratch [`ke_th_bp_update`] needs: `KeThBP_Init`'s arguments.
pub const WORK: u32 = 8;

/// The flag bits of `o + 0x3e` the update reads.
mod flag {
    /// Grow even when the head stands still (the old `0x800`, kept).
    pub const HELD: u32 = 0x40;
    pub const FROZEN: u32 = 2;
    pub const CAPPED: u32 = 4;
    pub const DONE: u32 = 8;
    pub const DEAD: u32 = 0x10;
    pub const RUNNING: u32 = 0x20;
    /// When the cap is reached, keep running.
    pub const LOOP: u32 = 0x80;
    pub const START: u32 = 0x100;
    pub const NORMALISE: u32 = 0x400;
    pub const PATH_END: u32 = 0x800;
    pub const STEERING: u32 = 0x2000;
}

/// The template a belt path follows (`p + 0x4c`).
mod tmpl {
    pub const STEP: u32 = 0x20;
    pub const FOLLOW: u32 = 0x24;
    pub const WOBBLE: u32 = 0x28;
    pub const STEER: u32 = 0x2c;
    pub const SPEED: u32 = 0x34;
    pub const CURL: u32 = 0x3c;
    pub const MAX_CHILDREN: u32 = 0x40;
    pub const CHILD_MASK: u32 = 0x41;
    pub const MAX_STEPS: u32 = 0x42;
}

/// The step's `(ld, st)` x87 helpers.
fn ld(bits: u32) -> f64 {
    x87::load(bits)
}

fn st(x: f64) -> u32 {
    x87::store(x)
}

/// An integer through `fild` and a stored `f32`.
fn int_f32(v: i32) -> f64 {
    ld(st(f64::from(v)))
}

/// One update of the belt path `p + 0x40` (`KeThBP_Update(p)`, `0x007637f0`).
///
/// The head's time `o + 0x18` moves on by the template's speed; the whole
/// steps passed (capped by the template's `+0x42` unless running) are
/// walked along the guide path (`o + 0x14`, path points linked by
/// `+0x1c`): each step the head turns toward the next point (by the
/// steering weight, full inside the point's near radius), wobbles by a
/// random rotation, follows the segment it replaces by `+0x24`, and moves
/// one step length. Segments the head passes are reused; a new one gets a
/// random curl, may spawn a child belt path from the pool `p + 0x48`
/// (`KeThBP_Init`), and is linked to it. Returns the original's `eax`.
#[allow(clippy::too_many_lines)] // one pass, as the original
pub fn ke_th_bp_update<M, C, F>(mem: &mut M, p: u32, work: u32, call: &mut C, fcall: &mut F) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
    F: FnMut(&mut M, u32, FArgs<'_>) -> f64,
{
    let o = mem.read::<u32>(p + 0x40);
    let t = mem.read::<u32>(p + 0x4c);
    let mut f = u32::from(mem.read::<u16>(o + 0x3e));
    let normalise = f & flag::NORMALISE != 0;
    if f & flag::PATH_END != 0 {
        f = (f & !flag::PATH_END) | flag::HELD;
    }
    if f & flag::DONE != 0 {
        if f & flag::RUNNING == 0 {
            #[allow(clippy::cast_possible_truncation)] // the flag word
            mem.write(o + 0x3e, (f | flag::DEAD) as u16);
            return f | flag::DEAD;
        }
        f = (f & !flag::DONE) | flag::START;
    }

    // the head's time: whole steps passed
    let mut time = x87::copy(mem.read::<u32>(o + 0x18));
    let mut counter = u32::from(mem.read::<u16>(o + 0x1c));
    let start = ffx_bridge::x87::ftol(mem, ld(time)).cast_signed();
    let n = u32::from(mem.read::<u8>(o + 0x37));
    let child_mask = u32::from(mem.read::<u8>(t + tmpl::CHILD_MASK));
    let mut may_spawn =
        child_mask != 0xff && mem.read::<u8>(o + 0x1e) < mem.read::<u8>(t + tmpl::MAX_CHILDREN);
    let x = ld(mem.read::<u32>(t + tmpl::SPEED)) + ld(time);
    let mut d = ffx_bridge::x87::ftol(mem, x)
        .cast_signed()
        .wrapping_sub(start);
    let start_f = int_f32(start);
    // a step all but complete counts
    let frac = ld(st(x - start_f - int_f32(d)));
    if (0.999_999..=1.0).contains(&frac) {
        d += 1;
    }
    let mut steps = d;
    let mut walk = d; // steps the head walks this update
    if f & flag::FROZEN != 0 {
        walk = 0;
    } else {
        time = st(x);
        if f & flag::CAPPED == 0 {
            let cap = i32::from(mem.read::<u8>(t + tmpl::MAX_STEPS));
            let now = ffx_bridge::x87::ftol(mem, ld(time)).cast_signed();
            if now >= cap {
                if f & flag::LOOP != 0 {
                    f |= flag::RUNNING;
                }
                steps = now - cap;
                if f & flag::RUNNING == 0 {
                    time = st(f64::from(cap));
                    walk -= steps;
                } else {
                    f = (f & !0x200) | flag::CAPPED;
                }
            }
        }
    }

    let running = f & flag::RUNNING;
    let mut point;
    if running == 0 {
        steps = 0;
        if f & flag::START != 0 {
            point = mem.read::<u32>(p + 0x44);
            mem.write(o + 0x14, point);
        } else {
            point = mem.read::<u32>(o + 0x14);
        }
    } else {
        #[allow(clippy::cast_sign_loss)] // the original's 32-bit add
        {
            counter = counter.wrapping_add(steps as u32);
        }
        point = mem.read::<u32>(o + 0x14);
        if point == 0 || start <= counter.cast_signed() {
            let w = mem.read::<u16>(o + 0x3e) | 0x10;
            mem.write(o + 0x3e, w);
            return point;
        }
    }

    // the segments the tail drops this update lose their children
    let mut cur = u32::from(mem.read::<u16>(o + 0x34));
    let seg_at = |cur: u32| o + 0x40 + cur * 0x30;
    let mut seg = seg_at(cur);
    let drop_mark = running | flag::DONE;
    let mut left = steps;
    while left > 0 {
        detach_children(mem, seg, drop_mark);
        cur += 1;
        if cur == n {
            cur = 0;
        }
        seg = seg_at(cur);
        left -= 1;
    }
    #[allow(clippy::cast_possible_truncation)] // the ring index
    mem.write(o + 0x34, cur as u16);
    let len = st(int_f32(steps) + ld(mem.read::<u32>(o + 0x30)));
    mem.write(o + 0x30, len);

    // the VU0 scratch: step length, steering, the path point, the head
    let r = |mem: &M, a: u32| mem.rebase(a);
    mem.write(
        r(mem, scratch::STEP),
        x87::copy(mem.read::<u32>(t + tmpl::STEP)),
    );
    mem.write(
        r(mem, scratch::STEER_BASE),
        x87::copy(mem.read::<u32>(t + tmpl::STEER)),
    );
    load_point(mem, point);
    let (vpoint, vdir, vhead) = (
        r(mem, scratch::POINT),
        r(mem, scratch::DIR),
        r(mem, scratch::HEAD),
    );
    call(mem, r(mem, COPY_VECTOR), &[vpoint, point]);
    call(mem, r(mem, COPY_XYZ), &[vdir, seg + 0x20]);
    mem.write(vdir + 0xc, 0u32);
    call(mem, r(mem, COPY_VECTOR), &[vhead, seg]);
    let first_new = start.wrapping_sub(counter.cast_signed());
    let last = walk.wrapping_add(first_new);
    let mut step = 0i32;

    let vu = Vu { p, t, normalise };
    if f & flag::START != 0 && running == 0 {
        // the first step's direction: toward the point, wobbled, along
        // the segment's own
        Vu::measure(mem, call, fcall);
        vu.steer(mem, call);
        vu.wobble(mem, call);
        vu.follow(mem, call, seg);
    }

    let mut prev_seg: u32;
    let mut path_end = false;
    loop {
        // the next path point once this one is reached (or passed)
        let dist = Vu::measure(mem, call, fcall);
        let reach = ld(mem.read::<u32>(r(mem, scratch::POINT_REACH)));
        let near = ld(mem.read::<u32>(point + 0x18));
        let arrived = if reach == 0.0 {
            near > ld(dist)
        } else {
            near < ld(dist)
        };
        if arrived {
            let next = mem.read::<u32>(point + 0x1c);
            if next == 0 {
                path_end = true;
                break;
            }
            point = next;
            call(mem, r(mem, COPY_VECTOR), &[r(mem, scratch::POINT), point]);
            if step == 0 {
                mem.write(o + 0x14, point);
            }
            load_point(mem, point);
        }
        step += 1;
        if step > last {
            break;
        }

        // one step: the ring moves on, the head with it
        prev_seg = seg;
        cur += 1;
        if cur == n {
            cur = 0;
        }
        seg = seg_at(cur);
        let (prev_head, prev_dir) = (r(mem, scratch::PREV_HEAD), r(mem, scratch::PREV_DIR));
        call(
            mem,
            r(mem, COPY_VECTOR),
            &[prev_head, r(mem, scratch::HEAD)],
        );
        call(mem, r(mem, COPY_XYZ), &[prev_dir, r(mem, scratch::DIR)]);
        let (vstep, vdir, vhead) = (
            r(mem, scratch::STEP_VEC),
            r(mem, scratch::DIR),
            r(mem, scratch::HEAD),
        );
        let len = x87::copy(mem.read::<u32>(r(mem, scratch::STEP)));
        call(mem, r(mem, SCALE_VECTOR), &[vstep, vdir, len]);
        call(mem, r(mem, ADD_VECTOR), &[vhead, vhead, vstep]);
        call(mem, r(mem, COPY_XYZ), &[seg, vhead]);
        if vu.steer(mem, call) {
            f |= flag::STEERING;
        } else {
            f &= !flag::STEERING;
        }
        let rnd = vu.wobble(mem, call);
        // the segment's curl: the previous one's plus a random amount
        let curl =
            st((int_f32(rnd) * 3.051_757_812_5e-5 * 2.0 - 1.0)
                * ld(mem.read::<u32>(t + tmpl::CURL)));
        let curl = clamp01(st(ld(curl) + ld(mem.read::<u32>(prev_seg + 0x1c))));
        if step > first_new {
            // a new segment
            mem.write(seg + 0x18, 0u32);
            mem.write(seg + 0xc, 0x3f80_0000u32);
            mem.write(seg + 0x1c, curl);
            #[allow(clippy::cast_sign_loss)] // a bit mask
            if may_spawn && rnd.cast_unsigned() & child_mask == 0 {
                let at =
                    st(ld(mem.read::<u32>(o + 0x30)) + start_f + int_f32(step)
                        - int_f32(first_new));
                if !spawn_child(
                    mem,
                    call,
                    Spawn {
                        p,
                        o,
                        seg,
                        prev_seg,
                        work,
                        point,
                        at,
                    },
                ) {
                    may_spawn = false;
                }
                may_spawn &= mem.read::<u8>(o + 0x1e) < mem.read::<u8>(t + tmpl::MAX_CHILDREN);
            }
        } else {
            // a reused segment: follow it, ease its curl, move its children
            vu.follow(mem, call, seg);
            let s1c = ld(mem.read::<u32>(seg + 0x1c));
            let eased = st((ld(curl) - s1c) * ld(mem.read::<u32>(t + tmpl::FOLLOW)) + s1c);
            mem.write(seg + 0x1c, eased);
            let mut c = mem.read::<u32>(seg + 0x18);
            while c != 0 {
                let cs = c + 0x40 + u32::from(mem.read::<u16>(c + 0x34)) * 0x30;
                call(mem, r(mem, COPY_XYZ), &[cs, r(mem, scratch::PREV_HEAD)]);
                call(
                    mem,
                    r(mem, COPY_XYZ),
                    &[cs + 0x20, r(mem, scratch::PREV_DIR)],
                );
                mem.write(cs + 0x1c, x87::copy(mem.read::<u32>(seg + 0x1c)));
                mem.write(c + 0x14, point);
                c = mem.read::<u32>(c + 8);
            }
        }
        call(mem, r(mem, COPY_XYZ), &[seg + 0x20, r(mem, scratch::DIR)]);
    }

    #[allow(clippy::cast_possible_truncation)] // the ring index byte
    mem.write(o + 0x36, cur as u8);
    if path_end {
        // the guide path ends: the head stops, the rest of the tail drops
        #[allow(clippy::cast_sign_loss)] // the original's 32-bit add
        let at = counter.wrapping_add(step as u32);
        time = st(f64::from(at.cast_signed()));
        f |= flag::PATH_END;
        let mark = (f & flag::RUNNING) | flag::DONE;
        let mut k = step + 1;
        while k <= first_new {
            cur += 1;
            if cur == n {
                cur = 0;
            }
            detach_children(mem, seg_at(cur), mark);
            k += 1;
        }
        if f & flag::LOOP != 0 {
            f |= flag::RUNNING;
        }
    }

    // the drawn span: from the tail index +0x34 to the head +0x36
    let tail = u32::from(mem.read::<u16>(o + 0x34));
    let last_seg = u32::from(mem.read::<u8>(o + 0x36));
    let (span, first) = match tail.cmp(&last_seg) {
        core::cmp::Ordering::Less => (last_seg - tail + 1, 0),
        core::cmp::Ordering::Greater => (n.wrapping_sub(tail), last_seg + 1),
        core::cmp::Ordering::Equal => (0, 0),
    };
    #[allow(clippy::cast_possible_truncation)] // 16-bit fields
    {
        mem.write(o + 0x38, span as u16);
        mem.write(o + 0x3c, first as u16);
        mem.write(o + 0x18, time);
        mem.write(o + 0x1c, counter as u16);
        mem.write(o + 0x3e, f as u16);
    }
    counter
}

/// A curl clamped to 0..=1 (NaN kept).
fn clamp01(v: u32) -> u32 {
    if 1.0 < ld(v) {
        0x3f80_0000
    } else if ld(v) < 0.0 {
        0
    } else {
        v
    }
}

/// The path point's radii and weight into the scratch (`fld`/`fstp`).
fn load_point<M: Mem>(mem: &mut M, point: u32) {
    for (dst, src) in [
        (scratch::POINT_FAR, 0x10),
        (scratch::POINT_NEAR, 0x18),
        (scratch::POINT_WEIGHT, 0x14),
        (scratch::POINT_REACH, 0xc),
    ] {
        let v = x87::copy(mem.read::<u32>(point + src));
        let a = mem.rebase(dst);
        mem.write(a, v);
    }
}

/// The belt paths hanging off `seg` (`+0x18`, linked by `+8`) let go:
/// `mark` into their flags, their link `+0xc` cleared.
fn detach_children<M: Mem>(mem: &mut M, seg: u32, mark: u32) {
    let mut c = mem.read::<u32>(seg + 0x18);
    while c != 0 {
        #[allow(clippy::cast_possible_truncation)] // the flag word
        let w = mem.read::<u16>(c + 0x3e) | mark as u16;
        mem.write(c + 0x3e, w);
        mem.write(c + 0xc, 0u32);
        c = mem.read::<u32>(c + 8);
    }
}

/// The VU0 steps of the head's direction, over the scratch.
struct Vu {
    p: u32,
    t: u32,
    normalise: bool,
}

impl Vu {
    /// The vector to the path point (`TO_POINT`) and its length (`DIST`).
    fn measure<M, C, F>(mem: &mut M, call: &mut C, fcall: &mut F) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
        F: FnMut(&mut M, u32, FArgs<'_>) -> f64,
    {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let (vto, vpoint, vhead) = (
            r(mem, scratch::TO_POINT),
            r(mem, scratch::POINT),
            r(mem, scratch::HEAD),
        );
        call(mem, r(mem, SUB_VECTOR), &[vto, vpoint, vhead]);
        let f_len = r(mem, VEC_LEN);
        let dist = st(fcall(mem, f_len, FArgs::Stack(&[vto])));
        mem.write(r(mem, scratch::DIST), dist);
        dist
    }

    /// Turn the direction toward the point: the steering weight is the
    /// template's, plus the point's weight ramping in from its far radius
    /// to its near one while the head is inside the far radius. Returns
    /// whether it is.
    fn steer<M, C>(&self, mem: &mut M, call: &mut C) -> bool
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let g = |mem: &M, a: u32| ld(mem.read::<u32>(mem.rebase(a)));
        let base = mem.read::<u32>(r(mem, scratch::STEER_BASE));
        mem.write(r(mem, scratch::STEER), x87::copy(base));
        let (dist, far) = (g(mem, scratch::DIST), g(mem, scratch::POINT_FAR));
        let inside = far > dist;
        if inside {
            let (near, weight) = (g(mem, scratch::POINT_NEAR), g(mem, scratch::POINT_WEIGHT));
            let k = ld(base) + (weight - (dist - near) * weight / (far - near));
            mem.write(r(mem, scratch::STEER), st(k));
        }
        let vto = r(mem, scratch::TO_POINT);
        call(mem, r(mem, NORMALIZE), &[vto, vto]);
        let k = g(mem, scratch::STEER);
        self.blend(mem, call, k, vto);
        inside
    }

    /// The direction wobbled by one of three rotations (random) by a
    /// random angle up to the template's `+0x28`. Returns the random
    /// number.
    fn wobble<M, C>(&self, mem: &mut M, call: &mut C) -> i32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let rnd = call(mem, r(mem, RND_I), &[]).cast_signed();
        let axis = (rnd % 3).cast_unsigned();
        let jitter = rnd % 1001 - 500;
        let angle = st(int_f32(jitter) * ld(mem.read::<u32>(self.t + tmpl::WOBBLE)) / 500.0);
        let rot = mem.read::<u32>(r(mem, ROTATIONS).wrapping_add(axis.wrapping_mul(4)));
        let (m, vdir) = (r(mem, scratch::ROT), r(mem, scratch::DIR));
        call(mem, rot, &[m, self.p, angle]);
        call(mem, r(mem, APPLY_MATRIX), &[vdir, m, vdir]);
        rnd
    }

    /// The direction pulled toward the segment's own (`seg + 0x20`) by
    /// `1 - +0x24`.
    fn follow<M, C>(&self, mem: &mut M, call: &mut C, seg: u32)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let keep = ld(x87::copy(mem.read::<u32>(self.t + tmpl::FOLLOW)));
        let (vdir, w) = (mem.rebase(scratch::DIR), st(1.0 - keep));
        for i in 0..3 {
            let v = ld(mem.read::<u32>(vdir + 4 * i));
            let s = ld(mem.read::<u32>(seg + 0x20 + 4 * i));
            mem.write(vdir + 4 * i, st(keep * v + ld(w) * s));
        }
        if self.normalise {
            call(mem, mem.rebase(NORMALIZE), &[vdir, vdir]);
        }
    }

    /// `dir = (1 - k) dir + k toward` (xyz), then normalised if asked.
    fn blend<M, C>(&self, mem: &mut M, call: &mut C, k: f64, toward: u32)
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let (vdir, w) = (mem.rebase(scratch::DIR), st(1.0 - k));
        for i in 0..3 {
            let v = ld(mem.read::<u32>(vdir + 4 * i));
            let n = ld(mem.read::<u32>(toward + 4 * i));
            mem.write(vdir + 4 * i, st(ld(w) * v + n * k));
        }
        if self.normalise {
            call(mem, mem.rebase(NORMALIZE), &[vdir, vdir]);
        }
    }
}

/// A child belt path to spawn.
#[derive(Clone, Copy)]
struct Spawn {
    p: u32,
    o: u32,
    seg: u32,
    prev_seg: u32,
    work: u32,
    point: u32,
    /// Its length `+0x30`.
    at: u32,
}

/// A child belt path off the new segment: a block from the pool
/// `p + 0x48` (free list `+4`) set up by `KeThBP_Init` with the pool's
/// segment count, the parent's `0x1400` flags, path point and child
/// count, starting at the previous head along the previous direction,
/// linked into the parent's list (`+0`/`+4`) and the segment's
/// (`+0x18`, through the child's `+8`/`+0xc`). `false` when the pool is
/// empty.
fn spawn_child<M, C>(mem: &mut M, call: &mut C, s: Spawn) -> bool
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    let pool = mem.read::<u32>(s.p + 0x48);
    let nb = mem.read::<u32>(pool + 4);
    if nb == 0 {
        return false;
    }
    mem.write(pool + 4, mem.read::<u32>(nb));
    let count = mem.read::<u8>(s.o + 0x1e).wrapping_add(1);
    mem.write(s.o + 0x1e, count);
    mem.write(s.work, nb);
    mem.write(s.work + 4, mem.read::<u8>(pool + 1));
    ke_th_bp_init(mem, s.work);
    let flags = mem.read::<u16>(nb + 0x3e) | (mem.read::<u16>(s.o + 0x3e) & 0x1400);
    mem.write(nb + 0x3e, flags);
    mem.write(nb + 0x14, s.point);
    mem.write(nb + 0x1e, count);
    mem.write(nb + 0x30, s.at);
    let r = |mem: &M, a: u32| mem.rebase(a);
    call(
        mem,
        r(mem, COPY_XYZ),
        &[nb + 0x40, r(mem, scratch::PREV_HEAD)],
    );
    call(
        mem,
        r(mem, COPY_XYZ),
        &[nb + 0x60, r(mem, scratch::PREV_DIR)],
    );
    mem.write(nb + 0x5c, x87::copy(mem.read::<u32>(s.prev_seg + 0x1c)));
    // the parent's list
    let head = mem.read::<u32>(s.o);
    if head != 0 {
        mem.write(head + 4, nb);
    }
    mem.write(nb, head);
    mem.write(nb + 4, s.o);
    mem.write(s.o, nb);
    // the segment's list
    let head = mem.read::<u32>(s.seg + 0x18);
    if head != 0 {
        mem.write(head + 0xc, nb + 8);
    }
    mem.write(nb + 8, head);
    mem.write(nb + 0xc, s.seg + 0x18);
    mem.write(s.seg + 0x18, nb);
    true
}

/// The strip [`ke_th_bp_mk_strip`] builds and the state it keeps between
/// path points (`0x0230f9c8`..).
pub mod strip {
    /// The walk: `+4` the next path point (xyzw, `+0x1c` its age), `+0x14`
    /// the step between points, `+0x1c` the points left.
    pub const STATE: u32 = 0x0230_f9c8;
    /// `1 / w` of the next point, and of the last one.
    pub const INV_W: u32 = 0x0230_f9cc;
    pub const PREV_INV_W: u32 = 0x0230_f9d0;
    /// The direction (x, y) to the next point, and the last one.
    pub const DIR: u32 = 0x0230_f9d8;
    pub const PREV_DIR: u32 = 0x0230_f9e0;
    /// The side offset (x, y) at the strip's first end.
    pub const OFFSET: u32 = 0x0230_f9e8;
    /// `1 - age * rate`, and the rate.
    pub const FADE: u32 = 0x0230_f9f0;
    pub const FADE_RATE: u32 = 0x0230_f9f4;
    /// The inner edge's share of the width.
    pub const INNER: u32 = 0x0230_f9f8;
    /// The half widths (x, y), and their scale.
    pub const WIDTH: u32 = 0x0230_fa00;
    pub const WIDTH_SCALE: u32 = 0x0230_fa0c;
    /// The colour scale, the two colours' steps per point, the colours
    /// (edge, centre).
    pub const COLOR_SCALE: u32 = 0x0230_fa10;
    pub const EDGE_STEP: u32 = 0x0230_fa20;
    pub const CENTRE_STEP: u32 = 0x0230_fa30;
    pub const EDGE: u32 = 0x0230_fa40;
    pub const CENTRE: u32 = 0x0230_fa50;
    /// The vertex built at the strip's second end, and at its first.
    pub const SIDE1: u32 = 0x0230_fa60;
    pub const SIDE0: u32 = 0x0230_fa70;
    /// The next point (projected, over w), the strip's second end and
    /// its first.
    pub const NEXT: u32 = 0x0230_fa80;
    pub const END1: u32 = 0x0230_fa90;
    pub const END0: u32 = 0x0230_faa0;
    /// The projection, and the matrix the vertices go back through.
    pub const PROJECT: u32 = 0x0230_fac0;
    pub const UNPROJECT: u32 = 0x0230_fb00;
    /// Ten vertices (xyzw) a point, and their colours `0x5000` on.
    pub const VERTS: u32 = 0x0230_5908;
    pub const COLORS: u32 = VERTS + 0x5000;
}

/// `FFXVu0MulVector(dst, a, b)`.
pub const MUL_VECTOR: u32 = 0x006e_d5d0;
/// `KeAddScaledColor(out, base, step, k)`.
pub const ADD_SCALED_COLOR: u32 = 0x0076_4c40;

/// The scratch [`ke_th_bp_mk_strip`] needs: four colours and a vertex.
pub const STRIP_WORK: u32 = 0x50;

/// The four colours of a point's vertices (scaled, over 256 when
/// stored): the edge colour at the first end and at the second, the
/// centre colour at both.
#[derive(Clone, Copy)]
struct StripColors {
    edge0: u32,
    edge1: u32,
    centre0: u32,
    centre1: u32,
}

/// One vertex of the strip: where it is (its end and how far along the
/// side offset, `None` for the end itself) and its colour (alpha from
/// the colour or 0).
struct StripVertex {
    second_end: bool,
    side: Option<(f64, bool)>,
    color: fn(&StripColors) -> u32,
    alpha: bool,
}

/// The ten vertices of a point: the outer edge, the inner edge, the
/// centre, the other inner and outer edge, each at both ends. `side`:
/// (share of the offset, inner), negative on the far side.
const STRIP: [StripVertex; 10] = {
    const fn v(
        second_end: bool,
        side: Option<(f64, bool)>,
        color: fn(&StripColors) -> u32,
        alpha: bool,
    ) -> StripVertex {
        StripVertex {
            second_end,
            side,
            color,
            alpha,
        }
    }
    [
        v(false, Some((1.0, false)), |c| c.edge0, false),
        v(true, Some((1.0, false)), |c| c.edge1, false),
        v(false, Some((1.0, true)), |c| c.edge0, true),
        v(true, Some((1.0, true)), |c| c.edge1, true),
        v(false, None, |c| c.centre0, true),
        v(true, None, |c| c.centre1, true),
        v(false, Some((-1.0, true)), |c| c.edge0, true),
        v(true, Some((-1.0, true)), |c| c.edge1, true),
        v(false, Some((-1.0, false)), |c| c.edge0, false),
        v(true, Some((-1.0, false)), |c| c.edge1, false),
    ]
};

/// `1 / sqrt(x² + y²)` stored as `f32`, or 0 for a zero length.
fn inv_len(x: f64, y: f64) -> f64 {
    let len = ld(st((x * x + y * y).sqrt()));
    if len == 0.0 { len } else { ld(st(1.0 / len)) }
}

/// The belt path's strip, point by point (`KeThBP_MkStrip_00764de0()`,
/// `0x00764de0`, called by `KeThBP_MkGifPk`).
///
/// For each point left in the walk ([`strip::STATE`]): the point goes
/// through the projection and over its w ([`strip::NEXT`]); the edge and
/// centre colours step on (`KeAddScaledColor`, scaled before and after);
/// the side offset at the strip's second end is the bisector of the last
/// and the new direction turned a quarter, times the half widths, the
/// width scale, the fade (`1 - age * rate`) and the last point's `1 /
/// w`. Ten vertices go out between the strip's two ends ([`STRIP`]),
/// each times its w and back through [`strip::UNPROJECT`], with their
/// colours over 256; then the ends move on. Returns the walk's address
/// (the original's `eax`).
#[allow(clippy::too_many_lines)] // one pass, as the original
pub fn ke_th_bp_mk_strip<M, C>(mem: &mut M, work: u32, call: &mut C) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    use strip::{COLOR_SCALE, DIR, END0, END1, NEXT, OFFSET, PREV_DIR, PREV_INV_W, SIDE0, SIDE1};
    let r = |mem: &M, a: u32| mem.rebase(a);
    let g = |mem: &M, a: u32| ld(mem.read::<u32>(mem.rebase(a)));
    let put = |mem: &mut M, a: u32, x: f64| {
        let at = mem.rebase(a);
        mem.write(at, st(x));
    };
    let copy = |mem: &mut M, dst: u32, src: u32| {
        let v = x87::copy(mem.read::<u32>(mem.rebase(src)));
        let at = mem.rebase(dst);
        mem.write(at, v);
    };
    let state_at = r(mem, strip::STATE);
    let mut out = r(mem, strip::VERTS);
    let colors = [work, work + 0x10, work + 0x20, work + 0x30];
    let vertex = work + 0x40;
    loop {
        let state = mem.read::<u32>(state_at);
        if mem.read::<i32>(state + 0x1c) <= 0 {
            return state;
        }
        call(mem, r(mem, COPY_VECTOR), &[r(mem, SIDE0), r(mem, END0)]);
        call(mem, r(mem, COPY_VECTOR), &[r(mem, SIDE1), r(mem, END1)]);

        // the next point, projected
        let point = mem.read::<u32>(state + 4);
        let age = ld(mem.read::<u32>(point + 0x1c));
        put(mem, strip::FADE, 1.0 - age * g(mem, strip::FADE_RATE));
        call(mem, r(mem, COPY_VECTOR), &[r(mem, NEXT), point]);
        let (next, project) = (r(mem, NEXT), r(mem, strip::PROJECT));
        call(mem, r(mem, APPLY_MATRIX), &[next, project, next]);
        let w = g(mem, NEXT + 0xc);
        put(mem, strip::INV_W, if w == 0.0 { 0.0 } else { 1.0 / w });
        let inv_w = g(mem, strip::INV_W);
        for i in 0..3 {
            put(mem, NEXT + 4 * i, g(mem, NEXT + 4 * i) * inv_w);
        }

        // the colours, before and after their steps
        let scale = r(mem, COLOR_SCALE);
        for (base, step, before, after) in [
            (strip::EDGE, strip::EDGE_STEP, colors[0], colors[1]),
            (strip::CENTRE, strip::CENTRE_STEP, colors[2], colors[3]),
        ] {
            let (base, step) = (r(mem, base), r(mem, step));
            call(mem, r(mem, MUL_VECTOR), &[before, base, scale]);
            call(
                mem,
                r(mem, ADD_SCALED_COLOR),
                &[base, base, step, 0x3f80_0000],
            );
            call(mem, r(mem, MUL_VECTOR), &[after, base, scale]);
        }
        let c = StripColors {
            edge0: colors[0],
            edge1: colors[1],
            centre0: colors[2],
            centre1: colors[3],
        };

        // the direction, and the side offset along the bisector
        put(mem, DIR, g(mem, NEXT) - g(mem, END1));
        put(mem, DIR + 4, g(mem, NEXT + 4) - g(mem, END1 + 4));
        let k = inv_len(g(mem, DIR), g(mem, DIR + 4));
        put(mem, DIR, g(mem, DIR) * k);
        put(mem, DIR + 4, k * g(mem, DIR + 4));
        let bx = ld(st(g(mem, PREV_DIR) + g(mem, DIR)));
        let by = ld(st(g(mem, PREV_DIR + 4) + g(mem, DIR + 4)));
        let k = inv_len(bx, by);
        let (nx, ny) = (ld(st(bx * k)), ld(st(k * by)));
        let (width, fade) = (g(mem, strip::WIDTH_SCALE), g(mem, strip::FADE));
        let prev_inv_w = g(mem, PREV_INV_W);
        let ox = st(ny * width * fade * prev_inv_w * g(mem, strip::WIDTH));
        let oy = st(-nx * width * fade * prev_inv_w * g(mem, strip::WIDTH + 4));
        copy(mem, PREV_DIR, DIR);
        copy(mem, PREV_DIR + 4, DIR + 4);
        copy(mem, PREV_INV_W, strip::INV_W);

        // the ten vertices
        let inner = g(mem, strip::INNER);
        let offset0 = (g(mem, OFFSET), g(mem, OFFSET + 4));
        let offset1 = (ld(ox), ld(oy));
        for (i, v) in (0u32..).zip(&STRIP) {
            let (end, side, offset) = if v.second_end {
                (END1, SIDE1, offset1)
            } else {
                (END0, SIDE0, offset0)
            };
            let src = if let Some((sign, in_edge)) = v.side {
                for (j, o) in [(0, offset.0), (4, offset.1)] {
                    let e = g(mem, end + j);
                    let o = if in_edge { o * inner } else { o };
                    put(mem, side + j, if sign < 0.0 { e - o } else { e + o });
                }
                side
            } else {
                end
            };
            // times its w, back through the matrix
            let [x, y, z, w] = mem.read::<[u32; 4]>(r(mem, src)).map(ld);
            mem.write(vertex, [st(w * x), st(w * y), st(z * w), st(w)]);
            call(
                mem,
                r(mem, APPLY_MATRIX),
                &[vertex, r(mem, strip::UNPROJECT), vertex],
            );
            let at = out + 0x10 * i;
            let pos = mem.read::<[u32; 4]>(vertex).map(x87::copy);
            mem.write(at, pos);
            let rgba = mem.read::<[u32; 4]>((v.color)(&c)).map(ld);
            let alpha = if v.alpha { st(rgba[3] / 256.0) } else { 0 };
            let rgb = [rgba[0], rgba[1], rgba[2]].map(|x| st(x / 256.0));
            mem.write(at + 0x5000, [rgb[0], rgb[1], rgb[2], alpha]);
        }

        // the ends move on
        let o = r(mem, OFFSET);
        mem.write(o, [x87::copy(ox), x87::copy(oy)]);
        call(mem, r(mem, COPY_VECTOR), &[r(mem, END0), r(mem, END1)]);
        call(mem, r(mem, COPY_VECTOR), &[r(mem, END1), r(mem, NEXT)]);
        let state = mem.read::<u32>(state_at);
        let left = mem.read::<i32>(state + 0x1c).wrapping_sub(1);
        mem.write(state + 0x1c, left);
        out += 0xa0;
        let state = mem.read::<u32>(state_at);
        let next_point = mem
            .read::<u32>(state + 4)
            .wrapping_add(mem.read::<u32>(state + 0x14));
        mem.write(state + 4, next_point);
    }
}

/// The strip's end cap ([`ke_th_bp_mk_strip_end`]).
pub mod strip_end {
    /// How much the width scale grows at the cap.
    pub const WIDTH_STEP: u32 = 0x0230_fa08;
    /// Six vertices (xyzw), and their colours.
    pub const VERTS: u32 = 0x0230_f908;
    pub const COLORS: u32 = 0x0230_f968;
}

/// The scratch [`ke_th_bp_mk_strip_end`] needs: three colours and a
/// vertex.
pub const STRIP_END_WORK: u32 = 0x40;

/// The cap's vertices after the first: the first end plus (share of the
/// offset, inner) and their colour (the edge colour after its step, or
/// the centre's) and alpha.
/// (side: share of the offset and inner, centre colour, alpha).
type CapVertex = (Option<(f64, bool)>, bool, bool);

const STRIP_END: [CapVertex; 5] = [
    (Some((1.0, false)), false, false),
    (Some((1.0, true)), false, true),
    (None, true, true),
    (Some((-1.0, true)), false, true),
    (Some((-1.0, false)), false, false),
];

/// The belt path strip's end cap (`KeThBP_MkStripEnd_00765a10()`,
/// `0x00765a10`, called by `KeThBP_MkGifPk` after
/// [`ke_th_bp_mk_strip`]).
///
/// The walk's `+0x20` keeps its `+0` (which then moves on by 0xd0). Six
/// vertices into [`strip_end::VERTS`]: the strip's second end (the edge
/// colour scaled, alpha 0), then around its first: plus the offset, plus
/// the inner offset, itself, minus the inner offset, minus the offset;
/// each times its w and back through [`strip::UNPROJECT`], colours over
/// 256 into [`strip_end::COLORS`]. Between them the edge and centre
/// colours step on, and the width scale grows by
/// [`strip_end::WIDTH_STEP`] (not below 0). Returns the walk's address
/// (the original's `eax`).
pub fn ke_th_bp_mk_strip_end<M, C>(mem: &mut M, work: u32, call: &mut C) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    use strip::{CENTRE, COLOR_SCALE, EDGE, END0, END1, OFFSET, SIDE1};
    let r = |mem: &M, a: u32| mem.rebase(a);
    let g = |mem: &M, a: u32| ld(mem.read::<u32>(mem.rebase(a)));
    let put = |mem: &mut M, a: u32, x: f64| {
        let at = mem.rebase(a);
        mem.write(at, st(x));
    };
    let state = mem.read::<u32>(r(mem, strip::STATE));
    mem.write(state + 0x20, mem.read::<u32>(state));
    let (edge, centre, vertex) = (work, work + 0x10, work + 0x30);
    let scale = r(mem, COLOR_SCALE);
    // a vertex `src` times its w, back through the matrix, with a colour
    let emit = |mem: &mut M, call: &mut C, i: u32, src: u32, color: u32, alpha: bool| {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let [x, y, z, w] = mem.read::<[u32; 4]>(r(mem, src)).map(ld);
        mem.write(vertex, [st(w * x), st(w * y), st(z * w), st(w)]);
        call(
            mem,
            r(mem, APPLY_MATRIX),
            &[vertex, r(mem, strip::UNPROJECT), vertex],
        );
        let pos = mem.read::<[u32; 4]>(vertex).map(x87::copy);
        let at = r(mem, strip_end::VERTS) + 0x10 * i;
        mem.write(at, pos);
        let rgba = mem.read::<[u32; 4]>(color).map(ld);
        let a = if alpha { st(rgba[3] / 256.0) } else { 0 };
        let rgb = [rgba[0], rgba[1], rgba[2]].map(|c| st(c / 256.0));
        let at = r(mem, strip_end::COLORS) + 0x10 * i;
        mem.write(at, [rgb[0], rgb[1], rgb[2], a]);
    };

    // the second end, in the edge colour before its step
    call(mem, r(mem, MUL_VECTOR), &[edge, r(mem, EDGE), scale]);
    emit(mem, call, 0, END1, edge, false);
    let e = r(mem, EDGE);
    call(
        mem,
        r(mem, ADD_SCALED_COLOR),
        &[e, e, r(mem, strip::EDGE_STEP), 0x3f80_0000],
    );
    call(mem, r(mem, MUL_VECTOR), &[edge, e, scale]);
    // the width scale grows (not below 0)
    let width = st(g(mem, strip_end::WIDTH_STEP) + g(mem, strip::WIDTH_SCALE));
    let width = if ld(width) < 0.0 { 0 } else { width };
    let at = r(mem, strip::WIDTH_SCALE);
    mem.write(at, width);

    // around the first end
    let inner = g(mem, strip::INNER);
    let offset = (g(mem, OFFSET), g(mem, OFFSET + 4));
    for (i, &(side, centre_color, alpha)) in (1u32..).zip(&STRIP_END) {
        let src = if let Some((sign, in_edge)) = side {
            for (j, o) in [(0, offset.0), (4, offset.1)] {
                let e = g(mem, END0 + j);
                let o = if in_edge { o * inner } else { o };
                put(mem, SIDE1 + j, if sign < 0.0 { e - o } else { e + o });
            }
            SIDE1
        } else {
            END0
        };
        if centre_color {
            let c = r(mem, CENTRE);
            call(
                mem,
                r(mem, ADD_SCALED_COLOR),
                &[c, c, r(mem, strip::CENTRE_STEP), 0x3f80_0000],
            );
            call(mem, r(mem, MUL_VECTOR), &[centre, c, scale]);
        }
        emit(
            mem,
            call,
            i,
            src,
            if centre_color { centre } else { edge },
            alpha,
        );
    }
    let state = mem.read::<u32>(r(mem, strip::STATE));
    let v = mem.read::<u32>(state).wrapping_add(0xd0);
    mem.write(state, v);
    state
}

/// `KeThBP_Update(p)` (`0x007637f0`), called through the game by
/// [`ke_th_cp_update`].
pub const BP_UPDATE: u32 = 0x0076_37f0;
/// The last updated thread's kind byte and object (`0x02305840`,
/// `0x02305844`).
const LAST_KIND: u32 = 0x0230_5840;
const LAST_OBJECT: u32 = 0x0230_5844;
/// [`ke_th_cp_update`]'s scratch, `KeThBP_Update`'s argument: a unit
/// matrix (0x40 bytes, 16-byte aligned), the belt path `+0x40`, then
/// the thread's `+0x04`, `+0x0c` and `+0x08` words.
pub const CP_WORK: u32 = 0x50;

/// `KeThCp_Update(t)` (`0x007649a0`): updates every belt path of the
/// thread `t` (object `t[0]`, its list head at the object's `+0`). The
/// object's flag bit 1 (`+4`) is taken and cleared: set, belt paths with
/// `+0x3e` bit 8 also get bit 5 (running). For each belt path `bp` (links
/// through `get_ptr32_func`): with `+0x3e` bit 9 its head (segment `u16
/// +0x34`) and the one two on get `t[4]` and `t[5]`
/// (`KeVu0CopyVecXYZ`); then `KeThBP_Update` runs on `work` (see
/// [`CP_WORK`]). A belt path with bit 4 (dead) is unlinked (`+0` next,
/// `+4` previous), quit ([`crate::ke::bridge::ke_th_bp_quit`]) and
/// pushed on the free list `t[3] + 4`. Then `0x02305840` = the byte at
/// `t[3]`, `0x02305844` = the object, which is also returned.
pub fn ke_th_cp_update<M, C>(mem: &mut M, t: u32, work: u32, call: &mut C) -> u32
where
    M: Mem,
    C: FnMut(&mut M, u32, &[u32]) -> u32,
{
    use crate::ke::{GET_PTR32_FUNC, IS_PTR32_NULL, SET_PTR32, VU0_UNIT_MATRIX};
    let r = |mem: &M, a: u32| mem.rebase(a);
    let object = mem.read::<u32>(t);
    let flags = mem.read::<u8>(object + 4);
    let running = if flags & 2 != 0 {
        mem.write(object + 4, flags & !2);
        flag::RUNNING
    } else {
        0
    };
    let f = r(mem, VU0_UNIT_MATRIX);
    call(mem, f, &[work]);
    let [a, b, c] = [4, 0xc, 8].map(|o| mem.read::<u32>(t + o));
    mem.write(work + 0x44, [a, b, c]);
    let get = |mem: &mut M, call: &mut C, p: u32| {
        let f = r(mem, GET_PTR32_FUNC);
        call(mem, f, &[p])
    };
    let head = mem.read::<u32>(object);
    let mut bp = get(mem, call, head);
    while bp != 0 {
        if u32::from(mem.read::<u16>(bp + 0x3e)) & 0x200 != 0 {
            let seg = u32::from(mem.read::<u16>(bp + 0x34));
            let f = r(mem, COPY_XYZ);
            let to = mem.read::<u32>(t + 0x10);
            call(mem, f, &[bp + 0x40 + 0x30 * seg, to]);
            let seg = u32::from(mem.read::<u16>(bp + 0x34));
            let to = mem.read::<u32>(t + 0x14);
            call(mem, f, &[bp + 0x30 * (seg + 2), to]);
        }
        let state = u32::from(mem.read::<u16>(bp + 0x3e));
        if state & flag::START != 0 {
            #[allow(clippy::cast_possible_truncation)] // the low half
            mem.write(bp + 0x3e, (state | running) as u16);
        }
        mem.write(work + 0x40, bp);
        let f = r(mem, BP_UPDATE);
        call(mem, f, &[work]);
        let next_link = mem.read::<u32>(bp);
        if u32::from(mem.read::<u8>(bp + 0x3e)) & flag::DEAD == 0 {
            bp = get(mem, call, next_link);
            continue;
        }
        let next = get(mem, call, next_link);
        let f = r(mem, IS_PTR32_NULL);
        if call(mem, f, &[mem.read::<u32>(bp)]) == 0 {
            let link = mem.read::<u32>(bp);
            let n = get(mem, call, link);
            mem.write(n + 4, mem.read::<u32>(bp + 4));
        }
        let back = mem.read::<u32>(bp + 4);
        let p = get(mem, call, back);
        mem.write(p, mem.read::<u32>(bp));
        crate::ke::bridge::ke_th_bp_quit(mem, bp);
        let pool = mem.read::<u32>(t + 0xc);
        mem.write(bp, mem.read::<u32>(pool + 4));
        let f = r(mem, SET_PTR32);
        let link = call(mem, f, &[bp]);
        let pool = mem.read::<u32>(t + 0xc);
        mem.write(pool + 4, link);
        bp = next;
    }
    let kind = mem.read::<u32>(t + 0xc);
    let kind = u32::from(mem.read::<u8>(kind));
    mem.write(r(mem, LAST_KIND), kind);
    mem.write(r(mem, LAST_OBJECT), object);
    object
}
