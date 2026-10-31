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
    /// The sampler pair's base field (`+G`: the guard; `+G+4` and
    /// `+G+0x28` the two sampler states; `[+G] + 0x10` the array).
    pub base: u32,
    /// Where the effect's constant buffer goes.
    pub buffer: u32,
    /// The parameters looked up by name and their fields.
    pub params: &'static [(u32, u32)],
}

/// The copies: `PMotionBlurBase::initializeCommonShaders` and
/// `PScreenSpaceReflectionBase::initializeShaders`.
pub const SAMPLER_SHADERS: [SamplerShaders; 2] = [
    SamplerShaders {
        at: 0x00a1_8c80,
        base: 0xcdc,
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
/// The words of the shape's frame: the two 16-byte sampler records.
pub const SAMPLER_FRAME_WORDS: u32 = 8;

/// The ports over game memory.
pub mod bridge {
    use super::{
        Conv, FIND_PARAM, Host, INIT_COMMON, NOT_READY, NVN_PARAMS, SAMPLER_CALLS, SAMPLER_DATA,
        SamplerShaders,
    };

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
        if off.wrapping_add(size) > h.read::<u32>(data) || off < 4 {
            return false;
        }
        h.call_sym(
            SAMPLER_CALLS[1],
            Conv::Cdecl,
            &[data.wrapping_add(off), rec, size],
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
        let first = this + c.base + 4;
        let second = this + c.base + 0x28;
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
}
