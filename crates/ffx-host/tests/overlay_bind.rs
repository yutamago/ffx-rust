//! Every spell overlay binds every entry to a Rust port (docs/host.md,
//! "Overlays"; docs/formats/magic.md, "Porting the overlays"): each
//! `magicFiles/FFX/magic_*.dll` of the player's game is loaded through
//! `FFX.exe`'s own `LoadLibraryA`, as `MagicFile_Start` loads it, and
//! every address the game can enter its code by (the exports
//! `InitMagicPRX` and `GetEffectOverlayTable`, the entry point
//! `DllMain`, and every relocated word pointing into its code: the
//! overlay table's entries, the callbacks and tables it stores, `push
//! offset` operands) must bind to a port of `ffx-magic` by code identity.
//! `DllMain` (attach and detach), `InitMagicPRX` and
//! `GetEffectOverlayTable` run, all as ports: nothing of the overlay runs
//! in Unicorn. The player's files are read as data. Needs `FFX_GAME_DIR`;
//! skips without it.
//!
//! Two kinds of relocated word point into code without being an entry,
//! and stay unbound (the host offers them all; binding nothing there
//! costs nothing):
//! - a jump table's address and case labels (`jmp [reg*4 + T]`) and a
//!   byte index table's address (`movzx eax, byte [reg + B]`), tables
//!   in the code after their function: only that function reads them,
//!   and it is ported;
//! - the CRT's ([`CRT`], the same 9 functions in all 581 DLLs): its
//!   initialisers and `atexit` callbacks (run only by the original
//!   `DllMain`'s `_CRT_INIT`, which the port leaves out: docs/formats/
//!   magic.md, "The CRT"), `__security_check_cookie` (pushed for
//!   `_except_handler4`; the ports inline the check), and
//!   `_except_handler4` with the `__try` filters and handlers of its
//!   scope tables (reached only by an exception through an SEH frame the
//!   CRT set up; no port sets one up).
//!
//! `FFX_BIND_REPORT=<file>` writes one row per entry (DLL, RVA, the port
//! or what kind of unbound entry it is).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use ffx_bridge::Mem as _;
use ffx_host::exec::Exec;
use ffx_magic::{Callee, Image, Sig};

/// `g_MagicImports`, the table `MagicFile_Start` hands `InitMagicPRX`.
const MAGIC_IMPORTS: u32 = 0x00c6_4ce8;

/// The entries of the overlay table `GetEffectOverlayTable(0)` returns.
const TABLE_ENTRIES: u32 = 7;

/// A CRT function's identity: its `rel32` fields reach other CRT code
/// or import thunks, unchecked (as `DllMain`'s port).
const fn crt(len: u32, hash: u64, calls: &'static [(u32, Callee)]) -> Sig {
    Sig {
        len,
        hash,
        calls,
        imports: &[],
    }
}

const ANY: Callee = Callee::Any;

/// The CRT functions the relocated words of every overlay reach
/// (`magic_0003` addresses; `magic_0481`'s CRT tail from 0x100033b8 is
/// the same code), and whether they are entered at their start (an
/// entry) or only at labels inside (scope-table filters and handlers).
/// Names from the code's shape (MSVC 11's DLL start-up).
const CRT: [(&str, Sig, bool); 9] = [
    // cmp ecx, [cookie]; jne; repz ret; jmp __report_gsfailure
    (
        "__security_check_cookie (0x100076a1)",
        crt(15, 0x8032_9929_0a0c_a257, &[(0xb, ANY)]),
        true,
    ),
    // the first `__xi` initialiser: the onexit table (malloc 0x80),
    // then two atexit registrations (the next two rows)
    (
        "__xi initialiser: onexit table, atexit (0x100076b0)",
        crt(
            76,
            0xcc64_a97f_b0b4_5dc1,
            &[(0x2d, ANY), (0x37, ANY), (0x43, ANY)],
        ),
        true,
    ),
    // the second `__xi` initialiser: the onexit table (calloc 0x20, 4)
    (
        "__xi initialiser: onexit table (0x10007cd1)",
        crt(60, 0xb895_a140_eeb9_2186, &[]),
        true,
    ),
    // atexit callback: calls every word of an (empty) function table
    (
        "atexit callback: function table walk (0x10007ea3)",
        crt(32, 0xf08b_9b22_4068_920c, &[]),
        true,
    ),
    // atexit callback: one call with a data address
    (
        "atexit callback (0x10007ed0)",
        crt(12, 0x7eec_f4c7_b852_59ed, &[(0x6, ANY)]),
        true,
    ),
    // pushed by __SEH_prolog4 and the CRT's own SEH frames
    (
        "_except_handler4 (0x10007f39)",
        crt(35, 0x55cf_8d1d_741d_276e, &[(0x1a, ANY)]),
        true,
    ),
    (
        "__DllMainCRTStartup: __try labels (0x10007914)",
        crt(
            296,
            0x188c_b27e_ff7d_7194,
            &[
                (0x8, ANY),
                (0x31, ANY),
                (0x59, ANY),
                (0x65, ANY),
                (0x72, ANY),
                (0x7e, ANY),
                (0x98, ANY),
                (0xa4, ANY),
                (0xcb, ANY),
                (0xfd, ANY),
                (0x119, ANY),
                (0x120, ANY),
            ],
        ),
        false,
    ),
    (
        "CRT SEH function: __try labels (0x10007be0)",
        crt(186, 0xec97_1b92_670b_358f, &[(0x40, ANY), (0x5a, ANY)]),
        false,
    ),
    (
        "onexit registration: __try label (0x10007d0d)",
        crt(
            156,
            0xe9b2_fa08_3b91_7956,
            &[
                (0x8, ANY),
                (0x31, ANY),
                (0x65, ANY),
                (0x8d, ANY),
                (0x94, ANY),
            ],
        ),
        false,
    ),
];

fn game_dir() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    Some(PathBuf::from(dir))
}

/// The import `KERNEL32.dll!name`'s stub, as `FFX.exe`'s IAT holds it.
fn kernel32(exec: &Exec, name: &str) -> u32 {
    let slot = exec
        .image()
        .imports
        .iter()
        .find(|i| i.dll.eq_ignore_ascii_case("KERNEL32.dll") && i.name == name)
        .unwrap_or_else(|| panic!("{name} not imported"))
        .slot;
    exec.mem().read(slot)
}

/// A NUL-terminated copy of `s` in the arena.
fn put_str(exec: &mut Exec, s: &str) -> u32 {
    let at = exec.alloc(u32::try_from(s.len() + 1).unwrap());
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    exec.mem().write_bytes(at, &bytes);
    at
}

/// The player's overlays, by number.
fn overlays(dir: &std::path::Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir.join("magicFiles/FFX"))
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| {
            let l = n.to_ascii_lowercase();
            l.starts_with("magic_") && l.ends_with(".dll")
        })
        .collect();
    out.sort();
    out
}

const JUMP_TABLE: &str = "jump table: its address or a case label";
const BYTE_TABLE: &str = "byte index table (a jump table's)";

/// The tables MSVC puts in the code after a function, by address: for
/// each `jmp [reg*4 + T]` (`ff 24 <sib>`, scale 4, no base) the table
/// `T` and its case labels (the relocated words from `T` on, while they
/// point into the code), and for each `movzx r32, byte [reg + B]` (`0f
/// b6`, `disp32`; the byte index a jump table's `jmp` takes) the byte
/// table `B`. Only relocated operands count (`relocs`: the image's
/// relocated words, RVAs).
fn code_tables(img: &Image, relocs: &HashSet<u32>) -> HashMap<u32, &'static str> {
    let byte = |a: u32| img.word(a).to_le_bytes()[0];
    // a relocated word starting exactly at `a`
    let reloc_at = |a: u32| relocs.contains(&(a - img.base));
    let mut out = HashMap::new();
    for p in img.at(img.text.0)..img.at(img.text.1).saturating_sub(8) {
        if byte(p) == 0xff && byte(p + 1) == 0x24 && byte(p + 2) & 0xc7 == 0x85 && reloc_at(p + 3) {
            let mut t = img.word(p + 3);
            out.insert(t, JUMP_TABLE);
            while img.in_text(t) && reloc_at(t) && img.in_text(img.word(t)) {
                out.insert(img.word(t), JUMP_TABLE);
                t += 4;
            }
        } else if byte(p) == 0x0f && byte(p + 1) == 0xb6 && byte(p + 2) & 0xc0 == 0x80 {
            // mod 10: disp32 after the ModRM byte, or after a SIB byte
            let disp = if byte(p + 2) & 7 == 4 { p + 4 } else { p + 3 };
            if reloc_at(disp) && img.in_text(img.word(disp)) {
                out.entry(img.word(disp)).or_insert(BYTE_TABLE);
            }
        }
    }
    out
}

/// What an unbound entry `at` of `img` is: one of the tables in the code
/// ([`code_tables`]), or one of the [`CRT`] functions (its start, or a
/// label inside); `None` for anything else (an entry without a port).
fn unbound_kind(img: &Image, tables: &HashMap<u32, &'static str>, at: u32) -> Option<&'static str> {
    if let Some(&kind) = tables.get(&at) {
        return Some(kind);
    }
    for (name, sig, entry) in &CRT {
        let found = if *entry {
            img.matches(at, sig)
        } else {
            (at.saturating_sub(sig.len - 1)..at).any(|s| img.matches(s, sig))
        };
        if found {
            return Some(name);
        }
    }
    None
}

/// What one overlay's load gave.
#[derive(Debug, Default)]
struct Load {
    /// Entries without a port that are none of the allowed kinds.
    unbound: Vec<u32>,
    /// Overlay table entries without a port.
    table_unbound: Vec<u32>,
    emulated: u64,
    rust: u64,
}

/// Loads `dll` as the game does, runs `InitMagicPRX` and
/// `GetEffectOverlayTable(0)` through their exports and frees it; what
/// bound where goes into `report` (`dll\trva\tport or kind`) and the
/// allowed unbound kinds are counted in `kinds`.
fn load(
    exec: &mut Exec,
    dir: &std::path::Path,
    dll: &str,
    report: &mut String,
    kinds: &mut BTreeMap<&'static str, usize>,
) -> Load {
    let lla = kernel32(exec, "LoadLibraryA");
    let gpa = kernel32(exec, "GetProcAddress");
    let free = kernel32(exec, "FreeLibrary");
    let path = put_str(exec, &format!("magicFiles\\FFX\\{dll}"));
    let base = exec.call_stdcall(lla, &[path]).unwrap();
    assert_ne!(base, 0, "LoadLibraryA({dll})");
    let file = std::fs::read(dir.join("magicFiles/FFX").join(dll)).unwrap();
    let img = Image::new(dll, &file, base).unwrap();
    let relocs: HashSet<u32> = ffx_magic::pe::Pe::parse(&file)
        .unwrap()
        .relocs
        .into_iter()
        .collect();
    let tables = code_tables(&img, &relocs);
    let entries = ffx_host::library::bindings(base).unwrap_or_else(|| panic!("{dll}: no entries"));
    let mut out = Load::default();
    for &(at, port) in &entries {
        let what = match port {
            Some(p) => p,
            None => match unbound_kind(&img, &tables, at) {
                Some(kind) => {
                    *kinds.entry(kind).or_default() += 1;
                    kind
                }
                None => {
                    out.unbound.push(at);
                    "-"
                }
            },
        };
        let _ = writeln!(report, "{dll}\t{:#x}\t{what}", at - base);
    }
    let bound = |at: u32| entries.iter().any(|&(a, p)| a == at && p.is_some());
    let name_init = put_str(exec, "InitMagicPRX");
    let name_table = put_str(exec, "GetEffectOverlayTable");
    let init = exec.call_stdcall(gpa, &[base, name_init]).unwrap();
    let table_fn = exec.call_stdcall(gpa, &[base, name_table]).unwrap();
    assert!(init != 0 && table_fn != 0, "{dll}'s exports");
    assert!(bound(init), "{dll}: InitMagicPRX {init:#x} binds no port");
    assert!(
        bound(table_fn),
        "{dll}: GetEffectOverlayTable {table_fn:#x} binds no port"
    );
    exec.call_cdecl(init, &[MAGIC_IMPORTS]).unwrap();
    let table = exec.call_cdecl(table_fn, &[0]).unwrap();
    assert_ne!(table, 0, "{dll}: GetEffectOverlayTable(0)");
    for i in 0..TABLE_ENTRIES {
        let f = exec.mem().read::<u32>(table + 4 * i);
        if f != 0 && !bound(f) {
            out.table_unbound.push(f);
        }
    }
    assert_ne!(exec.call_stdcall(free, &[base]).unwrap(), 0, "FreeLibrary");
    (out.rust, out.emulated) = ffx_host::library::calls(base).unwrap();
    out
}

#[test]
fn every_overlay_binds_every_entry() {
    let Some(dir) = game_dir() else { return };
    let names = overlays(&dir);
    assert!(names.len() >= 581, "{} overlays", names.len());
    let mut exec = ffx_host::boot(&dir.join("FFX.exe")).unwrap();
    ffx_host::library::select_rust(None);
    let mut report = String::from("# dll\trva\tport or kind\n");
    let mut kinds = BTreeMap::new();
    let mut problems = Vec::new();
    for dll in &names {
        let l = load(&mut exec, &dir, dll, &mut report, &mut kinds);
        for f in &l.table_unbound {
            problems.push(format!("{dll}: overlay table entry {f:#x} binds no port"));
        }
        // DllMain attach and detach, InitMagicPRX, GetEffectOverlayTable
        if l.emulated != 0 || l.rust != 4 {
            problems.push(format!(
                "{dll}: {} calls in Rust (4 expected), {} in Unicorn",
                l.rust, l.emulated
            ));
        }
        for &at in &l.unbound {
            problems.push(format!("{dll}: entry {at:#x} binds no port"));
        }
    }
    if let Some(path) = std::env::var_os("FFX_BIND_REPORT") {
        std::fs::write(path, &report).unwrap();
    }
    let total = report.lines().count() - 1;
    eprintln!("{} overlays, {total} entries; unbound:", names.len());
    for (kind, n) in &kinds {
        eprintln!("  {n:6} {kind}");
    }
    assert!(
        problems.is_empty(),
        "{} problems:\n{}",
        problems.len(),
        problems.join("\n")
    );
    // every CRT function above is in every overlay, entered as counted
    let n = names.len();
    for (name, _, _) in &CRT {
        let k = kinds.get(name).copied().unwrap_or(0);
        assert!(k >= n, "{name}: {k} entries in {n} overlays");
    }
}
