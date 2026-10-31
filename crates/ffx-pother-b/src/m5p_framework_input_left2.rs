//! The leftovers of list `phyre-framework-input` (M5 stage 1, round
//! "left2"): a 2D normalisation through `_CIsqrt`, the crash-dump
//! exception filter and the window's screen-mode switch.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `_CIsqrt` (the CRT intrinsic's import thunk: `ST0` in and out).
pub const CI_SQRT: u32 = 0x0094_97b8;

/// The crash dump's imports (slots): `GetCurrentThreadId`, `CreateFileW`,
/// `GetCurrentProcess`, `GetProcessId`, `CloseHandle`.
pub const DUMP_SLOTS: [u32; 5] = [
    0x00b0_c108,
    0x00b0_c188,
    0x00b0_c19c,
    0x00b0_c1a0,
    0x00b0_c0dc,
];
/// `MiniDumpWriteDump`'s import thunk (`jmp [0x00b0c72c]`, stdcall, 7
/// words).
pub const MINI_DUMP_WRITE_DUMP: u32 = 0x0094_8d3c;
/// Whether a crash writes a dump (a word; 0: no).
pub const DUMP_ENABLED: u32 = 0x00c3_0f90;
/// The dump file's name (a wide string).
pub const DUMP_PATH: u32 = 0x00b4_0e30;
/// `MiniDumpWithDataSegs | ...` as the filter passes it.
pub const DUMP_TYPE: u32 = 0x1824;

/// The screen-mode switch's imports (slots): `ChangeDisplaySettingsW`,
/// `GetWindowLongW`, `SetWindowLongW`, `GetWindowRect`,
/// `AdjustWindowRect`, `SetWindowPos`, `GetModuleHandleW`, `LoadIconW`,
/// `SendMessageW`.
pub const SCREEN_SLOTS: [u32; 9] = [
    0x00b0_c658,
    0x00b0_c6c0,
    0x00b0_c6bc,
    0x00b0_c680,
    0x00b0_c6cc,
    0x00b0_c6e0,
    0x00b0_c114,
    0x00b0_c65c,
    0x00b0_c6ac,
];
/// The CRT's `memset` thunk (cdecl).
pub const MEMSET: u32 = 0x0094_964c;
/// The screen settings' pointer (`+4`/`+6` two signed 16-bit sizes, `+8`
/// the mode: 0 windowed, 1 full screen, 2 borderless).
pub const G_SCREEN_SETTINGS: u32 = 0x022f_b504;
/// The window.
pub const G_HWND: u32 = 0x00cc_9ce8;
/// The window's icon, loaded once.
pub const G_ICON: u32 = 0x00cc_9cec;
/// The application whose `+0x39c` (16 bits) names the icon resource.
pub const G_APPLICATION: u32 = 0x00cc_9cd8;
/// The words of [`bridge::apply_screen_mode`]'s frame: the `DEVMODEW`
/// (0x9c bytes) then the `RECT`.
pub const SCREEN_FRAME_WORDS: u32 = 0x2b;

/// The ports over game memory.
pub mod bridge {
    use super::{
        CI_SQRT, Conv, DUMP_ENABLED, DUMP_PATH, DUMP_SLOTS, DUMP_TYPE, G_APPLICATION, G_HWND,
        G_ICON, G_SCREEN_SETTINGS, Host, MEMSET, MINI_DUMP_WRITE_DUMP, Mem, SCREEN_SLOTS,
    };
    use ffx_bridge::x87::Reg;

    fn import(h: &impl Mem, slot: u32) -> u32 {
        h.read(h.rebase(slot))
    }

    /// `0x0062a0d0` (cdecl `(out, v)`): `v`'s two floats divided by their
    /// length, the length `_CIsqrt(f32(f32(x * x) + y * y))` rounded to a
    /// float as the original stores each step (x87 model,
    /// `ffx_bridge::x87`); `out`.
    pub fn normalize2(h: &mut impl Host, out: u32, v: u32) -> u32 {
        let x = f32::from_bits(h.read(v));
        let y = f32::from_bits(h.read(v + 4));
        let rx = Reg::load(x);
        let xx = (rx * rx).store();
        let ry = Reg::load(y);
        let sum = (Reg::load(xx) + ry * ry).store();
        let addr = h.rebase(CI_SQRT);
        let len = h.call_intrinsic(addr, Reg::load(sum).value());
        let len = Reg::load(len);
        let nx = (Reg::load(x) / len).store();
        let ny = len.divr_m32(y).store();
        h.write(out, nx.to_bits());
        h.write(out + 4, ny.to_bits());
        out
    }

    /// `CrashDump_UnhandledExceptionFilter(pointers)` (`0x006268e0`,
    /// stdcall): the dump's exception information `{thread id, pointers,
    /// client pointers 1}` in `info` (the port's frame; the original's),
    /// and when dumps are on: the file created, `MiniDumpWriteDump` of
    /// this process into it, the file closed. 1
    /// (`EXCEPTION_EXECUTE_HANDLER`).
    pub fn crash_filter(h: &mut impl Host, pointers: u32, info: u32) -> u32 {
        let f = import(h, DUMP_SLOTS[0]);
        let tid = h.call(f, Conv::Stdcall, &[]);
        h.write(info, tid);
        h.write(info + 4, pointers);
        h.write(info + 8, 1u32);
        let enabled: u32 = h.read(h.rebase(DUMP_ENABLED));
        if enabled != 0 {
            let path = h.rebase(DUMP_PATH);
            let f = import(h, DUMP_SLOTS[1]);
            let file = h.call(f, Conv::Stdcall, &[path, 0x4000_0000, 0, 0, 2, 0, 0]);
            let f = import(h, DUMP_SLOTS[2]);
            let process = h.call(f, Conv::Stdcall, &[]);
            let f = import(h, DUMP_SLOTS[3]);
            let pid = h.call(f, Conv::Stdcall, &[process]);
            let f = import(h, DUMP_SLOTS[2]);
            let process = h.call(f, Conv::Stdcall, &[]);
            h.call_sym(
                MINI_DUMP_WRITE_DUMP,
                Conv::Stdcall,
                &[process, pid, file, DUMP_TYPE, info, 0, 0],
            );
            let f = import(h, DUMP_SLOTS[4]);
            h.call(f, Conv::Stdcall, &[file]);
        }
        1
    }

    /// `PWindowWin32Base::applyScreenMode` (`0x0062bf00`, cdecl; also
    /// reached through the jump at `0x0062bef0`): a `DEVMODEW` (zeroed by
    /// `memset`, size 0x9c, 32 bits, the two sizes, fields 0x1c0000) and a
    /// `RECT` in `frame` (the port's; the original's frame at `ebp -
    /// 0xb0`). By the mode: the display settings reset (0, 2) or changed to
    /// the `DEVMODEW` (1); the window's style (0xca0000 windowed, else a
    /// popup), its rectangle adjusted for a client area of the two sizes,
    /// and the window placed at 0, 0 (behind every other for 0 and 2);
    /// windowed, the icon (loaded once) sent with `WM_SETICON`. Other
    /// modes do nothing more after the `memset`.
    pub fn apply_screen_mode(h: &mut impl Host, frame: u32) {
        let settings: u32 = h.read(h.rebase(G_SCREEN_SETTINGS));
        let height = i32::from(h.read::<i16>(settings + 4)).cast_unsigned();
        let width = i32::from(h.read::<i16>(settings + 6)).cast_unsigned();
        let mode: u32 = h.read(settings + 8);
        let dm = frame;
        let rect = frame + 0x9c;
        h.call_sym(MEMSET, Conv::Cdecl, &[dm, 0, 0x9c]);
        h.write(dm + 0x24, 0x9cu16);
        h.write(dm + 0x6c, width);
        h.write(dm + 0x70, height);
        h.write(dm + 0x68, 0x20u32);
        h.write(dm + 0x28, 0x001c_0000u32);
        let (display, style, after) = match mode {
            0 => ([0, 0x4000_0000], 0x00ca_0000, 0xffff_fffe),
            1 => ([dm, 4], 0x8000_0000, 0),
            2 => ([0, 0x4000_0000], 0x8000_0000, 0xffff_fffe),
            _ => return,
        };
        let hwnd_cell = h.rebase(G_HWND);
        let f = import(h, SCREEN_SLOTS[0]);
        h.call(f, Conv::Stdcall, &display); // call width: 2
        let hwnd: u32 = h.read(hwnd_cell);
        let f = import(h, SCREEN_SLOTS[1]);
        h.call(f, Conv::Stdcall, &[hwnd, 0xffff_fff0]);
        let hwnd: u32 = h.read(hwnd_cell);
        let f = import(h, SCREEN_SLOTS[2]);
        h.call(f, Conv::Stdcall, &[hwnd, 0xffff_fff0, style]);
        let hwnd: u32 = h.read(hwnd_cell);
        let f = import(h, SCREEN_SLOTS[3]);
        h.call(f, Conv::Stdcall, &[hwnd, rect]);
        h.write(rect, 0u32);
        h.write(rect + 4, 0u32);
        h.write(rect + 8, width);
        h.write(rect + 0xc, height);
        let f = import(h, SCREEN_SLOTS[4]);
        h.call(f, Conv::Stdcall, &[rect, style, 0]);
        let left: u32 = h.read(rect);
        let top: u32 = h.read(rect + 4);
        let right: u32 = h.read(rect + 8);
        let bottom: u32 = h.read(rect + 0xc);
        let hwnd: u32 = h.read(hwnd_cell);
        let f = import(h, SCREEN_SLOTS[5]);
        h.call(
            f,
            Conv::Stdcall,
            &[
                hwnd,
                after,
                0,
                0,
                right.wrapping_sub(left),
                bottom.wrapping_sub(top),
                0x40,
            ],
        );
        if mode != 0 {
            return;
        }
        let icon_cell = h.rebase(G_ICON);
        let mut icon: u32 = h.read(icon_cell);
        if icon == 0 {
            let app: u32 = h.read(h.rebase(G_APPLICATION));
            let id = u32::from(h.read::<u16>(app + 0x39c));
            let f = import(h, SCREEN_SLOTS[6]);
            let module = h.call(f, Conv::Stdcall, &[0]);
            let f = import(h, SCREEN_SLOTS[7]);
            icon = h.call(f, Conv::Stdcall, &[module, id]);
            h.write(icon_cell, icon);
        }
        let hwnd: u32 = h.read(hwnd_cell);
        let f = import(h, SCREEN_SLOTS[8]);
        h.call(f, Conv::Stdcall, &[hwnd, 0x80, 1, icon]);
    }
}
