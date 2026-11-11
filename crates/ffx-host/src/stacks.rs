//! The harness's `stacks` command (docs/harness.md, "Two processes"):
//! every game thread's emulated call stack, named from `symbols/ffx.sym`.
//!
//! For each thread: its role (main, render, loader) or its routine, the
//! host entries it is inside (innermost first: the import or port it waits
//! in), and the return addresses on its game stack from the innermost
//! entry's `esp` up (words that follow a `call` into `FFX.exe` or a DLL),
//! innermost first. A thread inside no host entry runs emulated code: its
//! stack is read from where it last entered one (marked `running`). Ahead
//! of the threads: the clock's state and the render lock's and loader's
//! words. One line (threads apart by ` | `), as the hook's `stacks` is.

use std::sync::OnceLock;

use ffx_bridge::Mem as _;

use crate::arena::ArenaMem;
use crate::exec::Shared;
use crate::exec::spot::{Frame, Kind};
use crate::report::Symbols;

/// The render lock (`0x00c94ee4`): waiters `+0xc`, owner `+0x10`,
/// recursion `+0x14`.
const RENDER_LOCK: u32 = 0x00c9_4ee4;
/// `g_AsyncLoadManager`: its two request lists' heads `+0x20`, `+0x2c`.
const ASYNC_LOAD_MANAGER: u32 = 0x00cc_c88c;
/// Return addresses listed per thread.
const MAX_RETURNS: usize = 40;

/// `ffx.sym` for the names: `FFX_HOST_SYM`, else `symbols/ffx.sym` from
/// the current directory, else `ffx.sym` beside the executable; no file,
/// no names (the addresses stay).
fn symbols() -> &'static Symbols {
    static SYMS: OnceLock<Symbols> = OnceLock::new();
    SYMS.get_or_init(|| {
        Symbols::locate()
            .map(|p| Symbols::read(&p))
            .unwrap_or_default()
    })
}

/// `name+off` for a code address (`FFX.exe` by `ffx.sym`, a DLL by its
/// nearest export).
fn name(shared: &Shared, syms: &Symbols, a: u32) -> String {
    match shared.dlls.iter().find(|d| d.contains(a)) {
        Some(d) => d.describe(a),
        None => match syms.containing(a) {
            Some((s, n)) if s == a => format!("{n}@{a:08x}"),
            Some((s, n)) => format!("{n}+{:x}@{a:08x}", a - s),
            None => format!("{a:08x}"),
        },
    }
}

/// Whether the word at `a` is a return address: code, after a `call`.
fn after_call(shared: &Shared, mem: &ArenaMem, a: u32) -> bool {
    let code = shared.image().is_code(a) || shared.dlls.iter().any(|d| d.contains(a));
    if !code || !shared.arena().is_mapped(a.wrapping_sub(6), 6) {
        return false;
    }
    let b = |k: u32| mem.read::<u8>(a.wrapping_sub(k));
    b(5) == 0xe8
        || (b(2) == 0xff && (b(1) & 0x38) == 0x10)
        || (b(3) == 0xff && (b(2) & 0x38) == 0x10)
        || (b(6) == 0xff && (b(5) & 0x38) == 0x10)
}

/// The return addresses from `esp` to the stack's end, innermost first.
fn returns(shared: &Shared, mem: &ArenaMem, esp: u32, end: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut p = esp & !3;
    while p < end && out.len() < MAX_RETURNS {
        if !shared.arena().is_mapped(p, 4) {
            break;
        }
        let w: u32 = mem.read(p);
        if after_call(shared, mem, w) {
            out.push(w);
        }
        p += 4;
    }
    out
}

fn entry(shared: &Shared, syms: &Symbols, f: &Frame) -> String {
    match f.kind {
        Kind::Import => shared
            .import_name(f.what as usize)
            .map_or_else(|| format!("import#{}", f.what), ToString::to_string),
        Kind::Port => shared.port_name(f.what).map_or_else(
            || name(shared, syms, f.what),
            |n| format!("{n}@{:08x}", f.what),
        ),
        Kind::Trap => format!("trap@{:08x}", f.what),
    }
}

/// The `stacks` reply (after `ok`).
#[must_use]
pub fn stacks(shared: &Shared) -> String {
    let syms = symbols();
    let mem = ArenaMem {
        arena: std::sync::Arc::clone(shared.arena()),
    };
    let word = |a: u32| {
        if shared.arena().is_mapped(a, 4) {
            mem.read::<u32>(a)
        } else {
            0
        }
    };
    let manager = word(ASYNC_LOAD_MANAGER);
    let (lists, lists2) = if manager == 0 {
        (0, 0)
    } else {
        (word(manager + 0x20), word(manager + 0x2c))
    };
    let mut out = vec![format!(
        "{} lock owner {:#x} rec {} waiters {} loader lists {lists:#x} {lists2:#x}",
        shared.clock.state(),
        word(RENDER_LOCK + 0x10),
        word(RENDER_LOCK + 0x14),
        word(RENDER_LOCK + 0xc),
    )];
    let (render, loader) = shared.clock.roles();
    let entries: Vec<(u32, u32)> = shared
        .threads
        .lock()
        .map(|t| t.iter().map(|i| (i.id, i.entry)).collect())
        .unwrap_or_default();
    let spots = shared.spots.lock().map(|s| s.clone()).unwrap_or_default();
    for s in spots {
        let role = if s.id == crate::exec::MAIN_THREAD {
            "main".to_string()
        } else if s.id == render {
            "render".to_string()
        } else if s.id == loader {
            "loader".to_string()
        } else {
            entries
                .iter()
                .find(|&&(id, _)| id == s.id)
                .map_or_else(String::new, |&(_, e)| name(shared, syms, e))
        };
        let (frames, deeper) = s.frames();
        let (state, esp) = match frames.last() {
            Some(f) => {
                let mut v: Vec<String> = frames
                    .iter()
                    .rev()
                    .map(|f| entry(shared, syms, f))
                    .collect();
                if deeper > 0 {
                    v.insert(0, format!("(+{deeper} deeper)"));
                }
                (format!("in {}", v.join(" < ")), f.esp)
            }
            None => ("running".to_string(), s.last_esp()),
        };
        let rets: Vec<String> = returns(shared, &mem, esp, s.stack.1)
            .into_iter()
            .map(|a| name(shared, syms, a))
            .collect();
        out.push(format!(
            "{:#x} {role} {state} esp {esp:#x}: {}",
            s.id,
            rets.join(" < ")
        ));
    }
    out.join(" | ")
}
