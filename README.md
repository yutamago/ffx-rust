# ffx-decomp

A reimplementation of Final Fantasy X (the Steam HD Remaster) in Rust,
recovered by reverse engineering `FFX.exe`.

You need your own copy of the game. This repository contains no game code or
data, and never will: binaries, archives and extracted files stay out of git.

## Status

| Part | State |
|---|---|
| VBF archives (`ffx-vbf`) | Reads every file; writes archives (`ffx vbf pack`): repacking `FFX_Data.vbf` unchanged reproduces it byte for byte |
| Kernel tables (`ffx-kernel`) | All 26 tables × 8 languages read, written and round-tripped through TOML byte for byte, plus the shared equipment (starting, shop, found) and treasure-chest tables; commands, items, equipment and chests named, the equipment layout checked in battle |
| Monsters (`ffx-monster`) | All 361 monster files: stats (HP, MP, strength, defense, magic, magic defense, …) and rewards (gil, AP, drops, steal, bribe) editable via TOML; byte-exact round trip |
| Battles (`ffx-battle`) | The encounter table (96 battle fields: encounter rate, music, formations and their weights) and all 863 formation files (monster slots, flags) editable via TOML; byte-exact round trip; `ffx battle list` shows every field's formations with their monsters |
| Dialogue (`ffx-text`) | Every event's dialogue and battle's texts, all 8 languages (3,882 tables, 171,253 messages), editable via TOML with readable control codes; texts can grow, messages can be added; byte-exact round trip; checked in game (the opening's first subtitle) |
| Scripts (`ffx-script`) | Bytecode interpreter decoded; all 758 event and monster AI scripts (17.3M instructions) disassemble and reassemble byte for byte; edits that grow code are relocated; native calls resolved to their FFX.exe handlers |
| Textures (`ffx-phyre`) | All 44,364 PC textures to upright DDS and back, byte for byte (DXT1/3/5, ARGB8, L8, cube maps) |
| Models and maps (`ffx-phyre`) | All 1,266 meshes to valid glTF with node hierarchy, instances, skins, embedded base color textures and animations; edited glTF geometry written back (vertex and triangle counts may change), unchanged models byte for byte; new primitives become new segments, with new materials and textures, and vertices may be weighted to any joint (palettes and skeleton matrices grow); checked in game. Vertex streams and materials come from the decoded pointer and array fixups, which match the original unpackers. |
| Motions (`ffx-motion`) | PS2 skeletons and all 3,156 motion groups (21,165 clips): decoded, rebuilt byte for byte, editable as JSON; exported as glTF animations on the remastered models (857 `chr/` models validate) |
| Text (`ffx-text`) | Encoding fully decoded for all 8 languages; all 92,224 kernel strings round-trip |
| `ffx` CLI | `vbf list/cat/extract/pack`, `kernel tables/export/import`, `monster list/export/import`, `battle list/export/import`, `text export/import`, `script list/disasm/asm`, `texture export/import`, `model export/import`, `motion export/import`, `phyre dump` |
| Function replacement | `dinput8.dll` proxy patches 2 functions (`zlib_uncompress`, `Path_ToLowerAscii`) with parity-tested Rust; runs in the game |
| Mods | `mods/<mod>/<archive path>` replaces archive files in the running game, also files that grew (the PS2 file layer's sizes are hooked) |
| In-game tests (`ffx-harness`) | The game runs in the background under a TCP harness (buttons, keys, memory, screenshots, file events); tests play into the first scene with an edited event script (grown and relocated; its write read back from memory), a resized model and edited motions, start a battle to check monster, formation and encounter-table mods, and check a kernel mod in memory; saves and settings are sandboxed, randomness seeded, frames run back to back |
| Symbol map (`symbols/ffx.sym`) | file system, archive, zlib, text, kernel, settings, PhyreEngine fixups and motion code |

## Layout

```text
crates/ffx-core     Rust reimplementations of FFX.exe functions
crates/ffx-oracle   runs original FFX.exe functions in an emulator, for parity tests
crates/ffx-hook     patches FFX.exe functions to jump into ffx-core; test harness
crates/ffx-harness  launches and drives the game from tests
crates/ffx-dinput8  dinput8.dll proxy that loads ffx-hook into the game
crates/ffx-vbf      VBF archive reader
crates/ffx-kernel   battle kernel tables: reader, writer, TOML export/import
crates/ffx-monster  monster files: stats and rewards
crates/ffx-battle   encounter table and battle formations
crates/ffx-script   script disassembler and assembler
crates/ffx-phyre    PhyreEngine clusters: textures to and from DDS, models to glTF
crates/ffx-motion   PS2 skeletons and motion groups: reader, writer, JSON
crates/ffx-text     in-game text encoding
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

cargo run --release -p ffx-cli -- kernel export item -o item.toml   # items as editable TOML
```

## Tests

```bash
cargo test                                    # synthetic archives; metamenu.vbf if FFX_GAME_DIR is set
cargo test --release -p ffx-vbf --test game_data -- --ignored   # all of FFX_Data.vbf (~20 GB)
cargo test --release -p ffx-oracle            # Rust vs. original machine code (needs FFX_GAME_DIR)
```

## Playing and modding

```bash
tools/run-game.sh                 # FFX.exe on Windows, English, with hooks and mods (no Steam needed)
tools/run-game.sh --no-hooks      # the original game

# Make a mod: export a table, edit it, import it into a mod folder
cargo run --release -p ffx-cli -- kernel export item -o item.toml
cargo run --release -p ffx-cli -- kernel import item item.toml --mod-dir "$FFX_GAME_DIR/mods/my-mod"
```

Replacing game functions and how mods load: [docs/hooking.md](docs/hooking.md). Driving the running game from tests: [docs/harness.md](docs/harness.md).

## Reverse engineering setup

See [docs/workflow.md](docs/workflow.md).

## Prior work

[Fahrenheit](https://github.com/fahrenheit-crew/fahrenheit) and
[VBFExtract](https://github.com/Kaldaien/VBFExtract) are used only as
references for facts. No code is copied from them; everything here is
derived from `FFX.exe` and the game files.
