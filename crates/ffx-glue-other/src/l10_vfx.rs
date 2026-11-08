//! List `vfx`, round 10 (`tail10`): `opd_heap_display` (`0x00a63ff0`,
//! cdecl `(used, size, mode)`), the effects' debug heap meter.
//!
//! `mode & 3` picks a screen corner: the bar's four corner vectors
//! (`0x022c96e0..0x022c971c`), its position (`0x022c96d0`) and rotation
//! (`0x022c9750`), and two colour ramps from the table at `0x00c87c90` /
//! `0x00c87d10` (the second half unless `mode & 0x800`). The corner's
//! record (`0x022c9760 + 0x40 * corner`) keeps the peak and a falling
//! level of `used / size` (`+0x14`, `+0x10`; the level drops by 0.02 a
//! frame, or follows a higher ratio). 48 quads go into the PS2 packet at
//! `0x023115a0` (`0x90` bytes each: four transformed corners through
//! `FFXVu0RotTransPers`, coloured by the ramp below `48 * level`, else
//! grey), each step moving the bar; the packet's tags and its closing
//! record, then `sceGsSyncPath`, `FlushCache`, `sceDmaSend(0x1000a000,
//! packet)`. With the debug numbers on (`0x02321542`) the peak in percent
//! and the size go to `op_num` at the corner's place, by `mode`.
//! x87 arithmetic in the original's order (`ffx_bridge::x87`).

use ffx_graphic::host::{Conv, Host};

/// `FFXVu0CameraMatrix(out, eye, at, up)`.
pub const CAMERA_MATRIX: u32 = 0x0062_fc80;
/// `FFXSamp0ViewScreenMatrix(out, 9 floats)`.
pub const VIEW_SCREEN_MATRIX: u32 = 0x0093_d960;
/// `op_unit_m_scale(m, s)`.
pub const UNIT_M_SCALE: u32 = 0x007e_a700;
/// `op_rot_m(out, m, rot)`.
pub const ROT_M: u32 = 0x007e_9530;
/// `op_apply_m_w1(out, m, v)`.
pub const APPLY_M_W1: u32 = 0x007e_79a0;
/// `op_mul_m(out, a, b)`.
pub const MUL_M: u32 = 0x007e_83b0;
/// `FFXVu0RotTransPers(out, m, v, flag)`.
pub const ROT_TRANS_PERS: u32 = 0x006e_d840;
/// `sceGsSyncPath(mode, timeout)`.
pub const GS_SYNC_PATH: u32 = 0x0062_f880;
/// `FlushCache(mode)`.
pub const FLUSH_CACHE: u32 = 0x0062_f4c0;
/// `sceDmaSend(channel, packet)`.
pub const DMA_SEND: u32 = 0x0062_f710;
/// `op_num_xy_set(x, y)`.
pub const NUM_XY_SET: u32 = 0x007e_65e0;
/// `op_num(format, ...)` (cdecl, varargs).
pub const NUM: u32 = 0x007e_63a0;

/// The PS2 GIF channel `sceDmaSend` gets (a PS2 address, not the image's).
pub const GIF_CHANNEL: u32 = 0x1000_a000;
/// The debug numbers' switch (byte).
pub const SHOW_NUMBERS: u32 = 0x0232_1542;
/// The meter's state block (`0x022c9350..0x022c97a0`).
pub const STATE: u32 = 0x022c_9350;
/// The packet's head (`0x02311540`).
pub const PACKET: u32 = 0x0231_1540;
/// The first quad (`0x023115a0`).
pub const QUADS: u32 = 0x0231_15a0;
/// The camera vectors handed to `FFXVu0CameraMatrix`.
pub const EYE: u32 = 0x00c8_7c60;
/// The colour ramps' table (first ramp; the steps at `+0x80`).
pub const RAMPS: u32 = 0x00c8_7c90;
/// The flag picking the view's aspect (byte).
pub const WIDE: u32 = 0x011a_0064;
/// The packet's closing record's eight words.
pub const CLOSING: u32 = 0x011a_0770;

/// The float constants (`.rdata`), read from the image.
mod k {
    pub const MINUS_8: u32 = 0x00b5_8258;
    pub const MINUS_2: u32 = 0x00b4_1da4;
    pub const TWO: u32 = 0x00b9_2268;
    pub const EIGHT: u32 = 0x00b9_2314;
    pub const X_RIGHT: u32 = 0x00b5_9830;
    pub const X_LEFT: u32 = 0x00b7_bf6c;
    pub const ROT_RIGHT: u32 = 0x00b7_bf68;
    pub const ROT_LEFT: u32 = 0x00b7_bf70;
    pub const FAR: u32 = 0x00b4_3ddc;
    pub const DEPTH: u32 = 0x00b5_1a24;
    pub const SCREEN: u32 = 0x00b5_1a20;
    pub const ASPECT_WIDE: u32 = 0x00b5_82fc;
    pub const ASPECT: u32 = 0x00b6_70c4;
    pub const NEAR: u32 = 0x00b3_4254;
    pub const DIST: u32 = 0x00b4_1d9c;
    pub const HALF: u32 = 0x00b9_2194;
    // doubles
    pub const DROP: u32 = 0x00b4_a168;
    pub const QUADS: u32 = 0x00b5_f1e0;
    pub const PERCENT: u32 = 0x00b9_22e0;
    pub const STEP: u32 = 0x00b3_eb68;
    pub const HALF_D: u32 = 0x00b9_2220;
    pub const TWO_D: u32 = 0x00b9_2270;
}

/// The format strings.
mod fmt {
    pub const ET: u32 = 0x00b7_bf3c;
    pub const FPRIM: u32 = 0x00b7_bf60;
    pub const DEC: u32 = 0x00b5_6864;
    pub const HEX: u32 = 0x00b7_bf5c;
    pub const PAR: u32 = 0x00b7_bf44;
    pub const PRIM: u32 = 0x00b7_bf4c;
    pub const FPAR: u32 = 0x00b7_bf54;
}

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // h, g, q as in the module's other bridges
pub mod bridge {
    use super::{
        APPLY_M_W1, CAMERA_MATRIX, CLOSING, Conv, DMA_SEND, EYE, FLUSH_CACHE, GIF_CHANNEL,
        GS_SYNC_PATH, Host, MUL_M, NUM, NUM_XY_SET, PACKET, QUADS, RAMPS, ROT_M, ROT_TRANS_PERS,
        SHOW_NUMBERS, STATE, UNIT_M_SCALE, VIEW_SCREEN_MATRIX, WIDE, fmt, k,
    };
    use ffx_bridge::x87;

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// An image word.
    fn word(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    /// An image double.
    fn double(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    /// The state block's word at `0x022c9350 + off` (its address).
    fn st(h: &impl Host, off: u32) -> u32 {
        h.rebase(STATE).wrapping_add(off)
    }

    /// `fld m32` of a state word.
    fn ld(h: &impl Host, off: u32) -> f64 {
        x87::load(h.read(st(h, off)))
    }

    /// `fstp m32` into a state word.
    fn put(h: &mut impl Host, off: u32, x: f64) {
        let at = st(h, off);
        h.write(at, x87::store(x));
    }

    /// `fild` then `fstp m32`.
    fn int_f(i: u32) -> f64 {
        f64::from(f32::from_bits(x87::store(f64::from(i.cast_signed()))))
    }

    /// `opd_heap_display(used, size, mode)`; returns the `eax` the original
    /// leaves (the last call's).
    #[allow(clippy::too_many_lines)] // one original
    pub fn heap_display(h: &mut impl Host, used: u32, size: u32, mode: u32) -> u32 {
        let q = mode & 3;
        let slot = 0x410 + 0x40 * q; // 0x022c9760 + 0x40 q
        let low = q < 2;
        let m8 = word(h, k::MINUS_8);
        let m2 = word(h, k::MINUS_2);
        let two = word(h, k::TWO);
        let eight = word(h, k::EIGHT);
        let one = 1.0f32.to_bits();
        // the bar's corners
        let (a, b) = if low { (0, eight) } else { (m8, 0) };
        for (off, v) in [
            (0x390, a),
            (0x394, m2),
            (0x398, 0),
            (0x39c, one),
            (0x3a0, b),
            (0x3a4, m2),
            (0x3a8, 0),
            (0x3ac, one),
            (0x3b0, a),
            (0x3b4, two),
            (0x3b8, 0),
            (0x3bc, one),
            (0x3c0, b),
            (0x3c4, two),
            (0x3c8, 0),
            (0x3cc, one),
        ] {
            let at = st(h, off);
            h.write(at, v);
        }
        // the position and rotation
        let x = word(h, if low { k::X_RIGHT } else { k::X_LEFT });
        let y = if q & 1 == 0 { m2 } else { two };
        let rot = word(h, if low { k::ROT_RIGHT } else { k::ROT_LEFT });
        for (off, v) in [
            (0x380, x),
            (0x384, y),
            (0x388, 0),
            (0x38c, one),
            (0x400, 0),
            (0x404, rot),
            (0x408, 0),
            (0x40c, one),
        ] {
            let at = st(h, off);
            h.write(at, v);
        }
        // the colour ramps
        let wide = mode & 0x800 != 0;
        let row = 4 * q + if wide { 0 } else { 0x10 };
        let ramps = h.rebase(RAMPS);
        for i in 0..4 {
            let c: u32 = h.read(ramps.wrapping_add(4 * (row + i)));
            let s: u32 = h.read(ramps.wrapping_add(0x80 + 4 * (row + i)));
            let (ca, sa) = (st(h, 0x3d0 + 4 * i), st(h, 0x3e0 + 4 * i));
            h.write(ca, c);
            h.write(sa, s);
        }
        let grey = [
            0x80,
            if wide { 0x80 } else { 0 },
            if wide { 0 } else { 0x80 },
            0x20,
        ];
        for (i, v) in (0u32..).zip(grey) {
            let at = st(h, 0x3f0 + 4 * i);
            h.write(at, v);
        }

        // the level: the peak and a falling meter
        let sz = int_f(size);
        let level = if sz == 0.0 {
            0.0
        } else {
            let ratio = x87::load(x87::store(x87::div(int_f(used), sz)));
            if ld(h, slot + 0x14) < ratio {
                put(h, slot + 0x14, ratio);
            }
            let avg = ld(h, slot + 0x10);
            let v = if avg >= ratio {
                x87::load(x87::store(x87::sub(avg, double(h, k::DROP))))
            } else {
                ratio
            };
            put(h, slot + 0x10, v);
            v
        };
        let lit = x87::ftol(h, x87::mul(level, double(h, k::QUADS))).cast_signed();

        // the matrices
        let (eye, view) = (h.rebase(EYE), st(h, 0x80));
        cdecl(h, CAMERA_MATRIX, &[view, eye, eye + 0x10, eye + 0x20]);
        let aspect = if h.read::<u8>(h.rebase(WIDE)) == 0 {
            k::ASPECT
        } else {
            k::ASPECT_WIDE
        };
        let screen = word(h, k::SCREEN);
        let proj = [
            st(h, 0xc0),
            word(h, k::DIST),
            word(h, k::NEAR),
            word(h, aspect),
            screen,
            screen,
            one,
            word(h, k::DEPTH),
            one,
            word(h, k::FAR),
        ];
        cdecl(h, VIEW_SCREEN_MATRIX, &proj);

        // the packet's tags
        let packet = h.rebase(PACKET);
        for (off, v) in [
            (0x10, 1_u32),
            (0x18, 0xeeee_eeee),
            (0x20, 0x44),
            (0x28, 0x42),
            (0x30, 0x0003_000d),
            (0x38, 0x47),
            (0x40, 1),
            (0x48, 0x14),
        ] {
            h.write(packet.wrapping_add(off), v);
        }
        let half = word(h, k::HALF);
        let step = double(h, k::STEP);
        let quads = h.rebase(QUADS);
        let mut quad = quads;
        let (m, w, mv, corners) = (st(h, 0), st(h, 0x30), st(h, 0x40), st(h, 0x390));
        for i in 0..48i32 {
            cdecl(h, UNIT_M_SCALE, &[m, half]);
            cdecl(h, ROT_M, &[m, m, st(h, 0x400)]);
            cdecl(h, APPLY_M_W1, &[w, view, st(h, 0x380)]);
            cdecl(h, MUL_M, &[mv, st(h, 0xc0), m]);
            for (c, last) in [(0u32, 0x8000u32), (1, 0x8000), (2, 0), (3, 0)] {
                let out = quad.wrapping_add(0x20 * (c + 1));
                cdecl(h, ROT_TRANS_PERS, &[out, mv, corners + 0x10 * c, 0]);
                h.write(out.wrapping_add(0xc), last);
            }
            h.write(quad.wrapping_add(0x8c), 0_u32);
            h.write(quad.wrapping_add(8), 0x42_u32);
            let ramp: [u32; 4] = std::array::from_fn(|j| h.read(st(h, 0x3d0 + 4 * j as u32)));
            let colour = if i < lit {
                h.write(quad, 0x48_u32);
                ramp
            } else {
                h.write(quad, 0x42_u32);
                std::array::from_fn(|j| h.read(st(h, 0x3f0 + 4 * j as u32)))
            };
            for c in 0..4u32 {
                for (j, v) in (0u32..).zip(colour) {
                    h.write(quad.wrapping_add(0x10 + 0x20 * c + 4 * j), v);
                }
            }
            for j in 0..4u32 {
                let d: u32 = h.read(st(h, 0x3e0 + 4 * j));
                let at = st(h, 0x3d0 + 4 * j);
                h.write(at, ramp[j as usize].wrapping_add(d));
            }
            let n: u32 = h.read(st(h, 0x3fc));
            let at = st(h, 0x3fc);
            h.write(at, n.wrapping_add(1));
            // the bar moves
            let half_d = double(h, k::HALF_D);
            let two_d = double(h, k::TWO_D);
            let y = ld(h, 0x384);
            put(
                h,
                0x384,
                if q & 1 == 0 {
                    x87::sub(y, half_d)
                } else {
                    x87::add(y, half_d)
                },
            );
            if low {
                let v = x87::add(ld(h, 0x3a0), step);
                put(h, 0x3a0, v);
                let v = x87::add(step, ld(h, 0x3c0));
                put(h, 0x3c0, v);
                let v = x87::sub(ld(h, 0x404), two_d);
                put(h, 0x404, v);
            } else {
                let v = x87::sub(ld(h, 0x390), step);
                put(h, 0x390, v);
                let v = x87::sub(ld(h, 0x3b0), step);
                put(h, 0x3b0, v);
                let v = x87::add(ld(h, 0x404), two_d);
                put(h, 0x404, v);
            }
            quad = quad.wrapping_add(0x90);
        }

        // the packet's closing record and head
        let span = quad.wrapping_sub(quads).cast_signed();
        let count = (span + ((span >> 31) & 0xf)) >> 4;
        h.write(packet.wrapping_add(0x58), 0x1414_141e_u32);
        h.write(packet.wrapping_add(0x50), count / 9);
        let closing = h.rebase(CLOSING);
        for j in 0..8u32 {
            let v: u32 = h.read(closing.wrapping_add(4 * j));
            h.write(quad.wrapping_add(0x10 + 4 * j), v);
        }
        h.write(quad, 0x8001_u32);
        h.write(quad.wrapping_add(8), 0xeeee_eeee_u32);
        let len = quad.wrapping_sub(packet).wrapping_add(0x30).cast_signed();
        let qw = ((len + ((len >> 31) & 0xf)) >> 4)
            .wrapping_sub(1)
            .cast_unsigned();
        h.write(packet, qw | 0x7000_0000);
        h.write(packet.wrapping_add(8), 0_u32);
        cdecl(h, GS_SYNC_PATH, &[0, 0]);
        cdecl(h, FLUSH_CACHE, &[0]);
        let eax = cdecl(h, DMA_SEND, &[GIF_CHANNEL, packet]);
        if h.read::<u8>(h.rebase(SHOW_NUMBERS)) == 0 {
            return eax;
        }

        // the numbers
        let pct = x87::ftol(h, x87::mul(ld(h, slot + 0x14), double(h, k::PERCENT)));
        let lines: &[(u32, u32, u32, Option<u32>)] = match mode {
            0x800 => &[(2, 1, fmt::ET, Some(pct))],
            0x601 => &[
                (2, 0x22, fmt::FPRIM, None),
                (2, 0x23, fmt::DEC, Some(pct)),
                (2, 0x24, fmt::HEX, Some(size)),
            ],
            0x801 => &[(2, 0x25, fmt::PAR, Some(pct))],
            0x802 => &[(0x38, 1, fmt::PRIM, Some(pct))],
            0x8603 => &[
                (0x3a, 0x22, fmt::FPAR, None),
                (0x3b, 0x23, fmt::DEC, Some(pct)),
                (0x38, 0x24, fmt::HEX, Some(size.wrapping_add(0x10))),
            ],
            _ => match q {
                0 => &[(2, 1, fmt::HEX, Some(size)), (2, 2, fmt::DEC, Some(pct))],
                1 => &[
                    (2, 0x23, fmt::DEC, Some(pct)),
                    (2, 0x24, fmt::HEX, Some(size)),
                ],
                2 => &[
                    (0x38, 1, fmt::HEX, Some(size)),
                    (0x3b, 2, fmt::DEC, Some(pct)),
                ],
                _ => &[
                    (0x3b, 0x23, fmt::DEC, Some(pct)),
                    (0x38, 0x24, fmt::HEX, Some(size.wrapping_add(0x10))),
                ],
            },
        };
        let mut eax = 0;
        for &(x, y, f, v) in lines {
            cdecl(h, NUM_XY_SET, &[x, y]);
            let f = h.rebase(f);
            eax = match v {
                Some(v) => cdecl(h, NUM, &[f, v]),
                None => cdecl(h, NUM, &[f]),
            };
        }
        eax
    }
}
