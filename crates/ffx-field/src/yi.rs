//! `yi`: the field map renderer's state and its display objects
//! (`docs/formats/field.md`).

use ffx_bridge::Global;

/// What the field renderer does (`yiRenderEnable`/`yiRenderDisable`); bits
/// 0 and 4 are set once a map is read (`yiMapReadMaster`), bit 0 gates the
/// field particles (`yiCallFieldParticle`).
pub const RENDER_STATUS: Global<u32> = Global::new(0x018d_ed08);
/// 1/32 and 32.0 (`f64`s in `.rdata`): `yiRender_setupFogParam`'s scale.
pub const FOG_EIGHTH: u32 = 0x00b5_01a0;
/// See [`FOG_EIGHTH`].
pub const FOG_SCALE32: u32 = 0x00b4_12f8;
/// 255.0 as an `f64` (`.rdata`).
pub const FOG_255: u32 = 0x00b1_70c0;
/// 255.0 as an `f32` (`.rdata`).
pub const FOG_255_F32: u32 = 0x00b4_80d4;
/// The window handlers by kind (`yiWindowSetHandlers`): 6 words a kind.
pub const WINDOW_HANDLERS: u32 = 0x00c5_d788;
/// The idle window handler (`0x00907e50`) the window finishers set.
pub const WINDOW_IDLE: u32 = 0x0090_7e50;
/// The handler of an open window (`0x00907d70`, returns 0).
pub const WINDOW_OPEN_IDLE: u32 = 0x0090_7d70;
/// `yiWindowFinishAll` (`0x00907040`), a closing window's last handler.
pub const WINDOW_FINISH_ALL: u32 = 0x0090_7040;
/// 416.0 and 512.0 as `f32`s (`.rdata`): the window area's height and
/// width.
pub const WINDOW_HEIGHT_F32: u32 = 0x00b4_1d98;
/// See [`WINDOW_HEIGHT_F32`].
pub const WINDOW_WIDTH_F32: u32 = 0x00b4_1d9c;
/// 416.0, 512.0, 0.0 and 6.0 as `f64`s (`.rdata`): the slide sizes and
/// the six frames they take.
pub const WINDOW_HEIGHT: u32 = 0x00b4_1d30;
/// See [`WINDOW_HEIGHT`].
pub const WINDOW_WIDTH: u32 = 0x00b4_1d38;
/// See [`WINDOW_HEIGHT`].
pub const WINDOW_ZERO: u32 = 0x00b0_db90;
/// See [`WINDOW_HEIGHT`].
pub const WINDOW_FRAMES: u32 = 0x00b3_eb88;
/// 8.0 and 28.0 (`f32`s, `.rdata`): the bar window's top and bottom.
pub const WINDOW_BAR_TOP: u32 = 0x00b9_2314;
/// See [`WINDOW_BAR_TOP`].
pub const WINDOW_BAR_BOTTOM: u32 = 0x00b5_ee74;
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
    /// `+0x88`: a float kept with the transform by `yiPushRenderState`.
    pub const WORD_88: u32 = 0x88;
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
    /// The objects that can take an animation (`yiAnimAttachDefault`'s
    /// bound).
    pub const ANIM_IDS: u32 = 0x68788;
    /// Animations in use.
    pub const ANIM_COUNT: u32 = 0x9cabc;
    /// Its flags (bit 2: colour-scaled background, bit 25: no frame
    /// register in the GS set-up).
    pub const FLAGS: u32 = 0x67f08;
    /// Its colour scale, four floats (`yiRender_setupparamGS`).
    pub const COLOR_SCALE: u32 = 0x6820c;
    /// Its colour offset.
    pub const COLOR_OFFSET: u32 = 0x6821c;
    /// The background colour, RGB bytes.
    pub const BACK_COLOR: u32 = 0x68340;
    /// The frame buffer base (bytes; `>> 11` for the GS).
    pub const FRAME_BASE: u32 = 0x9cab4;
    /// The animation slots: `+0` flags (bit 0 in use; `u16` type at `+0`
    /// for keyframes), `+2` `u16` argument, `+4` `u16` kind, `+6` `u16`
    /// object id, `+8` keyframe index, `+0xc`/`+0x10` arguments, `+0x14`
    /// and `+0x18` the start time, `+0x24` the keyframe data.
    pub const ANIM: u32 = 0x9cac0;
    /// Bytes per slot.
    pub const ANIM_SIZE: u32 = 0x28;
    /// Slots.
    pub const ANIMS: u32 = 0x40;
    /// The keyframe data pointers, one per keyframe index
    /// (`yiKeyFrame_attachChr`).
    pub const KEYFRAMES: u32 = 0x9d4c4;
    /// Texture swaps in use (`yiTexSwap_init` clears it).
    pub const TEX_SWAP_COUNT: u32 = 0x9d9c4;
    /// The texture swap entries.
    pub const TEX_SWAP: u32 = 0x9d9c8;
    /// Bytes per entry.
    pub const TEX_SWAP_SIZE: u32 = 0x720;
    /// Entries.
    pub const TEX_SWAPS: u32 = 0x40;

    /// Offsets in a texture swap entry.
    pub mod tex_swap {
        /// `+0x4`: the frame shown (`yiTexSwapSetFrame`, with bit 0 of
        /// `+0`).
        pub const FRAME: u32 = 0x4;
        /// `+0x1c`: its frame count.
        pub const FRAMES: u32 = 0x1c;
    }
}

/// More renderer state (round 4: colours, the render state save area).
pub mod state {
    use ffx_bridge::Global;

    /// The renderer object (`yiRender_*`'s `this`; the texture swaps of
    /// `yiTexSwapSetFrame`).
    pub const RENDER: Global<u32> = Global::new(0x0187_6e00);
    /// The colour scale, four floats (`yiGetColorScale`, `yiSetColorScale`).
    pub const COLOR_SCALE: Global<[f32; 4]> = Global::new(0x018d_f00c);
    /// The battle centre, four floats divided by the scale factor
    /// (`yiSetBattleCenter`).
    pub const BATTLE_CENTER: Global<[u32; 4]> = Global::new(0x018d_f02c);
    /// A word saved with the render state (`0x018df140`).
    pub const RENDER_WORD: Global<u32> = Global::new(0x018d_f140);
    /// Three floats saved with the render state (`0x018df144`).
    pub const RENDER_FLOATS: Global<[u32; 3]> = Global::new(0x018d_f144);
    /// The colour shift, four floats (`yiSetColorShift` copies it out).
    pub const COLOR_SHIFT: Global<[u32; 4]> = Global::new(0x018d_f150);
    /// The background colour, RGBA bytes (`yiGetBackColor`).
    pub const BACK_COLOR: Global<[u8; 4]> = Global::new(0x018d_f178);
    /// 255.0 as an `f64` (`.rdata`, `yiGetBackColor`'s bound).
    pub const BACK_MAX: Global<f64> = Global::new(0x00b1_70c0);
    /// 255.0 as an `f32` (`.rdata`, what it stores past the bound).
    pub const BACK_MAX_F32: Global<f32> = Global::new(0x00b4_80d4);
    /// The render state save area (`yiPushRenderState`).
    pub const SAVED: Global<u32> = Global::new(0x0193_1230);
    /// Four floats a script native sets (`yiSetFieldVec4`).
    pub const FIELD_VEC4: Global<[u32; 4]> = Global::new(0x0193_1208);
    /// Six floats a script native sets (`yiSetFieldVec6`).
    pub const FIELD_VEC6: Global<[u32; 6]> = Global::new(0x0193_1218);
    /// Flags and floats two script natives set (`yiSetFieldPair`,
    /// `yiSetFieldVec3Int`).
    pub const FIELD_FLAGS: Global<u32> = Global::new(0x0193_4e78);
    /// The view matrix (`yiSetViewMatrix`).
    pub const VIEW_MATRIX: Global<[u32; 16]> = Global::new(0x018d_ed38);
    /// Two floats `yiSetViewMatrix` takes from `+0x20` of its matrix.
    pub const VIEW_EXTRA: Global<[u32; 2]> = Global::new(0x018d_f168);
    /// CLUTs in the first VRAM area (`yiClutAlloc_makepacket`: 32, or 16
    /// for texture mode 4).
    pub const CLUT_SPLIT: Global<u32> = Global::new(0x0193_bb14);
    /// Non-zero once a map is read (`yiCallFieldParticle`'s gate).
    pub const MAP_READ: Global<u32> = Global::new(0x0191_38a0);
    /// Non-zero: animations time by the vertical blank counter, else by
    /// fields (`yiAnimAttachDefault`).
    pub const USE_VCOUNT: Global<u32> = Global::new(0x0193_bb08);
    /// The image VRAM regions' limits and next addresses (`0x025d17a8`).
    pub const VRAM_REGIONS: Global<u32> = Global::new(0x025d_17a8);
    /// Per region (`yiVramAllocPsm` order): next and limit offsets from
    /// [`VRAM_REGIONS`]; region 0 is `yiVramAlloc`'s.
    pub const VRAM_REGION: [(u32, u32); 4] = [(0x14, 0x4), (0x10, 0x0), (0x18, 0x8), (0x1c, 0xc)];

    /// Offsets in the save area.
    pub mod saved {
        /// The render status.
        pub const STATUS: u32 = 0x0;
        /// The colour scale.
        pub const SCALE: u32 = 0x4;
        /// The colour offset.
        pub const OFFSET: u32 = 0x14;
        /// The word.
        pub const WORD: u32 = 0x24;
        /// The three floats.
        pub const FLOATS: u32 = 0x28;
        /// The colour shift.
        pub const SHIFT: u32 = 0x34;
        /// The display objects: per object its `+0x88` float, rotation,
        /// translation and matrix.
        pub const DOBJS: u32 = 0x44;
        /// Bytes per object.
        pub const DOBJ_SIZE: u32 = 100;
        /// Objects saved.
        pub const DOBJ_COUNT: u32 = 128;
    }
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

    // ---- round 4: colours, render state, textures ----

    use super::state;
    use ffx_bridge::x87::{Reg, ftol};
    use ffx_ps2::vu0::bridge::copy_vector;

    /// `yiGetColorScale(out)` (`0x009088d0`): returns `FFXVu0CopyVector`'s
    /// `eax` (the last word).
    pub fn yi_get_color_scale(mem: &mut impl Mem, out: u32) -> u32 {
        let s = state::COLOR_SCALE.ptr(mem).addr;
        copy_vector(mem, out, s)
    }

    /// `yiSetColorScale(v)` (`0x0090a270`).
    pub fn yi_set_color_scale(mem: &mut impl Mem, v: u32) -> u32 {
        let s = state::COLOR_SCALE.ptr(mem).addr;
        copy_vector(mem, s, v)
    }

    /// `yiSetColorShift(out)` (`0x00908c30`, Fahrenheit's name): copies the
    /// colour shift *out*.
    pub fn yi_set_color_shift(mem: &mut impl Mem, out: u32) -> u32 {
        let s = state::COLOR_SHIFT.ptr(mem).addr;
        copy_vector(mem, out, s)
    }

    /// `yiSetBattleCenter(v)` (`0x0090a0e0`): `v` divided by the scale
    /// factor (four words from `g_YiScaleFactor`), each `fld; fdiv m32;
    /// fstp`.
    pub fn yi_set_battle_center(mem: &mut impl Mem, v: u32) -> u32 {
        let c = state::BATTLE_CENTER.ptr(mem).addr;
        let r = copy_vector(mem, c, v);
        let s = SCALE_FACTOR.ptr(mem).addr;
        for i in 0..4 {
            let x = mem.read::<f32>(c + 4 * i);
            let k = mem.read::<f32>(s + 4 * i);
            mem.write(c + 4 * i, Reg::load(x).div_m32(k).store());
        }
        r
    }

    /// `yiGetBattleCenter(out)` (`0x009087f0`): the centre times the scale
    /// factor.
    pub fn yi_get_battle_center(mem: &mut impl Mem, out: u32) -> u32 {
        let c = state::BATTLE_CENTER.ptr(mem).addr;
        let r = copy_vector(mem, out, c);
        let s = SCALE_FACTOR.ptr(mem).addr;
        for i in 0..4 {
            let x = mem.read::<f32>(out + 4 * i);
            let k = mem.read::<f32>(s + 4 * i);
            mem.write(out + 4 * i, Reg::load(x).mul_m32(k).store());
        }
        r
    }

    fn copy_float(mem: &mut impl Mem, dst: u32, src: u32) {
        let v = mem.read::<u32>(src);
        mem.write(dst, x87::copy(v));
    }

    fn copy_word(mem: &mut impl Mem, dst: u32, src: u32) {
        let v = mem.read::<u32>(src);
        mem.write(dst, v);
    }

    /// `yiPushRenderState()` (`0x00909c60`): the render status, colours and
    /// every display object's transform into the save area; returns the
    /// `eax` of the last `FFXVu0CopyVector`.
    pub fn yi_push_render_state(mem: &mut impl Mem) -> u32 {
        render_state(mem, true)
    }

    /// `yiPopRenderState()` (`0x00909b90`): the reverse.
    pub fn yi_pop_render_state(mem: &mut impl Mem) -> u32 {
        render_state(mem, false)
    }

    fn render_state(mem: &mut impl Mem, push: bool) -> u32 {
        let live = [
            state::RENDER_FLOATS.ptr(mem).addr,
            RENDER_STATUS.ptr(mem).addr,
            state::RENDER_WORD.ptr(mem).addr,
            state::COLOR_SHIFT.ptr(mem).addr,
            state::COLOR_SCALE.ptr(mem).addr,
            COLOR_OFFSET.ptr(mem).addr,
            DOBJS.ptr(mem).addr,
        ];
        let save = state::SAVED.ptr(mem).addr;
        let saved = [
            save + state::saved::FLOATS,
            save + state::saved::STATUS,
            save + state::saved::WORD,
            save + state::saved::SHIFT,
            save + state::saved::SCALE,
            save + state::saved::OFFSET,
            save + state::saved::DOBJS,
        ];
        let (src, dst) = if push { (live, saved) } else { (saved, live) };
        for i in 0..3 {
            copy_float(mem, dst[0] + 4 * i, src[0] + 4 * i);
        }
        copy_word(mem, dst[1], src[1]);
        copy_word(mem, dst[2], src[2]);
        let mut r = 0;
        for k in 3..6 {
            r = copy_vector(mem, dst[k], src[k]);
        }
        for i in 0..state::saved::DOBJ_COUNT {
            let o = live[6] + i * dobj::SIZE;
            let s = saved[6] + i * state::saved::DOBJ_SIZE;
            let pairs = [
                (s, o + dobj::WORD_88),
                (s + 4, o + dobj::ROTATION),
                (s + 0x14, o + dobj::TRANSLATION),
                (s + 0x24, o + dobj::MATRIX),
            ];
            let at = |p: (u32, u32), to_save: bool| if to_save { p } else { (p.1, p.0) };
            let (d, f) = at(pairs[0], push);
            copy_float(mem, d, f);
            let (d, f) = at(pairs[1], push);
            copy_vector(mem, d, f);
            let (d, f) = at(pairs[2], push);
            r = copy_vector(mem, d, f);
            let (d, f) = at(pairs[3], push);
            for w in 0..16 {
                copy_word(mem, d + 4 * w, f + 4 * w);
            }
        }
        r
    }

    /// `yiGetBackColor(out)` (`0x009086d0`): the background colour, four
    /// ints. With render status bits 0 and 2 each channel is
    /// `channel * scale + offset` (each step stored as `f32`), below 0 made
    /// 0, above 255.0 (an `f64` in `.rdata`) made 255.0 (an `f32` there),
    /// through `_ftol2_sse`; else the raw bytes. Returns the last word.
    pub fn yi_get_back_color(mem: &mut impl Mem, out: u32) -> u32 {
        let c = state::BACK_COLOR.get(mem);
        let status = RENDER_STATUS.get(mem);
        if status & 1 == 0 || status & 4 == 0 {
            let w = c.map(u32::from);
            mem.write(out, w);
            return w[3];
        }
        let mut v = c.map(f32::from);
        let top = state::BACK_MAX.get(mem);
        let top_f = state::BACK_MAX_F32.get(mem);
        let scale = state::COLOR_SCALE.get(mem);
        let offset = COLOR_OFFSET.get(mem);
        let mut last = 0;
        for i in 0..4 {
            let x = Reg::load(scale[i]).mul_m32(v[i]).store();
            let x = Reg::load(x).add_m32(f32::from_bits(offset[i])).store();
            v[i] = x;
            let lx = ffx_bridge::x87::load(x.to_bits());
            // fcomp with 0.0: below (not unordered) stores +0.0
            if lx < 0.0 {
                v[i] = 0.0;
            }
            let lv = ffx_bridge::x87::load(v[i].to_bits());
            // fcomp with 255.0: above (not equal, not unordered) stores 255.0f
            if lv > top {
                v[i] = top_f;
            }
            last = ftol(mem, ffx_bridge::x87::load(v[i].to_bits()));
            mem.write(out + 4 * u32::try_from(i).unwrap_or(0), last);
        }
        last
    }

    /// `yiImage_init(image)` (`0x009188c0`): returns `set_ptr32(0)`, 0.
    pub fn yi_image_init(mem: &mut impl Mem, im: u32) -> u32 {
        mem.write(im, 0u32);
        mem.write(im + 0x114, 0u32);
        mem.write(im + 0x110, 0u32);
        mem.write(im + 4, 0u8);
        mem.write(im + 0x144, 0u32);
        mem.write(im + 0x124, u32::MAX);
        mem.write(im + 0x160, 0u32);
        mem.write(im + 0x15c, 0u32);
        0
    }

    /// `yiTexSwap_init(render)` (`0x009152d0`): clears the texture swap
    /// count and its 64 entries (each eight words and 64 four-word slots
    /// of `0x1c` bytes); returns the pointer the loop ends on.
    pub fn yi_tex_swap_init(mem: &mut impl Mem, r: u32) -> u32 {
        mem.write(r.wrapping_add(render::TEX_SWAP_COUNT), 0u32);
        let mut end = 0;
        for i in 0..render::TEX_SWAPS {
            let e = r.wrapping_add(render::TEX_SWAP + i * render::TEX_SWAP_SIZE);
            mem.write(e, [0u32; 8]);
            for k in 0..64 {
                mem.write(e.wrapping_add(0x20 + k * 0x1c), [0u32; 4]);
            }
            end = e.wrapping_add(4 + 0x20 + 64 * 0x1c);
        }
        end
    }

    /// `yiTexSwapSetFrame(index, frame)` (`0x00909e30`, script native
    /// `0x008574c0`): for an entry below the count and a frame below its
    /// frame count, sets its bit 0 and the frame; returns 0.
    pub fn yi_tex_swap_set_frame(mem: &mut impl Mem, index: u32, frame: u32) -> u32 {
        let r = state::RENDER.ptr(mem).addr;
        if index < mem.read::<u32>(r + render::TEX_SWAP_COUNT) {
            let e = r.wrapping_add(render::TEX_SWAP + index.wrapping_mul(render::TEX_SWAP_SIZE));
            if frame < mem.read::<u32>(e.wrapping_add(render::tex_swap::FRAMES)) {
                let f = mem.read::<u32>(e);
                mem.write(e, f | 1);
                mem.write(e.wrapping_add(render::tex_swap::FRAME), frame);
            }
        }
        0
    }

    fn clock(mem: &impl Mem) -> u32 {
        if state::USE_VCOUNT.get(mem) == 0 {
            yi_get_fcount(mem)
        } else {
            yi_get_vcount(mem)
        }
    }

    fn free_anim(mem: &impl Mem, r: u32) -> Option<u32> {
        (0..render::ANIMS).find(|&i| {
            let a = r.wrapping_add(render::ANIM + i * render::ANIM_SIZE);
            mem.read::<u8>(a) & 1 == 0
        })
    }

    /// `yiAnimAttachDefault_009143c0(render, id, a, b)`: for an id below
    /// the object count not attached yet, the first free animation slot
    /// (type 4, flags 5, both times the current clock); always returns 0.
    pub fn yi_anim_attach_default(mem: &mut impl Mem, r: u32, id: u32, a: u32, b: u32) -> u32 {
        if id >= mem.read::<u32>(r.wrapping_add(render::ANIM_IDS)) {
            return 0;
        }
        for i in 0..render::ANIMS {
            let s = r.wrapping_add(render::ANIM + i * render::ANIM_SIZE);
            if mem.read::<u8>(s) & 1 != 0 && u32::from(mem.read::<u16>(s + 6)) == id {
                return 0;
            }
        }
        let now = clock(mem);
        let Some(i) = free_anim(mem, r) else { return 0 };
        let s = r.wrapping_add(render::ANIM + i * render::ANIM_SIZE);
        mem.write(s + 4, 4u16);
        mem.write(s + 0xc, a);
        mem.write(s + 0x10, b);
        mem.write(s + 0x14, now);
        mem.write(s + 0x18, now);
        mem.write(s, 5u32);
        #[allow(clippy::cast_possible_truncation)] // the low 16 bits
        mem.write(s + 6, id as u16);
        mem.write(s + 8, 0u32);
        mem.write(s + 0x24, 0u32);
        let n = mem.read::<u32>(r.wrapping_add(render::ANIM_COUNT));
        mem.write(r.wrapping_add(render::ANIM_COUNT), n.wrapping_add(1));
        0
    }

    /// `yiKeyFrame_attachChr(render, id, keyframe, arg)` (`0x00914630`):
    /// with keyframe data `keyframe` and an id below the texture swap
    /// count, the first free animation slot (type 1); returns the `eax` it
    /// ends with: the clock, 64 when no slot is free, else the keyframe
    /// data.
    pub fn yi_key_frame_attach_chr(mem: &mut impl Mem, r: u32, id: u32, kf: u32, arg: u16) -> u32 {
        let now = clock(mem);
        let table = r.wrapping_add(render::KEYFRAMES + kf.wrapping_mul(4));
        let data = mem.read::<u32>(table);
        if data == 0 || id >= mem.read::<u32>(r.wrapping_add(render::TEX_SWAP_COUNT)) {
            return now;
        }
        let Some(i) = free_anim(mem, r) else {
            return render::ANIMS;
        };
        let s = r.wrapping_add(render::ANIM + i * render::ANIM_SIZE);
        mem.write(s, 1u16);
        mem.write(s + 2, arg);
        let ty = mem.read::<u16>(data.wrapping_add(8));
        mem.write(s + 4, ty);
        #[allow(clippy::cast_possible_truncation)] // the low 16 bits
        {
            mem.write(s + 6, id as u16);
            mem.write(s + 8, kf as u16);
        }
        mem.write(s + 0xa, 0u16);
        mem.write(s + 0x14, now);
        mem.write(s + 0x18, now);
        let data = mem.read::<u32>(table);
        mem.write(s + 0x24, data);
        let n = mem.read::<u32>(r.wrapping_add(render::ANIM_COUNT));
        mem.write(r.wrapping_add(render::ANIM_COUNT), n.wrapping_add(1));
        data
    }

    /// `yiSetFieldVec4i(a, b, c, d)` (`0x0090a1d0`): four ints as floats
    /// (`fild; fstp`).
    pub fn yi_set_field_vec4i(mem: &mut impl Mem, v: [i32; 4]) {
        let at = state::FIELD_VEC4.ptr(mem).addr;
        #[allow(clippy::cast_precision_loss)] // fild rounds to f32 on the store
        mem.write(at, v.map(|x| x87::store(f64::from(x))));
    }

    /// `yiSetFieldVec4(a, b, c, d)` (`0x0090a200`, script native
    /// `0x0091c600`): four floats.
    pub fn yi_set_field_vec4(mem: &mut impl Mem, v: [u32; 4]) {
        let at = state::FIELD_VEC4.ptr(mem).addr;
        mem.write(at, v.map(x87::copy));
    }

    /// `yiSetFieldVec6(a..f)` (`0x0090a230`, script native `0x0091c6b0`):
    /// six floats.
    pub fn yi_set_field_vec6(mem: &mut impl Mem, v: [u32; 6]) {
        let at = state::FIELD_VEC6.ptr(mem).addr;
        mem.write(at, v.map(x87::copy));
    }

    /// `yiSetFieldPair(on, a, b)` (`0x0090a430`, script native
    /// `0x008591d0`): two floats; flags bit 0, and bit 1 when `on`;
    /// returns 0.
    pub fn yi_set_field_pair(mem: &mut impl Mem, on: u32, a: u32, b: u32) -> u32 {
        let base = state::FIELD_FLAGS.ptr(mem).addr;
        let f = mem.read::<u32>(base);
        mem.write(base + 4, x87::copy(a));
        mem.write(base + 0x24, x87::copy(b));
        mem.write(base, f | if on == 0 { 1 } else { 3 });
        0
    }

    /// `yiSetFieldVec3Int(a, b, c, n)` (`0x0090a470`, script native
    /// `0x00858c90`): three floats and an int; returns 0.
    pub fn yi_set_field_vec3_int(mem: &mut impl Mem, a: u32, b: u32, c: u32, n: u32) -> u32 {
        let base = state::FIELD_FLAGS.ptr(mem).addr;
        mem.write(base + 0x18, x87::copy(a));
        mem.write(base + 0x28, n);
        mem.write(base + 0x1c, x87::copy(b));
        mem.write(base + 0x20, x87::copy(c));
        0
    }

    /// `yiMapGetSection(map, index)` (`0x00907f00`): for data starting
    /// with `MAP1`, its section `index` (an offset at `+0x10 + 4 * index`)
    /// as a pointer; 0 for another tag or an empty offset.
    pub fn yi_map_get_section(mem: &impl Mem, map: u32, index: u32) -> u32 {
        if mem.read::<[u8; 4]>(map) != *b"MAP1" {
            return 0;
        }
        match mem.read::<u32>(map.wrapping_add(0x10).wrapping_add(index.wrapping_mul(4))) {
            0 => 0,
            o => o.wrapping_add(map),
        }
    }

    /// `yiVramAllocPsm(bytes, psm)` (`0x00918090`): like `yiVramAlloc`
    /// from the main region, then the second; for PSMT8 (`0x13`) and PSMT4
    /// (`0x14`) images four or eight times the size from the third or the
    /// fourth (in their units); -1 when none fits.
    pub fn yi_vram_alloc_psm(mem: &mut impl Mem, bytes: i32, psm: u32) -> u32 {
        let size = super::vram_end(0, bytes);
        if let Some(a) = vram_take(mem, 0, size).or_else(|| vram_take(mem, 1, size)) {
            return a;
        }
        let n = match psm {
            0x13 => size.wrapping_shl(2),
            0x14 => size.wrapping_shl(3),
            _ => return u32::MAX,
        };
        vram_take(mem, 2, n)
            .or_else(|| vram_take(mem, 3, n))
            .unwrap_or(u32::MAX)
    }

    /// Takes `n` bytes from VRAM region `i`: its old next address `>> 6`.
    fn vram_take(mem: &mut impl Mem, i: u32, n: u32) -> Option<u32> {
        let regions = state::VRAM_REGIONS.ptr(mem).addr;
        let (next, limit) = state::VRAM_REGION[i as usize];
        let (next, limit) = (regions + next, regions + limit);
        let at = mem.read::<u32>(next);
        let end = at.wrapping_add(n);
        if end > mem.read::<u32>(limit) {
            return None;
        }
        mem.write(next, end);
        Some(at >> 6)
    }

    /// `yiAttachChrDObj(id, a, b)` (`0x00908350`):
    /// [`yi_anim_attach_default`] on the renderer; returns 0.
    pub fn yi_attach_chr_dobj(mem: &mut impl Mem, id: u32, a: u32, b: u32) -> u32 {
        let r = state::RENDER.ptr(mem).addr;
        yi_anim_attach_default(mem, r, id, a, b)
    }

    /// `yiCallFieldParticle()` (`0x009083e0`): with a map read
    /// (`0x019138a0`) and render status bit 0, the field particles run
    /// (`pppFpLoop`, `fp_loop`, a tail jump).
    pub fn yi_call_field_particle<M: Mem>(mem: &mut M, fp_loop: impl FnOnce(&mut M)) {
        if state::MAP_READ.get(mem) != 0 && RENDER_STATUS.get(mem) & 1 != 0 {
            fp_loop(mem);
        }
    }

    /// `yiClutAlloc_makepacket(packet, cluts, n)` (`0x00917050`): the CLUT
    /// upload packet: a VIF `DIRECT` of `n * 0x46` quadwords, then 72
    /// transfers of `0x460` bytes from `+0x10`, each a GIF A+D tag of four
    /// registers (`BITBLTBUF`, `TRXPOS`, `TRXREG` 16 x 16, `TRXDIR`; the
    /// PC build leaves their values 0), an `IMAGE` tag of 64 quadwords and
    /// the next 256-colour CLUT from `cluts` (left alone when null). The
    /// texture mode (`+0x67f20`: 2, 3 → 32, 4 → 16) sets how many CLUTs
    /// go to the first VRAM area (`0x0193bb14`, which the PC build does
    /// not otherwise use). The packet (`0x13b10` bytes, its end count at
    /// `+0x13b10`) is copied to the second buffer at `+0x13b14` and the
    /// count to `+0x676a4`. Returns 0.
    pub fn yi_clut_alloc_makepacket(mem: &mut impl Mem, pk: u32, cluts: u32, n: u32) -> u32 {
        match mem.read::<u32>(pk.wrapping_add(0x67f20)) {
            2 | 3 => state::CLUT_SPLIT.set(mem, 0x20),
            4 => state::CLUT_SPLIT.set(mem, 0x10),
            _ => {}
        }
        let qwc = n.wrapping_mul(0x46);
        mem.write(pk.wrapping_add(0xc), qwc | 0x5100_0000);
        mem.write(pk, [0u32; 3]);
        let mut src = cluts;
        for i in 0..72u32 {
            let rec = pk.wrapping_add(0x10 + i * 0x460);
            mem.write(
                rec,
                [
                    4u32,
                    0x1003_0000,
                    0xe,
                    0,
                    0,
                    0,
                    0x50,
                    0,
                    0,
                    0,
                    0x51,
                    0,
                    0x10,
                    0,
                    0x52,
                    0,
                    0,
                    0,
                    0x53,
                    0,
                    0x8040,
                    0x0800_0000,
                    0,
                    0,
                ],
            );
            if src != 0 {
                let c = mem.read::<[u32; 256]>(src);
                mem.write(rec.wrapping_add(0x60), c);
                src = src.wrapping_add(0x400);
            }
        }
        let end = qwc.wrapping_add(1);
        mem.write(pk.wrapping_add(0x13b10), end);
        let mut buf = vec![0u8; 0x13b10];
        mem.read_bytes(pk, &mut buf);
        mem.write_bytes(pk.wrapping_add(0x13b14), &buf);
        let end = mem.read::<u32>(pk.wrapping_add(0x13b10));
        mem.write(pk.wrapping_add(0x676a4), end);
        0
    }

    /// The renderer's background colour as the GS `BGCOLOR`-style word
    /// `b << 16 | g << 8 | r`: the raw bytes (`+0x68340`), or with flag
    /// bit 2 (`+0x67f08`) each scaled and offset by the renderer's own
    /// colour scale and offset (`+0x6820c`, `+0x6821c`), clamped to
    /// 0..=255 (as `yiGetBackColor`) and converted with `_ftol2`.
    fn render_back_color(mem: &impl Mem, r: u32) -> u32 {
        let c = mem.read::<[u8; 3]>(r.wrapping_add(render::BACK_COLOR));
        if mem.read::<u8>(r.wrapping_add(render::FLAGS)) & 4 == 0 {
            return u32::from(c[2]) << 16 | u32::from(c[1]) << 8 | u32::from(c[0]);
        }
        let top = state::BACK_MAX.get(mem);
        let top_f = state::BACK_MAX_F32.get(mem);
        let ch = |i: u32| -> u32 {
            let s = mem.read::<f32>(r.wrapping_add(render::COLOR_SCALE + 4 * i));
            let o = mem.read::<f32>(r.wrapping_add(render::COLOR_OFFSET + 4 * i));
            let x = Reg::load(s).mul_m32(f32::from(c[i as usize])).store();
            let mut v = Reg::load(o).add_m32(x).store();
            if ffx_bridge::x87::load(v.to_bits()) < 0.0 {
                v = 0.0;
            }
            if ffx_bridge::x87::load(v.to_bits()) > top {
                v = top_f;
            }
            ffx_bridge::x87::ftol2(ffx_bridge::x87::load(v.to_bits()))
        };
        let (red, green, blue) = (ch(0), ch(1), ch(2));
        blue.wrapping_shl(16) | green.wrapping_shl(8) | red
    }

    /// `yiRender_setupparamGS(render)` (`0x0090d260`): the two GS
    /// set-up packets (`+0x677a8`, `+0x678a8`): a DMA tag of 12
    /// quadwords, VIF `FLUSH` and `DIRECT` 11, a GIF A+D tag of 10
    /// registers (`PRMODECONT`, `TEST`, `COLCLAMP`, `DTHE`, ..., the
    /// background colour, `FOGCOL`-style `0x3d`, `0x3b`, `0x49`, and the
    /// frame register `0x4f` from `+0x9cab4 >> 11` unless flag bit 25 is
    /// set). The two differ only in `0x7000d` / `0x5000d`.
    pub fn yi_render_setupparam_gs(mem: &mut impl Mem, r: u32) {
        let color = render_back_color(mem, r);
        let flags = mem.read::<u32>(r.wrapping_add(render::FLAGS));
        let (frame, frame_reg) = if flags & 0x0200_0000 == 0 {
            let fb = mem.read::<u32>(r.wrapping_add(render::FRAME_BASE));
            ((fb >> 11) | 0x0100_0000, 0x4f)
        } else {
            (0, 0)
        };
        for (at, mode) in [(0x677a8, 0x0007_000du32), (0x678a8, 0x0005_000d)] {
            let words: [u32; 52] = [
                0x2000_000c,
                0,
                0,
                0,
                0x1100_0000,
                0,
                0,
                0x5100_000b,
                0x800a,
                0x1000_0000,
                0xe,
                0,
                0,
                0,
                0x3f,
                0,
                mode,
                0,
                0x48,
                0,
                0x60,
                0,
                0x15,
                0,
                0x44,
                0,
                0x43,
                0,
                0,
                0,
                0x4b,
                0,
                0,
                0,
                10,
                0,
                color,
                0,
                0x3d,
                0,
                0x80,
                0,
                0x3b,
                0,
                0,
                0,
                0x49,
                0,
                frame,
                0,
                frame_reg,
                0,
            ];
            mem.write(r.wrapping_add(at), words);
        }
    }

    /// `yiArcPrintSize()` (`0x0090e6f0`): prints the archive section sizes
    /// through `dbgPrintf`, empty on PC; returns 0.
    #[must_use]
    pub const fn yi_arc_print_size() -> u32 {
        0
    }

    /// `yiSetViewMatrix(m)` (`0x0090a980`): unless render status bit 11 is
    /// set, the view matrix (`0x018ded38`, `FFXVu0CopyMatrix`) and the
    /// floats at `m +0x20`, `+0x24` into `0x018df168`.
    pub fn yi_set_view_matrix(mem: &mut impl Mem, m: u32) {
        if RENDER_STATUS.get(mem) & 0x800 != 0 || m == 0 {
            return;
        }
        let at = state::VIEW_MATRIX.ptr(mem).addr;
        ffx_ps2::vu0::bridge::matrix_copy(mem, at, m);
        let a = mem.read::<u32>(m + 0x20);
        let b = mem.read::<u32>(m + 0x24);
        let at = state::VIEW_EXTRA.ptr(mem).addr;
        mem.write(at, [x87::copy(a), x87::copy(b)]);
    }

    /// `yiPolyBufClear(buf)` (`0x00911970`): zeroes the first `0x4000`
    /// bytes, the counters at `+0x4000`/`+0x4008` and the words
    /// `+0x4048..=+0x4080`, and the `u16` at `+0x4084`. Returns `buf`.
    pub fn yi_poly_buf_clear(mem: &mut impl Mem, buf: u32) -> u32 {
        for row in 0..0x400u32 {
            let at = buf.wrapping_add(8 + 16 * row);
            for d in [4u32, 0, 0u32.wrapping_sub(4), 0u32.wrapping_sub(8)] {
                mem.write(at.wrapping_add(d), 0u32);
            }
        }
        for off in [
            0x4000u32, 0x4008, 0x4054, 0x4050, 0x404c, 0x4048, 0x4064, 0x4060, 0x405c, 0x4058,
            0x4074, 0x4078, 0x406c, 0x4070, 0x407c, 0x4080,
        ] {
            mem.write(buf.wrapping_add(off), 0u32);
        }
        mem.write(buf.wrapping_add(0x4084), 0u16);
        buf
    }

    /// The colour bytes of three GIF `RGBAQ` quadwords from `gif + 16 *
    /// first` (`r g b a` each) to `out`, the alphas first set to `alpha`
    /// (none when `first + 3` wraps); `+0x12` cleared. Returns the last.
    fn tri_colors(mem: &mut impl Mem, out: u32, gif: u32, first: u32, alpha: u32) -> u32 {
        mem.write(out.wrapping_add(0x12), 0u16);
        let end = first.wrapping_add(3);
        if first < end {
            let mut at = first.wrapping_mul(16).wrapping_add(0xc).wrapping_add(gif);
            for _ in first..end {
                mem.write(at, alpha);
                at = at.wrapping_add(16);
            }
        }
        let q = first.wrapping_mul(16).wrapping_add(gif);
        let mut last = 0;
        for i in 0..12u32 {
            last = mem.read::<u8>(q.wrapping_add(4 * i));
            mem.write(out.wrapping_add(i), last);
        }
        u32::from(last)
    }

    /// `yiTriColorsFromGif(out, gif, first)` (`0x00910340`): a triangle's
    /// three colours (`r g b a`, 12 bytes) from the GIF packet's `RGBAQ`
    /// quadwords at `first`, their alphas first set to 0x80; `+0x12`
    /// cleared. Returns the last byte copied.
    pub fn yi_tri_colors_from_gif(mem: &mut impl Mem, out: u32, gif: u32, first: u32) -> u32 {
        tri_colors(mem, out, gif, first, 0x80)
    }

    /// `yiTriColorsUvFromGif(out, gif, first, uv)` (`0x009103e0`): as
    /// [`yi_tri_colors_from_gif`] with alpha 0x7f, then the three `UV`
    /// quadwords at `uv` (`u16` u and v each) to `+0x14`. Returns the last
    /// `u16` copied.
    pub fn yi_tri_colors_uv_from_gif(
        mem: &mut impl Mem,
        out: u32,
        gif: u32,
        first: u32,
        uv: u32,
    ) -> u32 {
        tri_colors(mem, out, gif, first, 0x7f);
        let q = uv.wrapping_mul(16).wrapping_add(gif);
        let mut last = 0;
        for (i, off) in [0u32, 4, 0x10, 0x14, 0x20, 0x24].into_iter().enumerate() {
            last = mem.read::<u16>(q.wrapping_add(off));
            mem.write(
                out.wrapping_add(0x14 + 2 * u32::try_from(i).unwrap_or(0)),
                last,
            );
        }
        u32::from(last)
    }

    /// `yiLoadMatrixChunk(scene, chunk)` (`0x0090ed10`): `+4` of the chunk
    /// to `+0x6823c`, then 64 floats from `+0x40` to `+0x68240` (`fld` /
    /// `fstp`, so [`x87::copy`]).
    pub fn yi_load_matrix_chunk(mem: &mut impl Mem, scene: u32, chunk: u32) {
        let w = mem.read::<u32>(chunk.wrapping_add(4));
        mem.write(scene.wrapping_add(0x6823c), w);
        for i in 0..64u32 {
            let v = mem.read::<u32>(chunk.wrapping_add(0x40 + 4 * i));
            mem.write(scene.wrapping_add(0x68240 + 4 * i), x87::copy(v));
        }
    }

    /// `yiRender_setupFogParam(scene)` (`0x0090d1b0`): from the fog scale
    /// (`+0x681f8`, times 1/32 and 32) and the near/far (`+0x68348`,
    /// `+0x6834c`): `+0x68350` = -255 / (far - near) (0 when equal),
    /// `+0x68354` = 255 - `+0x68344` (255 with flag 8 at `+0x67f08`),
    /// `+0x68358` = -near, `+0x6835c` = 0.
    pub fn yi_render_setup_fog_param(mem: &mut impl Mem, scene: u32) {
        let ld = |mem: &_, off: u32| {
            ffx_bridge::x87::load(Mem::read::<u32>(mem, scene.wrapping_add(off)))
        };
        let k = |mem: &_, at: u32| {
            let a = Mem::rebase(mem, at);
            Mem::read::<f64>(mem, a)
        };
        let s = ld(&*mem, 0x681f8) * k(&*mem, super::FOG_EIGHTH) * k(&*mem, super::FOG_SCALE32);
        let far = f64::from(f32::from_bits(x87::store(ld(&*mem, 0x6834c) * s)));
        let near = f64::from(f32::from_bits(x87::store(s * ld(&*mem, 0x68348))));
        let c255 = k(&*mem, super::FOG_255);
        #[allow(clippy::float_cmp)] // fucomp: exact
        let r = if near == far {
            0
        } else {
            x87::store(-(c255 / (far - near)))
        };
        mem.write(scene.wrapping_add(0x68350), r);
        let p = ld(&*mem, 0x68344);
        mem.write(scene.wrapping_add(0x68354), x87::store(c255 - p));
        mem.write(scene.wrapping_add(0x68358), x87::store(-near));
        mem.write(scene.wrapping_add(0x6835c), 0u32);
        if mem.read::<u8>(scene.wrapping_add(0x67f08)) & 8 != 0 {
            let a = mem.rebase(super::FOG_255_F32);
            let v = mem.read::<u32>(a);
            mem.write(scene.wrapping_add(0x68354), x87::copy(v));
        }
    }

    /// The fields `yiObjSave`/`yiObjLoad` copy: (object offset, record
    /// offset, size: 2 a `u16`, 4 a word, 0 a float through `fld`/`fstp`).
    const OBJ_RECORD: [(u32, u32, u8); 18] = [
        (0x80, 0, 2),
        (0x82, 2, 2),
        (0x84, 4, 2),
        (0x88, 8, 4),
        (0x86, 0x30, 2),
        (0x50, 0x10, 0),
        (0x54, 0x14, 0),
        (0x58, 0x18, 0),
        (0x5c, 0x1c, 0),
        (0x60, 0x20, 0),
        (0x64, 0x24, 0),
        (0x68, 0x28, 0),
        (0x6c, 0x2c, 0),
        (0x8c, 0x34, 4),
        (0x90, 0x38, 2),
        (0x92, 0x3a, 2),
        (0x94, 0x3c, 2),
        (0x96, 0x3e, 2),
    ];

    /// `yiObjSave(record, obj)` (`0x0090ee60`): the object's state to a
    /// record ([`OBJ_RECORD`]; `+0x84` widened to a word). Returns the
    /// last `u16`.
    pub fn yi_obj_save(mem: &mut impl Mem, rec: u32, obj: u32) -> u32 {
        let mut last = 0;
        for (o, r, size) in OBJ_RECORD {
            let (from, to) = (obj.wrapping_add(o), rec.wrapping_add(r));
            match (size, o) {
                (2, 0x84) => mem.write(to, u32::from(mem.read::<u16>(from))),
                (2, _) => {
                    let v = mem.read::<u16>(from);
                    last = u32::from(v);
                    mem.write(to, v);
                }
                (4, _) => mem.write(to, mem.read::<u32>(from)),
                _ => mem.write(to, x87::copy(mem.read::<u32>(from))),
            }
        }
        last
    }

    /// `yiObjLoad(obj, record)` (`0x0090edb0`): the reverse of
    /// [`yi_obj_save`] (`+0x84` from the record's low half). Returns the
    /// last `u16`.
    pub fn yi_obj_load(mem: &mut impl Mem, obj: u32, rec: u32) -> u32 {
        let mut last = 0;
        for (o, r, size) in OBJ_RECORD {
            let (from, to) = (rec.wrapping_add(r), obj.wrapping_add(o));
            match size {
                2 => {
                    let v = mem.read::<u16>(from);
                    last = u32::from(v);
                    mem.write(to, v);
                }
                4 => mem.write(to, mem.read::<u32>(from)),
                _ => mem.write(to, x87::copy(mem.read::<u32>(from))),
            }
        }
        last
    }

    /// `yiWindowSetHandlers(window)` (`0x00907d80`): the six handlers of
    /// the window's kind (`+4`, `i16`; 6 words a kind from `0x00c5d788`) to
    /// `+0x60`..`+0x70` and `+0x78`. Returns the last.
    pub fn yi_window_set_handlers(mem: &mut impl Mem, window: u32) -> u32 {
        let mut last = 0;
        for (i, off) in [0x60u32, 0x64, 0x68, 0x6c, 0x70, 0x78]
            .into_iter()
            .enumerate()
        {
            let kind = i32::from(mem.read::<i16>(window.wrapping_add(4))).cast_unsigned();
            let table = mem.rebase(super::WINDOW_HANDLERS);
            let at = kind
                .wrapping_mul(0x18)
                .wrapping_add(table)
                .wrapping_add(4 * u32::try_from(i).unwrap_or(0));
            last = mem.read::<u32>(at);
            mem.write(window.wrapping_add(off), last);
        }
        last
    }

    /// `yiCameraLoad(camera, record)` (`0x0090ef10`): a camera record from
    /// the map file: `u16`s to `+0x30`, `+0x32`, `+0x36`, `+0x34`, `+0x38`
    /// a float, `+0x48` a word, and twelve floats (`+0x10..`) to `+0`.
    /// Returns the last `u16` (`+6`).
    pub fn yi_camera_load(mem: &mut impl Mem, cam: u32, rec: u32) -> u32 {
        let h = |mem: &mut _, d: u32, s: u32| {
            let v = Mem::read::<u16>(mem, rec.wrapping_add(s));
            Mem::write(mem, cam.wrapping_add(d), v);
            u32::from(v)
        };
        h(mem, 0x30, 0);
        h(mem, 0x32, 2);
        h(mem, 0x36, 4);
        let f = x87::copy(mem.read::<u32>(rec.wrapping_add(8)));
        mem.write(cam.wrapping_add(0x38), f);
        let w = mem.read::<u32>(rec.wrapping_add(0xc));
        mem.write(cam.wrapping_add(0x48), w);
        let last = h(mem, 0x34, 6);
        for i in 0..12u32 {
            let f = x87::copy(mem.read::<u32>(rec.wrapping_add(0x10 + 4 * i)));
            mem.write(cam.wrapping_add(4 * i), f);
        }
        last
    }

    /// `yiFogLoad(scene, record)` (`0x0090f090`): the fog record: `+0xc`
    /// to `+0x68340`, the floats `+0x10..=+0x18` to `+0x68344..`, `+0x1c`
    /// and `+0x20` to `+0xba400`/`+0xba404`, the colour words `+0..=+8` to
    /// `+0x68378..`, the floats `+0x30..=+0x3c` to `+0x6822c..`. Returns
    /// the colour's last word.
    pub fn yi_fog_load(mem: &mut impl Mem, scene: u32, rec: u32) -> u32 {
        let w = mem.read::<u32>(rec.wrapping_add(0xc));
        mem.write(scene.wrapping_add(0x68340), w);
        for (d, s) in [
            (0x68344u32, 0x10u32),
            (0x68348, 0x14),
            (0x6834c, 0x18),
            (0xba400, 0x1c),
            (0xba404, 0x20),
        ] {
            let f = x87::copy(mem.read::<u32>(rec.wrapping_add(s)));
            mem.write(scene.wrapping_add(d), f);
        }
        let mut last = 0;
        for i in 0..3u32 {
            last = mem.read::<u32>(rec.wrapping_add(4 * i));
            mem.write(scene.wrapping_add(0x68378 + 4 * i), last);
        }
        for i in 0..4u32 {
            let f = x87::copy(mem.read::<u32>(rec.wrapping_add(0x30 + 4 * i)));
            mem.write(scene.wrapping_add(0x6822c + 4 * i), f);
        }
        last
    }

    /// `yiWindow*` handlers (window kinds of [`super::WINDOW_HANDLERS`]):
    /// the window's `+0` (`u16`, active), `+6` (`i16`, opening progress
    /// in 1/4096), `+8` (`u16`, state: 2 open, 4 closed), `+0xc`/`+0xe`
    /// (`i16`, frames left and the reload), `+0x10` (`i16`, height),
    /// `+0x18..=+0x24` (the rectangle animated from), `+0x28..=+0x34` (the
    /// rectangle: x0, y0, x1, y1), `+0x38`/`+0x3c` (the per-frame steps),
    /// `+0x48..=+0x54` (the rectangle drawn), `+0x58` (colour), the
    /// handlers `+0x60..` and the current one `+0x74`. Each returns 1 when
    /// it moved to the next state.
    fn win_copy(mem: &mut impl Mem, w: u32, to: u32, from: u32) {
        let v = x87::copy(mem.read::<u32>(w.wrapping_add(from)));
        mem.write(w.wrapping_add(to), v);
    }

    fn win_const(mem: &impl Mem, c: u32) -> u32 {
        let at = mem.rebase(c);
        x87::copy(mem.read::<u32>(at))
    }

    fn win_f64(mem: &impl Mem, c: u32) -> f64 {
        let at = mem.rebase(c);
        mem.read::<f64>(at)
    }

    fn win_ld(mem: &impl Mem, w: u32, off: u32) -> f64 {
        ffx_bridge::x87::load(mem.read::<u32>(w.wrapping_add(off)))
    }

    /// The state change of a finished step: state `state`, the handler
    /// from slot `slot`. Returns 1.
    fn win_next(mem: &mut impl Mem, w: u32, state: u16, slot: u32) -> u32 {
        mem.write(w.wrapping_add(8), state);
        let h = mem.read::<u32>(w.wrapping_add(slot));
        mem.write(w.wrapping_add(0x74), h);
        1
    }

    /// A slide step: `+field` plus (or minus) the step at `+step`, one
    /// frame (`+0xc`) less; when none is left, the frames reloaded from
    /// `+0xe` (`reload`) and [`win_next`].
    #[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)] // the variants, as flags
    fn win_step(
        mem: &mut impl Mem,
        w: u32,
        field: u32,
        step: u32,
        add: bool,
        reload: bool,
        state: u16,
        slot: u32,
    ) -> u32 {
        let v = if add {
            win_ld(mem, w, step) + win_ld(mem, w, field)
        } else {
            win_ld(mem, w, field) - win_ld(mem, w, step)
        };
        let frames = mem.read::<i16>(w.wrapping_add(0xc)).wrapping_sub(1);
        mem.write(w.wrapping_add(0xc), frames);
        mem.write(w.wrapping_add(field), x87::store(v));
        if frames > 0 {
            return 0;
        }
        if reload {
            let r = mem.read::<u16>(w.wrapping_add(0xe));
            mem.write(w.wrapping_add(0xc), r);
        }
        win_next(mem, w, state, slot)
    }

    /// The last handler of a close: inactive (`+0`, and `+8` when
    /// `clear_state`), the idle handler (`0x00907e50`). Returns 1.
    fn win_finish(mem: &mut impl Mem, w: u32, clear_state: bool) -> u32 {
        mem.write(w, 0u16);
        if clear_state {
            mem.write(w.wrapping_add(8), 0u16);
        }
        let idle = mem.rebase(super::WINDOW_IDLE);
        mem.write(w.wrapping_add(0x74), idle);
        1
    }

    /// The drawn rectangle from the rectangle (x0, y0, `a`, `b`).
    fn win_drawn(mem: &mut impl Mem, w: u32, a: u32, b: u32) {
        win_copy(mem, w, 0x48, 0x28);
        win_copy(mem, w, 0x4c, 0x2c);
        win_copy(mem, w, 0x50, a);
        win_copy(mem, w, 0x54, b);
    }

    /// `yiWindowGrowInit(w)` (`0x00907cc0`, kind 0): progress 0, drawn
    /// x1 = x0 (a zero-width rectangle).
    pub fn yi_window_grow_init(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(6), 0u16);
        win_drawn(mem, w, 0x28, 0x34);
        0
    }

    /// The drawn right edge of a growing window: `x0 + height * progress
    /// / 4096` (`f32`).
    fn win_grow_edge(mem: &mut impl Mem, w: u32) -> f64 {
        let p = i32::from(mem.read::<i16>(w.wrapping_add(0x10)))
            * i32::from(mem.read::<i16>(w.wrapping_add(6)));
        #[allow(clippy::cast_precision_loss)] // fild, then fstp m32 rounds
        let f = f64::from((p / 4096) as f32);
        let t = x87::store(f + win_ld(mem, w, 0x28));
        mem.write(w.wrapping_add(0x50), t);
        f64::from(f32::from_bits(t))
    }

    /// `yiWindowGrowOpen(w)` (`0x00907cf0`): progress + 0x2aa (at most
    /// 0x1000); the drawn x1 from it; open (state 2, the handler
    /// `0x00907d70`) once it reaches x1.
    pub fn yi_window_grow_open(mem: &mut impl Mem, w: u32) -> u32 {
        let p = mem.read::<i16>(w.wrapping_add(6)).wrapping_add(0x2aa);
        mem.write(w.wrapping_add(6), if p >= 0x1000 { 0x1000 } else { p });
        let t = win_grow_edge(mem, w);
        if win_ld(mem, w, 0x30) <= t {
            mem.write(w.wrapping_add(8), 2u16);
            let h = mem.rebase(super::WINDOW_OPEN_IDLE);
            mem.write(w.wrapping_add(0x74), h);
            return 1;
        }
        0
    }

    /// `yiWindowGrowClose(w)` (`0x00906fc0`): progress - 0x2aa (at least
    /// 0); closed (state 4, the handler `yiWindowFinishAll`) once the
    /// drawn x1 is back at x0.
    pub fn yi_window_grow_close(mem: &mut impl Mem, w: u32) -> u32 {
        let p = mem.read::<i16>(w.wrapping_add(6)).wrapping_add(-0x2aa);
        mem.write(w.wrapping_add(6), if p > 0 { p } else { 0 });
        let t = win_grow_edge(mem, w);
        if t <= win_ld(mem, w, 0x28) {
            mem.write(w.wrapping_add(8), 4u16);
            let h = mem.rebase(super::WINDOW_FINISH_ALL);
            mem.write(w.wrapping_add(0x74), h);
            return 1;
        }
        0
    }

    /// `yiWindowFinishAll(w)` (`0x00907040`): [`win_finish`], state 0.
    pub fn yi_window_finish_all(mem: &mut impl Mem, w: u32) -> u32 {
        win_finish(mem, w, true)
    }

    /// `yiWindowFinish(w)` (`0x009071f0`, `0x00907370`, `0x009075b0`,
    /// `0x009077e0`, `0x00907a20`: the same code): [`win_finish`].
    pub fn yi_window_finish(mem: &mut impl Mem, w: u32) -> u32 {
        win_finish(mem, w, false)
    }

    /// `yiWindowPopInit(w)` (`0x00907160`, kinds 1 and 2): no frames, the
    /// drawn rectangle the rectangle.
    pub fn yi_window_pop_init(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(0xc), 0u32);
        win_drawn(mem, w, 0x30, 0x34);
        0
    }

    /// `yiWindowPopOpen(w)` (`0x00907190`): open at once (state 2, slot
    /// `+0x68`).
    pub fn yi_window_pop_open(mem: &mut impl Mem, w: u32) -> u32 {
        win_next(mem, w, 2, 0x68)
    }

    /// `yiWindowPopClose(w)` (`0x009071c0`): one frame (`+0xc`) less;
    /// closed (state 4, slot `+0x70`) when none is left.
    pub fn yi_window_pop_close(mem: &mut impl Mem, w: u32) -> u32 {
        let f = mem.read::<i16>(w.wrapping_add(0xc)).wrapping_sub(1);
        mem.write(w.wrapping_add(0xc), f);
        if f > 0 {
            return 0;
        }
        win_next(mem, w, 4, 0x70)
    }

    /// `yiWindowSlideDownInit(w)` (`0x00907450`, kind 3): the window drops
    /// from y = 416 (the screen's bottom) to its rectangle: from `(x0, 0,
    /// x1, 0)`, drawn `(x0, 0, x1, 0)`, 6 frames of `(416 - 0) / 6`, y1
    /// 416.
    pub fn yi_window_slide_down_init(mem: &mut impl Mem, w: u32) -> u32 {
        let k = win_const(mem, super::WINDOW_HEIGHT_F32);
        mem.write(w.wrapping_add(0x34), k);
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        for off in [0x2cu32, 0x24, 0x1c] {
            mem.write(w.wrapping_add(off), 0u32);
        }
        win_copy(mem, w, 0x18, 0x28);
        win_copy(mem, w, 0x20, 0x30);
        win_copy(mem, w, 0x50, 0x20);
        win_copy(mem, w, 0x48, 0x18);
        mem.write(w.wrapping_add(0x4c), 0u32);
        mem.write(w.wrapping_add(0x54), 0u32);
        let s = (win_f64(mem, super::WINDOW_HEIGHT) - win_f64(mem, super::WINDOW_ZERO))
            / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x3c), x87::store(s));
        0
    }

    /// `yiWindowSlideDownOpen(w)` (`0x009072e0`): the drawn y1 (`+0x54`)
    /// grows by the step; then open.
    pub fn yi_window_slide_down_open(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x54, 0x3c, true, true, 2, 0x68)
    }

    /// `yiWindowSlideDownClose(w)` (`0x00907330`): the drawn y1 shrinks;
    /// then closed.
    pub fn yi_window_slide_down_close(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x54, 0x3c, false, false, 4, 0x70)
    }

    /// `yiWindowSlideUpInitHere(w)` (`0x009074b0`, kind 4): from the
    /// bottom edge (416) up to the rectangle moved so its bottom is at
    /// 416: y0 = y0 - (416 - y1), 6 frames.
    pub fn yi_window_slide_up_init_here(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        let h = win_f64(mem, super::WINDOW_HEIGHT);
        let t = x87::store(win_ld(mem, w, 0x2c) - (h - win_ld(mem, w, 0x34)));
        mem.write(w.wrapping_add(0x2c), t);
        let k = win_const(mem, super::WINDOW_HEIGHT_F32);
        mem.write(w.wrapping_add(0x24), k);
        mem.write(w.wrapping_add(0x1c), k);
        slide_up_rest(mem, w, k, f64::from(f32::from_bits(t)));
        0
    }

    /// The common tail of the kind 4/5 inits: from `(x0, 416, x1, 416)`,
    /// drawn the same, the step `(y1 - y0) / 6`.
    fn slide_up_rest(mem: &mut impl Mem, w: u32, k: u32, y0: f64) {
        win_copy(mem, w, 0x18, 0x28);
        win_copy(mem, w, 0x20, 0x30);
        win_copy(mem, w, 0x50, 0x20);
        win_copy(mem, w, 0x48, 0x18);
        mem.write(w.wrapping_add(0x4c), k);
        mem.write(w.wrapping_add(0x54), k);
        let s = (win_ld(mem, w, 0x34) - y0) / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x3c), x87::store(s));
    }

    /// `yiWindowSlideUpInit(w)` (`0x00907690`, kind 5): the rectangle
    /// `(x0, 0, x1, 416)` slides up from the bottom edge.
    pub fn yi_window_slide_up_init(mem: &mut impl Mem, w: u32) -> u32 {
        let k = win_const(mem, super::WINDOW_HEIGHT_F32);
        mem.write(w.wrapping_add(0x34), k);
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        mem.write(w.wrapping_add(0x2c), 0u32);
        mem.write(w.wrapping_add(0x24), k);
        mem.write(w.wrapping_add(0x1c), k);
        win_copy(mem, w, 0x18, 0x28);
        win_copy(mem, w, 0x20, 0x30);
        win_copy(mem, w, 0x50, 0x20);
        win_copy(mem, w, 0x48, 0x18);
        mem.write(w.wrapping_add(0x4c), k);
        mem.write(w.wrapping_add(0x54), k);
        let s = (win_f64(mem, super::WINDOW_HEIGHT) - win_f64(mem, super::WINDOW_ZERO))
            / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x3c), x87::store(s));
        0
    }

    /// `yiWindowSlideUpOpen(w)` (`0x00907520`): the drawn y0 (`+0x4c`)
    /// rises by the step; then open.
    pub fn yi_window_slide_up_open(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x4c, 0x3c, false, true, 2, 0x68)
    }

    /// `yiWindowSlideUpClose(w)` (`0x00907570`): the drawn y0 falls; then
    /// closed.
    pub fn yi_window_slide_up_close(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x4c, 0x3c, true, false, 4, 0x70)
    }

    /// `yiWindowSlideRightInitHere(w)` (`0x009076f0`, kind 6): x1 = x1 +
    /// x0, from the left edge (0), 6 frames of `(x1 - x0) / 6`.
    pub fn yi_window_slide_right_init_here(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        let t = x87::store(win_ld(mem, w, 0x30) + win_ld(mem, w, 0x28));
        mem.write(w.wrapping_add(0x30), t);
        mem.write(w.wrapping_add(0x20), 0u32);
        mem.write(w.wrapping_add(0x18), 0u32);
        side_rest(mem, w, 0);
        let s = (f64::from(f32::from_bits(t)) - win_ld(mem, w, 0x28))
            / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x38), x87::store(s));
        0
    }

    /// The common middle of the kind 6..=9 inits: from y0/y1 of the
    /// rectangle, drawn `(edge, y0, edge, y1)`.
    fn side_rest(mem: &mut impl Mem, w: u32, edge: u32) {
        win_copy(mem, w, 0x1c, 0x2c);
        win_copy(mem, w, 0x24, 0x34);
        win_copy(mem, w, 0x54, 0x24);
        mem.write(w.wrapping_add(0x48), edge);
        mem.write(w.wrapping_add(0x50), edge);
        win_copy(mem, w, 0x4c, 0x1c);
    }

    /// `yiWindowSlideRightInit(w)` (`0x009078c0`, kind 7): the rectangle
    /// `(0, y0, 512, y1)` slides in from the left edge.
    pub fn yi_window_slide_right_init(mem: &mut impl Mem, w: u32) -> u32 {
        let k = win_const(mem, super::WINDOW_WIDTH_F32);
        mem.write(w.wrapping_add(0x30), k);
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        for off in [0x28u32, 0x20, 0x18] {
            mem.write(w.wrapping_add(off), 0u32);
        }
        side_rest(mem, w, 0);
        let s = (win_f64(mem, super::WINDOW_WIDTH) - win_f64(mem, super::WINDOW_ZERO))
            / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x38), x87::store(s));
        0
    }

    /// `yiWindowSlideRightOpen(w)` (`0x00907750`): the drawn x1 (`+0x50`)
    /// grows by the step; then open.
    pub fn yi_window_slide_right_open(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x50, 0x38, true, true, 2, 0x68)
    }

    /// `yiWindowSlideRightClose(w)` (`0x009077a0`): the drawn x1 shrinks;
    /// then closed.
    pub fn yi_window_slide_right_close(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x50, 0x38, false, false, 4, 0x70)
    }

    /// `yiWindowSlideLeftInitHere(w)` (`0x00907920`, kind 8): x0 = x0 -
    /// (512 - x1), from the right edge (512), 6 frames of `(x1 - x0) / 6`.
    pub fn yi_window_slide_left_init_here(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        let width = win_f64(mem, super::WINDOW_WIDTH);
        let t = x87::store(win_ld(mem, w, 0x28) - (width - win_ld(mem, w, 0x30)));
        mem.write(w.wrapping_add(0x28), t);
        let k = win_const(mem, super::WINDOW_WIDTH_F32);
        mem.write(w.wrapping_add(0x20), k);
        mem.write(w.wrapping_add(0x18), k);
        side_rest(mem, w, k);
        let s = (win_ld(mem, w, 0x30) - f64::from(f32::from_bits(t)))
            / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x38), x87::store(s));
        0
    }

    /// `yiWindowSlideLeftInit(w)` (`0x00907b00`, kind 9): the rectangle
    /// `(0, y0, 512, y1)` slides in from the right edge.
    pub fn yi_window_slide_left_init(mem: &mut impl Mem, w: u32) -> u32 {
        let k = win_const(mem, super::WINDOW_WIDTH_F32);
        mem.write(w.wrapping_add(0x30), k);
        mem.write(w.wrapping_add(0xc), 0x0006_0006u32);
        mem.write(w.wrapping_add(0x28), 0u32);
        mem.write(w.wrapping_add(0x20), k);
        mem.write(w.wrapping_add(0x18), k);
        side_rest(mem, w, k);
        let s = (win_f64(mem, super::WINDOW_WIDTH) - win_f64(mem, super::WINDOW_ZERO))
            / win_f64(mem, super::WINDOW_FRAMES);
        mem.write(w.wrapping_add(0x38), x87::store(s));
        0
    }

    /// `yiWindowSlideLeftOpen(w)` (`0x00907990`): the drawn x0 (`+0x48`)
    /// moves left by the step; then open.
    pub fn yi_window_slide_left_open(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x48, 0x38, false, true, 2, 0x68)
    }

    /// `yiWindowSlideLeftClose(w)` (`0x009079e0`): the drawn x0 moves
    /// back; then closed.
    pub fn yi_window_slide_left_close(mem: &mut impl Mem, w: u32) -> u32 {
        win_step(mem, w, 0x48, 0x38, true, false, 4, 0x70)
    }

    /// `yiWindowBarInit(w)` (`0x00907b60`, kind 10): the drawn rectangle
    /// `(0, 8, 512, 28)` and colour `0x80808080`.
    pub fn yi_window_bar_init(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(0x48), 0u32);
        mem.write(w.wrapping_add(0x58), 0x8080_8080u32);
        for (off, c) in [
            (0x4cu32, super::WINDOW_BAR_TOP),
            (0x50, super::WINDOW_WIDTH_F32),
            (0x54, super::WINDOW_BAR_BOTTOM),
        ] {
            let v = win_const(mem, c);
            mem.write(w.wrapping_add(off), v);
        }
        0
    }

    /// `yiWindowBarOpen(w)` (`0x00907ba0`): open at once (state 2, the
    /// handler `0x00907d70`).
    pub fn yi_window_bar_open(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(8), 2u16);
        let h = mem.rebase(super::WINDOW_OPEN_IDLE);
        mem.write(w.wrapping_add(0x74), h);
        1
    }

    /// `yiWindowBarClose(w)` (`0x00907bd0`): closed at once (state 4, the
    /// handler `yiWindowFinishAll`).
    pub fn yi_window_bar_close(mem: &mut impl Mem, w: u32) -> u32 {
        mem.write(w.wrapping_add(8), 4u16);
        let h = mem.rebase(super::WINDOW_FINISH_ALL);
        mem.write(w.wrapping_add(0x74), h);
        1
    }
}
