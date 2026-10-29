//! Object callbacks the effect overlays repeat with their own constants
//! (round 4, the own work list's families): a particle part started on
//! the object ([`part_start`]: `*G_P = ITEM`, the program list of slot
//! 941 `(*G_D + 0x10)`, `pppStartPart(obj, 0x1000, list[entry], 1)`),
//! then twelve `mov dword [obj + d8], imm32` (the part's position,
//! colour and scale words). The entry, the fields and the values are the
//! instructions' operands, read from each DLL's code; each copy binds by
//! its own signature (the code differs only in those bytes).
//!
//! Four layouts: the list entry as a displacement (`push [eax + d8]`,
//! the stores from `+0x3d`) or entry 0 (`push [eax]`, from `+0x3c`), and
//! returning 1 (`mov eax, 1`) or what `pppStartPart`'s wrapper left in
//! `eax`. Ported from `magic_0206` 0x100018a0 / 0x10001650, `magic_0375`
//! 0x10001000 and `magic_0372` 0x10001180; 47 copies in 22 overlays.

use crate::objcb::{PartAt, part_start};
use crate::{Callee, Frame, Game, Image, Ret, Sig};

/// A `disp8` operand byte at `at`, sign-extended as the CPU does.
#[must_use]
pub fn disp8(img: &Image, at: u32) -> u32 {
    i32::from((img.word(at) as u8).cast_signed()).cast_unsigned()
}

/// The part start and the twelve stores; `entry_op` is the offset of the
/// list entry's `disp8` (`None`: `push [eax]`, entry 0), `call` the
/// offset of the wrapper's `rel32`, `stores` the offset of the first
/// `mov dword [esi + d8], imm32` (7 bytes each). Returns the wrapper's
/// `eax`.
fn part_and_stores<G: Game>(
    g: &mut G,
    img: &Image,
    fr: &Frame,
    entry_op: Option<u32>,
    call: u32,
    stores: u32,
) -> u32 {
    let f = fr.at;
    let obj = fr.arg(g, 0);
    let entry = entry_op.map_or(0, |op| disp8(img, f + op));
    let at = PartAt {
        gp: 0x04,
        item: 0x0b,
        gd: 0x10,
        table: 0x18,
        entry,
        call,
    };
    let (eax, _) = part_start(g, img, f, fr.ecx, obj, at);
    for k in 0..12 {
        // c7 46 d8 imm32
        let insn = f + stores + 7 * k;
        let field = disp8(img, insn + 2);
        g.write::<u32>(obj.wrapping_add(field), img.word(insn + 3));
    }
    eax
}

/// `(obj)`: part `list[d8 / 4]` and the stores; returns 1 (`magic_0206`
/// 0x100018a0, 153 bytes).
pub fn stores_d8_ret1<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    part_and_stores(g, img, fr, Some(0x2d), 0x36, 0x3d);
    Ret::eax(1)
}

/// `(obj)`: part `list[d8 / 4]` and the stores; returns the wrapper's
/// `eax` (`magic_0375` 0x10001000, 148 bytes).
pub fn stores_d8<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    Ret::eax(part_and_stores(g, img, fr, Some(0x2d), 0x36, 0x3d))
}

/// `(obj)`: part `list[0]` and the stores; returns 1 (`magic_0206`
/// 0x10001650, 152 bytes).
pub fn stores_0_ret1<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    part_and_stores(g, img, fr, None, 0x35, 0x3c);
    Ret::eax(1)
}

/// `(obj)`: part `list[0]` and the stores; returns the wrapper's `eax`
/// (`magic_0372` 0x10001180, 147 bytes).
pub fn stores_0<G: Game>(g: &mut G, img: &Image, fr: &Frame) -> Ret {
    Ret::eax(part_and_stores(g, img, fr, None, 0x35, 0x3c))
}

/// [`stores_d8_ret1`] in `magic_0206` (0x100018a0, 153 bytes).
pub const STORES_D8_RET1_0206_18A0: Sig = Sig {
    len: 153,
    hash: 0xf863_894d_2329_42d0,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0255` (0x10001190, 153 bytes).
pub const STORES_D8_RET1_0255_1190: Sig = Sig {
    len: 153,
    hash: 0x818f_05f4_6869_9de3,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0371` (0x10001000, 153 bytes).
pub const STORES_D8_RET1_0371_1000: Sig = Sig {
    len: 153,
    hash: 0x90e6_cf42_7a70_1a7e,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0376` (0x10001000, 153 bytes).
pub const STORES_D8_RET1_0376_1000: Sig = Sig {
    len: 153,
    hash: 0x05b4_7670_1503_3b5d,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0376` (0x100013e0, 153 bytes).
pub const STORES_D8_RET1_0376_13E0: Sig = Sig {
    len: 153,
    hash: 0x3344_6f9b_02ec_f84d,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0376` (0x10001a00, 153 bytes).
pub const STORES_D8_RET1_0376_1A00: Sig = Sig {
    len: 153,
    hash: 0xc748_d5f5_2416_34ee,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0488` (0x100012b0, 153 bytes).
pub const STORES_D8_RET1_0488_12B0: Sig = Sig {
    len: 153,
    hash: 0x8f06_e8f2_c726_16c9,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0489` (0x100018e0, 153 bytes).
pub const STORES_D8_RET1_0489_18E0: Sig = Sig {
    len: 153,
    hash: 0x5c95_9c9a_8b73_63e7,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0496` (0x10001270, 153 bytes).
pub const STORES_D8_RET1_0496_1270: Sig = Sig {
    len: 153,
    hash: 0x5fe6_f18d_b0af_032e,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0664` (0x10001250, 153 bytes).
pub const STORES_D8_RET1_0664_1250: Sig = Sig {
    len: 153,
    hash: 0x0340_15ec_d4af_c775,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0664` (0x10001bc0, 153 bytes).
pub const STORES_D8_RET1_0664_1BC0: Sig = Sig {
    len: 153,
    hash: 0xbe4d_15b6_6e34_cc84,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0664` (0x10001d80, 153 bytes).
pub const STORES_D8_RET1_0664_1D80: Sig = Sig {
    len: 153,
    hash: 0xdba1_ad66_3db3_8e08,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0665` (0x10001250, 153 bytes).
pub const STORES_D8_RET1_0665_1250: Sig = Sig {
    len: 153,
    hash: 0xc1b2_496b_48a2_8de7,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0665` (0x100013b0, 153 bytes).
pub const STORES_D8_RET1_0665_13B0: Sig = Sig {
    len: 153,
    hash: 0x6035_b079_286c_f545,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0665` (0x100018d0, 153 bytes).
pub const STORES_D8_RET1_0665_18D0: Sig = Sig {
    len: 153,
    hash: 0xb1b6_1818_79b5_de7a,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0665` (0x10001a70, 153 bytes).
pub const STORES_D8_RET1_0665_1A70: Sig = Sig {
    len: 153,
    hash: 0xca17_f582_627e_c44f,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0666` (0x10001670, 153 bytes).
pub const STORES_D8_RET1_0666_1670: Sig = Sig {
    len: 153,
    hash: 0x6289_9ad8_5cb8_6f64,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0666` (0x10001860, 153 bytes).
pub const STORES_D8_RET1_0666_1860: Sig = Sig {
    len: 153,
    hash: 0xb4f3_bf31_7244_32cd,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0666` (0x10002500, 153 bytes).
pub const STORES_D8_RET1_0666_2500: Sig = Sig {
    len: 153,
    hash: 0x471c_f50a_234c_c256,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0666` (0x100026d0, 153 bytes).
pub const STORES_D8_RET1_0666_26D0: Sig = Sig {
    len: 153,
    hash: 0x2371_b01a_48bd_6e71,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0669` (0x10001810, 153 bytes).
pub const STORES_D8_RET1_0669_1810: Sig = Sig {
    len: 153,
    hash: 0xeaaf_0230_f59b_d9fc,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8_ret1`] in `magic_0669` (0x100019f0, 153 bytes).
pub const STORES_D8_RET1_0669_19F0: Sig = Sig {
    len: 153,
    hash: 0x729f_a101_f093_13e8,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8`] in `magic_0375` (0x10001000, 148 bytes).
pub const STORES_D8_0375_1000: Sig = Sig {
    len: 148,
    hash: 0x56bf_b39e_612d_6750,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8`] in `magic_0375` (0x100017d0, 148 bytes).
pub const STORES_D8_0375_17D0: Sig = Sig {
    len: 148,
    hash: 0xa32e_5378_c6e1_70a8,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8`] in `magic_0375` (0x10001900, 148 bytes).
pub const STORES_D8_0375_1900: Sig = Sig {
    len: 148,
    hash: 0x9fb3_4768_1db5_c7f2,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8`] in `magic_0375` (0x10001a80, 148 bytes).
pub const STORES_D8_0375_1A80: Sig = Sig {
    len: 148,
    hash: 0x5164_fbc9_6643_ee18,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8`] in `magic_0375` (0x10001b20, 148 bytes).
pub const STORES_D8_0375_1B20: Sig = Sig {
    len: 148,
    hash: 0xb071_b538_c825_9618,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8`] in `magic_0375` (0x10001dd0, 148 bytes).
pub const STORES_D8_0375_1DD0: Sig = Sig {
    len: 148,
    hash: 0x2621_81b2_be4f_9363,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8`] in `magic_0375` (0x10001e70, 148 bytes).
pub const STORES_D8_0375_1E70: Sig = Sig {
    len: 148,
    hash: 0x5819_f5f3_8ddc_6e39,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8`] in `magic_0375` (0x10001f10, 148 bytes).
pub const STORES_D8_0375_1F10: Sig = Sig {
    len: 148,
    hash: 0x71c6_8974_bfb2_50b1,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8`] in `magic_0376` (0x10001ff0, 148 bytes).
pub const STORES_D8_0376_1FF0: Sig = Sig {
    len: 148,
    hash: 0xdead_9721_8a35_e9b9,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8`] in `magic_0376` (0x100021f0, 148 bytes).
pub const STORES_D8_0376_21F0: Sig = Sig {
    len: 148,
    hash: 0xb605_70a6_d12b_6826,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8`] in `magic_0487` (0x10001000, 148 bytes).
pub const STORES_D8_0487_1000: Sig = Sig {
    len: 148,
    hash: 0x7e63_15f2_268e_fdfd,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_d8`] in `magic_0498` (0x10001000, 148 bytes).
pub const STORES_D8_0498_1000: Sig = Sig {
    len: 148,
    hash: 0x8093_dcac_e8b1_ae77,
    calls: &[(0x36, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0_ret1`] in `magic_0206` (0x10001650, 152 bytes).
pub const STORES_0_RET1_0206_1650: Sig = Sig {
    len: 152,
    hash: 0x9f9e_88e9_5568_e70f,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0_ret1`] in `magic_0255` (0x10001000, 152 bytes).
pub const STORES_0_RET1_0255_1000: Sig = Sig {
    len: 152,
    hash: 0xe304_3578_a6db_e929,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0_ret1`] in `magic_0376` (0x10002290, 152 bytes).
pub const STORES_0_RET1_0376_2290: Sig = Sig {
    len: 152,
    hash: 0xadd1_5f17_acf6_9428,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0_ret1`] in `magic_0497` (0x10001190, 152 bytes).
pub const STORES_0_RET1_0497_1190: Sig = Sig {
    len: 152,
    hash: 0x3f23_67dc_c89f_9391,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0_ret1`] in `magic_0658` (0x10001000, 152 bytes).
pub const STORES_0_RET1_0658_1000: Sig = Sig {
    len: 152,
    hash: 0x10bc_37ff_f8da_9c34,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0_ret1`] in `magic_0669` (0x10001b90, 152 bytes).
pub const STORES_0_RET1_0669_1B90: Sig = Sig {
    len: 152,
    hash: 0x65f2_7a57_5c04_64df,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0`] in `magic_0372` (0x10001180, 147 bytes).
pub const STORES_0_0372_1180: Sig = Sig {
    len: 147,
    hash: 0xe8b8_ba1b_9b30_304d,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0`] in `magic_0498` (0x10001160, 147 bytes).
pub const STORES_0_0498_1160: Sig = Sig {
    len: 147,
    hash: 0xb062_b573_85d7_a5ca,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0`] in `magic_0536` (0x10001200, 147 bytes).
pub const STORES_0_0536_1200: Sig = Sig {
    len: 147,
    hash: 0x454c_5b49_8bd6_2c73,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0`] in `magic_0539` (0x10001000, 147 bytes).
pub const STORES_0_0539_1000: Sig = Sig {
    len: 147,
    hash: 0x27ae_9ad9_e664_d5dd,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0`] in `magic_0561` (0x10001000, 147 bytes).
pub const STORES_0_0561_1000: Sig = Sig {
    len: 147,
    hash: 0x329c_f21b_65b3_0970,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0`] in `magic_0616` (0x10001000, 147 bytes).
pub const STORES_0_0616_1000: Sig = Sig {
    len: 147,
    hash: 0x2bce_4e00_77a4_1ba2,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};

/// [`stores_0`] in `magic_0617` (0x10001000, 147 bytes).
pub const STORES_0_0617_1000: Sig = Sig {
    len: 147,
    hash: 0x74a2_edb4_5b40_59e6,
    calls: &[(0x35, Callee::Sig(&crate::opu_parts::START_PART))],
    imports: &[],
};
