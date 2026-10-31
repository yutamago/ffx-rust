# Replacing FFX.exe functions with Rust

The rebuild proceeds one function at a time. Each replacement is written in
Rust, checked against the original machine code, and then patched into the
running game in place of the original. Once enough of the game is replaced,
`FFX.exe` is no longer needed.

## Pieces

| Crate | Role |
|---|---|
| module crates (`ffx-battle`, `ffx-script`, `ffx-prender`, ...) | The ports: a core on the module's own types and a `bridge` over game memory (`docs/porting.md`) |
| `ffx-bridge` | Game memory as a port sees it (`Mem`), and calls into code that is still original (`Host`) |
| `ffx-oracle` | Runs the original functions from `FFX.exe` in a Unicorn x86 emulator, so tests can compare them with the ports on Linux |
| `ffx-hook` | The hook table, the patcher, and the shims with the originals' exact signatures (`src/ported/<module>.rs`) |
| `ffx-dinput8` | `dinput8.dll` proxy that loads `ffx-hook` into the game |

## How a hook works

`FFX.exe` imports `DINPUT8.dll!DirectInput8Create`, and Windows looks for
DLLs in the game folder first. Our `dinput8.dll` is therefore loaded before
any game code runs. Its `DllMain`:

1. checks the main module's PE `TimeDateStamp` and `SizeOfImage` against the
   supported build, and patches nothing if they differ;
2. rebases each hook address onto the module's real base (ASLR is on);
3. checks that each hand-written hook's site still holds the original
   prologue (the `ported!` entries carry none; `tools/check-ported.py`
   checks instead that each is a function start of at least five bytes,
   by the dump's sizes and `symbols/sizes.tsv`);
4. overwrites the first five bytes with `jmp rel32` to the Rust shim.

A replacement never calls its original again. A wrapper that does
(`Hook::original`) gets a trampoline: the stolen prologue instructions,
then a jump back past them. `DirectInput8Create` itself forwards to the
system `dinput8.dll`.

Results go to `ffx-hook.log` next to `FFX.exe`. Two environment variables
choose the hooks, for A/B comparisons and gates:

- `FFX_HOOK_DISABLE=<names>` (comma-separated, the full `ffx.sym` names)
  keeps those originals. `replacements` stands for every hook without a
  trampoline, so a replay can run the original code.
- `FFX_HOOK_ONLY=<names>` installs only the named replacements; the
  wrappers stay. `FFX_HOOK_ONLY=@<file>` reads the names from a file,
  comma- or line-separated, because the M4 port lists outgrew the 128 KB
  one environment string may hold. The path is a Windows path
  (`tools/scene-gate-capture.sh` passes `wslpath -w`).
  `tools/run-game.sh` passes both on.

One port has a fallback. In lockstep the harness gives the asset loader
its turn at the render lock (`docs/harness.md`, "Load pacing"), and the
port of `Phyre_PMutex_lockInner_00431850` (`ported/p1_load.rs`) asks for
that turn first. When the port is left out (`FFX_HOOK_DISABLE`,
`FFX_HOOK_ONLY`, original-code runs), `loader_turn::install_fallback`
installs a wrapper at the same address instead: the turn rule, then the
original through a trampoline. One hook sits at the address either way.

## Mods

`Phyre_PSerialization_PStreamFileWin32_openFile` (`0x00607f40`) is the only way the game opens archive files.
It normalizes the path, asks the archives, and if they lack the file opens
it from disk with `CreateFileW`; the size, read and close methods then use
that handle. The `Phyre_PSerialization_PStreamFileWin32_openFile` hook adds one step in front: for a read open of
a path that a mod provides, it opens the mod's file read-only and returns,
so the game's own loose-file code reads it. Everything else goes to the
original through a trampoline.

```text
<game folder>/mods/<mod name>/<archive path>
<game folder>/mods/test-potion/ffx_ps2/ffx/master/new_uspc/battle/kernel/item.bin
```

Mods load in name order; a later mod's file replaces an earlier one's, and
the log says so. A mod folder may hold a `mod.toml` (name, description,
`enabled`); with `enabled = false` the mod is skipped. The desktop app
writes it. `tools/run-game.sh --log-opens` (environment
`FFX_HOOK_LOG_OPENS=1`) also logs every file the game reads, as the archive
path a mod would use. The file must be opened read-only: `Phyre_PSerialization_PStreamFile_closeFile` calls
`SetEndOfFile`, which would truncate a writable mod file.

Files of the PS2 game (`ffx_ps2/…`) are also sized by the PS2 file layer,
from its own tables: `CDfileSize_CD` (`0x0076c320`, sectors from
`cdrom.mdg`) and `CDfileSize_PC` (`0x00a428a0`, `sizetbl.bin`), and buffers
and reads use that size. A mod file that grew would be cut at the original
size: a motion group 8 bytes larger lost the end of its sequence table and
crashed the game. Both functions are wrapped: for a PS2 file number whose
path (`CDfileName_PC`, `0x00a42830`, `host0:/ffx/…` → `ffx_ps2/ffx/…`) a mod
provides, they return the mod file's size rounded up to 16 bytes.

`ffx kernel import <table> <edits.toml> --mod-dir <game folder>/mods/<mod>`
writes a table straight into a mod. Verified in the game: with the
`test-potion` mod, the item table in game memory (read with
`tools/frida/peek.py 0x0112a940 0x4100 --deref`) has item 0 named
"Mega Potion" with power 20.

## The hook table

`ffx_hook::hooks()` is the hand-written hooks in `crates/ffx-hook/src/lib.rs`
followed by the `ported!` entries of `crates/ffx-hook/src/ported/*.rs`
(13,905 on 2026-10-26, one file per module or port list). How a port gets
its `ported!` line: `docs/porting.md`.

To find where a hook's port lives, `tools/hook-map.py` lists every `ported!`
entry: address, name, convention, the shim (file:line), the Rust function the
shim calls, and the oracle test files that name the address.
`--find NAME|ADDR` filters it, and `--html FILE` writes a searchable page;
`tools/status.sh` keeps one at `~/ffx-work/status/hook-map.html`.

`ported!` takes `extern "C"` (cdecl), `"stdcall"`, `"thiscall"` or
`"fastcall"`, and makes a thunk per shim. The thunk keeps `eax`, `ecx` and
`edx` for the caller except where the return type puts the result: `eax`
for the integer types, `edx:eax` for `u64`/`i64`, `ST0` for `f32`/`f64`.
`fn shim(args; slots: u32)` also hands the shim the address of the
caller's argument words, for originals that store into their own argument
slots (`docs/workflow.md`, "Argument slots"; not for fastcall).

`extern "fastcall-caller-pops"` is for originals in a custom convention
that MSVC's link-time code generation made for some internal functions:
the first two arguments in `ecx` and `edx` like fastcall, the rest on the
stack, and the caller pops those (`ret`, then the caller's `add esp`).
`Settings_ParseIntList_00401690` and `Settings_ValidateBindings` (their
caller `Settings_SetValue`) are the first. Rust has no such ABI: the
macro declares the shim `extern "C"` with the two register arguments
first, and the thunk pushes `edx`, then `ecx`, below its copy of the stack
words and returns with a plain `ret`. In the oracle the convention is
`Conv::FastcallCallerPops`; `tools/check-callconv.py` and
`tools/check-ported.py` know it.

A few originals are a lone 5-byte `jmp rel32` that the Ghidra dump has
no function for (it folded them into a caller or left a label):
`user_free_thunk` (`0x00687010`) and `code_006f2740`. They are hooked like
any function: `tools/check-ported.py` accepts an address the dump lacks
when FFX.exe's bytes there are `e9 rel32` to a dump function start, outside
every dump function's body (`progress.jmp_thunk`), and checks the shim
against the target's `ret`; check-clobber reads the target's code for the
thunk's callers. The patch covers exactly the thunk's five bytes.

The hand-written hooks:

| Function | Address | What |
|---|---|---|
| `zlib_uncompress` | `0x00405e20` | Replaced (`ffx-core`): 1,500 synthetic streams (fit, spare room, too small, truncated, flipped bit; all result codes) and every compressed block in `metamenu.vbf` |
| `Path_ToLowerAscii` | `0x00607d50` | Replaced (`ffx-core`): NULL, empty, all 255 byte values, 2,000 random strings; output bytes and return value |
| `Phyre_PSerialization_PStreamFileWin32_openFile` | `0x00607f40` | Wrapped: mod lookup, then the original through a trampoline; trampoline checked by the smoke test, mod loading in the game |
| `CDfileSize_CD`, `CDfileSize_PC` | `0x0076c320`, `0x00a428a0` | Wrapped: mod files report their own size; in the game by `an_edited_motion_group_loads_in_the_game`. Other files go to the ports (`ffx_system::m5b_hdd` `file_size_cd`/`file_size_pc`, `ported/m5_system_b.rs`) instead of the trampolines; their names for `FFX_HOOK_DISABLE` and `FFX_HOOK_ONLY` are `CDfileSize_CD_port` and `CDfileSize_PC_port`, and `replacements` turns them off too |
| `Input_KeyHeld` | `0x00629d50` | Wrapped for the test harness (`docs/harness.md`) |
| `Phyre_PTimerWin32_GetTime` | `0x0042fd80` | Wrapped for the harness: adds the skipped time |
| `Phyre_PSerialization_PStreamFileWin32_openFile` | `0x00607f40` | Wrapped: mod lookup, then its port (`ported/m5p_framework_input_df.rs`, `ffx_pother_b::m5p_framework_input_df`), or with the port off (name `Phyre_PSerialization_PStreamFileWin32_openFile_port`) the original through a trampoline; trampoline checked by the smoke test, mod loading in the game |
| `Input_KeyHeld` | `0x00629d50` | Wrapped for the test harness (`docs/harness.md`); the wrapper runs the port (`ported/m5_system_a_df.rs`, `ffx_system::m5a_df`; name `Input_KeyHeld_port`) |
| `Phyre_PTimerWin32_GetTime` | `0x0042fd80` | Wrapped for the harness: adds the skipped time to the port's (`ported/m5p_framework_input_df.rs`; name `Phyre_PTimerWin32_GetTime_port`) |
| `Rng_ClockSeed` | `0x00798950` | Wrapped for the harness: `FFX_HARNESS_SEED` instead of the clock |
| `MsScreenProcess` | `0x007bc110` | Its port (`ffx_battle::screen`), wrapped for the external renderer's free camera (`gfx::live::cam_update`) |
| `rcbgGuideUpdateEx` | `0x0091da80` | Its port (`ffx_field::guide_update`), wrapped for the renderer's minimap (`minimap.rs`) |
| `Phyre_PRendering_PRenderer_renderWorldCulledSorted` | `0x009f3c30` | Wrapped for captures (`world.rs`). The wrapper calls the port (`ported/phyre_render.rs` `culled_sorted`) instead of its trampoline, since two hooks can't share an address. Its name for `FFX_HOOK_DISABLE` and `FFX_HOOK_ONLY` is `Phyre_PRendering_PRenderer_renderWorldCulledSorted_port`; `replacements` turns it off too. It reads only a comma list from `FFX_HOOK_ONLY`, not `@<file>`, so with a file the port is off |
| `AsyncLoadManager_threadMain_00691360` | `0x00691360` | Wrapped for the harness's loader turn (`loader_turn.rs`: notes the loader thread). The wrapper runs the port (`ported/m5_system_b.rs`, `ffx_system::m5b_async`) instead of its trampoline; its name for `FFX_HOOK_DISABLE` and `FFX_HOOK_ONLY` is `AsyncLoadManager_threadMain_00691360_port`, and `replacements` turns it off too |

The harness's frame-boundary wrappers (`harness.rs`, installed only with
`FFX_HARNESS`) run ports too: `FFXApplication_animate` (name
`FFXApplication_animate_port`), `Phyre_PRendering_PRenderer_flip`
(`0x005b6af0`, `ported/m5p_render_b_df.rs`, name
`Phyre_PRendering_PRenderer_flip_port`) and `Renderer_PresentFlip`
(`0x00593760`, `ffx_prender::m5p_render_b_small`, name
`Renderer_PresentFlip_port`). Without the harness, `harness::install`
puts each in as a plain replacement at the same address.

**Wrapper-owned addresses.** Two hooks can't share an address, and the
hook check (`progress.duplicate_hooks`, run by check-ported and
verify-batch) refuses a `ported!` line at an address a hand-written hook
holds. Such a port is a `pub(crate)` function in its list's `ported/`
file, re-exported from `ported/mod.rs` with an `_on()` that reads its
`<name>_port` from `FFX_HOOK_DISABLE`/`FFX_HOOK_ONLY`
(`ported::wrapper_port_on`); the wrapper calls it where it called its
trampoline, and keeps the trampoline for when it is off. Its `ffx.sym`
line says which wrapper calls it; it has a parity test like any port.
`progress.py` and `reach.py` still count these addresses as wrapped.

Only functions whose callers can be satisfied through their signature are
safe to replace. A function that the game expects to touch global state
must touch the same state in Rust.

## What can't be hooked

The hook is a 5-byte `jmp`, so some functions keep their original bytes:

- **Under five bytes.** A bare `ret`, `ret 4`, `xor eax, eax; ret`,
  `mov eax, ecx; ret`, a `lea`/`mov` field return: the jump would overwrite
  the padding or the next function. `tools/classify.py` puts the game's
  ones in the `stub` category (`symbols/classes.tsv`); callers' ports
  reproduce them or call them.
- **A tail-jump forwarder** (`0x00a34b00`): it jumps to `[arg0 + 4]` on its
  caller's stack and return address, passing on however many words the
  target takes. A port would have to call the target and can't know how
  many words to copy or who pops them.
- **The SetThreadName idiom** (`0x004300b0`): it raises the exception
  `0x406d1388` inside its own SEH scope, which a Rust port can't install.

The M4 lists of these, with the evidence, are in
`symbols/m4-not-portable.tsv`.

### Calling originals from a port

A port calls a function that is still the game's through its host or a
callback the shim hands it. Each path passes a fixed number of words. In
the game a longer list is refused or cut, without a panic (a panic in a
function the game calls ends the process); the oracle's test hosts fail
such a call (`ffx_oracle::check_width`), and `python3 -I
tools/check-callwidth.py` finds the wide calls no test reaches. It must
report "0 problems".

The bridge hosts:

| Call | Words | Result |
|---|---|---|
| `ffx_bridge::Host::call` (cdecl) | 0-6 (`CALL_WORDS`); more is not called (0) | `eax` |
| `ffx_script` `NativeHost::call_n` | 0-12 (`CALL_N_WORDS`) | `eax` |
| `ffx_script` `NativeHost::call_float` | 0-6 | `ST0` as `f64` |
| `ffx_graphic::host::Host::call` (`Conv`) | 12 for cdecl, 16 stack words after `this` for thiscall, 8 otherwise (`max_words`) | `eax` |
| `ffx_graphic` `call_st0` / `call_st0_f64` | 8 | `ST0` as `f32` / `f64` |
| `ffx_graphic` `call_u64` / `call_sym_u64` | 8 | `edx:eax` |
| `ffx_graphic` `call_wide` / `call_sym_wide` (cdecl) | 0-32 (`WIDE_WORDS`); more is not called (0) | `eax` |

The `call_sym*` forms take the `ffx.sym` address and rebase it.
`call_wide` is for callees past twelve words, such as
`AsyncLoadManager_loadCluster`'s nineteen from `PhyreScene_fieldMapLoad`.
In the game `ffx_graphic`'s `call` does not refuse a longer list: a cdecl
call drops the words past the limit and the callee-pops conventions pop
more than were pushed. Stay under the limit.

The shim-side helpers in `crates/ffx-hook/src/ported/mod.rs`:

| Helper | Arguments | Result |
|---|---|---|
| `call_cdecl(f, &[u32])` | 0-12 stack words (the rest zero; more are logged and cut) | `eax` |
| `call_cdecl_f64` / `_f32` / `_u64` | as `call_cdecl` | `ST0` / `edx:eax` |
| `call_cdecl_words(f, &[u32])` | 0-32 stack words; more is logged and not called (0) | `eax` |
| `call_thiscall(f, this, &[u32])` | `ecx` + exactly the callee's 0-8 words; more is logged and not called (0) | `eax` |

The `ST0` forms are only for callees that always leave `ST0` set: popping
an empty x87 stack gives a NaN and unbalances it.

`call_cdecl_words` is for callees that take a struct by value: the slice
is the stack as the original pushes it, lowest address first, so a struct
argument is its words in memory order (rounded up to whole words) in its
place among the others. `opu_unit_etc` (`0x0080b180`) passes
`yonExtMapRegister(id, <0x4c-byte struct>)` as 20 words. Where the
original's struct holds stack words it never wrote, the bridge takes them
as an explicit `junk` argument (as for `op_irandom_p`'s three words past its
argument), the shim passes 0 and the oracle test fills the original's frame
with known values and hands the same to the port.

Every call from Rust into game code goes through the call router,
`ffx_bridge::route` (docs/host.md, "One source of truth"): the helpers
above, the hosts' `RawMem` impls (`ffx_bridge::Host`,
`ffx_graphic::host::Host`, `ffx_script`'s `NativeHost`), and a shim's own
calls:

| Call | Arguments | Result |
|---|---|---|
| `route::cdecl::<R>(f, &[u32])` | exactly these stack words | `R` from `eax` (`u32`, `i32`, `u8`, ...), `edx:eax` (`u64`), `ST0` (`f32`: `fstp dword`, `f64`: `fstp qword`) or nothing (`()`) |
| `route::stdcall` | exactly the callee's words (it pops them) | as `cdecl` |
| `route::thiscall::<R>(f, this, &[u32])` | `ecx` + exactly the callee's words | as `cdecl` |
| `route::fastcall` / `route::caller_pops` | the first two in `ecx`, `edx`, the rest on the stack | as `cdecl` |
| `route::x87::<R>(f, &[f64])` | operands loaded with `fld qword` in order (the last in `ST0`): a CRT `_CI*` intrinsic through its thunk or import slot | `ST0` as `R` |

In the hooked game a route call is a native call (one asm routine,
`route::native`, that pushes the words, sets `ecx`/`edx`, calls, puts
`esp` back and reads `eax`, `edx` and `ST0`); in `ffx-host` it goes to the
function table. A shim calls nothing else: `tools/check-ported.py`
rejects a `transmute`d function pointer or an `asm!` block in
`ported/*.rs`. An `fld dword` operand is `f64::from(x)` (exact). An import
slot's function is `super::import_slot(slot)`.

## Threads

The game runs part of its frame off the main thread, so a port can run on
several threads at once, and must be reentrant: no process-wide scratch,
no Rust state shared between threads without the original's locking.

- **Scratch is per call or per thread.** Use `ported::StackFrame::<W>` (a
  zeroed frame on the shim's stack, the original's frame) or a
  `thread_local!` buffer when the result must outlive the call. Never a
  `static` array or cell handed out by address: `tools/check-ported.py`
  rejects one. A shared `Ch_CalcElement` frame garbled every battle model
  (2026-10-17): two job threads overwrote each other's skeleton matrices
  before their upload. The oracle cannot see this: it is single-threaded.
  The one exception is a table filled once before the game runs and only
  read after, such as the vtable of an object Rust hands the game
  (`rust_audio.rs`); its line carries `// check-ported: read-only table`.
- **Hand the game a `StackFrame`, not a Rust local.** In `ffx-host` a
  `StackFrame` lives on the game thread's stack in the arena, where
  Unicorn's originals reach it; a local's address (`&raw mut x`,
  `buf.as_mut_ptr()`, a `thread_local!`'s) does not. `tools/host-ports.py`
  keeps such shims out of ffx-host's table (306 on 2026-10-29) and
  check-ported fails until its list is regenerated.
- The hosts' `callee_scratch()` / `scratch()` (`ffx-bridge`, `ffx-graphic`,
  `ffx-script`) are per thread, so a bridge must not hold one across a call
  that can re-enter another port on the same thread.
- **Stack.** JobWorker threads commit 128 KB (`0x00703a50`: CreateThread
  with 0x20000; 1 MB reserved, the image default), so a 48 KB frame such as
  `Ch_CalcElement`'s fits, as it does for the original. The dedicated
  threads are smaller: VsyncEmulationThread, AsyncLoadManager,
  AsyncControllerTaskManager and Read command commit 32 KB, FmodManager
  64 KB. Keep big frames off what those reach.

What runs where (`python3 -I tools/job-threads.py` lists the hooked ports
each entry reaches through direct calls; calls through pointers are not
followed, so the lists are lower bounds):

| Thread | Entry | Started by | Hooked ports reached |
|---|---|---|---|
| JobWorker (several, `JobSchedule::JobScheduler`, `0x00cdede4`) | `Ch_CalcMiddle_Batch` `0x00833530` | `Ch_CalcMain`, `Ch_CalcMiddle` through `0x00a44b90`, one job per ~200 elements (battles run several) | 72: chara (`Ch_Neck`, `Ch_CalcElement`, `Ch_ElemCalcQuaternion`, `Ch_Mouth`, `Ch_OptposCalc`, shades), main (`Sg_Quaternion*`, `Sg_AngleApproach`), ps2_emu (`FFXVu0*` matrices), render (`graphicCharacterInstanceSetSkeletonMatrices*`, lighting), system (`cdc_*` matrices) |
| JobWorker | `Ch_Cloth_PerCharacter` `0x0082e390` | `Ch_DrawMain` through `0x00a44b40` | 93: main (`Sg_ClothStep`, `Sg_ClothRod*`, `Sg_ClothPlaceAnchors`, envelope asm), chara (cloth flags), system (the page allocator: `user_malloc`, `PageManager_*`), vfx (vector copies) |
| JobWorker | `JobSchedule_pppPart_fillBuffer` `0x0071ed10` | `JobSchedule_pppPart_fillBuffer_Kick` (`updateFFX`) | 20: ps2_emu (`FFXVu0*`), render (`graphicVFXUpdateParameterOnly`, cut-scene 2D), map (`KeDttMdl_UseSingleInstance`) |
| JobWorker | `eiAbmDraw`'s four jobs (`0x00a53450`, `0x00a534b0`, `0x00a534c0`, `0x00a53510`) | `eiAbmDraw` through `0x00a8ed00`..`0x00a8ed60` | about 50: map (`AbmapMakeNodeMatrix`, `AbmapMakeLinkMatrix`), vfx (`op1_md_draw_eiabm*`), system (allocator, input) |
| JobWorker | `asyncFFXInitialize` `0x00636410` | the loader through `0x00633670` | most of the game (about 1500): it runs initialisation beside the main thread while loading |
| VsyncEmulationThread | `VsyncHandler` `0x00887d60` → callback `0x0086ca20` | `TkSetVsyncHandler` → `createVsyncEmuThread` | 3: `MsGetSaveEventAddress`, `FFXApplication_getAppTime`, `FFXGetVideoMode` |
| AsyncLoadManager | `0x00691360` | `AsyncLoadManager_createThread` | 46 (file reads, allocator) |
| AsyncControllerTaskManager | `0x0068d740` | its constructor `0x0068c830` | 52 (input, allocator, `FreeCamera_FreeCamera`) |
| Read command | `0x006bd1e0` | `0x006bca90` | 75 (event loads: `TkSetEventJump`, `Atel*`, `Sg_SetPlayer`, allocator) |
| FmodManager | `0x006fa1b0` (`0x006fa700`, `0x006fa7e0`) | `FmodManager_createThread` | 46 (allocator) |
| render thread | `Phyre_PRendering_PRenderThread_threadEntry` `0x00616010` | the renderer (`Phyre_PRendering_PRenderThread`) | none directly (commands run through pointers) |

Phyre's own pools (`Phyre_PThreadPool_startThreads`,
`Phyre_PThreadPoolWorkStealing_workerThread`, the animation pool
`ANIThreadPoolJobs`), the Bullet step thread, the `BigFileStream` thread-pool
work and the video decoder (`_beginthreadex` at `0x0040c2c0`) run engine code
through pointers; no hooked port is known to be reached from them.

## Building

```bash
tools/build-hook.sh
```

This cross-compiles with `cargo xwin` for `i686-pc-windows-msvc`
(`XWIN_ARCH=x86`), with the CRT linked statically, so the DLL imports only
system libraries. `tools/shims/lld-link` routes linking to rustc's own
`rust-lld`, because Ubuntu's `lld-link` 18 cannot parse the `EXPORTAS`
directive rustc emits for 32-bit stdcall exports.

## Testing

```bash
# Parity against the original machine code (Linux)
FFX_GAME_DIR=~/steam-games/FFX-X2-HD cargo test --release -p ffx-oracle

# Patch the real FFX.exe on Windows, without starting the game
target/i686-pc-windows-msvc/release/examples/hook-smoke.exe "$(wslpath -w ~/steam-games/FFX-X2-HD/FFX.exe)"
```

The smoke test maps `FFX.exe` with `DONT_RESOLVE_DLL_REFERENCES` (no game
code runs), installs the hooks, and calls the original addresses, which now
run Rust.

### Virtual memory in the oracle

`KERNEL32!VirtualAlloc`, `VirtualFree` and `VirtualQuery` have stand-ins
in the oracle (`crates/ffx-oracle/src/win32.rs`, battle round 36), so code
that grows a heap past what the snapshot left (the bone allocator under
`SG_InitBones` on the field snapshot) runs instead of stopping at an
unstubbed import:

- `VirtualAlloc(0, size, ..)` maps a new zeroed read-write region (size
  rounded up to pages, protection ignored) at the lowest 64 KB-aligned gap
  from `0x10000000` up that neither the oracle's memory nor the snapshot
  (mapped yet or not) uses. Two oracles in the same state get the same
  addresses for the same calls, so original and port stay comparable.
  `MEM_TOP_DOWN` is ignored.
- A fixed address commits: inside a region handed out it returns the page;
  elsewhere (memory the game reserved before the snapshot) the pages that
  are neither mapped nor in the snapshot are mapped zeroed.
- `VirtualFree` never unmaps (an address is never handed out twice):
  `MEM_RELEASE` of a region's base marks it released, a second release or
  one not at the base fails; decommits keep the bytes; memory from before
  the snapshot always succeeds.
- `VirtualQuery` describes only the regions handed out (committed,
  private, read-write, from the page to the region's end; free once
  released) and returns 0 for anything else, which the game's allocators
  take as "don't release".

`Oracle::map_zeroed(addr, len)` maps zeroed pages at a fixed place for
tests of code that writes through a null or made-up pointer (the null
character `Ch_Allocate` uses when every slot is taken). Pages already
mapped or in the snapshot stay as they are.

### Harvested call states

Parity tests can run on real calls instead of hand-built inputs
(`crates/ffx-oracle/src/harvest.rs`). `Oracle::harvest` runs a driver (an
original top-level routine) for some frames on a snapshot; a block hook at
each target's entry saves the call as the original made it: registers, x87
and SSE state, 64 KiB of stack above `ESP`, and every page written so far
that differs from the snapshot. The game's clock reads a system DLL, so the
harvest patches `Phyre_PTimerWin32_GetTime` to a fake clock
(`Oracle::fake_clock`); the patch travels with each state. States are
deduplicated by a hash of `EAX`/`ECX`/`EDX`, the first eight stack words and
64 bytes behind each, and capped per target.

```bash
tools/harvest.sh                        # fills ~/ffx-work/harvest from field.snap and battle.snap
cargo run --release -p ffx-oracle --example harvest -- --snapshot <snap> \
    --driver 0x00820ae0 --frames 60 --max 8 [--targets ported|<file>|<addr,...>]
```

The store is `~/ffx-work/harvest/<snapshot>/<addr>/<n>.state` (`FFX_HARVEST`
moves it): an 8-byte magic, the dedupe key, an LZ4 block. Drivers in
`tools/harvest.sh`: `Sg_MainLoop` (`0x00820ae0`, the game's frame: about 600
ported targets on each snapshot), `Yn_FieldProc` (`0x0091a8c0`) and
`MsBattleProcess` (`0x00790c10`). The store stays outside git; rerun the
script after a new snapshot or a new hook.

A test is one line at the end of its module's file,
`crates/ffx-oracle/tests/harvested_<module>.rs` (`harvested_main.rs` for ffx-sg,
`harvested_battle.rs`, `harvested_vfx.rs`, and so on). `harvested.rs` keeps only
the store's layout check; the shared helpers are in `tests/common/mod.rs`. The
example is from `harvested_main.rs`:

```rust
harvest_test!(sg_seq_step, 0x0083_7900, args = 1, |m: &mut Oracle, s| {
    ffx_sg::seq::bridge::seq_step(&mut OracleHost(m), s.arg(0))
});
```

For each stored state it builds two oracles on the snapshot, applies the
state to both (`Oracle::apply_state`), runs the original from the saved
registers in one (`Oracle::run_state`, returning to the oracle) and the
closure in the other, then compares the result (`u32`, `i32`, `bool`,
`u64` as `EDX:EAX`, `f32`/`f64` in `ST0` bit for bit with NaN payloads,
`Masked`, `()`), every byte written
outside the stacks, and the caller's stack above the `args` words. A float
the original leaves in `ST0` fails a non-float port. `s.arg(i)`, `s.ecx()`,
`s.edx()` read the call's arguments; `OracleHost` is the `ffx_bridge::Host`
for bridges that call originals (their frames go below the saved stack).
Tests skip without `FFX_GAME_DIR` or states; `FFX_HARVEST_REQUIRE=1` makes a
missing store a failure, `FFX_HARVEST_LIMIT=n` caps states per snapshot.
`ffx_oracle::harvested(addr)` gives the cases for custom checks
(`Case::compare` returns the difference instead of panicking, for mutation
runners).

Rough branch reach: `FFX_ORACLE_COVER=0x<addr>+<len hex>,...` with
`FFX_ORACLE_COVER_OUT=<file>` makes every oracle append the blocks it ran in
those ranges (`ffx.sym` addresses) to the file when dropped.

## In the game

The game runs on Windows straight from its folder on the WSL filesystem,
without Steam:

```bash
tools/run-game.sh             # builds and installs the hooks as DINPUT8.dll, then starts FFX.exe
tools/run-game.sh --no-hooks  # unmodified game
tools/run-game.sh --lang de   # language: en (default), jp, fr, de, it, es, kr, ch
```

The language comes from `Documents\Square Enix\FINAL FANTASY X&X-2 HD
Remaster\GameSetting.ini` (`Settings_Load`, `0x00401cb0`), which the Steam
launcher normally writes. Without it the game creates the file with
`Language=jp`, so the script sets the `Language=` line on every launch.

Then check `ffx-hook.log` in the game folder: every hook should say
`Installed` (one block of lines, written at once: the log may sit on the slow WSL share).

No Steam patch is needed. `SteamAPI_RestartAppIfNecessary(0)` returns false,
`SteamAPI_Init()` fails, and the game carries on with Steam features
(achievements, callbacks) off; they are all guarded by the "Steam
initialized" flag at `0x00cc9cf5` set in `Steam_Init` (`0x00623470`).

The script handles two quirks of running from the WSL filesystem:

- Windows only loads images that have the Linux execute bit.
- The filesystem is case-sensitive, and `FFX.exe` imports `MSVCR110.dll`,
  `MSVCP110.dll` and `DINPUT8.dll` in upper case. The script adds uppercase
  hard links for the two runtime DLLs and installs our proxy as `DINPUT8.dll`.
  With the wrong case, Windows reports the DLL as missing.

`tools/frida/trace-startup.py` spawns the game under frida-server and logs
Steam API calls and a backtrace at exit, which is how the above was found.
