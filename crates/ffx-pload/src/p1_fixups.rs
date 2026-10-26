//! List `p1-load`, continuation round 3, batch 5: the cluster reader's
//! user fixups (`PClusterReaderBinary::loadUserFixups`, `0x0057c5d0`).
//!
//! A user fixup is a pointer the file cannot hold (a function, an
//! asset elsewhere): the file has a 12-byte record per fixup (the type
//! index in the namespace mapping, the data's length, its offset in the
//! fixup names block), and the type turns the data into a 16-byte
//! result (`PType::resolveFixupForAddress`). The cluster header (the
//! reader's `+8`) holds the fixup count (`+0x30`) and the names block's
//! size (`+0x34`); the reader's `+4` is the mapping, `+0xc` whether the
//! file's words need swapping.

use ffx_graphic::host::{Conv, Host};

/// `PhyreMalloc(size, align)` and `PhyreFree(p)` (cdecl).
const MALLOC: u32 = 0x0042_fb80;
const FREE: u32 = 0x0042_fb20;
/// `PStreamReader::readExact(buffer, size)` (thiscall on the stream).
const READ_EXACT: u32 = 0x0057_dd30;
/// The 12-byte record's word swap (cdecl `(record)`).
const SWAP_RECORD: u32 = 0x0057_6d20;
/// `PNamespaceMapping::entry(i, &a, &b)` (thiscall on the mapping).
const MAPPING_ENTRY: u32 = 0x0047_e730;
/// `PType::resolveFixupForAddress(&out, data, len, ctx)` (thiscall on
/// the type).
const RESOLVE: u32 = 0x0043_7980;
/// `PAssetReferenceImport`'s class descriptor: its fixups are kept as
/// they are, for the asset references to resolve later.
const IMPORT_CLASS: u32 = 0x00c9_4030;
/// `PhyrePrintf(level, format, ...)` and the messages.
const PRINTF: u32 = 0x0043_52f0;
const NULL_POINTER: u32 = 0x00b3_103c;
const NO_NAME: u32 = 0x00b1_2ab0;
const DATA_HEAD: u32 = 0x00b3_106c;
const DATA_HEX: u32 = 0x00b3_1070;
const DATA_MID: u32 = 0x00b3_1078;
const DATA_CHAR: u32 = 0x00b3_107c;
const NEWLINE: u32 = 0x00b7_cae0;

/// The no-memory error.
const NO_BLOCK: u32 = 0xd;
/// A fixup whose type has no class.
const NO_CLASS: u32 = 0x13;

/// `array` (a count word, bit 31 not owned, then the data word)
/// resized to `n` elements of `size` bytes (`align`): the old data
/// freed when owned and different. False when the block cannot be had.
fn resize(h: &mut impl Host, array: u32, n: u32, size: u32, align: u32) -> bool {
    if n == h.read(array) {
        return true;
    }
    let block = if n == 0 {
        0
    } else {
        let b = h.call_sym(MALLOC, Conv::Cdecl, &[n.wrapping_mul(size), align]);
        if b == 0 {
            return false;
        }
        b
    };
    let old: u32 = h.read(array + 4);
    if old != block && h.read::<u32>(array) >> 31 == 0 && old != 0 {
        h.call_sym(FREE, Conv::Cdecl, &[old]);
    }
    h.write(array + 4, block);
    h.write(array, n);
    true
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        Conv, DATA_CHAR, DATA_HEAD, DATA_HEX, DATA_MID, FREE, Host, IMPORT_CLASS, MALLOC,
        MAPPING_ENTRY, NEWLINE, NO_BLOCK, NO_CLASS, NO_NAME, NULL_POINTER, PRINTF, READ_EXACT,
        RESOLVE, SWAP_RECORD, resize,
    };

    /// The bytes of its frame below its cookie (`ebp - 0x3c` up to
    /// `ebp - 0x10`).
    pub const FIXUPS_FRAME: u32 = 0x2c;

    /// `PhyrePrintf(1, f, args...)`.
    fn print(h: &mut impl Host, f: u32, args: &[u32]) {
        let f = h.rebase(f);
        let mut words = vec![1, f];
        words.extend_from_slice(args);
        h.call_sym(PRINTF, Conv::Cdecl, &words);
    }

    /// The records' block freed (when made and owned); `r`.
    fn release(h: &mut impl Host, owned: bool, records: u32, r: u32) -> u32 {
        if owned && records != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[records]);
        }
        r
    }

    /// One fixup's message when its type resolved it to null: the
    /// type's name, and for data under 32 bytes, the bytes in hex and
    /// as characters (`?` below a space).
    fn report_null(h: &mut impl Host, class: u32, data: u32, len: u32) {
        let mut name: u32 = h.read(class + 0x18);
        if name == 0 {
            name = h.rebase(NO_NAME);
        }
        print(h, NULL_POINTER, &[name]);
        if len >= 0x20 {
            return;
        }
        print(h, DATA_HEAD, &[]);
        for i in 0..len {
            let b: u8 = h.read(data.wrapping_add(i));
            print(h, DATA_HEX, &[u32::from(b)]);
        }
        print(h, DATA_MID, &[]);
        for i in 0..len {
            let b: u8 = h.read(data.wrapping_add(i));
            print(h, DATA_CHAR, &[if b < 0x20 { 0x3f } else { u32::from(b) }]);
        }
        print(h, NEWLINE, &[]);
    }

    /// `PClusterReaderBinary::loadUserFixups(stream, ctx, &results,
    /// &names)` (`0x0057c5d0`, thiscall, `ret 0x10`): the names block
    /// (`names`, resized to the header's `+0x34` bytes, alignment 1) and
    /// the 12-byte records (a temporary block, words swapped when the
    /// reader says so) read from `stream`; `results` resized to the
    /// header's `+0x30` 16-byte entries. Each record whose type index the
    /// mapping knows (`entry`) gets its result: for a
    /// `PAssetReferenceImport` the class, the data's address, its length
    /// and a 1 byte, kept for later; else what the class's
    /// `resolveFixupForAddress` writes (with the byte cleared), and a
    /// message when that is null. 0, 0xd without memory, 0x13 for a type
    /// without a class, or the stream's error; the temporary block freed.
    /// `frame` is the original's `ebp - 0x3c`: the result being built
    /// (`+0`), the entry outputs (`+0x24`, `+0x28`).
    pub fn load_user_fixups(
        h: &mut impl Host,
        reader: u32,
        stream: u32,
        ctx: u32,
        results: u32,
        names: u32,
        frame: u32,
    ) -> u32 {
        let header: u32 = h.read(reader + 8);
        let nb: u32 = h.read(header + 0x34);
        if !resize(h, names, nb, 1, 1) {
            return NO_BLOCK;
        }
        let base: u32 = h.read(names + 4);
        let r = h.call_sym(READ_EXACT, Conv::Thiscall(stream), &[base, nb]);
        if r != 0 {
            return r;
        }
        let header: u32 = h.read(reader + 8);
        let n: u32 = h.read(header + 0x30);
        if !resize(h, results, n, 0x10, 4) {
            return NO_BLOCK;
        }
        let mut dst: u32 = h.read(results + 4);
        let records = if n == 0 {
            0
        } else {
            let b = h.call_sym(MALLOC, Conv::Cdecl, &[n.wrapping_mul(12), 4]);
            if b == 0 {
                return NO_BLOCK;
            }
            b
        };
        // the block is freed at the end unless its count word (n) has
        // bit 31
        let owned = n >> 31 == 0;
        let swap = h.read::<u8>(reader + 0xc) != 0;
        let r = if n != 0 && records != 0 {
            h.call_sym(
                READ_EXACT,
                Conv::Thiscall(stream),
                &[records, n.wrapping_mul(12)],
            )
        } else {
            0
        };
        if records != 0 && swap && n != 0 {
            for i in 0..n {
                h.call_sym(SWAP_RECORD, Conv::Cdecl, &[records.wrapping_add(12 * i)]);
            }
        }
        if r != 0 {
            return release(h, owned, records, r);
        }
        let rec = frame;
        let (b_out, a_out) = (frame + 0x24, frame + 0x28);
        let import = h.rebase(IMPORT_CLASS);
        for i in 0..n {
            let e = records.wrapping_add(12 * i);
            h.write(b_out, 0u32);
            h.write(a_out, 0u32);
            let map: u32 = h.read(reader + 4);
            let index: u32 = h.read(e);
            let found = h.call_sym(MAPPING_ENTRY, Conv::Thiscall(map), &[index, a_out, b_out]);
            if found as u8 != 0 {
                let data = h.read::<u32>(e + 8).wrapping_add(base);
                let a: u32 = h.read(a_out);
                let class = if a != 0 {
                    h.read(a + 4)
                } else {
                    let b: u32 = h.read(b_out);
                    if b == 0 {
                        return release(h, owned, records, NO_CLASS);
                    }
                    h.read(b + 8)
                };
                if class == 0 {
                    return release(h, owned, records, NO_CLASS);
                }
                h.write(rec, 0u32);
                h.write(rec + 0xc, 0u8);
                h.write(rec + 4, 0u32);
                h.write(rec + 8, 0u32);
                if class == import {
                    let len: u32 = h.read(e + 4);
                    h.write(rec, class);
                    h.write(rec + 0xc, 1u8);
                    h.write(rec + 4, data);
                    h.write(rec + 8, len);
                } else {
                    let len: u32 = h.read(e + 4);
                    h.call_sym(RESOLVE, Conv::Thiscall(class), &[rec, data, len, ctx]);
                    let value: u32 = h.read(rec);
                    h.write(rec + 0xc, 0u8);
                    if value == 0 {
                        report_null(h, class, data, len);
                    }
                }
                // junk read: [ebp-0x2f..-0x2d] (the result's last word
                // above its byte) never written here; 0 in the port's
                // frame (brief 2026-10-24)
                let words: [u32; 4] = h.read(rec);
                h.write(dst, words);
            }
            dst = dst.wrapping_add(0x10);
        }
        release(h, owned, records, 0)
    }
}
