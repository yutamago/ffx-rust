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
}
