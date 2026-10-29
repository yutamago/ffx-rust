//! The leftovers of list `phyre-render-b` (M5 stage 1, round "left2"):
//! `PDeferredLightingNVN::initializeShaders` (`0x00a1a4f0`), a table of
//! shader parameters looked up by name after the base class's common
//! shaders.

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

/// The ports over game memory.
pub mod bridge {
    use super::{Conv, FIND_PARAM, Host, INIT_COMMON, NOT_READY, NVN_PARAMS};

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
}
