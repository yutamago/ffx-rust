//! Files for the `CreateFile` family (docs/host.md, "Imports"): the game
//! folder through `std::fs`, read-only, and a documents folder of the
//! caller's (`GameSetting.ini`, the saves), read-write. The game sees its
//! folder as [`GAME_DIR_WIN`] and the documents folder as [`DOCUMENTS_WIN`];
//! relative paths are relative to the game folder, as the original's
//! working directory is the game folder.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// The game folder as the game sees it.
pub const GAME_DIR_WIN: &str = "C:\\FFX";
/// The documents folder as the game sees it (`SHGetFolderPathW`).
pub const DOCUMENTS_WIN: &str = "C:\\FFXDocuments";

/// Open files by handle.
#[derive(Debug, Default)]
pub struct Files {
    root: PathBuf,
    /// The documents folder on the host (`None`: none, nothing there
    /// opens).
    pub documents: Option<PathBuf>,
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
            documents: None,
            open: HashMap::new(),
            next: 0x1000,
            log: Vec::new(),
        }
    }

    /// The host path of a game path, and whether it is in the documents
    /// folder (writable); `None` outside both folders.
    #[must_use]
    pub fn resolve(&self, name: &str) -> Option<(PathBuf, bool)> {
        let lower = name.to_ascii_lowercase();
        let tail = |prefix: &str| {
            lower
                .strip_prefix(&prefix.to_ascii_lowercase())
                .filter(|r| r.is_empty() || r.starts_with(['\\', '/']))
                .map(|r| &name[name.len() - r.len()..])
        };
        let (mut p, rel, docs) = if let Some(rest) = tail(DOCUMENTS_WIN) {
            (self.documents.clone()?, rest, true)
        } else if let Some(rest) = tail(GAME_DIR_WIN) {
            (self.root.clone(), rest, false)
        } else if name.len() > 1 && name.as_bytes()[1] == b':' {
            return None;
        } else {
            (self.root.clone(), name, false)
        };
        for part in rel
            .split(['\\', '/'])
            .filter(|s| !s.is_empty() && *s != ".")
        {
            if part == ".." {
                return None;
            }
            p.push(part);
        }
        Some((p, docs))
    }

    /// Opens `name` (`disposition`: `CREATE_NEW` 1, `CREATE_ALWAYS` 2,
    /// `OPEN_EXISTING` 3, `OPEN_ALWAYS` 4, `TRUNCATE_EXISTING` 5). The game
    /// folder opens for reading only; the documents folder for anything.
    pub fn open(&mut self, name: &str, write: bool, disposition: u32) -> Option<u32> {
        let found = self.resolve(name).and_then(|(p, docs)| {
            if !docs {
                return (!write && matches!(disposition, 3 | 4))
                    .then(|| std::fs::File::open(&p).ok())
                    .flatten();
            }
            let mut o = std::fs::OpenOptions::new();
            o.read(true).write(write);
            match disposition {
                1 => o.write(true).create_new(true),
                2 => o.write(true).create(true).truncate(true),
                4 => o.write(true).create(true),
                5 => o.write(true).truncate(true),
                _ => &mut o,
            };
            o.open(&p).ok()
        });
        self.log.push((name.to_string(), found.is_some()));
        if found.is_none() && write {
            // a write the game expects to work (its save writer crashes on
            // purpose when `_wfopen` fails): noted at once
            crate::log::note(&format!(
                "files: {name} not opened for writing (disposition {disposition}; host path {:?})",
                self.resolve(name).map(|(p, _)| p)
            ));
        }
        let f = found?;
        self.next += 4;
        self.open.insert(self.next, f);
        Some(self.next)
    }

    /// Writes `data`; the bytes written.
    pub fn write(&mut self, h: u32, data: &[u8]) -> Option<usize> {
        use std::io::Write as _;
        let f = self.open.get_mut(&h)?;
        f.write_all(data).ok()?;
        Some(data.len())
    }

    /// `CreateDirectory` in the documents folder (elsewhere: refused).
    pub fn create_dir(&mut self, name: &str) -> bool {
        match self.resolve(name) {
            Some((p, true)) => std::fs::create_dir_all(p).is_ok(),
            _ => false,
        }
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
        match self
            .resolve(name)
            .and_then(|(p, _)| std::fs::metadata(p).ok())
        {
            Some(m) if m.is_dir() => 0x10,
            Some(_) => 0x80,
            None => u32::MAX,
        }
    }
}
