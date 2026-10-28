//! Files for the `CreateFile` family: the game folder through `std::fs`,
//! read-only (docs/host.md, "Imports"). The game sees its folder as
//! [`GAME_DIR_WIN`]; relative paths are relative to it, as the original's
//! working directory is the game folder.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// The game folder as the game sees it.
pub const GAME_DIR_WIN: &str = "C:\\FFX";

/// Open files by handle.
#[derive(Debug, Default)]
pub struct Files {
    root: PathBuf,
    open: HashMap<u32, std::fs::File>,
    next: u32,
    /// Every path opened (or tried), for the report.
    pub log: Vec<(String, bool)>,
}

impl Files {
    /// Files under `root` (the real game folder).
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            open: HashMap::new(),
            next: 0x1000,
            log: Vec::new(),
        }
    }

    /// The host path of a game path, or `None` outside the game folder.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<PathBuf> {
        let lower = name.to_ascii_lowercase();
        let prefix = GAME_DIR_WIN.to_ascii_lowercase();
        let rel = if let Some(rest) = lower.strip_prefix(&prefix) {
            &name[name.len() - rest.len()..]
        } else if name.len() > 1 && name.as_bytes()[1] == b':' {
            return None;
        } else {
            name
        };
        let mut p = self.root.clone();
        for part in rel
            .split(['\\', '/'])
            .filter(|s| !s.is_empty() && *s != ".")
        {
            if part == ".." {
                return None;
            }
            p.push(part);
        }
        Some(p)
    }

    /// Opens `name` for reading (`disposition` 3 or 4, `OPEN_EXISTING` /
    /// `OPEN_ALWAYS`); write opens and creations are refused.
    pub fn open(&mut self, name: &str, write: bool, disposition: u32) -> Option<u32> {
        let found = (!write && matches!(disposition, 3 | 4))
            .then(|| self.resolve(name))
            .flatten()
            .and_then(|p| std::fs::File::open(p).ok());
        self.log.push((name.to_string(), found.is_some()));
        let f = found?;
        self.next += 4;
        self.open.insert(self.next, f);
        Some(self.next)
    }

    /// Reads up to `n` bytes.
    pub fn read(&mut self, h: u32, n: usize) -> Option<Vec<u8>> {
        let f = self.open.get_mut(&h)?;
        let mut buf = vec![0; n];
        let mut got = 0;
        while got < n {
            match f.read(&mut buf[got..]) {
                Ok(0) => break,
                Ok(k) => got += k,
                Err(_) => return None,
            }
        }
        buf.truncate(got);
        Some(buf)
    }

    /// The file's size.
    #[must_use]
    pub fn size(&self, h: u32) -> Option<u64> {
        self.open.get(&h)?.metadata().ok().map(|m| m.len())
    }

    /// Moves the file pointer (`FILE_BEGIN` 0, `FILE_CURRENT` 1,
    /// `FILE_END` 2); returns the new position.
    pub fn seek(&mut self, h: u32, to: i64, method: u32) -> Option<u64> {
        let f = self.open.get_mut(&h)?;
        let from = match method {
            0 => SeekFrom::Start(u64::try_from(to).ok()?),
            1 => SeekFrom::Current(to),
            _ => SeekFrom::End(to),
        };
        f.seek(from).ok()
    }

    /// Closes a handle (unknown handles are fine: events, threads).
    pub fn close(&mut self, h: u32) {
        self.open.remove(&h);
    }

    /// `GetFileAttributes`: `FILE_ATTRIBUTE_DIRECTORY` (0x10),
    /// `FILE_ATTRIBUTE_NORMAL` (0x80) or `INVALID_FILE_ATTRIBUTES`.
    #[must_use]
    pub fn attributes(&self, name: &str) -> u32 {
        match self.resolve(name).and_then(|p| std::fs::metadata(p).ok()) {
            Some(m) if m.is_dir() => 0x10,
            Some(_) => 0x80,
            None => u32::MAX,
        }
    }
}
