//! The window's events as the Win32 messages the game's window procedure
//! (`Phyre_PFramework_PWindowWin32Base_WndProc`, `0x00625130`) and its
//! message pump expect, word for word (docs/host.md, "Native platform").
//!
//! Two kinds, as on Windows:
//!
//! - **sent** (Windows calls the window procedure during another call):
//!   `WM_SIZE`, `WM_ACTIVATEAPP`, `WM_ACTIVATE`, `WM_SETFOCUS`,
//!   `WM_KILLFOCUS`, `WM_CLOSE`, `WM_DESTROY`. Of these, only the ones in
//!   [`GAME_SENT`] reach the game: what the Windows platform's
//!   `host_wndproc` queues for it (the rest it answers with
//!   `DefWindowProc` itself), so the game sees the same on both;
//! - **posted** (`PeekMessage` takes them from the queue): the keys
//!   (`WM_KEYDOWN`/`UP`, `WM_SYSKEYDOWN`/`UP` while Alt is held or for
//!   F10), the characters `TranslateMessage` makes of a key (`WM_CHAR`,
//!   `WM_SYSCHAR`), the mouse (`WM_MOUSEMOVE`, the button messages,
//!   `WM_MOUSEWHEEL`), `WM_QUIT`.

/// `WM_DESTROY`.
pub const WM_DESTROY: u32 = 0x0002;
/// `WM_SIZE`.
pub const WM_SIZE: u32 = 0x0005;
/// `WM_ACTIVATE`.
pub const WM_ACTIVATE: u32 = 0x0006;
/// `WM_SETFOCUS`.
pub const WM_SETFOCUS: u32 = 0x0007;
/// `WM_KILLFOCUS`.
pub const WM_KILLFOCUS: u32 = 0x0008;
/// `WM_CLOSE`.
pub const WM_CLOSE: u32 = 0x0010;
/// `WM_QUIT`.
pub const WM_QUIT: u32 = 0x0012;
/// `WM_ACTIVATEAPP`.
pub const WM_ACTIVATEAPP: u32 = 0x001c;
/// `WM_KEYDOWN`.
pub const WM_KEYDOWN: u32 = 0x0100;
/// `WM_KEYUP`.
pub const WM_KEYUP: u32 = 0x0101;
/// `WM_CHAR`.
pub const WM_CHAR: u32 = 0x0102;
/// `WM_SYSKEYDOWN`.
pub const WM_SYSKEYDOWN: u32 = 0x0104;
/// `WM_SYSKEYUP`.
pub const WM_SYSKEYUP: u32 = 0x0105;
/// `WM_SYSCHAR`.
pub const WM_SYSCHAR: u32 = 0x0106;
/// `WM_MOUSEMOVE`.
pub const WM_MOUSEMOVE: u32 = 0x0200;
/// `WM_MOUSEWHEEL`.
pub const WM_MOUSEWHEEL: u32 = 0x020a;

/// The sent messages the game's procedure gets (the Windows platform's
/// `host_wndproc` queues exactly these).
pub const GAME_SENT: [u32; 3] = [WM_SIZE, WM_ACTIVATE, WM_CLOSE];

/// One message: `WM_*`, `wParam`, `lParam`.
pub type Words = [u32; 3];

/// A mouse button: left, right, middle, X1, X2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    /// Left.
    Left,
    /// Right.
    Right,
    /// Middle.
    Middle,
    /// The first side button.
    X1,
    /// The second side button.
    X2,
}

impl Button {
    /// Its index in `DIMOUSESTATE2`'s button bytes.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Middle => 2,
            Self::X1 => 3,
            Self::X2 => 4,
        }
    }

    /// Its `MK_*` bit.
    #[must_use]
    pub fn mk(self) -> u32 {
        match self {
            Self::Left => 0x01,
            Self::Right => 0x02,
            Self::Middle => 0x10,
            Self::X1 => 0x20,
            Self::X2 => 0x40,
        }
    }
}

/// `MK_SHIFT`.
pub const MK_SHIFT: u32 = 0x04;
/// `MK_CONTROL`.
pub const MK_CONTROL: u32 = 0x08;

/// A key's message: `WM_KEYDOWN`/`UP`, or `WM_SYSKEYDOWN`/`UP` while Alt
/// is held (`alt`, the state before this key) and for F10 and Alt itself.
/// `lParam`: repeat count 1, the scan code (bits 16..23), the extended bit
/// (24), the context code (29: Alt held), the previous state (30: down
/// before; set for a repeat and every release) and the transition (31:
/// released).
#[must_use]
pub fn key(key: super::keys::Key, down: bool, repeat: bool, alt: bool) -> Words {
    use super::keys::{VK_F10, VK_MENU};
    let sys = alt || key.vk == VK_F10 || key.vk == VK_MENU;
    let message = match (sys, down) {
        (false, true) => WM_KEYDOWN,
        (false, false) => WM_KEYUP,
        (true, true) => WM_SYSKEYDOWN,
        (true, false) => WM_SYSKEYUP,
    };
    let context = alt || (down && key.vk == VK_MENU);
    let lparam = 1
        | u32::from(key.scan()) << 16
        | u32::from(key.extended()) << 24
        | u32::from(context) << 29
        | u32::from(repeat || !down) << 30
        | u32::from(!down) << 31;
    [message, u32::from(key.vk), lparam]
}

/// The characters `TranslateMessage` posts after the key message `m`:
/// `WM_CHAR` (or `WM_SYSCHAR` after a `WM_SYSKEYDOWN`) per UTF-16 unit of
/// `text`, with the key's `lParam`. None after a release.
#[must_use]
pub fn chars(m: Words, text: &str) -> Vec<Words> {
    let message = match m[0] {
        WM_KEYDOWN => WM_CHAR,
        WM_SYSKEYDOWN => WM_SYSCHAR,
        _ => return Vec::new(),
    };
    text.encode_utf16()
        .map(|u| [message, u32::from(u), m[2]])
        .collect()
}

/// `MAKELPARAM(x, y)`: client coordinates, each 16 bits.
#[must_use]
#[allow(clippy::cast_sign_loss)] // the low 16 bits of each, as Windows packs them
pub fn point(x: i32, y: i32) -> u32 {
    (x as u32 & 0xffff) | (y as u32 & 0xffff) << 16
}

/// `WM_MOUSEMOVE` at client `(x, y)` with the `MK_*` state `mk`.
#[must_use]
pub fn mouse_move(x: i32, y: i32, mk: u32) -> Words {
    [WM_MOUSEMOVE, mk, point(x, y)]
}

/// A button's message at client `(x, y)`; `mk` is the state after the
/// change (Windows reports it so). The side buttons are `WM_XBUTTON*`
/// with the button in `wParam`'s high word.
#[must_use]
pub fn button(b: Button, down: bool, x: i32, y: i32, mk: u32) -> Words {
    let (message, high) = match (b, down) {
        (Button::Left, true) => (0x0201, 0),
        (Button::Left, false) => (0x0202, 0),
        (Button::Right, true) => (0x0204, 0),
        (Button::Right, false) => (0x0205, 0),
        (Button::Middle, true) => (0x0207, 0),
        (Button::Middle, false) => (0x0208, 0),
        (Button::X1, true) => (0x020b, 1),
        (Button::X1, false) => (0x020c, 1),
        (Button::X2, true) => (0x020b, 2),
        (Button::X2, false) => (0x020c, 2),
    };
    [message, high << 16 | mk, point(x, y)]
}

/// `WM_MOUSEWHEEL`: `delta` in Windows' units (120 a notch, positive
/// away from the user) in `wParam`'s high word, the screen point in
/// `lParam`.
#[must_use]
#[allow(clippy::cast_sign_loss)] // the delta's 16 bits
pub fn wheel(delta: i32, screen_x: i32, screen_y: i32, mk: u32) -> Words {
    let d = delta.clamp(-32768, 32767) as u32 & 0xffff;
    [WM_MOUSEWHEEL, d << 16 | mk, point(screen_x, screen_y)]
}

/// `SIZE_RESTORED`, `SIZE_MINIMIZED`, `SIZE_MAXIMIZED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeKind {
    /// `SIZE_RESTORED` (0).
    Restored,
    /// `SIZE_MINIMIZED` (1).
    Minimized,
    /// `SIZE_MAXIMIZED` (2).
    Maximized,
}

/// `WM_SIZE`: the kind and the client size (`lParam` height << 16 |
/// width).
#[must_use]
pub fn size(kind: SizeKind, width: u32, height: u32) -> Words {
    let w = match kind {
        SizeKind::Restored => 0,
        SizeKind::Minimized => 1,
        SizeKind::Maximized => 2,
    };
    [WM_SIZE, w, (height & 0xffff) << 16 | (width & 0xffff)]
}

/// The sent messages of the window gaining (`active`) or losing the
/// focus, in Windows' order: `WM_ACTIVATEAPP`, `WM_ACTIVATE` (`WA_ACTIVE`
/// 1 / `WA_INACTIVE` 0, the minimised state in the high word),
/// `WM_SETFOCUS`; losing it, `WM_ACTIVATE`, `WM_KILLFOCUS`,
/// `WM_ACTIVATEAPP`.
#[must_use]
pub fn activate(active: bool, minimized: bool) -> Vec<Words> {
    let act = [
        WM_ACTIVATE,
        u32::from(minimized) << 16 | u32::from(active),
        0,
    ];
    if active {
        vec![[WM_ACTIVATEAPP, 1, 0], act, [WM_SETFOCUS, 0, 0]]
    } else {
        vec![act, [WM_KILLFOCUS, 0, 0], [WM_ACTIVATEAPP, 0, 0]]
    }
}

#[cfg(test)]
mod tests {
    use super::super::keys::{self, Key};
    use super::*;
    use winit::keyboard::KeyCode;

    fn k(c: KeyCode) -> Key {
        keys::key(c).unwrap()
    }

    #[test]
    fn key_messages_carry_windows_lparam_bits() {
        // 'A' down: repeat 1, scan 0x1e
        assert_eq!(
            key(k(KeyCode::KeyA), true, false, false),
            [WM_KEYDOWN, 0x41, 0x001e_0001]
        );
        // held down (auto-repeat): previous state set
        assert_eq!(
            key(k(KeyCode::KeyA), true, true, false),
            [WM_KEYDOWN, 0x41, 0x401e_0001]
        );
        // released: previous state and transition
        assert_eq!(
            key(k(KeyCode::KeyA), false, false, false),
            [WM_KEYUP, 0x41, 0xc01e_0001]
        );
        // the up arrow: extended
        assert_eq!(
            key(k(KeyCode::ArrowUp), true, false, false),
            [WM_KEYDOWN, 0x26, 0x0148_0001]
        );
        // Alt itself and Alt+Enter are system keys with the context bit
        assert_eq!(
            key(k(KeyCode::AltLeft), true, false, false),
            [WM_SYSKEYDOWN, 0x12, 0x2038_0001]
        );
        assert_eq!(
            key(k(KeyCode::Enter), true, false, true),
            [WM_SYSKEYDOWN, 0x0d, 0x201c_0001]
        );
        assert_eq!(
            key(k(KeyCode::AltLeft), false, false, true),
            [WM_SYSKEYUP, 0x12, 0xe038_0001]
        );
        // F10 without Alt: a system key, no context bit
        assert_eq!(
            key(k(KeyCode::F10), true, false, false),
            [WM_SYSKEYDOWN, 0x79, 0x0044_0001]
        );
        // the right Control: VK_CONTROL, extended
        assert_eq!(
            key(k(KeyCode::ControlRight), false, false, false),
            [WM_KEYUP, 0x11, 0xc11d_0001]
        );
    }

    #[test]
    fn characters_follow_the_key_down() {
        let down = key(k(KeyCode::KeyA), true, false, false);
        assert_eq!(chars(down, "a"), [[WM_CHAR, 0x61, 0x001e_0001]]);
        // a character outside the BMP: two units
        assert_eq!(chars(down, "\u{1f600}").len(), 2);
        let sys = key(k(KeyCode::KeyA), true, false, true);
        assert_eq!(chars(sys, "a")[0][0], WM_SYSCHAR);
        assert_eq!(
            chars(key(k(KeyCode::KeyA), false, false, false), "a"),
            Vec::<Words>::new()
        );
    }

    #[test]
    fn mouse_messages() {
        assert_eq!(mouse_move(10, 20, 0), [WM_MOUSEMOVE, 0, 0x0014_000a]);
        // negative coordinates: their low 16 bits (GET_X_LPARAM sign-extends)
        assert_eq!(point(-1, -2), 0xfffe_ffff);
        assert_eq!(
            button(Button::Left, true, 1, 2, 1),
            [0x0201, 1, 0x0002_0001]
        );
        assert_eq!(
            button(Button::Right, false, 1, 2, 0),
            [0x0205, 0, 0x0002_0001]
        );
        assert_eq!(
            button(Button::X2, true, 0, 0, 0x40),
            [0x020b, 0x0002_0040, 0]
        );
        assert_eq!(
            wheel(120, 5, 6, 0),
            [WM_MOUSEWHEEL, 0x0078_0000, 0x0006_0005]
        );
        assert_eq!(wheel(-120, 0, 0, MK_SHIFT), [WM_MOUSEWHEEL, 0xff88_0004, 0]);
    }

    #[test]
    fn window_state_messages() {
        assert_eq!(
            size(SizeKind::Restored, 1280, 720),
            [WM_SIZE, 0, 0x02d0_0500]
        );
        assert_eq!(size(SizeKind::Minimized, 0, 0), [WM_SIZE, 1, 0]);
        let on = activate(true, false);
        assert_eq!(
            on.iter().map(|m| m[0]).collect::<Vec<_>>(),
            [WM_ACTIVATEAPP, WM_ACTIVATE, WM_SETFOCUS]
        );
        assert_eq!(on[1], [WM_ACTIVATE, 1, 0]);
        let off = activate(false, true);
        assert_eq!(off[0], [WM_ACTIVATE, 0x0001_0000, 0]);
        assert_eq!(
            off.iter().map(|m| m[0]).collect::<Vec<_>>(),
            [WM_ACTIVATE, WM_KILLFOCUS, WM_ACTIVATEAPP]
        );
        // the game gets what host_wndproc forwards on Windows
        let delivered: Vec<u32> = on
            .iter()
            .chain(&off)
            .map(|m| m[0])
            .filter(|m| GAME_SENT.contains(m))
            .collect();
        assert_eq!(delivered, [WM_ACTIVATE, WM_ACTIVATE]);
    }
}
