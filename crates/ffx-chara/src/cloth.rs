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
