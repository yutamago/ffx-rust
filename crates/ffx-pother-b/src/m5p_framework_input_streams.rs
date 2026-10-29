//! `PSerialization` file streams (list `phyre-framework-input`, batch 2):
//! the reader/writer file classes' open, seek, write, reset and
//! destructors, `PStream`'s deleting destructor, and the engine-object
//! registry's deleting destructor (`PUtilityInputs`).
//!
//! A `PStreamReaderFile` is `PStreamReader` (`vtable`, `+4` the open
//! result, `+8` the name `PString`, `+0xc` the position, `+0x10` the size)
//! with its `PStreamFile` at `+0x14` (handle, archive entry). A
//! `PStreamWriterFile` keeps its `PStreamFile` at `+0x10`.
//!
//! The Win32 calls (`SetFilePointer`, `WriteFile`) go through `FFX.exe`'s
//! import slots; `openFile` (`0x00607f40`) stays the original, so the mod
//! loader's wrapper still sees every open.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `Phyre::PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `operator delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `PStream`'s vtable.
pub const STREAM_VTABLE: u32 = 0x00b3_eccc;
/// `PStreamReaderFile`'s vtable.
pub const READER_FILE_VTABLE: u32 = 0x00b3_0ac0;
/// `PStreamWriterFile`'s vtable.
pub const WRITER_FILE_VTABLE: u32 = 0x00b7_3d4c;
/// The registry entry's vtable `0x009f4590` stores.
pub const REGISTRY_ENTRY_VTABLE: u32 = 0x00b0_e078;
/// The registry's list head (`+4` of each entry is the next).
pub const REGISTRY_HEAD: u32 = 0x00c9_01a8;
/// `VFile_fileSeek(offset, whence)` (thiscall, `ret 8`).
pub const VFILE_SEEK: u32 = 0x0061_ac30;
/// `PStreamFileWin32::openFile(path, mode, 3 more)` (thiscall, `ret 0x14`).
pub const OPEN_FILE: u32 = 0x0060_7f40;
/// `PStreamFile::PStreamFile(path, a, b, c, d)` (thiscall, `ret 0x14`).
pub const FILE_CTOR: u32 = 0x0060_7bc0;
/// `PStreamFile`'s destructor: a bare `ret` (thiscall).
pub const FILE_DTOR: u32 = 0x0060_7c10;
/// `PStreamFile::getFileSize()` (thiscall).
pub const FILE_SIZE: u32 = 0x0060_7dc0;
/// `Phyre::PString::Assign(text)` (thiscall, `ret 4`).
pub const STRING_ASSIGN: u32 = 0x0044_5cf0;
/// `PStreamReader::~PStreamReader` (thiscall).
pub const READER_DTOR: u32 = 0x0057_dc70;
/// `PStreamReader::PStreamReader(name)` (thiscall, `ret 4`; in the list).
pub const READER_CTOR: u32 = 0x009f_0630;
/// `PStreamWriter`'s destructor (thiscall; in the list, `0x009f0660`).
pub const WRITER_DTOR: u32 = 0x009f_0660;
/// Import slot of `KERNEL32!SetFilePointer` (stdcall, 4 words).
pub const SET_FILE_POINTER_SLOT: u32 = 0x00b0_c148;
/// Import slot of `KERNEL32!WriteFile` (stdcall, 5 words).
pub const WRITE_FILE_SLOT: u32 = 0x00b0_c14c;

/// `INVALID_HANDLE_VALUE` / `INVALID_SET_FILE_POINTER`.
pub const INVALID: u32 = 0xffff_ffff;

/// The position `PStreamReaderFile::reset` seeks back to: the name word
/// `+8` with its bit 0 (the "not owned" flag) cleared, as the original's
/// `[+8] - ([+8] & 1)`.
#[must_use]
pub const fn reset_target(word8: u32) -> u32 {
    word8 - (word8 & 1)
}

pub mod bridge {
    use super::{
        Conv, FILE_CTOR, FILE_DTOR, FILE_SIZE, Host, INVALID, Mem, OPEN_FILE, OPERATOR_DELETE,
        PHYRE_FREE, READER_CTOR, READER_DTOR, READER_FILE_VTABLE, REGISTRY_ENTRY_VTABLE,
        REGISTRY_HEAD, SET_FILE_POINTER_SLOT, STREAM_VTABLE, STRING_ASSIGN, VFILE_SEEK,
        WRITE_FILE_SLOT, WRITER_DTOR, WRITER_FILE_VTABLE, reset_target,
    };

    fn import(h: &impl Mem, slot: u32) -> u32 {
        h.read(h.rebase(slot))
    }

    /// `PStreamFileWin32::seek(offset)` (`0x00608120`, thiscall on the
    /// `PStreamFile`, `ret 4`): relative to the current position, through
    /// the archive entry `+4` (`VFile_fileSeek(offset, 1)`) or the handle
    /// (`SetFilePointer(handle, offset, 0, FILE_CURRENT)`); `offset`, or 0
    /// when the seek returns -1. A file with neither (handle -1) answers
    /// `offset`.
    pub fn file_seek(h: &mut impl Host, file: u32, offset: u32) -> u32 {
        let entry: u32 = h.read(file + 4);
        let r = if entry == 0 {
            let handle: u32 = h.read(file);
            if handle == INVALID {
                return offset;
            }
            let f = import(h, SET_FILE_POINTER_SLOT);
            h.call(f, Conv::Stdcall, &[handle, offset, 0, 1])
        } else {
            h.call_sym(VFILE_SEEK, Conv::Thiscall(entry), &[offset, 1])
        };
        if r == INVALID { 0 } else { offset }
    }

    /// `PStreamFileWin32::write(data, size)` (`0x00608160`, `ret 8`): 0
    /// for an archive entry (read-only); else `WriteFile(handle, data,
    /// size, &written, 0)` with `written` in the frame word at `written`;
    /// 1 when it returns 1, else 0. `None` for the archive case, where the
    /// original clears only `al`.
    pub fn file_write(
        h: &mut impl Host,
        file: u32,
        data: u32,
        size: u32,
        written: u32,
    ) -> Option<u32> {
        let entry: u32 = h.read(file + 4);
        if entry != 0 {
            return None;
        }
        let handle: u32 = h.read(file);
        let f = import(h, WRITE_FILE_SLOT);
        let r = h.call(f, Conv::Stdcall, &[handle, data, size, written, 0]);
        Some(u32::from(r == 1))
    }

    /// `PStreamReaderFile` slot (`0x00576a10`, thiscall): the stream's
    /// virtual `+4` (is it open); if so, the virtual `+0xc` seek to
    /// [`reset_target`] of `+8`, and on success `+0xc` = 0 and `al = 1`.
    /// Returns `eax` with `al` 0 on failure (the upper bytes are the
    /// failing call's).
    pub fn reader_reset(h: &mut impl Host, this: u32) -> u32 {
        let open = h.call_virtual(this, 1, &[]);
        if open as u8 == 0 {
            return open & 0xffff_ff00;
        }
        let word8: u32 = h.read(this + 8);
        let ok = h.call_virtual(this, 3, &[reset_target(word8)]);
        if ok as u8 == 0 {
            return ok & 0xffff_ff00;
        }
        h.write(this + 0xc, 0u32);
        (ok & 0xffff_ff00) | 1
    }

    /// `PStreamReaderFile::open(path)` (`0x00576990`, `ret 4`): `openFile`
    /// on the file at `+0x14` (`path, 1, 0, 0, 1`) to `+4`, the name, the
    /// position 0 and the size; 1 when the open returned 0, else 0.
    pub fn reader_open(h: &mut impl Host, this: u32, path: u32) -> u32 {
        let file = this + 0x14;
        let r = h.call_sym(OPEN_FILE, Conv::Thiscall(file), &[path, 1, 0, 0, 1]);
        h.write(this + 4, r);
        h.call_sym(STRING_ASSIGN, Conv::Thiscall(this + 8), &[path]);
        h.write(this + 0xc, 0u32);
        h.write(this + 0x10, INVALID);
        let size = h.call_sym(FILE_SIZE, Conv::Thiscall(file), &[]);
        h.write(this + 0x10, size);
        u32::from(r == 0)
    }

    /// `PStreamReaderFile`'s scalar deleting destructor (`0x00576820`,
    /// `ret 4`): its vtable, `~PStreamReader`, the file's destructor,
    /// `PhyreFree` when `flags & 1`; `this`.
    pub fn reader_file_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(READER_FILE_VTABLE);
        h.write(this, vt);
        h.call_sym(READER_DTOR, Conv::Thiscall(this), &[]);
        h.call_sym(FILE_DTOR, Conv::Thiscall(this + 0x14), &[]);
        if flags & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `PStream`'s scalar deleting destructor (`0x0060f100`, `ret 4`): its
    /// vtable, the name buffer `+8` freed unless its bit 0 is set,
    /// `PhyreFree(this)` when `flags & 1`; `this`.
    pub fn stream_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(STREAM_VTABLE);
        h.write(this, vt);
        let name: u32 = h.read(this + 8);
        if name & 1 == 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[name]);
        }
        if flags & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `PStreamWriter::write(data, n)` (`0x009f06a0`, `ret 8`): the
    /// stream's virtual `+0x14` (`data, n`); on `al` set the position
    /// `+0xc` grows by `n` and the answer is 0, else 9.
    pub fn writer_write(h: &mut impl Host, this: u32, data: u32, n: u32) -> u32 {
        let ok = h.call_virtual(this, 5, &[data, n]);
        if ok as u8 == 0 {
            return 9;
        }
        let pos: u32 = h.read(this + 0xc);
        h.write(this + 0xc, pos.wrapping_add(n));
        0
    }

    /// `PStreamWriterFile::PStreamWriterFile(path)` (`0x009f06d0`, `ret
    /// 4`): the file at `+0x10` (`path, 0, 0, 0, 1`), `PStreamReader`'s
    /// constructor with `path`, its own vtable; `this`.
    pub fn writer_file_ctor(h: &mut impl Host, this: u32, path: u32) -> u32 {
        h.call_sym(FILE_CTOR, Conv::Thiscall(this + 0x10), &[path, 0, 0, 0, 1]);
        h.call_sym(READER_CTOR, Conv::Thiscall(this), &[path]);
        let vt = h.rebase(WRITER_FILE_VTABLE);
        h.write(this, vt);
        this
    }

    /// `PStreamWriterFile::~PStreamWriterFile` (`0x009f0740`): its vtable,
    /// `PStreamWriter`'s destructor, the file's (a bare `ret`, which keeps
    /// the `eax` before it); that `eax`.
    pub fn writer_file_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(WRITER_FILE_VTABLE);
        h.write(this, vt);
        let eax = h.call_sym(WRITER_DTOR, Conv::Thiscall(this), &[]);
        h.call_sym(FILE_DTOR, Conv::Thiscall(this + 0x10), &[]);
        eax
    }

    /// The registry entry's scalar deleting destructor (`0x009f4590`,
    /// `ret 4`): its vtable, the entry unlinked from the list at
    /// [`REGISTRY_HEAD`] (next at `+4`, then cleared) when it is there,
    /// `operator delete` when `flags & 1`; `this`.
    pub fn registry_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(REGISTRY_ENTRY_VTABLE);
        h.write(this, vt);
        let mut link = h.rebase(REGISTRY_HEAD);
        let first: u32 = h.read(link);
        if first != 0 {
            loop {
                let e: u32 = h.read(link);
                if e == this {
                    break;
                }
                let next: u32 = h.read(e + 4);
                link = e + 4;
                if next == 0 {
                    break;
                }
            }
            let at: u32 = h.read(link);
            if at != 0 {
                let next: u32 = h.read(this + 4);
                h.write(link, next);
                h.write(this + 4, 0u32);
            }
        }
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }
}
