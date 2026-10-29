//! Maps the player's `FFX.exe` into the arena, read as data
//! (docs/host.md, "Loading FFX.exe").
//!
//! The image goes to its preferred base, `0x00400000`, so every `ffx.sym`
//! address is its own game address and nothing is relocated. The headers
//! and every section are copied; `.text` too, for the development-only
//! Unicorn fallback (the release maps only the data sections). The import
//! address table is left for [`crate::imports`] to bind.

use goblin::pe::PE;

use crate::arena::{Arena, PAGE};
use crate::{Error, Result};

/// The supported build: `FFX.exe`'s PE `TimeDateStamp` and `SizeOfImage`
/// (the hook checks the same pair, `ffx-hook`'s `check_build`).
pub const SIZE_OF_IMAGE: u32 = 0x0237_d000;

/// A section as mapped.
#[derive(Debug, Clone)]
pub struct Section {
    /// Its name (`.text`, `.rdata`, ...).
    pub name: String,
    /// Game address.
    pub start: u32,
    /// Size in memory.
    pub size: u32,
    /// Whether it holds code.
    pub code: bool,
}

/// One import as the image names it.
#[derive(Debug, Clone)]
pub struct Import {
    /// The DLL, as written in the image (`KERNEL32.dll`).
    pub dll: String,
    /// The function name (or `#<ordinal>`).
    pub name: String,
    /// The game address of its IAT slot.
    pub slot: u32,
}

/// `FFX.exe` in the arena.
#[derive(Debug, Clone)]
pub struct Image {
    /// Where it is (always the preferred base).
    pub base: u32,
    /// `SizeOfImage`.
    pub size: u32,
    /// The entry point's game address (`__tmainCRTStartup`'s wrapper).
    pub entry: u32,
    /// The sections.
    pub sections: Vec<Section>,
    /// Every import.
    pub imports: Vec<Import>,
    /// Where its x87 instructions the emulator gets wrong are (RVAs).
    pub x87: ffx_fpu::Sites,
}

impl Image {
    /// Whether `addr` lies in a code section.
    #[must_use]
    pub fn is_code(&self, addr: u32) -> bool {
        self.sections
            .iter()
            .any(|s| s.code && addr >= s.start && addr - s.start < s.size)
    }

    /// The `.text` section.
    #[must_use]
    pub fn text(&self) -> Option<&Section> {
        self.sections.iter().find(|s| s.name == ".text")
    }
}

/// Maps `bytes` (the contents of `FFX.exe`) into `arena`.
///
/// # Errors
/// When the file is not the supported build or a mapping fails.
pub fn load(arena: &Arena, bytes: &[u8]) -> Result<Image> {
    let pe = PE::parse(bytes)?;
    let opt = pe
        .header
        .optional_header
        .ok_or_else(|| Error::Image("no optional header".into()))?;
    let base = u32::try_from(pe.image_base).map_err(|_| Error::Image("image base".into()))?;
    let size = opt.windows_fields.size_of_image;
    if size != SIZE_OF_IMAGE || base != ffx_bridge::IMAGE_BASE {
        return Err(Error::Image(format!(
            "not the supported FFX.exe (base {base:#x}, size {size:#x})"
        )));
    }
    arena.map(base, size.next_multiple_of(PAGE), "FFX.exe")?;
    let headers = (opt.windows_fields.size_of_headers as usize).min(bytes.len());
    arena.fill(base, &bytes[..headers]);
    let mut sections = Vec::new();
    for s in &pe.sections {
        let name = String::from_utf8_lossy(&s.name)
            .trim_end_matches('\0')
            .to_string();
        let raw = s.pointer_to_raw_data as usize;
        let len = (s.size_of_raw_data as usize)
            .min(s.virtual_size as usize)
            .min(bytes.len().saturating_sub(raw));
        let start = base + s.virtual_address;
        arena.fill(start, &bytes[raw..raw + len]);
        sections.push(Section {
            name,
            start,
            size: s.virtual_size.max(s.size_of_raw_data),
            code: s.characteristics & 0x2000_0000 != 0,
        });
    }
    let imports = pe
        .imports
        .iter()
        .map(|i| Import {
            dll: i.dll.to_string(),
            name: i.name.to_string(),
            slot: base + u32::try_from(i.offset).unwrap_or(0),
        })
        .collect();
    let x87 = x87_sites(bytes, &pe);
    Ok(Image {
        base,
        size,
        entry: base + pe.entry,
        sections,
        imports,
        x87,
    })
}

/// The x87 sites ([`ffx_fpu::sites`]) of a PE file's code sections, from
/// the file's bytes (`FFX.exe` or a DLL).
pub(crate) fn x87_sites(bytes: &[u8], pe: &PE) -> ffx_fpu::Sites {
    let code: Vec<(u32, u32)> = pe
        .sections
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
        .map(|s| (s.virtual_address, s.virtual_size))
        .collect();
    let end = code.iter().map(|&(va, n)| va + n).max().unwrap_or(0) as usize;
    // the code sections laid out as in memory, enough for the RVAs
    let mut image = vec![0u8; end];
    for s in pe
        .sections
        .iter()
        .filter(|s| s.characteristics & 0x2000_0000 != 0)
    {
        let raw = s.pointer_to_raw_data as usize;
        let va = s.virtual_address as usize;
        let len = (s.size_of_raw_data as usize)
            .min(s.virtual_size as usize)
            .min(bytes.len().saturating_sub(raw))
            .min(end.saturating_sub(va));
        image[va..va + len].copy_from_slice(&bytes[raw..raw + len]);
    }
    let (stamp, check_sum) = (
        pe.header.coff_header.time_date_stamp,
        pe.header
            .optional_header
            .map_or(0, |o| o.windows_fields.check_sum),
    );
    ffx_fpu::sites(&image, stamp, check_sum, &code)
}
