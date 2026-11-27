//! Steam's library, loaded at run time (docs/host.md, "Steam"): Windows'
//! `steam_api64.dll`, Linux's `libsteam_api.so`, macOS's
//! `libsteam_api.dylib`; `FFX_STEAM_API`, else beside the running
//! executable. The flat exports and the interfaces' vtable slots are the
//! same on every system (`this` first in the C convention: Win64's on
//! Windows, System V's or AAPCS64's elsewhere); what differs, the callback
//! objects' vtable order and the callback structs' packing, is
//! `crate::steam`'s (`HostCallback`, `CALLBACKS`).

use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::path::Path;

use crate::steam::{Backend, Call, Event, HostCallback, Iface, Reply};

/// The library's file name on this system.
pub const LIBRARY: &str = if cfg!(windows) {
    "steam_api64.dll"
} else if cfg!(target_os = "macos") {
    "libsteam_api.dylib"
} else {
    "libsteam_api.so"
};

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

/// The library's entry points and the interfaces it gave.
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
// are the library's, which stays loaded for the process.
unsafe impl Send for Dll {}

/// Where the library is looked for: `FFX_STEAM_API`, else [`LIBRARY`]
/// beside the running executable.
#[must_use]
pub fn dll_path() -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("FFX_STEAM_API") {
        return Some(p.into());
    }
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join(LIBRARY)))
}

/// The library as the platform's Steam, or why there is none.
///
/// # Errors
/// When the library is missing, won't load or lacks an export.
pub fn load() -> Result<Box<dyn Backend>, String> {
    let path = dll_path().ok_or_else(|| format!("no path for {LIBRARY}"))?;
    if !path.is_file() {
        return Err(format!("no {}", path.display()));
    }
    Dll::load(&path).map(|d| Box::new(d) as Box<dyn Backend>)
}

impl Dll {
    /// Loads the library at `path` and finds [`crate::steam::EXPORTS`].
    /// The library stays loaded for the process (Steam's threads and the
    /// callbacks' twins may use it until the end).
    ///
    /// # Errors
    /// When it won't load or lacks an export.
    pub fn load(path: &Path) -> Result<Self, String> {
        /// The export `name` as a `T`.
        fn f<T: Copy>(lib: &'static libloading::Library, name: &str) -> Result<T, String> {
            // SAFETY: each caller names the export's declared signature
            // (steam_api.h, steam_api_flat.h) as `T`.
            unsafe { lib.get::<T>(name.as_bytes()) }
                .map(|s| *s)
                .map_err(|_| format!("{LIBRARY} has no {name}"))
        }
        // SAFETY: Steam's library: its initialisers only set up its own
        // state.
        let lib = unsafe { libloading::Library::new(path) }
            .map_err(|e| format!("{} did not load: {e}", path.display()))?;
        let lib: &'static libloading::Library = Box::leak(Box::new(lib));
        Ok(Self {
            init: f(lib, "SteamInternal_SteamAPI_Init")?,
            run_callbacks: f(lib, "SteamAPI_RunCallbacks")?,
            shutdown: f(lib, "SteamAPI_Shutdown")?,
            register: f(lib, "SteamAPI_RegisterCallback")?,
            unregister: f(lib, "SteamAPI_UnregisterCallback")?,
            get_user: f(lib, "SteamAPI_GetHSteamUser")?,
            get_pipe: f(lib, "SteamAPI_GetHSteamPipe")?,
            client: f(lib, "SteamClient")?,
            get_user_iface: f(lib, "SteamAPI_ISteamClient_GetISteamUser")?,
            get_utils: f(lib, "SteamAPI_ISteamClient_GetISteamUtils")?,
            get_user_stats: f(lib, "SteamAPI_ISteamClient_GetISteamUserStats")?,
            ifaces: [std::ptr::null_mut(); 3],
            twins: HashMap::new(),
        })
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
        let versions = crate::steam::init_versions();
        let mut err = vec![0u8; crate::steam::ERR_MSG];
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
        let size = crate::steam::callback(id).map_or(size, |c| c.size64.min(size));
        let twin = self
            .twins
            .entry(guest)
            .or_insert_with(|| HostCallback::new(guest, id, size, flags));
        twin.id = id;
        twin.size = size;
        let p: *mut HostCallback = &raw mut **twin;
        // SAFETY: a boxed twin that outlives its registration.
        unsafe { (self.register)(p, id) };
        (twin.flags, twin.id)
    }

    fn unregister(&mut self, guest: u32) -> Option<u8> {
        let twin = self.twins.get_mut(&guest)?;
        let p: *mut HostCallback = &raw mut **twin;
        // SAFETY: a twin the DLL may hold.
        unsafe { (self.unregister)(p) };
        Some(twin.flags)
    }

    #[allow(clippy::too_many_lines)] // one arm per method
    fn call(&mut self, call: &Call) -> Reply {
        type Bool0 = unsafe extern "C" fn(*mut c_void) -> u8;
        // Each arm: the method's slot in the version's vtable and its
        // signature (`isteam*.h`; `this` first, in the C convention).
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
                Call::LoggedOn => self
                    .slot(Iface::User, 1)
                    .map_or(Reply::Bool(false), |(t, f)| {
                        Reply::Bool(std::mem::transmute::<*const c_void, Bool0>(f)(t) != 0)
                    }),
                Call::RequestCurrentStats => self
                    .slot(Iface::UserStats, 0)
                    .map_or(Reply::Bool(false), |(t, f)| {
                        Reply::Bool(std::mem::transmute::<*const c_void, Bool0>(f)(t) != 0)
                    }),
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
                Call::StoreStats => self
                    .slot(Iface::UserStats, 10)
                    .map_or(Reply::Bool(false), |(t, f)| {
                        Reply::Bool(std::mem::transmute::<*const c_void, Bool0>(f)(t) != 0)
                    }),
                Call::GetAchievementDisplayAttribute(name, key) => self
                    .slot(Iface::UserStats, 12)
                    .map_or(Reply::Text(None), |(t, f)| {
                        let f = std::mem::transmute::<
                            *const c_void,
                            unsafe extern "C" fn(
                                *mut c_void,
                                *const c_char,
                                *const c_char,
                            ) -> *const c_char,
                        >(f);
                        Reply::Text(Self::text(f(t, name.as_ptr(), key.as_ptr())))
                    }),
            }
        }
    }
}
