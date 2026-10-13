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

`File_Open` (`0x00607f40`) is the only way the game opens archive files.
It normalizes the path, asks the archives, and if they lack the file opens
it from disk with `CreateFileW`; the size, read and close methods then use
that handle. The `File_Open` hook adds one step in front: for a read open of
a path that a mod provides, it opens the mod's file read-only and returns,
so the game's own loose-file code reads it. Everything else goes to the
original through a trampoline.

```text
<game folder>/mods/<mod name>/<archive path>
<game folder>/mods/test-potion/ffx_ps2/ffx/master/new_uspc/battle/kernel/item.bin
```

Mods load in name order; a later mod's file replaces an earlier one's, and
the log says so. The file must be opened read-only: `File_Close` calls
`SetEndOfFile`, which would truncate a writable mod file.

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
| `File_Open` | `0x00607f40` | Wrapped, not replaced: mod lookup, then the original through a trampoline; trampoline checked by the smoke test, mod loading in the game |

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
`Installed`.

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
