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

    /// `TOSeqDrawLayer(i, a)` (`0x008ae340`): [`draw`]`(i, a, 0)`; its
    /// result.
    pub fn draw_layer<M: Mem>(
        mem: &mut M,
        i: i32,
        a: u32,
        call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        draw(mem, i, a, 0, call)
    }

    /// `TkMenuExecScene(i, handle, out)` (`ffx.sym` address).
    pub const TK_MENU_EXEC_SCENE: u32 = 0x00a7_b460;

    /// `TOMenuExecLayerHandle(i, layer, handle, out)` (`0x00a7b220`, caller
    /// `TOSeqDraw`): `layer` clamped to 0..=15; when scene `i`'s data
    /// (`g_TkMenuSceneData[i]`) has a list for that layer (offset at
    /// `+0x90 + layer * 4`, from the data's start: a count, then that many
    /// handle words), `TkMenuExecScene(i, handle, out)` runs once for each
    /// entry equal to `handle`. `EAX` as the original leaves it: the
    /// clamped layer (no data), 0 (no list), the list's offset (count 0),
    /// else the last entry read, or `TkMenuExecScene`'s result when that
    /// entry matched. `call` takes an `ffx.sym` address.
    pub fn exec_layer_handle<M: Mem>(
        mem: &mut M,
        [i, layer, handle, out]: [u32; 4],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let layer = layer.cast_signed().clamp(0, 15).cast_unsigned();
        let table = mem.rebase(0x022f_b4dc);
        let data = mem.read::<u32>(table.wrapping_add(i.wrapping_mul(4)));
        if data == 0 {
            return layer;
        }
        let off = mem.read::<u32>(data.wrapping_add(0x90).wrapping_add(layer * 4));
        if off == 0 {
            return 0;
        }
        let list = off.wrapping_add(data);
        let count = mem.read::<u32>(list);
        let mut eax = off;
        for k in 1..=count {
            eax = mem.read::<u32>(list.wrapping_add(k.wrapping_mul(4)));
            if eax == handle {
                eax = call(mem, TK_MENU_EXEC_SCENE, &[i, handle, out]);
            }
        }
        eax
    }
    /// `TkMenuStartScene(i, handle)`.
    pub const TK_MENU_START_SCENE: u32 = 0x00a7_c880;
    /// `TkMenuIsTermScene(i, handle)`.
    pub const TK_MENU_IS_TERM_SCENE: u32 = 0x00a7_bb90;
    /// `TkMenuStopScene(i, handle)`.
    pub const TK_MENU_STOP_SCENE: u32 = 0x00a7_c8a0;

    /// `TOSeqCtrl(i)` (`0x008ae0f0`): steps scene `i`'s eight slots
    /// ([`SLOTS`]; the state read as `i16`, the handle `+0x1c` read again
    /// for each call). State 1 starts the scene (`TkMenuStartScene`) and
    /// goes on as state 2; state 2 becomes 3 when `+4` is 1, else 4 once
    /// `TkMenuIsTermScene`; state 4 stops it (`TkMenuStopScene`) and frees
    /// the slot (0). Then the slots in use (state not 0) move to the front
    /// in order and the rest are zeroed. `call` takes an `ffx.sym`
    /// address.
    pub fn seq_ctrl<M: Mem>(mem: &mut M, i: i32, mut call: impl FnMut(&mut M, u32, &[u32]) -> u32) {
        let first = slot(mem, i, 0);
        let scene = i;
        let i = i.cast_unsigned();
        for k in 0..8 {
            let slot = slot(mem, scene, k);
            let handle = |mem: &M| mem.read::<u32>(slot.wrapping_add(0x1c));
            let mut state: i16 = mem.read(slot);
            if state == 1 {
                let s = handle(mem);
                call(mem, TK_MENU_START_SCENE, &[i, s]);
                mem.write(slot, 2i16);
                state = 2;
            }
            if state == 2 {
                if mem.read::<u16>(slot.wrapping_add(4)) == 1 {
                    mem.write(slot, 3i16);
                } else {
                    let s = handle(mem);
                    if call(mem, TK_MENU_IS_TERM_SCENE, &[i, s]) != 0 {
                        mem.write(slot, 4i16);
                    }
                }
            } else if state == 4 {
                let s = handle(mem);
                call(mem, TK_MENU_STOP_SCENE, &[i, s]);
                mem.write(slot, 0i16);
            }
        }
        let mut packed = [[0u8; 0x20]; 8];
        let mut used = 0;
        for k in 0..8 {
            let slot = slot(mem, scene, k);
            if mem.read::<u16>(slot) != 0 {
                packed[used] = mem.read(slot);
                used += 1;
            }
        }
        mem.write(first, packed);
    }

    /// `MenuScene_KeyPick_00a7b290(keys, track, frame, frames, out)`
    /// (`0x00a7b290`): a scene track's key pair (`u16 keys[track+8]` the
    /// first, `keys[track+0xa]` the second, `A`/`B`) handed to
    /// `0x00a7bbc0(key, from, to, t, out)` (its `eax`): `(*B, B, B, 0)`
    /// while `i16 track+6` is negative or `frame == frames`, `(*A, A, A, 0)`
    /// at `frame` 0; otherwise `t = f32(MenuScene_KeyEase(*B, frame,
    /// frames))` (`fcall`, ST0) gives `(*A, B, A, t)` when the keys' low 11
    /// bits match, else `(*B, B, B, 0)` from `t >= 0.5` on and `(*A, A, A,
    /// 0)` below it or for NaN. `*B` is the key read on entry, `*A` the
    /// one read after the ease (as the original keeps them).
    pub fn menu_scene_key_pick<M: Mem>(
        mem: &mut M,
        [keys, track, frame, frames, out]: [u32; 5],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
        mut fcall: impl FnMut(&mut M, u32, &[u32]) -> f64,
    ) -> u32 {
        let a = keys.wrapping_add(2 * u32::from(mem.read::<u16>(track.wrapping_add(8))));
        let b = keys.wrapping_add(2 * u32::from(mem.read::<u16>(track.wrapping_add(0xa))));
        let key_a = u32::from(mem.read::<u16>(a));
        let key_b = u32::from(mem.read::<u16>(b));
        let pick = |mem: &mut M, call: &mut dyn FnMut(&mut M, u32, &[u32]) -> u32, k, p, q, t| {
            call(mem, 0x00a7_bbc0, &[k, p, q, t, out])
        };
        if mem.read::<i16>(track.wrapping_add(6)) < 0 {
            return pick(mem, &mut call, key_b, b, b, 0);
        }
        if frame == 0 {
            return pick(mem, &mut call, key_a, a, a, 0);
        }
        if frame == frames {
            return pick(mem, &mut call, key_b, b, b, 0);
        }
        let ease = u32::from(mem.read::<u16>(b));
        let t = ffx_bridge::x87::store(fcall(mem, 0x00a7_c6f0, &[ease, frame, frames]));
        let key_a = u32::from(mem.read::<u16>(a));
        if (key_b ^ key_a).trailing_zeros() >= 11 {
            return pick(mem, &mut call, key_a, b, a, t);
        }
        if f32::from_bits(t) >= 0.5 {
            pick(mem, &mut call, key_b, b, b, 0)
        } else {
            pick(mem, &mut call, key_a, a, a, 0)
        }
    }

    /// The key-script operand lengths in words, by opcode (`0x00c89c40`,
    /// eleven bytes: 2 2 2 2 10 10 12 12 2 2 16).
    pub const KEY_OP_WORDS: u32 = 0x00c8_9c40;

    /// `MenuScene_TrackStep_00a7b510(base, list, track, table, out)`
    /// (`0x00a7b510`): one frame of a scene track's key script. The script
    /// is `base + table[track.key]` (`track.key` = `u32 track+0` low 30
    /// bits), the program counter `u16 track+8`, in 16-bit words whose low
    /// 11 bits are the opcode. While the wait `u16 track+4` is not zero it
    /// counts down and `MenuScene_KeyPick` runs at once (returns 1).
    /// Otherwise the opcodes run until one waits:
    /// - 0: end. Wait 0, frames -1, `track+0` = the key alone (flags
    ///   cleared); returns 0.
    /// - 1: step (`track+0xa` = the old counter).
    /// - 2: wait for list entry `i16 track+0xc` (found by the operand
    ///   against `list[1]`'s low 30 bits, the first entry's, for every
    ///   entry, as the original compares) while its `+4` bit 31 is set;
    ///   then entry -1, frames 0, step.
    /// - 3: hold: wait = operand (minus 1 and stop after this frame when
    ///   not 0), frames -1, step.
    /// - 4..=7, 10: key: wait = frames = operand, step; stop when not 0
    ///   (wait minus 1).
    /// - 8: loop back to `u16 track+0xe` while the count `i16 track+0x10`
    ///   (bit 11 of the word: 1; else the operand when it is not above 0,
    ///   else minus 1) stays above 0, else count 0 and step.
    /// - 9: loop mark: step, `track+0xe` = the new counter.
    ///
    /// Opcodes above 10 change nothing (with no hold pending they spin, as
    /// in the original). Steps use [`KEY_OP_WORDS`]. Each stop ends in
    /// `MenuScene_KeyPick(script, track, u16 wait, i16 frames, out)`;
    /// returns 1 except after opcode 0.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::too_many_lines,
        clippy::similar_names
    )] // 16-bit counters as the original stores them; one interpreter
    pub fn menu_scene_track_step<M: Mem>(
        mem: &mut M,
        [base, list, track, table, out]: [u32; 5],
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        const KEY_PICK: u32 = 0x00a7_b290;
        let key = mem.read::<u32>(track) & 0x3fff_ffff;
        let slot = table.wrapping_add(key.wrapping_mul(4));
        let script = mem.read::<u32>(slot).wrapping_add(base);
        let mut word = script.wrapping_add(2 * u32::from(mem.read::<u16>(track.wrapping_add(8))));
        let wait = mem.read::<u16>(track.wrapping_add(4));
        let frames = |mem: &M| i32::from(mem.read::<i16>(track.wrapping_add(6)));
        if wait != 0 {
            let wait = wait.wrapping_sub(1);
            let f = frames(mem).cast_unsigned();
            mem.write(track.wrapping_add(4), wait);
            call(mem, KEY_PICK, &[script, track, u32::from(wait), f, out]);
            return 1;
        }
        let ops = mem.rebase(KEY_OP_WORDS);
        let len = |mem: &M, op: u32| u32::from(mem.read::<u8>(ops.wrapping_add(op)));
        let set16 = |mem: &mut M, at: u32, v: u16| mem.write(track.wrapping_add(at), v);
        let get16 = |mem: &M, at: u32| mem.read::<u16>(track.wrapping_add(at));
        let mut go_on = true;
        let mut result = 1;
        loop {
            let op_word = u32::from(mem.read::<u16>(word));
            let op = op_word & 0x7ff;
            // step: pointer and counter by the opcode's length
            let step = |mem: &mut M, word: &mut u32, n: u32| {
                *word = word.wrapping_add(2 * n);
                let pc = get16(mem, 8);
                mem.write(track.wrapping_add(8), pc.wrapping_add(n as u16));
            };
            match op {
                0 => {
                    set16(mem, 4, 0);
                    set16(mem, 6, 0xffff);
                    mem.write(track, key);
                    result = 0;
                    break;
                }
                1 => {
                    let n = len(mem, 1);
                    word = word.wrapping_add(2 * n);
                    let pc = get16(mem, 8);
                    set16(mem, 0xa, pc);
                    set16(mem, 8, pc.wrapping_add(n as u16));
                }
                2 => {
                    set16(mem, 4, 0);
                    set16(mem, 6, 0xffff);
                    if mem.read::<i16>(track.wrapping_add(0xc)) < 0 {
                        let want = i32::from(mem.read::<i16>(word.wrapping_add(2)));
                        let count = mem.read::<i32>(list);
                        let first = mem.read::<u32>(list.wrapping_add(4)) & 0x3fff_ffff;
                        let mut i = 0i32;
                        while i < count {
                            if want.cast_unsigned() == first {
                                set16(mem, 0xc, i as u16);
                                break;
                            }
                            i += 1;
                        }
                    }
                    let entry = mem.read::<i16>(track.wrapping_add(0xc));
                    if entry >= 0 {
                        let at = list
                            .wrapping_add(20u32.wrapping_mul(i32::from(entry).cast_unsigned()))
                            .wrapping_add(4);
                        if mem.read::<u32>(at) & 0x8000_0000 != 0 {
                            break;
                        }
                    }
                    set16(mem, 0xc, 0xffff);
                    set16(mem, 6, 0);
                    let n = len(mem, 2);
                    step(mem, &mut word, n);
                }
                3 => {
                    let hold = mem.read::<u16>(word.wrapping_add(2));
                    set16(mem, 4, hold);
                    set16(mem, 6, 0xffff);
                    if hold != 0 {
                        go_on = false;
                        set16(mem, 4, hold - 1);
                    }
                    let n = len(mem, 3);
                    word = word.wrapping_add(2 * n);
                    let pc = get16(mem, 8);
                    set16(mem, 0xa, pc);
                    set16(mem, 8, pc.wrapping_add(n as u16));
                }
                4..=7 | 10 => {
                    let n = len(mem, op);
                    let wait = mem.read::<u16>(word.wrapping_add(2));
                    word = word.wrapping_add(2 * n);
                    let pc = get16(mem, 8);
                    set16(mem, 0xa, pc);
                    set16(mem, 8, pc.wrapping_add(n as u16));
                    set16(mem, 4, wait);
                    set16(mem, 6, wait);
                    if wait != 0 {
                        set16(mem, 4, wait - 1);
                        break;
                    }
                }
                8 => {
                    let count = if op_word & 0x800 != 0 {
                        1
                    } else {
                        let c = mem.read::<i16>(track.wrapping_add(0x10));
                        if c > 0 {
                            c - 1
                        } else {
                            mem.read::<i16>(word.wrapping_add(2))
                        }
                    };
                    mem.write(track.wrapping_add(0x10), count);
                    if count > 0 {
                        let mark = get16(mem, 0xe);
                        set16(mem, 8, mark);
                        word = base
                            .wrapping_add(2 * u32::from(mark))
                            .wrapping_add(mem.read::<u32>(slot));
                    } else {
                        let n = len(mem, 8);
                        step(mem, &mut word, n);
                        set16(mem, 0x10, 0);
                    }
                }
                9 => {
                    let n = len(mem, 9);
                    step(mem, &mut word, n);
                    let pc = get16(mem, 8);
                    set16(mem, 0xe, pc);
                }
                _ => {}
            }
            if !go_on {
                break;
            }
        }
        let wait = u32::from(get16(mem, 4));
        let f = frames(mem).cast_unsigned();
        call(mem, KEY_PICK, &[script, track, wait, f, out]);
        result
    }
}
