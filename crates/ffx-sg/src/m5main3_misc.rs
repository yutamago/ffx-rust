//! M5 `main`, round 3: small leftovers of the `main` list (the movie
//! viewer's camera, ...).

/// The originals over game memory.
#[allow(
    clippy::cast_possible_truncation,
    clippy::many_single_char_names,
    clippy::similar_names
)]
pub mod bridge {
    use ffx_bridge::{Mem, x87};
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    /// `FFXSamp0ViewScreenMatrix(m, scrz, ax, ay, cx, cy, zmin, zmax,
    /// nearz, farz)` (cdecl, ten words).
    pub const VIEW_SCREEN_MATRIX: u32 = 0x0093_d960;
    /// `_CItan`'s import thunk.
    pub const CI_TAN: u32 = 0x0094_97be;
    /// The constants the original loads: 4.0, 3.0, 3.1415 (an `f32` pi
    /// widened), 180.0, 0.5 (`f64`) and the default screen distance 1024.0
    /// (`f32`).
    pub const C_FOUR: u32 = 0x00b4_3d80;
    pub const C_THREE: u32 = 0x00b2_c5c8;
    pub const C_PI: u32 = 0x00b7_af40;
    pub const C_180: u32 = 0x00b4_8378;
    pub const C_HALF: u32 = 0x00b9_2220;
    pub const C_SCRZ: u32 = 0x00b5_5178;
    /// `g_Screens` (`0x01137000`): screen 0's centre and z range, as
    /// `FFXSamp0ViewScreenMatrix` takes them (`+0x20`, `+0x24`, `+0x00`,
    /// `+0x04`, `+0x08`, `+0x0c`).
    pub const SCREENS: u32 = 0x0113_7000;

    fn qword(h: &impl Mem, sym: u32) -> f64 {
        h.read(h.rebase(sym))
    }

    /// `fld dword [p]`.
    fn fld(h: &impl Mem, p: u32) -> f64 {
        x87::load(h.read(p))
    }

    /// `fild dword` then `fstp dword`: the integer rounded to `f32`.
    fn int_f32(v: u32) -> f64 {
        x87::load(x87::store(f64::from(v.cast_signed())))
    }

    /// `FUN_00a43e60(camera, width, height, view)` (cdecl): the movie
    /// viewer's projection and camera. The field of view (degrees, `f32`
    /// at `camera + 0x24`) gives `t = tan(fov * pi / 180 * 0.5)`
    /// (`_CItan`); the screen distance `d` at `view + 0x50510` is
    /// `width * 0.5 / t` when byte `+0x5076c` is set, else 1024;
    /// `FFXSamp0ViewScreenMatrix(view + 0x50420, d, s, s * (height * 4) /
    /// (width * 3), screen 0's centre and z range)` with `s = width / (t *
    /// 2d)`. Then three vectors from the camera's nine floats (the
    /// position, a second point, their difference), y and z negated, w
    /// 1.0, at `+0x504e0`. Every step in `f64` as the x87 at 53 bits,
    /// rounded where the original stores.
    pub fn movie_camera(h: &mut impl Host, camera: u32, width: u32, height: u32, view: u32) {
        let w = int_f32(width);
        let hh = int_f32(height);
        let aspect = x87::store(x87::div(
            x87::mul(hh, qword(h, C_FOUR)),
            x87::mul(w, qword(h, C_THREE)),
        ));
        let fov = fld(h, at(camera, 0x24));
        let rad = x87::store(x87::div(x87::mul(fov, qword(h, C_PI)), qword(h, C_180)));
        let arg = x87::mul(x87::load(rad), qword(h, C_HALF));
        let tan_at = h.rebase(CI_TAN);
        let t = x87::load(h.call_intrinsic(tan_at, arg).to_bits());
        let d_at = at(view, 0x5_0510);
        if h.read::<u8>(at(view, 0x5_076c)) == 0 {
            let c: u32 = h.read(h.rebase(C_SCRZ));
            h.write(d_at, x87::store(x87::load(c)));
        } else {
            let d = x87::div(x87::mul(w, qword(h, C_HALF)), t);
            h.write(d_at, x87::store(d));
        }
        let d = fld(h, d_at);
        let span = x87::store(x87::mul(t, x87::add(d, d)));
        let s = x87::store(x87::div(w, x87::load(span)));
        let sy = x87::store(x87::mul(x87::load(s), x87::load(aspect)));
        let scr = h.rebase(SCREENS);
        let args = [
            at(view, 0x5_0420),
            x87::store(fld(h, d_at)),
            s,
            sy,
            x87::store(fld(h, at(scr, 0x20))),
            x87::store(fld(h, at(scr, 0x24))),
            x87::store(fld(h, at(scr, 0))),
            x87::store(fld(h, at(scr, 4))),
            x87::store(fld(h, at(scr, 8))),
            x87::store(fld(h, at(scr, 0xc))),
        ];
        h.call_sym(VIEW_SCREEN_MATRIX, Conv::Cdecl, &args);
        let one = 1.0f32.to_bits();
        let out = |i: u32| at(view, 0x5_04e0 + 4 * i);
        let v = x87::store(fld(h, camera));
        h.write(out(0), v);
        let v = x87::store(-fld(h, at(camera, 0x4)));
        h.write(out(1), v);
        let v = x87::store(-fld(h, at(camera, 0x8)));
        h.write(out(2), v);
        h.write(out(3), one);
        let v = x87::store(fld(h, at(camera, 0xc)));
        h.write(out(4), v);
        let v = x87::store(-fld(h, at(camera, 0x10)));
        h.write(out(5), v);
        let v = x87::store(-fld(h, at(camera, 0x14)));
        h.write(out(6), v);
        h.write(out(7), one);
        let v = x87::store(x87::sub(fld(h, at(camera, 0x18)), fld(h, camera)));
        h.write(out(8), v);
        let v = x87::store(-x87::sub(fld(h, at(camera, 0x1c)), fld(h, at(camera, 0x4))));
        h.write(out(9), v);
        let v = x87::store(-x87::sub(fld(h, at(camera, 0x20)), fld(h, at(camera, 0x8))));
        h.write(out(10), v);
        h.write(out(11), one);
    }

    /// The debug light table (`0x01303c20`): six lights of 0x1c bytes
    /// (`i32` r, g, b at `+0..+8`, `+0xc` -1, `+0x18` its index as `u16`,
    /// `+0x1a` a byte set), and below it seven copies of the whole table
    /// from `0x01303780`.
    pub const LIGHTS: u32 = 0x0130_3c20;
    pub const LIGHT: u32 = 0x1c;
    pub const LIGHT_COPIES: u32 = 0x0130_3780;
    /// `Sg_LightGetColor(i, out)` (cdecl; writes four `f32` to `out`).
    pub const LIGHT_GET_COLOR: u32 = 0x0083_60f0;
    /// 255.0 (`f64`).
    pub const C_255: u32 = 0x00b1_70c0;

    /// `code_0084ea00()` (cdecl; not a function in the dump, called by the
    /// debug clock's set-up after `Sg_ClockFromHost`): the debug light
    /// table from the game's lights: lights 0..=3 get `Sg_LightGetColor`'s
    /// r, g, b times 255 (`_ftol2_sse`) and -1; every light its index and
    /// the byte `+0x1a` set; then the table is copied seven times below
    /// itself. `buf` is the original's 16-byte frame buffer (`ebp -
    /// 0x20`) the colour goes through. Returns the end of the copies (in
    /// `eax`).
    pub fn light_table_init(h: &mut impl Host, buf: u32) -> u32 {
        let table = h.rebase(LIGHTS);
        for i in 0..6u32 {
            let e = at(table, i * LIGHT);
            h.write(at(e, 0x18), i as u16);
            if i <= 3 {
                h.call_sym(LIGHT_GET_COLOR, Conv::Cdecl, &[i, buf]);
                let k = qword(h, C_255);
                let r = x87::mul(fld(h, buf), k);
                let r = x87::ftol(h, r);
                let g = x87::ftol(h, x87::mul(fld(h, at(buf, 4)), k));
                h.write(e, r);
                let b = x87::ftol(h, x87::mul(k, fld(h, at(buf, 8))));
                h.write(at(e, 4), g);
                h.write(at(e, 8), b);
                h.write(at(e, 0xc), u32::MAX);
            }
            h.write(at(e, 0x1a), 1u8);
        }
        let copies = h.rebase(LIGHT_COPIES);
        let mut row = [0u8; 6 * LIGHT as usize];
        for k in 0..7 {
            h.read_bytes(table, &mut row);
            h.write_bytes(at(copies, k * 6 * LIGHT), &row);
        }
        at(copies, 7 * 6 * LIGHT + LIGHT)
    }

    /// `FFXUsbMouseSync(0)`, `FFXUsbMouseRead2(0, dst)`, `dbgPrintf` and
    /// `"USB mouse error.\n"`.
    pub const USB_MOUSE_SYNC: u32 = 0x0085_5bf0;
    pub const USB_MOUSE_READ2: u32 = 0x0085_5bc0;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const MOUSE_ERROR: u32 = 0x00b5_b4f4;
    /// The USB mouse's report (`FFXUsbMouseRead2`'s 0x20 bytes): `+8`
    /// buttons, `+0xc` dx, `+0x10` dy, `+0x14` wheel.
    pub const MOUSE_REPORT: u32 = 0x0132_5280;
    /// The mouse's buttons, whether it moved, its position (`f32` x, y)
    /// and the GUI's integer position (`g_GuMouseX`, `g_GuMouseY`).
    pub const MOUSE_BUTTONS: u32 = 0x0132_5260;
    pub const MOUSE_MOVED: u32 = 0x0132_525c;
    pub const MOUSE_POS: u32 = 0x0132_5248;
    pub const GU_MOUSE: u32 = 0x0132_5240;
    /// `g_SgMouseWheel`, and the camera's mouse deltas (`f32` y-ish,
    /// x-ish) at `0x023cbc08`, `0x023cbc0c`.
    pub const WHEEL: u32 = 0x023c_bc04;
    pub const MOUSE_DELTA: u32 = 0x023c_bc08;
    /// 1248.0 (`f32`), 4.5 (`f64`); 1024.0, 0.5 and 3.0 as above.
    pub const C_1248: u32 = 0x00b5_b508;
    pub const C_4_5: u32 = 0x00b4_b458;

    /// `Sg_MouseMain()` (`0x00851920`, cdecl): reads the USB mouse
    /// (`FFXUsbMouseSync(0)`, `FFXUsbMouseRead2(0, report)`); on an error
    /// prints `"USB mouse error.\n"` and returns 1. Else the buttons and
    /// wheel taken over, the moved flag, the float position moved by
    /// (dx, dy) and clamped to 0..=1024 x 0..=1248 (NaN kept), the GUI's
    /// position `x / 2`, `y / 3` (`_ftol2_sse`, before the clamp) clamped
    /// to 0..=0x200 x 0..=0x1a0, the camera deltas `dx * 3 / 2` and `dy *
    /// 4.5 / 3`. Returns 0.
    pub fn mouse_main(h: &mut impl Host) -> u32 {
        h.call_sym(USB_MOUSE_SYNC, Conv::Cdecl, &[0]);
        let report = h.rebase(MOUSE_REPORT);
        if h.call_sym(USB_MOUSE_READ2, Conv::Cdecl, &[0, report]) != 0 {
            let msg = h.rebase(MOUSE_ERROR);
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[msg]);
            return 1;
        }
        let buttons: u32 = h.read(at(report, 8));
        h.write(h.rebase(MOUSE_BUTTONS), buttons);
        let wheel_at = h.rebase(WHEEL);
        let wheel = h
            .read::<u32>(wheel_at)
            .wrapping_add(h.read(at(report, 0x14)));
        h.write(wheel_at, wheel);
        let dx: u32 = h.read(at(report, 0xc));
        let dy: u32 = h.read(at(report, 0x10));
        h.write(h.rebase(MOUSE_MOVED), u32::from(dx != 0 || dy != 0));
        let pos = h.rebase(MOUSE_POS);
        let fx = int_f32(dx);
        let x = x87::store(x87::add(fx, fld(h, pos)));
        h.write(pos, x);
        let fy = int_f32(dy);
        let y = x87::store(x87::add(fy, fld(h, at(pos, 4))));
        h.write(at(pos, 4), y);
        let mx = fld(h, pos);
        let gx = x87::ftol(h, x87::mul(qword(h, C_HALF), mx));
        let my = fld(h, at(pos, 4));
        let gy = x87::ftol(h, x87::div(my, qword(h, C_THREE)));
        let mut mx = mx;
        let mut my = my;
        if fld(h, h.rebase(C_SCRZ)) < mx {
            h.write(pos, x87::store(fld(h, h.rebase(C_SCRZ))));
            mx = fld(h, pos);
        }
        if fld(h, h.rebase(C_1248)) < my {
            h.write(at(pos, 4), x87::store(fld(h, h.rebase(C_1248))));
            my = fld(h, at(pos, 4));
        }
        if mx < 0.0 {
            h.write(pos, 0u32);
        }
        if my < 0.0 {
            h.write(at(pos, 4), 0u32);
        }
        let clamp = |v: u32, hi: i32| {
            let v = v.cast_signed();
            if v > hi {
                hi
            } else if v < 0 {
                0
            } else {
                v
            }
        };
        let (gx, gy) = (clamp(gx, 0x200), clamp(gy, 0x1a0));
        let gu = h.rebase(GU_MOUSE);
        h.write(gu, gx);
        h.write(at(gu, 4), gy);
        let d = h.rebase(MOUSE_DELTA);
        let sx = dx.cast_signed().wrapping_mul(3) / 2;
        h.write(at(d, 4), x87::store(f64::from(sx)));
        let sy = x87::div(x87::mul(fy, qword(h, C_4_5)), qword(h, C_THREE));
        h.write(d, x87::store(sy));
        0
    }

    /// `Sg_LightSetColor(kind, rgba)`, `Sg_LightSetDir(kind, xyzw)`
    /// (cdecl; both read four `f32` from the pointer), `Angle_Wrap_0084eb60
    /// (a)` (`f32` in, `ST0` out), `_CIsin`, `_CIcos` (thunks).
    pub const LIGHT_SET_COLOR: u32 = 0x0083_6310;
    pub const LIGHT_SET_DIR: u32 = 0x0083_6430;
    pub const ANGLE_WRAP: u32 = 0x0084_eb60;
    pub const CI_SIN: u32 = 0x0094_98c8;
    pub const CI_COS: u32 = 0x0094_98c2;
    /// 3.141592 (`f64`).
    pub const C_PI6: u32 = 0x00b5_82f0;

    /// An angle in degrees (`i32`) to radians as the original rounds it,
    /// then wrapped by `Angle_Wrap_0084eb60`.
    fn wrapped(h: &mut impl Host, deg: u32) -> f64 {
        let rad = x87::store(x87::div(
            x87::mul(int_f32(deg), qword(h, C_PI6)),
            qword(h, C_180),
        ));
        let rad = x87::store(x87::load(rad));
        let w = h.call_sym_st0(ANGLE_WRAP, Conv::Cdecl, &[rad]);
        x87::load(w.to_bits())
    }

    /// `_CIsin`/`_CIcos` of `x`, stored as `f32` and loaded again.
    fn trig(h: &mut impl Host, sym: u32, x: f64) -> f64 {
        let at_ = h.rebase(sym);
        x87::load(h.call_intrinsic(at_, x).to_bits())
    }

    /// `FUN_0084ebd0(light)` (cdecl): applies a debug light table entry
    /// (see [`LIGHTS`]): its colour (`i32` r, g, b / 255, or black when the
    /// byte `+0x1a` is clear; alpha 0) through `Sg_LightSetColor`, and its
    /// direction from the angles `+0x14` (elevation) and `+0x10` (degrees):
    /// `(sin b cos a, sin a, cos b cos a, 0)` through `Sg_LightSetDir`, both
    /// for the light kind `+0x18` (`i16`). `buf` is the original's 16-byte
    /// frame buffer (`ebp - 0x20`). Returns `Sg_LightSetDir`'s `eax`.
    pub fn light_apply(h: &mut impl Host, light: u32, buf: u32) -> u32 {
        h.write(at(buf, 0xc), 0u32);
        let rgb = if h.read::<u8>(at(light, 0x1a)) == 0 {
            [0u32; 3]
        } else {
            let k = qword(h, C_255);
            let mut c = [0u32; 3];
            for (i, v) in (0u32..).zip(&mut c) {
                *v = x87::store(x87::div(int_f32(h.read(at(light, 4 * i))), k));
            }
            c
        };
        h.write(buf, rgb);
        let kind = i32::from(h.read::<i16>(at(light, 0x18))).cast_unsigned();
        h.call_sym(LIGHT_SET_COLOR, Conv::Cdecl, &[kind, buf]);
        let a = wrapped(h, h.read(at(light, 0x14)));
        let sa = trig(h, CI_SIN, a);
        h.write(at(buf, 4), x87::store(sa));
        let ca = trig(h, CI_COS, a);
        let b = wrapped(h, h.read(at(light, 0x10)));
        let sb = trig(h, CI_SIN, b);
        h.write(buf, x87::store(x87::mul(sb, ca)));
        let cb = trig(h, CI_COS, b);
        h.write(at(buf, 8), x87::store(x87::mul(cb, ca)));
        let kind = i32::from(h.read::<i16>(at(light, 0x18))).cast_unsigned();
        h.call_sym(LIGHT_SET_DIR, Conv::Cdecl, &[kind, buf])
    }
    /// The PS2 memory card stubs DECI2's callback calls (`xor eax, eax;
    /// ret`): a read `(socket, buffer, n)` and a write `(socket, buffer,
    /// n)`.
    pub const DECI2_READ: u32 = 0x0062_f650;
    pub const DECI2_WRITE: u32 = 0x0062_f670;

    /// `FUN_0084ed10(event, n, req)` (cdecl): `Sg_Deci2Open`'s DECI2
    /// callback on its request block (`+0` socket, `+4` bytes left to
    /// send, `+8` bytes received, `+0xc` send cursor, `+0x10` receive
    /// buffer, `+0x14` status). 1 read: `n` more bytes into the buffer
    /// (status -1 past 0x10000 bytes or on an error); 2 done: status =
    /// bytes received, the count cleared; 3 write: the next bytes sent,
    /// the cursor and the bytes left moved (status -1 on an error); 4:
    /// status -1 unless everything was sent; 5 nothing; anything else
    /// status -1. Returns what the original leaves in `eax`: the stub's
    /// count, the old received count, the request, or 4.
    pub fn deci2_callback(h: &mut impl Host, event: u32, n: u32, req: u32) -> u32 {
        match event {
            1 => {
                let got: u32 = h.read(at(req, 8));
                let sum = got.wrapping_add(n);
                if sum > 0x1_0000 {
                    fail(h, req);
                    return sum;
                }
                let buf: u32 = h.read(at(req, 0x10));
                let sock: u32 = h.read(req);
                let r = h.call_sym(DECI2_READ, Conv::Cdecl, &[sock, got.wrapping_add(buf), n]);
                if r.cast_signed() < 0 {
                    fail(h, req);
                }
                let got: u32 = h.read(at(req, 8));
                h.write(at(req, 8), got.wrapping_add(r));
                r
            }
            2 => {
                let got: u32 = h.read(at(req, 8));
                h.write(at(req, 0x14), got);
                h.write(at(req, 8), 0u32);
                got
            }
            3 => {
                let sock: u32 = h.read(req);
                let cursor: u32 = h.read(at(req, 0xc));
                let left: u32 = h.read(at(req, 4));
                let r = h.call_sym(DECI2_WRITE, Conv::Cdecl, &[sock, cursor, left]);
                if r.cast_signed() < 0 {
                    fail(h, req);
                    return r;
                }
                let cursor: u32 = h.read(at(req, 0xc));
                h.write(at(req, 0xc), cursor.wrapping_add(r));
                let left: u32 = h.read(at(req, 4));
                h.write(at(req, 4), left.wrapping_sub(r));
                r
            }
            4 => {
                if h.read::<u32>(at(req, 4)) != 0 {
                    fail(h, req);
                }
                req
            }
            5 => 4,
            _ => {
                fail(h, req);
                req
            }
        }
    }

    /// The request's status -1.
    fn fail(h: &mut impl Mem, req: u32) {
        h.write(at(req, 0x14), u32::MAX);
    }
}
