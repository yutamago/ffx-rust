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
/// `PStream::PStream(name)` (thiscall).
pub const STREAM_CTOR: u32 = 0x0060_f050;

/// `PStreamFile::s_streamPrefix`.
pub const STREAM_PREFIX: Global<u32> = Global::new(0x00cc_9698);
/// `PUtilitySerialization`'s initialized flag.
pub const SERIALIZATION_READY: Global<u8> = Global::new(0x00c9_0280);
/// The asset tree (`PAssetReference`), its first byte its initialized
/// flag.
pub const ASSET_TREE: u32 = 0x00c9_40c4;

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
        ASSET_TREE, CLOSE_FILE, Conv, FREE, Host, IS_OPEN, Mem, READ, READER_FILE, READER_VTABLE,
        SERIALIZATION_READY, STREAM_CTOR, STREAM_PREFIX, STREAM_VTABLE, core,
    };

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

    /// `PStreamWriterFile::close(writer)` (`0x00431840`, thiscall):
    /// `0x00431850(writer, &writer->file)`; its `eax`.
    pub fn writer_close(h: &mut impl Host, writer: u32) -> u32 {
        h.call_sym(0x0043_1850, Conv::Thiscall(writer), &[writer + 0x10])
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
