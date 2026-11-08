//! M5 round 11 (`tail11`): `MsBtlPosWriteBin` (`0x007ca580`, cdecl
//! `(name, out)`), the battle position table (`g_BtlPosTable`,
//! `0x0113e278`) written out as the binary file: to `name` through
//! `sceOpen` / `sceWrite` (a debug path; `sceWrite` writes nothing on the
//! PC), or into the buffer `out` when `name` is 0 (the only caller,
//! `MsBtlPosWriteAll_007c9bc0`).
//!
//! The table (`S`): `+0` the kind (0: full records), `+4..+0xc` an offset,
//! `+0x10..+0x18` a scale, `+0x1c` the record count, `+0x24` / `+0x2c`
//! extra words, `+0x30` the records (0x74 bytes: `+0` position, `+0xc`,
//! `+0x30`, `+0x54` point counts with the point arrays `+0x28`/`+0x2c`,
//! `+0x4c`/`+0x50`, `+0x60`/`+0x64`; `+0x1c` and `+0x40` two vectors;
//! `+0x68` sub-records of 0x14 bytes at `+0x70`, each with `+8` points at
//! `+0x10`). Every point goes out as `offset + position + point * scale`
//! (16 bytes, w 0); the headers carry the byte counts and the 16-byte
//! offsets of each part in the file.
//!
//! x87: each coordinate is `(a + b) + p * s` at 53 bits in the original's
//! operand order, rounded to `f32` where it stores (`ffx_bridge::x87`).

/// The original as a bridge over [`ffx_graphic::host::Host`].
pub mod bridge {
    use ffx_bridge::x87::{add, load, mul, store};
    use ffx_graphic::host::{Conv, Host};

    /// `g_BtlPosTable` (`0x0113e278`).
    pub const TABLE: u32 = 0x0113_e278;
    /// The current record (`0x0113e27c`) and sub-record (`0x0113e284`),
    /// left at the last ones visited.
    pub const CUR: u32 = 0x0113_e27c;
    /// See [`CUR`].
    pub const CUR_SUB: u32 = 0x0113_e284;
    /// The parser's file name and line (`0x0113e28c`, `0x0113e288`) for the
    /// error message, and the error word (`0x0113e238`).
    pub const FILE: u32 = 0x0113_e28c;
    /// See [`FILE`].
    pub const LINE: u32 = 0x0113_e288;
    /// See [`FILE`].
    pub const ERROR: u32 = 0x0113_e238;
    /// `g_BtlPosFocus` (`0x023114f4`).
    pub const FOCUS: u32 = 0x0231_14f4;
    /// `sceOpen(name, mode)`, `sceWrite(fd, buf, n)`, `sceClose(fd)`,
    /// `dbgPrintf(format, ...)`, `MsStructClear(p, n)` (cdecl).
    pub const SCE_OPEN: u32 = 0x0062_fa30;
    /// See [`SCE_OPEN`].
    pub const SCE_WRITE: u32 = 0x0062_fdc0;
    /// See [`SCE_OPEN`].
    pub const SCE_CLOSE: u32 = 0x0062_f610;
    /// See [`SCE_OPEN`].
    pub const DBG_PRINTF: u32 = 0x0062_f500;
    /// See [`SCE_OPEN`].
    pub const STRUCT_CLEAR: u32 = 0x0079_f230;
    /// MSVCR110 import slots: `malloc`, `free`.
    pub const MALLOC_IAT: u32 = 0x00b0_c468;
    /// See [`MALLOC_IAT`].
    pub const FREE_IAT: u32 = 0x00b0_c370;
    /// The formats: `"(%s) : "`, `"line %d : "`, the open error, `"Out <%s>\n"`.
    pub const MSG_FILE: u32 = 0x00b5_5904;
    /// See [`MSG_FILE`].
    pub const MSG_LINE: u32 = 0x00b5_590c;
    /// See [`MSG_FILE`].
    pub const MSG_OPEN: u32 = 0x00b5_5938;
    /// See [`MSG_FILE`].
    pub const MSG_OUT: u32 = 0x00b5_56a4;

    /// Bytes of [`write_bin`]'s frame, from the original's `ebp - 0xa8` up
    /// to `ebp`: the records `sceWrite` is handed: the full header at `+0`
    /// (`ebp - 0xa8`, 0x60 bytes), the short header at `+0x60` (`ebp -
    /// 0x48`, 0x20), the sub-record at `+0x84` (`ebp - 0x24`), the point at
    /// `+0x94` (`ebp - 0x14`), 16 bytes each.
    pub const FRAME: u32 = 0xa8;
    const FULL: u32 = 0;
    const SHORT: u32 = 0x60;
    const SUB: u32 = 0x84;
    const POINT: u32 = 0x94;

    /// Where the records go.
    #[derive(Clone, Copy)]
    enum Out {
        /// `sceWrite(fd, record, n)`.
        File(u32),
        /// Copied to the cursor, which moves on.
        Buffer(u32),
    }

    struct W {
        frame: u32,
        out: Out,
    }

    impl W {
        fn clear(&self, h: &mut impl Host, at: u32, n: u32) {
            h.call_sym(STRUCT_CLEAR, Conv::Cdecl, &[self.frame + at, n]);
        }

        fn emit(&mut self, h: &mut impl Host, at: u32, n: u32) {
            match self.out {
                Out::File(fd) => {
                    h.call_sym(SCE_WRITE, Conv::Cdecl, &[fd, self.frame + at, n]);
                }
                Out::Buffer(cursor) => {
                    for k in (0..n).step_by(4) {
                        let v: u32 = h.read(self.frame + at + k);
                        h.write(cursor.wrapping_add(k), v);
                    }
                    self.out = Out::Buffer(cursor.wrapping_add(n));
                }
            }
        }

        /// One point: `offset + position + p * scale` of the record `e` and
        /// the point at `p`, w 0.
        fn point(&mut self, h: &mut impl Host, e: u32, p: u32) {
            self.clear(h, POINT, 0x10);
            let s: u32 = h.read(h.rebase(TABLE));
            let x = add(mul(rd(h, p), rd(h, s + 0x10)), add(rd(h, s + 4), rd(h, e)));
            let y = add(
                mul(rd(h, p.wrapping_add(4)), rd(h, s + 0x14)),
                add(rd(h, e.wrapping_add(4)), rd(h, s + 8)),
            );
            let z = add(
                mul(rd(h, p.wrapping_add(8)), rd(h, s + 0x18)),
                add(rd(h, e.wrapping_add(8)), rd(h, s + 0xc)),
            );
            for (k, v) in [x, y, z].into_iter().enumerate() {
                h.write(self.frame + POINT + 4 * k as u32, store(v));
            }
            self.emit(h, POINT, 0x10);
        }

        /// The `count` points of the array at `e + array` (12 bytes each).
        fn points(&mut self, h: &mut impl Host, e: u32, count: u32, array: u32) {
            let mut k = 0i32;
            while k < h.read::<i32>(e.wrapping_add(count)) {
                let base: u32 = h.read(e.wrapping_add(array));
                self.point(
                    h,
                    e,
                    base.wrapping_add(12u32.wrapping_mul(k.cast_unsigned())),
                );
                k += 1;
            }
        }
    }

    fn rd(h: &impl Host, at: u32) -> f64 {
        load(h.read(at))
    }

    /// Six times the record count (re-read, as the original).
    fn six(h: &impl Host, s: u32) -> u32 {
        h.read::<u32>(s.wrapping_add(0x1c)).wrapping_mul(6)
    }

    fn table(h: &impl Host) -> u32 {
        h.read(h.rebase(TABLE))
    }

    fn count(h: &impl Host) -> i32 {
        h.read(table(h).wrapping_add(0x1c))
    }

    fn record(h: &impl Host, i: i32) -> u32 {
        let records: u32 = h.read(table(h).wrapping_add(0x30));
        records.wrapping_add(0x74u32.wrapping_mul(i.cast_unsigned()))
    }

    /// `MsBtlPosWriteBin(name, out)` (`0x007ca580`, cdecl; `frame` see
    /// [`FRAME`]) -> the `eax` the original leaves: with `out` the record
    /// count (0 when none) or, for full tables with extra words, the last
    /// extra word; with `name` `sceClose`'s (or `free`'s) `eax`, or the
    /// last `dbgPrintf`'s when the file does not open. `name` and `out`
    /// both 0: nothing; the original returns its stack cookie there (junk,
    /// brief 2026-10-24: 0; the only caller passes a buffer).
    pub fn write_bin(h: &mut impl Host, frame: u32, name: u32, out: u32) -> u32 {
        let mut fd = 0u32;
        let mut buf = 0u32;
        let target = if name != 0 {
            fd = h.call_sym(SCE_OPEN, Conv::Cdecl, &[name, 0x602]);
            let malloc: u32 = h.read(h.rebase(MALLOC_IAT));
            buf = h.call(malloc, Conv::Cdecl, &[0x1000]);
            if fd.cast_signed() < 0 {
                let file: u32 = h.read(h.rebase(FILE));
                if file != 0 {
                    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[h.rebase(MSG_FILE), file]);
                }
                let line: u32 = h.read(h.rebase(LINE));
                if line != 0 {
                    h.call_sym(DBG_PRINTF, Conv::Cdecl, &[h.rebase(MSG_LINE), line]);
                }
                let eax = h.call_sym(DBG_PRINTF, Conv::Cdecl, &[h.rebase(MSG_OPEN), name]);
                h.write(h.rebase(ERROR), 2u32);
                return eax;
            }
            Out::File(fd)
        } else if out == 0 {
            return 0;
        } else {
            Out::Buffer(out)
        };
        if h.read::<u32>(h.rebase(FOCUS)) != 0 && name != 0 {
            h.call_sym(DBG_PRINTF, Conv::Cdecl, &[h.rebase(MSG_OUT), name]);
        }
        let mut w = W { frame, out: target };
        let full = h.read::<u32>(table(h)) == 0;
        let eax = if full {
            full_table(h, &mut w)
        } else {
            short_table(h, &mut w)
        };
        if name == 0 {
            return eax;
        }
        // the trailing sceWrite of `buf` needs a length the original keeps
        // at 0: never
        let mut eax = buf;
        if buf != 0 {
            let free: u32 = h.read(h.rebase(FREE_IAT));
            eax = h.call(free, Conv::Cdecl, &[buf]);
        }
        if fd.cast_signed() > 0 {
            eax = h.call_sym(SCE_CLOSE, Conv::Cdecl, &[fd]);
        }
        eax
    }

    /// The short form (kind not 0): 0x20-byte headers, then each record's
    /// first two point arrays. Returns the record count (0 when none).
    fn short_table(h: &mut impl Host, w: &mut W) -> u32 {
        let mut points = 0u32;
        let mut i = 0i32;
        while i < count(h) {
            w.clear(h, SHORT, 0x20);
            let s = table(h);
            let at = w.frame + SHORT;
            h.write(at, h.read::<u8>(s));
            h.write(at + 1, h.read::<u8>(s.wrapping_add(0x1c)));
            let e = record(h, i);
            h.write(at + 4, h.read::<u8>(e.wrapping_add(0xc)));
            let n: u32 = h.read(s.wrapping_add(0x1c));
            h.write(
                at + 0x10,
                points.wrapping_add(n.wrapping_mul(2)).wrapping_shl(4),
            );
            h.write(h.rebase(CUR), e);
            let a: u32 = h.read(e.wrapping_add(0xc));
            points = points.wrapping_add(a);
            let n: u32 = h.read(s.wrapping_add(0x1c));
            h.write(
                at + 0x14,
                points.wrapping_add(n.wrapping_mul(2)).wrapping_shl(4),
            );
            points = points.wrapping_add(a);
            h.write(at + 6, 0u8);
            h.write(at + 0x18, 0u32);
            w.emit(h, SHORT, 0x20);
            i += 1;
        }
        let mut i = 0i32;
        while i < count(h) {
            let e = record(h, i);
            h.write(h.rebase(CUR), e);
            w.points(h, e, 0xc, 0x28);
            w.points(h, e, 0xc, 0x2c);
            i += 1;
        }
        i.max(0).cast_unsigned()
    }

    /// The full form (kind 0): the 0x60-byte headers, the sub-records'
    /// 16-byte headers, every record's six point arrays, the sub-records'
    /// points, the extra words. Returns the last extra word, or else the
    /// record count (0 when none).
    #[allow(clippy::too_many_lines)] // the original's six passes, in order
    fn full_table(h: &mut impl Host, w: &mut W) -> u32 {
        // the totals: sub-records, points of the three pairs, sub points
        let (mut subs, mut pairs, mut sub_points) = (0u32, 0u32, 0u32);
        let n0 = count(h);
        if n0 > 0 {
            let records: u32 = h.read(table(h).wrapping_add(0x30));
            for i in 0..n0.cast_unsigned() {
                let e = records.wrapping_add(0x74 * i);
                let m: i32 = h.read(e.wrapping_add(0x68));
                subs = subs.wrapping_add(m.cast_unsigned());
                let abc = [0x54u32, 0x30, 0xc]
                    .into_iter()
                    .fold(0u32, |t, off| t.wrapping_add(h.read(e.wrapping_add(off))));
                pairs = pairs.wrapping_add(abc.wrapping_mul(2));
                h.write(h.rebase(CUR), e);
                let list: u32 = h.read(e.wrapping_add(0x70));
                for k in 0..m.max(0).cast_unsigned() {
                    let r = list.wrapping_add(0x14 * k);
                    sub_points = sub_points.wrapping_add(h.read(r.wrapping_add(8)));
                    if k + 1 == m.cast_unsigned() {
                        h.write(h.rebase(CUR_SUB), r);
                    }
                }
            }
        }
        // the headers
        let (mut part, mut sub_index) = (0u32, 0u32);
        let mut i = 0i32;
        while i < count(h) {
            w.clear(h, FULL, 0x60);
            let s = table(h);
            let at = w.frame + FULL;
            h.write(at, h.read::<u8>(s));
            h.write(at + 1, h.read::<u8>(s.wrapping_add(0x1c)));
            let e = record(h, i);
            h.write(h.rebase(CUR), e);
            let mut words = Vec::with_capacity(8);
            words.push(subs.wrapping_add(six(h, s)).wrapping_add(part));
            for off in [0xcu32, 0xc, 0x54, 0x54, 0x30] {
                part = part.wrapping_add(h.read(e.wrapping_add(off)));
                words.push(subs.wrapping_add(six(h, s)).wrapping_add(part));
            }
            part = part.wrapping_add(h.read(e.wrapping_add(0x30)));
            words.push(sub_index.wrapping_add(six(h, s)));
            words.push(
                sub_points
                    .wrapping_add(six(h, s))
                    .wrapping_add(pairs)
                    .wrapping_add(subs),
            );
            for (k, v) in words.into_iter().enumerate() {
                h.write(at + 0x10 + 4 * k as u32, v.wrapping_shl(4));
            }
            for (k, off) in [0xcu32, 0x54, 0x30].into_iter().enumerate() {
                h.write(at + 4 + k as u32, h.read::<u8>(e.wrapping_add(off)));
            }
            h.write(at + 8, h.read::<u8>(e.wrapping_add(0x68)));
            h.write(at + 9, h.read::<u8>(s.wrapping_add(0x24)));
            h.write(at + 2, h.read::<u8>(e.wrapping_add(0x18)));
            h.write(at + 3, h.read::<u8>(e.wrapping_add(0x3c)));
            let mut origin = [0f64; 3];
            for c in 0..3u32 {
                origin[c as usize] = add(rd(h, e + 4 * c), rd(h, s + 4 + 4 * c));
                h.write(at + 0x30 + 4 * c, store(origin[c as usize]));
            }
            for (row, vec) in [(0x40u32, 0x1cu32), (0x50, 0x40)] {
                for c in 0..3u32 {
                    let v = add(
                        mul(rd(h, e + vec + 4 * c), rd(h, s + 0x10 + 4 * c)),
                        add(rd(h, e + 4 * c), rd(h, s + 4 + 4 * c)),
                    );
                    h.write(at + row + 4 * c, store(v));
                }
            }
            w.emit(h, FULL, 0x60);
            let e: u32 = h.read(h.rebase(CUR));
            sub_index = sub_index.wrapping_add(h.read(e.wrapping_add(0x68)));
            i += 1;
        }
        // the sub-records
        let mut offset = 0u32;
        let mut i = 0i32;
        while i < count(h) {
            let e = record(h, i);
            h.write(h.rebase(CUR), e);
            let mut k = 0i32;
            while k < h.read::<i32>(e.wrapping_add(0x68)) {
                w.clear(h, SUB, 0x10);
                let s = table(h);
                let list: u32 = h.read(e.wrapping_add(0x70));
                let r = list.wrapping_add(0x14u32.wrapping_mul(k.cast_unsigned()));
                let six = h.read::<u32>(s.wrapping_add(0x1c)).wrapping_mul(6);
                let at = w.frame + SUB;
                let v = pairs
                    .wrapping_add(six)
                    .wrapping_add(subs)
                    .wrapping_add(offset);
                h.write(at, v.wrapping_shl(4));
                h.write(at + 4, h.read::<u8>(r));
                h.write(at + 5, h.read::<u8>(r.wrapping_add(4)));
                h.write(h.rebase(CUR_SUB), r);
                h.write(at + 6, h.read::<u8>(r.wrapping_add(8)));
                w.emit(h, SUB, 0x10);
                offset = offset.wrapping_add(h.read(r.wrapping_add(8)));
                k += 1;
            }
            i += 1;
        }
        // the records' points
        let mut i = 0i32;
        while i < count(h) {
            let e = record(h, i);
            h.write(h.rebase(CUR), e);
            for (n, array) in [
                (0xc, 0x28),
                (0xc, 0x2c),
                (0x54, 0x60),
                (0x54, 0x64),
                (0x30, 0x4c),
                (0x30, 0x50),
            ] {
                w.points(h, e, n, array);
            }
            i += 1;
        }
        // the sub-records' points
        let mut i = 0i32;
        while i < count(h) {
            let e = record(h, i);
            h.write(h.rebase(CUR), e);
            let mut k = 0i32;
            while k < h.read::<i32>(e.wrapping_add(0x68)) {
                let list: u32 = h.read(e.wrapping_add(0x70));
                let r = list.wrapping_add(0x14u32.wrapping_mul(k.cast_unsigned()));
                h.write(h.rebase(CUR_SUB), r);
                let mut j = 0i32;
                while j < h.read::<i32>(r.wrapping_add(8)) {
                    let p: u32 = h.read(r.wrapping_add(0x10));
                    w.point(h, e, p.wrapping_add(12u32.wrapping_mul(j.cast_unsigned())));
                    j += 1;
                }
                k += 1;
            }
            i += 1;
        }
        let mut eax = i.max(0).cast_unsigned();
        // the extra words
        let mut k = 0i32;
        while k < h.read::<i32>(table(h).wrapping_add(0x24)) {
            let words: u32 = h.read(table(h).wrapping_add(0x2c));
            let at = words.wrapping_add(4u32.wrapping_mul(k.cast_unsigned()));
            match w.out {
                Out::File(fd) => {
                    h.call_sym(SCE_WRITE, Conv::Cdecl, &[fd, at, 4]);
                }
                Out::Buffer(cursor) => {
                    eax = h.read(at);
                    h.write(cursor, eax);
                    w.out = Out::Buffer(cursor.wrapping_add(4));
                }
            }
            k += 1;
        }
        eax
    }
}
