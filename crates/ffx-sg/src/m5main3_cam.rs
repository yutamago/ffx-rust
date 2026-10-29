//! M5 `main`, round 3: the camera debug screen (`camera_debug.c`, the
//! movie and name-input debug windows): its file list, the camera path
//! lists and their dump.
//!
//! The screen's camera lists (`0x025d5f44 + 4 k` the flags, 12 bytes an
//! entry, bit 1 selected; `0x025d5f4c + 4 k` the counts; `0x025d5f54 + 4
//! k` the points, 16 bytes: x, y, z, w) and the current list
//! `0x025d5f90`.

/// The originals over game memory.
#[allow(clippy::many_single_char_names)]
pub mod bridge {
    use ffx_bridge::{Mem, x87};
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    fn cdecl(h: &mut impl Host, sym: u32, args: &[u32]) -> u32 {
        h.call_sym(sym, Conv::Cdecl, args)
    }

    /// `user_free_thunk(p)`, `Gu_MakeFileList(list, pattern)`,
    /// `user_malloc(n)`, `movie_err(msg, file, line)`, `memset`'s thunk,
    /// `Sg_StrCopy_00942030(dst, src)`, the window text thunk
    /// `(window, id, text)` (cdecl).
    pub const USER_FREE: u32 = 0x0068_7010;
    pub const MAKE_FILE_LIST: u32 = 0x0084_98e0;
    pub const USER_MALLOC: u32 = 0x0068_7020;
    pub const MOVIE_ERR: u32 = 0x0076_efa0;
    pub const MEMSET: u32 = 0x0094_964c;
    pub const STR_COPY: u32 = 0x0094_2030;
    pub const SET_TEXT: u32 = 0x0094_11f0;
    /// The file pattern, the error's message and file, the "none" entry
    /// and the empty text.
    pub const PATTERN: u32 = 0x025d_5fa0;
    pub const ERR_MSG: u32 = 0x00b6_8fd0;
    pub const ERR_FILE: u32 = 0x00b6_8fac;
    pub const NONE_ENTRY: u32 = 0x00b6_8fe8;
    pub const EMPTY: u32 = 0x00c6_12bc;
    /// The first page the list gets a "none" entry below
    /// (`0x025d5f94`).
    pub const NONE_BELOW: u32 = 0x025d_5f94;

    /// `set_path_name(window, ms, controls, page)` (`0x00942360`, cdecl):
    /// the file list of `ms` (`+0x208` the list: count, ..., `+0x210`
    /// names) from the pattern, its names copied into `+0x214` (0x200
    /// bytes each, the old buffer freed; `"ms->filename != NULL"` when the
    /// allocation fails), with a "none" entry first when `page` is below
    /// `0x025d5f94`; `+0x218` the count, `+0x21c` cleared. Then controls
    /// 0x103.. (`controls - 0x102` of them) show the names, empty past
    /// the end. The original uses its `ms` argument slot as the name
    /// offset.
    pub fn set_path_name(h: &mut impl Host, window: u32, ms: u32, controls: u32, page: u32) {
        let old: u32 = h.read(at(ms, 0x214));
        if old != 0 {
            cdecl(h, USER_FREE, &[old]);
        }
        let list = at(ms, 0x208);
        let pat = h.rebase(PATTERN);
        cdecl(h, MAKE_FILE_LIST, &[list, pat]);
        h.write(at(ms, 0x214), 0u32);
        h.write(at(ms, 0x21c), 0u32);
        h.write(at(ms, 0x218), 0u32);
        let n: u32 = h.read(list);
        if n == 0 {
            return;
        }
        let names = cdecl(h, USER_MALLOC, &[n.wrapping_add(8) << 9]);
        h.write(at(ms, 0x214), names);
        if names == 0 {
            let (m, f) = (h.rebase(ERR_MSG), h.rebase(ERR_FILE));
            cdecl(h, MOVIE_ERR, &[m, f, 0xad]);
        }
        let n: u32 = h.read(list);
        let buf: u32 = h.read(at(ms, 0x214));
        cdecl(h, MEMSET, &[buf, 0, n.wrapping_add(8) << 9]);
        let below = h.read::<i32>(h.rebase(NONE_BELOW)) > page.cast_signed();
        let n: u32 = h.read(list);
        let first = if below {
            h.write(at(ms, 0x218), n.wrapping_add(1));
            let buf: u32 = h.read(at(ms, 0x214));
            let none = h.rebase(NONE_ENTRY);
            cdecl(h, STR_COPY, &[buf, none]);
            0x200
        } else {
            h.write(at(ms, 0x218), n);
            0
        };
        let mut i = 0i32;
        while i < h.read::<i32>(list) {
            let src: u32 = h.read(at(h.read(at(list, 8)), 4 * i.cast_unsigned()));
            let dst = at(h.read(at(ms, 0x214)), first + 0x200 * i.cast_unsigned());
            cdecl(h, STR_COPY, &[dst, src]);
            i += 1;
        }
        let count = controls.wrapping_sub(0x102).cast_signed();
        let mut i = 0i32;
        while i < count {
            if h.read::<i32>(at(ms, 0x218)) <= i {
                break;
            }
            let mut text = at(h.read(at(ms, 0x214)), 0x200 * i.cast_unsigned());
            if text == 0 {
                text = h.rebase(EMPTY);
            }
            cdecl(h, SET_TEXT, &[window, (i + 0x103).cast_unsigned(), text]);
            i += 1;
        }
        while i < count {
            let e = h.rebase(EMPTY);
            cdecl(h, SET_TEXT, &[window, (i + 0x103).cast_unsigned(), e]);
            i += 1;
        }
    }

    /// `rcuiCopyVector(dst, src)` (cdecl).
    pub const COPY_VECTOR: u32 = 0x0093_ce50;

    /// `FUN_00942260(dst, m, v)` (cdecl): the 4x4 matrix `m` (column-major
    /// `f32`) times the vector `v` into the original's frame buffer `buf`
    /// (`ebp - 0x14`, 16 bytes), each row summed in the original's order
    /// on the x87 at 53 bits, then `rcuiCopyVector(dst, buf)`. Returns
    /// its `eax`.
    pub fn mat_vec(h: &mut impl Host, dst: u32, m: u32, v: u32, buf: u32) -> u32 {
        for r in 0..4 {
            let mut t = [0f64; 4];
            for (k, term) in (0u32..).zip(&mut t) {
                let a = x87::load(h.read(at(m, 4 * (r + 4 * k))));
                *term = x87::mul(a, x87::load(h.read(at(v, 4 * k))));
            }
            let s = if r == 0 {
                x87::add(t[1], t[0])
            } else {
                x87::add(t[0], t[1])
            };
            let s = x87::add(x87::add(s, t[2]), t[3]);
            h.write(at(buf, 4 * r), x87::store(s));
        }
        cdecl(h, COPY_VECTOR, &[dst, buf])
    }

    /// The screen's camera lists, the current list and the camera.
    pub const LIST_FLAGS: u32 = 0x025d_5f44;
    pub const LIST_COUNTS: u32 = 0x025d_5f4c;
    pub const LIST_POINTS: u32 = 0x025d_5f54;
    pub const LIST: u32 = 0x025d_5f90;
    pub const CAMERA: u32 = 0x025d_5ee0;
    /// `MsCameraSetRect(camera, side, &xyz)`, `MsCameraGetPosNow(side,
    /// &xyz)` (writes three `f32`), `dbgPrintf`; 200.0 (`f64`).
    pub const SET_RECT: u32 = 0x007b_f940;
    pub const GET_POS_NOW: u32 = 0x007b_d470;
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    pub const C_200: u32 = 0x00b5_4cf0;

    /// The current list's (flags, count, points).
    fn current(h: &impl Mem) -> (u32, u32, u32) {
        let k = h.read::<u32>(h.rebase(LIST)).wrapping_mul(4);
        (
            h.read(at(h.rebase(LIST_FLAGS), k)),
            h.read(at(h.rebase(LIST_COUNTS), k)),
            h.read(at(h.rebase(LIST_POINTS), k)),
        )
    }

    /// The first selected point (flag bit 1) of the current list:
    /// `Ok(point)`, or `Err(eax)` (the index reached, 0 for an empty list).
    fn selected(h: &impl Mem) -> Result<u32, u32> {
        let (mut flags, n, mut point) = current(h);
        let n = n.cast_signed();
        if n <= 0 {
            return Err(0);
        }
        let mut i = 0;
        while h.read::<u8>(flags) & 2 == 0 {
            i += 1;
            flags = at(flags, 0xc);
            point = at(point, 0x10);
            if i >= n {
                return Err(i.cast_unsigned());
            }
        }
        Ok(point)
    }

    /// `FUN_00941a30()` (cdecl, frame at `ebp`): the camera onto the
    /// first selected point of the current list: side 0 at the point,
    /// side 1 200 below (y - 200). The two `xyz` buffers are the
    /// original's frame (`ebp - 0x24`, `ebp - 0x14`). Returns
    /// `MsCameraSetRect`'s `eax`, or the index reached.
    pub fn rect_from_point(h: &mut impl Host, ebp: u32) -> u32 {
        let p = match selected(h) {
            Ok(p) => p,
            Err(e) => return e,
        };
        let (a, b) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
        let x = x87::store(x87::load(h.read(p)));
        h.write(b, x);
        h.write(a, x87::store(x87::load(x)));
        let y = x87::store(x87::load(h.read(at(p, 4))));
        h.write(at(a, 4), y);
        let z = x87::store(x87::load(h.read(at(p, 8))));
        h.write(at(b, 8), z);
        h.write(at(a, 8), x87::store(x87::load(z)));
        let c: f64 = h.read(h.rebase(C_200));
        let y2 = x87::store(x87::sub(x87::load(h.read(at(p, 4))), c));
        h.write(at(b, 4), y2);
        let cam: u32 = h.read(h.rebase(CAMERA));
        cdecl(h, SET_RECT, &[cam, 0, a]);
        let cam: u32 = h.read(h.rebase(CAMERA));
        cdecl(h, SET_RECT, &[cam, 1, b])
    }

    /// `FUN_00942060()` (cdecl, frame at `ebp`): both camera sides keep
    /// their current position (`MsCameraGetPosNow` into `ebp - 0x24` and
    /// `ebp - 0x14`) but take the first selected point's y. Returns as
    /// [`rect_from_point`].
    pub fn rect_keep_y(h: &mut impl Host, ebp: u32) -> u32 {
        let p = match selected(h) {
            Ok(p) => p,
            Err(e) => return e,
        };
        let (a, b) = (ebp.wrapping_sub(0x24), ebp.wrapping_sub(0x14));
        cdecl(h, GET_POS_NOW, &[0, a]);
        cdecl(h, GET_POS_NOW, &[1, b]);
        let y = x87::store(x87::load(h.read(at(p, 4))));
        h.write(at(b, 4), y);
        h.write(at(a, 4), x87::store(x87::load(y)));
        let cam: u32 = h.read(h.rebase(CAMERA));
        cdecl(h, SET_RECT, &[cam, 0, a]);
        let cam: u32 = h.read(h.rebase(CAMERA));
        cdecl(h, SET_RECT, &[cam, 1, b])
    }

    /// The dump's formats: header, count, `".float %f,%f,%f,%f\n"`,
    /// footer.
    pub const DUMP_FORMATS: [u32; 4] = [0x00b6_9060, 0x00b6_9078, 0x00b6_9088, 0x00b6_909c];

    /// `FUN_00941ae0()` (cdecl): the current list's points printed as
    /// assembler `.float` lines (each `f32` widened to `f64`) between a
    /// header with the count and a footer. Returns the last `dbgPrintf`'s
    /// `eax`, or the list index when the list is empty.
    pub fn dump_points(h: &mut impl Host) -> u32 {
        let k: u32 = h.read(h.rebase(LIST));
        let (_, n, points) = current(h);
        if n == 0 {
            return k;
        }
        let f = h.rebase(DUMP_FORMATS[0]);
        cdecl(h, DBG_PRINTF, &[f]);
        let f = h.rebase(DUMP_FORMATS[1]);
        cdecl(h, DBG_PRINTF, &[f, n]);
        let mut p = points;
        let mut left = n.cast_signed();
        while left > 0 {
            let mut args = vec![h.rebase(DUMP_FORMATS[2])];
            for i in 0..4 {
                let d = x87::load(h.read(at(p, 4 * i))).to_bits();
                #[allow(clippy::cast_possible_truncation)] // the two halves
                args.extend([d as u32, (d >> 32) as u32]);
            }
            cdecl(h, DBG_PRINTF, &args); // call width: 9 (format, four f64)
            p = at(p, 0x10);
            left -= 1;
        }
        let f = h.rebase(DUMP_FORMATS[3]);
        cdecl(h, DBG_PRINTF, &[f])
    }

    /// The path file's name (a byte string, empty: no file), `sceOpen(name,
    /// mode)`, `sceWrite(fd, p, n)`, `sceClose(fd)` (PS2 stubs), and the
    /// error and done formats.
    pub const PATH_FILE: u32 = 0x025d_60a0;
    pub const SCE_OPEN: u32 = 0x0062_fa30;
    pub const SCE_WRITE: u32 = 0x0062_fdc0;
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    pub const FMT_OPEN_ERR: u32 = 0x00b6_9168;
    pub const FMT_WRITTEN: u32 = 0x00b6_9180;

    /// `FUN_00942140()` (cdecl, frame at `ebp`, 0xc bytes): the current
    /// list written to the path file (when named and the list has flags,
    /// points and a count): the count and three zero words, then each
    /// point's x, y, z and a zero `f32`, each word through `sceWrite` (the
    /// count from `ebp - 8`, zeros from `ebp - 4` and `ebp - 0xc`).
    pub fn write_points(h: &mut impl Host, ebp: u32) {
        let name = h.rebase(PATH_FILE);
        if h.read::<u8>(name) == 0 {
            return;
        }
        let k = h.read::<u32>(h.rebase(LIST)).wrapping_mul(4);
        if h.read::<u32>(at(h.rebase(LIST_FLAGS), k)) == 0 {
            return;
        }
        let count_at = ebp.wrapping_sub(8);
        let n: u32 = h.read(at(h.rebase(LIST_COUNTS), k));
        h.write(count_at, n);
        if n == 0 {
            return;
        }
        let mut p: u32 = h.read(at(h.rebase(LIST_POINTS), k));
        if p == 0 {
            return;
        }
        let fd = cdecl(h, SCE_OPEN, &[name, 0x602]);
        if fd.cast_signed() < 0 {
            let f = h.rebase(FMT_OPEN_ERR);
            cdecl(h, DBG_PRINTF, &[f, name]);
            return;
        }
        cdecl(h, SCE_WRITE, &[fd, count_at, 4]);
        let zero = ebp.wrapping_sub(4);
        h.write(zero, 0u32);
        for _ in 0..3 {
            cdecl(h, SCE_WRITE, &[fd, zero, 4]);
        }
        let fzero = ebp.wrapping_sub(0xc);
        h.write(fzero, 0u32);
        let mut i = 0i32;
        while i < h.read::<i32>(count_at) {
            for o in [0, 4, 8] {
                cdecl(h, SCE_WRITE, &[fd, at(p, o), 4]);
            }
            cdecl(h, SCE_WRITE, &[fd, fzero, 4]);
            p = at(p, 0x10);
            i += 1;
        }
        cdecl(h, SCE_CLOSE, &[fd]);
        let f = h.rebase(FMT_WRITTEN);
        cdecl(h, DBG_PRINTF, &[f, name]);
    }
}
