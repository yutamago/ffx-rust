//! `MsGetChrStatInfo` (`docs/rewrite-plan.md`, M2): the battle scripts'
//! and AI's question about a group of actors (`MsGetChrBit`'s selector):
//! one of 0x15a properties, summed, counted, averaged or the largest.

use Field::{Bit, Info, Mean, Set, Val};
use Ty::{I8, I16, U8, U16};

/// How a field is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ty {
    U8,
    I8,
    U16,
    I16,
}

/// What a property adds per selected actor (`MsGetChrStatInfo`'s jump
/// table at `0x007b4504`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    /// The actor's field.
    Val(Ty, u32),
    /// Bit `n` of the actor's word at the offset.
    Bit(u32, u32),
    /// The actor's field, counted: the result is the mean.
    Mean(Ty, u32),
    /// 1 when the actor's byte has a bit of the mask.
    Set(u32, u8),
    /// The field of the actor's info record (`+0xf88`) when it has one.
    Info(Ty, u32),
    /// Its own code.
    Other,
}

/// The property `kind`'s field, as the jump table's simple cases read it.
#[allow(clippy::too_many_lines, clippy::match_same_arms)]
const fn field(kind: u32) -> Field {
    match kind {
        0x9 => Mean(U8, 0x5a8),
        0xa => Mean(U8, 0x5a9),
        0xb => Mean(U8, 0x5aa),
        0xc => Mean(U8, 0x5ab),
        0xd => Mean(U8, 0x5ac),
        0xe => Mean(U8, 0x5ad),
        0xf => Mean(U8, 0x5ae),
        0x10 => Mean(U8, 0x5af),
        0x11 => Mean(U8, 0x5ba),
        0x12 => Mean(U8, 0x5bb),
        0x13 | 0x118 => Mean(U8, 0x5bc),
        0x14 => Mean(U8, 0x5bd),
        0x15 => Set(0xdc8, 0xff),
        0x16 => Set(0x590, 0x1),
        0x17 => Set(0x590, 0x2),
        0x18 => Set(0x590, 0x4),
        0x1a => Set(0x505, 0xff),
        0x1c => Mean(I8, 0x4fc),
        0x1d => Mean(I8, 0x4fe),
        0x1e => Val(I8, 0x4ff),
        0x1f => Mean(I8, 0x504),
        0x20 => Bit(0x5b8, 0),
        0x21 => Bit(0x5b8, 1),
        0x22 => Bit(0x5b8, 0x2),
        0x23 => Bit(0x5b8, 0x3),
        0x24 => Bit(0x5b8, 0x4),
        0x25 => Bit(0x606, 0x4),
        0x26 => Bit(0x606, 0x5),
        0x27 => Bit(0x606, 0x6),
        0x28 => Bit(0x606, 0x7),
        0x29 => Bit(0x607, 0),
        0x2a => Bit(0x606, 0x9),
        0x2b => Bit(0x606, 0xa),
        0x2c => Bit(0x606, 0xb),
        0x2d => Set(0x608, 0xff),
        0x2e => Set(0x609, 0xff),
        0x2f => Set(0x60a, 0xff),
        0x30 => Set(0x60b, 0xff),
        0x31 => Set(0x60c, 0xff),
        0x32 => Set(0x60d, 0xff),
        0x33 => Set(0x60e, 0xff),
        0x34 => Set(0x60f, 0xff),
        0x35 => Set(0x610, 0xff),
        0x36 => Set(0x611, 0xff),
        0x37 => Set(0x612, 0xff),
        0x38 => Set(0x613, 0xff),
        0x39 => Set(0x614, 0xff),
        0x3a => Bit(0x6bc, 0),
        0x3b => Bit(0x6bc, 1),
        0x3c => Bit(0x6bc, 0x2),
        0x3d => Bit(0x6bc, 0x3),
        0x3e => Bit(0x6bc, 0x4),
        0x3f => Bit(0x6bc, 0x7),
        0x42 => Bit(0x6bc, 0x6),
        0x43 => Bit(0x6bc, 0x5),
        0x44 => Bit(0x6bc, 0x9),
        0x45 => Bit(0x6bc, 0xa),
        0x46 => Bit(0x6bc, 0xb),
        0x47 => Bit(0x6bc, 0xc),
        0x4a => Bit(0x6bc, 0xd),
        0x4d => Bit(0x6c0, 0),
        0x4e => Bit(0x6c0, 1),
        0x4f => Mean(U8, 0x3f5),
        0x50 => Val(I8, 0xddd),
        0x51 => Val(I8, 0xdd6),
        0x52 => Val(I8, 0xdd9),
        0x53 => Val(I8, 0xdd7),
        0x54 => Val(I8, 0x16),
        0x55 => Val(I8, 0x509),
        0x56 => Val(I8, 0x50b),
        0x58 => Val(I8, 0x24),
        0x59 => Val(I8, 0x422),
        0x5a => Val(U8, 0x416),
        0x5b => Val(I8, 0x420),
        0x5d => Val(U8, 0x412),
        0x5e => Val(U8, 0x43e),
        0x5f => Val(I8, 0x508),
        0x60 => Val(I8, 0x50d),
        0x61 => Val(I8, 0x50e),
        0x62 => Bit(0x5da, 0),
        0x63 => Bit(0x5da, 1),
        0x64 => Bit(0x5da, 0x2),
        0x65 => Bit(0x5da, 0x3),
        0x66 => Bit(0x5da, 0x4),
        0x67 => Bit(0x5db, 0),
        0x68 => Bit(0x5db, 1),
        0x69 => Bit(0x5db, 0x2),
        0x6a => Bit(0x5db, 0x3),
        0x6b => Bit(0x5db, 0x4),
        0x6c => Bit(0x5dc, 0),
        0x6d => Bit(0x5dc, 1),
        0x6e => Bit(0x5dc, 0x2),
        0x6f => Bit(0x5dc, 0x3),
        0x70 => Bit(0x5dc, 0x4),
        0x71 => Bit(0x5dd, 0),
        0x72 => Bit(0x5dd, 1),
        0x73 => Bit(0x5dd, 0x2),
        0x74 => Bit(0x5dd, 0x3),
        0x75 => Bit(0x5dd, 0x4),
        0x76 => Val(U8, 0x426),
        0x77 => Bit(0x447, 0),
        0x79 => Val(U8, 0xdeb),
        0x7a => Val(I8, 0x448),
        0x7b => Val(U8, 0x40a),
        0x7c => Mean(U8, 0x40c),
        0x7d => Val(U8, 0x41e),
        0x7e => Val(U8, 0x41f),
        0x7f | 0x14c => Val(I8, 0x2d),
        0x80 => Val(U8, 0xe0c),
        0x81 => Info(U16, 0x24),
        0x82 => Info(U8, 0x28),
        0x83 => Info(U16, 0x26),
        0x84 => Info(U8, 0x29),
        0x85 => Val(I8, 0x451),
        0x86 => Val(I8, 0x43d),
        0x87 => Val(I8, 0xdf9),
        0x88 => Val(U8, 0x719),
        0x89 => Val(U8, 0x718),
        0x8a => Info(U8, 0x8),
        0x8b => Info(U8, 0x9),
        0x8c => Info(U8, 0xb),
        0x8d => Info(U8, 0xa),
        0x8e => Val(I8, 0xdd2),
        0x8f => Bit(0x616, 0),
        0x90 => Bit(0x616, 1),
        0x91 => Bit(0x616, 0x2),
        0x92 => Bit(0x616, 0x3),
        0x93 => Bit(0x616, 0x4),
        0x94 => Bit(0x616, 0x5),
        0x95 => Bit(0x616, 0x6),
        0x96 => Bit(0x616, 0x7),
        0x97 => Bit(0x617, 0),
        0x98 => Bit(0x616, 0x9),
        0x99 => Bit(0x616, 0xa),
        0x9a => Bit(0x616, 0xb),
        0x9b => Bit(0x616, 0xc),
        0x9c => Bit(0x616, 0xd),
        0x9d => Bit(0x616, 0xe),
        0x9f => Val(U8, 0x5c9),
        0xa0 => Val(U8, 0x5c8),
        0xa1 => Val(I8, 0x44e),
        0xa2 => Val(U8, 0x41c),
        0xa3 => Val(U8, 0x41d),
        0xa4 => Val(U8, 0x41b),
        0xa5 => Val(I8, 0x433),
        0xa9 => Val(U8, 0x423),
        0xaa => Val(U8, 0xe0e),
        0xab => Val(U8, 0x425),
        0xac => Val(I8, 0xdd3),
        0xad => Val(I8, 0xdcd),
        0xaf => Val(U8, 0x641),
        0xb0 => Val(U8, 0x642),
        0xb1 => Val(U8, 0x643),
        0xb2 => Val(U8, 0x644),
        0xb3 => Val(U8, 0x645),
        0xb4 => Val(U8, 0x646),
        0xb5 => Val(U8, 0x647),
        0xb6 => Val(U8, 0x648),
        0xb7 => Val(U8, 0x649),
        0xb8 => Val(U8, 0x64a),
        0xb9 => Val(U8, 0x64b),
        0xba => Val(U8, 0x64c),
        0xbb => Val(U8, 0x64d),
        0xbc => Val(U8, 0x64e),
        0xbd => Val(U8, 0x64f),
        0xbe => Val(U8, 0x650),
        0xbf => Val(U8, 0x651),
        0xc0 => Val(U8, 0x652),
        0xc1 => Val(U8, 0x653),
        0xc2 => Val(U8, 0x654),
        0xc3 => Val(U8, 0x655),
        0xc4 => Val(U8, 0x656),
        0xc5 => Val(U8, 0x657),
        0xc6 => Val(U8, 0x658),
        0xc7 => Val(U8, 0x659),
        0xc8 => Bit(0x65a, 0),
        0xc9 => Bit(0x65a, 1),
        0xca => Bit(0x65a, 0x2),
        0xcb => Bit(0x65a, 0x3),
        0xcc => Bit(0x65a, 0x4),
        0xcd => Bit(0x65a, 0x5),
        0xce => Bit(0x65a, 0x6),
        0xcf => Bit(0x65a, 0x7),
        0xd0 => Bit(0x65a, 0x9),
        0xd1 => Bit(0x65b, 0),
        0xd2 => Bit(0x65a, 0xa),
        0xd3 => Bit(0x65a, 0xb),
        0xd4 => Bit(0x65a, 0xc),
        0xd5 => Bit(0x65a, 0xd),
        0xd6 => Bit(0x65a, 0xe),
        0xd7 => Val(I8, 0xdd8),
        0xd8 => Val(I8, 0x19),
        0xd9 => Val(I8, 0x1b),
        0xdc => Val(I8, 0x43b),
        0xde => Val(U8, 0x23),
        0xe0 => Val(U8, 0x446),
        0xe1 => Val(I8, 0xdda),
        0xe2 => Val(U8, 0xe10),
        0xe3 => Val(U8, 0xe11),
        0xe4 => Val(I8, 0x4b8),
        0xe5 => Val(U8, 0x408),
        0xe6 => Val(U8, 0x4b9),
        0xe7 => Val(U8, 0x4ba),
        0xe8 => Val(U8, 0x65c),
        0xe9 => Val(U8, 0x424),
        0xea => Val(U8, 0x4bb),
        0xeb => Val(U8, 0xf74),
        0xec => Val(U8, 0xf75),
        0xed => Val(I8, 0x18),
        0xee => Val(I8, 0x21),
        0xef => Val(I8, 0x22),
        0xf0 => Bit(0x5b9, 0),
        0xf1 => Val(I8, 0x26),
        0xf5 => Val(I8, 0x43c),
        0xf6 => Val(U8, 0x6db),
        0xf7 => Val(I8, 0x44a),
        0xfa => Val(I8, 0x44b),
        0xfb => Val(U16, 0x400),
        0xfc => Val(U8, 0x405),
        0xfd => Val(U8, 0x4b4),
        0x100 => Val(U8, 0x5cb),
        0x101 => Val(U8, 0x5c4),
        0x102 => Val(U8, 0x6ce),
        0x103 => Val(U8, 0x5c3),
        0x104 => Val(I16, 0xe1a),
        0x106 => Val(U8, 0x6d1),
        0x107 => Bit(0x5b8, 0x7),
        0x108 => Bit(0x5b8, 0x6),
        0x109 => Bit(0x5b8, 0x5),
        0x10a => Info(U16, 0x6),
        0x10b => Bit(0x5b8, 0x9),
        0x10d => Val(U8, 0x409),
        0x114 => Val(U8, 0x6df),
        0x115 => Val(U8, 0x6e0),
        0x116 => Val(U8, 0x40d),
        0x117 => Val(U8, 0x6e1),
        0x121 => Val(U8, 0x44f),
        0x122 => Bit(0x5b8, 0xa),
        0x123 => Val(U8, 0x40e),
        0x126 => Val(U8, 0x700),
        0x127 => Val(I8, 0xdca),
        0x129 => Val(U8, 0x3f8),
        0x12e => Val(U8, 0x5b0),
        0x12f => Val(U8, 0x5b1),
        0x130 => Val(U8, 0x5b2),
        0x131 => Val(U8, 0x5b3),
        0x132 => Val(U8, 0x5b4),
        0x133 => Val(U8, 0x5b5),
        0x134 => Val(U8, 0x5b6),
        0x135 => Val(U8, 0x5b7),
        0x136 => Val(U8, 0x6cc),
        0x137 => Val(U8, 0x6cd),
        0x138 => Val(U8, 0x6cf),
        0x139 => Val(U8, 0x6d0),
        0x13a => Info(U16, 0xc),
        0x13b => Info(U16, 0xe),
        0x13c => Info(U16, 0x10),
        0x13d => Info(U16, 0x12),
        0x13e => Info(U16, 0x18),
        0x13f => Info(U16, 0x1a),
        0x140 => Info(U16, 0x1c),
        0x141 => Info(U16, 0x1e),
        0x142 => Info(U8, 0x14),
        0x143 => Info(U8, 0x15),
        0x144 => Info(U8, 0x16),
        0x145 => Info(U8, 0x17),
        0x146 => Info(U8, 0x20),
        0x147 => Info(U8, 0x21),
        0x148 => Info(U8, 0x22),
        0x149 => Info(U8, 0x23),
        0x14a => Val(U8, 0x3f9),
        0x14e => Val(U8, 0x3b),
        0x14f => Val(U8, 0x437),
        0x150 => Val(I8, 0x1c),
        0x151 => Val(I8, 0xddb),
        0x152 => Val(U8, 0x442),
        0x153 => Val(I8, 0x39),
        0x154 => Val(U8, 0x3fb),
        0x156 => Val(I8, 0xdd1),
        0x157 => Val(I8, 0x506),
        0x158 => Val(I8, 0xdd5),
        _ => Field::Other,
    }
}

/// The game's memory, through [`ffx_bridge::Mem`].
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    use super::{Field, Ty, field};
    use crate::pos::bridge::Trig;

    /// `0x0112a907`: property 0x1b counts actors whose `+0xdec` is clear
    /// instead of set.
    pub const STAT_INVERT_1B: Global<u8> = Global::new(0x0112_a907);
    /// 100.0, the scale of the float properties.
    pub const HUNDRED: Global<f64> = Global::new(0x00b9_22e0);

    fn get(mem: &impl Mem, at: u32, ty: Ty) -> u32 {
        match ty {
            Ty::U8 => u32::from(mem.read::<u8>(at)),
            Ty::I8 => i32::from(mem.read::<i8>(at)).cast_unsigned(),
            Ty::U16 => u32::from(mem.read::<u16>(at)),
            Ty::I16 => i32::from(mem.read::<i16>(at)).cast_unsigned(),
        }
    }

    /// `MsGetChrStatInfo(chr, kind, selector, out)` (`0x007b2dc0`): property
    /// `kind` (0-0x159) over the actors `MsGetChrBit(chr, selector, 0, 1)`
    /// selects. Most add up a field, a bit or a condition (1 per actor);
    /// the "mean" ones (HP, MP, stats, overdrive) also count, and then
    /// `out` gets the sum over the count (signed) and the count is
    /// returned. Otherwise `out` (when not null) gets the sum and so does
    /// the return. 0x113 takes the largest, 0x12b-0x12d the last actor's.
    /// Property 0xf4 asks [`crate::pos::bridge::ms_check_btl_pos`]
    /// (`trig` for its summoners).
    #[allow(clippy::too_many_lines)]
    pub fn ms_get_chr_stat_info<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        chr: u32,
        kind: u32,
        selector: u32,
        out: u32,
    ) -> u32 {
        use crate::actor::bridge as a;
        use ffx_bridge::x87::{ftol, load, mul};
        let bits = a::ms_get_chr_bit(mem, chr, selector, 0, 1);
        let mut sum = 0u32;
        let mut count = 0u32;
        for i in 0..0x1f {
            if bits.cast_signed() >> i & 1 == 0 {
                continue;
            }
            let actor = a::ms_get_chr(mem, i);
            let info: u32 = mem.read(actor.wrapping_add(0xf88));
            let at = |o: u32| actor.wrapping_add(o);
            let byte = |mem: &M, o: u32| mem.read::<u8>(actor.wrapping_add(o));
            let one = |c: bool| u32::from(c);
            let alive = |mem: &M| {
                a::ms_get_chr_stat_death(mem, i) == 0 && a::ms_get_chr_stat_stone(mem, i) == 0
            };
            // actor in battle (+0xdc8), alive, not stone, not +0x608/+0x609
            let able = |mem: &M| {
                byte(mem, 0xdc8) != 0
                    && alive(mem)
                    && byte(mem, 0x608) == 0
                    && byte(mem, 0x609) == 0
            };
            let free = |mem: &M| able(mem) && mem.read::<u16>(at(0x606)) & 0x300 == 0;
            let add = match field(kind) {
                Field::Val(ty, o) => get(mem, at(o), ty),
                Field::Bit(o, n) => u32::from(mem.read::<u16>(at(o))) >> n & 1,
                Field::Mean(ty, o) => {
                    count = count.wrapping_add(1);
                    get(mem, at(o), ty)
                }
                Field::Set(o, m) => one(byte(mem, o) & m != 0),
                Field::Info(ty, o) => {
                    if info == 0 {
                        0
                    } else {
                        get(mem, info.wrapping_add(o), ty)
                    }
                }
                Field::Other => match kind {
                    0..=3 | 8 => {
                        count = count.wrapping_add(1);
                        match kind {
                            0 => a::ms_get_ram_chr_hp(mem, i),
                            1 => a::ms_get_ram_chr_mp(mem, i),
                            2 => a::ms_get_ram_chr_hp_max(mem, i),
                            3 => a::ms_get_ram_chr_mp_max(mem, i),
                            _ => a::ms_get_chr_stat_weak(mem, i),
                        }
                    }
                    4 => one(a::ms_get_chr_stat_death(mem, i) == 0),
                    5 => one(a::ms_get_chr_stat_poison(mem, i) == 1),
                    6 => one(a::ms_get_chr_stat_stone(mem, i) == 1),
                    7 => one(a::ms_get_chr_stat_zombie(mem, i) == 1),
                    0xf2 => one(a::ms_get_chr_stat_death(mem, i) == 1),
                    0x19 => one(a::ms_get_ram_chr_monster(i) != 0),
                    0x1b => {
                        let set = byte(mem, 0xdec) != 0;
                        one(if STAT_INVERT_1B.get(mem) != 0 {
                            !set
                        } else {
                            set
                        })
                    }
                    0x4c => u32::from(mem.read::<u16>(at(0x6be))) >> 15,
                    0x57 => {
                        let c: u32 = mem.read(actor);
                        if c == 0 {
                            0
                        } else {
                            ffx_chara::chr::bridge::ch_effect_get_effect_level(mem, c)
                        }
                    }
                    0x5c => 1,
                    0x78 => u32::from(byte(mem, 0x447) & 2),
                    0x9e | 0x124 | 0x125 => crate::body::bridge::battle_chr_move_flags(mem, actor),
                    0xa6..=0xa8 => mem.read(at(0xd34 + (kind - 0xa6) * 4)),
                    0xae => get(mem, at(0x17), Ty::I8) ^ 1,
                    0xda | 0xdb => {
                        let v = i32::from(mem.read::<u16>(at(0x4b0 + (kind - 0xda) * 2)));
                        let d = i32::from(mem.read::<u16>(at(0x400)));
                        if d == 0 { v } else { (v << 8) / d }.cast_unsigned()
                    }
                    0xdd => a::ms_get_chr_vigor(mem, i),
                    0xdf => one(alive(mem)),
                    0xf3 => one(a::ms_get_chr_stat_death(mem, i) == 1
                        || a::ms_get_chr_stat_stone(mem, i) == 1),
                    0xf4 => {
                        let flat = get(mem, at(0x34), Ty::I8);
                        one(crate::pos::bridge::ms_check_btl_pos(mem, trig, i, actor, flat) != 0)
                    }
                    0xf8 => one(able(mem)),
                    0xf9 => one(free(mem)),
                    0xfe => one(free(mem) && a::ms_get_chr_vigor(mem, i) != 0),
                    0xff => a::ms_auto_command_status_check(mem, actor, 0, 0, 1),
                    0x105 => {
                        let f: u32 = mem.read(at(0x4e8));
                        ftol(mem, mul(load(f), HUNDRED.get(mem)))
                    }
                    0x128 => {
                        let f: u32 = mem.read(at(0x4ec));
                        ftol(mem, load(f)).wrapping_mul(100)
                    }
                    0x10c => mem.read(at(0x5a4)),
                    0x10e..=0x110 => {
                        let f = match kind {
                            0x10e => crate::body::bridge::ms_get_chr_cam_width(mem, i),
                            0x10f => crate::body::bridge::ms_get_chr_cam_height(mem, i),
                            _ => crate::body::bridge::ms_get_chr_height(mem, i),
                        };
                        ftol(mem, load(f.to_bits())).wrapping_mul(100)
                    }
                    0x111 => crate::save::bridge::ms_get_youjinbou_com(mem),
                    0x112 => crate::damage::bridge::GIL_AMOUNT.get(mem).cast_unsigned(),
                    0x113 => {
                        if info != 0 {
                            let v = u32::from(mem.read::<u8>(info.wrapping_add(0x112)));
                            if v.cast_signed() > sum.cast_signed() {
                                sum = v;
                            }
                        }
                        0
                    }
                    0x119 => {
                        let hp: u32 = mem.read(at(0x5d0));
                        let max: u32 = mem.read(at(0x594));
                        one(hp < max >> 1)
                    }
                    0x11a => {
                        let sister = (0xf..=0x11).contains(&i);
                        let full = byte(mem, 0x5bc) >= byte(mem, 0x5bd)
                            || crate::damage::bridge::DEBUG_NO_COST.get(mem) != 0;
                        one(sister
                            && full
                            && byte(mem, 0xdcc) == 0
                            && mem.read::<u16>(at(0x616)) & 0x400 == 0
                            && a::ms_auto_command_status_check(mem, actor, 1, 1, 1) != 0)
                    }
                    0x11b | 0x11c => {
                        let o = (kind - 0x11b) * 4;
                        let now: u32 = mem.read(at(0x5d0 + o));
                        let max: u32 = mem.read(at(0x594 + o));
                        one(now >= max)
                    }
                    0x11d => one((0x60e..=0x611).all(|o| byte(mem, o) != 0)),
                    0x11e..=0x120 => {
                        let o = [0x60b, 0x60c, 0x613][(kind - 0x11e) as usize];
                        one(byte(mem, o) != 0 || byte(mem, 0x60d) != 0)
                    }
                    0x12a => {
                        let targets: u32 = mem.read(at(0xdc0));
                        one(a::ms_check_same_target(mem, i, actor, targets) == 0)
                    }
                    0x12b..=0x12d => {
                        if info != 0 {
                            let o = [2, 4, 0][(kind - 0x12b) as usize];
                            sum = u32::from(mem.read::<u16>(info.wrapping_add(o)));
                        }
                        0
                    }
                    0x14d => crate::save::bridge::ms_get_save_ply_join(mem, i),
                    0x155 => one(byte(mem, 0xdc8) != 0
                        && a::ms_get_chr_stat_stone(mem, i) == 0
                        && byte(mem, 0x617) & 1 == 0),
                    0x159 => u32::from(mem.read::<u16>(at(0xdde))),
                    _ => 0,
                },
            };
            sum = sum.wrapping_add(add);
        }
        let (value, ret) = if count.cast_signed() > 0 {
            (
                (sum.cast_signed() / count.cast_signed()).cast_unsigned(),
                count,
            )
        } else {
            (sum, sum)
        };
        if out != 0 {
            mem.write(out, value);
        }
        ret
    }

    /// `MsActTargetSearch(chr, selector, kind, _, rng, mode, resolved,
    /// pick)` (`0x007acd40`): the actors of `selector` (a bit set itself
    /// with `resolved`, else through `MsGetChrBit(chr, selector, 0, 1)`)
    /// whose property `kind` ([`ms_get_chr_stat_info`] of the one actor) is
    /// nonzero (with mode bit 0x80: zero). Mode 1 keeps those with the
    /// highest value (`out`), 2 the lowest, 0 all. Returns their bits; `pick`
    /// (when not null) gets one: of several, the `brnd(rng) % n`-th (signed),
    /// else the last kept (0xff for none).
    #[allow(clippy::too_many_arguments)]
    pub fn ms_act_target_search<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        chr: u32,
        selector: u32,
        kind: u32,
        rng: u32,
        mode: u32,
        resolved: u32,
        pick: u32,
    ) -> u32 {
        let bits = if resolved != 0 {
            selector
        } else {
            crate::actor::bridge::ms_get_chr_bit(mem, chr, selector, 0, 1)
        };
        let zero = mode & 0x80 != 0;
        let mode = mode & 0x7f;
        let value = SEARCH_FRAME;
        let mut set = 0u32;
        let mut count = 0i32;
        let mut chosen = 0xffu32;
        let mut best = 0i32;
        for i in 0..0x1f {
            if bits.cast_signed() >> i & 1 == 0 {
                continue;
            }
            let mut f = ffx_bridge::frame::Frame::new(mem, SEARCH_FRAME, 4);
            let r = ms_get_chr_stat_info(&mut f, trig, chr, kind, i, value);
            let v: i32 = f.read(value);
            if (r != 0) == zero {
                continue;
            }
            let bit = 1 << i;
            if chosen == 0xff || mode == 0 {
                set |= bit;
                count += 1;
                chosen = i;
                best = v;
            } else if best == v {
                set |= bit;
                count += 1;
            } else if (mode == 2 && best > v) || (mode == 1 && best < v) {
                set = bit;
                count = 1;
                chosen = i;
                best = v;
            }
        }
        if count > 1 {
            if pick == 0 {
                return set;
            }
            let r = crate::rng::bridge::brnd(mem, rng).cast_signed() % count;
            let mut n = 0;
            for i in 0..0x1f {
                if set.cast_signed() >> i & 1 != 0 {
                    if n == r {
                        chosen = i;
                        break;
                    }
                    n += 1;
                }
            }
        }
        if pick != 0 {
            mem.write(pick, chosen);
        }
        set
    }

    /// [`ms_act_target_search`]'s value (on the original's stack).
    pub const SEARCH_FRAME: u32 = 0xff90_0000;

    /// `0x0112c9fb`: set when a guard takes a hit for its target.
    pub const GUARD_TOOK: Global<u8> = Global::new(0x0112_c9fb);
    /// `0x0112c9fa`: set when a command was reflected.
    pub const REFLECTED: Global<u8> = Global::new(0x0112_c9fa);

    /// `MsCalcCommandTarget(user, hits, counts, command, event, &targets,
    /// &total)` (`0x0078ba70`): `MsCalcCommand`'s target step. Each target
    /// of `*targets` gets the command's hit count (`+0x2b`) in `counts[i]`
    /// (random targets, `+0x1c` bit 0x8000: that many random picks among
    /// them); `*total` grows by the hits. A single-target command (`+0x20`
    /// & 3 == 1, `+0x1a` & 0xc clear) goes to the target's guard
    /// (`MsGetGuardChr`; the guard records the user `+0x6d3` and target
    /// `+0x6d4`, the target loses `+0x6dd`; Tidus guarding a party member
    /// from a monster earns its affection). With `+0x1c` bit 0x80 a target
    /// under reflect (`+0x60d`, or `+0x62c` bit 0x20) sends it back: to the
    /// user, or (both on one side) to a random actor of the other side the
    /// cursor may pick. The final targets get `hits[i] += 1` and `+0xded` =
    /// `user`; they are stored in `*targets` and `g_BtlChrSet`. Returns the
    /// reflecting targets' bits.
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub fn ms_calc_command_target<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        user: u32,
        hits: u32,
        counts: u32,
        command: u32,
        event: u32,
        targets: u32,
        total: u32,
    ) -> u32 {
        use crate::actor::bridge as a;
        let mut sum: u32 = mem.read(total);
        let mut set: u32 = mem.read(targets);
        mem.write(hits, [0u8; 0x1f]);
        let n = u32::from(mem.read::<u8>(command.wrapping_add(0x2b)));
        let flags: u32 = mem.read(command.wrapping_add(0x1c));
        let count = |i: u32| counts.wrapping_add(i);
        let bump = |mem: &mut M, at: u32, by: u8| {
            let v = mem.read::<u8>(at).wrapping_add(by);
            mem.write(at, v);
        };
        let pick_one = |mem: &mut M, selector: u32, rng: u32| -> u32 {
            let mut f = ffx_bridge::frame::Frame::new(mem, TARGET_FRAME, 4);
            let at = f.at(0);
            ms_act_target_search(&mut f, trig, user, selector, 0x5c, rng, 0, 1, at);
            f.read(at)
        };
        if flags & 0x8000 != 0 {
            mem.write(counts, [0u8; 0x1f]);
            for _ in 0..n {
                let p = pick_one(mem, set, 5);
                if p != 0xff {
                    bump(mem, count(p), 1);
                }
            }
            sum = sum.wrapping_add(n);
        } else {
            for i in 0..0x1f {
                if set.cast_signed() >> i & 1 != 0 {
                    #[allow(clippy::cast_possible_truncation)] // its low byte
                    mem.write(count(i), n as u8);
                    sum = sum.wrapping_add(n);
                } else {
                    mem.write(count(i), 0u8);
                }
            }
        }
        let range = mem.read::<u8>(command.wrapping_add(0x20)) & 3;
        let kinds = mem.read::<u8>(command.wrapping_add(0x1a)) & 0xc;
        if range == 1 && kinds == 0 {
            let mut guarded = 0u32;
            for i in 0..0x1f {
                if set.cast_signed() >> i & 1 == 0 {
                    continue;
                }
                let g = if i == user {
                    i
                } else {
                    a::ms_get_guard_chr(mem, user, i)
                };
                guarded |= 1 << (g & 0x1f);
                if i == g {
                    continue;
                }
                GUARD_TOOK.set(mem, 1);
                let ga = a::ms_get_chr(mem, g);
                #[allow(clippy::cast_possible_truncation)] // bytes
                {
                    mem.write(ga.wrapping_add(0x6d3), user as u8);
                    mem.write(ga.wrapping_add(0x6d4), i as u8);
                }
                let c = mem.read::<u8>(count(i));
                bump(mem, count(g), c);
                mem.write(count(i), 0u8);
                let ta = a::ms_get_chr(mem, i);
                mem.write(ta.wrapping_add(0x6dd), 0u8);
                if g == 0 && i < 8 && a::ms_get_ram_chr_monster(user) == 1 {
                    crate::save::bridge::ms_add_love_param(mem, i.cast_signed(), 1);
                }
            }
            set = guarded;
        }
        let reflectable = flags >> 7 & 1 != 0;
        let mut reflected = 0u32;
        let mut done = 0u32;
        let user_monster = a::ms_get_ram_chr_monster(user);
        for i in 0..0x1f {
            if set.cast_signed() >> i & 1 == 0 {
                continue;
            }
            let mut to = i;
            let ta = a::ms_get_chr(mem, i);
            let reflect = mem.read::<u8>(ta.wrapping_add(0x60d)) != 0
                || mem.read::<u8>(ta.wrapping_add(0x62c)) & 0x20 != 0;
            if reflectable && reflect {
                reflected |= 1 << i;
                let monster = a::ms_get_ram_chr_monster(i);
                if user_monster == monster {
                    let side = (-15i32 + i32::from(monster != 0)).cast_unsigned();
                    let bits = a::ms_get_cursor_target(mem, user, side, command, event);
                    to = pick_one(mem, bits, 6);
                    if to != 0xff {
                        let c = mem.read::<u8>(count(i));
                        bump(mem, count(to), c);
                    }
                } else {
                    to = user;
                    let c = mem.read::<u8>(count(i));
                    bump(mem, count(user), c);
                }
                mem.write(count(i), 0u8);
            }
            if to != 0xff {
                bump(mem, hits.wrapping_add(to), 1);
                done |= 1 << (to & 0x1f);
                let ta = a::ms_get_chr(mem, to);
                #[allow(clippy::cast_possible_truncation)] // a byte
                mem.write(ta.wrapping_add(0xded), user as u8);
            }
        }
        if reflected != 0 {
            REFLECTED.set(mem, 1);
        }
        a::CHR_SET.set(mem, done);
        mem.write(targets, done);
        mem.write(total, sum);
        reflected
    }

    /// [`ms_calc_command_target`]'s picks (on the original's stack).
    pub const TARGET_FRAME: u32 = 0xff80_0000;

    /// [`ms_calc_command`]'s locals (on the original's stack): the command
    /// ID, the hit total, the drain sums (12 bytes), the hits and misses
    /// counts, then the per-actor hit and count bytes.
    pub const CALC_COMMAND_FRAME: u32 = 0xff70_0000;

    /// `MsCalcCommand(cue, k)` (`0x007892e0`): runs command `k` of a cue
    /// entry (`+0` the actor, `+3` the commands, each 16 bytes from `+8`:
    /// IDs, then targets at `+0x10`): the record (`MsGetCommand`, ID to the
    /// actor's `+0xf5c`), its CTB cost (`+0xde8`), the targets
    /// ([`ms_calc_command_target`], reflecting ones at `+0x14`), and for
    /// each target hit an action buffer with `MsCalcDamage`, steal
    /// (`+0x1c` bit 0x200) and mug (`+0x21` bit 1) results, flags and the
    /// target's `+0xdd0` (bit 0x40000 with the user's `+0x6c0` bit 4 and
    /// steals allowed). A drain command (`+0x1c` bit 0x100) fills the
    /// user's own buffer (`MsCalcDrainResult`; a hit on itself counts). The
    /// hit total goes to the user's `+0x6db` and is returned (0 for a null
    /// cue). `lang` and `achievement` are `TOGetFFXLang` and the achievement
    /// unlock (main's).
    #[allow(clippy::too_many_lines, clippy::many_single_char_names)]
    pub fn ms_calc_command<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        cue: u32,
        k: u32,
        mut lang: impl FnMut(&mut M) -> u32,
        mut achievement: impl FnMut(&mut M, u32),
    ) -> u32 {
        use crate::actor::bridge::ms_get_chr;
        use crate::damage::bridge::{self as d, DmgCalcOut};
        use crate::save::bridge as s;
        if cue == 0 {
            return 0;
        }
        let mut f = ffx_bridge::frame::Frame::new(mem, CALC_COMMAND_FRAME, 0x60);
        let mem = &mut f;
        let (idp, total, drain, hits_n, misses) =
            (mem.at(0), mem.at(4), mem.at(8), mem.at(0x14), mem.at(0x18));
        let (hits, counts) = (mem.at(0x20), mem.at(0x40));
        let chr = u32::from(mem.read::<u8>(cue));
        let user = ms_get_chr(mem, chr);
        let entry = cue.wrapping_add(k.wrapping_mul(16));
        let record =
            crate::command::bridge::ms_get_command(mem, u32::MAX, entry.wrapping_add(8), idp);
        let id: u32 = mem.read(idp);
        #[allow(clippy::cast_possible_truncation)] // the low half, the low byte
        {
            mem.write(user.wrapping_add(0xf5c), id as u16);
            let cost = crate::command::bridge::ms_calc_cost_ctb(mem, cue);
            mem.write(user.wrapping_add(0xde8), cost as u8);
        }
        let n = u32::from(mem.read::<u8>(cue.wrapping_add(3)));
        if k.cast_signed() < n.cast_signed() && record != 0 {
            let flags: u32 = mem.read(record.wrapping_add(0x1c));
            let mut last = -1i32;
            let mut on_self = 0u32;
            let mut own = 0;
            if flags & 0x100 != 0 {
                own = s::ms_get_action_buffer(mem, user);
                if own != 0 {
                    #[allow(clippy::cast_possible_truncation)] // bytes
                    mem.write(own.wrapping_add(2), [chr as u8, k as u8]);
                }
            }
            let reflected = ms_calc_command_target(
                mem,
                trig,
                chr,
                hits,
                counts,
                record,
                id,
                entry.wrapping_add(0x10),
                total,
            );
            mem.write(entry.wrapping_add(0x14), reflected);
            for t in 0..0x1f {
                if mem.read::<u8>(hits.wrapping_add(t)) == 0 {
                    continue;
                }
                if own != 0 && t == chr {
                    on_self += 1;
                    continue;
                }
                last = t.cast_signed();
                mem.write(misses, 0u32);
                let target = ms_get_chr(mem, t);
                let buffer = s::ms_get_action_buffer(mem, target);
                if buffer == 0 {
                    continue;
                }
                let v: u32 = mem.read(target.wrapping_add(0x5a4));
                mem.write(target.wrapping_add(0xf60), v);
                let out = DmgCalcOut {
                    damage: drain,
                    hits: hits_n,
                    misses,
                    changed: 0,
                };
                let c = i32::from(mem.read::<u8>(counts.wrapping_add(t)));
                d::ms_calc_damage(mem, chr, user, t, target, record, id, buffer, out, c, |m| {
                    lang(m.inner())
                });
                let effect = mem.read::<u8>(record.wrapping_add(0x58));
                mem.write(buffer.wrapping_add(0x14), effect);
                #[allow(clippy::cast_possible_truncation)] // bytes
                mem.write(buffer.wrapping_add(2), [chr as u8, k as u8]);
                if flags & 0x0200_0000 != 0 && mem.read::<u8>(target.wrapping_add(0xdce)) == 0 {
                    let v = mem.read::<u8>(buffer.wrapping_add(4)) | 1;
                    mem.write(buffer.wrapping_add(4), v);
                }
                if mem.read::<u8>(record.wrapping_add(0x1f)) & 1 != 0 {
                    mem.write(buffer.wrapping_add(6), 1u8);
                }
                let spoils: u32 = mem.read(target.wrapping_add(0xf88));
                let fail: i32 = mem.read(misses);
                if flags & 0x200 != 0 {
                    s::battle_steal(
                        mem,
                        user,
                        target,
                        spoils,
                        buffer,
                        fail,
                        |m| lang(m.inner()),
                        |m, a| achievement(m.inner(), a),
                    );
                } else {
                    mem.write(buffer.wrapping_add(7), 0u8);
                    mem.write(buffer.wrapping_add(8), 0u32);
                }
                if mem.read::<u8>(record.wrapping_add(0x21)) & 1 != 0 {
                    let fail: i32 = mem.read(misses);
                    s::battle_steal_gil(mem, target, spoils, buffer, fail);
                } else {
                    mem.write(buffer.wrapping_add(0xc), 0u8);
                    mem.write(buffer.wrapping_add(0x10), 0u32);
                }
                let mark = flags & 0x4_0000 != 0
                    && mem.read::<u8>(user.wrapping_add(0x6c0)) & 4 != 0
                    && s::STEAL_RULE.get(mem) == 0;
                mem.write(target.wrapping_add(0xdd0), u8::from(mark));
            }
            if own != 0 {
                let hit: u32 = mem.read(hits_n);
                if hit != 0 || on_self != 0 {
                    d::ms_calc_drain_result(
                        mem,
                        chr,
                        user,
                        record,
                        own,
                        drain,
                        i32::from(hit == 0),
                    );
                } else {
                    mem.write(own.wrapping_add(2), 0xffu16);
                }
                if id == 0x3020 && last >= 0 {
                    let v = mem.read::<u8>(own.wrapping_add(4)) | 2;
                    mem.write(own.wrapping_add(4), v);
                    mem.write(own.wrapping_add(0x17), last.to_le_bytes()[0]);
                }
            }
        }
        let t: u32 = mem.read(total);
        #[allow(clippy::cast_possible_truncation)] // the low byte
        mem.write(user.wrapping_add(0x6db), t as u8);
        t
    }

    /// `MsGetComData`'s record for IDs without a table (and its default).
    pub const NO_COMMAND: Global<u8> = Global::new(0x0113_6008);

    /// `MsGetComData(id, &name)` (`0x0079a4b0`): the kernel record of a
    /// command, item or ability ID by its kind (`id >> 12`: 2 items, 3
    /// commands, 4 and 6 monster commands, 8 abilities; `name` gets the
    /// string table), else [`NO_COMMAND`] with `name` the actor's (0), the
    /// equipment's (5, 7, 0xb) or the key item's (0xa) name.
    pub fn ms_get_com_data(mem: &mut impl Mem, id: u32, name: u32) -> u32 {
        let (record, s) = com_data(mem, id, name != 0);
        if let Some(s) = s {
            mem.write(name, s);
        }
        record
    }

    /// `MsGetComInfo(i, id, what)` (`0x0078cd30`): a field of command `id`'s
    /// record ([`ms_get_com_data`]; 0 without one): 0 its byte `+0x28`
    /// (with `+0x1c` bit 0x40000 actor `i`'s `+0x5c1` instead), 1 `+0x20`
    /// & 3, 2-4 `+0x23` bits 1, 2, 4, 5-9 bits 4 down to 0 of `+0x2d`
    /// (under that bit with the actor's `+0x5d9` bits as well), 10 `+0x1a`
    /// >> 2 & 3; others 0.
    pub fn ms_get_com_info(mem: &mut impl Mem, i: u32, id: u32, what: u32) -> u32 {
        let rec = ms_get_com_data(mem, id, 0);
        if rec == 0 {
            return 0;
        }
        let actor = crate::actor::bridge::ms_get_chr(mem, i);
        let (first, flags) = if mem.read::<u32>(rec + 0x1c) & 0x4_0000 == 0 {
            (mem.read::<u8>(rec + 0x28), mem.read::<u8>(rec + 0x2d))
        } else {
            (
                mem.read::<u8>(actor.wrapping_add(0x5c1)),
                mem.read::<u8>(actor.wrapping_add(0x5d9)) | mem.read::<u8>(rec + 0x2d),
            )
        };
        let flags = u32::from(flags);
        match what {
            0 => u32::from(first),
            1 => u32::from(mem.read::<u16>(rec + 0x20)) & 3,
            2..=4 => u32::from(mem.read::<u8>(rec + 0x23)) & (1 << (what - 2)),
            5..=9 => flags >> (9 - what) & 1,
            10 => u32::from(mem.read::<u16>(rec + 0x1a)) >> 2 & 3,
            _ => 0,
        }
    }

    /// [`ms_get_com_data`] for Rust callers: the record and, when `name`
    /// is asked for, the name it would store (`None` for the kinds that
    /// leave it alone).
    pub fn com_data<M: Mem>(mem: &mut M, id: u32, name: bool) -> (u32, Option<u32>) {
        use ffx_kernel::bridge::{self as k, rom, rom_strings};
        let i = id & 0xfff;
        let none = NO_COMMAND.ptr(mem).addr;
        let kind = id.cast_signed() >> 12;
        let record = |mem: &mut M| match kind {
            2 => rom(mem, k::ITEM, i, 0),
            3 => rom(mem, k::COMMAND, i, 0),
            4 => rom(mem, k::MON_MAGIC1, i, 0),
            6 => rom(mem, k::MON_MAGIC2, i, 0),
            8 => rom(mem, k::A_ABILITY, i, 0),
            _ => none,
        };
        if !name {
            return (record(mem), None);
        }
        let (record, s) = match kind {
            0 => (none, crate::actor::bridge::ms_get_ram_chr_name(mem, i)),
            2 => rom_strings(mem, k::ITEM, i),
            3 => rom_strings(mem, k::COMMAND, i),
            4 => rom_strings(mem, k::MON_MAGIC1, i),
            6 => rom_strings(mem, k::MON_MAGIC2, i),
            8 => rom_strings(mem, k::A_ABILITY, i),
            5 | 7 | 0xb => (none, crate::save::bridge::ms_get_save_weapon_name(mem, id)),
            0xa => (none, crate::minor::bridge::ms_important_name(mem, i)),
            _ => return (none, None),
        };
        (record, Some(s))
    }

    /// `MsGetRandomGroupTarget(bits)` (`0x00799910`): the actors of `bits`
    /// in one of their groups (`MsGetChrGroup`), the `brnd(5) % n`-th
    /// (signed) of the `n` groups present; 0 for none.
    pub fn ms_get_random_group_target(mem: &mut impl Mem, bits: u32) -> u32 {
        use crate::actor::bridge::ms_get_chr_group;
        let mut groups = [0u8; 8];
        for i in 0..0x1f {
            if bits.cast_signed() >> i & 1 != 0 {
                let g = ms_get_chr_group(mem, i) as usize;
                groups[g] = groups[g].wrapping_add(1);
            }
        }
        let n = groups.iter().map(|&g| i32::from(g != 0)).sum::<i32>();
        if n == 0 {
            return 0;
        }
        let r = crate::rng::bridge::brnd(mem, 5).cast_signed() % n;
        let mut chosen = r;
        let mut seen = 0;
        for (g, &c) in (0i32..).zip(groups.iter()) {
            if c != 0 {
                seen += 1;
                if seen > r {
                    chosen = g;
                    break;
                }
            }
        }
        let mut out = 0;
        for i in 0..0x1f {
            if bits.cast_signed() >> i & 1 != 0 && ms_get_chr_group(mem, i).cast_signed() == chosen
            {
                out |= 1 << i;
            }
        }
        out
    }

    /// `MsGetCommandTarget(chr, id, side, event)` (`0x007996d0`): the
    /// default targets of command `id` for `chr`: the cursor's choices
    /// (`MsGetCursorTarget`) on `chr`'s own side (`side`'s low byte set)
    /// or the other (when negative, by the command's `+0x1a` bit 1: set for
    /// the other side), then by `+0x1a` bits 2-3: 0 one at random
    /// ([`ms_act_target_search`], generator 4), 1 a random group, 2 all,
    /// 3 `chr` itself.
    pub fn ms_get_command_target<M: Mem>(
        mem: &mut M,
        trig: &dyn Fn(Trig, u32) -> u32,
        chr: u32,
        id: u32,
        side: i32,
        event: u32,
    ) -> u32 {
        let _ = event;
        let record = ms_get_com_data(mem, id, 0);
        let flags = mem.read::<u16>(record.wrapping_add(0x1a));
        let own = if side < 0 {
            (!flags >> 1) & 1 != 0
        } else {
            side.to_le_bytes()[0] != 0
        };
        let monster = crate::actor::bridge::ms_get_ram_chr_monster(chr) != 0;
        // -15 the monsters, -14 the party
        let sel = (-15 + i32::from(own != monster)).cast_unsigned();
        let bits = crate::actor::bridge::ms_get_cursor_target(mem, chr, sel, record, id);
        match (flags >> 2) & 3 {
            0 => {
                if bits == 0 {
                    return 0;
                }
                let mut f = ffx_bridge::frame::Frame::new(mem, TARGET_FRAME, 4);
                let at = f.at(0);
                ms_act_target_search(&mut f, trig, chr, bits, 0x5c, 4, 0, 1, at);
                let pick: u32 = f.read(at);
                1u32.wrapping_shl(pick)
            }
            1 => ms_get_random_group_target(mem, bits),
            2 => bits,
            _ => 1u32.wrapping_shl(chr),
        }
    }
}
