//! The render-side glue at 0x00690000 and up, first batch (M4, list
//! `h3-glue-render`, agent h3b): `ShadowManager_selectShadowSolution`,
//! the shadow solution (0-4) of a field map by its level hash.

/// `ShadowManager_clearTargets_006ce130(this; renderer)` (`ret 4`).
pub const CLEAR_TARGETS: u32 = 0x006c_e130;
/// The shadow filter predicates stored at `+0x4048` (cdecl, one word):
/// `Compute_006cc8b0` (`== 1`) by default, and the four map-specific ones
/// of solution 3.
pub const FILTER_DEFAULT: u32 = 0x006c_c8b0;
pub const FILTER_049D: u32 = 0x006c_c8e0;
pub const FILTER_0B96: u32 = 0x006c_c8c0;
pub const FILTER_12BE: u32 = 0x006c_c900;
pub const FILTER_1432: u32 = 0x006c_c920;
pub const FILTER_178D: u32 = 0x006c_c950;
/// Solution 4's four floats (`+0x4034`, `+0x403c`, `+0x4038`, `+0x4040`
/// read back from them) and two more globals it sets.
pub const CASCADE_PARAMS: u32 = 0x00c3_43ec;
pub const CASCADE_EXTRA_A: u32 = 0x00cd_ec2c;
pub const CASCADE_EXTRA_B: u32 = 0x00cd_ec34;
/// The default `+0x403c` (f32).
pub const DEFAULT_403C: u32 = 0x00c3_4400;
/// f32 constants: `+0x4034`'s (-10.0) and `+0x4024`'s.
pub const C_B304FC: u32 = 0x00b3_04fc;
pub const C_B92340: u32 = 0x00b9_2340;
/// Solution 4's sources (the generic and `bsyt00`'s), in the order the
/// original stores them to `CASCADE_PARAMS + 4`, `+ 8`, `+ 0xc`,
/// `CASCADE_EXTRA_A`, `CASCADE_EXTRA_B`, after `+0` from
/// `C_B304FC` / `C_B92318`.
pub const FOUR_GENERIC: [u32; 5] = [
    0x00b4_9984,
    0x00b4_9980,
    0x00b4_9990,
    0x00b4_7c70,
    0x00b4_7c5c,
];
pub const C_B92318: u32 = 0x00b9_2318;
pub const FOUR_BSYT: [u32; 5] = [
    0x00b4_9988,
    0x00b4_5bb8,
    0x00b4_9994,
    0x00b4_9978,
    0x00b4_99a4,
];
/// `bsyt00`'s `+0x4028` (the generic one stores 0).
pub const C_B499A8: u32 = 0x00b4_99a8;

/// Level hashes (`graphicLevelSetName`'s `hashU` of the map folder) of
/// solution 2.
pub const TWO: [i32; 143] = [
    0x0150, 0x0189, 0x0199, 0x01ca, 0x01f7, 0x0211, 0x021e, 0x0228, 0x0234, 0x0265, 0x0278, 0x02de,
    0x02e6, 0x02fa, 0x02fc, 0x03e8, 0x0438, 0x0456, 0x04a6, 0x04c4, 0x0513, 0x0514, 0x05c5, 0x0633,
    0x063b, 0x0649, 0x066f, 0x0672, 0x0679, 0x06a1, 0x06e1, 0x06e7, 0x076f, 0x07b4, 0x0819, 0x0822,
    0x08a7, 0x0974, 0x0aa4, 0x0ac5, 0x0ad2, 0x0b28, 0x0b40, 0x0bae, 0x0bea, 0x0dc5, 0x0dfe, 0x0e6c,
    0x0e7c, 0x0ea7, 0x0eda, 0x0eea, 0x0f01, 0x0f0b, 0x0f17, 0x0f5b, 0x0f62, 0x0f85, 0x0f99, 0x0fc1,
    0x0fdf, 0x10cb, 0x1139, 0x1189, 0x11cf, 0x11d3, 0x11f6, 0x11f7, 0x1250, 0x125c, 0x12ea, 0x131b,
    0x131e, 0x1383, 0x13c0, 0x13ca, 0x142e, 0x1505, 0x156a, 0x158a, 0x1657, 0x174f, 0x17bd, 0x180b,
    0x1818, 0x1823, 0x1883, 0x1891, 0x18cd, 0x193b, 0x1955, 0x19a9, 0x1a3a, 0x1a6b, 0x1acd, 0x1ad9,
    0x1b4f, 0x1b8a, 0x1bbd, 0x1bcd, 0x1be4, 0x1c0e, 0x1c36, 0x1c45, 0x1c52, 0x1c68, 0x1c99, 0x1cc0,
    0x1cc2, 0x1d2e, 0x1d58, 0x1dae, 0x1e1c, 0x1e48, 0x1e6c, 0x1e8a, 0x1eb6, 0x1ed9, 0x1eda, 0x1f20,
    0x1f48, 0x2001, 0x203b, 0x20a3, 0x20a6, 0x20ad, 0x217f, 0x21e8, 0x222f, 0x22db, 0x2416, 0x24a0,
    0x24c9, 0x24d8, 0x24de, 0x24f8, 0x2506, 0x2566, 0x2574, 0x25b0, 0x25ca, 0x261e, 0x268c,
];

/// Level hashes of solution 4 (`0x12be` and `0x1733` have their own
/// rules).
pub const FOUR: [i32; 27] = [
    0x012b, 0x0482, 0x056d, 0x0a6c, 0x0b48, 0x0c04, 0x0fb6, 0x1165, 0x132c, 0x1666, 0x17e6, 0x1899,
    0x18e7, 0x1b5f, 0x1b84, 0x1cd6, 0x1f33, 0x1f90, 0x1fa1, 0x200f, 0x2349, 0x248b, 0x24f9, 0x255c,
    0x257c, 0x25e2, 0x2638,
];

/// Level hashes of solution 1.
pub const ONE: [i32; 4] = [0x0099, 0x0fc9, 0x1cac, 0x21c1];

/// What a level hash selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Solution {
    Zero,
    One,
    Two,
    /// Solution 3 with the filter predicate it stores (`None`: the
    /// default one, already stored).
    Three(Option<u32>),
    /// Solution 4, the generic parameters.
    Four,
    /// Solution 4 with `bsyt00`'s parameters.
    FourBsyt,
}

/// The decision over the level hash, the map id and the scenario (outside
/// battle).
#[must_use]
pub fn select(hash: i32, map: u32, scenario: i32) -> Solution {
    if hash <= 0 {
        return Solution::Zero;
    }
    match hash {
        // mihn04: 4 before the scenario reaches 0xb54
        0x12be if scenario < 0xb54 => Solution::Four,
        0x12be => Solution::Three(Some(FILTER_12BE)),
        // bsyt00: only on map 0x18a
        0x1733 if map == 0x18a => Solution::FourBsyt,
        0x1733 => Solution::Zero,
        0x049d => Solution::Three(Some(FILTER_049D)),
        0x0b96 => Solution::Three(Some(FILTER_0B96)),
        0x1432 => Solution::Three(Some(FILTER_1432)),
        0x178d => Solution::Three(Some(FILTER_178D)),
        // the default filter, stored again
        0x1d21 => Solution::Three(Some(FILTER_DEFAULT)),
        h if FOUR.contains(&h) => Solution::Four,
        h if TWO.contains(&h) => Solution::Two,
        h if ONE.contains(&h) => Solution::One,
        _ => Solution::Zero,
    }
}

pub mod bridge {
    //! The original over game memory.

    #[allow(clippy::wildcard_imports)] // the module's constants
    use super::*;
    use ffx_bridge::x87;
    use ffx_graphic::host::{Conv, Host};

    /// An f32 moved through the x87 (`fld dword` / `fstp dword`).
    fn copy(h: &mut impl Host, to: u32, from: u32) {
        let v: u32 = h.read(from);
        h.write(to, x87::store(x87::load(v)));
    }

    /// `ShadowManager_selectShadowSolution` (`0x006ccd10`): thiscall
    /// `(this; arg0, battle, map, hash, scenario, renderer)`, `ret 0x18`.
    /// `arg0` is never read. Clears the shadow targets, resets the
    /// solution state (`+0x4030`..`+0x405c`, `+0x403c` from `0x00c34400`)
    /// and returns the solution: 2 in battle, else [`select`]'s, with the
    /// state each one sets.
    #[allow(clippy::too_many_arguments)] // the original's
    pub fn select_shadow_solution(
        h: &mut impl Host,
        this: u32,
        battle: u8,
        map: u32,
        hash: u32,
        scenario: u32,
        renderer: u32,
    ) -> u32 {
        h.write(this + 0x4050, hash);
        h.call(h.rebase(CLEAR_TARGETS), Conv::Thiscall(this), &[renderer]);
        let default_403c = h.rebase(DEFAULT_403C);
        copy(h, this + 0x403c, default_403c);
        h.write(this + 0x4030, 0u8);
        h.write(this + 0x4044, 1u8);
        h.write(this + 0x4048, h.rebase(FILTER_DEFAULT));
        h.write(this + 0x404c, 1u8);
        h.write(this + 0x4054, 0u8);
        h.write(this + 0x4058, 0u32);
        h.write(this + 0x405c, 0u32);
        if battle != 0 {
            return 2;
        }
        let minus_ten = h.rebase(C_B304FC);
        match select(hash as i32, map, scenario as i32) {
            Solution::Zero => 0,
            Solution::One => 1,
            Solution::Two => {
                copy(h, this + 0x4034, minus_ten);
                copy(h, this + 0x403c, default_403c);
                h.write(this + 0x400c, 1u8);
                2
            }
            Solution::Three(filter) => {
                if let Some(f) = filter {
                    h.write(this + 0x4048, h.rebase(f));
                }
                copy(h, this + 0x4034, minus_ten);
                copy(h, this + 0x403c, default_403c);
                h.write(this + 0x400c, 1u8);
                h.write(this + 0x4030, 1u8);
                3
            }
            Solution::Four => {
                four(h, this, C_B304FC, FOUR_GENERIC, None);
                4
            }
            Solution::FourBsyt => {
                four(h, this, C_B92318, FOUR_BSYT, Some(C_B499A8));
                h.write(this + 0x4054, 1u8);
                h.write(this + 0x4044, 0u8);
                4
            }
        }
    }

    /// Solution 4's stores: the cascade parameters from `first` and
    /// `rest`, read back into `+0x4034`..`+0x4040`; `+0x4020` 0, `+0x4024`
    /// from `0x00b92340`, `+0x4028` from `c4028` (or 0), `+0x402c` 0;
    /// `+0x400c` set.
    fn four(h: &mut impl Host, this: u32, first: u32, rest: [u32; 5], c4028: Option<u32>) {
        let params = h.rebase(CASCADE_PARAMS);
        let first_bits: u32 = h.read(h.rebase(first));
        // fld; fst [params] (kept in ST0 for +0x4034)
        let st0 = x87::load(first_bits);
        h.write(params, x87::store(st0));
        let to = [
            params + 4,
            params + 8,
            params + 0xc,
            h.rebase(CASCADE_EXTRA_A),
            h.rebase(CASCADE_EXTRA_B),
        ];
        for (dst, src) in to.into_iter().zip(rest) {
            let src = h.rebase(src);
            copy(h, dst, src);
        }
        h.write(this + 0x4034, x87::store(st0));
        copy(h, this + 0x4038, params + 8);
        copy(h, this + 0x403c, params + 4);
        copy(h, this + 0x4040, params + 0xc);
        h.write(this + 0x4020, 0u32);
        let c = h.rebase(C_B92340);
        copy(h, this + 0x4024, c);
        match c4028 {
            Some(c) => {
                let c = h.rebase(c);
                copy(h, this + 0x4028, c);
            }
            None => h.write(this + 0x4028, 0u32),
        }
        h.write(this + 0x402c, 0u32);
        h.write(this + 0x400c, 1u8);
    }
}
