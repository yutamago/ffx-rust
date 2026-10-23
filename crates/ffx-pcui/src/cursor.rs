//! The sphere grid cursor: the stick or the d-pad to a move
//! (`0x00a56bd0`), the nearest node (`0x00a56a40`) and the best node in a
//! direction (`0x00a56860`). Floats follow the x87 code: 53-bit
//! arithmetic, rounded to `f32` where the original stores.

#![allow(clippy::many_single_char_names)] // vector components, stick axes

use ffx_bridge::x87::{self, Reg};

/// The stick's acceleration (`g_Abmap + 0x115e4`) after a frame: up 0.1
/// (to at most 2) while the stick is pushed far (`x² + y² > 0x3840`),
/// else down 0.1 (to at least 1); the step is stored before the clamp, a
/// NaN stays.
#[must_use]
pub fn stick_speed(speed: f32, far: bool) -> f32 {
    if far {
        let s = (Reg::load(speed) + Reg::constant(0.1)).store();
        if 2.0 < f64::from(s) { 2.0 } else { s }
    } else {
        let s = (Reg::load(speed) - Reg::constant(0.1)).store();
        if 1.0 > f64::from(s) { 1.0 } else { s }
    }
}

/// A stick axis (`0x80` centred, already less `0x80`) past its dead zone:
/// beyond ±0x40 less 0x40, else 0 (byte arithmetic).
#[must_use]
pub const fn stick_axis(v: i8) -> i8 {
    if v > 0x40 {
        v.wrapping_sub(0x40)
    } else if v < -0x40 {
        v.wrapping_add(0x40)
    } else {
        0
    }
}

/// The analog move: each axis past its dead zone, times 0.125, times the
/// acceleration (`x * 0.125 * s`, `0.125 * y * s`).
#[must_use]
pub fn analog_move(x: i8, y: i8, speed: f32) -> [f32; 2] {
    let k = Reg::constant(0.125);
    let s = Reg::load(speed);
    let mx = (Reg::constant(f64::from(stick_axis(x))) * k * s).store();
    let my = (k * Reg::constant(f64::from(stick_axis(y))) * s).store();
    [mx, my]
}

/// The d-pad move for the held buttons `held` (`g_Abmap + 0x1166c`):
/// `0x8000` left (-1) or `0x2000` right (1), `0x1000` up (-1) or `0x4000`
/// down (1); `None` for an axis not pressed.
#[must_use]
pub fn pad_move(held: u16) -> [Option<f32>; 2] {
    let x = if held & 0x8000 != 0 {
        Some(-1.0)
    } else if held & 0x2000 != 0 {
        Some(1.0)
    } else {
        None
    };
    let y = if held & 0x1000 != 0 {
        Some(-1.0)
    } else if held & 0x4000 != 0 {
        Some(1.0)
    } else {
        None
    };
    [x, y]
}

/// The emulated VU0 length the node searches compute inline in `vf10`
/// (`0x00c8f830`) and `Q` (`0x00c8f788`): the components squared and
/// summed in place (`x²`, `x² + y²`, `x² + y² + z²`), `Q = sqrt|sum|`,
/// then `vf00.x + Q` over `vf10.x`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vu0Length {
    /// `vf10` after: (`vf00.x + Q`, `x² + y²`, `x² + y² + z²`, w).
    pub vf10: [f32; 4],
    /// `Q`.
    pub q: f32,
}

/// [`Vu0Length`] of `(x, y, z, w)` with `vf00.x` = `vf00x`.
#[must_use]
pub fn vu0_length(v: [f32; 4], vf00x: f32) -> Vu0Length {
    let [x, y, z, w] = v;
    let xx = (Reg::load(x) * Reg::load(x)).store();
    let yy = Reg::load(y).mul_m32(y).store();
    let zz = Reg::load(z).mul_m32(z).store();
    let sxy = Reg::load(xx).add_m32(yy).store();
    let sum = Reg::load(sxy).add_m32(zz).store();
    let abs = f32::from_bits(x87::store(x87::load(sum.to_bits()).abs()));
    let q = f32::from_bits(x87::store(x87::sqrt(x87::load(abs.to_bits()))));
    let len = Reg::load(vf00x).add_m32(q).store();
    Vu0Length {
        vf10: [len, sxy, sum, w],
        q,
    }
}

/// A node's position (`i16` x, y) less `pos`, each rounded to `f32`.
#[must_use]
pub fn node_offset(node: [i16; 2], pos: [f32; 2]) -> [f32; 2] {
    [
        Reg::constant(f64::from(node[0])).sub_m32(pos[0]).store(),
        Reg::constant(f64::from(node[1])).sub_m32(pos[1]).store(),
    ]
}

/// `FFXVu0InnerProduct(a, b)`: `(a.y b.y + a.x b.x) + a.z b.z`, stored.
#[must_use]
pub fn inner_product(a: [f32; 3], b: [f32; 3]) -> f32 {
    let xy = Reg::load(a[1]).mul_m32(b[1]) + Reg::load(a[0]).mul_m32(b[0]);
    (xy + Reg::load(a[2]).mul_m32(b[2])).store()
}

/// The direction search's score: the projection `dot` over the distance
/// squared (`dot / (d * d)`, the square not stored).
#[must_use]
pub fn direction_score(dot: f32, d: f32) -> f32 {
    (Reg::load(dot) / (Reg::load(d) * Reg::load(d))).store()
}

/// The originals, where the game keeps their state (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, x87};
    use ffx_graphic::host::{Conv, Host};

    use super::{Vu0Length, node_offset, vu0_length};
    use crate::abmap::bridge::{ABMAP, TO_MENU_GET_CONTROL_PAD_ANALOG_HIST};

    /// `g_Vu0_Vf00`: the emulated VU0's `vf00` (`x` first).
    pub const VU0_VF00: Global<f32> = Global::new(0x00c0_a004);
    /// `g_Vu0_Q`.
    pub const VU0_Q: Global<f32> = Global::new(0x00c8_f788);
    /// The emulated VU0's `vf10` (four `f32`).
    pub const VU0_VF10: u32 = 0x00c8_f830;

    /// `AbmapReadCursorStick(move)` (`0x00a56bd0`): `move` (4 floats)
    /// gets the cursor's move for this frame. With the left stick
    /// (`TOMenuGetControlPadAnalogHist(0, 0, 2 | 3, -1)`) out of
    /// `0x41..=0xbf` on either axis: the acceleration updated
    /// ([`super::stick_speed`]), the move ([`super::analog_move`]) into
    /// `move[0..2]`, and 1. Otherwise the d-pad ([`super::pad_move`]) into
    /// `move[0..2]` (0 where not pressed), `move[2..4]` = (0, 1), 0; a
    /// press on one axis only waits a frame (counter byte
    /// `g_Abmap + 0x115ca`, up to 2, cleared with nothing pressed; while
    /// it is below 2 the move is 0).
    pub fn read_cursor_stick(mem: &mut impl Host, mv: u32) -> u32 {
        mem.write(mv.wrapping_add(4), 0f32);
        mem.write(mv, 0f32);
        let hist = TO_MENU_GET_CONTROL_PAD_ANALOG_HIST;
        let a = mem.call_sym(hist, Conv::Cdecl, &[0, 0, 2, u32::MAX]);
        let b = mem.call_sym(hist, Conv::Cdecl, &[0, 0, 3, u32::MAX]);
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        let (x, y) = (a.wrapping_sub(0x80) as i8, b.wrapping_sub(0x80) as i8);
        #[allow(clippy::cast_possible_truncation)]
        let pushed = |v: u32| (v as u8).wrapping_sub(0x41) > 0x7e;
        if pushed(a) || pushed(b) {
            let far = i32::from(x) * i32::from(x) + i32::from(y) * i32::from(y) > 0x3840;
            let at = ABMAP.get(mem).wrapping_add(0x1_15e4);
            let s: f32 = mem.read(at);
            mem.write(at, super::stick_speed(s, far));
            let s: f32 = mem.read(at);
            let [mx, my] = super::analog_move(x, y, s);
            mem.write(mv, mx);
            mem.write(mv.wrapping_add(4), my);
            return 1;
        }
        let g = ABMAP.get(mem);
        let [h, v] = super::pad_move(mem.read(g.wrapping_add(0x1_166c)));
        if let Some(h) = h {
            mem.write(mv, h);
        }
        if let Some(v) = v {
            mem.write(mv.wrapping_add(4), v);
        }
        let count = ABMAP.get(mem).wrapping_add(0x1_15ca);
        match (h.is_some(), v.is_some()) {
            (false, false) => mem.write(count, 0u8),
            (true, true) => {}
            _ => {
                let n = mem.read::<u8>(count).wrapping_add(1);
                mem.write(count, n);
                if n < 2 {
                    mem.write(mv.wrapping_add(4), 0f32);
                    mem.write(mv, 0f32);
                } else {
                    mem.write(count, 2u8);
                }
            }
        }
        mem.write(mv.wrapping_add(8), 0f32);
        mem.write(mv.wrapping_add(12), 1f32);
        0
    }

    /// The node range the searches walk: `0x28` bytes each from
    /// `g_Abmap + 0x808`, up to the `i16` count at `+2` (the end computed
    /// in 32 bits, compared unsigned).
    fn node_range(mem: &impl Host) -> (u32, u32) {
        let g = ABMAP.get(mem);
        let n: i16 = mem.read(g.wrapping_add(2));
        let end = (i32::from(n) * 5 + 0x101).cast_unsigned().wrapping_mul(8);
        (g.wrapping_add(0x808), g.wrapping_add(end))
    }

    /// Whether the search visits node `p`: it has a kind (`+6` not
    /// `0xffff`) and `filter` (a cdecl function of the node; none when 0)
    /// accepts it.
    fn visits(mem: &mut impl Host, p: u32, filter: u32) -> bool {
        mem.read::<u16>(p.wrapping_add(6)) != 0xffff
            && (filter == 0 || mem.call(filter, Conv::Cdecl, &[p]) != 0)
    }

    /// Node `p`'s position less the two floats at `pos`.
    fn offset(mem: &impl Host, p: u32, pos: u32) -> [f32; 2] {
        let node = [mem.read(p), mem.read(p.wrapping_add(2))];
        let at = [mem.read(pos), mem.read(pos.wrapping_add(4))];
        node_offset(node, at)
    }

    /// The VU0 length of `(dx, dy, 0, 1)`, left in `vf10` and `Q` as the
    /// original leaves it; returns the length.
    fn length(mem: &mut impl Host, d: [f32; 2]) -> f32 {
        let l: Vu0Length = vu0_length([d[0], d[1], 0.0, 1.0], VU0_VF00.get(mem));
        let at = mem.rebase(VU0_VF10);
        for (i, v) in (0u32..).zip(l.vf10) {
            mem.write(at + 4 * i, v);
        }
        VU0_Q.set(mem, l.q);
        l.vf10[0]
    }

    /// `AbmapFindNearestNode(dist, pos, filter)` (`0x00a56a40`): the
    /// visited node nearest `pos` (the first at the smallest distance
    /// below 1e9), its distance into `*dist` (1e9 with none); returns the
    /// node, 0 with none. Each distance goes through the VU0 scratch
    /// ([`super::vu0_length`]).
    pub fn find_nearest_node(mem: &mut impl Host, dist: u32, pos: u32, filter: u32) -> u32 {
        let (mut p, end) = node_range(mem);
        let mut best = 1.0e9f32;
        let mut found = 0;
        while p < end {
            if visits(mem, p, filter) {
                let d = offset(mem, p, pos);
                let len = length(mem, d);
                if f64::from(best) > f64::from(len) {
                    best = len;
                    found = p;
                }
            }
            p = p.wrapping_add(0x28);
        }
        mem.write(dist, best);
        found
    }

    /// `AbmapFindNodeInDirection(pos, dir, filter)` (`0x00a56860`): the
    /// visited node ahead of `pos` along `dir` (3 floats): of those whose
    /// offset projects on `dir` above 0.5 (`FFXVu0InnerProduct`, computed
    /// here as it does) at a positive distance, the first with the
    /// highest projection over distance squared
    /// ([`super::direction_score`]) above 0; returns it, 0 with none.
    pub fn find_node_in_direction(mem: &mut impl Host, pos: u32, dir: u32, filter: u32) -> u32 {
        let (mut p, end) = node_range(mem);
        let mut best = 0f32;
        let mut found = 0;
        while p < end {
            if visits(mem, p, filter) {
                let d = offset(mem, p, pos);
                let along = [
                    mem.read(dir),
                    mem.read(dir.wrapping_add(4)),
                    mem.read(dir.wrapping_add(8)),
                ];
                let dot = super::inner_product([d[0], d[1], 0.0], along);
                if f64::from(dot) > 0.5 {
                    let len = length(mem, d);
                    if 0.0 < f64::from(len) {
                        let score = super::direction_score(dot, len);
                        if f64::from(best) < f64::from(score) {
                            best = score;
                            found = p;
                        }
                    }
                }
            }
            p = p.wrapping_add(0x28);
        }
        found
    }

    /// `AbmapReadCursorStick(move)`, `AbmapFindNearestNode(dist, pos,
    /// filter)`, `AbmapFindNodeInDirection(pos, dir, filter)`,
    /// `AbmapNodeRadius(node)` (`ST0`), `AbmapTransitionToHoming(node,
    /// t)` and `SndSepPlaySimple(id)`.
    pub const ABMAP_READ_CURSOR_STICK: u32 = 0x00a5_6bd0;
    pub const ABMAP_FIND_NEAREST_NODE: u32 = 0x00a5_6a40;
    pub const ABMAP_FIND_NODE_IN_DIRECTION: u32 = 0x00a5_6860;
    pub const ABMAP_NODE_RADIUS: u32 = 0x00a5_6f30;
    pub const ABMAP_TRANSITION_TO_HOMING: u32 = 0x00a4_8d20;
    pub const SND_SEP_PLAY_SIMPLE: u32 = 0x0088_6e80;
    /// The cursor sound.
    pub const SFX_MOVE: u32 = 0x8000_0001;
    /// Bytes of scratch [`free_cursor_step`] needs, laid out as the
    /// original's frame from `ebp - 0x18`: the nearest node's distance,
    /// then the move (4 floats).
    pub const FREE_CURSOR_WORK: u32 = 0x14;

    /// The node record `p` is (records of 0x28 bytes from `g_Abmap +
    /// 0x808`, `g_Abmap` read now), as the original's `imul` by
    /// `0x66666667` divides: toward zero.
    fn node_index(mem: &impl Host, p: u32) -> i32 {
        let off = p.wrapping_sub(ABMAP.get(mem)).wrapping_sub(0x808);
        off.cast_signed() / 0x28
    }

    /// The cursor sound, then `AbmapTransitionToHoming(node of p, t)`.
    fn home_on(mem: &mut impl Host, p: u32, t: f32) {
        mem.call_sym(SND_SEP_PLAY_SIMPLE, Conv::Cdecl, &[SFX_MOVE]);
        let node = node_index(mem, p).cast_unsigned();
        mem.call_sym(
            ABMAP_TRANSITION_TO_HOMING,
            Conv::Cdecl,
            &[node, t.to_bits()],
        );
    }

    /// `AbmapFreeCursorStep_00a58ff0(filter)` (name ours; from the
    /// sphere grid's free-cursor states `0x00a44ef0` and
    /// `abmCalcChrMoveCurMoveCheck`): one frame of the free cursor at
    /// `g_Abmap + 0x112b8`, nodes visited as `filter` says. Nothing while
    /// `+0x115b0` is set; a negative speed `+0x115e4` becomes 1.
    ///
    /// When `AbmapReadCursorStick` reports no stick move: right after a
    /// stick move (byte `+0x115cd`, then cleared) the cursor homes on the
    /// nearest node (t 0.5), else, with a d-pad move (either axis not 0,
    /// NaN counts), on the node in that direction (t 0.25), each with the
    /// cursor sound; no node, nothing. With a stick move: the flag set,
    /// the move added (rounded to floats), x clamped to `+0x1160c ..=
    /// +0x11614` and y to `+0x11610 ..= +0x11618` (a NaN bound or
    /// position is left alone), the position (4 words) copied to
    /// `+0x11308`; when the nearest node lies within the cursor radius
    /// `+0x112f4` it becomes the cursor node `+0x112fc` (`u16`) and the
    /// radius moves 2 toward the node's (`AbmapNodeRadius`), not past it.
    /// The nearest node is not checked for null there. `eax` is the
    /// last callee's (the stack cookie check leaves it; callers read none).
    pub fn free_cursor_step(mem: &mut impl Host, filter: u32, work: u32) {
        let g = ABMAP.get(mem);
        let pos = g.wrapping_add(0x1_12b8);
        if mem.read::<u32>(g.wrapping_add(0x1_15b0)) != 0 {
            return;
        }
        let speed = g.wrapping_add(0x1_15e4);
        if mem.read::<f32>(speed) < 0.0 {
            mem.write(speed, 1.0f32);
        }
        let (dist, mv) = (work, work.wrapping_add(4));
        let stick = mem.call_sym(ABMAP_READ_CURSOR_STICK, Conv::Cdecl, &[mv]);
        let g2 = ABMAP.get(mem);
        let after_stick = g2.wrapping_add(0x1_15cd);
        if stick == 0 {
            if mem.read::<u8>(after_stick) != 0 {
                mem.write(after_stick, 0u8);
                let near = mem.call_sym(ABMAP_FIND_NEAREST_NODE, Conv::Cdecl, &[dist, pos, filter]);
                if near != 0 {
                    home_on(mem, near, 0.5);
                }
            } else {
                let (dx, dy) = (mem.read::<f32>(mv), mem.read::<f32>(mv.wrapping_add(4)));
                if dx != 0.0 || dy != 0.0 {
                    let to = mem.call_sym(
                        ABMAP_FIND_NODE_IN_DIRECTION,
                        Conv::Cdecl,
                        &[pos, mv, filter],
                    );
                    if to != 0 {
                        home_on(mem, to, 0.25);
                    }
                }
            }
            return;
        }
        mem.write(after_stick, 1u8);
        for k in 0..2 {
            let at = pos.wrapping_add(4 * k);
            let v = x87::add(
                f64::from(mem.read::<f32>(at)),
                f64::from(mem.read::<f32>(mv.wrapping_add(4 * k))),
            );
            mem.write(at, x87::store(v));
        }
        let g3 = ABMAP.get(mem);
        for (k, lo, hi) in [(0, 0x1_160c, 0x1_1614), (1, 0x1_1610, 0x1_1618)] {
            let at = pos.wrapping_add(4 * k);
            let v: f32 = mem.read(at);
            let min: u32 = mem.read(g3.wrapping_add(lo));
            let max: u32 = mem.read(g3.wrapping_add(hi));
            if f32::from_bits(min) > v {
                mem.write(at, min);
            } else if f32::from_bits(max) < v {
                mem.write(at, max);
            }
        }
        let g4 = ABMAP.get(mem);
        for k in 0..4 {
            let w: u32 = mem.read(g4.wrapping_add(0x1_12b8 + 4 * k));
            mem.write(g4.wrapping_add(0x1_1308 + 4 * k), w);
        }
        let near = mem.call_sym(ABMAP_FIND_NEAREST_NODE, Conv::Cdecl, &[dist, pos, filter]);
        let radius = pos.wrapping_add(0x3c);
        // farther, or a NaN on either side: nothing
        let within = mem.read::<f32>(dist) <= mem.read::<f32>(radius);
        if !within {
            return;
        }
        let node = node_index(mem, near).to_le_bytes();
        let node = u16::from_le_bytes([node[0], node[1]]);
        mem.write(pos.wrapping_add(0x44), node);
        let target = mem.call_sym_st0(ABMAP_NODE_RADIUS, Conv::Cdecl, &[u32::from(node)]);
        let r: f32 = mem.read(radius);
        if target < r {
            let r2 = f32::from_bits(x87::store(x87::sub(f64::from(r), 2.0)));
            mem.write(radius, if r2 < target { target } else { r2 });
        } else if r < target {
            let r2 = f32::from_bits(x87::store(x87::add(f64::from(r), 2.0)));
            mem.write(radius, if r2 > target { target } else { r2 });
        }
    }
}
