//! List p6-render-c, continuation round 2 (batch 13): post-processing
//! render-target set-up, the camera's view matrices and the renderer's
//! `setCamera` (matrices in the frame), the render thread's constructor
//! and main loop, a few constructors and destructors, the Iggy player's
//! creation and `PUtilityText`'s constructor.
//!
//! Conventions are read from the disassembly; `eax` is returned wherever
//! the original defines it.

use ffx_bridge::x87;
use ffx_graphic::host::{Conv, Host};

/// `Phyre_PhyreFree(p)` and `Phyre_PhyreMalloc(n)` (cdecl); the CRT's
/// `memset(p, c, n)` (cdecl).
pub const FREE: u32 = 0x0042_fb20;
pub const MALLOC: u32 = 0x0042_fb40;
pub const MEMSET: u32 = 0x0094_964c;
/// The MSVC `eh vector constructor iterator(p, size, n, ctor, dtor)`
/// (stdcall).
pub const VECTOR_CTOR: u32 = 0x0094_966a;

/// A post effect's render targets: `PCluster::create(cluster; 1, type,
/// 0)` (thiscall) of the 2D texture type, `PTexture2DBase::setDimensions
/// (tex; w, h, format, 0)`, `updateMaxLevel(tex)`, the render target's
/// creation `0x0064adf0(cluster; tex)` (thiscall), and the owner string
/// `"PPostProcessUtils"`.
pub const CLUSTER_CREATE: u32 = 0x0044_1010;
pub const TEXTURE_TYPE: u32 = 0x00ca_37d0;
pub const SET_DIMENSIONS: u32 = 0x004c_40c0;
pub const TARGET_FORMAT: u32 = 0x00ca_310c;
pub const UPDATE_MAX_LEVEL: u32 = 0x004c_4250;
pub const CREATE_TARGET: u32 = 0x0064_adf0;
pub const OWNER: u32 = 0x00b7_6848;
/// The legacy glow's other set-up: `0x00440b70(cluster; 1, type, 0)`
/// (thiscall) of the render target type `0x00ca4200`, whose object `+0x34`
/// takes the texture (`0x00583a60(target; tex)`); the effects
/// `0x00a05e70(cluster)` and `0x00a06120(cluster)` (cdecl).
pub const CLUSTER_CREATE2: u32 = 0x0044_0b70;
pub const TARGET_TYPE: u32 = 0x00ca_4200;
pub const TARGET_SET_TEXTURE: u32 = 0x0058_3a60;
pub const GLOW_EFFECT: u32 = 0x00a0_5e70;
pub const GLOW_GPU_EFFECT: u32 = 0x00a0_6120;
/// `PLegacyGlowGPUBase`'s parts: the base set-up `0x00a1a190(this; c, w,
/// h)` and `0x00a0ff50(this; w, h, 7, 1.0)` (thiscall).
pub const GLOW_BASE_SETUP: u32 = 0x00a1_a190;
pub const GLOW_TARGETS: u32 = 0x00a0_ff50;

/// `PDeferredLightingBase`'s deleting destructor's vtables, its part
/// destructor `0x00a24a60` and the three members' `0x00585230`.
pub const DEFERRED_VTABLE: u32 = 0x00b7_4160;
pub const POST_EFFECT_VTABLE: u32 = 0x00b7_4058;
pub const DEFERRED_PARTS: u32 = 0x00a2_4a60;
pub const MEMBER_DTOR: u32 = 0x0058_5230;
pub const DEFERRED_MEMBERS: [u32; 3] = [0x3f30, 0x3f0c, 0x3ee8];
/// `PDeferredLighting`'s vtable and parts destructor `0x00a24b00`.
pub const DEFERRED_LIGHTING_VTABLE: u32 = 0x00b7_4270;
pub const DEFERRED_LIGHTING_PARTS: u32 = 0x00a2_4b00;

/// The matrix helpers (Vectormath): `0x00451080(m; src)` (thiscall,
/// a 4x4 from a 3x4 or a node), `0x004cc0c0(out, m)` (cdecl, the
/// inverse), `0x004c9430(m; src)` (thiscall, copy), `0x004ca360(out, a,
/// b)` (cdecl, the product), `0x00453340(m; src)` (thiscall, copy) and
/// the camera's `0x00451190(camera; m)` (thiscall).
pub const FROM_NODE: u32 = 0x0045_1080;
pub const INVERSE: u32 = 0x004c_c0c0;
pub const COPY: u32 = 0x004c_9430;
pub const MULTIPLY: u32 = 0x004c_a360;
pub const COPY2: u32 = 0x0045_3340;
pub const SET_VIEW: u32 = 0x0045_1190;

/// The render thread's constructor's parts: the thread base
/// `0x0042fcc0`, its vtable-like word `0x00b31404`, the 16 queue entries
/// (20 bytes; `0x00613fc0`/`0x00614210`), the semaphores
/// (`0x0042fc90`), the 4 event flags (8 bytes; `0x004316d0`/`0x00431740`).
pub const THREAD_BASE: u32 = 0x0042_fcc0;
pub const THREAD_WORD: u32 = 0x00b3_1404;
pub const ENTRY_CTOR: u32 = 0x0061_3fc0;
pub const ENTRY_DTOR: u32 = 0x0061_4210;
pub const SEMAPHORE_CTOR: u32 = 0x0042_fc90;
pub const FLAG_CTOR: u32 = 0x0043_16d0;
pub const FLAG_DTOR: u32 = 0x0043_1740;

/// The render thread's loop: the profiler's thread name
/// (`0x0060ed70(name; "...", 0x2000)`, `0x0060ed40(list; name)`,
/// `0x0060ede0(list; name)` on `0x00c2e2cc`), the event flags' `wait`,
/// `reset` and the semaphore's `signal(sem; 1, 0)`, the commands
/// (`processRenderCommands(this; first, end)`) and the warning print
/// `0x004352f0(1, "...")` (cdecl).
pub const NAME_INIT: u32 = 0x0060_ed70;
pub const NAME_TEXT: u32 = 0x00b3_f100;
pub const NAME_LIST: u32 = 0x00c2_e2cc;
pub const NAME_ADD: u32 = 0x0060_ed40;
pub const NAME_REMOVE: u32 = 0x0060_ede0;
pub const EVENT_WAIT: u32 = 0x0043_02a0;
pub const EVENT_RESET: u32 = 0x0043_01e0;
pub const SEMAPHORE_SIGNAL: u32 = 0x0043_0210;
pub const PROCESS_COMMANDS: u32 = 0x0061_5550;
pub const PRINT: u32 = 0x0043_52f0;
pub const NOT_RUNNING: u32 = 0x00b3_f110;

/// The Iggy player's object: its constructor `0x00a3d3c0`, set-up
/// `0x00a3d900(this; 7 words)` and destructor `0x00a3d4d0(this; 0)`.
pub const PLAYER_CTOR: u32 = 0x00a3_d3c0;
pub const PLAYER_INIT: u32 = 0x00a3_d900;
pub const PLAYER_DTOR: u32 = 0x00a3_d4d0;

/// `PUtilityText`'s constructor: `PUtility`'s vtable, name and level,
/// the utility list `0x00c901a8` (by level `+0xc`), the class's vtable and
/// its seven render-pass slots.
pub const UTILITY_VTABLE: u32 = 0x00b0_e078;
pub const UTILITY_NAME: u32 = 0x00b7_ab50;
pub const UTILITY_LIST: u32 = 0x00c9_01a8;
pub const UTILITY_TEXT_VTABLE: u32 = 0x00b7_ab44;
pub const TEXT_PASSES: u32 = 0x01a8_58f4;
pub const TEXT_PASS_BLOCKS: u32 = 0x01a8_5920;

/// `0x005aa4b0`'s base destructor.
pub const NETWORK_BASE_DTOR: u32 = 0x0049_69f0;
/// `0x005ae0f0`'s tail call `0x005c1f90(packet)` (cdecl).
pub const PACKET_SUBMIT: u32 = 0x005c_1f90;

/// A word a constructor stores: a constant, or a float loaded from the
/// image (`fld dword [addr]`, `fstp`).
#[derive(Debug, Clone, Copy)]
pub enum Init {
    Word(u32, u32),
    Float(u32, u32),
    /// An image address (rebased), such as a vtable.
    Image(u32, u32),
}

/// `0x00620590`'s stores, in order.
pub const STORES_620590: &[Init] = &{
    let mut t = [Init::Word(0, 0); 46];
    let order: [u32; 46] = [
        0x0, 0x4, 0x8, 0xc, 0x10, 0x14, 0x9c, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c, 0x30, 0x34, 0x38,
        0x3c, 0x40, 0x44, 0x48, 0x4c, 0x50, 0x54, 0x58, 0x5c, 0x60, 0x64, 0x68, 0x6c, 0x70, 0x74,
        0x78, 0x7c, 0x80, 0x84, 0x88, 0x8c, 0x90, 0x94, 0x98, 0xa0, 0xa8, 0xac, 0xb0, 0xb4, 0xa4,
    ];
    let mut i = 0;
    while i < 46 {
        t[i] = Init::Word(order[i], if order[i] == 0xa4 { 0x10000 } else { 0 });
        i += 1;
    }
    t
};

/// `0x00620780`'s stores, in order (the floats from the image).
pub const STORES_620780: &[Init] = &[
    Init::Word(0x0, 0),
    Init::Word(0x4, 0),
    Init::Word(0x8, 0),
    Init::Float(0x10, 0x00b3_eb44),
    Init::Float(0x14, 0x00b3_eb44),
    Init::Float(0x18, 0x00b3_eb44),
    Init::Float(0x20, 0x00b4_1310),
    Init::Float(0x24, 0x00b4_131c),
    Init::Word(0x28, 0x3f80_0000),
    Init::Word(0x2c, 0),
    Init::Word(0x30, 0),
    Init::Word(0x34, 0),
    Init::Word(0x38, 0),
    Init::Float(0x80, 0x00b4_12e0),
    Init::Float(0x84, 0x00b4_12c8),
    Init::Float(0x88, 0x00b4_12cc),
    Init::Word(0x8c, 0),
    Init::Float(0x90, 0x00b4_1318),
    Init::Float(0x94, 0x00b4_1314),
    Init::Float(0x98, 0x00b4_12e4),
    Init::Word(0x9c, 0),
    Init::Word(0xa0, 0),
    Init::Word(0xa4, 0),
    Init::Word(0xa8, 0),
    Init::Word(0xac, 0),
    Init::Word(0xb0, 0),
    Init::Word(0xb4, 0),
    Init::Word(0xbc, 0),
    Init::Word(0xd0, 0),
    Init::Word(0xc0, 0),
    Init::Word(0xc4, 0),
    Init::Word(0xc8, 0),
    Init::Word(0xcc, 0),
];

/// A post effect's sampler state's constructor `0x00583b20(s)` and
/// release `0x0059c770(s)` (thiscall); the effect's release
/// `0x00495a10(effect)` (thiscall on `[slot]`).
pub const SAMPLER_CTOR: u32 = 0x0058_3b20;
pub const SAMPLER_RELEASE: u32 = 0x0059_c770;
pub const EFFECT_RELEASE: u32 = 0x0049_5a10;
/// The render target release `0x00a0e520(target)` (cdecl).
pub const TARGET_RELEASE: u32 = 0x00a0_e520;

/// A post effect's constructor: the `PPostEffectBase` header (`+4..+0xc`
/// 0, the half-word `+0x10` = 0x100) when `header`, the stores `pre`
/// (the vtable among them), the two sampler states at `samplers`
/// constructed, the stores `mid`, then each sampler's defaults (bytes
/// `+0`, `+1` = its index, `+2..+4` = 2) where its flags word (`+0x1c`)
/// has bit 7 clear. Returns `this`.
#[derive(Debug, Clone, Copy)]
pub struct PostCtor {
    pub header: bool,
    pub pre: &'static [Init],
    pub samplers: [u32; 2],
    pub mid: &'static [Init],
}

/// The post effects' constructors of this list.
pub const POST_CTORS: &[(u32, PostCtor)] = &[
    (
        0x009f_c160,
        PostCtor {
            header: true,
            pre: &[Init::Image(0, 0x00b7_5a88)],
            samplers: [0x14, 0x38],
            mid: &[
                Init::Word(0x5c, 0),
                Init::Word(0x60, 0),
                Init::Word(0x64, 0),
                Init::Word(0x68, 0),
                Init::Word(0x6c, 0),
            ],
        },
    ),
    (
        0x009f_c310,
        PostCtor {
            header: true,
            pre: &[Init::Image(0, 0x00b7_52e8)],
            samplers: [0x14, 0x38],
            mid: &[
                Init::Word(0x68, 0x3f80_0000),
                Init::Word(0x5c, 0),
                Init::Word(0x60, 0),
                Init::Float(0x6c, 0x00b2_607c),
                Init::Word(0x64, 3),
                Init::Word(0x74, 0),
                Init::Float(0x70, 0x00b3_eb98),
                Init::Word(0x78, 0),
                Init::Word(0x7c, 0),
                Init::Word(0x80, 0),
                Init::Word(0x84, 0),
                Init::Word(0x88, 0),
                Init::Word(0x8c, 0),
            ],
        },
    ),
    (
        0x009f_c540,
        PostCtor {
            header: false,
            pre: &[],
            samplers: [0, 0x24],
            mid: &[
                Init::Word(0x5c, 0x3f80_0000),
                Init::Word(0x48, 0),
                Init::Word(0x4c, 0),
                Init::Word(0x50, 3),
                Init::Word(0x60, 0),
                Init::Word(0x64, 0),
                Init::Word(0x6c, 0),
                Init::Word(0x70, 0),
                Init::Word(0x74, 0),
            ],
        },
    ),
    (
        0x009f_c7e0,
        PostCtor {
            header: true,
            pre: &[Init::Image(0, 0x00b7_4eb8)],
            samplers: [0x14, 0x38],
            mid: &[
                Init::Word(0x5c, 0),
                Init::Word(0x68, 0),
                Init::Word(0x6c, 0),
                Init::Word(0x70, 0),
                Init::Word(0x74, 0),
                Init::Word(0x78, 0),
                Init::Word(0x7c, 0),
                Init::Word(0x80, 0),
                Init::Word(0x84, 0),
                Init::Word(0x88, 0),
                Init::Word(0x8c, 0),
                Init::Word(0x60, 0),
                Init::Word(0x64, 0),
            ],
        },
    ),
    (
        0x009f_d080,
        PostCtor {
            header: true,
            pre: &[
                Init::Word(0x14, 0),
                Init::Word(0x18, 0x3f80_0000),
                Init::Image(0, 0x00b7_5d88),
                Init::Word(0x1c, 0),
            ],
            samplers: [0x20, 0x44],
            mid: &[
                Init::Word(0x68, 0),
                Init::Word(0x6c, 0),
                Init::Word(0x70, 0),
                Init::Word(0x74, 0),
                Init::Word(0x78, 0),
                Init::Word(0x7c, 0),
                Init::Word(0x80, 0),
                Init::Word(0x84, 0),
                Init::Word(0x88, 0),
                Init::Word(0x8c, 0),
                Init::Word(0x90, 0),
            ],
        },
    ),
    (
        0x009f_cbb0,
        PostCtor {
            header: true,
            pre: &[
                Init::Float(0xcd4, 0x00b9_2318),
                Init::Image(0, 0x00b7_4820),
                Init::Word(0xcd8, 0),
                Init::Word(0xcdc, 0),
            ],
            samplers: [0xce0, 0xd04],
            mid: MOTION_BLUR_MID,
        },
    ),
];

/// `PMotionBlurBase`'s constructor (`0x009fcbb0`) after its sampler
/// states: nine words zeroed at `+0xd28`, then identity matrices (`fld1`/
/// `fldz` stores) at `+0x14`, `+0x54` and 32 more from `+0xd4`.
pub const MOTION_BLUR_MID: &[Init] = &{
    let mut t = [Init::Word(0, 0); 9 + 34 * 16];
    let mut i = 0;
    while i < 9 {
        t[i] = Init::Word(0xd28 + 4 * i as u32, 0);
        i += 1;
    }
    let mut m = 0;
    while m < 34 {
        let base = if m < 2 {
            0x14 + 0x40 * m as u32
        } else {
            0xd4 + 0x40 * (m as u32 - 2)
        };
        let mut k = 0;
        while k < 16 {
            let v = if k % 5 == 0 { 0x3f80_0000 } else { 0 };
            t[9 + 16 * m + k] = Init::Word(base + 4 * k as u32, v);
            k += 1;
        }
        m += 1;
    }
    t
};

/// The constructor at `addr` ([`POST_CTORS`]).
#[must_use]
pub const fn post_ctor(addr: u32) -> PostCtor {
    let mut i = 0;
    while i < POST_CTORS.len() {
        if POST_CTORS[i].0 == addr {
            return POST_CTORS[i].1;
        }
        i += 1;
    }
    panic!("no such constructor")
}

/// A post effect's destructor: its vtable, the render targets at
/// `targets` released (`0x00a0e520`; `inline` ones the same, inlined in
/// the original), each sampler state released when its flags word
/// (`+0x1c`) has bit 7 set, the effect `[+effect]` released and cleared,
/// the two sampler states destroyed (`0x00585230`, in `members` order),
/// `PPostEffectBase`'s vtable. `eax`: the last destruction's.
#[derive(Debug, Clone, Copy)]
pub struct PostDtor {
    pub vtable: u32,
    pub targets: &'static [u32],
    pub inline: &'static [u32],
    pub samplers: Option<[u32; 2]>,
    pub effect: Option<u32>,
    pub members: [u32; 2],
}

/// The post effects' destructors of this list.
pub const POST_DTORS: &[(u32, PostDtor)] = &[
    (
        0x009f_dd30,
        PostDtor {
            vtable: 0x00b7_4488,
            targets: &[0xf2c, 0xf30, 0xf34],
            inline: &[],
            samplers: Some([0xf40, 0xf64]),
            effect: Some(0xf38),
            members: [0xf64, 0xf40],
        },
    ),
    (
        0x009f_df60,
        PostDtor {
            vtable: 0x00b7_5a88,
            targets: &[0x5c],
            inline: &[],
            samplers: Some([0x14, 0x38]),
            effect: Some(0x60),
            members: [0x38, 0x14],
        },
    ),
    (
        0x009f_e020,
        PostDtor {
            vtable: 0x00b7_52e8,
            targets: &[],
            inline: &[],
            samplers: Some([0x14, 0x38]),
            effect: Some(0x5c),
            members: [0x38, 0x14],
        },
    ),
    (
        0x009f_e2a0,
        PostDtor {
            vtable: 0x00b7_4eb8,
            targets: &[0x5c],
            inline: &[0x60, 0x64],
            samplers: None,
            effect: None,
            members: [0x38, 0x14],
        },
    ),
    (
        0x009f_e640,
        PostDtor {
            vtable: 0x00b7_4820,
            targets: &[0xcd8],
            inline: &[],
            samplers: Some([0xce0, 0xd04]),
            effect: Some(0xcdc),
            members: [0xd04, 0xce0],
        },
    ),
    (
        0x009f_e890,
        PostDtor {
            vtable: 0x00b7_5d88,
            targets: &[0x68, 0x6c, 0x70],
            inline: &[],
            samplers: Some([0x20, 0x44]),
            effect: Some(0x1c),
            members: [0x44, 0x20],
        },
    ),
];

/// The destructor at `addr` ([`POST_DTORS`]).
#[must_use]
pub const fn post_dtor(addr: u32) -> PostDtor {
    let mut i = 0;
    while i < POST_DTORS.len() {
        if POST_DTORS[i].0 == addr {
            return POST_DTORS[i].1;
        }
        i += 1;
    }
    panic!("no such destructor")
}

/// `0x009fc6f0`'s stores after its view set-up, in order.
pub const CAMERA_STORES: &[Init] = &[
    Init::Word(0x30, 0x3f80_0000),
    Init::Word(0x34, 0x3f80_0000),
    Init::Word(0x38, 0x3f80_0000),
    Init::Word(0x3c, 0x3f80_0000),
    Init::Word(0x40, 0),
    Init::Word(0x44, 0),
    Init::Word(0x48, 0),
    Init::Word(0x4c, 0),
    Init::Float(0x54, 0x00b1_f2c8),
    Init::Word(0x50, 0),
    Init::Word(0x64, 0),
    Init::Float(0x58, 0x00b3_04f8),
    Init::Word(0x68, 0),
    Init::Word(0x78, 0),
    Init::Word(0x5c, 0),
    Init::Word(0x60, 0x3f80_0000),
    Init::Word(0x6c, 0x3f80_0000),
    Init::Float(0x70, 0x00b1_7504),
    Init::Word(0x74, 0),
];

/// A resource's `unbind`: the bound word it clears, whether it checks its
/// owner (`+0x18`) first, its record count (thiscall), its `PArray` of
/// records (count, then data or the one record inline), the record size,
/// the COM objects it releases (slot 2) and the block it frees, in order.
#[derive(Debug, Clone, Copy)]
pub struct Unbind {
    pub clear: u32,
    pub owner: bool,
    pub count: u32,
    pub array: u32,
    pub stride: u32,
    pub release: &'static [u32],
    pub free: Option<u32>,
}

/// The `unbind`s: the structured buffer's, the texture kinds' (2D, cube,
/// 3D).
pub const UNBINDS: &[(u32, Unbind)] = &[
    (
        0x0059_ca60,
        Unbind {
            clear: 0x44,
            owner: false,
            count: 0x0059_3ba0,
            array: 0x14,
            stride: 20,
            release: &[4, 8, 0, 0xc],
            free: None,
        },
    ),
    (
        0x0059_cc40,
        Unbind {
            clear: 0x70,
            owner: true,
            count: 0x0059_3bb0,
            array: 0x24,
            stride: 28,
            release: &[8, 0xc, 0, 4, 0x14],
            free: Some(0x10),
        },
    ),
    (
        0x0059_cf00,
        Unbind {
            clear: 0x64,
            owner: true,
            count: 0x0059_3bc0,
            array: 0x28,
            stride: 20,
            release: &[4, 8, 0],
            free: Some(0xc),
        },
    ),
    (
        0x0059_d0f0,
        Unbind {
            clear: 0x64,
            owner: true,
            count: 0x0059_3bd0,
            array: 0x20,
            stride: 20,
            release: &[4, 8, 0],
            free: Some(0xc),
        },
    ),
];

/// The `unbind` at `addr` ([`UNBINDS`]).
#[must_use]
pub const fn unbind_of(addr: u32) -> Unbind {
    let mut i = 0;
    while i < UNBINDS.len() {
        if UNBINDS[i].0 == addr {
            return UNBINDS[i].1;
        }
        i += 1;
    }
    panic!("no such unbind")
}

/// The texture's owner check `0x0059c650(owner)` (thiscall on `+0x18`).
pub const UNBIND_OWNER: u32 = 0x0059_c650;

/// `0x005b5150`'s callees: `PDynamicMesh::getDataBlockCount(mesh; seg)`
/// (thiscall) and the descriptor's kind count `0x005b52b0(array)` (cdecl).
pub const DATA_BLOCK_COUNT: u32 = 0x0049_8770;
pub const COUNT_KINDS: u32 = 0x005b_52b0;
/// `0x005b53f0`'s lookups (thiscall; index): the cube and 3D textures'
/// record addresses.
pub const CUBE_RECORD: u32 = 0x005b_8840;
pub const VOLUME_RECORD: u32 = 0x005b_88e0;
/// The parameter kinds whose capture slot holds a texture.
pub const TEXTURE_KINDS: [u8; 8] = [0x42, 0xd9, 0xda, 0xdb, 0xdc, 0x12, 0x43, 0x44];
/// The callback command's opcode and the rings' allocator.
pub const CALLBACK_OPCODE: u32 = 0x2c;
pub const RING_ALLOCATE: u32 = 0x0061_3e10;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        CALLBACK_OPCODE, COUNT_KINDS, CUBE_RECORD, DATA_BLOCK_COUNT, RING_ALLOCATE, TEXTURE_KINDS,
        VOLUME_RECORD,
    };
    use super::{
        CAMERA_STORES, EFFECT_RELEASE, PostCtor, PostDtor, SAMPLER_CTOR, SAMPLER_RELEASE,
        TARGET_RELEASE,
    };
    use super::{
        CLUSTER_CREATE, CLUSTER_CREATE2, COPY, COPY2, CREATE_TARGET, Conv, DEFERRED_LIGHTING_PARTS,
        DEFERRED_LIGHTING_VTABLE, DEFERRED_MEMBERS, DEFERRED_PARTS, DEFERRED_VTABLE, ENTRY_CTOR,
        ENTRY_DTOR, EVENT_RESET, EVENT_WAIT, FLAG_CTOR, FLAG_DTOR, FREE, FROM_NODE,
        GLOW_BASE_SETUP, GLOW_EFFECT, GLOW_GPU_EFFECT, GLOW_TARGETS, Host, INVERSE, Init, MALLOC,
        MEMBER_DTOR, MEMSET, MULTIPLY, NAME_ADD, NAME_INIT, NAME_LIST, NAME_REMOVE, NAME_TEXT,
        NETWORK_BASE_DTOR, NOT_RUNNING, OWNER, PACKET_SUBMIT, PLAYER_CTOR, PLAYER_DTOR,
        PLAYER_INIT, POST_EFFECT_VTABLE, PRINT, PROCESS_COMMANDS, SEMAPHORE_CTOR, SEMAPHORE_SIGNAL,
        SET_DIMENSIONS, SET_VIEW, TARGET_FORMAT, TARGET_SET_TEXTURE, TARGET_TYPE, TEXT_PASS_BLOCKS,
        TEXT_PASSES, TEXTURE_TYPE, THREAD_BASE, THREAD_WORD, UPDATE_MAX_LEVEL, UTILITY_LIST,
        UTILITY_NAME, UTILITY_TEXT_VTABLE, UTILITY_VTABLE, VECTOR_CTOR, x87,
    };
    use super::{UNBIND_OWNER, Unbind};
    use crate::p6_encode;

    /// The records of a texture's `PArray` at `array` (count `+0` without
    /// bit 31; one record inline at `+4`, more through the pointer `+4`)
    /// as the originals read them for each access: (count, base).
    fn records(h: &mut impl Host, tex: u32, array: u32) -> (u32, u32) {
        let c = h.read::<u32>(tex.wrapping_add(array)) & 0x7fff_ffff;
        (c, base_of(h, tex, array, c))
    }

    fn base_of(h: &mut impl Host, tex: u32, array: u32, c: u32) -> u32 {
        match c {
            0 => 0,
            1 => tex.wrapping_add(array + 4),
            _ => h.read(tex.wrapping_add(array + 4)),
        }
    }

    /// A resource's `unbind` ([`Unbind`], thiscall): the bound word
    /// cleared (after the owner `+0x18` is read, where it has one); an owner's check
    /// `0x0059c650` error returned; then for each record (the count asked
    /// again after each) its COM objects released (`Release`, slot 2) and
    /// cleared, its block freed and cleared. 0.
    pub fn unbind(h: &mut impl Host, tex: u32, u: &Unbind) -> u32 {
        let owner: u32 = if u.owner { h.read(tex + 0x18) } else { 0 };
        h.write(tex.wrapping_add(u.clear), 0u32);
        if owner != 0 {
            let e = h.call_sym(UNBIND_OWNER, Conv::Thiscall(owner), &[]);
            if e != 0 {
                return e;
            }
        }
        let mut i = 0u32;
        let mut n = h.call_sym(u.count, Conv::Thiscall(tex), &[]);
        while i < n {
            let off = i.wrapping_mul(u.stride);
            for (k, &f) in u.release.iter().chain(u.free.iter()).enumerate() {
                let (c, base) = records(h, tex, u.array);
                if h.read::<u32>(base.wrapping_add(off).wrapping_add(f)) == 0 {
                    continue;
                }
                let base = base_of(h, tex, u.array, c);
                let obj: u32 = h.read(base.wrapping_add(off).wrapping_add(f));
                if k < u.release.len() {
                    let vt: u32 = h.read(obj);
                    let release: u32 = h.read(vt + 8);
                    h.call(release, Conv::Stdcall, &[obj]);
                } else {
                    h.call_sym(FREE, Conv::Cdecl, &[obj]);
                }
                let (_, base) = records(h, tex, u.array);
                h.write(base.wrapping_add(off).wrapping_add(f), 0u32);
            }
            i += 1;
            n = h.call_sym(u.count, Conv::Thiscall(tex), &[]);
        }
        0
    }

    /// A post effect's constructor ([`PostCtor`]) on `this` -> `this`.
    pub fn post_ctor(h: &mut impl Host, this: u32, c: &PostCtor) -> u32 {
        if c.header {
            for off in [4, 8, 0xc] {
                h.write(this + off, 0u32);
            }
            h.write(this + 0x10, 0x100u16);
        }
        stores(h, this, c.pre);
        for s in c.samplers {
            h.call_sym(SAMPLER_CTOR, Conv::Thiscall(this.wrapping_add(s)), &[]);
        }
        stores(h, this, c.mid);
        for (k, s) in c.samplers.into_iter().enumerate() {
            let state = this.wrapping_add(s);
            let first = u8::from(k == 1);
            for (i, v) in [first, first, 2, 2, 2].into_iter().enumerate() {
                let flags: u32 = h.read(state + 0x1c);
                if flags >> 7 & 1 == 0 {
                    h.write(state + i as u32, v);
                }
            }
        }
        this
    }

    /// A post effect's destructor ([`PostDtor`]) on `this`; `eax` the last
    /// member destruction's.
    pub fn post_dtor(h: &mut impl Host, this: u32, d: &PostDtor) -> u32 {
        let vt = h.rebase(d.vtable);
        h.write(this, vt);
        for &off in d.targets {
            let t: u32 = h.read(this.wrapping_add(off));
            h.call_sym(TARGET_RELEASE, Conv::Cdecl, &[t]);
        }
        for &off in d.inline {
            let t: u32 = h.read(this.wrapping_add(off));
            crate::p6_small::bridge::target_release(h, t);
        }
        if let Some(samplers) = d.samplers {
            for s in samplers {
                let state = this.wrapping_add(s);
                let flags: u32 = h.read(state + 0x1c);
                if flags >> 7 & 1 != 0 {
                    h.call_sym(SAMPLER_RELEASE, Conv::Thiscall(state), &[]);
                }
            }
        }
        if let Some(e) = d.effect {
            let slot = this.wrapping_add(e);
            let effect: u32 = h.read(slot);
            if effect != 0 {
                let inner: u32 = h.read(effect);
                h.call_sym(EFFECT_RELEASE, Conv::Thiscall(inner), &[]);
            }
            h.write(slot, 0u32);
        }
        let mut eax = 0;
        for m in d.members {
            eax = h.call_sym(MEMBER_DTOR, Conv::Thiscall(this.wrapping_add(m)), &[]);
        }
        let vt = h.rebase(POST_EFFECT_VTABLE);
        h.write(this, vt);
        eax
    }

    /// `0x009fc6f0(this)` (thiscall): an identity matrix in the frame
    /// (`frame`, the original's `ebp - 0x40`) handed to `0x00451190(this;
    /// m)`, then [`CAMERA_STORES`]. `this`.
    pub fn camera_defaults(h: &mut impl Host, this: u32, frame: u32) -> u32 {
        for k in 0..16u32 {
            let v = if k % 5 == 0 { 1.0f32 } else { 0.0 };
            h.write(frame + 4 * k, v.to_bits());
        }
        h.call_sym(SET_VIEW, Conv::Thiscall(this), &[frame]);
        stores(h, this, CAMERA_STORES)
    }

    /// `PDeferredLighting`'s destructor `0x009fdc00(this)` (thiscall): its
    /// vtable `0x00b74270`, its parts `0x00a24b00`, its 13 render targets
    /// (`+0x3f94..+0x3fc4`) released, then as the base: its vtable, parts,
    /// three members, `PPostEffectBase`'s vtable. `eax`: the last member's.
    pub fn deferred_dtor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(DEFERRED_LIGHTING_VTABLE);
        h.write(this, vt);
        h.call_sym(DEFERRED_LIGHTING_PARTS, Conv::Thiscall(this), &[]);
        for k in 0..13 {
            let t: u32 = h.read(this.wrapping_add(0x3f94 + 4 * k));
            h.call_sym(TARGET_RELEASE, Conv::Cdecl, &[t]);
        }
        let vt = h.rebase(DEFERRED_VTABLE);
        h.write(this, vt);
        h.call_sym(DEFERRED_PARTS, Conv::Thiscall(this), &[]);
        let mut eax = 0;
        for m in DEFERRED_MEMBERS {
            eax = h.call_sym(MEMBER_DTOR, Conv::Thiscall(this.wrapping_add(m)), &[]);
        }
        let vt = h.rebase(POST_EFFECT_VTABLE);
        h.write(this, vt);
        eax
    }

    /// `0x009fe790(obj)` (thiscall): the two render targets `+8`, `+0xc`
    /// released, the owned `PArray` (`+0` count, bit 31 a view; `+4`
    /// data, freed even when null) freed and cleared. `eax`: the free's,
    /// or 0xfe for a view.
    pub fn targets_array_dtor(h: &mut impl Host, obj: u32) -> u32 {
        for off in [8, 0xc] {
            let t: u32 = h.read(obj + off);
            h.call_sym(TARGET_RELEASE, Conv::Cdecl, &[t]);
        }
        let count: u32 = h.read(obj);
        let mut eax = 0xfe;
        if count >> 31 == 0 {
            let data: u32 = h.read(obj + 4);
            eax = h.call_sym(FREE, Conv::Cdecl, &[data]);
        }
        h.write(obj, 0u32);
        h.write(obj + 4, 0u32);
        eax
    }

    /// `[at] += v`, read and written in place (the originals' `add [m]`).
    fn add_to(h: &mut impl Host, at: u32, v: u32) {
        let x: u32 = h.read(at);
        h.write(at, x.wrapping_add(v));
    }

    /// `0x005b5150(total, blocks, table, flags, material, mesh, node,
    /// seg, instance)` (cdecl, 9 words): a mesh instance's buffer sizes.
    /// `*total` and `*blocks` start at 0; flag bit 1 adds the material
    /// size (`[material]` rounded to 16), bit 2 the mesh's `u16 +0x32`,
    /// bit 3 the node's `+0x1c`; a dynamic mesh (`[instance+4]`) adds its
    /// data blocks (0x40 each, rounded, 0x40 more when the segment's
    /// `+0xc` is set) to both; then the pointer table (`[node+0x14]`
    /// without bit 31, 2, both arrays' texture counts, 2 more for a
    /// dynamic mesh; 4 bytes each + 4, at least 8; rounded to 16) to
    /// `*table` and `*total`. `eax`: `table`.
    pub fn mesh_sizes(h: &mut impl Host, a: [u32; 9]) -> u32 {
        let [
            total,
            blocks,
            table,
            flags,
            material,
            mesh,
            node,
            seg,
            instance,
        ] = a;
        let dynamic = if instance == 0 {
            0
        } else {
            h.read(instance + 4)
        };
        h.write(total, 0u32);
        h.write(blocks, 0u32);
        if flags & 2 != 0 {
            let m: u32 = h.read(material);
            add_to(h, total, m.wrapping_add(0xf) & !0xf);
        }
        if flags & 4 != 0 {
            let m: u16 = h.read(mesh.wrapping_add(0x32));
            add_to(h, total, u32::from(m));
        }
        if flags & 8 != 0 {
            let m: u32 = h.read(node.wrapping_add(0x1c));
            add_to(h, total, m);
        }
        if dynamic != 0 {
            let n = h.call_sym(DATA_BLOCK_COUNT, Conv::Thiscall(dynamic), &[seg]);
            let segs: u32 = h.read(dynamic + 0x14);
            let extra: u32 = h.read(segs.wrapping_add(seg.wrapping_mul(16)).wrapping_add(0xc));
            let v = (n.wrapping_shl(6).wrapping_add(0xf) & !0xf).wrapping_add(if extra != 0 {
                0x40
            } else {
                0
            });
            h.write(blocks, v);
            add_to(h, total, v);
        }
        let k1 = h.call_sym(COUNT_KINDS, Conv::Cdecl, &[material.wrapping_add(8)]);
        let k2 = h.call_sym(COUNT_KINDS, Conv::Cdecl, &[node.wrapping_add(0xc)]);
        let n: u32 = h.read(node.wrapping_add(0x14));
        let mut t = (n & 0x7fff_ffff)
            .wrapping_add(2)
            .wrapping_add(k1.wrapping_add(k2));
        if dynamic != 0 {
            t = t.wrapping_add(2);
        }
        let size = if t > 1 {
            t.wrapping_mul(4).wrapping_add(4)
        } else {
            8
        };
        let size = size.wrapping_add(0xf) & !0xf;
        h.write(table, size);
        add_to(h, total, size);
        table
    }

    /// `0x005b53f0(base, params, out)` (cdecl): for each texture-kind
    /// parameter (16-byte entries at `[params+4]`, count `[params]`
    /// without bit 31) whose capture slot (`base + u16 +8`) holds a
    /// texture (`+0xc`), its record address stored at `out` (advancing):
    /// a 2D texture's `+0x18 + 28 x` the slot's index into its records
    /// (`+0x28`: inline for one, a pointer for more), the cube's and 3D's
    /// through their lookups. The cursor after the last.
    pub fn texture_pointers(h: &mut impl Host, base: u32, params: u32, mut out: u32) -> u32 {
        let n = h.read::<u32>(params) & 0x7fff_ffff;
        for k in 0..n {
            let table: u32 = h.read(params + 4);
            let e = table.wrapping_add(16 * k);
            let kind: u8 = h.read(e.wrapping_add(2));
            if !TEXTURE_KINDS.contains(&kind) {
                continue;
            }
            let off: u16 = h.read(e.wrapping_add(8));
            let slot = base.wrapping_add(u32::from(off));
            let ty: u32 = h.read(slot);
            let index: u32 = h.read(slot.wrapping_add(4));
            let tex: u32 = h.read(slot.wrapping_add(0xc));
            let v = match ty {
                0x42 if tex != 0 => {
                    let c = h.read::<u32>(tex + 0x24) & 0x7fff_ffff;
                    let records = match c {
                        0 => 0,
                        1 => tex + 0x28,
                        _ => h.read(tex + 0x28),
                    };
                    records
                        .wrapping_add(index.wrapping_mul(28))
                        .wrapping_add(0x18)
                }
                0x43 if tex != 0 => h.call_sym(CUBE_RECORD, Conv::Thiscall(tex), &[index]),
                0x44 if tex != 0 => h.call_sym(VOLUME_RECORD, Conv::Thiscall(tex), &[index]),
                _ => continue,
            };
            h.write(out, v);
            out = out.wrapping_add(4);
        }
        out
    }

    /// `0x005b5940(renderer; f, size, out)` (thiscall, `ret 0xc`): a
    /// callback command with a data block: 1 for no `f`; room for the
    /// block (`size` + 15 when set, rounded to 16) and a command as the
    /// encoders check it (`frame` the original's `ebp - 8`); the block
    /// (to `*out`, 0 without one; 0xd when the ring is full) and the
    /// command (0xd); `*out` rounded up to 16; the command `0x2c {f,
    /// *out}`. 0.
    pub fn queue_data_callback(
        h: &mut impl Host,
        renderer: u32,
        frame: u32,
        f: u32,
        size: u32,
        out: u32,
    ) -> u32 {
        if f == 0 {
            return 1;
        }
        let size = size.wrapping_add(if size != 0 { 0xf } else { 0 });
        let aligned = size.wrapping_add(0xf) & !0xf;
        h.write(out, 0u32);
        if p6_encode::bridge::full(h, renderer + 0x1c, aligned, frame + 4)
            || p6_encode::bridge::full(h, renderer + 0x30, p6_encode::COMMAND_SIZE, frame)
        {
            h.call_virtual(renderer, 2, &[]);
        }
        if size != 0 {
            let block = h.call_sym(RING_ALLOCATE, Conv::Thiscall(renderer + 0x1c), &[aligned]);
            h.write(out, block);
            if block == 0 {
                return 0xd;
            }
        }
        let cmd = h.call_sym(
            RING_ALLOCATE,
            Conv::Thiscall(renderer + 0x30),
            &[p6_encode::COMMAND_SIZE],
        );
        if cmd == 0 {
            return 0xd;
        }
        if h.read::<u32>(renderer + 0x4c) == 0 {
            h.write(renderer + 0x4c, cmd);
        }
        h.write(renderer + 0x50, cmd.wrapping_add(p6_encode::COMMAND_SIZE));
        let o: u32 = h.read(out);
        add_to(h, out, o.wrapping_neg() & 0xf);
        h.write(cmd + 8, f);
        h.write(cmd, CALLBACK_OPCODE);
        let o: u32 = h.read(out);
        h.write(cmd + 0xc, o);
        0
    }

    /// A constructor's stores ([`Init`]) on `obj`; returns `obj`.
    pub fn stores(h: &mut impl Host, obj: u32, table: &[Init]) -> u32 {
        for &s in table {
            match s {
                Init::Word(off, v) => h.write(obj.wrapping_add(off), v),
                Init::Float(off, at) => {
                    let v: f32 = h.read(h.rebase(at));
                    h.write(obj.wrapping_add(off), x87::round_trip(v));
                }
                Init::Image(off, at) => {
                    let v = h.rebase(at);
                    h.write(obj.wrapping_add(off), v);
                }
            }
        }
        obj
    }

    /// A 2D render target of `w` x `h` in `cluster`: the texture
    /// (`PCluster::create`), its size (an error gives 0), its flags (bit 1
    /// of `+0x14` and the byte `+5` cleared), `updateMaxLevel`, the target
    /// (`0x0064adf0`) marked (byte `+5` 0, owner `"PPostProcessUtils"`).
    /// The target, or 0.
    fn create_target(h: &mut impl Host, cluster: u32, w: u32, ht: u32) -> u32 {
        let ty = h.rebase(TEXTURE_TYPE);
        let tex = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, ty, 0]);
        if tex == 0 {
            return 0;
        }
        let format = h.rebase(TARGET_FORMAT);
        if h.call_sym(SET_DIMENSIONS, Conv::Thiscall(tex), &[w, ht, format, 0]) != 0 {
            return 0;
        }
        let f: u32 = h.read(tex + 0x14);
        h.write(tex + 0x14, f & !2);
        h.write(tex + 5, 0u8);
        h.call_sym(UPDATE_MAX_LEVEL, Conv::Thiscall(tex), &[]);
        let rt = h.call_sym(CREATE_TARGET, Conv::Thiscall(cluster), &[tex]);
        if rt != 0 {
            h.write(rt + 5, 0u8);
            let owner = h.rebase(OWNER);
            h.write(rt + 0xc, owner);
        }
        rt
    }

    /// `0x00a1a2c0(this; cluster, w, h)` (thiscall, `ret 0xc`): the glow
    /// target (`+0x5c`; 0xd without one) and the GPU glow's effect
    /// (`+0x60`, `0x00a06120(cluster)`). 0.
    pub fn glow_gpu_setup(h: &mut impl Host, this: u32, cluster: u32, w: u32, ht: u32) -> u32 {
        let rt = create_target(h, cluster, w, ht);
        h.write(this + 0x5c, rt);
        if rt == 0 {
            return 0xd;
        }
        let fx = h.call_sym(GLOW_GPU_EFFECT, Conv::Cdecl, &[cluster]);
        h.write(this + 0x60, fx);
        0
    }

    /// `0x00a176d0(this; cluster, w, h)` (thiscall, `ret 0xc`): the base
    /// set-up (`0x00a1a190`, its error returned), the glow targets
    /// (`0x00a0ff50(w, h, 7, 1.0)`), a render target at `+0xfb0`; 0xd
    /// without it, else the targets' result.
    pub fn glow_setup(h: &mut impl Host, this: u32, cluster: u32, w: u32, ht: u32) -> u32 {
        let r = h.call_sym(GLOW_BASE_SETUP, Conv::Thiscall(this), &[cluster, w, ht]);
        if r != 0 {
            return r;
        }
        let r = h.call_sym(
            GLOW_TARGETS,
            Conv::Thiscall(this),
            &[w, ht, 7, 1.0f32.to_bits()],
        );
        let rt = create_target(h, cluster, w, ht);
        h.write(this + 0xfb0, rt);
        if rt == 0 { 0xd } else { r }
    }

    /// `0x00a1a190(this; cluster, w, h)` (thiscall, `ret 0xc`): three
    /// half-size render targets (`+0xf2c..+0xf34`): each a texture
    /// (`w / 2` x `h / 2`) bound to a new render target object's `+0x34`
    /// (`0x00440b70`, `0x00583a60`; marked as the others); 0xd at the
    /// first missing; then the glow effect at `+0xf38`. 0.
    pub fn glow_base_setup(h: &mut impl Host, this: u32, cluster: u32, w: u32, ht: u32) -> u32 {
        for k in 0..3 {
            let rt = half_target(h, cluster, w >> 1, ht >> 1);
            h.write(this + 0xf2c + 4 * k, rt);
            if rt == 0 {
                return 0xd;
            }
        }
        let fx = h.call_sym(GLOW_EFFECT, Conv::Cdecl, &[cluster]);
        h.write(this + 0xf38, fx);
        0
    }

    /// A half-size render target (`0x00a1a190`'s and `0x00a17bd0`'s): a
    /// `w` x `ht` texture bound to a new render target object; 0 at the
    /// first failure.
    pub fn half_target(h: &mut impl Host, cluster: u32, w: u32, ht: u32) -> u32 {
        let ty = h.rebase(TEXTURE_TYPE);
        let tex = h.call_sym(CLUSTER_CREATE, Conv::Thiscall(cluster), &[1, ty, 0]);
        if tex == 0 {
            return 0;
        }
        let format = h.rebase(TARGET_FORMAT);
        if h.call_sym(SET_DIMENSIONS, Conv::Thiscall(tex), &[w, ht, format, 0]) != 0 {
            return 0;
        }
        let f: u32 = h.read(tex + 0x14);
        h.write(tex + 0x14, f & !2);
        h.write(tex + 5, 0u8);
        h.call_sym(UPDATE_MAX_LEVEL, Conv::Thiscall(tex), &[]);
        let ty = h.rebase(TARGET_TYPE);
        let obj = h.call_sym(CLUSTER_CREATE2, Conv::Thiscall(cluster), &[1, ty, 0]);
        if obj == 0 {
            return 0;
        }
        let rt: u32 = h.read(obj + 0x34);
        if rt == 0 {
            return 0;
        }
        h.call_sym(TARGET_SET_TEXTURE, Conv::Thiscall(rt), &[tex]);
        h.write(rt + 5, 0u8);
        let owner = h.rebase(OWNER);
        h.write(rt + 0xc, owner);
        rt
    }

    /// `PDeferredLightingBase`'s deleting destructor (`this; flags`, `ret
    /// 4`): its vtable, its parts (`0x00a24a60`), three members
    /// (`0x00585230`), `PPostEffectBase`'s vtable, freed for flag bit 0.
    /// `this`.
    pub fn deferred_deleting_dtor(h: &mut impl Host, this: u32, flags: u32) -> u32 {
        let vt = h.rebase(DEFERRED_VTABLE);
        h.write(this, vt);
        h.call_sym(DEFERRED_PARTS, Conv::Thiscall(this), &[]);
        for m in DEFERRED_MEMBERS {
            h.call_sym(MEMBER_DTOR, Conv::Thiscall(this.wrapping_add(m)), &[]);
        }
        let vt = h.rebase(POST_EFFECT_VTABLE);
        h.write(this, vt);
        if flags & 1 != 0 {
            h.call_sym(FREE, Conv::Cdecl, &[this]);
        }
        this
    }

    /// `PRendererBase::setCamera(this; camera)` (`0x005babd0`, `ret 4`):
    /// the camera at `+0x5c`; its world matrix (its parent's `+0xb0`
    /// node's, or its own inverted) into `+0x60`, the inverse of that
    /// into `+0xa0..+0xdc` (`fld`/`fstp` each), `+0x58` cleared. The
    /// matrices live in the frame (`frame`, the original's `ebp - 0xc0`:
    /// C, B, A of 64 bytes). `eax`: the last copy's.
    pub fn set_camera(h: &mut impl Host, this: u32, frame: u32, camera: u32) -> u32 {
        let (c, b, a) = (frame, frame + 0x40, frame + 0x80);
        let parent: u32 = h.read(camera + 0xb0);
        h.write(this + 0x5c, camera);
        let src = if parent != 0 {
            h.call_sym(FROM_NODE, Conv::Thiscall(a), &[parent]);
            a
        } else {
            h.call_sym(FROM_NODE, Conv::Thiscall(a), &[camera]);
            let r = h.call_sym(INVERSE, Conv::Cdecl, &[c, a]);
            h.call_sym(COPY, Conv::Thiscall(b), &[r]);
            b
        };
        let view = this + 0x60;
        h.call_sym(COPY2, Conv::Thiscall(view), &[src]);
        let r = h.call_sym(INVERSE, Conv::Cdecl, &[c, view]);
        let eax = h.call_sym(COPY, Conv::Thiscall(a), &[r]);
        for k in 0..16 {
            let v: f32 = h.read(a + 4 * k);
            h.write(this + 0xa0 + 4 * k, x87::round_trip(v));
            if k == 0 {
                h.write(this + 0x58, 0u32);
            }
        }
        eax
    }

    /// `PCamera::updateViewMatrices(camera)` (`0x006720c0`): the world
    /// matrix (identity, or the node `+0xb0`'s), its inverse, the view
    /// (`inverse x +0x30`) into `+0x70`, the camera's view set from the
    /// inverse (`0x00451190`). The matrices in the frame (`frame`, the
    /// original's `ebp - 0xc0`: C, B, A). `eax`: the last call's.
    pub fn update_view_matrices(h: &mut impl Host, camera: u32, frame: u32) -> u32 {
        let (c, b, a) = (frame, frame + 0x40, frame + 0x80);
        for k in 0..16u32 {
            let v = if k % 5 == 0 { 1.0f32 } else { 0.0 };
            h.write(a + 4 * k, v.to_bits());
        }
        let node: u32 = h.read(camera + 0xb0);
        let src = if node != 0 {
            h.call_sym(FROM_NODE, Conv::Thiscall(a), &[node]);
            a
        } else {
            for k in 0..16 {
                let w: u32 = h.read(a + 4 * k);
                h.write(b + 4 * k, w);
            }
            b
        };
        for k in 0..16 {
            let w: u32 = h.read(src + 4 * k);
            h.write(c + 4 * k, w);
        }
        let r = h.call_sym(INVERSE, Conv::Cdecl, &[b, c]);
        h.call_sym(COPY, Conv::Thiscall(a), &[r]);
        let r = h.call_sym(MULTIPLY, Conv::Cdecl, &[c, camera + 0x30, a]);
        h.call_sym(COPY2, Conv::Thiscall(camera + 0x70), &[r]);
        h.call_sym(SET_VIEW, Conv::Thiscall(camera), &[a])
    }

    /// `PRenderThread::PRenderThread(this)` (`0x00614010`) -> `this`: the
    /// thread base, its fields (`+4` byte 0, `+8`, `+0xc` 0), the 16
    /// queue entries from `+0x10`, `+0x150..+0x158` zero, the two
    /// semaphores (`+0x15c`, `+0x164`, their bytes after), the 4 event
    /// flags from `+0x16c`, the mask `+0x18c` and `+0x208` zero, 0x78
    /// bytes from `+0x190` zeroed.
    pub fn thread_ctor(h: &mut impl Host, this: u32) -> u32 {
        h.call_sym(THREAD_BASE, Conv::Thiscall(this), &[]);
        h.write(this + 4, 0u8);
        let w = h.rebase(THREAD_WORD);
        h.write(this + 8, w);
        h.write(this + 0xc, 0u32);
        let (c, d) = (h.rebase(ENTRY_CTOR), h.rebase(ENTRY_DTOR));
        h.call_sym(VECTOR_CTOR, Conv::Stdcall, &[this + 0x10, 0x14, 0x10, c, d]);
        for off in [0x150, 0x154, 0x158] {
            h.write(this + off, 0u32);
        }
        h.call_sym(SEMAPHORE_CTOR, Conv::Thiscall(this + 0x15c), &[]);
        h.write(this + 0x160, 0u8);
        h.call_sym(SEMAPHORE_CTOR, Conv::Thiscall(this + 0x164), &[]);
        let (c, d) = (h.rebase(FLAG_CTOR), h.rebase(FLAG_DTOR));
        h.write(this + 0x168, 0u8);
        h.call_sym(VECTOR_CTOR, Conv::Stdcall, &[this + 0x16c, 8, 4, c, d]);
        h.write(this + 0x18c, 0u32);
        h.write(this + 0x208, 0u32);
        h.call_sym(MEMSET, Conv::Cdecl, &[this + 0x190, 0, 0x78]);
        this
    }

    /// `PRenderThread::threadEntry(this)` (`0x00616010`): the render
    /// thread's loop. Its profiler name (`this + 8`) registered; then per
    /// queue entry (`+0x150` of 16, 20 bytes from `+0x10`): its flag
    /// (`+0x18`) waited on, then by the state `+0xc`: 1 runs the entry's
    /// commands (`processRenderCommands(first, end)`, the entry cleared,
    /// its semaphore `+0x20` signalled when set), 0 prints the warning,
    /// 2 stops after this round; the queue semaphore `+0x15c` signalled
    /// each round. The name removed; 0.
    pub fn thread_entry(h: &mut impl Host, this: u32) -> u32 {
        let name = this + 8;
        let text = h.rebase(NAME_TEXT);
        h.call_sym(NAME_INIT, Conv::Thiscall(name), &[text, 0x2000]);
        let list = h.rebase(NAME_LIST);
        h.call_sym(NAME_ADD, Conv::Thiscall(list), &[name]);
        let mut stop = false;
        while !stop {
            let i: u32 = h.read(this + 0x150);
            let entry = this.wrapping_add(i.wrapping_mul(0x14));
            let next = i.wrapping_add(1);
            h.write(this + 0x150, if next == 0x10 { 0 } else { next });
            h.call_sym(EVENT_WAIT, Conv::Thiscall(entry + 0x18), &[]);
            match h.read::<u32>(this + 0xc) {
                0 => {
                    h.call_sym(EVENT_RESET, Conv::Thiscall(entry + 0x18), &[]);
                    let msg = h.rebase(NOT_RUNNING);
                    h.call_sym(PRINT, Conv::Cdecl, &[1, msg]);
                }
                1 => {
                    h.call_sym(EVENT_RESET, Conv::Thiscall(entry + 0x18), &[]);
                    let end: u32 = h.read(entry + 0x14);
                    let first: u32 = h.read(entry + 0x10);
                    h.call_sym(PROCESS_COMMANDS, Conv::Thiscall(this), &[first, end]);
                    let sem: u32 = h.read(entry + 0x20);
                    h.write(entry + 0x20, 0u32);
                    h.write(entry + 0x14, 0u32);
                    h.write(entry + 0x10, 0u32);
                    if sem != 0 {
                        h.call_sym(SEMAPHORE_SIGNAL, Conv::Thiscall(sem), &[1, 0]);
                    }
                }
                2 => stop = true,
                _ => {}
            }
            h.call_sym(SEMAPHORE_SIGNAL, Conv::Thiscall(this + 0x15c), &[1, 0]);
        }
        h.call_sym(NAME_REMOVE, Conv::Thiscall(list), &[name]);
        0
    }

    /// `0x00a3e4f0(7 words)` (cdecl): a 0x2c-byte Iggy player object
    /// (`PhyreMalloc`, constructed), set up with the words; on a set-up
    /// error destroyed and freed, 0. The object, or 0.
    pub fn player_create(h: &mut impl Host, args: [u32; 7]) -> u32 {
        let p = h.call_sym(MALLOC, Conv::Cdecl, &[0x2c]);
        if p == 0 {
            return 0;
        }
        h.call_sym(PLAYER_CTOR, Conv::Thiscall(p), &[]);
        if h.call_sym(PLAYER_INIT, Conv::Thiscall(p), &args) != 0 {
            h.call_sym(PLAYER_DTOR, Conv::Thiscall(p), &[]);
            h.call_sym(FREE, Conv::Cdecl, &[p]);
            return 0;
        }
        p
    }

    /// `PUtilityText::PUtilityText(this)` (`0x00a3e730`, `ecx`) ->
    /// `this`: as a `PUtility` (vtable, name, level 0xff) linked into the
    /// utility list before the first of level 0xff or more; the class's
    /// vtable; its seven render-pass slots pointed at their blocks.
    pub fn utility_text_ctor(h: &mut impl Host, this: u32) -> u32 {
        let vt = h.rebase(UTILITY_VTABLE);
        h.write(this, vt);
        let name = h.rebase(UTILITY_NAME);
        h.write(this + 8, name);
        h.write(this + 0xc, 0xffu32);
        let mut link = h.rebase(UTILITY_LIST);
        if h.read::<u32>(link) != 0 {
            loop {
                let u: u32 = h.read(link);
                if h.read::<u32>(u + 0xc) >= 0xff {
                    break;
                }
                let next: u32 = h.read(u + 4);
                link = u + 4;
                if next == 0 {
                    break;
                }
            }
        }
        let first: u32 = h.read(link);
        h.write(this + 4, first);
        h.write(link, this);
        let vt = h.rebase(UTILITY_TEXT_VTABLE);
        h.write(this, vt);
        let slots = h.rebase(TEXT_PASSES);
        let blocks = h.rebase(TEXT_PASS_BLOCKS);
        for k in 0..7 {
            h.write(slots + 4 * k, blocks + 0x10 * k);
        }
        this
    }

    /// `0x005aa4b0(net)` (thiscall): four owned `PArray`s (count, data at
    /// `+0x34`, `+0x2c`, `+0x24`, `+0x1c`; bit 31 a view) freed (the last
    /// even when null) and cleared, then the base destructor
    /// (`0x004969f0`); its `eax`.
    pub fn network_dtor(h: &mut impl Host, net: u32) -> u32 {
        for (count, null_ok) in [(0x34, false), (0x2c, false), (0x24, false), (0x1c, true)] {
            let c: u32 = h.read(net + count);
            if c >> 31 == 0 {
                let data: u32 = h.read(net + count + 4);
                if data != 0 || null_ok {
                    h.call_sym(FREE, Conv::Cdecl, &[data]);
                }
            }
            h.write(net + count + 4, 0u32);
            h.write(net + count, 0u32);
        }
        h.call_sym(NETWORK_BASE_DTOR, Conv::Thiscall(net), &[])
    }

    /// `0x005ae0f0(this; i, packet, table, _)` (thiscall, `ret 0x10`):
    /// segment `i`'s packet (`[+0x38] + 16 i`; 0 for none) filled: its
    /// list reset (`+0x28` at `+0x24`, `+0x2c` 0), for each input
    /// (`[seg]+0x18` without bit 31; index `[+0x30][seg+8 + k]`, skipped
    /// when negative) the table entry's `+0x3c` (0 for none) at
    /// `+0x10 + u16 +0x22 + 4 k`; the half-word `+0x1e` = 4 x `[+0x10]`'s
    /// `u16 +0xc`; `+0x14` = `[+0x20] + 4 x [seg+4]`; `+0x24` = the
    /// segment info's `+8` (`[+0x14] + 16 i`); then `0x005c1f90(packet)`'s
    /// result.
    pub fn fill_packet(h: &mut impl Host, this: u32, i: u32, packet: u32, table: u32) -> u32 {
        let seg = h.read::<u32>(this + 0x38).wrapping_add(i << 4);
        let info = h.read::<u32>(this + 0x14).wrapping_add(i << 4);
        let first: u32 = h.read(seg);
        if first == 0 {
            return 0;
        }
        let base = packet.wrapping_add(0x10);
        h.write(base + 0x18, base + 0x14);
        let p: u32 = h.read(base);
        let off: u16 = h.read(base + 0x12);
        let dst = base.wrapping_add(u32::from(off));
        h.write(base + 0x1c, 0u32);
        let n = h.read::<u32>(first + 0x18) & 0x7fff_ffff;
        for k in 0..n {
            let at: u32 = h.read(seg + 8);
            let ids: u32 = h.read(this + 0x30);
            let idx: i32 = h.read(ids.wrapping_add(at.wrapping_add(k).wrapping_mul(4)));
            if idx >= 0 {
                let obj: u32 = h.read(table.wrapping_add((idx as u32).wrapping_mul(4)));
                let v = if obj == 0 { 0 } else { h.read(obj + 0x3c) };
                h.write(dst.wrapping_add(k.wrapping_mul(4)), v);
            }
        }
        let c: u16 = h.read(p + 0xc);
        h.write(base + 0xe, c.wrapping_shl(2));
        let s4: u32 = h.read(seg + 4);
        let w20: u32 = h.read(this + 0x20);
        h.write(packet + 0x14, w20.wrapping_add(s4.wrapping_mul(4)));
        let w: u32 = h.read(info + 8);
        h.write(base + 0x14, w);
        h.call_sym(PACKET_SUBMIT, Conv::Cdecl, &[packet])
    }
}
