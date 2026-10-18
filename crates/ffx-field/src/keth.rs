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
