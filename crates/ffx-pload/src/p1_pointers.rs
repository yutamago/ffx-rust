//! List `p1-load`, round 4: the cluster reader's array fixups
//! (`PClusterReaderBinary::fixupDataPointers`, `0x0057a320`, Fahrenheit's
//! name from the Switch build) and pointer-array fixups
//! (`Cluster_ApplyPointerArrayFixups`, `0x0057acd0`).
//!
//! The reader (`r`): `+4` the namespace mapping (`+0x1c` its mismatch
//! flags), `+8` the cluster header (`+0x10` the instance-list count,
//! `+0x14` the fixup data's size, `+0x18` the array-fixup count), `+0xc`
//! whether the file's words need swapping, `+0x10` the fixup lists,
//! `+0x14` the instance-list headers (0x24 bytes each: `+0` the list's
//! data offset, `+0x18` its array-fixup count), `+0x18`/`+0x1c` the
//! instance lists (a count, then the one list inline or the array's
//! address), `+0x24` the lists' class entries.
//!
//! An array fixup (16 bytes, unpacked by `Cluster_UnpackArrayFixups`):
//! `+0` the data member (an index, or with bit 31 a byte offset in the
//! object), `+4` the object's index in its list, `+8` the element count,
//! `+0xc` the array's offset in the list's array data.

use ffx_graphic::host::{Conv, Host};

/// `PhyreMalloc(size, align)` and `PhyreFree(p)` (cdecl).
const MALLOC: u32 = 0x0042_fb80;
const FREE: u32 = 0x0042_fb20;
/// `PStreamReader::readExact(buffer, size)` (thiscall on the stream).
const READ_EXACT: u32 = 0x0057_dd30;
/// `Cluster_UnpackArrayFixups(&out, &cursor, lists, pointer_arrays)`
/// (cdecl).
const UNPACK_ARRAYS: u32 = 0x0060_dac0;
/// `PInstanceList::instanceData(i)` (thiscall on the list).
const INSTANCE_DATA: u32 = 0x0057_65e0;
/// `Fixup_FindMember(index, &object, offset, &out, class_entry)`
/// (thiscall on the lookup cache, `ret 0x14`).
const FIND_MEMBER: u32 = 0x0057_a280;
/// `PClassDataMember::getDynamicType(object)` (thiscall on the member).
const DYNAMIC_TYPE: u32 = 0x0057_5690;
/// The element swap `(array, count, 1)` (thiscall on the dynamic type).
const SWAP: u32 = 0x0043_d650;
/// `PhyrePrintf(level, format, ...)` and the messages.
const PRINTF: u32 = 0x0043_52f0;
const NO_MEMBER: u32 = 0x00b3_1188;
const NO_ARRAY: u32 = 0x00b3_1300;
const NO_POINTER_ARRAY: u32 = 0x00b3_11d8;
const EXTERNAL_FAILED: u32 = 0x00b3_1220;
/// The CRT's `memset(p, c, n)` (cdecl).
const MEMSET: u32 = 0x0094_964c;

/// The no-memory error, and an object not in its list.
const NO_BLOCK: u32 = 0xd;
const NO_OBJECT: u32 = 0x13;

/// The ports over game memory.
pub mod bridge {
    use super::{
        Conv, DYNAMIC_TYPE, EXTERNAL_FAILED, FIND_MEMBER, FREE, Host, INSTANCE_DATA, MALLOC,
        MEMSET, NO_ARRAY, NO_BLOCK, NO_MEMBER, NO_OBJECT, NO_POINTER_ARRAY, PRINTF, READ_EXACT,
        SWAP, UNPACK_ARRAYS,
    };

    /// The bytes of the frame the port keeps at the original's offsets:
    /// from `ebp - 0x8c` (the member lookup's cache) up to and with the
    /// argument word at `ebp + 8` (the object, whose address the lookup
    /// gets).
    pub const FRAME: u32 = 0x98;
    /// Where `ebp` is in it.
    const EBP: u32 = 0x8c;

    /// `PClusterReaderBinary::fixupDataPointers(stream)` (thiscall on the
    /// reader, `ret 4`): reads the fixup data (`readExact`), unpacks the
    /// array fixups into a block of their own, and points each array
    /// member of each object at its data: the list's array data (the
    /// list's data offset plus, on files of an aligned layout, its
    /// objects' size, aligned to 16) plus the fixup's offset, through the
    /// member's slot 8 `(object, array, count)` (the member's word is 0
    /// unless that succeeds); the elements swapped first on a swapping
    /// file whose type's slot 1 says so. 0, or the first error: 0xd
    /// without a block, the stream's or the unpacking's, 0x13 for an
    /// object not in its list. `frame` is [`FRAME`] bytes at the
    /// original's `ebp - 0x8c`.
    pub fn fixup_data_pointers(h: &mut impl Host, reader: u32, stream: u32, frame: u32) -> u32 {
        let ebp = frame + EBP;
        let headers: u32 = h.read(reader + 0x14);
        let n = h.read::<u32>(reader + 0x18) & 0x7fff_ffff;
        let first = match n {
            0 => 0,
            1 => reader + 0x1c,
            _ => h.read(reader + 0x1c),
        };
        let header: u32 = h.read(reader + 8);
        let lists: u32 = h.read(header + 0x10);
        let end = first.wrapping_add(lists.wrapping_mul(4));
        let class_entries: u32 = h.read(reader + 0x24);
        let mapping: u32 = h.read(reader + 4);
        let plain = h.read::<u32>(mapping + 0x1c) & 0x8000_0005 == 0;
        let fixups: u32 = h.read(header + 0x18);
        h.write(ebp - 0x60, 0u32);
        let records = if fixups == 0 {
            0
        } else {
            h.call_sym(MALLOC, Conv::Cdecl, &[fixups << 4, 4])
        };
        h.write(ebp - 0x5c, records);
        h.write(ebp - 0x60, if records == 0 { 0 } else { fixups });
        let header: u32 = h.read(reader + 8);
        let size: u32 = h.read(header + 0x14);
        let data = if size == 0 {
            0
        } else {
            h.call_sym(MALLOC, Conv::Cdecl, &[size, 1])
        };
        let data_size = if data == 0 { 0 } else { size };
        h.write(ebp - 0x44, data);
        let r = if fixups != 0 && (records == 0 || data == 0) {
            NO_BLOCK
        } else {
            let r = h.call_sym(READ_EXACT, Conv::Thiscall(stream), &[data, size]);
            if r == 0 {
                let r = h.call_sym(
                    UNPACK_ARRAYS,
                    Conv::Cdecl,
                    &[ebp - 0x60, ebp - 0x44, reader + 0x10, 0],
                );
                if r == 0 {
                    let lookup = Lookup { reader, ebp, plain };
                    lookup.apply(h, records, (first, end), headers, class_entries)
                } else {
                    r
                }
            } else {
                r
            }
        };
        // the blocks freed when owned (bit 31 of their counts clear); the
        // records' count and block as the unpacking left them
        if data_size >> 31 == 0 && data != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        let count: u32 = h.read(ebp - 0x60);
        let block: u32 = h.read(ebp - 0x5c);
        if count >> 31 == 0 && block != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[block]);
        }
        r
    }

    /// What the per-object pass needs.
    struct Lookup {
        reader: u32,
        ebp: u32,
        /// The mapping's flags have none of bits 0, 2 and 31.
        plain: bool,
    }

    impl Lookup {
        /// Every list's fixups, from `records` on: 0, or 0x13.
        fn apply(
            &self,
            h: &mut impl Host,
            records: u32,
            (first, end): (u32, u32),
            headers: u32,
            class_entries: u32,
        ) -> u32 {
            let ebp = self.ebp;
            // the member lookup's cache: the mapping, then 8 words of 0
            let mapping: u32 = h.read(self.reader + 4);
            h.write(ebp - 0x8c, mapping);
            for k in 1..9 {
                h.write(ebp - 0x8c + 4 * k, 0u32);
            }
            let mut record = records;
            let mut list_at = first;
            let mut header = headers.wrapping_add(0xc);
            let mut entry_at = class_entries;
            while list_at < end {
                let count: u32 = h.read(header + 0xc);
                if count != 0 {
                    let list: u32 = h.read(list_at);
                    if list == 0 {
                        record = record.wrapping_add(count << 4);
                    } else {
                        let entry: u32 = h.read(entry_at);
                        let base = self.array_base(h, list, entry, header);
                        for _ in 0..count {
                            if !self.one(h, record, list, entry, base) {
                                return NO_OBJECT;
                            }
                            record = record.wrapping_add(0x10);
                        }
                    }
                }
                list_at = list_at.wrapping_add(4);
                header = header.wrapping_add(0x24);
                entry_at = entry_at.wrapping_add(4);
            }
            0
        }

        /// Where a list's array data starts: after its objects (aligned
        /// to 16) on an aligned layout, else at its header's offset.
        fn array_base(&self, h: &mut impl Host, list: u32, entry: u32, header: u32) -> u32 {
            let class: u32 = h.read(list + 0x2c);
            let data: u32 = h.read(list + 0x34);
            if !self.plain && h.read::<u32>(entry + 0x1c) != 0 {
                let size: u32 = h.read(class + 0x1c);
                let objects: u32 = h.read(list + 0x3c);
                let base = data.wrapping_add(size.wrapping_mul(objects));
                base.wrapping_add(base.wrapping_neg() & 0xf)
            } else {
                let offset: u32 = h.read(header);
                data.wrapping_add(offset)
            }
        }

        /// One fixup: false when its object is not in the list.
        fn one(&self, h: &mut impl Host, record: u32, list: u32, entry: u32, base: u32) -> bool {
            let ebp = self.ebp;
            let class: u32 = h.read(list + 0x2c);
            let elements: u32 = h.read(record + 8);
            let index: u32 = h.read(record + 4);
            let object = h.call_sym(INSTANCE_DATA, Conv::Thiscall(list), &[index]);
            h.write(ebp + 8, object);
            if object == 0 {
                return false;
            }
            let word: u32 = h.read(record);
            let (member_index, offset) = if word >> 31 == 0 {
                (word, 0)
            } else {
                (u32::MAX, word & 0x7fff_ffff)
            };
            h.write(ebp - 0x4c, 0u32);
            let found = h.call_sym(
                FIND_MEMBER,
                Conv::Thiscall(ebp - 0x8c),
                &[member_index, ebp + 8, offset, ebp - 0x4c, entry],
            );
            if found == 0 {
                let name: u32 = h.read(class + 0x18);
                let size: u32 = h.read(class + 0x1c);
                print(h, NO_MEMBER, &[name, offset, size]);
                return true;
            }
            let member: u32 = h.read(found + 8);
            if member == 0 {
                let name: u32 = h.read(class + 0x18);
                let member_name: u32 = h.read(found + 0xc);
                print(h, NO_ARRAY, &[name, member_name]);
                return true;
            }
            let owner: u32 = h.read(member + 0xc);
            let delta = h
                .read::<u32>(member + 0x24)
                .wrapping_sub(h.read::<u32>(owner + 0x88));
            let object: u32 = h.read(ebp + 8);
            let array = h.read::<u32>(record + 0xc).wrapping_add(base);
            let slot = delta.wrapping_add(object);
            let dynamic = h.call_sym(DYNAMIC_TYPE, Conv::Thiscall(member), &[object]);
            let in_place = h.call_virtual(dynamic, 1, &[]);
            if h.read::<u8>(self.reader + 0xc) != 0 && in_place == 0 {
                h.call_sym(SWAP, Conv::Thiscall(dynamic), &[array, elements, 1]);
            }
            h.write(slot, 0u32);
            let object: u32 = h.read(ebp + 8);
            if h.call_virtual(member, 8, &[object, array, elements]) != 0 {
                h.write(slot, array);
            }
            true
        }
    }

    /// The bytes of `Cluster_ApplyPointerArrayFixups`'s frame the port
    /// keeps at the original's offsets: from `ebp - 0x7c` (the member
    /// lookup's cache) up to the object word at `ebp - 0x18`.
    pub const ARRAYS_FRAME: u32 = 0x68;
    /// Where `ebp` is in it.
    const ARRAYS_EBP: u32 = 0x7c;

    /// The instance lists `[first, end)` of the reader.
    fn lists(h: &mut impl Host, reader: u32) -> (u32, u32) {
        let n = h.read::<u32>(reader + 0x18) & 0x7fff_ffff;
        let first = match n {
            0 => 0,
            1 => reader + 0x1c,
            _ => h.read(reader + 0x1c),
        };
        let header: u32 = h.read(reader + 8);
        let lists: u32 = h.read(header + 0x10);
        (first, first.wrapping_add(lists.wrapping_mul(4)))
    }

    /// `Cluster_ApplyPointerArrayFixups(stream, array)` (`0x0057acd0`,
    /// thiscall on the reader, `ret 8`): reads the fixup data (the
    /// header's `+0x24` bytes), unpacks the pointer-array fixups (the
    /// header's `+0x28`, `Cluster_UnpackArrayFixups(..., 1)`), and points
    /// each pointer-array member at its storage through the member's slot
    /// 8 `(object, storage, count)` (the member's word cleared first; a
    /// failure printed), the storage moving on by the count's words. The
    /// storage starts at `array`, or (on a file of a non-plain mapping)
    /// per list after its objects and array data, cleared (`memset`) for
    /// the list's pointer words. 0, or as [`fixup_data_pointers`].
    /// `frame` is [`ARRAYS_FRAME`] bytes at the original's `ebp - 0x7c`.
    pub fn fixup_pointer_arrays(
        h: &mut impl Host,
        reader: u32,
        stream: u32,
        array: u32,
        frame: u32,
    ) -> u32 {
        let ebp = frame + ARRAYS_EBP;
        let headers: u32 = h.read(reader + 0x14);
        let (first, end) = lists(h, reader);
        let header: u32 = h.read(reader + 8);
        let fixups: u32 = h.read(header + 0x28);
        let entries: u32 = h.read(reader + 0x24);
        let mapping: u32 = h.read(reader + 4);
        h.write(ebp - 0x7c, mapping);
        let plain = h.read::<u32>(mapping + 0x1c) & 0x8000_0005 == 0;
        let size: u32 = h.read(header + 0x24);
        for k in 1..9 {
            h.write(ebp - 0x7c + 4 * k, 0u32);
        }
        let data = if size == 0 {
            0
        } else {
            h.call_sym(MALLOC, Conv::Cdecl, &[size, 1])
        };
        let data_size = if data == 0 { 0 } else { size };
        h.write(ebp - 0x2c, data);
        h.write(ebp - 0x50, 0u32);
        let records = if fixups == 0 {
            0
        } else {
            h.call_sym(MALLOC, Conv::Cdecl, &[fixups << 4, 4])
        };
        h.write(ebp - 0x4c, records);
        h.write(ebp - 0x50, if records == 0 { 0 } else { fixups });
        let r = if fixups != 0 && (records == 0 || data == 0) {
            NO_BLOCK
        } else {
            match h.call_sym(READ_EXACT, Conv::Thiscall(stream), &[data, size]) {
                0 => match h.call_sym(
                    UNPACK_ARRAYS,
                    Conv::Cdecl,
                    &[ebp - 0x50, ebp - 0x2c, reader + 0x10, 1],
                ) {
                    0 => {
                        let pass = Arrays { ebp, plain };
                        pass.apply(h, records, (first, end), (headers, entries), array)
                    }
                    r => r,
                },
                r => r,
            }
        };
        let count: u32 = h.read(ebp - 0x50);
        let block: u32 = h.read(ebp - 0x4c);
        if count >> 31 == 0 && block != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[block]);
        }
        h.write(ebp - 0x4c, 0u32);
        h.write(ebp - 0x50, 0u32);
        if data_size >> 31 == 0 && data != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        r
    }

    /// What the pointer-array pass needs.
    struct Arrays {
        ebp: u32,
        /// The mapping's flags have none of bits 0, 2 and 31.
        plain: bool,
    }

    impl Arrays {
        /// Every list's pointer-array fixups: 0, or 0x13.
        fn apply(
            &self,
            h: &mut impl Host,
            records: u32,
            (first, end): (u32, u32),
            (headers, entries): (u32, u32),
            array: u32,
        ) -> u32 {
            let mut record = records;
            let mut storage = array;
            let mut header = headers.wrapping_add(0x10);
            let mut list_at = first;
            let mut entry_at = entries;
            while list_at < end {
                let count: u32 = h.read(header + 0x10);
                if count != 0 {
                    let list: u32 = h.read(list_at);
                    if list == 0 {
                        record = record.wrapping_add(count << 4);
                    } else {
                        let class: u32 = h.read(list + 0x2c);
                        let entry: u32 = h.read(entry_at);
                        if !self.plain {
                            storage = Self::storage(h, list, class, entry, header);
                            let words: u32 = h.read(header + 4);
                            h.call_sym(MEMSET, Conv::Cdecl, &[storage, 0, words << 2]);
                        }
                        for _ in 0..count {
                            match self.one(h, record, list, (class, entry), storage) {
                                Some(next) => storage = next,
                                None => return NO_OBJECT,
                            }
                            record = record.wrapping_add(0x10);
                        }
                    }
                }
                list_at = list_at.wrapping_add(4);
                header = header.wrapping_add(0x24);
                entry_at = entry_at.wrapping_add(4);
            }
            0
        }

        /// A list's pointer storage on a non-plain file: after its
        /// objects (aligned to 16) and its array data (rounded to 4)
        /// when the class entry says the layout is aligned, else after
        /// its header's offset, aligned to 4.
        fn storage(h: &mut impl Host, list: u32, class: u32, entry: u32, header: u32) -> u32 {
            let size: u32 = h.read(class + 0x1c);
            let objects: u32 = h.read(list + 0x3c);
            let data: u32 = h.read(list + 0x34);
            if h.read::<u32>(entry + 0x1c) != 0 {
                let base = size.wrapping_mul(objects).wrapping_add(data);
                let arrays: u32 = h.read(header);
                (arrays.wrapping_add(3) & !3)
                    .wrapping_add(base.wrapping_neg() & 0xf)
                    .wrapping_add(base)
            } else {
                let offset: u32 = h.read(header - 8);
                let at = offset.wrapping_add(data);
                (at.wrapping_neg() & 3).wrapping_add(at)
            }
        }

        /// One fixup: the storage after it, or `None` when its object is
        /// not in the list.
        fn one(
            &self,
            h: &mut impl Host,
            record: u32,
            list: u32,
            (class, entry): (u32, u32),
            storage: u32,
        ) -> Option<u32> {
            let ebp = self.ebp;
            let elements: u32 = h.read(record + 8);
            let index: u32 = h.read(record + 4);
            let object = h.call_sym(INSTANCE_DATA, Conv::Thiscall(list), &[index]);
            h.write(ebp - 0x18, object);
            if object == 0 {
                return None;
            }
            let word: u32 = h.read(record);
            let (member_index, offset) = if word >> 31 == 0 {
                (word, 0)
            } else {
                (u32::MAX, word & 0x7fff_ffff)
            };
            h.write(ebp - 0x44, 0u32);
            let found = h.call_sym(
                FIND_MEMBER,
                Conv::Thiscall(ebp - 0x7c),
                &[member_index, ebp - 0x18, offset, ebp - 0x44, entry],
            );
            if found == 0 {
                let name: u32 = h.read(class + 0x18);
                let size: u32 = h.read(class + 0x1c);
                print(h, NO_MEMBER, &[name, offset, size]);
                return Some(storage);
            }
            let member: u32 = h.read(found + 8);
            if member == 0 {
                let name: u32 = h.read(class + 0x18);
                let member_name: u32 = h.read(found + 0xc);
                print(h, NO_POINTER_ARRAY, &[name, member_name]);
                return Some(storage);
            }
            let owner: u32 = h.read(member + 0xc);
            let delta = h
                .read::<u32>(member + 0x24)
                .wrapping_sub(h.read::<u32>(owner + 0x88));
            let object: u32 = h.read(ebp - 0x18);
            h.write(delta.wrapping_add(object), 0u32);
            let object: u32 = h.read(ebp - 0x18);
            if h.call_virtual(member, 8, &[object, storage, elements]) != 0 {
                let name: u32 = h.read(class + 0x18);
                let member_name: u32 = h.read(found + 0xc);
                print(h, EXTERNAL_FAILED, &[name, member_name]);
            }
            let step = if storage == 0 { 0 } else { elements };
            Some(storage.wrapping_add(step.wrapping_mul(4)))
        }
    }

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
}
