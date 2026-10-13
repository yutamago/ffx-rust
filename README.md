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
| Symbol map (`symbols/ffx.sym`) | 30 names: file system and archive code |

## Layout

```text
crates/ffx-vbf    VBF archive reader
crates/ffx-cli    the `ffx` command-line tool
symbols/ffx.sym   recovered names for FFX.exe, the source of truth
docs/formats      file format descriptions, with the evidence behind them
tools/ghidra      headless Ghidra scripts (apply symbols, dump decompiled code)
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
```

## Reverse engineering setup

See [docs/workflow.md](docs/workflow.md).

## Prior work

[Fahrenheit](https://github.com/fahrenheit-crew/fahrenheit) and
[VBFExtract](https://github.com/Kaldaien/VBFExtract) are used only as
references for facts. No code is copied from them; everything here is
derived from `FFX.exe` and the game files.
