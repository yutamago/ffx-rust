//! List `p1-load`, round 4: the cluster reader's header classes
//! (`PClusterReaderBinary::loadAndFixHeaderClasses`, `0x0057b6c0`,
//! thiscall on the reader, `(stream)`, `ret 4`; Fahrenheit's name from
//! the Switch build).
//!
//! A header class (descriptor flag `+0x90` bit 2) keeps child objects
//! in its list's data. The header (`+8`) gives the per-list child-record
//! counts (`+0x3c` words, read raw and byte-swapped on a swapping file)
//! and the child records (`+0x40`, 16 bytes each: the mapping index, the
//! data offset, flags, the count; swapped by `0x00576c70`). Per header
//! list, in list order:
//! - every child's class must fit the file's size, the list's class too
//!   (else 9, with a message per class); a missing mapping entry is 0x13;
//! - "aligned" when one of the classes is stored converted (class entry
//!   `+0x14` neither 0 nor bit 31 alone) and the descriptor has flag bit
//!   4: the children are then packed after the list's objects, each
//!   aligned to its class (`+0x20`), moving the data (`memmove`);
//! - a child of a converted class is rebuilt element by element through
//!   a scratch block (`memcpy`, `0x005788f0`), else swapped on a swapping
//!   file (`0x0043d650`); a class child gets its defaults
//!   (`applyDefaultBufferWithValidation`, `0x0043c160`). A child whose
//!   class has no runtime version is skipped with a message; records
//!   with flag bit 1 are skipped.

use ffx_graphic::host::{Conv, Host};

/// `PhyreMalloc(size, align)` and `PhyreFree(p)` (cdecl).
const MALLOC: u32 = 0x0042_fb80;
const FREE: u32 = 0x0042_fb20;
/// `PStreamReader::read` and `readExact(buffer, size)` (thiscall on the
/// stream).
const READ: u32 = 0x0057_dd10;
const READ_EXACT: u32 = 0x0057_dd30;
/// The child record's word swap (cdecl `(record)`).
const SWAP_RECORD: u32 = 0x0057_6c70;
/// `PNamespaceMapping::entry(i, &type, &class)` (thiscall on the
/// mapping): `al` whether `i` is in the mapping.
const MAPPING_ENTRY: u32 = 0x0047_e730;
/// The element converter `(entry, src, dst, 1)` (cdecl).
const CONVERT: u32 = 0x0057_88f0;
/// The scratch block holder's release (thiscall on `{count, block}`).
const RELEASE: u32 = 0x0057_7950;
/// The elements' swap `(p, count, 1)` (thiscall on the type or class).
const SWAP: u32 = 0x0043_d650;
/// `PClassDescriptor::applyDefaultBufferWithValidation(p, count)`
/// (thiscall).
const APPLY_DEFAULTS: u32 = 0x0043_c160;
/// The CRT's `memcpy` (cdecl) and its `memmove` import slot.
const MEMCPY: u32 = 0x0094_9256;
const MEMMOVE: u32 = 0x00b0_c34c;
/// `PhyrePrintf(level, format, ...)` and the messages.
const PRINTF: u32 = 0x0043_52f0;
const HEADER_LARGER: u32 = 0x00b3_1088;
const CLASS_LARGER: u32 = 0x00b3_10e0;
const NO_RUNTIME: u32 = 0x00b3_1130;

/// The errors.
const NO_BLOCK: u32 = 0xd;
const NO_ENTRY: u32 = 0x13;
const NO_CLASS_ENTRY: u32 = 6;
const TOO_LARGE: u32 = 9;

/// The ports over game memory.
pub mod bridge {
    use super::{
        APPLY_DEFAULTS, CLASS_LARGER, CONVERT, Conv, FREE, HEADER_LARGER, Host, MALLOC,
        MAPPING_ENTRY, MEMCPY, MEMMOVE, NO_BLOCK, NO_CLASS_ENTRY, NO_ENTRY, NO_RUNTIME, PRINTF,
        READ, READ_EXACT, RELEASE, SWAP, SWAP_RECORD, TOO_LARGE,
    };

    /// The frame's bytes (`ebp - 0x74` to `ebp - 0x24`): the mapping
    /// lookups' outputs and the scratch block holder.
    pub const FRAME: u32 = 0x50;
    /// Where `ebp` is in it.
    const EBP: u32 = 0x74;

    /// `PhyrePrintf(1, f, args...)`.
    fn print(h: &mut impl Host, f: u32, args: &[u32]) {
        let f = h.rebase(f);
        let mut words = [0u32; 5];
        words[0] = 1;
        words[1] = f;
        let n = args.len().min(3);
        words[2..2 + n].copy_from_slice(&args[..n]);
        h.call_sym(PRINTF, Conv::Cdecl, &words[..2 + n]);
    }

    /// A class entry stored in another layout: its `+0x14` neither 0 nor
    /// bit 31 alone.
    fn converted(h: &mut impl Host, entry: u32) -> bool {
        let w: u32 = h.read(entry + 0x14);
        w != 0 && w != 0x8000_0000
    }

    /// The file's size of a class entry (its record's `+4`, 28 bits).
    fn file_size(h: &mut impl Host, entry: u32) -> u32 {
        let record: u32 = h.read(entry + 4);
        h.read::<u32>(record + 4) & 0x0fff_ffff
    }

    /// `memmove(dst, src, n)` through the CRT import.
    fn memmove(h: &mut impl Host, dst: u32, src: u32, n: u32) {
        let slot = h.rebase(MEMMOVE);
        let f: u32 = h.read(slot);
        h.call(f, Conv::Cdecl, &[dst, src, n]);
    }

    /// `PClusterReaderBinary::loadAndFixHeaderClasses(stream)`: 0 or the
    /// first error. `frame` is [`FRAME`] bytes at the original's `ebp -
    /// 0x74`.
    pub fn load_header_classes(h: &mut impl Host, reader: u32, stream: u32, frame: u32) -> u32 {
        let ebp = frame + EBP;
        let header: u32 = h.read(reader + 8);
        let lists: u32 = h.read(header + 0x3c);
        let counts = if lists == 0 {
            0
        } else {
            h.call_sym(MALLOC, Conv::Cdecl, &[lists.wrapping_mul(4), 4])
        };
        let counts_owned = if counts == 0 { 0 } else { lists };
        if lists != 0 {
            if counts == 0 {
                return NO_BLOCK;
            }
            let bytes = lists.wrapping_mul(4);
            h.call_sym(READ, Conv::Thiscall(stream), &[counts, bytes]);
            if h.read::<u8>(reader + 0xc) != 0 {
                let end = counts.wrapping_add(bytes);
                let mut at = counts;
                while at < end {
                    let w: u32 = h.read(at);
                    h.write(at, w.swap_bytes());
                    at += 4;
                }
            }
        }
        let header: u32 = h.read(reader + 8);
        let children: u32 = h.read(header + 0x40);
        let records = if children == 0 {
            0
        } else {
            h.call_sym(MALLOC, Conv::Cdecl, &[children << 4, 4])
        };
        let records_owned = if records == 0 { 0 } else { children };
        let r = if children != 0 && records == 0 {
            NO_BLOCK
        } else {
            let swap = h.read::<u8>(reader + 0xc) != 0;
            let r = if children != 0 && records != 0 {
                h.call_sym(
                    READ_EXACT,
                    Conv::Thiscall(stream),
                    &[records, children << 4],
                )
            } else {
                0
            };
            if records != 0 && swap {
                for k in 0..children {
                    h.call_sym(SWAP_RECORD, Conv::Cdecl, &[records.wrapping_add(k << 4)]);
                }
            }
            if r != 0 || lists == 0 {
                r
            } else {
                let pass = Pass { reader, ebp };
                pass.run(h, counts, records)
            }
        };
        if records_owned >> 31 == 0 && records != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[records]);
        }
        if counts_owned >> 31 == 0 && counts != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[counts]);
        }
        r
    }

    /// The per-list pass.
    struct Pass {
        reader: u32,
        ebp: u32,
    }

    impl Pass {
        /// The mapping's entry `i` into the frame words at `ebp - ty` and
        /// `ebp - class` (cleared first); `al`.
        fn entry(&self, h: &mut impl Host, i: u32, ty: u32, class: u32) -> bool {
            let ebp = self.ebp;
            h.write(ebp - class, 0u32);
            h.write(ebp - ty, 0u32);
            let mapping: u32 = h.read(self.reader + 4);
            h.call_sym(
                MAPPING_ENTRY,
                Conv::Thiscall(mapping),
                &[i, ebp - ty, ebp - class],
            ) & 0xff
                != 0
        }

        /// Every header list: 0 or the first error.
        fn run(&self, h: &mut impl Host, counts: u32, records: u32) -> u32 {
            let reader = self.reader;
            let entries: u32 = h.read(reader + 0x24);
            let header: u32 = h.read(reader + 8);
            let n: u32 = h.read(header + 0x10);
            let count = h.read::<u32>(reader + 0x18) & 0x7fff_ffff;
            let lists = match count {
                0 => 0,
                1 => reader + 0x1c,
                _ => h.read(reader + 0x1c),
            };
            let mut record = records;
            let mut count_at = counts;
            for i in 0..n {
                let list: u32 = h.read(lists.wrapping_add(4 * i));
                if list == 0 {
                    return 0;
                }
                let class: u32 = h.read(list + 0x2c);
                let flags = h.read::<u32>(class + 0x90) & 0x07ff_ffff;
                if flags & 4 == 0 {
                    continue;
                }
                let entry: u32 = h.read(entries.wrapping_add(4 * i));
                if entry == 0 {
                    return NO_CLASS_ENTRY;
                }
                let size: u32 = h.read(class + 0x1c);
                let k: u32 = h.read(count_at);
                count_at = count_at.wrapping_add(4);
                let mut stored = converted(h, entry);
                let file = file_size(h, entry);
                let end = record.wrapping_add(k << 4);
                let mut larger = size > file;
                if larger {
                    let name: u32 = h.read(class + 0x18);
                    print(h, HEADER_LARGER, &[name, size, file]);
                }
                let mut at = record;
                while at < end {
                    let i: u32 = h.read(at);
                    if !self.entry(h, i, 0x50, 0x30) {
                        return NO_ENTRY;
                    }
                    let child: u32 = h.read(self.ebp - 0x30);
                    if child != 0 {
                        let desc: u32 = h.read(child + 8);
                        if desc != 0 {
                            stored |= converted(h, child);
                            let runtime: u32 = h.read(desc + 0x1c);
                            let file = file_size(h, child);
                            if runtime > file {
                                let name: u32 = h.read(desc + 0x18);
                                print(h, CLASS_LARGER, &[name, runtime, file]);
                                larger = true;
                            }
                        }
                    }
                    at = at.wrapping_add(0x10);
                }
                if larger {
                    return TOO_LARGE;
                }
                let aligned = stored && (flags >> 4) & 1 != 0;
                let base: u32 = h.read(list + 0x30);
                let mut dst = size.wrapping_add(base);
                while record < end {
                    if h.read::<u8>(record + 8) & 2 == 0 {
                        match self.child(h, record, base, dst, aligned) {
                            Ok(next) => dst = next,
                            Err(r) => return r,
                        }
                    }
                    record = record.wrapping_add(0x10);
                }
            }
            0
        }

        /// One child record: the packing cursor after it, or the error.
        fn child(
            &self,
            h: &mut impl Host,
            record: u32,
            base: u32,
            dst: u32,
            aligned: bool,
        ) -> Result<u32, u32> {
            let ebp = self.ebp;
            let i: u32 = h.read(record);
            self.entry(h, i, 0x54, 0x28);
            let mut src = h.read::<u32>(record + 4).wrapping_add(base);
            let count: u32 = h.read(record + 0xc);
            let swap_at = self.reader + 0xc;
            let class: u32 = h.read(ebp - 0x28);
            if class == 0 {
                // a plain type
                let ty_entry: u32 = h.read(ebp - 0x54);
                if ty_entry == 0 {
                    return Ok(dst);
                }
                let ty: u32 = h.read(ty_entry + 4);
                if ty == 0 {
                    return Ok(dst);
                }
                let mut dst = dst;
                if aligned {
                    let size = h.read::<u32>(ty + 0x1c).wrapping_mul(count);
                    let align: u32 = h.read(ty + 0x20);
                    dst = dst.wrapping_add(align.wrapping_sub(1) & dst.wrapping_neg());
                    if dst != src {
                        memmove(h, dst, src, size);
                        src = dst;
                    }
                    dst = dst.wrapping_add(size);
                }
                if h.read::<u8>(swap_at) != 0 {
                    h.call_sym(SWAP, Conv::Thiscall(ty), &[src, count, 1]);
                }
                return Ok(dst);
            }
            let desc: u32 = h.read(class + 8);
            if desc == 0 {
                let name: u32 = h.read(class + 0xc);
                print(h, NO_RUNTIME, &[name]);
                return Ok(dst);
            }
            let mut dst = if aligned {
                let align: u32 = h.read(desc + 0x20);
                dst.wrapping_add(align.wrapping_sub(1) & dst.wrapping_neg())
            } else {
                src
            };
            h.write(ebp - 0x50, dst);
            let size: u32 = h.read(desc + 0x1c);
            if converted(h, class) {
                // element by element through a scratch block
                let file = file_size(h, class);
                h.write(ebp - 0x30, 0u32);
                let scratch = if file == 0 {
                    0
                } else {
                    h.call_sym(MALLOC, Conv::Cdecl, &[file, 0x10])
                };
                h.write(ebp - 0x30, scratch);
                let owned = if scratch == 0 { 0 } else { file };
                h.write(ebp - 0x74, owned);
                h.write(ebp - 0x70, scratch);
                if scratch == 0 {
                    h.call_sym(RELEASE, Conv::Thiscall(ebp - 0x74), &[]);
                    return Err(NO_BLOCK);
                }
                let mut out = dst;
                for _ in 0..count {
                    h.call_sym(MEMCPY, Conv::Cdecl, &[scratch, src, file]);
                    h.call_sym(CONVERT, Conv::Cdecl, &[class, scratch, out, 1]);
                    out = out.wrapping_add(size);
                    src = src.wrapping_add(file);
                }
                if owned >> 31 == 0 {
                    h.call_sym(FREE, Conv::Cdecl, &[scratch]);
                }
                h.write(ebp - 0x70, 0u32);
                h.write(ebp - 0x74, 0u32);
            } else if aligned && dst != src {
                memmove(h, dst, src, size.wrapping_mul(count));
            }
            let at = h.read::<u32>(desc + 0x8c).wrapping_add(dst);
            if h.read::<u8>(swap_at) != 0 {
                h.call_sym(SWAP, Conv::Thiscall(desc), &[at, count, 1]);
            }
            h.call_sym(APPLY_DEFAULTS, Conv::Thiscall(desc), &[at, count]);
            if aligned {
                let size: u32 = h.read(desc + 0x1c);
                dst = dst.wrapping_add(size.wrapping_mul(count));
            }
            Ok(dst)
        }
    }
}
