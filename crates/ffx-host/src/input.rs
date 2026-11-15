//! The player's real input devices (`docs/host.md`, "Windows", "Input"):
//! the keyboard, the mouse and game controllers through the host's own
//! `DirectInput8` (64-bit `dinput8.dll`), and `XInput` through the host's
//! `xinput9_1_0.dll` (the game's import; else `xinput1_4.dll`).
//!
//! The game's `IDirectInput8A` and its devices stay the layer's COM objects
//! (`crate::gfx`, `class::DINPUT_DEVICE`); each device the game creates
//! gets a real host device behind it ([`create_device`]), and its methods
//! go to that device: the data format (the game's 32-bit `DIDATAFORMAT`
//! rebuilt with host pointers, [`read_format`]), the cooperative level
//! with the real window (the Windows platform's: the game holds the real `HWND`),
//! properties, `Acquire`, `Poll`, `GetDeviceState`, `GetDeviceData`,
//! `EnumObjects`. So focus works as in `FFX.exe`: the game asks for a
//! non-exclusive foreground keyboard and mouse (`Input_AddKeyboard`,
//! `Input_AddMouse`: level 6), which read nothing while its window is in
//! the background, and `Keyboard_Update` acquires again when it comes
//! back.
//!
//! `EnumDevices` adds the attached game controllers to the system mouse
//! and keyboard (`crate::stand_ins::dinput_enum_devices`), and
//! `Input_IsXInputDevice` (`0x006244f0`, a WMI query in the game, whose
//! `CoCreateInstance` the host refuses) is answered from the host's raw
//! input device names (an `IG_` name with the controller's vendor and
//! product id), so an `XInput` pad becomes the game's `XInput` device as
//! in `FFX.exe` (`Input_AddPadDevice`).
//!
//! When the real devices are used ([`real`]): on Windows, unless
//! `FFX_HOST_INPUT=0`, and not in a harness game in background mode
//! (`FFX_HARNESS_BACKGROUND=1`), where `ffx-hook`'s harness drops the real
//! devices in `FFX.exe` too (so typing in other windows never reaches a
//! test). The keys a harness test holds are added to the keyboard's
//! state in every case, and in the harness a failing keyboard reads as
//! all keys up plus the test's keys instead of failing ([`keyboard`]),
//! as the hook's `get_device_state` does.

use ffx_bridge::Mem as _;

use crate::exec::Exec;
use crate::table::{Args, Conv, Entry, Port, Ret, Table};

/// `E_NOTIMPL`.
const E_NOTIMPL: u32 = 0x8000_4001;
/// `ERROR_DEVICE_NOT_CONNECTED`.
const NOT_CONNECTED: u32 = 0x48f;
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
            cfg!(windows),
            &env("FFX_HOST_INPUT"),
            std::env::var_os("FFX_HARNESS").is_some(),
            crate::harness::background(),
        )
    })
}

/// [`real`]'s rule: on Windows, not switched off, not a background
/// harness game.
#[must_use]
pub fn decide(windows: bool, switch: &str, harness: bool, background: bool) -> bool {
    windows && switch != "0" && !(harness && background)
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

/// An `IDirectInputDevice8A` method of the game's device `a[0]` on its
/// real device (in the layer's call, `crate::gfx`; nothing here calls back
/// into the game). `None`: the device has no real one, the layer answers.
pub fn device_call(e: &mut Exec, name: &str, a: &[u32]) -> Option<u32> {
    let dev = a[0];
    if !host::has(dev) {
        return None;
    }
    let mut mem = e.mem();
    let r = match name {
        "SetDataFormat" => {
            let f = read_format(&mem, a[1])?;
            host::set_data_format(dev, &f)?
        }
        "SetCooperativeLevel" => host::set_cooperative_level(dev, a[1], a[2])?,
        "SetProperty" | "GetProperty" => {
            if a[2] == 0 {
                return Some(0x8007_0057);
            }
            let size = mem.read::<u32>(a[2]).clamp(16, 256) as usize;
            let mut data = vec![0u8; size];
            mem.read_bytes(a[2], &mut data);
            // MAKEDIPROP(n): a small number, not a GUID's address
            let prop = if a[1] < 0x1_0000 {
                host::Prop::Id(a[1])
            } else {
                host::Prop::Guid(crate::com::read_guid(&mem, a[1]))
            };
            let set = name == "SetProperty";
            let r = host::property(dev, &prop, &mut data, set)?;
            if !set && r >= 0 {
                mem.write_bytes(a[2], &data);
            }
            r
        }
        "Acquire" | "Unacquire" | "Poll" => host::simple(dev, name)?,
        "GetDeviceState" => {
            let size = a[1].min(4096) as usize;
            let state = host::device_state(dev, size)?;
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
            match host::device_data(dev, flags, if out == 0 { max } else { max.min(4096) })? {
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
            let r = host::info(dev, name, &mut data)?;
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
    match host::create_device(handle, guid) {
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
    host::release(handle);
}

/// `IDirectInput8A::GetDeviceStatus(guid)` on the host (`None`: the real
/// devices are not used).
#[must_use]
pub fn device_status(guid: u128) -> Option<u32> {
    real().then(|| host::device_status(guid)).flatten().map(hr)
}

/// The attached game controllers' `DIDEVICEINSTANCEA`s for the game's
/// `EnumDevices(class, .., flags)` (none when the real devices are not
/// used, or for a class that is not all (0) or game controllers).
#[must_use]
pub fn controllers(class: u32, flags: u32) -> Vec<Vec<u8>> {
    if !real() || (class != 0 && class != CLASS_GAME_CONTROLLERS) {
        return Vec::new();
    }
    host::controllers(flags)
}

/// `IDirectInputDevice8A::EnumObjects(this, callback, ref, flags)`: the
/// real device's objects, each handed to the game's callback (stdcall
/// (instance, ref), on while it answers `DIENUM_CONTINUE`), outside the
/// layer's lock. A device without a real one: `E_NOTIMPL`, as before.
pub fn enum_objects(e: &mut Exec, args: &Args) -> Ret {
    let (dev, callback, user, flags) = (a(args, 0), a(args, 1), a(args, 2), a(args, 3));
    let Some(objects) = host::objects(dev, flags) else {
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
    match host::xinput_get_state(a(args, 0)) {
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
    Ret::eax(host::xinput_set_state(a(args, 0), v))
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
    let yes = data1 != 0 && host::xinput_ids().contains(&(data1 & 0xffff, data1 >> 16));
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

#[cfg(windows)]
mod host {
    //! The host's `DirectInput8`, `XInput` and raw input device list.

    use std::collections::HashMap;
    use std::sync::Mutex;

    use windows::Win32::Devices::HumanInterfaceDevice as di;
    use windows::Win32::Foundation::{HINSTANCE, HWND};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::core::{GUID, Interface as _};

    use super::{DEVICE_INSTANCE, Format, OBJECT_INSTANCE};

    /// A property: `MAKEDIPROP(n)` or a GUID.
    pub enum Prop {
        Id(u32),
        Guid(u128),
    }

    struct Host {
        input: di::IDirectInput8A,
        devices: HashMap<u32, di::IDirectInputDevice8A>,
    }

    // SAFETY: DirectInput's objects may be called from any thread; the
    // game's input threads reach them only through this lock.
    unsafe impl Send for Host {}

    /// The host's `IDirectInput8A` (made on first use; `Err` once it failed).
    static HOST: Mutex<Option<Result<Host, ()>>> = Mutex::new(None);

    fn with<T>(f: impl FnOnce(&mut Host) -> T) -> Option<T> {
        let mut g = HOST.lock().ok()?;
        if g.is_none() {
            *g = Some(open().map_err(|e| {
                crate::log::note(&format!("input: no host DirectInput: {e}"));
            }));
        }
        g.as_mut()?.as_mut().ok().map(f)
    }

    fn open() -> windows::core::Result<Host> {
        // SAFETY: plain calls; `out` receives the interface pointer.
        unsafe {
            let module = GetModuleHandleW(None)?;
            let mut out = std::ptr::null_mut();
            di::DirectInput8Create(
                HINSTANCE(module.0),
                0x0800,
                &di::IDirectInput8A::IID,
                &raw mut out,
                None,
            )?;
            Ok(Host {
                input: di::IDirectInput8A::from_raw(out),
                devices: HashMap::new(),
            })
        }
    }

    fn device<T>(dev: u32, f: impl FnOnce(&di::IDirectInputDevice8A) -> T) -> Option<T> {
        with(|h| h.devices.get(&dev).map(f)).flatten()
    }

    fn code(r: windows::core::Result<()>) -> i32 {
        r.map_or_else(|e| e.code().0, |()| 0)
    }

    pub fn has(dev: u32) -> bool {
        HOST.lock()
            .is_ok_and(|g| matches!(&*g, Some(Ok(h)) if h.devices.contains_key(&dev)))
    }

    pub fn create_device(handle: u32, guid: u128) -> Result<String, i32> {
        with(|h| {
            let mut dev = None;
            // SAFETY: a GUID on the stack, the out slot.
            unsafe {
                h.input
                    .CreateDevice(&GUID::from_u128(guid), &raw mut dev, None)
                    .map_err(|e| e.code().0)?;
            }
            let dev = dev.ok_or(-1)?;
            let mut info = di::DIDEVICEINSTANCEA {
                dwSize: u32::try_from(std::mem::size_of::<di::DIDEVICEINSTANCEA>()).unwrap_or(0),
                ..Default::default()
            };
            // SAFETY: a sized struct.
            let name = if unsafe { dev.GetDeviceInfo(&raw mut info) }.is_ok() {
                let raw = info.tszProductName.map(i8::cast_unsigned);
                let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
                String::from_utf8_lossy(&raw[..end]).into_owned()
            } else {
                "device".to_owned()
            };
            h.devices.insert(handle, dev);
            Ok(name)
        })
        .unwrap_or(Err(-1))
    }

    pub fn release(handle: u32) {
        let _ = with(|h| h.devices.remove(&handle));
    }

    pub fn device_status(guid: u128) -> Option<i32> {
        // SAFETY: a GUID on the stack.
        with(|h| code(unsafe { h.input.GetDeviceStatus(&GUID::from_u128(guid)) }))
    }

    unsafe extern "system" fn collect_device(
        inst: *mut di::DIDEVICEINSTANCEA,
        user: *mut core::ffi::c_void,
    ) -> windows::core::BOOL {
        // SAFETY: DirectInput hands a live instance; `user` is the Vec
        // `controllers` passed.
        unsafe {
            let out = &mut *user.cast::<Vec<Vec<u8>>>();
            let bytes = std::slice::from_raw_parts(inst.cast::<u8>(), DEVICE_INSTANCE);
            out.push(bytes.to_vec());
        }
        windows::core::BOOL(1)
    }

    pub fn controllers(flags: u32) -> Vec<Vec<u8>> {
        let mut out: Vec<Vec<u8>> = Vec::new();
        let _ = with(|h| {
            // SAFETY: the callback writes into `out` during the call only.
            unsafe {
                h.input.EnumDevices(
                    super::CLASS_GAME_CONTROLLERS,
                    Some(collect_device),
                    (&raw mut out).cast(),
                    flags,
                )
            }
        });
        out
    }

    unsafe extern "system" fn collect_object(
        inst: *mut di::DIDEVICEOBJECTINSTANCEA,
        user: *mut core::ffi::c_void,
    ) -> windows::core::BOOL {
        // SAFETY: as `collect_device`.
        unsafe {
            let out = &mut *user.cast::<Vec<Vec<u8>>>();
            let bytes = std::slice::from_raw_parts(inst.cast::<u8>(), OBJECT_INSTANCE);
            out.push(bytes.to_vec());
        }
        windows::core::BOOL(1)
    }

    pub fn objects(dev: u32, flags: u32) -> Option<Vec<Vec<u8>>> {
        device(dev, |d| {
            let mut out: Vec<Vec<u8>> = Vec::new();
            // SAFETY: the callback writes into `out` during the call only.
            let _ = unsafe { d.EnumObjects(Some(collect_object), (&raw mut out).cast(), flags) };
            out
        })
    }

    pub fn set_data_format(dev: u32, f: &Format) -> Option<i32> {
        let guids: Vec<Option<GUID>> = f
            .objects
            .iter()
            .map(|o| o.guid.map(GUID::from_u128))
            .collect();
        let mut objects: Vec<di::DIOBJECTDATAFORMAT> = f
            .objects
            .iter()
            .zip(&guids)
            .map(|(o, g)| di::DIOBJECTDATAFORMAT {
                pguid: g.as_ref().map_or(std::ptr::null(), std::ptr::from_ref),
                dwOfs: o.ofs,
                dwType: o.kind,
                dwFlags: o.flags,
            })
            .collect();
        let mut format = di::DIDATAFORMAT {
            dwSize: u32::try_from(std::mem::size_of::<di::DIDATAFORMAT>()).unwrap_or(0),
            dwObjSize: u32::try_from(std::mem::size_of::<di::DIOBJECTDATAFORMAT>()).unwrap_or(0),
            dwFlags: f.flags,
            dwDataSize: f.data_size,
            dwNumObjs: u32::try_from(objects.len()).unwrap_or(0),
            rgodf: objects.as_mut_ptr(),
        };
        // SAFETY: the format and its objects live through the call
        // (DirectInput copies them).
        device(dev, |d| code(unsafe { d.SetDataFormat(&raw mut format) }))
    }

    pub fn set_cooperative_level(dev: u32, window: u32, flags: u32) -> Option<i32> {
        let window = HWND(crate::platform::windows::hwnd(window));
        // SAFETY: the real window the game created (the platform's).
        device(dev, |d| {
            code(unsafe { d.SetCooperativeLevel(window, flags) })
        })
    }

    pub fn property(dev: u32, prop: &Prop, data: &mut [u8], set: bool) -> Option<i32> {
        let guid;
        let p: *const GUID = match *prop {
            // MAKEDIPROP(n): DirectInput's property numbers, never read
            // through
            Prop::Id(n) => n as usize as *const GUID,
            Prop::Guid(g) => {
                guid = GUID::from_u128(g);
                &raw const guid
            }
        };
        // the property's words, aligned
        let mut words: Vec<u32> = data
            .chunks(4)
            .map(|c| {
                let mut w = [0u8; 4];
                w[..c.len()].copy_from_slice(c);
                u32::from_le_bytes(w)
            })
            .collect();
        let header = words.as_mut_ptr().cast::<di::DIPROPHEADER>();
        let r = device(dev, |d| {
            // SAFETY: `words` holds the game's whole property (its dwSize).
            code(unsafe {
                if set {
                    d.SetProperty(p, header)
                } else {
                    d.GetProperty(p, header)
                }
            })
        });
        let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        data.copy_from_slice(&bytes[..data.len()]);
        r
    }

    pub fn simple(dev: u32, name: &str) -> Option<i32> {
        device(dev, |d| {
            // SAFETY: plain calls.
            code(unsafe {
                match name {
                    "Acquire" => d.Acquire(),
                    "Unacquire" => d.Unacquire(),
                    _ => d.Poll(),
                }
            })
        })
    }

    pub fn device_state(dev: u32, size: usize) -> Option<Result<Vec<u8>, i32>> {
        device(dev, |d| {
            let mut buf = vec![0u8; size];
            let n = u32::try_from(size).unwrap_or(0);
            // SAFETY: a buffer of `size` bytes.
            unsafe { d.GetDeviceState(n, buf.as_mut_ptr().cast()) }
                .map(|()| buf)
                .map_err(|e| e.code().0)
        })
    }

    #[allow(clippy::type_complexity)] // items and the HRESULT
    pub fn device_data(
        dev: u32,
        flags: u32,
        max: u32,
    ) -> Option<Result<(Vec<[u32; 5]>, i32), i32>> {
        device(dev, |d| {
            let mut items = vec![di::DIDEVICEOBJECTDATA::default(); max as usize];
            let mut n = max;
            let size = u32::try_from(std::mem::size_of::<di::DIDEVICEOBJECTDATA>()).unwrap_or(0);
            let ptr = if max == 0 {
                std::ptr::null_mut()
            } else {
                items.as_mut_ptr()
            };
            // SAFETY: `max` items.
            unsafe { d.GetDeviceData(size, ptr, &raw mut n, flags) }
                .map(|()| {
                    items.truncate(n as usize);
                    #[allow(clippy::cast_possible_truncation)] // the game's uAppData is 32-bit
                    let words = items
                        .iter()
                        .map(|i| {
                            [
                                i.dwOfs,
                                i.dwData,
                                i.dwTimeStamp,
                                i.dwSequence,
                                i.uAppData as u32,
                            ]
                        })
                        .collect();
                    (words, 0)
                })
                .map_err(|e| e.code().0)
        })
    }

    pub fn info(dev: u32, name: &str, data: &mut [u8]) -> Option<i32> {
        device(dev, |d| {
            // SAFETY: `data` holds the game's struct (its dwSize).
            code(unsafe {
                if name == "GetCapabilities" {
                    d.GetCapabilities(data.as_mut_ptr().cast())
                } else {
                    d.GetDeviceInfo(data.as_mut_ptr().cast())
                }
            })
        })
    }

    type GetState = unsafe extern "system" fn(u32, *mut u8) -> u32;
    type SetState = unsafe extern "system" fn(u32, *mut u8) -> u32;

    /// The host's `XInputGetState` and `XInputSetState`.
    fn xinput() -> Option<(GetState, SetState)> {
        use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
        static XINPUT: std::sync::OnceLock<Option<(usize, usize)>> = std::sync::OnceLock::new();
        let (g, s) = (*XINPUT.get_or_init(|| {
            for dll in [c"xinput9_1_0.dll", c"xinput1_4.dll"] {
                // SAFETY: NUL-terminated names.
                unsafe {
                    let m = LoadLibraryA(dll.as_ptr().cast());
                    if m.is_null() {
                        continue;
                    }
                    let g = GetProcAddress(m, c"XInputGetState".as_ptr().cast());
                    let s = GetProcAddress(m, c"XInputSetState".as_ptr().cast());
                    if let (Some(g), Some(s)) = (g, s) {
                        return Some((g as usize, s as usize));
                    }
                }
            }
            None
        }))?;
        // SAFETY: the exports' own signatures.
        unsafe {
            Some((
                std::mem::transmute::<usize, GetState>(g),
                std::mem::transmute::<usize, SetState>(s),
            ))
        }
    }

    pub fn xinput_get_state(user: u32) -> Result<[u8; 16], u32> {
        let (get, _) = xinput().ok_or(super::NOT_CONNECTED)?;
        let mut state = [0u8; 16];
        // SAFETY: XINPUT_STATE is 16 bytes.
        match unsafe { get(user, state.as_mut_ptr()) } {
            0 => Ok(state),
            r => Err(r),
        }
    }

    pub fn xinput_set_state(user: u32, mut v: [u8; 4]) -> u32 {
        match xinput() {
            // SAFETY: XINPUT_VIBRATION is 4 bytes.
            Some((_, set)) => unsafe { set(user, v.as_mut_ptr()) },
            None => super::NOT_CONNECTED,
        }
    }

    /// The vendor and product ids of the host's `XInput` devices (raw
    /// input HID names with `IG_`).
    pub fn xinput_ids() -> Vec<(u32, u32)> {
        use windows_sys::Win32::UI::Input::{
            GetRawInputDeviceInfoW, GetRawInputDeviceList, RAWINPUTDEVICELIST, RIDI_DEVICENAME,
            RIM_TYPEHID,
        };
        let size = u32::try_from(std::mem::size_of::<RAWINPUTDEVICELIST>()).unwrap_or(0);
        let mut n = 0u32;
        // SAFETY: the count query, then a list of `n`.
        unsafe {
            if GetRawInputDeviceList(std::ptr::null_mut(), &raw mut n, size) != 0 {
                return Vec::new();
            }
            let mut list = vec![std::mem::zeroed::<RAWINPUTDEVICELIST>(); n as usize];
            let got = GetRawInputDeviceList(list.as_mut_ptr(), &raw mut n, size);
            if got == u32::MAX {
                return Vec::new();
            }
            list.truncate(got as usize);
            list.iter()
                .filter(|d| d.dwType == RIM_TYPEHID)
                .filter_map(|d| {
                    let mut len = 0u32;
                    GetRawInputDeviceInfoW(
                        d.hDevice,
                        RIDI_DEVICENAME,
                        std::ptr::null_mut(),
                        &raw mut len,
                    );
                    let mut name = vec![0u16; len as usize + 1];
                    let r = GetRawInputDeviceInfoW(
                        d.hDevice,
                        RIDI_DEVICENAME,
                        name.as_mut_ptr().cast(),
                        &raw mut len,
                    );
                    if r == u32::MAX {
                        return None;
                    }
                    let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
                    super::xinput_name_ids(&String::from_utf16_lossy(&name[..end]))
                })
                .collect()
        }
    }
}

#[cfg(not(windows))]
mod host {
    //! No host devices off Windows ([`super::real`] is false there).

    use super::Format;

    #[allow(dead_code)] // the Windows side's shape
    pub enum Prop {
        Id(u32),
        Guid(u128),
    }

    pub fn has(_: u32) -> bool {
        false
    }
    pub fn create_device(_: u32, _: u128) -> Result<String, i32> {
        Err(-1)
    }
    pub fn release(_: u32) {}
    pub fn device_status(_: u128) -> Option<i32> {
        None
    }
    pub fn controllers(_: u32) -> Vec<Vec<u8>> {
        Vec::new()
    }
    pub fn objects(_: u32, _: u32) -> Option<Vec<Vec<u8>>> {
        None
    }
    pub fn set_data_format(_: u32, _: &Format) -> Option<i32> {
        None
    }
    pub fn set_cooperative_level(_: u32, _: u32, _: u32) -> Option<i32> {
        None
    }
    pub fn property(_: u32, _: &Prop, _: &mut [u8], _: bool) -> Option<i32> {
        None
    }
    pub fn simple(_: u32, _: &str) -> Option<i32> {
        None
    }
    pub fn device_state(_: u32, _: usize) -> Option<Result<Vec<u8>, i32>> {
        None
    }
    #[allow(clippy::type_complexity)]
    pub fn device_data(_: u32, _: u32, _: u32) -> Option<Result<(Vec<[u32; 5]>, i32), i32>> {
        None
    }
    pub fn info(_: u32, _: &str, _: &mut [u8]) -> Option<i32> {
        None
    }
    pub fn xinput_get_state(_: u32) -> Result<[u8; 16], u32> {
        Err(super::NOT_CONNECTED)
    }
    pub fn xinput_set_state(_: u32, _: [u8; 4]) -> u32 {
        super::NOT_CONNECTED
    }
    pub fn xinput_ids() -> Vec<(u32, u32)> {
        Vec::new()
    }
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

    /// A GUID's bytes as the game's memory holds them.
    #[cfg(windows)]
    fn guid_bytes(g: u128) -> [u8; 16] {
        let mut b = [0u8; 16];
        #[allow(clippy::cast_possible_truncation)] // the GUID's fields
        {
            b[..4].copy_from_slice(&((g >> 96) as u32).to_le_bytes());
            b[4..6].copy_from_slice(&((g >> 80) as u16).to_le_bytes());
            b[6..8].copy_from_slice(&((g >> 64) as u16).to_le_bytes());
            b[8..].copy_from_slice(&(g as u64).to_be_bytes());
        }
        b
    }

    /// Windows only (run the test binary on Windows): the host's
    /// keyboard and mouse open, take the game's 32-bit data formats
    /// rebuilt with host pointers, a property by `MAKEDIPROP` number,
    /// acquire at the default level (background, non-exclusive: no
    /// window here) and read; `XInput` loads.
    #[cfg(windows)]
    #[test]
    fn the_hosts_devices_read_through_the_games_formats() {
        const KEY: u128 = 0x5572_8220_d33c_11cf_bfc7_4445_5354_0000;
        const X_AXIS: u128 = 0xa36d_02e0_c9f3_11cf_bfc7_4445_5354_0000;
        const Y_AXIS: u128 = 0xa36d_02e1_c9f3_11cf_bfc7_4445_5354_0000;
        const Z_AXIS: u128 = 0xa36d_02e2_c9f3_11cf_bfc7_4445_5354_0000;
        const SYS_KEYBOARD: u128 = 0x6f1d_2b61_d5a0_11cf_bfc7_4445_5354_0000;
        const SYS_MOUSE: u128 = 0x6f1d_2b60_d5a0_11cf_bfc7_4445_5354_0000;
        let mut m = VecMem::new(0x1000, 0x3000);
        // c_dfDIKeyboard: 256 optional buttons with GUID_Key
        m.write_bytes(0x1080, &guid_bytes(KEY));
        m.write(0x1000, [24u32, 16, 2, 256, 256, 0x1100]);
        for i in 0..256u32 {
            m.write(0x1100 + 16 * i, [0x1080u32, i, 0x8000_000c | (i << 8), 0]);
        }
        // c_dfDIMouse2: X, Y, (Z), 8 buttons
        m.write_bytes(0x3000, &guid_bytes(X_AXIS));
        m.write_bytes(0x3010, &guid_bytes(Y_AXIS));
        m.write_bytes(0x3020, &guid_bytes(Z_AXIS));
        m.write(0x3100, [24u32, 16, 2, 20, 11, 0x3200]);
        m.write(0x3200, [0x3000u32, 0, 0x00ff_ff03, 0]);
        m.write(0x3210, [0x3010u32, 4, 0x00ff_ff03, 0]);
        m.write(0x3220, [0x3020u32, 8, 0x80ff_ff03, 0]);
        for i in 0..8u32 {
            let kind = if i < 2 { 0x00ff_ff0c } else { 0x80ff_ff0c };
            m.write(0x3230 + 16 * i, [0u32, 12 + i, kind, 0]);
        }
        let keyboard = read_format(&m, 0x1000).unwrap();
        let mouse = read_format(&m, 0x3100).unwrap();
        assert!(host::create_device(0x7001, SYS_KEYBOARD).is_ok());
        assert!(host::create_device(0x7002, SYS_MOUSE).is_ok());
        assert!(host::has(0x7001) && host::has(0x7002) && !host::has(0x7003));
        assert_eq!(host::set_data_format(0x7001, &keyboard), Some(0));
        assert_eq!(host::set_data_format(0x7002, &mouse), Some(0));
        // DIPROP_BUFFERSIZE (MAKEDIPROP(1)): DIPROPDWORD, 16 items
        let mut prop: Vec<u8> = [20u32, 16, 0, 0, 16]
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        assert_eq!(
            host::property(0x7001, &host::Prop::Id(1), &mut prop, true),
            Some(0)
        );
        prop[16..].fill(0);
        assert_eq!(
            host::property(0x7001, &host::Prop::Id(1), &mut prop, false),
            Some(0)
        );
        assert_eq!(&prop[16..], &16u32.to_le_bytes());
        assert_eq!(host::simple(0x7001, "Acquire"), Some(0));
        assert_eq!(host::simple(0x7002, "Acquire"), Some(0));
        let state = host::device_state(0x7001, 256).unwrap().unwrap();
        assert_eq!(state.len(), 256);
        let state = host::device_state(0x7002, 20).unwrap().unwrap();
        assert_eq!(state.len(), 20);
        assert!(matches!(host::device_data(0x7001, 0, 16), Some(Ok(_))));
        assert_eq!(host::objects(0x7002, 0).map(|o| o.is_empty()), Some(false));
        host::release(0x7001);
        host::release(0x7002);
        assert!(!host::has(0x7001));
        match host::xinput_get_state(0) {
            Ok(_) | Err(NOT_CONNECTED) => {}
            Err(r) => panic!("XInputGetState: {r:#x}"),
        }
        let _ = host::xinput_ids();
    }
}
