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

    /// The movie allocator (`mkvparser_SetAllocator`'s `malloc`, cdecl
    /// `(size)`), `ReadUInt`, `UnserializeInt` and `__alldvrm`.
    pub const ALLOCATOR: u32 = 0x022f_b510;
    pub const READ_UINT: u32 = 0x0041_e710;
    pub const UNSERIALIZE_INT: u32 = 0x0041_ec10;

    fn alloc(h: &mut impl Host, n: u32) -> u32 {
        let f: u32 = h.read(h.rebase(ALLOCATOR));
        h.call(f, Conv::Cdecl, &[n])
    }

    /// `ReadUInt(reader, pos, &len)`: [`read_uint`] itself (the original's
    /// callee; its custom convention has no `Host` call), its byte in
    /// `len + 0x12` of the caller's frame.
    fn read_uint_at(h: &mut impl Host, reader: u32, len: u32, pos: u64) -> i64 {
        read_uint(h, reader, len, pos, at(len, 0x12)).cast_signed()
    }

    /// `new Frame[n]` as the original computes the size (`n * 16`, all
    /// ones when that overflows).
    fn alloc_frames(h: &mut impl Host, n: u32) -> u32 {
        let bytes = u64::from(n) * 16;
        let size = if bytes > u64::from(u32::MAX) {
            u32::MAX
        } else {
            bytes as u32
        };
        alloc(h, size)
    }

    /// A frame's `len` (`+8`) sign-extended.
    fn frame_len(h: &impl Host, f: u32) -> i64 {
        i64::from(h.read::<i32>(at(f, 8)))
    }

    fn write_i64(h: &mut impl Host, p: u32, v: i64) {
        let v = v.cast_unsigned();
        h.write(p, v as u32);
        h.write(at(p, 4), (v >> 32) as u32);
    }

    /// Whether a 64-bit length fits a frame's `long` the way the original
    /// tests it: negative values pass (they are stored as they are), else
    /// at most `0x7fffffff`.
    fn fits_long(v: i64) -> bool {
        v < 0 || v <= 0x7fff_ffff
    }

    /// Sets the frames' positions in turn from `pos`; the end position.
    fn place_frames(
        h: &mut impl Host,
        frames: u32,
        end: u32,
        mut pos: i64,
        stop: i64,
        line: u32,
    ) -> i64 {
        let mut f = frames;
        while f != end {
            let len = frame_len(h, f);
            if pos.wrapping_add(len) > stop {
                wassert(h, 0x00b9_1f48, line);
            }
            write_i64(h, f, pos);
            pos = pos.wrapping_add(frame_len(h, f));
            f = at(f, 0x10);
        }
        pos
    }

    /// `mkvparser::Block::Parse(block; cluster)` (`0x0042d470`, thiscall,
    /// `ret 4`) -> 0, -1 without a cluster or segment, -2 for a bad block.
    /// The block (`+0` start, `+8` size, `+0x10` track, `+0x18` time code
    /// (`i16`), `+0x1a` flags, `+0x1c` frames, `+0x20` count) is read from
    /// the segment's reader: the track number (`ReadUInt`), the time code
    /// (`UnserializeInt`, 2 bytes, a `short`), the flags byte; then the
    /// frames of its lacing: none (one frame to the end), Xiph (sizes as
    /// runs of 0xff bytes), fixed (the rest split evenly, `__alldvrm`) or
    /// EBML (a varint, then signed varint differences); every frame's
    /// position follows. The original's asserts are reported as it does.
    /// `frame` (32 bytes) holds the original's locals the callees write:
    /// the varint length (+0, `ebp - 0x28`), the time code (+8, 8 bytes,
    /// `ebp - 0x40`), the lace byte (+0x10, `ebp - 1`), the count byte
    /// (+0x11, the top byte of its `cluster` slot), and the callees' own
    /// bytes (+0x12, +0x18..0x20).
    #[allow(clippy::too_many_lines)] // the four lacings, as the original
    pub fn block_parse(h: &mut impl Host, block: u32, cluster: u32, frame: u32) -> u32 {
        const BAD: u32 = (-2i32).cast_unsigned();
        if cluster == 0 || h.read::<u32>(cluster) == 0 {
            return u32::MAX;
        }
        if read_i64(h, block) < 0 {
            wassert(h, 0x00b9_1e98, 0x1f20);
        }
        if read_i64(h, at(block, 8)) < 0 {
            wassert(h, 0x00b9_111c, 0x1f21);
        }
        if read_i64(h, at(block, 0x10)) > 0 {
            wassert(h, 0x00b9_1eb4, 0x1f22);
        }
        if h.read::<u32>(at(block, 0x1c)) != 0 {
            wassert(h, 0x00b9_1ed0, 0x1f23);
        }
        if h.read::<i32>(at(block, 0x20)) > 0 {
            wassert(h, 0x00b9_1ef4, 0x1f24);
        }
        let start = read_i64(h, block);
        let stop = start.wrapping_add(read_i64(h, at(block, 8)));
        let segment: u32 = h.read(cluster);
        let reader: u32 = h.read(segment);
        let lenp = frame;
        let track = read_uint_at(h, reader, lenp, start.cast_unsigned());
        write_i64(h, at(block, 0x10), track);
        if track <= 0 {
            return BAD;
        }
        let mut pos = start.wrapping_add(i64::from(h.read::<i32>(lenp)));
        if pos > stop || stop.wrapping_sub(pos) < 2 {
            return BAD;
        }
        let tc = at(frame, 8);
        // `UnserializeInt` ([`unserialize_int`] itself, as `ReadUInt`)
        if unserialize_int(h, reader, 2, pos.cast_unsigned(), tc, at(frame, 0x18)) != 0 {
            return BAD;
        }
        let value = read_i64(h, tc);
        if !(-0x8000..=0x7fff).contains(&value) {
            return BAD;
        }
        pos = pos.wrapping_add(2);
        h.write(at(block, 0x18), value as u16);
        if stop.wrapping_sub(pos) <= 0 {
            return BAD;
        }
        let flags = at(block, 0x1a);
        if read(h, reader, pos.cast_unsigned(), 1, flags) != 0 {
            return BAD;
        }
        let lacing = (h.read::<u8>(flags) >> 1) & 3;
        pos = pos.wrapping_add(1);
        if lacing == 0 {
            if pos > stop {
                return BAD;
            }
            h.write(at(block, 0x20), 1u32);
            let f = alloc_frames(h, 1);
            h.write(at(block, 0x1c), f);
            write_i64(h, f, pos);
            let size = stop.wrapping_sub(pos);
            if !fits_long(size) {
                return BAD;
            }
            h.write(at(f, 8), size as u32);
            return 0;
        }
        if pos >= stop {
            return BAD;
        }
        let count_byte = at(frame, 0x11);
        if read(h, reader, pos.cast_unsigned(), 1, count_byte) != 0 {
            return BAD;
        }
        pos = pos.wrapping_add(1);
        if pos > stop {
            wassert(h, 0x00b9_0958, 0x1f77);
        }
        let count = u32::from(h.read::<u8>(count_byte)) + 1;
        h.write(at(block, 0x20), count);
        let frames = alloc_frames(h, count);
        h.write(at(block, 0x1c), frames);
        if frames == 0 {
            wassert(h, 0x00b9_1f1c, 0x1f7c);
        }
        match lacing {
            1 => block_xiph(h, block, reader, pos, stop, at(frame, 0x10)),
            2 => block_fixed(h, block, pos, stop),
            _ => block_ebml(h, block, reader, pos, stop, lenp),
        }
    }

    /// Xiph lacing (`Block::Parse`, lacing 1).
    fn block_xiph(
        h: &mut impl Host,
        block: u32,
        reader: u32,
        mut pos: i64,
        stop: i64,
        byte: u32,
    ) -> u32 {
        const BAD: u32 = (-2i32).cast_unsigned();
        let n: u32 = h.read(at(block, 0x20));
        let frames: u32 = h.read(at(block, 0x1c));
        let end = at(frames, n << 4);
        let mut pf = frames;
        let mut total = 0u32;
        let mut left = n.cast_signed();
        while left > 1 {
            let mut size = 0u32;
            loop {
                if pos >= stop {
                    return BAD;
                }
                if read(h, reader, pos.cast_unsigned(), 1, byte) != 0 {
                    return BAD;
                }
                pos = pos.wrapping_add(1);
                let b: u8 = h.read(byte);
                size = size.wrapping_add(u32::from(b));
                if b != 0xff {
                    break;
                }
            }
            let f = pf;
            pf = at(pf, 0x10);
            if pf >= end {
                wassert(h, 0x00b9_1f30, 0x1f9c);
            }
            total = total.wrapping_add(size);
            h.write(f, 0u32);
            h.write(at(f, 4), 0u32);
            h.write(at(f, 8), size);
            left -= 1;
        }
        if pf >= end {
            wassert(h, 0x00b9_1f30, 0x1fa6);
        }
        if pos > stop {
            wassert(h, 0x00b9_0958, 0x1fa7);
        }
        if at(pf, 0x10) != end {
            return BAD;
        }
        h.write(pf, 0u32);
        h.write(at(pf, 4), 0u32);
        let rest = stop.wrapping_sub(pos);
        let total = i64::from(total.cast_signed());
        if rest < total {
            return BAD;
        }
        let last = rest.wrapping_sub(total);
        if !fits_long(last) {
            return BAD;
        }
        h.write(at(pf, 8), last as u32);
        let pos = place_frames(h, frames, end, pos, stop, 0x1fc1);
        if pos != stop {
            wassert(h, 0x00b9_0970, 0x1fc7);
        }
        0
    }

    /// `__alldvrm` (`0x00aa5200`): 64-bit signed division with the
    /// remainder (quotient `edx:eax`, remainder `ecx:ebx`).
    pub const ALLDVRM: u32 = 0x00aa_5200;

    /// Fixed-size lacing (`Block::Parse`, lacing 2): the rest split into
    /// `count` frames (64-bit signed division; a remainder or a frame size
    /// past a `long` gives -2).
    fn block_fixed(h: &mut impl Host, block: u32, pos: i64, stop: i64) -> u32 {
        const BAD: u32 = (-2i32).cast_unsigned();
        let n: u32 = h.read(at(block, 0x20));
        let rest = stop.wrapping_sub(pos);
        let d = i64::from(n.cast_signed());
        // the original's `__alldvrm`; a zero count cannot occur (count
        // byte + 1)
        let (Some(q), Some(r)) = (rest.checked_div(d), rest.checked_rem(d)) else {
            return BAD;
        };
        if r != 0 || !fits_long(q) {
            return BAD;
        }
        let frames: u32 = h.read(at(block, 0x1c));
        let end = at(frames, n << 4);
        let mut p = pos;
        let mut f = frames;
        while f != end {
            let next = p.wrapping_add(q);
            if next > stop {
                wassert(h, 0x00b9_1f74, 0x1fd7);
            }
            write_i64(h, f, p);
            h.write(at(f, 8), q as u32);
            p = next;
            f = at(f, 0x10);
        }
        if p != stop {
            wassert(h, 0x00b9_0970, 0x1fe1);
        }
        0
    }

    /// EBML lacing (`Block::Parse`, lacing 3): the first size a varint,
    /// each next one the previous plus a signed varint difference (biased
    /// by `2^(7 len - 1) - 1`), the last frame the rest.
    #[allow(clippy::too_many_lines)] // the original's checks
    fn block_ebml(
        h: &mut impl Host,
        block: u32,
        reader: u32,
        pos: i64,
        stop: i64,
        lenp: u32,
    ) -> u32 {
        const BAD: u32 = (-2i32).cast_unsigned();
        let n: u32 = h.read(at(block, 0x20));
        if pos >= stop {
            return BAD;
        }
        let first = read_uint_at(h, reader, lenp, pos.cast_unsigned());
        if !(0..=0x7fff_ffff).contains(&first) {
            return BAD;
        }
        let mut pos = pos.wrapping_add(i64::from(h.read::<i32>(lenp)));
        if pos > stop || first.wrapping_add(pos) > stop {
            return BAD;
        }
        let frames: u32 = h.read(at(block, 0x1c));
        let end = at(frames, n << 4);
        h.write(at(frames, 8), first as u32);
        h.write(frames, 0u32);
        h.write(at(frames, 4), 0u32);
        let mut pf = frames;
        let mut size = first;
        let mut total = first as u32;
        let mut left = n.cast_signed() - 1;
        while left > 1 {
            if pos >= stop {
                return BAD;
            }
            if pf >= end {
                wassert(h, 0x00b9_1f30, 0x200d);
            }
            let prev = pf;
            pf = at(pf, 0x10);
            if frame_len(h, prev) != size {
                wassert(h, 0x00b9_1fc4, 0x2010);
            }
            if frame_len(h, prev) != size {
                return BAD;
            }
            if pf >= end {
                wassert(h, 0x00b9_1f30, 0x2014);
            }
            h.write(pf, 0u32);
            h.write(at(pf, 4), 0u32);
            let delta = read_uint_at(h, reader, lenp, pos.cast_unsigned());
            if delta < 0 {
                return BAD;
            }
            let len = h.read::<i32>(lenp);
            pos = pos.wrapping_add(i64::from(len));
            if pos > stop {
                return BAD;
            }
            // the bias 2^(7 len - 1) - 1 as the original builds it
            // (`bts` into the low word, moved up at 32, gone at 64)
            let bit = (len.wrapping_mul(7).wrapping_sub(1)).cast_unsigned();
            let low = 1u32 << (bit & 31);
            let (lo, hi) = if bit >= 64 {
                (0, 0)
            } else if bit >= 32 {
                (0, low)
            } else {
                (low, 0)
            };
            let bias = (u64::from(hi) << 32 | u64::from(lo)).cast_signed();
            size = size.wrapping_add(delta.wrapping_sub(bias).wrapping_add(1));
            if !(0..=0x7fff_ffff).contains(&size) {
                return BAD;
            }
            total = total.wrapping_add(size as u32);
            h.write(at(pf, 8), size as u32);
            left -= 1;
        }
        if pos > stop {
            wassert(h, 0x00b9_0958, 0x2038);
        }
        if pf >= end {
            wassert(h, 0x00b9_1f30, 0x2039);
        }
        let prev = pf;
        pf = at(pf, 0x10);
        if frame_len(h, prev) != size {
            wassert(h, 0x00b9_1fc4, 0x203c);
        }
        if frame_len(h, prev) != size {
            return BAD;
        }
        if pf >= end {
            wassert(h, 0x00b9_1f30, 0x2040);
        }
        if at(pf, 0x10) != end {
            wassert(h, 0x00b9_1ff4, 0x2043);
        }
        h.write(pf, 0u32);
        h.write(at(pf, 4), 0u32);
        let rest = stop.wrapping_sub(pos);
        let total = i64::from(total.cast_signed());
        if rest < total {
            return BAD;
        }
        let last = rest.wrapping_sub(total);
        if !fits_long(last) {
            return BAD;
        }
        h.write(at(pf, 8), last as u32);
        let pos = place_frames(h, frames, end, pos, stop, 0x2057);
        if pos != stop {
            wassert(h, 0x00b9_0970, 0x205d);
        }
        0
    }

    /// `mkvparser::Cluster::Load(cluster; &pos, &len)` (thiscall, `ret 8`),
    /// `ParseBlockGroup(cluster; size, &pos, &len)` and
    /// `ParseSimpleBlock(cluster; size, &pos, &len)` (thiscall, `ret 0x10`;
    /// `size` 64-bit).
    pub const CLUSTER_LOAD: u32 = 0x0042_9e10;
    pub const PARSE_BLOCK_GROUP: u32 = 0x0042_af10;
    pub const PARSE_SIMPLE_BLOCK: u32 = 0x0042_aca0;
    pub const GET_UINT_LENGTH: u32 = 0x0041_e850;

    /// EBML's "unknown size" for a varint of `len` bytes: `2^(7 len) - 1`
    /// as the original builds it (the bit set in the low word, moved up
    /// at 32, gone at 64; then minus one).
    fn unknown_size(len: i32) -> i64 {
        let bit = len.wrapping_mul(7).cast_unsigned();
        let low = 1u32 << (bit & 31);
        let (lo, hi) = if bit >= 64 {
            (low, 0)
        } else if bit >= 32 {
            (0, low)
        } else {
            (low, 0)
        };
        // at 64 and up the original's xor leaves the low word 0 too
        let lo = if bit >= 64 { 0 } else { lo };
        (u64::from(hi) << 32 | u64::from(lo))
            .wrapping_sub(1)
            .cast_signed()
    }

    /// `GetUIntLength(reader, pos, &len)`: [`get_uint_length`] itself, its
    /// frame at `scratch`.
    fn uint_length_at(h: &mut impl Host, reader: u32, len: u32, pos: u64, scratch: u32) -> i64 {
        get_uint_length(h, reader, len, pos, scratch).cast_signed()
    }

    /// The id or size varint at `*pos`, as `Cluster::Parse` reads each:
    /// -3 with `*len` 1 when its first byte is not available, the length
    /// (`GetUIntLength`: an error as it is, a nonzero result -3), -2 past
    /// the cluster, -3 past the available data; then `ReadUInt` (an error
    /// as its low word). `Ok(value)` or the status to return.
    fn parse_varint(
        h: &mut impl Host,
        reader: u32,
        pos: u32,
        len: u32,
        cluster_stop: i64,
        avail: i64,
        scratch: u32,
    ) -> Result<i64, u32> {
        parse_varint_or(h, reader, pos, len, cluster_stop, avail, scratch, None)
    }

    /// [`parse_varint`] with `read_err` (when given) returned for a
    /// negative `ReadUInt` result instead of its low word.
    #[allow(clippy::too_many_arguments)] // the parser's state, as the original's locals
    fn parse_varint_or(
        h: &mut impl Host,
        reader: u32,
        pos: u32,
        len: u32,
        cluster_stop: i64,
        avail: i64,
        scratch: u32,
        read_err: Option<u32>,
    ) -> Result<i64, u32> {
        let p = read_i64(h, pos);
        if p.wrapping_add(1) > avail {
            h.write(len, 1u32);
            return Err((-3i32).cast_unsigned());
        }
        let r = uint_length_at(h, reader, len, p.cast_unsigned(), scratch);
        if r < 0 {
            return Err(r as u32);
        }
        if r != 0 {
            return Err((-3i32).cast_unsigned());
        }
        let p = read_i64(h, pos);
        let end = p.wrapping_add(i64::from(h.read::<i32>(len)));
        if cluster_stop >= 0 && end > cluster_stop {
            return Err((-2i32).cast_unsigned());
        }
        if end > avail {
            return Err((-3i32).cast_unsigned());
        }
        let v = read_uint(h, reader, len, p.cast_unsigned(), at(scratch, 0x18));
        let v = v.cast_signed();
        if v < 0 {
            return Err(read_err.unwrap_or(v as u32));
        }
        Ok(v)
    }

    /// `mkvparser::Cluster::Parse(cluster; &pos, &len)` (`0x0042a550`,
    /// thiscall, `ret 8`): 1 at the cluster's end, 0 after one more block
    /// entry, -1 when the last entry's block lies past the file, -2 for a
    /// bad element, -3 (with `*len` the bytes wanted) when the data is not
    /// there yet; a negative `Load` or reader status as it is. After
    /// `Load` ([`CLUSTER_LOAD`]) the elements from `m_pos` (`+0x18`) are
    /// walked to the cluster's stop (`+8` start plus `+0x20` size when
    /// known): each id and size read (`GetUIntLength`, `ReadUInt`), a
    /// `BlockGroup` (0x20) or `SimpleBlock` (0x23) handed to
    /// [`PARSE_BLOCK_GROUP`] / [`PARSE_SIMPLE_BLOCK`] (their status
    /// returned), anything else skipped; a `Cluster` or `Cues` id, or the
    /// file's end, ends an unknown-size cluster there (its size set). At
    /// the end `m_pos` is the position and the last entry's block is
    /// checked against the file's size. `frame` holds the original's
    /// `Length` results (+0 total, +8 available: `esp + 0x30` of its
    /// aligned frame) and the readers' scratch (+0x10..+0x30).
    #[allow(clippy::too_many_lines)] // the original's loop
    pub fn cluster_parse(h: &mut impl Host, this: u32, pos: u32, len: u32, frame: u32) -> u32 {
        const BAD: u32 = (-2i32).cast_unsigned();
        const MORE: u32 = (-3i32).cast_unsigned();
        let r = h.call_sym(CLUSTER_LOAD, Conv::Thiscall(this), &[pos, len]);
        if r.cast_signed() < 0 {
            return r;
        }
        if read_i64(h, at(this, 0x18)) < read_i64(h, at(this, 8)) {
            wassert(h, 0x00b9_1904, 0x1885);
        }
        if read_i64(h, at(this, 0x28)) < 0 {
            wassert(h, 0x00b9_0ee0, 0x1886);
        }
        let size = read_i64(h, at(this, 0x20));
        let cluster_stop = if size >= 0 {
            let stop = read_i64(h, at(this, 8)).wrapping_add(size);
            if stop >= 0 && read_i64(h, at(this, 0x18)) >= stop {
                return 1;
            }
            stop
        } else {
            -1
        };
        let segment: u32 = h.read(this);
        let reader: u32 = h.read(segment);
        let (total_at, avail_at) = (frame, at(frame, 8));
        let scratch = at(frame, 0x10);
        let s = length(h, reader, total_at, avail_at);
        if s.cast_signed() < 0 {
            return s;
        }
        let total = read_i64(h, total_at);
        let avail = read_i64(h, avail_at);
        if total >= 0 && avail > total {
            wassert(h, 0x00b9_05f8, 0x1899);
        }
        let m_pos = read_i64(h, at(this, 0x18));
        write_i64(h, pos, m_pos);
        loop {
            let p = read_i64(h, pos);
            if cluster_stop >= 0 && p >= cluster_stop {
                break;
            }
            if total >= 0 && p >= total {
                return cluster_parse_end(h, this, pos, cluster_stop, total, true);
            }
            let id = match parse_varint(h, reader, pos, len, cluster_stop, avail, scratch) {
                Ok(v) => v,
                Err(s) => return s,
            };
            if id == 0 {
                return BAD;
            }
            if id == 0x0f43_b675 || id == 0x0c53_bb6b {
                return cluster_parse_end(h, this, pos, cluster_stop, total, true);
            }
            let p = read_i64(h, pos).wrapping_add(i64::from(h.read::<i32>(len)));
            write_i64(h, pos, p);
            let size = match parse_varint(h, reader, pos, len, cluster_stop, avail, scratch) {
                Ok(v) => v,
                Err(s) => return s,
            };
            if size == unknown_size(h.read::<i32>(len)) {
                return BAD;
            }
            let p = read_i64(h, pos).wrapping_add(i64::from(h.read::<i32>(len)));
            write_i64(h, pos, p);
            if cluster_stop >= 0 && p > cluster_stop {
                return BAD;
            }
            if size == 0 {
                continue;
            }
            let payload_stop = p.wrapping_add(size);
            if cluster_stop < 0 {
                if total >= 0 && payload_stop > total {
                    let start = read_i64(h, at(this, 8));
                    write_i64(h, at(this, 0x20), total.wrapping_sub(start));
                    write_i64(h, pos, total);
                    return cluster_parse_end(h, this, pos, cluster_stop, total, false);
                }
                if payload_stop > avail {
                    h.write(len, size as u32);
                    return MORE;
                }
            } else if payload_stop > cluster_stop {
                if id == 0x20 || id == 0x23 {
                    return BAD;
                }
                write_i64(h, pos, cluster_stop);
                break;
            }
            let su = size.cast_unsigned();
            let args = [su as u32, (su >> 32) as u32, pos, len];
            if id == 0x20 {
                return h.call_sym(PARSE_BLOCK_GROUP, Conv::Thiscall(this), &args);
            }
            if id == 0x23 {
                return h.call_sym(PARSE_SIMPLE_BLOCK, Conv::Thiscall(this), &args);
            }
            write_i64(h, pos, payload_stop);
            if cluster_stop >= 0 && payload_stop > cluster_stop {
                wassert(h, 0x00b9_19d8, 0x1918);
            }
        }
        cluster_parse_end(h, this, pos, cluster_stop, total, false)
    }

    /// `Cluster::Parse`'s end: an unknown element size set from `*pos`
    /// when `size_from_pos` (a `Cluster`/`Cues` id or the file's end),
    /// the size asserted positive, `m_pos = *pos`, then the last entry's
    /// block checked against the file's total.
    fn cluster_parse_end(
        h: &mut impl Host,
        this: u32,
        pos: u32,
        cluster_stop: i64,
        total: i64,
        size_from_pos: bool,
    ) -> u32 {
        let size = read_i64(h, at(this, 0x20));
        if size_from_pos && size > 0 {
            // straight to the m_pos store, no assert
        } else {
            if size_from_pos && size < 0 {
                let p = read_i64(h, pos);
                let start = read_i64(h, at(this, 8));
                write_i64(h, at(this, 0x20), p.wrapping_sub(start));
            }
            if read_i64(h, at(this, 0x20)) <= 0 {
                wassert(h, 0x00b9_1a30, 0x191b);
            }
        }
        let p = read_i64(h, pos);
        write_i64(h, at(this, 0x18), p);
        if cluster_stop >= 0 && read_i64(h, at(this, 0x18)) > cluster_stop {
            wassert(h, 0x00b9_1a58, 0x191e);
        }
        let count: i32 = h.read(at(this, 0x38));
        if count <= 0 {
            return 1;
        }
        let entries: u32 = h.read(at(this, 0x30));
        let last: u32 = h.read(at(entries, (count.cast_unsigned() - 1).wrapping_mul(4)));
        if last == 0 {
            wassert(h, 0x00b9_1ab4, 0x1924);
        }
        let vt: u32 = h.read(last);
        let f: u32 = h.read(at(vt, 4));
        let block = h.call(f, Conv::Thiscall(last), &[]);
        if block == 0 {
            wassert(h, 0x00b9_14ec, 0x1927);
        }
        let start = read_i64(h, block);
        if total >= 0 && start > total {
            return u32::MAX;
        }
        let stop = start.wrapping_add(read_i64(h, at(block, 8)));
        if cluster_stop >= 0 && stop > cluster_stop {
            wassert(h, 0x00b9_1ac0, 0x1931);
        }
        if total >= 0 && stop > total {
            return u32::MAX;
        }
        1
    }

    /// `mkvparser::Cluster::Load(cluster; &pos, &len)` (`0x00429e10`,
    /// thiscall, `ret 8`): 0 once loaded (a time code `+0x28` set);
    /// else, from the element's start, the `Cluster` id and size are read
    /// (`GetUIntLength`, `ReadUInt`: -3 with `*len` 1 or -3 when the data
    /// is not there, -2 for another id or a zero size, -1 for a bad size),
    /// then the children until the first `BlockGroup` (0x20) or
    /// `SimpleBlock` (0x23), a `Cluster` or `Cues` id, or the cluster's
    /// stop: the `Timecode` (0x67) read (`UnserializeUInt`; `*len` its size
    /// first). Without a time code or a block, -2; else `m_pos` is the
    /// time code's end, `m_timecode` set and, for a known size, the
    /// element size; 0. `frame` holds `Length`'s results (+0, +8) and the
    /// readers' scratch (+0x10..+0x30).
    #[allow(clippy::too_many_lines)] // the original's loop
    pub fn cluster_load(h: &mut impl Host, this: u32, pos: u32, len: u32, frame: u32) -> u32 {
        const BAD: u32 = (-2i32).cast_unsigned();
        const MORE: u32 = (-3i32).cast_unsigned();
        if h.read::<u32>(this) == 0 {
            wassert(h, 0x00b9_18ec, 0x178f);
        }
        if read_i64(h, at(this, 0x18)) < read_i64(h, at(this, 8)) {
            wassert(h, 0x00b9_1904, 0x1790);
        }
        if read_i64(h, at(this, 0x28)) >= 0 {
            return 0;
        }
        if read_i64(h, at(this, 0x18)) != read_i64(h, at(this, 8)) {
            wassert(h, 0x00b9_1938, 0x1795);
        }
        if read_i64(h, at(this, 0x20)) >= 0 {
            wassert(h, 0x00b9_196c, 0x1796);
        }
        let segment: u32 = h.read(this);
        let reader: u32 = h.read(segment);
        let (total_at, avail_at) = (frame, at(frame, 8));
        let scratch = at(frame, 0x10);
        let s = length(h, reader, total_at, avail_at);
        if s.cast_signed() < 0 {
            return s;
        }
        let total = read_i64(h, total_at);
        let avail = read_i64(h, avail_at);
        if total >= 0 {
            if avail > total {
                wassert(h, 0x00b9_05f8, 0x17a1);
            }
            if read_i64(h, at(this, 0x18)) > total {
                wassert(h, 0x00b9_1998, 0x17a2);
            }
        }
        let m_pos = read_i64(h, at(this, 0x18));
        write_i64(h, pos, m_pos);
        // the cluster's own id and size: no cluster stop yet
        let id = match parse_varint(h, reader, pos, len, -1, avail, scratch) {
            Ok(v) => v,
            Err(s) => return s,
        };
        if id != 0x0f43_b675 {
            return BAD;
        }
        let p = read_i64(h, pos).wrapping_add(i64::from(h.read::<i32>(len)));
        write_i64(h, pos, p);
        // a bad size gives -1 (the original returns its cluster size, -1)
        let size = match parse_varint_or(h, reader, pos, len, -1, avail, scratch, Some(u32::MAX)) {
            Ok(v) => v,
            Err(s) => return s,
        };
        if size == 0 {
            return BAD;
        }
        let p = read_i64(h, pos).wrapping_add(i64::from(h.read::<i32>(len)));
        write_i64(h, pos, p);
        let cluster_size = if size == unknown_size(h.read::<i32>(len)) {
            -1
        } else {
            size
        };
        let cluster_stop = if cluster_size < 0 {
            -1
        } else {
            p.wrapping_add(cluster_size)
        };
        let mut timecode = -1i64;
        let mut new_pos = -1i64;
        let mut block = false;
        loop {
            let p = read_i64(h, pos);
            if cluster_stop >= 0 && p >= cluster_stop {
                break;
            }
            let id = match parse_varint(h, reader, pos, len, cluster_stop, avail, scratch) {
                Ok(v) => v,
                Err(s) => return s,
            };
            if id == 0 {
                return BAD;
            }
            if id == 0x0f43_b675 || id == 0x0c53_bb6b {
                break;
            }
            let p = read_i64(h, pos).wrapping_add(i64::from(h.read::<i32>(len)));
            write_i64(h, pos, p);
            let size = match parse_varint(h, reader, pos, len, cluster_stop, avail, scratch) {
                Ok(v) => v,
                Err(s) => return s,
            };
            if size == unknown_size(h.read::<i32>(len)) {
                return BAD;
            }
            let p = read_i64(h, pos).wrapping_add(i64::from(h.read::<i32>(len)));
            write_i64(h, pos, p);
            if cluster_stop >= 0 && p > cluster_stop {
                return BAD;
            }
            if size == 0 {
                continue;
            }
            if cluster_stop >= 0 && p.wrapping_add(size) > cluster_stop {
                return BAD;
            }
            if id == 0x67 {
                h.write(len, size as u32);
                if p.wrapping_add(size) > avail {
                    return MORE;
                }
                timecode = unserialize_uint(
                    h,
                    reader,
                    p.cast_unsigned(),
                    size.cast_unsigned(),
                    at(scratch, 0x18),
                )
                .cast_signed();
                if timecode < 0 {
                    return timecode as u32;
                }
                new_pos = p.wrapping_add(size);
            } else if id == 0x20 || id == 0x23 {
                block = true;
                break;
            }
            let p = p.wrapping_add(size);
            write_i64(h, pos, p);
            if cluster_stop >= 0 && p > cluster_stop {
                wassert(h, 0x00b9_19d8, 0x186b);
            }
        }
        if cluster_stop >= 0 && read_i64(h, pos) > cluster_stop {
            wassert(h, 0x00b9_19d8, 0x186e);
        }
        if timecode < 0 || !block {
            return BAD;
        }
        write_i64(h, at(this, 0x18), new_pos);
        write_i64(h, at(this, 0x28), timecode);
        if cluster_size >= 0 {
            let start = read_i64(h, at(this, 8));
            write_i64(h, at(this, 0x20), cluster_stop.wrapping_sub(start));
        }
        0
    }

    /// `mkvparser::Cluster::CreateBlock(cluster; id, start, size, discard)`
    /// (thiscall, `ret 0x20`; four 64-bit words), `CreateBlockGroup(cluster;
    /// start, size, discard)` (`ret 0x18`), the movie allocator's `free`
    /// and the `SimpleBlock` vtable.
    pub const CREATE_BLOCK: u32 = 0x0042_c130;
    pub const CREATE_BLOCK_GROUP: u32 = 0x0042_c300;
    pub const CREATE_SIMPLE_BLOCK: u32 = 0x0042_c740;
    pub const BLOCK_PARSE: u32 = 0x0042_d470;
    pub const DEALLOCATOR: u32 = 0x022f_b514;
    pub const SIMPLE_BLOCK_VTABLE: u32 = 0x00b9_20cc;

    fn free(h: &mut impl Host, p: u32) {
        let f: u32 = h.read(h.rebase(DEALLOCATOR));
        h.call(f, Conv::Cdecl, &[p]);
    }

    fn words64(v: i64) -> [u32; 2] {
        let v = v.cast_unsigned();
        [v as u32, (v >> 32) as u32]
    }

    /// `mkvparser::Cluster::ParseSimpleBlock(cluster; size, &pos, &len)`
    /// (`0x0042aca0`, thiscall, `ret 0x10`; `size` 64-bit): the block's
    /// track number (`GetUIntLength`/`ReadUInt`), time code and flags
    /// checked against the block's stop and the available data (-2 bad,
    /// -3 with `*len` the bytes wanted), a laced block wholly available;
    /// then `CreateBlock(0x23, start, size, 0)` ([`CREATE_BLOCK`], its
    /// error returned) and `m_pos` past the block; 0. `frame`: `Length`'s
    /// results (+0, +8), the readers' scratch (+0x10..+0x30) and the flags
    /// byte (+0x30; the original's is the top byte of its `pos` slot).
    #[allow(clippy::too_many_lines)] // the original's checks
    pub fn parse_simple_block(
        h: &mut impl Host,
        this: u32,
        size: i64,
        pos: u32,
        len: u32,
        frame: u32,
    ) -> u32 {
        const BAD: u32 = (-2i32).cast_unsigned();
        const MORE: u32 = (-3i32).cast_unsigned();
        let start = read_i64(h, pos);
        let stop = start.wrapping_add(size);
        let segment: u32 = h.read(this);
        let reader: u32 = h.read(segment);
        let (total_at, avail_at) = (frame, at(frame, 8));
        let scratch = at(frame, 0x10);
        let s = length(h, reader, total_at, avail_at);
        if s.cast_signed() < 0 {
            return s;
        }
        let total = read_i64(h, total_at);
        let avail = read_i64(h, avail_at);
        if total >= 0 && avail > total {
            wassert(h, 0x00b9_05f8, 0x1948);
        }
        let p = read_i64(h, pos);
        if p.wrapping_add(1) > avail {
            h.write(len, 1u32);
            return MORE;
        }
        let r = uint_length_at(h, reader, len, p.cast_unsigned(), scratch);
        if r < 0 {
            return r as u32;
        }
        if r != 0 {
            return MORE;
        }
        let p = read_i64(h, pos);
        let end = p.wrapping_add(i64::from(h.read::<i32>(len)));
        if end > stop {
            return BAD;
        }
        if end > avail {
            return MORE;
        }
        let track = read_uint(h, reader, len, p.cast_unsigned(), at(scratch, 0x18)).cast_signed();
        if track < 0 {
            return track as u32;
        }
        if track == 0 {
            return BAD;
        }
        let p = read_i64(h, pos).wrapping_add(i64::from(h.read::<i32>(len)));
        write_i64(h, pos, p);
        if p.wrapping_add(2) > stop {
            return BAD;
        }
        if p.wrapping_add(2) > avail {
            h.write(len, 2u32);
            return MORE;
        }
        let p = p.wrapping_add(2);
        write_i64(h, pos, p);
        if p.wrapping_add(1) > stop {
            return BAD;
        }
        if p.wrapping_add(1) > avail {
            h.write(len, 1u32);
            return MORE;
        }
        let flags = at(frame, 0x30);
        let s = read(h, reader, p.cast_unsigned(), 1, flags);
        if s.cast_signed() < 0 {
            h.write(len, 1u32);
            return s;
        }
        let p = p.wrapping_add(1);
        write_i64(h, pos, p);
        if p > avail {
            wassert(h, 0x00b9_1b1c, 0x19a3);
        }
        if p >= stop {
            return BAD;
        }
        if h.read::<u8>(flags) & 6 != 0 && stop > avail {
            h.write(len, stop.wrapping_sub(p) as u32);
            return MORE;
        }
        let [s0, s1] = words64(start);
        let [z0, z1] = words64(size);
        let s = h.call_sym(
            CREATE_BLOCK,
            Conv::Thiscall(this),
            &[0x23, 0, s0, s1, z0, z1, 0, 0],
        );
        if s != 0 {
            return s;
        }
        write_i64(h, at(this, 0x18), stop);
        0
    }

    /// `mkvparser::Cluster::CreateBlock(cluster; id, start, size, discard)`
    /// (`0x0042c130`): the entry array made (0x400 entries, 0x1000 bytes,
    /// from the movie allocator) when the cluster has none (count `+0x38`
    /// negative), else doubled when full (the old entries copied, the old
    /// array freed); then `CreateBlockGroup(start, size, discard)` for a
    /// `BlockGroup` (0x20) or `CreateSimpleBlock(start, size)` (their
    /// results). The ids and the array's state are asserted as the
    /// original does.
    pub fn create_block(
        h: &mut impl Host,
        this: u32,
        id: i64,
        start: i64,
        size: i64,
        discard: i64,
    ) -> u32 {
        if id != 0x20 && id != 0x23 {
            wassert(h, 0x00b9_1c40, 0x1c66);
        }
        if h.read::<i32>(at(this, 0x38)) < 0 {
            if h.read::<u32>(at(this, 0x30)) != 0 {
                wassert(h, 0x00b9_1c7c, 0x1c69);
            }
            if h.read::<u32>(at(this, 0x34)) != 0 {
                wassert(h, 0x00b9_1ca0, 0x1c6a);
            }
            h.write(at(this, 0x34), 0x400u32);
            let a = alloc(h, 0x1000);
            h.write(at(this, 0x30), a);
            h.write(at(this, 0x38), 0u32);
        } else {
            if h.read::<u32>(at(this, 0x30)) == 0 {
                wassert(h, 0x00b9_1b88, 0x1c71);
            }
            if h.read::<i32>(at(this, 0x34)) <= 0 {
                wassert(h, 0x00b9_1b9c, 0x1c72);
            }
            if h.read::<i32>(at(this, 0x38)) > h.read::<i32>(at(this, 0x34)) {
                wassert(h, 0x00b9_1bc8, 0x1c73);
            }
            let cap: i32 = h.read(at(this, 0x34));
            if h.read::<i32>(at(this, 0x38)) >= cap {
                let n = cap.wrapping_add(cap).cast_unsigned();
                let bytes = u64::from(n) * 4;
                let a = alloc(
                    h,
                    if bytes > u64::from(u32::MAX) {
                        u32::MAX
                    } else {
                        bytes as u32
                    },
                );
                if a == 0 {
                    wassert(h, 0x00b9_1cc8, 0x1c79);
                }
                let old: u32 = h.read(at(this, 0x30));
                let count: u32 = h.read(at(this, 0x38));
                for i in 0..count {
                    let e: u32 = h.read(at(old, 4 * i));
                    h.write(at(a, 4 * i), e);
                }
                let old: u32 = h.read(at(this, 0x30));
                if old != 0 {
                    free(h, old);
                }
                h.write(at(this, 0x30), a);
                h.write(at(this, 0x34), n);
            }
        }
        let [s0, s1] = words64(start);
        let [z0, z1] = words64(size);
        if id == 0x20 {
            let [d0, d1] = words64(discard);
            return h.call_sym(
                CREATE_BLOCK_GROUP,
                Conv::Thiscall(this),
                &[s0, s1, z0, z1, d0, d1],
            );
        }
        h.call_sym(CREATE_SIMPLE_BLOCK, Conv::Thiscall(this), &[s0, s1, z0, z1])
    }

    /// `mkvparser::Cluster::CreateSimpleBlock(cluster; start, size)`
    /// (`0x0042c740`, thiscall, `ret 0x10`): a `SimpleBlock` (0x40 bytes
    /// from the movie allocator: vtable, cluster, index, the block at
    /// `+0x10` with its start and size, no track, time code -1, no
    /// frames) in the next entry, its block parsed ([`BLOCK_PARSE`] with
    /// the cluster): on success the count grows and 0; else the entry is
    /// deleted (vtable slot 0, 1) and cleared and the status returned; -1
    /// when the allocation fails.
    pub fn create_simple_block(h: &mut impl Host, this: u32, start: i64, size: i64) -> u32 {
        if h.read::<u32>(at(this, 0x30)) == 0 {
            wassert(h, 0x00b9_1b88, 0x1cf5);
        }
        if h.read::<i32>(at(this, 0x34)) <= 0 {
            wassert(h, 0x00b9_1b9c, 0x1cf6);
        }
        if h.read::<i32>(at(this, 0x38)) < 0 {
            wassert(h, 0x00b9_1cd8, 0x1cf7);
        }
        if h.read::<i32>(at(this, 0x38)) >= h.read::<i32>(at(this, 0x34)) {
            wassert(h, 0x00b9_1d08, 0x1cf8);
        }
        let index: u32 = h.read(at(this, 0x38));
        let entries: u32 = h.read(at(this, 0x30));
        let slot = at(entries, index.wrapping_mul(4));
        let e = alloc(h, 0x40);
        if e != 0 {
            h.write(at(e, 4), this);
            h.write(at(e, 8), index);
            let vt = h.rebase(SIMPLE_BLOCK_VTABLE);
            h.write(e, vt);
            write_i64(h, at(e, 0x10), start);
            write_i64(h, at(e, 0x18), size);
            h.write(at(e, 0x20), 0u32);
            h.write(at(e, 0x24), 0u32);
            h.write(at(e, 0x28), 0xffffu16);
            h.write(at(e, 0x2a), 0u8);
            h.write(at(e, 0x2c), 0u32);
            h.write(at(e, 0x30), u32::MAX);
            h.write(at(e, 0x38), 0u32);
            h.write(at(e, 0x3c), 0u32);
        }
        h.write(slot, e);
        if e == 0 {
            return u32::MAX;
        }
        let cluster: u32 = h.read(at(e, 4));
        let s = h.call_sym(BLOCK_PARSE, Conv::Thiscall(at(e, 0x10)), &[cluster]);
        if s == 0 {
            let n = h.read::<u32>(at(this, 0x38)).wrapping_add(1);
            h.write(at(this, 0x38), n);
            return 0;
        }
        let e: u32 = h.read(slot);
        if e != 0 {
            let vt: u32 = h.read(e);
            let f: u32 = h.read(vt);
            h.call(f, Conv::Thiscall(e), &[1]);
        }
        h.write(slot, 0u32);
        s
    }

    /// `mkvparser::Cluster::ParseBlockGroup(cluster; size, &pos, &len)`
    /// (`0x0042af10`, thiscall, `ret 0x10`; `size` 64-bit): the group
    /// wholly in the file (-2 past the total; -3 with `*len` its size past
    /// the available data); its children walked (id and size varints
    /// against the group's stop: -2 / -3): `DiscardPadding` (0x35a2) read
    /// (`GetUIntLength`, then `UnserializeInt` of that length), a `Block`
    /// (0x21) checked as [`parse_simple_block`] checks one (track, time
    /// code, flags, a laced block wholly available) and skipped, others
    /// skipped; at the group's end `CreateBlock(0x20, start, size,
    /// discard)` ([`CREATE_BLOCK`]) and `m_pos` past the group. `frame`:
    /// `Length`'s results (+0, +8), the readers' scratch (+0x10..+0x30),
    /// the flags byte (+0x30), the discard padding (+0x38, 8 bytes).
    #[allow(clippy::too_many_lines)] // the original's checks
    pub fn parse_block_group(
        h: &mut impl Host,
        this: u32,
        size: i64,
        pos: u32,
        len: u32,
        frame: u32,
    ) -> u32 {
        const BAD: u32 = (-2i32).cast_unsigned();
        const MORE: u32 = (-3i32).cast_unsigned();
        let start = read_i64(h, pos);
        let stop = start.wrapping_add(size);
        let segment: u32 = h.read(this);
        let reader: u32 = h.read(segment);
        let (total_at, avail_at) = (frame, at(frame, 8));
        let scratch = at(frame, 0x10);
        let s = length(h, reader, total_at, avail_at);
        if s.cast_signed() < 0 {
            return s;
        }
        let total = read_i64(h, total_at);
        let avail = read_i64(h, avail_at);
        if total >= 0 {
            if avail > total {
                wassert(h, 0x00b9_05f8, 0x19c9);
            }
            if stop > total {
                return BAD;
            }
        }
        if stop > avail {
            h.write(len, size as u32);
            return MORE;
        }
        let discard = at(frame, 0x38);
        write_i64(h, discard, 0);
        while read_i64(h, pos) < stop {
            let id = match parse_varint(h, reader, pos, len, stop, avail, scratch) {
                Ok(v) => v,
                Err(s) => return s,
            };
            if id == 0 {
                return BAD;
            }
            let p = read_i64(h, pos).wrapping_add(i64::from(h.read::<i32>(len)));
            write_i64(h, pos, p);
            let size = match parse_varint(h, reader, pos, len, stop, avail, scratch) {
                Ok(v) => v,
                Err(s) => return s,
            };
            let p = read_i64(h, pos).wrapping_add(i64::from(h.read::<i32>(len)));
            write_i64(h, pos, p);
            if p > stop {
                return BAD;
            }
            if size == 0 {
                continue;
            }
            if size == unknown_size(h.read::<i32>(len)) {
                return BAD;
            }
            if id == 0x35a2 {
                let r = uint_length_at(h, reader, len, p.cast_unsigned(), scratch);
                if r < 0 {
                    return r as u32;
                }
                let n: u32 = h.read(len);
                let s =
                    unserialize_int(h, reader, n, p.cast_unsigned(), discard, at(scratch, 0x18));
                if s.cast_signed() < 0 {
                    return s;
                }
            } else if id == 0x21 {
                let block_stop = p.wrapping_add(size);
                if block_stop > stop {
                    return BAD;
                }
                let track = match parse_varint(h, reader, pos, len, block_stop, avail, scratch) {
                    Ok(v) => v,
                    Err(s) => return s,
                };
                if track == 0 {
                    return BAD;
                }
                let p = read_i64(h, pos).wrapping_add(i64::from(h.read::<i32>(len)));
                write_i64(h, pos, p);
                if p.wrapping_add(2) > block_stop {
                    return BAD;
                }
                if p.wrapping_add(2) > avail {
                    h.write(len, 2u32);
                    return MORE;
                }
                let p = p.wrapping_add(2);
                write_i64(h, pos, p);
                if p.wrapping_add(1) > block_stop {
                    return BAD;
                }
                if p.wrapping_add(1) > avail {
                    h.write(len, 1u32);
                    return MORE;
                }
                let flags = at(frame, 0x30);
                let s = read(h, reader, p.cast_unsigned(), 1, flags);
                if s.cast_signed() < 0 {
                    h.write(len, 1u32);
                    return s;
                }
                let p = p.wrapping_add(1);
                write_i64(h, pos, p);
                if p > avail {
                    wassert(h, 0x00b9_1b1c, 0x1a91);
                }
                if p >= block_stop {
                    return BAD;
                }
                if h.read::<u8>(flags) & 6 != 0 && block_stop > avail {
                    h.write(len, block_stop.wrapping_sub(p) as u32);
                    return MORE;
                }
                write_i64(h, pos, block_stop);
                continue;
            }
            let p = read_i64(h, pos).wrapping_add(size);
            write_i64(h, pos, p);
            if p > stop {
                return BAD;
            }
        }
        if read_i64(h, pos) != stop {
            wassert(h, 0x00b9_1b60, 0x1aa1);
        }
        let [s0, s1] = words64(start);
        let [z0, z1] = words64(size);
        let d = read_i64(h, discard);
        let [d0, d1] = words64(d);
        let s = h.call_sym(
            CREATE_BLOCK,
            Conv::Thiscall(this),
            &[0x20, 0, s0, s1, z0, z1, d0, d1],
        );
        if s == 0 {
            write_i64(h, at(this, 0x18), stop);
        }
        s
    }

    /// `mkvparser::BlockGroup`'s constructor (`0x0042d200`, thiscall, 14
    /// words) and `BlockGroup::Parse` (`0x0042d370`, thiscall) and the
    /// group's vtable.
    pub const BLOCK_GROUP_CTOR: u32 = 0x0042_d200;
    pub const BLOCK_GROUP_PARSE: u32 = 0x0042_d370;
    pub const BLOCK_GROUP_VTABLE: u32 = 0x00b9_20ac;

    /// `0x0042d200(this; cluster, index, start, size, prev, next,
    /// duration, discard)` (all but the first two 64-bit): the entry's
    /// fields (vtable, cluster, index), the block at `+0x10` with its start
    /// and size, no track, time code -1, no frames; prev `+0x40`, next
    /// `+0x48`, duration `+0x50`, the block's discard padding `+0x38`.
    /// Returns `this`.
    pub fn block_group_ctor(
        h: &mut impl Host,
        this: u32,
        cluster: u32,
        index: u32,
        w: &[u32; 12],
    ) -> u32 {
        h.write(at(this, 4), cluster);
        h.write(at(this, 8), index);
        let vt = h.rebase(BLOCK_GROUP_VTABLE);
        h.write(this, vt);
        for (i, o) in [0x10u32, 0x14, 0x18, 0x1c].into_iter().enumerate() {
            h.write(at(this, o), w[i]);
        }
        h.write(at(this, 0x28), 0xffffu16);
        h.write(at(this, 0x30), u32::MAX);
        h.write(at(this, 0x38), w[10]);
        h.write(at(this, 0x3c), w[11]);
        h.write(at(this, 0x20), 0u32);
        h.write(at(this, 0x24), 0u32);
        h.write(at(this, 0x2a), 0u8);
        h.write(at(this, 0x2c), 0u32);
        for (i, o) in [0x40u32, 0x44, 0x48, 0x4c, 0x50, 0x54]
            .into_iter()
            .enumerate()
        {
            h.write(at(this, o), w[4 + i]);
        }
        this
    }

    /// `mkvparser::Cluster::CreateBlockGroup(cluster; start, size,
    /// discard)` (`0x0042c300`, thiscall, `ret 0x18`): the group's children
    /// read again (`ReadUInt`; the asserts as the original's): the
    /// `Block`'s position and size (the first one), `BlockDuration`
    /// (`UnserializeUInt`), each `ReferenceBlock` (`UnserializeInt`: -1
    /// when it fails; a time at most 0 is the previous one, else the
    /// next); a `BlockGroup` (0x58 bytes from the movie allocator,
    /// [`BLOCK_GROUP_CTOR`]) in the next entry, parsed
    /// ([`BLOCK_GROUP_PARSE`]): the count grows and 0, else the entry is
    /// deleted (vtable slot 0, 1) and cleared and the status returned; -1
    /// without memory. `frame`: the varint length (+0), the reference
    /// time (+8, 8 bytes) and the readers' scratch (+0x10..+0x30).
    #[allow(clippy::too_many_lines)] // the original's loop
    pub fn create_block_group(
        h: &mut impl Host,
        this: u32,
        start: i64,
        size: i64,
        discard: i64,
        frame: u32,
    ) -> u32 {
        if h.read::<u32>(at(this, 0x30)) == 0 {
            wassert(h, 0x00b9_1b88, 0x1c92);
        }
        if h.read::<i32>(at(this, 0x34)) <= 0 {
            wassert(h, 0x00b9_1b9c, 0x1c93);
        }
        if h.read::<i32>(at(this, 0x38)) < 0 {
            wassert(h, 0x00b9_1cd8, 0x1c94);
        }
        if h.read::<i32>(at(this, 0x38)) >= h.read::<i32>(at(this, 0x34)) {
            wassert(h, 0x00b9_1d08, 0x1c95);
        }
        let segment: u32 = h.read(this);
        let reader: u32 = h.read(segment);
        let stop = start.wrapping_add(size);
        let lenp = frame;
        let time = at(frame, 8);
        let scratch = at(frame, 0x10);
        let mut prev = 1i64;
        let mut next = 0i64;
        let mut duration = -1i64;
        let mut bpos = -1i64;
        let mut bsize = -1i64;
        let mut pos = start;
        if pos < stop {
            loop {
                let id = read_uint(h, reader, lenp, pos.cast_unsigned(), at(scratch, 0x18))
                    .cast_signed();
                if id < 0 {
                    wassert(h, 0x00b9_0414, 0x1cac);
                }
                pos = pos.wrapping_add(i64::from(h.read::<i32>(lenp)));
                if pos > stop {
                    wassert(h, 0x00b9_0ab0, 0x1cad);
                }
                let size = read_uint(h, reader, lenp, pos.cast_unsigned(), at(scratch, 0x18))
                    .cast_signed();
                if size < 0 {
                    wassert(h, 0x00b9_047c, 0x1cb2);
                }
                pos = pos.wrapping_add(i64::from(h.read::<i32>(lenp)));
                if pos > stop {
                    wassert(h, 0x00b9_0ab0, 0x1cb3);
                }
                if id == 0x21 {
                    if bpos < 0 {
                        bpos = pos;
                        bsize = size;
                    }
                } else if id == 0x1b {
                    if size > 8 {
                        wassert(h, 0x00b9_0400, 0x1cbd);
                    }
                    duration = unserialize_uint(
                        h,
                        reader,
                        pos.cast_unsigned(),
                        size.cast_unsigned(),
                        at(scratch, 0x18),
                    )
                    .cast_signed();
                    if duration < 0 {
                        wassert(h, 0x00b9_1d4c, 0x1cc0);
                    }
                } else if id == 0x7b {
                    if size > 8 {
                        wassert(h, 0x00b9_0400, 0x1cc2);
                    }
                    if unserialize_int(
                        h,
                        reader,
                        size as u32,
                        pos.cast_unsigned(),
                        time,
                        at(scratch, 0x18),
                    ) != 0
                    {
                        wassert(h, 0x00b9_03c0, 0x1cc8);
                        return u32::MAX;
                    }
                    let t = read_i64(h, time);
                    if t <= 0 {
                        prev = t;
                    } else {
                        next = t;
                    }
                }
                pos = pos.wrapping_add(size);
                if pos > stop {
                    wassert(h, 0x00b9_0958, 0x1cd3);
                }
                if pos >= stop {
                    break;
                }
            }
        }
        if pos != stop {
            wassert(h, 0x00b9_0970, 0x1cd6);
        }
        if bpos < 0 {
            wassert(h, 0x00b9_1d68, 0x1cd7);
        }
        if bsize < 0 {
            wassert(h, 0x00b9_1d7c, 0x1cd8);
        }
        let index: u32 = h.read(at(this, 0x38));
        let entries: u32 = h.read(at(this, 0x30));
        let slot = at(entries, index.wrapping_mul(4));
        let mem = alloc(h, 0x58);
        let e = if mem == 0 {
            0
        } else {
            let mut w = [0u32; 12];
            for (i, v) in [bpos, bsize, prev, next, duration, discard]
                .into_iter()
                .enumerate()
            {
                let [lo, hi] = words64(v);
                w[2 * i] = lo;
                w[2 * i + 1] = hi;
            }
            let mut args = vec![this, index];
            args.extend(w);
            h.call_sym(BLOCK_GROUP_CTOR, Conv::Thiscall(mem), &args) // call width: 14
        };
        h.write(slot, e);
        if e == 0 {
            return u32::MAX;
        }
        let s = h.call_sym(BLOCK_GROUP_PARSE, Conv::Thiscall(e), &[]);
        if s == 0 {
            let n = h.read::<u32>(at(this, 0x38)).wrapping_add(1);
            h.write(at(this, 0x38), n);
            return 0;
        }
        let e: u32 = h.read(slot);
        if e != 0 {
            let vt: u32 = h.read(e);
            let f: u32 = h.read(vt);
            h.call(f, Conv::Thiscall(e), &[1]);
        }
        h.write(slot, 0u32);
        s
    }

    /// `mkvparser::Cluster::Create(segment, idx, off)` (`0x0042b620`,
    /// fastcall: `ecx` the segment, `edx` the index, `off` 64-bit on the
    /// stack, the caller pops) -> a new cluster (`0x40` bytes from the
    /// movie allocator) at the segment's start (`+0x10`) plus `off`: its
    /// segment, start (`+8`, also `m_pos` `+0x18`) and index (`+0x10`), the
    /// time code and size unknown (`+0x20`, `+0x28` -1), no entries
    /// (`+0x30`, `+0x34`), the entry count -1 (`+0x38`); the other words
    /// as the allocator left them. A null segment, a negative `off` and a
    /// failed allocation (0 returned) are reported.
    pub fn cluster_create(h: &mut impl Host, segment: u32, idx: u32, off: i64) -> u32 {
        if segment == 0 {
            wassert(h, 0x00b9_0fdc, 0x1ad1);
        }
        if off < 0 {
            wassert(h, 0x00b9_06d4, 0x1ad2);
        }
        let start = read_i64(h, at(segment, 0x10)).wrapping_add(off);
        let c = alloc(h, 0x40);
        if c == 0 {
            wassert(h, 0x00b9_0694, 0x1ad8);
            return 0;
        }
        h.write(c, segment);
        write_i64(h, at(c, 8), start);
        h.write(at(c, 0x10), idx);
        write_i64(h, at(c, 0x18), start);
        write_i64(h, at(c, 0x20), -1);
        write_i64(h, at(c, 0x28), -1);
        h.write(at(c, 0x30), 0u32);
        h.write(at(c, 0x34), 0u32);
        h.write(at(c, 0x38), u32::MAX);
        c
    }

    /// `mkvparser::Cluster::HasBlockEntries(segment, off, &pos, &len)`
    /// (`0x0042b8a0`, fastcall: `ecx` the segment, `edx` `&pos`, `off`
    /// 64-bit and `&len` on the stack, the caller pops) -> 1 when the
    /// cluster at the segment's start plus `off` holds a `BlockGroup` or
    /// `SimpleBlock` before its end, the next `Cluster` or `Cues`; 0 when
    /// it holds none (or lies past the file, or its size is 0); -1 for
    /// another id there; -2 for a bad element (past the segment or the
    /// cluster, an unknown size inside); -3 (with `*len` 1 for a first byte
    /// not there) when the data is not available yet; a negative `Length`,
    /// `GetUIntLength` or `ReadUInt` result as its low word. `*pos` follows
    /// the walk. `frame`: `Length`'s results (+0 total, +8 available, the
    /// original's `ebp - 0x28`) and the readers' scratch (+0x10..+0x2c).
    #[allow(clippy::too_many_lines)] // the original's checks
    pub fn has_block_entries<H: Host>(
        h: &mut H,
        segment: u32,
        off: i64,
        pos: u32,
        len: u32,
        frame: u32,
    ) -> u32 {
        const BAD: u32 = (-2i32).cast_unsigned();
        const MORE: u32 = (-3i32).cast_unsigned();
        if segment == 0 {
            wassert(h, 0x00b9_0fdc, 0x1b56);
        }
        if off < 0 {
            wassert(h, 0x00b9_06d4, 0x1b57);
        }
        let reader: u32 = h.read(segment);
        let (total_at, avail_at) = (frame, at(frame, 8));
        let scratch = at(frame, 0x10);
        let s = length(h, reader, total_at, avail_at);
        if s.cast_signed() < 0 {
            return s;
        }
        let total = read_i64(h, total_at);
        let avail = read_i64(h, avail_at);
        if total >= 0 && avail > total {
            wassert(h, 0x00b9_05f8, 0x1b62);
        }
        let start = read_i64(h, at(segment, 0x10));
        let p = start.wrapping_add(off);
        write_i64(h, pos, p);
        if total >= 0 && p >= total {
            return 0;
        }
        let m_size = read_i64(h, at(segment, 0x18));
        let segment_stop = if m_size < 0 {
            -1
        } else {
            m_size.wrapping_add(read_i64(h, at(segment, 0x10)))
        };
        // the cluster's id, then its size: against the segment, the file
        // and the available data
        let header = |h: &mut H| -> Result<i64, u32> {
            let p = read_i64(h, pos);
            if p.wrapping_add(1) > avail {
                h.write(len, 1u32);
                return Err(MORE);
            }
            let r = uint_length_at(h, reader, len, p.cast_unsigned(), scratch);
            if r < 0 {
                return Err(r as u32);
            }
            if r != 0 {
                return Err(MORE);
            }
            let end = p.wrapping_add(i64::from(h.read::<i32>(len)));
            if segment_stop >= 0 && end > segment_stop {
                return Err(BAD);
            }
            if total >= 0 && end > total {
                return Err(0);
            }
            if end > avail {
                return Err(MORE);
            }
            let v = read_uint(h, reader, len, p.cast_unsigned(), at(scratch, 0x18)).cast_signed();
            if v < 0 {
                return Err(v as u32);
            }
            Ok(v)
        };
        let id = match header(h) {
            Ok(v) => v,
            Err(s) => return s,
        };
        if id != 0x0f43_b675 {
            return u32::MAX;
        }
        let p = read_i64(h, pos).wrapping_add(i64::from(h.read::<i32>(len)));
        write_i64(h, pos, p);
        let size = match header(h) {
            Ok(v) => v,
            Err(s) => return s,
        };
        if size == 0 {
            return 0;
        }
        let l = h.read::<i32>(len);
        let p = read_i64(h, pos).wrapping_add(i64::from(l));
        write_i64(h, pos, p);
        let mut cluster_stop = -1i64;
        if size != unknown_size(l) {
            cluster_stop = p.wrapping_add(size);
            if cluster_stop < 0 {
                wassert(h, 0x00b9_1c1c, 0x1bb7);
            }
            if segment_stop >= 0 && cluster_stop > segment_stop {
                return BAD;
            }
            if total >= 0 && cluster_stop > total {
                return 0;
            }
        }
        loop {
            let p = read_i64(h, pos);
            if cluster_stop >= 0 && p >= cluster_stop {
                return 0;
            }
            let id = match parse_varint(h, reader, pos, len, cluster_stop, avail, scratch) {
                Ok(v) => v,
                Err(s) => return s,
            };
            if id == 0x0f43_b675 || id == 0x0c53_bb6b {
                return 0;
            }
            let p = read_i64(h, pos).wrapping_add(i64::from(h.read::<i32>(len)));
            write_i64(h, pos, p);
            if cluster_stop >= 0 && p >= cluster_stop {
                return BAD;
            }
            let size = match parse_varint(h, reader, pos, len, cluster_stop, avail, scratch) {
                Ok(v) => v,
                Err(s) => return s,
            };
            let l = h.read::<i32>(len);
            let p = read_i64(h, pos).wrapping_add(i64::from(l));
            write_i64(h, pos, p);
            if cluster_stop >= 0 && p > cluster_stop {
                return BAD;
            }
            if size == 0 {
                continue;
            }
            if size == unknown_size(l) {
                return BAD;
            }
            if cluster_stop >= 0 && p.wrapping_add(size) > cluster_stop {
                return BAD;
            }
            if id == 0x20 || id == 0x23 {
                return 1;
            }
            let p = p.wrapping_add(size);
            write_i64(h, pos, p);
            if cluster_stop >= 0 && p > cluster_stop {
                wassert(h, 0x00b9_19d8, 0x1c20);
            }
        }
    }

    /// `mkvparser::Segment::PreloadCluster(segment; cluster, idx)`
    /// (`0x00421270`, thiscall, `ret 8`): `cluster` (a preloaded one, its
    /// index negative) put at `idx` of the cluster array (`+0x80`) after the
    /// loaded (`+0x84`) ones, the preloaded ones from there moved up one;
    /// the array grown first when full (`+0x8c` its size: doubled, 0x800
    /// from nothing; the movie allocator, the old one freed); the preload
    /// count (`+0x88`) one more. Bad arguments and states are reported.
    pub fn segment_preload_cluster(h: &mut impl Host, seg: u32, cluster: u32, idx: i32) {
        if cluster == 0 {
            wassert(h, 0x00b9_0694, 0x636);
        }
        if h.read::<i32>(at(cluster, 0x10)) >= 0 {
            wassert(h, 0x00b9_06a8, 0x637);
        }
        if idx < h.read::<i32>(at(seg, 0x84)) {
            wassert(h, 0x00b9_0880, 0x638);
        }
        let count = h
            .read::<i32>(at(seg, 0x88))
            .wrapping_add(h.read::<i32>(at(seg, 0x84)));
        if h.read::<i32>(at(seg, 0x8c)) < count {
            wassert(h, 0x00b9_07b4, 0x63d);
        }
        let size: i32 = h.read(at(seg, 0x8c));
        if count >= size {
            grow_clusters(h, seg, size, count);
        }
        if h.read::<u32>(at(seg, 0x80)) == 0 {
            wassert(h, 0x00b9_06e8, 0x651);
        }
        let arr: u32 = h.read(at(seg, 0x80));
        let slot = idx.cast_unsigned().wrapping_mul(4);
        let p = at(arr, slot);
        let mut q = at(arr, count.cast_unsigned().wrapping_mul(4));
        if q < p {
            wassert(h, 0x00b9_08ac, 0x656);
        }
        let arr: u32 = h.read(at(seg, 0x80));
        let cap: u32 = h.read(at(seg, 0x8c));
        if q >= at(arr, cap.wrapping_mul(4)) {
            wassert(h, 0x00b9_0828, 0x657);
        }
        while q > p {
            let prev: u32 = h.read(q.wrapping_sub(4));
            if h.read::<i32>(at(prev, 0x10)) >= 0 {
                wassert(h, 0x00b9_0858, 0x65b);
            }
            let prev: u32 = h.read(q.wrapping_sub(4));
            h.write(q, prev);
            q = q.wrapping_sub(4);
        }
        let arr: u32 = h.read(at(seg, 0x80));
        h.write(at(arr, slot), cluster);
        let n = h.read::<u32>(at(seg, 0x88)).wrapping_add(1);
        h.write(at(seg, 0x88), n);
    }

    /// `mkvparser::Segment::GetNext(segment; cur)` (`0x004238d0`,
    /// thiscall, `ret 4`) -> the cluster after `cur`. A loaded one (index
    /// `+0x10` not negative) gives the next loaded one, or the end-of-stream
    /// cluster (`+0x20`) after the last. A preloaded one (index negative):
    /// its element (`+8`, absolute) is read (a `Cluster` id, else reported
    /// and 0) and the following top-level elements walked to the segment's
    /// stop (`+0x10` start plus `+0x18` size); the first `Cluster` with
    /// block entries ([`has_block_entries`]; none or an error goes on) at
    /// an offset above 0 is looked up among the preloaded clusters by
    /// position (found: returned) or created ([`cluster_create`]) and
    /// preloaded ([`segment_preload_cluster`]) in its place. 0 when the
    /// segment ends first or a length read fails. Broken states are
    /// reported (`_wassert`) as the original's asserts. `frame`: the
    /// varint length (+0), the cluster's `pos` and `len` (+8, +0x10), the
    /// readers' scratch (+0x14..+0x30) and the cluster's frame (+0x30..).
    #[allow(clippy::too_many_lines)] // the original's walk
    pub fn segment_get_next<H: Host>(h: &mut H, seg: u32, cur: u32, frame: u32) -> u32 {
        if cur == 0 {
            wassert(h, 0x00b9_1014, 0xb92);
        }
        let eos = at(seg, 0x20);
        if cur == eos {
            wassert(h, 0x00b9_1020, 0xb93);
        }
        if h.read::<u32>(at(seg, 0x80)) == 0 {
            wassert(h, 0x00b9_06e8, 0xb94);
        }
        let idx: i32 = h.read(at(cur, 0x10));
        if idx >= 0 {
            if h.read::<i32>(at(seg, 0x84)) <= 0 {
                wassert(h, 0x00b9_1040, 0xb99);
            }
            if idx >= h.read::<i32>(at(seg, 0x84)) {
                wassert(h, 0x00b9_1068, 0xb9a);
            }
            let clusters: u32 = h.read(at(seg, 0x80));
            if h.read::<u32>(at(clusters, idx.cast_unsigned().wrapping_mul(4))) != cur {
                wassert(h, 0x00b9_1094, 0xb9b);
            }
            let idx = idx.wrapping_add(1);
            if idx >= h.read::<i32>(at(seg, 0x84)) {
                return eos;
            }
            let clusters: u32 = h.read(at(seg, 0x80));
            let next: u32 = h.read(at(clusters, idx.cast_unsigned().wrapping_mul(4)));
            if next == 0 {
                wassert(h, 0x00b9_0dd4, 0xba3);
            }
            if h.read::<i32>(at(next, 0x10)) < 0 {
                wassert(h, 0x00b9_10c8, 0xba4);
            }
            if h.read::<i32>(at(next, 0x10)) != idx {
                wassert(h, 0x00b9_10f0, 0xba5);
            }
            return next;
        }
        if h.read::<i32>(at(seg, 0x88)) <= 0 {
            wassert(h, 0x00b9_0e14, 0xbaa);
        }
        let mut p = read_i64(h, at(cur, 8));
        if read_i64(h, at(seg, 0x18)) < 0 {
            wassert(h, 0x00b9_111c, 0xbae);
        }
        let stop = read_i64(h, at(seg, 0x18)).wrapping_add(read_i64(h, at(seg, 0x10)));
        let len = frame;
        let scratch = at(frame, 0x14);
        // `GetUIntLength` then `ReadUInt` at `p`: the length's status
        // reported (`line`, and `line + 1` for a varint past the stop);
        // with `bail` a bad status returned before the read
        let varint = |h: &mut H, p: i64, line: u32, bail: bool| -> (i64, i64, i64) {
            let reader: u32 = h.read(seg);
            let r = uint_length_at(h, reader, len, p.cast_unsigned(), scratch);
            if r != 0 {
                wassert(h, 0x00b9_1134, line);
            }
            let l = i64::from(h.read::<i32>(len));
            if p.wrapping_add(l) > stop {
                wassert(h, 0x00b9_0ab0, line + 1);
            }
            if bail && r != 0 {
                return (r, 0, l);
            }
            let reader: u32 = h.read(seg);
            let v = read_uint(h, reader, len, p.cast_unsigned(), at(scratch, 0x18)).cast_signed();
            (0, v, i64::from(h.read::<i32>(len)))
        };
        let (r, id, l) = varint(h, p, 0xbb5, true);
        if r != 0 {
            return 0;
        }
        if id != 0x0f43_b675 {
            wassert(h, 0x00b9_114c, 0xbbb);
            return 0;
        }
        p = p.wrapping_add(l);
        // (the size's length status is reported, not acted on)
        let (_, size, l) = varint(h, p, 0xbc3, false);
        if size <= 0 {
            wassert(h, 0x00b9_03ec, 0xbc7);
        }
        p = p.wrapping_add(l).wrapping_add(size);
        if p > stop {
            wassert(h, 0x00b9_0ad8, 0xbcb);
        }
        while p < stop {
            let (r, id, l) = varint(h, p, 0xbd8, true);
            if r != 0 {
                return 0;
            }
            let id_pos = p;
            if id <= 0 {
                wassert(h, 0x00b9_1170, 0xbe0);
            }
            p = p.wrapping_add(l);
            let (_, size, l) = varint(h, p, 0xbe6, false);
            if size < 0 {
                wassert(h, 0x00b9_047c, 0xbea);
            }
            p = p.wrapping_add(l);
            let next = p.wrapping_add(size);
            if next > stop {
                wassert(h, 0x00b9_0ad8, 0xbed);
            }
            if size == 0 {
                continue;
            }
            if id == 0x0f43_b675 {
                let off = id_pos.wrapping_sub(read_i64(h, at(seg, 0x10)));
                let status =
                    has_block_entries(h, seg, off, at(frame, 8), at(frame, 0x10), at(frame, 0x30))
                        .cast_signed();
                if status < 0 {
                    wassert(h, 0x00b9_0360, 0xbfc);
                }
                if status > 0 {
                    if off <= 0 {
                        return 0;
                    }
                    return preloaded_at(h, seg, off);
                }
            }
            p = next;
        }
        0
    }

    /// The preloaded cluster at `off` for [`segment_get_next`]: a binary
    /// search over the preloaded ones (after the loaded ones) by position
    /// (each one's start less its segment's start); else a new one created
    /// and preloaded where the search ended.
    fn preloaded_at(h: &mut impl Host, seg: u32, off: i64) -> u32 {
        let clusters: u32 = h.read(at(seg, 0x80));
        let count: u32 = h.read(at(seg, 0x84));
        let mut i = at(clusters, count.wrapping_mul(4));
        let preload: u32 = h.read(at(seg, 0x88));
        let end = at(i, preload.wrapping_mul(4));
        let mut j = end;
        while i < j {
            let n = (j.wrapping_sub(i).cast_signed() >> 2) / 2;
            let k = at(i, n.cast_unsigned().wrapping_mul(4));
            if k >= end {
                wassert(h, 0x00b9_0c4c, 0xc17);
            }
            let c: u32 = h.read(k);
            if c == 0 {
                wassert(h, 0x00b9_0dd4, 0xc1a);
            }
            if h.read::<i32>(at(c, 0x10)) >= 0 {
                wassert(h, 0x00b9_1180, 0xc1b);
            }
            let cs: u32 = h.read(c);
            let cpos = read_i64(h, at(c, 8)).wrapping_sub(read_i64(h, at(cs, 0x10)));
            if cpos < 0 {
                wassert(h, 0x00b9_034c, 0x1b0d);
            }
            match cpos.cmp(&off) {
                std::cmp::Ordering::Less => i = at(k, 4),
                std::cmp::Ordering::Equal => return c,
                std::cmp::Ordering::Greater => j = k,
            }
        }
        if i != j {
            wassert(h, 0x00b9_0c6c, 0xc2b);
        }
        let c = cluster_create(h, seg, u32::MAX, off);
        if c == 0 {
            wassert(h, 0x00b9_0dd4, 0xc2e);
        }
        let clusters: u32 = h.read(at(seg, 0x80));
        let idx = i.wrapping_sub(clusters).cast_signed() >> 2;
        h.call_sym(
            PRELOAD_CLUSTER,
            Conv::Thiscall(seg),
            &[c, idx.cast_unsigned()],
        );
        if h.read::<u32>(at(seg, 0x80)) == 0 {
            wassert(h, 0x00b9_06e8, 0xc33);
        }
        if idx >= h.read::<i32>(at(seg, 0x8c)) {
            wassert(h, 0x00b9_11a8, 0xc34);
        }
        let clusters: u32 = h.read(at(seg, 0x80));
        if h.read::<u32>(at(clusters, idx.cast_unsigned().wrapping_mul(4))) != c {
            wassert(h, 0x00b9_11dc, 0xc35);
        }
        c
    }

    /// `Segment::PreloadCluster` (thiscall, `ret 8`), `Cluster::Create`
    /// and `Cluster::HasBlockEntries` (fastcall, the caller pops).
    pub const PRELOAD_CLUSTER: u32 = 0x0042_1270;
    pub const CLUSTER_CREATE: u32 = 0x0042_b620;
    pub const HAS_BLOCK_ENTRIES: u32 = 0x0042_b8a0;
    pub const SEGMENT_GET_NEXT: u32 = 0x0042_38d0;

    /// `Segment::DoLoadClusterUnknownSize` and `Segment::AppendCluster`
    /// (thiscall, `ret 8` / `ret 4`).
    pub const DO_LOAD_CLUSTER: u32 = 0x0042_0730;
    pub const DO_LOAD_CLUSTER_UNKNOWN_SIZE: u32 = 0x0042_0f70;
    pub const APPEND_CLUSTER: u32 = 0x0042_1060;

    /// `mkvparser::Segment::DoLoadClusterUnknownSize(segment; &pos, &len)`
    /// (`0x00420f70`, thiscall, `ret 8`): the cluster of unknown size
    /// being loaded (`+0x68`) parsed on ([`CLUSTER_PARSE`]): an error as it
    /// is; at its end (positive) its size (`+0x20`, a negative one
    /// reported) is known, `*pos` and `m_pos` (`+0x60`) go past it and it is
    /// no longer the one loaded; 2 (also while it goes on). A known `m_pos`
    /// and no such cluster are reported.
    pub fn segment_do_load_cluster_unknown_size(
        h: &mut impl Host,
        seg: u32,
        pos: u32,
        len: u32,
    ) -> u32 {
        if read_i64(h, at(seg, 0x60)) >= 0 {
            wassert(h, 0x00b9_0738, 0x54b);
        }
        if h.read::<u32>(at(seg, 0x68)) == 0 {
            wassert(h, 0x00b9_074c, 0x54c);
        }
        let cl: u32 = h.read(at(seg, 0x68));
        let s = h.call_sym(CLUSTER_PARSE, Conv::Thiscall(cl), &[pos, len]);
        if s.cast_signed() < 0 {
            return s;
        }
        if s != 0 {
            let cl: u32 = h.read(at(seg, 0x68));
            let start = read_i64(h, at(cl, 8));
            let size = read_i64(h, at(cl, 0x20));
            if size < 0 {
                wassert(h, 0x00b9_047c, 0x5f2);
            }
            h.write(at(seg, 0x68), 0u32);
            let end = size.wrapping_add(start);
            write_i64(h, pos, end);
            write_i64(h, at(seg, 0x60), read_i64(h, pos));
        }
        2
    }

    /// `mkvparser::Segment::AppendCluster(segment; cluster)` (`0x00421060`,
    /// thiscall, `ret 4`): `cluster` (a loaded one: its index the loaded
    /// count `+0x84`) put after the loaded ones in the cluster array
    /// (`+0x80`), the preloaded ones (`+0x88`) moved up one; the array
    /// grown first when full (as [`segment_preload_cluster`]); the loaded
    /// count one more. Bad arguments and states are reported.
    pub fn segment_append_cluster(h: &mut impl Host, seg: u32, cluster: u32) {
        if cluster == 0 {
            wassert(h, 0x00b9_0694, 0x5fe);
        }
        if h.read::<i32>(at(cluster, 0x10)) < 0 {
            wassert(h, 0x00b9_0784, 0x5ff);
        }
        let count = h
            .read::<i32>(at(seg, 0x88))
            .wrapping_add(h.read::<i32>(at(seg, 0x84)));
        if h.read::<i32>(at(seg, 0x8c)) < count {
            wassert(h, 0x00b9_07b4, 0x604);
        }
        let idx: i32 = h.read(at(cluster, 0x10));
        if idx != h.read::<i32>(at(seg, 0x84)) {
            wassert(h, 0x00b9_07d0, 0x607);
        }
        let size: i32 = h.read(at(seg, 0x8c));
        if count >= size {
            grow_clusters(h, seg, size, count);
        }
        if h.read::<i32>(at(seg, 0x88)) > 0 {
            if h.read::<u32>(at(seg, 0x80)) == 0 {
                wassert(h, 0x00b9_06e8, 0x61c);
            }
            let arr: u32 = h.read(at(seg, 0x80));
            let loaded: u32 = h.read(at(seg, 0x84));
            let p = at(arr, loaded.wrapping_mul(4));
            if h.read::<u32>(p) == 0 {
                wassert(h, 0x00b9_07fc, 0x61f);
            }
            let first: u32 = h.read(p);
            if h.read::<i32>(at(first, 0x10)) >= 0 {
                wassert(h, 0x00b9_0804, 0x620);
            }
            let pre: u32 = h.read(at(seg, 0x88));
            let mut q = at(p, pre.wrapping_mul(4));
            let arr: u32 = h.read(at(seg, 0x80));
            let cap: u32 = h.read(at(seg, 0x8c));
            if q >= at(arr, cap.wrapping_mul(4)) {
                wassert(h, 0x00b9_0828, 0x623);
            }
            loop {
                let prev: u32 = h.read(q.wrapping_sub(4));
                if h.read::<i32>(at(prev, 0x10)) >= 0 {
                    wassert(h, 0x00b9_0858, 0x627);
                }
                let prev: u32 = h.read(q.wrapping_sub(4));
                h.write(q, prev);
                q = q.wrapping_sub(4);
                if q == p {
                    break;
                }
            }
        }
        let arr: u32 = h.read(at(seg, 0x80));
        h.write(at(arr, idx.cast_unsigned().wrapping_mul(4)), cluster);
        let n = h.read::<u32>(at(seg, 0x84)).wrapping_add(1);
        h.write(at(seg, 0x84), n);
    }

    /// The cluster array grown as `PreloadCluster`/`AppendCluster` grow it:
    /// `size` doubled (0x800 from nothing) words from the movie allocator,
    /// the `count` entries copied, the old array freed.
    fn grow_clusters(h: &mut impl Host, seg: u32, size: i32, count: i32) {
        let n = if size <= 0 {
            0x800
        } else {
            size.wrapping_mul(2)
        }
        .cast_unsigned();
        let bytes = u64::from(n) * 4;
        let q = alloc(h, u32::try_from(bytes).unwrap_or(u32::MAX));
        let old: u32 = h.read(at(seg, 0x80));
        let end = at(old, count.cast_unsigned().wrapping_mul(4));
        let (mut s, mut d) = (old, q);
        while s != end {
            let v: u32 = h.read(s);
            h.write(d, v);
            s = at(s, 4);
            d = at(d, 4);
        }
        let old: u32 = h.read(at(seg, 0x80));
        if old != 0 {
            free(h, old);
        }
        h.write(at(seg, 0x80), q);
        h.write(at(seg, 0x8c), n);
    }

    /// `mkvparser::Segment::DoLoadCluster(segment; &pos, &len)`
    /// (`0x00420730`, thiscall, `ret 8`): an `m_pos` (`+0x60`) of unknown
    /// size goes on in [`DO_LOAD_CLUSTER_UNKNOWN_SIZE`]. Else the top-level
    /// elements from `m_pos` are read (id and size as `Cluster::Parse`
    /// reads them, against the segment's stop and the available data): 1
    /// at the file's or the segment's end; empty ones and others skipped
    /// (an unknown size there -2); the first `Cues` makes the segment's
    /// cues (`+0x78`, 0x40 bytes from the movie allocator); a `Cluster`
    /// asks [`has_block_entries`] (its error, `pos` and `len` returned).
    /// The first preloaded cluster at that offset becomes the next loaded
    /// one (none: -2; its size from the element or the cluster); else one
    /// with entries is created ([`cluster_create`]) and appended
    /// ([`APPEND_CLUSTER`]): an unknown size makes it the one loaded
    /// (`+0x68`, `m_pos` minus the position); 0 with `m_pos` past it.
    /// Without entries `m_pos` goes past it: 2, or 1 (`m_pos` the end) at
    /// the file's or the segment's end. `frame`: `Length`'s results (+0,
    /// +8), the readers' scratch (+0x10..+0x2c), the cluster's `pos` and
    /// `len` (+0x30, +0x38) and its frame (+0x40..+0x70).
    #[allow(clippy::too_many_lines)] // the original's walk
    pub fn segment_do_load_cluster<H: Host>(
        h: &mut H,
        seg: u32,
        pos: u32,
        len: u32,
        frame: u32,
    ) -> u32 {
        const BAD: u32 = (-2i32).cast_unsigned();
        if read_i64(h, at(seg, 0x60)) < 0 {
            return h.call_sym(
                DO_LOAD_CLUSTER_UNKNOWN_SIZE,
                Conv::Thiscall(seg),
                &[pos, len],
            );
        }
        let reader: u32 = h.read(seg);
        let (total_at, avail_at) = (frame, at(frame, 8));
        let scratch = at(frame, 0x10);
        let s = length(h, reader, total_at, avail_at);
        if s.cast_signed() < 0 {
            return s;
        }
        let total = read_i64(h, total_at);
        let avail = read_i64(h, avail_at);
        if total >= 0 && avail > total {
            wassert(h, 0x00b9_05f8, 0x3b7);
        }
        let m_size = read_i64(h, at(seg, 0x18));
        let segment_stop = if m_size < 0 {
            -1
        } else {
            read_i64(h, at(seg, 0x10)).wrapping_add(m_size)
        };
        // the original's `ebp - 0x34`, which an element's end or the cues'
        // size last went through (handed back as `len` when the cluster's
        // `Length` fails; never written: 0)
        let mut last34 = 0u32;
        loop {
            let m_pos = read_i64(h, at(seg, 0x60));
            if (total >= 0 && m_pos >= total) || (segment_stop >= 0 && m_pos >= segment_stop) {
                return 1;
            }
            write_i64(h, pos, m_pos);
            let id = match parse_varint(h, reader, pos, len, segment_stop, avail, scratch) {
                Ok(v) => v,
                Err(s) => return s,
            };
            let id_pos = read_i64(h, pos);
            let p = id_pos.wrapping_add(i64::from(h.read::<i32>(len)));
            write_i64(h, pos, p);
            let size = match parse_varint(h, reader, pos, len, segment_stop, avail, scratch) {
                Ok(v) => v,
                Err(s) => return s,
            };
            let l = h.read::<i32>(len);
            let p = read_i64(h, pos).wrapping_add(i64::from(l));
            write_i64(h, pos, p);
            if size == 0 {
                write_i64(h, at(seg, 0x60), p);
                continue;
            }
            let unknown = unknown_size(l);
            if segment_stop >= 0 && size != unknown {
                let end = p.wrapping_add(size);
                last34 = end as u32;
                if end > segment_stop {
                    return BAD;
                }
            }
            if id == 0x0c53_bb6b {
                if size == unknown {
                    return BAD;
                }
                if h.read::<u32>(at(seg, 0x78)) == 0 {
                    let element_size = p.wrapping_sub(id_pos).wrapping_add(size);
                    last34 = element_size as u32;
                    let c = alloc(h, 0x40);
                    if c != 0 {
                        h.write(c, seg);
                        write_i64(h, at(c, 8), p);
                        write_i64(h, at(c, 0x10), size);
                        write_i64(h, at(c, 0x18), id_pos);
                        write_i64(h, at(c, 0x20), element_size);
                        for o in [0x28, 0x2c, 0x30] {
                            h.write(at(c, o), 0u32);
                        }
                        write_i64(h, at(c, 0x38), p);
                    }
                    h.write(at(seg, 0x78), c);
                    if c == 0 {
                        wassert(h, 0x00b9_0638, 0x422);
                    }
                }
            } else if id == 0x0f43_b675 {
                return load_cluster_at(
                    h,
                    seg,
                    pos,
                    len,
                    frame,
                    id_pos,
                    (size != unknown).then_some(size),
                    (total, segment_stop, last34),
                );
            } else if size == unknown {
                return BAD;
            }
            let p = read_i64(h, pos).wrapping_add(size);
            write_i64(h, at(seg, 0x60), p);
        }
    }

    /// [`segment_do_load_cluster`] at a `Cluster` element (`id_pos`; its
    /// payload at `*pos`, `size` when known); `limits` the file's size,
    /// the segment's stop and the original's `ebp - 0x34`.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)] // the walk's state and checks
    fn load_cluster_at<H: Host>(
        h: &mut H,
        seg: u32,
        pos: u32,
        len: u32,
        frame: u32,
        id_pos: i64,
        size: Option<i64>,
        limits: (i64, i64, u32),
    ) -> u32 {
        const BAD: u32 = (-2i32).cast_unsigned();
        let (total, segment_stop, last34) = limits;
        let off = id_pos.wrapping_sub(read_i64(h, at(seg, 0x10)));
        let cluster_size = size.unwrap_or(-1);
        if off < 0 {
            wassert(h, 0x00b9_0648, 0x43b);
        }
        // the cluster's `pos` and `len` as the original's frame holds them
        // (its `Length` failing leaves them): the id's high word, junk (0)
        let (cpos, clen) = (at(frame, 0x30), at(frame, 0x38));
        write_i64(h, cpos, 0);
        h.write(clen, last34);
        let status = has_block_entries(h, seg, off, cpos, clen, at(frame, 0x40));
        if status.cast_signed() < 0 {
            write_i64(h, pos, read_i64(h, cpos));
            let l: u32 = h.read(clen);
            h.write(len, l);
            return status;
        }
        let idx: u32 = h.read(at(seg, 0x84));
        if h.read::<i32>(at(seg, 0x88)) > 0 {
            if idx.cast_signed() >= h.read::<i32>(at(seg, 0x8c)) {
                wassert(h, 0x00b9_066c, 0x469);
            }
            let arr: u32 = h.read(at(seg, 0x80));
            let c: u32 = h.read(at(arr, idx.wrapping_mul(4)));
            if c == 0 {
                wassert(h, 0x00b9_0694, 0x46c);
            }
            if h.read::<i32>(at(c, 0x10)) >= 0 {
                wassert(h, 0x00b9_06a8, 0x46d);
            }
            let cs: u32 = h.read(c);
            let c_off = read_i64(h, at(c, 8)).wrapping_sub(read_i64(h, at(cs, 0x10)));
            if c_off < 0 {
                wassert(h, 0x00b9_034c, 0x1b0d);
                wassert(h, 0x00b9_06d4, 0x470);
            }
            if c_off == off {
                if status == 0 {
                    return BAD;
                }
                if cluster_size >= 0 {
                    write_i64(h, pos, read_i64(h, pos).wrapping_add(cluster_size));
                } else {
                    let sz = read_i64(h, at(c, 0x20));
                    if sz <= 0 {
                        return BAD;
                    }
                    write_i64(h, pos, read_i64(h, at(c, 8)).wrapping_add(sz));
                }
                h.write(at(c, 0x10), idx);
                let n = h.read::<u32>(at(seg, 0x84)).wrapping_add(1);
                h.write(at(seg, 0x84), n);
                let n = h.read::<u32>(at(seg, 0x88)).wrapping_sub(1);
                h.write(at(seg, 0x88), n);
                write_i64(h, at(seg, 0x60), read_i64(h, pos));
                if segment_stop >= 0 && read_i64(h, at(seg, 0x60)) > segment_stop {
                    wassert(h, 0x00b9_0598, 0x486);
                }
                return 0;
            }
        }
        if status == 0 {
            if cluster_size < 0 {
                return BAD;
            }
            let p = read_i64(h, pos).wrapping_add(cluster_size);
            write_i64(h, pos, p);
            let end = if total >= 0 && p >= total {
                Some(total)
            } else if segment_stop >= 0 && p >= segment_stop {
                Some(segment_stop)
            } else {
                None
            };
            write_i64(h, at(seg, 0x60), end.unwrap_or(p));
            return if end.is_some() { 1 } else { 2 };
        }
        let c = cluster_create(h, seg, idx, off);
        if c == 0 {
            wassert(h, 0x00b9_0694, 0x4a4);
        }
        h.call_sym(APPEND_CLUSTER, Conv::Thiscall(seg), &[c]);
        if h.read::<u32>(at(seg, 0x80)) == 0 {
            wassert(h, 0x00b9_06e8, 0x4a7);
        }
        if idx.cast_signed() >= h.read::<i32>(at(seg, 0x8c)) {
            wassert(h, 0x00b9_066c, 0x4a8);
        }
        let arr: u32 = h.read(at(seg, 0x80));
        if h.read::<u32>(at(arr, idx.wrapping_mul(4))) != c {
            wassert(h, 0x00b9_0700, 0x4a9);
        }
        if cluster_size < 0 {
            h.write(at(seg, 0x68), c);
            let p = read_i64(h, pos);
            write_i64(h, at(seg, 0x60), p.wrapping_neg());
            return 0;
        }
        let p = read_i64(h, pos).wrapping_add(cluster_size);
        write_i64(h, pos, p);
        write_i64(h, at(seg, 0x60), p);
        if segment_stop >= 0 && read_i64(h, at(seg, 0x60)) > segment_stop {
            wassert(h, 0x00b9_0598, 0x4af);
        }
        0
    }

    /// `Cluster::~Cluster` (ported as `Phyre_PObjectTable_deleteAll`),
    /// the parts' destructors and `eh_vector_destructor_iterator` (CRT,
    /// stdcall `(array, size, count, dtor)`), with the element destructors
    /// it gets.
    pub const CLUSTER_DTOR: u32 = 0x0042_b7c0;
    pub const SEGMENT_DTOR: u32 = 0x0041_fa80;
    pub const TRACKS_DELETING_DTOR: u32 = 0x0041_fbe0;
    pub const INFO_DELETING_DTOR: u32 = 0x0041_fc30;
    pub const CUES_DTOR: u32 = 0x0042_1fe0;
    pub const CHAPTERS_DTOR: u32 = 0x0042_52f0;
    pub const ATOM_CLEAR: u32 = 0x0042_5d20;
    pub const EH_VECTOR_DTOR: u32 = 0x0094_96d2;
    pub const ATOM_DTOR: u32 = 0x0042_5b30;
    pub const DISPLAY_DTOR: u32 = 0x0042_6240;

    /// A `new[]` array of `size`-byte elements with a count word before
    /// it: each element destroyed (`dtor`), the block freed.
    fn delete_array(h: &mut impl Host, arr: u32, size: u32, dtor: u32) {
        let n: u32 = h.read(arr.wrapping_sub(4));
        let f = h.rebase(dtor);
        h.call_sym(EH_VECTOR_DTOR, Conv::Stdcall, &[arr, size, n, f]);
        let block = arr.wrapping_sub(4);
        if block != 0 {
            free(h, block);
        }
    }

    /// `mkvparser::Segment::~Segment(segment)` (`0x0041fa80`, thiscall):
    /// every loaded and preloaded cluster (`+0x80`, `+0x84` + `+0x88`)
    /// destroyed ([`CLUSTER_DTOR`]) and freed (a null one reported), the
    /// array freed; the tracks (`+0x74`) and info (`+0x70`) deleted, the
    /// cues (`+0x78`), chapters (`+0x7c`) destroyed and freed, the seek
    /// head (`+0x6c`) and its two arrays freed; the end-of-stream cluster
    /// (`+0x20`) destroyed. (The original's exception frame has nothing to
    /// unwind here.)
    pub fn segment_dtor(h: &mut impl Host, seg: u32) {
        let n = h
            .read::<u32>(at(seg, 0x88))
            .wrapping_add(h.read::<u32>(at(seg, 0x84)));
        let mut p: u32 = h.read(at(seg, 0x80));
        let end = at(p, n.wrapping_mul(4));
        while p != end {
            let c: u32 = h.read(p);
            p = at(p, 4);
            if c == 0 {
                wassert(h, 0x00b9_0514, 0x26d);
            } else {
                h.call_sym(CLUSTER_DTOR, Conv::Thiscall(c), &[]);
                free(h, c);
            }
        }
        let arr: u32 = h.read(at(seg, 0x80));
        if arr != 0 {
            free(h, arr);
        }
        let tracks: u32 = h.read(at(seg, 0x74));
        if tracks != 0 {
            h.call_sym(TRACKS_DELETING_DTOR, Conv::Thiscall(tracks), &[tracks]);
        }
        let info: u32 = h.read(at(seg, 0x70));
        if info != 0 {
            h.call_sym(INFO_DELETING_DTOR, Conv::Thiscall(info), &[info]);
        }
        for (o, dtor) in [(0x78, CUES_DTOR), (0x7c, CHAPTERS_DTOR)] {
            let x: u32 = h.read(at(seg, o));
            if x != 0 {
                h.call_sym(dtor, Conv::Thiscall(x), &[]);
                free(h, x);
            }
        }
        let seek: u32 = h.read(at(seg, 0x6c));
        if seek != 0 {
            for o in [0x28, 0x30] {
                let a: u32 = h.read(at(seek, o));
                if a != 0 {
                    free(h, a);
                }
            }
            free(h, seek);
        }
        h.call_sym(CLUSTER_DTOR, Conv::Thiscall(at(seg, 0x20)), &[]);
    }

    /// `mkvparser::Tracks`' deleting destructor (`0x0041fbe0`, thiscall,
    /// `ret 4`; the flags not looked at) -> `tracks`: each track
    /// (`+0x28..+0x2c`) deleted through its vtable (slot 0, flags 1), the
    /// array and the tracks freed.
    pub fn tracks_deleting_dtor(h: &mut impl Host, tracks: u32) -> u32 {
        let mut p: u32 = h.read(at(tracks, 0x28));
        let end: u32 = h.read(at(tracks, 0x2c));
        while p != end {
            let t: u32 = h.read(p);
            p = at(p, 4);
            if t != 0 {
                let vt: u32 = h.read(t);
                let f: u32 = h.read(vt);
                h.call(f, Conv::Thiscall(t), &[1]);
            }
        }
        let arr: u32 = h.read(at(tracks, 0x28));
        if arr != 0 {
            free(h, arr);
        }
        free(h, tracks);
        tracks
    }

    /// `mkvparser::SegmentInfo`'s deleting destructor (`0x0041fc30`,
    /// thiscall, `ret 4`; the flags not looked at) -> `info`: its three
    /// strings (`+0x38`, `+0x3c`, `+0x40`) freed and cleared, `info` freed.
    pub fn info_deleting_dtor(h: &mut impl Host, info: u32) -> u32 {
        for o in [0x38, 0x3c, 0x40] {
            let s: u32 = h.read(at(info, o));
            if s != 0 {
                free(h, s);
            }
            h.write(at(info, o), 0u32);
        }
        free(h, info);
        info
    }

    /// `mkvparser::Cues::~Cues(cues)` (`0x00421fe0`, thiscall): each cue
    /// point (`+0x28`, `+0x2c` + `+0x30` of them; a null one reported) and
    /// its track positions (`+0x20`) freed, then the array.
    pub fn cues_dtor(h: &mut impl Host, cues: u32) {
        let n = h
            .read::<u32>(at(cues, 0x30))
            .wrapping_add(h.read::<u32>(at(cues, 0x2c)));
        let mut p: u32 = h.read(at(cues, 0x28));
        let end = at(p, n.wrapping_mul(4));
        while p != end {
            let c: u32 = h.read(p);
            p = at(p, 4);
            if c == 0 {
                wassert(h, 0x00b9_0a60, 0x85c);
            } else {
                let tp: u32 = h.read(at(c, 0x20));
                if tp != 0 {
                    free(h, tp);
                }
                free(h, c);
            }
        }
        let arr: u32 = h.read(at(cues, 0x28));
        if arr != 0 {
            free(h, arr);
        }
    }

    /// `mkvparser::Chapters::~Chapters(chapters)` (`0x004252f0`, thiscall):
    /// the editions (`+0x28`, 12 bytes each, `+0x30` of them) from the
    /// last: each one's atoms (`+0`, `+8` of them, 0x30 bytes) cleared
    /// from the last ([`ATOM_CLEAR`]), the atom array deleted, the edition
    /// cleared; the count goes down as it goes.
    pub fn chapters_dtor(h: &mut impl Host, ch: u32) {
        while h.read::<i32>(at(ch, 0x30)) > 0 {
            let n = h.read::<u32>(at(ch, 0x30)).wrapping_sub(1);
            h.write(at(ch, 0x30), n);
            let editions: u32 = h.read(at(ch, 0x28));
            let e = at(editions, n.wrapping_mul(12));
            while h.read::<i32>(at(e, 8)) > 0 {
                let k = h.read::<u32>(at(e, 8)).wrapping_sub(1);
                h.write(at(e, 8), k);
                let atoms: u32 = h.read(e);
                h.call_sym(
                    ATOM_CLEAR,
                    Conv::Thiscall(at(atoms, k.wrapping_mul(0x30))),
                    &[],
                );
            }
            let atoms: u32 = h.read(e);
            if atoms != 0 {
                delete_array(h, atoms, 0x30, ATOM_DTOR);
            }
            h.write(e, 0u32);
            h.write(at(e, 4), 0u32);
        }
    }

    /// `mkvparser::Chapters::Atom::Clear(atom)` (`0x00425d20`, thiscall):
    /// its string id (`+0`) freed and cleared; its displays (`+0x20`, 12
    /// bytes each, `+0x28` of them) from the last: their three strings
    /// freed and cleared, the count going down; the display array deleted
    /// and cleared (`+0x20`, `+0x24`).
    pub fn atom_clear(h: &mut impl Host, atom: u32) {
        let s: u32 = h.read(atom);
        if s != 0 {
            free(h, s);
        }
        h.write(atom, 0u32);
        while h.read::<i32>(at(atom, 0x28)) > 0 {
            let n = h.read::<u32>(at(atom, 0x28)).wrapping_sub(1);
            h.write(at(atom, 0x28), n);
            let displays: u32 = h.read(at(atom, 0x20));
            let d = at(displays, n.wrapping_mul(12));
            for o in [0, 4, 8] {
                let s: u32 = h.read(at(d, o));
                if s != 0 {
                    free(h, s);
                }
                h.write(at(d, o), 0u32);
            }
        }
        let displays: u32 = h.read(at(atom, 0x20));
        if displays != 0 {
            delete_array(h, displays, 0xc, DISPLAY_DTOR);
        }
        h.write(at(atom, 0x20), 0u32);
        h.write(at(atom, 0x24), 0u32);
    }
}
