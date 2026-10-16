//! The character module's own settings: drawing switches, the shadow
//! colour and direction, the camera and screen matrices it draws with,
//! cloth editing values, mouth and talk settings, the effect packet
//! writer. All globals of `FFX.exe`.

/// `Ch_GuiSetMouthTh`'s threshold: clamped to 0..=200.
#[must_use]
pub const fn mouth_threshold(v: i32) -> i32 {
    if v < 0 {
        0
    } else if v > 200 {
        200
    } else {
        v
    }
}

/// The originals, on the globals.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::mouth_threshold;

    /// `i8` cloth parameter editing (`Ch_ClothSetParaEdit`).
    pub const CLOTH_PARA_EDIT: Global<i8> = Global::new(0x012f_bc54);
    /// `f32` cloth y (`Ch_ClothSetY`).
    pub const CLOTH_Y: Global<f32> = Global::new(0x012f_bc58);
    /// `f32` cloth xz (`Ch_ClothSetXZ`).
    pub const CLOTH_XZ: Global<f32> = Global::new(0x012f_bc5c);
    /// `u8` set by `Ch_MemMoveSetTrigger`.
    pub const MEM_MOVE_TRIGGER: Global<u8> = Global::new(0x012f_bda0);
    /// `i8` drawing switch (`Ch_DrawSetParaEdit`).
    pub const DRAW_PARA_EDIT: Global<i8> = Global::new(0x0130_07d9);
    /// `i8` forced specular (`Ch_DrawSetFourceSpec`).
    pub const DRAW_FOURCE_SPEC: Global<i8> = Global::new(0x0130_07da);
    /// `i8` specular off (`Ch_DrawSetDisableSpec`).
    pub const DRAW_DISABLE_SPEC: Global<i8> = Global::new(0x0130_07db);
    /// `i8` (`Ch_EnableGetTexture`).
    pub const ENABLE_TEXTURE: Global<i8> = Global::new(0x0130_07e8);
    /// The camera matrix (64 bytes, `Ch_SysGetCameraMatrixPtr`).
    pub const CAMERA_MATRIX: Global<[u32; 16]> = Global::new(0x0130_08f8);
    /// The screen matrix (64 bytes, `Ch_SysGetScreenMatrixPtr`).
    pub const SCREEN_MATRIX: Global<[u32; 16]> = Global::new(0x0130_0938);
    /// `u8` shadow mode (`Ch_DrawSetShadowMode`).
    pub const SHADOW_MODE: Global<u8> = Global::new(0x0130_08bd);
    /// `u8` × 3 shadow colour (`Ch_DrawSetShadowColor`).
    pub const SHADOW_COLOR: Global<[u8; 3]> = Global::new(0x0130_08bf);
    /// `u32` × 2 shadow direction (`Ch_DrawSetShadowDir`).
    pub const SHADOW_DIR: Global<[u32; 2]> = Global::new(0x0130_08c4);
    /// `u8` × 2 talk settings (`Ch_GuiSetTalk`).
    pub const TALK: Global<[u8; 2]> = Global::new(0x0130_08d9);
    /// `i8` motion event dump (`Ch_SetDumpMotEv`).
    pub const DUMP_MOT_EV: Global<i8> = Global::new(0x0130_08db);
    /// `u32` × n by index (`Ch_SetFuka`).
    pub const FUKA: Global<u32> = Global::new(0x0130_09b8);
    /// `u32`, 0 or 1 (`Ch_GuiSetCalcNormAll`).
    pub const GUI_CALC_NORMAL: Global<u32> = Global::new(0x0130_09e0);
    /// `u32` (`Ch_MotionSetHalf`).
    pub const MOTION_HALF: Global<u32> = Global::new(0x0130_17f8);
    /// `i16` mouth threshold (`Ch_GuiSetMouthTh`).
    pub const MOUTH_TH: Global<i16> = Global::new(0x00c4_9718);
    /// Where the effect packet writer is, or 0 when it only measures
    /// (`Ch_EffectPacketStoreEND`).
    pub const PACKET_PTR: Global<u32> = Global::new(0x0130_3658);
    /// Bytes the effect packet would take (`Ch_EffectPacketGetSize`).
    pub const PACKET_SIZE: Global<u32> = Global::new(0x0130_365c);
    /// `u16` packet material (`Ch_EffectPacketStoreMAT`).
    pub const PACKET_MAT: Global<u16> = Global::new(0x0130_3674);
    /// `u16` packet program (`Ch_EffectPacketStorePRG`).
    pub const PACKET_PRG: Global<u16> = Global::new(0x0130_367c);

    /// `Ch_*Get*` of an `i8` setting, sign-extended.
    pub fn get_i8(mem: &impl Mem, g: Global<i8>) -> i32 {
        i32::from(g.get(mem))
    }

    /// `Ch_*Set*` of a byte setting: the low byte of `v`.
    pub fn set_i8(mem: &mut impl Mem, g: Global<i8>, v: u32) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // the low byte
        g.set(mem, v as u8 as i8);
    }

    /// `Ch_ClothSetXZ(v)` (`0x00824a40`) and `Ch_ClothSetY(v)`
    /// (`0x00824a50`).
    pub fn set_f32(mem: &mut impl Mem, g: Global<f32>, v: f32) {
        g.set(mem, crate::x87::load(v));
    }

    /// `Ch_MemMoveSetTrigger()` (`0x00827250`).
    pub fn ch_mem_move_set_trigger(mem: &mut impl Mem) {
        MEM_MOVE_TRIGGER.set(mem, 1);
    }

    /// `Ch_DrawGetShadowColor(&r, &g, &b)` (`0x0082ebf0`): null outputs
    /// are skipped.
    pub fn ch_draw_get_shadow_color(mem: &mut impl Mem, out: [u32; 3]) {
        let c = SHADOW_COLOR.get(mem);
        for (o, v) in out.into_iter().zip(c) {
            if o != 0 {
                mem.write(o, u32::from(v));
            }
        }
    }

    /// `Ch_DrawSetShadowColor(r, g, b)` (`0x00830100`).
    pub fn ch_draw_set_shadow_color(mem: &mut impl Mem, [r, g, b]: [u32; 3]) {
        #[allow(clippy::cast_possible_truncation)] // low bytes
        SHADOW_COLOR.set(mem, [r as u8, g as u8, b as u8]);
    }

    /// `Ch_DrawGetShadowDir(&a, &b)` (`0x0082ec30`): null outputs are
    /// skipped.
    pub fn ch_draw_get_shadow_dir(mem: &mut impl Mem, out: [u32; 2]) {
        let d = SHADOW_DIR.get(mem);
        for (o, v) in out.into_iter().zip(d) {
            if o != 0 {
                mem.write(o, v);
            }
        }
    }

    /// `Ch_DrawSetShadowDir(a, b)` (`0x00830120`).
    pub fn ch_draw_set_shadow_dir(mem: &mut impl Mem, d: [u32; 2]) {
        SHADOW_DIR.set(mem, d);
    }

    /// `Ch_DrawSetShadowMode(mode)` (`0x00830140`).
    pub fn ch_draw_set_shadow_mode(mem: &mut impl Mem, mode: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        SHADOW_MODE.set(mem, mode as u8);
    }

    /// `Ch_GuiGetCalcNormal()` (`0x00834060`).
    pub fn ch_gui_get_calc_normal(mem: &impl Mem) -> u32 {
        GUI_CALC_NORMAL.get(mem)
    }

    /// `Ch_GuiSetCalcNormAll(on)` (`0x00834090`).
    pub fn ch_gui_set_calc_norm_all(mem: &mut impl Mem, on: u32) {
        GUI_CALC_NORMAL.set(mem, on & 1);
    }

    /// `Ch_GuiSetMouthTh(v)` (`0x008340a0`): clamped to 0..=200; returns
    /// that (left in `eax`).
    pub fn ch_gui_set_mouth_th(mem: &mut impl Mem, v: i32) -> i32 {
        let t = mouth_threshold(v);
        #[allow(clippy::cast_possible_truncation)] // 0..=200
        MOUTH_TH.set(mem, t as i16);
        t
    }

    /// `Ch_GuiSetTalk(a, b)` (`0x008340d0`).
    pub fn ch_gui_set_talk(mem: &mut impl Mem, a: u32, b: u32) {
        #[allow(clippy::cast_possible_truncation)] // low bytes
        TALK.set(mem, [a as u8, b as u8]);
    }

    /// `Ch_GetFuka(i)` (`0x00833f60`); the index is not checked.
    pub fn ch_get_fuka(mem: &impl Mem, i: u32) -> u32 {
        FUKA.ptr(mem).at(i).get(mem)
    }

    /// `Ch_SetFuka(i, v)` (`0x008357d0`).
    pub fn ch_set_fuka(mem: &mut impl Mem, i: u32, v: u32) {
        let p = FUKA.ptr(mem).at(i);
        p.set(mem, v);
    }

    /// `Ch_SysGetScreenMatrixPtr()` (`0x00835b70`).
    pub fn ch_sys_get_screen_matrix_ptr(mem: &impl Mem) -> u32 {
        SCREEN_MATRIX.ptr(mem).addr
    }

    /// `Ch_SysGetCameraMatrixPtr()` (`0x00835ba0`).
    pub fn ch_sys_get_camera_matrix_ptr(mem: &impl Mem) -> u32 {
        CAMERA_MATRIX.ptr(mem).addr
    }

    /// `Ch_SysSetScreenMatrix(m)` (`0x00835e90`) and
    /// `Ch_SysSetCameraMatrix(m)` (`0x00835eb0`): 64 bytes from `m`.
    pub fn set_matrix(mem: &mut impl Mem, g: Global<[u32; 16]>, m: u32) {
        let v: [u32; 16] = mem.read(m);
        g.set(mem, v);
    }

    /// `Ch_MotionGetHalf()` (`0x00839870`).
    pub fn ch_motion_get_half(mem: &impl Mem) -> u32 {
        MOTION_HALF.get(mem)
    }

    /// `Ch_MotionSetHalf(v)` (`0x00839950`).
    pub fn ch_motion_set_half(mem: &mut impl Mem, v: u32) {
        MOTION_HALF.set(mem, v);
    }

    /// `Ch_EffectPacketGetSize()` (`0x00845700`).
    pub fn ch_effect_packet_get_size(mem: &impl Mem) -> u32 {
        PACKET_SIZE.get(mem)
    }

    /// `Ch_EffectPacketStoreEND()` (`0x008457d0`): the 16-byte end tag
    /// (`0xff00`, words 2 and 3 zero) when writing, else only its size.
    pub fn ch_effect_packet_store_end(mem: &mut impl Mem) {
        let p = PACKET_PTR.get(mem);
        if p == 0 {
            let s = PACKET_SIZE.get(mem);
            PACKET_SIZE.set(mem, s.wrapping_add(16));
            return;
        }
        mem.write(p, 0xff00u32);
        mem.write(p + 8, 0u32);
        mem.write(p + 0xc, 0u32);
        PACKET_PTR.set(mem, p.wrapping_add(16));
    }

    /// `Ch_EffectPacketStoreMAT(v)` (`0x00845800`) and
    /// `Ch_EffectPacketStorePRG(v)` (`0x00845810`).
    pub fn set_u16(mem: &mut impl Mem, g: Global<u16>, v: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low half
        g.set(mem, v as u16);
    }

    /// `u8` icon drawing (`Ch_SetDrawIcon`).
    pub const DRAW_ICON: Global<u8> = Global::new(0x0130_07e9);
    /// `u8` second icon drawing (`Ch_SetDrawIcon2`).
    pub const DRAW_ICON2: Global<u8> = Global::new(0x0130_07ea);
    /// `u32` set by `Ch_SetAllocateErr`.
    pub const ALLOCATE_ERR: Global<u32> = Global::new(0x012f_bd98);
    /// `i8` switch set by `Ch_SysSetSwitch7d8` (bit 0).
    pub const SWITCH_7D8: Global<i8> = Global::new(0x0130_07d8);
    /// `u32` set by `Ch_SysSet7e4`.
    pub const SYS_7E4: Global<u32> = Global::new(0x0130_07e4);
    /// `u32` set by `Ch_SysSet6e0`.
    pub const SYS_6E0: Global<u32> = Global::new(0x00c4_96e0);
    /// `u32` cleared by `Ch_FunctionHistoryClear`.
    pub const HISTORY_9E8: Global<u32> = Global::new(0x0130_09e8);

    /// `Ch_SetDrawIcon(on)` (`0x00830c70`) and `Ch_SetDrawIcon2(on)`
    /// (`0x00830c80`): bit 0.
    pub fn set_bit0(mem: &mut impl Mem, g: Global<u8>, on: u32) {
        #[allow(clippy::cast_possible_truncation)] // bit 0
        g.set(mem, (on & 1) as u8);
    }

    /// `Ch_SetAllocateErr()` (`0x008285d0`).
    pub fn ch_set_allocate_err(mem: &mut impl Mem) {
        ALLOCATE_ERR.set(mem, 1);
    }

    /// `Ch_SysSetSwitch7d8(on)` (`0x008317b0`): bit 0; returns the old
    /// value, sign-extended.
    pub fn ch_sys_set_switch_7d8(mem: &mut impl Mem, on: u32) -> i32 {
        let old = i32::from(SWITCH_7D8.get(mem));
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // bit 0
        SWITCH_7D8.set(mem, (on & 1) as i8);
        old
    }

    /// The tag of the current effect packet block (`Ch_EffectPacketStorePoly`).
    pub const PACKET_HEAD: Global<u32> = Global::new(0x0130_3654);
    /// Polygons measured while only measuring.
    pub const PACKET_POLYS: Global<u32> = Global::new(0x0130_3660);
    /// The largest x and y measured (`i32`).
    pub const PACKET_MAX: Global<[i32; 2]> = Global::new(0x0130_3664);
    /// `u16` texture index, MAT, PRG and one more state, each followed by
    /// the value the current block was opened with (`0x0130366c`..`8a`,
    /// 4 bytes apart).
    pub const PACKET_STATE: Global<u16> = Global::new(0x0130_366c);
    /// The texture of the current block (a texture list entry, or 0).
    pub const PACKET_TEX: Global<u32> = Global::new(0x0130_368c);

    /// `u16 * 4096 / (size * 16)` truncated, as `fistp` in chop mode; the
    /// low half stored.
    fn uv(v: u16, size: u16) -> u16 {
        let q = f64::from(v) * 4096.0 / (f64::from(size) * 16.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // in range
        let r = if q.is_finite() {
            q.trunc() as i64 as u32
        } else {
            0x8000_0000
        };
        #[allow(clippy::cast_possible_truncation)] // the low half
        let r = r as u16;
        r
    }

    /// `Ch_EffectPacketStorePoly(a, b, c)` (`0x00845820`): one triangle
    /// (vertices of `u16` x, y, u, v). A changed texture/MAT/PRG state
    /// opens a new 32-byte block (tag `0x1000`, the texture's registers,
    /// or `0x8044` without one). A triangle takes 28 bytes: x's at `+0`,
    /// y's at `+0x14`, with a texture u, v in 4.12 of its size at `+8`.
    /// While only measuring, the sizes, a polygon count and the largest x
    /// and y are kept instead.
    pub fn ch_effect_packet_store_poly(mem: &mut impl Mem, a: u32, b: u32, c: u32) {
        let st = PACKET_STATE.ptr(mem).addr;
        let changed =
            (0..4).any(|k| mem.read::<u16>(st + 8 * k) != mem.read::<u16>(st + 8 * k + 4));
        let mut size_add = 0u32;
        let mut p = PACKET_PTR.get(mem);
        if changed {
            for k in 0..4 {
                let v: u16 = mem.read(st + 8 * k);
                mem.write(st + 8 * k + 4, v);
            }
            if p == 0 {
                size_add = 0x20;
            } else {
                let list = crate::texanim::bridge::TEX0_LIST.get(mem);
                let tex = if list == 0 {
                    0
                } else {
                    let i = i32::from(mem.read::<i16>(st));
                    list.wrapping_add(i.cast_unsigned() << 5)
                };
                PACKET_TEX.set(mem, tex);
                PACKET_HEAD.set(mem, p);
                mem.write(p, [0x1000u32, 0x7fc0_c0c0]);
                if tex == 0 {
                    mem.write(p + 0xc, 0u32);
                    mem.write(p + 8, 0u32);
                    mem.write(p + 0x10, 0x8044u16);
                } else {
                    let r: [u32; 2] = mem.read(tex + 0x14);
                    mem.write(p + 8, r);
                    let v: [u8; 2] = mem.read(tex + 0x11);
                    mem.write(p + 0x10, v);
                }
                p += 0x20;
                PACKET_PTR.set(mem, p);
            }
        }
        if p == 0 {
            let s = PACKET_SIZE.get(mem);
            PACKET_SIZE.set(mem, s.wrapping_add(size_add).wrapping_add(0x1c));
            let mut max = PACKET_MAX.get(mem);
            for (k, m) in (0u32..).zip(max.iter_mut()) {
                let v = [a, b, c].map(|q| i32::from(mem.read::<u16>(q + 2 * k)));
                let top = v[0].max(v[1]).max(v[2]);
                if top > *m {
                    *m = top;
                }
            }
            PACKET_MAX.set(mem, max);
            let n = PACKET_POLYS.get(mem);
            PACKET_POLYS.set(mem, n.wrapping_add(1));
            return;
        }
        let tex = PACKET_TEX.get(mem);
        mem.write(p + 6, 0u16);
        let v = [a, b, c].map(|q| mem.read::<[u16; 4]>(q));
        mem.write(p, [v[0][0], v[1][0], v[2][0]]);
        mem.write(p + 0x14, [v[0][1], v[1][1], v[2][1]]);
        if tex != 0 {
            let (w, h): (u16, u16) = (mem.read(tex + 8), mem.read(tex + 0xa));
            for (k, q) in (0u32..).zip(v) {
                mem.write(p + 8 + 4 * k, [uv(q[2], w), uv(q[3], h)]);
            }
        }
        let head = PACKET_HEAD.get(mem);
        let n: u16 = mem.read(head + 2);
        mem.write(head + 2, n.wrapping_add(1));
        PACKET_PTR.set(mem, p + 0x1c);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_clamp() {
        assert_eq!(mouth_threshold(-1), 0);
        assert_eq!(mouth_threshold(201), 200);
        assert_eq!(mouth_threshold(77), 77);
    }
}
