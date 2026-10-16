//! One-instruction functions the compiler kept: constant and global
//! getters, argument pass-throughs, and flag or vtable stores. Most are
//! reached through vtables or a single caller; the tables below list them
//! by address so the hook and the parity test share one source.

/// Getters returning a plain number: `(function, value)`.
pub const CONSTANTS: [(u32, u32); 7] = [
    (0x0040_5ba0, 0x80),      // 2 callers in the model buffer code
    (0x0059_e5c0, 4),         // vtable only
    (0x0064_7cb0, 0x6900),    // the save data size (caller 0x008b5450)
    (0x006f_1580, 0x98),      // vtable only
    (0x006f_1590, 0x18),      // vtable only
    (0x006f_15a0, 0x10),      // vtable only
    (0x008b_7cc0, 0x64_0064), // two 16-bit 100s (caller 0x00857ea0)
];

/// Getters returning the address of something in the image: `(function,
/// address)` (rebased like any global).
pub const ADDRESSES: [(u32, u32); 6] = [
    (0x0040_13a0, 0x00b7_c79c),
    (0x0040_1480, 0x00b7_c7b4),
    (0x0040_1530, 0x00b7_c7d8),
    (0x0067_9a00, 0x00b4_2a34), // caller 0x006c0560
    (0x0077_7230, 0x00c4_1748), // caller 0x0076cf80
    (0x008b_ec70, 0x023c_c480), // caller 0x00774c00
];

/// Getters returning a `u32` global: `(function, global)`.
pub const GLOBALS: [(u32, u32); 10] = [
    (0x0064_7c90, 0x0132_5ac4), // caller Sg_MainLoop
    (0x0077_4670, 0x0112_a010),
    (0x008a_cf20, 0x0184_1d30),
    (0x008b_9130, 0x025d_16c4),
    (0x008b_e320, 0x00c5_a084),
    (0x0090_6730, 0x0187_63e0),
    (0x009d_a2c0, 0x0194_0ab4),
    (0x009d_a3a0, 0x00c6_4ca4),
    (0x00a4_2fa0, 0x01a8_5ba8), // g_FullArenaLocalization
    (0x00a7_ae90, 0x023c_beb0),
];

/// `ret 0xc` methods returning -1 (vtable only).
pub const MINUS_ONE_3: [u32; 6] = [
    0x0061_a110,
    0x0067_65d0,
    0x0067_7080,
    0x0067_7c30,
    0x0067_8b50,
    0x00a4_2490,
];

/// Cdecl pass-throughs of the first argument (frame-pointer builds of
/// `return x;`, vtable only or inlined elsewhere).
pub const FIRST: [u32; 32] = [
    0x0060_ab50,
    0x0060_ab60,
    0x0060_b850,
    0x0060_b860,
    0x0060_c010,
    0x0060_c020,
    0x0060_c700,
    0x0060_c710,
    0x0060_c720,
    0x0060_c730,
    0x0060_c870,
    0x0060_c880,
    0x0061_e720,
    0x0061_e730,
    0x0063_32d0,
    0x0063_32e0,
    0x0063_32f0,
    0x0063_3300,
    0x0063_3310,
    0x0063_3320,
    0x0063_3330,
    0x0068_6cb0,
    0x0068_6f80,
    0x0069_a3d0,
    0x0069_bda0,
    0x006f_4870,
    0x006f_4880,
    0x008e_d020,
    0x008e_d030,
    0x008e_d050, // jumps to Identity_008ed040
    0x00a4_1900,
    0x00a4_1910,
];

/// The same with `ret 4` (stdcall).
pub const FIRST_STDCALL: [u32; 8] = [
    0x0068_6c50,
    0x0069_a350,
    0x0069_a360,
    0x0069_a390,
    0x00a4_1850,
    0x00a4_1860,
    0x00a4_1870,
    0x00a4_18f0,
];

/// The value in a `(function, value)` table.
#[must_use]
pub const fn lookup(table: &[(u32, u32)], function: u32) -> u32 {
    let mut i = 0;
    while i < table.len() {
        if table[i].0 == function {
            return table[i].1;
        }
        i += 1;
    }
    panic!("not in the table")
}

/// `0x00401000`: the second argument.
#[must_use]
pub const fn second(_first: u32, second: u32) -> u32 {
    second
}

/// `0x0060ab40` and `0x0061e710`: the first argument's low byte or half
/// (only `al`/`ax` written, the rest of `eax` is the caller's).
#[must_use]
pub const fn first_u8(x: u8) -> u8 {
    x
}

/// See [`first_u8`].
#[must_use]
pub const fn first_u16(x: u16) -> u16 {
    x
}

/// `0x0060ab70`: the first argument zero-extended to 64 bits.
#[must_use]
pub const fn zero_extend(x: u32) -> u64 {
    x as u64
}

/// `0x0060ab80` and `0x0061d110`: `a + b`.
#[must_use]
pub const fn add(a: u32, b: u32) -> u32 {
    a.wrapping_add(b)
}

/// `0x0061e6e0`: `b - a`.
#[must_use]
pub const fn difference(a: u32, b: u32) -> u32 {
    b.wrapping_sub(a)
}

/// `0x006770b0` (`ret 4`): 0x18.
pub const SIZE_0X18: u32 = 0x18;

/// The originals over game memory (`ffx_bridge::Mem`).
pub mod bridge {
    use ffx_bridge::{Global, Mem};

    /// A getter of [`super::ADDRESSES`]: the rebased address.
    pub fn address(mem: &impl Mem, function: u32) -> u32 {
        mem.rebase(super::lookup(&super::ADDRESSES, function))
    }

    /// A getter of [`super::GLOBALS`]: the global's value.
    pub fn global(mem: &impl Mem, function: u32) -> u32 {
        Global::<u32>::new(super::lookup(&super::GLOBALS, function)).get(mem)
    }

    /// `Phyre::PInputs::PInputSource` vftable (`0x00b70938`), stored by
    /// `0x009e24f0` (`thiscall`).
    pub const PINPUT_SOURCE_VFTABLE: u32 = 0x00b7_0938;
    /// `Phyre::PWorldMatrixController` vftable (`0x00b3f674`), stored by
    /// `0x0061a730` (`thiscall`).
    pub const PWORLD_MATRIX_CONTROLLER_VFTABLE: u32 = 0x00b3_f674;

    /// The base-class part of a constructor: `this`'s vftable set
    /// (`eax` left alone).
    pub fn set_vftable(mem: &mut impl Mem, this: u32, vftable: u32) {
        let v = mem.rebase(vftable);
        mem.write(this, v);
    }

    /// `0x006f1db0` (`thiscall`): `this[0] = 1`.
    pub fn set_word0(mem: &mut impl Mem, this: u32) {
        mem.write(this, 1u32);
    }

    /// `0x006f1dc0` (`thiscall`): byte `+0x364` of `this` set.
    pub fn set_byte364(mem: &mut impl Mem, this: u32) {
        mem.write(this.wrapping_add(0x364), 1u8);
    }

    /// `0x006f20d0`, `0x006f2150`, `0x006f21a0` (`thiscall`): whether the
    /// save-data manager's state (`+8`) is `state` (6 deleting, 4 loading,
    /// 2 saving finished, by their callers).
    pub fn state_is(mem: &impl Mem, this: u32, state: u32) -> u32 {
        u32::from(mem.read::<u32>(this.wrapping_add(8)) == state)
    }

    /// `0x0061acb0` (`thiscall`): the 64-bit value at `+8` of the record
    /// `this + 0xc` points at (`PStreamFile::getFileSize`'s entry size).
    pub fn entry_size64(mem: &impl Mem, this: u32) -> u64 {
        let e: u32 = mem.read(this.wrapping_add(0xc));
        u64::from(mem.read::<u32>(e.wrapping_add(8)))
            | u64::from(mem.read::<u32>(e.wrapping_add(0xc))) << 32
    }

    /// `0x00631310` (`thiscall`): `this + 0x8c`.
    #[must_use]
    pub const fn stick_info(this: u32) -> u32 {
        this.wrapping_add(0x8c)
    }

    /// `0x008e2d50`: word `+0x64` of `g_TkMenuWorldMapWindow`'s window.
    pub fn world_map_window_word64(mem: &impl Mem) -> u32 {
        let w = Global::<u32>::new(0x0187_1698).get(mem);
        mem.read(w.wrapping_add(0x64))
    }
}
