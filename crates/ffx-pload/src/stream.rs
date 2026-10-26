//! `PStream`, `PStreamReader`, `PStreamReaderFile` and the serialization
//! start-up.
//!
//! Layouts:
//! - `PStream` (`0x0060f050`): vtable, a word (11), its name (a heap
//!   copy, `+8`; bit 0 set when not owned);
//! - `PStreamReader` (`0x0057dc40`): a `PStream`, then the bytes read so
//!   far (`+0xc`) and a word `-1` (`+0x10`); its vtable slot 5 (`+0x14`)
//!   reads `(buffer, size)` and returns the count;
//! - `PStreamReaderFile`: a `PStreamReader`, then its `PStreamFile` at
//!   `+0x14` (the archive entry or the loose file's handle);
//! - `PStreamWriterFile`: its file at `+0x10`.

use ffx_bridge::{Global, Mem};
use ffx_graphic::host::{Conv, Host};

/// `PStreamReader`'s vtable.
pub const READER_VTABLE: u32 = 0x00b3_1430;
/// `PStreamReaderFile`'s vtable.
pub const READER_FILE_VTABLE: u32 = 0x00b3_0ac0;
/// `PStream`'s vtable.
pub const STREAM_VTABLE: u32 = 0x00b3_eccc;
/// Where a `PStreamReaderFile` keeps its `PStreamFile`.
pub const READER_FILE: u32 = 0x14;
/// `PhyreEngine`'s free (cdecl, one word).
pub const FREE: u32 = 0x0042_fb20;
/// `PStreamFile::closeFile` (thiscall).
pub const CLOSE_FILE: u32 = 0x0060_7d80;
/// `PStreamFileWin32::isOpen` (thiscall).
pub const IS_OPEN: u32 = 0x0060_7f20;
/// `PStreamFile::read(buffer, size)` (thiscall).
pub const READ: u32 = 0x0060_8090;
/// `PStreamFileWin32::openFile(path, ...)` (thiscall, five words; the
/// mods wrap it, `docs/hooking.md`).
pub const OPEN_FILE: u32 = 0x0060_7f40;
/// `PStream::PStream(name)` (thiscall).
pub const STREAM_CTOR: u32 = 0x0060_f050;

/// `PStreamFile::s_streamPrefix`.
pub const STREAM_PREFIX: Global<u32> = Global::new(0x00cc_9698);
/// `PUtilitySerialization`'s initialized flag.
pub const SERIALIZATION_READY: Global<u8> = Global::new(0x00c9_0280);
/// The asset tree (`PAssetReference`), its first byte its initialized
/// flag.
pub const ASSET_TREE: u32 = 0x00c9_40c4;
/// `PPlatform::GetPlatformId()`'s result (`0x005f0490`): `"TLLB"` read as
/// a little-endian word.
pub const PLATFORM_ID: u32 = 0x424c_4c54;

/// Pure parts.
pub mod core {
    /// `0x0057dd30`'s result: 0 when the stream moved `want` bytes, else
    /// [`crate::error::SHORT_IO`].
    #[must_use]
    pub const fn exact_or_error(got: u32, want: u32) -> u32 {
        if got == want {
            0
        } else {
            crate::error::SHORT_IO
        }
    }
}

/// The ports over game memory.
pub mod bridge {
    use super::{
        ASSET_TREE, CLOSE_FILE, Conv, FREE, Host, IS_OPEN, Mem, OPEN_FILE, READ, READER_FILE,
        READER_FILE_VTABLE, READER_VTABLE, SERIALIZATION_READY, STREAM_CTOR, STREAM_PREFIX,
        STREAM_VTABLE, core,
    };

    /// An import's address (the word in the import table at `slot`).
    fn import(h: &impl Host, slot: u32) -> u32 {
        h.read(h.rebase(slot))
    }

    /// `PStreamFile::PStreamFile(file, path, b, c, d, e)` (`0x00607bc0`,
    /// thiscall): no handle (`-1`), no archive entry; opens `path` with
    /// `openFile` (`0x00607f40`, thiscall, five words) when it is not 0;
    /// returns `file`.
    pub fn file_ctor(h: &mut impl Host, file: u32, args: [u32; 5]) -> u32 {
        h.write(file, u32::MAX);
        h.write(file + 4, 0u32);
        if args[0] != 0 {
            h.call_sym(OPEN_FILE, Conv::Thiscall(file), &args);
        }
        file
    }

    /// `PStreamFile::closeFile(file)` (`0x00607d80`, thiscall): releases
    /// the archive entry (`0x0061aa50`, thiscall), or flushes and closes
    /// the loose file's handle (the imports at `0x00b0c144` and
    /// `0x00b0c0dc`, stdcall). `al` = 1 over the last value `eax` held: the
    /// handle (`-1`), or the close's result.
    pub fn close_file(h: &mut impl Host, file: u32) -> u32 {
        let entry: u32 = h.read(file + 4);
        if entry != 0 {
            h.call_sym(0x0061_aa50, Conv::Thiscall(entry), &[]);
            h.write(file + 4, 0u32);
        }
        let handle: u32 = h.read(file);
        if handle == u32::MAX {
            return (handle & !0xff) | 1;
        }
        let flush = import(h, 0x00b0_c144);
        h.call(flush, Conv::Stdcall, &[handle]);
        let handle: u32 = h.read(file);
        let close = import(h, 0x00b0_c0dc);
        let r = h.call(close, Conv::Stdcall, &[handle]);
        h.write(file, u32::MAX);
        (r & !0xff) | 1
    }

    /// `PStreamFile::read(file, buffer, size)` (`0x00608090`, thiscall):
    /// from the archive entry (`0x0061aa60`, thiscall), or `ReadFile` on
    /// the loose handle (the import at `0x00b0c140`) into `count`, a word
    /// the callee writes (the original's local); the count read, 0 without
    /// a file.
    pub fn file_read(h: &mut impl Host, count: u32, file: u32, buffer: u32, size: u32) -> u32 {
        let entry: u32 = h.read(file + 4);
        if entry != 0 {
            return h.call_sym(0x0061_aa60, Conv::Thiscall(entry), &[buffer, size]);
        }
        let handle: u32 = h.read(file);
        h.write(count, 0u32);
        if handle == u32::MAX {
            return 0;
        }
        let read = import(h, 0x00b0_c140);
        h.call(read, Conv::Stdcall, &[handle, buffer, size, count, 0]);
        h.read(count)
    }

    /// `PNamespacePacked::needsEndianSwap(ns)` (`0x0042b830`, thiscall):
    /// whether its first word is 0.
    pub fn needs_endian_swap(m: &impl Mem, ns: u32) -> u32 {
        let v: u32 = m.read(ns);
        u32::from(v == 0)
    }

    /// `PStreamReaderFile::close(reader)` (`0x00576930`, thiscall):
    /// `closeFile` on its file; its `eax`.
    pub fn reader_close(h: &mut impl Host, reader: u32) -> u32 {
        h.call_sym(CLOSE_FILE, Conv::Thiscall(reader + READER_FILE), &[])
    }

    /// `PStreamReaderFile::isOpen(reader)` (`0x00576980`, thiscall):
    /// `isOpen` on its file; its `eax`.
    pub fn reader_is_open(h: &mut impl Host, reader: u32) -> u32 {
        h.call_sym(IS_OPEN, Conv::Thiscall(reader + READER_FILE), &[])
    }

    /// `PStreamReaderFile` slot 5, `read(reader, buffer, size)`
    /// (`0x00576950`, thiscall): `read` on its file; the count.
    pub fn reader_read(h: &mut impl Host, reader: u32, buffer: u32, size: u32) -> u32 {
        h.call_sym(READ, Conv::Thiscall(reader + READER_FILE), &[buffer, size])
    }

    /// A recursive mutex's lock (`0x00431840`, thiscall; Ghidra's dump
    /// calls it `PStreamWriterFile::close`): `0x00431850(mutex; &mutex->inner)`
    /// (`+0x10`); its `eax`. `0x004319d0` is the unlock.
    pub fn mutex_lock(h: &mut impl Host, mutex: u32) -> u32 {
        h.call_sym(0x0043_1850, Conv::Thiscall(mutex), &[mutex + 0x10])
    }

    /// The unlock of [`mutex_lock`] (`0x004319d0`, original).
    pub fn mutex_unlock(h: &mut impl Host, mutex: u32) {
        h.call_sym(0x0043_19d0, Conv::Thiscall(mutex), &[]);
    }

    /// `PStream::~PStream(stream)` (`0x0060f0c0`, thiscall): its vtable
    /// back, frees the name it owns (bit 0 clear).
    pub fn stream_dtor(h: &mut impl Host, stream: u32) {
        let vt = h.rebase(STREAM_VTABLE);
        h.write(stream, vt);
        let name: u32 = h.read(stream + 8);
        if name & 1 == 0 {
            h.call_sym(FREE, Conv::Cdecl, &[name]);
        }
    }

    /// `PStreamReader::~PStreamReader(reader)` (`0x0057dc70`, thiscall):
    /// its vtable back, then [`stream_dtor`].
    pub fn reader_dtor(h: &mut impl Host, reader: u32) {
        let vt = h.rebase(READER_VTABLE);
        h.write(reader, vt);
        stream_dtor(h, reader);
    }

    /// `PStreamReaderFile::~PStreamReaderFile(reader)` (`0x005767c0`,
    /// thiscall): its vtable, [`reader_dtor`], then its file's destructor
    /// (`0x00607c10`, an empty body).
    pub fn reader_file_dtor(h: &mut impl Host, reader: u32) {
        let vt = h.rebase(READER_FILE_VTABLE);
        h.write(reader, vt);
        reader_dtor(h, reader);
        h.call_sym(0x0060_7c10, Conv::Thiscall(reader + READER_FILE), &[]);
    }

    /// `PStream::PStream(stream, name)` (`0x0060f050`, thiscall): its
    /// vtable, 0xb, and a copy of `name` (alignment 2; 0 for none);
    /// returns `stream`.
    pub fn stream_ctor(h: &mut impl Host, stream: u32, name: u32) -> u32 {
        let vt = h.rebase(STREAM_VTABLE);
        h.write(stream, [vt, 0xb]);
        let copy = if name == 0 {
            0
        } else {
            crate::descriptor::bridge::copy_string(h, name, 2)
        };
        h.write(stream + 8, copy);
        stream
    }

    /// `PStreamReaderFile::PStreamReaderFile(reader, path)`
    /// (`0x005766d0`, thiscall): its file opened for reading
    /// ([`file_ctor`] with `(path, 1, 0, 0, 1)`), [`reader_ctor`], its
    /// vtable, and the file's size (`getFileSize`, `0x00607dc0`, low
    /// word) at `+0x10`; returns `reader`.
    pub fn reader_file_ctor(h: &mut impl Host, reader: u32, path: u32) -> u32 {
        file_ctor(h, reader + READER_FILE, [path, 1, 0, 0, 1]);
        reader_ctor(h, reader, path);
        let vt = h.rebase(READER_FILE_VTABLE);
        h.write(reader, vt);
        let size = h.call_sym(0x0060_7dc0, Conv::Thiscall(reader + READER_FILE), &[]);
        h.write(reader + 0x10, size);
        reader
    }

    /// `PStreamReader::PStreamReader(reader, name)` (`0x0057dc40`,
    /// thiscall): `PStream(name)`, then its vtable, count 0 and `-1`;
    /// returns `reader`.
    pub fn reader_ctor(h: &mut impl Host, reader: u32, name: u32) -> u32 {
        h.call_sym(STREAM_CTOR, Conv::Thiscall(reader), &[name]);
        let vt = h.rebase(READER_VTABLE);
        h.write(reader, vt);
        h.write(reader + 0xc, 0u32);
        h.write(reader + 0x10, u32::MAX);
        reader
    }

    /// `PStreamReader::read(reader, buffer, size)` (`0x0057dd10`,
    /// thiscall): slot 5, adds the count to `+0xc`; returns it.
    pub fn reader_read_counted(h: &mut impl Host, reader: u32, buffer: u32, size: u32) -> u32 {
        let n = h.call_virtual(reader, 5, &[buffer, size]);
        let total: u32 = h.read(reader + 0xc);
        h.write(reader + 0xc, total.wrapping_add(n));
        n
    }

    /// `PStreamReader::readExact(reader, buffer, size)` (`0x0057dd30`,
    /// thiscall): as [`reader_read_counted`]; 0 when it read `size`
    /// bytes, else [`crate::error::SHORT_IO`].
    pub fn reader_read_exact(h: &mut impl Host, reader: u32, buffer: u32, size: u32) -> u32 {
        let n = reader_read_counted(h, reader, buffer, size);
        core::exact_or_error(n, size)
    }

    /// `PStreamFile::SetStreamPrefix(prefix)` (`0x00607d40`, cdecl);
    /// returns it.
    pub fn set_stream_prefix(m: &mut impl Mem, prefix: u32) -> u32 {
        STREAM_PREFIX.set(m, prefix);
        prefix
    }

    /// `PUtilitySerialization::IsInitialized()` (`0x00432020`): the flag
    /// byte.
    pub fn serialization_ready(m: &impl Mem) -> u8 {
        SERIALIZATION_READY.get(m)
    }

    /// `PUtilitySerialization::initialize()` (`0x004320e0`): the cluster
    /// header types' `Bind`s and registration, then the flag; 0.
    pub fn serialization_init(h: &mut impl Host) -> u32 {
        h.call_sym(0x0046_8890, Conv::Cdecl, &[]);
        h.call_sym(0x0046_9a70, Conv::Cdecl, &[]);
        crate::descriptor::bridge::register_cluster_header(h);
        h.call_sym(0x0046_a340, Conv::Cdecl, &[]);
        SERIALIZATION_READY.set(h, 1);
        0
    }

    /// `PAssetReference::InitializeAssetTree()` (`0x0045d600`):
    /// [`crate::error::ALREADY`] when the tree is set up, else its
    /// initializer `0x004317f0` (thiscall on the tree); its `eax`.
    pub fn init_asset_tree(h: &mut impl Host) -> u32 {
        let tree = h.rebase(ASSET_TREE);
        let ready: u8 = h.read(tree);
        if ready != 0 {
            return crate::error::ALREADY;
        }
        h.call_sym(0x0043_17f0, Conv::Thiscall(tree), &[])
    }

    /// `PUtilityGame::initialize()` (`0x009f4c80`): `0x00a33540()`; 0.
    pub fn game_init(h: &mut impl Host) -> u32 {
        h.call_sym(0x00a3_3540, Conv::Cdecl, &[]);
        0
    }
}
