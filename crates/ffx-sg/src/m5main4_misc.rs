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

    /// The ID draw's mode (`0x013034b4`), its "colour from the triangle"
    /// flag (`0x013034b0`), the packet's quadword count (`0x01303648`) and
    /// base (`0x0130364c`); `Sg_IdExistShade()`, `Sg_IdDrawTriangles(verts,
    /// model, m, packet)`, `get_ptr32_func(p)`, `Sg_IdProject(out, v, m)`
    /// (cdecl).
    pub const ID_MODE: u32 = 0x0130_34b4;
    pub const ID_OWN_COLOUR: u32 = 0x0130_34b0;
    pub const ID_COUNT: u32 = 0x0130_3648;
    pub const ID_PACKET: u32 = 0x0130_364c;
    pub const ID_EXIST_SHADE: u32 = 0x0084_4bb0;
    pub const ID_DRAW_TRIANGLES: u32 = 0x0084_47a0;
    pub const GET_PTR32: u32 = 0x00a4_4650;
    pub const ID_PROJECT: u32 = 0x0084_5030;
    pub const ID_DRAW_LINES_SYM: u32 = 0x0084_4490;

    /// `i16` vertex `k`'s `(x, y, z)` (8-byte vertices from `verts`) as
    /// `f32` into `v`.
    fn load_vertex(h: &mut impl Host, verts: u32, k: u32, v: u32) {
        for j in 0..3 {
            let c = h.read::<i16>(at(verts, k.wrapping_mul(8).wrapping_add(2 * j)));
            h.write(at(v, 4 * j), f32::from(c).to_bits());
        }
    }

    /// `FUN_00844490(verts, model, m, packet, a5)` (cdecl): the debug ID
    /// view of `model` (`+8` the triangle count, `+0xc` the triangles: 16
    /// bytes, three `i16` vertex numbers, the colour `+0xc`). In mode 3
    /// with shading the triangles are drawn filled (`Sg_IdDrawTriangles`,
    /// its `eax`); else (mode 2 colours by the triangle, the others by its
    /// number) a GIF packet at `packet` with one coloured outline per
    /// triangle whose three corners project (`Sg_IdProject` into the
    /// packet, the corners in `vecs`: the original's three 16-byte `xyzw`
    /// locals at `ebp - 0x34`, `w` = 1), its count in `+0x30`, and the
    /// end tag. Returns the end tag (`(count - 1) | 0x70000000`).
    pub fn id_draw_lines(
        h: &mut impl Host,
        verts: u32,
        model: u32,
        m: u32,
        packet: u32,
        a5: u32,
        vecs: u32,
    ) -> u32 {
        let own = h.rebase(ID_OWN_COLOUR);
        h.write(own, 0u32);
        match h.read::<u32>(h.rebase(ID_MODE)) {
            2 => h.write(own, 1u32),
            3 if cdecl(h, ID_EXIST_SHADE, &[]) != 0 => {
                return cdecl(h, ID_DRAW_TRIANGLES, &[verts, model, m, packet, a5]);
            }
            _ => {}
        }
        for (o, x) in [
            (0x10, 1u32),
            (0x14, 0x1000_0000),
            (0x18, 0x00ee_eeee),
            (0x1c, 0),
            (0x20, 0x0007_07fa),
            (0x24, 0),
            (0x28, 0x47),
            (0x2c, 0),
            (0x30, 0x8000),
            (0x34, 0x4001_c000),
            (0x38, 0x4441),
            (0x3c, 0),
        ] {
            h.write(at(packet, o), x);
        }
        let tris_at: u32 = h.read(at(model, 0xc));
        let tris = cdecl(h, GET_PTR32, &[tris_at]);
        let (count_at, base_at) = (h.rebase(ID_COUNT), h.rebase(ID_PACKET));
        h.write(base_at, packet);
        h.write(count_at, 4u32);
        let one = 1.0f32.to_bits();
        let (b, a, c) = (vecs, at(vecs, 0x10), at(vecs, 0x20));
        for v in [a, b, c] {
            h.write(at(v, 0xc), one);
        }
        let mut drawn = 0u32;
        let mut i = 0i32;
        let mut t = tris;
        while i < i32::from(h.read::<i16>(at(model, 8))) {
            let mut shown = true;
            for (j, v) in [(0u32, a), (2, b), (4, c)] {
                let k = i32::from(h.read::<i16>(at(t, j))).cast_unsigned();
                load_vertex(h, verts, k, v);
                let n: u32 = h.read(count_at);
                let base: u32 = h.read(base_at);
                let out = at(base, n.wrapping_add(1 + j / 2) << 4);
                if cdecl(h, ID_PROJECT, &[out, v, m]) != 0 {
                    shown = false;
                    break;
                }
            }
            if shown {
                let colour = if h.read::<u32>(own) == 0 {
                    i.cast_unsigned()
                } else {
                    h.read(at(t, 0xc))
                } & 0x3f;
                let n: u32 = h.read(count_at);
                let base: u32 = h.read(base_at);
                let q = at(base, n << 4);
                h.write(q, (colour & 3) << 6);
                h.write(at(q, 4), (colour >> 2 & 3) << 6);
                h.write(at(q, 8), (colour >> 4 & 3) << 6);
                h.write(at(q, 0xc), 0x80u32);
                h.write(count_at, n.wrapping_add(4));
                drawn = drawn.wrapping_add(1);
            }
            i += 1;
            t = at(t, 0x10);
        }
        let flags: u32 = h.read(at(packet, 0x30));
        h.write(at(packet, 0x30), flags ^ ((flags ^ drawn) & 0x7fff));
        let base: u32 = h.read(base_at);
        let n: u32 = h.read(count_at);
        for o in [4, 8, 0xc] {
            h.write(at(base, o), 0u32);
        }
        let tag = n.wrapping_sub(1) | 0x7000_0000;
        h.write(base, tag);
        tag
    }
}
