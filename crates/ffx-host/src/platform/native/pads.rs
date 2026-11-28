//! Game controllers through gilrs as the game's `XInput` pads (docs/host.md,
//! "Native platform", "Input").
//!
//! The game finds pads by `EnumDevices(DI8DEVCLASS_GAMECTRL)`: each
//! controller's `DIDEVICEINSTANCEA` (type byte 0x15, a gamepad) goes to
//! `Input_AddPadDevice`, which asks `Input_IsXInputDevice` (the host
//! answers from [`ids`]: every pad here is an `XInput` pad) and then
//! reads `XInputGetState(n)` for its `n`-th `XInput` pad each update. So
//! every connected controller gets an `XInput` slot 0..3 (the first free
//! one, kept while it stays connected), is enumerated in slot order, and
//! its gilrs state becomes an `XINPUT_STATE` ([`xinput_state`]): gilrs maps
//! the controller to its standard layout (SDL's mappings), so the face
//! buttons are where an Xbox pad has them.
//!
//! One thread (`ffx-rust-pads`) owns gilrs (not `Send` on every system)
//! and polls it every 4 ms; rumble (`XInputSetState`) goes to it through a
//! channel and plays as a 100 ms force-feedback effect renewed while the
//! game keeps it on, where the controller and the system support it.

use std::sync::Mutex;
use std::sync::mpsc;

/// What the game reads of one pad.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pad {
    /// gilrs' id (its `usize`).
    pub id: usize,
    /// The `XInput` slot.
    pub slot: u32,
    /// USB vendor and product (an Xbox 360 pad's when the system gives
    /// none).
    pub ids: (u32, u32),
    /// Changes so far (`dwPacketNumber`).
    pub packet: u32,
    /// `XINPUT_GAMEPAD_*` bits.
    pub buttons: u16,
    /// Left and right trigger, 0..=1.
    pub triggers: [f32; 2],
    /// Left X, left Y, right X, right Y, -1..=1, up positive.
    pub sticks: [f32; 4],
}

/// The pads by slot, kept by the pads thread.
static PADS: Mutex<Vec<Pad>> = Mutex::new(Vec::new());
/// Rumble requests for the pads thread: slot, left and right motor.
static RUMBLE: Mutex<Option<mpsc::Sender<(u32, u16, u16)>>> = Mutex::new(None);

/// An Xbox 360 controller's vendor and product.
pub const XBOX360: (u32, u32) = (0x045e, 0x028e);

/// `XINPUT_GAMEPAD_*`: D-pad up, down, left, right, Start, Back, the
/// sticks' presses, the shoulders, A, B, X, Y.
pub mod button {
    pub const DPAD_UP: u16 = 0x0001;
    pub const DPAD_DOWN: u16 = 0x0002;
    pub const DPAD_LEFT: u16 = 0x0004;
    pub const DPAD_RIGHT: u16 = 0x0008;
    pub const START: u16 = 0x0010;
    pub const BACK: u16 = 0x0020;
    pub const LEFT_THUMB: u16 = 0x0040;
    pub const RIGHT_THUMB: u16 = 0x0080;
    pub const LEFT_SHOULDER: u16 = 0x0100;
    pub const RIGHT_SHOULDER: u16 = 0x0200;
    pub const A: u16 = 0x1000;
    pub const B: u16 = 0x2000;
    pub const X: u16 = 0x4000;
    pub const Y: u16 = 0x8000;
}

/// The `XINPUT_STATE` of `pad` (16 bytes: packet number, buttons, the
/// triggers 0..=255, the sticks -32768..=32767).
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped to the field's range first
pub fn xinput_state(pad: &Pad) -> [u8; 16] {
    let trigger = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let stick = |v: f32| {
        let v = v.clamp(-1.0, 1.0);
        if v < 0.0 {
            (v * 32768.0).round() as i16
        } else {
            (v * 32767.0).round() as i16
        }
    };
    let mut s = [0u8; 16];
    s[0..4].copy_from_slice(&pad.packet.to_le_bytes());
    s[4..6].copy_from_slice(&pad.buttons.to_le_bytes());
    s[6] = trigger(pad.triggers[0]);
    s[7] = trigger(pad.triggers[1]);
    for (i, v) in pad.sticks.iter().enumerate() {
        s[8 + 2 * i..10 + 2 * i].copy_from_slice(&stick(*v).to_le_bytes());
    }
    s
}

/// A pad's `DIDEVICEINSTANCEA`: `guidInstance` unique to its gilrs id,
/// `guidProduct` `DirectInput`'s product GUID for its vendor and product
/// (`Data1` = product << 16 | vendor, `Data4` "\0\0PIDVID", what
/// `Input_IsXInputDevice` reads), a gamepad (`DI8DEVTYPE_GAMEPAD` 0x15,
/// `DI8DEVTYPEGAMEPAD_STANDARD` 2, `DIDEVTYPE_HID`).
#[must_use]
pub fn instance(pad: &Pad, name: &str) -> Vec<u8> {
    let (vendor, product) = pad.ids;
    let product_guid =
        u128::from(product << 16 | vendor) << 96 | u128::from(u64::from_be_bytes(*b"\0\0PIDVID"));
    let instance_guid = 0xffc0_0000_0000_11f0_8000_4646_5852_0000 | pad.id as u128 & 0xffff;
    super::dinput::instance(instance_guid, product_guid, 0x0001_0215, name)
}

/// Starts the pads thread (once).
pub fn start() {
    static STARTED: std::sync::Once = std::sync::Once::new();
    STARTED.call_once(|| {
        let (tx, rx) = mpsc::channel();
        if let Ok(mut r) = RUMBLE.lock() {
            *r = Some(tx);
        }
        let spawned = std::thread::Builder::new()
            .name("ffx-rust-pads".into())
            .spawn(move || run(&rx));
        if let Err(e) = spawned {
            crate::log::note(&format!("pads: no thread: {e}"));
        }
    });
}

/// The pads by slot (a copy).
#[must_use]
pub fn pads() -> Vec<Pad> {
    PADS.lock().map(|p| p.clone()).unwrap_or_default()
}

/// Names by gilrs id, for the device list.
static NAMES: Mutex<Vec<(usize, String)>> = Mutex::new(Vec::new());

/// The pad's name.
#[must_use]
pub fn name(id: usize) -> String {
    NAMES
        .lock()
        .ok()
        .and_then(|n| n.iter().find(|(i, _)| *i == id).map(|(_, s)| s.clone()))
        .unwrap_or_else(|| "Controller".into())
}

/// `XInputSetState(slot, vibration)`: the motors' speeds.
pub fn rumble(slot: u32, left: u16, right: u16) -> bool {
    RUMBLE
        .lock()
        .ok()
        .and_then(|r| r.as_ref().map(|tx| tx.send((slot, left, right)).is_ok()))
        .unwrap_or(false)
}

/// The first `XInput` slot `pads` leaves free.
#[must_use]
pub fn free_slot(pads: &[Pad]) -> Option<u32> {
    (0..4).find(|s| pads.iter().all(|p| p.slot != *s))
}

fn run(rumble: &mpsc::Receiver<(u32, u16, u16)>) {
    let mut gilrs = match gilrs::Gilrs::new() {
        Ok(g) => g,
        Err(gilrs::Error::NotImplemented(g)) => {
            crate::log::note("pads: gilrs has no back end on this system");
            g
        }
        Err(e) => {
            crate::log::note(&format!("pads: gilrs: {e}"));
            return;
        }
    };
    let mut effects: Vec<(u32, gilrs::ff::Effect)> = Vec::new();
    loop {
        while gilrs.next_event().is_some() {}
        let mut list = pads();
        list.retain(|p| gamepad_id(&gilrs, p.id).is_some());
        for (id, g) in gilrs.gamepads() {
            let id_n: usize = id.into();
            if !list.iter().any(|p| p.id == id_n) {
                let Some(slot) = free_slot(&list) else {
                    continue;
                };
                let ids = match (g.vendor_id(), g.product_id()) {
                    (Some(v), Some(p)) => (u32::from(v), u32::from(p)),
                    _ => XBOX360,
                };
                crate::log::note(&format!(
                    "pads: {} ({:04x}:{:04x}) as XInput pad {slot}",
                    g.name(),
                    ids.0,
                    ids.1
                ));
                if let Ok(mut n) = NAMES.lock() {
                    n.retain(|(i, _)| *i != id_n);
                    n.push((id_n, g.name().to_owned()));
                }
                list.push(Pad {
                    id: id_n,
                    slot,
                    ids,
                    ..Pad::default()
                });
            }
            if let Some(p) = list.iter_mut().find(|p| p.id == id_n) {
                let next = read(&g, p);
                if next != *p {
                    *p = Pad {
                        packet: p.packet.wrapping_add(1),
                        ..next
                    };
                }
            }
        }
        list.sort_by_key(|p| p.slot);
        while let Ok((slot, left, right)) = rumble.try_recv() {
            effects.retain(|(s, _)| *s != slot);
            if (left, right) != (0, 0)
                && let Some(p) = list.iter().find(|p| p.slot == slot)
                && let Some(e) = rumble_effect(&mut gilrs, p.id, left, right)
            {
                effects.push((slot, e));
            }
        }
        if let Ok(mut p) = PADS.lock() {
            *p = list;
        }
        std::thread::sleep(std::time::Duration::from_millis(4));
    }
}

/// The connected gamepad whose id is `id` (gilrs' `usize`).
fn gamepad_id(gilrs: &gilrs::Gilrs, id: usize) -> Option<gilrs::GamepadId> {
    gilrs
        .gamepads()
        .map(|(g, _)| g)
        .find(|&g| usize::from(g) == id)
}

/// A pad's state from gilrs' standard layout.
fn read(g: &gilrs::Gamepad<'_>, old: &Pad) -> Pad {
    use gilrs::{Axis, Button as B};
    let mut buttons = 0u16;
    for (b, bit) in [
        (B::DPadUp, button::DPAD_UP),
        (B::DPadDown, button::DPAD_DOWN),
        (B::DPadLeft, button::DPAD_LEFT),
        (B::DPadRight, button::DPAD_RIGHT),
        (B::Start, button::START),
        (B::Select, button::BACK),
        (B::LeftThumb, button::LEFT_THUMB),
        (B::RightThumb, button::RIGHT_THUMB),
        (B::LeftTrigger, button::LEFT_SHOULDER),
        (B::RightTrigger, button::RIGHT_SHOULDER),
        (B::South, button::A),
        (B::East, button::B),
        (B::West, button::X),
        (B::North, button::Y),
    ] {
        if g.is_pressed(b) {
            buttons |= bit;
        }
    }
    let trigger = |b| {
        g.button_data(b)
            .map_or(0.0, gilrs::ev::state::ButtonData::value)
    };
    Pad {
        buttons,
        triggers: [trigger(B::LeftTrigger2), trigger(B::RightTrigger2)],
        sticks: [
            g.value(Axis::LeftStickX),
            g.value(Axis::LeftStickY),
            g.value(Axis::RightStickX),
            g.value(Axis::RightStickY),
        ],
        ..*old
    }
}

/// A 100 ms rumble on pad `id` (left: the strong motor), repeated while
/// it is kept; `None` where force feedback is missing.
fn rumble_effect(
    gilrs: &mut gilrs::Gilrs,
    id: usize,
    left: u16,
    right: u16,
) -> Option<gilrs::ff::Effect> {
    use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Repeat, Ticks};
    let id = gamepad_id(gilrs, id)?;
    if !gilrs.gamepad(id).is_ff_supported() {
        return None;
    }
    let effect = EffectBuilder::new()
        .add_effect(BaseEffect {
            kind: BaseEffectType::Strong { magnitude: left },
            ..BaseEffect::default()
        })
        .add_effect(BaseEffect {
            kind: BaseEffectType::Weak { magnitude: right },
            ..BaseEffect::default()
        })
        .repeat(Repeat::For(Ticks::from_ms(100)))
        .add_gamepad(&gilrs.gamepad(id))
        .finish(gilrs)
        .ok()?;
    effect.play().ok()?;
    Some(effect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xinput_state_layout() {
        let pad = Pad {
            packet: 7,
            buttons: button::A | button::DPAD_LEFT,
            triggers: [1.0, 0.5],
            sticks: [-1.0, 1.0, 0.0, -0.5],
            ..Pad::default()
        };
        let s = xinput_state(&pad);
        assert_eq!(&s[0..4], &7u32.to_le_bytes());
        assert_eq!(u16::from_le_bytes([s[4], s[5]]), 0x1004);
        assert_eq!((s[6], s[7]), (255, 128));
        let w = |i: usize| i16::from_le_bytes([s[8 + 2 * i], s[9 + 2 * i]]);
        assert_eq!((w(0), w(1), w(2), w(3)), (-32768, 32767, 0, -16384));
        // out of range: clamped
        let s = xinput_state(&Pad {
            triggers: [2.0, -1.0],
            sticks: [3.0; 4],
            ..Pad::default()
        });
        assert_eq!((s[6], s[7]), (255, 0));
        assert_eq!(i16::from_le_bytes([s[8], s[9]]), 32767);
    }

    #[test]
    fn a_pad_enumerates_as_an_xinput_gamepad() {
        let pad = Pad {
            id: 3,
            ids: XBOX360,
            ..Pad::default()
        };
        let inst = instance(&pad, "Xbox 360 Controller");
        assert_eq!(inst.len(), 0x244);
        // the type byte Input_EnumDevicesCallback switches on
        assert_eq!(inst[0x24], 0x15);
        // guidProduct.Data1: what Input_IsXInputDevice compares with the ids
        let data1 = u32::from_le_bytes(inst[0x14..0x18].try_into().unwrap());
        assert_eq!((data1 & 0xffff, data1 >> 16), XBOX360);
        assert_eq!(&inst[0x1c..0x24], b"\0\0PIDVID");
        // guidInstance differs by pad
        let other = instance(&Pad { id: 4, ..pad }, "x");
        assert_ne!(&inst[4..0x14], &other[4..0x14]);
    }

    #[test]
    fn slots_are_kept_and_reused() {
        let p = |id, slot| Pad {
            id,
            slot,
            ..Pad::default()
        };
        assert_eq!(free_slot(&[]), Some(0));
        assert_eq!(free_slot(&[p(1, 0), p(2, 2)]), Some(1));
        assert_eq!(free_slot(&[p(1, 0), p(2, 1), p(3, 2), p(4, 3)]), None);
    }
}
