//! Physical keys (winit's `KeyCode`, the key's place on a US keyboard, not
//! its label) as the game's two key sources see them: `DirectInput`'s scan
//! code (`DIK_*`, the index into `GetDeviceState`'s 256 bytes) and the
//! window messages' virtual key and `lParam` scan code (docs/host.md,
//! "Native platform").
//!
//! `DIK_*` codes are the PC keyboard's set-1 scan codes, `0x80` added for
//! the `0xE0`-prefixed ones (`DIK_RCONTROL` 0x9D, the arrows 0xC8..). The
//! messages carry the low 7 bits in `lParam` bits 16..23 and the prefix as
//! the extended-key bit 24, with two exceptions Windows has: Num Lock is
//! scan code 0x45 *with* the extended bit, Pause 0x45 without it
//! (`DIK_NUMLOCK` 0x45, `DIK_PAUSE` 0xC5).

use winit::keyboard::KeyCode;

/// One key: its `DIK_*` code and its virtual key (`VK_*`; the generic
/// `VK_SHIFT`, `VK_CONTROL`, `VK_MENU` for both sides, as `WM_KEYDOWN`
/// reports them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    /// `DIK_*`.
    pub dik: u8,
    /// `VK_*`.
    pub vk: u8,
}

/// `VK_MENU` (either Alt).
pub const VK_MENU: u8 = 0x12;
/// `VK_F10` (a system key, as Alt combinations are).
pub const VK_F10: u8 = 0x79;

impl Key {
    /// The scan code in `lParam` bits 16..23.
    #[must_use]
    pub fn scan(self) -> u8 {
        if self.dik == 0xc5 {
            0x45
        } else {
            self.dik & 0x7f
        }
    }

    /// The extended-key bit (`lParam` bit 24).
    #[must_use]
    pub fn extended(self) -> bool {
        (self.dik & 0x80 != 0 && self.dik != 0xc5) || self.dik == 0x45
    }
}

/// Every mapped key: winit's code, `DIK_*`, `VK_*`.
const KEYS: &[(KeyCode, u8, u8)] = &[
    (KeyCode::Escape, 0x01, 0x1b),
    (KeyCode::Digit1, 0x02, b'1'),
    (KeyCode::Digit2, 0x03, b'2'),
    (KeyCode::Digit3, 0x04, b'3'),
    (KeyCode::Digit4, 0x05, b'4'),
    (KeyCode::Digit5, 0x06, b'5'),
    (KeyCode::Digit6, 0x07, b'6'),
    (KeyCode::Digit7, 0x08, b'7'),
    (KeyCode::Digit8, 0x09, b'8'),
    (KeyCode::Digit9, 0x0a, b'9'),
    (KeyCode::Digit0, 0x0b, b'0'),
    (KeyCode::Minus, 0x0c, 0xbd),
    (KeyCode::Equal, 0x0d, 0xbb),
    (KeyCode::Backspace, 0x0e, 0x08),
    (KeyCode::Tab, 0x0f, 0x09),
    (KeyCode::KeyQ, 0x10, b'Q'),
    (KeyCode::KeyW, 0x11, b'W'),
    (KeyCode::KeyE, 0x12, b'E'),
    (KeyCode::KeyR, 0x13, b'R'),
    (KeyCode::KeyT, 0x14, b'T'),
    (KeyCode::KeyY, 0x15, b'Y'),
    (KeyCode::KeyU, 0x16, b'U'),
    (KeyCode::KeyI, 0x17, b'I'),
    (KeyCode::KeyO, 0x18, b'O'),
    (KeyCode::KeyP, 0x19, b'P'),
    (KeyCode::BracketLeft, 0x1a, 0xdb),
    (KeyCode::BracketRight, 0x1b, 0xdd),
    (KeyCode::Enter, 0x1c, 0x0d),
    (KeyCode::ControlLeft, 0x1d, 0x11),
    (KeyCode::KeyA, 0x1e, b'A'),
    (KeyCode::KeyS, 0x1f, b'S'),
    (KeyCode::KeyD, 0x20, b'D'),
    (KeyCode::KeyF, 0x21, b'F'),
    (KeyCode::KeyG, 0x22, b'G'),
    (KeyCode::KeyH, 0x23, b'H'),
    (KeyCode::KeyJ, 0x24, b'J'),
    (KeyCode::KeyK, 0x25, b'K'),
    (KeyCode::KeyL, 0x26, b'L'),
    (KeyCode::Semicolon, 0x27, 0xba),
    (KeyCode::Quote, 0x28, 0xde),
    (KeyCode::Backquote, 0x29, 0xc0),
    (KeyCode::ShiftLeft, 0x2a, 0x10),
    (KeyCode::Backslash, 0x2b, 0xdc),
    (KeyCode::KeyZ, 0x2c, b'Z'),
    (KeyCode::KeyX, 0x2d, b'X'),
    (KeyCode::KeyC, 0x2e, b'C'),
    (KeyCode::KeyV, 0x2f, b'V'),
    (KeyCode::KeyB, 0x30, b'B'),
    (KeyCode::KeyN, 0x31, b'N'),
    (KeyCode::KeyM, 0x32, b'M'),
    (KeyCode::Comma, 0x33, 0xbc),
    (KeyCode::Period, 0x34, 0xbe),
    (KeyCode::Slash, 0x35, 0xbf),
    (KeyCode::ShiftRight, 0x36, 0x10),
    (KeyCode::NumpadMultiply, 0x37, 0x6a),
    (KeyCode::AltLeft, 0x38, VK_MENU),
    (KeyCode::Space, 0x39, 0x20),
    (KeyCode::CapsLock, 0x3a, 0x14),
    (KeyCode::F1, 0x3b, 0x70),
    (KeyCode::F2, 0x3c, 0x71),
    (KeyCode::F3, 0x3d, 0x72),
    (KeyCode::F4, 0x3e, 0x73),
    (KeyCode::F5, 0x3f, 0x74),
    (KeyCode::F6, 0x40, 0x75),
    (KeyCode::F7, 0x41, 0x76),
    (KeyCode::F8, 0x42, 0x77),
    (KeyCode::F9, 0x43, 0x78),
    (KeyCode::F10, 0x44, VK_F10),
    (KeyCode::NumLock, 0x45, 0x90),
    (KeyCode::ScrollLock, 0x46, 0x91),
    (KeyCode::Numpad7, 0x47, 0x67),
    (KeyCode::Numpad8, 0x48, 0x68),
    (KeyCode::Numpad9, 0x49, 0x69),
    (KeyCode::NumpadSubtract, 0x4a, 0x6d),
    (KeyCode::Numpad4, 0x4b, 0x64),
    (KeyCode::Numpad5, 0x4c, 0x65),
    (KeyCode::Numpad6, 0x4d, 0x66),
    (KeyCode::NumpadAdd, 0x4e, 0x6b),
    (KeyCode::Numpad1, 0x4f, 0x61),
    (KeyCode::Numpad2, 0x50, 0x62),
    (KeyCode::Numpad3, 0x51, 0x63),
    (KeyCode::Numpad0, 0x52, 0x60),
    (KeyCode::NumpadDecimal, 0x53, 0x6e),
    (KeyCode::IntlBackslash, 0x56, 0xe2),
    (KeyCode::F11, 0x57, 0x7a),
    (KeyCode::F12, 0x58, 0x7b),
    (KeyCode::F13, 0x64, 0x7c),
    (KeyCode::F14, 0x65, 0x7d),
    (KeyCode::F15, 0x66, 0x7e),
    (KeyCode::KanaMode, 0x70, 0x15),
    (KeyCode::IntlRo, 0x73, 0xc1),
    (KeyCode::Convert, 0x79, 0x1c),
    (KeyCode::NonConvert, 0x7b, 0x1d),
    (KeyCode::IntlYen, 0x7d, 0xdc),
    (KeyCode::NumpadEqual, 0x8d, 0x92),
    (KeyCode::MediaTrackPrevious, 0x90, 0xb1),
    (KeyCode::MediaTrackNext, 0x99, 0xb0),
    (KeyCode::NumpadEnter, 0x9c, 0x0d),
    (KeyCode::ControlRight, 0x9d, 0x11),
    (KeyCode::AudioVolumeMute, 0xa0, 0xad),
    (KeyCode::MediaPlayPause, 0xa2, 0xb3),
    (KeyCode::MediaStop, 0xa4, 0xb2),
    (KeyCode::AudioVolumeDown, 0xae, 0xae),
    (KeyCode::AudioVolumeUp, 0xb0, 0xaf),
    (KeyCode::NumpadDivide, 0xb5, 0x6f),
    (KeyCode::PrintScreen, 0xb7, 0x2c),
    (KeyCode::AltRight, 0xb8, VK_MENU),
    (KeyCode::Pause, 0xc5, 0x13),
    (KeyCode::Home, 0xc7, 0x24),
    (KeyCode::ArrowUp, 0xc8, 0x26),
    (KeyCode::PageUp, 0xc9, 0x21),
    (KeyCode::ArrowLeft, 0xcb, 0x25),
    (KeyCode::ArrowRight, 0xcd, 0x27),
    (KeyCode::End, 0xcf, 0x23),
    (KeyCode::ArrowDown, 0xd0, 0x28),
    (KeyCode::PageDown, 0xd1, 0x22),
    (KeyCode::Insert, 0xd2, 0x2d),
    (KeyCode::Delete, 0xd3, 0x2e),
    (KeyCode::SuperLeft, 0xdb, 0x5b),
    (KeyCode::SuperRight, 0xdc, 0x5c),
    (KeyCode::ContextMenu, 0xdd, 0x5d),
    (KeyCode::Sleep, 0xdf, 0x5f),
];

/// The key at winit's physical `code` (`None`: a key the PC keyboard has
/// no scan code for).
#[must_use]
pub fn key(code: KeyCode) -> Option<Key> {
    KEYS.iter()
        .find(|&&(c, _, _)| c == code)
        .map(|&(_, dik, vk)| Key { dik, vk })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_codes_and_virtual_keys() {
        let k = |c| key(c).unwrap();
        // the game's defaults: arrows, Enter, Escape, the letters
        assert_eq!(
            k(KeyCode::ArrowUp),
            Key {
                dik: 0xc8,
                vk: 0x26
            }
        );
        assert_eq!(
            (k(KeyCode::ArrowUp).scan(), k(KeyCode::ArrowUp).extended()),
            (0x48, true)
        );
        assert_eq!(
            k(KeyCode::Enter),
            Key {
                dik: 0x1c,
                vk: 0x0d
            }
        );
        assert!(!k(KeyCode::Enter).extended());
        assert_eq!(
            (
                k(KeyCode::NumpadEnter).scan(),
                k(KeyCode::NumpadEnter).extended()
            ),
            (0x1c, true)
        );
        assert_eq!(
            k(KeyCode::KeyA),
            Key {
                dik: 0x1e,
                vk: b'A'
            }
        );
        // both shifts are VK_SHIFT, told apart by the scan code
        assert_eq!(
            (k(KeyCode::ShiftLeft).vk, k(KeyCode::ShiftRight).vk),
            (0x10, 0x10)
        );
        assert_eq!(
            (k(KeyCode::ShiftLeft).dik, k(KeyCode::ShiftRight).dik),
            (0x2a, 0x36)
        );
        // Windows' two exceptions
        assert_eq!(
            (k(KeyCode::NumLock).scan(), k(KeyCode::NumLock).extended()),
            (0x45, true)
        );
        assert_eq!(
            (k(KeyCode::Pause).scan(), k(KeyCode::Pause).extended()),
            (0x45, false)
        );
        assert_eq!(key(KeyCode::Fn), None);
    }

    #[test]
    fn the_table_has_no_duplicates() {
        for (i, a) in KEYS.iter().enumerate() {
            for b in &KEYS[i + 1..] {
                assert_ne!(a.0, b.0, "{:?} twice", a.0);
                assert_ne!(a.1, b.1, "DIK {:#x} twice", a.1);
            }
        }
    }
}
