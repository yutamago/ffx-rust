//! Steam (docs/host.md, "Steam"): `FFX.exe`'s `steam_api.dll` imports
//! bridged to the real 64-bit `steam_api64.dll`.
//!
//! The game reaches Steam in three ways (docs/formats/platform.md,
//! "Steam"): the flat imports (`SteamAPI_Init`, `SteamAPI_RunCallbacks`,
//! `SteamAPI_Shutdown`, `SteamAPI_RegisterCallback`/`UnregisterCallback`,
//! the accessors `SteamUser`, `SteamUtils`, `SteamUserStats`), the
//! interfaces' vtables (thiscall methods of `SteamUser017`,
//! `SteamUtils007` and `STEAMUSERSTATS_INTERFACE_VERSION011`), and the
//! callbacks Steam calls back on the game's `CCallback` objects during
//! `SteamAPI_RunCallbacks`.
//!
//! Here each interface is a word in the arena pointing at a vtable whose
//! slots are `int3` traps ([`crate::exec::Trap`], as [`crate::com`]
//! does), so the game's interface pointers are `u32` handles; the trap
//! reads the 32-bit arguments (strings from the arena), makes the 64-bit
//! call through a [`Backend`] and writes the results back (a `bool` out
//! parameter, a string copied into the arena). Each callback object the
//! game registers gets a 64-bit twin ([`HostCallback`]) that the DLL
//! calls; its events are queued and, once `SteamAPI_RunCallbacks` has
//! returned, handed to the game's object through its own vtable
//! (`Run(void *)`, slot 1), in the order Steam produced them.
//!
//! When `steam_api64.dll` is missing (beside `ffx-rust.exe`, or at
//! `FFX_STEAM_API`), on Linux, or when `SteamAPI_Init` fails, Steam is
//! absent as before this bridge: `SteamAPI_Init` returns false, so
//! `Steam_Init` leaves `g_SteamReady` clear and the game never asks for an
//! interface; the reason goes to the notes.
//!
//! Never forwarded: `SteamAPI_RestartAppIfNecessary` (it would start the
//! Steam build of `FFX.exe`) and `ISteamUserStats::ResetAllStats` (the
//! debug tweak menu's item 0xc; it would wipe the player's achievements).

use std::collections::HashMap;
use std::ffi::{CString, c_void};
use std::sync::Mutex;

use ffx_bridge::Mem;

use crate::exec::{Exec, Stop, Trap};
use crate::table::{Args, Conv, Ret};

/// FFX/X-2 HD Remaster's Steam app id (`steamapps/appmanifest_359870.acf`).
pub const APP_ID: u32 = 359_870;

/// The interfaces the game asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Iface {
    /// `ISteamUser`.
    User,
    /// `ISteamUtils`.
    Utils,
    /// `ISteamUserStats`.
    UserStats,
}

impl Iface {
    /// Every interface, in handle order.
    pub const ALL: [Self; 3] = [Self::User, Self::Utils, Self::UserStats];

    /// The version string the game's own 32-bit `steam_api.dll` asks
    /// `steamclient` for behind `SteamUser()` and co (its strings, beside
    /// `SteamClient016`; `FFX.exe` names none). The 64-bit side asks for
    /// the same versions, so the vtable slots are the same.
    #[must_use]
    pub const fn version(self) -> &'static str {
        match self {
            Self::User => "SteamUser017",
            Self::Utils => "SteamUtils007",
            Self::UserStats => "STEAMUSERSTATS_INTERFACE_VERSION011",
        }
    }

    /// The `steam_api.dll` accessor that returns it.
    #[must_use]
    pub const fn accessor(self) -> &'static str {
        match self {
            Self::User => "SteamUser",
            Self::Utils => "SteamUtils",
            Self::UserStats => "SteamUserStats",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::User => 0,
            Self::Utils => 1,
            Self::UserStats => 2,
        }
    }
}

/// What a bridged method does with its arguments and result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `uint32 f()`.
    GetAppId,
    /// `void f(ENotificationPosition)`.
    SetOverlayNotificationPosition,
    /// `bool f()`: `ISteamUser::BLoggedOn`.
    LoggedOn,
    /// `bool f()`: `RequestCurrentStats`.
    RequestCurrentStats,
    /// `bool f(const char *name, bool *achieved)`.
    GetAchievement,
    /// `bool f(const char *name)`.
    SetAchievement,
    /// `bool f()`: `StoreStats`.
    StoreStats,
    /// `const char *f(const char *name, const char *key)`.
    GetAchievementDisplayAttribute,
    /// `bool f(bool achievementsToo)`: refused, never forwarded.
    ResetAllStats,
}

/// One bridged vtable slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Method {
    /// The interface.
    pub iface: Iface,
    /// The vtable slot (the game's call offset / 4).
    pub slot: usize,
    /// The method's name in `isteam*.h`.
    pub name: &'static str,
    /// Stack words besides `this` (thiscall: `this` in `ecx`, callee pops).
    pub words: usize,
    /// What it does.
    pub kind: Kind,
}

/// Every vtable slot `FFX.exe` calls, from its call sites (`call [edx+N]`
/// after the accessor; docs/formats/platform.md, "steam_api.dll"):
/// `Steam_Init` `0x006234c1` (Utils +0x28), `0x006234d1` (User +0x04);
/// `StatsAndAchievements_ctor` `0x006f1296` (Utils +0x24), `0x006f12e8`
/// (User +0x04), `0x006f12f8` (`UserStats` +0x00);
/// `StatsAndAchievements_ResetAll` `0x006f142c` (+0x54);
/// `StatsAndAchievements_OnUserStatsReceived` `0x006f1649` (+0x18),
/// `0x006f165e` and `0x006f168b` (+0x30);
/// `StatsAndAchievements_RequestStats` `0x006f17fa` (User +0x04),
/// `0x006f180c` (+0x00); `StatsAndAchievements_SetAchievementByName`
/// `0x006f18cc` (+0x1c), `0x006f18dc` (+0x28);
/// `StatsAndAchievements_SetAchievement_006f1900` `0x006f193b` (+0x1c),
/// `0x006f1948` (+0x28).
pub const METHODS: &[Method] = &[
    Method {
        iface: Iface::User,
        slot: 1,
        name: "BLoggedOn",
        words: 0,
        kind: Kind::LoggedOn,
    },
    Method {
        iface: Iface::Utils,
        slot: 9,
        name: "GetAppID",
        words: 0,
        kind: Kind::GetAppId,
    },
    Method {
        iface: Iface::Utils,
        slot: 10,
        name: "SetOverlayNotificationPosition",
        words: 1,
        kind: Kind::SetOverlayNotificationPosition,
    },
    Method {
        iface: Iface::UserStats,
        slot: 0,
        name: "RequestCurrentStats",
        words: 0,
        kind: Kind::RequestCurrentStats,
    },
    Method {
        iface: Iface::UserStats,
        slot: 6,
        name: "GetAchievement",
        words: 2,
        kind: Kind::GetAchievement,
    },
    Method {
        iface: Iface::UserStats,
        slot: 7,
        name: "SetAchievement",
        words: 1,
        kind: Kind::SetAchievement,
    },
    Method {
        iface: Iface::UserStats,
        slot: 10,
        name: "StoreStats",
        words: 0,
        kind: Kind::StoreStats,
    },
    Method {
        iface: Iface::UserStats,
        slot: 12,
        name: "GetAchievementDisplayAttribute",
        words: 2,
        kind: Kind::GetAchievementDisplayAttribute,
    },
    Method {
        iface: Iface::UserStats,
        slot: 21,
        name: "ResetAllStats",
        words: 1,
        kind: Kind::ResetAllStats,
    },
];

/// The bridged method at `slot` of `iface`.
#[must_use]
pub fn method(iface: Iface, slot: usize) -> Option<&'static Method> {
    METHODS.iter().find(|m| m.iface == iface && m.slot == slot)
}

/// A callback struct's layout on both sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Callback {
    /// `k_iCallback`.
    pub id: i32,
    /// The struct's name.
    pub name: &'static str,
    /// Its size for the 32-bit game (its `CCallback`'s
    /// `GetCallbackSizeBytes`).
    pub size32: u32,
    /// Its size in the 64-bit DLL.
    pub size64: u32,
    /// Its fields: name, 64-bit offset, 32-bit offset, length.
    pub fields: &'static [(&'static str, u32, u32, u32)],
}

/// The callbacks the game registers (`StatsAndAchievements_ctor`; ids
/// 0x44d..0x44f). Steam packs its callback structs to 8 on Windows on
/// both widths (`VALVE_CALLBACK_PACK_LARGE`), so the 64-bit and 32-bit
/// layouts agree; the sizes are the game's own (`ReturnConst_006f1590`
/// 0x18, `ReturnConst_006f15a0` 0x10, `ReturnConst_006f1580` 0x98 in the
/// vtables `0x00b4b9f0`, `0x00b4ba00`, `0x00b4ba10`, slot 2).
pub const CALLBACKS: &[Callback] = &[
    Callback {
        id: 1101,
        name: "UserStatsReceived_t",
        size32: 0x18,
        size64: 0x18,
        fields: &[
            ("m_nGameID", 0, 0, 8),
            ("m_eResult", 8, 8, 4),
            // `CSteamID` is packed to 1 (steamclientpublic.h), so it
            // follows the result at +0xc; the size rounds up to 8
            ("m_steamIDUser", 0xc, 0xc, 8),
        ],
    },
    Callback {
        id: 1102,
        name: "UserStatsStored_t",
        size32: 0x10,
        size64: 0x10,
        fields: &[("m_nGameID", 0, 0, 8), ("m_eResult", 8, 8, 4)],
    },
    Callback {
        id: 1103,
        name: "UserAchievementStored_t",
        size32: 0x98,
        size64: 0x98,
        fields: &[
            ("m_nGameID", 0, 0, 8),
            ("m_bGroupAchievement", 8, 8, 1),
            ("m_rgchAchievementName", 9, 9, 128),
            ("m_nCurProgress", 0x8c, 0x8c, 4),
            ("m_nMaxProgress", 0x90, 0x90, 4),
        ],
    },
];

/// The layout of callback `id`.
#[must_use]
pub fn callback(id: i32) -> Option<&'static Callback> {
    CALLBACKS.iter().find(|c| c.id == id)
}

/// A 64-bit callback struct as the game's 32-bit one: each field moved to
/// its 32-bit offset, the padding zero. `None` for an id the bridge does
/// not know.
#[must_use]
pub fn marshal_callback(id: i32, host: &[u8]) -> Option<Vec<u8>> {
    let c = callback(id)?;
    let mut out = vec![0u8; c.size32 as usize];
    for &(_, off64, off32, len) in c.fields {
        let (from, to, len) = (off64 as usize, off32 as usize, len as usize);
        if let (Some(src), Some(dst)) = (host.get(from..from + len), out.get_mut(to..to + len)) {
            dst.copy_from_slice(src);
        }
    }
    Some(out)
}

/// A call into the 64-bit side, its arguments marshalled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Call {
    /// `ISteamUtils::GetAppID`.
    GetAppId,
    /// `ISteamUtils::SetOverlayNotificationPosition`.
    SetOverlayNotificationPosition(i32),
    /// `ISteamUser::BLoggedOn`.
    LoggedOn,
    /// `ISteamUserStats::RequestCurrentStats`.
    RequestCurrentStats,
    /// `ISteamUserStats::GetAchievement` (the out parameter is the
    /// bridge's).
    GetAchievement(CString),
    /// `ISteamUserStats::SetAchievement`.
    SetAchievement(CString),
    /// `ISteamUserStats::StoreStats`.
    StoreStats,
    /// `ISteamUserStats::GetAchievementDisplayAttribute`.
    GetAchievementDisplayAttribute(CString, CString),
}

/// A 64-bit call's result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// Nothing (`void`).
    Void,
    /// A `bool`.
    Bool(bool),
    /// A `uint32`.
    U32(u32),
    /// `GetAchievement`: its `bool`, and `*pbAchieved` when it returned
    /// true.
    Achievement(bool, Option<bool>),
    /// A string (`None`: a null pointer).
    Text(Option<Vec<u8>>),
}

/// A callback Steam delivered to a registered object's twin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// The game's `CCallback` object.
    pub guest: u32,
    /// The callback id.
    pub id: i32,
    /// The 64-bit struct's bytes.
    pub data: Vec<u8>,
}

/// The 64-bit side: `steam_api64.dll` (Windows), or a stand-in in tests.
pub trait Backend: Send {
    /// `SteamAPI_Init` and the interfaces.
    ///
    /// # Errors
    /// Steam's reason (not running, no app id, an interface version it
    /// does not serve).
    fn init(&mut self) -> Result<(), String>;
    /// Whether the DLL gave interface `iface`.
    fn has(&self, iface: Iface) -> bool;
    /// `SteamAPI_RunCallbacks`: the events it delivered, in order.
    fn run_callbacks(&mut self) -> Vec<Event>;
    /// `SteamAPI_Shutdown`.
    fn shutdown(&mut self);
    /// `SteamAPI_RegisterCallback` for the game's object `guest` (its
    /// flags byte `flags`) as callback `id`, its struct `size` bytes: the
    /// flags byte and id the DLL left in the twin.
    fn register(&mut self, guest: u32, id: i32, size: u32, flags: u8) -> (u8, i32);
    /// `SteamAPI_UnregisterCallback`: the twin's flags byte afterwards,
    /// `None` when `guest` has no twin.
    fn unregister(&mut self, guest: u32) -> Option<u8>;
    /// A method call (never `ResetAllStats`: [`Call`] has no such call).
    fn call(&mut self, call: &Call) -> Reply;
}

/// The bridge's state while Steam is present.
pub struct Bridge {
    backend: Box<dyn Backend>,
    /// The game's interface handles, by [`Iface::index`].
    objects: [u32; 3],
    /// The game's registered `CCallback` objects: id and 32-bit size.
    registered: HashMap<u32, (i32, u32)>,
    /// Strings handed to the game, by content (`GetAchievementDisplayAttribute`
    /// returns a pointer the game copies at once; one block per distinct
    /// string).
    strings: HashMap<Vec<u8>, u32>,
}

impl std::fmt::Debug for Bridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bridge")
            .field("objects", &self.objects)
            .field("registered", &self.registered)
            .finish_non_exhaustive()
    }
}

/// One callback to hand to the game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    /// The game's `CCallback` object (`ecx`).
    pub object: u32,
    /// Its `Run(void *)`: vtable slot 1.
    pub run: u32,
    /// The 32-bit struct.
    pub data: Vec<u8>,
}

impl Bridge {
    /// A bridge over `backend` (after its `init` succeeded), the game's
    /// interface handles at `objects`.
    #[must_use]
    pub fn new(backend: Box<dyn Backend>, objects: [u32; 3]) -> Self {
        Self {
            backend,
            objects,
            registered: HashMap::new(),
            strings: HashMap::new(),
        }
    }

    /// The accessor's answer: the handle, or 0 when the DLL has no such
    /// interface.
    #[must_use]
    pub fn accessor(&self, iface: Iface) -> u32 {
        if self.backend.has(iface) {
            self.objects[iface.index()]
        } else {
            0
        }
    }

    /// `SteamAPI_RegisterCallback(object, id)`: registers the twin and
    /// mirrors the flags byte (+4) and id (+8) into the game's object as
    /// `steam_api.dll` sets them. An id the bridge has no layout for is
    /// marked registered but never delivered (noted).
    pub fn register(&mut self, mem: &mut impl Mem, object: u32, id: i32) {
        let flags: u8 = mem.read(object + 4);
        let (flags, id) = if let Some(c) = callback(id) {
            self.registered.insert(object, (id, c.size32));
            self.backend.register(object, id, c.size32, flags)
        } else {
            crate::log::note(&format!(
                "Steam: callback {id} on {object:#010x} has no layout here; never delivered"
            ));
            (flags | 1, id)
        };
        mem.write(object + 4, flags);
        mem.write(object + 8, id);
    }

    /// `SteamAPI_UnregisterCallback(object)`.
    pub fn unregister(&mut self, mem: &mut impl Mem, object: u32) {
        self.registered.remove(&object);
        let flags = match self.backend.unregister(object) {
            Some(f) => f,
            None => mem.read::<u8>(object + 4) & !1,
        };
        mem.write(object + 4, flags);
    }

    /// `SteamAPI_RunCallbacks`: what to hand to the game, in order (the
    /// events of objects still registered, their structs marshalled; the
    /// `Run` address read from each object's vtable).
    pub fn run_callbacks(&mut self, mem: &impl Mem) -> Vec<Delivery> {
        let events = self.backend.run_callbacks();
        plan(mem, &self.registered, &events)
    }

    /// `SteamAPI_Shutdown`.
    pub fn shutdown(&mut self) {
        self.backend.shutdown();
    }

    /// A vtable call: the arguments read from `mem`, the call made, the
    /// results written back (`alloc` gives arena blocks for strings).
    pub fn dispatch(
        &mut self,
        m: &Method,
        mem: &mut impl Mem,
        args: &Args,
        alloc: &mut dyn FnMut(u32) -> u32,
    ) -> Ret {
        let word = |i: usize| args.stack.get(i).copied().unwrap_or(0);
        let call = match m.kind {
            Kind::GetAppId => Call::GetAppId,
            #[allow(clippy::cast_possible_wrap)] // an enum, as the game pushes it
            Kind::SetOverlayNotificationPosition => {
                Call::SetOverlayNotificationPosition(word(0) as i32)
            }
            Kind::LoggedOn => Call::LoggedOn,
            Kind::RequestCurrentStats => Call::RequestCurrentStats,
            Kind::GetAchievement => match read_cstr(mem, word(0)) {
                Some(n) => Call::GetAchievement(n),
                None => return Ret::eax(0),
            },
            Kind::SetAchievement => match read_cstr(mem, word(0)) {
                Some(n) => {
                    crate::log::note(&format!("Steam: SetAchievement({})", n.to_string_lossy()));
                    Call::SetAchievement(n)
                }
                None => return Ret::eax(0),
            },
            Kind::StoreStats => Call::StoreStats,
            Kind::GetAchievementDisplayAttribute => {
                match (read_cstr(mem, word(0)), read_cstr(mem, word(1))) {
                    (Some(n), Some(k)) => Call::GetAchievementDisplayAttribute(n, k),
                    _ => return Ret::eax(0),
                }
            }
            Kind::ResetAllStats => {
                crate::log::note("Steam: ResetAllStats refused (the bridge never forwards it)");
                return Ret::eax(0);
            }
        };
        match self.backend.call(&call) {
            Reply::Void => Ret::VOID,
            Reply::Bool(b) => Ret::eax(u32::from(b)),
            Reply::U32(v) => Ret::eax(v),
            Reply::Achievement(ok, achieved) => {
                if let (true, Some(a)) = (ok, achieved)
                    && word(1) != 0
                {
                    mem.write(word(1), u8::from(a));
                }
                Ret::eax(u32::from(ok))
            }
            Reply::Text(None) => Ret::eax(0),
            Reply::Text(Some(s)) => Ret::eax(self.intern(mem, &s, alloc)),
        }
    }

    /// `s` as a NUL-terminated string in the arena (0 when the heap is
    /// full); the same block for the same string.
    fn intern(&mut self, mem: &mut impl Mem, s: &[u8], alloc: &mut dyn FnMut(u32) -> u32) -> u32 {
        if let Some(&at) = self.strings.get(s) {
            return at;
        }
        let Ok(len) = u32::try_from(s.len() + 1) else {
            return 0;
        };
        let at = alloc(len);
        if at == 0 {
            return 0;
        }
        mem.write_bytes(at, s);
        mem.write(at + len - 1, 0u8);
        self.strings.insert(s.to_vec(), at);
        at
    }
}

/// The deliveries for `events`: objects still in `registered` only, each
/// struct marshalled to the game's layout and size, `Run` from slot 1 of
/// the object's vtable.
#[must_use]
pub fn plan(
    mem: &impl Mem,
    registered: &HashMap<u32, (i32, u32)>,
    events: &[Event],
) -> Vec<Delivery> {
    events
        .iter()
        .filter_map(|ev| {
            let &(id, _) = registered.get(&ev.guest)?;
            if id != ev.id {
                return None;
            }
            let data = marshal_callback(id, &ev.data)?;
            let vtable: u32 = mem.read(ev.guest);
            Some(Delivery {
                object: ev.guest,
                run: mem.read(vtable + 4),
                data,
            })
        })
        .collect()
}

/// The C string at `at` (at most 4 KiB; `None` for a null pointer or one
/// with an inner NUL past the limit).
#[must_use]
pub fn read_cstr(mem: &impl Mem, at: u32) -> Option<CString> {
    if at == 0 {
        return None;
    }
    let mut s = Vec::new();
    for i in 0..4096u32 {
        let b: u8 = mem.read(at.wrapping_add(i));
        if b == 0 {
            return CString::new(s).ok();
        }
        s.push(b);
    }
    None
}

// ---------------------------------------------------------------------------
// The 64-bit callback objects: what `SteamAPI_RegisterCallback` in
// `steam_api64.dll` takes. MSVC lays out `CCallbackBase` as the vtable
// pointer, `uint8 m_nCallbackFlags` (+8), `int m_iCallback` (+12); its
// virtuals are `Run(void *)` and `Run(void *, bool, SteamAPICall_t)`,
// which MSVC places in reverse (the game's vtables show the same: slot 0
// `ret 0x10`, slot 1 `ret 4`), then `GetCallbackSizeBytes`.

/// The twin vtable.
#[repr(C)]
struct HostVtable {
    run_call_result: unsafe extern "C" fn(*mut HostCallback, *const c_void, u8, u64),
    run: unsafe extern "C" fn(*mut HostCallback, *const c_void),
    size: unsafe extern "C" fn(*mut HostCallback) -> i32,
}

/// A game `CCallback` object's 64-bit twin.
#[repr(C)]
pub struct HostCallback {
    vtable: &'static HostVtable,
    /// `m_nCallbackFlags`: the DLL's.
    flags: u8,
    /// `m_iCallback`: the DLL's.
    id: i32,
    /// The game's object.
    guest: u32,
    /// Bytes to copy from a delivered struct.
    size: u32,
}

static HOST_VTABLE: HostVtable = HostVtable {
    run_call_result,
    run: run_callback,
    size: callback_size,
};

/// Events the DLL delivered during the current `SteamAPI_RunCallbacks`.
static PENDING: Mutex<Vec<Event>> = Mutex::new(Vec::new());

impl HostCallback {
    /// A twin for the game's object `guest`, callback `id`, `size` bytes
    /// (the 64-bit struct's), flags `flags` (the game's, minus
    /// "registered", which the DLL sets).
    #[must_use]
    pub fn new(guest: u32, id: i32, size: u32, flags: u8) -> Box<Self> {
        Box::new(Self {
            vtable: &HOST_VTABLE,
            flags: flags & !1,
            id,
            guest,
            size,
        })
    }

    /// `m_nCallbackFlags`.
    #[must_use]
    pub fn flags(&self) -> u8 {
        self.flags
    }

    /// `m_iCallback`.
    #[must_use]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// The events the twins received since the last call.
    #[must_use]
    pub fn take_events() -> Vec<Event> {
        PENDING
            .lock()
            .map(|mut p| std::mem::take(&mut *p))
            .unwrap_or_default()
    }
}

/// Queues `this`'s callback struct at `p`.
///
/// # Safety
/// `this` is a live [`HostCallback`]; `p` points at its callback's struct
/// (`size` bytes) or is null.
unsafe fn queue(this: *mut HostCallback, p: *const c_void) {
    if this.is_null() || p.is_null() {
        return;
    }
    // SAFETY: the caller's promise.
    let (guest, id, size) = unsafe { ((*this).guest, (*this).id, (*this).size) };
    // SAFETY: as above, `size` bytes at `p`.
    let data = unsafe { std::slice::from_raw_parts(p.cast::<u8>(), size as usize) }.to_vec();
    if let Ok(mut q) = PENDING.lock() {
        q.push(Event { guest, id, data });
    }
}

/// Slot 0: `Run(void *, bool, SteamAPICall_t)` (call results; the game
/// registers none, so this is never expected, but queued all the same).
unsafe extern "C" fn run_call_result(this: *mut HostCallback, p: *const c_void, _: u8, _: u64) {
    // SAFETY: the DLL calls it on a twin it was given, with its struct.
    unsafe { queue(this, p) }
}

/// Slot 1: `Run(void *)`.
unsafe extern "C" fn run_callback(this: *mut HostCallback, p: *const c_void) {
    // SAFETY: as `run_call_result`.
    unsafe { queue(this, p) }
}

/// Slot 2: `GetCallbackSizeBytes`.
unsafe extern "C" fn callback_size(this: *mut HostCallback) -> i32 {
    if this.is_null() {
        return 0;
    }
    // SAFETY: a twin the DLL was given.
    i32::try_from(unsafe { (*this).size }).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// The process's bridge and the imports.

/// Steam while present (`None`: absent).
static STEAM: Mutex<Option<Bridge>> = Mutex::new(None);

/// A back end the next `SteamAPI_Init` takes instead of loading the DLL
/// (tests).
static OVERRIDE: Mutex<Option<Box<dyn Backend>>> = Mutex::new(None);

/// Makes the next `SteamAPI_Init` use `backend` instead of
/// `steam_api64.dll` (tests: a stand-in for Steam).
pub fn use_backend(backend: Box<dyn Backend>) {
    if let Ok(mut o) = OVERRIDE.lock() {
        *o = Some(backend);
    }
}

/// Whether Steam is present (a successful `SteamAPI_Init` and no
/// `SteamAPI_Shutdown` since).
#[must_use]
pub fn present() -> bool {
    STEAM.lock().is_ok_and(|s| s.is_some())
}

/// Where `steam_api64.dll` is looked for: `FFX_STEAM_API`, else beside
/// the running executable.
#[must_use]
pub fn dll_path() -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("FFX_STEAM_API") {
        return Some(p.into());
    }
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("steam_api64.dll")))
}

/// The real back end, or why there is none.
fn load() -> Result<Box<dyn Backend>, String> {
    #[cfg(windows)]
    {
        let path = dll_path().ok_or("no path for steam_api64.dll")?;
        if !path.is_file() {
            return Err(format!("no {}", path.display()));
        }
        dll::Dll::load(&path).map(|d| Box::new(d) as Box<dyn Backend>)
    }
    #[cfg(not(windows))]
    {
        Err("steam_api64.dll is Windows-only".into())
    }
}

/// The game's interface handles: one word each pointing at its vtable,
/// whose slots are `int3` traps (`SLOTS` per interface; a slot without a
/// bridged method stops the run, naming it).
fn make_objects(e: &Exec) -> crate::Result<[u32; 3]> {
    const SLOTS: u32 = 64;
    let arena = e.arena();
    let at = arena.map_anywhere(0x0300_0000, 0x1000, "Steam interfaces")?;
    let mut objects = [0u32; 3];
    for (i, iface) in Iface::ALL.into_iter().enumerate() {
        let n = u32::try_from(i).unwrap_or(0);
        let object = at + 16 * n;
        let vtable = at + 0x40 + n * SLOTS * 4;
        let stubs = at + 0x40 + 3 * SLOTS * 4 + n * SLOTS;
        arena.fill(object, &vtable.to_le_bytes());
        for slot in 0..SLOTS {
            let stub = stubs + slot;
            arena.fill(stub, &[0xcc]);
            arena.fill(vtable + 4 * slot, &stub.to_le_bytes());
            let words = method(iface, slot as usize).map_or(0, |m| m.words);
            e.shared().add_trap(
                stub,
                Trap {
                    run: method_trap,
                    tag: (n << 8) | slot,
                    conv: Conv::Thiscall,
                    words,
                },
            );
        }
        objects[i] = object;
    }
    Ok(objects)
}

/// Every interface slot's trap.
fn method_trap(e: &mut Exec, tag: u32, a: &Args) -> Ret {
    let iface = Iface::ALL[(tag >> 8) as usize % 3];
    let slot = (tag & 0xff) as usize;
    let Some(m) = method(iface, slot) else {
        e.end(Stop::Failed(
            format!(
                "Steam: {} slot {slot} is not bridged (src/steam.rs METHODS)",
                iface.version()
            ),
            0,
        ));
        return Ret::VOID;
    };
    let Ok(mut steam) = STEAM.lock() else {
        return Ret::eax(0);
    };
    let Some(bridge) = steam.as_mut() else {
        return Ret::eax(0);
    };
    let mut mem = e.mem();
    let mut alloc = |n| e.alloc(n);
    bridge.dispatch(m, &mut mem, a, &mut alloc)
}

/// `SteamAPI_RestartAppIfNecessary(appId)`: false, never forwarded (the
/// game passes 0; a restart would start the Steam build of `FFX.exe`).
fn restart_app(_: &mut Exec, _: &Args) -> Ret {
    Ret::eax(0)
}

/// `SteamAPI_Init()`: the DLL loaded and started, the interfaces made;
/// false (Steam absent, as before the bridge) when any of it fails.
fn init(e: &mut Exec, _: &Args) -> Ret {
    if present() {
        return Ret::eax(1);
    }
    let backend = OVERRIDE.lock().ok().and_then(|mut o| o.take());
    let mut backend = match backend.map_or_else(load, Ok) {
        Ok(b) => b,
        Err(why) => {
            crate::log::note(&format!(
                "Steam: absent ({why}); SteamAPI_Init returns false"
            ));
            return Ret::eax(0);
        }
    };
    if let Err(why) = backend.init() {
        crate::log::note(&format!(
            "Steam: absent (steam_api64.dll: {why}; Steam must run, and ffx-rust.exe be started \
             through Steam or with SteamAppId={APP_ID}); SteamAPI_Init returns false"
        ));
        return Ret::eax(0);
    }
    let objects = match make_objects(e) {
        Ok(o) => o,
        Err(why) => {
            backend.shutdown();
            crate::log::note(&format!(
                "Steam: absent (no room for the interfaces: {why})"
            ));
            return Ret::eax(0);
        }
    };
    let missing: Vec<&str> = Iface::ALL
        .iter()
        .filter(|i| !backend.has(**i))
        .map(|i| i.version())
        .collect();
    crate::log::note(&format!(
        "Steam: present (steam_api64.dll); interfaces at {:#010x}{}",
        objects[0],
        if missing.is_empty() {
            String::new()
        } else {
            format!(", missing {}", missing.join(", "))
        }
    ));
    if let Ok(mut s) = STEAM.lock() {
        *s = Some(Bridge::new(backend, objects));
    }
    Ret::eax(1)
}

/// `SteamAPI_RunCallbacks()`: Steam's events to the game's objects.
fn run_callbacks(e: &mut Exec, _: &Args) -> Ret {
    let deliveries = {
        let Ok(mut s) = STEAM.lock() else {
            return Ret::VOID;
        };
        let Some(b) = s.as_mut() else {
            return Ret::VOID;
        };
        b.run_callbacks(&e.mem())
    };
    // outside the lock: the handlers call the interfaces
    for d in deliveries {
        let Ok(len) = u32::try_from(d.data.len()) else {
            continue;
        };
        let buf = e.alloc(len);
        if buf == 0 {
            crate::log::note("Steam: no heap for a callback struct; dropped");
            continue;
        }
        e.mem().write_bytes(buf, &d.data);
        let args = Args {
            ecx: d.object,
            stack: vec![buf],
            ..Args::default()
        };
        let r = e.call(d.run, Conv::Thiscall, &args);
        e.release(buf);
        if r.is_err() {
            break;
        }
    }
    Ret::VOID
}

/// `SteamAPI_Shutdown()`.
fn shutdown(_: &mut Exec, _: &Args) -> Ret {
    if let Ok(mut s) = STEAM.lock()
        && let Some(mut b) = s.take()
    {
        b.shutdown();
        crate::log::note("Steam: SteamAPI_Shutdown");
    }
    Ret::VOID
}

/// `SteamAPI_RegisterCallback(CCallbackBase *, int)`: nothing while
/// Steam is absent.
fn register_callback(e: &mut Exec, a: &Args) -> Ret {
    if let Ok(mut s) = STEAM.lock()
        && let Some(b) = s.as_mut()
    {
        #[allow(clippy::cast_possible_wrap)] // `int iCallback`
        b.register(&mut e.mem(), a.stack[0], a.stack[1] as i32);
    }
    Ret::VOID
}

/// `SteamAPI_UnregisterCallback(CCallbackBase *)`: nothing while Steam is
/// absent.
fn unregister_callback(e: &mut Exec, a: &Args) -> Ret {
    if let Ok(mut s) = STEAM.lock()
        && let Some(b) = s.as_mut()
    {
        b.unregister(&mut e.mem(), a.stack[0]);
    }
    Ret::VOID
}

fn accessor(iface: Iface) -> Ret {
    let h = STEAM
        .lock()
        .ok()
        .and_then(|s| s.as_ref().map(|b| b.accessor(iface)))
        .unwrap_or(0);
    Ret::eax(h)
}

fn steam_user(_: &mut Exec, _: &Args) -> Ret {
    accessor(Iface::User)
}

fn steam_utils(_: &mut Exec, _: &Args) -> Ret {
    accessor(Iface::Utils)
}

fn steam_user_stats(_: &mut Exec, _: &Args) -> Ret {
    accessor(Iface::UserStats)
}

/// `FFX.exe`'s nine `steam_api.dll` imports (all cdecl).
pub const TABLE: &[(&str, &str, Conv, usize, crate::imports::ImportFn)] = &[
    (
        "steam_api.dll",
        "SteamAPI_RestartAppIfNecessary",
        Conv::Cdecl,
        1,
        restart_app,
    ),
    ("steam_api.dll", "SteamAPI_Init", Conv::Cdecl, 0, init),
    (
        "steam_api.dll",
        "SteamAPI_RunCallbacks",
        Conv::Cdecl,
        0,
        run_callbacks,
    ),
    (
        "steam_api.dll",
        "SteamAPI_Shutdown",
        Conv::Cdecl,
        0,
        shutdown,
    ),
    (
        "steam_api.dll",
        "SteamAPI_RegisterCallback",
        Conv::Cdecl,
        2,
        register_callback,
    ),
    (
        "steam_api.dll",
        "SteamAPI_UnregisterCallback",
        Conv::Cdecl,
        1,
        unregister_callback,
    ),
    ("steam_api.dll", "SteamUser", Conv::Cdecl, 0, steam_user),
    ("steam_api.dll", "SteamUtils", Conv::Cdecl, 0, steam_utils),
    (
        "steam_api.dll",
        "SteamUserStats",
        Conv::Cdecl,
        0,
        steam_user_stats,
    ),
];

/// The `steam_api64.dll` exports the bridge calls (Steamworks SDK 1.65's
/// DLL: `SteamAPI_Init` is an inline over `SteamInternal_SteamAPI_Init`
/// there, and the interfaces come from `ISteamClient` at the game's
/// versions, not from the SDK's own `SteamAPI_SteamUserStats_v013` and
/// co).
pub const EXPORTS: &[&str] = &[
    "SteamInternal_SteamAPI_Init",
    "SteamAPI_RunCallbacks",
    "SteamAPI_Shutdown",
    "SteamAPI_RegisterCallback",
    "SteamAPI_UnregisterCallback",
    "SteamAPI_GetHSteamUser",
    "SteamAPI_GetHSteamPipe",
    "SteamClient",
    "SteamAPI_ISteamClient_GetISteamUser",
    "SteamAPI_ISteamClient_GetISteamUtils",
    "SteamAPI_ISteamClient_GetISteamUserStats",
];

/// The interface versions `SteamInternal_SteamAPI_Init` checks the
/// client serves: the game's three, each NUL-terminated, then an empty
/// one.
#[must_use]
pub fn init_versions() -> Vec<u8> {
    let mut v = Vec::new();
    for i in Iface::ALL {
        v.extend_from_slice(i.version().as_bytes());
        v.push(0);
    }
    v.push(0);
    v
}

/// `SteamErrMsg`'s size (`k_cchMaxSteamErrMsg`).
pub const ERR_MSG: usize = 1024;

#[cfg(windows)]
pub mod dll {
    //! The real `steam_api64.dll`, loaded at run time.

    use std::collections::HashMap;
    use std::ffi::{CStr, c_char, c_void};
    use std::path::Path;

    use windows_sys::Win32::Foundation::HMODULE;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    use super::{Backend, Call, Event, HostCallback, Iface, Reply};

    /// `SteamInternal_SteamAPI_Init(versions, SteamErrMsg *)` ->
    /// `ESteamAPIInitResult` (0: OK).
    type Init = unsafe extern "C" fn(*const c_char, *mut c_char) -> i32;
    type Void = unsafe extern "C" fn();
    type Register = unsafe extern "C" fn(*mut HostCallback, i32);
    type Unregister = unsafe extern "C" fn(*mut HostCallback);
    type Handle = unsafe extern "C" fn() -> i32;
    type Client = unsafe extern "C" fn() -> *mut c_void;
    /// `SteamAPI_ISteamClient_GetISteamUser` / `GetISteamUserStats`:
    /// (client, user, pipe, version).
    type UserIface = unsafe extern "C" fn(*mut c_void, i32, i32, *const c_char) -> *mut c_void;
    /// `SteamAPI_ISteamClient_GetISteamUtils`: (client, pipe, version).
    type PipeIface = unsafe extern "C" fn(*mut c_void, i32, *const c_char) -> *mut c_void;

    /// `steam_api64.dll`'s entry points and the interfaces it gave.
    pub struct Dll {
        init: Init,
        run_callbacks: Void,
        shutdown: Void,
        register: Register,
        unregister: Unregister,
        get_user: Handle,
        get_pipe: Handle,
        client: Client,
        get_user_iface: UserIface,
        get_utils: PipeIface,
        get_user_stats: UserIface,
        /// The 64-bit interface pointers, by [`Iface::index`].
        ifaces: [*mut c_void; 3],
        /// The twins, by the game's object (kept for the process: the DLL
        /// may hold a pointer to one until `SteamAPI_Shutdown`).
        twins: HashMap<u32, Box<HostCallback>>,
    }

    // SAFETY: the game calls Steam from its main thread only; the pointers
    // are the DLL's and live for the process.
    unsafe impl Send for Dll {}

    fn export(module: HMODULE, name: &str) -> Result<unsafe extern "system" fn() -> isize, String> {
        let c = format!("{name}\0");
        // SAFETY: a module handle from LoadLibraryW and a NUL-terminated name.
        unsafe { GetProcAddress(module, c.as_ptr()) }
            .ok_or_else(|| format!("steam_api64.dll has no {name}"))
    }

    impl Dll {
        /// Loads the DLL at `path` and finds [`super::EXPORTS`].
        ///
        /// # Errors
        /// When it won't load or lacks an export.
        pub fn load(path: &Path) -> Result<Self, String> {
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
            // SAFETY: a NUL-terminated wide path.
            let module = unsafe { LoadLibraryW(wide.as_ptr()) };
            if module.is_null() {
                return Err(format!("{} did not load", path.display()));
            }
            let f = |name| export(module, name);
            // SAFETY (each transmute): the export's declared signature
            // (steam_api.h, steam_api_flat.h).
            unsafe {
                Ok(Self {
                    init: std::mem::transmute::<_, Init>(f("SteamInternal_SteamAPI_Init")?),
                    run_callbacks: std::mem::transmute::<_, Void>(f("SteamAPI_RunCallbacks")?),
                    shutdown: std::mem::transmute::<_, Void>(f("SteamAPI_Shutdown")?),
                    register: std::mem::transmute::<_, Register>(f("SteamAPI_RegisterCallback")?),
                    unregister: std::mem::transmute::<_, Unregister>(f(
                        "SteamAPI_UnregisterCallback",
                    )?),
                    get_user: std::mem::transmute::<_, Handle>(f("SteamAPI_GetHSteamUser")?),
                    get_pipe: std::mem::transmute::<_, Handle>(f("SteamAPI_GetHSteamPipe")?),
                    client: std::mem::transmute::<_, Client>(f("SteamClient")?),
                    get_user_iface: std::mem::transmute::<_, UserIface>(f(
                        "SteamAPI_ISteamClient_GetISteamUser",
                    )?),
                    get_utils: std::mem::transmute::<_, PipeIface>(f(
                        "SteamAPI_ISteamClient_GetISteamUtils",
                    )?),
                    get_user_stats: std::mem::transmute::<_, UserIface>(f(
                        "SteamAPI_ISteamClient_GetISteamUserStats",
                    )?),
                    ifaces: [std::ptr::null_mut(); 3],
                    twins: HashMap::new(),
                })
            }
        }

        /// Slot `slot` of interface `iface`'s vtable.
        fn slot(&self, iface: Iface, slot: usize) -> Option<(*mut c_void, *const c_void)> {
            let this = self.ifaces[iface.index()];
            if this.is_null() {
                return None;
            }
            // SAFETY: a live interface: its first word is its vtable, which
            // has the version's slots.
            let f = unsafe { *(*this.cast::<*const *const c_void>()).add(slot) };
            Some((this, f))
        }

        fn text(p: *const c_char) -> Option<Vec<u8>> {
            // SAFETY: a string the DLL returned, valid until the next call.
            (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_bytes().to_vec())
        }
    }

    impl Backend for Dll {
        fn init(&mut self) -> Result<(), String> {
            let versions = super::init_versions();
            let mut err = vec![0u8; super::ERR_MSG];
            // SAFETY: the version list as the SDK's inline builds it, and a
            // `SteamErrMsg` buffer.
            let r = unsafe { (self.init)(versions.as_ptr().cast(), err.as_mut_ptr().cast()) };
            if r != 0 {
                let msg = CStr::from_bytes_until_nul(&err)
                    .map(|c| c.to_string_lossy().into_owned())
                    .unwrap_or_default();
                return Err(format!("SteamAPI_Init failed ({r}: {msg})"));
            }
            // SAFETY: after a successful init: the global user and pipe,
            // and the client they belong to.
            let (user, pipe, client) =
                unsafe { ((self.get_user)(), (self.get_pipe)(), (self.client)()) };
            if client.is_null() {
                return Err("SteamClient() is null".into());
            }
            for iface in Iface::ALL {
                let v = format!("{}\0", iface.version());
                let v = v.as_ptr().cast();
                // SAFETY: the client, its handles and a NUL-terminated
                // version string.
                self.ifaces[iface.index()] = unsafe {
                    match iface {
                        Iface::User => (self.get_user_iface)(client, user, pipe, v),
                        Iface::Utils => (self.get_utils)(client, pipe, v),
                        Iface::UserStats => (self.get_user_stats)(client, user, pipe, v),
                    }
                };
            }
            Ok(())
        }

        fn has(&self, iface: Iface) -> bool {
            !self.ifaces[iface.index()].is_null()
        }

        fn run_callbacks(&mut self) -> Vec<Event> {
            // SAFETY: the export; it calls the twins' `Run`.
            unsafe { (self.run_callbacks)() };
            HostCallback::take_events()
        }

        fn shutdown(&mut self) {
            // SAFETY: the export.
            unsafe { (self.shutdown)() };
            self.ifaces = [std::ptr::null_mut(); 3];
        }

        fn register(&mut self, guest: u32, id: i32, size: u32, flags: u8) -> (u8, i32) {
            let size = super::callback(id).map_or(size, |c| c.size64.min(size));
            let twin = self
                .twins
                .entry(guest)
                .or_insert_with(|| HostCallback::new(guest, id, size, flags));
            twin.id = id;
            twin.size = size;
            let p: *mut HostCallback = &mut **twin;
            // SAFETY: a boxed twin that outlives its registration.
            unsafe { (self.register)(p, id) };
            (twin.flags, twin.id)
        }

        fn unregister(&mut self, guest: u32) -> Option<u8> {
            let twin = self.twins.get_mut(&guest)?;
            let p: *mut HostCallback = &mut **twin;
            // SAFETY: a twin the DLL may hold.
            unsafe { (self.unregister)(p) };
            Some(twin.flags)
        }

        #[allow(clippy::too_many_lines)] // one arm per method
        fn call(&mut self, call: &Call) -> Reply {
            type Bool0 = unsafe extern "C" fn(*mut c_void) -> u8;
            // Each arm: the method's slot in the version's vtable and its
            // signature (`isteam*.h`; `this` first, the Win64 convention).
            // SAFETY (every arm): a live interface's slot, called with the
            // signature its header declares.
            unsafe {
                match call {
                    Call::GetAppId => self.slot(Iface::Utils, 9).map_or(Reply::U32(0), |(t, f)| {
                        let f = std::mem::transmute::<
                            *const c_void,
                            unsafe extern "C" fn(*mut c_void) -> u32,
                        >(f);
                        Reply::U32(f(t))
                    }),
                    Call::SetOverlayNotificationPosition(pos) => {
                        if let Some((t, f)) = self.slot(Iface::Utils, 10) {
                            let f = std::mem::transmute::<
                                *const c_void,
                                unsafe extern "C" fn(*mut c_void, i32),
                            >(f);
                            f(t, *pos);
                        }
                        Reply::Void
                    }
                    Call::LoggedOn => {
                        self.slot(Iface::User, 1)
                            .map_or(Reply::Bool(false), |(t, f)| {
                                Reply::Bool(std::mem::transmute::<*const c_void, Bool0>(f)(t) != 0)
                            })
                    }
                    Call::RequestCurrentStats => {
                        self.slot(Iface::UserStats, 0)
                            .map_or(Reply::Bool(false), |(t, f)| {
                                Reply::Bool(std::mem::transmute::<*const c_void, Bool0>(f)(t) != 0)
                            })
                    }
                    Call::GetAchievement(name) => self.slot(Iface::UserStats, 6).map_or(
                        Reply::Achievement(false, None),
                        |(t, f)| {
                            let f = std::mem::transmute::<
                                *const c_void,
                                unsafe extern "C" fn(*mut c_void, *const c_char, *mut u8) -> u8,
                            >(f);
                            let mut achieved = 0u8;
                            let ok = f(t, name.as_ptr(), &raw mut achieved) != 0;
                            Reply::Achievement(ok, ok.then_some(achieved != 0))
                        },
                    ),
                    Call::SetAchievement(name) => {
                        self.slot(Iface::UserStats, 7)
                            .map_or(Reply::Bool(false), |(t, f)| {
                                let f = std::mem::transmute::<
                                    *const c_void,
                                    unsafe extern "C" fn(*mut c_void, *const c_char) -> u8,
                                >(f);
                                Reply::Bool(f(t, name.as_ptr()) != 0)
                            })
                    }
                    Call::StoreStats => {
                        self.slot(Iface::UserStats, 10)
                            .map_or(Reply::Bool(false), |(t, f)| {
                                Reply::Bool(std::mem::transmute::<*const c_void, Bool0>(f)(t) != 0)
                            })
                    }
                    Call::GetAchievementDisplayAttribute(name, key) => self
                        .slot(Iface::UserStats, 12)
                        .map_or(Reply::Text(None), |(t, f)| {
                            let f = std::mem::transmute::<
                                *const c_void,
                                unsafe extern "C" fn(
                                    *mut c_void,
                                    *const c_char,
                                    *const c_char,
                                )
                                    -> *const c_char,
                            >(f);
                            Reply::Text(Self::text(f(t, name.as_ptr(), key.as_ptr())))
                        }),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffx_bridge::VecMem;

    /// A stand-in for Steam: answers from tables, records what it was asked.
    #[derive(Default)]
    struct Fake {
        calls: std::sync::Arc<Mutex<Vec<Call>>>,
        events: Vec<Event>,
        twins: HashMap<u32, Box<HostCallback>>,
    }

    impl Backend for Fake {
        fn init(&mut self) -> Result<(), String> {
            Ok(())
        }
        fn has(&self, iface: Iface) -> bool {
            iface != Iface::User
        }
        fn run_callbacks(&mut self) -> Vec<Event> {
            std::mem::take(&mut self.events)
        }
        fn shutdown(&mut self) {}
        fn register(&mut self, guest: u32, id: i32, size: u32, flags: u8) -> (u8, i32) {
            let t = HostCallback::new(guest, id, size, flags);
            // as steam_api: "registered", and the id
            let t = self.twins.entry(guest).or_insert(t);
            t.flags |= 1;
            t.id = id;
            (t.flags, t.id)
        }
        fn unregister(&mut self, guest: u32) -> Option<u8> {
            let t = self.twins.get_mut(&guest)?;
            t.flags &= !1;
            Some(t.flags)
        }
        fn call(&mut self, call: &Call) -> Reply {
            self.calls.lock().unwrap().push(call.clone());
            match call {
                Call::GetAppId => Reply::U32(APP_ID),
                Call::SetOverlayNotificationPosition(_) => Reply::Void,
                Call::LoggedOn | Call::RequestCurrentStats | Call::StoreStats => Reply::Bool(true),
                Call::GetAchievement(n) => {
                    let known = n.as_bytes().starts_with(b"ACH_");
                    Reply::Achievement(known, known.then_some(n.as_bytes().ends_with(b"FFX")))
                }
                Call::SetAchievement(_) => Reply::Bool(true),
                Call::GetAchievementDisplayAttribute(n, k) => Reply::Text(Some(
                    format!("{}:{}", n.to_string_lossy(), k.to_string_lossy()).into_bytes(),
                )),
            }
        }
    }

    fn args(words: &[u32]) -> Args {
        Args {
            ecx: 0x5000,
            stack: words.to_vec(),
            ..Args::default()
        }
    }

    fn put_str(m: &mut VecMem, at: u32, s: &str) {
        m.write_bytes(at, s.as_bytes());
        m.write(at + u32::try_from(s.len()).unwrap(), 0u8);
    }

    #[test]
    fn the_table_has_every_slot_the_game_calls_with_its_words() {
        // (interface, call offset, pushed words) at FFX.exe's call sites
        for (iface, off, words) in [
            (Iface::Utils, 0x28, 1), // 0x006234c1: push 1
            (Iface::User, 0x04, 0),  // 0x006234d1, 0x006f12e8, 0x006f17fa
            (Iface::Utils, 0x24, 0), // 0x006f1296
            (Iface::UserStats, 0x00, 0),
            (Iface::UserStats, 0x54, 1), // 0x006f142c: push 1
            (Iface::UserStats, 0x18, 2), // 0x006f1649: name, &achieved
            (Iface::UserStats, 0x30, 2), // 0x006f165e: name, "name"/"desc"
            (Iface::UserStats, 0x1c, 1), // 0x006f18cc, 0x006f193b
            (Iface::UserStats, 0x28, 0), // 0x006f18dc, 0x006f1948
        ] {
            let m = method(iface, off / 4).unwrap_or_else(|| panic!("{iface:?} +{off:#x}"));
            assert_eq!(m.words, words, "{}", m.name);
        }
        assert_eq!(METHODS.len(), 9);
        assert!(method(Iface::UserStats, 21).is_some_and(|m| m.kind == Kind::ResetAllStats));
    }

    #[test]
    fn every_import_of_ffx_exe_from_steam_api_is_bound() {
        // docs/formats/platform.md, "steam_api.dll": the nine IAT slots
        for name in [
            "SteamUserStats",
            "SteamUtils",
            "SteamAPI_UnregisterCallback",
            "SteamUser",
            "SteamAPI_Init",
            "SteamAPI_RestartAppIfNecessary",
            "SteamAPI_Shutdown",
            "SteamAPI_RegisterCallback",
            "SteamAPI_RunCallbacks",
        ] {
            assert!(TABLE.iter().any(|t| t.1 == name), "{name}");
        }
        assert_eq!(TABLE.len(), 9);
        assert!(TABLE.iter().all(|t| t.2 == Conv::Cdecl));
    }

    #[test]
    fn strings_go_in_and_out_through_the_arena() {
        let mut m = VecMem::new(0x1000, 0x2000);
        put_str(&mut m, 0x1100, "ACH_COMPLETION_OF_FFX");
        put_str(&mut m, 0x1180, "desc");
        let mut b = Bridge::new(Box::new(Fake::default()), [0x2000, 0x2010, 0x2020]);
        let mut next = 0x1800;
        let mut alloc = |n: u32| {
            let at = next;
            next += n.next_multiple_of(8);
            at
        };
        let get_attr = method(Iface::UserStats, 12).unwrap();
        let r = b.dispatch(get_attr, &mut m, &args(&[0x1100, 0x1180]), &mut alloc);
        assert_eq!(r.eax, 0x1800);
        assert_eq!(
            read_cstr(&m, r.eax).unwrap().as_bytes(),
            b"ACH_COMPLETION_OF_FFX:desc"
        );
        // the same string, the same block
        let r2 = b.dispatch(get_attr, &mut m, &args(&[0x1100, 0x1180]), &mut alloc);
        assert_eq!(r2.eax, 0x1800);
        // a null name: false, no call
        let r3 = b.dispatch(get_attr, &mut m, &args(&[0, 0x1180]), &mut alloc);
        assert_eq!(r3.eax, 0);
    }

    #[test]
    fn get_achievement_writes_the_bool_only_on_success() {
        let mut m = VecMem::new(0x1000, 0x1000);
        put_str(&mut m, 0x1100, "ACH_COMPLETION_OF_FFX");
        put_str(&mut m, 0x1140, "NOPE");
        m.write(0x1200, 0xccu8);
        m.write(0x1201, 0xccu8);
        let mut b = Bridge::new(Box::new(Fake::default()), [0; 3]);
        let mut alloc = |_| 0;
        let get = method(Iface::UserStats, 6).unwrap();
        let r = b.dispatch(get, &mut m, &args(&[0x1100, 0x1200]), &mut alloc);
        assert_eq!((r.eax, m.read::<u8>(0x1200)), (1, 1));
        let r = b.dispatch(get, &mut m, &args(&[0x1140, 0x1201]), &mut alloc);
        assert_eq!((r.eax, m.read::<u8>(0x1201)), (0, 0xcc));
    }

    #[test]
    fn reset_all_stats_never_reaches_steam() {
        let mut m = VecMem::new(0x1000, 0x100);
        let fake = Box::new(Fake::default());
        let calls = std::sync::Arc::clone(&fake.calls);
        let mut b = Bridge::new(fake, [0; 3]);
        let mut alloc = |_| 0;
        let r = b.dispatch(
            method(Iface::UserStats, 21).unwrap(),
            &mut m,
            &args(&[1]),
            &mut alloc,
        );
        assert_eq!(r.eax, 0);
        assert!(calls.lock().unwrap().is_empty(), "Steam saw ResetAllStats");
        let r = b.dispatch(
            method(Iface::Utils, 9).unwrap(),
            &mut m,
            &args(&[]),
            &mut alloc,
        );
        assert_eq!(r.eax, APP_ID);
        assert_eq!(*calls.lock().unwrap(), vec![Call::GetAppId]);
    }

    #[test]
    fn the_accessors_answer_only_for_interfaces_the_dll_gave() {
        let b = Bridge::new(Box::new(Fake::default()), [0x2000, 0x2010, 0x2020]);
        assert_eq!(b.accessor(Iface::User), 0);
        assert_eq!(b.accessor(Iface::Utils), 0x2010);
        assert_eq!(b.accessor(Iface::UserStats), 0x2020);
    }

    #[test]
    fn callback_structs_keep_their_fields() {
        // UserStatsReceived_t: game id, k_EResultOK, the user
        let mut host = vec![0u8; 0x18];
        host[0..8].copy_from_slice(&u64::from(APP_ID).to_le_bytes());
        host[8..12].copy_from_slice(&1u32.to_le_bytes());
        host[0xc..0x14].copy_from_slice(&0x0110_0001_0000_0042u64.to_le_bytes());
        let g = marshal_callback(1101, &host).unwrap();
        assert_eq!(g, host);
        // what StatsAndAchievements_OnUserStatsReceived reads: the game id
        // at +0/+4 against its own (+0x3c/+0x40), the result at +8
        assert_eq!(&g[0..4], &APP_ID.to_le_bytes());
        assert_eq!(&g[8..12], &1u32.to_le_bytes());
        // UserAchievementStored_t: the name at +9, progress at +0x8c/+0x90
        let mut host = vec![0xeeu8; 0x98];
        host[9..9 + 4].copy_from_slice(b"ACH\0");
        let g = marshal_callback(1103, &host).unwrap();
        assert_eq!(g.len(), 0x98);
        assert_eq!(&g[9..13], b"ACH\0");
        // padding is not copied (bytes 0xc..0x10 of 1102)
        let g = marshal_callback(1102, &[0xee; 0x10]).unwrap();
        assert_eq!(&g[0xc..], &[0, 0, 0, 0]);
        assert_eq!(marshal_callback(1234, &[]), None);
        // a short host struct leaves the rest zero
        assert_eq!(marshal_callback(1102, &[1; 4]).unwrap(), vec![0u8; 0x10]);
        // each layout fits its sizes
        for c in CALLBACKS {
            for &(n, o64, o32, len) in c.fields {
                assert!(
                    o64 + len <= c.size64 && o32 + len <= c.size32,
                    "{}.{n}",
                    c.name
                );
            }
        }
    }

    #[test]
    fn registration_mirrors_the_flags_and_callbacks_reach_the_run_slot() {
        let mut m = VecMem::new(0x1000, 0x1000);
        // a CCallback as the game builds one: vtable, flags 0, id, obj, fn
        let (obj, vt) = (0x1100, 0x1200);
        m.write(obj, vt);
        m.write(vt, 0x006f_1860u32); // Run(p, bool, call)
        m.write(vt + 4, 0x006f_1850u32); // Run(p)
        m.write(vt + 8, 0x006f_1590u32); // GetCallbackSizeBytes
        let mut fake = Fake::default();
        let mut stats = vec![0u8; 0x18];
        stats[8] = 1;
        fake.events = vec![
            Event {
                guest: obj,
                id: 1101,
                data: stats.clone(),
            },
            // an object the game never registered
            Event {
                guest: 0x1400,
                id: 1101,
                data: stats.clone(),
            },
        ];
        let mut b = Bridge::new(Box::new(fake), [0; 3]);
        b.register(&mut m, obj, 1101);
        assert_eq!(m.read::<u8>(obj + 4), 1);
        assert_eq!(m.read::<i32>(obj + 8), 1101);
        let d = b.run_callbacks(&m);
        assert_eq!(
            d,
            vec![Delivery {
                object: obj,
                run: 0x006f_1850,
                data: stats
            }]
        );
        b.unregister(&mut m, obj);
        assert_eq!(m.read::<u8>(obj + 4), 0);
        // nothing for an unregistered object
        let ev = [Event {
            guest: obj,
            id: 1101,
            data: vec![0; 0x18],
        }];
        assert!(plan(&m, &b.registered, &ev).is_empty());
    }

    #[test]
    fn the_twins_are_laid_out_as_steam_api64_expects() {
        // CCallbackBase on x64: vtable, flags at +8, id at +12
        assert_eq!(std::mem::offset_of!(HostCallback, flags), 8);
        assert_eq!(std::mem::offset_of!(HostCallback, id), 12);
        let mut t = HostCallback::new(0x1234, 1102, 0x10, 0b11);
        assert_eq!(t.flags(), 0b10, "registered is the DLL's to set");
        assert_eq!(t.id(), 1102);
        // the DLL's view: slot 2 is the size, slot 1 queues the struct
        let p: *mut HostCallback = &raw mut *t;
        let data = [7u8; 0x10];
        // SAFETY: a live twin and a 16-byte struct, as the DLL calls it.
        unsafe {
            assert_eq!((t.vtable.size)(p), 0x10);
            (t.vtable.run)(p, data.as_ptr().cast());
            (t.vtable.run_call_result)(p, data.as_ptr().cast(), 0, 0);
        }
        let ev: Vec<Event> = HostCallback::take_events()
            .into_iter()
            .filter(|e| e.guest == 0x1234)
            .collect();
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0].data, data);
        assert_eq!(ev[0].id, 1102);
    }

    #[test]
    fn c_strings_stop_at_nul_and_null_is_none() {
        let mut m = VecMem::new(0x1000, 0x2000);
        put_str(&mut m, 0x1000, "name");
        assert_eq!(read_cstr(&m, 0x1000).unwrap().as_bytes(), b"name");
        assert_eq!(read_cstr(&m, 0), None);
        m.write_bytes(0x1100, &[b'x'; 0x1000]);
        assert_eq!(read_cstr(&m, 0x1100), None, "longer than 4 KiB");
    }
}
