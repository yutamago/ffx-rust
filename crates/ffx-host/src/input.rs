//! The game-facing side of the player's input devices (`docs/host.md`,
//! "Windows", "Input", "The platform seam"): the keyboard, the mouse and
//! game controllers through the platform's devices
//! ([`crate::platform::Platform`]'s input methods; on Windows the host's
//! own `DirectInput8`, 64-bit `dinput8.dll`, and `XInput` through
//! `xinput9_1_0.dll`, the game's import, else `xinput1_4.dll`).
//!
//! The game's `IDirectInput8A` and its devices stay the layer's COM objects
//! (`crate::gfx`, `class::DINPUT_DEVICE`); each device the game creates
//! gets one of the platform's behind it ([`create_device`]), and its
//! methods go to that device: the data format (the game's 32-bit
//! `DIDATAFORMAT` read from the arena, [`read_format`]; Windows rebuilds
//! it with host pointers), the cooperative level with the platform's
//! window (the game holds the real `HWND`), properties, `Acquire`, `Poll`,
//! `GetDeviceState`, `GetDeviceData`, `EnumObjects`. So focus works as in
//! `FFX.exe`: the game asks for a non-exclusive foreground keyboard and
//! mouse (`Input_AddKeyboard`, `Input_AddMouse`: level 6), which read
//! nothing while its window is in the background, and `Keyboard_Update`
//! acquires again when it comes back.
//!
//! `EnumDevices` adds the attached game controllers to the system mouse
//! and keyboard (`crate::stand_ins::dinput_enum_devices`), and
//! `Input_IsXInputDevice` (`0x006244f0`, a WMI query in the game, whose
//! `CoCreateInstance` the host refuses) is answered from the platform's
//! `XInput` pads (on Windows the raw input device names: an `IG_` name with
//! the controller's vendor and product id), so an `XInput` pad becomes the
//! game's `XInput` device as in `FFX.exe` (`Input_AddPadDevice`).
//!
//! When the real devices are used ([`real`]): when the platform has them
//! (Windows; not headless), unless `FFX_HOST_INPUT=0`, and not in a
//! harness game in background mode (`FFX_HARNESS_BACKGROUND=1`), where
//! `ffx-hook`'s harness drops the real devices in `FFX.exe` too (so typing
//! in other windows never reaches a test). The keys a harness test holds
//! are added to the keyboard's state in every case, and in the harness a
//! failing keyboard reads as all keys up plus the test's keys instead of
//! failing ([`keyboard`]), as the hook's `get_device_state` does.

use ffx_bridge::Mem as _;

use crate::exec::Exec;
use crate::platform::{self, DeviceCall, DeviceInfo, Property};
use crate::table::{Args, Conv, Entry, Port, Ret, Table};

/// `E_NOTIMPL`.
const E_NOTIMPL: u32 = 0x8000_4001;
/// `ERROR_DEVICE_NOT_CONNECTED`.
pub const NOT_CONNECTED: u32 = 0x48f;
/// `DI8DEVCLASS_GAMECTRL`.
pub const CLASS_GAME_CONTROLLERS: u32 = 4;
/// `sizeof(DIDEVICEINSTANCEA)` (no pointers: the same in both builds).
pub const DEVICE_INSTANCE: usize = 0x244;
/// `sizeof(DIDEVICEOBJECTINSTANCEA)` (no pointers either).
pub const OBJECT_INSTANCE: usize = 0x13c;
/// `Input_IsXInputDevice` (cdecl(&guidProduct) -> al).
const IS_XINPUT: u32 = 0x0062_44f0;

/// Whether the real devices reach the game (the module's doc).
#[must_use]
pub fn real() -> bool {
    static REAL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *REAL.get_or_init(|| {
        let env = |n: &str| std::env::var(n).unwrap_or_default();
        decide(
            platform::get().input_devices(),
            &env("FFX_HOST_INPUT"),
            std::env::var_os("FFX_HARNESS").is_some(),
            crate::harness::background(),
        )
    })
}

/// [`real`]'s rule: the platform has devices (Windows), not switched
/// off, not a background harness game.
#[must_use]
pub fn decide(devices: bool, switch: &str, harness: bool, background: bool) -> bool {
    devices && switch != "0" && !(harness && background)
}

/// Whether a harness game runs (its keys are added, a failure reads as
/// keys up).
fn harness() -> bool {
    std::env::var_os("FFX_HARNESS").is_some()
}

/// What the keyboard's `GetDeviceState` answers: `real` is the host
/// device's state or its failure (`None`: no real device), `held` the
/// harness test's keys (`crate::harness::keyboard_state`, 0x80 per held
/// key). Returns the `HRESULT` and the state to write (`None`: write
/// nothing, the failure stands so the game acquires again, as outside
/// the harness in `FFX.exe`).
#[must_use]
pub fn keyboard(
    real: Option<Result<&[u8], i32>>,
    held: &[u8],
    harness: bool,
) -> (i32, Option<Vec<u8>>) {
    let mut state = match real {
        Some(Ok(s)) => s.to_vec(),
        Some(Err(hr)) if !harness => return (hr, None),
        _ => vec![0; held.len()],
    };
    state.resize(held.len(), 0);
    for (s, &h) in state.iter_mut().zip(held) {
        if h & 0x80 != 0 {
            *s = 0x80;
        }
    }
    (0, Some(state))
}

/// The game's `DIDATAFORMAT` (32-bit: size, object size, flags, data
/// size, object count, objects; each object 16 bytes: `pguid`, offset,
/// type, flags).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Format {
    pub flags: u32,
    pub data_size: u32,
    pub objects: Vec<Object>,
}

/// One `DIOBJECTDATAFORMAT`; `guid` read from the arena (`None` for a
/// null `pguid`: any object of the type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Object {
    pub guid: Option<u128>,
    pub ofs: u32,
    pub kind: u32,
    pub flags: u32,
}

/// Reads the game's data format at `at` (`None` when its sizes are not
/// the 32-bit ones or it has more than 1,024 objects).
#[must_use]
pub fn read_format(mem: &impl ffx_bridge::Mem, at: u32) -> Option<Format> {
    if at == 0 {
        return None;
    }
    let w = |i: u32| mem.read::<u32>(at + 4 * i);
    let (size, obj_size, flags, data_size, n, list) = (w(0), w(1), w(2), w(3), w(4), w(5));
    if size != 24 || obj_size != 16 || n > 1024 || (n != 0 && list == 0) {
        return None;
    }
    let objects = (0..n)
        .map(|i| {
            let o = list + 16 * i;
            let pguid = mem.read::<u32>(o);
            Object {
                guid: (pguid != 0).then(|| crate::com::read_guid(mem, pguid)),
                ofs: mem.read(o + 4),
                kind: mem.read(o + 8),
                flags: mem.read(o + 12),
            }
        })
        .collect();
    Some(Format {
        flags,
        data_size,
        objects,
    })
}

/// One `DIDEVICEOBJECTDATA` as the game's build lays it out (offset,
/// data, time stamp, sequence, `uAppData` as a 32-bit word): the first
/// `stride` bytes of it (20, or 16 for the `DirectX` 3 form).
#[must_use]
pub fn object_data_bytes(item: [u32; 5], stride: usize) -> Vec<u8> {
    let mut b: Vec<u8> = item.iter().flat_map(|w| w.to_le_bytes()).collect();
    b.resize(stride, 0);
    b
}

fn a(args: &Args, i: usize) -> u32 {
    args.stack.get(i).copied().unwrap_or(0)
}

#[allow(clippy::cast_sign_loss)] // an HRESULT's bits
fn hr(v: i32) -> u32 {
    v as u32
}

/// `Acquire`, `Unacquire` or `Poll` by the method's name.
fn call_of(name: &str) -> DeviceCall {
    match name {
        "Acquire" => DeviceCall::Acquire,
        "Unacquire" => DeviceCall::Unacquire,
        _ => DeviceCall::Poll,
    }
}

/// An `IDirectInputDevice8A` method of the game's device `a[0]` on its
/// real device (in the layer's call, `crate::gfx`; nothing here calls back
/// into the game). `None`: the device has no real one, the layer answers.
pub fn device_call(e: &mut Exec, name: &str, a: &[u32]) -> Option<u32> {
    let dev = a[0];
    let host = platform::get();
    if !host.input_has(dev) {
        return None;
    }
    let mut mem = e.mem();
    let r = match name {
        "SetDataFormat" => {
            let f = read_format(&mem, a[1])?;
            host.input_set_format(dev, &f)?
        }
        "SetCooperativeLevel" => host.input_set_cooperative_level(dev, a[1], a[2])?,
        "SetProperty" | "GetProperty" => {
            if a[2] == 0 {
                return Some(0x8007_0057);
            }
            let size = mem.read::<u32>(a[2]).clamp(16, 256) as usize;
            let mut data = vec![0u8; size];
            mem.read_bytes(a[2], &mut data);
            // MAKEDIPROP(n): a small number, not a GUID's address
            let prop = if a[1] < 0x1_0000 {
                Property::Id(a[1])
            } else {
                Property::Guid(crate::com::read_guid(&mem, a[1]))
            };
            let set = name == "SetProperty";
            let r = host.input_property(dev, &prop, &mut data, set)?;
            if !set && r >= 0 {
                mem.write_bytes(a[2], &data);
            }
            r
        }
        "Acquire" | "Unacquire" | "Poll" => host.input_call(dev, call_of(name))?,
        "GetDeviceState" => {
            let size = a[1].min(4096) as usize;
            let state = host.input_state(dev, size)?;
            if size == 256 {
                let mut held = [0u8; 256];
                crate::harness::keyboard_state(&mut held);
                let (r, out) = keyboard(Some(state.as_deref().map_err(|&r| r)), &held, harness());
                if let (Some(out), true) = (out, a[2] != 0) {
                    mem.write_bytes(a[2], &out);
                }
                r
            } else {
                match state {
                    Ok(s) => {
                        if a[2] != 0 {
                            mem.write_bytes(a[2], &s);
                        }
                        0
                    }
                    Err(r) => r,
                }
            }
        }
        "GetDeviceData" => {
            // (size of one item, items, &count, flags)
            let (stride, out, count_at, flags) = (a[1] as usize, a[2], a[3], a[4]);
            if count_at == 0 || !(16..=64).contains(&stride) {
                return Some(0x8007_0057);
            }
            let max = mem.read::<u32>(count_at);
            match host.input_data(dev, flags, if out == 0 { max } else { max.min(4096) })? {
                Ok((items, r)) => {
                    if out != 0 {
                        for (i, item) in (0u32..).zip(&items) {
                            #[allow(clippy::cast_possible_truncation)] // at most 64
                            let at = out + i * stride as u32;
                            mem.write_bytes(at, &object_data_bytes(*item, stride));
                        }
                    }
                    mem.write(count_at, u32::try_from(items.len()).unwrap_or(0));
                    r
                }
                Err(r) => {
                    mem.write(count_at, 0u32);
                    r
                }
            }
        }
        "GetCapabilities" | "GetDeviceInfo" => {
            if a[1] == 0 {
                return Some(0x8007_0057);
            }
            let size = mem.read::<u32>(a[1]).min(0x244) as usize;
            let mut data = vec![0u8; size.max(4)];
            mem.read_bytes(a[1], &mut data);
            let what = if name == "GetCapabilities" {
                DeviceInfo::Capabilities
            } else {
                DeviceInfo::Instance
            };
            let r = host.input_info(dev, what, &mut data)?;
            if r >= 0 {
                mem.write_bytes(a[1], &data);
            }
            r
        }
        _ => return None,
    };
    Some(hr(r))
}

/// The layer's `CreateDevice` made `handle` for `guid`: its real device
/// when the real devices are used. `Err` (the `HRESULT`) for a game
/// controller the host cannot open (the system keyboard and mouse fall
/// back to the layer's stand-ins).
///
/// # Errors
/// The host's `HRESULT` for a controller it cannot open.
pub fn create_device(handle: u32, guid: u128) -> Result<(), u32> {
    if !real() {
        return Ok(());
    }
    match platform::get().input_open(handle, guid) {
        Ok(name) => {
            crate::log::note(&format!(
                "input: {name} {} on the host's DirectInput",
                crate::com::iid_text(guid)
            ));
            Ok(())
        }
        Err(r) => {
            crate::log::note(&format!(
                "input: no host device for {}: {:#x}",
                crate::com::iid_text(guid),
                hr(r)
            ));
            if system_device(guid) {
                Ok(())
            } else {
                Err(hr(r))
            }
        }
    }
}

/// `GUID_SysMouse`, `GUID_SysKeyboard`.
fn system_device(guid: u128) -> bool {
    guid == 0x6f1d_2b60_d5a0_11cf_bfc7_4445_5354_0000
        || guid == 0x6f1d_2b61_d5a0_11cf_bfc7_4445_5354_0000
}

/// The layer released the game's device `handle`.
pub fn release(handle: u32) {
    platform::get().input_release(handle);
}

/// `IDirectInput8A::GetDeviceStatus(guid)` on the host (`None`: the real
/// devices are not used).
#[must_use]
pub fn device_status(guid: u128) -> Option<u32> {
    real()
        .then(|| platform::get().input_status(guid))
        .flatten()
        .map(hr)
}

/// The attached game controllers' `DIDEVICEINSTANCEA`s for the game's
/// `EnumDevices(class, .., flags)` (none when the real devices are not
/// used, or for a class that is not all (0) or game controllers).
#[must_use]
pub fn controllers(class: u32, flags: u32) -> Vec<Vec<u8>> {
    if !real() || (class != 0 && class != CLASS_GAME_CONTROLLERS) {
        return Vec::new();
    }
    platform::get().input_controllers(flags)
}

/// `IDirectInputDevice8A::EnumObjects(this, callback, ref, flags)`: the
/// real device's objects, each handed to the game's callback (stdcall
/// (instance, ref), on while it answers `DIENUM_CONTINUE`), outside the
/// layer's lock. A device without a real one: `E_NOTIMPL`, as before.
pub fn enum_objects(e: &mut Exec, args: &Args) -> Ret {
    let (dev, callback, user, flags) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    let Some(objects) = platform::get().input_objects(dev, flags) else {
        return Ret::eax(E_NOTIMPL);
    };
    if callback == 0 {
        return Ret::eax(0x8007_0057);
    }
    #[allow(clippy::cast_possible_truncation)] // 0x13c
    let inst = e.alloc(OBJECT_INSTANCE as u32);
    if inst == 0 {
        return Ret::eax(0x8007_000e);
    }
    for o in objects {
        e.mem().write_bytes(inst, &o);
        match e.call_stdcall(callback, &[inst, user]) {
            Ok(1) => {}
            _ => break,
        }
    }
    e.release(inst);
    Ret::eax(0)
}

/// `XInputGetState(user, &state)`: the host's pad (`XINPUT_STATE`, 16
/// bytes in both builds); no controller when the real devices are not
/// used.
pub fn xinput_get_state(e: &mut Exec, args: &Args) -> Ret {
    if !real() {
        return Ret::eax(NOT_CONNECTED);
    }
    match platform::get().xinput_state(a(args, 0)) {
        Ok(state) => {
            if a(args, 1) != 0 {
                e.mem().write_bytes(a(args, 1), &state);
            }
            Ret::eax(0)
        }
        Err(r) => Ret::eax(r),
    }
}

/// `XInputSetState(user, &vibration)` (4 bytes): the host's pad.
pub fn xinput_set_state(e: &mut Exec, args: &Args) -> Ret {
    if !real() || a(args, 1) == 0 {
        return Ret::eax(NOT_CONNECTED);
    }
    let mut v = [0u8; 4];
    e.mem().read_bytes(a(args, 1), &mut v);
    Ret::eax(platform::get().xinput_set_state(a(args, 0), v))
}

/// `Input_IsXInputDevice(&guidProduct)` (cdecl -> al): with the real
/// devices, whether the host has an `XInput` device of that vendor and
/// product (`Data1`: vendor in the low word, product in the high one);
/// otherwise the game's port (its WMI query, which the host's
/// `CoCreateInstance` refuses: "not an `XInput` device").
fn is_xinput_device(e: &mut Exec, args: &Args) -> Ret {
    if !real() {
        return e
            .call_original(IS_XINPUT, Conv::Cdecl, args)
            .unwrap_or(Ret::eax(0));
    }
    let guid = a(args, 0);
    let data1 = if guid == 0 {
        0
    } else {
        e.mem().read::<u32>(guid)
    };
    let yes = data1 != 0
        && platform::get()
            .xinput_ids()
            .contains(&(data1 & 0xffff, data1 >> 16));
    Ret::eax(u32::from(yes))
}

/// Whether a raw input device name (`\\?\HID#VID_045E&PID_028E&IG_00#...`)
/// is an `XInput` device's, and its vendor and product ids.
#[must_use]
pub fn xinput_name_ids(name: &str) -> Option<(u32, u32)> {
    let up = name.to_ascii_uppercase();
    if !up.contains("IG_") {
        return None;
    }
    let hex = |key: &str| -> Option<u32> {
        let at = up.find(key)? + key.len();
        u32::from_str_radix(up.get(at..at + 4)?, 16).ok()
    };
    Some((hex("VID_")?, hex("PID_")?))
}

/// Registers the wrapper of `Input_IsXInputDevice` (after the `ported!`
/// entries: the port is its original).
pub fn register(t: &mut Table) {
    // push ebp; mov ebp, esp; sub esp, 0x88
    t.wrap(
        IS_XINPUT,
        Entry {
            name: "Input_IsXInputDevice",
            conv: Conv::Cdecl,
            words: 1,
            port: Port::Exec(is_xinput_device),
        },
        &[0x55, 0x8b, 0xec, 0x81, 0xec, 0x88, 0x00, 0x00, 0x00],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffx_bridge::VecMem;

    #[test]
    fn the_real_devices_are_used_off_the_background_harness_on_windows() {
        assert!(decide(true, "", false, false));
        assert!(decide(true, "1", true, false));
        assert!(!decide(true, "", true, true));
        assert!(!decide(true, "0", false, false));
        assert!(!decide(false, "", false, false));
        // background without a harness is not the harness's mode
        assert!(decide(true, "", false, true));
    }

    #[test]
    fn the_harness_keys_are_ored_into_the_real_keyboard() {
        let mut real = [0u8; 256];
        real[0x1c] = 0x80; // Enter, pressed by the player
        real[0x01] = 0x01; // a byte without bit 7 stays as it is
        let mut held = [0u8; 256];
        held[0x2c] = 0x80; // Z, held by the test
        let (hr, out) = keyboard(Some(Ok(&real)), &held, true);
        let out = out.unwrap();
        assert_eq!(hr, 0);
        assert_eq!(
            (out[0x1c], out[0x2c], out[0x01], out[0x02]),
            (0x80, 0x80, 0x01, 0)
        );
        // the same outside the harness (no keys held then)
        let (_, out) = keyboard(Some(Ok(&real)), &[0u8; 256], false);
        assert_eq!(out.unwrap(), real.to_vec());
    }

    #[test]
    fn a_failing_keyboard_fails_outside_the_harness_and_reads_the_test_keys_in_it() {
        const INPUT_LOST: i32 = i32::from_ne_bytes(0x8007_001e_u32.to_ne_bytes());
        let mut held = [0u8; 256];
        held[0x39] = 0x80;
        // outside: the failure stands, nothing written (the game acquires again)
        assert_eq!(
            keyboard(Some(Err(INPUT_LOST)), &held, false),
            (INPUT_LOST, None)
        );
        // in the harness: keys up plus the test's
        let (hr, out) = keyboard(Some(Err(INPUT_LOST)), &held, true);
        assert_eq!(hr, 0);
        assert_eq!(out.unwrap(), held.to_vec());
        // no real device (the stand-in): the test's keys, as before
        let (hr, out) = keyboard(None, &held, false);
        assert_eq!((hr, out.unwrap()), (0, held.to_vec()));
    }

    #[test]
    fn the_games_data_format_is_read_with_its_guids() {
        let mut m = VecMem::new(0x1000, 0x1000);
        // c_dfDIMouse2-like: 2 objects, the first with GUID_XAxis
        m.write(0x1000, [24u32, 16, 2, 0x14, 2, 0x1100]);
        m.write(0x1100, [0x1200u32, 0, 0x00ff_ff03, 0]);
        m.write(0x1110, [0u32, 0xc, 0x8000_ff0c, 0]);
        m.write_bytes(
            0x1200,
            &[
                0xe0, 0x23, 0x48, 0xa3, 0x9f, 0x8f, 0xcf, 0x11, 0xbf, 0xc7, 0x44, 0x45, 0x53, 0x54,
                0, 0,
            ],
        );
        let f = read_format(&m, 0x1000).unwrap();
        assert_eq!((f.flags, f.data_size), (2, 0x14));
        assert_eq!(
            f.objects,
            vec![
                Object {
                    guid: Some(0xa348_23e0_8f9f_11cf_bfc7_4445_5354_0000),
                    ofs: 0,
                    kind: 0x00ff_ff03,
                    flags: 0
                },
                Object {
                    guid: None,
                    ofs: 0xc,
                    kind: 0x8000_ff0c,
                    flags: 0
                },
            ]
        );
        // not the 32-bit layout
        m.write(0x1000, 32u32);
        assert_eq!(read_format(&m, 0x1000), None);
        assert_eq!(read_format(&m, 0), None);
    }

    #[test]
    fn buffered_items_take_the_games_stride() {
        let b = object_data_bytes([0x1c, 0x80, 7, 9, 0x1234], 20);
        assert_eq!(b.len(), 20);
        assert_eq!(&b[16..], &0x1234u32.to_le_bytes());
        assert_eq!(object_data_bytes([1, 2, 3, 4, 5], 16).len(), 16);
    }

    #[test]
    fn xinput_devices_are_named_with_ig() {
        assert_eq!(
            xinput_name_ids(
                r"\\?\HID#VID_045E&PID_028E&IG_00#3&1b2c&0&0000#{4d1e55b2-f16f-11cf-88cb-001111000030}"
            ),
            Some((0x045e, 0x028e))
        );
        assert_eq!(
            xinput_name_ids(r"\\?\hid#vid_054c&pid_0ce6&mi_03#8&2c1&0&0000#{...}"),
            None
        );
        assert_eq!(xinput_name_ids(r"\\?\HID#IG_00#x"), None);
    }
}
