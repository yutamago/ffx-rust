//! More of the character object: cloth, binding to another character's
//! element, elements (bones), shade, point lights, shadow, mouth and neck,
//! optional positions, motion state ([`crate::layout`]). All cdecl with
//! the character first.

/// `Ch_ClothSetFast`'s settle frames for a case of its per-id table, and
/// whether `CLOTH_81C` becomes 2 (case 1). Cases beyond 4 do not occur
/// (the original's jump table has five).
#[must_use]
pub const fn cloth_fast(case: u8) -> (u16, bool) {
    match case {
        0 => (10, false),
        1 => (10, true),
        2 => (15, false),
        3 => (20, false),
        _ => (1, false),
    }
}

/// The accessors where the game keeps the characters.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::cloth_fast;
    use crate::chr::bridge::{
        blk, ch_en_body_hit, ch_en_map_id, ch_set_thickness, en_flag, f, flag, put_f, set_flag,
    };
    use crate::handle::bridge::{CHR_ARRAY, CHR_COUNT};
    use crate::layout as l;
    use crate::x87::{self, Reg};

    /// `Ch_ClothSetFast`'s case by `id - 1` (`0x7a` bytes in its code).
    pub const CLOTH_FAST_CASES: Global<u8> = Global::new(0x0082_49b4);
    /// `Ch_ClothSetFast`'s case 1 value (2.0).
    pub const CLOTH_FAST_81C: Global<f32> = Global::new(0x00b9_2268);
    /// `Ch_ClothMeshMappingOffset`'s y offset for character `0x65`.
    pub const CLOTH_OFFSET_Y_0065: Global<f32> = Global::new(0x00b5_91ac);
    /// `Ch_MouthInit`'s value (1234.0).
    pub const MOUTH_INIT: Global<f32> = Global::new(0x00b5_99ec);
    /// `Ch_NeckSetSpeed`'s default (6.0).
    pub const NECK_SPEED_DEFAULT: Global<f32> = Global::new(0x00b9_2308);
    /// `i8`: `Ch_DrawCalcShade` multiplies the shade by the light value.
    pub const SHADE_BY_LIGHT: Global<i8> = Global::new(0x00c4_96ea);

    fn u8_at(mem: &impl Mem, at: u32) -> i32 {
        i32::from(mem.read::<i8>(at))
    }

    // ---- cloth

    /// `Ch_ClothEnableGrav(chr, on)` (`0x008245f0`): returns the bit that
    /// changed.
    pub fn ch_cloth_enable_grav(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        set_flag(mem, chr, l::DIRTY, l::D_CLOTH_GRAV, on)
    }

    /// `Ch_ClothMeshMappingOffset(chr)` (`0x00824620`): offset 0, except
    /// character `0x65`'s y.
    pub fn ch_cloth_mesh_mapping_offset(mem: &mut impl Mem, chr: u32) {
        for i in 0..3 {
            mem.write(chr + l::CLOTH_OFFSET + 4 * i, 0u32);
        }
        if mem.read::<u16>(chr + l::ID) == 0x65 {
            let y = CLOTH_OFFSET_Y_0065.get(mem);
            put_f(mem, chr + l::CLOTH_OFFSET + 4, y);
        }
    }

    /// `Ch_ClothSetFast(chr)` (`0x00824920`): settle frames by id.
    pub fn ch_cloth_set_fast(mem: &mut impl Mem, chr: u32) {
        mem.write(chr + l::CLOTH_81C, 0.0f32);
        mem.write(chr + l::CLOTH_810, 1u16);
        let i = u32::from(mem.read::<u16>(chr + l::ID)).wrapping_sub(1);
        let case = if i > 0x79 {
            4
        } else {
            CLOTH_FAST_CASES.ptr(mem).at(i).get(mem)
        };
        let (frames, two) = cloth_fast(case);
        if two {
            let v = CLOTH_FAST_81C.get(mem);
            put_f(mem, chr + l::CLOTH_81C, v);
        }
        mem.write(chr + l::CLOTH_FRAMES, frames);
    }

    /// `Ch_ClothSetEnable(chr, on)` (`0x0082a930`).
    pub fn ch_cloth_set_enable(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        set_flag(mem, chr, l::FLAGS, l::F_CLOTH, on)
    }

    // ---- flags and position

    /// `Ch_DisableBodyHit(chr, off)` (`0x0082a950`):
    /// `Ch_EnBodyHit(chr, 1 - (off & 1))`.
    pub fn ch_disable_body_hit(mem: &mut impl Mem, chr: u32, off: u32) -> u32 {
        ch_en_body_hit(mem, chr, 1 - (off & 1))
    }

    /// `Ch_DisableID(chr, off)` (`0x0082a980`): `Ch_EnMapID(chr, 1 - (off & 1))`.
    pub fn ch_disable_id(mem: &mut impl Mem, chr: u32, off: u32) -> u32 {
        ch_en_map_id(mem, chr, 1 - (off & 1))
    }

    /// `Ch_AddY(chr, dy)` (`0x0082a8b0`).
    pub fn ch_add_y(mem: &mut impl Mem, chr: u32, dy: f32) {
        let v = x87::add(dy, f(mem, chr + l::POS + 4));
        mem.write(chr + l::POS + 4, v);
    }

    /// `Ch_AllSetThicknessEvent(t)` (`0x0082a8d0`): `Ch_SetThickness` on
    /// every active character.
    pub fn ch_all_set_thickness_event(mem: &mut impl Mem, t: f32) {
        let t = x87::load(t);
        let mut c = CHR_ARRAY.get(mem);
        loop {
            #[allow(clippy::cast_sign_loss)] // the original multiplies unsigned
            let end = CHR_ARRAY
                .get(mem)
                .wrapping_add((CHR_COUNT.get(mem) as u32).wrapping_mul(l::SIZE));
            if c == end {
                break;
            }
            if mem.read::<u8>(c + l::ACTIVE) != 0 {
                ch_set_thickness(mem, c, t);
            }
            c = c.wrapping_add(l::SIZE);
        }
    }

    /// `Ch_WorkSwap(chr)` (`0x00828b30`): copies the character to `chr +
    /// count × 0x880` (past the array when `chr` is its first) and returns
    /// that address.
    pub fn ch_work_swap(mem: &mut impl Mem, chr: u32) -> u32 {
        #[allow(clippy::cast_sign_loss)]
        let dst = chr.wrapping_add((CHR_COUNT.get(mem) as u32).wrapping_mul(l::SIZE));
        let mut buf = vec![0u8; l::SIZE as usize];
        mem.read_bytes(chr, &mut buf);
        mem.write_bytes(dst, &buf);
        dst
    }

    /// `Ch_GetVel(chr, out)` (`0x00834000`): the next position minus the
    /// position.
    pub fn ch_get_vel(mem: &mut impl Mem, chr: u32, out: u32) {
        vel(mem, chr, l::POS_NEXT, out);
    }

    /// `Ch_GetVel2(chr, out)` (`0x00834030`): the second point minus the
    /// position.
    pub fn ch_get_vel2(mem: &mut impl Mem, chr: u32, out: u32) {
        vel(mem, chr, l::POS_2, out);
    }

    fn vel(mem: &mut impl Mem, chr: u32, from: u32, out: u32) {
        for i in 0..3 {
            let v = Reg::load(f(mem, chr + from + 4 * i))
                .sub_m32(f(mem, chr + l::POS + 4 * i))
                .store();
            mem.write(out + 4 * i, v);
        }
    }

    // ---- drawing

    /// `Ch_DrawEnEffect(chr, on)` (`0x0082eb60`).
    pub fn ch_draw_en_effect(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        set_flag(mem, chr, l::FLAGS, l::F_DRAW_EFFECT, on)
    }

    /// `Ch_DrawSetCull(chr, on)` (`0x008300b0`).
    pub fn ch_draw_set_cull(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        set_flag(mem, chr, l::FLAGS, l::F_CULL, on)
    }

    /// `Ch_DrawCalcShade(chr, &r, &g, &b)` (`0x0082eaf0`): the shade
    /// (times the light value when [`SHADE_BY_LIGHT`]) times each colour
    /// fade's current value and colour multiplier.
    pub fn ch_draw_calc_shade(mem: &mut impl Mem, chr: u32, r: u32, g: u32, b: u32) {
        let shade = f(mem, chr + l::FADE_SHADE);
        let s = if SHADE_BY_LIGHT.get(mem) != 0 {
            Reg::load(f(mem, chr + l::FADE_LIGHT))
                .mul_m32(shade)
                .store()
        } else {
            x87::load(shade)
        };
        let s = Reg::load(s);
        let col = |i: u32| chr + l::FADE_SHADE_COL + 12 * i;
        let eff = |i: u32| chr + l::SHADE_COL_EFFECT + 4 * i;
        let vr = Reg::load(f(mem, col(0)))
            .mul_st(s)
            .mul_st(Reg::load(f(mem, eff(0))))
            .store();
        mem.write(r, vr);
        let vg = Reg::load(f(mem, col(1)))
            .mul_st(s)
            .mul_m32(f(mem, eff(1)))
            .store();
        mem.write(g, vg);
        let vb = s.mul_m32(f(mem, col(2))).mul_m32(f(mem, eff(2))).store();
        mem.write(b, vb);
    }

    /// `Ch_DrawGetModelAlpha(chr, model)` (`0x0082eba0`): the alpha fade's
    /// value, times the model's alpha unless `model` is -1.
    pub fn ch_draw_get_model_alpha(mem: &impl Mem, chr: u32, model: u32) -> f32 {
        let alpha = f(mem, chr + l::FADE_ALPHA);
        if model == u32::MAX {
            return alpha;
        }
        let models: u32 = mem.read(chr + l::MODELS);
        let m = models.wrapping_add(model.wrapping_mul(l::MODEL_SIZE));
        Reg::load(f(mem, m + l::MODEL_ALPHA)).mul_m32(alpha).store()
    }

    // ---- models, lights, shadow

    /// `Ch_GetNbElem(chr)` (`0x008303f0`).
    pub fn ch_get_nb_elem(mem: &impl Mem, chr: u32) -> u32 {
        let h: u32 = mem.read(chr + l::MODEL_HEADER);
        u32::from(mem.read::<u16>(h + l::HEADER_ELEMS))
    }

    /// `Ch_GetNbModel(chr)` (`0x00830410`): 0 for headers older than
    /// `0x831`.
    pub fn ch_get_nb_model(mem: &impl Mem, chr: u32) -> u32 {
        let h: u32 = mem.read(chr + l::MODEL_HEADER);
        if mem.read::<u16>(h + l::HEADER_VERSION) < 0x831 {
            0
        } else {
            u32::from(mem.read::<u16>(h + l::HEADER_MODELS))
        }
    }

    /// `Ch_GetShadowDirH(chr)` (`0x00830440`).
    pub fn ch_get_shadow_dir_h(mem: &impl Mem, chr: u32) -> u32 {
        mem.read(chr + l::SHADOW_DIR_H)
    }

    /// `Ch_SetShadowDirH(chr, dir)` (`0x00830c90`).
    pub fn ch_set_shadow_dir_h(mem: &mut impl Mem, chr: u32, dir: u32) {
        mem.write(chr + l::SHADOW_DIR_H, dir);
    }

    /// `Ch_PLightGetEnable(chr)` (`0x00830ad0`), sign-extended.
    pub fn ch_plight_get_enable(mem: &impl Mem, chr: u32) -> i32 {
        u8_at(mem, chr + l::PLIGHT)
    }

    /// `Ch_PLightSetEnable(chr, bits)` (`0x00830ae0`).
    pub fn ch_plight_set_enable(mem: &mut impl Mem, chr: u32, bits: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(chr + l::PLIGHT, bits as u8);
    }

    /// `Ch_PLightSetEnableMask(chr, bits, mask)` (`0x00830b00`): the
    /// `mask` bits cleared, then `bits` set.
    pub fn ch_plight_set_enable_mask(mem: &mut impl Mem, chr: u32, bits: u32, mask: u32) {
        let v: u8 = mem.read(chr + l::PLIGHT);
        #[allow(clippy::cast_possible_truncation)] // low bytes
        mem.write(chr + l::PLIGHT, (v & !(mask as u8)) | bits as u8);
    }

    /// `Ch_ShadowSetHeight(chr, h)` (`0x00830cb0`).
    pub fn ch_shadow_set_height(mem: &mut impl Mem, chr: u32, h: f32) {
        put_f(mem, chr + l::SHADOW_HEIGHT, h);
    }

    // ---- binding and elements

    fn bind(mem: &mut impl Mem, chr: u32, parent: u32, elem: u32) {
        mem.write(chr + l::BIND_PARENT, parent);
        let id = if parent == 0 {
            u32::MAX
        } else {
            mem.read(parent + l::BIND_SELF)
        };
        mem.write(chr + l::BIND_PARENT_ID, id);
        mem.write(chr + l::BIND_ELEM, elem);
    }

    /// `Ch_BindSetScale(chr, s)` (`0x00832610`).
    pub fn ch_bind_set_scale(mem: &mut impl Mem, chr: u32, s: f32) {
        put_f(mem, chr + l::BIND_SCALE, s);
    }

    /// `Ch_BindToElemDirect(chr, parent, elem)` (`0x00832680`): bound with
    /// no offset.
    pub fn ch_bind_to_elem_direct(mem: &mut impl Mem, chr: u32, parent: u32, elem: u32) {
        bind(mem, chr, parent, elem);
        for i in 0..3 {
            mem.write(chr + l::BIND_OFFSET + 4 * i, 0.0f32);
        }
        mem.write(chr + l::BIND_1B0, 1.0f32);
    }

    /// `Ch_BindToElemDirectOfs(chr, parent, elem, x, y, z)` (`0x008326e0`):
    /// bound with an offset of (x, y, z) × 100 × the block's bind scale.
    pub fn ch_bind_to_elem_direct_ofs(
        mem: &mut impl Mem,
        chr: u32,
        parent: u32,
        elem: u32,
        ofs: [f32; 3],
    ) {
        bind(mem, chr, parent, elem);
        let s = f(mem, blk(mem, chr) + l::BLK_BIND_SCALE);
        let c = Reg::constant(100.0);
        let vx = Reg::load(ofs[0]).mul_st(c).mul_st(Reg::load(s)).store();
        mem.write(chr + l::BIND_OFFSET, vx);
        let vy = Reg::load(ofs[1]).mul_st(c).mul_m32(s).store();
        mem.write(chr + l::BIND_OFFSET + 4, vy);
        let vz = c.mul_m32(ofs[2]).mul_m32(s).store();
        mem.write(chr + l::BIND_OFFSET + 8, vz);
        mem.write(chr + l::BIND_1B0, 1.0f32);
    }

    fn elem(mem: &impl Mem, chr: u32, e: u32) -> u32 {
        let elems: u32 = mem.read(chr + l::ELEMS);
        elems.wrapping_add(e.wrapping_mul(l::ELEM_SIZE))
    }

    /// `Ch_ElemGetParent(chr, elem)` (`0x00833cf0`).
    pub fn ch_elem_get_parent(mem: &impl Mem, chr: u32, e: u32) -> u32 {
        let data: u32 = mem.read(elem(mem, chr, e) + l::ELEM_DATA);
        mem.read(data + 4)
    }

    /// `Ch_ElemSetQuatMatrix(chr, elem, matrix, weight)` (`0x00833df0`).
    pub fn ch_elem_set_quat_matrix(mem: &mut impl Mem, chr: u32, e: u32, matrix: u32, w: f32) {
        let at = elem(mem, chr, e);
        mem.write(at + l::ELEM_QUAT_MATRIX, matrix);
        let at = elem(mem, chr, e);
        put_f(mem, at + l::ELEM_QUAT_WEIGHT, w);
    }

    /// `Ch_ElemSetReplaceMatrix(chr, elem, matrix)` (`0x00833e30`).
    pub fn ch_elem_set_replace_matrix(mem: &mut impl Mem, chr: u32, e: u32, matrix: u32) {
        let at = elem(mem, chr, e);
        mem.write(at + l::ELEM_REPLACE_MATRIX, matrix);
    }

    /// `Ch_RelElemMapping(list, delta)` (`0x008377a0`): adds `delta` to
    /// each of the `list[1]` pointers at `list[0]`.
    pub fn ch_rel_elem_mapping(mem: &mut impl Mem, list: u32, delta: u32) {
        let ptrs: u32 = mem.read(list);
        let mut i = 0u32;
        while i < mem.read::<u32>(list + 4) {
            let at = ptrs.wrapping_add(4 * i);
            let v: u32 = mem.read(at);
            mem.write(at, v.wrapping_add(delta));
            i += 1;
        }
    }

    // ---- normals

    /// `Ch_GetCalcNorm(chr)` (`0x00833f30`).
    pub fn ch_get_calc_norm(mem: &impl Mem, chr: u32) -> u32 {
        flag(mem, chr, l::FLAGS, l::F_CALC_NORM)
    }

    /// `Ch_SetCalcNorm(chr, on)` (`0x00835750`): returns the old bit.
    pub fn ch_set_calc_norm(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        en_flag(mem, chr, l::FLAGS, l::F_CALC_NORM, on)
    }

    /// `Ch_SetCalcNorm2(chr, on)` (`0x00835780`): a debug print (a no-op in
    /// this build), then [`l::F_CALC_NORM2`]; returns the bit that changed.
    pub fn ch_set_calc_norm2(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        set_flag(mem, chr, l::FLAGS, l::F_CALC_NORM2, on)
    }

    /// `Ch_ReserveCalcNorm(chr)` (`0x00835710`) and `Ch_ReserveCalcNorm2`
    /// (`0x00835730`, `bit` [`l::F_RESERVE_CALC_NORM2`]): the bit, unless
    /// the normals are recomputed every frame anyway.
    pub fn ch_reserve_calc_norm(mem: &mut impl Mem, chr: u32, bit: u32) {
        let w: u32 = mem.read(chr + l::FLAGS);
        if w & l::F_CALC_NORM == 0 {
            mem.write(chr + l::FLAGS, w | bit);
        }
    }

    // ---- mouth, neck, sound

    /// `Ch_MouthGetAutoTalk(chr)` (`0x00834740`), sign-extended.
    pub fn ch_mouth_get_auto_talk(mem: &impl Mem, chr: u32) -> i32 {
        u8_at(mem, chr + l::MOUTH_AUTO_TALK)
    }

    /// `Ch_MouthSetAutoTalk(chr, on)` (`0x00834770`).
    pub fn ch_mouth_set_auto_talk(mem: &mut impl Mem, chr: u32, on: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(chr + l::MOUTH_AUTO_TALK, on as u8);
    }

    /// `Ch_MouthInit(chr)` (`0x00834750`).
    pub fn ch_mouth_init(mem: &mut impl Mem, chr: u32) {
        let v = MOUTH_INIT.get(mem);
        put_f(mem, chr + l::MOUTH_4D8, v);
    }

    /// `Ch_NeckSetLimit(chr, a, b)` (`0x00834fd0`).
    pub fn ch_neck_set_limit(mem: &mut impl Mem, chr: u32, a: f32, b: f32) {
        put_f(mem, chr + l::NECK_LIMIT, a);
        put_f(mem, chr + l::NECK_LIMIT + 4, b);
    }

    /// `Ch_NeckSetSpeed(chr, speed)` (`0x00835020`): 0 takes the default.
    pub fn ch_neck_set_speed(mem: &mut impl Mem, chr: u32, speed: f32) {
        let v = if speed == 0.0 {
            NECK_SPEED_DEFAULT.get(mem)
        } else {
            speed
        };
        put_f(mem, chr + l::NECK_SPEED, v);
    }

    /// `Ch_SoundSetId(chr, id)` (`0x00835b50`).
    pub fn ch_sound_set_id(mem: &mut impl Mem, chr: u32, id: u32) {
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(chr + l::SOUND_ID, id as u8);
    }

    // ---- optional positions

    /// `Ch_OptposGetBottom/Top/CameraRef(chr, out)` (`0x00835600`,
    /// `0x008356e0`, `0x00835630`): 16 bytes from `chr + field`.
    pub fn ch_optpos_copy(mem: &mut impl Mem, chr: u32, field: u32, out: u32) {
        let v: [u32; 4] = mem.read(chr + field);
        mem.write(out, v);
    }

    /// `Ch_OptposGetExist(chr, i)` (`0x00835680`): 1 when position `i`'s x
    /// is not 0 (NaN counts).
    pub fn ch_optpos_get_exist(mem: &impl Mem, chr: u32, i: u32) -> u32 {
        let at = chr.wrapping_add(i.wrapping_add(l::OPTPOS / 16).wrapping_mul(16));
        u32::from(f(mem, at) != 0.0)
    }

    // ---- motion

    /// `Ch_MotionGetId(chr)` (`0x008371a0`).
    pub fn ch_motion_get_id(mem: &impl Mem, chr: u32) -> u32 {
        mem.read(chr + l::MOTION_ID)
    }

    /// `Ch_MotionGetFrame(chr)` (`0x00839860`).
    pub fn ch_motion_get_frame(mem: &impl Mem, chr: u32) -> u32 {
        mem.read(chr + l::MOTION_FRAME)
    }

    /// `Ch_MotionSetLoop(chr, n)` (`0x00839960`).
    pub fn ch_motion_set_loop(mem: &mut impl Mem, chr: u32, n: u32) {
        #[allow(clippy::cast_possible_truncation)] // a u16 field
        mem.write(chr + l::MOTION_LOOP, n as u16);
    }

    /// `Ch_GetWaitMotion(chr)` (`0x008392c0`): non-zero while a scripted
    /// motion plays, sign-extended.
    pub fn ch_get_wait_motion(mem: &impl Mem, chr: u32) -> i32 {
        u8_at(mem, chr + l::MOTION_SCRIPTED)
    }

    /// `Ch_GetMotionType(chr)` (`0x00833f70`), sign-extended.
    pub fn ch_get_motion_type(mem: &impl Mem, chr: u32) -> i32 {
        u8_at(mem, chr + l::MOTION_TYPE)
    }

    /// `Ch_GetKeepFps(chr)` (`0x008392a0`).
    pub fn ch_get_keep_fps(mem: &impl Mem, chr: u32) -> u32 {
        flag(mem, chr, l::FLAGS, l::F_KEEP_FPS)
    }

    /// `Ch_SetKeepFps(chr, on)` (`0x00839b50`): returns the old bit.
    pub fn ch_set_keep_fps(mem: &mut impl Mem, chr: u32, on: u32) -> u32 {
        en_flag(mem, chr, l::FLAGS, l::F_KEEP_FPS, on)
    }

    /// The ids `Ch_IsNotExceptionId` looks for (9 words).
    pub const EXCEPTION_IDS: Global<u32> = Global::new(0x00b5_9840);

    /// `Ch_IsNotExceptionId(id)` (`0x00831780`): 0 when `id` is one of
    /// the nine words at [`EXCEPTION_IDS`], else 1.
    pub fn ch_is_not_exception_id(mem: &impl Mem, id: u32) -> u32 {
        let t = EXCEPTION_IDS.ptr(mem).addr;
        u32::from(!(0..9).any(|i| mem.read::<u32>(t + 4 * i) == id))
    }

    /// `Ch_ElemInMotion(chr, e)` (`0x008342c0`): 1 when element `e` is
    /// among the elements the motion at `+0x758` animates (`u16` count
    /// `+2`, `u16` list from `+8`; every element when there is none),
    /// looking at no more than the model's element count.
    pub fn ch_elem_in_motion(mem: &impl Mem, chr: u32, e: u32) -> u32 {
        let m: u32 = mem.read(chr + 0x758);
        let n = if m == 0 {
            0
        } else {
            i32::from(mem.read::<u16>(m + 2))
        };
        let header: u32 = mem.read(chr + l::MODEL_HEADER);
        let elems = i32::from(mem.read::<u16>(header + l::HEADER_ELEMS));
        let mut i = 0i32;
        while i < elems {
            let v = if m == 0 {
                i.cast_unsigned()
            } else {
                if i >= n {
                    return 0;
                }
                u32::from(mem.read::<u16>(m.wrapping_add(8 + 2 * i.cast_unsigned())))
            };
            if e == v {
                return 1;
            }
            i += 1;
        }
        0
    }

    /// `Ch_SetCalcNorm2` (`0x007969c0`) on a holder `{chr, …, +0x3c u8,
    /// +0x3d u8}`: on when either byte is set; returns it.
    pub fn ch_set_calc_norm2_holder(mem: &mut impl Mem, p: u32) -> u32 {
        let on = u32::from(mem.read::<u8>(p + 0x3c) != 0 || mem.read::<u8>(p + 0x3d) != 0);
        let chr: u32 = mem.read(p);
        if chr != 0 {
            ch_set_calc_norm2(mem, chr, on);
        }
        on
    }

    /// A C runtime intrinsic `Ch_Neck` calls through its import thunk:
    /// `_CIatan2` (`0x009498bc`, operands ST1 then ST0), `_CIsin`
    /// (`0x009498c8`), `_CIcos` (`0x009498c2`), `_CIsqrt` (`0x009497b8`).
    pub const CI_ATAN2: u32 = 0x0094_98bc;
    pub const CI_SIN: u32 = 0x0094_98c8;
    pub const CI_COS: u32 = 0x0094_98c2;
    pub const CI_SQRT: u32 = 0x0094_97b8;
    /// The functions `Ch_Neck` calls: `Ch_OptposGetElement`,
    /// `Ch_ElemSetQuatMatrix`, `Ch_CheckHandle`, `Ch_OptposGet`,
    /// `Ch_OptposGetExist`, `Sg_CameraGetPos`, `FFXVu0UnitMatrix`,
    /// `FFXVu0RotMatrixZ`, `FFXVu0RotMatrixY`, `FFXVu0Normalize`,
    /// `Angle_Wrap_00836720` and `Sg_RadToDeg` (`f32` in, `ST0` out),
    /// `Sg_VectorToMatrix2`, `FFXVu0MulMatrix`, `Sg_MatrixToQuaternion`,
    /// `Sg_QuaternionSlerp`, `Sg_QuaternionToMatrix`.
    pub const OPTPOS_GET_ELEMENT: u32 = 0x0083_5660;
    pub const ELEM_SET_QUAT_MATRIX: u32 = 0x0083_3df0;
    pub const CHECK_HANDLE: u32 = 0x0082_61b0;
    pub const OPTPOS_GET: u32 = 0x0083_54f0;
    pub const OPTPOS_GET_EXIST: u32 = 0x0083_5680;
    pub const CAMERA_GET_POS: u32 = 0x0083_f260;
    pub const UNIT_MATRIX: u32 = 0x006e_d9a0;
    pub const ROT_MATRIX_Z: u32 = 0x006e_d800;
    pub const ROT_MATRIX_Y: u32 = 0x006e_d7c0;
    pub const NORMALIZE: u32 = 0x006e_d6f0;
    pub const ANGLE_WRAP: u32 = 0x0083_6720;
    pub const RAD_TO_DEG: u32 = 0x0083_66f0;
    pub const VECTOR_TO_MATRIX2: u32 = 0x0084_5560;
    pub const MUL_MATRIX: u32 = 0x006e_d5c0;
    pub const MATRIX_TO_QUATERNION: u32 = 0x0084_5170;
    pub const QUATERNION_SLERP: u32 = 0x0084_5310;
    pub const QUATERNION_TO_MATRIX: u32 = 0x0084_5450;
    /// Bytes of scratch `ch_neck` needs: the original's stack frame below
    /// its saved cookie (offset `0x9c - n` stands for `[ebp - n]`).
    pub const NECK_SCRATCH: u32 = 0x9c;

    /// How `ch_neck` reaches other code: `call` a cdecl function for EAX,
    /// `fcall` a cdecl function for its `ST0` result, `ci` an intrinsic
    /// (thunk, operands loaded in order, so the last is `ST0`) for its
    /// result as the caller stores it (`fstp dword`).
    pub struct NeckCalls<'a, M> {
        pub call: &'a mut dyn FnMut(&mut M, u32, &[u32]) -> u32,
        pub fcall: &'a mut dyn FnMut(&mut M, u32, &[u32]) -> f64,
        pub ci: &'a mut dyn FnMut(&mut M, u32, &[f64]) -> f32,
    }

    /// `Ch_Neck(chr)` (`0x008348d0`): turns the head (option point 9's
    /// element) toward a target. The weight `+0x418` eases toward `+0x414`
    /// by `1 / +0x420` (above 0.99 it becomes 1, at most 0.01 it becomes
    /// 0; 0 releases the element with no matrix). The target by mode
    /// (`i8` `+0x40d`): 1 the camera, 2 character `+0x4c0`'s point 9 (a
    /// bad handle releases the element), 3 the position at `+0x4c0`, 4 a
    /// fixed turn (angles `+0x4c4` about z then `+0x4c0` about y) straight
    /// into the matrix `+0x46c`; others leave the target unset (the
    /// original's stack). The yaw toward it, relative to the shoulders
    /// (points 0x13, 0x14) and the facing `+0x158`, in degrees, clamps to
    /// the window `+0x424` .. `+0x428` (flag `+0x4d0` remembering the
    /// side, which shifts the lower edge by ±15), and `+0x41c` follows it
    /// (by a third, or a quarter past 90 degrees). The head's matrix is
    /// built from that yaw and the pitch, mirrored for models with byte
    /// `+0x40` of `+0x1c4` = 1, times `+0x42c`, blended (`+0x4bc`, less a
    /// quarter each frame) with the quaternion at `+0x4ac`, and set with
    /// the weight. Every float as the x87 at 53 bits. Returns
    /// `Ch_ElemSetQuatMatrix`'s result (0 without the element).
    #[allow(clippy::too_many_lines, clippy::similar_names)]
    pub fn ch_neck<M: Mem>(mem: &mut M, chr: u32, scratch: u32, c: &mut NeckCalls<'_, M>) -> u32 {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let fl = |mem: &M, a: u32| f64::from(mem.read::<f32>(a));
        let cf = |mem: &M, a: u32| f64::from(mem.read::<f32>(mem.rebase(a)));
        let cd = |mem: &M, a: u32| mem.read::<f64>(mem.rebase(a));
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r32 = |x: f64| x as f32;
        let bits = |mem: &M, a: u32| crate::x87::load(mem.read::<f32>(a)).to_bits();
        let at = |off: u32| scratch + 0x9c - off;
        let (t, p, q, v, m, vec) = (at(0x74), at(0x64), at(0x9c), at(0x54), at(0x44), at(0x84));
        let elem = (c.call)(mem, r(mem, OPTPOS_GET_ELEMENT), &[chr, 9, 0]);
        if elem == 0 {
            return 0;
        }
        let set = r(mem, ELEM_SET_QUAT_MATRIX);
        let w = r32(
            (fl(mem, chr + 0x414) - fl(mem, chr + 0x418)) / fl(mem, chr + 0x420)
                + fl(mem, chr + 0x418),
        );
        mem.write(chr + 0x418, w);
        if f64::from(w) > cd(mem, 0x00b3_eb70) {
            mem.write(chr + 0x418, 1f32);
        }
        // `fcomp`: 0.01 not below the weight, ordered
        if cf(mem, 0x00b1_f2b0) >= fl(mem, chr + 0x418) {
            mem.write(chr + 0x418, 0f32);
        }
        if fl(mem, chr + 0x418) == 0.0 {
            return (c.call)(mem, set, &[chr, elem, 0, 0]);
        }
        let mat = chr + 0x46c;
        let mode = mem.read::<i8>(chr + 0x40d);
        if mode == 4 {
            (c.call)(mem, r(mem, UNIT_MATRIX), &[m]);
            let z = bits(mem, chr + 0x4c4);
            (c.call)(mem, r(mem, ROT_MATRIX_Z), &[m, m, z]);
            let y = bits(mem, chr + 0x4c0);
            (c.call)(mem, r(mem, ROT_MATRIX_Y), &[mat, m, y]);
        } else {
            match mode {
                1 => {
                    (c.call)(mem, r(mem, CAMERA_GET_POS), &[t]);
                }
                2 => {
                    let h: u32 = mem.read(chr + 0x4c0);
                    if (c.call)(mem, r(mem, CHECK_HANDLE), &[h]) != 0 {
                        return (c.call)(mem, set, &[chr, elem, 0, 0]);
                    }
                    let h: u32 = mem.read(chr + 0x4c0);
                    (c.call)(mem, r(mem, OPTPOS_GET), &[h, 9, t]);
                }
                3 => {
                    for k in 0..4 {
                        let x: u32 = mem.read(chr + 0x4c0 + 4 * k);
                        mem.write(t + 4 * k, x);
                    }
                }
                _ => {}
            }
            // the shoulders' yaw
            let mut a = 0f32;
            let exist = r(mem, OPTPOS_GET_EXIST);
            if (c.call)(mem, exist, &[chr, 0x13]) != 0 && (c.call)(mem, exist, &[chr, 0x14]) != 0 {
                let get = r(mem, OPTPOS_GET);
                (c.call)(mem, get, &[chr, 0x13, p]);
                (c.call)(mem, get, &[chr, 0x14, q]);
                mem.write(p, r32(fl(mem, p) - fl(mem, q)));
                mem.write(p + 8, r32(fl(mem, p + 8) - fl(mem, q + 8)));
                let quarter = bits(mem, r(mem, 0x00b5_99c0));
                let d = (c.fcall)(mem, r(mem, ANGLE_WRAP), &[quarter]);
                let y = (c.ci)(mem, r(mem, CI_ATAN2), &[fl(mem, p + 8), fl(mem, p)]);
                let s = r32(f64::from(y) + d - fl(mem, chr + 0x158));
                let s = r32((c.fcall)(mem, r(mem, ANGLE_WRAP), &[s.to_bits()]));
                a = r32((c.fcall)(mem, r(mem, RAD_TO_DEG), &[s.to_bits()]));
            }
            // the target's yaw
            (c.call)(mem, r(mem, OPTPOS_GET), &[chr, 9, v]);
            for k in 0..3 {
                mem.write(v + 4 * k, r32(fl(mem, v + 4 * k) - fl(mem, t + 4 * k)));
            }
            (c.call)(mem, r(mem, NORMALIZE), &[v, v]);
            let b = (c.ci)(mem, r(mem, CI_ATAN2), &[fl(mem, v + 8), fl(mem, v)]);
            let quarter = bits(mem, r(mem, 0x00b5_99c0));
            let wq = (c.fcall)(mem, r(mem, ANGLE_WRAP), &[quarter]);
            let s = r32(f64::from(b) - wq - fl(mem, chr + 0x158));
            let s = r32((c.fcall)(mem, r(mem, ANGLE_WRAP), &[s.to_bits()]));
            let mut deg = r32((c.fcall)(mem, r(mem, RAD_TO_DEG), &[s.to_bits()]));
            // the window
            let off = if mem.read::<u32>(chr + 0x4d0) == 0 {
                cf(mem, 0x00b5_99e8)
            } else {
                cf(mem, 0x00b5_99f8)
            };
            let a64 = f64::from(a);
            let lo = (off - cd(mem, 0x00b5_8cd0)) + a64;
            let d64 = f64::from(deg);
            let hi = fl(mem, chr + 0x424) + a64;
            if d64 > lo && hi > d64 {
                mem.write(chr + 0x4d0, 1u32);
                deg = r32(hi);
            } else {
                let to_upper = if d64 <= lo {
                    true
                } else {
                    // above lo, or unordered
                    let h2 = fl(mem, chr + 0x428) + a64;
                    h2 < d64
                };
                if to_upper {
                    mem.write(chr + 0x4d0, 0u32);
                    deg = r32(a64 + fl(mem, chr + 0x428));
                }
            }
            if cf(mem, 0x00b4_8394) > f64::from(deg) {
                deg = r32(f64::from(deg) + cd(mem, 0x00b4_8380));
            }
            let diff = f64::from(deg) - fl(mem, chr + 0x41c);
            let mag = if 0.0 > diff { -diff } else { diff };
            let step = if mag > cd(mem, 0x00b5_8cd0) {
                diff * cd(mem, 0x00b3_4f90)
            } else {
                diff / cd(mem, 0x00b4_1d00)
            };
            mem.write(chr + 0x41c, r32(step + fl(mem, chr + 0x41c)));
            let rad = r32(fl(mem, chr + 0x41c) * cd(mem, 0x00b5_82f0) / cd(mem, 0x00b4_8378));
            let th = r32((c.fcall)(mem, r(mem, ANGLE_WRAP), &[rad.to_bits()]));
            let s = (c.ci)(mem, r(mem, CI_SIN), &[f64::from(th)]);
            mem.write(vec, s);
            let co = (c.ci)(mem, r(mem, CI_COS), &[f64::from(th)]);
            mem.write(vec + 8, co);
            let (x, y, z) = (fl(mem, v), fl(mem, v + 4), fl(mem, v + 8));
            let h = (c.ci)(mem, r(mem, CI_SQRT), &[x * x + z * z]);
            let pitch = (c.ci)(mem, r(mem, CI_ATAN2), &[y, f64::from(h)]);
            let sp = (c.ci)(mem, r(mem, CI_SIN), &[f64::from(pitch)]);
            mem.write(vec + 4, sp);
            let model: u32 = mem.read(chr + 0x1c4);
            if mem.read::<i8>(model.wrapping_add(0x40)) == 1 {
                mem.write(vec + 8, -crate::x87::load(mem.read::<f32>(vec + 8)));
                mem.write(vec + 4, -crate::x87::load(mem.read::<f32>(vec + 4)));
            }
            (c.call)(mem, r(mem, VECTOR_TO_MATRIX2), &[mat, vec]);
        }
        (c.call)(mem, r(mem, MUL_MATRIX), &[mat, chr + 0x42c, mat]);
        if fl(mem, chr + 0x4bc) != 0.0 {
            let qt = p;
            (c.call)(mem, r(mem, MATRIX_TO_QUATERNION), &[mat, qt]);
            let k = bits(mem, chr + 0x4bc);
            (c.call)(mem, r(mem, QUATERNION_SLERP), &[qt, chr + 0x4ac, k, qt]);
            (c.call)(mem, r(mem, QUATERNION_TO_MATRIX), &[qt, mat]);
            mem.write(
                chr + 0x4bc,
                r32(fl(mem, chr + 0x4bc) - cd(mem, 0x00b3_4f90)),
            );
        }
        let wb = bits(mem, chr + 0x418);
        (c.call)(mem, set, &[chr, elem, mat, wb])
    }

    /// `Ch_NeckSetLimit(chr, lo, hi)` (`0x00834fd0`), `FFXVu0RotMatrixX`
    /// (`0x006ed780`), `FFXVu0CopyMatrix` (`0x006ed360`).
    pub const NECK_SET_LIMIT: u32 = 0x0083_4fd0;
    pub const ROT_MATRIX_X: u32 = 0x006e_d780;
    pub const COPY_MATRIX: u32 = 0x006e_d360;
    /// Bytes of scratch `ch_neck_init` needs (its matrix).
    pub const NECK_INIT_SCRATCH: u32 = 0x40;

    /// `Ch_NeckInit(chr)` (`0x00834e60`): neck limits 20 and 160
    /// (`Ch_NeckSetLimit`); with a head (option point 9's element, 0x160
    /// bytes each at `+0x328`), its rest turn (x, y, z angles at the
    /// element's `+0xe4`..`+0xec`) times its bone matrix (`+0x88` of the
    /// element's `+0` pointer) becomes `+0x42c`, copied to `+0x46c` and as
    /// a quaternion to `+0x4ac`; `+0x41c` = the wrapped quarter turn. The
    /// original returns nothing usable (EAX is `Angle_Wrap`'s status word).
    pub fn ch_neck_init<M: Mem>(mem: &mut M, chr: u32, scratch: u32, c: &mut NeckCalls<'_, M>) {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let bits = |mem: &M, a: u32| crate::x87::load(mem.read::<f32>(a)).to_bits();
        let (lo, hi) = (
            bits(mem, r(mem, 0x00b4_ae10)),
            bits(mem, r(mem, 0x00b5_4c78)),
        );
        (c.call)(mem, r(mem, NECK_SET_LIMIT), &[chr, lo, hi]);
        let elem = (c.call)(mem, r(mem, OPTPOS_GET_ELEMENT), &[chr, 9, 0]);
        if elem == 0 {
            return;
        }
        let e = elem.wrapping_mul(0x160).wrapping_add(mem.read(chr + 0x328));
        let m = scratch;
        (c.call)(mem, r(mem, UNIT_MATRIX), &[m]);
        for (f, off) in [
            (ROT_MATRIX_X, 0xe4u32),
            (ROT_MATRIX_Y, 0xe8),
            (ROT_MATRIX_Z, 0xec),
        ] {
            let a = bits(mem, e.wrapping_add(off));
            (c.call)(mem, r(mem, f), &[m, m, a]);
        }
        let bone = mem.read::<u32>(e).wrapping_add(0x88);
        (c.call)(mem, r(mem, MUL_MATRIX), &[chr + 0x42c, bone, m]);
        (c.call)(mem, r(mem, COPY_MATRIX), &[chr + 0x46c, chr + 0x42c]);
        (c.call)(
            mem,
            r(mem, MATRIX_TO_QUATERNION),
            &[chr + 0x46c, chr + 0x4ac],
        );
        let quarter = bits(mem, r(mem, 0x00b5_99c0));
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let w = (c.fcall)(mem, r(mem, ANGLE_WRAP), &[quarter]) as f32;
        mem.write(chr + 0x41c, w);
    }

    /// Bytes of scratch `ch_elem_calc_quaternion` needs (two quaternions,
    /// the original's `[ebp-0x24]` and `[ebp-0x14]`).
    pub const ELEM_QUAT_SCRATCH: u32 = 0x20;

    /// `Ch_ElemCalcQuaternion(e)` (`0x00833960`): an element whose blend
    /// (`f32` `+0xe0`) is not 0 takes the matrix at `[+0xcc]` into `+0x88`:
    /// copied at 1, else slerped (quaternions of both by the blend); its
    /// position `+0xb8`..`+0xc0` is kept (floats through x87). The original
    /// returns nothing usable (EAX holds its frame cookie at 0).
    pub fn ch_elem_calc_quaternion<M: Mem>(
        mem: &mut M,
        e: u32,
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) {
        let w = f64::from(mem.read::<f32>(e + 0xe0));
        if w == 0.0 {
            return;
        }
        let keep = [0u32, 4, 8].map(|k| crate::x87::load(mem.read(e + 0xb8 + k)));
        let mat = e + 0x88;
        #[allow(clippy::float_cmp)] // the original's fucompp with 1
        if w == 1.0 {
            let src: u32 = mem.read(e + 0xcc);
            call(mem, mem.rebase(COPY_MATRIX), &[mat, src]);
        } else {
            let (a, b) = (scratch, scratch + 0x10);
            let to_q = mem.rebase(MATRIX_TO_QUATERNION);
            call(mem, to_q, &[mat, a]);
            let src: u32 = mem.read(e + 0xcc);
            call(mem, to_q, &[src, b]);
            let k = crate::x87::load(mem.read::<f32>(e + 0xe0)).to_bits();
            call(mem, mem.rebase(QUATERNION_SLERP), &[a, b, k, b]);
            call(mem, mem.rebase(QUATERNION_TO_MATRIX), &[b, mat]);
        }
        for (k, v) in (0u32..).zip(keep) {
            mem.write(e + 0xb8 + 4 * k, v);
        }
    }

    /// The mouth mode (`i8`: 1 a scripted shape, 2 shut, 3 talking, others
    /// follow the character's talk byte `+0x4d4`) and the shape index
    /// (`i8`) `Ch_Mouth` reads, the shapes (`u8`), the open threshold
    /// (`i16`, `g_ChrMouthTh`).
    pub const MOUTH_MODE: u32 = 0x0130_08d9;
    pub const MOUTH_SHAPE: u32 = 0x0130_08da;
    pub const MOUTH_SHAPES: u32 = 0x00c4_971c;
    pub const MOUTH_TH: u32 = 0x00c4_9718;
    /// `TexAnim_PosSet(anim, which, pos)` (`0x0077e9a0`).
    pub const TEX_ANIM_POS_SET: u32 = 0x0077_e9a0;

    /// `Ch_Mouth(chr)` (`0x008344f0`): the mouth opening `+0x4dc` (tenths
    /// of a degree) moves toward a goal `+0x4de` by a step `+0x4e3`
    /// (1-20) while a counter `+0x4e1` (1-4) runs; a new goal (talking:
    /// `rand` % 100 − 30, at least 0; else 0) comes when the counter runs
    /// out, or at once when not talking and not shut. Mode 1 takes the goal
    /// from the shape table. The opening turns the jaw element (option
    /// point 13) about the model's axis (byte `+0x42` of `+0x1c4`; its
    /// rest angles `+0x44`..`+0x4c`, negated with byte `+0x43`), and with
    /// a mouth texture (`+0x6fc`) shows its open frame past the threshold
    /// (`TexAnim_PosSet(+0x700, 1, open)`). Returns what EAX holds.
    #[allow(clippy::too_many_lines)]
    pub fn ch_mouth<M: Mem>(mem: &mut M, chr: u32, c: &mut NeckCalls<'_, M>) -> u32 {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let fl = |mem: &M, a: u32| f64::from(mem.read::<f32>(a));
        let ld = |mem: &M, a: u32| crate::x87::load(mem.read::<f32>(a));
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r32 = |x: f64| x as f32;
        let rand_at: u32 = mem.read(r(mem, crate::texanim::bridge::RAND_SLOT));
        let mode = mem.read::<i8>(r(mem, MOUTH_MODE));
        let talking = mode == 1 || (mode != 2 && (mode == 3 || mem.read::<u8>(chr + 0x4d4) != 0));
        let new_goal = |mem: &mut M, c: &mut NeckCalls<'_, M>, zero: bool| {
            if zero {
                mem.write(chr + 0x4de, 0u16);
            }
            let x = (c.call)(mem, rand_at, &[]).cast_signed() >> 5;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            mem.write(chr + 0x4e1, ((x & 3) + 1) as u8);
            let x = (c.call)(mem, rand_at, &[]).cast_signed() >> 5;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            mem.write(chr + 0x4e3, ((x & 0xff) % 20 + 1) as u8);
        };
        if talking {
            if mem.read::<u8>(chr + 0x4e1) == 0 {
                let x = (c.call)(mem, rand_at, &[]).cast_signed() >> 5;
                #[allow(clippy::cast_possible_truncation)]
                let mut g = x as i16;
                mem.write(chr + 0x4de, g);
                if g < 0 {
                    g = g.wrapping_neg();
                    mem.write(chr + 0x4de, g);
                }
                let g = (mem.read::<i16>(chr + 0x4de) % 100).wrapping_sub(30);
                mem.write(chr + 0x4de, g);
                new_goal(mem, c, g < 0);
            }
        } else {
            let goal: u16 = mem.read(chr + 0x4de);
            if mem.read::<u16>(chr + 0x4dc) == goal && goal == 0 {
                return 0;
            }
            new_goal(mem, c, true);
        }
        if mem.read::<i8>(r(mem, MOUTH_MODE)) == 1 {
            let k = i32::from(mem.read::<i8>(r(mem, MOUTH_SHAPE))).cast_unsigned();
            let v = mem.read::<u8>(r(mem, MOUTH_SHAPES).wrapping_add(k));
            mem.write(chr + 0x4de, u16::from(v));
        }
        let cur = mem.read::<i16>(chr + 0x4dc);
        let goal = mem.read::<i16>(chr + 0x4de);
        let n = mem.read::<u8>(chr + 0x4e1).wrapping_sub(1);
        mem.write(chr + 0x4e1, n);
        if cur != goal {
            let step = i16::from(mem.read::<u8>(chr + 0x4e3));
            let v = if cur > goal {
                cur.wrapping_sub(step)
            } else {
                step.wrapping_add(cur)
            };
            mem.write(chr + 0x4dc, v);
            if (cur > goal && v < goal) || (cur < goal && v > goal) {
                mem.write(chr + 0x4dc, goal);
            }
        }
        let mut eax = (c.call)(mem, r(mem, OPTPOS_GET_ELEMENT), &[chr, 0xd, 0]);
        if eax != 0 {
            let e = eax.wrapping_mul(0x160).wrapping_add(mem.read(chr + 0x328));
            let model: u32 = mem.read(chr + 0x1c4);
            let mut a =
                r32(f64::from(mem.read::<i16>(chr + 0x4dc)) / mem.read::<f64>(r(mem, 0x00b9_22b0)));
            if mem.read::<u8>(model.wrapping_add(0x43)) != 0 {
                a = -a;
            }
            let b = r32(f64::from(a) * mem.read::<f64>(r(mem, 0x00b5_82f0))
                / mem.read::<f64>(r(mem, 0x00b4_8378)));
            let th = f64::from(r32((c.fcall)(mem, r(mem, ANGLE_WRAP), &[b.to_bits()])));
            let v = ld(mem, model.wrapping_add(0x44));
            mem.write(e.wrapping_add(0xe4), v);
            let model: u32 = mem.read(chr + 0x1c4);
            let v = ld(mem, model.wrapping_add(0x48));
            mem.write(e.wrapping_add(0xe8), v);
            let model: u32 = mem.read(chr + 0x1c4);
            let z = ld(mem, model.wrapping_add(0x4c));
            mem.write(e.wrapping_add(0xec), z);
            let model: u32 = mem.read(chr + 0x1c4);
            let kind = i32::from(mem.read::<i8>(model.wrapping_add(0x42)));
            match kind {
                0 | 1 => {
                    let at = e.wrapping_add(if kind == 0 { 0xe4 } else { 0xe8 });
                    mem.write(at, r32(fl(mem, at) + th));
                }
                2 => mem.write(e.wrapping_add(0xec), r32(f64::from(z) + th)),
                _ => {}
            }
            eax = if (0..=2).contains(&kind) {
                0
            } else {
                (kind - 2).cast_unsigned()
            };
        }
        if mem.read::<u32>(chr + 0x6fc) != 0 {
            let open = u32::from(mem.read::<i16>(chr + 0x4dc) > mem.read::<i16>(r(mem, MOUTH_TH)));
            let anim: u32 = mem.read(chr + 0x700);
            eax = (c.call)(mem, r(mem, TEX_ANIM_POS_SET), &[anim, 1, open]);
        }
        eax
    }

    /// `graphicShaderGetVertexInfo()` (`0x00643db0`) and
    /// `FFXVu0RotTransPers(out, m, v, n)` (`0x006ed840`).
    pub const GET_VERTEX_INFO: u32 = 0x0064_3db0;
    pub const ROT_TRANS_PERS: u32 = 0x006e_d840;
    /// Bytes of scratch `ch_draw_quad` needs: the original's frame
    /// (offset `0x88 - n` stands for `[ebp - n]`).
    pub const QUAD_SCRATCH: u32 = 0x88;

    /// `Ch_DrawQuad(m, p2, p3, p4, w, h, y, u0, v0, u1, v1)` (`0x00831d10`):
    /// appends a quad to the character shader's vertex lists (`+0x94` of
    /// `graphicShaderGetVertexInfo()`: vertex count `+4`, index count `+8`,
    /// positions (12 bytes) `+0xc`, normals (16) `+0x14`, texture
    /// coordinates (8) `+0x18`, `u16` indices `+0x1c`). Its corners (±h/2,
    /// `y`, ±w/2) go through `m` (`FFXVu0RotTransPers`), the coordinates
    /// are (u0, v0), (u0, v1), (u1, v1), (u1, v0), the normals (0, 0, 0,
    /// 1), the two triangles n, n+2, n+1 and n, n+3, n+2. Every float
    /// through x87. Returns `p2` (what EAX holds); `p3`, `p4` are unused.
    #[allow(clippy::too_many_arguments)]
    pub fn ch_draw_quad<M: Mem>(
        mem: &mut M,
        m: u32,
        p2: u32,
        size: [u32; 2],
        y: u32,
        uv: [u32; 4],
        scratch: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let ld = |b: u32| crate::x87::load(f32::from_bits(b));
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let half = |mem: &M, b: u32| {
            (f64::from(f32::from_bits(b)) * mem.read::<f64>(mem.rebase(0x00b9_2220))) as f32
        };
        let at = |n: u32| scratch + 0x88 - n;
        let (a, b) = (half(mem, size[0]), half(mem, size[1]));
        let y = ld(y);
        let corners = [
            [-b, y, -a, 1.0],
            [b, y, -a, 1.0],
            [b, y, a, 1.0],
            [-b, y, a, 1.0],
        ];
        for (k, c) in (0u32..).zip(corners) {
            for (i, v) in (0u32..).zip(c) {
                mem.write(at(0x64) + 0x10 * k + 4 * i, v);
            }
        }
        let [u0, v0, u1, v1] = uv.map(ld);
        for (k, (u, v)) in (0u32..).zip([(u0, v0), (u0, v1), (u1, v1), (u1, v0)]) {
            mem.write(at(0x24) + 4 * k, u);
            mem.write(at(0x14) + 4 * k, v);
        }
        let info = call(mem, mem.rebase(GET_VERTEX_INFO), &[]);
        let vi: u32 = mem.read(info.wrapping_add(0x94));
        let n: u32 = mem.read(vi.wrapping_add(4));
        let rtp = mem.rebase(ROT_TRANS_PERS);
        for k in 0..4u32 {
            let out = at(0x74);
            call(mem, rtp, &[out, m, at(0x64) + 0x10 * k, 1]);
            let j = n.wrapping_add(k);
            for i in 0..3 {
                let v = crate::x87::load(mem.read::<f32>(out + 4 * i));
                let base: u32 = mem.read(vi.wrapping_add(0xc));
                mem.write(base.wrapping_add(j.wrapping_mul(12)).wrapping_add(4 * i), v);
            }
            for (i, src) in [at(0x24), at(0x14)].into_iter().enumerate() {
                let v = crate::x87::load(mem.read::<f32>(src + 4 * k));
                let base: u32 = mem.read(vi.wrapping_add(0x18));
                #[allow(clippy::cast_possible_truncation)] // 0 or 1
                mem.write(
                    base.wrapping_add(j.wrapping_mul(8))
                        .wrapping_add(4 * i as u32),
                    v,
                );
            }
            for (i, v) in (0u32..).zip([0f32, 0.0, 0.0, 1.0]) {
                let base: u32 = mem.read(vi.wrapping_add(0x14));
                mem.write(base.wrapping_add(j.wrapping_mul(16)).wrapping_add(4 * i), v);
            }
        }
        let n: u32 = mem.read(vi.wrapping_add(4));
        let i: u32 = mem.read(vi.wrapping_add(8));
        for (k, d) in (0u32..).zip([0u32, 2, 1, 0, 3, 2]) {
            let base: u32 = mem.read(vi.wrapping_add(0x1c));
            #[allow(clippy::cast_possible_truncation)] // the low half, as stored
            mem.write(
                base.wrapping_add(i.wrapping_add(k).wrapping_mul(2)),
                n.wrapping_add(d) as u16,
            );
        }
        let i: u32 = mem.read(vi.wrapping_add(8));
        mem.write(vi.wrapping_add(8), i.wrapping_add(6));
        let n: u32 = mem.read(vi.wrapping_add(4));
        mem.write(vi.wrapping_add(4), n.wrapping_add(4));
        p2
    }

    /// The 13-step circle tables `Ch_InitCircleTable` fills: cosines at
    /// `0x013007ec`, sines at `0x01300860`.
    pub const CIRCLE_COS: u32 = 0x0130_07ec;
    pub const CIRCLE_SIN: u32 = 0x0130_0860;

    /// `Ch_InitCircleTable()` (`0x0082e5a0`): for i in 0..13, the angle
    /// `i × [0x00b59638] / [0x00b4b468]` (as `f32`) gives `_CIcos` and
    /// `_CIsin` (`ci`, results as stored) into the tables.
    pub fn ch_init_circle_table<M: Mem>(
        mem: &mut M,
        ci: &mut impl FnMut(&mut M, u32, &[f64]) -> f32,
    ) {
        let (mul, div) = (
            mem.read::<f64>(mem.rebase(0x00b5_9638)),
            mem.read::<f64>(mem.rebase(0x00b4_b468)),
        );
        for i in 0..13u32 {
            let a = f64::from(i) * mul / div;
            #[allow(clippy::cast_possible_truncation)] // fstp dword
            let a = f64::from(a as f32);
            let c = ci(mem, mem.rebase(CI_COS), &[a]);
            mem.write(mem.rebase(CIRCLE_COS) + 4 * i, c);
            let s = ci(mem, mem.rebase(CI_SIN), &[a]);
            mem.write(mem.rebase(CIRCLE_SIN) + 4 * i, s);
        }
    }

    /// `graphicUpdateShadowMap` (`0x00644a00`, 12 words) and `Ch_DrawQuad`
    /// (`0x00831d10`).
    pub const UPDATE_SHADOW_MAP: u32 = 0x0064_4a00;
    pub const DRAW_QUAD: u32 = 0x0083_1d10;
    /// Bytes of scratch `ch_draw_shadow_quad` needs: the original's frame
    /// (offset `0xa4 - n` stands for `[ebp - n]`).
    pub const SHADOW_QUAD_SCRATCH: u32 = 0xa4;

    /// `Ch_DrawShadowQuad(chr, -, p3)` (`0x0082e3e0`): the shadow's quad.
    /// Its matrix turns by the facing (`-+0x158` about y) and moves to the
    /// position `+0x7c`..`+0x84`. `graphicUpdateShadowMap(out, +0x830,
    /// +0x838, &(+0x694), +0x16c, &(+0x69c), &w, &h, len, +0x36c, +0x4,
    /// id)` (`len` the length of `+0x1e0`..`+0x1e8` scaled by
    /// `[0x00b0dba0]`, `_CIsqrt` through `ci`) gives its size and texture
    /// coordinates. `Ch_DrawQuad(m, p3, +0x4e4 × [0x00b59878] (a fixed
    /// [0x00b59880] for ids 7 and 0x6b), +0x354, w, h, +0x17c, uv)` draws
    /// it. Returns `p3`.
    pub fn ch_draw_shadow_quad<M: Mem>(
        mem: &mut M,
        chr: u32,
        p3: u32,
        scratch: u32,
        c: &mut NeckCalls<'_, M>,
    ) -> u32 {
        let r = |mem: &M, a: u32| mem.rebase(a);
        let fl = |mem: &M, a: u32| f64::from(mem.read::<f32>(a));
        let ld = |mem: &M, a: u32| crate::x87::load(mem.read::<f32>(a));
        #[allow(clippy::cast_possible_truncation)] // fstp dword
        let r32 = |x: f64| x as f32;
        let at = |n: u32| scratch + 0xa4 - n;
        let (m, n) = (at(0x44), at(0x84));
        (c.call)(mem, r(mem, UNIT_MATRIX), &[m]);
        let yaw = (-ld(mem, chr + 0x158)).to_bits();
        (c.call)(mem, r(mem, ROT_MATRIX_Y), &[m, m, yaw]);
        (c.call)(mem, r(mem, UNIT_MATRIX), &[n]);
        for (to, from) in [(4u32, 0x7cu32), (0x14, 0x80), (0x24, 0x84)] {
            let v = ld(mem, chr + from);
            mem.write(n + to, v);
        }
        (c.call)(mem, r(mem, MUL_MATRIX), &[m, n, m]);
        for (to, from) in [(0x14u32, 0x694u32), (0x10, 0x16c), (0xc, 0x69c)] {
            let v = ld(mem, chr + from);
            mem.write(at(to), v);
        }
        let id = u32::from(mem.read::<u16>(chr));
        let mut half = r32(fl(mem, chr + 0x4e4) * mem.read::<f64>(r(mem, 0x00b5_9878)));
        if id == 7 || id == 0x6b {
            half = ld(mem, r(mem, 0x00b5_9880));
        }
        mem.write(at(0x8c), half);
        let k = mem.read::<f64>(r(mem, 0x00b0_dba0));
        let (a, b, cc) = (
            fl(mem, chr + 0x1e4) * k,
            fl(mem, chr + 0x1e0) * k,
            fl(mem, chr + 0x1e8) * k,
        );
        let len = (c.ci)(mem, r(mem, CI_SQRT), &[(b * b + a * a) + cc * cc]);
        mem.write(at(0x88), len);
        let args = [
            at(0xa4),
            mem.read(chr + 0x830),
            mem.read(chr + 0x838),
            at(0x14),
            ld(mem, at(0x10)).to_bits(),
            at(0xc),
            at(0x90),
            at(0x94),
            len.to_bits(),
            ld(mem, chr + 0x36c).to_bits(),
            mem.read(chr + 4),
            id,
        ];
        let uv = (c.call)(mem, r(mem, UPDATE_SHADOW_MAP), &args);
        let quad = [
            m,
            p3,
            ld(mem, at(0x8c)).to_bits(),
            ld(mem, chr + 0x354).to_bits(),
            ld(mem, at(0x90)).to_bits(),
            ld(mem, at(0x94)).to_bits(),
            ld(mem, chr + 0x17c).to_bits(),
            mem.read(uv),
            mem.read(uv.wrapping_add(4)),
            mem.read(uv.wrapping_add(8)),
            mem.read(uv.wrapping_add(0xc)),
        ];
        (c.call)(mem, r(mem, DRAW_QUAD), &quad);
        p3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloth_cases() {
        assert_eq!(cloth_fast(1), (10, true));
        assert_eq!(cloth_fast(3), (20, false));
        assert_eq!(cloth_fast(4), (1, false));
    }
}
