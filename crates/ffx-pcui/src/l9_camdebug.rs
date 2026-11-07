//! M5 stage 1, round 9 (`tail9`, list `menu`): `CamDebug_mouse_00940660`,
//! the mouse handler of the SPLINE EDIT camera debug window
//! (`Gui_SplineEditProc_0093ee10`): picking a spline point under the
//! cursor (its screen position through `DebugView_project_0093fd60`),
//! dragging it (`CamDebug_ScreenPick_00940f30`, the whole spline with it
//! in mode 1), raising the camera, turning it (`Cam_SetPolar`) and moving
//! it (`MsCameraSetRect`, along the view direction once the camera is
//! more than 10 units from its target).
//!
//! x87 code as the formulas at 53 bits, each value rounded to `f32`
//! where the original stores it.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // the original's registers
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// The callees (cdecl): `Gu_MouseGetX/Y()`, `MsCameraGetPosNow(side,
    /// out)`, `DebugView_project_0093fd60(v, out)`, `MsGetCameraMatrix(v,
    /// d)`, `CamDebug_ScreenPick_00940f30(px, py, cam, point, offset)`,
    /// `Sg_NameListClear_00942320()`, `MsCameraSetRect(cam, side, xyz)`,
    /// `MsCameraGetAngle(a, b, c)`, `Cam_SetPolar(cam, side, theta, phi,
    /// len)`; `_CIsqrt` (`0x009497b8`, `ST0`).
    pub const MOUSE_X: u32 = 0x0084_9ae0;
    pub const MOUSE_Y: u32 = 0x0084_9af0;
    pub const GET_POS: u32 = 0x007b_d470;
    pub const PROJECT: u32 = 0x0093_fd60;
    pub const CAMERA_MATRIX: u32 = 0x007c_04b0;
    pub const SCREEN_PICK: u32 = 0x0094_0f30;
    pub const NAME_LIST_CLEAR: u32 = 0x0094_2320;
    pub const SET_RECT: u32 = 0x007b_f940;
    pub const GET_ANGLE: u32 = 0x007b_d2d0;
    pub const SET_POLAR: u32 = 0x007b_f700;
    pub const SQRT: u32 = 0x0094_97b8;
    /// The pad bits (`0x023cbc10`) and the window's state: the camera's two
    /// positions now (`0x025d5ef4`, `0x025d5ee4`) and saved (`0x025d5f14`,
    /// `0x025d5f04`), the camera handle (`0x025d5ee0`), the turn, raise and
    /// move flags (bytes `0x025d5f29`, `0x025d5f2b`, `0x025d5f2a`), the
    /// drag's start (i16 `0x025d5f34`, `0x025d5f36`), the dragged point's
    /// start (`0x025d5f38`), the angles (`0x025d5f80`), the drag mode
    /// (`0x025d5f8c`), the last pick (`0x025d5f7c`), the current spline
    /// (`0x025d5f90`) and per spline (words, by index): flags (12-byte
    /// entries) `0x025d5f44`, counts `0x025d5f4c`, points (16 bytes)
    /// `0x025d5f54`, their copies `0x025d5f5c`, the picked entry
    /// `0x025d5f64`, point `0x025d5f6c` and index `0x025d5f74`.
    pub const PAD: u32 = 0x023c_bc10;
    pub const POS0: u32 = 0x025d_5ef4;
    pub const POS1: u32 = 0x025d_5ee4;
    pub const SAVE0: u32 = 0x025d_5f14;
    pub const SAVE1: u32 = 0x025d_5f04;
    pub const CAMERA: u32 = 0x025d_5ee0;
    pub const TURN: u32 = 0x025d_5f29;
    pub const MOVE: u32 = 0x025d_5f2a;
    pub const RAISE: u32 = 0x025d_5f2b;
    pub const START_X: u32 = 0x025d_5f34;
    pub const START_Y: u32 = 0x025d_5f36;
    pub const ANCHOR: u32 = 0x025d_5f38;
    pub const ANGLES: u32 = 0x025d_5f80;
    pub const DRAG_MODE: u32 = 0x025d_5f8c;
    pub const LAST_PICK: u32 = 0x025d_5f7c;
    pub const SPLINE: u32 = 0x025d_5f90;
    pub const FLAGS: u32 = 0x025d_5f44;
    pub const COUNTS: u32 = 0x025d_5f4c;
    pub const POINTS: u32 = 0x025d_5f54;
    pub const COPIES: u32 = 0x025d_5f5c;
    pub const PICKED: u32 = 0x025d_5f64;
    pub const PICKED_POINT: u32 = 0x025d_5f6c;
    pub const PICKED_INDEX: u32 = 0x025d_5f74;
    /// The doubles: the raise divisor (100, `0x00b922e0`), the turn
    /// divisor (400, `0x00b691c8`), pi (`0x00b92288`), the move divisors
    /// (10, `0x00b922b0`; 400000, `0x00b691d0`) and the distance limit
    /// (10.0f, `0x00b92318`).
    pub const RAISE_DIV: u32 = 0x00b9_22e0;
    pub const TURN_DIV: u32 = 0x00b6_91c8;
    pub const PI: u32 = 0x00b9_2288;
    pub const MOVE_DIV: u32 = 0x00b9_22b0;
    pub const MOVE_DIV2: u32 = 0x00b6_91d0;
    pub const NEAR: u32 = 0x00b9_2318;

    /// The original's frame below `ebp` the callees see: the point
    /// (`ebp - 0x14`, 4 floats), the projection (`ebp - 0x24`, 2 words)
    /// and the offset (`ebp - 0x48`, 3 floats).
    pub const FRAME: u32 = 0x48;

    fn g(h: &impl Host, sym: u32) -> u32 {
        h.read(h.rebase(sym))
    }

    fn gf(h: &impl Host, sym: u32) -> f64 {
        x87::load(g(h, sym))
    }

    fn set(h: &mut impl Host, sym: u32, v: u32) {
        let at = h.rebase(sym);
        h.write(at, v);
    }

    fn set_u8(h: &mut impl Host, sym: u32, v: u8) {
        let at = h.rebase(sym);
        h.write(at, v);
    }

    fn get_u8(h: &impl Host, sym: u32) -> u8 {
        h.read(h.rebase(sym))
    }

    fn k(h: &impl Host, sym: u32) -> f64 {
        f64::from_bits(h.read(h.rebase(sym)))
    }

    /// A word of the current spline's per-spline table `table`.
    fn per(h: &impl Host, table: u32) -> u32 {
        let i = g(h, SPLINE);
        h.read(h.rebase(table).wrapping_add(i.wrapping_mul(4)))
    }

    fn set_per(h: &mut impl Host, table: u32, v: u32) {
        let i = g(&*h, SPLINE);
        let at = h.rebase(table).wrapping_add(i.wrapping_mul(4));
        h.write(at, v);
    }

    fn sub(h: &impl Host, a: u32, b: u32) -> u32 {
        x87::store(x87::sub(gf(h, a), gf(h, b)))
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `|a - b| < 8` as the original's `cdq; xor; sub` absolute value (of
    /// the wrapped difference).
    fn near8(a: u32, b: u32) -> bool {
        let d = a.wrapping_sub(b).cast_signed();
        let abs = if d < 0 { d.wrapping_neg() } else { d };
        abs < 8
    }

    fn set_rects(h: &mut impl Host) {
        let cam = g(&*h, CAMERA);
        let (p0, p1) = (h.rebase(POS0), h.rebase(POS1));
        cdecl(h, SET_RECT, &[cam, 0, p0]);
        cdecl(h, SET_RECT, &[cam, 1, p1]);
    }

    /// A 16-bit start coordinate (`mov word`).
    fn set_start(h: &mut impl Host, sym: u32, v: u32) {
        let [lo, hi, ..] = v.to_le_bytes();
        let at = h.rebase(sym);
        h.write(at, u16::from_le_bytes([lo, hi]));
    }

    /// The drag's offset from its start (`i16` start sign-extended).
    fn from_start(h: &impl Host, sym: u32, v: u32) -> u32 {
        v.wrapping_sub(i32::from(h.read::<i16>(h.rebase(sym))).cast_unsigned())
    }

    /// `fild dword; fstp dword`.
    fn int_float(v: u32) -> f64 {
        x87::load(x87::store(f64::from(v.cast_signed())))
    }

    /// The point picked under the cursor (`pad` bit 2, not raising): the
    /// spline's flagged points projected; the first within 8 pixels picked
    /// (a release flags it 2 and ends the step: `true`); a pick keeps the
    /// point as the drag's anchor and the spline's points as copies.
    fn pick(h: &mut impl Host, ebp: u32, mx: u32, my: u32, release: bool) -> bool {
        let fr = |o: u32| ebp.wrapping_sub(o);
        set_per(h, PICKED, 0);
        let n = per(&*h, COUNTS).cast_signed();
        let mut entry = per(&*h, FLAGS);
        let mut point = per(&*h, POINTS);
        let mut i = 0_i32;
        while i < n {
            if h.read::<u32>(entry) != 0 {
                let v = fr(0x14);
                for j in 0..3 {
                    let x = x87::store(x87::load(h.read(point.wrapping_add(4 * j))));
                    h.write(v.wrapping_add(4 * j), x);
                }
                h.write(v.wrapping_add(0xc), x87::store(1.0));
                let out = fr(0x24);
                cdecl(h, PROJECT, &[v, out]);
                let (px, py): (u32, u32) = (h.read(out), h.read(out.wrapping_add(4)));
                if near8(mx, px) && near8(my, py) {
                    set_per(h, PICKED, entry);
                    set_per(h, PICKED_POINT, point);
                    set_per(h, PICKED_INDEX, i.cast_unsigned());
                    if release {
                        let f: u32 = h.read(entry);
                        h.write(entry, f | 2);
                        set(h, LAST_PICK, i.cast_unsigned());
                        return true;
                    }
                    break;
                }
            }
            i += 1;
            point = point.wrapping_add(0x10);
            entry = entry.wrapping_add(0xc);
        }
        if per(&*h, PICKED) != 0 {
            set_u8(h, TURN, 1);
            let anchor = h.rebase(ANCHOR);
            for j in 0..3 {
                let pp = per(&*h, PICKED_POINT);
                let x = x87::store(x87::load(h.read(pp.wrapping_add(4 * j))));
                h.write(anchor.wrapping_add(4 * j), x);
            }
            let mut k = 0_u32;
            while k.cast_signed() < per(&*h, COUNTS).cast_signed() {
                for j in 0..4 {
                    let o = k.wrapping_mul(0x10).wrapping_add(4 * j);
                    let x = x87::store(x87::load(h.read(per(&*h, POINTS).wrapping_add(o))));
                    let dst = per(&*h, COPIES).wrapping_add(o);
                    h.write(dst, x);
                }
                k += 1;
            }
        }
        false
    }

    /// Dragging the picked point to the cursor (`CamDebug_ScreenPick`
    /// from the anchor along the camera offset); in mode 1 the whole spline
    /// moved by the point's move (from the copies). Returns the offset
    /// (the camera's, or the move in mode 1).
    fn drag(h: &mut impl Host, ebp: u32, mx: u32, my: u32, d: [u32; 3]) -> [u32; 3] {
        let fr = |o: u32| ebp.wrapping_sub(o);
        let v = fr(0x14);
        let anchor = h.rebase(ANCHOR);
        for j in 0..3 {
            let x = x87::store(x87::load(h.read(anchor.wrapping_add(4 * j))));
            h.write(v.wrapping_add(4 * j), x);
        }
        h.write(v.wrapping_add(0xc), x87::store(1.0));
        let off = fr(0x48);
        for (j, x) in (0..3_u32).zip(d) {
            h.write(off.wrapping_add(4 * j), x87::store(x87::load(x)));
        }
        let m = cdecl(h, CAMERA_MATRIX, &[v, off]);
        cdecl(h, SCREEN_PICK, &[mx, my, m, v, off]);
        for j in 0..3 {
            let x = x87::store(x87::load(h.read(v.wrapping_add(4 * j))));
            let pp = per(&*h, PICKED_POINT);
            h.write(pp.wrapping_add(4 * j), x);
        }
        if g(&*h, DRAG_MODE) != 1 {
            return d;
        }
        let moved = [0, 1, 2].map(|j: u32| {
            x87::store(x87::sub(
                x87::load(h.read(v.wrapping_add(4 * j))),
                x87::load(h.read(anchor.wrapping_add(4 * j))),
            ))
        });
        let mut k = 0_u32;
        while k.cast_signed() < per(&*h, COUNTS).cast_signed() {
            let o = k.wrapping_mul(0x10);
            for j in 0..3 {
                let src = per(&*h, COPIES).wrapping_add(o).wrapping_add(4 * j);
                let x = x87::store(x87::add(
                    x87::load(h.read(src)),
                    x87::load(moved[j as usize]),
                ));
                let dst = per(&*h, POINTS).wrapping_add(o).wrapping_add(4 * j);
                h.write(dst, x);
            }
            let src = per(&*h, COPIES).wrapping_add(o).wrapping_add(0xc);
            let w = x87::store(x87::load(h.read(src)));
            let dst = per(&*h, POINTS).wrapping_add(o).wrapping_add(0xc);
            h.write(dst, w);
            k += 1;
        }
        moved
    }

    /// Raising the camera: both saved positions up by `|d| d / 100`
    /// (`d` the cursor's y from the drag's start).
    fn raise(h: &mut impl Host, my: u32) {
        let d = from_start(&*h, START_Y, my).cast_signed();
        let r = d.wrapping_abs().wrapping_mul(d).cast_unsigned();
        let s = x87::load(x87::store(x87::div(int_float(r), k(&*h, RAISE_DIV))));
        for (src, dst) in [(SAVE0, POS0), (SAVE1, POS1)] {
            let x = x87::store(gf(&*h, src));
            set(h, dst, x);
            let y = x87::store(x87::sub(gf(&*h, src + 4), s));
            set(h, dst + 4, y);
            let z = x87::store(gf(&*h, src + 8));
            set(h, dst + 8, z);
        }
        set_rects(h);
    }

    /// Turning the camera: `Cam_SetPolar` from the kept angles by the
    /// cursor's move / 400.
    fn turn(h: &mut impl Host, mx: u32, my: u32) {
        let len = x87::store(gf(&*h, ANGLES + 8));
        let div = k(&*h, TURN_DIV);
        let dy = int_float(from_start(&*h, START_Y, my));
        let phi = x87::store(x87::add(x87::div(dy, div), gf(&*h, ANGLES + 4)));
        let dx = int_float(from_start(&*h, START_X, mx));
        let theta = x87::store(x87::sub(gf(&*h, ANGLES), x87::div(dx, div)));
        let cam = g(&*h, CAMERA);
        cdecl(h, SET_POLAR, &[cam, 1, theta, phi, len]);
    }

    /// Moving the camera: both positions from the saved ones by the
    /// cursor's move: along the camera's direction (squared / 400000)
    /// when it is more than 10 from its target, else by the move / 10.
    fn shift(h: &mut impl Host, mx: u32, my: u32) {
        let ex = sub(&*h, SAVE0, SAVE1);
        let ez = sub(&*h, SAVE0 + 8, SAVE1 + 8);
        let ddx = from_start(&*h, START_X, mx);
        let ddy = from_start(&*h, START_Y, my);
        let (ex, ez) = (x87::load(ex), x87::load(ez));
        let len = h.call_intrinsic(h.rebase(SQRT), x87::add(x87::mul(ez, ez), x87::mul(ex, ex)));
        let near = gf(&*h, NEAR);
        let (ox, oz) = if f64::from(len) <= near {
            let div = k(&*h, MOVE_DIV);
            let ddxf = int_float(ddx);
            let ox = x87::store(x87::div(-ddxf, div));
            let oz = x87::store(x87::div(int_float(ddy), div));
            (ox, oz)
        } else {
            let (fy, fx) = (int_float(ddy), int_float(ddx));
            let a = x87::load(x87::store(x87::mul(fy, ex)));
            let b = x87::load(x87::store(x87::sub(a, x87::mul(fx, ez))));
            let c = x87::load(x87::store(x87::mul(fy, ez)));
            let d = x87::load(x87::store(x87::add(c, x87::mul(fx, ex))));
            let div = k(&*h, MOVE_DIV2);
            let ox = x87::store(x87::div(x87::mul(b.abs(), b), div));
            let oz = x87::store(x87::div(x87::mul(d.abs(), d), div));
            (ox, oz)
        };
        let (ox, oz) = (x87::load(ox), x87::load(oz));
        for (src, dst) in [(SAVE0, POS0), (SAVE1, POS1)] {
            let x = x87::store(x87::add(gf(&*h, src), ox));
            set(h, dst, x);
            let y = x87::store(gf(&*h, src + 4));
            set(h, dst + 4, y);
            let z = x87::store(x87::add(gf(&*h, src + 8), oz));
            set(h, dst + 8, z);
        }
        set_rects(h);
    }

    /// `CamDebug_mouse_00940660(flags)` (cdecl; `flags` bit 6: a release):
    /// the window's mouse step (module doc), `ebp` its frame ([`FRAME`]
    /// bytes below it).
    pub fn mouse(h: &mut impl Host, ebp: u32, flags: u32) {
        let release = flags & 0x40 != 0;
        let mx = cdecl(h, MOUSE_X, &[]);
        let my = cdecl(h, MOUSE_Y, &[]).wrapping_mul(2);
        let (p0, p1, s0, s1) = (
            h.rebase(POS0),
            h.rebase(POS1),
            h.rebase(SAVE0),
            h.rebase(SAVE1),
        );
        cdecl(h, GET_POS, &[0, p0]);
        cdecl(h, GET_POS, &[1, p1]);
        let mut d = [
            sub(&*h, POS0, POS1),
            sub(&*h, POS0 + 4, POS1 + 4),
            sub(&*h, POS0 + 8, POS1 + 8),
        ];
        let pad = g(&*h, PAD);
        if pad & 0x100 != 0 && pad & 1 != 0 && per(&*h, PICKED) == 0 {
            if get_u8(&*h, RAISE) == 0 {
                cdecl(h, GET_POS, &[0, s0]);
                cdecl(h, GET_POS, &[1, s1]);
                set_start(h, START_X, mx);
                set_start(h, START_Y, my);
            }
            // move off, raise on (one word), turn off
            let at = h.rebase(MOVE);
            h.write(at, 0x100_u16);
            set_u8(h, TURN, 0);
        }
        if g(&*h, PAD) & 2 != 0 && get_u8(&*h, RAISE) == 0 && pick(h, ebp, mx, my, release) {
            return;
        }
        if release {
            if g(&*h, PAD) & 0x100 != 0 {
                cdecl(h, NAME_LIST_CLEAR, &[]);
            }
            let at = h.rebase(TURN);
            h.write(at, 0_u16);
            set_u8(h, RAISE, 0);
            return;
        }
        if g(&*h, PAD) & 4 != 0 {
            set_u8(h, TURN, 0);
            set_u8(h, RAISE, 0);
            set_per(h, PICKED, 0);
            set_per(h, PICKED_POINT, 0);
        }
        if g(&*h, PAD) & 0x400 != 0 {
            // move and raise off (one word)
            let at = h.rebase(MOVE);
            h.write(at, 0_u16);
            set_per(h, PICKED, 0);
            set_per(h, PICKED_POINT, 0);
            set_per(h, PICKED_INDEX, 0);
        }
        let mut raise_check = true;
        if get_u8(&*h, TURN) != 0 && get_u8(&*h, RAISE) == 0 {
            if per(&*h, PICKED) == 0 {
                raise_check = false;
            } else {
                d = drag(h, ebp, mx, my, d);
            }
        }
        if raise_check && get_u8(&*h, RAISE) != 0 {
            raise(h, my);
        }
        if per(&*h, PICKED) == 0 && get_u8(&*h, RAISE) == 0 {
            if g(&*h, PAD) & 2 != 0 {
                let a = [ANGLES, ANGLES + 4, ANGLES + 8].map(|s| h.rebase(s));
                cdecl(h, GET_ANGLE, &a);
                // fucom: an unordered compare counts as unequal
                let ne = |x: u32| {
                    let v = x87::load(x);
                    v.is_nan() || v != 0.0
                };
                if ne(d[0]) || ne(d[2]) {
                    let a1 = x87::store(x87::add(gf(&*h, ANGLES + 4), k(&*h, PI)));
                    set(h, ANGLES + 4, a1);
                }
                set_start(h, START_X, mx);
                set_start(h, START_Y, my);
                set_u8(h, TURN, 1);
                turn(h, mx, my);
            } else if get_u8(&*h, TURN) != 0 {
                turn(h, mx, my);
            }
        }
        if per(&*h, PICKED) == 0 && get_u8(&*h, RAISE) == 0 {
            if g(&*h, PAD) & 0x200 != 0 {
                set_start(h, START_X, mx);
                set_start(h, START_Y, my);
                cdecl(h, GET_POS, &[0, s0]);
                cdecl(h, GET_POS, &[1, s1]);
                set_u8(h, MOVE, 1);
                shift(h, mx, my);
            } else if get_u8(&*h, MOVE) != 0 {
                shift(h, mx, my);
            }
        }
    }
}
