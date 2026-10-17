//! The sphere grid screen's list panels (`g_AbmapPad`: 0x340-byte panels,
//! the open one's index at `+0x2700`, its handler at `+0x2704`): the
//! cursor, the scrolling and the choice (`AbmapPadCtrl`, `0x00a583e0`).

/// The cursor sound.
pub const SFX_MOVE: u32 = 0x8000_0001;
/// The refusal buzz.
pub const SFX_BUZZ: u32 = 0x8000_0003;
/// The cancel sound.
pub const SFX_CANCEL: u32 = 0x8000_0004;

/// A list panel's scroll state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct List {
    /// `+0x16`: rows shown.
    pub page: i16,
    /// `+0x18`: the cursor's entry.
    pub cursor: i16,
    /// `+0x1a`: the first entry shown.
    pub top: i16,
    /// `+0x1c`: the first entry shown before a page flip.
    pub prev_top: i16,
    /// `+0x1e`: entries.
    pub count: i16,
    /// `+0x26`: the scroll animation (1 up, 2 down, 3 page up, 4 page
    /// down); the panel takes no input while it is set.
    pub anim: u16,
}

/// Moves the cursor of `l` for the repeating buttons `buttons`: when the
/// list is longer than a page, bit 0 flips a page up (while not at the
/// top), else bit 1 a page down (while a page fits below) and the frame
/// ends there (`true`); otherwise up (`0x1000`) or down (`0x4000`) one
/// entry, scrolling to keep it shown. Cursor sounds go to `play`, only
/// when the cursor can move. All arithmetic on the `i16` fields wraps.
pub fn scroll(l: &mut List, buttons: u16, play: &mut impl FnMut(u32)) -> bool {
    let count = l.count;
    if count > l.page {
        if buttons & 1 != 0 {
            if l.top > 0 {
                play(SFX_MOVE);
                l.prev_top = l.top;
                l.top = l.top.wrapping_sub(l.page).max(0);
                l.cursor = l.cursor.wrapping_add(l.top.wrapping_sub(l.prev_top));
                l.anim = 3;
                return true;
            }
        } else if buttons & 2 != 0 && i32::from(l.top) < i32::from(count) - i32::from(l.page) {
            play(SFX_MOVE);
            l.prev_top = l.top;
            let top = l.top.wrapping_add(l.page);
            l.top = if i32::from(top) >= i32::from(l.count) - i32::from(l.page) {
                l.count.wrapping_sub(l.page)
            } else {
                top
            };
            l.cursor = l.cursor.wrapping_add(l.top.wrapping_sub(l.prev_top));
            l.anim = 4;
            return true;
        }
    }
    if buttons & 0x1000 != 0 {
        if l.cursor > 0 {
            play(SFX_MOVE);
        }
        l.cursor = l.cursor.wrapping_sub(1).max(0);
        if l.top > l.cursor {
            l.top = l.cursor;
            l.anim = 1;
        }
    } else if buttons & 0x4000 != 0 {
        if i32::from(l.cursor) < i32::from(count) - 1 {
            play(SFX_MOVE);
        }
        l.cursor = l.cursor.wrapping_add(1);
        if l.cursor >= l.count {
            l.cursor = l.count.wrapping_sub(1);
        }
        if i32::from(l.top) + i32::from(l.page) <= i32::from(l.cursor) {
            l.top = l.cursor.wrapping_sub(l.page).wrapping_add(1);
            l.anim = 2;
        }
    }
    false
}

/// What the pressed buttons do to the panel after the cursor moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Nothing pressed.
    Nothing,
    /// The buzz: a locked entry chosen, or cancel on a panel that has no
    /// cancel.
    Refuse,
    /// Cancel on a panel whose cancel puts the cursor on entry 1.
    FirstEntry,
    /// The panel closes and its handler gets the entry: chosen
    /// (`confirmed`) or cancelled, after `sound` when there is one.
    Close { sound: Option<u32>, confirmed: bool },
}

/// The choice for the pressed buttons `edge` (`0x20` choose, `0x40`
/// cancel) on a panel with flags `flags` (`+0x24`: 8 silent choice, 4
/// cancel to entry 1, 2 cancel closes) whose cursor entry is `locked`.
#[must_use]
pub fn choice(edge: u16, flags: u8, locked: bool) -> Choice {
    if edge & 0x20 != 0 {
        if locked {
            Choice::Refuse
        } else {
            Choice::Close {
                sound: (flags & 8 == 0).then_some(SFX_MOVE),
                confirmed: true,
            }
        }
    } else if edge & 0x40 == 0 {
        Choice::Nothing
    } else if flags & 6 == 0 {
        Choice::Refuse
    } else if flags & 4 != 0 {
        Choice::FirstEntry
    } else {
        Choice::Close {
            sound: Some(SFX_CANCEL),
            confirmed: false,
        }
    }
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Global;
    use ffx_graphic::host::{Conv, Host};

    use super::{Choice, List};
    use crate::abmap::bridge::{ABMAP, SND_SEP_PLAY_SIMPLE};

    /// `g_AbmapPad`: the panels (0x340 bytes each), `+0x2700` the open
    /// one (`i16`, negative for none), `+0x2704` its handler.
    pub const PAD: Global<u32> = Global::new(0x01a8_6108);

    /// The list fields at `+0x16..+0x20`, then the animation at `+0x26`.
    fn read_list(mem: &impl Host, p: u32) -> List {
        let w = |o: u32| mem.read::<i16>(p.wrapping_add(o));
        List {
            page: w(0x16),
            cursor: w(0x18),
            top: w(0x1a),
            prev_top: w(0x1c),
            count: w(0x1e),
            anim: mem.read(p.wrapping_add(0x26)),
        }
    }

    /// Writes the fields of `l` that differ from `was`.
    fn write_list(mem: &mut impl Host, p: u32, was: &List, l: &List) {
        for (o, a, b) in [
            (0x18, was.cursor, l.cursor),
            (0x1a, was.top, l.top),
            (0x1c, was.prev_top, l.prev_top),
        ] {
            if a != b {
                mem.write(p.wrapping_add(o), b);
            }
        }
        if was.anim != l.anim {
            mem.write(p.wrapping_add(0x26), l.anim);
        }
    }

    /// `AbmapPadCtrl()` (`0x00a583e0`): the open panel (none when
    /// `+0x2700` is negative), unless it is animating (byte `+0x26`): the
    /// cursor and scrolling ([`super::scroll`]) for the repeating buttons
    /// at `g_Abmap + 0x11674`, then the choice ([`super::choice`]) for the
    /// pressed ones at `g_Abmap + 0x1166e` on the cursor's entry (12 bytes
    /// at `+0x40`, locked when byte `+4` is set). A closing panel is
    /// hidden (`+0x23`), none is open, and the handler gets
    /// `(panel, entry, cursor, confirmed)`. Sounds through
    /// `SndSepPlaySimple`. No caller reads `eax`.
    pub fn pad_ctrl(mem: &mut impl Host) {
        let pad = PAD.get(mem);
        let open: i16 = mem.read(pad.wrapping_add(0x2700));
        if open < 0 {
            return;
        }
        let p = pad.wrapping_add(i32::from(open).cast_unsigned().wrapping_mul(0x340));
        if mem.read::<u8>(p.wrapping_add(0x26)) != 0 {
            return;
        }
        let buttons: u16 = mem.read(ABMAP.get(mem).wrapping_add(0x1_1674));
        let was = read_list(mem, p);
        let mut l = was;
        let mut sounds = [0u32; 2];
        let mut n = 0;
        let flipped = super::scroll(&mut l, buttons, &mut |s| {
            sounds[n] = s;
            n += 1;
        });
        for &s in &sounds[..n] {
            mem.call_sym(SND_SEP_PLAY_SIMPLE, Conv::Cdecl, &[s]);
        }
        write_list(mem, p, &was, &l);
        if flipped {
            return;
        }
        let edge: u16 = mem.read(ABMAP.get(mem).wrapping_add(0x1_166e));
        let entry = p
            .wrapping_add(0x40)
            .wrapping_add(i32::from(l.cursor).cast_unsigned().wrapping_mul(12));
        let flags: u8 = mem.read(p.wrapping_add(0x24));
        let locked = mem.read::<u8>(entry.wrapping_add(4)) != 0;
        match super::choice(edge, flags, locked) {
            Choice::Nothing => {}
            Choice::Refuse => {
                mem.call_sym(SND_SEP_PLAY_SIMPLE, Conv::Cdecl, &[super::SFX_BUZZ]);
            }
            Choice::FirstEntry => {
                mem.call_sym(SND_SEP_PLAY_SIMPLE, Conv::Cdecl, &[super::SFX_MOVE]);
                mem.write(p.wrapping_add(0x18), 1u16);
            }
            Choice::Close { sound, confirmed } => {
                if let Some(s) = sound {
                    mem.call_sym(SND_SEP_PLAY_SIMPLE, Conv::Cdecl, &[s]);
                }
                let pad = PAD.get(mem);
                mem.write(p.wrapping_add(0x23), 0u8);
                mem.write(pad.wrapping_add(0x2700), 0xffffu16);
                let cursor: i16 = mem.read(p.wrapping_add(0x18));
                let handler: u32 = mem.read(pad.wrapping_add(0x2704));
                let args = [
                    p,
                    entry,
                    i32::from(cursor).cast_unsigned(),
                    u32::from(confirmed),
                ];
                mem.call(handler, Conv::Cdecl, &args);
            }
        }
    }
}
