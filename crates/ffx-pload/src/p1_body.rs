//! List `p1-load`, round 4: the cluster reader's body (`0x0057c130`,
//! thiscall on the reader, `(stream)`, `ret 4`; name ours): after the
//! cluster header, it reads the rest of a `PBinary` cluster file and
//! fixes it up, calling the reader's steps in order:
//!
//! 1. the instance-list headers' block (`+0x10` count, `+0x14` block, 36
//!    bytes each) sized to the header's list count (`+0x10`), the lists
//!    (`+0x18`, `0x0057ccf0`) and the list classes (`+0x20`,
//!    `0x0047efc0`) resized;
//! 2. the cluster's `+0x44` bits 0-1 cleared; the list headers
//!    (`0x0057c4a0`: alignment, size, largest alignment), the tables'
//!    placement (`0x00579090`, only with a plain mapping: none of the
//!    mapping's flag bits 0, 2, 31), the lists (`0x0057b3a0`) and their
//!    data (`0x0057c940`);
//! 3. the user fixups (`0x0057c5d0`), the header classes (`0x0057b6c0`),
//!    the pointer-array fixups (`0x0057acd0`), the pointer fixups
//!    (`0x0057a6b0`), the array fixups (`0x0057a320`);
//! 4. every object marked loaded (`+0x49`); the objects of the two
//!    descriptor classes `0x00c93f98` and `0x00c91330`, then those of
//!    `0x00c91180`, get their class's slot 14 (the first error stops);
//! 5. a dynamic class (record `+0x1c` bit 0) the cluster's namespace
//!    now knows by name (`0x0043ea30`) under another descriptor replaces
//!    the list's class (`+0x2c`), after `0x004494a0(default data)` on it.
//!
//! The user fixups' results and names blocks are freed when owned. The
//! steps' outputs are the original's frame words at `ebp - 0x30` to
//! `ebp - 0x14`, whose addresses they get.

use ffx_graphic::host::{Conv, Host};

/// `PhyreMalloc(size, align)` and `PhyreFree(p)` (cdecl).
const MALLOC: u32 = 0x0042_fb80;
const FREE: u32 = 0x0042_fb20;
/// The steps (thiscall).
const RESIZE_LISTS: u32 = 0x0057_ccf0;
const RESIZE_CLASSES: u32 = 0x0047_efc0;
const READ_LIST_HEADERS: u32 = 0x0057_c4a0;
const PLACE_TABLES: u32 = 0x0057_9090;
const INIT_LISTS: u32 = 0x0057_b3a0;
const READ_LISTS: u32 = 0x0057_c940;
const USER_FIXUPS: u32 = 0x0057_c5d0;
const HEADER_CLASSES: u32 = 0x0057_b6c0;
const POINTER_ARRAYS: u32 = 0x0057_acd0;
const POINTERS: u32 = 0x0057_a6b0;
const DATA_POINTERS: u32 = 0x0057_a320;
/// `PNamespace`'s class lookup by name (thiscall on the cluster).
const FIND_CLASS: u32 = 0x0043_ea30;
/// The dynamic class's default-data refresh (thiscall `(data)`).
const REFRESH: u32 = 0x0044_94a0;
/// The descriptors whose objects get slot 14, in two passes.
const FIRST_CLASSES: [u32; 2] = [0x00c9_3f98, 0x00c9_1330];
const SECOND_CLASS: u32 = 0x00c9_1180;

/// The no-memory error.
const NO_BLOCK: u32 = 0xd;

/// The ports over game memory.
pub mod bridge {
    use super::{
        Conv, DATA_POINTERS, FIND_CLASS, FIRST_CLASSES, FREE, HEADER_CLASSES, Host, INIT_LISTS,
        MALLOC, NO_BLOCK, PLACE_TABLES, POINTER_ARRAYS, POINTERS, READ_LIST_HEADERS, READ_LISTS,
        REFRESH, RESIZE_CLASSES, RESIZE_LISTS, SECOND_CLASS, USER_FIXUPS,
    };

    /// The frame's bytes (`ebp - 0x30` to `ebp - 0x14`, 8 words).
    pub const FRAME: u32 = 0x20;
    /// Where `ebp` is in it.
    const EBP: u32 = 0x30;

    /// `0x0057c130(stream)` on `reader`: 0 or the first error. `frame`
    /// is [`FRAME`] bytes at the original's `ebp - 0x30`.
    pub fn read_body(h: &mut impl Host, reader: u32, stream: u32, frame: u32) -> u32 {
        let ebp = frame + EBP;
        let header: u32 = h.read(reader + 8);
        let n: u32 = h.read(header + 0x10);
        if n != h.read::<u32>(reader + 0x10) {
            let block = if n == 0 {
                0
            } else {
                h.call_sym(MALLOC, Conv::Cdecl, &[n.wrapping_mul(36), 4])
            };
            if block == 0 && n != 0 {
                return NO_BLOCK;
            }
            let old: u32 = h.read(reader + 0x14);
            let count: u32 = h.read(reader + 0x10);
            if old != block && count >> 31 == 0 && old != 0 {
                h.call_sym(FREE, Conv::Cdecl, &[old]);
            }
            h.write(reader + 0x14, block);
            h.write(reader + 0x10, n);
        }
        let r = h.call_sym(RESIZE_LISTS, Conv::Thiscall(reader + 0x18), &[n]);
        if r != 0 {
            return r;
        }
        let r = h.call_sym(RESIZE_CLASSES, Conv::Thiscall(reader + 0x20), &[n]);
        if r != 0 {
            return r;
        }
        let count = h.read::<u32>(reader + 0x18) & 0x7fff_ffff;
        let first = match count {
            0 => 0,
            1 => reader + 0x1c,
            _ => h.read(reader + 0x1c),
        };
        let end = first.wrapping_add(n.wrapping_mul(4));
        let cluster: u32 = h.read(reader);
        let flags: u32 = h.read(cluster + 0x44);
        h.write(cluster + 0x44, flags & !3);
        for off in [0x1c, 0x18, 0x14] {
            h.write(ebp - off, 0u32);
        }
        h.call_sym(
            READ_LIST_HEADERS,
            Conv::Thiscall(reader),
            &[stream, ebp - 0x1c, ebp - 0x18, ebp - 0x14],
        );
        h.write(ebp - 0x2c, 0u32);
        h.write(ebp - 0x24, 0u32);
        let mapping: u32 = h.read(reader + 4);
        if h.read::<u32>(mapping + 0x1c) & 0x8000_0005 == 0 {
            let size: u32 = h.read(ebp - 0x18);
            let align: u32 = h.read(ebp - 0x14);
            let r = h.call_sym(
                PLACE_TABLES,
                Conv::Thiscall(reader),
                &[ebp - 0x2c, ebp - 0x24, size, align],
            );
            if r != 0 {
                return r;
            }
        }
        let r = h.call_sym(INIT_LISTS, Conv::Thiscall(reader), &[]);
        if r != 0 {
            return r;
        }
        let data: u32 = h.read(ebp - 0x24);
        let align: u32 = h.read(ebp - 0x1c);
        let r = h.call_sym(READ_LISTS, Conv::Thiscall(reader), &[stream, data, align]);
        if r != 0 {
            return r;
        }
        // the user fixups' context ({lists, tables}), results and names
        h.write(ebp - 0x30, reader + 0x18);
        for off in [0x28, 0x24, 0x20, 0x1c] {
            h.write(ebp - off, 0u32);
        }
        let r = fix_up(h, reader, stream, ebp, (first, end));
        // the results' and names' blocks, when owned
        let names: u32 = h.read(ebp - 0x20);
        let block: u32 = h.read(ebp - 0x1c);
        if names >> 31 == 0 && block != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[block]);
        }
        let results: u32 = h.read(ebp - 0x28);
        let block: u32 = h.read(ebp - 0x24);
        if results >> 31 == 0 && block != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[block]);
        }
        r
    }

    /// Steps 3 to 5: 0 or the first error.
    fn fix_up(h: &mut impl Host, reader: u32, stream: u32, ebp: u32, lists: (u32, u32)) -> u32 {
        let r = h.call_sym(
            USER_FIXUPS,
            Conv::Thiscall(reader),
            &[stream, ebp - 0x30, ebp - 0x28, ebp - 0x20],
        );
        if r != 0 {
            return r;
        }
        let r = h.call_sym(HEADER_CLASSES, Conv::Thiscall(reader), &[stream]);
        if r != 0 {
            return r;
        }
        let tables: u32 = h.read(ebp - 0x2c);
        let r = h.call_sym(POINTER_ARRAYS, Conv::Thiscall(reader), &[stream, tables]);
        if r != 0 {
            return r;
        }
        let r = h.call_sym(
            POINTERS,
            Conv::Thiscall(reader),
            &[stream, ebp - 0x30, ebp - 0x28],
        );
        if r != 0 {
            return r;
        }
        let r = h.call_sym(DATA_POINTERS, Conv::Thiscall(reader), &[stream]);
        if r != 0 {
            return r;
        }
        let r = finish_objects(h, lists);
        if r != 0 {
            return r;
        }
        replace_dynamic_classes(h, reader, lists);
        0
    }

    /// Step 4: 0 or the first slot 14 error.
    fn finish_objects(h: &mut impl Host, (first, end): (u32, u32)) -> u32 {
        let firsts = FIRST_CLASSES.map(|c| h.rebase(c));
        let mut at = first;
        while at < end {
            let object: u32 = h.read(at);
            if object != 0 {
                let class: u32 = h.read(object + 0x2c);
                h.write(object + 0x49, 1u8);
                if firsts.contains(&class) {
                    let r = h.call_virtual(class, 14, &[object]);
                    if r != 0 {
                        return r;
                    }
                }
            }
            at = at.wrapping_add(4);
        }
        let second = h.rebase(SECOND_CLASS);
        let mut at = first;
        while at < end {
            let object: u32 = h.read(at);
            if object != 0 && h.read::<u32>(object + 0x2c) == second {
                let r = h.call_virtual(second, 14, &[object]);
                if r != 0 {
                    return r;
                }
            }
            at = at.wrapping_add(4);
        }
        0
    }

    /// Step 5.
    fn replace_dynamic_classes(h: &mut impl Host, reader: u32, (first, end): (u32, u32)) {
        let cluster: u32 = h.read(reader);
        let mut entry_at: u32 = h.read(reader + 0x24);
        let mut at = first;
        while at < end {
            let entry: u32 = h.read(entry_at);
            let object: u32 = h.read(at);
            if entry != 0 && object != 0 {
                let record: u32 = h.read(entry + 4);
                if h.read::<u8>(record + 0x1c) & 1 != 0 {
                    let name: u32 = h.read(entry + 0xc);
                    let class = h.call_sym(FIND_CLASS, Conv::Thiscall(cluster), &[name]);
                    if class != 0 && class != h.read::<u32>(entry + 8) {
                        let data: u32 = h.read(entry + 0x18);
                        h.call_sym(REFRESH, Conv::Thiscall(class), &[data]);
                        h.write(object + 0x2c, class);
                    }
                }
            }
            at = at.wrapping_add(4);
            entry_at = entry_at.wrapping_add(4);
        }
    }
}
