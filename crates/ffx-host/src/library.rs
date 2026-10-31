//! `LoadLibraryA`, `GetProcAddress` and `FreeLibrary` for the game's spell
//! overlays (`magicFiles/FFX/magic_<nnnn>.dll`, docs/formats/magic.md;
//! docs/host.md, "Overlays"): the player's own DLL from the game folder,
//! mapped into the arena and relocated like the C runtime DLLs
//! ([`crate::dll`]), its imports bound to what `FFX.exe`'s same imports
//! reach, `DllMain` run on the loading thread, and its code run in
//! Unicorn, development only (M5 runs the original overlays in Unicorn on
//! purpose: docs/formats/magic.md, "Running the original overlays").
//!
//! Every load gets a fresh range and a freed overlay's range is never
//! reused: Unicorn keeps each thread's translations by address and doesn't
//! see the host write new code there.

use std::sync::Mutex;

use ffx_bridge::Mem;

use crate::dll::Dll;
use crate::exec::Exec;
use crate::imports::{a, read_str};
use crate::table::{Args, Ret};

/// The overlays loaded now.
static LOADED: Mutex<Vec<Dll>> = Mutex::new(Vec::new());

/// Whether `name` is a file this layer loads: a DLL under the game
/// folder's `magicFiles`.
fn is_overlay(name: &str) -> bool {
    let lower = name.replace('/', "\\").to_ascii_lowercase();
    lower.ends_with(".dll") && lower.contains("magicfiles\\")
}

/// `LoadLibraryA(name)`: an overlay, else 0 (the other libraries the game
/// asks for, `cg.dll`, `psapi.dll`, are optional to it).
pub fn load_library_a(e: &mut Exec, args: &Args) -> Ret {
    let name = read_str(&e.mem(), a(args, 0), false);
    Ret::eax(load(e, &name).unwrap_or(0))
}

fn load(e: &mut Exec, name: &str) -> Option<u32> {
    if !is_overlay(name) {
        return None;
    }
    let path = e.shared().files.lock().ok()?.resolve(name)?.0;
    let Ok(bytes) = std::fs::read(&path) else {
        crate::log::note(&format!("LoadLibraryA({name}): not found"));
        return None;
    };
    let file = name.rsplit(['\\', '/']).next().unwrap_or(name);
    let dll = match crate::dll::load(e.arena(), &bytes, file) {
        Ok(d) => d,
        Err(why) => {
            crate::log::note(&format!("LoadLibraryA({name}): {why}"));
            return None;
        }
    };
    if let Err(why) = e.sync_regions() {
        crate::log::note(&format!("LoadLibraryA({name}): {why}"));
        return None;
    }
    let mut mem = e.mem();
    for imp in &dll.imports {
        let Some(target) = import_target(e, &imp.dll, &imp.name) else {
            crate::log::note(&format!(
                "LoadLibraryA({file}): import {}!{} not bound; the load fails",
                imp.dll, imp.name
            ));
            return None;
        };
        mem.write(imp.slot, target);
    }
    let (base, entry) = (dll.base, dll.entry);
    if let Ok(mut l) = LOADED.lock() {
        l.push(dll);
    }
    let ok = e.call_stdcall(entry, &[base, 1, 0]).unwrap_or(0);
    if ok == 0 {
        crate::log::note(&format!("LoadLibraryA({file}): DllMain returned FALSE"));
        forget(base);
        return None;
    }
    crate::log::note(&format!("LoadLibraryA({file}) at {base:#x}"));
    Some(base)
}

/// Where an overlay's import `dll!name` goes: where the same import of
/// `FFX.exe` (or of a C runtime DLL) goes, its IAT slot's value, so the
/// overlay reaches the same Rust side or DLL export (`rand` the game's
/// sequence); else the loaded DLL's export.
fn import_target(e: &Exec, dll: &str, name: &str) -> Option<u32> {
    let mem = e.mem();
    let same = |i: &crate::image::Import| i.dll.eq_ignore_ascii_case(dll) && i.name == name;
    if let Some(i) = e.image().imports.iter().find(|i| same(i)) {
        return Some(mem.read::<u32>(i.slot));
    }
    for d in &e.shared().dlls {
        if let Some(i) = d.imports.iter().find(|i| same(i)) {
            return Some(mem.read::<u32>(i.slot));
        }
    }
    e.shared().dll(dll)?.exports.get(name).copied()
}

fn forget(base: u32) {
    if let Ok(mut l) = LOADED.lock() {
        l.retain(|d| d.base != base);
    }
}

/// `GetProcAddress(module, name)`: an overlay's export by name, else 0.
pub fn get_proc_address(e: &mut Exec, args: &Args) -> Ret {
    let (module, at) = (a(args, 0), a(args, 1));
    if at < 0x1_0000 {
        return Ret::eax(0);
    }
    let name = read_str(&e.mem(), at, false);
    let found = LOADED.lock().ok().and_then(|l| {
        l.iter()
            .find(|d| d.base == module)
            .and_then(|d| d.exports.get(&name).copied())
    });
    Ret::eax(found.unwrap_or(0))
}

/// `FreeLibrary(module)`: an overlay's `DllMain(base, DLL_PROCESS_DETACH,
/// 0)`, then it is forgotten (its range stays mapped, unused); 1.
pub fn free_library(e: &mut Exec, args: &Args) -> Ret {
    let module = a(args, 0);
    let entry = LOADED
        .lock()
        .ok()
        .and_then(|l| l.iter().find(|d| d.base == module).map(|d| d.entry));
    if let Some(entry) = entry {
        let _ = e.call_stdcall(entry, &[module, 0, 0]);
        forget(module);
        crate::log::note(&format!("FreeLibrary({module:#x})"));
    }
    Ret::eax(1)
}

/// The loaded overlay holding `addr`, as `name+off` (reports).
#[must_use]
pub fn describe(addr: u32) -> Option<String> {
    let l = LOADED.lock().ok()?;
    l.iter()
        .find(|d| d.contains(addr))
        .map(|d| d.describe(addr))
}

#[cfg(test)]
mod tests {
    use super::is_overlay;

    #[test]
    fn only_the_spell_overlays_load() {
        assert!(is_overlay("magicFiles\\FFX\\magic_0481.dll"));
        assert!(is_overlay("./magicFiles/FFX/magic_0003.dll"));
        assert!(!is_overlay("cg.dll"));
        assert!(!is_overlay("psapi.dll"));
    }
}
