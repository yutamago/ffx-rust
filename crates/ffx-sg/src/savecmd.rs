//! Kernel menu accessors in the function tables from `0x00c410c8`, each
//! reached only through its table slot:
//!
//! - `KernelSaveCommand_<cmd>_<addr>`: `push cmd; push chr; call
//!   MsGetSaveCommand`, its `eax` returned;
//! - `KernelSlotAbilityName_<off>_<addr>`, `KernelSlotItemName_..`: the
//!   `u16` at `off` of record `i` (0x1d8 bytes) of `*0x0112a034` written
//!   over the argument, then a tail jump to `Kernel_GetAbilityName` or
//!   `Kernel_GetItemName`.
//!
//! The system module's copies of both shapes (`0x00771c30..`) go through
//! the same bridges in `ffx_system::tolib`.

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::Mem;

    /// Each accessor's address and the command it asks for.
    pub const SAVE_COMMANDS: [(u32, u32); 78] = [
        (0x0076_f180, 0x3056),
        (0x0076_f1d0, 0x3058),
        (0x0076_f220, 0x3059),
        (0x0076_f240, 0x305a),
        (0x0076_f290, 0x305b),
        (0x0076_f2e0, 0x305c),
        (0x0076_f330, 0x305d),
        (0x0076_f370, 0x305e),
        (0x0076_f450, 0x305f),
        (0x0077_0320, 0x3060),
        (0x0077_0950, 0x3061),
        (0x0077_0970, 0x3062),
        (0x0077_09b0, 0x3063),
        (0x0077_09f0, 0x3064),
        (0x0077_0a40, 0x3065),
        (0x0077_0a90, 0x3066),
        (0x0077_0ae0, 0x3067),
        (0x0077_0b30, 0x3068),
        (0x0077_0b80, 0x3069),
        (0x0077_0ba0, 0x306a),
        (0x0077_0bf0, 0x306b),
        (0x0077_0c40, 0x306c),
        (0x0077_0c90, 0x306d),
        (0x0077_0ce0, 0x306e),
        (0x0077_0d30, 0x306f),
        (0x0077_0d50, 0x3070),
        (0x0077_0da0, 0x3071),
        (0x0077_0df0, 0x3072),
        (0x0077_0e40, 0x3073),
        (0x0077_0e90, 0x3074),
        (0x0077_0ee0, 0x3075),
        (0x0077_0f00, 0x3076),
        (0x0077_0f50, 0x3077),
        (0x0077_0fa0, 0x308b),
        (0x0077_0ff0, 0x308c),
        (0x0077_1040, 0x308d),
        (0x0077_1090, 0x308e),
        (0x0077_10b0, 0x308f),
        (0x0077_1100, 0x3090),
        (0x0077_1150, 0x3091),
        (0x0077_11a0, 0x3092),
        (0x0077_11f0, 0x3093),
        (0x0077_1240, 0x3094),
        (0x0077_1260, 0x3095),
        (0x0077_12b0, 0x3096),
        (0x0077_1300, 0x3097),
        (0x0077_1350, 0x3098),
        (0x0077_13a0, 0x3099),
        (0x0077_13f0, 0x309a),
        (0x0077_1410, 0x309b),
        (0x0077_1460, 0x309c),
        (0x0077_14b0, 0x309d),
        (0x0077_1500, 0x309e),
        (0x0077_1550, 0x309f),
        (0x0077_15a0, 0x30a0),
        (0x0077_15c0, 0x30a1),
        (0x0077_1610, 0x30a2),
        (0x0077_1660, 0x30a3),
        (0x0077_16b0, 0x30a4),
        (0x0077_1700, 0x30a5),
        (0x0077_1750, 0x30a6),
        (0x0077_1770, 0x30a7),
        (0x0077_17c0, 0x30a8),
        (0x0077_1810, 0x30a9),
        (0x0077_1860, 0x30aa),
        (0x0077_18b0, 0x30ab),
        (0x0077_1900, 0x30ac),
        (0x0077_1920, 0x30ad),
        (0x0077_1970, 0x30ae),
        (0x0077_19c0, 0x30af),
        (0x0077_1a10, 0x30b0),
        (0x0077_1a60, 0x30b1),
        (0x0077_1ab0, 0x30b2),
        (0x0077_1ad0, 0x30b3),
        (0x0077_1b20, 0x30b4),
        (0x0077_1b70, 0x30b5),
        (0x0077_1bc0, 0x30b6),
        (0x0077_1c10, 0x30b7),
    ];

    /// The command the accessor at `at` asks for.
    #[must_use]
    pub fn command(at: u32) -> Option<u32> {
        SAVE_COMMANDS
            .iter()
            .find(|&&(addr, _)| addr == at)
            .map(|&(_, cmd)| cmd)
    }

    /// The accessor at `at` (one of [`SAVE_COMMANDS`]) for character
    /// `chr`: `MsGetSaveCommand(chr, cmd)` through `call`; 0 for an
    /// address that is none of them.
    pub fn kernel_save_command<M: Mem>(
        mem: &mut M,
        at: u32,
        chr: u32,
        call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        command(at).map_or(0, |cmd| {
            ffx_system::tolib::bridge::kernel_save_command(mem, chr, cmd, call)
        })
    }

    /// `Kernel_GetAbilityName(id)`.
    pub const ABILITY_NAME: u32 = 0x0077_4720;
    /// `Kernel_GetItemName(id)`.
    pub const ITEM_NAME: u32 = 0x0077_67a0;

    /// The slot name accessors: address, offset of the `u16` id in a
    /// 0x1d8-byte record of `*0x0112a034`, name function.
    pub const SLOT_NAMES: [(u32, u32, u32); 60] = [
        (0x0076_f1a0, 0x09a, ITEM_NAME),
        (0x0076_f1f0, 0x09c, ITEM_NAME),
        (0x0076_f260, 0x09e, ITEM_NAME),
        (0x0076_f2b0, 0x0a4, ITEM_NAME),
        (0x0076_f300, 0x0a6, ITEM_NAME),
        (0x0077_0920, 0x0aa, ABILITY_NAME),
        (0x0077_0a10, 0x0b2, ABILITY_NAME),
        (0x0077_0a60, 0x0b4, ABILITY_NAME),
        (0x0077_0ab0, 0x0b6, ABILITY_NAME),
        (0x0077_0b00, 0x0b8, ABILITY_NAME),
        (0x0077_0b50, 0x0ba, ABILITY_NAME),
        (0x0077_0bc0, 0x0bc, ABILITY_NAME),
        (0x0077_0c10, 0x0be, ABILITY_NAME),
        (0x0077_0c60, 0x0c0, ABILITY_NAME),
        (0x0077_0cb0, 0x0c2, ABILITY_NAME),
        (0x0077_0d00, 0x0c4, ABILITY_NAME),
        (0x0077_0d70, 0x0c6, ABILITY_NAME),
        (0x0077_0dc0, 0x0c8, ABILITY_NAME),
        (0x0077_0e10, 0x0ca, ABILITY_NAME),
        (0x0077_0e60, 0x0cc, ABILITY_NAME),
        (0x0077_0eb0, 0x0ce, ABILITY_NAME),
        (0x0077_0f20, 0x0d0, ABILITY_NAME),
        (0x0077_0f70, 0x0d2, ABILITY_NAME),
        (0x0077_0fc0, 0x0d4, ABILITY_NAME),
        (0x0077_1010, 0x0d6, ABILITY_NAME),
        (0x0077_1060, 0x0d8, ABILITY_NAME),
        (0x0077_10d0, 0x0da, ABILITY_NAME),
        (0x0077_1120, 0x0dc, ABILITY_NAME),
        (0x0077_1170, 0x0de, ABILITY_NAME),
        (0x0077_11c0, 0x0e0, ABILITY_NAME),
        (0x0077_1210, 0x0e2, ABILITY_NAME),
        (0x0077_1280, 0x0e4, ABILITY_NAME),
        (0x0077_12d0, 0x0e6, ABILITY_NAME),
        (0x0077_1320, 0x0e8, ABILITY_NAME),
        (0x0077_1370, 0x0ea, ABILITY_NAME),
        (0x0077_13c0, 0x0ec, ABILITY_NAME),
        (0x0077_1430, 0x0ee, ABILITY_NAME),
        (0x0077_1480, 0x0f0, ABILITY_NAME),
        (0x0077_14d0, 0x0f2, ABILITY_NAME),
        (0x0077_1520, 0x0f4, ABILITY_NAME),
        (0x0077_1570, 0x0f6, ABILITY_NAME),
        (0x0077_15e0, 0x0f8, ABILITY_NAME),
        (0x0077_1630, 0x0fa, ABILITY_NAME),
        (0x0077_1680, 0x0fc, ABILITY_NAME),
        (0x0077_16d0, 0x0fe, ABILITY_NAME),
        (0x0077_1720, 0x100, ABILITY_NAME),
        (0x0077_1790, 0x102, ABILITY_NAME),
        (0x0077_17e0, 0x104, ABILITY_NAME),
        (0x0077_1830, 0x106, ABILITY_NAME),
        (0x0077_1880, 0x108, ABILITY_NAME),
        (0x0077_18d0, 0x10a, ABILITY_NAME),
        (0x0077_1940, 0x10c, ABILITY_NAME),
        (0x0077_1990, 0x10e, ABILITY_NAME),
        (0x0077_19e0, 0x110, ABILITY_NAME),
        (0x0077_1a30, 0x112, ABILITY_NAME),
        (0x0077_1a80, 0x114, ABILITY_NAME),
        (0x0077_1af0, 0x116, ABILITY_NAME),
        (0x0077_1b40, 0x118, ABILITY_NAME),
        (0x0077_1b90, 0x11a, ABILITY_NAME),
        (0x0077_1be0, 0x11c, ABILITY_NAME),
    ];

    /// The accessor at `at` (one of [`SLOT_NAMES`]) for record `i`: the
    /// name function of the record's id (a tail jump in the original),
    /// through `call`; 0 for an address that is none of them.
    pub fn kernel_slot_name<M: Mem>(
        mem: &mut M,
        at: u32,
        i: u32,
        call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        SLOT_NAMES
            .iter()
            .find(|&&(addr, _, _)| addr == at)
            .map_or(0, |&(_, off, f)| {
                ffx_system::tolib::bridge::kernel_slot_name(mem, i, off, f, call)
            })
    }
}
