//! The small functions of list `phyre-framework-input` (M5 stage 1, batch
//! 1): template copies the framework, input and stream classes share, and
//! the input devices' one-line virtual methods.
//!
//! - `PClassDescriptorForType<T>` destructors (unwind funclets and static
//!   teardown): the copy's vtable, then a tail jump to the parent
//!   destructor ([`TAIL_DTORS`], [`bridge::tail_dtor`]);
//! - scalar deleting destructors: the vtable and/or the class's own
//!   destructor, then the allocator when `flags & 1` ([`DELETES`],
//!   [`bridge::deleting_dtor`]);
//! - constant virtual methods ([`CONSTANTS`]), getters, the pad's button
//!   and vibration bytes, the timer and window helpers that call one
//!   Win32 import.
//!
//! Everything else (`PhyreFree`, `operator delete`, the import slots,
//! callees outside the list) stays a call to the original. Every port
//! returns the `eax` the original leaves where its callers can read it.

use ffx_bridge::{Mem, x87};
use ffx_graphic::host::{Conv, Host};

/// `Phyre::PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `operator delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `Phyre::PClassDescriptor::Destruct` (thiscall, no stack words).
pub const DESTRUCT: u32 = 0x0043_b3e0;
/// `Phyre::PSerialization::PStream::~PStream` (thiscall, no stack words).
pub const STREAM_DTOR: u32 = 0x0060_f0c0;
/// `Phyre::PSerialization::PStream::PStream(name)` (thiscall, `ret 4`).
pub const STREAM_CTOR: u32 = 0x0060_f050;
/// `Phyre::PFramework::PApplication::~PApplication` (thiscall).
pub const APPLICATION_DTOR: u32 = 0x0062_0bc0;
/// `PApplicationScriptBuffer`'s destructor body (thiscall).
pub const SCRIPT_BUFFER_DTOR: u32 = 0x0062_0d80;
/// The static-object unlink every `StaticDtor_Thiscall_*` reaches
/// (cdecl `(object)`: takes it out of the list at `0x00ca54f8`).
pub const STATIC_UNLINK: u32 = 0x0043_27b0;
/// `PStreamFileWin32::seek(offset)` (thiscall, `ret 4`).
pub const FILE_SEEK: u32 = 0x0060_8120;
/// `Phyre::PhyrePrintf(level, fmt, ...)` (cdecl, varargs).
pub const PRINTF: u32 = 0x0043_52f0;
/// `"%s\n"`-style format `PApplicationScriptProxy::Echo` prints with.
pub const ECHO_FORMAT: u32 = 0x00b1_2ad0;
/// The joypad-axis source's map getter (`[ecx + 0xc]`, thiscall).
pub const AXIS_MAP_GETTER: u32 = 0x00a2_e0c0;
/// `PMotionBlur`'s per-target check (thiscall `(a, b)`, `ret 8`, in the
/// list).
pub const MOTION_BLUR_CHECK: u32 = 0x00a2_35f0;
/// `PUtilityGameplay`'s spline evaluation (thiscall `(out, t)`, `ret 8`;
/// `eax` = `out`).
pub const SPLINE_EVALUATE: u32 = 0x0054_ba40;
/// `PStreamReader`'s vtable.
pub const STREAM_READER_VTABLE: u32 = 0x00b3_1430;
/// `PStreamWriter`'s vtable (`PStreamReader::PStreamReader` stores it too).
pub const STREAM_WRITER_VTABLE: u32 = 0x00b7_3d30;
/// `PInputDevice`'s vtable.
pub const INPUT_DEVICE_VTABLE: u32 = 0x00b3_fbf8;
/// `PProcessorProfileTimelineBase`'s vtable.
pub const TIMELINE_BASE_VTABLE: u32 = 0x00b3_13fc;
/// `PInputSource`'s vtable.
pub const INPUT_SOURCE_VTABLE: u32 = 0x00b7_0938;

/// Import slot of `KERNEL32!Sleep` (stdcall `(ms)`).
pub const SLEEP_SLOT: u32 = 0x00b0_c100;
/// Import slot of `WINMM!timeEndPeriod` (stdcall `(period)`).
pub const TIME_END_PERIOD_SLOT: u32 = 0x00b0_c708;
/// Import slot of `USER32!ChangeDisplaySettingsA` (stdcall `(mode, flags)`).
pub const CHANGE_DISPLAY_SETTINGS_SLOT: u32 = 0x00b0_c658;
/// Import slot of `USER32!ReleaseDC` (stdcall `(window, dc)`).
pub const RELEASE_DC_SLOT: u32 = 0x00b0_c6d8;
/// `g_hWnd`.
pub const G_HWND: u32 = 0x00cc_9ce8;
/// The byte `FFXApplication_pollInput*` set through `0x006250d0`.
pub const G_INPUT_POLL_FLAG: u32 = 0x00cc_9cf4;

/// The top bit of a `PArray` count: the elements are not the array's.
pub const NOT_OWNED: u32 = 0x8000_0000;

/// A `PClassDescriptorForType<T>`-style destructor copy: stores `vtable`,
/// then tail-jumps to `parent` (thiscall, no stack words).
#[derive(Debug, Clone, Copy)]
pub struct TailDtor {
    pub vtable: u32,
    pub parent: u32,
}

/// Every tail destructor of the list: `(function, copy)`.
pub const TAIL_DTORS: &[(u32, TailDtor)] = &[
    (
        0x004d_b900,
        TailDtor {
            vtable: 0x00b2_0b84,
            parent: DESTRUCT,
        },
    ),
    (
        0x004d_b910,
        TailDtor {
            vtable: 0x00b2_0d94,
            parent: DESTRUCT,
        },
    ),
    (
        0x004d_b920,
        TailDtor {
            vtable: 0x00b2_0c8c,
            parent: DESTRUCT,
        },
    ),
    (
        0x004d_b930,
        TailDtor {
            vtable: 0x00b2_017c,
            parent: DESTRUCT,
        },
    ),
    (
        0x004d_b940,
        TailDtor {
            vtable: 0x00b2_026c,
            parent: DESTRUCT,
        },
    ),
    (
        0x004d_b950,
        TailDtor {
            vtable: 0x00b2_0094,
            parent: DESTRUCT,
        },
    ),
    (
        0x0062_0a40,
        TailDtor {
            vtable: 0x00b3_fdac,
            parent: DESTRUCT,
        },
    ),
    (
        0x0062_0a50,
        TailDtor {
            vtable: 0x00b3_fe7c,
            parent: DESTRUCT,
        },
    ),
    (
        0x0062_0a60,
        TailDtor {
            vtable: 0x00b3_fcc4,
            parent: DESTRUCT,
        },
    ),
    (
        0x009f_0660,
        TailDtor {
            vtable: STREAM_WRITER_VTABLE,
            parent: STREAM_DTOR,
        },
    ),
    (
        0x00a3_3180,
        TailDtor {
            vtable: 0x00b7_a204,
            parent: DESTRUCT,
        },
    ),
    (
        0x00a3_3190,
        TailDtor {
            vtable: 0x00b7_a00c,
            parent: DESTRUCT,
        },
    ),
];

/// A scalar deleting destructor copy: stores `vtable` (when set), runs
/// `inner` (the class's destructor, thiscall, when set), then `free(this)`
/// (cdecl) when `flags & 1`; `eax` = `this`.
#[derive(Debug, Clone, Copy)]
pub struct Deleting {
    pub vtable: Option<u32>,
    pub inner: Option<u32>,
    pub free: u32,
}

/// Every scalar deleting destructor of the list: `(function, copy)`.
pub const DELETES: &[(u32, Deleting)] = &[
    (
        0x0057_dc80,
        Deleting {
            vtable: Some(STREAM_READER_VTABLE),
            inner: Some(STREAM_DTOR),
            free: PHYRE_FREE,
        },
    ),
    (
        0x0062_1a00,
        Deleting {
            vtable: None,
            inner: Some(APPLICATION_DTOR),
            free: PHYRE_FREE,
        },
    ),
    (
        0x0062_1ad0,
        Deleting {
            vtable: None,
            inner: Some(SCRIPT_BUFFER_DTOR),
            free: OPERATOR_DELETE,
        },
    ),
    (
        0x0062_1bd0,
        Deleting {
            vtable: Some(INPUT_DEVICE_VTABLE),
            inner: None,
            free: OPERATOR_DELETE,
        },
    ),
    (
        0x0062_1c80,
        Deleting {
            vtable: Some(INPUT_DEVICE_VTABLE),
            inner: None,
            free: PHYRE_FREE,
        },
    ),
    (
        0x009e_3de0,
        Deleting {
            vtable: Some(INPUT_SOURCE_VTABLE),
            inner: None,
            free: PHYRE_FREE,
        },
    ),
    (
        0x009f_0670,
        Deleting {
            vtable: Some(STREAM_WRITER_VTABLE),
            inner: Some(STREAM_DTOR),
            free: PHYRE_FREE,
        },
    ),
];

/// A virtual method that returns a constant: `(function, eax, stack words
/// popped)`. `ecx` is never read.
pub const CONSTANTS: &[(u32, u32, u32)] = &[
    (0x0060_ede0, 0, 1),
    (0x0060_ee10, 0, 4),
    (0x0062_5d00, 0x17, 2),
    (0x0062_5ed0, 0x17, 2),
    (0x0062_8200, 4, 0),
    (0x0062_8220, 2, 0),
    (0x0062_8230, 3, 0),
    (0x0062_9810, 0, 2),
];

/// The two `PArray<T>` release copies (`0x00620900`, `0x00620930`).
pub const RELEASES: [u32; 2] = [0x0062_0900, 0x0062_0930];

/// The copy of `f` in `table` (a const lookup for the shims).
///
/// # Panics
/// At compile time when `f` is not in the table.
#[must_use]
pub const fn tail_of(f: u32) -> TailDtor {
    let mut i = 0;
    while i < TAIL_DTORS.len() {
        if TAIL_DTORS[i].0 == f {
            return TAIL_DTORS[i].1;
        }
        i += 1;
    }
    panic!("not a tail destructor of the list");
}

/// The copy of `f` in [`DELETES`].
///
/// # Panics
/// At compile time when `f` is not in the table.
#[must_use]
pub const fn delete_of(f: u32) -> Deleting {
    let mut i = 0;
    while i < DELETES.len() {
        if DELETES[i].0 == f {
            return DELETES[i].1;
        }
        i += 1;
    }
    panic!("not a deleting destructor of the list");
}

/// The constant `f` of [`CONSTANTS`] returns.
///
/// # Panics
/// At compile time when `f` is not in the table.
#[must_use]
pub const fn constant_of(f: u32) -> u32 {
    let mut i = 0;
    while i < CONSTANTS.len() {
        if CONSTANTS[i].0 == f {
            return CONSTANTS[i].1;
        }
        i += 1;
    }
    panic!("not a constant method of the list");
}

/// `PUtilityGameplay`'s count rule (`0x0054b300`): a spline of `n` points
/// (`count & 0x7fffffff`) has no segments below two points, else `n - 1`
/// open or `n` closed.
#[must_use]
pub const fn segment_count(count: u32, closed: bool) -> u32 {
    let n = count & !NOT_OWNED;
    if n < 2 {
        0
    } else if closed {
        n
    } else {
        n - 1
    }
}

/// What `PInputDevicePad::getButton(id)` leaves in `eax`: the button byte
/// for ids `0xb..=0x1a` (`byte`), else 0, in `al`; the rest of `eax` is the
/// id's upper bytes (the original loads the id into `eax` first).
#[must_use]
pub const fn button_eax(id: u32, byte: u8) -> u32 {
    (id & 0xffff_ff00) | byte as u32
}

/// Whether `getButton` reads a button byte for `id` (`id - 0xb <= 0xf`,
/// unsigned).
#[must_use]
pub const fn is_button(id: u32) -> bool {
    id.wrapping_sub(0xb) <= 0xf
}

/// An `fld dword` / `fstp dword` copy of a float: the bits, with a
/// signalling NaN quieted.
#[must_use]
pub fn float_copy(bits: u32) -> u32 {
    x87::store(x87::load(bits))
}

pub mod bridge {
    use super::{
        AXIS_MAP_GETTER, CHANGE_DISPLAY_SETTINGS_SLOT, Conv, Deleting, ECHO_FORMAT, FILE_SEEK,
        G_HWND, G_INPUT_POLL_FLAG, Host, MOTION_BLUR_CHECK, Mem, NOT_OWNED, PHYRE_FREE, PRINTF,
        RELEASE_DC_SLOT, SLEEP_SLOT, SPLINE_EVALUATE, STATIC_UNLINK, STREAM_CTOR,
        TIME_END_PERIOD_SLOT, TailDtor, button_eax, float_copy, is_button, segment_count,
    };

    /// The function an import slot holds.
    fn import(h: &impl Mem, slot: u32) -> u32 {
        h.read(h.rebase(slot))
    }

    /// A tail destructor copy: `*this = vtable`, then `parent(this)`; its
    /// `eax`.
    pub fn tail_dtor(h: &mut impl Host, d: TailDtor, this: u32) -> u32 {
        let vt = h.rebase(d.vtable);
        h.write(this, vt);
        h.call_sym(d.parent, Conv::Thiscall(this), &[])
    }

    /// `*this = vtable` (`0x0060ec70`, `0x00620ed0`: a base class's
    /// destructor body; `eax` stays the caller's). Vtables are image
    /// addresses (relocated).
    pub fn set_vtable(m: &mut impl Mem, this: u32, vtable: u32) {
        let vt = m.rebase(vtable);
        m.write(this, vt);
    }

    /// A scalar deleting destructor copy (`ret 4`); `eax` = `this`.
    pub fn deleting_dtor(h: &mut impl Host, d: Deleting, this: u32, flags: u32) -> u32 {
        if let Some(vt) = d.vtable {
            let vt = h.rebase(vt);
            h.write(this, vt);
        }
        if let Some(inner) = d.inner {
            h.call_sym(inner, Conv::Thiscall(this), &[]);
        }
        if flags & 1 != 0 {
            h.call_sym(d.free, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `PArray<T>::release` (`0x00620900`, `0x00620930`): `PhyreFree` of
    /// the elements when the array owns them (count's top bit clear), then
    /// both words zero. `eax`: `PhyreFree`'s, or `0xfe` (the original's
    /// `not al` of the top bit) for a borrowed buffer.
    pub fn release(h: &mut impl Host, this: u32) -> u32 {
        let count: u32 = h.read(this);
        let eax = if count & NOT_OWNED == 0 {
            let data: u32 = h.read(this + 4);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data])
        } else {
            0xfe
        };
        h.write(this, 0u32);
        h.write(this + 4, 0u32);
        eax
    }

    /// `PhyreOS::Sleep(ms)` (`0x00430030`): `Sleep(ms)`; its `eax`.
    pub fn sleep(h: &mut impl Host, ms: u32) -> u32 {
        let f = import(h, SLEEP_SLOT);
        h.call(f, Conv::Stdcall, &[ms])
    }

    /// `PTimerWin32::Terminate` (`0x00430040`): `timeEndPeriod(5)`; its
    /// `eax`.
    pub fn timer_terminate(h: &mut impl Host) -> u32 {
        let f = import(h, TIME_END_PERIOD_SLOT);
        h.call(f, Conv::Stdcall, &[5])
    }

    /// `0x004632f0` (thiscall `(src)`, `ret 4`): the three floats at `src`
    /// to `this + 0x20..0x2c` (`fld`/`fstp`); `eax` = `this`.
    pub fn set_vec3_20(m: &mut impl Mem, this: u32, src: u32) -> u32 {
        for i in 0..3 {
            let v: u32 = m.read(src + 4 * i);
            m.write(this + 0x20 + 4 * i, float_copy(v));
        }
        this
    }

    /// `0x004dbab0` (thiscall, no stack words): the static-object unlink
    /// of `this` (cdecl); its `eax`.
    pub fn static_unlink(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(STATIC_UNLINK, Conv::Cdecl, &[this])
    }

    /// `0x0054b300` (thiscall): the spline's segment count
    /// ([`segment_count`] of `+4`, closed when the byte at `+0x18` is set).
    pub fn spline_segments(m: &impl Mem, this: u32) -> u32 {
        let count: u32 = m.read(this + 4);
        let closed: u8 = m.read(this + 0x18);
        segment_count(count, closed != 0)
    }

    /// `0x0054b520` (thiscall `(t, out)`, `ret 8`): the spline's point at
    /// `t` ([`SPLINE_EVALUATE`] into the 16-byte local at `frame`), its
    /// three floats to `out + 0x30..0x3c`. `t` is passed on through
    /// `fld`/`fstp` (a signalling NaN quieted). `eax`: the evaluation's
    /// (the local's address in the original).
    pub fn spline_point_to_30(h: &mut impl Host, this: u32, t: u32, out: u32, frame: u32) -> u32 {
        let p = h.call_sym(
            SPLINE_EVALUATE,
            Conv::Thiscall(this),
            &[frame, float_copy(t)],
        );
        for i in 0..3 {
            let v: u32 = h.read(p + 4 * i);
            h.write(out + 0x30 + 4 * i, float_copy(v));
        }
        p
    }

    /// `PStreamReaderFile`'s seek (`0x00576a80`, thiscall `(offset)`):
    /// the file's seek on `this + 0x14` (tail jump); its `eax`.
    pub fn reader_file_seek(h: &mut impl Host, this: u32, offset: u32) -> u32 {
        h.call_sym(FILE_SEEK, Conv::Thiscall(this + 0x14), &[offset])
    }

    /// `0x0057dcb0` (cdecl `(head)`): frees the singly linked blocks of
    /// `*head` (each's first word the next), leaving `*head` 0. `Some`:
    /// the last `PhyreFree`'s `eax`; `None` for an empty list, where the
    /// original leaves the caller's `eax` (its two direct callers read
    /// neither).
    pub fn free_chain(h: &mut impl Host, head: u32) -> Option<u32> {
        let mut eax = None;
        loop {
            let node: u32 = h.read(head);
            if node == 0 {
                return eax;
            }
            let next: u32 = h.read(node);
            h.write(head, next);
            eax = Some(h.call_sym(PHYRE_FREE, Conv::Cdecl, &[node]));
        }
    }

    /// `0x0057e160` (thiscall `(n)`, `ret 4`): 0 for `n == 0`, else the
    /// object's virtual `+0x18` (`(n)`) added to `+0xc`; its `eax`.
    pub fn read_counted(h: &mut impl Host, this: u32, n: u32) -> u32 {
        if n == 0 {
            return 0;
        }
        let got = h.call_virtual(this, 6, &[n]);
        let pos: u32 = h.read(this + 0xc);
        h.write(this + 0xc, pos.wrapping_add(got));
        got
    }

    /// `PApplicationScriptProxy::Echo` (`0x00623310`, cdecl `(state,
    /// text)`): `PhyrePrintf(2, ECHO_FORMAT, text)`; 0.
    pub fn echo(h: &mut impl Host, text: u32) -> u32 {
        h.call_sym(PRINTF, Conv::Cdecl, &[2, h.rebase(ECHO_FORMAT), text]);
        0
    }

    /// `0x006250d0` (cdecl `(flag)`): the byte to [`G_INPUT_POLL_FLAG`].
    pub fn set_poll_flag(m: &mut impl Mem, flag: u8) {
        let at = m.rebase(G_INPUT_POLL_FLAG);
        m.write(at, flag);
    }

    /// `PWindowWin32Base::restoreDisplay` (`0x006270a0`, thiscall): the
    /// desktop's display mode back (`ChangeDisplaySettingsA(0, 0)`) when
    /// the window's settings (`[this]`) say full screen (`+0x2c`), then
    /// `ReleaseDC(g_hWnd, [this + 4])`; its `eax`.
    pub fn restore_display(h: &mut impl Host, this: u32) -> u32 {
        let settings: u32 = h.read(this);
        let full: u8 = h.read(settings + 0x2c);
        if full != 0 {
            let f = import(h, CHANGE_DISPLAY_SETTINGS_SLOT);
            h.call(f, Conv::Stdcall, &[0, 0]);
        }
        let dc: u32 = h.read(this + 4);
        let wnd: u32 = h.read(h.rebase(G_HWND));
        let f = import(h, RELEASE_DC_SLOT);
        h.call(f, Conv::Stdcall, &[wnd, dc])
    }

    /// `0x006277b0` (thiscall): the first `'\n'` from the cursor `+0xc`
    /// up to the end `+0x10`, or 0.
    pub fn find_newline(m: &impl Mem, this: u32) -> u32 {
        let mut p: u32 = m.read(this + 0xc);
        let end: u32 = m.read(this + 0x10);
        while p < end {
            let c: u8 = m.read(p);
            if c == b'\n' {
                return p;
            }
            p += 1;
        }
        0
    }

    /// `PInputDevicePad::getButton(id)` (`0x006289f0`): see
    /// [`button_eax`].
    pub fn pad_button(m: &impl Mem, this: u32, id: u32) -> u32 {
        let byte = if is_button(id) {
            m.read::<u8>(this.wrapping_add(id).wrapping_add(0x38d))
        } else {
            0
        };
        button_eax(id, byte)
    }

    /// `0x00629750` (thiscall `(i)`): word `i` of the table at `+0x2c0`.
    pub fn viewport_at(m: &impl Mem, this: u32, i: u32) -> u32 {
        let table: u32 = m.read(this + 0x2c0);
        m.read(table.wrapping_add(i.wrapping_mul(4)))
    }

    /// `PApplication::getWindowDisplayInfo` (`0x006297a0`): `+0x264`.
    pub fn window_display_info(m: &impl Mem, this: u32) -> u32 {
        m.read(this + 0x264)
    }

    /// `PInputDevicePad::setVibration(low, high)` (`0x0062cb70`): the two
    /// bytes to `+0x58`, `+0x59`, and `+0x5a` = 1 (changed).
    pub fn set_vibration(m: &mut impl Mem, this: u32, low: u8, high: u8) {
        m.write(this + 0x58, low);
        m.write(this + 0x59, high);
        m.write(this + 0x5a, 1u8);
    }

    /// `PInputSourceJoypadAxis` slot 1 (`0x009f0330`): `+0x18` = the map's
    /// getter on `+0x1c` (0 without one). `Some(eax)` when the getter ran;
    /// without a map the original leaves the caller's `eax`.
    pub fn axis_refresh(h: &mut impl Host, this: u32) -> Option<u32> {
        let map: u32 = h.read(this + 0x1c);
        if map == 0 {
            h.write(this + 0x18, 0u32);
            return None;
        }
        let v = h.call_sym(AXIS_MAP_GETTER, Conv::Thiscall(map), &[]);
        h.write(this + 0x18, v);
        Some(v)
    }

    /// `PStreamReader::PStreamReader(name)` (`0x009f0630`): the stream's
    /// constructor, then the writer vtable and `+0xc` = 0; `eax` = `this`.
    pub fn stream_reader_ctor(h: &mut impl Host, this: u32, name: u32) -> u32 {
        h.call_sym(STREAM_CTOR, Conv::Thiscall(this), &[name]);
        let vt = h.rebase(super::STREAM_WRITER_VTABLE);
        h.write(this, vt);
        h.write(this + 0xc, 0u32);
        this
    }

    /// `0x00a16500` (thiscall `(i)`): element `i` of the `PArray` at
    /// `this` plus 4, or 0 when out of range or null.
    pub fn element_plus4(m: &impl Mem, this: u32, i: u32) -> u32 {
        let count: u32 = m.read(this);
        if i >= count & !NOT_OWNED {
            return 0;
        }
        let data: u32 = m.read(this + 4);
        let e: u32 = m.read(data.wrapping_add(i.wrapping_mul(4)));
        if e == 0 { 0 } else { e.wrapping_add(4) }
    }

    /// `0x00a16530` (thiscall `(i)`): the word at `+4` of element `i`, or
    /// 0 when out of range or null.
    pub fn element_word4(m: &impl Mem, this: u32, i: u32) -> u32 {
        let count: u32 = m.read(this);
        if i >= count & !NOT_OWNED {
            return 0;
        }
        let data: u32 = m.read(this + 4);
        let e: u32 = m.read(data.wrapping_add(i.wrapping_mul(4)));
        if e == 0 { 0 } else { m.read(e.wrapping_add(4)) }
    }

    /// `PMotionBlur` slot 3 (`0x00a1b980`, thiscall `(a, b)`, `ret 8`): 0
    /// unless enabled (`+0x10`); else the check's `eax` (on `this`).
    pub fn motion_blur_check(h: &mut impl Host, this: u32, a: u32, b: u32) -> u32 {
        let on: u8 = h.read(this + 0x10);
        if on == 0 {
            return 0;
        }
        h.call_sym(MOTION_BLUR_CHECK, Conv::Thiscall(this), &[a, b])
    }
}
