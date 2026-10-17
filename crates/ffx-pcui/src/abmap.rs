//! The sphere grid screen's helpers (`g_Abmap`, the grid `LoadAbmap`
//! reads): zoom scale, node radius, range tests.

/// The map scale of zoom level `level` (`g_Abmap + 0x115cc`): 1, 1/2, 1/4,
/// 1/8; 1 for any other level.
#[must_use]
pub const fn zoom_scale(level: u8) -> f32 {
    match level {
        1 => 0.5,
        2 => 0.25,
        3 => 0.125,
        _ => 1.0,
    }
}

/// The drawn radius of a node whose kind has size `size`: half the size
/// (arithmetic shift) plus 3, rounded to f32 once.
#[must_use]
pub fn node_radius(size: i16) -> f32 {
    #[allow(clippy::cast_possible_truncation)] // the original's fstp dword
    let r = (f64::from(size >> 1) + 3.0) as f32;
    r
}

/// Whether `[a, b]` and `[c, d]` overlap, as `0x00a57000` decides it:
/// when `c` is past `a` it must not be past `b`; when `c` is before `a`,
/// `d` must reach `a`; equal or unordered starts overlap. A NaN end
/// against an ordered start fails.
#[must_use]
pub fn ranges_overlap(a: f32, b: f32, c: f32, d: f32) -> bool {
    if c > a {
        b >= c
    } else if c < a {
        d >= a
    } else {
        true
    }
}

/// The sphere grid's pad state (`0x00a57520`'s second argument): buttons
/// held now, newly pressed, held last frame, repeat and first-repeat
/// masks, and two counters per button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pad {
    /// `+0`.
    pub now: u16,
    /// `+2`: pressed this frame.
    pub pressed: u16,
    /// `+4`: held last frame.
    pub held: u16,
    /// `+6`: pressed or repeating this frame.
    pub repeat: u16,
    /// `+8`: pressed, or the first repeat of a cycle.
    pub first: u16,
    /// `+0xa`: frames held, up to 6.
    pub hold: [u8; 16],
    /// `+0x1a`: repeat cycle, 0..3.
    pub cycle: [u8; 16],
}

/// Folds this frame's buttons into the pad: `analog` (the left stick's
/// horizontal byte, `0x80` centred) adds 0x1000 left of `0x40` and 0x4000
/// right of `0xc0` when given; a button held 6 frames repeats every
/// frame, flagging `first` every third.
pub fn read_pad(pad: &mut Pad, buttons: u16, analog: Option<u8>) {
    pad.now = buttons;
    if let Some(a) = analog {
        #[allow(clippy::cast_possible_wrap)] // a signed byte
        let a = a.wrapping_add(0x80) as i8;
        if a < -0x40 {
            pad.now |= 0x1000;
        } else if a > 0x40 {
            pad.now |= 0x4000;
        }
    }
    pad.pressed = !pad.held & pad.now;
    pad.held = pad.now;
    pad.repeat = 0;
    pad.first = 0;
    for i in 0..16 {
        let m = 1u16 << i;
        if pad.pressed & m != 0 {
            pad.repeat |= m;
            pad.first |= m;
        } else if pad.now & m != 0 {
            pad.hold[i] = pad.hold[i].wrapping_add(1);
            if pad.hold[i] >= 6 {
                pad.hold[i] = 6;
                pad.repeat |= m;
                if pad.cycle[i] == 0 {
                    pad.first |= m;
                }
                pad.cycle[i] = pad.cycle[i].wrapping_add(1);
                if pad.cycle[i] >= 3 {
                    pad.cycle[i] = 0;
                }
            }
        } else {
            pad.hold[i] = 0;
            pad.cycle[i] = 0;
        }
    }
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};
    use ffx_graphic::host::{Conv, Host};

    use super::Pad;

    /// `g_Ee_Gpr`: the emulated R5900 registers; `r0` (always zero) first.
    pub const EE_GPR: Global<[u32; 4]> = Global::new(0x00c8_f508);
    /// `TOMenuGetControlPad2()`: the buttons held.
    pub const TO_MENU_GET_CONTROL_PAD2: u32 = 0x008b_e440;
    /// `TOMenuGetControlPadAnalogHist(pad, frame, axis, -1)`.
    pub const TO_MENU_GET_CONTROL_PAD_ANALOG_HIST: u32 = 0x008b_e450;

    /// `AbmapResetVectors()` (`0x00a57120`): the eight vectors at
    /// `g_Abmap + 0x11520..0x115a0` from `r0` (zero), the `w` of the first
    /// four then 1.0; returns `g_Abmap` (left in `eax`).
    pub fn reset_vectors(mem: &mut impl Mem) -> u32 {
        for off in [
            0x11560u32, 0x11570, 0x11580, 0x11590, 0x11520, 0x11530, 0x11540, 0x11550,
        ] {
            let g = ABMAP.get(mem);
            let r0 = EE_GPR.get(mem);
            mem.write(g.wrapping_add(off), r0);
        }
        for off in [0x1155cu32, 0x1154c, 0x1153c, 0x1152c] {
            let g = ABMAP.get(mem);
            mem.write(g.wrapping_add(off), 1.0f32);
        }
        ABMAP.get(mem)
    }

    /// `AbmapReadPad(no_stick, pad)` (`0x00a57520`): [`super::read_pad`]
    /// with `TOMenuGetControlPad2`, and the stick
    /// (`TOMenuGetControlPadAnalogHist(0, 0, 3, -1)`) when `no_stick` is
    /// 0; returns 0 (left in `eax`).
    pub fn read_pad(mem: &mut impl Host, no_stick: u32, pad: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // a 16-bit result
        let buttons = mem.call_sym(TO_MENU_GET_CONTROL_PAD2, Conv::Cdecl, &[]) as u16;
        let analog = (no_stick == 0).then(|| {
            #[allow(clippy::cast_possible_truncation)] // a byte result
            let a = mem.call_sym(
                TO_MENU_GET_CONTROL_PAD_ANALOG_HIST,
                Conv::Cdecl,
                &[0, 0, 3, u32::MAX],
            ) as u8;
            a
        });
        let at = |o: u32| pad.wrapping_add(o);
        let [now, pressed, held, repeat, first] = mem.read(at(0));
        let mut p = Pad {
            now,
            pressed,
            held,
            repeat,
            first,
            hold: mem.read(at(0xa)),
            cycle: mem.read(at(0x1a)),
        };
        super::read_pad(&mut p, buttons, analog);
        mem.write(at(0), [p.now, p.pressed, p.held, p.repeat, p.first]);
        mem.write(at(0xa), p.hold);
        mem.write(at(0x1a), p.cycle);
        0
    }

    /// `g_Abmap`: the sphere grid data.
    pub const ABMAP: Global<u32> = Global::new(0x0230_5834);

    /// `AbmapZoomScale()` (`0x00a56820`): [`super::zoom_scale`] of the
    /// current zoom level (`g_Abmap + 0x115cc`).
    pub fn zoom_scale(mem: &impl Mem) -> f32 {
        let g = ABMAP.get(mem);
        super::zoom_scale(mem.read(g.wrapping_add(0x115cc)))
    }

    /// `AbmapNodeRadius(node)` (`0x00a56f30`): [`super::node_radius`] of
    /// the size of node `node`'s kind (node `+6`, kind sizes at `+0xf834`,
    /// 0x30 each).
    pub fn node_radius(mem: &impl Mem, node: u32) -> f32 {
        let g = ABMAP.get(mem);
        let kind: u16 = mem.read(g.wrapping_add(node.wrapping_mul(0x28)).wrapping_add(0x80e));
        let size: i16 = mem.read(g.wrapping_add(u32::from(kind) * 0x30 + 0xf834));
        super::node_radius(size)
    }

    /// `AbmapRangesOverlap(a, b, c, d)` (`0x00a57000`): 1 or 0.
    #[must_use]
    pub fn ranges_overlap(a: f32, b: f32, c: f32, d: f32) -> u32 {
        u32::from(super::ranges_overlap(a, b, c, d))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap() {
        assert!(ranges_overlap(0.0, 2.0, 1.0, 3.0));
        assert!(!ranges_overlap(0.0, 0.5, 1.0, 3.0));
        assert!(ranges_overlap(1.0, 2.0, 0.0, 1.0));
        assert!(!ranges_overlap(1.0, 2.0, 0.0, 0.5));
        assert!(ranges_overlap(1.0, f32::NAN, 1.0, f32::NAN));
        assert!(ranges_overlap(f32::NAN, 0.0, 1.0, 0.0));
        assert!(!ranges_overlap(0.0, f32::NAN, 1.0, 3.0));
    }

    #[test]
    fn radius() {
        assert_eq!(node_radius(-3), 1.0);
        assert_eq!(node_radius(10), 8.0);
    }
}
