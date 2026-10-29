//! Drives the running game through the `ffx-hook` test harness
//! (`docs/harness.md`): launches `FFX.exe` from WSL with
//! `tools/run-game.sh --instance <n> --harness <port> --background`, then
//! sends it one command per line over `127.0.0.1`. Every launch takes a
//! slot of the pool ([`slot`]): its own game folder, port, Documents
//! folder and output directory, so several games run side by side.
//!
//! ```no_run
//! let mut game = ffx_harness::Game::launch(&ffx_harness::Options::default())?;
//! game.wait(60)?;
//! game.press("cross", 6)?;
//! let shot = game.screenshot()?;
//! game.quit()?;
//! # Ok::<(), ffx_harness::Error>(())
//! ```

pub mod pacing;
pub mod replay;
pub mod screens;
pub mod slot;

use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("the harness answered: {0}")]
    Harness(String),
    #[error("{0}")]
    Launch(String),
    #[error("bad screenshot: {0}")]
    Image(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;

/// How to start the game.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)] // independent launch switches
pub struct Options {
    /// The repository (for `tools/run-game.sh`).
    pub repo: PathBuf,
    /// The harness port; `None` (the default) uses the slot's,
    /// [`base_port`] + slot.
    pub port: Option<u16>,
    /// Show the window without taking the focus.
    pub background: bool,
    /// How long to wait for the harness to answer.
    pub startup: Duration,
    /// The game's Documents folder: settings and saves are read and
    /// written there, never in the user's own (unless [`Home::Real`]).
    pub home: Home,
    /// Copy the user's saves into the sandboxed home before each launch (a
    /// fresh copy, so every run starts from the same saves). The settings
    /// are never the user's: the sandbox has its own
    /// ([`SANDBOX_SETTINGS`]).
    pub copy_user_data: bool,
    /// Save files to put into the sandboxed home after that copy:
    /// `(slot name, source file)`, the name as the game's (`ffx_001`; entry
    /// 1 of the load screen is the first manual save). Lets a test run from
    /// a given save, independent of the user's.
    pub saves: Vec<(String, PathBuf)>,
    /// Mod folders to put into the game instance's `mods/` (as
    /// `mods/<folder name>`), for this launch only. These are the only mods
    /// a test game loads: the game folder's own (the user's) are left out.
    pub mods: Vec<PathBuf>,
    /// Seed of the random number generators (instead of the clock), so
    /// encounters, battles and drops repeat. `None` keeps the clock.
    pub seed: Option<u32>,
    /// Run frames back to back (no frame limiter; game time still advances
    /// a frame interval per frame).
    pub fast: bool,
    /// `FFX_GFX` for the graphics layer (`1`: tracking and captures; see
    /// `ffx-hook`'s `gfx`), or `None` for off.
    pub gfx: Option<String>,
    /// Run the external renderer (`--renderer`).
    pub renderer: bool,
    /// Game time never follows real time (`--virtual-clock`, with `fast`).
    pub virtual_clock: bool,
    /// Hooks not to install (`--hook-disable`): names, or `replacements`
    /// for every pure replacement (the original code runs).
    pub hook_disable: Option<String>,
    /// The Rust map path (`--rust-map`, `FFX_RUST_MAP=1`; turns the
    /// graphics layer on too): the map world's passes are drawn through
    /// `ffx-draw` (`ffx-hook`'s `gfx::scene`, `docs/draw.md`).
    /// `FFX_RUST_MAP=build` (passed on to the game) has the scene builder
    /// draw the supported maps (`gfx::map_build`).
    pub rust_map: bool,
    /// Settings merged over [`SANDBOX_SETTINGS`] (`docs/harness.md`,
    /// "Sandbox settings"): a name, `tools/sandbox/<name>.ini`, or a path
    /// (relative to the repository). Its `key=value` lines replace the
    /// default file's and add keys it lacks. `None` (the default unless
    /// `FFX_SETTINGS` names one) keeps the default file alone.
    pub settings: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            repo: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            port: None,
            background: true,
            startup: Duration::from_secs(90),
            home: Home::Instance,
            copy_user_data: true,
            saves: Vec::new(),
            mods: Vec::new(),
            seed: Some(1),
            fast: true,
            gfx: std::env::var("FFX_GFX").ok().filter(|v| !v.is_empty()),
            renderer: std::env::var("FFX_RENDERER").is_ok_and(|v| v == "1"),
            virtual_clock: false,
            hook_disable: std::env::var("FFX_HOOK_DISABLE")
                .ok()
                .filter(|v| !v.is_empty()),
            rust_map: std::env::var("FFX_RUST_MAP").is_ok_and(|v| v == "1" || v == "build"),
            settings: std::env::var("FFX_SETTINGS").ok().filter(|v| !v.is_empty()),
        }
    }
}

/// Where the game's Documents folder is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Home {
    /// The slot's own sandbox, [`slot_dir`]`/home`.
    Instance,
    /// This folder (made afresh at each launch).
    Dir(PathBuf),
    /// The user's real Documents folder (settings and saves are changed).
    Real,
}

/// The first slot's harness port (`FFX_HARNESS_PORT`, default 27100); slot
/// `n` listens on this + `n`.
#[must_use]
pub fn base_port() -> u16 {
    std::env::var("FFX_HARNESS_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(27100)
}

/// The harness's own folder: `FFX_HARNESS_DIR`, else `~/ffx-work/harness`
/// (not `/tmp`, which a WSL shutdown empties).
#[must_use]
pub fn work_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("FFX_HARNESS_DIR") {
        return PathBuf::from(d);
    }
    let home = std::env::var_os("HOME").map_or_else(std::env::temp_dir, PathBuf::from);
    home.join("ffx-work/harness")
}

/// Slot `n`'s output directory (WSL side): [`work_dir`]`/<n>`, for
/// screenshots, its sandboxed home and anything a test writes.
#[must_use]
pub fn slot_dir(n: u32) -> PathBuf {
    work_dir().join(n.to_string())
}

/// Game instance `n`'s folder (`tools/instance.sh`).
#[must_use]
pub fn instance_dir(n: u32) -> PathBuf {
    if let Some(d) = std::env::var_os("FFX_INSTANCES_DIR") {
        return PathBuf::from(d).join(n.to_string());
    }
    let home = std::env::var_os("HOME").map_or_else(std::env::temp_dir, PathBuf::from);
    home.join("ffx-work/instances").join(n.to_string())
}

/// A screenshot: 8-bit RGB, rows top first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<u8>,
}

impl Image {
    /// Parses the harness's 32-bit bottom-up BMP.
    ///
    /// # Errors
    /// Fails on anything else.
    pub fn from_bmp(b: &[u8]) -> Result<Self> {
        let word = |o: usize| {
            b.get(o..o + 4)
                .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
                .ok_or(Error::Image("truncated"))
        };
        if b.get(..2) != Some(b"BM") || b.get(28..30) != Some(&[32, 0]) {
            return Err(Error::Image("not a 32-bit BMP"));
        }
        let (offset, width, height) = (word(10)? as usize, word(18)? as usize, word(22)? as usize);
        let mut rgb = Vec::with_capacity(3 * width * height);
        for y in (0..height).rev() {
            let row = b
                .get(offset + 4 * width * y..offset + 4 * width * (y + 1))
                .ok_or(Error::Image("truncated"))?;
            for px in row.as_chunks::<4>().0 {
                rgb.extend([px[2], px[1], px[0]]);
            }
        }
        Ok(Self { width, height, rgb })
    }

    /// The mean colour of the rectangle at (`left`, `top`).
    #[must_use]
    pub fn mean(&self, left: usize, top: usize, width: usize, height: usize) -> [f32; 3] {
        let mut sum = [0u64; 3];
        let mut count = 0u64;
        for row in top..(top + height).min(self.height) {
            for col in left..(left + width).min(self.width) {
                let at = 3 * (row * self.width + col);
                for (total, &value) in sum.iter_mut().zip(&self.rgb[at..at + 3]) {
                    *total += u64::from(value);
                }
                count += 1;
            }
        }
        #[allow(clippy::cast_precision_loss)] // pixel sums
        sum.map(|total| total as f32 / count.max(1) as f32)
    }

    /// Writes the image as PNG (uncompressed deflate blocks, no dependency).
    ///
    /// # Errors
    /// Fails on write errors.
    pub fn save_png(&self, path: &Path) -> Result<()> {
        let mut raw = Vec::with_capacity((3 * self.width + 1) * self.height);
        for row in self.rgb.chunks(3 * self.width) {
            raw.push(0);
            raw.extend(row);
        }
        let mut z = vec![0x78, 0x01];
        for (i, block) in raw.chunks(0xffff).enumerate() {
            let last = (i + 1) * 0xffff >= raw.len();
            z.push(u8::from(last));
            let len = u16::try_from(block.len()).expect("block size");
            z.extend(len.to_le_bytes());
            z.extend((!len).to_le_bytes());
            z.extend(block);
        }
        let (mut a, mut b) = (1u32, 0u32);
        for &x in &raw {
            a = (a + u32::from(x)) % 65521;
            b = (b + a) % 65521;
        }
        z.extend((b << 16 | a).to_be_bytes());
        let chunk = |out: &mut Vec<u8>, kind: &[u8], data: &[u8]| {
            out.extend(u32::try_from(data.len()).expect("chunk size").to_be_bytes());
            let start = out.len();
            out.extend(kind);
            out.extend(data);
            out.extend(crc32(&out[start..]).to_be_bytes());
        };
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = Vec::new();
        ihdr.extend(u32::try_from(self.width).expect("width").to_be_bytes());
        ihdr.extend(u32::try_from(self.height).expect("height").to_be_bytes());
        ihdr.extend([8, 2, 0, 0, 0]);
        chunk(&mut png, b"IHDR", &ihdr);
        chunk(&mut png, b"IDAT", &z);
        chunk(&mut png, b"IEND", &[]);
        Ok(std::fs::write(path, png)?)
    }
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                0xedb8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A running game with the harness. A game this process launched is ended
/// when the value is dropped (a failing test does not leave it running).
pub struct Game {
    stream: BufReader<TcpStream>,
    shot_dir: PathBuf,
    port: u16,
    /// The slot (instance) this game runs in, when launched.
    instance: Option<u32>,
    /// The length of the instance's `ffx-hook.log` before the launch.
    log_start: u64,
    owned: bool,
    log: Vec<String>,
    /// The game slot while this launched the game ([`slot`]).
    slot: Option<slot::Slot>,
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = self.end();
    }
}

/// The game's folder under Documents.
const GAME_DOCUMENTS: &str = "Square Enix/FINAL FANTASY X&X-2 HD Remaster";

/// The user's Documents folder, as a WSL path.
///
/// # Errors
/// Fails when Windows cannot be asked.
pub fn user_documents() -> Result<PathBuf> {
    let out = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-Command",
            "[Environment]::GetFolderPath('MyDocuments')",
        ])
        .output()?;
    let windows = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let out = Command::new("wslpath").arg(&windows).output()?;
    if !out.status.success() || windows.is_empty() {
        return Err(Error::Launch("cannot find the Documents folder".into()));
    }
    Ok(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

/// The folder the sandbox's saves are copied from: the user's game folder
/// under Documents, or `FFX_SAVES_FROM` (a WSL path to a copy of that folder
/// with the same layout). A second machine sets it to the main machine's
/// saves, so its replays load the saves they were written for and its own
/// user's saves are not even read.
fn saves_source() -> Result<PathBuf> {
    match std::env::var_os("FFX_SAVES_FROM").filter(|v| !v.is_empty()) {
        Some(dir) => Ok(PathBuf::from(dir)),
        None => Ok(user_documents()?.join(GAME_DOCUMENTS)),
    }
}

/// Copies a file with its modification and access times. The load and save
/// lists sort the saves by their modification time to the second
/// (`SaveDataMachine_ScanFiles` `0x006f0a70`, `SortSaveTable_006f0fb0`):
/// copies made all in one second tied, and which one sorted first then
/// varied from launch to launch (menu-save's list entry 1).
fn copy_file(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::copy(from, to)?;
    let meta = std::fs::metadata(from)?;
    let mut times = std::fs::FileTimes::new().set_modified(meta.modified()?);
    if let Ok(accessed) = meta.accessed() {
        times = times.set_accessed(accessed);
    }
    std::fs::File::options()
        .write(true)
        .open(to)?
        .set_times(times)
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            copy_file(&entry.path(), &target)?;
        }
    }
    Ok(())
}

/// The saves' folder under the game's Documents folder.
const SAVE_FOLDER: &str = "FINAL FANTASY X";

/// The sandbox's settings file in the repository (`docs/harness.md`,
/// "Sandbox settings"): every test game starts from it, never from the
/// user's.
pub const SANDBOX_SETTINGS: &str = "tools/sandbox/GameSetting.ini";

/// The file a settings name stands for: `tools/sandbox/<name>.ini`, or
/// `name` itself (relative to `repo`) when it has a `/` or ends in `.ini`.
#[must_use]
pub fn settings_path(repo: &Path, name: &str) -> PathBuf {
    if name.contains(['/', '\\']) || name.ends_with(".ini") {
        repo.join(name)
    } else {
        repo.join("tools/sandbox").join(format!("{name}.ini"))
    }
}

/// The `key=value` lines of a settings file, without `#` comments and blank
/// lines.
fn settings_lines(path: &Path) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Error::Launch(format!("sandbox settings {}: {e}", path.display())))?;
    Ok(text
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.trim_start().starts_with('#') && !l.trim().is_empty())
        .map(str::to_owned)
        .collect())
}

/// `base`'s lines with each key `extra` sets replaced in place, and
/// `extra`'s other keys appended in its order.
fn merge_settings(base: &[String], extra: &[String]) -> Vec<String> {
    let key = |l: &str| l.split_once('=').map_or(l, |(k, _)| k).trim().to_owned();
    let mut out: Vec<String> = base
        .iter()
        .map(|l| {
            let k = key(l);
            extra
                .iter()
                .rev()
                .find(|e| key(e) == k)
                .unwrap_or(l)
                .clone()
        })
        .collect();
    for e in extra {
        let k = key(e);
        if !out.iter().any(|l| key(l) == k) {
            out.push(e.clone());
        }
    }
    out
}

/// The settings file's name in the game's Documents folder.
const SETTINGS_FILE: &str = "GameSetting.ini";

/// Empties the sandbox Documents folder and copies the user's saves into it
/// (their game folder without `GameSetting.ini`), then the chosen saves,
/// and writes the sandbox's own settings (`settings`, the repository's
/// [`SANDBOX_SETTINGS`]). The user's own files are only read.
fn prepare_home(
    home: &Path,
    settings: &Path,
    overlay: Option<&Path>,
    copy: bool,
    saves: &[(String, PathBuf)],
) -> Result<()> {
    if let Ok(real) = user_documents()
        && home.starts_with(&real)
    {
        return Err(Error::Launch(format!(
            "{} is inside the user's Documents folder",
            home.display()
        )));
    }
    if home.exists() {
        std::fs::remove_dir_all(home)?;
    }
    std::fs::create_dir_all(home)?;
    if copy {
        let source = saves_source()?;
        if source.is_dir() {
            copy_tree(&source, &home.join(GAME_DOCUMENTS))?;
        }
    }
    // The user's settings (texture quality, key bindings, ...) are not the
    // tests' business: the sandbox has its own.
    let target = home.join(GAME_DOCUMENTS).join(SETTINGS_FILE);
    std::fs::create_dir_all(home.join(GAME_DOCUMENTS))?;
    // Without the file's `#` comment lines: the game's own parser
    // (`Settings_Load` `0x00401cb0`) knows only `key=value` lines.
    let mut merged = settings_lines(settings)?;
    if let Some(overlay) = overlay {
        merged = merge_settings(&merged, &settings_lines(overlay)?);
    }
    let mut lines = String::new();
    for l in merged {
        lines.push_str(&l);
        lines.push('\n');
    }
    std::fs::write(&target, lines)?;
    let folder = home.join(GAME_DOCUMENTS).join(SAVE_FOLDER);
    for (name, source) in saves {
        if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
            return Err(Error::Launch(format!("bad save name {name:?}")));
        }
        std::fs::create_dir_all(&folder)?;
        copy_file(source, &folder.join(name))
            .map_err(|e| Error::Launch(format!("save {name} from {}: {e}", source.display())))?;
    }
    test_display(&target)
}

/// Tests run in a 1280x720 window (the sandbox's settings say so too):
/// screenshots keep one size and the desktop stays usable. Their sound
/// plays at 5% of the master volume (`FFX_TEST_VOLUME`, 0-100): several
/// games may run at once. Only the sandbox's settings are changed, never
/// the user's.
fn test_display(settings: &Path) -> Result<()> {
    let old = std::fs::read_to_string(settings)?;
    let volume = std::env::var("FFX_TEST_VOLUME")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .map_or(5, |v| v.min(100));
    let mut out = String::new();
    for line in old.lines() {
        if !line.starts_with("Resolution=")
            && !line.starts_with("ScreenMode=")
            && !line.starts_with("MasterVolume=")
            && !line.starts_with("Mute=")
        {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("Resolution=1280*720\nScreenMode=SM_WINDOW\n");
    let _ = writeln!(out, "Mute=Off\nMasterVolume={volume}");
    std::fs::create_dir_all(settings.parent().unwrap_or(settings))?;
    std::fs::write(settings, out)?;
    Ok(())
}

/// A WSL path as Windows sees it.
fn windows_path(path: &Path) -> Result<String> {
    let out = Command::new("wslpath").arg("-w").arg(path).output()?;
    if !out.status.success() {
        return Err(Error::Launch(format!(
            "wslpath failed for {}",
            path.display()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

impl Game {
    /// Starts the game (installing the hook DLL) and connects.
    ///
    /// # Errors
    /// Fails when the launcher fails or the harness does not answer in
    /// time.
    pub fn launch(options: &Options) -> Result<Self> {
        // Take a slot of the pool (waiting for one), unless a caller
        // already holds one for us (`game-slot run`).
        let (number, slot) = match slot::held() {
            Some(n) => (n, None),
            None => {
                let s = slot::Slot::acquire()?;
                (s.number(), Some(s))
            }
        };
        let port = options
            .port
            .unwrap_or_else(|| base_port().saturating_add(u16::try_from(number).unwrap_or(0)));
        Self::end_orphan(port)?;
        let mut cmd = Command::new(options.repo.join("tools/run-game.sh"));
        cmd.arg("--instance").arg(number.to_string());
        cmd.arg("--harness").arg(port.to_string());
        for dir in &options.mods {
            cmd.arg("--mod").arg(dir);
        }
        // The launcher refuses to start a game when someone else has the slot.
        cmd.env("FFX_GAME_SLOT_HELD", "1");
        if options.fast {
            cmd.arg("--fast");
        }
        if let Some(seed) = options.seed {
            cmd.arg("--seed").arg(seed.to_string());
        }
        if options.virtual_clock {
            cmd.arg("--virtual-clock");
        }
        if let Some(list) = &options.hook_disable {
            cmd.arg("--hook-disable").arg(list);
        }
        let home = match &options.home {
            Home::Instance => Some(slot_dir(number).join("home")),
            Home::Dir(dir) => Some(dir.clone()),
            Home::Real => None,
        };
        if let Some(home) = home {
            let overlay = options
                .settings
                .as_deref()
                .map(|name| settings_path(&options.repo, name));
            prepare_home(
                &home,
                &options.repo.join(SANDBOX_SETTINGS),
                overlay.as_deref(),
                options.copy_user_data,
                &options.saves,
            )?;
            cmd.arg("--home").arg(home);
        } else if !options.saves.is_empty() {
            return Err(Error::Launch(
                "saves are only put into a sandboxed home".into(),
            ));
        }
        if options.background {
            cmd.arg("--background");
        }
        if let Some(gfx) = &options.gfx {
            cmd.env("FFX_GFX", gfx).arg("--gfx");
        }
        if options.renderer {
            cmd.arg("--renderer");
        }
        if options.rust_map {
            cmd.arg("--rust-map");
        }
        let log_start =
            std::fs::metadata(instance_dir(number).join("ffx-hook.log")).map_or(0, |m| m.len());
        let status = cmd.status()?;
        if !status.success() {
            return Err(Error::Launch(format!("tools/run-game.sh: {status}")));
        }
        let start = Instant::now();
        loop {
            // Ready once the first frame is presented, during the start-up
            // screens: the first update (the frame boundary, where the
            // commands that touch the game run) comes later, when the
            // memochek logo starts, which in fast mode lasts about 120
            // frames (a quarter of a second): the loads must be in time to
            // press Start there.
            if let Ok(mut game) = Self::connect(port)
                && game.presents().is_ok_and(|f| f > 0)
            {
                game.owned = true;
                game.slot = slot;
                game.instance = Some(number);
                game.log_start = log_start;
                game.shot_dir = slot_dir(number);
                std::fs::create_dir_all(&game.shot_dir)?;
                return Ok(game);
            }
            if start.elapsed() > options.startup {
                return Err(Error::Launch("the harness did not answer".into()));
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// Ends a harness game left answering on `port` (its test was stopped
    /// before it could end it). The slot is ours, so such a game is an
    /// orphan; games started without the harness never answer on the port
    /// and are left alone. A game still shutting down gets 15 s first.
    fn end_orphan(port: u16) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(15);
        while Self::connect(port).is_ok_and(|mut g| {
            g.stream
                .get_mut()
                .set_read_timeout(Some(Duration::from_secs(2)))
                .is_ok()
                && g.command("ping").is_ok()
        }) {
            if Instant::now() > deadline {
                if let Ok(mut orphan) = Self::connect(port) {
                    eprintln!("game slot: ending an orphaned game on port {port}");
                    let _ = orphan.command("quit");
                }
                std::thread::sleep(Duration::from_secs(3));
                if Self::connect(port).is_ok() {
                    return Err(Error::Launch(format!(
                        "a game already answers on port {port} and did not quit"
                    )));
                }
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        Ok(())
    }

    /// The harness port.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// The slot (game instance) of a launched game.
    #[must_use]
    pub fn instance(&self) -> Option<u32> {
        self.instance
    }

    /// The folder the game runs from: its instance folder.
    #[must_use]
    pub fn game_dir(&self) -> Option<PathBuf> {
        self.instance.map(instance_dir)
    }

    /// The hook's log of this game (`ffx-hook.log` in its folder).
    #[must_use]
    pub fn log_path(&self) -> Option<PathBuf> {
        self.game_dir().map(|d| d.join("ffx-hook.log"))
    }

    /// What the hook logged since this game was launched (its own lines:
    /// the log is the instance's).
    #[must_use]
    pub fn new_log(&self) -> String {
        use std::io::{Read as _, Seek as _};
        let Some(path) = self.log_path() else {
            return String::new();
        };
        let mut text = Vec::new();
        if let Ok(mut f) = std::fs::File::open(path)
            && f.seek(std::io::SeekFrom::Start(self.log_start)).is_ok()
        {
            let _ = f.read_to_end(&mut text);
        }
        String::from_utf8_lossy(&text).into_owned()
    }

    /// The slot's output directory (WSL side): screenshots go here.
    #[must_use]
    pub fn out_dir(&self) -> &Path {
        &self.shot_dir
    }

    /// Connects to a game that is already running.
    ///
    /// # Errors
    /// Fails when nothing listens on the port.
    pub fn connect(port: u16) -> Result<Self> {
        let addr = ("127.0.0.1", port);
        let stream = std::net::ToSocketAddrs::to_socket_addrs(&addr)?
            .next()
            .map(|a| TcpStream::connect_timeout(&a, Duration::from_secs(2)))
            .ok_or_else(|| Error::Launch("no address".into()))??;
        stream.set_read_timeout(Some(Duration::from_secs(600)))?;
        // Each command goes out at once as one segment (see the hook's
        // `serve`; `docs/harness.md`, "Command latency").
        stream.set_nodelay(true)?;
        let shot_dir = work_dir();
        std::fs::create_dir_all(&shot_dir)?;
        Ok(Self {
            stream: BufReader::new(stream),
            shot_dir,
            port,
            instance: None,
            log_start: 0,
            owned: false,
            log: Vec::new(),
            slot: None,
        })
    }

    /// Sends one command; returns the reply after `ok`.
    ///
    /// # Errors
    /// Fails on `err` replies and broken connections.
    pub fn command(&mut self, line: &str) -> Result<String> {
        self.stream
            .get_mut()
            .write_all(format!("{line}\n").as_bytes())?;
        let mut reply = String::new();
        if self.stream.read_line(&mut reply)? == 0 {
            return Err(Error::Harness("connection closed".into()));
        }
        let reply = reply.trim_end();
        match reply.strip_prefix("ok") {
            Some(rest) => Ok(rest.trim_start().to_owned()),
            None => Err(Error::Harness(reply.to_owned())),
        }
    }

    /// Updates the game's main thread has finished: the frame boundary it
    /// stands at (`docs/harness.md`, "Frames").
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn frame(&mut self) -> Result<u32> {
        self.command("frame")?
            .parse()
            .map_err(|_| Error::Harness("frame count".into()))
    }

    /// Images presented so far, the start-up screens' included (which
    /// come before the first update).
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn presents(&mut self) -> Result<u32> {
        let stats = self.command("stats")?;
        let mut words = stats.split_whitespace();
        words
            .by_ref()
            .find(|w| *w == "presents")
            .and_then(|_| words.next())
            .and_then(|n| n.parse().ok())
            .ok_or_else(|| Error::Harness(format!("no present count in {stats:?}")))
    }

    /// Waits `frames` frames.
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn wait(&mut self, frames: u32) -> Result<()> {
        self.command(&format!("wait {frames}")).map(drop)
    }

    /// Holds PS2 buttons (`cross`, `up+circle`, …) for `frames`, then waits
    /// as long again.
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn press(&mut self, buttons: &str, frames: u32) -> Result<()> {
        self.command(&format!("press {buttons} {frames}")).map(drop)
    }

    /// Holds keyboard scan codes (for the Flash menus) for `frames`, then
    /// waits as long again.
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn key(&mut self, codes: &[u8], frames: u32) -> Result<()> {
        let codes: Vec<String> = codes.iter().map(|c| format!("{c:02x}")).collect();
        self.command(&format!("key {} {frames}", codes.join(",")))
            .map(drop)
    }

    /// Reads game memory at an `FFX.exe` address.
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn read(&mut self, address: u32, len: usize) -> Result<Vec<u8>> {
        hex(&self.command(&format!("read {address:#x} {len}"))?)
    }

    /// Reads `len` bytes at a process address.
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn read_abs(&mut self, address: u32, len: usize) -> Result<Vec<u8>> {
        hex(&self.command(&format!("readabs {address:#x} {len}"))?)
    }

    /// Reads `len` bytes at `*pointer + offset`.
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn read_through(&mut self, pointer: u32, offset: u32, len: usize) -> Result<Vec<u8>> {
        hex(&self.command(&format!("readp {pointer:#x} {offset:#x} {len}"))?)
    }

    /// Files the game opened since the last call (`open <path>` or
    /// `mod <path>`).
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn events(&mut self) -> Result<Vec<String>> {
        let n: usize = self
            .command("events")?
            .parse()
            .map_err(|_| Error::Harness("event count".into()))?;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let mut line = String::new();
            self.stream.read_line(&mut line)?;
            out.push(line.trim_end().to_owned());
        }
        self.log.extend(out.iter().cloned());
        Ok(out)
    }

    /// Every event read so far, by any call.
    #[must_use]
    pub fn opened(&self) -> &[String] {
        &self.log
    }

    /// Waits until the game opens a file whose path contains `needle`,
    /// checking every `step` frames for at least `limit` frames and
    /// [`MIN_WAIT`] of real time. Returns the event.
    ///
    /// # Errors
    /// Fails on timeout.
    pub fn wait_for_open(&mut self, needle: &str, step: u32, limit: u32) -> Result<String> {
        let mut deadline = Deadline::new(self, limit)?;
        let seen = self.log.len();
        loop {
            if let Some(e) = self.events()?.into_iter().find(|e| e.contains(needle)) {
                return Ok(e);
            }
            if deadline.passed(self, &format!("a file with {needle:?}"))? {
                let last: Vec<&str> = self.log[seen..]
                    .iter()
                    .rev()
                    .take(5)
                    .map(String::as_str)
                    .collect();
                return Err(Error::Harness(format!(
                    "no file with {needle:?} opened ({} events meanwhile, the last: {last:?})",
                    self.log.len() - seen
                )));
            }
            self.wait(step)?;
        }
    }

    /// The frame about to be presented.
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn screenshot(&mut self) -> Result<Image> {
        let path = self.shot_dir.join("shot.bmp");
        self.command(&format!("shot {}", windows_path(&path)?))?;
        Image::from_bmp(&std::fs::read(&path)?)
    }

    /// Disconnects and leaves the game running.
    pub fn detach(mut self) {
        self.owned = false;
    }

    /// Ends the game (also when it was only connected to).
    ///
    /// # Errors
    /// Fails when the command cannot be sent.
    pub fn quit(mut self) -> Result<()> {
        self.owned = true;
        self.end()
    }

    fn end(&mut self) -> Result<()> {
        if !self.owned {
            return Ok(());
        }
        self.owned = false;
        self.stream.get_mut().write_all(b"quit\n")?;
        // The process ends, which closes the connection.
        self.stream
            .get_mut()
            .set_read_timeout(Some(Duration::from_secs(20)))?;
        let mut rest = String::new();
        let _ = self.stream.read_line(&mut rest);
        Ok(())
    }
}

/// The least real time a wait for the game gives up after, whatever its
/// frame limit. Frames run back to back in fast mode, but the asset loader
/// works on its own thread in real time: loads take more frames when
/// several games share the machine (`docs/harness.md`, "Parallel games").
pub const MIN_WAIT: Duration = Duration::from_secs(60);

/// A wait's end: `frames` frames from now and [`MIN_WAIT`], whichever is
/// later. Says on stderr when the real-time floor is what keeps it going.
pub struct Deadline {
    start_frame: u32,
    frames: u32,
    start: Instant,
    said: bool,
}

impl Deadline {
    /// Starts counting now.
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn new(game: &mut Game, frames: u32) -> Result<Self> {
        Ok(Self {
            start_frame: game.frame()?,
            frames,
            start: Instant::now(),
            said: false,
        })
    }

    /// Whether the wait for `what` is over.
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn passed(&mut self, game: &mut Game, what: &str) -> Result<bool> {
        let frames = game.frame()?.saturating_sub(self.start_frame);
        if frames <= self.frames {
            return Ok(false);
        }
        let elapsed = self.start.elapsed();
        if elapsed < MIN_WAIT {
            if !self.said {
                eprintln!(
                    "harness: still waiting for {what} after {frames} frames ({:.1} s)",
                    elapsed.as_secs_f64()
                );
                self.said = true;
            }
            return Ok(false);
        }
        Ok(true)
    }
}

fn hex(s: &str) -> Result<Vec<u8>> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16))
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| Error::Harness(format!("not hex: {s}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bmp_round_trip_through_the_parser() {
        // 2×1, bottom-up, BGRA.
        let mut bmp = b"BM".to_vec();
        bmp.extend(62u32.to_le_bytes());
        bmp.extend([0; 4]);
        bmp.extend(54u32.to_le_bytes());
        bmp.extend(40u32.to_le_bytes());
        bmp.extend(2u32.to_le_bytes());
        bmp.extend(1u32.to_le_bytes());
        bmp.extend(1u16.to_le_bytes());
        bmp.extend(32u16.to_le_bytes());
        bmp.extend([0; 24]);
        bmp.extend([3, 2, 1, 255, 6, 5, 4, 255]);
        let img = Image::from_bmp(&bmp).unwrap();
        assert_eq!((img.width, img.height), (2, 1));
        assert_eq!(img.rgb, [1, 2, 3, 4, 5, 6]);
        assert_eq!(img.mean(0, 0, 2, 1), [2.5, 3.5, 4.5]);
    }

    #[test]
    fn crc_matches_the_png_reference() {
        assert_eq!(crc32(b"IEND"), 0xae42_6082);
    }

    /// Chosen saves land in the sandbox's save folder, settings are the
    /// repository's sandbox file with the test display; bad names are
    /// refused.
    #[test]
    fn prepare_home_puts_the_chosen_saves() {
        let dir = std::env::temp_dir().join(format!("ffx-home-test-{}", std::process::id()));
        let source = dir.join("save.bin");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&source, b"save").unwrap();
        let home = dir.join("home");
        let sandbox = Options::default().repo.join(SANDBOX_SETTINGS);
        prepare_home(
            &home,
            &sandbox,
            None,
            false,
            &[("ffx_007".into(), source.clone())],
        )
        .unwrap();
        let game = home.join(GAME_DOCUMENTS);
        assert_eq!(
            std::fs::read(game.join(SAVE_FOLDER).join("ffx_007")).unwrap(),
            b"save"
        );
        let settings = std::fs::read_to_string(game.join(SETTINGS_FILE)).unwrap();
        assert!(settings.contains("ScreenMode=SM_WINDOW"), "{settings}");
        assert_eq!(
            settings
                .lines()
                .filter(|l| l.starts_with("TextureQuality="))
                .count(),
            1,
            "{settings}"
        );
        assert!(prepare_home(&home, &sandbox, None, false, &[("../x".into(), source)]).is_err());
        // an overlay replaces keys in place and appends new ones
        let overlay = dir.join("over.ini");
        std::fs::write(
            &overlay,
            "# comment\nHDAO=High\nQuality=VQ_CUSTOM\nNewKey=1\n",
        )
        .unwrap();
        prepare_home(&home, &sandbox, Some(&overlay), false, &[]).unwrap();
        let settings = std::fs::read_to_string(game.join(SETTINGS_FILE)).unwrap();
        let keys: Vec<&str> = settings
            .lines()
            .filter_map(|l| l.split_once('=').map(|(k, _)| k))
            .collect();
        assert!(
            settings.contains("HDAO=High\n") && !settings.contains("HDAO=Off"),
            "{settings}"
        );
        assert!(settings.contains("Quality=VQ_CUSTOM\n"), "{settings}");
        assert!(!settings.contains('#'), "{settings}");
        assert_eq!(keys.iter().filter(|k| **k == "HDAO").count(), 1);
        assert!(
            keys.iter().position(|k| *k == "Quality") < keys.iter().position(|k| *k == "HDAO"),
            "in place: {settings}"
        );
        assert_eq!(keys.iter().filter(|k| **k == "NewKey").count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_names() {
        let repo = Path::new("/r");
        assert_eq!(
            settings_path(repo, "hdao"),
            Path::new("/r/tools/sandbox/hdao.ini")
        );
        assert_eq!(settings_path(repo, "x/y.ini"), Path::new("/r/x/y.ini"));
    }
}
