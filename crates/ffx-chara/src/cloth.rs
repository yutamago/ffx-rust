//! Cloth: the per-character exceptions of the PC port (which characters,
//! models and motions get cloth, a larger collision radius or anchor
//! vertices), and resetting a character's cloth vertices.
//!
//! The exceptions are the original's literal tables: character ids,
//! model numbers, motion ids and map numbers in its code.

/// `Ch_UseAnchorVertex(chr, model, v)`'s table: whether vertex group `m`
/// (`v % 100`) of `model` of character `id` is anchored.
#[must_use]
pub const fn anchor_vertex(id: u16, model: u16, m: u16) -> bool {
    (id == 0x66 && model == 2 && (m == 10 || m == 11 || m == 8))
        || (id == 4 && (m == 10 || m == 9))
        || (id == 0x68 && m == 10)
        || (id == 7 && m == 6)
        || (id == 3 && ((model == 1 && m == 1) || (model == 2 && (m == 1 || m == 2))))
        || (id == 0x67 && model == 1 && (m == 1 || m == 0))
}

/// The originals, on the characters.
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::anchor_vertex;
    use crate::handle::bridge::{CHR_ARRAY, CHR_COUNT, each_active};
    use crate::layout as l;
    use crate::parts::bridge::ch_reserve_calc_norm;

    /// The current map (field) number.
    pub const MAP: Global<u32> = Global::new(0x012f_bc40);
    /// `Ch_UseLargeCollisionRad`'s scales: 1.05, 1.165, 1.2, 1.25, 1.03.
    pub const RAD_1_05: Global<f32> = Global::new(0x00b5_9180);
    /// 1.165.
    pub const RAD_1_165: Global<f32> = Global::new(0x00b5_9184);
    /// 1.2.
    pub const RAD_1_2: Global<f32> = Global::new(0x00b4_7c88);
    /// 1.25.
    pub const RAD_1_25: Global<f32> = Global::new(0x00b5_9188);
    /// 1.03.
    pub const RAD_1_03: Global<f32> = Global::new(0x00b5_917c);
    /// `Ch_ClothSetAtariJimenCheck`'s case by `id - 2` (`0x6a` bytes in its
    /// code): 0 by motion and map, 1 by map, 2 none.
    pub const ATARI_JIMEN_CASES: Global<u8> = Global::new(0x0082_48ac);
    /// [`l::FLAGS`]: cloth set up (`Ch_ClothReset2` needs it).
    pub const F_CLOTH_READY: u32 = 1 << 21;
    /// [`l::FLAGS`]: cloth reset (`Ch_ClothReset2` sets it; bound
    /// characters with it are skipped by `Ch_ClothResetRelation`).
    pub const F_CLOTH_RESET: u32 = 1 << 23;
    /// [`l::DIRTY`] bit 3: cloth ground (atari jimen) check.
    pub const D_ATARI_JIMEN: u32 = 1 << 3;

    fn id(mem: &impl Mem, chr: u32) -> u16 {
        mem.read(chr + l::ID)
    }

    /// `Ch_UseAnchorVertex(chr, model, v)` (`0x008316d0`).
    pub fn ch_use_anchor_vertex(mem: &impl Mem, chr: u32, model: u32, v: u32) -> u32 {
        #[allow(clippy::cast_possible_truncation)] // the low halves
        let (model, m) = (model as u16, (v as u16) % 100);
        u32::from(anchor_vertex(id(mem, chr), model, m))
    }

    /// `Ch_UseLargeCollisionRad(chr, model, part)` (`0x00824b40`): the
    /// collision radius scale, 1 but for some characters, models and maps.
    pub fn ch_use_large_collision_rad(mem: &impl Mem, chr: u32, a: u32, b: u32) -> f32 {
        let id = u32::from(id(mem, chr));
        let map = MAP.get(mem);
        let r = |g: Global<f32>| crate::x87::load(g.get(mem));
        if id == 6 && a == 3 && b == 0 {
            return if map == 0x15d || map == 0x84 {
                1.0
            } else {
                r(RAD_1_05)
            };
        }
        if id == 3 {
            if a == 4 && b == 0 && map == 0x107 {
                return r(RAD_1_165);
            }
            if b == 1 && (a == 1 || a == 2) {
                return 1.0;
            }
        }
        if id == 0x67 && a == 1 && b == 0 {
            let motion: u32 = mem.read(chr + l::MOTION_ID);
            return if motion == 0x0067_100b || motion == 0x0067_100c {
                r(RAD_1_2)
            } else {
                1.0
            };
        }
        if (id == 7 && a == 0 && b == 3) || (id == 0x6b && a == 2 && b <= 1) {
            return r(RAD_1_25);
        }
        if map == 0x17d && id == 0x66 && b <= 1 {
            return 1.0;
        }
        r(RAD_1_03)
    }

    /// `Ch_ShouldDisableClothForMotion(chr, model, part)` (`0x00824a60`):
    /// 1 where cloth is off (some characters in some motions, models,
    /// maps, or spell effects).
    pub fn ch_should_disable_cloth_for_motion(mem: &impl Mem, chr: u32, a: u32, b: u32) -> u32 {
        let id = id(mem, chr);
        #[allow(clippy::cast_possible_truncation)] // 16-bit compares
        let (a, b) = (a as u16, b as u16);
        let motion: u32 = mem.read(chr + l::MOTION_ID);
        if id == 0x66
            && a == 2
            && b.wrapping_sub(10) <= 3
            && matches!(motion, 0x0066_1032 | 0x0066_1036 | 0x0066_1037)
        {
            return 1;
        }
        let map = MAP.get(mem);
        if (id == 4 && a == 0 && b == 10 && (map == 0x15d || map == 0x84))
            || (mem.read::<u8>(chr + 0x87c) != 0 && a == 0 && b == 1)
            || (id == 2 && a == 2 && b.wrapping_sub(7) <= 3 && map == 0x3b)
            || (id == 0x20f1 && map == 0x39 && b == 0)
            || (id == 0x3003 && ffx_system::instance::bridge::magic_id(mem) == 0xa0)
        {
            return 1;
        }
        0
    }

    /// `Ch_ClothSetAtariJimenCheck(chr)` (`0x00824800`): the ground check
    /// bit, by character id, motion and map (then a debug print, a no-op
    /// in this build).
    pub fn ch_cloth_set_atari_jimen_check(mem: &mut impl Mem, chr: u32) {
        let d: u32 = mem.read(chr + l::DIRTY);
        let mut d = d & !D_ATARI_JIMEN;
        mem.write(chr + l::DIRTY, d);
        let i = u32::from(id(mem, chr)).wrapping_sub(2);
        let case = if i > 0x69 {
            2
        } else {
            ATARI_JIMEN_CASES.ptr(mem).at(i).get(mem)
        };
        let map = MAP.get(mem);
        match case {
            0 => {
                let m: u32 = mem.read(chr + l::MOTION_ID);
                #[allow(clippy::cast_possible_wrap)] // signed compares
                let s = m as i32;
                if s >= 0x2113d && (s <= 0x2113f || m == 0x0066_10cc) {
                    d |= D_ATARI_JIMEN;
                    mem.write(chr + l::DIRTY, d);
                }
                if map == 0xa1 {
                    let v: u32 = mem.read(chr + l::DIRTY);
                    mem.write(chr + l::DIRTY, v | D_ATARI_JIMEN);
                }
            }
            1 if map == 0xb7 || map == 0x135 => {
                mem.write(chr + l::DIRTY, d | D_ATARI_JIMEN);
            }
            _ => {}
        }
    }

    /// `Ch_ClothReset2(chr)` (`0x00824670`): every cloth of every model
    /// back to the model's vertices (positions, zero velocities, previous
    /// positions), then [`F_CLOTH_RESET`] and `Ch_ClothResetRelation`.
    /// Nothing unless [`F_CLOTH_READY`]; a model without a cloth buffer
    /// stops it.
    pub fn ch_cloth_reset2(mem: &mut impl Mem, chr: u32) {
        let flags: u32 = mem.read(chr + l::FLAGS);
        if flags & F_CLOTH_READY == 0 {
            return;
        }
        let mut m = 0u32;
        while m < u32::from(
            mem.read::<u16>(mem.read::<u32>(chr + l::MODEL_HEADER) + l::HEADER_MODELS),
        ) {
            let models: u32 = mem.read(chr + l::MODELS);
            let model = models.wrapping_add(m.wrapping_mul(l::MODEL_SIZE));
            let data: u32 = mem.read(model);
            if mem.read::<u16>(data + 0x18) != 0 {
                let mut buf: u32 = mem.read(model + 0xc);
                if buf == 0 {
                    return;
                }
                let mut c = 0u32;
                while c < u32::from(mem.read::<u16>(data + 0x18)) {
                    let e = mem
                        .read::<u32>(data + 0x1c)
                        .wrapping_add(c.wrapping_mul(0x24));
                    let mut idx: u32 = mem.read(e + 8);
                    #[allow(clippy::cast_sign_loss)] // movsx
                    let k = (i32::from(mem.read::<i8>(e + 0x13))
                        + i32::from(mem.read::<i16>(e + 0x1c))) as u32;
                    let mut j = 0i32;
                    while j < i32::from(mem.read::<i16>(e + 0x1c)) {
                        let v = u32::from(mem.read::<u16>(idx));
                        idx = idx.wrapping_add(2);
                        let verts: u32 = mem.read(model + 8);
                        let src = verts.wrapping_add(v * 16);
                        let ju = j.cast_unsigned();
                        let pos = buf.wrapping_add(16u32.wrapping_mul(ju));
                        let w: [u32; 4] = mem.read(src);
                        mem.write(pos, w);
                        let vel = buf.wrapping_add(16u32.wrapping_mul(k.wrapping_add(ju)));
                        mem.write(vel, [0u32; 3]);
                        let w: [u32; 4] = mem.read(pos);
                        mem.write(
                            buf.wrapping_add(
                                16u32.wrapping_mul(k.wrapping_mul(2).wrapping_add(ju)),
                            ),
                            w,
                        );
                        j += 1;
                    }
                    buf = buf.wrapping_add(48u32.wrapping_mul(k));
                    c += 1;
                }
            }
            m += 1;
        }
        let w: u32 = mem.read(chr + l::FLAGS);
        mem.write(chr + l::FLAGS, w | F_CLOTH_RESET);
        ch_cloth_reset_relation(mem, chr);
    }

    /// `Ch_ClothReset(chr)` (`0x00824650`): `Ch_ReserveCalcNorm2`, then
    /// `Ch_ClothReset2`.
    pub fn ch_cloth_reset(mem: &mut impl Mem, chr: u32) {
        ch_reserve_calc_norm(mem, chr, l::F_RESERVE_CALC_NORM2);
        ch_cloth_reset2(mem, chr);
    }

    /// `Ch_ClothResetRelation(parent)` (`0x00833860`): `Ch_ClothReset` on
    /// the active characters bound to `parent` whose cloth is not reset
    /// yet.
    pub fn ch_cloth_reset_relation(mem: &mut impl Mem, parent: u32) {
        let mut c = CHR_ARRAY.get(mem);
        loop {
            #[allow(clippy::cast_sign_loss)]
            let end = CHR_ARRAY
                .get(mem)
                .wrapping_add((CHR_COUNT.get(mem) as u32).wrapping_mul(l::SIZE));
            if c == end {
                break;
            }
            if mem.read::<u8>(c + l::ACTIVE) != 0
                && mem.read::<u32>(c + l::FLAGS) & F_CLOTH_RESET == 0
                && c != parent
                && mem.read::<u32>(c + l::BIND_PARENT) == parent
            {
                ch_cloth_reset(mem, c);
            }
            c = c.wrapping_add(l::SIZE);
        }
    }

    /// `Ch_ClothResetAll()` (`0x00833800`).
    pub fn ch_cloth_reset_all(mem: &mut impl Mem) {
        each_active(mem, |m, c| ch_cloth_reset(m, c));
    }

    /// The cloth flip (−1.0, `f32`) on x and y of `Ch_UpdateClothMatrix`.
    pub const CLOTH_FLIP: Global<f32> = Global::new(0x00b9_2340);
    /// `1000.0` (`double`): the translation scale.
    pub const K_1000: Global<f64> = Global::new(0x00b0_dba0);

    /// `Ch_UpdateClothMatrix(chr)` (`0x008314c0`): with the cloth set up
    /// ([`F_CLOTH_READY`]) and a render cloth at `+0x834`: its `+8` takes
    /// the hide byte; when its state (`+0xc`) is not 0 it becomes 2 and
    /// each cloth mesh of each list (`+0x14` lists from `+0x30`, each a
    /// count and mesh pointers) gets its model's matrix: the base
    /// element's global matrix with the translation × 1000, through the
    /// root matrix, flipped on x and y. Meshes of one model in a row
    /// share it. Unless the state was 2, each mesh's vectors (`+0x44`
    /// count at `+0x4c`) lose their x, y, z first.
    ///
    /// A first mesh of model `0xffff` would get the original's
    /// uninitialised local matrix; here it gets zeros.
    pub fn ch_update_cloth_matrix(mem: &mut impl Mem, chr: u32) {
        use ffx_system::math::mul4x4;
        let rc: u32 = mem.read(chr + 0x834);
        if rc == 0 || mem.read::<u32>(chr + l::FLAGS) & F_CLOTH_READY == 0 {
            return;
        }
        let mut flip = ffx_ps2::vu0::identity();
        let c = crate::x87::load(CLOTH_FLIP.get(mem));
        flip[0] = c;
        flip[5] = c;
        let hide = i32::from(mem.read::<i8>(chr + l::HIDE));
        mem.write(rc + 8, hide);
        let state: u32 = mem.read(rc + 0xc);
        if state == 0 {
            return;
        }
        mem.write(rc + 0xc, 2u32);
        let mut cached = 0xffffu16;
        let mut m = [0f32; 16];
        let mut i = 0u32;
        while i < mem.read::<u32>(rc + 0x14) {
            let list: u32 = mem.read(rc + 0x30 + 4 * i);
            if list != 0 {
                let mut k = 0u32;
                while k < mem.read::<u32>(list) {
                    let mesh: u32 = mem.read(list + 4 + 4 * k);
                    if mesh != 0 {
                        if state != 2 {
                            let v: u32 = mem.read(mesh + 0x4c);
                            let mut j = 0u32;
                            while j < mem.read::<u32>(mesh + 0x44) {
                                mem.write(v.wrapping_add(16 * j), [0f32; 3]);
                                j += 1;
                            }
                        }
                        let model: u16 = mem.read(mesh + 0x40);
                        if model != cached {
                            let models: u32 = mem.read(chr + l::MODELS);
                            let data: u32 =
                                mem.read(models.wrapping_add(u32::from(model) * l::MODEL_SIZE));
                            let e = u32::from(mem.read::<u16>(data));
                            let g = mem
                                .read::<u32>(chr + l::ELEMS)
                                .wrapping_add(0x88)
                                .wrapping_add(e * l::ELEM_SIZE);
                            m = mem.read(g);
                            let k1000 = crate::x87::Reg::constant(K_1000.get(mem));
                            m[12] = crate::x87::Reg::load(m[12]).mul_st(k1000).store();
                            m[13] = crate::x87::Reg::load(m[13]).mul_st(k1000).store();
                            m[14] = k1000.mul_m32(m[14]).store();
                            let root: [f32; 16] = mem.read(chr + 0x1d0);
                            m = mul4x4(&root, &m);
                            m = mul4x4(&flip, &m);
                            cached = model;
                        }
                        mem.write(mesh, m);
                    }
                    k += 1;
                }
            }
            i += 1;
        }
    }

    // ---- round 10: the cloth meshes' rest poses

    /// `FFXVu0CopyMatrix(dst, src)` (`0x006ed360`).
    pub const VU0_COPY_MATRIX: u32 = 0x006e_d360;

    /// `Ch_GetRestPoseMatrices(chr)` (`0x00826ba0`): without a cloth (flag
    /// bit 21 of `+0x194`) each model's cloth lists (`+0x2c`, `+0x30` of
    /// the 0x38-byte models at `+0x1bc`) are cleared. With one, each model
    /// with cloth meshes (`u16` `+0x18` of its data `+0`) gets a list of
    /// that many pointers (`user_malloc`, `memclear`) and each entry a
    /// 0x4c-byte block; when the header has bones (`u16` `+8` of `+0x1c0`)
    /// the block takes the model's bone matrix (`FFXVu0CopyMatrix` from
    /// the bone at `u16` `+0` of the data, 0x160-byte bones at `+0x328`,
    /// matrix at `+0x88`) with its translation times 1000. Returns 0.
    pub fn ch_get_rest_pose_matrices<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let count = |mem: &M| {
            let h: u32 = mem.read(chr + 0x1c0);
            u32::from(mem.read::<u16>(h + 6))
        };
        let model = |mem: &M, i: u32| mem.read::<u32>(chr + 0x1bc).wrapping_add(0x38 * i);
        let meshes = |mem: &M, i: u32| {
            let d: u32 = mem.read(model(mem, i));
            u32::from(mem.read::<u16>(d + 0x18))
        };
        if mem.read::<u32>(chr + 0x194) & 0x20_0000 == 0 {
            let mut i = 0;
            while i < count(mem) {
                mem.write(model(mem, i) + 0x2c, 0u32);
                mem.write(model(mem, i) + 0x30, 0u32);
                i += 1;
            }
            return 0;
        }
        let (malloc, clear) = (
            mem.rebase(crate::handle::bridge::USER_MALLOC),
            mem.rebase(crate::handle::bridge::MEMCLEAR),
        );
        let mut i = 0;
        while i < count(mem) {
            let n = meshes(mem, i);
            if n != 0 {
                if mem.read::<u32>(model(mem, i) + 0x2c) == 0 {
                    let p = call(mem, malloc, &[n << 2]);
                    mem.write(model(mem, i) + 0x2c, p);
                    let p: u32 = mem.read(model(mem, i) + 0x2c);
                    let n = meshes(mem, i);
                    call(mem, clear, &[p, n << 2]);
                }
                let mut k = 0;
                while k < meshes(mem, i) {
                    let list: u32 = mem.read(model(mem, i) + 0x2c);
                    if mem.read::<u32>(list + 4 * k) == 0 {
                        let b = call(mem, malloc, &[0x4c]);
                        let list: u32 = mem.read(model(mem, i) + 0x2c);
                        mem.write(list + 4 * k, b);
                        let b: u32 = mem.read(list + 4 * k);
                        call(mem, clear, &[b, 0x4c]);
                    }
                    let list: u32 = mem.read(model(mem, i) + 0x2c);
                    let b: u32 = mem.read(list + 4 * k);
                    let hdr: u32 = mem.read(chr + 0x1c0);
                    if mem.read::<u16>(hdr + 8) != 0 {
                        let d: u32 = mem.read(model(mem, i));
                        let bone = u32::from(mem.read::<u16>(d));
                        let src = mem.read::<u32>(chr + 0x328) + 0x88 + bone.wrapping_mul(0x160);
                        call(mem, mem.rebase(VU0_COPY_MATRIX), &[b, src]);
                        for off in [0x30, 0x34, 0x38] {
                            let v = crate::x87::load(mem.read::<f32>(b + off));
                            #[allow(clippy::cast_possible_truncation)] // f32 stores
                            mem.write(b + off, (f64::from(v) * 1000.0) as f32);
                        }
                    }
                    k += 1;
                }
            }
            i += 1;
        }
        0
    }

    /// `Ch_ClothMeshDestroy(chr)` (`0x00826260`): frees what
    /// [`ch_get_rest_pose_matrices`] and the cloth mapping made: each
    /// mesh block's `+0x44` and `+0x48` lists and the block, then each
    /// model's `+0x2c` and `+0x30` lists (`user_free_thunk`; the pointers
    /// stay). Returns 0.
    pub fn ch_cloth_mesh_destroy<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let free = mem.rebase(crate::chr::bridge::USER_FREE);
        let count = |mem: &M| {
            let h: u32 = mem.read(chr + 0x1c0);
            u32::from(mem.read::<u16>(h + 6))
        };
        let model = |mem: &M, i: u32| mem.read::<u32>(chr + 0x1bc).wrapping_add(0x38 * i);
        let mut i = 0;
        while i < count(mem) {
            let m = model(mem, i);
            if mem.read::<u32>(m + 0x2c) != 0 {
                let mut k = 0u32;
                loop {
                    let m = model(mem, i);
                    let d: u32 = mem.read(m);
                    if k >= u32::from(mem.read::<u16>(d + 0x18)) {
                        break;
                    }
                    let list: u32 = mem.read(m + 0x2c);
                    let b: u32 = mem.read(list + 4 * k);
                    if b != 0 {
                        let p: u32 = mem.read(b + 0x44);
                        call(mem, free, &[p]);
                        for off in [0x48u32, 0] {
                            let list: u32 = mem.read(model(mem, i) + 0x2c);
                            let b: u32 = mem.read(list + 4 * k);
                            let p: u32 = if off == 0 { b } else { mem.read(b + off) };
                            call(mem, free, &[p]);
                        }
                    }
                    k += 1;
                }
                for off in [0x2c, 0x30] {
                    let p: u32 = mem.read(model(mem, i) + off);
                    call(mem, free, &[p]);
                }
            }
            i += 1;
        }
        0
    }

    /// `Cache_Exist_ClothMeshData(kind, id)` (`0x008407d0`).
    pub const CACHE_EXIST_CLOTH_MESH: u32 = 0x0084_07d0;

    /// `Ch_ClothMeshMapping(chr)` (`0x00826340`): the cloth meshes from
    /// the cached cloth mesh data of the character's id (`u16` `+0`).
    /// Without data: each model's mesh blocks and list freed, its `+0x2c`
    /// and `+0x30` cleared; returns 0. With data: the bounding box (6 `f32`
    /// × 1000 into `+0x840`..`+0x854`) and its inverse extents
    /// (`1 / (max - min)` into `+0x858`..`+0x860`); then from data `+0x18` records
    /// (`u16` model, mesh, triangle count; 8 bytes, then 0x20 bytes per
    /// triangle: three `u16` vertices and a `u16` value) up to model
    /// 0xffff: the model's mesh list (`+0x2c`, `u16` `+0x18` of its data
    /// entries) and vertex flags (`+0x30`, `u16` `+4` bytes, cleared, with
    /// `+0x34` = 0) made when missing, the mesh's 0x4c-byte block too; the
    /// block's count `+0x40`, its vertex triples `+0x44` (6 bytes each) and
    /// values `+0x48` (2 bytes each) made when missing and filled, each
    /// vertex flagged 1. Returns 1.
    #[allow(clippy::too_many_lines)] // one pass over the records
    pub fn ch_cloth_mesh_mapping<M: Mem>(
        mem: &mut M,
        chr: u32,
        call: &mut impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let id = u32::from(mem.read::<u16>(chr));
        let data = call(mem, mem.rebase(CACHE_EXIST_CLOTH_MESH), &[1, id]);
        let malloc = mem.rebase(crate::handle::bridge::USER_MALLOC);
        let clear = mem.rebase(crate::handle::bridge::MEMCLEAR);
        let free = mem.rebase(crate::chr::bridge::USER_FREE);
        let model = |mem: &M, i: u32| mem.read::<u32>(chr + 0x1bc).wrapping_add(0x38 * i);
        let mdata = |mem: &M, i: u32| -> u32 { mem.read(model(mem, i)) };
        if data == 0 {
            let count = |mem: &M| {
                let h: u32 = mem.read(chr + 0x1c0);
                u32::from(mem.read::<u16>(h + 6))
            };
            let mut i = 0;
            while i < count(mem) {
                if mem.read::<u32>(model(mem, i) + 0x2c) != 0 {
                    let mut k = 0;
                    while k < u32::from(mem.read::<u16>(mdata(mem, i) + 0x18)) {
                        let list: u32 = mem.read(model(mem, i) + 0x2c);
                        let b: u32 = mem.read(list + 4 * k);
                        if b != 0 {
                            call(mem, free, &[b]);
                        }
                        k += 1;
                    }
                    let list: u32 = mem.read(model(mem, i) + 0x2c);
                    call(mem, free, &[list]);
                }
                mem.write(model(mem, i) + 0x2c, 0u32);
                mem.write(model(mem, i) + 0x30, 0u32);
                i += 1;
            }
            return 0;
        }
        let k1000 = |v: f32| f64::from(crate::x87::load(v)) * 1000.0;
        for k in 0..6 {
            #[allow(clippy::cast_possible_truncation)] // f32 stores
            let v = k1000(mem.read(data + 4 * k)) as f32;
            mem.write(chr + 0x840 + 4 * k, v);
        }
        for k in 0..3 {
            let max = f64::from(crate::x87::load(mem.read::<f32>(chr + 0x84c + 4 * k)));
            let min = f64::from(crate::x87::load(mem.read::<f32>(chr + 0x840 + 4 * k)));
            #[allow(clippy::cast_possible_truncation)] // f32 stores
            mem.write(chr + 0x858 + 4 * k, (1.0 / (max - min)) as f32);
        }
        let mut p = data.wrapping_add(0x18);
        loop {
            let m = u32::from(mem.read::<u16>(p));
            let mesh = u32::from(mem.read::<u16>(p + 2));
            let count: u16 = mem.read(p + 4);
            if m == 0xffff {
                return 1;
            }
            if mem.read::<u32>(model(mem, m) + 0x2c) == 0 {
                let n = u32::from(mem.read::<u16>(mdata(mem, m) + 0x18)) << 2;
                let list = call(mem, malloc, &[n]);
                mem.write(model(mem, m) + 0x2c, list);
                let list: u32 = mem.read(model(mem, m) + 0x2c);
                let n = u32::from(mem.read::<u16>(mdata(mem, m) + 0x18)) << 2;
                call(mem, clear, &[list, n]);
            }
            if mem.read::<u32>(model(mem, m) + 0x30) == 0 {
                let n = u32::from(mem.read::<u16>(mdata(mem, m) + 4));
                let flags = call(mem, malloc, &[n]);
                mem.write(model(mem, m) + 0x30, flags);
                mem.write(model(mem, m) + 0x34, 0u32);
                let mut i = 0;
                while i < u32::from(mem.read::<u16>(mdata(mem, m) + 4)) {
                    let flags: u32 = mem.read(model(mem, m) + 0x30);
                    mem.write(flags.wrapping_add(i), 0u8);
                    i += 1;
                }
            }
            let list: u32 = mem.read(model(mem, m) + 0x2c);
            if mem.read::<u32>(list.wrapping_add(4 * mesh)) == 0 {
                let b = call(mem, malloc, &[0x4c]);
                let list: u32 = mem.read(model(mem, m) + 0x2c);
                mem.write(list.wrapping_add(4 * mesh), b);
                let b: u32 = mem.read(list.wrapping_add(4 * mesh));
                call(mem, clear, &[b, 0x4c]);
            }
            let list: u32 = mem.read(model(mem, m) + 0x2c);
            let b: u32 = mem.read(list.wrapping_add(4 * mesh));
            mem.write(b + 0x40, count);
            if mem.read::<u32>(b + 0x44) == 0 {
                let n = i32::from(count.cast_signed()) * 6;
                let v = call(mem, malloc, &[n.cast_unsigned()]);
                mem.write(b + 0x44, v);
            }
            if mem.read::<u32>(b + 0x48) == 0 {
                let n = i32::from(mem.read::<i16>(b + 0x40)) * 2;
                let v = call(mem, malloc, &[n.cast_unsigned()]);
                mem.write(b + 0x48, v);
            }
            p = p.wrapping_add(8);
            let mut t = 0i32;
            while t < i32::from(mem.read::<i16>(b + 0x40)) {
                for k in 0..3 {
                    let v: u16 = mem.read(p + 2 * k);
                    let tri_now: u32 = mem
                        .read::<u32>(b + 0x44)
                        .wrapping_add((6 * t).cast_unsigned());
                    mem.write(tri_now + 2 * k, v);
                }
                for k in 0..3 {
                    let tri_now: u32 = mem
                        .read::<u32>(b + 0x44)
                        .wrapping_add((6 * t).cast_unsigned());
                    let vtx = i32::from(mem.read::<i16>(tri_now + 2 * k));
                    let flags: u32 = mem.read(model(mem, m) + 0x30);
                    mem.write(flags.wrapping_add(vtx.cast_unsigned()), 1u8);
                }
                let value: u16 = mem.read(p + 6);
                let values: u32 = mem.read(b + 0x48);
                mem.write(values.wrapping_add((2 * t).cast_unsigned()), value);
                p = p.wrapping_add(0x20);
                t += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchors() {
        assert!(anchor_vertex(0x66, 2, 11));
        assert!(!anchor_vertex(0x66, 1, 11));
        assert!(anchor_vertex(0x67, 1, 0));
        assert!(!anchor_vertex(0x67, 2, 0));
        assert!(anchor_vertex(4, 9, 9));
    }
}
