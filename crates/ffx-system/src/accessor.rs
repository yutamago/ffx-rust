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

/// Cdecl `b - a` (frame-pointer builds of pointer differences; vtable only).
pub const SUB2: [u32; 33] = [
    0x0061_e6f0,
    0x0061_e700,
    0x0067_5c70,
    0x0067_6960,
    0x0067_72d0,
    0x0067_8010,
    0x009d_db30,
    0x009d_db40,
    0x009d_db50,
    0x009d_db60,
    0x009d_db70,
    0x009d_db80,
    0x009d_db90,
    0x009d_dba0,
    0x009d_dbb0,
    0x009d_dbc0,
    0x009d_dbd0,
    0x009d_dbe0,
    0x009d_dbf0,
    0x009d_dc00,
    0x009d_dc10,
    0x009d_dc20,
    0x009d_dc30,
    0x009d_dc40,
    0x009d_dc50,
    0x009d_dc60,
    0x009d_dc70,
    0x009d_dc80,
    0x009d_dc90,
    0x009d_dca0,
    0x009d_dcb0,
    0x009d_dcc0,
    0x009d_dcd0,
];

/// Cdecl `a + b`.
pub const ADD2: [u32; 10] = [
    0x0061_e740,
    0x0063_31a0,
    0x0069_c4b0,
    0x0069_c4d0,
    0x006e_45c0,
    0x006f_5370,
    0x006f_5a00,
    0x0070_2fd0,
    0x009d_dce0,
    0x009d_dcf0,
];

/// Cdecl `a + (b as u16)`.
pub const ADD_U16: [u32; 6] = [
    0x0063_31b0,
    0x0069_c4c0,
    0x006e_45d0,
    0x006f_5380,
    0x006f_5750,
    0x0070_2fe0,
];

/// Cdecl `a + (b as u8)`.
pub const ADD_U8: [u32; 3] = [0x0060_ab90, 0x0061_d120, 0x0069_c4e0];

/// `xor eax, eax; ret 4`: one stack argument, 0.
pub const ZERO_RET4: [u32; 5] = [
    0x0067_65c0,
    0x0067_7060,
    0x0067_7c20,
    0x0067_8b40,
    0x00a4_2360,
];

/// `thiscall(this, v, _)`, `ret 8`: `*this = v`, returns `this`.
pub const STORE0_RET_THIS: [u32; 9] = [
    0x0063_38e0,
    0x0063_38f0,
    0x0063_3900,
    0x0063_3910,
    0x0063_3920,
    0x0063_3930,
    0x0063_3940,
    0x0063_3950,
    0x0063_3960,
];

/// `thiscall(this, p)`, `ret 4`: `*p = *this`, returns `p`.
pub const COPY_THIS_TO_P: [u32; 4] = [0x0068_6ce0, 0x0068_6e80, 0x0069_af30, 0x006f_3fe0];

/// `thiscall(this, p)`, `ret 4`: `*p = this[1]`, returns `p`.
pub const COPY_THIS4_TO_P: [u32; 2] = [0x006f_4090, 0x006f_40a0];

/// `thiscall(this, v)`, `ret 4`: `this[2] = v`, returns `v`.
pub const STORE8_RET_V: [u32; 2] = [0x0064_7bb0, 0x006f_c340];

/// `thiscall(this, i)`, `ret 4`: `this[2] + 4 i`.
pub const INDEX4_AT8: [u32; 4] = [0x0069_0bd0, 0x0069_0be0, 0x0069_0bf0, 0x0069_0c00];

/// `thiscall(this, i)`, `ret 4`: `this[1] + 4 i`.
pub const INDEX4_AT4: [u32; 3] = [0x009e_2cb0, 0x009e_2cc0, 0x009e_2cd0];

/// `thiscall(this, i)`, `ret 4`: `this[2] + 8 i`.
pub const INDEX8_AT8: [u32; 2] = [0x0063_1170, 0x0063_1180];

/// Cdecl word-global setters `(function, global)`: store `v`, return it.
pub const WORD_SETTERS2: [(u32, u32); 12] = [
    (0x0061_9e70, 0x00cc_9a38),
    (0x0064_8680, 0x00cc_b9a0),
    (0x0069_1b50, 0x00cc_c890),
    (0x0069_1b60, 0x00cc_c894),
    (0x0076_65a0, 0x0230_5844),
    (0x0076_65b0, 0x0230_5840),
    (0x0077_5810, 0x0112_a018),
    (0x0089_bc50, 0x00c5_8ef8),
    (0x008a_b570, 0x00c5_9f38),
    (0x008b_e9b0, 0x00c5_a084),
    (0x009d_a790, 0x00c6_4ca4),
    (0x009d_afd0, 0x0193_fdac),
];

/// Cdecl byte-global setters `(function, global)`.
pub const BYTE_SETTERS: [(u32, u32); 4] = [
    (0x0082_1a20, 0x012f_b8fb),
    (0x0082_1a30, 0x012f_b8fc),
    (0x0089_a850, 0x0133_d782),
    (0x0089_bc60, 0x00c5_8ef4),
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

    /// A word global's setter: stores `v`, returns it (`0x0076cfc0`
    /// `0x01128ac4`, `0x008af690` `0x01841e0c`, `0x00a43210` `0x01a85ba4`).
    pub const WORD_SETTERS: [(u32, u32); 3] = [
        (0x0076_cfc0, 0x0112_8ac4),
        (0x008a_f690, 0x0184_1e0c),
        (0x00a4_3210, 0x01a8_5ba4),
    ];

    /// A setter of [`WORD_SETTERS`].
    pub fn set_word_global(mem: &mut impl Mem, function: u32, v: u32) -> u32 {
        Global::<u32>::new(super::lookup(&WORD_SETTERS, function)).set(mem, v);
        v
    }

    /// `0x0076dc60(v)`: the byte global `0x01128ac0` set.
    pub fn set_byte_01128ac0(mem: &mut impl Mem, v: u8) {
        Global::<u8>::new(0x0112_8ac0).set(mem, v);
    }

    /// `0x006f24d0()`: word `+8` of `*0x00ce81f4` cleared; returns the object.
    pub fn clear_word8_of_00ce81f4(mem: &mut impl Mem) -> u32 {
        let o = Global::<u32>::new(0x00ce_81f4).get(mem);
        mem.write(o.wrapping_add(8), 0u32);
        o
    }

    /// `thiscall(this, v)`, `ret 4`: word `off` of `this` set, `v` returned
    /// (`0x006f24f0` `+0xc`, `0x006f2520` `+0x394`).
    pub fn set_member(mem: &mut impl Mem, this: u32, off: u32, v: u32) -> u32 {
        mem.write(this.wrapping_add(off), v);
        v
    }

    /// `0x008729c0()`: on `*0x01326b28`, byte 0 gets bit 7 and word `+0x210`
    /// bit 1; returns the object.
    pub fn flag_01326b28(mem: &mut impl Mem) -> u32 {
        let o = Global::<u32>::new(0x0132_6b28).get(mem);
        let b: u8 = mem.read(o);
        mem.write(o, b | 0x80);
        let w: u32 = mem.read(o.wrapping_add(0x210));
        mem.write(o.wrapping_add(0x210), w | 2);
        o
    }

    /// `0x008729e0()`: 1 when `0x01325b88` is set and the low two bits of
    /// byte 3 of `*0x01326b28` are 2.
    pub fn check_01326b28(mem: &impl Mem) -> u32 {
        if Global::<u32>::new(0x0132_5b88).get(mem) == 0 {
            return 0;
        }
        let o = Global::<u32>::new(0x0132_6b28).get(mem);
        u32::from(mem.read::<u8>(o.wrapping_add(3)) & 3 == 2)
    }

    /// `0x008e2dd0(v)`: word `+0x68` of `g_TkMenuWorldMapWindow`'s window
    /// set; returns the window.
    pub fn set_world_map_window_word68(mem: &mut impl Mem, v: u32) -> u32 {
        let w = Global::<u32>::new(0x0187_1698).get(mem);
        mem.write(w.wrapping_add(0x68), v);
        w
    }

    /// `0x006f2120()` (`thiscall`): byte `+0x364` returned and cleared.
    pub fn take_byte364(mem: &mut impl Mem, this: u32) -> u8 {
        let b: u8 = mem.read(this.wrapping_add(0x364));
        if b != 0 {
            mem.write(this.wrapping_add(0x364), 0u8);
        }
        b
    }

    /// `0x006f2430(i)` (`thiscall`, `ret 4`): byte `+0x3bc + i` set;
    /// returns `i`.
    pub fn set_flag_3bc(mem: &mut impl Mem, this: u32, i: u32) -> u32 {
        mem.write(this.wrapping_add(0x3bc).wrapping_add(i), 1u8);
        i
    }

    /// `0x006e4fd0()` (`thiscall`): `+0x10` 0, the floats `+4`, `+8`, `+0x14`
    /// 0.0.
    pub fn reset_e4fd0(mem: &mut impl Mem, this: u32) {
        mem.write(this.wrapping_add(0x10), 0u32);
        for off in [4, 8, 0x14] {
            mem.write(this.wrapping_add(off), 0.0f32);
        }
    }

    /// `thiscall(this, src)`, `ret 4`: `n` floats from `src` to `this + at`
    /// through the x87 stack (a signalling NaN becomes quiet); returns
    /// `src` (`0x007037e0`: 2 to `+0x14`; `0x00703750`: 4 to `+4`).
    pub fn copy_floats(mem: &mut impl Mem, this: u32, src: u32, at: u32, n: u32) -> u32 {
        for k in 0..n {
            let v: f32 = mem.read(src.wrapping_add(4 * k));
            mem.write(
                this.wrapping_add(at + 4 * k),
                ffx_bridge::x87::round_trip(v),
            );
        }
        src
    }

    /// `0x0076cfa0(n)`: the byte `0x00c40e19` is 1 unless `n` is 1.
    pub fn set_c40e19(mem: &mut impl Mem, n: u32) {
        Global::<u8>::new(0x00c4_0e19).set(mem, u8::from(n.wrapping_sub(1) != 0));
    }

    /// `0x0061acf0()` (`thiscall`): `+0xc`, `+0x10`, `+0x14` and byte `+0`
    /// zeroed (a `VFile` released).
    pub fn vfile_reset(mem: &mut impl Mem, this: u32) {
        mem.write(this.wrapping_add(0xc), [0u32; 3]);
        mem.write(this, 0u8);
    }

    /// `0x006dbf30(i)` (`thiscall`, `ret 4`): whether word `+0x1f8` of the
    /// 0x1e8-byte record `i` is 8.
    pub fn record_state_is8(mem: &impl Mem, this: u32, i: u32) -> u32 {
        let at = this.wrapping_add(i.wrapping_mul(0x1e8)).wrapping_add(0x1f8);
        u32::from(mem.read::<u32>(at) == 8)
    }

    /// `0x00778960(i)`: word 1 of the 12-byte HDD head `i` (`0x0112a52c`)
    /// over 2048, rounded toward zero.
    pub fn hdd_head_sectors(mem: &impl Mem, i: u32) -> i32 {
        let v: i32 = mem.read(mem.rebase(0x0112_a52c).wrapping_add(i.wrapping_mul(12)));
        v / 2048
    }

    /// `std::_Tree::_Min(node)` (`0x00a419b0`) and `_Max` (`0x00a41990`):
    /// follows the left (`+0`) or right (`+8`) links while the next node is
    /// not nil (`+0xd`).
    pub fn tree_extreme(mem: &impl Mem, node: u32, link: u32) -> u32 {
        let mut n = node;
        loop {
            let next: u32 = mem.read(n.wrapping_add(link));
            if mem.read::<u8>(next.wrapping_add(0xd)) != 0 {
                return n;
            }
            n = next;
        }
    }

    /// `0x0067aa50(x)`: entry `x - *0x00c33468` of the seven words at
    /// `0x00c334d4`, 0 outside them.
    pub fn table_00c334d4(mem: &impl Mem, x: u32) -> u32 {
        let k = x
            .wrapping_sub(Global::<u32>::new(0x00c3_3468).get(mem))
            .cast_signed();
        if !(0..7).contains(&k) {
            return 0;
        }
        mem.read(
            Global::<u32>::new(0x00c3_34d4)
                .ptr(mem)
                .addr
                .wrapping_add(k.cast_unsigned() * 4),
        )
    }

    /// `0x00684ca0(i)` (`thiscall`, `ret 4`): word `+0xc` of entry 0 when
    /// the count (`+4`) is 2 or more, else of entry `i` (8 bytes each from
    /// `+0x10`).
    pub fn entry_word_c(mem: &impl Mem, this: u32, i: u32) -> u32 {
        let e: u32 = if mem.read::<i32>(this.wrapping_add(4)) >= 2 {
            mem.read(this.wrapping_add(0x10))
        } else {
            mem.read(this.wrapping_add(i.wrapping_mul(8)).wrapping_add(0x10))
        };
        mem.read(e.wrapping_add(0xc))
    }

    /// `0x00776910(rec)`: unlinks a record from the list in the block at
    /// `*0x0112a03c` (`+0x38` next, `+0x44` previous, both offsets in the
    /// block) and marks it 7. Returns its previous offset.
    pub fn unlink_record(mem: &mut impl Mem, rec: u32) -> u32 {
        let base = Global::<u32>::new(0x0112_a03c).get(mem);
        let prev: u32 = mem.read(rec.wrapping_add(0x44));
        let next: u32 = mem.read(rec.wrapping_add(0x38));
        mem.write(base.wrapping_add(prev).wrapping_add(0x38), next);
        let prev: u32 = mem.read(rec.wrapping_add(0x44));
        mem.write(next.wrapping_add(base).wrapping_add(0x44), prev);
        mem.write(rec, 7u8);
        prev
    }

    /// `0x009da750()`: 64 bytes copied from `0x022fb620` to `0x02311480`
    /// and 64 from `0x022fb660` to `0x02311440`.
    pub fn copy_9da750(mem: &mut impl Mem) {
        for (from, to) in [(0x022f_b620u32, 0x0231_1480u32), (0x022f_b660, 0x0231_1440)] {
            let v: [u32; 16] = mem.read(mem.rebase(from));
            let at = mem.rebase(to);
            mem.write(at, v);
        }
    }

    /// `0x00402b50()` (`thiscall`): the 0x51 words at `0x00c8a538` copied to
    /// `+0x60`, the count 0x51 at `+0x42c`. Returns the table's end.
    pub fn copy_c8a538(mem: &mut impl Mem, this: u32) -> u32 {
        let src = mem.rebase(0x00c8_a538);
        for k in 0..0x51u32 {
            let v: u32 = mem.read(src + 4 * k);
            mem.write(this.wrapping_add(0x60 + 4 * k), v);
        }
        mem.write(this.wrapping_add(0x42c), 0x51u32);
        src + 4 * 0x51
    }

    /// `0x00647440(p)`: the three words at `p` byte-swapped in place;
    /// returns 0x1c.
    pub fn swap3(mem: &mut impl Mem, p: u32) -> u32 {
        for k in 0..3u32 {
            let v: u32 = mem.read(p + 4 * k);
            mem.write(p + 4 * k, v.swap_bytes());
        }
        0x1c
    }

    /// `0x00647490(p, out)`: [`swap3`], then word 1 (swapped) to `*out`.
    pub fn swap3_out(mem: &mut impl Mem, p: u32, out: u32) -> u32 {
        swap3(mem, p);
        let v: u32 = mem.read(p + 4);
        mem.write(out, v);
        0x1c
    }

    /// `0x00402b00()` (`thiscall`, the settings): the three 0x51-word default
    /// tables (`0x00c8a538`, `0x00c8a3f0`, `0x00c8a2a8`) copied to `+0x60`,
    /// `+0x1a4` and `+0x2e8`, the count 0x51 at `+0x42c`. Returns 0.
    pub fn copy_default_tables(mem: &mut impl Mem, this: u32) -> u32 {
        for (src, dst) in [
            (0x00c8_a538u32, 0x60u32),
            (0x00c8_a3f0, 0x1a4),
            (0x00c8_a2a8, 0x2e8),
        ] {
            let src = mem.rebase(src);
            for k in 0..0x51u32 {
                let v: u32 = mem.read(src + 4 * k);
                mem.write(this.wrapping_add(dst + 4 * k), v);
            }
        }
        mem.write(this.wrapping_add(0x42c), 0x51u32);
        0
    }

    /// `0x00632930(out, a, b, c)` (`thiscall`, `ret 0x10`): the three words
    /// at `+0xac` to `out`, replaced by `a`, `b`, `c`; `+0`, `+4` zeroed.
    /// Returns `out`.
    pub fn swap_state_ac(mem: &mut impl Mem, this: u32, out: u32, v: [u32; 3]) -> u32 {
        let old: [u32; 3] = mem.read(this.wrapping_add(0xac));
        mem.write(out, old);
        mem.write(this.wrapping_add(0xac), v[0]);
        mem.write(this.wrapping_add(0xb0), v[1]);
        mem.write(this.wrapping_add(4), 0u32);
        mem.write(this, 0u32);
        mem.write(this.wrapping_add(0xb4), v[2]);
        out
    }

    /// `0x006fb320(n)` (`thiscall`, `ret 4`): for `n` in 1..=500 other than
    /// the current `+0xc`: `+0xc` = `n`, `+8` and `+0x1c` 0, then `+0x1c` 1
    /// when `n` is among the `+0x18` words at `*(this + 0x40)`. Returns 0.
    pub fn select_id(mem: &mut impl Mem, this: u32, n: u32) -> u32 {
        if n.wrapping_sub(1) > 0x1f3 || mem.read::<u32>(this.wrapping_add(0xc)) == n {
            return 0;
        }
        let count: i32 = mem.read(this.wrapping_add(0x18));
        mem.write(this.wrapping_add(0xc), n);
        mem.write(this.wrapping_add(0x1c), 0u32);
        mem.write(this.wrapping_add(8), 0u32);
        let list: u32 = mem.read(this.wrapping_add(0x40));
        for k in 0..count.max(0).cast_unsigned() {
            if mem.read::<u32>(list.wrapping_add(4 * k)) == n {
                mem.write(this.wrapping_add(0x1c), 1u32);
                break;
            }
        }
        0
    }

    /// `0x006e4020(key, a, b)` (`thiscall`, `ret 0xc`): the 12-byte entry
    /// (from `+4`, count `+0`) with `key`, appended when missing, gets the
    /// floats `a`, `b` (through the x87 stack). Returns 3 times its index.
    /// (A count of 0x100 or more writes to address 0 in the original.)
    pub fn set_pair_floats(mem: &mut impl Mem, this: u32, key: u32, a: f32, b: f32) -> u32 {
        let count: u32 = mem.read(this);
        let mut i = 0;
        while i < count && mem.read::<u32>(this.wrapping_add(4 + 12 * i)) != key {
            i += 1;
        }
        if i == count {
            mem.write(this, count.wrapping_add(1));
        }
        let e = this.wrapping_add(12 * i);
        mem.write(e.wrapping_add(8), ffx_bridge::x87::round_trip(a));
        mem.write(e.wrapping_add(4), key);
        mem.write(e.wrapping_add(0xc), ffx_bridge::x87::round_trip(b));
        i * 3
    }

    /// `Fixup_RunAll(fn, src, ctx, &dst)` (`0x0060ca70`, `thiscall` on the
    /// list, `ret 0x10`) and `Fixup_RunAll16` (`0x0060ca10`): for each of
    /// the list's `+4` items, `size` bytes (28 or 16) copied from `src` to
    /// `*dst`, then `fn(*dst, i, ctx, *list)` (cdecl, passed to `call`), and
    /// `*dst` moved past them. Returns `&dst`'s address for the 28-byte form
    /// and `fn`'s last result for the 16-byte one (0 for an empty list).
    pub fn fixup_run_all<M: Mem>(
        mem: &mut M,
        list: u32,
        args: [u32; 4],
        size: u32,
        mut call: impl FnMut(&mut M, u32, &[u32]) -> u32,
    ) -> u32 {
        let [f, src, ctx, dst] = args;
        let mut result = 0;
        let mut i = 0u32;
        while i < mem.read::<u32>(list.wrapping_add(4)) {
            let at: u32 = mem.read(dst);
            for k in (0..size).step_by(4) {
                let v: u32 = mem.read(src + k);
                mem.write(at + k, v);
            }
            let first: u32 = mem.read(list);
            result = call(mem, f, &[at, i, ctx, first]);
            let at: u32 = mem.read(dst);
            mem.write(dst, at.wrapping_add(size));
            i += 1;
        }
        if size == 0x1c && i > 0 { dst } else { result }
    }

    /// `0x006fbdc0(base, n)`: `n` 0x18-byte records reset (a null one
    /// skipped): byte `+0` 0, floats `+8..+0x18` 0.0. Returns the last
    /// record's `+0xc` slot address minus 0xc, as `eax` ends (the last
    /// record).
    pub fn reset_records_18(mem: &mut impl Mem, base: u32, n: u32) -> u32 {
        let mut rec = base;
        let mut last = 0;
        for _ in 0..n {
            last = rec;
            if rec != 0 {
                mem.write(rec.wrapping_add(8), 0.0f32);
                mem.write(rec, 0u8);
                mem.write(rec.wrapping_add(0xc), [0.0f32; 3]);
            }
            rec = rec.wrapping_add(0x18);
        }
        last
    }

    /// `0x006473d0(p)`: the 16-bit halves of `p[0..0x200]` and
    /// `p[0x300..0x340]` byte-swapped; returns 0x340.
    pub fn swap_halves(mem: &mut impl Mem, p: u32) -> u32 {
        for (from, to) in [(0u32, 0x200u32), (0x300, 0x340)] {
            for at in (from..to).step_by(2) {
                let v: u16 = mem.read(p + at);
                mem.write(p + at, v.swap_bytes());
            }
        }
        0x340
    }

    fn swap16(mem: &mut impl Mem, p: u32, from: u32, to: u32) {
        for at in (from..to).step_by(2) {
            let v: u16 = mem.read(p + at);
            mem.write(p + at, v.swap_bytes());
        }
    }

    fn swap32(mem: &mut impl Mem, p: u32, from: u32, to: u32) {
        for at in (from..to).step_by(4) {
            let v: u32 = mem.read(p + at);
            mem.write(p + at, v.swap_bytes());
        }
    }

    /// `0x00647220(p)`: halves of `p[0xf00..0xf10]` and words of
    /// `p[0xf10..0xf18]` byte-swapped; returns 0x1320.
    pub fn swap_header_f00(mem: &mut impl Mem, p: u32) -> u32 {
        swap16(mem, p, 0xf00, 0xf10);
        swap32(mem, p, 0xf10, 0xf18);
        0x1320
    }

    /// `0x00647560(p)`: words of `p[0..0x10]`, halves of `p[0x24..0x54]`,
    /// words of `p[0x5c..0x84]` byte-swapped; returns 0x84.
    pub fn swap_header_84(mem: &mut impl Mem, p: u32) -> u32 {
        swap32(mem, p, 0, 0x10);
        swap16(mem, p, 0x24, 0x54);
        swap32(mem, p, 0x5c, 0x84);
        0x84
    }

    /// `0x00687190(a, b)` (`thiscall`, `ret 8`): `+0` `a`, `+8` `b`, the
    /// other words to `+0x40` zeroed; returns `this`.
    pub fn init_68(mem: &mut impl Mem, this: u32, a: u32, b: u32) -> u32 {
        mem.write(this, a);
        mem.write(this.wrapping_add(8), b);
        mem.write(this.wrapping_add(4), 0u32);
        for off in (0xc..=0x40).step_by(4) {
            mem.write(this.wrapping_add(off), 0u32);
        }
        this
    }

    /// `0x0061d730(a, b)` (stdcall, `ret 8`): two pairs of signed 64-bit
    /// values compared in order: -1, 0 or 1.
    pub fn compare_i64_pairs(mem: &impl Mem, a: u32, b: u32) -> i32 {
        let x: [i64; 2] = [mem.read(a), mem.read(a.wrapping_add(8))];
        let y: [i64; 2] = [mem.read(b), mem.read(b.wrapping_add(8))];
        match x.cmp(&y) {
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Greater => 1,
        }
    }

    /// A setter of [`super::WORD_SETTERS2`]: stores `v`, returns it.
    pub fn set_word_global2(mem: &mut impl Mem, function: u32, v: u32) -> u32 {
        Global::<u32>::new(super::lookup(&super::WORD_SETTERS2, function)).set(mem, v);
        v
    }

    /// A setter of [`super::BYTE_SETTERS`].
    pub fn set_byte_global(mem: &mut impl Mem, function: u32, v: u8) {
        Global::<u8>::new(super::lookup(&super::BYTE_SETTERS, function)).set(mem, v);
    }

    /// [`super::COPY_THIS_TO_P`] (`at` 0) and [`super::COPY_THIS4_TO_P`]
    /// (`at` 4): `*p = this[at / 4]`; returns `p`.
    pub fn copy_member_to(mem: &mut impl Mem, this: u32, at: u32, p: u32) -> u32 {
        let v: u32 = mem.read(this.wrapping_add(at));
        mem.write(p, v);
        p
    }

    /// The `INDEX*` tables: `*(this + at) + i * scale`.
    pub fn index_member(mem: &impl Mem, this: u32, at: u32, i: u32, scale: u32) -> u32 {
        mem.read::<u32>(this.wrapping_add(at))
            .wrapping_add(i.wrapping_mul(scale))
    }
    /// `Vec3At24_0061a7f0(out)` (`thiscall`, `ret 4`): the three floats at
    /// `this + 0x24` to `out` through the x87 stack (a signalling NaN
    /// becomes quiet); returns `out`.
    pub fn vec3_at_24(mem: &mut impl Mem, this: u32, out: u32) -> u32 {
        for k in 0..3 {
            let v: f32 = mem.read(this.wrapping_add(0x24 + 4 * k));
            mem.write(out.wrapping_add(4 * k), ffx_bridge::x87::round_trip(v));
        }
        out
    }
}
