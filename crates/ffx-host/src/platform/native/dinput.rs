//! The system keyboard and mouse as `DirectInput8` devices, made from the
//! window's events (docs/host.md, "Native platform", "Input").
//!
//! What the game asks of them (`Input_AddKeyboard`, `Input_AddMouse`,
//! `Keyboard_Update`, the mouse update): `CreateDevice(GUID_SysKeyboard |
//! GUID_SysMouse)`, `SetDataFormat(c_dfDIKeyboard | c_dfDIMouse2)`,
//! `SetCooperativeLevel(window, 6)` (non-exclusive, foreground),
//! `Acquire`, then `GetDeviceState` every update: the keyboard's 256 scan
//! code bytes (0x80: down), the mouse's `DIMOUSESTATE2` (motion since the
//! last read in `lX`, `lY`, the wheel in `lZ`, eight button bytes).
//!
//! Foreground devices behave as `DirectInput`'s do: losing the window's
//! focus unacquires them (the next `GetDeviceState` fails with
//! `DIERR_INPUTLOST`, later ones with `DIERR_NOTACQUIRED`), `Acquire`
//! fails with `DIERR_OTHERAPPHASPRIO` while the window has no focus, and
//! the game acquires again once it has (`Keyboard_Update`).

use std::collections::HashMap;

/// `GUID_SysKeyboard`.
pub const GUID_SYS_KEYBOARD: u128 = 0x6f1d_2b61_d5a0_11cf_bfc7_4445_5354_0000;
/// `GUID_SysMouse`.
pub const GUID_SYS_MOUSE: u128 = 0x6f1d_2b60_d5a0_11cf_bfc7_4445_5354_0000;

/// `DI_OK`.
pub const DI_OK: i32 = 0;
/// `S_FALSE` (`DI_NOEFFECT`: already acquired).
pub const DI_NOEFFECT: i32 = 1;
/// `DIERR_INPUTLOST`.
pub const DIERR_INPUTLOST: i32 = 0x8007_001e_u32.cast_signed();
/// `DIERR_NOTACQUIRED`.
pub const DIERR_NOTACQUIRED: i32 = 0x8007_000c_u32.cast_signed();
/// `DIERR_OTHERAPPHASPRIO`.
pub const DIERR_OTHERAPPHASPRIO: i32 = 0x8007_0005_u32.cast_signed();
/// `DIERR_INVALIDPARAM`.
pub const DIERR_INVALIDPARAM: i32 = 0x8007_0057_u32.cast_signed();
/// `DIERR_DEVICENOTREG` (no such device).
pub const DIERR_DEVICENOTREG: i32 = 0x8004_0154_u32.cast_signed();
/// `DIERR_UNSUPPORTED` (`E_NOTIMPL`).
pub const DIERR_UNSUPPORTED: i32 = 0x8000_4001_u32.cast_signed();

/// Which system device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `GUID_SysKeyboard`.
    Keyboard,
    /// `GUID_SysMouse`.
    Mouse,
}

impl Kind {
    /// The device of `guid`.
    #[must_use]
    pub fn of(guid: u128) -> Option<Self> {
        match guid {
            GUID_SYS_KEYBOARD => Some(Self::Keyboard),
            GUID_SYS_MOUSE => Some(Self::Mouse),
            _ => None,
        }
    }

    /// Its name (`tszProductName`).
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Keyboard => "Keyboard",
            Self::Mouse => "Mouse",
        }
    }

    /// `dwDevType`: `DI8DEVTYPE_KEYBOARD` 0x13 with `DI8DEVTYPEKEYBOARD_PCENH`
    /// (4), `DI8DEVTYPE_MOUSE` 0x12 with `DI8DEVTYPEMOUSE_UNKNOWN`
    /// (1), as `DirectInput` describes the system devices.
    #[must_use]
    pub fn dev_type(self) -> u32 {
        match self {
            Self::Keyboard => 0x13 | 4 << 8,
            Self::Mouse => 0x12 | 1 << 8,
        }
    }

    fn guid(self) -> u128 {
        match self {
            Self::Keyboard => GUID_SYS_KEYBOARD,
            Self::Mouse => GUID_SYS_MOUSE,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Device {
    kind: Kind,
    acquired: bool,
    /// Unacquired by a focus loss and not read since (`DIERR_INPUTLOST`).
    lost: bool,
}

/// The keyboard and the mouse as the window's events leave them, and
/// the game's devices on them.
#[derive(Debug)]
pub struct Input {
    /// `DIK_*` → 0x80 while down.
    keys: [u8; 256],
    /// Motion and wheel since the mouse devices' last read.
    motion: [i32; 3],
    /// `DIMOUSESTATE2`'s button bytes.
    buttons: [u8; 8],
    /// The window has the focus.
    focused: bool,
    devices: HashMap<u32, Device>,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            keys: [0; 256],
            motion: [0; 3],
            buttons: [0; 8],
            focused: false,
            devices: HashMap::new(),
        }
    }
}

impl Input {
    /// A key went down or up (only while focused: a foreground device
    /// reads nothing otherwise).
    pub fn key(&mut self, dik: u8, down: bool) {
        if self.focused {
            self.keys[usize::from(dik)] = if down { 0x80 } else { 0 };
        }
    }

    /// The mouse moved by `(dx, dy)` counts (raw motion, not the pointer).
    pub fn motion(&mut self, dx: i32, dy: i32) {
        if self.focused {
            self.motion[0] = self.motion[0].saturating_add(dx);
            self.motion[1] = self.motion[1].saturating_add(dy);
        }
    }

    /// The wheel turned by `delta` (120 a notch).
    pub fn wheel(&mut self, delta: i32) {
        if self.focused {
            self.motion[2] = self.motion[2].saturating_add(delta);
        }
    }

    /// Button `index` (`DIMOUSESTATE2.rgbButtons`) went down or up.
    pub fn button(&mut self, index: usize, down: bool) {
        if self.focused
            && let Some(b) = self.buttons.get_mut(index)
        {
            *b = if down { 0x80 } else { 0 };
        }
    }

    /// The window gained or lost the focus: losing it releases every key
    /// and button and unacquires every device.
    pub fn focus(&mut self, focused: bool) {
        self.focused = focused;
        if !focused {
            self.keys = [0; 256];
            self.buttons = [0; 8];
            self.motion = [0; 3];
            for d in self.devices.values_mut() {
                if d.acquired {
                    d.acquired = false;
                    d.lost = true;
                }
            }
        }
    }

    /// Whether the window has the focus.
    #[must_use]
    pub fn focused(&self) -> bool {
        self.focused
    }

    /// `CreateDevice(guid)` for the layer's `handle`: the device's name.
    ///
    /// # Errors
    /// `DIERR_DEVICENOTREG` for anything but the system keyboard and mouse.
    pub fn open(&mut self, handle: u32, guid: u128) -> Result<&'static str, i32> {
        let kind = Kind::of(guid).ok_or(DIERR_DEVICENOTREG)?;
        self.devices.insert(
            handle,
            Device {
                kind,
                acquired: false,
                lost: false,
            },
        );
        Ok(kind.name())
    }

    /// Whether `handle` is one of these devices.
    #[must_use]
    pub fn has(&self, handle: u32) -> bool {
        self.devices.contains_key(&handle)
    }

    /// The device's kind.
    #[must_use]
    pub fn kind(&self, handle: u32) -> Option<Kind> {
        self.devices.get(&handle).map(|d| d.kind)
    }

    /// The game released `handle`.
    pub fn release(&mut self, handle: u32) {
        self.devices.remove(&handle);
    }

    /// `Acquire`.
    pub fn acquire(&mut self, handle: u32) -> Option<i32> {
        let focused = self.focused;
        let d = self.devices.get_mut(&handle)?;
        Some(if d.acquired {
            DI_NOEFFECT
        } else if !focused {
            DIERR_OTHERAPPHASPRIO
        } else {
            d.acquired = true;
            d.lost = false;
            if d.kind == Kind::Mouse {
                self.motion = [0; 3];
            }
            DI_OK
        })
    }

    /// `Unacquire`.
    pub fn unacquire(&mut self, handle: u32) -> Option<i32> {
        let d = self.devices.get_mut(&handle)?;
        let was = d.acquired;
        d.acquired = false;
        d.lost = false;
        Some(if was { DI_OK } else { DI_NOEFFECT })
    }

    /// `GetDeviceState(size)`: the keyboard's first `size` scan-code
    /// bytes; the mouse's `DIMOUSESTATE` (16 bytes) or `DIMOUSESTATE2`
    /// (20), its motion taken.
    ///
    /// # Errors
    /// The `HRESULT` of a device that is not acquired, or a size neither
    /// format has.
    pub fn state(&mut self, handle: u32, size: usize) -> Option<Result<Vec<u8>, i32>> {
        let d = self.devices.get_mut(&handle)?;
        if !d.acquired {
            let r = if d.lost {
                DIERR_INPUTLOST
            } else {
                DIERR_NOTACQUIRED
            };
            d.lost = false;
            return Some(Err(r));
        }
        Some(match d.kind {
            Kind::Keyboard if size <= 256 => Ok(self.keys[..size].to_vec()),
            Kind::Mouse if size == 16 || size == 20 => {
                let mut out: Vec<u8> = self.motion.iter().flat_map(|v| v.to_le_bytes()).collect();
                out.extend_from_slice(&self.buttons[..size - 12]);
                self.motion = [0; 3];
                Ok(out)
            }
            _ => Err(DIERR_INVALIDPARAM),
        })
    }

    /// `GetCapabilities` (`DIDEVCAPS`, at least its first 0x18 bytes:
    /// size, flags `DIDC_ATTACHED`, type, axes, buttons, POVs) or
    /// `GetDeviceInfo` (`DIDEVICEINSTANCEA`) into `data`, its `dwSize`
    /// kept.
    pub fn info(&self, handle: u32, what: super::DeviceInfo, data: &mut [u8]) -> Option<i32> {
        let kind = self.kind(handle)?;
        let full = match what {
            super::DeviceInfo::Capabilities => {
                let (axes, buttons) = match kind {
                    Kind::Keyboard => (0, 128),
                    Kind::Mouse => (3, 8),
                };
                [
                    u32::try_from(data.len()).unwrap_or(0),
                    1,
                    kind.dev_type(),
                    axes,
                    buttons,
                    0,
                ]
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .collect()
            }
            super::DeviceInfo::Instance => {
                instance(kind.guid(), kind.guid(), kind.dev_type(), kind.name())
            }
        };
        let n = data.len().min(full.len());
        data[4..n].copy_from_slice(&full[4..n]);
        Some(DI_OK)
    }
}

/// A `DIDEVICEINSTANCEA` (0x244 bytes): `dwSize`, `guidInstance` (+4),
/// `guidProduct` (+0x14), `dwDevType` (+0x24), `tszInstanceName`
/// (+0x28) and `tszProductName` (+0x12c), 260 bytes each, the force
/// feedback driver and the HID usage zero.
#[must_use]
pub fn instance(guid_instance: u128, guid_product: u128, dev_type: u32, name: &str) -> Vec<u8> {
    let mut b = vec![0u8; crate::input::DEVICE_INSTANCE];
    b[0..4].copy_from_slice(&0x244u32.to_le_bytes());
    b[4..0x14].copy_from_slice(&guid_bytes(guid_instance));
    b[0x14..0x24].copy_from_slice(&guid_bytes(guid_product));
    b[0x24..0x28].copy_from_slice(&dev_type.to_le_bytes());
    let n = name.len().min(259);
    b[0x28..0x28 + n].copy_from_slice(&name.as_bytes()[..n]);
    b[0x12c..0x12c + n].copy_from_slice(&name.as_bytes()[..n]);
    b
}

/// A GUID's 16 bytes as Windows stores it (`Data1`..`Data3` little-endian,
/// `Data4` in order) from its `u128` (`Data1` in the top 32 bits, as
/// `crate::com::read_guid` reads it).
#[must_use]
#[allow(clippy::cast_possible_truncation)] // the fields' bits
pub fn guid_bytes(g: u128) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[0..4].copy_from_slice(&((g >> 96) as u32).to_le_bytes());
    b[4..6].copy_from_slice(&((g >> 80) as u16).to_le_bytes());
    b[6..8].copy_from_slice(&((g >> 64) as u16).to_le_bytes());
    b[8..16].copy_from_slice(&(g as u64).to_be_bytes());
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreground_devices_follow_the_focus() {
        let mut i = Input::default();
        assert_eq!(i.open(7, GUID_SYS_KEYBOARD), Ok("Keyboard"));
        assert_eq!(i.open(8, 0x1234), Err(DIERR_DEVICENOTREG));
        // not focused yet: Acquire refused, the state not acquired
        assert_eq!(i.acquire(7), Some(DIERR_OTHERAPPHASPRIO));
        assert_eq!(i.state(7, 256), Some(Err(DIERR_NOTACQUIRED)));
        i.focus(true);
        assert_eq!(i.acquire(7), Some(DI_OK));
        assert_eq!(i.acquire(7), Some(DI_NOEFFECT));
        i.key(0xc8, true);
        let s = i.state(7, 256).unwrap().unwrap();
        assert_eq!((s.len(), s[0xc8], s[0x1c]), (256, 0x80, 0));
        // focus lost: keys released, input lost once, then not acquired
        i.focus(false);
        i.key(0x1c, true);
        assert_eq!(i.state(7, 256), Some(Err(DIERR_INPUTLOST)));
        assert_eq!(i.state(7, 256), Some(Err(DIERR_NOTACQUIRED)));
        assert_eq!(i.acquire(7), Some(DIERR_OTHERAPPHASPRIO));
        i.focus(true);
        assert_eq!(i.acquire(7), Some(DI_OK));
        assert_eq!(i.state(7, 256).unwrap().unwrap()[0xc8], 0);
        // not our device: the layer answers
        assert_eq!(i.state(9, 256), None);
    }

    #[test]
    fn the_mouse_reads_motion_since_the_last_read() {
        let mut i = Input::default();
        i.focus(true);
        i.open(3, GUID_SYS_MOUSE).unwrap();
        i.motion(5, 5); // before Acquire: dropped by it
        assert_eq!(i.acquire(3), Some(DI_OK));
        i.motion(3, -4);
        i.motion(1, 1);
        i.wheel(-120);
        i.button(0, true);
        let s = i.state(3, 20).unwrap().unwrap();
        let w = |n: usize| i32::from_le_bytes(s[4 * n..4 * n + 4].try_into().unwrap());
        assert_eq!((w(0), w(1), w(2)), (4, -3, -120));
        assert_eq!(&s[12..20], &[0x80, 0, 0, 0, 0, 0, 0, 0]);
        let s = i.state(3, 16).unwrap().unwrap();
        assert_eq!((s.len(), &s[..12]), (16, &[0u8; 12][..]));
        assert_eq!(s[12], 0x80);
        assert_eq!(i.state(3, 24), Some(Err(DIERR_INVALIDPARAM)));
    }

    #[test]
    fn device_descriptions() {
        let mut i = Input::default();
        i.open(1, GUID_SYS_MOUSE).unwrap();
        let mut caps = vec![0u8; 0x2c];
        caps[0] = 0x2c;
        assert_eq!(
            i.info(1, super::super::DeviceInfo::Capabilities, &mut caps),
            Some(0)
        );
        assert_eq!(
            &caps[..0x18],
            &[
                0x2c, 0, 0, 0, 1, 0, 0, 0, 0x12, 1, 0, 0, 3, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0
            ]
        );
        let inst = instance(GUID_SYS_MOUSE, GUID_SYS_MOUSE, 0x112, "Mouse");
        // GUID_SysMouse {6F1D2B60-D5A0-11CF-BFC7-444553540000} as stored
        assert_eq!(
            &inst[4..0x14],
            &[
                0x60, 0x2b, 0x1d, 0x6f, 0xa0, 0xd5, 0xcf, 0x11, 0xbf, 0xc7, 0x44, 0x45, 0x53, 0x54,
                0, 0
            ]
        );
        assert_eq!(inst[0x24], 0x12);
        assert_eq!(&inst[0x12c..0x131], b"Mouse");
    }
}
