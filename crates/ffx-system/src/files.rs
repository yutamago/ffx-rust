//! The file layer above the streams: the `sce*`/`fios*` wrappers the PS2
//! code calls, the Phyre stream size and open checks, and the CD library's
//! name and read entry points (`g_CdLib`'s function table).
//!
//! The streams' own reads and opens (`Phyre::PSerialization::PStreamFile`,
//! `File_Probe`) and the CD table's functions are not ported yet; the
//! bridges take them as callbacks, which get the game memory first.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use crate::cd::bridge::get_cd;

    /// `PStreamFileWin32::isOpen()` (`0x00607f20`, `thiscall`): a handle
    /// other than -1 (`+0`) or a big-file entry (`+4`).
    pub fn stream_is_open(mem: &impl Mem, stream: u32) -> bool {
        mem.read::<u32>(stream) != u32::MAX || mem.read::<u32>(stream.wrapping_add(4)) != 0
    }

    /// `PStreamFile::getFileSize()` (`0x00607dc0`, `thiscall`): the big-file
    /// entry's 64-bit size (`*(entry + 0xc) + 8`), else `GetFileSizeEx` on
    /// the handle (passed in: `Some(size)` on success), else 0. Only the low
    /// word is meaningful when the handle answers.
    pub fn stream_file_size<M: Mem>(
        mem: &mut M,
        stream: u32,
        get_file_size_ex: impl FnOnce(&mut M, u32) -> Option<u64>,
    ) -> u64 {
        let entry: u32 = mem.read(stream.wrapping_add(4));
        if entry != 0 {
            return crate::accessor::bridge::entry_size64(mem, entry);
        }
        let handle: u32 = mem.read(stream);
        if handle == u32::MAX {
            return 0;
        }
        get_file_size_ex(mem, handle).unwrap_or(0)
    }

    /// `PhyreFIOSStreamFile::getFileSize()` (`0x006f9d50`, `thiscall`):
    /// [`stream_file_size`] of an open stream, else -1 (in `eax`).
    pub fn fios_stream_file_size<M: Mem>(
        mem: &mut M,
        stream: u32,
        get_file_size_ex: impl FnOnce(&mut M, u32) -> Option<u64>,
    ) -> u64 {
        if stream_is_open(mem, stream) {
            stream_file_size(mem, stream, get_file_size_ex)
        } else {
            u64::from(u32::MAX)
        }
    }

    /// `fiosFileSize(stream)` (`0x00679570`) and `sceFileSize(stream)`
    /// (`0x0062f760`, which only adds its own null check): 0 for a null
    /// stream, else [`fios_stream_file_size`] (its low word).
    pub fn fios_file_size<M: Mem>(
        mem: &mut M,
        stream: u32,
        get_file_size_ex: impl FnOnce(&mut M, u32) -> Option<u64>,
    ) -> u32 {
        if stream == 0 {
            return 0;
        }
        #[allow(clippy::cast_possible_truncation)] // eax
        let size = fios_stream_file_size(mem, stream, get_file_size_ex) as u32;
        size
    }

    /// `fiosRead(stream, buf, n)` (`0x00679800`): 0 for a null stream, else
    /// `PStreamFile::read(buf, n)` (passed in) on it.
    pub fn fios_read<M: Mem>(
        mem: &mut M,
        stream: u32,
        buf: u32,
        n: u32,
        read: impl FnOnce(&mut M, u32, u32, u32) -> u32,
    ) -> u32 {
        if stream == 0 {
            return 0;
        }
        read(mem, stream, buf, n)
    }

    /// `sceRead(fd, buf, n)` (`0x0062fc00`): [`fios_read`]; returns 0.
    pub fn sce_read<M: Mem>(
        mem: &mut M,
        fd: u32,
        buf: u32,
        n: u32,
        read: impl FnOnce(&mut M, u32, u32, u32) -> u32,
    ) -> u32 {
        fios_read(mem, fd, buf, n, read);
        0
    }

    /// `fiosIsFileExist(name)` (`0x006796f0`): the low byte of
    /// `PhyreFIOS::isFileExist(name)` (`0x006f9d70`, a jump to `File_Probe`,
    /// passed in).
    pub fn fios_is_file_exist<M: Mem>(
        mem: &mut M,
        name: u32,
        probe: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        probe(mem, name) & 0xff
    }

    /// `g_CdLib`'s entry `slot` (a function pointer).
    pub fn cd_function(mem: &impl Mem, slot: u32) -> u32 {
        mem.read(get_cd(mem).wrapping_add(slot))
    }

    /// `TODVDFileRead(file, buf, len)` (`0x008aba30`): through `g_CdLib`'s
    /// table, `+0x10(file)` (open), `+0(buf, len, 0, 0)` (read), `+8(0)`
    /// (wait); `call(mem, function, args)` runs one (cdecl). Returns what
    /// the wait returns.
    pub fn dvd_file_read<M: Mem>(
        mem: &mut M,
        file: u32,
        buf: u32,
        len: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table = get_cd(mem);
        let open: u32 = mem.read(table.wrapping_add(0x10));
        call(mem, open, &[file]);
        let read: u32 = mem.read(table);
        call(mem, read, &[buf, len, 0, 0]);
        let wait: u32 = mem.read(table.wrapping_add(8));
        call(mem, wait, &[0])
    }

    /// `TOCDGetFileName(file, out)` (`0x008ab7d0`): `g_CdLib`'s `+0x10(file)`
    /// then `+0x58(out)`; returns what the second returns.
    pub fn cd_get_file_name<M: Mem>(
        mem: &mut M,
        file: u32,
        out: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table = get_cd(mem);
        let open: u32 = mem.read(table.wrapping_add(0x10));
        call(mem, open, &[file]);
        let name: u32 = mem.read(table.wrapping_add(0x58));
        call(mem, name, &[out])
    }

    /// `CDfileName_PC(n)` (`0x00a42830`): `0x00a42c00` (passed in) of `n`
    /// relative to the current module (`g_CdLib + 0x20` added).
    pub fn cd_file_name_pc<M: Mem>(
        mem: &mut M,
        n: u32,
        name: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let at = mem
            .read::<u32>(get_cd(mem).wrapping_add(0x20))
            .wrapping_add(n);
        name(mem, at)
    }

    /// `GetCDFileName_PC(n)` (`0x00a429d0`): as [`cd_file_name_pc`] when
    /// `g_CdLib + 0x30` is set, else 0.
    pub fn get_cd_file_name_pc<M: Mem>(
        mem: &mut M,
        n: u32,
        name: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let at = mem
            .read::<u32>(get_cd(mem).wrapping_add(0x20))
            .wrapping_add(n);
        if mem.read::<u32>(get_cd(mem).wrapping_add(0x30)) == 0 {
            return 0;
        }
        name(mem, at)
    }

    /// `VFile::assign(handle)` (`0x0061aa10`, `thiscall`, `ret 4`): the
    /// handle at `+0xc`, the 64-bit position (`+0x10`) zeroed, the shared
    /// state (`*(this + 0x18)`) words `+0x10`, `+0x14`, `+8`, `+0xc` zeroed,
    /// the in-use byte `+0` set. Returns the state pointer (left in `eax`).
    pub fn vfile_assign(mem: &mut impl Mem, this: u32, handle: u32) -> u32 {
        mem.write(this.wrapping_add(0xc), handle);
        let state: u32 = mem.read(this.wrapping_add(0x18));
        mem.write(this.wrapping_add(0x10), 0u64);
        for off in [0x10, 0x14, 8, 0xc] {
            mem.write(state.wrapping_add(off), 0u32);
        }
        mem.write(this, 1u8);
        state
    }

    /// `VFile::fileRead(buf, n)` (`0x0061aa60`, `thiscall`, `ret 8`):
    /// `FileSys_Read(buf, n)` on the same object (passed in), its count
    /// added to the 64-bit position; the count (0 stays 0).
    pub fn vfile_file_read<M: Mem>(
        mem: &mut M,
        this: u32,
        buf: u32,
        n: u32,
        read: impl FnOnce(&mut M, u32, u32, u32) -> u32,
    ) -> u32 {
        let r = read(mem, this, buf, n);
        if r == 0 {
            return 0;
        }
        let at = this.wrapping_add(0x10);
        let pos: u64 = mem.read(at);
        mem.write(at, pos.wrapping_add(u64::from(r)));
        r
    }

    /// `BigFileStream::setStreamPrefix(path)` (`0x0061c3a0`, `thiscall`,
    /// `ret 4`): the path's length at `+0x3c`, a copy in a new block (`new[]`
    /// of the length plus one) at `+0x40`. Returns the address past the
    /// copy's terminator.
    pub fn set_stream_prefix(
        mem: &mut impl Mem,
        os: &mut impl crate::heap::Os,
        this: u32,
        path: u32,
    ) -> u32 {
        let mut len = 0u32;
        while mem.read::<u8>(path.wrapping_add(len)) != 0 {
            len += 1;
        }
        mem.write(this.wrapping_add(0x3c), len);
        let copy = crate::heap::user_malloc(mem, os, len.wrapping_add(1));
        mem.write(this.wrapping_add(0x40), copy);
        let mut k = 0u32;
        loop {
            let c: u8 = mem.read(path.wrapping_add(k));
            mem.write(copy.wrapping_add(k), c);
            k += 1;
            if c == 0 {
                return copy.wrapping_add(k);
            }
        }
    }

    /// The stream path prefix (`g_BasePath`, `0x00cc9698`).
    pub const STREAM_PREFIX: ffx_bridge::Global<u32> = ffx_bridge::Global::new(0x00cc_9698);

    /// `PStreamFile::GetStreamPrefix()` (`0x00607d30`).
    pub fn stream_prefix(mem: &impl Mem) -> u32 {
        STREAM_PREFIX.get(mem)
    }

    fn c_str(mem: &impl Mem, p: u32) -> Vec<u8> {
        let mut v = Vec::new();
        loop {
            let c: u8 = mem.read(p.wrapping_add(u32::try_from(v.len()).unwrap_or(u32::MAX)));
            if c == 0 {
                return v;
            }
            v.push(c);
        }
    }

    fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
        if needle.is_empty() {
            return Some(0);
        }
        hay.windows(needle.len()).position(|w| w == needle)
    }

    /// [`unify_filename`]'s PC path for `path` (no size limit).
    fn unified(mem: &impl Mem, path: &[u8]) -> Vec<u8> {
        let mut buf = match find(path, b".cgfx.phyre") {
            Some(at) => {
                let mut v = path[..at].to_vec();
                v.extend_from_slice(b".fx.phyre");
                v
            }
            None => path.to_vec(),
        };
        let prefix_at = stream_prefix(mem);
        let prefix = c_str(mem, prefix_at);
        let has_prefix = find(&buf, &prefix).is_some();
        let pre = if has_prefix { Vec::new() } else { prefix };
        match find(&buf, b"GCM") {
            Some(at) => {
                let mut v = pre;
                v.extend_from_slice(&buf[..at]);
                v.extend_from_slice(b"D3D11");
                v.extend_from_slice(&buf[at + 3..]);
                v
            }
            None => {
                let mut v = pre;
                v.append(&mut buf);
                v
            }
        }
    }

    /// `getSa_MenuTextureName(out, n)` (`0x00774ae0`): the help picture
    /// path for the language (`TOGetFFXLang`, passed in): `help`,
    /// `help_us`, `help_fr`, `help_es`, `help_de`, `help_it` for 0..=5,
    /// `help_kr` and `help_ch` for 9 and 10, through [`unify_filename`]
    /// into a 0x100-byte buffer, then `sprintf(out, that, n)` (its `%d`).
    /// For the other languages the original formats an uninitialised stack
    /// buffer; here `out` becomes empty.
    pub fn help_texture_name(mem: &mut impl Mem, out: u32, n: i32, lang: u32) {
        let src = match lang {
            0 => 0x00b5_3144,
            1 => 0x00b5_3288,
            2 => 0x00b5_31c8,
            3 => 0x00b5_3248,
            4 => 0x00b5_3188,
            5 => 0x00b5_3208,
            9 => 0x00b5_32c8,
            10 => 0x00b5_3308,
            _ => {
                mem.write(out, 0u8);
                return;
            }
        };
        let src = mem.rebase(src);
        let path = c_str(mem, src);
        let fmt = unified(mem, &path);
        // sprintf_s(buf, 0x100, ...) leaves an empty format when it overflows
        let fmt = if fmt.len() >= 0x100 { Vec::new() } else { fmt };
        let mut text = Vec::with_capacity(fmt.len() + 12);
        let mut k = 0;
        while k < fmt.len() {
            match (fmt[k], fmt.get(k + 1)) {
                (b'%', Some(b'd')) => {
                    text.extend_from_slice(n.to_string().as_bytes());
                    k += 2;
                }
                (b'%', Some(b'%')) => {
                    text.push(b'%');
                    k += 2;
                }
                (c, _) => {
                    text.push(c);
                    k += 1;
                }
            }
        }
        text.push(0);
        mem.write_bytes(out, &text);
    }

    /// `fiosUnifyFilename(src, out, size)` (`0x00679820`): `out` cleared
    /// (`size` bytes), an overflow message printed (`Phyre_PhyrePrintf(1,
    /// fmt, src)`, passed in) when `src` is longer than `size`; then the
    /// PC path: `.cgfx.phyre` becomes `.fx.phyre`, `GCM` becomes `D3D11`,
    /// and the stream prefix goes in front unless the path already holds it
    /// (`sprintf_s(out, size, ...)`: an empty `out` and -1 when it does not
    /// fit). Returns the length written.
    pub fn unify_filename<M: Mem>(
        mem: &mut M,
        src: u32,
        out: u32,
        size: u32,
        print: impl FnOnce(&mut M, u32, u32, u32),
    ) -> u32 {
        mem.write_bytes(out, &vec![0; size as usize]);
        let path = c_str(mem, src);
        if size.cast_signed() < i32::try_from(path.len()).unwrap_or(i32::MAX) {
            let fmt = mem.rebase(0x00b4_4dbc);
            print(mem, 1, fmt, src);
        }
        let text = unified(mem, &path);
        if text.len() >= size as usize {
            // MSVCR110's `sprintf_s` on a short buffer: it writes `size - 1`
            // bytes and a NUL, then clears the first byte and calls the
            // invalid-parameter handler (which ends the game; the oracle's
            // returns -1).
            if size > 0 {
                let n = size as usize - 1;
                mem.write_bytes(out, &text[..n]);
                mem.write(out.wrapping_add(size - 1), 0u8);
                mem.write(out, 0u8);
            }
            return u32::MAX;
        }
        mem.write_bytes(out, &text);
        mem.write(
            out.wrapping_add(u32::try_from(text.len()).unwrap_or(0)),
            0u8,
        );
        u32::try_from(text.len()).unwrap_or(u32::MAX)
    }

    /// `VFile::fileSeek(offset, whence)` (`0x0061ac30`, `thiscall`, `ret 8`):
    /// the 64-bit position (`+0x10`) set from the start (0), the current
    /// position (1) or the end (2, the size at `*(this + 0xc) + 8`); -1 for
    /// another `whence` (nothing moved), a negative `offset` or an `offset`
    /// past the size (both after moving). Returns the position's low word.
    /// (The bound is checked on `offset` itself, as the original does.)
    pub fn vfile_seek(mem: &mut impl Mem, this: u32, offset: i32, whence: u32) -> u32 {
        let off = i64::from(offset);
        let at = this.wrapping_add(0x10);
        let entry: u32 = mem.read(this.wrapping_add(0xc));
        match whence {
            0 => mem.write(at, off.cast_unsigned()),
            1 => {
                let pos: u64 = mem.read(at);
                mem.write(at, pos.wrapping_add(off.cast_unsigned()));
            }
            2 => {
                let size: u64 = mem.read(entry.wrapping_add(8));
                mem.write(at, size.wrapping_sub(off.cast_unsigned()));
            }
            _ => return u32::MAX,
        }
        if offset < 0 {
            return u32::MAX;
        }
        let entry: u32 = mem.read(this.wrapping_add(0xc));
        let size: u64 = mem.read(entry.wrapping_add(8));
        if off.cast_unsigned() > size {
            return u32::MAX;
        }
        mem.read(at)
    }

    /// `0x008aba60(file, name)`: through `g_CdLib`'s table, `+0x10(file)`
    /// (open) and `+0x14(name)` (the size); for a non-zero size a block of
    /// it from `malloc` (`user_malloc`), read into with
    /// [`dvd_file_read`]`(file, name, block)`. The block, or 0.
    pub fn dvd_file_load<M: Mem>(
        mem: &mut M,
        file: u32,
        name: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        malloc: impl FnOnce(&mut M, u32) -> u32,
    ) -> u32 {
        let table = get_cd(mem);
        let open: u32 = mem.read(table.wrapping_add(0x10));
        call(mem, open, &[file]);
        let size_of: u32 = mem.read(table.wrapping_add(0x14));
        let size = call(mem, size_of, &[name]);
        if size == 0 {
            return 0;
        }
        let block = malloc(mem, size);
        if block == 0 {
            return 0;
        }
        dvd_file_read(mem, file, name, block, call);
        block
    }

    /// The CD table `0x008abab0` keeps (`0x01865f04`).
    pub const CD_TABLE_008ABAB0: Global<u32> = Global::new(0x0186_5f04);

    /// `0x008abab0(file, name, buf)`: `g_CdLib`'s table kept in
    /// [`CD_TABLE_008ABAB0`]; `+0x10(file)`, `+0x14(name)` (the size), and
    /// for a non-zero size `+0(name, buf, 0, 0)` (read), what that returns
    /// (0 for no size).
    pub fn dvd_file_start_read<M: Mem>(
        mem: &mut M,
        file: u32,
        name: u32,
        buf: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let table = get_cd(mem);
        CD_TABLE_008ABAB0.set(mem, table);
        let open: u32 = mem.read(table.wrapping_add(0x10));
        call(mem, open, &[file]);
        let table = CD_TABLE_008ABAB0.get(mem);
        let size_of: u32 = mem.read(table.wrapping_add(0x14));
        if call(mem, size_of, &[name]) == 0 {
            return 0;
        }
        let table = CD_TABLE_008ABAB0.get(mem);
        let read: u32 = mem.read(table);
        call(mem, read, &[name, buf, 0, 0])
    }
}
