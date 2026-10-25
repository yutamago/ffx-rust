//! The PC platform boundary of `FFX.exe`: which imports of the window,
//! input, Steam and movie code the game calls, and from where.
//!
//! This is the census behind `docs/formats/platform.md`. It reads only the
//! executable and `symbols/ffx.sym`:
//!
//! - every import is found through its IAT slot: a base relocation in an
//!   executable section whose dword is the slot's address is an
//!   instruction that reads the slot (`call [slot]`, `jmp [slot]`,
//!   `mov reg, [slot]`). `FFX.exe` has a relocation table, so this part is
//!   exact;
//! - a `jmp [slot]` is an import thunk; its callers are the `call`/`jmp
//!   rel32` instructions aimed at it (a raw scan: a hit whose opcode byte
//!   lies inside a relocated dword is dropped);
//! - each site is given to the function holding it: the nearest function
//!   start at or before it, where starts are the `symbols/ffx.sym`
//!   functions, `call rel32` targets, code pointers stored in data
//!   sections and `push offset` immediates.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;

/// The DLLs whose every import the census lists (`docs/formats/platform.md`).
pub const PLATFORM_DLLS: &[&str] = &[
    "USER32.dll",
    "DINPUT8.dll",
    "XINPUT9_1_0.dll",
    "steam_api.dll",
    "IMM32.dll",
    "WINMM.dll",
    "SHELL32.dll",
    "SHLWAPI.dll",
    "VERSION.dll",
    "d3d11.dll",
];

/// The statically linked movie player and its codecs, as address ranges
/// (start inclusive, end exclusive) with a label. The census lists every
/// import called from inside them, whatever its DLL
/// (`docs/formats/platform.md`, "Movies").
pub const MOVIE_CODE: &[(u32, u32, &str)] = &[
    (0x0040_9370, 0x0041_e5b0, "libvpx 1.4.0 (C)"),
    (0x0041_e5b0, 0x0042_e1d0, "libwebm mkvparser"),
    (0x0042_e1d0, 0x0042_ee40, "libogg bitwise"),
    (0x006d_6970, 0x006d_aec0, "PhyFMVPlayerManager"),
    (0x00a2_6e70, 0x00a2_8890, "PVideoPlaybackWin32"),
    (0x00a3_dd20, 0x00a3_e3e0, "CDataFrameQueue, VbfMkvReader"),
    (0x00a7_c920, 0x00a8_d6be, "libvpx 1.4.0 (x86 DSP)"),
    (0x00a8_ee40, 0x00aa_51e0, "libVorbis 1.3.5"),
];

/// A section of the image.
#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub va: u32,
    pub size: u32,
    raw: usize,
    raw_size: usize,
    pub exec: bool,
}

/// One imported function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub dll: String,
    pub name: String,
    /// The IAT slot's address.
    pub slot: u32,
}

/// The instruction that reads an IAT slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `call [slot]`.
    Call,
    /// `jmp [slot]`: an import thunk (its callers are listed separately).
    Jmp,
    /// `mov reg, [slot]` (the pointer is called later through the register).
    Load,
    /// Any other instruction with the slot's address as an operand.
    Other,
}

/// A place where the game reaches an import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Site {
    /// The instruction's address.
    pub at: u32,
    pub kind: Kind,
    /// For a `call`/`jmp rel32` to an import thunk: the thunk.
    pub thunk: Option<u32>,
}

/// `FFX.exe`, parsed for the census.
pub struct Image {
    bytes: Vec<u8>,
    pub base: u32,
    pub sections: Vec<Section>,
    relocs: BTreeSet<u32>,
    pub imports: Vec<Import>,
    /// dword value -> relocated places in executable sections holding it.
    code_operands: HashMap<u32, Vec<u32>>,
    /// target -> `call`/`jmp rel32` sites aimed at it.
    branches: HashMap<u32, Vec<u32>>,
    starts: BTreeSet<u32>,
}

fn u32_le(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

impl Image {
    /// Parses a 32-bit PE image.
    ///
    /// # Errors
    /// When the bytes are not a 32-bit PE with a relocation table.
    pub fn parse(bytes: Vec<u8>) -> Result<Self, String> {
        let pe = goblin::pe::PE::parse(&bytes).map_err(|e| e.to_string())?;
        if pe.is_64 {
            return Err("a 64-bit image".into());
        }
        let base = u32::try_from(pe.image_base).map_err(|e| e.to_string())?;
        let sections: Vec<Section> = pe
            .sections
            .iter()
            .map(|s| Section {
                name: String::from_utf8_lossy(&s.name)
                    .trim_end_matches('\0')
                    .to_string(),
                va: base + s.virtual_address,
                size: s.virtual_size,
                raw: s.pointer_to_raw_data as usize,
                raw_size: s.size_of_raw_data as usize,
                exec: s.characteristics & goblin::pe::section_table::IMAGE_SCN_MEM_EXECUTE != 0,
            })
            .collect();
        let imports = pe
            .imports
            .iter()
            .map(|i| {
                Ok(Import {
                    dll: i.dll.to_string(),
                    name: i.name.to_string(),
                    slot: base + u32::try_from(i.offset).map_err(|e| e.to_string())?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut relocs = BTreeSet::new();
        let data = pe.relocation_data.as_ref().ok_or("no relocation table")?;
        for block in data.blocks() {
            let block = block.map_err(|e| e.to_string())?;
            for w in block.words() {
                let w = w.map_err(|e| e.to_string())?;
                // IMAGE_REL_BASED_HIGHLOW
                if w.reloc_type() == 3 {
                    relocs.insert(base + block.rva + u32::from(w.offset()));
                }
            }
        }
        drop(pe);
        let mut image = Image {
            bytes,
            base,
            sections,
            relocs,
            imports,
            code_operands: HashMap::new(),
            branches: HashMap::new(),
            starts: BTreeSet::new(),
        };
        image.index();
        Ok(image)
    }

    fn section(&self, va: u32) -> Option<&Section> {
        self.sections
            .iter()
            .find(|s| va >= s.va && va - s.va < s.size.max(u32::try_from(s.raw_size).unwrap_or(0)))
    }

    fn offset(&self, va: u32) -> Option<usize> {
        let s = self.section(va)?;
        let d = (va - s.va) as usize;
        (d < s.raw_size).then_some(s.raw + d)
    }

    /// The byte at `va`.
    #[must_use]
    pub fn u8_at(&self, va: u32) -> Option<u8> {
        self.bytes.get(self.offset(va)?).copied()
    }

    /// The dword at `va`.
    #[must_use]
    pub fn u32_at(&self, va: u32) -> Option<u32> {
        u32_le(&self.bytes, self.offset(va)?)
    }

    /// Whether `va` lies in an executable section.
    #[must_use]
    pub fn is_exec(&self, va: u32) -> bool {
        self.section(va).is_some_and(|s| s.exec)
    }

    /// Whether a relocated dword covers `va`.
    fn in_reloc(&self, va: u32) -> bool {
        self.relocs
            .range(va.saturating_sub(3)..=va)
            .next()
            .is_some()
    }

    fn index(&mut self) {
        let mut code_operands: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut starts = BTreeSet::new();
        for &r in &self.relocs {
            let Some(v) = self.u32_at(r) else { continue };
            if self.is_exec(r) {
                code_operands.entry(v).or_default().push(r);
                // push offset f
                if self.is_exec(v) && v % 16 == 0 && self.u8_at(r.wrapping_sub(1)) == Some(0x68) {
                    starts.insert(v);
                }
            } else if self.is_exec(v) && v % 16 == 0 {
                starts.insert(v);
            }
        }
        let mut branches: HashMap<u32, Vec<u32>> = HashMap::new();
        for s in self.sections.iter().filter(|s| s.exec) {
            let end = s.raw_size.min(s.size as usize);
            let code = &self.bytes[s.raw..s.raw + end];
            for i in 0..end.saturating_sub(4) {
                let op = code[i];
                if op != 0xe8 && op != 0xe9 {
                    continue;
                }
                let Some(rel) = u32_le(code, i + 1) else {
                    continue;
                };
                let at = s.va + u32::try_from(i).unwrap_or(0);
                let target = at.wrapping_add(5).wrapping_add(rel);
                if !(target >= s.va && target - s.va < s.size) || self.in_reloc(at) {
                    continue;
                }
                branches.entry(target).or_default().push(at);
            }
            // MSVC starts every function on 16 bytes and fills the gap
            // with int3 after the previous function's last `ret` or `jmp`
            // (0xcc bytes can also be operands, so the padding counts only
            // after one), or starts with `push ebp; mov ebp, esp`.
            let mut a = s.va.next_multiple_of(16);
            while a < s.va + u32::try_from(end).unwrap_or(0) {
                let i = (a - s.va) as usize;
                let mut p = i;
                while p > 0 && code[p - 1] == 0xcc {
                    p -= 1;
                }
                let byte = |k: usize| p.checked_sub(k).map(|j| code[j]);
                let padded = p < i
                    && (byte(1) == Some(0xc3)
                        || byte(3) == Some(0xc2)
                        || byte(5) == Some(0xe9)
                        || byte(2) == Some(0xeb));
                let frame = code.get(i..i + 3) == Some(&[0x55, 0x8b, 0xec]);
                if (padded && code[i] != 0xcc) || frame {
                    starts.insert(a);
                }
                a += 16;
            }
        }
        self.code_operands = code_operands;
        self.branches = branches;
        self.starts = starts;
    }

    /// Adds known function starts (e.g. `symbols/ffx.sym`'s).
    pub fn add_starts(&mut self, starts: impl IntoIterator<Item = u32>) {
        self.starts.extend(starts);
    }

    /// The function start at or before `va`.
    #[must_use]
    pub fn function_of(&self, va: u32) -> Option<u32> {
        self.starts.range(..=va).next_back().copied()
    }

    /// The imports of `dll` (case-insensitive), in IAT order.
    pub fn imports_of<'a>(&'a self, dll: &'a str) -> impl Iterator<Item = &'a Import> + 'a {
        self.imports
            .iter()
            .filter(move |i| i.dll.eq_ignore_ascii_case(dll))
    }

    /// Every place that reaches the import in `slot`: the instructions
    /// reading the slot, and the calls to its thunks (the thunks' own
    /// `jmp [slot]` is listed too, as `Kind::Jmp`).
    #[must_use]
    pub fn sites(&self, slot: u32) -> Vec<Site> {
        let mut out = Vec::new();
        for &r in self.code_operands.get(&slot).into_iter().flatten() {
            let b1 = self.u8_at(r.wrapping_sub(1));
            let b2 = self.u8_at(r.wrapping_sub(2));
            let (at, kind) = match (b2, b1) {
                (Some(0xff), Some(0x15)) => (r - 2, Kind::Call),
                (Some(0xff), Some(0x25)) => (r - 2, Kind::Jmp),
                (_, Some(0xa1)) => (r - 1, Kind::Load),
                (Some(0x8b), Some(m)) if m & 0xc7 == 0x05 => (r - 2, Kind::Load),
                _ => (r, Kind::Other),
            };
            out.push(Site {
                at,
                kind,
                thunk: None,
            });
            if kind == Kind::Jmp {
                for &c in self.branches.get(&at).into_iter().flatten() {
                    out.push(Site {
                        at: c,
                        kind: if self.u8_at(c) == Some(0xe8) {
                            Kind::Call
                        } else {
                            Kind::Jmp
                        },
                        thunk: Some(at),
                    });
                }
            }
        }
        out.sort_by_key(|s| s.at);
        out
    }

    /// The sites of `slot` that call it (`call [slot]`, a load of the
    /// pointer, or a call/jmp to a thunk), without the thunks themselves.
    #[must_use]
    pub fn callers(&self, slot: u32) -> Vec<Site> {
        self.sites(slot)
            .into_iter()
            .filter(|s| !(s.kind == Kind::Jmp && s.thunk.is_none()))
            .collect()
    }
}

/// `symbols/ffx.sym`'s function names by address.
#[must_use]
pub fn parse_sym(text: &str) -> BTreeMap<u32, String> {
    let mut out = BTreeMap::new();
    for line in text.lines() {
        let mut w = line.split_whitespace();
        let (Some(a), Some("function"), Some(n)) = (w.next(), w.next(), w.next()) else {
            continue;
        };
        if let Some(h) = a.strip_prefix("0x")
            && let Ok(a) = u32::from_str_radix(h, 16)
        {
            out.insert(a, n.to_string());
        }
    }
    out
}

/// One function that calls an import, with its sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller {
    pub function: u32,
    pub name: Option<String>,
    pub sites: Vec<Site>,
}

/// The callers of the import in `slot`, grouped by function.
#[must_use]
pub fn callers_by_function(image: &Image, names: &BTreeMap<u32, String>, slot: u32) -> Vec<Caller> {
    let mut by: BTreeMap<u32, Vec<Site>> = BTreeMap::new();
    for s in image.callers(slot) {
        let f = image.function_of(s.at).unwrap_or(s.at);
        by.entry(f).or_default().push(s);
    }
    by.into_iter()
        .map(|(function, sites)| Caller {
            function,
            name: names.get(&function).cloned(),
            sites,
        })
        .collect()
}

/// The label of the movie/codec range holding `va`.
#[must_use]
pub fn movie_range(va: u32) -> Option<&'static str> {
    MOVIE_CODE
        .iter()
        .find(|(s, e, _)| va >= *s && va < *e)
        .map(|r| r.2)
}

/// One caller as a table cell: the function, then its sites, or (`brief`)
/// only their number.
fn caller_cell(c: &Caller, brief: bool) -> String {
    let mut s = format!("`0x{:08x}`", c.function);
    if let Some(n) = &c.name {
        let _ = write!(s, " {n}");
    }
    if brief {
        if c.sites.len() > 1 {
            let _ = write!(s, " ({} sites)", c.sites.len());
        }
        return s;
    }
    let sites: Vec<String> = c
        .sites
        .iter()
        .map(|x| match x.thunk {
            Some(t) => format!("0x{:08x} via 0x{t:08x}", x.at),
            None => format!("0x{:08x}", x.at),
        })
        .collect();
    let _ = write!(s, " ({})", sites.join(", "));
    s
}

/// The census as Markdown: one table per platform DLL (import, slot,
/// callers), then the imports the movie and codec code calls.
#[must_use]
pub fn census_markdown(image: &Image, names: &BTreeMap<u32, String>) -> String {
    let mut out = String::new();
    for dll in PLATFORM_DLLS {
        let _ = writeln!(out, "#### `{dll}`\n");
        let _ = writeln!(out, "| Import | Slot | Callers (function (sites)) |");
        let _ = writeln!(out, "|---|---|---|");
        for i in image.imports_of(dll) {
            let callers = callers_by_function(image, names, i.slot);
            let cells: Vec<String> = callers.iter().map(|c| caller_cell(c, false)).collect();
            let cell = if cells.is_empty() {
                "none".to_string()
            } else {
                cells.join("; ")
            };
            let _ = writeln!(out, "| `{}` | `0x{:08x}` | {cell} |", i.name, i.slot);
        }
        out.push('\n');
    }
    let _ = writeln!(out, "#### Imports called from the movie and codec code\n");
    let _ = writeln!(
        out,
        "| DLL | Import | Callers (function, number of sites when more than one) |"
    );
    let _ = writeln!(out, "|---|---|---|");
    for i in &image.imports {
        let callers: Vec<Caller> = callers_by_function(image, names, i.slot)
            .into_iter()
            .filter_map(|mut c| {
                c.sites.retain(|s| movie_range(s.at).is_some());
                (!c.sites.is_empty()).then_some(c)
            })
            .collect();
        if callers.is_empty() {
            continue;
        }
        let cells: Vec<String> = callers.iter().map(|c| caller_cell(c, true)).collect();
        let _ = writeln!(out, "| {} | `{}` | {} |", i.dll, i.name, cells.join("; "));
    }
    out
}
