//! h2-render-b, last round, batch 2: `PSceneWideParameters::getBuffer`
//! (`0x004fdec0`), the per-frame scene-wide parameter buffer.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// The callees.
const KEY_INITIALIZE: u32 = 0x004f_e940;
const LOCK: u32 = 0x0043_1920;
const UNLOCK: u32 = 0x0043_1a80;
const STORE_KEY: u32 = 0x004f_d2a0;
const SLEEP: u32 = 0x0043_0030;
const PRINTF: u32 = 0x0043_52f0;
const INVERSE: u32 = 0x0045_1080;
const MATRIX_PRODUCT: u32 = 0x004c_c0c0;
const MATRIX_COPY: u32 = 0x004c_9430;
const IDENTITY_INIT: u32 = 0x004f_ee70;
const ATEXIT: u32 = 0x0094_9207;
const IS_BOUND: u32 = 0x0059_5f70;
const BIND: u32 = 0x0058_cf20;
const MAPPING_CTOR: u32 = 0x0058_44e0;
const MAPPING_DTOR: u32 = 0x0058_59f0;
const RING_NEXT: u32 = 0x004f_8460;
const TRANSPOSE: u32 = 0x004f_93f0;
const MATRIX_STORE: u32 = 0x0045_3340;
const MARK_SLOT: u32 = 0x0059_d320;

/// The one-time object (`0x00c94f00`, guard bit 0 of `0x00c9628c`) and
/// its `atexit` destructor.
const ONCE_GUARD: u32 = 0x00c9_628c;
const ONCE_OBJECT: u32 = 0x00c9_4f00;
const ONCE_AT_EXIT: u32 = 0x00af_b710;
/// The back buffer's width and height (unsigned words).
const SCREEN_WIDTH: u32 = 0x00c9_5100;
const SCREEN_HEIGHT: u32 = 0x00c9_5104;
/// "Warning: Waiting for Scene Wide parameter buffer to become free ...".
const WAIT_WARNING: u32 = 0x00b2_4088;

/// The frame offsets (from the original's `ebp`) of the key and the
/// matrices the callees fill, and of the values copied out.
mod at {
    /// The 0x110-byte key (`PSceneWideParametersKey`).
    pub const KEY: i32 = -0x190;
    /// Matrices in the key: the view (`-0x190`), the projection
    /// (`-0x150`) and the one inverted (`-0x110`).
    pub const PROJECTION: i32 = -0x150;
    pub const TO_INVERT: i32 = -0x110;
    pub const INVERSE: i32 = -0x250;
    pub const PRODUCT: i32 = -0x490;
    pub const COMBINED: i32 = -0x1d0;
    pub const INVERSE2: i32 = -0x210;
    pub const PRODUCT2: i32 = -0x310;
    pub const COMBINED2: i32 = -0x290;
    pub const T_INVERSE2: i32 = -0x450;
    pub const T_VIEW: i32 = -0x3d0;
    pub const T_PROJECTION: i32 = -0x350;
    pub const T_COMBINED2: i32 = -0x390;
    pub const T_COMBINED: i32 = -0x410;
    pub const COMBINED_T: i32 = -0x2d0;
    /// Vectors and scalars in the key.
    pub const POSITION: i32 = -0xd0;
    pub const DIRECTION: i32 = -0xc0;
    pub const NEAR: i32 = -0xb0;
    pub const FAR: i32 = -0xac;
    pub const VIEWPORT: i32 = -0xa8;
    pub const UP: i32 = -0xa0;
    pub const TIME: i32 = -0x90;
    /// The mapped slot (`PConstantBufferMapping`, one word).
    pub const MAPPING: i32 = -0x30;
    /// The values copied out: a vector, three records, a scalar (the
    /// third argument's slot).
    pub const VECTOR: i32 = -0x24;
    pub const SLOT_RECORD: i32 = -0x80;
    pub const TEXTURE_RECORD: i32 = -0x68;
    pub const SAMPLER_RECORD: i32 = -0x50;
    pub const SCALAR: i32 = 0x10;
}

/// `fld m32; fstp m32`.
fn copy(w: u32) -> u32 {
    x87::store(x87::load(w))
}

pub mod bridge {
    //! The originals over game memory.

    use super::{
        ATEXIT, BIND, Conv, Host, IDENTITY_INIT, INVERSE, IS_BOUND, KEY_INITIALIZE, LOCK,
        MAPPING_CTOR, MAPPING_DTOR, MARK_SLOT, MATRIX_COPY, MATRIX_PRODUCT, MATRIX_STORE,
        ONCE_AT_EXIT, ONCE_GUARD, ONCE_OBJECT, PRINTF, RING_NEXT, SCREEN_HEIGHT, SCREEN_WIDTH,
        SLEEP, STORE_KEY, TRANSPOSE, UNLOCK, WAIT_WARNING, at, copy, x87,
    };

    /// The bytes of `getBuffer`'s frame below `ebp` the port uses
    /// (`ebp - 0x490` up to it), and above it (the third argument's
    /// slot, `ebp + 0x10`).
    pub const FRAME_BELOW: u32 = 0x490;
    pub const FRAME_ABOVE: u32 = 0x14;

    /// `u32 -> f32` as `fild` (plus 2^32 for the top bit) and `fstp`
    /// round it.
    fn unsigned_to_f32(u: u32) -> u32 {
        x87::store(f64::from(u))
    }

    /// `PSceneWideParameters::getBuffer(this; a, camera, c)`
    /// (`0x004fdec0`, thiscall, `ret 0xc`): the key of `(a, camera, c)`
    /// (`PSceneWideParametersKey::initialize`, into the frame's
    /// `ebp - 0x190`); under the lock `+0x1c`, when it equals the current
    /// buffer's key (`+0x38`, two 0x110-byte keys by `[+0x34]` & 1) the
    /// current buffer (`+0xc` + its index x the stride `+8`, u16).
    /// Else the next of `[+0x10]` buffers (the index wraps to 0): the key
    /// stored (`0x004fd2a0`), a wait while the buffer's use count (`+4`)
    /// is set (`Sleep(1)`; past a million rounds a warning and a write to
    /// address 0), its first word the stride, the matrices computed into
    /// the frame (inverse, product, copy), the one-time object
    /// `0x00c94f00`, the screen size as floats; the buffer's constant
    /// buffer (`+0x18`, 0x3c bytes each) bound when it is not, a ring
    /// slot mapped (`0x004f8460` into `ebp - 0x30`), and when it was:
    /// four transposed matrices, the camera vectors, near/far and their
    /// difference and product, the viewport, the screen size and its
    /// inverse written into it, the slot marked and the cursor advanced.
    /// Then every parameter definition (16 bytes at `[+4]`, count `[+0]`)
    /// whose value exists and fits (offset `+8` at least 4, size `+0xa` &
    /// 0x1fff, their sum within the buffer's first word) has its value
    /// copied into the buffer: by its type (byte `+2`, 0..0x1c) a vector
    /// of the key, a frame matrix, a scalar, a record (slot, texture or
    /// sampler), or a block of `camera`. Unlocks and returns the buffer.
    /// `ebp` is the original's frame ([`FRAME_BELOW`], [`FRAME_ABOVE`]).
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    pub fn get_buffer(h: &mut impl Host, this: u32, a: u32, camera: u32, c: u32, ebp: u32) -> u32 {
        let f = |off: i32| ebp.wrapping_add_signed(off);
        let key = f(at::KEY);
        h.call_sym(KEY_INITIALIZE, Conv::Thiscall(key), &[a, camera, c]);
        let lock = this + 0x1c;
        h.call_sym(LOCK, Conv::Thiscall(lock), &[]);
        let current: u32 = h.read(this + 0x34);
        let slot = this + 0x38 + (current & 1) * 0x110;
        let same = (0..0x44).all(|k| h.read::<u32>(slot + 4 * k) == h.read::<u32>(key + 4 * k));
        if same {
            let stride = u32::from(h.read::<u16>(this + 8));
            let buffer = stride
                .wrapping_mul(h.read(this + 0x34))
                .wrapping_add(h.read(this + 0xc));
            h.call_sym(UNLOCK, Conv::Thiscall(lock), &[]);
            return buffer;
        }
        let next = current.wrapping_add(1);
        let index = if next == h.read::<u32>(this + 0x10) {
            0
        } else {
            next
        };
        h.write(this + 0x34, index);
        h.call_sym(
            STORE_KEY,
            Conv::Thiscall(this + 0x38 + (index & 1) * 0x110),
            &[key],
        );
        let buffer = u32::from(h.read::<u16>(this + 8))
            .wrapping_mul(index)
            .wrapping_add(h.read(this + 0xc));
        let constants = h
            .read::<u32>(this + 0x18)
            .wrapping_add(index.wrapping_mul(0x3c));
        if h.read::<u32>(buffer + 4) != 0 {
            let mut rounds = 0u32;
            while h.read::<u32>(buffer + 4) != 0 {
                h.call_sym(SLEEP, Conv::Cdecl, &[1]);
                let waited = rounds;
                rounds = rounds.wrapping_add(1);
                if waited.cast_signed() > 1_000_000 {
                    let warning = h.rebase(WAIT_WARNING);
                    let refs: u32 = h.read(buffer + 4);
                    h.call_sym(PRINTF, Conv::Cdecl, &[2, warning, refs]);
                    // the original's deliberate crash
                    h.write(0, 0u32);
                }
            }
        }
        let stride = u32::from(h.read::<u16>(this + 8));
        h.write(buffer, stride);
        h.call_sym(INVERSE, Conv::Thiscall(f(at::INVERSE)), &[f(at::TO_INVERT)]);
        let p = h.call_sym(
            MATRIX_PRODUCT,
            Conv::Cdecl,
            &[f(at::PRODUCT), f(at::INVERSE)],
        );
        h.call_sym(MATRIX_COPY, Conv::Thiscall(f(at::COMBINED)), &[p]);
        let guard = h.rebase(ONCE_GUARD);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            let object = h.rebase(ONCE_OBJECT);
            h.call_sym(IDENTITY_INIT, Conv::Thiscall(object), &[]);
            let d = h.rebase(ONCE_AT_EXIT);
            h.call_sym(ATEXIT, Conv::Cdecl, &[d]);
        }
        let width = unsigned_to_f32(h.read(h.rebase(SCREEN_WIDTH)));
        let height = unsigned_to_f32(h.read(h.rebase(SCREEN_HEIGHT)));
        // `isBound` answers in `al`
        let bound = h.call_sym(IS_BOUND, Conv::Thiscall(constants), &[]).to_le_bytes()[0];
        if bound == 0 {
            h.call_sym(BIND, Conv::Thiscall(constants), &[]);
        }
        let mapping = f(at::MAPPING);
        h.call_sym(MAPPING_CTOR, Conv::Thiscall(mapping), &[]);
        let near_at = f(at::NEAR);
        let far_at = f(at::FAR);
        if h.call_sym(RING_NEXT, Conv::Thiscall(constants), &[mapping]) == 0 {
            let m: u32 = h.read(mapping);
            h.call_sym(
                INVERSE,
                Conv::Thiscall(f(at::INVERSE2)),
                &[f(at::TO_INVERT)],
            );
            let p = h.call_sym(
                MATRIX_PRODUCT,
                Conv::Cdecl,
                &[f(at::PRODUCT2), f(at::INVERSE2)],
            );
            h.call_sym(MATRIX_COPY, Conv::Thiscall(f(at::COMBINED2)), &[p]);
            for (to, out, from) in [
                (m + 0x40, at::T_INVERSE2, at::INVERSE2),
                (m + 0x80, at::T_VIEW, at::KEY),
                (m, at::T_PROJECTION, at::PROJECTION),
                (m + 0xc0, at::T_COMBINED2, at::COMBINED2),
            ] {
                let t = h.call_sym(TRANSPOSE, Conv::Cdecl, &[f(out), f(from)]);
                h.call_sym(MATRIX_STORE, Conv::Thiscall(to), &[t]);
            }
            let near: u32 = h.read(near_at);
            let far: u32 = h.read(far_at);
            h.write(m + 0x120, copy(near));
            h.write(m + 0x124, copy(far));
            h.write(
                m + 0x11c,
                x87::store(x87::sub(x87::load(far), x87::load(near))),
            );
            h.write(
                m + 0x10c,
                x87::store(x87::mul(x87::load(near), x87::load(far))),
            );
            for (to, from) in [
                (0x100, at::DIRECTION),
                (0x104, at::DIRECTION + 4),
                (0x108, at::DIRECTION + 8),
                (0x110, at::UP),
                (0x114, at::UP + 4),
                (0x118, at::UP + 8),
                (0x140, at::TIME),
            ] {
                let v: u32 = h.read(f(from));
                h.write(m + to, copy(v));
            }
            h.write(m + 0x138, width);
            h.write(m + 0x13c, height);
            h.write(m + 0x130, x87::store(x87::div(1.0, x87::load(width))));
            h.write(m + 0x134, x87::store(x87::div(1.0, x87::load(height))));
            let vx: u32 = h.read(f(at::VIEWPORT));
            h.write(m + 0x128, copy(vx));
            let vy: u32 = h.read(f(at::VIEWPORT + 4));
            h.write(m + 0x12c, copy(vy));
            let next = h.read::<u32>(constants + 4).wrapping_add(1);
            let mut slots = h.read::<u32>(constants + 0xc) & 0x7fff_ffff;
            if slots == 0 {
                slots = 1;
            }
            let next = if next == slots { 0 } else { next };
            if h.call_sym(MARK_SLOT, Conv::Thiscall(constants), &[next, mapping]) == 0 {
                h.write(constants + 4, next);
                h.write(constants + 8, 0u8);
            }
        }
        let mut count = h.read::<u32>(this) & 0x7fff_ffff;
        if count != 0 {
            let vector = f(at::VECTOR);
            let slot_record = f(at::SLOT_RECORD);
            let texture_record = f(at::TEXTURE_RECORD);
            let sampler_record = f(at::SAMPLER_RECORD);
            let scalar = f(at::SCALAR);
            h.write(texture_record, 0x42u32);
            h.write(slot_record, 0x4au32);
            h.write(slot_record + 8, 0u32);
            let mut near: u32 = h.read(near_at);
            let mut far: u32 = h.read(far_at);
            let mut offset = 0u32;
            loop {
                let entry = h.read::<u32>(this + 4).wrapping_add(offset);
                let kind = h.read::<u8>(entry + 2);
                for w in [texture_record + 4, texture_record + 8, texture_record + 0xc] {
                    h.write(w, 0u32);
                }
                h.write(slot_record + 4, 0u32);
                h.write(slot_record + 0xc, 0u32);
                h.write(sampler_record, 0x49u32);
                for k in 1..5 {
                    h.write(sampler_record + 4 * k, 0u32);
                }
                // the value's address, or none
                let source = match kind {
                    0 | 1 => {
                        let from = if kind == 0 {
                            at::POSITION
                        } else {
                            at::DIRECTION
                        };
                        for k in 0..3 {
                            let v: u32 = h.read(f(from + 4 * k));
                            h.write(vector + 4 * k as u32, copy(v));
                        }
                        h.write(vector + 0xc, if kind == 0 { 0u32 } else { 0x3f80_0000 });
                        Some(vector)
                    }
                    2 => Some(key),
                    3 => Some(f(at::INVERSE)),
                    4 => Some(f(at::COMBINED)),
                    5 => {
                        let t = h.call_sym(
                            TRANSPOSE,
                            Conv::Cdecl,
                            &[f(at::T_COMBINED), f(at::COMBINED)],
                        );
                        h.call_sym(MATRIX_STORE, Conv::Thiscall(f(at::COMBINED_T)), &[t]);
                        near = h.read(near_at);
                        far = h.read(far_at);
                        Some(f(at::COMBINED_T))
                    }
                    6 => Some(f(at::PROJECTION)),
                    7 => {
                        h.write(vector, copy(near));
                        h.write(vector + 4, copy(far));
                        Some(vector)
                    }
                    8..=0xb => {
                        let v = match kind {
                            8 => x87::mul(x87::load(near), x87::load(far)),
                            9 => x87::sub(x87::load(far), x87::load(near)),
                            0xa => x87::div(1.0, x87::sub(x87::load(far), x87::load(near))),
                            _ => {
                                let m5: u32 = h.read(f(at::KEY + 0x14));
                                let m0: u32 = h.read(f(at::KEY));
                                x87::div(x87::load(m0), x87::load(m5))
                            }
                        };
                        h.write(scalar, x87::store(v));
                        Some(scalar)
                    }
                    0xc | 0xd => {
                        let x: u32 = h.read(f(at::VIEWPORT));
                        let y: u32 = h.read(f(at::VIEWPORT + 4));
                        if kind == 0xc {
                            h.write(vector, copy(x));
                            h.write(vector + 4, copy(y));
                        } else {
                            h.write(vector, x87::store(x87::div(1.0, x87::load(x))));
                            h.write(vector + 4, x87::store(x87::div(1.0, x87::load(y))));
                        }
                        Some(vector)
                    }
                    0xe => {
                        h.write(vector, width);
                        h.write(vector + 4, height);
                        Some(vector)
                    }
                    0xf => {
                        h.write(vector, x87::store(x87::div(1.0, x87::load(width))));
                        h.write(vector + 4, x87::store(x87::div(1.0, x87::load(height))));
                        Some(vector)
                    }
                    0x12 => {
                        let texture: u32 = h.read(camera + 0xbc);
                        h.write(texture_record + 0xc, texture);
                        let id = if texture == 0 {
                            0
                        } else {
                            h.read::<u32>(texture + 8)
                        };
                        let sampler: u32 = h.read(camera + 0xc0);
                        h.write(texture_record + 4, id);
                        h.write(texture_record + 8, sampler);
                        Some(texture_record)
                    }
                    0x13 => {
                        let cursor: u32 = h.read(constants + 4);
                        h.write(slot_record + 0xc, constants);
                        h.write(slot_record + 4, cursor);
                        Some(slot_record)
                    }
                    0x14 | 0x15 => {
                        let off = if kind == 0x14 { 0xc8 } else { 0xcc };
                        let state: u32 = h.read(camera + off);
                        if state != 0 && h.read::<u8>(state + 8) & 8 != 0 {
                            h.write(sampler_record + 0xc, state);
                            let v: u32 = h.read(state + 0xc);
                            h.write(sampler_record + 4, v);
                        } else {
                            h.write(sampler_record + 0xc, 0u32);
                        }
                        Some(sampler_record)
                    }
                    0x10 | 0x11 | 0x16..=0x1c => {
                        let off = match kind {
                            0x10 => 0,
                            0x11 => 0xd0,
                            0x16 => 0x10,
                            0x17 => 0x20,
                            0x18 => 0x24,
                            0x19 => 0x28,
                            0x1a => 0x2c,
                            0x1b => 0x30,
                            _ => 0x40,
                        };
                        let p = camera.wrapping_add(off);
                        (p != 0).then_some(p)
                    }
                    _ => None,
                };
                if let Some(source) = source {
                    let size = u32::from(h.read::<u16>(entry + 0xa) & 0x1fff);
                    let at = h.read::<u16>(entry + 8);
                    let room: u32 = h.read(buffer);
                    if size + u32::from(at) <= room && at >= 4 {
                        let mut bytes = vec![0u8; size as usize];
                        h.read_bytes(source, &mut bytes);
                        h.write_bytes(buffer.wrapping_add(u32::from(at)), &bytes);
                        near = h.read(near_at);
                        far = h.read(far_at);
                    }
                }
                offset = offset.wrapping_add(0x10);
                count -= 1;
                if count == 0 {
                    break;
                }
            }
        }
        h.call_sym(MAPPING_DTOR, Conv::Thiscall(mapping), &[]);
        h.call_sym(UNLOCK, Conv::Thiscall(lock), &[]);
        buffer
    }
}
