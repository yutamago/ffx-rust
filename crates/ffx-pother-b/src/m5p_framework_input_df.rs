//! List `phyre-framework-input`'s run-time frames and wrapper-owned
//! addresses (M5 stage 1, round "design-frames"):
//! - `PApplicationScriptProxy::Framerate` (`0x006235d0`): the argument
//!   copied into an `_alloca_probe_16` buffer, cut at its first comma,
//!   printed with the application's frame rate;
//! - `PStreamFileWin32::openFile` (`0x00607f40`): the path as UTF-16 and as
//!   a normalised archive name in two such buffers, the archives first,
//!   then `CreateFileW`; the mod wrapper (`mods.rs`) owns the address and
//!   calls this port;
//! - `PTimerWin32::GetTime` (`0x0042fd80`): seconds from
//!   `GetSystemTimePreciseAsFileTime` or the performance counter; the
//!   harness's clock wrapper owns the address and calls this port.
//!
//! The alloca bridges are split at the alloca (docs/porting.md, "Run-time
//! frames and varargs"): the part before it returns the sizes, the shim
//! carves the frames, the part after takes their addresses.

use ffx_graphic::host::{Conv, Host};

/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;
/// `crt_memcpy_thunk` (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// `"Framerate:%s: %f %d\n"`.
pub const FRAMERATE_FORMAT: u32 = 0x00b4_0278;
/// `openFile`'s log line (`PhyrePrintf(2, ..., path)`).
pub const OPEN_FORMAT: u32 = 0x00b3_ebc4;
/// `"%s"`.
pub const PERCENT_S: u32 = 0x00b0_e160;
/// `Str_Utf8ToWide(src, dst, dst_end)` (cdecl).
pub const UTF8_TO_WIDE: u32 = 0x0060_7c40;
/// `Path_ToLowerAscii(path)` (cdecl).
pub const TO_LOWER: u32 = 0x0060_7d50;
/// `BigFileStream_get()` (cdecl).
pub const FILE_SYS: u32 = 0x0061_bdb0;
/// `BigFileStream_openFile(stream; name)` (thiscall, ret 4).
pub const ARCHIVE_OPEN: u32 = 0x0061_bf10;
/// `sprintf`'s import slot (cdecl).
pub const SPRINTF_SLOT: u32 = 0x00b0_c3dc;
/// `KERNEL32` import slots (stdcall): `CreateFileW` (7 words),
/// `CloseHandle` (1), `QueryPerformanceCounter` (1).
pub const CREATE_FILE_SLOT: u32 = 0x00b0_c138;
/// See [`CREATE_FILE_SLOT`].
pub const CLOSE_HANDLE_SLOT: u32 = 0x00b0_c0dc;
/// See [`CREATE_FILE_SLOT`].
pub const QPC_SLOT: u32 = 0x00b0_c0e4;
/// `GetSystemTimePreciseAsFileTime` when the system has it (stdcall,
/// `(&filetime)`), else 0.
pub const PRECISE_TIME: u32 = 0x00c9_004c;
/// The performance counter `QueryPerformanceCounter` fills (`i64`).
pub const COUNTER: u32 = 0x00c9_0068;
/// Seconds per performance-counter tick (`f64`).
pub const SECONDS_PER_TICK: u32 = 0x00c9_0078;
/// FILETIME ticks per second, `1e7` (`f64`).
pub const FILETIME_RATE: u32 = 0x00b0_dba8;
/// `INVALID_HANDLE_VALUE`.
pub const INVALID_HANDLE: u32 = 0xffff_ffff;

/// The originals over game memory.
pub mod bridge {
    use super::{
        ARCHIVE_OPEN, CLOSE_HANDLE_SLOT, COUNTER, CREATE_FILE_SLOT, Conv, FILE_SYS, FILETIME_RATE,
        FRAMERATE_FORMAT, Host, INVALID_HANDLE, MEMCPY, OPEN_FORMAT, PERCENT_S, PRECISE_TIME,
        PRINTF, QPC_SLOT, SECONDS_PER_TICK, SPRINTF_SLOT, TO_LOWER, UTF8_TO_WIDE,
    };
    use ffx_bridge::x87;

    /// `strlen` as the originals inline it.
    fn str_len(h: &impl Host, s: u32) -> u32 {
        let mut n = 0u32;
        while h.read::<u8>(s.wrapping_add(n)) != 0 {
            n = n.wrapping_add(1);
        }
        n
    }

    /// The bytes `Framerate(app, text)` asks `_alloca_probe_16` for:
    /// the text with its NUL.
    pub fn framerate_size(h: &impl Host, text: u32) -> u32 {
        str_len(h, text).wrapping_add(1)
    }

    /// The copy the `Framerate` shape makes in its buffer: `memcpy(buf,
    /// text, bytes)`, then the first comma after the first byte (when that
    /// is not the NUL) ends it.
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

    /// `PApplicationScriptProxy::Framerate(app, text)` (cdecl) -> 0, with
    /// `buf` the [`framerate_size`] bytes: the text up to its first comma,
    /// then `PhyrePrintf(2, "Framerate:%s: %f %d\n", buf, (double)
    /// app+0x30, app+0x34)`.
    pub fn framerate(h: &mut impl Host, app: u32, text: u32, buf: u32, bytes: u32) -> u32 {
        copy_to_comma(h, buf, text, bytes);
        let rate = h.read::<u32>(app.wrapping_add(0x34));
        let fps = x87::load(h.read::<u32>(app.wrapping_add(0x30))).to_bits();
        let fmt = h.rebase(FRAMERATE_FORMAT);
        #[allow(clippy::cast_possible_truncation)] // the double's two words
        h.call_sym(
            PRINTF,
            Conv::Cdecl,
            &[2, fmt, buf, fps as u32, (fps >> 32) as u32, rate],
        );
        0
    }

    /// The sizes `openFile(file; path, ...)` asks `_alloca_probe_16` for,
    /// after its log line `PhyrePrintf(2, ..., path)`: `None` for a null or
    /// empty path (no buffers, 1), else `(len, wide, narrow)`: the path's
    /// length, `2 * (len + 1)` bytes for the UTF-16 copy and `len + 1` for
    /// the archive name.
    pub fn open_file_sizes(h: &mut impl Host, path: u32) -> Option<(u32, u32, u32)> {
        if path == 0 || h.read::<u8>(path) == 0 {
            return None;
        }
        let fmt = h.rebase(OPEN_FORMAT);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, fmt, path]);
        let len = str_len(h, path);
        Some((
            len,
            len.wrapping_mul(2).wrapping_add(2),
            len.wrapping_add(1),
        ))
    }

    /// `CreateFileW(wide, access, share, 0, disposition, 0x8000000
    /// (sequential scan), 0)`.
    fn create_file(h: &mut impl Host, wide: u32, access: u32, share: u32, disposition: u32) -> u32 {
        let f: u32 = h.read(h.rebase(CREATE_FILE_SLOT));
        h.call(
            f,
            Conv::Stdcall,
            &[wide, access, share, 0, disposition, 0x0800_0000, 0],
        )
    }

    /// The two names `openFile` and `File_Probe` build: `Str_Utf8ToWide(path,
    /// wide, wide + 2 * (len + 1))`, `sprintf(narrow, "%s", path)`, every
    /// `\` of it made `/`, `Path_ToLowerAscii(narrow)`.
    fn names(h: &mut impl Host, path: u32, len: u32, wide: u32, narrow: u32) {
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
    }

    /// `PStreamFileWin32::openFile(file; path, mode, 3 unused)` (thiscall,
    /// ret 0x14) -> 0, 1 (no path), 0xa (no file) or 0xc (made but not
    /// readable), with `bufs` the `(len, wide, narrow)` length and
    /// buffers of [`open_file_sizes`] (`None`: no path). The archive entry
    /// (`BigFileStream_openFile`) goes to `file + 4`; without one the file
    /// on disk, its handle to `file`: read opens (mode byte set) read-only,
    /// write opens `OPEN_ALWAYS` for writing, or when that fails, a
    /// read-only open closed again.
    pub fn open_file(
        h: &mut impl Host,
        file: u32,
        path: u32,
        mode: u32,
        bufs: Option<(u32, u32, u32)>,
    ) -> u32 {
        let Some((len, wide, narrow)) = bufs else {
            return 1;
        };
        names(h, path, len, wide, narrow);
        let fs = h.call_sym(FILE_SYS, Conv::Cdecl, &[]);
        let entry = h.call_sym(ARCHIVE_OPEN, Conv::Thiscall(fs), &[narrow]);
        h.write(file.wrapping_add(4), entry);
        if entry != 0 {
            return 0;
        }
        if mode & 0xff != 0 {
            let handle = create_file(h, wide, 1, 1, 3);
            h.write(file, handle);
            return if handle == INVALID_HANDLE { 0xa } else { 0 };
        }
        let handle = create_file(h, wide, 2, 0, 4);
        h.write(file, handle);
        if handle != INVALID_HANDLE {
            return 0;
        }
        let handle = create_file(h, wide, 1, 1, 3);
        h.write(file, handle);
        if handle == INVALID_HANDLE {
            return 0xa;
        }
        let close: u32 = h.read(h.rebase(CLOSE_HANDLE_SLOT));
        h.call(close, Conv::Stdcall, &[handle]);
        h.write(file, INVALID_HANDLE);
        0xc
    }

    /// `PTimerWin32::GetTime()` (cdecl) -> seconds in `ST0`: with
    /// `GetSystemTimePreciseAsFileTime`, the FILETIME at `frame` (2 words)
    /// as an unsigned count (`fild` of its low 63 bits plus 2^63 for the
    /// top bit, one rounding) over 1e7; else `QueryPerformanceCounter`
    /// into its global, times the seconds per tick.
    pub fn get_time(h: &mut impl Host, frame: u32) -> f64 {
        let precise: u32 = h.read(h.rebase(PRECISE_TIME));
        if precise != 0 {
            h.call(precise, Conv::Stdcall, &[frame]);
            let ticks = u64::from(h.read::<u32>(frame))
                | (u64::from(h.read::<u32>(frame.wrapping_add(4))) << 32);
            #[allow(clippy::cast_precision_loss)] // the original's one rounding
            let t = ticks as f64;
            return t / h.read::<f64>(h.rebase(FILETIME_RATE));
        }
        let qpc: u32 = h.read(h.rebase(QPC_SLOT));
        let counter = h.rebase(COUNTER);
        h.call(qpc, Conv::Stdcall, &[counter]);
        #[allow(clippy::cast_precision_loss)] // fild, then fstp qword
        let c = h.read::<i64>(counter) as f64;
        c * h.read::<f64>(h.rebase(SECONDS_PER_TICK))
    }
}
