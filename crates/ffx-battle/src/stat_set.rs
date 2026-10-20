//! `MsSetChrStatInfo` (`docs/rewrite-plan.md`, M2): the battle scripts'
//! and AI's change to a group of actors (`MsGetChrBit`'s selector): one of
//! 0x15a properties set to a value, the counterpart of `MsGetChrStatInfo`
//! (`crate::stat`). Most properties are a store into the actor (or its info
//! record at `+0xf88`); some call the battle's other code (through the
//! game) and some ask for the status, weakness or stat refresh afterwards.

use Set::{B, Bit8, Bit16, Clamp, D, IB, IW, Other, W};
use Src::{Bit, V};

/// What a byte or bit store takes from the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Src {
    /// The value's low bits.
    V,
    /// Bit 0 of the value as it came (the original's local; property
    /// 0x14c can set it for the actors after).
    Bit,
}

/// What a property stores per selected actor (`MsSetChrStatInfo`'s jump
/// table at `0x007b66dc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Set {
    /// A byte of the actor.
    B(u32, Src),
    /// A `u16` of the actor: the value's low 16 bits.
    W(u32),
    /// A `u32` of the actor: the value.
    D(u32),
    /// A byte of the actor: the value as it came, kept within a range
    /// (`MsCheckRange`).
    Clamp(u32, i32, i32),
    /// Bit `n` of the actor's byte = bit 0 of the source.
    Bit8(u32, u32, Src),
    /// Bit `n` of the actor's `u16` = bit 0 of the source.
    Bit16(u32, u32, Src),
    /// A byte of the actor's info record (`+0xf88`), when it has one.
    IB(u32),
    /// A `u16` of the info record, when it has one.
    IW(u32),
    /// Its own code, or nothing.
    Other,
}

/// The property `kind`'s store, as the jump table's simple cases do it.
#[allow(clippy::too_many_lines, clippy::match_same_arms)]
const fn set(kind: u32) -> Set {
    match kind {
        1 => D(0x5d4),
        2 => D(0x594),
        3 => D(0x598),
        5 => Bit16(0x606, 3, V),
        6 => Bit16(0x606, 2, V),
        7 => Bit16(0x606, 1, V),
        0xa => Clamp(0x5a9, 1, 0xff),
        0xb => Clamp(0x5aa, 1, 0xff),
        0xc => Clamp(0x5ab, 1, 0xff),
        0xd => Clamp(0x5ac, 1, 0xff),
        0xe => Clamp(0x5ad, 1, 0xff),
        0xf => Clamp(0x5ae, 0, 0xff),
        0x10 => Clamp(0x5af, 1, 0xff),
        0x11 => Clamp(0x5ba, 0, 0xff),
        0x12 => Clamp(0x5bb, 1, 100),
        0x13 => Clamp(0x5bc, 1, 100),
        0x14 => Clamp(0x5bd, 1, 100),
        0x16 => Bit8(0x590, 0, V),
        0x17 => Bit8(0x590, 1, V),
        0x18 => Bit8(0x590, 2, V),
        0x20..=0x24 => Bit16(0x5b8, kind - 0x20, Bit),
        0x25..=0x2c => Bit16(0x606, kind - 0x25 + 4, Bit),
        0x3a..=0x3e => Bit16(0x6bc, kind - 0x3a, Bit),
        0x3f => Bit16(0x6bc, 7, Bit),
        0x42 => Bit16(0x6bc, 6, Bit),
        0x43 => Bit16(0x6bc, 5, Bit),
        0x44..=0x47 => Bit16(0x6bc, kind - 0x44 + 9, Bit),
        0x4a => Bit16(0x6bc, 13, Bit),
        0x4c => Bit16(0x6be, 15, Bit),
        0x4d => Bit16(0x6c0, 0, Bit),
        0x4e => Bit16(0x6c0, 1, Bit),
        0x62..=0x66 => Bit8(0x5da, kind - 0x62, Bit),
        0x67..=0x6b => Bit8(0x5db, kind - 0x67, Bit),
        0x6c..=0x70 => Bit8(0x5dc, kind - 0x6c, Bit),
        0x71..=0x75 => Bit8(0x5dd, kind - 0x71, Bit),
        0x77 => Bit8(0x447, 0, Bit),
        0x78 => Bit8(0x447, 1, Bit),
        0x81 => IW(0x24),
        0x82 => IB(0x28),
        0x83 => IW(0x26),
        0x84 => IB(0x29),
        0x8a => IB(0x8),
        0x8b => IB(0x9),
        0x8c => IB(0xb),
        0x8d => IB(0xa),
        0x8f..=0x96 | 0x98..=0x9d => Bit16(0x616, kind - 0x8f, Bit),
        0x9e => D(0x4ac),
        0xa6 => D(0xd34),
        0xa7 => D(0xd38),
        0xa8 => D(0xd3c),
        0xc8..=0xcf => Bit16(0x65a, kind - 0xc8, Bit),
        0xd0 => Bit16(0x65a, 9, Bit),
        0xd1 => Bit16(0x65a, 8, Bit),
        0xd2..=0xd6 => Bit16(0x65a, kind - 0xd2 + 10, Bit),
        0xda => W(0x4b0),
        0xdb => W(0x4b2),
        0xf0 => Bit16(0x5b8, 8, V),
        0x104 => W(0xe1a),
        0x107 => Bit16(0x5b8, 7, Bit),
        0x108 => Bit16(0x5b8, 6, Bit),
        0x109 => Bit16(0x5b8, 5, Bit),
        0x10a => IW(0x6),
        0x10b => Bit16(0x5b8, 9, Bit),
        0x10c => D(0x5a4),
        0x113 => IB(0x112),
        0x122 => Bit16(0x5b8, 10, Bit),
        0x12b => IW(0x2),
        0x12c => IW(0x4),
        0x12d => IW(0x0),
        0x12e..=0x135 => B(0x5b0 + kind - 0x12e, V),
        0x13a..=0x13d => IW(0xc + 2 * (kind - 0x13a)),
        0x13e..=0x141 => IW(0x18 + 2 * (kind - 0x13e)),
        0x142..=0x145 => IB(0x14 + kind - 0x142),
        0x146..=0x149 => IB(0x20 + kind - 0x146),
        0x159 => W(0xdde),
        0x8 => B(0x63d, V),
        0x15 => B(0xdc8, Bit),
        0x1a => B(0x505, Bit),
        0x1b => B(0xdec, Bit),
        0x1e => B(0x4ff, V),
        0x2d => B(0x608, V),
        0x2e => B(0x609, V),
        0x2f => B(0x60a, V),
        0x30 => B(0x60b, V),
        0x31 => B(0x60c, V),
        0x32 => B(0x60d, V),
        0x33 => B(0x60e, V),
        0x34 => B(0x60f, V),
        0x35 => B(0x610, V),
        0x36 => B(0x611, V),
        0x38 => B(0x613, V),
        0x39 => B(0x614, V),
        0x50 => B(0xddd, Bit),
        0x51 => B(0xdd6, Bit),
        0x52 => B(0xdd9, Bit),
        0x53 => B(0xdd7, Bit),
        0x58 => B(0x24, Bit),
        0x5b => B(0x420, V),
        0x5d => B(0x412, Bit),
        0x5e => B(0x43e, Bit),
        0x5f => B(0x508, Bit),
        0x60 => B(0x50d, Bit),
        0x76 => B(0x426, Bit),
        0x79 => B(0xdeb, V),
        0x7a => B(0x448, Bit),
        0x7b => B(0x40a, Bit),
        0x7c => B(0x40c, V),
        0x7d => B(0x41e, Bit),
        0x7e => B(0x41f, Bit),
        0x80 => B(0xe0c, V),
        0x85 => B(0x451, V),
        0x86 => B(0x43d, V),
        0x87 => B(0xdf9, V),
        0x88 => B(0x719, Bit),
        0x89 => B(0x718, Bit),
        0x8e => B(0xdd2, Bit),
        0x9f => B(0x5c9, V),
        0xa0 => B(0x5c8, V),
        0xa1 => B(0x44e, V),
        0xa2 => B(0x41c, V),
        0xa3 => B(0x41d, V),
        0xa4 => B(0x41b, V),
        0xa5 => B(0x433, V),
        0xa9 => B(0x423, V),
        0xaa => B(0xe0e, V),
        0xab => B(0x425, V),
        0xac => B(0xdd3, Bit),
        0xad => B(0xdcd, Bit),
        0xb0 => B(0x642, V),
        0xb1 => B(0x643, V),
        0xb2 => B(0x644, V),
        0xb3 => B(0x645, V),
        0xb4 => B(0x646, V),
        0xb5 => B(0x647, V),
        0xb6 => B(0x648, V),
        0xb7 => B(0x649, V),
        0xb8 => B(0x64a, V),
        0xb9 => B(0x64b, V),
        0xba => B(0x64c, V),
        0xbb => B(0x64d, V),
        0xbc => B(0x64e, V),
        0xbd => B(0x64f, V),
        0xbe => B(0x650, V),
        0xbf => B(0x651, V),
        0xc0 => B(0x652, V),
        0xc1 => B(0x653, V),
        0xc2 => B(0x654, V),
        0xc3 => B(0x655, V),
        0xc4 => B(0x656, V),
        0xc5 => B(0x657, V),
        0xc6 => B(0x658, V),
        0xc7 => B(0x659, V),
        0xd7 => B(0xdd8, Bit),
        0xd9 => B(0x1b, Bit),
        0xdc => B(0x43b, V),
        0xde => B(0x23, Bit),
        0xe0 => B(0x446, V),
        0xe1 => B(0xdda, Bit),
        0xe2 => B(0xe10, V),
        0xe3 => B(0xe11, V),
        0xe4 => B(0x4b8, V),
        0xe5 => B(0x408, V),
        0xe6 => B(0x4b9, V),
        0xe7 => B(0x4ba, V),
        0xe8 => B(0x65c, V),
        0xe9 => B(0x424, V),
        0xea => B(0x4bb, V),
        0xeb => B(0xf74, V),
        0xec => B(0xf75, V),
        0xee => B(0x21, Bit),
        0xef => B(0x22, Bit),
        0xf5 => B(0x43c, V),
        0xf6 => B(0x6db, V),
        0xf7 => B(0x44a, V),
        0xfa => B(0x44b, V),
        0xfc => B(0x405, Bit),
        0xfd => B(0x4b4, Bit),
        0x100 => B(0x5cb, Bit),
        0x101 => B(0x5c4, V),
        0x102 => B(0x6ce, Bit),
        0x103 => B(0x5c3, V),
        0x106 => B(0x6d1, V),
        0x10d => B(0x409, V),
        0x114 => B(0x6df, V),
        0x115 => B(0x6e0, V),
        0x116 => B(0x40d, V),
        0x121 => B(0x44f, V),
        0x123 => B(0x40e, V),
        0x126 => B(0x700, V),
        0x127 => B(0xdca, Bit),
        0x129 => B(0x3f8, Bit),
        0x136 => B(0x6cc, V),
        0x137 => B(0x6cd, V),
        0x138 => B(0x6cf, Bit),
        0x139 => B(0x6d0, Bit),
        0x14a => B(0x3f9, Bit),
        0x14f => B(0x437, Bit),
        0x150 => B(0x1c, Bit),
        0x151 => B(0xddb, Bit),
        0x152 => B(0x442, V),
        0x153 => B(0x39, V),
        0x154 => B(0x3fb, V),
        0x157 => B(0x506, Bit),
        0x158 => B(0xdd5, Bit),
        _ => Other,
    }
}

/// The properties after which the actor's status is checked
/// (`MsStatusEffectCheck`).
const fn status(kind: u32) -> bool {
    matches!(kind, 4..=7 | 0x29..=0x39)
}

/// The properties after which the actor's stats are rebuilt
/// (`MsResetAutoStatus`, `MsSetRamChrParam`, `MsSetAutoStatus`).
const fn rebuild(kind: u32) -> bool {
    matches!(kind, 0x12e..=0x135)
}

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{Set, Src, rebuild, set, status};
    use crate::exec::bridge::Call;

    /// `"sins07_07"` and `"znkd09_00"`: the maps `MsSetChrStatInfo` treats
    /// specially (looked for in the battle file name,
    /// [`crate::read::bridge::FILE_NAME`], with the CRT's `strstr`).
    pub const SINS07_07: u32 = 0x00b5_508c;
    pub const ZNKD09_00: u32 = 0x00b5_5098;
    /// The CRT `strstr` import slot.
    pub const CRT_STRSTR: u32 = 0x00b0_c378;
    /// Set by property 0x57 = 4 on monster 0x14 in `"znkd09_00"`: its
    /// property 0 (HP) then becomes 100.
    pub const ZNKD_HP: Global<u32> = Global::new(0x0113_6ac0);
    /// `g_BtlGilAmount` (property 0x112).
    pub const GIL_AMOUNT: Global<u32> = Global::new(0x0112_be90);

    /// The battle's other functions the properties call, through the game.
    pub const MS_ALIVE_PROCESS: u32 = 0x0078_9160;
    pub const MS_SET_CHR_IMAGE_DAMAGE_ATTACK: u32 = 0x0078_d790;
    pub const MS_SET_CHR_VISIBLE_MAIN: u32 = 0x0079_6860;
    pub const CH_EFFECT_SET_EFFECT_LEVEL: u32 = 0x0082_a9d0;
    pub const BATTLE_FIRST_SELECTED_ACTOR: u32 = 0x0079_47f0;
    pub const MS_SET_CHR_BODY_HIT: u32 = 0x0079_6f20;
    pub const MS_BLOW_PROCESS: u32 = 0x0078_91b0;
    pub const MS_SET_CHR_VISIBLE_APPEAR: u32 = 0x0079_66a0;
    pub const MS_SET_CHR_VISIBLE_CAMERA: u32 = 0x0079_67a0;
    pub const MS_SET_CHR_VISIBLE: u32 = 0x0079_6660;
    pub const MS_SET_CHR_SHADOW_MODE: u32 = 0x0079_62d0;
    pub const MS_MOVE_RESET: u32 = 0x007a_a280;
    pub const MS_MOTION_SPEED_SET_NORMAL: u32 = 0x007a_a260;
    pub const CH_SET_THICKNESS_BTL_CAM: u32 = 0x0082_b8c0;
    pub const CH_SET_CAM_HEIGHT: u32 = 0x0082_b0f0;
    pub const CH_SET_FIX_HEIGHT: u32 = 0x0082_b200;
    pub const MS_ADD_YOUJINBOU_COM: u32 = 0x007b_2950;
    pub const MS_MOTION_SPEED_RESET: u32 = 0x007a_b730;
    pub const MS_SET_SAVE_PLY_JOIN: u32 = 0x0078_69b0;
    pub const MS_SET_ETERNAL_RELIFE: u32 = 0x0078_deb0;
    pub const MS_STATUS_EFFECT_CHECK: u32 = 0x0079_eff0;
    pub const MS_SET_CHR_WEAK: u32 = 0x0078_d7f0;
    pub const MS_CHECK_AUTO_STATUS: u32 = 0x007a_f260;
    pub const MS_DAMAGE_CHECK_DEATH: u32 = 0x0078_c740;
    pub const MS_RESET_AUTO_STATUS: u32 = 0x0079_b1a0;
    pub const MS_SET_RAM_CHR_PARAM: u32 = 0x0079_c5f0;
    pub const MS_SET_AUTO_STATUS: u32 = 0x0079_b290;
    /// 100.0 (`f64`), the per-cent scale of properties 0x105, 0x128 and
    /// 0x10e-0x110.
    pub const HUNDRED: u32 = 0x00b9_22e0;

    fn game<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32, args: &[u32]) -> u32 {
        let f = mem.rebase(sym);
        call(mem, f, args)
    }

    /// Whether the battle file's name holds the C string at `sym`.
    fn file_has<M: Mem>(mem: &mut M, call: Call<'_, M>, sym: u32) -> bool {
        let strstr: u32 = mem.read(mem.rebase(CRT_STRSTR));
        let (name, s) = (mem.rebase(crate::read::bridge::FILE_NAME), mem.rebase(sym));
        call(mem, strstr, &[name, s]) != 0
    }

    /// `v` (an `i32` as the original's `fild` reads it) per cent: `f32(v)
    /// / 100.0`, divided by the actor's `f32` at `by` when not 0, rounded
    /// to `f32`.
    fn per_cent(mem: &impl Mem, v: u32, by: Option<u32>) -> u32 {
        use ffx_bridge::x87::{div, load, store};
        let hundred = f64::from_bits(mem.read(mem.rebase(HUNDRED)));
        let f = div(load(store(f64::from(v.cast_signed()))), hundred);
        store(match by {
            Some(at) => div(f, load(mem.read(at))),
            None => f,
        })
    }

    /// `MsSetChrStatInfo(chr, kind, selector, v)` (`0x007b4b70`): property
    /// `kind` of every actor `selector` picks (`MsGetChrBit(chr, selector,
    /// 0, 1)`; property 0x88 on battle 0x60ee in `"sins07_07"` picks -5) set
    /// to `v`; how many actors. In `"znkd09_00"` property 0x57 = 4 of
    /// monster 0x14 marks [`ZNKD_HP`], after which its property 0 is 100.
    /// Byte properties take the value's low byte or its bit 0, ranged
    /// ones `MsCheckRange` of the value as it came. Afterwards, per actor:
    /// the status check for [`status`] properties, the weakness and auto
    /// status (and for 0xf2 the death check) for properties 0, 4 and 0xf2,
    /// the stat rebuild for [`rebuild`] ones. Every call to the battle's
    /// other functions goes through `call`.
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ms_set_chr_stat_info<M: Mem>(
        mem: &mut M,
        call: Call<'_, M>,
        chr: u32,
        kind: u32,
        selector: u32,
        v: u32,
    ) -> u32 {
        use crate::actor::bridge::{ms_get_chr, ms_get_chr_bit};
        use ffx_kernel::bridge::ms_check_range;
        let range = |lo: i32, hi: i32| ms_check_range(v.cast_signed(), lo, hi).to_le_bytes()[0];
        let mut bit = v & 1;
        let selector = if kind == 0x88
            && crate::parse::bridge::PARSED_COMMAND.get(mem) == 0x60ee
            && file_has(mem, call, SINS07_07)
        {
            0xffff_fffb
        } else {
            selector
        };
        let mut v = v;
        if !file_has(mem, call, ZNKD09_00) {
            ZNKD_HP.set(mem, 0);
        } else if file_has(mem, call, ZNKD09_00) && chr == 0x14 && kind == 0x57 && v == 4 {
            ZNKD_HP.set(mem, 1);
        } else if ZNKD_HP.get(mem) != 0
            && file_has(mem, call, ZNKD09_00)
            && chr == 0x14
            && kind == 0
        {
            v = 100;
        }
        let bits = ms_get_chr_bit(mem, chr, selector, 0, 1);
        let mut count = 0;
        for i in 0..0x1f {
            if bits.cast_signed() >> i & 1 == 0 {
                continue;
            }
            let a = ms_get_chr(mem, i);
            let at = |o: u32| a.wrapping_add(o);
            let ch: u32 = mem.read(a);
            let info: u32 = mem.read(at(0xf88));
            let v8 = v.to_le_bytes()[0];
            let src = |s: Src, bit: u32| match s {
                Src::V => v,
                Src::Bit => bit,
            };
            // 1: weakness and auto status, 2: and the death check
            let mut weak = 0;
            match set(kind) {
                Set::B(o, s) => mem.write(at(o), src(s, bit).to_le_bytes()[0]),
                #[allow(clippy::cast_possible_truncation)] // the low 16 bits
                Set::W(o) => mem.write(at(o), v as u16),
                Set::D(o) => mem.write(at(o), v),
                Set::Clamp(o, lo, hi) => mem.write(at(o), range(lo, hi)),
                Set::Bit8(o, n, s) => {
                    let old: u8 = mem.read(at(o));
                    let m = 1u8 << n;
                    let set = src(s, bit) & 1 != 0;
                    mem.write(at(o), if set { old | m } else { old & !m });
                }
                Set::Bit16(o, n, s) => {
                    let old: u16 = mem.read(at(o));
                    let m = 1u16 << n;
                    let set = src(s, bit) & 1 != 0;
                    mem.write(at(o), if set { old | m } else { old & !m });
                }
                Set::IB(o) => {
                    if info != 0 {
                        mem.write(info.wrapping_add(o), v8);
                    }
                }
                #[allow(clippy::cast_possible_truncation)] // the low 16 bits
                Set::IW(o) => {
                    if info != 0 {
                        mem.write(info.wrapping_add(o), v as u16);
                    }
                }
                Set::Other => match kind {
                    // HP: alive again with some
                    0 => {
                        weak = 1;
                        let flags: u16 = mem.read(at(0x606));
                        mem.write(at(0x5d0), v);
                        if flags & 1 != 0 && v.cast_signed() > 0 {
                            mem.write(at(0x606), flags & 0xfffe);
                        }
                    }
                    // dead or not
                    4 => {
                        let old: u16 = mem.read(at(0x606));
                        mem.write(at(0x606), old & !1 | (v & 1) as u16);
                        weak = 1;
                        game(mem, call, MS_ALIVE_PROCESS, &[i, a, i]);
                    }
                    9 => {
                        mem.write(at(0x5a8), range(1, 0xff));
                        game(mem, call, MS_SET_CHR_IMAGE_DAMAGE_ATTACK, &[a]);
                    }
                    0x1c | 0x55 => {
                        if kind == 0x1c {
                            mem.write(at(0x4fc), v8);
                        }
                        mem.write(at(0x509), v8);
                        mem.write(at(0x500), v8);
                    }
                    0x1d | 0x56 => {
                        if kind == 0x1d {
                            mem.write(at(0x4fe), v8);
                        }
                        mem.write(at(0x50b), v8);
                        mem.write(at(0x502), v8);
                    }
                    0x1f => mem.write(at(0x504), v8 & 7),
                    0x37 => {
                        let old: u8 = mem.read(at(0x612));
                        mem.write(at(0x612), v8);
                        if v != 0 && old == 0 {
                            mem.write(at(0x6d2), 0u8);
                        }
                    }
                    0x4f => {
                        mem.write(at(0x3f5), v8);
                        mem.write(at(0x3f6), v8);
                    }
                    0x54 => {
                        game(mem, call, MS_SET_CHR_VISIBLE_MAIN, &[i, bit]);
                    }
                    0x57 => {
                        mem.write(at(0xe2c), v);
                        if ch != 0 {
                            game(mem, call, CH_EFFECT_SET_EFFECT_LEVEL, &[ch, v]);
                        }
                    }
                    0x59 | 0x5a => {
                        let first = game(mem, call, BATTLE_FIRST_SELECTED_ACTOR, &[i, v, 0, 1]);
                        let o = if kind == 0x59 { 0x422 } else { 0x416 };
                        mem.write(at(o), first.to_le_bytes()[0]);
                    }
                    0x61 => {
                        mem.write(at(0x50e), bit.to_le_bytes()[0]);
                        mem.write(at(0x50f), 0u8);
                    }
                    0x7f => {
                        game(mem, call, MS_SET_CHR_BODY_HIT, &[a, 0, bit]);
                    }
                    0x97 => {
                        let old: u16 = mem.read(at(0x616));
                        mem.write(at(0x616), old & !0x100 | ((bit & 1) as u16) << 8);
                        game(mem, call, MS_BLOW_PROCESS, &[i, a]);
                    }
                    0xae => {
                        game(mem, call, MS_SET_CHR_VISIBLE_APPEAR, &[i, bit ^ 1]);
                    }
                    0xd8 => {
                        game(mem, call, MS_SET_CHR_VISIBLE_CAMERA, &[i, bit]);
                    }
                    0xed => {
                        game(mem, call, MS_SET_CHR_VISIBLE, &[i, bit]);
                    }
                    0xf1 => {
                        game(mem, call, MS_SET_CHR_SHADOW_MODE, &[i, 0, v]);
                    }
                    0xf2 => {
                        if bit != 0 {
                            weak = 2;
                            mem.write(at(0x5d0), 0u32);
                        }
                    }
                    0xfb => {
                        let f = if bit != 0 {
                            MS_MOVE_RESET
                        } else {
                            MS_MOTION_SPEED_SET_NORMAL
                        };
                        game(mem, call, f, &[a]);
                    }
                    0x105 => {
                        mem.write(at(0x44d), 1u8);
                        let f = per_cent(mem, v, None);
                        mem.write(at(0x4e8), f);
                    }
                    0x128 => {
                        let f = per_cent(mem, v, None);
                        mem.write(at(0x4ec), f);
                    }
                    0x10e..=0x110 => {
                        if ch != 0 {
                            let (by, f) = match kind {
                                0x10e => (0x3a8, CH_SET_THICKNESS_BTL_CAM),
                                0x10f => (0x3a4, CH_SET_CAM_HEIGHT),
                                _ => (0x3a4, CH_SET_FIX_HEIGHT),
                            };
                            let x = per_cent(mem, v, Some(at(by)));
                            game(mem, call, f, &[ch, x]);
                        }
                    }
                    0x111 => {
                        game(mem, call, MS_ADD_YOUJINBOU_COM, &[v]);
                    }
                    0x112 => GIL_AMOUNT.set(mem, v),
                    0x117 => {
                        let n = u32::from(mem.read::<u8>(at(0x6e1))).wrapping_add(v);
                        let n = ms_check_range(n.cast_signed(), 0, 100);
                        mem.write(at(0x6e1), n.to_le_bytes()[0]);
                    }
                    0x118 => {
                        let max = i32::from(mem.read::<u8>(at(0x5bd)));
                        let n = u32::from(mem.read::<u8>(at(0x5bc))).wrapping_add(v);
                        let n = ms_check_range(n.cast_signed(), 0, max);
                        mem.write(at(0x5bc), n.to_le_bytes()[0]);
                    }
                    0x124 => {
                        let w: u32 = mem.read(at(0x4ac));
                        mem.write(at(0x4ac), w & !v);
                    }
                    0x125 => {
                        let w: u32 = mem.read(at(0x4ac));
                        mem.write(at(0x4ac), w | v);
                    }
                    // the motion speed's reset, unless it is speed 7 or 0x10 or
                    // held (bit 7)
                    0x14b => {
                        if bit != 0 {
                            let s: u8 = mem.read(at(0x415));
                            let speed = s & 0x7f;
                            if speed != 7 && speed != 0x10 && s & 0x80 == 0 {
                                game(mem, call, MS_MOTION_SPEED_RESET, &[a]);
                            }
                        }
                    }
                    // body hit 3: an alive actor turns the bit on for the rest
                    0x14c => {
                        if bit == 0 && mem.read::<u8>(at(0xdcc)) == 0 {
                            bit = 1;
                        }
                        game(mem, call, MS_SET_CHR_BODY_HIT, &[a, 3, bit]);
                    }
                    0x14d => {
                        game(mem, call, MS_SET_SAVE_PLY_JOIN, &[i, bit]);
                    }
                    0x14e => {
                        mem.write(at(0x3b), bit.to_le_bytes()[0]);
                        game(mem, call, MS_SET_ETERNAL_RELIFE, &[a]);
                    }
                    _ => {}
                },
            }
            count += 1;
            if status(kind) {
                game(mem, call, MS_STATUS_EFFECT_CHECK, &[i]);
            }
            if weak != 0 {
                game(mem, call, MS_SET_CHR_WEAK, &[i, u32::MAX]);
                game(mem, call, MS_CHECK_AUTO_STATUS, &[i, a]);
                if weak == 2 {
                    game(mem, call, MS_DAMAGE_CHECK_DEATH, &[i, i, 0, 1]);
                }
            }
            if rebuild(kind) {
                game(mem, call, MS_RESET_AUTO_STATUS, &[a]);
                game(mem, call, MS_SET_RAM_CHR_PARAM, &[i]);
                game(mem, call, MS_SET_AUTO_STATUS, &[a]);
            }
        }
        count
    }
}
