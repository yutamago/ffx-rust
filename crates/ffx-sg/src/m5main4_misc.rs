//! M5 stage 1, lists `small` and `main`, round 4: leftovers of the debug
//! tools (the movie camera's default path, the camera turned onto a
//! point).

/// The originals over game memory.
#[allow(clippy::many_single_char_names)] // registers and indices
pub mod bridge {
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    /// The camera debug lists: the current list `0x025d5f90`, and per
    /// list the point flags `0x025d5f44[]`, the point count
    /// `0x025d5f4c[]` and the points `0x025d5f54[]`.
    pub const CUR_LIST: u32 = 0x025d_5f90;
    pub const LIST_FLAGS: u32 = 0x025d_5f44;
    pub const LIST_COUNT: u32 = 0x025d_5f4c;
    pub const LIST_POINTS: u32 = 0x025d_5f54;
    /// The `f32` constants 10, -10, -20, -30.
    pub const TEN: u32 = 0x00b9_2318;
    pub const MINUS_TEN: u32 = 0x00b3_04fc;
    pub const MINUS_TWENTY: u32 = 0x00b5_4d08;
    pub const MINUS_THIRTY: u32 = 0x00b9_235c;

    /// `FUN_00941980()` (cdecl): an empty current list gets five default
    /// points (16 bytes each: `x` 10, -10, -20, -30 for the first, third,
    /// fourth and fifth, 0 elsewhere) whose flag words (12 bytes apart)
    /// are set to 1.
    pub fn default_points(h: &mut impl Host) {
        let k: u32 = h.read(h.rebase(CUR_LIST));
        let count_at = at(h.rebase(LIST_COUNT), k.wrapping_mul(4));
        if h.read::<u32>(count_at) != 0 {
            return;
        }
        let flags: u32 = h.read(at(h.rebase(LIST_FLAGS), k.wrapping_mul(4)));
        let points: u32 = h.read(at(h.rebase(LIST_POINTS), k.wrapping_mul(4)));
        h.write(count_at, 5u32);
        let firsts = [
            Some(TEN),
            None,
            Some(MINUS_TEN),
            Some(MINUS_TWENTY),
            Some(MINUS_THIRTY),
        ];
        for (i, first) in (0u32..).zip(firsts) {
            h.write(at(flags, 12 * i), 1u32);
            // fld/fstp dword of a normal constant: its bits
            let x: u32 = match first {
                Some(c) => h.read(h.rebase(c)),
                None => 0,
            };
            let p = at(points, 16 * i);
            h.write(p, x);
            for j in 1..4 {
                h.write(at(p, 4 * j), 0u32);
            }
        }
    }

    /// The debug camera (`0x025d5ee0`), its polar angles and distance
    /// (`f32` at `0x025d5f80/84/88`), the constants 200 (`f32`) and pi
    /// (`f64`); `MsCameraGetAngle(&yaw, &pitch, &len)`,
    /// `MsCameraSetRect(cam, side, xyz)`, `Cam_SetPolar(cam, side, yaw,
    /// pitch, len)` (cdecl).
    pub const CAMERA: u32 = 0x025d_5ee0;
    pub const YAW: u32 = 0x025d_5f80;
    pub const PITCH: u32 = 0x025d_5f84;
    pub const DISTANCE: u32 = 0x025d_5f88;
    pub const F200: u32 = 0x00b6_59bc;
    pub const PI_F64: u32 = 0x00b9_2288;
    pub const GET_ANGLE: u32 = 0x007b_d2d0;
    pub const SET_RECT: u32 = 0x007b_f940;
    pub const SET_POLAR: u32 = 0x007b_f700;

    /// `fld dword` / `fstp dword` of the bits at `p`.
    fn copy_f32(h: &impl Host, p: u32) -> u32 {
        x87::store(x87::load(h.read(p)))
    }

    /// `FUN_00942520()` (cdecl): the debug camera turned round onto the
    /// first selected point (flag bit 1) of the current list: its angles
    /// read (`MsCameraGetAngle`), side 0 set at the point (`xyz`, the
    /// original's 12-byte local at `ebp - 0x14`), the distance 200 and
    /// the pitch turned by pi (`f32` + `f64`, rounded to `f32`), side 1
    /// set from them (`Cam_SetPolar`). Returns its `eax`, or the index
    /// reached (0 for an empty list) when no point is selected.
    pub fn point_orbit(h: &mut impl Host, xyz: u32) -> u32 {
        let k: u32 = h.read(h.rebase(CUR_LIST));
        let n: i32 = h.read(at(h.rebase(LIST_COUNT), k.wrapping_mul(4)));
        let mut flags: u32 = h.read(at(h.rebase(LIST_FLAGS), k.wrapping_mul(4)));
        let mut point: u32 = h.read(at(h.rebase(LIST_POINTS), k.wrapping_mul(4)));
        if n <= 0 {
            return 0;
        }
        let mut i = 0i32;
        while h.read::<u8>(flags) & 2 == 0 {
            i += 1;
            flags = at(flags, 0xc);
            point = at(point, 0x10);
            if i >= n {
                return i.cast_unsigned();
            }
        }
        let (yaw, pitch, dist) = (h.rebase(YAW), h.rebase(PITCH), h.rebase(DISTANCE));
        cdecl(h, GET_ANGLE, &[yaw, pitch, dist]);
        for j in 0..3 {
            let v = copy_f32(h, at(point, 4 * j));
            h.write(at(xyz, 4 * j), v);
        }
        let cam: u32 = h.read(h.rebase(CAMERA));
        let d = copy_f32(h, h.rebase(F200));
        h.write(dist, d);
        let pi = f64::from_bits(h.read::<u64>(h.rebase(PI_F64)));
        let turned = x87::store(x87::add(x87::load(h.read(pitch)), pi));
        h.write(pitch, turned);
        cdecl(h, SET_RECT, &[cam, 0, xyz]);
        let (d, p, y) = (copy_f32(h, dist), copy_f32(h, pitch), copy_f32(h, yaw));
        let cam: u32 = h.read(h.rebase(CAMERA));
        cdecl(h, SET_POLAR, &[cam, 1, y, p, d])
    }
}
