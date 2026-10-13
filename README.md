# ffx-decomp

A reimplementation of Final Fantasy X (the Steam HD Remaster) in Rust,
recovered by reverse engineering `FFX.exe`.

You need your own copy of the game. This repository contains no game code or
data, and never will: binaries, archives and extracted files stay out of git.

## Status

| Part | State |
|---|---|
| VBF archive reader (`ffx-vbf`) | Done. Reads every file in `FFX_Data.vbf` |
| `ffx` CLI | `vbf list`, `vbf cat`, `vbf extract` |
| Function replacement | `dinput8.dll` proxy patches 2 functions (`zlib_uncompress`, `Path_ToLowerAscii`); parity with the originals verified under emulation and on Windows; not yet run in the game |
| Symbol map (`symbols/ffx.sym`) | 33 names: file system, archive and zlib code |

## Layout

```text
crates/ffx-core     Rust reimplementations of FFX.exe functions
crates/ffx-oracle   runs original FFX.exe functions in an emulator, for parity tests
crates/ffx-hook     patches FFX.exe functions to jump into ffx-core
crates/ffx-dinput8  dinput8.dll proxy that loads ffx-hook into the game
crates/ffx-vbf      VBF archive reader
crates/ffx-cli      the `ffx` command-line tool
symbols/ffx.sym     recovered names for FFX.exe, the source of truth
docs/formats        file format descriptions, with the evidence behind them
tools/ghidra        headless Ghidra scripts (apply symbols, dump decompiled code)
tools/build-hook.sh builds the 32-bit dinput8.dll
```

## Usage

```bash
export FFX_GAME_DIR=~/steam-games/FFX-X2-HD     # the folder containing FFX.exe

cargo run --release -p ffx-cli -- vbf list "$FFX_GAME_DIR/data/FFX_Data.vbf" | head
cargo run --release -p ffx-cli -- vbf extract "$FFX_GAME_DIR/data/FFX_Data.vbf" \
    --out extracted --prefix ffx_ps2/ffx/master/new_uspc/battle/kernel/
```

## Tests

```bash
cargo test                                    # synthetic archives; metamenu.vbf if FFX_GAME_DIR is set
cargo test --release -p ffx-vbf --test game_data -- --ignored   # all of FFX_Data.vbf (~20 GB)
cargo test --release -p ffx-oracle            # Rust vs. original machine code (needs FFX_GAME_DIR)
```

Replacing game functions: see [docs/hooking.md](docs/hooking.md).

## Reverse engineering setup

See [docs/workflow.md](docs/workflow.md).

## Prior work

[Fahrenheit](https://github.com/fahrenheit-crew/fahrenheit) and
[VBFExtract](https://github.com/Kaldaien/VBFExtract) are used only as
references for facts. No code is copied from them; everything here is
derived from `FFX.exe` and the game files.
