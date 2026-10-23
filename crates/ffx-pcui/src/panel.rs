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
    use crate::abmap::bridge::{
        ABMAP, ABMAP_INIT_HOMING, ABMAP_RESET_CURSOR_CAMERA, ABMAP_STATE_HOME_ON_NODE,
        SND_SEP_PLAY_SIMPLE,
    };

    /// `AbmapResetHelp()`.
    pub const ABMAP_RESET_HELP: u32 = 0x00a5_b930;
    /// `spendSlv(character, levels)`: spends sphere levels.
    pub const SPEND_SLV: u32 = 0x0078_6ef0;
    /// `AbmapSetChrMarker(character)`.
    pub const ABMAP_SET_CHR_MARKER: u32 = 0x00a5_a990;
    /// `AbmapPositionPlyTag(character)`.
    pub const ABMAP_POSITION_PLY_TAG: u32 = 0x00a5_8080;

    /// `g_AbmapPad`: the panels (0x340 bytes each), `+0x2700` the open
    /// one (`i16`, negative for none), `+0x2704` its handler.
    pub const PAD: Global<u32> = Global::new(0x01a8_6108);
    /// `g_AbmapRunning`: 1 while the sphere grid runs (`eiAbmCalc` sets
    /// it).
    pub const RUNNING: Global<u32> = Global::new(0x01a8_5f70);
    /// `g_AbmapCjkText`: 1 for Japanese, Korean and Chinese text (set by
    /// `eiAbmStart`).
    pub const CJK_TEXT: Global<u32> = Global::new(0x01a8_5f74);
    /// Where a menu frame's rectangle tweens in from: `i16` x 0x30, y
    /// 0x164 (one word; no relocation, not an address, though Ghidra
    /// shows it as `&DAT_01640030`).
    pub const RECT_ORIGIN: u32 = 0x0164_0030;
    /// `AbmapStoreSave()`.
    pub const ABMAP_STORE_SAVE: u32 = 0x00a5_bb70;
    /// `eiAbmParaGet()`.
    pub const EI_ABM_PARA_GET: u32 = 0x00a5_4860;
    /// `TOMenuTermAbilityMap2()`: back to the main menu.
    pub const TO_MENU_TERM_ABILITY_MAP2: u32 = 0x008e_27e0;
    /// `AbmapPadSlideInStep(panel)`: a panel's step that slides it in.
    pub const ABMAP_PAD_SLIDE_IN_STEP: u32 = 0x00a5_a270;
    /// `AbmapRectTweenIn(rect)`: a menu frame's step that grows it.
    pub const ABMAP_RECT_TWEEN_IN: u32 = 0x00a5_a080;
    /// `AbmapShowItemTargets(chr, node, item)`.
    pub const ABMAP_SHOW_ITEM_TARGETS: u32 = 0x00a5_b980;
    /// `AbmapEnterCommandMenu()`.
    pub const ABMAP_ENTER_COMMAND_MENU: u32 = 0x00a5_9950;
    /// `AbmapEnterItemMenu()`.
    pub const ABMAP_ENTER_ITEM_MENU: u32 = 0x00a5_98a0;
    /// `AbmapInitChoosingMoveTarget()`.
    pub const ABMAP_INIT_CHOOSING_MOVE_TARGET: u32 = 0x00a5_b7b0;

    /// `AbmapPadQuitHandler(panel, entry, choice, confirmed)`
    /// (`0x00a56060`): the handler of the panel asking whether to leave
    /// the sphere grid. Byte `g_AbmapPad + 0x20a3` cleared first. Choice 0
    /// confirmed leaves: `AbmapStoreSave`, `eiAbmParaGet`,
    /// `TOMenuTermAbilityMap2`, `g_AbmapRunning` = 0; `eax` is the last
    /// call's. Anything else: the cancel sound, and panel 6 (`+0x1380`)
    /// slides back in (`+0x29` = 0, step `+0x34` = `AbmapPadSlideInStep`)
    /// when it is hidden (`+0x23` 0) or has no step; `eax` is
    /// `g_AbmapPad`.
    pub fn quit_handler(mem: &mut impl Host, choice: u32, confirmed: u32) -> u32 {
        let pad = PAD.get(mem);
        mem.write(pad.wrapping_add(0x20a3), 0u8);
        if confirmed != 0 && choice == 0 {
            mem.call_sym(ABMAP_STORE_SAVE, Conv::Cdecl, &[]);
            mem.call_sym(EI_ABM_PARA_GET, Conv::Cdecl, &[]);
            let eax = mem.call_sym(TO_MENU_TERM_ABILITY_MAP2, Conv::Cdecl, &[]);
            RUNNING.set(mem, 0);
            return eax;
        }
        mem.call_sym(SND_SEP_PLAY_SIMPLE, Conv::Cdecl, &[super::SFX_CANCEL]);
        let pad = PAD.get(mem);
        let shown: u8 = mem.read(pad.wrapping_add(0x13a3));
        let step: u32 = mem.read(pad.wrapping_add(0x13b4));
        if shown == 0 || step != 0 {
            mem.write(pad.wrapping_add(0x13a9), 0u8);
            let f = mem.rebase(ABMAP_PAD_SLIDE_IN_STEP);
            mem.write(pad.wrapping_add(0x13b4), f);
        }
        pad
    }

    /// Starts the frame tween of the menu frame at `frame`: shown (`+0`),
    /// from [`RECT_ORIGIN`] (`+7`) to `size` (`i16` w, h at `+0xb`), step
    /// 0 (`+0xf`), `AbmapRectTweenIn` (`+0x11`).
    fn tween_frame_in(mem: &mut impl Host, frame: u32, size: u32) {
        mem.write(frame, 1u8);
        mem.write(frame.wrapping_add(7), RECT_ORIGIN);
        mem.write(frame.wrapping_add(0xb), size);
        mem.write(frame.wrapping_add(0xf), 0u8);
        let f = mem.rebase(ABMAP_RECT_TWEEN_IN);
        mem.write(frame.wrapping_add(0x11), f);
    }

    /// `AbmapPadItemHandler(panel, entry, choice, confirmed)`
    /// (`0x00a560d0`): the item menu's handler (`AbmapEnterItemMenu`).
    /// Cancelled: a tail call to `AbmapEnterCommandMenu`. Chosen: the
    /// frame at `g_AbmapPad + 0x1a23` tweens in to `0x140090` with CJK
    /// text (`g_AbmapCjkText`), else `0x1400b0`, then
    /// `AbmapShowItemTargets(chr +0x115bc, its node u16 +0x110cc + 0x50
    /// chr, item u16 entry +8)`; `eax` is the callee's.
    pub fn item_handler(mem: &mut impl Host, entry: u32, confirmed: u32) -> u32 {
        if confirmed == 0 {
            return mem.call_sym(ABMAP_ENTER_COMMAND_MENU, Conv::Cdecl, &[]);
        }
        let cjk = CJK_TEXT.get(mem) != 0;
        let pad = PAD.get(mem);
        let size = if cjk { 0x0014_0090 } else { 0x0014_00b0 };
        tween_frame_in(mem, pad.wrapping_add(0x1a23), size);
        let g = ABMAP.get(mem);
        let chr: u8 = mem.read(g.wrapping_add(0x1_15bc));
        let node: u16 = mem.read(g.wrapping_add(0x1_10cc).wrapping_add(u32::from(chr) * 0x50));
        let item: u16 = mem.read(entry.wrapping_add(8));
        mem.call_sym(
            ABMAP_SHOW_ITEM_TARGETS,
            Conv::Cdecl,
            &[u32::from(chr), u32::from(node), u32::from(item)],
        )
    }

    /// `AbmapPadCommandHandler(panel, entry, choice, confirmed)`
    /// (`0x00a56330`): the command menu's handler (`AbmapEnterCommandMenu`).
    /// Cancelled: a tail call to `AbmapResetHelp`. Choice 0 (move): sound
    /// `0x80000051`, `AbmapInitChoosingMoveTarget`, and the frame at
    /// `g_AbmapPad + 0x16e3` tweens in to `0x140050`; `eax` is
    /// `g_AbmapPad`. Choice 1 (items): the cursor sound and a tail call to
    /// `AbmapEnterItemMenu`. Other choices do nothing and return the
    /// choice.
    pub fn command_handler(mem: &mut impl Host, choice: u32, confirmed: u32) -> u32 {
        if confirmed == 0 {
            return mem.call_sym(ABMAP_RESET_HELP, Conv::Cdecl, &[]);
        }
        match choice {
            0 => {
                mem.call_sym(SND_SEP_PLAY_SIMPLE, Conv::Cdecl, &[0x8000_0051]);
                mem.call_sym(ABMAP_INIT_CHOOSING_MOVE_TARGET, Conv::Cdecl, &[]);
                let pad = PAD.get(mem);
                tween_frame_in(mem, pad.wrapping_add(0x16e3), 0x0014_0050);
                pad
            }
            1 => {
                mem.call_sym(SND_SEP_PLAY_SIMPLE, Conv::Cdecl, &[super::SFX_MOVE]);
                mem.call_sym(ABMAP_ENTER_ITEM_MENU, Conv::Cdecl, &[])
            }
            _ => choice,
        }
    }

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

    /// `AbmapPadActivateHandler(panel, entry, choice, confirmed)`
    /// (`0x00a56160`): the handler of the panel asking whether to keep
    /// the nodes just activated. `AbmapResetHelp` first. Choice 0 keeps
    /// them: a sound (`0x80000052`) and `spendSlv(character +0x115bc,
    /// levels +0x1161c)`. Choice 1 undoes them: the cancel sound, the
    /// character's bit cleared from every link (0x14 bytes from
    /// `g_Abmap + 0xa814`, `i16` count at `+4`) pending (byte `+2` bit 3,
    /// `+0x116ac` = 1 for each), the character back on its node (`u16`
    /// `+0x11630` into `+0x110cc + 0x50 * c`, `c` = byte `+0x11638`): its
    /// marker, its name tag, `+0x115c7` = 1, the cursor camera reset, the
    /// cursor homing on that node at 0.5, `AbmapState_HomeOnNode` pushed
    /// unless a state is stacked (`+0x115b0`). Then every link's pending
    /// nibble (byte `+2`, low 4 bits) cleared, `g_AbmapPad + 0x1d63` = 0,
    /// three words at `+0x16c0` and one at `+0x16d0` copied 8 and 4 bytes
    /// on, the levels zeroed. Returns `g_Abmap` (`eax`). A negative link
    /// count would loop through memory as the original does.
    pub fn activate_handler(mem: &mut impl Host, choice: u32) -> u32 {
        mem.call_sym(ABMAP_RESET_HELP, Conv::Cdecl, &[]);
        if choice == 0 {
            mem.call_sym(SND_SEP_PLAY_SIMPLE, Conv::Cdecl, &[0x8000_0052]);
            let g = ABMAP.get(mem);
            let chr: u8 = mem.read(g.wrapping_add(0x1_15bc));
            let levels: u32 = mem.read(g.wrapping_add(0x1_161c));
            mem.call_sym(SPEND_SLV, Conv::Cdecl, &[u32::from(chr), levels]);
        } else if choice == 1 {
            let g = ABMAP.get(mem);
            let links = i32::from(mem.read::<i16>(g.wrapping_add(4))).cast_unsigned();
            mem.call_sym(SND_SEP_PLAY_SIMPLE, Conv::Cdecl, &[super::SFX_CANCEL]);
            let mut link = g.wrapping_add(0xa814);
            let mut left = links;
            while left != 0 {
                left = left.wrapping_sub(1);
                if mem.read::<u8>(link.wrapping_add(2)) & 8 != 0 {
                    let chr: u8 = mem.read(ABMAP.get(mem).wrapping_add(0x1_15bc));
                    // `shl dl, cl`: the count masked to 5 bits, 8-bit result
                    let bit = if chr & 0x1f < 8 {
                        1u8 << (chr & 0x1f)
                    } else {
                        0
                    };
                    let v: u8 = mem.read(link);
                    mem.write(link, v & !bit);
                    let g = ABMAP.get(mem);
                    mem.write(g.wrapping_add(0x1_16ac), 1u32);
                }
                link = link.wrapping_add(0x14);
            }
            let g = ABMAP.get(mem);
            let chr: u8 = mem.read(g.wrapping_add(0x1_1638));
            let slot = g.wrapping_add(0x1_10cc).wrapping_add(u32::from(chr) * 0x50);
            let node: u16 = mem.read(g.wrapping_add(0x1_1630));
            mem.write(slot, node);
            for f in [ABMAP_SET_CHR_MARKER, ABMAP_POSITION_PLY_TAG] {
                let c: u8 = mem.read(ABMAP.get(mem).wrapping_add(0x1_1638));
                mem.call_sym(f, Conv::Cdecl, &[u32::from(c)]);
            }
            let g2 = ABMAP.get(mem);
            mem.write(g2.wrapping_add(0x1_15c7), 1u8);
            mem.call_sym(ABMAP_RESET_CURSOR_CAMERA, Conv::Cdecl, &[]);
            let node: u16 = mem.read(slot);
            mem.call_sym(
                ABMAP_INIT_HOMING,
                Conv::Cdecl,
                &[u32::from(node), 0.5f32.to_bits()],
            );
            let g = ABMAP.get(mem);
            if mem.read::<u32>(g.wrapping_add(0x1_15b0)) == 0 {
                let state: u32 = mem.read(g.wrapping_add(0x1_15a8));
                mem.write(g.wrapping_add(0x1_15b0), state);
                let home = mem.rebase(ABMAP_STATE_HOME_ON_NODE);
                let g = ABMAP.get(mem);
                mem.write(g.wrapping_add(0x1_15a8), home);
            }
        }
        let g = ABMAP.get(mem);
        let links = i32::from(mem.read::<i16>(g.wrapping_add(4))).cast_unsigned();
        let mut at = g.wrapping_add(0xa816);
        for _ in 0..links {
            let v: u8 = mem.read(at);
            mem.write(at, v & 0xf0);
            at = at.wrapping_add(0x14);
        }
        let pad = PAD.get(mem);
        mem.write(pad.wrapping_add(0x1d63), 0u8);
        for (to, from) in [
            (0x16c8, 0x16c0),
            (0x16ca, 0x16c2),
            (0x16cc, 0x16c4),
            (0x16ce, 0x16c6),
            (0x16d4, 0x16d0),
            (0x16d6, 0x16d2),
        ] {
            let w: u16 = mem.read(pad.wrapping_add(from));
            mem.write(pad.wrapping_add(to), w);
        }
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x1_161c), 0u32);
        g
    }

    /// `FUN_00a462f0(x, y, right, bottom, count)` (unnamed; by its
    /// arguments the panel's frame), `TODrawScissorXYWH(x, y, w, h)`,
    /// `AbmapDrawScrollBar(x, y, height, first, total, shown)`,
    /// `FUN_00a4f290(panel, first, dx)` (unnamed; drawn when `+0x25` is
    /// set, the cursor by the look of it).
    pub const ABMAP_PAD_DRAW_FRAME: u32 = 0x00a4_62f0;
    pub const TO_DRAW_SCISSOR_XYWH: u32 = 0x008f_9230;
    pub const ABMAP_DRAW_SCROLL_BAR: u32 = 0x00a4_6120;
    pub const ABMAP_PAD_DRAW_CURSOR: u32 = 0x00a4_f290;
    /// The panel whose frame shows its entry count (`g_AbmapPad +
    /// 0x1a00`, panel 8).
    pub const COUNTED_PANEL: u32 = 0x1a00;
    /// A list row's height.
    pub const ROW: i32 = 0x14;

    /// `AbmapPadDrawList_00a58900(panel, dx)` (name ours; from the panel
    /// pass `0x00a58660`): the panel's frame at `x = i16 +8 + dx`, `y =
    /// i16 +0xa`, size `i16 +0xc` x `i16 +0xe` (the count `i16 +0x1e` for
    /// panel 8 only), its scissor (`y - 1`, `h - 2`). The first row shown
    /// is the top `+0x1a`, or `cursor - page + 1` once the cursor `+0x18`
    /// is a page `+0x16` or more below it. With `+0x28` set and more than
    /// two rows a page, the scroll bar (x, y, height as `u16`). Rows are
    /// drawn through the panel's callback `+0x38` (re-read for each),
    /// `cb(panel, entry, index, x, y)`, entries 12 bytes from `+0x40`,
    /// rows [`ROW`] apart, columns `+0x22` (`i16 +0xc / cols` apart, from
    /// `x + i16 +0x20`), `cols * page` cells; by the scroll animation
    /// `+0x26` (`+0x27` its step): 0 still (stops at the count, re-read),
    /// 1 one row up, 2 one row down (`page + 1` rows offset by `step / 8`,
    /// the step then grows by 0x35 and the animation ends at 0x8c), 3 a
    /// page up, 4 a page down (the step grows by 1, the animation ends at
    /// 8; the old page from `+0x1c` slides out of a shrinking scissor while
    /// the new one follows). Then the full-screen scissor and, with `+0x25`
    /// set, the cursor. A panel with no columns faults on the division in
    /// the original; the port draws its rows at the same x.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)] // five animations; x, y, p
    pub fn draw_list<M, C>(mem: &mut M, panel: u32, dx: i32, call: &mut C)
    where
        M: ffx_bridge::Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let p = panel;
        let x = word(mem, p, 8).wrapping_add(dx);
        let y = word(mem, p, 0xa);
        let counted = p == PAD.get(mem).wrapping_add(COUNTED_PANEL);
        let count = if counted { word(mem, p, 0x1e) } else { 0 };
        let right = word(mem, p, 0xc).wrapping_add(x);
        let bottom = word(mem, p, 0xe).wrapping_add(y);
        cdecl(
            mem,
            call,
            ABMAP_PAD_DRAW_FRAME,
            &[x, y, right, bottom, count],
        );
        scissor(mem, call, p, x);
        let top = word(mem, p, 0x1a);
        let cursor = word(mem, p, 0x18);
        let page = word(mem, p, 0x16);
        let first = if cursor - top >= page {
            cursor - page + 1
        } else {
            top
        };
        if byte(mem, p, 0x28) != 0 && page > 2 {
            // x, y and height as the u16 the original computes them in
            let w16 = |v: i32| i32::from(v.to_le_bytes()[0]) | i32::from(v.to_le_bytes()[1]) << 8;
            let bar = [
                w16(word(mem, p, 0xc) - 8 + x),
                w16(word(mem, p, 0xa) - 3),
                w16(word(mem, p, 0xe) + 4),
                word(mem, p, 0x1a),
                word(mem, p, 0x1e),
                page,
            ];
            cdecl(mem, call, ABMAP_DRAW_SCROLL_BAR, &bar);
        }
        let cols = i32::from(byte(mem, p, 0x22));
        let col_w = word(mem, p, 0xc).checked_div(cols).unwrap_or(0);
        let x0 = word(mem, p, 0x20).wrapping_add(x);
        let row_y = word(mem, p, 0xa);
        let cells = cols * word(mem, p, 0x16);
        match byte(mem, p, 0x26) {
            0 => {
                let (mut cx, mut cy, mut col) = (x0, row_y, 0);
                for k in 0..cells {
                    let index = first + k;
                    if index >= word(mem, p, 0x1e) {
                        break;
                    }
                    draw_row(mem, call, p, index, cx, cy);
                    cx = cx.wrapping_add(col_w);
                    col += 1;
                    if col >= i32::from(byte(mem, p, 0x22)) {
                        col = 0;
                        cx = x0;
                        cy = cy.wrapping_add(ROW);
                    }
                }
            }
            anim @ (1 | 2) => {
                let shift = i32::from(byte(mem, p, 0x27) >> 3);
                // up: from the first row, `step / 8 - 20` above; down: from
                // the row before it, `step / 8` above
                let (from, mut cy) = if anim == 1 {
                    (first, shift - ROW + row_y)
                } else {
                    (first - 1, row_y - shift)
                };
                let mut j = 0;
                while j < word(mem, p, 0x16) + 1 {
                    draw_row(mem, call, p, from + j, x0, cy);
                    cy = cy.wrapping_add(ROW);
                    j += 1;
                }
                let step = byte(mem, p, 0x27).wrapping_add(0x35);
                mem.write(p.wrapping_add(0x27), step);
                if step >= 0x8c {
                    mem.write(p.wrapping_add(0x26), 0u8);
                }
            }
            anim @ (3 | 4) => {
                let old = word(mem, p, 0x1c);
                let step = byte(mem, p, 0x27).wrapping_add(1);
                mem.write(p.wrapping_add(0x27), step);
                if step >= 8 {
                    mem.write(p.wrapping_add(0x26), 0u8);
                }
                let h = word(mem, p, 0xe);
                // C division toward zero
                let off = i32::from(step) * h / 8;
                let w = word(mem, p, 0xc);
                let old_rect = if anim == 3 {
                    [x, row_y - 1 + off, w, h - off - 2]
                } else {
                    [x, row_y - 1, w, h - off - 2]
                };
                cdecl(mem, call, TO_DRAW_SCISSOR_XYWH, &old_rect);
                if byte(mem, p, 0x26) != 0 {
                    for k in 0..cells {
                        draw_row(mem, call, p, old + k, x0, row_y + ROW * k);
                    }
                }
                scissor(mem, call, p, x);
                let new_y = if anim == 3 {
                    word(mem, p, 0xa) - word(mem, p, 0xe) + off
                } else {
                    word(mem, p, 0xa) + (h - off)
                };
                for k in 0..cells {
                    draw_row(mem, call, p, first + k, x0, new_y + ROW * k);
                }
            }
            _ => {}
        }
        cdecl(mem, call, TO_DRAW_SCISSOR_XYWH, &[0, 0, 0x200, 0x1a0]);
        if byte(mem, p, 0x25) != 0 {
            cdecl(
                mem,
                call,
                ABMAP_PAD_DRAW_CURSOR,
                &[p.cast_signed(), first, dx],
            );
        }
    }

    /// The `i16` at `p + off`, widened.
    fn word(mem: &impl ffx_bridge::Mem, p: u32, off: u32) -> i32 {
        i32::from(mem.read::<i16>(p.wrapping_add(off)))
    }

    fn byte(mem: &impl ffx_bridge::Mem, p: u32, off: u32) -> u8 {
        mem.read(p.wrapping_add(off))
    }

    /// The original at `sym` through `call`, the words as `i32`.
    fn cdecl<M, C>(mem: &mut M, call: &mut C, sym: u32, args: &[i32]) -> u32
    where
        M: ffx_bridge::Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let f = mem.rebase(sym);
        let words: Vec<u32> = args.iter().map(|v| v.cast_unsigned()).collect();
        call(mem, f, &words)
    }

    /// The panel's own scissor: from `y - 1`, `h - 2` high (`+0xa`,
    /// `+0xc`, `+0xe` read now).
    fn scissor<M, C>(mem: &mut M, call: &mut C, p: u32, x: i32)
    where
        M: ffx_bridge::Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let rect = [
            x,
            word(mem, p, 0xa) - 1,
            word(mem, p, 0xc),
            word(mem, p, 0xe) - 2,
        ];
        cdecl(mem, call, TO_DRAW_SCISSOR_XYWH, &rect);
    }

    /// Row `index` (entry `+0x40 + 12 index`) through the panel's
    /// callback `+0x38`, read now: `cb(panel, entry, index, x, y)`.
    fn draw_row<M, C>(mem: &mut M, call: &mut C, p: u32, index: i32, x: i32, y: i32)
    where
        M: ffx_bridge::Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let entry = p
            .wrapping_add(0x40)
            .wrapping_add(index.cast_unsigned().wrapping_mul(0xc));
        let cb: u32 = mem.read(p.wrapping_add(0x38));
        call(
            mem,
            cb,
            &[
                p,
                entry,
                index.cast_unsigned(),
                x.cast_unsigned(),
                y.cast_unsigned(),
            ],
        );
    }

    /// `MsGetSaveConfigHiragana()`, `MsMenuGetHelp(i, id, second)`,
    /// `AbmapHelpMessageStep_00a59350(text)`, `MsGetRomItem(id, &strings)`,
    /// `MsGetExcelData(id, table, &strings)` and the table word
    /// `0x01a860e4` the item help reads.
    pub const MS_GET_SAVE_CONFIG_HIRAGANA: u32 = 0x0078_51f0;
    pub const MS_MENU_GET_HELP: u32 = 0x0078_fbb0;
    pub const ABMAP_HELP_MESSAGE_STEP: u32 = 0x00a5_9350;
    pub const MS_GET_ROM_ITEM: u32 = 0x0079_09f0;
    pub const MS_GET_EXCEL_DATA: u32 = 0x007a_b870;
    pub const ITEM_HELP_TABLE: Global<u32> = Global::new(0x01a8_60e4);

    /// `AbmapPadMenuHelp_00a570a0(panel, entry)` (name ours; the help
    /// callback `+0x3c` the grid's start `0x00a54e3a` gives its menu
    /// panels): the help of menu text `entry + 8` (`MsMenuGetHelp(9, id,
    /// MsGetSaveConfigHiragana() & 1)`) shown through
    /// `AbmapHelpMessageStep_00a59350`, whose `eax` it returns.
    pub fn menu_help(mem: &mut impl Host, entry: u32) -> u32 {
        let id: u32 = mem.read(entry.wrapping_add(8));
        let kana = mem.call_sym(MS_GET_SAVE_CONFIG_HIRAGANA, Conv::Cdecl, &[]) & 1;
        let text = mem.call_sym(MS_MENU_GET_HELP, Conv::Cdecl, &[9, id, kana]);
        mem.call_sym(ABMAP_HELP_MESSAGE_STEP, Conv::Cdecl, &[text])
    }

    /// `AbmapPadItemHelp_00a57040(panel, entry)` (name ours; the help
    /// callback of the item panel, `g_AbmapPad + 0x1a3c`): the item
    /// `u16 entry + 8` (`MsGetRomItem`, its strings word into `strings`,
    /// which is the original's `entry` argument slot, so it starts as
    /// `entry`); with a help id
    /// (byte `+0x5d` not 0xff) the help record (`MsGetExcelData(id, [table
    /// 0x01a860e4], strings)`, its strings word over the first) and, when
    /// there is one, the text at strings + its `u16` (`+4` with
    /// `MsGetSaveConfigHiragana() & 1`) through
    /// `AbmapHelpMessageStep_00a59350`. `eax`: that call's, else 0 (no
    /// record), else the item record with `al` 0xff.
    pub fn item_help(mem: &mut impl Host, entry: u32, strings: u32) -> u32 {
        // the slot holds the argument until a callee writes it
        mem.write(strings, entry);
        let id = mem.read::<u32>(entry.wrapping_add(8)) & 0xffff;
        let item = mem.call_sym(MS_GET_ROM_ITEM, Conv::Cdecl, &[id, strings]);
        let help: u8 = mem.read(item.wrapping_add(0x5d));
        if help == 0xff {
            return item & 0xffff_ff00 | 0xff;
        }
        let table = ITEM_HELP_TABLE.get(mem);
        let rec = mem.call_sym(
            MS_GET_EXCEL_DATA,
            Conv::Cdecl,
            &[u32::from(help), table, strings],
        );
        if rec == 0 {
            return 0;
        }
        let base: u32 = mem.read(strings);
        let kana = mem.call_sym(MS_GET_SAVE_CONFIG_HIRAGANA, Conv::Cdecl, &[]) & 1;
        let off: u16 = mem.read(rec.wrapping_add(4 * kana));
        let text = u32::from(off).wrapping_add(base);
        mem.call_sym(ABMAP_HELP_MESSAGE_STEP, Conv::Cdecl, &[text])
    }

    /// `AbmapPadDrawList_00a58900(panel, dx)`.
    pub const ABMAP_PAD_DRAW_LIST: u32 = 0x00a5_8900;
    /// The panels [`draw_panels`] walks (0x340 bytes each).
    pub const PANELS: u32 = 12;

    /// The slide of a side panel `width` wide: from the grid's slide
    /// state (byte `+0x115c2` moving, `+0x115c0` out, `+0x115c1` the
    /// step, counted down when going out) the panel's offset, `width` when
    /// it is fully in and 0 when fully out.
    fn slide_offset(mem: &impl ffx_bridge::Mem, width: i32, base: i32) -> i32 {
        let g = ABMAP.get(mem);
        let flag = |off: u32| mem.read::<u8>(g.wrapping_add(off)) != 0;
        if flag(0x1_15c2) {
            let step = i32::from(mem.read::<u8>(g.wrapping_add(0x1_15c1)));
            let f = if flag(0x1_15c0) { 3 - step } else { step };
            // C division toward zero
            f * width / 4 + base
        } else if flag(0x1_15c0) {
            0
        } else {
            width
        }
    }

    /// `AbmapDrawPanels_00a58660()` (name ours; the sphere grid's draw
    /// pass): each of the [`PANELS`] panels of `g_AbmapPad` (the base read
    /// once) that is shown (byte `+0x23`) through
    /// `AbmapPadDrawList_00a58900(panel, dx)`. Panels 6, 7 and 8 slide
    /// ([`slide_offset`], widths 0x4c, 0x88, 0xa5 from 0xf, 0x1b, 0x21):
    /// drawn at `off - width` while `off > 0`, and while `off < width`
    /// again at `off +` 0x120/0xf0 (6, CJK text or not), 0x150 (7) or
    /// 0x110/0xf0 (8); 9 and 10 at 0, or once the grid is out at
    /// 0x140/0x110 and 0x130/0x110; 11 at 0 or 0x150; the rest at 0. Then
    /// the open panel's (`+0x2700`, `i16`, none when negative) help
    /// callback `+0x3c`, when set, gets `(panel, its cursor entry, cursor
    /// +0x18)`; `eax` its result, else `g_AbmapPad`.
    pub fn draw_panels<M, C>(mem: &mut M, call: &mut C) -> u32
    where
        M: ffx_bridge::Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
    {
        let mut draw = |mem: &mut M, p: u32, dx: i32| {
            cdecl(mem, call, ABMAP_PAD_DRAW_LIST, &[p.cast_signed(), dx]);
        };
        let base = PAD.get(mem);
        for i in 0..PANELS {
            let p = base.wrapping_add(0x340 * i);
            if mem.read::<u8>(p.wrapping_add(0x23)) == 0 {
                continue;
            }
            let cjk = |mem: &M| CJK_TEXT.get(mem) != 0;
            let out = |mem: &M| mem.read::<u8>(ABMAP.get(mem).wrapping_add(0x1_15c0)) != 0;
            match i {
                6..=8 => {
                    let (width, start) = [(0x4c, 0xf), (0x88, 0x1b), (0xa5, 0x21)][i as usize - 6];
                    let off = slide_offset(mem, width, start);
                    if off > 0 {
                        draw(mem, p, off - width);
                    }
                    if off < width {
                        let second = match i {
                            6 => {
                                if cjk(mem) {
                                    0x120
                                } else {
                                    0xf0
                                }
                            }
                            7 => 0x150,
                            _ => {
                                if cjk(mem) {
                                    0x110
                                } else {
                                    0xf0
                                }
                            }
                        };
                        draw(mem, p, off + second);
                    }
                }
                9 | 10 => {
                    let dx = if !out(mem) {
                        0
                    } else if cjk(mem) {
                        if i == 9 { 0x140 } else { 0x130 }
                    } else {
                        0x110
                    };
                    draw(mem, p, dx);
                }
                11 => {
                    let dx = if out(mem) { 0x150 } else { 0 };
                    draw(mem, p, dx);
                }
                _ => draw(mem, p, 0),
            }
        }
        let pad = PAD.get(mem);
        let open = mem.read::<i16>(pad.wrapping_add(0x2700));
        if open < 0 {
            return pad;
        }
        let panel = pad.wrapping_add(0x340u32.wrapping_mul(u32::from(open.cast_unsigned())));
        let help: u32 = mem.read(panel.wrapping_add(0x3c));
        if help == 0 {
            return pad;
        }
        let cursor = i32::from(mem.read::<i16>(panel.wrapping_add(0x18)));
        let entry = panel
            .wrapping_add(0x40)
            .wrapping_add(cursor.cast_unsigned().wrapping_mul(0xc));
        call(mem, help, &[panel, entry, cursor.cast_unsigned()])
    }
}
