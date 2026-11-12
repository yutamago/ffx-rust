//! The saves and settings round trip (M5 stage 5; `docs/harness.md`,
//! "Saves round trip"): what one program writes into its sandboxed
//! Documents folder, decoded and compared byte for byte with what another
//! wrote.
//!
//! The files (`docs/formats/save.md`, "What a game writes"):
//!
//! - `FINAL FANTASY X/ffx_NNN`, `0x6900` bytes: a `0x40`-byte header
//!   (`MenuSave_PackSave` `0x008b3e60`), the game state block
//!   (`g_SaveData` `0x0112ca90`, `0x68c0` bytes) at `0x40`, the CRC as a
//!   word at `0x64f4` and as a `u16` at `0x1a`
//!   (`SaveDataWriteCrc` `0x00648f20`: `TkCRC(0x64b8, file + 0x40)`).
//! - `GameSetting.ini` (`Settings_Save` `0x00401d80`, `Settings_Format`
//!   `0x004022f0`): `Key=Value` lines, LF, in key order.
//!
//! Two writers of the same state may differ only in the play time (block
//! `+0xbc`, copied to header `+0x10`: `Atel_PlayTimeUpdate_0086ca20` counts
//! it from `graphicGetTime`, so its ticks fall at another point of a run in
//! every run, original code included) and the two CRCs, which cover it.
//! [`Field::may_differ`] marks those four fields and nothing else; every
//! other byte, the settings file's included, must be the same.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// A save file's size (`SaveData_Init` `0x00647fb0`).
pub const FILE_SIZE: usize = 0x6900;
/// Where the game state block starts in the file.
pub const BLOCK: usize = 0x40;
/// The block's size (`TkMenuCopyLoadedSave` `0x008b54a0`).
pub const BLOCK_SIZE: usize = 0x68c0;
/// The bytes from [`BLOCK`] the CRC covers (`SaveDataCheckCrc`
/// `0x00647d70`).
pub const CRC_LEN: usize = 0x64b8;
/// The CRC as a word, inside the covered bytes (zeroed while computing).
pub const CRC_WORD: usize = 0x64f4;
/// The header's `u16` copy of the CRC.
pub const CRC_HALF: usize = 0x1a;
/// The header's copy of the play time (`MenuSave_PackSave`: header `+0x10`
/// = block `+0xbc`).
pub const HEADER_PLAY_TIME: usize = 0x10;
/// The play time in the block, as a file offset (block `+0xbc`, `u32`
/// seconds, capped at 3599999).
pub const PLAY_TIME: usize = BLOCK + 0xbc;

/// A named range of a save file (file offsets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    pub at: usize,
    pub len: usize,
    pub name: &'static str,
    /// Size of one element for a table (`equipment[n]`), else 0.
    pub stride: usize,
    /// May differ between two writers of the same state (the play time and
    /// the CRCs over it; module docs).
    pub may_differ: bool,
}

const fn f(at: usize, len: usize, name: &'static str) -> Field {
    Field {
        at,
        len,
        name,
        stride: 0,
        may_differ: false,
    }
}

const fn b(at: usize, len: usize, name: &'static str) -> Field {
    f(BLOCK + at, len, name)
}

const fn table(at: usize, count: usize, stride: usize, name: &'static str) -> Field {
    Field {
        at: BLOCK + at,
        len: count * stride,
        name,
        stride,
        may_differ: false,
    }
}

const fn varies(mut field: Field) -> Field {
    field.may_differ = true;
    field
}

/// The fields this module names (`docs/formats/save.md`; header from
/// `MenuSave_PackSave` `0x008b3e60`). A byte inside several is named by the
/// smallest; bytes in none are named by their block offset.
pub const FIELDS: &[Field] = &[
    f(
        0x00,
        4,
        "header: save counter (g_MenuSaveCounter 0x0186627c)",
    ),
    f(0x04, 1, "header: PackSave's third argument"),
    f(0x05, 7, "header: party shown in the list"),
    f(0x0c, 1, "header: language (TOGetFFXLang)"),
    f(0x0d, 3, "header: unused"),
    varies(f(HEADER_PLAY_TIME, 4, "header: play time (block +0xbc)")),
    f(0x14, 4, "header: gil (MsGetGIL)"),
    f(0x18, 2, "header: location (AtelGetSaveDic)"),
    varies(f(CRC_HALF, 2, "header: CRC (TkCRC)")),
    f(0x1c, 2, "header: version (0x100)"),
    f(0x1e, 1, "header: Al Bhed Rikku"),
    f(0x1f, 1, "header: English config"),
    f(0x20, 0x20, "header: player name"),
    b(0x00, 2, "map"),
    b(0x02, 2, "previous map"),
    b(0x04, 4, "event jump map numbers"),
    b(0x08, 4, "event jump map ids"),
    b(0x0c, 1, "entrance"),
    b(0x0d, 1, "previous entrance"),
    b(0x0e, 2, "atel_save_dic_index"),
    b(0x10, 1, "battle scene group"),
    b(0x11, 2, "map fade type and frames"),
    b(0x13, 1, "battle_status"),
    b(0x18, 8, "flying_ship_pos"),
    b(0x20, 5, "pushed party (AtelPushMember)"),
    b(0x28, 2, "is_cam_underwater, is_map_underwater"),
    b(0x2a, 1, "save status (TkSetEvSaveStatus)"),
    b(0x2b, 1, "tk_event_new_game"),
    b(0x2c, 0x20, "affection"),
    b(0x4c, 0x50, "affection_room_flags"),
    b(0xa8, 4, "the save's music and texture video mode"),
    b(0xb4, 4, "world map marker"),
    b(0xb8, 4, "saved entrance and map (where a load returns)"),
    varies(f(PLAY_TIME, 4, "play time (Atel_PlayTimeUpdate_0086ca20)")),
    b(0xc0, 0x10, "saved_* (previous map, entrance)"),
    b(0xd1, 1, "albhed_rikku"),
    b(0xd2, 1, "drop_shadow_mode"),
    b(0xd4, 3, "atel_force_place_id"),
    b(0xd7, 1, "atel_water_btl_effect"),
    b(0xd8, 0xc, "on-memory movie"),
    b(0xe4, 4, "rand_encounter_modifiers"),
    b(0xe8, 2, "btl_end_tag_always"),
    b(0xea, 2, "sphere_monitor"),
    b(0x1ec, CRC_LEN - 4 - 0x1ec, "script variables (storage 0)"),
    b(0x5ec, 1, "soundtrack_type"),
    b(0xbec, 2, "story_progress"),
    b(0xc36, 2, "SaveSphereTipsShown"),
    b(0xc38, 0x5a, "celestials, airship, Jecht spheres, aeons"),
    b(0xc54, 4, "LastSaveSphereMap"),
    b(0xc58, 4, "LastSaveSphereEntrance"),
    b(0xcd7, 42, "battle_dialog_lines_seen"),
    b(0x3d0c, 4, "config (g_SaveMenu)"),
    b(0x3d10, 4, "unlocked_primers"),
    b(0x3d14, 4, "battle_count"),
    b(0x3d48, 4, "gil"),
    b(0x3d58, 3, "party_frontline"),
    b(0x3d5b, 17, "party_backline"),
    b(0x3d6c, 0x38, "ability_map_limit"),
    b(0x3da4, 4, "yojimbo_compatibility"),
    b(0x3da8, 4, "yojimbo_type"),
    b(0x3dac, 4, "tidus_limit_uses"),
    b(0x3db0, 4, "successful_rikku_steals"),
    b(0x3db4, 4, "bribe_gil_spent"),
    b(0x3dcc, 0x100, "event_flags"),
    b(0x3ecc, 0x200, "inventory_ids"),
    b(0x40cc, 0x100, "inventory_counts"),
    b(0x41cc, 0x20, "items_acquired"),
    b(0x41ec, 0x20, "items_used"),
    b(0x420c, 0x200, "monsters_captured"),
    b(0x440c, 0x40, "monsters_seen"),
    b(0x444c, 0x40, "monsters_defeated"),
    b(0x448c, 0x10, "key_items"),
    table(0x449c, 200, 22, "equipment"),
    table(0x55cc, 18, 0x94, "ply_saves"),
    table(0x6034, 18, 44, "commands_used"),
    table(0x634c, 18, 20, "character_names"),
    varies(f(CRC_WORD, 4, "CRC word (SaveDataWriteCrc)")),
    f(
        BLOCK + CRC_LEN,
        BLOCK_SIZE - CRC_LEN,
        "the tail (outside the CRC)",
    ),
];

/// The field a file offset falls in: the smallest of [`FIELDS`] holding
/// it, or `None` (a block byte no field names).
#[must_use]
pub fn field_at(at: usize) -> Option<&'static Field> {
    FIELDS
        .iter()
        .filter(|f| at >= f.at && at < f.at + f.len)
        .min_by_key(|f| f.len)
}

/// A file offset named for a report: the field, with the element and the
/// offset inside it (`ply_saves[0] +0x1c`), and the block or storage 0
/// offset where that helps.
#[must_use]
pub fn describe(at: usize) -> String {
    let block = at.checked_sub(BLOCK);
    match field_at(at) {
        Some(field) if field.stride > 0 => {
            let i = (at - field.at) / field.stride;
            let off = (at - field.at) % field.stride;
            format!("{}[{i}] +{off:#x}", field.name)
        }
        Some(field) if field.name.starts_with("script variables") => {
            let off = at - BLOCK;
            format!(
                "{} block +{off:#x} (storage 0 +{:#x})",
                field.name,
                off - 0x1ec
            )
        }
        Some(field) => field.name.to_owned(),
        None => block.map_or_else(|| format!("file +{at:#x}"), |o| format!("block +{o:#x}")),
    }
}

/// The table `TkCRC` builds: CRC-16 (polynomial `0x1021`) of each byte,
/// except entry 255, which stays 0 (its loop stops at 254;
/// `ffx_menu::tk::crc_table`, the port, is checked against this in the
/// tests).
fn crc_table() -> [u16; 256] {
    let mut t = [0u16; 256];
    for (i, e) in t.iter_mut().enumerate().take(0xff) {
        let mut v = u16::try_from(i).unwrap_or(0) << 8;
        for _ in 0..8 {
            v = if v & 0x8000 != 0 {
                (v << 1) ^ 0x1021
            } else {
                v << 1
            };
        }
        *e = v;
    }
    t
}

/// `TkCRC(len, data)` (`0x008b1450`): CRC-16/CCITT-FALSE inverted, with
/// the table's entry 255 zero.
#[must_use]
pub fn tk_crc(data: &[u8]) -> u16 {
    let t = crc_table();
    let mut c = 0xffffu16;
    for &x in data {
        c = (c << 8) ^ t[usize::from(c.to_be_bytes()[0] ^ x)];
    }
    !c
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    b.get(at..at + 2)
        .map_or(0, |s| u16::from_le_bytes([s[0], s[1]]))
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    b.get(at..at + 4)
        .map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// The CRC a save should hold: `TkCRC(0x64b8, file + 0x40)` with the word
/// at [`CRC_WORD`] taken as 0 (`SaveDataWriteCrc`, `SaveDataCheckCrc`).
/// `None` for a file too short.
#[must_use]
pub fn computed_crc(file: &[u8]) -> Option<u16> {
    let mut covered = file.get(BLOCK..BLOCK + CRC_LEN)?.to_vec();
    covered[CRC_WORD - BLOCK..CRC_WORD - BLOCK + 4].fill(0);
    Some(tk_crc(&covered))
}

/// What `SaveDataCheckCrc` checks: the header's `u16` equals the word, and
/// both are the computed CRC.
#[must_use]
pub fn crc_ok(file: &[u8]) -> bool {
    computed_crc(file)
        .is_some_and(|c| u16_at(file, CRC_HALF) == c && u32_at(file, CRC_WORD) == u32::from(c))
}

/// A save's header and the block words a report shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub size: usize,
    pub counter: u32,
    pub language: u8,
    pub play_time: u32,
    pub gil: u32,
    pub location: u16,
    pub crc: u16,
    pub crc_word: u32,
    pub crc_ok: bool,
    pub name: String,
    pub map: u16,
    pub entrance: u8,
    pub story: u16,
}

/// Decodes a save's header and a few block words.
#[must_use]
pub fn summary(file: &[u8]) -> Summary {
    let name = file
        .get(0x20..0x40)
        .map(|n| {
            let end = n.iter().position(|&c| c == 0).unwrap_or(n.len());
            hex_all(&n[..end])
        })
        .unwrap_or_default();
    Summary {
        size: file.len(),
        counter: u32_at(file, 0),
        language: file.get(0x0c).copied().unwrap_or(0),
        play_time: u32_at(file, HEADER_PLAY_TIME),
        gil: u32_at(file, 0x14),
        location: u16_at(file, 0x18),
        crc: u16_at(file, CRC_HALF),
        crc_word: u32_at(file, CRC_WORD),
        crc_ok: crc_ok(file),
        name,
        map: u16_at(file, BLOCK),
        entrance: file.get(BLOCK + 0xc).copied().unwrap_or(0),
        story: u16_at(file, BLOCK + 0xbec),
    }
}

/// Play time seconds as `h:mm:ss`.
#[must_use]
pub fn play_time_text(seconds: u32) -> String {
    format!(
        "{}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}

/// One field's difference between two saves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    /// File offset of the first differing byte.
    pub at: usize,
    /// Bytes from `at` to the last differing byte of the same field.
    pub len: usize,
    pub field: String,
    pub may_differ: bool,
    pub a: Vec<u8>,
    pub b: Vec<u8>,
}

/// The differences between two saves (or two blocks: `base` is the file
/// offset of `a[0]`, [`BLOCK`] for a `g_SaveData` dump), grouped by field
/// and element: one entry per field (per table element) with differing
/// bytes. Bytes past the shorter one count as differing.
#[must_use]
pub fn diff(a: &[u8], b: &[u8], base: usize) -> Vec<Difference> {
    let mut out: Vec<Difference> = Vec::new();
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i), b.get(i));
        if x == y {
            continue;
        }
        let at = base + i;
        let key = group(at);
        if let Some(last) = out.last_mut()
            && group(last.at) == key
        {
            last.len = at - last.at + 1;
            continue;
        }
        let field = field_at(at);
        out.push(Difference {
            at,
            len: 1,
            field: describe(at),
            may_differ: field.is_some_and(|f| f.may_differ),
            a: Vec::new(),
            b: Vec::new(),
        });
    }
    for d in &mut out {
        let range = d.at - base..d.at - base + d.len;
        d.a = a
            .get(range.start..range.end.min(a.len()))
            .unwrap_or(&[])
            .to_vec();
        d.b = b
            .get(range.start..range.end.min(b.len()))
            .unwrap_or(&[])
            .to_vec();
    }
    out
}

/// What groups differing bytes into one [`Difference`]: the field and,
/// for a table, the element; unnamed bytes by 16-byte line.
fn group(at: usize) -> (usize, usize) {
    match field_at(at) {
        Some(f) if f.stride > 0 => (f.at, (at - f.at) / f.stride),
        Some(f) if f.name.starts_with("script variables") => (f.at, (at - f.at) / 16),
        Some(f) => (f.at, 0),
        None => (at / 16 * 16, usize::MAX),
    }
}

/// Every byte in hex.
fn hex_all(b: &[u8]) -> String {
    b.iter().fold(String::new(), |mut s, x| {
        let _ = write!(s, "{x:02x}");
        s
    })
}

/// The first 32 bytes in hex, and the length when there are more.
fn hex(b: &[u8]) -> String {
    const MAX: usize = 32;
    let mut s = hex_all(&b[..b.len().min(MAX)]);
    if b.len() > MAX {
        let _ = write!(s, "... ({} bytes)", b.len());
    }
    s
}

/// One line per difference: offset, field, both values, and `(may
/// differ)` for the documented fields.
#[must_use]
pub fn report(diffs: &[Difference]) -> String {
    let mut s = String::new();
    for d in diffs {
        let note = if d.may_differ { " (may differ)" } else { "" };
        let _ = writeln!(
            s,
            "  +{:#06x} {}{note}: {} | {}",
            d.at,
            d.field,
            hex(&d.a),
            hex(&d.b)
        );
    }
    s
}

/// One key's difference between two settings files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingDifference {
    pub key: String,
    pub a: Option<String>,
    pub b: Option<String>,
}

/// The `Key=Value` lines of a settings file, in order; `\r` is kept in the
/// value (the game's parser splits words at spaces, `\r` among them, but a
/// byte comparison must see it).
#[must_use]
pub fn settings_lines(text: &str) -> Vec<(String, String)> {
    text.split('\n')
        .filter(|l| !l.is_empty())
        .map(|l| match l.split_once('=') {
            Some((k, v)) => (k.to_owned(), v.to_owned()),
            None => (l.to_owned(), String::new()),
        })
        .collect()
}

/// The keys whose values differ (or that one file lacks), in `a`'s order
/// then `b`'s extra keys; plus an entry `(order)` when both hold the same
/// keys and values in another order, and `(bytes)` when the files differ
/// only in bytes no key shows (line ends, a missing last newline).
#[must_use]
pub fn diff_settings(a: &[u8], b: &[u8]) -> Vec<SettingDifference> {
    if a == b {
        return Vec::new();
    }
    let (ta, tb) = (String::from_utf8_lossy(a), String::from_utf8_lossy(b));
    let (la, lb) = (settings_lines(&ta), settings_lines(&tb));
    let value = |lines: &[(String, String)], k: &str| {
        lines
            .iter()
            .find(|(key, _)| key == k)
            .map(|(_, v)| v.clone())
    };
    let mut out = Vec::new();
    for (k, v) in &la {
        let other = value(&lb, k);
        if other.as_ref() != Some(v) {
            out.push(SettingDifference {
                key: k.clone(),
                a: Some(v.clone()),
                b: other,
            });
        }
    }
    for (k, v) in &lb {
        if value(&la, k).is_none() {
            out.push(SettingDifference {
                key: k.clone(),
                a: None,
                b: Some(v.clone()),
            });
        }
    }
    if out.is_empty() {
        let note = if la == lb { "(bytes)" } else { "(order)" };
        out.push(SettingDifference {
            key: note.into(),
            a: None,
            b: None,
        });
    }
    out
}

/// A file a game added, changed or removed in its home.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    New(PathBuf),
    Changed(PathBuf),
    /// Written with the bytes it had (a newer modification time: the
    /// harness copies homes with their times).
    Rewritten(PathBuf),
    Removed(PathBuf),
}

impl Change {
    /// The file's path in the home.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::New(p) | Self::Changed(p) | Self::Rewritten(p) | Self::Removed(p) => p,
        }
    }

    /// The kind, for a report.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::New(_) => "new",
            Self::Changed(_) => "changed",
            Self::Rewritten(_) => "rewritten (same bytes)",
            Self::Removed(_) => "removed",
        }
    }
}

fn files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            files(root, &path, out)?;
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push(rel.to_owned());
        }
    }
    Ok(())
}

/// What a game wrote: the files of `home` (after the run) that `seed`
/// (what it started with, copied with its times) lacks, holds with other
/// bytes or with an older modification time, and the files gone. Paths are
/// relative, sorted.
///
/// # Errors
/// When a folder or file cannot be read.
pub fn written(seed: &Path, home: &Path) -> std::io::Result<Vec<Change>> {
    let (mut before, mut after) = (Vec::new(), Vec::new());
    files(seed, seed, &mut before)?;
    files(home, home, &mut after)?;
    let mut out = Vec::new();
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified());
    for rel in &after {
        if before.contains(rel) {
            let (old, new) = (seed.join(rel), home.join(rel));
            if std::fs::read(&old)? != std::fs::read(&new)? {
                out.push(Change::Changed(rel.clone()));
            } else if modified(&old)? != modified(&new)? {
                out.push(Change::Rewritten(rel.clone()));
            }
        } else {
            out.push(Change::New(rel.clone()));
        }
    }
    for rel in before {
        if !after.contains(&rel) {
            out.push(Change::Removed(rel));
        }
    }
    out.sort_by(|x, y| x.path().cmp(y.path()));
    Ok(out)
}

/// The settings file's path in a home.
#[must_use]
pub fn settings_file(home: &Path) -> PathBuf {
    home.join(crate::GAME_DOCUMENTS).join(crate::SETTINGS_FILE)
}

/// Whether a home-relative path is a save slot file (`.../FINAL FANTASY
/// X/ffx_NNN`).
#[must_use]
pub fn is_save(rel: &Path) -> bool {
    let parent = rel.parent().and_then(Path::file_name);
    let name = rel.file_name().and_then(|n| n.to_str()).unwrap_or("");
    parent.is_some_and(|p| p == crate::SAVE_FOLDER)
        && name.len() == 7
        && name.starts_with("ffx_")
        && name[4..].bytes().all(|c| c.is_ascii_digit())
}

/// The bytes of every harness `read`/`readp` reply to `command` in a
/// replay's replies (lines `command -> hex`, [`crate::replay::play`]), in
/// order.
#[must_use]
pub fn reply_bytes(replies: &str, command: &str) -> Vec<Vec<u8>> {
    let prefix = format!("{command} -> ");
    replies
        .lines()
        .filter_map(|l| l.strip_prefix(&prefix))
        .filter_map(|hex| {
            let hex = hex.trim();
            (0..hex.len() / 2)
                .map(|i| u8::from_str_radix(hex.get(2 * i..2 * i + 2)?, 16).ok())
                .collect()
        })
        .collect()
}

/// The round trip's four runs (`tools/saves-roundtrip.sh`): the writer
/// replay on each program, then the reader replay on each with the other
/// program's written home.
pub const STEPS: [&str; 4] = ["write-orig", "write-rust", "read-orig", "read-rust"];

/// The writer whose home a reader step starts from.
#[must_use]
pub fn source_of(step: &str) -> Option<&'static str> {
    match step {
        "read-orig" => Some("write-rust"),
        "read-rust" => Some("write-orig"),
        _ => None,
    }
}

/// The harness commands whose replies the comparison reads (in
/// `tools/replays/saves-write.txt` and `saves-read.txt`).
pub const READ_SETTINGS: &str = "readp 0x022fb504 0 0x430";
/// `g_SaveData`, the whole block.
pub const READ_BLOCK: &str = "read 0x0112ca90 0x68c0";

/// The settings the writer replay sets (`saves-write.txt`).
pub const WRITER_SETTINGS: [(&str, &str); 2] = [("VoiceVolume", "70"), ("MusicVolume", "60")];

/// One finished step: its folder (`home`, `seed`, `replies.txt`) and what
/// its game wrote.
#[derive(Debug, Clone)]
pub struct Run {
    pub step: String,
    pub home: PathBuf,
    pub changes: Vec<Change>,
    pub replies: String,
}

impl Run {
    /// Reads step `step` of `out`; `None` when it has no `home` and `seed`.
    ///
    /// # Errors
    /// When its folders cannot be read.
    pub fn load(out: &Path, step: &str) -> std::io::Result<Option<Self>> {
        let dir = out.join(step);
        let (home, seed) = (dir.join("home"), dir.join("seed"));
        if !home.is_dir() || !seed.is_dir() {
            return Ok(None);
        }
        Ok(Some(Self {
            step: step.to_owned(),
            changes: written(&seed, &home)?,
            replies: std::fs::read_to_string(dir.join("replies.txt")).unwrap_or_default(),
            home,
        }))
    }

    /// The saves the run made in new slots (not the autosave, `ffx_000`):
    /// "New Save Data".
    #[must_use]
    pub fn new_manual_saves(&self) -> Vec<&Path> {
        self.changes
            .iter()
            .filter_map(|c| match c {
                Change::New(p) if is_save(p) && !p.ends_with("ffx_000") => Some(p.as_path()),
                _ => None,
            })
            .collect()
    }

    fn file(&self, rel: &Path) -> Vec<u8> {
        std::fs::read(self.home.join(rel)).unwrap_or_default()
    }

    fn reads(&self, command: &str) -> Vec<Vec<u8>> {
        reply_bytes(&self.replies, command)
    }
}

/// The round trip's verdict: the report's lines, gate failures, missing
/// runs.
#[derive(Debug, Clone, Default)]
pub struct Verdict {
    pub text: String,
    pub failures: usize,
    pub missing: usize,
}

impl Verdict {
    fn line(&mut self, s: impl AsRef<str>) {
        self.text.push_str(s.as_ref());
        self.text.push('\n');
    }

    fn fail(&mut self, s: impl AsRef<str>) {
        self.failures += 1;
        self.line(format!("FAIL {}", s.as_ref()));
    }

    /// Two saves: `ok` when only the documented fields differ.
    fn saves(&mut self, what: &str, a: &[u8], b: &[u8], gate: bool) {
        let d = diff(a, b, 0);
        let other = d.iter().filter(|d| !d.may_differ).count();
        if d.is_empty() {
            self.line(format!("ok   {what}: identical"));
        } else if other == 0 {
            self.line(format!(
                "ok   {what}: only the play time and CRCs differ ({} | {})",
                play_time_text(summary(a).play_time),
                play_time_text(summary(b).play_time)
            ));
        } else if gate {
            self.fail(format!("{what}: {other} other fields differ"));
        } else {
            self.line(format!("note {what}: {other} other fields differ"));
        }
        if other > 0 {
            self.text.push_str(&report(&d));
        }
    }

    /// Two settings files: `ok` only when byte for byte the same.
    fn settings(&mut self, what: &str, a: &[u8], b: &[u8]) {
        let d = diff_settings(a, b);
        if d.is_empty() {
            self.line(format!("ok   {what}: identical ({} bytes)", a.len()));
            return;
        }
        self.fail(format!("{what}: {} keys differ", d.len()));
        for x in d {
            self.line(format!("  {}: {:?} | {:?}", x.key, x.a, x.b));
        }
    }

    /// Two state dumps (block or settings object): reported, gated when
    /// `gate`; block dumps may differ in the play time.
    fn dumps(
        &mut self,
        what: &str,
        a: Option<&Vec<u8>>,
        b: Option<&Vec<u8>>,
        base: usize,
        gate: bool,
    ) {
        let (Some(a), Some(b)) = (a, b) else {
            self.line(format!(
                "note {what}: a run has no such read in replies.txt"
            ));
            return;
        };
        let d = diff(a, b, base);
        let other = d.iter().filter(|d| base == 0 || !d.may_differ).count();
        if other == 0 {
            self.line(format!(
                "ok   {what}: identical{}",
                if d.is_empty() {
                    ""
                } else {
                    " but the play time"
                }
            ));
            return;
        }
        if gate {
            self.fail(format!("{what}: {other} ranges differ"));
        } else {
            self.line(format!("note {what}: {other} ranges differ"));
        }
        if base == 0 {
            // a settings object: word offsets are the keys' (docs/formats/platform.md)
            for x in &d {
                self.line(format!("  +{:#05x}: {} | {}", x.at, hex(&x.a), hex(&x.b)));
            }
        } else {
            self.text.push_str(&report(&d));
        }
    }
}

/// The value of `key` in a settings file.
fn setting(text: &[u8], key: &str) -> Option<String> {
    settings_lines(&String::from_utf8_lossy(text))
        .into_iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v)
}

fn list_run(v: &mut Verdict, run: &Run) {
    v.line(format!(
        "== {}: {} files written",
        run.step,
        run.changes.len()
    ));
    for c in &run.changes {
        v.line(format!("   {} {}", c.kind(), short(c.path())));
        if is_save(c.path()) && !matches!(c, Change::Removed(_)) {
            let s = summary(&run.file(c.path()));
            v.line(format!(
                "     size {:#x}, counter {}, play time {}, gil {}, map {:#x} entrance {}, story {}, CRC {:#06x}/{:#x} {}",
                s.size,
                s.counter,
                play_time_text(s.play_time),
                s.gil,
                s.map,
                s.entrance,
                s.story,
                s.crc,
                s.crc_word,
                if s.crc_ok { "good" } else { "BAD" }
            ));
        }
    }
    for c in &run.changes {
        let p = c.path();
        if is_save(p) && !matches!(c, Change::Removed(_)) && !crc_ok(&run.file(p)) {
            v.fail(format!("{}: {} fails SaveDataCheckCrc", run.step, short(p)));
        }
        if matches!(c, Change::Removed(_)) {
            v.fail(format!("{}: {} removed", run.step, short(p)));
        }
        if !is_save(p) && *p != *Path::new(crate::GAME_DOCUMENTS).join(crate::SETTINGS_FILE) {
            v.line(format!(
                "note {}: {} is neither a save nor the settings",
                run.step,
                short(p)
            ));
        }
    }
}

fn settings_rel() -> PathBuf {
    Path::new(crate::GAME_DOCUMENTS).join(crate::SETTINGS_FILE)
}

/// A home-relative path without the game's Documents folder, for reports.
fn short(p: &Path) -> String {
    p.strip_prefix(crate::GAME_DOCUMENTS)
        .unwrap_or(p)
        .display()
        .to_string()
}

/// The writer: a manual save made, the settings written with the
/// replay's values.
fn check_writer(v: &mut Verdict, w: &Run) {
    if w.new_manual_saves().len() != 1 {
        v.fail(format!(
            "{}: {} new manual saves, not 1",
            w.step,
            w.new_manual_saves().len()
        ));
    }
    let text = w.file(&settings_rel());
    if !w.changes.iter().any(|c| c.path() == settings_rel()) {
        v.fail(format!("{}: GameSetting.ini not written", w.step));
    }
    for (key, want) in WRITER_SETTINGS {
        if setting(&text, key).as_deref() != Some(want) {
            v.fail(format!(
                "{}: {key} is {:?}, not {want}",
                w.step,
                setting(&text, key)
            ));
        }
    }
}

/// The reader against the writer whose files it started with.
fn check_reader(v: &mut Verdict, r: &Run, w: &Run) {
    v.line(format!("== {} read {}'s files", r.step, w.step));
    let rel = settings_rel();
    match r.changes.iter().find(|c| c.path() == rel) {
        Some(Change::Rewritten(_)) => v.line(format!(
            "ok   {}: GameSetting.ini written back with the bytes it read",
            r.step
        )),
        Some(_) => v.settings(
            &format!("{} GameSetting.ini against {}'s", r.step, w.step),
            &w.file(&rel),
            &r.file(&rel),
        ),
        None => v.fail(format!(
            "{}: GameSetting.ini not written back (same time as {}'s)",
            r.step, w.step
        )),
    }
    // what Settings_Load made of the file: the writer's object after its change
    v.dumps(
        &format!("settings object read by {} against {}'s", r.step, w.step),
        w.reads(READ_SETTINGS).last(),
        r.reads(READ_SETTINGS).first(),
        0,
        true,
    );
    let (written_by_w, written_by_r) = (w.new_manual_saves(), r.new_manual_saves());
    match (written_by_w.first(), written_by_r.first()) {
        (Some(w_save), Some(r_save)) => {
            let (w_bytes, r_bytes) = (w.file(w_save), r.file(r_save));
            v.saves(
                &format!(
                    "{}'s save {} against {}'s {}",
                    r.step,
                    short(r_save),
                    w.step,
                    short(w_save)
                ),
                &w_bytes,
                &r_bytes,
                true,
            );
            v.dumps(
                &format!("the block {} loaded against {}'s save", r.step, w.step),
                w_bytes
                    .get(BLOCK..BLOCK + BLOCK_SIZE)
                    .map(<[u8]>::to_vec)
                    .as_ref(),
                r.reads(READ_BLOCK).first(),
                BLOCK,
                false,
            );
        }
        _ => v.fail(format!("{} or {} made no new manual save", r.step, w.step)),
    }
}

/// Two runs of one replay, one per program: the same files written, each
/// the same minus the documented fields.
fn check_pair(v: &mut Verdict, a: &Run, b: &Run) {
    v.line(format!("== {} against {}", a.step, b.step));
    let paths = |r: &Run| -> Vec<PathBuf> {
        r.changes
            .iter()
            .filter(|c| !matches!(c, Change::Removed(_)))
            .map(|c| c.path().to_owned())
            .collect()
    };
    let (pa, pb) = (paths(a), paths(b));
    if pa != pb {
        v.fail(format!(
            "written files differ: {:?} | {:?}",
            pa.iter().map(|p| short(p)).collect::<Vec<_>>(),
            pb.iter().map(|p| short(p)).collect::<Vec<_>>()
        ));
    }
    for p in pa.iter().filter(|p| pb.contains(p)) {
        let what = short(p);
        if is_save(p) {
            v.saves(&what, &a.file(p), &b.file(p), true);
        } else {
            v.settings(&what, &a.file(p), &b.file(p));
        }
    }
    v.dumps(
        "settings object (last read)",
        a.reads(READ_SETTINGS).last(),
        b.reads(READ_SETTINGS).last(),
        0,
        true,
    );
    v.dumps(
        "g_SaveData (block read)",
        a.reads(READ_BLOCK).first(),
        b.reads(READ_BLOCK).first(),
        BLOCK,
        false,
    );
}

/// Compares the round trip's runs in `out` (`out/<step>/`, [`STEPS`]):
/// each run's written files (saves decoded, CRCs checked); the writers
/// against each other; each reader against the writer it read (its
/// settings written back byte for byte, its settings object, its new save
/// against the writer's minus the play time and CRCs); the readers against
/// each other. Missing runs are reported and counted.
///
/// # Errors
/// When a run's folders cannot be read.
pub fn compare(out: &Path) -> std::io::Result<Verdict> {
    let mut v = Verdict::default();
    let mut runs = Vec::new();
    for step in STEPS {
        match Run::load(out, step)? {
            Some(r) => runs.push(r),
            None => {
                v.missing += 1;
                v.line(format!(
                    "== {step}: not run ({} has no home and seed)",
                    out.join(step).display()
                ));
            }
        }
    }
    let get = |s: &str| runs.iter().find(|r| r.step == s);
    for r in &runs {
        list_run(&mut v, r);
    }
    for w in ["write-orig", "write-rust"].iter().filter_map(|s| get(s)) {
        check_writer(&mut v, w);
    }
    if let (Some(a), Some(b)) = (get("write-orig"), get("write-rust")) {
        check_pair(&mut v, a, b);
    }
    for r in ["read-orig", "read-rust"] {
        if let (Some(r), Some(w)) = (get(r), source_of(r).and_then(get)) {
            check_reader(&mut v, r, w);
        }
    }
    if let (Some(a), Some(b)) = (get("read-orig"), get("read-rust")) {
        check_pair(&mut v, a, b);
    }
    let verdict = if v.failures > 0 {
        format!("FAIL: {} checks failed", v.failures)
    } else if v.missing > 0 {
        format!("INCOMPLETE: {} runs missing, no check failed", v.missing)
    } else {
        "PASS: every file written by one program reads back in the other and matches byte for byte but the play time and CRCs".into()
    };
    v.line(verdict);
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save as `MenuSave_PackSave` + `SaveDataWriteCrc` leave it.
    fn save(play_time: u32, gil: u32) -> Vec<u8> {
        let mut s = vec![0u8; FILE_SIZE];
        s[0] = 1;
        s[0x0c] = 1;
        s[HEADER_PLAY_TIME..HEADER_PLAY_TIME + 4].copy_from_slice(&play_time.to_le_bytes());
        s[PLAY_TIME..PLAY_TIME + 4].copy_from_slice(&play_time.to_le_bytes());
        s[0x14..0x18].copy_from_slice(&gil.to_le_bytes());
        s[BLOCK + 0x3d48..BLOCK + 0x3d4c].copy_from_slice(&gil.to_le_bytes());
        s[BLOCK..BLOCK + 2].copy_from_slice(&0x0107u16.to_le_bytes());
        // the tail is copied but outside the CRC
        s[FILE_SIZE - 1] = 0x5a;
        let crc = computed_crc(&s).unwrap();
        s[CRC_HALF..CRC_HALF + 2].copy_from_slice(&crc.to_le_bytes());
        s[CRC_WORD..CRC_WORD + 4].copy_from_slice(&u32::from(crc).to_le_bytes());
        s
    }

    #[test]
    fn the_crc_is_the_games() {
        let data: Vec<u8> = (0..0x1000u32)
            .map(|i| i.wrapping_mul(2_654_435_761).to_le_bytes()[1])
            .collect();
        // 0xff bytes reach the table's entry 255 (zero, unlike CCITT's)
        let ones = [0xffu8; 64];
        for d in [&data[..], &ones[..], &[][..], b"123456789"] {
            assert_eq!(tk_crc(d), ffx_menu::tk::crc(d));
        }
        assert_eq!(crc_table(), ffx_menu::tk::crc_table());
    }

    #[test]
    fn crc_check_is_the_loaders() {
        let s = save(100, 5000);
        assert!(crc_ok(&s));
        // the tail is outside the CRC
        let mut t = s.clone();
        t[FILE_SIZE - 2] ^= 1;
        assert!(crc_ok(&t));
        // a covered byte breaks it, as does a mismatched copy
        let mut t = s.clone();
        t[BLOCK + 0x100] ^= 1;
        assert!(!crc_ok(&t));
        let mut t = s.clone();
        t[CRC_HALF] ^= 1;
        assert!(!crc_ok(&t));
        assert!(!crc_ok(&s[..0x100]));
        let sum = summary(&s);
        assert_eq!(
            (sum.counter, sum.play_time, sum.gil, sum.map, sum.crc_ok),
            (1, 100, 5000, 0x0107, true)
        );
        assert_eq!(play_time_text(3 * 3600 + 7 * 60 + 9), "3:07:09");
    }

    #[test]
    fn only_play_time_and_crcs_may_differ() {
        let (a, b) = (save(100, 5000), save(101, 5000));
        let d = diff(&a, &b, 0);
        let fields: Vec<&str> = d.iter().map(|d| d.field.as_str()).collect();
        assert_eq!(
            fields,
            [
                "header: play time (block +0xbc)",
                "header: CRC (TkCRC)",
                "play time (Atel_PlayTimeUpdate_0086ca20)",
                "CRC word (SaveDataWriteCrc)"
            ]
        );
        assert!(d.iter().all(|d| d.may_differ));
        assert_eq!(
            (d[0].at, d[0].len, d[0].a.clone(), d[0].b.clone()),
            (0x10, 1, vec![100], vec![101])
        );
        // gil is not one of them
        let d = diff(&a, &save(100, 5001), 0);
        let gil: Vec<&Difference> = d.iter().filter(|d| !d.may_differ).collect();
        assert_eq!(gil.len(), 2);
        assert_eq!(gil[0].field, "header: gil (MsGetGIL)");
        assert_eq!(gil[1].field, "gil");
        assert!(
            report(&d).contains("+0x3d88 gil: 88 | 89"),
            "{}",
            report(&d)
        );
        assert_eq!(diff(&a, &a, 0), []);
    }

    #[test]
    fn fields_are_named_by_element_and_block_offset() {
        assert_eq!(
            describe(BLOCK + 0x55cc + 0x94 * 2 + 0x1c),
            "ply_saves[2] +0x1c"
        );
        assert_eq!(describe(BLOCK + 0x449c + 22 + 1), "equipment[1] +0x1");
        assert_eq!(
            describe(BLOCK + 0x300),
            "script variables (storage 0) block +0x300 (storage 0 +0x114)"
        );
        // inside storage 0, a named field wins
        assert_eq!(describe(BLOCK + 0xbec), "story_progress");
        assert_eq!(describe(BLOCK + 0x30), "affection");
        assert_eq!(describe(BLOCK + 0x100), "block +0x100");
        assert_eq!(describe(FILE_SIZE - 1), "the tail (outside the CRC)");
        // two bytes of one element are one difference, two elements two
        let a = vec![0u8; FILE_SIZE];
        let mut b = a.clone();
        let p = BLOCK + 0x55cc;
        b[p + 4] = 1;
        b[p + 8] = 1;
        b[p + 0x94] = 1;
        let d = diff(&a, &b, 0);
        assert_eq!(d.len(), 2);
        assert_eq!((d[0].at, d[0].len), (p + 4, 5));
        // a block dump against the file's block: offsets are file offsets
        let d = diff(&a[BLOCK..], &b[BLOCK..], BLOCK);
        assert_eq!(d[0].at, p + 4);
        // a short file: the missing bytes differ
        let d = diff(&a[..0x20], &a[..0x10], 0);
        assert_eq!((d[0].at, d[0].len, d[0].b.len()), (0x10, 4, 0));
        // every field lies inside the file and no unmarked field may differ
        for f in FIELDS {
            assert!(f.at + f.len <= FILE_SIZE, "{}", f.name);
        }
        assert_eq!(FIELDS.iter().filter(|f| f.may_differ).count(), 4);
    }

    #[test]
    fn settings_differences_by_key() {
        let a = b"Language=en\nVoiceVolume=70\nMusicVolume=60\n";
        assert_eq!(diff_settings(a, a), []);
        let d = diff_settings(a, b"Language=en\nVoiceVolume=100\nMusicVolume=60\nX=1\n");
        assert_eq!(
            d,
            [
                SettingDifference {
                    key: "VoiceVolume".into(),
                    a: Some("70".into()),
                    b: Some("100".into())
                },
                SettingDifference {
                    key: "X".into(),
                    a: None,
                    b: Some("1".into())
                }
            ]
        );
        // CRLF shows as a value with its \r
        let d = diff_settings(a, b"Language=en\r\nVoiceVolume=70\r\nMusicVolume=60\r\n");
        assert_eq!(d.len(), 3);
        assert_eq!(d[0].b.as_deref(), Some("en\r"));
        // same lines, another order; a missing last newline
        let d = diff_settings(a, b"VoiceVolume=70\nLanguage=en\nMusicVolume=60\n");
        assert_eq!(d[0].key, "(order)");
        let d = diff_settings(a, b"Language=en\nVoiceVolume=70\nMusicVolume=60");
        assert_eq!(d[0].key, "(bytes)");
    }

    #[test]
    fn written_files_and_save_names() {
        let dir = std::env::temp_dir().join(format!("ffx-saves-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let game = Path::new(crate::GAME_DOCUMENTS);
        let saves = game.join(crate::SAVE_FOLDER);
        let (seed, home) = (dir.join("seed"), dir.join("home"));
        for root in [&seed, &home] {
            std::fs::create_dir_all(root.join(&saves)).unwrap();
        }
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000);
        for name in ["ffx_005", "ffx_006"] {
            let p = seed.join(&saves).join(name);
            std::fs::write(&p, b"same").unwrap();
            std::fs::File::options()
                .write(true)
                .open(&p)
                .unwrap()
                .set_modified(old)
                .unwrap();
        }
        // kept as it was (the harness's copy keeps the time), and written
        // again with the same bytes
        crate::copy_file(
            &seed.join(&saves).join("ffx_005"),
            &home.join(&saves).join("ffx_005"),
        )
        .unwrap();
        std::fs::write(home.join(&saves).join("ffx_006"), b"same").unwrap();
        std::fs::write(seed.join(game).join("GameSetting.ini"), b"a=1\n").unwrap();
        std::fs::write(home.join(game).join("GameSetting.ini"), b"a=2\n").unwrap();
        std::fs::write(home.join(&saves).join("ffx_004"), b"new").unwrap();
        std::fs::write(seed.join(&saves).join("ffx_009"), b"gone").unwrap();
        let w = written(&seed, &home).unwrap();
        assert_eq!(
            w,
            [
                Change::New(saves.join("ffx_004")),
                Change::Rewritten(saves.join("ffx_006")),
                Change::Removed(saves.join("ffx_009")),
                Change::Changed(game.join("GameSetting.ini")),
            ]
        );
        assert!(is_save(&saves.join("ffx_004")));
        assert!(!is_save(&saves.join("ffx_04")));
        assert!(!is_save(&game.join("ffx_004")));
        assert!(!is_save(&game.join("GameSetting.ini")));
        assert_eq!(
            settings_file(&home),
            home.join(game).join("GameSetting.ini")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Four synthetic runs as the round trip leaves them; `tweak` changes
    /// read-orig's home before the comparison.
    fn round_trip(tag: &str, tweak: impl Fn(&Path)) -> Verdict {
        let out = std::env::temp_dir().join(format!("ffx-roundtrip-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let game = Path::new(crate::GAME_DOCUMENTS);
        let saves = game.join(crate::SAVE_FOLDER);
        let base = "Language=en\nVoiceVolume=100\nMusicVolume=100\nResolution=1280*720\nScreenMode=SM_WINDOW\n";
        let changed = base
            .replace("VoiceVolume=100", "VoiceVolume=70")
            .replace("MusicVolume=100", "MusicVolume=60");
        let object = |v: u8| {
            let mut o = vec![0u8; 0x430];
            o[0x18] = v;
            o
        };
        let block = |s: &[u8]| hex(&s[BLOCK..BLOCK + 4]);
        let mut writer_saves = Vec::new();
        for (step, play) in [("write-orig", 100), ("write-rust", 101)] {
            let dir = out.join(step);
            let seed = dir.join("seed");
            std::fs::create_dir_all(seed.join(&saves)).unwrap();
            std::fs::write(seed.join(game).join("GameSetting.ini"), base).unwrap();
            std::fs::write(seed.join(&saves).join("ffx_005"), save(50, 5000)).unwrap();
            crate::copy_tree(&seed, &dir.join("home")).unwrap();
            let home = dir.join("home");
            std::fs::write(home.join(game).join("GameSetting.ini"), &changed).unwrap();
            let s = save(play, 4000);
            std::fs::write(home.join(&saves).join("ffx_004"), &s).unwrap();
            std::fs::write(home.join(&saves).join("ffx_000"), save(play - 10, 5000)).unwrap();
            std::fs::write(
                dir.join("replies.txt"),
                format!(
                    "{READ_SETTINGS} -> {}\n{READ_BLOCK} -> {}\n",
                    hex_all(&object(70)),
                    hex_all(&s[BLOCK..BLOCK + BLOCK_SIZE])
                ),
            )
            .unwrap();
            writer_saves.push(block(&s));
        }
        for (step, play) in [("read-orig", 130), ("read-rust", 131)] {
            let dir = out.join(step);
            let from = out.join(source_of(step).unwrap()).join("home");
            crate::copy_tree(&from, &dir.join("seed")).unwrap();
            crate::copy_tree(&from, &dir.join("home")).unwrap();
            let home = dir.join("home");
            // rewritten with the same bytes, a newer time
            let ini = home.join(game).join("GameSetting.ini");
            let text = std::fs::read(&ini).unwrap();
            std::fs::File::options()
                .write(true)
                .open(&ini)
                .unwrap()
                .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(5))
                .unwrap();
            assert_eq!(std::fs::read(&ini).unwrap(), text);
            let s = save(play, 4000);
            std::fs::write(home.join(&saves).join("ffx_006"), &s).unwrap();
            std::fs::write(home.join(&saves).join("ffx_000"), save(play - 10, 4000)).unwrap();
            std::fs::write(
                dir.join("replies.txt"),
                format!(
                    "{READ_SETTINGS} -> {}\n{READ_BLOCK} -> {}\n{READ_SETTINGS} -> {}\n",
                    hex_all(&object(70)),
                    hex_all(&save(play - 20, 4000)[BLOCK..BLOCK + BLOCK_SIZE]),
                    hex_all(&object(70)),
                ),
            )
            .unwrap();
        }
        tweak(&out.join("read-orig").join("home"));
        let v = compare(&out).unwrap();
        let _ = std::fs::remove_dir_all(&out);
        assert_eq!(writer_saves[0], writer_saves[1]);
        v
    }

    #[test]
    fn a_clean_round_trip_passes() {
        let v = round_trip("clean", |_| {});

        assert_eq!((v.failures, v.missing), (0, 0), "{}", v.text);
        assert!(v.text.contains("PASS"), "{}", v.text);
        assert!(
            v.text.contains("written back with the bytes it read"),
            "{}",
            v.text
        );
        assert!(
            v.text
                .contains("only the play time and CRCs differ (0:01:40 | 0:01:41)"),
            "{}",
            v.text
        );
    }

    #[test]
    fn a_reader_that_writes_other_bytes_fails() {
        let game = Path::new(crate::GAME_DOCUMENTS);
        // the settings read back with another value
        let v = round_trip("settings", |home| {
            let ini = home.join(game).join("GameSetting.ini");
            let text = std::fs::read_to_string(&ini)
                .unwrap()
                .replace("VoiceVolume=70", "VoiceVolume=100");
            std::fs::write(&ini, text).unwrap();
        });
        assert!(v.failures >= 2, "{}", v.text);
        assert!(
            v.text.contains("VoiceVolume: Some(\"70\") | Some(\"100\")"),
            "{}",
            v.text
        );
        // the save written back with other gil
        let v = round_trip("gil", |home| {
            let p = home.join(game).join(crate::SAVE_FOLDER).join("ffx_006");
            std::fs::write(&p, save(130, 4001)).unwrap();
        });
        assert!(v.failures >= 2, "{}", v.text);
        assert!(v.text.contains("+0x3d88 gil"), "{}", v.text);
        // a broken CRC
        let v = round_trip("crc", |home| {
            let p = home.join(game).join(crate::SAVE_FOLDER).join("ffx_006");
            let mut s = save(130, 4000);
            s[CRC_WORD] ^= 1;
            std::fs::write(&p, s).unwrap();
        });
        assert!(v.text.contains("fails SaveDataCheckCrc"), "{}", v.text);
        assert!(v.failures >= 1);
    }

    #[test]
    fn missing_runs_are_counted() {
        let out = std::env::temp_dir().join(format!("ffx-roundtrip-none-{}", std::process::id()));
        let v = compare(&out).unwrap();
        assert_eq!((v.failures, v.missing), (0, 4));
        assert!(v.text.contains("INCOMPLETE"), "{}", v.text);
    }

    #[test]
    fn replies_give_their_bytes() {
        let replies =
            "wait 3 -> 12\nread 0x0112ca90 0x4 -> 0701aabb\nread 0x0112ca90 0x4 -> 0801aabb\n";
        assert_eq!(
            reply_bytes(replies, "read 0x0112ca90 0x4"),
            [vec![7, 1, 0xaa, 0xbb], vec![8, 1, 0xaa, 0xbb]]
        );
        assert_eq!(reply_bytes(replies, "read 0x0 0x4"), Vec::<Vec<u8>>::new());
    }
}
