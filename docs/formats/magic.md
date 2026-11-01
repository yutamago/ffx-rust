# Spell overlays and `g_MagicImports`

The spell and summon effects are 581 DLLs in `magicFiles/FFX/`
(`magic_0003.dll` … `magic_0710.dll`), the PS2's effect overlays compiled
for Windows (FFX-2 has its own set in `magicFiles/FFX-2/`, not counted
here). Each exports `GetEffectOverlayTable` and `InitMagicPRX`.

`onMagicDLLLoaded` (`0x009db110`) looks up `InitMagicPRX`, and
`MagicFile_Start` (`0x009da7f0`) calls it with `g_MagicImports`
(`0x00c64ce8`): 947 slots of 4 bytes (`0xecc`), the overlays' only way into
the game. The table is complete when `FFX.exe` is loaded: every slot is a
relocated `.data` word (or 0), and nothing in `FFX.exe` writes it. Its
only reference is `MagicFile_Start`'s `push 0x00c64ce8` (`0x009da81f`), and
no init code or registration function fills it. It ends where the next
object starts: a zero padding word at `+0xecc`, then an RTTI type
descriptor at `0x00c65bb8`. `tools/reach.py` (`Image.magic_imports`) reads
the length from the image this way. The overlays call slots up to 946
(`+0xec8`).

- Slots 0–740: 741 code pointers. Slots 728–740 are the runtime's.
- Slots 741–765: 25 data pointers. `0x0230ffe0` (next to
  `g_pppCurrentPart`), `0x02332e8c`, `g_OpusBattle` (`0x012a40c0`) and
  further game variables. `InitMagicPRX` copies them into its own globals
  (it reads `+0xb94` onwards, up to `+0xea0`).
- Slots 766–946: 172 code pointers and 8 more data pointers (769, 827,
  850, 880, 919, 921, 935, 936). Slot 854 is null, and no overlay uses it.

`InitMagicPRX` keeps the table's address in a global of its own. Every
later call loads that global and calls through `[table + 4 * slot]`.

The slot list below covered only slots 0–743 until 2026-11-01. In that
earlier list, "744 slots (`0xba0`)" was just where the list stopped, not
the end of the table. `tools/reach.py` already reached slots 744–946 as
plain `.data` pointers, so its verdicts did not change. No function was
dropped wrongly.

Replacing a slot's function in `FFX.exe` (`docs/hooking.md`) therefore
covers every overlay at once: the slots point at the patched originals.

## How the use counts were made

For each overlay: find `InitMagicPRX` (export ordinal 2), take the global it
stores the table pointer in (the first `mov ds:G, eax` after it loads its
argument), then scan the whole disassembly (`objdump -d -M intel`): a
register loaded from `ds:G` is tracked until it is overwritten or a call
clobbers it, and each `[reg + off]` through a tracked register counts as a
use of slot `off / 4`. *overlays* is the number of DLLs with at least one
use, *refs* the total. It is a static count of call sites, not of calls
made at run time; a pointer copied elsewhere before use would be missed.
Slots 728–740 are used by all 581 overlays: each carries the same small
wrappers around them. Rows 744–946 were added on 2026-11-01 and counted
the same way (`tools/overlay-census.py`'s register tracking over each whole
`.text`). The same recount gives the earlier numbers for 741 of slots
0–743. The other three are off by at most one reference, except slot 0
(28 overlays here, 0 in the earlier count).

*module* is `tools/progress.py`'s placement (a `?` marks a function the
Ghidra dump does not list, placed by the function before it). *name* is
`symbols/ffx.sym`'s; in parentheses, the particle program table record
(`g_pppProgramTable`, `0x00c3a508`) and field when only that names it.

Slot functions by module: effects 327, battle 115, map 115, characters 84,
system 36, main 25, PS2 emulation 22, script 6, menu 6, CRT 4, movie 1;
3 data slots. 730 of the 744 slots are used by at least one overlay.
Slots 744–946: render 89, effects 39, PS2 emulation 15, system 10, menu 6,
battle 5, characters 5, movie, map and main 1 each; 30 data slots and the
null slot 854. At least one overlay uses 128 of these 203 slots. Data
slots count 0 here: `InitMagicPRX` reads them through its argument, not
through the global, except slot 749 (2 overlays).

## Functions the Ghidra dump hides

The dump (`tools/ghidra/dump.sh`) folded some functions into the one
before them or left them undefined, so `tools/check-ported.py` and
`tools/check-clobber.py` could not see them. `tools/ghidra/ApplySymbols.java`
now creates a function wherever `symbols/ffx.sym` has a `function` line, so
naming them is enough:

- `pppKeMatSN` (`0x00734020`, program table record `pppKeMatSN` step): its
  aligned-stack prologue (`push ebx; mov ebx, esp; and esp, -16`) made
  Ghidra treat it as part of `FUN_00733ff0`. In the dump since the round 5
  re-dump, and ported.
- 83 slots pointed at addresses outside every dumped function. The
  program-table names (pppDrawMdl `0x00737770`, pppDrawMdlTs `0x00737f40`,
  ...) brought 33 of them in. The other 50 now have `function` lines too
  (evidence in their comments): thunks (`jmp` to a named function or an
  `MSVCR110` import), lone `ret`/`xor eax, eax; ret` bodies, small `TO*`
  scene wrappers, `rcef*` helpers, and slot 733 `0x00a74d30`, which every
  overlay uses and which has the same aligned-stack prologue as
  `pppKeMatSN`.

## Running the original overlays

The overlays are not ported on M5's path (user, 2026-10-28): the 64-bit
program runs the player's own DLLs in Unicorn inside its arena, which
makes them identical by construction. `crates/ffx-overlay` does it, and
`ffx-host` embeds it unchanged. Unicorn is C, the one exception to "pure
Rust", for the overlays only. Unicorn is GPL-2.0 (`unicorn-engine` and
`unicorn-engine-sys` both say so), so a binary that links it is GPL-2.0
as a whole; the sources stay MIT OR Apache-2.0.

**What the 581 DLLs need.** Every one has the same five sections (`.text`,
`.rdata`, `.data`, `.rsrc`, `.reloc`), a preferred base of `0x10000000`,
base relocations, no TLS directory, and the same 25 imports for the CRT's
start-up: `MSVCR110` `_lock`, `_unlock`, `_malloc_crt`, `_calloc_crt`,
`free`, `__dllonexit`, `_onexit`, `_initterm`, `_initterm_e`,
`__clean_type_info_names_internal`, `_amsg_exit`, `_except_handler4_common`,
`__CppXcptFilter`, `__crtTerminateProcess`, `__crtUnhandledException`,
`_crt_debugger_hook`, and `KERNEL32` `GetTickCount64`,
`GetSystemTimeAsFileTime`, `GetCurrentThreadId`, `QueryPerformanceCounter`,
`IsProcessorFeaturePresent`, `IsDebuggerPresent`, `EncodePointer`,
`DecodePointer`, `DisableThreadLibraryCalls`. Some import more: `memset`
(281 DLLs), `_libm_sse2_sqrt_precise` (121), `_CIatan2` (90), `rand` (81),
`_libm_sse2_atan_precise` (18), `_libm_sse2_sin_precise` (6),
`_libm_sse2_cos_precise` (4), `memcpy` (2) and `_libm_sse2_acos_precise`
(1). Everything else goes through `g_MagicImports`. `GetEffectOverlayTable(0)`
returns a table in `.data` whose first 7 words point into `.text`
(`magic_0003`, `magic_0150`, `magic_0481`); the battle code calls them by
offset (`+0x4`, `+0xc`, `+0x10`, ...).

**The crate (`ffx_overlay`).**
- `configure(Config { base, service, import_table })` once per process.
  Guest address `a` lives at host address `base + a`: 0 in the 32-bit
  game, the arena's start in `ffx-host`. `service` is a 64 KiB guest range
  nothing else uses (the import traps; its last 16 bytes are the return
  sentinel). `import_table` is `g_MagicImports` (address, slot count),
  so a call-out to one of its entries reports the slot. The table has
  947 slots. The 32-bit gate (`overlay_emu.rs`) still passes 744, so a
  call-out to slots 744–946 reports no slot. Its `call_out` ignores the
  slot, so the gate's behaviour is the same.
- `Pe::parse(file)` reads a DLL as data. The embedder reserves
  `pe.span()` bytes at a 64 KiB-aligned base, makes everything writable
  except `pe.text_range()`, and calls `Overlay::load(name, file, &pe, base)`:
  headers and data sections are written there, relocated, with the import
  address table pointing at the traps. The code itself is kept in a
  private relocated copy that Unicorn runs; nothing writes the real
  `.text` range.
- `Overlay::entry_point()` (`DllMain`), `export("InitMagicPRX")`,
  `export("GetEffectOverlayTable")`, `table_entries(table)` give
  addresses. `OverlayFn(addr).call(host, stack_top, ecx, &args)` runs one
  on a stack the embedder owns; `ffx_overlay::call(host, pc, &regs)` runs
  from any state (`regs.esp` at the return address). `Overlay::unload()`
  forgets one; the embedder runs `DllMain(base, 0, 0)` first and frees the
  memory after.
- The embedder implements `Host`:
  - `call_out(target, slot, &mut regs)`: overlay code jumped to `target`,
    outside every overlay's code. `regs.esp` points at the return address,
    the arguments above it. Run the function and leave the state after its
    `ret` in `regs`: `esp` past the return address and the bytes it
    popped, `eax`/`edx`, and `st[0]` with `st_len` 1 for a float result.
    In `ffx-host` this is the function table, by address.
  - `import(name, &mut regs)`: the imports whose results must be the
    game's own: `rand` (the game's sequence, shared with `FFX.exe`'s
    calls), `_CIatan2` (x87: two values in `regs.st`), and the four
    `_libm_sse2_*_precise` without a stand-in (`xmm0` in and out). The
    others have stand-ins in the crate: the CRT start-up ones, `memset`,
    `memcpy`, `sqrt` (exact on both sides), and fixed `KERNEL32` answers
    (they only feed the security cookie).
  - `alloc`/`free`: zeroed guest memory for the overlays' CRT and each
    thread's Unicorn bookkeeping (8 KiB: a descriptor table and the
    `fs` block).
- Memory: Unicorn maps guest memory the first time overlay code touches
  it, in 1 MiB blocks (64 KiB next to an overlay's image), read-write and
  never executable, from `base + a`. The first 64 KiB stay unmapped. Only
  overlay code is executable, so any jump elsewhere stops Unicorn and
  becomes an import, a call-out, or the return to the caller. An
  overlay's `fs` points at a private thread block, so its exception frames
  never enter the real thread's chain.
- Threads: one Unicorn instance per thread that runs overlay code, made on
  first use (the async loader runs `DllMain`, the main thread the rest).
  Calls nest: a call-out may run code that calls back into an overlay on
  the same thread (the instance's registers are saved around it). Loading
  or unloading an overlay bumps a generation; each instance drops stale
  mappings and translated code the next time it runs.

**What `ffx-host` provides at stage 3.** The arena as `base`; a service
range and 8 KiB per thread from its heap; `g_MagicImports` mapped from the
player's `FFX.exe` like the other data; `call_out` through the function
table (every slot is a ported function's address); `rand` and the libm
functions through the CRT ports (`rand`'s sequence and MSVCR110's results,
`crates/ffx-oracle/tests/runtime.rs`); `LoadLibraryA`/`GetProcAddress`/
`FreeLibrary` of `PhyreFIOS_LoadsPrx`, `MagicDLLAsyncLoad`,
`onMagicDLLLoaded` and `PhyreFIOS_UnloadPrx` answered with
`Overlay::load`/`export`/`unload` and the image base as the module handle;
and a call of a function-table address inside an overlay's code (the
overlay table's entries, `InitMagicPRX` through `0x022fb6a0`, callbacks
an overlay stored) answered with `ffx_overlay::call` on the caller's stack.

**The gate in the 32-bit game (`FFX_MAGIC_EMU=1`).**
`crates/ffx-hook/src/overlay_emu.rs` wraps `FFX.exe`'s `LoadLibraryA`,
`GetProcAddress` and `FreeLibrary` imports. A `magicFiles\...\magic_*.dll`
is loaded by `ffx-overlay` into memory reserved at its preferred base;
its `.text` pages stay reserved, so any native jump into them faults, and
a vectored handler continues the fault in a thunk that runs the call in
Unicorn (identity mapping: `base` 0, the game's own memory, the caller's
own stack, with the driver 64 KiB below it). Call-outs call the real
address natively with the overlay's registers, its x87/SSE state and 32
argument words copied below the driver (words the callee changed are
copied back); `rand` and the libm imports call the game's `MSVCR110`. The
cost goes to `ffx-hook.log` when an overlay is unloaded (`overlay-emu:`
lines: per entry point, calls, time, time in native call-outs).
`tools/run-game.sh` passes `FFX_MAGIC_EMU` on.

With it, `battle-magic`, `battle-overdrive` and `battle-ifrit` match the
originals (traces and screenshots; 2026-10-29). Measured cost (lockstep
replays, the busy shared machine): time in Unicorn itself is 0.08 to
0.9 ms per frame for the per-frame entry point (`magic_0481` 0.32 ms,
`magic_0390` 0.87 ms, `magic_0604` 0.08 ms). The native call-outs those
entries make take far longer (`magic_0481`: about 1,000 per frame, 10 ms;
`magic_0604`: 840 per frame, 9.7 ms; about 10 µs each), but that is the
called game code (ported functions, `op_shape_draw`) plus the boundary;
how much of it a native run spends too is not measured yet.

Building Unicorn for the hook (`i686-pc-windows-msvc` under `cargo xwin`)
needs CMake, Ninja and `llvm-lib` (`tools/shims/cmake`, `ninja`,
`llvm-lib`), and `tools/xwin-unicorn.cmake` (set in `.cargo/config.toml`):
cargo-xwin's toolchain file sets `CMAKE_MSVC_RUNTIME_LIBRARY` while the
`cmake` crate passes `-MT`, which Unicorn's `CMakeLists.txt` refuses. The
first build of a target directory compiles Unicorn (about 3 minutes).

## Porting the overlays

Decision (user, 2026-10-29): the 581 overlays are ported to Rust before
the release. Unicorn (GPL-2.0) stays a development and test tool: it runs
the player's DLLs as the reference for the parity tests and as the
fallback for code without a port. `crates/ffx-magic` holds the ports.

**What a Rust overlay is.** The game still loads `magic_NNNN.dll` and
gets the same module handle, exports and data: the player's file is read
as data, its headers and data sections laid out at the image base the
original gets (`ffx_magic::Image::new`, `write_data`), relocated. Only the
code is replaced: every address the game calls into the overlay's code
(`DllMain`, `InitMagicPRX`, `GetEffectOverlayTable`, the overlay table's
entries, callbacks the overlay stored, the particle programs' step
functions, which are slot thunks) is answered by a port bound to that
address. Pointers the game stores into overlay data stay the same, and
the ports reach the game only through `g_MagicImports`, read through the
DLL's own global as the original reads it.

**Binding by code identity.** A port declares the function it replaces as
an `ffx_magic::Sig`: the length (trailing `int3` left out) and the FNV-1a
hash of its bytes with every relocated word and every `rel32` field
zeroed (what the link moves); the `rel32` callees (another `Sig`, an
import thunk such as `memset`, or `Any` for the CRT's internal calls) and
`call [IAT]` imports are checked too. `ffx_magic::bind(img, addr)` finds
the port whose signature matches the code at `addr`, so one port binds
in every DLL that carries the same function: the runtime in all 581, the
OPU host in 130. The addresses that differ between DLLs (their globals,
tables and strings) are read from the original instructions' operands:
`img.word(f + off)`, where `off` is the operand's offset in the function
(the comments name the instruction). Slot thunks differ only in their
slot and bind by shape (`runtime::slot_of_thunk`).

**The interface.** Ports are generic over `ffx_magic::Game`
(`ffx_bridge::Mem` plus `call(target, ecx, args) -> Out` with `eax`,
`edx` and ST0, and the MSVCR110 imports whose results must be the game's:
`rand`, `_libm_sse2_*_precise`, `_CIatan2`). An entry is
`fn(&mut G, &Image, &Frame) -> Ret`: the frame has the called address,
`esp` at the return address (stack arguments through `Frame::arg`) and
`eax`/`ecx`/`edx` (register arguments; `eax` for code that returns it
untouched, such as the lone `ret`). `Ret` gives `eax`, `edx`, ST0 and the
bytes popped, or a tail jump (`Ret::jump(target)`: the original's
`jmp eax` with the caller's stack as it was at the entry; the host
continues there). Internal functions (custom register conventions from
LTCG, such as the OPU set-up's `ecx`) are plain Rust functions the ports
call; they get no entry. Each call through a slot passes exactly the
words on the original's stack at that call, earlier pushes the original
leaves there included (the OPU host's per-frame calls see the camera and
screen matrices after their own arguments).

Hosts: the 32-bit hook (`overlay_emu.rs`, below), the parity tests
(`crates/ffx-oracle/tests/magic/mod.rs`) and `ffx-host`, which traps the
bound ports at their overlay addresses when it loads a DLL and runs the
rest in Unicorn (`docs/host.md`, "Overlays"; on by default there,
`FFX_MAGIC_RUST=0` turns it off).

**Floats.** The overlays are SSE code: f32 operations rounded at each
step, in the original's order, no FMA. x87 appears only to return a float
in ST0 (an `Out::st0`/`Ret::st0` `f64`, exact at the game's 53-bit
precision). The libm imports are the game's MSVCR110 through `Game`, never
Rust's `f64::sin`.

**The CRT.** `DllMain` (`_DllMainCRTStartup`, the same 35 bytes in all
581) is ported as TRUE with no state: the cookie, the `onexit` list and
the attach count are the CRT's own bookkeeping. The parity test runs the
original's `DllMain` attach and detach, takes every word it writes, and
fails if any is an operand of the DLL's ported functions; those words are
the only ones left out of the comparison. The initialiser tables of the
DLLs seen so far hold only the CRT's own functions; a DLL with C++ static
constructors would need them ported.

**Parity harness** (`crates/ffx-oracle/tests/magic/mod.rs`, one test file
per DLL: `magic_NNNN.rs`). `Twin::load(n)` loads the DLL in Unicorn
(`ffx-overlay`, a 4 GiB arena, the preferred base, `DllMain` run) and
gives the port a sparse copy of the same memory. The stand-in
`g_MagicImports` has 1024 words: each word the player's `FFX.exe` has
pointing into code becomes a trap (`TRAPS + 16 * slot`), each data word a
zeroed page of its own (`Twin::data(slot)`). `Twin::init()` runs
`InitMagicPRX` on both sides and checks it. `check_entry(addr, regs,
args, Eax::Checked)` runs the original and the bound port from the same
registers and stack, and fails on any difference in:
- `eax` (unless the test marks the original's as junk), ST0 (bit for
  bit), the bytes popped, a tail jump's target;
- the call-outs in order: target, slot, and the port's argument words
  against the original's stack;
- memory: every page either side's set-up or the port wrote, the image,
  the stack above the entry, the data pages, the heap and the scratch
  area.
`Twin::check(addr, …, |g, img, fr| port(…))` does the same for an
internal function with its own convention. `ecx` at a call-out is
compared where the slot's function reads it before writing it (a
method's `this`): the slots in `magic/this_slots.rs`, generated from the
player's `FFX.exe` by `tools/check-thisarg.py --magic-slots --write`
(`--magic-slots` alone fails when the file is out of date); elsewhere it
is whatever the overlay code left there. Float arguments in SSE registers
(LTCG internal functions) go in `Call::with_xmm(&[Xmm::f32(..), ..])`:
`xmm0`..`xmm3` of the original (`ffx_overlay::Regs`) and `Frame::xmm` of
the port (`Frame::xmm_f32(i)`/`xmm_f64(i)`); a port returning a float in
`xmm0` gives `Ret::xmm0(Xmm::f32(x), eax)`, and the low lane is compared
bit for bit with the original's (`Twin::last` keeps the original's
registers). A test may load several DLLs one after another on its thread
(`Twin::load` starts a fresh Unicorn instance each time,
`ffx_overlay::release_thread`; `magic_harness.rs`). Stubs (`Twin::stub(slot, |m,
args| eax)`) answer a slot for both sides alike and may read and write
memory; make them read every argument word the original passes and write
what they see (a stub that ignores an argument cannot catch a port that
drops it), and fill the tables a function clears with non-zero bytes
first. `Twin::stub_st0(slot, |m, args| f64)` makes a slot also return
a float in ST0 (the game's float functions), on both sides;
`Twin::stub_edx(slot, ..)` gives a 64-bit result's `edx` (without one
the port sees 0 and the original whatever `edx` held). Both sides draw
`rand` from the same state, and the end states are compared. Unicorn
(QEMU) picks between two NaN operands of an SSE instruction by the x87
rule (the larger significand), not the CPU's (the destination's), so
tests avoid inputs where two different NaNs meet; ports of SSE code use
`ffx_magic::sb_sse` (`add`, `sub`, `mul`, `div`: the CPU's NaN results,
whatever the compiler or target). A call-out
logs 16 words of the original's stack; the port's words are compared as
a prefix of them, so a port that drops trailing words the callee never
reads (an earlier call's words the original leaves on the stack) still
passes. Mutants go through
`tools/mutants.py` as for every port (`MUT_TEST=magic_NNNN MUT_SNAPS=`,
no snapshots).

**Locals.** A function that hands the address of a local to the game
(`lea eax, [ebp - 0x44]; push eax`) is ported with the local where the
original's frame has it, below the entry `esp` (`m0052::scaled_camera`,
`m0003::tick`), so the game sees the same pointers and the parity test
the same words. The space is the port's: the 32-bit hook runs ports 64
KiB below the caller's stack (`overlay_emu.rs`, `GAP`); other hosts
must leave the same room. The memory comparison skips the stack below
the entry, so a stub that receives such a pointer reads what is there
(`magic_0052.rs`).

The comparison covers every page the test set up or the port touched,
and also every page the original's run made resident (found with
`mincore`), so a port that leaves out a store the original makes fails
even when the test never touched that page. The 4 GiB arena is taken
zeroed from the allocator instead of cleared, so only touched pages are
resident. Words one DLL's test writes outside its regions are cleared
when the next DLL loads, so tests in one process don't see each other's
leftovers (2026-11-01, magic-testgaps).

**The gate in the game.** `FFX_MAGIC_RUST=1` (every DLL) or a comma list
of numbers (`FFX_MAGIC_RUST=604,309`) makes `overlay_emu.rs` answer each
trapped call into a selected DLL's code with the port bound there; code
without a port still runs in Unicorn, on the same memory. It needs the
`dev-unicorn` build (the loader is `FFX_MAGIC_EMU`'s) and
`tools/run-game.sh` passes it on. At unload, `ffx-hook.log` lists each
entry as `+0xNNNN rust: N calls` or `+0xNNNN unicorn: …`; a DLL is fully
ported when its replays show no `unicorn` line. The replays:
`FFX_MAGIC_RUST=1 tools/replays/run-all.sh battle-magic battle-overdrive`
and `tools/replays/run-all.sh --extra battle-ifrit` (battle-magic reaches
`magic_0481`; battle-overdrive `magic_0390`, `0595`, `0604`;
battle-ifrit `magic_0309`, `0596`).

**One port per identity.** Two ports with the same code identity
(length, masked hash, callees and imports: `ffx_magic::sigs_overlap`)
would both match one address and `bind` would silently take the first, so
`PORTS` and `INTERNAL` refuse a second registration when the crate builds
(`first_overlap`; `cargo test -p ffx-magic one_port_per_identity` names
the pair). Before porting a function, look it up (`magic-sig --covered`)
and reuse the existing port; a test of another DLL's copy binds to it
through `check_entry`.

**Claiming and naming.** The first work list, `~/ffx-work/m5/magic/worklist.tsv`,
cut the DLLs into 15 lists (contiguous ranges of about equal bytes); that
membership is fixed. The current one is `~/ffx-work/m5/magic/worklist5.tsv`
(`tools/magic-worklist.py`, from `tools/overlay-census.py`'s
`overlay-dll-functions.tsv` in `~/ffx-work/m5/magic/census2`; a rerun
never rewrites the lists file and
refuses to overwrite its output without `--replace`): one row per
function not covered yet, by code identity (the identity `bind` uses;
the census hash also keeps read-only constants and splits what one port
covers), "shared first": every function used by 2 or more DLLs, callees
before callers, then each DLL's own code (the replays' DLLs first). Each
row gives the `Sig` length and hash, the DLLs using it, the owner (the
lowest-numbered DLL, and its list) and the uncovered callees.
`worklist5-dlls.tsv` gives per DLL what is covered and left and the
shared functions it waits on. The CRT is never ported: its mnemonic
groups (`magic_0481`'s CRT tail, and `magic_0267`'s stack probe at
`0x10005c80`) and every function using its failure imports
(`__report_gsfailure`, `__raise_securityfailure`) count as covered. Claim DLLs with
`tools/claim.py magic_NNNN --agent <id>` (the DLL name is the claim).
Modules: `ffx-magic/src/mNNNN.rs` for a DLL's own functions, named for
what the code does once known (`opu.rs`, the OPU host); a function shared
by several DLLs goes into the module of the family (or `runtime.rs`),
its doc comment naming the DLL and address it was ported from, its size
and how many DLLs carry it. A new port: its `Sig` (`cargo run -p
ffx-magic --example magic-sig -- <dll> <addr> <len> [rel32 offsets]`
prints the hash; `--bind <dll> <addr>…` shows what binds), a line in the
`ports!` list (entries) or `INTERNAL` (internal functions), the parity
test, the mutants, and the replay that reaches the DLL with
`FFX_MAGIC_RUST`.

**The census's functions (2026-11-01).** `tools/overlay-census.py` cuts
each DLL's `.text` by recursive descent over objdump's instructions
(disassembled again from any address the linear sweep did not decode as
an instruction start: it desyncs on the jump tables MSVC puts after a
function). A function starts only at the entry point, an export, a
pointer into `.text` from a data section, a direct call target, a code
pointer taken in reachable code (`push offset f`, `mov [x], offset f`)
or a tail-call target (a jump out of a function's range); the starts
are found to a fixed point. Each function covers every instruction
reachable from its start by fall-through, jumps, conditional jumps and
jump tables (not calls) up to the next start, plus its jump tables and
byte index tables after its code, and ends at the last of them. The
CRT's `__try` filters and handlers (the `__SEH_prolog4` scope tables'
words) are labels of their function, not starts. The first census
(`~/ffx-work/m5/magic/census`) also started a function at every 16-byte
aligned address after a `0xcc` byte and kept only starts the linear
sweep decoded: an instruction ending in `0xcc` (`lea eax, [ebp-0x34]`,
`subss xmm1, xmm4`) cut one function into rows ending mid-instruction
(`magic_0162` `0x100021d0`, 2074 bytes, was 3 rows), call targets after a
jump table were lost and merged into their neighbour, and functions
reached only through a code pointer (`magic_0176` `0x10001710`) were
merged into the one before. `census2` has 95,313 rows (95,586 before):
against the first census 1,830 clusters merged, 3,300 split, 451
resized, 2 recut (`overlay-row-changes.tsv`; `--old DIR` prints them per
DLL), no code left unreached (`overlay-unreached.tsv` lists any). Ports
bind by their own `Sig`, not by the rows, so a port of a cut piece still
binds where the bytes match; `magic-sig --covered-len <dll> <addr>…`
prints the bound `Sig`'s length to compare with a row's size. A merged
row whose start a port covered counted as covered as a whole: 44
functions after such a start had no port (and 150 `__security_check_cookie`
copies merged behind a slot thunk, CRT). `worklist5.tsv` (from `census2`):
4,316 identities, 52 CRT, 2,428 covered, 1,836 left (230 shared, 1,606
own); the first census gave 4,578, 76, 2,649 and 1,853 with the same ports.

**Status (2026-10-29, merge of the pilot and lists 1-6).** 76 entries
and 66 internal functions (plus the slot thunks by shape): the runtime,
the OPU host and its start-up variants, the node lists, the object host
(224 overlays), the map effects' task functions (`rcef.rs`) and the own
code of the families listed in `ffx-magic/src/lib.rs`. Nine functions
had been ported more than once, by lists that saw different census
hashes: `xor eax, eax; ret` (kept: `runtime::zero_ret`), the OPU start-up
with the byte flag (`l3_opu::start_up`), the free-list take (three
copies; `m0032::pop_node`) and six `m0187`/`m0196` wrappers that are the
map effects' functions (`rcef`); one port each is kept, and every test
of a removed copy now checks the kept one.
Of 4,578 distinct functions, 430 are covered and 75 are the CRT; 4,073
are left (1,205 used by 2+ DLLs come first in `worklist2.tsv`), and 121
DLLs are fully covered.

**Status (2026-10-29, pilot).** 25 ports (21 entries, 3 internal
functions, plus the slot thunks by shape): the runtime (`DllMain`,
`InitMagicPRX`, `GetEffectOverlayTable` in all 581 DLLs, every slot
thunk, the lone `ret` of 267); the OPU host of 128-130 DLLs (start,
per-frame run, stop, set-up, the printf wrapper, and the nine particle
part callbacks with the `pppStartPart` wrapper); `magic_0604`'s two own
entries. Fully covered: `magic_0604`, `magic_0306`, `magic_0593`,
`magic_0615`. With `FFX_MAGIC_RUST=1` battle-magic, battle-overdrive and
battle-ifrit match the originals (traces and screenshots), and
`magic_0604` runs in battle-overdrive with no Unicorn entry left;
`magic_0309` and `magic_0595` fall back for 2-3 one-shot entries each,
`magic_0481` for its four effect functions (the 3,982-byte per-frame
one), `magic_0390` and `magic_0596` for their own code.

## The slots

| slot | offset | address | name | module | overlays | refs |
|---:|---:|---|---|---|---:|---:|
| 0 | `0x0` | `0x00824f90` | Ch_Allocate | chara | 0 | 0 |
| 1 | `0x4` | `0x00832760` | Ch_CalcElement | chara | 2 | 4 |
| 2 | `0x8` | `0x00824650` | Ch_ClothReset | chara | 0 | 0 |
| 3 | `0xc` | `0x00824670` | Ch_ClothReset2 | chara | 164 | 165 |
| 4 | `0x10` | `0x00829ef0` | Ch_DataRead | chara | 27 | 40 |
| 5 | `0x14` | `0x0082a040` | Ch_DataReadSync | chara | 9 | 12 |
| 6 | `0x18` | `0x0082eb60` | Ch_DrawEnEffect | chara | 88 | 98 |
| 7 | `0x1c` | `0x0082a9a0` | Ch_EffectGetEffectLevel | chara | 3 | 3 |
| 8 | `0x20` | `0x0082a9d0` | Ch_EffectSetEffectLevel | chara | 10 | 18 |
| 9 | `0x24` | `0x00833e30` | Ch_ElemSetReplaceMatrix | chara | 2 | 2 |
| 10 | `0x28` | `0x0082aa30` | Ch_EnClip | chara | 25 | 48 |
| 11 | `0x2c` | `0x0082abc0` | Ch_GetGravMode | chara | 16 | 20 |
| 12 | `0x30` | `0x0082ac40` | Ch_GetJimen | chara | 222 | 356 |
| 13 | `0x34` | `0x00833fc0` | Ch_GetRootLightMatrix | chara | 6 | 6 |
| 14 | `0x38` | `0x0082ad00` | Ch_GetScale | chara | 115 | 115 |
| 15 | `0x3c` | `0x0082ad30` | Ch_GetScaleX | chara | 73 | 154 |
| 16 | `0x40` | `0x0082ad40` | Ch_GetScaleY | chara | 88 | 190 |
| 17 | `0x44` | `0x0082ad50` | Ch_GetScaleZ | chara | 83 | 170 |
| 18 | `0x48` | `0x00836af0` | Ch_GetSeqFrame | chara | 164 | 328 |
| 19 | `0x4c` | `0x0082ad90` | Ch_GetThicknessBtl | chara | 22 | 38 |
| 20 | `0x50` | `0x008392c0` | Ch_GetWaitMotion | chara | 15 | 34 |
| 21 | `0x54` | `0x0082ae30` | Ch_ModelSetAlpha | chara | 3 | 8 |
| 22 | `0x58` | `0x00839860` | Ch_MotionGetFrame | chara | 7 | 7 |
| 23 | `0x5c` | `0x0082b010` | Ch_MotionSetNextHokan | chara | 158 | 159 |
| 24 | `0x60` | `0x00824f00` | ChN_DataDispose | chara | 19 | 30 |
| 25 | `0x64` | `0x00836830` | ChN_DisposeSystemMGRP | chara | 25 | 36 |
| 26 | `0x68` | `0x00836870` | ChN_ReadSystemMGRP | chara | 28 | 47 |
| 27 | `0x6c` | `0x00836a40` |  | chara? | 27 | 40 |
| 28 | `0x70` | `0x00836a50` | ChN_ReadSystemMGRPsync | chara | 9 | 12 |
| 29 | `0x74` | `0x00835600` | Ch_OptposGetBottom | chara | 116 | 117 |
| 30 | `0x78` | `0x0082b080` | Ch_ResetMotion | chara | 183 | 352 |
| 31 | `0x7c` | `0x0082b400` | Ch_SetOrgMatrixScale | chara | 3 | 15 |
| 32 | `0x80` | `0x0082b440` | Ch_SetPos | chara | 0 | 0 |
| 33 | `0x84` | `0x0082b480` | Ch_SetPos3 | chara | 17 | 21 |
| 34 | `0x88` | `0x0082b520` | Ch_SetRot | chara | 121 | 121 |
| 35 | `0x8c` | `0x0082b560` | Ch_SetScale | chara | 117 | 118 |
| 36 | `0x90` | `0x00837ac0` | Ch_SetSequence | chara | 274 | 299 |
| 37 | `0x94` | `0x00837d00` | Ch_SetSysMotion | chara | 167 | 189 |
| 38 | `0x98` | `0x0082b900` | Ch_SetTransparent | chara | 103 | 158 |
| 39 | `0x9c` | `0x00740eb0` | pppDrawShapeCon | vfx | 448 | 448 |
| 40 | `0xa0` | `0x0075b740` | pppAccele | vfx | 529 | 529 |
| 41 | `0xa4` | `0x0075b810` | pppAcceleCon | vfx | 529 | 529 |
| 42 | `0xa8` | `0x0075b850` | pppAngAccele | vfx | 497 | 497 |
| 43 | `0xac` | `0x0075b8c0` | pppAngAcceleCon | vfx | 497 | 497 |
| 44 | `0xb0` | `0x0075b900` | pppSclAccele | vfx | 565 | 565 |
| 45 | `0xb4` | `0x0075b9d0` | pppSclAcceleCon | vfx | 565 | 565 |
| 46 | `0xb8` | `0x0075ba40` | pppColAccele | vfx | 575 | 575 |
| 47 | `0xbc` | `0x0075bac0` | pppColAcceleCon | vfx | 575 | 575 |
| 48 | `0xc0` | `0x0075bde0` | pppMove | vfx | 554 | 554 |
| 49 | `0xc4` | `0x0075beb0` | pppMoveCon | vfx | 554 | 554 |
| 50 | `0xc8` | `0x0075bef0` | pppAngMove | vfx | 543 | 543 |
| 51 | `0xcc` | `0x0075bf60` | pppAngMoveCon | vfx | 543 | 543 |
| 52 | `0xd0` | `0x0075bfa0` | pppSclMove | vfx | 565 | 565 |
| 53 | `0xd4` | `0x0075c070` | pppSclMoveCon | vfx | 565 | 565 |
| 54 | `0xd8` | `0x0075c390` | pppColMove | vfx | 575 | 575 |
| 55 | `0xdc` | `0x0075c410` | pppColMoveCon | vfx | 575 | 575 |
| 56 | `0xe0` | `0x0075cfe0` | pppScale | vfx | 575 | 575 |
| 57 | `0xe4` | `0x0075d050` | pppScaleCon | vfx | 575 | 575 |
| 58 | `0xe8` | `0x0075d1f0` | pppColor | vfx | 575 | 575 |
| 59 | `0xec` | `0x0075d240` | pppColorCon | vfx | 575 | 575 |
| 60 | `0xf0` | `0x0075e430` | pppKeDrct | vfx | 114 | 114 |
| 61 | `0xf4` | `0x0075e470` | pppKeDrctCon | vfx | 114 | 114 |
| 62 | `0xf8` | `0x00730ab0` | pppRandFV | vfx | 518 | 518 |
| 63 | `0xfc` | `0x00730ce0` | pppRandUpFV | vfx | 239 | 239 |
| 64 | `0x100` | `0x00730ef0` | pppRandDownFV | vfx | 158 | 158 |
| 65 | `0x104` | `0x00731100` | pppRandIV | vfx | 545 | 545 |
| 66 | `0x108` | `0x007325f0` | pppSRandFV | vfx | 391 | 391 |
| 67 | `0x10c` | `0x0072d390` | pppMatrixXYZ | vfx | 575 | 575 |
| 68 | `0x110` | `0x0072d750` | pppMatrixYXZ | vfx | 60 | 60 |
| 69 | `0x114` | `0x00734130` | pppMatrixLoc | vfx | 113 | 113 |
| 70 | `0x118` | `0x00734170` | pppMatrixScl | vfx | 485 | 485 |
| 71 | `0x11c` | `0x00734880` | pppKeParMatR | vfx | 69 | 69 |
| 72 | `0x120` | `0x00734b70` | pppDrawMatrix | vfx | 536 | 536 |
| 73 | `0x124` | `0x00734bd0` | pppDrawMatrixFront | vfx | 568 | 568 |
| 74 | `0x128` | `0x00734ef0` | pppKeDMatFrDraw | vfx | 195 | 195 |
| 75 | `0x12c` | `0x00736d80` | pppKeThTp | vfx | 113 | 113 |
| 76 | `0x130` | `0x00736e60` | pppKeThTpCon | vfx | 113 | 113 |
| 77 | `0x134` | `0x00736e90` | pppKeThSft | vfx | 98 | 98 |
| 78 | `0x138` | `0x00737130` | pppKeThSftCon | vfx | 98 | 98 |
| 79 | `0x13c` | `0x00736050` | (pppKeTh step) | vfx? | 113 | 113 |
| 80 | `0x140` | `0x00736580` | pppKeThDraw | vfx | 113 | 113 |
| 81 | `0x144` | `0x00736460` | pppKeThCon | vfx | 113 | 113 |
| 82 | `0x148` | `0x007364e0` | pppKeThCon2 | vfx | 113 | 113 |
| 83 | `0x14c` | `0x00736570` | (pppKeTh Des) | vfx? | 113 | 113 |
| 84 | `0x150` | `0x00737770` | (pppDrawMdl draw) | vfx? | 311 | 311 |
| 85 | `0x154` | `0x00737f40` | (pppDrawMdlTs draw) | vfx? | 192 | 192 |
| 86 | `0x158` | `0x007384f0` | pppDrawMdlTsCon | vfx | 192 | 192 |
| 87 | `0x15c` | `0x007409e0` | pppDrawShape | vfx | 448 | 448 |
| 88 | `0x160` | `0x0072e790` | (pppKeMdlDtt draw) | vfx? | 181 | 181 |
| 89 | `0x164` | `0x0072e730` | pppKeMdlDttCon | vfx | 181 | 181 |
| 90 | `0x168` | `0x0072e760` | pppKeMdlDttCon2 | vfx | 181 | 181 |
| 91 | `0x16c` | `0x00757840` | pppVertexAp | vfx | 442 | 442 |
| 92 | `0x170` | `0x00757b30` | pppVertexApCon | vfx | 442 | 442 |
| 93 | `0x174` | `0x00758960` | pppKeBornRnd3 | vfx | 60 | 60 |
| 94 | `0x178` | `0x007589d0` | pppKeBornRnd3Con | vfx | 60 | 60 |
| 95 | `0x17c` | `0x00758f20` | (pppKeBornRnd5 step) | vfx | 300 | 300 |
| 96 | `0x180` | `0x00758f90` | pppKeBornRnd5Con | vfx | 300 | 300 |
| 97 | `0x184` | `0x00759200` | (pppKeBornRnd6 step) | vfx | 113 | 113 |
| 98 | `0x188` | `0x00759270` | pppKeBornRnd6Con | vfx | 113 | 113 |
| 99 | `0x18c` | `0x0075c450` | pppPoint | vfx | 575 | 575 |
| 100 | `0x190` | `0x0075c4c0` | pppPointCon | vfx | 575 | 575 |
| 101 | `0x194` | `0x0075ce30` | pppAngle | vfx | 575 | 575 |
| 102 | `0x198` | `0x0075ce80` | pppAngleCon | vfx | 575 | 575 |
| 103 | `0x19c` | `0x00736960` | pppKeThRes32Con | vfx | 39 | 39 |
| 104 | `0x1a0` | `0x007597b0` | pppKeGrvTgt | vfx | 112 | 112 |
| 105 | `0x1a4` | `0x00759800` | pppKeGrvTgtCon | vfx | 112 | 112 |
| 106 | `0x1a8` | `0x00759650` | pppKeGrvEff | vfx | 109 | 109 |
| 107 | `0x1ac` | `0x007597a0` | (pppKeGrvEff Con) | vfx? | 109 | 109 |
| 108 | `0x1b0` | `0x00732130` | pppRandUpHCV | vfx | 76 | 76 |
| 109 | `0x1b4` | `0x00732b40` | pppSRandDownFV | vfx | 51 | 51 |
| 110 | `0x1b8` | `0x00735160` | pppKeZCrctDraw | vfx | 370 | 370 |
| 111 | `0x1bc` | `0x007354c0` | pppKeZCrctShpDraw | vfx | 356 | 356 |
| 112 | `0x1c0` | `0x00738530` | (pppDrawMdl2 draw) | vfx? | 136 | 136 |
| 113 | `0x1c4` | `0x007394c0` | (pppDrawMdl3 draw) | vfx? | 319 | 319 |
| 114 | `0x1c8` | `0x00740ed0` | pppDrawShapeX | vfx | 353 | 353 |
| 115 | `0x1cc` | `0x00741450` | pppDrawShapeXCon | vfx | 353 | 353 |
| 116 | `0x1d0` | `0x0074c490` | pppKeShpTail | vfx | 47 | 47 |
| 117 | `0x1d4` | `0x0074c570` | (pppKeShpTail draw) | vfx? | 47 | 47 |
| 118 | `0x1d8` | `0x0074c540` | pppKeShpTailCon | vfx | 47 | 47 |
| 119 | `0x1dc` | `0x00757490` | pppPointRAp | vfx | 37 | 37 |
| 120 | `0x1e0` | `0x00757750` | pppPointRApCon | vfx | 37 | 37 |
| 121 | `0x1e4` | `0x0072eb20` | pppVtMime | vfx | 41 | 41 |
| 122 | `0x1e8` | `0x0072ed70` | pppVtMimeCon | vfx | 41 | 41 |
| 123 | `0x1ec` | `0x0072ede0` | pppVtMimeDes | vfx | 41 | 41 |
| 124 | `0x1f0` | `0x007587b0` | pppKeBornRnd2 | vfx | 81 | 81 |
| 125 | `0x1f4` | `0x00758820` | pppKeBornRnd2Con | vfx | 81 | 81 |
| 126 | `0x1f8` | `0x00755000` | (pppKeShpDtt step) | vfx? | 188 | 188 |
| 127 | `0x1fc` | `0x00755010` | pppKeShpDttCon | vfx | 188 | 188 |
| 128 | `0x200` | `0x00755040` | (pppKeShpDtt draw) | vfx? | 188 | 188 |
| 129 | `0x204` | `0x00734640` | pppParMatrix | vfx | 395 | 395 |
| 130 | `0x208` | `0x00757ff0` | pppVertexApLc | vfx | 385 | 385 |
| 131 | `0x20c` | `0x00758250` | pppVertexApLcCon | vfx | 385 | 385 |
| 132 | `0x210` | `0x0075e310` | pppKeMvYpEff | vfx | 63 | 63 |
| 133 | `0x214` | `0x00734eb0` | pppKeDMatDraw | vfx | 186 | 186 |
| 134 | `0x218` | `0x00738ec0` | (pppDrawMdlTs2 draw) | vfx? | 116 | 116 |
| 135 | `0x21c` | `0x00739480` | pppDrawMdlTs2Con | vfx | 116 | 116 |
| 136 | `0x220` | `0x0073a370` | (pppDrawMdlTs3 draw) | vfx? | 289 | 289 |
| 137 | `0x224` | `0x0073aac0` | pppDrawMdlTs3Con | vfx | 289 | 289 |
| 138 | `0x228` | `0x007573c0` | pppPointAp | vfx | 152 | 152 |
| 139 | `0x22c` | `0x00757470` | pppPointApCon | vfx | 152 | 152 |
| 140 | `0x230` | `0x00759fd0` | pppKeLnsArndCon | vfx | 86 | 86 |
| 141 | `0x234` | `0x00759ff0` | (pppKeLnsArnd draw) | vfx? | 86 | 86 |
| 142 | `0x238` | `0x0075ab30` | pppKeLnsFlsCon | vfx | 72 | 72 |
| 143 | `0x23c` | `0x0075ab50` | (pppKeLnsFls draw) | vfx? | 72 | 72 |
| 144 | `0x240` | `0x0075af30` | pppKeLnsLpCon | vfx | 113 | 113 |
| 145 | `0x244` | `0x0075af60` | pppKeLnsLpCon2 | vfx | 113 | 113 |
| 146 | `0x248` | `0x0075af80` | pppKeLnsLpDraw | vfx | 113 | 113 |
| 147 | `0x24c` | `0x00759f30` | pppKeLnsLpSft | vfx | 110 | 110 |
| 148 | `0x250` | `0x00759fb0` | pppKeLnsLpSftCon | vfx | 110 | 110 |
| 149 | `0x254` | `0x0074e490` | pppKeShpTail2 | vfx | 198 | 198 |
| 150 | `0x258` | `0x0074e540` | pppKeShpTail2Con | vfx | 198 | 198 |
| 151 | `0x25c` | `0x0074e570` | (pppKeShpTail2 draw) | vfx? | 198 | 198 |
| 152 | `0x260` | `0x007502c0` | pppKeShpTail3 | vfx | 79 | 79 |
| 153 | `0x264` | `0x00750510` | pppKeShpTail3Con | vfx | 79 | 79 |
| 154 | `0x268` | `0x007505a0` | pppKeShpTail3Con2 | vfx | 79 | 79 |
| 155 | `0x26c` | `0x00750620` | (pppKeShpTail3 draw) | vfx? | 79 | 79 |
| 156 | `0x270` | `0x00731ec0` | pppRandHCV | vfx | 98 | 98 |
| 157 | `0x274` | `0x0075d340` | pppVertexAttend | vfx | 32 | 32 |
| 158 | `0x278` | `0x007582e0` | pppVertexApAt | vfx | 32 | 32 |
| 159 | `0x27c` | `0x007584b0` | pppVertexApAtCon | vfx | 32 | 32 |
| 160 | `0x280` | `0x00716710` | pppCallUsrFunc | vfx | 339 | 341 |
| 161 | `0x284` | `0x00734cf0` | pppDrawMatrixWood | vfx | 138 | 138 |
| 162 | `0x288` | `0x00734d90` | pppDrawMatrixWoodSta | vfx | 138 | 138 |
| 163 | `0x28c` | `0x0074f4d0` | pppKeShpTail2X | vfx | 47 | 47 |
| 164 | `0x290` | `0x0074f5b0` | (pppKeShpTail2X draw) | vfx? | 47 | 47 |
| 165 | `0x294` | `0x0074f580` | pppKeShpTail2XCon | vfx | 47 | 47 |
| 166 | `0x298` | `0x00759bf0` | pppKeHmgEff | vfx | 55 | 55 |
| 167 | `0x29c` | `0x00759d70` | (pppKeHmgEff Con) | vfx? | 55 | 55 |
| 168 | `0x2a0` | `0x00788de0` | MsGetEff2Chr | battle | 338 | 338 |
| 169 | `0x2a4` | `0x007948a0` | MsGetChrBtlDir | battle | 10 | 13 |
| 170 | `0x2a8` | `0x00795e20` | MsSetChrDirF | battle | 8 | 8 |
| 171 | `0x2ac` | `0x0075a3a0` | (pppKeLnsClm draw) | vfx? | 39 | 39 |
| 172 | `0x2b0` | `0x0075a370` | pppKeLnsClmCon | vfx | 39 | 39 |
| 173 | `0x2b4` | `0x007e3b70` | op_oef2_set | vfx | 7 | 7 |
| 174 | `0x2b8` | `0x0072f4b0` | pppRandFloat | vfx | 32 | 32 |
| 175 | `0x2bc` | `0x0072d930` | pppMatrixYZX | vfx | 38 | 38 |
| 176 | `0x2c0` | `0x00736720` | pppKeThRes16Con | vfx | 33 | 33 |
| 177 | `0x2c4` | `0x00736b40` | pppKeThRes48Con | vfx | 7 | 7 |
| 178 | `0x2c8` | `0x0075a760` | (pppKeLnsCrn draw) | vfx? | 16 | 16 |
| 179 | `0x2cc` | `0x0075a740` | pppKeLnsCrnCon | vfx | 16 | 16 |
| 180 | `0x2d0` | `0x0072db10` | pppMatrixZXY | vfx | 42 | 42 |
| 181 | `0x2d4` | `0x007328b0` | pppSRandUpFV | vfx | 35 | 35 |
| 182 | `0x2d8` | `0x0072dcf0` | pppMatrixZYX | vfx | 15 | 15 |
| 183 | `0x2dc` | `0x00749810` | pppKeMdlTfdDraw | vfx | 3 | 3 |
| 184 | `0x2e0` | `0x007497e0` | pppKeMdlTfdCon | vfx | 3 | 3 |
| 185 | `0x2e4` | `0x00749ff0` | pppKeMdlTfdUv3Draw | vfx | 10 | 10 |
| 186 | `0x2e8` | `0x00749f90` | pppKeMdlTfdUv3Con | vfx | 10 | 10 |
| 187 | `0x2ec` | `0x00736c00` | pppKeThRes64Con | vfx | 16 | 16 |
| 188 | `0x2f0` | `0x0072edb0` | pppVtMimeCon2 | vfx | 26 | 26 |
| 189 | `0x2f4` | `0x00751ca0` | pppKeShpTail3X | vfx | 32 | 32 |
| 190 | `0x2f8` | `0x00751fc0` | (pppKeShpTail3X draw) | vfx? | 32 | 32 |
| 191 | `0x2fc` | `0x00751eb0` | pppKeShpTail3XCon | vfx | 32 | 32 |
| 192 | `0x300` | `0x00751f40` | pppKeShpTail3XCon2 | vfx | 32 | 32 |
| 193 | `0x304` | `0x00732390` | pppRandDownHCV | vfx | 25 | 25 |
| 194 | `0x308` | `0x00734c50` | pppDrawMatrixNoRot | vfx | 1 | 1 |
| 195 | `0x30c` | `0x00739d30` | (pppDrawMdlSemi3 draw) | vfx? | 36 | 36 |
| 196 | `0x310` | `0x0075a190` | pppKeLnsArndTDraw | vfx | 1 | 1 |
| 197 | `0x314` | `0x0075a0f0` | (pppKeLnsArndT Con) | vfx | 1 | 1 |
| 198 | `0x318` | `0x0075a140` | pppKeLnsArndTDes | vfx | 1 | 1 |
| 199 | `0x31c` | `0x0075a540` | pppKeLnsClmTDraw | vfx | 1 | 1 |
| 200 | `0x320` | `0x0075a4a0` | pppKeLnsClmTCon | vfx | 1 | 1 |
| 201 | `0x324` | `0x0075a4f0` | pppKeLnsClmTDes | vfx | 1 | 1 |
| 202 | `0x328` | `0x0075a900` | pppKeLnsCrnTDraw | vfx | 1 | 1 |
| 203 | `0x32c` | `0x0075a870` | pppKeLnsCrnTCon | vfx | 1 | 1 |
| 204 | `0x330` | `0x0075a8b0` | pppKeLnsCrnTDes | vfx | 1 | 1 |
| 205 | `0x334` | `0x0075ad00` | pppKeLnsFlsTDraw | vfx | 1 | 1 |
| 206 | `0x338` | `0x0075ac60` | pppKeLnsFlsTCon | vfx | 1 | 1 |
| 207 | `0x33c` | `0x0075acb0` | pppKeLnsFlsTDes | vfx | 1 | 1 |
| 208 | `0x340` | `0x007e46b0` | op_oef2_clut_load | vfx | 11 | 11 |
| 209 | `0x344` | `0x007e5030` | op_oef2_texture_load | vfx | 11 | 11 |
| 210 | `0x348` | `0x00833f80` | Ch_GetRootGlobalMatrix | chara | 71 | 141 |
| 211 | `0x34c` | `0x0082b3e0` | Ch_SetOrgMatrix | chara | 166 | 297 |
| 212 | `0x350` | `0x0082b820` | Ch_SetShadowMode | chara | 4 | 6 |
| 213 | `0x354` | `0x00787950` | MsEffectEndMotion | battle | 288 | 380 |
| 214 | `0x358` | `0x00788920` | MsEffectTerminate | battle | 411 | 412 |
| 215 | `0x35c` | `0x00795080` | MsGetChrHandle | battle | 268 | 800 |
| 216 | `0x360` | `0x007950f0` | MsGetChrHeight | battle | 13 | 13 |
| 217 | `0x364` | `0x007956e0` | MsGetChrPos | battle | 175 | 399 |
| 218 | `0x368` | `0x007b7090` | MsGetTargetList | battle | 333 | 399 |
| 219 | `0x36c` | `0x00795b40` | MsGetWeaponHandle | battle | 149 | 696 |
| 220 | `0x370` | `0x007ffe10` | op_oef2_se_on | vfx | 358 | 481 |
| 221 | `0x374` | `0x008002c0` | op_oef2_sound_cue_multi | vfx | 369 | 407 |
| 222 | `0x378` | `0x007e4830` | op_oef2_tex_clut_cue_multi | vfx | 410 | 556 |
| 223 | `0x37c` | `0x007367b0` | pppKeThRes16x4Con | vfx | 4 | 4 |
| 224 | `0x380` | `0x007fcbd0` |  | vfx? | 1 | 1 |
| 225 | `0x384` | `0x007fce10` | op_pray_effect_check | vfx | 411 | 411 |
| 226 | `0x388` | `0x00788200` | MsEffectReady | battle | 411 | 411 |
| 227 | `0x38c` | `0x00787ca0` | MsEffectMotionLevel | battle | 357 | 357 |
| 228 | `0x390` | `0x007882c0` | MsEffectSetMotion | battle | 289 | 555 |
| 229 | `0x394` | `0x006ed2e0` | FFXVu0ApplyMatrix | ps2-emu | 377 | 1908 |
| 230 | `0x398` | `0x006ed360` | FFXVu0CopyMatrix | ps2-emu | 406 | 1939 |
| 231 | `0x39c` | `0x006ed5c0` |  | ps2-emu? | 408 | 1738 |
| 232 | `0x3a0` | `0x006ed6f0` | FFXVu0Normalize | ps2-emu | 148 | 568 |
| 233 | `0x3a4` | `0x006ed8f0` | FFXVu0ScaleVector | ps2-emu | 138 | 2237 |
| 234 | `0x3a8` | `0x006ed940` | FFXVu0SubVector | ps2-emu | 194 | 502 |
| 235 | `0x3ac` | `0x006ed9a0` | FFXVu0UnitMatrix | ps2-emu | 274 | 1279 |
| 236 | `0x3b0` | `0x00833c30` | Ch_ElemGetMatrix | chara | 159 | 171 |
| 237 | `0x3b4` | `0x00833d10` | Ch_ElemGetPos | chara | 236 | 536 |
| 238 | `0x3b8` | `0x007966a0` | MsSetChrVisibleAppear | battle | 30 | 40 |
| 239 | `0x3bc` | `0x0078d980` | MsSetDamage | battle | 275 | 320 |
| 240 | `0x3c0` | `0x00790260` | MsTestCheckMotion | battle | 5 | 7 |
| 241 | `0x3c4` | `0x009dbb40` |  | system | 3 | 12 |
| 242 | `0x3c8` | `0x007ed0c0` | op_shape_draw | vfx | 1 | 1 |
| 243 | `0x3cc` | `0x0072c730` | pppGetRotMatrixX | vfx | 4 | 6 |
| 244 | `0x3d0` | `0x0072cb10` | pppGetRotMatrixZ | vfx | 6 | 8 |
| 245 | `0x3d4` | `0x00820530` | Sg_GetDBuffDC | main | 39 | 54 |
| 246 | `0x3d8` | `0x0082ab80` | Ch_GetDir | chara | 1 | 1 |
| 247 | `0x3dc` | `0x0072c920` | pppGetRotMatrixY | vfx | 11 | 33 |
| 248 | `0x3e0` | `0x006ed290` | FFXVu0AddVector | ps2-emu | 254 | 1314 |
| 249 | `0x3e4` | `0x0082b380` | Ch_SetMotionSpeed | chara | 160 | 337 |
| 250 | `0x3e8` | `0x007bc0c0` | MsScreenOffset | battle | 241 | 492 |
| 251 | `0x3ec` | `0x00795a20` | MsGetChrWidth | battle | 36 | 40 |
| 252 | `0x3f0` | `0x007eaac0` |  | vfx? | 2 | 2 |
| 253 | `0x3f4` | `0x0082d7e0` | Sg_GetCurExecFrames | main | 180 | 246 |
| 254 | `0x3f8` | `0x0082a9f0` | Ch_EnBodyHit | chara | 1 | 4 |
| 255 | `0x3fc` | `0x00796150` | MsSetChrPos | battle | 176 | 502 |
| 256 | `0x400` | `0x0082b5d0` | Ch_SetShadeCol | chara | 91 | 115 |
| 257 | `0x404` | `0x0082b2a0` | Ch_SetHideEffect | chara | 107 | 469 |
| 258 | `0x408` | `0x00717410` | pppSetMatrix | vfx | 62 | 109 |
| 259 | `0x40c` | `0x007175f0` |  | vfx? | 4 | 4 |
| 260 | `0x410` | `0x0086aee0` | AtelGetCharHandleFromBindPC2 | script | 32 | 62 |
| 261 | `0x414` | `0x0086b030` | AtelGetEffectFlag | script | 45 | 148 |
| 262 | `0x418` | `0x0082ac90` | Ch_GetPos | chara | 335 | 995 |
| 263 | `0x41c` | `0x008303f0` | Ch_GetNbElem | chara | 5 | 5 |
| 264 | `0x420` | `0x007e6a00` | op_spr_alloc | vfx | 118 | 123 |
| 265 | `0x424` | `0x007e6a60` | op_spr_free | vfx | 118 | 123 |
| 266 | `0x428` | `0x006ed720` | FFXVu0OuterProduct | ps2-emu | 20 | 60 |
| 267 | `0x42c` | `0x00909e10` | yiRenderDisable | map | 354 | 488 |
| 268 | `0x430` | `0x008354f0` | Ch_OptposGet | chara | 287 | 743 |
| 269 | `0x434` | `0x007ea2d0` | op_spline_xyz_table | vfx | 7 | 14 |
| 270 | `0x438` | `0x007ea120` | op_spline_xyz | vfx | 7 | 21 |
| 271 | `0x43c` | `0x00909e20` | yiRenderEnable | map | 263 | 553 |
| 272 | `0x440` | `0x0074d490` | pppKeShpTailX | vfx | 2 | 2 |
| 273 | `0x444` | `0x0074d570` | (pppKeShpTailX draw) | vfx? | 2 | 2 |
| 274 | `0x448` | `0x0074d540` | pppKeShpTailXCon | vfx | 2 | 2 |
| 275 | `0x44c` | `0x00826950` | Ch_DrawMainEffect | chara | 16 | 16 |
| 276 | `0x450` | `0x00830160` | Ch_DrawSetSituation | chara | 125 | 400 |
| 277 | `0x454` | `0x0082ab60` | Ch_GetDataScale | chara | 184 | 319 |
| 278 | `0x458` | `0x0082aea0` | Ch_ModelSetHide | chara | 11 | 45 |
| 279 | `0x45c` | `0x00887c30` |  | system? | 4 | 4 |
| 280 | `0x460` | `0x007816b0` | MsBattleGetNum | battle | 2 | 12 |
| 281 | `0x464` | `0x0079ddb0` | MsBattleSpecialCameraPause | battle | 43 | 45 |
| 282 | `0x468` | `0x007bd510` | MsCameraGetScrDpt | battle | 15 | 25 |
| 283 | `0x46c` | `0x007bed60` | MsCameraResetMove | battle | 43 | 86 |
| 284 | `0x470` | `0x007bf940` | MsCameraSetRect | battle | 45 | 98 |
| 285 | `0x474` | `0x007bfa80` | MsCameraSetRoll | battle | 43 | 50 |
| 286 | `0x478` | `0x007bfbd0` | MsCameraSetScrDpt | battle | 44 | 64 |
| 287 | `0x47c` | `0x007877d0` | MsEffectCameraGet | battle | 43 | 44 |
| 288 | `0x480` | `0x007b6c50` | MsGetBattleList | battle | 109 | 364 |
| 289 | `0x484` | `0x00794fb0` | MsGetChrGround | battle | 122 | 241 |
| 290 | `0x488` | `0x00795720` | MsGetChrScale | battle | 120 | 135 |
| 291 | `0x48c` | `0x0079a680` | MsGetDamageInfo | battle | 229 | 235 |
| 292 | `0x490` | `0x00782ae0` | MsGetMapCenter | battle | 9 | 18 |
| 293 | `0x494` | `0x0079aef0` | MsGetRamChrMonster | battle | 87 | 170 |
| 294 | `0x498` | `0x00788e10` | MsGetSummonerNum | battle | 9 | 16 |
| 295 | `0x49c` | `0x007962b0` | MsSetChrShadow | battle | 88 | 106 |
| 296 | `0x4a0` | `0x00796660` | MsSetChrVisible | battle | 215 | 796 |
| 297 | `0x4a4` | `0x009db270` |  | system | 5 | 13 |
| 298 | `0x4a8` | `0x009db490` |  | system | 4 | 8 |
| 299 | `0x4ac` | `0x009dba10` |  | system? | 5 | 5 |
| 300 | `0x4b0` | `0x009dbe10` |  | system | 4 | 4 |
| 301 | `0x4b4` | `0x009db190` |  | system? | 0 | 0 |
| 302 | `0x4b8` | `0x00888d70` | TkGetControlerHist | menu | 12 | 12 |
| 303 | `0x4bc` | `0x00888e50` | TkGetControlerReptHist | menu | 10 | 10 |
| 304 | `0x4c0` | `0x00888e80` | TkGetControlerTrigHist | menu | 10 | 10 |
| 305 | `0x4c4` | `0x00887dc0` | TkGetLastSyncVCounter | menu | 10 | 10 |
| 306 | `0x4c8` | `0x00875fb0` | TkSetVibrateStream | menu | 111 | 183 |
| 307 | `0x4cc` | `0x00798930` | frnd | battle | 81 | 429 |
| 308 | `0x4d0` | `0x00a65750` | op1_md_draw | vfx | 4 | 4 |
| 309 | `0x4d4` | `0x007fba40` | op_et_bindeff_draw_off | vfx | 4 | 4 |
| 310 | `0x4d8` | `0x007fbab0` | op_et_bindeff_draw_on | vfx | 4 | 4 |
| 311 | `0x4dc` | `0x007eb300` | op_func_after_ot3d_free | vfx | 26 | 38 |
| 312 | `0x4e0` | `0x007eb320` | op_func_after_ot3d_set | vfx | 26 | 38 |
| 313 | `0x4e4` | `0x007e46d0` | op_oef2_tex_clut_cue_check | vfx | 199 | 205 |
| 314 | `0x4e8` | `0x007e6700` | op_random | vfx | 123 | 1811 |
| 315 | `0x4ec` | `0x007ec7e0` | op_zbuf_clear | vfx | 10 | 14 |
| 316 | `0x4f0` | `0x00734020` | pppKeMatSN | vfx? | 8 | 8 |
| 317 | `0x4f4` | `0x007340f0` | pppKeMatSNCon | vfx | 8 | 8 |
| 318 | `0x4f8` | `0x00749e60` | pppKeMdlTfd3Con | vfx | 11 | 11 |
| 319 | `0x4fc` | `0x00749e90` | pppKeMdlTfd3Draw | vfx | 11 | 11 |
| 320 | `0x500` | `0x007498f0` | pppKeMdlTfdUvCon | vfx | 4 | 4 |
| 321 | `0x504` | `0x00749950` | pppKeMdlTfdUvDraw | vfx | 4 | 4 |
| 322 | `0x508` | `0x00736840` | pppKeThRes24Con | vfx | 39 | 39 |
| 323 | `0x50c` | `0x0072d570` | pppMatrixXZY | vfx | 8 | 8 |
| 324 | `0x510` | `0x007989a0` | rnd | battle | 91 | 273 |
| 325 | `0x514` | `0x006ed370` | FFXVu0CopyVector | ps2-emu | 193 | 1815 |
| 326 | `0x518` | `0x0062fd70` | FFXVu0InterVectorXYZ | ps2-emu | 44 | 76 |
| 327 | `0x51c` | `0x006ed450` |  | ps2-emu? | 40 | 73 |
| 328 | `0x520` | `0x006ed780` | FFXVu0RotMatrixX | ps2-emu | 230 | 772 |
| 329 | `0x524` | `0x006ed7c0` | FFXVu0RotMatrixY | ps2-emu | 231 | 866 |
| 330 | `0x528` | `0x006ed800` | FFXVu0RotMatrixZ | ps2-emu | 240 | 584 |
| 331 | `0x52c` | `0x009086d0` | yiGetBackColor | map | 303 | 303 |
| 332 | `0x530` | `0x0090a040` | yiSetBackColor | map | 345 | 487 |
| 333 | `0x534` | `0x0082ace0` | Ch_GetRot | chara | 290 | 425 |
| 334 | `0x538` | `0x00787760` | MsEffectCameraEnd | battle | 4 | 5 |
| 335 | `0x53c` | `0x007fccc0` | op_get_chrhd_all_list | vfx | 1 | 2 |
| 336 | `0x540` | `0x007fcd10` | op_get_chrhd_by_chrno | vfx | 1 | 2 |
| 337 | `0x544` | `0x0072c480` | pppCreateHeap | vfx | 1 | 1 |
| 338 | `0x548` | `0x0074a1e0` | (pppNeiDrawMdlPointLight draw) | vfx? | 2 | 2 |
| 339 | `0x54c` | `0x0074ae70` | (pppNeiDrawMdlTsPointLight draw) | vfx? | 1 | 1 |
| 340 | `0x550` | `0x0074b500` | pppNeiDrawMdlTsPointLightCon | vfx | 1 | 1 |
| 341 | `0x554` | `0x0075d5d0` | pppNeiLightEikyo | vfx | 5 | 5 |
| 342 | `0x558` | `0x0075d6c0` | pppNeiLightEikyoCon | vfx | 5 | 5 |
| 343 | `0x55c` | `0x0075d700` | pppNeiPointLight | vfx | 5 | 5 |
| 344 | `0x560` | `0x0075d7c0` | pppNeiPointLightCon | vfx | 5 | 5 |
| 345 | `0x564` | `0x0083c710` | Ch_ClutCreate | chara | 9 | 9 |
| 346 | `0x568` | `0x0083c810` | Ch_ClutDispose | chara | 9 | 9 |
| 347 | `0x56c` | `0x00795660` | MsGetChrMotionSignal | battle | 4 | 4 |
| 348 | `0x570` | `0x00807870` | op_chara_hsv | vfx | 9 | 13 |
| 349 | `0x574` | `0x008079f0` | op_chara_hsv_make | vfx | 9 | 13 |
| 350 | `0x578` | `0x00807b20` | op_chara_hsv_size_get | vfx | 5 | 5 |
| 351 | `0x57c` | `0x00716a30` | pppMemAlloc | vfx | 4 | 4 |
| 352 | `0x580` | `0x007eaad0` | op_flash_path3 | vfx | 2 | 2 |
| 353 | `0x584` | `0x0090a270` | yiSetColorScale | map | 347 | 630 |
| 354 | `0x588` | `0x008311d0` | Ch_TextureLoadClut | chara | 26 | 29 |
| 355 | `0x58c` | `0x00831300` | Ch_TextureLoadImage | chara | 26 | 29 |
| 356 | `0x590` | `0x00831390` | Ch_TextureSetClutBase | chara | 49 | 134 |
| 357 | `0x594` | `0x008313c0` | Ch_TextureSetImageBase | chara | 49 | 134 |
| 358 | `0x598` | `0x009dbaf0` |  | system | 1 | 1 |
| 359 | `0x59c` | `0x009dbd90` |  | system | 2 | 4 |
| 360 | `0x5a0` | `0x00a68e10` | op1_mdc_ref_nolight | vfx | 0 | 0 |
| 361 | `0x5a4` | `0x00a6a3a0` | op_md_chara_path3 | vfx | 1 | 1 |
| 362 | `0x5a8` | `0x00803360` |  | vfx | 30 | 33 |
| 363 | `0x5ac` | `0x008037c0` |  | vfx | 10 | 10 |
| 364 | `0x5b0` | `0x0082aba0` | Ch_GetFixHeight | chara | 38 | 46 |
| 365 | `0x5b4` | `0x0082adb0` | Ch_GetThicknessBtlCam | chara | 1 | 1 |
| 366 | `0x5b8` | `0x00794c20` | MsGetChrCenterPos | battle | 117 | 244 |
| 367 | `0x5bc` | `0x0079aa50` | MsGetDamageStatus | battle | 215 | 215 |
| 368 | `0x5c0` | `0x007b6fb0` | MsGetTargetLinkList | battle | 2 | 3 |
| 369 | `0x5c4` | `0x0078f950` | MsMenuBitStatus | battle | 1 | 1 |
| 370 | `0x5c8` | `0x0082cd30` | Sg_GetFocus | main | 1 | 2 |
| 371 | `0x5cc` | `0x0082ce20` | Sg_SetFocus | main | 81 | 183 |
| 372 | `0x5d0` | `0x00890ec0` |  | system? | 1 | 1 |
| 373 | `0x5d4` | `0x00897770` |  | system? | 1 | 1 |
| 374 | `0x5d8` | `0x00897dc0` |  | system? | 1 | 1 |
| 375 | `0x5dc` | `0x00898bd0` | TOBtlOpenLibraWindow | system | 1 | 1 |
| 376 | `0x5e0` | `0x007eb3a0` | op_func_before_wot_free | vfx | 6 | 6 |
| 377 | `0x5e4` | `0x007eb3c0` | op_func_before_wot_set | vfx | 6 | 6 |
| 378 | `0x5e8` | `0x007e4c30` | op_oef2_tex_clut_cue_trans | vfx | 84 | 84 |
| 379 | `0x5ec` | `0x007e4e40` | op_oef2_tex_clut_ovl_clear | vfx | 3 | 3 |
| 380 | `0x5f0` | `0x007e4e80` | op_oef2_tex_clut_ovl_trans | vfx | 4 | 4 |
| 381 | `0x5f4` | `0x007ec7d0` | op_wot_triger | vfx | 6 | 12 |
| 382 | `0x5f8` | `0x0062f880` | sceGsSyncPath | system | 144 | 302 |
| 383 | `0x5fc` | `0x007ea700` | op_unit_m_scale | vfx | 11 | 11 |
| 384 | `0x600` | `0x008266f0` | Ch_Dispose | chara | 28 | 51 |
| 385 | `0x604` | `0x0082b1e0` | Ch_SetFieldMode | chara | 6 | 6 |
| 386 | `0x608` | `0x0082b240` | Ch_SetGravMode | chara | 6 | 6 |
| 387 | `0x60c` | `0x0082b3a0` | Ch_SetMotionType | chara | 6 | 6 |
| 388 | `0x610` | `0x0078ca20` | MsDamageSetMotion | battle | 20 | 26 |
| 389 | `0x614` | `0x007b7f50` | MsEffectSetCameraBufFloat | battle | 5 | 31 |
| 390 | `0x618` | `0x00795610` | MsGetChrModelID | battle | 65 | 78 |
| 391 | `0x61c` | `0x0082cc20` | Sg_FilterAdd | main | 5 | 5 |
| 392 | `0x620` | `0x00a8d87a` |  | crt? | 0 | 0 |
| 393 | `0x624` | `0x007ffd20` | op_oef2_se_3d_on | vfx | 349 | 399 |
| 394 | `0x628` | `0x0062fce0` | FFXVu0InnerProduct | ps2-emu | 6 | 16 |
| 395 | `0x62c` | `0x0093d830` | FFXSamp0TransMatrix | movie | 4 | 8 |
| 396 | `0x630` | `0x00738a40` | (pppDrawMdlSemi2 draw) | vfx? | 9 | 9 |
| 397 | `0x634` | `0x00909ff0` | yiSetColorOffset | map | 230 | 321 |
| 398 | `0x638` | `0x00830ae0` | Ch_PLightSetEnable | chara | 3 | 5 |
| 399 | `0x63c` | `0x00835750` | Ch_SetCalcNorm | chara | 16 | 31 |
| 400 | `0x640` | `0x00831950` | Sg_PLightAllocate | main | 3 | 3 |
| 401 | `0x644` | `0x008319b0` | Sg_PLightDefine | main | 3 | 6 |
| 402 | `0x648` | `0x00831a30` |  | main? | 3 | 3 |
| 403 | `0x64c` | `0x00889b80` | TkDegtoRad | menu | 10 | 13 |
| 404 | `0x650` | `0x007e6780` | op_random_p | vfx | 87 | 278 |
| 405 | `0x654` | `0x0086b050` | AtelGetEffectParam | script | 1 | 1 |
| 406 | `0x658` | `0x00908df0` | yiGetMatrixDObj | map | 1 | 1 |
| 407 | `0x65c` | `0x007b72c0` | MsGetTargetSortList | battle | 3 | 3 |
| 408 | `0x660` | `0x007ffc60` | op_oef2_se_3d_move | vfx | 341 | 341 |
| 409 | `0x664` | `0x007368a0` | pppKeThRes24x4Con | vfx | 28 | 28 |
| 410 | `0x668` | `0x008355e0` | Ch_OptposGetBase | chara | 1 | 1 |
| 411 | `0x66c` | `0x00787840` | MsEffectCheckMotion | battle | 277 | 370 |
| 412 | `0x670` | `0x007b7170` | MsGetTargetRandomList | battle | 7 | 7 |
| 413 | `0x674` | `0x0086ae90` | AtelGetCharHandleFromBindPC | script | 130 | 131 |
| 414 | `0x678` | `0x007336e0` | pppSRandHCV | vfx | 3 | 3 |
| 415 | `0x67c` | `0x007a9a20` | MsMenuWeakElement | battle | 5 | 8 |
| 416 | `0x680` | `0x00736c60` | pppKeThRes64x4Con | vfx | 13 | 13 |
| 417 | `0x684` | `0x007eb370` | op_func_before_ot_set | vfx | 1 | 1 |
| 418 | `0x688` | `0x007eb350` | op_func_before_ot_free | vfx | 1 | 1 |
| 419 | `0x68c` | `0x00736ba0` | pppKeThRes48x4Con | vfx | 7 | 7 |
| 420 | `0x690` | `0x0082cca0` | Sg_Flash | main | 1 | 1 |
| 421 | `0x694` | `0x0062f4c0` | FlushCache | system | 3 | 3 |
| 422 | `0x698` | `0x0075b680` |  | vfx? | 4 | 4 |
| 423 | `0x69c` | `0x0062f7a0` | sceGsExecLoadImage | system | 3 | 3 |
| 424 | `0x6a0` | `0x0062f7b0` | sceGsExecStoreImage | system | 2 | 4 |
| 425 | `0x6a4` | `0x0062f820` | sceGsSetDefLoadImage | system | 3 | 3 |
| 426 | `0x6a8` | `0x0062f830` | sceGsSetDefStoreImage | system | 2 | 4 |
| 427 | `0x6ac` | `0x00930430` | rcEffectTaskMotionWait | map | 28 | 28 |
| 428 | `0x6b0` | `0x00930490` | rcEffectTaskParWait | map | 38 | 38 |
| 429 | `0x6b4` | `0x009304b0` | rcEffectTaskPrayWait | map | 40 | 40 |
| 430 | `0x6b8` | `0x009304d0` |  | map? | 39 | 39 |
| 431 | `0x6bc` | `0x00930520` | rcEffectTaskTaskWait | map | 26 | 26 |
| 432 | `0x6c0` | `0x0092df90` |  | map? | 39 | 39 |
| 433 | `0x6c4` | `0x0092dfb0` | rcefEffectReady | map | 40 | 40 |
| 434 | `0x6c8` | `0x00925170` | rcefMainCreateTask | map | 40 | 216 |
| 435 | `0x6cc` | `0x009251c0` | rcefMainFini | map | 40 | 40 |
| 436 | `0x6d0` | `0x00925210` | rcefMainInit | map | 40 | 40 |
| 437 | `0x6d4` | `0x00925440` | rcefMainProcRunAfter | map | 40 | 40 |
| 438 | `0x6d8` | `0x009254f0` | rcefMainSetEnv | map | 40 | 40 |
| 439 | `0x6dc` | `0x00925510` | rcefMainSetEnv_00925510 | map | 40 | 40 |
| 440 | `0x6e0` | `0x009256d0` | rcefMainSetPar | map | 38 | 86 |
| 441 | `0x6e4` | `0x0092f970` | rcefObjAttachChr | map | 20 | 28 |
| 442 | `0x6e8` | `0x0092f9a0` | rcefObjAttachChrOpt | map | 35 | 44 |
| 443 | `0x6ec` | `0x00930200` | rcefObjSetMode | map | 32 | 55 |
| 444 | `0x6f0` | `0x0092e680` | rcefPlaySE | map | 39 | 57 |
| 445 | `0x6f4` | `0x0092e6b0` | rcefPlayStream | map | 8 | 8 |
| 446 | `0x6f8` | `0x0092e9a0` | rcefSetDamage | map | 35 | 40 |
| 447 | `0x6fc` | `0x0092eb10` |  | map? | 28 | 80 |
| 448 | `0x700` | `0x0092ebf0` | rcefStreamCue | map | 8 | 8 |
| 449 | `0x704` | `0x0092ecb0` | rcefTaskCallSeq | map | 40 | 271 |
| 450 | `0x708` | `0x0092ee20` | rcefTaskSetSeq | map | 40 | 452 |
| 451 | `0x70c` | `0x0092ee60` | rcefTaskWait | map | 37 | 137 |
| 452 | `0x710` | `0x00949cf0` |  | crt? | 0 | 0 |
| 453 | `0x714` | `0x00921250` | rcVec3fLength | map | 12 | 13 |
| 454 | `0x718` | `0x0092d860` |  | map? | 1 | 1 |
| 455 | `0x71c` | `0x0092d870` |  | map? | 1 | 1 |
| 456 | `0x720` | `0x0092e2f0` |  | map | 10 | 11 |
| 457 | `0x724` | `0x0092e340` |  | map | 12 | 14 |
| 458 | `0x728` | `0x00925590` |  | map | 14 | 21 |
| 459 | `0x72c` | `0x0092e750` |  | map | 3 | 3 |
| 460 | `0x730` | `0x0092e960` |  | map | 3 | 5 |
| 461 | `0x734` | `0x0092ee40` | rcefTaskSetSeqCont | map | 12 | 20 |
| 462 | `0x738` | `0x00930500` |  | map | 8 | 8 |
| 463 | `0x73c` | `0x0092e270` |  | map | 8 | 13 |
| 464 | `0x740` | `0x00921360` | rcVec3fNormalize | map | 5 | 5 |
| 465 | `0x744` | `0x00930550` |  | map | 1 | 1 |
| 466 | `0x748` | `0x00930600` |  | map | 1 | 1 |
| 467 | `0x74c` | `0x00930630` |  | map | 1 | 3 |
| 468 | `0x750` | `0x00930800` |  | map? | 1 | 1 |
| 469 | `0x754` | `0x0092e4f0` |  | map | 1 | 1 |
| 470 | `0x758` | `0x00736a20` | pppKeThRes32x4Con | vfx | 40 | 40 |
| 471 | `0x75c` | `0x009303c0` | rcEffectTaskFlagWait | map | 10 | 10 |
| 472 | `0x760` | `0x0092e770` |  | map? | 9 | 9 |
| 473 | `0x764` | `0x0092e9c0` |  | map | 10 | 10 |
| 474 | `0x768` | `0x0092e0a0` |  | map | 7 | 13 |
| 475 | `0x76c` | `0x00737b20` | (pppDrawMdlSemi draw) | vfx? | 5 | 5 |
| 476 | `0x770` | `0x0092e020` |  | map | 1 | 2 |
| 477 | `0x774` | `0x0092dfd0` |  | map | 4 | 10 |
| 478 | `0x778` | `0x0092d880` |  | map | 3 | 3 |
| 479 | `0x77c` | `0x0092d9a0` |  | map | 2 | 6 |
| 480 | `0x780` | `0x0092e000` |  | map | 3 | 7 |
| 481 | `0x784` | `0x0092e1c0` | rcefGetChrHandle | map | 5 | 6 |
| 482 | `0x788` | `0x0092e580` |  | map | 6 | 12 |
| 483 | `0x78c` | `0x0092e730` |  | map | 3 | 5 |
| 484 | `0x790` | `0x0092e830` |  | map | 3 | 9 |
| 485 | `0x794` | `0x0092ea40` |  | map | 3 | 5 |
| 486 | `0x798` | `0x0092eb20` |  | map | 5 | 10 |
| 487 | `0x79c` | `0x00930350` | rcEffectTaskAnimeWait | map | 4 | 4 |
| 488 | `0x7a0` | `0x0092e110` |  | map | 2 | 6 |
| 489 | `0x7a4` | `0x0092e3a0` |  | map | 2 | 6 |
| 490 | `0x7a8` | `0x00925690` |  | map | 3 | 3 |
| 491 | `0x7ac` | `0x0092eba0` |  | map | 1 | 1 |
| 492 | `0x7b0` | `0x00795840` | MsGetChrStatWeak | battle | 2 | 3 |
| 493 | `0x7b4` | `0x0079e7c0` |  | battle | 1 | 1 |
| 494 | `0x7b8` | `0x007e9a30` | op_rt_m_yxz | vfx | 1 | 2 |
| 495 | `0x7bc` | `0x00930460` |  | map | 1 | 1 |
| 496 | `0x7c0` | `0x0092d7e0` |  | map | 1 | 2 |
| 497 | `0x7c4` | `0x0092d810` |  | map | 1 | 1 |
| 498 | `0x7c8` | `0x0092d8c0` |  | map | 1 | 1 |
| 499 | `0x7cc` | `0x0092dd30` | rcefChgChrClut | map | 1 | 2 |
| 500 | `0x7d0` | `0x0092de50` |  | map | 1 | 2 |
| 501 | `0x7d4` | `0x0092deb0` |  | map | 1 | 1 |
| 502 | `0x7d8` | `0x0092df10` |  | map | 1 | 1 |
| 503 | `0x7dc` | `0x0092e170` |  | map | 1 | 2 |
| 504 | `0x7e0` | `0x0092e630` |  | map | 1 | 1 |
| 505 | `0x7e4` | `0x009301e0` | rcefObjClearMode | map | 1 | 1 |
| 506 | `0x7e8` | `0x0092e980` |  | map? | 1 | 1 |
| 507 | `0x7ec` | `0x0092e990` |  | map? | 1 | 1 |
| 508 | `0x7f0` | `0x0092eb70` |  | map | 1 | 2 |
| 509 | `0x7f4` | `0x00734400` | pppKeOfsMatXYZ | vfx | 8 | 8 |
| 510 | `0x7f8` | `0x0075cd30` | (pppKeOfsPt step) | vfx | 8 | 8 |
| 511 | `0x7fc` | `0x0075cdf0` | pppKeOfsPtCon | vfx | 8 | 8 |
| 512 | `0x800` | `0x009212c0` | rcVec3fMul | map | 1 | 1 |
| 513 | `0x804` | `0x0092e360` |  | map | 3 | 3 |
| 514 | `0x808` | `0x0092e400` | rcAngleDiff | map | 3 | 3 |
| 515 | `0x80c` | `0x009249a0` |  | map? | 3 | 3 |
| 516 | `0x810` | `0x009249c0` | rcefTargetGetHeight | map | 3 | 3 |
| 517 | `0x814` | `0x009249e0` | rcefTargetGetAngle | map | 3 | 3 |
| 518 | `0x818` | `0x00924a00` | rcefTargetGetDist | map | 3 | 3 |
| 519 | `0x81c` | `0x00924a20` |  | map | 3 | 3 |
| 520 | `0x820` | `0x00782970` | MsGetBattleScene | battle | 93 | 141 |
| 521 | `0x824` | `0x0092e7a0` |  | map? | 1 | 1 |
| 522 | `0x828` | `0x0092ec60` | rcefStreamCue_0092ec60 | map | 4 | 5 |
| 523 | `0x82c` | `0x007b7f30` |  | battle | 11 | 18 |
| 524 | `0x830` | `0x0092e780` | rcefResetMotion | map | 1 | 1 |
| 525 | `0x834` | `0x007fbcd0` | op_et_bindeff_off_signal | vfx | 86 | 88 |
| 526 | `0x838` | `0x00925130` |  | map? | 1 | 3 |
| 527 | `0x83c` | `0x00925710` |  | map | 1 | 2 |
| 528 | `0x840` | `0x0079fd20` | MsChrSEfoot | battle | 3 | 6 |
| 529 | `0x844` | `0x00787c30` |  | battle | 158 | 158 |
| 530 | `0x848` | `0x00788430` | MsEffectSetSpeed | battle | 163 | 183 |
| 531 | `0x84c` | `0x007948e0` | MsGetChrBtlPos | battle | 121 | 370 |
| 532 | `0x850` | `0x00794d70` |  | battle | 3 | 4 |
| 533 | `0x854` | `0x00794db0` |  | battle | 1 | 1 |
| 534 | `0x858` | `0x00794f70` |  | battle | 130 | 160 |
| 535 | `0x85c` | `0x00795640` |  | battle | 3 | 5 |
| 536 | `0x860` | `0x007956a0` | MsGetChrMotionSpeed | battle | 1 | 1 |
| 537 | `0x864` | `0x00795eb0` | MsSetChrDirTarget | battle | 5 | 6 |
| 538 | `0x868` | `0x00796070` |  | battle | 119 | 240 |
| 539 | `0x86c` | `0x007960d0` | MsSetChrMotionLevel | battle | 1 | 1 |
| 540 | `0x870` | `0x00796110` | MsSetChrMoveFlag | battle | 2 | 2 |
| 541 | `0x874` | `0x00796480` | MsSetChrTransparent | battle | 98 | 113 |
| 542 | `0x878` | `0x007968e0` | MsSetChrWeight | battle | 2 | 2 |
| 543 | `0x87c` | `0x0082bee0` | Sg_Blur | main | 224 | 332 |
| 544 | `0x880` | `0x008360f0` | Sg_LightGetColor | main | 30 | 114 |
| 545 | `0x884` | `0x00836240` |  | main | 18 | 18 |
| 546 | `0x888` | `0x007fb290` | op_et_bindeff | vfx | 85 | 88 |
| 547 | `0x88c` | `0x007fc130` | op_et_draw_flags_restore | vfx | 199 | 205 |
| 548 | `0x890` | `0x007fc190` | op_et_draw_flags_save | vfx | 14 | 14 |
| 549 | `0x894` | `0x008000f0` | op_oef2_sound_cue_check | vfx | 216 | 252 |
| 550 | `0x898` | `0x00800330` |  | vfx? | 100 | 124 |
| 551 | `0x89c` | `0x00800390` | op_oef2_stream_cue | vfx | 132 | 166 |
| 552 | `0x8a0` | `0x0062fca0` | FFXVu0CopyVectorXYZ | ps2-emu | 220 | 223 |
| 553 | `0x8a4` | `0x006ed4f0` | FFXVu0LightColorMatrix | ps2-emu | 23 | 23 |
| 554 | `0x8a8` | `0x0093d550` |  | map | 220 | 244 |
| 555 | `0x8ac` | `0x00908660` |  | map | 224 | 224 |
| 556 | `0x8b0` | `0x009088d0` | yiGetColorScale | map | 224 | 224 |
| 557 | `0x8b4` | `0x00908960` | yiGetDrawMask | map | 8 | 16 |
| 558 | `0x8b8` | `0x00908980` | yiGetFogColor | map | 79 | 79 |
| 559 | `0x8bc` | `0x00908bb0` | yiGetFogParam | map | 5 | 5 |
| 560 | `0x8c0` | `0x0090a2b0` | yiSetDrawMask | map | 8 | 16 |
| 561 | `0x8c4` | `0x008205e0` | Sg_GetPause | main | 66 | 77 |
| 562 | `0x8c8` | `0x0083b650` | Sg_PacketGetPtr | main | 38 | 52 |
| 563 | `0x8cc` | `0x007ec430` |  | vfx | 3 | 3 |
| 564 | `0x8d0` | `0x006ed420` | FFXVu0DivVectorXYZ | ps2-emu | 9 | 17 |
| 565 | `0x8d4` | `0x006ed840` | FFXVu0RotTransPers | ps2-emu | 135 | 251 |
| 566 | `0x8d8` | `0x0079eae0` |  | battle | 3 | 6 |
| 567 | `0x8dc` | `0x00a68c90` |  | vfx | 7 | 7 |
| 568 | `0x8e0` | `0x007ff090` | op_et_chr_show | vfx | 4 | 8 |
| 569 | `0x8e4` | `0x00736690` | pppKeThRes128Con | vfx | 29 | 29 |
| 570 | `0x8e8` | `0x007371c0` | (pppNeiChrPointLight draw) | vfx | 3 | 3 |
| 571 | `0x8ec` | `0x007372b0` | (pppNeiChrPointLight Con) | vfx? | 3 | 3 |
| 572 | `0x8f0` | `0x007372c0` | (pppNeiChrPointLight Des) | vfx? | 3 | 3 |
| 573 | `0x8f4` | `0x00731560` | pppRandDownIV | vfx | 5 | 5 |
| 574 | `0x8f8` | `0x007fc910` |  | vfx? | 6 | 6 |
| 575 | `0x8fc` | `0x007fc900` |  | vfx? | 6 | 6 |
| 576 | `0x900` | `0x00795ff0` |  | battle | 2 | 2 |
| 577 | `0x904` | `0x00909d30` |  | map | 0 | 0 |
| 578 | `0x908` | `0x00909d70` |  | map | 0 | 0 |
| 579 | `0x90c` | `0x00833940` | Ch_Set7f8 | chara | 2 | 4 |
| 580 | `0x910` | `0x0079ad90` | MsGetRamChrDisableZan | battle | 2 | 2 |
| 581 | `0x914` | `0x007ec420` |  | vfx | 13 | 13 |
| 582 | `0x918` | `0x00757370` | (pppDrawHook draw) | vfx | 10 | 10 |
| 583 | `0x91c` | `0x0082cba0` | Sg_FadeInW | main | 1 | 1 |
| 584 | `0x920` | `0x0083c9d0` | Ch_ClutGetPtr | chara | 1 | 1 |
| 585 | `0x924` | `0x0083cd20` | Ch_TexGetClut | chara | 1 | 1 |
| 586 | `0x928` | `0x0083cd90` | Ch_TexGetNbClut | chara | 1 | 1 |
| 587 | `0x92c` | `0x0083d070` | Ch_TextureSetAnimTimer | chara | 1 | 1 |
| 588 | `0x930` | `0x008361c0` | Sg_LightGetDir | main | 8 | 12 |
| 589 | `0x934` | `0x00836310` | Sg_LightSetColor | main | 6 | 24 |
| 590 | `0x938` | `0x00836430` | Sg_LightSetDir | main | 6 | 11 |
| 591 | `0x93c` | `0x00731350` | pppRandUpIV | vfx | 6 | 6 |
| 592 | `0x940` | `0x007b7f20` |  | battle? | 2 | 2 |
| 593 | `0x944` | `0x007b2b70` |  | vfx? | 2 | 2 |
| 594 | `0x948` | `0x007b2be0` |  | vfx? | 2 | 2 |
| 595 | `0x94c` | `0x007b2ba0` |  | vfx? | 2 | 2 |
| 596 | `0x950` | `0x007ab140` | MsMoveSplineSet | battle | 1 | 1 |
| 597 | `0x954` | `0x007ab0b0` | MsMoveSplineRegist | battle | 1 | 5 |
| 598 | `0x958` | `0x007ab010` |  | battle | 1 | 1 |
| 599 | `0x95c` | `0x00794280` |  | battle | 1 | 1 |
| 600 | `0x960` | `0x00795d40` |  | battle | 1 | 1 |
| 601 | `0x964` | `0x00794d20` | MsGetChrDir | battle | 118 | 166 |
| 602 | `0x968` | `0x0082aa70` | Ch_EnMapID | chara | 1 | 1 |
| 603 | `0x96c` | `0x00768d30` | KeDttSpr_MkGifPk_Pre | map | 1 | 1 |
| 604 | `0x970` | `0x00767790` | KeDttSpr_MkGifPk | map | 1 | 1 |
| 605 | `0x974` | `0x007366c0` | pppKeThRes128x4Con | vfx | 2 | 2 |
| 606 | `0x978` | `0x0082b970` | Ch_Vu1SetClipmode | chara | 1 | 2 |
| 607 | `0x97c` | `0x00749b40` | (pppKeMdlTfd2 draw) | vfx | 2 | 2 |
| 608 | `0x980` | `0x00749b10` | pppKeMdlTfd2Con | vfx | 2 | 2 |
| 609 | `0x984` | `0x007341e0` | pppKeMatPht | vfx | 1 | 1 |
| 610 | `0x988` | `0x00735020` | pppKeDMatPhtFrDraw | vfx | 1 | 1 |
| 611 | `0x98c` | `0x00753f10` | pppKeShpTailPht | vfx | 1 | 1 |
| 612 | `0x990` | `0x00754300` | (pppKeShpTailPht draw) | vfx? | 1 | 1 |
| 613 | `0x994` | `0x00754180` | pppKeShpTailPhtCon | vfx | 1 | 1 |
| 614 | `0x998` | `0x00754260` | pppKeShpTailPhtCon2 | vfx | 1 | 1 |
| 615 | `0x99c` | `0x00749c90` | (pppKeMdlTfdUv2 draw) | vfx | 1 | 1 |
| 616 | `0x9a0` | `0x00749c30` | pppKeMdlTfdUv2Con | vfx | 1 | 1 |
| 617 | `0x9a4` | `0x007b7280` | MsGetTargetRandomList2 | battle | 2 | 2 |
| 618 | `0x9a8` | `0x0079a760` |  | battle | 2 | 2 |
| 619 | `0x9ac` | `0x00729570` | pppFpIDdispOff | vfx | 1 | 1 |
| 620 | `0x9b0` | `0x00a69de0` |  | vfx | 1 | 1 |
| 621 | `0x9b4` | `0x008356b0` | Ch_OptposGetMatrix | chara | 1 | 1 |
| 622 | `0x9b8` | `0x0079a860` |  | battle | 10 | 10 |
| 623 | `0x9bc` | `0x007fc9b0` |  | vfx | 13 | 13 |
| 624 | `0x9c0` | `0x007fca40` | op_et_weapon_afterimage_fade | vfx | 13 | 13 |
| 625 | `0x9c4` | `0x00795da0` |  | battle | 2 | 2 |
| 626 | `0x9c8` | `0x00788e90` | MsSetChrMoveFlagAll | battle | 2 | 2 |
| 627 | `0x9cc` | `0x007b7d00` |  | battle | 2 | 3 |
| 628 | `0x9d0` | `0x00821cb0` | Sg_SetPause | main | 16 | 48 |
| 629 | `0x9d4` | `0x007afde0` | MsEffectLimitSync | battle | 6 | 6 |
| 630 | `0x9d8` | `0x007afd30` | MsEffectLimitStart | battle | 6 | 6 |
| 631 | `0x9dc` | `0x007fc9a0` |  | vfx? | 1 | 1 |
| 632 | `0x9e0` | `0x00832630` | Ch_BindToElem | chara | 2 | 8 |
| 633 | `0x9e4` | `0x007fc990` |  | vfx? | 1 | 1 |
| 634 | `0x9e8` | `0x0082b1b0` | Ch_SetDirF | chara | 2 | 2 |
| 635 | `0x9ec` | `0x007b7d70` |  | battle | 1 | 1 |
| 636 | `0x9f0` | `0x007b7e20` |  | battle | 1 | 1 |
| 637 | `0x9f4` | `0x007b7e80` |  | battle | 1 | 1 |
| 638 | `0x9f8` | `0x00733a00` | pppSRandUpHCV | vfx | 1 | 1 |
| 639 | `0x9fc` | `0x0082cbe0` | Sg_FadeOutW | main | 2 | 2 |
| 640 | `0xa00` | `0x00795780` | MsGetChrStatDeath | battle | 1 | 1 |
| 641 | `0xa04` | `0x007abba0` | MsGetSaveChrWeaponModel | battle | 20 | 20 |
| 642 | `0xa08` | `0x00741570` | pppDrawShapeRevCon | vfx | 20 | 20 |
| 643 | `0xa0c` | `0x007415a0` | (pppDrawShapeRev draw) | vfx? | 20 | 20 |
| 644 | `0xa10` | `0x00736cc0` | pppKeThRes8Con | vfx | 20 | 20 |
| 645 | `0xa14` | `0x007b1910` |  | battle | 10 | 10 |
| 646 | `0xa18` | `0x007b19f0` |  | battle | 10 | 10 |
| 647 | `0xa1c` | `0x007b1a70` | MsRestartMagicCamera | battle | 10 | 10 |
| 648 | `0xa20` | `0x00890e70` |  | system? | 10 | 10 |
| 649 | `0xa24` | `0x00890f10` |  | system? | 10 | 10 |
| 650 | `0xa28` | `0x008974a0` |  | system | 10 | 10 |
| 651 | `0xa2c` | `0x00897780` |  | system? | 10 | 20 |
| 652 | `0xa30` | `0x008977c0` |  | system? | 10 | 10 |
| 653 | `0xa34` | `0x008977e0` |  | system? | 10 | 10 |
| 654 | `0xa38` | `0x00897f00` |  | system | 10 | 10 |
| 655 | `0xa3c` | `0x00898230` |  | system? | 10 | 10 |
| 656 | `0xa40` | `0x00898dc0` |  | system? | 10 | 10 |
| 657 | `0xa44` | `0x0089a2a0` |  | system | 10 | 10 |
| 658 | `0xa48` | `0x0089a2d0` |  | system | 10 | 10 |
| 659 | `0xa4c` | `0x0089ab20` |  | system | 10 | 10 |
| 660 | `0xa50` | `0x0089ab50` |  | system | 10 | 10 |
| 661 | `0xa54` | `0x0089ab80` |  | system? | 10 | 10 |
| 662 | `0xa58` | `0x0089aba0` |  | system | 10 | 10 |
| 663 | `0xa5c` | `0x007ffde0` |  | vfx | 10 | 10 |
| 664 | `0xa60` | `0x00794990` |  | battle | 116 | 120 |
| 665 | `0xa64` | `0x00795980` | MsGetChrTopPos | battle | 116 | 119 |
| 666 | `0xa68` | `0x00795dd0` |  | battle | 8 | 8 |
| 667 | `0xa6c` | `0x006ed3a0` | FFXVu0DivVector | ps2-emu | 116 | 118 |
| 668 | `0xa70` | `0x006ed5d0` | FFXVu0MulVector | ps2-emu | 116 | 469 |
| 669 | `0xa74` | `0x0090a2c0` |  | map | 116 | 234 |
| 670 | `0xa78` | `0x0082afb0` | Ch_MotionSetHokan | chara | 7 | 29 |
| 671 | `0xa7c` | `0x0082b2c0` | Ch_SetHideEvent | chara | 3 | 4 |
| 672 | `0xa80` | `0x007bd470` | MsCameraGetPosNow | battle | 2 | 3 |
| 673 | `0xa84` | `0x007e6b20` |  | vfx | 28 | 30 |
| 674 | `0xa88` | `0x007bc0e0` |  | battle | 9 | 11 |
| 675 | `0xa8c` | `0x0075b690` | (pppKeAcmCic step) | vfx | 3 | 3 |
| 676 | `0xa90` | `0x00764bb0` | KeThResHd_Init | map | 10 | 16 |
| 677 | `0xa94` | `0x007644d0` | KeThCp_AddBp | map | 12 | 12 |
| 678 | `0xa98` | `0x00764800` | KeThCp_Init | map | 10 | 10 |
| 679 | `0xa9c` | `0x007649a0` | KeThCp_Update | map | 12 | 12 |
| 680 | `0xaa0` | `0x007645e0` | KeThCp_Draw | map | 12 | 12 |
| 681 | `0xaa4` | `0x00736c30` | pppKeThRes64x16Con | vfx | 2 | 2 |
| 682 | `0xaa8` | `0x00736a80` | pppKeThRes40Con | vfx | 1 | 1 |
| 683 | `0xaac` | `0x00733d10` | pppSRandDownHCV | vfx | 4 | 4 |
| 684 | `0xab0` | `0x00787c10` | MsEffectGetTargetFlag | battle | 9 | 9 |
| 685 | `0xab4` | `0x0075ede0` | KeVu0CopyVecXYZ | map | 2 | 2 |
| 686 | `0xab8` | `0x007bd000` | MsCameraFree | battle | 2 | 2 |
| 687 | `0xabc` | `0x007bd110` | MsCameraGet | battle | 2 | 2 |
| 688 | `0xac0` | `0x007883f0` | MsEffectSetMotionFlag | battle | 3 | 3 |
| 689 | `0xac4` | `0x00788dd0` |  | battle | 2 | 6 |
| 690 | `0xac8` | `0x007bc750` |  | battle | 2 | 4 |
| 691 | `0xacc` | `0x007bc890` |  | battle | 2 | 4 |
| 692 | `0xad0` | `0x007bc950` |  | main | 2 | 8 |
| 693 | `0xad4` | `0x0082bd10` | Sg_AccSetAlpha | main | 2 | 4 |
| 694 | `0xad8` | `0x0082bdc0` |  | main | 3 | 6 |
| 695 | `0xadc` | `0x007fbb90` | op_et_bindeff_off | vfx | 2 | 2 |
| 696 | `0xae0` | `0x00790290` |  | battle | 2 | 4 |
| 697 | `0xae4` | `0x00a69300` |  | vfx | 1 | 1 |
| 698 | `0xae8` | `0x00784ff0` | MsGetSaveChrSummon | battle | 1 | 1 |
| 699 | `0xaec` | `0x00759d80` | pppKeHitBorn | vfx | 1 | 1 |
| 700 | `0xaf0` | `0x00759f10` | pppKeThHitBornCon | vfx | 1 | 1 |
| 701 | `0xaf4` | `0x007ab540` |  | battle | 3 | 5 |
| 702 | `0xaf8` | `0x00795c90` |  | battle | 2 | 2 |
| 703 | `0xafc` | `0x00794e80` |  | battle | 1 | 1 |
| 704 | `0xb00` | `0x00803220` |  | vfx | 0 | 0 |
| 705 | `0xb04` | `0x008360a0` |  | main | 1 | 3 |
| 706 | `0xb08` | `0x0075fda0` | KeVu0VecLen | map | 2 | 2 |
| 707 | `0xb0c` | `0x0082cb60` | Sg_EnFocus | main | 2 | 3 |
| 708 | `0xb10` | `0x00729660` | Fp_End | vfx | 1 | 10 |
| 709 | `0xb14` | `0x00729700` | Fp_Restart | vfx | 1 | 5 |
| 710 | `0xb18` | `0x007169d0` | pppInitEnv | vfx | 537 | 543 |
| 711 | `0xb1c` | `0x00711fa0` | pppInitData | vfx | 537 | 543 |
| 712 | `0xb20` | `0x00712250` | pppRunPart | vfx | 537 | 617 |
| 713 | `0xb24` | `0x00711f20` | pppAllFreePObject | vfx | 532 | 1659 |
| 714 | `0xb28` | `0x007123c0` | pppStartPart | vfx | 537 | 537 |
| 715 | `0xb2c` | `0x007fda80` |  | vfx | 130 | 130 |
| 716 | `0xb30` | `0x0080cc00` | opu_main_host | vfx | 130 | 390 |
| 717 | `0xb34` | `0x007fdc80` |  | vfx? | 130 | 130 |
| 718 | `0xb38` | `0x007e3e30` | op_oef2_set2 | vfx | 541 | 633 |
| 719 | `0xb3c` | `0x007e3810` | op_oef2_a_get | vfx | 536 | 2398 |
| 720 | `0xb40` | `0x009dbea0` |  | system? | 354 | 354 |
| 721 | `0xb44` | `0x0080bd50` | opp_main | vfx | 130 | 130 |
| 722 | `0xb48` | `0x007ff2b0` | op_heap_display_set_par | vfx | 130 | 130 |
| 723 | `0xb4c` | `0x007bba50` | MsCopyScreenMatrix | battle | 539 | 778 |
| 724 | `0xb50` | `0x007c0070` | MsCopyCameraMatrix | battle | 539 | 784 |
| 725 | `0xb54` | `0x0094964c` |  | crt? | 0 | 0 |
| 726 | `0xb58` | `0x00949f50` |  | crt? | 142 | 142 |
| 727 | `0xb5c` | `0x0062f510` | debug_exit_trace | system | 130 | 130 |
| 728 | `0xb60` | `0x00a73810` |  | vfx | 581 | 581 |
| 729 | `0xb64` | `0x00a74410` |  | vfx | 581 | 581 |
| 730 | `0xb68` | `0x00a73d90` | opp2_md_break | vfx | 581 | 581 |
| 731 | `0xb6c` | `0x00a74a00` |  | vfx | 581 | 581 |
| 732 | `0xb70` | `0x00a74c60` |  | vfx | 581 | 581 |
| 733 | `0xb74` | `0x00a74d30` |  | vfx? | 581 | 581 |
| 734 | `0xb78` | `0x00a756e0` |  | vfx | 581 | 581 |
| 735 | `0xb7c` | `0x007f5f70` |  | vfx | 581 | 581 |
| 736 | `0xb80` | `0x00a6f4d0` |  | vfx | 581 | 581 |
| 737 | `0xb84` | `0x00a76720` |  | script | 581 | 581 |
| 738 | `0xb88` | `0x00a76b60` |  | script | 581 | 581 |
| 739 | `0xb8c` | `0x00a6dbe0` |  | vfx | 581 | 581 |
| 740 | `0xb90` | `0x00a70910` |  | vfx | 581 | 581 |
| 741 | `0xb94` | `0x0230ffe0` |  | data | 0 | 0 |
| 742 | `0xb98` | `0x02332e8c` |  | data | 0 | 0 |
| 743 | `0xb9c` | `0x012a40c0` |  | data | 0 | 0 |
| 744 | `0xba0` | `0x0113fcd0` |  | data | 0 | 0 |
| 745 | `0xba4` | `0x0113fd10` |  | data | 0 | 0 |
| 746 | `0xba8` | `0x02321540` |  | data | 0 | 0 |
| 747 | `0xbac` | `0x0119fe90` |  | data | 0 | 0 |
| 748 | `0xbb0` | `0x02322780` |  | data | 0 | 0 |
| 749 | `0xbb4` | `0x022fb450` |  | data | 2 | 6 |
| 750 | `0xbb8` | `0x011a0060` |  | data | 0 | 0 |
| 751 | `0xbbc` | `0x00c44bf0` |  | data | 0 | 0 |
| 752 | `0xbc0` | `0x0230ff60` |  | data | 0 | 0 |
| 753 | `0xbc4` | `0x0230fd2c` |  | data | 0 | 0 |
| 754 | `0xbc8` | `0x0230ffa0` |  | data | 0 | 0 |
| 755 | `0xbcc` | `0x0230fe20` |  | data | 0 | 0 |
| 756 | `0xbd0` | `0x0230fe60` |  | data | 0 | 0 |
| 757 | `0xbd4` | `0x0230ff20` |  | data | 0 | 0 |
| 758 | `0xbd8` | `0x0230fd34` |  | data | 0 | 0 |
| 759 | `0xbdc` | `0x00c498dc` |  | data | 0 | 0 |
| 760 | `0xbe0` | `0x025d5a20` |  | data | 0 | 0 |
| 761 | `0xbe4` | `0x0230fd30` |  | data | 0 | 0 |
| 762 | `0xbe8` | `0x0230fd50` |  | data | 0 | 0 |
| 763 | `0xbec` | `0x011a0710` |  | data | 0 | 0 |
| 764 | `0xbf0` | `0x023cbbf0` |  | data | 0 | 0 |
| 765 | `0xbf4` | `0x00c3e6c0` |  | data | 0 | 0 |
| 766 | `0xbf8` | `0x009db1f0` |  | ps2-emu? | 0 | 0 |
| 767 | `0xbfc` | `0x009db1a0` |  | ps2-emu? | 0 | 0 |
| 768 | `0xc00` | `0x009db1b0` |  | ps2-emu? | 0 | 0 |
| 769 | `0xc04` | `0x00c64ce0` |  | data | 0 | 0 |
| 770 | `0xc08` | `0x00643900` | graphicSetScreenFadeColor | render | 0 | 0 |
| 771 | `0xc0c` | `0x009db230` | PppCurrentProgramKey_009db230 | ps2-emu | 8 | 28 |
| 772 | `0xc10` | `0x009db250` | WideKey_009db250 | ps2-emu | 123 | 435 |
| 773 | `0xc14` | `0x006453d0` | graphicVFXGetRenderDataByInstance | vfx | 232 | 324 |
| 774 | `0xc18` | `0x0071b8a0` | updateVFXRenderDataShp | system | 123 | 161 |
| 775 | `0xc1c` | `0x006458a0` | graphicVFXUpdate | vfx | 231 | 286 |
| 776 | `0xc20` | `0x00712c30` | calculateVFXSP_AlphaFunction | vfx | 249 | 410 |
| 777 | `0xc24` | `0x00712d00` | calculateVFXSP_ZMat | vfx | 229 | 290 |
| 778 | `0xc28` | `0x00711460` | InitVFXRenderDataShp | vfx | 123 | 161 |
| 779 | `0xc2c` | `0x00645280` | graphicVFXDestroy | vfx | 220 | 399 |
| 780 | `0xc30` | `0x00643980` | graphicSetSpecularMapParams | render | 0 | 0 |
| 781 | `0xc34` | `0x00643150` | graphicSetFogColour | render | 4 | 4 |
| 782 | `0xc38` | `0x006431e0` | graphicSetFogNear | render | 4 | 4 |
| 783 | `0xc3c` | `0x006431a0` | graphicSetFogFar | render | 4 | 4 |
| 784 | `0xc40` | `0x00643180` | graphicSetFogParam_00643180 | render | 4 | 4 |
| 785 | `0xc44` | `0x006431c0` | graphicSetFogLimit | render | 345 | 680 |
| 786 | `0xc48` | `0x00641160` | graphicGetFogColor | render | 4 | 4 |
| 787 | `0xc4c` | `0x006411d0` | graphicGetFogNear | render | 4 | 4 |
| 788 | `0xc50` | `0x006411b0` | graphicGetFogFar | render | 4 | 4 |
| 789 | `0xc54` | `0x006411a0` | graphicGetAppFloat78_006411a0 | render | 4 | 4 |
| 790 | `0xc58` | `0x006411c0` | graphicGetFogLimit | render | 302 | 309 |
| 791 | `0xc5c` | `0x00643270` | graphicSetGlobalAmbientColor | render | 116 | 233 |
| 792 | `0xc60` | `0x00641230` | graphicGetGlobalAmbientColor | render | 1 | 1 |
| 793 | `0xc64` | `0x00643320` | graphicSetAppFloatD0_00643320 | render | 0 | 0 |
| 794 | `0xc68` | `0x006432e0` | graphicSetAppFloatD4_006432e0 | render | 0 | 0 |
| 795 | `0xc6c` | `0x006432c0` | graphicSetAppFloatD8_006432c0 | render | 0 | 0 |
| 796 | `0xc70` | `0x00643300` | graphicSetAppFloatDC_00643300 | render | 0 | 0 |
| 797 | `0xc74` | `0x006412a0` | graphicGetAppFloatD0_006412a0 | render | 0 | 0 |
| 798 | `0xc78` | `0x00641280` | graphicGetAppFloatD4_00641280 | render | 0 | 0 |
| 799 | `0xc7c` | `0x00641270` | graphicGetAppFloatD8_00641270 | render | 0 | 0 |
| 800 | `0xc80` | `0x00641290` | graphicGetAppFloatDC_00641290 | render | 0 | 0 |
| 801 | `0xc84` | `0x00643630` | graphicSetSceneSix0_00643630 | render | 121 | 126 |
| 802 | `0xc88` | `0x008ab570` | SetGlobal_00c59f38 | system | 6 | 12 |
| 803 | `0xc8c` | `0x009dbe90` | AlignedMalloc_thunk_009dbe90 | ps2-emu | 182 | 186 |
| 804 | `0xc90` | `0x009dbe80` | AlignedFree_thunk_009dbe80 | ps2-emu | 180 | 184 |
| 805 | `0xc94` | `0x009db1c0` |  | ps2-emu? | 0 | 0 |
| 806 | `0xc98` | `0x009db200` |  | ps2-emu? | 0 | 0 |
| 807 | `0xc9c` | `0x009db210` |  | ps2-emu? | 0 | 0 |
| 808 | `0xca0` | `0x007b1b50` | MsSetReelResult | battle | 10 | 10 |
| 809 | `0xca4` | `0x009066d0` | TOSetUIVertexXY_009066d0 | system | 10 | 280 |
| 810 | `0xca8` | `0x009066b0` | TOSetUIVertexUV_009066b0 | system | 10 | 20 |
| 811 | `0xcac` | `0x00906680` | SeveralStoresSetTableConstantsArgumentsAddresses_00906680 | system | 10 | 280 |
| 812 | `0xcb0` | `0x00906420` | graphicDrawUIElementAfterSet | render | 10 | 140 |
| 813 | `0xcb4` | `0x008e8980` | OpenScissor | system | 10 | 10 |
| 814 | `0xcb8` | `0x008e5cd0` | CloseScissor | menu | 10 | 10 |
| 815 | `0xcbc` | `0x00711830` | pppKaGra001Draw_00711830 | vfx | 10 | 16 |
| 816 | `0xcc0` | `0x0080a4a0` |  | vfx? | 19 | 19 |
| 817 | `0xcc4` | `0x0080a460` |  | vfx? | 16 | 16 |
| 818 | `0xcc8` | `0x00642110` | graphicMagic_DrawColorModel_KaMdldrw | render | 2 | 2 |
| 819 | `0xccc` | `0x00718610` | updateVFXRenderData | render | 121 | 129 |
| 820 | `0xcd0` | `0x00711180` | InitVFXRenderData | vfx | 113 | 121 |
| 821 | `0xcd4` | `0x00639110` | graphicActivateBlurEffectEx | render | 54 | 60 |
| 822 | `0xcd8` | `0x00643830` | graphicBlurEffectSetPositiveParams_00643830 | render | 38 | 44 |
| 823 | `0xcdc` | `0x00643890` | graphicBlurEffectSetFloats6c_00643890 | render | 38 | 44 |
| 824 | `0xce0` | `0x00643860` | graphicBlurEffectSetFloats74_00643860 | render | 38 | 44 |
| 825 | `0xce4` | `0x00826b70` | Ch_GetChrRenderInstance | chara | 14 | 14 |
| 826 | `0xce8` | `0x009db1d0` |  | ps2-emu? | 0 | 0 |
| 827 | `0xcec` | `0x0113335c` |  | data | 0 | 0 |
| 828 | `0xcf0` | `0x007117c0` | opu_fake_env_model_draw_007117c0 | vfx | 2 | 6 |
| 829 | `0xcf4` | `0x00712ec0` | checkEnvMdl0434 | vfx | 1 | 2 |
| 830 | `0xcf8` | `0x00712690` | setPolyTris0434 | vfx | 1 | 2 |
| 831 | `0xcfc` | `0x00713160` | checkPolyImg0434 | vfx | 3 | 6 |
| 832 | `0xd00` | `0x007128d0` | setPolyQuad0434 | vfx | 3 | 6 |
| 833 | `0xd04` | `0x0063b810` | graphicCharacterInstanceSetMeshAlpha | render | 14 | 26 |
| 834 | `0xd08` | `0x0063b9e0` | graphicCharacterInstanceSetMeshAlphaThreshold | render | 2 | 2 |
| 835 | `0xd0c` | `0x0063c9e0` | graphicCharacterLightingSet4_0063c9e0 | render | 0 | 0 |
| 836 | `0xd10` | `0x0063ca50` | graphicCharacterLightingSet8_0063ca50 | render | 0 | 0 |
| 837 | `0xd14` | `0x0063cb00` | graphicCharacterLightingSetLightColor | render | 0 | 0 |
| 838 | `0xd18` | `0x0063cdd0` | graphicCharacterLightingSetShadeCoeff | render | 0 | 0 |
| 839 | `0xd1c` | `0x0063d050` | graphicCharacterLightingSetShadeColor | render | 0 | 0 |
| 840 | `0xd20` | `0x00645400` | graphicVFXIsTextureLoaded | vfx | 0 | 0 |
| 841 | `0xd24` | `0x006436a0` | graphicSetSceneSix1_006436a0 | render | 0 | 0 |
| 842 | `0xd28` | `0x0063c430` | graphicCharacterInstanceSetWorldMatrixByMdlID | render | 2 | 6 |
| 843 | `0xd2c` | `0x0063c3e0` | graphicCharacterInstanceSetWorldMatrix | render | 19 | 19 |
| 844 | `0xd30` | `0x00830cd0` | Ch_SysDrawSetSituation | chara | 0 | 0 |
| 845 | `0xd34` | `0x0063df50` | graphicDetachChrClusterFrom3DWorld | render | 6 | 7 |
| 846 | `0xd38` | `0x00639350` | graphicAttachChrClusterTo3DWorld_00639350 | render | 5 | 5 |
| 847 | `0xd3c` | `0x0063b7f0` | graphicCharacterInstanceSetDrawSituationCallback | render | 14 | 28 |
| 848 | `0xd40` | `0x00643b80` | graphicSceneMatrixTranspose_00643b80 | render | 0 | 0 |
| 849 | `0xd44` | `0x0082b260` | Ch_SetHideBattle | chara | 2 | 2 |
| 850 | `0xd48` | `0x00ccca58` |  | data | 0 | 0 |
| 851 | `0xd4c` | `0x009db220` |  | ps2-emu? | 0 | 0 |
| 852 | `0xd50` | `0x00826db0` | Ch_HandleRewind | chara | 1 | 1 |
| 853 | `0xd54` | `0x00826d70` | Ch_HandleGetNext | chara | 1 | 2 |
| 854 | `0xd58` | `0x00000000` | (null) | - | 0 | 0 |
| 855 | `0xd5c` | `0x0063d810` | graphicCharacterSetShaderType | render | 4 | 4 |
| 856 | `0xd60` | `0x0063d870` | graphicCharacterShaderRestore | render | 4 | 4 |
| 857 | `0xd64` | `0x0063d940` | graphicCharacterShaderSetParameterVector_0063d940 | render | 4 | 9 |
| 858 | `0xd68` | `0x00642fe0` | movie_disp2 | movie | 9 | 9 |
| 859 | `0xd6c` | `0x006438c0` | graphicSetNumberText_006438c0 | render | 4 | 4 |
| 860 | `0xd70` | `0x007116d0` | pppNoiseBlurDraw_007116d0 | vfx | 1 | 1 |
| 861 | `0xd74` | `0x009db180` | memcpy_thunk_009db180 | ps2-emu | 4 | 4 |
| 862 | `0xd78` | `0x00711750` | pppAkazaScrFltDraw_00711750 | vfx | 2 | 2 |
| 863 | `0xd7c` | `0x006429f0` | graphicRegisterDelegation_006429f0 | render | 0 | 0 |
| 864 | `0xd80` | `0x00645e60` | graphicsRegisterResetScreenFadeDelegation | render | 0 | 0 |
| 865 | `0xd84` | `0x00645ba0` | graphicVFXUpdateWithType | vfx | 2 | 6 |
| 866 | `0xd88` | `0x009db1e0` |  | ps2-emu? | 0 | 0 |
| 867 | `0xd8c` | `0x00a5bf50` | updateVFXRenderData_ScrollModelEntry_00a5bf50 | map | 2 | 6 |
| 868 | `0xd90` | `0x006438e0` | graphicSetScreenFadeAlphaMode | render | 0 | 0 |
| 869 | `0xd94` | `0x007bd300` | MsCameraGetID | battle | 2 | 5 |
| 870 | `0xd98` | `0x00645600` | graphicVFXPath3Update | vfx | 9 | 17 |
| 871 | `0xd9c` | `0x00710fd0` | InitVFXPath3RenderData | vfx | 8 | 8 |
| 872 | `0xda0` | `0x00711030` | InitVFXPath3RenderDataShp | vfx | 0 | 0 |
| 873 | `0xda4` | `0x00711500` | InitVFXRenderData_KaGra001 | vfx | 7 | 7 |
| 874 | `0xda8` | `0x007110d0` | InitVFXRenderDataShp_OShapeVu1 | vfx | 4 | 4 |
| 875 | `0xdac` | `0x0071c0b0` | updateVFXRenderDataShp_OShapeVu1 | system | 7 | 11 |
| 876 | `0xdb0` | `0x00699080` | graphicPostProcessSetFlag_00699080 | render | 0 | 0 |
| 877 | `0xdb4` | `0x008e5f60` | MenuDrawUiElement_008e5f60 | menu | 0 | 0 |
| 878 | `0xdb8` | `0x00641e40` | graphicMagic0146SetKey | render | 3 | 4 |
| 879 | `0xdbc` | `0x00643770` | graphicSetNoLightSpecularPassParam | render | 9 | 9 |
| 880 | `0xdc0` | `0x0230fca0` |  | data | 0 | 0 |
| 881 | `0xdc4` | `0x00644de0` | graphicVFXCreate | vfx | 3 | 3 |
| 882 | `0xdc8` | `0x00645040` | graphicVFXCreateDataBuffer | vfx | 3 | 3 |
| 883 | `0xdcc` | `0x00645780` | graphicVFXSetup | vfx | 3 | 3 |
| 884 | `0xdd0` | `0x00645710` | graphicVFXSetVisible | vfx | 3 | 3 |
| 885 | `0xdd4` | `0x00645450` | graphicVFXIsTextureWithBpLoaded | vfx | 19 | 21 |
| 886 | `0xdd8` | `0x0063fdb0` | graphicDynGeoMemManagerGetBufferForSegment | render | 8 | 8 |
| 887 | `0xddc` | `0x0063fc50` | GetFieldPtr94_0063fc50 | render | 3 | 3 |
| 888 | `0xde0` | `0x0063fc70` | GetField90_0063fc70 | render | 0 | 0 |
| 889 | `0xde4` | `0x0063fd30` | SetField90_0063fd30 | render | 0 | 0 |
| 890 | `0xde8` | `0x0063fc40` | FieldAddr98_0063fc40 | render | 3 | 3 |
| 891 | `0xdec` | `0x0063fc30` | FieldAddra4_0063fc30 | render | 3 | 3 |
| 892 | `0xdf0` | `0x0063fd50` | SetField80_0063fd50 | render | 3 | 3 |
| 893 | `0xdf4` | `0x0063fc80` | Identity_0063fc80 | render | 3 | 3 |
| 894 | `0xdf8` | `0x0063fcf0` | GetField4_0063fcf0 | render | 0 | 0 |
| 895 | `0xdfc` | `0x0063fd10` | SetField4_0063fd10 | render | 0 | 0 |
| 896 | `0xe00` | `0x0063fcb0` | GetField8_0063fcb0 | render | 0 | 0 |
| 897 | `0xe04` | `0x0063fd00` | SetField8_0063fd00 | render | 0 | 0 |
| 898 | `0xe08` | `0x0063fce0` | GetFieldc_0063fce0 | render | 3 | 3 |
| 899 | `0xe0c` | `0x0063fcc0` | GetField10_0063fcc0 | render | 0 | 0 |
| 900 | `0xe10` | `0x0063fc90` | GetField14_0063fc90 | render | 3 | 3 |
| 901 | `0xe14` | `0x0063fcd0` | GetField18_0063fcd0 | render | 2 | 2 |
| 902 | `0xe18` | `0x0063fca0` | GetField1c_0063fca0 | render | 3 | 3 |
| 903 | `0xe1c` | `0x0063fd20` | SetField24_0063fd20 | render | 2 | 2 |
| 904 | `0xe20` | `0x00644be0` | graphicVFXAddCopyBufferEvent | vfx | 1 | 2 |
| 905 | `0xe24` | `0x00642e00` | graphicSetAkazaNoLightSpecularPassParam | render | 7 | 8 |
| 906 | `0xe28` | `0x0063d8e0` | graphicCharacterShaderSetParameterTexture | render | 1 | 1 |
| 907 | `0xe2c` | `0x007113d0` | InitVFXRenderDataShapeField | vfx | 5 | 5 |
| 908 | `0xe30` | `0x0071a4e0` | updateVFXRenderDataShapeField | render | 5 | 9 |
| 909 | `0xe34` | `0x00713290` | clearVFXRenderDataShp | vfx | 1 | 2 |
| 910 | `0xe38` | `0x00712bd0` | caculateVFXRenderDataShapeFieldBufferSize | vfx | 1 | 1 |
| 911 | `0xe3c` | `0x007fa6d0` | opp_0706_cmd_007fa6d0 | vfx | 581 | 581 |
| 912 | `0xe40` | `0x00642e40` | graphicSetAkazaSpecularPassParam | render | 2 | 2 |
| 913 | `0xe44` | `0x00639170` | graphicAddMeshMatrix | render | 4 | 4 |
| 914 | `0xe48` | `0x00641e60` | graphicMagic0708ConvBallChr | render | 1 | 1 |
| 915 | `0xe4c` | `0x00639240` | graphicAddScreenFadeTask | render | 91 | 101 |
| 916 | `0xe50` | `0x007bd860` | MsCameraMoveAcc | battle | 1 | 2 |
| 917 | `0xe54` | `0x009db170` |  | ps2-emu | 1 | 2 |
| 918 | `0xe58` | `0x006437e0` | SetGlobal00cec1a0_006437e0 | render | 1 | 2 |
| 919 | `0xe5c` | `0x01133398` |  | data | 0 | 0 |
| 920 | `0xe60` | `0x0063a2b0` | graphicCharacterInstanceGetBoundingBox | render | 0 | 0 |
| 921 | `0xe64` | `0x00c3bbb0` |  | data | 0 | 0 |
| 922 | `0xe68` | `0x0063c390` | graphicCharacterInstanceSetVisible | render | 10 | 10 |
| 923 | `0xe6c` | `0x009dafd0` | SetGlobal_0193fdac | system | 1 | 2 |
| 924 | `0xe70` | `0x0063a410` | graphicCharacterGetBounds_0063a410 | render | 2 | 2 |
| 925 | `0xe74` | `0x00794020` | MsGetChr | battle | 4 | 4 |
| 926 | `0xe78` | `0x00793650` | MsBtlChrNumCheck | battle | 4 | 4 |
| 927 | `0xe7c` | `0x006392c0` | graphicAddScreenFadeTaskBeforeVFX | render | 1 | 1 |
| 928 | `0xe80` | `0x00644dd0` | graphicClearSpecialRenderList | render | 1 | 1 |
| 929 | `0xe84` | `0x0063dac0` | graphicSceneClearList_0063dac0 | render | 2 | 4 |
| 930 | `0xe88` | `0x0063d710` | graphicCharacterSetClipPlane | render | 2 | 4 |
| 931 | `0xe8c` | `0x00644ca0` | graphicVFXAddDrawCharacterEvent | vfx | 16 | 18 |
| 932 | `0xe90` | `0x00645840` | graphicVFXSyncPath | vfx | 3 | 3 |
| 933 | `0xe94` | `0x00643670` | graphicSetMaterialColourScale2 | render | 225 | 322 |
| 934 | `0xe98` | `0x0063a610` | graphicCharacterGetJointBounds_0063a610 | render | 2 | 2 |
| 935 | `0xe9c` | `0x01940ac4` |  | data | 0 | 0 |
| 936 | `0xea0` | `0x012fb7d0` |  | data | 0 | 0 |
| 937 | `0xea4` | `0x00713320` | clearVFXRenderDataShpWithMemset | vfx | 4 | 4 |
| 938 | `0xea8` | `0x00644cf0` | graphicVFXAddDrawZPrePassEvent | vfx | 2 | 2 |
| 939 | `0xeac` | `0x00641100` | LocalizationManager_GetWord0 | system | 1 | 1 |
| 940 | `0xeb0` | `0x00a44660` | SetGlobal01a85f38_00a44660 | menu | 0 | 0 |
| 941 | `0xeb4` | `0x00a44650` | get_ptr32_func | menu | 539 | 2848 |
| 942 | `0xeb8` | `0x00a44700` | set_ptr32 | system | 35 | 55 |
| 943 | `0xebc` | `0x00a446b0` | Compute_00a446b0 | menu | 0 | 0 |
| 944 | `0xec0` | `0x00a446a0` | is_ptr32_null | menu | 0 | 0 |
| 945 | `0xec4` | `0x00644c70` | graphicVFXAddCopyBufferEventWithRGB | vfx | 2 | 2 |
| 946 | `0xec8` | `0x00829c60` | set0434isDarkYojimbo | main | 1 | 1 |
