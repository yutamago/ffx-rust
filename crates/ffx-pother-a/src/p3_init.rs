//! Start-up code around the `Bind` tables ([`crate::p3_bind`]): the enum
//! bind descriptors, the subsystems' bind lists and the scripting
//! initialisation (Lua's standard libraries).

use crate::{cdecl, thiscall};
use ffx_graphic::host::Host;

/// `Phyre_Quad_init_00570830` (thiscall `(descriptor, name)`): the enum
/// descriptor's name and three zero words.
pub const ENUM_INIT: u32 = 0x0057_0830;
/// `Phyre_PEnumDescriptor_setTokens` (thiscall `(descriptor, tokens,
/// count)`).
pub const ENUM_SET_TOKENS: u32 = 0x0057_0a60;
/// `Phyre_FindNamed_00436350` (cdecl `(name)`): the entry of that name in
/// the list at [`ENUM_LIST`], or 0.
pub const FIND_NAMED: u32 = 0x0043_6350;
/// The head of the named-descriptor list `{name, next}` `FIND_NAMED`
/// walks.
pub const ENUM_LIST: u32 = 0x00cb_3358;

/// An enum's bind descriptor function (cdecl `()`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumBind {
    /// The guard word (bit 0: the descriptor is constructed).
    pub guard: u32,
    /// The descriptor `{name, next, ...}`.
    pub descriptor: u32,
    /// Its name string.
    pub name: u32,
    /// The token table and its count.
    pub tokens: u32,
    pub count: u32,
}

/// `0x005f_09a0`
/// `Phyre_PPhysics_PPhysicsRigidBodyBase_PCollisionGroupBindDescriptor`.
pub const COLLISION_GROUP: EnumBind = EnumBind {
    guard: 0x00cc_0bd8,
    descriptor: 0x00cc_0bc8,
    name: 0x00b3_bdf0,
    tokens: 0x00b3_be00,
    count: 0xf,
};

/// `0x005f_0a40`
/// `Phyre_PPhysics_PPhysicsRigidBodyBase_PRigidBodyTypeBindDescriptor`.
pub const RIGID_BODY_TYPE: EnumBind = EnumBind {
    guard: 0x00cc_0bec,
    descriptor: 0x00cc_0bdc,
    name: 0x00b3_c01c,
    tokens: 0x00b3_c02c,
    count: 3,
};

/// One call of a bind list, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Init {
    /// `f()` (cdecl), its `eax` unused.
    Call(u32),
    /// `RegisterType(getter())`.
    RegisterGot(u32),
}

/// A subsystem's bind list (cdecl `()`): its `Bind`s in order, then its
/// entries registered one by one until one fails (non-zero), then the
/// tail call when there is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindList {
    pub calls: &'static [Init],
    /// The entry registrar (cdecl `(entry)`, 0 on success).
    pub register: u32,
    pub entries: &'static [u32],
    /// Tail-jumped to (cdecl `()`) when every entry registered.
    pub tail: Option<u32>,
}

/// `Phyre_PTypeDispenser_RegisterType` (cdecl `(descriptor)`).
pub const REGISTER_TYPE: u32 = 0x0043_95b0;

/// `0x0043_3020`: the animation classes' binds, the animation enums
/// (`Phyre_PTypeEnum_registerEntry_00507ad0`), then the network instance's
/// thread pool.
pub const ANIMATION_BINDS: BindList = BindList {
    calls: &[
        Init::Call(0x0054_0750),
        Init::Call(0x0053_f510),
        Init::Call(0x0052_ee70),
        Init::Call(0x0051_0450),
        Init::Call(0x0051_05d0),
        Init::Call(0x0051_06b0),
        Init::Call(0x0052_bcb0),
        Init::Call(0x0050_e120),
        Init::Call(0x0050_d2f0),
        Init::Call(0x0050_b9a0),
        Init::Call(0x0050_9c50),
        Init::Call(0x0051_3c40),
        Init::Call(0x0050_f330),
        Init::Call(0x0052_5440),
        Init::Call(0x0052_3e00),
        Init::Call(0x0053_5bf0),
        Init::Call(0x0051_f470),
        Init::Call(0x0052_4770),
        Init::Call(0x0051_f550),
        Init::Call(0x0052_1300),
        Init::RegisterGot(0x0050_78c0),
        Init::Call(0x0053_5e10),
        Init::Call(0x0053_6210),
        Init::Call(0x0051_9de0),
        Init::Call(0x0050_8450),
        Init::Call(0x0052_8330),
        Init::Call(0x0052_8530),
        Init::Call(0x0050_8470),
        Init::Call(0x0053_01b0),
        Init::Call(0x0053_0f50),
        Init::Call(0x0052_d9e0),
        Init::Call(0x0052_1bb0),
        Init::Call(0x0052_2700),
        Init::Call(0x0052_3340),
    ],
    register: 0x0050_7ad0,
    entries: &[
        0x00c1_a924,
        0x00c1_a934,
        0x00c1_a944,
        0x00c1_a954,
        0x00c1_a964,
        0x00c1_a974,
        0x00c1_a984,
        0x00c1_a994,
        0x00c1_a9a4,
        0x00c1_a9b4,
        0x00c1_a9c4,
        0x00c1_a9d4,
        0x00c1_a9e4,
        0x00c1_a9f4,
        0x00c1_aa04,
        0x00c1_aa14,
        0x00c1_aa24,
        0x00c1_aa34,
        0x00c1_aa44,
    ],
    tail: Some(0x0053_9640),
};

/// `0x0050_3ee0`: the dynamic geometry classes' binds and their named
/// modifiers (`Phyre_PDynamicGeometry_addNamed_005c8660`).
pub const DYNAMIC_GEOMETRY_BINDS: BindList = BindList {
    calls: &[
        Init::RegisterGot(0x005c_7fe0),
        Init::Call(0x005c_b420),
        Init::Call(0x005c_17c0),
        Init::Call(0x005c_0eb0),
        Init::Call(0x005c_0790),
        Init::Call(0x005c_1430),
        Init::Call(0x005c_1560),
        Init::Call(0x005c_1390),
        Init::Call(0x005c_1080),
        Init::Call(0x005c_0970),
        Init::Call(0x005c_16e0),
        Init::Call(0x005b_1600),
        Init::Call(0x005b_16e0),
        Init::Call(0x005b_1400),
        Init::Call(0x005a_b400),
        Init::Call(0x005a_b0e0),
    ],
    register: 0x005c_8660,
    entries: &[
        0x00cb_d428,
        0x00cb_d4a8,
        0x00cb_d528,
        0x00cb_d5a8,
        0x00cb_d628,
        0x00cb_d6a8,
        0x00cb_d728,
        0x00cb_d7a8,
        0x00cb_d828,
        0x00c2_9498,
        0x00c2_94d8,
        0x00c2_9518,
        0x00c2_9558,
        0x00c2_9598,
    ],
    tail: None,
};

/// The scripting library's binds `0x005d_6960` calls first:
/// `PScript`, `PScheduler`, `PAsyncProcessHeader`, `0x006195a0`.
pub const SCRIPTING_BINDS: [u32; 4] = [0x005d_49d0, 0x005c_f700, 0x0061_7960, 0x0061_95a0];
/// The Lua library array (`{count | 0x80000000 when not owned, data}`)
/// and the `{name, luaopen}` pairs `0x005d_6960` pushes into it.
pub const LUA_LIBS: u32 = 0x00cb_e6d8;
pub const LUA_LIB_PAIRS: [(u32, u32); 4] = [
    (0x00b3_9320, 0x0095_4080), // "math"
    (0x00b3_9328, 0x0095_3640), // "bit32"
    (0x00b3_9330, 0x0095_2290), // "string"
    (0x00b3_9338, 0x0095_0e00), // "coroutine"
];
/// `Phyre_PhyreMalloc_0042fb80` (cdecl `(size, align)`).
pub const PHYRE_MALLOC: u32 = 0x0042_fb80;
/// `Phyre_PArray_pushInto8_005d6a60` (thiscall `(array, block, &element)`).
pub const PUSH_INTO8: u32 = 0x005d_6a60;
/// The bytes of `0x005d_6960`'s frame: the pair at `[ebp-8]`.
pub const LUA_FRAME: u32 = 8;

pub mod bridge {
    //! The originals over game memory.

    use super::{
        BindList, ENUM_INIT, ENUM_LIST, ENUM_SET_TOKENS, EnumBind, FIND_NAMED, Host, Init,
        LUA_LIB_PAIRS, LUA_LIBS, PHYRE_MALLOC, PUSH_INTO8, REGISTER_TYPE, SCRIPTING_BINDS, cdecl,
        thiscall,
    };

    /// An enum bind descriptor function: the descriptor constructed once,
    /// its tokens set, and, when it has a name no listed descriptor has,
    /// pushed at the head of [`ENUM_LIST`]. `eax`: 0 without a name, the
    /// namesake found, else the old head.
    pub fn enum_bind(h: &mut impl Host, e: &EnumBind) -> u32 {
        let guard = h.rebase(e.guard);
        let d = h.rebase(e.descriptor);
        let w: u32 = h.read(guard);
        if w & 1 == 0 {
            h.write(guard, w | 1);
            let name = h.rebase(e.name);
            thiscall(h, ENUM_INIT, d, &[name]);
        }
        let tokens = h.rebase(e.tokens);
        thiscall(h, ENUM_SET_TOKENS, d, &[tokens, e.count]);
        let name: u32 = h.read(d);
        if name == 0 {
            return 0;
        }
        let found = cdecl(h, FIND_NAMED, &[name]);
        if found != 0 {
            return found;
        }
        let list = h.rebase(ENUM_LIST);
        let head: u32 = h.read(list);
        h.write(d + 4, head);
        h.write(list, d);
        head
    }

    /// A bind list; `eax` as the original returns it: the first failing
    /// entry's, else the tail call's (or the last entry's 0).
    pub fn bind_list(h: &mut impl Host, l: &BindList) -> u32 {
        for c in l.calls {
            match *c {
                Init::Call(f) => {
                    cdecl(h, f, &[]);
                }
                Init::RegisterGot(g) => {
                    let t = cdecl(h, g, &[]);
                    cdecl(h, REGISTER_TYPE, &[t]);
                }
            }
        }
        let mut eax = 0;
        for &e in l.entries {
            let e = h.rebase(e);
            eax = cdecl(h, l.register, &[e]);
            if eax != 0 {
                return eax;
            }
        }
        match l.tail {
            Some(t) => cdecl(h, t, &[]),
            None => eax,
        }
    }

    /// `0x005d_6960` (cdecl `()`): the scripting binds, then each Lua
    /// library's `{name, luaopen}` pair (built in `frame`, the original's
    /// `[ebp-8]`) pushed into [`LUA_LIBS`] through a fresh block of
    /// `(count + 1) * 8` bytes. Returns 0.
    pub fn scripting_initialize(h: &mut impl Host, frame: u32) -> u32 {
        for f in SCRIPTING_BINDS {
            cdecl(h, f, &[]);
        }
        let libs = h.rebase(LUA_LIBS);
        for (name, open) in LUA_LIB_PAIRS {
            let count: u32 = h.read(libs);
            let size = ((count & 0x7fff_ffff).wrapping_add(1)) << 3;
            let (name, open) = (h.rebase(name), h.rebase(open));
            h.write(frame, name);
            h.write(frame + 4, open);
            let block = cdecl(h, PHYRE_MALLOC, &[size, 4]);
            thiscall(h, PUSH_INTO8, libs, &[block, frame]);
        }
        0
    }
}
