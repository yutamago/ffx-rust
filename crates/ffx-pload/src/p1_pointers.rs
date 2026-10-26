//! List `p1-load`, round 4: the cluster reader's array fixups
//! (`PClusterReaderBinary::fixupDataPointers`, `0x0057a320`, Fahrenheit's
//! name from the Switch build) and pointer-array fixups
//! (`Cluster_ApplyPointerArrayFixups`, `0x0057acd0`) and pointer fixups
//! (`Cluster_ApplyPointerFixups`, `0x0057a6b0`).
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
/// `Cluster_UnpackPointerFixups(&out, &cursor, lists)` (cdecl).
const UNPACK_POINTERS: u32 = 0x0060_db20;
/// `PTypeDispenser::GetType<PType>()` (cdecl).
const PTYPE_TYPE: u32 = 0x0043_74d0;
/// The class test `(class)` (thiscall on a class descriptor): `al`.
const IS_KIND_OF: u32 = 0x0043_1420;
/// The target lookup `(&target, offset, class_entry, class)` (thiscall
/// on its cache).
const FIND_TARGET: u32 = 0x0057_a200;
/// `PType::resolveFixupForAddress(at, data, len, ctx)` (thiscall).
const RESOLVE: u32 = 0x0043_7980;
const NO_POINTER_MEMBER: u32 = 0x00b3_1268;
const NO_POINTER_TARGET: u32 = 0x00b3_12c0;

/// The no-memory error, and an object not in its list.
const NO_BLOCK: u32 = 0xd;
const NO_OBJECT: u32 = 0x13;

/// The ports over game memory.
pub mod bridge {
    use super::{
        Conv, DYNAMIC_TYPE, EXTERNAL_FAILED, FIND_MEMBER, FIND_TARGET, FREE, Host, INSTANCE_DATA,
        IS_KIND_OF, MALLOC, MEMSET, NO_ARRAY, NO_BLOCK, NO_MEMBER, NO_OBJECT, NO_POINTER_ARRAY,
        NO_POINTER_MEMBER, NO_POINTER_TARGET, PRINTF, PTYPE_TYPE, READ_EXACT, RESOLVE, SWAP,
        UNPACK_ARRAYS, UNPACK_POINTERS,
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

    /// The bytes of `Cluster_ApplyPointerFixups`' frame the port keeps
    /// at the original's offsets: from `ebp - 0xc8` (the two lookup
    /// caches) up to the object word at `ebp - 0x20`.
    pub const POINTERS_FRAME: u32 = 0xac;
    /// Where `ebp` is in it.
    const POINTERS_EBP: u32 = 0xc8;

    /// The instance list `index` of the reader (no bounds check, as the
    /// original's first use).
    fn list_at(h: &mut impl Host, reader: u32, index: u32) -> u32 {
        let n = h.read::<u32>(reader + 0x18) & 0x7fff_ffff;
        let first = match n {
            0 => 0,
            1 => reader + 0x1c,
            _ => h.read(reader + 0x1c),
        };
        h.read(first.wrapping_add(index.wrapping_mul(4)))
    }

    /// `Cluster_ApplyPointerFixups(stream, ctx, &fixups)` (`0x0057a6b0`,
    /// thiscall on the reader, `ret 0xc`): reads the fixup data (the
    /// header's `+0x1c` bytes), unpacks the pointer fixups (the header's
    /// `+0x20`, 28 bytes each, `Cluster_UnpackPointerFixups`), and
    /// writes each pointer of each object: a user fixup (index below the
    /// header's `+0x30`: its value, or for an import its resolver
    /// `PType::resolveFixupForAddress` with `ctx`), else the target
    /// object's address (found through `0x0057a200` for a class the
    /// target's class doesn't already match, less the class's `+0x88`)
    /// or a list's data plus the fixup's offset. Pointer arrays of kind
    /// 2 go through the member's slot 6 (an element) or slot 8 (the
    /// whole array). 0, or as [`fixup_data_pointers`]. `frame` is
    /// [`POINTERS_FRAME`] bytes at the original's `ebp - 0xc8`.
    pub fn fixup_pointers(
        h: &mut impl Host,
        reader: u32,
        (stream, ctx, fixups): (u32, u32, u32),
        frame: u32,
    ) -> u32 {
        let ebp = frame + POINTERS_EBP;
        let (first, end) = lists(h, reader);
        let header: u32 = h.read(reader + 8);
        let count: u32 = h.read(header + 0x20);
        let users: u32 = h.read(header + 0x30);
        let headers: u32 = h.read(reader + 0x14);
        let entries: u32 = h.read(reader + 0x24);
        let mapping: u32 = h.read(reader + 4);
        let plain = h.read::<u32>(mapping + 0x1c) & 0x8000_0005 == 0;
        for cache in [0xc8, 0xa4] {
            h.write(ebp - cache, mapping);
            for k in 1..9 {
                h.write(ebp - cache + 4 * k, 0u32);
            }
        }
        h.write(ebp - 0x50, 0u32);
        let records = if count == 0 {
            0
        } else {
            h.call_sym(MALLOC, Conv::Cdecl, &[count.wrapping_mul(28), 4])
        };
        h.write(ebp - 0x4c, records);
        h.write(ebp - 0x50, if records == 0 { 0 } else { count });
        let header: u32 = h.read(reader + 8);
        let size: u32 = h.read(header + 0x1c);
        let data = if size == 0 {
            0
        } else {
            h.call_sym(MALLOC, Conv::Cdecl, &[size, 1])
        };
        let data_size = if data == 0 { 0 } else { size };
        h.write(ebp - 0x70, data);
        let r = if count != 0 && (records == 0 || data == 0) {
            NO_BLOCK
        } else {
            match h.call_sym(READ_EXACT, Conv::Thiscall(stream), &[data, size]) {
                0 => match h.call_sym(
                    UNPACK_POINTERS,
                    Conv::Cdecl,
                    &[ebp - 0x50, ebp - 0x70, reader + 0x10],
                ) {
                    0 => {
                        let pass = Pointers {
                            reader,
                            ebp,
                            plain,
                            users,
                            ctx,
                            fixups,
                            headers,
                        };
                        pass.apply(h, records, (first, end), entries)
                    }
                    r => r,
                },
                r => r,
            }
        };
        if data_size >> 31 == 0 && data != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        let count: u32 = h.read(ebp - 0x50);
        let block: u32 = h.read(ebp - 0x4c);
        if count >> 31 == 0 && block != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[block]);
        }
        r
    }

    /// What the pointer pass needs.
    struct Pointers {
        reader: u32,
        ebp: u32,
        plain: bool,
        /// The user fixup count (the header's `+0x30`).
        users: u32,
        ctx: u32,
        /// The user fixups' results `{count, data}` (16 bytes each:
        /// value or resolver type, data, length, an import byte).
        fixups: u32,
        /// The instance-list headers (36 bytes each).
        headers: u32,
    }

    impl Pointers {
        /// Every list's pointer fixups: 0, or 0x13.
        fn apply(
            &self,
            h: &mut impl Host,
            records: u32,
            (first, end): (u32, u32),
            entries: u32,
        ) -> u32 {
            let mut record = records;
            let mut list_at = first;
            let mut header = self.headers.wrapping_add(0x1c);
            let mut entry_at = entries;
            while list_at < end {
                let count: u32 = h.read(header);
                if count != 0 {
                    let list: u32 = h.read(list_at);
                    if list == 0 {
                        record = record.wrapping_add(count.wrapping_mul(28));
                    } else {
                        let class: u32 = h.read(list + 0x2c);
                        let quiet = (h.read::<u32>(class + 0x90) >> 2) & 1 != 0;
                        let entry: u32 = h.read(entry_at);
                        let aligned = !self.plain && h.read::<u32>(entry + 0x1c) != 0;
                        for _ in 0..count {
                            let source = Source {
                                list,
                                class,
                                entry,
                                quiet,
                                aligned,
                            };
                            if !self.one(h, record, &source) {
                                return NO_OBJECT;
                            }
                            record = record.wrapping_add(28);
                        }
                    }
                }
                list_at = list_at.wrapping_add(4);
                header = header.wrapping_add(0x24);
                entry_at = entry_at.wrapping_add(4);
            }
            0
        }

        /// One pointer fixup: false for an object not in its list (or a
        /// target that isn't).
        fn one(&self, h: &mut impl Host, record: u32, s: &Source) -> bool {
            let ebp = self.ebp;
            let target_index: u32 = h.read(record + 0xc);
            let user: u32 = h.read(record + 0x18);
            let target_list = list_at(h, self.reader, target_index);
            let index: u32 = h.read(record + 4);
            let object = h.call_sym(INSTANCE_DATA, Conv::Thiscall(s.list), &[index]);
            h.write(ebp - 0x20, object);
            let target = self.target(h, record, user);
            h.write(ebp - 0x30, target);
            if object == 0 {
                return false;
            }
            let word: u32 = h.read(record);
            let (member_index, offset) = if word >> 31 == 0 {
                (word, 0)
            } else {
                (u32::MAX, word & 0x7fff_ffff)
            };
            h.write(ebp - 0x74, 0u32);
            let found = h.call_sym(
                FIND_MEMBER,
                Conv::Thiscall(ebp - 0xc8),
                &[member_index, ebp - 0x20, offset, ebp - 0x74, s.entry],
            );
            if found == 0 && !s.quiet {
                let name: u32 = h.read(s.class + 0x18);
                let size: u32 = h.read(s.class + 0x1c);
                print(h, NO_POINTER_MEMBER, &[name, offset, size]);
                return true;
            }
            if h.read::<u32>(ebp - 0x30) == 0 && user >= self.users {
                return false;
            }
            let member = if found == 0 {
                0
            } else {
                let m: u32 = h.read(found + 8);
                if m == 0 {
                    let name: u32 = h.read(s.class + 0x18);
                    let member_name: u32 = h.read(found + 0xc);
                    print(h, NO_POINTER_TARGET, &[name, member_name]);
                    return true;
                }
                m
            };
            let (value, import) = if user < self.users {
                let results: u32 = h.read(self.fixups + 4);
                let fixup = results.wrapping_add(user << 4);
                let import = h.read::<u8>(fixup + 0xc) != 0;
                (h.read(fixup), if import { fixup } else { 0 })
            } else {
                (
                    self.target_value(h, record, member, target_list, s.aligned),
                    0,
                )
            };
            self.store(h, record, member, offset, value, import);
            true
        }

        /// The target object of a record whose user index is past the
        /// user fixups, or 0.
        fn target(&self, h: &mut impl Host, record: u32, user: u32) -> u32 {
            if user < self.users {
                return 0;
            }
            let index: u32 = h.read(record + 0xc);
            let n = h.read::<u32>(self.reader + 0x18) & 0x7fff_ffff;
            if index >= n {
                return 0;
            }
            let list = list_at(h, self.reader, index);
            if list == 0 {
                return 0;
            }
            let i: u32 = h.read(record + 8);
            h.call_sym(INSTANCE_DATA, Conv::Thiscall(list), &[i])
        }

        /// The address a non-user pointer gets.
        fn target_value(
            &self,
            h: &mut impl Host,
            record: u32,
            member: u32,
            target_list: u32,
            aligned: bool,
        ) -> u32 {
            let ebp = self.ebp;
            let target_class: u32 = h.read(target_list + 0x2c);
            let offset: u32 = h.read(record + 0x10);
            // the class the member points at
            let mut class = 0;
            let mut have = false;
            if member != 0 {
                let ty: u32 = h.read(member + 0x18);
                if ty != 0 {
                    let c = h.call_virtual(ty, 1, &[]);
                    let ptype = h.call_sym(PTYPE_TYPE, Conv::Cdecl, &[]);
                    if ty != ptype || c != 0 {
                        class = c;
                        have = true;
                    }
                }
            }
            if !have {
                class = h.read(target_list + 0x2c);
            }
            if class == 0 {
                // the list's data
                let base = if aligned {
                    let data: u32 = h.read(target_list + 0x34);
                    let objects: u32 = h.read(target_list + 0x3c);
                    let size: u32 = h.read(target_class + 0x1c);
                    let at = data.wrapping_add(objects.wrapping_mul(size));
                    (at.wrapping_neg() & 0xf).wrapping_add(at)
                } else {
                    let index: u32 = h.read(record + 0xc);
                    let header = self.headers.wrapping_add(index.wrapping_mul(36));
                    let data: u32 = h.read(header + 0xc);
                    data.wrapping_add(h.read::<u32>(target_list + 0x34))
                };
                return offset.wrapping_add(base);
            }
            let mut at: u32 = h.read(ebp - 0x30);
            let derived = offset == 0
                && h.call_sym(IS_KIND_OF, Conv::Thiscall(target_class), &[class]) & 0xff != 0;
            if !derived {
                let index: u32 = h.read(record + 0xc);
                let entries: u32 = h.read(self.reader + 0x24);
                let entry: u32 = h.read(entries.wrapping_add(index.wrapping_mul(4)));
                if h.call_sym(
                    FIND_TARGET,
                    Conv::Thiscall(ebp - 0xa4),
                    &[ebp - 0x30, offset, entry, class],
                ) != 0
                {
                    at = h.read(ebp - 0x30);
                }
            }
            at.wrapping_sub(h.read(class + 0x88))
        }

        /// Stores `value` (or resolves the import fixup `import`) at the
        /// pointer: through the member's slots for its kind-2 arrays.
        fn store(
            &self,
            h: &mut impl Host,
            record: u32,
            member: u32,
            offset: u32,
            value: u32,
            import: u32,
        ) {
            let ebp = self.ebp;
            let flags: u32 = if member == 0 {
                2
            } else {
                h.read(member + 0x14)
            };
            let kind = if member == 0 {
                0
            } else {
                let k = h.call_virtual(member, 1, &[]);
                if k == 0 { 0 } else { h.read(k + 0x30) }
            };
            let object: u32 = h.read(ebp - 0x20);
            let at = if member == 0 {
                offset
            } else {
                let owner: u32 = h.read(member + 0xc);
                h.read::<u32>(member + 0x24)
                    .wrapping_sub(h.read::<u32>(owner + 0x88))
            };
            let slot = at.wrapping_add(object);
            let element: u32 = h.read(record + 0x14);
            if kind == 2 {
                if (flags >> 1) & 1 != 0 {
                    let array = h.call_virtual(member, 6, &[object]);
                    if array == 0 {
                        return;
                    }
                    let element: u32 = h.read(record + 0x14);
                    self.put(
                        h,
                        array.wrapping_add(element.wrapping_mul(4)),
                        value,
                        import,
                    );
                } else {
                    let n = element;
                    let mut object = object;
                    if (flags >> 6) & 1 == 0 || n != 1 {
                        h.write(slot, 0u32);
                        object = h.read(ebp - 0x20);
                    }
                    h.call_virtual(member, 8, &[object, value, n]);
                }
            } else {
                self.put(h, slot.wrapping_add(element.wrapping_mul(4)), value, import);
            }
        }

        /// `*at = value`, or the import's resolver on `at`.
        fn put(&self, h: &mut impl Host, at: u32, value: u32, import: u32) {
            if import == 0 {
                h.write(at, value);
            } else {
                let ty: u32 = h.read(import);
                let data: u32 = h.read(import + 4);
                let len: u32 = h.read(import + 8);
                h.call_sym(RESOLVE, Conv::Thiscall(ty), &[at, data, len, self.ctx]);
            }
        }
    }

    /// The list a pointer fixup comes from.
    struct Source {
        list: u32,
        class: u32,
        entry: u32,
        /// The class's flag bit 2: a member not found is not reported.
        quiet: bool,
        /// A non-plain file whose class entry has `+0x1c`.
        aligned: bool,
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
