//! The leftovers of list `phyre-render-b` (M5 stage 1, round "left2"):
//! `PDeferredLightingNVN::initializeShaders` (`0x00a1a4f0`), a table of
//! shader parameters looked up by name after the base class's common
//! shaders; and the two-sampler `initializeShaders` shape, one generic
//! port with a table of its copies ([`SAMPLER_SHADERS`]: the screen-space
//! reflection's, and the motion blur's from list `phyre-framework-input`).

use ffx_graphic::host::{Conv, Host};

/// `ShaderParams_FindByName(name)` (thiscall on the effect).
pub const FIND_PARAM: u32 = 0x0056_cbf0;
/// `PDeferredLightingBase::initializeCommonShaders(a, effect)` (thiscall).
pub const INIT_COMMON: u32 = 0x00a1_82f0;
/// What `initializeShaders` returns without its two buffers.
pub const NOT_READY: u32 = 0x15;

/// The parameters `0x00a1a4f0` looks up, in its order: the name and the
/// field of the object that takes the parameter.
pub const NVN_PARAMS: [(u32, u32); 22] = [
    (0x00b4_25d8, 0x3fd8),
    (0x00b7_6414, 0x3fdc),
    (0x00b7_6424, 0x3fe0),
    (0x00b7_6430, 0x3fe4),
    (0x00b7_643c, 0x3fe8),
    (0x00b7_6448, 0x3fec),
    (0x00b7_6458, 0x3ff0),
    (0x00b7_646c, 0x3ff4),
    (0x00b7_6224, 0x3ff8),
    (0x00b7_6480, 0x3ffc),
    (0x00b7_6498, 0x4000),
    (0x00b7_64b4, 0x4024),
    (0x00b7_64cc, 0x4008),
    (0x00b7_64e0, 0x4004),
    (0x00b7_6508, 0x400c),
    (0x00b7_6528, 0x4010),
    (0x00b7_6540, 0x4014),
    (0x00b7_6558, 0x4028),
    (0x00b7_6578, 0x4018),
    (0x00b7_6594, 0x401c),
    (0x00b7_65b8, 0x403c),
    (0x00b7_65cc, 0x4020),
];

/// One copy of the post-processing `initializeShaders(cluster, effect)`
/// shape that binds two samplers ([`bridge::init_sampler_shaders`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SamplerShaders {
    /// The original's address.
    pub at: u32,
    /// The guard's field (`[+G] + 0x10` takes the instance array).
    pub base: u32,
    /// 0: the guard is the field as it is; else a cdecl `(cluster)`
    /// whose answer is stored there first.
    pub guard_call: u32,
    /// The two sampler states.
    pub samplers: [u32; 2],
    /// 0, or the field of a parameter that takes `{1, 1, 0, 0}` before
    /// the samplers are registered.
    pub vec_param: u32,
    /// Where the effect's constant buffer goes.
    pub buffer: u32,
    /// The parameters looked up by name and their fields.
    pub params: &'static [(u32, u32)],
}

/// The copies: `PMotionBlurBase::initializeCommonShaders`,
/// `PScreenSpaceReflectionBase::initializeShaders` and
/// `PGlowBase::initializeShaders`.
pub const SAMPLER_SHADERS: [SamplerShaders; 3] = [
    SamplerShaders {
        at: 0x00a1_8c80,
        base: 0xcdc,
        guard_call: 0,
        samplers: [0xce0, 0xd04],
        vec_param: 0,
        buffer: 0xd28,
        params: &[
            (0x00b4_25d8, 0xd2c),
            (0x00b7_61b8, 0xd30),
            (0x00b7_67e4, 0xd34),
            (0x00b7_66ac, 0xd3c),
            (0x00b7_66c0, 0xd40),
            (0x00b7_67f4, 0xd38),
            (0x00b7_6804, 0xd44),
            (0x00b7_6824, 0xd48),
        ],
    },
    SamplerShaders {
        at: 0x00a1_ae00,
        base: 0x1c,
        guard_call: 0,
        samplers: [0x20, 0x44],
        vec_param: 0,
        buffer: 0x74,
        params: &[
            (0x00b4_25d8, 0x78),
            (0x00b7_61b8, 0x7c),
            (0x00b7_6c18, 0x80),
            (0x00b7_61a4, 0x84),
            (0x00b7_6c30, 0x88),
            (0x00b7_6c40, 0x8c),
            (0x00b7_6c54, 0x90),
        ],
    },
    SamplerShaders {
        at: 0x00a1_a830,
        base: 0x5c,
        guard_call: 0x00a0_6120,
        samplers: [0x14, 0x38],
        vec_param: 0x84,
        buffer: 0x60,
        params: &[
            (0x00b7_6a8c, 0x74),
            (0x00b4_25d8, 0x78),
            (0x00b7_6a98, 0x7c),
            (0x00b4_94d8, 0x80),
            (0x00b4_25e4, 0x84),
            (0x00b7_6aac, 0x88),
            (0x00b7_6ac0, 0x8c),
        ],
    },
];

/// The [`SAMPLER_SHADERS`] copy at `at` (a compile error when missing).
#[must_use]
pub const fn sampler_shaders_of(at: u32) -> SamplerShaders {
    let mut i = 0;
    while i < SAMPLER_SHADERS.len() {
        if SAMPLER_SHADERS[i].at == at {
            return SAMPLER_SHADERS[i];
        }
        i += 1;
    }
    panic!("not in the table");
}

/// The shape's callees: the effect's constant buffer `(cluster, 0, 0)`
/// (thiscall on the effect), `memcpy` (cdecl), the cluster's instance
/// array `(1, class, 0)` (thiscall on the cluster), `PhyreFree` (cdecl),
/// the sampler state's registration (thiscall); the class, the two
/// sampler names, the record tag.
pub const SAMPLER_CALLS: [u32; 5] = [
    0x004e_6a40,
    0x0094_9256,
    0x0044_1010,
    0x0042_fb20,
    0x0058_cf80,
];
/// See [`SAMPLER_CALLS`].
pub const SAMPLER_DATA: [u32; 4] = [0x00c9_98e8, 0x00b7_62d0, 0x00b3_018c, 0x47];
/// The words of the shape's frame: the second sampler's record, the
/// first's, the `{1, 1, 0, 0}` vector (16 bytes each, as the original's
/// frame from its lowest address).
pub const SAMPLER_FRAME_WORDS: u32 = 12;

/// `0x006e5be0`'s callees and data: `ClusterManager::GetInstance()`
/// (cdecl), `ClusterManager::loadPCluster(name)` (thiscall, ret 4),
/// `PApplication::GetApplication()`, `PApplication_FixupClusters(&c, n)`
/// (cdecl), `PhyreSceneCursor_Seek3` (thiscall on the cursor), the
/// cluster global, the class the cursor walks, the global it sets.
pub const LIGHT_CLUSTER: [u32; 8] = [
    0x0069_a1a0,
    0x0069_b940,
    0x0062_3b90,
    0x0062_3580,
    0x0065_ec40,
    0x00ce_63cc,
    0x00ca_5ff0,
    0x00ce_63d4,
];

/// `PDeferredLightingBase::initializeCommonShaders`'s callees beyond
/// [`SAMPLER_CALLS`]: `PCluster::deleteObject(obj)` (thiscall on the
/// cluster, ret 4), the word array's `set(i, v)` (thiscall, ret 8),
/// `PParameterBuffer::setValueDesc(desc, sampler)` (thiscall on the
/// material's buffer, ret 8); the second sampler's name.
pub const COMMON_CALLS: [u32; 4] = [0x0044_1240, 0x0048_c540, 0x0056_e600, 0x00b7_62e4];
/// The parameters `initializeCommonShaders` looks up, in its order:
/// the name and the field that takes it.
pub const COMMON_PARAMS: [(u32, u32); 15] = [
    (0x00b7_61a4, 0x3f54),
    (0x00b7_61b8, 0x3f58),
    (0x00b7_61c4, 0x3f5c),
    (0x00b7_61dc, 0x3f60),
    (0x00b7_61ec, 0x3f64),
    (0x00b7_6204, 0x3f84),
    (0x00b7_6210, 0x3f7c),
    (0x00b7_6224, 0x3f80),
    (0x00b7_6238, 0x3f68),
    (0x00b7_6254, 0x3f6c),
    (0x00b7_6270, 0x3f70),
    (0x00b7_628c, 0x3f74),
    (0x00b7_62a8, 0x3f78),
    (0x00b7_62c0, 0x3f88),
    (0x00b2_3e5c, 0x3f8c),
];

/// `0x005930e0`'s `memset` (cdecl) and the sampler class instances it
/// passes.
pub const TABLES_MEMSET: u32 = 0x0094_964c;
/// See [`TABLES_MEMSET`].
pub const TABLES_CLASSES: u32 = 0x00b3_3a80;
/// The words of `0x005930e0`'s frame: the zeroed table.
pub const TABLES_FRAME_WORDS: u32 = 0x80;

/// `0x0059a390`'s callees: `PMatrix4_RowTimes(out, v)` (thiscall, ret 8),
/// `PMatrix4_Transpose(out, m)` (cdecl).
pub const DEPTH_CALLS: [u32; 2] = [0x004c_9c20, 0x004f_93f0];
/// The words of `0x0059a390`'s frame.
pub const DEPTH_FRAME_WORDS: u32 = 0x45;

/// `0x00a13400`'s data and callees: what the renderer's `+0x48` is set
/// to, the tile setup `0x005b6620` (thiscall on the renderer, ret 0x14),
/// `PParameterBuffer_writeParameter(parameter, src)` (thiscall, ret 8).
pub const TEXTURE_PARAMS: [u32; 3] = [0x0194_4c3c, 0x005b_6620, 0x004d_57d0];
/// The words of `0x00a13400`'s frame.
pub const TEXTURE_FRAME_WORDS: u32 = 21;

/// `0x00a965d0`'s constants in the image: 6.0 (`float`), 30.0 and 0.1
/// (doubles), 16.0 (`float`).
pub const PROFILE_CONSTS: [u32; 4] = [0x00b9_2308, 0x00b9_22c8, 0x00b9_21f0, 0x00b9_231c];

/// The ports over game memory.
pub mod bridge {
    use super::LIGHT_CLUSTER;
    use super::PROFILE_CONSTS;
    use super::{COMMON_CALLS, COMMON_PARAMS};
    use super::{
        Conv, FIND_PARAM, Host, INIT_COMMON, NOT_READY, NVN_PARAMS, SAMPLER_CALLS, SAMPLER_DATA,
        SamplerShaders,
    };
    use super::{DEPTH_CALLS, TABLES_CLASSES, TABLES_MEMSET, TEXTURE_PARAMS};

    /// `PDeferredLightingNVN::initializeShaders(a, effect)` (`0x00a1a4f0`,
    /// thiscall, `ret 8`): 0x15 without the buffers at `+0x3ee0` and
    /// `+0x3ee4`; the common shaders' error; else each of
    /// [`NVN_PARAMS`] looked up in the effect and stored; 0.
    pub fn init_nvn_shaders(h: &mut impl Host, this: u32, a: u32, effect: u32) -> u32 {
        let b0: u32 = h.read(this + 0x3ee0);
        let b1: u32 = h.read(this + 0x3ee4);
        if b0 == 0 || b1 == 0 {
            return NOT_READY;
        }
        let r = h.call_sym(INIT_COMMON, Conv::Thiscall(this), &[a, effect]);
        if r != 0 {
            return r;
        }
        for (name, field) in NVN_PARAMS {
            let n = h.rebase(name);
            let p = h.call_sym(FIND_PARAM, Conv::Thiscall(effect), &[n]);
            h.write(this + field, p);
        }
        0
    }

    /// One sampler parameter's record `{0x47, 0, sampler, 0}` at `rec`
    /// (the original's frame) copied into the constant buffer `data` at
    /// the parameter's offset (`+8`, 16 bits) for its size (`+0xa`, 13
    /// bits; the copy reads that many bytes from the record, whatever
    /// follows it in the frame). `false` (error 3) when it does not fit
    /// or the offset is under 4.
    fn put_sampler(h: &mut impl Host, param: u32, data: u32, rec: u32, sampler: u32) -> bool {
        let off = u32::from(h.read::<u16>(param + 8));
        let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
        h.write(rec, SAMPLER_DATA[3]);
        h.write(rec + 4, 0u32);
        h.write(rec + 8, sampler);
        h.write(rec + 0xc, 0u32);
        copy_param(h, off, size, data, rec)
    }

    /// A parameter's bytes (offset `+8`, size `+0xa` 13 bits) copied from
    /// `src` into the constant buffer `data`; `false` (error 3) when they
    /// do not fit or the offset is under 4.
    fn put_block(h: &mut impl Host, param: u32, data: u32, src: u32) -> bool {
        let off = u32::from(h.read::<u16>(param + 8));
        let size = u32::from(h.read::<u16>(param + 0xa) & 0x1fff);
        copy_param(h, off, size, data, src)
    }

    fn copy_param(h: &mut impl Host, off: u32, size: u32, data: u32, src: u32) -> bool {
        if off.wrapping_add(size) > h.read::<u32>(data) || off < 4 {
            return false;
        }
        h.call_sym(
            SAMPLER_CALLS[1],
            Conv::Cdecl,
            &[data.wrapping_add(off), src, size],
        );
        true
    }

    /// The post-processing `initializeShaders(cluster, effect)` shape with
    /// two samplers (`0x00a18c80`, `0x00a1ae00`; thiscall, `ret 8`): 0x15
    /// without the guard (`+G`); the effect's constant buffer (`+B`, its
    /// block copied from the previous one `[+8] + 4` when their sizes
    /// match); the parameters looked up; a one-element instance array
    /// from the cluster holding the buffer, kept at `[+G] + 0x10`; both
    /// sampler states registered (their error returned); the two sampler
    /// parameters filled from records in `frame` (8 words: the second
    /// record, then the first; the original's frame). 0, or 3 when a
    /// sampler does not fit.
    pub fn init_sampler_shaders(
        h: &mut impl Host,
        c: &SamplerShaders,
        this: u32,
        cluster: u32,
        effect: u32,
        frame: u32,
    ) -> u32 {
        if c.guard_call != 0 {
            let g = h.call_sym(c.guard_call, Conv::Cdecl, &[cluster]);
            h.write(this + c.base, g);
        }
        if h.read::<u32>(this + c.base) == 0 {
            return NOT_READY;
        }
        let buf = h.call_sym(SAMPLER_CALLS[0], Conv::Thiscall(effect), &[cluster, 0, 0]);
        h.write(this + c.buffer, buf);
        let prev: u32 = h.read(this + 8);
        if prev != 0 {
            let dst: u32 = h.read(buf + 4);
            let src: u32 = h.read(prev + 4);
            let a: u32 = h.read(dst + 8);
            let b: u32 = h.read(src + 8);
            if a & 0x7fff_ffff == b & 0x7fff_ffff {
                let n: u32 = h.read(src);
                h.call_sym(SAMPLER_CALLS[1], Conv::Cdecl, &[dst, src, n]);
            }
        }
        for &(name, field) in c.params {
            let n = h.rebase(name);
            let p = h.call_sym(FIND_PARAM, Conv::Thiscall(effect), &[n]);
            h.write(this + field, p);
        }
        let class = h.rebase(SAMPLER_DATA[0]);
        let arr = h.call_sym(SAMPLER_CALLS[2], Conv::Thiscall(cluster), &[1, class, 0]);
        let head: u32 = h.read(arr);
        if head != 1 {
            let count = head & 0x7fff_ffff;
            let slot = arr + 4;
            let data = match count {
                0 => 0,
                1 => slot,
                _ => h.read(slot),
            };
            h.write(slot, 0u32);
            if data != slot && head & 0x8000_0000 == 0 {
                h.call_sym(SAMPLER_CALLS[3], Conv::Cdecl, &[data]);
            }
            h.write(arr, 1u32);
        }
        let count = h.read::<u32>(arr) & 0x7fff_ffff;
        if count != 0 {
            let at = if count > 1 { h.read(arr + 4) } else { arr + 4 };
            let buf: u32 = h.read(this + c.buffer);
            h.write(at, buf);
        }
        let guard: u32 = h.read(this + c.base);
        h.write(guard + 0x10, arr);
        if c.vec_param != 0 {
            let p: u32 = h.read(this + c.vec_param);
            if p != 0 {
                let buf: u32 = h.read(this + c.buffer);
                let data: u32 = h.read(buf + 4);
                let v = frame + 0x20;
                let one = 1.0f32.to_bits();
                for (i, w) in [one, one, 0, 0].into_iter().enumerate() {
                    h.write(v + 4 * i as u32, w);
                }
                if !put_block(h, p, data, v) {
                    return 3;
                }
            }
        }
        let first = this + c.samplers[0];
        let second = this + c.samplers[1];
        for s in [first, second] {
            let r = h.call_sym(SAMPLER_CALLS[4], Conv::Thiscall(s), &[]);
            if r != 0 {
                return r;
            }
        }
        let n1 = h.rebase(SAMPLER_DATA[1]);
        let p1 = h.call_sym(FIND_PARAM, Conv::Thiscall(effect), &[n1]);
        let n2 = h.rebase(SAMPLER_DATA[2]);
        let p2 = h.call_sym(FIND_PARAM, Conv::Thiscall(effect), &[n2]);
        for (p, rec, sampler) in [(p1, frame + 0x10, first), (p2, frame, second)] {
            if p == 0 {
                continue;
            }
            let buf: u32 = h.read(this + c.buffer);
            let data: u32 = h.read(buf + 4);
            if !put_sampler(h, p, data, rec, sampler) {
                return 3;
            }
        }
        0
    }

    /// `0x006e5be0` (cdecl `()`, the light setup's cluster load; reached
    /// by a tail jump from `0x006e5df0`, which unloads the old one first):
    /// `[0x00ce63cc] = loadPCluster(GetInstance(); name)`; with a cluster,
    /// `GetApplication()`, `PApplication_FixupClusters(&[0x00ce63cc], 1)`,
    /// then a scene cursor over the cluster's object list (`+0x1c`, empty
    /// when it points at itself) for the class `0x00ca5ff0`
    /// (`PhyreSceneCursor_Seek3`); when it finds one (cursor `+0xc` set),
    /// `[0x00ce63d4]` = cursor `+0x10` + cursor `+0`.
    ///
    /// The name the original passes is `[ebp-4]`, a frame word it never
    /// writes (stale stack: the caller's last return address, or older);
    /// the port passes 0 (the junk-read policy, brief 2026-10-24).
    ///
    /// `frame` is the cursor, 8 words (the original's `ebp - 0x24`).
    pub fn load_light_cluster(h: &mut impl Host, frame: u32) {
        let manager = h.call_sym(LIGHT_CLUSTER[0], Conv::Cdecl, &[]);
        let name = 0; // junk read: [ebp-0x4] never written; 0 (brief 2026-10-24)
        let cluster = h.call_sym(LIGHT_CLUSTER[1], Conv::Thiscall(manager), &[name]);
        let global = h.rebase(LIGHT_CLUSTER[5]);
        h.write(global, cluster);
        if cluster == 0 {
            return;
        }
        h.call_sym(LIGHT_CLUSTER[2], Conv::Cdecl, &[]);
        h.call_sym(LIGHT_CLUSTER[3], Conv::Cdecl, &[global, 1]);
        let cluster: u32 = h.read(global);
        let list = cluster.wrapping_add(0x1c);
        let first: u32 = h.read(list);
        let first = if first == list { 0 } else { first };
        let class = h.rebase(LIGHT_CLUSTER[6]);
        for (i, w) in [0, 0, 0, 0, 0, list, first, class].into_iter().enumerate() {
            h.write(frame + 4 * i as u32, w);
        }
        h.call_sym(LIGHT_CLUSTER[4], Conv::Thiscall(frame), &[]);
        if h.read::<u32>(frame + 0xc) != 0 {
            let v = h
                .read::<u32>(frame + 0x10)
                .wrapping_add(h.read::<u32>(frame));
            let at = h.rebase(LIGHT_CLUSTER[7]);
            h.write(at, v);
        }
    }

    /// `PDeferredLightingBase::initializeCommonShaders(cluster, effect)`
    /// (`0x00a182f0`, thiscall, `ret 8`): 0x15 unless the three targets
    /// `+0x3edc`, `+0x3ee0`, `+0x3ee4` are there; the old material
    /// (`+0x3ed8`) deleted from the cluster; a new one from the effect
    /// (its buffer's block copied from the previous one `[+8] + 4` when
    /// their sizes match); a one-element instance array from the cluster
    /// holding the material, kept in each target's `+0x10`; the fifteen
    /// parameters ([`COMMON_PARAMS`]); the three sampler states
    /// (`+0x3ee8`, `+0x3f0c`, `+0x3f30`) registered (their error
    /// returned); the point-clamp and shadow-map samplers bound to the
    /// first two when the material has them (their error returned). 0.
    pub fn init_common_shaders(h: &mut impl Host, this: u32, cluster: u32, effect: u32) -> u32 {
        for target in [0x3edc, 0x3ee0, 0x3ee4] {
            if h.read::<u32>(this + target) == 0 {
                return NOT_READY;
            }
        }
        let old: u32 = h.read(this + 0x3ed8);
        if old != 0 {
            h.call_sym(COMMON_CALLS[0], Conv::Thiscall(cluster), &[old]);
        }
        let material = h.call_sym(SAMPLER_CALLS[0], Conv::Thiscall(effect), &[cluster, 0, 0]);
        h.write(this + 0x3ed8, material);
        let prev: u32 = h.read(this + 8);
        if prev != 0 {
            let dst: u32 = h.read(material + 4);
            let src: u32 = h.read(prev + 4);
            let a: u32 = h.read(dst + 8);
            let b: u32 = h.read(src + 8);
            if a & 0x7fff_ffff == b & 0x7fff_ffff {
                let n: u32 = h.read(src);
                h.call_sym(SAMPLER_CALLS[1], Conv::Cdecl, &[dst, src, n]);
            }
        }
        let class = h.rebase(SAMPLER_DATA[0]);
        let arr = h.call_sym(SAMPLER_CALLS[2], Conv::Thiscall(cluster), &[1, class, 0]);
        let head: u32 = h.read(arr);
        if head != 1 {
            let count = head & 0x7fff_ffff;
            let slot = arr.wrapping_add(4);
            let data = match count {
                0 => 0,
                1 => slot,
                _ => h.read(slot),
            };
            if slot != 0 {
                h.write(slot, 0u32);
                if data != slot && head & 0x8000_0000 == 0 {
                    h.call_sym(SAMPLER_CALLS[3], Conv::Cdecl, &[data]);
                }
                h.write(arr, 1u32);
            }
        }
        let material: u32 = h.read(this + 0x3ed8);
        h.call_sym(COMMON_CALLS[1], Conv::Thiscall(arr), &[0, material]);
        for target in [0x3edc, 0x3ee0, 0x3ee4] {
            let t: u32 = h.read(this + target);
            h.write(t + 0x10, arr);
        }
        for (name, field) in COMMON_PARAMS {
            let n = h.rebase(name);
            let p = h.call_sym(FIND_PARAM, Conv::Thiscall(effect), &[n]);
            h.write(this + field, p);
        }
        for sampler in [0x3ee8, 0x3f0c, 0x3f30] {
            let r = h.call_sym(SAMPLER_CALLS[4], Conv::Thiscall(this + sampler), &[]);
            if r != 0 {
                return r;
            }
        }
        let names = [SAMPLER_DATA[1], COMMON_CALLS[3]];
        let mut found = [0u32; 2];
        for (f, name) in found.iter_mut().zip(names) {
            let n = h.rebase(name);
            *f = h.call_sym(FIND_PARAM, Conv::Thiscall(effect), &[n]);
        }
        for (desc, sampler) in found.into_iter().zip([0x3ee8, 0x3f0c]) {
            if desc == 0 {
                continue;
            }
            let material: u32 = h.read(this + 0x3ed8);
            let buffer: u32 = h.read(material + 4);
            let r = h.call_sym(
                COMMON_CALLS[2],
                Conv::Thiscall(buffer),
                &[desc, this + sampler],
            );
            if r != 0 {
                return r;
            }
        }
        0
    }

    /// `0x005b4c00` (thiscall `(m; out, v)`, ret 8 -> `out`): `out` = the
    /// 4x4 `m` (16 floats, rows) applied to the column `v`: for each of
    /// `out[2]`, `out[1]`, `out[0]` (in that order) the products
    /// `f32(v_i m[c + 4i])`, summed as `f32(f32(f32(p1 + p0) + p2) + p3)`;
    /// `out[3]` = `v[3]`. Every value passes through `float` memory, as
    /// the original's.
    pub fn transform_column(h: &mut impl Host, m: u32, out: u32, v: u32) -> u32 {
        use ffx_bridge::x87::Reg;
        let mut col = [0u32; 16];
        for (i, w) in col.iter_mut().enumerate() {
            *w = h.read(m + 4 * i as u32);
        }
        let ld = |bits: u32| Reg::load(f32::from_bits(bits));
        let w = ld(h.read(v + 0xc)).store();
        let mut r = [0f32; 3];
        for c in [2usize, 1, 0] {
            let mut p = [0f32; 4];
            for (i, pi) in p.iter_mut().enumerate() {
                let vi = ld(h.read(v + 4 * i as u32)).store();
                let mi = ld(col[c + 4 * i]).store();
                *pi = (Reg::load(vi) * Reg::load(mi)).store();
            }
            let s = (Reg::load(p[1]) + Reg::load(p[0])).store();
            let s = (Reg::load(p[2]) + Reg::load(s)).store();
            r[c] = (Reg::load(p[3]) + Reg::load(s)).store();
        }
        h.write(out + 4, r[1].to_bits());
        h.write(out, r[0].to_bits());
        h.write(out + 8, r[2].to_bits());
        h.write(out + 0xc, w.to_bits());
        out
    }

    /// `ID3D11DeviceContext` method `slot` (a byte offset into the
    /// vtable) on `ctx` (COM: stdcall, `ctx` first).
    fn com(h: &mut impl Host, ctx: u32, slot: u32, args: &[u32]) {
        let vt: u32 = h.read(ctx);
        let f: u32 = h.read(vt.wrapping_add(slot));
        let mut words = vec![ctx];
        words.extend_from_slice(args);
        h.call(f, Conv::Stdcall, &words);
    }

    /// The tables' constant buffer (`+0`, when there is one) updated from
    /// `+8` (method `+0xc0`, the size `+4` masked as the row and depth
    /// pitch).
    fn upload_tables(h: &mut impl Host, this: u32, ctx: u32) {
        let buffer: u32 = h.read(this);
        if buffer != 0 {
            let size = h.read::<u32>(this + 4) & 0x7fff_ffff;
            let src: u32 = h.read(this + 8);
            com(h, ctx, 0xc0, &[buffer, 0, 0, src, size, size]);
        }
    }

    /// `0x005930e0` (thiscall `(tables; renderer, unused)`, ret 8 -> `al`
    /// 1): the shader resource tables bound on the renderer's context
    /// (`[[renderer] + 0x70]`): the constant buffer `+0` (when there is
    /// one; method `+0xc0` with `+8` and the size `+4` masked twice);
    /// every stage's 128 resource slots cleared from a zeroed 0x200-byte
    /// table (`frame`, the original's `ebp - 0x204`: its first word
    /// stored, the rest `memset`) through methods `+0x20`, `+0x64`,
    /// `+0x7c`, `+0x10c`, `+0xec`, `+0xfc`, then `+0x84(0, 0, 0)`; with a
    /// slot index (`+0x18`, not -1) the buffer uploaded again and stored
    /// at `+0x27c + 4 * index`; the samplers `+0x25c` (8 of them when the
    /// device's feature level `+0xb4` is 0xb000 or more, else 1, with the
    /// class instances `0x00b33a80`; the device's `+0x300` cleared), then
    /// `+0x114([+0x4e0], 0, 0)`, the 8 constant buffers `+0x27c`, the 128
    /// resources `+0x1c`, the 16 `+0x21c`.
    pub fn bind_resource_tables(h: &mut impl Host, this: u32, renderer: u32, frame: u32) -> u8 {
        let device: u32 = h.read(renderer);
        let ctx: u32 = h.read(device + 0x70);
        upload_tables(h, this, ctx);
        h.write(frame, 0u32);
        h.call_sym(TABLES_MEMSET, Conv::Cdecl, &[frame + 4, 0, 0x1fc]);
        for slot in [0x20, 0x64, 0x7c, 0x10c, 0xec, 0xfc] {
            com(h, ctx, slot, &[0, 0x80, frame]);
        }
        com(h, ctx, 0x84, &[0, 0, 0]);
        let index: u32 = h.read(this + 0x18);
        if index != u32::MAX {
            let device: u32 = h.read(renderer);
            let ctx2: u32 = h.read(device + 0x70);
            upload_tables(h, this, ctx2);
            let buffer: u32 = h.read(this);
            h.write(
                this.wrapping_add(index.wrapping_mul(4)).wrapping_add(0x27c),
                buffer,
            );
        }
        let level: u32 = h.read(device + 0xb4);
        let n = if level.cast_signed() >= 0xb000 { 8 } else { 1 };
        let classes = h.rebase(TABLES_CLASSES);
        com(h, ctx, 0x110, &[0, n, this + 0x25c, classes]);
        h.write(device + 0x300, 0u32);
        let p: u32 = h.read(this + 0x4e0);
        com(h, ctx, 0x114, &[p, 0, 0]);
        com(h, ctx, 0x11c, &[0, 8, this + 0x27c]);
        com(h, ctx, 0x10c, &[0, 0x80, this + 0x1c]);
        com(h, ctx, 0x118, &[0, 0x10, this + 0x21c]);
        1
    }

    /// `0x0059a390` (thiscall `(this; renderer, m, color)`, ret 0xc -> 0):
    /// the constant buffer `[this + 0x48]` updated (context method
    /// `+0xc0` on `[renderer + 0x70]`, 0x50 bytes) with the transpose of
    /// `m` with its depth remapped to 0..1 (each row of `m` times the
    /// matrix with rows `{1,0,0,0}, {0,1,0,0}, {0,0,0.5,0}, {0,0,0.5,1}`,
    /// `PMatrix4_RowTimes`; `PMatrix4_Transpose`) followed by `color`.
    ///
    /// `frame` is 69 words, the original's `ebp - 0x114`: the transpose's
    /// output at `+0`, the rows' at `+0x40..+0x7c`, the block at `+0x80`,
    /// the remap matrix at `+0xd4`.
    pub fn update_depth_transform(
        h: &mut impl Host,
        this: u32,
        renderer: u32,
        m: u32,
        color: u32,
        frame: u32,
    ) -> u32 {
        let remap = frame + 0xd4;
        let (one, half) = (1.0f32.to_bits(), 0.5f32.to_bits());
        let init = [
            one, 0, 0, 0, //
            0, one, 0, 0, //
            0, 0, half, 0, //
            0, 0, half, one,
        ];
        for (i, w) in init.into_iter().enumerate() {
            h.write(remap + 4 * i as u32, w);
        }
        let r3 = h.call_sym(
            DEPTH_CALLS[0],
            Conv::Thiscall(remap),
            &[frame + 0x50, m + 0x30],
        );
        let r2 = h.call_sym(
            DEPTH_CALLS[0],
            Conv::Thiscall(remap),
            &[frame + 0x60, m + 0x20],
        );
        let r1 = h.call_sym(
            DEPTH_CALLS[0],
            Conv::Thiscall(remap),
            &[frame + 0x70, m + 0x10],
        );
        let r0 = h.call_sym(DEPTH_CALLS[0], Conv::Thiscall(remap), &[frame + 0x40, m]);
        for (row, src) in [r0, r1, r2, r3].into_iter().enumerate() {
            copy_floats(h, src, remap + 0x10 * row as u32, 4);
        }
        let t = h.call_sym(DEPTH_CALLS[1], Conv::Cdecl, &[frame, remap]);
        let block = frame + 0x80;
        copy_floats(h, t, block, 16);
        copy_floats(h, color, block + 0x40, 4);
        let ctx: u32 = h.read(renderer + 0x70);
        let buffer: u32 = h.read(this + 0x48);
        com(h, ctx, 0xc0, &[buffer, 0, 0, block, 0x50, 0x50]);
        0
    }

    /// `n` floats from `src` to `dst` through the x87 (`fld`/`fstp`).
    fn copy_floats(h: &mut impl Host, src: u32, dst: u32, n: u32) {
        for i in 0..n {
            let x = ffx_bridge::x87::round_trip(f32::from_bits(h.read(src + 4 * i)));
            h.write(dst + 4 * i, x.to_bits());
        }
    }

    /// One texture parameter record `{tag, 0, 0, value, 0}` staged at
    /// `rec` (five words, as the original's frame) and copied into the
    /// material's buffer at the parameter's offset (`memcpy`, its size
    /// `+0xa & 0x1fff`); `false` (error 3) when it does not fit or the
    /// offset is under 4.
    fn put_texture(
        h: &mut impl Host,
        param: u32,
        buffer: u32,
        rec: u32,
        tag: u32,
        value: u32,
    ) -> bool {
        for (i, w) in [tag, 0, 0, value, 0].into_iter().enumerate() {
            h.write(rec + 4 * i as u32, w);
        }
        put_block(h, param, buffer, rec)
    }

    /// `0x00a13400` (thiscall `(this; renderer, unused)`, ret 8): the
    /// deferred lighting's texture parameters set in its material's
    /// buffer (`[[+0x3ed8] + 4]`): `+0x4024` (tag 0x48, the texture
    /// `+0x4034`) and `+0x403c` (`+0x4038`), each only of type `'H'`
    /// (else 5); `+0x4014` (tag 0x49, the shadow texture `+0x3fd0`, which
    /// must have flag 8 at `+8` when there is one) of type `'I'`, 0x14 or
    /// 0x15; each failing copy's 3 returned. Then the renderer's `+0x48`
    /// set to `0x01944c3c` and `0x005b6620([mat], [mat + 4], ceil(w / 8),
    /// ceil(h / 8), 1)` with the size of the target `[+0x3fc4]` (its
    /// object `+0`, none when the byte `+4` is set; `+0x1c`, `+0x20`);
    /// its error returned; then `+0x4010` (tag 0x48, `+0x3fd0`, type
    /// `'H'`) written with `PParameterBuffer_writeParameter`. 0.
    ///
    /// `frame` is 21 words, the original's `ebp - 0x54`: the records at
    /// `+0x3c`, `+0x28`, `+0x14`, `+0` in the order above (a copy longer
    /// than 20 bytes reads past its record into the original's stale
    /// stack; the game's parameters are 16 bytes).
    pub fn set_texture_params(h: &mut impl Host, this: u32, renderer: u32, frame: u32) -> u32 {
        for (param, value, rec) in [(0x4024, 0x4034, 0x3c), (0x403c, 0x4038, 0x28)] {
            let p: u32 = h.read(this + param);
            if p == 0 {
                continue;
            }
            let value: u32 = h.read(this + value);
            let material: u32 = h.read(this + 0x3ed8);
            let buffer: u32 = h.read(material + 4);
            if h.read::<u8>(p + 2) != b'H' {
                return 5;
            }
            if !put_texture(h, p, buffer, frame + rec, 0x48, value) {
                return 3;
            }
        }
        let p: u32 = h.read(this + 0x4014);
        if p != 0 {
            let material: u32 = h.read(this + 0x3ed8);
            let buffer: u32 = h.read(material + 4);
            let shadow: u32 = h.read(this + 0x3fd0);
            let ty: u8 = h.read(p + 2);
            if ty != 0x49 && ty != 0x14 && ty != 0x15 {
                return 5;
            }
            if shadow != 0 && h.read::<u8>(shadow + 8) & 8 == 0 {
                return 5;
            }
            if !put_texture(h, p, buffer, frame + 0x14, 0x49, shadow) {
                return 3;
            }
        }
        let target: u32 = h.read(this + 0x3fc4);
        let object: u32 = if h.read::<u8>(target + 4) == 0 {
            h.read(target)
        } else {
            0
        };
        let w: u32 = h.read(object.wrapping_add(0x1c));
        let ht: u32 = h.read(object.wrapping_add(0x20));
        let at = h.rebase(TEXTURE_PARAMS[0]);
        h.write(renderer + 0x48, at);
        let material: u32 = h.read(this + 0x3ed8);
        let (first, second): (u32, u32) = (h.read(material), h.read(material + 4));
        let tiles = |n: u32| (n >> 3) + u32::from(n & 7 != 0);
        let r = h.call_sym(
            TEXTURE_PARAMS[1],
            Conv::Thiscall(renderer),
            &[first, second, tiles(w), tiles(ht), 1],
        );
        if r != 0 {
            return r;
        }
        let p: u32 = h.read(this + 0x4010);
        if p != 0 {
            let shadow: u32 = h.read(this + 0x3fd0);
            for (i, word) in [0x48, 0, 0, shadow, 0].into_iter().enumerate() {
                h.write(frame + 4 * i as u32, word);
            }
            let material: u32 = h.read(this + 0x3ed8);
            let buffer: u32 = h.read(material + 4);
            if h.read::<u8>(p + 2) != b'H' {
                return 5;
            }
            let r = h.call_sym(TEXTURE_PARAMS[2], Conv::Thiscall(buffer), &[p, frame]);
            if r != 0 {
                return r;
            }
        }
        0
    }

    /// `cvttss2si`: truncation toward zero, `0x80000000` for NaN and
    /// values out of the `i32` range.
    fn cvtt(x: f32) -> i32 {
        ffx_bridge::x87::cvttsd2si(f64::from(x)).cast_signed()
    }

    /// A float of game memory.
    fn rf(h: &impl ffx_bridge::Mem, at: u32) -> f32 {
        f32::from_bits(h.read(at))
    }

    /// `0x00a965d0` (fastcall `(this, tables; values, floors, out,
    /// bias)`, the caller pops; SSE): for each run of equal keys
    /// (`[+0x14]`, `[+0]` of them) the largest value of the run (`values`,
    /// `comiss`); when it plus 6 beats the floor at the run's last index,
    /// a profile is chosen (row: the key `>> [+0x20]` clamped to 0..16;
    /// level: `cvttsd2si((f64(v + off) - 30) * 0.1)` clamped to 0..7,
    /// `off` = `[[+4] + 0x1f0] - bias`; `[[tables + 4 row] + 4 level]`:
    /// `{first, end, heights...}` as floats) and its heights plus the
    /// value raised into `out` from the position
    /// `(key - [+0x1c]) + (first - 16) * stride - stride / 2` (`stride`
    /// `[+0x24]`) on, one per
    /// height, every `stride`, while under `[+0x28]` (positions under 1
    /// skipped). Each operation one SSE instruction: `f32` arithmetic,
    /// `cvtdq2ps` for the integers, truncating conversions. The 6, 30,
    /// 0.1 and 16 are read from the image.
    pub fn raise_profiles(
        h: &mut impl Host,
        this: u32,
        tables: u32,
        values: u32,
        floors: u32,
        out: u32,
        bias: u32,
    ) {
        let n: i32 = h.read::<u32>(this).cast_signed();
        let owner: u32 = h.read(this + 4);
        let off = rf(h, owner + 0x1f0) - f32::from_bits(bias);
        let six = rf(h, h.rebase(PROFILE_CONSTS[0]));
        let thirty = f64::from_bits(h.read(h.rebase(PROFILE_CONSTS[1])));
        let tenth = f64::from_bits(h.read(h.rebase(PROFILE_CONSTS[2])));
        let sixteen = rf(h, h.rebase(PROFILE_CONSTS[3]));
        let mut i: i32 = 0;
        while i < n {
            let keys: u32 = h.read(this + 0x14);
            let at = |i: i32| i.cast_unsigned().wrapping_mul(4);
            let mut v = rf(h, values.wrapping_add(at(i)));
            let key: i32 = h.read::<u32>(keys.wrapping_add(at(i))).cast_signed();
            while i + 1 < n && h.read::<u32>(keys.wrapping_add(at(i + 1))).cast_signed() == key {
                i += 1;
                let x = rf(h, values.wrapping_add(at(i)));
                if x > v {
                    v = x;
                }
            }
            let floor = rf(h, floors.wrapping_add(at(i)));
            if v + six > floor {
                let shift = h.read::<u32>(this + 0x20) & 31;
                let row = key >> shift;
                let row = if row >= 0x11 {
                    0x10
                } else if row < 0 {
                    0
                } else {
                    row
                };
                let limit: i32 = h.read::<u32>(this + 0x28).cast_signed();
                let stride: i32 = h.read::<u32>(this + 0x24).cast_signed();
                let e =
                    ffx_bridge::x87::cvttsd2si((f64::from(v + off) - thirty) * tenth).cast_signed();
                let level = if e > 0 { e } else { 0 };
                let level = if level < 7 { level } else { 7 };
                let rows: u32 = h.read(tables.wrapping_add(at(row)));
                let profile: u32 = h.read(rows.wrapping_add(at(level)));
                let first = rf(h, profile);
                let end = cvtt(rf(h, profile + 4));
                #[allow(clippy::cast_precision_loss)] // cvtdq2ps
                let pos = {
                    let base: i32 = h.read::<u32>(this + 0x1c).cast_signed();
                    let here = h
                        .read::<u32>(keys.wrapping_add(at(i)))
                        .cast_signed()
                        .wrapping_sub(base);
                    let x = here as f32 + (first - sixteen) * stride as f32;
                    x - (stride >> 1) as f32
                };
                let mut pos = cvtt(pos);
                let mut j = cvtt(first);
                while j < end {
                    if pos > 0 {
                        let x = rf(h, profile.wrapping_add(at(j)).wrapping_add(8)) + v;
                        let dst = out.wrapping_add(at(pos));
                        if x > rf(h, dst) {
                            h.write(dst, x.to_bits());
                        }
                    }
                    pos = pos.wrapping_add(stride);
                    if pos >= limit {
                        break;
                    }
                    j += 1;
                }
            }
            i += 1;
        }
    }
}
