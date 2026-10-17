//! The PC port's pointer (mouse and touch) zones: an object with two
//! arrays (each a count word, top bit masked off, and a pointer), hover
//! regions (`+0x150`/`+0x154`, 0x150 bytes each, 32 integer rectangles)
//! and buttons (`+0x158`/`+0x15c`, 0x1c8 bytes each, 8 float
//! rectangles with press state). The update at `0x0068a3a0` resets the
//! buttons, then hands each pointer event to the hover test (moves) and to
//! the button handler.

#![allow(clippy::many_single_char_names)] // rectangle edges and points

use ffx_bridge::x87::{Reg, round_trip};

/// Rectangles per hover region.
pub const RECTS: usize = 32;
/// Buttons per button group.
pub const BUTTONS: usize = 8;

/// A pointer event (`+8` x, `+0xc` y, `+0x18` kind, `+0x1c` pointer id).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Event {
    /// 0, 4, 5: move; 1: press; 2: (state 3); 3: release.
    pub kind: u32,
    /// Screen position.
    pub x: f32,
    /// Screen position.
    pub y: f32,
    /// Which pointer.
    pub id: u8,
}

/// One button, 0x38 bytes (the last 4 untouched).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Button {
    /// `+0`: left, top, right, bottom.
    pub rect: [f32; 4],
    /// `+0x10`: this frame's event (1 hover, 2 press, 3, 4 release).
    pub state: u8,
    /// `+0x14`: the pointer that pressed it.
    pub id: u8,
    /// `+0x15`: a copy of `state`.
    pub state2: u8,
    /// `+0x16`: press phase (1 held, 2 dragged out, 3 back in, 4 off).
    pub phase: u8,
    /// `+0x17`: released this frame.
    pub released: u8,
    /// `+0x18`: when it was pressed (`graphicGetTime`).
    pub time: f32,
    /// `+0x1c`: release minus press position.
    pub delta: [f32; 2],
    /// `+0x24`: press position.
    pub start: [f32; 2],
    /// `+0x2c`: release position.
    pub end: [f32; 2],
}

/// Strictly inside `[l, r] x [t, b]`; false with any NaN.
fn inside(l: f32, t: f32, r: f32, b: f32, x: f32, y: f32) -> bool {
    l < x && x < r && t < y && y < b
}

impl Button {
    fn contains(&self, x: f32, y: f32) -> bool {
        let [l, t, r, b] = self.rect;
        inside(l, t, r, b, x, y)
    }

    /// The per-frame reset (`0x0068ab30`): event bytes cleared, a finished
    /// press (phase 3 or 4) forgotten, the press time zeroed when no press
    /// is on.
    pub fn reset(&mut self) {
        self.state = 0;
        self.state2 = 0;
        self.released = 0;
        if self.phase == 3 || self.phase == 4 {
            self.phase = 0;
        }
        if self.phase == 0 {
            self.time = 0.0;
        }
    }
}

/// `0x0068b470` for one group: first every button under the pointer takes
/// the event (a press records the pointer, position and `now()`), then
/// every button pressed by this pointer follows it (a move ends the press
/// unless still inside and held; a release records the end and the drag).
pub fn handle(buttons: &mut [Button; BUTTONS], ev: Event, mut now: impl FnMut() -> f32) {
    for b in buttons.iter_mut() {
        if !b.contains(ev.x, ev.y) {
            continue;
        }
        match ev.kind {
            0 | 4 | 5 => {
                b.state = 1;
                b.state2 = 1;
            }
            1 => {
                b.id = ev.id;
                b.phase = 1;
                b.start = [round_trip(ev.x), round_trip(ev.y)];
                b.state = 2;
                b.state2 = 2;
                b.time = now();
            }
            2 => {
                b.state = 3;
                b.state2 = 3;
            }
            _ => {}
        }
    }
    for b in buttons.iter_mut() {
        if b.id != ev.id {
            continue;
        }
        let inside = b.contains(ev.x, ev.y);
        match ev.kind {
            0 | 4 | 5 => b.phase = if inside && b.phase == 1 { 3 } else { 4 },
            3 => {
                if !inside && b.phase == 1 {
                    b.phase = 2;
                }
                b.state = 4;
                b.state2 = 4;
                b.released = 1;
                b.end = [round_trip(ev.x), round_trip(ev.y)];
                b.delta = [
                    Reg::load(b.end[0]).sub_m32(b.start[0]).store(),
                    Reg::load(b.end[1]).sub_m32(b.start[1]).store(),
                ];
            }
            _ => {}
        }
    }
}

/// `0x0068b6d0` for one region: the first of its rectangles (`[l, t, r,
/// b]`, integers) strictly containing the pointer, or -1.
#[must_use]
pub fn hover(rects: &[[i16; 4]; RECTS], x: f32, y: f32) -> i32 {
    rects
        .iter()
        .position(|&[l, t, r, b]| {
            let (x, y) = (f64::from(x), f64::from(y));
            f64::from(l) < x && x < f64::from(r) && f64::from(t) < y && y < f64::from(b)
        })
        .map_or(-1, |i| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // < 32
            let i = i as i32;
            i
        })
}

/// The originals, where the game keeps their state.
pub mod bridge {
    use ffx_bridge::Mem;
    use ffx_graphic::host::{Conv, Host};

    use super::{BUTTONS, Button, Event, RECTS};

    /// `graphicGetTime()`: seconds, in `ST0`.
    pub const GRAPHIC_GET_TIME: u32 = 0x0064_1410;

    /// Region and group counts mask off the top bit.
    fn count(mem: &impl Mem, at: u32) -> u32 {
        mem.read::<u32>(at) & 0x7fff_ffff
    }

    fn read_button(mem: &impl Mem, at: u32) -> Button {
        let f = |o: u32| mem.read::<f32>(at.wrapping_add(o));
        let b = |o: u32| mem.read::<u8>(at.wrapping_add(o));
        Button {
            rect: [f(0), f(4), f(8), f(0xc)],
            state: b(0x10),
            id: b(0x14),
            state2: b(0x15),
            phase: b(0x16),
            released: b(0x17),
            time: f(0x18),
            delta: [f(0x1c), f(0x20)],
            start: [f(0x24), f(0x28)],
            end: [f(0x2c), f(0x30)],
        }
    }

    /// Writes back the fields the originals write (a byte for byte
    /// rewrite of unchanged ones is harmless; the rectangle is never
    /// written).
    fn write_button(mem: &mut impl Mem, at: u32, b: &Button) {
        let at = |o: u32| at.wrapping_add(o);
        mem.write(at(0x10), b.state);
        mem.write(at(0x14), [b.id, b.state2, b.phase, b.released]);
        mem.write(at(0x18), [b.time, b.delta[0], b.delta[1]]);
        mem.write(at(0x24), [b.start[0], b.start[1], b.end[0], b.end[1]]);
    }

    /// `GuiTouch_initGroup(group)` (`0x0068a420`, fastcall): a button
    /// group (0x1c8 bytes) emptied: `+0` (`u16`), `+4` (byte), `+0x1c4`
    /// (`u32`) zero; each of the 8 buttons gets a zero rectangle, state,
    /// phase, press time and byte `+0x34`, and pointer `0xff` (the `u16`
    /// at `+0x14`, its copy of the state 0); the rest is left.
    pub fn init_group(mem: &mut impl Mem, group: u32) {
        let at = |o: u32| group.wrapping_add(o);
        mem.write(at(0x1c4), 0u32);
        mem.write(at(0), 0u16);
        mem.write(at(4), 0u8);
        for j in 0..8u32 {
            let b = |o: u32| at(8 + 0x38 * j + o);
            mem.write(b(0), [0f32; 4]);
            mem.write(b(0x10), 0u8);
            mem.write(b(0x14), 0xffu16);
            mem.write(b(0x16), 0u8);
            mem.write(b(0x18), 0f32);
            mem.write(b(0x34), 0u8);
        }
    }

    /// `GuiTouch_resetButtons(this)` (`0x0068ab30`, thiscall):
    /// [`Button::reset`] on every button of every active group.
    pub fn reset_buttons(mem: &mut impl Mem, this: u32) {
        let mut i = 0;
        while i < count(mem, this.wrapping_add(0x158)) {
            let g = mem
                .read::<u32>(this.wrapping_add(0x15c))
                .wrapping_add(i.wrapping_mul(0x1c8));
            if mem.read::<u8>(g.wrapping_add(1)) != 0 {
                for j in 0..8u32 {
                    let at = g.wrapping_add(8 + 0x38 * j);
                    let mut b = read_button(mem, at);
                    b.reset();
                    write_button(mem, at, &b);
                }
            }
            i += 1;
        }
    }

    /// `GuiTouch_handleEvent(this, event)` (`0x0068b470`, thiscall):
    /// [`super::handle`] on every active group; a press calls
    /// `graphicGetTime`.
    pub fn handle_event(mem: &mut impl Host, this: u32, event: u32) {
        let mut i = 0;
        while i < count(mem, this.wrapping_add(0x158)) {
            let g = mem
                .read::<u32>(this.wrapping_add(0x15c))
                .wrapping_add(i.wrapping_mul(0x1c8));
            if mem.read::<u8>(g.wrapping_add(1)) != 0 {
                let ev = Event {
                    kind: mem.read(event.wrapping_add(0x18)),
                    x: mem.read(event.wrapping_add(8)),
                    y: mem.read(event.wrapping_add(0xc)),
                    id: mem.read(event.wrapping_add(0x1c)),
                };
                let at = |j: usize| {
                    #[allow(clippy::cast_possible_truncation)] // j < 8
                    g.wrapping_add(8 + 0x38 * j as u32)
                };
                let mut buttons: [Button; BUTTONS] =
                    std::array::from_fn(|j| read_button(mem, at(j)));
                super::handle(&mut buttons, ev, || {
                    mem.call_sym_st0(GRAPHIC_GET_TIME, Conv::Cdecl, &[])
                });
                for (j, b) in buttons.iter().enumerate() {
                    write_button(mem, at(j), b);
                }
            }
            i += 1;
        }
    }

    /// `GuiTouch_hover(this, x, y)` (`0x0068b6d0`, thiscall): every active
    /// region (byte `+1`) gets [`super::hover`] at `+4`.
    pub fn hover(mem: &mut impl Mem, this: u32, x: f32, y: f32) {
        let mut i = 0;
        while i < count(mem, this.wrapping_add(0x150)) {
            let r = mem
                .read::<u32>(this.wrapping_add(0x154))
                .wrapping_add(i.wrapping_mul(0x150));
            if mem.read::<u8>(r.wrapping_add(1)) != 0 {
                let rects: [[i16; 4]; RECTS] = std::array::from_fn(|k| {
                    #[allow(clippy::cast_possible_truncation)] // k < 32
                    mem.read(r.wrapping_add(0xe + 10 * k as u32))
                });
                mem.write(r.wrapping_add(4), super::hover(&rects, x, y));
            }
            i += 1;
        }
    }
}
