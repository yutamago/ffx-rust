//! `yi`: the field map renderer's state and its display objects
//! (`docs/formats/field.md`).

use ffx_bridge::Global;

/// What the field renderer does (`yiRenderEnable`/`yiRenderDisable`); bits
/// 0 and 4 are set once a map is read (`yiMapReadMaster`), bit 0 gates the
/// field particles (`yiCallFieldParticle`).
pub const RENDER_STATUS: Global<u32> = Global::new(0x018d_ed08);
/// The draw mask (`yiSetDrawMask`): -9 after a map is read, -1 when the
/// battle map circle is on (`yiMapReadMaster`); script native 0x8000 sets
/// its bits.
pub const DRAW_MASK: Global<u32> = Global::new(0x018d_ed0c);
/// The field scale factor, x, y, z (`yiSetScaleFactor`).
pub const SCALE_FACTOR: Global<[u32; 3]> = Global::new(0x018d_eff8);
/// The display objects in use (`yiGetMatrixDObj` bound).
pub const DOBJ_COUNT: Global<u32> = Global::new(0x018d_f588);
/// The display objects, [`dobj::SIZE`] bytes each.
pub const DOBJS: Global<u8> = Global::new(0x018d_f59c);
/// The map data read (`yiMapReadMaster`); `+4` is its status
/// (`yiGetMapStatus`).
pub const MAP_DATA: Global<u32> = Global::new(0x0191_38a8);
/// The vertical blank counter, +2 a frame (main loop, `0x00821e80`).
pub const SG_VCOUNT: Global<u32> = Global::new(0x012f_b7f0);
/// The main loop's frame counter (`Sg_MainLoop`, which swaps the display
/// buffers with it).
pub const SG_FRAME: Global<u32> = Global::new(0x023c_bbf0);

/// Offsets in a display object.
pub mod dobj {
    /// Size (`yiGetMatrixDObj` stride).
    pub const SIZE: u32 = 0xb4;
    /// `+0x0`: its matrix (16 floats).
    pub const MATRIX: u32 = 0x0;
    /// `+0x40`: rotation (`0x00908190` adds to it and rebuilds the matrix).
    pub const ROTATION: u32 = 0x40;
    /// `+0x50`: translation.
    pub const TRANSLATION: u32 = 0x50;
    /// `+0x74`: the group (`u16`, `yiGetMatrixGroup`).
    pub const GROUP: u32 = 0x74;
    /// `+0x76`: the rotation order for `yiRotMatrixSeq` (`u16`).
    pub const ROT_ORDER: u32 = 0x76;
}

/// Offsets in the renderer object (`yiRender_*`).
pub mod render {
    /// Words `yiRender_resettex` clears.
    pub const TEX_CLEARED: [u32; 3] = [0x9c198, 0xbe0ac, 0xbe0b0];
    /// The word it sets to -1.
    pub const TEX_NONE: u32 = 0x67f34;
    /// The word `yiScreff_init` clears.
    pub const SCREFF: u32 = 0xbe078;
}

/// The renderer state where the game keeps it, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{
        DOBJ_COUNT, DOBJS, DRAW_MASK, MAP_DATA, RENDER_STATUS, SCALE_FACTOR, SG_FRAME, SG_VCOUNT,
        dobj, render,
    };
    use crate::x87;

    /// `yiGetRenderStatus()` (`0x00908f10`).
    pub fn yi_get_render_status(mem: &impl Mem) -> u32 {
        RENDER_STATUS.get(mem)
    }

    /// `yiRenderEnable(bits)` (`0x00909e20`).
    pub fn yi_render_enable(mem: &mut impl Mem, bits: u32) {
        let s = RENDER_STATUS.get(mem);
        RENDER_STATUS.set(mem, s | bits);
    }

    /// `yiRenderDisable(bits)` (`0x00909e10`).
    pub fn yi_render_disable(mem: &mut impl Mem, bits: u32) {
        let s = RENDER_STATUS.get(mem);
        RENDER_STATUS.set(mem, s & !bits);
    }

    /// `yiGetDrawMask()` (`0x00908960`).
    pub fn yi_get_draw_mask(mem: &impl Mem) -> u32 {
        DRAW_MASK.get(mem)
    }

    /// `yiSetDrawMask(mask)` (`0x0090a2b0`).
    pub fn yi_set_draw_mask(mem: &mut impl Mem, mask: u32) {
        DRAW_MASK.set(mem, mask);
    }

    /// `yiGetVCount()` (`0x00908f70`).
    pub fn yi_get_vcount(mem: &impl Mem) -> u32 {
        SG_VCOUNT.get(mem)
    }

    /// `yiGetFCount()` (`0x00908970`): the frame counter in fields (two a
    /// frame).
    pub fn yi_get_fcount(mem: &impl Mem) -> u32 {
        SG_FRAME.get(mem).wrapping_mul(2)
    }

    /// `yiGetMapStatus()` (`0x00908dd0`): the map data's status, -1 without
    /// map data.
    pub fn yi_get_map_status(mem: &impl Mem) -> u32 {
        match MAP_DATA.get(mem) {
            0 => u32::MAX,
            m => mem.read(m + 4),
        }
    }

    fn copy_matrix(mem: &mut impl Mem, index: u32, out: u32) {
        let at = DOBJS.ptr(mem).addr + index.wrapping_mul(dobj::SIZE) + dobj::MATRIX;
        let m = mem.read::<[u32; 16]>(at);
        mem.write(out, m);
    }

    /// `yiGetMatrixDObj(index, out)` (`0x00908df0`): copies the object's
    /// matrix; 0, or -1 when there is no such object.
    pub fn yi_get_matrix_dobj(mem: &mut impl Mem, index: u32, out: u32) -> i32 {
        if index >= DOBJ_COUNT.get(mem) {
            return -1;
        }
        copy_matrix(mem, index, out);
        0
    }

    /// `yiGetMatrixGroup(group, out)` (`0x00908e30`): copies the matrix of
    /// the first object in `group`; 0, or -1 when none is.
    pub fn yi_get_matrix_group(mem: &mut impl Mem, group: u32, out: u32) -> i32 {
        let base = DOBJS.ptr(mem).addr;
        for i in 0..DOBJ_COUNT.get(mem) {
            let g = mem.read::<u16>(base + i * dobj::SIZE + dobj::GROUP);
            if u32::from(g) == group {
                copy_matrix(mem, i, out);
                return 0;
            }
        }
        -1
    }

    /// `yiSetScaleFactor(v)` (`0x0090a950`).
    pub fn yi_set_scale_factor(mem: &mut impl Mem, v: u32) {
        let s = mem.read::<[u32; 3]>(v).map(x87::copy);
        SCALE_FACTOR.set(mem, s);
    }

    /// `yiRender_resettex(render)` (`0x0090cea0`).
    pub fn yi_render_resettex(mem: &mut impl Mem, r: u32) {
        for o in render::TEX_CLEARED {
            mem.write(r.wrapping_add(o), 0u32);
        }
        mem.write(r.wrapping_add(render::TEX_NONE), u32::MAX);
    }

    /// `yiScreff_init(render)` (`0x00913f30`).
    pub fn yi_screff_init(mem: &mut impl Mem, r: u32) {
        mem.write(r.wrapping_add(render::SCREFF), 0u32);
    }
}
