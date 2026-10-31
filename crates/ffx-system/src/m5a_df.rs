//! List `system-a`'s run-time frames and the harness-owned key query (M5
//! stage 1, round "design-frames"):
//! - `File_Probe` (`0x00607e00`): the path as UTF-16 and as a normalised
//!   archive name in two `_alloca_probe_16` buffers; the archives, else a
//!   loose file on disk;
//! - the script commands `0x00624410` (the input-log replay: its name up
//!   to the first comma, copied into such a buffer) and `0x00624da0`
//!   (`PApplication::screenshot`: the back buffer read back and written as
//!   a 24-bit BMP to the file named the same way);
//! - `Input_KeyHeld` (`0x00629d50`), which the harness's key wrapper
//!   (`harness.rs`) owns and calls.
//!
//! The alloca bridges are split at the alloca (docs/porting.md, "Run-time
//! frames and varargs"): the part before it returns the sizes, the shim
//! carves the frames, the part after takes their addresses.

/// The originals over game memory.
pub mod bridge {
    use ffx_graphic::host::{Conv, Host};

    /// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
    pub const PRINTF: u32 = 0x0043_52f0;
    /// `crt_memcpy_thunk` (cdecl).
    pub const MEMCPY: u32 = 0x0094_9256;
    /// `"%s"`.
    pub const PERCENT_S: u32 = 0x00b0_e160;
    /// `Str_Utf8ToWide(src, dst, dst_end)` (cdecl).
    pub const UTF8_TO_WIDE: u32 = 0x0060_7c40;
    /// `Path_ToLowerAscii(path)` (cdecl).
    pub const TO_LOWER: u32 = 0x0060_7d50;
    /// `BigFileStream_get()` (cdecl).
    pub const FILE_SYS: u32 = 0x0061_bdb0;
    /// `FileSys_Exists(stream; name)` (thiscall, ret 4) -> `al`.
    pub const EXISTS: u32 = 0x0061_be40;
    /// `sprintf`'s import slot (cdecl).
    pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
    /// `KERNEL32` import slots (stdcall): `CreateFileW` (7 words),
    /// `SetEndOfFile` (1), `CloseHandle` (1).
    pub const CREATE_FILE_SLOT: u32 = 0x00b0_c138;
    /// See [`CREATE_FILE_SLOT`].
    pub const SET_END_OF_FILE_SLOT: u32 = 0x00b0_c144;
    /// See [`CREATE_FILE_SLOT`].
    pub const CLOSE_HANDLE_SLOT: u32 = 0x00b0_c0dc;
    /// `INVALID_HANDLE_VALUE`.
    pub const INVALID_HANDLE: u32 = 0xffff_ffff;
    /// `PApplicationScriptProxy::readInputLog(app, name)` (cdecl).
    pub const READ_INPUT_LOG: u32 = 0x0062_b5f0;
    /// `"PApplication::screenshot: %s \n"`.
    pub const SCREENSHOT_FORMAT: u32 = 0x00b4_0768;
    /// `PRenderer::syncRender(renderer)` (thiscall).
    pub const SYNC_RENDER: u32 = 0x005b_c3f0;
    /// `PRenderScopeLock_enter(lock)` / `_leave(lock)` (thiscall).
    pub const LOCK_ENTER: u32 = 0x0046_3d50;
    /// See [`LOCK_ENTER`].
    pub const LOCK_LEAVE: u32 = 0x0046_3e90;
    /// The render interface singleton (a function-local static), its
    /// guard word (bit 0: made), its constructor (thiscall) and the
    /// destructor `atexit` gets.
    pub const RENDER_INTERFACE: u32 = 0x00c9_4f00;
    /// See [`RENDER_INTERFACE`].
    pub const RENDER_INTERFACE_GUARD: u32 = 0x00c9_628c;
    /// See [`RENDER_INTERFACE`].
    pub const RENDER_INTERFACE_CTOR: u32 = 0x004f_ee70;
    /// See [`RENDER_INTERFACE`].
    pub const RENDER_INTERFACE_DTOR: u32 = 0x00af_b710;
    /// `atexit(f)` (cdecl).
    pub const ATEXIT: u32 = 0x0094_9207;
    /// `PRenderInterface::getViewportSize(&w, &h, &pitch)` (thiscall,
    /// ret 0xc) -> error.
    pub const VIEWPORT_SIZE: u32 = 0x0059_4f40;
    /// The back buffer read back into `pixels` (thiscall on the render
    /// interface, ret 4) -> error.
    pub const READ_PIXELS: u32 = 0x0058_ec50;
    /// `PhyreMalloc(size, align)` / `PhyreFree(p)` (cdecl).
    pub const MALLOC: u32 = 0x0042_fb80;
    /// See [`MALLOC`].
    pub const FREE: u32 = 0x0042_fb20;
    /// `PStreamWriterFile`'s constructor `(path)` (thiscall, ret 4), its
    /// `isOpen` (`al`), `write(data, n)` (ret 8, 0 or 9), `close` and
    /// destructor (thiscall).
    pub const WRITER_CTOR: u32 = 0x009f_06d0;
    /// See [`WRITER_CTOR`].
    pub const WRITER_IS_OPEN: u32 = 0x009f_08b0;
    /// See [`WRITER_CTOR`].
    pub const WRITER_WRITE: u32 = 0x009f_06a0;
    /// See [`WRITER_CTOR`].
    pub const WRITER_CLOSE: u32 = 0x009f_08a0;
    /// See [`WRITER_CTOR`].
    pub const WRITER_DTOR: u32 = 0x009f_0740;
    /// `0x00624da0`'s frame below `ebp` that callees see by address:
    /// `ebp - 0x44`..`ebp - 0x10` (the writer at `-0x44`, its error word
    /// `-0x40`; the scope lock `-0x1c`, width `-0x18`, height `-0x14`,
    /// pitch `-0x20`).
    pub const SCREENSHOT_FRAME: u32 = 0x34;
    /// Offsets in [`SCREENSHOT_FRAME`] (from `ebp - 0x44`).
    const WRITER: u32 = 0;
    const PITCH: u32 = 0x24;
    const LOCK: u32 = 0x28;
    const WIDTH: u32 = 0x2c;
    const HEIGHT: u32 = 0x30;

    /// `strlen` as the originals inline it.
    fn str_len(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The sizes `File_Probe(path)` asks `_alloca_probe_16` for: `None`
    /// for a null or empty path (no buffers, 0), else `(len, wide,
    /// narrow)`: the length, `2 * (len + 1)` bytes for the UTF-16 copy,
    /// `len + 1` for the archive name.
    pub fn probe_sizes(h: &impl Host, path: u32) -> Option<(u32, u32, u32)> {
        if path == 0 || h.read::<u8>(path) == 0 {
            return None;
        }
        let len = str_len(h, path);
        Some((
            len,
            len.wrapping_mul(2).wrapping_add(2),
            len.wrapping_add(1),
        ))
    }

    /// `File_Probe(path)` (cdecl) -> `al`, with `bufs` the `(len, wide,
    /// narrow)` of [`probe_sizes`]: the UTF-16 path
    /// (`Str_Utf8ToWide(path, wide, wide + 2 * (len + 1))`) and the
    /// archive name (`sprintf(narrow, "%s", path)`, `\` made `/`,
    /// `Path_ToLowerAscii`); 1 when the archives have it
    /// (`FileSys_Exists`), else 1 when `CreateFileW(wide, read, share
    /// read, open existing, sequential)` opens it (then `SetEndOfFile`
    /// and `CloseHandle`), else 0.
    pub fn file_probe(h: &mut impl Host, path: u32, bufs: Option<(u32, u32, u32)>) -> u8 {
        let Some((len, wide, narrow)) = bufs else {
            return 0;
        };
        let end = wide.wrapping_add(len.wrapping_add(1).wrapping_mul(2));
        h.call_sym(UTF8_TO_WIDE, Conv::Cdecl, &[path, wide, end]);
        let sprintf: u32 = h.read(h.rebase(SPRINTF_SLOT));
        let fmt = h.rebase(PERCENT_S);
        h.call(sprintf, Conv::Cdecl, &[narrow, fmt, path]);
        if narrow != 0 && h.read::<u8>(narrow) != 0 {
            let mut p = narrow;
            loop {
                if h.read::<u8>(p) == b'\\' {
                    h.write(p, b'/');
                }
                p = p.wrapping_add(1);
                if h.read::<u8>(p) == 0 {
                    break;
                }
            }
        }
        h.call_sym(TO_LOWER, Conv::Cdecl, &[narrow]);
        let fs = h.call_sym(FILE_SYS, Conv::Cdecl, &[]);
        if h.call_sym(EXISTS, Conv::Thiscall(fs), &[narrow]) & 0xff != 0 {
            return 1;
        }
        let create: u32 = h.read(h.rebase(CREATE_FILE_SLOT));
        let handle = h.call(create, Conv::Stdcall, &[wide, 1, 1, 0, 3, 0x0800_0000, 0]);
        if handle == INVALID_HANDLE {
            return 0;
        }
        let set_end: u32 = h.read(h.rebase(SET_END_OF_FILE_SLOT));
        let close: u32 = h.read(h.rebase(CLOSE_HANDLE_SLOT));
        h.call(set_end, Conv::Stdcall, &[handle]);
        h.call(close, Conv::Stdcall, &[handle]);
        1
    }

    /// The bytes the script commands' name buffer takes (`0x00624410`,
    /// `0x00624da0`): the argument with its NUL.
    pub fn name_size(h: &impl Host, text: u32) -> u32 {
        str_len(h, text).wrapping_add(1)
    }

    /// `memcpy(buf, text, bytes)`, then the first comma after the first
    /// byte (when that is not the NUL) ends it.
    fn copy_to_comma(h: &mut impl Host, buf: u32, text: u32, bytes: u32) {
        h.call_sym(MEMCPY, Conv::Cdecl, &[buf, text, bytes]);
        if h.read::<u8>(buf) == 0 {
            return;
        }
        let mut p = buf;
        loop {
            p = p.wrapping_add(1);
            match h.read::<u8>(p) {
                b',' => {
                    h.write(p, 0u8);
                    return;
                }
                0 => return,
                _ => {}
            }
        }
    }

    /// `0x00624410(app, text)` (cdecl) -> `eax`: the text up to its first
    /// comma in `buf` ([`name_size`] bytes), then
    /// `PApplicationScriptProxy::readInputLog(app, buf)`, its `eax`.
    pub fn read_input_log_cmd(h: &mut impl Host, app: u32, text: u32, buf: u32, bytes: u32) -> u32 {
        copy_to_comma(h, buf, text, bytes);
        h.call_sym(READ_INPUT_LOG, Conv::Cdecl, &[app, buf])
    }

    /// The render interface singleton, made on first use as the original
    /// inlines it (guard bit 0, constructor, `atexit` of its destructor).
    fn render_interface(h: &mut impl Host) -> u32 {
        let guard = h.rebase(RENDER_INTERFACE_GUARD);
        let ri = h.rebase(RENDER_INTERFACE);
        let g: u32 = h.read(guard);
        if g & 1 == 0 {
            h.write(guard, g | 1);
            h.call_sym(RENDER_INTERFACE_CTOR, Conv::Thiscall(ri), &[]);
            let dtor = h.rebase(RENDER_INTERFACE_DTOR);
            h.call_sym(ATEXIT, Conv::Cdecl, &[dtor]);
        }
        ri
    }

    /// `PApplication::screenshot(app, text)` (`0x00624da0`, cdecl; without
    /// its SEH frame) -> 0 or an error, with `buf` the name buffer
    /// ([`name_size`] bytes) and `frame` its locals ([`SCREENSHOT_FRAME`]
    /// bytes at the original's `ebp - 0x44`): the name up to its first
    /// comma; for an application, `PhyrePrintf(2, ..., buf)`, the
    /// renderer at `app + 0x124` flushed (its method `+8`, `syncRender`),
    /// the viewport size under the render lock (an error is returned),
    /// `PhyreMalloc(pitch * height + 0x38, 4)` (0xd without), the pixels
    /// read back under the lock after a 2-byte pad and the 0x36-byte BMP
    /// header (an error frees and is returned), red and blue swapped,
    /// the header written, and the whole block from the header on
    /// written to the file (9 when the write fails; the writer's error
    /// word when it does not open); freed.
    pub fn screenshot(
        h: &mut impl Host,
        app: u32,
        text: u32,
        buf: u32,
        bytes: u32,
        frame: u32,
    ) -> u32 {
        copy_to_comma(h, buf, text, bytes);
        if app == 0 {
            return 0;
        }
        let fmt = h.rebase(SCREENSHOT_FORMAT);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, fmt, buf]);
        let renderer = app.wrapping_add(0x124);
        let vt: u32 = h.read(renderer);
        let flush: u32 = h.read(vt.wrapping_add(8));
        h.call(flush, Conv::Thiscall(renderer), &[]);
        h.call_sym(SYNC_RENDER, Conv::Thiscall(renderer), &[]);
        let (lock, width, height, pitch) =
            (frame + LOCK, frame + WIDTH, frame + HEIGHT, frame + PITCH);
        h.write(width, 0u32);
        h.write(height, 0u32);
        h.write(pitch, 0u32);
        h.call_sym(LOCK_ENTER, Conv::Thiscall(lock), &[]);
        let ri = render_interface(h);
        let err = h.call_sym(VIEWPORT_SIZE, Conv::Thiscall(ri), &[width, height, pitch]);
        h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[]);
        if err != 0 {
            return err;
        }
        let size = h
            .read::<u32>(pitch)
            .wrapping_mul(h.read::<u32>(height))
            .wrapping_add(0x38);
        let block = h.call_sym(MALLOC, Conv::Cdecl, &[size, 4]);
        if block == 0 {
            return 0xd;
        }
        let bmp = block.wrapping_add(2);
        let pixels = bmp.wrapping_add(0x36);
        h.call_sym(LOCK_ENTER, Conv::Thiscall(lock), &[]);
        let ri = render_interface(h);
        let read = h.call_sym(READ_PIXELS, Conv::Thiscall(ri), &[pixels]);
        h.call_sym(LOCK_LEAVE, Conv::Thiscall(lock), &[]);
        let mut result = read;
        if read == 0 {
            let mut row = 0u32;
            while row < h.read::<u32>(height) {
                let mut p = row
                    .wrapping_mul(h.read::<u32>(pitch))
                    .wrapping_add(0x36)
                    .wrapping_add(bmp);
                let mut col = 0u32;
                while col < h.read::<u32>(width) {
                    let r: u8 = h.read(p.wrapping_add(2));
                    let b: u8 = h.read(p);
                    h.write(p, r);
                    h.write(p.wrapping_add(2), b);
                    col += 1;
                    p = p.wrapping_add(3);
                }
                row += 1;
            }
            h.write(bmp, 0x4d42u16);
            h.write(bmp + 2, 0u32);
            h.write(bmp + 6, 0u32);
            h.write(bmp + 0xa, 0x36u32);
            h.write(bmp + 0xe, 0x28u32);
            let (w, ht) = (h.read::<u32>(width), h.read::<u32>(height));
            h.write(bmp + 0x12, w);
            h.write(bmp + 0x16, ht);
            h.write(bmp + 0x1a, 0x0018_0001u32);
            for k in 0..6 {
                h.write(bmp + 0x1e + 4 * k, 0u32);
            }
            let writer = frame + WRITER;
            h.call_sym(WRITER_CTOR, Conv::Thiscall(writer), &[buf]);
            result = if h
                .call_sym(WRITER_IS_OPEN, Conv::Thiscall(writer), &[])
                .to_le_bytes()[0]
                == 0
            {
                h.read(writer.wrapping_add(4))
            } else if h.call_sym(WRITER_WRITE, Conv::Thiscall(writer), &[bmp, size]) != 0 {
                9
            } else {
                read
            };
            h.call_sym(WRITER_CLOSE, Conv::Thiscall(writer), &[]);
            h.call_sym(WRITER_DTOR, Conv::Thiscall(writer), &[]);
        }
        h.call_sym(FREE, Conv::Cdecl, &[block]);
        result
    }

    /// `Input_KeyHeld(devices; binding, device)` (thiscall, ret 8) ->
    /// `eax`, the answer in `al`: keyboard bindings are 28 and up; the
    /// binding less 28 (under 0x100) is a byte of the state at `+0x18` of keyboard
    /// `device` (under 7, `[devices + 0x2b8] + 0x80 + 4 * device`), held
    /// when not 0. The rest of `eax` is what the original leaves there:
    /// the device table's upper bytes before the key is known, the key's
    /// after.
    pub fn key_held(h: &impl Host, devices: u32, binding: u32, device: u32) -> u32 {
        let table: u32 = h.read(devices.wrapping_add(0x2b8));
        if device >= 7 {
            return table & !0xff;
        }
        let keyboard: u32 = h.read(
            table
                .wrapping_add(device.wrapping_mul(4))
                .wrapping_add(0x80),
        );
        if keyboard == 0 {
            return table & !0xff;
        }
        let key = binding.wrapping_sub(28);
        if key >= 0x100 {
            return key & !0xff;
        }
        key & !0xff | u32::from(h.read::<u8>(keyboard.wrapping_add(key).wrapping_add(0x18)) != 0)
    }
}
