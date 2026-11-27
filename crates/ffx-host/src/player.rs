//! A player's run: what `ffx-rust` needs when it is started with no
//! arguments (a double-click, Steam's launch options, a desktop entry),
//! found the way the original finds it (docs/host.md, "A player's run").
//!
//! - the game folder: `FFX_GAME_DIR`, else the current folder, else the
//!   executable's folder, else Steam's install of FINAL FANTASY X/X-2 HD
//!   Remaster (app [`STEAM_APP_ID`]) in any of Steam's libraries;
//! - the documents folder: the platform's (Windows: the one `FFX.exe`
//!   uses, so settings and saves stay shared with the original), else on
//!   Linux the Proton prefix of the game (shared with the original under
//!   Proton), else a folder of our own ([`own_documents`]);
//! - `SteamAppId`, which `steam_api` needs when Steam didn't start us.
//!
//! Everything but [`steam_roots`] and [`own_documents`] is plain path
//! logic, tested on every host.

use std::path::{Path, PathBuf};

/// Steam's app id of FINAL FANTASY X/X-2 HD Remaster.
pub const STEAM_APP_ID: &str = "359870";

/// Whether `dir` holds the game (`FFX.exe`).
#[must_use]
pub fn has_game(dir: &Path) -> bool {
    dir.join("FFX.exe").is_file()
}

/// The game folder of a player's run: `env_dir` (`FFX_GAME_DIR`) when
/// given, else the first of `cwd`, `exe_dir` that holds the game, else the
/// game in one of `steam_roots`' libraries.
#[must_use]
pub fn find_game(
    env_dir: Option<PathBuf>,
    cwd: Option<&Path>,
    exe_dir: Option<&Path>,
    steam_roots: &[PathBuf],
) -> Option<PathBuf> {
    if let Some(d) = env_dir.filter(|d| !d.as_os_str().is_empty()) {
        return Some(d);
    }
    if let Some(d) = [cwd, exe_dir].into_iter().flatten().find(|d| has_game(d)) {
        return Some(d.to_path_buf());
    }
    steam_roots.iter().find_map(|root| steam_install(root))
}

/// The game's folder under the Steam installation `root`, from its
/// libraries (`steamapps/libraryfolders.vdf`, `root` itself first).
#[must_use]
pub fn steam_install(root: &Path) -> Option<PathBuf> {
    let mut libraries = vec![root.to_path_buf()];
    if let Some(text) = child_ci(root, "steamapps").and_then(|a| read_ci(&a, "libraryfolders.vdf"))
    {
        for lib in vdf_values(&text, "path") {
            let lib = PathBuf::from(lib);
            if !libraries.contains(&lib) {
                libraries.push(lib);
            }
        }
    }
    libraries.iter().find_map(|lib| {
        let apps = child_ci(lib, "steamapps")?;
        let manifest = read_ci(&apps, &format!("appmanifest_{STEAM_APP_ID}.acf"))?;
        let dir = vdf_values(&manifest, "installdir").into_iter().next()?;
        let game = child_ci(&apps, "common")?.join(dir);
        has_game(&game).then_some(game)
    })
}

/// `dir/name` when it exists, the name matched without case (Steam on
/// Linux has both `steamapps` and `SteamApps`).
fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.exists() {
        return Some(exact);
    }
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}

/// `dir/name`'s text, the name matched without case.
fn read_ci(dir: &Path, name: &str) -> Option<String> {
    std::fs::read_to_string(child_ci(dir, name)?).ok()
}

/// Every value of `key` in a Valve `KeyValues` text (`"key"  "value"` on a
/// line), unescaped (`\\` → `\`, `\"` → `"`).
#[must_use]
pub fn vdf_values(text: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut parts = quoted(line.trim());
        if parts.next().is_some_and(|k| k.eq_ignore_ascii_case(key))
            && let Some(v) = parts.next()
        {
            out.push(v);
        }
    }
    out
}

/// The quoted strings of a line, unescaped.
fn quoted(line: &str) -> impl Iterator<Item = String> + '_ {
    let mut chars = line.chars();
    std::iter::from_fn(move || {
        chars.by_ref().find(|&c| c == '"')?;
        let mut s = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => return Some(s),
                '\\' => s.extend(chars.next()),
                c => s.push(c),
            }
        }
        None
    })
}

/// The Proton prefix's documents folder of the game at `game`
/// (`<library>/steamapps/common/<dir>` →
/// `<library>/steamapps/compatdata/359870/pfx/drive_c/users/steamuser/Documents`),
/// when Proton made it: the original's settings and saves under Proton.
#[must_use]
pub fn proton_documents(game: &Path) -> Option<PathBuf> {
    let apps = game.parent()?.parent()?;
    let docs = apps
        .join("compatdata")
        .join(STEAM_APP_ID)
        .join("pfx/drive_c/users/steamuser/Documents");
    docs.is_dir().then_some(docs)
}

/// The documents folder of a player's run on `game` when the platform has
/// none: Linux's Proton prefix of the game, else [`own_documents`].
#[must_use]
pub fn documents(platform: Option<PathBuf>, game: &Path) -> Option<PathBuf> {
    platform
        .or_else(|| {
            if cfg!(target_os = "linux") {
                proton_documents(game)
            } else {
                None
            }
        })
        .or_else(own_documents)
}

/// A documents folder of our own, for a system with no Windows documents
/// folder and no Proton prefix: `$XDG_DATA_HOME/ffx-rust/Documents`
/// (`~/.local/share/...`) on Linux, `~/Library/Application
/// Support/ffx-rust/Documents` on macOS, `%USERPROFILE%\Documents`
/// elsewhere. Created on first use.
#[must_use]
pub fn own_documents() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    let home = home.filter(|h| !h.is_empty()).map(PathBuf::from);
    let dir = if cfg!(target_os = "macos") {
        home?.join("Library/Application Support/ffx-rust/Documents")
    } else if cfg!(windows) {
        home?.join("Documents")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .filter(|d| !d.is_empty())
            .map(PathBuf::from)
            .or_else(|| home.map(|h| h.join(".local/share")))?
            .join("ffx-rust/Documents")
    };
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Steam's installations on this system, most likely first (they may not
/// exist).
#[must_use]
pub fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    roots.extend(windows_steam_path());
    let home = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from);
    if cfg!(windows) {
        for var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(p) = std::env::var_os(var) {
                roots.push(PathBuf::from(p).join("Steam"));
            }
        }
    } else if cfg!(target_os = "macos") {
        roots.extend(home.map(|h| h.join("Library/Application Support/Steam")));
    } else if let Some(h) = home {
        // the native package, the Debian/Ubuntu one, Flatpak, Snap
        roots.push(h.join(".steam/steam"));
        roots.push(h.join(".local/share/Steam"));
        roots.push(h.join(".steam/debian-installation"));
        roots.push(h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"));
        roots.push(h.join("snap/steam/common/.local/share/Steam"));
    }
    let mut seen = Vec::new();
    roots.retain(|r| {
        let key = std::fs::canonicalize(r).unwrap_or_else(|_| r.clone());
        let new = r.is_dir() && !seen.contains(&key);
        seen.push(key);
        new
    });
    roots
}

/// Steam's folder from the registry (`HKCU\Software\Valve\Steam`,
/// `SteamPath`), as the Steam client writes it.
#[cfg(windows)]
fn windows_steam_path() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_SZ, RegGetValueW};
    let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let (key, value) = (wide("Software\\Valve\\Steam"), wide("SteamPath"));
    let mut buf = vec![0u16; 1024];
    let mut bytes = u32::try_from(buf.len() * 2).unwrap_or(u32::MAX);
    // SAFETY: the strings are NUL-terminated, `buf` holds `bytes` bytes.
    let r = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &raw mut bytes,
        )
    };
    if r != 0 {
        return None;
    }
    let len = (bytes as usize / 2).saturating_sub(1).min(buf.len());
    let p = PathBuf::from(std::ffi::OsString::from_wide(&buf[..len]));
    (!p.as_os_str().is_empty()).then_some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ffx-player-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn game_at(dir: &Path) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("FFX.exe"), b"MZ").unwrap();
    }

    #[test]
    fn vdf() {
        let text = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
	}
	"1"
	{
		"path"		"/mnt/games/Steam \"x\""
	}
}"#;
        assert_eq!(
            vdf_values(text, "path"),
            ["C:\\Program Files (x86)\\Steam", "/mnt/games/Steam \"x\""]
        );
        assert_eq!(vdf_values(text, "label"), [""]);
        assert_eq!(vdf_values(text, "missing"), Vec::<String>::new());
    }

    #[test]
    fn game_folder_order() {
        let t = temp("order");
        let (cwd, exe) = (t.join("cwd"), t.join("exe"));
        std::fs::create_dir_all(&cwd).unwrap();
        game_at(&exe);
        // FFX_GAME_DIR wins, even without the game in it (boot says why)
        let env = Some(t.join("elsewhere"));
        assert_eq!(find_game(env.clone(), Some(&cwd), Some(&exe), &[]), env);
        // an empty FFX_GAME_DIR counts as unset
        assert_eq!(
            find_game(Some(PathBuf::new()), Some(&cwd), Some(&exe), &[]),
            Some(exe.clone())
        );
        assert_eq!(
            find_game(None, Some(&cwd), Some(&exe), &[]),
            Some(exe.clone())
        );
        game_at(&cwd);
        assert_eq!(find_game(None, Some(&cwd), Some(&exe), &[]), Some(cwd));
        assert_eq!(find_game(None, None, None, &[]), None);
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn steam_libraries() {
        let t = temp("steam");
        let (root, lib) = (t.join("Steam"), t.join("Library2"));
        std::fs::create_dir_all(root.join("steamapps")).unwrap();
        std::fs::write(
            root.join("steamapps/libraryfolders.vdf"),
            format!(
                "\"libraryfolders\"\n{{\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t}}\n}}\n",
                lib.display().to_string().replace('\\', "\\\\")
            ),
        )
        .unwrap();
        assert_eq!(steam_install(&root), None);
        let apps = lib.join("SteamApps");
        std::fs::create_dir_all(&apps).unwrap();
        std::fs::write(
            apps.join(format!("appmanifest_{STEAM_APP_ID}.acf")),
            "\"AppState\"\n{\n\t\"installdir\"\t\t\"FINAL FANTASY FFX&FFX-2 HD Remaster\"\n}\n",
        )
        .unwrap();
        // the manifest without the game: not found
        assert_eq!(steam_install(&root), None);
        let game = apps.join("common/FINAL FANTASY FFX&FFX-2 HD Remaster");
        game_at(&game);
        assert_eq!(steam_install(&root), Some(game.clone()));
        assert_eq!(
            find_game(None, None, None, std::slice::from_ref(&root)),
            Some(game)
        );
        let _ = std::fs::remove_dir_all(&t);
    }

    #[test]
    fn proton_prefix() {
        let t = temp("proton");
        let game = t.join("steamapps/common/FFX");
        game_at(&game);
        assert_eq!(proton_documents(&game), None);
        let docs = t.join("steamapps/compatdata/359870/pfx/drive_c/users/steamuser/Documents");
        std::fs::create_dir_all(&docs).unwrap();
        assert_eq!(proton_documents(&game), Some(docs));
        let given = t.join("given");
        assert_eq!(documents(Some(given.clone()), &game), Some(given));
        let _ = std::fs::remove_dir_all(&t);
    }
}
