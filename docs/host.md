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
| `src/exec.rs` | `Exec`: the mixed Rust/Unicorn executor, `Exec::call`, the stats |
| `src/imports.rs` | the Rust side of each import (C runtime, Win32 stand-ins, milestones) |
| `src/files.rs` | the `CreateFile` family over `std::fs`, rooted at the game folder |
| `src/ports.rs` | what this build registers in the table (by hand for now, see below) |
| `src/report.rs` | the run in numbers: Rust vs Unicorn functions, imports, the stop |
| `src/bin/ffx-rust.rs` | the program |
| `tests/boot.rs` | the Linux check against the image |

```bash
cargo run -p ffx-host --bin ffx-rust -- --game-dir ~/steam-games/FFX-X2-HD --cover --budget 60 [--lenient] [--stop-at DLL!name]
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test -p ffx-host
tools/build-host.sh        # i686-pc-windows-msvc, through cargo xwin (Unicorn needs cmake and ninja, below)
```

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
- **Failures.** An unstubbed import, a fault or the budget ends the whole
  run (`Exec::end`): every level returns `Error::Stopped`, since callers
  such as `_initterm` or a port can't all carry a `Result`. The outcome
  says where (`Stop::Failed(why, eip)`, the caller of an unstubbed import).
- **x87.** The control word starts at `0x027f` (53-bit), as Windows starts
  threads. The oracle's exact transcendental instructions (`ffx-oracle`'s
  `x87`) are not wired in yet: they matter if MSVCR110's math runs
  emulated (below, "What blocks the next step").

## Imports

`FFX.exe` imports 536 functions from 19 DLLs (MSVCR110 188, KERNEL32 112,
MSVCP110 75, USER32 41, iggy_w32 41, fmodex 23, fmod_event 17, steam_api 9,
ADVAPI32 7, the rest under 5 each). Each IAT slot points at its own `int3`
in the stub page; the four data imports (`_acmdln`, `_fmode`, `_commode`,
`_HUGE`) point at cells instead. `imports::TABLE` gives each import's
convention, stack words and Rust side.

- **C runtime and STL → Rust.** `MSVCR110` functions are written in Rust
  (`malloc` on the arena heap, `_initterm` calling through `Exec::call`,
  `rand` with MSVC's sequence). `MSVCP110` (iostreams, locale, strings) is
  the next wall (below). `--lenient` lets unstubbed `MSVCR110` functions
  (cdecl, so nothing to pop) return 0 and only lists them, to see further.
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
- **Milestones.** `CreateWindowExA` and `D3D11CreateDevice*` end the run
  with a milestone; `--stop-at DLL!name` adds any import.

## Threads

The game starts its threads with `CreateThread` and the thread pool
(`docs/hooking.md`, "Threads": JobWorkers, the loader, the render thread,
VsyncEmulation, FMOD, the controller task manager). This round records
each `CreateThread` (entry, parameter) and runs none. Next:

- one host thread per game thread, each with its own Unicorn instance
  (Unicorn is single-threaded) mapping the same arena regions, its own
  stack region and TEB, and its own `Exec` state; the function table is
  shared and read-only;
- regions mapped at run time (`VirtualAlloc`, heap growth) are mapped into
  every thread's Unicorn (a region list with a generation counter checked
  at each emulator start);
- the heap behind a lock (the C runtime's is locked too);
- synchronisation imports (events, semaphores, critical sections,
  `Interlocked*`) on real host primitives, so the threads interleave as
  they do in `FFX.exe`. Emulated `lock`-prefixed instructions on shared
  arena memory are atomic within one Unicorn instance only: the threads'
  own sync goes through those imports, which is what the game does.

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

`tests/boot.rs` (needs `FFX_GAME_DIR`, skips without it):

- `unicorn_code_enters_a_port_and_matches_the_original`: code in the arena
  calls `MsCheckRange` the way the game does; once with the port (Unicorn
  → Rust), once with an empty table (all Unicorn); the results match;
- `boots_through_the_crt_into_win_main`: the CRT start-up, every static
  initialiser and `WinMain`'s start run under the executor and the run
  ends at a missing import, not at an executor fault.

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

## Status (2026-10-28)

`ffx-rust` on the player's `FFX.exe` (Linux, release build):

- the CRT start-up, 454 static initialisers (each registers its
  destructor with `_onexit`) and `Game_Main` (`WinMain`) run until
  `Game_Main` reads `GameSetting.ini` (`fopen`, not found in the sandboxed
  documents folder) and builds its first C++ stream: 139 functions in
  Unicorn (5,657 basic blocks), 0 in Rust (no port on this path: the two
  hand-registered ones aren't reached and the `ported!` lists aren't in
  the table yet);
- 889,693 instructions emulated, 1,006 calls from Rust into Unicorn
  (`_initterm` and `_initterm_e`'s initialisers), 3,287 emulator starts
  (every import and port call stops and resumes the emulator);
  0.42 to 0.68 s emulated, 0.45 to 0.7 s wall, so 1.3 to 2.1 million
  instructions a second on start-up code, which runs once and is
  dominated by Unicorn's translation of each new block;
- one `CreateThread` recorded, not run (`0x00948c30`, parameter
  `0x00947e30`);
- 139 of the 536 imports have a Rust side; the first unstubbed one is
  `MSVCP110.dll!std::basic_ios<char>::basic_ios()` from `0x00403772`.

### What blocks the next step

1. **MSVCP110.** The game's iostreams, locale and strings are MSVCP110
   classes whose members the inlined templates in `FFX.exe` touch
   directly, so the Rust side must keep MSVCP110's object layouts.
   Quickest way on: run the player's `MSVCP110.dll` and `MSVCR110.dll`
   in Unicorn too, development only like `FFX.exe`'s own code (mapped into
   the arena, relocated, their `KERNEL32` imports bound to the same stubs;
   `ffx-oracle` already loads MSVCR110 this way), then replace their
   functions with Rust as stage 1 reaches them.
2. **The `ported!` lists in the table** (above, "One source of truth"):
   the call router in `ffx-bridge`, then the host-side `ported!`.
3. **Windows.** The forwarding imports (window, input, files, D3D11 COM
   handles), `ffx-rust.exe` started by the harness in background mode, and
   the threads.

