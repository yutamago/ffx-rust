//! M5 `system-a` (round 3): the cluster pointer-fixup unpackers
//! (`Fixup_UnpackList`, 28-byte records; `Fixup_UnpackArrayList`, 16-byte
//! records) and the record runs they hand each list to: `Bitmask`,
//! `Except`, `Listed`, `Arithmetic`, each in a 16- and a 28-byte form.
//!
//! A run gets a two-word descriptor (`+0` the list's flags, handed to the
//! callback; `+4` the number of objects `N`), a callback (cdecl
//! `(record, index, &cursor, flags)`), a template record copied into each
//! output record first, the stream cursor (`&cursor`) and the output
//! cursor (`&out`, a pointer to the next record, advanced past each one).
//! Indices come as one byte each while `N < 0x100`, else as LEB128
//! (`fixup::bridge::read_leb128`'s rule).

/// The originals over game memory and the original callees.
pub mod bridge {
    use crate::fixup::bridge::read_leb128;
    use ffx_graphic::host::{Conv, Host};

    /// `Fixup_ReadLeb128(&cursor)` (cdecl).
    pub const READ_LEB128: u32 = 0x0060_ac00;
    /// `Fixup_ReadRecord(record; &cursor, flags)` (thiscall, `ret 8`).
    pub const READ_RECORD: u32 = 0x0060_e730;
    /// `Phyre_PhyrePrintf(level, fmt, ..)` (cdecl, varargs).
    pub const PRINTF: u32 = 0x0043_52f0;
    /// The two messages: an unknown nested kind, an unknown list kind.
    pub const MSG_NESTED: u32 = 0x00b3_ebe4;
    pub const MSG_KIND: u32 = 0x00b3_ec10;
    /// The runs (thiscall `(desc; cb, tmpl, &cursor, &out)`, `ret 0x10`):
    /// the 28-byte ones, then the 16-byte ones.
    pub const RUN_ALL: u32 = 0x0060_ca70;
    pub const RUN_LISTED: u32 = 0x0060_cf40;
    pub const RUN_EXCEPT: u32 = 0x0060_cd50;
    pub const RUN_BITMASK: u32 = 0x0060_cb70;
    pub const RUN_ARITHMETIC: u32 = 0x0060_d080;
    pub const RUN_ALL16: u32 = 0x0060_ca10;
    pub const RUN_LISTED16: u32 = 0x0060_ce70;
    pub const RUN_EXCEPT16: u32 = 0x0060_cc20;
    pub const RUN_BITMASK16: u32 = 0x0060_cac0;
    pub const RUN_ARITHMETIC16: u32 = 0x0060_d000;
    /// The callbacks the unpackers hand the runs: `Fixup_SetObjectAndRead`
    /// and `Fixup_SetObject` (28-byte records); `0x0060ae50` and
    /// `0x0060ae30` (16-byte records).
    pub const SET_OBJECT_AND_READ: u32 = 0x0060_ae90;
    pub const SET_OBJECT: u32 = 0x0060_ae40;
    pub const SET_ARRAY_OBJECT_AND_READ: u32 = 0x0060_ae50;
    pub const SET_ARRAY_OBJECT: u32 = 0x0060_ae30;

    /// A run's arguments.
    #[derive(Debug, Clone, Copy)]
    pub struct Run {
        /// The descriptor (`ecx`).
        pub desc: u32,
        /// The callback (a function pointer, cdecl).
        pub cb: u32,
        /// The template record.
        pub tmpl: u32,
        /// `&cursor`.
        pub cursor: u32,
        /// `&out`.
        pub out: u32,
        /// The record size: 16 or 28.
        pub size: u32,
    }

    fn leb(h: &mut impl Host, cursor: u32) -> u32 {
        h.call_sym(READ_LEB128, Conv::Cdecl, &[cursor])
    }

    /// One index from the stream: a byte while `N < 0x100`, else LEB128.
    fn index(h: &mut impl Host, r: Run) -> u32 {
        if h.read::<u32>(r.desc.wrapping_add(4)) < 0x100 {
            let p: u32 = h.read(r.cursor);
            let b: u8 = h.read(p);
            h.write(r.cursor, p.wrapping_add(1));
            u32::from(b)
        } else {
            read_leb128(h, r.cursor)
        }
    }

    /// The template copied into the next output record, the callback on
    /// it with `index`, the output cursor moved past it. The callback's
    /// `eax`.
    fn emit(h: &mut impl Host, r: Run, index: u32) -> u32 {
        let at: u32 = h.read(r.out);
        for k in (0..r.size).step_by(4) {
            let w: u32 = h.read(r.tmpl.wrapping_add(k));
            h.write(at.wrapping_add(k), w);
        }
        let flags: u32 = h.read(r.desc);
        let at: u32 = h.read(r.out);
        let v = h.call(r.cb, Conv::Cdecl, &[at, index, r.cursor, flags]);
        let at: u32 = h.read(r.out);
        h.write(r.out, at.wrapping_add(r.size));
        v
    }

    /// `Fixup_RunBitmask` / `Fixup_RunBitmask16`: the `N` bits at the
    /// cursor (low bit first; the cursor moved past their `ceil(N / 8)`
    /// bytes first), a record for each set bit.
    pub fn run_bitmask(h: &mut impl Host, r: Run) {
        let n: u32 = h.read(r.desc.wrapping_add(4));
        let mut bytes: u32 = h.read(r.cursor);
        let end = bytes
            .wrapping_add(u32::from(n & 7 != 0))
            .wrapping_add(n >> 3);
        h.write(r.cursor, end);
        let mut bit = 1u8;
        let mut i = 0u32;
        while i < h.read::<u32>(r.desc.wrapping_add(4)) {
            let bit_index = i % 8;
            if bit_index == 0 {
                bit = 1;
            }
            if h.read::<u8>(bytes) & bit != 0 {
                emit(h, r, i);
            }
            if bit_index == 7 {
                bytes = bytes.wrapping_add(1);
            } else {
                bit = bit.wrapping_add(bit);
            }
            i = i.wrapping_add(1);
        }
    }

    /// `Fixup_RunExcept` / `Fixup_RunExcept16`: a LEB128 count of
    /// excluded indices, each read as it comes: a record for every index
    /// before it (from where the last one left off), none for it; then a
    /// record for each index up to `N`. Returns the count.
    pub fn run_except(h: &mut impl Host, r: Run) -> u32 {
        let count = leb(h, r.cursor);
        let mut i = 0u32;
        for _ in 0..count {
            let skip = index(h, r);
            while i < skip {
                emit(h, r, i);
                i = i.wrapping_add(1);
            }
            i = skip.wrapping_add(1);
        }
        while i < h.read::<u32>(r.desc.wrapping_add(4)) {
            emit(h, r, i);
            i = i.wrapping_add(1);
        }
        count
    }

    /// `Fixup_RunListed` / `Fixup_RunListed16`: a LEB128 count, then that
    /// many indices, a record for each. Returns the count.
    pub fn run_listed(h: &mut impl Host, r: Run) -> u32 {
        let count = leb(h, r.cursor);
        for _ in 0..count {
            let i = index(h, r);
            emit(h, r, i);
        }
        count
    }

    /// `Fixup_RunArithmetic` / `Fixup_RunArithmetic16`: LEB128 start,
    /// step and count, a record for each index of the progression.
    /// Returns the last callback's `eax` (0 for none).
    pub fn run_arithmetic(h: &mut impl Host, r: Run) -> u32 {
        let mut i = leb(h, r.cursor);
        let step = leb(h, r.cursor);
        let count = leb(h, r.cursor);
        let mut v = 0;
        for _ in 0..count {
            v = emit(h, r, i);
            i = i.wrapping_add(step);
        }
        v
    }

    /// The frame `Fixup_UnpackList` needs: the nested template at `ebp -
    /// 0x58`, the template at `ebp - 0x3c`, the descriptor at `ebp -
    /// 0x20`.
    pub const LIST_FRAME: u32 = 0x58;
    /// The frame `Fixup_UnpackArrayList` needs: the template at `ebp -
    /// 0x44`, the nested one at `ebp - 0x34`, the descriptor at `ebp -
    /// 0x24`.
    pub const ARRAY_FRAME: u32 = 0x44;

    /// A list header's first byte and value: `kind` (bits 0-2) and the
    /// flags (the byte with bit 0 set and bits 1-2 cleared; bit 1 set
    /// when there is one object); the cursor moved past it, then the
    /// value (LEB128, rotated right by one) read.
    fn list_header(h: &mut impl Host, cursor: u32, count: u32) -> (u32, u32, u32) {
        let p: u32 = h.read(cursor);
        let b = u32::from(h.read::<u8>(p));
        let mut flags = (b & !6) | 1;
        if count == 1 {
            flags |= 2;
        }
        h.write(cursor, p.wrapping_add(1));
        let v = read_leb128(h, cursor).rotate_right(1);
        (b & 7, flags, v)
    }

    /// A run of `table` (`RUN_*`, thiscall `ret 0x10`) on the
    /// descriptor `desc`.
    fn run(h: &mut impl Host, sym: u32, desc: u32, args: [u32; 4]) -> u32 {
        h.call_sym(sym, Conv::Thiscall(desc), &args)
    }

    /// `Fixup_UnpackList(&cursor, records, n, count)` (cdecl): the `n`
    /// 28-byte pointer-fixup records at `records` unpacked list by list,
    /// each list's records for the `count` objects (`out` = the address
    /// of the caller's `records` slot, holding `records`: the output
    /// cursor the runs advance). A list is a header (kind, flags, value: the template's
    /// `+0`, and with flag 0x20 a LEB128 `+0xc`), then by kind: 0 all
    /// objects (`Fixup_RunAll`), 2 listed and 3 all-except (then
    /// `Fixup_ReadRecord` on each record made), 4 a bitmask, 6 an
    /// arithmetic progression (`Fixup_SetObjectAndRead` the callback but
    /// for 2 and 3, `Fixup_SetObject`); 5 explicit records (the
    /// template, `+0` and `+4` read unless flags 1 / 2, then
    /// `Fixup_ReadRecord`); 1 nested lists over the template copy at
    /// `ebp - 0x58` until `count` records are made (each: a kind byte,
    /// then `+0x18` (less 1; -1 when 0), `+8`, `+0xc` (without flag 0x20),
    /// `+0x10` (without 0x40) and `+0x14` (without 8), a missing `+0x18`
    /// skipping the next three; kinds 2..6 but 5 run as above, the rest
    /// reported). An unknown kind is reported (`Phyre_PhyrePrintf`).
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::many_single_char_names
    )] // the seven list kinds; the original's four words, the frame and the slot
    pub fn unpack_list(
        h: &mut impl Host,
        ebp: u32,
        cursor: u32,
        records: u32,
        out: u32,
        n: u32,
        count: u32,
    ) {
        let tmpl2 = ebp.wrapping_sub(0x58);
        let tmpl = ebp.wrapping_sub(0x3c);
        let desc = ebp.wrapping_sub(0x20);
        let mut rec = records;
        let end = rec.wrapping_add(n.wrapping_mul(28));
        if rec < end {
            h.write(desc.wrapping_add(4), count);
        }
        while rec < end {
            for k in 1..7u32 {
                h.write(tmpl.wrapping_add(4 * k), if k == 6 { u32::MAX } else { 0 });
            }
            let (kind, flags, v) = list_header(h, cursor, count);
            h.write(tmpl, v);
            let has_c = flags & 0x20 != 0;
            if has_c {
                let c = leb(h, cursor);
                h.write(tmpl.wrapping_add(0xc), c);
            }
            h.write(desc, flags);
            let start = rec;
            match kind {
                0 => {
                    run(
                        h,
                        RUN_ALL,
                        desc,
                        [h.rebase(SET_OBJECT_AND_READ), tmpl, cursor, out],
                    );
                    rec = h.read(out);
                }
                2 | 3 => {
                    let (sym, cb) = if kind == 2 {
                        (RUN_LISTED, SET_OBJECT)
                    } else {
                        (RUN_EXCEPT, SET_OBJECT)
                    };
                    let made = run(h, sym, desc, [h.rebase(cb), tmpl, cursor, out]);
                    let records = if kind == 2 {
                        made
                    } else {
                        count.wrapping_sub(made)
                    };
                    let mut at = start;
                    for _ in 0..records {
                        h.call_sym(READ_RECORD, Conv::Thiscall(at), &[cursor, flags]);
                        at = at.wrapping_add(0x1c);
                    }
                    rec = h.read(out);
                }
                4 | 6 => {
                    let sym = if kind == 4 {
                        RUN_BITMASK
                    } else {
                        RUN_ARITHMETIC
                    };
                    run(
                        h,
                        sym,
                        desc,
                        [h.rebase(SET_OBJECT_AND_READ), tmpl, cursor, out],
                    );
                    rec = h.read(out);
                }
                5 => {
                    let records = leb(h, cursor);
                    for _ in 0..records {
                        for k in 0..7u32 {
                            let w: u32 = h.read(tmpl.wrapping_add(4 * k));
                            h.write(rec.wrapping_add(4 * k), w);
                        }
                        if flags & 1 == 0 {
                            let x = leb(h, cursor).rotate_right(1);
                            h.write(rec, x);
                        }
                        if flags & 2 == 0 {
                            let x = leb(h, cursor);
                            h.write(rec.wrapping_add(4), x);
                        }
                        h.call_sym(READ_RECORD, Conv::Thiscall(rec), &[cursor, flags]);
                        rec = rec.wrapping_add(0x1c);
                        h.write(out, rec);
                    }
                }
                1 => {
                    let end2 = rec.wrapping_add(count.wrapping_mul(28));
                    for k in 0..7u32 {
                        let w: u32 = h.read(tmpl.wrapping_add(4 * k));
                        h.write(tmpl2.wrapping_add(4 * k), w);
                    }
                    loop {
                        let p: u32 = h.read(cursor);
                        let kind2 = u32::from(h.read::<u8>(p));
                        h.write(cursor, p.wrapping_add(1));
                        let mut more = true;
                        if flags & 0x10 == 0 {
                            let x = leb(h, cursor);
                            h.write(tmpl2.wrapping_add(0x18), x.wrapping_sub(1));
                            more = x == 0;
                        }
                        if more {
                            let x = leb(h, cursor);
                            h.write(tmpl2.wrapping_add(8), x);
                            if !has_c {
                                let x = leb(h, cursor);
                                h.write(tmpl2.wrapping_add(0xc), x);
                            }
                            if flags & 0x40 == 0 {
                                let x = leb(h, cursor);
                                h.write(tmpl2.wrapping_add(0x10), x);
                            }
                        }
                        if flags & 8 == 0 {
                            let x = leb(h, cursor);
                            h.write(tmpl2.wrapping_add(0x14), x);
                        }
                        let sym = match kind2 {
                            2 => Some(RUN_LISTED),
                            3 => Some(RUN_EXCEPT),
                            4 => Some(RUN_BITMASK),
                            6 => Some(RUN_ARITHMETIC),
                            _ => None,
                        };
                        if let Some(sym) = sym {
                            run(h, sym, desc, [h.rebase(SET_OBJECT), tmpl2, cursor, out]);
                            rec = h.read(out);
                        } else {
                            let msg = h.rebase(MSG_NESTED);
                            h.call_sym(PRINTF, Conv::Cdecl, &[3, msg, 1]);
                        }
                        if rec == end2 {
                            break;
                        }
                    }
                }
                _ => {
                    let msg = h.rebase(MSG_KIND);
                    h.call_sym(PRINTF, Conv::Cdecl, &[3, msg, kind]);
                }
            }
        }
    }

    /// `Fixup_UnpackArrayList(&cursor, records, n, count)` (cdecl): the
    /// 16-byte counterpart of [`unpack_list`] (the array fixups). The
    /// template is four words (the value, then zeros); the callbacks are
    /// `0x0060ae50` (`0x0060ae30` for kinds 2 and 3, and nested); after a
    /// listed or all-except run each record made gets `+8` (unless flag
    /// 8) and `+0xc` read; explicit records (5) are the template with `+0`
    /// (unless flag 1), `+4` (unless 2), `+8` (unless 8) and `+0xc` read;
    /// nested lists (1) a kind byte, `+8` (unless flag 8) and `+0xc` (the
    /// stream value, the cursor checked for null, else 0; the value also
    /// goes into the frame word `ebp - 8`) each.
    #[allow(
        clippy::too_many_lines,
        clippy::too_many_arguments,
        clippy::many_single_char_names
    )] // the seven list kinds; the original's four words, the frame and the slot
    pub fn unpack_array_list(
        h: &mut impl Host,
        ebp: u32,
        cursor: u32,
        records: u32,
        out: u32,
        n: u32,
        count: u32,
    ) {
        let tmpl = ebp.wrapping_sub(0x44);
        let tmpl2 = ebp.wrapping_sub(0x34);
        let desc = ebp.wrapping_sub(0x24);
        let spill = ebp.wrapping_sub(8);
        let mut rec = records;
        let end = rec.wrapping_add(n.wrapping_mul(16));
        if rec < end {
            for k in 1..4u32 {
                h.write(tmpl.wrapping_add(4 * k), 0u32);
            }
            h.write(desc.wrapping_add(4), count);
        }
        while rec < end {
            let (kind, flags, v) = list_header(h, cursor, count);
            h.write(spill, v);
            h.write(tmpl, v);
            h.write(desc, flags);
            let start = rec;
            match kind {
                0 => {
                    let cb = h.rebase(SET_ARRAY_OBJECT_AND_READ);
                    run(h, RUN_ALL16, desc, [cb, tmpl, cursor, out]);
                    rec = h.read(out);
                }
                2 | 3 => {
                    let sym = if kind == 2 {
                        RUN_LISTED16
                    } else {
                        RUN_EXCEPT16
                    };
                    let cb = h.rebase(SET_ARRAY_OBJECT);
                    let made = run(h, sym, desc, [cb, tmpl, cursor, out]);
                    let records = if kind == 2 {
                        made
                    } else {
                        count.wrapping_sub(made)
                    };
                    let mut at = start.wrapping_add(0xc);
                    for _ in 0..records {
                        if flags & 8 == 0 {
                            let x = leb(h, cursor);
                            h.write(at.wrapping_sub(4), x);
                        }
                        let x = leb(h, cursor);
                        h.write(at, x);
                        at = at.wrapping_add(0x10);
                    }
                    rec = h.read(out);
                }
                4 | 6 => {
                    let sym = if kind == 4 {
                        RUN_BITMASK16
                    } else {
                        RUN_ARITHMETIC16
                    };
                    let cb = h.rebase(SET_ARRAY_OBJECT_AND_READ);
                    run(h, sym, desc, [cb, tmpl, cursor, out]);
                    rec = h.read(out);
                }
                5 => {
                    let records = leb(h, cursor);
                    for _ in 0..records {
                        let v: u32 = h.read(spill);
                        h.write(rec, v);
                        for k in 1..4u32 {
                            h.write(rec.wrapping_add(4 * k), 0u32);
                        }
                        if flags & 1 == 0 {
                            let x = leb(h, cursor).rotate_right(1);
                            h.write(rec, x);
                        }
                        if flags & 2 == 0 {
                            let x = leb(h, cursor);
                            h.write(rec.wrapping_add(4), x);
                        }
                        if flags & 8 == 0 {
                            let x = leb(h, cursor);
                            h.write(rec.wrapping_add(8), x);
                        }
                        let x = leb(h, cursor);
                        h.write(rec.wrapping_add(0xc), x);
                        rec = rec.wrapping_add(0x10);
                        h.write(out, rec);
                    }
                }
                1 => {
                    let end2 = rec.wrapping_add(count.wrapping_mul(16));
                    h.write(tmpl2, v);
                    for k in 1..4u32 {
                        h.write(tmpl2.wrapping_add(4 * k), 0u32);
                    }
                    loop {
                        let p: u32 = h.read(cursor);
                        let kind2 = u32::from(h.read::<u8>(p));
                        h.write(cursor, p.wrapping_add(1));
                        if flags & 8 == 0 {
                            let x = leb(h, cursor);
                            h.write(tmpl2.wrapping_add(8), x);
                        }
                        let mut x = 0;
                        if h.read::<u32>(cursor) != 0 {
                            x = read_leb128(h, cursor);
                            h.write(spill, x);
                        }
                        h.write(tmpl2.wrapping_add(0xc), x);
                        let sym = match kind2 {
                            2 => Some(RUN_LISTED16),
                            3 => Some(RUN_EXCEPT16),
                            4 => Some(RUN_BITMASK16),
                            6 => Some(RUN_ARITHMETIC16),
                            _ => None,
                        };
                        if let Some(sym) = sym {
                            let cb = h.rebase(SET_ARRAY_OBJECT);
                            run(h, sym, desc, [cb, tmpl2, cursor, out]);
                            rec = h.read(out);
                        } else {
                            let msg = h.rebase(MSG_NESTED);
                            h.call_sym(PRINTF, Conv::Cdecl, &[3, msg, 1]);
                        }
                        if rec == end2 {
                            break;
                        }
                    }
                }
                _ => {
                    let msg = h.rebase(MSG_KIND);
                    h.call_sym(PRINTF, Conv::Cdecl, &[3, msg, kind]);
                }
            }
        }
    }
}
