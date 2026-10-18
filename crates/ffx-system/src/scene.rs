//! `TOSeqGet*`: numbers, colours, points and rectangles of a menu scene's
//! named parameter, through the menu's parameter lookups
//! (`TkMenuGetParamHandle`, `TkMenuGetParamOffset`), which the caller
//! passes in (they live in `ffx-menu`).

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    /// Parameter `name` of scene `i`: its values `0..outs.len()` stored at
    /// `outs` (`u16` each unless `wide`), each right after it is looked up;
    /// all 0 when the parameter is missing.
    fn get<M: Mem>(
        mem: &mut M,
        i: i32,
        name: u32,
        outs: &[u32],
        wide: bool,
        handle: impl Fn(&M, i32, u32) -> u32,
        offset: impl Fn(&M, i32, i32, i32) -> u32,
    ) {
        let h = handle(mem, i, name);
        for (n, &out) in (0..).zip(outs) {
            let v = if h == 0 {
                0
            } else {
                offset(mem, i, h.cast_signed(), n)
            };
            if wide {
                mem.write(out, v);
            } else {
                #[allow(clippy::cast_possible_truncation)] // the original stores AX
                mem.write(out, v as u16);
            }
        }
    }

    /// `TOSeqGetNum(i, name, &n)` (`0x008ae410`).
    pub fn get_num<M: Mem>(
        mem: &mut M,
        i: i32,
        name: u32,
        out: u32,
        handle: impl Fn(&M, i32, u32) -> u32,
        offset: impl Fn(&M, i32, i32, i32) -> u32,
    ) {
        get(mem, i, name, &[out], false, handle, offset);
    }

    /// `TOSeqGetColor(i, name, &rgba)` (`0x008ae360`): a whole word.
    pub fn get_color<M: Mem>(
        mem: &mut M,
        i: i32,
        name: u32,
        out: u32,
        handle: impl Fn(&M, i32, u32) -> u32,
        offset: impl Fn(&M, i32, i32, i32) -> u32,
    ) {
        get(mem, i, name, &[out], true, handle, offset);
    }

    /// `TOSeqGetPoint(i, name, &x, &y)` (`0x008ae450`).
    pub fn get_point<M: Mem>(
        mem: &mut M,
        i: i32,
        name: u32,
        outs: [u32; 2],
        handle: impl Fn(&M, i32, u32) -> u32,
        offset: impl Fn(&M, i32, i32, i32) -> u32,
    ) {
        get(mem, i, name, &outs, false, handle, offset);
    }

    /// `TOSeqGetRect(i, name, &x, &y, &w, &h)` (`0x008ae4b0`).
    pub fn get_rect<M: Mem>(
        mem: &mut M,
        i: i32,
        name: u32,
        outs: [u32; 4],
        handle: impl Fn(&M, i32, u32) -> u32,
        offset: impl Fn(&M, i32, i32, i32) -> u32,
    ) {
        get(mem, i, name, &outs, false, handle, offset);
    }

    /// The scenes' sequence slots (`0x01865f20`): 0x100 bytes per scene,
    /// eight 0x20-byte slots of a state (`+0`, u16: 0 free, 1..=3 running,
    /// 4 stopped), a draw-continue flag (`+4`, u16), a name (`+8`) and a
    /// handle (`+0x1c`).
    pub const SLOTS: ffx_bridge::Global<[u8; 0x20]> = ffx_bridge::Global::new(0x0186_5f20);

    /// Slot `j` of scene `i`.
    fn slot(mem: &impl Mem, i: i32, j: u32) -> u32 {
        SLOTS
            .ptr(mem)
            .addr
            .wrapping_add(i.cast_unsigned().wrapping_shl(8))
            .wrapping_add(j * 0x20)
    }

    /// `TOMenuExecLayerHandle(i, a, handle, b)` (`0x00a7b220`).
    const EXEC_LAYER_HANDLE: u32 = 0x00a7_b220;

    /// `TOSeqDraw(i, a, b)` (`0x008ae2f0`): each slot of scene `i` in state
    /// 2 or 3, in slot order, runs `TOMenuExecLayerHandle(i, a, handle, b)`
    /// on its handle (`+0x1c`). Returns what the original leaves in `eax`:
    /// the last slot's call result, or its state when it is not drawn.
    pub fn draw<M: Mem>(
        mem: &mut M,
        i: i32,
        a: u32,
        b: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let mut eax = 0;
        for j in 0..8 {
            let s = slot(mem, i, j);
            let state: u16 = mem.read(s);
            eax = u32::from(state);
            if state == 2 || state == 3 {
                let handle: u32 = mem.read(s.wrapping_add(0x1c));
                eax = call(mem, EXEC_LAYER_HANDLE, &[i.cast_unsigned(), a, handle, b]);
            }
        }
        eax
    }

    /// Whether the C strings at `a` and `b` are equal.
    fn str_eq(mem: &impl Mem, left: u32, right: u32) -> bool {
        let mut k = 0u32;
        loop {
            let (lb, rb): (u8, u8) = (
                mem.read(left.wrapping_add(k)),
                mem.read(right.wrapping_add(k)),
            );
            if lb != rb {
                return false;
            }
            if lb == 0 {
                return true;
            }
            k += 1;
        }
    }

    /// The first slot of scene `i` named `name`.
    fn find(mem: &impl Mem, i: i32, name: u32) -> Option<u32> {
        (0..8)
            .map(|j| slot(mem, i, j))
            .find(|&s| str_eq(mem, name, s.wrapping_add(8)))
    }

    fn running(mem: &impl Mem, slot: u32) -> bool {
        matches!(mem.read::<u16>(slot), 1..=3)
    }

    /// `TOSeqCheckTermScene(i, name)` (`0x008ae070`): whether the sequence
    /// `name` of scene `i` has ended (`TkMenuIsTermScene` on its handle,
    /// passed in); 0 when no slot has the name.
    pub fn check_term_scene<M: Mem>(
        mem: &mut M,
        i: i32,
        name: u32,
        is_term: impl Fn(&M, i32, u32) -> u32,
    ) -> u32 {
        match find(mem, i, name) {
            Some(s) => {
                let handle: u32 = mem.read(s.wrapping_add(0x1c));
                u32::from(is_term(mem, i, handle) != 0)
            }
            None => 0,
        }
    }

    /// `TOSeqStartScene(i, name)` (`0x008af0d0`) and, with `draw_continue`,
    /// `TOSeqStartSceneDrawContinue` (`0x008af1b0`): unless a slot already
    /// has the name, the first free slot gets `TkMenuGetSceneHandle(i,
    /// name)` (passed in; stored even when 0) and, when that is not 0,
    /// state 1 (and the draw-continue flag) and the name. 1 when started.
    pub fn start_scene<M: Mem>(
        mem: &mut M,
        i: i32,
        name: u32,
        draw_continue: bool,
        scene_handle: impl Fn(&M, i32, u32) -> u32,
    ) -> u32 {
        if find(mem, i, name).is_some() {
            return 0;
        }
        let Some(s) = (0..8)
            .map(|j| slot(mem, i, j))
            .find(|&s| mem.read::<u16>(s) == 0)
        else {
            return 0;
        };
        let handle = scene_handle(mem, i, name);
        mem.write(s.wrapping_add(0x1c), handle);
        if handle == 0 {
            return 0;
        }
        mem.write(s, 1u16);
        if draw_continue {
            mem.write(s.wrapping_add(4), 1u16);
        }
        let mut k = 0u32;
        loop {
            let c: u8 = mem.read(name.wrapping_add(k));
            mem.write(s.wrapping_add(8 + k), c);
            if c == 0 {
                return 1;
            }
            k += 1;
        }
    }

    /// `TOSeqStopScene(i, name)` (`0x008af4f0`): the first running slot
    /// named `name` stopped (state 4, name cleared); 1 when one was.
    pub fn stop_scene(mem: &mut impl Mem, i: i32, name: u32) -> u32 {
        for j in 0..8 {
            let s = slot(mem, i, j);
            if running(mem, s) && str_eq(mem, name, s.wrapping_add(8)) {
                mem.write(s, 4u16);
                mem.write(s.wrapping_add(8), 0u8);
                return 1;
            }
        }
        0
    }

    /// `TOSeqInitAllScene(i)` (`0x008ae5a0`): every running slot of scene
    /// `i` stopped (state 4, name cleared). Returns `i << 8` (left in `eax`).
    pub fn init_all_scene(mem: &mut impl Mem, i: i32) -> u32 {
        for j in 0..8 {
            let s = slot(mem, i, j);
            if running(mem, s) {
                mem.write(s, 4u16);
                mem.write(s.wrapping_add(8), 0u8);
            }
        }
        i.cast_unsigned().wrapping_shl(8)
    }

    /// `0x008adf20()`: the slots of the first three scenes cleared (state
    /// and name's first byte); returns the end (`0x01866228`).
    pub fn clear_slots(mem: &mut impl Mem) -> u32 {
        for i in 0..3 {
            for j in 0..8 {
                let s = slot(mem, i, j);
                mem.write(s, 0u16);
                mem.write(s.wrapping_add(8), 0u8);
            }
        }
        mem.rebase(0x0186_6228)
    }

    /// `0x008ae530(i, name)`: the state of the slot of scene `i` named
    /// `name` (sign-extended), 0 when none.
    pub fn scene_state(mem: &impl Mem, i: i32, name: u32) -> i32 {
        find(mem, i, name).map_or(0, |s| i32::from(mem.read::<i16>(s)))
    }

    /// `0x008af060(i, name, x, y)`: every slot of scene `i` named `name`
    /// (all eight checked) moved with `TkMenuSetSceneOffset(i, handle, x, y)`
    /// (the menu's, passed in) on its handle (`+0x1c`).
    pub fn set_scene_pos_by_name<M: Mem>(
        mem: &mut M,
        i: i32,
        name: u32,
        x: u32,
        y: u32,
        mut set_pos: impl FnMut(&mut M, i32, u32, u32, u32),
    ) {
        for j in 0..8 {
            let s = slot(mem, i, j);
            if str_eq(mem, name, s.wrapping_add(8)) {
                let handle: u32 = mem.read(s.wrapping_add(0x1c));
                set_pos(mem, i, handle, x, y);
            }
        }
    }

    /// `TOSeqGetColorRGBA(i, name, &r, &g, &b, &a)` (`0x008ae3a0`): the
    /// colour parameter `name` of scene `i` (`TkMenuGetParamHandle`, then
    /// `TkMenuGetParamOffset(i, handle, 0)`, the menu's, passed in) split
    /// into its four bytes, or all 0 without the parameter. Returns `a`'s
    /// address (left in `eax`).
    pub fn get_color_rgba<M: Mem>(
        mem: &mut M,
        i: i32,
        name: u32,
        out: [u32; 4],
        get_handle: impl FnOnce(&mut M, i32, u32) -> u32,
        get_offset: impl FnOnce(&mut M, i32, u32, u32) -> u32,
    ) -> u32 {
        let h = get_handle(mem, i, name);
        let c = if h == 0 { 0 } else { get_offset(mem, i, h, 0) };
        for (k, &p) in out.iter().enumerate() {
            mem.write(p, c.to_le_bytes()[k]);
        }
        out[3]
    }

    /// `0x008ae000(i, name)`: 1 when one of scene `i`'s eight slots is
    /// named `name` (any state), else 0.
    pub fn has_scene(mem: &impl Mem, i: i32, name: u32) -> u32 {
        u32::from(find(mem, i, name).is_some())
    }

    /// `TOSeqInitScene(i)` (`0x008aeec0`): every running slot of scene `i`
    /// stopped (state 4, name cleared) except those kept (`u16` 1 at
    /// `+2`). Returns `i << 8` (left in `eax`).
    pub fn init_scene(mem: &mut impl Mem, i: i32) -> u32 {
        for j in 0..8 {
            let s = slot(mem, i, j);
            if mem.read::<u16>(s.wrapping_add(2)) != 1 && running(mem, s) {
                mem.write(s, 4u16);
                mem.write(s.wrapping_add(8), 0u8);
            }
        }
        i.cast_unsigned().wrapping_shl(8)
    }

    /// What a `TOSeqInitParam` entry reads (the `TOSeqGet*` getter).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Param {
        /// `TOSeqGetColor(i, name, &rgba)`.
        Color,
        /// `TOSeqGetColorRGBA(i, name, &r, &g, &b, &a)`.
        Rgba,
        /// `TOSeqGetRect(i, name, &x, &y, &w, &h)`.
        Rect,
        /// `TOSeqGetNum(i, name, &n)`.
        Num,
        /// `TOSeqGetPoint(i, name, &x, &y)`.
        Point,
    }

    /// An output the original writes into its own stack frame (a rect's
    /// height nobody keeps); [`init_param`] gets a scratch word for it.
    pub const LOCAL: u32 = 0;

    /// `TOSeqInitParam`'s 92 reads in order: (getter, scene, parameter
    /// name, output globals), taken from the call sequence at
    /// `0x008ae6f0`.
    pub const INIT_PARAMS: [(Param, i32, u32, &[u32]); 92] = [
        (Param::Color, 1, 0x00b6_27e0, &[0x025d_16b4]),
        (Param::Color, 1, 0x00b6_27f0, &[0x025d_16b0]),
        (Param::Color, 1, 0x00b6_2800, &[0x025d_16a4]),
        (Param::Color, 1, 0x00b6_2808, &[0x025d_16a8]),
        (Param::Color, 1, 0x00b5_ed34, &[0x025d_16ac]),
        (Param::Color, 1, 0x00b6_2810, &[0x025d_16a0]),
        (Param::Color, 1, 0x00b6_281c, &[0x025d_169c]),
        (Param::Color, 1, 0x00b6_2828, &[0x025d_1698]),
        (Param::Color, 1, 0x00b6_2838, &[0x025d_1694]),
        (Param::Color, 1, 0x00b6_2848, &[0x025d_1690]),
        (Param::Color, 1, 0x00b6_2854, &[0x025d_168c]),
        (Param::Color, 1, 0x00b6_2860, &[0x025d_1688]),
        (Param::Color, 1, 0x00b6_286c, &[0x025d_1684]),
        (
            Param::Rgba,
            1,
            0x00b6_2878,
            &[0x025d_1683, 0x025d_1681, 0x025d_1682, 0x025d_1680],
        ),
        (
            Param::Rgba,
            1,
            0x00b6_2884,
            &[0x025d_167f, 0x025d_167d, 0x025d_167e, 0x025d_167c],
        ),
        (
            Param::Rgba,
            1,
            0x00b6_2890,
            &[0x025d_167b, 0x025d_1679, 0x025d_167a, 0x025d_1678],
        ),
        (
            Param::Rect,
            1,
            0x00b6_289c,
            &[0x025d_1676, 0x025d_1674, 0x025d_1672, LOCAL],
        ),
        (Param::Num, 1, 0x00b6_28ac, &[0x025d_1670]),
        (Param::Color, 1, 0x00b6_28bc, &[0x025d_166c]),
        (Param::Color, 1, 0x00b6_28cc, &[0x025d_1668]),
        (Param::Point, 1, 0x00b6_28dc, &[0x025d_1664, 0x025d_1662]),
        (Param::Point, 1, 0x00b6_28e8, &[0x025d_1660, 0x025d_165e]),
        (Param::Point, 1, 0x00b6_28f4, &[0x025d_165c, 0x025d_165a]),
        (Param::Point, 1, 0x00b6_2900, &[0x025d_1658, 0x025d_1656]),
        (Param::Point, 1, 0x00b6_290c, &[0x025d_1654, 0x025d_1652]),
        (Param::Point, 1, 0x00b6_2918, &[0x025d_1650, 0x025d_164e]),
        (Param::Point, 1, 0x00b6_2928, &[0x025d_164c, 0x025d_164a]),
        (
            Param::Rect,
            1,
            0x00b6_2934,
            &[0x025d_1648, 0x025d_1646, 0x025d_1644, 0x025d_1642],
        ),
        (Param::Color, 1, 0x00b6_2940, &[0x025d_163c]),
        (Param::Color, 1, 0x00b6_294c, &[0x025d_1638]),
        (Param::Num, 1, 0x00b6_2958, &[0x025d_1640]),
        (Param::Point, 1, 0x00b6_2968, &[0x025d_1634, 0x025d_1632]),
        (
            Param::Rect,
            1,
            0x00b6_2974,
            &[0x025d_1630, 0x025d_162e, 0x025d_162c, LOCAL],
        ),
        (Param::Color, 1, 0x00b6_2984, &[0x025d_1628]),
        (Param::Color, 1, 0x00b6_2994, &[0x025d_1624]),
        (
            Param::Rect,
            1,
            0x00b6_29a4,
            &[0x025d_161c, 0x025d_161a, 0x025d_1618, LOCAL],
        ),
        (Param::Color, 1, 0x00b6_29b4, &[0x025d_1614]),
        (Param::Color, 1, 0x00b6_29c4, &[0x025d_1610]),
        (Param::Point, 1, 0x00b6_29d4, &[0x025d_160c, 0x025d_160a]),
        (
            Param::Rect,
            1,
            0x00b6_29e0,
            &[0x025d_1608, 0x025d_1606, 0x025d_1604, LOCAL],
        ),
        (Param::Color, 1, 0x00b6_29f4, &[0x025d_1600]),
        (Param::Color, 1, 0x00b6_2a08, &[0x025d_15fc]),
        (
            Param::Rect,
            1,
            0x00b6_2a1c,
            &[0x025d_15f8, 0x025d_15f6, 0x025d_15f4, LOCAL],
        ),
        (Param::Color, 1, 0x00b6_2a2c, &[0x025d_15f0]),
        (Param::Color, 1, 0x00b6_2a3c, &[0x025d_15ec]),
        (Param::Point, 1, 0x00b6_2a4c, &[0x025d_15ea, 0x025d_15e8]),
        (Param::Point, 1, 0x00b6_2a5c, &[0x025d_15e6, 0x025d_15e4]),
        (Param::Color, 1, 0x00b6_2a6c, &[0x025d_15e0]),
        (Param::Color, 1, 0x00b6_2a6c, &[0x025d_15dc]),
        (Param::Color, 1, 0x00b6_2a78, &[0x025d_15d8]),
        (Param::Color, 1, 0x00b6_2a88, &[0x025d_15d4]),
        (Param::Color, 1, 0x00b6_2a98, &[0x025d_15d0]),
        (Param::Color, 1, 0x00b6_2a98, &[0x025d_15cc]),
        (Param::Color, 1, 0x00b6_2aa4, &[0x025d_15c8]),
        (Param::Color, 1, 0x00b6_2ab0, &[0x025d_15c4]),
        (Param::Color, 1, 0x00b6_2abc, &[0x025d_15b8]),
        (Param::Color, 1, 0x00b6_2ac8, &[0x025d_15b4]),
        (Param::Point, 1, 0x00b6_2ad4, &[0x025d_15b0, 0x025d_15ae]),
        (
            Param::Rect,
            1,
            0x00b6_2ae0,
            &[0x025d_0da2, 0x025d_0da0, 0x025d_0b1a, 0x025d_0b18],
        ),
        (Param::Color, 1, 0x00b6_2af0, &[0x025d_0b14]),
        (Param::Color, 1, 0x00b6_2b00, &[0x025d_0b10]),
        (
            Param::Rect,
            1,
            0x00b6_2b10,
            &[0x025d_0b0e, 0x025d_0b0c, 0x025d_0b0a, 0x025d_0b08],
        ),
        (Param::Color, 1, 0x00b6_2b1c, &[0x025d_0b04]),
        (Param::Color, 1, 0x00b6_2b28, &[0x025d_0b00]),
        (Param::Point, 1, 0x00b6_2b34, &[0x025d_0afe, 0x025d_0afc]),
        (Param::Point, 2, 0x00b6_2b40, &[0x025d_0afa, 0x025d_0af8]),
        (Param::Color, 2, 0x00b6_2b48, &[0x025d_0af4]),
        (Param::Color, 2, 0x00b6_2b50, &[0x025d_0af0]),
        (Param::Point, 2, 0x00b6_2b58, &[0x025d_0aee, 0x025d_0aec]),
        (Param::Num, 2, 0x00b6_2b60, &[0x025d_0aea]),
        (Param::Num, 2, 0x00b6_2b68, &[0x025d_0ae8]),
        (Param::Color, 2, 0x00b6_2b70, &[0x025d_0ae4]),
        (Param::Color, 2, 0x00b6_2b78, &[0x025d_0ae0]),
        (Param::Color, 2, 0x00b6_2b80, &[0x025d_0adc]),
        (Param::Color, 2, 0x00b6_2b88, &[0x025d_0ad8]),
        (Param::Point, 2, 0x00b6_2b90, &[0x025d_0ad6, 0x025d_0ad4]),
        (
            Param::Rect,
            2,
            0x00b6_2b98,
            &[0x025d_0ad2, 0x025d_0ad0, 0x025d_0ace, 0x025d_0acc],
        ),
        (
            Param::Rgba,
            2,
            0x00b6_2ba0,
            &[0x025d_0acb, 0x025d_0aca, 0x025d_0ac9, 0x025d_0ac8],
        ),
        (Param::Point, 2, 0x00b6_2ba8, &[0x025d_0ac6, 0x025d_0ac4]),
        (Param::Point, 2, 0x00b6_2bb0, &[0x025d_0ac2, 0x025d_0ac0]),
        (Param::Point, 2, 0x00b6_2bb8, &[0x025d_0abe, 0x025d_0abc]),
        (Param::Point, 2, 0x00b6_2bc0, &[0x025d_0aba, 0x025d_0ab8]),
        (
            Param::Rect,
            2,
            0x00b6_2bc8,
            &[0x025d_0ab6, 0x025d_0ab4, 0x025d_0ab2, 0x025d_0ab0],
        ),
        (Param::Color, 2, 0x00b6_2bd0, &[0x025d_0aac]),
        (
            Param::Rect,
            2,
            0x00b6_2bd8,
            &[0x025d_0aa8, 0x025d_0aa6, 0x025d_0aa4, 0x025d_0aa2],
        ),
        (
            Param::Rect,
            2,
            0x00b6_2be0,
            &[0x025d_0aa0, 0x025d_0a9e, 0x025d_0a9c, 0x025d_0a9a],
        ),
        (Param::Num, 2, 0x00b6_2be8, &[0x025d_0a98]),
        (Param::Point, 2, 0x00b6_2bf0, &[0x025d_0a96, 0x025d_0a94]),
        (Param::Point, 2, 0x00b6_2bf8, &[0x025d_0a92, 0x025d_0a90]),
        (Param::Point, 2, 0x00b6_2c00, &[0x025d_0a8e, 0x025d_0a8c]),
        (Param::Point, 2, 0x00b6_2c08, &[0x025d_0a8a, 0x025d_0a88]),
        (Param::Num, 2, 0x00b6_2c10, &[0x025d_0a80]),
    ];

    /// `TOSeqInitParam()` (`0x008ae6f0`): the menu's layout parameters
    /// read from scenes 1 and 2 into their globals ([`INIT_PARAMS`]), with
    /// `TkMenuGetParamHandle`/`TkMenuGetParamOffset` passed in; `scratch`
    /// takes the outputs the original discards ([`LOCAL`]).
    pub fn init_param<M: Mem>(
        mem: &mut M,
        scratch: u32,
        handle: impl Fn(&M, i32, u32) -> u32,
        offset: impl Fn(&M, i32, i32, i32) -> u32,
    ) {
        for &(p, i, name, outs) in &INIT_PARAMS {
            let name = mem.rebase(name);
            let o: Vec<u32> = outs
                .iter()
                .map(|&a| if a == LOCAL { scratch } else { mem.rebase(a) })
                .collect();
            match p {
                Param::Color => get_color(mem, i, name, o[0], &handle, &offset),
                Param::Num => get_num(mem, i, name, o[0], &handle, &offset),
                Param::Point => get_point(mem, i, name, [o[0], o[1]], &handle, &offset),
                Param::Rect => get_rect(mem, i, name, [o[0], o[1], o[2], o[3]], &handle, &offset),
                Param::Rgba => {
                    get_color_rgba(
                        mem,
                        i,
                        name,
                        [o[0], o[1], o[2], o[3]],
                        |m, i, n| handle(m, i, n),
                        |m, i, h, n| offset(m, i, h.cast_signed(), n.cast_signed()),
                    );
                }
            }
        }
    }

    /// `TOSeqStopExecScene(i)` (`0x008af380`): every slot of scene `i`
    /// that is running (state 2 or 3) and not kept (`+2` is not 1) stops:
    /// state 4 and the name's first byte (`+8`) cleared. Returns `i <<
    /// 8` (the original's `eax`).
    pub fn stop_exec_scene(mem: &mut impl Mem, i: i32) -> u32 {
        for j in 0..8 {
            let s = slot(mem, i, j);
            let kept = mem.read::<u16>(s.wrapping_add(2)) == 1;
            let state: u16 = mem.read(s);
            if !kept && (state == 2 || state == 3) {
                mem.write(s, 4u16);
                mem.write(s.wrapping_add(8), 0u8);
            }
        }
        i.cast_unsigned().wrapping_shl(8)
    }
}
