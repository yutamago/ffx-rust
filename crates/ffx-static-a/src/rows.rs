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

/// What the class-descriptor initialiser does to its flag word after the
/// buffers are set: `And`: `[flags] &= m`, then `[this] = vt2`; `AndOr`:
/// `[this] = vt2`, then `[flags] = ([flags] & m1) | m2`; `Keep`: nothing (the
/// `|= 2` stays).
#[derive(Debug, Clone, Copy)]
pub enum FlagTail {
    And { m: u32, vt2: u32 },
    AndOr { m1: u32, m2: u32, vt2: u32 },
    Keep,
}

/// `Phyre::PClassDescriptor(this, GetNamespace(), name, size, align, base, c)`
/// (thiscall, six words), then the flag word `|= 2`, the vtable `vt1`, and the
/// three buffer setters, the tail, and `atexit(dtor)`. `getter` is 0 when
/// `base` is the base; otherwise the base is the result of the getter (cdecl,
/// no arguments), called before the namespace.
#[derive(Debug, Clone, Copy)]
pub struct ClassDesc {
    pub at: u32,
    pub base: u32,
    pub getter: u32,
    pub c: u32,
    pub align: u32,
    pub size: u32,
    pub name: u32,
    pub this: u32,
    pub flags: u32,
    pub vt1: u32,
    pub def: u32,
    pub val: u32,
    pub wm: u32,
    pub tail: FlagTail,
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

/// `Ctor8(this, w0..w7)` as a thiscall (eight stack words, the callee pops
/// them), then `atexit(dtor)`. Bit `i` of `addr` set: `words[i]` is an image
/// address (the loader relocates it); the others are literals.
#[derive(Debug, Clone, Copy)]
pub struct Ctor8 {
    pub at: u32,
    pub this: u32,
    pub words: [u32; 8],
    pub addr: u8,
    pub dtor: u32,
}

/// `PhyreStaticEntry(this, a, b)` as a thiscall (two stack words, the callee
/// pops them), then `atexit(dtor)`. Bit `i` of `addr`: `words[i]` is an image
/// address.
#[derive(Debug, Clone, Copy)]
pub struct StaticEntry {
    pub at: u32,
    pub this: u32,
    pub words: [u32; 2],
    pub addr: u8,
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

/// Every class-descriptor initialiser (the base is `base`, or the getter's result when `getter` is set; the flag tail is `tail`).
pub const CLASS_DESC: &[ClassDesc] = &[];

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

/// Every eight-word constructor initialiser.
pub const CTOR8: &[Ctor8] = &[
    Ctor8 {
        at: 0x00ae_49e0,
        this: 0x00ca_3094,
        words: [
            0x00b1_db7c,
            0x0000_0005,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0039,
            0x0000_0039,
            0x0000_0039,
            0x0000_0039,
        ],
        addr: 0x01,
        dtor: 0x00af_e050,
    },
    Ctor8 {
        at: 0x00ae_4a10,
        this: 0x00ca_315c,
        words: [
            0x00b1_dba0,
            0x0000_000d,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0018,
            0x0000_0018,
            0x0000_0018,
            0x0000_0018,
        ],
        addr: 0x01,
        dtor: 0x00af_e060,
    },
    Ctor8 {
        at: 0x00ae_4a40,
        this: 0x00ca_2ff4,
        words: [
            0x00b1_db6c,
            0x0000_0001,
            0x0000_01f8,
            0x0000_0000,
            0x0000_003d,
            0x0000_003d,
            0x0000_003d,
            0x0000_003d,
        ],
        addr: 0x01,
        dtor: 0x00af_e070,
    },
    Ctor8 {
        at: 0x00ae_4a70,
        this: 0x00ca_3134,
        words: [
            0x00b1_db98,
            0x0000_000c,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0057,
            0x0000_0057,
            0x0000_0057,
            0x0000_0057,
        ],
        addr: 0x01,
        dtor: 0x00af_e080,
    },
    Ctor8 {
        at: 0x00ae_4aa0,
        this: 0x00ca_3224,
        words: [
            0x00b1_dbc8,
            0x0000_0012,
            0x0000_0038,
            0x0000_0000,
            0x0000_0050,
            0x0000_0000,
            0x0000_0050,
            0x0000_0000,
        ],
        addr: 0x01,
        dtor: 0x00af_e090,
    },
    Ctor8 {
        at: 0x00ae_4ad0,
        this: 0x00ca_324c,
        words: [
            0x00b1_dbcc,
            0x0000_0013,
            0x0000_0038,
            0x0000_0000,
            0x0000_0053,
            0x0000_0000,
            0x0000_0053,
            0x0000_0000,
        ],
        addr: 0x01,
        dtor: 0x00af_e0a0,
    },
    Ctor8 {
        at: 0x00ae_4b00,
        this: 0x00ca_342c,
        words: [
            0x00b1_dc24,
            0x0000_0026,
            0x0000_0078,
            0x0000_0000,
            0x0000_0035,
            0x0000_0037,
            0x0000_0038,
            0x0000_0038,
        ],
        addr: 0x01,
        dtor: 0x00af_e0b0,
    },
    Ctor8 {
        at: 0x00ae_4b30,
        this: 0x00ca_347c,
        words: [
            0x00b1_dc34,
            0x0000_0028,
            0x0000_0078,
            0x0000_0000,
            0x0000_002c,
            0x0000_002d,
            0x0000_002e,
            0x0000_002e,
        ],
        addr: 0x01,
        dtor: 0x00af_e0c0,
    },
    Ctor8 {
        at: 0x00ae_4b60,
        this: 0x00ca_3454,
        words: [
            0x00b1_dc2c,
            0x0000_0027,
            0x0000_0078,
            0x0000_0000,
            0x0000_002c,
            0x0000_002d,
            0x0000_002e,
            0x0000_002e,
        ],
        addr: 0x01,
        dtor: 0x00af_e0d0,
    },
    Ctor8 {
        at: 0x00ae_4b90,
        this: 0x00ca_34a4,
        words: [
            0x00b1_dc40,
            0x0000_0029,
            0x0000_0078,
            0x0000_0000,
            0x0000_0027,
            0x0000_0028,
            0x0000_0029,
            0x0000_0029,
        ],
        addr: 0x01,
        dtor: 0x00af_e0e0,
    },
    Ctor8 {
        at: 0x00ae_4bc0,
        this: 0x00ca_31ac,
        words: [
            0x00b1_dbb0,
            0x0000_000f,
            0x0000_0038,
            0x0000_0000,
            0x0000_0047,
            0x0000_0000,
            0x0000_0047,
            0x0000_0000,
        ],
        addr: 0x01,
        dtor: 0x00af_e0f0,
    },
    Ctor8 {
        at: 0x00ae_4bf0,
        this: 0x00ca_31d4,
        words: [
            0x00b1_dbb8,
            0x0000_0010,
            0x0000_0038,
            0x0000_0000,
            0x0000_004a,
            0x0000_0000,
            0x0000_004a,
            0x0000_0000,
        ],
        addr: 0x01,
        dtor: 0x00af_e100,
    },
    Ctor8 {
        at: 0x00ae_4c20,
        this: 0x00ca_31fc,
        words: [
            0x00b1_dbc0,
            0x0000_0011,
            0x0000_0038,
            0x0000_0000,
            0x0000_004d,
            0x0000_0000,
            0x0000_004d,
            0x0000_0000,
        ],
        addr: 0x01,
        dtor: 0x00af_e110,
    },
    Ctor8 {
        at: 0x00ae_4c50,
        this: 0x00ca_32ec,
        words: [
            0x00b1_dbe8,
            0x0000_001f,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0036,
            0x0000_0036,
            0x0000_0036,
            0x0000_0036,
        ],
        addr: 0x01,
        dtor: 0x00af_e120,
    },
    Ctor8 {
        at: 0x00ae_4c80,
        this: 0x00ca_306c,
        words: [
            0x00b1_db78,
            0x0000_0004,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0039,
            0x0000_0039,
            0x0000_0039,
            0x0000_0039,
        ],
        addr: 0x01,
        dtor: 0x00af_e130,
    },
    Ctor8 {
        at: 0x00ae_4cb0,
        this: 0x00ca_333c,
        words: [
            0x00b1_dbf8,
            0x0000_0021,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0029,
            0x0000_0029,
            0x0000_0029,
            0x0000_0029,
        ],
        addr: 0x01,
        dtor: 0x00af_e140,
    },
    Ctor8 {
        at: 0x00ae_4ce0,
        this: 0x00ca_2fcc,
        words: [
            0x00b1_db68,
            0x0000_0000,
            0x0000_01f8,
            0x0000_0000,
            0x0000_003d,
            0x0000_003d,
            0x0000_003d,
            0x0000_003d,
        ],
        addr: 0x01,
        dtor: 0x00af_e150,
    },
    Ctor8 {
        at: 0x00ae_4d10,
        this: 0x00ca_338c,
        words: [
            0x00b1_dc08,
            0x0000_0023,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0022,
            0x0000_0022,
            0x0000_0022,
            0x0000_0022,
        ],
        addr: 0x01,
        dtor: 0x00af_e160,
    },
    Ctor8 {
        at: 0x00ae_4d40,
        this: 0x00ca_30bc,
        words: [
            0x00b1_db80,
            0x0000_0006,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0024,
            0x0000_0024,
            0x0000_0024,
            0x0000_0024,
        ],
        addr: 0x01,
        dtor: 0x00af_e170,
    },
    Ctor8 {
        at: 0x00ae_4d70,
        this: 0x00ca_33dc,
        words: [
            0x00b1_dc18,
            0x0000_0025,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0010,
            0x0000_0010,
            0x0000_0010,
            0x0000_0010,
        ],
        addr: 0x01,
        dtor: 0x00af_e180,
    },
    Ctor8 {
        at: 0x00ae_4da0,
        this: 0x00ca_301c,
        words: [
            0x00b1_db70,
            0x0000_0002,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0031,
            0x0000_0031,
            0x0000_0031,
            0x0000_0031,
        ],
        addr: 0x01,
        dtor: 0x00af_e190,
    },
    Ctor8 {
        at: 0x00ae_4dd0,
        this: 0x00ca_32c4,
        words: [
            0x00b1_dbe0,
            0x0000_001e,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0036,
            0x0000_0036,
            0x0000_0036,
            0x0000_0036,
        ],
        addr: 0x01,
        dtor: 0x00af_e1a0,
    },
    Ctor8 {
        at: 0x00ae_4e00,
        this: 0x00ca_3314,
        words: [
            0x00b1_dbf0,
            0x0000_0020,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0029,
            0x0000_0029,
            0x0000_0029,
            0x0000_0029,
        ],
        addr: 0x01,
        dtor: 0x00af_e1b0,
    },
    Ctor8 {
        at: 0x00ae_4e30,
        this: 0x00ca_3404,
        words: [
            0x00b1_dc20,
            0x0000_002f,
            0x0000_01f8,
            0x0000_0000,
            0x0000_002a,
            0x0000_002a,
            0x0000_002a,
            0x0000_002a,
        ],
        addr: 0x01,
        dtor: 0x00af_e1c0,
    },
    Ctor8 {
        at: 0x00ae_4e60,
        this: 0x00ca_3364,
        words: [
            0x00b1_dc00,
            0x0000_0022,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0022,
            0x0000_0022,
            0x0000_0022,
            0x0000_0022,
        ],
        addr: 0x01,
        dtor: 0x00af_e1d0,
    },
    Ctor8 {
        at: 0x00ae_4e90,
        this: 0x00ca_30e4,
        words: [
            0x00b1_db88,
            0x0000_0007,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0024,
            0x0000_0024,
            0x0000_0024,
            0x0000_0024,
        ],
        addr: 0x01,
        dtor: 0x00af_e1e0,
    },
    Ctor8 {
        at: 0x00ae_4ec0,
        this: 0x00ca_33b4,
        words: [
            0x00b1_dc10,
            0x0000_0024,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0010,
            0x0000_0010,
            0x0000_0010,
            0x0000_0010,
        ],
        addr: 0x01,
        dtor: 0x00af_e1f0,
    },
    Ctor8 {
        at: 0x00ae_4ef0,
        this: 0x00ca_3044,
        words: [
            0x00b1_db74,
            0x0000_0003,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0031,
            0x0000_0031,
            0x0000_0031,
            0x0000_0031,
        ],
        addr: 0x01,
        dtor: 0x00af_e200,
    },
    Ctor8 {
        at: 0x00ae_4f20,
        this: 0x00ca_3274,
        words: [
            0x00b1_dbd0,
            0x0000_001c,
            0x0000_01f8,
            0x0000_0000,
            0x0000_000a,
            0x0000_000a,
            0x0000_000a,
            0x0000_000a,
        ],
        addr: 0x01,
        dtor: 0x00af_e210,
    },
    Ctor8 {
        at: 0x00ae_4f50,
        this: 0x00ca_3184,
        words: [
            0x00b1_dba8,
            0x0000_000e,
            0x0000_01f8,
            0x0000_0000,
            0x0000_000c,
            0x0000_000c,
            0x0000_000c,
            0x0000_000c,
        ],
        addr: 0x01,
        dtor: 0x00af_e220,
    },
    Ctor8 {
        at: 0x00ae_4f80,
        this: 0x00ca_329c,
        words: [
            0x00b1_dbd8,
            0x0000_001d,
            0x0000_01f8,
            0x0000_0000,
            0x0000_0002,
            0x0000_0002,
            0x0000_0002,
            0x0000_0002,
        ],
        addr: 0x01,
        dtor: 0x00af_e230,
    },
    Ctor8 {
        at: 0x00ae_4fb0,
        this: 0x00ca_310c,
        words: [
            0x00b1_db90,
            0x0000_000b,
            0x0000_01f8,
            0x0000_0000,
            0x0000_001c,
            0x0000_001c,
            0x0000_001c,
            0x0000_001c,
        ],
        addr: 0x01,
        dtor: 0x00af_e240,
    },
    Ctor8 {
        at: 0x00ae_5000,
        this: 0x00ca_34cc,
        words: [
            0x00b1_2ab0,
            0x0000_0030,
            0x0000_0000,
            0x0000_0000,
            0x0000_0000,
            0x0000_0000,
            0x0000_0000,
            0x0000_0000,
        ],
        addr: 0x01,
        dtor: 0x00af_e250,
    },
];

/// Every two-word static entry initialiser.
pub const STATIC_ENTRY: &[StaticEntry] = &[];
