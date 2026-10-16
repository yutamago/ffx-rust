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

/// Set with bit 8 of [`RENDER_STATUS`] by `yiSetRenderParam` (script
/// native `0x0091c580`).
pub const RENDER_PARAM: Global<u32> = Global::new(0x018d_ed2c);
/// Three bytes `yiSetColor3` stores (script native `0x00859000`).
pub const COLOR3: Global<[u8; 3]> = Global::new(0x0193_4ea4);

/// A matrix the camera set-up multiplies in before the camera
/// (`0x0090a140`: `cdc_Mul4x4(out, camera, it)`; `yiGetViewAdjust`,
/// `yiSetViewAdjust`).
pub const VIEW_ADJUST: Global<[u32; 16]> = Global::new(0x018d_ee38);
/// The colour offset, four floats `channel - 128` (`yiSetColorOffset`;
/// saved and restored with the render state).
pub const COLOR_OFFSET: Global<[u32; 4]> = Global::new(0x018d_f01c);
/// The image VRAM allocator's next free address (`yiVramAlloc`, bytes,
/// 64-byte steps).
pub const VRAM_NEXT: Global<u32> = Global::new(0x025d_17bc);
/// Its end (`yiVramSetLimit`).
pub const VRAM_LIMIT: Global<u32> = Global::new(0x025d_17ac);

/// `yiVramSetLimit(pages)`: the allocator's end, `(0x38 - pages) * 0x4000`.
#[must_use]
pub fn vram_limit(pages: u32) -> u32 {
    0x38u32.wrapping_sub(pages).wrapping_shl(14)
}

/// `yiVramAlloc(bytes)`: the allocator's next address after `bytes`
/// rounded up to 64 (C division of a negative rounds toward zero).
#[must_use]
pub fn vram_end(next: u32, bytes: i32) -> u32 {
    let n = bytes.wrapping_add(0x3f);
    let rounded = n.wrapping_add((n >> 31) & 0x3f) & !0x3f;
    rounded.cast_unsigned().wrapping_add(next)
}

/// `yiPow2Ceil(n)`: the smallest `1 << k`, `k < 16`, not below `n`; 0 when
/// none is.
#[must_use]
pub fn pow2_ceil(n: i32) -> i32 {
    (0..16).map(|k| 1i32 << k).find(|&p| n <= p).unwrap_or(0)
}

/// `yiHighestBit(bits)`: the index of the highest of the low 16 bits set,
/// -1 for none.
#[must_use]
pub fn highest_bit(bits: u32) -> i32 {
    (0..16).rev().find(|&k| bits & (1 << k) != 0).unwrap_or(-1)
}

/// `yiLog2Ceil16(bits)`: for the low 16 bits, the index of the only bit
/// set, or the highest one's plus 1 when several are; -1 for none.
#[must_use]
pub fn log2_ceil16(bits: u32) -> i32 {
    let mut r = -1;
    for k in 0..16 {
        if bits & (1 << k) != 0 {
            r = if r == -1 { k } else { k + 1 };
        }
    }
    r
}

/// `yiIsPow2(bits)`: whether at most one of the low 16 bits is set.
#[must_use]
pub fn is_pow2(bits: u32) -> bool {
    (bits & 0xffff).count_ones() <= 1
}

/// The quadword of a GS register write in A+D mode: the data, then the
/// register (`yiPkAddGsAD` and its copies).
#[must_use]
pub const fn gs_ad(reg: u32, data: u32) -> [u32; 4] {
    [data, 0, reg, 0]
}

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
    /// `+0x90`: two RGBA colours, a byte per channel (`yiGetColorDObj`).
    pub const COLORS: u32 = 0x90;
}

/// Offsets in the renderer object (`yiRender_*`).
pub mod render {
    /// Words `yiRender_resettex` clears.
    pub const TEX_CLEARED: [u32; 3] = [0x9c198, 0xbe0ac, 0xbe0b0];
    /// The word it sets to -1.
    pub const TEX_NONE: u32 = 0x67f34;
    /// The DMA packet's write position (`yiRender_addRef`).
    pub const PACKET: u32 = 0xba3f0;
    /// Tags added to it.
    pub const PACKET_COUNT: u32 = 0xba3f8;
    /// The word `yiScreff_init` clears.
    pub const SCREFF: u32 = 0xbe078;
}

/// The renderer state where the game keeps it, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::Mem;

    use super::{
        COLOR_OFFSET, COLOR3, DOBJ_COUNT, DOBJS, DRAW_MASK, MAP_DATA, RENDER_PARAM, RENDER_STATUS,
        SCALE_FACTOR, SG_FRAME, SG_VCOUNT, VIEW_ADJUST, VRAM_LIMIT, VRAM_NEXT, dobj, render,
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

    /// `yiSetRenderParam(v)` (`0x00908580`): render status bit 8 and the
    /// value.
    pub fn yi_set_render_param(mem: &mut impl Mem, v: u32) {
        let s = RENDER_STATUS.get(mem);
        RENDER_STATUS.set(mem, s | 0x100);
        RENDER_PARAM.set(mem, v);
    }

    /// `yiCheckDObj(index)` (`0x009086b0`): 0 for an object in use, else -1.
    pub fn yi_check_dobj(mem: &impl Mem, index: u32) -> i32 {
        if index < DOBJ_COUNT.get(mem) { 0 } else { -1 }
    }

    /// `yiSetColor3(r, g, b)` (`0x0090a400`): the low bytes; returns 0.
    pub fn yi_set_color3(mem: &mut impl Mem, r: u8, g: u8, b: u8) -> u32 {
        COLOR3.set(mem, [r, g, b]);
        0
    }

    /// `yiSetMatrixDObj(index, m)` (`0x0090a520`): 0, or -1 when there is
    /// no such object.
    pub fn yi_set_matrix_dobj(mem: &mut impl Mem, index: u32, m: u32) -> i32 {
        if index >= DOBJ_COUNT.get(mem) {
            return -1;
        }
        put_matrix(mem, index, m);
        0
    }

    /// `yiSetMatrixGroup(group, m)` (`0x0090a560`): the first object in
    /// `group`; 0, or -1 when none is.
    pub fn yi_set_matrix_group(mem: &mut impl Mem, group: u32, m: u32) -> i32 {
        match find_group(mem, group) {
            Some(i) => {
                put_matrix(mem, i, m);
                0
            }
            None => -1,
        }
    }

    fn find_group(mem: &impl Mem, group: u32) -> Option<u32> {
        let base = DOBJS.ptr(mem).addr;
        (0..DOBJ_COUNT.get(mem))
            .find(|&i| u32::from(mem.read::<u16>(base + i * dobj::SIZE + dobj::GROUP)) == group)
    }

    fn put_matrix(mem: &mut impl Mem, index: u32, m: u32) {
        let at = DOBJS.ptr(mem).addr + index.wrapping_mul(dobj::SIZE) + dobj::MATRIX;
        let v = mem.read::<[u32; 16]>(m);
        mem.write(at, v);
    }

    /// `yiGetDObjGroup(index)` (`0x00908c50`): its group, -1 when there is
    /// no such object.
    pub fn yi_get_dobj_group(mem: &impl Mem, index: u32) -> u32 {
        if index >= DOBJ_COUNT.get(mem) {
            return u32::MAX;
        }
        let at = DOBJS.ptr(mem).addr + index.wrapping_mul(dobj::SIZE) + dobj::GROUP;
        u32::from(mem.read::<u16>(at))
    }

    /// `yiGetColorDObj(index, a, b)` (`0x00908e90`): its two colours, a
    /// byte per channel, widened to four words each.
    pub fn yi_get_color_dobj(mem: &mut impl Mem, index: u32, a: u32, b: u32) -> i32 {
        if index >= DOBJ_COUNT.get(mem) {
            return -1;
        }
        let at = DOBJS.ptr(mem).addr + index.wrapping_mul(dobj::SIZE) + dobj::COLORS;
        let c = mem.read::<[u8; 8]>(at);
        mem.write(a, [c[0], c[1], c[2], c[3]].map(u32::from));
        mem.write(b, [c[4], c[5], c[6], c[7]].map(u32::from));
        0
    }

    /// `yiSetColorDObj(index, a, b)` (`0x0090a5c0`): the low bytes of the
    /// four words of each.
    pub fn yi_set_color_dobj(mem: &mut impl Mem, index: u32, a: u32, b: u32) -> i32 {
        if index >= DOBJ_COUNT.get(mem) {
            return -1;
        }
        let at = DOBJS.ptr(mem).addr + index.wrapping_mul(dobj::SIZE) + dobj::COLORS;
        for (k, src) in [a, b].into_iter().enumerate() {
            for i in 0..4u32 {
                let v = mem.read::<u8>(src + 4 * i);
                mem.write(at + 4 * u32::try_from(k).unwrap_or(0) + i, v);
            }
        }
        0
    }

    /// `yiGetViewAdjust(out)` (`0x00908f20`).
    pub fn yi_get_view_adjust(mem: &mut impl Mem, out: u32) {
        let m = VIEW_ADJUST.get(mem);
        mem.write(out, m);
    }

    /// `yiSetViewAdjust(m)` (`0x0090a730`).
    pub fn yi_set_view_adjust(mem: &mut impl Mem, m: u32) {
        let v = mem.read::<[u32; 16]>(m);
        VIEW_ADJUST.set(mem, v);
    }

    /// `yiSetColorOffset(rgba)` (`0x00909ff0`): four ints, each less 128,
    /// as floats.
    pub fn yi_set_color_offset(mem: &mut impl Mem, rgba: u32) {
        for i in 0..4 {
            let c = mem.read::<i32>(rgba + 4 * i).wrapping_add(-0x80);
            let at = COLOR_OFFSET.ptr(mem).addr + 4 * i;
            #[allow(clippy::cast_precision_loss)] // fild then fstp m32
            mem.write(at, (c as f32).to_bits());
        }
    }

    /// `yiPkAddGsAD(q, reg, data)` (`0x0090ae20`, with identical copies at
    /// `0x0090bc40` and `0x00910040`).
    pub fn yi_pk_add_gs_ad(mem: &mut impl Mem, q: u32, reg: u32, data: u32) {
        mem.write(q, super::gs_ad(reg, data));
    }

    /// Appends a DMA "ref" tag (ID 3: `qwc` quadwords from `addr`) to the
    /// renderer's packet and counts it.
    fn add_ref(mem: &mut impl Mem, r: u32, addr: u32, qwc: u16) {
        let at = mem.read::<u32>(r.wrapping_add(render::PACKET));
        mem.write(at, [u32::from(qwc) | 0x3000_0000, addr, 0, 0]);
        mem.write(r.wrapping_add(render::PACKET), at.wrapping_add(0x10));
        let n = mem.read::<u32>(r.wrapping_add(render::PACKET_COUNT));
        mem.write(r.wrapping_add(render::PACKET_COUNT), n.wrapping_add(1));
    }

    /// `yiRender_addRef(render, addr, qwc)` (`0x0090c1f0`), `qwc` the low
    /// 16 bits.
    pub fn yi_render_add_ref(mem: &mut impl Mem, r: u32, addr: u32, qwc: u16) {
        add_ref(mem, r, addr, qwc);
    }

    /// `yiRender_addRefData(render, block)` (`0x0090c230`): a block whose
    /// first `u16` is its quadword count and whose data starts at `+0x10`.
    pub fn yi_render_add_ref_data(mem: &mut impl Mem, r: u32, block: u32) {
        let qwc = mem.read::<u16>(block);
        add_ref(mem, r, block.wrapping_add(0x10), qwc);
    }

    /// Copies 16 words one at a time, as `rep movsd`.
    fn copy16(mem: &mut impl Mem, dst: u32, src: u32) {
        for i in 0..16 {
            let v = mem.read::<u32>(src + 4 * i);
            mem.write(dst + 4 * i, v);
        }
    }

    /// `yiPkUnpackMatrix(m, out, addr)` (`0x0090cd10`): a VIF `UNPACK V4-32`
    /// of 4 quadwords to VU memory `addr` (`0x6c04xxxx` in the fourth word of
    /// a quadword of `NOP`s), then the matrix.
    pub fn yi_pk_unpack_matrix(mem: &mut impl Mem, m: u32, out: u32, addr: u32) {
        mem.write(out, [0u32, 0, 0, addr | 0x6c04_0000]);
        copy16(mem, out + 0x10, m);
    }

    /// `yiPkUnpackMatrixDma(m, out, addr)` (`0x0090ccb0`): the same behind a
    /// DMA tag of ID 2 (`next`) and 5 quadwords, and VIF `FLUSH`,
    /// `STCYCL 1,1`.
    pub fn yi_pk_unpack_matrix_dma(mem: &mut impl Mem, m: u32, out: u32, addr: u32) {
        mem.write(
            out,
            [
                0x2000_0005u32,
                0,
                0,
                0,
                0x1100_0000,
                0,
                0x0100_0101,
                addr | 0x6c04_0000,
            ],
        );
        copy16(mem, out + 0x20, m);
    }

    /// `yiVramAlloc(bytes)` (`0x00918130`): the 64-byte block address, or
    /// -1 when it does not fit.
    pub fn yi_vram_alloc(mem: &mut impl Mem, bytes: i32) -> u32 {
        let next = VRAM_NEXT.get(mem);
        let end = super::vram_end(next, bytes);
        if end > VRAM_LIMIT.get(mem) {
            return u32::MAX;
        }
        VRAM_NEXT.set(mem, end);
        next >> 6
    }

    /// `yiVramSetLimit(pages)` (`0x00919630`): returns 0.
    pub fn yi_vram_set_limit(mem: &mut impl Mem, pages: u32) -> u32 {
        VRAM_LIMIT.set(mem, super::vram_limit(pages));
        0
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
