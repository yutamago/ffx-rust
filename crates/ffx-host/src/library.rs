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
//!
//! **Rust ports** (`crates/ffx-magic`, docs/formats/magic.md "Porting the
//! overlays"): at load, before `DllMain`, every address the game can reach
//! the overlay's code by (its exports, and every relocated word in the
//! image that points into its code: the overlay table's entries, stored
//! callbacks, `push offset` operands) is offered to [`ffx_magic::bind`];
//! where a port's code identity matches, the address gets an `int3` and a
//! trap that runs the port (as the hook's `FFX_MAGIC_RUST`), 64 KiB below
//! the caller's stack (ports keep locals below their entry `esp`). The
//! rest of the overlay runs in Unicorn. `FFX_MAGIC_RUST` picks the DLLs
//! as in the hook: unset or `1` every DLL, `0` none (all Unicorn), or a
//! comma list of numbers. Calls are counted per entry, Rust and Unicorn
//! (a block hook on the overlay's code counts the unported entries); the
//! counts are noted at `FreeLibrary` and in [`summary`].

use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use ffx_bridge::Mem;
use unicorn_engine::RegisterX86;

use crate::arena::ArenaMem;
use crate::dll::Dll;
use crate::exec::{Exec, Halted, Stop, Trap, TrapExit};
use crate::imports::{a, read_str};
use crate::table::{Args, Conv, Ret};

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
    bind_ports(e, &dll, &bytes, file);
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
        for line in counts(Some(module)) {
            crate::log::note(&line);
        }
    }
    Ret::eax(1)
}

/// The stack a port leaves below its entry `esp` for its own locals
/// before its calls into the game push theirs (the hook's `GAP`).
const GAP: u32 = 0x1_0000;

/// Which DLLs run their Rust ports.
#[derive(Debug, Clone)]
enum Selection {
    /// As `FFX_MAGIC_RUST` says.
    Env,
    /// Every DLL.
    All,
    /// These numbers (none when empty).
    Only(Vec<u16>),
}

impl Selection {
    fn has(&self, number: u16) -> bool {
        match self {
            Selection::Env | Selection::All => true,
            Selection::Only(l) => l.contains(&number),
        }
    }
}

/// What [`select_rust`] chose.
static SELECTION: RwLock<Selection> = RwLock::new(Selection::Env);

/// Which overlays loaded from now on run their Rust ports: `None` every
/// DLL, `Some(list)` those numbers (an empty list: none, all Unicorn).
/// Replaces `FFX_MAGIC_RUST` (tests).
pub fn select_rust(list: Option<&[u16]>) {
    if let Ok(mut s) = SELECTION.write() {
        *s = list.map_or(Selection::All, |l| Selection::Only(l.to_vec()));
    }
}

/// Which DLLs run their Rust ports (`FFX_MAGIC_RUST` unless
/// [`select_rust`] chose; module docs).
fn selected(number: u16) -> bool {
    static ENV: OnceLock<Selection> = OnceLock::new();
    match SELECTION.read().map(|s| s.clone()) {
        Ok(Selection::Env) | Err(_) => {}
        Ok(s) => return s.has(number),
    }
    ENV.get_or_init(|| match std::env::var("FFX_MAGIC_RUST") {
        Err(_) => Selection::All,
        Ok(v) if v.trim() == "1" || v.trim().is_empty() => Selection::All,
        Ok(v) if v.trim() == "0" => Selection::Only(Vec::new()),
        Ok(v) => Selection::Only(v.split(',').filter_map(|n| n.trim().parse().ok()).collect()),
    })
    .has(number)
}

/// The Rust and Unicorn calls into the overlay loaded at `base` so far
/// (tests; [`summary`] for the report).
#[must_use]
pub fn calls(base: u32) -> Option<(u64, u64)> {
    let (c, bound) = (COUNTED.read().ok()?, BOUND.read().ok()?);
    let o = c.iter().find(|o| o.base == base)?;
    let rust = o
        .rust
        .iter()
        .map(|&i| bound[i].calls.load(Ordering::Relaxed))
        .sum();
    let emu = o.unicorn.values().map(|n| n.load(Ordering::Relaxed)).sum();
    Some((rust, emu))
}

/// What an overlay's ports need besides its image: where its `rand`,
/// libm and `_CIatan2` imports go (the game's own, as the DLL's IAT).
#[derive(Debug)]
struct Overlay {
    img: ffx_magic::Image,
    rand: Option<u32>,
    libm: [Option<u32>; 4],
    atan2: Option<u32>,
}

/// A bound port: the address, the overlay, the port's index in
/// `ffx_magic::PORTS`, and its calls.
#[derive(Debug)]
struct Bound {
    at: u32,
    overlay: Arc<Overlay>,
    port: usize,
    calls: AtomicU64,
}

/// Every port bound so far (a trap's tag is its index; never shrinks).
static BOUND: RwLock<Vec<Arc<Bound>>> = RwLock::new(Vec::new());

/// One loaded overlay's entries, for the counts.
#[derive(Debug)]
struct Counted {
    name: String,
    base: u32,
    /// Its code, `begin..end`.
    code: (u32, u32),
    /// Indices into [`BOUND`].
    rust: Vec<usize>,
    /// The entries without a port: block starts counted in Unicorn.
    unicorn: Arc<EntryCounts>,
    reported: std::sync::atomic::AtomicBool,
}

/// Calls per entry address, counted from any thread.
type EntryCounts = HashMap<u32, AtomicU64>;

static COUNTED: RwLock<Vec<Counted>> = RwLock::new(Vec::new());
/// `COUNTED.len()`, read without the lock by [`sync_hooks`].
static COUNTED_LEN: AtomicUsize = AtomicUsize::new(0);

/// Gives this thread's emulator the counting hooks of overlays loaded
/// since it last looked (from [`Exec::sync_regions`]).
pub(crate) fn sync_hooks(e: &mut Exec) -> crate::Result<()> {
    let n = COUNTED_LEN.load(Ordering::Acquire);
    if e.overlay_hooks >= n {
        return Ok(());
    }
    let todo: Vec<(u32, u32, Arc<EntryCounts>)> = match COUNTED.read() {
        Ok(c) => c[e.overlay_hooks.min(c.len())..]
            .iter()
            .map(|o| (o.code.0, o.code.1, Arc::clone(&o.unicorn)))
            .collect(),
        Err(_) => return Ok(()),
    };
    e.overlay_hooks = n;
    for (b, end, counts) in todo {
        e.add_entry_counter(b, end, counts)?;
    }
    Ok(())
}

/// The addresses the game can enter `img`'s code by: its exports, its
/// entry point and every relocated word pointing into its code.
fn entries(img: &ffx_magic::Image, dll: &Dll) -> BTreeSet<u32> {
    let mut out: BTreeSet<u32> = img
        .exports
        .iter()
        .map(|(_, rva)| img.at(*rva))
        .chain(std::iter::once(dll.entry))
        .chain(img.operands(img.base, dll.size))
        .filter(|&w| img.in_text(w))
        .collect();
    out.retain(|&w| dll.contains(w));
    out
}

/// Binds the ports of `dll` (just mapped, before `DllMain`): an `int3`
/// and a trap at each entry a port matches; the others counted.
fn bind_ports(e: &mut Exec, dll: &Dll, bytes: &[u8], file: &str) {
    let img = match ffx_magic::Image::new(file, bytes, dll.base) {
        Ok(i) => i,
        Err(why) => {
            crate::log::note(&format!("{file}: no Rust ports ({why})"));
            return;
        }
    };
    let rust = selected(img.number);
    let imp = |name: &str| import_target(e, "MSVCR110.dll", name);
    let overlay = Arc::new(Overlay {
        rand: imp("rand"),
        libm: [
            imp("_libm_sse2_sin_precise"),
            imp("_libm_sse2_cos_precise"),
            imp("_libm_sse2_atan_precise"),
            imp("_libm_sse2_acos_precise"),
        ],
        atan2: imp("_CIatan2"),
        img,
    });
    let img = &overlay.img;
    let mut bound = Vec::new();
    let mut unicorn = HashMap::new();
    for at in entries(img, dll) {
        match ffx_magic::bind(img, at).filter(|_| rust) {
            Some(port) => {
                let Ok(mut b) = BOUND.write() else { return };
                let tag = u32::try_from(b.len()).unwrap_or(u32::MAX);
                b.push(Arc::new(Bound {
                    at,
                    overlay: Arc::clone(&overlay),
                    port,
                    calls: AtomicU64::new(0),
                }));
                drop(b);
                bound.push(tag as usize);
                e.write_code(at, &[0xcc]);
                e.shared().add_trap(
                    at,
                    Trap {
                        run: overlay_trap,
                        tag,
                        conv: Conv::Cdecl,
                        words: 0,
                    },
                );
            }
            None => {
                unicorn.insert(at, AtomicU64::new(0));
            }
        }
    }
    let code = (img.at(img.text.0), img.at(img.text.1));
    crate::log::note(&format!(
        "{file}: {} entries, {} bound to Rust ports, {} in Unicorn",
        bound.len() + unicorn.len(),
        bound.len(),
        unicorn.len()
    ));
    if let Ok(mut c) = COUNTED.write() {
        c.push(Counted {
            name: file.to_string(),
            base: dll.base,
            code,
            rust: bound,
            unicorn: Arc::new(unicorn),
            reported: std::sync::atomic::AtomicBool::new(false),
        });
        COUNTED_LEN.store(c.len(), Ordering::Release);
    }
}

/// The trap at a bound entry: runs the port for the call (from Unicorn,
/// or from Rust through `Exec::call`, whose words go on the game stack
/// first, where the port reads them).
fn overlay_trap(e: &mut Exec, tag: u32, args: &Args) -> Ret {
    let Some(b) = BOUND.read().ok().and_then(|b| b.get(tag as usize).cloned()) else {
        return Ret::VOID;
    };
    if args.slots != 0 {
        return run_bound(e, &b, args);
    }
    let r = e.on_game_stack(args, |e, a| {
        let r = run_bound(e, &b, a);
        (r, e.trap_exit.take())
    });
    match r {
        Ok((
            _,
            Some(TrapExit {
                jump: Some(target), ..
            }),
        )) => {
            // a tail jump from a call out of Rust: the target, called
            // with the same words and registers
            e.call(target, Conv::FastcallCallerPops, args)
                .unwrap_or(Ret::VOID)
        }
        Ok((r, _)) => r,
        Err(_) => Ret::VOID,
    }
}

fn run_bound(e: &mut Exec, bound: &Bound, args: &Args) -> Ret {
    let b = bound;
    let Some(port) = ffx_magic::entry::<HostGame>(b.port) else {
        return Ret::VOID;
    };
    let esp = args.slots.wrapping_sub(4);
    let frame = ffx_magic::Frame {
        at: b.at,
        esp,
        eax: e.reg(RegisterX86::EAX).unwrap_or(0),
        ecx: args.ecx,
        edx: args.edx,
        // the game calls overlay entries cdecl or thiscall: no float
        // arguments in SSE registers
        xmm: [0; 4],
    };
    let Ok(now) = e.esp() else {
        return Ret::VOID;
    };
    let low = esp.wrapping_sub(GAP) & !0xf;
    if low >= e.stack_floor() && low < esp {
        let _ = e.set_esp(low);
    }
    let mem = e.mem();
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut g = HostGame {
            e: &mut *e,
            mem,
            overlay: &b.overlay,
        };
        port(&mut g, &b.overlay.img, &frame)
    }));
    let _ = e.set_esp(now);
    let out = match r {
        Ok(out) => out,
        Err(p) if p.is::<Halted>() => return Ret::VOID,
        Err(p) => {
            let why = p
                .downcast_ref::<&str>()
                .map(ToString::to_string)
                .or_else(|| p.downcast_ref::<String>().cloned())
                .unwrap_or_default();
            e.end(Stop::Failed(
                format!(
                    "overlay port {} ({}) panicked: {why}",
                    ffx_magic::PORTS[b.port].0,
                    describe(b.at).unwrap_or_default()
                ),
                b.at,
            ));
            return Ret::VOID;
        }
    };
    b.calls.fetch_add(1, Ordering::Relaxed);
    e.trap_exit = Some(TrapExit {
        pops: out.pops,
        jump: out.jump,
    });
    if let Some(x) = out.xmm0 {
        #[allow(clippy::cast_possible_truncation)] // the low lane
        let _ = e.set_xmm0(x.reg() as u64);
    }
    Ret {
        eax: out.eax,
        edx: Some(out.edx),
        st0: out.st0,
        sets_eax: true,
    }
}

/// The game as an overlay port sees it in `ffx-rust`: the arena, and
/// calls through the executor (the table, the imports, Unicorn).
struct HostGame<'a> {
    e: &'a mut Exec,
    mem: ArenaMem,
    overlay: &'a Overlay,
}

impl Mem for HostGame<'_> {
    fn read_bytes(&self, addr: u32, out: &mut [u8]) {
        self.mem.read_bytes(addr, out);
    }
    fn write_bytes(&mut self, addr: u32, data: &[u8]) {
        self.mem.write_bytes(addr, data);
    }
    fn image_base(&self) -> u32 {
        ffx_bridge::IMAGE_BASE
    }
}

impl ffx_magic::Game for HostGame<'_> {
    fn call(&mut self, target: u32, ecx: u32, args: &[u32]) -> ffx_magic::Out {
        let emulated = self.e.table().get(target).is_none()
            && self.e.shared().trap(target).is_none()
            && !self.e.is_import_stub(target);
        let a = Args {
            ecx,
            edx: 0,
            stack: args.to_vec(),
            slots: 0,
        };
        // thiscall sets ecx; a callee that pops nothing returns below the
        // sentinel's expected esp, which the executor accepts
        let Ok(r) = self.e.call(target, Conv::Thiscall, &a) else {
            crate::exec::halted()
        };
        if emulated && r.st0.is_some() {
            // the original's fstp takes it off Unicorn's x87 stack
            let _ = self.e.pop_st0();
        }
        ffx_magic::Out {
            eax: r.eax,
            edx: r.edx.unwrap_or(0),
            st0: r.st0,
            xmm0: 0,
        }
    }
    fn rand(&mut self) -> u32 {
        let Some(f) = self.overlay.rand else { return 0 };
        match self.e.call_cdecl(f, &[]) {
            Ok(v) => v,
            Err(_) => crate::exec::halted(),
        }
    }
    fn libm(&mut self, f: ffx_magic::Libm, x: f64) -> f64 {
        let i = match f {
            ffx_magic::Libm::Sin => 0,
            ffx_magic::Libm::Cos => 1,
            ffx_magic::Libm::Atan => 2,
            ffx_magic::Libm::Acos => 3,
        };
        let Some(at) = self.overlay.libm[i] else {
            return f64::NAN;
        };
        let v = self.e.call_sse(at, x);
        if self.e.stopped().is_some() {
            crate::exec::halted()
        }
        v
    }
    fn atan2(&mut self, y: f64, x: f64) -> f64 {
        let Some(at) = self.overlay.atan2 else {
            return f64::NAN;
        };
        // ST1 = y, ST0 = x (the last operand is ST0)
        ffx_bridge::route::Router::call_x87(
            &mut *self.e,
            at,
            &[y, x],
            ffx_bridge::route::Float::F64,
        )
    }
}

/// The overlay call counts, one line per overlay and per entry that ran:
/// of the overlay loaded at `base`, or of every overlay not reported yet
/// (`None`). Each overlay is reported once.
fn counts(base: Option<u32>) -> Vec<String> {
    let (Ok(c), Ok(bound)) = (COUNTED.read(), BOUND.read()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for o in c.iter().filter(|o| base.is_none_or(|b| b == o.base)) {
        if o.reported.swap(true, Ordering::Relaxed) {
            continue;
        }
        let mut rows: Vec<(u32, String)> = Vec::new();
        let (mut rust, mut emu) = (0u64, 0u64);
        for &i in &o.rust {
            let b = &bound[i];
            let n = b.calls.load(Ordering::Relaxed);
            rust += n;
            if n > 0 {
                rows.push((
                    b.at,
                    format!(
                        "  +{:#07x} rust: {n} calls ({})",
                        b.at - o.base,
                        ffx_magic::PORTS[b.port].0
                    ),
                ));
            }
        }
        for (&at, n) in o.unicorn.iter() {
            let n = n.load(Ordering::Relaxed);
            emu += n;
            if n > 0 {
                rows.push((at, format!("  +{:#07x} unicorn: {n} calls", at - o.base)));
            }
        }
        rows.sort_unstable();
        out.push(format!(
            "overlay calls: {} rust {rust} ({} entries bound), unicorn {emu} ({} entries)",
            o.name,
            o.rust.len(),
            o.unicorn.len()
        ));
        out.extend(rows.into_iter().map(|r| r.1));
    }
    out
}

/// Every overlay's call counts not reported at its `FreeLibrary` yet,
/// and the totals over the run: for the harness's `quit` and the report.
#[must_use]
pub fn summary() -> Vec<String> {
    let mut out = counts(None);
    let (Ok(c), Ok(bound)) = (COUNTED.read(), BOUND.read()) else {
        return out;
    };
    let rust: u64 = bound.iter().map(|b| b.calls.load(Ordering::Relaxed)).sum();
    let emu: u64 = c
        .iter()
        .flat_map(|o| o.unicorn.values())
        .map(|n| n.load(Ordering::Relaxed))
        .sum();
    if !c.is_empty() {
        out.push(format!(
            "overlay calls in all: rust {rust}, unicorn {emu} ({} overlays loaded)",
            c.len()
        ));
    }
    out
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
