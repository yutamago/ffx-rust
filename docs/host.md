# ffx-host and ffx-rust.exe

`ffx-rust.exe` runs FFX without loading `FFX.exe` (M5 stages 3 and 4,
`docs/rewrite-plan.md`). It maps the player's own `FFX.exe` as data, runs
the ports from one function table, and, during development only, runs every
function that has no port yet from `FFX.exe`'s `.text` in Unicorn, on the
same memory. The release runs no `FFX.exe` code.

## Crates

| Crate / file | What |
|---|---|
| `crates/ffx-host` (lib) | the arena, the loader, the function table, the executor, the imports |
| `src/arena.rs` | `Arena` (the game's 32-bit address space) and `ArenaMem` (`ffx_bridge::Mem` over it) |
| `src/image.rs` | maps `FFX.exe` into the arena at `0x00400000`; lists sections and imports |
| `src/table.rs` | `Table`, `Entry`, `Conv`, `Args`, `Ret`: every port under its original address |
| `src/dll.rs` | loads the player's `MSVCR110.dll` and `MSVCP110.dll` into the arena, relocated (development only) |
| `src/exec.rs` | `Shared` (what every game thread sees) and `Exec` (one game thread): the mixed Rust/Unicorn executor, `Exec::call`, threads, the stats |
| `src/exec/seh.rs` | structured exceptions: `RaiseException` and faults through the `fs:[0]` chain, `RtlUnwind`, the report of an unhandled one (below, "Exceptions") |
| `crates/ffx-fpu` | the x87 instructions Unicorn gets wrong, computed as the FPU does (moved out of `ffx-oracle`, which uses it too; below, "x87") |
| `src/imports.rs` | the Rust side of each import (C runtime, Win32 stand-ins, Steam, Iggy, milestones) |
| `src/win32.rs` | threads, synchronisation, TLS, modules and code pages: what the DLLs and the threads need |
| `src/sync.rs` | events, semaphores, mutexes, thread handles, critical sections on host primitives |
| `src/files.rs` | the `CreateFile` family over `std::fs`: the game folder (read-only) and a documents folder (read-write) |
| `src/ports.rs` | what this build registers in the table (by hand for now, see below) |
| `src/report.rs` | the run in numbers: Rust vs Unicorn functions (calls, time), imports, threads, the stop |
| `src/bin/ffx-rust.rs` | the program |
| `tests/boot.rs` | the Linux check against the image |
| `tests/x87.rs`, `tests/seh.rs` | MSVCR110's maths against the CPU's recorded results; throw and catch, faults |

```bash
cargo run --release -p ffx-host --bin ffx-rust -- --game-dir ~/steam-games/FFX-X2-HD \
    --documents <empty folder> --cover --budget 60 \
    [--profile FILE] [--routes FILE] [--count] [--no-dlls] [--lenient] [--stop-at DLL!name]
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test --release -p ffx-host
FFX_HOST_TRACE=200 ffx-rust ...   # the last 200 basic blocks of a thread that failed
tools/build-host.sh        # i686-pc-windows-msvc, through cargo xwin (Unicorn needs cmake and ninja, below)
```

- `--documents DIR`: the folder the game sees as `C:\FFXDocuments`
  (`SHGetFolderPathW`), where it reads and writes
  `Square Enix\FINAL FANTASY X&X-2 HD Remaster\GameSetting.ini` and the
  saves. Without it every file there is missing and the game's
  `Settings_Save` ends in the C runtime's invalid-parameter handler
  (`fwrite` on the NULL stream `fopen` gave it), as the original would.
- `--profile FILE`: every function that ran in Unicorn, tab-separated:
  module, start, name, calls, basic blocks, clock samples (about 1 ms
  each). Implies `--cover`.
- `--routes FILE`: every import of `FFX.exe` and the DLLs and where it went
  (`rust`, `dll`, `missing`).
- `--count`: counts instructions (a code hook on every instruction: about
  twice as slow; the report gives instructions per second).

## The arena

Every game address is a `u32` offset from `Arena::base`. The bridges'
`Mem` reads and writes through it, so a port runs unchanged on any target.

| | x86_64 (stage 4) | i686 (stage 3) |
|---|---|---|
| base | one 4 GB reservation (`mmap PROT_NONE` / `VirtualAlloc MEM_RESERVE`) | 0: the process's own address space |
| a region | committed inside the reservation (`mprotect` / `MEM_COMMIT`) | `VirtualAlloc` at its game address; refused if the address is taken |
| game address = host pointer | no: `base + addr` | yes |

Regions (`Arena::regions`):

| Region | Where |
|---|---|
| `FFX.exe` (headers and every section) | `0x00400000`–`0x0277d000`, its preferred base, so every `ffx.sym` address is its own game address and nothing is relocated |
| main stack (2 MB), stub page, thread block (GDT, TEB, PEB, TLS slots) | the first free gaps above `0x03000000` |
| heap (512 MB, the C runtime's and `HeapAlloc`) | the first gap above `0x10000000` |
| `VirtualAlloc` regions | the first free gaps above `0x03000000` |

Regions are 64 KB aligned, Windows' allocation granularity. Pages outside
every region stay unmapped: Unicorn reports an access there as
`BadAccess`, and `ArenaMem` reads zeros, drops the write and counts it
(the report lists the first ones).

On i686 `ffx-rust.exe` must leave `0x00400000`–`0x0277d000` free:
link it with `/BASE` above it (and `/DYNAMICBASE`), and map the image
before anything else allocates. Not done yet: the next step on Windows.

The release maps only the data sections (`.rdata`, `.data`, `.rodata`,
`_RDATA`, `.rsrc`). This build maps `.text` too, read as data, for Unicorn.

## The function table

`Table` maps an original address to an `Entry`: name, convention, stack
words, and the port, `fn(&mut Exec, &Args) -> Ret`. `Args` holds `ecx`,
`edx`, the stack words and the address of the first one (`slots`, for
originals that write their own argument slots). `Ret` says where the
result goes: `eax`, `edx:eax`, `ST0`, or nothing (the caller keeps its
registers, as the hook's thunks keep them).

Everything that calls code goes through the table:

- Unicorn code reaching a ported address: the address holds `int3` in the
  arena's copy of `.text`, and the executor runs the port;
- a port calling an address (`call_sym`, `call_virtual`, a stored function
  pointer): `Exec::call` takes the port when there is one, the import's
  Rust side for an import stub, else Unicorn;
- later, a mod replaces an entry by address.

### One source of truth

The ports are the hook's `ported!` lists (`crates/ffx-hook/src/ported/*.rs`,
15,158 entries on 2026-10-28). `ffx-host` is to compile those same files a
second time, with its own `ported/mod.rs`, instead of keeping a second
list:

- `ported!` expands to `Entry` registrations: `$addr`, `stringify!($name)`,
  the ABI string mapped by `Conv::from_abi`, the stack words from the
  argument sizes (`stack_words`, already in the macro), and an adapter that
  decodes `Args` into the shim's typed arguments (a `FromWords` trait for
  `u32`/`i32`/`u8`/`u16`/`bool`/`f32`, two words for `u64`/`f64`) and
  encodes the result into `Ret` (an `IntoRet` trait: integers to `eax`,
  64-bit to `edx:eax`, floats to `ST0`, `()` to nothing). No thunk, no
  patch;
- `mem()` returns `ArenaMem`; `StackFrame::at()` hands out words on a
  per-thread stack inside the arena, not the host stack;
- `call_cdecl`, `call_cdecl_words`, `call_thiscall` and the float forms
  call `Exec::call`.

What has to change first, all in shared code, so it is its own round:

- **A call router.** 61 shims call originals by `transmute(crate::rebase(..))`,
  22 `asm!` blocks call the CRT's x87 intrinsics through the import slots
  (`_CIsin`, `_CIcos`, `_CIpow`, ...), and the `Host` impls for `RawMem`
  (`ffx_bridge::Host`, `ffx_graphic::host::Host`, `ffx_script::NativeHost`)
  call native addresses. All of them must go through one routine,
  `ffx_bridge::route` (address, convention, words → `eax`/`edx`/`ST0`):
  a native call in the hooked game, `Exec::call` in `ffx-host`. The
  intrinsics become Rust functions with the CRT's results (they are imports
  of `MSVCR110`, and the CRT becomes Rust `std` anyway).
- **No host-only crate paths.** The shims use `crate::rebase`,
  `crate::log`, `crate::harness`, `crate::iggy_log`, `crate::loader_turn`,
  `crate::minimap`, `crate::hook_only` and `ffx_bridge::RawMem` directly
  (`sound.rs`); `ffx-host`'s `mod.rs` provides the same names.
- **Per-thread host state.** The router needs the current thread's `Exec`
  (a `thread_local!` set by the executor), since a port gets no context
  argument.

Until then `src/ports.rs` registers ports by hand through their portable
bridges (`Path_ToLowerAscii`, `MsCheckRange`), enough to prove both
directions.

## Rust and Unicorn

Unicorn maps every arena region with `mem_map_ptr` on the arena's own host
memory, so the ports and the original code share one memory, page for
page. Nothing runs inside a Unicorn hook except bookkeeping: an `int3`
stops the emulator, and the executor's loop decides.

- **Unicorn → Rust.** The `int3` at a ported address stops the emulator at
  the callee's entry (`esp` at the return address). The executor reads the
  entry's stack words, saves every general register, runs the port,
  restores them, applies `Ret`, and resumes at a return stub in the stub
  page: `ret 4n` for a callee-pops convention (stdcall, thiscall, fastcall)
  with `n` stack words, `ret` for cdecl and `fastcall-caller-pops`, and
  `fld qword [cell]; ret 4n` for a float result (the value is the port's
  `f64`, which the game's 53-bit x87 holds exactly). Saving and restoring
  `eax`, `ecx` and `edx` keeps what the LTCG callers expect: MSVC's
  whole-program optimisation lets a caller keep those live across a call it
  knows leaves them alone (`docs/hooking.md`, the thunks).
- **Rust → Unicorn.** `Exec::call` pushes the stack words below the
  current `esp` (a port runs between an original's call and its return, so
  that frame stays intact), pushes the sentinel (an `int3` in the stub page)
  as the return address, sets `ecx`/`edx` by the convention, empties the
  x87 stack, and runs until the sentinel. Then it reads `eax`, `edx` and
  `ST0`, and restores the registers it saved. Calls nest as plain Rust
  recursion; the innermost sentinel return belongs to the innermost call.
- **Conventions.** `Conv` is cdecl, stdcall, thiscall (`this` in `ecx`),
  fastcall (`ecx`, `edx`) and `FastcallCallerPops`, the LTCG custom
  convention (`ecx`, `edx`, stack words the caller pops: `ret`, then the
  caller's `add esp`). `Conv::callee_pops` picks the return stub.
- **Failures.** An unstubbed import, or a fault or exception no frame
  handles (below, "Exceptions"), ends the thread, on the
  main thread the whole run (`Exec::end`; a milestone or the budget end the
  run from any thread): every level returns `Error::Stopped`, since callers
  such as `_initterm` or a port can't all carry a `Result`. The outcome
  says where (`Stop::Failed(why, eip)`, the caller of an unstubbed import),
  the report adds the return addresses found on the stack, and
  `FFX_HOST_TRACE=n` the last `n` basic blocks.
- **x87.** The control word starts at `0x027f` (53-bit), as Windows starts
  threads. The transcendental instructions run exactly (below, "x87").

## x87

Unicorn (QEMU) computes `fsin`, `fcos`, `fsincos`, `fptan`, `fpatan`,
`f2xm1`, `fyl2x`, `fyl2xp1`, `fprem` and `fprem1` through host `double`
functions: 11 bits of the 64-bit mantissa are lost and `fprem`'s remainder
loses its sign, so MSVCR110's maths (which runs on them; `pow(3, 4)` came
out `81.00000000000003`) is wrong in the last bits. `ffx-fpu` (the oracle's
module, now a crate both use) computes them as an Intel FPU does: 320-bit
arithmetic rounded to the 64-bit mantissa in the control word's rounding
mode, the FPU's 66-bit pi for the argument reduction, `fprem` exact.

- **Where.** `ffx_fpu::sites` lists them per module from the PE header's
  `TimeDateStamp` and `CheckSum`: the player's `MSVCR110.dll`
  (11.0.51106.1) has 22, at RVAs from a linear disassembly; its
  `MSVCP110.dll` and `FFX.exe` have none (22 and 798 `0xd9 xx` byte pairs
  in `.text`, none an instruction start: the game's own maths calls the
  runtime or uses SSE). A module of another build gets every possible
  pair, watched by a code hook each (a hook fires only where an
  instruction really starts; slower, never patched).
- **How.** The known sites hold `int3` in the arena; the executor's
  interrupt hook computes the instruction on the emulator's registers
  (`ffx_fpu::unicorn::step`: `ST0`/`ST1`, the stack top and tags, the
  condition codes and flags) and the emulator goes on after it, without
  stopping. The report counts them ("exact x87 instructions").
- **Checked.** `tests/x87.rs` runs the 1,327 recorded cases of
  `ffx-oracle`'s `tests/data/msvcr110-math.tsv` (the cdecl maths exports,
  the `_CI*` intrinsics and the `_libm_sse2_*_precise` ones, recorded from
  the same DLL on an Intel CPU at control word `0x027f`) through the
  player's DLL in `ffx-host`: all match, bit for bit, NaNs included.

## Exceptions

Emulated code throws and catches as on 32-bit Windows (`src/exec/seh.rs`):

- **Dispatch.** `RaiseException` (a C++ `throw` is MSVCR110's
  `_CxxThrowException` → `RaiseException(0xe06d7363, ...)`) and faults
  (access violation `0xc0000005` with its read/write/execute and
  address, division by zero, illegal instruction, a stray `int3`) write an
  `EXCEPTION_RECORD`, a `CONTEXT` and a dispatcher context below the
  stack and call each handler on the thread's `fs:[0]` chain,
  `handler(record, frame, context, dispatcher context)`, as
  `RtlDispatchException` does: `ExceptionContinueSearch` goes to the next
  frame, `ExceptionContinueExecution` loads the registers (and the x87
  state) from the `CONTEXT`, `ExceptionNestedException` marks the record
  nested until past the frame it names.
- **In the executor's loop, not in Rust.** A handler is entered with a
  return address in the thread's stub page (an `int3`), so a handler that
  never returns (a catch: MSVCR110's `_JumpToContinuation` goes on in the
  catching function) just goes on, at no cost to the host stack. Each
  call is wrapped in a registration of the dispatcher's own, as
  `RtlpExecuteHandler2` does; MSVCR110's catch code relies on it
  (`_UnwindNestedFrames` saves and restores that head, `_JumpToContinuation`
  unlinks it), and its handler reports a nested exception.
- **`RtlUnwind(target, ip, record, value)`** (MSVCR110's import) calls the
  handlers from the head down to `target` with `EXCEPTION_UNWINDING` (and
  `EXCEPTION_EXIT_UNWIND` for 0), each in an unwind registration, unlinks
  each frame, and returns `value` to its caller (x86 ignores `ip`). These
  handlers return (they run destructors and `__finally` blocks), so they
  are called from Rust.
- **Across a Rust port.** When a port called the code that threw and the
  catch is in a frame older than the port, the catching function returns
  to an outer level's sentinel (`esp` tells which: each Rust → Unicorn
  call records the `esp` its sentinel return leaves). Every Rust level in
  between unwinds (`Exec::call` returns `Error::Stopped` to the port,
  whose further calls do nothing), and the outer level returns as if its
  own call had, with the registers the sentinel return left: the port's
  frame is abandoned, as an unwind abandons a frame without handlers.
  Ports hold no destructors that need to run, so nothing is lost.
- **Unhandled.** At the chain's end (no frame took it), or when the C
  runtime gives up on it (`UnhandledExceptionFilter`, which its start-up's
  `__except` and `terminate` call), the thread ends, on the main thread the
  run: `Stop::Failed` names the exception, for a C++ one its type from the
  throw information (`unhandled C++ exception std::bad_alloc`), with the
  `eip` of the throw or fault, the return addresses on the stack and the
  last blocks (`FFX_HOST_TRACE`). The report counts the exceptions
  dispatched.
- **Checked.** `tests/seh.rs` builds MSVC-shaped frames (an EH
  registration, `__CxxFrameHandler3` tables: unwind map, try block, catch
  handlers) and throws an `int` through the player's MSVCR110: caught
  across `FFX.exe`'s `FUN_00948c30` (`call [ebp+8]`), the try block's
  object destroyed by the unwind, the chain empty after; a `catch (...)`
  that rethrows (`throw;`) to an outer catch; the same catch across a Rust
  port in `FUN_00948c30`'s place (the port's frame abandoned, the
  catching function's tail run once); an access violation handled by an
  SEH handler that fixes the `CONTEXT` and continues; an unhandled throw
  ending the run with its report.
- **Not done.** Collided unwinds (an unwind started inside an unwind
  handler that reaches the outer one) go on as a plain continue; the
  vectored handlers (`AddVectoredExceptionHandler`) don't exist; a
  thread's top frame is the chain's end, not Windows' `BaseThreadInitThunk`
  filter (the same report either way); faults inside the Rust side
  (`ArenaMem`'s dropped writes) are not exceptions.

## Imports

`FFX.exe` imports 536 functions from 19 DLLs (MSVCR110 188, KERNEL32 112,
MSVCP110 75, USER32 41, iggy_w32 41, fmodex 23, fmod_event 17, steam_api 9,
ADVAPI32 7, the rest under 5 each). Each IAT slot points at its own `int3`
in the stub page; the four data imports (`_acmdln`, `_fmode`, `_commode`,
`_HUGE`) point at cells instead. `imports::TABLE` gives each import's
convention, stack words and Rust side.

- **C runtime and STL: the player's DLLs, some Rust.** See "The C runtime
  DLLs" below: `FFX.exe`'s `MSVCR110` and `MSVCP110` imports go to the
  player's own DLLs in Unicorn, except the 26 whose Rust side is exact
  (`RUST_WITH_DLLS`). `--no-dlls` runs the Rust side only, as the first
  round did (the run then stops at `MSVCP110.dll!basic_ios`); `--lenient`
  lets unstubbed `MSVCR110` functions (cdecl, so nothing to pop) return 0
  and only lists them.
- **Win32.** This round's stand-ins are deterministic and platform-free,
  so the Linux check runs them (fixed clocks, fake handles, files from the
  game folder through `std::fs`, read-only). On Windows the window, input,
  file and timer calls forward to the real APIs from a 64-bit-clean
  wrapper (pointer arguments translated through the arena base, handles
  through the handle table at stage 4).
- **D3D11, DXGI.** COM objects live outside the arena: the game gets a
  `u32` handle whose "vtable" is a table of stub addresses in the arena,
  one per method, each forwarding to the real interface. `D3D11CreateDevice*`
  is a milestone for now.
- **FMOD, Iggy, movies, Steam.** `rust_audio` (the M4 mixer), `ffx-swf`
  (Ruffle), `ffx-movie`, and `steam_api.dll` (`steam_api64.dll` at stage 4).
  This round's stand-ins, for the Linux check: Steam is absent
  (`SteamAPI_RestartAppIfNecessary` and `SteamAPI_Init` return false, so
  `Steam_Init` skips the interfaces and achievements); every Iggy export
  is a null stand-in that returns 0 (its stack size comes from the
  decorated name, `_IggyInit@4`).
- **The window.** `CreateWindowExA` returns a fake handle and records a
  milestone; `ShowWindow`, `SetWindowPos`, `GetClientRect` (1920 x 1080),
  `PeekMessage` (no messages) and the rest of the window calls on the way
  to the device are stand-ins.
- **Milestones.** The renderer's start-up first calls `D3D11CreateDevice`
  for the feature level only (no device, no context): `S_OK` and 11_0,
  recorded. `D3D11CreateDeviceAndSwapChain` (or `D3D11CreateDevice` asked
  for a device) ends the run: the Linux check can't go past it without COM
  stand-ins for the device, the context and the swap chain. `--stop-at
  DLL!name` stops at any import, the window included.

## The C runtime DLLs

The inlined STL templates in `FFX.exe` touch MSVCP110's objects directly
(`basic_ios`, `locale` facets, `basic_string`), so a Rust side must keep
MSVCP110's layouts. Until it does, the player's own `MSVCR110.dll` and
`MSVCP110.dll` (from the game folder, read as data like `FFX.exe`, never in
git) run in Unicorn beside `FFX.exe`, development only (`src/dll.rs`):

- mapped into the arena at the first free 64 KB gap above `0x04000000`
  (both prefer `0x10000000`, where the heap is), relocated
  (`IMAGE_REL_BASED_HIGHLOW`), then `DllMain(DLL_PROCESS_ATTACH)`, MSVCR110
  first, before the entry point; each new thread runs
  `DllMain(DLL_THREAD_ATTACH)` first;
- their `KERNEL32` imports go to the same Rust sides as `FFX.exe`'s
  (`src/win32.rs`: TLS slots in each thread's TEB, Windows-1252 code pages,
  the "C" locale, the standard handles, module handles; `GetProcAddress`
  finds nothing, so MSVCR110 takes its `Tls*` fallbacks for `Fls*`);
  `MSVCP110`'s `MSVCR110` imports go straight to the loaded MSVCR110's
  exports;
- `FFX.exe`'s data imports (`_acmdln`, `_fmode`, `_commode`, `_HUGE`)
  point at the DLL's own variables.

Which of `FFX.exe`'s imports go where (`ffx-rust --routes FILE` lists
every one):

| | Rust | DLL | missing |
|---|---|---|---|
| `MSVCR110.dll` (188) | 26: start-up and exit (`__set_app_type`, `__setusermatherr`, `__getmainargs`, `_initterm`, `_initterm_e`, `_onexit`, `__dllonexit`, `__crtGetShowWindowMode`, `_crt_debugger_hook`, `__crtSetUnhandledExceptionFilter`, `_amsg_exit`, `exit`, `_exit`, `_cexit`, `abort`), the heap (`malloc`, `calloc`, `free`, `_recalloc`, `_calloc_crt`), `memset`, `memcpy`, `memmove`, `_strdup`, `rand`, `srand` | the other 162 (`sprintf*`, `fopen`, `fwrite`, `_controlfp_s`, `_lock`, the `_CI*` math intrinsics, ...) | 0 |
| `MSVCP110.dll` (75) | 0 | 75 | 0 |
| the DLLs' own imports (383) | 123 (`KERNEL32`: the stand-ins above) | 170 (MSVCP110 → MSVCR110, all) | 90 (the rest of `KERNEL32`: `FindFirstFileExW`, `CreateProcessW`, timer queues, ...; none called on the way to the device) |

The Rust sides kept are exact: the heap is one heap (the DLL's `malloc`
calls `HeapAlloc`, whose Rust side allocates from the same arena heap as
the Rust `malloc`, so a block can be freed on either side), the copies
allow overlap as MSVCR110's do, and `rand` keeps MSVC's sequence per
thread (seed 1). Everything else goes to the DLL: the Rust `sprintf`,
`fopen`, `_controlfp_s` were approximations.

The DLLs' x87 transcendental instructions run exactly (above, "x87")
and their C++ exceptions dispatch through the SEH chain (above,
"Exceptions").

## Threads

Every game thread is a host thread with its own `Exec` (`src/exec.rs`):
its own Unicorn instance (Unicorn is single-threaded) mapping the same
arena regions, its own stack region (1 MB; the main thread 2 MB), thread
block (GDT, TEB with the TLS slots, PEB) and return stubs (the float
result cell is per thread). `Shared` holds what they share: the arena, the
image, the DLLs, the function table, the import bindings (read-only), the
heap (behind a lock), the files, the synchronisation objects.

- `CreateThread(attr, stack, entry, param, flags, &id)` starts a host
  thread that runs `DllMain(DLL_THREAD_ATTACH)` and then `entry(param)`
  (stdcall); `CREATE_SUSPENDED` waits for `ResumeThread`. The thread
  handle is signalled when the routine returns or calls `ExitThread`;
  `GetExitCodeThread` gives its code. `SubmitThreadpoolWork` runs the work
  item's callback on a thread of its own the same way.
- Synchronisation maps onto host primitives (`src/sync.rs`, one lock and
  one condition variable for all objects): events (manual and auto
  reset), semaphores, mutexes, thread handles, `WaitForSingleObject(Ex)`,
  `WaitForMultipleObjects`, and critical sections keyed by the game
  address of their `CRITICAL_SECTION` (owner and recursion count, so
  `TryEnterCriticalSection` and recursion behave). `Interlocked*` are host
  atomics on the arena word. `Sleep` really sleeps.
- Regions mapped at run time (`VirtualAlloc`, another thread's stack) are
  mapped into each thread's emulator before it next runs (the arena keeps
  a generation counter).
- Stopping: a failure on a game thread ends that thread (its handle is
  signalled, the report gives its outcome and return addresses); a failure
  on the main thread, a milestone, `exit` or the budget end the run. A
  watchdog thread then calls Unicorn's `uc_emu_stop` (thread-safe) on
  every live emulator, waits wake up within 50 ms, and every thread
  unwinds.
- Emulated `lock`-prefixed instructions are atomic within one Unicorn
  instance only; the game's own synchronisation goes through the imports
  above, which is what keeps the threads apart.

`tests/boot.rs`'s
`game_threads_run_in_parallel_under_the_games_own_critical_section`
proves it on emulated code: two threads started through `FFX.exe`'s own
`CreateThread` import each add to one counter 3,000 times with a plain
`inc` under one critical section (through the game's IAT), and the main
thread waits on both thread handles: the count is exactly 6,000.

## The harness: a second process

The stage 3 gate runs the replays against the original in a second
process (`docs/harness.md` gets a "Two processes" section then):

- `ffx-rust.exe` takes `FFX_HARNESS=<port>` like the hook and serves the
  same command set, from `ffx-host` (the harness code moves out of
  `ffx-hook` into a crate both link);
- `ffx-harness` starts it like `FFX.exe`: in a game slot, background mode
  (no focus, bottom of the window stack), the slot's sandboxed Documents
  folder; `Game::launch` gets an `Exe::Rust` option and `tools/run-game.sh`
  a `--rust` flag;
- a replay runs twice, once per program, with the same inputs and the
  virtual clock, and `replay-compare.py`, `shot-compare.py`, the draw-diff
  gates and the WAV checks compare the two runs as they compare two
  `FFX.exe` runs today.

Launching `ffx-rust.exe` on Windows counts as running the game: only
through the harness's background mode. This round did not launch it.

## The Linux check

`tests/boot.rs` (needs `FFX_GAME_DIR`, skips without it; run it with
`--release`):

- `unicorn_code_enters_a_port_and_matches_the_original`: code in the arena
  calls `MsCheckRange` the way the game does; once with the port (Unicorn
  → Rust), once with an empty table (all Unicorn); the results match;
- `game_threads_run_in_parallel_under_the_games_own_critical_section`:
  above, "Threads";
- `boots_past_the_window_to_the_d3d11_device_and_swap_chain`: with an
  empty documents folder, the DLLs start, the CRT start-up and `Game_Main`
  run, the game writes its default `GameSetting.ini`, creates the window,
  probes the feature level and stops at `D3D11CreateDeviceAndSwapChain`;
  the heap's decommit thread started on the way;
- `without_the_dlls_the_stl_is_the_wall`: `--no-dlls` still stops at the
  first `MSVCP110` import.

`tests/x87.rs` and `tests/seh.rs` (same conditions): MSVCR110's maths
against the CPU (above, "x87"), throw, catch, rethrow, faults and the
unhandled report (above, "Exceptions").

## Building for Windows

`tools/build-host.sh` (`build`, or `clippy` with cargo's arguments after
it) builds `ffx-rust.exe` for i686 through cargo xwin, like the hook.
Unicorn's C build needs cmake, ninja and `llvm-lib`, and cargo-xwin's
cmake toolchain file sets `CMAKE_MSVC_RUNTIME_LIBRARY`, which unicorn's
`CMakeLists.txt` refuses next to the `-MT` that `cc` passes. The script
supplies cmake and ninja through `uvx` when they are missing, links the
versioned `llvm-lib-<n>`, and points `CMAKE_TOOLCHAIN_FILE_i686-pc-windows-msvc`
at a copy of the toolchain file without that line (all under
`target/host-shims`). `build.rs` links the program at `/BASE:0x70000000`
with `/LARGEADDRESSAWARE`, so the image range is free and the arena can
use the upper 2 GB.

## Status (2026-10-29)

`ffx-rust` on the player's `FFX.exe` with its `MSVCR110.dll` and
`MSVCP110.dll` (Linux, release build, an empty documents folder):

- the DLLs start (`DllMain`), then the CRT start-up, the static
  initialisers and `Game_Main` run: `GameSetting.ini` is missing, so the
  game writes its default one (through MSVCP110's streams and MSVCR110's
  `fopen`/`fwrite` into the documents folder), Steam is absent, the Phyre
  framework registers its classes, Iggy initialises (null stand-ins),
  `CreateWindowExA` gets the fake window, the renderer probes the feature
  level (`D3D11CreateDevice` without a device: 11_0), and the run stops at
  `D3D11CreateDeviceAndSwapChain`, the first stop point the Linux check
  can't pass;
- 1,875 `FFX.exe` functions ran in Unicorn and 0 in Rust (no port on this
  path: the hand-registered ones aren't reached and the `ported!` lists
  aren't in the table yet), plus code in about 120 regions of the DLLs
  (named by their nearest export, which is not always the function);
  20,374 basic blocks;
- 5.9 million instructions, 23,137 emulator starts (every import and port
  call stops and resumes the emulator), 1,011 calls from Rust into Unicorn;
  0.26 to 1.4 s emulated, 0.36 to 1.9 s wall depending on the machine's
  load, so 4 to 23 million instructions a second (12 million with the
  instruction-counting hook on); the start-up code runs once, so Unicorn's
  translation of each new block dominates;
- where the time goes (`--profile`, samples of about 1 ms): `mdct_init`
  (the audio decoder's tables), `Phyre_PNamespace_getClassDescriptor`
  (407 calls, 708,848 blocks: a linear search over the class list),
  `Phyre_PClassDescriptor_calculateValidationBuffer` and
  `calculateDefaultBuffer`, MSVCP110's locale and number formatting;
- one game thread: the heap's decommit thread (`0x00948c30`, parameter
  `0x00947e30`, from `UserHeap` start-up) runs on its own host thread and
  Unicorn instance (`DLL_THREAD_ATTACH`, its TLS, then real waits on its
  event) until the run ends; no other thread starts before the device;
- imports: of `FFX.exe`'s 536 (and 4 data), 198 go to Rust, 237 to the
  DLLs, 105 are still missing; none of the missing ones is called before
  the device;
- with a timeout on every `emu_start` (Unicorn starts a timer thread for
  each one) the same run took 21 s; a watchdog thread that stops the
  emulators only when the run ends or runs out of time replaced it.

### What blocks the next step

1. **The D3D11 device.** Going past `D3D11CreateDeviceAndSwapChain` needs the COM
   stand-ins of "Imports" (handles with stub vtables): on Windows
   forwarding to the real device, on Linux recorded ones (the device, the
   immediate context, the swap chain, DXGI; the renderer's start-up right
   after `D3D11CreateDeviceAndSwapChain` queries interfaces and creates
   the back buffer views). The game's other threads start past it.
2. **The `ported!` lists in the table** (above, "One source of truth"):
   the call router in `ffx-bridge` (the host-table round), then the
   host-side `ported!`. Until then every function runs in Unicorn.
3. Done (2026-10-29): the DLLs' x87 maths is exact (above, "x87"), and
   structured exceptions and C++ `throw`/`catch` dispatch through the SEH
   chain (above, "Exceptions").
4. **Windows.** The forwarding imports (window, input, files, D3D11),
   `ffx-rust.exe` started by the harness in background mode.
