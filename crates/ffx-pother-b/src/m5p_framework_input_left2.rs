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

/// `readAll`'s callees: `PhyreMalloc(size, align)`, `PhyreFree`,
/// `PhyreMalloc(size)` (cdecl), `PArray<PUInt8>::resize(n)` (thiscall),
/// the chunk list's release `(&list)` (cdecl).
pub const READ_CALLS: [u32; 5] = [
    0x0042_fb80,
    0x0042_fb20,
    0x0042_fb40,
    0x0044_a010,
    0x0057_dcb0,
];
/// `memcpy` (cdecl).
pub const MEMCPY: u32 = 0x0094_9256;
/// `0x006285b0`'s callees and data: the `PStreamReaderFile` constructor
/// `(path)`, `isOpen`, `readAll(&array, terminate)`, the file's close (on
/// the stream `+0x14`), the destructor (all thiscall); the message for a
/// file that does not open, `sscanf`'s import slot, its format.
pub const VALUE_FILE: [u32; 8] = [
    0x0057_66d0,
    0x0057_6980,
    0x0057_dd60,
    0x0060_7d80,
    0x0057_67c0,
    0x00b4_03c8,
    0x00b0_c418,
    0x00b4_03dc,
];
/// `Phyre_PhyrePrintf(level, format, ...)` (cdecl).
pub const PRINTF: u32 = 0x0043_52f0;
/// The words of [`bridge::read_value_file`]'s frame (the stream, the
/// array, the value).
pub const VALUE_FRAME_WORDS: u32 = 10;

/// `_stricmp`'s import slot (cdecl).
pub const STRICMP_SLOT: u32 = 0x00b0_c398;
/// The script's pad button names and their bytes in a pad's state block
/// (`+0xd + index`), in the order `0x00622d40`/`0x0062c730` compare them.
pub const BUTTONS: [(u32, u32); 10] = [
    (0x00b4_0290, 0xd),
    (0x00b2_bf3c, 0xc),
    (0x00b4_0298, 0xe),
    (0x00b4_02a4, 0xb),
    (0x00b8_f9bc, 0x14),
    (0x00b4_02ac, 0x13),
    (0x00b8_f8a0, 0x17),
    (0x00b8_f8a4, 0x19),
    (0x00b8_f890, 0x1a),
    (0x00b8_f898, 0x18),
];
/// The message for an unknown button name.
pub const BUTTON_UNKNOWN: u32 = 0x00b4_02b4;

/// The touch device's lock and unlock (`PCriticalSectionSimple`, thiscall
/// on the device `+0x17c`).
pub const TOUCH_LOCK: [u32; 2] = [0x0043_1920, 0x0043_1a80];
/// `PMeshParticleSystemBase`'s constructor's callees: the `+0x14` block's
/// reset and `PSamplerStateD3D11`'s constructor (thiscall).
pub const PARTICLE_CALLS: [u32; 2] = [0x009f_c970, 0x0058_3b20];
/// Its vtable and the float it puts in four words.
pub const PARTICLE_DATA: [u32; 2] = [0x00b7_51b8, 0x00b4_12c4];

/// The `DirectInput` pad's update: the `DirectInput` object's cell, the pad
/// index getter (thiscall), the mappings' update (thiscall), the
/// connected and disconnected messages.
pub const DINPUT_PAD: [u32; 5] = [
    0x00cc_9cd4,
    0x0062_8170,
    0x0062_d550,
    0x00b4_109c,
    0x00b4_10d8,
];
/// `DIERR_INPUTLOST`.
pub const DIERR_INPUTLOST: u32 = 0x8007_001e;
/// The words of `DIJOYSTATE2` ([`bridge::dinput_pad_update`]'s frame).
pub const DIJOYSTATE2_WORDS: u32 = 0x44;

/// The `XInput` pad's update: `XInputGetState` and `XInputSetState` (the
/// import thunks, stdcall), unused, the connected and disconnected
/// messages, the vibration scale (a double).
pub const XINPUT: [u32; 6] = [
    0x009d_a25a,
    0x009d_a260,
    0,
    0x00b4_1118,
    0x00b4_1150,
    0x00b1_70d8,
];
/// [`XINPUT_BUTTONS`]' marks for the two triggers (their bytes, not bits).
pub const XINPUT_LEFT_TRIGGER: u32 = 100;
/// See [`XINPUT_LEFT_TRIGGER`].
pub const XINPUT_RIGHT_TRIGGER: u32 = 101;
/// The `wButtons` bit (or trigger) behind each of the pad's 16 button
/// bytes (`+0x398..`).
pub const XINPUT_BUTTONS: [u32; 16] = [
    14,
    12,
    13,
    15,
    8,
    9,
    XINPUT_LEFT_TRIGGER,
    XINPUT_RIGHT_TRIGGER,
    5,
    4,
    6,
    7,
    0,
    3,
    1,
    2,
];
/// The words of [`bridge::xinput_pad_update`]'s frame.
pub const XINPUT_FRAME_WORDS: u32 = 8;

/// The ports over game memory.
pub mod bridge {
    use super::{BUTTON_UNKNOWN, BUTTONS, PARTICLE_CALLS, PARTICLE_DATA, STRICMP_SLOT, TOUCH_LOCK};
    use super::{
        CI_SQRT, Conv, DUMP_ENABLED, DUMP_PATH, DUMP_SLOTS, DUMP_TYPE, G_APPLICATION, G_HWND,
        G_ICON, G_SCREEN_SETTINGS, Host, MEMCPY, MEMSET, MINI_DUMP_WRITE_DUMP, Mem, PRINTF,
        READ_CALLS, SCREEN_SLOTS, VALUE_FILE,
    };
    use super::{DIERR_INPUTLOST, DINPUT_PAD};
    use super::{XINPUT, XINPUT_BUTTONS, XINPUT_LEFT_TRIGGER, XINPUT_RIGHT_TRIGGER};
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

    /// `PStream::readAll(array, terminate)` (`0x0057dd60`, thiscall on the
    /// stream, `ret 8`): the stream's remaining bytes into a
    /// `PArray<PUInt8>` (count `+0`, bit 31 external; data `+4`), a NUL
    /// after them for `terminate`. A stream of known size (`+0x10` less
    /// the position `+0xc`) reads once into the array (grown to fit with
    /// `PhyreMalloc(n, 1)`); one of unknown size reads 0x800-byte chunks
    /// into a list of 0x808-byte blocks (`PhyreMalloc`) and copies them
    /// over after the array's resize (`0x0044a010`). The array's count
    /// becomes at most what was read (an emptied owned array loses its
    /// block). 0; 0xd when an allocation fails; the resize's error (the
    /// chunk list freed by `0x0057dcb0` through `list`, a word of the
    /// port's frame; the original's).
    pub fn read_all(h: &mut impl Host, stream: u32, array: u32, terminate: u32, list: u32) -> u32 {
        let nul = terminate & 0xff;
        let size: u32 = h.read(stream + 0x10);
        let pos: u32 = h.read(stream + 0xc);
        let total;
        if size != u32::MAX && size.wrapping_sub(pos) != u32::MAX {
            let rem = size.wrapping_sub(pos);
            let need = nul.wrapping_add(rem);
            let head: u32 = h.read(array);
            if head & 0x7fff_ffff < need && need != head {
                let mut block = 0;
                if need != 0 {
                    block = h.call_sym(READ_CALLS[0], Conv::Cdecl, &[need, 1]);
                    if block == 0 {
                        return 0xd;
                    }
                }
                let old: u32 = h.read(array + 4);
                let head: u32 = h.read(array);
                if old != block && head & 0x8000_0000 == 0 && old != 0 {
                    h.call_sym(READ_CALLS[1], Conv::Cdecl, &[old]);
                }
                h.write(array + 4, block);
                h.write(array, need);
            }
            let data: u32 = h.read(array + 4);
            let n = h.call_virtual(stream, 5, &[data, rem]);
            let p: u32 = h.read(stream + 0xc);
            h.write(stream + 0xc, p.wrapping_add(n));
            total = nul.wrapping_add(n);
            if nul != 0 {
                h.write(data.wrapping_add(rem), 0u8);
            }
        } else {
            let mut count = 0u32;
            h.write(list, 0u32);
            loop {
                let chunk = h.call_sym(READ_CALLS[2], Conv::Cdecl, &[0x808]);
                if chunk == 0 {
                    let mut b: u32 = h.read(list);
                    while b != 0 {
                        let next: u32 = h.read(b);
                        h.call_sym(READ_CALLS[1], Conv::Cdecl, &[b]);
                        b = next;
                    }
                    return 0xd;
                }
                h.write(chunk, 0u32);
                let n = h.call_virtual(stream, 5, &[chunk + 4, 0x800]);
                let p: u32 = h.read(stream + 0xc);
                h.write(stream + 0xc, p.wrapping_add(n));
                h.write(chunk + 0x804, n);
                if n == 0 {
                    h.call_sym(READ_CALLS[1], Conv::Cdecl, &[chunk]);
                    break;
                }
                let mut at = list;
                if h.read::<u32>(list) != 0 {
                    loop {
                        at = h.read(at);
                        if h.read::<u32>(at) == 0 {
                            break;
                        }
                    }
                }
                h.write(at, chunk);
                count = count.wrapping_add(n);
                h.write(chunk, 0u32);
            }
            total = nul.wrapping_add(count);
            if h.read::<u32>(array) & 0x7fff_ffff < total {
                let r = h.call_sym(READ_CALLS[3], Conv::Thiscall(array), &[total]);
                if r != 0 {
                    h.call_sym(READ_CALLS[4], Conv::Cdecl, &[list]);
                    return r;
                }
            }
            let mut dst: u32 = h.read(array + 4);
            let mut b: u32 = h.read(list);
            while b != 0 {
                let next: u32 = h.read(b);
                let n: u32 = h.read(b + 0x804);
                h.call_sym(MEMCPY, Conv::Cdecl, &[dst, b + 4, n]);
                dst = dst.wrapping_add(n);
                h.call_sym(READ_CALLS[1], Conv::Cdecl, &[b]);
                b = next;
            }
            if nul != 0 {
                h.write(dst, 0u8);
            }
        }
        let head: u32 = h.read(array);
        let len = (head & 0x7fff_ffff).min(total);
        let head = (head & 0x8000_0000) | len;
        h.write(array, head);
        if head == 0 {
            let data: u32 = h.read(array + 4);
            h.call_sym(READ_CALLS[1], Conv::Cdecl, &[data]);
            h.write(array + 4, 0u32);
        }
        0
    }

    /// `0x006285b0` (cdecl `(path)`): the file read whole (a
    /// `PStreamReaderFile` on `frame`, its array at `frame + 0x1c`, the
    /// scanned value at `frame + 0x24`: the original's frame at `ebp -
    /// 0x34`), each line scanned with `sscanf(line, 0x00b403dc, &value)`;
    /// the last value (0 when the read failed), 0x13 with a message when
    /// the file does not open. The stream closed and destroyed, the
    /// array's block freed unless external.
    pub fn read_value_file(h: &mut impl Host, path: u32, frame: u32) -> u32 {
        let stream = frame;
        let array = frame + 0x1c;
        let value = frame + 0x24;
        h.call_sym(VALUE_FILE[0], Conv::Thiscall(stream), &[path]);
        h.write(value, 0u32);
        let result;
        let is_open = h.call_sym(VALUE_FILE[1], Conv::Thiscall(stream), &[]) & 0xff;
        if is_open == 0 {
            let msg = h.rebase(VALUE_FILE[5]);
            h.call_sym(PRINTF, Conv::Cdecl, &[2, msg, path]);
            result = 0x13;
        } else {
            h.write(array, 0u32);
            h.write(array + 4, 0u32);
            let r = h.call_sym(VALUE_FILE[2], Conv::Thiscall(stream), &[array, 1]);
            h.call_sym(VALUE_FILE[3], Conv::Thiscall(stream + 0x14), &[]);
            let data: u32 = h.read(array + 4);
            if r == 0 {
                let len = h.read::<u32>(array) & 0x7fff_ffff;
                let end = data.wrapping_add(len);
                let mut p = data;
                if p < end {
                    let sscanf: u32 = h.read(h.rebase(VALUE_FILE[6]));
                    let fmt = h.rebase(VALUE_FILE[7]);
                    loop {
                        let line = p;
                        if h.read::<u8>(p) != b'\n' {
                            while p < end {
                                p = p.wrapping_add(1);
                                if h.read::<u8>(p) == b'\n' {
                                    break;
                                }
                            }
                        }
                        h.call(sscanf, Conv::Cdecl, &[line, fmt, value]);
                        p = p.wrapping_add(1);
                        if p >= end {
                            break;
                        }
                    }
                }
            }
            result = h.read(value);
            let head: u32 = h.read(array);
            let data: u32 = h.read(array + 4);
            if head & 0x8000_0000 == 0 && data != 0 {
                h.call_sym(READ_CALLS[1], Conv::Cdecl, &[data]);
            }
            h.write(array + 4, 0u32);
            h.write(array, 0u32);
        }
        h.call_sym(VALUE_FILE[4], Conv::Thiscall(stream), &[]);
        result
    }

    /// The pad button names the script commands know (case-insensitive),
    /// with each button's byte in the pad's state block (`+0xd + index`).
    fn button_index(h: &mut impl Host, name: u32) -> Option<u32> {
        let stricmp: u32 = h.read(h.rebase(STRICMP_SLOT));
        for (s, index) in BUTTONS {
            let s = h.rebase(s);
            if h.call(stricmp, Conv::Cdecl, &[name, s]) == 0 {
                return Some(index);
            }
        }
        None
    }

    /// The first pad of the application (`[[app + 0x2b8] + 0x38]`), 0 for
    /// none.
    fn first_pad(h: &mut impl Host, app: u32) -> u32 {
        let input: u32 = h.read(app + 0x2b8);
        h.read(input + 0x38)
    }

    /// `PApplicationScriptProxy::buttonPress(app, name)` (`0x00622d40`,
    /// cdecl): the named button of the first pad set to 1; 0, or 5 with a
    /// message for an unknown name; 0 without an application, a name or
    /// a pad.
    pub fn button_press(h: &mut impl Host, app: u32, name: u32) -> u32 {
        if app == 0 || name == 0 {
            return 0;
        }
        let pad = first_pad(h, app);
        if pad == 0 {
            return 0;
        }
        set_button(h, pad, name, 1)
    }

    /// `0x0062c730` (cdecl `(app, name, value byte)`): the named button
    /// of the first pad set to `value`; as [`button_press`], but the name
    /// is not checked for 0.
    pub fn button_set(h: &mut impl Host, app: u32, name: u32, value: u32) -> u32 {
        if app == 0 {
            return 0;
        }
        let pad = first_pad(h, app);
        if pad == 0 {
            return 0;
        }
        #[allow(clippy::cast_possible_truncation)] // the value is a byte
        set_button(h, pad, name, value as u8)
    }

    fn set_button(h: &mut impl Host, pad: u32, name: u32, value: u8) -> u32 {
        if let Some(index) = button_index(h, name) {
            h.write(pad + 0xd + index, value);
            return 0;
        }
        let msg = h.rebase(BUTTON_UNKNOWN);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, msg, name]);
        5
    }

    /// An intrusive list node `{next, prev, owner +8, .. +0x28, +0x2c}`
    /// unlinked and emptied when its owner is `owner`.
    fn unlink_if_owned(h: &mut impl Host, node: u32, owner: u32) {
        if h.read::<u32>(node + 8) != owner {
            return;
        }
        let next: u32 = h.read(node);
        let prev: u32 = h.read(node + 4);
        h.write(next + 4, prev);
        h.write(prev, next);
        h.write(node, node);
        h.write(node + 4, node);
        h.write(node + 8, 0u32);
        h.write(node + 0x28, 0u32);
        h.write(node + 0x2c, 0u32);
    }

    /// The input map's release (`0x0062d170`, thiscall, from
    /// `PApplication::onExit`): when a mapping is set (`+0x9c`) its five
    /// nodes (`+0x18` step `0x30`) that belong to it are unlinked and
    /// `+4..+0x14` cleared; then for each of the 18 devices (`+0x38`) its
    /// four nodes (`+0x6c` step `0x30`) that belong to the device.
    pub fn release_input_map(h: &mut impl Host, this: u32) {
        let map: u32 = h.read(this + 0x9c);
        if map != 0 {
            let base = map + 0x18;
            for k in 0..5 {
                let owner: u32 = h.read(this + 0x9c);
                unlink_if_owned(h, base + 0x30 * k, owner);
            }
            for off in [0x14, 0x10, 0xc, 8, 4] {
                h.write(this + off, 0u32);
            }
        }
        for i in 0..18 {
            let dev: u32 = h.read(this + 0x38 + 4 * i);
            if dev == 0 {
                continue;
            }
            for k in 0..4 {
                unlink_if_owned(h, dev + 0x6c + 0x30 * k, dev);
            }
        }
    }

    /// `{next, prev}` list node `node` unlinked from its list and put at
    /// the tail of `list`.
    fn move_to_tail(h: &mut impl Host, node: u32, list: u32) {
        let next: u32 = h.read(node);
        let prev: u32 = h.read(node + 4);
        h.write(next + 4, prev);
        h.write(prev, next);
        h.write(node, node);
        h.write(node + 4, node);
        let tail: u32 = h.read(list + 4);
        h.write(node, list);
        h.write(node + 4, tail);
        h.write(tail, node);
        h.write(list + 4, node);
    }

    /// The first node of list `head` after `node` (0 at the end).
    fn after(h: &impl Mem, node: u32, head: u32) -> u32 {
        let next: u32 = h.read(node);
        if next == head { 0 } else { next }
    }

    /// `PInputDeviceTouch::update` (`0x0062de50`, thiscall): under the
    /// lock (`+0x17c`) the state reset (`+0x158` 0.0, `+0x18`, `+0x15c`,
    /// `+0xc4` 1.0, `+0x160` 16 bits, `+0xc8`/`+0xcc` 0.0); the touches of
    /// the active list (`+0x164`) that are no longer down (`+0x18` 0) move
    /// to the free list (`+0x174`); the active and pending (`+0x16c`)
    /// lists swap; the device's virtual `+0x18`; every pending touch goes
    /// back to the active list with `+0x18` cleared. The unlock's `eax`.
    pub fn touch_update(h: &mut impl Host, this: u32) -> u32 {
        let lock = this + 0x17c;
        h.call_sym(TOUCH_LOCK[0], Conv::Thiscall(lock), &[]);
        h.write(this + 0x158, 0u32);
        h.write(this + 0x18, 0u32);
        h.write(this + 0x15c, 0u32);
        h.write(this + 0xc4, 1.0f32.to_bits());
        h.write(this + 0x160, 0u16);
        h.write(this + 0xc8, 0u32);
        h.write(this + 0xcc, 0u32);
        let active = this + 0x164;
        let free = this + 0x174;
        let pending = this + 0x16c;
        let first: u32 = h.read(active);
        let mut node = if first == active { 0 } else { first };
        while node != 0 {
            let next = after(h, node, active);
            if h.read::<u32>(node + 0x18) == 0 {
                move_to_tail(h, node, free);
            }
            node = next;
        }
        // swap the two lists' ends
        let a_first: u32 = h.read(active);
        let p_first: u32 = h.read(pending);
        h.write(pending, a_first);
        h.write(active, p_first);
        let p_last: u32 = h.read(pending + 4);
        let a_last: u32 = h.read(active + 4);
        h.write(pending + 4, a_last);
        h.write(active + 4, p_last);
        for (list, was_empty) in [(active, p_first == pending), (pending, a_first == active)] {
            if was_empty {
                h.write(list + 4, list);
                h.write(list, list);
            } else {
                let f: u32 = h.read(list);
                h.write(f + 4, list);
                let l: u32 = h.read(list + 4);
                h.write(l, list);
            }
        }
        h.call_virtual(this, 6, &[]);
        let first: u32 = h.read(pending);
        let mut node = if first == pending { 0 } else { first };
        while node != 0 {
            let next = after(h, node, pending);
            let n: u32 = h.read(node);
            let p: u32 = h.read(node + 4);
            h.write(n + 4, p);
            h.write(p, n);
            h.write(node, node);
            h.write(node + 4, node);
            h.write(node + 0x18, 0u32);
            let tail: u32 = h.read(active + 4);
            h.write(node, active);
            h.write(node + 4, tail);
            h.write(tail, node);
            h.write(active + 4, node);
            node = next;
        }
        h.call_sym(TOUCH_LOCK[1], Conv::Thiscall(lock), &[])
    }

    /// `PMeshParticleSystemBase`'s constructor (`0x009fc9e0`, thiscall):
    /// the base words, the vtable, the `+0x14` block (`0x009fc970`), the
    /// sampler state at `+0x3084` (`0x00583b20`), the words `+0x30a8..
    /// +0x30c0` 0, four floats of `0x00b412c4` (0.2) at `+0x31c4`, the
    /// rest 0; the sampler's first five bytes (0, 0, 2, 2, 2) unless its
    /// flags (`+0x1c` bit 7) say it is fixed. `this`.
    pub fn particle_system_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.write(this + 4, 0u32);
        h.write(this + 8, 0u32);
        h.write(this + 0xc, 0u32);
        h.write(this + 0x10, 0x100u16);
        let vt = h.rebase(PARTICLE_DATA[0]);
        h.write(this, vt);
        h.call_sym(PARTICLE_CALLS[0], Conv::Thiscall(this + 0x14), &[]);
        let sampler = this + 0x3084;
        h.call_sym(PARTICLE_CALLS[1], Conv::Thiscall(sampler), &[]);
        let c = f32::from_bits(h.read(h.rebase(PARTICLE_DATA[1])));
        let c = ffx_bridge::x87::round_trip(c);
        for off in (0x30a8..=0x30c0).step_by(4) {
            h.write(this + off, 0u32);
        }
        for off in [0x31c4, 0x31c8, 0x31cc, 0x31d0] {
            h.write(this + off, c.to_bits());
        }
        h.write(this + 0x31d8, 0u32);
        for off in [0x31d4, 0x31dc, 0x31e0, 0x31e4] {
            h.write(this + off, 0u32);
        }
        for (i, v) in [0u8, 0, 2, 2, 2].into_iter().enumerate() {
            let flags: u32 = h.read(sampler + 0x1c);
            if flags & 0x80 == 0 {
                h.write(sampler + i as u32, v);
            }
        }
        this
    }

    /// `PInputDevicePadDirectInput::update` (`0x0062d9d0`, thiscall, the
    /// device vtable's `+0x14`): with a `DirectInput` device (`+0x14`): the
    /// connection state from the `DirectInput` object's `vt[5](guid +0x130)`
    /// (a message when it changes, `+0x69`); a `DIJOYSTATE2` in `state`
    /// (0x110 bytes, zeroed; the port's frame, the original's) filled by
    /// `GetDeviceState` after a successful `Poll`, else the device
    /// acquired again until it stops answering `DIERR_INPUTLOST`; the
    /// axes (`+0x370/+0x37c/+0x374/+0x380`), the 16 buttons' top bits
    /// (`+0x398`) and the POV's eight directions (`+0x3a4..+0x3a7`); then
    /// the mappings updated (`0x0062d550`).
    pub fn dinput_pad_update(h: &mut impl Host, this: u32, state: u32) {
        let dev: u32 = h.read(this + 0x14);
        if dev == 0 {
            return;
        }
        let di: u32 = h.read(h.rebase(DINPUT_PAD[0]));
        let vt: u32 = h.read(di);
        let f: u32 = h.read(vt + 0x14);
        let r = h.call(f, Conv::Stdcall, &[di, this + 0x130]);
        let connected: u8 = h.read(this + 0x69);
        let change = if r == 0 {
            connected == 0
        } else {
            connected != 0
        };
        if change {
            let index = h.call_sym(DINPUT_PAD[1], Conv::Thiscall(this), &[]);
            let msg = h.rebase(if r == 0 { DINPUT_PAD[3] } else { DINPUT_PAD[4] });
            h.call_sym(PRINTF, Conv::Cdecl, &[2, msg, index]);
            h.write(this + 0x69, u8::from(r == 0));
        }
        h.call_sym(MEMSET, Conv::Cdecl, &[state, 0, 0x110]);
        let vt: u32 = h.read(dev);
        let poll: u32 = h.read(vt + 0x64);
        if h.call(poll, Conv::Stdcall, &[dev]).cast_signed() < 0 {
            loop {
                let vt: u32 = h.read(dev);
                let acquire: u32 = h.read(vt + 0x1c);
                if h.call(acquire, Conv::Stdcall, &[dev]) != DIERR_INPUTLOST {
                    break;
                }
            }
        } else {
            let get: u32 = h.read(vt + 0x24);
            h.call(get, Conv::Stdcall, &[dev, 0x110, state]);
        }
        for (from, to) in [(0, 0x370), (4, 0x37c), (8, 0x374), (0x14, 0x380)] {
            let v: u32 = h.read(state + from);
            h.write(this + to, v);
        }
        for i in 0..16 {
            let b: u8 = h.read(state + 0x30 + i);
            h.write(this + 0x398 + i, b >> 7);
        }
        let pov: u32 = h.read(state + 0x20);
        h.write(this + 0x3a4, 0u32);
        let set: &[u32] = match pov {
            0 => &[0],
            4500 => &[0, 1],
            9000 => &[1],
            13500 => &[1, 2],
            18000 => &[2],
            22500 => &[2, 3],
            27000 => &[3],
            31500 => &[0, 3],
            _ => &[],
        };
        for &d in set {
            h.write(this + 0x3a4 + d, 1u8);
        }
        h.call_sym(DINPUT_PAD[2], Conv::Thiscall(this), &[]);
    }

    /// `fistp dword` with the rounding mode set to truncate: in range,
    /// toward zero; else the integer indefinite.
    fn fistp_trunc(x: f64) -> u32 {
        if x.is_nan() || x >= 2_147_483_648.0 || x <= -2_147_483_649.0 {
            0x8000_0000
        } else {
            #[allow(clippy::cast_possible_truncation)] // in range, truncated
            let v = x as i32;
            v.cast_unsigned()
        }
    }

    /// An axis as the pad keeps it: the stick's signed 16 bits divided by
    /// 128 (toward zero).
    fn axis(raw: u16) -> u32 {
        (i32::from(raw.cast_signed()) / 128).cast_unsigned()
    }

    /// `PInputDevicePadXInput::update` (`0x0062dba0`, thiscall, a device
    /// vtable slot): `XInputGetState(pad +0x3b0)` into the `XINPUT_STATE`
    /// at `frame + 0x10` (zeroed first; the port's frame, the original's
    /// `ebp - 0x24`); a new packet (`+0x3b4`) updates the pad: on success
    /// the axes (/128, the Y axes negated) and the 16 buttons, else all
    /// cleared; a message when the connection changes (`+0x69`). Then the
    /// vibration `{left, right}` at `frame` from the motors (`+0x58`,
    /// `+0x59`) times the scale at `0x00b170d8` (truncated) is set, the
    /// state kept at `+0x3b4..+0x3c0`, and the mappings updated.
    pub fn xinput_pad_update(h: &mut impl Host, this: u32, frame: u32) {
        let state = frame + 0x10;
        for i in 0..4 {
            h.write(state + 4 * i, 0u32);
        }
        let pad: u32 = h.read(this + 0x3b0);
        let r = h.call_sym(XINPUT[0], Conv::Stdcall, &[pad, state]);
        let packet: u32 = h.read(state);
        if packet != h.read::<u32>(this + 0x3b4) {
            let connected: u8 = h.read(this + 0x69);
            if r == 0 {
                if connected == 0 {
                    xinput_message(h, this, XINPUT[3]);
                    h.write(this + 0x69, 1u8);
                }
                let buttons: u32 = h.read(state + 4);
                let lx: u16 = h.read(state + 8);
                let ly: u16 = h.read(state + 0xa);
                let rx: u16 = h.read(state + 0xc);
                let ry: u16 = h.read(state + 0xe);
                h.write(this + 0x370, axis(lx));
                h.write(this + 0x37c, axis(ly).wrapping_neg());
                h.write(this + 0x374, axis(rx));
                h.write(this + 0x380, axis(ry).wrapping_neg());
                for (i, bit) in XINPUT_BUTTONS.into_iter().enumerate() {
                    let v = match bit {
                        XINPUT_LEFT_TRIGGER => u8::from(h.read::<u8>(state + 6) > 0),
                        XINPUT_RIGHT_TRIGGER => u8::from(h.read::<u8>(state + 7) > 0),
                        _ => u8::from(buttons >> bit & 1 != 0),
                    };
                    h.write(this + 0x398 + i as u32, v);
                }
            } else {
                if connected != 0 {
                    xinput_message(h, this, XINPUT[4]);
                    h.write(this + 0x69, 0u8);
                }
                for off in [0x370, 0x37c, 0x374, 0x380, 0x398, 0x39c, 0x3a0, 0x3a4] {
                    h.write(this + off, 0u32);
                }
            }
        }
        let scale = f64::from_bits(h.read(h.rebase(XINPUT[5])));
        h.write(frame, 0u32);
        for (i, motor) in [0x58u32, 0x59].into_iter().enumerate() {
            let v = f64::from(h.read::<u8>(this + motor));
            #[allow(clippy::cast_possible_truncation)] // the low word
            let w = fistp_trunc(v * scale) as u16;
            h.write(frame + 2 * i as u32, w);
        }
        let pad: u32 = h.read(this + 0x3b0);
        h.call_sym(XINPUT[1], Conv::Stdcall, &[pad, frame]);
        for i in 0..4 {
            let v: u32 = h.read(state + 4 * i);
            h.write(this + 0x3b4 + 4 * i, v);
        }
        h.call_sym(DINPUT_PAD[2], Conv::Thiscall(this), &[]);
    }

    /// The connection message `(2, msg, pad index, pad)`: the original
    /// pushes the pad number for the index getter, which leaves it on the
    /// stack as the message's fourth word.
    fn xinput_message(h: &mut impl Host, this: u32, msg: u32) {
        let pad: u32 = h.read(this + 0x3b0);
        let index = h.call_sym(DINPUT_PAD[1], Conv::Thiscall(this), &[]);
        let msg = h.rebase(msg);
        h.call_sym(PRINTF, Conv::Cdecl, &[2, msg, index, pad]);
    }
}
