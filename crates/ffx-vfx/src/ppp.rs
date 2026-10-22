//! Particle programs (`ppp*`): the small per-object steps an effect's
//! script runs, and the rotation matrices they use.
//!
//! A program gets the object (`obj`), for the per-frame steps a key
//! (`key`: the frame it applies at in its first word, then its values),
//! and its control block (`ctrl`), whose word at `+0xc` points at the
//! offsets of the program's work areas in the object: work area `i` is at
//! `obj + 0xa0 + offsets[i]`. A step applies its key only when the key's
//! frame is the object's (`obj + 0xc`), and does nothing while effects are
//! stopped (`g_pppStopStatus`). The `...Con` functions set a work area up.
//!
//! Floats go through the x87 as in the original ([`crate::x87`]): even a
//! copy quiets a signalling NaN.

use crate::x87;

/// `angle` (0x10000 a turn) as the indices of its sine and cosine in the
/// game's 4096-entry sine table.
#[must_use]
pub const fn sin_cos_index(angle: i32) -> (u32, u32) {
    #[allow(clippy::cast_sign_loss)] // masked to 12 bits
    const fn at(a: i32) -> u32 {
        ((a >> 4) & 0xfff) as u32
    }
    (at(angle), at(angle.wrapping_add(0x4000)))
}

/// A matrix the way `pppGetRotMatrix*` build it: the first 15 words from
/// `zero` (the emulated `r0`, four words at a time), the 16th untouched
/// until the axis entries are set. `s` and `c` are `f32` bits.
fn base(zero: [u32; 4]) -> [u32; 16] {
    let mut m = [0; 16];
    for (i, w) in m.iter_mut().enumerate() {
        *w = zero[i % 4];
    }
    m
}

const ONE: u32 = 0x3f80_0000;
const SIGN: u32 = 0x8000_0000;

/// `pppGetRotMatrixX`: rotation about X from sine and cosine bits.
#[must_use]
pub fn rot_x(zero: [u32; 4], s: u32, c: u32) -> [u32; 16] {
    let (s, c) = (x87::load(s), x87::load(c));
    let mut m = base(zero);
    m[0] = ONE;
    m[5] = c;
    m[6] = s;
    m[9] = s ^ SIGN;
    m[10] = c;
    m[15] = ONE;
    m
}

/// `pppGetRotMatrixY`: rotation about Y.
#[must_use]
pub fn rot_y(zero: [u32; 4], s: u32, c: u32) -> [u32; 16] {
    let (s, c) = (x87::load(s), x87::load(c));
    let mut m = base(zero);
    m[0] = c;
    m[2] = s ^ SIGN;
    m[5] = ONE;
    m[8] = s;
    m[10] = c;
    m[15] = ONE;
    m
}

/// `pppGetRotMatrixZ`: rotation about Z.
#[must_use]
pub fn rot_z(zero: [u32; 4], s: u32, c: u32) -> [u32; 16] {
    let (s, c) = (x87::load(s), x87::load(c));
    let mut m = base(zero);
    m[0] = c;
    m[1] = s;
    m[4] = s ^ SIGN;
    m[5] = c;
    m[10] = ONE;
    m[15] = ONE;
    m
}

/// `pppNeiLightEikyo`'s running sums over its six floats `w`: `w[1] +=
/// w[0]`, `w[2] += w[1]`, `w[4] += w[3]`, `w[5] += w[4]`.
pub fn nei_light_accumulate(w: &mut [f32; 6]) {
    w[1] = x87::add(w[1], w[0]);
    w[2] = x87::add(w[1], w[2]);
    w[4] = x87::add(w[3], w[4]);
    w[5] = x87::add(w[4], w[5]);
}

/// The programs where the game keeps their objects.
pub mod bridge {
    use ffx_bridge::{Global, Mem, Ptr};

    use crate::x87;

    /// `g_pppStopStatus`: while nonzero, the per-frame steps do nothing.
    pub const STOP: Global<u32> = Global::new(0x0230_fd34);
    /// `g_pppSinTable`: 4096 `f32` sines over a turn.
    pub const SIN: Global<u32> = Global::new(0x00c4_4bf0);
    /// `g_Ee_Gpr`'s `r0` (the emulated zero register, 16 bytes).
    pub const R0: Global<[u32; 4]> = Global::new(0x00c8_f508);
    /// `g_pppColorConShift`: added once to `pppColorCon`'s work offset,
    /// then cleared.
    pub const COLOR_CON_SHIFT: Global<u32> = Global::new(0x00c8_b500);
    /// `g_pppWaitDone`: set by `pppClearWait`, polled by `pppSendWait`.
    pub const WAIT_DONE: Global<u32> = Global::new(0x010e_7640);

    /// Work area `i`'s offset (`ctrl + 0xc` points at the offsets).
    fn offset(mem: &impl Mem, ctrl: u32, i: u32) -> u32 {
        let offsets = mem.read::<u32>(ctrl.wrapping_add(0xc));
        mem.read(offsets.wrapping_add(4 * i))
    }

    /// Address `at` (from `0xa0`) of work area 0.
    fn work(mem: &impl Mem, obj: u32, ctrl: u32, at: u32) -> Ptr<u32> {
        Ptr::new(obj.wrapping_add(offset(mem, ctrl, 0)).wrapping_add(at))
    }

    /// Whether a step applies `key` now.
    fn applies(mem: &impl Mem, obj: u32, key: u32) -> bool {
        mem.read::<u32>(key) == mem.read::<u32>(obj.wrapping_add(0xc))
    }

    fn zero(mem: &mut impl Mem, obj: u32, ctrl: u32, at: &[u32]) {
        for &a in at {
            work(mem, obj, ctrl, a).set(mem, 0);
        }
    }

    fn get_f32(mem: &impl Mem, p: Ptr<u32>) -> f32 {
        f32::from_bits(p.get(mem))
    }

    /// `pppKeDrctCon(obj, ctrl)` (`0x0075e470`): work `+0xac` = 1.0.
    pub fn ppp_ke_drct_con(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        work(mem, obj, ctrl, 0xac).set(mem, 0x3f80_0000);
    }

    /// `pppKeDrct(obj, key, ctrl)` (`0x0075e430`): copies the key's three
    /// floats at `+0x10` to work `+0xa0`.
    pub fn ppp_ke_drct(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if !applies(mem, obj, key) {
            return;
        }
        for i in 0..3 {
            let v = x87::load(mem.read(key + 0x10 + 4 * i));
            work(mem, obj, ctrl, 0xa0 + 4 * i).set(mem, v);
        }
    }

    /// `pppKeMatSNCon(obj, ctrl)` (`0x007340f0`).
    pub fn ppp_ke_mat_sn_con(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        zero(mem, obj, ctrl, &[0x120, 0x124, 0x128]);
    }

    /// `pppVtMimeCon(obj, ctrl)` (`0x0072ed70`).
    pub fn ppp_vt_mime_con(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        zero(mem, obj, ctrl, &[0xa8, 0xac, 0xa4, 0xa0]);
    }

    /// `g_pppVtMimeT`: the last blend factor [`ppp_vt_mime`] used.
    pub const VT_MIME_T: Global<u32> = Global::new(0x00c8_f7f0);

    /// `pppVtMime(obj, key, ctrl)` (`0x0072eb20`): the vertex morph
    /// between two models. Work area 0 holds the blend factor `t` (`+0xa0`),
    /// its rate (`+0xa4`), the rate's step (`+0xa8`) and the morphed
    /// vertices (`+0xac`). Unless stopped: rate += step, then `t` += rate
    /// (each rounded to `f32`); a key that applies adds its three floats
    /// at `+0xc` to `t`, rate and step. Then, unless the key's models
    /// (`+4`, `+8`, entries of the environment's 32-byte model table at
    /// `env +0x20`) are both 0xffff: with `n` the first model's vertex
    /// count (`+0`) and both models' `i16` vertices at `ptr32(+0x14)`
    /// (6 bytes each), the buffer (allocated on first use,
    /// `pppMemAlloc(env, env + 0x10, 8 n)`; none returns at once) gets
    /// each coordinate as `ftol(f32(to - from) * t + from)`, [`VT_MIME_T`]
    /// = `t`, and `obj +0x90` = the buffer.
    #[allow(clippy::many_single_char_names)]
    pub fn ppp_vt_mime<M: Mem>(mem: &mut M, obj: u32, key: u32, ctrl: u32) {
        use ffx_bridge::x87::{Reg, ftol};
        use ffx_system::stub::ptr32;
        let w = obj.wrapping_add(offset(mem, ctrl, 0));
        let f = |mem: &M, at: u32| f32::from_bits(mem.read(w.wrapping_add(at)));
        if STOP.get(mem) == 0 {
            let rate = Reg::load(f(mem, 0xa8)).add_m32(f(mem, 0xa4)).store();
            mem.write(w + 0xa4, rate);
            let t = (Reg::load(f(mem, 0xa0)) + Reg::load(rate)).store();
            mem.write(w + 0xa0, t);
            if applies(mem, obj, key) {
                let k = |mem: &M, at: u32| f32::from_bits(mem.read(key.wrapping_add(at)));
                let t = Reg::load(t).add_m32(k(mem, 0xc)).store();
                mem.write(w + 0xa0, t);
                let rate = Reg::load(rate).add_m32(k(mem, 0x10)).store();
                mem.write(w + 0xa4, rate);
                let step = Reg::load(k(mem, 0x14)).add_m32(f(mem, 0xa8)).store();
                mem.write(w + 0xa8, step);
            }
        }
        let (m1, m2): (u32, u32) = (mem.read(key + 4), mem.read(key + 8));
        if m1 == 0xffff && m2 == 0xffff {
            return;
        }
        let env = |mem: &M| mem.read::<u32>(crate::heap::bridge::ENV_SLOT.get(mem));
        let model = |mem: &M, m: u32| {
            let table: u32 = mem.read(env(mem).wrapping_add(0x20));
            table.wrapping_add(m << 5)
        };
        let from = ptr32(mem.read(model(mem, m1).wrapping_add(0x14)));
        let to = ptr32(mem.read(model(mem, m2).wrapping_add(0x14)));
        let t = Reg::load(f(mem, 0xa0)).store();
        let e = env(mem);
        let n: u32 = mem.read(model(mem, m1));
        let buf = Ptr::<u32>::new(w + 0xac);
        if buf.get(mem) == 0 {
            let p = crate::heap::bridge::ppp_mem_alloc(mem, e, e.wrapping_add(0x10), n << 3);
            buf.set(mem, p);
            if p == 0 {
                return;
            }
        }
        VT_MIME_T.set(mem, t.to_bits());
        let out = buf.get(mem);
        for i in 0..n {
            for c in 0..3 {
                let at = 6 * i + 2 * c;
                let a = i32::from(mem.read::<i16>(from.wrapping_add(at)));
                let b = i32::from(mem.read::<i16>(to.wrapping_add(at)));
                // fild, then fstp m32 (exact for these)
                let d = Reg::constant(f64::from(b - a)).store();
                let v = Reg::load(d).mul_m32(f(mem, 0xa0)) + Reg::constant(f64::from(a));
                #[allow(clippy::cast_possible_truncation)] // the low word in ax
                mem.write(out.wrapping_add(at), ftol(mem, v.value()) as u16);
            }
        }
        let p = buf.get(mem);
        mem.write(obj.wrapping_add(0x90), p);
    }

    /// `pppEiWfacc(obj, key, ctrl)` (`0x0075bb30`): a pull toward a
    /// target. With `A` and `B` the data the key's references (`+4`/`+8`
    /// and `+0xc`/`+0x10`: object index into `g_pppCurrentPart`'s objects
    /// and offset, plus 0xa0; index -1 the rand target `0x00d52398`) and
    /// `d` = work area 0 `+0xa0..+0xa8` minus `B`'s three floats, through
    /// the emulated VU0: `0x00c8f830` = (`d`, `stale`: the original's
    /// uninitialised local, at `+0xc`), `0x00c8f810` = `d + vf00`, the
    /// squares and their running sum (`0x00c8f830..`, `0x00c8f824`,
    /// `0x00c8f828`), `0x00c8f830` = `len` = `sqrt(|sum|) + vf00.x`, `Q` =
    /// `vf00.w / len`, `0x00c8f820` = `n` = `Q (d + vf00)`. When `len` is
    /// below `A`'s radius `r` (`+0x10`) and `i` = `f32(A . n)`
    /// (`FFXVu0InnerProduct`) above its cosine `c` (`+0x18`), work area 1
    /// `+0xa0..+0xa8` += `n k` with `k` = `f32(f32(f32((i - c) / (1 - c) x
    /// A +0x1c) x (r - len) / r) / key +0x14)`.
    #[allow(clippy::many_single_char_names)]
    pub fn ppp_ei_wfacc<M: Mem>(mem: &mut M, obj: u32, key: u32, ctrl: u32, stale: u32) {
        use ffx_bridge::x87::{Reg, round_trip};
        use ffx_system::stub::ptr32;
        let f = |mem: &M, at: u32| f32::from_bits(mem.read(at));
        let w0 = obj.wrapping_add(offset(mem, ctrl, 0));
        let target = |mem: &M, at: u32| {
            let id: u32 = mem.read(key.wrapping_add(at));
            if id == u32::MAX {
                return crate::rand::bridge::TARGET.ptr(mem).addr;
            }
            let part = crate::prog::bridge::CURRENT_PART.get(mem);
            let objs: u32 = mem.read(part.wrapping_add(0x3c));
            let base = ptr32(mem.read(objs.wrapping_add(id.wrapping_mul(16)).wrapping_add(4)));
            let off: u32 = mem.read(key.wrapping_add(at + 4));
            off.wrapping_add(0xa0).wrapping_add(base)
        };
        let (a, b) = (target(mem, 4), target(mem, 0xc));
        let d: [f32; 3] = std::array::from_fn(|i| {
            let i = 4 * u32::try_from(i).unwrap_or(0);
            Reg::load(f(mem, w0.wrapping_add(0xa0 + i)))
                .sub_m32(f(mem, b.wrapping_add(i)))
                .store()
        });
        let vf = |mem: &M, at: u32| ffx_bridge::Global::<u32>::new(at).ptr(mem).addr;
        let (r20, r30) = (vf(mem, 0x00c8_f820), vf(mem, 0x00c8_f830));
        let r10 = vf(mem, 0x00c8_f810);
        mem.write(r30, d);
        mem.write(r30 + 0xc, stale);
        let vf00: [f32; 4] = mem.read(vf(mem, 0x00c0_a004));
        let e: [f32; 3] = std::array::from_fn(|i| Reg::load(d[i]).add_m32(vf00[i]).store());
        mem.write(r10, e);
        let xx = (Reg::load(d[0]) * Reg::load(d[0])).store();
        let yy = Reg::load(d[1]).mul_m32(d[1]).store();
        let zz = Reg::load(d[2]).mul_m32(d[2]).store();
        let s1 = Reg::load(yy).add_m32(xx).store();
        let s2 = Reg::load(zz).add_m32(s1).store();
        mem.write(r30 + 4, yy);
        mem.write(r30 + 8, zz);
        mem.write(r20 + 4, s1);
        mem.write(r20 + 8, s2);
        let q = Reg::load(round_trip(f32::from_bits(s2.to_bits() & 0x7fff_ffff)))
            .sqrt()
            .store();
        let len = Reg::load(q).add_m32(vf00[0]).store();
        mem.write(r30, len);
        let big_q = Reg::load(vf00[3]).div_m32(len).store();
        mem.write(vf(mem, 0x00c8_f788), big_q);
        let qr = Reg::load(big_q);
        let n = [
            (Reg::load(e[0]) * qr).store(),
            (Reg::load(e[1]) * qr).store(),
            qr.mul_m32(e[2]).store(),
        ];
        mem.write(r20, n);
        let radius = f(mem, a.wrapping_add(0x10));
        if len.partial_cmp(&radius) != Some(std::cmp::Ordering::Less) {
            return;
        }
        let av: [f32; 3] = mem.read(a);
        let i = round_trip(ffx_ps2::vu0::inner(av, n));
        let c = f(mem, a.wrapping_add(0x18));
        if c.partial_cmp(&i) != Some(std::cmp::Ordering::Less) {
            return;
        }
        let k1 = (Reg::load(i).sub_m32(c) / (Reg::ONE - Reg::load(c)))
            .mul_m32(f(mem, a.wrapping_add(0x1c)))
            .store();
        let k2 = (Reg::load(k1) * (Reg::load(radius).sub_m32(len).div_m32(radius))).store();
        let k3 = Reg::load(k2)
            .div_m32(f(mem, key.wrapping_add(0x14)))
            .store();
        let k = Reg::load(k3);
        let w1 = obj.wrapping_add(offset(mem, ctrl, 1));
        let x = (Reg::load(n[0]) * k + Reg::load(f(mem, w1 + 0xa0))).store();
        mem.write(w1 + 0xa0, x);
        let y = (Reg::load(n[1]) * k).add_m32(f(mem, w1 + 0xa4)).store();
        mem.write(w1 + 0xa4, y);
        let z = k.mul_m32(n[2]).add_m32(f(mem, w1 + 0xa8)).store();
        mem.write(w1 + 0xa8, z);
    }

    /// `pppNeiLightEikyoCon(obj, ctrl)` (`0x0075d6c0`).
    pub fn ppp_nei_light_eikyo_con(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        zero(mem, obj, ctrl, &[0xa8, 0xa4, 0xa0, 0xb4, 0xb0, 0xac]);
    }

    /// `pppKeThCon2(obj, ctrl)` (`0x007364e0`).
    pub fn ppp_ke_th_con2(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        zero(
            mem,
            obj,
            ctrl,
            &[
                0x10c, 0x110, 0x114, 0x118, 0x11c, 0x120, 0x124, 0x128, 0x12c, 0xbc, 0xd4, 0xdc,
            ],
        );
    }

    /// `pppKeThSftCon(obj, ctrl)` (`0x00737130`).
    pub fn ppp_ke_th_sft_con(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        zero(
            mem,
            obj,
            ctrl,
            &[
                0xc8, 0xa0, 0xc0, 0xa4, 0xc4, 0xa8, 0xcc, 0xac, 0xb0, 0xb4, 0xb8, 0xbc,
            ],
        );
    }

    /// `pppColorCon(obj, ctrl)` (`0x0075d240`): work `+0xa0` (shifted by
    /// `g_pppColorConShift`, which it clears) from the low eight bytes of
    /// `r0`.
    pub fn ppp_color_con(mem: &mut impl Mem, obj: u32, ctrl: u32) {
        let shift = COLOR_CON_SHIFT.get(mem);
        let at = work(mem, obj, ctrl, 0xa0).addr.wrapping_add(shift);
        let r0 = R0.get(mem);
        mem.write(at, r0[0]);
        mem.write(at.wrapping_add(4), r0[1]);
        COLOR_CON_SHIFT.set(mem, 0);
    }

    /// `pppColor(obj, key, ctrl)` (`0x0075d1f0`): adds the key's four
    /// `i16` at `+8` to work `+0xa0` (an RGBA step).
    pub fn ppp_color(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if STOP.get(mem) != 0 || !applies(mem, obj, key) {
            return;
        }
        let at = work(mem, obj, ctrl, 0xa0).addr;
        for i in 0..4 {
            let p = Ptr::<u16>::new(at.wrapping_add(2 * i));
            let d = mem.read::<u16>(key + 8 + 2 * i);
            let v = p.get(mem).wrapping_add(d);
            p.set(mem, v);
        }
    }

    /// `pppAngle(obj, key, ctrl)` (`0x0075ce30`): adds the key's four
    /// `i32` at `+0x10` to work `+0xa0`.
    pub fn ppp_angle(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if STOP.get(mem) != 0 || !applies(mem, obj, key) {
            return;
        }
        for i in 0..4 {
            let p = work(mem, obj, ctrl, 0xa0 + 4 * i);
            let v = p.get(mem).wrapping_add(mem.read(key + 0x10 + 4 * i));
            p.set(mem, v);
        }
    }

    /// `pppNeiLightEikyo(obj, key, ctrl)` (`0x0075d5d0`): adds the key's
    /// six floats at `+4` to work `+0xa0`, then the running sums
    /// ([`super::nei_light_accumulate`]), the sums every frame.
    pub fn ppp_nei_light_eikyo(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if STOP.get(mem) != 0 {
            return;
        }
        let at = |mem: &_, i: u32| work(mem, obj, ctrl, 0xa0 + 4 * i);
        if applies(mem, obj, key) {
            for i in 0..6 {
                let k = f32::from_bits(mem.read(key + 4 + 4 * i));
                let p = at(mem, i);
                let v = x87::add(k, get_f32(mem, p));
                p.set(mem, v.to_bits());
            }
        }
        let mut w = [0f32; 6];
        for (i, v) in (0..).zip(w.iter_mut()) {
            *v = get_f32(mem, at(mem, i));
        }
        super::nei_light_accumulate(&mut w);
        for i in [1, 2, 4, 5] {
            at(mem, i).set(mem, w[i as usize].to_bits());
        }
    }

    /// `pppKeLnsLpSft(obj, key, ctrl)` (`0x00759f30`): adds the key's two
    /// floats at `+4` to work area 0, then (unless the key applied at frame
    /// 0) work area 0's two floats to work area 1's `+0x13c` and `+0x148`.
    pub fn ppp_ke_lns_lp_sft(mem: &mut impl Mem, obj: u32, key: u32, ctrl: u32) {
        if STOP.get(mem) != 0 {
            return;
        }
        let w0 = obj.wrapping_add(offset(mem, ctrl, 0));
        let w1 = obj.wrapping_add(offset(mem, ctrl, 1));
        let (a0, a4) = (Ptr::<u32>::new(w0 + 0xa0), Ptr::<u32>::new(w0 + 0xa4));
        if applies(mem, obj, key) {
            for (p, k) in [(a0, 4), (a4, 8)] {
                let v = x87::add(f32::from_bits(mem.read(key + k)), get_f32(mem, p));
                p.set(mem, v.to_bits());
            }
            if mem.read::<u32>(obj.wrapping_add(0xc)) == 0 {
                return;
            }
        }
        let p = Ptr::<u32>::new(w1.wrapping_add(0x13c));
        let v = x87::add(get_f32(mem, p), get_f32(mem, a0));
        p.set(mem, v.to_bits());
        let p = Ptr::<u32>::new(w1.wrapping_add(0x148));
        let v = x87::add(get_f32(mem, a4), get_f32(mem, p));
        p.set(mem, v.to_bits());
    }

    /// The packet buffer `pppSendPacket` fills (`0x010d5640`): a 0x14-byte
    /// head, then the data.
    pub const SEND_PACKET: Global<u32> = Global::new(0x010d_5640);
    /// The RPC client `pppSendPacket` calls (`0x010e7644`).
    pub const SEND_CLIENT: Global<u32> = Global::new(0x010e_7644);

    /// `pppSendPacket(rpc, tag, data, n)` (`0x0076b3d0`): waits for
    /// [`WAIT_DONE`], then builds the packet at [`SEND_PACKET`]: `u16 +0` =
    /// the size `n + 0x14` (also `rpc +8`, `rpc +0x10` = the packet), `u16
    /// +2` = 0, `u16 +4` = `rpc +4`, `u16 +6` = 0x4845, `+8`/`+0xc` = 0,
    /// `+0x10` the four bytes at `tag`, the `n` data bytes after
    /// (`memcpy`). [`WAIT_DONE`] = 0 around `FlushCache(0)` and
    /// `FFXSifCallRpc(client, 1, 0, packet, size, 0, 0, 0, 0)` (through
    /// `call`), then 1. Returns 0.
    pub fn ppp_send_packet<M: Mem>(
        mem: &mut M,
        rpc: u32,
        tag: u32,
        data: u32,
        n: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        while WAIT_DONE.get(mem) == 0 {
            // another thread clears the wait (pppClearWait)
            std::hint::spin_loop();
            std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
        }
        send_packet(mem, [rpc, tag, data, n], true, call)
    }

    /// `pppExSendPacket(rpc, tag, data, n)` (`0x0076b210`): [`ppp_send_packet`]
    /// without the waits: [`WAIT_DONE`] is only set (1) after the call.
    /// Returns 0.
    pub fn ppp_ex_send_packet<M: Mem>(
        mem: &mut M,
        rpc: u32,
        tag: u32,
        data: u32,
        n: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        send_packet(mem, [rpc, tag, data, n], false, call)
    }

    /// The packet of [`ppp_send_packet`] and [`ppp_ex_send_packet`]; `wait`
    /// clears [`WAIT_DONE`] before the flush.
    fn send_packet<M: Mem>(
        mem: &mut M,
        [rpc, tag, data, n]: [u32; 4],
        wait: bool,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let pk = SEND_PACKET.ptr(mem).addr;
        mem.write(rpc.wrapping_add(0x10), pk);
        mem.write(rpc.wrapping_add(8), n.wrapping_add(0x14));
        let size: u32 = mem.read(rpc.wrapping_add(8));
        let id: u32 = mem.read(rpc.wrapping_add(4));
        #[allow(clippy::cast_possible_truncation)] // the u16 fields
        mem.write(pk, [size as u16, 0, id as u16, 0x4845]);
        mem.write(pk.wrapping_add(8), [0u32; 2]);
        let t: [u8; 4] = mem.read(tag);
        mem.write(pk.wrapping_add(0x10), t);
        for i in 0..n {
            let b: u8 = mem.read(data.wrapping_add(i));
            mem.write(pk.wrapping_add(0x14 + i), b);
        }
        if wait {
            WAIT_DONE.set(mem, 0);
        }
        let flush = Global::<u32>::new(0x0062_f4c0).ptr(mem).addr;
        call(mem, flush, &[0]);
        let size: u32 = mem.read(rpc.wrapping_add(8));
        let at: u32 = mem.read(rpc.wrapping_add(0x10));
        let client = SEND_CLIENT.ptr(mem).addr;
        let rpc_call = Global::<u32>::new(0x00a4_4750).ptr(mem).addr;
        call(mem, rpc_call, &[client, 1, 0, at, size, 0, 0, 0, 0]);
        WAIT_DONE.set(mem, 1);
        0
    }

    /// `pppClearWait()` (`0x0076b200`).
    pub fn ppp_clear_wait(mem: &mut impl Mem) {
        WAIT_DONE.set(mem, 1);
    }

    /// The rotation `f` (`rot_x`, `rot_y`, `rot_z`) by `angle`, from
    /// `g_pppSinTable` and `r0`.
    pub fn rotation(
        mem: &impl Mem,
        angle: i32,
        f: fn([u32; 4], u32, u32) -> [u32; 16],
    ) -> [u32; 16] {
        let (si, ci) = super::sin_cos_index(angle);
        let table = SIN.ptr(mem);
        let (s, c) = (table.at(si).get(mem), table.at(ci).get(mem));
        f(R0.get(mem), s, c)
    }

    fn rot(mem: &mut impl Mem, out: u32, angle: i32, f: fn([u32; 4], u32, u32) -> [u32; 16]) {
        let m = rotation(mem, angle, f);
        mem.write(out, m);
    }

    /// `pppGetRotMatrixX(out, angle)` (`0x0072c730`).
    pub fn ppp_get_rot_matrix_x(mem: &mut impl Mem, out: u32, angle: i32) {
        rot(mem, out, angle, super::rot_x);
    }

    /// `pppGetRotMatrixY(out, angle)` (`0x0072c920`).
    pub fn ppp_get_rot_matrix_y(mem: &mut impl Mem, out: u32, angle: i32) {
        rot(mem, out, angle, super::rot_y);
    }

    /// `pppGetRotMatrixZ(out, angle)` (`0x0072cb10`).
    pub fn ppp_get_rot_matrix_z(mem: &mut impl Mem, out: u32, angle: i32) {
        rot(mem, out, angle, super::rot_z);
    }

    /// `pppDataRcv()` (`0x007614a0`; `pppPartDraw` and `pppPartLoop`),
    /// `frame` the original's `ebp` (its matrix at `frame - 0x44`): after
    /// `rpc_read(0x00e524a8)`, takes the `pmcom` queue's entries (8 bytes
    /// `{command, data}` at `0x010d4d28`, read index `0x010d5528` up to
    /// `g_KePmcomQueueIndex`, wrapping at 0x100) until it is empty or
    /// `0x00d52390` is set, each after `eidprintfPrint` and followed by
    /// another `rpc_read`:
    /// - 1, 2: the render state (`0x00d4b500`, 0x180 bytes, to
    ///   `0x00d4b680`); 1 copies four vectors on (`lmemcpy`) and always
    ///   builds the camera (`g_pppCameraMatrix` = camera x the received
    ///   matrix with y flipped and `+0x38` scaled by `0x00d4b700`) and
    ///   screen matrix (`FFXSamp0ViewScreenMatrix`, scale x 512), 2 copies
    ///   the battle camera instead when `0x00d4b7f8` is 1; then the
    ///   `0x00d4b4c8` records at `0x00d4e3a0` (0x80 apart) get `0x00d4b7ec`
    ///   and, with `0x00d4b7f0`, `0x00d4e378` = `60 / it x 0x107 - 0x83`.
    /// - 3: `pppSendPacket` of the init reply, `pppSendWait`.
    /// - 4, 8: an `rsd` record (`0x00d4ca40 + 0x20 i16 +0x18`): sizes, the
    ///   data and its two offsets; 4 then `pppDataSetTex_007628c0`, 8
    ///   `pppDataLastPair_00761250`; `g_pppPmcomTexFirst` = -1.
    /// - 5, 9: a shape (`0x00d4b828`, `0x00d4db40`); 5 resets the texture
    ///   animation. 7: a `vsf` (`0x00d4ba28`, `0x00d4dd40`). 10:
    ///   `pppSetPmcom2Texture_007629e0(data, 1)`.
    /// - 11: a new `pdt`: `pppFreeAllEnvObjects`, `pppInitPdt(data,
    ///   0x00c3e780, 0)`, the bounds reset, `FlushCache(0)`, the load count
    ///   `0x00d4b4c0` up.
    /// - 16, 21, 22: flags and copies after a `dbgPrintf`; 23
    ///   `AtelMapLoad(0x00d4b818)`; -1 nothing; anything else a
    ///   `dbgPrintf`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ppp_data_rcv<M: Mem>(
        mem: &mut M,
        frame: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        use ffx_bridge::x87::{load, mul, round_trip, store};
        use ffx_system::stub::ptr32;
        let g = |mem: &M, a: u32| Global::<u32>::new(a).ptr(mem).addr;
        let rd = |mem: &M, a: u32| mem.read::<u32>(g(mem, a));
        let wr = |mem: &mut M, a: u32, v: u32| {
            let at = g(mem, a);
            mem.write(at, v);
        };
        let mut fun = |mem: &mut M, f: u32, args: &[u32]| {
            let f = g(mem, f);
            call(mem, f, args)
        };
        let rpc = g(mem, 0x00e5_24a8);
        fun(mem, 0x0076_b4f0, &[rpc]);
        loop {
            let i = rd(mem, 0x010d_5528);
            if rd(mem, 0x010d_552c) == i {
                return;
            }
            let entry = g(mem, 0x010d_4d28).wrapping_add(i.wrapping_mul(8));
            let next = i.wrapping_add(1);
            wr(
                mem,
                0x010d_5528,
                if next.cast_signed() >= 0x100 { 0 } else { next },
            );
            if entry == 0 || rd(mem, 0x00d5_2390) != 0 {
                return;
            }
            fun(mem, 0x0076_b8d0, &[]);
            let cmd = mem.read::<u32>(entry).cast_signed();
            let data = |mem: &M| mem.read::<u32>(entry.wrapping_add(4));
            let print = |mem: &mut M,
                         fun: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
                         fmt: u32,
                         args: &[u32]| {
                let mut a = vec![g(mem, fmt)];
                a.extend_from_slice(args);
                fun(mem, 0x0062_fbf0, &a);
            };
            let debug = |mem: &mut M, fun: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, msg: u32| {
                let m = g(mem, msg);
                fun(mem, 0x0062_f500, &[m]);
            };
            match cmd {
                -1 => {}
                1 | 2 => {
                    if cmd == 1 {
                        let wide = |mem: &M, a: u32| {
                            let d = load(rd(mem, a)).to_bits();
                            #[allow(clippy::cast_possible_truncation)] // the halves
                            [d as u32, (d >> 32) as u32]
                        };
                        let (x, y) = (wide(mem, 0x00d4_b580), wide(mem, 0x00d4_b584));
                        print(mem, &mut fun, 0x00b5_1854, &[x[0], x[1], y[0], y[1]]);
                    }
                    let state: [u32; 0x60] = mem.read(g(mem, 0x00d4_b500));
                    mem.write(g(mem, 0x00d4_b680), state);
                    if cmd == 2 {
                        print(mem, &mut fun, 0x00b5_1868, &[]);
                    } else {
                        for (to, from) in [
                            (0x00d4_e3d0u32, 0x00d4_b6c0u32),
                            (0x00d4_e3e0, 0x00d4_b6d0),
                            (0x00d4_e3f0, 0x00d4_b6e0),
                            (0x0230_fd50, 0x00d4_b6f0),
                        ] {
                            let (to, from) = (g(mem, to), g(mem, from));
                            fun(mem, 0x0076_b0f0, &[to, from, 4]);
                        }
                    }
                    let (screen, camera) = (g(mem, 0x0230_fe60), g(mem, 0x0230_ff60));
                    if cmd == 2 && rd(mem, 0x00d4_b7f8) == 1 {
                        fun(mem, 0x007b_ba50, &[screen]);
                        fun(mem, 0x007c_0070, &[camera]);
                    } else {
                        let flip = frame.wrapping_sub(0x44);
                        fun(mem, 0x006e_d9a0, &[flip]);
                        let y = round_trip(f32::from_bits(rd(mem, 0x00b9_2340)));
                        mem.write(flip.wrapping_add(0x14), y);
                        let state = g(mem, 0x00d4_b680);
                        fun(mem, 0x006e_d5c0, &[camera, state, flip]);
                        let scale = load(rd(mem, 0x00d4_b700));
                        let z = load(mem.read(camera.wrapping_add(0x38)));
                        mem.write(camera.wrapping_add(0x38), store(mul(z, scale)));
                        fun(mem, 0x006e_d5c0, &[camera, flip, camera]);
                        let scale = load(rd(mem, 0x00d4_b700));
                        let far = store(mul(mul(scale, 65536.0), load(rd(mem, 0x00d4_b704))));
                        let one = 1f32.to_bits();
                        let args = [
                            screen,
                            store(mul(scale, 512.0)),
                            one,
                            round_trip(f32::from_bits(rd(mem, 0x00b5_1a18))).to_bits(),
                            rd(mem, 0x00b5_1a20),
                            rd(mem, 0x00b5_1a20),
                            one,
                            round_trip(f32::from_bits(rd(mem, 0x00b5_1a24))).to_bits(),
                            one,
                            far,
                        ];
                        fun(mem, 0x0093_d960, &args);
                        if cmd == 1 {
                            let s = store(mul(load(rd(mem, 0x00d4_b700)), 512.0));
                            wr(mem, 0x00c3_e720, s);
                        }
                    }
                    let n = rd(mem, 0x00d4_b4c8).cast_signed();
                    let v = rd(mem, 0x00d4_b7ec);
                    let base = g(mem, 0x00d4_e3a0);
                    for k in 0..n.max(0).cast_unsigned() {
                        mem.write(base.wrapping_add(0x80 * k), v);
                    }
                    let d = rd(mem, 0x00d4_b7f0).cast_signed();
                    if d != 0 {
                        let r = 60i32.wrapping_div(d).wrapping_mul(0x107).wrapping_sub(0x83);
                        wr(mem, 0x00d4_e378, r.cast_unsigned());
                    }
                }
                3 => {
                    print(mem, &mut fun, 0x00b5_1878, &[]);
                    let reply = g(mem, 0x00b5_188c);
                    fun(mem, 0x0076_b3d0, &[rpc, reply, 0, 0]);
                    fun(mem, 0x0076_b4a0, &[]);
                    print(mem, &mut fun, 0x00b5_1894, &[]);
                }
                4 | 8 => {
                    let d = data(mem);
                    print(mem, &mut fun, 0x00b5_18b0, &[d]);
                    let src = ptr32(data(mem));
                    let no = i32::from(mem.read::<i16>(src.wrapping_add(0x18)));
                    print(mem, &mut fun, 0x00b5_18d0, &[no.cast_unsigned()]);
                    let rec = g(mem, 0x00d4_ca40).wrapping_add(no.cast_unsigned() << 5);
                    mem.write(rec.wrapping_add(0x1c), ptr32(src));
                    for (to, from) in [(0u32, 0x12u32), (4, 0x14), (8, 0x10)] {
                        let v = i32::from(mem.read::<i16>(src.wrapping_add(from)));
                        mem.write(rec.wrapping_add(to), v);
                    }
                    for (to, from) in [(0x14u32, 8u32), (0x18, 0xc)] {
                        let v = mem.read::<u32>(src.wrapping_add(from)).wrapping_add(src);
                        mem.write(rec.wrapping_add(to), ptr32(v));
                    }
                    let (a, b) = (rec.wrapping_add(0xc), rec.wrapping_add(0x10));
                    let rsd = ptr32(mem.read(rec.wrapping_add(0x1c)));
                    if mem.read::<u32>(entry) == 4 {
                        let first = rd(mem, 0x010d_4cf4);
                        fun(mem, 0x0076_28c0, &[rsd, first, a, b]);
                    } else {
                        fun(mem, 0x0076_1250, &[rsd, a, b]);
                    }
                    wr(mem, 0x010d_4cf4, u32::MAX);
                }
                5 | 9 => {
                    let d = data(mem);
                    print(mem, &mut fun, 0x00b5_18ec, &[d]);
                    let src = ptr32(data(mem));
                    let no = mem.read::<u32>(src);
                    let shape = ptr32(data(mem));
                    mem.write(g(mem, 0x00d4_b828).wrapping_add(no.wrapping_mul(4)), shape);
                    let tex = ptr32(src.wrapping_add(0x10));
                    mem.write(g(mem, 0x00d4_db40).wrapping_add(no.wrapping_mul(4)), tex);
                    if mem.read::<u32>(entry) == 5 {
                        let count = rd(mem, 0x00d4_be34);
                        let at = g(mem, 0x00d4_be2c).wrapping_add(count.wrapping_mul(12));
                        let v = mem.read::<u32>(at);
                        let hi = v.cast_signed().wrapping_shr(31).cast_unsigned();
                        fun(mem, 0x007e_cf30, &[ptr32(tex), v, hi]);
                    }
                }
                7 => {
                    let d = data(mem);
                    print(mem, &mut fun, 0x00b5_190c, &[d]);
                    let src = ptr32(data(mem));
                    let no = mem.read::<u32>(src);
                    let vsf = ptr32(data(mem));
                    mem.write(g(mem, 0x00d4_ba28).wrapping_add(no.wrapping_mul(4)), vsf);
                    let at = g(mem, 0x00d4_dd40).wrapping_add(no.wrapping_mul(8));
                    let w: u16 = mem.read(src.wrapping_add(4));
                    mem.write(at, w);
                    let h: u16 = mem.read(src.wrapping_add(8));
                    mem.write(at.wrapping_add(2), h);
                    mem.write(at.wrapping_add(4), ptr32(src.wrapping_add(0xc)));
                }
                10 => {
                    print(mem, &mut fun, 0x00b5_192c, &[]);
                    let tex = ptr32(data(mem));
                    fun(mem, 0x0076_29e0, &[tex, 1]);
                }
                11 => {
                    let d = data(mem);
                    print(mem, &mut fun, 0x00b5_1948, &[d]);
                    fun(mem, 0x0076_05d0, &[]);
                    let pdt = ptr32(data(mem));
                    wr(mem, 0x00d4_bc30, pdt);
                    let programs = g(mem, 0x00c3_e780);
                    fun(mem, 0x0071_20f0, &[pdt, programs, 0]);
                    let max = round_trip(f32::from_bits(rd(mem, 0x00b1_7504))).to_bits();
                    let hundred = round_trip(f32::from_bits(rd(mem, 0x00b3_04f8))).to_bits();
                    for (at, v) in [
                        (0x010d_2528u32, max),
                        (0x00d4_e3b0, max),
                        (0x010d_2520, 0),
                        (0x00d4_e390, 0),
                        (0x010d_2530, hundred),
                        (0x010d_2524, 0),
                        (0x010d_252c, hundred),
                        (0x00d4_b4c8, 1),
                        (0x00d4_e3b8, hundred),
                        (0x00d4_b4c4, 1),
                        (0x00d4_e3b4, hundred),
                        (0x0230_fd38, 0),
                    ] {
                        wr(mem, at, v);
                    }
                    fun(mem, 0x0062_f4c0, &[0]);
                    let n = rd(mem, 0x00d4_b4c0).wrapping_add(1);
                    wr(mem, 0x00d4_b4c0, n);
                }
                16 => {
                    debug(mem, &mut fun, 0x00b5_1968);
                    let at = g(mem, 0x00d4_e3ad);
                    mem.write(at, 1u8);
                }
                21 => {
                    debug(mem, &mut fun, 0x00b5_1980);
                    let v = rd(mem, 0x00d4_b808);
                    wr(mem, 0x00d4_b810, v);
                    let v = rd(mem, 0x00d4_b80c);
                    wr(mem, 0x00d4_b814, v);
                }
                22 => {
                    debug(mem, &mut fun, 0x00b5_198c);
                    let v = rd(mem, 0x00d4_b800);
                    wr(mem, 0x00d4_b804, v);
                    let at = g(mem, 0x00d5_249e);
                    mem.write(at, 1u8);
                }
                23 => {
                    let map = rd(mem, 0x00d4_b818);
                    fun(mem, 0x0086_dca0, &[map]);
                }
                0xfe => debug(mem, &mut fun, 0x00b5_19a0),
                _ => debug(mem, &mut fun, 0x00b5_19bc),
            }
            fun(mem, 0x0076_b4f0, &[rpc]);
        }
    }
}
