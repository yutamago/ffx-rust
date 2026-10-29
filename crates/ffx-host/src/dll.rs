//! The player's own C runtime DLLs in the arena (docs/host.md, "The C
//! runtime DLLs"): `MSVCR110.dll` and `MSVCP110.dll` from the game folder,
//! read as data like `FFX.exe`, relocated into a free range and run in
//! Unicorn, development only. The inlined STL templates in `FFX.exe` touch
//! MSVCP110's objects directly (streams, locales, strings), so they need
//! the real layouts; the DLLs give them those until the Rust side keeps
//! the same layouts.
//!
//! Their imports are bound by the executor ([`crate::exec`]): `KERNEL32`
//! and the rest to the same Rust sides as `FFX.exe`'s, `MSVCP110`'s
//! `MSVCR110` imports straight to the loaded `MSVCR110`'s exports.

use std::collections::HashMap;

use goblin::pe::PE;

use crate::arena::{Arena, PAGE};
use crate::image::Import;
use crate::{Error, Result};

/// The DLLs, in load order (`MSVCP110` imports `MSVCR110`).
pub const RUNTIME: [&str; 2] = ["MSVCR110.dll", "MSVCP110.dll"];

/// Where the DLLs go (the first free gap at or above).
const DLL_HINT: u32 = 0x0400_0000;

/// A loaded DLL.
#[derive(Debug, Clone)]
pub struct Dll {
    /// Its name as `FFX.exe` imports it (`MSVCR110.dll`).
    pub name: String,
    /// Where it is.
    pub base: u32,
    /// `SizeOfImage`, rounded up to pages.
    pub size: u32,
    /// `DllMain`.
    pub entry: u32,
    /// Exports by name.
    pub exports: HashMap<String, u32>,
    /// Its imports, IAT slots at their loaded addresses.
    pub imports: Vec<Import>,
}

impl Dll {
    /// Whether `addr` lies inside it.
    #[must_use]
    pub fn contains(&self, addr: u32) -> bool {
        addr.wrapping_sub(self.base) < self.size
    }

    /// The export at or below `addr` (for reports: `name+off`).
    #[must_use]
    pub fn describe(&self, addr: u32) -> String {
        let best = self
            .exports
            .iter()
            .filter(|&(_, &a)| a <= addr)
            .max_by_key(|&(_, &a)| a);
        match best {
            Some((n, &a)) if a == addr => format!("{}!{n}", self.name),
            Some((n, &a)) => format!("{}!{n}+{:#x}", self.name, addr - a),
            None => format!("{}+{:#x}", self.name, addr - self.base),
        }
    }
}

/// Loads every [`RUNTIME`] DLL found in `dir` (the game folder), in order.
/// A missing file stops the list there (`MSVCP110` needs `MSVCR110`).
///
/// # Errors
/// When a file is unreadable as a PE or a mapping fails.
pub fn load_runtime(arena: &Arena, dir: &std::path::Path) -> Result<Vec<Dll>> {
    let mut out = Vec::new();
    for name in RUNTIME {
        let lower = name.to_ascii_lowercase();
        let Some(bytes) = [name, lower.as_str()]
            .iter()
            .find_map(|n| std::fs::read(dir.join(n)).ok())
        else {
            break;
        };
        out.push(load(arena, &bytes, name)?);
    }
    Ok(out)
}

/// Maps the DLL `bytes` into a free range of `arena`, relocated.
///
/// # Errors
/// When `bytes` is not a PE or the arena has no room.
pub fn load(arena: &Arena, bytes: &[u8], name: &str) -> Result<Dll> {
    let pe = PE::parse(bytes)?;
    let opt = pe
        .header
        .optional_header
        .ok_or_else(|| Error::Image(format!("{name}: no optional header")))?;
    let size = opt.windows_fields.size_of_image.next_multiple_of(PAGE);
    let mut image = vec![0u8; size as usize];
    let headers = (opt.windows_fields.size_of_headers as usize)
        .min(bytes.len())
        .min(image.len());
    image[..headers].copy_from_slice(&bytes[..headers]);
    for s in &pe.sections {
        let raw = s.pointer_to_raw_data as usize;
        let va = s.virtual_address as usize;
        let len = (s.size_of_raw_data as usize)
            .min(bytes.len().saturating_sub(raw))
            .min(image.len().saturating_sub(va));
        image[va..va + len].copy_from_slice(&bytes[raw..raw + len]);
    }
    let base = arena.map_anywhere(DLL_HINT, size, name)?;
    let preferred = u32::try_from(pe.image_base).map_err(|_| Error::Image(name.into()))?;
    relocate(&mut image, &pe, base.wrapping_sub(preferred));
    arena.fill(base, &image);
    let exports = pe
        .exports
        .iter()
        .filter(|e| e.reexport.is_none())
        .filter_map(|e| Some((e.name?.to_string(), base + u32::try_from(e.rva).ok()?)))
        .collect();
    let imports = pe
        .imports
        .iter()
        .map(|i| Import {
            dll: i.dll.to_string(),
            name: i.name.to_string(),
            slot: base + u32::try_from(i.offset).unwrap_or(0),
        })
        .collect();
    Ok(Dll {
        name: name.to_string(),
        base,
        size,
        entry: base + opt.standard_fields.address_of_entry_point,
        exports,
        imports,
    })
}

/// Applies the base relocations (`IMAGE_REL_BASED_HIGHLOW`, the only kind
/// a 32-bit DLL has) for a load `delta` away from the preferred base.
fn relocate(image: &mut [u8], pe: &PE, delta: u32) {
    let Some(dir) = pe
        .header
        .optional_header
        .and_then(|o| o.data_directories.get_base_relocation_table().copied())
    else {
        return;
    };
    let word = |b: &[u8], i: usize| -> Option<u32> {
        Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
    };
    let mut at = dir.virtual_address as usize;
    let end = at + dir.size as usize;
    while at + 8 <= end {
        let (Some(page), Some(block)) = (word(image, at), word(image, at + 4)) else {
            return;
        };
        let block = block as usize;
        if block < 8 {
            return;
        }
        for k in 0..(block - 8) / 2 {
            let e = u16::from_le_bytes([image[at + 8 + 2 * k], image[at + 9 + 2 * k]]);
            if e >> 12 != 3 {
                continue;
            }
            let p = page as usize + usize::from(e & 0xfff);
            if let Some(v) = word(image, p) {
                image[p..p + 4].copy_from_slice(&v.wrapping_add(delta).to_le_bytes());
            }
        }
        at += block;
    }
}
