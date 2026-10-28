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
