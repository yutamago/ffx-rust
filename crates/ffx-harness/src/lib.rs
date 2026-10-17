//! Drives the running game through the `ffx-hook` test harness
//! (`docs/harness.md`): launches `FFX.exe` from WSL with
//! `tools/run-game.sh --harness <port> --background`, then sends it one
//! command per line over `127.0.0.1`.
//!
//! ```no_run
//! let mut game = ffx_harness::Game::launch(&ffx_harness::Options::default())?;
//! game.wait(60)?;
//! game.press("cross", 6)?;
//! let shot = game.screenshot()?;
//! game.quit()?;
//! # Ok::<(), ffx_harness::Error>(())
//! ```

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
    pub port: u16,
    /// Show the window without taking the focus.
    pub background: bool,
    /// How long to wait for the harness to answer.
    pub startup: Duration,
    /// The game's Documents folder: settings and saves are read and
    /// written there, never in the user's own. `None` uses the real one.
    pub home: Option<PathBuf>,
    /// Copy the user's settings and saves into `home` before each launch
    /// (a fresh copy, so every run starts from the same saves).
    pub copy_user_data: bool,
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
}

impl Default for Options {
    fn default() -> Self {
        Self {
            repo: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
            port: std::env::var("FFX_HARNESS_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(27100),
            background: true,
            startup: Duration::from_secs(90),
            home: Some(std::env::temp_dir().join("ffx-harness/home")),
            copy_user_data: true,
            seed: Some(1),
            fast: true,
            gfx: std::env::var("FFX_GFX").ok().filter(|v| !v.is_empty()),
            renderer: std::env::var("FFX_RENDERER").is_ok_and(|v| v == "1"),
            virtual_clock: false,
            hook_disable: std::env::var("FFX_HOOK_DISABLE")
                .ok()
                .filter(|v| !v.is_empty()),
        }
    }
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

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Empties the sandbox Documents folder and copies the user's game folder
/// (settings and saves) into it. The user's own files are only read.
fn prepare_home(home: &Path, copy: bool) -> Result<()> {
    if home.exists() {
        std::fs::remove_dir_all(home)?;
    }
    std::fs::create_dir_all(home)?;
    if copy {
        let source = user_documents()?.join(GAME_DOCUMENTS);
        if source.is_dir() {
            copy_tree(&source, &home.join(GAME_DOCUMENTS))?;
        }
    }
    test_display(&home.join(GAME_DOCUMENTS).join("GameSetting.ini"))
}

/// Tests run in a 1280x720 window whatever the user plays at: screenshots
/// keep one size and the desktop stays usable. Their sound plays at 5% of
/// the master volume (`FFX_TEST_VOLUME`, 0-100): several games may run at
/// once. Only the sandbox's settings are changed, never the user's.
fn test_display(settings: &Path) -> Result<()> {
    let old = std::fs::read_to_string(settings).unwrap_or_else(|_| "Language=en\n".into());
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

/// Whether a parent process holds the game slot for this one
/// (`FFX_GAME_SLOT_HELD=1`, set by `game-slot run`).
fn slot_held() -> bool {
    std::env::var("FFX_GAME_SLOT_HELD").is_ok_and(|v| v == "1")
}

impl Game {
    /// Starts the game (installing the hook DLL) and connects.
    ///
    /// # Errors
    /// Fails when the launcher fails or the harness does not answer in
    /// time.
    pub fn launch(options: &Options) -> Result<Self> {
        // One game at a time: queue for it unless a caller already holds
        // the slot for us (`game-slot run`).
        let held = slot_held();
        let slot = if held {
            None
        } else {
            Some(slot::Slot::acquire()?)
        };
        // A game that is still shutting down frees the port within seconds.
        let deadline = Instant::now() + Duration::from_secs(15);
        while Self::connect(options.port).is_ok_and(|mut g| {
            g.stream
                .get_mut()
                .set_read_timeout(Some(Duration::from_secs(2)))
                .is_ok()
                && g.command("ping").is_ok()
        }) {
            if Instant::now() > deadline {
                // The slot is ours, so a harness game still answering is an
                // orphan (its test was stopped before it could end it): end
                // it the way `Game::end` does. Games started without the
                // harness never answer on the port and are left alone.
                if let Ok(mut orphan) = Self::connect(options.port) {
                    eprintln!(
                        "game slot: ending an orphaned game on port {}",
                        options.port
                    );
                    let _ = orphan.command("quit");
                }
                std::thread::sleep(Duration::from_secs(3));
                if Self::connect(options.port).is_ok() {
                    return Err(Error::Launch(format!(
                        "a game already answers on port {} and did not quit",
                        options.port
                    )));
                }
                break;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        let mut cmd = Command::new(options.repo.join("tools/run-game.sh"));
        cmd.arg("--harness").arg(options.port.to_string());
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
        if let Some(home) = &options.home {
            prepare_home(home, options.copy_user_data)?;
            cmd.arg("--home").arg(home);
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
        let status = cmd.status()?;
        if !status.success() {
            return Err(Error::Launch(format!("tools/run-game.sh: {status}")));
        }
        let start = Instant::now();
        loop {
            // Ready once the first frame is presented (the frame hook runs
            // the commands that touch the game).
            if let Ok(mut game) = Self::connect(options.port)
                && game.frame().is_ok_and(|f| f > 0)
            {
                game.owned = true;
                game.slot = slot;
                return Ok(game);
            }
            if start.elapsed() > options.startup {
                return Err(Error::Launch("the harness did not answer".into()));
            }
            std::thread::sleep(Duration::from_millis(500));
        }
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
        let shot_dir = std::env::temp_dir().join("ffx-harness");
        std::fs::create_dir_all(&shot_dir)?;
        Ok(Self {
            stream: BufReader::new(stream),
            shot_dir,
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
        writeln!(self.stream.get_mut(), "{line}")?;
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

    /// Frames presented so far.
    ///
    /// # Errors
    /// See [`Game::command`].
    pub fn frame(&mut self) -> Result<u32> {
        self.command("frame")?
            .parse()
            .map_err(|_| Error::Harness("frame count".into()))
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
    /// checking every `step` frames for at most `limit` frames. Returns the
    /// event.
    ///
    /// # Errors
    /// Fails on timeout.
    pub fn wait_for_open(&mut self, needle: &str, step: u32, limit: u32) -> Result<String> {
        let start = self.frame()?;
        loop {
            if let Some(e) = self.events()?.into_iter().find(|e| e.contains(needle)) {
                return Ok(e);
            }
            if self.frame()? - start > limit {
                return Err(Error::Harness(format!("no file with {needle:?} opened")));
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
        writeln!(self.stream.get_mut(), "quit")?;
        // The process ends, which closes the connection.
        self.stream
            .get_mut()
            .set_read_timeout(Some(Duration::from_secs(20)))?;
        let mut rest = String::new();
        let _ = self.stream.read_line(&mut rest);
        Ok(())
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
}
