//! Input-device and application-script teardown (list
//! `phyre-framework-input`, batch 4): the devices' scalar deleting
//! destructors (`DirectInput` devices released through their COM vtables),
//! the script buffer's and script's destructors, the application's
//! `PArray` releases, the suspend/resume notifications, the async-command
//! drain, and the input log's array resize and token scanner.
//!
//! COM calls (`IDirectInputDevice8::Unacquire` at vtable `+0x20`,
//! `Release` at `+8`) are stdcall with the interface as the first stack
//! word, as the originals push it.

use ffx_bridge::Mem;
use ffx_graphic::host::{Conv, Host};

/// `Phyre::PhyreFree(p)` (cdecl).
pub const PHYRE_FREE: u32 = 0x0042_fb20;
/// `Phyre::PhyreMalloc(size, align)` (cdecl).
pub const PHYRE_MALLOC_ALIGNED: u32 = 0x0042_fb80;
/// `operator delete(p)` (cdecl).
pub const OPERATOR_DELETE: u32 = 0x0063_0390;
/// `PCriticalSectionBase::release` (thiscall).
pub const CS_RELEASE: u32 = 0x0043_1950;
/// `eh vector destructor iterator(array, size, count, dtor)` (stdcall).
pub const EH_VECTOR_DTOR: u32 = 0x0094_96d2;
/// The 0x30-byte slot destructor the devices hand it.
pub const SLOT_DTOR: u32 = 0x0062_1190;
/// `PApplicationScriptBuffer`'s destructor (thiscall; in the list).
pub const SCRIPT_BUFFER_DTOR: u32 = 0x0062_0d80;
/// `PApplication`'s 0xe4-byte array release (thiscall; in the list).
pub const ARRAY_E4_RELEASE: u32 = 0x0062_0990;
/// The notifier tree's holder (`+0` the head).
pub const NOTIFIERS: u32 = 0x00cc_9ee4;
/// `Phyre_PhyreOS_Sleep(ms)` (cdecl; in the list).
pub const SLEEP: u32 = 0x0043_0030;
/// The async command queue's drain step (cdecl `(queue, &done)`).
pub const DRAIN_STEP: u32 = 0x005c_d0c0;

pub const DEVICE_VTABLE: u32 = 0x00b3_fbf8;
pub const KEYBOARD_VTABLE: u32 = 0x00b3_fc30;
pub const MOUSE_VTABLE: u32 = 0x00b3_fc14;
pub const TOUCH_VTABLE: u32 = 0x00b3_fc4c;
pub const PAD_VTABLE: u32 = 0x00b3_fca4;
pub const PAD_DIRECT_INPUT_VTABLE: u32 = 0x00b3_ff80;
pub const SCRIPT_BUFFER_VTABLE: u32 = 0x00b3_fc8c;
pub const SCRIPT_VTABLE: u32 = 0x00b3_fc94;

/// Which device a deleting destructor tears down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    /// `PInputDevicePad` and `PInputDevicePadXInput`: the four pad slots.
    Pad,
    /// `PInputDevicePadDirectInput`: the `DirectInput` device, then the pad.
    PadDirectInput,
    /// `PInputDeviceTouch`: the critical section at `+0x17c`.
    Touch,
    /// `PInputDeviceKeyboard`: the `DirectInput` device at `+0x14`.
    Keyboard,
    /// `PInputDeviceMouse`: the wrapper at `+0x14` (device at its `+4`)
    /// and five button slots at `+0x18`.
    Mouse,
}

/// Every device deleting destructor of the list.
pub const DEVICE_DELETES: &[(u32, Device)] = &[
    (0x0062_1c00, Device::Keyboard),
    (0x0062_1cb0, Device::Mouse),
    (0x0062_1d50, Device::Pad),
    (0x0062_1dd0, Device::PadDirectInput),
    (0x0062_1e70, Device::Pad),
    (0x0062_1ef0, Device::Touch),
];

/// The script deleting destructors (`PApplicationScript`,
/// `PApplicationScriptProxy`: one body).
pub const SCRIPT_DELETES: [u32; 2] = [0x0062_1a30, 0x0062_1b00];

/// The device of `f` in [`DEVICE_DELETES`].
///
/// # Panics
/// At compile time when `f` is not in the table.
#[must_use]
pub const fn device_of(f: u32) -> Device {
    let mut i = 0;
    while i < DEVICE_DELETES.len() {
        if DEVICE_DELETES[i].0 == f {
            return DEVICE_DELETES[i].1;
        }
        i += 1;
    }
    panic!("not a device deleting destructor");
}

/// What the original's `PArray` release leaves in `eax` when it frees
/// nothing: `0xfe` (the `not al` of a set top bit) for a borrowed buffer,
/// else the null data word.
#[must_use]
pub const fn kept_eax(count: u32) -> u32 {
    if count & 0x8000_0000 != 0 { 0xfe } else { 0 }
}

pub mod bridge {
    use super::{
        ARRAY_E4_RELEASE, CS_RELEASE, Conv, DEVICE_VTABLE, DRAIN_STEP, Device, EH_VECTOR_DTOR,
        Host, KEYBOARD_VTABLE, MOUSE_VTABLE, Mem, NOTIFIERS, OPERATOR_DELETE,
        PAD_DIRECT_INPUT_VTABLE, PAD_VTABLE, PHYRE_FREE, PHYRE_MALLOC_ALIGNED, SCRIPT_BUFFER_DTOR,
        SCRIPT_BUFFER_VTABLE, SCRIPT_VTABLE, SLEEP, SLOT_DTOR, TOUCH_VTABLE, kept_eax,
    };

    fn set_vtable(h: &mut impl Mem, this: u32, vt: u32) {
        let vt = h.rebase(vt);
        h.write(this, vt);
    }

    /// `Unacquire` and `Release` of a `DirectInput` device (stdcall COM).
    fn release_device(h: &mut impl Host, dev: u32) {
        let vt: u32 = h.read(dev);
        let unacquire: u32 = h.read(vt + 0x20);
        h.call(unacquire, Conv::Stdcall, &[dev]);
        let vt: u32 = h.read(dev);
        let release: u32 = h.read(vt + 8);
        h.call(release, Conv::Stdcall, &[dev]);
    }

    /// `count` slots of 0x30 bytes at `at` destroyed.
    fn destroy_slots(h: &mut impl Host, at: u32, count: u32) {
        let dtor = h.rebase(SLOT_DTOR);
        h.call_sym(EH_VECTOR_DTOR, Conv::Stdcall, &[at, 0x30, count, dtor]);
    }

    /// A device's scalar deleting destructor (`ret 4`): the device's own
    /// teardown, `PInputDevice`'s vtable, `PhyreFree` when `flags & 1`;
    /// `this`.
    pub fn device_delete(h: &mut impl Host, device: Device, this: u32, flags: u32) -> u32 {
        match device {
            Device::Keyboard => {
                set_vtable(h, this, KEYBOARD_VTABLE);
                let dev: u32 = h.read(this + 0x14);
                if dev != 0 {
                    release_device(h, dev);
                }
            }
            Device::Mouse => {
                set_vtable(h, this, MOUSE_VTABLE);
                let wrapper: u32 = h.read(this + 0x14);
                if wrapper != 0 {
                    let dev: u32 = h.read(wrapper + 4);
                    let vt: u32 = h.read(dev);
                    let unacquire: u32 = h.read(vt + 0x20);
                    h.call(unacquire, Conv::Stdcall, &[dev]);
                    let dev: u32 = h.read(wrapper + 4);
                    let vt: u32 = h.read(dev);
                    let release: u32 = h.read(vt + 8);
                    h.call(release, Conv::Stdcall, &[dev]);
                    h.call_sym(PHYRE_FREE, Conv::Cdecl, &[wrapper]);
                }
                destroy_slots(h, this + 0x18, 5);
            }
            Device::Pad => {
                set_vtable(h, this, PAD_VTABLE);
                destroy_slots(h, this + 0x6c, 4);
            }
            Device::PadDirectInput => {
                set_vtable(h, this, PAD_DIRECT_INPUT_VTABLE);
                let dev: u32 = h.read(this + 0x14);
                if dev != 0 {
                    release_device(h, dev);
                }
                set_vtable(h, this, PAD_VTABLE);
                destroy_slots(h, this + 0x6c, 4);
            }
            Device::Touch => {
                set_vtable(h, this, TOUCH_VTABLE);
                h.call_sym(CS_RELEASE, Conv::Thiscall(this + 0x17c), &[]);
            }
        }
        set_vtable(h, this, DEVICE_VTABLE);
        if flags & 1 != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// A `PArray` release that skips null data: `PhyreFree(data)` when the
    /// array owns a buffer, then both words zero; the original's `eax`.
    fn release_checked(h: &mut impl Host, count_at: u32, data_at: u32) -> u32 {
        let count: u32 = h.read(count_at);
        let data: u32 = h.read(data_at);
        let eax = if count & 0x8000_0000 == 0 && data != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data])
        } else {
            kept_eax(count)
        };
        h.write(data_at, 0u32);
        h.write(count_at, 0u32);
        eax
    }

    /// A `PArray` release that frees owned data even when null.
    fn release_owned(h: &mut impl Host, count_at: u32, data_at: u32) -> u32 {
        let count: u32 = h.read(count_at);
        let eax = if count & 0x8000_0000 == 0 {
            let data: u32 = h.read(data_at);
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data])
        } else {
            0xfe
        };
        h.write(data_at, 0u32);
        h.write(count_at, 0u32);
        eax
    }

    /// `PApplicationScriptBuffer::~PApplicationScriptBuffer` (`0x00620d80`):
    /// its vtable; the buffer (count `+4`, data `+8`) freed and cleared
    /// when it has a count, then once more as a checked release; that
    /// release's `eax`.
    pub fn script_buffer_dtor(h: &mut impl Host, this: u32) -> u32 {
        set_vtable(h, this, SCRIPT_BUFFER_VTABLE);
        let count: u32 = h.read(this + 4);
        if count != 0 {
            let data: u32 = h.read(this + 8);
            if data != 0 && count & 0x8000_0000 == 0 {
                h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data]);
            }
            h.write(this + 8, 0u32);
            h.write(this + 4, 0u32);
        }
        release_checked(h, this + 4, this + 8)
    }

    /// `PApplicationScript::~PApplicationScript` (`0x00620d00`): its
    /// vtable, the command text `+0x14` freed, `+0x14..+0x20` and `+0x24`
    /// cleared, then the buffer's destructor; its `eax`.
    pub fn script_dtor(h: &mut impl Host, this: u32) -> u32 {
        set_vtable(h, this, SCRIPT_VTABLE);
        let text: u32 = h.read(this + 0x14);
        if text != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[text]);
        }
        for off in [0x14, 0x18, 0x1c, 0x24] {
            h.write(this + off, 0u32);
        }
        h.call_sym(SCRIPT_BUFFER_DTOR, Conv::Thiscall(this), &[])
    }

    /// The scripts' scalar deleting destructor (`0x00621a30`,
    /// `0x00621b00`): [`script_dtor`]'s body (the buffer's destructor
    /// called), `operator delete` when `flags & 1`; `this`.
    pub fn script_delete(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        set_vtable(h, this, SCRIPT_VTABLE);
        let text: u32 = h.read(this + 0x14);
        if text != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[text]);
        }
        for off in [0x14, 0x18, 0x1c, 0x24] {
            h.write(this + off, 0u32);
        }
        h.call_sym(SCRIPT_BUFFER_DTOR, Conv::Thiscall(this), &[]);
        if flags & 1 != 0 {
            h.call_sym(OPERATOR_DELETE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// The 0xe4-byte element array release (`0x00620990`): for an owned,
    /// non-null buffer each element's virtual destructor (slot 0, flags 0)
    /// and `PhyreFree`; both words zero. `eax`: `PhyreFree`'s, 0 for no
    /// buffer, `0xfe` for a borrowed one.
    pub fn release_e4(h: &mut impl Host, this: u32) -> u32 {
        let count: u32 = h.read(this);
        let data: u32 = h.read(this + 4);
        let eax = if count & 0x8000_0000 == 0 && data != 0 {
            let mut e = data;
            for _ in 0..count {
                h.call_virtual(e, 0, &[0]);
                e = e.wrapping_add(0xe4);
            }
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[data])
        } else {
            kept_eax(count)
        };
        h.write(this + 4, 0u32);
        h.write(this, 0u32);
        eax
    }

    /// `0x006211b0` (thiscall): the element array at `+8`, then the array
    /// at `this` (checked release); that release's `eax`.
    pub fn release_pair(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(ARRAY_E4_RELEASE, Conv::Thiscall(this + 8), &[]);
        release_checked(h, this, this + 4)
    }

    /// `0x00621230` (thiscall): the arrays at `+0xa4` and `+0x9c` released
    /// (owned data freed even when null); the second's `eax`.
    pub fn release_a4_9c(h: &mut impl Host, this: u32) -> u32 {
        release_owned(h, this + 0xa4, this + 0xa8);
        release_owned(h, this + 0x9c, this + 0xa0)
    }

    /// `PApplication::NotifySuspend` / `NotifyResume` (`0x00627850`,
    /// `0x006278b0`): virtual `slot` (0 or 1) of every value (`+0x10`) of
    /// the notifier map, in order.
    pub fn notify_all(h: &mut impl Host, slot: u32) {
        let head: u32 = h.read(h.rebase(NOTIFIERS));
        let mut node: u32 = h.read(head);
        while node != head {
            let obj: u32 = h.read(node + 0x10);
            h.call_virtual(obj, slot, &[]);
            node = next_node(h, node);
        }
    }

    fn is_nil(m: &impl Mem, node: u32) -> bool {
        m.read::<u8>(node + 0xd) != 0
    }

    /// The in-order successor (`std::map` `++`), inlined in the notifiers.
    fn next_node(m: &impl Mem, node: u32) -> u32 {
        if is_nil(m, node) {
            return node;
        }
        let right: u32 = m.read(node + 8);
        if !is_nil(m, right) {
            let mut cur = right;
            loop {
                let left: u32 = m.read(cur);
                if is_nil(m, left) {
                    return cur;
                }
                cur = left;
            }
        }
        let mut cur = node;
        let mut p: u32 = m.read(node + 4);
        while !is_nil(m, p) && cur == m.read::<u32>(p + 8) {
            cur = p;
            p = m.read(p + 4);
        }
        p
    }

    /// `0x0062d100` (thiscall): the lock released, the queue (`+0x24`)
    /// drained into `+0x28` (a 1 ms sleep between tries while it is not
    /// done and `+0x24 != +0x28`), then `+0x18..+0x28` cleared; 0.
    pub fn drain_commands(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(CS_RELEASE, Conv::Thiscall(this), &[]);
        let done = this + 0x28;
        let queue: u32 = h.read(this + 0x24);
        if h.call_sym(DRAIN_STEP, Conv::Cdecl, &[queue, done]) == 0 {
            loop {
                let queue: u32 = h.read(this + 0x24);
                if queue == h.read::<u32>(done) {
                    break;
                }
                h.call_sym(SLEEP, Conv::Cdecl, &[1]);
                let queue: u32 = h.read(this + 0x24);
                if h.call_sym(DRAIN_STEP, Conv::Cdecl, &[queue, done]) != 0 {
                    break;
                }
            }
        }
        h.write(done, 0u32);
        for off in [0x24, 0x18, 0x1c, 0x20] {
            h.write(this + off, 0u32);
        }
        0
    }

    /// `PArray<0x90-byte record>::resize(n)` (`0x0062c1f0`, `ret 4`): 0
    /// when the count is already `n`; else a new buffer (`PhyreMalloc(n *
    /// 0x90, 4)`, none for 0; 0xd when that fails), the old one freed when
    /// owned, distinct and non-null, and the new count and buffer; 0.
    pub fn resize_90(h: &mut impl Host, this: u32, n: u32) -> u32 {
        let count: u32 = h.read(this);
        if n == count {
            return 0;
        }
        let mut data = 0;
        if n != 0 {
            data = h.call_sym(
                PHYRE_MALLOC_ALIGNED,
                Conv::Cdecl,
                &[n.wrapping_mul(0x90), 4],
            );
            if data == 0 {
                return 0xd;
            }
        }
        let old: u32 = h.read(this + 4);
        if old != data && count & 0x8000_0000 == 0 && old != 0 {
            h.call_sym(PHYRE_FREE, Conv::Cdecl, &[old]);
        }
        h.write(this + 4, data);
        h.write(this, n);
        0
    }

    /// The input log's token scanner (`0x00628e40`, cdecl `(p, end,
    /// delim)`): 0 when `p >= end`; skips to `delim` (or a NUL, or `end`);
    /// 0 unless it stopped on `delim` or a NUL; then the token after it
    /// runs to the next `delim`, NUL or `end`, which becomes a NUL; the
    /// token's start.
    pub fn next_token(m: &mut impl Mem, p: u32, end: u32, delim: u8) -> u32 {
        if p >= end {
            return 0;
        }
        let d = i32::from(delim as i8);
        let mut p = p;
        let mut c: u8 = m.read(p);
        while i32::from(c as i8) != d && c != 0 && p != end {
            p += 1;
            c = m.read(p);
        }
        let c: u8 = m.read(p);
        if c != delim && c != 0 {
            return 0;
        }
        let mut q = p + 1;
        loop {
            let c: u8 = m.read(q);
            if i32::from(c as i8) == d || c == 0 || q == end {
                break;
            }
            q += 1;
        }
        m.write(q, 0u8);
        p + 1
    }
}
