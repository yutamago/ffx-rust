//! The Steam bridge (docs/host.md, "Steam"; src/steam.rs).
//!
//! - On the player's `FFX.exe` (`FFX_GAME_DIR`; skips without it): the
//!   game's own Steam code (`StatsAndAchievements_Create`, its callbacks,
//!   `SaveDataUnlockAchievement`, `StatsAndAchievements_ResetAll`) against
//!   a stand-in for Steam, once with the ported table and once with the
//!   originals in Unicorn; both must ask Steam the same and end with the
//!   same memory.
//! - `steam_api64.dll` (`FFX_STEAM_API`, ignored by default): the exports
//!   the bridge needs; on Windows also loaded.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use ffx_bridge::Mem as _;
use ffx_host::exec::Exec;
use ffx_host::steam::{self, APP_ID, Backend, Call, Event, Iface, Reply};
use ffx_host::table::{Args, Conv, Table};

/// `steam_api.dll`'s IAT slots (docs/formats/platform.md).
const IAT_INIT: u32 = 0x00b0_c8a8;
const IAT_RUN_CALLBACKS: u32 = 0x00b0_c8b8;
const IAT_SHUTDOWN: u32 = 0x00b0_c8b0;
const STATS_CREATE: u32 = 0x006f_1430;
const G_STATS: u32 = 0x00ce_81a8;
const G_STEAM_READY: u32 = 0x00cc_9cf5;
const UNLOCK: u32 = 0x0064_88c0;
const RESET_ALL: u32 = 0x006f_1420;
/// The 0x22 achievement records, 0x190 bytes each: +4 the API name.
const RECORDS: u32 = 0x00c3_5790;

fn exe() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir).join("FFX.exe"))
}

/// What the stand-in was asked, for both runs to compare.
#[derive(Default, Debug)]
struct Log {
    calls: Vec<Call>,
    registered: Vec<(u32, i32, u32)>,
    unregistered: Vec<u32>,
}

/// Steam as a table: achieved are the names ending in `S` or `E`; the
/// display attributes are `<key>:<name>`; `RequestCurrentStats` makes
/// the next `RunCallbacks` deliver `UserStatsReceived_t` (OK).
struct Fake {
    log: Arc<Mutex<Log>>,
    flags: HashMap<u32, (u8, i32)>,
    stats_requested: bool,
}

impl Backend for Fake {
    fn init(&mut self) -> Result<(), String> {
        Ok(())
    }
    fn has(&self, _: Iface) -> bool {
        true
    }
    fn run_callbacks(&mut self) -> Vec<Event> {
        if !std::mem::take(&mut self.stats_requested) {
            return Vec::new();
        }
        let mut data = vec![0u8; 0x18];
        data[0..8].copy_from_slice(&u64::from(APP_ID).to_le_bytes());
        data[8..12].copy_from_slice(&1u32.to_le_bytes());
        data[0xc..0x14].copy_from_slice(&0x0110_0001_0000_0042u64.to_le_bytes());
        self.flags
            .iter()
            .filter(|(_, (f, id))| f & 1 != 0 && *id == 1101)
            .map(|(&guest, _)| Event {
                guest,
                id: 1101,
                data: data.clone(),
            })
            .collect()
    }
    fn shutdown(&mut self) {}
    fn register(&mut self, guest: u32, id: i32, size: u32, flags: u8) -> (u8, i32) {
        self.log.lock().unwrap().registered.push((guest, id, size));
        self.flags.insert(guest, (flags | 1, id));
        (flags | 1, id)
    }
    fn unregister(&mut self, guest: u32) -> Option<u8> {
        self.log.lock().unwrap().unregistered.push(guest);
        let f = self.flags.get_mut(&guest)?;
        f.0 &= !1;
        Some(f.0)
    }
    fn call(&mut self, call: &Call) -> Reply {
        self.log.lock().unwrap().calls.push(call.clone());
        match call {
            Call::GetAppId => Reply::U32(APP_ID),
            Call::SetOverlayNotificationPosition(_) => Reply::Void,
            Call::LoggedOn | Call::StoreStats | Call::SetAchievement(_) => Reply::Bool(true),
            Call::RequestCurrentStats => {
                self.stats_requested = true;
                Reply::Bool(true)
            }
            Call::GetAchievement(n) => {
                let last = n.as_bytes().last().copied();
                Reply::Achievement(true, Some(matches!(last, Some(b'S' | b'E'))))
            }
            Call::GetAchievementDisplayAttribute(n, k) => Reply::Text(Some(
                format!("{}:{}", k.to_string_lossy(), n.to_string_lossy()).into_bytes(),
            )),
        }
    }
}

fn import(e: &mut Exec, slot: u32) -> u32 {
    let stub: u32 = e.mem().read(slot);
    e.call_cdecl(stub, &[]).unwrap()
}

fn read(e: &Exec, at: u32, n: usize) -> Vec<u8> {
    let mut b = vec![0; n];
    e.mem().read_bytes(at, &mut b);
    b
}

fn cstr(e: &Exec, at: u32) -> String {
    steam::read_cstr(&e.mem(), at)
        .map(|c| c.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// One run: the log, the object's bytes, the records' bytes.
fn run(table: Table) -> (Log, Vec<u8>, Vec<u8>) {
    let exe = exe().unwrap();
    let mut e = ffx_host::boot_with(&exe, table).unwrap();
    let log = Arc::new(Mutex::new(Log::default()));
    steam::use_backend(Box::new(Fake {
        log: Arc::clone(&log),
        flags: HashMap::new(),
        stats_requested: false,
    }));
    assert_eq!(import(&mut e, IAT_INIT) & 0xff, 1, "SteamAPI_Init");
    assert!(steam::present());
    e.call_cdecl(STATS_CREATE, &[]).unwrap();
    let obj: u32 = e.mem().read(G_STATS);
    assert_ne!(obj, 0, "StatsAndAchievements_Create");
    // the three CCallback objects, registered as steam_api.dll marks them
    for (off, id) in [(0, 1101), (0x14, 1102), (0x28, 1103)] {
        let cb = obj + off;
        assert_eq!(e.mem().read::<u8>(cb + 4) & 1, 1, "{id}: registered flag");
        assert_eq!(e.mem().read::<i32>(cb + 8), id);
        // the game's own GetCallbackSizeBytes (slot 2) agrees with the
        // bridge's layout
        let vt: u32 = e.mem().read(cb);
        let size_fn: u32 = e.mem().read(vt + 8);
        let args = Args {
            ecx: cb,
            ..Args::default()
        };
        let size = e.call(size_fn, Conv::Thiscall, &args).unwrap().eax;
        assert_eq!(size, steam::callback(id).unwrap().size32, "{id}: size");
    }
    assert_eq!(e.mem().read::<u8>(obj + 0x4c), 0, "stats not received yet");
    // UserStatsReceived_t through RunCallbacks to the game's handler
    import(&mut e, IAT_RUN_CALLBACKS);
    assert_eq!(e.mem().read::<u8>(obj + 0x4c), 1, "stats received");
    for i in 0..0x22u32 {
        let rec = RECORDS + i * 0x190;
        let name = cstr(&e, e.mem().read(rec + 4));
        let achieved = name.ends_with('S') || name.ends_with('E');
        assert_eq!(
            e.mem().read::<u8>(rec + 0x188),
            u8::from(achieved),
            "{name}"
        );
        assert_eq!(cstr(&e, rec + 8), format!("name:{name}"));
        assert_eq!(cstr(&e, rec + 0x88), format!("desc:{name}"));
    }
    // an unlock through the game's path (the stand-in only records it)
    let id = (1..0x21u32)
        .find(|&i| e.mem().read::<u8>(RECORDS + i * 0x190 + 0x188) == 0)
        .unwrap();
    e.mem().write(G_STEAM_READY, 1u8);
    e.call_cdecl(UNLOCK, &[id]).unwrap();
    let name = cstr(&e, e.mem().read(RECORDS + id * 0x190 + 4));
    {
        let l = log.lock().unwrap();
        let n = l.calls.len();
        assert_eq!(
            &l.calls[n - 2..],
            &[
                Call::SetAchievement(std::ffi::CString::new(name).unwrap()),
                Call::StoreStats
            ]
        );
    }
    // the debug menu's reset: refused, Steam never sees it
    let before = log.lock().unwrap().calls.len();
    let args = Args {
        ecx: obj,
        ..Args::default()
    };
    e.call(RESET_ALL, Conv::Thiscall, &args).unwrap();
    assert_eq!(
        log.lock().unwrap().calls.len(),
        before,
        "ResetAllStats reached Steam"
    );
    let obj_bytes = read(&e, obj, 0x50);
    let records = read(&e, RECORDS, 0x22 * 0x190);
    import(&mut e, IAT_SHUTDOWN);
    assert!(!steam::present());
    let log = std::mem::take(&mut *log.lock().unwrap());
    (log, obj_bytes, records)
}

#[test]
fn the_games_steam_code_runs_on_the_bridge_as_the_original() {
    if exe().is_none() {
        return;
    }
    let (ported, pobj, precs) = run(ffx_host::ports::ported());
    let (original, oobj, orecs) = run(Table::new());
    eprintln!(
        "Steam calls: {} (ported), {} (original); registered {:?}",
        ported.calls.len(),
        original.calls.len(),
        ported.registered
    );
    // RequestCurrentStats in the constructor, GetAppID, BLoggedOn ...
    assert!(ported.calls.contains(&Call::RequestCurrentStats));
    assert!(ported.calls.contains(&Call::GetAppId));
    assert_eq!(ported.calls, original.calls);
    assert_eq!(ported.registered, original.registered);
    assert_eq!(ported.unregistered, original.unregistered);
    assert_eq!(precs, orecs, "the achievement records");
    // the object: the same but for the heap addresses the two runs chose
    // alike (one boot each, the same allocations)
    assert_eq!(pobj, oobj, "the StatsAndAchievements object");
}

#[test]
#[ignore = "needs FFX_STEAM_API (the Steamworks SDK's win64 steam_api64.dll)"]
fn steam_api64_has_every_export_the_bridge_calls() {
    let Some(path) = std::env::var_os("FFX_STEAM_API") else {
        eprintln!("FFX_STEAM_API is not set; skipping");
        return;
    };
    let bytes = std::fs::read(&path).unwrap();
    let pe = goblin::pe::PE::parse(&bytes).unwrap();
    assert!(pe.is_64, "steam_api64.dll is the 64-bit one");
    let names: Vec<&str> = pe.exports.iter().filter_map(|e| e.name).collect();
    for want in steam::EXPORTS {
        assert!(names.contains(want), "no export {want}");
    }
    #[cfg(windows)]
    {
        // loads and resolves every export (no SteamAPI_Init: that needs
        // Steam and touches the account)
        steam::dll::Dll::load(std::path::Path::new(&path)).unwrap();
    }
}
