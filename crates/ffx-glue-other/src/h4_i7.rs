//! Continuation round 7 of list `h4-glue-other`, batch 7: the cluster
//! manager's preallocated data list.

use ffx_graphic::host::{Conv, Host};

use crate::h4_i1::USER_MALLOC;
use crate::h4_i2::{OPERATOR_DELETE, STREAM_READ};

/// `std::string::assign(this; text, n)` (`0x00404270`, thiscall, ret 8).
pub const STRING_ASSIGN: u32 = 0x0040_4270;
/// `AssetListReader_getPlatformSpecificPath(out, path)` (cdecl; `path` a
/// `std::string` by value, 6 words, which the callee destroys).
pub const PLATFORM_PATH: u32 = 0x0069_0590;
/// `fiosIsFileExist(path)` (cdecl).
pub const FILE_EXISTS: u32 = 0x0067_96f0;
/// `Phyre_PSerialization_PStreamReaderFile_PStreamReaderFile(reader;
/// path)` (thiscall, ret 4).
pub const READER_CTOR: u32 = 0x0057_66d0;
/// `Phyre_PSerialization_PStreamReaderFile_isOpen(reader)` (thiscall):
/// `al`.
pub const READER_IS_OPEN: u32 = 0x0057_6980;
/// `Phyre_PSerialization_PStreamReaderFile_close(reader)` (thiscall).
pub const READER_CLOSE: u32 = 0x0057_6930;
/// `Phyre_PSerialization_PStreamReaderFile_dtor_PStreamReaderFile(reader)`
/// (thiscall).
pub const READER_DTOR: u32 = 0x0057_67c0;
/// Bytes of [`create_preallocate_data_list`]'s frame, from `ebp - 0x5c`:
/// the path argument the original builds in its outgoing argument area
/// (`+0`, a 0x18-byte `std::string`), the reader (`+0x20`; its file
/// stream at `+0x34`, the data range `+0x2c`..`+0x30`) and the platform
/// path (`+0x40`, a `std::string`: its capacity at `+0x54`).
pub const PREALLOCATE_FRAME: u32 = 0x58;

/// The text of the `std::string` at `s` (MSVC: inline below 16 bytes of
/// capacity).
fn string_text(h: &impl Host, s: u32) -> u32 {
    if h.read::<u32>(s.wrapping_add(0x14)) < 0x10 {
        s
    } else {
        h.read(s)
    }
}

/// `ClusterManager_createPreallocateDataList(name)` (`0x0063d260`, cdecl,
/// from `Ch_LoadClothFileTask`; `frame` see [`PREALLOCATE_FRAME`]): for a
/// name, its platform path (a `std::string` made from the name, passed by
/// value to `AssetListReader_getPlatformSpecificPath`); when that file
/// exists and opens, a `user_malloc` block of its size (the reader's end
/// minus its start, -1 for an unknown end) read whole. The reader is
/// destroyed and the path string freed when on the heap. Returns the
/// block, or 0 (Ghidra: void).
pub fn create_preallocate_data_list(h: &mut impl Host, frame: u32, name: u32) -> u32 {
    if name == 0 {
        return 0;
    }
    let arg = frame;
    let reader = frame.wrapping_add(0x20);
    let path = frame.wrapping_add(0x40);
    h.write(arg.wrapping_add(0x14), 0xfu32);
    h.write(arg.wrapping_add(0x10), 0u32);
    h.write(arg, 0u8);
    let mut n = 0u32;
    while h.read::<u8>(name.wrapping_add(n)) != 0 {
        n += 1;
    }
    h.call_sym(STRING_ASSIGN, Conv::Thiscall(arg), &[name, n]);
    let mut words = [path, 0, 0, 0, 0, 0, 0];
    for (k, w) in words.iter_mut().skip(1).enumerate() {
        *w = h.read(arg.wrapping_add(4 * k as u32));
    }
    h.call_sym(PLATFORM_PATH, Conv::Cdecl, &words);
    let text = string_text(h, path);
    let mut block = 0;
    if h.call_sym(FILE_EXISTS, Conv::Cdecl, &[text]) != 0 {
        let text = string_text(h, path);
        h.call_sym(READER_CTOR, Conv::Thiscall(reader), &[text]);
        if h.call_sym(READER_IS_OPEN, Conv::Thiscall(reader), &[]) as u8 != 0 {
            let end: u32 = h.read(reader.wrapping_add(0x10));
            let size = if end == u32::MAX {
                u32::MAX
            } else {
                end.wrapping_sub(h.read(reader.wrapping_add(0xc)))
            };
            block = h.call_sym(USER_MALLOC, Conv::Cdecl, &[size]);
            h.call_sym(
                STREAM_READ,
                Conv::Thiscall(reader.wrapping_add(0x14)),
                &[block, size],
            );
            h.call_sym(READER_CLOSE, Conv::Thiscall(reader), &[]);
        }
        h.call_sym(READER_DTOR, Conv::Thiscall(reader), &[]);
    }
    if h.read::<u32>(path.wrapping_add(0x14)) >= 0x10 {
        let p: u32 = h.read(path);
        h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[p]);
    }
    block
}
