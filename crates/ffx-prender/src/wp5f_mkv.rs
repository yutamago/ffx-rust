//! M5 finish, WP5 part 5 (wp5f): libwebm's `mkvparser` (the movie
//! demuxer `FFX.exe` links; `PVideoPlaybackWin32_DemuxLoop` calls it):
//! the entry points the ported demux loop calls, read from the machine
//! code. `_wassert` reports go through its import slot as the originals'
//! (the asserts stay on in this build).

pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    fn at(p: u32, o: u32) -> u32 {
        p.wrapping_add(o)
    }

    /// `_wassert(msg, file, line)`'s import slot and the source file's
    /// name (`mkvparser.cpp`).
    pub const WASSERT: u32 = 0x00b0_c5c0;
    pub const FILE: u32 = 0x00b9_02f0;

    fn wassert(h: &mut impl Host, msg: u32, line: u32) {
        let f: u32 = h.read(h.rebase(WASSERT));
        let (m, file) = (h.rebase(msg), h.rebase(FILE));
        h.call(f, Conv::Cdecl, &[m, file, line]);
    }

    /// `mkvparser::Cluster::Parse(cluster; &pos, &len)` (thiscall, `ret 8`).
    pub const CLUSTER_PARSE: u32 = 0x0042_a550;

    /// `mkvparser_Segment_GetFirst(segment)` (thiscall): the first loaded
    /// cluster (`+0x80`, count `+0x84`), else the segment's end-of-stream
    /// cluster (`+0x20`); a null first entry is reported (`_wassert`).
    pub fn segment_get_first(h: &mut impl Host, seg: u32) -> u32 {
        let clusters: u32 = h.read(at(seg, 0x80));
        if clusters == 0 || h.read::<i32>(at(seg, 0x84)) <= 0 {
            return at(seg, 0x20);
        }
        let c: u32 = h.read(clusters);
        if c == 0 {
            wassert(h, 0x00b9_0694, 0xb7e);
        }
        c
    }

    /// `mkvparser_Tracks_GetTrackByNumber(tracks; n)` (thiscall, `ret 4`):
    /// the track (list `+0x28..+0x2c`) whose number (`+0x24`) is `n`, or
    /// 0 (also for a negative `n`).
    pub fn tracks_get_track_by_number(h: &mut impl Host, tracks: u32, n: u32) -> u32 {
        if n.cast_signed() < 0 {
            return 0;
        }
        let mut p: u32 = h.read(at(tracks, 0x28));
        let end: u32 = h.read(at(tracks, 0x2c));
        while p != end {
            let t: u32 = h.read(p);
            p = at(p, 4);
            if t != 0 && n == h.read::<u32>(at(t, 0x24)) {
                return t;
            }
        }
        0
    }

    /// `mkvparser_BlockEntry_EOS(entry)` (thiscall): 1 when its kind
    /// (`GetKind`, vtable slot 2) is 0 (`kBlockEOS`), else 0.
    pub fn block_entry_eos(h: &mut impl Host, entry: u32) -> u32 {
        let vt: u32 = h.read(entry);
        let f: u32 = h.read(at(vt, 8));
        u32::from(h.call(f, Conv::Thiscall(entry), &[]) == 0)
    }

    /// `mkvparser_Cluster_GetFirst(cluster; &out)` (thiscall, `ret 4`):
    /// without entries (`+0x38`, signed) the cluster is parsed first
    /// ([`CLUSTER_PARSE`] into `pos`, 8 bytes, and `len`, the original's
    /// `ebp - 0x10` and `ebp - 4`): an error (negative) gives `*out` 0 and
    /// the error; still none gives `*out` 0 and 0. Else `*out` is the first
    /// entry (`[+0x30]`; a null array or entry is reported) and 0.
    pub fn cluster_get_first(h: &mut impl Host, cl: u32, out: u32, pos: u32, len: u32) -> u32 {
        if h.read::<i32>(at(cl, 0x38)) <= 0 {
            let r = h.call_sym(CLUSTER_PARSE, Conv::Thiscall(cl), &[pos, len]);
            if r.cast_signed() < 0 {
                h.write(out, 0u32);
                return r;
            }
            if h.read::<i32>(at(cl, 0x38)) <= 0 {
                h.write(out, 0u32);
                return 0;
            }
        }
        if h.read::<u32>(at(cl, 0x30)) == 0 {
            wassert(h, 0x00b9_1b88, 0x1d25);
        }
        let entries: u32 = h.read(at(cl, 0x30));
        let e: u32 = h.read(entries);
        h.write(out, e);
        if e == 0 {
            wassert(h, 0x00b9_1d94, 0x1d28);
        }
        0
    }

    /// `mkvparser_Cluster_GetNext(cluster; cur, &out)` (thiscall, `ret 8`):
    /// the entry after `cur` (its index `+8`, checked against the array:
    /// `_wassert` reports a null `cur`, a missing array, no entries, an
    /// index out of range and a mismatch); past the loaded entries the
    /// cluster is parsed on ([`CLUSTER_PARSE`] into `pos`, the original's
    /// 8 bytes at `ebp - 8`, and `len`, its `cur` slot): an error gives
    /// `*out` 0 and the error, a positive result (the end) `*out` 0 and 0.
    /// Then `*out` is the next entry (a null one reported) and 0.
    pub fn cluster_get_next(
        h: &mut impl Host,
        cl: u32,
        cur: u32,
        out: u32,
        pos: u32,
        len: u32,
    ) -> u32 {
        if cur == 0 {
            wassert(h, 0x00b9_1014, 0x1d4d);
        }
        if h.read::<u32>(at(cl, 0x30)) == 0 {
            wassert(h, 0x00b9_1b88, 0x1d4e);
        }
        if h.read::<i32>(at(cl, 0x38)) <= 0 {
            wassert(h, 0x00b9_1da4, 0x1d4f);
        }
        let mut idx: u32 = h.read(at(cur, 8));
        if idx >= h.read::<u32>(at(cl, 0x38)) {
            wassert(h, 0x00b9_1dcc, 0x1d52);
        }
        let entries: u32 = h.read(at(cl, 0x30));
        if h.read::<u32>(at(entries, idx.wrapping_mul(4))) != cur {
            wassert(h, 0x00b9_1e08, 0x1d53);
        }
        idx = idx.wrapping_add(1);
        if idx >= h.read::<u32>(at(cl, 0x38)) {
            let r = h.call_sym(CLUSTER_PARSE, Conv::Thiscall(cl), &[pos, len]);
            if r.cast_signed() < 0 {
                h.write(out, 0u32);
                return r;
            }
            if r != 0 {
                h.write(out, 0u32);
                return 0;
            }
            if h.read::<u32>(at(cl, 0x30)) == 0 {
                wassert(h, 0x00b9_1b88, 0x1d67);
            }
            if h.read::<i32>(at(cl, 0x38)) <= 0 {
                wassert(h, 0x00b9_1da4, 0x1d68);
            }
            if idx >= h.read::<u32>(at(cl, 0x38)) {
                wassert(h, 0x00b9_1dcc, 0x1d69);
            }
        }
        let entries: u32 = h.read(at(cl, 0x30));
        let e: u32 = h.read(at(entries, idx.wrapping_mul(4)));
        h.write(out, e);
        if e == 0 {
            wassert(h, 0x00b9_0dd4, 0x1d6d);
        }
        0
    }

    /// `IMkvReader::Read(pos, len, buf)` (vtable slot 0, thiscall, 4
    /// words: `pos` 64-bit) -> status (0 read, negative error, positive
    /// past the available data).
    fn read(h: &mut impl Host, reader: u32, pos: u64, len: u32, buf: u32) -> u32 {
        let vt: u32 = h.read(reader);
        let f: u32 = h.read(vt);
        h.call(
            f,
            Conv::Thiscall(reader),
            &[pos as u32, (pos >> 32) as u32, len, buf],
        )
    }

    /// `IMkvReader::Length(&total, &available)` (slot 1).
    fn length(h: &mut impl Host, reader: u32, total: u32, avail: u32) -> u32 {
        let vt: u32 = h.read(reader);
        let f: u32 = h.read(at(vt, 4));
        h.call(f, Conv::Thiscall(reader), &[total, avail])
    }

    fn sext(v: u32) -> u64 {
        i64::from(v.cast_signed()).cast_unsigned()
    }

    fn read_i64(h: &impl Host, p: u32) -> i64 {
        let lo: u32 = h.read(p);
        let hi: u32 = h.read(at(p, 4));
        (u64::from(hi) << 32 | u64::from(lo)).cast_signed()
    }

    /// The asserts every reader helper starts with: a null reader, a
    /// negative position.
    fn check_reader(h: &mut impl Host, reader: u32, pos: u64, line: u32) {
        if reader == 0 {
            wassert(h, 0x00b9_033c, line);
        }
        if pos.cast_signed() < 0 {
            wassert(h, 0x00b9_034c, line + 1);
        }
    }

    /// The length of an EBML varint from its first byte: 1 plus the
    /// leading zero bits; and the marker bit.
    fn varint_len(b: u8) -> (u32, u8) {
        let mut mark = 0x80u8;
        let mut len = 1;
        while b & mark == 0 {
            len += 1;
            mark >>= 1;
        }
        (len, mark)
    }

    /// `mkvparser::ReadUInt(reader, pos, &len)` (`0x0041e710`, fastcall:
    /// `ecx` the reader, `edx` `&len`, `pos` on the stack, the caller
    /// pops) -> the varint at `pos` (its marker bit cleared), `*len` its
    /// size; -2 for a 0 first byte, -3 when a read is past the data
    /// (`*len` 1), a read error as it is (`*len` 1). `buf` is the
    /// original's byte at `ebp - 1`.
    pub fn read_uint(h: &mut impl Host, reader: u32, len: u32, pos: u64, buf: u32) -> u64 {
        check_reader(h, reader, pos, 0x1e);
        h.write(len, 1u32);
        let s = read(h, reader, pos, 1, buf);
        if s.cast_signed() < 0 {
            return sext(s);
        }
        if s != 0 {
            return (-3i64).cast_unsigned();
        }
        let b: u8 = h.read(buf);
        if b == 0 {
            return (-2i64).cast_unsigned();
        }
        let (n, mark) = varint_len(b);
        h.write(len, n);
        let mut v = u64::from(b & !mark);
        let mut p = pos.wrapping_add(1);
        let mut i = 1u32;
        while i.cast_signed() < h.read::<i32>(len) {
            let s = read(h, reader, p, 1, buf);
            if s.cast_signed() < 0 {
                h.write(len, 1u32);
                return sext(s);
            }
            if s != 0 {
                h.write(len, 1u32);
                return (-3i64).cast_unsigned();
            }
            v = v << 8 | u64::from(h.read::<u8>(buf));
            p = p.wrapping_add(1);
            i += 1;
        }
        v
    }

    /// `mkvparser::GetUIntLength(reader, pos, &len)` (`0x0041e850`,
    /// fastcall as [`read_uint`]) -> 0 with `*len` the varint's size at
    /// `pos`; `pos` itself when it is past the available data (`Length`
    /// into `frame`: total +0, available +8, the original's `ebp - 0x18`;
    /// a bad status or `available > total` reported); -2 for a 0 byte; a
    /// read error as it is (a positive status reported). `frame + 0x17`
    /// is the byte read (`ebp - 1`).
    pub fn get_uint_length(h: &mut impl Host, reader: u32, len: u32, pos: u64, frame: u32) -> u64 {
        check_reader(h, reader, pos, 0x61);
        let (total, avail) = (frame, at(frame, 8));
        if length(h, reader, total, avail).cast_signed() < 0 {
            wassert(h, 0x00b9_0360, 0x67);
        }
        let t = read_i64(h, total);
        let a = read_i64(h, avail);
        if t >= 0 && a > t {
            wassert(h, 0x00b9_0378, 0x68);
        }
        h.write(len, 1u32);
        if pos.cast_signed() >= a {
            return pos;
        }
        let buf = at(frame, 0x17);
        let s = read(h, reader, pos, 1, buf);
        if s.cast_signed() < 0 {
            return sext(s);
        }
        if s != 0 {
            wassert(h, 0x00b9_03c0, 0x76);
        }
        let b: u8 = h.read(buf);
        if b == 0 {
            return (-2i64).cast_unsigned();
        }
        h.write(len, varint_len(b).0);
        0
    }

    /// `mkvparser::UnserializeUInt(reader, pos, size)` (`0x0041e980`,
    /// fastcall: `ecx` the reader, `pos` and `size` 64-bit on the stack,
    /// the caller pops) -> the big-endian unsigned value of `size` (1..8)
    /// bytes at `pos`, -2 for another size, a read error as it is. The
    /// original zeroes its `pos` slot and reads each byte into its top
    /// byte (the caller pops the slots and never reads them again);
    /// `slots` is an 8-byte scratch area of the port's for it.
    pub fn unserialize_uint(
        h: &mut impl Host,
        reader: u32,
        pos: u64,
        size: u64,
        slots: u32,
    ) -> u64 {
        check_reader(h, reader, pos, 0x87);
        if size.wrapping_sub(1) > 7 {
            return (-2i64).cast_unsigned();
        }
        h.write(slots, 0u32);
        h.write(at(slots, 4), 0u32);
        let buf = at(slots, 7);
        let mut v = 0u64;
        let mut p = pos;
        let mut i = 0u64;
        while i.cast_signed() < size.cast_signed() {
            let s = read(h, reader, p, 1, buf);
            if s.cast_signed() < 0 {
                return sext(s);
            }
            v = v << 8 | u64::from(h.read::<u8>(buf));
            p = p.wrapping_add(1);
            i += 1;
        }
        v
    }

    /// `mkvparser::UnserializeInt(reader, size, pos, &result)`
    /// (`0x0041ec10`, fastcall: `ecx` the reader, `edx` the size, `pos`
    /// 64-bit and `&result` on the stack, the caller pops) -> 0 with the
    /// big-endian signed value (the first byte sign-extended) in
    /// `*result` (64-bit), or a read error. Sizes outside 1..8 are
    /// reported. The byte is read into the top byte of the original's
    /// `pos` slot; `slots` is an 8-byte scratch area of the port's for it.
    pub fn unserialize_int(
        h: &mut impl Host,
        reader: u32,
        size: u32,
        pos: u64,
        result: u32,
        slots: u32,
    ) -> u32 {
        check_reader(h, reader, pos, 0xde);
        if size.cast_signed() <= 0 {
            wassert(h, 0x00b9_03ec, 0xe0);
        }
        if size.cast_signed() > 8 {
            wassert(h, 0x00b9_0400, 0xe1);
        }
        // the slot's top byte is `pos`'s until a read lands there (a
        // positive status writes nothing)
        let buf = at(slots, 7);
        h.write(buf, (pos >> 56) as u8);
        let s = read(h, reader, pos, 1, buf);
        if s.cast_signed() < 0 {
            return s;
        }
        let first = i64::from(h.read::<i8>(buf)).cast_unsigned();
        h.write(result, first as u32);
        h.write(at(result, 4), (first >> 32) as u32);
        let mut p = pos.wrapping_add(1);
        let mut i = 1u32;
        while i.cast_signed() < size.cast_signed() {
            let s = read(h, reader, p, 1, buf);
            if s.cast_signed() < 0 {
                return s;
            }
            let lo: u32 = h.read(result);
            let hi: u32 = h.read(at(result, 4));
            let v = (u64::from(hi) << 32 | u64::from(lo)) << 8 | u64::from(h.read::<u8>(buf));
            h.write(result, v as u32);
            h.write(at(result, 4), (v >> 32) as u32);
            p = p.wrapping_add(1);
            i += 1;
        }
        0
    }
}
