//! Continuation round 4 of list `h4-glue-other`, batch 7: the sound
//! event group's list load.

use ffx_graphic::host::{Conv, Host};

use crate::h4_c2::USER_FREE_THUNK;
use crate::h4_d1::FIOS_EXISTS;
use crate::h4_d4::{FIOS_CLOSE, FIOS_OPEN, FIOS_READ, FIOS_SIZE};
use crate::h4_e5::OPERATOR_DELETE_ARRAY;
use crate::h4_f3::Words;
use crate::misc::OPERATOR_NEW_2;

/// `Array_Reserve_00667e00(array; n)` (thiscall).
pub const ARRAY_RESERVE: u32 = 0x0066_7e00;
/// `IdTree_insertArray_00710ba0(tree; &key, &array)` (thiscall).
pub const ID_TREE_INSERT_ARRAY: u32 = 0x0071_0ba0;
/// Bytes of [`event_group_load`]'s frame, from `ebp - 0x34`: the id
/// array (`+0`: count, capacity, entries) and the key (`+0x10`), whose
/// addresses the tree's insert takes.
pub const GROUP_LOAD_FRAME: u32 = 0x14;

/// `SfxEventGroup_load_00710c50(this; path)` (thiscall, ret 4; `frame`
/// see [`GROUP_LOAD_FRAME`]; without its SEH frame): an existing file
/// (`fiosIsFileExist`, `fiosOpen(path, 1)`; else 0) is read whole into a
/// new buffer (`operator new` of its size plus a 0 byte). Its words are
/// records: a key, a count `n` and `n` ids. Each record whose count word
/// lies inside the file builds an id array (reserved for `n > 0`, signed,
/// with `Array_Reserve`; the ids appended as the engine's `PArray` push
/// does, `n` taken as unsigned, none for 0) and, unless the ids ran past
/// the file, inserts it under the key into the tree at `+4`
/// (`IdTree_insertArray`); the array's entries are freed. The buffer is
/// freed (`operator delete[]`, its `eax` returned).
pub fn event_group_load(h: &mut impl Host, frame: u32, this: u32, path: u32) -> u32 {
    let arr = frame;
    let key = frame.wrapping_add(0x10);
    if h.call_sym(FIOS_EXISTS, Conv::Cdecl, &[path]) == 0 {
        return 0;
    }
    let file = h.call_sym(FIOS_OPEN, Conv::Cdecl, &[path, 1]);
    if file == 0 {
        return 0;
    }
    let size = h.call_sym(FIOS_SIZE, Conv::Cdecl, &[file]);
    let buf = h.call_sym(OPERATOR_NEW_2, Conv::Cdecl, &[size.wrapping_add(1)]);
    h.write(buf.wrapping_add(size), 0u8);
    h.call_sym(FIOS_READ, Conv::Cdecl, &[file, buf, size]);
    h.call_sym(FIOS_CLOSE, Conv::Cdecl, &[file]);
    let words = size >> 2;
    let mut i = 0u32;
    while i < words {
        i = i.wrapping_add(2);
        let k: u32 = h.read(buf.wrapping_add(4 * (i.wrapping_sub(2))));
        h.write(key, k);
        let mut n: u32 = h.read(buf.wrapping_add(4 * (i.wrapping_sub(1))));
        if i >= words {
            break;
        }
        for off in [0u32, 4, 8] {
            h.write(arr.wrapping_add(off), 0u32);
        }
        if n.cast_signed() > 0 {
            h.call_sym(ARRAY_RESERVE, Conv::Thiscall(arr), &[n]);
        }
        let mut ids = Words::load(h, arr);
        if n != 0 {
            loop {
                let v: u32 = h.read(buf.wrapping_add(4 * i));
                i = i.wrapping_add(1);
                let slot = ids.grow(h);
                h.write(arr.wrapping_add(4), ids.cap);
                h.write(arr.wrapping_add(8), ids.data);
                if slot != 0 {
                    h.write(slot, v);
                }
                n = n.wrapping_sub(1);
                if n == 0 {
                    break;
                }
            }
            h.write(arr, ids.count);
        }
        if i <= words {
            h.call_sym(
                ID_TREE_INSERT_ARRAY,
                Conv::Thiscall(this.wrapping_add(4)),
                &[key, arr],
            );
        }
        if ids.data != 0 {
            h.call_sym(USER_FREE_THUNK, Conv::Cdecl, &[ids.data]);
        }
    }
    h.call_sym(OPERATOR_DELETE_ARRAY, Conv::Cdecl, &[buf])
}
