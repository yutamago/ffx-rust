//! `Abmap`: helpers of the sphere grid (the ability map), placed with the
//! map module by `tools/progress.py`.

use ffx_bridge::Global;

/// The sphere grid data (`LoadAbmap`): nodes of `0x28` bytes from
/// `+0x808`, links of `0x14` bytes, node kinds of `0x30` bytes.
pub const ABMAP: Global<u32> = Global::new(0x0230_5834);

/// 3.0, an `f64` in `.rdata`: added to a node's radius when homing
/// (`AbmapInitHoming`).
pub const HOMING_MARGIN: Global<f64> = Global::new(0x00b2_c5c8);
/// `AbmapState_MovingToTarget`, the state `AbmapInitMovingToTarget` sets.
pub const STATE_MOVING_TO_TARGET: u32 = 0x00a5_9990;
/// `AbmapState_HomeOnNode`, the state `AbmapTransitionToHoming` sets.
pub const STATE_HOME_ON_NODE: u32 = 0x00a5_9e80;
/// The step `AbmapCalcMoveCost` adds to the next node (1 or 4).
pub const MOVE_STEP: Global<u32> = Global::new(0x01a8_6100);
/// The highest cost it walks to (`i32`, the low 16 bits of the cost
/// compared).
pub const MOVE_LIMIT: Global<i32> = Global::new(0x01a8_60fc);
/// The link byte mask that makes a step cost 1.
pub const MOVE_MASK: Global<u8> = Global::new(0x01a8_6104);
/// The pad state `AbmapCtrlTilt` reads (`+0x2700`, `i16`).
pub const TILT_PAD: Global<u32> = Global::new(0x01a8_6108);
/// The tilt levels 1 and 2 (`f32`s in `.rdata`: -3640.0, -7281.0).
pub const TILT_LEVELS: u32 = 0x00b7_b728;
/// The zoom levels 1..=3 (`f32`s in `.rdata`: 0.5, 0.25, 0.125;
/// `LoadSaveAbmap`).
pub const ZOOM_LEVELS: [u32; 3] = [0x00b9_2194, 0x00b9_2180, 0x00b4_9368];
/// 0.375 (`.rdata`): at most this zoom `+0x116b0` is -2.
pub const ZOOM_NEAR: u32 = 0x00b4_936c;
/// The help screen's state (`0x00a45010`), set by `AbmapResetHelp`.
pub const STATE_HELP: u32 = 0x00a4_5010;
/// `AbmapSetHelpMessage`, the message hook `AbmapResetHelp` sets.
pub const SET_HELP_MESSAGE: u32 = 0x00a4_54a0;
/// 1.0 and 8.0 (`f64`s in `.rdata`), `AbmapPulseCursor`'s scale.
pub const PULSE_ONE: u32 = 0x00b9_2248;
/// See [`PULSE_ONE`].
pub const PULSE_SCALE: u32 = 0x00b9_22a8;
/// The emulated VU0 registers `vf01` and `vf05` (`AbmapVu0LoadMatrix1`/`2`
/// load four each).
pub const VU0_MATRIX1: u32 = 0x00c8_f7a0;
/// See [`VU0_MATRIX1`].
pub const VU0_MATRIX2: u32 = 0x00c8_f7e0;
/// The sine table (4096 `f32`s, a quarter turn in 1024).
pub const SIN_TABLE: u32 = 0x00c4_4bf0;
/// `SndSepPlaySimple(sfx)` (`0x00886e80`).
pub const SND_SEP_PLAY_SIMPLE: u32 = 0x0088_6e80;
/// `op_oef2_a_get(src, kind, id)` (`0x007e3810`).
pub const OP_OEF2_A_GET: u32 = 0x007e_3810;
/// The eight node kind ids (`0x00c86008`, bytes).
pub const KIND_IDS: u32 = 0x00c8_6008;
/// `g_AbmapPad` (`0x01a86108`): the menu pads, 0x340 bytes each.
pub const ABMAP_PAD: u32 = 0x01a8_6108;

/// `AbmapPadSlideOutStep` (`0x00a59800`), the pad step `AbmapPadStartSlideOut`
/// installs.
pub const PAD_SLIDE_OUT_STEP: u32 = 0x00a5_9800;
/// `AbmapPadSlideInStep` (`0x00a5a270`), the pad step `AbmapPadStartSlideIn`
/// installs.
pub const PAD_SLIDE_IN_STEP: u32 = 0x00a5_a270;
/// `MsGetSaveSP(chr)` (`0x007853e0`).
pub const MS_GET_SAVE_SP: u32 = 0x0078_53e0;
/// `TOAmapMakeItemList(kind)` (`0x008c6730`).
pub const TO_AMAP_MAKE_ITEM_LIST: u32 = 0x008c_6730;
/// The jump to `TOAmapGetItemListMax` the original calls (`0x008c6700`).
pub const TO_AMAP_GET_ITEM_LIST_MAX: u32 = 0x008c_6700;
/// The grid's current character byte (`g_Abmap + 0x115bc`).
pub const CURRENT_CHR: u32 = 0x115bc;
/// `MsGetSavePlayerPtr(chr)` (`0x00785330`).
pub const MS_GET_SAVE_PLAYER_PTR: u32 = 0x0078_5330;
/// `MsGetSaveItemNum(item)` (`0x007904b0`).
pub const MS_GET_SAVE_ITEM_NUM: u32 = 0x0079_04b0;
/// `MsGetRomItem(item, out)` (`0x007909f0`).
pub const MS_GET_ROM_ITEM: u32 = 0x0079_09f0;
/// `MsGetExcelData(i, table, out)` (`0x007ab870`).
pub const MS_GET_EXCEL_DATA: u32 = 0x007a_b870;
/// The item ability excel table pointer (`0x01a860e4`).
pub const ITEM_EXCEL_TABLE: u32 = 0x01a8_60e4;
/// The node excel table pointer (`0x01a860e0`).
pub const NODE_EXCEL_TABLE: u32 = 0x01a8_60e0;
/// `AbmapForEachNode(cb, ctx)` (`0x00a59710`).
pub const ABMAP_FOR_EACH_NODE: u32 = 0x00a5_9710;
/// `AbmapForNodeAndLinked(node, cb, ctx)` (`0x00a59760`).
pub const ABMAP_FOR_NODE_AND_LINKED: u32 = 0x00a5_9760;
/// `AbmapItemNodeEffect` (`0x00a49440`), the iterators' callback.
pub const ITEM_NODE_EFFECT: u32 = 0x00a4_9440;
/// `AbmapItemUsableAt` (`0x00a49310`).
pub const ITEM_USABLE_AT: u32 = 0x00a4_9310;
/// `g_pppEnvSlot`.
pub const PPP_ENV_SLOT: u32 = 0x00c3_a4cc;
/// The sphere grid's ppp environment (`AbmapStartPart`).
pub const ABMAP_PPP_ENV: u32 = 0x01a8_6034;
/// The sphere grid's ppp data pointer (its `+0x10` the part table).
pub const ABMAP_PPP_DATA: u32 = 0x0230_5800;
/// `get_ptr32_func(p)` (`0x00a44650`).
pub const GET_PTR32_FUNC: u32 = 0x00a4_4650;
/// `pppStartPart(part, flags, id, a, b)` (`0x007123c0`).
pub const PPP_START_PART: u32 = 0x0071_23c0;
/// `abmap_test_panel(bit, node)` (`0x00a457d0`).
pub const ABMAP_TEST_PANEL: u32 = 0x00a4_57d0;
/// `abmap_other_has_panel(chr, node)` (`0x00a45800`).
pub const ABMAP_OTHER_HAS_PANEL: u32 = 0x00a4_5800;
/// `abmap_chr_node(chr)` (`0x00a45870`).
pub const ABMAP_CHR_NODE: u32 = 0x00a4_5870;
/// `MsGetSavePlyJoined(chr)` (`0x007853a0`).
pub const MS_GET_SAVE_PLY_JOINED: u32 = 0x0078_53a0;
/// `AbmapMarkLockedEntries(pad)` (`0x00a5a2e0`).
pub const MARK_LOCKED_ENTRIES: u32 = 0x00a5_a2e0;
/// `MsGetSaveMenu()` (`0x00785290`).
pub const MS_GET_SAVE_MENU: u32 = 0x0078_5290;
/// `GetCD()` (`0x0076d0d0`): the CD library's function table.
pub const GET_CD: u32 = 0x0076_d0d0;
/// `achievementUnlockAchievement(id)` (`0x00822380`).
pub const ACHIEVEMENT_UNLOCK: u32 = 0x0082_2380;
/// `pppCreateHeap(env, buffer, size)` (`0x0072c480`).
pub const PPP_CREATE_HEAP: u32 = 0x0072_c480;
/// The sphere grid's ppp heap (0x7d000 bytes).
pub const ABMAP_PPP_HEAP: u32 = 0x016c_1870;
/// The warp's ppp part.
pub const ABMAP_WARP_PART: u32 = 0x01a8_6060;
/// 0.5 (`f32`, `.rdata`): the warp part's scale.
pub const WARP_PART_SCALE: u32 = 0x00b9_2194;
/// The warp's draw hook (`0x00a4c430`).
pub const WARP_DRAW: u32 = 0x00a4_c430;
/// `AbmapState_Warping` (`0x00a47f00`).
pub const STATE_WARPING: u32 = 0x00a4_7f00;
/// `MsMenuStatusNumSize()` (`0x00790220`).
pub const MS_MENU_STATUS_NUM_SIZE: u32 = 0x0079_0220;
/// `MsMenuStatusNum(i, 0)` (`0x00790200`): a menu status entry.
pub const MS_MENU_STATUS_NUM: u32 = 0x0079_0200;
/// `MsGetRomPlyCommand(id, tmp)` (`0x00790a90`).
pub const MS_GET_ROM_PLY_COMMAND: u32 = 0x0079_0a90;
/// `dbgPrintf` (`0x0062f500`).
pub const DBG_PRINTF: u32 = 0x0062_f500;
/// `debug_exit_trace` (`0x0062f510`).
pub const DEBUG_EXIT_TRACE: u32 = 0x0062_f510;
/// `"addMenuPrim error!!!  menu->mprimn >= MAXMENUPRIM\n"` (`.rdata`).
pub const MENU_PRIM_ERROR: u32 = 0x00b7_b558;
/// The command menu's (pad 7) handler, which opens the item menu.
pub const COMMAND_MENU_STEP: u32 = 0x00a5_6330;
/// The item menu's (pad 8) handler, which goes back to the command menu.
pub const ITEM_MENU_STEP: u32 = 0x00a5_60d0;
/// `AbmapItemApplyNode` (`0x00a5b490`), the iterators' callback.
pub const ITEM_APPLY_NODE: u32 = 0x00a5_b490;
/// `0x00a5d120(kind, node, row, chr)`: a special item's abilities.
pub const ABMAP_SPECIAL_EFFECT: u32 = 0x00a5_d120;
/// The kind shape record tables (`0x01a85f98`, then `0x01a85fe8`).
pub const KIND_SHAPE_TABLES: u32 = 0x01a8_5f98;
/// The size class shapes (`0x0230578c`, `0x02305790`, `0x02305794`).
pub const SIZE_CLASS_SHAPES: u32 = 0x0230_578c;
/// 7.8125e-5 (`f64`, `.rdata`): a kind's scale per size unit.
pub const KIND_SCALE: u32 = 0x00b7_b608;
/// 3.0 (`f64`, `.rdata`): added to a kind's circle radius.
pub const KIND_RADIUS_PAD: u32 = 0x00b2_c5c8;
/// 9362.286 (`f64`, `.rdata`): the circle's angle step (65536 / 7).
pub const KIND_ANGLE_STEP: u32 = 0x00b7_b6c0;

/// 256.0 (`f32`, `.rdata`): the cursor distance (`AbmapResetCursorCamera`).
pub const CURSOR_DISTANCE: u32 = 0x00b5_01c0;
/// 32.0 (`f64`, `.rdata`): how far the cursor sits below the character.
pub const CURSOR_LIFT: u32 = 0x00b4_12f8;
/// The characters' cursor colours, a vector each (`0x00c86010`).
pub const CHR_COLORS: u32 = 0x00c8_6010;
/// `MsGetSaveAbilityMap()` (`0x00784f40`): the save's sphere grid block.
pub const MS_GET_SAVE_ABILITY_MAP: u32 = 0x0078_4f40;
/// The player tag globals `AbmapSetupPlyTag` fills (`0x02305804`..30).
pub const PLY_TAG: u32 = 0x0230_5804;
/// 2.0 (`f64`, `.rdata`).
pub const TWO_F64: u32 = 0x00b9_2270;
/// 1.5, 3.0, 5.0 (`f32`s, `.rdata`): `AbmapScaledLevelValue`'s tables.
pub const LEVEL_SCALES: [u32; 3] = [0x00b3_eb58, 0x00b3_eb94, 0x00b3_eb98];

/// Offsets in the sphere grid data.
pub mod layout {
    /// `+0x4`: the link count (`i16`).
    pub const LINK_COUNT: u32 = 0x4;
    /// `+0x808`: the nodes.
    pub const NODES: u32 = 0x808;
    /// Bytes per node: `+0` x and `+2` y (`i16`), `+6` the kind (`u16`),
    /// `+0x21` the panel bits (`abmap_lost_panel`).
    pub const NODE_SIZE: u32 = 0x28;
    /// `+0xa816`: the first link's flags byte.
    pub const LINK_FLAGS: u32 = 0xa816;
    /// Bytes per link.
    pub const LINK_SIZE: u32 = 0x14;
    /// `+0xf834`: the node kinds' sizes (`i16`, twice the radius), `0x30`
    /// bytes per kind.
    pub const KIND_SIZE: u32 = 0xf834;
    /// Bytes per node kind.
    pub const KIND_STRIDE: u32 = 0x30;
}

/// The sphere grid where the game keeps it.
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{ABMAP, layout};
    use crate::x87;
    use ffx_bridge::x87::{Reg, ftol};

    /// `AbmapAllLinksLimitFlag(mask)` (`0x00a473d0`): every link's flags
    /// byte masked with `mask`. Returns the `eax` it ends with: the data
    /// pointer with no links, else past the last link's flags.
    pub fn abmap_all_links_limit_flag(mem: &mut impl Mem, mask: u8) -> u32 {
        let g = ABMAP.get(mem);
        let n = mem.read::<i16>(g.wrapping_add(layout::LINK_COUNT));
        if n == 0 {
            return g;
        }
        #[allow(clippy::cast_sign_loss)] // the original counts down from it
        let n = i32::from(n) as u32;
        let mut at = g.wrapping_add(layout::LINK_FLAGS);
        for _ in 0..n {
            let b = mem.read::<u8>(at);
            mem.write(at, b & mask);
            at = at.wrapping_add(layout::LINK_SIZE);
        }
        at
    }

    /// `abmap_lost_panel(bit, node)` (`0x00a458c0`): clears panel bit
    /// `bit` (an 8-bit shift: none from 8 on) of the node. Returns the data
    /// pointer's upper bytes with the mask in `al`.
    pub fn abmap_lost_panel(mem: &mut impl Mem, bit: u32, node: u32) -> u32 {
        let g = ABMAP.get(mem);
        let n = bit & 31;
        let mask = !(if n < 8 { 1u8 << n } else { 0 });
        let at = g
            .wrapping_add(node.wrapping_mul(layout::NODE_SIZE))
            .wrapping_add(0x829);
        let b = mem.read::<u8>(at);
        mem.write(at, b & mask);
        (g & !0xff) | u32::from(mask)
    }

    /// `AbmapGetNodePosRadius(node, out)` (`0x00a5b4c0`): the node's x, y
    /// (`i16`s as floats), 0.0, 1.0; returns half its kind's size (`int`
    /// division) as a float. Node `0xffff` gives the origin and 0.0.
    pub fn abmap_get_node_pos_radius(mem: &mut impl Mem, node: u16, out: u32) -> f32 {
        if node == 0xffff {
            mem.write(out, [0f32, 0.0, 0.0, 1.0]);
            return 0.0;
        }
        let g = ABMAP.get(mem);
        let n = g
            .wrapping_add(layout::NODES)
            .wrapping_add(u32::from(node) * layout::NODE_SIZE);
        let x = mem.read::<i16>(n);
        let y = mem.read::<i16>(n.wrapping_add(2));
        mem.write(out, [f32::from(x), f32::from(y), 0.0, 1.0]);
        let kind = u32::from(mem.read::<u16>(n.wrapping_add(6)));
        let k = mem.read::<i16>(
            g.wrapping_add(layout::KIND_SIZE)
                .wrapping_add(kind * layout::KIND_STRIDE),
        );
        // cdq; sub; sar 1: toward zero
        f32::from(k / 2)
    }

    // ---- round 5: moving on the grid ----

    /// `AbmapInitHoming(node, t)` (`0x00a48d70`): homing on `node`: the
    /// camera's four words (`+0x112b8`) kept at `+0x1163c`, `+0x11620` 0.0,
    /// `+0x11624` = `t`, `+0x11628` = `+0x112f4`, `+0x1162c` the node's
    /// radius (half its kind's size, 16-bit) plus 3.0 (an `f64` in
    /// `.rdata`), the node at `+0x11634` and `+0x112fc` (`u16`). Returns
    /// the data pointer.
    pub fn abmap_init_homing(mem: &mut impl Mem, node: u32, t: u32) -> u32 {
        let g = ABMAP.get(mem);
        let cam = mem.read::<[u32; 4]>(g.wrapping_add(0x112b8));
        mem.write(g.wrapping_add(0x1163c), cam);
        mem.write(g.wrapping_add(0x11620), 0f32);
        mem.write(g.wrapping_add(0x11624), x87::copy(t));
        let v = mem.read::<u32>(g.wrapping_add(0x112f4));
        mem.write(g.wrapping_add(0x11628), x87::copy(v));
        let kind = u32::from(
            mem.read::<u16>(
                g.wrapping_add(layout::NODES + 6)
                    .wrapping_add(node.wrapping_mul(layout::NODE_SIZE)),
            ),
        );
        let size = mem.read::<i16>(
            g.wrapping_add(layout::KIND_SIZE)
                .wrapping_add(kind * layout::KIND_STRIDE),
        );
        let r = f64::from(size >> 1) + super::HOMING_MARGIN.get(mem);
        mem.write(g.wrapping_add(0x1162c), x87::store(r));
        #[allow(clippy::cast_possible_truncation)] // the low half
        let n = node as u16;
        mem.write(g.wrapping_add(0x11634), n);
        mem.write(g.wrapping_add(0x112fc), n);
        g
    }

    /// `AbmapInitMovingToTarget(target, node)` (`0x00a48c80`): the
    /// target's node (`u16` at `+0x110cc + 0x50 * target`) as both ends
    /// (`+0x11630`, `+0x11632`), `node` at `+0x11634`, `+0x11620` 1.0,
    /// `+0x11624` 0.0, the target at `+0x11638`; with no state saved
    /// (`+0x115b0`) the current state (`+0x115a8`) is saved and replaced
    /// by `AbmapState_MovingToTarget`. Returns the data pointer.
    pub fn abmap_init_moving_to_target(mem: &mut impl Mem, target: u32, node: u16) -> u32 {
        let g = ABMAP.get(mem);
        let n = mem.read::<u16>(
            g.wrapping_add(0x110cc)
                .wrapping_add(target.wrapping_mul(0x50)),
        );
        mem.write(g.wrapping_add(0x11632), n);
        mem.write(g.wrapping_add(0x11630), n);
        mem.write(g.wrapping_add(0x11634), node);
        mem.write(g.wrapping_add(0x11620), 1f32);
        mem.write(g.wrapping_add(0x11624), 0f32);
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(g.wrapping_add(0x11638), target as u8);
        if mem.read::<u32>(g.wrapping_add(0x115b0)) == 0 {
            let s = mem.read::<u32>(g.wrapping_add(0x115a8));
            mem.write(g.wrapping_add(0x115b0), s);
            let f = mem.rebase(super::STATE_MOVING_TO_TARGET);
            mem.write(g.wrapping_add(0x115a8), f);
        }
        g
    }

    /// `AbmapCalcMoveCost(node, cost)` (`0x00a47560`): a depth-first walk
    /// recording each node's cheapest cost (`u16` at `+0x24` of the node).
    /// The step to the next node (`0x01a86100`) is 1 when the link's
    /// `+0xc` byte matches the mask (`0x01a86104`), else 4; links marked
    /// (bits 0, 1 of `+0xe`) are skipped and marked while walked. Returns
    /// 1 when the node is past the limit (`0x01a860fc`, the low 16 bits
    /// compared) or no cheaper, else 0 (bit 2 of the link's `+0xe` stays
    /// set only when the walk on was not pruned).
    pub fn abmap_calc_move_cost(mem: &mut impl Mem, node: u16, cost: u32) -> u32 {
        let total = cost.wrapping_add(super::MOVE_STEP.get(mem));
        #[allow(clippy::cast_possible_truncation)] // the low half
        let low = total as u16;
        if i32::from(low) > super::MOVE_LIMIT.get(mem) {
            return 1;
        }
        let g = ABMAP.get(mem);
        let n = u32::from(node) * layout::NODE_SIZE;
        let best = g.wrapping_add(n).wrapping_add(0x82c);
        if mem.read::<u16>(best) <= low {
            return 1;
        }
        mem.write(best, low);
        let links = ABMAP.get(mem).wrapping_add(0x814).wrapping_add(n);
        for k in 0..5 {
            let link = mem.read::<u32>(links + 4 * k);
            if link == 0 {
                return 0;
            }
            let flags = mem.read::<u8>(link + 0xe);
            if flags & 3 != 0 {
                continue;
            }
            let hit = mem.read::<u8>(link + 0xc) & super::MOVE_MASK.get(mem) != 0;
            super::MOVE_STEP.set(mem, if hit { 1 } else { 4 });
            mem.write(link + 0xe, flags | 5);
            let a = mem.read::<u16>(link);
            let next = if a == node {
                mem.read::<u16>(link + 2)
            } else {
                a
            };
            if abmap_calc_move_cost(mem, next, total) != 0 {
                let f = mem.read::<u8>(link + 0xe);
                mem.write(link + 0xe, f & 0xfb);
            }
            let f = mem.read::<u8>(link + 0xe);
            mem.write(link + 0xe, f & 0xfe);
        }
        0
    }

    /// `AbmapCtrlTilt()` (`0x00a48f50`): with the pad's `+0x2700` (`i16`,
    /// `0x01a86108`) negative, buttons (`+0x1166e` bit 1 up, bit 0 down)
    /// step the tilt level (`+0x115cb`, 0..=2, wrapping up, clamping
    /// down) and start a 30-frame change (`+0x115ce`) from the current
    /// tilt (`+0x11340`, as a float at `+0x115d4`) to 0, -3640 or -7281
    /// (`f32`s in `.rdata`, `+0x115d8`). Each frame of a change the tilt
    /// is `from + sin_table[((30 - left) << 14) / 30 >> 4 & 0xfff] * (to -
    /// from)` through `_ftol2_sse`; on the last it is `to` and `+0x115c7`
    /// is set. Returns the `eax` it ends with.
    pub fn abmap_ctrl_tilt(mem: &mut impl Mem) -> u32 {
        let g = ABMAP.get(mem);
        let pad = super::TILT_PAD.get(mem);
        if mem.read::<i16>(pad.wrapping_add(0x2700)) < 0 {
            let b = mem.read::<u16>(g.wrapping_add(0x1166e));
            let lv = g.wrapping_add(0x115cb);
            let step = if b & 2 != 0 {
                let l = mem.read::<u8>(lv).wrapping_add(1);
                mem.write(lv, if l > 2 { 0 } else { l });
                true
            } else if b & 1 != 0 {
                let l = mem.read::<u8>(lv).wrapping_sub(1);
                mem.write(lv, if l > 2 { 2 } else { l });
                true
            } else {
                false
            };
            if step {
                mem.write(g.wrapping_add(0x115ce), 30u16);
                let cur = mem.read::<i32>(g.wrapping_add(0x11340));
                mem.write(g.wrapping_add(0x115d4), x87::store(f64::from(cur)));
                let to = match mem.read::<u8>(lv) {
                    1 => mem.read::<u32>(mem.rebase(super::TILT_LEVELS)),
                    2 => mem.read::<u32>(mem.rebase(super::TILT_LEVELS + 4)),
                    _ => 0,
                };
                mem.write(g.wrapping_add(0x115d8), x87::copy(to));
            }
        }
        let left = mem.read::<i16>(g.wrapping_add(0x115ce));
        if left <= 0 {
            return u32::from(left.cast_unsigned());
        }
        let left = left - 1;
        mem.write(g.wrapping_add(0x115ce), left);
        let to = mem.read::<f32>(g.wrapping_add(0x115d8));
        if left == 0 {
            let v = ftol(mem, x87::load(to.to_bits()));
            mem.write(g.wrapping_add(0x11340), v);
            mem.write(g.wrapping_add(0x115c7), 1u8);
            return g;
        }
        let k = ((30 - i32::from(left)) << 14) / 30;
        #[allow(clippy::cast_sign_loss)] // masked to 12 bits
        let i = ((k >> 4) & 0xfff) as u32;
        let s = mem.read::<f32>(mem.rebase(super::SIN_TABLE) + 4 * i);
        let from = mem.read::<f32>(g.wrapping_add(0x115d4));
        let v = (Reg::load(s) * Reg::load(to).sub_m32(from))
            .add_m32(from)
            .store();
        let r = ftol(mem, x87::load(v.to_bits()));
        mem.write(g.wrapping_add(0x11340), r);
        r
    }

    /// `AbmapZoomScale()` (`0x00a56820`, in `ST0`): the zoom for the level.
    pub const ZOOM_SCALE: u32 = 0x00a5_6820;

    /// `AbmapCtrlZoom()` (`0x00a490c0`): with the pad's `+0x2700` (`i16`)
    /// negative and button bit 8 (`+0x1166e`), the zoom level (`+0x115cc`,
    /// 0..=2) steps up, wrapping, and a 30-frame change (`+0x115d0`) starts
    /// from the zoom (`+0x11350`, kept at `+0x115dc`) to `AbmapZoomScale()`
    /// (`+0x115e0`). Each frame of a change the three zoom words
    /// (`+0x11350..+0x11358`) get `from + (to - from) * sin_table[((30 -
    /// left) << 14) / 30 >> 4 & 0xfff]` (on the x87, stored once); at most
    /// 0.375 (not NaN) sets `+0x116b0` to -2 unless negative, at least 0.375
    /// sets it to 2 unless positive, and `+0x116ac` 0 becomes 1. On the
    /// last frame they are `to` and `+0x115c7` is set. Returns the `eax` it
    /// ends with: `g_Abmap`, or the frame count when no change runs.
    pub fn abmap_ctrl_zoom<M, F>(mem: &mut M, fcall: &mut F) -> u32
    where
        M: Mem,
        F: FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    {
        let g = ABMAP.get(mem);
        let pad = super::TILT_PAD.get(mem);
        let pressed = mem.read::<i16>(pad.wrapping_add(0x2700)) < 0
            && mem.read::<u16>(g.wrapping_add(0x1166e)) & 0x100 != 0;
        if pressed {
            let level = g.wrapping_add(0x115cc);
            let l = mem.read::<u8>(level).wrapping_add(1);
            mem.write(level, if l > 2 { 0 } else { l });
            mem.write(g.wrapping_add(0x115d0), 30u16);
            let zoom = x87::copy(mem.read::<u32>(g.wrapping_add(0x11350)));
            mem.write(g.wrapping_add(0x115dc), zoom);
            let f = mem.rebase(ZOOM_SCALE);
            let to = fcall(mem, f, crate::camera::FArgs::Stack(&[]));
            mem.write(ABMAP.get(mem).wrapping_add(0x115e0), x87::store(to));
        }
        let g = ABMAP.get(mem);
        let left = mem.read::<i16>(g.wrapping_add(0x115d0));
        if left <= 0 {
            return u32::from(left.cast_unsigned());
        }
        let left = left - 1;
        mem.write(g.wrapping_add(0x115d0), left);
        let zooms = [0x11358, 0x11354, 0x11350];
        if left == 0 {
            let to = x87::copy(mem.read::<u32>(g.wrapping_add(0x115e0)));
            for o in zooms {
                mem.write(g.wrapping_add(o), to);
            }
            mem.write(g.wrapping_add(0x115c7), 1u8);
            return g;
        }
        let k = ((30 - i32::from(left)) << 14) / 30;
        #[allow(clippy::cast_sign_loss)] // masked to 12 bits
        let i = ((k >> 4) & 0xfff) as u32;
        let sin = x87::load(mem.read::<u32>(mem.rebase(super::SIN_TABLE) + 4 * i));
        let to = x87::load(mem.read::<u32>(g.wrapping_add(0x115e0)));
        let from = x87::load(mem.read::<u32>(g.wrapping_add(0x115dc)));
        let zoom = x87::store((to - from) * sin + from);
        for o in zooms {
            mem.write(g.wrapping_add(o), zoom);
        }
        let near = x87::load(mem.read::<u32>(mem.rebase(super::ZOOM_NEAR)));
        let z = x87::load(zoom);
        let level = g.wrapping_add(0x116b0);
        // a NaN zoom sets neither
        if z <= near && mem.read::<i32>(level) >= 0 {
            mem.write(level, -2i32);
        }
        if near <= z && mem.read::<i32>(level) <= 0 {
            mem.write(level, 2i32);
        }
        let shown = g.wrapping_add(0x116ac);
        if mem.read::<u32>(shown) == 0 {
            mem.write(shown, 1u32);
        }
        g
    }

    /// Copies the camera's four words (`+0x112b8`) to `+0x11308`; returns
    /// the last.
    fn sync_camera(mem: &mut impl Mem) -> u32 {
        let g = ABMAP.get(mem);
        let c = mem.read::<[u32; 4]>(g.wrapping_add(0x112b8));
        mem.write(g.wrapping_add(0x11308), c);
        c[3]
    }

    /// `AbmapState_HomeOnNode()` (`0x00a59e80`): the homing state: its
    /// progress (`+0x11620`) advances by the rate (`+0x11624`); below 1.0
    /// the camera's x, y move from the start (`+0x1163c`, through
    /// `FFXVu0InterVectorXYZ`) toward the node and the zoom (`+0x112f4`)
    /// from `+0x11628` toward `+0x1162c`; at 1.0 they arrive and the saved
    /// state (`+0x115b0`) comes back. The camera is copied to `+0x11308`.
    /// Returns the `eax` it ends with.
    pub fn abmap_state_home_on_node(mem: &mut impl Mem) -> u32 {
        let g = ABMAP.get(mem);
        let node = u32::from(mem.read::<u16>(g.wrapping_add(0x11634)));
        let n = g
            .wrapping_add(layout::NODES)
            .wrapping_add(node * layout::NODE_SIZE);
        let x = f32::from(mem.read::<i16>(n));
        let y = f32::from(mem.read::<i16>(n.wrapping_add(2)));
        let rate = mem.read::<f32>(g.wrapping_add(0x11624));
        let s = Reg::load(rate)
            .add_m32(mem.read::<f32>(g.wrapping_add(0x11620)))
            .store();
        mem.write(g.wrapping_add(0x11620), s);
        let below = x87::load(s.to_bits()) < 1.0; // fcom: false when unordered
        if !below {
            mem.write(g.wrapping_add(0x112b8), x);
            mem.write(g.wrapping_add(0x112bc), y);
            sync_camera(mem);
            let gg = ABMAP.get(mem);
            let z = mem.read::<u32>(gg.wrapping_add(0x1162c));
            mem.write(g.wrapping_add(0x112f4), x87::copy(z));
            let gg = ABMAP.get(mem);
            let saved = mem.read::<u32>(gg.wrapping_add(0x115b0));
            mem.write(gg.wrapping_add(0x115a8), saved);
            mem.write(gg.wrapping_add(0x115b0), 0u32);
            return gg;
        }
        let start = g.wrapping_add(0x1163c);
        let bx = mem.read::<f32>(start);
        let by = mem.read::<f32>(start.wrapping_add(4));
        let rx = ffx_ps2::vu0::inter_xyz(0, x, bx, s);
        let ry = ffx_ps2::vu0::inter_xyz(1, y, by, s);
        mem.write(g.wrapping_add(0x112b8), x87::copy(rx.to_bits()));
        mem.write(g.wrapping_add(0x112bc), x87::copy(ry.to_bits()));
        let gg = ABMAP.get(mem);
        let to = mem.read::<f32>(gg.wrapping_add(0x1162c));
        let from = mem.read::<f32>(gg.wrapping_add(0x11628));
        let t = mem.read::<f32>(gg.wrapping_add(0x11620));
        let z = Reg::load(to).sub_m32(from).mul_m32(t).add_m32(from).store();
        mem.write(g.wrapping_add(0x112f4), z);
        sync_camera(mem)
    }

    /// The tilt for a level (0, or the `.rdata` levels 1 and 2), as
    /// stored in `+0x115d8` (bits).
    fn tilt_level(mem: &impl Mem, level: u8) -> u32 {
        match level {
            1 => mem.read::<u32>(mem.rebase(super::TILT_LEVELS)),
            2 => mem.read::<u32>(mem.rebase(super::TILT_LEVELS + 4)),
            _ => 0,
        }
    }

    /// `LoadSaveAbmap()` (`0x00a49590`): the saved sphere grid
    /// (`MsGetSaveAbilityMap`) into the grid data: per node with a kind
    /// (not `0xffff`) the kind (`0xff` → `0xffff`) and panel bits, per
    /// link its flags (from `+0xa00`), the seven characters' nodes (from
    /// `+0xf00`, every `0x50` bytes from `+0x110cc`), the tilt level
    /// (`+0xf18`) and zoom level (`+0xf19`); then the tilt (through
    /// `_ftol2_sse`), the zoom (1, 0.5, 0.25, 0.125 into `+0x11350`..58)
    /// and `+0x116b0` (-2 at most 0.375, else 2; `.rdata` `f32`s).
    /// Returns the grid data pointer.
    pub fn load_save_abmap(mem: &mut impl Mem) -> u32 {
        let save = ffx_battle::save::bridge::ms_get_save_ability_map(mem);
        let g = ABMAP.get(mem);
        let mut i = 0i32;
        while i < i32::from(mem.read::<i16>(g.wrapping_add(2))) {
            #[allow(clippy::cast_sign_loss)] // i >= 0
            let k = i as u32;
            let n = g.wrapping_add(k * layout::NODE_SIZE);
            if mem.read::<u16>(n.wrapping_add(0x80e)) != 0xffff {
                let b = mem.read::<u8>(save.wrapping_add(2 * k));
                let kind = if b == 0xff { 0xffff } else { u16::from(b) };
                mem.write(n.wrapping_add(0x80e), kind);
                let p = mem.read::<u8>(save.wrapping_add(2 * k + 1));
                mem.write(n.wrapping_add(0x829), p);
            }
            i += 1;
        }
        let mut i = 0i32;
        while i < i32::from(mem.read::<i16>(g.wrapping_add(layout::LINK_COUNT))) {
            #[allow(clippy::cast_sign_loss)] // i >= 0
            let k = i as u32;
            let f = mem.read::<u8>(save.wrapping_add(0xa00 + k));
            mem.write(g.wrapping_add(0xa814 + k * layout::LINK_SIZE), f);
            i += 1;
        }
        for c in 0..7 {
            let v = mem.read::<u16>(save.wrapping_add(0xf00 + 2 * c));
            mem.write(g.wrapping_add(0x110cc + 0x50 * c), v);
        }
        let tilt = mem.read::<u8>(save.wrapping_add(0xf18));
        mem.write(g.wrapping_add(0x115cb), tilt);
        let zoom = mem.read::<u8>(save.wrapping_add(0xf19));
        mem.write(g.wrapping_add(0x115cc), zoom);
        let t = tilt_level(mem, mem.read::<u8>(g.wrapping_add(0x115cb)));
        let v = ftol(mem, x87::load(t));
        mem.write(g.wrapping_add(0x11340), v);
        let z = match mem.read::<u8>(g.wrapping_add(0x115cc)) {
            k @ 1..=3 => mem.read::<f32>(mem.rebase(super::ZOOM_LEVELS[usize::from(k) - 1])),
            _ => 1.0,
        };
        let z = f32::from_bits(x87::copy(z.to_bits()));
        mem.write(g.wrapping_add(0x11358), z);
        mem.write(g.wrapping_add(0x11354), z);
        mem.write(g.wrapping_add(0x11350), z);
        let lim = mem.read::<f32>(mem.rebase(super::ZOOM_NEAR));
        // fcomp: at most (or unordered) gives -2
        let far = x87::load(z.to_bits()) > f64::from(lim);
        mem.write(
            g.wrapping_add(0x116b0),
            if far { 2 } else { 0xffff_fffe_u32 },
        );
        g
    }

    /// `abmap_test_panel(bit, node)` (`0x00a457d0`): the node's panel
    /// bits masked with `1 << bit` (32-bit shift).
    pub fn abmap_test_panel(mem: &impl Mem, bit: u32, node: u32) -> u32 {
        let g = ABMAP.get(mem);
        let b = mem.read::<u8>(
            g.wrapping_add(0x829)
                .wrapping_add(node.wrapping_mul(layout::NODE_SIZE)),
        );
        u32::from(b) & 1u32.wrapping_shl(bit & 31)
    }

    /// `abmap_other_has_panel(chr, node)` (`0x00a45800`): 1 when a
    /// character other than `chr` among the first seven who has joined
    /// (`MsGetSavePlyJoined` 1) has its panel bit set on the node.
    pub fn abmap_other_has_panel(mem: &impl Mem, chr: u32, node: u32) -> u32 {
        let g = ABMAP.get(mem);
        let b = mem.read::<u8>(
            g.wrapping_add(0x829)
                .wrapping_add(node.wrapping_mul(layout::NODE_SIZE)),
        );
        let hit = (0..7u32).any(|c| {
            c != chr
                && ffx_battle::save::bridge::ms_get_save_ply_joined(mem, c) == 1
                && u32::from(b) & (1 << c) != 0
        });
        u32::from(hit)
    }

    /// `abmap_chr_node(chr)` (`0x00a45870`): the character's node (`u16`
    /// at `+0x110cc + 0x50 * chr`) once joined, else -1.
    pub fn abmap_chr_node(mem: &impl Mem, chr: u32) -> u32 {
        if ffx_battle::save::bridge::ms_get_save_ply_joined(mem, chr) == 0 {
            return u32::MAX;
        }
        let g = ABMAP.get(mem);
        u32::from(mem.read::<u16>(g.wrapping_add(0x110cc).wrapping_add(chr.wrapping_mul(0x50))))
    }

    /// `AbmapTransitionToHoming(node, t)` (`0x00a48d20`):
    /// [`abmap_init_homing`], then with no state saved the homing state
    /// (`AbmapState_HomeOnNode`) replaces the current one, which is saved.
    pub fn abmap_transition_to_homing(mem: &mut impl Mem, node: u32, t: u32) {
        abmap_init_homing(mem, node, x87::copy(t));
        let g = ABMAP.get(mem);
        if mem.read::<u32>(g.wrapping_add(0x115b0)) == 0 {
            let s = mem.read::<u32>(g.wrapping_add(0x115a8));
            mem.write(g.wrapping_add(0x115b0), s);
            let f = mem.rebase(super::STATE_HOME_ON_NODE);
            let g = ABMAP.get(mem);
            mem.write(g.wrapping_add(0x115a8), f);
        }
    }

    /// `AbmapMakeNodeMatrix(out, pos, scale)` (`0x00a5ad30`): a 4x4 matrix
    /// filled from the emulated `r0` register (`crate::vu0::EE_GPR`, four
    /// words, a row at a time), `scale` on the diagonal's first three, the
    /// translation row the `i16` position, 1.0. Returns the `eax` it ends
    /// with: the position's z sign-extended.
    pub fn abmap_make_node_matrix(mem: &mut impl Mem, out: u32, pos: u32, scale: u32) -> u32 {
        let r0 = crate::vu0::EE_GPR.ptr(mem).addr;
        for row in 0..4u32 {
            for i in 0..4u32 {
                let w = mem.read::<u32>(r0 + 4 * i);
                mem.write(out.wrapping_add(16 * row + 4 * i), w);
            }
        }
        let s = x87::copy(scale);
        for at in [0x28u32, 0x14, 0] {
            mem.write(out.wrapping_add(at), s);
        }
        let mut z = 0;
        for i in 0..3u32 {
            z = i32::from(mem.read::<i16>(pos.wrapping_add(2 * i)));
            #[allow(clippy::cast_precision_loss)] // fild, then fstp m32 rounds
            mem.write(out.wrapping_add(0x30 + 4 * i), (z as f32).to_bits());
        }
        mem.write(out.wrapping_add(0x3c), 1.0f32.to_bits());
        z.cast_unsigned()
    }

    /// `AbmapForEachNode(f, arg)` (`0x00a59710`): `f(node, arg)` for the
    /// node count (`+2`, `i16`) of nodes, skipping kind -1, until one
    /// returns non-zero. Returns that value; 0xffff when none did; the data
    /// pointer with a count of 0.
    pub fn abmap_for_each_node<M: Mem>(
        mem: &mut M,
        f: u32,
        arg: u32,
        mut call: impl FnMut(&mut M, u32, u32, u32) -> u32,
    ) -> u32 {
        let g = ABMAP.get(mem);
        let mut n = i32::from(mem.read::<i16>(g.wrapping_add(2)));
        let mut node = g.wrapping_add(layout::NODES);
        if n == 0 {
            return g;
        }
        loop {
            n = n.wrapping_sub(1);
            if mem.read::<u16>(node.wrapping_add(6)) != 0xffff {
                let r = call(mem, f, node, arg);
                if r != 0 {
                    return r;
                }
            }
            node = node.wrapping_add(layout::NODE_SIZE);
            if n == 0 {
                return 0xffff;
            }
        }
    }

    /// `AbmapForNodeAndLinked(node, f, arg)` (`0x00a59760`): `f(node, arg)`,
    /// then for each link (`0x14` bytes from `+0xa808`, its two `u16` ends)
    /// that touches the node, `f(other end, arg)`, until one returns
    /// non-zero. Returns that value, else where the link scan stopped.
    pub fn abmap_for_node_and_linked<M: Mem>(
        mem: &mut M,
        node: u16,
        f: u32,
        arg: u32,
        mut call: impl FnMut(&mut M, u32, u32, u32) -> u32,
    ) -> u32 {
        let at = |g: u32, n: u16| {
            g.wrapping_add(layout::NODES)
                .wrapping_add(u32::from(n) * layout::NODE_SIZE)
        };
        let g = ABMAP.get(mem);
        let r = call(mem, f, at(g, node), arg);
        if r != 0 {
            return r;
        }
        let mut last = 0u32;
        loop {
            let g = ABMAP.get(mem);
            let count = i32::from(mem.read::<i16>(g.wrapping_add(layout::LINK_COUNT)));
            let end = g.wrapping_add(
                count
                    .wrapping_mul(5)
                    .wrapping_add(0x2a02)
                    .cast_unsigned()
                    .wrapping_mul(4),
            );
            let mut l = if last == 0 {
                g.wrapping_add(0xa808)
            } else {
                last.wrapping_add(layout::LINK_SIZE)
            };
            let other = loop {
                if l >= end {
                    return l;
                }
                let (a, b) = (mem.read::<u16>(l), mem.read::<u16>(l.wrapping_add(2)));
                if a == node {
                    break b;
                }
                if b == node {
                    break a;
                }
                l = l.wrapping_add(layout::LINK_SIZE);
            };
            last = l;
            let r = call(mem, f, at(g, other), arg);
            if r != 0 {
                return r;
            }
        }
    }

    /// `AbmapResetHelp()` (`0x00a5b930`): clears `+0x115c4` and `+0x115c3`,
    /// the state to `0x00a45010` and the message hook to
    /// `AbmapSetHelpMessage`. Returns the data pointer.
    pub fn abmap_reset_help(mem: &mut impl Mem) -> u32 {
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x115c4), 0u8);
        let s = mem.rebase(super::STATE_HELP);
        mem.write(g.wrapping_add(0x115a8), s);
        let h = mem.rebase(super::SET_HELP_MESSAGE);
        mem.write(g.wrapping_add(0x115ac), h);
        mem.write(g.wrapping_add(0x115c3), 0u8);
        g
    }

    /// `AbmapPulseCursor()` (`0x00a47440`): the sine of the frame counter
    /// (`+0x11698`, 32 steps) to `+0x115a4`, and `+0x115c5` to
    /// `((sin + 1) * 8 + 0x70) * +0x115c6 >> 7` (truncated). Returns the
    /// byte before the scaling.
    pub fn abmap_pulse_cursor(mem: &mut impl Mem) -> u32 {
        let g = ABMAP.get(mem);
        let i = mem.read::<u32>(g.wrapping_add(0x11698)) & 0x1f;
        let table = mem.rebase(super::SIN_TABLE);
        let s = x87::copy(mem.read::<u32>(table + (i << 9)));
        mem.write(g.wrapping_add(0x115a4), s);
        let one = mem.read::<f64>(mem.rebase(super::PULSE_ONE));
        let eight = mem.read::<f64>(mem.rebase(super::PULSE_SCALE));
        let v = f64::from(f32::from_bits(x87::store(ffx_bridge::x87::load(s) + one)));
        let k = ffx_bridge::x87::cvttsd2si(v * eight);
        #[allow(clippy::cast_possible_truncation)] // the low byte
        let b = (k as u8).wrapping_add(0x70);
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x115c5), b);
        let m = u32::from(mem.read::<u8>(g.wrapping_add(0x115c6))) * u32::from(b);
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(g.wrapping_add(0x115c5), (m >> 7) as u8);
        u32::from(b)
    }

    /// `AbmapSetChrMarker(chr)` (`0x00a5a990`): the character's marker
    /// (`0x50` bytes at `+0x11088`) placed on its node (`+0x44`): x, y,
    /// 0, 1, and `+0x3c` half its kind's size + 3.0. Returns the marker.
    pub fn abmap_set_chr_marker(mem: &mut impl Mem, chr: u32) -> u32 {
        let g = ABMAP.get(mem);
        let m = g.wrapping_add(0x11088).wrapping_add(chr.wrapping_mul(0x50));
        let node = g
            .wrapping_add(layout::NODES)
            .wrapping_add(u32::from(mem.read::<u16>(m.wrapping_add(0x44))) * layout::NODE_SIZE);
        for i in 0..2u32 {
            let v = i32::from(mem.read::<i16>(node.wrapping_add(2 * i)));
            #[allow(clippy::cast_precision_loss)] // fild, then fstp m32 rounds
            mem.write(m.wrapping_add(4 * i), (v as f32).to_bits());
        }
        mem.write(m.wrapping_add(8), 0.0f32.to_bits());
        mem.write(m.wrapping_add(0xc), 1.0f32.to_bits());
        let g = ABMAP.get(mem);
        let node = g
            .wrapping_add(layout::NODES)
            .wrapping_add(u32::from(mem.read::<u16>(m.wrapping_add(0x44))) * layout::NODE_SIZE);
        let kind = u32::from(mem.read::<u16>(node.wrapping_add(6)));
        let size = mem.read::<i16>(
            g.wrapping_add(layout::KIND_SIZE)
                .wrapping_add(kind * layout::KIND_STRIDE),
        ) >> 1;
        let margin = super::HOMING_MARGIN.get(mem);
        mem.write(m.wrapping_add(0x3c), x87::store(f64::from(size) + margin));
        m
    }

    /// `0x00a5cee0` and `0x00a5cf10` (two copies, `(_, n)`): `n / 4`
    /// (toward zero), or 1 when that is 0.
    #[must_use]
    pub fn abmap_quarter_or_one(n: i32) -> u32 {
        let q = n / 4;
        if q == 0 { 1 } else { q.cast_unsigned() }
    }

    /// `AbmapMakeLinkMatrix(out, a, b, scale)` (`0x00a5a360`): as
    /// [`abmap_make_node_matrix`], the translation the sum of two `i16`
    /// (x, y) positions, z 0.0, 1.0. Returns `b`'s y sign-extended.
    pub fn abmap_make_link_matrix(mem: &mut impl Mem, out: u32, a: u32, b: u32, scale: u32) -> u32 {
        let r0 = crate::vu0::EE_GPR.ptr(mem).addr;
        for row in 0..4u32 {
            for i in 0..4u32 {
                let w = mem.read::<u32>(r0 + 4 * i);
                mem.write(out.wrapping_add(16 * row + 4 * i), w);
            }
        }
        let s = x87::copy(scale);
        for at in [0x28u32, 0x14, 0] {
            mem.write(out.wrapping_add(at), s);
        }
        let mut y = 0;
        for i in 0..2u32 {
            let p = i32::from(mem.read::<i16>(a.wrapping_add(2 * i)));
            y = i32::from(mem.read::<i16>(b.wrapping_add(2 * i)));
            mem.write(
                out.wrapping_add(0x30 + 4 * i),
                x87::store(f64::from(p) + f64::from(y)),
            );
        }
        mem.write(out.wrapping_add(0x38), 0u32);
        mem.write(out.wrapping_add(0x3c), 1.0f32.to_bits());
        y.cast_unsigned()
    }

    /// `AbmapVu0LoadMatrix1(m)` (`0x00a5be30`): the 4x4 matrix into the
    /// emulated VU0 registers `vf01`..`vf04` (`0x00c8f7a0`), as `lqc2`
    /// would. Returns the last word.
    pub fn abmap_vu0_load_matrix1(mem: &mut impl Mem, m: u32) -> u32 {
        load_vf_matrix(mem, m, super::VU0_MATRIX1)
    }

    /// `AbmapVu0LoadMatrix2(m)` (`0x00a5bec0`): as
    /// [`abmap_vu0_load_matrix1`] into `vf05`..`vf08` (`0x00c8f7e0`).
    pub fn abmap_vu0_load_matrix2(mem: &mut impl Mem, m: u32) -> u32 {
        load_vf_matrix(mem, m, super::VU0_MATRIX2)
    }

    fn load_vf_matrix(mem: &mut impl Mem, m: u32, vf: u32) -> u32 {
        let to = mem.rebase(vf);
        let mut last = 0;
        for i in 0..16u32 {
            last = mem.read::<u32>(m.wrapping_add(4 * i));
            mem.write(to + 4 * i, last);
        }
        last
    }

    // ---- round 12 ----

    /// `AbmapResetCursorCamera()` (`0x00a5b030`): the cursor distance
    /// (`+0x115a0`, [`super::CURSOR_DISTANCE`]), the current character's
    /// position (`+0x11088 + 0x50 * +0x115bc`) to `+0x11520` with y moved
    /// down by [`super::CURSOR_LIFT`] and w twice the distance, that
    /// character's colour ([`super::CHR_COLORS`]) to `+0x11560`; then
    /// tail-calls `AbmapPulseCursor` and returns its result.
    pub fn abmap_reset_cursor_camera(mem: &mut impl Mem) -> u32 {
        let src = ABMAP.get(mem);
        let chr = u32::from(mem.read::<u8>(src.wrapping_add(0x115bc)));
        let d = x87::copy(mem.read(mem.rebase(super::CURSOR_DISTANCE)));
        mem.write(src.wrapping_add(0x115a0), d);
        let pos = src.wrapping_add(0x11088 + chr * 0x50);
        let p = mem.read::<[u32; 4]>(pos);
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x11520), p);
        let lift = mem.read::<f64>(mem.rebase(super::CURSOR_LIFT));
        let y = x87::store(x87::load(mem.read(pos.wrapping_add(8))) - lift);
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x11528), y);
        let g = ABMAP.get(mem);
        let chr = u32::from(mem.read::<u8>(g.wrapping_add(0x115bc)));
        let c = mem.read::<[u32; 4]>(mem.rebase(super::CHR_COLORS) + chr * 0x10);
        mem.write(g.wrapping_add(0x11560), c);
        let g = ABMAP.get(mem);
        let d = x87::load(mem.read(g.wrapping_add(0x115a0)));
        mem.write(g.wrapping_add(0x1152c), x87::store(d + d));
        abmap_pulse_cursor(mem)
    }

    /// `AbmapStoreSave()` (`0x00a5bb70`): the sphere grid's progress into
    /// `MsGetSaveAbilityMap`'s block: each node's two bytes (`+0x80e`,
    /// `+0x829` of the 0x28-byte nodes) as pairs, each link's flags byte
    /// (`+0xa814`, 0x14 apart) from `+0xa00`, the seven characters'
    /// `u16`s (`+0x110cc`, 0x50 apart) at `+0xf00` and the bytes
    /// `+0x115cb`/`+0x115cc` at `+0xf18`. Returns the last byte.
    pub fn abmap_store_save<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::MS_GET_SAVE_ABILITY_MAP);
        let out = call(mem, f, &[]);
        let mut i = 0u32;
        while i32::from(mem.read::<i16>(ABMAP.get(mem).wrapping_add(2))) > i.cast_signed() {
            let g = ABMAP.get(mem);
            let n = g.wrapping_add(i * 0x28);
            let a = mem.read::<u8>(n.wrapping_add(0x80e));
            mem.write(out.wrapping_add(i * 2), a);
            let b = mem.read::<u8>(n.wrapping_add(0x829));
            mem.write(out.wrapping_add(i * 2 + 1), b);
            i += 1;
        }
        let mut i = 0u32;
        while i32::from(mem.read::<i16>(ABMAP.get(mem).wrapping_add(4))) > i.cast_signed() {
            let g = ABMAP.get(mem);
            let a = mem.read::<u8>(g.wrapping_add(0xa814 + i * 0x14));
            mem.write(out.wrapping_add(0xa00 + i), a);
            i += 1;
        }
        for k in 0..7u32 {
            let g = ABMAP.get(mem);
            let v = mem.read::<u16>(g.wrapping_add(0x110cc + k * 0x50));
            mem.write(out.wrapping_add(0xf00 + k * 2), v);
        }
        let g = ABMAP.get(mem);
        let a = mem.read::<u8>(g.wrapping_add(0x115cb));
        mem.write(out.wrapping_add(0xf18), a);
        let g = ABMAP.get(mem);
        let b = mem.read::<u8>(g.wrapping_add(0x115cc));
        mem.write(out.wrapping_add(0xf19), b);
        u32::from(b)
    }

    /// `AbmapSetupPlyTag(tag)` (`0x00a47c60`): the tag's position
    /// (`+0x30`, `+0x32`) and width 0x12 (`+0x38`) into the tag globals
    /// ([`super::PLY_TAG`]), its screen size through the zoom
    /// (`+0x11350`): `ftol((x + 0x12) / zoom)` and `ftol((y + 2.0) /
    /// zoom)`, the second also halved into `+0x34` and kept at `+0x36`.
    /// Returns the original's `eax`: the high half of `(y + 4) << 4`, the
    /// low half of the second size.
    pub fn abmap_setup_ply_tag(mem: &mut impl Mem, tag: u32) -> u32 {
        let t = super::PLY_TAG;
        mem.write(tag.wrapping_add(0x38), 0x12u16);
        let g = ABMAP.get(mem);
        let x = i32::from(mem.read::<i16>(tag.wrapping_add(0x30)));
        mem.write(mem.rebase(t + 0x2c), x);
        let y = i32::from(mem.read::<i16>(tag.wrapping_add(0x32)));
        mem.write(mem.rebase(t + 0x28), y);
        let w = x.wrapping_add(i32::from(mem.read::<i16>(tag.wrapping_add(0x38))));
        let wf = x87::load(x87::store(f64::from(w)));
        let zoom = x87::load(mem.read(g.wrapping_add(0x11350)));
        let a = ftol(mem, wf / zoom);
        mem.write(mem.rebase(t + 0x24), a);
        let y = mem.read::<i32>(mem.rebase(t + 0x28));
        let yf = x87::load(x87::store(f64::from(y)));
        let two = mem.read::<f64>(mem.rebase(super::TWO_F64));
        let zoom = x87::load(mem.read(g.wrapping_add(0x11350)));
        let b = ftol(mem, (yf + two) / zoom);
        let c = mem.read::<i32>(mem.rebase(t + 0x28));
        let h = c.wrapping_add(4);
        mem.write(mem.rebase(t + 0x14), (h / 2) << 4);
        mem.write(mem.rebase(t + 0x20), b);
        mem.write(mem.rebase(t + 0x10), h << 4);
        mem.write(mem.rebase(t + 0x1c), 0u32);
        mem.write(mem.rebase(t + 0x18), 0u32);
        mem.write(mem.rebase(t + 0xc), 0xbu32);
        mem.write(mem.rebase(t + 0x8), 0xau32);
        mem.write(mem.rebase(t + 0x4), 0xffff_ffeeu32);
        mem.write(mem.rebase(t), 0u32);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the low half
        mem.write(tag.wrapping_add(0x34), (b.cast_signed() >> 1) as u16);
        let lo = mem.read::<u16>(mem.rebase(t + 0x20));
        mem.write(tag.wrapping_add(0x36), lo);
        (h.cast_unsigned().wrapping_shl(4) & 0xffff_0000) | u32::from(lo)
    }

    /// `AbmapScaledLevelValue(level, i, j)` (`0x00a5cbe0`): 100000 for `j`
    /// 4, else `ftol(T[i] * (level + 50) * T[j])` for `T` = 1, 1, 1.5, 3,
    /// 5 ([`super::LEVEL_SCALES`]: 1.5, 3, 5 from `.rdata`). Indices past
    /// the original's two local tables (`i` past 4, `j` past 9) read its
    /// stack; this gives 0 for them.
    pub fn abmap_scaled_level_value(mem: &impl Mem, level: i32, i: u32, j: u32) -> u32 {
        if j == 4 {
            return 100_000;
        }
        let [a, b, c] = super::LEVEL_SCALES.map(|at| mem.read::<u32>(mem.rebase(at)));
        let one = 1f32.to_bits();
        // the locals from ebp-0x2c: the j table, then the i table
        let both = [one, one, a, b, c, one, one, a, b, c];
        let (Some(&ti), Some(&tj)) = (both.get(5 + i as usize), both.get(j as usize)) else {
            return 0;
        };
        let l = x87::load(x87::store(f64::from(level.wrapping_add(0x32))));
        ftol(mem, x87::load(ti) * l * x87::load(tj))
    }

    /// `abmap_snd_chg_panel0()` (`0x00a45910`) and `abmap_snd_chg_panel1`
    /// (`0x00a45920`): `SndSepPlaySimple` of sound 0x8000006d or
    /// 0x8000006e; returns its `eax`.
    pub fn abmap_snd_chg_panel<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        which: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::SND_SEP_PLAY_SIMPLE);
        call(mem, f, &[0x8000_006d + which])
    }

    // ---- round 18 ----

    /// `AbmapSetKindSizes(src)` (`0x00a5adf0`): for the eight node kinds
    /// (ids at [`super::KIND_IDS`]), `op_oef2_a_get(src, 2, id)`'s `u16`s
    /// `+0xc`/`+0xe` into the grid's kind sizes (`+0xf808`, four bytes
    /// each). Returns the last `eax`: the record's high half and the last
    /// `u16`.
    pub fn abmap_set_kind_sizes<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        src: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::OP_OEF2_A_GET);
        let mut r = 0;
        for i in 0..8u32 {
            let id = u32::from(mem.read::<u8>(mem.rebase(super::KIND_IDS) + i));
            let g = ABMAP.get(mem);
            let rec = call(mem, f, &[src, 2, id]);
            let a = mem.read::<u16>(rec.wrapping_add(0xc));
            mem.write(g.wrapping_add(0xf808 + 4 * i), a);
            let b = mem.read::<u16>(rec.wrapping_add(0xe));
            mem.write(g.wrapping_add(0xf80a + 4 * i), b);
            r = (rec & 0xffff_0000) | u32::from(b);
        }
        r
    }

    /// `AbmapMarkLockedEntries(pad)` (`0x00a5a2e0`): for each of the menu
    /// pad's entries (`g_AbmapPad + 0x340 * pad`, count `+0x1e`, 12-byte
    /// entries from `+0x44`), byte 0 set when the current character's save
    /// (`MsGetSavePlayerPtr`) lacks the bit the entry's word `+4` names
    /// (in the `u16`s from `+0x3e`). Returns the count, 0 without entries.
    pub fn abmap_mark_locked_entries<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        pad: u32,
        call: &mut C,
    ) -> u32 {
        let chr = u32::from(mem.read::<u8>(ABMAP.get(mem).wrapping_add(0x115bc)));
        let p = pad
            .wrapping_mul(0x340)
            .wrapping_add(mem.read::<u32>(mem.rebase(super::ABMAP_PAD)));
        let f = mem.rebase(super::MS_GET_SAVE_PLAYER_PTR);
        let save = call(mem, f, &[chr]);
        let mut r = 0;
        let mut i = 0i32;
        while i32::from(mem.read::<i16>(p.wrapping_add(0x1e))) > i {
            let e = p.wrapping_add(0x44).wrapping_add(0xc * i.cast_unsigned());
            let bit = mem.read::<u32>(e.wrapping_add(4));
            let w =
                u32::from(mem.read::<u16>(save.wrapping_add(0x3e).wrapping_add((bit >> 4) * 2)));
            mem.write(e, u8::from(w & (1 << (bit & 0xf)) == 0));
            r = i32::from(mem.read::<i16>(p.wrapping_add(0x1e))).cast_unsigned();
            i += 1;
        }
        r
    }

    /// `AbmapLinkCells(link)` (`0x00a5a760`): each end node's cell on the
    /// grid's 20-wide map: `(x + 0xa00) / 256 + (y + 0x920) / 256 * 20`
    /// (`+8` for end 0, `+0xa` for end 1). Returns the second cell as
    /// the full 32-bit value.
    pub fn abmap_link_cells(mem: &mut impl Mem, link: u32) -> u32 {
        let mut r = 0;
        for (end, out) in [(0u32, 8u32), (2, 0xa)] {
            let n = u32::from(mem.read::<u16>(link.wrapping_add(end)));
            let node = ABMAP.get(mem).wrapping_add(n * 0x28);
            let y = i32::from(mem.read::<i16>(node.wrapping_add(0x80a))) + 0x920;
            let x = i32::from(mem.read::<i16>(node.wrapping_add(0x808))) + 0xa00;
            let c = (x / 256).wrapping_add((y / 256).wrapping_mul(20));
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the low half
            mem.write(link.wrapping_add(out), c as u16);
            r = c.cast_unsigned();
        }
        r
    }

    /// `AbmapCollectNodeLinks()` (`0x00a5b140`): for each node in use
    /// (`+6` not -1), the links (`+0xa808`, 0x14 each) touching it into its
    /// five slots (`+0xc`), the rest cleared; a sixth link and on still go
    /// into the following words (the original's overflow, after a
    /// `dbgPrintf`). Returns the node pointer past the last.
    pub fn abmap_collect_node_links(mem: &mut impl Mem) -> u32 {
        let mut g = ABMAP.get(mem);
        let mut nd = g.wrapping_add(0x808);
        let mut n = 0u32;
        while i32::from(mem.read::<i16>(g.wrapping_add(2))) > n.cast_signed() {
            if mem.read::<u16>(nd.wrapping_add(6)) != 0xffff {
                let mut count = 0u32;
                let mut last = 0u32;
                let mut slot = nd.wrapping_add(0xc);
                loop {
                    let links = i32::from(mem.read::<i16>(g.wrapping_add(4)));
                    let end = g.wrapping_add(((links * 5 + 0x2a02) * 4).cast_unsigned());
                    let mut l = if last == 0 {
                        g.wrapping_add(0xa808)
                    } else {
                        last.wrapping_add(0x14)
                    };
                    let mut found = None;
                    while l < end {
                        #[allow(clippy::cast_possible_truncation)] // `cmp word`
                        let id = n as u16;
                        if mem.read::<u16>(l) == id || mem.read::<u16>(l.wrapping_add(2)) == id {
                            found = Some(l);
                            break;
                        }
                        l = l.wrapping_add(0x14);
                    }
                    let Some(l) = found else {
                        for k in count..5 {
                            mem.write(nd.wrapping_add(0xc + 4 * k), 0u32);
                        }
                        g = ABMAP.get(mem);
                        break;
                    };
                    last = l;
                    mem.write(slot, l);
                    count += 1;
                    slot = slot.wrapping_add(4);
                    g = ABMAP.get(mem);
                }
            }
            nd = nd.wrapping_add(0x28);
            n += 1;
        }
        nd
    }

    // ---- round 19: item effects over the sphere grid ----

    /// The item lookup shared by the item iterators: the item's ability
    /// record (`MsGetRomItem`, its byte `+0x5d`), then its excel row
    /// (`MsGetExcelData` from the table at `0x01a860e4`), or 0 for 0xff.
    fn item_excel<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        item: u32,
        tmp: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::MS_GET_ROM_ITEM);
        let rec = call(mem, f, &[item, tmp]);
        let k = mem.read::<u8>(rec.wrapping_add(0x5d));
        if k == 0xff {
            return 0;
        }
        let table = mem.read::<u32>(mem.rebase(super::ITEM_EXCEL_TABLE));
        let f = mem.rebase(super::MS_GET_EXCEL_DATA);
        call(mem, f, &[u32::from(k), table, tmp])
    }

    /// Runs `cb` over every node (an excel row whose byte `+0xc` is `' '`,
    /// `AbmapForEachNode`) or over `node` and its linked nodes
    /// (`AbmapForNodeAndLinked`) with the context `ctx`; returns the
    /// context's result (`+0xc`).
    fn item_walk<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        node: u32,
        cb: u32,
        ctx: u32,
        call: &mut C,
    ) -> u32 {
        let excel = mem.read::<u32>(ctx);
        if excel != 0 {
            let cb = mem.rebase(cb);
            if mem.read::<u8>(excel.wrapping_add(0xc)) == 0x20 {
                let f = mem.rebase(super::ABMAP_FOR_EACH_NODE);
                call(mem, f, &[cb, ctx]);
            } else {
                let f = mem.rebase(super::ABMAP_FOR_NODE_AND_LINKED);
                call(mem, f, &[node, cb, ctx]);
            }
        }
        mem.read::<u32>(ctx.wrapping_add(0xc))
    }

    /// `AbmapItemUsableAt(chr, node, item, work)` (`0x00a49310`): 0 when
    /// the item is not held (`MsGetSaveItemNum`); else the abilities the
    /// item would grant `chr` there, collected by [`abmap_item_node_effect`]
    /// (`0x00a49440`) over the nodes the item reaches. `work` holds the
    /// original's locals: the context (excel row, `chr`, `1 << chr`, the
    /// result) and a scratch word at `+0x14`.
    pub fn abmap_item_usable_at<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 3],
        work: u32,
        call: &mut C,
    ) -> u32 {
        let [chr, node, item] = args;
        let f = mem.rebase(super::MS_GET_SAVE_ITEM_NUM);
        let held = call(mem, f, &[item]);
        if held == 0 {
            return 0;
        }
        mem.write(work.wrapping_add(4), chr);
        mem.write(work.wrapping_add(0xc), 0u32);
        mem.write(work.wrapping_add(8), 1u32.wrapping_shl(chr));
        let excel = item_excel(mem, item, work.wrapping_add(0x14), call);
        mem.write(work, excel);
        item_walk(mem, node, super::ITEM_NODE_EFFECT, work, call)
    }

    /// `AbmapItemApplyAt(chr, node, item, mark, work)` (`0x00a5b400`): as
    /// [`abmap_item_usable_at`] without the held check and with the mark
    /// byte (`+0x10` of the context), through `0x00a5b490`
    /// ([`abmap_item_apply_node`]).
    pub fn abmap_item_apply_at<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 4],
        work: u32,
        call: &mut C,
    ) -> u32 {
        let [chr, node, item, mark] = args;
        mem.write(work.wrapping_add(4), chr);
        mem.write(work.wrapping_add(0xc), 0u32);
        mem.write(work.wrapping_add(8), 1u32.wrapping_shl(chr));
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        mem.write(work.wrapping_add(0x10), mark as u8);
        // the original's scratch is the mark's own argument slot
        mem.write(work.wrapping_add(0x14), mark);
        let excel = item_excel(mem, item, work.wrapping_add(0x14), call);
        mem.write(work, excel);
        item_walk(mem, node, super::ITEM_APPLY_NODE, work, call)
    }

    /// `AbmapItemNodeEffect(node, ctx, tmp)` (`0x00a49440`): the node's
    /// excel row (`MsGetExcelData` of its id `+6`, table `0x01a860e0`).
    /// With the item's special kind (`*ctx +0xd`) the abilities
    /// `0x00a5d120(kind, node index, row, chr)` returns; else, unless the
    /// node is already activated for the character (`+0x21 & ctx[2]`) or
    /// the row is a hidden one (bit 15 of `+0x10`), the item's ability
    /// bits (`*ctx +0xa`) present in the row. Either is OR-ed into
    /// `ctx[3]`, which is returned.
    pub fn abmap_item_node_effect<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        node: u32,
        ctx: u32,
        tmp: u32,
        call: &mut C,
    ) -> u32 {
        let id = u32::from(mem.read::<u16>(node.wrapping_add(6)));
        let table = mem.read::<u32>(mem.rebase(super::NODE_EXCEL_TABLE));
        let f = mem.rebase(super::MS_GET_EXCEL_DATA);
        let row = call(mem, f, &[id, table, tmp]);
        let item = mem.read::<u32>(ctx);
        let kind = mem.read::<u8>(item.wrapping_add(0xd));
        let res = ctx.wrapping_add(0xc);
        if kind != 0 {
            let g = ABMAP.get(mem);
            let idx = node.wrapping_sub(g).wrapping_sub(0x808).cast_signed() / 0x28;
            let chr = mem.read::<u32>(ctx.wrapping_add(4));
            let f = mem.rebase(super::ABMAP_SPECIAL_EFFECT);
            let r = call(mem, f, &[u32::from(kind), idx.cast_unsigned(), row, chr]);
            let v = mem.read::<u32>(res) | r;
            mem.write(res, v);
            return v;
        }
        let act = mem.read::<u8>(node.wrapping_add(0x21));
        if mem.read::<u8>(ctx.wrapping_add(8)) & act == 0 {
            let bits = u32::from(mem.read::<u16>(row.wrapping_add(0x10)));
            if bits & 0x8000 == 0 {
                let want = u32::from(mem.read::<u16>(item.wrapping_add(0xa)));
                let v = mem.read::<u32>(res) | (want & bits);
                mem.write(res, v);
            }
        }
        mem.read::<u32>(res)
    }

    /// `AbmapItemApplyNode(node, ctx, tmp)` (`0x00a5b490`): with any
    /// effect ([`abmap_item_node_effect`]) the node's byte `+0x22` gains the
    /// context's mark (`+0x10`) and the result is cleared. Returns 0.
    pub fn abmap_item_apply_node<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        node: u32,
        ctx: u32,
        tmp: u32,
        call: &mut C,
    ) -> u32 {
        if abmap_item_node_effect(mem, node, ctx, tmp, call) != 0 {
            let m = mem.read::<u8>(ctx.wrapping_add(0x10));
            let v = mem.read::<u8>(node.wrapping_add(0x22)) | m;
            mem.write(node.wrapping_add(0x22), v);
            mem.write(ctx.wrapping_add(0xc), 0u32);
        }
        0
    }

    /// `AbmapSetupKindShapes(tmp)` (`0x00a5b230`): for each of the 130
    /// node kinds (0x30 bytes from the grid's `+0xf828`), from its excel
    /// row (`MsGetExcelData`, table `0x01a860e0`; `tmp` the original's
    /// scratch): the two shape records its byte `+0x16` selects
    /// (`0x01a85f98`, `0x01a85fe8`, each plus its `+0x10` offset), the
    /// size from the first one's box (`+0x2c - +0x20`, `+0x2e - +0x22`,
    /// over 16; 16 x 16 without one), the size class's shape
    /// (`0x0230578c`/`90`/`94` for under 24, under 32, else), the scale
    /// `size * 7.8125e-5` and, for visible rows (`+0x10` set without bit
    /// 15), seven points of a circle of radius `size / 2 + 3` (sine table
    /// steps of 9362.29), else `+0xf83c` 0x1000. Returns 130.
    #[allow(clippy::too_many_lines)] // the original's steps
    pub fn abmap_setup_kind_shapes<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        tmp: u32,
        call: &mut C,
    ) -> u32 {
        let table = mem.rebase(super::KIND_SHAPE_TABLES);
        let sin = mem.rebase(super::SIN_TABLE);
        for i in 0..130u32 {
            let g = ABMAP.get(mem);
            let k = g.wrapping_add(0xf828 + 0x30 * i);
            let excel = mem.read::<u32>(mem.rebase(super::NODE_EXCEL_TABLE));
            let f = mem.rebase(super::MS_GET_EXCEL_DATA);
            let row = call(mem, f, &[i, excel, tmp]);
            for (j, base) in [(0u32, 0u32), (4, 0x50)] {
                let id = u32::from(mem.read::<u8>(row.wrapping_add(0x16)));
                let a = mem.read::<u32>(table.wrapping_add(base + 4 * id));
                let off = i32::from(mem.read::<i16>(a.wrapping_add(0x10)));
                mem.write(k.wrapping_add(j), a.wrapping_add(off.cast_unsigned()));
            }
            let d = mem.read::<u32>(k);
            if d == 0 {
                mem.write(k.wrapping_add(0xc), 0x0010_0010u32);
            } else {
                let w = (i32::from(mem.read::<i16>(d.wrapping_add(0x2c)))
                    - i32::from(mem.read::<i16>(d.wrapping_add(0x20))))
                    >> 4;
                #[allow(clippy::cast_possible_truncation)] // the low half
                mem.write(k.wrapping_add(0xc), w as i16);
                let h = (i32::from(mem.read::<i16>(d.wrapping_add(0x2e)))
                    - i32::from(mem.read::<i16>(d.wrapping_add(0x22))))
                    >> 4;
                #[allow(clippy::cast_possible_truncation)] // the low half
                mem.write(k.wrapping_add(0xe), h as i16);
            }
            let size = mem.read::<i16>(k.wrapping_add(0xc));
            let class = if size < 0x18 {
                0
            } else if size < 0x20 {
                1
            } else {
                2
            };
            let p = mem.read::<u32>(mem.rebase(super::SIZE_CLASS_SHAPES) + 4 * class);
            let off = i32::from(mem.read::<i16>(p.wrapping_add(0x10)));
            mem.write(k.wrapping_add(8), p.wrapping_add(off.cast_unsigned()));
            let s = x87::load(x87::store(f64::from(size)));
            let scale = mem.read::<f64>(mem.rebase(super::KIND_SCALE));
            mem.write(k.wrapping_add(0x10), x87::store(s * scale));
            let flags = mem.read::<u16>(row.wrapping_add(0x10));
            if flags == 0 || flags & 0x8000 != 0 {
                mem.write(k.wrapping_add(0x14), 0x1000u16);
                continue;
            }
            let three = mem.read::<f64>(mem.rebase(super::KIND_RADIUS_PAD));
            let step = mem.read::<f64>(mem.rebase(super::KIND_ANGLE_STEP));
            let r = x87::load(x87::store(f64::from(size >> 1) + three));
            let mut a = 0f32.to_bits();
            for n in 0..7u32 {
                let ang = ftol(mem, x87::load(a)).cast_signed();
                let i0 = ((ang >> 4) & 0xfff).cast_unsigned();
                let x = ftol(mem, x87::load(mem.read(sin + 4 * i0)) * r);
                let at = k.wrapping_add(0x14 + 4 * n);
                #[allow(clippy::cast_possible_truncation)] // the low half
                mem.write(at, x as u16);
                let i1 = ((ang.wrapping_add(0x4000) >> 4) & 0xfff).cast_unsigned();
                let y = ftol(mem, x87::load(mem.read(sin + 4 * i1)) * r);
                #[allow(clippy::cast_possible_truncation)] // the low half
                mem.write(at.wrapping_add(2), (y as u16).wrapping_neg());
                a = x87::store(x87::load(a) + step);
            }
        }
        130
    }

    // ---- round 20 ----

    /// A pad (`g_AbmapPad + 0x340 * pad`).
    fn pad_at(mem: &impl Mem, pad: u32) -> u32 {
        let base: u32 = mem.read(mem.rebase(super::ABMAP_PAD));
        pad.wrapping_mul(0x340).wrapping_add(base)
    }

    /// `(n * h) / 3 + h / 6` as the original's magic-number divisions
    /// (both truncate toward zero), plus `base`, as an `i16`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the original's 16-bit add
    fn pad_offset(n: i32, h: i32, base: u16) -> u16 {
        let v = n.wrapping_mul(h) / 3 + h / 6;
        (v as u16).wrapping_add(base)
    }

    /// `AbmapPadStartSlideOut(pad)` (`0x00a48c20`): unless the pad is hidden
    /// (`+0x23` 0) and idle (`+0x34` null), restarts its step counter
    /// (`+0x29`) and installs [`abmap_pad_slide_out_step`].
    pub fn abmap_pad_start_slide_out(mem: &mut impl Mem, pad: u32) {
        let p = pad_at(mem, pad);
        if mem.read::<u8>(p.wrapping_add(0x23)) != 0 || mem.read::<u32>(p.wrapping_add(0x34)) != 0 {
            mem.write(p.wrapping_add(0x29), 0u8);
            let f = mem.rebase(super::PAD_SLIDE_OUT_STEP);
            mem.write(p.wrapping_add(0x34), f);
        }
    }

    /// `AbmapPadStartSlideIn(pad)` (`0x00a48f20`): unless the pad is shown
    /// (`+0x23` set) and idle, restarts its counter and installs
    /// [`abmap_pad_slide_in_step`].
    pub fn abmap_pad_start_slide_in(mem: &mut impl Mem, pad: u32) {
        let p = pad_at(mem, pad);
        if mem.read::<u8>(p.wrapping_add(0x23)) == 0 || mem.read::<u32>(p.wrapping_add(0x34)) != 0 {
            mem.write(p.wrapping_add(0x29), 0u8);
            let f = mem.rebase(super::PAD_SLIDE_IN_STEP);
            mem.write(p.wrapping_add(0x34), f);
        }
    }

    /// `AbmapPadSlideOutStep(p)` (`0x00a59800`): counts `+0x29` up; from 3 the
    /// pad is hidden (`+0x23` 0) and the step removed (`+0x34`), else its
    /// position `+0xa` is `+2 + n h / 3 + h / 6` (`h` the `i16` at `+6`,
    /// its height: half, then five sixths of the way out).
    pub fn abmap_pad_slide_out_step(mem: &mut impl Mem, p: u32) {
        let n = mem.read::<u8>(p.wrapping_add(0x29)).wrapping_add(1);
        mem.write(p.wrapping_add(0x29), n);
        if n >= 3 {
            mem.write(p.wrapping_add(0x23), 0u8);
            mem.write(p.wrapping_add(0x34), 0u32);
            return;
        }
        let h = i32::from(mem.read::<i16>(p.wrapping_add(6)));
        let base: u16 = mem.read(p.wrapping_add(2));
        mem.write(p.wrapping_add(0xa), pad_offset(i32::from(n), h, base));
    }

    /// `AbmapPadSlideInStep(p)` (`0x00a5a270`): the reverse of
    /// [`abmap_pad_slide_out_step`]: the pad is shown (`+0x23` 1); from 3 the
    /// step is removed and `+0xa` is `+2`, else `+2 + (3 - n) h / 3 + h /
    /// 6`.
    pub fn abmap_pad_slide_in_step(mem: &mut impl Mem, p: u32) {
        let n = mem.read::<u8>(p.wrapping_add(0x29)).wrapping_add(1);
        mem.write(p.wrapping_add(0x29), n);
        mem.write(p.wrapping_add(0x23), 1u8);
        let base: u16 = mem.read(p.wrapping_add(2));
        if n >= 3 {
            mem.write(p.wrapping_add(0x34), 0u32);
            mem.write(p.wrapping_add(0xa), base);
            return;
        }
        let h = i32::from(mem.read::<i16>(p.wrapping_add(6)));
        mem.write(p.wrapping_add(0xa), pad_offset(3 - i32::from(n), h, base));
    }

    /// One coordinate of a rectangle tween: `from + ((to - from) * t >>
    /// 16)`, in 32 bits, as an `i16`.
    #[allow(clippy::cast_possible_truncation)] // the original's 16-bit store
    fn tween(from: i16, to: i16, t: i32) -> i16 {
        let d = i32::from(to).wrapping_sub(i32::from(from)).wrapping_mul(t) >> 16;
        d.wrapping_add(i32::from(from)) as i16
    }

    /// The cell coordinates of a rectangle's `+0xc` and `+0xe`:
    /// `(y + 15) / 16` and `(h + 15) / 20`, truncating, to `+0x14` and
    /// `+0x16`.
    #[allow(clippy::cast_possible_truncation)] // 16-bit stores
    fn rect_cells(y: i16, h: i16) -> (i16, i16) {
        (
            ((i32::from(y) + 15) / 16) as i16,
            ((i32::from(h) + 15) / 20) as i16,
        )
    }

    /// The four `i16`s at `r + at`.
    fn rect(mem: &impl Mem, r: u32, at: u32) -> [i16; 4] {
        mem.read(r.wrapping_add(at))
    }

    /// `AbmapRectTweenIn(r)` (`0x00a5a080`): counts `+0x32` up; below 4 the
    /// rectangle `+8` moves `n / 4` of the way from `+0` to `+0x2a`, else
    /// it is `+0x2a` and the step (`+0x34`) is removed; then the cells
    /// ([`rect_cells`]).
    pub fn abmap_rect_tween_in(mem: &mut impl Mem, r: u32) {
        let n = mem.read::<u8>(r.wrapping_add(0x32)).wrapping_add(1);
        mem.write(r.wrapping_add(0x32), n);
        let to = rect(mem, r, 0x2a);
        if n >= 4 {
            mem.write(r.wrapping_add(8), to);
            mem.write(r.wrapping_add(0x34), 0u32);
        } else {
            let from = rect(mem, r, 0);
            let t = (i32::from(n) << 16) >> 2;
            let v = [0, 1, 2, 3].map(|i| tween(from[i], to[i], t));
            mem.write(r.wrapping_add(8), v);
        }
        let v = rect(mem, r, 8);
        let (a, b) = rect_cells(v[2], v[3]);
        mem.write(r.wrapping_add(0x14), a);
        mem.write(r.wrapping_add(0x16), b);
    }

    /// `AbmapRectTweenOut(r)` (`0x00a5a170`): the reverse of
    /// [`abmap_rect_tween_in`]: `(3 - n) / 4` of the way from `+0` to
    /// `+0x2a`; from 4 the rectangle is `+0` with its saved cells (`+0x10`,
    /// `+0x12`) and the step removed.
    pub fn abmap_rect_tween_out(mem: &mut impl Mem, r: u32) {
        let n = mem.read::<u8>(r.wrapping_add(0x32)).wrapping_add(1);
        mem.write(r.wrapping_add(0x32), n);
        let from = rect(mem, r, 0);
        if n >= 4 {
            mem.write(r.wrapping_add(8), from);
            let cells: [i16; 2] = mem.read(r.wrapping_add(0x10));
            mem.write(r.wrapping_add(0x14), cells[0]);
            mem.write(r.wrapping_add(0x34), 0u32);
            mem.write(r.wrapping_add(0x16), cells[1]);
            return;
        }
        let to = rect(mem, r, 0x2a);
        let t = ((3 - i32::from(n)) << 16) / 4;
        let v = [0, 1, 2, 3].map(|i| tween(from[i], to[i], t));
        mem.write(r.wrapping_add(8), v);
        let (a, b) = rect_cells(v[2], v[3]);
        mem.write(r.wrapping_add(0x14), a);
        mem.write(r.wrapping_add(0x16), b);
    }

    /// `AbmapMaskNodeFlags(mask)` (`0x00a47400`): ANDs `mask` into the
    /// flags byte (`+0x22`) of every node (`+0x808`, 0x28 bytes, the `i16`
    /// count at `+2`) whose kind (`+6`) is not -1.
    pub fn abmap_mask_node_flags(mem: &mut impl Mem, mask: u8) {
        let g = ABMAP.get(mem);
        // a negative count runs (practically) forever in the original
        let n = i32::from(mem.read::<i16>(g.wrapping_add(2))).cast_unsigned();
        for i in 0..n {
            let nd = g.wrapping_add(0x808).wrapping_add(0x28u32.wrapping_mul(i));
            if mem.read::<i16>(nd.wrapping_add(6)) != -1 {
                let f: u8 = mem.read(nd.wrapping_add(0x22));
                mem.write(nd.wrapping_add(0x22), f & mask);
            }
        }
    }

    /// `AbmapMarkEmptyMenus()` (`0x00a5aca0`): the pad's menu entries
    /// (`+0x1704`, 12 bytes, the `i16` count at `+0x16de` read again after
    /// each): entry 0 is marked (1) when the current character has no
    /// sphere points (`MsGetSaveSP`), entry 1 when the item list
    /// (`TOAmapMakeItemList(3)`, `TOAmapGetItemListMax`) is empty, the rest
    /// cleared. Returns what the original leaves in `eax`: the last count
    /// read, or 0.
    pub fn abmap_mark_empty_menus<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let pad: u32 = mem.read(mem.rebase(super::ABMAP_PAD));
        let count = |mem: &M| i32::from(mem.read::<i16>(pad.wrapping_add(0x16de)));
        if count(mem) <= 0 {
            return 0;
        }
        let mut i = 0i32;
        let mut e = pad.wrapping_add(0x1704);
        loop {
            mem.write(e, 0u8);
            let r = match i {
                0 => {
                    let g = ABMAP.get(mem);
                    let chr: u8 = mem.read(g.wrapping_add(super::CURRENT_CHR));
                    let f = mem.rebase(super::MS_GET_SAVE_SP);
                    Some(call(mem, f, &[u32::from(chr)]))
                }
                1 => {
                    let f = mem.rebase(super::TO_AMAP_MAKE_ITEM_LIST);
                    call(mem, f, &[3]);
                    let f = mem.rebase(super::TO_AMAP_GET_ITEM_LIST_MAX);
                    Some(call(mem, f, &[]))
                }
                _ => None,
            };
            if r == Some(0) {
                mem.write(e, 1u8);
            }
            let c = count(mem);
            i += 1;
            e = e.wrapping_add(0xc);
            if i >= c {
                return c.cast_unsigned();
            }
        }
    }

    // ---- round 21 ----

    /// `AbmapItemEffectAtNode(chr, node, item, work)` (`0x00a493b0`, from
    /// `AbmapTryUseItem`; name ours): [`abmap_item_usable_at`] for the one
    /// node `node` (index): 0 when the item is not held, else
    /// `AbmapItemNodeEffect` of the node with the same context (`work`, the
    /// original's locals) and its `eax`.
    pub fn abmap_item_effect_at_node<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 3],
        work: u32,
        call: &mut C,
    ) -> u32 {
        let [chr, node, item] = args;
        let f = mem.rebase(super::MS_GET_SAVE_ITEM_NUM);
        if call(mem, f, &[item]) == 0 {
            return 0;
        }
        mem.write(work.wrapping_add(4), chr);
        mem.write(work.wrapping_add(0xc), 0u32);
        mem.write(work.wrapping_add(8), 1u32.wrapping_shl(chr));
        let excel = item_excel(mem, item, work.wrapping_add(0x14), call);
        mem.write(work, excel);
        let nd = ABMAP
            .get(mem)
            .wrapping_add(0x28u32.wrapping_mul(node))
            .wrapping_add(0x808);
        let f = mem.rebase(super::ITEM_NODE_EFFECT);
        call(mem, f, &[nd, work])
    }

    /// Shows pad `pad` (`+0x23` = 1, `+0x24` = 3, `+0x25` = 1) and makes it
    /// the current menu (`+0x2700`) with the handler `step` (`+0x2704`).
    fn open_menu_pad(mem: &mut impl Mem, pad: u32, step: u32) -> u32 {
        let base: u32 = mem.read(mem.rebase(super::ABMAP_PAD));
        let p = base.wrapping_add(0x340 * pad);
        mem.write(p.wrapping_add(0x23), 0x0301u16);
        mem.write(p.wrapping_add(0x25), 1u8);
        #[allow(clippy::cast_possible_truncation)] // a pad index
        mem.write(base.wrapping_add(0x2700), pad as u16);
        let step = mem.rebase(step);
        mem.write(base.wrapping_add(0x2704), step);
        base
    }

    /// `AbmapEnterCommandMenu()` (`0x00a59950`, from the item menu's
    /// handler `0x00a560d0`; name ours): [`abmap_mark_empty_menus`], then
    /// pad 7 shown and current with the handler `0x00a56330`, bit 3 of its
    /// `+0x24` set. Returns the pad array (left in `eax`).
    pub fn abmap_enter_command_menu<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        abmap_mark_empty_menus(mem, call);
        let base = open_menu_pad(mem, 7, super::COMMAND_MENU_STEP);
        let at = base.wrapping_add(0x16e4);
        let v: u8 = mem.read(at);
        mem.write(at, v | 8);
        base
    }

    /// `AbmapEnterItemMenu()` (`0x00a598a0`, from the command menu's
    /// handler `0x00a56330`; name ours): every item menu entry (`+0x1a44`,
    /// 12 bytes, the `i16` count at `+0x1a1e` read again after each) marked
    /// when `AbmapItemUsableAt(current chr, its node, the entry's item
    /// +4)` is 0, then pad 8 shown and current with the handler
    /// `0x00a560d0`. Returns 8 (left in `eax`).
    pub fn abmap_enter_item_menu<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let g = ABMAP.get(mem);
        let chr = u32::from(mem.read::<u8>(g.wrapping_add(super::CURRENT_CHR)));
        let node = u32::from(mem.read::<u16>(g.wrapping_add(0x110cc + 0x50 * chr)));
        let pad: u32 = mem.read(mem.rebase(super::ABMAP_PAD));
        let count = |mem: &M| i32::from(mem.read::<i16>(pad.wrapping_add(0x1a1e)));
        let mut i = 0;
        let mut e = pad.wrapping_add(0x1a44);
        while i < count(mem) {
            let g = ABMAP.get(mem);
            let chr = u32::from(mem.read::<u8>(g.wrapping_add(super::CURRENT_CHR)));
            let item = u32::from(mem.read::<u16>(e.wrapping_add(4)));
            let f = mem.rebase(super::ITEM_USABLE_AT);
            let r = call(mem, f, &[chr, node, item]);
            mem.write(e, u8::from(r == 0));
            i += 1;
            e = e.wrapping_add(0xc);
        }
        open_menu_pad(mem, 8, super::ITEM_MENU_STEP);
        8
    }

    // ---- round 22 ----

    /// `addMenuPrim` as inlined in the menu fillers: appends an entry
    /// (`+0x40` `a`, `+0x44` 0, `+0x48` `b`; 12 bytes from `+0x40`) to
    /// menu pad `p`, its `i16` count at `+0x1e` (at most 0x40) and the
    /// "more than a page" flag `+0x28` (count above rows `+0x22` times
    /// columns `+0x16`). A full pad prints the original's error and calls
    /// `debug_exit_trace(0)`, whose `eax` is returned.
    fn add_menu_prim<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        p: u32,
        a: u32,
        b: u32,
        call: &mut C,
    ) -> Option<u32> {
        let n = mem.read::<i16>(p.wrapping_add(0x1e));
        if n >= 0x40 {
            let fmt = mem.rebase(super::MENU_PRIM_ERROR);
            let f = mem.rebase(super::DBG_PRINTF);
            call(mem, f, &[fmt, 0x40]);
            let f = mem.rebase(super::DEBUG_EXIT_TRACE);
            return Some(call(mem, f, &[0]));
        }
        let rows = i32::from(mem.read::<u8>(p.wrapping_add(0x22)));
        let cols = i32::from(mem.read::<i16>(p.wrapping_add(0x16)));
        let m = n.wrapping_add(1);
        mem.write(p.wrapping_add(0x1e), m);
        mem.write(
            p.wrapping_add(0x28),
            u8::from(i32::from(m) > rows.wrapping_mul(cols)),
        );
        let e = p.wrapping_add(i32::from(n).wrapping_mul(12).cast_unsigned());
        mem.write(e.wrapping_add(0x40), a);
        mem.write(e.wrapping_add(0x48), b);
        mem.write(e.wrapping_add(0x44), 0u8);
        None
    }

    /// `AbmapFillCommandMenu(pad, kind, work)` (`0x00a459e0`, from
    /// `eiAbmStart` with pads 2..=5 and kinds 0x40, 0x41, 0x44, 0x45; name
    /// ours): every menu status entry (`MsMenuStatusNum(i, 0)` over
    /// `MsMenuStatusNumSize()`) of `kind` (byte 0) becomes an entry of menu
    /// pad `pad`: its command id (`+2`, looked up with
    /// `MsGetRomPlyCommand` into `work`) and the id - 0x3000. Returns what
    /// the original leaves in `eax`.
    pub fn abmap_fill_command_menu<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        pad: u32,
        kind: u32,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::MS_MENU_STATUS_NUM_SIZE);
        let n = call(mem, f, &[]).cast_signed();
        let mut last = n.cast_unsigned();
        let mut i = 0i32;
        while i < n {
            let f = mem.rebase(super::MS_MENU_STATUS_NUM);
            let e = call(mem, f, &[i.cast_unsigned(), 0]);
            last = e;
            if u32::from(mem.read::<u8>(e)) == kind {
                let id = u32::from(mem.read::<u16>(e.wrapping_add(2)));
                let f = mem.rebase(super::MS_GET_ROM_PLY_COMMAND);
                call(mem, f, &[id, work]);
                let base: u32 = mem.read(mem.rebase(super::ABMAP_PAD));
                let p = base.wrapping_add(pad.wrapping_mul(0x340));
                let id = u32::from(mem.read::<u16>(e.wrapping_add(2)));
                let b = id.wrapping_sub(0x3000);
                last = add_menu_prim(mem, p, id, b, call).unwrap_or(b);
            }
            i += 1;
        }
        last
    }

    // ---- round 26 ----

    /// `AbmapStartPart(part, n, x, y, z, a, b, c, d, e, f)` (`0x00a5bad0`,
    /// from the sphere grid's states; name ours): the ppp environment set
    /// to the grid's (`0x01a86034`), `pppStartPart(part, 0x1000, table[n],
    /// 1, 0)` with the table `get_ptr32_func(data +0x10)` (data at
    /// `0x02305800`), then `+0x1e` 1, `+0x40..` x, y, z, 1.0, `+0x50..` the
    /// truncated a, b, c, `+0x5c` 1, `+0x60..` d, e, f and `+0x6c` 1.0.
    /// Returns the truncated c (left in `eax`).
    pub fn abmap_start_part<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 11],
        call: &mut C,
    ) -> u32 {
        use ffx_bridge::x87::{ftol, load, round_trip};
        let [part, n, x, y, z, a, b, c, d, e, f] = args;
        let slot = mem.read::<u32>(mem.rebase(super::PPP_ENV_SLOT));
        let env = mem.rebase(super::ABMAP_PPP_ENV);
        mem.write(slot, env);
        let data = mem.read::<u32>(mem.rebase(super::ABMAP_PPP_DATA));
        let g = mem.rebase(super::GET_PTR32_FUNC);
        let tab = call(mem, g, &[mem.read::<u32>(data.wrapping_add(0x10))]);
        let id = mem.read::<u32>(tab.wrapping_add(n.wrapping_mul(4)));
        let g = mem.rebase(super::PPP_START_PART);
        call(mem, g, &[part, 0x1000, id, 1, 0]);
        let rt = |v: u32| round_trip(f32::from_bits(v)).to_bits();
        mem.write(part.wrapping_add(0x1e), 1u16);
        mem.write(part.wrapping_add(0x40), rt(x));
        mem.write(part.wrapping_add(0x44), rt(y));
        mem.write(part.wrapping_add(0x48), rt(z));
        mem.write(part.wrapping_add(0x4c), 1.0f32.to_bits());
        let ia = ftol(mem, load(a));
        mem.write(part.wrapping_add(0x50), ia);
        let ib = ftol(mem, load(b));
        mem.write(part.wrapping_add(0x54), ib);
        let ic = ftol(mem, load(c));
        mem.write(part.wrapping_add(0x60), rt(d));
        mem.write(part.wrapping_add(0x58), ic);
        mem.write(part.wrapping_add(0x5c), 1u32);
        mem.write(part.wrapping_add(0x64), rt(e));
        mem.write(part.wrapping_add(0x68), rt(f));
        mem.write(part.wrapping_add(0x6c), 1.0f32.to_bits());
        ic
    }

    // ---- round 27 ----

    /// `AbmapStartWarp(chr, target)` (`0x00a48a80`, from
    /// `abmap_set_chr_pos`; name ours): the warp sound (`0x80000070`), the
    /// character's node (`+0x110cc + 0x50 chr`) position as floats into
    /// `+0x11308`/`+0x1163c` and `+0x1130c`/`+0x11640`, `+0x11620` 0.0,
    /// `+0x11634` the target, `+0x11638` the character, the grid's ppp
    /// heap (`pppCreateHeap`), [`abmap_start_part`] of part 2 at that
    /// position (scale 0.5), and the state hooks: `+0x115ac` the warp's
    /// draw (`0x00a4c430`) and `+0x115a8` `AbmapState_Warping`, each saved
    /// first unless one is saved already. Returns what the original leaves
    /// in `eax`.
    pub fn abmap_start_warp<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        chr: u32,
        target: u32,
        call: &mut C,
    ) -> u32 {
        let g0 = ABMAP.get(mem);
        let node = u32::from(
            mem.read::<u16>(
                g0.wrapping_add(0x110cc)
                    .wrapping_add(chr.wrapping_mul(0x50)),
            ),
        );
        let nd = g0.wrapping_add(node.wrapping_mul(0x28));
        let f = mem.rebase(super::SND_SEP_PLAY_SIMPLE);
        call(mem, f, &[0x8000_0070]);
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x1164c), 0u8);
        mem.write(g.wrapping_add(0x1164d), 0u8);
        let coord =
            |mem: &M, at: u32| -> u32 { f32::from(mem.read::<i16>(nd.wrapping_add(at))).to_bits() };
        let x = coord(mem, 0x808);
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x11308), x);
        mem.write(g.wrapping_add(0x1163c), x);
        let y = coord(mem, 0x80a);
        mem.write(g.wrapping_add(0x1130c), y);
        mem.write(g.wrapping_add(0x11640), y);
        mem.write(g.wrapping_add(0x11620), 0u32);
        #[allow(clippy::cast_possible_truncation)] // a 16-bit argument
        mem.write(g.wrapping_add(0x11634), target as u16);
        #[allow(clippy::cast_possible_truncation)] // a byte argument
        mem.write(g.wrapping_add(0x11638), chr as u8);
        let f = mem.rebase(super::PPP_CREATE_HEAP);
        let env = mem.rebase(super::ABMAP_PPP_ENV);
        let heap = mem.rebase(super::ABMAP_PPP_HEAP);
        call(mem, f, &[env, heap, 0x7d000]);
        let h = ffx_bridge::x87::round_trip(mem.read::<f32>(mem.rebase(super::WARP_PART_SCALE)))
            .to_bits();
        let part = mem.rebase(super::ABMAP_WARP_PART);
        let (x, y) = (coord(mem, 0x808), coord(mem, 0x80a));
        let mut eax = abmap_start_part(mem, [part, 2, x, y, 0, 0, 0, 0, h, h, h], call);
        let g = ABMAP.get(mem);
        if mem.read::<u32>(g.wrapping_add(0x115b4)) == 0 {
            let v = mem.read::<u32>(g.wrapping_add(0x115ac));
            mem.write(g.wrapping_add(0x115b4), v);
            let g = ABMAP.get(mem);
            mem.write(g.wrapping_add(0x115ac), mem.rebase(super::WARP_DRAW));
            eax = g;
        }
        let g = ABMAP.get(mem);
        if mem.read::<u32>(g.wrapping_add(0x115b0)) == 0 {
            let v = mem.read::<u32>(g.wrapping_add(0x115a8));
            mem.write(g.wrapping_add(0x115b0), v);
            let g = ABMAP.get(mem);
            mem.write(g.wrapping_add(0x115a8), mem.rebase(super::STATE_WARPING));
            eax = g;
        }
        eax
    }

    // ---- round 28 ----

    /// `abmap_set_chr_pos(chr, target)` (`0x00a458f0`): [`abmap_start_warp`]
    /// and `+0x115c7` = 1. Returns the grid (left in `eax`).
    pub fn abmap_set_chr_pos<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        chr: u32,
        target: u32,
        call: &mut C,
    ) -> u32 {
        abmap_start_warp(mem, chr, target, call);
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x115c7), 1u8);
        g
    }

    // ---- round 29 ----

    /// `AbmapCheckAchievements(work)` (`0x00a47210`, from `0x00a48910` and
    /// `AbmapTryChangePlayer`; name ours): marks the menu pads' locked
    /// entries (`AbmapMarkLockedEntries` 3, 2, 4, 5), reads the grid's node
    /// list for the save's grid kind (`MsGetSaveMenu` bits 14..15: file
    /// 0x11, 0x12 or 0x13 through `GetCD`'s reader: `+0x10(0x25)`, `+0
    /// (id, work, 0, 0)`, `+8(0)`) into `work` (0x484 bytes), and unlocks
    /// achievement 0x17 once per character bit 0..6 when every listed node
    /// (bytes, 0xff skipped; the count at `+2` and the list at `+8` when
    /// the file starts with 0x31, else the list at `+2` and, as the
    /// original, the ability map's address as the count) has that bit in
    /// the save's ability map (`MsGetSaveAbilityMap`, byte `1 + 2 i`), and
    /// achievement 0x1d when every listed node there is 0x7f. An empty
    /// list unlocks both.
    pub fn abmap_check_achievements<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        work: u32,
        call: &mut C,
    ) {
        let f = mem.rebase(super::MARK_LOCKED_ENTRIES);
        for pad in [3, 2, 4, 5] {
            call(mem, f, &[pad]);
        }
        let f = mem.rebase(super::MS_GET_SAVE_MENU);
        let menu = call(mem, f, &[]);
        let kind = mem.read::<u32>(menu).cast_signed() >> 14 & 3;
        let file = match kind {
            0 => 0x11,
            1 => 0x12,
            _ => 0x13,
        };
        let f = mem.rebase(super::GET_CD);
        let cd = call(mem, f, &[]);
        let f = mem.read::<u32>(cd.wrapping_add(0x10));
        call(mem, f, &[0x25]);
        let f = mem.read::<u32>(cd);
        call(mem, f, &[file, work, 0, 0]);
        let f = mem.read::<u32>(cd.wrapping_add(8));
        call(mem, f, &[0]);
        let f = mem.rebase(super::MS_GET_SAVE_ABILITY_MAP);
        let save = call(mem, f, &[]);
        let (n, list) = if mem.read::<u16>(work) == 0x31 {
            (
                i32::from(mem.read::<u16>(work.wrapping_add(2))),
                work.wrapping_add(8),
            )
        } else {
            (save.cast_signed(), work.wrapping_add(2))
        };
        let unlock = mem.rebase(super::ACHIEVEMENT_UNLOCK);
        let every = |mem: &M, ok: &dyn Fn(u8) -> bool| {
            (0..n.max(0).cast_unsigned()).all(|j| {
                mem.read::<u8>(list.wrapping_add(j)) == 0xff
                    || ok(mem.read::<u8>(save.wrapping_add(1).wrapping_add(j.wrapping_mul(2))))
            })
        };
        for c in 0..7 {
            let bit = 1u8 << c;
            if n <= 0 || every(mem, &|b| b & bit != 0) {
                call(mem, unlock, &[0x17]);
            }
        }
        if n <= 0 || every(mem, &|b| b == 0x7f) {
            call(mem, unlock, &[0x1d]);
        }
    }

    /// `AbmapSpecialEffect(kind, node, row, chr)` (`0x00a5d120`, the
    /// special item kinds of [`abmap_item_node_effect`]): 1 when the item
    /// of `kind` applies to node index `node` with the excel row `row`
    /// (its type `+0x16`) for `chr`, else 0:
    /// - 1..=4: type 0x12, 0x11, 0 or 0x10; 5..=7 and 0x10..=0x16: type 1;
    /// - 8..=0xb: the node lacks panel bit `chr` (`abmap_test_panel`), the
    ///   type is 0xf, 0xe, 0xc or 0xd and another character has it
    ///   (`abmap_other_has_panel`);
    /// - 0xc, 0xd, 0xe, 0xf: `chr` is not on the node (`abmap_chr_node`)
    ///   and, in turn, it has the panel bit, another character has it,
    ///   another joined character (`MsGetSavePlyJoined`, 0..=6) stands on
    ///   it, or the row is not hidden (bit 15 of `+0x10`);
    /// - 0x17: type 2..=0xb; 0x18: as 0x17 with no panel bit and another
    ///   character's.
    pub fn abmap_special_effect<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        args: [u32; 4],
        call: &mut C,
    ) -> u32 {
        let [kind, node, row, chr] = args;
        let ty = mem.read::<u8>(row.wrapping_add(0x16));
        let test = |mem: &mut M, call: &mut C| {
            let f = mem.rebase(super::ABMAP_TEST_PANEL);
            call(mem, f, &[chr, node]) != 0
        };
        let other = |mem: &mut M, call: &mut C| {
            let f = mem.rebase(super::ABMAP_OTHER_HAS_PANEL);
            call(mem, f, &[chr, node]) != 0
        };
        let on_node = |mem: &mut M, call: &mut C, c: u32| {
            let f = mem.rebase(super::ABMAP_CHR_NODE);
            call(mem, f, &[c]) == node
        };
        let r = match kind {
            1 => ty == 0x12,
            2 => ty == 0x11,
            3 => ty == 0,
            4 => ty == 0x10,
            5..=7 | 0x10..=0x16 => ty == 1,
            8..=0xb => {
                let want = [0xf, 0xe, 0xc, 0xd][(kind - 8) as usize];
                !test(mem, call) && ty == want && other(mem, call)
            }
            0xc => !on_node(mem, call, chr) && test(mem, call),
            0xd => !on_node(mem, call, chr) && other(mem, call),
            0xe => {
                !on_node(mem, call, chr)
                    && (0..7).any(|c| {
                        if c == chr {
                            return false;
                        }
                        let f = mem.rebase(super::MS_GET_SAVE_PLY_JOINED);
                        call(mem, f, &[c]) != 0 && on_node(mem, call, c)
                    })
            }
            0xf => {
                !on_node(mem, call, chr) && mem.read::<u16>(row.wrapping_add(0x10)) & 0x8000 == 0
            }
            0x17 => (2..0xc).contains(&ty),
            0x18 => {
                !test(mem, call)
                    && (2..0xc).contains(&mem.read::<u8>(row.wrapping_add(0x16)))
                    && other(mem, call)
            }
            _ => false,
        };
        u32::from(r)
    }

    // ---- round 30 ----

    /// `AbmapCalcBounds()` (`0x00a5ae40`, from `LoadAbmap`; name ours):
    /// the grid's extent: from 0.0, the minimum and maximum over the `i16`
    /// count (`+0`) of nodes (`+8`, 16 bytes: `i16` x `+0`, y `+2`, kind
    /// `+6`) of `x -/+ w / 2` and `y -/+ h / 2` (the kind's size, `i16`s at
    /// `+0xf808 + 4 kind`; halves toward zero) as floats into
    /// `+0x115fc`/`+0x11600` (minimum x, y) and `+0x11604`/`+0x11608`
    /// (maximum), copied to `+0x1160c`, `+0x11610`, `+0x11614` and
    /// `+0x11618`. Returns the grid (left in `eax`).
    pub fn abmap_calc_bounds(mem: &mut impl Mem) -> u32 {
        let g = ABMAP.get(mem);
        for at in [0x11608, 0x11604, 0x11600, 0x115fc] {
            mem.write(g.wrapping_add(at), 0u32);
        }
        let f = |v: i32| -> f32 {
            #[allow(clippy::cast_precision_loss)] // i16 sums: exact
            let x = v as f32;
            x
        };
        let mut n = i32::from(mem.read::<i16>(g));
        let mut rec = g.wrapping_add(8);
        while n != 0 {
            n = n.wrapping_sub(1);
            let x = i32::from(mem.read::<i16>(rec));
            let y = i32::from(mem.read::<i16>(rec.wrapping_add(2)));
            let kind = u32::from(mem.read::<u16>(rec.wrapping_add(6)));
            let size = g.wrapping_add(0xf808).wrapping_add(kind.wrapping_mul(4));
            let hw = i32::from(mem.read::<i16>(size)) / 2;
            let hh = i32::from(mem.read::<i16>(size.wrapping_add(2))) / 2;
            let ext = [
                (0x115fc, f(x - hw), false),
                (0x11600, f(y - hh), false),
                (0x11604, f(hw + x), true),
                (0x11608, f(hh + y), true),
            ];
            for (at, v, max) in ext {
                let cur = f32::from_bits(mem.read::<u32>(g.wrapping_add(at)));
                if (max && v > cur) || (!max && v < cur) {
                    mem.write(g.wrapping_add(at), v.to_bits());
                }
            }
            rec = rec.wrapping_add(0x10);
        }
        for (from, to) in [
            (0x115fc, 0x1160c),
            (0x11604, 0x11614),
            (0x11600, 0x11610),
            (0x11608, 0x11618),
        ] {
            let v = mem.read::<u32>(g.wrapping_add(from));
            mem.write(
                g.wrapping_add(to),
                ffx_bridge::x87::round_trip(f32::from_bits(v)).to_bits(),
            );
        }
        g
    }

    // ---- field round 46: the help line ----

    /// `TOAddPktBaseMesFontLAbmap(pkt, kind, ..)` (`0x008e8fb0`).
    pub const TO_ADD_PKT_MES_FONT_L_ABMAP: u32 = 0x008e_8fb0;

    /// `AbmapHelpLineDraw(a, b, c)` (`0x00a594c0`, name ours; the five
    /// calls of `AbmapHelpMessageStep_00a59350`): a text line through
    /// `TOAddPktBaseMesFontLAbmap(pos + 0x10, kind, a, b, c, 0, 0, 0x80,
    /// 0x80, 0x80, alpha, 0)`, the packet position (`0x0230fd2c`) set to
    /// its result, which it returns. The fade byte `+0x115b9` of the grid
    /// data is the alpha; 0x80 (opaque) draws with kind 0, anything else
    /// with kind 7.
    pub fn abmap_help_line_draw<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [a, b, c]: [u32; 3],
        call: &mut C,
    ) -> u32 {
        let alpha = u32::from(mem.read::<u8>(ABMAP.get(mem).wrapping_add(0x1_15b9)));
        let kind = if alpha == 0x80 { 0 } else { 7 };
        let pos = crate::ke::PACKET_POS.get(mem).wrapping_add(0x10);
        let f = mem.rebase(TO_ADD_PKT_MES_FONT_L_ABMAP);
        let r = call(
            mem,
            f,
            &[pos, kind, a, b, c, 0, 0, 0x80, 0x80, 0x80, alpha, 0],
        );
        crate::ke::PACKET_POS.set(mem, r);
        r
    }

    /// `ToMakeBtlEasyDigitRightAbmap(..)` (`0x00905660`).
    pub const TO_MAKE_DIGIT_RIGHT_ABMAP: u32 = 0x0090_5660;
    /// Its last two arguments (`f32`, `.rdata`).
    pub const DIGIT_RIGHT_CONSTS: [u32; 2] = [0x00b3_4254, 0x00b9_2194];

    /// `AbmapDigitRight(a, x, y, d)` (`0x00a59630`, name ours; from
    /// `0x00a4f630`): `ToMakeBtlEasyDigitRightAbmap(a, f32(x), f32(y), d,
    /// [0x00b34254], [0x00b92194])` with the integer position as floats
    /// (`fild`, stored as `f32`) and the two constants copied through the
    /// x87; returns its `eax`.
    pub fn abmap_digit_right<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [a, x, y, d]: [u32; 4],
        call: &mut C,
    ) -> u32 {
        use ffx_bridge::x87::{round_trip, store};
        let fx = store(f64::from(x.cast_signed()));
        let fy = store(f64::from(y.cast_signed()));
        let [c1, c2] =
            DIGIT_RIGHT_CONSTS.map(|k| round_trip(mem.read::<f32>(mem.rebase(k))).to_bits());
        let f = mem.rebase(TO_MAKE_DIGIT_RIGHT_ABMAP);
        call(mem, f, &[a, fx, fy, d, c1, c2])
    }

    // ---- field r13 ----

    /// `AbmapPositionPlyTag(chr)` (`0x00a58080`, in `ffx-pcui`).
    pub const POSITION_PLY_TAG: u32 = 0x00a5_8080;
    /// 1/12 (an `f64` in `.rdata`, the `f32` value widened): the warp's
    /// step per frame.
    pub const WARP_STEP: u32 = 0x00b4_b448;

    /// The fade byte (`+0x115c6`) for frame `n` of a 40-frame fade:
    /// `(n << 7) / 40`, its low byte, at least `0x80` kept at `0x80`.
    /// Also returns the low word of the division's `imul` (left in `eax`
    /// by the fade-in).
    fn warp_fade(n: u8) -> (u8, u32) {
        let x = u32::from(n) << 7;
        #[allow(clippy::cast_possible_truncation)] // the low byte is stored
        let q = (x / 40) as u8;
        (q, 0x6666_6667u32.wrapping_mul(x))
    }

    /// `AbmapState_Warping()` (`0x00a47f00`, the state
    /// [`abmap_start_warp`] installs), by its phase (`+0x1164c`):
    /// - 0: the fade-out: frame `+0x1164d` counts up, the fade
    ///   (`+0x115c6`) is `0x80 - (n << 7) / 40` (a byte; `0x80` from
    ///   `0x80` up); once the warp part ([`super::ABMAP_WARP_PART`]
    ///   `+0x1e`) is done the character's marker word `+0x110c4 + 0x50
    ///   chr` is 0.0, phase 1 and the fade 0.
    /// - 1: the move: the progress (`+0x11620`) steps by 1/12; below 1.0
    ///   (or NaN) the camera x, y (`+0x11308`) are `FFXVu0InterVectorXYZ`
    ///   from the start (`+0x1163c`) to the target node (`+0x11634`).
    ///   From 1.0: phase 2, the frame 0, the warp sound, the camera at the
    ///   node (x, y, 0.0, 1.0), the grid's ppp heap again, part 1 of the
    ///   warp effect there (scale 0.5), the character (`+0x11638`) on the
    ///   node (`+0x110cc + 0x50 chr`), its marker and player tag placed,
    ///   `+0x115c7` set, the cursor camera reset.
    /// - 2: the fade-in: `(n << 7) / 40`; once the part is done the fade
    ///   is `0x80` and the saved state and draw hooks (`+0x115b0`,
    ///   `+0x115b4`) come back to `+0x115a8`/`+0x115ac` (and are cleared).
    ///
    /// Returns the `eax` it ends with: the grid, the cursor camera's
    /// result, the phase minus 2 for an unknown phase, and in the
    /// fade-out's running frames the division's `imul` low word with the
    /// fade in its low byte.
    pub fn abmap_state_warping<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let g = ABMAP.get(mem);
        let part_running =
            |mem: &M| mem.read::<u16>(mem.rebase(super::ABMAP_WARP_PART).wrapping_add(0x1e)) != 0;
        match mem.read::<u8>(g.wrapping_add(0x1164c)) {
            0 => {
                let frame = g.wrapping_add(0x1164d);
                mem.write(frame, mem.read::<u8>(frame).wrapping_add(1));
                let g = ABMAP.get(mem);
                let (q, low) = warp_fade(mem.read::<u8>(g.wrapping_add(0x1164d)));
                let fade = 0x80u8.wrapping_sub(q);
                mem.write(g.wrapping_add(0x115c6), fade);
                let g = ABMAP.get(mem);
                if mem.read::<u8>(g.wrapping_add(0x115c6)) >= 0x80 {
                    mem.write(g.wrapping_add(0x115c6), 0x80u8);
                }
                if part_running(mem) {
                    return (low & !0xff) | u32::from(fade);
                }
                let g = ABMAP.get(mem);
                let chr = u32::from(mem.read::<u8>(g.wrapping_add(0x11638)));
                mem.write(g.wrapping_add(0x110c4 + 0x50 * chr), 0u32);
                let g = ABMAP.get(mem);
                mem.write(g.wrapping_add(0x1164c), 1u8);
                let g = ABMAP.get(mem);
                mem.write(g.wrapping_add(0x115c6), 0u8);
                g
            }
            1 => warp_move(mem, g, call),
            2 => {
                let frame = g.wrapping_add(0x1164d);
                mem.write(frame, mem.read::<u8>(frame).wrapping_add(1));
                let g = ABMAP.get(mem);
                let (q, _) = warp_fade(mem.read::<u8>(g.wrapping_add(0x1164d)));
                mem.write(g.wrapping_add(0x115c6), q);
                let g = ABMAP.get(mem);
                if mem.read::<u8>(g.wrapping_add(0x115c6)) >= 0x80 {
                    mem.write(g.wrapping_add(0x115c6), 0x80u8);
                }
                let g = ABMAP.get(mem);
                if part_running(mem) {
                    return g;
                }
                mem.write(g.wrapping_add(0x115c6), 0x80u8);
                for (hook, saved) in [(0x115a8, 0x115b0), (0x115ac, 0x115b4)] {
                    let g = ABMAP.get(mem);
                    let v = mem.read::<u32>(g.wrapping_add(saved));
                    mem.write(g.wrapping_add(hook), v);
                    let g = ABMAP.get(mem);
                    mem.write(g.wrapping_add(saved), 0u32);
                }
                ABMAP.get(mem)
            }
            phase => u32::from(phase) - 2,
        }
    }

    /// Phase 1 of [`abmap_state_warping`] (`g` the grid it started with).
    fn warp_move<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        g: u32,
        call: &mut C,
    ) -> u32 {
        let node = u32::from(mem.read::<u16>(g.wrapping_add(0x11634)));
        let at = g
            .wrapping_add(layout::NODES)
            .wrapping_add(node * layout::NODE_SIZE);
        let x = f32::from(mem.read::<i16>(at));
        let y = f32::from(mem.read::<i16>(at.wrapping_add(2)));
        let step = mem.read::<f64>(mem.rebase(WARP_STEP));
        let t = x87::store(x87::load(mem.read::<u32>(g.wrapping_add(0x11620))) + step);
        mem.write(g.wrapping_add(0x11620), t);
        let t = f32::from_bits(t);
        // fcom: below 1.0 or unordered keeps moving
        if t < 1.0 || t.is_nan() {
            let start = g.wrapping_add(0x1163c);
            let bx = mem.read::<f32>(start);
            let by = mem.read::<f32>(start.wrapping_add(4));
            let rx = ffx_ps2::vu0::inter_xyz(0, x, bx, t);
            let ry = ffx_ps2::vu0::inter_xyz(1, y, by, t);
            let g = ABMAP.get(mem);
            mem.write(g.wrapping_add(0x11308), x87::copy(rx.to_bits()));
            let g = ABMAP.get(mem);
            mem.write(g.wrapping_add(0x1130c), x87::copy(ry.to_bits()));
            return g;
        }
        mem.write(g.wrapping_add(0x1164c), 2u8);
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x1164d), 0u8);
        let f = mem.rebase(super::SND_SEP_PLAY_SIMPLE);
        call(mem, f, &[0x8000_0070]);
        let g = ABMAP.get(mem);
        let camera = [x.to_bits(), y.to_bits(), 0, 1.0f32.to_bits()];
        mem.write(g.wrapping_add(0x11308), camera);
        let f = mem.rebase(super::PPP_CREATE_HEAP);
        let env = mem.rebase(super::ABMAP_PPP_ENV);
        let heap = mem.rebase(super::ABMAP_PPP_HEAP);
        call(mem, f, &[env, heap, 0x7d000]);
        let h = ffx_bridge::x87::round_trip(mem.read::<f32>(mem.rebase(super::WARP_PART_SCALE)))
            .to_bits();
        let part = mem.rebase(super::ABMAP_WARP_PART);
        let (x, y) = (x.to_bits(), y.to_bits());
        abmap_start_part(mem, [part, 1, x, y, 0, 0, 0, 0, h, h, h], call);
        let g = ABMAP.get(mem);
        let chr = u32::from(mem.read::<u8>(g.wrapping_add(0x11638)));
        let target = mem.read::<u16>(g.wrapping_add(0x11634));
        mem.write(g.wrapping_add(0x110cc + 0x50 * chr), target);
        let chr = u32::from(mem.read::<u8>(ABMAP.get(mem).wrapping_add(0x11638)));
        abmap_set_chr_marker(mem, chr);
        let chr = u32::from(mem.read::<u8>(ABMAP.get(mem).wrapping_add(0x11638)));
        let f = mem.rebase(POSITION_PLY_TAG);
        call(mem, f, &[chr]);
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x115c7), 1u8);
        abmap_reset_cursor_camera(mem)
    }

    /// `AbmapFlagConnectedLinks(flag)` (`0x00a58df0`) and
    /// `AbmapCalcMoveCost(node, cost)` (`0x00a47560`).
    pub const MOVE_COSTS_CALLS: [u32; 2] = [0x00a5_8df0, 0x00a4_7560];

    /// `AbmapCalcMoveCosts(node, limit, bit)` (`0x00a474d0`): every node
    /// with a kind (`+6` not `0xffff`) loses its cost (`+0x24` = `0xffff`),
    /// every link the low nibble of its flags (`+0xa816`), the connected
    /// links are flagged (`AbmapFlagConnectedLinks(2)`), then the walk
    /// starts from `node` (`AbmapCalcMoveCost(node, 0)`) with the limit
    /// `limit` (`u16`) * 4 ([`super::MOVE_LIMIT`]), the mask `1 << bit`
    /// (low 5 bits; [`super::MOVE_MASK`] as a word) and the step 0
    /// ([`super::MOVE_STEP`]). Returns the walk's `eax`.
    pub fn abmap_calc_move_costs<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [node, limit, bit]: [u32; 3],
        call: &mut C,
    ) -> u32 {
        let g = ABMAP.get(mem);
        let nodes = mem.read::<i16>(g.wrapping_add(2));
        let mut at = g.wrapping_add(layout::NODES);
        for _ in 0..i32::from(nodes).cast_unsigned() {
            if mem.read::<u16>(at.wrapping_add(6)) != 0xffff {
                mem.write(at.wrapping_add(0x24), 0xffffu16);
            }
            at = at.wrapping_add(layout::NODE_SIZE);
        }
        let g = ABMAP.get(mem);
        let links = mem.read::<i16>(g.wrapping_add(layout::LINK_COUNT));
        let mut at = g.wrapping_add(layout::LINK_FLAGS);
        for _ in 0..i32::from(links).cast_unsigned() {
            mem.write(at, mem.read::<u8>(at) & 0xf0);
            at = at.wrapping_add(layout::LINK_SIZE);
        }
        let [flag_links, walk] = MOVE_COSTS_CALLS.map(|f| mem.rebase(f));
        call(mem, flag_links, &[2]);
        super::MOVE_LIMIT.set(mem, ((limit & 0xffff) << 2).cast_signed());
        let mask = super::MOVE_MASK.ptr(mem).addr;
        mem.write(mask, 1u32 << (bit & 0x1f));
        super::MOVE_STEP.set(mem, 0);
        call(mem, walk, &[node, 0])
    }

    /// The state [`abmap_select_node`] sets while homing (`0x00a5a020`).
    pub const STATE_SELECT_HOMING: u32 = 0x00a5_a020;

    /// `AbmapSelectNode_00a48e40(node, t)` (`0x00a48e40`; name ours): on
    /// the cursor's node (`+0x112fc`) the command menu opens
    /// ([`abmap_enter_command_menu`]); elsewhere the cursor homes on
    /// `node` ([`abmap_init_homing`], `t`) and, with no state saved
    /// (`+0x115b0`), the state goes to [`STATE_SELECT_HOMING`], the old one
    /// saved. Returns the pad array or the grid (left in `eax`).
    pub fn abmap_select_node<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [node, t]: [u32; 2],
        call: &mut C,
    ) -> u32 {
        let g = ABMAP.get(mem);
        if node == u32::from(mem.read::<u16>(g.wrapping_add(0x112fc))) {
            return abmap_enter_command_menu(mem, call);
        }
        abmap_init_homing(mem, node, t);
        let g = ABMAP.get(mem);
        if mem.read::<u32>(g.wrapping_add(0x115b0)) == 0 {
            let state = mem.read::<u32>(g.wrapping_add(0x115a8));
            mem.write(g.wrapping_add(0x115b0), state);
            let g = ABMAP.get(mem);
            mem.write(g.wrapping_add(0x115a8), mem.rebase(STATE_SELECT_HOMING));
            return g;
        }
        g
    }

    /// `AbmapPlyTagRect(tag, out)` (`0x00a482d0`) and `AbmapRectsOverlap`
    /// (`0x00a49270`), [`abmap_spread_ply_tags`]'s calls.
    pub const SPREAD_CALLS: [u32; 2] = [0x00a4_82d0, 0x00a4_9270];

    /// `AbmapSpreadPlyTags(work)` (`0x00a5a4b0`, from `0x00a53570`; name
    /// ours), `work` the original's seven boxes (`0x70` bytes): the seven
    /// characters' name tags are reset (corner `+0x4e` 0,
    /// `AbmapPositionPlyTag`), then, from the current character
    /// (`+0x115bc`) round to it, each tag's box is taken
    /// (`AbmapPlyTagRect`) and, while it overlaps one of the boxes taken
    /// before it, the tag moves to its next corner and is taken again,
    /// up to corner 7. A second round does the same against all six other
    /// boxes. Returns the character after the last (left in `eax`).
    pub fn abmap_spread_ply_tags<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        work: u32,
        call: &mut C,
    ) -> u32 {
        let position = mem.rebase(POSITION_PLY_TAG);
        for chr in 0..7u32 {
            let corner = ABMAP.get(mem).wrapping_add(0x110d6 + 0x50 * chr);
            mem.write(corner, 0u8);
            call(mem, position, &[chr]);
        }
        let first = |mem: &M| u32::from(mem.read::<u8>(ABMAP.get(mem).wrapping_add(0x115bc)));
        let next = |chr: u32| if chr + 1 > 6 { 0 } else { chr + 1 };
        let mut chr = first(mem);
        for n in 0..7 {
            let tag = ply_tag(mem, chr);
            let f = mem.rebase(SPREAD_CALLS[0]);
            call(mem, f, &[tag, work.wrapping_add(0x10 * n)]);
            settle_ply_tag(mem, [chr, n, work], |j| j < n, call);
            chr = next(chr);
        }
        let mut chr = first(mem);
        for m in 0..7 {
            settle_ply_tag(mem, [chr, m, work], |j| j != m, call);
            chr = next(chr);
        }
        chr
    }

    /// Character `chr`'s name tag (`g_Abmap + 0x11088`, `0x50` each).
    fn ply_tag<M: Mem>(mem: &M, chr: u32) -> u32 {
        ABMAP
            .get(mem)
            .wrapping_add(0x11088)
            .wrapping_add(0x50 * chr)
    }

    /// [`abmap_spread_ply_tags`]'s step: while box `n` of `work` overlaps
    /// one of the boxes `others` picks (in order, the first hit ends the
    /// search), `chr`'s tag moves to its next corner and its box is taken
    /// again; corner 7 ends it.
    fn settle_ply_tag<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [chr, n, work]: [u32; 3],
        others: impl Fn(u32) -> bool,
        call: &mut C,
    ) {
        let position = mem.rebase(POSITION_PLY_TAG);
        let [tag_rect, overlap] = SPREAD_CALLS.map(|f| mem.rebase(f));
        let rect = |i: u32| work.wrapping_add(0x10 * i);
        let tag = ply_tag(mem, chr);
        let corner = tag.wrapping_add(0x4e);
        loop {
            let hit = (0..7)
                .filter(|&j| others(j))
                .any(|j| call(mem, overlap, &[rect(n), rect(j)]) != 0);
            if !hit {
                return;
            }
            mem.write(corner, mem.read::<u8>(corner).wrapping_add(1));
            call(mem, position, &[chr]);
            call(mem, tag_rect, &[tag, rect(n)]);
            if mem.read::<u8>(corner) >= 7 {
                return;
            }
        }
    }

    /// `AbmapStartPart` (`0x00a5bad0`) and `AbmapShowChangeMessage`
    /// (`0x00a5a640`), [`abmap_cheng_panel`]'s calls.
    pub const CHENG_PANEL_CALLS: [u32; 2] = [0x00a5_bad0, 0x00a5_a640];
    /// The node change's draw hook (`0x00a51700`) and state
    /// (`AbmapState_ChangingNode`, `0x00a47d50`).
    pub const CHENG_PANEL_HOOKS: [u32; 2] = [0x00a5_1700, 0x00a4_7d50];

    /// `abmap_cheng_panel(kind, node)` (`0x00a48740`, Fahrenheit's name):
    /// node `node`'s panel starts changing to kind `kind`. The ppp heap
    /// is made again and the warp part starts on the node (kind 5 when the
    /// save menu's bits `0xc000` are `0x8000`, else 3; scale 0.5).
    /// `+0x11628` and `+0x1162c` are the node's current kind's and the new
    /// kind's radii (half the size word at `+0xf834`, records of `0x30`,
    /// plus 3.0, [`super::HOMING_MARGIN`]); `+0x11650` 0,
    /// `+0x1164c`/`+0x1164e` `kind`/`node`, `AbmapShowChangeMessage(kind,
    /// node)`, then the draw hook and state are saved and set as in
    /// [`abmap_start_warp`]. The node is read through the grid pointer
    /// taken before the calls. Returns the `eax` it ends with.
    pub fn abmap_cheng_panel<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [kind, node]: [u32; 2],
        call: &mut C,
    ) -> u32 {
        let at = ABMAP
            .get(mem)
            .wrapping_add(layout::NODES)
            .wrapping_add(node.wrapping_mul(layout::NODE_SIZE));
        let f = mem.rebase(super::MS_GET_SAVE_MENU);
        let menu = call(mem, f, &[]);
        let f = mem.rebase(super::PPP_CREATE_HEAP);
        let env = mem.rebase(super::ABMAP_PPP_ENV);
        let heap = mem.rebase(super::ABMAP_PPP_HEAP);
        call(mem, f, &[env, heap, 0x7d000]);
        let h = x87::copy(mem.read::<u32>(mem.rebase(super::WARP_PART_SCALE)));
        let look = if mem.read::<u32>(menu) & 0xc000 == 0x8000 {
            5
        } else {
            3
        };
        let coord = |mem: &M, o: u32| f32::from(mem.read::<i16>(at.wrapping_add(o))).to_bits();
        let (x, y) = (coord(mem, 0), coord(mem, 2));
        let part = mem.rebase(super::ABMAP_WARP_PART);
        let [start_part, show_message] = CHENG_PANEL_CALLS.map(|f| mem.rebase(f));
        call(mem, start_part, &[part, look, x, y, 0, 0, 0, 0, h, h, h]);
        let margin = super::HOMING_MARGIN.get(mem);
        let radius = |mem: &M, kind: u32| {
            let size = ABMAP
                .get(mem)
                .wrapping_add(layout::KIND_SIZE)
                .wrapping_add(kind.wrapping_mul(layout::KIND_STRIDE));
            x87::store(f64::from(mem.read::<i16>(size) >> 1) + margin)
        };
        let g = ABMAP.get(mem);
        let old = u32::from(
            mem.read::<u16>(
                g.wrapping_add(layout::NODES + 6)
                    .wrapping_add(node.wrapping_mul(layout::NODE_SIZE)),
            ),
        );
        let v = radius(mem, old);
        mem.write(g.wrapping_add(0x11628), v);
        let v = radius(mem, kind);
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x1162c), v);
        mem.write(ABMAP.get(mem).wrapping_add(0x11650), 0u8);
        #[allow(clippy::cast_possible_truncation)] // 16-bit kind and node
        {
            mem.write(ABMAP.get(mem).wrapping_add(0x1164c), kind as u16);
            mem.write(ABMAP.get(mem).wrapping_add(0x1164e), node as u16);
        }
        let mut eax = call(mem, show_message, &[kind, node]);
        let [draw, state] = CHENG_PANEL_HOOKS.map(|f| mem.rebase(f));
        for (saved, slot, hook) in [(0x115b4, 0x115ac, draw), (0x115b0, 0x115a8, state)] {
            let g = ABMAP.get(mem);
            if mem.read::<u32>(g.wrapping_add(saved)) == 0 {
                let v = mem.read::<u32>(g.wrapping_add(slot));
                mem.write(g.wrapping_add(saved), v);
                let g = ABMAP.get(mem);
                mem.write(g.wrapping_add(slot), hook);
                eax = g;
            }
        }
        eax
    }

    /// `AbmapRangesOverlap(a, b, c, d)` (`0x00a57000`, floats).
    pub const RANGES_OVERLAP: u32 = 0x00a5_7000;
    /// `1e11` (`0x00b7b710`), `AbmapPlyTagRect`'s mark for a box that is
    /// not on screen.
    pub const OFF_SCREEN: u32 = 0x00b7_b710;

    /// `AbmapRectsOverlap(a, b)` (`0x00a49270`, from `0x00a5a4b0`; name
    /// ours): whether two `(x0, y0, x1, y1)` boxes from `AbmapPlyTagRect`
    /// overlap. A box whose `x0` is [`OFF_SCREEN`] never does. The box
    /// further left must reach the other's `x0` (`x1 >= x0`; equal or
    /// unordered `x0`s pass), then the y ranges go to
    /// `AbmapRangesOverlap(a.y0, a.y1, b.y0, b.y1)`. Returns 0 or 1.
    pub fn abmap_rects_overlap<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [a, b]: [u32; 2],
        call: &mut C,
    ) -> u32 {
        let word = |mem: &M, at: u32, i: u32| mem.read::<u32>(at.wrapping_add(4 * i));
        let float = |mem: &M, at: u32, i: u32| f32::from_bits(word(mem, at, i));
        let off = f32::from_bits(mem.read::<u32>(mem.rebase(OFF_SCREEN)));
        #[allow(clippy::float_cmp)] // `fucom`'s exact match
        let off_screen = float(mem, a, 0) == off || float(mem, b, 0) == off;
        if off_screen {
            return 0;
        }
        let (ax, bx) = (float(mem, a, 0), float(mem, b, 0));
        let reaches = if bx > ax {
            float(mem, a, 2) >= bx
        } else if bx < ax {
            float(mem, b, 2) >= ax
        } else {
            true
        };
        if !reaches {
            return 0;
        }
        // Passed through `fld`/`fstp`, which quiets a signalling NaN.
        let args = [
            word(mem, a, 1),
            word(mem, a, 3),
            word(mem, b, 1),
            word(mem, b, 3),
        ]
        .map(x87::copy);
        let f = mem.rebase(RANGES_OVERLAP);
        u32::from(call(mem, f, &args) != 0)
    }

    /// `FFXVu0CopyMatrix`, `pppSetMatrix`, `FFXVu0MulMatrix`, `pppRunPart`
    /// and `pppAllFreePObject` (`0x006ed360`, `0x00717410`, `0x006ed5c0`,
    /// `0x00712250`, `0x00711f20`): [`abmap_run_part`]'s calls.
    pub const RUN_PART_CALLS: [u32; 5] = [
        0x006e_d360,
        0x0071_7410,
        0x006e_d5c0,
        0x0071_2250,
        0x0071_1f20,
    ];
    /// `g_pppScreenMatrix`, `g_pppCameraMatrix`, `g_pppViewMatrix`,
    /// `g_pppWorldMatrix` and the view matrix copy (`0x0230fe20`).
    pub const PPP_MATRICES: [u32; 5] = [
        0x0230_fe60,
        0x0230_ff60,
        0x0230_ff20,
        0x0230_ffa0,
        0x0230_fe20,
    ];
    /// 29184.0 (`f64`, `.rdata`): added to the screen matrix's z.
    pub const RUN_PART_DEPTH: u32 = 0x00b7_b6c8;

    /// `AbmapRunPart(part)` (`0x00a45930`, a part's draw hook; name ours):
    /// a running part (`+0x1e`) under the grid's matrices: the ppp screen
    /// and camera matrices from `+0x113a0`/`+0x11420`, the screen's z
    /// translation (`+0x38`) moved by 29184.0, `pppSetMatrix(part)`, the
    /// view matrix (camera times world) and its copy, then
    /// `pppRunPart(part, 0x30)` in the grid's ppp environment; a finished
    /// part's objects are freed (`pppAllFreePObject`) and `+0x1e` cleared.
    /// Returns 0 (the original leaves its caller's `eax` when the part is
    /// not running).
    pub fn abmap_run_part<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        part: u32,
        call: &mut C,
    ) -> u32 {
        if mem.read::<u16>(part.wrapping_add(0x1e)) == 0 {
            return 0;
        }
        let [copy, set_matrix, mul, run, free] = RUN_PART_CALLS.map(|f| mem.rebase(f));
        let [screen, camera, view, world, view_copy] = PPP_MATRICES.map(|m| mem.rebase(m));
        let g = ABMAP.get(mem);
        call(mem, copy, &[screen, g.wrapping_add(0x113a0)]);
        let g = ABMAP.get(mem);
        call(mem, copy, &[camera, g.wrapping_add(0x11420)]);
        let z = screen.wrapping_add(0x38);
        let depth = mem.read::<f64>(mem.rebase(RUN_PART_DEPTH));
        mem.write(z, x87::store(x87::load(mem.read::<u32>(z)) + depth));
        call(mem, set_matrix, &[part]);
        call(mem, mul, &[view, camera, world]);
        call(mem, copy, &[view_copy, view]);
        let env = mem.rebase(super::ABMAP_PPP_ENV);
        let slot = mem.read::<u32>(mem.rebase(super::PPP_ENV_SLOT));
        mem.write(slot, env);
        if call(mem, run, &[part, 0x30]) != 0 {
            let slot = mem.read::<u32>(mem.rebase(super::PPP_ENV_SLOT));
            mem.write(slot, env);
            call(mem, free, &[part]);
            mem.write(part.wrapping_add(0x1e), 0u16);
        }
        0
    }

    /// `MsGetSaveConfigHiragana()` (`0x007851f0`) and `MsMenuGetText(group,
    /// id, kana)` (`0x0078fcf0`).
    pub const CHANGE_MESSAGE_CALLS: [u32; 2] = [0x0078_51f0, 0x0078_fcf0];

    /// `AbmapShowChangeMessage(kind, node)` (`0x00a5a640`, from
    /// `abmap_cheng_panel`; name ours): the panel-change message: the
    /// node's (`+0x80e`) and the new kind's node excel rows
    /// (`MsGetExcelData` from [`super::NODE_EXCEL_TABLE`], their string
    /// bases in `work`, two words), the timer `+0x11666` 0x78, `+0x11660`
    /// 0, the text `MsMenuGetText(9, 0x22, kana)` at `+0x11654` and the two
    /// names (each row's `u16` for the kana setting plus its base) at
    /// `+0x11658`/`+0x1165c`, `+0x11664` 2. The kana setting is read anew
    /// (`MsGetSaveConfigHiragana() & 1`) for each. Returns the grid.
    pub fn abmap_show_change_message<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [kind, node]: [u32; 2],
        work: u32,
        call: &mut C,
    ) -> u32 {
        let [kana, text] = CHANGE_MESSAGE_CALLS.map(|f| mem.rebase(f));
        let excel = mem.rebase(super::MS_GET_EXCEL_DATA);
        let table = mem.read::<u32>(mem.rebase(super::NODE_EXCEL_TABLE));
        let g = ABMAP.get(mem);
        let old =
            u32::from(mem.read::<u16>(g.wrapping_add(0x80e).wrapping_add(node.wrapping_mul(0x28))));
        let (old_base, new_base) = (work, work.wrapping_add(4));
        let old_row = call(mem, excel, &[old, table, old_base]);
        let table = mem.read::<u32>(mem.rebase(super::NODE_EXCEL_TABLE));
        let new_row = call(mem, excel, &[kind, table, new_base]);
        mem.write(ABMAP.get(mem).wrapping_add(0x11666), 0x78u16);
        mem.write(ABMAP.get(mem).wrapping_add(0x11660), 0u32);
        let k = call(mem, kana, &[]) & 1;
        let message = call(mem, text, &[9, 0x22, k]);
        mem.write(ABMAP.get(mem).wrapping_add(0x11654), message);
        for (row, base, at) in [(old_row, old_base, 0x11658), (new_row, new_base, 0x1165c)] {
            let base = mem.read::<u32>(base);
            let k = call(mem, kana, &[]) & 1;
            let name = u32::from(mem.read::<u16>(row.wrapping_add(4 * k))).wrapping_add(base);
            mem.write(ABMAP.get(mem).wrapping_add(at), name);
        }
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x11664), 2u16);
        g
    }

    /// `AbmapGetNodePosRadius(node, out)` (`0x00a5b4c0`, in `ST0`),
    /// `AbmapCalcLinkPointsStraight` (`0x00a581f0`) and
    /// `AbmapCalcLinkPointsCurved` (`0x00a57710`).
    pub const LINK_POINT_CALLS: [u32; 3] = [0x00a5_b4c0, 0x00a5_81f0, 0x00a5_7710];

    /// `AbmapCalcLinkPoints()` (`0x00a5a800`): every link (`+0xa808`, `+4`
    /// of them) gets the point buffer position `+0x11668` at `+0x10`, then
    /// its points from its ends' positions and radii
    /// (`AbmapGetNodePosRadius`, in `work`: 0x30 bytes, the radii stored
    /// as `f32`): `AbmapCalcLinkPointsStraight(out, a, b, ra, rb, link
    /// +0xd)` when its centre node (`+4`) is 0xffff, else
    /// `AbmapCalcLinkPointsCurved(out, a, b, c, ra, rb, link +0xd)`; each
    /// returns the next position, kept at `+0x11668`. Returns the grid.
    pub fn abmap_calc_link_points<M, C, F>(
        mem: &mut M,
        work: u32,
        call: &mut C,
        fcall: &mut F,
    ) -> u32
    where
        M: Mem,
        C: FnMut(&mut M, u32, &[u32]) -> u32,
        F: FnMut(&mut M, u32, crate::camera::FArgs<'_>) -> f64,
    {
        use crate::camera::FArgs;
        let [pos_radius, straight, curved] = LINK_POINT_CALLS.map(|f| mem.rebase(f));
        let g = ABMAP.get(mem);
        let mut left = i32::from(mem.read::<i16>(g.wrapping_add(4)));
        let mut out = mem.read::<u32>(g.wrapping_add(0x11668));
        let mut link = g.wrapping_add(0xa808);
        let (a, b, c) = (work, work.wrapping_add(0x10), work.wrapping_add(0x20));
        while left != 0 {
            left = left.wrapping_sub(1);
            mem.write(link.wrapping_add(0x10), out);
            let end = |mem: &M, o: u32| u32::from(mem.read::<u16>(link.wrapping_add(o)));
            let n = end(mem, 0);
            let ra = x87::store(fcall(mem, pos_radius, FArgs::Stack(&[n, a])));
            let n = end(mem, 2);
            let rb = x87::store(fcall(mem, pos_radius, FArgs::Stack(&[n, b])));
            let centre = end(mem, 4);
            let kind = link.wrapping_add(0xd);
            out = if centre == 0xffff {
                call(mem, straight, &[out, a, b, ra, rb, kind])
            } else {
                fcall(mem, pos_radius, FArgs::Stack(&[centre, c]));
                call(mem, curved, &[out, a, b, c, ra, rb, kind])
            };
            link = link.wrapping_add(layout::LINK_SIZE);
        }
        let g = ABMAP.get(mem);
        mem.write(g.wrapping_add(0x11668), out);
        g
    }

    /// `AbmapItemApplyAt(chr, node, item, mark)` (`0x00a5b400`).
    pub const ITEM_APPLY_AT: u32 = 0x00a5_b400;
    /// `AbmapFindNearestNode(dist, pos, filter)` (`0x00a56a40`).
    pub const FIND_NEAREST_NODE: u32 = 0x00a5_6a40;
    /// `abmCalcChrMoveCurMoveCheck` (`0x00a45000`), its filter.
    pub const MOVE_CHECK: u32 = 0x00a4_5000;
    /// The grid's item-target state and draw hooks (`0x00a452d0`,
    /// `0x00a45500`, a copy of `AbmapSetHelpMessage`).
    pub const ITEM_TARGET_HOOKS: [u32; 2] = [0x00a4_52d0, 0x00a4_5500];

    /// `AbmapShowItemTargets(chr, node, item, work)` (`0x00a5b980`, from
    /// the item menu's handler; name ours): every node with a kind loses
    /// its marks (bits 0..1 of `+0x22`), `AbmapItemApplyAt(chr, node, item,
    /// 3)` marks where the item applies, `+0x116a0`/`+0x1169c` 0, the
    /// state and draw hooks are [`ITEM_TARGET_HOOKS`], and the cursor
    /// homes ([`abmap_init_homing`], 0.5) on the node nearest the current
    /// character (`AbmapFindNearestNode(work, its marker, MOVE_CHECK)`)
    /// unless it is on it (`+0x112fc`) or there is none, saving the state
    /// for `AbmapState_HomeOnNode` when none is saved. `+0x115c3` is set.
    /// Returns the `eax` it ends with.
    pub fn abmap_show_item_targets<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        [chr, node, item]: [u32; 3],
        work: u32,
        call: &mut C,
    ) -> u32 {
        let g = ABMAP.get(mem);
        let mut left = i32::from(mem.read::<i16>(g.wrapping_add(2)));
        let mut at = g.wrapping_add(layout::NODES);
        while left != 0 {
            left = left.wrapping_sub(1);
            if mem.read::<u16>(at.wrapping_add(6)) != 0xffff {
                let marks = at.wrapping_add(0x22);
                mem.write(marks, mem.read::<u8>(marks) & 0xfc);
            }
            at = at.wrapping_add(layout::NODE_SIZE);
        }
        let f = mem.rebase(ITEM_APPLY_AT);
        call(mem, f, &[chr, node, item, 3]);
        let [state, draw] = ITEM_TARGET_HOOKS.map(|f| mem.rebase(f));
        for (o, v) in [
            (0x116a0, 0),
            (0x1169c, 0),
            (0x115a8, state),
            (0x115ac, draw),
        ] {
            mem.write(ABMAP.get(mem).wrapping_add(o), v);
        }
        let markers = ABMAP.get(mem).wrapping_add(0x11088);
        let current = u32::from(mem.read::<u8>(markers.wrapping_add(0x534)));
        let f = mem.rebase(FIND_NEAREST_NODE);
        let filter = mem.rebase(MOVE_CHECK);
        let found = call(
            mem,
            f,
            &[work, markers.wrapping_add(current * 0x50), filter],
        );
        let g = ABMAP.get(mem);
        let offset = found
            .wrapping_sub(g)
            .wrapping_sub(layout::NODES)
            .cast_signed();
        let index = (offset / 0x28).cast_unsigned();
        let mut eax = 0x6666_6667u32.wrapping_mul(offset.cast_unsigned());
        if found != 0 {
            let on = u32::from(mem.read::<u16>(g.wrapping_add(0x112fc)));
            eax = on;
            if index != on {
                let half = x87::copy(mem.read::<u32>(mem.rebase(super::WARP_PART_SCALE)));
                eax = abmap_init_homing(mem, index, half);
                let g = ABMAP.get(mem);
                if mem.read::<u32>(g.wrapping_add(0x115b0)) == 0 {
                    let state = mem.read::<u32>(g.wrapping_add(0x115a8));
                    mem.write(g.wrapping_add(0x115b0), state);
                    let home = mem.rebase(super::STATE_HOME_ON_NODE);
                    mem.write(ABMAP.get(mem).wrapping_add(0x115a8), home);
                    eax = ABMAP.get(mem);
                }
            }
        }
        mem.write(ABMAP.get(mem).wrapping_add(0x115c3), 1u8);
        eax
    }

    /// The sphere grid file's buffer (`0x01740870`): `+0` `'1'` when
    /// loaded, `+2`/`+4`/`+6` the kind, node and link counts (`u16`),
    /// the records from `+0x10`.
    pub const ABMAP_FILE: u32 = 0x0174_0870;

    /// `count` bytes from `src` to `dst`, one at a time from the first.
    fn copy_bytes(mem: &mut impl Mem, dst: u32, src: u32, count: u32) {
        for i in 0..count {
            let b = mem.read::<u8>(src.wrapping_add(i));
            mem.write(dst.wrapping_add(i), b);
        }
    }

    /// `LoadAbmap()` (`0x00a45570`): the save's grid kind (`MsGetSaveMenu`
    /// bits 14..15) picks file 9, 10 or 11, read through `GetCD`'s
    /// reader (`+0x10(0x25)`, `+0(file, buffer, 0, 0)`, `+8(0)`) into
    /// [`ABMAP_FILE`]. When it starts with `'1'`, the three counts go to
    /// the grid's `+0`, `+2`, `+4` and, from `+0x10` of the file in turn:
    /// the kinds (16 bytes each, to `+8`), the nodes (12 bytes each to
    /// `+0x808`, 0x28 apart; `+0x26` the low 10 bits of the nodes left
    /// after it, bit-reversed, shifted up 6, and `+0xa` its cell
    /// `(x + 0xa00) / 256 + 20 ((y + 0x920) / 256)`), the links (8 bytes
    /// each to `+0xa808`, 0x14 apart, kept only when its two nodes differ
    /// and both are below the node count, then [`abmap_link_cells`]; the
    /// kept count to `+4`). Then [`abmap_collect_node_links`] and
    /// [`abmap_calc_bounds`], whose result it returns.
    pub fn load_abmap<M: Mem, C: FnMut(&mut M, u32, &[u32]) -> u32>(
        mem: &mut M,
        call: &mut C,
    ) -> u32 {
        let f = mem.rebase(super::MS_GET_SAVE_MENU);
        let menu = call(mem, f, &[]);
        let file = mem.rebase(ABMAP_FILE);
        mem.write(file, 0u8);
        let id = match mem.read::<u32>(menu).cast_signed() >> 14 & 3 {
            0 => 9,
            1 => 10,
            _ => 11,
        };
        let f = mem.rebase(super::GET_CD);
        let cd = call(mem, f, &[]);
        let f = mem.read::<u32>(cd.wrapping_add(0x10));
        call(mem, f, &[0x25]);
        let f = mem.read::<u32>(cd);
        call(mem, f, &[id, file, 0, 0]);
        let f = mem.read::<u32>(cd.wrapping_add(8));
        call(mem, f, &[0]);
        if mem.read::<u8>(file) == b'1' {
            load_records(mem, file);
        }
        abmap_collect_node_links(mem);
        abmap_calc_bounds(mem)
    }

    /// The records of [`load_abmap`]'s file.
    fn load_records(mem: &mut impl Mem, file: u32) {
        for o in [0u32, 2, 4] {
            let n = mem.read::<u16>(file.wrapping_add(2 + o));
            mem.write(ABMAP.get(mem).wrapping_add(o), n);
        }
        let mut src = file.wrapping_add(0x10);
        let g = ABMAP.get(mem);
        let mut dst = g.wrapping_add(8);
        let mut left = i32::from(mem.read::<i16>(g));
        while left != 0 {
            left = left.wrapping_sub(1);
            copy_bytes(mem, dst, src, 0x10);
            src = src.wrapping_add(0x10);
            dst = dst.wrapping_add(0x10);
        }
        let g = ABMAP.get(mem);
        let mut node = g.wrapping_add(layout::NODES);
        let mut left = i32::from(mem.read::<i16>(g.wrapping_add(2)));
        while left != 0 {
            left = left.wrapping_sub(1);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 10 bits
            let flags = ((left.cast_unsigned() & 0x3ff) as u16).reverse_bits() >> 6;
            copy_bytes(mem, node, src, 0xc);
            src = src.wrapping_add(0xc);
            let y = i32::from(mem.read::<i16>(node.wrapping_add(2)));
            mem.write(node.wrapping_add(0x26), flags << 6);
            let x = i32::from(mem.read::<i16>(node));
            let cell = (x + 0xa00) / 0x100 + (y + 0x920) / 0x100 * 20;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // the low word
            mem.write(node.wrapping_add(0xa), cell as u16);
            node = node.wrapping_add(layout::NODE_SIZE);
        }
        let g = ABMAP.get(mem);
        let mut link = g.wrapping_add(0xa808);
        let mut left = i32::from(mem.read::<i16>(g.wrapping_add(4)));
        let mut kept = 0u16;
        while left != 0 {
            left = left.wrapping_sub(1);
            copy_bytes(mem, link, src, 8);
            src = src.wrapping_add(8);
            let (a, b) = (mem.read::<u16>(link), mem.read::<u16>(link.wrapping_add(2)));
            let nodes = i32::from(mem.read::<i16>(ABMAP.get(mem).wrapping_add(2)));
            if a != b && i32::from(a) < nodes && i32::from(b) < nodes {
                abmap_link_cells(mem, link);
                link = link.wrapping_add(layout::LINK_SIZE);
                kept = kept.wrapping_add(1);
            }
        }
        mem.write(ABMAP.get(mem).wrapping_add(4), kept);
    }
}
