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

/// The box words `AbmapSetupPlyTag` leaves at `0x02305804..0x02305834`
/// for `AbmapPositionPlyTag` (named by their offset from `0x02305800`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TagBox {
    /// `+0x14`, `+0x18`: the offsets for corners with bit 2 set.
    pub wide: [u32; 2],
    /// `+0x1c`, `+0x20`: the offsets otherwise.
    pub narrow: [u32; 2],
    /// `+0x04`.
    pub w04: u32,
    /// `+0x08`.
    pub w08: u32,
    /// `+0x0c`.
    pub w0c: u32,
    /// `+0x10`.
    pub w10: u32,
    /// `+0x30`.
    pub w30: u32,
}

/// A tag's box offsets (`+0x46..+0x4e`, `u16`) for its corner: the
/// horizontal pair word negated on the left corners (1, 2), the vertical
/// one on the lower corners (0, 1); then the text offset and height.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // 16-bit stores of 32-bit words
pub fn tag_offsets(corner: u8, b: &TagBox) -> [u16; 4] {
    let [v, h] = if corner & 4 != 0 { b.wide } else { b.narrow };
    let (h, v) = (h as u16, v as u16);
    let sum = (b.w30.wrapping_add(b.w0c) as u16).wrapping_neg();
    match corner & 3 {
        0 => [h, v.wrapping_neg(), b.w10 as u16, b.w08 as u16],
        1 => [h.wrapping_neg(), v.wrapping_neg(), sum, b.w08 as u16],
        2 => [h.wrapping_neg(), v, sum, b.w04 as u16],
        _ => [h, v, b.w10 as u16, b.w04 as u16],
    }
}

/// The help message's macro slots for its kind (`g_Abmap + 0x11664`): the
/// byte 1 of each slot, slot 0 first. Kind 0 has
/// three, the third a byte-1 of 1; kinds 1 and 2 have two; others none.
#[must_use]
pub fn help_macros(kind: i16) -> &'static [u8] {
    match kind {
        0 => &[0, 0, 1],
        1 | 2 => &[0, 0],
        _ => &[],
    }
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::x87::{Reg, ftol, round_trip};
    use ffx_bridge::{Global, Mem};
    use ffx_graphic::host::{Conv, Host};

    use super::Pad;

    /// `g_Ee_Gpr`: the emulated R5900 registers; `r0` (always zero) first.
    pub const EE_GPR: Global<[u32; 4]> = Global::new(0x00c8_f508);
    /// `TOMenuGetControlPad2()`: the buttons held.
    pub const TO_MENU_GET_CONTROL_PAD2: u32 = 0x008b_e440;
    /// `TOMenuGetControlPadAnalogHist(pad, frame, axis, -1)`.
    pub const TO_MENU_GET_CONTROL_PAD_ANALOG_HIST: u32 = 0x008b_e450;

    /// `TkMenuSetPrevPlayer()`.
    pub const TK_MENU_SET_PREV_PLAYER: u32 = 0x008a_afa0;
    /// `TkMenuSetNextPlayer()`.
    pub const TK_MENU_SET_NEXT_PLAYER: u32 = 0x008a_af10;
    /// `TkMenuGetCurrentPlayer()`: the character shown, in `al`.
    pub const TK_MENU_GET_CURRENT_PLAYER: u32 = 0x008a_9860;
    /// `SndSepPlaySimple(sfx)`.
    pub const SND_SEP_PLAY_SIMPLE: u32 = 0x0088_6e80;
    /// `AbmapInitHoming(node, speed)`.
    pub const ABMAP_INIT_HOMING: u32 = 0x00a4_8d70;
    /// `AbmapCheckAchievements()`.
    pub const ABMAP_CHECK_ACHIEVEMENTS: u32 = 0x00a4_7210;
    /// `AbmapMarkEmptyMenus()`.
    pub const ABMAP_MARK_EMPTY_MENUS: u32 = 0x00a5_aca0;
    /// `AbmapResetCursorCamera()`.
    pub const ABMAP_RESET_CURSOR_CAMERA: u32 = 0x00a5_b030;
    /// `AbmapState_HomeOnNode`: the state that moves the cursor to a node.
    pub const ABMAP_STATE_HOME_ON_NODE: u32 = 0x00a5_9e80;
    /// `MsGetExcelData(id, table, &strings)`.
    pub const MS_GET_EXCEL_DATA: u32 = 0x007a_b870;
    /// `g_AbmapNodeKinds`: the table `AbmapFlagConnectedLinks` looks node
    /// kinds up in.
    pub const NODE_KINDS: Global<u32> = Global::new(0x01a8_60e0);

    /// `AbmapTryChangePlayer()` (`0x00a58ec0`): unless a state is stacked
    /// (`+0x115b0`), L1/R1 (pad edge bits 4/8 at `+0x11672`) switch to
    /// the previous/next character (direction at `+0x115be`, cleared when
    /// the character stays): a sound, the cursor homing on the new
    /// character's node (`+0x110cc + 0x50 * character`) at speed 0.25,
    /// `AbmapState_HomeOnNode` pushed over the current state, the menus
    /// refreshed. Ends in `AbmapResetCursorCamera`, whose `eax` it returns
    /// (`g_Abmap` when a state is stacked).
    pub fn try_change_player(mem: &mut impl Host) -> u32 {
        let g = ABMAP.get(mem);
        if mem.read::<u32>(g.wrapping_add(0x115b0)) != 0 {
            return g;
        }
        let edge: u16 = mem.read(g.wrapping_add(0x11672));
        if edge & 4 != 0 {
            mem.call_sym(TK_MENU_SET_PREV_PLAYER, Conv::Cdecl, &[]);
            let g = ABMAP.get(mem);
            mem.write(g.wrapping_add(0x115be), 1u8);
        } else if edge & 8 != 0 {
            mem.write(g.wrapping_add(0x115be), 2u8);
            mem.call_sym(TK_MENU_SET_NEXT_PLAYER, Conv::Cdecl, &[]);
        } else {
            return mem.call_sym(ABMAP_RESET_CURSOR_CAMERA, Conv::Cdecl, &[]);
        }
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x115bf), 0u8);
        let was: u8 = mem.read(g.wrapping_add(0x115bc));
        mem.write(g.wrapping_add(0x115bd), was);
        #[allow(clippy::cast_possible_truncation)] // a byte result
        let now = mem.call_sym(TK_MENU_GET_CURRENT_PLAYER, Conv::Cdecl, &[]) as u8;
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x115bc), now);
        if mem.read::<u8>(g.wrapping_add(0x115bd)) == mem.read::<u8>(g.wrapping_add(0x115bc)) {
            mem.write(g.wrapping_add(0x115be), 0u8);
        } else {
            mem.write(g.wrapping_add(0x116ac), 1u32);
            let g = ABMAP.get(mem);
            mem.write(g.wrapping_add(0x116a8), u32::MAX);
            mem.call_sym(SND_SEP_PLAY_SIMPLE, Conv::Cdecl, &[0x8000_0004]);
        }
        let g = ABMAP.get(mem);
        let who = u32::from(mem.read::<u8>(g.wrapping_add(0x115bc)));
        let node: u16 = mem.read(g.wrapping_add(0x110cc + who * 0x50));
        mem.call_sym(
            ABMAP_INIT_HOMING,
            Conv::Cdecl,
            &[u32::from(node), 0.25f32.to_bits()],
        );
        let g = ABMAP.get(mem);
        if mem.read::<u32>(g.wrapping_add(0x115b0)) == 0 {
            let state: u32 = mem.read(g.wrapping_add(0x115a8));
            mem.write(g.wrapping_add(0x115b0), state);
            let g = ABMAP.get(mem);
            let home = mem.rebase(ABMAP_STATE_HOME_ON_NODE);
            mem.write(g.wrapping_add(0x115a8), home);
        }
        mem.call_sym(ABMAP_CHECK_ACHIEVEMENTS, Conv::Cdecl, &[]);
        mem.call_sym(ABMAP_MARK_EMPTY_MENUS, Conv::Cdecl, &[]);
        mem.call_sym(ABMAP_RESET_CURSOR_CAMERA, Conv::Cdecl, &[])
    }

    /// `AbmapFlagConnectedLinks(flag)` (`0x00a58df0`): for every node
    /// whose kind's record (`MsGetExcelData(kind, NODE_KINDS)`) has bit
    /// 0x8000 at `+0x10`, ORs `flag` into byte `+0xe` of every link
    /// (`+0xa808`, 0x14 each, `+4` count) with that node at either end.
    /// Nodes of kind 0xffff are skipped. Returns what it leaves in `eax`:
    /// the node pointer past the last (0 with no nodes).
    pub fn flag_connected_links(mem: &mut impl Host, flag: u32) -> u32 {
        let g0 = ABMAP.get(mem);
        let mut g = g0;
        if mem.read::<i16>(g.wrapping_add(2)) <= 0 {
            return 0;
        }
        let mut node = g0.wrapping_add(0x80e);
        let mut i: u32 = 0;
        loop {
            let kind: u16 = mem.read(node);
            if kind != 0xffff {
                let table = NODE_KINDS.get(mem);
                let strings = mem.scratch();
                let rec = mem.call_sym(
                    MS_GET_EXCEL_DATA,
                    Conv::Cdecl,
                    &[u32::from(kind), table, strings],
                );
                g = ABMAP.get(mem);
                if mem.read::<u16>(rec.wrapping_add(0x10)) & 0x8000 != 0 {
                    let mut link = None::<u32>;
                    loop {
                        g = ABMAP.get(mem);
                        let count = i32::from(mem.read::<i16>(g.wrapping_add(4)));
                        #[allow(clippy::cast_sign_loss)] // the original's address arithmetic
                        let end = g.wrapping_add(((count * 5 + 0x2a02) * 4) as u32);
                        let mut at =
                            link.map_or(g.wrapping_add(0xa808), |l: u32| l.wrapping_add(0x14));
                        #[allow(clippy::cast_possible_truncation)] // compared as 16 bits
                        let me = i as u16;
                        while at < end
                            && mem.read::<u16>(at) != me
                            && mem.read::<u16>(at.wrapping_add(2)) != me
                        {
                            at = at.wrapping_add(0x14);
                        }
                        if at >= end {
                            break;
                        }
                        let b: u8 = mem.read(at.wrapping_add(0xe));
                        #[allow(clippy::cast_possible_truncation)] // a byte argument
                        mem.write(at.wrapping_add(0xe), b | flag as u8);
                        link = Some(at);
                    }
                }
            }
            node = node.wrapping_add(0x28);
            i += 1;
            if i32::try_from(i).unwrap_or(i32::MAX) >= i32::from(mem.read::<i16>(g.wrapping_add(2)))
            {
                return node;
            }
        }
    }

    /// `TOGetSaveChrName(chr)`: the character's name.
    pub const TO_GET_SAVE_CHR_NAME: u32 = 0x008a_c850;
    /// `TOGetTextWidth(text, arg)`: in `ST0`.
    pub const TO_GET_TEXT_WIDTH: u32 = 0x008b_70c0;
    /// The narrowest name tag (`f32`).
    pub const TAG_MIN_WIDTH: u32 = 0x00b4_1d84;
    /// `AbmapSetupPlyTag(tag)`: the tag's box sizes into
    /// `TAG_BOX` (`0x02305804..0x02305834`).
    pub const ABMAP_SETUP_PLY_TAG: u32 = 0x00a4_7c60;
    /// `g_pppSinTable`: 4096 sines (`f32`) over a turn.
    pub const SIN_TABLE: u32 = 0x00c4_4bf0;
    /// What `AbmapSetupPlyTag` leaves for `AbmapPositionPlyTag`.
    pub const TAG_BOX: u32 = 0x0230_5804;

    /// Character `chr`'s name tag (`g_Abmap + 0x11088`, 0x50 each).
    fn ply_tag(mem: &impl Mem, chr: u32) -> u32 {
        ABMAP
            .get(mem)
            .wrapping_add(0x11088)
            .wrapping_add(chr.wrapping_mul(0x50))
    }

    /// `AbmapInitPlyTag(chr, node, scale, a, b, c)` (`0x00a57f80`):
    /// character `chr`'s tag placed on node `node` (`+0x44`; position
    /// `+0`, `+4` from the node's `i16` x, y; `+8` = 0, `+0xc` = 1; radius
    /// `+0x3c` as [`super::node_radius`]), `a`, `b`, `c` at `+0x20..+0x2c`,
    /// `scale` at `+0x40`, corner `+0x4e` = 0. For the seven party members
    /// (`chr <= 6`, signed) the name (`+0x2c`) and its width at least
    /// `TAG_MIN_WIDTH` (`ftol`, `u16` `+0x30`), height `+0x32` = 16.
    /// Returns the tag.
    #[allow(clippy::many_single_char_names)] // g, n, x, y, w
    pub fn init_ply_tag(mem: &mut impl Host, args: [u32; 6]) -> u32 {
        let [chr, node, scale, w0, w1, w2] = args;
        let tag = ply_tag(mem, chr);
        let at = |o: u32| tag.wrapping_add(o);
        #[allow(clippy::cast_possible_truncation)] // a 16-bit field
        mem.write(at(0x44), node as u16);
        let g = ABMAP.get(mem);
        let n = g.wrapping_add(node.wrapping_mul(0x28));
        let x: i16 = mem.read(n.wrapping_add(0x808));
        mem.write(at(0), f32::from(x));
        let y: i16 = mem.read(n.wrapping_add(0x80a));
        mem.write(at(4), f32::from(y));
        mem.write(at(8), [0.0f32, 1.0]);
        let radius = node_radius(mem, node);
        mem.write(at(0x20), [w0, w1]);
        mem.write(at(0x3c), radius);
        mem.write(at(0x4e), 0u8);
        mem.write(at(0x28), w2);
        mem.write(at(0x40), round_trip(f32::from_bits(scale)));
        #[allow(clippy::cast_possible_wrap)] // compared signed
        if chr as i32 <= 6 {
            let name = mem.call_sym(TO_GET_SAVE_CHR_NAME, Conv::Cdecl, &[chr]);
            mem.write(at(0x2c), name);
            let w = mem.call_sym_st0(TO_GET_TEXT_WIDTH, Conv::Cdecl, &[name, 1]);
            let min: f32 = mem.read(mem.rebase(TAG_MIN_WIDTH));
            let w = if w < min { min } else { w };
            #[allow(clippy::cast_possible_truncation)] // the original stores ax
            let w = ftol(mem, f64::from(w)) as u16;
            mem.write(at(0x30), [w, 0x10]);
        }
        tag
    }

    /// `AbmapPositionPlyTag(chr)` (`0x00a58080`, Fahrenheit's name):
    /// `AbmapSetupPlyTag`, then the tag's anchor (`+0x10`, `+0x14`; `+0x18`
    /// = 0, `+0x1c` = 1) at the radius `+0x3c` from its position toward
    /// corner `+0x4e & 3` (angle `corner * 0x4000 + 0x2000`, y up), and
    /// the box offsets `+0x46..+0x4c` from `TAG_BOX` by corner (bit 2 picks
    /// the wide or narrow pair). Returns the tag.
    pub fn position_ply_tag<M: Host>(mem: &mut M, chr: u32) -> u32 {
        let tag = ply_tag(mem, chr);
        let at = |o: u32| tag.wrapping_add(o);
        let corner: u8 = mem.read(at(0x4e));
        let angle = u32::from(u16::from(corner).wrapping_mul(0x4000).wrapping_add(0x2000));
        mem.call_sym(ABMAP_SETUP_PLY_TAG, Conv::Cdecl, &[tag]);
        let sin = |mem: &mut M, i: u32| -> f32 { mem.read(mem.rebase(SIN_TABLE + 4 * i)) };
        let cos = sin(mem, ((angle + 0x4000) >> 4) & 0xfff);
        let sn = sin(mem, angle >> 4);
        let [x, y]: [f32; 2] = mem.read(at(0));
        let r: f32 = mem.read(at(0x3c));
        mem.write(at(0x10), Reg::load(cos).mul_m32(r).add_m32(x).store());
        let r: f32 = mem.read(at(0x3c));
        mem.write(at(0x14), (Reg::load(y) - Reg::load(sn).mul_m32(r)).store());
        mem.write(at(0x18), [0.0f32, 1.0]);
        let corner: u8 = mem.read(at(0x4e));
        let word = |mem: &mut M, o: u32| -> u32 { mem.read(mem.rebase(TAG_BOX + o)) };
        let tb = super::TagBox {
            wide: [word(mem, 0x10), word(mem, 0x14)],
            narrow: [word(mem, 0x18), word(mem, 0x1c)],
            w04: word(mem, 0),
            w08: word(mem, 4),
            w0c: word(mem, 8),
            w10: word(mem, 0xc),
            w30: word(mem, 0x2c),
        };
        mem.write(at(0x46), super::tag_offsets(corner, &tb));
        tag
    }

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

    /// `MsGetSaveMenu()`: the save's menu settings (`u32`; bits
    /// `0xc000` the sphere grid kind, `0x8000` expert).
    pub const MS_GET_SAVE_MENU: u32 = 0x0078_5290;
    /// `GetCD()`: the CD reader's function table (`+0` read(file, buf,
    /// 0, 0), `+8` wait(0), `+0x10` select(group)).
    pub const GET_CD: u32 = 0x0076_d0d0;
    /// `g_AbmapOefData`: where the grid's effect file lands.
    pub const OEF_DATA: u32 = 0x011a_1260;
    /// `g_AbmapOef`: the effect file `AbmapLoadOef` loaded.
    pub const OEF: Global<u32> = Global::new(0x01a8_60e8);
    /// `op_oef2_set2(oef)`.
    pub const OP_OEF2_SET2: u32 = 0x007e_3e30;
    /// `op_oef2_tex_clut_cue_multi(oef, first, last, mask)`.
    pub const OP_OEF2_TEX_CLUT_CUE_MULTI: u32 = 0x007e_4830;
    /// `op_oef2_tex_clut_cue_trans(mask)`.
    pub const OP_OEF2_TEX_CLUT_CUE_TRANS: u32 = 0x007e_4c30;
    /// `op_oef2_tex_clut_ovl_trans(mask)`.
    pub const OP_OEF2_TEX_CLUT_OVL_TRANS: u32 = 0x007e_4e80;
    /// `op_oef2_tex_clut_ovl_clear(mask)`.
    pub const OP_OEF2_TEX_CLUT_OVL_CLEAR: u32 = 0x007e_4e40;
    /// `AbmapSetKindSizes(oef)`.
    pub const ABMAP_SET_KIND_SIZES: u32 = 0x00a5_adf0;
    /// `AbmapSetupKindShapes()`.
    pub const ABMAP_SETUP_KIND_SHAPES: u32 = 0x00a5_b230;

    /// `AbmapLoadOef_00a57620()` (`0x00a57620`, from `eiAbmStart`): the
    /// expert grid (`MsGetSaveMenu` bits `0xc000` = `0x8000`) sets
    /// `g_Abmap + 0x116a4` = 2 and reads file 0x14, else 0 and file 8 (CD
    /// group 0x25) into `g_AbmapOefData`, waits, and makes it
    /// `g_AbmapOef`; then its textures and CLUTs 0..=0x23 and 0x24..=0x3e8
    /// are cued (mask 0x100) and transferred, the overlay transferred and
    /// cleared, the kind sizes read from it, and a tail call to
    /// `AbmapSetupKindShapes` (`eax` its result).
    pub fn load_oef(mem: &mut impl Host) -> u32 {
        let menu = mem.call_sym(MS_GET_SAVE_MENU, Conv::Cdecl, &[]);
        let expert = mem.read::<u32>(menu) & 0xc000 == 0x8000;
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x1_16a4), if expert { 2u32 } else { 0 });
        let cd = mem.call_sym(GET_CD, Conv::Cdecl, &[]);
        let select: u32 = mem.read(cd.wrapping_add(0x10));
        mem.call(select, Conv::Cdecl, &[0x25]);
        let data = mem.rebase(OEF_DATA);
        let read: u32 = mem.read(cd);
        let file = if expert { 0x14 } else { 8 };
        mem.call(read, Conv::Cdecl, &[file, data, 0, 0]);
        let wait: u32 = mem.read(cd.wrapping_add(8));
        mem.call(wait, Conv::Cdecl, &[0]);
        OEF.set(mem, data);
        mem.call_sym(OP_OEF2_SET2, Conv::Cdecl, &[data]);
        for (first, last) in [(0, 0x23), (0x24, 0x3e8)] {
            let oef = OEF.get(mem);
            mem.call_sym(
                OP_OEF2_TEX_CLUT_CUE_MULTI,
                Conv::Cdecl,
                &[oef, first, last, 0x100],
            );
            mem.call_sym(OP_OEF2_TEX_CLUT_CUE_TRANS, Conv::Cdecl, &[0x100]);
        }
        mem.call_sym(OP_OEF2_TEX_CLUT_OVL_TRANS, Conv::Cdecl, &[0x100]);
        mem.call_sym(OP_OEF2_TEX_CLUT_OVL_CLEAR, Conv::Cdecl, &[0x100]);
        let oef = OEF.get(mem);
        mem.call_sym(ABMAP_SET_KIND_SIZES, Conv::Cdecl, &[oef]);
        mem.call_sym(ABMAP_SETUP_KIND_SHAPES, Conv::Cdecl, &[])
    }

    /// `TOMacroSetByte1(win, n, v)`.
    pub const TO_MACRO_SET_BYTE1: u32 = 0x008b_84f0;
    /// `TOMacroSetValue_008b8510(win, n, v)`.
    pub const TO_MACRO_SET_VALUE: u32 = 0x008b_8510;
    /// `TOAddPktBaseMesFontLAbmap(pkt, kind, text, x, y, mode, ..., a,
    /// shadow)`, 12 words.
    pub const TO_ADD_PKT_BASE_MES_FONT_L_ABMAP: u32 = 0x008e_8fb0;
    /// `AbmapHelpLineDraw(text, x, y)`.
    pub const ABMAP_HELP_LINE_DRAW: u32 = 0x00a5_94c0;
    /// `g_PppPacketPos`: the packet write position.
    pub const PACKET_POS: Global<u32> = Global::new(0x0230_fd2c);

    /// `AbmapHelpMessageStep_00a59350(fallback)` (`0x00a59350`), callees
    /// through `call` (cdecl): while the help timer (`i16` `g_Abmap +
    /// 0x11666`) is positive it counts down, the message's macro values go
    /// to window 7 ([`super::help_macros`] of the kind `i16` `+0x11664`:
    /// byte 1 then the word, values from `+0x11658`, 4 apart), and the
    /// message `+0x11654`, when set, is drawn at (0x48, 0x23) in window 7
    /// with grey 0x80 and the fade byte `+0x115b9` as alpha into
    /// `g_PppPacketPos` (`+ 0x10`), `eax` the new position; with no message
    /// `eax` is `g_Abmap`. Once the timer is out, `fallback` (when not 0)
    /// is drawn by `AbmapHelpLineDraw(fallback, 0x48, 0x23)` (`eax` its
    /// result), else `eax` is 0.
    pub fn help_message_step<M: Mem>(
        mem: &mut M,
        fallback: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let g = ABMAP.get(mem);
        let timer: i16 = mem.read(g.wrapping_add(0x1_1666));
        if timer <= 0 {
            if fallback == 0 {
                return 0;
            }
            let f = mem.rebase(ABMAP_HELP_LINE_DRAW);
            return call(mem, f, &[fallback, 0x48, 0x23]);
        }
        mem.write(g.wrapping_add(0x1_1666), timer.wrapping_sub(1));
        let kind: i16 = mem.read(ABMAP.get(mem).wrapping_add(0x1_1664));
        for (n, &byte) in (0u32..).zip(super::help_macros(kind)) {
            let set_byte = mem.rebase(TO_MACRO_SET_BYTE1);
            call(mem, set_byte, &[7, n, u32::from(byte)]);
            let value: u32 = mem.read(ABMAP.get(mem).wrapping_add(0x1_1658 + 4 * n));
            let set_value = mem.rebase(TO_MACRO_SET_VALUE);
            call(mem, set_value, &[7, n, value]);
        }
        let g = ABMAP.get(mem);
        let text: u32 = mem.read(g.wrapping_add(0x1_1654));
        if text == 0 {
            return g;
        }
        let alpha = u32::from(mem.read::<u8>(g.wrapping_add(0x1_15b9)));
        let pos = PACKET_POS.get(mem).wrapping_add(0x10);
        let f = mem.rebase(TO_ADD_PKT_BASE_MES_FONT_L_ABMAP);
        let end = call(
            mem,
            f,
            &[pos, 7, text, 0x48, 0x23, 0, 0, 0x80, 0x80, 0x80, alpha, 0],
        );
        PACKET_POS.set(mem, end);
        end
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
