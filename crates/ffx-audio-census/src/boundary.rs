//! The FMOD boundary as `FFX.exe`'s bytes show it: every import of
//! `fmod_event.dll` and `fmodex.dll`, the `jmp [slot]` thunk the game
//! calls it through, and every `call` of that thunk.

use goblin::pe::PE;

/// One FMOD import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub dll: String,
    /// The decorated name (or the C name).
    pub name: String,
    /// Address of the import table slot.
    pub slot: u32,
    /// Addresses of the `jmp dword ptr [slot]` thunks.
    pub thunks: Vec<u32>,
    /// Addresses of the `call` instructions that reach the import: `call
    /// thunk` (`E8 rel32`) or `call dword ptr [slot]` (`FF 15`).
    pub calls: Vec<u32>,
}

/// The image could not be read.
#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

/// The DLLs of the boundary.
pub const FMOD_DLLS: [&str; 2] = ["fmod_event.dll", "fmodex.dll"];

/// Every FMOD import of `exe` (the bytes of `FFX.exe`), with its thunks
/// and calls, in import table order.
///
/// # Errors
/// When `exe` is not a PE32 image.
pub fn imports(exe: &[u8]) -> Result<Vec<Import>, Error> {
    let pe = PE::parse(exe).map_err(|e| Error(e.to_string()))?;
    let base = u32::try_from(pe.image_base).map_err(|_| Error("not PE32".into()))?;
    let mut out: Vec<Import> = pe
        .imports
        .iter()
        .filter(|i| FMOD_DLLS.iter().any(|d| d.eq_ignore_ascii_case(i.dll)))
        .map(|i| Import {
            dll: i.dll.to_ascii_lowercase(),
            name: i.name.to_string(),
            slot: base + u32::try_from(i.offset).unwrap_or(0),
            thunks: Vec::new(),
            calls: Vec::new(),
        })
        .collect();
    let text: Vec<(u32, &[u8])> = pe
        .sections
        .iter()
        .filter(|s| s.characteristics & goblin::pe::section_table::IMAGE_SCN_MEM_EXECUTE != 0)
        .filter_map(|s| {
            let start = s.pointer_to_raw_data as usize;
            let len = (s.size_of_raw_data as usize).min(s.virtual_size as usize);
            exe.get(start..start + len)
                .map(|b| (base + s.virtual_address, b))
        })
        .collect();
    let word = |b: &[u8], at: usize| u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
    // thunks and direct `call [slot]`
    for (va, bytes) in &text {
        for at in 0..bytes.len().saturating_sub(6) {
            if bytes[at] != 0xff || !matches!(bytes[at + 1], 0x25 | 0x15) {
                continue;
            }
            let target = word(bytes, at + 2);
            if let Some(i) = out.iter_mut().find(|i| i.slot == target) {
                let address = va + u32::try_from(at).unwrap_or(0);
                if bytes[at + 1] == 0x25 {
                    i.thunks.push(address);
                } else {
                    i.calls.push(address);
                }
            }
        }
    }
    // calls of the thunks
    for (va, bytes) in &text {
        for at in 0..bytes.len().saturating_sub(5) {
            if bytes[at] != 0xe8 {
                continue;
            }
            let address = va + u32::try_from(at).unwrap_or(0);
            let target = address.wrapping_add(5).wrapping_add(word(bytes, at + 1));
            if let Some(i) = out.iter_mut().find(|i| i.thunks.contains(&target)) {
                i.calls.push(address);
            }
        }
    }
    for i in &mut out {
        i.calls.sort_unstable();
    }
    Ok(out)
}
