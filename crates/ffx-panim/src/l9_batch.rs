//! M5 stage 1, round 9 (`tail9`, list `phyre-scripting`):
//! `PAnimationController_batch_005258d0`, the batch function
//! `Phyre_PAnimation_PAnimationController_getBatchFunction` hands out: one
//! animation clip evaluated at the batch's time into the target records:
//! the constant channels copied, every animated channel's key found (a
//! cached search near the last key, the binary search
//! `Phyre_PAnimation_findKeyIndex_005253c0` beyond it) and its value
//! interpolated by the channel's kind (linear, quaternion: linear when
//! the ends are close, else `Phyre_PAnimation_slerp_0050ed70`; step).
//!
//! x87 code as the formulas at 53 bits (`ffx_bridge::x87`), each value
//! rounded to `f32` where the original stores it.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the original's registers
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// `Phyre_PAnimation_findKeyIndex_005253c0(keys, n, value)` and
    /// `Phyre_PAnimation_slerp_0050ed70(out, t, a, b, threshold)` (cdecl).
    pub const FIND_KEY: u32 = 0x0052_53c0;
    pub const SLERP: u32 = 0x0050_ed70;
    /// The constants: the cached time's start (`-1.0f`, `0x00b92340`),
    /// the negation factor (`-1.0`, double, `0x00b92350`), the dot
    /// product above which the quaternions are blended linearly (`0.7f`,
    /// `0x00b2607c`) and the slerp threshold (`0x00b26080`, f32).
    pub const NO_TIME: u32 = 0x00b9_2340;
    pub const MINUS_ONE: u32 = 0x00b9_2350;
    pub const NEAR: u32 = 0x00b2_607c;
    pub const SLERP_THRESHOLD: u32 = 0x00b2_6080;
    /// The batch function's own frame for the slerp: the two quaternions
    /// and its result (`0x10` bytes each).
    pub const SLERP_FRAME: u32 = 0x30;

    fn f(h: &impl Host, at: u32) -> f64 {
        x87::load(h.read(at))
    }

    /// `fld dword; fstp dword`: a float word as the x87 copies it.
    fn copy(h: &impl Host, at: u32) -> u32 {
        x87::store(f(h, at))
    }

    /// `a + r32(r32(b - a) * u)` per lane, each sum rounded: the linear
    /// blend both kinds use.
    fn blend(a: [u32; 4], b: [u32; 4], u: f64) -> [u32; 4] {
        let d = [0, 1, 2, 3].map(|i| x87::store(x87::sub(x87::load(b[i]), x87::load(a[i]))));
        let m = d.map(|d| x87::store(x87::mul(x87::load(d), u)));
        [0, 1, 2, 3].map(|i| x87::store(x87::add(x87::load(a[i]), x87::load(m[i]))))
    }

    /// The four lanes at `at` as the x87 copies them.
    fn lanes(h: &impl Host, at: u32) -> [u32; 4] {
        [0, 4, 8, 0xc].map(|o| copy(h, at.wrapping_add(o)))
    }

    /// The key search state the original keeps across the channels of a
    /// call: the time and key pointer of the last search, its key's index
    /// relative to that pointer, and the offset from the channel's
    /// starting index to the key found.
    struct Cache {
        time: u32,
        keys: u32,
        rel: u32,
        delta: u32,
    }

    /// `PAnimationController_batch_005258d0(out, _, _, batch, cursors,
    /// targets)` (cdecl) -> 0. `batch`: `[batch]` the offset of its header
    /// (`+0` the time, f32; `+6` the constant channels' count), `+0x14` the
    /// clip (`+2` the animated channels, `+4` the constant ones, `+6` the
    /// groups, `+8` each group's channel count (u16), then the animated
    /// channels' (target, kind) i16 pairs from `+8 + 2 * round4(groups)`,
    /// the constant ones' after them, the tracks after those, 4-aligned:
    /// 0x10 bytes each, keys `+0`, values `+4`, key count `+8` (24 bits),
    /// lanes per key `+0xb`), `+0x1c` the constant values (`+0x24..+0x30`
    /// of each 0x34-byte record). `targets`: `+0` records (0x10 bytes:
    /// `+0` and `+8` copied), `+4` count, `+8` the first index. Each target
    /// in range gets 0x20 bytes at `[out] + 32 * index`: the four lanes,
    /// the record's `+0`, the channel's kind, the record's `+8`.
    /// `cursors`: one key index per animated channel, read and updated.
    /// `slerp` is 0x30 bytes of scratch the slerp's arguments go in.
    #[allow(clippy::too_many_lines)]
    pub fn batch<H: Host>(
        h: &mut H,
        slerp: u32,
        out: u32,
        batch: u32,
        cursors: u32,
        targets: u32,
    ) -> u32 {
        let header = batch.wrapping_add(h.read(batch));
        let time: u32 = x87::store(f(&*h, header));
        let t = x87::load(time);
        let n_const = u32::from(h.read::<u16>(header.wrapping_add(6)));
        let dst_base: u32 = h.read(out);
        let clip: u32 = h.read(batch.wrapping_add(0x14));
        let const_values: u32 = h.read(batch.wrapping_add(0x1c));
        let n_groups = u32::from(h.read::<u16>(clip.wrapping_add(6)));
        let n_anim = u32::from(h.read::<u16>(clip.wrapping_add(2)));
        let n_static = u32::from(h.read::<u16>(clip.wrapping_add(4)));
        let round4 = n_groups.wrapping_add(3) & !3;
        let anim_list = clip.wrapping_add(8).wrapping_add(round4.wrapping_mul(2));
        let const_list = clip.wrapping_add(
            round4
                .wrapping_add(4)
                .wrapping_add(n_anim.wrapping_mul(2))
                .wrapping_mul(2),
        );
        let (records, n_targets, first): (u32, u32, u32) = (
            h.read(targets),
            h.read(targets.wrapping_add(4)),
            h.read(targets.wrapping_add(8)),
        );
        // the constant channels
        let mut entry = const_list;
        let mut value = const_values.wrapping_add(0x2c);
        for _ in 0..n_const {
            let target = i32::from(h.read::<i16>(entry))
                .cast_unsigned()
                .wrapping_sub(first);
            if target < n_targets {
                let kind = u32::from(h.read::<u16>(entry.wrapping_add(2)));
                let rec = records.wrapping_add(target.wrapping_mul(0x10));
                let (r0, r8): (u32, u32) = (h.read(rec), h.read(rec.wrapping_add(8)));
                let dst = dst_base.wrapping_add(target.wrapping_mul(0x20));
                h.write(dst.wrapping_add(0x10), r0);
                h.write(dst.wrapping_add(0x14), kind);
                h.write(dst.wrapping_add(0x18), r8);
                let v = lanes(&*h, value.wrapping_sub(8));
                for (o, x) in (0..16).step_by(4).zip(v) {
                    h.write(dst.wrapping_add(o), x);
                }
            }
            entry = entry.wrapping_add(4);
            value = value.wrapping_add(0x34);
        }
        // the animated channels, group by group
        let after = round4
            .wrapping_add(n_anim.wrapping_add(n_static).wrapping_mul(2))
            .wrapping_add(4)
            .wrapping_mul(2)
            .wrapping_add(clip);
        let mut desc = after.wrapping_add(after.wrapping_neg() & 3);
        let mut chan = anim_list;
        let mut cursor = cursors;
        let mut cache = Cache {
            time: h.read(h.rebase(NO_TIME)),
            keys: 0,
            rel: 0,
            delta: 0,
        };
        let mut result = [0_u32; 4];
        for group in 0..n_groups {
            let count = h.read::<u16>(clip.wrapping_add(8).wrapping_add(2 * group));
            let comps = u32::from(h.read::<u8>(desc.wrapping_add(0xb)));
            let window = (0x080a_1020_u32 >> (comps.wrapping_mul(8).wrapping_sub(8) & 0x1f)) & 0xff;
            for _ in 0..count {
                let target = i32::from(h.read::<i16>(chan))
                    .cast_unsigned()
                    .wrapping_sub(first);
                if target < n_targets {
                    let kind = u32::from(h.read::<u16>(chan.wrapping_add(2)));
                    let n = h.read::<u32>(desc.wrapping_add(8)) & 0xff_ffff;
                    let mut start: u32 = h.read(cursor);
                    if start >= n.wrapping_sub(1) {
                        start = n.wrapping_sub(1);
                    }
                    let keys_base: u32 = h.read(desc);
                    let prev = start.wrapping_sub(1);
                    let base = if prev.cast_signed() <= 0 { 0 } else { prev };
                    let keyp = keys_base.wrapping_add(base.wrapping_mul(4));
                    let values: u32 = h.read(desc.wrapping_add(4));
                    let rel0 = start.wrapping_sub(base);
                    let mut vals = values.wrapping_add(base.wrapping_mul(comps).wrapping_mul(4));
                    // fucompp: an ordered equal
                    #[allow(clippy::float_cmp)]
                    let same_time = t == x87::load(cache.time);
                    let (index, key_at) = if same_time && cache.keys == keyp {
                        let index = rel0.wrapping_add(cache.delta);
                        vals = vals.wrapping_add(comps.wrapping_mul(cache.rel).wrapping_mul(4));
                        (index, keyp.wrapping_add(cache.rel.wrapping_mul(4)))
                    } else {
                        cache.time = time;
                        cache.keys = keyp;
                        let avail = n.wrapping_sub(base);
                        let lim = if window.wrapping_sub(1).cast_signed() < avail.cast_signed() {
                            window.wrapping_sub(1)
                        } else {
                            avail
                        };
                        let k = |h: &H, i: u32| f(h, keyp.wrapping_add(i.wrapping_mul(4)));
                        let (index, key_at) = if f(&*h, keyp) > t {
                            // before the window: the binary search over the keys before
                            let i = h.call_sym(FIND_KEY, Conv::Cdecl, &[keys_base, base, time]);
                            let back = base.wrapping_sub(i).wrapping_mul(comps).wrapping_mul(4);
                            vals = vals.wrapping_sub(back);
                            (i, keyp.wrapping_add(i.wrapping_sub(base).wrapping_mul(4)))
                        } else {
                            let r = if k(&*h, lim.wrapping_sub(1)) < t {
                                // past the window: the binary search from it
                                h.call_sym(FIND_KEY, Conv::Cdecl, &[keyp, avail, time])
                            } else {
                                // in the window: back to the last key not after
                                // the time (NaN stops), then forward over the
                                // keys before it
                                let mut i = rel0;
                                while i.cast_signed() >= 0 && k(&*h, i) > t {
                                    i = i.wrapping_sub(1);
                                }
                                let mut next = i.wrapping_add(1);
                                while next.cast_signed() < lim.cast_signed() && k(&*h, next) < t {
                                    next = next.wrapping_add(1);
                                    i = i.wrapping_add(1);
                                }
                                i
                            };
                            vals = vals.wrapping_add(r.wrapping_mul(comps).wrapping_mul(4));
                            (r.wrapping_add(base), keyp.wrapping_add(r.wrapping_mul(4)))
                        };
                        cache.rel = key_at
                            .wrapping_sub(keyp)
                            .cast_signed()
                            .wrapping_shr(2)
                            .cast_unsigned();
                        cache.delta = index.wrapping_sub(rel0);
                        (index, key_at)
                    };
                    h.write(cursor, index);
                    let k0 = f(&*h, key_at);
                    let interpolate =
                        k0 < t && index.cast_signed() < n.wrapping_sub(1).cast_signed();
                    if interpolate {
                        match kind {
                            0 => {
                                let k1 = f(&*h, key_at.wrapping_add(4));
                                let u = x87::store(x87::div(x87::sub(t, k0), x87::sub(k1, k0)));
                                let a = lanes(&*h, vals);
                                let b = lanes(&*h, vals.wrapping_add(comps.wrapping_mul(4)));
                                result = blend(a, b, x87::load(u));
                            }
                            1 => {
                                let k1 = f(&*h, key_at.wrapping_add(4));
                                let u = x87::store(x87::div(x87::sub(t, k0), x87::sub(k1, k0)));
                                let a = lanes(&*h, vals);
                                let b = lanes(&*h, vals.wrapping_add(comps.wrapping_mul(4)));
                                result = quaternion(h, slerp, a, b, u);
                            }
                            2 => result = lanes(&*h, vals),
                            _ => {}
                        }
                    } else {
                        result = lanes(&*h, vals);
                    }
                    let rec = records.wrapping_add(target.wrapping_mul(0x10));
                    let dst = dst_base.wrapping_add(target.wrapping_mul(0x20));
                    for (o, v) in (0..16).step_by(4).zip(result) {
                        h.write(dst.wrapping_add(o), v);
                    }
                    let (r0, r8): (u32, u32) = (h.read(rec), h.read(rec.wrapping_add(8)));
                    h.write(dst.wrapping_add(0x14), kind);
                    h.write(dst.wrapping_add(0x10), r0);
                    h.write(dst.wrapping_add(0x18), r8);
                }
                cursor = cursor.wrapping_add(4);
                chan = chan.wrapping_add(4);
                desc = desc.wrapping_add(0x10);
            }
        }
        0
    }

    /// The quaternion blend from `a` to `b` at `u` (f32): the dot product
    /// summed with each partial rounded; below 0 `a` negated (times -1.0);
    /// above 0.7 the linear blend, else the slerp (its result read back
    /// from its `out`).
    fn quaternion(h: &mut impl Host, scratch: u32, a: [u32; 4], b: [u32; 4], u: u32) -> [u32; 4] {
        let ld = x87::load;
        let mut dot = x87::store(x87::mul(ld(b[0]), ld(a[0])));
        for i in 1..4 {
            dot = x87::store(x87::add(ld(dot), x87::mul(ld(b[i]), ld(a[i]))));
        }
        let mut a = a;
        let mut d = ld(dot);
        if 0.0 > d {
            d = -d;
            let minus = f64::from_bits(h.read(h.rebase(MINUS_ONE)));
            a = a.map(|x| x87::store(x87::mul(ld(x), minus)));
            dot = x87::store(d);
        }
        let near = f(&*h, h.rebase(NEAR));
        if near < ld(dot) {
            return blend(a, b, ld(u));
        }
        let (qa, qb, res) = (
            scratch,
            scratch.wrapping_add(0x10),
            scratch.wrapping_add(0x20),
        );
        for i in 0..4_u32 {
            h.write(qa.wrapping_add(4 * i), a[i as usize]);
            h.write(qb.wrapping_add(4 * i), b[i as usize]);
        }
        let threshold = copy(&*h, h.rebase(SLERP_THRESHOLD));
        let r = h.call_sym(SLERP, Conv::Cdecl, &[res, u, qa, qb, threshold]);
        lanes(&*h, r)
    }
}
