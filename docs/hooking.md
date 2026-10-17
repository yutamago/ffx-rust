# Replacing FFX.exe functions with Rust

The rebuild proceeds one function at a time. Each replacement is written in
Rust, checked against the original machine code, and then patched into the
running game in place of the original. Once enough of the game is replaced,
`FFX.exe` is no longer needed.

## Pieces

| Crate | Role |
|---|---|
| `ffx-core` | The reimplementations, as safe, platform-independent Rust |
| `ffx-oracle` | Runs the original functions from `FFX.exe` in a Unicorn x86 emulator, so tests can compare them with `ffx-core` on Linux |
| `ffx-hook` | The hook table, the patcher, and `extern "C"` shims with the originals' exact signatures |
| `ffx-dinput8` | `dinput8.dll` proxy that loads `ffx-hook` into the game |

## How a hook works

`FFX.exe` imports `DINPUT8.dll!DirectInput8Create`, and Windows looks for
DLLs in the game folder first. Our `dinput8.dll` is therefore loaded before
any game code runs. Its `DllMain`:

1. checks the main module's PE `TimeDateStamp` and `SizeOfImage` against the
   supported build, and patches nothing if they differ;
2. rebases each hook address onto the module's real base (ASLR is on);
3. checks that each site still holds the original first five bytes;
4. overwrites them with `jmp rel32` to the Rust shim.

The original is never called again, so no trampoline is needed.
`DirectInput8Create` itself forwards to the system `dinput8.dll`.

Results go to `ffx-hook.log` next to `FFX.exe`. Set
`FFX_HOOK_DISABLE=zlib_uncompress,Path_ToLowerAscii` (any subset) to keep
those originals, which is useful for A/B comparisons.

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

## Replaced so far

| Function | Address | Checked by |
|---|---|---|
| `zlib_uncompress` | `0x00405e20` | 1,500 synthetic streams (fit, spare room, too small, truncated, flipped bit; all result codes) and every compressed block in `metamenu.vbf` |
| `Path_ToLowerAscii` | `0x00607d50` | NULL, empty, all 255 byte values, 2,000 random strings; output bytes and return value |
| `Phyre_PSerialization_PStreamFileWin32_openFile` | `0x00607f40` | Wrapped, not replaced: mod lookup, then the original through a trampoline; trampoline checked by the smoke test, mod loading in the game |
| `CDfileSize_CD`, `CDfileSize_PC` | `0x0076c320`, `0x00a428a0` | Wrapped: mod files report their own size; in the game by `an_edited_motion_group_loads_in_the_game` |
| `Input_KeyHeld` | `0x00629d50` | Wrapped for the test harness (`docs/harness.md`) |

## Adding a replacement

1. Understand the original: decompile it, then read its assembly for the
   calling convention, return value and side effects. Record the name in
   `symbols/ffx.sym`.
2. Write it in `ffx-core`, with unit tests.
3. Add a parity test in `crates/ffx-oracle/tests/parity.rs` that runs both on
   many inputs, including edge cases and invalid ones, and asserts that
   every interesting outcome actually occurred.
4. Add a shim to `crates/ffx-hook/src/replacements.rs` with the original's
   exact signature, and an entry to `hooks()` with the first five bytes of
   the original.
5. Build and run the Windows smoke test, then try it in the game.

Only functions whose callers can be satisfied through their signature are
safe to replace this way. A function that the game expects to touch global
state must touch the same state in Rust.

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
