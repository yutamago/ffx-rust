//! The constants of the static-init-a originals, read off their machine
//! code: each row is one function at its `ffx.sym` address. The parity
//! tests (`ffx-oracle/tests/m5_static_init_a.rs`) run every row against its
//! original, and the shims (`ffx-hook/src/ported/m5_static_init_a.rs`) pass
//! their own address to look the row up.

/// `atexit(dtor)` alone: the original pushes the destructor, calls `atexit`
/// and returns its `eax`.
#[derive(Debug, Clone, Copy)]
pub struct DtorOnly {
    pub at: u32,
    pub dtor: u32,
}

/// `*dst = counter++; atexit(dtor)`: the original reads the counter, stores
/// the old value in `dst`, stores the value plus one back, and returns the
/// `atexit` result.
#[derive(Debug, Clone, Copy)]
pub struct NextId {
    pub at: u32,
    pub counter: u32,
    pub dst: u32,
    pub dtor: u32,
}

/// `*g = 0`.
#[derive(Debug, Clone, Copy)]
pub struct ZeroWord {
    pub at: u32,
    pub g: u32,
}

/// `*dst = *src`, returning the copied word (`eax`).
#[derive(Debug, Clone, Copy)]
pub struct CopyWord {
    pub at: u32,
    pub src: u32,
    pub dst: u32,
}

/// `if FindPass(name) == 0 { *dst = *head; *head = node }; atexit(dtor)`: a
/// scene render pass type linked into its list unless one of that name is
/// already there.
#[derive(Debug, Clone, Copy)]
pub struct FindLink {
    pub at: u32,
    pub name: u32,
    pub head: u32,
    pub dst: u32,
    pub node: u32,
    pub dtor: u32,
}

/// `callee(this[, arg])` as a thiscall, then `atexit(dtor)`: a static
/// object's constructor and its destructor's registration.
#[derive(Debug, Clone, Copy)]
pub struct CallDtor {
    pub at: u32,
    pub callee: u32,
    pub this: u32,
    pub arg: Option<u32>,
    pub dtor: u32,
}

/// Every `atexit`-only initialiser.
pub const DTOR_ONLY: &[DtorOnly] = &[
    DtorOnly {
        at: 0x00ae_03b0,
        dtor: 0x00af_9b20,
    },
    DtorOnly {
        at: 0x00ae_03c0,
        dtor: 0x00af_9b10,
    },
    DtorOnly {
        at: 0x00ae_03d0,
        dtor: 0x00af_9b00,
    },
    DtorOnly {
        at: 0x00ae_0ff0,
        dtor: 0x00af_a690,
    },
    DtorOnly {
        at: 0x00ae_1a00,
        dtor: 0x00af_b980,
    },
    DtorOnly {
        at: 0x00ae_5ce0,
        dtor: 0x00af_e870,
    },
    DtorOnly {
        at: 0x00ae_9080,
        dtor: 0x00b0_0fd0,
    },
    DtorOnly {
        at: 0x00ae_a4d0,
        dtor: 0x00b0_3570,
    },
    DtorOnly {
        at: 0x00ae_a4e0,
        dtor: 0x00b0_3580,
    },
    DtorOnly {
        at: 0x00ae_a4f0,
        dtor: 0x00b0_3590,
    },
    DtorOnly {
        at: 0x00ae_bef0,
        dtor: 0x00b0_4760,
    },
    DtorOnly {
        at: 0x00ae_bf00,
        dtor: 0x00b0_4770,
    },
    DtorOnly {
        at: 0x00ae_bf10,
        dtor: 0x00b0_4780,
    },
    DtorOnly {
        at: 0x00ae_bf20,
        dtor: 0x00b0_4790,
    },
    DtorOnly {
        at: 0x00ae_bf30,
        dtor: 0x00b0_47a0,
    },
    DtorOnly {
        at: 0x00ae_c9b0,
        dtor: 0x00b0_4b40,
    },
];

/// Every counter initialiser (`static` ids).
pub const NEXT_ID: &[NextId] = &[
    NextId {
        at: 0x00ae_7790,
        counter: 0x00ca_9738,
        dst: 0x00c1_a9d0,
        dtor: 0x00af_ff40,
    },
    NextId {
        at: 0x00ae_77b0,
        counter: 0x00ca_9738,
        dst: 0x00c1_aa00,
        dtor: 0x00af_ff50,
    },
    NextId {
        at: 0x00ae_77d0,
        counter: 0x00ca_9738,
        dst: 0x00c1_a9f0,
        dtor: 0x00af_ff60,
    },
    NextId {
        at: 0x00ae_77f0,
        counter: 0x00ca_9738,
        dst: 0x00c1_a9e0,
        dtor: 0x00af_ff70,
    },
    NextId {
        at: 0x00ae_7810,
        counter: 0x00ca_9738,
        dst: 0x00c1_a9a0,
        dtor: 0x00af_ff80,
    },
    NextId {
        at: 0x00ae_7830,
        counter: 0x00ca_9738,
        dst: 0x00c1_a9c0,
        dtor: 0x00af_ff90,
    },
    NextId {
        at: 0x00ae_7850,
        counter: 0x00ca_9738,
        dst: 0x00c1_a9b0,
        dtor: 0x00af_ffa0,
    },
    NextId {
        at: 0x00ae_7870,
        counter: 0x00ca_9738,
        dst: 0x00c1_aa10,
        dtor: 0x00af_ffb0,
    },
    NextId {
        at: 0x00ae_7890,
        counter: 0x00ca_9738,
        dst: 0x00c1_a930,
        dtor: 0x00af_ffc0,
    },
    NextId {
        at: 0x00ae_78b0,
        counter: 0x00ca_9738,
        dst: 0x00c1_a950,
        dtor: 0x00af_ffd0,
    },
    NextId {
        at: 0x00ae_78d0,
        counter: 0x00ca_9738,
        dst: 0x00c1_a990,
        dtor: 0x00af_ffe0,
    },
    NextId {
        at: 0x00ae_78f0,
        counter: 0x00ca_9738,
        dst: 0x00c1_a980,
        dtor: 0x00af_fff0,
    },
    NextId {
        at: 0x00ae_7910,
        counter: 0x00ca_9738,
        dst: 0x00c1_a970,
        dtor: 0x00b0_0000,
    },
    NextId {
        at: 0x00ae_7930,
        counter: 0x00ca_9738,
        dst: 0x00c1_a960,
        dtor: 0x00b0_0010,
    },
    NextId {
        at: 0x00ae_7950,
        counter: 0x00ca_9738,
        dst: 0x00c1_a940,
        dtor: 0x00b0_0020,
    },
    NextId {
        at: 0x00ae_7970,
        counter: 0x00ca_9738,
        dst: 0x00c1_aa20,
        dtor: 0x00b0_0030,
    },
    NextId {
        at: 0x00ae_7990,
        counter: 0x00ca_9738,
        dst: 0x00c1_aa40,
        dtor: 0x00b0_0040,
    },
    NextId {
        at: 0x00ae_79b0,
        counter: 0x00ca_9738,
        dst: 0x00c1_aa30,
        dtor: 0x00b0_0050,
    },
    NextId {
        at: 0x00ae_79d0,
        counter: 0x00ca_9738,
        dst: 0x00c1_aa50,
        dtor: 0x00b0_0060,
    },
];

/// Every initialiser that clears one word.
pub const ZERO_WORD: &[ZeroWord] = &[
    ZeroWord {
        at: 0x00ae_03e0,
        g: 0x022f_b538,
    },
    ZeroWord {
        at: 0x00ae_03f0,
        g: 0x022f_b534,
    },
];

/// Every initialiser that copies one word.
pub const COPY_WORD: &[CopyWord] = &[
    CopyWord {
        at: 0x00ae_0400,
        src: 0x00b0_c468,
        dst: 0x022f_b510,
    },
    CopyWord {
        at: 0x00ae_0410,
        src: 0x00b0_c370,
        dst: 0x022f_b514,
    },
];

/// Every scene render pass type registration (`FindPass`, then link).
pub const FIND_LINK: &[FindLink] = &[
    FindLink {
        at: 0x00ae_6290,
        name: 0x00b2_0a58,
        head: 0x00ca_54f8,
        dst: 0x00c1_69c8,
        node: 0x00c1_69c4,
        dtor: 0x00af_ea70,
    },
    FindLink {
        at: 0x00ae_62d0,
        name: 0x00b2_0acc,
        head: 0x00ca_54f8,
        dst: 0x00c1_6bb8,
        node: 0x00c1_6bb4,
        dtor: 0x00af_ea80,
    },
    FindLink {
        at: 0x00ae_6310,
        name: 0x00b2_0ae4,
        head: 0x00ca_54f8,
        dst: 0x00c1_6bc8,
        node: 0x00c1_6bc4,
        dtor: 0x00af_ea90,
    },
    FindLink {
        at: 0x00ae_6350,
        name: 0x00b2_0958,
        head: 0x00ca_54f8,
        dst: 0x00c1_6a88,
        node: 0x00c1_6a84,
        dtor: 0x00af_eaa0,
    },
    FindLink {
        at: 0x00ae_6390,
        name: 0x00b2_0aac,
        head: 0x00ca_54f8,
        dst: 0x00c1_6ba8,
        node: 0x00c1_6ba4,
        dtor: 0x00af_eab0,
    },
    FindLink {
        at: 0x00ae_63d0,
        name: 0x00b2_0a98,
        head: 0x00ca_54f8,
        dst: 0x00c1_6b98,
        node: 0x00c1_6b94,
        dtor: 0x00af_eac0,
    },
    FindLink {
        at: 0x00ae_6410,
        name: 0x00b2_0a2c,
        head: 0x00ca_54f8,
        dst: 0x00c1_6b28,
        node: 0x00c1_6b24,
        dtor: 0x00af_ead0,
    },
    FindLink {
        at: 0x00ae_6450,
        name: 0x00b2_0a38,
        head: 0x00ca_54f8,
        dst: 0x00c1_6b38,
        node: 0x00c1_6b34,
        dtor: 0x00af_eae0,
    },
    FindLink {
        at: 0x00ae_6490,
        name: 0x00b2_0a74,
        head: 0x00ca_54f8,
        dst: 0x00c1_6b78,
        node: 0x00c1_6b74,
        dtor: 0x00af_eaf0,
    },
    FindLink {
        at: 0x00ae_64d0,
        name: 0x00b2_0a14,
        head: 0x00ca_54f8,
        dst: 0x00c1_6b18,
        node: 0x00c1_6b14,
        dtor: 0x00af_eb00,
    },
    FindLink {
        at: 0x00ae_6510,
        name: 0x00b2_0a24,
        head: 0x00ca_54f8,
        dst: 0x00c1_6b48,
        node: 0x00c1_6b44,
        dtor: 0x00af_eb10,
    },
    FindLink {
        at: 0x00ae_6550,
        name: 0x00b1_3d7c,
        head: 0x00ca_54f8,
        dst: 0x00c1_6bd8,
        node: 0x00c1_6bd4,
        dtor: 0x00af_eb20,
    },
    FindLink {
        at: 0x00ae_6590,
        name: 0x00b2_08f8,
        head: 0x00ca_54f8,
        dst: 0x00c1_69e8,
        node: 0x00c1_69e4,
        dtor: 0x00af_eb30,
    },
    FindLink {
        at: 0x00ae_65d0,
        name: 0x00b2_08e4,
        head: 0x00ca_54f8,
        dst: 0x00c1_69b8,
        node: 0x00c1_69b4,
        dtor: 0x00af_eb40,
    },
    FindLink {
        at: 0x00ae_6610,
        name: 0x00b2_0a88,
        head: 0x00ca_54f8,
        dst: 0x00c1_6b88,
        node: 0x00c1_6b84,
        dtor: 0x00af_eb50,
    },
    FindLink {
        at: 0x00ae_6650,
        name: 0x00b2_09f8,
        head: 0x00ca_54f8,
        dst: 0x00c1_6af8,
        node: 0x00c1_6af4,
        dtor: 0x00af_eb60,
    },
    FindLink {
        at: 0x00ae_6690,
        name: 0x00b2_0a64,
        head: 0x00ca_54f8,
        dst: 0x00c1_6b68,
        node: 0x00c1_6b64,
        dtor: 0x00af_eb70,
    },
    FindLink {
        at: 0x00ae_66d0,
        name: 0x00b2_0938,
        head: 0x00ca_54f8,
        dst: 0x00c1_6a68,
        node: 0x00c1_6a64,
        dtor: 0x00af_eb80,
    },
    FindLink {
        at: 0x00ae_6710,
        name: 0x00b7_c868,
        head: 0x00ca_54f8,
        dst: 0x00c1_6a58,
        node: 0x00c1_6a54,
        dtor: 0x00af_eb90,
    },
    FindLink {
        at: 0x00ae_6750,
        name: 0x00b2_09e4,
        head: 0x00ca_54f8,
        dst: 0x00c1_6ae8,
        node: 0x00c1_6ae4,
        dtor: 0x00af_eba0,
    },
    FindLink {
        at: 0x00ae_6790,
        name: 0x00b2_0968,
        head: 0x00ca_54f8,
        dst: 0x00c1_6a98,
        node: 0x00c1_6a94,
        dtor: 0x00af_ebb0,
    },
    FindLink {
        at: 0x00ae_67d0,
        name: 0x00b2_09ac,
        head: 0x00ca_54f8,
        dst: 0x00c1_6ac8,
        node: 0x00c1_6ac4,
        dtor: 0x00af_ebc0,
    },
    FindLink {
        at: 0x00ae_6810,
        name: 0x00b2_09c8,
        head: 0x00ca_54f8,
        dst: 0x00c1_6ad8,
        node: 0x00c1_6ad4,
        dtor: 0x00af_ebd0,
    },
    FindLink {
        at: 0x00ae_6850,
        name: 0x00b2_0978,
        head: 0x00ca_54f8,
        dst: 0x00c1_6aa8,
        node: 0x00c1_6aa4,
        dtor: 0x00af_ebe0,
    },
    FindLink {
        at: 0x00ae_6890,
        name: 0x00b2_0990,
        head: 0x00ca_54f8,
        dst: 0x00c1_6ab8,
        node: 0x00c1_6ab4,
        dtor: 0x00af_ebf0,
    },
    FindLink {
        at: 0x00ae_68d0,
        name: 0x00b2_0a44,
        head: 0x00ca_54f8,
        dst: 0x00c1_6b58,
        node: 0x00c1_6b54,
        dtor: 0x00af_ec00,
    },
    FindLink {
        at: 0x00ae_6910,
        name: 0x00b2_0a04,
        head: 0x00ca_54f8,
        dst: 0x00c1_6b08,
        node: 0x00c1_6b04,
        dtor: 0x00af_ec10,
    },
    FindLink {
        at: 0x00ae_6950,
        name: 0x00b2_0908,
        head: 0x00ca_54f8,
        dst: 0x00c1_69f8,
        node: 0x00c1_69f4,
        dtor: 0x00af_ec20,
    },
    FindLink {
        at: 0x00ae_6990,
        name: 0x00b2_08ec,
        head: 0x00ca_54f8,
        dst: 0x00c1_69d8,
        node: 0x00c1_69d4,
        dtor: 0x00af_ec30,
    },
    FindLink {
        at: 0x00ae_69d0,
        name: 0x00b2_094c,
        head: 0x00ca_54f8,
        dst: 0x00c1_6a78,
        node: 0x00c1_6a74,
        dtor: 0x00af_ec40,
    },
    FindLink {
        at: 0x00ae_6a10,
        name: 0x00b2_091c,
        head: 0x00ca_54f8,
        dst: 0x00c1_6a38,
        node: 0x00c1_6a34,
        dtor: 0x00af_ec50,
    },
    FindLink {
        at: 0x00ae_6a50,
        name: 0x00b2_0928,
        head: 0x00ca_54f8,
        dst: 0x00c1_6a48,
        node: 0x00c1_6a44,
        dtor: 0x00af_ec60,
    },
    FindLink {
        at: 0x00ae_6a90,
        name: 0x00b2_08b4,
        head: 0x00ca_54f8,
        dst: 0x00c1_6a08,
        node: 0x00c1_6a04,
        dtor: 0x00af_ec70,
    },
    FindLink {
        at: 0x00ae_6ad0,
        name: 0x00b2_08d0,
        head: 0x00ca_54f8,
        dst: 0x00c1_6a28,
        node: 0x00c1_6a24,
        dtor: 0x00af_ec80,
    },
    FindLink {
        at: 0x00ae_6b10,
        name: 0x00b2_08c0,
        head: 0x00ca_54f8,
        dst: 0x00c1_6a18,
        node: 0x00c1_6a14,
        dtor: 0x00af_ec90,
    },
];

/// Every constructor-then-destructor initialiser.
pub const CALL_DTOR: &[CallDtor] = &[
    CallDtor {
        at: 0x00ae_0510,
        callee: 0x0042_ef40,
        this: 0x00c8_fb78,
        arg: None,
        dtor: 0x00af_9ba0,
    },
    CallDtor {
        at: 0x00ae_0710,
        callee: 0x0043_3870,
        this: 0x00c9_0288,
        arg: None,
        dtor: 0x00af_9e10,
    },
    CallDtor {
        at: 0x00ae_0730,
        callee: 0x0057_0670,
        this: 0x00c9_02e8,
        arg: None,
        dtor: 0x00af_9f00,
    },
    CallDtor {
        at: 0x00ae_0870,
        callee: 0x0057_5870,
        this: 0x00c9_07f4,
        arg: Some(0x00b0_ec30),
        dtor: 0x00af_a060,
    },
    CallDtor {
        at: 0x00ae_1620,
        callee: 0x0043_1690,
        this: 0x00c9_40c4,
        arg: None,
        dtor: 0x00af_af60,
    },
    CallDtor {
        at: 0x00ae_5ba0,
        callee: 0x004d_2fa0,
        this: 0x00ca_4ec4,
        arg: None,
        dtor: 0x00af_e7a0,
    },
    CallDtor {
        at: 0x00ae_7170,
        callee: 0x005b_3620,
        this: 0x00ca_8998,
        arg: None,
        dtor: 0x00af_f950,
    },
    CallDtor {
        at: 0x00ae_7190,
        callee: 0x005b_3620,
        this: 0x00ca_89a8,
        arg: None,
        dtor: 0x00af_f960,
    },
    CallDtor {
        at: 0x00ae_a5b0,
        callee: 0x0043_15e0,
        this: 0x00cb_3c94,
        arg: None,
        dtor: 0x00b0_3680,
    },
];
