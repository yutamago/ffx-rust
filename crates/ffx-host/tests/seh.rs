//! Structured exceptions and C++ `throw`/`catch` in emulated code
//! (docs/host.md, "Exceptions"): MSVCR110's `_CxxThrowException` and
//! `__CxxFrameHandler3` (the player's DLL) on hand-built frames that look
//! like MSVC's, across `FFX.exe`'s `FUN_00948c30` (`call [ebp+8]`, no
//! handler of its own) and across a Rust port in its place; rethrow;
//! `catch (...)`; a fault handled by an SEH handler that fixes the
//! context; an unhandled throw ending the run with a report. Needs
//! `FFX_GAME_DIR`; skips without it.

use std::collections::HashMap;
use std::path::PathBuf;

use ffx_bridge::Mem as _;
use ffx_host::exec::{Exec, Stop};
use ffx_host::table::{Conv, Entry, Port, Ret, Table};

/// `FUN_00948c30(f)`: `push ebp; mov ebp, esp; call [ebp+8]; pop ebp;
/// ret 4` (the heap's decommit thread routine).
const CALL_ARG: u32 = 0x0094_8c30;

fn boot(table: Table) -> Option<Exec> {
    let Some(dir) = std::env::var_os("FFX_GAME_DIR") else {
        eprintln!("FFX_GAME_DIR is not set; skipping");
        return None;
    };
    let exe = PathBuf::from(dir).join("FFX.exe");
    Some(ffx_host::boot_opts(&exe, table, &ffx_host::Options::default()).unwrap())
}

/// The IAT slot of `MSVCR110.dll!name` in `FFX.exe`.
fn slot(exec: &Exec, name: &str) -> u32 {
    exec.image()
        .imports
        .iter()
        .find(|i| i.dll.eq_ignore_ascii_case("MSVCR110.dll") && i.name == name)
        .unwrap_or_else(|| panic!("{name} not imported"))
        .slot
}

/// A few x86 instructions at a known base, with labels for absolute
/// addresses (`imm32` fixups).
struct Asm {
    base: u32,
    code: Vec<u8>,
    labels: HashMap<&'static str, u32>,
    fixups: Vec<(usize, &'static str)>,
}

impl Asm {
    fn new(base: u32) -> Self {
        Self {
            base,
            code: Vec::new(),
            labels: HashMap::new(),
            fixups: Vec::new(),
        }
    }

    fn here(&self) -> u32 {
        self.base + u32::try_from(self.code.len()).unwrap()
    }

    fn label(&mut self, name: &'static str) -> &mut Self {
        self.labels.insert(name, self.here());
        self
    }

    fn b(&mut self, bytes: &[u8]) -> &mut Self {
        self.code.extend_from_slice(bytes);
        self
    }

    fn d(&mut self, v: u32) -> &mut Self {
        self.code.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// An `imm32`/`disp32` that is a label's address.
    fn at(&mut self, label: &'static str) -> &mut Self {
        self.fixups.push((self.code.len(), label));
        self.d(0)
    }

    fn align(&mut self) -> &mut Self {
        while !self.code.len().is_multiple_of(16) {
            self.code.push(0xcc);
        }
        self
    }

    fn finish(mut self, exec: &mut Exec) -> HashMap<&'static str, u32> {
        for (off, l) in std::mem::take(&mut self.fixups) {
            let v = self.labels[l];
            self.code[off..off + 4].copy_from_slice(&v.to_le_bytes());
        }
        exec.write_code(self.base, &self.code);
        self.labels
    }
}

/// An MSVC-style function with a C++ try block and one catch, its
/// `__CxxFrameHandler3` tables, and a state-1 object whose destructor
/// counts (`<p>dtors`):
///
/// ```c++
/// int <p>(void (*f)()) {           // cdecl
///     int result = -1;
///     try { Counted c; CALL(f); }  // CALL: FUN_00948c30(f), or `call f`
///     catch (int e) { result = e; }       // `catch_all`: catch (...) {
///                                         //   ++<p>caught; throw; }
///     return result;
/// }
/// ```
#[allow(clippy::too_many_arguments)]
fn catcher(
    a: &mut Asm,
    labels: [&'static str; 9],
    via_ffx: bool,
    catch_all: bool,
    frame_handler_slot: u32,
    throw_slot: u32,
) {
    let [
        func,
        thunk,
        info,
        unwind,
        tries,
        handlers,
        dtor,
        funclet,
        after,
    ] = labels;
    a.align().label(func);
    a.b(&[0x55, 0x8b, 0xec]); // push ebp; mov ebp, esp
    a.b(&[0x6a, 0xff]); // push -1 (state)
    a.b(&[0x68]).at(thunk); // push handler
    a.b(&[0x64, 0xa1]).d(0); // mov eax, fs:[0]
    a.b(&[0x50]); // push eax
    a.b(&[0x64, 0x89, 0x25]).d(0); // mov fs:[0], esp
    a.b(&[0x83, 0xec, 0x0c]); // sub esp, 12
    a.b(&[0x89, 0x65, 0xf0]); // mov [ebp-0x10], esp
    a.b(&[0xc7, 0x45, 0xe8]).d(u32::MAX); // mov [ebp-0x18], -1
    a.b(&[0xc7, 0x45, 0xfc]).d(1); // mov [ebp-4], 1 (in try, Counted alive)
    if via_ffx {
        a.b(&[0xff, 0x75, 0x08]); // push [ebp+8]
        a.b(&[0xb8]).d(CALL_ARG); // mov eax, FUN_00948c30
        a.b(&[0xff, 0xd0]); // call eax
    } else {
        a.b(&[0xff, 0x55, 0x08]); // call [ebp+8]
    }
    a.b(&[0xc7, 0x45, 0xfc]).d(u32::MAX); // mov [ebp-4], -1
    a.label(after);
    a.b(&[0xff, 0x05]).at("afters"); // inc dword [afters]
    a.b(&[0x8b, 0x45, 0xe8]); // mov eax, [ebp-0x18]
    a.b(&[0x8b, 0x4d, 0xf4]); // mov ecx, [ebp-0xc]
    a.b(&[0x64, 0x89, 0x0d]).d(0); // mov fs:[0], ecx
    a.b(&[0x8b, 0xe5, 0x5d, 0xc3]); // mov esp, ebp; pop ebp; ret
    // the catch block (ebp = the frame's)
    a.align().label(funclet);
    a.b(&[0xc7, 0x45, 0xfc]).d(2); // mov [ebp-4], 2 (the catch state, as MSVC's code does)
    if catch_all {
        a.b(&[0xff, 0x05]).at("caught"); // inc dword [caught]
        a.b(&[0x6a, 0x00, 0x6a, 0x00]); // push 0; push 0 (throw;)
        a.b(&[0xff, 0x15]).d(throw_slot); // call [_CxxThrowException]
        a.b(&[0xcc]);
    } else {
        a.b(&[0x8b, 0x45, 0xec]); // mov eax, [ebp-0x14] (e)
        a.b(&[0x89, 0x45, 0xe8]); // mov [ebp-0x18], eax
        a.b(&[0xb8]).at(after); // mov eax, continuation
        a.b(&[0xc3]);
    }
    // Counted's destructor (an unwind funclet)
    a.align().label(dtor);
    a.b(&[0xff, 0x05]).at("dtors"); // inc dword [dtors]
    a.b(&[0xc3]);
    a.align().label(thunk);
    a.b(&[0xb8]).at(info); // mov eax, FuncInfo
    a.b(&[0xff, 0x25]).d(frame_handler_slot); // jmp [__CxxFrameHandler3]
    // FuncInfo: magic, maxState, unwind map, try blocks, IP map, ES list, flags
    a.align().label(info);
    a.d(0x1993_0522)
        .d(3)
        .at(unwind)
        .d(1)
        .at(tries)
        .d(0)
        .d(0)
        .d(0)
        .d(0);
    // states: 0 (try) → -1; 1 (Counted) → 0, its destructor; 2 (catch) → -1
    a.label(unwind);
    a.d(u32::MAX).d(0).d(0).at(dtor).d(u32::MAX).d(0);
    // try 0..1, catch state 2, one handler
    a.label(tries);
    a.d(0).d(1).d(2).d(1).at(handlers);
    // adjectives, type (0: catch (...)), catch object at ebp-0x14, block
    a.label(handlers);
    a.d(0);
    if catch_all {
        a.d(0);
    } else {
        a.at("int_type");
    }
    a.d((-0x14i32).cast_unsigned()).at(funclet);
}

/// Everything the tests use, in one code block:
/// - `catch_int(f)`: [`catcher`], through `FUN_00948c30`;
/// - `rethrower()`: [`catcher`] with `catch (...) { ++caught; throw; }`
///   around a direct call of `thrower`;
/// - `thrower()`: `throw value;` (an `int`);
/// - `fault()`: reads address 0 under an SEH handler that skips the
///   instruction, sets `eax` to `0x1234` and continues.
fn program(exec: &mut Exec) -> HashMap<&'static str, u32> {
    let base = exec.alloc_code(&[0xcc; 0x800]).unwrap();
    let handler3 = slot(exec, "__CxxFrameHandler3");
    let throw = slot(exec, "_CxxThrowException");
    let mut a = Asm::new(base);
    catcher(
        &mut a,
        ["catch_int", "t1", "i1", "u1", "y1", "h1", "d1", "f1", "a1"],
        true,
        false,
        handler3,
        throw,
    );
    catcher(
        &mut a,
        ["rethrower", "t2", "i2", "u2", "y2", "h2", "d2", "f2", "a2"],
        false,
        true,
        handler3,
        throw,
    );
    // thrower: value is thrown from a global
    a.align().label("thrower");
    a.b(&[0xa1]).at("value"); // mov eax, [value]
    a.b(&[0xa3]).at("object"); // mov [object], eax
    a.b(&[0x68]).at("throw_info"); // push ThrowInfo
    a.b(&[0x68]).at("object"); // push &object
    a.b(&[0xff, 0x15]).d(throw); // call [_CxxThrowException]
    a.b(&[0xcc]);
    // fault: a registration of its own, a read of 0
    a.align().label("fault");
    a.b(&[0x68]).at("fix"); // push fix
    a.b(&[0x64, 0xff, 0x35]).d(0); // push dword fs:[0]
    a.b(&[0x64, 0x89, 0x25]).d(0); // mov fs:[0], esp
    a.b(&[0x31, 0xc0]); // xor eax, eax
    a.b(&[0x8b, 0x00]); // mov eax, [eax]  (faults)
    a.b(&[0x8b, 0x0c, 0x24]); // mov ecx, [esp]
    a.b(&[0x64, 0x89, 0x0d]).d(0); // mov fs:[0], ecx
    a.b(&[0x83, 0xc4, 0x08, 0xc3]); // add esp, 8; ret
    // fix(record, frame, context, dc): note the code, skip 2 bytes, eax = 0x1234
    a.align().label("fix");
    a.b(&[0x8b, 0x4c, 0x24, 0x04, 0x8b, 0x09]); // mov ecx, [esp+4]; mov ecx, [ecx]
    a.b(&[0x89, 0x0d]).at("code"); // mov [code], ecx
    a.b(&[0x8b, 0x44, 0x24, 0x0c]); // mov eax, [esp+12]
    a.b(&[0x83, 0x80, 0xb8, 0x00, 0x00, 0x00, 0x02]); // add dword [eax+0xb8], 2
    a.b(&[0xc7, 0x80, 0xb0, 0x00, 0x00, 0x00]).d(0x1234); // mov [eax+0xb0], 0x1234
    a.b(&[0x31, 0xc0, 0xc3]); // xor eax, eax (ExceptionContinueExecution); ret
    // data: counters, the thrown int, its type
    a.align()
        .label("dtors")
        .d(0)
        .label("caught")
        .d(0)
        .label("code")
        .d(0);
    a.label("value")
        .d(0)
        .label("object")
        .d(0)
        .label("afters")
        .d(0);
    // TypeDescriptor {vftable, spare, ".H"} (int)
    a.label("int_type").d(0).d(0).b(b".H\0\0");
    // CatchableType {simple, type, this displacement (0, -1, 0), size 4, no copy}
    a.label("catchable")
        .d(1)
        .at("int_type")
        .d(0)
        .d(u32::MAX)
        .d(0)
        .d(4)
        .d(0);
    a.label("catchables").d(1).at("catchable");
    // ThrowInfo {attributes, unwind, forward compat, catchable types}
    a.label("throw_info").d(0).d(0).d(0).at("catchables");
    assert!(a.code.len() <= 0x800);
    a.finish(exec)
}

fn word(exec: &Exec, at: u32) -> u32 {
    exec.mem().read(at)
}

fn set(exec: &Exec, at: u32, v: u32) {
    let mut m = exec.mem();
    m.write(at, v);
}

fn fs0(exec: &Exec) -> u32 {
    word(exec, exec.teb)
}

#[test]
fn a_throw_is_caught_across_an_ffx_exe_frame() {
    let Some(mut exec) = boot(Table::new()) else {
        return;
    };
    let l = program(&mut exec);
    for (n, v) in [(1, 42), (2, 7)] {
        set(&exec, l["value"], v);
        let r = exec.call_cdecl(l["catch_int"], &[l["thrower"]]).unwrap();
        assert_eq!(r, v, "the caught value");
        assert_eq!(
            word(&exec, l["dtors"]),
            n,
            "the try block's object destroyed"
        );
        assert_eq!(fs0(&exec), u32::MAX, "the chain is empty again");
    }
    assert_eq!(exec.exceptions_raised(), 2);
    // without a throw: the try block ends normally
    let ret_minus_one = exec.alloc_code(&[0xc3]).unwrap();
    let r = exec.call_cdecl(l["catch_int"], &[ret_minus_one]).unwrap();
    assert_eq!(r, u32::MAX);
}

#[test]
fn a_rethrow_from_catch_all_reaches_the_outer_catch() {
    let Some(mut exec) = boot(Table::new()) else {
        return;
    };
    let l = program(&mut exec);
    set(&exec, l["value"], 99);
    // catch_int(rethrower), rethrower's try calls thrower: [ebp+8] of
    // rethrower is FUN_00948c30's caller's word, so pass thrower through a
    // trampoline: push thrower; call rethrower; add esp, 4; ret
    let mut t = vec![0x68];
    t.extend(l["thrower"].to_le_bytes());
    t.push(0xe8);
    let at = exec.alloc_code(&[0; 16]).unwrap();
    let rel = l["rethrower"].wrapping_sub(at + 10);
    t.extend(rel.to_le_bytes());
    t.extend([0x83, 0xc4, 0x04, 0xc3]);
    exec.write_code(at, &t);
    let r = exec.call_cdecl(l["catch_int"], &[at]);
    let r = r.unwrap_or_else(|_| panic!("{:?} {:x?}", exec.stopped(), exec.trace));
    assert_eq!(r, 99);
    assert_eq!(word(&exec, l["caught"]), 1, "catch (...) ran");
    assert_eq!(
        word(&exec, l["dtors"]),
        2,
        "both try blocks' objects destroyed"
    );
    assert_eq!(fs0(&exec), u32::MAX);
    assert_eq!(exec.exceptions_raised(), 2, "the throw and the rethrow");
}

/// `FUN_00948c30` as a Rust port that calls its argument through
/// `Exec::call`: the catch continues in `catch_int`'s frame, older than
/// the port's Rust frame, which is abandoned.
#[test]
fn a_throw_is_caught_across_a_rust_port() {
    let mut table = Table::new();
    table.register(
        CALL_ARG,
        Entry {
            name: "call_arg",
            conv: Conv::Stdcall,
            words: 1,
            port: Port::Exec(|e, args| {
                let _ = e.call_cdecl(args.stack[0], &[]);
                Ret::VOID
            }),
        },
    );
    let Some(mut exec) = boot(table) else { return };
    let l = program(&mut exec);
    set(&exec, l["value"], 5);
    let r = exec.call_cdecl(l["catch_int"], &[l["thrower"]]).unwrap();
    assert_eq!(r, 5);
    assert_eq!(exec.stats.ports.get(&CALL_ARG), Some(&1));
    // catch_int's tail ran once: the port's frame was abandoned, not
    // returned into
    assert_eq!(word(&exec, l["afters"]), 1);
    assert_eq!(fs0(&exec), u32::MAX);
    assert!(exec.stopped().is_none());
    // and the executor goes on normally
    set(&exec, l["value"], 6);
    let r = exec.call_cdecl(l["catch_int"], &[l["thrower"]]).unwrap();
    assert_eq!(r, 6);
    assert_eq!(word(&exec, l["afters"]), 2);
}

#[test]
fn a_fault_goes_to_the_frames_handler_which_continues() {
    let Some(mut exec) = boot(Table::new()) else {
        return;
    };
    let l = program(&mut exec);
    let r = exec.call_cdecl(l["fault"], &[]).unwrap();
    assert_eq!(r, 0x1234);
    assert_eq!(word(&exec, l["code"]), 0xc000_0005, "an access violation");
    assert_eq!(fs0(&exec), u32::MAX);
}

#[test]
fn an_unhandled_throw_ends_the_run_with_a_report() {
    let Some(mut exec) = boot(Table::new()) else {
        return;
    };
    let l = program(&mut exec);
    set(&exec, l["value"], 3);
    let r = exec.call_stdcall(CALL_ARG, &[l["thrower"]]);
    assert!(r.is_err());
    match exec.stopped() {
        Some(Stop::Failed(why, eip)) => {
            assert!(
                why.contains("unhandled C++ exception int"),
                "the report names it: {why}"
            );
            assert_ne!(*eip, 0);
        }
        s => panic!("{s:?}"),
    }
}
