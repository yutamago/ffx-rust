//! The Windows platform's input: the host's `DirectInput8` (64-bit
//! `dinput8.dll`), `XInput` (`xinput9_1_0.dll`, else `xinput1_4.dll`) and
//! the raw input device list (docs/host.md, "Windows", "Input").

use std::collections::HashMap;
use std::sync::Mutex;

use windows::Win32::Devices::HumanInterfaceDevice as di;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::core::{GUID, Interface as _};

use crate::input::{CLASS_GAME_CONTROLLERS, DEVICE_INSTANCE, Format, NOT_CONNECTED, OBJECT_INSTANCE};
use crate::platform::{DeviceCall, DeviceInfo, Property};

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
                CLASS_GAME_CONTROLLERS,
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
    let window = HWND(super::hwnd(window));
    // SAFETY: the real window the game created (the platform's).
    device(dev, |d| {
        code(unsafe { d.SetCooperativeLevel(window, flags) })
    })
}

pub fn property(dev: u32, prop: &Property, data: &mut [u8], set: bool) -> Option<i32> {
    let guid;
    let p: *const GUID = match *prop {
        // MAKEDIPROP(n): DirectInput's property numbers, never read
        // through
        Property::Id(n) => n as usize as *const GUID,
        Property::Guid(g) => {
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

pub fn simple(dev: u32, call: DeviceCall) -> Option<i32> {
    device(dev, |d| {
        // SAFETY: plain calls.
        code(unsafe {
            match call {
                DeviceCall::Acquire => d.Acquire(),
                DeviceCall::Unacquire => d.Unacquire(),
                DeviceCall::Poll => d.Poll(),
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
pub fn device_data(dev: u32, flags: u32, max: u32) -> Option<Result<(Vec<[u32; 5]>, i32), i32>> {
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

pub fn info(dev: u32, what: DeviceInfo, data: &mut [u8]) -> Option<i32> {
    device(dev, |d| {
        // SAFETY: `data` holds the game's struct (its dwSize).
        code(unsafe {
            match what {
                DeviceInfo::Capabilities => d.GetCapabilities(data.as_mut_ptr().cast()),
                DeviceInfo::Instance => d.GetDeviceInfo(data.as_mut_ptr().cast()),
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
    let (get, _) = xinput().ok_or(NOT_CONNECTED)?;
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
        None => NOT_CONNECTED,
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
                crate::input::xinput_name_ids(&String::from_utf16_lossy(&name[..end]))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::read_format;
    use ffx_bridge::{Mem as _, VecMem};

    /// A GUID's bytes as the game's memory holds them.
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
        assert!(create_device(0x7001, SYS_KEYBOARD).is_ok());
        assert!(create_device(0x7002, SYS_MOUSE).is_ok());
        assert!(has(0x7001) && has(0x7002) && !has(0x7003));
        assert_eq!(set_data_format(0x7001, &keyboard), Some(0));
        assert_eq!(set_data_format(0x7002, &mouse), Some(0));
        // DIPROP_BUFFERSIZE (MAKEDIPROP(1)): DIPROPDWORD, 16 items
        let mut prop: Vec<u8> = [20u32, 16, 0, 0, 16]
            .iter()
            .flat_map(|w| w.to_le_bytes())
            .collect();
        assert_eq!(
            property(0x7001, &Property::Id(1), &mut prop, true),
            Some(0)
        );
        prop[16..].fill(0);
        assert_eq!(
            property(0x7001, &Property::Id(1), &mut prop, false),
            Some(0)
        );
        assert_eq!(&prop[16..], &16u32.to_le_bytes());
        assert_eq!(simple(0x7001, DeviceCall::Acquire), Some(0));
        assert_eq!(simple(0x7002, DeviceCall::Acquire), Some(0));
        let state = device_state(0x7001, 256).unwrap().unwrap();
        assert_eq!(state.len(), 256);
        let state = device_state(0x7002, 20).unwrap().unwrap();
        assert_eq!(state.len(), 20);
        assert!(matches!(device_data(0x7001, 0, 16), Some(Ok(_))));
        assert_eq!(objects(0x7002, 0).map(|o| o.is_empty()), Some(false));
        release(0x7001);
        release(0x7002);
        assert!(!has(0x7001));
        match xinput_get_state(0) {
            Ok(_) | Err(NOT_CONNECTED) => {}
            Err(r) => panic!("XInputGetState: {r:#x}"),
        }
        let _ = xinput_ids();
    }
}
