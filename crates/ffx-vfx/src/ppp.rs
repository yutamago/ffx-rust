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

    /// `pppClearWait()` (`0x0076b200`).
    pub fn ppp_clear_wait(mem: &mut impl Mem) {
        WAIT_DONE.set(mem, 1);
    }

    fn rot(mem: &mut impl Mem, out: u32, angle: i32, f: fn([u32; 4], u32, u32) -> [u32; 16]) {
        let (si, ci) = super::sin_cos_index(angle);
        let table = SIN.ptr(mem);
        let (s, c) = (table.at(si).get(mem), table.at(ci).get(mem));
        let m = f(R0.get(mem), s, c);
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
}
