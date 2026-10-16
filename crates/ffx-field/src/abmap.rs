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
/// The sine table (4096 `f32`s, a quarter turn in 1024).
pub const SIN_TABLE: u32 = 0x00c4_4bf0;

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
}
