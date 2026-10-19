//! The battle camera's and screens' shakes (`docs/rewrite-plan.md`, M2).
//!
//! A shake record is 0x24 bytes: `+0` the kind (0 idle, 1-9, see
//! [`bridge::ms_camera_shake_process`]), `+1` the jitter range, `+2` the
//! amplitude (`u16`), `+4` the frames left and `+6` the length (`i16`),
//! `+8` the last value (kinds 5 and 6), and for the spring (kind 9) `+0xc`
//! its start, `+0x10` its position, `+0x14` its speed, `+0x18` its pull,
//! `+0x1c` the start pull, `+0x20` steps a frame, `+0x21` the fade and
//! `+0x22` the random range it was set with.
//!
//! Each camera side (`g_Cameras` + camera · 0xda0 + side · 0x61c) has three
//! records at `+0x5e0` (x, y, z), each with a value at `+0xc0`, `+0xd0` and
//! `+0xe0` + 4 k; each screen (`0x1c0` bytes from `0x01137000`) two at
//! `+0x16c`, with values at `+0x15c`, `+0x164` + 4 k.

/// Bytes per shake record.
pub const RECORD: u32 = 0x24;

/// The jitter `MsCameraShakeProcess` adds to the amplitude's `32`: half the
/// range less a random remainder (`idiv`: the sign of `roll`).
#[must_use]
pub fn jitter(range: u8, roll: u32) -> i32 {
    i32::from(range >> 1) - roll.cast_signed() % i32::from(range)
}

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::x87::{add, div, load, mul, store};
    use ffx_bridge::{Global, Mem};

    use super::{RECORD, jitter};
    use crate::camera::bridge::{CAMERA_SIZE, CAMERAS, SCREEN_SIZE, SCREENS, ms_camera_get_id};
    use crate::camera::x87_copy;
    use crate::pos::bridge::Trig;
    use crate::rng::bridge::brnd;

    /// Bytes per camera side.
    pub const SIDE_SIZE: u32 = 0x61c;
    /// The shake records' offset in a camera side.
    pub const CAMERA_SHAKES: u32 = 0x5e0;
    /// The shake records' offset in a screen.
    pub const SCREEN_SHAKES: u32 = 0x16c;
    /// `1/32` (`0x00b501a0`), `0.3` (`0x00b551e8`), `0.1` (`0x00b551e0`),
    /// `0.5` (`0x00b92220`) and `1/256` (`0x00b50198`), as doubles.
    pub const THIRTY_SECOND: Global<f64> = Global::new(0x00b5_01a0);
    pub const PHASE_3: Global<f64> = Global::new(0x00b5_51e8);
    pub const PHASE_1: Global<f64> = Global::new(0x00b5_51e0);
    pub const HALF: Global<f64> = Global::new(0x00b9_2220);
    pub const TWO_FIFTY_SIXTH: Global<f64> = Global::new(0x00b5_0198);

    /// Camera `cam`'s side `side` (`g_Cameras` + `cam` · 0xda0 + `side` ·
    /// 0x61c).
    fn camera_side(mem: &impl Mem, cam: u32, side: u32) -> u32 {
        CAMERAS
            .ptr(mem)
            .addr
            .wrapping_add(cam.wrapping_mul(CAMERA_SIZE))
            .wrapping_add(side.wrapping_mul(SIDE_SIZE))
    }

    /// Screen `i`.
    fn screen(mem: &impl Mem, i: u32) -> u32 {
        SCREENS
            .ptr(mem)
            .addr
            .wrapping_add(i.wrapping_mul(SCREEN_SIZE))
    }

    /// `MsCameraCheckShake(cam, side)` (`0x007c4f10`): -1 while one of the
    /// three shakes of camera `cam & 0xff`'s side `side & 1` runs, else 0.
    pub fn ms_camera_check_shake(mem: &impl Mem, cam: u32, side: u32) -> i32 {
        let base = camera_side(mem, cam & 0xff, side & 1) + CAMERA_SHAKES;
        let busy = (0..3).any(|k| mem.read::<u8>(base + RECORD * k) != 0);
        if busy { -1 } else { 0 }
    }

    /// `MsScreenCheckShake(i)` (`0x007c5680`): -1 while one of screen
    /// `i`'s two shakes runs, else 0 (and 0 for `i` above 2).
    pub fn ms_screen_check_shake(mem: &impl Mem, i: u32) -> i32 {
        if i > 2 {
            return 0;
        }
        let base = screen(mem, i) + SCREEN_SHAKES;
        let busy = (0..2).any(|k| mem.read::<u8>(base + RECORD * k) != 0);
        if busy { -1 } else { 0 }
    }

    /// `MsCameraResetShake(id, side)` (`0x007c4f50`): for a camera whose
    /// current request is `id` ([`ms_camera_get_id`]), stops the three
    /// shakes of its side `side & 1` and zeroes their values at `+0xc0`.
    /// Returns `eax` as the original leaves it: `side` · 0x61c, or the
    /// current request when it is not `id`.
    pub fn ms_camera_reset_shake(mem: &mut impl Mem, id: u32, side: u32) -> u32 {
        let current = ms_camera_get_id(mem, id);
        if current != id {
            return current;
        }
        let base = camera_side(mem, id & 0xff, side & 1);
        for k in 0..3 {
            mem.write(base + CAMERA_SHAKES + RECORD * k, 0u8);
            mem.write(base + 0xc0 + 4 * k, 0u32);
        }
        (side & 1) * SIDE_SIZE
    }

    /// The parameters both `MsCameraShake` and `MsScreenShake` set.
    #[derive(Debug, Clone, Copy)]
    pub struct Start {
        pub kind: u8,
        /// `f32` bits, into the value at `+0xe0` (camera) or `+0x164`
        /// (screen).
        pub value: u32,
        pub length: u16,
        pub amplitude: u16,
        pub range: u8,
    }

    /// One record started: kind, range, amplitude, frames left and length,
    /// the last value 0.
    fn start(mem: &mut impl Mem, rec: u32, s: Start) {
        mem.write(rec, s.kind);
        mem.write(rec + 1, s.range);
        mem.write(rec + 2, s.amplitude);
        mem.write(rec + 4, s.length);
        mem.write(rec + 6, s.length);
        mem.write(rec + 8, 0u32);
    }

    /// `MsCameraShake(id, side, which, kind, value, length, amplitude,
    /// range)` (`0x007c4fc0`): for a camera whose current request is `id`,
    /// starts the shakes in `which` (bits 0-2) of its side `side & 1`,
    /// their values at `+0xd0` zeroed and at `+0xe0` set to `value`.
    /// Returns `eax` as [`ms_camera_reset_shake`] does.
    pub fn ms_camera_shake(mem: &mut impl Mem, id: u32, side: u32, which: u32, s: Start) -> u32 {
        let current = ms_camera_get_id(mem, id);
        if current != id {
            return current;
        }
        let base = camera_side(mem, id & 0xff, side & 1);
        let value = x87_copy(s.value);
        for k in 0..3 {
            if which & (1 << k) != 0 {
                start(mem, base + CAMERA_SHAKES + RECORD * k, s);
                mem.write(base + 0xd0 + 4 * k, 0u32);
                mem.write(base + 0xe0 + 4 * k, value);
            }
        }
        (side & 1) * SIDE_SIZE
    }

    /// `MsScreenShake(i, which, kind, value, length, amplitude, range)`
    /// (`0x007c56f0`): starts the shakes in `which` (bits 0-1) of screen
    /// `i` (below 3), their values at `+0x15c` zeroed and at `+0x164` set to
    /// `value`.
    pub fn ms_screen_shake(mem: &mut impl Mem, i: u32, which: u32, s: Start) {
        if i > 2 {
            return;
        }
        let base = screen(mem, i);
        let value = x87_copy(s.value);
        for k in 0..2 {
            if which & (1 << k) != 0 {
                start(mem, base + SCREEN_SHAKES + RECORD * k, s);
                mem.write(base + 0x15c + 4 * k, 0u32);
                mem.write(base + 0x164 + 4 * k, value);
            }
        }
    }

    /// `MsScreenShake2(i, which, kind, steps, length, speed, range, pull,
    /// fade)` (`0x007c57b0`): starts screen `i`'s springs (kind 9) in
    /// `which` (bits 0-1) when `i` is below 3 and `length` is not 0: start
    /// `speed`, start pull `pull`, position 0, the random `range` kept
    /// (`i16`). A negative range turns the speed round and is used as
    /// positive; a nonzero one scales speed and pull by
    /// `(256 - brnd(9) % range) / 256` (the `f32` of the integer, then
    /// times each, then times 1/256, stored), carried to the next record
    /// as the next start.
    #[allow(clippy::too_many_arguments)]
    pub fn ms_screen_shake2(
        mem: &mut impl Mem,
        i: u32,
        which: u32,
        kind: u8,
        steps: u8,
        length: u32,
        speed: u32,
        range: i32,
        pull: u32,
        fade: u8,
    ) {
        if i > 2 || length == 0 {
            return;
        }
        let base = screen(mem, i) + SCREEN_SHAKES;
        let k256 = TWO_FIFTY_SIXTH.get(mem);
        let (mut speed, mut pull, mut range) = (x87_copy(speed), x87_copy(pull), range);
        for k in 0..2 {
            if which & (1 << k) == 0 {
                continue;
            }
            let rec = base + RECORD * k;
            mem.write(rec + 0x10, 0u32);
            mem.write(rec + 0x20, steps);
            mem.write(rec + 0xc, speed);
            mem.write(rec, kind);
            mem.write(rec + 0x1c, pull);
            let [l0, l1, ..] = length.to_le_bytes();
            mem.write(rec + 4, [l0, l1]);
            mem.write(rec + 6, [l0, l1]);
            let [r0, r1, ..] = range.to_le_bytes();
            mem.write(rec + 0x22, [r0, r1]);
            if range < 0 {
                range = range.wrapping_neg();
                speed ^= 0x8000_0000;
            }
            if range != 0 {
                let roll = brnd(mem, 9).cast_signed();
                let n = 256 - roll.wrapping_rem(range);
                #[allow(clippy::cast_precision_loss)] // fild, fstp dword
                let f = load((n as f32).to_bits());
                speed = store(mul(mul(f, load(speed)), k256));
                pull = store(mul(mul(f, load(pull)), k256));
            }
            mem.write(rec + 0x14, speed);
            mem.write(rec + 0x21, fade);
            mem.write(rec + 0x18, pull);
        }
    }

    /// Kind 9's spring, `+0x20` steps: `pos += speed` (stored), then
    /// `speed += pull`, the pull turned round while `pos` is above 0.
    fn spring(mem: &mut impl Mem, shake: u32) {
        let at = |o: u32| shake.wrapping_add(o);
        let steps: u8 = mem.read(at(0x20));
        for _ in 0..steps {
            let speed: u32 = mem.read(at(0x14));
            let pos: u32 = mem.read(at(0x10));
            let pos = store(add(load(speed), load(pos)));
            mem.write(at(0x10), pos);
            let mut pull = x87_copy(mem.read(at(0x18)));
            if load(pos) > 0.0 {
                pull ^= 0x8000_0000;
            }
            let speed: u32 = mem.read(at(0x14));
            mem.write(at(0x14), store(add(load(pull), load(speed))));
        }
    }

    /// `MsCameraShakeProcess(shake, angle, out)` (`0x007c50f0`): one frame
    /// of the shake record at `shake` into `*out` (`f32`).
    ///
    /// A jitter ([`super::jitter`], `brnd(9)`) when the range is set; then,
    /// for a nonzero length, a wave `w` times the amplitude, `(j + 32)`
    /// and 1/32 (each product at the x87's 53 bits):
    /// - 1, 2, 7: `w = sin(angle)`;
    /// - 3, 4: `w = (sin(angle) + cos(angle + 0.3)) · 0.5`;
    /// - 5, 6, 8: the same with `+ 0.1`.
    ///
    /// Kinds 1-6 fade it in (odd: × frames left / length) or out (even:
    /// × (length − frames left) / length); 5 and 6 draw `brnd(9)` first
    /// and hold half the last value instead on odd frames or when its low
    /// nibble is 0, keeping the new value at `+8`; 7 and 8 do not fade.
    /// Kind 9 is a spring: `steps` times `pos += speed`, `speed += pull`
    /// (`-pull` while `pos` is above 0), its position faded by the mode
    /// (1 in, 2 out, else as is). Then a frame is counted down; below 0
    /// the shake ends (kind 0, `*out` 0), for kinds 7-9 only with a
    /// positive length. `trig` is `_CIcos`/`_CIsin` on a double, the
    /// result stored as `f32`.
    pub fn ms_camera_shake_process<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, f64) -> u32,
        shake: u32,
        angle: u32,
        out: u32,
    ) {
        let at = |o: u32| shake.wrapping_add(o);
        let kind: u8 = mem.read(shake);
        if kind == 0 {
            return;
        }
        let range: u8 = mem.read(at(1));
        let jitter = if range == 0 {
            0
        } else {
            jitter(range, brnd(mem, 9))
        };
        if !(1..=9).contains(&kind) {
            return;
        }
        let amplitude = f64::from(mem.read::<u16>(at(2)));
        let left: i16 = mem.read(at(4));
        let length: i16 = mem.read(at(6));
        let lengthf = f64::from(length);
        let (k32, half) = (THIRTY_SECOND.get(mem), HALF.get(mem));
        let (phase_3, phase_1) = (PHASE_3.get(mem), PHASE_1.get(mem));
        // `fild`, then `fstp dword`: exact for these small integers
        #[allow(clippy::cast_precision_loss)]
        let as_f32 = |n: i32| load((n as f32).to_bits());
        let scaled = |w: f64| mul(mul(mul(w, amplitude), as_f32(jitter + 32)), k32);
        let angle_f = load(angle);
        let sine = || load(trig(Trig::Sin, angle_f));
        // the cosine first, then the sine
        let blend = |phase: f64| {
            let c = load(trig(Trig::Cos, add(angle_f, phase)));
            let s = sine();
            mul(add(s, c), half)
        };
        let fade_in = f64::from(left);
        let fade_out = as_f32(i32::from(length) - i32::from(left));
        let count_down = |mem: &mut M, needs_length: bool| {
            let left = left.wrapping_sub(1);
            mem.write(at(4), left);
            if left < 0 && (!needs_length || length > 0) {
                mem.write(shake, 0u8);
                mem.write(out, 0u32);
            }
        };
        match kind {
            1..=4 => {
                if length == 0 {
                    return;
                }
                let w = if kind <= 2 { sine() } else { blend(phase_3) };
                let a = load(store(scaled(w)));
                let fade = if kind % 2 == 1 { fade_in } else { fade_out };
                mem.write(out, store(div(mul(a, fade), lengthf)));
                count_down(mem, false);
            }
            5 | 6 => {
                let roll = brnd(mem, 9) & 0xf;
                let mut a = store(scaled(blend(phase_1)));
                if length == 0 {
                    return;
                }
                if left & 1 != 0 || roll == 0 {
                    let last: u32 = mem.read(at(8));
                    a = store(mul(half, load(last)));
                }
                let fade = if kind == 5 { fade_in } else { fade_out };
                mem.write(out, store(div(mul(fade, load(a)), lengthf)));
                mem.write(at(8), a);
                count_down(mem, false);
            }
            7 | 8 => {
                if length == 0 {
                    return;
                }
                let w = if kind == 7 { sine() } else { blend(phase_1) };
                mem.write(out, store(scaled(w)));
                count_down(mem, true);
            }
            _ => {
                if length == 0 {
                    return;
                }
                spring(mem, shake);
                let pos = x87_copy(mem.read(at(0x10)));
                let mode: u8 = mem.read(at(0x21));
                let v = match mode {
                    1 => store(div(mul(fade_in, load(pos)), lengthf)),
                    2 => store(div(mul(fade_out, load(pos)), lengthf)),
                    _ => pos,
                };
                mem.write(out, v);
                count_down(mem, true);
            }
        }
    }
}
