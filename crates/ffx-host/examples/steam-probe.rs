//! The Steam bridge's 64-bit side against the real Steam, read-only
//! (docs/host.md, "Steam"): loads `steam_api64.dll` (`FFX_STEAM_API`, or
//! the first argument), starts it as `SteamAPI_Init` does for the game,
//! asks for the game's interface versions, requests the stats and prints
//! every achievement's state, name and description. It never sets, clears
//! or stores anything and never calls `ResetAllStats` (the bridge has no
//! such call).
//!
//! ```text
//! steam-probe.exe --exports            # load the DLL, resolve the exports; no Steam needed
//! steam-probe.exe                      # Steam running, logged on: init and read
//! ```
//!
//! Run it from a scratch folder holding `steam_appid.txt` (`359870`), or
//! with `SteamAppId=359870` set, never from the game folder.
//! `FFX_GAME_DIR` (a Windows path) gives the achievements' API names from
//! `FFX.exe`; without it two known ones are read.

#[cfg(windows)]
fn main() {
    use std::ffi::CString;
    use std::time::{Duration, Instant};

    use ffx_host::steam::{self, Backend, Call, Iface, Reply};

    let mut args = std::env::args().skip(1);
    let first = args.next();
    let exports_only = first.as_deref() == Some("--exports");
    let path = std::env::var_os("FFX_STEAM_API")
        .map(std::path::PathBuf::from)
        .or_else(|| first.filter(|a| !a.starts_with("--")).map(Into::into))
        .unwrap_or_else(|| "steam_api64.dll".into());
    let mut dll = match steam::dll::Dll::load(&path) {
        Ok(d) => d,
        Err(why) => {
            eprintln!("steam-probe: {why}");
            std::process::exit(2);
        }
    };
    println!(
        "loaded {}: every export the bridge calls found",
        path.display()
    );
    if exports_only {
        return;
    }
    if let Err(why) = dll.init() {
        eprintln!("steam-probe: {why}");
        std::process::exit(3);
    }
    for i in Iface::ALL {
        println!(
            "{}: {}",
            i.version(),
            if dll.has(i) { "served" } else { "NOT served" }
        );
    }
    let ask = |dll: &mut steam::dll::Dll, c: Call| dll.call(&c);
    println!("GetAppID: {:?}", ask(&mut dll, Call::GetAppId));
    println!("BLoggedOn: {:?}", ask(&mut dll, Call::LoggedOn));
    // a twin for UserStatsReceived_t, as the game's constructor registers
    let (flags, id) = dll.register(1, 1101, 0x18, 0);
    println!("RegisterCallback(1101): flags {flags:#x}, id {id}");
    println!(
        "RequestCurrentStats: {:?}",
        ask(&mut dll, Call::RequestCurrentStats)
    );
    let start = Instant::now();
    let mut received = None;
    while received.is_none() && start.elapsed() < Duration::from_secs(15) {
        received = dll.run_callbacks().into_iter().find(|e| e.id == 1101);
        std::thread::sleep(Duration::from_millis(50));
    }
    match &received {
        Some(e) => {
            let game = u64::from_le_bytes(e.data[0..8].try_into().unwrap_or_default());
            let result = u32::from_le_bytes(e.data[8..12].try_into().unwrap_or_default());
            println!(
                "UserStatsReceived_t after {:?}: game id {game}, result {result} (1 = OK)",
                start.elapsed()
            );
        }
        None => println!("no UserStatsReceived_t within 15 s"),
    }
    for name in names() {
        let Ok(n) = CString::new(name.clone()) else {
            continue;
        };
        let got = ask(&mut dll, Call::GetAchievement(n.clone()));
        let text = |dll: &mut steam::dll::Dll, key: &str| match ask(
            dll,
            Call::GetAchievementDisplayAttribute(n.clone(), CString::new(key).unwrap_or_default()),
        ) {
            Reply::Text(Some(t)) => String::from_utf8_lossy(&t).into_owned(),
            r => format!("{r:?}"),
        };
        let (title, desc) = (text(&mut dll, "name"), text(&mut dll, "desc"));
        println!("{name}: {got:?} \"{title}\" - {desc}");
    }
    dll.unregister(1);
    dll.shutdown();
    println!("SteamAPI_Shutdown");
}

/// The achievements' API names: `FFX.exe`'s records (`0x00c35790`, 0x22
/// of 0x190 bytes, +4 the name), or two known ones.
#[cfg(windows)]
fn names() -> Vec<String> {
    let known = || {
        vec![
            "ACH_COMPLETION_OF_FFX".to_string(),
            "ACH_A_JOURNEY_CATALYST".to_string(),
        ]
    };
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        return known();
    };
    let Ok(bytes) = std::fs::read(std::path::Path::new(&dir).join("FFX.exe")) else {
        return known();
    };
    let Ok(pe) = goblin::pe::PE::parse(&bytes) else {
        return known();
    };
    let base = u32::try_from(pe.image_base).unwrap_or(0x0040_0000);
    let at = |va: u32| -> Option<usize> {
        let rva = va.checked_sub(base)?;
        pe.sections.iter().find_map(|s| {
            let off = rva.checked_sub(s.virtual_address)?;
            (off < s.size_of_raw_data).then(|| (s.pointer_to_raw_data + off) as usize)
        })
    };
    let word = |va: u32| {
        at(va)
            .and_then(|o| bytes.get(o..o + 4))
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let mut out = Vec::new();
    for i in 0..0x22u32 {
        let Some(name) = word(0x00c3_5790 + i * 0x190 + 4).and_then(at) else {
            return known();
        };
        let end = bytes[name..].iter().position(|&b| b == 0).unwrap_or(0);
        out.push(String::from_utf8_lossy(&bytes[name..name + end]).into_owned());
    }
    out
}

#[cfg(not(windows))]
fn main() {
    eprintln!("steam-probe: steam_api64.dll is Windows-only (build for x86_64-pc-windows-msvc)");
}
