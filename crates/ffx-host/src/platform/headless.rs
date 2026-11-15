//! [`HeadlessPlatform`]: no window, no input devices, no device sound
//! (docs/host.md, "The platform seam"). What the Linux check and the
//! null-device runs had as stand-ins: a fake window that gets no messages
//! and is the 1920 x 1080 screen, and the recording null device
//! ([`crate::gfx::Null`]).

use super::{Message, Platform, Point, Rect, WindowRequest};

/// The stand-in window's handle (`CreateWindowExA`).
pub const WINDOW: u32 = 0x0001_0010;

/// The headless platform.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeadlessPlatform;

/// The stand-in window and the screen: 1920 x 1080 at the origin.
const SCREEN: Rect = Rect {
    left: 0,
    top: 0,
    right: 1920,
    bottom: 1080,
};

impl Platform for HeadlessPlatform {
    fn name(&self) -> &'static str {
        "headless"
    }

    fn windowed(&self) -> bool {
        false
    }

    fn set_icon_source(&self, _: &std::path::Path) {}

    fn register_class(&self, _: u32, _: &[u8]) -> u16 {
        0xc001
    }

    fn create_window(&self, _: &WindowRequest<'_>) -> u32 {
        WINDOW
    }

    fn show_window(&self, _: u32, _: i32) -> i32 {
        0
    }

    fn set_window_pos(&self, _: u32, _: u32, _: i32, _: i32, _: i32, _: i32, _: u32) -> i32 {
        1
    }

    fn client_rect(&self, _: u32) -> (i32, Rect) {
        (1, SCREEN)
    }

    fn window_rect(&self, _: u32) -> (i32, Rect) {
        (1, SCREEN)
    }

    fn adjust_window_rect(&self, _: &mut Rect, _: u32, _: i32) -> i32 {
        1
    }

    fn client_to_screen(&self, _: u32, _: &mut Point) -> i32 {
        1
    }

    fn take_sent(&self) -> Option<[u32; 4]> {
        None
    }

    fn peek_message(&self, _: u32, _: u32, _: u32, _: u32) -> Option<Message> {
        None
    }

    fn translate_message(&self, _: &Message) -> i32 {
        1
    }

    fn dispatch_message(&self, _: &Message) -> u32 {
        0
    }

    fn def_window_proc(&self, _: u32, _: u32, _: u32, _: u32) -> u32 {
        0
    }

    fn set_window_long(&self, _: u32, _: i32, _: i32) -> i32 {
        0
    }

    fn window_long(&self, _: u32, _: i32) -> i32 {
        0
    }

    fn set_window_text(&self, _: u32, _: &[u8]) -> i32 {
        1
    }

    fn update_window(&self, _: u32) -> i32 {
        1
    }

    fn destroy_window(&self, _: u32) -> i32 {
        1
    }

    fn set_parent(&self, _: u32, _: u32) -> u32 {
        0
    }

    fn post_message(&self, _: u32, _: u32, _: u32, _: u32) -> i32 {
        1
    }

    fn post_quit_message(&self, _: i32) {}

    fn system_metric(&self, index: i32) -> i32 {
        match index {
            0 | 16 | 78 => 1920, // SM_CXSCREEN, SM_CXFULLSCREEN, SM_CXVIRTUALSCREEN
            1 | 17 | 79 => 1080,
            80 => 1, // SM_CMONITORS
            _ => 0,
        }
    }

    fn cursor_pos(&self) -> Point {
        // far outside the window, where the harness's background games
        // keep the pointer the game reads (docs/harness.md, "The mouse in
        // background games"), so no menu sees a hover
        Point {
            x: -32108,
            y: -32131,
        }
    }

    fn set_cursor_pos(&self, _: i32, _: i32) -> i32 {
        1
    }

    fn clip_cursor(&self, _: Option<Rect>) -> i32 {
        1
    }

    fn clip_rect(&self) -> Rect {
        SCREEN
    }

    fn show_cursor(&self, _: i32) -> i32 {
        0
    }

    fn timer_period(&self, _: bool, _: u32) -> u32 {
        0
    }

    fn graphics(&self) -> Box<dyn crate::gfx::Backend> {
        Box::new(crate::gfx::Null::default())
    }
}
